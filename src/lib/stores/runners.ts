import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { writable } from "svelte/store";
import type { Runner, RunnerRelease, RunnerSourceInfo } from "$lib/types";

export const runners = writable<Runner[]>([]);

export async function refreshRunners(): Promise<void> {
  runners.set(await invoke<Runner[]>("list_runners"));
}

export const runnerSources = writable<RunnerSourceInfo[]>([]);

export async function refreshRunnerSources(): Promise<void> {
  runnerSources.set(await invoke<RunnerSourceInfo[]>("list_runner_sources"));
}

export const runnerReleases = writable<RunnerRelease[]>([]);

export async function refreshRunnerReleases(source: string): Promise<void> {
  runnerReleases.set(await invoke<RunnerRelease[]>("list_runner_releases", { source }));
}

export interface RunnerDownloadState {
  downloaded: number;
  total?: number;
  done: boolean;
  error?: string;
}

export const runnerDownloadState = writable<Record<string, RunnerDownloadState>>({});

function patchDownloadState(tag: string, patch: Partial<RunnerDownloadState>) {
  runnerDownloadState.update((state) => {
    const current: RunnerDownloadState = state[tag] ?? { downloaded: 0, done: false };
    return { ...state, [tag]: { ...current, ...patch } };
  });
}

interface DownloadProgressPayload {
  tag: string;
  downloaded: number;
  total: number | null;
}

interface DownloadErrorPayload {
  tag: string;
  message: string;
}

interface DownloadDonePayload {
  tag: string;
}

let eventsInitialized = false;

// Registers the download-progress listeners once; must run client-side only
// (call from onMount), since it touches the Tauri IPC bridge.
export function initRunnerDownloadEvents(): void {
  if (eventsInitialized) return;
  eventsInitialized = true;

  listen<DownloadProgressPayload>("runner-download-progress", (event) => {
    patchDownloadState(event.payload.tag, {
      downloaded: event.payload.downloaded,
      total: event.payload.total ?? undefined,
    });
  });

  listen<DownloadDonePayload>("runner-download-done", (event) => {
    patchDownloadState(event.payload.tag, { done: true, error: undefined });
    refreshRunners();
  });

  listen<DownloadErrorPayload>("runner-download-error", (event) => {
    patchDownloadState(event.payload.tag, { done: false, error: event.payload.message });
  });
}

export async function downloadRunner(
  source: string,
  tag: string,
  downloadUrl: string,
): Promise<void> {
  patchDownloadState(tag, { downloaded: 0, total: undefined, done: false, error: undefined });
  try {
    await invoke("download_runner", { source, tag, downloadUrl });
  } catch {
    // Outcome is already surfaced via the runner-download-error event.
  }
}
