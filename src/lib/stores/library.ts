import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { refreshGames, refreshSteamGames } from "$lib/stores/games";
import { refreshPrefixes } from "$lib/stores/prefixes";

// Listens for `config-changed`, fired when another Prefixr process (the
// command line, mostly) changed the library while the window is open; the
// backend has already taken the new config over by then. Reloads what that
// can change: the games, the prefixes, and which games are in Steam.
export function listenForConfigChanges(): Promise<UnlistenFn> {
  return listen("config-changed", async () => {
    await Promise.allSettled([refreshGames(), refreshPrefixes(), refreshSteamGames()]);
  });
}
