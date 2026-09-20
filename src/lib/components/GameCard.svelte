<script lang="ts">
  import type { Game } from "$lib/types";
  import type { GameRunState } from "$lib/stores/games";

  let {
    game,
    runState,
    onLaunch,
    onEdit,
    onRemove,
    onShowLog,
  }: {
    game: Game;
    runState?: GameRunState;
    onLaunch: () => void;
    onEdit: () => void;
    onRemove: () => void;
    onShowLog: (path: string) => void;
  } = $props();
</script>

<article class="card">
  <div class="corner-actions">
    <button type="button" class="ghost" onclick={onEdit} aria-label="Spiel bearbeiten">✎</button>
    <button type="button" class="ghost" onclick={onRemove} aria-label="Spiel entfernen">✕</button>
  </div>
  <div class="icon">🎮</div>
  <h3>{game.name}</h3>
  <span class="runner">{game.runner_id}</span>

  <button
    type="button"
    class="primary start"
    onclick={onLaunch}
    disabled={runState?.running || runState?.initializing}
  >
    {#if runState?.initializing}
      Initialisiere…
    {:else if runState?.running}
      Läuft…
    {:else}
      Start
    {/if}
  </button>

  {#if runState?.error}
    <div class="toast">
      <span>{runState.error}</span>
      {#if runState.logPath}
        <button type="button" class="ghost" onclick={() => onShowLog(runState.logPath!)}>
          Log anzeigen
        </button>
      {/if}
    </div>
  {/if}
</article>

<style>
  .card {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.5em;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 1.4em 1em 1em;
    text-align: center;
    transition: border-color 0.15s;
  }

  .card:hover {
    border-color: var(--accent);
  }

  .corner-actions {
    position: absolute;
    top: 0.5em;
    right: 0.5em;
    display: flex;
    gap: 0.3em;
    opacity: 0;
    transition: opacity 0.15s;
  }

  .corner-actions button {
    padding: 0.2em 0.5em;
    font-size: 0.75em;
  }

  .card:hover .corner-actions {
    opacity: 1;
  }

  .icon {
    font-size: 2.4em;
  }

  h3 {
    font-size: 1em;
    word-break: break-word;
  }

  .runner {
    font-size: 0.8em;
    color: var(--text-muted);
    background: var(--surface-raised);
    padding: 0.15em 0.6em;
    border-radius: 999px;
  }

  .start {
    width: 100%;
    margin-top: 0.4em;
  }

  .toast {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 0.4em;
    background: var(--danger-bg);
    color: var(--danger);
    border-radius: 8px;
    padding: 0.5em 0.7em;
    font-size: 0.85em;
    text-align: left;
  }
</style>
