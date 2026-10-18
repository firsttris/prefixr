<!--
  Backing up and restoring the library: a backup to a file of the user's,
  restoring one from a file, and the automatic backups Prefixr makes itself
  (every few days when turned on, and before every restore) to restore with
  one click.
-->
<script lang="ts">
  import * as m from "$lib/paraglide/messages";
  import { onMount } from "svelte";
  import { open, save } from "@tauri-apps/plugin-dialog";
  import type { BackupConfig, BackupInfo } from "$lib/types";
  import {
    createBackup,
    defaultBackupName,
    getBackupConfig,
    inspectBackup,
    listBackups,
    listenForBackups,
    restoreBackup,
    saveBackupConfig,
  } from "$lib/stores/backup";
  import { backendError, getLocale } from "$lib/i18n/index.svelte";
  import { formatBytes } from "$lib/stores/gameStores";
  import ConfirmDialog from "./ConfirmDialog.svelte";
  import SettingToggle from "./SettingToggle.svelte";

  let automatic = $state<BackupInfo[]>([]);
  let settings = $state<BackupConfig | null>(null);
  let includeSecrets = $state(false);
  let busy = $state<"create" | "restore" | null>(null);
  let error = $state<unknown>(null);
  let created = $state<BackupInfo | null>(null);
  // The backup waiting for the user's OK to be restored.
  let confirming = $state<BackupInfo | null>(null);
  let { restored = null }: { restored?: string | null } = $props();

  async function refresh() {
    try {
      automatic = await listBackups();
    } catch (e) {
      error = e;
    }
  }

  // Also when Prefixr made one meanwhile: the daily one right after the
  // start, or the one before a restore that then failed.
  onMount(() => {
    refresh();
    getBackupConfig()
      .then((s) => (settings = s))
      .catch((e) => (error = e));
    const unlisten = listenForBackups(refresh);
    return () => {
      unlisten.then((stop) => stop());
    };
  });

  async function attempt(kind: "create" | "restore", action: () => Promise<void>) {
    busy = kind;
    error = null;
    try {
      await action();
    } catch (e) {
      error = e;
    } finally {
      busy = null;
      refresh();
    }
  }

  // Saved on every change; the backend brings the numbers into range.
  async function saveSettings(change: Partial<BackupConfig>) {
    if (!settings) return;
    error = null;
    try {
      settings = await saveBackupConfig({ ...settings, ...change });
    } catch (e) {
      error = e;
    }
  }

  async function browseDir() {
    const selected = await open({ directory: true, defaultPath: settings?.dir ?? undefined });
    if (typeof selected === "string") saveSettings({ dir: selected });
  }

  function create() {
    attempt("create", async () => {
      const path = await save({
        defaultPath: defaultBackupName(new Date()),
        filters: [{ name: m.backup_fileFilter(), extensions: ["gz"] }],
      });
      if (!path) return;
      created = await createBackup(path, includeSecrets);
    });
  }

  function pickFile() {
    attempt("restore", async () => {
      const path = await open({
        multiple: false,
        directory: false,
        filters: [{ name: m.backup_fileFilter(), extensions: ["gz"] }],
      });
      if (typeof path !== "string") return;
      confirming = await inspectBackup(path);
    });
  }

  function restore(backup: BackupInfo) {
    confirming = null;
    attempt("restore", async () => {
      await restoreBackup(backup.path);
    });
  }

  function when(secs: number): string {
    return new Date(secs * 1000).toLocaleString(getLocale(), {
      dateStyle: "medium",
      timeStyle: "short",
    });
  }

  function reason(backup: BackupInfo): string {
    switch (backup.reason) {
      case "daily":
        return m.backup_reasonDaily();
      case "before-restore":
        return m.backup_reasonBeforeRestore();
      default:
        return m.backup_reasonManual();
    }
  }

  function summary(backup: BackupInfo): string {
    return m.backup_summary({
      games: backup.games,
      categories: backup.categories,
      size: formatBytes(backup.size_bytes, getLocale()) ?? "",
    });
  }
</script>

<p class="intro">{m.backup_intro()}</p>

{#if restored}
  <p class="done banner" role="status">{m.backup_restored({ file: restored })}</p>
{/if}

<div class="panels">
  <section class="panel">
    <h2>{m.backup_createHeading()}</h2>
    <p class="hint">{m.backup_createHint()}</p>
    <label class="check">
      <input type="checkbox" bind:checked={includeSecrets} />
      {m.backup_includeSecrets()}
    </label>
    <div class="row">
      <button type="button" class="primary" disabled={busy !== null} onclick={create}>
        {busy === "create" ? m.backup_creating() : m.backup_create()}
      </button>
      {#if created}
        <span class="done" role="status">{m.backup_created({ file: created.file_name })}</span>
      {/if}
    </div>
  </section>

  <section class="panel">
    <h2>{m.backup_restoreHeading()}</h2>
    <p class="hint">{m.backup_restoreHint()}</p>
    <div class="row">
      <button type="button" disabled={busy !== null} onclick={pickFile}>
        {busy === "restore" ? m.backup_restoring() : m.backup_restoreFromFile()}
      </button>
    </div>
  </section>

  <section class="panel">
    <h2>{m.backup_automaticHeading()}</h2>
    {#if settings}
      <SettingToggle
        label={m.backup_automaticToggle()}
        description={m.backup_automaticToggleHint()}
        checked={settings.automatic}
        onToggle={(automatic) => saveSettings({ automatic })}
      >
        {#if settings.automatic}
          <div class="fields">
            <label>
              {m.backup_intervalLabel()}
              <input
                type="number"
                min="1"
                max="365"
                value={settings.interval_days}
                onchange={(e) =>
                  saveSettings({ interval_days: e.currentTarget.valueAsNumber || 1 })}
              />
            </label>
            <label>
              {m.backup_keepLabel()}
              <input
                type="number"
                min="1"
                max="100"
                value={settings.keep}
                onchange={(e) => saveSettings({ keep: e.currentTarget.valueAsNumber || 1 })}
              />
              <span class="hint">{m.backup_keepHint()}</span>
            </label>
          </div>
        {/if}
      </SettingToggle>
      <label>
        {m.backup_dirLabel()}
        <div class="row">
          <input
            type="text"
            class="dir"
            placeholder={m.backup_dirDefault()}
            value={settings.dir ?? ""}
            onchange={(e) => saveSettings({ dir: e.currentTarget.value.trim() || null })}
          />
          <button type="button" onclick={browseDir}>{m.backup_browse()}</button>
          {#if settings.dir}
            <button type="button" class="ghost" onclick={() => saveSettings({ dir: null })}
              >{m.common_reset()}</button
            >
          {/if}
        </div>
        <span class="hint">{m.backup_dirHint()}</span>
      </label>
    {/if}
    <p class="hint">{m.backup_automaticHint()}</p>
    {#if automatic.length === 0}
      <p class="muted">{m.backup_automaticEmpty()}</p>
    {:else}
      <ul class="backups">
        {#each automatic as backup (backup.path)}
          <li>
            <div class="info">
              <span class="name">{when(backup.created_at)}</span>
              <span class="muted">{reason(backup)} · {summary(backup)}</span>
            </div>
            <button
              type="button"
              disabled={busy !== null}
              onclick={() => (confirming = backup)}
              aria-label={m.backup_restoreThis({ when: when(backup.created_at) })}
              >{m.backup_restore()}</button
            >
          </li>
        {/each}
      </ul>
    {/if}
  </section>
</div>

{#if error}
  <p class="error" role="alert">{backendError(error)}</p>
{/if}

<ConfirmDialog
  open={confirming !== null}
  title={m.backup_confirmTitle()}
  message={confirming
    ? m.backup_confirmMessage({
        when: when(confirming.created_at),
        summary: summary(confirming),
      })
    : ""}
  confirmLabel={m.backup_restore()}
  onConfirm={() => confirming && restore(confirming)}
  onCancel={() => (confirming = null)}
/>

<style>
  .intro {
    color: var(--text-muted);
    font-size: 0.9em;
    max-width: 65ch;
    margin-bottom: 1.4em;
  }

  .panels {
    display: flex;
    flex-direction: column;
    gap: 0.9em;
    max-width: var(--page-max);
  }

  .panel {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 1.1em 1.2em;
    display: flex;
    flex-direction: column;
    gap: 0.7em;
  }

  h2 {
    font-size: 1em;
  }

  .hint,
  .muted {
    color: var(--text-muted);
    font-size: 0.85em;
    max-width: 65ch;
  }

  .check {
    flex-direction: row;
    align-items: center;
    gap: 0.5em;
  }

  .check input {
    width: auto;
  }

  .fields {
    display: flex;
    gap: 1em;
    flex-wrap: wrap;
  }

  .fields label {
    flex: 1;
    min-width: 12em;
  }

  .row .dir {
    flex: 1;
    min-width: 12em;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 0.8em;
    flex-wrap: wrap;
  }

  .done {
    color: var(--success);
    font-size: 0.85em;
  }

  .banner {
    max-width: var(--page-max);
    margin-bottom: 1em;
    padding: 0.6em 0.9em;
    border-radius: 8px;
    background: var(--success-bg);
  }

  .backups {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.4em;
  }

  li {
    display: flex;
    align-items: center;
    gap: 1em;
    padding: 0.5em 0.7em;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface-raised);
  }

  .info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .name {
    font-weight: 600;
  }

  .error {
    color: var(--danger);
    margin-top: 1em;
  }
</style>
