use crate::error::AppError;
use std::fs;
use std::path::PathBuf;

use tauri::{AppHandle, Manager, State};

use crate::config::{save_config, ConfigState};
use crate::models::{GraphicsConfig, VkBasaltSettings};

/// One file per game, since vkBasalt settings can be overridden per game
/// (see `GraphicsOverrides`) — a shared file would let a second game's launch
/// rewrite the effects of one that's still running.
pub(crate) fn vkbasalt_conf_path(app: &AppHandle, game_id: &str) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Could not resolve data directory: {e}"))?
        .join("vkbasalt")
        .join(format!("{game_id}.conf")))
}

/// Builds a vkBasalt.conf (see https://github.com/DadSchoorse/vkBasalt) from
/// the app's settings. Unlike MangoHud's boolean-by-presence format, vkBasalt
/// needs an explicit `effects = ` chain (colon-separated, applied in that
/// order) plus each active effect's own parameters — so, unlike
/// `mangohud::render_conf`, this always writes the enabled effects' tuning
/// values rather than just their presence.
///
/// Only vkBasalt's builtin effects (`cas`, `dls`, `fxaa`, `smaa`, `lut`) are
/// used: any other name in `effects` is looked up as a ReShade shader whose
/// path the config has to provide, and fails without one.
fn render_vkbasalt_conf(settings: &VkBasaltSettings) -> String {
    let mut effects = Vec::new();
    if settings.sharpen {
        effects.push("cas");
    }
    if settings.smaa {
        effects.push("smaa");
    }

    let mut lines = vec![format!("effects = {}", effects.join(":"))];

    if settings.sharpen {
        lines.push(format!("casSharpness = {}", settings.sharpness));
    }
    if settings.smaa {
        lines.push("smaaEdgeDetection = luma".to_string());
        lines.push("smaaThreshold = 0.05".to_string());
        lines.push("smaaMaxSearchSteps = 32".to_string());
        lines.push("smaaMaxSearchStepsDiag = 16".to_string());
        lines.push("smaaCornerRounding = 25".to_string());
    }

    lines.push("enableOnLaunch = True".to_string());
    lines.join("\n") + "\n"
}

/// Writes this game's `vkBasalt.conf` from the given settings and returns
/// its path, for `launch_game` to point `VKBASALT_CONFIG_FILE` at.
/// Regenerated on every launch — same rationale as
/// `mangohud::ensure_mangohud_conf`.
pub fn ensure_vkbasalt_conf(
    app: &AppHandle,
    game_id: &str,
    settings: &VkBasaltSettings,
) -> Result<PathBuf, String> {
    let path = vkbasalt_conf_path(app, game_id)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Could not create {}: {e}", parent.display()))?;
    }
    fs::write(&path, render_vkbasalt_conf(settings))
        .map_err(|e| format!("Could not write {}: {e}", path.display()))?;
    Ok(path)
}

#[tauri::command]
pub fn get_graphics_config(state: State<ConfigState>) -> Result<GraphicsConfig, AppError> {
    let config = state
        .lock()
        .map_err(|_| "Configuration is locked".to_string())?;
    Ok(config.graphics.clone())
}

#[tauri::command]
pub fn save_graphics_config(
    app: AppHandle,
    state: State<ConfigState>,
    config: GraphicsConfig,
) -> Result<GraphicsConfig, AppError> {
    let mut app_config = state
        .lock()
        .map_err(|_| "Configuration is locked".to_string())?;
    app_config.graphics = config;
    save_config(&app, &app_config)?;
    Ok(app_config.graphics.clone())
}

#[cfg(test)]
mod tests {
    use super::render_vkbasalt_conf;
    use crate::models::VkBasaltSettings;

    #[test]
    fn renders_enabled_vkbasalt_effects_in_order() {
        let settings = VkBasaltSettings {
            enabled: true,
            sharpen: true,
            sharpness: 0.65,
            smaa: true,
        };

        assert_eq!(
            render_vkbasalt_conf(&settings),
            "effects = cas:smaa\ncasSharpness = 0.65\nsmaaEdgeDetection = luma\nsmaaThreshold = 0.05\nsmaaMaxSearchSteps = 32\nsmaaMaxSearchStepsDiag = 16\nsmaaCornerRounding = 25\nenableOnLaunch = True\n"
        );
    }

    #[test]
    fn omits_disabled_vkbasalt_effects() {
        let settings = VkBasaltSettings {
            enabled: false,
            sharpen: false,
            sharpness: 0.4,
            smaa: false,
        };

        assert_eq!(
            render_vkbasalt_conf(&settings),
            "effects = \nenableOnLaunch = True\n"
        );
    }
}
