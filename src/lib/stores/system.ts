import { invoke } from "@tauri-apps/api/core";
import { writable } from "svelte/store";
import type { MaxMapCountStatus } from "$lib/types";

export const maxMapCountStatus = writable<MaxMapCountStatus | null>(null);

export async function refreshMaxMapCountStatus(): Promise<void> {
  maxMapCountStatus.set(await invoke<MaxMapCountStatus>("check_max_map_count"));
}

export async function fixMaxMapCount(): Promise<void> {
  await invoke("fix_max_map_count");
  await refreshMaxMapCountStatus();
}
