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
#[serde(bound(deserialize = "T: Deserialize<'de> + Default"))]
struct SgdbEnvelope<T> {
    success: bool,
    #[serde(default)]
    data: T,
    #[serde(default)]
    errors: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct SgdbGameMatch {
    id: i64,
    name: String,
    #[serde(default)]
    verified: bool,
    /// Platforms SteamGridDB knows the game on, e.g. `steam`.
    #[serde(default)]
    types: Vec<String>,
}

/// A game looked up by id with `platformdata=steam`.
#[derive(Debug, Default, Deserialize)]
struct SgdbGame {
    name: String,
    #[serde(default)]
    external_platform_data: SgdbPlatformData,
}

#[derive(Debug, Default, Deserialize)]
struct SgdbPlatformData {
    #[serde(default)]
    steam: Vec<SgdbPlatformEntry>,
}

#[derive(Debug, Deserialize)]
struct SgdbPlatformEntry {
    id: String,
}

/// Shape shared by SteamGridDB's grid and icon endpoints — both return the
/// same asset fields, just for different image kinds.
#[derive(Debug, Deserialize)]
struct SgdbAsset {
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

/// A SteamGridDB image asset option (a grid/cover or an icon), as surfaced
/// to the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct SteamGridDbGrid {
    pub id: i64,
    pub url: String,
    pub thumb: String,
    pub width: u32,
    pub height: u32,
}

impl From<SgdbAsset> for SteamGridDbGrid {
    fn from(a: SgdbAsset) -> Self {
        SteamGridDbGrid {
            id: a.id,
            url: a.url,
            thumb: a.thumb,
            width: a.width,
            height: a.height,
        }
    }
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
pub(crate) fn read_api_key(state: &State<ConfigState>) -> Result<Option<String>, String> {
    let config = state
        .lock()
        .map_err(|_| "Configuration is locked".to_string())?;
    Ok(config
        .steamgriddb
        .api_key
        .clone()
        .filter(|key| !key.trim().is_empty()))
}

fn require_api_key(state: &State<ConfigState>) -> Result<String, String> {
    read_api_key(state)?.ok_or_else(|| "No SteamGridDB API key configured".to_string())
}

/// Sends a GET request to a SteamGridDB API endpoint and unwraps its
/// envelope, mapping an unsuccessful response's `errors` into the `Err`.
async fn sgdb_get<T: for<'de> Deserialize<'de> + Default>(
    url: reqwest::Url,
    api_key: &str,
) -> Result<T, String> {
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

/// A Steam release of a SteamGridDB game.
#[derive(Debug, Clone)]
pub(crate) struct SteamApp {
    pub app_id: String,
    /// SteamGridDB's name for the game.
    pub name: String,
}

/// The Steam app ids SteamGridDB has on record for a game — usually one,
/// none for games that were never on Steam.
pub(crate) async fn steam_apps(api_key: &str, steamgriddb_id: i64) -> Result<Vec<SteamApp>, String> {
    let mut url = build_url(&["games", "id", &steamgriddb_id.to_string()])?;
    url.query_pairs_mut().append_pair("platformdata", "steam");
    let game: SgdbGame = sgdb_get(url, api_key).await?;
    Ok(game
        .external_platform_data
        .steam
        .into_iter()
        .map(|entry| SteamApp {
            app_id: entry.id,
            name: game.name.clone(),
        })
        .collect())
}

/// Searches SteamGridDB by name and looks up the Steam app ids of the first
/// `limit` matches that are on Steam at all.
pub(crate) async fn search_steam_apps(
    api_key: &str,
    query: &str,
    limit: usize,
) -> Result<Vec<SteamApp>, String> {
    let url = build_url(&["search", "autocomplete", query])?;
    let matches: Vec<SgdbGameMatch> = sgdb_get(url, api_key).await?;
    let lookups = matches
        .iter()
        .filter(|m| m.types.iter().any(|t| t == "steam"))
        .take(limit)
        .map(|m| steam_apps(api_key, m.id));
    // A failed lookup only costs that one suggestion.
    Ok(futures_util::future::join_all(lookups)
        .await
        .into_iter()
        .filter_map(Result::ok)
        .flatten()
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
    let grids: Vec<SgdbAsset> = sgdb_get(url, &api_key).await?;

    Ok(grids.into_iter().map(SteamGridDbGrid::from).collect())
}

/// Lists icon artwork options for a SteamGridDB game id.
#[tauri::command]
pub async fn list_steamgriddb_icons(
    state: State<'_, ConfigState>,
    steamgriddb_id: i64,
) -> Result<Vec<SteamGridDbGrid>, String> {
    let api_key = require_api_key(&state)?;

    let url = build_url(&["icons", "game", &steamgriddb_id.to_string()])?;
    let icons: Vec<SgdbAsset> = sgdb_get(url, &api_key).await?;

    Ok(icons.into_iter().map(SteamGridDbGrid::from).collect())
}

pub(crate) fn artwork_dir(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Could not resolve data directory: {e}"))?
        .join("artwork"))
}

/// The image file extension implied by a SteamGridDB image URL (which always
/// carries a real one), defaulting to `png` for anything unrecognized.
pub(crate) fn image_extension(url: &str) -> &'static str {
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

fn image_mime(ext: &str) -> &'static str {
    match ext {
        "jpg" => "image/jpeg",
        "webp" => "image/webp",
        _ => "image/png",
    }
}

/// The cache path for a game's asset of a given `kind` ("" for the cover,
/// kept suffix-less for backward compatibility with already-cached files;
/// "_icon" etc. for anything added since).
pub(crate) fn asset_cache_path(dir: &Path, id: Uuid, kind: &str, ext: &str) -> PathBuf {
    dir.join(format!("{id}{kind}.{ext}"))
}

/// Removes any previously cached asset file of `kind` for `id`, regardless
/// of extension, so switching between differently-formatted images doesn't
/// leave stale files behind.
fn remove_stale_asset_files(dir: &Path, id: Uuid, kind: &str) -> Result<(), String> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Ok(());
    };
    let prefix = format!("{id}{kind}.");
    for entry in entries.filter_map(|e| e.ok()) {
        if entry.file_name().to_string_lossy().starts_with(&prefix) {
            fs::remove_file(entry.path())
                .map_err(|e| format!("Could not remove cached artwork: {e}"))?;
        }
    }
    Ok(())
}

/// Downloads an artwork image from its source URL.
async fn download_image(image_url: &str) -> Result<Vec<u8>, String> {
    let response = reqwest::Client::new()
        .get(image_url)
        .send()
        .await
        .map_err(|e| format!("Could not download image: {e}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "Could not download image (status {})",
            response.status()
        ));
    }
    Ok(response
        .bytes()
        .await
        .map_err(|e| format!("Could not read image data: {e}"))?
        .to_vec())
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
    let bytes = download_image(&image_url).await?;

    let dir = artwork_dir(&app)?;
    fs::create_dir_all(&dir).map_err(|e| format!("Could not create artwork directory: {e}"))?;

    let game_uuid = Uuid::parse_str(&game_id).map_err(|e| format!("Invalid game id: {e}"))?;
    remove_stale_asset_files(&dir, game_uuid, "")?;
    let ext = image_extension(&image_url);
    fs::write(asset_cache_path(&dir, game_uuid, "", ext), &bytes)
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
    let ext = image_extension(&cover_url);
    let path = asset_cache_path(&artwork_dir(&app)?, game_uuid, "", ext);
    if !path.exists() {
        return Ok(None);
    }

    let bytes = fs::read(&path).map_err(|e| format!("Could not read cover file: {e}"))?;
    Ok(Some(format!(
        "data:{};base64,{}",
        image_mime(ext),
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

    remove_stale_asset_files(&artwork_dir(&app)?, game_uuid, "")?;
    save_config(&app, &config)?;
    Ok(updated)
}

/// Downloads the chosen icon image, caches it to disk, and records the
/// SteamGridDB match on the game so the picker can reopen without
/// re-searching.
#[tauri::command]
pub async fn set_game_icon(
    app: AppHandle,
    state: State<'_, ConfigState>,
    game_id: String,
    steamgriddb_id: i64,
    icon_grid_id: i64,
    image_url: String,
) -> Result<Game, String> {
    let bytes = download_image(&image_url).await?;

    let dir = artwork_dir(&app)?;
    fs::create_dir_all(&dir).map_err(|e| format!("Could not create artwork directory: {e}"))?;

    let game_uuid = Uuid::parse_str(&game_id).map_err(|e| format!("Invalid game id: {e}"))?;
    remove_stale_asset_files(&dir, game_uuid, "_icon")?;
    let ext = image_extension(&image_url);
    fs::write(asset_cache_path(&dir, game_uuid, "_icon", ext), &bytes)
        .map_err(|e| format!("Could not write icon file: {e}"))?;

    let mut config = state
        .lock()
        .map_err(|_| "Configuration is locked".to_string())?;
    let game = find_game(&mut config, &game_id)?;
    game.steamgriddb_id = Some(steamgriddb_id);
    game.steamgriddb_icon_grid_id = Some(icon_grid_id);
    game.steamgriddb_icon_url = Some(image_url);
    let updated = game.clone();
    save_config(&app, &config)?;
    Ok(updated)
}

/// Reads a game's cached icon, if any, as a `data:image/...;base64,...`
/// URI. Returns `Ok(None)` both when the game has no icon set and when the
/// cache file is unexpectedly missing.
#[tauri::command]
pub fn get_game_icon(app: AppHandle, state: State<ConfigState>, game_id: String) -> Result<Option<String>, String> {
    let icon_url = {
        let mut config = state
            .lock()
            .map_err(|_| "Configuration is locked".to_string())?;
        let game = find_game(&mut config, &game_id)?;
        match &game.steamgriddb_icon_url {
            Some(url) => url.clone(),
            None => return Ok(None),
        }
    };

    let game_uuid = Uuid::parse_str(&game_id).map_err(|e| format!("Invalid game id: {e}"))?;
    let ext = image_extension(&icon_url);
    let path = asset_cache_path(&artwork_dir(&app)?, game_uuid, "_icon", ext);
    if !path.exists() {
        return Ok(None);
    }

    let bytes = fs::read(&path).map_err(|e| format!("Could not read icon file: {e}"))?;
    Ok(Some(format!(
        "data:{};base64,{}",
        image_mime(ext),
        STANDARD.encode(bytes)
    )))
}

/// Clears a game's SteamGridDB icon and deletes its cached file. Keeps
/// `steamgriddb_id` for the same reason `remove_game_cover` does.
#[tauri::command]
pub fn remove_game_icon(app: AppHandle, state: State<ConfigState>, game_id: String) -> Result<Game, String> {
    let mut config = state
        .lock()
        .map_err(|_| "Configuration is locked".to_string())?;
    let game = find_game(&mut config, &game_id)?;
    let game_uuid = game.id;
    game.steamgriddb_icon_url = None;
    game.steamgriddb_icon_grid_id = None;
    let updated = game.clone();

    remove_stale_asset_files(&artwork_dir(&app)?, game_uuid, "_icon")?;
    save_config(&app, &config)?;
    Ok(updated)
}
