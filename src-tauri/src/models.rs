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
