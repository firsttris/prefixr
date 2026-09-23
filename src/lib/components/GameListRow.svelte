<script lang="ts">
  import type { Game } from "$lib/types";
  import type { GameRunState } from "$lib/stores/games";
  import { getGameCover } from "$lib/stores/steamgriddb";
  import ConfirmDialog from "./ConfirmDialog.svelte";

  let {
    game,
    runState,
    inSteam,
    onLaunch,
    onKill,
    onEdit,
    onRemove,
    onShowLog,
    onCreateDesktopShortcut,
    onCreateMenuShortcut,
    onExportToSteam,
    onRemoveFromSteam,
    onEditArtwork,
  }: {
    game: Game;
    runState?: GameRunState;
    inSteam: boolean;
    onLaunch: () => void;
    onKill: () => void;
    onEdit: () => void;
    onRemove: () => void;
    onShowLog: (path: string) => void;
    onCreateDesktopShortcut: () => Promise<void>;
    onCreateMenuShortcut: () => Promise<void>;
    onExportToSteam: () => void;
    onRemoveFromSteam: () => void;
    onEditArtwork: () => void;
  } = $props();

  let shortcutState = $state<"idle" | "creating" | "done" | string>("idle");
  let shortcutMenuOpen = $state(false);
  let shortcutMenuUpward = $state(false);
  let shortcutButtonEl: HTMLButtonElement | undefined;
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

  function toggleShortcutMenu() {
    if (!shortcutMenuOpen && shortcutButtonEl) {
      const rect = shortcutButtonEl.getBoundingClientRect();
      shortcutMenuUpward = window.innerHeight - rect.bottom < 200;
    }
    shortcutMenuOpen = !shortcutMenuOpen;
  }

  async function handleCreateShortcut(target: "desktop" | "menu") {
    shortcutMenuOpen = false;
    shortcutState = "creating";
    try {
      await (target === "desktop" ? onCreateDesktopShortcut() : onCreateMenuShortcut());
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
      <svg class="icon fallback" viewBox="0 0 64 40" aria-hidden="true">
        <path
          fill="#64748b"
          d="M19 6h26a12 12 0 0 1 12 12v5a7 7 0 0 1-12.5 4.5L40 24H24l-4.5 3.5A7 7 0 0 1 7 23v-5A12 12 0 0 1 19 6z"
        />
        <ellipse cx="32" cy="11" rx="16" ry="4" fill="#fff" opacity="0.1" />
        <rect x="15.5" y="11" width="3" height="12" rx="1.5" fill="#e2e8f0" />
        <rect x="11" y="15.5" width="12" height="3" rx="1.5" fill="#e2e8f0" />
        <circle cx="47" cy="12" r="2.2" fill="#22c55e" />
        <circle cx="52" cy="17" r="2.2" fill="#ef4444" />
        <circle cx="47" cy="22" r="2.2" fill="#eab308" />
        <circle cx="42" cy="17" r="2.2" fill="#3b82f6" />
      </svg>
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
    <div class="shortcut-menu">
      <button
        bind:this={shortcutButtonEl}
        type="button"
        class="icon-btn"
        onclick={toggleShortcutMenu}
        disabled={shortcutState === "creating"}
        aria-haspopup="true"
        aria-expanded={shortcutMenuOpen}
        aria-label="Verknüpfung erstellen oder zu Steam hinzufügen"
        title="Verknüpfung erstellen oder zu Steam hinzufügen"
      >
        {shortcutState === "done" ? "✓" : "🔗"}
      </button>
      {#if shortcutMenuOpen}
        <div
          class="menu-backdrop"
          onclick={() => (shortcutMenuOpen = false)}
          role="presentation"
        ></div>
        <div class="menu" class:menu-up={shortcutMenuUpward} role="menu">
          <button type="button" role="menuitem" onclick={() => handleCreateShortcut("desktop")}>
            Auf Desktop
          </button>
          <button type="button" role="menuitem" onclick={() => handleCreateShortcut("menu")}>
            Ins Startmenü
          </button>
          <button
            type="button"
            role="menuitem"
            onclick={() => {
              shortcutMenuOpen = false;
              onExportToSteam();
            }}
          >
            {inSteam ? "In Steam aktualisieren" : "Zu Steam hinzufügen"}
          </button>
          {#if inSteam}
            <button
              type="button"
              role="menuitem"
              onclick={() => {
                shortcutMenuOpen = false;
                onRemoveFromSteam();
              }}
            >
              Aus Steam entfernen
            </button>
          {/if}
        </div>
      {/if}
    </div>
    <button
      type="button"
      class="icon-btn"
      onclick={onEditArtwork}
      aria-label="Artwork auswählen"
      title="Artwork auswählen"
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
  message={`„${game.name}“ wird aus der Bibliothek${inSteam ? " und aus Steam" : ""} entfernt. Der Prefix und die Spieldateien bleiben erhalten.`}
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

  .shortcut-menu {
    position: relative;
    display: flex;
  }

  .menu-backdrop {
    position: fixed;
    inset: 0;
    z-index: 10;
  }

  .menu {
    position: absolute;
    top: calc(100% + 0.3em);
    right: 0;
    z-index: 11;
    display: flex;
    flex-direction: column;
    min-width: 9.5em;
    background: var(--surface-raised);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 0.3em;
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.4);
  }

  .menu.menu-up {
    top: auto;
    bottom: calc(100% + 0.3em);
  }

  .menu button {
    text-align: left;
    background: transparent;
    border: none;
    padding: 0.5em 0.6em;
    border-radius: 6px;
    font-size: 0.85em;
    color: var(--text);
  }

  .menu button:hover {
    background: var(--surface);
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
