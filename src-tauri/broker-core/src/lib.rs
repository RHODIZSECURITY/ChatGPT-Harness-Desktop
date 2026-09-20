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
const SENSITIVE_TOKEN_PREFIXES: [&str; 5] = ["ghp_", "github_pat_", "sk-", "akia", "xoxb-"];

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
    fn provisioning_fails_closed_until_signature_verification_exists() {
        let result = provisioning_blocked();
        assert_eq!(result.operation, RuntimeOperation::Provision);
        assert_eq!(result.state, OperationState::Blocked);
        assert!(result.detail.unwrap().contains("signed runtime manifest"));
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
