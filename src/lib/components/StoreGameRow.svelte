<!-- StoreGameCard for the library's list view, laid out like GameListRow. -->
<script lang="ts">
  import * as m from "$lib/paraglide/messages";
  import type { StoreGame } from "$lib/types";
  import { storeCoverUrl, storeName } from "$lib/stores/gameStores";
  import { backendError } from "$lib/i18n/index.svelte";
  import GameFallbackIcon from "./GameFallbackIcon.svelte";
  import StoreInstallError from "./StoreInstallError.svelte";

  let {
    game,
    progress,
    error,
    onInstall,
    onCancel,
    onDismissError,
  }: {
    game: StoreGame;
    progress?: number;
    error?: unknown;
    onInstall: () => void;
    onCancel: () => void;
    onDismissError: () => void;
  } = $props();

  let coverSrc = $derived(storeCoverUrl(game));
  let installing = $derived(progress !== undefined);
</script>

<div class="wrap">
  <div class="row" class:installing>
    <div class="thumb">
      {#if coverSrc}
        <img class="cover" src={coverSrc} alt="" loading="lazy" decoding="async" />
      {:else}
        <GameFallbackIcon class="icon fallback" />
      {/if}
    </div>

    <div class="meta">
      <span class="name" title={game.title}>{game.title}</span>
      <span class="pill">{storeName(game.store)} · {m.storeGame_notInstalled()}</span>
    </div>

    {#if installing}
      <div
        class="bar"
        role="progressbar"
        aria-label={m.storeGame_progressLabel({ name: game.title })}
        aria-valuemin="0"
        aria-valuemax="100"
        aria-valuenow={Math.floor(progress ?? 0)}
      >
        <div class="fill" style:width="{progress ?? 0}%"></div>
      </div>
      <span class="percent">{m.storeGame_percent({ percent: Math.floor(progress ?? 0) })}</span>
      <button type="button" onclick={onCancel}>{m.common_cancel()}</button>
    {:else}
      <button type="button" class="primary" onclick={onInstall}>{m.storeGame_install()}</button>
    {/if}
  </div>
  {#if error}
    <StoreInstallError message={backendError(error)} {error} inline onDismiss={onDismissError} />
  {/if}
</div>

<style>
  .row {
    display: flex;
    align-items: center;
    gap: 1em;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 0.6em 0.8em;
  }

  .row.installing {
    border-color: var(--accent);
  }

  .thumb {
    flex-shrink: 0;
    width: 2.8em;
    height: 2.8em;
    border-radius: 6px;
    overflow: hidden;
    background: var(--surface-raised);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .thumb .cover {
    width: 100%;
    height: 100%;
    object-fit: cover;
    filter: saturate(0.75) brightness(0.85);
  }

  .thumb :global(.icon) {
    width: 1.7em;
    height: 1.7em;
    opacity: 0.6;
  }

  .meta {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: baseline;
    gap: 0.7em;
  }

  .name {
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--text-muted);
  }

  .pill {
    flex-shrink: 0;
    font-size: 0.75em;
    color: var(--text-muted);
    background: var(--surface-raised);
    padding: 0.15em 0.6em;
    border-radius: 999px;
  }

  .bar {
    width: 8em;
    height: 6px;
    border-radius: 999px;
    background: var(--surface-raised);
    overflow: hidden;
    flex-shrink: 0;
  }

  .fill {
    height: 100%;
    background: var(--accent);
    transition: width 0.3s ease;
  }

  .percent {
    font-size: 0.8em;
    color: var(--text-muted);
    width: 3em;
    text-align: right;
  }
</style>
