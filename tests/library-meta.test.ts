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
    await meta.setInCategory("k", "a", true);
    expect(invoke).toHaveBeenLastCalledWith("set_in_category", { id: "k", key: "a", member: true });
    expect(get(meta.libraryMetaLoaded)).toBe(true);
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
  it("shows the umu-database's matches first, then ProtonDB's ratings", async () => {
    invoke
      .mockResolvedValueOnce({ a: { umu_id: "umu-1", protondb_tier: null } })
      .mockResolvedValueOnce({ a: { umu_id: "umu-1", protondb_tier: "gold" } });
    await meta.refreshCompatibility(["a", "b"]);
    expect(invoke.mock.calls).toEqual([
      ["get_compatibility", { keys: ["a", "b"], protondb: false }],
      ["get_compatibility", { keys: ["a", "b"], protondb: true }],
    ]);
    expect(get(meta.compatibility).a?.protondb_tier).toBe("gold");
  });

  it("only looks up games it doesn't know yet", async () => {
    invoke.mockResolvedValue({});
    await meta.refreshCompatibility(["a", "b", "c"]);
    // "a" and "b" were asked about in the test before and aren't asked again.
    expect(invoke).toHaveBeenLastCalledWith("get_compatibility", { keys: ["c"], protondb: true });

    invoke.mockClear();
    await meta.refreshCompatibility(["a", "b", "c"]);
    expect(invoke).not.toHaveBeenCalled();
  });

  it("keeps the umu badges when ProtonDB can't be reached", async () => {
    invoke
      .mockResolvedValueOnce({ y: { umu_id: "umu-5" } })
      .mockRejectedValueOnce(new Error("offline"));
    await meta.refreshCompatibility(["y"]);
    expect(get(meta.compatibility).y?.umu_id).toBe("umu-5");
  });

  it("shows nothing rather than failing when offline, and asks again later", async () => {
    invoke.mockRejectedValueOnce(new Error("offline"));
    await expect(meta.refreshCompatibility(["z"])).resolves.toBeUndefined();
    expect(get(meta.compatibility)).toEqual({});
    invoke.mockResolvedValue({ z: { umu_id: "umu-9" } });
    await meta.refreshCompatibility(["z"]);
    expect(get(meta.compatibility).z?.umu_id).toBe("umu-9");
  });
});
