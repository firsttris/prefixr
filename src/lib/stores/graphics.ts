import { invoke } from "@tauri-apps/api/core";
import { writable } from "svelte/store";
import type { GraphicsConfig } from "$lib/types";

export const graphicsConfig = writable<GraphicsConfig | null>(null);

export async function refreshGraphicsConfig(): Promise<void> {
  graphicsConfig.set(await invoke<GraphicsConfig>("get_graphics_config"));
}

export async function saveGraphicsConfig(config: GraphicsConfig): Promise<void> {
  const saved = await invoke<GraphicsConfig>("save_graphics_config", { config });
  graphicsConfig.set(saved);
}
