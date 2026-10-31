<script lang="ts">
  import * as m from "#lib/paraglide/messages.js";
  import type { Compatibility, Game } from "#lib/types.ts";
  import { killGame, launchGame, type GameRunState } from "#lib/stores/games.ts";
  import { exeIconUrl } from "#lib/stores/steamgriddb.ts";
  import { storeName } from "#lib/stores/gameStores.ts";
  import type { GameTest } from "#lib/stores/libraryMeta.ts";
  import GameActionsMenu from "./GameActionsMenu.svelte";
  import TestDot from "./TestDot.svelte";
  import CompatBadges from "./CompatBadges.svelte";
  import FavoriteStar from "./FavoriteStar.svelte";
  import GameFallbackIcon from "./GameFallbackIcon.svelte";
  import GameItemNotices from "./GameItemNotices.svelte";
  import { GameItem } from "./gameItem.svelte";

  let {
    game,
    runState,
    inSteam,
    onEdit,
    onEditArtwork,
    onRemove,
    onExportToSteam,
    onRemoveFromSteam,
    onShowNotes,
    favorite,
    compat,
    onToggleFavorite,
    onOpenDetails,
    test,
  }: {
    game: Game;
    runState?: GameRunState;
    inSteam: boolean;
    onEdit: () => void;
    onEditArtwork: () => void;
    onRemove: () => void;
    onExportToSteam: () => void;
    onRemoveFromSteam: () => void;
    onShowNotes: () => void;
    favorite: boolean;
    compat?: Compatibility;
    onToggleFavorite: () => void;
    onOpenDetails: () => void;
    test: GameTest | null;
  } = $props();

  const item = new GameItem(() => game);
</script>

<article
  class="card"
  class:running={runState?.running}
  oncontextmenu={(e) => {
    e.preventDefault();
    onOpenDetails();
  }}
>
  <div class="cover-wrap">
    <button
      type="button"
      class="cover-hit"
      aria-label={m.details_openFor({ name: game.name })}
      title={game.name}
      onclick={onOpenDetails}
    ></button>
    {#if item.coverSrc}
      <img class="cover" src={item.coverSrc} alt="" loading="lazy" decoding="async" />
    {:else}
      <div class="cover placeholder">
        {#if game.icon}
          <img class="icon" src={exeIconUrl(game)} alt="" />
        {:else}
          <GameFallbackIcon class="icon fallback" />
        {/if}
      </div>
    {/if}

    {#if runState?.running}
      <span class="status-badge running"><span class="dot"></span>{m.gameCard_running()}</span>
    {:else if runState?.initializing}
      <span class="status-badge init"><span class="dot"></span>{m.gameCard_initializing()}</span>
    {:else if test}
      <span class="test-corner"><TestDot {test} onImage /></span>
    {/if}

    <div class="menu-corner" class:favorite>
      <FavoriteStar {favorite} onToggle={onToggleFavorite} onImage />
      <GameActionsMenu
        {item}
        {inSteam}
        full
        label={m.gameCard_moreOptions()}
        triggerClass="icon-btn on-image menu-trigger"
        {onEdit}
        {onEditArtwork}
        {onExportToSteam}
        {onRemoveFromSteam}
        onShowNotes={game.notes ? onShowNotes : undefined}
      >
        {#snippet trigger()}⋮{/snippet}
      </GameActionsMenu>
    </div>

    <div class="overlay">
      <!-- The title spans the card's full width; only the pills share their
           row with the play button, so long names aren't cut off early. -->
      <h3 title={game.name}>{game.name}</h3>
      <div class="bottom-row">
        <div class="pills">
          {#if game.store}
            <span class="runner">{storeName(game.store.store)}</span>
          {/if}
          <span class="runner">{game.runner_id}</span>
          <CompatBadges {compat} />
        </div>

        <button
          type="button"
          class="play-btn"
          class:running={runState?.running}
          onclick={() => (runState?.running ? killGame(game.id) : launchGame(game.id))}
          disabled={runState?.initializing}
          aria-label={runState?.running
            ? m.gameCard_playKill()
            : runState?.initializing
              ? m.gameCard_playInitializing()
              : m.gameCard_playStart()}
          title={runState?.running
            ? m.gameCard_playKill()
            : runState?.initializing
              ? m.gameCard_playInitializing()
              : m.gameCard_playStart()}
        >
          {#if runState?.initializing}
            <svg class="play-icon spin" viewBox="0 0 20 20" fill="none" aria-hidden="true">
              <circle
                cx="10"
                cy="10"
                r="7"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-dasharray="24 20"
              />
            </svg>
          {:else if runState?.running}
            <svg class="play-icon" viewBox="0 0 20 20" fill="none" aria-hidden="true">
              <rect x="6" y="6" width="8" height="8" rx="1.5" fill="currentColor" />
            </svg>
          {:else}
            <svg class="play-icon" viewBox="0 0 20 20" fill="none" aria-hidden="true">
              <path d="M7.2 5.3v9.4l7.6-4.7-7.6-4.7z" fill="currentColor" />
            </svg>
          {/if}
        </button>
      </div>
    </div>
  </div>

  <GameItemNotices {game} {item} {runState} {inSteam} {onRemove} />
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

  .card.running {
    border-color: var(--success);
    box-shadow: 0 0 0 1px var(--success);
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
    object-fit: contain;
    image-rendering: -webkit-optimize-contrast;
  }

  .status-badge {
    position: absolute;
    top: 0.6em;
    left: 0.6em;
    display: flex;
    align-items: center;
    gap: 0.4em;
    font-size: 0.72em;
    font-weight: 600;
    padding: 0.3em 0.65em;
    border-radius: 999px;
  }

  .test-corner {
    position: absolute;
    top: 0.6em;
    left: 0.6em;
    z-index: 2;
    display: flex;
  }

  /* No backdrop blur on the cards: one per pill and button over a
     thousand covers makes scrolling stutter. Tints sit on a dark base
     instead, so they still read on bright cover art. */
  .status-badge.running {
    background: linear-gradient(var(--success-bg), var(--success-bg)) rgba(15, 16, 20, 0.75);
    color: var(--success);
    border: 1px solid rgba(62, 207, 142, 0.4);
  }

  .status-badge.init {
    background: linear-gradient(rgba(91, 140, 255, 0.18), rgba(91, 140, 255, 0.18))
      rgba(15, 16, 20, 0.75);
    color: var(--accent);
    border: 1px solid rgba(91, 140, 255, 0.4);
  }

  .dot {
    width: 0.5em;
    height: 0.5em;
    border-radius: 50%;
    background: currentColor;
  }

  .status-badge.running .dot {
    animation: pulse 1.4s ease-in-out infinite;
  }

  @keyframes pulse {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.35;
    }
  }

  /* Bottom overlay: sits on top of the cover art, scrim fades from
     near-opaque at the bottom (where text/buttons live) to fully
     transparent, so it works on both bright and dark cover art. */
  .overlay {
    position: absolute;
    inset: auto 0 0 0;
    z-index: 2;
    display: flex;
    flex-direction: column;
    gap: 0.3em;
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
    text-align: left;
    font-size: 0.92em;
    color: #fff;
    text-shadow: 0 1px 4px rgba(0, 0, 0, 0.7);
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .pills {
    display: flex;
    flex-wrap: wrap;
    gap: 0.3em;
  }

  .runner {
    align-self: flex-start;
    font-size: 0.72em;
    color: #fff;
    background: rgba(255, 255, 255, 0.14);
    border: 1px solid rgba(255, 255, 255, 0.18);
    padding: 0.15em 0.6em;
    border-radius: 999px;
  }

  .menu-corner :global(.menu-trigger) {
    font-size: 1.1em;
    font-weight: 700;
  }

  /* Icon buttons need their own dark translucent background here since
     they float directly on cover art instead of a themed surface. */
  .menu-corner :global(.icon-btn.on-image) {
    background: rgba(15, 16, 20, 0.7);
    border-color: rgba(255, 255, 255, 0.18);
    color: #fff;
  }

  .menu-corner :global(.icon-btn.on-image:hover:not(:disabled)) {
    background: rgba(15, 16, 20, 0.75);
    border-color: var(--accent);
    color: var(--accent);
  }

  .menu-corner {
    position: absolute;
    top: 0.6em;
    right: 0.6em;
    z-index: 5;
    display: flex;
    gap: 0.3em;
  }

  /* The star only shows on a favorite, or while the card is pointed at. */
  .menu-corner:not(.favorite) :global(.star) {
    opacity: 0;
    transition: opacity 0.15s;
  }

  .card:hover .menu-corner :global(.star),
  .card:focus-within .menu-corner :global(.star) {
    opacity: 1;
  }

  /* The whole cover opens the details; the overlay's text lets clicks
     through to it, its buttons don't. */
  .cover-hit {
    position: absolute;
    inset: 0;
    z-index: 1;
    padding: 0;
    border: none;
    border-radius: 0;
    background: transparent;
    cursor: pointer;
  }

  .cover-hit:focus-visible {
    outline-offset: -3px;
  }

  .overlay {
    pointer-events: none;
  }

  /* The badges keep their tooltips; the cut-off title has the cover's. */
  .overlay :global(button),
  .overlay :global(.badges) {
    pointer-events: auto;
  }

  .bottom-row {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 0.6em;
  }

  .bottom-row .pills {
    flex: 1;
    min-width: 0;
  }

  /* Play/stop control: a translucent glass circle, not a solid CTA bar,
     so the cover art still shows through underneath it. */
  .play-btn {
    flex-shrink: 0;
    width: 2.9em;
    height: 2.9em;
    padding: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 50%;
    background: rgba(20, 20, 24, 0.6);
    border: 1px solid rgba(255, 255, 255, 0.4);
    color: #fff;
    transition:
      background-color 0.15s,
      border-color 0.15s;
  }

  .play-btn:hover:not(:disabled) {
    background: var(--accent);
    border-color: var(--accent);
  }

  .play-btn.running {
    background: rgba(239, 83, 80, 0.5);
    border-color: rgba(239, 83, 80, 0.8);
  }

  .play-btn.running:hover:not(:disabled) {
    background: var(--danger);
    border-color: var(--danger);
  }

  .play-icon {
    width: 1.5em;
    height: 1.5em;
  }

  .play-icon.spin {
    animation: spin 0.9s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
