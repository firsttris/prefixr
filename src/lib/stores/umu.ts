import { invoke } from "@tauri-apps/api/core";
import { writable } from "svelte/store";
import type { UmuMatch, UmuStatus } from "$lib/types";

export const umuStatus = writable<UmuStatus | null>(null);

export async function refreshUmuStatus(): Promise<void> {
  umuStatus.set(await invoke<UmuStatus>("get_umu_status"));
}

// Downloads the latest umu release, replacing the installed copy if any.
export async function installUmu(): Promise<void> {
  umuStatus.set(await invoke<UmuStatus>("install_umu"));
}

// Suggests UMU ids for a game by name, preferring the Steam app of its
// SteamGridDB match when it has one.
export async function searchUmuIds(
  query: string,
  steamgriddbId: number | null,
): Promise<UmuMatch[]> {
  return await invoke<UmuMatch[]>("search_umu_ids", { query, steamgriddbId });
}
