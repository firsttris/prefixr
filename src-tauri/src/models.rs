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
    /// The exe's embedded icon, as a `data:image/png;base64,...` URI.
    /// Extracted once when the game is added/updated; `None` if the exe has
    /// no icon resource or it couldn't be parsed.
    #[serde(default)]
    pub icon: Option<String>,
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
    pub esync_enabled: bool,
    pub fsync_enabled: bool,
    pub dxvk_async_enabled: bool,
    pub vkbasalt_enabled: bool,
    pub vkbasalt_sharpen: bool,
    pub vkbasalt_sharpness: f32,
    pub vkbasalt_smaa: bool,
    pub vkbasalt_deband: bool,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            gamemode_enabled: false,
            esync_enabled: false,
            fsync_enabled: false,
            dxvk_async_enabled: false,
            vkbasalt_enabled: false,
            vkbasalt_sharpen: true,
            vkbasalt_sharpness: 0.4,
            vkbasalt_smaa: false,
            vkbasalt_deband: false,
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
        }
    }
}
