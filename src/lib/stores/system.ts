import { invoke } from "@tauri-apps/api/core";
import { getVersion } from "@tauri-apps/api/app";
import { writable } from "svelte/store";
import type { MaxMapCountStatus } from "#lib/types.ts";

export const maxMapCountStatus = writable<MaxMapCountStatus | null>(null);

export async function refreshMaxMapCountStatus(): Promise<void> {
  maxMapCountStatus.set(await invoke<MaxMapCountStatus>("check_max_map_count"));
}

export async function fixMaxMapCount(): Promise<void> {
  await invoke("fix_max_map_count");
  await refreshMaxMapCountStatus();
}

// The running Prefixr's version, shown at the bottom of the sidebar.
export const appVersion = writable<string | null>(null);

export async function refreshAppVersion(): Promise<void> {
  appVersion.set(await getVersion());
}

// Opens this version's release notes on GitHub in the browser.
export async function openReleaseNotes(): Promise<void> {
  await invoke("open_release_notes");
}
