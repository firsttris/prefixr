//! `prefixr library …` and `prefixr category …`: the whole library,
//! store games not installed yet included, and how it's organized —
//! categories, favorites, hidden games. Made for an agent sorting a library
//! into categories: `library list --json` gives every game's key, and
//! `category assign` takes any number of them.

use serde::Serialize;
use tauri::{AppHandle, Manager};

use super::{config, table, Out};
use crate::commands::library::{game_key, store_key, LibraryConfig};
use crate::commands::stores::list_store_games;
use crate::config::{save_config, AppConfig, ConfigState};
use crate::error::AppError;
use crate::lock::LockExt;

#[derive(clap::Subcommand, Debug)]
pub(super) enum LibraryCommand {
    /// List every game: the library's own and the connected stores' ones
    /// not installed yet, with their keys, categories and play time
    List {
        /// Only the games in this category
        #[arg(long)]
        category: Option<String>,
        /// Only favorites
        #[arg(long)]
        favorites: bool,
        /// Also list hidden games
        #[arg(long)]
        hidden: bool,
    },
    /// Mark games as favorites (`--off` unmarks them)
    Favorite {
        #[arg(required = true)]
        games: Vec<String>,
        #[arg(long)]
        off: bool,
    },
    /// Hide games from the library (`--off` shows them again)
    Hide {
        #[arg(required = true)]
        games: Vec<String>,
        #[arg(long)]
        off: bool,
    },
}

#[derive(clap::Subcommand, Debug)]
pub(super) enum CategoryCommand {
    /// List the categories and how many games each holds
    List,
    /// Create a category
    Add { name: String },
    /// Rename a category
    Rename { category: String, name: String },
    /// Delete a category; its games stay in the library
    Remove { category: String },
    /// Put games in a category (by key, id, or name, as `library list` shows them)
    Assign {
        category: String,
        #[arg(required = true)]
        games: Vec<String>,
    },
    /// Take games out of a category
    Unassign {
        category: String,
        #[arg(required = true)]
        games: Vec<String>,
    },
}

pub(super) fn changes_config(command: &LibraryCommand) -> bool {
    !matches!(command, LibraryCommand::List { .. })
}

pub(super) fn category_changes_config(command: &CategoryCommand) -> bool {
    !matches!(command, CategoryCommand::List)
}

/// A game of the whole library, as listed and as JSON.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct LibraryEntry {
    /// What categories, favorites and the commands here know it by.
    pub key: String,
    pub name: String,
    /// `local` for a game added by hand, or the store's: `epic`, `gog`, …
    pub source: String,
    pub installed: bool,
    /// The library game's id, once installed.
    pub game_id: Option<String>,
    pub categories: Vec<String>,
    pub favorite: bool,
    pub hidden: bool,
    pub playtime_secs: u64,
    pub last_played: Option<u64>,
}

/// Every game, installed ones first in library order, then the stores'
/// not installed yet.
pub(crate) fn entries(
    config: &AppConfig,
    store_games: &[(String, String, String)],
) -> Vec<LibraryEntry> {
    let library = &config.library;
    let entry = |key: String, name: String, source: String, game_id: Option<String>| {
        let activity = library.activity.get(&key).cloned().unwrap_or_default();
        LibraryEntry {
            categories: library.categories_of(&key),
            favorite: library.favorites.contains(&key),
            hidden: library.hidden.contains(&key),
            playtime_secs: activity.playtime_secs,
            last_played: activity.last_played,
            installed: game_id.is_some(),
            key,
            name,
            source,
            game_id,
        }
    };
    let mut list: Vec<LibraryEntry> = config
        .games
        .iter()
        .map(|game| {
            let source = game
                .store
                .as_ref()
                .map_or("local", |link| link.store.slug())
                .to_string();
            entry(
                game_key(game),
                game.name.clone(),
                source,
                Some(game.id.to_string()),
            )
        })
        .collect();
    for (key, title, source) in store_games {
        if !list.iter().any(|e| &e.key == key) {
            list.push(entry(key.clone(), title.clone(), source.clone(), None));
        }
    }
    list
}

/// The entry `query` names: its key, its game's id or the start of it, its
/// name (ignoring case), or the one entry whose name contains it.
pub(crate) fn find_entry<'a>(
    entries: &'a [LibraryEntry],
    query: &str,
) -> Result<&'a LibraryEntry, String> {
    let query = query.trim();
    let lower = query.to_lowercase();
    let pick = |matches: Vec<&'a LibraryEntry>| -> Option<Result<&'a LibraryEntry, String>> {
        match matches.as_slice() {
            [] => None,
            [entry] => Some(Ok(*entry)),
            several => Some(Err(format!(
                "\"{query}\" matches several games: {}",
                several
                    .iter()
                    .map(|e| format!("{} ({})", e.name, e.key))
                    .collect::<Vec<_>>()
                    .join(", ")
            ))),
        }
    };
    let exact = |e: &&LibraryEntry| e.key == query || e.game_id.as_deref() == Some(lower.as_str());
    if let Some(entry) = entries.iter().find(exact) {
        return Ok(entry);
    }
    let by_id_start = entries
        .iter()
        .filter(|e| {
            lower.len() >= 4
                && e.game_id
                    .as_deref()
                    .is_some_and(|id| id.replace('-', "").starts_with(&lower.replace('-', "")))
        })
        .collect();
    let by_name = || {
        entries
            .iter()
            .filter(|e| e.name.trim().to_lowercase() == lower)
            .collect()
    };
    let by_name_part = || {
        entries
            .iter()
            .filter(|e| e.name.to_lowercase().contains(&lower))
            .collect()
    };
    pick(by_id_start)
        .or_else(|| pick(by_name()))
        .or_else(|| pick(by_name_part()))
        .unwrap_or_else(|| Err(format!("No game matches \"{query}\"")))
}

pub(super) fn all_entries(app: &AppHandle) -> Result<Vec<LibraryEntry>, AppError> {
    let store_games: Vec<_> = list_store_games(app.clone(), app.state())?
        .into_iter()
        .map(|game| {
            (
                store_key(game.store, &game.app_name),
                game.title,
                game.store.slug().to_string(),
            )
        })
        .collect();
    Ok(entries(&config(app), &store_games))
}

fn keys(entries: &[LibraryEntry], queries: &[String]) -> Result<Vec<String>, AppError> {
    let mut keys = Vec::new();
    for query in queries {
        let key = find_entry(entries, query)?.key.clone();
        if !keys.contains(&key) {
            keys.push(key);
        }
    }
    Ok(keys)
}

/// Changes the library's organization and saves it.
fn change(
    app: &AppHandle,
    edit: impl FnOnce(&mut LibraryConfig) -> Result<(), String>,
) -> Result<(), AppError> {
    let state = app.state::<ConfigState>();
    let mut config = state.locked();
    edit(&mut config.library)?;
    save_config(app, &config)?;
    Ok(())
}

fn duration(secs: u64) -> String {
    match (secs / 3600, secs / 60 % 60) {
        (0, 0) => String::new(),
        (0, minutes) => format!("{minutes} min"),
        (hours, minutes) => format!("{hours} h {minutes} min"),
    }
}

fn entry_rows(entries: &[&LibraryEntry]) -> Vec<Vec<String>> {
    entries
        .iter()
        .map(|e| {
            let mut flags = Vec::new();
            if e.favorite {
                flags.push("favorite");
            }
            if e.hidden {
                flags.push("hidden");
            }
            vec![
                match &e.game_id {
                    Some(id) if e.source == "local" => id[..8].to_string(),
                    _ => e.key.clone(),
                },
                e.name.clone(),
                e.source.clone(),
                if e.installed { "installed" } else { "-" }.to_string(),
                e.categories.join(", "),
                duration(e.playtime_secs),
                flags.join(", "),
            ]
        })
        .collect()
}

pub(super) async fn library_command(
    app: &AppHandle,
    out: Out,
    command: LibraryCommand,
) -> Result<(), AppError> {
    let entries = all_entries(app)?;
    match command {
        LibraryCommand::List {
            category,
            favorites,
            hidden,
        } => {
            let category = match category {
                Some(query) => Some(config(app).library.find_category(&query)?.name.clone()),
                None => None,
            };
            let shown: Vec<&LibraryEntry> = entries
                .iter()
                .filter(|e| hidden || !e.hidden)
                .filter(|e| !favorites || e.favorite)
                .filter(|e| {
                    category
                        .as_ref()
                        .is_none_or(|name| e.categories.contains(name))
                })
                .collect();
            out.print(&shown, || {
                if shown.is_empty() {
                    return "No games".to_string();
                }
                table(
                    &[
                        "KEY",
                        "NAME",
                        "SOURCE",
                        "STATUS",
                        "CATEGORIES",
                        "PLAYED",
                        "FLAGS",
                    ],
                    &entry_rows(&shown),
                )
            })
        }
        LibraryCommand::Favorite { games, off } => {
            let keys = keys(&entries, &games)?;
            change(app, |library| {
                keys.iter().for_each(|key| library.set_favorite(key, !off));
                Ok(())
            })?;
            let verb = if off { "Unmarked" } else { "Marked" };
            out.print(&keys, || {
                format!("{verb} {} game(s) as favorite", keys.len())
            })
        }
        LibraryCommand::Hide { games, off } => {
            let keys = keys(&entries, &games)?;
            change(app, |library| {
                keys.iter().for_each(|key| library.set_hidden(key, !off));
                Ok(())
            })?;
            let verb = if off { "Showing" } else { "Hid" };
            out.print(&keys, || format!("{verb} {} game(s)", keys.len()))
        }
    }
}

pub(super) async fn category_command(
    app: &AppHandle,
    out: Out,
    command: CategoryCommand,
) -> Result<(), AppError> {
    match command {
        CategoryCommand::List => {
            let categories = config(app).library.categories;
            out.print(&categories, || {
                if categories.is_empty() {
                    return "No categories yet; add one with `prefixr category add <name>`"
                        .to_string();
                }
                let rows: Vec<Vec<String>> = categories
                    .iter()
                    .map(|c| vec![c.name.clone(), c.games.len().to_string(), c.id.clone()])
                    .collect();
                table(&["NAME", "GAMES", "ID"], &rows)
            })
        }
        CategoryCommand::Add { name } => {
            let mut created = None;
            change(app, |library| {
                created = Some(library.create_category(&name)?);
                Ok(())
            })?;
            let created = created.expect("set by the change");
            out.print(&created, || format!("Added category {}", created.name))
        }
        CategoryCommand::Rename { category, name } => {
            let mut renamed = None;
            change(app, |library| {
                library.rename_category(&category, &name)?;
                renamed = Some(library.find_category(&name)?.clone());
                Ok(())
            })?;
            let renamed = renamed.expect("set by the change");
            out.print(&renamed, || {
                format!("Renamed {category} to {}", renamed.name)
            })
        }
        CategoryCommand::Remove { category } => {
            let mut removed = None;
            change(app, |library| {
                removed = Some(library.find_category(&category)?.clone());
                library.delete_category(&category)
            })?;
            let removed = removed.expect("set by the change");
            out.print(&removed, || format!("Removed category {}", removed.name))
        }
        CategoryCommand::Assign { category, games } => assign(app, out, &category, &games, true),
        CategoryCommand::Unassign { category, games } => assign(app, out, &category, &games, false),
    }
}

fn assign(
    app: &AppHandle,
    out: Out,
    category: &str,
    games: &[String],
    member: bool,
) -> Result<(), AppError> {
    let keys = keys(&all_entries(app)?, games)?;
    let mut name = String::new();
    change(app, |library| {
        library.set_in_category(category, &keys, member)?;
        name = library.find_category(category)?.name.clone();
        Ok(())
    })?;
    let text = if member {
        format!("Put {} game(s) in {name}", keys.len())
    } else {
        format!("Took {} game(s) out of {name}", keys.len())
    };
    out.print(&keys, || text)
}

#[cfg(test)]
mod tests;
