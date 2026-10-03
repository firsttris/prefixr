import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

const root = new URL("..", import.meta.url).pathname;
const read = (lang: string) =>
  JSON.parse(readFileSync(join(root, `messages/${lang}.json`), "utf8")) as Record<string, unknown>;
const de = read("de");
const en = read("en");
const keys = Object.keys(de).filter((k) => k !== "$schema");

/** Placeholders a message uses; escaped braces are text. */
const placeholders = (v: unknown) =>
  [...new Set([...String(v).matchAll(/(?<!\\)\{(\w+)\}/g)].map((m) => m[1]))].sort().join(",");

const files = (dir: string, ext: RegExp): string[] =>
  readdirSync(dir, { withFileTypes: true }).flatMap((e) => {
    if (e.isDirectory()) return e.name === "paraglide" || e.name === "target" ? [] : files(join(dir, e.name), ext);
    return ext.test(e.name) ? [readFileSync(join(dir, e.name), "utf8")] : [];
  });

describe("messages/*.json", () => {
  it("has every message in both languages, none empty, with the same placeholders", () => {
    expect(Object.keys(en).sort()).toEqual(Object.keys(de).sort());
    expect(keys.filter((k) => ![de[k], en[k]].every((v) => typeof v === "string" && v.trim()))).toEqual([]);
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
