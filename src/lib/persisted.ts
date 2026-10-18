// Small choices of the user's kept across restarts in localStorage: the
// library's sort, filters, card size, collapsed sections. Losing them
// (private mode, cleared storage) only means the defaults again.

export function loadSetting<T>(key: string, fallback: T, valid: (value: unknown) => value is T): T {
  try {
    const raw = localStorage.getItem(key);
    if (raw === null) return fallback;
    const value: unknown = JSON.parse(raw);
    return valid(value) ? value : fallback;
  } catch {
    return fallback;
  }
}

export function saveSetting(key: string, value: unknown): void {
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch {
    // Not kept, then.
  }
}

export const isOneOf =
  <T extends string>(...options: readonly T[]) =>
  (value: unknown): value is T =>
    typeof value === "string" && (options as readonly string[]).includes(value);

export const isNumberIn =
  (min: number, max: number) =>
  (value: unknown): value is number =>
    typeof value === "number" && Number.isFinite(value) && value >= min && value <= max;

export const isStringList = (value: unknown): value is string[] =>
  Array.isArray(value) && value.every((item) => typeof item === "string");
