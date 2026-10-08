import { invoke } from "@tauri-apps/api/core";
import { writable } from "svelte/store";
import type { DirectXLayerStatus } from "$lib/types";

export const directXLayers = writable<DirectXLayerStatus[]>([]);

export async function refreshDirectXLayers(): Promise<void> {
  directXLayers.set(await invoke<DirectXLayerStatus[]>("get_directx_layers_status"));
}

// Replaces each cached layer that isn't the latest release anymore.
export async function updateDirectXLayers(): Promise<void> {
  directXLayers.set(await invoke<DirectXLayerStatus[]>("update_directx_layers"));
}
