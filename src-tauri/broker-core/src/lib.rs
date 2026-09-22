use serde::{Deserialize, Serialize};

/// Detached-signature verification for the release manifest. It lives in its
/// own module because the ordering it enforces — verify the exact bytes
/// before anything parses them — is a property of that module's types, and
/// keeping both the handle's constructor and the key-taking verifier private
/// to it is what stops a caller here from choosing its own trust anchor.
mod manifest;

/// Parsing of a manifest that has already been verified. Separate from
/// `manifest` so the verifier stays readable on its own.
mod release;

pub use manifest::{
    verify_release_manifest, ManifestVerifyError, VerifiedManifestBytes, MANIFEST_PUBLIC_KEY_LEN,
    MANIFEST_SIGNATURE_LEN, MAX_MANIFEST_BYTES,
};
/// Re-exported deliberately narrowly. `verify_release_manifest` is the only
/// entry point: it consults the anchor pinned at build time, and there is no
/// way from outside the module to verify against any other key. The two
/// length constants are public because the error documentation refers to
/// them, not because a caller needs to build a key or a signature by hand.
pub use release::{
    advanced_rollback_floor, decide_update, parse_release_manifest, resolve_runtime_bundle,
    BundleError, Compatibility, DesktopRelease, ImageRef, InstalledRelease, ManifestParseError,
    ProviderRelease, ReleaseManifest, RuntimeBundle, RuntimeRelease, UpdateDecision, UpdateRefusal,
    SUPPORTED_SCHEMA_VERSION,
};

/// The first distro name. A first install lands here; see `DistroSlot`.
pub const MANAGED_DISTRO_NAME: &str = "RHODIZ-Harness";

/// The second distro name, used to prepare a release without touching the one
/// currently serving the operator.
pub const STAGING_DISTRO_NAME: &str = "RHODIZ-Harness-Next";

/// Which of the two distro names a `wsl.exe` command is aimed at.
///
/// A transactional update never writes over the distro that is currently
/// serving the operator: the new rootfs is imported and fully configured
/// under the inactive name, and only once that has succeeded does the active
/// name move. `wsl.exe` exposes no rename primitive, so "swap" here means
/// recording which name is active — not moving a distro between names. The
/// two names are therefore slots that alternate across updates, and neither
/// one is permanently "the production distro".
///
/// The serde spelling is the variant name, not its position: a slot recorded
/// on disk as `0`/`1` would silently point at the wrong distro if the variants
/// were ever reordered, and the wrong distro here is the live one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DistroSlot {
    /// `MANAGED_DISTRO_NAME`. The slot a first install lands in.
    Primary,
    /// `STAGING_DISTRO_NAME`.
    Secondary,
}

impl DistroSlot {
    /// The slot a first install lands in, before any update has alternated.
    pub const INITIAL: Self = Self::Primary;

    pub const fn distro_name(self) -> &'static str {
        match self {
            Self::Primary => MANAGED_DISTRO_NAME,
            Self::Secondary => STAGING_DISTRO_NAME,
        }
    }

    /// The slot an update must be staged in, given the slot serving now.
    ///
    /// An involution, so staging is always the one name that is not live and
    /// the caller cannot arrive at the live one by iterating.
    pub const fn other(self) -> Self {
        match self {
            Self::Primary => Self::Secondary,
            Self::Secondary => Self::Primary,
        }
    }
}
pub const RUNTIME_BOOTSTRAP_UNIT: &str = "rhodiz-harness-bootstrap.service";
pub const WSL_EXE: &str = "wsl.exe";
pub const WSL_STATUS_ARGS: [&str; 1] = ["--status"];
/// Every distro-scoped systemctl vector, which differ only in the verb.
///
/// A `const fn` returning a fixed-length array of `&'static str`: the vectors
/// stay allocation-free and as auditable as the constants they replaced, and
/// the slot is the only thing a caller can vary.
const fn systemctl_args(slot: DistroSlot, verb: &'static str) -> [&'static str; 8] {
    [
        "--distribution",
        slot.distro_name(),
        "--user",
        "root",
        "--exec",
        "systemctl",
        verb,
        RUNTIME_BOOTSTRAP_UNIT,
    ]
}

pub const fn wsl_start_args(slot: DistroSlot) -> [&'static str; 8] {
    systemctl_args(slot, "start")
}

pub const fn wsl_stop_args(slot: DistroSlot) -> [&'static str; 8] {
    systemctl_args(slot, "stop")
}

/// Read-only probe. `systemctl is-active` mutates nothing, so verification can
/// run while a lifecycle mutation holds the lock.
pub const fn wsl_verify_args(slot: DistroSlot) -> [&'static str; 8] {
    systemctl_args(slot, "is-active")
}

/// Clears a latched failure so the restart below is not refused by a
/// start-limit counter. Best-effort: see `wsl_repair_restart_args`.
pub const fn wsl_repair_reset_args(slot: DistroSlot) -> [&'static str; 8] {
    systemctl_args(slot, "reset-failed")
}

/// The step whose outcome decides the repair result.
pub const fn wsl_repair_restart_args(slot: DistroSlot) -> [&'static str; 8] {
    systemctl_args(slot, "restart")
}

pub const fn wsl_log_args_prefix(slot: DistroSlot) -> [&'static str; 11] {
    [
        "--distribution",
        slot.distro_name(),
        "--user",
        "root",
        "--exec",
        "journalctl",
        "--unit",
        RUNTIME_BOOTSTRAP_UNIT,
        "--no-pager",
        "--output=short-iso",
        "--lines",
    ]
}

pub const DEFAULT_LOG_LINES: u16 = 200;
pub const MAX_LOG_LINES: u16 = 500;
pub const MAX_LOG_LINE_CHARS: usize = 2_048;
pub const COMMAND_TIMEOUT_SECS: u64 = 15;

/// Location of the cross-process lifecycle lock, under `%LOCALAPPDATA%`.
pub const LIFECYCLE_LOCK_DIRECTORY: &str = "com.rhodiz.harness.desktop";
pub const LIFECYCLE_LOCK_FILE: &str = "runtime-lifecycle.lock";
pub const MAX_CAPTURE_BYTES: usize = 2 * 1024 * 1024;

/// Matched anywhere in a log line. Each reads as a word, so a substring test
/// does not produce meaningful false positives.
const SENSITIVE_LOG_PHRASES: [&str; 13] = [
    "authorization",
    "bearer ",
    "token=",
    "token:",
    "password",
    "secret",
    "api_key",
    "api-key",
    "workspace_lease",
    "private_key",
    "client_secret",
    "access_key",
    "-----begin",
];

/// Credential shapes recognised by how a token *starts*. Matching these as bare
/// substrings would redact ordinary operational lines: `sk-` occurs inside
/// `disk-usage`, `task-runner` and `risk-report`, and `akia` inside any word
/// that happens to contain it.
const SENSITIVE_TOKEN_PREFIXES: [&str; 6] = [
    "ghp_",
    "github_pat_",
    "sk-",
    "akia",
    "xoxb-",
    // A JWT always starts with the base64 of `{"`, so any bearer-style token
    // pasted into a log line is caught even without a labelling phrase.
    "eyj",
];

/// Splits on every character that cannot appear inside a credential token, so a
/// prefix only counts at a real token boundary. `key="sk-abc"` still yields the
/// bare token `sk-abc`, while `disk-usage` stays a single token that does not
/// start with `sk-`.
fn has_sensitive_token_prefix(normalized: &str) -> bool {
    normalized
        .split(|character: char| {
            !(character.is_ascii_alphanumeric() || character == '-' || character == '_')
        })
        .any(|token| {
            SENSITIVE_TOKEN_PREFIXES
                .iter()
                .any(|prefix| token.starts_with(prefix))
        })
}

fn is_sensitive_log_line(normalized: &str) -> bool {
    SENSITIVE_LOG_PHRASES
        .iter()
        .any(|phrase| normalized.contains(phrase))
        || has_sensitive_token_prefix(normalized)
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ComponentState {
    Ready,
    Stopped,
    Missing,
    Unavailable,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct RuntimeComponent {
    pub state: ComponentState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct RuntimeStatus {
    pub platform: &'static str,
    pub wsl2: RuntimeComponent,
    pub docker: RuntimeComponent,
    pub core: RuntimeComponent,
    pub route: RuntimeComponent,
    pub memory: RuntimeComponent,
    pub providers: RuntimeComponent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandOutcome {
    Success,
    Exit(i32),
    SpawnFailed,
    TimedOut,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeOperation {
    Provision,
    Verify,
    Repair,
    Start,
    Stop,
    Logs,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OperationState {
    Succeeded,
    Failed,
    Blocked,
    TimedOut,
    Unsupported,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct RuntimeOperationResult {
    pub operation: RuntimeOperation,
    pub state: OperationState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct RuntimeLogsResult {
    pub state: OperationState,
    pub lines: Vec<String>,
    pub truncated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// `systemctl is-active` reports an inactive, failed or unknown unit with this
/// exit code. Any *other* non-zero code did not come from `systemctl` at all: it
/// came from `wsl.exe` failing to reach the distribution, which is a different
/// fault with a different remedy.
///
/// **Assumption, not measured evidence.** This mapping has not been exercised
/// against a real `wsl.exe`; confirming it belongs to Windows certification.
pub const SYSTEMCTL_INACTIVE_EXIT_CODE: i32 = 3;

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UnitState {
    /// The bootstrap unit is running.
    Active,
    /// The distribution answered, but the unit is not running.
    Inactive,
    /// `wsl.exe` ran but could not reach the managed distribution.
    DistroUnreachable,
    /// `wsl.exe` itself could not be launched.
    WslMissing,
    /// The probe exceeded its time budget, so the state is unknown.
    Unknown,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct RuntimeVerifyResult {
    pub operation: RuntimeOperation,
    pub unit: UnitState,
    /// Whether every check the broker can perform today came back healthy. A
    /// state the broker cannot determine is never healthy.
    pub healthy: bool,
    pub detail: String,
    /// Components whose typed probes do not exist yet. Reported so the renderer
    /// cannot mistake "not probed" for "verified healthy".
    pub unprobed: Vec<&'static str>,
}

/// Components that verification deliberately does not inspect, because the code
/// that would install them does not exist yet (plan tasks 4.3 and 4.4).
pub const UNPROBED_COMPONENTS: [&str; 5] = ["docker", "core", "route", "memory", "providers"];

pub fn classify_unit_probe(outcome: CommandOutcome) -> (UnitState, String) {
    match outcome {
        CommandOutcome::Success => (
            UnitState::Active,
            format!("{RUNTIME_BOOTSTRAP_UNIT} is active in {MANAGED_DISTRO_NAME}"),
        ),
        CommandOutcome::Exit(SYSTEMCTL_INACTIVE_EXIT_CODE) => (
            UnitState::Inactive,
            format!(
                "{MANAGED_DISTRO_NAME} responded but {RUNTIME_BOOTSTRAP_UNIT} is not running; \
                 repair or start the managed runtime"
            ),
        ),
        CommandOutcome::Exit(code) => (
            UnitState::DistroUnreachable,
            format!(
                "the {MANAGED_DISTRO_NAME} distribution could not be reached (exit code {code}); \
                 provision the managed runtime"
            ),
        ),
        CommandOutcome::SpawnFailed => (
            UnitState::WslMissing,
            "wsl.exe could not be launched; install or enable WSL2".to_string(),
        ),
        CommandOutcome::TimedOut => (
            UnitState::Unknown,
            "the managed runtime probe timed out; state is unknown".to_string(),
        ),
    }
}

pub fn verify_result(outcome: CommandOutcome) -> RuntimeVerifyResult {
    let (unit, detail) = classify_unit_probe(outcome);
    RuntimeVerifyResult {
        operation: RuntimeOperation::Verify,
        healthy: unit == UnitState::Active,
        unit,
        detail,
        unprobed: UNPROBED_COMPONENTS.to_vec(),
    }
}

/// Verification on a platform that has no managed runtime at all.
pub fn unsupported_verify() -> RuntimeVerifyResult {
    RuntimeVerifyResult {
        operation: RuntimeOperation::Verify,
        unit: UnitState::Unknown,
        healthy: false,
        detail: "managed runtime verification is available only on Windows".to_string(),
        unprobed: UNPROBED_COMPONENTS.to_vec(),
    }
}

pub fn classify_wsl_status(outcome: CommandOutcome) -> RuntimeComponent {
    match outcome {
        CommandOutcome::Success => RuntimeComponent {
            state: ComponentState::Ready,
            detail: None,
        },
        CommandOutcome::Exit(code) => RuntimeComponent {
            state: ComponentState::Stopped,
            detail: Some(format!("wsl status exit code {code}")),
        },
        CommandOutcome::SpawnFailed => RuntimeComponent {
            state: ComponentState::Missing,
            detail: None,
        },
        CommandOutcome::TimedOut => RuntimeComponent {
            state: ComponentState::Unavailable,
            detail: Some("wsl status timed out".to_string()),
        },
    }
}

pub fn operation_result(
    operation: RuntimeOperation,
    outcome: CommandOutcome,
) -> RuntimeOperationResult {
    let (state, detail) = match outcome {
        CommandOutcome::Success => (OperationState::Succeeded, None),
        CommandOutcome::Exit(code) => (
            OperationState::Failed,
            Some(format!("managed runtime command exit code {code}")),
        ),
        CommandOutcome::SpawnFailed => (
            OperationState::Failed,
            Some("managed runtime command could not start".to_string()),
        ),
        CommandOutcome::TimedOut => (
            OperationState::TimedOut,
            Some("managed runtime command timed out".to_string()),
        ),
    };
    RuntimeOperationResult {
        operation,
        state,
        detail,
    }
}

pub fn unsupported_operation(operation: RuntimeOperation) -> RuntimeOperationResult {
    RuntimeOperationResult {
        operation,
        state: OperationState::Unsupported,
        detail: Some("managed runtime lifecycle is available only on Windows".to_string()),
    }
}

/// Another lifecycle operation holds the lock inside this process.
pub fn lifecycle_busy(operation: RuntimeOperation) -> RuntimeOperationResult {
    RuntimeOperationResult {
        operation,
        state: OperationState::Blocked,
        detail: Some("another managed runtime lifecycle operation is active".to_string()),
    }
}

/// The lock could not be taken across processes. This covers both a second
/// application process holding it and a lock file that cannot be created, so it
/// fails closed in either case.
pub fn lifecycle_cross_process_busy(operation: RuntimeOperation) -> RuntimeOperationResult {
    RuntimeOperationResult {
        operation,
        state: OperationState::Blocked,
        detail: Some(
            "the cross-process managed runtime lifecycle lock could not be acquired".to_string(),
        ),
    }
}

/// The in-process lock was poisoned by a panic while held.
pub fn lifecycle_lock_unavailable(operation: RuntimeOperation) -> RuntimeOperationResult {
    RuntimeOperationResult {
        operation,
        state: OperationState::Failed,
        detail: Some("managed runtime lifecycle lock is unavailable".to_string()),
    }
}

/// The broker could not tell which distro slot is live.
///
/// Every distro-scoped command names a slot, and the slot is only knowable
/// from the persisted state file. Once an update can stage into the inactive
/// slot, two distros can exist at once, so an unreadable state file is not a
/// missing value with a safe default -- it is a command with no defensible
/// target. These constructors exist so each surface refuses in its own shape
/// instead of guessing.
pub fn lifecycle_slot_unresolved(
    operation: RuntimeOperation,
    reason: &str,
) -> RuntimeOperationResult {
    RuntimeOperationResult {
        operation,
        state: OperationState::Failed,
        detail: Some(format!(
            "the active runtime slot could not be resolved: {reason}"
        )),
    }
}

/// Verification's shape of the same refusal. `unit` is `Unknown` rather than
/// any concrete state: nothing was probed, so nothing is known.
pub fn verify_slot_unresolved(reason: &str) -> RuntimeVerifyResult {
    RuntimeVerifyResult {
        operation: RuntimeOperation::Verify,
        unit: UnitState::Unknown,
        healthy: false,
        detail: format!("the active runtime slot could not be resolved: {reason}"),
        unprobed: UNPROBED_COMPONENTS.to_vec(),
    }
}

/// Logs' shape of the same refusal: no lines, and `truncated` false because
/// nothing was read to truncate.
pub fn logs_slot_unresolved(reason: &str) -> RuntimeLogsResult {
    RuntimeLogsResult {
        state: OperationState::Failed,
        lines: Vec::new(),
        truncated: false,
        detail: Some(format!(
            "the active runtime slot could not be resolved: {reason}"
        )),
    }
}

pub fn provisioning_blocked() -> RuntimeOperationResult {
    RuntimeOperationResult {
        operation: RuntimeOperation::Provision,
        state: OperationState::Blocked,
        detail: Some(
            "signed runtime manifest verification is required before provisioning".to_string(),
        ),
    }
}

/// Probe used to determine the installed WSL version.
pub const WSL_VERSION_ARGS: [&str; 1] = ["--version"];

/// Tears the distro down so the next start boots it fresh.
///
/// Required after writing `/etc/wsl.conf`: WSL reads that file at boot, so a
/// running distro keeps the configuration it started with and systemd would
/// appear not to have been enabled.
pub const fn wsl_terminate_args(slot: DistroSlot) -> [&'static str; 2] {
    ["--terminate", slot.distro_name()]
}

/// Written to `/etc/wsl.conf` to turn systemd on inside the managed distro.
///
/// A trailing newline because this is a whole file, not a fragment.
pub const WSL_CONF_CONTENTS: &str = "[boot]\nsystemd=true\n";

/// Writes `/etc/wsl.conf` from stdin.
///
/// `tee` rather than a shell. The obvious spelling is
/// `bash -c "echo ... > /etc/wsl.conf"`, and it is wrong here for the same
/// reason every other vector in this module avoids a shell: it hands a string
/// to an interpreter that assigns meaning to characters inside it. `tee`
/// reads the bytes from stdin and writes them, so the content never passes
/// through anything that could interpret it, and the argument vector stays
/// fixed and auditable.
pub const fn wsl_write_conf_args(slot: DistroSlot) -> [&'static str; 6] {
    [
        "--distribution",
        slot.distro_name(),
        "--user",
        "root",
        "--exec",
        "tee",
    ]
}

/// Why an import was refused before `wsl.exe` was ever spawned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImportArgsError {
    /// A path that `wsl.exe` would read as a flag rather than a path.
    PathLooksLikeAFlag,
    /// An empty path.
    PathEmpty,
}

impl ImportArgsError {
    pub fn message(self) -> &'static str {
        match self {
            Self::PathLooksLikeAFlag => {
                "a path beginning with '-' would be parsed as a flag; refusing to provision"
            }
            Self::PathEmpty => "an empty path cannot be imported; refusing to provision",
        }
    }
}

/// Delegates to [`ImportArgsError::message`] so the operator-facing prose has exactly
/// one definition. Writing it twice would let the two drift, and a refusal naming a path is the
/// operator's only signal that provisioning stopped before running wsl.exe.
impl std::fmt::Display for ImportArgsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str((*self).message())
    }
}

impl std::error::Error for ImportArgsError {}

/// Builds the argument vector that imports the runtime rootfs.
///
/// Unlike every other vector here this one cannot be a constant, because two
/// of its arguments are paths chosen at runtime. That is exactly why it
/// validates: a path beginning with `-` is indistinguishable from a flag once
/// it is in the vector, so `--import` would silently take the next real
/// argument as its value. Refused here rather than defended against later.
///
/// `--version 2` is pinned and not a parameter. WSL1 has no systemd and no
/// working Docker, so importing under it produces a distro that fails much
/// later and much less legibly than a refusal here would.
///
/// The slot is a parameter because an update must import into the name that
/// is not currently serving the operator; see `DistroSlot`.
pub fn wsl_import_args(
    slot: DistroSlot,
    install_dir: &str,
    rootfs_tarball: &str,
) -> Result<Vec<String>, ImportArgsError> {
    for path in [install_dir, rootfs_tarball] {
        if path.is_empty() {
            return Err(ImportArgsError::PathEmpty);
        }
        if path.starts_with('-') {
            return Err(ImportArgsError::PathLooksLikeAFlag);
        }
    }

    Ok(vec![
        "--import".to_string(),
        slot.distro_name().to_string(),
        install_dir.to_string(),
        rootfs_tarball.to_string(),
        "--version".to_string(),
        "2".to_string(),
    ])
}

/// Discards the slot that is *not* serving the operator.
///
/// This is the rollback path, and it takes the slot that is live rather than
/// the slot to delete. That inversion is the point: `--unregister` destroys a
/// distro and everything in it, so a rollback that accepted the name to
/// remove would be one argument-passing mistake away from deleting the
/// runtime the operator is relying on. Deriving the target from the live slot
/// makes naming the live slot unrepresentable rather than merely discouraged.
///
/// Used on both edges of an update: to clear a half-prepared staging distro
/// after a failure, and to reclaim the superseded one after a successful
/// swap. In both cases "the slot that is not live" is the correct target, so
/// one vector serves both and neither can be aimed at the live distro.
pub const fn wsl_discard_inactive_args(active: DistroSlot) -> [&'static str; 2] {
    ["--unregister", active.other().distro_name()]
}

/// Interim floor for the WSL2 feature set the broker relies on. The release
/// manifest (task 4.4) is the final authority for the minimum; until it
/// exists this constant is the explicit, reviewable stand-in and is written
/// as an assumption pending Windows certification, not as a certified fact.
pub const MINIMUM_WSL_VERSION: (u32, u32, u32) = (2, 0, 0);

/// Decodes the UTF-16LE text `wsl.exe` emits on stdout. `wsl.exe` output is
/// UTF-16LE on Windows, so decoding it as UTF-8 would produce mojibake; this
/// is the deliberate, documented exception to the broker's exit-code-only
/// classification, and the only content it ever inspects is a version
/// number. Accepts an optional BOM, stops at the first NUL, and fails closed
/// (returns `None`) on anything it cannot decode exactly.
///
/// The captured output in `evidence/windows/` carries NO BOM, so the leading
/// `FF FE` branch is defensive rather than the observed path. It stays because
/// dropping it would make the decoder reject the one shape it cannot rule out,
/// and a mis-decode here fails provisioning closed on a working install.
pub fn decode_utf16le(bytes: &[u8]) -> Option<String> {
    let payload = if bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] == 0xFE {
        &bytes[2..]
    } else {
        bytes
    };
    if payload.len() % 2 != 0 {
        return None;
    }
    let units: Vec<u16> = payload
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .take_while(|unit| *unit != 0)
        .collect();
    let text = String::from_utf16(&units).ok()?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.to_string())
}

fn parse_version_component(bytes: &[u8], start: usize) -> Option<(u32, usize)> {
    let mut end = start;
    while end < bytes.len() && bytes[end].is_ascii_digit() {
        end += 1;
    }
    if end == start || end - start > 9 {
        return None;
    }
    let digits = std::str::from_utf8(&bytes[start..end]).ok()?;
    let value = digits.parse::<u32>().ok()?;
    Some((value, end))
}

fn parse_dotted_component(bytes: &[u8], after: usize) -> Option<(u32, usize)> {
    let (value, end) = parse_version_component(bytes, after)?;
    if bytes.get(end) != Some(&b'.') {
        return None;
    }
    Some((value, end + 1))
}

/// Extracts the first three dotted numeric components from WSL version text,
/// matching `major.minor.patch` anywhere in it.
///
/// Deliberately label-free. Real `wsl.exe --version` output is localized — a
/// Spanish Windows 11 host prints `Versión de WSL: 2.7.10.0` — so matching an
/// English label would turn the display language into a supported-configuration
/// question. The scan is certified against that capture
/// (`evidence/windows/`, replayed by `tests/wsl_evidence_replay.rs`): it reads
/// `2.7.10`, dropping the fourth component, past six further version lines that
/// each carry dotted numbers of their own.
///
/// What remains assumed is that the WSL version is the FIRST such triple in the
/// output. That held on the certified host and is not guaranteed by any
/// documented contract, so the scan stays best-effort: any failure to extract
/// makes the caller fail closed rather than assume the minimum is met.
pub fn extract_wsl_version(text: &str) -> Option<(u32, u32, u32)> {
    let bytes = text.as_bytes();
    let mut start = 0;
    while start < bytes.len() {
        if !bytes[start].is_ascii_digit() {
            start += 1;
            continue;
        }
        if let Some(version) = parse_triple(bytes, start) {
            return Some(version);
        }
        // This run of digits is not the start of a version triple. Skip the
        // whole run rather than retrying at each digit inside it: retrying
        // would let a long number match a suffix of itself, and aborting the
        // scan would make a leading number elsewhere in the line ("WSL 2 —
        // version: 2.0.9.0") hide the real version.
        while start < bytes.len() && bytes[start].is_ascii_digit() {
            start += 1;
        }
    }
    None
}

/// Parses `major.minor.patch` anchored exactly at `start`.
fn parse_triple(bytes: &[u8], start: usize) -> Option<(u32, u32, u32)> {
    let (major, after_major) = parse_dotted_component(bytes, start)?;
    let (minor, after_minor) = parse_dotted_component(bytes, after_major)?;
    let (patch, _) = parse_version_component(bytes, after_minor)?;
    Some((major, minor, patch))
}

/// True when an extracted WSL version meets `MINIMUM_WSL_VERSION`.
pub fn wsl_version_sufficient(version: (u32, u32, u32)) -> bool {
    version >= MINIMUM_WSL_VERSION
}

/// Provisioning preflight over the two fixed probes (`wsl.exe --status` and
/// `wsl.exe --version`). Every route is fail-closed: provisioning performs no
/// mutation today, so the result is always `blocked`, and the detail states
/// what the user can actually do next for each WSL route — absent, present
/// but outdated, present and sufficient, or unprobeable. An undecodable
/// version never passes silently: refusing to provision is preferred to
/// assuming the minimum is met.
pub fn provisioning_preflight(
    status_outcome: CommandOutcome,
    version: Option<(u32, u32, u32)>,
) -> RuntimeOperationResult {
    let detail = match (status_outcome, version) {
        (CommandOutcome::SpawnFailed, _) => Some(
            "WSL is not installed. Install WSL2 (for example with `wsl --install` from an elevated shell) and retry. Provisioning stopped before creating anything.".to_string(),
        ),
        (CommandOutcome::TimedOut, _) => Some(
            "the WSL status probe timed out, so WSL is not reachable yet; provisioning stopped before creating anything".to_string(),
        ),
        (CommandOutcome::Exit(code), _) => Some(format!(
            "the WSL status probe exited with code {code}; refusing to provision"
        )),
        (CommandOutcome::Success, None) => Some(
            "the installed WSL version could not be determined; refusing to provision rather than assume it meets the minimum".to_string(),
        ),
        (CommandOutcome::Success, Some(found)) if !wsl_version_sufficient(found) => Some(format!(
            "WSL version {}.{}.{} is below the minimum {}.{}.{}; update WSL (for example with `wsl --update`) and retry. Provisioning stopped before creating anything.",
            found.0, found.1, found.2, MINIMUM_WSL_VERSION.0, MINIMUM_WSL_VERSION.1, MINIMUM_WSL_VERSION.2
        )),
        // WSL is present and new enough, so the only thing still standing
        // between us and provisioning is the manifest gate. Defer to it rather
        // than restating its message here.
        (CommandOutcome::Success, Some(_)) => return provisioning_blocked(),
    };
    RuntimeOperationResult {
        operation: RuntimeOperation::Provision,
        state: OperationState::Blocked,
        detail,
    }
}

pub fn normalize_log_lines(requested: Option<u16>) -> u16 {
    requested
        .unwrap_or(DEFAULT_LOG_LINES)
        .clamp(1, MAX_LOG_LINES)
}

pub fn runtime_log_args(slot: DistroSlot, requested: Option<u16>) -> Vec<String> {
    let mut args = wsl_log_args_prefix(slot)
        .iter()
        .map(|value| (*value).to_string())
        .collect::<Vec<_>>();
    args.push(normalize_log_lines(requested).to_string());
    args
}

pub fn sanitize_log_text(raw: &str, requested: Option<u16>) -> (Vec<String>, bool) {
    let limit = usize::from(normalize_log_lines(requested));
    let mut lines = Vec::with_capacity(limit.min(raw.lines().count()));
    let mut truncated = false;

    for (index, line) in raw.lines().enumerate() {
        if index >= limit {
            truncated = true;
            break;
        }
        let bounded = line.chars().take(MAX_LOG_LINE_CHARS).collect::<String>();
        if line.chars().count() > MAX_LOG_LINE_CHARS {
            truncated = true;
        }
        let normalized = bounded.to_ascii_lowercase();
        if is_sensitive_log_line(&normalized) {
            lines.push("[REDACTED SENSITIVE LOG LINE]".to_string());
        } else {
            lines.push(bounded);
        }
    }
    (lines, truncated)
}

pub fn unavailable() -> RuntimeComponent {
    RuntimeComponent {
        state: ComponentState::Unavailable,
        detail: None,
    }
}

pub fn runtime_status(platform: &'static str, wsl2: RuntimeComponent) -> RuntimeStatus {
    RuntimeStatus {
        platform,
        wsl2,
        docker: unavailable(),
        core: unavailable(),
        route: unavailable(),
        memory: unavailable(),
        providers: unavailable(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn managed_runtime_commands_are_fixed() {
        assert_eq!(MANAGED_DISTRO_NAME, "RHODIZ-Harness");
        assert_eq!(RUNTIME_BOOTSTRAP_UNIT, "rhodiz-harness-bootstrap.service");
        assert_eq!(WSL_EXE, "wsl.exe");
        assert_eq!(WSL_STATUS_ARGS, ["--status"]);
        assert_eq!(
            wsl_start_args(DistroSlot::INITIAL),
            [
                "--distribution",
                "RHODIZ-Harness",
                "--user",
                "root",
                "--exec",
                "systemctl",
                "start",
                "rhodiz-harness-bootstrap.service"
            ]
        );
        assert_eq!(
            wsl_stop_args(DistroSlot::INITIAL),
            [
                "--distribution",
                "RHODIZ-Harness",
                "--user",
                "root",
                "--exec",
                "systemctl",
                "stop",
                "rhodiz-harness-bootstrap.service"
            ]
        );
    }

    #[test]
    fn log_line_count_is_bounded_and_only_numeric() {
        assert_eq!(normalize_log_lines(None), DEFAULT_LOG_LINES);
        assert_eq!(normalize_log_lines(Some(0)), 1);
        assert_eq!(normalize_log_lines(Some(MAX_LOG_LINES + 1)), MAX_LOG_LINES);

        let args = runtime_log_args(DistroSlot::INITIAL, Some(9));
        assert_eq!(args.last().map(String::as_str), Some("9"));
        assert!(args.contains(&"journalctl".to_string()));
        assert!(args.contains(&"rhodiz-harness-bootstrap.service".to_string()));
        assert!(!args.iter().any(|arg| arg.contains("powershell")));
        assert!(!args.iter().any(|arg| arg == "sh" || arg == "bash"));
    }

    #[test]
    fn logs_are_redacted_and_payload_bounded() {
        let oversized = "x".repeat(MAX_LOG_LINE_CHARS + 5);
        let raw = format!("healthy\nauthorization: Bearer top-secret\n{oversized}\nextra");
        let (lines, truncated) = sanitize_log_text(&raw, Some(3));
        assert_eq!(lines[0], "healthy");
        assert_eq!(lines[1], "[REDACTED SENSITIVE LOG LINE]");
        assert_eq!(lines[2].chars().count(), MAX_LOG_LINE_CHARS);
        assert!(truncated);
    }

    #[test]
    fn credential_prefixes_are_redacted_at_token_boundaries() {
        let raw = [
            "issued sk-proj-abcdef0123456789",
            "rotated key=\"ghp_ABCDEF0123456789\"",
            "aws id AKIAIOSFODNN7EXAMPLE rotated",
            "slack hook xoxb-1111-2222",
            "pat github_pat_11ABCDEFG0123456789",
        ]
        .join("\n");
        let (lines, _) = sanitize_log_text(&raw, Some(5));
        assert!(
            lines
                .iter()
                .all(|line| line == "[REDACTED SENSITIVE LOG LINE]"),
            "every credential-shaped token must be redacted: {lines:?}"
        );
    }

    #[test]
    fn ordinary_operational_lines_survive_prefix_matching() {
        let raw = [
            "disk-usage at 91 percent",
            "task-runner started in 12ms",
            "risk-report generated",
            "desk-service reachable",
            "user Nakia connected",
        ]
        .join("\n");
        let (lines, truncated) = sanitize_log_text(&raw, Some(5));
        assert!(
            !lines
                .iter()
                .any(|line| line == "[REDACTED SENSITIVE LOG LINE]"),
            "ordinary lines must not be redacted: {lines:?}"
        );
        assert!(!truncated);
        assert_eq!(lines[0], "disk-usage at 91 percent");
    }

    #[test]
    fn jwt_shaped_tokens_are_redacted() {
        let raw = "auth header eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.payload.sig accepted";
        let (lines, _) = sanitize_log_text(raw, Some(1));
        assert_eq!(lines[0], "[REDACTED SENSITIVE LOG LINE]");
    }

    #[test]
    fn lifecycle_contention_is_blocked_not_failed_and_never_succeeds() {
        for build in [lifecycle_busy, lifecycle_cross_process_busy] {
            let result = build(RuntimeOperation::Start);
            assert_eq!(result.state, OperationState::Blocked);
            assert_ne!(result.state, OperationState::Succeeded);
            assert!(result.detail.is_some());
        }
        let poisoned = lifecycle_lock_unavailable(RuntimeOperation::Stop);
        assert_eq!(poisoned.state, OperationState::Failed);
    }

    #[test]
    fn an_unresolved_slot_never_reads_as_healthy_or_succeeded() {
        let reason = "installed state does not parse";

        let lifecycle = lifecycle_slot_unresolved(RuntimeOperation::Start, reason);
        assert_eq!(lifecycle.state, OperationState::Failed);
        assert_eq!(lifecycle.operation, RuntimeOperation::Start);

        let verify = verify_slot_unresolved(reason);
        assert!(!verify.healthy);
        assert_eq!(verify.unit, UnitState::Unknown);
        // Nothing was probed, so nothing may be reported as probed.
        assert_eq!(verify.unprobed.len(), UNPROBED_COMPONENTS.len());

        let logs = logs_slot_unresolved(reason);
        assert_eq!(logs.state, OperationState::Failed);
        assert!(logs.lines.is_empty());
        // No read happened, so there is nothing that could have been truncated.
        assert!(!logs.truncated);

        // The cause travels to the renderer in all three shapes. A refusal
        // that does not say why is one an operator cannot act on.
        assert!(lifecycle.detail.unwrap().contains(reason));
        assert!(verify.detail.contains(reason));
        assert!(logs.detail.unwrap().contains(reason));
    }

    #[test]
    fn discarding_derives_the_dead_slot_from_the_live_one() {
        for live in [DistroSlot::INITIAL, DistroSlot::INITIAL.other()] {
            let args = wsl_discard_inactive_args(live);
            assert_eq!(args[0], "--unregister");
            // The live distro can never be the one named for destruction.
            assert_ne!(args[1], live.distro_name());
            assert_eq!(args[1], live.other().distro_name());
        }
    }

    #[test]
    fn lifecycle_lock_path_components_are_fixed() {
        assert_eq!(LIFECYCLE_LOCK_DIRECTORY, "com.rhodiz.harness.desktop");
        assert_eq!(LIFECYCLE_LOCK_FILE, "runtime-lifecycle.lock");
        assert!(!LIFECYCLE_LOCK_FILE.contains('/'));
        assert!(!LIFECYCLE_LOCK_FILE.contains('\\'));
    }

    #[test]
    fn provisioning_fails_closed_until_signature_verification_exists() {
        let result = provisioning_blocked();
        assert_eq!(result.operation, RuntimeOperation::Provision);
        assert_eq!(result.state, OperationState::Blocked);
        assert!(result.detail.unwrap().contains("signed runtime manifest"));
    }

    #[test]
    fn verify_and_repair_arg_vectors_are_fixed_and_shell_free() {
        assert_eq!(
            wsl_verify_args(DistroSlot::INITIAL),
            [
                "--distribution",
                "RHODIZ-Harness",
                "--user",
                "root",
                "--exec",
                "systemctl",
                "is-active",
                "rhodiz-harness-bootstrap.service"
            ]
        );
        assert_eq!(
            wsl_repair_reset_args(DistroSlot::INITIAL)[6],
            "reset-failed"
        );
        assert_eq!(wsl_repair_restart_args(DistroSlot::INITIAL)[6], "restart");

        // Every slot-addressed vector must name the slot it was asked for and
        // nothing else: a lifecycle command that silently addressed the other
        // slot would operate on a distro the operator is not running.
        for slot in [DistroSlot::Primary, DistroSlot::Secondary] {
            for args in [
                wsl_verify_args(slot),
                wsl_repair_reset_args(slot),
                wsl_repair_restart_args(slot),
                wsl_start_args(slot),
                wsl_stop_args(slot),
            ] {
                assert_eq!(args[1], slot.distro_name());
                assert_eq!(args[7], RUNTIME_BOOTSTRAP_UNIT);
                assert!(!args.iter().any(|arg| *arg == "sh" || *arg == "bash"));
                assert!(!args.iter().any(|arg| arg.contains("powershell")));
                assert!(!args.iter().any(|arg| arg.contains("cmd.exe")));
            }
            assert_eq!(wsl_log_args_prefix(slot)[1], slot.distro_name());
        }
    }

    #[test]
    fn repair_never_provisions_installs_or_deletes() {
        // The bounded remedy is restarting a unit. Anything that could create or
        // destroy a distribution must stay out of these vectors.
        for args in [
            wsl_repair_reset_args(DistroSlot::Primary),
            wsl_repair_restart_args(DistroSlot::Primary),
            wsl_repair_reset_args(DistroSlot::Secondary),
            wsl_repair_restart_args(DistroSlot::Secondary),
        ] {
            for forbidden in [
                "--install",
                "--import",
                "--unregister",
                "--terminate",
                "--shutdown",
                "--set-default",
            ] {
                assert!(
                    !args.contains(&forbidden),
                    "repair must not carry {forbidden}: {args:?}"
                );
            }
        }
    }

    #[test]
    fn unit_probe_separates_an_inactive_unit_from_an_unreachable_distro() {
        // The two faults have different remedies, so they must not collapse into
        // one state.
        let (inactive, inactive_detail) =
            classify_unit_probe(CommandOutcome::Exit(SYSTEMCTL_INACTIVE_EXIT_CODE));
        assert_eq!(inactive, UnitState::Inactive);
        assert!(inactive_detail.contains("not running"));

        let (unreachable, unreachable_detail) = classify_unit_probe(CommandOutcome::Exit(1));
        assert_eq!(unreachable, UnitState::DistroUnreachable);
        assert!(unreachable_detail.contains("provision"));

        assert_ne!(inactive, unreachable);
    }

    #[test]
    fn verification_is_healthy_only_when_the_unit_is_active() {
        assert!(verify_result(CommandOutcome::Success).healthy);

        for outcome in [
            CommandOutcome::Exit(SYSTEMCTL_INACTIVE_EXIT_CODE),
            CommandOutcome::Exit(1),
            CommandOutcome::SpawnFailed,
            CommandOutcome::TimedOut,
        ] {
            let result = verify_result(outcome);
            assert!(
                !result.healthy,
                "an undetermined or failing probe must never read healthy: {result:?}"
            );
            assert!(!result.detail.is_empty());
        }

        // A timeout is unknown state, never a pass.
        assert_eq!(
            verify_result(CommandOutcome::TimedOut).unit,
            UnitState::Unknown
        );
        assert!(!unsupported_verify().healthy);
    }

    #[test]
    fn verification_declares_what_it_did_not_probe() {
        // Silence about Docker/Core/Route/Memory/Providers would read as a pass.
        for result in [verify_result(CommandOutcome::Success), unsupported_verify()] {
            assert_eq!(
                result.unprobed,
                vec!["docker", "core", "route", "memory", "providers"]
            );
        }
    }

    #[test]
    fn wsl_outcomes_are_fail_closed() {
        assert_eq!(
            classify_wsl_status(CommandOutcome::Success).state,
            ComponentState::Ready
        );
        assert_eq!(
            classify_wsl_status(CommandOutcome::Exit(5)).state,
            ComponentState::Stopped
        );
        assert_eq!(
            classify_wsl_status(CommandOutcome::SpawnFailed).state,
            ComponentState::Missing
        );
        assert_eq!(
            classify_wsl_status(CommandOutcome::TimedOut).state,
            ComponentState::Unavailable
        );
    }

    #[test]
    fn lifecycle_timeout_is_not_reported_as_success() {
        let result = operation_result(RuntimeOperation::Start, CommandOutcome::TimedOut);
        assert_eq!(result.state, OperationState::TimedOut);
    }

    #[test]
    fn non_wsl_components_start_unavailable() {
        let status = runtime_status("unsupported", unavailable());
        assert_eq!(status.platform, "unsupported");
        assert_eq!(status.docker.state, ComponentState::Unavailable);
        assert_eq!(status.core.state, ComponentState::Unavailable);
        assert_eq!(status.route.state, ComponentState::Unavailable);
        assert_eq!(status.memory.state, ComponentState::Unavailable);
        assert_eq!(status.providers.state, ComponentState::Unavailable);
    }

    /// Encodes ASCII as the UTF-16LE bytes `wsl.exe` actually emits.
    fn utf16le(text: &str) -> Vec<u8> {
        text.encode_utf16()
            .flat_map(|unit| unit.to_le_bytes())
            .collect()
    }

    #[test]
    fn utf16le_decode_handles_bom_nul_and_trim() {
        let mut with_bom = vec![0xFF, 0xFE];
        with_bom.extend(utf16le("  2.1.3 \r\n"));
        assert_eq!(decode_utf16le(&with_bom), Some("2.1.3".to_string()));

        let mut nul_terminated = utf16le("2.1.3");
        nul_terminated.extend(utf16le("\u{0}trailing ignored"));
        assert_eq!(decode_utf16le(&nul_terminated), Some("2.1.3".to_string()));

        assert_eq!(
            decode_utf16le(&utf16le("WSL version: 2.1.3")),
            Some("WSL version: 2.1.3".to_string())
        );
    }

    #[test]
    fn utf16le_decode_fails_closed_on_malformed_input() {
        // Odd byte count cannot be a UTF-16 stream.
        assert_eq!(decode_utf16le(&[0x41, 0x00, 0x42]), None);
        // Unpaired surrogate is not valid UTF-16.
        assert_eq!(decode_utf16le(&[0x00, 0xD8]), None);
        // BOM-only and whitespace-only output are not a version source.
        assert_eq!(decode_utf16le(&[0xFF, 0xFE]), None);
        assert_eq!(decode_utf16le(&utf16le("   ")), None);
    }

    #[test]
    fn version_extraction_finds_the_first_dotted_triple() {
        assert_eq!(extract_wsl_version("WSL version: 2.1.3.0"), Some((2, 1, 3)));
        assert_eq!(extract_wsl_version("12.34.56 extra"), Some((12, 34, 56)));
        assert_eq!(extract_wsl_version("no version here"), None);
        // A component wider than nine digits cannot be a version, so the
        // extraction fails closed instead of overflowing or truncating.
        assert_eq!(extract_wsl_version("123456789012.1.2"), None);
        // A partial triple is not a version either.
        assert_eq!(extract_wsl_version("2.1"), None);
        // A bare number ahead of the triple must not hide it, and must not be
        // rescanned digit by digit into a bogus match.
        assert_eq!(
            extract_wsl_version("WSL 2 - version: 2.0.9.0"),
            Some((2, 0, 9))
        );
    }

    #[test]
    fn version_extraction_survives_a_localized_windows() {
        // `wsl.exe --version` translates its labels. A Spanish Windows 11 host
        // prints "Versión de WSL:", so a parser keyed to the English label
        // would read nothing and the caller would fail closed on a perfectly
        // good install. The scan is label-free precisely so that a locale is
        // not a supported-configuration question; this test is what keeps it
        // that way. The authoritative bytes live in evidence/windows/ and are
        // replayed by tests/wsl_evidence_replay.rs — this fixture is the same
        // shape, transcribed from the first three lines of that capture.
        let localized = "Versión de WSL: 2.7.10.0\r\n\
                         Versión de kernel: 6.18.33.2-2\r\n\
                         Versión de WSLg: 1.0.73.2\r\n";
        assert_eq!(extract_wsl_version(localized), Some((2, 7, 10)));
    }

    #[test]
    fn wsl_version_sufficient_compares_component_wise() {
        assert!(wsl_version_sufficient(MINIMUM_WSL_VERSION));
        assert!(wsl_version_sufficient((99, 0, 0)));
        assert!(!wsl_version_sufficient((1, 9, 9)));
    }

    #[test]
    fn preflight_reports_wsl_absent_with_actionable_detail() {
        let result = provisioning_preflight(CommandOutcome::SpawnFailed, None);
        assert_eq!(result.operation, RuntimeOperation::Provision);
        assert_eq!(result.state, OperationState::Blocked);
        assert!(result.detail.unwrap().contains("wsl --install"));
    }

    #[test]
    fn preflight_reports_outdated_wsl_and_stops_before_creating_anything() {
        let result = provisioning_preflight(CommandOutcome::Success, Some((1, 9, 9)));
        assert_eq!(result.state, OperationState::Blocked);
        let detail = result.detail.unwrap();
        assert!(detail.contains("1.9.9"));
        assert!(detail.contains("wsl --update"));
        assert!(detail.contains("before creating anything"));
    }

    #[test]
    fn preflight_refuses_to_provision_when_the_version_cannot_be_determined() {
        let result = provisioning_preflight(CommandOutcome::Success, None);
        assert_eq!(result.state, OperationState::Blocked);
        assert!(result.detail.unwrap().contains("could not be determined"));
    }

    #[test]
    fn preflight_keeps_the_signed_manifest_gate_for_sufficient_wsl() {
        let result = provisioning_preflight(CommandOutcome::Success, Some((9, 9, 9)));
        assert_eq!(result.state, OperationState::Blocked);
        assert!(result
            .detail
            .unwrap()
            .contains("signed runtime manifest verification"));
    }

    #[test]
    fn preflight_treats_timeout_and_nonzero_status_as_not_probeable() {
        let timed_out = provisioning_preflight(CommandOutcome::TimedOut, None);
        assert_eq!(timed_out.state, OperationState::Blocked);
        let exited = provisioning_preflight(CommandOutcome::Exit(7), None);
        assert_eq!(exited.state, OperationState::Blocked);
        assert!(exited.detail.unwrap().contains("7"));
    }

    #[test]
    fn provisioning_arg_vectors_are_fixed_shell_free_and_pin_the_distro() {
        for slot in [DistroSlot::Primary, DistroSlot::Secondary] {
            let terminate = wsl_terminate_args(slot);
            let write_conf = wsl_write_conf_args(slot);
            let discard = wsl_discard_inactive_args(slot);

            assert_eq!(terminate, ["--terminate", slot.distro_name()]);
            assert_eq!(write_conf[1], slot.distro_name());
            assert_eq!(write_conf[5], "tee");

            // The same shell-freedom the lifecycle vectors hold. Writing a
            // config file is where a shell is most tempting and least
            // defensible.
            for arg in write_conf.iter().chain(&terminate).chain(&discard) {
                assert!(*arg != "sh" && *arg != "bash", "{arg} is a shell");
                assert!(!arg.contains("powershell"));
                assert!(!arg.contains("cmd.exe"));
                // No redirection, no separators: nothing that only means
                // anything to an interpreter.
                assert!(!arg.contains('>') && !arg.contains('|') && !arg.contains(';'));
            }
        }
    }

    #[test]
    fn the_two_slots_are_distinct_names_and_other_is_an_involution() {
        assert_ne!(
            DistroSlot::Primary.distro_name(),
            DistroSlot::Secondary.distro_name(),
            "a single name cannot hold a live distro and a staged one at once"
        );
        assert_eq!(DistroSlot::Primary.distro_name(), MANAGED_DISTRO_NAME);
        assert_eq!(DistroSlot::Secondary.distro_name(), STAGING_DISTRO_NAME);
        assert_eq!(DistroSlot::INITIAL, DistroSlot::Primary);

        for slot in [DistroSlot::Primary, DistroSlot::Secondary] {
            assert_ne!(slot.other(), slot, "staging must never be the live slot");
            assert_eq!(
                slot.other().other(),
                slot,
                "two updates return to the start"
            );
        }
    }

    #[test]
    fn a_rollback_can_never_be_aimed_at_the_live_distro() {
        // The invariant the whole staging design rests on. `--unregister`
        // destroys a distro and everything in it, so the one thing cleanup
        // must be incapable of is naming the distro the operator is using.
        // `wsl_discard_inactive_args` takes the live slot, not the target, so
        // there is no argument a caller could pass to delete the live one.
        for live in [DistroSlot::Primary, DistroSlot::Secondary] {
            let discard = wsl_discard_inactive_args(live);
            assert_eq!(discard[0], "--unregister");
            assert_ne!(
                discard[1],
                live.distro_name(),
                "cleanup named the distro that is serving the operator"
            );
            assert_eq!(discard[1], live.other().distro_name());
        }
    }

    #[test]
    fn an_update_stages_into_the_name_that_is_not_live() {
        // A first install lands in Primary, so its update must be prepared in
        // Secondary and leave the live rootfs in place; the update after that
        // alternates back. At no point does an import name the live slot.
        let mut live = DistroSlot::INITIAL;
        for _ in 0..4 {
            let staging = live.other();
            let import = wsl_import_args(staging, r"C:\RHODIZ\next", r"C:\RHODIZ\next.tar")
                .expect("ordinary Windows paths must be accepted");
            assert_eq!(import[1], staging.distro_name());
            assert_ne!(
                import[1],
                live.distro_name(),
                "an import overwrote the live distro"
            );
            // The swap is the active slot moving; nothing renames a distro.
            live = staging;
        }
    }

    #[test]
    fn the_wsl_conf_we_write_enables_systemd_and_is_a_whole_file() {
        assert_eq!(WSL_CONF_CONTENTS, "[boot]\nsystemd=true\n");
        assert!(
            WSL_CONF_CONTENTS.ends_with('\n'),
            "a config file without a final newline is a fragment"
        );
    }

    #[test]
    fn an_import_pins_the_distro_and_wsl2() {
        let args = wsl_import_args(
            DistroSlot::INITIAL,
            r"C:\ProgramData\RHODIZ\distro",
            r"C:\ProgramData\RHODIZ\rootfs.tar",
        )
        .expect("ordinary Windows paths must be accepted");
        assert_eq!(args[0], "--import");
        assert_eq!(args[1], MANAGED_DISTRO_NAME);
        assert_eq!(
            &args[4..],
            ["--version", "2"],
            "WSL1 has no systemd and no working Docker; the version is not a parameter"
        );
    }

    #[test]
    fn a_path_that_would_be_read_as_a_flag_is_refused_before_spawning() {
        // Argument injection, not shell injection: there is no shell here, but
        // `--import` takes positional values, so a path beginning with '-'
        // would be consumed as a flag and the next real argument taken as the
        // value. Caught before wsl.exe ever runs.
        assert_eq!(
            wsl_import_args(DistroSlot::INITIAL, "--version", "/tmp/rootfs.tar"),
            Err(ImportArgsError::PathLooksLikeAFlag)
        );
        assert_eq!(
            wsl_import_args(DistroSlot::INITIAL, "/tmp/dir", "--version"),
            Err(ImportArgsError::PathLooksLikeAFlag)
        );
        assert_eq!(
            wsl_import_args(DistroSlot::INITIAL, "", "/tmp/rootfs.tar"),
            Err(ImportArgsError::PathEmpty)
        );
        for error in [
            ImportArgsError::PathLooksLikeAFlag,
            ImportArgsError::PathEmpty,
        ] {
            assert!(error.message().ends_with("; refusing to provision"));
        }
    }

    #[test]
    fn a_path_containing_shell_metacharacters_is_passed_through_untouched() {
        // Deliberately NOT rejected. There is no shell in the vector, so a
        // space or an ampersand in a Windows path is just a character. A
        // filter here would reject legitimate paths while protecting against
        // an interpreter that is not being invoked, and would suggest the
        // vector is unsafe without one.
        let args = wsl_import_args(
            DistroSlot::INITIAL,
            r"C:\Users\Ada & Co\distro",
            r"C:\tmp\root fs.tar",
        )
        .expect("paths with spaces and ampersands are ordinary on Windows");
        assert_eq!(args[2], r"C:\Users\Ada & Co\distro");
        assert_eq!(args[3], r"C:\tmp\root fs.tar");
    }

    #[test]
    fn import_args_error_display_renders_exactly_the_message_text() {
        for e in [
            ImportArgsError::PathLooksLikeAFlag,
            ImportArgsError::PathEmpty,
        ] {
            assert_eq!(e.to_string(), e.message());
        }
    }

    /// The broker records the active slot on disk. Pinning the spelling here
    /// means a variant rename shows up as a failing test rather than as a
    /// state file that silently reads back as the other slot.
    #[test]
    fn distro_slot_serializes_under_its_variant_name() {
        assert_eq!(
            serde_json::to_string(&DistroSlot::Primary).expect("slot serializes"),
            "\"primary\"",
        );
        assert_eq!(
            serde_json::to_string(&DistroSlot::Secondary).expect("slot serializes"),
            "\"secondary\"",
        );
    }

    #[test]
    fn distro_slot_round_trips_through_json() {
        for slot in [DistroSlot::Primary, DistroSlot::Secondary] {
            let encoded = serde_json::to_string(&slot).expect("slot serializes");
            let decoded: DistroSlot = serde_json::from_str(&encoded).expect("slot deserializes");
            assert_eq!(decoded, slot);
            assert_eq!(decoded.distro_name(), slot.distro_name());
        }
    }
}
