<script lang="ts">
  import { onMount } from "svelte";
  import {
    runners,
    runnerSources,
    refreshRunnerSources,
    runnerReleases,
    refreshRunnerReleases,
    runnerDownloadState,
    downloadRunner,
    initRunnerDownloadEvents,
  } from "$lib/stores/runners";
  import { backendError, t, getLocale } from "$lib/i18n/index.svelte";

  let loading = $state(true);
  let loadError = $state<unknown>(null);
  let selectedSource = $state("");

  onMount(async () => {
    initRunnerDownloadEvents();
    try {
      await refreshRunnerSources();
      selectedSource = $runnerSources[0]?.id ?? "";
      if (selectedSource) {
        await refreshRunnerReleases(selectedSource);
      }
    } catch (e) {
      loadError = e;
    } finally {
      loading = false;
    }
  });

  async function selectSource(id: string): Promise<void> {
    if (id === selectedSource) return;
    selectedSource = id;
    loading = true;
    loadError = null;
    try {
      await refreshRunnerReleases(id);
    } catch (e) {
      loadError = e;
    } finally {
      loading = false;
    }
  }

  function isInstalled(tag: string): boolean {
    return $runners.some((r) => r.id === tag);
  }

  function formatSize(bytes: number): string {
    return `${(bytes / 1024 / 1024).toFixed(0)} MB`;
  }

  function formatDate(iso: string): string {
    return new Date(iso).toLocaleDateString(getLocale() === "de" ? "de-DE" : "en-US", {
      year: "numeric",
      month: "short",
      day: "numeric",
    });
  }
</script>

<div class="tabs">
  {#each $runnerSources as source (source.id)}
    <button
      type="button"
      class="tab"
      class:active={source.id === selectedSource}
      onclick={() => selectSource(source.id)}
    >
      {source.label}
    </button>
  {/each}
</div>

{#if loading}
  <p class="hint">{t("runnerDownloads.loading")}</p>
{:else if loadError}
  <p class="error">{backendError(loadError)}</p>
{:else if $runnerReleases.length === 0}
  <p class="hint">{t("runnerDownloads.noReleases")}</p>
{:else}
  <ul>
    {#each $runnerReleases as release (release.tag)}
      {@const state = $runnerDownloadState[release.tag]}
      {@const installed = isInstalled(release.tag)}
      <li class="release">
        <div class="row">
          <div class="info">
            <span class="name">{release.name}</span>
            <span class="meta">{formatDate(release.published_at)} · {formatSize(release.size)}</span>
          </div>

          {#if installed || state?.done}
            <span class="badge">{t("runnerDownloads.installed")}</span>
          {:else if state && !state.error}
            <div class="progress">
              <div
                class="bar"
                style:width={state.total
                  ? `${Math.min(100, (state.downloaded / state.total) * 100)}%`
                  : "30%"}
              ></div>
            </div>
          {:else}
            <button
              type="button"
              onclick={() => downloadRunner(release.source, release.tag, release.download_url)}
            >
              {t("runnerDownloads.download")}
            </button>
          {/if}
        </div>

        {#if state?.error}
          <p class="error">{backendError(state.error)}</p>
        {/if}
      </li>
    {/each}
  </ul>
{/if}

<style>
  .tabs {
    display: flex;
    gap: 0.4em;
    flex-wrap: wrap;
    margin-bottom: 0.8em;
  }

  .tab {
    padding: 0.4em 0.9em;
    border-radius: 999px;
    background: var(--surface-raised);
    color: var(--text-muted);
    font-size: 0.85em;
    border: 1px solid transparent;
  }

  .tab.active {
    background: var(--accent);
    color: var(--accent-contrast, #fff);
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.5em;
    max-height: 60vh;
    overflow-y: auto;
  }

  .release {
    display: flex;
    flex-direction: column;
    gap: 0.3em;
    padding: 0.6em 0.8em;
    background: var(--surface-raised);
    border-radius: 8px;
  }

  .row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1em;
  }

  .info {
    display: flex;
    flex-direction: column;
    gap: 0.2em;
    text-align: left;
  }

  .name {
    font-weight: 600;
  }

  .meta {
    font-size: 0.8em;
    color: var(--text-muted);
  }

  .badge {
    font-size: 0.8em;
    color: var(--text-muted);
    background: var(--bg);
    padding: 0.3em 0.7em;
    border-radius: 999px;
    white-space: nowrap;
  }

  .progress {
    width: 100px;
    height: 8px;
    background: var(--bg);
    border-radius: 999px;
    overflow: hidden;
    flex-shrink: 0;
  }

  .bar {
    height: 100%;
    background: var(--accent);
    transition: width 0.2s;
  }

  .hint {
    color: var(--text-muted);
  }

  .error {
    color: var(--danger);
    font-size: 0.85em;
  }
</style>
