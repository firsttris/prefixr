<script lang="ts">
  import { onMount } from "svelte";
  import { openPath, revealItemInDir } from "@tauri-apps/plugin-opener";
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
  import GameCard from "./GameCard.svelte";
  import type { Game } from "$lib/types";

  let { onEdit }: { onEdit: (game: Game) => void } = $props();

  onMount(() => {
    initGameEvents();
    refreshGames();
  });

  async function showLog(logPath: string) {
    try {
      await openPath(logPath);
    } catch {
      // No default app for the log file on this system — fall back to just
      // showing it in the file manager instead of failing silently.
      try {
        await revealItemInDir(logPath);
      } catch (e) {
        console.error("Could not open or reveal log file", e);
      }
    }
  }
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
