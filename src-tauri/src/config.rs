use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use tauri::{AppHandle, Emitter, Manager};

use crate::commands::icons;
use crate::commands::library::LibraryConfig;
use crate::commands::prefixes::effective_prefix_path;
use crate::lock::LockExt;
use crate::models::{
    Game, GitHubConfig, GraphicsConfig, MangoHudConfig, PerformanceConfig, PrefixInfo,
    ProtonConfig, SteamGridDbConfig, StoresConfig,
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
    #[serde(default)]
    pub stores: StoresConfig,
    #[serde(default)]
    pub library: LibraryConfig,
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
            stores: StoresConfig::default(),
            library: LibraryConfig::default(),
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

/// Holds `config.json.lock` exclusively, waiting while another process
/// holds it. A command line call that changes the config holds it from
/// reading the config until it exits, so two of them running at once can't
/// write their own older copies over each other's changes. Released when
/// the file is closed, including when the process ends in any way.
pub fn lock_config(app: &AppHandle) -> Result<fs::File, String> {
    lock_file(&config_file_path(app)?.with_extension("json.lock"))
}

fn lock_file(path: &Path) -> Result<fs::File, String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Could not create config directory: {e}"))?;
    }
    let file = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(path)
        .map_err(|e| format!("Could not open {}: {e}", path.display()))?;
    file.lock()
        .map_err(|e| format!("Could not lock {}: {e}", path.display()))?;
    Ok(file)
}

pub fn load_config(app: &AppHandle) -> Result<AppConfig, String> {
    let path = config_file_path(app)?;
    if !path.exists() {
        return AppConfig::default_for(app);
    }
    let invalid = |e: &dyn std::fmt::Display| format!("{} is invalid: {e}", path.display());
    let raw =
        fs::read_to_string(&path).map_err(|e| format!("Could not read {}: {e}", path.display()))?;
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

/// Fills in each game's exe icon path (see `icons::exe_icon_path`). A
/// config from before those files existed still has the icons inline as
/// `data:` URIs: they're moved to files here, and `save_config` leaves the
/// field out from then on. Best-effort throughout, the icons are only
/// cosmetic.
fn load_icons(app: &AppHandle, config: &mut AppConfig) {
    for game in &mut config.games {
        if let Some(data_url) = &game.icon {
            let missing = icons::exe_icon_path(app, game.id).is_ok_and(|path| !path.exists());
            if let (true, Some(png)) = (missing, icons::data_url_png(data_url)) {
                let _ = icons::store_exe_icon(app, game.id, Some(&png));
            }
        }
        game.icon = icons::exe_icon_file(app, game.id);
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
            let graphics =
                json!({ "gamescope": take("gamescope_"), "vkbasalt": take("vkbasalt_") });
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
    fs::rename(&tmp, &path).map_err(|e| format!("Could not write config file: {e}"))?;
    if let Some(known) = app.try_state::<ConfigFileStamp>() {
        known.set(FileStamp::of(&path));
    }
    Ok(())
}

/// Changes the config as it is on disk now, for a change made long after
/// this process read it: the end of a store install, a game's play time.
/// The config lock keeps other processes' changes out meanwhile, and this
/// process's own state is locked before the file is read, so nothing
/// changed here in between is lost either.
///
/// Not for a process that holds the config lock itself (a command line
/// call that changes the config holds it from the start): it would wait for
/// itself.
pub fn update_config<T>(
    app: &AppHandle,
    change: impl FnOnce(&mut AppConfig) -> Result<T, String>,
) -> Result<T, String> {
    let _lock = lock_config(app)?;
    let state = app.state::<ConfigState>();
    let mut current = state.locked();
    let mut config = load_config(app)?;
    let result = change(&mut config)?;
    save_config(app, &config)?;
    *current = config;
    Ok(result)
}

/// When and how big `config.json` was, to tell a change by another process
/// (the command line, a `--run` Prefixr) from this process's own writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FileStamp {
    modified: SystemTime,
    len: u64,
}

impl FileStamp {
    pub(crate) fn of(path: &Path) -> Option<Self> {
        let metadata = fs::metadata(path).ok()?;
        Some(Self {
            modified: metadata.modified().ok()?,
            len: metadata.len(),
        })
    }
}

/// `config.json` as this process last read or wrote it.
#[derive(Default)]
pub struct ConfigFileStamp(Mutex<Option<FileStamp>>);

impl ConfigFileStamp {
    fn get(&self) -> Option<FileStamp> {
        *self.0.locked()
    }

    fn set(&self, stamp: Option<FileStamp>) {
        *self.0.locked() = stamp;
    }
}

/// How often the window's process looks for changes to `config.json`.
const WATCH_INTERVAL: Duration = Duration::from_millis(500);

/// Picks up changes another process makes to `config.json` — the command
/// line adding a game, say — while the window is open, and tells the
/// frontend with `config-changed`. Without this, the window's next save would
/// write its older copy over them.
///
/// A file that can't be read (a hand edit in progress) is skipped until it
/// changes again; the window keeps what it has.
pub fn watch_config_file(app: &AppHandle) {
    let Ok(path) = config_file_path(app) else {
        return;
    };
    app.state::<ConfigFileStamp>().set(FileStamp::of(&path));
    let app = app.clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(WATCH_INTERVAL);
        let known = app.state::<ConfigFileStamp>();
        let current = FileStamp::of(&path);
        if current == known.get() {
            continue;
        }
        let loaded = load_config(&app);
        // In the same order as `save_config` takes them: the config, then
        // the stamp.
        let state = app.state::<ConfigState>();
        let mut config = state.locked();
        // Checked again while no save of this process's own can happen: one
        // since the check above already took the file as it is now, and a
        // file that changed once more is read again on the next round.
        let now = FileStamp::of(&path);
        if now == known.get() || now != current {
            continue;
        }
        known.set(current);
        if let Ok(loaded) = loaded {
            *config = loaded;
            drop(config);
            let _ = app.emit("config-changed", ());
        }
    });
}

#[cfg(test)]
mod tests;
