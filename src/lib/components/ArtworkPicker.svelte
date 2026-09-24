<script lang="ts">
  import { onMount, untrack } from "svelte";
  import type { ArtworkKind, Game, SteamGridDbGameMatch, SteamGridDbGrid } from "$lib/types";
  import {
    searchSteamGridDbGames,
    listSteamGridDbGrids,
    listSteamGridDbIcons,
    setGameCover,
    removeGameCover,
    setGameIcon,
    removeGameIcon,
    listSteamGridDbArtwork,
    setGameArtwork,
    removeGameArtwork,
  } from "$lib/stores/steamgriddb";
  import { prettifyExeName } from "$lib/gameName";
  import { backendError, t } from "$lib/i18n/index.svelte";

  let { game, onDone }: { game: Game; onDone: () => void } = $props();

  type Kind = "cover" | "icon" | ArtworkKind;

  const kindLabels: Record<Kind, string> = $derived({
    cover: t("artworkPicker.kindCover"),
    icon: t("artworkPicker.kindIcon"),
    wide: t("artworkPicker.kindWide"),
    hero: t("artworkPicker.kindHero"),
    logo: t("artworkPicker.kindLogo"),
  });

  // Only Steam shows these; Prefixr itself uses the cover and icon.
  const steamOnlyHints: Partial<Record<Kind, string>> = $derived({
    wide: t("artworkPicker.hintWide"),
    hero: t("artworkPicker.hintHero"),
    logo: t("artworkPicker.hintLogo"),
  });

  // `game` is only read here to seed this picker's initial state — a fresh
  // instance is mounted per game (see +page.svelte), so a one-time snapshot
  // is intentional, not a missed reactivity dependency.
  const initial = untrack(() => ({
    stage: (game.steamgriddb_id ? "assets" : "search") as "search" | "assets",
    // Cleans up leftover ".exe"/camelCase artifacts in names stored before
    // GameForm started prettifying them — a no-op on an already-clean name.
    query: prettifyExeName(game.name),
    selectedGame: game.steamgriddb_id
      ? { id: game.steamgriddb_id, name: game.name, verified: false }
      : null,
  }));

  let kind = $state<Kind>("cover");
  let stage = $state<"search" | "assets">(initial.stage);
  let query = $state(initial.query);
  let matches = $state<SteamGridDbGameMatch[]>([]);
  let selectedGame = $state<SteamGridDbGameMatch | null>(initial.selectedGame);
  let assetOptions = $state<SteamGridDbGrid[]>([]);
  let loading = $state(false);
  let error = $state<unknown>(null);
  let picking = $state(false);
  let removing = $state(false);

  const currentUrl = $derived(
    kind === "cover"
      ? game.cover_url
      : kind === "icon"
        ? game.steamgriddb_icon_url
        : (game.artwork[kind] ?? null),
  );

  onMount(async () => {
    if (stage === "assets" && selectedGame) {
      await loadAssets(selectedGame.id);
    } else {
      await handleSearch();
    }
  });

  async function handleSearch() {
    if (!query.trim()) return;
    loading = true;
    error = null;
    matches = [];
    try {
      matches = await searchSteamGridDbGames(query);
    } catch (e) {
      error = e;
    } finally {
      loading = false;
    }
  }

  async function selectMatch(match: SteamGridDbGameMatch) {
    selectedGame = match;
    stage = "assets";
    await loadAssets(match.id);
  }

  async function loadAssets(steamgriddbId: number) {
    loading = true;
    error = null;
    assetOptions = [];
    try {
      assetOptions =
        kind === "cover"
          ? await listSteamGridDbGrids(steamgriddbId)
          : kind === "icon"
            ? await listSteamGridDbIcons(steamgriddbId)
            : await listSteamGridDbArtwork(steamgriddbId, kind);
    } catch (e) {
      error = e;
    } finally {
      loading = false;
    }
  }

  async function switchKind(next: Kind) {
    if (kind === next) return;
    kind = next;
    if (stage === "assets" && selectedGame) {
      await loadAssets(selectedGame.id);
    }
  }

  async function pickAsset(asset: SteamGridDbGrid) {
    if (!selectedGame) return;
    picking = true;
    error = null;
    try {
      if (kind === "cover") {
        await setGameCover(game.id, selectedGame.id, asset.id, asset.url);
      } else if (kind === "icon") {
        await setGameIcon(game.id, selectedGame.id, asset.id, asset.url);
      } else {
        await setGameArtwork(game.id, kind, selectedGame.id, asset.url);
      }
      onDone();
    } catch (e) {
      error = e;
      picking = false;
    }
  }

  async function handleRemove() {
    removing = true;
    error = null;
    try {
      if (kind === "cover") {
        await removeGameCover(game.id);
      } else if (kind === "icon") {
        await removeGameIcon(game.id);
      } else {
        await removeGameArtwork(game.id, kind);
      }
      onDone();
    } catch (e) {
      error = e;
      removing = false;
    }
  }

  function backToSearch() {
    stage = "search";
    selectedGame = null;
    assetOptions = [];
    error = null;
  }
</script>

<div class="picker">
  <div class="kind-tabs">
    {#each Object.keys(kindLabels) as k (k)}
      <button
        type="button"
        class="kind-tab"
        class:active={kind === k}
        onclick={() => switchKind(k as Kind)}
      >
        {kindLabels[k as Kind]}
      </button>
    {/each}
  </div>

  {#if steamOnlyHints[kind]}
    <p class="hint">{steamOnlyHints[kind]}</p>
  {/if}

  {#if currentUrl}
    <div class="current">
      <span class="tune-title">{t("artworkPicker.currentSelection")}</span>
      <div class="current-row">
        <img class="current-asset shape-{kind}" src={currentUrl} alt="" />
        <button type="button" class="ghost" disabled={removing} onclick={handleRemove}>
          {removing ? t("artworkPicker.removing") : t("common.remove")}
        </button>
      </div>
    </div>
  {/if}

  {#if stage === "search"}
    <div class="search-row">
      <input
        type="text"
        class="text-input"
        placeholder={t("artworkPicker.searchPlaceholder")}
        bind:value={query}
        onkeydown={(e) => e.key === "Enter" && handleSearch()}
      />
      <button type="button" onclick={handleSearch} disabled={loading}
        >{t("artworkPicker.searchButton")}</button
      >
    </div>

    {#if loading}
      <p class="hint">{t("artworkPicker.searching")}</p>
    {:else if error}
      <p class="error">{backendError(error)}</p>
    {:else if matches.length === 0}
      <p class="hint">{t("artworkPicker.noMatches", { query })}</p>
    {:else}
      <ul class="matches">
        {#each matches as match (match.id)}
          <li>
            <button type="button" class="match" onclick={() => selectMatch(match)}>
              <span>{match.name}</span>
              {#if match.verified}<span class="badge">{t("artworkPicker.verified")}</span>{/if}
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  {:else}
    <button type="button" class="ghost back" onclick={backToSearch}
      >{t("artworkPicker.backToSearch")}</button
    >

    {#if loading}
      <p class="hint">{t("artworkPicker.loadingKind", { kind: kindLabels[kind] })}</p>
    {:else if error}
      <p class="error">{backendError(error)}</p>
    {:else if assetOptions.length === 0}
      <p class="hint">{t("artworkPicker.noKindOptions", { kind: kindLabels[kind] })}</p>
    {:else}
      <div class="grid-options" class:landscape={kind === "wide" || kind === "hero" || kind === "logo"}>
        {#each assetOptions as asset (asset.id)}
          <button
            type="button"
            class="grid-option shape-{kind}"
            disabled={picking}
            onclick={() => pickAsset(asset)}
          >
            <img src={asset.thumb} alt="" loading="lazy" />
          </button>
        {/each}
      </div>
    {/if}
  {/if}
</div>

<style>
  .picker {
    display: flex;
    flex-direction: column;
    gap: 1em;
  }

  .kind-tabs {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4em;
  }

  .kind-tab {
    padding: 0.4em 1em;
    border-radius: 999px;
    background: var(--surface-raised);
    color: var(--text-muted);
  }

  .kind-tab.active {
    background: var(--accent);
    color: var(--on-accent, #fff);
  }

  .tune-title {
    display: block;
    font-size: 0.85em;
    color: var(--text-muted);
    margin-bottom: 0.4em;
  }

  .current-row {
    display: flex;
    align-items: center;
    gap: 0.8em;
  }

  .current-asset {
    width: 4em;
    aspect-ratio: 2 / 3;
    object-fit: cover;
    border-radius: 6px;
  }

  .current-asset.shape-icon {
    aspect-ratio: 1 / 1;
    object-fit: contain;
    background: var(--surface-raised);
  }

  .current-asset.shape-wide,
  .current-asset.shape-hero,
  .current-asset.shape-logo {
    width: 10em;
    aspect-ratio: auto;
    max-height: 5em;
    object-fit: contain;
    background: var(--surface-raised);
  }

  .search-row {
    display: flex;
    gap: 0.5em;
  }

  .text-input {
    flex: 1;
    padding: 0.5em 0.7em;
    border-radius: 8px;
    border: 1px solid var(--border);
    background: var(--surface-raised);
    color: var(--text);
    font-size: 0.9em;
  }

  .matches {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.4em;
    max-height: 40vh;
    overflow-y: auto;
  }

  .match {
    width: 100%;
    display: flex;
    justify-content: space-between;
    align-items: center;
    text-align: left;
    padding: 0.6em 0.8em;
    background: var(--surface-raised);
    border-radius: 8px;
  }

  .match:hover {
    border-color: var(--accent);
  }

  .badge {
    font-size: 0.75em;
    color: var(--text-muted);
    background: var(--bg);
    padding: 0.15em 0.6em;
    border-radius: 999px;
  }

  .back {
    align-self: flex-start;
  }

  .grid-options.landscape {
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  }

  .grid-options {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(110px, 1fr));
    gap: 0.6em;
    max-height: 50vh;
    overflow-y: auto;
  }

  .grid-option {
    /* WebKitGTK doesn't reliably combine `aspect-ratio` with a CSS Grid
       item's default stretch alignment (the box ends up taller than its
       track, bleeding into the row below) — the classic percentage-padding
       trick sizes the box from its own width instead, sidestepping that
       interaction entirely. */
    position: relative;
    display: block;
    width: 100%;
    padding: 0;
    padding-top: 150%; /* 2:3 */
    border-radius: 8px;
    overflow: hidden;
    border: 2px solid transparent;
    min-height: 0;
    min-width: 0;
  }

  .grid-option.shape-icon {
    padding-top: 100%; /* 1:1 */
    background: var(--surface-raised);
  }

  .grid-option.shape-wide {
    padding-top: 46.7%; /* 920×430 */
  }

  .grid-option.shape-hero {
    padding-top: 32.3%; /* 1920×620 */
  }

  .grid-option.shape-logo {
    padding-top: 50%;
    background: var(--surface-raised);
  }

  .grid-option:hover:not(:disabled) {
    border-color: var(--accent);
  }

  .grid-option img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .grid-option.shape-icon img,
  .grid-option.shape-logo img {
    object-fit: contain;
  }

  .hint {
    color: var(--text-muted);
  }

  .error {
    color: var(--danger);
    font-size: 0.85em;
  }
</style>
