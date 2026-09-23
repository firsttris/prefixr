import { de } from "./de";
import { en } from "./en";

export type Locale = "de" | "en";

const dictionaries = { de, en };

// Builds a union of dotted paths through a nested string dictionary, e.g.
// "nav.library" | "gameForm.title" | ... — gives t() autocomplete and
// catches typo'd/renamed keys at compile time.
type Paths<T> = T extends string
  ? never
  : {
      [K in keyof T & string]: T[K] extends string ? K : `${K}.${Paths<T[K]>}`;
    }[keyof T & string];

export type TranslationKey = Paths<typeof de>;

const STORAGE_KEY = "prefixr:locale";

function detectLocale(): Locale {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored === "de" || stored === "en") return stored;
  } catch {
    // localStorage unavailable (private mode, etc.) — fall through to
    // browser-locale detection below.
  }
  const lang = typeof navigator !== "undefined" ? navigator.language : undefined;
  if (!lang) return "de";
  return lang.toLowerCase().startsWith("de") ? "de" : "en";
}

let locale = $state<Locale>(detectLocale());

export function getLocale(): Locale {
  return locale;
}

export function setLocale(next: Locale): void {
  locale = next;
  try {
    localStorage.setItem(STORAGE_KEY, next);
  } catch {
    // Ignore — locale still applies for the rest of this session.
  }
}

function resolve(dict: typeof de, key: string): string | undefined {
  let node: unknown = dict;
  for (const part of key.split(".")) {
    if (typeof node !== "object" || node === null) return undefined;
    node = (node as Record<string, unknown>)[part];
  }
  return typeof node === "string" ? node : undefined;
}

export function t(key: TranslationKey, params?: Record<string, string | number>): string {
  const value = resolve(dictionaries[locale], key) ?? resolve(de, key) ?? key;
  if (!params) return value;
  return Object.entries(params).reduce(
    (text, [name, val]) => text.replaceAll(`{${name}}`, String(val)),
    value,
  );
}
