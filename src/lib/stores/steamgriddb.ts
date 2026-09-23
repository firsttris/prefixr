import { invoke } from "@tauri-apps/api/core";
import { writable } from "svelte/store";
import type {
  ArtworkKind,
  SteamGridDbConfig,
  SteamGridDbGameMatch,
  SteamGridDbGrid,
} from "$lib/types";
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

export async function listSteamGridDbIcons(steamgriddbId: number): Promise<SteamGridDbGrid[]> {
  return await invoke<SteamGridDbGrid[]>("list_steamgriddb_icons", { steamgriddbId });
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

export async function setGameIcon(
  gameId: string,
  steamgriddbId: number,
  iconGridId: number,
  imageUrl: string,
): Promise<void> {
  await invoke("set_game_icon", { gameId, steamgriddbId, iconGridId, imageUrl });
  await refreshGames();
}

export async function removeGameIcon(gameId: string): Promise<void> {
  await invoke("remove_game_icon", { gameId });
  await refreshGames();
}

export async function getGameIcon(gameId: string): Promise<string | null> {
  return await invoke<string | null>("get_game_icon", { gameId });
}

export async function listSteamGridDbArtwork(
  steamgriddbId: number,
  kind: ArtworkKind,
): Promise<SteamGridDbGrid[]> {
  return await invoke<SteamGridDbGrid[]>("list_steamgriddb_artwork", { steamgriddbId, kind });
}

export async function setGameArtwork(
  gameId: string,
  kind: ArtworkKind,
  steamgriddbId: number,
  imageUrl: string,
): Promise<void> {
  await invoke("set_game_artwork", { gameId, kind, steamgriddbId, imageUrl });
  await refreshGames();
}

export async function removeGameArtwork(gameId: string, kind: ArtworkKind): Promise<void> {
  await invoke("remove_game_artwork", { gameId, kind });
  await refreshGames();
}
