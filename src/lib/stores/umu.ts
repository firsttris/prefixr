import { invoke } from "@tauri-apps/api/core";
import { writable } from "svelte/store";
import type { UmuStatus } from "$lib/types";

export const umuStatus = writable<UmuStatus | null>(null);

export async function refreshUmuStatus(): Promise<void> {
  umuStatus.set(await invoke<UmuStatus>("get_umu_status"));
}

// Downloads the latest umu release, replacing the installed copy if any.
export async function installUmu(): Promise<void> {
  umuStatus.set(await invoke<UmuStatus>("install_umu"));
}
