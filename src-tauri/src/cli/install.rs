//! `prefixr game install`: runs a game's installer in a prefix and adds the
//! game it installed, in one step, so an agent can set a game up from a
//! setup file or a disc image the way the window's **Run installer** does.
//! A disc image is mounted with udisks for the install and unmounted after.
//! The installer's window shows on the desktop as usual: one that can't be
//! told to run silently waits for someone (or a computer-use tool) to click
//! it through.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use clap::Args;
use futures_util::future::Either;
use serde::Serialize;
use tauri::{AppHandle, Manager, State};

use super::store::{runner_or_default, stop_signal};
use super::{
    apply_settings, check_exe, config, ensure_prefix, name_from_exe, short_id, GameSettings, Out,
};
use crate::commands::games::{add_game, run_in_prefix};
use crate::commands::runners::{find_runner, runner_command, scan_runners, wineserver_binary};
use crate::commands::shell_link::{find_recently_created_shortcuts, DetectedShortcut};
use crate::config::{load_config, lock_config, ConfigState};
use crate::error::AppError;
use crate::host_env::host_std_command;
use crate::lock::LockExt;
use crate::models::{Game, GameInput};

#[derive(Args, Debug)]
pub(super) struct InstallGame {
    /// The installer: a setup .exe or .msi, or a disc image (.iso) whose
    /// installer is run
    installer: PathBuf,
    /// The prefix to install into; one Prefixr doesn't know yet is added (and created)
    #[arg(long)]
    prefix: PathBuf,
    /// The runner's id; the newest Proton otherwise
    #[arg(long)]
    runner: Option<String>,
    /// The name in the library; the name of the installer's shortcut otherwise
    #[arg(long)]
    name: Option<String>,
    /// The game's exe, for an installer that creates no shortcut or several
    #[arg(long)]
    exe: Option<PathBuf>,
    /// On a disc image: the installer's path inside it, if it isn't the one
    /// autorun.inf names or setup.exe
    #[arg(long)]
    setup: Option<PathBuf>,
    #[command(flatten)]
    settings: GameSettings,
    /// Arguments for the installer, after `--`: e.g. `-- /VERYSILENT
    /// /SUPPRESSMSGBOXES` for an Inno Setup installer (GOG's), `-- /S` for NSIS
    #[arg(last = true, allow_hyphen_values = true)]
    installer_args: Vec<String>,
}

#[derive(Serialize)]
struct InstallResult {
    game: Game,
    installer_exit_code: Option<i32>,
    log_path: PathBuf,
}

pub(super) async fn install(app: &AppHandle, out: Out, args: InstallGame) -> Result<(), AppError> {
    let runner_id = runner_or_default(&scan_runners(&config(app).runners_dir)?, args.runner)?;
    let installer = check_exe(&args.installer)?;
    let prefix = under_config_lock(app, || ensure_prefix(app, out, &args.prefix))?;

    let disc = if is_disc_image(&installer) {
        let disc = Disc::mount(&installer)?;
        out.note(&format!(
            "Mounted {} at {}",
            installer.display(),
            disc.mount_point.display()
        ));
        Some(disc)
    } else {
        None
    };
    let program = match &disc {
        Some(disc) => disc_installer(&disc.mount_point, args.setup.as_deref())?,
        None => installer,
    };

    out.note(&format!(
        "Running {} with {runner_id}; if it opens a window, it waits until that is clicked through…",
        program.display()
    ));
    let started = SystemTime::now();
    let state: State<ConfigState> = app.state();
    let install = Box::pin(async {
        let ran = run_in_prefix(
            app,
            &state,
            &prefix.display().to_string(),
            &runner_id,
            &program.display().to_string(),
            &args.installer_args,
        )
        .await?;
        // A setup.exe often only starts the real installer and exits: the
        // prefix's wineserver runs until everything in it has ended.
        wineserver(app, &runner_id, &ran.prefix, "-w").await;
        Ok::<_, AppError>(ran)
    });
    let ran = match futures_util::future::select(install, Box::pin(stop_signal())).await {
        Either::Left((ran, _)) => ran?,
        Either::Right(_) => {
            // Ended here rather than by the signal, so that what still runs
            // in the prefix lets go of a disc image, which is unmounted as
            // `disc` goes.
            wineserver(app, &runner_id, &prefix, "-k").await;
            return Err("Stopped: the installer was ended".to_string().into());
        }
    };
    drop(disc);

    let shortcuts = find_recently_created_shortcuts(&ran.prefix, started);
    let explain = || {
        format!(
            "The installer exited with {}. Its log: {}",
            ran.status,
            ran.log_path.display()
        )
    };
    let picked = match &args.exe {
        Some(exe) => {
            let exe = check_exe(exe)?;
            let name = shortcuts
                .iter()
                .find(|s| Path::new(&s.exe_path) == exe.as_path())
                .map(|s| s.name.clone());
            (exe, name)
        }
        None => pick_shortcut(&shortcuts, &ran.prefix, &runner_id)
            .map_err(|message| format!("{message}\n{}", explain()))?,
    };
    let (exe, shortcut_name) = picked;

    let mut input = GameInput {
        name: args
            .name
            .or(shortcut_name)
            .unwrap_or_else(|| name_from_exe(&exe)),
        exe_path: exe,
        prefix_path: prefix,
        runner_id,
        env_vars: Default::default(),
        launch_args: String::new(),
        notes: String::new(),
        umu_id: None,
        umu_store: None,
        overrides: Default::default(),
    };
    apply_settings(args.settings, &mut input);
    let game = under_config_lock(app, || add_game(app.clone(), app.state(), input))?;
    let result = InstallResult {
        game,
        installer_exit_code: ran.status.code(),
        log_path: ran.log_path,
    };
    out.print(&result, || {
        format!(
            "Installed and added \"{}\" ({}): {}",
            result.game.name,
            short_id(&result.game),
            result.game.exe_path.display()
        )
    })
}

/// Runs `change` on the config as it is on disk, holding the config lock.
/// `game install` doesn't hold it from its start (see `changes_config`):
/// an installer can run for an hour, and the window would wait for the lock
/// that long.
fn under_config_lock<T>(
    app: &AppHandle,
    change: impl FnOnce() -> Result<T, AppError>,
) -> Result<T, AppError> {
    let _lock = tokio::task::block_in_place(|| lock_config(app))?;
    *app.state::<ConfigState>().locked() = load_config(app)?;
    change()
}

/// Runs the prefix's wineserver with `flag`: `-w` waits until nothing runs
/// in the prefix anymore, `-k` ends what does. Not being able to isn't an
/// error: after `-w`, the shortcuts found are what there is so far.
async fn wineserver(app: &AppHandle, runner_id: &str, prefix: &Path, flag: &str) {
    let Ok(runner) = find_runner(&config(app).runners_dir, runner_id) else {
        return;
    };
    let Ok(wineserver) = wineserver_binary(&runner.path) else {
        return;
    };
    let prefix = prefix.display().to_string();
    let _ = runner_command(&wineserver, [("WINEPREFIX", prefix.as_str())])
        .arg(flag)
        .status()
        .await;
}

/// The game's exe among the shortcuts the installer created, and the
/// shortcut's name.
fn pick_shortcut(
    shortcuts: &[DetectedShortcut],
    prefix: &Path,
    runner_id: &str,
) -> Result<(PathBuf, Option<String>), String> {
    let add_hint = format!(
        "Add the game with `prefixr game add --exe <exe> --prefix {} --runner {runner_id}`.",
        prefix.display()
    );
    match shortcuts {
        [shortcut] => Ok((
            PathBuf::from(&shortcut.exe_path),
            Some(shortcut.name.clone()),
        )),
        [] => Err(format!(
            "The installer created no shortcut, so the game's exe is unknown. Look for it under {}. {add_hint}",
            prefix.join("drive_c").display()
        )),
        several => Err(format!(
            "The installer created several shortcuts:\n{}\n{add_hint}",
            several
                .iter()
                .map(|s| format!("  {}: {}", s.name, s.exe_path))
                .collect::<Vec<_>>()
                .join("\n")
        )),
    }
}

fn is_disc_image(path: &Path) -> bool {
    path.extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("iso"))
}

/// A disc image mounted read-only through udisks, which needs no root. It's
/// unmounted and its loop device removed again when dropped.
struct Disc {
    loop_device: String,
    mount_point: PathBuf,
}

impl Disc {
    fn mount(image: &Path) -> Result<Disc, String> {
        let output = udisksctl(&[
            "loop-setup",
            "--read-only",
            "--file",
            &image.to_string_lossy(),
        ])?;
        let loop_device = parse_loop_device(&output)
            .ok_or_else(|| format!("udisksctl didn't name a loop device: {output}"))?;
        let mut disc = Disc {
            loop_device,
            mount_point: PathBuf::new(),
        };
        // A desktop may have mounted it on its own already, which makes
        // this fail; where it's mounted is read from the mount table.
        let mounted = udisksctl(&["mount", "--block-device", &disc.loop_device]);
        let mounts = fs::read_to_string("/proc/self/mounts").unwrap_or_default();
        disc.mount_point = mount_point_of(&mounts, &disc.loop_device).ok_or_else(|| {
            format!(
                "Could not mount {}: {}",
                image.display(),
                mounted.err().unwrap_or_default()
            )
        })?;
        Ok(disc)
    }
}

impl Drop for Disc {
    fn drop(&mut self) {
        let _ = udisksctl(&["unmount", "--block-device", &self.loop_device]);
        let _ = udisksctl(&["loop-delete", "--block-device", &self.loop_device]);
    }
}

/// Runs `udisksctl` without asking for a password, returning its output.
/// With the host's environment (see `host_env`): under the AppImage's
/// `LD_LIBRARY_PATH` it loads the bundled GLib, older than the system's
/// libudisks2 needs, and dies with "undefined symbol".
fn udisksctl(args: &[&str]) -> Result<String, String> {
    let output = host_std_command("udisksctl")
        .args(args)
        .arg("--no-user-interaction")
        .output()
        .map_err(|e| {
            format!(
                "Mounting a disc image needs udisksctl (udisks2): {e}. Without it, extract the \
                 image instead (e.g. `7z x game.iso -ogame`) and pass the installer in it."
            )
        })?;
    let text = |bytes: &[u8]| String::from_utf8_lossy(bytes).trim().to_string();
    if output.status.success() {
        Ok(text(&output.stdout))
    } else {
        Err(text(&output.stderr))
    }
}

/// The device in udisksctl's "Mapped file game.iso as /dev/loop0."
fn parse_loop_device(output: &str) -> Option<String> {
    let (_, device) = output.rsplit_once(" as ")?;
    let device = device.trim().trim_end_matches('.');
    device.starts_with("/dev/").then(|| device.to_string())
}

/// Where `device` is mounted, from a mount table such as /proc/self/mounts,
/// in which a space in a path is written `\040`.
fn mount_point_of(mounts: &str, device: &str) -> Option<PathBuf> {
    mounts.lines().find_map(|line| {
        let mut fields = line.split(' ');
        (fields.next()? == device).then_some(())?;
        let point = fields.next()?;
        Some(PathBuf::from(unescape_mount_field(point)))
    })
}

fn unescape_mount_field(field: &str) -> String {
    let mut out = String::new();
    let mut rest = field;
    while let Some(at) = rest.find('\\') {
        out.push_str(&rest[..at]);
        let code = rest.get(at + 1..at + 4);
        match code.and_then(|code| u8::from_str_radix(code, 8).ok()) {
            Some(byte) => {
                out.push(byte as char);
                rest = &rest[at + 4..];
            }
            None => {
                out.push('\\');
                rest = &rest[at + 1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// The installer to run on a mounted disc: `setup` if given, else the
/// program autorun.inf starts, else a setup.exe or install.exe at the top.
/// A disc's file names may be in any case.
fn disc_installer(root: &Path, setup: Option<&Path>) -> Result<PathBuf, String> {
    if let Some(setup) = setup {
        return find_ignoring_case(root, &setup.to_string_lossy())
            .ok_or_else(|| format!("No {} on the disc", setup.display()));
    }
    let autorun = find_ignoring_case(root, "autorun.inf")
        .and_then(|path| fs::read(path).ok())
        .and_then(|bytes| autorun_program(&String::from_utf8_lossy(&bytes)))
        .and_then(|program| find_ignoring_case(root, &program));
    if let Some(program) = autorun {
        return Ok(program);
    }
    ["setup.exe", "install.exe"]
        .iter()
        .find_map(|name| find_ignoring_case(root, name))
        .ok_or_else(|| {
            let exes: Vec<String> = fs::read_dir(root)
                .into_iter()
                .flatten()
                .flatten()
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .filter(|name| name.to_lowercase().ends_with(".exe"))
                .collect();
            format!(
                "Found no installer on the disc; name it with --setup. Programs at its top: {}",
                if exes.is_empty() {
                    "none".to_string()
                } else {
                    exes.join(", ")
                }
            )
        })
}

/// The program an autorun.inf's `[autorun]` section starts with `open=` (or
/// `shellexecute=`), without its arguments, as a path with `/`.
fn autorun_program(inf: &str) -> Option<String> {
    let mut in_autorun = false;
    for line in inf.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_autorun = line.eq_ignore_ascii_case("[autorun]");
            continue;
        }
        if !in_autorun || line.starts_with(';') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if !(key.eq_ignore_ascii_case("open") || key.eq_ignore_ascii_case("shellexecute")) {
            continue;
        }
        let value = value.trim();
        let program = match value.strip_prefix('"') {
            Some(quoted) => quoted.split('"').next()?,
            None => value.split_whitespace().next()?,
        };
        let program = program.replace('\\', "/");
        let program = program.trim_start_matches("./").trim_start_matches('/');
        if !program.is_empty() {
            return Some(program.to_string());
        }
    }
    None
}

/// `relative` under `root`, each part of it matched ignoring case.
fn find_ignoring_case(root: &Path, relative: &str) -> Option<PathBuf> {
    let mut path = root.to_path_buf();
    for part in relative.split(['/', '\\']).filter(|p| !p.is_empty()) {
        let exact = path.join(part);
        path = if exact.exists() {
            exact
        } else {
            fs::read_dir(&path)
                .ok()?
                .flatten()
                .find(|entry| {
                    entry
                        .file_name()
                        .to_string_lossy()
                        .eq_ignore_ascii_case(part)
                })?
                .path()
        };
    }
    path.is_file().then_some(path)
}

#[cfg(test)]
mod tests;
