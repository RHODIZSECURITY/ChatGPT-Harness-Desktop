use rhodiz_harness_broker_core::{
    classify_wsl_status, lifecycle_busy, lifecycle_lock_unavailable, operation_result,
    runtime_status as build_status, sanitize_log_text, CommandOutcome, OperationState,
    RuntimeLogsResult, RuntimeOperation, RuntimeOperationResult, RuntimeStatus,
    RuntimeVerifyResult, COMMAND_TIMEOUT_SECS, MAX_CAPTURE_BYTES, WSL_EXE, WSL_STATUS_ARGS,
};

// The provisioning preflight only runs where there is a `wsl.exe` to probe;
// the fallback below never reaches it.
#[cfg(target_os = "windows")]
use rhodiz_harness_broker_core::{
    advanced_rollback_floor, decide_update, decode_utf16le, extract_wsl_version,
    parse_release_manifest, provisioning_preflight, resolve_runtime_bundle, runtime_log_args,
    verify_release_manifest, wsl_import_args, wsl_start_args, wsl_stop_args, wsl_terminate_args,
    wsl_write_conf_args, DistroSlot, InstalledRelease, ReleaseManifest, RuntimeBundle,
    UpdateDecision, MANAGED_DISTRO_NAME, MANIFEST_SIGNATURE_LEN, MAX_MANIFEST_BYTES,
    WSL_CONF_CONTENTS, WSL_VERSION_ARGS,
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
    verify_result, wsl_repair_reset_args, wsl_repair_restart_args, wsl_verify_args,
};

#[cfg(target_os = "windows")]
use rhodiz_harness_broker_core::{
    lifecycle_cross_process_busy, LIFECYCLE_LOCK_DIRECTORY, LIFECYCLE_LOCK_FILE,
};

use serde::{Deserialize, Serialize};
use std::sync::{Mutex, TryLockError};

#[cfg(target_os = "windows")]
use std::{
    env,
    fs::{self, File, OpenOptions},
    io::{self, ErrorKind, Read, Write},
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

// Every lifecycle command is aimed at whichever slot is serving right now,
// which is a fact that lives on disk rather than in the binary. Reading it
// per call rather than caching it means a slot swap by a concurrent
// provision is picked up by the next command instead of being shadowed by a
// value read at startup.
#[cfg(target_os = "windows")]
fn fixed_start_args() -> Vec<String> {
    fixed_args(wsl_start_args(active_slot()))
}

#[cfg(target_os = "windows")]
fn fixed_stop_args() -> Vec<String> {
    fixed_args(wsl_stop_args(active_slot()))
}

// There is no `wsl.exe` to aim at off Windows, and `platform_operation`
// discards the vector there rather than spawning anything. Building a real
// one would mean reading a state file that the fallback never writes.
#[cfg(not(target_os = "windows"))]
fn fixed_start_args() -> Vec<String> {
    Vec::new()
}

#[cfg(not(target_os = "windows"))]
fn fixed_stop_args() -> Vec<String> {
    Vec::new()
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
    verify_result(run_wsl(&fixed_args(wsl_verify_args(active_slot()))).outcome)
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
    let slot = active_slot();
    let _ = run_wsl(&fixed_args(wsl_repair_reset_args(slot)));
    operation_result(
        RuntimeOperation::Repair,
        run_wsl(&fixed_args(wsl_repair_restart_args(slot))).outcome,
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
    // Step 1: WSL preflight
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
    emit_progress(
        app.as_ref(),
        ProvisioningStep::FetchManifest,
        "Fetching release manifest and signature",
    );
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
    // An unreadable state file is not an empty one. `decide_update` reads the
    // anti-rollback floor out of this value, and `None` tells it there is no
    // floor to clear, so swallowing a read or parse failure here would let any
    // genuinely signed release through no matter how old -- which is the
    // replay attack the floor exists to stop. Refusing is the safe answer:
    // provisioning stops, and a machine that cannot read its own state was not
    // going to provision correctly anyway.
    let installed = match load_installed_release() {
        Ok(v) => v,
        Err(e) => {
            return RuntimeOperationResult {
                operation: RuntimeOperation::Provision,
                state: OperationState::Failed,
                detail: Some(format!(
                    "refusing to provision: cannot establish the anti-rollback floor ({e})"
                )),
            };
        }
    };
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
    let tarball_path = match download_rootfs(&bundle) {
        Ok(p) => p,
        Err(e) => {
            return RuntimeOperationResult {
                operation: RuntimeOperation::Provision,
                state: OperationState::Failed,
                detail: Some(format!("rootfs download failed: {e}")),
            };
        }
    };

    // Step 8: Report the digest check `download_rootfs` already performed.
    //
    // Past tense on purpose: the comparison happens inside the download, so by
    // the time this fires there is nothing left to verify. The wording also
    // deliberately does not claim the rootfs was verified, because it was not
    // -- see `download_rootfs` for what that digest actually covers. A UI
    // reading "rootfs verified" here would be reporting a guarantee the signed
    // manifest does not yet provide.
    emit_progress(
        app.as_ref(),
        ProvisioningStep::VerifyDigest,
        "Checked the downloaded tarball against the digest the manifest carries",
    );

    // Step 9: Import into WSL
    emit_progress(
        app.as_ref(),
        ProvisioningStep::ImportWsl,
        "Importing rootfs into WSL",
    );
    // Every distro-scoped command below names this slot rather than a fixed
    // distro. It is still the primary slot: staging the import into the
    // inactive slot needs the lifecycle commands to resolve the active slot
    // too, which they do not yet, and provisioning into a slot the rest of
    // the broker cannot address would be worse than not staging at all.
    let target_slot = DistroSlot::INITIAL;
    let install_dir = match compute_install_dir() {
        Ok(dir) => dir,
        Err(e) => {
            return RuntimeOperationResult {
                operation: RuntimeOperation::Provision,
                state: OperationState::Failed,
                detail: Some(format!("cannot determine the install directory: {e}")),
            };
        }
    };
    let import_args = match wsl_import_args(target_slot, &install_dir, &tarball_path) {
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
        &fixed_args(wsl_write_conf_args(target_slot)),
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
    let terminate_result = run_wsl(&fixed_args(wsl_terminate_args(target_slot)));
    if !matches!(terminate_result.outcome, CommandOutcome::Success) {
        return operation_result(RuntimeOperation::Provision, terminate_result.outcome);
    }

    // Step 12: Provision Docker inside the distro
    //
    // Ahead of persisting state, not after it. State is the record that this
    // release is installed and serving; writing it before the install is
    // finished would leave a failed provision claiming success, and the
    // recorded sequence would then raise the anti-rollback floor on behalf of
    // a distro that has no working Docker in it. The next provision would be
    // measured against that floor.
    emit_progress(
        app.as_ref(),
        ProvisioningStep::InstallDocker,
        "Installing Docker inside the distro",
    );
    if let Err(e) = provision_docker_in_distro() {
        return RuntimeOperationResult {
            operation: RuntimeOperation::Provision,
            state: OperationState::Failed,
            detail: Some(format!("docker provisioning failed: {e}")),
        };
    }

    // Step 13: Persist new installed state
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
    // Provisioning still imports into the primary slot, so that is the slot
    // this install leaves serving. Staging into the inactive slot is the next
    // change; recording the slot now is what lets the lifecycle commands
    // follow it when it starts to move.
    if let Err(e) = persist_installed_release(&new_installed, DistroSlot::INITIAL) {
        return RuntimeOperationResult {
            operation: RuntimeOperation::Provision,
            state: OperationState::Failed,
            detail: Some(format!("failed to persist installed state: {e}")),
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
    let client = manifest_client()?;

    // Fetch manifest
    let manifest_url = format!("{}/release.json", update_url.trim_end_matches('/'));
    let manifest_resp = client
        .get(&manifest_url)
        .send()
        .map_err(|e| format!("manifest GET failed: {e}"))?;
    if !manifest_resp.status().is_success() {
        return Err(format!("manifest HTTP {}", manifest_resp.status()));
    }
    let manifest_bytes = read_body_bounded(manifest_resp, MAX_MANIFEST_BYTES, "manifest")?;

    // Fetch signature
    let sig_url = format!("{}/release.json.sig", update_url.trim_end_matches('/'));
    let sig_resp = client
        .get(&sig_url)
        .send()
        .map_err(|e| format!("signature GET failed: {e}"))?;
    if !sig_resp.status().is_success() {
        return Err(format!("signature HTTP {}", sig_resp.status()));
    }
    let signature_bytes = read_body_bounded(sig_resp, MANIFEST_SIGNATURE_LEN, "signature")?;

    Ok((manifest_bytes, signature_bytes))
}

/// The broker's own on-disk schema.
///
/// Deliberately not `InstalledRelease` itself: that type is what
/// `decide_update` reasons about, and the slot is not part of a release's
/// identity. What is installed and where it is installed are separate facts,
/// and only the second one is a property of this machine. Keeping the schema
/// here also means the file format is versioned next to the code that reads
/// and writes it, rather than riding on a core type that has no stake in it.
#[cfg(target_os = "windows")]
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PersistedState {
    sequence: u64,
    rollback_floor: u64,
    /// Absent from state files written before slots existed. Such a file
    /// describes an install that went into the primary slot, which is what
    /// `INITIAL` is, so defaulting reads it faithfully rather than guessing.
    ///
    /// Without the default, serde treats the missing field as a parse error,
    /// and `load_persisted_state` turns a parse error into a refusal to
    /// provision -- so every pre-slot install would become a machine that
    /// cannot update. Defaulting is not a way around that refusal: it applies
    /// only where the old file's meaning is known exactly, which leaves
    /// nothing to refuse.
    #[serde(default = "initial_slot")]
    active_slot: DistroSlot,
}

#[cfg(target_os = "windows")]
fn initial_slot() -> DistroSlot {
    DistroSlot::INITIAL
}

/// Reads the state file, distinguishing "nothing is installed" from "cannot
/// tell what is installed".
///
/// Only a missing file is `Ok(None)`: that is the genuine pre-first-install
/// state. A file that exists but will not read or will not parse is an error,
/// because the alternative -- reporting it as absent -- makes a corrupt state
/// file indistinguishable from a fresh machine, and callers draw security
/// conclusions from that difference.
#[cfg(target_os = "windows")]
fn load_persisted_state() -> Result<Option<PersistedState>, String> {
    let path = installed_state_path()?;
    let data = match fs::read(&path) {
        Ok(data) => data,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(format!(
                "cannot read installed state at {}: {e}",
                path.display()
            ))
        }
    };
    serde_json::from_slice(&data)
        .map(Some)
        .map_err(|e| format!("installed state at {} does not parse: {e}", path.display()))
}

#[cfg(target_os = "windows")]
fn load_installed_release() -> Result<Option<InstalledRelease>, String> {
    Ok(load_persisted_state()?.map(|state| InstalledRelease {
        sequence: state.sequence,
        rollback_floor: state.rollback_floor,
    }))
}

/// The slot every lifecycle command is aimed at.
///
/// Absent state answers `INITIAL`: before the first provision there is no
/// file, and the primary slot is where a first install lands, so that is the
/// only slot a command could be talking about.
///
/// Unreadable state also answers `INITIAL`, and that is a knowing compromise
/// rather than a safe default. Today provisioning always imports into the
/// primary slot, so the secondary distro never exists and the fallback cannot
/// name the wrong one. That stops being true the moment the swap lands: a
/// corrupt state file would then aim `start` at a stale distro that is still
/// on disk. The fix belongs with the swap -- the lifecycle commands need to
/// carry a failure out to the renderer, which means threading `Result` through
/// all five of them -- and doing it here first would be a signature change
/// with nothing yet able to trigger the bug it guards.
#[cfg(target_os = "windows")]
fn active_slot() -> DistroSlot {
    load_persisted_state()
        .ok()
        .flatten()
        .map_or(DistroSlot::INITIAL, |state| state.active_slot)
}

#[cfg(target_os = "windows")]
fn persist_installed_release(
    installed: &InstalledRelease,
    active_slot: DistroSlot,
) -> Result<(), String> {
    let path = installed_state_path()?;
    // A first install writes into a directory no one has created yet.
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_vec(&PersistedState {
        sequence: installed.sequence,
        rollback_floor: installed.rollback_floor,
        active_slot,
    })
    .map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())
}

/// The per-user root every broker-owned path hangs off.
///
/// `LOCALAPPDATA` comes from the session, not from us, so a stripped or
/// service-side environment can arrive without it. Unwrapping there panics
/// inside a Tauri command handler, where the renderer sees an IPC call that
/// simply never returns and has no way to say why. Returning the failure lets
/// each caller report it as the operation failing, which is what it is.
#[cfg(target_os = "windows")]
fn local_app_data() -> Result<PathBuf, String> {
    env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .ok_or_else(|| "LOCALAPPDATA is not set in this environment".to_string())
}

#[cfg(target_os = "windows")]
fn installed_state_path() -> Result<PathBuf, String> {
    Ok(local_app_data()?.join("RHODIZ").join("installed.json"))
}

#[cfg(target_os = "windows")]
fn compute_install_dir() -> Result<String, String> {
    Ok(local_app_data()?
        .join("RHODIZ")
        .join("Distro")
        .to_string_lossy()
        .to_string())
}

#[cfg(target_os = "windows")]
fn download_rootfs(bundle: &RuntimeBundle) -> Result<String, String> {
    let download_dir = download_dir_path()?;
    fs::create_dir_all(&download_dir).map_err(|e| e.to_string())?;

    // The manifest does not carry the tarball URL directly; it is expected
    // alongside release.json at the same base URL as "rootfs.tar.gz".
    // This is a convention we control on the producer side.
    let update_url =
        env::var("RHODIZ_UPDATE_URL").map_err(|_| "RHODIZ_UPDATE_URL not set".to_string())?;
    let base = update_url.trim_end_matches('/');
    let tarball_url = format!("{}/rootfs.tar.gz", base);

    let tarball_path = download_dir.join("rootfs.tar.gz");
    let mut resp = rootfs_client()?
        .get(&tarball_url)
        .send()
        .map_err(|e| format!("rootfs GET failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("rootfs HTTP {}", resp.status()));
    }

    // Streamed, not buffered. Reading the body into a `Vec` first puts an
    // entire rootfs in memory, and the length is whatever the server chooses
    // to send -- a hostile or broken one can exhaust RAM before a single byte
    // is checked. `io::copy` moves it in fixed-size chunks instead.
    //
    // `take` is the ceiling on what reaches the disk. Without it the same
    // server fills the volume, and the digest comparison below cannot help:
    // it only runs once the write has finished. The bound is not a claim
    // about how large a real rootfs is -- it is an order of magnitude above
    // any plausible one -- so hitting it means the response is wrong, and the
    // partial file is removed rather than left to be imported.
    let mut file = File::create(&tarball_path).map_err(|e| e.to_string())?;
    let copied = io::copy(&mut (&mut resp).take(MAX_ROOTFS_BYTES + 1), &mut file)
        .map_err(|e| format!("rootfs write failed: {e}"))?;
    drop(file);
    if copied > MAX_ROOTFS_BYTES {
        let _ = fs::remove_file(&tarball_path);
        return Err(format!(
            "rootfs exceeds its {MAX_ROOTFS_BYTES}-byte ceiling"
        ));
    }

    // NOT a rootfs integrity check. `compose_sha256` is the digest of the
    // compose file, and the manifest schema carries no digest for the rootfs
    // tarball at all, so nothing here ties these bytes to anything the
    // release key signed. It is left in place because removing it would make
    // the gap invisible; closing it needs a `rootfs_sha256` field in the
    // signed manifest, which is a schema change, not a change here.
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

/// Reads a response body, refusing one larger than `limit`.
///
/// Deliberately not `read_bounded`, which this file already has: that one
/// serves log capture, where truncating an over-long `wsl.exe` transcript and
/// flagging it is the right answer. Truncating a manifest is not -- a prefix
/// of a signed document is not a shorter signed document, and the caller has
/// no use for one. So this refuses instead of trimming.
///
/// The ceiling has to be applied here, at the read, not by the code that
/// consumes the bytes. `verify_release_manifest` does enforce
/// `MAX_MANIFEST_BYTES`, but only once it has been handed a `Vec` that is
/// already in memory -- so a server answering the manifest URL with an endless
/// body exhausts RAM before a single check runs. `take` bounds the read
/// itself, and asking for one byte past the limit is what makes "too large"
/// distinguishable from "exactly at the limit".
#[cfg(target_os = "windows")]
fn read_body_bounded(
    mut resp: reqwest::blocking::Response,
    limit: usize,
    what: &str,
) -> Result<Vec<u8>, String> {
    let mut buf = Vec::new();
    (&mut resp)
        .take(limit as u64 + 1)
        .read_to_end(&mut buf)
        .map_err(|e| format!("{what} read failed: {e}"))?;
    if buf.len() > limit {
        return Err(format!("{what} exceeds its {limit}-byte ceiling"));
    }
    Ok(buf)
}

/// Ceiling on the rootfs tarball, in bytes.
///
/// Deliberately far above any real WSL rootfs. It exists so that a server
/// which streams without end is refused instead of filling the disk, not to
/// express an expected size -- the signed manifest carries no size for the
/// tarball, so there is nothing authoritative to compare against.
#[cfg(target_os = "windows")]
const MAX_ROOTFS_BYTES: u64 = 8 * 1024 * 1024 * 1024;

/// How long the broker waits to reach the update host.
///
/// Connecting is the one phase with a bounded, predictable cost, so it gets a
/// deadline regardless of what is being fetched.
#[cfg(target_os = "windows")]
const HTTP_CONNECT_TIMEOUT_SECS: u64 = 10;

/// Total deadline for the manifest and its signature.
///
/// Both are small and fixed-size -- `MAX_MANIFEST_BYTES` and
/// `MANIFEST_SIGNATURE_LEN` -- so a whole-request deadline is safe here: it
/// cannot cut short a legitimate transfer.
#[cfg(target_os = "windows")]
const MANIFEST_HTTP_TIMEOUT_SECS: u64 = 30;

#[cfg(target_os = "windows")]
fn manifest_client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(HTTP_CONNECT_TIMEOUT_SECS))
        .timeout(Duration::from_secs(MANIFEST_HTTP_TIMEOUT_SECS))
        .build()
        .map_err(|e| format!("cannot build the update client: {e}"))
}

/// The client for the rootfs body.
///
/// Connect is bounded; the transfer is not. A whole-request deadline would
/// cap how large a rootfs may legitimately be, or how slow a link may be, and
/// `reqwest`'s blocking builder exposes no per-read inactivity timeout to use
/// instead. So a server that trickles bytes indefinitely still stalls
/// provisioning -- a known gap, bounded only by `MAX_ROOTFS_BYTES`, and one
/// the operator can still interrupt by closing the app. Fixing it properly
/// needs an inactivity deadline the blocking API does not offer.
#[cfg(target_os = "windows")]
fn rootfs_client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(HTTP_CONNECT_TIMEOUT_SECS))
        .build()
        .map_err(|e| format!("cannot build the update client: {e}"))
}

#[cfg(target_os = "windows")]
fn download_dir_path() -> Result<PathBuf, String> {
    Ok(local_app_data()?.join("RHODIZ").join("Downloads"))
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
    let args = runtime_log_args(active_slot(), lines);
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
