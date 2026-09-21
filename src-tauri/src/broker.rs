use rhodiz_harness_broker_core::{
    classify_wsl_status, lifecycle_busy, lifecycle_lock_unavailable, operation_result,
    runtime_log_args, runtime_status as build_status, sanitize_log_text, CommandOutcome,
    OperationState, RuntimeLogsResult, RuntimeOperation, RuntimeOperationResult, RuntimeStatus,
    RuntimeVerifyResult, COMMAND_TIMEOUT_SECS, MAX_CAPTURE_BYTES, WSL_EXE, WSL_START_ARGS,
    WSL_STATUS_ARGS, WSL_STOP_ARGS,
};

// The provisioning preflight only runs where there is a `wsl.exe` to probe;
// the fallback below never reaches it.
#[cfg(target_os = "windows")]
use rhodiz_harness_broker_core::{
    decode_utf16le, extract_wsl_version, provisioning_preflight, WSL_VERSION_ARGS,
};

// Only the non-Windows fallbacks report "unsupported"; importing it
// unconditionally leaves a dead import on the one platform that ships.
#[cfg(not(target_os = "windows"))]
use rhodiz_harness_broker_core::{unsupported_operation, unsupported_verify};

// Same for the verify and repair probes: they are issued only by the Windows
// implementations of the platform functions below.
#[cfg(target_os = "windows")]
use rhodiz_harness_broker_core::{
    verify_result, WSL_REPAIR_RESET_ARGS, WSL_REPAIR_RESTART_ARGS, WSL_VERIFY_ARGS,
};

#[cfg(target_os = "windows")]
use rhodiz_harness_broker_core::{
    lifecycle_cross_process_busy, LIFECYCLE_LOCK_DIRECTORY, LIFECYCLE_LOCK_FILE,
};

use std::sync::{Mutex, TryLockError};

#[cfg(target_os = "windows")]
use std::{
    env,
    fs::{self, OpenOptions},
    io::Read,
    os::windows::{fs::OpenOptionsExt, process::CommandExt},
    path::PathBuf,
    process::{Command, Stdio},
    thread,
    time::Duration,
};
#[cfg(target_os = "windows")]
use wait_timeout::ChildExt;

/// Keeps `wsl.exe` from flashing a console window inside a
/// `windows_subsystem = "windows"` GUI process.
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Serializes the mutating lifecycle operations so a start and a stop can never
/// drive the same systemd unit concurrently.
static LIFECYCLE_MUTATION_LOCK: Mutex<()> = Mutex::new(());

#[cfg(target_os = "windows")]
struct ProcessCapture {
    outcome: CommandOutcome,
    stdout: Vec<u8>,
    truncated: bool,
}

#[cfg(target_os = "windows")]
fn read_bounded<R>(mut reader: R) -> (Vec<u8>, bool)
where
    R: Read,
{
    let mut stored = Vec::new();
    let mut buffer = [0_u8; 8 * 1024];
    let mut truncated = false;

    loop {
        match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => {
                let remaining = MAX_CAPTURE_BYTES.saturating_sub(stored.len());
                let keep = remaining.min(read);
                stored.extend_from_slice(&buffer[..keep]);
                truncated |= keep < read;
            }
            Err(_) => {
                truncated = true;
                break;
            }
        }
    }
    (stored, truncated)
}

#[cfg(target_os = "windows")]
fn wsl_executable_path() -> Option<PathBuf> {
    let system_root = PathBuf::from(env::var_os("SystemRoot")?);
    if !system_root.is_absolute() {
        return None;
    }
    Some(system_root.join("System32").join(WSL_EXE))
}

#[cfg(target_os = "windows")]
fn run_wsl(args: &[String]) -> ProcessCapture {
    let Some(executable) = wsl_executable_path() else {
        return ProcessCapture {
            outcome: CommandOutcome::SpawnFailed,
            stdout: Vec::new(),
            truncated: false,
        };
    };

    let mut child = match Command::new(executable)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
    {
        Ok(child) => child,
        Err(_) => {
            return ProcessCapture {
                outcome: CommandOutcome::SpawnFailed,
                stdout: Vec::new(),
                truncated: false,
            };
        }
    };

    let stdout_reader = child
        .stdout
        .take()
        .map(|pipe| thread::spawn(move || read_bounded(pipe)));
    let stderr_reader = child
        .stderr
        .take()
        .map(|pipe| thread::spawn(move || read_bounded(pipe)));

    let outcome = match child.wait_timeout(Duration::from_secs(COMMAND_TIMEOUT_SECS)) {
        Ok(Some(status)) if status.success() => CommandOutcome::Success,
        Ok(Some(status)) => CommandOutcome::Exit(status.code().unwrap_or(-1)),
        Ok(None) => {
            let _ = child.kill();
            let _ = child.wait();
            CommandOutcome::TimedOut
        }
        Err(_) => {
            let _ = child.kill();
            let _ = child.wait();
            CommandOutcome::SpawnFailed
        }
    };

    let (stdout, stdout_truncated) = stdout_reader
        .and_then(|handle| handle.join().ok())
        .unwrap_or_else(|| (Vec::new(), true));
    let stderr_truncated = stderr_reader
        .and_then(|handle| handle.join().ok())
        .map(|(_, truncated)| truncated)
        .unwrap_or(true);

    ProcessCapture {
        outcome,
        stdout,
        truncated: stdout_truncated || stderr_truncated,
    }
}

fn fixed_args<const N: usize>(args: [&str; N]) -> Vec<String> {
    args.iter().map(|value| (*value).to_string()).collect()
}

fn fixed_start_args() -> Vec<String> {
    fixed_args(WSL_START_ARGS)
}

fn fixed_stop_args() -> Vec<String> {
    fixed_args(WSL_STOP_ARGS)
}

#[cfg(target_os = "windows")]
fn platform_status() -> RuntimeStatus {
    let capture = run_wsl(&fixed_args(WSL_STATUS_ARGS));
    build_status("windows", classify_wsl_status(capture.outcome))
}

#[cfg(not(target_os = "windows"))]
fn platform_status() -> RuntimeStatus {
    build_status("unsupported", rhodiz_harness_broker_core::unavailable())
}

#[cfg(target_os = "windows")]
fn platform_operation(operation: RuntimeOperation, args: &[String]) -> RuntimeOperationResult {
    operation_result(operation, run_wsl(args).outcome)
}

#[cfg(not(target_os = "windows"))]
fn platform_operation(operation: RuntimeOperation, _args: &[String]) -> RuntimeOperationResult {
    unsupported_operation(operation)
}

#[cfg(target_os = "windows")]
fn platform_verify() -> RuntimeVerifyResult {
    verify_result(run_wsl(&fixed_args(WSL_VERIFY_ARGS)).outcome)
}

#[cfg(not(target_os = "windows"))]
fn platform_verify() -> RuntimeVerifyResult {
    unsupported_verify()
}

/// Repair performs exactly two spawns under one lock. `reset-failed` is
/// best-effort cleanup for a latched start-limit failure, so its outcome is
/// deliberately ignored: the result that decides the repair is `restart`. When
/// the distribution is unreachable, `restart` fails the same way, so the
/// diagnosis the failure produces stays correct.
#[cfg(target_os = "windows")]
fn platform_repair() -> RuntimeOperationResult {
    let _ = run_wsl(&fixed_args(WSL_REPAIR_RESET_ARGS));
    operation_result(
        RuntimeOperation::Repair,
        run_wsl(&fixed_args(WSL_REPAIR_RESTART_ARGS)).outcome,
    )
}

#[cfg(not(target_os = "windows"))]
fn platform_repair() -> RuntimeOperationResult {
    unsupported_operation(RuntimeOperation::Repair)
}

/// Cross-process half of the lifecycle guard.
///
/// The in-process mutex cannot see a second copy of the application, and
/// nothing in this build prevents one from being launched, so on its own it
/// would leave two processes free to drive the same systemd unit at once. The
/// file is opened denying all sharing, so a second process fails to open it at
/// all. Windows releases the handle when a process dies, so a crash cannot
/// strand the lock.
///
/// Returns `None` both when another process holds the lock and when the lock
/// file cannot be created, so the caller fails closed either way.
#[cfg(target_os = "windows")]
fn acquire_cross_process_lock() -> Option<std::fs::File> {
    let root = PathBuf::from(env::var_os("LOCALAPPDATA")?);
    if !root.is_absolute() {
        return None;
    }
    let directory = root.join(LIFECYCLE_LOCK_DIRECTORY);
    fs::create_dir_all(&directory).ok()?;
    OpenOptions::new()
        .create(true)
        .write(true)
        // The file is a lock, not data: its content is never read, so the
        // existing bytes are left untouched and the truncation question the
        // linter asks is answered explicitly as "no".
        .truncate(false)
        .share_mode(0)
        .open(directory.join(LIFECYCLE_LOCK_FILE))
        .ok()
}

fn with_lifecycle_lock<F>(operation: RuntimeOperation, action: F) -> RuntimeOperationResult
where
    F: FnOnce() -> RuntimeOperationResult,
{
    let _process_guard = match LIFECYCLE_MUTATION_LOCK.try_lock() {
        Ok(guard) => guard,
        Err(TryLockError::WouldBlock) => return lifecycle_busy(operation),
        Err(TryLockError::Poisoned(_)) => return lifecycle_lock_unavailable(operation),
    };

    #[cfg(target_os = "windows")]
    let _machine_guard = match acquire_cross_process_lock() {
        Some(file) => file,
        None => return lifecycle_cross_process_busy(operation),
    };

    action()
}

#[cfg(target_os = "windows")]
fn platform_provision() -> RuntimeOperationResult {
    let status = run_wsl(&fixed_args(WSL_STATUS_ARGS)).outcome;

    // The version probe only runs once the status probe proved WSL answers at
    // all. Spawning `wsl.exe --version` against an absent or wedged WSL would
    // just spend another COMMAND_TIMEOUT_SECS to learn what we already know,
    // and the preflight ignores the version on those branches anyway.
    let version = if matches!(status, CommandOutcome::Success) {
        read_wsl_version()
    } else {
        None
    };

    provisioning_preflight(status, version)
}

/// Reads the installed WSL version, or `None` if it cannot be established.
///
/// `wsl.exe --version` writes UTF-16LE, so this is the one place in the broker
/// that decodes stdout for anything but logs. A truncated capture is discarded
/// rather than parsed: the cut can land mid-version and yield a plausible but
/// wrong triple, and the caller treats `None` as "refuse to provision".
#[cfg(target_os = "windows")]
fn read_wsl_version() -> Option<(u32, u32, u32)> {
    let capture = run_wsl(&fixed_args(WSL_VERSION_ARGS));
    if !matches!(capture.outcome, CommandOutcome::Success) || capture.truncated {
        return None;
    }
    extract_wsl_version(&decode_utf16le(&capture.stdout)?)
}

#[cfg(not(target_os = "windows"))]
fn platform_provision() -> RuntimeOperationResult {
    unsupported_operation(RuntimeOperation::Provision)
}

// `async` keeps the bounded-but-blocking `wsl.exe` calls off the UI thread:
// Tauri runs a non-async command body on the main thread, where a probe that
// waits up to COMMAND_TIMEOUT_SECS would freeze the window.
#[tauri::command(async)]
pub fn runtime_status() -> RuntimeStatus {
    platform_status()
}

// Provisioning performs no mutation yet, but it takes the lifecycle lock all
// the same: once signed-manifest provisioning is implemented it must hold it,
// and a comment is easier to miss than a call site that is already correct.
#[tauri::command(async)]
pub fn runtime_provision() -> RuntimeOperationResult {
    with_lifecycle_lock(RuntimeOperation::Provision, platform_provision)
}

#[tauri::command(async)]
pub fn runtime_start() -> RuntimeOperationResult {
    with_lifecycle_lock(RuntimeOperation::Start, || {
        platform_operation(RuntimeOperation::Start, &fixed_start_args())
    })
}

#[tauri::command(async)]
pub fn runtime_stop() -> RuntimeOperationResult {
    with_lifecycle_lock(RuntimeOperation::Stop, || {
        platform_operation(RuntimeOperation::Stop, &fixed_stop_args())
    })
}

// Verification is read-only and deliberately takes no lock. Holding one would
// answer `blocked` during a start or a stop, exactly when a diagnosis is most
// needed. That is safe because the only probe it issues, `systemctl
// is-active`, mutates nothing; the broker-core constants document that
// property on the argument vector itself.
//
// The price of being lock-free is that the answer is an instantaneous
// snapshot: a probe issued while a start, stop or repair is mid-flight can
// report the transient value it happens to observe. That is preferable to
// blocking the diagnosis, and callers that need a settled state re-verify
// after the mutation returns.
#[tauri::command(async)]
pub fn runtime_verify() -> RuntimeVerifyResult {
    platform_verify()
}

// Repair is an explicit user-invoked mutation: clear a latched failure and
// restart the bootstrap unit, nothing else. It holds the lifecycle lock for
// both spawns, like every other mutating operation.
#[tauri::command(async)]
pub fn runtime_repair() -> RuntimeOperationResult {
    with_lifecycle_lock(RuntimeOperation::Repair, platform_repair)
}

#[cfg(target_os = "windows")]
fn platform_logs(lines: Option<u16>) -> RuntimeLogsResult {
    let args = runtime_log_args(lines);
    let capture = run_wsl(&args);

    if capture.outcome == CommandOutcome::Success {
        let raw = String::from_utf8_lossy(&capture.stdout);
        let (sanitized, content_truncated) = sanitize_log_text(&raw, lines);
        RuntimeLogsResult {
            state: OperationState::Succeeded,
            lines: sanitized,
            truncated: capture.truncated || content_truncated,
            detail: None,
        }
    } else {
        let result = operation_result(RuntimeOperation::Logs, capture.outcome);
        RuntimeLogsResult {
            state: result.state,
            lines: Vec::new(),
            truncated: capture.truncated,
            detail: result.detail,
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn platform_logs(_lines: Option<u16>) -> RuntimeLogsResult {
    let result = unsupported_operation(RuntimeOperation::Logs);
    RuntimeLogsResult {
        state: result.state,
        lines: Vec::new(),
        truncated: false,
        detail: result.detail,
    }
}

#[tauri::command(async)]
pub fn runtime_logs(lines: Option<u16>) -> RuntimeLogsResult {
    platform_logs(lines)
}
