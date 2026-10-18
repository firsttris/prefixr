//! `prefixr backup …`: back up the library and its settings, list the
//! automatic backups, and restore one; for an agent, a safe point before a
//! larger change (see `commands::backup`).

use std::path::PathBuf;
use std::time::SystemTime;

use tauri::AppHandle;

use super::{absolute, table, Out};
use crate::commands::backup::{
    automatic_backups, backup_info, create, file_name, restore, BackupInfo, BackupReason, Locations,
};
use crate::error::AppError;

#[derive(clap::Subcommand, Debug)]
pub(super) enum BackupCommand {
    /// Back up the library, its settings, categories, play times and artwork
    Create {
        /// Where to write it; prefixr-backup-<date>.tar.gz in this folder otherwise
        path: Option<PathBuf>,
        /// Also back up the SteamGridDB API key and the GitHub token
        #[arg(long)]
        with_secrets: bool,
    },
    /// List the automatic backups (when turned on, and before every restore), newest first
    List,
    /// Restore a backup: a file, or a number from `backup list`. What is now
    /// is backed up first, so this can be undone
    Restore { backup: String },
}

/// None of them hold the config lock from the start: a restore takes it
/// itself, around the moment it writes.
pub(super) fn changes_config(_command: &BackupCommand) -> bool {
    false
}

/// The automatic backups `backup list` shows, newest first: the ones it can
/// read. Numbers count these, so they always match the list.
pub(crate) fn readable_backups(backups_dir: &std::path::Path) -> Vec<BackupInfo> {
    automatic_backups(backups_dir)
        .iter()
        .filter_map(|path| backup_info(path).ok())
        .collect()
}

/// The backup `query` names: a number from `backup list` (1 is the newest),
/// or a file.
pub(crate) fn resolve(query: &str, automatic: &[PathBuf]) -> Result<PathBuf, AppError> {
    if let Ok(number) = query.parse::<usize>() {
        return automatic
            .get(number.wrapping_sub(1))
            .cloned()
            .ok_or_else(|| format!("No backup {number}; see `prefixr backup list`").into());
    }
    absolute(std::path::Path::new(query))
}

fn reason_name(reason: BackupReason) -> &'static str {
    match reason {
        BackupReason::Manual => "manual",
        BackupReason::Daily => "daily",
        BackupReason::BeforeRestore => "before restore",
    }
}

fn describe(info: &BackupInfo) -> String {
    format!(
        "{} ({} games, {} categories{})",
        info.file_name,
        info.games,
        info.categories,
        if info.secrets_included {
            ", with API keys"
        } else {
            ""
        }
    )
}

pub(super) async fn backup_command(
    app: &AppHandle,
    out: Out,
    command: BackupCommand,
) -> Result<(), AppError> {
    let locations = Locations::of(app)?;
    match command {
        BackupCommand::Create { path, with_secrets } => {
            let path = match path {
                Some(path) => absolute(&path)?,
                None => absolute(std::path::Path::new(&file_name(
                    BackupReason::Manual,
                    SystemTime::now(),
                )))?,
            };
            let info = create(app, &path, with_secrets)?;
            out.print(&info, || format!("Backed up to {}", info.path))
        }
        BackupCommand::List => {
            let backups = readable_backups(&locations.backups_dir);
            out.print(&backups, || {
                if backups.is_empty() {
                    return "No automatic backups yet".to_string();
                }
                let rows: Vec<Vec<String>> = backups
                    .iter()
                    .enumerate()
                    .map(|(i, b)| {
                        vec![
                            (i + 1).to_string(),
                            reason_name(b.reason).to_string(),
                            b.games.to_string(),
                            b.file_name.clone(),
                        ]
                    })
                    .collect();
                table(&["#", "KIND", "GAMES", "FILE"], &rows)
            })
        }
        BackupCommand::Restore { backup } => {
            let listed: Vec<PathBuf> = readable_backups(&locations.backups_dir)
                .into_iter()
                .map(|b| PathBuf::from(b.path))
                .collect();
            let path = resolve(&backup, &listed)?;
            let info = restore(app, &path)?;
            out.print(&info, || {
                format!(
                    "Restored {}. What was before is in `prefixr backup list` as number 1.",
                    describe(&info)
                )
            })
        }
    }
}

#[cfg(test)]
mod tests;
