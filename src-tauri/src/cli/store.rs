//! `prefixr store …`: the store accounts (Epic Games, GOG, Amazon Games)
//! from the command line, so an agent can connect them, refresh their game
//! lists and install or uninstall their games like the window does. Logging
//! in still takes a person: `store login` opens the store's page in the
//! browser, and the code it shows goes to `store connect`.

use std::cmp::Ordering;
use std::io::IsTerminal;
use std::path::Path;
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::{AppHandle, Listener, Manager};

use super::library::{all_entries, find_entry, LibraryEntry};
use super::{absolute, config, table, Out};
use crate::commands::games::LaunchingGames;
use crate::commands::runners::scan_runners;
use crate::commands::stores::{
    cancel_store_install, connect_store, disconnect_store, get_store_install_defaults,
    install_store_game, list_store_accounts, prepare_store_login, sync_store_library,
    uninstall_store_game, StoreInstallOptions, StoreInstallProgressPayload, StoreInstalls,
};
use crate::error::AppError;
use crate::host_env::xdg_open;
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

/// None of them hold the config lock from the start: an install or an
/// uninstall takes minutes, and the window doesn't wait for the lock, so a
/// copy of the config held that long would undo what the window saves
/// meanwhile. Each writes the config as it is on disk at its end instead
/// (see `config::update_config` and `uninstall_store_game`).
pub(super) fn changes_config(_command: &StoreCommand) -> bool {
    false
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

/// The runner given, checked to be installed, or the default one.
pub(super) fn runner_or_default(
    runners: &[Runner],
    runner: Option<String>,
) -> Result<String, AppError> {
    match runner {
        Some(id) if runners.iter().any(|r| r.id == id) => Ok(id),
        Some(id) => Err(format!("No runner {id}; see `prefixr runner list`").into()),
        None => Ok(default_runner(runners)
            .ok_or_else(|| {
                "No runner installed yet; download one with `prefixr runner download proton-ge`"
                    .to_string()
            })?
            .id
            .clone()),
    }
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
struct Login {
    store: StoreKind,
    /// The page to sign in on, also when the browser couldn't be opened
    /// (no desktop session): open it anywhere.
    url: String,
    opened: bool,
    next: String,
}

#[derive(Serialize)]
struct Synced {
    store: StoreKind,
    games: usize,
}

/// The progress line for `percent`, or `None` when nothing changed enough
/// to print one: every percent in a terminal (one line, rewritten), every
/// 10 % otherwise (an agent's log), starting with 0 %.
fn progress_line(name: &str, last: Option<i64>, percent: f64, interactive: bool) -> Option<String> {
    let percent = percent.floor() as i64;
    if interactive {
        (last != Some(percent)).then(|| format!("\r{name}: {percent} %   "))
    } else {
        last.is_none_or(|last| last / 10 != percent / 10)
            .then(|| format!("{name}: {percent} %\n"))
    }
}

/// Resolves on Ctrl+C or SIGTERM (an agent's command timeout, say).
pub(super) async fn stop_signal() {
    use tokio::signal::unix::{signal, SignalKind};
    let Ok(mut term) = signal(SignalKind::terminate()) else {
        let _ = tokio::signal::ctrl_c().await;
        return;
    };
    let ctrl_c = Box::pin(tokio::signal::ctrl_c());
    let term = Box::pin(term.recv());
    futures_util::future::select(ctrl_c, term).await;
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
            let url = prepare_store_login(app, store).await?;
            let opened = match xdg_open(&url).await {
                Ok(()) => true,
                Err(e) => {
                    out.note(&format!(
                        "Could not open the browser ({e}); open the address yourself."
                    ));
                    false
                }
            };
            let next = format!(
                "Log in, then run: prefixr store connect {} <code>  \
                 (the code the page shows, or the whole page or address)",
                store.slug()
            );
            let login = Login {
                store,
                url: url.clone(),
                opened,
                next: next.clone(),
            };
            out.print(&login, || format!("{url}\n{next}"))
        }
        StoreCommand::Connect { store, code } => {
            let store = StoreKind::from(store);
            let account = connect_store(app.clone(), store, code).await?;
            out.note("Fetching the game list…");
            // The login worked either way: a failed sync isn't a reason to
            // try the (now used) code again.
            let games = match sync_store_library(app.clone(), app.state(), store).await {
                Ok(games) => games.len().to_string(),
                Err(e) => {
                    out.note(&format!(
                        "Connected, but the game list failed: {}. Try `prefixr store sync {}`.",
                        e.localized(crate::locale::Locale::En),
                        store.slug()
                    ));
                    "?".to_string()
                }
            };
            let name = account.account_name.clone().unwrap_or_default();
            out.print(&account, || {
                format!(
                    "Connected {} as {name}: {games} games",
                    store.display_name()
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
            let synced = Synced {
                store,
                games: games.len(),
            };
            out.print(&synced, || {
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

    let runner_id = runner_or_default(&scan_runners(&config(app).runners_dir)?, runner)?;
    let install_dir = match dir {
        Some(dir) => absolute(Path::new(&dir))?.to_string_lossy().into_owned(),
        None => {
            get_store_install_defaults(app.clone(), app.state(), store, app_name.clone())
                .await?
                .install_dir
        }
    };
    let prefix = match prefix {
        Some(prefix) => Some(absolute(Path::new(&prefix))?.to_string_lossy().into_owned()),
        None => None,
    };

    out.note(&format!(
        "Installing {} with {runner_id} into {install_dir}…",
        entry.name
    ));
    let last = Arc::new(Mutex::new(None));
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
        let mut last = last.locked();
        if let Some(line) = progress_line(&name, *last, progress.percent, interactive) {
            eprint!("{line}");
            *last = Some(progress.percent.floor() as i64);
        }
    });
    let install = Box::pin(install_store_game(
        app.clone(),
        app.state(),
        app.state::<StoreInstalls>(),
        store,
        app_name.clone(),
        StoreInstallOptions {
            install_dir,
            prefix_path: prefix,
            runner_id,
        },
    ));
    // Ended from outside (Ctrl+C, an agent's timeout): the store's tool runs
    // in a process group of its own and would go on downloading, so it's
    // stopped like the window's Cancel does, and the install ends as one.
    let result = match futures_util::future::select(install, Box::pin(stop_signal())).await {
        futures_util::future::Either::Left((result, _)) => result,
        futures_util::future::Either::Right((_, install)) => {
            out.note("\nStopping the install…");
            let _ = cancel_store_install(app.state::<StoreInstalls>(), store, app_name);
            install.await
        }
    };
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
