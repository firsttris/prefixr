import { beforeEach, describe, expect, it, vi } from "vitest";
import { get } from "svelte/store";

const listeners = new Map<string, (event: { payload: unknown }) => void>();
const invoke = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => invoke(...args) }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: async (name: string, handler: (event: { payload: unknown }) => void) => {
    listeners.set(name, handler);
    return () => listeners.delete(name);
  },
}));

const { gameRunState, initGameEvents, launchGame } = await import("$lib/stores/games");

const emit = (name: string, payload: unknown) => listeners.get(name)!({ payload });
const state = (id: string) => get(gameRunState)[id];

describe("game run state", () => {
  beforeEach(async () => {
    invoke.mockReset();
    invoke.mockResolvedValue([]);
    initGameEvents();
    await vi.waitFor(() => expect(listeners.size).toBe(4));
  });

  it("follows a launch from start to exit", async () => {
    let finish!: () => void;
    invoke.mockImplementation((cmd: string) =>
      cmd === "launch_game" ? new Promise<void>((resolve) => (finish = resolve)) : Promise.resolve([]),
    );
    const launch = launchGame("a");
    expect(state("a")).toMatchObject({ initializing: true, running: false });

    emit("game-started", { id: "a", log_path: "/logs/a.log" });
    expect(state("a")).toMatchObject({ initializing: false, running: true, logPath: "/logs/a.log" });

    emit("game-exited", { id: "a", exit_code: 0 });
    finish();
    await launch;
    expect(state("a")).toMatchObject({ running: false, error: undefined });
  });

  it("doesn't start a game twice while it's starting", async () => {
    invoke.mockImplementation(() => new Promise(() => {}));
    launchGame("b");
    launchGame("b");
    expect(invoke.mock.calls.filter(([cmd]) => cmd === "launch_game")).toHaveLength(1);
  });

  it("keeps the launch error event's error over the rejection", async () => {
    invoke.mockImplementation(async (cmd: string) => {
      if (cmd !== "launch_game") return [];
      emit("game-launch-error", { id: "c", message: { code: "x" }, log_path: "/logs/c.log" });
      throw "Game exited with status 1";
    });
    await launchGame("c");
    expect(state("c")).toMatchObject({ initializing: false, error: { code: "x" }, logPath: "/logs/c.log" });
  });

  it("shows a refused launch, which sends no event", async () => {
    invoke.mockImplementation(async (cmd: string) => {
      if (cmd === "launch_game") throw { code: "game_already_running" };
      return [];
    });
    await launchGame("d");
    expect(state("d")).toMatchObject({ initializing: false, error: { code: "game_already_running" } });
  });
});
