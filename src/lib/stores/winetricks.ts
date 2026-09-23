import { invoke } from "@tauri-apps/api/core";

export interface WinetricksVerb {
  id: string;
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
// Labels/descriptions live in the i18n dictionaries under winetricksVerbs.<id>.
export const WINETRICKS_VERBS: WinetricksVerb[] = [
  { id: "vcrun2022" },
  { id: "vcrun2013" },
  { id: "vcrun2012" },
  { id: "vcrun2010" },
  { id: "vcrun2008" },
  { id: "vcrun2005" },
  { id: "dotnet48" },
  { id: "corefonts" },
  { id: "d3dcompiler_47" },
  { id: "d3dx9" },
  { id: "xact" },
  { id: "quartz" },
  { id: "physx" },
  { id: "gdiplus" },
  { id: "vb6run" },
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
