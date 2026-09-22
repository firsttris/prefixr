import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { writable } from "svelte/store";
import type { Game, GameInput } from "$lib/types";

export const games = writable<Game[]>([]);

export interface GameRunState {
  initializing: boolean;
  running: boolean;
  logPath?: string;
  error?: string;
}

export const gameRunState = writable<Record<string, GameRunState>>({});

function patchRunState(id: string, patch: Partial<GameRunState>) {
  gameRunState.update((state) => {
    const current: GameRunState = state[id] ?? { initializing: false, running: false };
    return { ...state, [id]: { ...current, ...patch } };
  });
}

export async function refreshGames(): Promise<void> {
  games.set(await invoke<Game[]>("list_games"));
}

export async function addGame(input: GameInput): Promise<void> {
  await invoke<Game>("add_game", { game: input });
  await refreshGames();
}

export async function updateGame(id: string, input: GameInput): Promise<void> {
  await invoke<Game>("update_game", { id, game: input });
  await refreshGames();
}

export async function removeGame(id: string): Promise<void> {
  await invoke("remove_game", { id });
  await refreshGames();
}

interface InitializingPayload {
  id: string;
}

interface StartedPayload {
  id: string;
  log_path: string;
}

interface ExitedPayload {
  id: string;
  exit_code: number | null;
}

interface LaunchErrorPayload {
  id: string;
  message: string;
  log_path: string | null;
}

let eventsInitialized = false;

// Registers the launch-status listeners once; must run client-side only
// (call from onMount), since it touches the Tauri IPC bridge.
export function initGameEvents(): void {
  if (eventsInitialized) return;
  eventsInitialized = true;

  listen<InitializingPayload>("game-initializing", (event) => {
    patchRunState(event.payload.id, { initializing: true, error: undefined });
  });

  listen<StartedPayload>("game-started", (event) => {
    patchRunState(event.payload.id, {
      initializing: false,
      running: true,
      logPath: event.payload.log_path,
      error: undefined,
    });
  });

  listen<ExitedPayload>("game-exited", (event) => {
    patchRunState(event.payload.id, { running: false });
  });

  listen<LaunchErrorPayload>("game-launch-error", (event) => {
    patchRunState(event.payload.id, {
      initializing: false,
      running: false,
      error: event.payload.message,
      logPath: event.payload.log_path ?? undefined,
    });
  });
}

export async function launchGame(id: string): Promise<void> {
  try {
    await invoke("launch_game", { id });
  } catch {
    // Outcome is already surfaced via the game-launch-error event.
  }
}

// Kills the game's whole process group (the Proton/Wine runner and
// everything it started), e.g. when a game has hung. The actual "not running
// anymore" state update arrives via the game-exited event once the process
// is reaped, not from this call directly.
export async function killGame(id: string): Promise<void> {
  await invoke("kill_game", { id });
}

// Checks whether the app was started via a desktop shortcut's
// `--launch <id>` argument; returns the game id once, then clears it.
export async function takePendingLaunch(): Promise<string | null> {
  return await invoke<string | null>("take_pending_launch");
}

// Checks whether the app was started via the "Mit Prefixr installieren"
// file-manager context menu entry's `--install <exe-path>` argument; returns
// the exe path once, then clears it. A second instance launched the same way
// while Prefixr is already running instead delivers the path via the
// `pending-install` event (see `listenForPendingInstall`).
export async function takePendingInstall(): Promise<string | null> {
  return await invoke<string | null>("take_pending_install");
}

// Listens for a `pending-install` event, fired when a second "Mit Prefixr
// installieren" click hands its exe path off to this already-running
// instance instead of opening a new one.
export function listenForPendingInstall(callback: (exePath: string) => void): void {
  listen<string>("pending-install", (event) => callback(event.payload));
}

export async function runInstaller(
  prefixPath: string,
  runnerId: string,
  exePath: string,
): Promise<void> {
  await invoke("run_installer", { prefixPath, runnerId, exePath });
}

export async function createDesktopShortcut(id: string): Promise<void> {
  await invoke("create_desktop_shortcut", { id });
}

export async function createMenuShortcut(id: string): Promise<void> {
  await invoke("create_menu_shortcut", { id });
}
