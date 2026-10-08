import { createDesktopShortcut, createMenuShortcut } from "$lib/stores/games";
import { getGameCover } from "$lib/stores/steamgriddb";
import type { Game } from "$lib/types";

export type ShortcutState = "idle" | "creating" | "done" | { error: unknown };

/**
 * What a library entry shows and does the same way in either view
 * (GameCard, GameListRow): its cover, creating its shortcuts, and asking
 * before it's removed. Create it while the component initializes.
 */
export class GameItem {
  coverSrc = $state<string | null>(null);
  shortcutState = $state<ShortcutState>("idle");
  confirmingRemove = $state(false);
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
