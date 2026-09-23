import { invoke } from "@tauri-apps/api/core";
import { writable } from "svelte/store";
import type { PrefixInfo } from "$lib/types";

export const prefixes = writable<PrefixInfo[]>([]);

export async function refreshPrefixes(): Promise<void> {
  prefixes.set(await invoke<PrefixInfo[]>("list_prefixes"));
}

// Returns the path registered: for a Proton compat data folder, its `pfx/`.
export async function addPrefix(path: string): Promise<string> {
  const added = await invoke<string>("add_prefix", { path });
  await refreshPrefixes();
  return added;
}

// Removes the prefix from Prefixr; with `deleteFiles` also its folder.
export async function deletePrefix(path: string, deleteFiles: boolean): Promise<void> {
  await invoke("delete_prefix", { path, deleteFiles });
  await refreshPrefixes();
}
