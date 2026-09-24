import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { get, writable } from "svelte/store";
import type { Game, GameInput, InstallerResult, SteamChange } from "$lib/types";

export const games = writable<Game[]>([]);

export interface GameRunState {
  initializing: boolean;
  running: boolean;
  logPath?: string;
  // Raw backend error (string or structured AppError) — pass through
  // backendError() to render it, so it stays correct across a language
  // switch instead of freezing at whatever language it arrived in.
  error?: unknown;
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
  // The backend's structured AppError (see src-tauri/src/error.rs) —
  // pass it through backendError() to render it, same as a command's Err.
  message: unknown;
  log_path: string | null;
}

interface ActiveGame {
  id: string;
  running: boolean;
}

let eventsInitialized = false;

// Registers the launch-status listeners once; must run client-side only
// (call from onMount), since it touches the Tauri IPC bridge.
export function initGameEvents(): void {
  if (eventsInitialized) return;
  eventsInitialized = true;

  const listeners = [
    listen<InitializingPayload>("game-initializing", (event) => {
      patchRunState(event.payload.id, { initializing: true, error: undefined });
    }),

    listen<StartedPayload>("game-started", (event) => {
      patchRunState(event.payload.id, {
        initializing: false,
        running: true,
        logPath: event.payload.log_path,
        error: undefined,
      });
    }),

    listen<ExitedPayload>("game-exited", (event) => {
      patchRunState(event.payload.id, { running: false });
    }),

    listen<LaunchErrorPayload>("game-launch-error", (event) => {
      patchRunState(event.payload.id, {
        initializing: false,
        running: false,
        error: event.payload.message,
        logPath: event.payload.log_path ?? undefined,
      });
    }),
  ];

  // Games launched before the webview (re)loaded sent their events to a
  // page that's gone; their current state comes from the backend instead,
  // once the listeners for anything after that are in place.
  Promise.all(listeners)
    .then(() => invoke<ActiveGame[]>("list_active_games"))
    .then((active) => {
      for (const { id, running } of active) {
        patchRunState(id, { initializing: !running, running });
      }
    })
    .catch(() => {});
}

// Shows the game as starting right away: preparing a launch (downloading
// umu, DXVK or wine-mono) can take a while before the backend sends any
// event, and the start button must not allow a second launch meanwhile.
export async function launchGame(id: string): Promise<void> {
  const current = get(gameRunState)[id];
  if (current?.running || current?.initializing) return;
  patchRunState(id, { initializing: true, error: undefined });
  try {
    await invoke("launch_game", { id });
  } catch (e) {
    // A failed launch is already surfaced via the game-launch-error event;
    // this only covers a refused one, which sends none — e.g. because Steam
    // already runs the game in a Prefixr of its own.
    const failed = get(gameRunState)[id]?.error;
    patchRunState(id, { initializing: false, error: failed ?? e });
  }
}

// Kills the game's whole session (for a Proton game, its umu container;
// for a Wine game, its wine session), e.g. when a game has hung. The actual "not running
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

// Listens for a `pending-launch` event, fired when a desktop shortcut is
// used while Prefixr is already running: that second instance hands its
// game id off to this one instead.
export function listenForPendingLaunch(callback: (id: string) => void): Promise<UnlistenFn> {
  return listen<string>("pending-launch", (event) => callback(event.payload));
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
export function listenForPendingInstall(
  callback: (exePath: string) => void,
): Promise<UnlistenFn> {
  return listen<string>("pending-install", (event) => callback(event.payload));
}

// Runs the installer exe under the given prefix/runner and waits for it to
// exit, then returns any `.exe` shortcuts it created on the Desktop/Start
// Menu — candidates for the game's own exe, offered to the user instead of
// making them hunt for it manually (see InstallDialog.svelte) — and the
// installer's log.
export async function runInstaller(
  prefixPath: string,
  runnerId: string,
  exePath: string,
): Promise<InstallerResult> {
  return await invoke<InstallerResult>("run_installer", { prefixPath, runnerId, exePath });
}

export async function createDesktopShortcut(id: string): Promise<void> {
  await invoke("create_desktop_shortcut", { id });
}

export async function createMenuShortcut(id: string): Promise<void> {
  await invoke("create_menu_shortcut", { id });
}

// Ids of the games that have an entry in Steam.
export const steamGameIds = writable<Set<string>>(new Set());

export async function refreshSteamGames(): Promise<void> {
  steamGameIds.set(new Set(await invoke<string[]>("list_steam_games")));
}

// Adds the game to Steam as a non-Steam game, or updates its entry. With
// `shutdownSteam`, a running Steam is quit for the write and started again.
export async function exportToSteam(id: string, shutdownSteam: boolean): Promise<SteamChange> {
  return await invoke<SteamChange>("export_to_steam", { id, shutdownSteam });
}

export async function removeFromSteam(id: string, shutdownSteam: boolean): Promise<SteamChange> {
  return await invoke<SteamChange>("remove_from_steam", { id, shutdownSteam });
}
