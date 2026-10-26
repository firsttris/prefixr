import { describe, expect, it, vi } from "vitest";

const reveal = vi.fn();
vi.mock("@tauri-apps/plugin-opener", () => ({
  revealItemsInDir: (...args: unknown[]) => reveal(...args),
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn(), convertFileSrc: (p: string) => p }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));

const { GameItem } = await import("$lib/components/gameItem.svelte");
import type { Game } from "$lib/types";

const game = { id: "a", exe_path: "/games/Quake/quake.exe", cover_url: null } as Game;

describe("showing a game in the file manager", () => {
  it("shows its exe, and why not when that fails", async () => {
    // Under vitest Svelte runs as on a server, where the cover's `$effect`
    // doesn't run: the item can be made outside a component.
    const item = new GameItem(() => game);
    reveal.mockResolvedValueOnce(undefined);
    await item.showInFileManager();
    expect(reveal).toHaveBeenCalledWith("/games/Quake/quake.exe");
    expect(item.revealError).toBeNull();

    reveal.mockRejectedValueOnce("No such file");
    await item.showInFileManager();
    expect(item.revealError).toBe("No such file");

    // A later try that works clears the message.
    reveal.mockResolvedValueOnce(undefined);
    await item.showInFileManager();
    expect(item.revealError).toBeNull();
  });
});
