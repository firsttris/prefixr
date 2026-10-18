import { describe, expect, it } from "vitest";
import {
  filterCounts,
  groupByRoot,
  matchesFilter,
  matchesQuery,
  namesLabel,
  prefixEntries,
  prefixOptions,
  sharedRoots,
  sortPrefixes,
} from "$lib/prefixView";
import type { Game, PrefixListing } from "$lib/types";

const ROOT = "/mnt/games/Prefixr";

const game = (id: string, name: string, prefix_path: string) => ({ id, name, prefix_path }) as Game;

const prefixes: PrefixListing[] = [
  { path: `${ROOT}/Zeta/prefix`, state: "ready" },
  { path: `${ROOT}/Alpha/prefix`, state: "not_set_up" },
  { path: `${ROOT}/Shared/prefix`, state: "ready" },
  { path: `${ROOT}/Leftover/prefix`, state: "ready" },
  { path: "/home/me/Games/epic/prefix", state: "ready" },
  { path: "/home/me/Games/bottles/old", state: "missing" },
  { path: "/opt/lonely", state: "ready" },
];

const games = [
  game("z", "Zeta Run", `${ROOT}/Zeta/prefix`),
  game("a", "Alpha Drift", `${ROOT}/Alpha/prefix/`),
  game("s2", "Pokémon Blue", `${ROOT}/Shared/prefix`),
  game("s1", "Echo Valley", `${ROOT}/Shared/prefix`),
  game("e", "Basalt", "/home/me/Games/epic/prefix"),
];

const entries = prefixEntries(prefixes, games);
const byPath = (path: string) => entries.find((e) => e.path === path)!;

describe("sharedRoots", () => {
  it("gives each path the deepest folder it shares with another one", () => {
    const roots = sharedRoots(prefixes.map((p) => p.path));
    expect(roots.get(`${ROOT}/Zeta/prefix`)).toBe(ROOT);
    expect(roots.get("/home/me/Games/epic/prefix")).toBe("/home/me/Games");
    // Only "/" in common with the rest: none.
    expect(roots.get("/opt/lonely")).toBe("");
  });

  it("finds none for a single path", () => {
    expect(sharedRoots(["/a/b/c"]).get("/a/b/c")).toBe("");
  });

  it("leaves a path its own when its folder would hold it alone", () => {
    // /home/me holds .local too, but only as the root of that one path.
    const roots = sharedRoots(["/home/me/Games/a", "/home/me/Games/b", "/home/me/.local/x"]);
    expect(roots.get("/home/me/Games/a")).toBe("/home/me/Games");
    expect(roots.get("/home/me/.local/x")).toBe("");
  });
});

describe("prefixEntries", () => {
  it("names a prefix's games, sorted, and shortens its path below the shared folder", () => {
    const shared = byPath(`${ROOT}/Shared/prefix`);
    expect(shared.games.map((g) => g.name)).toEqual(["Echo Valley", "Pokémon Blue"]);
    expect(shared.root).toBe(ROOT);
    expect(shared.rest).toBe("Shared/prefix");
    expect(byPath("/opt/lonely").rest).toBe("/opt/lonely");
  });

  it("matches a game's prefix path despite a trailing slash", () => {
    expect(byPath(`${ROOT}/Alpha/prefix`).games.map((g) => g.id)).toEqual(["a"]);
  });
});

describe("filters and search", () => {
  it("counts and filters by use and by state", () => {
    expect(filterCounts(entries)).toEqual({
      all: 7,
      unused: 3,
      shared: 1,
      not_set_up: 1,
      missing: 1,
    });
    const unused = entries.filter((e) => matchesFilter(e, "unused")).map((e) => e.rest);
    expect(unused).toEqual(["Leftover/prefix", "bottles/old", "/opt/lonely"]);
  });

  it("finds a prefix by path or by a game's name, ignoring case and accents", () => {
    const find = (query: string) =>
      entries.filter((e) => matchesQuery(e, query)).map((e) => e.rest);
    expect(find("pokemon")).toEqual(["Shared/prefix"]);
    expect(find("GAMES/EPIC")).toEqual(["epic/prefix"]);
    expect(find("  ")).toHaveLength(7);
    expect(find("nothing like it")).toEqual([]);
  });
});

describe("sortPrefixes", () => {
  const rests = (sort: "name" | "path" | "games") => sortPrefixes(entries, sort).map((e) => e.rest);

  it("sorts by the first game's name, unused ones last by path", () => {
    expect(rests("name")).toEqual([
      "Alpha/prefix",
      "epic/prefix",
      "Shared/prefix",
      "Zeta/prefix",
      "bottles/old",
      "Leftover/prefix",
      "/opt/lonely",
    ]);
  });

  it("sorts by path, or by how many games use it", () => {
    expect(rests("path")[0]).toBe("bottles/old");
    expect(rests("games").slice(0, 2)).toEqual(["Shared/prefix", "Alpha/prefix"]);
  });
});

describe("groupByRoot", () => {
  it("groups by shared folder, the ones under none last", () => {
    const groups = groupByRoot(sortPrefixes(entries, "name"));
    expect(groups.map((g) => [g.root, g.entries.length])).toEqual([
      ["/home/me/Games", 2],
      [ROOT, 4],
      ["", 1],
    ]);
  });
});

describe("prefix picker options", () => {
  it("shortens a list of names", () => {
    expect(namesLabel(["A"])).toBe("A");
    expect(namesLabel(["A", "B", "C", "D"])).toBe("A, B +2");
  });

  it("names used prefixes after their games and unused ones by path, sorted", () => {
    expect(prefixOptions(prefixes, games)).toEqual([
      { path: "/home/me/Games/bottles/old", label: "…/bottles/old", used: false },
      { path: `${ROOT}/Leftover/prefix`, label: "…/Leftover/prefix", used: false },
      { path: "/opt/lonely", label: "/opt/lonely", used: false },
      { path: `${ROOT}/Alpha/prefix`, label: "Alpha Drift", used: true },
      { path: "/home/me/Games/epic/prefix", label: "Basalt", used: true },
      { path: `${ROOT}/Shared/prefix`, label: "Echo Valley, Pokémon Blue", used: true },
      { path: `${ROOT}/Zeta/prefix`, label: "Zeta Run", used: true },
    ]);
  });

  it("falls back to the full path for two that would read the same", () => {
    const twins: PrefixListing[] = [
      { path: "/a/x/prefix", state: "ready" },
      { path: "/b/y/prefix", state: "ready" },
    ];
    const sameName = [game("1", "Doom", "/a/x/prefix"), game("2", "Doom", "/b/y/prefix")];
    expect(prefixOptions(twins, sameName).map((o) => o.label)).toEqual([
      "/a/x/prefix",
      "/b/y/prefix",
    ]);
  });
});
