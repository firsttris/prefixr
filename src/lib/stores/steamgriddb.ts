import { invoke } from "@tauri-apps/api/core";
import { writable } from "svelte/store";
import type { SteamGridDbConfig, SteamGridDbGameMatch, SteamGridDbGrid } from "$lib/types";
import { refreshGames } from "$lib/stores/games";

export const steamGridDbConfig = writable<SteamGridDbConfig | null>(null);

export async function refreshSteamGridDbConfig(): Promise<void> {
  steamGridDbConfig.set(await invoke<SteamGridDbConfig>("get_steamgriddb_config"));
}

export async function saveSteamGridDbConfig(config: SteamGridDbConfig): Promise<void> {
  const saved = await invoke<SteamGridDbConfig>("save_steamgriddb_config", { config });
  steamGridDbConfig.set(saved);
}

export async function searchSteamGridDbGames(query: string): Promise<SteamGridDbGameMatch[]> {
  return await invoke<SteamGridDbGameMatch[]>("search_steamgriddb_games", { query });
}

export async function listSteamGridDbGrids(steamgriddbId: number): Promise<SteamGridDbGrid[]> {
  return await invoke<SteamGridDbGrid[]>("list_steamgriddb_grids", { steamgriddbId });
}

export async function setGameCover(
  gameId: string,
  steamgriddbId: number,
  coverGridId: number,
  imageUrl: string,
): Promise<void> {
  await invoke("set_game_cover", { gameId, steamgriddbId, coverGridId, imageUrl });
  await refreshGames();
}

export async function removeGameCover(gameId: string): Promise<void> {
  await invoke("remove_game_cover", { gameId });
  await refreshGames();
}

export async function getGameCover(gameId: string): Promise<string | null> {
  return await invoke<string | null>("get_game_cover", { gameId });
}
