use std::fs;
use std::path::{Path, PathBuf};

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};
use uuid::Uuid;

use crate::config::{save_config, ConfigState};
use crate::models::{Game, SteamGridDbConfig};

const BASE_URL: &str = "https://www.steamgriddb.com/api/v2";

/// A SteamGridDB API envelope. Data-bearing endpoints wrap their payload in
/// `data`; failures set `success: false` and list human-readable reasons in
/// `errors` instead of using an HTTP error status.
#[derive(Debug, Deserialize)]
#[serde(bound(deserialize = "T: Deserialize<'de>"))]
struct SgdbEnvelope<T> {
    success: bool,
    #[serde(default)]
    data: Vec<T>,
    #[serde(default)]
    errors: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct SgdbGameMatch {
    id: i64,
    name: String,
    #[serde(default)]
    verified: bool,
}

#[derive(Debug, Deserialize)]
struct SgdbGrid {
    id: i64,
    url: String,
    thumb: String,
    width: u32,
    height: u32,
}

/// A SteamGridDB game search match, as surfaced to the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct SteamGridDbGameMatch {
    pub id: i64,
    pub name: String,
    pub verified: bool,
}

/// A SteamGridDB grid (cover art) option, as surfaced to the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct SteamGridDbGrid {
    pub id: i64,
    pub url: String,
    pub thumb: String,
    pub width: u32,
    pub height: u32,
}

/// Builds a SteamGridDB API URL from path segments, percent-encoding each
/// segment (so a game title with spaces/punctuation is safe to embed in the
/// search path without callers having to think about it).
fn build_url(path_segments: &[&str]) -> Result<reqwest::Url, String> {
    let mut url =
        reqwest::Url::parse(BASE_URL).map_err(|e| format!("Invalid SteamGridDB URL: {e}"))?;
    url.path_segments_mut()
        .map_err(|_| "Invalid SteamGridDB URL".to_string())?
        .extend(path_segments);
    Ok(url)
}

/// Reads the configured API key, without holding the config lock across an
/// `.await` point (a `std::sync::MutexGuard` isn't `Send`).
fn require_api_key(state: &State<ConfigState>) -> Result<String, String> {
    let config = state
        .lock()
        .map_err(|_| "Configuration is locked".to_string())?;
    match &config.steamgriddb.api_key {
        Some(key) if !key.trim().is_empty() => Ok(key.clone()),
        _ => Err("No SteamGridDB API key configured".to_string()),
    }
}

/// Sends a GET request to a SteamGridDB API endpoint and unwraps its
/// envelope, mapping an unsuccessful response's `errors` into the `Err`.
async fn sgdb_get<T: for<'de> Deserialize<'de>>(
    url: reqwest::Url,
    api_key: &str,
) -> Result<Vec<T>, String> {
    let response = reqwest::Client::new()
        .get(url)
        .bearer_auth(api_key)
        .send()
        .await
        .map_err(|e| format!("Could not reach SteamGridDB: {e}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "SteamGridDB API returned status {}",
            response.status()
        ));
    }

    let envelope: SgdbEnvelope<T> = response
        .json()
        .await
        .map_err(|e| format!("Could not parse SteamGridDB response: {e}"))?;

    if !envelope.success {
        return Err(if envelope.errors.is_empty() {
            "SteamGridDB request was not successful".to_string()
        } else {
            envelope.errors.join(", ")
        });
    }

    Ok(envelope.data)
}

#[tauri::command]
pub fn get_steamgriddb_config(state: State<ConfigState>) -> Result<SteamGridDbConfig, String> {
    let config = state
        .lock()
        .map_err(|_| "Configuration is locked".to_string())?;
    Ok(config.steamgriddb.clone())
}

#[tauri::command]
pub fn save_steamgriddb_config(
    app: AppHandle,
    state: State<ConfigState>,
    config: SteamGridDbConfig,
) -> Result<SteamGridDbConfig, String> {
    let mut app_config = state
        .lock()
        .map_err(|_| "Configuration is locked".to_string())?;
    app_config.steamgriddb = config;
    save_config(&app, &app_config)?;
    Ok(app_config.steamgriddb.clone())
}

/// Searches SteamGridDB for games matching `query`, so the user can confirm
/// (or correct) the match before its cover art is fetched.
#[tauri::command]
pub async fn search_steamgriddb_games(
    state: State<'_, ConfigState>,
    query: String,
) -> Result<Vec<SteamGridDbGameMatch>, String> {
    if query.trim().is_empty() {
        return Ok(Vec::new());
    }
    let api_key = require_api_key(&state)?;

    let url = build_url(&["search", "autocomplete", query.trim()])?;
    let matches: Vec<SgdbGameMatch> = sgdb_get(url, &api_key).await?;

    Ok(matches
        .into_iter()
        .map(|m| SteamGridDbGameMatch {
            id: m.id,
            name: m.name,
            verified: m.verified,
        })
        .collect())
}

/// Lists portrait grid ("cover") artwork options for a SteamGridDB game id.
#[tauri::command]
pub async fn list_steamgriddb_grids(
    state: State<'_, ConfigState>,
    steamgriddb_id: i64,
) -> Result<Vec<SteamGridDbGrid>, String> {
    let api_key = require_api_key(&state)?;

    let mut url = build_url(&["grids", "game", &steamgriddb_id.to_string()])?;
    url.query_pairs_mut().append_pair("dimensions", "600x900");
    let grids: Vec<SgdbGrid> = sgdb_get(url, &api_key).await?;

    Ok(grids
        .into_iter()
        .map(|g| SteamGridDbGrid {
            id: g.id,
            url: g.url,
            thumb: g.thumb,
            width: g.width,
            height: g.height,
        })
        .collect())
}

fn artwork_dir(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Could not resolve data directory: {e}"))?
        .join("artwork"))
}

/// The image file extension implied by a SteamGridDB image URL (which always
/// carries a real one), defaulting to `png` for anything unrecognized.
fn cover_extension(url: &str) -> &'static str {
    let last_segment = url
        .split('?')
        .next()
        .unwrap_or(url)
        .rsplit('/')
        .next()
        .unwrap_or(url);
    match last_segment.rsplit_once('.').map(|(_, ext)| ext.to_ascii_lowercase()) {
        Some(ext) if ext == "jpg" || ext == "jpeg" => "jpg",
        Some(ext) if ext == "webp" => "webp",
        _ => "png",
    }
}

fn cover_mime(ext: &str) -> &'static str {
    match ext {
        "jpg" => "image/jpeg",
        "webp" => "image/webp",
        _ => "image/png",
    }
}

fn cover_cache_path(dir: &Path, id: Uuid, ext: &str) -> PathBuf {
    dir.join(format!("{id}.{ext}"))
}

/// Removes any previously cached cover file for `id`, regardless of
/// extension, so switching between differently-formatted covers doesn't
/// leave stale files behind.
fn remove_stale_cover_files(dir: &Path, id: Uuid) -> Result<(), String> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Ok(());
    };
    let prefix = format!("{id}.");
    for entry in entries.filter_map(|e| e.ok()) {
        if entry.file_name().to_string_lossy().starts_with(&prefix) {
            fs::remove_file(entry.path())
                .map_err(|e| format!("Could not remove cached cover: {e}"))?;
        }
    }
    Ok(())
}

fn find_game<'a>(config: &'a mut crate::config::AppConfig, id: &str) -> Result<&'a mut Game, String> {
    let game_id = Uuid::parse_str(id).map_err(|e| format!("Invalid game id: {e}"))?;
    config
        .games
        .iter_mut()
        .find(|g| g.id == game_id)
        .ok_or_else(|| format!("No game with id {id}"))
}

/// Downloads the chosen grid image, caches it to disk, and records the
/// SteamGridDB match on the game so the picker can reopen without
/// re-searching.
#[tauri::command]
pub async fn set_game_cover(
    app: AppHandle,
    state: State<'_, ConfigState>,
    game_id: String,
    steamgriddb_id: i64,
    cover_grid_id: i64,
    image_url: String,
) -> Result<Game, String> {
    let response = reqwest::Client::new()
        .get(&image_url)
        .send()
        .await
        .map_err(|e| format!("Could not download cover: {e}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "Could not download cover (status {})",
            response.status()
        ));
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Could not read cover data: {e}"))?;

    let dir = artwork_dir(&app)?;
    fs::create_dir_all(&dir).map_err(|e| format!("Could not create artwork directory: {e}"))?;

    let game_uuid = Uuid::parse_str(&game_id).map_err(|e| format!("Invalid game id: {e}"))?;
    remove_stale_cover_files(&dir, game_uuid)?;
    let ext = cover_extension(&image_url);
    fs::write(cover_cache_path(&dir, game_uuid, ext), &bytes)
        .map_err(|e| format!("Could not write cover file: {e}"))?;

    let mut config = state
        .lock()
        .map_err(|_| "Configuration is locked".to_string())?;
    let game = find_game(&mut config, &game_id)?;
    game.steamgriddb_id = Some(steamgriddb_id);
    game.cover_grid_id = Some(cover_grid_id);
    game.cover_url = Some(image_url);
    let updated = game.clone();
    save_config(&app, &config)?;
    Ok(updated)
}

/// Reads a game's cached cover, if any, as a `data:image/...;base64,...`
/// URI. Returns `Ok(None)` (rather than an error) both when the game has no
/// cover set and when the cache file is unexpectedly missing, since either
/// case just means the card should fall back to showing no cover.
#[tauri::command]
pub fn get_game_cover(app: AppHandle, state: State<ConfigState>, game_id: String) -> Result<Option<String>, String> {
    let cover_url = {
        let mut config = state
            .lock()
            .map_err(|_| "Configuration is locked".to_string())?;
        let game = find_game(&mut config, &game_id)?;
        match &game.cover_url {
            Some(url) => url.clone(),
            None => return Ok(None),
        }
    };

    let game_uuid = Uuid::parse_str(&game_id).map_err(|e| format!("Invalid game id: {e}"))?;
    let ext = cover_extension(&cover_url);
    let path = cover_cache_path(&artwork_dir(&app)?, game_uuid, ext);
    if !path.exists() {
        return Ok(None);
    }

    let bytes = fs::read(&path).map_err(|e| format!("Could not read cover file: {e}"))?;
    Ok(Some(format!(
        "data:{};base64,{}",
        cover_mime(ext),
        STANDARD.encode(bytes)
    )))
}

/// Clears a game's cover and deletes its cached file. Deliberately keeps
/// `steamgriddb_id` so the picker can jump straight back to that game's
/// grid list rather than making the user search again.
#[tauri::command]
pub fn remove_game_cover(app: AppHandle, state: State<ConfigState>, game_id: String) -> Result<Game, String> {
    let mut config = state
        .lock()
        .map_err(|_| "Configuration is locked".to_string())?;
    let game = find_game(&mut config, &game_id)?;
    let game_uuid = game.id;
    game.cover_url = None;
    game.cover_grid_id = None;
    let updated = game.clone();

    remove_stale_cover_files(&artwork_dir(&app)?, game_uuid)?;
    save_config(&app, &config)?;
    Ok(updated)
}
