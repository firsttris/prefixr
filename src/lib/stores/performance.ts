import { invoke } from "@tauri-apps/api/core";
import { writable } from "svelte/store";
import type { PerformanceConfig } from "$lib/types";

export const performanceConfig = writable<PerformanceConfig | null>(null);

export async function refreshPerformanceConfig(): Promise<void> {
  performanceConfig.set(await invoke<PerformanceConfig>("get_performance_config"));
}

export async function savePerformanceConfig(config: PerformanceConfig): Promise<void> {
  const saved = await invoke<PerformanceConfig>("save_performance_config", { config });
  performanceConfig.set(saved);
}
