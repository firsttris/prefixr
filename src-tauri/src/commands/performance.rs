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
    let current = fs::read_to_string(MAX_MAP_COUNT_PATH)
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
        .unwrap_or(0);
    MaxMapCountStatus {
        current,
        recommended: RECOMMENDED_MAX_MAP_COUNT,
        sufficient: current >= RECOMMENDED_MAX_MAP_COUNT,
        can_fix: command_on_path("pkexec"),
    }
}

/// Writes the sysctl drop-in and applies it immediately via `pkexec` (a
/// PolicyKit prompt), rather than just `sysctl -w` alone — that would raise
/// the limit for this boot only and silently regress after the next reboot,
/// which would be more confusing than not offering a fix at all.
#[tauri::command]
pub async fn fix_max_map_count() -> Result<(), AppError> {
    if !command_on_path("pkexec") {
        return Err(AppError::PkexecNotInstalled {
            command: format!(
                "sudo sh -c 'echo \"vm.max_map_count = {RECOMMENDED_MAX_MAP_COUNT}\" > {SYSCTL_DROPIN_PATH} && sysctl --system'"
            ),
        });
    }
    let script = format!(
        "echo 'vm.max_map_count = {RECOMMENDED_MAX_MAP_COUNT}' > {SYSCTL_DROPIN_PATH} && sysctl --system"
    );
    let status = tokio::process::Command::new("pkexec")
        .args(["sh", "-c", &script])
        .status()
        .await
        .map_err(|e| format!("Could not run pkexec: {e}"))?;
    if !status.success() {
        return Err(AppError::SysctlChangeFailed {
            status: status.to_string(),
        });
    }
    Ok(())
}
