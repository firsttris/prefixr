<script lang="ts">
  import type { Game } from "$lib/types";
  import type { GameRunState } from "$lib/stores/games";
  import { getGameCover } from "$lib/stores/steamgriddb";
  import ConfirmDialog from "./ConfirmDialog.svelte";

  let {
    game,
    runState,
    onLaunch,
    onKill,
    onEdit,
    onRemove,
    onShowLog,
    onCreateShortcut,
    onEditCover,
  }: {
    game: Game;
    runState?: GameRunState;
    onLaunch: () => void;
    onKill: () => void;
    onEdit: () => void;
    onRemove: () => void;
    onShowLog: (path: string) => void;
    onCreateShortcut: () => Promise<void>;
    onEditCover: () => void;
  } = $props();

  let shortcutState = $state<"idle" | "creating" | "done" | string>("idle");
  let confirmingRemove = $state(false);

  let coverDataUrl = $state<string | null>(null);

  $effect(() => {
    if (!game.cover_url) {
      coverDataUrl = null;
      return;
    }
    getGameCover(game.id)
      .then((url) => (coverDataUrl = url))
      .catch(() => (coverDataUrl = null));
  });

  async function handleCreateShortcut() {
    shortcutState = "creating";
    try {
      await onCreateShortcut();
      shortcutState = "done";
      setTimeout(() => {
        shortcutState = "idle";
      }, 2000);
    } catch (e) {
      shortcutState = String(e);
    }
  }
</script>

<div class="row" class:running={runState?.running}>
  <div class="thumb">
    {#if coverDataUrl}
      <img class="cover" src={coverDataUrl} alt="" />
    {:else if game.icon}
      <img class="icon" src={game.icon} alt="" />
    {:else}
      <span class="icon fallback">🎮</span>
    {/if}
  </div>

  <div class="meta">
    <span class="name" title={game.name}>{game.name}</span>
    <span class="runner">{game.runner_id}</span>
  </div>

  <div class="status">
    {#if runState?.running}
      <span class="status-badge running"><span class="dot"></span>Läuft</span>
    {:else if runState?.initializing}
      <span class="status-badge init"><span class="dot"></span>Startet…</span>
    {:else if runState?.error}
      <span class="status-badge error">Fehler</span>
    {/if}
  </div>

  <div class="row-actions">
    <button
      type="button"
      class="icon-btn"
      onclick={handleCreateShortcut}
      disabled={shortcutState === "creating"}
      aria-label="Desktop-Verknüpfung erstellen"
      title="Desktop-Verknüpfung erstellen"
    >
      {shortcutState === "done" ? "✓" : "🔗"}
    </button>
    <button
      type="button"
      class="icon-btn"
      onclick={onEditCover}
      aria-label="Cover auswählen"
      title="Cover auswählen"
    >
      🖼
    </button>
    <button
      type="button"
      class="icon-btn"
      onclick={onEdit}
      aria-label="Spiel bearbeiten"
      title="Spiel bearbeiten"
    >
      ✎
    </button>
    <span class="divider"></span>
    <button
      type="button"
      class="icon-btn danger"
      onclick={() => (confirmingRemove = true)}
      aria-label="Spiel entfernen"
      title="Spiel entfernen"
    >
      ✕
    </button>

    {#if runState?.running}
      <button type="button" class="kill-sm" onclick={onKill}>Beenden</button>
    {:else}
      <button
        type="button"
        class="primary start-sm"
        onclick={onLaunch}
        disabled={runState?.initializing}
      >
        {runState?.initializing ? "Initialisiere…" : "▶ Start"}
      </button>
    {/if}
  </div>
</div>

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

{#if shortcutState !== "idle" && shortcutState !== "creating" && shortcutState !== "done"}
  <div class="toast">
    <span>Verknüpfung fehlgeschlagen: {shortcutState}</span>
  </div>
{/if}

<ConfirmDialog
  open={confirmingRemove}
  title="Spiel entfernen?"
  message={`„${game.name}“ wird aus der Bibliothek entfernt. Der Prefix und die Spieldateien bleiben erhalten.`}
  onConfirm={() => {
    confirmingRemove = false;
    onRemove();
  }}
  onCancel={() => (confirmingRemove = false)}
/>

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

  .thumb .icon {
    width: 1.7em;
    height: 1.7em;
    object-fit: contain;
  }

  .thumb .icon.fallback {
    font-size: 1.4em;
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

  .toast {
    display: flex;
    align-items: center;
    gap: 0.6em;
    background: var(--danger-bg);
    color: var(--danger);
    border-radius: 8px;
    padding: 0.5em 0.8em;
    margin-top: 0.4em;
    font-size: 0.85em;
  }
</style>
