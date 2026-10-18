import { beforeEach, describe, expect, it, vi } from "vitest";
import { get } from "svelte/store";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invoke(...args),
  convertFileSrc: (p: string) => p,
}));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));

const meta = await import("$lib/stores/libraryMeta");
import type { Game, LibraryConfig } from "$lib/types";

const library = (favorites: string[]): LibraryConfig => ({
  categories: [{ id: "k", name: "Kinder", games: ["a"] }],
  favorites,
  hidden: [],
  activity: {},
});

beforeEach(() => {
  invoke.mockReset();
  meta.compatibility.set({});
});

describe("the library's organization", () => {
  it("keys store games by their store entry, others by id", () => {
    expect(meta.gameKey({ id: "a", store: null } as Game)).toBe("a");
    expect(meta.gameKey({ id: "b", store: { store: "gog", app_name: "123" } } as Game)).toBe(
      "gog:123",
    );
    expect(
      meta.storeEntryKey({
        store: "epic",
        app_name: "Fn",
        title: "",
        cover_path: null,
        game_id: null,
      }),
    ).toBe("epic:Fn");
  });

  it("takes over what each change returns", async () => {
    invoke.mockResolvedValue(library(["a"]));
    await meta.setFavorite("a", true);
    expect(invoke).toHaveBeenCalledWith("set_favorite", { key: "a", favorite: true });
    expect(get(meta.libraryMeta).favorites).toEqual(["a"]);

    await meta.setHidden("a", false);
    expect(invoke).toHaveBeenLastCalledWith("set_hidden", { key: "a", hidden: false });
    await meta.createCategory("Koop");
    expect(invoke).toHaveBeenLastCalledWith("create_category", { name: "Koop" });
    await meta.renameCategory("k", "Kids");
    expect(invoke).toHaveBeenLastCalledWith("rename_category", { id: "k", name: "Kids" });
    await meta.deleteCategory("k");
    expect(invoke).toHaveBeenLastCalledWith("delete_category", { id: "k" });
    await meta.setGameCategories("a", ["k"]);
    expect(invoke).toHaveBeenLastCalledWith("set_game_categories", { key: "a", ids: ["k"] });
  });

  it("keeps the old state when a change fails", async () => {
    invoke.mockResolvedValueOnce(library(["a"]));
    await meta.refreshLibraryMeta();
    invoke.mockRejectedValueOnce(new Error("disk full"));
    await expect(meta.setFavorite("b", true)).rejects.toThrow("disk full");
    expect(get(meta.libraryMeta).favorites).toEqual(["a"]);
  });

  it("reads favorites, hidden games and categories of a game", () => {
    const config = { ...library(["a"]), hidden: ["b"] };
    expect(meta.isFavorite(config, "a")).toBe(true);
    expect(meta.isHidden(config, "a")).toBe(false);
    expect(meta.isHidden(config, "b")).toBe(true);
    expect(meta.categoryIdsOf(config, "a")).toEqual(["k"]);
    expect(meta.categoryIdsOf(config, "b")).toEqual([]);
  });

  it("formats play time", () => {
    expect(meta.formatPlaytime(30)).toBeNull();
    expect(meta.formatPlaytime(45 * 60)).toBe("45 min");
    expect(meta.formatPlaytime(3 * 3600)).toBe("3 h");
    expect(meta.formatPlaytime(3 * 3600 + 20 * 60)).toBe("3 h 20 min");
  });
});

describe("compatibility", () => {
  it("only looks up games it doesn't know yet", async () => {
    invoke.mockResolvedValueOnce({ a: { umu_id: "umu-1", protondb_tier: "gold" } });
    await meta.refreshCompatibility(["a", "b"]);
    expect(invoke).toHaveBeenCalledWith("get_compatibility", { keys: ["a", "b"] });
    expect(get(meta.compatibility).a?.protondb_tier).toBe("gold");

    // "b" was asked about and isn't known: not asked again.
    invoke.mockResolvedValueOnce({});
    await meta.refreshCompatibility(["a", "b", "c"]);
    expect(invoke).toHaveBeenLastCalledWith("get_compatibility", { keys: ["c"] });

    invoke.mockClear();
    await meta.refreshCompatibility(["a", "b", "c"]);
    expect(invoke).not.toHaveBeenCalled();
  });

  it("shows nothing rather than failing when offline, and asks again later", async () => {
    invoke.mockRejectedValueOnce(new Error("offline"));
    await expect(meta.refreshCompatibility(["z"])).resolves.toBeUndefined();
    expect(get(meta.compatibility)).toEqual({});
    invoke.mockResolvedValueOnce({ z: { umu_id: "umu-9" } });
    await meta.refreshCompatibility(["z"]);
    expect(get(meta.compatibility).z?.umu_id).toBe("umu-9");
  });
});
