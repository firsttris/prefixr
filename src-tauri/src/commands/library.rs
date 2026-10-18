//! How the library is organized beyond the games themselves: the user's
//! categories ("Kinder", "Koop", …), favorites, hidden games, when and how
//! long each game was played, and whether it was tested to run.
//!
//! All of it is keyed by a game's *library key* (see `game_key`), so it
//! covers both games added by hand and store games that aren't installed
//! yet, and a store game keeps its categories and play time through being
//! installed, uninstalled and installed again.

use std::collections::BTreeMap;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};
use uuid::Uuid;

use crate::config::{save_config, update_config, AppConfig, ConfigState};
use crate::error::AppError;
use crate::lock::LockExt;
use crate::models::{Game, StoreKind};

/// The library's organization, stored as `library` in `config.json`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct LibraryConfig {
    /// In the order the user sees them.
    #[serde(default)]
    pub categories: Vec<Category>,
    #[serde(default)]
    pub favorites: Vec<String>,
    /// Left out of the library unless asked for, e.g. a store's giveaways
    /// nobody intends to play.
    #[serde(default)]
    pub hidden: Vec<String>,
    #[serde(default)]
    pub activity: BTreeMap<String, GameActivity>,
    /// What testing a game found, by the user or an agent (`prefixr library
    /// test-status`). Untested games have no entry.
    #[serde(default)]
    pub tested: BTreeMap<String, TestResult>,
}

/// How a game ran when it was tested.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum TestStatus {
    /// It starts and plays.
    Works,
    /// It plays, with something wrong or missing (black videos, no sound,
    /// needs a login or a controller).
    Issues,
    /// It doesn't start, or can't be played.
    Broken,
}

/// The result of testing a game, and the setup it was tested with: once the
/// game has another runner or exe, the result may no longer hold.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct TestResult {
    pub status: TestStatus,
    /// What was seen or tried, e.g. "Videos stay black; PROTON_USE_WINED3D
    /// didn't help".
    #[serde(default)]
    pub note: String,
    /// Seconds since the Unix epoch.
    pub tested_at: u64,
    pub runner_id: String,
    pub exe_path: String,
}

impl TestResult {
    /// Whether `game` still has the setup it was tested with.
    pub fn is_current(&self, game: &Game) -> bool {
        self.runner_id == game.runner_id && Path::new(&self.exe_path) == game.exe_path
    }
}

/// A category of the user's, holding games by their library key.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct Category {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub games: Vec<String>,
}

/// When a game came into the library and how much it was played, in
/// seconds since the Unix epoch. Games from before this was recorded have
/// no `added_at`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct GameActivity {
    #[serde(default)]
    pub added_at: Option<u64>,
    #[serde(default)]
    pub last_played: Option<u64>,
    #[serde(default)]
    pub playtime_secs: u64,
}

/// The key the library's organization uses for a store's game:
/// `epic:<app name>`, as the frontend's `storeGameKey`.
pub fn store_key(store: StoreKind, app_name: &str) -> String {
    format!("{}:{app_name}", store.slug())
}

/// A game's library key: its store entry's for a game installed from a
/// store, its own id otherwise.
pub fn game_key(game: &Game) -> String {
    match &game.store {
        Some(link) => store_key(link.store, &link.app_name),
        None => game.id.to_string(),
    }
}

pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

fn toggle(list: &mut Vec<String>, key: &str, on: bool) {
    let present = list.iter().any(|k| k == key);
    if on && !present {
        list.push(key.to_string());
    } else if !on && present {
        list.retain(|k| k != key);
    }
}

fn clean_name(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("A category needs a name".to_string());
    }
    Ok(name.to_string())
}

impl LibraryConfig {
    pub fn set_favorite(&mut self, key: &str, favorite: bool) {
        toggle(&mut self.favorites, key, favorite);
    }

    pub fn set_hidden(&mut self, key: &str, hidden: bool) {
        toggle(&mut self.hidden, key, hidden);
    }

    /// The category named `query` (ignoring case), or with that id.
    pub fn find_category(&self, query: &str) -> Result<&Category, String> {
        let query = query.trim();
        self.categories
            .iter()
            .find(|c| c.id == query)
            .or_else(|| {
                self.categories
                    .iter()
                    .find(|c| c.name.to_lowercase() == query.to_lowercase())
            })
            .ok_or_else(|| format!("No category named {query}"))
    }

    fn category_mut(&mut self, query: &str) -> Result<&mut Category, String> {
        let id = self.find_category(query)?.id.clone();
        Ok(self
            .categories
            .iter_mut()
            .find(|c| c.id == id)
            .expect("found just now"))
    }

    fn check_unique(&self, name: &str, except: Option<&str>) -> Result<(), String> {
        let taken = self
            .categories
            .iter()
            .any(|c| Some(c.id.as_str()) != except && c.name.to_lowercase() == name.to_lowercase());
        if taken {
            Err(format!("There already is a category named {name}"))
        } else {
            Ok(())
        }
    }

    pub fn create_category(&mut self, name: &str) -> Result<Category, String> {
        let name = clean_name(name)?;
        self.check_unique(&name, None)?;
        let category = Category {
            id: Uuid::new_v4().to_string(),
            name,
            games: Vec::new(),
        };
        self.categories.push(category.clone());
        Ok(category)
    }

    pub fn rename_category(&mut self, query: &str, name: &str) -> Result<(), String> {
        let name = clean_name(name)?;
        let id = self.find_category(query)?.id.clone();
        self.check_unique(&name, Some(&id))?;
        self.category_mut(&id)?.name = name;
        Ok(())
    }

    pub fn delete_category(&mut self, query: &str) -> Result<(), String> {
        let id = self.find_category(query)?.id.clone();
        self.categories.retain(|c| c.id != id);
        Ok(())
    }

    /// Adds the games to the category, or takes them out of it.
    pub fn set_in_category(
        &mut self,
        query: &str,
        keys: &[String],
        member: bool,
    ) -> Result<(), String> {
        let category = self.category_mut(query)?;
        for key in keys {
            toggle(&mut category.games, key, member);
        }
        Ok(())
    }

    /// The names of the categories the game is in.
    pub fn categories_of(&self, key: &str) -> Vec<String> {
        self.categories
            .iter()
            .filter(|c| c.games.iter().any(|k| k == key))
            .map(|c| c.name.clone())
            .collect()
    }

    /// Notes when a game came into the library, unless it was before (a
    /// store game installed again).
    pub fn record_added(&mut self, key: &str, at: u64) {
        let activity = self.activity.entry(key.to_string()).or_default();
        activity.added_at.get_or_insert(at);
    }

    /// Adds a played session ending at `ended`.
    pub fn record_session(&mut self, key: &str, ended: u64, played: Duration) {
        let activity = self.activity.entry(key.to_string()).or_default();
        activity.last_played = Some(ended);
        activity.playtime_secs += played.as_secs();
    }

    /// After `game` was removed from the library: a store game stays in it
    /// as one to install again, and keeps its categories and play time for
    /// that; a game added by hand is gone for good.
    pub fn game_removed(&mut self, game: &Game) {
        if game.store.is_none() {
            self.forget(&game_key(game));
        }
    }

    /// Records how `game` ran when tested just now, with the setup it has;
    /// `None` takes the result away again.
    pub fn set_tested(&mut self, game: &Game, status: Option<TestStatus>, note: &str, at: u64) {
        let key = game_key(game);
        match status {
            Some(status) => {
                self.tested.insert(
                    key,
                    TestResult {
                        status,
                        note: note.trim().to_string(),
                        tested_at: at,
                        runner_id: game.runner_id.clone(),
                        exe_path: game.exe_path.display().to_string(),
                    },
                );
            }
            None => {
                self.tested.remove(&key);
            }
        }
    }

    /// Drops everything about a game that's gone for good.
    pub fn forget(&mut self, key: &str) {
        self.favorites.retain(|k| k != key);
        self.hidden.retain(|k| k != key);
        self.activity.remove(key);
        self.tested.remove(key);
        for category in &mut self.categories {
            category.games.retain(|k| k != key);
        }
    }
}

/// Records a session of a game that just ended, in the config as it is on
/// disk (see `update_config`): the Prefixr Steam started a game through
/// (`--run`) has held its copy since the game started, and the window or
/// the command line may well have changed the library meanwhile.
/// Best-effort, a lost session is no reason to fail the game's exit.
pub fn record_play_session(app: &AppHandle, key: &str, played: Duration) {
    let result = update_config(app, |config| {
        config.library.record_session(key, now_secs(), played);
        Ok(())
    });
    if let Err(e) = result {
        eprintln!("Could not record the play time of {key}: {e}");
    }
}

fn change(
    app: &AppHandle,
    state: &State<ConfigState>,
    edit: impl FnOnce(&mut LibraryConfig) -> Result<(), String>,
) -> Result<LibraryConfig, AppError> {
    let mut config = state.locked();
    let mut library = config.library.clone();
    edit(&mut library)?;
    config.library = library;
    save_config(app, &config)?;
    Ok(config.library.clone())
}

/// A copy of the library's organization as it is in `config`.
pub fn library_of(config: &AppConfig) -> LibraryConfig {
    config.library.clone()
}

#[tauri::command]
pub fn get_library(state: State<ConfigState>) -> LibraryConfig {
    library_of(&state.locked())
}

#[tauri::command(async)]
pub fn set_favorite(
    app: AppHandle,
    state: State<ConfigState>,
    key: String,
    favorite: bool,
) -> Result<LibraryConfig, AppError> {
    change(&app, &state, |library| {
        library.set_favorite(&key, favorite);
        Ok(())
    })
}

#[tauri::command(async)]
pub fn set_hidden(
    app: AppHandle,
    state: State<ConfigState>,
    key: String,
    hidden: bool,
) -> Result<LibraryConfig, AppError> {
    change(&app, &state, |library| {
        library.set_hidden(&key, hidden);
        Ok(())
    })
}

#[tauri::command(async)]
pub fn create_category(
    app: AppHandle,
    state: State<ConfigState>,
    name: String,
) -> Result<LibraryConfig, AppError> {
    change(&app, &state, |library| {
        library.create_category(&name).map(|_| ())
    })
}

#[tauri::command(async)]
pub fn rename_category(
    app: AppHandle,
    state: State<ConfigState>,
    id: String,
    name: String,
) -> Result<LibraryConfig, AppError> {
    change(&app, &state, |library| library.rename_category(&id, &name))
}

#[tauri::command(async)]
pub fn delete_category(
    app: AppHandle,
    state: State<ConfigState>,
    id: String,
) -> Result<LibraryConfig, AppError> {
    change(&app, &state, |library| library.delete_category(&id))
}

/// Records how a library game ran when tested, or (`status` left out) takes
/// the result away again.
#[tauri::command(async)]
pub fn set_test_status(
    app: AppHandle,
    state: State<ConfigState>,
    game_id: String,
    status: Option<TestStatus>,
    note: String,
) -> Result<LibraryConfig, AppError> {
    let game = state
        .locked()
        .games
        .iter()
        .find(|g| g.id.to_string() == game_id)
        .cloned()
        .ok_or_else(|| format!("No game with id {game_id}"))?;
    change(&app, &state, |library| {
        library.set_tested(&game, status, &note, now_secs());
        Ok(())
    })
}

/// Adds the game to one category or takes it out, rather than setting all
/// of its categories at once: quick clicks on two categories in a row then
/// can't undo each other.
#[tauri::command(async)]
pub fn set_in_category(
    app: AppHandle,
    state: State<ConfigState>,
    id: String,
    key: String,
    member: bool,
) -> Result<LibraryConfig, AppError> {
    change(&app, &state, |library| {
        library.set_in_category(&id, &[key], member)
    })
}

#[cfg(test)]
mod tests;
