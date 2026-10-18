use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;

/// The kind of compatibility layer a runner provides.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "lowercase")]
pub enum RunnerKind {
    Proton,
    Wine,
}

/// A Proton or Wine build discovered on disk under the configured runners directory.
/// `id` is the folder name and doubles as the stable identifier used elsewhere
/// (e.g. `Game::runner_id`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
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
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct PrefixInfo {
    pub path: PathBuf,
}

/// A game the user has added to the library.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct Game {
    pub id: Uuid,
    pub name: String,
    pub exe_path: PathBuf,
    pub prefix_path: PathBuf,
    pub runner_id: String,
    #[serde(default)]
    pub env_vars: HashMap<String, String>,
    /// Extra arguments passed to the game's exe itself (e.g. `--launcher-skip
    /// -dx11`), analogous to PortProton's `LAUNCH_PARAMETERS`. Split at
    /// launch time by `split_launch_args`.
    #[serde(default)]
    pub launch_args: String,
    /// Free text the user (or an agent setting the game up) keeps with the
    /// game: a crack's offline login, why an override is set, where a
    /// setting lives. Only shown, never used for a launch.
    #[serde(default)]
    pub notes: String,
    /// The path of the exe's embedded icon, saved as a PNG (see
    /// `icons::exe_icon_path`), which the frontend loads through the asset
    /// protocol. Extracted once when the game is added/updated; `None` if
    /// the exe has no icon resource or it couldn't be parsed. Never written
    /// to `config.json`, where versions before the file kept the icon
    /// itself as a `data:` URI (see `config::load_icons`).
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
    /// Chosen SteamGridDB images for Steam's other artwork slots, by kind:
    /// the source URL, cached at `artwork/{id}{suffix}.{ext}` (see
    /// `ArtworkKind::cache_suffix`). Only used by the Steam export.
    #[serde(default)]
    pub artwork: BTreeMap<ArtworkKind, String>,
    /// The game's UMU id (e.g. `umu-1091500`), passed to umu as `GAMEID` so
    /// umu-protonfixes applies this game's own fixes, and Proton its
    /// per-game hacks (umu derives `SteamAppId` from it). `None` leaves umu
    /// at `umu-default`, i.e. only the generic fixes. Proton runners only.
    #[serde(default)]
    pub umu_id: Option<String>,
    /// The store the `umu_id` entry belongs to (`gog`, `egs`, …), passed as
    /// `STORE`: protonfixes then looks in that store's fix folder instead of
    /// the generic umu one.
    #[serde(default)]
    pub umu_store: Option<String>,
    /// Per-game deviations from the global MangoHud/performance settings.
    /// Defaults to "inherit everything", so games from older configs behave
    /// exactly as before.
    #[serde(default)]
    pub overrides: GameOverrides,
    /// The store this game was installed from (see `commands::stores`), for
    /// a game Prefixr installed from a connected store account. Its launch
    /// then asks the store's tool for the command line (online games need
    /// the account's tokens on it), and removing it uninstalls its files.
    /// `None` for a game added by hand.
    #[serde(default)]
    pub store: Option<StoreLink>,
}

/// A store Prefixr can install games from. Each one is driven by a command
/// line tool from the Heroic Games Launcher project (see
/// `commands::stores::helpers`); a new store is a new variant here plus a
/// module of its own under `commands::stores`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "lowercase")]
pub enum StoreKind {
    /// The Epic Games Store, through Legendary.
    Epic,
    /// GOG, through gogdl.
    Gog,
    /// Amazon Games (Prime Gaming), through Nile.
    Amazon,
}

impl StoreKind {
    pub const ALL: [StoreKind; 3] = [StoreKind::Epic, StoreKind::Gog, StoreKind::Amazon];

    /// The name used in file and folder names (`stores/epic/…`).
    pub fn slug(self) -> &'static str {
        match self {
            StoreKind::Epic => "epic",
            StoreKind::Gog => "gog",
            StoreKind::Amazon => "amazon",
        }
    }

    /// The store's name as people know it, for log lines and errors that
    /// aren't translated.
    pub fn display_name(self) -> &'static str {
        match self {
            StoreKind::Epic => "Epic Games",
            StoreKind::Gog => "GOG",
            StoreKind::Amazon => "Amazon Games",
        }
    }
}

/// Ties a library game to the store entry it was installed from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct StoreLink {
    pub store: StoreKind,
    /// The store's own id for the game (Epic: the app name, e.g. `Fortnite`;
    /// GOG: the product id, e.g. `1207658924`; Amazon: the product id,
    /// e.g. `amzn1.adg.product.…`).
    pub app_name: String,
    /// The folder the store installed the game in.
    #[serde(default)]
    pub install_path: Option<PathBuf>,
    /// What the game wants installed in its prefix before it first runs
    /// (Visual C++ runtimes, DirectX, …), as the store lists it. Each runs
    /// once, on the first launch, and is then marked as installed.
    #[serde(default, deserialize_with = "one_or_many_prerequisites")]
    pub prerequisites: Vec<StorePrerequisites>,
}

/// A list of prerequisites, as well as what a config written before GOG
/// held there: Epic's one prerequisite (an object) or none (`null`).
fn one_or_many_prerequisites<'de, D>(deserializer: D) -> Result<Vec<StorePrerequisites>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum OneOrMany {
        Many(Vec<StorePrerequisites>),
        One(StorePrerequisites),
    }
    Ok(match Option::<OneOrMany>::deserialize(deserializer)? {
        Some(OneOrMany::Many(list)) => list,
        Some(OneOrMany::One(one)) => vec![one],
        None => Vec::new(),
    })
}

/// A store's post-install step: an installer shipped with the game.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct StorePrerequisites {
    pub name: String,
    /// The installer's path: relative to the game's install folder, or
    /// absolute for one shared by several games (GOG's redistributables).
    pub path: String,
    /// Its arguments, one string as the store gives them.
    #[serde(default)]
    pub args: String,
    #[serde(default)]
    pub installed: bool,
}

/// Steam's artwork slots besides the cover and icon, picked from
/// SteamGridDB just like those two.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "lowercase")]
pub enum ArtworkKind {
    /// The wide grid image, shown e.g. under "recent games".
    Wide,
    /// The banner at the top of the game's page.
    Hero,
    /// The logo laid over the hero.
    Logo,
}

impl ArtworkKind {
    pub const ALL: [ArtworkKind; 3] = [ArtworkKind::Wide, ArtworkKind::Hero, ArtworkKind::Logo];

    /// The suffix of the cached file, after the game id.
    pub fn cache_suffix(self) -> &'static str {
        match self {
            ArtworkKind::Wide => "_wide",
            ArtworkKind::Hero => "_hero",
            ArtworkKind::Logo => "_logo",
        }
    }

    /// The suffix of Steam's grid file, after the app id.
    pub fn grid_suffix(self) -> &'static str {
        match self {
            ArtworkKind::Wide => "",
            ArtworkKind::Hero => "_hero",
            ArtworkKind::Logo => "_logo",
        }
    }
}

impl Game {
    /// `GAMEID`/`STORE` for umu, empty without a `umu_id`.
    pub fn umu_env(&self) -> Vec<(String, String)> {
        let Some(id) = &self.umu_id else {
            return Vec::new();
        };
        let mut env = vec![("GAMEID".to_string(), id.clone())];
        if let Some(store) = &self.umu_store {
            env.push(("STORE".to_string(), store.clone()));
        }
        env
    }
}

/// Cleans up a UMU id as entered or picked in the game dialog: blank means
/// none, and a bare Steam app id becomes `umu-<id>` — the form umu expects,
/// and the only one it derives `SteamAppId` from.
pub fn normalize_umu_id(id: Option<String>) -> Option<String> {
    let id = id?.trim().to_string();
    if id.is_empty() {
        None
    } else if id.chars().all(|c| c.is_ascii_digit()) {
        Some(format!("umu-{id}"))
    } else {
        Some(id)
    }
}

/// Splits a game's launch arguments the way a Windows command line does:
/// at whitespace, except inside double quotes, which are removed — so
/// `-path "C:\My Games\x" -dx11` gives `-path`, `C:\My Games\x`, `-dx11`.
/// Unlike a shell (or `shlex`), backslashes are kept as they are, since
/// Windows paths are full of them, and a single quote is an ordinary
/// character, as in `-name O'Brien`.
pub fn split_launch_args(args: &str) -> Result<Vec<String>, AppError> {
    let mut result = Vec::new();
    let mut current = String::new();
    // Tracked apart from `current`, so `""` still counts as an argument.
    let mut in_arg = false;
    let mut quoted = false;
    for c in args.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                in_arg = true;
            }
            c if c.is_whitespace() && !quoted => {
                if in_arg {
                    result.push(std::mem::take(&mut current));
                    in_arg = false;
                }
            }
            c => {
                current.push(c);
                in_arg = true;
            }
        }
    }
    if quoted {
        return Err(AppError::UnclosedQuoteInLaunchArgs);
    }
    if in_arg {
        result.push(current);
    }
    Ok(result)
}

/// `none` is how the umu-database marks a standalone release; umu and
/// protonfixes treat it the same as no store at all.
pub fn normalize_umu_store(store: Option<String>) -> Option<String> {
    let store = store?.trim().to_ascii_lowercase();
    (!store.is_empty() && store != "none").then_some(store)
}

/// Payload for `add_game`; the id is assigned by the backend.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
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
    pub notes: String,
    #[serde(default)]
    pub umu_id: Option<String>,
    #[serde(default)]
    pub umu_store: Option<String>,
    #[serde(default)]
    pub overrides: GameOverrides,
}

// Game settings come in four categories, each with a global config in
// `AppConfig` and a matching block in `GameOverrides`:
//
// - Leistung (`PerformanceConfig`): GameMode, power profile, sleep inhibit.
// - Bild (`GraphicsConfig`): gamescope and vkBasalt.
// - Overlay (`MangoHudConfig`): MangoHud.
// - Proton (`ProtonConfig`): `PROTON_*` switches.
//
// A game inherits everything it doesn't override; `GameOverrides::resolve`
// layers the two at launch.

/// Per-game overrides of the global settings, one block per category. Every
/// field is `None` (or, for Proton, a missing key) to inherit the global
/// setting.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct GameOverrides {
    #[serde(default)]
    pub performance: PerformanceOverrides,
    #[serde(default)]
    pub graphics: GraphicsOverrides,
    #[serde(default)]
    pub overlay: OverlayOverrides,
    /// `PROTON_*` variable → on/off, layered key by key over
    /// `ProtonConfig::options`.
    #[serde(default)]
    pub proton: BTreeMap<String, bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct PerformanceOverrides {
    #[serde(default)]
    pub gamemode_enabled: Option<bool>,
    #[serde(default)]
    pub power_profile_enabled: Option<bool>,
    #[serde(default)]
    pub inhibit_sleep_enabled: Option<bool>,
}

/// gamescope and vkBasalt are overridden as a whole block rather than field
/// by field: a global `gamescope.width: None` already means "native", so
/// per-field options couldn't tell "inherit the width" apart from
/// "explicitly no width".
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct GraphicsOverrides {
    #[serde(default)]
    pub gamescope: Option<GamescopeSettings>,
    #[serde(default)]
    pub vkbasalt: Option<VkBasaltSettings>,
}

/// On/off separately from the look, so a game can switch MangoHud off (or
/// on) while still following the global layout.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct OverlayOverrides {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub layout: Option<MangoHudLayout>,
}

/// The settings a launch actually runs with, after `GameOverrides::resolve`.
#[derive(Debug, Clone)]
pub struct EffectiveSettings {
    pub performance: PerformanceConfig,
    pub graphics: GraphicsConfig,
    pub mangohud: MangoHudConfig,
    pub proton: BTreeMap<String, bool>,
}

impl GameOverrides {
    /// Layers this game's overrides over copies of the global settings.
    pub fn resolve(
        &self,
        performance: &PerformanceConfig,
        graphics: &GraphicsConfig,
        mangohud: &MangoHudConfig,
        proton: &ProtonConfig,
    ) -> EffectiveSettings {
        let perf = &self.performance;
        let performance = PerformanceConfig {
            gamemode_enabled: perf
                .gamemode_enabled
                .unwrap_or(performance.gamemode_enabled),
            power_profile_enabled: perf
                .power_profile_enabled
                .unwrap_or(performance.power_profile_enabled),
            inhibit_sleep_enabled: perf
                .inhibit_sleep_enabled
                .unwrap_or(performance.inhibit_sleep_enabled),
        };
        let graphics = GraphicsConfig {
            gamescope: self
                .graphics
                .gamescope
                .clone()
                .unwrap_or_else(|| graphics.gamescope.clone()),
            vkbasalt: self
                .graphics
                .vkbasalt
                .clone()
                .unwrap_or_else(|| graphics.vkbasalt.clone()),
        };
        let mangohud = MangoHudConfig {
            enabled: self.overlay.enabled.unwrap_or(mangohud.enabled),
            layout: self
                .overlay
                .layout
                .clone()
                .unwrap_or_else(|| mangohud.layout.clone()),
        };
        let mut proton_options = proton.options.clone();
        proton_options.extend(self.proton.iter().map(|(k, v)| (k.clone(), *v)));
        EffectiveSettings {
            performance,
            graphics,
            mangohud,
            proton: proton_options,
        }
    }
}

impl EffectiveSettings {
    /// The env vars for the Proton switches. `"0"` is an explicit off rather
    /// than a no-op: Proton turns some flags on by itself (e.g. `nvml` on
    /// Nvidia), and only a `0` switches those back off.
    pub fn proton_env(&self) -> impl Iterator<Item = (String, String)> + '_ {
        self.proton
            .iter()
            .map(|(name, on)| (name.clone(), if *on { "1" } else { "0" }.to_string()))
    }
}

/// Leistung: toggles that change how the system treats a running game.
/// Kept opt-in (all off by default), since we can't know whether the
/// underlying tool is even installed, so nothing gets silently switched on.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct PerformanceConfig {
    #[serde(default)]
    pub gamemode_enabled: bool,
    /// Holds the desktop's power-profiles-daemon at the "performance" profile
    /// for the duration of the game (via `powerprofilesctl launch`), released
    /// automatically on exit. GameMode does *not* reliably do this itself —
    /// it writes the CPU governor directly, which power-profiles-daemon (the
    /// governor owner on most modern distros) can just overwrite again, so
    /// `gamemoderun` alone often leaves the profile on "balanced". See
    /// `launch_game`.
    #[serde(default)]
    pub power_profile_enabled: bool,
    /// Wraps the game process with `systemd-inhibit` so the screensaver/sleep
    /// don't kick in mid-session. No-op if `systemd-inhibit` or the D-Bus
    /// system bus isn't available.
    #[serde(default)]
    pub inhibit_sleep_enabled: bool,
}

/// Bild: what the game's output goes through on its way to the screen.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct GraphicsConfig {
    #[serde(default)]
    pub gamescope: GamescopeSettings,
    #[serde(default)]
    pub vkbasalt: VkBasaltSettings,
}

/// Wraps the game in `gamescope`, giving it its own nested compositor
/// session with independent resolution/refresh-rate — useful on
/// handhelds/TVs. No-op if `gamescope` isn't installed, and skipped entirely
/// if we're already running inside a gamescope session ourselves (nesting it
/// again is pointless). See `launch_game`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct GamescopeSettings {
    #[serde(default)]
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

/// vkBasalt post-processing — see `commands::graphics`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(default)]
pub struct VkBasaltSettings {
    pub enabled: bool,
    pub sharpen: bool,
    pub sharpness: f32,
    pub smaa: bool,
}

impl Default for VkBasaltSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            sharpen: true,
            sharpness: 0.4,
            smaa: false,
        }
    }
}

/// Overlay: MangoHud (the in-game performance overlay) — see
/// `commands::mangohud`. Kept as a handful of friendly knobs plus a `preset`
/// label rather than exposing MangoHud's own sprawling config format, since
/// the point is to make "looking good" a couple of clicks rather than
/// hand-editing a `MangoHud.conf`. The layout is flattened into the same JSON
/// object, so it's stored exactly as before the split.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct MangoHudConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(flatten)]
    pub layout: MangoHudLayout,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(default)]
pub struct MangoHudLayout {
    pub preset: String,
    #[cfg_attr(
        test,
        ts(type = r#""top-left" | "top-right" | "bottom-left" | "bottom-right""#)
    )]
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
    pub show_gamemode: bool,
    pub show_vkbasalt: bool,
    pub show_hdr: bool,
    /// Static diagnostic info, handy since this app manages the runner and
    /// DXVK/VKD3D build a game actually ends up using (see
    /// `graphics_layers.rs`) — `show_engine_version` in particular surfaces
    /// the DXVK/VKD3D version MangoHud detects at runtime.
    pub show_driver: bool,
    pub show_engine_version: bool,
    pub show_wine: bool,
    pub show_gpu_name: bool,
    pub show_resolution: bool,
    /// Lays the stats out in a row instead of a column (MangoHud's
    /// `horizontal` option).
    pub horizontal: bool,
}

impl Default for MangoHudLayout {
    fn default() -> Self {
        Self {
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

/// Proton: `PROTON_*` switches (see `commands::proton_options`) set for every
/// game on a Proton runner, keyed by variable name; a missing key leaves
/// Proton's own default.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct ProtonConfig {
    #[serde(default)]
    pub options: BTreeMap<String, bool>,
}

/// SteamGridDB API settings — see `commands::steamgriddb`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct SteamGridDbConfig {
    #[serde(default)]
    pub api_key: Option<String>,
}

/// Store settings — see `commands::stores`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct StoresConfig {
    /// The folder each store's games were last installed to, offered again
    /// for its next one (each game gets a folder of its own inside it).
    /// Missing until a store's first install, which suggests
    /// `~/Games/<store>`.
    #[serde(default)]
    pub install_dirs: BTreeMap<StoreKind, PathBuf>,
}

/// GitHub settings — see `commands::github`. An optional personal access
/// token, sent as a bearer token on requests to `api.github.com` (runner
/// release listings) to raise its rate limit from 60 to 5000 requests/hour;
/// GitHub accepts unauthenticated requests too, so this stays optional.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct GitHubConfig {
    #[serde(default)]
    pub token: Option<String>,
}

/// The automatic backups — see `commands::backup`. Off unless the user
/// turns them on, so they never fill a disk nobody asked them to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(default)]
pub struct BackupConfig {
    pub automatic: bool,
    /// Days between two automatic backups (made when the window opens).
    pub interval_days: u32,
    /// How many automatic backups are kept, the ones before a restore
    /// included.
    pub keep: u32,
    /// Where they go; `backups/` under the data dir when unset.
    pub dir: Option<PathBuf>,
}

impl Default for BackupConfig {
    fn default() -> Self {
        Self {
            automatic: false,
            interval_days: 1,
            keep: 7,
            dir: None,
        }
    }
}

#[cfg(test)]
mod tests;
