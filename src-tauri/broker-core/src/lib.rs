use serde::Serialize;

/// Detached-signature verification for the release manifest. It lives in its
/// own module because the ordering it enforces — verify the exact bytes
/// before anything parses them — is a property of that module's types, and
/// keeping both the handle's constructor and the key-taking verifier private
/// to it is what stops a caller here from choosing its own trust anchor.
mod manifest;

/// Re-exported deliberately narrowly. `verify_release_manifest` is the only
/// entry point: it consults the anchor pinned at build time, and there is no
/// way from outside the module to verify against any other key. The two
/// length constants are public because the error documentation refers to
/// them, not because a caller needs to build a key or a signature by hand.
pub use manifest::{
    verify_release_manifest, ManifestVerifyError, VerifiedManifestBytes, MANIFEST_PUBLIC_KEY_LEN,
    MANIFEST_SIGNATURE_LEN, MAX_MANIFEST_BYTES,
};

pub const MANAGED_DISTRO_NAME: &str = "RHODIZ-Harness";
pub const RUNTIME_BOOTSTRAP_UNIT: &str = "rhodiz-harness-bootstrap.service";
pub const WSL_EXE: &str = "wsl.exe";
pub const WSL_STATUS_ARGS: [&str; 1] = ["--status"];
pub const WSL_START_ARGS: [&str; 8] = [
    "--distribution",
    MANAGED_DISTRO_NAME,
    "--user",
    "root",
    "--exec",
    "systemctl",
    "start",
    RUNTIME_BOOTSTRAP_UNIT,
];
pub const WSL_STOP_ARGS: [&str; 8] = [
    "--distribution",
    MANAGED_DISTRO_NAME,
    "--user",
    "root",
    "--exec",
    "systemctl",
    "stop",
    RUNTIME_BOOTSTRAP_UNIT,
];
/// Read-only probe. `systemctl is-active` mutates nothing, so verification can
/// run while a lifecycle mutation holds the lock.
pub const WSL_VERIFY_ARGS: [&str; 8] = [
    "--distribution",
    MANAGED_DISTRO_NAME,
    "--user",
    "root",
    "--exec",
    "systemctl",
    "is-active",
    RUNTIME_BOOTSTRAP_UNIT,
];
/// Clears a latched failure so the restart below is not refused by a start-limit
/// counter. Best-effort: see `WSL_REPAIR_RESTART_ARGS`.
pub const WSL_REPAIR_RESET_ARGS: [&str; 8] = [
    "--distribution",
    MANAGED_DISTRO_NAME,
    "--user",
    "root",
    "--exec",
    "systemctl",
    "reset-failed",
    RUNTIME_BOOTSTRAP_UNIT,
];
/// The step whose outcome decides the repair result.
pub const WSL_REPAIR_RESTART_ARGS: [&str; 8] = [
    "--distribution",
    MANAGED_DISTRO_NAME,
    "--user",
    "root",
    "--exec",
    "systemctl",
    "restart",
    RUNTIME_BOOTSTRAP_UNIT,
];
pub const WSL_LOG_ARGS_PREFIX: [&str; 11] = [
    "--distribution",
    MANAGED_DISTRO_NAME,
    "--user",
    "root",
    "--exec",
    "journalctl",
    "--unit",
    RUNTIME_BOOTSTRAP_UNIT,
    "--no-pager",
    "--output=short-iso",
    "--lines",
];

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
/// matching `major.minor.patch` anywhere in it. The exact output format of
/// `wsl.exe --version` is a documented assumption pending Windows
/// certification: the probe is deliberately best-effort, and any failure to
/// extract makes the caller fail closed rather than assume the minimum is
/// met.
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

pub fn runtime_log_args(requested: Option<u16>) -> Vec<String> {
    let mut args = WSL_LOG_ARGS_PREFIX
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
            WSL_START_ARGS,
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
            WSL_STOP_ARGS,
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

        let args = runtime_log_args(Some(9));
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
            WSL_VERIFY_ARGS,
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
        assert_eq!(WSL_REPAIR_RESET_ARGS[6], "reset-failed");
        assert_eq!(WSL_REPAIR_RESTART_ARGS[6], "restart");

        for args in [
            WSL_VERIFY_ARGS,
            WSL_REPAIR_RESET_ARGS,
            WSL_REPAIR_RESTART_ARGS,
        ] {
            assert_eq!(args[1], MANAGED_DISTRO_NAME);
            assert_eq!(args[7], RUNTIME_BOOTSTRAP_UNIT);
            assert!(!args.iter().any(|arg| *arg == "sh" || *arg == "bash"));
            assert!(!args.iter().any(|arg| arg.contains("powershell")));
            assert!(!args.iter().any(|arg| arg.contains("cmd.exe")));
        }
    }

    #[test]
    fn repair_never_provisions_installs_or_deletes() {
        // The bounded remedy is restarting a unit. Anything that could create or
        // destroy a distribution must stay out of these vectors.
        for args in [WSL_REPAIR_RESET_ARGS, WSL_REPAIR_RESTART_ARGS] {
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
}
