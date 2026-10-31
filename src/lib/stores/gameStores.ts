import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { writable } from "svelte/store";
import { refreshGames } from "#lib/stores/games.ts";
import { refreshPrefixes } from "#lib/stores/prefixes.ts";
import type {
  Game,
  StoreAccount,
  StoreGame,
  StoreInstallDefaults,
  StoreInstallFinishedPayload,
  StoreInstallOptions,
  StoreInstallProgressPayload,
  StoreKind,
} from "#lib/types.ts";

// Store accounts (Epic, for now) and their games — see `commands::stores`.
// "Store" here means a game store; the Svelte stores of this file are named
// after what they hold.

export const storeAccounts = writable<StoreAccount[]>([]);
export const storeGames = writable<StoreGame[]>([]);

/** A running install's progress (0–100), by `storeGameKey`. */
export const storeInstalls = writable<Record<string, number>>({});

/** An install's failure, by `storeGameKey`, until it's dismissed or retried. */
export const storeInstallErrors = writable<Record<string, unknown>>({});

export function storeGameKey(store: StoreKind, appName: string): string {
  return `${store}:${appName}`;
}

/** The store's name as people know it. */
export function storeName(store: StoreKind): string {
  switch (store) {
    case "epic":
      return "Epic Games";
    case "gog":
      return "GOG";
    case "amazon":
      return "Amazon Games";
  }
}

/** The store games not installed yet: the ones the library shows besides its own. */
export function notInstalled(games: StoreGame[]): StoreGame[] {
  return games.filter((game) => game.game_id === null);
}

/** "18.4 GB" — decimal units, as stores and file managers show sizes. */
export function formatBytes(bytes: number | null | undefined, locale?: string): string | null {
  if (bytes == null || !Number.isFinite(bytes) || bytes < 0) return null;
  const units = ["B", "KB", "MB", "GB", "TB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1000 && unit < units.length - 1) {
    value /= 1000;
    unit++;
  }
  const digits = unit === 0 || value >= 100 ? 0 : 1;
  return `${value.toLocaleString(locale, { maximumFractionDigits: digits, minimumFractionDigits: digits })} ${units[unit]}`;
}

/** An asset-protocol URL for a store game's cached cover. */
export function storeCoverUrl(game: StoreGame): string | null {
  return game.cover_path ? convertFileSrc(game.cover_path) : null;
}

export async function refreshStoreAccounts(): Promise<void> {
  storeAccounts.set(await invoke<StoreAccount[]>("list_store_accounts"));
}

export async function refreshStoreGames(): Promise<void> {
  storeGames.set(await invoke<StoreGame[]>("list_store_games"));
}

export async function openStoreLogin(store: StoreKind): Promise<void> {
  await invoke("open_store_login", { store });
}

/**
 * Signs in in a window of Prefixr's own that takes the code itself, then
 * fetches the account's games. False when the user closed the window.
 */
export async function loginStoreInApp(store: StoreKind): Promise<boolean> {
  const account = await invoke<StoreAccount | null>("login_store_in_app", { store });
  if (!account) return false;
  await refreshStoreAccounts();
  await syncStoreLibrary(store);
  return true;
}

/** Logs in with the pasted code, then fetches the account's games. */
export async function connectStore(store: StoreKind, code: string): Promise<void> {
  await invoke<StoreAccount>("connect_store", { store, code });
  await refreshStoreAccounts();
  await syncStoreLibrary(store);
}

export async function disconnectStore(store: StoreKind): Promise<void> {
  await invoke("disconnect_store", { store });
  await Promise.all([refreshStoreAccounts(), refreshStoreGames()]);
}

export async function syncStoreLibrary(store: StoreKind): Promise<void> {
  await invoke<StoreGame[]>("sync_store_library", { store });
  await Promise.all([refreshStoreGames(), refreshStoreAccounts()]);
}

export async function getStoreInstallDefaults(
  store: StoreKind,
  appName: string,
): Promise<StoreInstallDefaults> {
  return await invoke<StoreInstallDefaults>("get_store_install_defaults", { store, appName });
}

function setProgress(key: string, percent: number | null) {
  storeInstalls.update((installs) => {
    const next = { ...installs };
    if (percent === null) delete next[key];
    else next[key] = percent;
    return next;
  });
}

function setError(key: string, error: unknown) {
  storeInstallErrors.update((errors) => {
    const next = { ...errors };
    if (error === null) delete next[key];
    else next[key] = error;
    return next;
  });
}

export function dismissStoreInstallError(store: StoreKind, appName: string): void {
  setError(storeGameKey(store, appName), null);
}

/**
 * Installs a store game. Resolves once it's in the library; shows its
 * progress in `storeInstalls` meanwhile. A failure is kept in
 * `storeInstallErrors` for the card to show — except a cancel, which is no
 * failure.
 */
export async function installStoreGame(
  store: StoreKind,
  appName: string,
  options: StoreInstallOptions,
): Promise<Game | null> {
  const key = storeGameKey(store, appName);
  setError(key, null);
  setProgress(key, 0);
  try {
    const game = await invoke<Game>("install_store_game", { store, appName, options });
    return game;
  } catch (e) {
    if (!isCancelled(e)) setError(key, e);
    return null;
  } finally {
    setProgress(key, null);
    await Promise.allSettled([refreshGames(), refreshStoreGames(), refreshPrefixes()]);
  }
}

function isCancelled(error: unknown): boolean {
  return (
    typeof error === "object" &&
    error !== null &&
    (error as { code?: string }).code === "store_install_cancelled"
  );
}

export async function cancelStoreInstall(store: StoreKind, appName: string): Promise<void> {
  await invoke("cancel_store_install", { store, appName });
}

/** Deletes a store game's files and removes it from the library. */
export async function uninstallStoreGame(id: string): Promise<void> {
  await invoke("uninstall_store_game", { id });
  await Promise.allSettled([refreshGames(), refreshStoreGames()]);
}

let eventsStarted = false;

/**
 * Follows the progress of installs, including ones that started before
 * the window (re)loaded. Call once, client-side (from onMount).
 */
export function initStoreEvents(): Promise<UnlistenFn[]> | null {
  if (eventsStarted) return null;
  eventsStarted = true;
  const listener = listen<StoreInstallProgressPayload>("store-install-progress", (event) => {
    const { store, app_name, percent } = event.payload;
    // The final event can arrive after the install already resolved.
    storeInstalls.update((installs) => {
      const key = storeGameKey(store, app_name);
      return key in installs ? { ...installs, [key]: percent } : installs;
    });
  });
  // For installs this window didn't start (it reloaded meanwhile): their
  // end is only known from this.
  const finished = listen<StoreInstallFinishedPayload>("store-install-finished", (event) => {
    const { store, app_name } = event.payload;
    setProgress(storeGameKey(store, app_name), null);
    Promise.allSettled([refreshGames(), refreshStoreGames(), refreshPrefixes()]);
  });
  Promise.all([listener, finished])
    .then(() => invoke<StoreInstallProgressPayload[]>("list_store_installs"))
    .then((running) => {
      for (const { store, app_name, percent } of running) {
        setProgress(storeGameKey(store, app_name), percent);
      }
    })
    .catch(() => {});
  return Promise.all([listener, finished]);
}
