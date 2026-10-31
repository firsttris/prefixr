<!--
  The prefixes page. Built for an agent's prefix per game, a couple of
  hundred of them: each row is named after the games in it, the folder the
  prefixes share is said once above them instead of in every path, and a
  search, filters and a sort narrow it down. Adding one is behind a button,
  with the explanation of what a prefix is, as it only matters then.
-->
<script lang="ts">
  import * as m from "#lib/paraglide/messages.js";
  import Message from "#lib/components/Message.svelte";
  import Menu from "#lib/components/Menu.svelte";
  import { onDestroy, onMount, tick } from "svelte";
  import { loadAll } from "#lib/load.ts";
  import { open } from "@tauri-apps/plugin-dialog";
  import { revealItemInDir } from "@tauri-apps/plugin-opener";
  import {
    prefixes,
    prefixStates,
    refreshPrefixes,
    refreshPrefixStates,
    addPrefix,
    deletePrefix,
  } from "#lib/stores/prefixes.ts";
  import { games, refreshGames } from "#lib/stores/games.ts";
  import { showLog } from "#lib/logViewer.ts";
  import Modal from "#lib/components/Modal.svelte";
  import WinetricksInstaller from "#lib/components/WinetricksInstaller.svelte";
  import WineToolsLauncher from "#lib/components/WineToolsLauncher.svelte";
  import { backendError } from "#lib/i18n/index.svelte.ts";
  import { isOneOf, loadSetting, saveSetting } from "#lib/persisted.ts";
  import {
    filterCounts,
    groupByRoot,
    matchesFilter,
    matchesQuery,
    namesLabel,
    prefixEntries,
    revealTarget,
    samePath,
    sortPrefixes,
    type PrefixEntry,
    type PrefixFilter,
    type PrefixSort,
  } from "#lib/prefixView.ts";
  import type { Game } from "#lib/types.ts";

  let {
    focusPath = null,
    onEditGame,
  }: {
    // A prefix to scroll to and mark, e.g. the one a game's details link to.
    focusPath?: string | null;
    onEditGame?: (game: Game) => void;
  } = $props();

  const SORTS = ["name", "path", "games"] as const;
  // How many of a prefix's games its row names before "and N more".
  const NAMES_SHOWN = 3;

  let path = $state("");
  let adding = $state(false);
  let busy = $state(false);
  let error = $state<unknown>(null);
  let loadError = $state<unknown>(null);
  let loaded = $state(false);
  let winetricksFor = $state<PrefixEntry | null>(null);
  let wineToolsFor = $state<PrefixEntry | null>(null);
  let deleting = $state<string | null>(null);
  let deleteBusy = $state(false);
  let deleteError = $state<unknown>(null);
  let query = $state("");
  let filter = $state<PrefixFilter>("all");
  let sort = $state<PrefixSort>(loadSetting("prefixes-sort", "name", isOneOf(...SORTS)));
  let highlighted = $state<string | null>(null);
  let highlightTimer: ReturnType<typeof setTimeout> | undefined;
  let listEl = $state<HTMLElement>();

  $effect(() => saveSetting("prefixes-sort", sort));

  let entries = $derived(prefixEntries($prefixes, $games, $prefixStates));
  let counts = $derived(filterCounts(entries));
  let shown = $derived(
    sortPrefixes(
      entries.filter((e) => matchesFilter(e, filter) && matchesQuery(e, query)),
      sort,
    ),
  );
  let groups = $derived(groupByRoot(shown));
  // A heading per shared folder; not for a list of unrelated paths only.
  let showRoots = $derived(groups.length > 1 || (groups.length === 1 && groups[0]!.root !== ""));
  // The empty page opens on the form, without the button to toggle it:
  // there's nothing else to do there.
  let empty = $derived(loaded && !loadError && $prefixes.length === 0);
  let formOpen = $derived(adding || empty);

  // The filters besides "all", the rarer ones only when there's one.
  let filters = $derived(
    (
      [
        ["unused", m.prefixManager_filterUnused()],
        ["shared", m.prefixManager_filterShared()],
        ["not_set_up", m.prefixManager_filterNotSetUp()],
        ["missing", m.prefixManager_filterMissing()],
        ["unreadable", m.prefixManager_filterUnreadable()],
      ] as const
    ).filter(([value]) => value === "unused" || counts[value] > 0 || filter === value),
  );

  // The games that use the prefix about to be deleted, so the dialog can
  // name them.
  let affectedGames = $derived(
    deleting ? $games.filter((g) => samePath(g.prefix_path, deleting!)).map((g) => g.name) : [],
  );

  // A game's prefix asked for that isn't in the list (any more).
  let unlisted = $state<string | null>(null);
  let unlistedGames = $derived(
    unlisted ? $games.filter((g) => samePath(g.prefix_path, unlisted!)).map((g) => g.name) : [],
  );
  let readding = $state(false);
  let readdError = $state<unknown>(null);
  // Read out when a prefix is revealed, for whoever can't see the mark.
  let announcement = $state("");

  // The folders are looked at after the list is there, and don't hold it up.
  function refreshStates() {
    refreshPrefixStates().catch(() => {});
  }

  onMount(() => {
    loadAll(refreshPrefixes(), refreshGames())
      .catch((e) => (loadError = e))
      .finally(() => (loaded = true));
    refreshStates();
  });

  onDestroy(() => clearTimeout(highlightTimer));

  // Scrolls to a prefix, marks it for a moment and moves focus to it,
  // clearing whatever would hide it.
  async function reveal(prefixPath: string) {
    query = "";
    filter = "all";
    unlisted = null;
    highlighted = prefixPath;
    clearTimeout(highlightTimer);
    highlightTimer = setTimeout(() => (highlighted = null), 2500);
    await tick();
    const row = listEl?.querySelector<HTMLElement>(`[data-path="${CSS.escape(prefixPath)}"]`);
    row?.scrollIntoView({ block: "center" });
    row?.focus({ preventScroll: true });
    const entry = entries.find((e) => e.path === prefixPath);
    if (entry) announcement = m.prefixManager_revealed({ name: entryLabel(entry) });
  }

  // Once the list is there: the page may still be loading when it's asked.
  let handled: string | null = null;
  $effect(() => {
    const wanted = focusPath;
    if (!wanted || wanted === handled || !loaded) return;
    handled = wanted;
    const target = revealTarget(wanted, $prefixes);
    if (target?.listed) reveal(target.path);
    else if (target) unlisted = target.path;
  });

  async function addAgain() {
    if (!unlisted) return;
    readding = true;
    readdError = null;
    try {
      const added = await addPrefix(unlisted);
      refreshStates();
      await reveal(added);
    } catch (e) {
      readdError = e;
    } finally {
      readding = false;
    }
  }

  // What a prefix is called in a dialog's title: its games, or its path.
  function entryLabel(entry: PrefixEntry): string {
    return entry.games.length ? namesLabel(entry.games.map((g) => g.name)) : entry.rest;
  }

  async function pickFolder() {
    const selected = await open({ directory: true });
    if (typeof selected === "string") {
      path = selected;
    }
  }

  async function handleSubmit(event: Event) {
    event.preventDefault();
    if (!path) return;
    busy = true;
    error = null;
    try {
      const added = await addPrefix(path);
      path = "";
      adding = false;
      refreshStates();
      reveal(added);
    } catch (e) {
      error = e;
    } finally {
      busy = false;
    }
  }

  function askDelete(prefixPath: string) {
    deleteError = null;
    deleting = prefixPath;
  }

  async function confirmDelete() {
    if (!deleting) return;
    deleteBusy = true;
    deleteError = null;
    try {
      await deletePrefix(deleting);
      deleting = null;
    } catch (e) {
      deleteError = e;
    } finally {
      deleteBusy = false;
    }
  }
</script>

{#snippet drive()}<strong>{m.prefixManager_explainerDrive()}</strong>{/snippet}
{#snippet gameNames()}<strong>{affectedGames.join(", ")}</strong>{/snippet}

<section class="prefixes">
  <!-- Nothing to search, sort or toggle on the empty page: the form is open. -->
  {#if !empty}
    <div class="toolbar">
      <input
        type="search"
        bind:value={query}
        placeholder={m.prefixManager_searchPlaceholder()}
        aria-label={m.prefixManager_searchPlaceholder()}
      />
      <select bind:value={sort} aria-label={m.prefixManager_sortLabel()}>
        <option value="name">{m.prefixManager_sortName()}</option>
        <option value="path">{m.prefixManager_sortPath()}</option>
        <option value="games">{m.prefixManager_sortGames()}</option>
      </select>
      <button
        type="button"
        class="primary add-toggle"
        aria-expanded={formOpen}
        onclick={() => (adding = !adding)}
      >
        {m.prefixManager_addToggle()}
      </button>
    </div>
  {/if}

  <p class="visually-hidden" aria-live="polite">{announcement}</p>

  {#if unlisted}
    <div class="notice" role="status">
      <p>
        {m.prefixManager_notListed({ games: unlistedGames.join(", ") || "–" })}
      </p>
      <code class="dialog-path">{unlisted}</code>
      {#if readdError}
        <p class="error">{backendError(readdError)}</p>
      {/if}
      <div class="form-actions">
        <button type="button" class="ghost" onclick={() => (unlisted = null)}
          >{m.common_close()}</button
        >
        <button type="button" class="primary" disabled={readding} onclick={addAgain}>
          {readding ? m.prefixManager_adding() : m.prefixManager_addAgain()}
        </button>
      </div>
    </div>
  {/if}

  {#if formOpen}
    <form class="panel" onsubmit={handleSubmit}>
      <p class="explainer">
        <Message message={m.prefixManager_explainer} parts={{ drive }} />
      </p>
      <p class="hint">{m.prefixManager_hint()}</p>
      <label>
        {m.prefixManager_pathLabel()}
        <div class="row">
          <input placeholder={m.prefixManager_pathPlaceholder()} bind:value={path} />
          <button type="button" onclick={pickFolder}>{m.prefixManager_chooseFolder()}</button>
        </div>
      </label>
      {#if error}
        <p class="error">{backendError(error)}</p>
      {/if}
      <div class="form-actions">
        {#if $prefixes.length > 0}
          <button type="button" class="ghost" onclick={() => (adding = false)}
            >{m.common_cancel()}</button
          >
        {/if}
        <button type="submit" class="primary" disabled={busy || !path}>
          {busy ? m.prefixManager_adding() : m.prefixManager_addPrefix()}
        </button>
      </div>
    </form>
  {/if}

  {#if loadError}
    <p class="error">{backendError(loadError)}</p>
  {:else if $prefixes.length > 0}
    <div class="chips" role="group" aria-label={m.prefixManager_filterLabel()}>
      {@render chip("all", m.prefixManager_filterAll())}
      {#each filters as [value, label] (value)}
        {@render chip(value, label)}
      {/each}
      {#if shown.length !== entries.length}
        <span class="shown-count">
          {m.prefixManager_shownCount({ shown: shown.length, total: entries.length })}
        </span>
      {/if}
    </div>

    <div class="list" bind:this={listEl}>
      {#each groups as group (group.root)}
        {#if showRoots}
          <h2 class="root" title={group.root}>
            <span class="root-path">{group.root || m.prefixManager_elsewhere()}</span>
            <span class="count">{group.entries.length}</span>
          </h2>
        {/if}
        <ul>
          {#each group.entries as entry (entry.path)}
            {@render row(entry)}
          {/each}
        </ul>
      {:else}
        <p class="empty">{m.prefixManager_noMatch()}</p>
      {/each}
    </div>
  {/if}
</section>

{#snippet chip(value: PrefixFilter, label: string)}
  <button
    type="button"
    class="chip"
    class:active={filter === value}
    aria-pressed={filter === value}
    onclick={() => (filter = value)}
  >
    {label}
    <span class="count">{counts[value]}</span>
  </button>
{/snippet}

{#snippet row(entry: PrefixEntry)}
  {@const more = entry.games.length - NAMES_SHOWN}
  <!-- Focusable from code only, for reveal(). -->
  <li data-path={entry.path} tabindex="-1" class:highlighted={highlighted === entry.path}>
    <div class="prefix-info">
      <div class="names">
        {#each entry.games.slice(0, NAMES_SHOWN) as game, i (game.id)}
          {#if i > 0}<span class="sep">,</span>{/if}
          {#if onEditGame}
            <button
              type="button"
              class="game-link"
              title={m.prefixManager_editGame()}
              onclick={() => onEditGame(game)}>{game.name}</button
            >
          {:else}
            <span class="game-name">{game.name}</span>
          {/if}
        {:else}
          <span class="unused">{m.prefixManager_unused()}</span>
        {/each}
        {#if more > 0}
          <span class="more" title={entry.games.map((g) => g.name).join(", ")}>
            {m.prefixManager_moreGames({ count: more })}
          </span>
        {/if}
        {#if entry.state === "not_set_up"}
          <span class="badge" title={m.prefixManager_notSetUpTitle()}>
            {m.prefixManager_filterNotSetUp()}
          </span>
        {:else if entry.state === "missing"}
          <span class="badge missing" title={m.prefixManager_missingTitle()}>
            {m.prefixManager_filterMissing()}
          </span>
        {:else if entry.state === "unreadable"}
          <span class="badge missing" title={m.prefixManager_unreadableTitle()}>
            {m.prefixManager_filterUnreadable()}
          </span>
        {:else if entry.state === "timed_out"}
          <span class="badge" title={m.prefixManager_timedOutTitle()}>
            {m.prefixManager_timedOut()}
          </span>
        {/if}
      </div>
      <span class="path" title={entry.path}>{entry.rest}</span>
    </div>
    <div class="actions">
      <button
        type="button"
        class="ghost small"
        title={m.prefixManager_installDeps()}
        disabled={entry.state === "missing"}
        onclick={() => (winetricksFor = entry)}
      >
        {m.prefixManager_depsShort()}
      </button>
      <button
        type="button"
        class="ghost small"
        title={m.prefixManager_openWineTools()}
        disabled={entry.state === "missing"}
        onclick={() => (wineToolsFor = entry)}
      >
        {m.prefixManager_wineToolsTitle()}
      </button>
      <Menu label={m.prefixManager_moreActions()} triggerClass="icon-btn ghost-icon">
        {#snippet trigger()}⋮{/snippet}
        {#snippet items(close)}
          <button
            type="button"
            role="menuitem"
            disabled={entry.state === "missing"}
            onclick={() => {
              close();
              revealItemInDir(entry.path).catch(() => {});
            }}
          >
            {m.prefixManager_showFolder()}
          </button>
          <div role="separator"></div>
          <button
            type="button"
            role="menuitem"
            class="danger"
            onclick={() => {
              close();
              askDelete(entry.path);
            }}
          >
            {m.prefixManager_remove()}
          </button>
        {/snippet}
      </Menu>
    </div>
  </li>
{/snippet}

<Modal
  open={deleting !== null}
  title={m.prefixManager_removeTitle()}
  onClose={() => (deleting = null)}
>
  {#if deleting}
    <p class="dialog-path">{deleting}</p>
    {#if affectedGames.length > 0}
      <p>
        <Message message={m.prefixManager_usedBy} parts={{ games: gameNames }} />
      </p>
    {/if}
    <p>{m.prefixManager_deleteExplain()}</p>
    {#if deleteError}
      <p class="error">{backendError(deleteError)}</p>
    {/if}
    <div class="dialog-actions">
      <button type="button" class="ghost" onclick={() => (deleting = null)}
        >{m.common_cancel()}</button
      >
      <button type="button" class="danger" disabled={deleteBusy} onclick={confirmDelete}>
        {m.prefixManager_removeFromAppOnly()}
      </button>
    </div>
  {/if}
</Modal>

<Modal
  open={winetricksFor !== null}
  title={winetricksFor
    ? `${m.prefixManager_installDeps()} · ${entryLabel(winetricksFor)}`
    : m.prefixManager_installDeps()}
  onClose={() => (winetricksFor = null)}
>
  {#if winetricksFor}
    <WinetricksInstaller prefixPath={winetricksFor.path} onShowLog={showLog} />
  {/if}
</Modal>

<Modal
  open={wineToolsFor !== null}
  title={wineToolsFor
    ? `${m.prefixManager_wineToolsTitle()} · ${entryLabel(wineToolsFor)}`
    : m.prefixManager_wineToolsTitle()}
  onClose={() => (wineToolsFor = null)}
>
  {#if wineToolsFor}
    <WineToolsLauncher prefixPath={wineToolsFor.path} />
  {/if}
</Modal>

<style>
  .prefixes {
    display: flex;
    flex-direction: column;
    gap: 1em;
    max-width: var(--page-max);
  }

  .toolbar {
    display: flex;
    align-items: center;
    gap: 0.6em;
  }

  .toolbar input[type="search"] {
    flex: 1;
    max-width: 360px;
  }

  .toolbar select {
    width: auto;
  }

  .add-toggle {
    margin-left: auto;
    white-space: nowrap;
  }

  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }

  .notice {
    display: flex;
    flex-direction: column;
    gap: 0.6em;
    padding: 1em 1.2em;
    background: var(--surface);
    border: 1px solid var(--accent);
    border-radius: var(--radius);
    font-size: 0.9em;
  }

  .panel {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 1.2em;
    display: flex;
    flex-direction: column;
    gap: 1em;
  }

  .explainer {
    color: var(--text-muted);
    font-size: 0.9em;
    max-width: 70ch;
  }

  .explainer :global(strong) {
    color: var(--text);
  }

  .hint {
    color: var(--text-muted);
    font-size: 0.85em;
  }

  .row {
    display: flex;
    gap: 0.5em;
  }

  .row input {
    flex: 1;
  }

  .form-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.6em;
  }

  .error {
    color: var(--danger);
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.4em;
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: 0.35em;
    padding: 0.3em 0.8em;
    font-size: 0.82em;
    border-radius: 999px;
    background: var(--surface);
    color: var(--text-muted);
  }

  .chip.active {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
  }

  .chip .count {
    opacity: 0.75;
  }

  .shown-count {
    margin-left: auto;
    color: var(--text-muted);
    font-size: 0.82em;
  }

  .list {
    display: flex;
    flex-direction: column;
    gap: 0.4em;
  }

  .root {
    display: flex;
    align-items: baseline;
    gap: 0.6em;
    margin-top: 0.6em;
    font-size: 0.8em;
    font-weight: 600;
    color: var(--text-muted);
  }

  .root:first-child {
    margin-top: 0;
  }

  .root-path {
    font-family: monospace;
    word-break: break-all;
  }

  .root .count {
    font-weight: 400;
    opacity: 0.75;
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  li {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1em;
    padding: 0.35em 0.4em 0.35em 0.8em;
    background: var(--surface);
    border: 1px solid transparent;
    border-radius: 6px;
    font-size: 0.9em;
    transition: border-color 0.3s;
  }

  li:hover {
    background: var(--surface-raised);
  }

  li.highlighted {
    border-color: var(--accent);
  }
  .prefix-info {
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .names {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 0 0.3em;
  }

  .sep {
    margin-left: -0.3em;
    color: var(--text-muted);
  }

  .game-link,
  .game-name {
    font-weight: 600;
    color: var(--text);
  }

  .game-link {
    background: none;
    border: none;
    padding: 0;
    font-size: inherit;
    text-align: left;
  }

  .game-link:hover:not(:disabled) {
    color: var(--accent);
    text-decoration: underline;
  }

  .unused {
    color: var(--text-muted);
    font-style: italic;
  }

  .more {
    color: var(--text-muted);
    font-size: 0.9em;
  }

  .badge {
    align-self: center;
    padding: 0 0.55em;
    border-radius: 999px;
    border: 1px solid var(--border);
    color: var(--text-muted);
    font-size: 0.75em;
    white-space: nowrap;
  }

  .badge.missing {
    border-color: transparent;
    background: var(--danger-bg);
    color: var(--danger);
  }

  .path {
    color: var(--text-muted);
    font-family: monospace;
    font-size: 0.82em;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 0.2em;
    flex-shrink: 0;
  }

  button.small {
    padding: 0.3em 0.7em;
    font-size: 0.82em;
    color: var(--text-muted);
  }

  button.small:hover:not(:disabled) {
    color: var(--text);
  }

  .actions :global(button.ghost-icon) {
    width: 2em;
    height: 2em;
    background: transparent;
    border-color: transparent;
    font-size: 1.05em;
  }

  .empty {
    color: var(--text-muted);
    font-size: 0.9em;
    padding: 1em 0;
  }

  .dialog-path {
    font-family: monospace;
    word-break: break-all;
  }

  .dialog-actions {
    display: flex;
    justify-content: flex-end;
    flex-wrap: wrap;
    gap: 0.6em;
    margin-top: 1.2em;
  }
</style>
