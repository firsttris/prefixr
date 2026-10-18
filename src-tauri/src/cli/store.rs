//! `prefixr store …`: the store accounts (Epic Games, GOG, Amazon Games)
//! from the command line, so an agent can connect them, refresh their game
//! lists and install or uninstall their games like the window does. Logging
//! in still takes a person: `store login` opens the store's page in the
//! browser, and the code it shows goes to `store connect`.

use std::cmp::Ordering;
use std::io::IsTerminal;
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::{AppHandle, Listener, Manager};

use super::library::{all_entries, find_entry, LibraryEntry};
use super::{config, table, Out};
use crate::commands::games::LaunchingGames;
use crate::commands::runners::scan_runners;
use crate::commands::stores::{
    connect_store, disconnect_store, get_store_install_defaults, install_store_game,
    list_store_accounts, open_store_login, sync_store_library, uninstall_store_game,
    StoreInstallOptions, StoreInstallProgressPayload, StoreInstalls,
};
use crate::error::AppError;
use crate::lock::LockExt;
use crate::models::{Runner, RunnerKind, StoreKind};

#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum StoreArg {
    Epic,
    Gog,
    Amazon,
}

impl From<StoreArg> for StoreKind {
    fn from(store: StoreArg) -> Self {
        match store {
            StoreArg::Epic => StoreKind::Epic,
            StoreArg::Gog => StoreKind::Gog,
            StoreArg::Amazon => StoreKind::Amazon,
        }
    }
}

#[derive(clap::Subcommand, Debug)]
pub(super) enum StoreCommand {
    /// List the stores, whether they're connected, and how many games each has
    List,
    /// Open the store's login page in the browser; its code goes to `store connect`
    Login { store: StoreArg },
    /// Finish the login with the code from the store's page (or the whole page or address)
    Connect { store: StoreArg, code: String },
    /// Log out; games installed from the store stay
    Disconnect { store: StoreArg },
    /// Fetch the store's game list again
    Sync { store: StoreArg },
    /// Install a store game and add it to the library (by key or name, as
    /// `library list` shows it)
    Install {
        game: String,
        /// The folder the game's own folder goes in; the store's last one otherwise
        #[arg(long)]
        dir: Option<String>,
        /// An existing prefix to install into; a new one of its own otherwise
        #[arg(long)]
        prefix: Option<String>,
        /// The runner's id; the newest Proton otherwise
        #[arg(long)]
        runner: Option<String>,
    },
    /// Uninstall a store game: deletes its files; it stays in the store's list
    Uninstall { game: String },
}

/// Uninstalling removes a game from the library at once. An install takes
/// minutes, so it doesn't hold the config lock throughout: it writes its
/// results to the config as it is on disk then (see `config::update_config`).
pub(super) fn changes_config(command: &StoreCommand) -> bool {
    matches!(command, StoreCommand::Uninstall { .. })
}

/// Runner names compared with their numbers as numbers, as the window's
/// install dialog does: GE-Proton10-4 is newer than GE-Proton9-27.
fn natural_cmp(a: &str, b: &str) -> Ordering {
    let (mut a, mut b) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (a.peek().copied(), b.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) if x.is_ascii_digit() && y.is_ascii_digit() => {
                let number = |chars: &mut std::iter::Peekable<std::str::Chars>| {
                    let mut digits = String::new();
                    while let Some(c) = chars.peek().copied().filter(char::is_ascii_digit) {
                        digits.push(c);
                        chars.next();
                    }
                    digits.trim_start_matches('0').to_string()
                };
                let (x, y) = (number(&mut a), number(&mut b));
                let order = x.len().cmp(&y.len()).then_with(|| x.cmp(&y));
                if order != Ordering::Equal {
                    return order;
                }
            }
            (Some(x), Some(y)) => {
                let order = x.to_ascii_lowercase().cmp(&y.to_ascii_lowercase());
                if order != Ordering::Equal {
                    return order;
                }
                a.next();
                b.next();
            }
        }
    }
}

/// The runner a store game gets unless one is given: the newest Proton, or
/// the newest runner of any kind without one.
pub(crate) fn default_runner(runners: &[Runner]) -> Option<&Runner> {
    let newest = |kind: Option<RunnerKind>| {
        runners
            .iter()
            .filter(|r| kind.is_none_or(|kind| r.kind == kind))
            .max_by(|a, b| natural_cmp(&a.name, &b.name))
    };
    newest(Some(RunnerKind::Proton)).or_else(|| newest(None))
}

/// The store game `query` names, among the store games.
fn find_store_game<'a>(
    entries: &'a [LibraryEntry],
    query: &str,
) -> Result<&'a LibraryEntry, String> {
    let store_games: Vec<LibraryEntry> = entries
        .iter()
        .filter(|e| e.source != "local")
        .cloned()
        .collect();
    let found = find_entry(&store_games, query)?;
    Ok(entries
        .iter()
        .find(|e| e.key == found.key)
        .expect("found among them"))
}

/// `epic:Fortnite` → (Epic, "Fortnite").
fn split_key(key: &str) -> Option<(StoreKind, String)> {
    let (store, app_name) = key.split_once(':')?;
    let store = StoreKind::ALL.into_iter().find(|s| s.slug() == store)?;
    Some((store, app_name.to_string()))
}

#[derive(Serialize)]
struct Connected {
    store: StoreKind,
    login_opened: bool,
    next: String,
}

pub(super) async fn store_command(
    app: &AppHandle,
    out: Out,
    command: StoreCommand,
) -> Result<(), AppError> {
    match command {
        StoreCommand::List => {
            let accounts = list_store_accounts(app.clone()).await?;
            out.print(&accounts, || {
                let rows: Vec<Vec<String>> = accounts
                    .iter()
                    .map(|a| {
                        vec![
                            a.store.slug().to_string(),
                            if a.connected { "connected" } else { "-" }.to_string(),
                            a.account_name.clone().unwrap_or_default(),
                            a.game_count.to_string(),
                        ]
                    })
                    .collect();
                table(&["STORE", "STATUS", "ACCOUNT", "GAMES"], &rows)
            })
        }
        StoreCommand::Login { store } => {
            let store = StoreKind::from(store);
            open_store_login(app.clone(), store).await?;
            let next = format!(
                "Log in in the browser, then run: prefixr store connect {} <code>  \
                 (the code the page shows, or the whole page or address)",
                store.slug()
            );
            out.print(
                &Connected {
                    store,
                    login_opened: true,
                    next: next.clone(),
                },
                || next,
            )
        }
        StoreCommand::Connect { store, code } => {
            let store = StoreKind::from(store);
            let account = connect_store(app.clone(), store, code).await?;
            out.note("Fetching the game list…");
            let games = sync_store_library(app.clone(), app.state(), store).await?;
            let name = account.account_name.clone().unwrap_or_default();
            out.print(&account, || {
                format!(
                    "Connected {} as {name}: {} games",
                    store.display_name(),
                    games.len()
                )
            })
        }
        StoreCommand::Disconnect { store } => {
            let store = StoreKind::from(store);
            disconnect_store(app.clone(), app.state(), app.state(), store).await?;
            out.print(&store, || format!("Disconnected {}", store.display_name()))
        }
        StoreCommand::Sync { store } => {
            let store = StoreKind::from(store);
            let games = sync_store_library(app.clone(), app.state(), store).await?;
            out.print(&games.len(), || {
                format!("{}: {} games", store.display_name(), games.len())
            })
        }
        StoreCommand::Install {
            game,
            dir,
            prefix,
            runner,
        } => install(app, out, &game, dir, prefix, runner).await,
        StoreCommand::Uninstall { game } => {
            let entries = all_entries(app)?;
            let entry = find_store_game(&entries, &game)?;
            let Some(id) = entry.game_id.clone() else {
                return Err(format!("{} is not installed", entry.name).into());
            };
            uninstall_store_game(app.clone(), app.state(), app.state::<LaunchingGames>(), id)
                .await?;
            out.print(&entry.key, || format!("Uninstalled {}", entry.name))
        }
    }
}

async fn install(
    app: &AppHandle,
    out: Out,
    query: &str,
    dir: Option<String>,
    prefix: Option<String>,
    runner: Option<String>,
) -> Result<(), AppError> {
    let entries = all_entries(app)?;
    let entry = find_store_game(&entries, query)?.clone();
    if entry.installed {
        return Err(format!("{} is installed already", entry.name).into());
    }
    let (store, app_name) =
        split_key(&entry.key).ok_or_else(|| format!("{} is not a store game", entry.name))?;

    let runners = scan_runners(&config(app).runners_dir)?;
    let runner_id = match runner {
        Some(id) if runners.iter().any(|r| r.id == id) => id,
        Some(id) => return Err(format!("No runner {id}; see `prefixr runner list`").into()),
        None => default_runner(&runners)
            .ok_or_else(|| {
                "No runner installed yet; download one with `prefixr runner download proton-ge`"
                    .to_string()
            })?
            .id
            .clone(),
    };
    let install_dir = match dir {
        Some(dir) => dir,
        None => {
            get_store_install_defaults(app.clone(), app.state(), store, app_name.clone())
                .await?
                .install_dir
        }
    };

    out.note(&format!(
        "Installing {} with {runner_id} into {install_dir}…",
        entry.name
    ));
    // Progress on stderr: a line that updates in a terminal, every 10 %
    // otherwise (an agent's log).
    let last = Arc::new(Mutex::new(-1i64));
    let interactive = std::io::stderr().is_terminal();
    let (name, key_app) = (entry.name.clone(), app_name.clone());
    let listener = app.listen_any("store-install-progress", move |event| {
        let Ok(progress) = serde_json::from_str::<StoreInstallProgressPayload>(event.payload())
        else {
            return;
        };
        if progress.app_name != key_app {
            return;
        }
        let percent = progress.percent.floor() as i64;
        let mut last = last.locked();
        if interactive && percent != *last {
            eprint!("\r{name}: {percent} %   ");
        } else if !interactive && percent / 10 != *last / 10 {
            eprintln!("{name}: {percent} %");
        }
        *last = percent;
    });
    let result = install_store_game(
        app.clone(),
        app.state(),
        app.state::<StoreInstalls>(),
        store,
        app_name,
        StoreInstallOptions {
            install_dir,
            prefix_path: prefix,
            runner_id,
        },
    )
    .await;
    app.unlisten(listener);
    if interactive {
        eprintln!();
    }
    let game = result?;
    out.print(&game, || {
        format!("Installed {} ({})", game.name, &game.id.to_string()[..8])
    })
}

#[cfg(test)]
mod tests;
