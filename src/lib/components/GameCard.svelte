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
  let menuOpen = $state(false);
  let menuUpward = $state(false);
  let menuButtonEl: HTMLButtonElement | undefined;
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

  function toggleMenu() {
    if (!menuOpen && menuButtonEl) {
      const rect = menuButtonEl.getBoundingClientRect();
      menuUpward = window.innerHeight - rect.bottom < 330;
    }
    menuOpen = !menuOpen;
  }

  async function handleCreateShortcut(target: "desktop" | "menu") {
    menuOpen = false;
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

<article class="card" class:running={runState?.running}>
  <div class="cover-wrap">
    {#if coverDataUrl}
      <img class="cover" src={coverDataUrl} alt="" />
    {:else}
      <div class="cover placeholder">
        {#if game.icon}
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
    {/if}

    {#if runState?.running}
      <span class="status-badge running"><span class="dot"></span>Läuft</span>
    {:else if runState?.initializing}
      <span class="status-badge init"><span class="dot"></span>Startet…</span>
    {/if}

    <div class="menu-anchor">
      <button
        bind:this={menuButtonEl}
        type="button"
        class="icon-btn on-image menu-trigger"
        onclick={toggleMenu}
        aria-haspopup="true"
        aria-expanded={menuOpen}
        aria-label="Weitere Optionen"
        title="Weitere Optionen"
      >
        ⋮
      </button>
      {#if menuOpen}
        <div class="menu-backdrop" onclick={() => (menuOpen = false)} role="presentation"></div>
        <div class="menu" class:menu-up={menuUpward} role="menu">
          <button
            type="button"
            role="menuitem"
            onclick={() => {
              menuOpen = false;
              onEditArtwork();
            }}
          >
            <svg class="menu-icon" viewBox="0 0 20 20" fill="none" aria-hidden="true">
              <rect x="3" y="4" width="14" height="12" rx="2" stroke="currentColor" stroke-width="1.5" />
              <circle cx="7.5" cy="8.5" r="1.4" stroke="currentColor" stroke-width="1.5" />
              <path
                d="M4 14.5l4-4 3 3 2-2 3.5 3.5"
                stroke="currentColor"
                stroke-width="1.5"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
            </svg>
            Artwork auswählen
          </button>
          <button
            type="button"
            role="menuitem"
            onclick={() => {
              menuOpen = false;
              onEdit();
            }}
          >
            <svg class="menu-icon" viewBox="0 0 20 20" fill="none" aria-hidden="true">
              <path
                d="M13.4 3.6l3 3L6.3 16.7l-3.6.9.9-3.6L13.4 3.6z"
                stroke="currentColor"
                stroke-width="1.5"
                stroke-linejoin="round"
              />
            </svg>
            Spiel bearbeiten
          </button>
          <span class="menu-divider"></span>
          <button
            type="button"
            role="menuitem"
            disabled={shortcutState === "creating"}
            onclick={() => handleCreateShortcut("desktop")}
          >
            <svg class="menu-icon" viewBox="0 0 20 20" fill="none" aria-hidden="true">
              <path d="M8 12l4-4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
              <path
                d="M7.2 13.3L5.6 15a3 3 0 01-4.2-4.2l2.3-2.3a3 3 0 014.2 0"
                stroke="currentColor"
                stroke-width="1.5"
                stroke-linecap="round"
              />
              <path
                d="M12.8 6.7L14.4 5a3 3 0 014.2 4.2l-2.3 2.3a3 3 0 01-4.2 0"
                stroke="currentColor"
                stroke-width="1.5"
                stroke-linecap="round"
              />
            </svg>
            Auf Desktop
          </button>
          <button
            type="button"
            role="menuitem"
            disabled={shortcutState === "creating"}
            onclick={() => handleCreateShortcut("menu")}
          >
            <svg class="menu-icon" viewBox="0 0 20 20" fill="none" aria-hidden="true">
              <path d="M8 12l4-4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
              <path
                d="M7.2 13.3L5.6 15a3 3 0 01-4.2-4.2l2.3-2.3a3 3 0 014.2 0"
                stroke="currentColor"
                stroke-width="1.5"
                stroke-linecap="round"
              />
              <path
                d="M12.8 6.7L14.4 5a3 3 0 014.2 4.2l-2.3 2.3a3 3 0 01-4.2 0"
                stroke="currentColor"
                stroke-width="1.5"
                stroke-linecap="round"
              />
            </svg>
            Ins Startmenü
          </button>
          <button
            type="button"
            role="menuitem"
            onclick={() => {
              menuOpen = false;
              onExportToSteam();
            }}
          >
            <svg class="menu-icon" viewBox="0 0 20 20" fill="none" aria-hidden="true">
              <rect x="3" y="3" width="14" height="14" rx="3" stroke="currentColor" stroke-width="1.5" />
              <path d="M10 6.5v7M6.5 10h7" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
            </svg>
            {inSteam ? "In Steam aktualisieren" : "Zu Steam hinzufügen"}
          </button>
          {#if inSteam}
            <button
              type="button"
              role="menuitem"
              onclick={() => {
                menuOpen = false;
                onRemoveFromSteam();
              }}
            >
              <svg class="menu-icon" viewBox="0 0 20 20" fill="none" aria-hidden="true">
                <rect x="3" y="3" width="14" height="14" rx="3" stroke="currentColor" stroke-width="1.5" />
                <path d="M6.5 10h7" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
              </svg>
              Aus Steam entfernen
            </button>
          {/if}
          <span class="menu-divider"></span>
          <button
            type="button"
            role="menuitem"
            class="danger"
            onclick={() => {
              menuOpen = false;
              confirmingRemove = true;
            }}
          >
            <svg class="menu-icon" viewBox="0 0 20 20" fill="none" aria-hidden="true">
              <path
                d="M5 6h10M8 6V4.5a1 1 0 011-1h2a1 1 0 011 1V6M6.5 6l.6 8.6a1 1 0 001 .9h3.8a1 1 0 001-.9L13.5 6"
                stroke="currentColor"
                stroke-width="1.5"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
            </svg>
            Entfernen
          </button>
        </div>
      {/if}
    </div>

    <div class="overlay">
      <div class="bottom-row">
        <div class="info">
          <h3 title={game.name}>{game.name}</h3>
          <span class="runner">{game.runner_id}</span>
        </div>

        <button
          type="button"
          class="play-btn"
          class:running={runState?.running}
          onclick={runState?.running ? onKill : onLaunch}
          disabled={runState?.initializing}
          aria-label={runState?.running
            ? "Beenden (Prozess killen)"
            : runState?.initializing
              ? "Initialisiert…"
              : "Starten"}
          title={runState?.running
            ? "Beenden (Prozess killen)"
            : runState?.initializing
              ? "Initialisiert…"
              : "Starten"}
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
</article>

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

  .icon {
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
    backdrop-filter: blur(6px);
  }

  .status-badge.running {
    background: var(--success-bg);
    color: var(--success);
    border: 1px solid rgba(62, 207, 142, 0.4);
  }

  .status-badge.init {
    background: rgba(91, 140, 255, 0.18);
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

  .info {
    display: flex;
    flex-direction: column;
    gap: 0.3em;
    text-align: left;
  }

  h3 {
    font-size: 0.92em;
    color: #fff;
    text-shadow: 0 1px 4px rgba(0, 0, 0, 0.7);
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .runner {
    align-self: flex-start;
    font-size: 0.72em;
    color: #fff;
    background: rgba(255, 255, 255, 0.14);
    border: 1px solid rgba(255, 255, 255, 0.18);
    backdrop-filter: blur(6px);
    padding: 0.15em 0.6em;
    border-radius: 999px;
  }

  .menu-anchor {
    position: absolute;
    top: 0.6em;
    right: 0.6em;
    z-index: 5;
  }

  .menu-trigger {
    font-size: 1.1em;
    font-weight: 700;
  }

  /* Icon buttons need their own translucent + blurred background here
     since they float directly on cover art instead of a themed surface. */
  .icon-btn.on-image {
    background: rgba(15, 16, 20, 0.55);
    border-color: rgba(255, 255, 255, 0.18);
    color: #fff;
    backdrop-filter: blur(8px);
  }

  .icon-btn.on-image:hover:not(:disabled) {
    background: rgba(15, 16, 20, 0.75);
    border-color: var(--accent);
    color: var(--accent);
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
    min-width: 12em;
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
    display: flex;
    align-items: center;
    gap: 0.55em;
    text-align: left;
    background: transparent;
    border: none;
    padding: 0.5em 0.6em;
    border-radius: 6px;
    font-size: 0.85em;
    color: var(--text);
    white-space: nowrap;
  }

  .menu button:hover:not(:disabled) {
    background: var(--surface);
  }

  .menu-icon {
    width: 1.05em;
    height: 1.05em;
    flex-shrink: 0;
  }

  .menu button.danger {
    color: var(--danger);
  }

  .menu button.danger:hover:not(:disabled) {
    background: var(--danger-bg);
  }

  .menu-divider {
    height: 1px;
    background: var(--border);
    margin: 0.25em 0.4em;
  }

  .bottom-row {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 0.6em;
  }

  .bottom-row .info {
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
    background: rgba(20, 20, 24, 0.45);
    border: 1px solid rgba(255, 255, 255, 0.4);
    backdrop-filter: blur(8px);
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

  .toast {
    display: flex;
    flex-direction: column;
    gap: 0.4em;
    background: var(--danger-bg);
    color: var(--danger);
    border-radius: 8px;
    padding: 0.5em 0.7em;
    margin: 0.6em 0.9em 0.9em;
    font-size: 0.85em;
    text-align: left;
  }
</style>
