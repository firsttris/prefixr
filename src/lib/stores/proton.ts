import { invoke } from "@tauri-apps/api/core";
import { writable } from "svelte/store";
import type { ProtonConfig, ProtonOption } from "$lib/types";

export const protonConfig = writable<ProtonConfig | null>(null);

export async function refreshProtonConfig(): Promise<void> {
  protonConfig.set(await invoke<ProtonConfig>("get_proton_config"));
}

export async function saveProtonConfig(config: ProtonConfig): Promise<void> {
  const saved = await invoke<ProtonConfig>("save_proton_config", { config });
  protonConfig.set(saved);
}

// The switches a runner's `proton` script understands. A runner's script
// doesn't change while it's installed, so each one is only parsed once.
const optionsCache = new Map<string, Promise<ProtonOption[]>>();

export function listProtonOptions(runnerId: string): Promise<ProtonOption[]> {
  let options = optionsCache.get(runnerId);
  if (!options) {
    options = invoke<ProtonOption[]>("list_proton_options", { runnerId });
    optionsCache.set(runnerId, options);
    options.catch(() => optionsCache.delete(runnerId));
  }
  return options;
}
