use std::fs;
use std::path::PathBuf;

use tauri::{AppHandle, Manager, State};

use crate::config::{save_config, ConfigState};
use crate::models::PerformanceConfig;

fn vkbasalt_conf_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Could not resolve data directory: {e}"))?
        .join("vkbasalt")
        .join("vkBasalt.conf"))
}

/// Builds a vkBasalt.conf (see https://github.com/DadSchoorse/vkBasalt) from
/// the app's settings. Unlike MangoHud's boolean-by-presence format, vkBasalt
/// needs an explicit `effects = ` chain (colon-separated, applied in that
/// order) plus each active effect's own parameters — so, unlike
/// `mangohud::render_conf`, this always writes the enabled effects' tuning
/// values rather than just their presence.
fn render_vkbasalt_conf(config: &PerformanceConfig) -> String {
    let mut effects = Vec::new();
    if config.vkbasalt_sharpen {
        effects.push("cas");
    }
    if config.vkbasalt_smaa {
        effects.push("smaa");
    }
    if config.vkbasalt_deband {
        effects.push("deband");
    }

    let mut lines = vec![format!("effects = {}", effects.join(":"))];

    if config.vkbasalt_sharpen {
        lines.push(format!("casSharpness = {}", config.vkbasalt_sharpness));
    }
    if config.vkbasalt_smaa {
        lines.push("smaaEdgeDetection = luma".to_string());
        lines.push("smaaThreshold = 0.05".to_string());
        lines.push("smaaMaxSearchSteps = 32".to_string());
        lines.push("smaaMaxSearchStepsDiag = 16".to_string());
        lines.push("smaaCornerRounding = 25".to_string());
    }
    if config.vkbasalt_deband {
        lines.push("debandAvoidBanding = 3".to_string());
    }

    lines.push("enableOnLaunch = True".to_string());
    lines.join("\n") + "\n"
}

/// Writes the app's own `vkBasalt.conf` from the given settings and returns
/// its path, for `launch_game` to point `VKBASALT_CONFIG_FILE` at.
/// Regenerated on every launch — same rationale as
/// `mangohud::ensure_mangohud_conf`.
pub fn ensure_vkbasalt_conf(app: &AppHandle, config: &PerformanceConfig) -> Result<PathBuf, String> {
    let path = vkbasalt_conf_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Could not create {}: {e}", parent.display()))?;
    }
    fs::write(&path, render_vkbasalt_conf(config))
        .map_err(|e| format!("Could not write {}: {e}", path.display()))?;
    Ok(path)
}

#[tauri::command]
pub fn get_performance_config(state: State<ConfigState>) -> Result<PerformanceConfig, String> {
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
) -> Result<PerformanceConfig, String> {
    let mut app_config = state
        .lock()
        .map_err(|_| "Configuration is locked".to_string())?;
    app_config.performance = config;
    save_config(&app, &app_config)?;
    Ok(app_config.performance.clone())
}
