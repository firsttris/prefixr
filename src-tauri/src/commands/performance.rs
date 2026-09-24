use crate::error::AppError;
use std::fs;

use serde::Serialize;
use tauri::{AppHandle, State};

use crate::commands::games::command_on_path;
use crate::config::{save_config, ConfigState};
use crate::models::PerformanceConfig;

#[tauri::command]
pub fn get_performance_config(state: State<ConfigState>) -> Result<PerformanceConfig, AppError> {
    let config = state
        .lock()
        .map_err(|_| "Configuration is locked".to_string())?;
    Ok(config.performance.clone())
}

#[tauri::command]
pub fn save_performance_config(
    app: AppHandle,
    state: State<ConfigState>,
    config: PerformanceConfig,
) -> Result<PerformanceConfig, AppError> {
    let mut app_config = state
        .lock()
        .map_err(|_| "Configuration is locked".to_string())?;
    app_config.performance = config;
    save_config(&app, &app_config)?;
    Ok(app_config.performance.clone())
}

/// The value Steam's own client applies system-wide since ~2023. Several
/// modern titles and anti-cheat runtimes (EAC-backed games, Baldur's Gate 3,
/// Elden Ring, Diablo IV, ...) fail to start or crash on launch below this,
/// and most distros still ship a default several orders of magnitude lower.
/// Games launched outside Steam — this app's whole point — never get Steam's
/// own bump, so nothing raises this unless the user (or we) do.
const RECOMMENDED_MAX_MAP_COUNT: u64 = 2_147_483_642;

const MAX_MAP_COUNT_PATH: &str = "/proc/sys/vm/max_map_count";

/// Drop-in rather than editing `/etc/sysctl.conf` directly, so the fix stays
/// isolated and removable without touching whatever the user (or distro)
/// already has in there. `sysctl --system` (see `fix_max_map_count`) reads
/// every `/etc/sysctl.d/*.conf` file, this one included, so writing it here
/// is enough for it to take effect on the next `sysctl --system`/reboot too.
const SYSCTL_DROPIN_PATH: &str = "/etc/sysctl.d/99-prefixr-max-map-count.conf";

fn parse_max_map_count(raw: Option<&str>) -> u64 {
    raw.and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(0)
}

fn manual_fix_command() -> String {
    format!(
        "sudo sh -c 'echo \"vm.max_map_count = {RECOMMENDED_MAX_MAP_COUNT}\" > {SYSCTL_DROPIN_PATH} && sysctl --system'"
    )
}

fn sysctl_fix_script() -> String {
    format!(
        "echo 'vm.max_map_count = {RECOMMENDED_MAX_MAP_COUNT}' > {SYSCTL_DROPIN_PATH} && sysctl --system"
    )
}

fn build_max_map_count_status(current: u64, can_fix: bool) -> MaxMapCountStatus {
    MaxMapCountStatus {
        current,
        recommended: RECOMMENDED_MAX_MAP_COUNT,
        sufficient: current >= RECOMMENDED_MAX_MAP_COUNT,
        can_fix,
    }
}

fn finalize_max_map_count_fix(
    pkexec_available: bool,
    run_result: Result<(bool, String), String>,
) -> Result<(), AppError> {
    if !pkexec_available {
        return Err(AppError::PkexecNotInstalled {
            command: manual_fix_command(),
        });
    }

    let (success, status) = run_result.map_err(AppError::from)?;
    if !success {
        return Err(AppError::SysctlChangeFailed { status });
    }
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub struct MaxMapCountStatus {
    pub current: u64,
    pub recommended: u64,
    pub sufficient: bool,
    /// Whether `fix_max_map_count` can actually be offered — it shells out to
    /// `pkexec` for the privilege escalation, which isn't guaranteed to be
    /// installed (e.g. minimal window-manager-only setups without a
    /// PolicyKit agent). The frontend falls back to showing the equivalent
    /// shell command for the user to run manually when this is `false`.
    pub can_fix: bool,
}

#[tauri::command]
pub fn check_max_map_count() -> MaxMapCountStatus {
    let current = parse_max_map_count(fs::read_to_string(MAX_MAP_COUNT_PATH).ok().as_deref());
    build_max_map_count_status(current, command_on_path("pkexec"))
}

/// Writes the sysctl drop-in and applies it immediately via `pkexec` (a
/// PolicyKit prompt), rather than just `sysctl -w` alone — that would raise
/// the limit for this boot only and silently regress after the next reboot,
/// which would be more confusing than not offering a fix at all.
#[tauri::command]
pub async fn fix_max_map_count() -> Result<(), AppError> {
    let pkexec_available = command_on_path("pkexec");
    let script = sysctl_fix_script();
    let run_result = if pkexec_available {
        tokio::process::Command::new("pkexec")
            .args(["sh", "-c", &script])
            .status()
            .await
            .map(|status| (status.success(), status.to_string()))
            .map_err(|e| format!("Could not run pkexec: {e}"))
    } else {
        Ok((false, String::new()))
    };
    finalize_max_map_count_fix(pkexec_available, run_result)
}

#[cfg(test)]
mod tests {
    use super::{
        build_max_map_count_status, finalize_max_map_count_fix, manual_fix_command,
        parse_max_map_count, sysctl_fix_script, MaxMapCountStatus, RECOMMENDED_MAX_MAP_COUNT,
        SYSCTL_DROPIN_PATH,
    };
    use crate::error::AppError;

    fn assert_status(status: MaxMapCountStatus, current: u64, sufficient: bool, can_fix: bool) {
        assert_eq!(status.current, current);
        assert_eq!(status.recommended, RECOMMENDED_MAX_MAP_COUNT);
        assert_eq!(status.sufficient, sufficient);
        assert_eq!(status.can_fix, can_fix);
    }

    #[test]
    fn parses_max_map_count_or_falls_back_to_zero() {
        assert_eq!(parse_max_map_count(Some("2147483642\n")), RECOMMENDED_MAX_MAP_COUNT);
        assert_eq!(parse_max_map_count(Some("not-a-number")), 0);
        assert_eq!(parse_max_map_count(None), 0);
    }

    #[test]
    fn builds_status_with_threshold_and_fix_availability() {
        assert_status(
            build_max_map_count_status(RECOMMENDED_MAX_MAP_COUNT - 1, false),
            RECOMMENDED_MAX_MAP_COUNT - 1,
            false,
            false,
        );
        assert_status(
            build_max_map_count_status(RECOMMENDED_MAX_MAP_COUNT, true),
            RECOMMENDED_MAX_MAP_COUNT,
            true,
            true,
        );
    }

    #[test]
    fn exposes_manual_and_pkexec_fix_commands() {
        assert_eq!(
            manual_fix_command(),
            format!(
                "sudo sh -c 'echo \"vm.max_map_count = {RECOMMENDED_MAX_MAP_COUNT}\" > {SYSCTL_DROPIN_PATH} && sysctl --system'"
            )
        );
        assert_eq!(
            sysctl_fix_script(),
            format!(
                "echo 'vm.max_map_count = {RECOMMENDED_MAX_MAP_COUNT}' > {SYSCTL_DROPIN_PATH} && sysctl --system"
            )
        );
    }

    #[test]
    fn reports_missing_pkexec_with_manual_fallback() {
        let error = finalize_max_map_count_fix(false, Ok((true, "0".to_string()))).unwrap_err();

        match error {
            AppError::PkexecNotInstalled { command } => {
                assert_eq!(command, manual_fix_command());
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn reports_failed_or_unspawnable_pkexec_runs() {
        let error = finalize_max_map_count_fix(true, Ok((false, "exit status: 126".to_string())))
            .unwrap_err();
        match error {
            AppError::SysctlChangeFailed { status } => {
                assert_eq!(status, "exit status: 126");
            }
            other => panic!("unexpected error: {other:?}"),
        }

        let error = finalize_max_map_count_fix(true, Err("Could not run pkexec: boom".to_string()))
            .unwrap_err();
        match error {
            AppError::Other { message } => {
                assert_eq!(message, "Could not run pkexec: boom");
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn accepts_successful_pkexec_runs() {
        assert!(finalize_max_map_count_fix(true, Ok((true, "exit status: 0".to_string()))).is_ok());
    }
}
