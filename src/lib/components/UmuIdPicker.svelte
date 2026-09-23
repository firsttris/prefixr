<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { searchUmuIds } from "$lib/stores/umu";
  import { prettifyExeName } from "$lib/gameName";
  import type { UmuMatch } from "$lib/types";

  // The game's UMU id, which umu gets as `GAMEID` so umu-protonfixes
  // applies this game's own fixes — see `Game::umu_id` in models.rs.
  let {
    umuId,
    umuStore,
    name,
    steamgriddbId = null,
    onchange,
  }: {
    umuId: string | null;
    umuStore: string | null;
    // The game's name, the default search.
    name: string;
    // The game's SteamGridDB match from the artwork picker, whose Steam app
    // is the most reliable suggestion.
    steamgriddbId?: number | null;
    onchange: (umuId: string | null, umuStore: string | null) => void;
  } = $props();

  // Store names as protonfixes' `get_store_name` spells them.
  const STORE_LABELS: Record<string, string> = {
    amazon: "Amazon",
    battlenet: "Battle.net",
    ea: "EA",
    egs: "Epic",
    gog: "GOG",
    humble: "Humble",
    itchio: "itch.io",
    steam: "Steam",
    ubisoft: "Ubisoft",
    zoomplatform: "ZOOM Platform",
  };

  function storeLabel(match: UmuMatch): string {
    if (match.store) return STORE_LABELS[match.store] ?? match.store;
    return match.source === "steam" ? "Steam" : "ohne Store";
  }

  // Mounted fresh each time the Proton tab opens, so reading the name once
  // is enough.
  const initialQuery = untrack(() => prettifyExeName(name));
  let query = $state(initialQuery);
  let matches = $state<UmuMatch[]>([]);
  let searched = $state(false);
  let loading = $state(false);
  let error = $state("");

  onMount(() => {
    // Only suggest unasked while nothing is assigned yet.
    if (!umuId && query) handleSearch();
  });

  async function handleSearch() {
    if (!query.trim()) return;
    loading = true;
    error = "";
    try {
      // The artwork match describes the game's name, not whatever else the
      // user searches for.
      matches = await searchUmuIds(query, query === initialQuery ? steamgriddbId : null);
    } catch (e) {
      matches = [];
      error = String(e);
    } finally {
      loading = false;
      searched = true;
    }
  }

  function isSelected(match: UmuMatch): boolean {
    return match.umu_id === umuId && match.store === umuStore;
  }
</script>

<section class="umu-picker">
  <div>
    <span class="label">protonfixes-Zuordnung</span>
    <p class="hint">
      Mit der passenden ID wendet umu die Fixes für genau dieses Spiel an, und Proton seine
      spielspezifischen Anpassungen. Ohne ID greifen nur allgemeine Fixes.
    </p>
  </div>

  <div class="current" class:set={!!umuId}>
    {#if umuId}
      <span>
        <code>GAMEID={umuId}</code>
        {#if umuStore}<code>STORE={umuStore}</code>{/if}
      </span>
      <button type="button" class="reset" onclick={() => onchange(null, null)}>Entfernen</button>
    {:else}
      <span class="hint">Keine ID gesetzt, umu nutzt <code>umu-default</code>.</span>
    {/if}
  </div>

  <div class="search-row">
    <input
      type="text"
      placeholder="Spielname…"
      bind:value={query}
      onkeydown={(e) => {
        if (e.key === "Enter") {
          // Would otherwise submit the whole game form.
          e.preventDefault();
          handleSearch();
        }
      }}
    />
    <button type="button" onclick={handleSearch} disabled={loading}>Suchen</button>
  </div>

  {#if loading}
    <p class="hint">Suche…</p>
  {:else if error}
    <p class="error">{error}</p>
  {:else if searched && matches.length === 0}
    <p class="hint">Keine Treffer für „{query}“. Nicht jedes Spiel braucht eigene Fixes.</p>
  {:else if matches.length > 0}
    <ul class="matches">
      {#each matches as match (`${match.umu_id}:${match.store}`)}
        <li>
          <button
            type="button"
            class="match"
            class:selected={isSelected(match)}
            aria-pressed={isSelected(match)}
            onclick={() => onchange(match.umu_id, match.store)}
          >
            <span class="title">{match.title}</span>
            <span class="badge">{storeLabel(match)}</span>
            <code>{match.umu_id}</code>
          </button>
        </li>
      {/each}
    </ul>
    <p class="hint">
      Steam-Treffer kommen über SteamGridDB, die übrigen aus der umu-database. Bei mehreren
      Stores den wählen, aus dem das Spiel stammt.
    </p>
  {/if}

  <details class="manual">
    <summary>ID von Hand eingeben</summary>
    <input
      type="text"
      placeholder="z. B. umu-1091500"
      value={umuId ?? ""}
      oninput={(e) => onchange(e.currentTarget.value || null, null)}
    />
    <p class="hint">Eine reine Steam-App-ID wird beim Speichern zu <code>umu-&lt;ID&gt;</code>.</p>
  </details>
</section>

<style>
  .umu-picker {
    display: flex;
    flex-direction: column;
    gap: 0.7em;
    padding: 0.8em 0.9em;
    border-radius: 10px;
    background: var(--surface-raised);
    border: 1px solid var(--border);
  }

  .label {
    font-weight: 600;
    color: var(--text);
  }

  .hint {
    color: var(--text-muted);
    font-size: 0.82em;
    margin: 0.25em 0 0;
  }

  .current {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1em;
    min-height: 2em;
  }

  .current code {
    font-size: 0.85em;
    margin-right: 0.6em;
  }

  .current.set code {
    color: var(--accent);
  }

  .reset {
    padding: 0.25em 0.6em;
    font-size: 0.8em;
  }

  .search-row {
    display: flex;
    gap: 0.5em;
  }

  .search-row input {
    flex: 1;
  }

  .matches {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.3em;
    max-height: 30vh;
    overflow-y: auto;
  }

  .match {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 0.6em;
    text-align: left;
    padding: 0.45em 0.7em;
    background: var(--bg);
    border: 1px solid transparent;
    border-radius: 8px;
  }

  .match:hover {
    border-color: var(--border);
  }

  .match.selected {
    border-color: var(--accent);
  }

  .title {
    flex: 1;
    min-width: 0;
    overflow-wrap: anywhere;
  }

  .badge {
    flex-shrink: 0;
    font-size: 0.75em;
    color: var(--text-muted);
    background: var(--surface-raised);
    padding: 0.15em 0.6em;
    border-radius: 999px;
  }

  .match code {
    flex-shrink: 0;
    font-size: 0.78em;
    color: var(--text-muted);
  }

  .manual summary {
    cursor: pointer;
    color: var(--text-muted);
    font-size: 0.85em;
  }

  .manual[open] {
    display: flex;
    flex-direction: column;
    gap: 0.4em;
  }

  .manual summary + input {
    margin-top: 0.4em;
  }

  .error {
    color: var(--danger);
    font-size: 0.85em;
    margin: 0;
  }
</style>
