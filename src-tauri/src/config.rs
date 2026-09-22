use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::models::{
    Game, GitHubConfig, MangoHudConfig, PerformanceConfig, PrefixInfo, SteamGridDbConfig,
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
    serde_json::from_str(&raw).map_err(|e| format!("Config file is invalid: {e}"))
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
