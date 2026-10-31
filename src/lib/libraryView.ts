import type { Game, LibraryConfig, StoreGame, StoreKind, TestStatus } from "#lib/types.ts";
import { TEST_STATUSES, gameKey, storeEntryKey, testOf } from "#lib/stores/libraryMeta.ts";

// What the library shows: every game, installed or not, filtered by the
// toolbar's choices, sorted, and split into the installed ones and the
// stores' ones still to install. Kept free of Svelte so it can be tested.

export type Entry =
  | { kind: "game"; key: string; name: string; game: Game }
  | { kind: "store"; key: string; name: string; storeGame: StoreGame };

export type SortBy =
  "name-asc" | "name-desc" | "last-played" | "recently-added" | "playtime" | "runner";

export type Source = "all" | "local" | StoreKind;

/**
 * Which games: all, the installed ones, favorites, one category (`category:<id>`), the ones a
 * test found to work, have issues or be broken (`tested:<status>`), or the
 * hidden ones.
 */
export type Collection =
  "all" | "installed" | "favorites" | "hidden" | `category:${string}` | `tested:${TestStatus}`;

export interface ViewOptions {
  query: string;
  sortBy: SortBy;
  source: Source;
  collection: Collection;
}

export function entriesOf(games: Game[], storeGames: StoreGame[]): Entry[] {
  const list: Entry[] = games.map((game) => ({
    kind: "game",
    key: gameKey(game),
    name: game.name,
    game,
  }));
  const installed = new Set(list.map((entry) => entry.key));
  for (const storeGame of storeGames) {
    const key = storeEntryKey(storeGame);
    if (storeGame.game_id === null && !installed.has(key)) {
      list.push({ kind: "store", key, name: storeGame.title, storeGame });
    }
  }
  return list;
}

export function sourceOf(entry: Entry): "local" | StoreKind {
  return entry.kind === "store" ? entry.storeGame.store : (entry.game.store?.store ?? "local");
}

function inCollection(entry: Entry, collection: Collection, meta: LibraryConfig): boolean {
  const hidden = meta.hidden.includes(entry.key);
  if (collection === "hidden") return hidden;
  if (hidden) return false;
  if (collection === "all") return true;
  if (collection === "installed") return entry.kind === "game";
  if (collection === "favorites") return meta.favorites.includes(entry.key);
  if (collection.startsWith("tested:")) {
    // A result for another setup than the game has now doesn't count, as
    // `library list --tested`.
    const test = testOf(meta, entry.key, entry.kind === "game" ? entry.game : undefined);
    return !!test?.current && `tested:${test.result.status}` === collection;
  }
  const id = collection.slice("category:".length);
  return meta.categories.some((c) => c.id === id && c.games.includes(entry.key));
}

/** Lowercase, accents and punctuation gone: "Pokémon: Blue" finds "pokemon blue". */
export function searchText(text: string): string {
  return text
    .normalize("NFD")
    .replace(/\p{M}/gu, "")
    .toLowerCase()
    .replace(/[^\p{L}\p{N}]+/gu, " ")
    .trim();
}

export function sortEntries(list: Entry[], sortBy: SortBy, meta: LibraryConfig): Entry[] {
  const byName = (a: Entry, b: Entry) => a.name.localeCompare(b.name);
  const activity = (entry: Entry) => meta.activity[entry.key];
  // Newest or most first; games without the value last, by name.
  const descending = (value: (entry: Entry) => number | null | undefined) => (a: Entry, b: Entry) =>
    (value(b) ?? -1) - (value(a) ?? -1) || byName(a, b);
  // Not-installed games have no runner yet: last when sorting by runner.
  const runnerOf = (entry: Entry) => (entry.kind === "game" ? entry.game.runner_id : "￿");
  const sorted = [...list];
  switch (sortBy) {
    case "name-asc":
      return sorted.sort(byName);
    case "name-desc":
      return sorted.sort((a, b) => byName(b, a));
    case "last-played":
      return sorted.sort(descending((e) => activity(e)?.last_played));
    case "recently-added":
      return sorted.sort(descending((e) => activity(e)?.added_at));
    case "playtime":
      return sorted.sort(descending((e) => activity(e)?.playtime_secs || null));
    case "runner":
      return sorted.sort((a, b) => runnerOf(a).localeCompare(runnerOf(b)) || byName(a, b));
  }
}

export interface LibraryView {
  installed: Entry[];
  available: Entry[];
}

export function libraryView(
  entries: Entry[],
  options: ViewOptions,
  meta: LibraryConfig,
): LibraryView {
  const query = searchText(options.query);
  const shown = entries.filter(
    (entry) =>
      (!query || searchText(entry.name).includes(query)) &&
      (options.source === "all" || sourceOf(entry) === options.source) &&
      inCollection(entry, options.collection, meta),
  );
  const sorted = sortEntries(shown, options.sortBy, meta);
  return {
    installed: sorted.filter((entry) => entry.kind === "game"),
    available: sorted.filter((entry) => entry.kind === "store"),
  };
}

/** How many games each chip of the toolbar would show, before the search. */
export function collectionCounts(
  entries: Entry[],
  meta: LibraryConfig,
  source: Source,
): Record<Collection, number> {
  const counts: Record<string, number> = {};
  const collections: Collection[] = [
    "all",
    "installed",
    "favorites",
    "hidden",
    ...meta.categories.map((c) => `category:${c.id}` as const),
    ...TEST_STATUSES.map((status) => `tested:${status}` as const),
  ];
  const fromSource = entries.filter((e) => source === "all" || sourceOf(e) === source);
  for (const collection of collections) {
    counts[collection] = fromSource.filter((e) => inCollection(e, collection, meta)).length;
  }
  return counts as Record<Collection, number>;
}

export function sourceCounts(entries: Entry[], meta: LibraryConfig): Record<string, number> {
  const counts: Record<string, number> = { all: 0 };
  for (const entry of entries) {
    if (meta.hidden.includes(entry.key)) continue;
    counts.all += 1;
    const source = sourceOf(entry);
    counts[source] = (counts[source] ?? 0) + 1;
  }
  return counts;
}

/** The installed games played most recently, newest first, for "Weiterspielen". */
export function recentlyPlayed(entries: Entry[], meta: LibraryConfig, limit: number): Entry[] {
  return entries
    .filter(
      (e) => e.kind === "game" && meta.activity[e.key]?.last_played && !meta.hidden.includes(e.key),
    )
    .sort(
      (a, b) => (meta.activity[b.key]?.last_played ?? 0) - (meta.activity[a.key]?.last_played ?? 0),
    )
    .slice(0, limit);
}
