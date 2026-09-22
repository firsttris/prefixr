import { invoke } from "@tauri-apps/api/core";

export interface WinetricksVerb {
  id: string;
  label: string;
  description: string;
}

// The full catalogue entry as parsed backend-side from winetricks' own
// `w_metadata` declarations — just id/category/title, no curated
// label/description (that only exists for WINETRICKS_VERBS below).
export interface WinetricksVerbMeta {
  id: string;
  category: string;
  title: string;
}

// Curated subset of winetricks' full verb catalogue — the ones that actually
// explain most "game won't start" cases, named and described in plain
// language instead of exposing winetricks' whole sprawling package list
// (same "a few friendly knobs" approach as MangoHudConfig/PerformanceConfig).
// Shown front-and-center; the full ~370-entry dlls/fonts catalogue is
// available behind a search disclosure for the rest.
export const WINETRICKS_VERBS: WinetricksVerb[] = [
  {
    id: "vcrun2022",
    label: "Visual C++ 2015-2022 Runtime",
    description: "Deckt die meisten modernen Spiele ab, die eine fehlende MSVCP/VCRUNTIME-DLL melden.",
  },
  {
    id: "vcrun2013",
    label: "Visual C++ 2013 Runtime",
    description: "Für Titel aus etwa 2013-2015, die noch die alte msvcr120/msvcp120 erwarten.",
  },
  {
    id: "vcrun2012",
    label: "Visual C++ 2012 Runtime",
    description: "Für Titel aus etwa 2012-2014 (msvcr110/msvcp110).",
  },
  {
    id: "vcrun2010",
    label: "Visual C++ 2010 Runtime",
    description: "Für Titel aus etwa 2010-2012 (msvcr100/msvcp100).",
  },
  {
    id: "vcrun2008",
    label: "Visual C++ 2008 Runtime",
    description: "Für ältere Titel aus etwa 2008-2010 (msvcr90/msvcp90).",
  },
  {
    id: "vcrun2005",
    label: "Visual C++ 2005 Runtime",
    description: "Für ältere Titel aus etwa 2005-2008 (msvcr80/msvcp80).",
  },
  {
    id: "dotnet48",
    label: ".NET Framework 4.8",
    description: "Für Launcher oder Spiele, deren Installer/UI selbst in .NET geschrieben ist.",
  },
  {
    id: "corefonts",
    label: "Windows-Standardschriften",
    description: "Arial, Times New Roman & Co. — behebt fehlende oder falsch dargestellte Schrift.",
  },
  {
    id: "d3dcompiler_47",
    label: "DirectX Shader-Compiler",
    description: "d3dcompiler_47.dll — häufig fehlend bei DirectX-9-bis-11-Titeln.",
  },
  {
    id: "d3dx9",
    label: "DirectX 9 Hilfsbibliothek (D3DX9)",
    description: "Breiter als d3dcompiler_47 — für ältere DirectX-9-Titel, die die volle D3DX9-DLL erwarten.",
  },
  {
    id: "xact",
    label: "XAudio/XACT",
    description: "Behebt fehlenden oder stummen Sound bei älteren DirectX-Spielen.",
  },
  {
    id: "quartz",
    label: "DirectShow (quartz.dll)",
    description: "Für Video-Zwischensequenzen in älteren Titeln, die sonst schwarz bleiben oder abstürzen.",
  },
  {
    id: "physx",
    label: "PhysX",
    description: "Für ältere Spiele mit Nvidia PhysX-Effekten, die die Bibliothek explizit erwarten.",
  },
  {
    id: "gdiplus",
    label: "GDI+",
    description: "Für ältere Spiele-UIs/Launcher, die auf Microsofts GDI+-Grafikbibliothek aufbauen.",
  },
  {
    id: "vb6run",
    label: "Visual Basic 6 Runtime",
    description: "Für sehr alte Spiele-Installer/-Launcher, die noch in VB6 geschrieben sind.",
  },
];

export async function installWinetricksVerbs(
  prefixPath: string,
  runnerId: string,
  verbs: string[],
): Promise<string> {
  return await invoke<string>("install_winetricks_verbs", { prefixPath, runnerId, verbs });
}

export async function listAllWinetricksVerbs(): Promise<WinetricksVerbMeta[]> {
  return await invoke<WinetricksVerbMeta[]>("list_all_winetricks_verbs");
}

export async function listInstalledWinetricksVerbs(prefixPath: string): Promise<string[]> {
  return await invoke<string[]>("list_installed_winetricks_verbs", { prefixPath });
}
