use crate::error::AppError;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, State};

use crate::commands::games::LaunchingGames;
use crate::config::{save_config, AppConfig, ConfigState};
use crate::lock::LockExt;
use crate::models::PrefixInfo;

/// The folder to use as `WINEPREFIX` for a folder the user picked. A Proton
/// "compat data" folder (as Steam, Lutris or Heroic leave behind) keeps the
/// actual prefix in `pfx/`, next to Proton's own bookkeeping. Used as it is,
/// every check for `drive_c` would miss the existing prefix, and a Wine
/// runner would start a new, empty one next to it.
///
/// umu lays a prefix out the other way round, with `pfx` a link back to the
/// folder itself; that one is used as it is.
pub fn effective_prefix_path(path: &Path) -> PathBuf {
    let pfx = path.join("pfx");
    let links_back = fs::canonicalize(&pfx).ok() == fs::canonicalize(path).ok();
    if !path.join("drive_c").is_dir() && pfx.is_dir() && !links_back {
        pfx
    } else {
        path.to_path_buf()
    }
}

/// `path` as a prefix Prefixr knows: one in its list, or one a game uses
/// (a prefix removed from the list is still that game's own). The commands
/// that run something in a prefix (winetricks, Wine's tools, an installer)
/// only take those, rather than create and write to any directory they're
/// handed.
pub(crate) fn known_prefix(config: &AppConfig, path: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(path);
    let known = config.prefixes.iter().any(|p| p.path == path)
        || config.games.iter().any(|g| g.prefix_path == path);
    if known {
        Ok(path)
    } else {
        Err(format!("No prefix known at {}", path.display()))
    }
}

/// Registers a prefix path with the app — either a brand-new, empty folder
/// (created here, initialized lazily via `wineboot` on the first game launch
/// that uses it) or an already-existing Wine prefix from another tool. No
/// runner is involved here: which runner initializes/launches a prefix is
/// decided per-game, via `Game::runner_id`. Returns the path registered,
/// which for a Proton compat data folder is its `pfx/` (see
/// `effective_prefix_path`).
#[tauri::command(async)]
pub fn add_prefix(
    app: AppHandle,
    state: State<ConfigState>,
    path: String,
) -> Result<PathBuf, AppError> {
    let prefix_path = effective_prefix_path(Path::new(&path));

    let mut config = state.locked();

    if config.prefixes.iter().any(|p| p.path == prefix_path) {
        return Err(AppError::PrefixAlreadyExists {
            path: prefix_path.display().to_string(),
        });
    }

    if prefix_path.exists() {
        if !prefix_path.is_dir() {
            return Err(AppError::PathNotADirectory { path: path.clone() });
        }
        let has_contents = fs::read_dir(&prefix_path)
            .map_err(|e| format!("Could not read {path}: {e}"))?
            .next()
            .is_some();
        // A Proton "compat data" folder was already swapped for its `pfx/`.
        let looks_like_prefix = prefix_path.join("drive_c").is_dir();
        if has_contents && !looks_like_prefix {
            return Err(AppError::PrefixDirNotEmpty { path: path.clone() });
        }
    } else {
        fs::create_dir_all(&prefix_path)
            .map_err(|e| format!("Could not create prefix directory: {e}"))?;
    }

    config.prefixes.push(PrefixInfo {
        path: prefix_path.clone(),
    });
    save_config(&app, &config)?;
    Ok(prefix_path)
}

/// Removes a prefix from the app only; its folder on disk is left exactly as
/// it is. Refused while a game in it is running or being launched, so the UI
/// can't quietly hide a prefix a still-running game depends on.
#[tauri::command]
pub async fn delete_prefix(
    app: AppHandle,
    state: State<'_, ConfigState>,
    launching: State<'_, LaunchingGames>,
    path: String,
) -> Result<(), AppError> {
    let prefix_path = PathBuf::from(&path);

    // Every game in this prefix is held as launching until the folder is
    // gone, so none can be started while it's being deleted. A game that
    // already runs is launching until it exits, so the claim is refused.
    let mut guards = Vec::new();
    {
        let config = state.locked();
        if !config.prefixes.iter().any(|p| p.path == prefix_path) {
            return Err(format!("No prefix known at {path}").into());
        }
        for game in config.games.iter().filter(|g| g.prefix_path == prefix_path) {
            let Some(guard) = launching.claim(game.id) else {
                return Err(AppError::PrefixInUse {
                    game_name: game.name.clone(),
                });
            };
            guards.push(guard);
        }
    }

    let mut config = state.locked();
    config.prefixes.retain(|p| p.path != prefix_path);
    save_config(&app, &config).map_err(AppError::from)
}

/// The prefixes in the list, straight from the config: no disk access, so
/// every form that offers a prefix gets them at once. The prefixes page
/// asks `prefix_states` for how their folders are on top.
#[tauri::command]
pub fn list_prefixes(state: State<ConfigState>) -> Result<Vec<PrefixInfo>, AppError> {
    Ok(state.locked().prefixes.clone())
}

/// How a prefix's folder is: set up (Wine has made its `drive_c`), not yet
/// (that happens on the first launch of a game in it), not there (nothing,
/// or a file, at its path), there but not readable (permissions, I/O
/// errors), or no answer in time (a network share whose server is gone, a
/// drive that's still spinning up).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum PrefixState {
    Ready,
    NotSetUp,
    Missing,
    Unreadable,
    TimedOut,
}

/// A listed prefix and how its folder is.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct PrefixStatus {
    pub path: PathBuf,
    pub state: PrefixState,
}

pub fn prefix_state(path: &Path) -> PrefixState {
    use std::io::ErrorKind::{NotADirectory, NotFound};
    match fs::metadata(path) {
        Err(e) if matches!(e.kind(), NotFound | NotADirectory) => return PrefixState::Missing,
        Err(_) => return PrefixState::Unreadable,
        Ok(meta) if !meta.is_dir() => return PrefixState::Missing,
        Ok(_) => {}
    }
    match fs::metadata(path.join("drive_c")) {
        Ok(meta) if meta.is_dir() => PrefixState::Ready,
        Ok(_) => PrefixState::NotSetUp,
        Err(e) if matches!(e.kind(), NotFound | NotADirectory) => PrefixState::NotSetUp,
        Err(_) => PrefixState::Unreadable,
    }
}

/// How long one prefix's folder may take to answer before it's `TimedOut`.
const STATE_TIMEOUT: Duration = Duration::from_secs(3);

/// Each folder is looked at on a blocking thread of its own, all at once,
/// and waited for until a shared deadline. A `stat` stuck on a dead mount
/// keeps only its own thread, never a runtime worker, and the others still
/// answer.
pub(crate) async fn states_of(
    paths: Vec<PathBuf>,
    limit: Duration,
    check: fn(&Path) -> PrefixState,
) -> Vec<PrefixStatus> {
    let deadline = tokio::time::Instant::now() + limit;
    let checks: Vec<_> = paths
        .iter()
        .cloned()
        .map(|path| tauri::async_runtime::spawn_blocking(move || check(&path)))
        .collect();
    let mut states = Vec::with_capacity(paths.len());
    for (path, task) in paths.into_iter().zip(checks) {
        let state = match tokio::time::timeout_at(deadline, task).await {
            Ok(Ok(state)) => state,
            Ok(Err(_)) => PrefixState::Unreadable,
            Err(_) => PrefixState::TimedOut,
        };
        states.push(PrefixStatus { path, state });
    }
    states
}

/// How the listed prefixes' folders are. Only the prefixes page asks, as it
/// is the one that shows it.
#[tauri::command]
pub async fn prefix_states(state: State<'_, ConfigState>) -> Result<Vec<PrefixStatus>, AppError> {
    let paths = state
        .locked()
        .prefixes
        .iter()
        .map(|p| p.path.clone())
        .collect();
    Ok(states_of(paths, STATE_TIMEOUT, prefix_state).await)
}

#[cfg(test)]
mod tests;
