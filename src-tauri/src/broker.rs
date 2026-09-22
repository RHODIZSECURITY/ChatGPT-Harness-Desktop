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
    advanced_rollback_floor, decide_update, decode_utf16le, extract_wsl_version,
    parse_release_manifest, provisioning_preflight, resolve_runtime_bundle,
    verify_release_manifest, wsl_import_args, BundleError, ImportArgsError, InstalledRelease,
    ManifestParseError, ReleaseManifest, RuntimeBundle, UpdateDecision, UpdateRefusal,
    MANAGED_DISTRO_NAME, WSL_CONF_CONTENTS, WSL_TERMINATE_ARGS, WSL_VERSION_ARGS,
    WSL_WRITE_CONF_ARGS,
};

// `AppHandle` is referenced by both platforms (the non-Windows `platform_provision`
// takes one and ignores it); `Emitter` is the trait that actually emits events, so
// it belongs with the Windows implementation that does.
use tauri::AppHandle;
#[cfg(target_os = "windows")]
use tauri::Emitter;

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

use serde::Serialize;
use std::sync::{Mutex, TryLockError};

#[cfg(target_os = "windows")]
use std::{
    env,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::windows::{fs::OpenOptionsExt, process::CommandExt},
    path::PathBuf,
    process::{Command, Stdio},
    thread,
    time::Duration,
};
#[cfg(target_os = "windows")]
use wait_timeout::ChildExt;

// --- Provisioning progress events ---

/// Steps in the provisioning pipeline, emitted as progress events.
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProvisioningStep {
    FetchManifest,
    Verify,
    Parse,
    Decide,
    ResolveBundle,
    DownloadRootfs,
    VerifyDigest,
    ImportWsl,
    ConfigureSystemd,
    RestartWsl,
    InstallDocker,
    PersistState,
}

/// Payload for a provisioning progress event.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ProvisioningProgressPayload {
    pub step: ProvisioningStep,
    /// Human-readable detail for the UI.
    pub detail: String,
}

/// Emits a provisioning progress event if an `AppHandle` is available.
#[cfg(target_os = "windows")]
fn emit_progress(app: Option<&AppHandle>, step: ProvisioningStep, detail: &str) {
    if let Some(handle) = app {
        let _ = handle.emit(
            "provisioning-progress",
            ProvisioningProgressPayload {
                step,
                detail: detail.to_string(),
            },
        );
    }
}

#[cfg(not(target_os = "windows"))]
fn emit_progress(_app: Option<&AppHandle>, _step: ProvisioningStep, _detail: &str) {
    // No-op on non-Windows.
}

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
fn platform_provision(app: Option<AppHandle>) -> RuntimeOperationResult {
    // Step 1: WSL preflight (same as before)
    emit_progress(
        app.as_ref(),
        ProvisioningStep::FetchManifest,
        "Fetching release manifest and signature",
    );
    let status = run_wsl(&fixed_args(WSL_STATUS_ARGS)).outcome;
    let version = if matches!(status, CommandOutcome::Success) {
        read_wsl_version()
    } else {
        None
    };
    let preflight = provisioning_preflight(status, version);
    if preflight.state == OperationState::Blocked {
        return preflight;
    }

    // Step 2: Fetch manifest + signature from update channel
    let (manifest_bytes, signature_bytes) = match fetch_manifest_and_signature() {
        Ok(v) => v,
        Err(e) => {
            return RuntimeOperationResult {
                operation: RuntimeOperation::Provision,
                state: OperationState::Failed,
                detail: Some(format!("failed to fetch release manifest: {e}")),
            };
        }
    };

    // Step 3: Verify signature
    emit_progress(
        app.as_ref(),
        ProvisioningStep::Verify,
        "Verifying manifest signature",
    );
    let verified = match verify_release_manifest(&manifest_bytes, &signature_bytes) {
        Ok(v) => v,
        Err(e) => {
            return RuntimeOperationResult {
                operation: RuntimeOperation::Provision,
                state: OperationState::Failed,
                detail: Some(format!("release manifest verification failed: {e}")),
            };
        }
    };

    // Step 4: Parse manifest
    emit_progress(
        app.as_ref(),
        ProvisioningStep::Parse,
        "Parsing release manifest",
    );
    let manifest: ReleaseManifest = match parse_release_manifest(verified) {
        Ok(m) => m,
        Err(e) => {
            return RuntimeOperationResult {
                operation: RuntimeOperation::Provision,
                state: OperationState::Failed,
                detail: Some(format!("release manifest parse failed: {e}")),
            };
        }
    };

    // Step 5: Load installed state and decide
    emit_progress(
        app.as_ref(),
        ProvisioningStep::Decide,
        "Deciding update action",
    );
    let installed = load_installed_release();
    let decision = decide_update(&manifest, installed);
    let decision_clone = decision; // for match
    let decision_desc = match decision_clone {
        UpdateDecision::Install => "first install",
        UpdateDecision::Upgrade => "upgrade",
        UpdateDecision::RollBackWithinBounds => "rollback within bounds",
        UpdateDecision::AlreadyCurrent => "already current",
        UpdateDecision::Refused(r) => {
            return RuntimeOperationResult {
                operation: RuntimeOperation::Provision,
                state: OperationState::Failed,
                detail: Some(format!("update refused: {}", r.message())),
            };
        }
    };

    // If already current, succeed without mutation
    if matches!(decision, UpdateDecision::AlreadyCurrent) {
        return RuntimeOperationResult {
            operation: RuntimeOperation::Provision,
            state: OperationState::Succeeded,
            detail: Some("already running this release".to_string()),
        };
    }

    // Step 6: Resolve runtime bundle (pinned references)
    emit_progress(
        app.as_ref(),
        ProvisioningStep::ResolveBundle,
        "Resolving runtime bundle",
    );
    let bundle: RuntimeBundle = match resolve_runtime_bundle(&manifest) {
        Ok(b) => b,
        Err(e) => {
            return RuntimeOperationResult {
                operation: RuntimeOperation::Provision,
                state: OperationState::Failed,
                detail: Some(format!("runtime bundle resolution failed: {e}")),
            };
        }
    };

    // Step 7: Download and verify rootfs tarball
    emit_progress(
        app.as_ref(),
        ProvisioningStep::DownloadRootfs,
        "Downloading rootfs tarball",
    );
    let tarball_path = match download_rootfs(&manifest, &bundle) {
        Ok(p) => p,
        Err(e) => {
            return RuntimeOperationResult {
                operation: RuntimeOperation::Provision,
                state: OperationState::Failed,
                detail: Some(format!("rootfs download failed: {e}")),
            };
        }
    };

    // Step 8: Verify digest
    emit_progress(
        app.as_ref(),
        ProvisioningStep::VerifyDigest,
        "Verifying rootfs SHA256 digest",
    );
    // (Digest verification happens inside download_rootfs)

    // Step 9: Import into WSL
    emit_progress(
        app.as_ref(),
        ProvisioningStep::ImportWsl,
        "Importing rootfs into WSL",
    );
    let install_dir = compute_install_dir();
    let import_args = match wsl_import_args(&install_dir, &tarball_path) {
        Ok(args) => args,
        Err(e) => {
            return RuntimeOperationResult {
                operation: RuntimeOperation::Provision,
                state: OperationState::Failed,
                detail: Some(format!("import args validation failed: {e}")),
            };
        }
    };
    let import_result = run_wsl(&import_args);
    if !matches!(import_result.outcome, CommandOutcome::Success) {
        return operation_result(RuntimeOperation::Provision, import_result.outcome);
    }

    // Step 10: Write wsl.conf via tee (stdin) - Configure systemd
    emit_progress(
        app.as_ref(),
        ProvisioningStep::ConfigureSystemd,
        "Writing wsl.conf to enable systemd",
    );
    let write_conf_result = run_wsl_stdin(
        &fixed_args(WSL_WRITE_CONF_ARGS),
        WSL_CONF_CONTENTS.as_bytes(),
    );
    if !matches!(write_conf_result.outcome, CommandOutcome::Success) {
        return operation_result(RuntimeOperation::Provision, write_conf_result.outcome);
    }

    // Step 11: Terminate distro so systemd boots on next start
    emit_progress(
        app.as_ref(),
        ProvisioningStep::RestartWsl,
        "Terminating distro to boot with systemd",
    );
    let terminate_result = run_wsl(&fixed_args(WSL_TERMINATE_ARGS));
    if !matches!(terminate_result.outcome, CommandOutcome::Success) {
        return operation_result(RuntimeOperation::Provision, terminate_result.outcome);
    }

    // Step 12: Persist new installed state
    emit_progress(
        app.as_ref(),
        ProvisioningStep::PersistState,
        "Persisting installed release state",
    );
    let new_floor = advanced_rollback_floor(&manifest, installed);
    let new_installed = InstalledRelease {
        sequence: manifest.release_sequence,
        rollback_floor: new_floor,
    };
    if let Err(e) = persist_installed_release(&new_installed) {
        return RuntimeOperationResult {
            operation: RuntimeOperation::Provision,
            state: OperationState::Failed,
            detail: Some(format!("failed to persist installed state: {e}")),
        };
    }

    // Step 13: Provision Docker inside the distro
    emit_progress(
        app.as_ref(),
        ProvisioningStep::InstallDocker,
        "Installing Docker inside the distro",
    );

    // Step 12: Provision Docker inside the distro
    if let Err(e) = provision_docker_in_distro() {
        return RuntimeOperationResult {
            operation: RuntimeOperation::Provision,
            state: OperationState::Failed,
            detail: Some(format!("docker provisioning failed: {e}")),
        };
    }

    RuntimeOperationResult {
        operation: RuntimeOperation::Provision,
        state: OperationState::Succeeded,
        detail: Some(format!(
            "provisioned {} ({})",
            decision_desc, manifest.release_id
        )),
    }
}

// --- Helpers ---

#[cfg(target_os = "windows")]
fn fetch_manifest_and_signature() -> Result<(Vec<u8>, Vec<u8>), String> {
    let update_url = env::var("RHODIZ_UPDATE_URL")
        .map_err(|_| "RHODIZ_UPDATE_URL environment variable not set".to_string())?;
    let client = reqwest::blocking::Client::new();

    // Fetch manifest
    let manifest_url = format!("{}/release.json", update_url.trim_end_matches('/'));
    let manifest_resp = client
        .get(&manifest_url)
        .send()
        .map_err(|e| format!("manifest GET failed: {e}"))?;
    if !manifest_resp.status().is_success() {
        return Err(format!("manifest HTTP {}", manifest_resp.status()));
    }
    let manifest_bytes = manifest_resp
        .bytes()
        .map_err(|e| format!("manifest read failed: {e}"))?
        .to_vec();

    // Fetch signature
    let sig_url = format!("{}/release.json.sig", update_url.trim_end_matches('/'));
    let sig_resp = client
        .get(&sig_url)
        .send()
        .map_err(|e| format!("signature GET failed: {e}"))?;
    if !sig_resp.status().is_success() {
        return Err(format!("signature HTTP {}", sig_resp.status()));
    }
    let signature_bytes = sig_resp
        .bytes()
        .map_err(|e| format!("signature read failed: {e}"))?
        .to_vec();

    Ok((manifest_bytes, signature_bytes))
}

#[cfg(target_os = "windows")]
fn load_installed_release() -> Option<InstalledRelease> {
    let path = installed_state_path();
    let data = fs::read(&path).ok()?;
    serde_json::from_slice(&data).ok()
}

#[cfg(target_os = "windows")]
fn persist_installed_release(installed: &InstalledRelease) -> Result<(), String> {
    let path = installed_state_path();
    let json = serde_json::to_vec(installed).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())
}

#[cfg(target_os = "windows")]
fn installed_state_path() -> PathBuf {
    let root = PathBuf::from(env::var_os("LOCALAPPDATA").unwrap());
    root.join("RHODIZ").join("installed.json")
}

#[cfg(target_os = "windows")]
fn compute_install_dir() -> String {
    let root = PathBuf::from(env::var_os("LOCALAPPDATA").unwrap());
    root.join("RHODIZ")
        .join("Distro")
        .to_string_lossy()
        .to_string()
}

#[cfg(target_os = "windows")]
fn download_rootfs(manifest: &ReleaseManifest, bundle: &RuntimeBundle) -> Result<String, String> {
    let download_dir = download_dir_path();
    fs::create_dir_all(&download_dir).map_err(|e| e.to_string())?;

    // The manifest does not carry the tarball URL directly; it is expected
    // alongside release.json at the same base URL as "rootfs.tar.gz".
    // This is a convention we control on the producer side.
    let update_url =
        env::var("RHODIZ_UPDATE_URL").map_err(|_| "RHODIZ_UPDATE_URL not set".to_string())?;
    let base = update_url.trim_end_matches('/');
    let tarball_url = format!("{}/rootfs.tar.gz", base);

    let tarball_path = download_dir.join("rootfs.tar.gz");
    let client = reqwest::blocking::Client::new();
    let mut resp = client
        .get(&tarball_url)
        .send()
        .map_err(|e| format!("rootfs GET failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("rootfs HTTP {}", resp.status()));
    }

    let mut file = File::create(&tarball_path).map_err(|e| e.to_string())?;
    let bytes = resp
        .bytes()
        .map_err(|e| format!("rootfs read failed: {e}"))?;
    file.write_all(&bytes).map_err(|e| e.to_string())?;

    // Verify SHA256 of downloaded tarball against compose_sha256
    let computed = sha256_hex(&tarball_path)?;
    if computed != bundle.compose_sha256() {
        let _ = fs::remove_file(&tarball_path);
        return Err(format!(
            "rootfs sha256 mismatch: expected {}, got {}",
            bundle.compose_sha256(),
            computed
        ));
    }

    Ok(tarball_path.to_string_lossy().to_string())
}

#[cfg(target_os = "windows")]
fn download_dir_path() -> PathBuf {
    let root = PathBuf::from(env::var_os("LOCALAPPDATA").unwrap());
    root.join("RHODIZ").join("Downloads")
}

#[cfg(target_os = "windows")]
fn sha256_hex(path: &PathBuf) -> Result<String, String> {
    use sha2::{Digest, Sha256};
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = file.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

#[cfg(target_os = "windows")]
fn run_wsl_stdin(args: &[String], stdin_bytes: &[u8]) -> ProcessCapture {
    let Some(executable) = wsl_executable_path() else {
        return ProcessCapture {
            outcome: CommandOutcome::SpawnFailed,
            stdout: Vec::new(),
            truncated: false,
        };
    };

    let mut child = match Command::new(executable)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
    {
        Ok(c) => c,
        Err(_) => {
            return ProcessCapture {
                outcome: CommandOutcome::SpawnFailed,
                stdout: Vec::new(),
                truncated: false,
            };
        }
    };

    // Write stdin
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(stdin_bytes);
    }

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

#[cfg(target_os = "windows")]
fn provision_docker_in_distro() -> Result<(), String> {
    // Script to install Docker Engine and Compose plugin inside the distro.
    // Runs as root via wsl.exe --exec.
    let script = r#"
set -euo pipefail
apt-get update -y
apt-get install -y ca-certificates curl gnupg lsb-release
install -m 0755 -d /etc/apt/keyrings
curl -fsSL https://download.docker.com/linux/debian/gpg | gpg --dearmor -o /etc/apt/keyrings/docker.gpg
chmod a+r /etc/apt/keyrings/docker.gpg
echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/debian $(lsb_release -cs) stable" > /etc/apt/sources.list.d/docker.list
apt-get update -y
apt-get install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin
systemctl enable docker
"#;

    let args = vec![
        "--distribution".to_string(),
        MANAGED_DISTRO_NAME.to_string(),
        "--user".to_string(),
        "root".to_string(),
        "--exec".to_string(),
        "bash".to_string(),
        "-c".to_string(),
        script.to_string(),
    ];

    let result = run_wsl(&args);
    if !matches!(result.outcome, CommandOutcome::Success) {
        return Err(format!(
            "docker install script exited: {:?}",
            result.outcome
        ));
    }
    Ok(())
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
fn platform_provision(_app: Option<AppHandle>) -> RuntimeOperationResult {
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
pub fn runtime_provision(app: AppHandle) -> RuntimeOperationResult {
    with_lifecycle_lock(RuntimeOperation::Provision, move || {
        platform_provision(Some(app))
    })
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
