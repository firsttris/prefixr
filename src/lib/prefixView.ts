import type { Game, PrefixInfo, PrefixState } from "$lib/types";
import { searchText } from "$lib/libraryView";

// What the prefixes page and the prefix pickers show: each prefix with the
// games in it, its path shortened to what sets it apart from the others,
// filtered and sorted. Kept free of Svelte so it can be tested.

export type PrefixFilter = "all" | "unused" | "shared" | "not_set_up" | "missing" | "unreadable";
export type PrefixSort = "name" | "path" | "games";

export interface PrefixEntry {
  path: string;
  /** null until the folders have been looked at. */
  state: PrefixState | null;
  /** The games that use it, by name. */
  games: Game[];
  /** The folder it shares with other prefixes ("" for none)... */
  root: string;
  /** ...and the rest of its path below that. */
  rest: string;
}

export interface PrefixGroup {
  root: string;
  entries: PrefixEntry[];
}

function trimSlash(path: string): string {
  return path.length > 1 ? path.replace(/\/+$/, "") : path;
}

/**
 * Whether two paths name the same folder as the backend sees it: Rust
 * compares paths by their parts, so `/a/b/` is `/a/b`. A game's prefix may
 * be written either way (a CLI or an agent wrote it).
 */
export function samePath(a: string, b: string): boolean {
  return trimSlash(a) === trimSlash(b);
}

/** The listed prefix a path names, as the list spells it; null if none. */
export function listedPath(path: string, prefixes: PrefixInfo[]): string | null {
  return prefixes.find((p) => samePath(p.path, path))?.path ?? null;
}

/**
 * For each path, the deepest folder above it that holds another of the
 * paths too ("" when only "/" does). One agent's
 * `/mnt/games/Prefixr/<game>/prefix` all get `/mnt/games/Prefixr`, so the
 * page can say that once and show just `<game>/prefix` below it.
 */
export function sharedRoots(paths: string[]): Map<string, string> {
  const ancestors = (path: string) => {
    const parts = trimSlash(path).split("/");
    const list: string[] = [];
    for (let i = 2; i < parts.length; i++) list.push(parts.slice(0, i).join("/"));
    return list;
  };
  const counts = new Map<string, number>();
  for (const path of new Set(paths)) {
    for (const dir of ancestors(path)) counts.set(dir, (counts.get(dir) ?? 0) + 1);
  }
  const roots = new Map<string, string>();
  for (const path of paths) {
    const shared = ancestors(path).filter((dir) => (counts.get(dir) ?? 0) > 1);
    roots.set(path, shared.at(-1) ?? "");
  }
  // A folder that's the root of one path only (the others have deeper
  // ones) would be a heading over a single row: that one keeps its path.
  const uses = new Map<string, number>();
  for (const root of roots.values()) uses.set(root, (uses.get(root) ?? 0) + 1);
  for (const [path, root] of roots) {
    if ((uses.get(root) ?? 0) < 2) roots.set(path, "");
  }
  return roots;
}

export function prefixEntries(
  prefixes: PrefixInfo[],
  games: Game[],
  states: Record<string, PrefixState> = {},
): PrefixEntry[] {
  // Keyed as samePath compares.
  const byPath = new Map<string, Game[]>();
  for (const game of games) {
    const key = trimSlash(game.prefix_path);
    byPath.set(key, [...(byPath.get(key) ?? []), game]);
  }
  const roots = sharedRoots(prefixes.map((p) => p.path));
  return prefixes.map((prefix) => {
    const root = roots.get(prefix.path) ?? "";
    const path = trimSlash(prefix.path);
    return {
      path: prefix.path,
      state: states[prefix.path] ?? null,
      games: (byPath.get(path) ?? []).sort((a, b) => a.name.localeCompare(b.name)),
      root,
      rest: root ? path.slice(root.length + 1) : path,
    };
  });
}

export function matchesFilter(entry: PrefixEntry, filter: PrefixFilter): boolean {
  switch (filter) {
    case "all":
      return true;
    case "unused":
      return entry.games.length === 0;
    case "shared":
      return entry.games.length > 1;
    // Both are folders that couldn't be looked into.
    case "unreadable":
      return entry.state === "unreadable" || entry.state === "timed_out";
    default:
      return entry.state === filter;
  }
}

export function filterCounts(entries: PrefixEntry[]): Record<PrefixFilter, number> {
  const filters: PrefixFilter[] = [
    "all",
    "unused",
    "shared",
    "not_set_up",
    "missing",
    "unreadable",
  ];
  return Object.fromEntries(
    filters.map((filter) => [filter, entries.filter((e) => matchesFilter(e, filter)).length]),
  ) as Record<PrefixFilter, number>;
}

/** By path or by the name of any game in it. */
export function matchesQuery(entry: PrefixEntry, query: string): boolean {
  const wanted = searchText(query);
  if (!wanted) return true;
  return (
    searchText(entry.path).includes(wanted) ||
    entry.games.some((game) => searchText(game.name).includes(wanted))
  );
}

export function sortPrefixes(entries: PrefixEntry[], sort: PrefixSort): PrefixEntry[] {
  const byPath = (a: PrefixEntry, b: PrefixEntry) => a.path.localeCompare(b.path);
  // Unused ones have no name to go by, so they come last, by path.
  const byName = (a: PrefixEntry, b: PrefixEntry) => {
    const [nameA, nameB] = [a.games[0]?.name, b.games[0]?.name];
    if (nameA === undefined || nameB === undefined) {
      return Number(nameA === undefined) - Number(nameB === undefined) || byPath(a, b);
    }
    return nameA.localeCompare(nameB) || byPath(a, b);
  };
  const sorted = [...entries];
  switch (sort) {
    case "path":
      return sorted.sort(byPath);
    case "games":
      return sorted.sort((a, b) => b.games.length - a.games.length || byName(a, b));
    default:
      return sorted.sort(byName);
  }
}

/**
 * The sorted entries under each shared folder, keeping their order: the
 * folders come in the order of their first entry, so the chosen sort holds
 * across them too (by number of games, the folder with the most-shared
 * prefix comes first; by path, they're by path).
 */
export function groupByRoot(entries: PrefixEntry[]): PrefixGroup[] {
  const groups = new Map<string, PrefixEntry[]>();
  for (const entry of entries) {
    groups.set(entry.root, [...(groups.get(entry.root) ?? []), entry]);
  }
  return [...groups.entries()].map(([root, list]) => ({ root, entries: list }));
}

export interface PrefixOption {
  path: string;
  label: string;
  used: boolean;
}

/** "A, B +3": the first few names, and how many more. */
export function namesLabel(names: string[], shown = 2): string {
  const more = names.length - shown;
  return names.slice(0, shown).join(", ") + (more > 0 ? ` +${more}` : "");
}

/**
 * A prefix picker's options: one in use named after its games, an unused
 * one by its shortened path; each sorted by that, so typing a game's name
 * in the open list jumps to its prefix. Two that would read the same get
 * their full paths.
 */
export function prefixOptions(prefixes: PrefixInfo[], games: Game[]): PrefixOption[] {
  const options = prefixEntries(prefixes, games).map((entry) => ({
    path: entry.path,
    label: entry.games.length
      ? namesLabel(entry.games.map((g) => g.name))
      : entry.root
        ? `…/${entry.rest}`
        : entry.path,
    used: entry.games.length > 0,
  }));
  const seen = new Map<string, number>();
  for (const option of options) seen.set(option.label, (seen.get(option.label) ?? 0) + 1);
  for (const option of options) {
    if ((seen.get(option.label) ?? 0) > 1) option.label = option.path;
  }
  return options.sort((a, b) => a.label.localeCompare(b.label));
}

/**
 * The value a picker shows as an option of its own besides the list: a
 * game's prefix that was removed from the list is still the game's. None
 * for the empty value (a placeholder, or "new prefix") or for one the list
 * has, written with or without a trailing slash.
 */
export function pickerFallback(value: string, options: PrefixOption[]): string | null {
  if (!value || options.some((o) => samePath(o.path, value))) return null;
  return value;
}

export type RevealTarget = { listed: true; path: string } | { listed: false; path: string };

/**
 * Where "Show in prefixes" leads: to the listed prefix (as the list spells
 * it), or, for a game's prefix that was removed from the list, to a notice
 * that offers to add it again. Nothing for no path.
 */
export function revealTarget(path: string | null, prefixes: PrefixInfo[]): RevealTarget | null {
  if (!path) return null;
  const listed = listedPath(path, prefixes);
  return listed ? { listed: true, path: listed } : { listed: false, path };
}
