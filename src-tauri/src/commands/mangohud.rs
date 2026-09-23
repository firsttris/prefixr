use std::fs;
use std::path::PathBuf;

use tauri::{AppHandle, Manager, State};

use crate::config::{save_config, ConfigState};
use crate::models::{MangoHudConfig, MangoHudLayout};

/// Builds the contents of a `MangoHud.conf` (see
/// https://github.com/flightlessmango/MangoHud) from the app's friendlier
/// settings. MangoHud treats a boolean option as enabled simply by its key
/// being present in the file — there's no `=0` form — so a disabled stat is
/// left out entirely rather than written with a falsy value.
fn render_conf(config: &MangoHudLayout) -> String {
    let mut lines = Vec::new();

    if config.show_fps {
        lines.push("fps".to_string());
    }
    if config.show_frametime {
        lines.push("frametime".to_string());
    }
    if config.show_cpu {
        lines.push("cpu_stats".to_string());
    }
    if config.show_gpu {
        lines.push("gpu_stats".to_string());
    }
    if config.show_ram {
        lines.push("ram".to_string());
    }
    if config.show_vram {
        lines.push("vram".to_string());
    }
    if config.show_temps {
        lines.push("cpu_temp".to_string());
        lines.push("gpu_temp".to_string());
    }
    if config.show_gamemode {
        lines.push("gamemode".to_string());
    }
    if config.show_vkbasalt {
        lines.push("vkbasalt".to_string());
    }
    if config.show_hdr {
        lines.push("hdr".to_string());
    }
    if config.show_driver {
        lines.push("vulkan_driver".to_string());
    }
    if config.show_engine_version {
        lines.push("engine_version".to_string());
    }
    if config.show_wine {
        lines.push("wine".to_string());
    }
    if config.show_gpu_name {
        lines.push("gpu_name".to_string());
    }
    if config.show_resolution {
        lines.push("resolution".to_string());
    }
    if config.horizontal {
        lines.push("horizontal".to_string());
    }

    lines.push(format!("position={}", config.position));
    lines.push(format!("background_alpha={}", config.background_alpha));
    lines.push(format!("text_color={}", config.theme_color));
    if config.round_corners {
        lines.push("round_corners=10".to_string());
    }

    lines.join("\n") + "\n"
}

/// One file per game, since the layout can be overridden per game (see
/// `OverlayOverrides`) — same reasoning as `graphics::vkbasalt_conf_path`.
pub(crate) fn mangohud_conf_path(app: &AppHandle, game_id: &str) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Could not resolve data directory: {e}"))?
        .join("mangohud")
        .join(format!("{game_id}.conf")))
}

/// Writes this game's `MangoHud.conf` from the given layout and returns
/// its path, for `launch_game` to point `MANGOHUD_CONFIGFILE` at. Rewritten
/// on every launch rather than only on save — cheap, and avoids needing to
/// track whether the on-disk file is stale relative to `AppConfig`.
pub fn ensure_mangohud_conf(
    app: &AppHandle,
    game_id: &str,
    layout: &MangoHudLayout,
) -> Result<PathBuf, String> {
    let path = mangohud_conf_path(app, game_id)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Could not create {}: {e}", parent.display()))?;
    }
    fs::write(&path, render_conf(layout))
        .map_err(|e| format!("Could not write {}: {e}", path.display()))?;
    Ok(path)
}

#[tauri::command]
pub fn get_mangohud_config(state: State<ConfigState>) -> Result<MangoHudConfig, String> {
    let config = state
        .lock()
        .map_err(|_| "Configuration is locked".to_string())?;
    Ok(config.mangohud.clone())
}

#[tauri::command]
pub fn save_mangohud_config(
    app: AppHandle,
    state: State<ConfigState>,
    config: MangoHudConfig,
) -> Result<MangoHudConfig, String> {
    let mut app_config = state
        .lock()
        .map_err(|_| "Configuration is locked".to_string())?;
    app_config.mangohud = config;
    save_config(&app, &app_config)?;
    Ok(app_config.mangohud.clone())
}
