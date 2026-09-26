use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use tauri::{AppHandle, Manager};

use crate::commands::icons;
use crate::commands::prefixes::effective_prefix_path;
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
    let invalid = |e: &dyn std::fmt::Display| format!("{} is invalid: {e}", path.display());
    let raw = fs::read_to_string(&path)
        .map_err(|e| format!("Could not read {}: {e}", path.display()))?;
    let mut value: Value = serde_json::from_str(&raw).map_err(|e| invalid(&e))?;
    migrate(&mut value);
    let mut config: AppConfig = serde_json::from_value(value).map_err(|e| invalid(&e))?;
    load_icons(app, &mut config);
    fix_prefix_paths(&mut config);
    Ok(config)
}

/// A Proton compat data folder added before `effective_prefix_path` existed
/// was registered as it is, rather than its `pfx/` — and games set up in it
/// with it. Both are pointed at the actual prefix; the file follows on the
/// next save.
fn fix_prefix_paths(config: &mut AppConfig) {
    let mut seen = HashSet::new();
    config.prefixes.retain_mut(|prefix| {
        prefix.path = effective_prefix_path(&prefix.path);
        seen.insert(prefix.path.clone())
    });
    for game in &mut config.games {
        game.prefix_path = effective_prefix_path(&game.prefix_path);
    }
}

/// Fills in each game's exe icon from its file (see `icons::exe_icon_path`).
/// A config from before those files existed still has the icons inline:
/// they're moved to files here, and `save_config` leaves them out from then
/// on. Best-effort throughout, the icons are only cosmetic.
fn load_icons(app: &AppHandle, config: &mut AppConfig) {
    for game in &mut config.games {
        match &game.icon {
            Some(data_url) => {
                let missing = icons::exe_icon_path(app, game.id).is_ok_and(|path| !path.exists());
                if let (true, Some(png)) = (missing, icons::data_url_png(data_url)) {
                    let _ = icons::store_exe_icon(app, game.id, Some(&png));
                }
            }
            None => game.icon = icons::load_exe_icon(app, game.id),
        }
    }
}

/// The config as written to disk: everything but the games' exe icons,
/// which live in files of their own (see `load_icons`).
fn to_disk_json(config: &AppConfig) -> Result<String, String> {
    let mut value =
        serde_json::to_value(config).map_err(|e| format!("Could not serialize config: {e}"))?;
    let games = value.get_mut("games").and_then(Value::as_array_mut);
    for game in games.into_iter().flatten() {
        if let Some(game) = game.as_object_mut() {
            game.remove("icon");
        }
    }
    serde_json::to_string_pretty(&value).map_err(|e| format!("Could not serialize config: {e}"))
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

/// Writes the config to a temporary file first and swaps it in with a
/// rename, so a crash or a full disk mid-write never leaves a truncated
/// `config.json` behind — `load_config` refuses to start from one.
pub fn save_config(app: &AppHandle, config: &AppConfig) -> Result<(), String> {
    let path = config_file_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Could not create config directory: {e}"))?;
    }
    let raw = to_disk_json(config)?;
    let tmp = path.with_extension("json.tmp");
    let write = |tmp: &PathBuf| -> std::io::Result<()> {
        let mut file = fs::File::create(tmp)?;
        file.write_all(raw.as_bytes())?;
        file.sync_all()
    };
    write(&tmp).map_err(|e| format!("Could not write config file: {e}"))?;
    fs::rename(&tmp, &path).map_err(|e| format!("Could not write config file: {e}"))
}

    #[cfg(test)]
    mod tests;
