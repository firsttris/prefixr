use serde::Serialize;

use crate::locale::Locale;

/// A command's user-facing error. `Other` carries pre-formatted text exactly
/// as before (mostly technical/OS errors, e.g. "Could not read X: <os
/// err>") — the frontend shows those as-is, unlocalized, same as today.
/// The other variants carry just the data the situation needs, so the
/// frontend can render the sentence in the UI's current language from its
/// own i18n dictionaries (see `backendErrors` in `src/lib/i18n`). Add a
/// variant here — and a matching `backendErrors.<code>` entry in `de.ts`/
/// `en.ts`, and an arm in `localized()` below — for any error a user can
/// plausibly hit and should read in their chosen language, rather than
/// leaving it to fall through to `Other`.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum AppError {
    Other { message: String },
    /// games::launch_game — the game is already starting or running.
    GameAlreadyRunning,
    /// prefixes::delete_prefix — a game using this prefix is still running.
    PrefixInUse { game_name: String },
    /// steam::stop_steam — the `steam` binary isn't on PATH, so Prefixr
    /// can't ask a running Steam to quit for the shortcuts change.
    SteamCommandNotFound,
    /// steam::stop_steam — Steam didn't quit within `SHUTDOWN_TIMEOUT`.
    SteamShutdownTimedOut,
    /// steam::steam_root — only a Flatpak install was found, which can't be
    /// asked to run an outside program (Prefixr) in its sandbox.
    SteamOnlyFlatpak,
    /// steam::steam_root — no Steam install found at all.
    SteamNotFound,
    /// steam::steam_user_dir — Steam is installed but no account has ever
    /// logged into it on this machine.
    NoSteamAccountFound,
    /// steam::read_shortcuts_file — the existing shortcuts.vdf didn't parse;
    /// left untouched rather than risking data loss by overwriting it.
    ShortcutsVdfUnreadable { error: String },
    /// winetricks::install_winetricks_verbs — called with an empty selection.
    NoPackagesSelected,
    /// models::parse_launch_args — an odd number of `"` in the launch args.
    UnclosedQuoteInLaunchArgs,
    /// performance::fix_max_map_count — `pkexec` isn't installed, so the fix
    /// can't be applied automatically; `command` is the manual fallback
    /// shell command, shown verbatim regardless of language.
    PkexecNotInstalled { command: String },
    /// performance::fix_max_map_count — the `pkexec sysctl` call itself
    /// failed or was cancelled (e.g. the PolicyKit prompt was dismissed).
    SysctlChangeFailed { status: String },
    /// runners::delete_runner — still used by one or more games; `games` is
    /// their names, already joined for display (e.g. „A“, „B“).
    RunnerInUse { runner_name: String, games: String },
    /// winetricks::install_winetricks_verbs — the prefix wasn't ready
    /// (readying it failed before winetricks itself even ran); also reused
    /// by wine_tools::launch_wine_tool and games::run_installer for the same
    /// "readying the prefix failed" case. `message` is the underlying
    /// (often technical/untranslated) failure.
    WithLogDetails { message: String, log_path: String },
    /// winetricks::install_winetricks_verbs — the winetricks process itself
    /// exited with a non-zero status.
    WinetricksFailed { status: String, log_path: String },
    /// prefixes::add_prefix — the path is already a known prefix.
    PrefixAlreadyExists { path: String },
    /// prefixes::add_prefix — the path exists but isn't a directory.
    PathNotADirectory { path: String },
    /// prefixes::add_prefix — the folder has files in it that don't look
    /// like an existing Wine/Proton prefix (no `drive_c`).
    PrefixDirNotEmpty { path: String },
    /// runner_downloads::download_runner — a runner with this tag/version is
    /// already installed.
    RunnerAlreadyExists { tag: String },
    /// runner_downloads — GitHub's unauthenticated rate limit was hit; a
    /// token in Settings raises it (see `GitHubSettings.svelte`).
    GitHubRateLimited,
    /// runner_downloads — the GitHub API returned some other non-success
    /// status.
    GitHubApiError { status: String },
    /// wine_tools::launch_wine_tool — the tool's process itself failed to
    /// spawn (after the prefix was readied successfully).
    ToolLaunchFailed { tool: String, error: String },
    /// games::run_installer — the installer's process itself failed to
    /// spawn (after the prefix was readied successfully).
    SetupLaunchFailed { error: String, log_path: String },
    /// wine_tools::launch_wine_tool — `tool` isn't one of the fixed set the
    /// frontend offers; unreachable in normal use (defense in depth).
    UnknownWineTool { tool: String },
}

impl From<String> for AppError {
    fn from(message: String) -> Self {
        AppError::Other { message }
    }
}

impl AppError {
    /// Plain-text rendering for the handful of call sites that build a
    /// message outside the IPC boundary instead of sending the structured
    /// error to the frontend to translate — e.g. `launch_game_headless`'s
    /// crash dialog, shown from a window-less process the frontend never
    /// runs in at all. Everywhere else, translate `code` (see `t()` /
    /// `backendError()` in `src/lib/i18n`) instead of calling this.
    pub fn localized(&self, locale: Locale) -> String {
        use Locale::{De, En};
        match (self, locale) {
            (AppError::Other { message }, _) => message.clone(),
            (AppError::GameAlreadyRunning, De) => {
                "Das Spiel wird bereits gestartet oder läuft schon".into()
            }
            (AppError::GameAlreadyRunning, En) => {
                "The game is already starting or running".into()
            }
            (AppError::PrefixInUse { game_name }, De) => format!(
                "„{game_name}“ läuft noch in diesem Prefix. Beende das Spiel zuerst."
            ),
            (AppError::PrefixInUse { game_name }, En) => format!(
                "“{game_name}” is still running in this prefix. Close the game first."
            ),
            (AppError::SteamCommandNotFound, De) => {
                "Steam läuft. Bitte beende Steam und versuche es erneut.".into()
            }
            (AppError::SteamCommandNotFound, En) => {
                "Steam is running. Please quit Steam and try again.".into()
            }
            (AppError::SteamShutdownTimedOut, De) => {
                "Steam hat sich nicht innerhalb von 30 Sekunden beendet.".into()
            }
            (AppError::SteamShutdownTimedOut, En) => "Steam didn't quit within 30 seconds.".into(),
            (AppError::SteamOnlyFlatpak, De) => "Steam ist nur als Flatpak installiert. Das \
                 Flatpak darf keine Programme außerhalb seiner Sandbox starten, also auch \
                 Prefixr nicht."
                .into(),
            (AppError::SteamOnlyFlatpak, En) => "Steam is only installed as a Flatpak. The \
                 Flatpak isn't allowed to launch programs outside its sandbox, so it can't \
                 launch Prefixr either."
                .into(),
            (AppError::SteamNotFound, De) => {
                "Steam wurde nicht gefunden. Starte Steam einmal und melde dich an.".into()
            }
            (AppError::SteamNotFound, En) => {
                "Steam wasn't found. Start Steam once and sign in.".into()
            }
            (AppError::NoSteamAccountFound, De) => {
                "Kein Steam-Konto gefunden. Melde dich einmal in Steam an.".into()
            }
            (AppError::NoSteamAccountFound, En) => {
                "No Steam account found. Sign in to Steam once.".into()
            }
            (AppError::ShortcutsVdfUnreadable { error }, De) => format!(
                "shortcuts.vdf konnte nicht gelesen werden ({error}). Prefixr lässt die Datei \
                 deshalb unverändert."
            ),
            (AppError::ShortcutsVdfUnreadable { error }, En) => format!(
                "shortcuts.vdf could not be read ({error}). Prefixr is leaving the file \
                 unchanged."
            ),
            (AppError::NoPackagesSelected, De) => "Keine Pakete ausgewählt".into(),
            (AppError::NoPackagesSelected, En) => "No packages selected".into(),
            (AppError::UnclosedQuoteInLaunchArgs, De) => {
                "Startargumente: ein Anführungszeichen wird nicht geschlossen".into()
            }
            (AppError::UnclosedQuoteInLaunchArgs, En) => {
                "Launch arguments: a quotation mark is left unclosed".into()
            }
            (AppError::PkexecNotInstalled { command }, De) => {
                format!("pkexec ist nicht installiert. Bitte manuell ausführen: {command}")
            }
            (AppError::PkexecNotInstalled { command }, En) => {
                format!("pkexec is not installed. Please run manually: {command}")
            }
            (AppError::SysctlChangeFailed { status }, De) => {
                format!("sysctl-Anpassung fehlgeschlagen oder abgebrochen (Status {status})")
            }
            (AppError::SysctlChangeFailed { status }, En) => {
                format!("sysctl change failed or was cancelled (status {status})")
            }
            (AppError::RunnerInUse { runner_name, games }, De) => {
                format!("{runner_name} wird noch verwendet von {games}.")
            }
            (AppError::RunnerInUse { runner_name, games }, En) => {
                format!("{runner_name} is still used by {games}.")
            }
            (AppError::WithLogDetails { message, log_path }, De) => {
                format!("{message} — Details im Log: {log_path}")
            }
            (AppError::WithLogDetails { message, log_path }, En) => {
                format!("{message} — details in the log: {log_path}")
            }
            (AppError::WinetricksFailed { status, log_path }, De) => format!(
                "winetricks beendete sich mit Status {status} — Details im Log: {log_path}"
            ),
            (AppError::WinetricksFailed { status, log_path }, En) => format!(
                "winetricks exited with status {status} — details in the log: {log_path}"
            ),
            (AppError::PrefixAlreadyExists { path }, De) => {
                format!("Prefix unter {path} existiert bereits")
            }
            (AppError::PrefixAlreadyExists { path }, En) => {
                format!("Prefix at {path} already exists")
            }
            (AppError::PathNotADirectory { path }, De) => {
                format!("{path} ist kein Ordner")
            }
            (AppError::PathNotADirectory { path }, En) => {
                format!("{path} is not a directory")
            }
            (AppError::PrefixDirNotEmpty { path }, De) => format!(
                "{path} enthält bereits Dateien, sieht aber nicht wie ein Wine- oder \
                 Proton-Prefix aus"
            ),
            (AppError::PrefixDirNotEmpty { path }, En) => format!(
                "{path} already contains files but doesn't look like a Wine or Proton prefix"
            ),
            (AppError::RunnerAlreadyExists { tag }, De) => {
                format!("Runner „{tag}“ existiert bereits")
            }
            (AppError::RunnerAlreadyExists { tag }, En) => {
                format!("Runner '{tag}' already exists")
            }
            (AppError::GitHubRateLimited, De) => "GitHubs Limit für nicht angemeldete Anfragen \
                 ist ausgeschöpft — ein GitHub-Token in den Einstellungen hebt es an."
                .into(),
            (AppError::GitHubRateLimited, En) => "GitHub's rate limit for unauthenticated \
                 requests is exhausted — add a GitHub token in the settings to raise it."
                .into(),
            (AppError::GitHubApiError { status }, De) => {
                format!("GitHub-API antwortete mit Status {status}")
            }
            (AppError::GitHubApiError { status }, En) => {
                format!("GitHub API returned status {status}")
            }
            (AppError::ToolLaunchFailed { tool, error }, De) => {
                format!("Konnte {tool} nicht starten: {error}")
            }
            (AppError::ToolLaunchFailed { tool, error }, En) => {
                format!("Could not start {tool}: {error}")
            }
            (AppError::SetupLaunchFailed { error, log_path }, De) => format!(
                "Setup konnte nicht gestartet werden: {error} — Details im Log: {log_path}"
            ),
            (AppError::SetupLaunchFailed { error, log_path }, En) => format!(
                "Setup could not be started: {error} — details in the log: {log_path}"
            ),
            (AppError::UnknownWineTool { tool }, De) => format!("Unbekanntes Wine-Werkzeug: {tool}"),
            (AppError::UnknownWineTool { tool }, En) => format!("Unknown Wine tool: {tool}"),
        }
    }
}
