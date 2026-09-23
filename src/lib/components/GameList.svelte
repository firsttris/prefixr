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
    createMenuShortcut,
    exportToSteam,
    removeFromSteam,
    steamGameIds,
    refreshSteamGames,
  } from "$lib/stores/games";
  import { showLog } from "$lib/logViewer";
  import GameCard from "./GameCard.svelte";
  import GameListRow from "./GameListRow.svelte";
  import ConfirmDialog from "./ConfirmDialog.svelte";
  import type { Game } from "$lib/types";

  let {
    onEdit,
    onEditArtwork,
    onAddNew,
  }: {
    onEdit: (game: Game) => void;
    onEditArtwork: (game: Game) => void;
    onAddNew?: () => void;
  } = $props();

  const VIEW_MODE_KEY = "library-view-mode";

  function loadViewMode(): "grid" | "list" {
    try {
      const stored = localStorage.getItem(VIEW_MODE_KEY);
      if (stored === "grid" || stored === "list") return stored;
    } catch {
      // localStorage unavailable (e.g. private mode) — fall back silently.
    }
    return "grid";
  }

  let viewMode = $state<"grid" | "list">(loadViewMode());
  let query = $state("");
  let sortBy = $state<"name-asc" | "name-desc" | "runner">("name-asc");

  $effect(() => {
    try {
      localStorage.setItem(VIEW_MODE_KEY, viewMode);
    } catch {
      // Ignore — view mode just won't persist across restarts.
    }
  });

  let visibleGames = $derived.by(() => {
    const q = query.trim().toLowerCase();
    let list = q ? $games.filter((g) => g.name.toLowerCase().includes(q)) : $games.slice();

    if (sortBy === "name-asc") {
      list.sort((a, b) => a.name.localeCompare(b.name));
    } else if (sortBy === "name-desc") {
      list.sort((a, b) => b.name.localeCompare(a.name));
    } else {
      list.sort(
        (a, b) => a.runner_id.localeCompare(b.runner_id) || a.name.localeCompare(b.name),
      );
    }
    return list;
  });

  onMount(() => {
    initGameEvents();
    refreshGames();
    refreshSteamGames().catch(() => {});
  });

  // Steam changes run from here for both views, since a running Steam first
  // needs the user's OK to be quit (see steam.rs). "remove-game" takes the
  // game out of Steam before removing it from the library.
  type SteamAction = "export" | "remove" | "remove-game";

  let steamConfirm = $state<{ game: Game; action: SteamAction } | null>(null);
  let steamNotice = $state<{ text: string; kind: "busy" | "done" | "error" } | null>(null);
  let noticeTimer: ReturnType<typeof setTimeout> | undefined;

  function notify(text: string, kind: "busy" | "done" | "error") {
    clearTimeout(noticeTimer);
    steamNotice = { text, kind };
    if (kind !== "busy") {
      noticeTimer = setTimeout(() => (steamNotice = null), kind === "error" ? 10000 : 5000);
    }
  }

  async function runSteamAction(game: Game, action: SteamAction, shutdownSteam = false) {
    steamConfirm = null;
    notify(
      shutdownSteam
        ? "Steam wird beendet…"
        : action === "export"
          ? `„${game.name}“ wird zu Steam hinzugefügt…`
          : `„${game.name}“ wird aus Steam entfernt…`,
      "busy",
    );
    try {
      const result =
        action === "export"
          ? await exportToSteam(game.id, shutdownSteam)
          : await removeFromSteam(game.id, shutdownSteam);
      if (result.status === "steam_running") {
        steamNotice = null;
        steamConfirm = { game, action };
        return;
      }
      await refreshSteamGames();
      const restarted = result.restarted_steam ? " Steam startet neu." : "";
      if (action === "remove-game") {
        await removeGame(game.id);
        notify(`„${game.name}“ ist aus der Bibliothek und aus Steam entfernt.${restarted}`, "done");
      } else if (action === "remove") {
        notify(`„${game.name}“ ist aus Steam entfernt.${restarted}`, "done");
      } else {
        notify(
          `„${game.name}“ ist in Steam.${restarted || " Es erscheint beim nächsten Start von Steam."}`,
          "done",
        );
      }
    } catch (e) {
      notify(`Steam: ${e}`, "error");
    }
  }

  function handleRemove(game: Game) {
    if ($steamGameIds.has(game.id)) {
      runSteamAction(game, "remove-game");
    } else {
      removeGame(game.id).catch((e) =>
        notify(`„${game.name}“ konnte nicht entfernt werden: ${e}`, "error"),
      );
    }
  }
</script>

{#if $games.length > 0}
  <div class="toolbar">
    <input
      type="search"
      placeholder="Spiel suchen…"
      bind:value={query}
      aria-label="Spiel suchen"
    />

    <select bind:value={sortBy} aria-label="Sortierung">
      <option value="name-asc">Name (A–Z)</option>
      <option value="name-desc">Name (Z–A)</option>
      <option value="runner">Runner</option>
    </select>

    <div class="view-toggle" role="group" aria-label="Ansicht wechseln">
      <button
        type="button"
        class:active={viewMode === "grid"}
        onclick={() => (viewMode = "grid")}
        aria-label="Kachelansicht"
        title="Kachelansicht"
      >
        ▦
      </button>
      <button
        type="button"
        class:active={viewMode === "list"}
        onclick={() => (viewMode = "list")}
        aria-label="Listenansicht"
        title="Listenansicht"
      >
        ☰
      </button>
    </div>
  </div>
{/if}

{#if $games.length === 0}
  <div class="empty">
    <span class="empty-icon">🎮</span>
    <h3>Noch keine Spiele in deiner Bibliothek</h3>
    <p>Füge dein erstes Spiel hinzu, um loszulegen.</p>
    {#if onAddNew}
      <button type="button" class="primary" onclick={onAddNew}>+ Spiel hinzufügen</button>
    {/if}
  </div>
{:else if visibleGames.length === 0}
  <div class="empty">
    <span class="empty-icon">🔍</span>
    <h3>Keine Treffer</h3>
    <p>Kein Spiel gefunden für „{query}“.</p>
  </div>
{:else if viewMode === "grid"}
  <div class="grid">
    {#each visibleGames as game (game.id)}
      <GameCard
        {game}
        runState={$gameRunState[game.id]}
        inSteam={$steamGameIds.has(game.id)}
        onLaunch={() => launchGame(game.id)}
        onKill={() => killGame(game.id)}
        onEdit={() => onEdit(game)}
        onRemove={() => handleRemove(game)}
        onShowLog={showLog}
        onCreateDesktopShortcut={() => createDesktopShortcut(game.id)}
        onCreateMenuShortcut={() => createMenuShortcut(game.id)}
        onExportToSteam={() => runSteamAction(game, "export")}
        onRemoveFromSteam={() => runSteamAction(game, "remove")}
        onEditArtwork={() => onEditArtwork(game)}
      />
    {/each}
  </div>
{:else}
  <div class="list">
    {#each visibleGames as game (game.id)}
      <GameListRow
        {game}
        runState={$gameRunState[game.id]}
        inSteam={$steamGameIds.has(game.id)}
        onLaunch={() => launchGame(game.id)}
        onKill={() => killGame(game.id)}
        onEdit={() => onEdit(game)}
        onRemove={() => handleRemove(game)}
        onShowLog={showLog}
        onCreateDesktopShortcut={() => createDesktopShortcut(game.id)}
        onCreateMenuShortcut={() => createMenuShortcut(game.id)}
        onExportToSteam={() => runSteamAction(game, "export")}
        onRemoveFromSteam={() => runSteamAction(game, "remove")}
        onEditArtwork={() => onEditArtwork(game)}
      />
    {/each}
  </div>
{/if}

{#if steamNotice}
  <div class="notice {steamNotice.kind}" role="status">{steamNotice.text}</div>
{/if}

<ConfirmDialog
  open={steamConfirm !== null}
  title="Steam neu starten?"
  message={steamConfirm
    ? `Steam läuft gerade und würde die Änderung wieder überschreiben. Prefixr beendet Steam, ${
        steamConfirm.action === "export"
          ? `trägt „${steamConfirm.game.name}“ ein`
          : `entfernt „${steamConfirm.game.name}“`
      } und startet Steam danach neu. Ein laufendes Steam-Spiel wird dabei beendet.`
    : ""}
  confirmLabel="Steam neu starten"
  onConfirm={() => steamConfirm && runSteamAction(steamConfirm.game, steamConfirm.action, true)}
  onCancel={() => (steamConfirm = null)}
/>

<style>
  .notice {
    position: fixed;
    bottom: 1.2em;
    left: 50%;
    transform: translateX(-50%);
    z-index: 20;
    max-width: min(36em, calc(100vw - 2em));
    padding: 0.6em 1em;
    border-radius: 8px;
    font-size: 0.85em;
    background: var(--surface-raised);
    color: var(--text);
    border: 1px solid var(--border);
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.4);
  }

  .notice.done {
    background: var(--success-bg);
    color: var(--success);
    border-color: transparent;
  }

  .notice.error {
    background: var(--danger-bg);
    color: var(--danger);
    border-color: transparent;
  }

  .toolbar {
    display: flex;
    align-items: center;
    gap: 0.6em;
    margin-bottom: 1.3em;
  }

  .toolbar input[type="search"] {
    flex: 1;
    max-width: 320px;
  }

  .toolbar select {
    width: auto;
  }

  .view-toggle {
    display: flex;
    border: 1px solid var(--border);
    border-radius: 8px;
    overflow: hidden;
    flex-shrink: 0;
  }

  .view-toggle button {
    border: none;
    border-radius: 0;
    background: var(--surface-raised);
    padding: 0.5em 0.8em;
    color: var(--text-muted);
  }

  .view-toggle button + button {
    border-left: 1px solid var(--border);
  }

  .view-toggle button.active {
    background: var(--accent);
    color: #fff;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(170px, 1fr));
    gap: 1.1em;
  }

  .list {
    display: flex;
    flex-direction: column;
    gap: 0.5em;
  }

  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.5em;
    color: var(--text-muted);
    text-align: center;
    padding: 4em 1em;
    border: 1px dashed var(--border);
    border-radius: var(--radius);
  }

  .empty-icon {
    font-size: 2.4em;
    margin-bottom: 0.2em;
  }

  .empty h3 {
    color: var(--text);
  }

  .empty .primary {
    margin-top: 0.8em;
  }
</style>
