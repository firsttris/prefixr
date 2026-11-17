//! Laufzeit was called Prefixr up to 0.2. This carries an install of it
//! over on the first start under the new name, before anything reads the
//! config: the library and settings, the downloads (runners, the prefixes of
//! store games, artwork) and the caches move to the new folders, and the
//! paths in the config that pointed into the old ones are changed to match.
//!
//! Desktop and menu shortcuts, Steam's entries and the agent skill, which
//! call the old program, are rewritten later on the start (see
//! `commands::games::relink_legacy_shortcuts`,
//! `commands::steam::relink_legacy_shortcuts`, `migrate_skill`).

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::env::Env;

/// The Tauri identifier, which names the config, data and cache folders.
/// Checked against `tauri.conf.json` in the tests.
pub(crate) const IDENTIFIER: &str = "com.tristan.laufzeit";
/// Prefixr's Tauri identifier.
pub(crate) const LEGACY_IDENTIFIER: &str = "com.tristan.prefixr";
/// Prefixr's Flatpak app ID. The Flatpak's `.desktop` file names it in
/// `X-Flatpak-RenamedFrom`, so Flatpak moves `~/.var/app/<it>` over.
pub(crate) const LEGACY_FLATPAK_ID: &str = "io.github.firsttris.Prefixr";

/// Whether something was written by Prefixr, from a command line or a path:
/// its program (`/usr/bin/prefixr`, `prefixr_0.2.2_amd64.AppImage`) or its
/// Flatpak (`flatpak run io.github.firsttris.Prefixr`).
pub(crate) fn mentions_prefixr(text: &str) -> bool {
    text.to_ascii_lowercase().contains("prefixr")
}

/// The XDG base directories the config, data and cache folders are in, the
/// way Tauri finds them: `$XDG_CONFIG_HOME` etc. when set to an absolute
/// path, otherwise below `$HOME`. In a Flatpak these point into
/// `~/.var/app/<app ID>`.
struct BaseDirs {
    config: PathBuf,
    data: PathBuf,
    cache: PathBuf,
}

impl BaseDirs {
    fn from(env: Env) -> Option<Self> {
        let home = env("HOME")
            .filter(|home| !home.is_empty())
            .map(PathBuf::from)?;
        let dir = |key: &str, fallback: &str| {
            env(key)
                .map(PathBuf::from)
                .filter(|dir| dir.is_absolute())
                .unwrap_or_else(|| home.join(fallback))
        };
        Some(Self {
            config: dir("XDG_CONFIG_HOME", ".config"),
            data: dir("XDG_DATA_HOME", ".local/share"),
            cache: dir("XDG_CACHE_HOME", ".cache"),
        })
    }
}

/// The old path and the new one, for every folder that moved. A path the
/// config holds is changed by the first pair it starts with. In a Flatpak,
/// `~/.var/app/<Prefixr's ID>` comes first: Flatpak moved that folder, and
/// the rest of the path is then changed by the pairs after it.
pub(crate) fn moved_paths(env: Env) -> Vec<(PathBuf, PathBuf)> {
    let Some(dirs) = BaseDirs::from(env) else {
        return Vec::new();
    };
    let mut pairs = Vec::new();
    let flatpak_id = env("FLATPAK_ID").filter(|id| !id.is_empty());
    if let (Some(id), Some(home)) = (flatpak_id, env("HOME")) {
        let apps = PathBuf::from(home).join(".var/app");
        pairs.push((apps.join(LEGACY_FLATPAK_ID), apps.join(id)));
    }
    for base in [dirs.config, dirs.data, dirs.cache] {
        pairs.push((base.join(LEGACY_IDENTIFIER), base.join(IDENTIFIER)));
    }
    pairs
}

/// `path` with the first of `pairs` it's in changed to the new folder, then
/// the next pair applied to the result, and so on; `None` if none applies.
fn moved(path: &str, pairs: &[(PathBuf, PathBuf)]) -> Option<String> {
    let mut path = PathBuf::from(path);
    let mut changed = false;
    for (old, new) in pairs {
        if let Ok(rest) = path.strip_prefix(old) {
            path = if rest.as_os_str().is_empty() {
                new.clone()
            } else {
                new.join(rest)
            };
            changed = true;
        }
    }
    changed.then(|| path.to_string_lossy().into_owned())
}

/// Changes every string in `value` that is a path into one of the moved
/// folders. Returns whether anything changed.
pub(crate) fn rewrite_paths(value: &mut Value, pairs: &[(PathBuf, PathBuf)]) -> bool {
    match value {
        Value::String(text) => match moved(text, pairs) {
            Some(new) => {
                *text = new;
                true
            }
            None => false,
        },
        // Every item, not only up to the first one that changed.
        Value::Array(items) => {
            let mut changed = false;
            for item in items {
                changed |= rewrite_paths(item, pairs);
            }
            changed
        }
        Value::Object(fields) => {
            let mut changed = false;
            for item in fields.values_mut() {
                changed |= rewrite_paths(item, pairs);
            }
            changed
        }
        _ => false,
    }
}

/// A folder, not a symlink to one: the data folder leaves a symlink behind.
fn is_real_dir(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|meta| meta.is_dir())
}

/// Moves Prefixr's folders to Laufzeit's. A folder that exists under the new
/// name already is left as it is, so this runs once. Best-effort: a step
/// that fails is reported on stderr and the start goes on.
pub fn migrate_from_prefixr(env: Env) {
    if let Err(error) = migrate(env) {
        eprintln!("Moving Prefixr's files to Laufzeit's folders failed: {error}");
    }
}

pub(crate) fn migrate(env: Env) -> Result<(), String> {
    let Some(dirs) = BaseDirs::from(env) else {
        return Ok(());
    };
    let pending = |base: &Path| {
        let (old, new) = (base.join(LEGACY_IDENTIFIER), base.join(IDENTIFIER));
        (is_real_dir(&old) && !new.exists()).then_some((old, new))
    };
    let config = pending(&dirs.config);
    let data = pending(&dirs.data);
    let cache = pending(&dirs.cache);

    // The config first, while it's still in the old folder: another
    // Laufzeit starting at the same time sees the new folder only once it
    // holds the new paths. Its paths also follow a data folder moved
    // before; the symlink left there would cover them anyway.
    if let Some((old_config, _)) = &config {
        rewrite_config_file(&old_config.join("config.json"), &moved_paths(env))?;
    }
    if let Some((old, new)) = cache {
        rename(&old, &new)?;
    }
    if let Some((old, new)) = data {
        rename(&old, &new)?;
        // What else points into the data folder keeps working: a prefix's
        // own files, the store tools' install lists, a `.desktop` file's
        // icon, a prefix another launcher shares with Laufzeit.
        #[cfg(unix)]
        if let Err(e) = std::os::unix::fs::symlink(IDENTIFIER, &old) {
            eprintln!("Could not link {} to {IDENTIFIER}: {e}", old.display());
        }
    }
    if let Some((old, new)) = config {
        rename(&old, &new)?;
    }
    Ok(())
}

fn rename(old: &Path, new: &Path) -> Result<(), String> {
    match fs::rename(old, new) {
        Ok(()) => Ok(()),
        // Another Laufzeit starting at the same time moved it first.
        Err(_) if new.exists() && !old.exists() => Ok(()),
        Err(e) => Err(format!(
            "Could not move {} to {}: {e}",
            old.display(),
            new.display()
        )),
    }
}

/// Rewrites the moved paths in a config file, atomically; a missing file or
/// one that isn't JSON is left alone (the start then reports it as usual).
fn rewrite_config_file(path: &Path, pairs: &[(PathBuf, PathBuf)]) -> Result<(), String> {
    let Ok(raw) = fs::read(path) else {
        return Ok(());
    };
    let Ok(mut value) = serde_json::from_slice::<Value>(&raw) else {
        return Ok(());
    };
    if !rewrite_paths(&mut value, pairs) {
        return Ok(());
    }
    let text = serde_json::to_string_pretty(&value)
        .map_err(|e| format!("Could not serialize config: {e}"))?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, text).map_err(|e| format!("Could not write {}: {e}", tmp.display()))?;
    fs::rename(&tmp, path).map_err(|e| format!("Could not write {}: {e}", path.display()))
}

/// Replaces Prefixr's agent skill (`~/.claude/skills/prefixr`), which tells
/// the agent to run `prefixr`, with Laufzeit's. Only a folder holding just
/// the skill file Prefixr wrote is removed.
pub fn migrate_skill(env: Env) {
    let Some(home) = env("HOME").filter(|home| !home.is_empty()) else {
        return;
    };
    let skills = PathBuf::from(home).join(".claude/skills");
    let old = skills.join("prefixr");
    let old_file = old.join("SKILL.md");
    let Ok(text) = fs::read_to_string(&old_file) else {
        return;
    };
    if !text.contains("\n# Prefixr\n") {
        return;
    }
    let new = skills.join("laufzeit");
    if !new.join("SKILL.md").exists() {
        if let Err(error) = crate::cli::install_skill(&new) {
            eprintln!("{error}");
            return;
        }
    }
    let only_the_skill = fs::read_dir(&old).is_ok_and(|entries| entries.count() == 1);
    if only_the_skill {
        let _ = fs::remove_file(&old_file);
        let _ = fs::remove_dir(&old);
    }
}

#[cfg(test)]
mod tests;
