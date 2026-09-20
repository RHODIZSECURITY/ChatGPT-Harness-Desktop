use serde::Serialize;

pub const MANAGED_DISTRO_NAME: &str = "RHODIZ-Harness";
pub const WSL_EXE: &str = "wsl.exe";
pub const WSL_STATUS_ARGS: [&str; 1] = ["--status"];

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
    }
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
    fn managed_distro_and_wsl_status_command_are_fixed() {
        assert_eq!(MANAGED_DISTRO_NAME, "RHODIZ-Harness");
        assert_eq!(WSL_EXE, "wsl.exe");
        assert_eq!(WSL_STATUS_ARGS, ["--status"]);
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
