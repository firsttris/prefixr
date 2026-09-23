import { invoke } from "@tauri-apps/api/core";

export interface WineTool {
  id: string;
}

// Wine's own built-in GUI utilities — the same set PortProton, Lutris and
// Bottles expose straight from their prefix view, for when a game breaks and
// the fix means poking at the prefix itself instead of relaunching it.
// Labels/descriptions live in the i18n dictionaries under wineTools.<id>.
export const WINE_TOOLS: WineTool[] = [
  { id: "winecfg" },
  { id: "regedit" },
  { id: "cmd" },
  { id: "winefile" },
  { id: "taskmgr" },
  { id: "uninstaller" },
];

export async function launchWineTool(
  prefixPath: string,
  runnerId: string,
  tool: string,
): Promise<void> {
  await invoke("launch_wine_tool", { prefixPath, runnerId, tool });
}
