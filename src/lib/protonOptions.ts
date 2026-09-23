// Labels for the Proton switches worth showing up front. Which switches a
// runner offers at all comes from its own `proton` script (see
// `list_proton_options`); this table only decides how the familiar ones are
// presented. Keyed by Proton's internal config name rather than the variable,
// since that stays put when a variable gets an alias or is renamed. Anything
// a runner offers that isn't listed here still shows up, under "Erweitert".
//
// Descriptions say what switching *on* does: for the `PROTON_NO_*` ones
// that means turning something off.
export interface ProtonOptionInfo {
  label: string;
  description: string;
}

export const PROTON_OPTION_INFO: Record<string, ProtonOptionInfo> = {
  hdr: {
    label: "HDR",
    description:
      "Gibt HDR aus, wenn Monitor und Desktop es unterstützen. Braucht meist zusätzlich den Wayland-Treiber oder Gamescope.",
  },
  wayland: {
    label: "Wayland-Treiber",
    description: "Nativer Wayland-Treiber statt XWayland. Kann Latenz und Skalierung verbessern.",
  },
  wined3d: {
    label: "OpenGL statt DXVK (WineD3D)",
    description: "Für alte DirectX-8/9-Spiele, die mit DXVK Grafikfehler zeigen oder nicht starten.",
  },
  dlss: {
    label: "DLSS aktualisieren",
    description: "Ersetzt die DLSS-Version des Spiels durch die neueste. Nur Nvidia.",
  },
  fsr4: {
    label: "FSR 4 aktualisieren",
    description: "Hebt FSR 3.1 im Spiel auf FSR 4 an. Nur AMD RDNA4.",
  },
  fsr4rdna3: {
    label: "FSR 4 auf RDNA3",
    description: "Wie „FSR 4 aktualisieren“, aber mit der Variante für RDNA3-Karten.",
  },
  xess: {
    label: "XeSS aktualisieren",
    description: "Ersetzt die XeSS-Version des Spiels durch die neueste.",
  },
  disablenvapi: {
    label: "NVAPI abschalten",
    description: "Schaltet DLSS/Reflex-Unterstützung ab. Hilft, wenn ein Spiel mit Nvidia-Erkennung abstürzt.",
  },
  forcenvapi: {
    label: "NVAPI erzwingen",
    description:
      "Gibt sich auch auf AMD-Karten als Nvidia-GPU mit NVAPI aus. Für Spiele, die DLSS/Reflex-Optionen sonst ausblenden.",
  },
  nontsync: {
    label: "NTSync abschalten",
    description: "Nur bei Hängern oder Abstürzen, die mit NTSync zusammenhängen.",
  },
  nofsync: {
    label: "Fsync abschalten",
    description: "Nur bei Hängern oder Abstürzen, die mit Fsync zusammenhängen.",
  },
  localshadercache: {
    label: "Eigener Shader-Cache",
    description: "Legt den Shader-Cache pro Spiel an, statt ihn mit anderen Spielen zu teilen.",
  },
  forcelgadd: {
    label: "Mehr Speicher für 32-Bit-Spiele",
    description: "Setzt das Large-Address-Aware-Flag. Hilft alten, stark gemoddeten Spielen gegen Abstürze wegen Speichermangel.",
  },
  sdlinput: {
    label: "Controller über SDL",
    description: "Liest Controller über SDL statt über Steam Input ein.",
  },
};

// Order of the curated switches in the form.
export const PROTON_OPTION_ORDER = Object.keys(PROTON_OPTION_INFO);
