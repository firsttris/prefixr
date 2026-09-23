use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use uuid::Uuid;

use crate::commands::github::read_token;
use crate::commands::graphics_layers::{ensure_directx_layer_cache, ensure_wine_mono_msi};
use crate::commands::icons::extract_icon_data_url;
use crate::commands::mangohud::ensure_mangohud_conf;
use crate::commands::performance::ensure_vkbasalt_conf;
use crate::commands::runners::{
    find_runner, prefix_command, runner_command, wine_binary, wineserver_binary,
};
use crate::commands::shell_link::{find_recently_created_shortcuts, DetectedShortcut};
use crate::commands::steamgriddb::{artwork_dir, asset_cache_path, image_extension};
use crate::commands::umu::{ensure_umu, runtime_present};
use crate::config::{save_config, ConfigState};
use crate::models::{Game, GameInput, Runner, RunnerKind};
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
}

#[derive(Default)]
pub struct RunningGames(pub Mutex<HashMap<Uuid, RunningGame>>);

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

#[tauri::command]
pub async fn kill_game(running: State<'_, RunningGames>, id: String) -> Result<(), String> {
    let game_id = Uuid::parse_str(&id).map_err(|e| format!("Invalid game id: {e}"))?;
    kill_running_game(&running, game_id).await
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
#[tauri::command]
pub async fn run_installer(
    app: AppHandle,
    state: State<'_, ConfigState>,
    prefix_path: String,
    runner_id: String,
    exe_path: String,
) -> Result<Vec<DetectedShortcut>, String> {
    let runners_dir = {
        let config = state
            .lock()
            .map_err(|_| "Configuration is locked".to_string())?;
        config.runners_dir.clone()
    };
    let token = read_token(&state)?;

    let runner = find_runner(&runners_dir, &runner_id)?;
    let prefix = PathBuf::from(&prefix_path);

    let exe = PathBuf::from(&exe_path);
    let mut command = prefix_command(&app, token.as_deref(), &runner, &prefix_path).await?;
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
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .map_err(|e| format!("Konnte Setup nicht starten: {e}"))?;

    Ok(find_recently_created_shortcuts(&prefix, started_at))
}

#[tauri::command]
pub fn list_games(state: State<ConfigState>) -> Result<Vec<Game>, String> {
    let config = state
        .lock()
        .map_err(|_| "Configuration is locked".to_string())?;
    Ok(config.games.clone())
}

#[tauri::command]
pub fn add_game(
    app: AppHandle,
    state: State<ConfigState>,
    game: GameInput,
) -> Result<Game, String> {
    let icon = extract_icon_data_url(&game.exe_path);
    let new_game = Game {
        id: Uuid::new_v4(),
        name: game.name,
        exe_path: game.exe_path,
        prefix_path: game.prefix_path,
        runner_id: game.runner_id,
        env_vars: game.env_vars,
        launch_args: game.launch_args,
        icon,
        steamgriddb_id: None,
        cover_grid_id: None,
        cover_url: None,
        steamgriddb_icon_grid_id: None,
        steamgriddb_icon_url: None,
    };

    let mut config = state
        .lock()
        .map_err(|_| "Configuration is locked".to_string())?;
    config.games.push(new_game.clone());
    save_config(&app, &config)?;
    Ok(new_game)
}

#[tauri::command]
pub fn update_game(
    app: AppHandle,
    state: State<ConfigState>,
    id: String,
    game: GameInput,
) -> Result<Game, String> {
    let game_id = Uuid::parse_str(&id).map_err(|e| format!("Invalid game id: {e}"))?;
    let mut config = state
        .lock()
        .map_err(|_| "Configuration is locked".to_string())?;

    let existing = config
        .games
        .iter_mut()
        .find(|g| g.id == game_id)
        .ok_or_else(|| format!("No game with id {id}"))?;

    existing.icon = extract_icon_data_url(&game.exe_path);
    existing.name = game.name;
    existing.exe_path = game.exe_path;
    existing.prefix_path = game.prefix_path;
    existing.runner_id = game.runner_id;
    existing.env_vars = game.env_vars;
    existing.launch_args = game.launch_args;
    let updated = existing.clone();

    save_config(&app, &config)?;
    Ok(updated)
}

#[tauri::command]
pub fn remove_game(app: AppHandle, state: State<ConfigState>, id: String) -> Result<(), String> {
    let game_id = Uuid::parse_str(&id).map_err(|e| format!("Invalid game id: {e}"))?;
    let mut config = state
        .lock()
        .map_err(|_| "Configuration is locked".to_string())?;

    if !config.games.iter().any(|g| g.id == game_id) {
        return Err(format!("No game with id {id}"));
    }

    config.games.retain(|g| g.id != game_id);
    save_config(&app, &config)?;
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
    message: String,
    log_path: Option<String>,
}

fn log_file_path(app: &AppHandle, game_id: &str) -> Result<PathBuf, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Could not resolve data directory: {e}"))?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("Could not read system time: {e}"))?
        .as_secs();
    // .txt rather than .log: most Linux desktops have no default app
    // registered for .log, so "Log anzeigen" would silently fail to open it.
    Ok(data_dir
        .join("logs")
        .join(game_id)
        .join(format!("{timestamp}.txt")))
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
/// provides — forced to load as native since a plain Wine build's own
/// builtin implementations of these are the unaccelerated, OpenGL-backed
/// ones DXVK/VKD3D-Proton exist to replace.
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

    for (layer_dir, dir64, dir32, modules) in [
        ("dxvk", "x64", "x32", DXVK_MODULES),
        ("vkd3d-proton", "x64", "x86", VKD3D_MODULES),
    ] {
        for (arch_dir, subdir) in [(dir64, "system32"), (dir32, "syswow64")] {
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
    Ok(format!("{}=n", modules.join(",")))
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
    prefix_path: &Path,
    log_path: &Path,
) -> Result<(), String> {
    if prefix_path.join("drive_c/windows/mono").is_dir() {
        return Ok(());
    }

    let msi_path = ensure_wine_mono_msi(app).await?;
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
fn log_stdio(log_path: &Path) -> Result<(Stdio, Stdio), String> {
    let out = fs::OpenOptions::new()
        .append(true)
        .open(log_path)
        .map_err(|e| format!("Could not open log file: {e}"))?;
    let err = out
        .try_clone()
        .map_err(|e| format!("Could not open log file: {e}"))?;
    Ok((Stdio::from(out), Stdio::from(err)))
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
    game: &Game,
    id: &str,
    log_path: &Path,
    env: &mut Vec<(String, String)>,
) -> Result<PathBuf, String> {
    let prefix_path_str = game.prefix_path.to_string_lossy();
    let is_set_up =
        || game.prefix_path.join("drive_c").is_dir() && runtime_present(&runner.path);
    if !is_set_up() {
        let _ = app.emit("game-initializing", GameInitializingPayload { id });
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

/// Readies a Wine runner's launch and returns its `wine` binary, which the
/// game is launched with directly. Unlike Proton, a plain Wine build does
/// none of its own prefix setup, so that happens here: a fresh prefix gets
/// initialized with wineboot, wine-mono installed, and DXVK/VKD3D-Proton
/// linked in (see `sync_directx_overrides_from_cache`), with the
/// `WINEDLLOVERRIDES` entries those need appended to `dll_overrides`.
async fn prepare_wine(
    app: &AppHandle,
    runner: &Runner,
    game: &Game,
    id: &str,
    log_path: &Path,
    dll_overrides: &mut Vec<String>,
) -> Result<PathBuf, String> {
    let wine = wine_binary(&runner.path)?;
    let prefix_path_str = game.prefix_path.to_string_lossy();

    // Must run before wineboot's first initialization of this prefix (see
    // `steer_profile_to_steamuser`), but is otherwise idempotent, so it's
    // simplest to just always ensure it — cheap, and self-healing if
    // something ever removed the symlink.
    steer_profile_to_steamuser(&game.prefix_path)?;
    // A prefix is created without ever running wineboot (see `add_prefix`),
    // so an empty one — no `drive_c` yet — is initialized here on first use.
    if !game.prefix_path.join("drive_c").is_dir() {
        let _ = app.emit("game-initializing", GameInitializingPayload { id });
        let (out, err) = log_stdio(log_path)?;
        let status = runner_command(&wine, [("WINEPREFIX", prefix_path_str.as_ref())])
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

    install_wine_mono(app, &wine, &game.prefix_path, log_path).await?;
    let cache = ensure_directx_layer_cache(app).await?;
    dll_overrides.push(sync_directx_overrides_from_cache(&cache, &game.prefix_path)?);

    // Proton only re-copies its own DXVK/VKD3D (and builtin DLLs) into a
    // prefix when its `config_info` marker says the prefix was last set up
    // differently. The files were just replaced here, so drop that marker —
    // otherwise switching this prefix back to the same Proton runner later
    // would keep running on this upstream DXVK instead of Proton's own.
    let config_info = game.prefix_path.join("config_info");
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
    id: String,
) -> Result<(), String> {
    let log_path = log_file_path(&app, &id)?;
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Could not create log directory: {e}"))?;
    }
    fs::File::create(&log_path).map_err(|e| format!("Could not create log file: {e}"))?;

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
    result
}

async fn run_game(
    app: &AppHandle,
    state: &State<'_, ConfigState>,
    running: &State<'_, RunningGames>,
    id: &str,
    log_path: &Path,
) -> Result<(), String> {
    let game_id = Uuid::parse_str(id).map_err(|e| format!("Invalid game id: {e}"))?;

    let (game, runners_dir, mangohud, performance) = {
        let config = state
            .lock()
            .map_err(|_| "Configuration is locked".to_string())?;
        let game = config
            .games
            .iter()
            .find(|g| g.id == game_id)
            .cloned()
            .ok_or_else(|| format!("No game with id {id}"))?;
        (
            game,
            config.runners_dir.clone(),
            config.mangohud.clone(),
            config.performance.clone(),
        )
    };
    let token = read_token(state)?;

    let runner = find_runner(&runners_dir, &game.runner_id)?;
    let prefix_path_str = game.prefix_path.to_str().ok_or_else(|| {
        format!(
            "Prefix path is not valid UTF-8: {}",
            game.prefix_path.display()
        )
    })?;
    let log_path_string = log_path.display().to_string();

    // `kill_running_game` needs this exact value to reach the right
    // wineserver session.
    let wineprefix = prefix_path_str.to_string();
    let wineserver = wineserver_binary(&runner.path)?;

    let mut env = vec![("WINEPREFIX".to_string(), prefix_path_str.to_string())];
    // `winemenubuilder.exe=` is disabled unconditionally: wine's default
    // behavior of registering .desktop entries and file associations for
    // whatever the game installs is never wanted here, since this app is
    // itself the game's launcher/menu. Proton appends its own overrides to
    // this rather than replacing it.
    let mut dll_overrides = vec!["winemenubuilder.exe=".to_string()];

    let runner_binary = match runner.kind {
        RunnerKind::Proton => {
            prepare_proton(app, token.as_deref(), &runner, &game, id, log_path, &mut env).await?
        }
        RunnerKind::Wine => {
            prepare_wine(app, &runner, &game, id, log_path, &mut dll_overrides).await?
        }
    };
    env.push(("WINEDLLOVERRIDES".to_string(), dll_overrides.join(";")));

    if mangohud.enabled {
        let conf_path = ensure_mangohud_conf(app, &mangohud)?;
        env.push(("MANGOHUD".to_string(), "1".to_string()));
        env.push((
            "MANGOHUD_CONFIGFILE".to_string(),
            conf_path.display().to_string(),
        ));
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
    if performance.vkbasalt_enabled {
        let vkbasalt_conf = ensure_vkbasalt_conf(app, &performance)?;
        env.push(("ENABLE_VKBASALT".to_string(), "1".to_string()));
        env.push((
            "VKBASALT_CONFIG_FILE".to_string(),
            vkbasalt_conf.display().to_string(),
        ));
    }
    env.extend(game.env_vars.iter().map(|(k, v)| (k.clone(), v.clone())));

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
    // and everything else (wine, the other wrappers) runs inside it. Skipped
    // if we're already inside a gamescope session ourselves — nesting it
    // again is pointless and often broken.
    let use_gamescope = performance.gamescope_enabled
        && command_on_path("gamescope")
        && std::env::var_os("GAMESCOPE_WAYLAND_DISPLAY").is_none();
    if use_gamescope {
        let mut wrapped = vec!["gamescope".to_string()];
        if let Some(width) = performance.gamescope_width {
            wrapped.push("-w".to_string());
            wrapped.push(width.to_string());
        }
        if let Some(height) = performance.gamescope_height {
            wrapped.push("-h".to_string());
            wrapped.push(height.to_string());
        }
        if let Some(fps) = performance.gamescope_fps_limit {
            wrapped.push("-r".to_string());
            wrapped.push(fps.to_string());
        }
        if performance.gamescope_fullscreen {
            wrapped.push("-f".to_string());
        }
        wrapped.push("--".to_string());
        wrapped.append(&mut launch_chain);
        launch_chain = wrapped;
    }

    let launch_binary = PathBuf::from(&launch_chain[0]);
    let wrapper_args = launch_chain[1..].to_vec();
    let launch_env = env;

    // Per-game exe arguments (PortProton calls these `LAUNCH_PARAMETERS`),
    // e.g. `--launcher-skip -dx11`. Whitespace-split; no quoting support.
    let game_launch_args: Vec<String> = game
        .launch_args
        .split_whitespace()
        .map(str::to_string)
        .collect();

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

    let env_pairs: Vec<(&str, &str)> = launch_env
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();

    let mut child = runner_command(&launch_binary, env_pairs)
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

    if let Ok(mut running) = running.0.lock() {
        running.insert(
            game_id,
            RunningGame {
                name: game.name.clone(),
                pid: child.id(),
                wineserver: wineserver.clone(),
                wineprefix: wineprefix.clone(),
                exe_path: game.exe_path.clone(),
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

    if !status.success() {
        return Err(format!("Game exited with status {status}"));
    }

    Ok(())
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
fn write_shortcut_icon(app: &AppHandle, game: &Game) -> Result<Option<PathBuf>, String> {
    if let Some(icon_url) = &game.steamgriddb_icon_url {
        let ext = image_extension(icon_url);
        let path = asset_cache_path(&artwork_dir(app)?, game.id, "_icon", ext);
        if path.exists() {
            return Ok(Some(path));
        }
    }
    write_exe_icon_file(app, game)
}

/// Decodes a game's `icon` data URI (the exe's embedded icon) to a cached
/// PNG file, since a `.desktop` entry's `Icon=` needs a real file path, not
/// inline image data.
fn write_exe_icon_file(app: &AppHandle, game: &Game) -> Result<Option<PathBuf>, String> {
    let Some(data_url) = &game.icon else {
        return Ok(None);
    };
    let payload = data_url
        .split_once(',')
        .map(|(_, payload)| payload)
        .ok_or_else(|| "Icon data is not a valid data URI".to_string())?;
    let bytes = STANDARD
        .decode(payload)
        .map_err(|e| format!("Could not decode icon data: {e}"))?;

    let icons_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Could not resolve data directory: {e}"))?
        .join("shortcut-icons");
    fs::create_dir_all(&icons_dir)
        .map_err(|e| format!("Could not create shortcut icons directory: {e}"))?;

    let icon_path = icons_dir.join(format!("{}.png", game.id));
    fs::write(&icon_path, bytes).map_err(|e| format!("Could not write icon file: {e}"))?;
    Ok(Some(icon_path))
}

/// Resolves the path a `.desktop` shortcut should point to. When running from
/// an AppImage, `current_exe()` returns a path inside a temporary FUSE mount
/// (`/tmp/.mount_XXXXXX/...`) that's torn down when the process exits and
/// re-randomized on every launch — useless for a persistent shortcut.
/// AppImages set `APPIMAGE` to the real `.AppImage` file's path exactly for
/// cases like this, so that's preferred when present.
fn own_executable_path() -> Result<PathBuf, String> {
    if let Some(appimage_path) = std::env::var_os("APPIMAGE") {
        return Ok(PathBuf::from(appimage_path));
    }
    std::env::current_exe().map_err(|e| format!("Could not resolve own executable path: {e}"))
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
    contents.push_str(&format!("Name={}\n", game.name));
    contents.push_str(&format!(
        "Exec=\"{}\" --launch {}\n",
        exe_path.display(),
        game.id
    ));
    if let Some(icon_path) = &icon_path {
        contents.push_str(&format!("Icon={}\n", icon_path.display()));
    }
    contents.push_str("Terminal=false\n");
    contents.push_str("Categories=Game;\n");

    let shortcut_path = target_dir.join(format!("{}.desktop", sanitize_filename(&game.name)));
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
#[tauri::command]
pub fn create_desktop_shortcut(
    app: AppHandle,
    state: State<ConfigState>,
    id: String,
) -> Result<(), String> {
    let desktop_dir = desktop_directory()?;
    write_game_shortcut(&app, &state, &id, &desktop_dir)
}

/// Creates a `.desktop` entry in the user's XDG applications directory so
/// this game shows up in the desktop environment's start menu / app
/// launcher, mirroring how PortProton offers "Add to Menu" as a choice
/// separate from "Add to Desktop" rather than doing both at once.
/// `update-desktop-database` is nudged afterwards, best-effort, so menus
/// that cache entries (like KDE's) pick up the addition immediately instead
/// of waiting for their own refresh.
#[tauri::command]
pub fn create_menu_shortcut(
    app: AppHandle,
    state: State<ConfigState>,
    id: String,
) -> Result<(), String> {
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
    contents.push_str(&format!("Exec=\"{}\" --install %f\n", exe_path.display()));
    contents.push_str(&format!("Icon={}\n", icon_path.display()));
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
mod tests {
    use super::*;

    #[test]
    fn pkill_pattern_matches_paths_literally() {
        assert_eq!(
            pkill_pattern("/drive_c/Program Files (x86)/Game [v1.2]/game+.exe"),
            r"[/]drive_c/Program Files \(x86\)/Game \[v1\.2\]/game\+\.exe"
        );
        assert_eq!(pkill_pattern("umu-run /g.exe"), r"[u]mu-run /g\.exe");
        assert_eq!(pkill_pattern("^x"), r"\^x");
    }

    #[test]
    fn pkill_pattern_matches_the_process_but_not_itself() {
        // GNU sleep sums its arguments; the random fraction makes this
        // command line unique on the system.
        let fraction = format!("0.{}", Uuid::new_v4().as_u128() % 1_000_000_000);
        let target = format!("sleep 30 {fraction}");
        let mut child = std::process::Command::new("sleep")
            .args(["30", &fraction])
            .spawn()
            .unwrap();
        let pattern = pkill_pattern(&target);

        // `pgrep -f` matches exactly like `pkill -f`, without killing.
        let found = std::process::Command::new("pgrep")
            .args(["-f", &pattern])
            .output()
            .unwrap();
        let pids = String::from_utf8_lossy(&found.stdout);
        assert_eq!(pids.trim(), child.id().to_string());
        assert!(!regex_self_match(&pattern));

        child.kill().unwrap();
        child.wait().unwrap();
    }

    /// Whether `pattern` would match a command line containing `pattern`
    /// itself — checked with `grep -E`, which uses the same regex flavor.
    fn regex_self_match(pattern: &str) -> bool {
        use std::io::Write;
        let mut grep = std::process::Command::new("grep")
            .args(["-qE", pattern])
            .stdin(Stdio::piped())
            .spawn()
            .unwrap();
        grep.stdin.take().unwrap().write_all(pattern.as_bytes()).unwrap();
        grep.wait().unwrap().success()
    }

    #[test]
    fn process_tree_helpers_see_real_processes() {
        let mut child = std::process::Command::new("sh")
            .args(["-c", "sleep 30 & wait"])
            .spawn()
            .unwrap();
        let pid = child.id();
        // Give `sh` a moment to fork its `sleep`.
        std::thread::sleep(Duration::from_millis(200));

        assert!(process_running(pid));
        let descendants = process_descendants(pid);
        assert_eq!(descendants.len(), 1);

        tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap()
            .block_on(kill_process_tree(pid));
        child.wait().unwrap();

        assert!(!process_running(pid));
        // SIGKILL is delivered asynchronously, so the grandchild can take a
        // moment to actually disappear.
        let deadline = Instant::now() + Duration::from_secs(2);
        while process_running(descendants[0]) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(!process_running(descendants[0]));
    }
}
