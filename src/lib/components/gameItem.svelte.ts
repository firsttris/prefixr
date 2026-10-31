import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { createDesktopShortcut, createMenuShortcut } from "#lib/stores/games.ts";
import { getGameCover } from "#lib/stores/steamgriddb.ts";
import type { Game } from "#lib/types.ts";

export type ShortcutState = "idle" | "creating" | "done" | { error: unknown };

/**
 * What a library entry shows and does the same way in either view
 * (GameCard, GameListRow): its cover, creating its shortcuts, showing its
 * exe in the file manager, and asking before it's removed. Create it while the component initializes.
 */
export class GameItem {
  coverSrc = $state<string | null>(null);
  shortcutState = $state<ShortcutState>("idle");
  confirmingRemove = $state(false);
  // Why the file manager couldn't show the exe (moved, on a drive not
  // mounted), until the next try.
  revealError = $state<unknown>(null);
  #game: () => Game;
  #doneTimer: ReturnType<typeof setTimeout> | undefined;

  constructor(game: () => Game) {
    this.#game = game;

    // Only these primitives decide which cover to show; depending on them
    // instead of the game keeps every games refresh (a new object per game)
    // from fetching the cover again.
    const gameId = $derived(game().id);
    const coverUrl = $derived(game().cover_url);
    const coverGridId = $derived(game().cover_grid_id);

    $effect(() => {
      if (!coverUrl) {
        this.coverSrc = null;
        return;
      }
      let stale = false;
      getGameCover(gameId, coverGridId)
        .then((src) => {
          if (!stale) this.coverSrc = src;
        })
        .catch(() => {
          if (!stale) this.coverSrc = null;
        });
      return () => {
        stale = true;
      };
    });

    $effect(() => () => clearTimeout(this.#doneTimer));
  }

  /** Installed from a store: removing it uninstalls its files. */
  get fromStore(): boolean {
    return this.#game().store != null;
  }

  /** Opens the game's folder in the file manager, its exe selected. */
  async showInFileManager(): Promise<void> {
    this.revealError = null;
    try {
      await revealItemInDir(this.#game().exe_path);
    } catch (e) {
      this.revealError = e;
    }
  }

  async createShortcut(target: "desktop" | "menu"): Promise<void> {
    const id = this.#game().id;
    clearTimeout(this.#doneTimer);
    this.shortcutState = "creating";
    try {
      await (target === "desktop" ? createDesktopShortcut(id) : createMenuShortcut(id));
      this.shortcutState = "done";
      this.#doneTimer = setTimeout(() => (this.shortcutState = "idle"), 2000);
    } catch (e) {
      this.shortcutState = { error: e };
    }
  }
}
