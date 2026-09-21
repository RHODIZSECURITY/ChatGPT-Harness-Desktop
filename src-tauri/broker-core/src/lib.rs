use serde::Serialize;

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
}
