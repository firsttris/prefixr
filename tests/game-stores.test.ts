import { beforeEach, describe, expect, it, vi } from "vitest";
import { get } from "svelte/store";

const listeners = new Map<string, (event: { payload: unknown }) => void>();
const invoke = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invoke(...args),
  convertFileSrc: (path: string) => `asset://${path}`,
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: async (name: string, handler: (event: { payload: unknown }) => void) => {
    listeners.set(name, handler);
    return () => listeners.delete(name);
  },
}));

const stores = await import("$lib/stores/gameStores");
const {
  formatBytes,
  notInstalled,
  storeGameKey,
  storeCoverUrl,
  installStoreGame,
  initStoreEvents,
  storeInstalls,
  storeInstallErrors,
} = stores;

const game = (app_name: string, game_id: string | null = null) => ({
  store: "epic" as const,
  app_name,
  title: app_name,
  cover_path: null,
  game_id,
});

function answer(commands: Record<string, (args: Record<string, unknown>) => unknown>) {
  invoke.mockImplementation(async (cmd: string, args: Record<string, unknown>) => {
    if (cmd in commands) return commands[cmd]!(args);
    // The refreshes after an install.
    if (cmd === "list_games" || cmd === "list_store_games" || cmd === "list_prefixes") return [];
    throw new Error(`unexpected ${cmd}`);
  });
}

beforeEach(() => {
  invoke.mockReset();
  storeInstalls.set({});
  storeInstallErrors.set({});
});

describe("store helpers", () => {
  it("formats sizes in decimal units", () => {
    expect(formatBytes(0, "en")).toBe("0 B");
    expect(formatBytes(999, "en")).toBe("999 B");
    expect(formatBytes(19_756_849_152, "en")).toBe("19.8 GB");
    expect(formatBytes(240_000_000_000, "en")).toBe("240 GB");
    expect(formatBytes(19_756_849_152, "de")).toBe("19,8 GB");
    expect(formatBytes(null)).toBeNull();
    expect(formatBytes(-1)).toBeNull();
  });

  it("lists only the games not installed yet", () => {
    expect(notInstalled([game("A"), game("B", "id-b")]).map((g) => g.app_name)).toEqual(["A"]);
  });

  it("names the stores", () => {
    expect(stores.storeName("epic")).toBe("Epic Games");
    expect(stores.storeName("gog")).toBe("GOG");
  });

  it("keys games by store and id", () => {
    expect(storeGameKey("epic", "Fortnite")).toBe("epic:Fortnite");
  });

  it("loads covers through the asset protocol", () => {
    expect(storeCoverUrl({ ...game("A"), cover_path: "/data/artwork/store-epic-A.jpg" })).toBe(
      "asset:///data/artwork/store-epic-A.jpg",
    );
    expect(storeCoverUrl(game("A"))).toBeNull();
  });
});

describe("installing a store game", () => {
  const options = { install_dir: "/games", prefix_path: null, runner_id: "GE-Proton10-4" };

  it("shows progress while it runs and clears it after", async () => {
    let finish!: (value: unknown) => void;
    answer({ install_store_game: () => new Promise((resolve) => (finish = resolve)) });
    const done = installStoreGame("epic", "Lantern", options);
    expect(get(storeInstalls)).toEqual({ "epic:Lantern": 0 });
    finish({ id: "new" });
    expect(await done).toEqual({ id: "new" });
    expect(get(storeInstalls)).toEqual({});
    expect(invoke).toHaveBeenCalledWith("install_store_game", {
      store: "epic",
      appName: "Lantern",
      options,
    });
  });

  it("keeps a failure for the card", async () => {
    const error = { code: "store_install_failed", message: "no space", log_path: "/l.txt" };
    answer({
      install_store_game: () => {
        throw error;
      },
    });
    expect(await installStoreGame("epic", "Lantern", options)).toBeNull();
    expect(get(storeInstallErrors)).toEqual({ "epic:Lantern": error });
  });

  it("doesn't treat a cancel as a failure", async () => {
    answer({
      install_store_game: () => {
        throw { code: "store_install_cancelled" };
      },
    });
    expect(await installStoreGame("epic", "Lantern", options)).toBeNull();
    expect(get(storeInstallErrors)).toEqual({});
  });
});

describe("install events", () => {
  it("pick up installs a reloaded window didn't start, and their end", async () => {
    answer({
      list_store_installs: () => [{ store: "epic", app_name: "Ashen", percent: 42 }],
    });
    await initStoreEvents();
    await vi.waitFor(() => expect(get(storeInstalls)).toEqual({ "epic:Ashen": 42 }));

    listeners.get("store-install-progress")!({
      payload: { store: "epic", app_name: "Ashen", percent: 60 },
    });
    // Progress for an install this window doesn't know of is ignored.
    listeners.get("store-install-progress")!({
      payload: { store: "epic", app_name: "Other", percent: 10 },
    });
    expect(get(storeInstalls)).toEqual({ "epic:Ashen": 60 });

    listeners.get("store-install-finished")!({ payload: { store: "epic", app_name: "Ashen" } });
    expect(get(storeInstalls)).toEqual({});
  });
});
