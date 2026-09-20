import { invoke } from "@tauri-apps/api/core";
import { writable } from "svelte/store";
import type { PrefixInfo } from "$lib/types";

export const prefixes = writable<PrefixInfo[]>([]);

export async function refreshPrefixes(): Promise<void> {
  prefixes.set(await invoke<PrefixInfo[]>("list_prefixes"));
}

export async function addPrefix(path: string): Promise<void> {
  await invoke("add_prefix", { path });
  await refreshPrefixes();
}

export async function deletePrefix(path: string): Promise<void> {
  await invoke("delete_prefix", { path });
  await refreshPrefixes();
}
