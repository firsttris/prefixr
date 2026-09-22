import { invoke } from "@tauri-apps/api/core";
import { writable } from "svelte/store";
import type { GitHubConfig } from "$lib/types";

export const githubConfig = writable<GitHubConfig | null>(null);

export async function refreshGitHubConfig(): Promise<void> {
  githubConfig.set(await invoke<GitHubConfig>("get_github_config"));
}

export async function saveGitHubConfig(config: GitHubConfig): Promise<void> {
  const saved = await invoke<GitHubConfig>("save_github_config", { config });
  githubConfig.set(saved);
}
