use rhodiz_harness_broker_core::{
    classify_wsl_status, runtime_status as build_status, CommandOutcome, RuntimeStatus, WSL_EXE,
    WSL_STATUS_ARGS,
};

#[cfg(target_os = "windows")]
fn platform_status() -> RuntimeStatus {
    use std::process::Command;

    let outcome = match Command::new(WSL_EXE).args(WSL_STATUS_ARGS).status() {
        Ok(status) if status.success() => CommandOutcome::Success,
        Ok(status) => CommandOutcome::Exit(status.code().unwrap_or(-1)),
        Err(_) => CommandOutcome::SpawnFailed,
    };
    build_status("windows", classify_wsl_status(outcome))
}

#[cfg(not(target_os = "windows"))]
fn platform_status() -> RuntimeStatus {
    build_status("unsupported", rhodiz_harness_broker_core::unavailable())
}

#[tauri::command]
pub fn runtime_status() -> RuntimeStatus {
    platform_status()
}
