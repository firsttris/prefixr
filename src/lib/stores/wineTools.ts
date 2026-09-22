import { invoke } from "@tauri-apps/api/core";

export interface WineTool {
  id: string;
  label: string;
  description: string;
}

// Wine's own built-in GUI utilities — the same set PortProton, Lutris and
// Bottles expose straight from their prefix view, for when a game breaks and
// the fix means poking at the prefix itself instead of relaunching it.
export const WINE_TOOLS: WineTool[] = [
  {
    id: "winecfg",
    label: "Wine-Konfiguration",
    description: "Windows-Version, Bibliotheken-Overrides, Grafik- und Laufwerkseinstellungen.",
  },
  {
    id: "regedit",
    label: "Registrierungs-Editor",
    description: "Die Windows-Registry dieses Prefix direkt bearbeiten.",
  },
  {
    id: "cmd",
    label: "Eingabeaufforderung",
    description: "Eine Windows-Kommandozeile in diesem Prefix öffnen.",
  },
  {
    id: "winefile",
    label: "Datei-Explorer",
    description: "Wines eigener Dateimanager für das virtuelle C:-Laufwerk.",
  },
  {
    id: "taskmgr",
    label: "Task-Manager",
    description: "Laufende Windows-Prozesse in diesem Prefix anzeigen und beenden.",
  },
  {
    id: "uninstaller",
    label: "Programme deinstallieren",
    description: "Installierte Windows-Programme in diesem Prefix entfernen.",
  },
];

export async function launchWineTool(
  prefixPath: string,
  runnerId: string,
  tool: string,
): Promise<void> {
  await invoke("launch_wine_tool", { prefixPath, runnerId, tool });
}
