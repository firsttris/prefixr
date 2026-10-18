//! Backups of everything Prefixr keeps about the library, and restoring
//! one: `config.json` (the games, the prefix list, every setting, the
//! categories and play times), the chosen artwork and the exe icons, in one
//! `.tar.gz`. Not in it: the prefixes and the games' own files (far too big,
//! and they stay where they are), the store logins, and unless asked for,
//! the SteamGridDB API key and the GitHub token.
//!
//! Prefixr also backs up by itself into `backups/` under the data dir: once
//! a day when the window opens, and before every restore, so a restore (or
//! an agent's change) can always be undone. The newest `KEEP_AUTOMATIC` of
//! those are kept.

use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, SystemTime};

use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

use crate::commands::library::now_secs;
use crate::config::{load_config, lock_config, save_config, to_disk_json, AppConfig, ConfigState};
use crate::error::AppError;
use crate::lock::LockExt;

/// The archive layout's version; a backup from a newer Prefixr with a
/// higher one is refused rather than half understood.
const FORMAT: u32 = 1;
const MANIFEST: &str = "manifest.json";
const CONFIG: &str = "config.json";
/// The data dir's folders a backup holds, as they're named in it too.
const FOLDERS: [&str; 2] = ["artwork", "exe-icons"];
const KEEP_AUTOMATIC: usize = 7;
const DAILY: Duration = Duration::from_secs(24 * 60 * 60);

/// Why a backup was made.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "kebab-case")]
pub enum BackupReason {
    /// Made by the user, from the window or the command line.
    Manual,
    /// The daily one, when the window opens.
    Daily,
    /// Made right before a restore, to undo it.
    BeforeRestore,
}

/// What a backup says about itself, as `manifest.json` in it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Manifest {
    format: u32,
    app_version: String,
    created_at: u64,
    reason: BackupReason,
    games: usize,
    categories: usize,
    secrets_included: bool,
}

/// A backup, for the window's list and the command line.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct BackupInfo {
    pub path: String,
    pub file_name: String,
    /// Seconds since the Unix epoch.
    pub created_at: u64,
    pub reason: BackupReason,
    pub app_version: String,
    pub games: usize,
    pub categories: usize,
    /// Whether it holds the SteamGridDB API key and the GitHub token.
    pub secrets_included: bool,
    pub size_bytes: u64,
}

/// Where the files a backup covers live.
#[derive(Debug, Clone)]
pub struct Locations {
    pub data_dir: PathBuf,
    pub backups_dir: PathBuf,
}

impl Locations {
    pub fn of(app: &AppHandle) -> Result<Self, String> {
        let data_dir = app
            .path()
            .app_data_dir()
            .map_err(|e| format!("Could not resolve data directory: {e}"))?;
        Ok(Self {
            backups_dir: data_dir.join("backups"),
            data_dir,
        })
    }
}

/// The config as it goes into a backup: the secrets left out unless asked
/// for.
fn backup_config(config: &AppConfig, include_secrets: bool) -> AppConfig {
    let mut config = config.clone();
    if !include_secrets {
        config.steamgriddb.api_key = None;
        config.github.token = None;
    }
    config
}

/// A file name for a backup made now: `prefixr-backup-2026-10-18-0930.tar.gz`
/// for one of the user's, `daily-…`/`before-restore-…` for the automatic ones.
pub fn file_name(reason: BackupReason, at: SystemTime) -> String {
    let secs = at
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let (year, month, day, hour, minute, second) = civil(secs);
    let prefix = match reason {
        BackupReason::Manual => "prefixr-backup",
        BackupReason::Daily => "daily",
        BackupReason::BeforeRestore => "before-restore",
    };
    format!("{prefix}-{year:04}-{month:02}-{day:02}-{hour:02}{minute:02}{second:02}.tar.gz")
}

/// UTC date and time of a Unix timestamp (Howard Hinnant's civil_from_days).
fn civil(secs: u64) -> (i64, u32, u32, u32, u32, u32) {
    let days = (secs / 86_400) as i64;
    let rest = secs % 86_400;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = yoe + era * 400 + i64::from(month <= 2);
    let (hour, minute, second) = (
        (rest / 3600) as u32,
        (rest / 60 % 60) as u32,
        (rest % 60) as u32,
    );
    (year, month, day, hour, minute, second)
}

/// Writes a backup of `config` and the data dir's folders to `dest`.
pub fn write_backup(
    dest: &Path,
    config: &AppConfig,
    locations: &Locations,
    include_secrets: bool,
    reason: BackupReason,
    app_version: &str,
) -> Result<BackupInfo, String> {
    let config = backup_config(config, include_secrets);
    let manifest = Manifest {
        format: FORMAT,
        app_version: app_version.to_string(),
        created_at: now_secs(),
        reason,
        games: config.games.len(),
        categories: config.library.categories.len(),
        secrets_included: include_secrets,
    };
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Could not create {}: {e}", parent.display()))?;
    }
    // Written next to the target and renamed, so a failed backup never
    // leaves a broken file under the real name.
    let tmp = dest.with_extension("partial");
    let write = || -> std::io::Result<()> {
        let file = fs::File::create(&tmp)?;
        let mut archive = tar::Builder::new(GzEncoder::new(file, Compression::default()));
        let mut add = |name: &str, bytes: &[u8]| -> std::io::Result<()> {
            let mut header = tar::Header::new_gnu();
            header.set_size(bytes.len() as u64);
            header.set_mode(0o600);
            header.set_mtime(manifest.created_at);
            header.set_cksum();
            archive.append_data(&mut header, name, bytes)
        };
        add(MANIFEST, &serde_json::to_vec_pretty(&manifest)?)?;
        let json = to_disk_json(&config).map_err(std::io::Error::other)?;
        add(CONFIG, json.as_bytes())?;
        for folder in FOLDERS {
            let dir = locations.data_dir.join(folder);
            if dir.is_dir() {
                archive.append_dir_all(folder, &dir)?;
            }
        }
        archive.into_inner()?.finish()?.sync_all()
    };
    if let Err(e) = write() {
        let _ = fs::remove_file(&tmp);
        return Err(format!(
            "Could not write the backup {}: {e}",
            dest.display()
        ));
    }
    fs::rename(&tmp, dest)
        .map_err(|e| format!("Could not write the backup {}: {e}", dest.display()))?;
    info(dest, manifest)
}

fn info(path: &Path, manifest: Manifest) -> Result<BackupInfo, String> {
    Ok(BackupInfo {
        path: path.to_string_lossy().into_owned(),
        file_name: path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        created_at: manifest.created_at,
        reason: manifest.reason,
        app_version: manifest.app_version,
        games: manifest.games,
        categories: manifest.categories,
        secrets_included: manifest.secrets_included,
        size_bytes: fs::metadata(path).map(|m| m.len()).unwrap_or(0),
    })
}

fn open(path: &Path) -> Result<tar::Archive<GzDecoder<fs::File>>, String> {
    let file =
        fs::File::open(path).map_err(|e| format!("Could not open {}: {e}", path.display()))?;
    Ok(tar::Archive::new(GzDecoder::new(file)))
}

fn not_a_backup(path: &Path) -> String {
    format!("{} is not a Prefixr backup", path.display())
}

/// Reads a backup's manifest and config without unpacking the rest; checks
/// both, so a restore never starts from a file it can't use.
fn read_backup(path: &Path) -> Result<(Manifest, AppConfig), String> {
    let mut manifest = None;
    let mut config = None;
    for entry in open(path)?.entries().map_err(|_| not_a_backup(path))? {
        let mut entry = entry.map_err(|_| not_a_backup(path))?;
        let name = entry
            .path()
            .map_err(|_| not_a_backup(path))?
            .to_string_lossy()
            .into_owned();
        if name != MANIFEST && name != CONFIG {
            continue;
        }
        let mut bytes = Vec::new();
        entry
            .read_to_end(&mut bytes)
            .map_err(|_| not_a_backup(path))?;
        if name == MANIFEST {
            manifest =
                Some(serde_json::from_slice::<Manifest>(&bytes).map_err(|_| not_a_backup(path))?);
        } else {
            config = Some(bytes);
        }
        if manifest.is_some() && config.is_some() {
            break;
        }
    }
    let (Some(manifest), Some(config)) = (manifest, config) else {
        return Err(not_a_backup(path));
    };
    if manifest.format > FORMAT {
        return Err(format!(
            "{} was made by a newer Prefixr ({}); update Prefixr to restore it",
            path.display(),
            manifest.app_version
        ));
    }
    let config: AppConfig = serde_json::from_slice(&config)
        .map_err(|e| format!("The config in {} is invalid: {e}", path.display()))?;
    Ok((manifest, config))
}

pub fn backup_info(path: &Path) -> Result<BackupInfo, String> {
    let (manifest, _) = read_backup(path)?;
    info(path, manifest)
}

/// Only plain relative paths inside one of `FOLDERS`: nothing a crafted
/// archive could use to write elsewhere.
fn safe_folder_entry(path: &Path) -> bool {
    let mut components = path.components();
    let first_ok = matches!(components.next(), Some(Component::Normal(first)) if FOLDERS.iter().any(|f| first == *f));
    first_ok && components.all(|c| matches!(c, Component::Normal(_)))
}

/// Unpacks a backup's folders into `staging` (made fresh).
fn unpack_folders(path: &Path, staging: &Path) -> Result<(), String> {
    let _ = fs::remove_dir_all(staging);
    fs::create_dir_all(staging)
        .map_err(|e| format!("Could not create {}: {e}", staging.display()))?;
    for entry in open(path)?.entries().map_err(|_| not_a_backup(path))? {
        let mut entry = entry.map_err(|_| not_a_backup(path))?;
        let name = entry.path().map_err(|_| not_a_backup(path))?.into_owned();
        if !safe_folder_entry(&name) {
            continue;
        }
        let kind = entry.header().entry_type();
        if !(kind.is_file() || kind.is_dir()) {
            continue;
        }
        entry
            .unpack_in(staging)
            .map_err(|e| format!("Could not unpack {}: {e}", name.display()))?;
    }
    Ok(())
}

/// Puts the unpacked folders in place of the data dir's, keeping the old
/// ones until the new ones are in.
fn swap_folders(staging: &Path, data_dir: &Path) -> Result<(), String> {
    for folder in FOLDERS {
        let target = data_dir.join(folder);
        let old = data_dir.join(format!("{folder}.old"));
        let _ = fs::remove_dir_all(&old);
        if target.exists() {
            fs::rename(&target, &old)
                .map_err(|e| format!("Could not move {}: {e}", target.display()))?;
        }
        let new = staging.join(folder);
        let moved = if new.is_dir() {
            fs::rename(&new, &target)
        } else {
            fs::create_dir_all(&target)
        };
        if let Err(e) = moved {
            // Back as it was.
            let _ = fs::rename(&old, &target);
            return Err(format!("Could not put {} in place: {e}", target.display()));
        }
        let _ = fs::remove_dir_all(&old);
    }
    let _ = fs::remove_dir_all(staging);
    Ok(())
}

/// The restored config, keeping this machine's secrets where the backup
/// left them out.
fn merge_secrets(mut restored: AppConfig, current: &AppConfig) -> AppConfig {
    if restored.steamgriddb.api_key.is_none() {
        restored.steamgriddb.api_key = current.steamgriddb.api_key.clone();
    }
    if restored.github.token.is_none() {
        restored.github.token = current.github.token.clone();
    }
    restored
}

/// The automatic backups (daily and before restores), newest first.
pub fn automatic_backups(backups_dir: &Path) -> Vec<PathBuf> {
    let mut found: Vec<(SystemTime, PathBuf)> = fs::read_dir(backups_dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.to_string_lossy().ends_with(".tar.gz"))
        .map(|path| {
            let modified = fs::metadata(&path)
                .and_then(|m| m.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH);
            (modified, path)
        })
        .collect();
    found.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.1.cmp(&a.1)));
    found.into_iter().map(|(_, path)| path).collect()
}

/// Deletes all but the newest `keep` automatic backups, never `except`.
pub fn prune(backups_dir: &Path, keep: usize, except: Option<&Path>) {
    for old in automatic_backups(backups_dir).into_iter().skip(keep) {
        if Some(old.as_path()) != except {
            let _ = fs::remove_file(old);
        }
    }
}

/// Whether the daily backup is due: none in `backups_dir` from the last day.
pub fn daily_due(backups_dir: &Path, now: SystemTime) -> bool {
    !automatic_backups(backups_dir).iter().any(|path| {
        fs::metadata(path)
            .and_then(|m| m.modified())
            .is_ok_and(|modified| now.duration_since(modified).is_ok_and(|age| age < DAILY))
    })
}

fn app_version(app: &AppHandle) -> String {
    app.package_info().version.to_string()
}

/// Makes an automatic backup now and keeps the newest of them.
fn automatic_backup(
    app: &AppHandle,
    reason: BackupReason,
    except: Option<&Path>,
) -> Result<BackupInfo, String> {
    let locations = Locations::of(app)?;
    let config = app.state::<ConfigState>().locked().clone();
    let dest = locations
        .backups_dir
        .join(file_name(reason, SystemTime::now()));
    // With the secrets: these stay on this machine, and restoring one
    // should bring back exactly what was.
    let info = write_backup(&dest, &config, &locations, true, reason, &app_version(app))?;
    prune(&locations.backups_dir, KEEP_AUTOMATIC, except);
    Ok(info)
}

/// The daily backup, when the window opens. Best-effort.
pub fn daily_backup(app: &AppHandle) {
    let Ok(locations) = Locations::of(app) else {
        return;
    };
    if daily_due(&locations.backups_dir, SystemTime::now()) {
        if let Err(e) = automatic_backup(app, BackupReason::Daily, None) {
            eprintln!("Could not make the daily backup: {e}");
        }
    }
}

/// Makes a backup at `path` (a user's).
pub fn create(app: &AppHandle, path: &Path, include_secrets: bool) -> Result<BackupInfo, String> {
    let locations = Locations::of(app)?;
    let config = app.state::<ConfigState>().locked().clone();
    write_backup(
        path,
        &config,
        &locations,
        include_secrets,
        BackupReason::Manual,
        &app_version(app),
    )
}

/// Restores the backup at `path`: checks it, backs up what is now (to undo
/// it), puts its artwork and icons in place and takes over its config, under
/// the config lock. The window is told with `config-changed`.
pub fn restore(app: &AppHandle, path: &Path) -> Result<BackupInfo, String> {
    let (manifest, restored) = read_backup(path)?;
    let locations = Locations::of(app)?;
    let staging = locations.data_dir.join("restore.partial");
    unpack_folders(path, &staging)?;

    let _lock = lock_config(app)?;
    let state = app.state::<ConfigState>();
    let mut current = state.locked();
    // What's on disk now is what the undo must bring back.
    let now = load_config(app).unwrap_or_else(|_| current.clone());
    let undo = locations
        .backups_dir
        .join(file_name(BackupReason::BeforeRestore, SystemTime::now()));
    write_backup(
        &undo,
        &now,
        &locations,
        true,
        BackupReason::BeforeRestore,
        &app_version(app),
    )?;
    prune(&locations.backups_dir, KEEP_AUTOMATIC, Some(path));

    swap_folders(&staging, &locations.data_dir)?;
    save_config(app, &merge_secrets(restored, &now))?;
    // Read back, so the games' icon paths are filled in as on a start.
    *current = load_config(app)?;
    drop(current);
    let _ = app.emit("config-changed", ());
    info(path, manifest)
}

#[tauri::command(async)]
pub fn create_backup(
    app: AppHandle,
    path: String,
    include_secrets: bool,
) -> Result<BackupInfo, AppError> {
    Ok(create(&app, Path::new(&path), include_secrets)?)
}

/// The automatic backups, newest first; a file that isn't one is skipped.
#[tauri::command(async)]
pub fn list_backups(app: AppHandle) -> Result<Vec<BackupInfo>, AppError> {
    let locations = Locations::of(&app)?;
    Ok(automatic_backups(&locations.backups_dir)
        .iter()
        .filter_map(|path| backup_info(path).ok())
        .collect())
}

#[tauri::command(async)]
pub fn inspect_backup(path: String) -> Result<BackupInfo, AppError> {
    Ok(backup_info(Path::new(&path))?)
}

#[tauri::command(async)]
pub fn restore_backup(app: AppHandle, path: String) -> Result<BackupInfo, AppError> {
    Ok(restore(&app, Path::new(&path))?)
}

/// A writer for the tests' crafted archives.
#[cfg(test)]
pub(crate) fn write_raw_archive(dest: &Path, entries: &[(&str, &[u8])]) {
    let file = fs::File::create(dest).unwrap();
    let mut archive = tar::Builder::new(GzEncoder::new(file, Compression::default()));
    for (name, bytes) in entries {
        let mut header = tar::Header::new_gnu();
        header.set_size(bytes.len() as u64);
        header.set_mode(0o600);
        header.set_cksum();
        archive.append_data(&mut header, name, *bytes).unwrap();
    }
    archive
        .into_inner()
        .unwrap()
        .finish()
        .unwrap()
        .flush()
        .unwrap();
}

#[cfg(test)]
mod tests;
