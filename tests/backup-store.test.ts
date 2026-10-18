import { beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => invoke(...args) }));

const reload = vi.fn();
globalThis.location = { reload } as unknown as Location;

const backup = await import("$lib/stores/backup");

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

    invoke.mockRejectedValueOnce(new Error("not a Prefixr backup"));
    await expect(backup.restoreBackup("/b/y.tar.gz")).rejects.toThrow("not a Prefixr backup");
    expect(reload).toHaveBeenCalledOnce();
  });

  it("get a dated default name", () => {
    expect(backup.defaultBackupName(new Date(2026, 9, 8))).toBe("prefixr-backup-2026-10-08.tar.gz");
  });
});
