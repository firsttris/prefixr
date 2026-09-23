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
  import { t } from "$lib/i18n/index.svelte";
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
        ? t("gameList.steam.busyQuitting")
        : action === "export"
          ? t("gameList.steam.busyExporting", { name: game.name })
          : t("gameList.steam.busyRemoving", { name: game.name }),
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
      const restarted = result.restarted_steam ? t("gameList.steam.restartedSuffix") : "";
      if (action === "remove-game") {
        await removeGame(game.id);
        notify(t("gameList.steam.doneRemovedBoth", { name: game.name, restarted }), "done");
      } else if (action === "remove") {
        notify(t("gameList.steam.doneRemoved", { name: game.name, restarted }), "done");
      } else {
        notify(
          t("gameList.steam.doneAdded", {
            name: game.name,
            restarted: restarted || t("gameList.steam.doneAddedNextStartSuffix"),
          }),
          "done",
        );
      }
    } catch (e) {
      notify(t("gameList.steam.errorPrefix", { error: String(e) }), "error");
    }
  }

  function handleRemove(game: Game) {
    if ($steamGameIds.has(game.id)) {
      runSteamAction(game, "remove-game");
    } else {
      removeGame(game.id).catch((e) =>
        notify(t("gameList.steam.removeFailed", { name: game.name, error: String(e) }), "error"),
      );
    }
  }
</script>

{#if $games.length > 0}
  <div class="toolbar">
    <input
      type="search"
      placeholder={t("gameList.toolbar.searchPlaceholder")}
      bind:value={query}
      aria-label={t("gameList.toolbar.searchLabel")}
    />

    <select bind:value={sortBy} aria-label={t("gameList.toolbar.sortLabel")}>
      <option value="name-asc">{t("gameList.toolbar.sortNameAsc")}</option>
      <option value="name-desc">{t("gameList.toolbar.sortNameDesc")}</option>
      <option value="runner">{t("gameList.toolbar.sortRunner")}</option>
    </select>

    <div class="view-toggle" role="group" aria-label={t("gameList.toolbar.viewToggleLabel")}>
      <button
        type="button"
        class:active={viewMode === "grid"}
        onclick={() => (viewMode = "grid")}
        aria-label={t("gameList.toolbar.gridView")}
        title={t("gameList.toolbar.gridView")}
      >
        ▦
      </button>
      <button
        type="button"
        class:active={viewMode === "list"}
        onclick={() => (viewMode = "list")}
        aria-label={t("gameList.toolbar.listView")}
        title={t("gameList.toolbar.listView")}
      >
        ☰
      </button>
    </div>
  </div>
{/if}

{#if $games.length === 0}
  <div class="empty">
    <span class="empty-icon">🎮</span>
    <h3>{t("gameList.emptyLibrary.title")}</h3>
    <p>{t("gameList.emptyLibrary.hint")}</p>
    {#if onAddNew}
      <button type="button" class="primary" onclick={onAddNew}>{t("library.addGame")}</button>
    {/if}
  </div>
{:else if visibleGames.length === 0}
  <div class="empty">
    <span class="empty-icon">🔍</span>
    <h3>{t("gameList.emptySearch.title")}</h3>
    <p>{t("gameList.emptySearch.hint", { query })}</p>
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
  title={t("gameList.steam.confirmTitle")}
  message={steamConfirm
    ? t("gameList.steam.confirmMessage", {
        action:
          steamConfirm.action === "export"
            ? t("gameList.steam.confirmActionExport", { name: steamConfirm.game.name })
            : t("gameList.steam.confirmActionRemove", { name: steamConfirm.game.name }),
      })
    : ""}
  confirmLabel={t("gameList.steam.confirmButton")}
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
