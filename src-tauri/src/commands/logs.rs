use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Manager};

use crate::error::AppError;
use crate::host_env::xdg_open;

/// How many logs each game (or prefix) keeps. Every launch writes a new
/// one, so without a limit they'd pile up forever.
const LOGS_KEPT: usize = 10;

fn logs_root(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Could not resolve data directory: {e}"))?
        .join("logs"))
}

/// A game's logs: `logs/<game_id>/`.
pub fn game_log_dir(app: &AppHandle, game_id: &str) -> Result<PathBuf, String> {
    Ok(logs_root(app)?.join(game_id))
}

/// Logs of what runs in a prefix rather than as a game (winetricks, an
/// installer, Wine's own tools): `logs/prefixes/<sanitized-prefix-path>/`.
/// Keyed by prefix, not by game: a prefix can outlive or be shared across
/// several games, so there's no single game id that owns these.
/// The logs of a store tool's runs for one game (its installs), e.g.
/// `logs/stores/epic/Fortnite/`.
pub fn store_log_dir(app: &AppHandle, store: &str, app_name: &str) -> Result<PathBuf, String> {
    let dir_name = app_name.replace(['/', '\\', '.'], "_");
    Ok(logs_root(app)?.join("stores").join(store).join(dir_name))
}

pub fn prefix_log_dir(app: &AppHandle, prefix_path: &Path) -> Result<PathBuf, String> {
    let dir_name = prefix_path.to_string_lossy().replace(['/', '\\'], "_");
    Ok(logs_root(app)?.join("prefixes").join(dir_name))
}

/// Creates a new, empty log file in `dir` and removes the oldest ones
/// beyond `LOGS_KEPT`. Named after the current time in milliseconds, so two
/// runs within the same second (a quick retry after a failed start) don't
/// share, and truncate, one file. `.txt` rather than `.log`: most Linux
/// desktops have no default app registered for `.log`, so "Log anzeigen"
/// would silently fail to open it.
pub fn new_log_file(dir: &Path) -> Result<PathBuf, String> {
    fs::create_dir_all(dir).map_err(|e| format!("Could not create log directory: {e}"))?;
    prune_logs(dir, LOGS_KEPT - 1);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("Could not read system time: {e}"))?
        .as_millis();
    let path = dir.join(format!("{timestamp}.txt"));
    fs::File::create(&path).map_err(|e| format!("Could not create log file: {e}"))?;
    Ok(path)
}

/// Removes all but the `keep` newest logs in `dir`. Files are ordered by
/// the timestamp they're named after; older versions named them in seconds
/// rather than milliseconds, which still sorts them as older. Best-effort:
/// a log that can't be removed just stays.
fn prune_logs(dir: &Path, keep: usize) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut logs: Vec<(u128, PathBuf)> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter_map(|path| {
            let timestamp = path
                .file_name()?
                .to_str()?
                .strip_suffix(".txt")?
                .parse()
                .ok()?;
            Some((timestamp, path))
        })
        .collect();
    if logs.len() <= keep {
        return;
    }
    logs.sort();
    for (_, path) in &logs[..logs.len() - keep] {
        let _ = fs::remove_file(path);
    }
}

/// `path` as a log under `root`, with symlinks and `..` resolved — the only
/// files `open_log` opens.
fn log_file_in(root: &Path, path: &Path) -> Result<PathBuf, String> {
    let not_a_log = || format!("Not a Prefixr log: {}", path.display());
    let root = fs::canonicalize(root).map_err(|_| not_a_log())?;
    let path = fs::canonicalize(path).map_err(|_| not_a_log())?;
    if path.starts_with(&root) && path.is_file() {
        Ok(path)
    } else {
        Err(not_a_log())
    }
}

/// Opens a log in the desktop's default app for it (see `xdg_open`). An
/// error lets the frontend fall back to showing the file in the file
/// manager.
#[tauri::command]
pub async fn open_log(app: AppHandle, path: String) -> Result<(), AppError> {
    let path = log_file_in(&logs_root(&app)?, Path::new(&path))?;
    xdg_open(&path).await.map_err(AppError::from)
}

#[cfg(test)]
mod tests;
