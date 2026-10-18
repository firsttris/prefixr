<!--
  The store installs in one place: the ones running, with their speed and
  time left once the download's size is known, and the ones that failed,
  to try again or dismiss.
-->
<script lang="ts">
  import * as m from "$lib/paraglide/messages";
  import { untrack } from "svelte";
  import type { StoreGame } from "$lib/types";
  import {
    cancelStoreInstall,
    dismissStoreInstallError,
    formatBytes,
    getStoreInstallDefaults,
    storeCoverUrl,
    storeGames,
    storeInstallErrors,
    storeInstalls,
    storeName,
  } from "$lib/stores/gameStores";
  import { storeEntryKey } from "$lib/stores/libraryMeta";
  import { addSample, estimate, formatDuration, type Sample } from "$lib/downloadRate";
  import { backendError, getLocale } from "$lib/i18n/index.svelte";
  import GameFallbackIcon from "./GameFallbackIcon.svelte";

  let { onInstallStoreGame }: { onInstallStoreGame: (game: StoreGame) => void } = $props();

  let byKey = $derived(new Map($storeGames.map((game) => [storeEntryKey(game), game])));
  let active = $derived(Object.entries($storeInstalls));
  let failed = $derived(
    Object.entries($storeInstallErrors).filter(([key]) => !(key in $storeInstalls)),
  );

  // Progress samples and download sizes, by key, while this view is open;
  // each progress update adds a sample.
  let samples = $state<Record<string, Sample[]>>({});
  let sizes = $state<Record<string, number | null>>({});

  $effect(() => {
    const now = Date.now();
    const current = active;
    untrack(() => {
      const next: Record<string, Sample[]> = {};
      for (const [key, percent] of current) {
        next[key] = addSample(samples[key] ?? [], { at: now, percent });
      }
      samples = next;
    });
  });

  $effect(() => {
    const keys = active.map(([key]) => key);
    untrack(() => {
      for (const key of keys) {
        const game = byKey.get(key);
        if (!game || key in sizes) continue;
        sizes[key] = null;
        getStoreInstallDefaults(game.store, game.app_name)
          .then((defaults) => (sizes[key] = defaults.sizes.download_size))
          .catch(() => {});
      }
    });
  });
</script>

{#snippet thumb(game: StoreGame | undefined)}
  <div class="thumb">
    {#if game && storeCoverUrl(game)}
      <img src={storeCoverUrl(game)} alt="" />
    {:else}
      <GameFallbackIcon class="icon fallback" />
    {/if}
  </div>
{/snippet}

{#if active.length === 0 && failed.length === 0}
  <div class="empty">
    <span class="empty-icon">⬇</span>
    <h3>{m.downloads_empty_title()}</h3>
    <p>{m.downloads_empty_hint()}</p>
  </div>
{/if}

{#if active.length > 0}
  <h2 class="group">{m.downloads_active()}</h2>
  <ul class="downloads">
    {#each active as [key, percent] (key)}
      {@const game = byKey.get(key)}
      {@const rate = estimate(samples[key] ?? [], sizes[key])}
      <li class="download">
        {@render thumb(game)}
        <div class="info">
          <div class="title-row">
            <span class="name">{game?.title ?? key}</span>
            <span class="muted">{game ? storeName(game.store) : ""}</span>
          </div>
          <div
            class="bar"
            role="progressbar"
            aria-label={m.storeGame_progressLabel({ name: game?.title ?? key })}
            aria-valuemin="0"
            aria-valuemax="100"
            aria-valuenow={Math.floor(percent)}
          >
            <div class="fill" style:width="{percent}%"></div>
          </div>
          <div class="stats">
            {#if percent <= 0}
              <span>{m.downloads_preparing()}</span>
            {:else}
              <span>{m.storeGame_percent({ percent: Math.floor(percent) })}</span>
              {#if rate}
                <span
                  >{m.downloads_speed({
                    speed: formatBytes(rate.bytesPerSecond, getLocale()) ?? "",
                  })}</span
                >
                <span>{m.downloads_remaining({ time: formatDuration(rate.secondsLeft) })}</span>
              {/if}
            {/if}
          </div>
        </div>
        <button
          type="button"
          onclick={() => game && cancelStoreInstall(game.store, game.app_name)}
          disabled={!game}>{m.common_cancel()}</button
        >
      </li>
    {/each}
  </ul>
{/if}

{#if failed.length > 0}
  <h2 class="group">{m.downloads_failed()}</h2>
  <ul class="downloads">
    {#each failed as [key, error] (key)}
      {@const game = byKey.get(key)}
      <li class="download failed">
        {@render thumb(game)}
        <div class="info">
          <span class="name">{game?.title ?? key}</span>
          <span class="error">{backendError(error)}</span>
        </div>
        {#if game}
          <button type="button" class="primary" onclick={() => onInstallStoreGame(game)}
            >{m.downloads_retry()}</button
          >
          <button
            type="button"
            class="ghost"
            onclick={() => dismissStoreInstallError(game.store, game.app_name)}
            >{m.downloads_dismiss()}</button
          >
        {/if}
      </li>
    {/each}
  </ul>
{/if}

<style>
  .group {
    font-size: 0.85em;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted);
    margin: 0.5em 0 0.8em;
  }

  .downloads {
    list-style: none;
    margin: 0 0 1.6em;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.6em;
    max-width: 780px;
  }

  .download {
    display: flex;
    align-items: center;
    gap: 1em;
    padding: 0.7em 0.9em;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  .download.failed {
    border-color: rgba(239, 83, 80, 0.4);
  }

  .thumb {
    flex-shrink: 0;
    width: 3em;
    height: 4.5em;
    border-radius: 6px;
    overflow: hidden;
    background: var(--surface-raised);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .thumb img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .thumb :global(.icon) {
    width: 1.6em;
    height: 1.6em;
  }

  .info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 0.4em;
  }

  .title-row {
    display: flex;
    align-items: baseline;
    gap: 0.6em;
  }

  .name {
    font-weight: 600;
  }

  .muted,
  .stats {
    color: var(--text-muted);
    font-size: 0.85em;
  }

  .stats {
    display: flex;
    gap: 1em;
  }

  .error {
    color: var(--danger);
    font-size: 0.85em;
    overflow-wrap: anywhere;
  }

  .bar {
    height: 6px;
    border-radius: 999px;
    background: var(--surface-raised);
    overflow: hidden;
  }

  .fill {
    height: 100%;
    background: var(--accent);
    transition: width 0.3s;
  }

  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.5em;
    color: var(--text-muted);
    text-align: center;
    padding: 4em 1em;
    border: 1px dashed var(--border);
    border-radius: var(--radius);
    max-width: 780px;
  }

  .empty-icon {
    font-size: 2.4em;
  }

  .empty h3 {
    color: var(--text);
  }
</style>
