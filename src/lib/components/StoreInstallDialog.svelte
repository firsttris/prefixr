<!--
  Installing a store game: where it goes, which prefix and runner it gets.
  Everything is filled in already, so "Install" is usually all it takes.
  The install itself runs on after the dialog closes; its card shows the
  progress.
-->
<script lang="ts">
  import * as m from "$lib/paraglide/messages";
  import { onMount } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import RunnerSelect from "$lib/components/RunnerSelect.svelte";
  import { loadAll } from "$lib/load";
  import { getLocale } from "$lib/i18n/index.svelte";
  import { prefixes, refreshPrefixes } from "$lib/stores/prefixes";
  import { runners, refreshRunners } from "$lib/stores/runners";
  import {
    formatBytes,
    getStoreInstallDefaults,
    installStoreGame,
    storeName,
  } from "$lib/stores/gameStores";
  import { backendError } from "$lib/i18n/index.svelte";
  import type { StoreGame, StoreInstallDefaults } from "$lib/types";

  let {
    game,
    onClose,
  }: {
    game: StoreGame;
    onClose: () => void;
  } = $props();

  // "" is a prefix of the game's own, created by the install.
  const NEW_PREFIX = "";

  let defaults = $state<StoreInstallDefaults | null>(null);
  let installDir = $state("");
  let prefixPath = $state(NEW_PREFIX);
  let runnerId = $state("");
  let loading = $state(true);
  let error = $state<unknown>(null);

  onMount(() => {
    const loads = loadAll(
      getStoreInstallDefaults(game.store, game.app_name).then((d) => {
        defaults = d;
        installDir = d.install_dir;
      }),
      refreshPrefixes(),
      refreshRunners(),
    );
    loads.catch((e) => (error = e)).finally(() => (loading = false));
  });

  // The newest Proton runner as the default — by its name, compared with
  // its numbers as numbers (GE-Proton10-4 is newer than GE-Proton9-27).
  $effect(() => {
    if (!runnerId && $runners.length > 0) {
      const newestFirst = [...$runners].sort((a, b) =>
        b.name.localeCompare(a.name, undefined, { numeric: true }),
      );
      runnerId = (newestFirst.find((r) => r.kind === "proton") ?? newestFirst[0]!).id;
    }
  });

  let download = $derived(formatBytes(defaults?.sizes.download_size, getLocale()));
  let disk = $derived(formatBytes(defaults?.sizes.disk_size, getLocale()));
  let canInstall = $derived(!loading && installDir.trim() !== "" && runnerId !== "");

  async function browse() {
    const selected = await open({ directory: true, defaultPath: installDir || undefined });
    if (typeof selected === "string") installDir = selected;
  }

  function handleSubmit(event: Event) {
    event.preventDefault();
    if (!canInstall) return;
    // Runs on in the background; the card shows its progress and any error.
    installStoreGame(game.store, game.app_name, {
      install_dir: installDir.trim(),
      prefix_path: prefixPath === NEW_PREFIX ? null : prefixPath,
      runner_id: runnerId,
    });
    onClose();
  }
</script>

<form class="store-install" onsubmit={handleSubmit}>
  <div class="chips">
    <span class="chip">{storeName(game.store)}</span>
    {#if download}
      <span class="chip">{m.storeInstall_download({ size: download })}</span>
    {/if}
    {#if disk}
      <span class="chip">{m.storeInstall_disk({ size: disk })}</span>
    {/if}
  </div>

  <label>
    {m.storeInstall_folderLabel()}
    <div class="row">
      <input type="text" bind:value={installDir} disabled={loading} />
      <button type="button" onclick={browse} disabled={loading}>{m.storeInstall_browse()}</button>
    </div>
    <span class="hint">{m.storeInstall_folderHint()}</span>
  </label>

  <div class="two">
    <label>
      {m.gameForm_prefixLabel()}
      <select bind:value={prefixPath} disabled={loading}>
        <option value={NEW_PREFIX}>{m.storeInstall_newPrefix()}</option>
        {#each $prefixes as prefix (prefix.path)}
          <option value={prefix.path}>{prefix.path}</option>
        {/each}
      </select>
    </label>
    <label>
      {m.gameForm_runnerLabel()}
      <RunnerSelect bind:value={runnerId} disabled={loading} />
      {#if !loading && $runners.length === 0}
        <span class="hint">{m.storeInstall_noRunners()}</span>
      {/if}
    </label>
  </div>

  <div class="note">
    <svg viewBox="0 0 24 24" fill="none" aria-hidden="true">
      <circle cx="12" cy="12" r="9" stroke="currentColor" stroke-width="2" />
      <path d="M12 11v5M12 8h.01" stroke="currentColor" stroke-width="2" stroke-linecap="round" />
    </svg>
    <p>
      <strong>{m.storeInstall_noteTitle()}</strong>
      {m.storeInstall_noteText()}
    </p>
  </div>

  {#if error}
    <p class="error">{backendError(error)}</p>
  {/if}

  <div class="actions">
    <button type="button" class="ghost" onclick={onClose}>{m.common_cancel()}</button>
    <button type="submit" class="primary" disabled={!canInstall}>
      {loading ? m.storeInstall_loading() : m.storeGame_install()}
    </button>
  </div>
</form>

<style>
  .store-install {
    display: flex;
    flex-direction: column;
    gap: 1.1em;
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5em;
  }

  .chip {
    font-size: 0.8em;
    background: var(--surface-raised);
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 0.15em 0.7em;
  }

  label {
    display: flex;
    flex-direction: column;
    gap: 0.3em;
    font-size: 0.9em;
  }

  .row {
    display: flex;
    gap: 0.5em;
  }

  .row input {
    flex: 1;
  }

  .row button {
    flex-shrink: 0;
  }

  .two {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.9em;
  }

  .hint {
    color: var(--text-muted);
    font-size: 0.85em;
  }

  .note {
    display: flex;
    gap: 0.75em;
    align-items: flex-start;
    padding: 0.75em 0.9em;
    border-radius: var(--radius);
    background: var(--surface-raised);
    border: 1px solid var(--border);
    font-size: 0.85em;
    color: var(--text-muted);
  }

  .note svg {
    width: 1.3em;
    height: 1.3em;
    flex-shrink: 0;
    color: var(--accent);
    margin-top: 0.1em;
  }

  .note strong {
    display: block;
    color: var(--text);
    margin-bottom: 0.15em;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.6em;
  }

  .error {
    color: var(--danger);
  }
</style>
