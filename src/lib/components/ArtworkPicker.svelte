<script lang="ts">
  import { onMount, untrack } from "svelte";
  import type { Game, SteamGridDbGameMatch, SteamGridDbGrid } from "$lib/types";
  import { searchSteamGridDbGames, listSteamGridDbGrids, setGameCover, removeGameCover } from "$lib/stores/steamgriddb";
  import { prettifyExeName } from "$lib/gameName";

  let { game, onDone }: { game: Game; onDone: () => void } = $props();

  // `game` is only read here to seed this picker's initial state — a fresh
  // instance is mounted per game (see +page.svelte), so a one-time snapshot
  // is intentional, not a missed reactivity dependency.
  const initial = untrack(() => ({
    stage: (game.steamgriddb_id ? "grids" : "search") as "search" | "grids",
    // Cleans up leftover ".exe"/camelCase artifacts in names stored before
    // GameForm started prettifying them — a no-op on an already-clean name.
    query: prettifyExeName(game.name),
    selectedGame: game.steamgriddb_id
      ? { id: game.steamgriddb_id, name: game.name, verified: false }
      : null,
  }));

  let stage = $state<"search" | "grids">(initial.stage);
  let query = $state(initial.query);
  let matches = $state<SteamGridDbGameMatch[]>([]);
  let selectedGame = $state<SteamGridDbGameMatch | null>(initial.selectedGame);
  let grids = $state<SteamGridDbGrid[]>([]);
  let loading = $state(false);
  let error = $state("");
  let picking = $state(false);
  let removing = $state(false);

  onMount(async () => {
    if (stage === "grids" && selectedGame) {
      await loadGrids(selectedGame.id);
    } else {
      await handleSearch();
    }
  });

  async function handleSearch() {
    if (!query.trim()) return;
    loading = true;
    error = "";
    matches = [];
    try {
      matches = await searchSteamGridDbGames(query);
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function selectMatch(match: SteamGridDbGameMatch) {
    selectedGame = match;
    stage = "grids";
    await loadGrids(match.id);
  }

  async function loadGrids(steamgriddbId: number) {
    loading = true;
    error = "";
    grids = [];
    try {
      grids = await listSteamGridDbGrids(steamgriddbId);
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function pickGrid(grid: SteamGridDbGrid) {
    if (!selectedGame) return;
    picking = true;
    error = "";
    try {
      await setGameCover(game.id, selectedGame.id, grid.id, grid.url);
      onDone();
    } catch (e) {
      error = String(e);
      picking = false;
    }
  }

  async function handleRemove() {
    removing = true;
    error = "";
    try {
      await removeGameCover(game.id);
      onDone();
    } catch (e) {
      error = String(e);
      removing = false;
    }
  }

  function backToSearch() {
    stage = "search";
    selectedGame = null;
    grids = [];
    error = "";
  }
</script>

<div class="picker">
  {#if game.cover_url}
    <div class="current">
      <span class="tune-title">Aktuelles Cover</span>
      <div class="current-row">
        <img class="current-cover" src={game.cover_url} alt="" />
        <button type="button" class="ghost" disabled={removing} onclick={handleRemove}>
          {removing ? "Wird entfernt…" : "Entfernen"}
        </button>
      </div>
    </div>
  {/if}

  {#if stage === "search"}
    <div class="search-row">
      <input
        type="text"
        class="text-input"
        placeholder="Spielname…"
        bind:value={query}
        onkeydown={(e) => e.key === "Enter" && handleSearch()}
      />
      <button type="button" onclick={handleSearch} disabled={loading}>Suchen</button>
    </div>

    {#if loading}
      <p class="hint">Suche…</p>
    {:else if error}
      <p class="error">{error}</p>
    {:else if matches.length === 0}
      <p class="hint">Keine Treffer für „{query}".</p>
    {:else}
      <ul class="matches">
        {#each matches as match (match.id)}
          <li>
            <button type="button" class="match" onclick={() => selectMatch(match)}>
              <span>{match.name}</span>
              {#if match.verified}<span class="badge">verifiziert</span>{/if}
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  {:else}
    <button type="button" class="ghost back" onclick={backToSearch}>← Andere Suche</button>

    {#if loading}
      <p class="hint">Lade Cover-Vorschläge…</p>
    {:else if error}
      <p class="error">{error}</p>
    {:else if grids.length === 0}
      <p class="hint">Keine Cover gefunden.</p>
    {:else}
      <div class="grid-options">
        {#each grids as grid (grid.id)}
          <button type="button" class="grid-option" disabled={picking} onclick={() => pickGrid(grid)}>
            <img src={grid.thumb} alt="" loading="lazy" />
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

  .current-cover {
    width: 4em;
    aspect-ratio: 2 / 3;
    object-fit: cover;
    border-radius: 6px;
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

  .hint {
    color: var(--text-muted);
  }

  .error {
    color: var(--danger);
    font-size: 0.85em;
  }
</style>
