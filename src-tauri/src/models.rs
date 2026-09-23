use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The kind of compatibility layer a runner provides.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RunnerKind {
    Proton,
    Wine,
}

/// A Proton or Wine build discovered on disk under the configured runners directory.
/// `id` is the folder name and doubles as the stable identifier used elsewhere
/// (e.g. `Game::runner_id`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Runner {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub kind: RunnerKind,
}

/// A Wine prefix known to the app, tracked in the config file. Not tied to a
/// runner — the runner that (lazily) initializes it is decided per-game, via
/// `Game::runner_id`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefixInfo {
    pub path: PathBuf,
}

/// A game the user has added to the library.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub id: Uuid,
    pub name: String,
    pub exe_path: PathBuf,
    pub prefix_path: PathBuf,
    pub runner_id: String,
    #[serde(default)]
    pub env_vars: HashMap<String, String>,
    /// Extra arguments passed to the game's exe itself (e.g. `--launcher-skip
    /// -dx11`), analogous to PortProton's `LAUNCH_PARAMETERS`. Split on
    /// whitespace at launch time — see `launch_game`.
    #[serde(default)]
    pub launch_args: String,
    /// The exe's embedded icon, as a `data:image/png;base64,...` URI.
    /// Extracted once when the game is added/updated; `None` if the exe has
    /// no icon resource or it couldn't be parsed.
    #[serde(default)]
    pub icon: Option<String>,
    /// The matched SteamGridDB *game* id, so re-opening the cover picker (or
    /// a future Steam-shortcut export) can reuse the match without
    /// re-searching by name.
    #[serde(default)]
    pub steamgriddb_id: Option<i64>,
    /// The specific SteamGridDB grid asset id behind `cover_url`, kept for
    /// future reuse (e.g. a Steam shortcut export).
    #[serde(default)]
    pub cover_grid_id: Option<i64>,
    /// Source URL of the chosen SteamGridDB cover image. Its file extension
    /// also identifies the cached file's format at
    /// `artwork/{id}.{ext}` under the app data dir — see
    /// `commands::steamgriddb`.
    #[serde(default)]
    pub cover_url: Option<String>,
    /// The specific SteamGridDB icon asset id behind `steamgriddb_icon_url`,
    /// kept for future reuse (e.g. a Steam shortcut export).
    #[serde(default)]
    pub steamgriddb_icon_grid_id: Option<i64>,
    /// Source URL of the chosen SteamGridDB icon image (distinct from
    /// `icon`, which is extracted from the exe itself). Cached at
    /// `artwork/{id}_icon.{ext}` under the app data dir — see
    /// `commands::steamgriddb`.
    #[serde(default)]
    pub steamgriddb_icon_url: Option<String>,
    /// Per-game deviations from the global MangoHud/performance settings.
    /// Defaults to "inherit everything", so games from older configs behave
    /// exactly as before.
    #[serde(default)]
    pub overrides: GameOverrides,
}

/// Payload for `add_game`; the id is assigned by the backend.
#[derive(Debug, Clone, Deserialize)]
pub struct GameInput {
    pub name: String,
    pub exe_path: PathBuf,
    pub prefix_path: PathBuf,
    pub runner_id: String,
    #[serde(default)]
    pub env_vars: HashMap<String, String>,
    #[serde(default)]
    pub launch_args: String,
    #[serde(default)]
    pub overrides: GameOverrides,
}

/// Per-game overrides layered over the global `MangoHudConfig` and
/// `PerformanceConfig` at launch — see `GameOverrides::apply`. Every field
/// is `None` to inherit the global setting. vkBasalt and gamescope are
/// overridden as a whole block rather than field by field: a global
/// `gamescope_width: None` already means "native", so per-field options
/// couldn't tell "inherit the width" apart from "explicitly no width".
/// MangoHud only gets an on/off switch here — its layout is a matter of
/// taste, not of the game, so it stays global.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct GameOverrides {
    #[serde(default)]
    pub mangohud_enabled: Option<bool>,
    #[serde(default)]
    pub gamemode_enabled: Option<bool>,
    #[serde(default)]
    pub vkbasalt: Option<VkBasaltSettings>,
    #[serde(default)]
    pub gamescope: Option<GamescopeSettings>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct VkBasaltSettings {
    pub enabled: bool,
    pub sharpen: bool,
    pub sharpness: f32,
    pub smaa: bool,
    pub deband: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct GamescopeSettings {
    pub enabled: bool,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
    #[serde(default)]
    pub fps_limit: Option<u32>,
    #[serde(default)]
    pub fullscreen: bool,
}

impl GameOverrides {
    /// Writes this game's overrides into copies of the global settings,
    /// leaving everything it doesn't override untouched.
    pub fn apply(&self, mangohud: &mut MangoHudConfig, performance: &mut PerformanceConfig) {
        if let Some(enabled) = self.mangohud_enabled {
            mangohud.enabled = enabled;
        }
        if let Some(enabled) = self.gamemode_enabled {
            performance.gamemode_enabled = enabled;
        }
        if let Some(vkbasalt) = &self.vkbasalt {
            performance.vkbasalt_enabled = vkbasalt.enabled;
            performance.vkbasalt_sharpen = vkbasalt.sharpen;
            performance.vkbasalt_sharpness = vkbasalt.sharpness;
            performance.vkbasalt_smaa = vkbasalt.smaa;
            performance.vkbasalt_deband = vkbasalt.deband;
        }
        if let Some(gamescope) = &self.gamescope {
            performance.gamescope_enabled = gamescope.enabled;
            performance.gamescope_width = gamescope.width;
            performance.gamescope_height = gamescope.height;
            performance.gamescope_fps_limit = gamescope.fps_limit;
            performance.gamescope_fullscreen = gamescope.fullscreen;
        }
    }
}

/// Global MangoHud (the in-game performance overlay) settings, applied to
/// every game's launch — see `commands::mangohud`. Kept as a handful of
/// friendly knobs plus a `preset` label rather than exposing MangoHud's own
/// sprawling config format, since the point of this tab is to make "looking
/// good" a couple of clicks rather than hand-editing a `MangoHud.conf`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MangoHudConfig {
    pub enabled: bool,
    pub preset: String,
    pub position: String,
    pub theme_color: String,
    pub background_alpha: f32,
    pub round_corners: bool,
    pub show_fps: bool,
    pub show_frametime: bool,
    pub show_cpu: bool,
    pub show_gpu: bool,
    pub show_ram: bool,
    pub show_vram: bool,
    pub show_temps: bool,
    /// Small on/off indicator icons for other tools in the stack.
    #[serde(default)]
    pub show_gamemode: bool,
    #[serde(default)]
    pub show_vkbasalt: bool,
    #[serde(default)]
    pub show_hdr: bool,
    /// Static diagnostic info, handy since this app manages the runner and
    /// DXVK/VKD3D build a game actually ends up using (see
    /// `graphics_layers.rs`) — `show_engine_version` in particular surfaces
    /// the DXVK/VKD3D version MangoHud detects at runtime.
    #[serde(default)]
    pub show_driver: bool,
    #[serde(default)]
    pub show_engine_version: bool,
    #[serde(default)]
    pub show_wine: bool,
    #[serde(default)]
    pub show_gpu_name: bool,
    #[serde(default)]
    pub show_resolution: bool,
    /// Lays the stats out in a row instead of a column (MangoHud's
    /// `horizontal` option).
    #[serde(default)]
    pub horizontal: bool,
}

/// Global performance-tuning toggles applied to every game's launch — see
/// `commands::performance`. Kept opt-in (all off by default) for the same
/// reason as `MangoHudConfig`: we can't know whether the underlying tool
/// (GameMode, vkBasalt) is even installed, so nothing gets silently switched
/// on for the user.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PerformanceConfig {
    pub gamemode_enabled: bool,
    pub vkbasalt_enabled: bool,
    pub vkbasalt_sharpen: bool,
    pub vkbasalt_sharpness: f32,
    pub vkbasalt_smaa: bool,
    pub vkbasalt_deband: bool,
    /// Wraps the game process with `systemd-inhibit` so the screensaver/sleep
    /// don't kick in mid-session. No-op if `systemd-inhibit` or the D-Bus
    /// system bus isn't available.
    #[serde(default)]
    pub inhibit_sleep_enabled: bool,
    /// Holds the desktop's power-profiles-daemon at the "performance" profile
    /// for the duration of the game (via `powerprofilesctl launch`), released
    /// automatically on exit. GameMode does *not* reliably do this itself —
    /// it writes the CPU governor directly, which power-profiles-daemon (the
    /// governor owner on most modern distros) can just overwrite again, so
    /// `gamemoderun` alone often leaves the profile on "balanced". See
    /// `launch_game`.
    #[serde(default)]
    pub power_profile_enabled: bool,
    /// Wraps the game in `gamescope`, giving it its own nested compositor
    /// session with independent resolution/refresh-rate — useful on
    /// handhelds/TVs. No-op if `gamescope` isn't installed, and skipped
    /// entirely if we're already running inside a gamescope session
    /// ourselves (nesting it again is pointless). See `launch_game`.
    #[serde(default)]
    pub gamescope_enabled: bool,
    #[serde(default)]
    pub gamescope_width: Option<u32>,
    #[serde(default)]
    pub gamescope_height: Option<u32>,
    #[serde(default)]
    pub gamescope_fps_limit: Option<u32>,
    #[serde(default)]
    pub gamescope_fullscreen: bool,
}

/// SteamGridDB API settings — see `commands::steamgriddb`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SteamGridDbConfig {
    #[serde(default)]
    pub api_key: Option<String>,
}

/// GitHub settings — see `commands::github`. An optional personal access
/// token, sent as a bearer token on requests to `api.github.com` (runner
/// release listings) to raise its rate limit from 60 to 5000 requests/hour;
/// GitHub accepts unauthenticated requests too, so this stays optional.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GitHubConfig {
    #[serde(default)]
    pub token: Option<String>,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            gamemode_enabled: false,
            vkbasalt_enabled: false,
            vkbasalt_sharpen: true,
            vkbasalt_sharpness: 0.4,
            vkbasalt_smaa: false,
            vkbasalt_deband: false,
            inhibit_sleep_enabled: false,
            power_profile_enabled: false,
            gamescope_enabled: false,
            gamescope_width: None,
            gamescope_height: None,
            gamescope_fps_limit: None,
            gamescope_fullscreen: false,
        }
    }
}

impl Default for MangoHudConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            preset: "standard".to_string(),
            position: "top-left".to_string(),
            theme_color: "ffffff".to_string(),
            background_alpha: 0.4,
            round_corners: true,
            show_fps: true,
            show_frametime: true,
            show_cpu: true,
            show_gpu: true,
            show_ram: false,
            show_vram: false,
            show_temps: true,
            show_gamemode: false,
            show_vkbasalt: false,
            show_hdr: false,
            show_driver: false,
            show_engine_version: false,
            show_wine: false,
            show_gpu_name: false,
            show_resolution: false,
            horizontal: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_overrides_keep_global_settings() {
        let mut mangohud = MangoHudConfig {
            enabled: true,
            ..MangoHudConfig::default()
        };
        let mut performance = PerformanceConfig {
            gamemode_enabled: true,
            gamescope_enabled: true,
            gamescope_width: Some(1920),
            ..PerformanceConfig::default()
        };
        GameOverrides::default().apply(&mut mangohud, &mut performance);
        assert!(mangohud.enabled);
        assert!(performance.gamemode_enabled);
        assert!(performance.gamescope_enabled);
        assert_eq!(performance.gamescope_width, Some(1920));
    }

    #[test]
    fn overrides_replace_only_their_own_settings() {
        let mut mangohud = MangoHudConfig {
            enabled: true,
            ..MangoHudConfig::default()
        };
        let mut performance = PerformanceConfig {
            gamemode_enabled: true,
            vkbasalt_enabled: true,
            gamescope_enabled: true,
            gamescope_width: Some(1920),
            gamescope_height: Some(1080),
            ..PerformanceConfig::default()
        };
        let overrides = GameOverrides {
            mangohud_enabled: Some(false),
            gamemode_enabled: None,
            vkbasalt: None,
            gamescope: Some(GamescopeSettings {
                enabled: true,
                width: Some(1280),
                height: None,
                fps_limit: Some(40),
                fullscreen: true,
            }),
        };
        overrides.apply(&mut mangohud, &mut performance);
        assert!(!mangohud.enabled);
        assert!(performance.gamemode_enabled);
        assert!(performance.vkbasalt_enabled);
        assert_eq!(performance.gamescope_width, Some(1280));
        // The gamescope block is replaced as a whole, so the global height
        // doesn't leak through.
        assert_eq!(performance.gamescope_height, None);
        assert_eq!(performance.gamescope_fps_limit, Some(40));
        assert!(performance.gamescope_fullscreen);
    }

    #[test]
    fn games_without_overrides_deserialize() {
        let game: Game = serde_json::from_str(
            r#"{"id":"00000000-0000-0000-0000-000000000000","name":"x","exe_path":"/x.exe","prefix_path":"/p","runner_id":"r"}"#,
        )
        .unwrap();
        assert!(game.overrides.mangohud_enabled.is_none());
        assert!(game.overrides.gamescope.is_none());
    }
}
