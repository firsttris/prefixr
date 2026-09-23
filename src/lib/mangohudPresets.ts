import type { MangoHudLayout, MangoHudPosition } from "$lib/types";

// Starting points for the overlay's look — see OverlayEditor.svelte.
export interface MangoHudPreset {
  key: string;
  label: string;
  description: string;
  values: Omit<MangoHudLayout, "preset">;
}

export const PRESETS: MangoHudPreset[] = [
  {
    key: "minimal",
    label: "Minimal",
    description: "Nur die FPS-Zahl, dezent in der Ecke.",
    values: {
      position: "top-right",
      theme_color: "ffffff",
      background_alpha: 0.15,
      round_corners: true,
      show_fps: true,
      show_frametime: false,
      show_cpu: false,
      show_gpu: false,
      show_ram: false,
      show_vram: false,
      show_temps: false,
      show_gamemode: false,
      show_vkbasalt: false,
      show_hdr: false,
      show_driver: false,
      show_engine_version: false,
      show_wine: false,
      show_gpu_name: false,
      show_resolution: false,
      horizontal: false,
    },
  },
  {
    key: "standard",
    label: "Standard",
    description: "FPS, Auslastung und Temperaturen im Überblick.",
    values: {
      position: "top-left",
      theme_color: "ffffff",
      background_alpha: 0.4,
      round_corners: true,
      show_fps: true,
      show_frametime: true,
      show_cpu: true,
      show_gpu: true,
      show_ram: false,
      show_vram: false,
      show_temps: true,
      show_gamemode: false,
      show_vkbasalt: false,
      show_hdr: false,
      show_driver: false,
      show_engine_version: false,
      show_wine: false,
      show_gpu_name: false,
      show_resolution: false,
      horizontal: false,
    },
  },
  {
    key: "detailed",
    label: "Ausführlich",
    description: "Alle Werte, inklusive Status-Icons und technischer Infos.",
    values: {
      position: "top-left",
      theme_color: "00e5ff",
      background_alpha: 0.55,
      round_corners: true,
      show_fps: true,
      show_frametime: true,
      show_cpu: true,
      show_gpu: true,
      show_ram: true,
      show_vram: true,
      show_temps: true,
      show_gamemode: true,
      show_vkbasalt: true,
      show_hdr: true,
      show_driver: true,
      show_engine_version: true,
      show_wine: true,
      show_gpu_name: true,
      show_resolution: true,
      horizontal: false,
    },
  },
  {
    key: "competitive",
    label: "Wettkampf",
    description: "Groß, knallig, sonst nichts — für maximale Übersicht.",
    values: {
      position: "top-right",
      theme_color: "39ff14",
      background_alpha: 0.1,
      round_corners: false,
      show_fps: true,
      show_frametime: false,
      show_cpu: false,
      show_gpu: false,
      show_ram: false,
      show_vram: false,
      show_temps: false,
      show_gamemode: false,
      show_vkbasalt: false,
      show_hdr: false,
      show_driver: false,
      show_engine_version: false,
      show_wine: false,
      show_gpu_name: false,
      show_resolution: false,
      horizontal: false,
    },
  },
];

export const COLORS = [
  { name: "Weiß", hex: "ffffff" },
  { name: "Cyan", hex: "00e5ff" },
  { name: "Grün", hex: "39ff14" },
  { name: "Orange", hex: "ff9100" },
  { name: "Pink", hex: "ff4da6" },
];

export const POSITIONS: { value: MangoHudPosition; label: string }[] = [
  { value: "top-left", label: "Oben links" },
  { value: "top-right", label: "Oben rechts" },
  { value: "bottom-left", label: "Unten links" },
  { value: "bottom-right", label: "Unten rechts" },
];

