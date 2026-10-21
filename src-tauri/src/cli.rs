//! The command line: `prefixr game add …`, `prefixr prefix winetricks …` and
//! so on, for scripts and AI agents. Each subcommand runs the same Tauri
//! commands the window calls, in a Prefixr without a window (as `--run`
//! does), so a game added or launched here behaves exactly like one from the
//! window. An open window picks up the changes (see
//! `config::watch_config_file`).

use std::collections::HashSet;
use std::fs;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Mutex;

use clap::{Args, Parser, Subcommand, ValueEnum};
use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Listener, Manager, State};

use crate::commands::games::{
    add_game, create_desktop_shortcut, create_menu_shortcut, end_game_session,
    game_processes_visible, game_wine_session, launch_game, remove_game, run_in_prefix,
    stop_game_gracefully, stop_refusal, update_game, LaunchingGames, RunningGames,
};
use crate::commands::library::game_key;
use crate::commands::logs::game_log_dir;
use crate::commands::prefixes::{add_prefix, delete_prefix};
use crate::commands::runner_downloads::{
    download_runner, list_runner_releases, list_runner_sources,
};
use crate::commands::runners::{delete_runner, scan_runners, wineserver_binary};
use crate::commands::steam::{export_to_steam, remove_from_steam, SteamChange};
use crate::commands::steamgriddb::{
    list_steamgriddb_artwork, list_steamgriddb_grids, list_steamgriddb_icons,
    search_steamgriddb_games, set_game_artwork, set_game_cover, set_game_icon,
};
use crate::commands::stores::StoreInstalls;
use crate::commands::umu::{get_umu_status, install_umu};
use crate::commands::umu_database::search_umu_ids;
use crate::commands::wine_tools::launch_wine_tool;
use crate::commands::winetricks::{install_winetricks_verbs, list_installed_winetricks_verbs};
use crate::config::{load_config, lock_config, AppConfig, ConfigFileStamp, ConfigState};
use crate::error::AppError;
use crate::host_env::is_flatpak;
use crate::locale::{self, LocaleState};
use crate::lock::LockExt;
use crate::models::{ArtworkKind, Game, GameInput, RunnerKind};
use crate::tray::WindowVisible;

mod backup;
mod install;
mod library;
mod skill;
mod store;
use backup::BackupCommand;
use library::{CategoryCommand, LibraryCommand};
use skill::SkillCommand;
use store::StoreCommand;

/// The first arguments that make Prefixr a command line program rather than
/// open its window. Everything else, including `--launch`, `--run` and
/// `--install`, is left to the app as before.
const CLI_ARGS: &[&str] = &[
    "game",
    "library",
    "category",
    "store",
    "prefix",
    "runner",
    "umu",
    "backup",
    "skill",
    "help",
    "--help",
    "-h",
    "--version",
    "-V",
];

pub fn is_cli(args: &[String]) -> bool {
    // `--json` goes anywhere, also before the subcommand: `prefixr --json
    // game list` is the command line too, not the window.
    args.iter()
        .skip(1)
        .find(|arg| arg.as_str() != "--json")
        .is_some_and(|arg| CLI_ARGS.contains(&arg.as_str()))
}

#[derive(Parser, Debug)]
#[command(
    name = "prefixr",
    version,
    about = "Prefixr from the command line: manage the library, prefixes and runners, and launch games.",
    after_help = "Without a subcommand, Prefixr opens its window. Games are named by their id, \
                  the start of it, or their name."
)]
struct Cli {
    /// Print the result as JSON
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Command,
}

// Parsed once per run, so the size of the largest variant doesn't matter.
#[allow(clippy::large_enum_variant)]
#[derive(Subcommand, Debug)]
enum Command {
    /// The games in the library
    #[command(subcommand)]
    Game(GameCommand),
    /// The whole library, store games included: list it, mark favorites, hide games
    #[command(subcommand)]
    Library(LibraryCommand),
    /// The library's categories, and which games are in them
    #[command(subcommand)]
    Category(CategoryCommand),
    /// Store accounts (Epic Games, GOG, Amazon Games): connect, sync, install games
    #[command(subcommand)]
    Store(StoreCommand),
    /// Wine prefixes, and running things in them
    #[command(subcommand)]
    Prefix(PrefixCommand),
    /// Proton and Wine builds
    #[command(subcommand)]
    Runner(RunnerCommand),
    /// umu-launcher, and the UMU ids that select a game's protonfixes
    #[command(subcommand)]
    Umu(UmuCommand),
    /// Back up and restore the library, its settings and artwork
    #[command(subcommand)]
    Backup(BackupCommand),
    /// The Agent Skill that teaches an AI agent to use this command line
    #[command(subcommand)]
    Skill(SkillCommand),
}

#[derive(Subcommand, Debug)]
enum GameCommand {
    /// List the games
    List,
    /// Show everything about one game
    Show { game: String },
    /// Add a game to the library
    Add(AddGame),
    /// Run a game's installer (a setup .exe or .msi, or a disc image) in a
    /// prefix, then add the game it installed
    Install(install::InstallGame),
    /// Change a game; what isn't given stays as it is
    Edit(EditGame),
    /// Remove a game from the library; its prefix and files stay
    Remove { game: String },
    /// Start a game and wait until it exits
    Launch { game: String },
    /// End a game's whole session, wherever it was started
    Stop { game: String },
    /// Print the path of the game's latest log
    Log {
        game: String,
        /// Print the log itself
        #[arg(long)]
        print: bool,
    },
    /// Pick artwork from SteamGridDB (needs its API key)
    Artwork(ArtworkArgs),
    /// Create a desktop or start menu shortcut
    Shortcut {
        game: String,
        #[arg(long, value_enum, default_value_t = ShortcutPlace::Menu)]
        place: ShortcutPlace,
    },
    /// Add the game to Steam as a non-Steam game, or update its entry there
    SteamAdd {
        game: String,
        /// Quit a running Steam for the change and start it again
        #[arg(long)]
        restart_steam: bool,
    },
    /// Remove the game's entry from Steam
    SteamRemove {
        game: String,
        /// Quit a running Steam for the change and start it again
        #[arg(long)]
        restart_steam: bool,
    },
}

#[derive(Args, Debug)]
struct AddGame {
    /// The game's .exe
    #[arg(long)]
    exe: PathBuf,
    /// The name in the library; made from the exe's file name if left out
    #[arg(long)]
    name: Option<String>,
    /// The prefix folder; one Prefixr doesn't know yet is added (and created)
    #[arg(long)]
    prefix: PathBuf,
    /// The runner's id, as `prefixr runner list` shows it
    #[arg(long)]
    runner: String,
    #[command(flatten)]
    settings: GameSettings,
}

#[derive(Args, Debug)]
struct EditGame {
    game: String,
    #[arg(long)]
    name: Option<String>,
    #[arg(long)]
    exe: Option<PathBuf>,
    /// One Prefixr doesn't know yet is added (and created)
    #[arg(long)]
    prefix: Option<PathBuf>,
    #[arg(long)]
    runner: Option<String>,
    /// Remove all of the game's environment variables first
    #[arg(long)]
    clear_env: bool,
    #[command(flatten)]
    settings: GameSettings,
}

/// What `game add` and `game edit` share.
#[derive(Args, Debug, Default)]
struct GameSettings {
    /// Set an environment variable (repeatable); it wins over every other setting
    #[arg(long = "env", value_name = "KEY=VALUE", value_parser = parse_env)]
    env: Vec<(String, String)>,
    /// Remove an environment variable (repeatable)
    #[arg(long = "unset-env", value_name = "KEY")]
    unset_env: Vec<String>,
    /// Arguments for the game's exe, quoted as on a Windows command line
    #[arg(long, allow_hyphen_values = true)]
    args: Option<String>,
    /// Notes kept with the game, e.g. a crack's offline login or why a
    /// setting is needed ("" removes them)
    #[arg(long)]
    notes: Option<String>,
    /// The protonfixes match, e.g. umu-1091500 or a Steam app id ("" for none);
    /// see `prefixr umu search`
    #[arg(long)]
    umu_id: Option<String>,
    /// The store of the UMU id: gog, egs, …
    #[arg(long)]
    umu_store: Option<String>,
    /// Set a Proton switch for this game (repeatable), e.g. PROTON_USE_WINED3D=on
    #[arg(long = "proton", value_name = "NAME=on|off|default", value_parser = parse_proton)]
    proton: Vec<(String, Toggle)>,
    /// GameMode for this game
    #[arg(long, value_enum)]
    gamemode: Option<Toggle>,
    /// The performance power profile for this game
    #[arg(long, value_enum)]
    power_profile: Option<Toggle>,
    /// Keeping the system awake for this game
    #[arg(long, value_enum)]
    inhibit_sleep: Option<Toggle>,
    /// The MangoHud overlay for this game
    #[arg(long, value_enum)]
    mangohud: Option<Toggle>,
}

/// A per-game override: `default` follows the global setting again.
#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
enum Toggle {
    On,
    Off,
    Default,
}

impl Toggle {
    fn value(self) -> Option<bool> {
        match self {
            Toggle::On => Some(true),
            Toggle::Off => Some(false),
            Toggle::Default => None,
        }
    }
}

#[derive(Args, Debug)]
struct ArtworkArgs {
    game: String,
    /// The SteamGridDB game to take the images from; otherwise the game's
    /// earlier match, or the best search result
    #[arg(long)]
    steamgriddb_id: Option<i64>,
    /// What to search SteamGridDB for instead of the game's name
    #[arg(long)]
    query: Option<String>,
    /// The kinds of images to pick (default: all)
    #[arg(long, value_enum, value_delimiter = ',')]
    kinds: Vec<ImageKind>,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
enum ImageKind {
    Cover,
    Icon,
    Wide,
    Hero,
    Logo,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
enum ShortcutPlace {
    Desktop,
    Menu,
}

#[derive(Subcommand, Debug)]
enum PrefixCommand {
    /// List the prefixes
    List,
    /// Add a prefix: an empty or new folder, or an existing prefix
    Add { path: PathBuf },
    /// Remove a prefix from Prefixr; the folder stays
    Remove { path: PathBuf },
    /// Install winetricks packages, or list the installed ones when none are given
    Winetricks {
        prefix: PathBuf,
        #[command(flatten)]
        runner: RunnerArg,
        verbs: Vec<String>,
    },
    /// Open one of Wine's tools: winecfg, regedit, cmd, winefile, taskmgr, uninstaller
    Tool {
        prefix: PathBuf,
        #[command(flatten)]
        runner: RunnerArg,
        tool: String,
    },
    /// Run a program in the prefix and wait for it: an exe, or one of Wine's
    /// builtins such as `regedit /S file.reg`
    Run {
        prefix: PathBuf,
        #[command(flatten)]
        runner: RunnerArg,
        program: String,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

#[derive(Args, Debug)]
struct RunnerArg {
    /// The runner to use; defaults to the one all of the prefix's games use
    #[arg(long)]
    runner: Option<String>,
}

#[derive(Subcommand, Debug)]
enum RunnerCommand {
    /// List the installed runners
    List,
    /// List the sources runners can be downloaded from
    Sources,
    /// List a source's recent releases
    Releases { source: String },
    /// Download a release of a source, the latest if no tag is given
    Download { source: String, tag: Option<String> },
    /// Delete a runner no game uses
    Remove { runner: String },
}

#[derive(Subcommand, Debug)]
enum UmuCommand {
    /// Show the installed umu-launcher
    Status,
    /// Install or update umu-launcher
    Install,
    /// Search the UMU id (protonfixes match) for a game
    Search { query: String },
}

/// Whether `command` changes `config.json`. Only those wait for and hold
/// the config lock: a launch or a winetricks install can take minutes and
/// only reads it. `game install` takes it only for its changes, around the
/// installer (see `install::under_config_lock`).
fn changes_config(command: &Command) -> bool {
    matches!(
        command,
        Command::Game(
            GameCommand::Add(_)
                | GameCommand::Edit(_)
                | GameCommand::Remove { .. }
                | GameCommand::Artwork(_)
        ) | Command::Prefix(PrefixCommand::Add { .. } | PrefixCommand::Remove { .. })
    ) || match command {
        Command::Backup(command) => backup::changes_config(command),
        Command::Library(command) => library::changes_config(command),
        Command::Category(command) => library::category_changes_config(command),
        Command::Store(command) => store::changes_config(command),
        _ => false,
    }
}

/// `config.json.lock`, held for as long as the command line runs.
struct ConfigLock(#[allow(dead_code)] fs::File);

/// The exit code `run` ends with, set by the command before it ends the app.
static EXIT_CODE: AtomicI32 = AtomicI32::new(0);

/// Runs the command line in `args` and exits. Usage errors exit with 2, a
/// failed command with 1.
pub fn run(args: &[String], context: tauri::Context<tauri::Wry>) -> ! {
    let cli = Cli::try_parse_from(args).unwrap_or_else(|e| e.exit());
    let locale = LocaleState::default().get();
    // Needs neither the config nor the desktop session.
    let cli = match cli.command {
        Command::Skill(command) => {
            if let Err(error) = skill::skill_command(Out { json: cli.json }, command) {
                eprintln!("error: {}", error.localized(locale));
                std::process::exit(1);
            }
            std::process::exit(0);
        }
        command => Cli {
            json: cli.json,
            command,
        },
    };
    if crate::running_as_root() {
        eprintln!("{}", locale::text(locale, "native_root_message", &[]));
        std::process::exit(1);
    }

    let result = tauri::Builder::default()
        .setup(move |app| {
            app.manage(LocaleState::default());
            // Taken before the config is read, and kept until the process
            // exits (see `lock_config`).
            if changes_config(&cli.command) {
                match lock_config(app.handle()) {
                    Ok(lock) => app.manage(ConfigLock(lock)),
                    Err(message) => {
                        eprintln!("error: {message}");
                        std::process::exit(1);
                    }
                };
            }
            let config = load_config(app.handle()).unwrap_or_else(|message| {
                eprintln!(
                    "{}",
                    locale::text(locale, "native_config_message", &[("error", &message)])
                );
                std::process::exit(1);
            });
            app.manage(Mutex::new(config));
            app.manage(ConfigFileStamp::default());
            app.manage(RunningGames::default());
            app.manage(LaunchingGames::default());
            app.manage(StoreInstalls::default());
            app.manage(WindowVisible::new(false));

            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let code = match execute(&handle, cli).await {
                    Ok(()) => 0,
                    Err(error) => {
                        eprintln!("error: {}", error.localized(locale));
                        1
                    }
                };
                EXIT_CODE.store(code, Ordering::SeqCst);
                handle.exit(code);
            });
            Ok(())
        })
        .run(context);
    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
    std::process::exit(EXIT_CODE.load(Ordering::SeqCst))
}

async fn execute(app: &AppHandle, cli: Cli) -> Result<(), AppError> {
    let out = Out { json: cli.json };
    match cli.command {
        Command::Game(command) => game_command(app, out, command).await,
        Command::Library(command) => library::library_command(app, out, command).await,
        Command::Category(command) => library::category_command(app, out, command).await,
        Command::Store(command) => store::store_command(app, out, command).await,
        Command::Prefix(command) => prefix_command(app, out, command).await,
        Command::Runner(command) => runner_command(app, out, command).await,
        Command::Umu(command) => umu_command(app, out, command).await,
        Command::Backup(command) => backup::backup_command(app, out, command).await,
        Command::Skill(command) => skill::skill_command(out, command),
    }
}

/// How results are printed: as text for a person, or as JSON.
#[derive(Clone, Copy)]
struct Out {
    json: bool,
}

impl Out {
    /// Prints `value` as JSON, or `text` otherwise.
    fn print<T: Serialize>(self, value: &T, text: impl FnOnce() -> String) -> Result<(), AppError> {
        if self.json {
            let json = serde_json::to_string_pretty(value)
                .map_err(|e| format!("Could not serialize the result: {e}"))?;
            println!("{json}");
        } else {
            let text = text();
            if !text.is_empty() {
                println!("{text}");
            }
        }
        Ok(())
    }

    /// A message for a person, on stderr so it never mixes into JSON.
    fn note(self, message: &str) {
        eprintln!("{message}");
    }
}

/// Lines up `rows` under `headers`, columns two spaces apart.
pub(crate) fn table(headers: &[&str], rows: &[Vec<String>]) -> String {
    let mut widths: Vec<usize> = headers.iter().map(|h| h.chars().count()).collect();
    for row in rows {
        for (width, cell) in widths.iter_mut().zip(row) {
            *width = (*width).max(cell.chars().count());
        }
    }
    let line = |cells: Vec<&str>| -> String {
        let last = cells.len().saturating_sub(1);
        cells
            .iter()
            .enumerate()
            .map(|(i, cell)| {
                if i == last {
                    cell.to_string()
                } else {
                    format!("{cell:<width$}", width = widths[i])
                }
            })
            .collect::<Vec<_>>()
            .join("  ")
    };
    let mut lines = vec![line(headers.to_vec())];
    lines.extend(
        rows.iter()
            .map(|row| line(row.iter().map(String::as_str).collect())),
    );
    lines.join("\n")
}

/// The game `query` names: its id, the start of its id, its name (ignoring
/// case), or the one game whose name contains it.
pub(crate) fn find_game<'a>(games: &'a [Game], query: &str) -> Result<&'a Game, String> {
    let query = query.trim();
    let lower = query.to_lowercase();
    let pick = |matches: Vec<&'a Game>| -> Option<Result<&'a Game, String>> {
        match matches.as_slice() {
            [] => None,
            [game] => Some(Ok(*game)),
            several => Some(Err(format!(
                "\"{query}\" matches several games: {}",
                several
                    .iter()
                    .map(|g| format!("{} ({})", g.name, short_id(g)))
                    .collect::<Vec<_>>()
                    .join(", ")
            ))),
        }
    };
    if let Some(game) = games.iter().find(|g| g.id.to_string() == lower) {
        return Ok(game);
    }
    let by_id_start = || -> Vec<&'a Game> {
        if lower.len() < 4 {
            return Vec::new();
        }
        games
            .iter()
            .filter(|g| g.id.to_string().starts_with(&lower))
            .collect()
    };
    let by_name = || -> Vec<&'a Game> {
        games
            .iter()
            .filter(|g| g.name.trim().to_lowercase() == lower)
            .collect()
    };
    let by_name_part = || -> Vec<&'a Game> {
        games
            .iter()
            .filter(|g| g.name.to_lowercase().contains(&lower))
            .collect()
    };
    pick(by_id_start())
        .or_else(|| pick(by_name()))
        .or_else(|| pick(by_name_part()))
        .unwrap_or_else(|| Err(format!("No game matches \"{query}\"")))
}

fn short_id(game: &Game) -> String {
    game.id.simple().to_string()[..8].to_string()
}

/// A readable name from an exe's file name, as the window suggests one:
/// `DOOMTheDarkAges.exe` becomes `DOOM The Dark Ages`.
pub(crate) fn name_from_exe(exe: &Path) -> String {
    let file_name = exe
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let stem = match file_name.len().checked_sub(4) {
        Some(cut)
            if file_name.is_char_boundary(cut) && file_name[cut..].eq_ignore_ascii_case(".exe") =>
        {
            &file_name[..cut]
        }
        _ => file_name.as_str(),
    };
    let chars: Vec<char> = stem
        .chars()
        .map(|c| if "._-".contains(c) { ' ' } else { c })
        .collect();
    let mut name = String::new();
    for (i, &c) in chars.iter().enumerate() {
        let previous = i.checked_sub(1).map(|p| chars[p]);
        let next = chars.get(i + 1);
        let word_starts = c.is_uppercase()
            && previous.is_some_and(|p| {
                p.is_lowercase()
                    || p.is_ascii_digit()
                    || (p.is_uppercase() && next.is_some_and(|n| n.is_lowercase()))
            });
        if word_starts {
            name.push(' ');
        }
        name.push(c);
    }
    name.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn parse_env(arg: &str) -> Result<(String, String), String> {
    let (key, value) = arg
        .split_once('=')
        .ok_or_else(|| format!("expected KEY=VALUE, got \"{arg}\""))?;
    let key = key.trim();
    if key.is_empty() {
        return Err(format!("expected KEY=VALUE, got \"{arg}\""));
    }
    Ok((key.to_string(), value.to_string()))
}

fn parse_proton(arg: &str) -> Result<(String, Toggle), String> {
    let (name, toggle) = parse_env(arg)?;
    let toggle = Toggle::from_str(toggle.trim(), true)
        .map_err(|_| format!("expected on, off or default after \"{name}=\""))?;
    Ok((name.to_ascii_uppercase(), toggle))
}

/// Applies `settings` to a game's input, as `game add` and `game edit` take
/// them.
fn apply_settings(settings: GameSettings, input: &mut GameInput) {
    for (key, value) in settings.env {
        input.env_vars.insert(key, value);
    }
    for key in settings.unset_env {
        input.env_vars.remove(&key);
    }
    if let Some(args) = settings.args {
        input.launch_args = args;
    }
    if let Some(notes) = settings.notes {
        input.notes = notes.trim().to_string();
    }
    if let Some(umu_id) = settings.umu_id {
        input.umu_id = Some(umu_id);
        // A store belongs to the id it was picked with.
        input.umu_store = None;
    }
    if let Some(store) = settings.umu_store {
        input.umu_store = Some(store);
    }
    for (name, toggle) in settings.proton {
        match toggle.value() {
            Some(on) => input.overrides.proton.insert(name, on),
            None => input.overrides.proton.remove(&name),
        };
    }
    let performance = &mut input.overrides.performance;
    for (toggle, field) in [
        (settings.gamemode, &mut performance.gamemode_enabled),
        (
            settings.power_profile,
            &mut performance.power_profile_enabled,
        ),
        (
            settings.inhibit_sleep,
            &mut performance.inhibit_sleep_enabled,
        ),
        (settings.mangohud, &mut input.overrides.overlay.enabled),
    ] {
        if let Some(toggle) = toggle {
            *field = toggle.value();
        }
    }
}

fn game_input(game: &Game) -> GameInput {
    GameInput {
        name: game.name.clone(),
        exe_path: game.exe_path.clone(),
        prefix_path: game.prefix_path.clone(),
        runner_id: game.runner_id.clone(),
        env_vars: game.env_vars.clone(),
        launch_args: game.launch_args.clone(),
        notes: game.notes.clone(),
        umu_id: game.umu_id.clone(),
        umu_store: game.umu_store.clone(),
        overrides: game.overrides.clone(),
    }
}

fn config(app: &AppHandle) -> AppConfig {
    app.state::<ConfigState>().locked().clone()
}

fn lookup_game(app: &AppHandle, query: &str) -> Result<Game, AppError> {
    let config = config(app);
    Ok(find_game(&config.games, query)?.clone())
}

/// `path` made absolute, as the config keeps prefixes and exes.
fn absolute(path: &Path) -> Result<PathBuf, AppError> {
    std::path::absolute(path)
        .map_err(|e| AppError::from(format!("Invalid path {}: {e}", path.display())))
}

/// Checks that `runner_id` is an installed runner, naming the ones there are
/// otherwise.
fn check_runner(app: &AppHandle, runner_id: &str) -> Result<RunnerKind, AppError> {
    let runners = scan_runners(&config(app).runners_dir)?;
    runners
        .iter()
        .find(|r| r.id == runner_id)
        .map(|r| r.kind)
        .ok_or_else(|| {
            let installed: Vec<&str> = runners.iter().map(|r| r.id.as_str()).collect();
            format!(
                "Runner '{runner_id}' not found. Installed: {}",
                if installed.is_empty() {
                    "none (see `prefixr runner download`)".to_string()
                } else {
                    installed.join(", ")
                }
            )
            .into()
        })
}

/// The prefix at `path`, added to Prefixr first if it doesn't know it yet.
fn ensure_prefix(app: &AppHandle, out: Out, path: &Path) -> Result<PathBuf, AppError> {
    let path = absolute(path)?;
    let known = {
        let config = config(app);
        config.prefixes.iter().any(|p| p.path == path)
            || config.games.iter().any(|g| g.prefix_path == path)
    };
    if known {
        return Ok(path);
    }
    let added = add_prefix(app.clone(), app.state(), path.display().to_string())?;
    out.note(&format!("Added the prefix {}", added.display()));
    Ok(added)
}

fn check_exe(exe: &Path) -> Result<PathBuf, AppError> {
    let exe = absolute(exe)?;
    if !exe.is_file() {
        return Err(format!("No file at {}", exe.display()).into());
    }
    Ok(exe)
}

fn game_rows(config: &AppConfig) -> Vec<Vec<String>> {
    config
        .games
        .iter()
        .map(|g| {
            vec![
                short_id(g),
                g.name.clone(),
                g.runner_id.clone(),
                g.umu_id.clone().unwrap_or_default(),
                g.prefix_path.display().to_string(),
            ]
        })
        .collect()
}

fn describe_game(game: &Game) -> String {
    let mut lines = vec![
        format!("Name:        {}", game.name),
        format!("Id:          {}", game.id),
        format!("Exe:         {}", game.exe_path.display()),
        format!("Prefix:      {}", game.prefix_path.display()),
        format!("Runner:      {}", game.runner_id),
    ];
    if !game.launch_args.is_empty() {
        lines.push(format!("Arguments:   {}", game.launch_args));
    }
    if let Some(umu_id) = &game.umu_id {
        let store = game
            .umu_store
            .as_deref()
            .map(|s| format!(" ({s})"))
            .unwrap_or_default();
        lines.push(format!("UMU id:      {umu_id}{store}"));
    }
    let mut env: Vec<_> = game.env_vars.iter().collect();
    env.sort();
    for (key, value) in env {
        lines.push(format!("Environment: {key}={value}"));
    }
    let overrides = &game.overrides;
    let performance = &overrides.performance;
    for (name, value) in [
        ("GameMode", performance.gamemode_enabled),
        ("Power profile", performance.power_profile_enabled),
        ("Inhibit sleep", performance.inhibit_sleep_enabled),
        ("MangoHud", overrides.overlay.enabled),
    ] {
        if let Some(on) = value {
            lines.push(format!("Override:    {name} {}", on_off(on)));
        }
    }
    for (name, on) in &overrides.proton {
        lines.push(format!("Override:    {name} {}", on_off(*on)));
    }
    if overrides.graphics.gamescope.is_some() {
        lines.push("Override:    Gamescope settings".to_string());
    }
    if overrides.graphics.vkbasalt.is_some() {
        lines.push("Override:    vkBasalt settings".to_string());
    }
    if let Some(id) = game.steamgriddb_id {
        lines.push(format!("SteamGridDB: {id}"));
    }
    if !game.notes.is_empty() {
        lines.push("Notes:".to_string());
        lines.extend(game.notes.lines().map(|line| format!("  {line}")));
    }
    lines.join("\n")
}

fn on_off(on: bool) -> &'static str {
    if on {
        "on"
    } else {
        "off"
    }
}

async fn game_command(app: &AppHandle, out: Out, command: GameCommand) -> Result<(), AppError> {
    match command {
        GameCommand::List => {
            let config = config(app);
            out.print(&config.games, || {
                table(
                    &["ID", "NAME", "RUNNER", "UMU ID", "PREFIX"],
                    &game_rows(&config),
                )
            })
        }
        GameCommand::Show { game } => {
            let game = lookup_game(app, &game)?;
            let categories = config(app).library.categories_of(&game_key(&game));
            out.print(&game, || {
                let mut text = describe_game(&game);
                if !categories.is_empty() {
                    text.push_str(&format!("\nCategories:  {}", categories.join(", ")));
                }
                text
            })
        }
        GameCommand::Add(args) => {
            let exe = check_exe(&args.exe)?;
            check_runner(app, &args.runner)?;
            let prefix = ensure_prefix(app, out, &args.prefix)?;
            let mut input = GameInput {
                name: args.name.unwrap_or_else(|| name_from_exe(&exe)),
                exe_path: exe,
                prefix_path: prefix,
                runner_id: args.runner,
                env_vars: Default::default(),
                launch_args: String::new(),
                notes: String::new(),
                umu_id: None,
                umu_store: None,
                overrides: Default::default(),
            };
            apply_settings(args.settings, &mut input);
            let game = add_game(app.clone(), app.state(), input)?;
            out.print(&game, || {
                format!("Added \"{}\" ({})", game.name, short_id(&game))
            })
        }
        GameCommand::Edit(args) => {
            let game = lookup_game(app, &args.game)?;
            let mut input = game_input(&game);
            if let Some(name) = args.name {
                input.name = name;
            }
            if let Some(exe) = &args.exe {
                input.exe_path = check_exe(exe)?;
            }
            if let Some(runner) = args.runner {
                check_runner(app, &runner)?;
                input.runner_id = runner;
            }
            if let Some(prefix) = &args.prefix {
                input.prefix_path = ensure_prefix(app, out, prefix)?;
            }
            if args.clear_env {
                input.env_vars.clear();
            }
            apply_settings(args.settings, &mut input);
            let updated = update_game(app.clone(), app.state(), game.id.to_string(), input)?;
            out.print(&updated, || describe_game(&updated))
        }
        GameCommand::Remove { game } => {
            let game = lookup_game(app, &game)?;
            remove_game(app.clone(), app.state(), app.state(), game.id.to_string())?;
            out.print(&game.id, || format!("Removed \"{}\"", game.name))
        }
        GameCommand::Install(args) => install::install(app, out, args).await,
        GameCommand::Launch { game } => launch(app, out, &game).await,
        GameCommand::Stop { game } => {
            let game = lookup_game(app, &game)?;
            let runners = scan_runners(&config(app).runners_dir)?;
            let runner = runners
                .iter()
                .find(|r| r.id == game.runner_id)
                .ok_or_else(|| format!("Runner '{}' not found", game.runner_id))?;
            if game_processes_visible(game.id) {
                let session = game_wine_session(app, &app.state(), &game, runner).await?;
                stop_game_gracefully(game.id, &session, None).await;
            } else if is_flatpak() {
                return Err(stop_refusal(game.id, &game.name));
            } else {
                // Its processes run where this one can't read them (a
                // distrobox), or it was started without the marker.
                let wineserver = wineserver_binary(&runner.path)?;
                end_game_session(
                    &wineserver,
                    &game.prefix_path.to_string_lossy(),
                    &game.exe_path,
                )
                .await?;
            }
            out.print(&game.id, || format!("Stopped \"{}\"", game.name))
        }
        GameCommand::Log { game, print } => {
            let game = lookup_game(app, &game)?;
            let dir = game_log_dir(app, &game.id.to_string())?;
            let latest =
                latest_log(&dir).ok_or_else(|| format!("\"{}\" has no logs yet", game.name))?;
            if print {
                let text = fs::read_to_string(&latest)
                    .map_err(|e| format!("Could not read {}: {e}", latest.display()))?;
                print!("{text}");
                return Ok(());
            }
            out.print(&latest, || latest.display().to_string())
        }
        GameCommand::Artwork(args) => artwork(app, out, args).await,
        GameCommand::Shortcut { game, place } => {
            let game = lookup_game(app, &game)?;
            let id = game.id.to_string();
            match place {
                ShortcutPlace::Desktop => create_desktop_shortcut(app.clone(), app.state(), id)?,
                ShortcutPlace::Menu => create_menu_shortcut(app.clone(), app.state(), id)?,
            }
            out.print(&game.id, || {
                format!("Created a shortcut for \"{}\"", game.name)
            })
        }
        GameCommand::SteamAdd {
            game,
            restart_steam,
        } => {
            let game = lookup_game(app, &game)?;
            let change =
                export_to_steam(app.clone(), app.state(), game.id.to_string(), restart_steam)
                    .await?;
            steam_result(out, change, &format!("\"{}\" is in Steam", game.name))
        }
        GameCommand::SteamRemove {
            game,
            restart_steam,
        } => {
            let game = lookup_game(app, &game)?;
            let change = remove_from_steam(app.state(), game.id.to_string(), restart_steam).await?;
            steam_result(
                out,
                change,
                &format!("Removed \"{}\" from Steam", game.name),
            )
        }
    }
}

fn steam_result(out: Out, change: SteamChange, done: &str) -> Result<(), AppError> {
    match change {
        SteamChange::SteamRunning => Err("Steam is running and would overwrite the change. \
             Quit it, or pass --restart-steam."
            .to_string()
            .into()),
        SteamChange::Done { .. } => out.print(&change, || done.to_string()),
    }
}

/// The newest log in `dir`; logs are named after the time they were started.
fn latest_log(dir: &Path) -> Option<PathBuf> {
    fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "txt"))
        .max_by_key(|path| {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .and_then(|stem| stem.parse::<u128>().ok())
                .unwrap_or(0)
        })
}

/// `game launch`: starts the game as the window does, reports the steps on
/// stderr and waits for it to exit.
async fn launch(app: &AppHandle, out: Out, query: &str) -> Result<(), AppError> {
    let game = lookup_game(app, query)?;
    let listeners = [
        app.listen_any("game-initializing", move |_| {
            out.note("Setting up the prefix; the first time can take a few minutes …");
        }),
        app.listen_any("game-started", move |event| {
            let log = serde_json::from_str::<Value>(event.payload())
                .ok()
                .and_then(|payload| payload["log_path"].as_str().map(str::to_string))
                .unwrap_or_default();
            out.note(&format!("Running. Log: {log}"));
        }),
    ];
    let result = launch_game(
        app.clone(),
        app.state(),
        app.state(),
        app.state(),
        game.id.to_string(),
    )
    .await;
    for listener in listeners {
        app.unlisten(listener);
    }
    if result.is_err() {
        if let Some(log) = game_log_dir(app, &game.id.to_string())
            .ok()
            .and_then(|dir| latest_log(&dir))
        {
            out.note(&format!("Log: {}", log.display()));
        }
    }
    result?;
    out.print(&game.id, || format!("\"{}\" exited", game.name))
}

#[derive(Serialize)]
struct ArtworkPick {
    kind: &'static str,
    url: Option<String>,
}

async fn artwork(app: &AppHandle, out: Out, args: ArtworkArgs) -> Result<(), AppError> {
    let game = lookup_game(app, &args.game)?;
    let game_id = game.id.to_string();
    let steamgriddb_id = match args.steamgriddb_id.or(game.steamgriddb_id) {
        Some(id) => id,
        None => {
            let query = args.query.clone().unwrap_or_else(|| game.name.clone());
            let matches = search_steamgriddb_games(app.state(), query.clone()).await?;
            let best = matches
                .iter()
                .find(|m| m.name.eq_ignore_ascii_case(query.trim()))
                .or_else(|| matches.first())
                .ok_or_else(|| format!("SteamGridDB knows no game called \"{query}\""))?;
            out.note(&format!(
                "Using SteamGridDB's \"{}\" ({})",
                best.name, best.id
            ));
            best.id
        }
    };
    let kinds = if args.kinds.is_empty() {
        vec![
            ImageKind::Cover,
            ImageKind::Icon,
            ImageKind::Wide,
            ImageKind::Hero,
            ImageKind::Logo,
        ]
    } else {
        args.kinds
    };

    let mut picks = Vec::new();
    for kind in kinds {
        let (name, options) = match kind {
            ImageKind::Cover => (
                "cover",
                list_steamgriddb_grids(app.state(), steamgriddb_id).await?,
            ),
            ImageKind::Icon => (
                "icon",
                list_steamgriddb_icons(app.state(), steamgriddb_id).await?,
            ),
            ImageKind::Wide | ImageKind::Hero | ImageKind::Logo => {
                let artwork = artwork_kind(kind);
                (
                    image_kind_name(kind),
                    list_steamgriddb_artwork(app.state(), steamgriddb_id, artwork).await?,
                )
            }
        };
        // SteamGridDB lists the best-rated image first.
        let Some(image) = options.into_iter().next() else {
            out.note(&format!("No {name} image on SteamGridDB"));
            picks.push(ArtworkPick {
                kind: name,
                url: None,
            });
            continue;
        };
        let id = game_id.clone();
        match kind {
            ImageKind::Cover => {
                set_game_cover(
                    app.clone(),
                    app.state(),
                    id,
                    steamgriddb_id,
                    image.id,
                    image.url.clone(),
                )
                .await?
            }
            ImageKind::Icon => {
                set_game_icon(
                    app.clone(),
                    app.state(),
                    id,
                    steamgriddb_id,
                    image.id,
                    image.url.clone(),
                )
                .await?
            }
            _ => {
                set_game_artwork(
                    app.clone(),
                    app.state(),
                    id,
                    artwork_kind(kind),
                    steamgriddb_id,
                    image.url.clone(),
                )
                .await?
            }
        };
        picks.push(ArtworkPick {
            kind: name,
            url: Some(image.url),
        });
    }
    out.print(&picks, || {
        picks
            .iter()
            .filter_map(|pick| Some(format!("{}: {}", pick.kind, pick.url.as_ref()?)))
            .collect::<Vec<_>>()
            .join("\n")
    })
}

fn artwork_kind(kind: ImageKind) -> ArtworkKind {
    match kind {
        ImageKind::Hero => ArtworkKind::Hero,
        ImageKind::Logo => ArtworkKind::Logo,
        _ => ArtworkKind::Wide,
    }
}

fn image_kind_name(kind: ImageKind) -> &'static str {
    match kind {
        ImageKind::Cover => "cover",
        ImageKind::Icon => "icon",
        ImageKind::Wide => "wide",
        ImageKind::Hero => "hero",
        ImageKind::Logo => "logo",
    }
}

/// The runner to run something in `prefix` with: the one given, or the one
/// every game in the prefix uses.
fn prefix_runner(config: &AppConfig, prefix: &Path, runner: RunnerArg) -> Result<String, AppError> {
    if let Some(runner) = runner.runner {
        return Ok(runner);
    }
    let used: HashSet<&str> = config
        .games
        .iter()
        .filter(|g| g.prefix_path == prefix)
        .map(|g| g.runner_id.as_str())
        .collect();
    match used.into_iter().collect::<Vec<_>>().as_slice() {
        [runner] => Ok(runner.to_string()),
        [] => Err("No game uses this prefix yet; pass --runner"
            .to_string()
            .into()),
        _ => Err("The prefix's games use different runners; pass --runner"
            .to_string()
            .into()),
    }
}

#[derive(Serialize)]
struct PrefixRow {
    path: PathBuf,
    games: Vec<String>,
    exists: bool,
}

async fn prefix_command(app: &AppHandle, out: Out, command: PrefixCommand) -> Result<(), AppError> {
    match command {
        PrefixCommand::List => {
            let config = config(app);
            let rows: Vec<PrefixRow> = config
                .prefixes
                .iter()
                .map(|p| PrefixRow {
                    path: p.path.clone(),
                    games: config
                        .games
                        .iter()
                        .filter(|g| g.prefix_path == p.path)
                        .map(|g| g.name.clone())
                        .collect(),
                    exists: p.path.join("drive_c").is_dir(),
                })
                .collect();
            out.print(&rows, || {
                let lines: Vec<Vec<String>> = rows
                    .iter()
                    .map(|row| {
                        vec![
                            row.path.display().to_string(),
                            if row.exists { "yes" } else { "not yet" }.to_string(),
                            row.games.join(", "),
                        ]
                    })
                    .collect();
                table(&["PATH", "SET UP", "GAMES"], &lines)
            })
        }
        PrefixCommand::Add { path } => {
            let path = absolute(&path)?;
            let added = add_prefix(app.clone(), app.state(), path.display().to_string())?;
            out.print(&added, || format!("Added the prefix {}", added.display()))
        }
        PrefixCommand::Remove { path } => {
            let path = absolute(&path)?;
            delete_prefix(
                app.clone(),
                app.state(),
                app.state(),
                path.display().to_string(),
            )
            .await?;
            out.print(&path, || format!("Removed the prefix {}", path.display()))
        }
        PrefixCommand::Winetricks {
            prefix,
            runner,
            verbs,
        } => {
            let prefix = absolute(&prefix)?;
            let prefix_string = prefix.display().to_string();
            if verbs.is_empty() {
                let installed = list_installed_winetricks_verbs(app.state(), prefix_string)?;
                return out.print(&installed, || installed.join("\n"));
            }
            let runner = prefix_runner(&config(app), &prefix, runner)?;
            out.note(&format!("Installing {} with {runner} …", verbs.join(" ")));
            let log = install_winetricks_verbs(
                app.clone(),
                app.state(),
                prefix_string,
                runner,
                verbs.clone(),
            )
            .await?;
            out.print(&log, || {
                format!("Installed {}. Log: {log}", verbs.join(" "))
            })
        }
        PrefixCommand::Tool {
            prefix,
            runner,
            tool,
        } => {
            let prefix = absolute(&prefix)?;
            let runner = prefix_runner(&config(app), &prefix, runner)?;
            launch_wine_tool(
                app.clone(),
                app.state(),
                prefix.display().to_string(),
                runner,
                tool.clone(),
            )
            .await?;
            out.print(&tool, || format!("Started {tool}"))
        }
        PrefixCommand::Run {
            prefix,
            runner,
            program,
            args,
        } => {
            let prefix = absolute(&prefix)?;
            let runner = prefix_runner(&config(app), &prefix, runner)?;
            // A path is passed on absolute; a builtin's bare name as it is.
            let program = if Path::new(&program).components().count() > 1 {
                absolute(Path::new(&program))?.display().to_string()
            } else {
                program
            };
            let state: State<ConfigState> = app.state();
            let ran = run_in_prefix(
                app,
                &state,
                &prefix.display().to_string(),
                &runner,
                &program,
                &args,
            )
            .await?;
            #[derive(Serialize)]
            struct RunResult {
                exit_code: Option<i32>,
                log_path: PathBuf,
            }
            let result = RunResult {
                exit_code: ran.status.code(),
                log_path: ran.log_path,
            };
            out.print(&result, || {
                format!(
                    "{program} exited with {}. Log: {}",
                    ran.status,
                    result.log_path.display()
                )
            })?;
            if !ran.status.success() {
                return Err(format!("{program} exited with {}", ran.status).into());
            }
            Ok(())
        }
    }
}

async fn runner_command(app: &AppHandle, out: Out, command: RunnerCommand) -> Result<(), AppError> {
    match command {
        RunnerCommand::List => {
            let config = config(app);
            let runners = scan_runners(&config.runners_dir)?;
            out.print(&runners, || {
                let rows: Vec<Vec<String>> = runners
                    .iter()
                    .map(|r| {
                        let games = config.games.iter().filter(|g| g.runner_id == r.id).count();
                        vec![
                            r.id.clone(),
                            format!("{:?}", r.kind).to_lowercase(),
                            games.to_string(),
                        ]
                    })
                    .collect();
                table(&["ID", "KIND", "GAMES"], &rows)
            })
        }
        RunnerCommand::Sources => {
            let sources = list_runner_sources();
            out.print(&sources, || {
                let rows: Vec<Vec<String>> = sources
                    .iter()
                    .map(|s| {
                        vec![
                            s.id.to_string(),
                            format!("{:?}", s.kind).to_lowercase(),
                            s.label.to_string(),
                        ]
                    })
                    .collect();
                table(&["SOURCE", "KIND", "NAME"], &rows)
            })
        }
        RunnerCommand::Releases { source } => {
            let releases = list_runner_releases(app.state(), source).await?;
            out.print(&releases, || {
                let rows: Vec<Vec<String>> = releases
                    .iter()
                    .map(|r| {
                        vec![
                            r.tag.clone(),
                            r.published_at.get(..10).unwrap_or_default().to_string(),
                            format!("{} MB", r.size / 1_000_000),
                        ]
                    })
                    .collect();
                table(&["TAG", "DATE", "SIZE"], &rows)
            })
        }
        RunnerCommand::Download { source, tag } => {
            let releases = list_runner_releases(app.state(), source.clone()).await?;
            let release = match &tag {
                Some(tag) => releases.iter().find(|r| &r.tag == tag),
                None => releases.first(),
            }
            .ok_or_else(|| match &tag {
                Some(tag) => format!("No release {tag} of {source}"),
                None => format!("No releases of {source}"),
            })?;
            out.note(&format!("Downloading {} …", release.tag));
            let progress = std::io::stderr().is_terminal().then(|| {
                app.listen_any("runner-download-progress", |event| {
                    let Ok(payload) = serde_json::from_str::<Value>(event.payload()) else {
                        return;
                    };
                    let mb = |key: &str| payload[key].as_u64().map(|b| b / 1_000_000);
                    if let (Some(done), Some(total)) = (mb("downloaded"), mb("total")) {
                        eprint!("\r{done} / {total} MB");
                    }
                })
            });
            let result = download_runner(
                app.clone(),
                app.state(),
                source,
                release.download_url.clone(),
            )
            .await;
            if let Some(listener) = progress {
                app.unlisten(listener);
                eprintln!();
            }
            result?;
            out.print(&release.tag, || format!("Installed {}", release.tag))
        }
        RunnerCommand::Remove { runner } => {
            delete_runner(app.state(), runner.clone()).await?;
            out.print(&runner, || format!("Deleted {runner}"))
        }
    }
}

async fn umu_command(app: &AppHandle, out: Out, command: UmuCommand) -> Result<(), AppError> {
    match command {
        UmuCommand::Status => {
            let status = get_umu_status(app.clone())?;
            out.print(&status, || match (&status.installed, &status.version) {
                (true, Some(version)) => format!("umu-launcher {version}"),
                (true, None) => "umu-launcher (version unknown)".to_string(),
                (false, _) => "umu-launcher is not installed yet".to_string(),
            })
        }
        UmuCommand::Install => {
            let status = install_umu(app.clone(), app.state()).await?;
            out.print(&status, || {
                format!(
                    "Installed umu-launcher {}",
                    status.version.as_deref().unwrap_or_default()
                )
            })
        }
        UmuCommand::Search { query } => {
            let matches = search_umu_ids(app.clone(), app.state(), query, None).await?;
            out.print(&matches, || {
                let rows: Vec<Vec<String>> = matches
                    .iter()
                    .map(|m| {
                        vec![
                            m.umu_id.clone(),
                            m.store.clone().unwrap_or_default(),
                            m.title.clone(),
                        ]
                    })
                    .collect();
                table(&["UMU ID", "STORE", "TITLE"], &rows)
            })
        }
    }
}

#[cfg(test)]
mod tests;
