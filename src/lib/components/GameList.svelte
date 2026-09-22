<script lang="ts">
  import { onMount } from "svelte";
  import {
    games,
    gameRunState,
    refreshGames,
    removeGame,
    launchGame,
    killGame,
    initGameEvents,
    createDesktopShortcut,
  } from "$lib/stores/games";
  import { showLog } from "$lib/logViewer";
  import GameCard from "./GameCard.svelte";
  import type { Game } from "$lib/types";

  let { onEdit }: { onEdit: (game: Game) => void } = $props();

  onMount(() => {
    initGameEvents();
    refreshGames();
  });
</script>

{#if $games.length === 0}
  <div class="empty">
    <p>Noch keine Spiele in deiner Bibliothek.</p>
  </div>
{:else}
  <div class="grid">
    {#each $games as game (game.id)}
      <GameCard
        {game}
        runState={$gameRunState[game.id]}
        onLaunch={() => launchGame(game.id)}
        onKill={() => killGame(game.id)}
        onEdit={() => onEdit(game)}
        onRemove={() => removeGame(game.id)}
        onShowLog={showLog}
        onCreateShortcut={() => createDesktopShortcut(game.id)}
      />
    {/each}
  </div>
{/if}

<style>
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
    gap: 1em;
  }

  .empty {
    color: var(--text-muted);
    text-align: center;
    padding: 3em 1em;
    border: 1px dashed var(--border);
    border-radius: var(--radius);
  }
</style>
