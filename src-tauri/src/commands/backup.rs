//! Backups of everything Prefixr keeps about the library, and restoring
//! one: `config.json` (the games, the prefix list, every setting, the
//! categories and play times), the chosen artwork and the exe icons, in one
//! `.tar.gz`. Not in it: the prefixes and the games' own files (far too big,
//! and they stay where they are), the store logins, and unless asked for,
//! the SteamGridDB API key and the GitHub token.
//!
//! Prefixr also backs up by itself, into `backups/` under the data dir or
//! the folder the user picked: before every restore, so a restore can
//! always be undone, and when the user turned automatic backups on, every
//! few days when the window opens. The newest few of those are kept (see
//! `BackupConfig`).

use std::fs;
use std::io::Read;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, SystemTime};

use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::commands::library::now_secs;
use crate::config::{
    config_file_path, load_config, lock_config, save_config, to_disk_json, AppConfig, ConfigState,
};
use crate::error::AppError;
use crate::host_env::{host_config_home, is_flatpak};
use crate::lock::LockExt;
use crate::models::BackupConfig;

/// The archive layout's version; a backup from a newer Prefixr with a
/// higher one is refused rather than half understood.
const FORMAT: u32 = 1;
const MANIFEST: &str = "manifest.json";
const CONFIG: &str = "config.json";
/// The data dir's folders a backup holds, as they're named in it too.
const FOLDERS: [&str; 2] = ["artwork", "exe-icons"];
/// What the backup settings may hold; anything else is brought into range.
const MAX_INTERVAL_DAYS: u32 = 365;
const MAX_KEEP: u32 = 100;
/// The automatic backups' file name prefixes; other files in `backups/`
/// (a backup the user saved there) are never pruned.
const AUTOMATIC_PREFIXES: [&str; 2] = ["daily-", "before-restore-"];
/// More than a backup's own small files may hold is no backup's: a crafted
/// archive can't fill the memory with them.
const MAX_META_BYTES: u64 = 64 * 1024 * 1024;
/// More than any library's artwork comes to: a crafted archive can't fill
/// the disk the config is saved on.
const MAX_UNPACKED_BYTES: u64 = 16 * 1024 * 1024 * 1024;
/// Leftovers of a restore or backup that was cut off (a crash) are cleaned
/// up once they're this old, so a running one is never touched.
const LEFTOVER_AGE: Duration = Duration::from_secs(60 * 60);
const DAY: Duration = Duration::from_secs(24 * 60 * 60);

/// Why a backup was made.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "kebab-case")]
pub enum BackupReason {
    /// Made by the user, from the window or the command line.
    Manual,
    /// The automatic one, when the window opens (named for when it was
    /// always daily; kept so older backups still read).
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
        let settings = app.state::<ConfigState>().locked().backup.clone();
        Ok(Self {
            backups_dir: backups_dir(&data_dir, &settings),
            data_dir,
        })
    }
}

/// Where the automatic backups go: the user's folder, or `backups/` under
/// the data dir.
pub fn backups_dir(data_dir: &Path, settings: &BackupConfig) -> PathBuf {
    settings
        .dir
        .clone()
        .unwrap_or_else(|| data_dir.join("backups"))
}

/// The settings as they're kept: numbers in range, a folder only when it's
/// a full path.
pub fn sanitized_settings(mut settings: BackupConfig) -> Result<BackupConfig, String> {
    settings.interval_days = settings.interval_days.clamp(1, MAX_INTERVAL_DAYS);
    settings.keep = settings.keep.clamp(1, MAX_KEEP);
    settings.dir = settings
        .dir
        .filter(|dir| !dir.as_os_str().to_string_lossy().trim().is_empty());
    if let Some(dir) = &settings.dir {
        if !dir.is_absolute() {
            return Err(format!(
                "The backup folder must be a full path, not {}",
                dir.display()
            ));
        }
    }
    Ok(settings)
}

fn keep(settings: &BackupConfig) -> usize {
    settings.keep.clamp(1, MAX_KEEP) as usize
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
        // Only for this user: it can hold the API keys.
        let file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .mode(0o600)
            .open(&tmp)?;
        // Fast rather than small: covers are compressed already, and the
        // daily backup shouldn't hold the start up.
        let mut archive = tar::Builder::new(GzEncoder::new(file, Compression::fast()));
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
        if entry.header().size().unwrap_or(u64::MAX) > MAX_META_BYTES {
            return Err(not_a_backup(path));
        }
        let mut bytes = Vec::new();
        entry
            .by_ref()
            .take(MAX_META_BYTES)
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

/// Unpacks a backup's folders into `staging` (made fresh), at most
/// `max_bytes` of them; on any failure nothing is left behind.
fn unpack_folders(path: &Path, staging: &Path, max_bytes: u64) -> Result<(), String> {
    let _ = fs::remove_dir_all(staging);
    fs::create_dir_all(staging)
        .map_err(|e| format!("Could not create {}: {e}", staging.display()))?;
    let unpack = || -> Result<(), String> {
        let mut total = 0u64;
        for entry in open(path)?.entries().map_err(|_| not_a_backup(path))? {
            let mut entry = entry.map_err(|_| not_a_backup(path))?;
            let name = entry.path().map_err(|_| not_a_backup(path))?.into_owned();
            let kind = entry.header().entry_type();
            // No links of any kind: they could point anywhere.
            if !safe_folder_entry(&name) || !(kind.is_file() || kind.is_dir()) {
                continue;
            }
            total = total.saturating_add(entry.header().size().unwrap_or(u64::MAX));
            if total > max_bytes {
                return Err(format!(
                    "{} unpacks to more than any backup holds",
                    path.display()
                ));
            }
            entry
                .unpack_in(staging)
                .map_err(|e| format!("Could not unpack {}: {e}", name.display()))?;
        }
        Ok(())
    };
    unpack().inspect_err(|_| {
        let _ = fs::remove_dir_all(staging);
    })
}

/// The backup's folders put in place of the data dir's, the old ones kept
/// aside until the restore is through: `commit` drops them, `roll_back`
/// puts them back.
pub(crate) struct Swapped {
    /// Each folder, and where its old one was moved (`None`: there was none).
    folders: Vec<(PathBuf, Option<PathBuf>)>,
    /// The folders this put in, to take out again on a roll-back.
    put_in: Vec<PathBuf>,
}

impl Swapped {
    pub(crate) fn roll_back(&self) {
        for target in &self.put_in {
            let _ = fs::remove_dir_all(target);
        }
        for (target, old) in &self.folders {
            if let Some(old) = old {
                let _ = fs::rename(old, target);
            }
        }
    }

    pub(crate) fn commit(self) {
        for (_, old) in self.folders {
            if let Some(old) = old {
                let _ = fs::remove_dir_all(old);
            }
        }
    }
}

/// Moves all of the data dir's folders aside first, then puts all of the
/// unpacked ones in: a failure on the way puts everything back as it was.
pub(crate) fn put_folders_in(
    staging: &Path,
    data_dir: &Path,
    tag: &str,
) -> Result<Swapped, String> {
    let mut swapped = Swapped {
        folders: Vec::new(),
        put_in: Vec::new(),
    };
    for folder in FOLDERS {
        let target = data_dir.join(folder);
        if !target.exists() {
            swapped.folders.push((target, None));
            continue;
        }
        let old = data_dir.join(format!("{folder}.{tag}.old"));
        if let Err(e) = fs::rename(&target, &old) {
            swapped.roll_back();
            return Err(format!("Could not move {} aside: {e}", target.display()));
        }
        swapped.folders.push((target, Some(old)));
    }
    for folder in FOLDERS {
        let target = data_dir.join(folder);
        let new = staging.join(folder);
        let moved = if new.is_dir() {
            fs::rename(&new, &target)
        } else {
            fs::create_dir_all(&target)
        };
        if let Err(e) = moved {
            swapped.roll_back();
            return Err(format!("Could not put {} in place: {e}", target.display()));
        }
        swapped.put_in.push(target);
    }
    Ok(swapped)
}

/// The restored config, keeping what belongs to this machine: its secrets
/// where the backup left them out, and its runners folder (the runners
/// aren't in a backup, and another computer's path may not even be
/// writable here).
fn merge_local(mut restored: AppConfig, current: &AppConfig) -> AppConfig {
    restored.runners_dir = current.runners_dir.clone();
    if restored.steamgriddb.api_key.is_none() {
        restored.steamgriddb.api_key = current.steamgriddb.api_key.clone();
    }
    if restored.github.token.is_none() {
        restored.github.token = current.github.token.clone();
    }
    restored
}

fn is_automatic(path: &Path) -> bool {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    name.ends_with(".tar.gz")
        && AUTOMATIC_PREFIXES
            .iter()
            .any(|prefix| name.starts_with(prefix))
}

/// The automatic backups (daily and before restores), newest first.
pub fn automatic_backups(backups_dir: &Path) -> Vec<PathBuf> {
    let mut found: Vec<(SystemTime, PathBuf)> = fs::read_dir(backups_dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| is_automatic(path))
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

/// Whether the automatic backup is due: none in `backups_dir` from the
/// last `interval_days`.
pub fn automatic_due(backups_dir: &Path, now: SystemTime, interval_days: u32) -> bool {
    let interval = DAY * interval_days.clamp(1, MAX_INTERVAL_DAYS);
    !automatic_backups(backups_dir).iter().any(|path| {
        fs::metadata(path)
            .and_then(|m| m.modified())
            .is_ok_and(|modified| now.duration_since(modified).is_ok_and(|age| age < interval))
    })
}

/// Whether any of `paths` changed after `since` (a file's or a folder's
/// own change time: a folder's changes when a file is added or removed in
/// it). One that can't be read counts as changed.
pub(crate) fn changed_since(since: SystemTime, paths: &[PathBuf]) -> bool {
    paths.iter().any(|path| {
        fs::metadata(path)
            .and_then(|m| m.modified())
            .map_or(path.exists(), |modified| modified > since)
    })
}

/// Removes what a cut-off restore or backup left behind (older than
/// `LEFTOVER_AGE`, so never a running one's).
pub(crate) fn clean_leftovers(locations: &Locations, now: SystemTime) {
    let old_enough = |path: &Path| {
        fs::metadata(path)
            .and_then(|m| m.modified())
            .is_ok_and(|modified| {
                now.duration_since(modified)
                    .is_ok_and(|age| age > LEFTOVER_AGE)
            })
    };
    for dir in [&locations.data_dir, &locations.backups_dir] {
        for path in fs::read_dir(dir)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
        {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let leftover = name.ends_with(".partial")
                || (name.ends_with(".old")
                    && FOLDERS.iter().any(|f| name.starts_with(&format!("{f}."))));
            if leftover && old_enough(&path) {
                let _ = fs::remove_dir_all(&path).or_else(|_| fs::remove_file(&path));
            }
        }
    }
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
    prune(&locations.backups_dir, keep(&config.backup), except);
    Ok(info)
}

/// The automatic backup, when the window opens (or they were just turned
/// on): only when the user turned them on, it's due, and something changed
/// since the last one. Best-effort; the window's Backup page is told.
pub fn daily_backup(app: &AppHandle) {
    let Ok(locations) = Locations::of(app) else {
        return;
    };
    let now = SystemTime::now();
    clean_leftovers(&locations, now);
    let settings = app.state::<ConfigState>().locked().backup.clone();
    if !settings.automatic || !automatic_due(&locations.backups_dir, now, settings.interval_days) {
        return;
    }
    let newest = automatic_backups(&locations.backups_dir)
        .first()
        .and_then(|path| fs::metadata(path).and_then(|m| m.modified()).ok());
    let mut watched: Vec<PathBuf> = FOLDERS.iter().map(|f| locations.data_dir.join(f)).collect();
    if let Ok(config) = config_file_path(app) {
        watched.push(config);
    }
    if newest.is_some_and(|newest| !changed_since(newest, &watched)) {
        return;
    }
    match automatic_backup(app, BackupReason::Daily, None) {
        Ok(_) => {
            let _ = app.emit("backups-changed", ());
        }
        Err(e) => eprintln!("Could not make the automatic backup: {e}"),
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

/// Removes a restore's staging folder however the restore ends.
struct Staging(PathBuf);

impl Drop for Staging {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Restores the backup at `path`: checks it, unpacks it aside, backs up
/// what is now (to undo it), then under the config lock puts its folders in
/// and takes over its config; a failure on the way puts the folders back.
/// The window is told with `config-changed` and `backups-changed`.
pub fn restore(app: &AppHandle, path: &Path) -> Result<BackupInfo, String> {
    let (manifest, restored) = read_backup(path)?;
    let locations = Locations::of(app)?;
    // A folder of its own, so two restores at once (the window and the
    // command line) can't mix.
    let tag = format!(
        "{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    );
    let staging = Staging(locations.data_dir.join(format!("restore-{tag}.partial")));
    unpack_folders(path, &staging.0, MAX_UNPACKED_BYTES)?;

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
    prune(&locations.backups_dir, keep(&now.backup), Some(path));

    let swapped = put_folders_in(&staging.0, &locations.data_dir, &tag)?;
    if let Err(e) = save_config(app, &merge_local(restored, &now)) {
        swapped.roll_back();
        return Err(e);
    }
    swapped.commit();
    // Read back, so the games' icon paths are filled in as on a start.
    *current = load_config(app)?;
    drop(current);
    let _ = app.emit("config-changed", ());
    let _ = app.emit("backups-changed", ());
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

/// The native build's config file (AppImage, .deb, .rpm), when Prefixr
/// runs as a Flatpak and that file exists on the host: the Flatpak keeps its
/// library in `~/.var/app/<id>` and starts empty, so the empty library says
/// how to bring the native one over with a backup. `None` natively.
#[tauri::command]
pub fn native_library(app: AppHandle) -> Option<String> {
    if !is_flatpak() {
        return None;
    }
    let identifier = &app.config().identifier;
    native_config_file(&crate::env::process, identifier)
        .filter(|path| path.is_file())
        .map(|path| path.to_string_lossy().into_owned())
}

fn native_config_file(env: crate::env::Env, identifier: &str) -> Option<PathBuf> {
    Some(host_config_home(env)?.join(identifier).join("config.json"))
}

#[tauri::command]
pub fn get_backup_config(state: State<ConfigState>) -> Result<BackupConfig, AppError> {
    Ok(state.locked().backup.clone())
}

/// Saves the backup settings; turning them on makes the first one right
/// away (when due), so the user sees it work.
#[tauri::command(async)]
pub fn save_backup_config(
    app: AppHandle,
    state: State<ConfigState>,
    config: BackupConfig,
) -> Result<BackupConfig, AppError> {
    let settings = sanitized_settings(config)?;
    let mut app_config = state.locked();
    app_config.backup = settings.clone();
    save_config(&app, &app_config)?;
    drop(app_config);
    // Another folder lists other backups.
    let _ = app.emit("backups-changed", ());
    if settings.automatic {
        let app = app.clone();
        tauri::async_runtime::spawn_blocking(move || daily_backup(&app));
    }
    Ok(settings)
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
        .sync_all()
        .unwrap();
}

#[cfg(test)]
mod tests;
