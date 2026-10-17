//! Store accounts: games from a connected store show up in the library,
//! can be installed from there, and run like any other game.
//!
//! Prefixr does the part it is good at — the prefix, the runner, the launch
//! — and leaves the store's own protocol to a command line tool that's made
//! for it (`helpers`): logging in, listing the account's games, downloading
//! and installing them, and the account's arguments every launch of an
//! online game needs. Each store is a module of its own (`epic`); what the
//! frontend sees is the same for all of them.
//!
//! Only Epic is supported so far. GOG (gogdl) and Amazon (Nile) would add a
//! `StoreKind` variant, a module next to `epic` and their pinned tool.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use futures_util::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use uuid::Uuid;

use crate::commands::games::{remove_game, LaunchingGames};
use crate::commands::icons::{extract_icon_png, save_exe_icon};
use crate::commands::logs::{new_log_file, store_log_dir};
use crate::commands::prefixes::{effective_prefix_path, known_prefix};
use crate::commands::steamgriddb::{
    artwork_dir, asset_cache_path, download_image_unchecked, image_extension,
    remove_stale_asset_files,
};
use crate::config::{save_config, ConfigState};
use crate::error::AppError;
use crate::host_env::xdg_open;
use crate::lock::LockExt;
use crate::models::{Game, PrefixInfo, StoreKind, StoreLink};

pub mod epic;
pub mod gog;
pub mod helpers;

use epic::Legendary;
use gog::Gogdl;

/// A game of a store account, as listed by the store's tool.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoreLibraryEntry {
    /// The store's id for it.
    pub app_name: String,
    pub title: String,
    /// The store's cover image, if it has one Prefixr can use.
    #[serde(default)]
    pub cover_url: Option<String>,
}

/// A game of a connected store account, for the library.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct StoreGame {
    pub store: StoreKind,
    pub app_name: String,
    pub title: String,
    /// The cached cover file, loaded through the asset protocol.
    pub cover_path: Option<String>,
    /// The library game it was installed as; `None` while not installed.
    pub game_id: Option<String>,
}

/// A store's connection, for the settings page.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct StoreAccount {
    pub store: StoreKind,
    /// Logged in with an account.
    pub connected: bool,
    pub account_name: Option<String>,
    /// How many games the last library sync found.
    pub game_count: u32,
    /// The version of the store's tool Prefixr downloads.
    pub tool_name: String,
    pub tool_version: String,
    /// Where the tool's source is (it's GPL-3.0, like Prefixr).
    pub tool_source_url: String,
}

/// Sizes in bytes, as far as the store tells them.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct StoreInstallInfo {
    pub download_size: Option<u64>,
    pub disk_size: Option<u64>,
}

/// What the install dialog proposes before it asks anything.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct StoreInstallDefaults {
    pub install_dir: String,
    /// Where a new prefix of its own would go.
    pub new_prefix_path: String,
    pub sizes: StoreInstallInfo,
}

/// The choices of the install dialog.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct StoreInstallOptions {
    /// The folder the game's own folder is created in.
    pub install_dir: String,
    /// An existing prefix, or `None` for a new one of its own.
    #[serde(default)]
    pub prefix_path: Option<String>,
    pub runner_id: String,
}

/// How a store game starts, from the store's tool.
#[derive(Debug, Clone, PartialEq)]
pub struct StoreLaunch {
    pub exe_path: PathBuf,
    /// The game's own arguments, before the user's launch arguments.
    pub game_args: Vec<String>,
    /// The account's arguments (tokens), after the user's.
    pub account_args: Vec<String>,
    pub working_dir: PathBuf,
    pub env: Vec<(String, String)>,
}

#[derive(Clone, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct StoreInstallFinishedPayload {
    pub store: StoreKind,
    pub app_name: String,
}

#[derive(Clone, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct StoreInstallProgressPayload {
    pub store: StoreKind,
    pub app_name: String,
    /// 0–100; 0 until the download itself has begun.
    pub percent: f64,
}

/// A running install, so a second one isn't started for the same game and
/// a reloaded window can show the ones still going.
struct ActiveInstall {
    pid: Option<u32>,
    percent: f64,
    cancelled: bool,
}

/// The installs in progress, by store and game.
#[derive(Default)]
pub struct StoreInstalls(Mutex<HashMap<(StoreKind, String), ActiveInstall>>);

fn data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map_err(|e| format!("Could not resolve data directory: {e}"))
}

/// A store's own folder under the data dir: its tool's config and Prefixr's
/// cache of its library.
fn store_dir(app: &AppHandle, store: StoreKind) -> Result<PathBuf, String> {
    Ok(data_dir(app)?.join("stores").join(store.slug()))
}

fn library_cache_path(dir: &Path) -> PathBuf {
    dir.join("library.json")
}

/// The pinned tool a store runs on.
fn tool(store: StoreKind) -> &'static helpers::Helper {
    match store {
        StoreKind::Epic => &helpers::LEGENDARY,
        StoreKind::Gog => &helpers::GOGDL,
    }
}

/// Legendary, downloaded first if needed.
async fn legendary(app: &AppHandle) -> Result<Legendary, AppError> {
    let binary = helpers::ensure(app, &helpers::LEGENDARY).await?;
    Ok(Legendary {
        binary,
        config_dir: store_dir(app, StoreKind::Epic)?.join("legendary"),
    })
}

/// Legendary if it's downloaded already; a store that was never connected
/// isn't connected, and asking it shouldn't download anything.
fn legendary_if_installed(app: &AppHandle) -> Result<Option<Legendary>, String> {
    let Some(binary) = helpers::installed(app, &helpers::LEGENDARY) else {
        return Ok(None);
    };
    Ok(Some(Legendary {
        binary,
        config_dir: store_dir(app, StoreKind::Epic)?.join("legendary"),
    }))
}

/// gogdl, downloaded first if needed.
async fn gogdl(app: &AppHandle) -> Result<Gogdl, AppError> {
    let binary = helpers::ensure(app, &helpers::GOGDL).await?;
    Ok(Gogdl {
        binary,
        config_dir: store_dir(app, StoreKind::Gog)?.join("gogdl"),
    })
}

/// The GOG login's access token, refreshed by gogdl when it expired.
async fn gog_credentials(gogdl: &Gogdl) -> Result<gog::Credentials, AppError> {
    let json = gogdl
        .output(["auth"])
        .await
        .map_err(|e| command_failed(StoreKind::Gog, e))?;
    gog::parse_credentials(&json)?.ok_or(AppError::StoreNotConnected {
        store: StoreKind::Gog.display_name().to_string(),
    })
}

/// The GOG account's name, kept from the login so the settings page needn't
/// reach GOG to show it.
fn gog_account_file(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(store_dir(app, StoreKind::Gog)?.join("account.json"))
}

/// A GOG API's JSON, with the login's token.
async fn gog_get(url: &str, token: &str) -> Result<String, AppError> {
    let response = crate::http::client()
        .get(url)
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| command_failed(StoreKind::Gog, format!("Could not reach GOG: {e}")))?;
    let status = response.status();
    if status == reqwest::StatusCode::UNAUTHORIZED {
        return Err(AppError::StoreNotConnected {
            store: StoreKind::Gog.display_name().to_string(),
        });
    }
    if !status.is_success() {
        return Err(command_failed(
            StoreKind::Gog,
            format!("{url} answered {status}"),
        ));
    }
    response
        .text()
        .await
        .map_err(|e| command_failed(StoreKind::Gog, format!("Could not read GOG's answer: {e}")))
}

fn command_failed(store: StoreKind, message: String) -> AppError {
    let needs_login = match store {
        StoreKind::Epic => epic::needs_login(&message),
        StoreKind::Gog => false,
    };
    if needs_login {
        return AppError::StoreNotConnected {
            store: store.display_name().to_string(),
        };
    }
    AppError::StoreCommandFailed {
        store: store.display_name().to_string(),
        message,
    }
}

/// A name usable in a file name, for a store id that may hold anything.
pub fn file_safe(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// The cache path of a store game's cover in the artwork folder (which the
/// asset protocol may read), e.g. `store-epic-Fortnite.jpg`.
fn cover_cache_path(
    artwork: &Path,
    store: StoreKind,
    entry: &StoreLibraryEntry,
) -> Option<PathBuf> {
    let url = entry.cover_url.as_ref()?;
    Some(artwork.join(format!(
        "store-{}-{}.{}",
        store.slug(),
        file_safe(&entry.app_name),
        image_extension(url)
    )))
}

fn read_library(app: &AppHandle, store: StoreKind) -> Vec<StoreLibraryEntry> {
    store_dir(app, store)
        .ok()
        .and_then(|dir| fs::read(library_cache_path(&dir)).ok())
        .and_then(|raw| serde_json::from_slice(&raw).ok())
        .unwrap_or_default()
}

fn write_library(
    app: &AppHandle,
    store: StoreKind,
    entries: &[StoreLibraryEntry],
) -> Result<(), String> {
    let dir = store_dir(app, store)?;
    fs::create_dir_all(&dir).map_err(|e| format!("Could not create {}: {e}", dir.display()))?;
    let raw = serde_json::to_vec_pretty(entries)
        .map_err(|e| format!("Could not serialize the game list: {e}"))?;
    let path = library_cache_path(&dir);
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, raw)
        .and_then(|()| fs::rename(&tmp, &path))
        .map_err(|e| format!("Could not save the game list: {e}"))
}

/// The library games installed from each store game, by store and id.
fn installed_games(games: &[Game]) -> HashMap<(StoreKind, &str), &Game> {
    games
        .iter()
        .filter_map(|game| {
            let link = game.store.as_ref()?;
            Some(((link.store, link.app_name.as_str()), game))
        })
        .collect()
}

/// A store's games for the library: its cached list, joined with the games
/// installed from it.
pub fn store_games(
    store: StoreKind,
    entries: &[StoreLibraryEntry],
    games: &[Game],
    artwork: Option<&Path>,
) -> Vec<StoreGame> {
    let installed = installed_games(games);
    entries
        .iter()
        .map(|entry| StoreGame {
            store,
            app_name: entry.app_name.clone(),
            title: entry.title.clone(),
            cover_path: artwork
                .and_then(|dir| cover_cache_path(dir, store, entry))
                .filter(|path| path.is_file())
                .map(|path| path.to_string_lossy().into_owned()),
            game_id: installed
                .get(&(store, entry.app_name.as_str()))
                .map(|game| game.id.to_string()),
        })
        .collect()
}

#[tauri::command(async)]
pub fn list_store_games(
    app: AppHandle,
    state: State<ConfigState>,
) -> Result<Vec<StoreGame>, AppError> {
    let games = state.locked().games.clone();
    let artwork = artwork_dir(&app).ok();
    Ok(StoreKind::ALL
        .iter()
        .flat_map(|&store| {
            store_games(
                store,
                &read_library(&app, store),
                &games,
                artwork.as_deref(),
            )
        })
        .collect())
}

async fn account_name(app: &AppHandle, store: StoreKind) -> Result<Option<String>, AppError> {
    match store {
        StoreKind::Epic => {
            let Some(legendary) = legendary_if_installed(app)? else {
                return Ok(None);
            };
            let json = legendary
                .output(["status", "--json", "--offline"])
                .await
                .map_err(|e| command_failed(store, e))?;
            Ok(epic::account_name(&json)?)
        }
        StoreKind::Gog => {
            let dir = store_dir(app, store)?.join("gogdl");
            let logged_in = helpers::installed(app, &helpers::GOGDL).is_some()
                && fs::read_to_string(dir.join("auth.json")).is_ok_and(|raw| gog::has_login(&raw));
            if !logged_in {
                return Ok(None);
            }
            // The name as GOG had it at the login; the login itself counts.
            let name = fs::read_to_string(gog_account_file(app)?)
                .ok()
                .and_then(|raw| gog::parse_username(&raw))
                .unwrap_or_else(|| store.display_name().to_string());
            Ok(Some(name))
        }
    }
}

async fn store_account(app: &AppHandle, store: StoreKind) -> Result<StoreAccount, AppError> {
    let account_name = account_name(app, store).await?;
    let tool = tool(store);
    Ok(StoreAccount {
        store,
        connected: account_name.is_some(),
        account_name,
        game_count: read_library(app, store).len() as u32,
        tool_name: tool.display_name.to_string(),
        tool_version: tool.version.to_string(),
        tool_source_url: tool.source_url.to_string(),
    })
}

#[tauri::command]
pub async fn list_store_accounts(app: AppHandle) -> Result<Vec<StoreAccount>, AppError> {
    let mut accounts = Vec::new();
    for store in StoreKind::ALL {
        accounts.push(store_account(&app, store).await?);
    }
    Ok(accounts)
}

/// Opens the store's login page in the browser. The store's tool is
/// downloaded first, so a missing Python shows up now rather than after the
/// user logged in.
#[tauri::command]
pub async fn open_store_login(app: AppHandle, store: StoreKind) -> Result<(), AppError> {
    match store {
        StoreKind::Epic => {
            legendary(&app).await?;
            xdg_open(epic::LOGIN_URL).await?;
        }
        StoreKind::Gog => {
            gogdl(&app).await?;
            xdg_open(gog::LOGIN_URL).await?;
        }
    }
    Ok(())
}

/// Logs in with the code from the store's login page.
#[tauri::command]
pub async fn connect_store(
    app: AppHandle,
    store: StoreKind,
    code: String,
) -> Result<StoreAccount, AppError> {
    match store {
        StoreKind::Epic => {
            let code = epic::auth_code(&code).ok_or(AppError::StoreInvalidAuthCode)?;
            let legendary = legendary(&app).await?;
            // A failed login logs its error but still exits with 0, so the
            // account is read back to tell.
            legendary
                .output(["auth", "--code", code.as_str()])
                .await
                .map_err(|e| command_failed(store, e))?;
        }
        StoreKind::Gog => {
            let code = gog::auth_code(&code).ok_or(AppError::StoreInvalidAuthCode)?;
            let gogdl = gogdl(&app).await?;
            fs::create_dir_all(&gogdl.config_dir)
                .map_err(|e| format!("Could not create {}: {e}", gogdl.config_dir.display()))?;
            let json = gogdl
                .output(["auth", "--code", code.as_str()])
                .await
                .map_err(|e| command_failed(store, e))?;
            let credentials = gog::parse_credentials(&json)?.ok_or(AppError::StoreLoginFailed)?;
            // Only for showing the name: a login without it still works.
            let profile = format!("https://users.gog.com/users/{}", credentials.user_id);
            if let Ok(json) = gog_get(&profile, &credentials.access_token).await {
                let _ = fs::write(gog_account_file(&app)?, json);
            }
        }
    }
    let account = store_account(&app, store).await?;
    if !account.connected {
        return Err(AppError::StoreLoginFailed);
    }
    Ok(account)
}

/// Logs out and deletes the store's tool and its game list. Games installed
/// from it stay in the library and on disk; starting one connects again
/// first.
#[tauri::command]
pub async fn disconnect_store(
    app: AppHandle,
    state: State<'_, ConfigState>,
    installs: State<'_, StoreInstalls>,
    store: StoreKind,
) -> Result<(), AppError> {
    // An install needs the tool to the end (and the login to download), and
    // the tool would be deleted below while it runs.
    let installing = installs
        .0
        .locked()
        .keys()
        .find(|(s, _)| *s == store)
        .map(|(_, app_name)| app_name.clone());
    if let Some(app_name) = installing {
        let title = read_library(&app, store)
            .into_iter()
            .find(|entry| entry.app_name == app_name)
            .map_or(app_name, |entry| entry.title);
        return Err(AppError::StoreBusy { title });
    }
    match store {
        StoreKind::Epic => {
            if let Some(legendary) = legendary_if_installed(&app)? {
                legendary
                    .output(["auth", "--delete"])
                    .await
                    .map_err(|e| command_failed(store, e))?;
            }
        }
        StoreKind::Gog => {
            // gogdl has no logout; its login is just this file.
            let dir = store_dir(&app, store)?;
            let _ = fs::remove_file(dir.join("gogdl").join("auth.json"));
            let _ = fs::remove_file(gog_account_file(&app)?);
        }
    }
    let entries = read_library(&app, store);
    if let Ok(artwork) = artwork_dir(&app) {
        for entry in &entries {
            if let Some(path) = cover_cache_path(&artwork, store, entry) {
                let _ = fs::remove_file(path);
            }
        }
    }
    if let Ok(dir) = store_dir(&app, store) {
        let _ = fs::remove_file(library_cache_path(&dir));
    }
    // Games installed from the store still start through its tool.
    let still_used = state
        .locked()
        .games
        .iter()
        .any(|game| game.store.as_ref().is_some_and(|link| link.store == store));
    if !still_used {
        helpers::remove(&app, tool(store));
    }
    Ok(())
}

/// How many covers download at once after a sync.
const COVER_DOWNLOADS: usize = 6;

/// Downloads the covers not cached yet. Best-effort: a game without one
/// shows the placeholder.
async fn cache_covers(app: &AppHandle, store: StoreKind, entries: &[StoreLibraryEntry]) {
    let Ok(artwork) = artwork_dir(app) else {
        return;
    };
    if fs::create_dir_all(&artwork).is_err() {
        return;
    }
    let missing: Vec<(String, PathBuf)> = entries
        .iter()
        .filter_map(|entry| {
            let path = cover_cache_path(&artwork, store, entry)?;
            let url = entry.cover_url.clone()?;
            (!path.is_file()).then_some((url, path))
        })
        .collect();
    stream::iter(missing)
        .for_each_concurrent(COVER_DOWNLOADS, |(url, path)| async move {
            if !cover_url_allowed(store, &url) {
                return;
            }
            if let Ok(bytes) = download_image_unchecked(&url).await {
                let tmp = path.with_extension("part");
                if fs::write(&tmp, bytes).is_ok() {
                    let _ = fs::rename(&tmp, &path);
                }
            }
        })
        .await;
}

fn cover_url_allowed(store: StoreKind, url: &str) -> bool {
    match store {
        StoreKind::Epic => epic::is_epic_image_url(url),
        StoreKind::Gog => gog::is_gog_image_url(url),
    }
}

/// Fetches the account's game list from the store and caches it, with
/// the covers.
#[tauri::command]
pub async fn sync_store_library(
    app: AppHandle,
    state: State<'_, ConfigState>,
    store: StoreKind,
) -> Result<Vec<StoreGame>, AppError> {
    let entries = match store {
        StoreKind::Epic => {
            let legendary = legendary(&app).await?;
            let json = legendary
                .output(epic::library_args())
                .await
                .map_err(|e| command_failed(store, e))?;
            epic::parse_library(&json)?
        }
        StoreKind::Gog => gog_library(&app).await?,
    };
    write_library(&app, store, &entries)?;
    cache_covers(&app, store, &entries).await;
    let games = state.locked().games.clone();
    Ok(store_games(
        store,
        &entries,
        &games,
        artwork_dir(&app).ok().as_deref(),
    ))
}

/// How many games' details are fetched from GOG at once during a sync.
const GOG_LOOKUPS: usize = 8;

/// The GOG account's games: the ids from Galaxy's library, page by page,
/// then each one's title and cover from GOG's game database. A game whose
/// details can't be had is left out until the next sync.
async fn gog_library(app: &AppHandle) -> Result<Vec<StoreLibraryEntry>, AppError> {
    let gogdl = gogdl(app).await?;
    let credentials = gog_credentials(&gogdl).await?;
    let mut ids = Vec::new();
    let mut page_token: Option<String> = None;
    loop {
        let mut url = reqwest::Url::parse(&format!(
            "https://galaxy-library.gog.com/users/{}/releases",
            credentials.user_id
        ))
        .map_err(|e| e.to_string())?;
        if let Some(token) = &page_token {
            url.query_pairs_mut().append_pair("page_token", token);
        }
        let json = gog_get(url.as_str(), &credentials.access_token).await?;
        let (page_ids, next) = gog::parse_library_page(&json)?;
        ids.extend(page_ids);
        match next {
            // A token that comes back again would loop forever.
            Some(next) if page_token.as_ref() != Some(&next) => page_token = Some(next),
            _ => break,
        }
    }
    ids.sort();
    ids.dedup();
    let token = credentials.access_token.as_str();
    let mut entries: Vec<StoreLibraryEntry> = stream::iter(ids)
        .map(|id| async move {
            let url = format!("https://gamesdb.gog.com/platforms/gog/external_releases/{id}");
            let json = gog_get(&url, token).await.ok()?;
            gog::parse_gamesdb(&json)
        })
        .buffer_unordered(GOG_LOOKUPS)
        .filter_map(|entry| async move { entry })
        .collect()
        .await;
    entries.sort_by_key(|entry| entry.title.to_lowercase());
    Ok(entries)
}

/// `~/Games/Epic` or `~/Games/GOG`, for a store's first install.
fn default_install_dir(store: StoreKind) -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    let folder = match store {
        StoreKind::Epic => "Epic",
        StoreKind::Gog => "GOG",
    };
    Some(PathBuf::from(home).join("Games").join(folder))
}

/// Where a store game's own prefix goes.
fn new_prefix_path(app: &AppHandle, store: StoreKind, app_name: &str) -> Result<PathBuf, String> {
    Ok(data_dir(app)?
        .join("prefixes")
        .join(format!("{}-{}", store.slug(), file_safe(app_name))))
}

/// What the install dialog starts with: the last install folder, where a
/// new prefix would go, and how big the game is.
#[tauri::command]
pub async fn get_store_install_defaults(
    app: AppHandle,
    state: State<'_, ConfigState>,
    store: StoreKind,
    app_name: String,
) -> Result<StoreInstallDefaults, AppError> {
    let sizes = match store {
        StoreKind::Epic => {
            let legendary = legendary(&app).await?;
            // Sizes are a nice-to-have: without them the dialog still works.
            legendary
                .output(["info", app_name.as_str(), "--json", "--platform", "Windows"])
                .await
                .ok()
                .and_then(|json| epic::parse_install_info(&json).ok())
                .unwrap_or_default()
        }
        StoreKind::Gog => {
            let gogdl = gogdl(&app).await?;
            gogdl
                .output(gog::info_args(&app_name))
                .await
                .ok()
                .and_then(|json| gog::parse_info(&json).ok())
                .map(|plan| plan.sizes)
                .unwrap_or_default()
        }
    };
    let install_dir = state
        .locked()
        .stores
        .install_dirs
        .get(&store)
        .cloned()
        .or_else(|| default_install_dir(store))
        .unwrap_or_default();
    Ok(StoreInstallDefaults {
        install_dir: install_dir.to_string_lossy().into_owned(),
        new_prefix_path: new_prefix_path(&app, store, &app_name)?
            .to_string_lossy()
            .into_owned(),
        sizes,
    })
}

#[tauri::command]
pub fn list_store_installs(installs: State<StoreInstalls>) -> Vec<StoreInstallProgressPayload> {
    installs
        .0
        .locked()
        .iter()
        .map(|((store, app_name), install)| StoreInstallProgressPayload {
            store: *store,
            app_name: app_name.clone(),
            percent: install.percent,
        })
        .collect()
}

/// Stops an install. What was downloaded stays, and installing again
/// continues from there.
#[tauri::command]
pub fn cancel_store_install(
    installs: State<StoreInstalls>,
    store: StoreKind,
    app_name: String,
) -> Result<(), AppError> {
    let mut installs = installs.0.locked();
    let Some(install) = installs.get_mut(&(store, app_name)) else {
        return Ok(());
    };
    install.cancelled = true;
    if let Some(pid) = install.pid {
        // The tool runs in a process group of its own (see
        // `install_store_game`), which includes its download workers.
        unsafe {
            libc::kill(-(pid as libc::pid_t), libc::SIGTERM);
        }
    }
    Ok(())
}

/// Takes `install` out of `StoreInstalls` when the install ends, however
/// it ends.
struct InstallSlot<'a> {
    installs: &'a StoreInstalls,
    key: (StoreKind, String),
}

impl Drop for InstallSlot<'_> {
    fn drop(&mut self) {
        self.installs.0.locked().remove(&self.key);
    }
}

/// How often progress goes to the frontend at most.
const PROGRESS_INTERVAL: Duration = Duration::from_millis(250);

/// Copies a tool's output into its log, line by line, reporting progress
/// lines on the way.
async fn follow_output(
    output: impl AsyncRead + Unpin,
    log: Arc<Mutex<fs::File>>,
    on_progress: impl Fn(f64),
) {
    use std::io::Write;
    let mut lines = BufReader::new(output).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        let _ = writeln!(log.locked(), "{line}");
        if let Some(percent) = epic::parse_progress(&line) {
            on_progress(percent);
        }
    }
}

/// The prefix a store game is installed into: the one picked, or a new one
/// of its own, which is added to the prefix list.
fn install_prefix(
    app: &AppHandle,
    state: &State<'_, ConfigState>,
    store: StoreKind,
    app_name: &str,
    picked: Option<&str>,
) -> Result<PathBuf, AppError> {
    let mut config = state.locked();
    if let Some(path) = picked.filter(|path| !path.trim().is_empty()) {
        return Ok(known_prefix(&config, path)?);
    }
    let path = effective_prefix_path(&new_prefix_path(app, store, app_name)?);
    fs::create_dir_all(&path).map_err(|e| format!("Could not create prefix directory: {e}"))?;
    if !config.prefixes.iter().any(|prefix| prefix.path == path) {
        config.prefixes.push(PrefixInfo { path: path.clone() });
        save_config(app, &config)?;
    }
    Ok(path)
}

/// Puts the store's cover in place as the game's own (see
/// `steamgriddb::Slot::Cover`), so the card keeps it after the install.
fn adopt_cover(app: &AppHandle, store: StoreKind, entry: &StoreLibraryEntry, game: &mut Game) {
    let Ok(artwork) = artwork_dir(app) else {
        return;
    };
    let (Some(url), Some(cached)) = (&entry.cover_url, cover_cache_path(&artwork, store, entry))
    else {
        return;
    };
    if !cached.is_file() || remove_stale_asset_files(&artwork, game.id, "").is_err() {
        return;
    }
    let target = asset_cache_path(&artwork, game.id, "", image_extension(url));
    if fs::copy(&cached, &target).is_ok() {
        game.cover_url = Some(url.clone());
    }
}

/// Downloads and installs a store game and adds it to the library. Progress
/// goes out as `store-install-progress` events; the tool's output goes to a
/// log under `logs/stores/`. One install at a time per store: the tools
/// refuse a second while one runs.
#[tauri::command]
pub async fn install_store_game(
    app: AppHandle,
    state: State<'_, ConfigState>,
    installs: State<'_, StoreInstalls>,
    store: StoreKind,
    app_name: String,
    options: StoreInstallOptions,
) -> Result<Game, AppError> {
    let result = install(&app, &state, &installs, store, &app_name, options).await;
    // However it ended: a window that reloaded meanwhile only knows the
    // install from its progress events, and learns of the end from this.
    let _ = app.emit(
        "store-install-finished",
        StoreInstallFinishedPayload {
            store,
            app_name: app_name.clone(),
        },
    );
    result
}

async fn install(
    app: &AppHandle,
    state: &State<'_, ConfigState>,
    installs: &State<'_, StoreInstalls>,
    store: StoreKind,
    app_name: &str,
    options: StoreInstallOptions,
) -> Result<Game, AppError> {
    let app_name = app_name.to_string();
    let entry = read_library(app, store)
        .into_iter()
        .find(|entry| entry.app_name == app_name)
        .ok_or_else(|| format!("{app_name} is not in the {} library", store.display_name()))?;
    if installed_games(&state.locked().games).contains_key(&(store, app_name.as_str())) {
        return Err(AppError::StoreGameAlreadyInstalled { title: entry.title });
    }
    if options.runner_id.trim().is_empty() {
        return Err("No runner chosen".to_string().into());
    }

    let key = (store, app_name.clone());
    {
        let mut active = installs.0.locked();
        if let Some(((_, other), _)) = active.iter().find(|((s, _), _)| *s == store) {
            let title = read_library(app, store)
                .into_iter()
                .find(|e| &e.app_name == other)
                .map_or_else(|| other.clone(), |e| e.title);
            return Err(AppError::StoreInstallRunning { title });
        }
        active.insert(
            key.clone(),
            ActiveInstall {
                pid: None,
                percent: 0.0,
                cancelled: false,
            },
        );
    }
    let _slot = InstallSlot {
        installs,
        key: key.clone(),
    };

    // A picked prefix is checked now; a new one is only made once the game
    // is installed, so a failed or cancelled install leaves no unused
    // prefix behind.
    if let Some(picked) = options
        .prefix_path
        .as_deref()
        .filter(|p| !p.trim().is_empty())
    {
        known_prefix(&state.locked(), picked)?;
    }
    let install_dir = PathBuf::from(options.install_dir.trim());
    if !install_dir.is_absolute() {
        return Err(format!("Not an absolute folder: {}", install_dir.display()).into());
    }
    fs::create_dir_all(&install_dir)
        .map_err(|e| format!("Could not create {}: {e}", install_dir.display()))?;
    {
        let mut config = state.locked();
        if config.stores.install_dirs.get(&store) != Some(&install_dir) {
            config
                .stores
                .install_dirs
                .insert(store, install_dir.clone());
            save_config(app, &config)?;
        }
    }

    let emit = |percent: f64| {
        let _ = app.emit(
            "store-install-progress",
            StoreInstallProgressPayload {
                store,
                app_name: app_name.clone(),
                percent,
            },
        );
    };
    emit(0.0);

    let log_path = new_log_file(&store_log_dir(app, store.slug(), &app_name)?)?;
    let installed = match store {
        StoreKind::Epic => {
            let legendary = legendary(app).await?;
            run_install(
                legendary.command(epic::install_args(&app_name, &install_dir)),
                "Legendary",
                &log_path,
                installs,
                &key,
                &emit,
            )
            .await?;
            let json = legendary
                .output(["list-installed", "--json"])
                .await
                .map_err(|e| command_failed(store, e))?;
            // Legendary also exits with 0 when it gave up on an install
            // (not enough disk space, say), having logged why.
            epic::parse_installed(&json, &app_name)?.ok_or_else(|| {
                install_failed(
                    &log_path,
                    format!("{app_name} is not installed after the install finished"),
                )
            })?
        }
        StoreKind::Gog => {
            install_gog(
                app,
                &app_name,
                &install_dir,
                &log_path,
                installs,
                &key,
                &emit,
            )
            .await?
        }
    };

    let prefix_path = install_prefix(app, state, store, &app_name, options.prefix_path.as_deref())?;
    let id = Uuid::new_v4();
    let exe_path = installed.exe_path.clone();
    let icon = save_exe_icon(app, id, extract_icon_png(&exe_path).as_deref());
    let mut game = Game {
        id,
        name: entry.title.clone(),
        exe_path,
        prefix_path,
        runner_id: options.runner_id,
        env_vars: Default::default(),
        launch_args: String::new(),
        notes: String::new(),
        icon,
        steamgriddb_id: None,
        cover_grid_id: None,
        cover_url: None,
        steamgriddb_icon_grid_id: None,
        steamgriddb_icon_url: None,
        artwork: Default::default(),
        umu_id: None,
        umu_store: None,
        overrides: Default::default(),
        store: Some(StoreLink {
            store,
            app_name: app_name.clone(),
            install_path: Some(installed.install_path.clone()),
            prerequisites: installed
                .prerequisites
                .into_iter()
                .filter(|prereq| !prereq.installed)
                .collect(),
        }),
    };
    adopt_cover(app, store, &entry, &mut game);

    let mut config = state.locked();
    config.games.push(game.clone());
    save_config(app, &config)?;
    emit(100.0);
    Ok(game)
}

/// Installs a GOG game: what it needs is looked up first (its folder, its
/// redistributables), then gogdl downloads the game and the
/// redistributables, whose installers run in the prefix on the first start.
async fn install_gog(
    app: &AppHandle,
    app_name: &str,
    install_dir: &Path,
    log_path: &Path,
    installs: &StoreInstalls,
    key: &(StoreKind, String),
    emit: &(dyn Fn(f64) + Sync),
) -> Result<epic::InstalledGame, AppError> {
    let gogdl = gogdl(app).await?;
    let plan = gog::parse_info(
        &gogdl
            .output(gog::info_args(app_name))
            .await
            .map_err(|e| command_failed(StoreKind::Gog, e))?,
    )?;
    if plan.folder_name.trim().is_empty() {
        return Err(command_failed(
            StoreKind::Gog,
            format!("gogdl named no install folder for {app_name}"),
        ));
    }
    run_install(
        gogdl.command(gog::download_args(app_name, install_dir)),
        "gogdl",
        log_path,
        installs,
        key,
        emit,
    )
    .await?;
    let game_dir = install_dir.join(&plan.folder_name);
    let info_path = gog::info_file(&game_dir, app_name);
    // gogdl also ends with 0 when it stopped early (cancelled from the
    // outside, say); a finished install has the game's info file.
    let info = fs::read_to_string(&info_path).map_err(|_| {
        install_failed(
            log_path,
            format!("{} is missing after the install", info_path.display()),
        )
    })?;
    let launch = gog::parse_play_task(&info, &game_dir)?;

    // The shared redistributables (Visual C++, DirectX, …) go into one folder
    // for every GOG game; their installers run in each game's prefix.
    let mut prerequisites = Vec::new();
    if !plan.dependencies.is_empty() {
        let redist_dir = store_dir(app, StoreKind::Gog)?.join("redist");
        // gogdl deletes what it downloaded before for ids it isn't asked
        // for again, so it gets every id any installed game needed.
        let ids_file = redist_dir.join("prefixr-ids.json");
        let mut ids: Vec<String> = fs::read(&ids_file)
            .ok()
            .and_then(|raw| serde_json::from_slice(&raw).ok())
            .unwrap_or_default();
        ids.extend(plan.dependencies.iter().cloned());
        ids.sort();
        ids.dedup();
        let result = gogdl.output(gog::redist_args(&ids, &redist_dir)).await;
        if result.is_ok() {
            let _ = fs::create_dir_all(&redist_dir);
            let _ = fs::write(&ids_file, serde_json::to_vec(&ids).unwrap_or_default());
        }
        let manifest = fs::read_to_string(redist_dir.join(gog::REDIST_MANIFEST));
        match (result, manifest) {
            (Ok(_), Ok(manifest)) => {
                prerequisites = gog::redist_installers(&manifest, &plan.dependencies, &redist_dir)?;
            }
            // The game is installed; without them it often runs all the
            // same, and the log says what's missing.
            (result, _) => {
                use std::io::Write;
                let reason = result.err().unwrap_or_else(|| "no manifest".to_string());
                if let Ok(mut log) = fs::OpenOptions::new().append(true).open(log_path) {
                    let _ = writeln!(log, "[prefixr] Redistributables not downloaded: {reason}");
                }
            }
        }
    }
    Ok(epic::InstalledGame {
        install_path: game_dir,
        exe_path: launch.exe_path,
        prerequisites,
    })
}

/// Runs a store tool's install, its output into the log and its progress
/// to `emit`.
async fn run_install(
    mut command: tokio::process::Command,
    tool_name: &str,
    log_path: &Path,
    installs: &StoreInstalls,
    key: &(StoreKind, String),
    emit: &(dyn Fn(f64) + Sync),
) -> Result<(), AppError> {
    let log = fs::OpenOptions::new()
        .append(true)
        .open(log_path)
        .map_err(|e| format!("Could not open log file: {e}"))?;
    let log = Arc::new(Mutex::new(log));
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        // Its own process group, so cancelling stops its download workers
        // along with it.
        .process_group(0)
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| format!("Could not start {tool_name}: {e}"))?;
    if let Some(install) = installs.0.locked().get_mut(key) {
        install.pid = child.id();
    }

    let last_emit = Mutex::new(None::<Instant>);
    let on_progress = |percent: f64| {
        if let Some(install) = installs.0.locked().get_mut(key) {
            install.percent = percent;
        }
        let mut last = last_emit.locked();
        if last.is_none_or(|at| at.elapsed() >= PROGRESS_INTERVAL) {
            *last = Some(Instant::now());
            emit(percent);
        }
    };
    let stdout = child.stdout.take().expect("stdout is piped");
    let stderr = child.stderr.take().expect("stderr is piped");
    futures_util::future::join(
        follow_output(stdout, log.clone(), &on_progress),
        follow_output(stderr, log.clone(), &on_progress),
    )
    .await;
    let status = child
        .wait()
        .await
        .map_err(|e| format!("{tool_name} failed: {e}"))?;

    let cancelled = installs.0.locked().get(key).is_some_and(|i| i.cancelled);
    if cancelled {
        return Err(AppError::StoreInstallCancelled);
    }
    if !status.success() {
        return Err(install_failed(
            log_path,
            format!("{tool_name} exited with {status}"),
        ));
    }
    Ok(())
}

/// An install that failed, with the error the tool logged (or `fallback`)
/// and the log to read the rest in.
fn install_failed(log_path: &Path, fallback: String) -> AppError {
    let log_text = fs::read_to_string(log_path).unwrap_or_default();
    AppError::StoreInstallFailed {
        message: epic::error_message(&log_text).unwrap_or(fallback),
        log_path: log_path.display().to_string(),
    }
}

/// Uninstalls a store game: its files through the store's tool, then the
/// game from the library, as `remove_game` removes any other.
#[tauri::command]
pub async fn uninstall_store_game(
    app: AppHandle,
    state: State<'_, ConfigState>,
    launching: State<'_, LaunchingGames>,
    id: String,
) -> Result<(), AppError> {
    let game_id = Uuid::parse_str(&id).map_err(|e| format!("Invalid game id: {e}"))?;
    let link = state
        .locked()
        .games
        .iter()
        .find(|game| game.id == game_id)
        .ok_or_else(|| format!("No game with id {id}"))?
        .store
        .clone();
    let Some(link) = link else {
        return remove_game(app.clone(), state, launching, id);
    };
    {
        // Not while it runs; released again before `remove_game` claims it.
        let Some(_guard) = launching.claim(game_id) else {
            return Err(AppError::GameRunning);
        };
        match link.store {
            StoreKind::Epic => {
                let legendary = legendary(&app).await?;
                let output = legendary
                    .command([
                        "-y",
                        "uninstall",
                        link.app_name.as_str(),
                        "--skip-uninstaller",
                    ])
                    .output()
                    .await
                    .map_err(|e| format!("Could not run Legendary: {e}"))?;
                // Legendary exits with 0 whether it uninstalled or not (it
                // can't while another of its commands holds its lock), so
                // its list of installed games tells. A game it no longer
                // lists (deleted by hand, say) leaves the library all the
                // same.
                let installed = legendary
                    .output(["list-installed", "--json"])
                    .await
                    .map_err(|e| command_failed(link.store, e))?;
                if epic::parse_installed(&installed, &link.app_name)?.is_some() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let message = epic::error_message(&stderr).unwrap_or_else(|| {
                        format!("{} is still installed after the uninstall", link.app_name)
                    });
                    return Err(command_failed(link.store, message));
                }
            }
            StoreKind::Gog => {
                // gogdl has no uninstall: the game's folder goes, once it's
                // certain to be that game's (its info file is in it).
                if let Some(dir) = &link.install_path {
                    if gog::info_file(dir, &link.app_name).is_file() {
                        fs::remove_dir_all(dir)
                            .map_err(|e| format!("Could not delete {}: {e}", dir.display()))?;
                    }
                }
                if let Ok(dir) = store_dir(&app, StoreKind::Gog) {
                    let manifests = gog::manifests_dir(&dir.join("gogdl"));
                    let _ = fs::remove_file(manifests.join(file_safe(&link.app_name)));
                }
            }
        }
    }
    remove_game(app.clone(), state, launching, id)
}

/// How a store game starts right now. Asks online first (an online game
/// needs fresh tokens on its command line); without a connection, a game
/// that can run offline still starts.
pub async fn launch_command(app: &AppHandle, link: &StoreLink) -> Result<StoreLaunch, AppError> {
    match link.store {
        StoreKind::Epic => {
            let legendary = legendary(app).await?;
            let json = match legendary
                .output(epic::launch_args(&link.app_name, false))
                .await
            {
                Ok(json) => json,
                Err(online) => legendary
                    .output(epic::launch_args(&link.app_name, true))
                    .await
                    .map_err(|_| command_failed(link.store, online))?,
            };
            Ok(epic::parse_launch(&json)?)
        }
        // No tokens: GOG games are DRM-free. The play task comes from the
        // game's own info file, as the Galaxy client reads it.
        StoreKind::Gog => {
            let dir = link
                .install_path
                .as_ref()
                .ok_or_else(|| format!("No install folder known for {}", link.app_name))?;
            let path = gog::info_file(dir, &link.app_name);
            let info = fs::read_to_string(&path)
                .map_err(|e| format!("Could not read {}: {e}", path.display()))?;
            Ok(gog::parse_play_task(&info, dir)?)
        }
    }
}

/// Records that a store game's prerequisites ran, so later launches skip
/// them.
pub fn mark_prerequisites_installed(app: &AppHandle, game_id: Uuid) -> Result<(), String> {
    let state = app.state::<ConfigState>();
    let mut config = state.locked();
    let Some(game) = config.games.iter_mut().find(|game| game.id == game_id) else {
        return Ok(());
    };
    if let Some(link) = game.store.as_mut() {
        for prereq in &mut link.prerequisites {
            prereq.installed = true;
        }
    }
    save_config(app, &config)
}

#[cfg(test)]
mod tests;
