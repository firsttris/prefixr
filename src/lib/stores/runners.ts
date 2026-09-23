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

// GitHub's unauthenticated API rate limit (60 requests/hour) is easy to
// exhaust if every tab switch re-fetches releases from scratch, so cache
// each source's list for a while and only bypass it on an explicit refresh.
const RELEASES_CACHE_TTL_MS = 5 * 60 * 1000;
const releasesCache = new Map<string, { releases: RunnerRelease[]; fetchedAt: number }>();

export async function refreshRunnerReleases(source: string, force = false): Promise<void> {
  const cached = releasesCache.get(source);
  if (!force && cached && Date.now() - cached.fetchedAt < RELEASES_CACHE_TTL_MS) {
    runnerReleases.set(cached.releases);
    return;
  }
  const releases = await invoke<RunnerRelease[]>("list_runner_releases", { source });
  releasesCache.set(source, { releases, fetchedAt: Date.now() });
  runnerReleases.set(releases);
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
  } catch (e) {
    // Usually already set via the runner-download-error event, which isn't
    // sent for a failure before the download got going (e.g. no network).
    patchDownloadState(tag, { error: String(e) });
  }
}
