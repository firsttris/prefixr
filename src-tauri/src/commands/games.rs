use crate::error::AppError;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Listener, Manager, State};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
use uuid::Uuid;

use crate::commands::github::read_token;
use crate::commands::graphics_layers::{ensure_directx_layer_cache, ensure_wine_mono_msi};
use crate::commands::icons::{exe_icon_path, extract_icon_png, png_data_url, store_exe_icon};
use crate::commands::logs::{game_log_dir, new_log_file, prefix_log_dir};
use crate::locale::{Locale, LocaleState};
use crate::commands::graphics::{ensure_vkbasalt_conf, vkbasalt_conf_path};
use crate::commands::mangohud::{ensure_mangohud_conf, mangohud_conf_path};
use crate::commands::runners::{
    find_runner, prefix_command, runner_command, wine_binary, wineserver_binary,
};
use crate::commands::shell_link::{find_recently_created_shortcuts, DetectedShortcut};
use crate::commands::steamgriddb::{
    artwork_dir, asset_cache_path, image_extension, remove_game_artwork_files,
};
use crate::commands::umu::{ensure_umu, runtime_present};
use crate::config::{save_config, ConfigState};
use crate::models::{
    normalize_umu_id, normalize_umu_store, split_launch_args, Game, GameInput, Runner, RunnerKind,
};
use crate::tray::rebuild_tray_menu;

/// Checks whether `name` resolves to an executable file somewhere on `PATH`,
/// used to gate optional wrappers (e.g. `systemd-inhibit`) that may not be
/// installed on every system. `pub(crate)` since `commands::performance` also
/// needs it (to gate the `pkexec`-based max_map_count fix).
pub(crate) fn command_on_path(name: &str) -> bool {
    std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).any(|dir| dir.join(name).is_file()))
        .unwrap_or(false)
}

/// systemd units for CPU/process schedulers that manage niceness dynamically
/// (auto-nice daemons, sched-ext loaders). GameMode's `gamemoderun` also
/// writes niceness/governor directly, so running both fights over the same
/// knobs — seen in the wild as priority flapping. Checked in `launch_game`.
const COMPETING_SCHEDULER_UNITS: &[&str] = &[
    "ananicy.service",
    "ananicy-cpp.service",
    "scx.service",
    "scx_loader.service",
    "falcond.service",
];

/// Whether one of `COMPETING_SCHEDULER_UNITS` is currently active, in which
/// case GameMode's own niceness/governor tweaks should be skipped in favor of
/// `power_profile` (see `launch_game`).
async fn competing_scheduler_active() -> bool {
    for unit in COMPETING_SCHEDULER_UNITS {
        let active = tokio::process::Command::new("systemctl")
            .args(["is-active", "--quiet", unit])
            .status()
            .await
            .map(|status| status.success())
            .unwrap_or(false);
        if active {
            return true;
        }
    }
    false
}

/// A game process currently running under its runner, tracked so the tray
/// menu can list it and offer to kill it (e.g. when a Proton game hangs).
#[derive(Clone)]
pub struct RunningGame {
    pub name: String,
    /// PID of the process `launch_game` spawned — the outermost wrapper
    /// (gamescope, systemd-inhibit, ...) if any are in use, otherwise
    /// umu-run or wine itself. The root of the process tree
    /// `kill_running_game` takes down. `None` only if the OS didn't report
    /// one.
    pub pid: Option<u32>,
    /// Path to the `wineserver` binary belonging to the runner this game was
    /// launched with, and the `WINEPREFIX` it's managing that game's session
    /// under — the fallback half of `kill_running_game`. When a runner is
    /// invoked through `distrobox-host-exec` (see `runner_command`), the
    /// process actually runs on the host, in a different PID namespace than
    /// this app, so `pid` above only ever reaches the local relay, not the
    /// real process. Running `wineserver -k` through the same
    /// `runner_command` reaches the real session either way, because
    /// wineserver finds it via the prefix's socket file rather than by PID.
    pub wineserver: PathBuf,
    pub wineprefix: String,
    /// The game's own exe path, as passed to wine/umu-run. Present verbatim
    /// in the command line of the process we spawned and the ones below it,
    /// so it doubles as a `pkill -f` pattern — see `kill_running_game`.
    pub exe_path: PathBuf,
    /// Set by `kill_running_game`, so `run_game` doesn't report the exit
    /// status of a game the user ended as a launch error.
    pub killed: Arc<AtomicBool>,
}

#[derive(Default)]
pub struct RunningGames(pub Mutex<HashMap<Uuid, RunningGame>>);

/// Games with a `launch_game` in progress, from the click until the game
/// exits. Unlike `RunningGames`, this also covers the preparation before
/// the process is spawned (umu/DXVK/wine-mono downloads, prefix setup),
/// which can take a while without any window showing up.
///
/// Each claim also holds a lock file per game (see `lock_launch_file`),
/// which is what makes it visible to other Prefixr processes: a game Steam
/// started via `--run` runs in a windowless Prefixr of its own, and the
/// open one must neither start it a second time nor delete its prefix.
#[derive(Default)]
pub struct LaunchingGames(Mutex<HashSet<Uuid>>);

impl LaunchingGames {
    /// Marks `id` as launching, or `None` if it already is — in this
    /// process or another one.
    pub(crate) fn claim(&self, id: Uuid) -> Option<LaunchGuard<'_>> {
        // The lock is released before a guard exists: dropping one locks
        // again (and must never happen for a refused claim, whose drop
        // would end the launch already in progress).
        let inserted = self.0.lock().ok()?.insert(id);
        if !inserted {
            return None;
        }
        let mut guard = LaunchGuard {
            launching: self,
            id,
            _lock_file: None,
        };
        // Refused by another process: dropping the guard takes `id` back
        // out of this process's set, where it was only just inserted.
        guard._lock_file = lock_launch_file(id).ok()?;
        Some(guard)
    }

    /// The games launching or running in this process.
    pub(crate) fn ids(&self) -> Vec<Uuid> {
        self.0
            .lock()
            .map(|launching| launching.iter().copied().collect())
            .unwrap_or_default()
    }
}

/// Keeps a game in `LaunchingGames` until dropped.
pub(crate) struct LaunchGuard<'a> {
    launching: &'a LaunchingGames,
    id: Uuid,
    /// Held locked for as long as the guard lives; closing it unlocks it,
    /// and so does the process ending in any way.
    _lock_file: Option<fs::File>,
}

/// Where the per-game lock files live: the user's runtime directory, which
/// every Prefixr process of the same user shares, including one started by
/// Steam.
fn launch_lock_dir() -> PathBuf {
    match std::env::var_os("XDG_RUNTIME_DIR") {
        Some(dir) => PathBuf::from(dir).join("prefixr"),
        // SAFETY: getuid(2) can't fail and has no preconditions.
        None => std::env::temp_dir().join(format!("prefixr-{}", unsafe { libc::getuid() })),
    }
}

/// Locks the game's lock file, `Err` if another process holds it. A file
/// that can't be created or locked for any other reason gives `Ok(None)`:
/// the launch then goes ahead, guarded only within this process as before.
fn lock_launch_file(id: Uuid) -> Result<Option<fs::File>, ()> {
    let dir = launch_lock_dir();
    if fs::create_dir_all(&dir).is_err() {
        return Ok(None);
    }
    let Ok(file) = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(dir.join(format!("{id}.lock")))
    else {
        return Ok(None);
    };
    match file.try_lock() {
        Ok(()) => Ok(Some(file)),
        Err(fs::TryLockError::WouldBlock) => Err(()),
        Err(fs::TryLockError::Error(_)) => Ok(None),
    }
}

impl Drop for LaunchGuard<'_> {
    fn drop(&mut self) {
        if let Ok(mut launching) = self.launching.0.lock() {
            launching.remove(&self.id);
        }
    }
}

/// PIDs of every process below `root`, from each process's `PPid:` in
/// `/proc/<pid>/status` — the same way umu walks its own process tree.
fn process_descendants(root: u32) -> Vec<u32> {
    let Ok(entries) = fs::read_dir("/proc") else {
        return Vec::new();
    };
    let mut children: HashMap<u32, Vec<u32>> = HashMap::new();
    for entry in entries.flatten() {
        let Some(pid) = entry.file_name().to_str().and_then(|n| n.parse::<u32>().ok()) else {
            continue;
        };
        let Ok(status) = fs::read_to_string(entry.path().join("status")) else {
            continue;
        };
        let Some(ppid) = status
            .lines()
            .find_map(|line| line.strip_prefix("PPid:"))
            .and_then(|value| value.trim().parse::<u32>().ok())
        else {
            continue;
        };
        children.entry(ppid).or_default().push(pid);
    }

    let mut descendants = Vec::new();
    let mut queue = vec![root];
    while let Some(pid) = queue.pop() {
        for &child in children.get(&pid).into_iter().flatten() {
            if !descendants.contains(&child) {
                descendants.push(child);
                queue.push(child);
            }
        }
    }
    descendants
}

/// Whether `pid` is still actually running — a zombie (exited, just not
/// reaped yet) counts as gone.
fn process_running(pid: u32) -> bool {
    fs::read_to_string(format!("/proc/{pid}/stat"))
        .ok()
        .and_then(|stat| {
            stat.rsplit_once(')')
                .map(|(_, rest)| !rest.trim_start().starts_with('Z'))
        })
        .unwrap_or(false)
}

fn send_signal(pid: u32, signal: libc::c_int) {
    // SAFETY: kill(2) has no memory-safety preconditions; a pid that's
    // already gone just makes it fail with ESRCH, which is fine here.
    unsafe {
        libc::kill(pid as libc::pid_t, signal);
    }
}

/// Grace period between SIGTERM and SIGKILL in `kill_process_tree`.
const KILL_GRACE_PERIOD: Duration = Duration::from_secs(3);

/// SIGTERMs `root`, gives it `KILL_GRACE_PERIOD` to shut down, then
/// SIGKILLs whatever is left of it and its whole process tree.
///
/// For a Proton game this is effectively "kill the container": umu-run
/// registers itself as a child subreaper, so every process of the game's
/// session — including ones that daemonize, like wineserver — stays
/// somewhere below it rather than escaping to init, and it forwards a
/// SIGTERM to that entire tree itself.
async fn kill_process_tree(root: u32) {
    // Collected before signalling anything: once `root` is gone, its
    // orphaned descendants get reparented away and can't be found through it
    // anymore.
    let mut targets = process_descendants(root);
    send_signal(root, libc::SIGTERM);

    let deadline = Instant::now() + KILL_GRACE_PERIOD;
    while process_running(root) && Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    if process_running(root) {
        targets.extend(process_descendants(root));
        targets.push(root);
    }
    for pid in targets {
        if process_running(pid) {
            send_signal(pid, libc::SIGKILL);
        }
    }
}

/// Turns `text` into a `pkill -f` pattern (a POSIX extended regex) that
/// matches it literally. Metacharacters are escaped — game paths routinely
/// contain them, e.g. the parentheses in `Program Files (x86)`, which would
/// otherwise make the pattern silently never match. The first character is
/// wrapped in a bracket expression so the pattern doesn't match its own
/// literal text: through `distrobox-host-exec` (see `runner_command`), the
/// relay process carrying the pattern as an argument is visible to the
/// host-side pkill, which would otherwise kill it too.
fn pkill_pattern(text: &str) -> String {
    let mut pattern = String::with_capacity(text.len() + 2);
    for (i, c) in text.chars().enumerate() {
        if i == 0 {
            pattern.push('[');
            if c == '\\' || c == '^' || c == ']' {
                // Can't be escaped inside a bracket expression; the escaped
                // form below matches the same character without one.
                pattern.pop();
            } else {
                pattern.push(c);
                pattern.push(']');
                continue;
            }
        }
        if "\\^$.|?*+()[]{}".contains(c) {
            pattern.push('\\');
        }
        pattern.push(c);
    }
    pattern
}

/// Kills a running game's whole session, used by both the `kill_game`
/// command and the tray menu's per-game "beenden" entries. The tracked entry
/// itself is removed once `launch_game`'s `child.wait()` observes the
/// process actually exiting, not here.
///
/// 1. `kill_process_tree` on the process `launch_game` spawned — for a
///    Proton game, this takes down the whole umu container session.
/// 2. SIGTERM to umu-run found by its command line, via `pkill` through
///    `runner_command`: the same container shutdown for a game started
///    through `distrobox-host-exec`, whose real process tree runs on the
///    host and isn't below the relay process we spawned. A no-op for a Wine
///    game, or once step 1 already got it.
/// 3. As a fallback, the same two mechanisms PortProton's `kill_portwine`
///    combines: `wineserver -k9` (asks the whole wine session to terminate
///    via wineserver's own IPC) and `pkill -9 -f` on the game's exe path.
///    These cover a Wine game's wineserver, which detaches from the process
///    tree, and a game started through `distrobox-host-exec`, where the tree
///    visible to us only contains the relay (see `RunningGame::wineserver`).
pub async fn kill_running_game(running: &RunningGames, id: Uuid) -> Result<(), String> {
    let game = {
        let games = running
            .0
            .lock()
            .map_err(|_| "Running games list is locked".to_string())?;
        games
            .get(&id)
            .cloned()
            .ok_or_else(|| "Game is not running".to_string())?
    };
    game.killed.store(true, Ordering::SeqCst);

    if let Some(pid) = game.pid {
        kill_process_tree(pid).await;
    }

    let exe_path = game.exe_path.to_string_lossy();
    // umu-run's command line is `<python> .../umu/umu-run <exe> <args...>`.
    let umu_signalled = runner_command(Path::new("pkill"), [])
        .args(["-TERM", "-f"])
        .arg(pkill_pattern(&format!("umu-run {exe_path}")))
        .status()
        .await
        .is_ok_and(|status| status.success());
    if umu_signalled {
        tokio::time::sleep(KILL_GRACE_PERIOD).await;
    }

    let wineserver_result =
        runner_command(&game.wineserver, [("WINEPREFIX", game.wineprefix.as_str())])
            .arg("-k9")
            .status()
            .await;

    let pkill_result = runner_command(Path::new("pkill"), [])
        .args(["-9", "-f"])
        .arg(pkill_pattern(&exe_path))
        .status()
        .await;

    // Either one actually running is a success; only report an error if
    // both failed to even run (a real environment problem, e.g. neither
    // binary exists) rather than surfacing e.g. wineserver's "no session
    // found" exit code as a failure when the process tree kill already got
    // it.
    match (wineserver_result, pkill_result) {
        (Err(e), Err(_)) => Err(format!("Could not run wineserver: {e}")),
        _ => Ok(()),
    }
}

/// A game this Prefixr is launching or running, for the frontend to pick
/// its state back up after the webview reloaded (the launch events it
/// tracks that state from are gone by then).
#[derive(Serialize)]
pub struct ActiveGame {
    id: String,
    /// The process runs; otherwise it's still being prepared.
    running: bool,
}

#[tauri::command]
pub fn list_active_games(
    running: State<RunningGames>,
    launching: State<LaunchingGames>,
) -> Vec<ActiveGame> {
    let running = running
        .0
        .lock()
        .map(|games| games.keys().copied().collect::<HashSet<_>>())
        .unwrap_or_default();
    launching
        .ids()
        .into_iter()
        .map(|id| ActiveGame {
            id: id.to_string(),
            running: running.contains(&id),
        })
        .collect()
}

#[tauri::command]
pub async fn kill_game(running: State<'_, RunningGames>, id: String) -> Result<(), AppError> {
    let game_id = Uuid::parse_str(&id).map_err(|e| format!("Invalid game id: {e}"))?;
    kill_running_game(&running, game_id).await.map_err(AppError::from)
}

/// Holds a `--launch <game-id>` argument found at startup (see `run()` in
/// `lib.rs`), so the frontend can pick it up once and start that game
/// immediately — this is what a desktop shortcut created by
/// `create_desktop_shortcut` invokes the app with.
pub struct PendingLaunch(pub Mutex<Option<String>>);

#[tauri::command]
pub fn take_pending_launch(state: State<PendingLaunch>) -> Option<String> {
    state.0.lock().ok()?.take()
}

/// Holds a `--install <exe-path>` argument found at startup, or forwarded by
/// a second app instance (see `tauri_plugin_single_instance` in `lib.rs`) —
/// the "Install with Prefixr" file-manager context menu entry launches
/// Prefixr this way. The frontend picks it up once (either via
/// `take_pending_install` on startup, or via the `pending-install` event a
/// second instance triggers while the app is already running) and opens the
/// installer dialog with this exe path pre-filled.
pub struct PendingInstall(pub Mutex<Option<String>>);

#[tauri::command]
pub fn take_pending_install(state: State<PendingInstall>) -> Option<String> {
    state.0.lock().ok()?.take()
}

/// What `run_installer` reports back once the installer exited.
#[derive(Serialize)]
pub struct InstallerResult {
    shortcuts: Vec<DetectedShortcut>,
    log_path: String,
}

/// Runs an arbitrary exe (typically a game's setup/installer, as opposed to
/// the game's own exe once installed) under a chosen prefix and runner, then
/// (once the installer exits) reports any `.exe` shortcuts it created on the
/// Desktop / in the Start Menu — see `find_recently_created_shortcuts` — so
/// the frontend can offer them as the game's exe instead of making the user
/// hunt for it manually. This does not create a `Game` entry itself: that's
/// still a separate, deliberate step the frontend takes with whichever
/// candidate (or manually chosen exe) the user confirms.
///
/// Unlike `launch_wine_tool`, this is awaited rather than left detached: the
/// whole point is to know when the installer has finished so the shortcut
/// scan sees its result, and a GUI installer's own window is what the user
/// actually interacts with in the meantime, not this command.
///
/// The prefix is readied first exactly as for a game (see `prepare_prefix`),
/// since an installer is often the first thing to run in a fresh one. Its
/// output goes to a log of the prefix (see `logs::prefix_log_dir`), whose
/// path comes back along with the shortcuts, or in the error.
#[tauri::command]
pub async fn run_installer(
    app: AppHandle,
    state: State<'_, ConfigState>,
    prefix_path: String,
    runner_id: String,
    exe_path: String,
) -> Result<InstallerResult, AppError> {
    let runners_dir = {
        let config = state
            .lock()
            .map_err(|_| "Configuration is locked".to_string())?;
        config.runners_dir.clone()
    };
    let token = read_token(&state)?;

    let runner = find_runner(&runners_dir, &runner_id)?;
    let prefix = PathBuf::from(&prefix_path);
    let log_path = new_log_file(&prefix_log_dir(&app, &prefix)?)?;
    let log_path_string = log_path.display().to_string();

    let prepared = prepare_prefix(&app, token.as_deref(), &runner, &prefix, &log_path, &|| {})
        .await
        .map_err(|e| AppError::WithLogDetails {
            message: e,
            log_path: log_path_string.clone(),
        })?;
    let exe = PathBuf::from(&exe_path);
    let mut command = runner_command(&prepared.binary, env_pairs(&prepared.env));
    command.arg(&exe);
    // Installers commonly expect to run from their own directory (sibling
    // data files, relative paths) — mirrors how a user would run it by hand.
    if let Some(parent) = exe.parent() {
        command.current_dir(parent);
    }

    // Captured before the installer runs, so the shortcut scan below only
    // picks up files it actually created (or touched) just now, not
    // pre-existing shortcuts from an earlier install into the same prefix.
    let started_at = SystemTime::now();
    // Exit status is deliberately not checked: some installers return a
    // non-zero code on perfectly successful installs (e.g. "reboot
    // recommended"), so a shortcut having appeared is a more reliable
    // success signal than the process's own exit code.
    let (out, err) = log_stdio(&log_path)?;
    command
        .stdin(Stdio::null())
        .stdout(out)
        .stderr(err)
        .status()
        .await
        .map_err(|e| AppError::SetupLaunchFailed {
            error: e.to_string(),
            log_path: log_path_string.clone(),
        })?;

    Ok(InstallerResult {
        shortcuts: find_recently_created_shortcuts(&prefix, started_at),
        log_path: log_path.display().to_string(),
    })
}

/// An environment as `runner_command` takes it.
pub(crate) fn env_pairs(env: &[(String, String)]) -> Vec<(&str, &str)> {
    env.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect()
}

#[tauri::command]
pub fn list_games(state: State<ConfigState>) -> Result<Vec<Game>, AppError> {
    let config = state
        .lock()
        .map_err(|_| "Configuration is locked".to_string())?;
    Ok(config.games.clone())
}

/// A store only means something alongside an id.
fn umu_fields(id: Option<String>, store: Option<String>) -> (Option<String>, Option<String>) {
    let id = normalize_umu_id(id);
    let store = id.as_ref().and(normalize_umu_store(store));
    (id, store)
}

#[tauri::command(async)]
pub fn add_game(
    app: AppHandle,
    state: State<ConfigState>,
    game: GameInput,
) -> Result<Game, AppError> {
    // Checked here too, so a typo shows up when saving rather than as a
    // failed launch.
    split_launch_args(&game.launch_args)?;
    let icon = extract_icon_png(&game.exe_path);
    let (umu_id, umu_store) = umu_fields(game.umu_id, game.umu_store);
    let id = Uuid::new_v4();
    // Best-effort, like the icon itself: it's only cosmetic.
    let _ = store_exe_icon(&app, id, icon.as_deref());
    let new_game = Game {
        id,
        name: game.name,
        exe_path: game.exe_path,
        prefix_path: game.prefix_path,
        runner_id: game.runner_id,
        env_vars: game.env_vars,
        launch_args: game.launch_args,
        icon: icon.as_deref().map(png_data_url),
        steamgriddb_id: None,
        cover_grid_id: None,
        cover_url: None,
        steamgriddb_icon_grid_id: None,
        steamgriddb_icon_url: None,
        artwork: Default::default(),
        umu_id,
        umu_store,
        overrides: game.overrides,
    };

    let mut config = state
        .lock()
        .map_err(|_| "Configuration is locked".to_string())?;
    config.games.push(new_game.clone());
    save_config(&app, &config)?;
    Ok(new_game)
}

#[tauri::command(async)]
pub fn update_game(
    app: AppHandle,
    state: State<ConfigState>,
    id: String,
    game: GameInput,
) -> Result<Game, AppError> {
    let game_id = Uuid::parse_str(&id).map_err(|e| format!("Invalid game id: {e}"))?;
    split_launch_args(&game.launch_args)?;
    let find = |games: &[Game]| -> Result<usize, String> {
        games
            .iter()
            .position(|g| g.id == game_id)
            .ok_or_else(|| format!("No game with id {id}"))
    };

    // Parsing the exe for its icon means reading all of it, so that only
    // happens when there's something new to find — and outside the lock.
    let needs_icon = {
        let config = state
            .lock()
            .map_err(|_| "Configuration is locked".to_string())?;
        let existing = &config.games[find(&config.games)?];
        existing.exe_path != game.exe_path || existing.icon.is_none()
    };
    let icon = needs_icon.then(|| extract_icon_png(&game.exe_path));
    if let Some(icon) = &icon {
        let _ = store_exe_icon(&app, game_id, icon.as_deref());
    }

    let mut config = state
        .lock()
        .map_err(|_| "Configuration is locked".to_string())?;
    let index = find(&config.games)?;
    let existing = &mut config.games[index];

    if let Some(icon) = icon {
        existing.icon = icon.as_deref().map(png_data_url);
    }
    existing.name = game.name;
    existing.exe_path = game.exe_path;
    existing.prefix_path = game.prefix_path;
    existing.runner_id = game.runner_id;
    existing.env_vars = game.env_vars;
    existing.launch_args = game.launch_args;
    (existing.umu_id, existing.umu_store) = umu_fields(game.umu_id, game.umu_store);
    existing.overrides = game.overrides;
    let updated = existing.clone();

    save_config(&app, &config)?;
    Ok(updated)
}

/// Removes a game from the library along with everything Prefixr made for
/// it: its configs, icon and artwork, logs, and its desktop and menu
/// shortcuts, which would otherwise launch a game that no longer exists.
/// Its prefix stays, which other games may share. Best-effort past the
/// config itself: a leftover file is no reason to keep the game.
#[tauri::command(async)]
pub fn remove_game(app: AppHandle, state: State<ConfigState>, id: String) -> Result<(), AppError> {
    let game_id = Uuid::parse_str(&id).map_err(|e| format!("Invalid game id: {e}"))?;
    let mut config = state
        .lock()
        .map_err(|_| "Configuration is locked".to_string())?;

    if !config.games.iter().any(|g| g.id == game_id) {
        return Err(format!("No game with id {id}").into());
    }

    config.games.retain(|g| g.id != game_id);
    save_config(&app, &config)?;
    drop(config);

    let legacy_shortcut_icon = app
        .path()
        .app_data_dir()
        .map(|dir| dir.join("shortcut-icons").join(format!("{id}.png")))
        .map_err(|e| e.to_string());
    for path in [
        vkbasalt_conf_path(&app, &id),
        mangohud_conf_path(&app, &id),
        exe_icon_path(&app, game_id),
        legacy_shortcut_icon,
    ]
    .into_iter()
    .flatten()
    {
        let _ = fs::remove_file(path);
    }
    remove_game_artwork_files(&app, game_id);
    if let Ok(logs) = game_log_dir(&app, &id) {
        let _ = fs::remove_dir_all(logs);
    }

    let desktop_shortcuts = desktop_directory()
        .map(|dir| game_shortcuts(&dir, game_id))
        .unwrap_or_default();
    for shortcut in desktop_shortcuts {
        let _ = fs::remove_file(shortcut);
    }
    if let Ok(applications_dir) = applications_directory() {
        let menu_entries = game_shortcuts(&applications_dir, game_id);
        for entry in &menu_entries {
            let _ = fs::remove_file(entry);
        }
        if !menu_entries.is_empty() {
            let _ = std::process::Command::new("update-desktop-database")
                .arg(&applications_dir)
                .status();
        }
    }
    Ok(())
}

#[derive(Clone, Serialize)]
struct GameInitializingPayload<'a> {
    id: &'a str,
}

#[derive(Clone, Serialize)]
struct GameStartedPayload<'a> {
    id: &'a str,
    log_path: String,
}

#[derive(Clone, Serialize)]
struct GameExitedPayload<'a> {
    id: &'a str,
    exit_code: Option<i32>,
}

#[derive(Clone, Serialize)]
struct GameLaunchErrorPayload<'a> {
    id: &'a str,
    message: AppError,
    log_path: Option<String>,
}

/// Symlinks `dst_path` to `src_path`'s fully-resolved target, replacing
/// whatever (if anything) is already there — unless it's already correctly
/// linked, in which case it's left untouched. A missing `src_path` is not an
/// error: it just means this particular file isn't relevant (Direct3D
/// modules a given layer doesn't provide, e.g. DXVK has no d3d12).
fn relink_if_needed(src_path: &Path, dst_path: &Path) -> Result<(), String> {
    let Ok(resolved_src) = fs::canonicalize(src_path) else {
        return Ok(());
    };
    if fs::canonicalize(dst_path).ok().as_deref() == Some(resolved_src.as_path()) {
        return Ok(());
    }
    if fs::symlink_metadata(dst_path).is_ok() {
        fs::remove_file(dst_path)
            .map_err(|e| format!("Could not remove {}: {e}", dst_path.display()))?;
    }
    std::os::unix::fs::symlink(&resolved_src, dst_path)
        .map_err(|e| format!("Could not link {}: {e}", dst_path.display()))
}

/// Direct3D/DXGI modules DXVK provides, and D3D12 modules VKD3D-Proton
/// provides — preferred as native since a plain Wine build's own builtin
/// implementations of these are the unaccelerated, OpenGL-backed ones
/// DXVK/VKD3D-Proton exist to replace. Builtin stays the fallback (`n,b`,
/// as Bottles does) for a module the cached release doesn't ship, e.g.
/// `d3d8` in DXVK before 2.1, which would otherwise not load at all.
const DXVK_MODULES: &[&str] = &["d3d8", "d3d9", "d3d10core", "d3d11", "dxgi"];
const VKD3D_MODULES: &[&str] = &["d3d12", "d3d12core"];

/// Re-links DXVK and VKD3D-Proton's DLLs (from the shared cache — see
/// `graphics_layers::ensure_directx_layer_cache`) into a Wine prefix's
/// `system32`/`syswow64`, and returns the `WINEDLLOVERRIDES` value that
/// forces wine to actually load them. Mirrors PortProton's own
/// `CP_DXVK_FILES`/`CP_VKD3D_FILES` handling in `functions_helper`: a
/// prefix imported from another tool, or last used with a Proton runner,
/// may have these files pointing at a different build or be missing them
/// entirely, so they're re-linked on every launch rather than once.
fn sync_directx_overrides_from_cache(cache_dir: &Path, prefix_path: &Path) -> Result<String, String> {
    let windows_dir = prefix_path.join("drive_c/windows");
    let win32 = is_win32_prefix(prefix_path);

    for (layer_dir, dir64, dir32, modules) in [
        ("dxvk", "x64", "x32", DXVK_MODULES),
        ("vkd3d-proton", "x64", "x86", VKD3D_MODULES),
    ] {
        // A 32-bit prefix's `system32` holds 32-bit DLLs, and it has no
        // `syswow64` at all.
        let targets: &[(&str, &str)] = if win32 {
            &[(dir32, "system32")]
        } else {
            &[(dir64, "system32"), (dir32, "syswow64")]
        };
        for &(arch_dir, subdir) in targets {
            let src_dir = cache_dir.join(layer_dir).join(arch_dir);
            let dst_dir = windows_dir.join(subdir);
            if !dst_dir.is_dir() {
                continue;
            }
            for module in modules {
                let file_name = format!("{module}.dll");
                relink_if_needed(&src_dir.join(&file_name), &dst_dir.join(&file_name))?;
            }
        }
    }

    let modules: Vec<&str> = DXVK_MODULES
        .iter()
        .chain(VKD3D_MODULES.iter())
        .copied()
        .collect();
    Ok(format!("{}=n,b", modules.join(",")))
}

/// Whether a prefix was created as 32-bit only (`WINEARCH=win32`), as older
/// prefixes imported from other tools sometimes are. Wine records that in
/// the header of `system.reg`, e.g. `#arch=win32`.
fn is_win32_prefix(prefix_path: &Path) -> bool {
    use std::io::Read;
    let mut header = String::new();
    fs::File::open(prefix_path.join("system.reg"))
        .and_then(|file| file.take(512).read_to_string(&mut header))
        .is_ok_and(|_| header.lines().any(|line| line.trim() == "#arch=win32"))
}

/// Installs wine-mono into `prefix_path` via `msiexec`, unless it's already
/// there (`drive_c/windows/mono/`, where the installer places it). Wine
/// looks for it there whenever an app tries to host a .NET assembly, and,
/// finding it missing, shows its own "Wine Mono Installation" dialog asking
/// to download and install it — which would otherwise pop up unprompted on
/// whatever needs it first. Proton bundles Mono and installs it
/// automatically via its own wrapper script; a plain Wine build (e.g.
/// Kron4ek) doesn't, so this does the same job for a Wine-kind runner.
async fn install_wine_mono(
    app: &AppHandle,
    wine: &Path,
    runner_path: &Path,
    prefix_path: &Path,
    log_path: &Path,
) -> Result<(), String> {
    if prefix_path.join("drive_c/windows/mono").is_dir() {
        return Ok(());
    }

    let msi_path = ensure_wine_mono_msi(app, runner_path).await?;
    let prefix_str = prefix_path
        .to_str()
        .ok_or_else(|| format!("Prefix path is not valid UTF-8: {}", prefix_path.display()))?;
    let msi_str = msi_path
        .to_str()
        .ok_or_else(|| format!("Installer path is not valid UTF-8: {}", msi_path.display()))?;

    let out = fs::OpenOptions::new()
        .append(true)
        .open(log_path)
        .map_err(|e| format!("Could not open log file: {e}"))?;
    let err = out
        .try_clone()
        .map_err(|e| format!("Could not open log file: {e}"))?;

    let status = runner_command(wine, [("WINEPREFIX", prefix_str)])
        .args(["msiexec", "/i", msi_str, "/qn"])
        .stdout(Stdio::from(out))
        .stderr(Stdio::from(err))
        .status()
        .await
        .map_err(|e| format!("Could not run wine-mono installer: {e}"))?;

    if !status.success() {
        return Err(format!("wine-mono installer exited with status {status}"));
    }
    Ok(())
}

/// Steers a prefix's Windows user profile into `drive_c/users/steamuser`
/// instead of the real Linux username, by pre-creating that directory and
/// symlinking the real username to it before wineboot ever runs — wineboot
/// then transparently populates the profile through the symlink instead of
/// creating a separate one. Proton's own prefixes are already laid out this
/// way (Valve's `steamuser` is a fixed pseudo-account, not the host's real
/// user), and plenty of games — plus some anti-cheat systems — hardcode
/// "steamuser" as the expected profile name regardless of which
/// compatibility layer actually launched them, so a profile created under
/// the real username can break them (see PortProton's
/// `check_dirs_and_files_in_pfx`, which does the same thing).
///
/// A no-op once `drive_c/users/<username>` exists as its own real directory
/// or symlink — imported from elsewhere, or already initialized before this
/// existed — so this never clobbers a prefix's actual profile.
///
/// `pub(crate)`: `commands::winetricks` needs this too, since winetricks can
/// be the very first thing to touch a fresh prefix (installing dependencies
/// before ever launching the game once) and implicitly runs wineboot itself.
pub(crate) fn steer_profile_to_steamuser(prefix_path: &Path) -> Result<(), String> {
    let users_dir = prefix_path.join("drive_c/users");
    let steamuser_dir = users_dir.join("steamuser");
    fs::create_dir_all(&steamuser_dir)
        .map_err(|e| format!("Could not create {}: {e}", steamuser_dir.display()))?;

    let Some(username) = std::env::var_os("USER").filter(|u| u != "steamuser") else {
        return Ok(());
    };
    let user_dir = users_dir.join(&username);
    if fs::symlink_metadata(&user_dir).is_ok() {
        return Ok(());
    }
    std::os::unix::fs::symlink("steamuser", &user_dir)
        .map_err(|e| format!("Could not link {}: {e}", user_dir.display()))
}

/// Opens a launch's log file for appending, as a stdout/stderr pair for a
/// child process.
pub(crate) fn log_stdio(log_path: &Path) -> Result<(Stdio, Stdio), String> {
    let out = fs::OpenOptions::new()
        .append(true)
        .open(log_path)
        .map_err(|e| format!("Could not open log file: {e}"))?;
    let err = out
        .try_clone()
        .map_err(|e| format!("Could not open log file: {e}"))?;
    Ok((Stdio::from(out), Stdio::from(err)))
}

/// A prefix readied to run something in under a runner (see
/// `prepare_prefix`): the binary to run it with and the environment for it.
pub(crate) struct PreparedPrefix {
    /// umu-run for a Proton runner, the runner's `wine` for a Wine runner.
    /// Both take the exe (or a builtin tool's name) as their first argument.
    pub binary: PathBuf,
    pub env: Vec<(String, String)>,
}

/// Readies `prefix_path` to run something in under `runner` — the same for
/// a game, an installer, winetricks or one of Wine's own tools, so a fresh
/// prefix gets set up the same way whichever of them comes first. Output of
/// the setup goes to `log_path`. `on_initializing` is called before a
/// first-time setup that can take minutes (a new prefix, a Steam Runtime
/// download), so a caller can say so rather than seem to hang.
///
/// The environment has `WINEPREFIX` and `WINEDLLOVERRIDES`, with
/// `winemenubuilder.exe` disabled unconditionally: wine's default behavior
/// of registering .desktop entries and file associations for whatever runs
/// in the prefix is never wanted here, since this app is itself the game's
/// launcher/menu. Proton appends its own overrides to this rather than
/// replacing it.
pub(crate) async fn prepare_prefix(
    app: &AppHandle,
    token: Option<&str>,
    runner: &Runner,
    prefix_path: &Path,
    log_path: &Path,
    on_initializing: &(dyn Fn() + Sync),
) -> Result<PreparedPrefix, String> {
    let prefix_path_str = prefix_path
        .to_str()
        .ok_or_else(|| format!("Prefix path is not valid UTF-8: {}", prefix_path.display()))?;
    let mut env = vec![("WINEPREFIX".to_string(), prefix_path_str.to_string())];
    let mut dll_overrides = vec!["winemenubuilder.exe=".to_string()];

    let binary = match runner.kind {
        RunnerKind::Proton => {
            prepare_proton(app, token, runner, prefix_path, log_path, on_initializing, &mut env)
                .await?
        }
        RunnerKind::Wine => {
            // Proton turns wine's debug channels off itself unless asked to
            // log; plain Wine would write every `fixme:` into the log, which
            // costs some games noticeable performance. A game's own
            // `WINEDEBUG` comes later and still wins.
            env.push(("WINEDEBUG".to_string(), "-all".to_string()));
            prepare_wine(app, runner, prefix_path, log_path, on_initializing, &mut dll_overrides)
                .await?
        }
    };
    env.push(("WINEDLLOVERRIDES".to_string(), dll_overrides.join(";")));
    Ok(PreparedPrefix { binary, env })
}

/// Readies a Proton runner's launch and returns the binary to launch the
/// game through: umu-run (see `commands::umu`), which hands it to the
/// runner's real `proton` script inside the Steam Linux Runtime. Proton then
/// creates the prefix, keeps its DXVK/VKD3D in sync with the runner and
/// installs Mono/fonts itself, so nothing in the prefix needs setting up by
/// hand here — umu even steers the user profile to `steamuser` the same way
/// `steer_profile_to_steamuser` does for a Wine runner.
///
/// The one thing done up front is an explicit `createprefix` whenever the
/// prefix or the Steam Runtime this runner needs doesn't exist yet: both can
/// take minutes on first use (the runtime alone is several hundred MB),
/// which this surfaces as the frontend's "initializing" state instead of an
/// unexplained wait before the game appears.
async fn prepare_proton(
    app: &AppHandle,
    token: Option<&str>,
    runner: &Runner,
    prefix_path: &Path,
    log_path: &Path,
    on_initializing: &(dyn Fn() + Sync),
    env: &mut Vec<(String, String)>,
) -> Result<PathBuf, String> {
    let prefix_path_str = prefix_path.to_string_lossy();
    let is_set_up = || prefix_path.join("drive_c").is_dir() && runtime_present(&runner.path);
    if !is_set_up() {
        on_initializing();
        let (out, err) = log_stdio(log_path)?;
        // Exit status deliberately not checked: after setting everything up,
        // GE-Proton still tries to launch the empty exe `createprefix` hands
        // it and exits 1 on "file not found", even on complete success.
        prefix_command(app, token, runner, &prefix_path_str)
            .await?
            .arg("createprefix")
            .stdout(out)
            .stderr(err)
            .status()
            .await
            .map_err(|e| format!("Could not initialize prefix: {e}"))?;
        if !is_set_up() {
            return Err("Prefix initialization failed".to_string());
        }
    }

    env.push(("PROTONPATH".to_string(), runner.path.display().to_string()));
    ensure_umu(app, token).await
}

/// `WINEDLLOVERRIDES` for a fresh prefix's wineboot. Without `mscoree` and
/// `mshtml`, wineboot looks for wine-mono and wine-gecko, and a plain Wine
/// build (e.g. Kron4ek) ships neither, so Wine would ask to download each in
/// a dialog of its own. Mono is installed right after (`install_wine_mono`);
/// Gecko is left out, as Lutris does by default. `winemenubuilder.exe` stays
/// off here too, as in `run_game`.
const WINEBOOT_DLL_OVERRIDES: &str = "mscoree,mshtml=;winemenubuilder.exe=";

/// Readies a Wine runner's launch and returns its `wine` binary, which the
/// game is launched with directly. Unlike Proton, a plain Wine build does
/// none of its own prefix setup, so that happens here: a fresh prefix gets
/// initialized with wineboot, wine-mono installed, and DXVK/VKD3D-Proton
/// linked in (see `sync_directx_overrides_from_cache`), with the
/// `WINEDLLOVERRIDES` entries those need appended to `dll_overrides`.
async fn prepare_wine(
    app: &AppHandle,
    runner: &Runner,
    prefix_path: &Path,
    log_path: &Path,
    on_initializing: &(dyn Fn() + Sync),
    dll_overrides: &mut Vec<String>,
) -> Result<PathBuf, String> {
    let wine = wine_binary(&runner.path)?;
    let prefix_path_str = prefix_path.to_string_lossy();

    // A prefix is created without ever running wineboot (see `add_prefix`),
    // so an empty one — no `drive_c` yet — is initialized here on first use.
    // Checked before `steer_profile_to_steamuser`, which creates
    // `drive_c/users/steamuser` and so `drive_c` itself.
    let fresh = !prefix_path.join("drive_c").is_dir();
    // Must run before wineboot's first initialization of this prefix (see
    // `steer_profile_to_steamuser`), but is otherwise idempotent, so it's
    // simplest to just always ensure it — cheap, and self-healing if
    // something ever removed the symlink.
    steer_profile_to_steamuser(prefix_path)?;
    if fresh {
        on_initializing();
        let (out, err) = log_stdio(log_path)?;
        let env = [
            ("WINEPREFIX", prefix_path_str.as_ref()),
            ("WINEDLLOVERRIDES", WINEBOOT_DLL_OVERRIDES),
        ];
        let status = runner_command(&wine, env)
            .arg("wineboot")
            .stdout(out)
            .stderr(err)
            .status()
            .await
            .map_err(|e| format!("Could not initialize prefix: {e}"))?;
        if !status.success() {
            return Err(format!("Prefix initialization failed with status {status}"));
        }
    }

    install_wine_mono(app, &wine, &runner.path, prefix_path, log_path).await?;
    let cache = ensure_directx_layer_cache(app).await?;
    dll_overrides.push(sync_directx_overrides_from_cache(&cache, prefix_path)?);

    // Proton only re-copies its own DXVK/VKD3D (and builtin DLLs) into a
    // prefix when its `config_info` marker says the prefix was last set up
    // differently. The files were just replaced here, so drop that marker —
    // otherwise switching this prefix back to the same Proton runner later
    // would keep running on this upstream DXVK instead of Proton's own.
    let config_info = prefix_path.join("config_info");
    if config_info.exists() {
        fs::remove_file(&config_info)
            .map_err(|e| format!("Could not remove {}: {e}", config_info.display()))?;
    }

    Ok(wine)
}

/// Launches a game's exe under its configured runner and prefix. stdout/stderr
/// are redirected straight into a per-run log file. The frontend is told about
/// the outcome both via the command's own `Result` and via `game-started` /
/// `game-exited` / `game-launch-error` events, so it can show a live "running"
/// state as well as a final error with a link to the log.
#[tauri::command]
pub async fn launch_game(
    app: AppHandle,
    state: State<'_, ConfigState>,
    running: State<'_, RunningGames>,
    launching: State<'_, LaunchingGames>,
    id: String,
) -> Result<(), AppError> {
    let game_id = Uuid::parse_str(&id).map_err(|e| format!("Invalid game id: {e}"))?;
    // Refused without a `game-launch-error`: that would mark the launch
    // already in progress as failed.
    let Some(_guard) = launching.claim(game_id) else {
        return Err(AppError::GameAlreadyRunning);
    };

    let log_path = new_log_file(&game_log_dir(&app, &id)?)?;

    // Every failure from here on is reported in this one place — including
    // ones while still preparing, after `game-initializing` already went out
    // (e.g. a failed umu or Steam Runtime download) — so the frontend never
    // gets stuck showing a launch as still in progress.
    let result = run_game(&app, &state, &running, &id, &log_path).await;
    if let Err(message) = &result {
        let _ = app.emit(
            "game-launch-error",
            GameLaunchErrorPayload {
                id: &id,
                message: message.clone(),
                log_path: Some(log_path.display().to_string()),
            },
        );
    }
    result.map_err(AppError::from)
}

/// Runs a game without any window and exits along with it: how Steam
/// starts games exported to it (`--run <game-id>`, see `commands::steam`),
/// so Steam counts the game as running exactly as long as it really runs.
/// With no window to show an error in, a failure before the game got going
/// becomes a dialog. A game that ran and then exited with an error only
/// sets the exit code; its log has the details.
pub fn launch_game_headless(app: &AppHandle, id: String) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let started = Arc::new(AtomicBool::new(false));
        let flag = started.clone();
        app.listen_any("game-started", move |_| flag.store(true, Ordering::SeqCst));

        let logs_dir = game_log_dir(&app, &id).ok();
        let result = launch_game(app.clone(), app.state(), app.state(), app.state(), id).await;
        let code = match result {
            Ok(()) => 0,
            Err(message) => {
                if !started.load(Ordering::SeqCst) {
                    let locale = app.state::<LocaleState>().get();
                    let mut text = message.localized(locale);
                    if let Some(dir) = logs_dir {
                        text.push_str(&format!("\n\nLogs: {}", dir.display()));
                    }
                    let title = match locale {
                        Locale::De => "Spiel konnte nicht gestartet werden",
                        Locale::En => "Game could not be started",
                    };
                    let dialog = app
                        .dialog()
                        .message(text)
                        .title(title)
                        .kind(MessageDialogKind::Error);
                    let _ = tauri::async_runtime::spawn_blocking(move || dialog.blocking_show()).await;
                }
                1
            }
        };
        app.exit(code);
    });
}

async fn run_game(
    app: &AppHandle,
    state: &State<'_, ConfigState>,
    running: &State<'_, RunningGames>,
    id: &str,
    log_path: &Path,
) -> Result<(), AppError> {
    let game_id = Uuid::parse_str(id).map_err(|e| format!("Invalid game id: {e}"))?;

    let (game, runners_dir, settings) = {
        let config = state
            .lock()
            .map_err(|_| "Configuration is locked".to_string())?;
        let game = config
            .games
            .iter()
            .find(|g| g.id == game_id)
            .cloned()
            .ok_or_else(|| format!("No game with id {id}"))?;
        let settings = game.overrides.resolve(
            &config.performance,
            &config.graphics,
            &config.mangohud,
            &config.proton,
        );
        (game, config.runners_dir.clone(), settings)
    };
    let performance = &settings.performance;
    let graphics = &settings.graphics;
    let mangohud = &settings.mangohud;
    let token = read_token(state)?;

    let runner = find_runner(&runners_dir, &game.runner_id)?;
    let log_path_string = log_path.display().to_string();
    let wineserver = wineserver_binary(&runner.path)?;

    let on_initializing = || {
        let _ = app.emit("game-initializing", GameInitializingPayload { id });
    };
    let PreparedPrefix {
        binary: runner_binary,
        mut env,
    } = prepare_prefix(
        app,
        token.as_deref(),
        &runner,
        &game.prefix_path,
        log_path,
        &on_initializing,
    )
    .await?;
    // `kill_running_game` needs this exact value to reach the right
    // wineserver session.
    let wineprefix = game.prefix_path.to_string_lossy().into_owned();

    // Gamescope goes outermost (see below). It's a Vulkan program itself, so
    // the implicit-layer switches meant for the game would also load
    // MangoHud and vkBasalt into gamescope, drawing the overlay and applying
    // the effects a second time on its output. With gamescope, those go into
    // `game_only_env` instead, which only the command inside it gets, and
    // MangoHud preferably runs as gamescope's own `--mangoapp`. Skipped if
    // we're already inside a gamescope session ourselves — nesting it again
    // is pointless and often broken.
    let use_gamescope = graphics.gamescope.enabled
        && command_on_path("gamescope")
        && std::env::var_os("GAMESCOPE_WAYLAND_DISPLAY").is_none();
    let use_mangoapp = use_gamescope && mangohud.enabled && command_on_path("mangoapp");
    let mut game_only_env: Vec<(String, String)> = Vec::new();

    if mangohud.enabled {
        let conf_path = ensure_mangohud_conf(app, id, &mangohud.layout)?;
        // Also read by mangoapp, which gamescope starts with its own env.
        env.push((
            "MANGOHUD_CONFIGFILE".to_string(),
            conf_path.display().to_string(),
        ));
        if !use_mangoapp {
            let target = if use_gamescope { &mut game_only_env } else { &mut env };
            target.push(("MANGOHUD".to_string(), "1".to_string()));
        }
    }
    // GameMode writes niceness/CPU-governor directly, which fights a
    // competing auto-nice/scheduler daemon if one is running — so in that
    // case we skip the LD_PRELOAD and fall back to `power_profile` instead
    // (see `use_power_profile` below), which doesn't touch niceness at all.
    let scheduler_conflict =
        performance.gamemode_enabled && competing_scheduler_active().await;
    if performance.gamemode_enabled && !scheduler_conflict {
        env.push(("LD_PRELOAD".to_string(), "libgamemodeauto.so.0".to_string()));
    }
    if graphics.vkbasalt.enabled {
        let vkbasalt_conf = ensure_vkbasalt_conf(app, id, &graphics.vkbasalt)?;
        let target = if use_gamescope { &mut game_only_env } else { &mut env };
        target.push(("ENABLE_VKBASALT".to_string(), "1".to_string()));
        target.push((
            "VKBASALT_CONFIG_FILE".to_string(),
            vkbasalt_conf.display().to_string(),
        ));
    }
    // Before the game's own env vars, so a hand-written entry for the same
    // variable still wins. Wine runners don't read these names at all.
    if runner.kind == RunnerKind::Proton {
        env.extend(game.umu_env());
        env.extend(settings.proton_env());
    }
    add_game_env(&mut env, &game.env_vars);
    keep_inherited_preload(&mut env, std::env::var("LD_PRELOAD").ok());

    // Builds the actual launch as a chain of wrappers around the runner
    // binary (umu-run or wine), each one prepended in outer-to-inner order
    // (so the last one added is the one that directly execs the runner
    // binary). Both wrappers are only attempted once their binary and
    // backing service actually look reachable — otherwise the wrapper itself
    // would exit immediately, taking the whole game launch down with it
    // since it'd be the process we spawn.
    let mut launch_chain = vec![runner_binary.display().to_string()];

    // `powerprofilesctl launch` holds the desktop at the "performance" power
    // profile for exactly as long as this launch runs, releasing it
    // automatically on exit (even a crash) since the hold lives on the
    // D-Bus connection it opens. A quick `get` call first checks the daemon
    // is actually running and not just installed.
    let use_power_profile = (performance.power_profile_enabled || scheduler_conflict)
        && command_on_path("powerprofilesctl")
        && Path::new("/run/dbus/system_bus_socket").exists()
        && tokio::process::Command::new("powerprofilesctl")
            .arg("get")
            .output()
            .await
            .map(|out| out.status.success())
            .unwrap_or(false);
    if use_power_profile {
        // No `--` separator: `launch`'s `arguments` is an argparse
        // `REMAINDER` positional, which the upstream examples feed straight
        // after the named options rather than behind an explicit `--`.
        let mut wrapped = vec![
            "powerprofilesctl".to_string(),
            "launch".to_string(),
            "--profile".to_string(),
            "performance".to_string(),
            "--reason".to_string(),
            "Prefixr: game running".to_string(),
            "--appid".to_string(),
            "prefixr".to_string(),
        ];
        wrapped.append(&mut launch_chain);
        launch_chain = wrapped;
    }

    let use_inhibit_sleep = performance.inhibit_sleep_enabled
        && command_on_path("systemd-inhibit")
        && Path::new("/run/dbus/system_bus_socket").exists();
    if use_inhibit_sleep {
        let mut wrapped = vec![
            "systemd-inhibit".to_string(),
            "--mode=block".to_string(),
            "--who=Prefixr".to_string(),
            "--why=A game is running".to_string(),
            "--what=idle:sleep".to_string(),
            "--".to_string(),
        ];
        wrapped.append(&mut launch_chain);
        launch_chain = wrapped;
    }

    // Gamescope goes outermost: it opens its own nested compositor session
    // and everything else (wine, the other wrappers) runs inside it.
    if use_gamescope {
        let mut wrapped = vec!["gamescope".to_string()];
        if let Some(width) = graphics.gamescope.width {
            wrapped.push("-w".to_string());
            wrapped.push(width.to_string());
        }
        if let Some(height) = graphics.gamescope.height {
            wrapped.push("-h".to_string());
            wrapped.push(height.to_string());
        }
        if let Some(fps) = graphics.gamescope.fps_limit {
            wrapped.push("-r".to_string());
            wrapped.push(fps.to_string());
        }
        if graphics.gamescope.fullscreen {
            wrapped.push("-f".to_string());
        }
        if use_mangoapp {
            wrapped.push("--mangoapp".to_string());
        }
        wrapped.push("--".to_string());
        if !game_only_env.is_empty() {
            wrapped.push("env".to_string());
            wrapped.extend(game_only_env.iter().map(|(key, value)| format!("{key}={value}")));
        }
        wrapped.append(&mut launch_chain);
        launch_chain = wrapped;
    }

    let launch_binary = PathBuf::from(&launch_chain[0]);
    let wrapper_args = launch_chain[1..].to_vec();
    let launch_env = env;

    // Per-game exe arguments (PortProton calls these `LAUNCH_PARAMETERS`),
    // e.g. `--launcher-skip -dx11`.
    let game_launch_args = split_launch_args(&game.launch_args)?;

    if scheduler_conflict {
        let note = if use_power_profile {
            "[prefixr] GameMode uebersprungen: konkurrierender Scheduler-Daemon aktiv, nutze Power-Profile stattdessen\n"
        } else {
            "[prefixr] GameMode uebersprungen: konkurrierender Scheduler-Daemon aktiv, powerprofilesctl nicht verfuegbar\n"
        };
        let _ = fs::OpenOptions::new()
            .append(true)
            .open(log_path)
            .and_then(|mut f| std::io::Write::write_all(&mut f, note.as_bytes()));
    }

    let (log_out, log_err) = log_stdio(log_path)?;

    let mut child = runner_command(&launch_binary, env_pairs(&launch_env))
        .args(&wrapper_args)
        .arg(&game.exe_path)
        .args(&game_launch_args)
        .stdout(log_out)
        .stderr(log_err)
        // Detaches the game from this app's own process group, so e.g. a
        // Ctrl+C on a dev server running Prefixr doesn't also hit the game.
        .process_group(0)
        .spawn()
        .map_err(|e| format!("Could not start game: {e}"))?;

    let killed = Arc::new(AtomicBool::new(false));
    if let Ok(mut running) = running.0.lock() {
        running.insert(
            game_id,
            RunningGame {
                name: game.name.clone(),
                pid: child.id(),
                wineserver: wineserver.clone(),
                wineprefix: wineprefix.clone(),
                exe_path: game.exe_path.clone(),
                killed: killed.clone(),
            },
        );
    }
    rebuild_tray_menu(app);

    let _ = app.emit(
        "game-started",
        GameStartedPayload {
            id,
            log_path: log_path_string,
        },
    );

    let wait_result = child.wait().await;

    if let Ok(mut running) = running.0.lock() {
        running.remove(&game_id);
    }
    rebuild_tray_menu(app);

    let status = wait_result.map_err(|e| format!("Game process failed: {e}"))?;

    let _ = app.emit(
        "game-exited",
        GameExitedPayload {
            id,
            exit_code: status.code(),
        },
    );

    if !status.success() && !killed.load(Ordering::SeqCst) {
        return Err(format!("Game exited with status {status}").into());
    }

    Ok(())
}

/// Variables holding a list, with their separator: a game's own value is
/// appended to the one set up for the launch rather than replacing it.
/// Otherwise a hand-written `WINEDLLOVERRIDES` would drop the DXVK/VKD3D
/// overrides (a later entry for the same DLL still wins in wine), and a
/// hand-written `LD_PRELOAD` would drop GameMode.
const LIST_ENV_VARS: &[(&str, &str)] = &[("WINEDLLOVERRIDES", ";"), ("LD_PRELOAD", ":")];

/// Adds a game's own env vars after the ones set up for the launch, so they
/// win — except for `LIST_ENV_VARS`, which are merged.
fn add_game_env(env: &mut Vec<(String, String)>, game_vars: &HashMap<String, String>) {
    for (key, value) in game_vars {
        let separator = LIST_ENV_VARS
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, separator)| *separator);
        let existing = env.iter_mut().find(|(k, _)| k == key);
        match (separator, existing) {
            (Some(separator), Some((_, existing))) if !value.is_empty() => {
                existing.push_str(separator);
                existing.push_str(value);
            }
            _ => env.push((key.clone(), value.clone())),
        }
    }
}

/// Appends the `LD_PRELOAD` Prefixr itself was started with to the one set up
/// for the launch, which would otherwise replace it. That's how Steam
/// injects its overlay into a game it started via `--run`, so without this
/// GameMode (or a game's own `LD_PRELOAD`) would switch the overlay off. A
/// launch that doesn't set `LD_PRELOAD` inherits it unchanged anyway, and a
/// game's explicitly empty one still clears it.
fn keep_inherited_preload(env: &mut [(String, String)], inherited: Option<String>) {
    let Some(inherited) = inherited.filter(|value| !value.trim().is_empty()) else {
        return;
    };
    // The last entry for a variable is the one the process gets.
    if let Some((_, value)) = env.iter_mut().rev().find(|(key, _)| key == "LD_PRELOAD") {
        if !value.is_empty() {
            value.push(':');
            value.push_str(&inherited);
        }
    }
}

/// The user's Desktop folder, honoring a localized `XDG_DESKTOP_DIR` (e.g.
/// "Schreibtisch" on a German system) if `~/.config/user-dirs.dirs` sets one,
/// falling back to `~/Desktop`.
fn desktop_directory() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;

    if let Ok(contents) = fs::read_to_string(home.join(".config/user-dirs.dirs")) {
        for line in contents.lines() {
            if let Some(value) = line.trim().strip_prefix("XDG_DESKTOP_DIR=") {
                let value = value
                    .trim_matches('"')
                    .replace("$HOME", &home.to_string_lossy());
                return Ok(PathBuf::from(value));
            }
        }
    }

    Ok(home.join("Desktop"))
}

/// The user's XDG applications directory (`$XDG_DATA_HOME/applications`,
/// falling back to `~/.local/share/applications`) — where a `.desktop` file
/// needs to live for the desktop environment's start menu / app launcher to
/// pick it up, as opposed to `desktop_directory` for an icon on the Desktop.
fn applications_directory() -> Result<PathBuf, String> {
    if let Some(data_home) = std::env::var_os("XDG_DATA_HOME") {
        return Ok(PathBuf::from(data_home).join("applications"));
    }
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;
    Ok(home.join(".local/share/applications"))
}

/// Keeps a filename safe across filesystems by replacing anything but
/// alphanumerics, spaces, dashes and underscores.
fn sanitize_filename(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        "game".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Resolves the file path a `.desktop` shortcut's `Icon=` should point to.
/// Prefers a chosen SteamGridDB icon (already cached to disk as a real file)
/// over the exe's embedded icon, since it's the higher-quality, user-picked
/// option when both exist.
pub(crate) fn write_shortcut_icon(app: &AppHandle, game: &Game) -> Result<Option<PathBuf>, String> {
    if let Some(icon_url) = &game.steamgriddb_icon_url {
        let ext = image_extension(icon_url);
        let path = asset_cache_path(&artwork_dir(app)?, game.id, "_icon", ext);
        if path.exists() {
            return Ok(Some(path));
        }
    }
    let path = exe_icon_path(app, game.id)?;
    Ok(path.is_file().then_some(path))
}

/// Resolves the path a `.desktop` shortcut should point to. When running from
/// an AppImage, `current_exe()` returns a path inside a temporary FUSE mount
/// (`/tmp/.mount_XXXXXX/...`) that's torn down when the process exits and
/// re-randomized on every launch — useless for a persistent shortcut.
/// AppImages set `APPIMAGE` to the real `.AppImage` file's path exactly for
/// cases like this, so that's preferred when present.
pub(crate) fn own_executable_path() -> Result<PathBuf, String> {
    if let Some(appimage_path) = std::env::var_os("APPIMAGE") {
        return Ok(PathBuf::from(appimage_path));
    }
    std::env::current_exe().map_err(|e| format!("Could not resolve own executable path: {e}"))
}

/// Escapes a value for a `.desktop` file: a backslash starts an escape
/// sequence there, and a line break would end the entry — e.g. a game name
/// pasted with a trailing newline.
fn desktop_string(value: &str) -> String {
    value
        .chars()
        .map(|c| match c {
            '\\' => "\\\\".to_string(),
            c if c.is_control() => " ".to_string(),
            c => c.to_string(),
        })
        .collect()
}

/// Quotes a path as one argument of an `Exec=` line, per the Desktop Entry
/// spec: inside the quotes, `"`, `` ` ``, `$` and `\` take a backslash; the
/// whole line is then a string value, whose own escaping doubles every
/// backslash; and a literal `%` is `%%`, since `%` starts a field code.
fn desktop_exec_arg(path: &Path) -> String {
    let mut quoted = String::from("\"");
    for c in path.to_string_lossy().chars() {
        match c {
            '"' | '`' | '$' => {
                quoted.push_str("\\\\");
                quoted.push(c);
            }
            '\\' => quoted.push_str("\\\\\\\\"),
            '%' => quoted.push_str("%%"),
            c if c.is_control() => quoted.push(' '),
            c => quoted.push(c),
        }
    }
    quoted.push('"');
    quoted
}

/// The `.desktop` files in `dir` that start this game, recognized by the
/// `--launch <id>` on their `Exec=` line — which also finds ones from older
/// versions, named after the game alone.
fn game_shortcuts(dir: &Path, game_id: Uuid) -> Vec<PathBuf> {
    let launch = format!("--launch {game_id}");
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "desktop"))
        .filter(|path| {
            fs::read_to_string(path).is_ok_and(|contents| {
                contents
                    .lines()
                    .any(|line| line.starts_with("Exec=") && line.contains(&launch))
            })
        })
        .collect()
}

/// Looks up a game by id and writes a `.desktop` file for it into
/// `target_dir`, launching this game directly by re-invoking the app's own
/// executable with `--launch <game-id>` (picked up on startup via
/// `take_pending_launch`). Shared by `create_desktop_shortcut` and
/// `create_menu_shortcut`, which only differ in which directory a `.desktop`
/// file needs to land in to show up.
fn write_game_shortcut(
    app: &AppHandle,
    state: &State<ConfigState>,
    id: &str,
    target_dir: &Path,
) -> Result<(), String> {
    let game_id = Uuid::parse_str(id).map_err(|e| format!("Invalid game id: {e}"))?;
    let game = {
        let config = state
            .lock()
            .map_err(|_| "Configuration is locked".to_string())?;
        config
            .games
            .iter()
            .find(|g| g.id == game_id)
            .cloned()
            .ok_or_else(|| format!("No game with id {id}"))?
    };

    fs::create_dir_all(target_dir)
        .map_err(|e| format!("Could not access {}: {e}", target_dir.display()))?;

    let icon_path = write_shortcut_icon(app, &game)?;
    let exe_path = own_executable_path()?;

    let mut contents = String::new();
    contents.push_str("[Desktop Entry]\n");
    contents.push_str("Type=Application\n");
    contents.push_str(&format!("Name={}\n", desktop_string(&game.name)));
    contents.push_str(&format!(
        "Exec={} --launch {}\n",
        desktop_exec_arg(&exe_path),
        game.id
    ));
    if let Some(icon_path) = &icon_path {
        contents.push_str(&format!("Icon={}\n", desktop_string(&icon_path.to_string_lossy())));
    }
    contents.push_str("Terminal=false\n");
    contents.push_str("Categories=Game;\n");

    // The id in the name keeps two games of the same (sanitized) name apart;
    // a shortcut from before a rename, or from an older version named after
    // the game alone, is replaced rather than left behind.
    for old in game_shortcuts(target_dir, game.id) {
        let _ = fs::remove_file(old);
    }
    let short_id = &game.id.simple().to_string()[..8];
    let shortcut_path =
        target_dir.join(format!("{}-{short_id}.desktop", sanitize_filename(&game.name)));
    fs::write(&shortcut_path, contents)
        .map_err(|e| format!("Could not write shortcut file: {e}"))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&shortcut_path)
            .map_err(|e| format!("Could not read shortcut permissions: {e}"))?
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&shortcut_path, perms)
            .map_err(|e| format!("Could not set shortcut permissions: {e}"))?;
    }

    Ok(())
}

/// Creates a `.desktop` shortcut on the user's Desktop that launches this
/// game directly.
#[tauri::command(async)]
pub fn create_desktop_shortcut(
    app: AppHandle,
    state: State<ConfigState>,
    id: String,
) -> Result<(), AppError> {
    let desktop_dir = desktop_directory()?;
    write_game_shortcut(&app, &state, &id, &desktop_dir).map_err(AppError::from)
}

/// Creates a `.desktop` entry in the user's XDG applications directory so
/// this game shows up in the desktop environment's start menu / app
/// launcher, mirroring how PortProton offers "Add to Menu" as a choice
/// separate from "Add to Desktop" rather than doing both at once.
/// `update-desktop-database` is nudged afterwards, best-effort, so menus
/// that cache entries (like KDE's) pick up the addition immediately instead
/// of waiting for their own refresh.
#[tauri::command(async)]
pub fn create_menu_shortcut(
    app: AppHandle,
    state: State<ConfigState>,
    id: String,
) -> Result<(), AppError> {
    let applications_dir = applications_directory()?;
    write_game_shortcut(&app, &state, &id, &applications_dir)?;
    let _ = std::process::Command::new("update-desktop-database")
        .arg(&applications_dir)
        .status();
    Ok(())
}

/// Bundled at compile time so the install `.desktop` entry always has an
/// icon to point to, regardless of packaging format (see
/// `ensure_install_desktop_entry`).
const APP_ICON_PNG: &[u8] = include_bytes!("../../icons/128x128.png");

/// MIME types shared-mime-info registers for Windows executables across
/// mainstream distros — matching any of these is what makes a file manager's
/// "Öffnen mit" offer this entry when right-clicking an `.exe`.
const EXE_MIME_TYPES: &str = "application/x-ms-dos-executable;application/x-msdownload;application/vnd.microsoft.portable-executable;";

/// Writes (idempotently overwrites) a second, hidden `.desktop` entry that
/// registers Prefixr as a handler for Windows executables — this is what
/// makes "Mit Prefixr installieren" show up when right-clicking an `.exe` in
/// a file manager's "Öffnen mit" menu, the same mechanism PortProton uses for
/// its own "Install with PortProton" entry. `NoDisplay=true` keeps it out of
/// the application menu/launcher, where the main `.desktop` entry (created by
/// the platform installer/bundler) already covers opening Prefixr itself.
/// Best-effort and re-run on every startup so it self-heals and stays in
/// sync with the app's own executable path (relevant for an AppImage, whose
/// mount path can move between updates).
pub fn ensure_install_desktop_entry(app: &AppHandle) -> Result<(), String> {
    let applications_dir = applications_directory()?;
    fs::create_dir_all(&applications_dir)
        .map_err(|e| format!("Could not access {}: {e}", applications_dir.display()))?;

    let icons_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Could not resolve data directory: {e}"))?
        .join("shortcut-icons");
    fs::create_dir_all(&icons_dir)
        .map_err(|e| format!("Could not create shortcut icons directory: {e}"))?;
    let icon_path = icons_dir.join("prefixr-install.png");
    fs::write(&icon_path, APP_ICON_PNG)
        .map_err(|e| format!("Could not write install icon: {e}"))?;

    let exe_path = own_executable_path()?;
    let mut contents = String::new();
    contents.push_str("[Desktop Entry]\n");
    contents.push_str("Type=Application\n");
    // No leading "Mit"/"with": file managers that build their own "Open
    // With %s" wrapper (e.g. GNOME Files) would otherwise double it up into
    // "Mit Mit Prefixr installieren öffnen" / "Open With Install with
    // Prefixr". Leaving the verb out of the name lets each file manager
    // compose its own sentence around it, same as every other app's entry.
    contents.push_str("Name=Prefixr installieren\n");
    contents.push_str("Name[en]=Install Prefixr\n");
    contents.push_str(&format!("Exec={} --install %f\n", desktop_exec_arg(&exe_path)));
    contents.push_str(&format!("Icon={}\n", desktop_string(&icon_path.to_string_lossy())));
    contents.push_str("Terminal=false\n");
    contents.push_str("NoDisplay=true\n");
    contents.push_str(&format!("MimeType={EXE_MIME_TYPES}\n"));

    let desktop_path = applications_dir.join("com.tristan.prefixr.install.desktop");
    fs::write(&desktop_path, contents)
        .map_err(|e| format!("Could not write install desktop entry: {e}"))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&desktop_path)
            .map_err(|e| format!("Could not read desktop entry permissions: {e}"))?
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&desktop_path, perms)
            .map_err(|e| format!("Could not set desktop entry permissions: {e}"))?;
    }

    let _ = std::process::Command::new("update-desktop-database")
        .arg(&applications_dir)
        .status();

    Ok(())
}

#[cfg(test)]
mod tests;
