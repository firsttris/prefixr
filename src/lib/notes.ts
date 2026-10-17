// A game's notes are plain text with two conventions that structure them
// for reading, both still readable as plain text:
//
// - a line ending in ":" is a heading, e.g. "Fixes:"
// - a line starting with "- " is a list item; an indented line after it
//   continues that item
//
// Everything else is text as written.
export type NotesBlock =
  | { kind: "heading"; text: string }
  | { kind: "list"; items: string[] }
  | { kind: "text"; text: string };

export function parseNotes(notes: string): NotesBlock[] {
  const blocks: NotesBlock[] = [];
  const last = () => blocks[blocks.length - 1];
  for (const raw of notes.split(/\r?\n/)) {
    const line = raw.trimEnd();
    const trimmed = line.trim();
    const block = last();
    if (trimmed === "") {
      // A blank line ends a paragraph; a list may go on after one.
      if (block?.kind === "text") blocks.push({ kind: "text", text: "" });
      continue;
    }
    const item = /^[-•]\s+(.*)$/.exec(trimmed);
    if (item) {
      if (block?.kind === "list") block.items.push(item[1]!);
      else blocks.push({ kind: "list", items: [item[1]!] });
    } else if (/^\s/.test(line) && block?.kind === "list") {
      block.items[block.items.length - 1] += ` ${trimmed}`;
    } else if (trimmed.endsWith(":")) {
      blocks.push({ kind: "heading", text: trimmed.slice(0, -1) });
    } else if (block?.kind === "text" && block.text !== "") {
      block.text += `\n${trimmed}`;
    } else if (block?.kind === "text") {
      block.text = trimmed;
    } else {
      blocks.push({ kind: "text", text: trimmed });
    }
  }
  return blocks.filter((b) => b.kind !== "text" || b.text !== "");
}
