import { describe, expect, it } from "vitest";
import { parseNotes } from "$lib/notes";

describe("parseNotes", () => {
  it("splits notes into text, headings and lists", () => {
    const notes = [
      "Status: Läuft ✓",
      "",
      "Fixes:",
      "- WINEDLLOVERRIDES=amd_ags_x64=n",
      "  (sonst „Steam must be running“)",
      "- vcrun2022 installiert",
      "",
      "Hinweise:",
      "• Netz war gesperrt",
    ].join("\n");
    expect(parseNotes(notes)).toEqual([
      { kind: "text", text: "Status: Läuft ✓" },
      { kind: "heading", text: "Fixes" },
      {
        kind: "list",
        items: [
          "WINEDLLOVERRIDES=amd_ags_x64=n (sonst „Steam must be running“)",
          "vcrun2022 installiert",
        ],
      },
      { kind: "heading", text: "Hinweise" },
      { kind: "list", items: ["Netz war gesperrt"] },
    ]);
  });

  it("keeps plain notes as they are", () => {
    expect(parseNotes("Login: RELOADED\nPasswort 1\n\nMehr")).toEqual([
      { kind: "text", text: "Login: RELOADED\nPasswort 1" },
      { kind: "text", text: "Mehr" },
    ]);
    expect(parseNotes("")).toEqual([]);
  });

  it("continues a list after a blank line", () => {
    expect(parseNotes("- a\n\n- b")).toEqual([{ kind: "list", items: ["a", "b"] }]);
  });
});
