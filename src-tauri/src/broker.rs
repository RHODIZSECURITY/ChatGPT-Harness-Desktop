use rhodiz_harness_broker_core::{
    classify_wsl_status, operation_result, provisioning_blocked, runtime_log_args,
    runtime_status as build_status, sanitize_log_text, CommandOutcome, OperationState,
    RuntimeLogsResult, RuntimeOperation, RuntimeOperationResult, RuntimeStatus,
    COMMAND_TIMEOUT_SECS, MAX_CAPTURE_BYTES, WSL_EXE, WSL_START_ARGS, WSL_STATUS_ARGS,
    WSL_STOP_ARGS,
};

// Only the non-Windows fallbacks report "unsupported"; importing it
// unconditionally leaves a dead import on the one platform that ships.
#[cfg(not(target_os = "windows"))]
use rhodiz_harness_broker_core::unsupported_operation;

use std::sync::{Mutex, TryLockError};

#[cfg(target_os = "windows")]
use std::{
    env,
    io::Read,
    os::windows::process::CommandExt,
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

fn with_lifecycle_lock<F>(operation: RuntimeOperation, action: F) -> RuntimeOperationResult
where
    F: FnOnce() -> RuntimeOperationResult,
{
    match LIFECYCLE_MUTATION_LOCK.try_lock() {
        Ok(_guard) => action(),
        Err(TryLockError::WouldBlock) => RuntimeOperationResult {
            operation,
            state: OperationState::Blocked,
            detail: Some("another managed runtime lifecycle operation is active".to_string()),
        },
        Err(TryLockError::Poisoned(_)) => RuntimeOperationResult {
            operation,
            state: OperationState::Failed,
            detail: Some("managed runtime lifecycle lock is unavailable".to_string()),
        },
    }
}

#[cfg(target_os = "windows")]
fn platform_provision() -> RuntimeOperationResult {
    provisioning_blocked()
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

// Provisioning performs no mutation yet, so it does not take the lifecycle
// lock; it must do so once signed-manifest provisioning is implemented.
#[tauri::command]
pub fn runtime_provision() -> RuntimeOperationResult {
    platform_provision()
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
