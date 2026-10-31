import { beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => invoke(...args) }));
const listen = vi.fn(async (name: string, handler: () => void) => {
  void name;
  void handler;
  return () => {};
});
vi.mock("@tauri-apps/api/event", () => ({
  listen: (name: string, handler: () => void) => listen(name, handler),
}));

const session = new Map<string, string>();
globalThis.sessionStorage = {
  getItem: (key: string) => session.get(key) ?? null,
  setItem: (key: string, value: string) => void session.set(key, value),
  removeItem: (key: string) => void session.delete(key),
} as Storage;

const reload = vi.fn();
globalThis.location = { reload } as unknown as Location;

const backup = await import("#lib/stores/backup.ts");

const info = { path: "/b/x.tar.gz", file_name: "x.tar.gz", created_at: 1, reason: "manual" };

beforeEach(() => {
  invoke.mockReset();
  reload.mockReset();
});

describe("backups", () => {
  it("are created, listed and inspected through the backend", async () => {
    invoke.mockResolvedValue(info);
    await expect(backup.createBackup("/b/x.tar.gz", true)).resolves.toEqual(info);
    expect(invoke).toHaveBeenLastCalledWith("create_backup", {
      path: "/b/x.tar.gz",
      includeSecrets: true,
    });
    await backup.inspectBackup("/b/x.tar.gz");
    expect(invoke).toHaveBeenLastCalledWith("inspect_backup", { path: "/b/x.tar.gz" });
    invoke.mockResolvedValue([info]);
    await expect(backup.listBackups()).resolves.toEqual([info]);
    expect(invoke).toHaveBeenLastCalledWith("list_backups");
  });

  it("start the window over after a restore, and not after a failed one", async () => {
    invoke.mockResolvedValueOnce(info);
    await backup.restoreBackup("/b/x.tar.gz");
    expect(invoke).toHaveBeenLastCalledWith("restore_backup", { path: "/b/x.tar.gz" });
    expect(reload).toHaveBeenCalledOnce();
    // The reloaded window says what was restored, once.
    expect(backup.takeRestoredBackup()).toBe("x.tar.gz");
    expect(backup.takeRestoredBackup()).toBeNull();

    invoke.mockRejectedValueOnce(new Error("not a Prefixr backup"));
    await expect(backup.restoreBackup("/b/y.tar.gz")).rejects.toThrow("not a Prefixr backup");
    expect(reload).toHaveBeenCalledOnce();
  });

  it("keep their settings in the backend", async () => {
    const settings = { automatic: true, interval_days: 3, keep: 5, dir: "/mnt/nas" };
    invoke.mockResolvedValue(settings);
    await expect(backup.getBackupConfig()).resolves.toEqual(settings);
    expect(invoke).toHaveBeenLastCalledWith("get_backup_config");
    await expect(backup.saveBackupConfig(settings)).resolves.toEqual(settings);
    expect(invoke).toHaveBeenLastCalledWith("save_backup_config", { config: settings });
  });

  it("tell the page when Prefixr made one", async () => {
    const onChange = vi.fn();
    await backup.listenForBackups(onChange);
    expect(listen).toHaveBeenCalledWith("backups-changed", onChange);
  });

  it("get a dated default name", () => {
    expect(backup.defaultBackupName(new Date(2026, 9, 8))).toBe("prefixr-backup-2026-10-08.tar.gz");
  });
});
