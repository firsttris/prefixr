use serde::Serialize;

use crate::locale::Locale;

/// A command's user-facing error. `Other` carries pre-formatted text exactly
/// as before (mostly technical/OS errors, e.g. "Could not read X: <os
/// err>") — the frontend shows those as-is, unlocalized, same as today.
/// The other variants carry just the data the situation needs, so the
/// frontend can render the sentence in the UI's current language (see
/// `backendError()` in `src/lib/i18n`). Add a variant here — and a matching
/// `backendErrors_<code>` message in `messages/{de,en}.json` whose
/// placeholders are the variant's fields — for any error a user can
/// plausibly hit and should read in their chosen language, rather than
/// leaving it to fall through to `Other`. `every_app_error_has_a_text`
/// checks that the message exists.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum AppError {
    Other {
        message: String,
    },
    /// games::launch_game — the game is already starting or running.
    GameAlreadyRunning,
    /// games::remove_game — the game is still starting or running.
    GameRunning,
    /// prefixes::delete_prefix — a game using this prefix is still running.
    PrefixInUse {
        game_name: String,
    },
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
    ShortcutsVdfUnreadable {
        error: String,
    },
    /// winetricks::install_winetricks_verbs — called with an empty selection.
    NoPackagesSelected,
    /// models::parse_launch_args — an odd number of `"` in the launch args.
    UnclosedQuoteInLaunchArgs,
    /// performance::fix_max_map_count — `pkexec` isn't installed, so the fix
    /// can't be applied automatically; `command` is the manual fallback
    /// shell command, shown verbatim regardless of language.
    PkexecNotInstalled {
        command: String,
    },
    /// performance::fix_max_map_count — the `pkexec sysctl` call itself
    /// failed or was cancelled (e.g. the PolicyKit prompt was dismissed).
    SysctlChangeFailed {
        status: String,
    },
    /// runners::delete_runner — still used by one or more games; `games` is
    /// their names, already joined for display (e.g. „A“, „B“).
    RunnerInUse {
        runner_name: String,
        games: String,
    },
    /// winetricks::install_winetricks_verbs — the prefix wasn't ready
    /// (readying it failed before winetricks itself even ran); also reused
    /// by wine_tools::launch_wine_tool and games::run_installer for the same
    /// "readying the prefix failed" case. `message` is the underlying
    /// (often technical/untranslated) failure.
    WithLogDetails {
        message: String,
        log_path: String,
    },
    /// winetricks::install_winetricks_verbs — the winetricks process itself
    /// exited with a non-zero status.
    WinetricksFailed {
        status: String,
        log_path: String,
    },
    /// prefixes::add_prefix — the path is already a known prefix.
    PrefixAlreadyExists {
        path: String,
    },
    /// prefixes::add_prefix — the path exists but isn't a directory.
    PathNotADirectory {
        path: String,
    },
    /// prefixes::add_prefix — the folder has files in it that don't look
    /// like an existing Wine/Proton prefix (no `drive_c`).
    PrefixDirNotEmpty {
        path: String,
    },
    /// runner_downloads::download_runner — a runner with this tag/version is
    /// already installed.
    RunnerAlreadyExists {
        tag: String,
    },
    /// runner_downloads — GitHub's unauthenticated rate limit was hit; a
    /// token in Settings raises it (see `GitHubSettings.svelte`).
    GitHubRateLimited,
    /// runner_downloads — the GitHub API returned some other non-success
    /// status.
    GitHubApiError {
        status: String,
    },
    /// wine_tools::launch_wine_tool — the tool's process itself failed to
    /// spawn (after the prefix was readied successfully).
    ToolLaunchFailed {
        tool: String,
        error: String,
    },
    /// games::run_installer — the installer's process itself failed to
    /// spawn (after the prefix was readied successfully).
    SetupLaunchFailed {
        error: String,
        log_path: String,
    },
    /// wine_tools::launch_wine_tool — `tool` isn't one of the fixed set the
    /// frontend offers; unreachable in normal use (defense in depth).
    UnknownWineTool {
        tool: String,
    },
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
        if let AppError::Other { message } = self {
            return message.clone();
        }
        // The same text the frontend shows: `backendErrors_<code>` with the
        // variant's fields as placeholders (see `serde(tag = "code")` above).
        let value = serde_json::to_value(self).expect("AppError serializes");
        let fields = value.as_object().expect("AppError is an object");
        let code = fields["code"].as_str().unwrap_or_default();
        let params: Vec<(&str, &str)> = fields
            .iter()
            .filter(|(k, _)| k.as_str() != "code")
            .map(|(k, v)| (k.as_str(), v.as_str().unwrap_or_default()))
            .collect();
        crate::locale::text(locale, &format!("backendErrors_{code}"), &params)
    }
}
