/**
 * The interface language. The texts live in messages/{de,en}.json and are compiled by Paraglide
 * to src/lib/paraglide: components call them directly, with named inputs where a text has them.
 * The language is a reactive $state: Paraglide asks for it on every call, so switching re-renders
 * every text without a reload. It is also handed to Rust (set_ui_locale, see +layout.svelte) for
 * the few texts Rust renders itself from the same files (src-tauri/src/locale.rs).
 */
import * as m from "$lib/paraglide/messages";
import { overwriteGetLocale } from "$lib/paraglide/runtime";

export type Locale = "de" | "en";

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
  if (!lang) return "en";
  return lang.toLowerCase().startsWith("de") ? "de" : "en";
}

let locale = $state<Locale>(detectLocale());

// Paraglide's m.*() ask getLocale(): reading the $state here makes every text reactive.
overwriteGetLocale(() => locale);

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

type Inputs = Record<string, string | number>;
const messages = m as unknown as Record<string, ((inputs?: Inputs) => string) | undefined>;

/** A message picked by a runtime id (tool, option, preset …); unknown ids come back as they are. */
export function pickMsg(map: Record<string, () => string>, id: string): string {
  return map[id]?.() ?? id;
}

// A Tauri command's Err value: either a plain string (a handful of internal
// helpers that never got a structured code — see AppError::Other in
// src-tauri/src/error.rs) or `{ code, ...params }` for anything with a
// `backendErrors_<code>` message, translated the same way as any other UI
// text. Every catch block that shows a command's error uses this.
export function backendError(e: unknown): string {
  if (e === null || e === undefined || e === "") return "";
  if (typeof e === "string") return e;
  if (e && typeof e === "object" && "code" in e && typeof e.code === "string") {
    const { code, ...params } = e as { code: string } & Inputs;
    const message = messages[`backendErrors_${code}`];
    // A code without a message (e.g. a newer backend than this frontend knows
    // about): better to show the error's own data than nothing.
    return message ? message(params) : JSON.stringify(e);
  }
  return String(e);
}
