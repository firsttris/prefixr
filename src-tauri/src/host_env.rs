//! The environment of the programs Prefixr starts.
//!
//! Run as an AppImage, Prefixr itself starts with an environment its
//! `AppRun` set up for the libraries bundled in the AppImage: `PYTHONHOME`,
//! `PYTHONPATH`, `LD_LIBRARY_PATH`, `PERLLIB`, `GTK_PATH` and others, all
//! pointing into the AppImage's mount. Inherited as they are, they break what
//! Prefixr starts: umu-run's Python can't find its own standard library
//! ("No module named 'encodings'"), and Wine, Steam or winetricks load the
//! AppImage's libraries instead of the system's. So every program Prefixr
//! starts gets the environment without them, through `host_command`.
//!
//! The same goes for variables that only mean something on Windows: Wine
//! hands the whole environment on to the Windows programs it runs, where
//! they take effect (see `WINDOWS_ONLY_VARS`).

use std::ffi::{OsStr, OsString};
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::env::{self, Env};

/// Whether Prefixr runs as a Flatpak. Every Flatpak-specific branch goes
/// through this (or `is_flatpak_in`), so the native builds (AppImage, .deb,
/// .rpm) keep behaving exactly as before.
pub fn is_flatpak() -> bool {
    static FLATPAK: OnceLock<bool> = OnceLock::new();
    *FLATPAK.get_or_init(|| is_flatpak_in(&env::process) || Path::new("/.flatpak-info").exists())
}

/// `is_flatpak` from the environment alone: Flatpak sets `FLATPAK_ID` and
/// `container=flatpak` in every sandbox. umu checks the latter
/// (`umu_consts.py`), Heroic and Bottles the former.
pub fn is_flatpak_in(env: Env) -> bool {
    env("FLATPAK_ID").is_some_and(|id| !id.is_empty())
        || env("container").is_some_and(|value| value == "flatpak")
}

/// The Flatpak's app ID, e.g. `io.github.firsttris.Prefixr`; `None` outside
/// a Flatpak.
pub fn flatpak_id() -> Option<String> {
    env::process("FLATPAK_ID")
        .and_then(|id| id.into_string().ok())
        .filter(|id| !id.is_empty())
}

/// The folder Flatpak shares between an app's sandbox and the host, at the
/// same path in both: `$XDG_RUNTIME_DIR/app/<app ID>`. Files another program
/// on the host has to read go there, e.g. the tray icon (Heroic's
/// `heroic-run` points `TMPDIR` at it for the same reason). The sandbox's own
/// `$XDG_RUNTIME_DIR` is private. `None` outside a Flatpak.
pub fn flatpak_shared_runtime_dir(env: Env) -> Option<PathBuf> {
    let id = env("FLATPAK_ID").filter(|id| !id.is_empty())?;
    let runtime_dir = env("XDG_RUNTIME_DIR").filter(|dir| !dir.is_empty())?;
    Some(PathBuf::from(runtime_dir).join("app").join(id))
}

/// The host's data directory, which the other launchers, Steam and the
/// desktop share. Natively `$XDG_DATA_HOME`, falling back to
/// `~/.local/share`. In a Flatpak `$XDG_DATA_HOME` points into the app's own
/// `~/.var/app/<id>/data`, so it's `$HOST_XDG_DATA_HOME` there instead
/// (Flatpak passes on the host's value), falling back to `~/.local/share`
/// — the same choice umu makes (`umu_consts.py`).
pub fn host_data_home(env: Env) -> Option<PathBuf> {
    let key = if is_flatpak_in(env) {
        "HOST_XDG_DATA_HOME"
    } else {
        "XDG_DATA_HOME"
    };
    env(key)
        .filter(|dir| !dir.is_empty())
        .map(PathBuf::from)
        .or_else(|| env("HOME").map(|home| PathBuf::from(home).join(".local/share")))
}

/// The host's config directory: natively `$XDG_CONFIG_HOME`, in a Flatpak
/// `$HOST_XDG_CONFIG_HOME` (see `host_data_home`), falling back to
/// `~/.config`.
pub fn host_config_home(env: Env) -> Option<PathBuf> {
    let key = if is_flatpak_in(env) {
        "HOST_XDG_CONFIG_HOME"
    } else {
        "XDG_CONFIG_HOME"
    };
    env(key)
        .filter(|dir| !dir.is_empty())
        .map(PathBuf::from)
        .or_else(|| env("HOME").map(|home| PathBuf::from(home).join(".config")))
}

/// A change to a child's environment: the variable's new value, or `None`
/// to remove it.
pub type EnvFix = (OsString, Option<OsString>);

/// The AppImage runtime's own variables, which describe the AppImage rather
/// than point into it, and mean nothing to another program.
const APPIMAGE_VARS: &[&str] = &["APPDIR", "APPIMAGE", "ARGV0", "OWD"];

/// Variables a Windows program acts on that some Linux tools set in their own
/// environment. `NoDefaultCurrentDirectoryInExePath` (set by e.g. Claude
/// Code's shell) stops `CreateProcess` and `SearchPath` from looking in the
/// current directory, so a program that starts a helper exe sitting next to
/// it by its bare name can't find it anymore: installers that unpack with
/// external decompressors then fail right at the start.
const WINDOWS_ONLY_VARS: &[&str] = &["NoDefaultCurrentDirectoryInExePath"];

/// The changes that remove `WINDOWS_ONLY_VARS` from `vars`.
pub fn windows_fixes(vars: impl IntoIterator<Item = (OsString, OsString)>) -> Vec<EnvFix> {
    vars.into_iter()
        .filter(|(key, _)| WINDOWS_ONLY_VARS.iter().any(|name| key == *name))
        .map(|(key, _)| (key, None))
        .collect()
}

/// The changes that take an AppImage's setup back out of `vars`: every
/// entry of a variable that points into the AppImage's mount (`APPDIR`) is
/// dropped from it, a variable left without entries is removed, and so are
/// the runtime's own variables. Nothing without `APPDIR`, i.e. when Prefixr
/// doesn't run as an AppImage.
pub fn appimage_fixes(vars: impl IntoIterator<Item = (OsString, OsString)>) -> Vec<EnvFix> {
    let vars: Vec<(OsString, OsString)> = vars.into_iter().collect();
    let Some(appdir) = vars
        .iter()
        .find(|(key, _)| key == "APPDIR")
        .map(|(_, value)| trim_trailing_slashes(value.as_bytes()).to_vec())
        .filter(|appdir| !appdir.is_empty())
    else {
        return Vec::new();
    };

    let mut fixes = Vec::new();
    for (key, value) in vars {
        if APPIMAGE_VARS.iter().any(|name| key == *name) {
            fixes.push((key, None));
            continue;
        }
        let entries: Vec<&[u8]> = value.as_bytes().split(|&b| b == b':').collect();
        if !entries.iter().any(|entry| inside(entry, &appdir)) {
            continue;
        }
        let kept: Vec<&[u8]> = entries
            .into_iter()
            .filter(|entry| !inside(entry, &appdir))
            .collect();
        let fix = if kept.iter().all(|entry| entry.is_empty()) {
            None
        } else {
            Some(OsString::from_vec(kept.join(&b':')))
        };
        fixes.push((key, fix));
    }
    fixes
}

fn trim_trailing_slashes(path: &[u8]) -> &[u8] {
    let end = path.iter().rposition(|&b| b != b'/').map_or(0, |i| i + 1);
    &path[..end]
}

/// Whether `entry` is `appdir` or something below it.
fn inside(entry: &[u8], appdir: &[u8]) -> bool {
    entry.starts_with(appdir) && entry.get(appdir.len()).is_none_or(|&b| b == b'/')
}

/// `appimage_fixes` and `windows_fixes` for this process's own environment,
/// worked out once.
fn process_fixes() -> &'static [EnvFix] {
    static FIXES: OnceLock<Vec<EnvFix>> = OnceLock::new();
    FIXES.get_or_init(|| {
        let mut fixes = appimage_fixes(std::env::vars_os());
        fixes.extend(windows_fixes(std::env::vars_os()));
        fixes
    })
}

/// A command for `program` with the environment Prefixr itself was started
/// in (see the module's docs). Variables the caller sets on it afterwards
/// still win.
pub fn host_command(program: impl AsRef<OsStr>) -> tokio::process::Command {
    let mut command = tokio::process::Command::new(program);
    for (key, value) in process_fixes() {
        match value {
            Some(value) => command.env(key, value),
            None => command.env_remove(key),
        };
    }
    command
}

/// `host_command` for a command run synchronously.
pub fn host_std_command(program: impl AsRef<OsStr>) -> std::process::Command {
    let mut command = std::process::Command::new(program);
    for (key, value) in process_fixes() {
        match value {
            Some(value) => command.env(key, value),
            None => command.env_remove(key),
        };
    }
    command
}

/// Opens `target` (a file or a URL) with the desktop's default app for it.
/// `xdg-open` gets the host's environment: the opener plugin would start it
/// with the AppImage's, under which the desktop's own opener (e.g.
/// `kde-open`) can't even load its libraries.
pub async fn xdg_open(target: impl AsRef<OsStr>) -> Result<(), String> {
    let status = host_command("xdg-open")
        .arg(target)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await
        .map_err(|e| format!("Could not run xdg-open: {e}"))?;
    if !status.success() {
        return Err(format!("xdg-open exited with {status}"));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
