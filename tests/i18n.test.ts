import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

const root = new URL("..", import.meta.url).pathname;
const read = (lang: string) =>
  JSON.parse(readFileSync(join(root, `messages/${lang}.json`), "utf8")) as Record<string, unknown>;
const de = read("de");
const en = read("en");
const keys = Object.keys(de).filter((k) => k !== "$schema");

/** A message's texts: itself, or each of its variants (plural forms). */
type Variant = { match: Record<string, string> };
const texts = (v: unknown): unknown[] =>
  Array.isArray(v) ? (v as Variant[]).flatMap((variant) => Object.values(variant.match ?? {})) : [v];

/** Placeholders a message uses; escaped braces are text. */
const placeholders = (v: unknown) =>
  [
    ...new Set(texts(v).flatMap((t) => [...String(t).matchAll(/(?<!\\)\{(\w+)\}/g)].map((m) => m[1]))),
  ]
    .sort()
    .join(",");

const files = (dir: string, ext: RegExp): string[] =>
  readdirSync(dir, { withFileTypes: true }).flatMap((e) => {
    if (e.isDirectory()) return e.name === "paraglide" || e.name === "target" ? [] : files(join(dir, e.name), ext);
    return ext.test(e.name) ? [readFileSync(join(dir, e.name), "utf8")] : [];
  });

describe("messages/*.json", () => {
  it("has every message in both languages, none empty, with the same placeholders", () => {
    expect(Object.keys(en).sort()).toEqual(Object.keys(de).sort());
    const filled = (v: unknown) => {
      const all = texts(v);
      return all.length > 0 && all.every((t) => typeof t === "string" && t.trim());
    };
    expect(keys.filter((k) => ![de[k], en[k]].every(filled))).toEqual([]);
    expect(keys.filter((k) => placeholders(de[k]) !== placeholders(en[k]))).toEqual([]);
  });

  it("every message is used, every used key exists", () => {
    // Message calls only count in files that import the Paraglide messages as m.
    const ui = files(join(root, "src"), /\.(svelte|ts)$/).filter((s) => s.includes("$lib/paraglide/messages"));
    const called = ui.flatMap((s) => [...s.matchAll(/\bm\.(\w+)\b/g)].map((x) => x[1]!));
    // Rust renders some texts itself (tray, native dialogs): text(locale, "key", …), or a key
    // picked first ("native_tray_hideWindow").
    const rust = files(join(root, "src-tauri/src"), /\.rs$/).flatMap((s) => [
      ...[...s.matchAll(/text\(\s*\w+,\s*"(\w+)"/g)].map((x) => x[1]!),
      ...[...s.matchAll(/"(native_\w+)"/g)].map((x) => x[1]!),
    ]);
    // Backend errors are rendered by code: backendErrors_<code> (checked against AppError in Rust).
    const used = new Set([...called, ...rust]);
    expect(keys.filter((k) => !used.has(k) && !k.startsWith("backendErrors_"))).toEqual([]);
    expect([...called, ...rust].filter((k) => !(k in de))).toEqual([]);
  });
});
