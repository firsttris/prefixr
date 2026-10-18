import { describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn(), convertFileSrc: (p: string) => p }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));

const { entriesOf, libraryView, collectionCounts, sourceCounts, recentlyPlayed, searchText } =
  await import("$lib/libraryView");
import type { Game, LibraryConfig, StoreGame } from "$lib/types";

const game = (id: string, name: string, extra: Partial<Game> = {}) =>
  ({ id, name, runner_id: "wine", store: null, ...extra }) as Game;
const storeGame = (app: string, title: string, gameId: string | null = null): StoreGame => ({
  store: "epic",
  app_name: app,
  title,
  cover_path: null,
  game_id: gameId,
});

const games = [
  game("a", "Quake"),
  game("b", "Doom", { store: { store: "epic", app_name: "doom" } as Game["store"] }),
];
const store = [
  storeGame("doom", "Doom", "b"),
  storeGame("pokemon", "Pokémon: Blue"),
  storeGame("free", "Giveaway"),
];

const meta: LibraryConfig = {
  categories: [{ id: "k", name: "Kinder", games: ["epic:pokemon", "a"] }],
  favorites: ["epic:doom"],
  hidden: ["epic:free"],
  activity: {
    a: { added_at: 10, last_played: 300, playtime_secs: 60 },
    "epic:doom": { added_at: 20, last_played: 500, playtime_secs: 7200 },
  },
  tested: {},
};

const view = (options: Partial<Parameters<typeof libraryView>[1]>) =>
  libraryView(
    entriesOf(games, store),
    { query: "", sortBy: "name-asc", source: "all", collection: "all", ...options },
    meta,
  );
const names = (entries: { name: string }[]) => entries.map((e) => e.name);

describe("the library view", () => {
  it("lists installed store games once, by their store key", () => {
    const entries = entriesOf(games, store);
    expect(entries.map((e) => e.key)).toEqual(["a", "epic:doom", "epic:pokemon", "epic:free"]);
  });

  it("splits installed from installable and leaves hidden games out", () => {
    const { installed, available } = view({});
    expect(names(installed)).toEqual(["Doom", "Quake"]);
    expect(names(available)).toEqual(["Pokémon: Blue"]);
    expect(names(view({ collection: "hidden" }).available)).toEqual(["Giveaway"]);
  });

  it("filters by favorites, category, source and an accent-blind search", () => {
    expect(names(view({ collection: "favorites" }).installed)).toEqual(["Doom"]);
    const kids = view({ collection: "category:k" });
    expect([...names(kids.installed), ...names(kids.available)]).toEqual([
      "Quake",
      "Pokémon: Blue",
    ]);
    expect(names(view({ source: "local" }).installed)).toEqual(["Quake"]);
    expect(names(view({ query: "pokemon blue" }).available)).toEqual(["Pokémon: Blue"]);
    expect(searchText("DOOM: Eternal")).toBe("doom eternal");
  });

  it("sorts by play activity, unplayed games last", () => {
    expect(names(view({ sortBy: "last-played" }).installed)).toEqual(["Doom", "Quake"]);
    expect(names(view({ sortBy: "playtime" }).installed)).toEqual(["Doom", "Quake"]);
    expect(names(view({ sortBy: "recently-added" }).installed)).toEqual(["Doom", "Quake"]);
    expect(names(view({ sortBy: "name-desc" }).installed)).toEqual(["Quake", "Doom"]);
  });

  it("counts what each chip would show", () => {
    const entries = entriesOf(games, store);
    const counts = collectionCounts(entries, meta, "all");
    expect(counts.all).toBe(3);
    expect(counts.favorites).toBe(1);
    expect(counts.hidden).toBe(1);
    expect(counts["category:k"]).toBe(2);
    expect(sourceCounts(entries, meta)).toEqual({ all: 3, local: 1, epic: 2 });
  });

  it("offers the most recently played games to continue", () => {
    expect(names(recentlyPlayed(entriesOf(games, store), meta, 5))).toEqual(["Doom", "Quake"]);
  });

  it("filters by test result, one for another runner counting as untested", () => {
    const quake = games[0];
    const tested: LibraryConfig = {
      ...meta,
      tested: {
        a: { status: "works", note: "", tested_at: 1, runner_id: "wine", exe_path: "/q.exe" },
        // Doom was tested with another runner than it has now.
        "epic:doom": {
          status: "broken",
          note: "",
          tested_at: 1,
          runner_id: "GE-Proton10-7",
          exe_path: "/d.exe",
        },
      },
    };
    games[0] = { ...quake, exe_path: "/q.exe" };
    games[1] = { ...games[1], exe_path: "/d.exe" };
    const entries = entriesOf(games, store);
    const shown = (collection: Parameters<typeof libraryView>[1]["collection"]) =>
      names(
        libraryView(entries, { query: "", sortBy: "name-asc", source: "all", collection }, tested)
          .installed,
      );
    expect(shown("tested:works")).toEqual(["Quake"]);
    expect(shown("tested:broken")).toEqual([]);
    const counts = collectionCounts(entries, tested, "all");
    expect(counts["tested:works"]).toBe(1);
    expect(counts["tested:broken"]).toBe(0);
    expect(counts["tested:issues"]).toBe(0);
    games[0] = quake;
  });
});
