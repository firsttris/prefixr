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

use std::ffi::{OsStr, OsString};
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::sync::OnceLock;

/// A change to a child's environment: the variable's new value, or `None`
/// to remove it.
pub type EnvFix = (OsString, Option<OsString>);

/// The AppImage runtime's own variables, which describe the AppImage rather
/// than point into it, and mean nothing to another program.
const APPIMAGE_VARS: &[&str] = &["APPDIR", "APPIMAGE", "ARGV0", "OWD"];

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

/// `appimage_fixes` for this process's own environment, worked out once.
fn process_fixes() -> &'static [EnvFix] {
    static FIXES: OnceLock<Vec<EnvFix>> = OnceLock::new();
    FIXES.get_or_init(|| appimage_fixes(std::env::vars_os()))
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
