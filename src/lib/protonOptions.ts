// The Proton switches worth showing up front. Which switches a runner offers
// at all comes from its own `proton` script (see `list_proton_options`);
// this list only decides how the familiar ones are presented, in this order.
// Keyed by Proton's internal config name rather than the variable, since
// that stays put when a variable gets an alias or is renamed. Anything a
// runner offers that isn't listed here still shows up, under "Erweitert".
//
// Labels/descriptions live in the i18n dictionaries under protonOptions.<key>
// (descriptions say what switching *on* does: for the `PROTON_NO_*` ones
// that means turning something off).
export const PROTON_OPTION_ORDER: string[] = [
  "hdr",
  "wayland",
  "wined3d",
  "dlss",
  "fsr4",
  "fsr4rdna3",
  "xess",
  "disablenvapi",
  "forcenvapi",
  "nontsync",
  "nofsync",
  "localshadercache",
  "forcelgadd",
  "sdlinput",
];
