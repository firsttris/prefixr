import { invoke } from "@tauri-apps/api/core";
import { get, writable } from "svelte/store";
import type { Compatibility, Game, LibraryConfig, StoreGame } from "$lib/types";
import { storeGameKey } from "$lib/stores/gameStores";

// How the library is organized (categories, favorites, hidden games) and
// played (see `commands::library`), plus what's known about how well its
// games run (see `commands::compatibility`). All keyed by `entryKey`.

const EMPTY: LibraryConfig = { categories: [], favorites: [], hidden: [], activity: {} };

export const libraryMeta = writable<LibraryConfig>(EMPTY);

export const compatibility = writable<Record<string, Compatibility>>({});

/** A library game's key: its store entry's when installed from a store. */
export function gameKey(game: Game): string {
  return game.store ? storeGameKey(game.store.store, game.store.app_name) : game.id;
}

export function storeEntryKey(game: StoreGame): string {
  return storeGameKey(game.store, game.app_name);
}

export async function refreshLibraryMeta(): Promise<void> {
  libraryMeta.set(await invoke<LibraryConfig>("get_library"));
}

async function change(command: string, args: Record<string, unknown>): Promise<void> {
  libraryMeta.set(await invoke<LibraryConfig>(command, args));
}

export const setFavorite = (key: string, favorite: boolean) =>
  change("set_favorite", { key, favorite });

export const setHidden = (key: string, hidden: boolean) => change("set_hidden", { key, hidden });

export const createCategory = (name: string) => change("create_category", { name });

export const renameCategory = (id: string, name: string) => change("rename_category", { id, name });

export const deleteCategory = (id: string) => change("delete_category", { id });

export const setGameCategories = (key: string, ids: string[]) =>
  change("set_game_categories", { key, ids });

export function isFavorite(meta: LibraryConfig, key: string): boolean {
  return meta.favorites.includes(key);
}

export function isHidden(meta: LibraryConfig, key: string): boolean {
  return meta.hidden.includes(key);
}

/** The ids of the categories the game is in. */
export function categoryIdsOf(meta: LibraryConfig, key: string): string[] {
  return meta.categories.filter((c) => c.games.includes(key)).map((c) => c.id);
}

// The games already looked up, found or not, so a game nothing is known
// about isn't asked about again on every change of the library.
const asked = new Set<string>();

/**
 * Looks up how well these games run; only the ones not looked up yet.
 * Best-effort: nothing known is simply nothing shown.
 */
export async function refreshCompatibility(keys: string[]): Promise<void> {
  const known = get(compatibility);
  const missing = keys.filter((key) => !(key in known) && !asked.has(key));
  if (missing.length === 0) return;
  missing.forEach((key) => asked.add(key));
  try {
    const found = await invoke<Record<string, Compatibility>>("get_compatibility", {
      keys: missing,
    });
    compatibility.update((current) => ({ ...current, ...found }));
  } catch {
    // Offline and nothing cached: no badges, and asked again next time.
    missing.forEach((key) => asked.delete(key));
  }
}

/** "3 h 20 min", "45 min" — or null for less than a minute. */
export function formatPlaytime(secs: number): string | null {
  const hours = Math.floor(secs / 3600);
  const minutes = Math.floor((secs % 3600) / 60);
  if (hours === 0 && minutes === 0) return null;
  if (hours === 0) return `${minutes} min`;
  return minutes === 0 ? `${hours} h` : `${hours} h ${minutes} min`;
}
