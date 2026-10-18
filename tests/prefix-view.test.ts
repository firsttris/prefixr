import { describe, expect, it } from "vitest";
import {
  filterCounts,
  groupByRoot,
  listedPath,
  matchesFilter,
  matchesQuery,
  namesLabel,
  pickerFallback,
  prefixEntries,
  prefixOptions,
  revealTarget,
  samePath,
  sharedRoots,
  sortPrefixes,
  type PrefixSort,
} from "$lib/prefixView";
import type { Game, PrefixInfo, PrefixState } from "$lib/types";

const ROOT = "/mnt/games/Prefixr";

const game = (id: string, name: string, prefix_path: string) => ({ id, name, prefix_path }) as Game;

const prefixes: PrefixInfo[] = [
  { path: `${ROOT}/Zeta/prefix` },
  { path: `${ROOT}/Alpha/prefix` },
  { path: `${ROOT}/Shared/prefix` },
  { path: `${ROOT}/Leftover/prefix` },
  { path: "/home/me/Games/epic/prefix" },
  { path: "/home/me/Games/bottles/old" },
  { path: "/opt/lonely" },
];

const states: Record<string, PrefixState> = {
  [`${ROOT}/Zeta/prefix`]: "ready",
  [`${ROOT}/Alpha/prefix`]: "not_set_up",
  [`${ROOT}/Shared/prefix`]: "timed_out",
  [`${ROOT}/Leftover/prefix`]: "unreadable",
  "/home/me/Games/epic/prefix": "ready",
  "/home/me/Games/bottles/old": "missing",
  // /opt/lonely: not looked at (yet).
};

const games = [
  game("z", "Zeta Run", `${ROOT}/Zeta/prefix`),
  // Written with a trailing slash, as a CLI or an agent may have.
  game("a", "Alpha Drift", `${ROOT}/Alpha/prefix/`),
  game("s2", "Pokémon Blue", `${ROOT}/Shared/prefix`),
  game("s1", "Echo Valley", `${ROOT}/Shared/prefix`),
  game("e", "Basalt", "/home/me/Games/epic/prefix"),
];

const entries = prefixEntries(prefixes, games, states);
const byPath = (path: string) => entries.find((e) => e.path === path)!;

describe("samePath", () => {
  it("takes a path with and without a trailing slash as the same", () => {
    expect(samePath("/a/b/", "/a/b")).toBe(true);
    expect(samePath("/a/b//", "/a/b")).toBe(true);
    expect(samePath("/a/b", "/a/bc")).toBe(false);
    expect(samePath("/", "/")).toBe(true);
  });

  it("finds a listed prefix however its path is written, as the list spells it", () => {
    expect(listedPath(`${ROOT}/Alpha/prefix/`, prefixes)).toBe(`${ROOT}/Alpha/prefix`);
    expect(listedPath("/elsewhere", prefixes)).toBeNull();
  });
});

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

  it("has no state for a folder not looked at yet", () => {
    expect(byPath("/opt/lonely").state).toBeNull();
    expect(prefixEntries(prefixes, games).every((e) => e.state === null)).toBe(true);
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
      // Can't be read, or no answer in time.
      unreadable: 2,
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
  const rests = (sort: PrefixSort) => sortPrefixes(entries, sort).map((e) => e.rest);

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
  const roots = (sort: PrefixSort) =>
    groupByRoot(sortPrefixes(entries, sort)).map((g) => [g.root, g.entries.length]);

  it("orders the folders by their first entry, so the sort holds across them", () => {
    // By path: the folders by path, the one under none (/opt) last.
    expect(roots("path")).toEqual([
      ["/home/me/Games", 2],
      [ROOT, 4],
      ["", 1],
    ]);
    // By number of games: the folder of the most-shared prefix first.
    expect(roots("games")[0]).toEqual([ROOT, 4]);
    // By game: the folder of the first name (Alpha Drift) first.
    expect(roots("name")).toEqual([
      [ROOT, 4],
      ["/home/me/Games", 2],
      ["", 1],
    ]);
  });

  it("keeps the order of the entries within a folder", () => {
    const [first] = groupByRoot(sortPrefixes(entries, "games"));
    expect(first!.entries.map((e) => e.rest)).toEqual([
      "Shared/prefix",
      "Alpha/prefix",
      "Zeta/prefix",
      "Leftover/prefix",
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
    const twins: PrefixInfo[] = [{ path: "/a/x/prefix" }, { path: "/b/y/prefix" }];
    const sameName = [game("1", "Doom", "/a/x/prefix"), game("2", "Doom", "/b/y/prefix")];
    expect(prefixOptions(twins, sameName).map((o) => o.label)).toEqual([
      "/a/x/prefix",
      "/b/y/prefix",
    ]);
  });

  describe("the extra option for a value the list doesn't have", () => {
    const options = prefixOptions(prefixes, games);

    it("is there for a game's prefix removed from the list", () => {
      expect(pickerFallback("/removed/prefix", options)).toBe("/removed/prefix");
    });

    it("isn't there for the empty value: a placeholder, or the store's new prefix", () => {
      expect(pickerFallback("", options)).toBeNull();
    });

    it("isn't there for a listed prefix, with or without a trailing slash", () => {
      expect(pickerFallback(`${ROOT}/Alpha/prefix`, options)).toBeNull();
      expect(pickerFallback(`${ROOT}/Alpha/prefix/`, options)).toBeNull();
    });
  });
});

describe("revealTarget", () => {
  it("leads to the listed prefix, as the list spells it", () => {
    expect(revealTarget(`${ROOT}/Alpha/prefix/`, prefixes)).toEqual({
      listed: true,
      path: `${ROOT}/Alpha/prefix`,
    });
  });

  it("leads to a notice for a game's prefix that isn't listed any more", () => {
    expect(revealTarget("/removed/prefix", prefixes)).toEqual({
      listed: false,
      path: "/removed/prefix",
    });
  });

  it("leads nowhere without a path", () => {
    expect(revealTarget(null, prefixes)).toBeNull();
    expect(revealTarget("", prefixes)).toBeNull();
  });
});
