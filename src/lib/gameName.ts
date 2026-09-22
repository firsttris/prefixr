/**
 * Turns a raw .exe filename (as picked from disk, often
 * "SomeGameTitle.exe" with no spaces) into a human-readable guess at the
 * game's title — strips the extension and splits at word boundaries
 * (acronym/word and lowercase/uppercase transitions). Used both to seed the
 * "Name" field when adding a game and, since older/unfixed names may still
 * be sitting in an existing library, to seed the SteamGridDB search query
 * in ArtworkPicker.
 */
export function prettifyExeName(fileName: string): string {
  let name = fileName.replace(/\.exe$/i, "");
  name = name.replace(/[._-]+/g, " ");
  // Acronym followed by a new word, e.g. "DOOMThe" -> "DOOM The".
  name = name.replace(/([A-Z]+)([A-Z][a-z])/g, "$1 $2");
  // Plain camelCase/PascalCase boundary, e.g. "TheDark" -> "The Dark".
  name = name.replace(/([a-z0-9])([A-Z])/g, "$1 $2");
  return name.replace(/\s+/g, " ").trim();
}
