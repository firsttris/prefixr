<!--
  Everything about one game in a panel at the window's right edge: its
  status and play time, how well it runs and was tested to, its
  categories, and what can be
  done with it. Opens from a click on a card's cover or a right-click on
  a card or row; Escape or a click beside it closes it.
-->
<script lang="ts">
  import * as m from "$lib/paraglide/messages";
  import type { Entry } from "$lib/libraryView";
  import type { Game, StoreGame, StoreInstallInfo } from "$lib/types";
  import { gameRunState, killGame, launchGame } from "$lib/stores/games";
  import {
    formatBytes,
    getStoreInstallDefaults,
    storeCoverUrl,
    storeName,
  } from "$lib/stores/gameStores";
  import { getGameCover } from "$lib/stores/steamgriddb";
  import {
    categoryIdsOf,
    compatibility,
    formatPlaytime,
    libraryMeta,
    setFavorite,
    setInCategory,
    setHidden,
    setTestStatus,
    testOf,
  } from "$lib/stores/libraryMeta";
  import { TEST_STATUSES, TEST_SYMBOL, testLabel, testedOn } from "$lib/testStatus";
  import type { TestStatus } from "$lib/types";
  import { revealItemsInDir } from "@tauri-apps/plugin-opener";
  import { backendError, getLocale } from "$lib/i18n/index.svelte";
  import { isTopLayer, openLayer, trapTab } from "$lib/layers";
  import CompatBadges, { tierName } from "./CompatBadges.svelte";
  import FavoriteStar from "./FavoriteStar.svelte";
  import GameFallbackIcon from "./GameFallbackIcon.svelte";

  let {
    entry,
    onClose,
    onEdit,
    onEditArtwork,
    onInstallStoreGame,
    onShowPrefix,
  }: {
    entry: Entry | null;
    onClose: () => void;
    onEdit: (game: Game) => void;
    onEditArtwork: (game: Game) => void;
    onInstallStoreGame: (game: StoreGame) => void;
    // To the prefixes page, at this game's prefix.
    onShowPrefix?: (path: string) => void;
  } = $props();

  let key = $derived(entry?.key ?? "");
  let game = $derived(entry?.kind === "game" ? entry.game : null);
  let storeGame = $derived(entry?.kind === "store" ? entry.storeGame : null);
  let favorite = $derived($libraryMeta.favorites.includes(key));
  let hidden = $derived($libraryMeta.hidden.includes(key));
  let inCategories = $derived(categoryIdsOf($libraryMeta, key));
  let activity = $derived($libraryMeta.activity[key]);
  let compat = $derived($compatibility[key]);
  let test = $derived(testOf($libraryMeta, key, game ?? undefined));
  let runState = $derived(game ? $gameRunState[game.id] : undefined);
  let source = $derived(
    game?.store ? storeName(game.store.store) : storeGame ? storeName(storeGame.store) : null,
  );

  let coverSrc = $state<string | null>(null);
  $effect(() => {
    coverSrc = storeGame ? storeCoverUrl(storeGame) : null;
    if (!game?.cover_url) return;
    let stale = false;
    getGameCover(game.id, game.cover_grid_id)
      .then((src) => !stale && (coverSrc = src))
      .catch(() => {});
    return () => {
      stale = true;
    };
  });

  // A store game's size, asked of the store's tool when the panel opens.
  let sizes = $state<StoreInstallInfo | "loading" | null>(null);
  $effect(() => {
    sizes = null;
    if (!storeGame) return;
    const { store, app_name } = storeGame;
    let stale = false;
    sizes = "loading";
    getStoreInstallDefaults(store, app_name)
      .then((defaults) => !stale && (sizes = defaults.sizes))
      .catch(() => !stale && (sizes = null));
    return () => {
      stale = true;
    };
  });

  let error = $state<unknown>(null);
  async function attempt(action: () => Promise<void>) {
    try {
      await action();
      error = null;
    } catch (e) {
      error = e;
    }
  }

  // A note an agent wrote stays when the status is changed here.
  function setTest(status: TestStatus | null, note = test?.result.note ?? "") {
    const id = game?.id;
    if (id) attempt(() => setTestStatus(id, status, note));
  }

  function toggleCategory(id: string) {
    attempt(() => setInCategory(id, key, !inCategories.includes(id)));
  }

  function date(secs: number | null | undefined): string {
    if (!secs) return m.details_never();
    return new Date(secs * 1000).toLocaleDateString(getLocale(), {
      day: "numeric",
      month: "long",
      year: "numeric",
    });
  }

  let panel = $state<HTMLElement>();
  let layer: ReturnType<typeof openLayer> | null = null;
  $effect(() => {
    if (!panel) return;
    const opener = document.activeElement;
    layer = openLayer();
    panel.focus();
    return () => {
      layer?.close();
      layer = null;
      if (opener instanceof HTMLElement && opener.isConnected) opener.focus();
    };
  });

  // A dialog opened from here (editing, installing) is on top and handles
  // the keys itself.
  function handleKeydown(event: KeyboardEvent) {
    if (!layer || !isTopLayer(layer.id) || !panel) return;
    if (event.key === "Escape") onClose();
    else trapTab(event, panel);
  }
</script>

<svelte:window onkeydown={entry ? handleKeydown : undefined} />

{#if entry}
  <div class="backdrop" onclick={onClose} role="presentation"></div>
  <div
    bind:this={panel}
    class="panel"
    role="dialog"
    aria-modal="true"
    aria-label={entry.name}
    tabindex="-1"
  >
    <div class="hero">
      {#if coverSrc}
        <img class="cover" src={coverSrc} alt="" class:muted={!game} />
      {:else}
        <div class="cover placeholder"><GameFallbackIcon class="icon fallback" /></div>
      {/if}
      <button
        type="button"
        class="close icon-btn"
        aria-label={m.details_close()}
        title={m.details_close()}
        onclick={onClose}>✕</button
      >
    </div>

    <div class="content">
      <div class="title-row">
        <h2>{entry.name}</h2>
        <FavoriteStar {favorite} onToggle={() => attempt(() => setFavorite(key, !favorite))} />
      </div>
      <div class="pills">
        <span class="pill">{source ?? m.details_ownGame()}</span>
        <span class="pill" class:ok={!!game}>
          {game ? m.details_installed() : m.details_notInstalled()}
        </span>
        <CompatBadges {compat} />
      </div>

      <div class="actions">
        {#if game}
          {#if runState?.running}
            <button type="button" class="danger" onclick={() => killGame(game.id)}
              >{m.details_stop()}</button
            >
          {:else}
            <button
              type="button"
              class="primary"
              disabled={runState?.initializing}
              onclick={() => launchGame(game.id)}
              >{runState?.initializing ? m.gameCard_initializing() : m.details_play()}</button
            >
          {/if}
          <button type="button" onclick={() => onEdit(game)}>{m.details_edit()}</button>
          <button type="button" onclick={() => onEditArtwork(game)}>{m.details_artwork()}</button>
        {:else if storeGame}
          <button type="button" class="primary" onclick={() => onInstallStoreGame(storeGame)}
            >{m.details_install()}</button
          >
        {/if}
      </div>

      <dl class="facts">
        <dt>{m.details_playtime()}</dt>
        <dd>{formatPlaytime(activity?.playtime_secs ?? 0) ?? m.details_noPlaytime()}</dd>
        <dt>{m.details_lastPlayed()}</dt>
        <dd>{date(activity?.last_played)}</dd>
        {#if activity?.added_at}
          <dt>{m.details_added()}</dt>
          <dd>{date(activity.added_at)}</dd>
        {/if}
        {#if storeGame}
          <dt>{m.details_size()}</dt>
          <dd>
            {#if sizes === "loading"}
              {m.details_sizeLoading()}
            {:else if sizes && (sizes.download_size || sizes.disk_size)}
              {m.details_downloadSize({
                download: formatBytes(sizes.download_size, getLocale()) ?? "?",
                disk: formatBytes(sizes.disk_size, getLocale()) ?? "?",
              })}
            {:else}
              {m.details_unknown()}
            {/if}
          </dd>
        {/if}
        {#if game}
          <dt>{m.details_runner()}</dt>
          <dd>{game.runner_id}</dd>
          <dt>{m.details_prefix()}</dt>
          <dd>
            <span class="path">{game.prefix_path}</span>
            {#if onShowPrefix}
              <button
                type="button"
                class="link link-button"
                onclick={() => {
                  const path = game?.prefix_path;
                  onClose();
                  if (path) onShowPrefix(path);
                }}>{m.details_showPrefix()} →</button
              >
            {/if}
          </dd>
          <dt>{m.details_exe()}</dt>
          <dd>
            <span class="path">{game.exe_path}</span>
            <button
              type="button"
              class="link link-button"
              onclick={() => {
                const exe = game?.exe_path;
                if (exe) revealItemsInDir(exe).catch((e) => (error = e));
              }}>{m.gameCard_menuShowInFileManager()} →</button
            >
          </dd>
        {/if}
      </dl>

      <h3>{m.details_compatibility()}</h3>
      {#if compat?.umu_id || compat?.protondb_tier}
        {#if compat.protondb_tier}
          <p>{m.details_protondb({ tier: tierName(compat.protondb_tier) })}</p>
        {/if}
        {#if compat.umu_id}
          <p class="muted">{m.details_umuFix({ id: compat.umu_id })}</p>
        {/if}
        {#if compat.steam_app_id}
          <a
            class="link"
            href="https://www.protondb.com/app/{compat.steam_app_id}"
            target="_blank"
            rel="noreferrer">{m.details_protondbLink()} ↗</a
          >
        {/if}
      {:else}
        <p class="muted">{m.details_noUmuFix()}</p>
      {/if}

      {#if game}
        <h3>{m.details_test()}</h3>
        <div class="test-choice" role="group" aria-label={m.details_test()}>
          {#each TEST_STATUSES as status (status)}
            {@const chosen = test?.result.status === status}
            <button
              type="button"
              class="test-option"
              class:active={chosen}
              aria-pressed={chosen}
              title={chosen ? m.details_testClear() : undefined}
              onclick={() => setTest(chosen ? null : status)}
            >
              <span class="test-symbol {status}" aria-hidden="true">{TEST_SYMBOL[status]}</span>
              {testLabel(status)}
            </button>
          {/each}
        </div>
        {#if test}
          <p class="muted">{testedOn(test.result.tested_at)}</p>
          {#if test.result.note}
            <p class="notes">{test.result.note}</p>
          {/if}
          {#if !test.current}
            <p class="muted">{m.details_testOutdated({ runner: test.result.runner_id })}</p>
          {/if}
        {:else}
          <p class="muted">{m.details_untested()}</p>
        {/if}
      {/if}

      <h3>{m.details_categories()}</h3>
      {#if $libraryMeta.categories.length === 0}
        <p class="muted">{m.details_noCategories()}</p>
      {:else}
        <div class="categories">
          {#each $libraryMeta.categories as category (category.id)}
            <button
              type="button"
              class="category"
              class:active={inCategories.includes(category.id)}
              aria-pressed={inCategories.includes(category.id)}
              onclick={() => toggleCategory(category.id)}
            >
              {inCategories.includes(category.id) ? "✓ " : ""}{category.name}
            </button>
          {/each}
        </div>
      {/if}

      {#if game?.notes}
        <h3>{m.details_notes()}</h3>
        <p class="notes">{game.notes}</p>
      {/if}

      <div class="footer">
        {#if hidden}
          <p class="muted">{m.details_hiddenNote()}</p>
        {/if}
        <button type="button" class="ghost" onclick={() => attempt(() => setHidden(key, !hidden))}>
          {hidden ? m.details_unhide() : m.details_hide()}
        </button>
      </div>

      {#if error}
        <p class="error">{backendError(error)}</p>
      {/if}
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 90;
    background: rgba(0, 0, 0, 0.4);
  }

  .panel {
    position: fixed;
    top: 0;
    right: 0;
    bottom: 0;
    z-index: 95;
    width: min(28em, 100vw);
    overflow-y: auto;
    background: var(--surface);
    border-left: 1px solid var(--border);
    box-shadow: var(--shadow-lg);
    outline: none;
    animation: slide-in 0.18s ease-out;
  }

  @keyframes slide-in {
    from {
      transform: translateX(2em);
      opacity: 0;
    }
  }

  .hero {
    position: relative;
    height: 15em;
    background: var(--surface-raised);
    overflow: hidden;
  }

  .cover {
    width: 100%;
    height: 100%;
    object-fit: cover;
    object-position: center 20%;
    display: block;
    mask-image: linear-gradient(to bottom, #000 55%, transparent);
  }

  .cover.muted {
    filter: saturate(0.7);
  }

  .cover.placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    mask-image: none;
  }

  .placeholder :global(.icon) {
    width: 30%;
    height: 50%;
  }

  .close {
    position: absolute;
    top: 0.7em;
    right: 0.7em;
    background: rgba(15, 16, 20, 0.6);
    color: #fff;
    backdrop-filter: blur(8px);
  }

  .content {
    padding: 0 1.3em 1.5em;
    margin-top: -2.2em;
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 0.7em;
  }

  .title-row {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 0.5em;
  }

  h2 {
    font-size: 1.35em;
    line-height: 1.25;
  }

  h3 {
    margin-top: 0.6em;
    font-size: 0.78em;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted);
  }

  .pills {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.35em;
  }

  .pill {
    font-size: 0.75em;
    padding: 0.15em 0.6em;
    border-radius: 999px;
    background: var(--surface-raised);
    border: 1px solid var(--border);
    color: var(--text-muted);
  }

  .pill.ok {
    color: var(--success);
    border-color: rgba(62, 207, 142, 0.35);
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5em;
  }

  .facts {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 0.35em 1em;
    margin: 0.3em 0 0;
    font-size: 0.88em;
  }

  dt {
    color: var(--text-muted);
  }

  dd {
    margin: 0;
    min-width: 0;
  }

  .path {
    font-family: ui-monospace, monospace;
    font-size: 0.9em;
    overflow-wrap: anywhere;
    user-select: text;
  }

  .muted {
    color: var(--text-muted);
    font-size: 0.88em;
  }

  .link {
    color: var(--accent);
    font-size: 0.88em;
    text-decoration: none;
  }

  .link-button {
    display: block;
    background: none;
    border: none;
    padding: 0;
    margin-top: 0.15em;
  }

  .link-button:hover:not(:disabled) {
    text-decoration: underline;
  }

  .categories {
    display: flex;
    flex-wrap: wrap;
    gap: 0.35em;
  }

  .category {
    padding: 0.3em 0.8em;
    font-size: 0.85em;
    border-radius: 999px;
  }

  .category.active {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
  }

  .test-choice {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4em;
  }

  .test-option {
    display: inline-flex;
    align-items: center;
    gap: 0.4em;
    padding: 0.35em 0.7em;
    border-radius: 999px;
    font-size: 0.85em;
  }

  .test-option.active {
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent);
  }

  .test-symbol {
    display: inline-grid;
    place-items: center;
    width: 1.3em;
    height: 1.3em;
    border-radius: 50%;
    font-size: 0.8em;
    font-weight: 700;
  }

  .test-symbol.works {
    background: var(--success);
    color: #08130d;
  }

  .test-symbol.issues {
    background: #f5b43c;
    color: #1d1404;
  }

  .test-symbol.broken {
    background: var(--danger);
    color: #fff;
  }

  .notes {
    white-space: pre-wrap;
    font-size: 0.88em;
    user-select: text;
    max-height: 12em;
    overflow-y: auto;
  }

  .footer {
    margin-top: 0.8em;
    padding-top: 0.8em;
    border-top: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 0.4em;
  }

  .error {
    color: var(--danger);
    font-size: 0.85em;
  }
</style>
