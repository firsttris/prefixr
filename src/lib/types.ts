// The shapes of what the backend sends and takes, generated from the Rust
// types by ts-rs into ./bindings (see `#[derive(TS)]` and .cargo/config.toml)
// whenever the Rust tests run — CI fails if they're out of date. Edit the
// Rust types, not these.
export type { ActiveGame } from "./bindings/ActiveGame";
export type { AppError } from "./bindings/AppError";
export type { ArtworkKind } from "./bindings/ArtworkKind";
export type { DetectedShortcut } from "./bindings/DetectedShortcut";
export type { DirectXLayerStatus } from "./bindings/DirectXLayerStatus";
export type { Game } from "./bindings/Game";
export type { GameExitedPayload } from "./bindings/GameExitedPayload";
export type { GameInitializingPayload } from "./bindings/GameInitializingPayload";
export type { GameInput } from "./bindings/GameInput";
export type { GameLaunchErrorPayload } from "./bindings/GameLaunchErrorPayload";
export type { GameOverrides } from "./bindings/GameOverrides";
export type { GameStartedPayload } from "./bindings/GameStartedPayload";
export type { GamescopeSettings } from "./bindings/GamescopeSettings";
export type { GitHubConfig } from "./bindings/GitHubConfig";
export type { GraphicsConfig } from "./bindings/GraphicsConfig";
export type { GraphicsOverrides } from "./bindings/GraphicsOverrides";
export type { InstallerResult } from "./bindings/InstallerResult";
export type { MangoHudConfig } from "./bindings/MangoHudConfig";
export type { MangoHudLayout } from "./bindings/MangoHudLayout";
export type { MaxMapCountStatus } from "./bindings/MaxMapCountStatus";
export type { OverlayOverrides } from "./bindings/OverlayOverrides";
export type { PerformanceConfig } from "./bindings/PerformanceConfig";
export type { PerformanceOverrides } from "./bindings/PerformanceOverrides";
export type { PrefixInfo } from "./bindings/PrefixInfo";
export type { ProtonConfig } from "./bindings/ProtonConfig";
export type { ProtonOption } from "./bindings/ProtonOption";
export type { Runner } from "./bindings/Runner";
export type { RunnerDownloadDonePayload } from "./bindings/RunnerDownloadDonePayload";
export type { RunnerDownloadProgressPayload } from "./bindings/RunnerDownloadProgressPayload";
export type { RunnerKind } from "./bindings/RunnerKind";
export type { RunnerRelease } from "./bindings/RunnerRelease";
export type { RunnerSourceInfo } from "./bindings/RunnerSourceInfo";
export type { SteamChange } from "./bindings/SteamChange";
export type { SteamGridDbConfig } from "./bindings/SteamGridDbConfig";
export type { SteamGridDbGameMatch } from "./bindings/SteamGridDbGameMatch";
export type { SteamGridDbGrid } from "./bindings/SteamGridDbGrid";
export type { UmuMatch } from "./bindings/UmuMatch";
export type { UmuMatchSource } from "./bindings/UmuMatchSource";
export type { UmuStatus } from "./bindings/UmuStatus";
export type { VkBasaltSettings } from "./bindings/VkBasaltSettings";
export type { WinetricksVerbMeta } from "./bindings/WinetricksVerbMeta";

import type { MangoHudLayout } from "./bindings/MangoHudLayout";

export type MangoHudPosition = MangoHudLayout["position"];
