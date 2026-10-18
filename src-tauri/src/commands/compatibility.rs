//! How well a game is known to run on Linux, shown before it's installed:
//! whether umu-protonfixes has fixes for it (the umu-database knows the
//! store game, or the game has a UMU id set), and its ProtonDB rating,
//! which needs a Steam app id, i.e. a `umu-<Steam app id>` UMU id.
//!
//! ProtonDB ratings are cached for a week in `protondb.json` under the
//! cache dir; a library opening costs no requests then.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::PathBuf;

use futures_util::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

use crate::commands::library::{game_key, now_secs};
use crate::commands::umu_database::umu_ids_for_store_games;
use crate::config::ConfigState;
use crate::error::AppError;
use crate::lock::LockExt;
use crate::models::StoreKind;

const PROTONDB_URL: &str = "https://www.protondb.com/api/v1/reports/summaries";
const CACHE_MAX_AGE_SECS: u64 = 7 * 24 * 60 * 60;
/// A game ProtonDB has no reports for is asked about again after a day.
const MISSING_MAX_AGE_SECS: u64 = 24 * 60 * 60;
const PARALLEL_REQUESTS: usize = 6;

/// What's known about a game's compatibility, for the library.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct Compatibility {
    /// The UMU id umu-protonfixes knows the game by, if any.
    pub umu_id: Option<String>,
    /// The store of that id (`gog`, `egs`, …).
    pub umu_store: Option<String>,
    /// `platinum`, `gold`, `silver`, `bronze` or `borked`, as ProtonDB has it.
    pub protondb_tier: Option<String>,
    pub steam_app_id: Option<u32>,
}

/// The Steam app id in a UMU id: `umu-1091500` → 1091500.
pub fn steam_app_id(umu_id: &str) -> Option<u32> {
    umu_id.strip_prefix("umu-")?.parse().ok()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedTier {
    tier: Option<String>,
    fetched_at: u64,
}

#[derive(Deserialize)]
struct Summary {
    tier: Option<String>,
}

/// The rating in a ProtonDB summary; `pending` (too few reports) counts as
/// none.
fn parse_tier(json: &str) -> Option<String> {
    let tier = serde_json::from_str::<Summary>(json).ok()?.tier?;
    let tier = tier.trim().to_ascii_lowercase();
    matches!(
        tier.as_str(),
        "platinum" | "gold" | "silver" | "bronze" | "borked"
    )
    .then_some(tier)
}

fn fresh(cached: &CachedTier, now: u64) -> bool {
    let max_age = if cached.tier.is_some() {
        CACHE_MAX_AGE_SECS
    } else {
        MISSING_MAX_AGE_SECS
    };
    now.saturating_sub(cached.fetched_at) < max_age
}

fn cache_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_cache_dir()
        .map_err(|e| format!("Could not resolve cache directory: {e}"))?
        .join("protondb.json"))
}

fn read_cache(path: &PathBuf) -> BTreeMap<u32, CachedTier> {
    fs::read(path)
        .ok()
        .and_then(|raw| serde_json::from_slice(&raw).ok())
        .unwrap_or_default()
}

/// `None` when ProtonDB couldn't be asked (offline): not cached then.
async fn fetch_tier(app_id: u32) -> Option<Option<String>> {
    let response = crate::http::client()
        .get(format!("{PROTONDB_URL}/{app_id}.json"))
        .send()
        .await
        .ok()?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Some(None);
    }
    if !response.status().is_success() {
        return None;
    }
    Some(parse_tier(&response.text().await.ok()?))
}

/// The ProtonDB ratings of these Steam apps, from the cache or ProtonDB.
async fn protondb_tiers(app: &AppHandle, app_ids: Vec<u32>) -> HashMap<u32, String> {
    let path = cache_path(app).ok();
    let mut cache = path.as_ref().map(read_cache).unwrap_or_default();
    let now = now_secs();
    let missing: Vec<u32> = app_ids
        .iter()
        .copied()
        .filter(|id| !cache.get(id).is_some_and(|cached| fresh(cached, now)))
        .collect();
    let fetched: Vec<(u32, Option<Option<String>>)> = stream::iter(missing)
        .map(|id| async move { (id, fetch_tier(id).await) })
        .buffer_unordered(PARALLEL_REQUESTS)
        .collect()
        .await;
    let mut changed = false;
    for (id, tier) in fetched {
        if let Some(tier) = tier {
            cache.insert(
                id,
                CachedTier {
                    tier,
                    fetched_at: now,
                },
            );
            changed = true;
        }
    }
    if let (true, Some(path)) = (changed, path) {
        if let Some(dir) = path.parent() {
            let _ = fs::create_dir_all(dir);
        }
        if let Ok(json) = serde_json::to_vec(&cache) {
            let _ = fs::write(&path, json);
        }
    }
    app_ids
        .into_iter()
        .filter_map(|id| Some((id, cache.get(&id)?.tier.clone()?)))
        .collect()
}

/// The library key of a store game (`epic:<app>`) split up.
fn parse_store_key(key: &str) -> Option<(StoreKind, String)> {
    let (store, app_name) = key.split_once(':')?;
    let store = StoreKind::ALL.into_iter().find(|s| s.slug() == store)?;
    Some((store, app_name.to_string()))
}

/// The compatibility of the games with these library keys. A game's own
/// UMU id wins over the umu-database's entry for its store game. Best-effort:
/// a game nothing is known about is left out.
#[tauri::command]
pub async fn get_compatibility(
    app: AppHandle,
    state: State<'_, ConfigState>,
    keys: Vec<String>,
) -> Result<HashMap<String, Compatibility>, AppError> {
    let own: HashMap<String, (String, Option<String>)> = state
        .locked()
        .games
        .iter()
        .filter_map(|game| {
            Some((
                game_key(game),
                (game.umu_id.clone()?, game.umu_store.clone()),
            ))
        })
        .collect();
    let store_games: Vec<(StoreKind, String)> = keys
        .iter()
        .filter(|key| !own.contains_key(*key))
        .filter_map(|key| parse_store_key(key))
        .collect();
    // Offline with no cached copy: only the games' own ids then.
    let database = umu_ids_for_store_games(&app, &store_games)
        .await
        .unwrap_or_default();

    let mut result: HashMap<String, Compatibility> = HashMap::new();
    for key in &keys {
        let umu = own.get(key).cloned().or_else(|| {
            let (umu_id, store) = database.get(&parse_store_key(key)?)?.clone();
            Some((umu_id, Some(store)))
        });
        if let Some((umu_id, umu_store)) = umu {
            result.insert(
                key.clone(),
                Compatibility {
                    steam_app_id: steam_app_id(&umu_id),
                    umu_id: Some(umu_id),
                    umu_store,
                    protondb_tier: None,
                },
            );
        }
    }

    let mut app_ids: Vec<u32> = result.values().filter_map(|c| c.steam_app_id).collect();
    app_ids.sort_unstable();
    app_ids.dedup();
    let tiers = protondb_tiers(&app, app_ids).await;
    for compatibility in result.values_mut() {
        compatibility.protondb_tier = compatibility
            .steam_app_id
            .and_then(|id| tiers.get(&id).cloned());
    }
    Ok(result)
}

#[cfg(test)]
mod tests;
