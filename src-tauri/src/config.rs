use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use tauri::{AppHandle, Manager};

use crate::models::{
    Game, GitHubConfig, GraphicsConfig, MangoHudConfig, PerformanceConfig, PrefixInfo,
    ProtonConfig, SteamGridDbConfig,
};

const CONFIG_FILE_NAME: &str = "config.json";

/// The full persisted application state, stored as a single JSON file in the
/// platform config directory (e.g. `~/.config/prefixr/config.json`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub runners_dir: PathBuf,
    #[serde(default)]
    pub prefixes: Vec<PrefixInfo>,
    #[serde(default)]
    pub games: Vec<Game>,
    #[serde(default)]
    pub mangohud: MangoHudConfig,
    #[serde(default)]
    pub performance: PerformanceConfig,
    #[serde(default)]
    pub graphics: GraphicsConfig,
    #[serde(default)]
    pub proton: ProtonConfig,
    #[serde(default)]
    pub steamgriddb: SteamGridDbConfig,
    #[serde(default)]
    pub github: GitHubConfig,
}

impl AppConfig {
    fn default_for(app: &AppHandle) -> Result<Self, String> {
        let data_dir = app
            .path()
            .app_data_dir()
            .map_err(|e| format!("Could not resolve data directory: {e}"))?;
        Ok(Self {
            runners_dir: data_dir.join("runners"),
            prefixes: Vec::new(),
            games: Vec::new(),
            mangohud: MangoHudConfig::default(),
            performance: PerformanceConfig::default(),
            graphics: GraphicsConfig::default(),
            proton: ProtonConfig::default(),
            steamgriddb: SteamGridDbConfig::default(),
            github: GitHubConfig::default(),
        })
    }
}

/// Shared, lock-protected app state managed by Tauri and injected into commands
/// as `State<ConfigState>`.
pub type ConfigState = Mutex<AppConfig>;

fn config_file_path(app: &AppHandle) -> Result<PathBuf, String> {
    let config_dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("Could not resolve config directory: {e}"))?;
    Ok(config_dir.join(CONFIG_FILE_NAME))
}

pub fn load_config(app: &AppHandle) -> Result<AppConfig, String> {
    let path = config_file_path(app)?;
    if !path.exists() {
        return AppConfig::default_for(app);
    }
    let raw = fs::read_to_string(&path).map_err(|e| format!("Could not read config file: {e}"))?;
    let mut value: Value =
        serde_json::from_str(&raw).map_err(|e| format!("Config file is invalid: {e}"))?;
    migrate(&mut value);
    serde_json::from_value(value).map_err(|e| format!("Config file is invalid: {e}"))
}

/// Brings a config written by an older version into the current shape. The
/// file is rewritten in the new shape on the next save.
///
/// - gamescope and vkBasalt used to live in `performance` as flat
///   `gamescope_*`/`vkbasalt_*` fields; they now have their own `graphics`
///   block.
/// - A game's `overrides` used to be one flat object (`mangohud_enabled`,
///   `gamemode_enabled`, `vkbasalt`, `gamescope`, `proton_options`); it's now
///   split by the same categories as the global settings.
fn migrate(config: &mut Value) {
    let Some(config) = config.as_object_mut() else {
        return;
    };

    if !config.contains_key("graphics") {
        if let Some(performance) = config.get("performance").and_then(Value::as_object) {
            let take = |prefix: &str| -> Map<String, Value> {
                performance
                    .iter()
                    .filter_map(|(k, v)| Some((k.strip_prefix(prefix)?.to_string(), v.clone())))
                    .collect()
            };
            let graphics = json!({ "gamescope": take("gamescope_"), "vkbasalt": take("vkbasalt_") });
            config.insert("graphics".to_string(), graphics);
        }
    }

    const LEGACY_OVERRIDE_KEYS: &[&str] = &[
        "mangohud_enabled",
        "gamemode_enabled",
        "vkbasalt",
        "gamescope",
        "proton_options",
    ];
    let games = config.get_mut("games").and_then(Value::as_array_mut);
    for game in games.into_iter().flatten() {
        let Some(overrides) = game.get_mut("overrides") else {
            continue;
        };
        let Some(old) = overrides.as_object() else {
            continue;
        };
        if !LEGACY_OVERRIDE_KEYS.iter().any(|k| old.contains_key(*k)) {
            continue;
        }
        let get = |key: &str| old.get(key).cloned().unwrap_or(Value::Null);
        *overrides = json!({
            "performance": { "gamemode_enabled": get("gamemode_enabled") },
            "graphics": { "gamescope": get("gamescope"), "vkbasalt": get("vkbasalt") },
            "overlay": { "enabled": get("mangohud_enabled") },
            "proton": old.get("proton_options").cloned().unwrap_or_else(|| json!({})),
        });
    }
}

pub fn save_config(app: &AppHandle, config: &AppConfig) -> Result<(), String> {
    let path = config_file_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Could not create config directory: {e}"))?;
    }
    let raw = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Could not serialize config: {e}"))?;
    fs::write(&path, raw).map_err(|e| format!("Could not write config file: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_flat_performance_and_overrides() {
        let mut value = json!({
            "runners_dir": "/r",
            "performance": {
                "gamemode_enabled": true,
                "power_profile_enabled": false,
                "inhibit_sleep_enabled": true,
                "vkbasalt_enabled": true,
                "vkbasalt_sharpen": true,
                "vkbasalt_sharpness": 0.6,
                "vkbasalt_smaa": false,
                "vkbasalt_deband": true,
                "gamescope_enabled": true,
                "gamescope_width": 1280,
                "gamescope_height": null,
                "gamescope_fps_limit": 40,
                "gamescope_fullscreen": false
            },
            "games": [{
                "id": "00000000-0000-0000-0000-000000000000",
                "name": "x",
                "exe_path": "/x.exe",
                "prefix_path": "/p",
                "runner_id": "r",
                "overrides": {
                    "mangohud_enabled": false,
                    "gamemode_enabled": null,
                    "vkbasalt": null,
                    "gamescope": {
                        "enabled": true, "width": 800, "height": 600,
                        "fps_limit": null, "fullscreen": true
                    },
                    "proton_options": { "PROTON_ENABLE_HDR": true }
                }
            }]
        });
        migrate(&mut value);
        let config: AppConfig = serde_json::from_value(value).unwrap();

        assert!(config.performance.gamemode_enabled);
        assert!(config.performance.inhibit_sleep_enabled);
        assert!(config.graphics.vkbasalt.enabled);
        assert!(config.graphics.vkbasalt.deband);
        assert!((config.graphics.vkbasalt.sharpness - 0.6).abs() < 1e-6);
        assert!(config.graphics.gamescope.enabled);
        assert_eq!(config.graphics.gamescope.width, Some(1280));
        assert_eq!(config.graphics.gamescope.fps_limit, Some(40));

        let overrides = &config.games[0].overrides;
        assert_eq!(overrides.overlay.enabled, Some(false));
        assert!(overrides.performance.gamemode_enabled.is_none());
        assert!(overrides.graphics.vkbasalt.is_none());
        assert_eq!(overrides.graphics.gamescope.as_ref().unwrap().width, Some(800));
        assert_eq!(overrides.proton.get("PROTON_ENABLE_HDR"), Some(&true));
    }

    #[test]
    fn leaves_current_shape_alone() {
        let mut value = json!({
            "runners_dir": "/r",
            "graphics": { "gamescope": { "enabled": true } },
            "games": [{
                "id": "00000000-0000-0000-0000-000000000000",
                "name": "x",
                "exe_path": "/x.exe",
                "prefix_path": "/p",
                "runner_id": "r",
                "overrides": { "overlay": { "enabled": true }, "proton": { "A": false } }
            }]
        });
        let before = value.clone();
        migrate(&mut value);
        assert_eq!(value, before);
    }
}
