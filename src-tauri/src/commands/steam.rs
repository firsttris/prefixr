use crate::error::AppError;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, State};
use uuid::Uuid;

use crate::commands::binary_vdf::{self, Map, Value};
use crate::commands::games::{command_on_path, own_executable_path, write_shortcut_icon};
use crate::commands::steamgriddb::{artwork_dir, asset_cache_path, image_extension};
use crate::config::ConfigState;
use crate::models::{ArtworkKind, Game};

/// How long `steam -shutdown` gets before the export gives up.
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(30);

/// Extensions Steam picks up in its `grid` folder. A stale file in another
/// format would otherwise win over the one we just wrote.
const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp"];

/// Steam's artwork kinds, by grid file suffix: portrait cover, wide grid,
/// hero banner, logo, icon.
const GRID_SUFFIXES: &[&str] = &["p", "", "_hero", "_logo", "_icon"];

fn home_dir() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

/// Native Steam's data directory. `~/.steam/root` is Steam's own pointer to
/// it; the others are fallbacks for setups where that link is missing.
fn steam_root() -> Result<PathBuf, AppError> {
    let home = home_dir()?;
    let data_home = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".local/share"));
    let candidates = [
        home.join(".steam/root"),
        home.join(".steam/steam"),
        data_home.join("Steam"),
    ];
    if let Some(root) = candidates.into_iter().find(|p| p.join("userdata").is_dir()) {
        return Ok(root);
    }
    if home
        .join(".var/app/com.valvesoftware.Steam/.local/share/Steam/userdata")
        .is_dir()
    {
        return Err(AppError::SteamOnlyFlatpak);
    }
    Err(AppError::SteamNotFound)
}

/// The account that used Steam last: Steam rewrites its `localconfig.vdf`
/// whenever that account logs in or out. `0` is the anonymous account.
fn steam_user_dir(root: &Path) -> Result<PathBuf, AppError> {
    let entries = fs::read_dir(root.join("userdata"))
        .map_err(|e| format!("Could not read Steam's userdata directory: {e}"))?;
    entries
        .filter_map(Result::ok)
        .filter(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            name != "0" && !name.is_empty() && name.chars().all(|c| c.is_ascii_digit())
        })
        .map(|entry| {
            let path = entry.path();
            let modified = fs::metadata(path.join("config/localconfig.vdf"))
                .and_then(|m| m.modified())
                .ok();
            (modified, path)
        })
        .max_by_key(|(modified, _)| *modified)
        .map(|(_, path)| path)
        .ok_or(AppError::NoSteamAccountFound)
}

/// Whether any `steam` process runs: the client itself or its launcher
/// script, which both show up under that name.
fn steam_running() -> bool {
    let Ok(entries) = fs::read_dir("/proc") else {
        return false;
    };
    entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().chars().all(|c| c.is_ascii_digit()))
        .any(|entry| {
            fs::read_to_string(entry.path().join("comm"))
                .is_ok_and(|comm| comm.trim_end() == "steam")
        })
}

/// Asks Steam to quit and waits until it has. A running Steam keeps its own
/// copy of the shortcuts and writes it back, which would undo the export.
async fn stop_steam() -> Result<(), AppError> {
    if !command_on_path("steam") {
        return Err(AppError::SteamCommandNotFound);
    }
    let deadline = Instant::now() + SHUTDOWN_TIMEOUT;
    let mut child = tokio::process::Command::new("steam")
        .arg("-shutdown")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Could not ask Steam to quit: {e}"))?;
    // The `-shutdown` call is a `steam` process itself, so it has to be
    // gone before `steam_running` can tell anything.
    let _ = tokio::time::timeout(SHUTDOWN_TIMEOUT, child.wait()).await;
    while steam_running() {
        if Instant::now() > deadline {
            return Err(AppError::SteamShutdownTimedOut);
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    Ok(())
}

/// Starts Steam again after `stop_steam`, in its own process group so it
/// doesn't go down with Prefixr. Best-effort: the export itself is done.
fn start_steam() {
    let _ = tokio::process::Command::new("steam")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn();
}

/// The app id for a game's shortcut. Steam reads it from the shortcut's
/// `appid` field and names the grid images after it. Deriving it from the
/// game id (rather than from exe and name, as Steam once did) keeps it
/// stable when the game is renamed; the high bit marks a non-Steam game.
fn shortcut_app_id(game_id: Uuid) -> u32 {
    let mut crc = flate2::Crc::new();
    crc.update(game_id.to_string().as_bytes());
    crc.sum() | 0x8000_0000
}

/// `--run` rather than a desktop shortcut's `--launch`: the game runs in a
/// windowless Prefixr that exits with it, so Steam sees when it ends.
fn launch_options(game_id: Uuid) -> String {
    format!("--run {game_id}")
}

/// The index of the game's existing shortcut, recognized by its launch
/// options.
fn find_shortcut(shortcuts: &Map, game_id: Uuid) -> Option<usize> {
    let id = game_id.to_string();
    shortcuts.iter().position(|(_, entry)| match entry {
        Value::Map(fields) => matches!(
            binary_vdf::get(fields, "LaunchOptions"),
            Some(Value::String(options)) if options.contains(&id)
        ),
        _ => false,
    })
}

fn existing_app_id(shortcuts: &Map, game_id: Uuid) -> Option<u32> {
    let Value::Map(fields) = &shortcuts[find_shortcut(shortcuts, game_id)?].1 else {
        return None;
    };
    match binary_vdf::get(fields, "appid") {
        Some(Value::Int(app_id)) => Some(*app_id),
        _ => None,
    }
}

fn quoted(path: &Path) -> String {
    format!("\"{}\"", path.display())
}

/// Adds the game's shortcut or updates the one from an earlier export. An
/// update only touches the fields Prefixr owns, so what the user changed in
/// Steam (collections, hidden, own launch settings) stays.
fn upsert_shortcut(
    shortcuts: &mut Map,
    game: &Game,
    app_id: u32,
    exe: &Path,
    icon: Option<&Path>,
) {
    let index = find_shortcut(shortcuts, game.id).unwrap_or_else(|| {
        let s = |v: &str| Value::String(v.to_string());
        let defaults: Map = vec![
            ("IsHidden".into(), Value::Int(0)),
            ("AllowDesktopConfig".into(), Value::Int(1)),
            ("AllowOverlay".into(), Value::Int(1)),
            ("OpenVR".into(), Value::Int(0)),
            ("Devkit".into(), Value::Int(0)),
            ("DevkitGameID".into(), s("")),
            ("DevkitOverrideAppID".into(), Value::Int(0)),
            ("LastPlayTime".into(), Value::Int(0)),
            ("FlatpakAppID".into(), s("")),
            ("ShortcutPath".into(), s("")),
            ("tags".into(), Value::Map(Map::new())),
        ];
        shortcuts.push((String::new(), Value::Map(defaults)));
        shortcuts.len() - 1
    });
    let Value::Map(fields) = &mut shortcuts[index].1 else {
        unreachable!("find_shortcut only matches maps");
    };

    let start_dir = exe.parent().unwrap_or(exe);
    binary_vdf::set(fields, "appid", Value::Int(app_id));
    binary_vdf::set(fields, "AppName", Value::String(game.name.clone()));
    binary_vdf::set(fields, "Exe", Value::String(quoted(exe)));
    binary_vdf::set(fields, "StartDir", Value::String(quoted(start_dir)));
    binary_vdf::set(
        fields,
        "icon",
        Value::String(icon.map(|p| p.display().to_string()).unwrap_or_default()),
    );
    binary_vdf::set(fields, "LaunchOptions", Value::String(launch_options(game.id)));

    renumber(shortcuts);
}

/// Steam expects the entries keyed 0, 1, 2, … in order.
fn renumber(shortcuts: &mut Map) {
    for (i, (key, _)) in shortcuts.iter_mut().enumerate() {
        *key = i.to_string();
    }
}

/// Drops the game's shortcut, returning its app id if it had one.
fn remove_entry(shortcuts: &mut Map, game_id: Uuid) -> Option<Option<u32>> {
    let index = find_shortcut(shortcuts, game_id)?;
    let app_id = existing_app_id(shortcuts, game_id);
    shortcuts.remove(index);
    renumber(shortcuts);
    Some(app_id)
}

fn grid_paths(grid_dir: &Path, app_id: u32, suffix: &str) -> Vec<PathBuf> {
    IMAGE_EXTENSIONS
        .iter()
        .map(|ext| grid_dir.join(format!("{app_id}{suffix}.{ext}")))
        .collect()
}

fn remove_grid_files(grid_dir: &Path, app_id: u32, suffix: &str) {
    for path in grid_paths(grid_dir, app_id, suffix) {
        let _ = fs::remove_file(path);
    }
}

/// Copies an image into Steam's grid folder as `<app id><suffix>.<ext>`
/// (`p` is the portrait cover, `_icon` the icon), replacing any earlier one.
fn copy_to_grid(grid_dir: &Path, app_id: u32, suffix: &str, source: &Path) -> Result<PathBuf, String> {
    remove_grid_files(grid_dir, app_id, suffix);
    let ext = source.extension().and_then(|e| e.to_str()).unwrap_or("png");
    let target = grid_dir.join(format!("{app_id}{suffix}.{ext}"));
    fs::copy(source, &target).map_err(|e| format!("Could not copy artwork to Steam: {e}"))?;
    Ok(target)
}

fn read_shortcuts_file(path: &Path) -> Result<Map, AppError> {
    if !path.exists() {
        return Ok(Map::new());
    }
    let bytes = fs::read(path).map_err(|e| format!("Could not read shortcuts.vdf: {e}"))?;
    binary_vdf::parse(&bytes).map_err(|e| AppError::ShortcutsVdfUnreadable { error: e })
}

/// Keeps the previous file as `shortcuts.vdf.bak` and swaps the new one in
/// with a rename, so a failed write never leaves Steam with half a file.
fn write_shortcuts_file(path: &Path, file: &Map) -> Result<(), String> {
    if path.exists() {
        fs::copy(path, path.with_extension("vdf.bak"))
            .map_err(|e| format!("Could not back up shortcuts.vdf: {e}"))?;
    }
    let tmp = path.with_extension("vdf.tmp");
    fs::write(&tmp, binary_vdf::write(file))
        .map_err(|e| format!("Could not write shortcuts.vdf: {e}"))?;
    fs::rename(&tmp, path).map_err(|e| format!("Could not write shortcuts.vdf: {e}"))
}

struct SteamUser {
    grid_dir: PathBuf,
    shortcuts_path: PathBuf,
}

impl SteamUser {
    fn find() -> Result<Self, AppError> {
        let config_dir = steam_user_dir(&steam_root()?)?.join("config");
        Ok(Self {
            grid_dir: config_dir.join("grid"),
            shortcuts_path: config_dir.join("shortcuts.vdf"),
        })
    }
}

fn add_shortcut(app: &AppHandle, game: &Game, user: &SteamUser) -> Result<(), AppError> {
    let grid_dir = &user.grid_dir;
    fs::create_dir_all(grid_dir).map_err(|e| format!("Could not create Steam's grid folder: {e}"))?;

    let vdf_path = &user.shortcuts_path;
    let mut file = read_shortcuts_file(vdf_path)?;
    let shortcuts = binary_vdf::map_entry(&mut file, "shortcuts");
    let app_id = existing_app_id(shortcuts, game.id).unwrap_or_else(|| shortcut_app_id(game.id));

    // Only artwork the user picked goes to Steam; a slot without a pick
    // keeps whatever Steam has.
    let cache_dir = artwork_dir(app)?;
    let picked = game
        .cover_url
        .iter()
        .map(|url| ("", "p", url))
        .chain(ArtworkKind::ALL.iter().filter_map(|kind| {
            let url = game.artwork.get(kind)?;
            Some((kind.cache_suffix(), kind.grid_suffix(), url))
        }));
    for (cache_suffix, grid_suffix, url) in picked {
        let cached = asset_cache_path(&cache_dir, game.id, cache_suffix, image_extension(url));
        if cached.exists() {
            copy_to_grid(grid_dir, app_id, grid_suffix, &cached)?;
        }
    }
    let icon = match write_shortcut_icon(app, game)? {
        Some(source) => Some(copy_to_grid(grid_dir, app_id, "_icon", &source)?),
        None => None,
    };

    upsert_shortcut(shortcuts, game, app_id, &own_executable_path()?, icon.as_deref());
    write_shortcuts_file(vdf_path, &file).map_err(AppError::from)
}

fn remove_shortcut(game_id: Uuid, user: &SteamUser) -> Result<(), AppError> {
    let mut file = read_shortcuts_file(&user.shortcuts_path)?;
    let shortcuts = binary_vdf::map_entry(&mut file, "shortcuts");
    let Some(app_id) = remove_entry(shortcuts, game_id) else {
        return Ok(());
    };
    write_shortcuts_file(&user.shortcuts_path, &file)?;
    if let Some(app_id) = app_id {
        for suffix in GRID_SUFFIXES {
            remove_grid_files(&user.grid_dir, app_id, suffix);
        }
    }
    Ok(())
}

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum SteamChange {
    /// `restarted_steam`: Steam was quit for the change and started again.
    Done { restarted_steam: bool },
    /// Steam runs and `shutdown_steam` wasn't set; nothing was changed.
    SteamRunning,
}

/// Applies a change to Steam's shortcuts. Steam has to be closed for that:
/// with `shutdown_steam` it is quit and restarted around the change,
/// otherwise a running Steam is reported back so the user can decide.
async fn change_steam(
    shutdown_steam: bool,
    change: impl FnOnce() -> Result<(), AppError>,
) -> Result<SteamChange, AppError> {
    let was_running = steam_running();
    if was_running {
        if !shutdown_steam {
            return Ok(SteamChange::SteamRunning);
        }
        stop_steam().await?;
    }
    let result = change();
    if was_running {
        start_steam();
    }
    result.map(|()| SteamChange::Done {
        restarted_steam: was_running,
    })
}

fn find_game(state: &State<ConfigState>, id: &str) -> Result<Game, String> {
    let game_id = Uuid::parse_str(id).map_err(|e| format!("Invalid game id: {e}"))?;
    let config = state
        .lock()
        .map_err(|_| "Configuration is locked".to_string())?;
    config
        .games
        .iter()
        .find(|g| g.id == game_id)
        .cloned()
        .ok_or_else(|| format!("No game with id {id}"))
}

/// Adds a game to Steam as a non-Steam game (or updates its entry from an
/// earlier export), with the artwork picked for it. Steam starts it via
/// `--run <game-id>`.
#[tauri::command]
pub async fn export_to_steam(
    app: AppHandle,
    state: State<'_, ConfigState>,
    id: String,
    shutdown_steam: bool,
) -> Result<SteamChange, AppError> {
    let game = find_game(&state, &id)?;
    let user = SteamUser::find()?;
    change_steam(shutdown_steam, || add_shortcut(&app, &game, &user)).await
}

/// Removes a game's Steam entry and its artwork there, if it has one.
#[tauri::command]
pub async fn remove_from_steam(
    state: State<'_, ConfigState>,
    id: String,
    shutdown_steam: bool,
) -> Result<SteamChange, AppError> {
    let game = find_game(&state, &id)?;
    let user = SteamUser::find()?;
    change_steam(shutdown_steam, || remove_shortcut(game.id, &user)).await
}

/// The ids of the games that have an entry in Steam. Empty without Steam,
/// or if its shortcuts can't be read.
#[tauri::command]
pub fn list_steam_games(state: State<ConfigState>) -> Result<Vec<String>, AppError> {
    let Ok(user) = SteamUser::find() else {
        return Ok(Vec::new());
    };
    let Ok(mut file) = read_shortcuts_file(&user.shortcuts_path) else {
        return Ok(Vec::new());
    };
    let shortcuts = binary_vdf::map_entry(&mut file, "shortcuts");
    let config = state
        .lock()
        .map_err(|_| "Configuration is locked".to_string())?;
    Ok(config
        .games
        .iter()
        .filter(|g| find_shortcut(shortcuts, g.id).is_some())
        .map(|g| g.id.to_string())
        .collect())
}

#[cfg(test)]
mod tests;
