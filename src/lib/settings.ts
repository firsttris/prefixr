// Shared plumbing for the four settings categories (Leistung, Bild, Overlay,
// Proton). Each has one editor component used in two places: the global
// settings page, and the game dialog, where every value starts out
// inherited and can be overridden. The editors only ever see the effective
// values; in the game dialog they additionally get `OverrideHooks` to show
// which rows deviate from the global setting and to reset them.

export interface OverrideHooks<K extends string> {
  isOverridden: (key: K) => boolean;
  reset: (key: K) => void;
  // Tooltip for the reset button, naming the global value.
  resetTitle: (key: K) => string;
}

export function onOff(value: boolean | undefined): string {
  return value ? "an" : "aus";
}

// Field-by-field equality of two flat settings objects.
export function sameFields<T extends object>(a: T, b: T): boolean {
  return (Object.keys(a) as (keyof T)[]).every((key) => {
    const x = a[key];
    const y = b[key];
    if (typeof x === "number" && typeof y === "number") return Math.abs(x - y) < 1e-6;
    return x === y;
  });
}

// For settings overridden as a whole block (gamescope, vkBasalt): while both
// are switched off, the remaining values have no effect, so they don't count
// as a difference.
export function sameBlock<T extends { enabled: boolean }>(a: T, b: T): boolean {
  if (!a.enabled && !b.enabled) return true;
  return sameFields(a, b);
}
