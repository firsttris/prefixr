use crate::error::AppError;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

use crate::commands::steamgriddb::{read_api_key, search_steam_apps, steam_apps, SteamApp};
use crate::config::ConfigState;

/// The umu-database (https://github.com/Open-Wine-Components/umu-database)
/// maps games from other stores to the UMU id that umu-protonfixes keys its
/// fixes by, e.g. Cyberpunk 2077 on GOG → `umu-1091500`. Without a request
/// parameter its API returns the whole table (~1200 rows, ~200 KB), which is
/// cached and searched locally, so typing a search costs no requests.
///
/// Games on Steam don't need an entry there at all: their UMU id is simply
/// `umu-<Steam app id>`, which SteamGridDB can tell us (see
/// `search_umu_ids`).
const DATABASE_URL: &str = "https://umu.openwinecomponents.org/umu_api.php";
const CACHE_MAX_AGE: Duration = Duration::from_secs(24 * 60 * 60);
const MAX_RESULTS: usize = 15;
/// How many SteamGridDB name matches get their Steam app id looked up, one
/// request each.
const STEAM_LOOKUPS: usize = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DatabaseEntry {
    #[serde(default)]
    title: Option<String>,
    umu_id: String,
    #[serde(default)]
    acronym: Option<String>,
    #[serde(default)]
    store: Option<String>,
}

/// Where a suggestion came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum UmuMatchSource {
    /// The Steam app id SteamGridDB has on record for the game.
    Steam,
    Database,
}

/// A suggested UMU id for a game, as surfaced to the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct UmuMatch {
    pub umu_id: String,
    pub title: String,
    /// The store this entry is for (`gog`, `egs`, …); `None` for Steam and
    /// standalone releases.
    pub store: Option<String>,
    pub source: UmuMatchSource,
}

fn cache_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_cache_dir()
        .map_err(|e| format!("Could not resolve cache directory: {e}"))?
        .join("umu-database.json"))
}

fn read_cache(path: &PathBuf) -> Option<(Vec<DatabaseEntry>, Duration)> {
    let age = fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|modified| SystemTime::now().duration_since(modified).ok())
        .unwrap_or(Duration::MAX);
    let entries = serde_json::from_slice(&fs::read(path).ok()?).ok()?;
    Some((entries, age))
}

async fn download_database() -> Result<Vec<DatabaseEntry>, String> {
    let response = crate::http::client()
        .get(DATABASE_URL)
        .send()
        .await
        .map_err(|e| format!("Could not reach the umu-database: {e}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "umu-database returned status {}",
            response.status()
        ));
    }
    response
        .json()
        .await
        .map_err(|e| format!("Could not parse the umu-database: {e}"))
}

/// The cached table, refreshed once a day. A stale copy still beats none
/// when the download fails, e.g. offline.
async fn load_database(app: &AppHandle) -> Result<Vec<DatabaseEntry>, String> {
    let path = cache_path(app)?;
    let cached = read_cache(&path);
    if let Some((entries, age)) = &cached {
        if *age < CACHE_MAX_AGE {
            return Ok(entries.clone());
        }
    }
    match download_database().await {
        Ok(entries) => {
            if let Some(dir) = path.parent() {
                let _ = fs::create_dir_all(dir);
            }
            if let Ok(json) = serde_json::to_vec(&entries) {
                let _ = fs::write(&path, json);
            }
            Ok(entries)
        }
        Err(e) => cached.map(|(entries, _)| entries).ok_or(e),
    }
}

/// Lowercase words, punctuation dropped: "DOOM: The Dark Ages" and
/// "doom the dark ages" compare equal.
fn normalize(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// How well an entry matches the (normalized) query; `None` for no match.
fn score(entry: &DatabaseEntry, query: &str) -> Option<u32> {
    let title = normalize(entry.title.as_deref()?);
    let acronym = entry.acronym.as_deref().map(normalize).unwrap_or_default();
    if title == query {
        Some(100)
    } else if !acronym.is_empty() && acronym == query.replace(' ', "") {
        Some(90)
    } else if title.starts_with(query) {
        Some(80)
    } else if title.contains(query) {
        Some(60)
    } else if query.split(' ').all(|word| title.split(' ').any(|t| t == word)) {
        Some(40)
    } else {
        None
    }
}

fn search_database(entries: &[DatabaseEntry], query: &str) -> Vec<UmuMatch> {
    let query = normalize(query);
    if query.is_empty() {
        return Vec::new();
    }
    let mut scored: Vec<(u32, &DatabaseEntry)> = entries
        .iter()
        .filter_map(|entry| Some((score(entry, &query)?, entry)))
        .collect();
    scored.sort_by(|(a_score, a), (b_score, b)| {
        b_score.cmp(a_score).then_with(|| a.title.cmp(&b.title))
    });
    scored
        .into_iter()
        .map(|(_, entry)| UmuMatch {
            umu_id: entry.umu_id.clone(),
            title: entry.title.clone().unwrap_or_default(),
            store: crate::models::normalize_umu_store(entry.store.clone()),
            source: UmuMatchSource::Database,
        })
        .collect()
}

/// Steam matches first — they're the most direct mapping — then the
/// database's, dropping repeats of the same id for the same store (the
/// database lists one row per store *codename*, so the same game can appear
/// several times).
fn merge(steam: Vec<SteamApp>, database: Vec<UmuMatch>) -> Vec<UmuMatch> {
    let steam = steam.into_iter().map(|app| UmuMatch {
        umu_id: format!("umu-{}", app.app_id),
        title: app.name,
        store: None,
        source: UmuMatchSource::Steam,
    });
    let mut seen = HashSet::new();
    steam
        .chain(database)
        .filter(|m| seen.insert((m.umu_id.clone(), m.store.clone())))
        .take(MAX_RESULTS)
        .collect()
}

/// Suggests UMU ids for a game: the Steam app id of its SteamGridDB match
/// (the one picked for its artwork if there is one, otherwise the best name
/// matches), plus the umu-database's entries for the name. SteamGridDB is
/// optional — without an API key, or when it fails, only the database is
/// searched.
#[tauri::command]
pub async fn search_umu_ids(
    app: AppHandle,
    state: State<'_, ConfigState>,
    query: String,
    steamgriddb_id: Option<i64>,
) -> Result<Vec<UmuMatch>, AppError> {
    let query = query.trim();
    let api_key = read_api_key(&state)?;

    let steam = async {
        let Some(key) = api_key.as_deref() else {
            return Vec::new();
        };
        let result = match steamgriddb_id {
            Some(id) => steam_apps(key, id).await,
            None if !query.is_empty() => search_steam_apps(key, query, STEAM_LOOKUPS).await,
            None => Ok(Vec::new()),
        };
        result.unwrap_or_default()
    };
    let (steam, database) = futures_util::join!(steam, load_database(&app));

    let database = match database {
        Ok(entries) => search_database(&entries, query),
        // Still worth showing what SteamGridDB found.
        Err(_) if !steam.is_empty() => Vec::new(),
        Err(e) => return Err(e.into()),
    };
    Ok(merge(steam, database))
}

#[cfg(test)]
mod tests;
