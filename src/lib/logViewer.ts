import { openPath, revealItemInDir } from "@tauri-apps/plugin-opener";

// Opens a log file with the system's default app for it; falls back to just
// revealing it in the file manager when there's no default app registered
// (e.g. no default handler for .txt) rather than failing silently.
export async function showLog(path: string): Promise<void> {
  try {
    await openPath(path);
  } catch {
    try {
      await revealItemInDir(path);
    } catch (e) {
      console.error("Could not open or reveal log file", e);
    }
  }
}
