<script lang="ts">
  import * as m from "#lib/paraglide/messages.js";
  import { onDestroy, onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import {
    games,
    gameRunState,
    refreshGames,
    removeGame,
    initGameEvents,
    exportToSteam,
    removeFromSteam,
    steamGameIds,
    refreshSteamGames,
  } from "#lib/stores/games.ts";
  import {
    storeGames,
    storeInstalls,
    storeInstallErrors,
    storeName,
    refreshStoreGames,
    initStoreEvents,
    cancelStoreInstall,
    dismissStoreInstallError,
    uninstallStoreGame,
  } from "#lib/stores/gameStores.ts";
  import {
    compatibility,
    libraryMeta,
    libraryMetaLoaded,
    refreshCompatibility,
    refreshLibraryMeta,
    setFavorite,
    TEST_STATUSES,
    testOf,
  } from "#lib/stores/libraryMeta.ts";
  import {
    collectionCounts,
    entriesOf,
    libraryView,
    recentlyPlayed,
    sourceCounts,
    type Collection,
    type Entry,
    type SortBy,
    type Source,
  } from "#lib/libraryView.ts";
  import { isNumberIn, isOneOf, isStringList, loadSetting, saveSetting } from "#lib/persisted.ts";
  import { backendError } from "#lib/i18n/index.svelte.ts";
  import GameCard from "./GameCard.svelte";
  import GameListRow from "./GameListRow.svelte";
  import StoreGameCard from "./StoreGameCard.svelte";
  import StoreGameRow from "./StoreGameRow.svelte";
  import ConfirmDialog from "./ConfirmDialog.svelte";
  import NotesDialog from "./NotesDialog.svelte";
  import LibraryChips from "./LibraryChips.svelte";
  import GameDetails from "./GameDetails.svelte";
  import type { Game, StoreGame, StoreKind } from "#lib/types.ts";
  import type { GameFormTab } from "./GameForm.svelte";

  let {
    onEdit,
    onEditArtwork,
    onAddNew,
    onInstallStoreGame,
    onShowPrefix,
  }: {
    onEdit: (game: Game, tab?: GameFormTab) => void;
    onEditArtwork: (game: Game) => void;
    onAddNew?: () => void;
    onInstallStoreGame: (game: StoreGame) => void;
    onShowPrefix?: (path: string) => void;
  } = $props();

  let showingNotes = $state<Game | null>(null);
  // The key of the game whose details are open.
  let detailsKey = $state<string | null>(null);

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

  const SORTS = [
    "name-asc",
    "name-desc",
    "last-played",
    "recently-added",
    "playtime",
    "runner",
  ] as const satisfies readonly SortBy[];
  const isSource = (value: unknown): value is Source =>
    isOneOf("all", "local", "epic", "gog", "amazon")(value);
  const isCollection = (value: unknown): value is Collection =>
    isOneOf("all", "installed", "favorites", "hidden")(value) ||
    (typeof value === "string" &&
      (value.startsWith("category:") ||
        TEST_STATUSES.some((status) => value === `tested:${status}`)));

  let viewMode = $state<"grid" | "list">(loadViewMode());
  let query = $state("");
  let sortBy = $state<SortBy>(loadSetting("library-sort", "name-asc", isOneOf(...SORTS)));
  // Where games come from: added by hand ("local") or from a store.
  let source = $state<Source>(loadSetting("library-source", "all", isSource));
  let collection = $state<Collection>(loadSetting("library-collection", "all", isCollection));
  let cardSize = $state(loadSetting("library-card-size", 170, isNumberIn(130, 260)));
  let collapsed = $state<string[]>(loadSetting("library-collapsed", [], isStringList));

  $effect(() => saveSetting("library-sort", sortBy));
  $effect(() => saveSetting("library-source", source));
  $effect(() => saveSetting("library-collection", collection));
  $effect(() => saveSetting("library-card-size", cardSize));
  $effect(() => saveSetting("library-collapsed", collapsed));

  // A category that was deleted (here or from the command line) shows all;
  // not before the categories have loaded, which would drop a kept choice.
  $effect(() => {
    if (
      $libraryMetaLoaded &&
      collection.startsWith("category:") &&
      !$libraryMeta.categories.some((c) => `category:${c.id}` === collection)
    ) {
      collection = "all";
    }
  });

  let entries = $derived(entriesOf($games, $storeGames));
  let storesInLibrary = $derived([
    ...new Set($storeGames.map((game) => game.store)),
  ] as StoreKind[]);
  let hasAnything = $derived(entries.length > 0);
  // A kept store choice stays offered while that store has no games (logged
  // out, not loaded yet), so the filter can always be switched back.
  let sourceOptions = $derived([
    "all",
    "local",
    ...new Set([...storesInLibrary, ...(source === "all" || source === "local" ? [] : [source])]),
  ] as Source[]);
  let shown = $derived(libraryView(entries, { query, sortBy, source, collection }, $libraryMeta));
  let counts = $derived(collectionCounts(entries, $libraryMeta, source));
  let perSource = $derived(sourceCounts(entries, $libraryMeta));
  let continuePlaying = $derived(
    !query.trim() && collection === "all" && source === "all"
      ? recentlyPlayed(entries, $libraryMeta, 8)
      : [],
  );
  let detailsEntry = $derived(entries.find((entry) => entry.key === detailsKey) ?? null);

  $effect(() => {
    try {
      localStorage.setItem(VIEW_MODE_KEY, viewMode);
    } catch {
      // Ignore — view mode just won't persist across restarts.
    }
  });

  $effect(() => {
    refreshCompatibility(entries.map((entry) => entry.key));
  });

  function toggleSection(id: string) {
    collapsed = collapsed.includes(id) ? collapsed.filter((c) => c !== id) : [...collapsed, id];
  }

  let searchInput = $state<HTMLInputElement>();

  // "/" jumps to the search, as on many sites; not while typing elsewhere.
  function handleKeydown(event: KeyboardEvent) {
    const target = event.target as HTMLElement | null;
    const typing =
      target?.closest("input, textarea, select, [contenteditable='true'], [role='dialog']") != null;
    if (event.key === "/" && !typing && !event.ctrlKey && !event.metaKey && !event.altKey) {
      event.preventDefault();
      searchInput?.focus();
    }
  }

  // A library that failed to load must not look like an empty one.
  let loadError = $state<unknown>(null);

  // In the Flatpak, the native build's library, for the empty library's hint
  // on moving it over (see `native_library`).
  let nativeLibrary = $state<string | null>(null);

  onMount(() => {
    invoke<string | null>("native_library")
      .then((path) => (nativeLibrary = path))
      .catch(() => {});
    initGameEvents();
    refreshGames().catch((e) => (loadError = e));
    // Only marks which games are in Steam; without it, none are.
    refreshSteamGames().catch(() => {});
    // A store that can't be read leaves just the library's own games.
    refreshStoreGames().catch(() => {});
    initStoreEvents();
    refreshLibraryMeta().catch(() => {});
    // The play time of a game that just ended is recorded by then.
    const unlisten = listen("game-exited", () => refreshLibraryMeta().catch(() => {}));
    return () => {
      unlisten.then((stop) => stop());
    };
  });

  // Steam changes run from here for both views, since a running Steam first
  // needs the user's OK to be quit (see steam.rs). "remove-game" takes the
  // game out of Steam before removing it from the library.
  type SteamAction = "export" | "remove" | "remove-game";

  // `closeFirst`: in the Flatpak, where Prefixr can't see or quit Steam, the
  // user closes it instead of Prefixr restarting it.
  let steamConfirm = $state<{ game: Game; action: SteamAction; closeFirst: boolean } | null>(null);
  let steamNotice = $state<{ text: string; kind: "busy" | "done" | "error" } | null>(null);
  let noticeTimer: ReturnType<typeof setTimeout> | undefined;
  onDestroy(() => clearTimeout(noticeTimer));

  function notify(text: string, kind: "busy" | "done" | "error") {
    clearTimeout(noticeTimer);
    steamNotice = { text, kind };
    if (kind !== "busy") {
      noticeTimer = setTimeout(() => (steamNotice = null), kind === "error" ? 10000 : 5000);
    }
  }

  // `steamClosed`: the user confirmed in the Flatpak that Steam is closed, so
  // the backend writes the change without quitting Steam (`close_steam_first`).
  async function runSteamAction(
    game: Game,
    action: SteamAction,
    shutdownSteam = false,
    steamClosed = false,
  ) {
    steamConfirm = null;
    notify(
      shutdownSteam && !steamClosed
        ? m.gameList_steam_busyQuitting()
        : action === "export"
          ? m.gameList_steam_busyExporting({ name: game.name })
          : m.gameList_steam_busyRemoving({ name: game.name }),
      "busy",
    );
    try {
      const result =
        action === "export"
          ? await exportToSteam(game.id, shutdownSteam)
          : await removeFromSteam(game.id, shutdownSteam);
      if (result.status === "steam_running" || result.status === "close_steam_first") {
        steamNotice = null;
        steamConfirm = { game, action, closeFirst: result.status === "close_steam_first" };
        return;
      }
      await refreshSteamGames();
      const restarted = result.restarted_steam ? m.gameList_steam_restartedSuffix() : "";
      if (action === "remove-game") {
        await removeOrUninstall(game);
        notify(m.gameList_steam_doneRemovedBoth({ name: game.name, restarted }), "done");
      } else if (action === "remove") {
        notify(m.gameList_steam_doneRemoved({ name: game.name, restarted }), "done");
      } else {
        notify(
          m.gameList_steam_doneAdded({
            name: game.name,
            restarted: restarted || m.gameList_steam_doneAddedNextStartSuffix(),
          }),
          "done",
        );
      }
    } catch (e) {
      notify(m.gameList_steam_errorPrefix({ error: backendError(e) }), "error");
    }
  }

  // A store game's files go with it: they belong to the store's install.
  function removeOrUninstall(game: Game): Promise<void> {
    return game.store ? uninstallStoreGame(game.id) : removeGame(game.id);
  }

  function handleRemove(game: Game) {
    if ($steamGameIds.has(game.id)) {
      runSteamAction(game, "remove-game");
    } else {
      removeOrUninstall(game).catch((e) =>
        notify(m.gameList_steam_removeFailed({ name: game.name, error: backendError(e) }), "error"),
      );
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#snippet entryView(entry: Entry, asRow: boolean)}
  {@const favorite = $libraryMeta.favorites.includes(entry.key)}
  {@const compat = $compatibility[entry.key]}
  {#if entry.kind === "game"}
    {@const game = entry.game}
    {@const props = {
      game,
      runState: $gameRunState[game.id],
      inSteam: $steamGameIds.has(game.id),
      onEdit: () => onEdit(game),
      onEditArtwork: () => onEditArtwork(game),
      onRemove: () => handleRemove(game),
      onExportToSteam: () => runSteamAction(game, "export"),
      onRemoveFromSteam: () => runSteamAction(game, "remove"),
      onShowNotes: () => (showingNotes = game),
      favorite,
      compat,
      onToggleFavorite: () => setFavorite(entry.key, !favorite),
      onOpenDetails: () => (detailsKey = entry.key),
      test: testOf($libraryMeta, entry.key, game),
    }}
    {#if asRow}
      <GameListRow {...props} />
    {:else}
      <GameCard {...props} />
    {/if}
  {:else}
    {@const game = entry.storeGame}
    {@const props = {
      game,
      progress: $storeInstalls[entry.key],
      error: $storeInstallErrors[entry.key],
      onInstall: () => onInstallStoreGame(game),
      onCancel: () => cancelStoreInstall(game.store, game.app_name),
      onDismissError: () => dismissStoreInstallError(game.store, game.app_name),
      favorite,
      compat,
      onToggleFavorite: () => setFavorite(entry.key, !favorite),
      onOpenDetails: () => (detailsKey = entry.key),
    }}
    {#if asRow}
      <StoreGameRow {...props} />
    {:else}
      <StoreGameCard {...props} />
    {/if}
  {/if}
{/snippet}

{#snippet entryList(list: Entry[])}
  {#if viewMode === "grid"}
    <div class="grid" style:--card-min="{cardSize}px">
      {#each list as entry (entry.key)}
        {@render entryView(entry, false)}
      {/each}
    </div>
  {:else}
    <div class="list">
      {#each list as entry (entry.key)}
        {@render entryView(entry, true)}
      {/each}
    </div>
  {/if}
{/snippet}

{#snippet section(id: string, title: string, list: Entry[])}
  {@const isCollapsed = collapsed.includes(id)}
  <section class="section">
    <h2 class="section-title">
      <button
        type="button"
        class="section-toggle"
        aria-expanded={!isCollapsed}
        title={m.gameList_section_toggle()}
        onclick={() => toggleSection(id)}
      >
        <span class="chevron" class:collapsed={isCollapsed} aria-hidden="true">▾</span>
        {title}
        <span class="count">{list.length}</span>
      </button>
    </h2>
    {#if !isCollapsed}
      {@render entryList(list)}
    {/if}
  </section>
{/snippet}

{#if hasAnything}
  <div class="toolbar">
    <input
      bind:this={searchInput}
      type="search"
      placeholder={m.gameList_toolbar_searchShortcut()}
      bind:value={query}
      aria-label={m.gameList_toolbar_searchLabel()}
    />

    <select bind:value={sortBy} aria-label={m.gameList_toolbar_sortLabel()}>
      <option value="name-asc">{m.gameList_toolbar_sortNameAsc()}</option>
      <option value="name-desc">{m.gameList_toolbar_sortNameDesc()}</option>
      <option value="last-played">{m.gameList_toolbar_sortLastPlayed()}</option>
      <option value="recently-added">{m.gameList_toolbar_sortRecentlyAdded()}</option>
      <option value="playtime">{m.gameList_toolbar_sortPlaytime()}</option>
      <option value="runner">{m.gameList_toolbar_sortRunner()}</option>
    </select>

    <div class="toolbar-end">
      {#if viewMode === "grid"}
        <label class="size" title={m.gameList_toolbar_cardSize()}>
          <span class="size-icon small" aria-hidden="true">▪</span>
          <input
            type="range"
            min="130"
            max="260"
            step="10"
            bind:value={cardSize}
            aria-label={m.gameList_toolbar_cardSize()}
          />
          <span class="size-icon" aria-hidden="true">■</span>
        </label>
      {/if}

      <div class="view-toggle" role="group" aria-label={m.gameList_toolbar_viewToggleLabel()}>
        <button
          type="button"
          class:active={viewMode === "grid"}
          onclick={() => (viewMode = "grid")}
          aria-label={m.gameList_toolbar_gridView()}
          title={m.gameList_toolbar_gridView()}
        >
          ▦
        </button>
        <button
          type="button"
          class:active={viewMode === "list"}
          onclick={() => (viewMode = "list")}
          aria-label={m.gameList_toolbar_listView()}
          title={m.gameList_toolbar_listView()}
        >
          ☰
        </button>
      </div>
    </div>
  </div>

  {#if storesInLibrary.length > 0 || source !== "all"}
    <div class="sources" role="group" aria-label={m.gameList_sources_label()}>
      {#each sourceOptions as option (option)}
        <button
          type="button"
          class="source-chip"
          class:active={source === option}
          aria-pressed={source === option}
          onclick={() => (source = option)}
        >
          {option === "all"
            ? m.gameList_sources_all()
            : option === "local"
              ? m.gameList_sources_local()
              : storeName(option)}
          <span class="count">{perSource[option] ?? 0}</span>
        </button>
      {/each}
    </div>
  {/if}

  <LibraryChips bind:collection {counts} categories={$libraryMeta.categories} />
{/if}

{#if loadError && !hasAnything}
  <p class="load-error">{m.gameList_loadFailed({ error: backendError(loadError) })}</p>
{:else if !hasAnything}
  <div class="empty">
    <span class="empty-icon">🎮</span>
    <h3>{m.gameList_emptyLibrary_title()}</h3>
    <p>{m.gameList_emptyLibrary_hint()}</p>
    {#if nativeLibrary}
      <p class="native-library">{m.gameList_emptyLibrary_nativeFound({ path: nativeLibrary })}</p>
    {/if}
    {#if onAddNew}
      <button type="button" class="primary" onclick={onAddNew}>{m.library_addGame()}</button>
    {/if}
  </div>
{:else if shown.installed.length === 0 && shown.available.length === 0}
  {#if query.trim()}
    <div class="empty">
      <span class="empty-icon">🔍</span>
      <h3>{m.gameList_emptySearch_title()}</h3>
      <p>{m.gameList_emptySearch_hint({ query })}</p>
    </div>
  {:else}
    <div class="empty">
      <span class="empty-icon">🗂</span>
      <h3>{m.gameList_emptyCollection_title()}</h3>
      <p>{m.gameList_emptyCollection_hint()}</p>
    </div>
  {/if}
{:else}
  {#if continuePlaying.length > 0}
    <section class="section">
      <h2 class="section-title plain">{m.gameList_section_continue()}</h2>
      <div class="strip" style:--card-min="{cardSize}px">
        {#each continuePlaying as entry (entry.key)}
          {@render entryView(entry, false)}
        {/each}
      </div>
    </section>
  {/if}
  {#if shown.installed.length > 0}
    {@render section("installed", m.gameList_section_installed(), shown.installed)}
  {/if}
  {#if shown.available.length > 0}
    {@render section("available", m.gameList_section_available(), shown.available)}
  {/if}
{/if}

{#if steamNotice}
  <div class="notice {steamNotice.kind}" role="status">{steamNotice.text}</div>
{/if}

<GameDetails
  entry={detailsEntry}
  onClose={() => (detailsKey = null)}
  {onEdit}
  {onEditArtwork}
  {onInstallStoreGame}
  {onShowPrefix}
/>

<NotesDialog
  game={showingNotes}
  onClose={() => (showingNotes = null)}
  onEdit={(game) => {
    showingNotes = null;
    onEdit(game, "notes");
  }}
/>

<ConfirmDialog
  open={steamConfirm !== null}
  title={steamConfirm?.closeFirst
    ? m.gameList_steam_closeFirstTitle()
    : m.gameList_steam_confirmTitle()}
  message={steamConfirm?.closeFirst
    ? m.gameList_steam_closeFirstMessage({
        action:
          steamConfirm.action === "export"
            ? m.gameList_steam_closeFirstActionExport({ name: steamConfirm.game.name })
            : m.gameList_steam_closeFirstActionRemove({ name: steamConfirm.game.name }),
      })
    : steamConfirm
      ? m.gameList_steam_confirmMessage({
          action:
            steamConfirm.action === "export"
              ? m.gameList_steam_confirmActionExport({ name: steamConfirm.game.name })
              : m.gameList_steam_confirmActionRemove({ name: steamConfirm.game.name }),
        })
      : ""}
  confirmLabel={steamConfirm?.closeFirst
    ? m.gameList_steam_closeFirstButton()
    : m.gameList_steam_confirmButton()}
  onConfirm={() =>
    steamConfirm &&
    runSteamAction(steamConfirm.game, steamConfirm.action, true, steamConfirm.closeFirst)}
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

  .load-error {
    color: var(--danger);
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
    grid-template-columns: repeat(auto-fill, minmax(var(--card-min, 170px), 1fr));
    gap: 1.1em;
  }

  /* One row of cards at the grid's size; scrolls sideways when it's more. */
  .strip {
    display: grid;
    grid-auto-flow: column;
    grid-auto-columns: var(--card-min, 170px);
    gap: 1.1em;
    overflow-x: auto;
    padding-bottom: 0.4em;
  }

  .toolbar-end {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 0.8em;
  }

  .size {
    flex-direction: row;
    align-items: center;
    gap: 0.4em;
  }

  .size input {
    width: 7em;
    padding: 0;
    accent-color: var(--accent);
  }

  .size-icon {
    color: var(--text-muted);
    font-size: 0.9em;
  }

  .size-icon.small {
    font-size: 0.6em;
  }

  .sources {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4em;
    margin: -0.5em 0 0.7em;
  }

  .source-chip {
    padding: 0.3em 0.75em;
    font-size: 0.82em;
    border-radius: 999px;
    background: transparent;
    color: var(--text-muted);
  }

  .source-chip.active {
    background: var(--surface-raised);
    color: var(--text);
    border-color: var(--accent);
  }

  .count {
    margin-left: 0.35em;
    font-size: 0.85em;
    opacity: 0.7;
  }

  .section {
    margin-bottom: 1.8em;
  }

  .section-title {
    font-size: 1em;
    margin-bottom: 0.8em;
  }

  .section-title.plain {
    color: var(--text-muted);
    font-weight: 600;
  }

  .section-toggle {
    display: inline-flex;
    align-items: center;
    gap: 0.45em;
    padding: 0.2em 0.4em 0.2em 0.1em;
    background: none;
    border: none;
    color: var(--text-muted);
    font-size: 1em;
    font-weight: 600;
  }

  .section-toggle:hover:not(:disabled) {
    color: var(--text);
  }

  .chevron {
    display: inline-block;
    transition: transform 0.15s;
  }

  .chevron.collapsed {
    transform: rotate(-90deg);
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

  .native-library {
    max-width: 60ch;
    font-size: 0.9em;
  }
</style>
