import { invoke } from "@tauri-apps/api/core";
import { writable } from "svelte/store";
import type { MissingExtension } from "$lib/bindings/MissingExtension";

// In the Flatpak, the tools whose Flathub extension isn't installed (see
// `commands::flatpak_extensions`); always empty natively. Read once: an
// extension installed meanwhile needs a restart of the app anyway.
export const missingExtensions = writable<MissingExtension[]>([]);

let loaded = false;

export async function loadMissingExtensions(): Promise<void> {
  if (loaded) return;
  loaded = true;
  missingExtensions.set(await invoke<MissingExtension[]>("missing_flatpak_extensions"));
}
