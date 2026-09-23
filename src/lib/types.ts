export type RunnerKind = "proton" | "wine";

export interface Runner {
  id: string;
  name: string;
  path: string;
  kind: RunnerKind;
}

export interface PrefixInfo {
  path: string;
}

export interface Game {
  id: string;
  name: string;
  exe_path: string;
  prefix_path: string;
  runner_id: string;
  env_vars: Record<string, string>;
  launch_args: string;
  icon: string | null;
  steamgriddb_id: number | null;
  cover_grid_id: number | null;
  cover_url: string | null;
  steamgriddb_icon_grid_id: number | null;
  steamgriddb_icon_url: string | null;
  umu_id: string | null;
  umu_store: string | null;
  overrides: GameOverrides;
}

export interface GameInput {
  name: string;
  exe_path: string;
  prefix_path: string;
  runner_id: string;
  env_vars: Record<string, string>;
  launch_args: string;
  umu_id: string | null;
  umu_store: string | null;
  overrides: GameOverrides;
}

// Per-game overrides, one block per settings category (Leistung, Bild,
// Overlay, Proton); `null` or a missing key inherits the global value — see
// `GameOverrides` in models.rs.
export interface GameOverrides {
  performance: PerformanceOverrides;
  graphics: GraphicsOverrides;
  overlay: OverlayOverrides;
  proton: Record<string, boolean>;
}

export interface PerformanceOverrides {
  gamemode_enabled: boolean | null;
  power_profile_enabled: boolean | null;
  inhibit_sleep_enabled: boolean | null;
}

export interface GraphicsOverrides {
  gamescope: GamescopeSettings | null;
  vkbasalt: VkBasaltSettings | null;
}

export interface OverlayOverrides {
  enabled: boolean | null;
  layout: MangoHudLayout | null;
}

// A switch found in a Proton runner's `proton` script — see
// `list_proton_options` in proton_options.rs.
export interface ProtonOption {
  env: string;
  config: string;
  aliases: string[];
}

// An .exe shortcut found on the Desktop/Start Menu right after an
// installer finished running inside a prefix — see `run_installer`.
export interface DetectedShortcut {
  name: string;
  exe_path: string;
}

export interface RunnerSourceInfo {
  id: string;
  label: string;
  kind: RunnerKind;
}

export interface RunnerRelease {
  source: string;
  tag: string;
  name: string;
  published_at: string;
  download_url: string;
  size: number;
}

export type MangoHudPosition = "top-left" | "top-right" | "bottom-left" | "bottom-right";

// Overlay. Stored as one flat object; `layout` is everything but `enabled`.
export interface MangoHudLayout {
  preset: string;
  position: MangoHudPosition;
  theme_color: string;
  background_alpha: number;
  round_corners: boolean;
  show_fps: boolean;
  show_frametime: boolean;
  show_cpu: boolean;
  show_gpu: boolean;
  show_ram: boolean;
  show_vram: boolean;
  show_temps: boolean;
  show_gamemode: boolean;
  show_vkbasalt: boolean;
  show_hdr: boolean;
  show_driver: boolean;
  show_engine_version: boolean;
  show_wine: boolean;
  show_gpu_name: boolean;
  show_resolution: boolean;
  horizontal: boolean;
}

export interface MangoHudConfig extends MangoHudLayout {
  enabled: boolean;
}

// Leistung.
export interface PerformanceConfig {
  gamemode_enabled: boolean;
  power_profile_enabled: boolean;
  inhibit_sleep_enabled: boolean;
}

// Bild.
export interface GraphicsConfig {
  gamescope: GamescopeSettings;
  vkbasalt: VkBasaltSettings;
}

export interface VkBasaltSettings {
  enabled: boolean;
  sharpen: boolean;
  sharpness: number;
  smaa: boolean;
  deband: boolean;
}

export interface GamescopeSettings {
  enabled: boolean;
  width: number | null;
  height: number | null;
  fps_limit: number | null;
  fullscreen: boolean;
}

// Proton: `PROTON_*` variable → on/off; a missing key keeps Proton's default.
export interface ProtonConfig {
  options: Record<string, boolean>;
}

export interface SteamGridDbConfig {
  api_key: string | null;
}

export interface GitHubConfig {
  token: string | null;
}

export interface SteamGridDbGameMatch {
  id: number;
  name: string;
  verified: boolean;
}

export interface SteamGridDbGrid {
  id: number;
  url: string;
  thumb: string;
  width: number;
  height: number;
}

export interface MaxMapCountStatus {
  current: number;
  recommended: number;
  sufficient: boolean;
  can_fix: boolean;
}

// Our managed copy of umu-launcher, which every Proton runner is launched
// through — see `commands::umu`.
export interface UmuStatus {
  installed: boolean;
  version: string | null;
}

// A suggested UMU id (`GAMEID`) for a game — see `search_umu_ids` in
// umu_database.rs. `store` is null for Steam and standalone releases.
export interface UmuMatch {
  umu_id: string;
  title: string;
  store: string | null;
  source: "steam" | "database";
}
