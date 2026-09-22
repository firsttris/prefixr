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
}

export interface GameInput {
  name: string;
  exe_path: string;
  prefix_path: string;
  runner_id: string;
  env_vars: Record<string, string>;
  launch_args: string;
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

export interface MangoHudConfig {
  enabled: boolean;
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

export interface PerformanceConfig {
  gamemode_enabled: boolean;
  vkbasalt_enabled: boolean;
  vkbasalt_sharpen: boolean;
  vkbasalt_sharpness: number;
  vkbasalt_smaa: boolean;
  vkbasalt_deband: boolean;
  inhibit_sleep_enabled: boolean;
  power_profile_enabled: boolean;
}

export interface SteamGridDbConfig {
  api_key: string | null;
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
