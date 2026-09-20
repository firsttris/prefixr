<script lang="ts">
  import { onMount } from "svelte";
  import {
    runners,
    protonGeReleases,
    refreshProtonGeReleases,
    runnerDownloadState,
    downloadRunner,
    initRunnerDownloadEvents,
  } from "$lib/stores/runners";

  let loading = $state(true);
  let loadError = $state("");

  onMount(async () => {
    initRunnerDownloadEvents();
    try {
      await refreshProtonGeReleases();
    } catch (e) {
      loadError = String(e);
    } finally {
      loading = false;
    }
  });

  function isInstalled(tag: string): boolean {
    return $runners.some((r) => r.id === tag);
  }

  function formatSize(bytes: number): string {
    return `${(bytes / 1024 / 1024).toFixed(0)} MB`;
  }

  function formatDate(iso: string): string {
    return new Date(iso).toLocaleDateString("de-DE", {
      year: "numeric",
      month: "short",
      day: "numeric",
    });
  }
</script>

{#if loading}
  <p class="hint">Lade Versionen von GitHub…</p>
{:else if loadError}
  <p class="error">{loadError}</p>
{:else if $protonGeReleases.length === 0}
  <p class="hint">Keine Releases gefunden.</p>
{:else}
  <ul>
    {#each $protonGeReleases as release (release.tag)}
      {@const state = $runnerDownloadState[release.tag]}
      {@const installed = isInstalled(release.tag)}
      <li class="release">
        <div class="row">
          <div class="info">
            <span class="name">{release.name}</span>
            <span class="meta">{formatDate(release.published_at)} · {formatSize(release.size)}</span>
          </div>

          {#if installed || state?.done}
            <span class="badge">Installiert</span>
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
            <button type="button" onclick={() => downloadRunner(release.tag, release.download_url)}>
              Herunterladen
            </button>
          {/if}
        </div>

        {#if state?.error}
          <p class="error">{state.error}</p>
        {/if}
      </li>
    {/each}
  </ul>
{/if}

<style>
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
