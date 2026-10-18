import { describe, expect, it, vi } from "vitest";
import { get } from "svelte/store";

const listeners = new Map<string, () => Promise<void>>();
const invoke = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => invoke(...args) }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: async (name: string, handler: () => Promise<void>) => {
    listeners.set(name, handler);
    return () => listeners.delete(name);
  },
}));

const { listenForConfigChanges } = await import("$lib/stores/library");
const { games, steamGameIds } = await import("$lib/stores/games");
const { prefixes } = await import("$lib/stores/prefixes");
const { libraryMeta } = await import("$lib/stores/libraryMeta");

describe("config changes from another process", () => {
  it("reload the library", async () => {
    invoke.mockImplementation(async (cmd: string) => {
      switch (cmd) {
        case "list_games":
          return [{ id: "a", name: "Quake" }];
        case "list_prefixes":
          return [{ path: "/p" }];
        case "list_steam_games":
          return ["a"];
        case "get_library":
          return { categories: [], favorites: ["a"], hidden: [], activity: {}, tested: {} };
      }
      throw new Error(`unexpected ${cmd}`);
    });
    const unlisten = await listenForConfigChanges();

    await listeners.get("config-changed")!();

    expect(get(games)).toEqual([{ id: "a", name: "Quake" }]);
    expect(get(prefixes)).toEqual([{ path: "/p" }]);
    expect(get(steamGameIds)).toEqual(new Set(["a"]));
    expect(get(libraryMeta).favorites).toEqual(["a"]);
    unlisten();
    expect(listeners.has("config-changed")).toBe(false);
  });

  it("still reload the rest when one part fails", async () => {
    invoke.mockImplementation(async (cmd: string) => {
      if (cmd === "list_steam_games") throw new Error("no Steam");
      return cmd === "list_games" ? [{ id: "b", name: "Prey" }] : [];
    });
    await listenForConfigChanges();

    await listeners.get("config-changed")!();

    expect(get(games)).toEqual([{ id: "b", name: "Prey" }]);
  });
});
