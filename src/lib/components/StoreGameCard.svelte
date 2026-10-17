<!--
  A game of a connected store account that isn't installed yet, as a card
  in the library grid: its cover, and installing it — or the install's
  progress while it runs. Laid out like GameCard, so both sit in one grid.
-->
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

<article class="card" class:installing>
  <div class="cover-wrap">
    {#if coverSrc}
      <img class="cover" src={coverSrc} alt="" loading="lazy" decoding="async" />
    {:else}
      <div class="cover placeholder">
        <GameFallbackIcon class="icon fallback" />
      </div>
    {/if}

    <span class="source">{storeName(game.store)}</span>

    <div class="overlay">
      <h3 title={game.title}>{game.title}</h3>
      {#if installing}
        <div class="progress-row">
          <span>{m.storeGame_installing()}</span>
          <span>{m.storeGame_percent({ percent: Math.floor(progress ?? 0) })}</span>
        </div>
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
        <button type="button" class="cancel" onclick={onCancel}>{m.common_cancel()}</button>
      {:else}
        <button type="button" class="install" onclick={onInstall}>
          <svg viewBox="0 0 20 20" fill="none" aria-hidden="true">
            <path
              d="M10 3.5v9M6 8.8l4 4 4-4M4.5 16h11"
              stroke="currentColor"
              stroke-width="1.7"
              stroke-linecap="round"
              stroke-linejoin="round"
            />
          </svg>
          {m.storeGame_install()}
        </button>
      {/if}
    </div>
  </div>

  {#if error}
    <StoreInstallError message={backendError(error)} {error} onDismiss={onDismissError} />
  {/if}
</article>

<style>
  .card {
    display: flex;
    flex-direction: column;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
  }

  .card.installing {
    border-color: var(--accent);
  }

  .cover-wrap {
    position: relative;
    aspect-ratio: 2 / 3;
    overflow: hidden;
    background: var(--surface-raised);
  }

  .cover {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  /* Not installed: the cover a little dimmer than an installed game's. */
  .card:not(.installing) .cover {
    filter: saturate(0.75) brightness(0.85);
  }

  .cover.placeholder {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .placeholder :global(.icon) {
    width: 45%;
    height: 45%;
    opacity: 0.6;
  }

  .source {
    position: absolute;
    top: 0.6em;
    left: 0.6em;
    font-size: 0.72em;
    color: #fff;
    background: rgba(15, 16, 20, 0.62);
    border: 1px solid rgba(255, 255, 255, 0.2);
    backdrop-filter: blur(6px);
    padding: 0.15em 0.6em;
    border-radius: 999px;
  }

  .overlay {
    position: absolute;
    inset: auto 0 0 0;
    display: flex;
    flex-direction: column;
    gap: 0.5em;
    padding: 2.6em 0.65em 0.65em;
    background: linear-gradient(
      to top,
      rgba(8, 9, 12, 0.95) 0%,
      rgba(8, 9, 12, 0.86) 35%,
      rgba(8, 9, 12, 0.5) 65%,
      rgba(8, 9, 12, 0) 100%
    );
  }

  h3 {
    font-size: 0.92em;
    color: #fff;
    text-shadow: 0 1px 4px rgba(0, 0, 0, 0.7);
    text-align: left;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .install,
  .cancel {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.4em;
    width: 100%;
    font-size: 0.85em;
    padding: 0.45em 0.6em;
  }

  .install {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
    font-weight: 600;
  }

  .install:hover:not(:disabled) {
    background: var(--accent-hover);
    border-color: var(--accent-hover);
  }

  .install svg {
    width: 1.1em;
    height: 1.1em;
  }

  .cancel {
    background: rgba(15, 16, 20, 0.55);
    border-color: rgba(255, 255, 255, 0.2);
    color: #fff;
  }

  .progress-row {
    display: flex;
    justify-content: space-between;
    font-size: 0.78em;
    color: rgba(255, 255, 255, 0.85);
  }

  .bar {
    height: 6px;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.18);
    overflow: hidden;
  }

  .fill {
    height: 100%;
    background: var(--accent);
    border-radius: 999px;
    transition: width 0.3s ease;
  }
</style>
