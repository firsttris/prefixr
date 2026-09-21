import { invoke } from "@tauri-apps/api/core";
import { writable } from "svelte/store";
import type { MangoHudConfig } from "$lib/types";

export const mangoHudConfig = writable<MangoHudConfig | null>(null);

export async function refreshMangoHudConfig(): Promise<void> {
  mangoHudConfig.set(await invoke<MangoHudConfig>("get_mangohud_config"));
}

export async function saveMangoHudConfig(config: MangoHudConfig): Promise<void> {
  const saved = await invoke<MangoHudConfig>("save_mangohud_config", { config });
  mangoHudConfig.set(saved);
}
