import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { BackupInfo } from "$lib/types";

// Backups of the library and its settings (see `commands::backup`).

export async function createBackup(path: string, includeSecrets: boolean): Promise<BackupInfo> {
  return await invoke<BackupInfo>("create_backup", { path, includeSecrets });
}

/** The automatic backups (daily, and before every restore), newest first. */
export async function listBackups(): Promise<BackupInfo[]> {
  return await invoke<BackupInfo[]>("list_backups");
}

/** What a backup file holds, without restoring it; fails for a file that isn't one. */
export async function inspectBackup(path: string): Promise<BackupInfo> {
  return await invoke<BackupInfo>("inspect_backup", { path });
}

/**
 * Restores a backup. Everything the window shows comes from it then, so the
 * window starts over from it, as on a fresh start.
 */
export async function restoreBackup(path: string): Promise<BackupInfo> {
  const info = await invoke<BackupInfo>("restore_backup", { path });
  try {
    sessionStorage.setItem(RESTORED_KEY, info.file_name);
  } catch {
    // Then the restored window just doesn't say so.
  }
  location.reload();
  return info;
}

const RESTORED_KEY = "prefixr-restored-backup";

/**
 * The backup just restored, once, after the reload the restore did: for the
 * Backup page to open with a message about it.
 */
export function takeRestoredBackup(): string | null {
  try {
    const name = sessionStorage.getItem(RESTORED_KEY);
    sessionStorage.removeItem(RESTORED_KEY);
    return name;
  } catch {
    return null;
  }
}

/** Calls `onChange` when an automatic backup was made (the daily one, or before a restore). */
export function listenForBackups(onChange: () => void): Promise<UnlistenFn> {
  return listen("backups-changed", onChange);
}

/** A file name for a backup made now, in local time: prefixr-backup-2026-10-18.tar.gz. */
export function defaultBackupName(now: Date): string {
  const pad = (n: number) => String(n).padStart(2, "0");
  return `prefixr-backup-${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}.tar.gz`;
}
