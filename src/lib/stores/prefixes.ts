import { invoke } from "@tauri-apps/api/core";
import { writable } from "svelte/store";
import type { PrefixInfo, PrefixState, PrefixStatus } from "#lib/types.ts";

export const prefixes = writable<PrefixInfo[]>([]);

// How each listed prefix's folder is, by path. Only the prefixes page asks
// for it: it means a look at every folder, which can take a while on a
// network share or a sleeping drive.
export const prefixStates = writable<Record<string, PrefixState>>({});

export async function refreshPrefixes(): Promise<void> {
  prefixes.set(await invoke<PrefixInfo[]>("list_prefixes"));
}

export async function refreshPrefixStates(): Promise<void> {
  const states = await invoke<PrefixStatus[]>("prefix_states");
  prefixStates.set(Object.fromEntries(states.map((s) => [s.path, s.state])));
}

// Returns the path registered: for a Proton compat data folder, its `pfx/`.
export async function addPrefix(path: string): Promise<string> {
  const added = await invoke<string>("add_prefix", { path });
  await refreshPrefixes();
  return added;
}

// Removes the prefix from Prefixr only; the directory on disk is left as-is.
export async function deletePrefix(path: string): Promise<void> {
  await invoke("delete_prefix", { path });
  await refreshPrefixes();
}
