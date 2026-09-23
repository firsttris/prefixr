import { invoke } from "@tauri-apps/api/core";
import { writable } from "svelte/store";
import type { UmuMatch, UmuStatus } from "$lib/types";

export const umuStatus = writable<UmuStatus | null>(null);

export async function refreshUmuStatus(): Promise<void> {
  umuStatus.set(await invoke<UmuStatus>("get_umu_status"));
}

// The tag of umu's latest release, or null while unknown (not checked yet,
// or GitHub unreachable). Checked at most once per session: each check is a
// GitHub API request, whose anonymous rate limit is low.
export const latestUmuVersion = writable<string | null>(null);
let latestChecked = false;

export async function checkUmuUpdate(): Promise<void> {
  if (latestChecked) return;
  latestChecked = true;
  try {
    latestUmuVersion.set(await invoke<string>("latest_umu_version"));
  } catch {
    // Only a hint; the install button still fetches the latest release.
    latestChecked = false;
  }
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
