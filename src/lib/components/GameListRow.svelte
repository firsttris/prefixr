<script lang="ts">
  import * as m from "$lib/paraglide/messages";
  import type { Game } from "$lib/types";
  import { killGame, launchGame, type GameRunState } from "$lib/stores/games";
  import { exeIconUrl } from "$lib/stores/steamgriddb";
  import GameActionsMenu from "./GameActionsMenu.svelte";
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
  }: {
    game: Game;
    runState?: GameRunState;
    inSteam: boolean;
    onEdit: () => void;
    onEditArtwork: () => void;
    onRemove: () => void;
    onExportToSteam: () => void;
    onRemoveFromSteam: () => void;
  } = $props();

  const item = new GameItem(() => game);
</script>

<div class="row" class:running={runState?.running}>
  <div class="thumb">
    {#if item.coverSrc}
      <img class="cover" src={item.coverSrc} alt="" loading="lazy" decoding="async" />
    {:else if game.icon}
      <img class="icon" src={exeIconUrl(game)} alt="" />
    {:else}
      <GameFallbackIcon class="icon fallback" />
    {/if}
  </div>

  <div class="meta">
    <span class="name" title={game.name}>{game.name}</span>
    <span class="runner">{game.runner_id}</span>
  </div>

  <div class="status">
    {#if runState?.running}
      <span class="status-badge running"><span class="dot"></span>{m.gameCard_running()}</span>
    {:else if runState?.initializing}
      <span class="status-badge init"><span class="dot"></span>{m.gameCard_initializing()}</span>
    {:else if runState?.error}
      <span class="status-badge error">{m.gameCard_error()}</span>
    {/if}
  </div>

  <div class="row-actions">
    <GameActionsMenu
      {item}
      {inSteam}
      label={m.gameCard_shortcutMenuLabel()}
      {onEdit}
      {onEditArtwork}
      {onExportToSteam}
      {onRemoveFromSteam}
    >
      {#snippet trigger()}{item.shortcutState === "done" ? "✓" : "🔗"}{/snippet}
    </GameActionsMenu>
    <button
      type="button"
      class="icon-btn"
      onclick={onEditArtwork}
      aria-label={m.library_selectArtworkTitle()}
      title={m.library_selectArtworkTitle()}
    >
      🖼
    </button>
    <button
      type="button"
      class="icon-btn"
      onclick={onEdit}
      aria-label={m.library_editGameTitle()}
      title={m.library_editGameTitle()}
    >
      ✎
    </button>
    <span class="divider"></span>
    <button
      type="button"
      class="icon-btn danger"
      onclick={() => (item.confirmingRemove = true)}
      aria-label={m.gameCard_removeAriaLabel()}
      title={m.gameCard_removeAriaLabel()}
    >
      ✕
    </button>

    {#if runState?.running}
      <button type="button" class="kill-sm" onclick={() => killGame(game.id)}>{m.gameCard_kill()}</button>
    {:else}
      <button
        type="button"
        class="primary start-sm"
        onclick={() => launchGame(game.id)}
        disabled={runState?.initializing}
      >
        {runState?.initializing ? m.gameCard_initializingShort() : m.gameCard_start()}
      </button>
    {/if}
  </div>
</div>

<GameItemNotices {game} {item} {runState} {inSteam} inline {onRemove} />

<style>
  .row {
    display: flex;
    align-items: center;
    gap: 1em;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 0.6em 0.8em;
    transition: border-color 0.15s ease;
  }

  .row:hover {
    border-color: var(--accent);
  }

  .row.running {
    border-color: var(--success);
    box-shadow: 0 0 0 1px var(--success);
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
  }

  .thumb :global(.icon) {
    width: 1.7em;
    height: 1.7em;
    object-fit: contain;
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
  }

  .runner {
    flex-shrink: 0;
    font-size: 0.75em;
    color: var(--text-muted);
    background: var(--surface-raised);
    padding: 0.15em 0.6em;
    border-radius: 999px;
  }

  .status {
    flex-shrink: 0;
    width: 6.5em;
  }

  .status-badge {
    display: inline-flex;
    align-items: center;
    gap: 0.4em;
    font-size: 0.72em;
    font-weight: 600;
    padding: 0.3em 0.65em;
    border-radius: 999px;
  }

  .status-badge.running {
    background: var(--success-bg);
    color: var(--success);
  }

  .status-badge.init {
    background: rgba(91, 140, 255, 0.18);
    color: var(--accent);
  }

  .status-badge.error {
    background: var(--danger-bg);
    color: var(--danger);
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

  .row-actions {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 0.4em;
  }







  .divider {
    flex-shrink: 0;
    width: 1px;
    height: 1.4em;
    background: var(--border);
    margin: 0 0.1em;
  }

  .start-sm,
  .kill-sm {
    font-size: 0.85em;
    padding: 0.5em 0.9em;
    white-space: nowrap;
  }

  .kill-sm {
    background: var(--danger-bg);
    border-color: transparent;
    color: var(--danger);
  }

  .kill-sm:hover:not(:disabled) {
    border-color: var(--danger);
  }

</style>
