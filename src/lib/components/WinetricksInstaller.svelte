<script lang="ts">
  import * as m from "$lib/paraglide/messages";
  import RunnerSelect from "$lib/components/RunnerSelect.svelte";
  import Message from "$lib/components/Message.svelte";
  import { msgGroup } from "$lib/i18n/msg-groups";
  import { onMount, untrack } from "svelte";
  import { SvelteSet } from "svelte/reactivity";
  import { prefixRunner, refreshRunners } from "$lib/stores/runners";
  import { games } from "$lib/stores/games";
  import {
    WINETRICKS_VERBS,
    installWinetricksVerbs,
    listAllWinetricksVerbs,
    listInstalledWinetricksVerbs,
    type WinetricksVerbMeta,
  } from "$lib/stores/winetricks";
  import { backendError, pickMsg } from "$lib/i18n/index.svelte";

  let { prefixPath, onShowLog }: { prefixPath: string; onShowLog: (path: string) => void } =
    $props();

  // Started with the runner the prefix's games use, if they agree on one.
  let runnerId = $state(untrack(() => prefixRunner($games, prefixPath)));
  const selected = new SvelteSet<string>();
  let installing = $state(false);
  let error = $state<unknown>(null);
  let successLogPath = $state("");

  let installed = $state<Set<string>>(new Set());

  let catalogueOpen = $state(false);
  let catalogueLoading = $state(false);
  let catalogueError = $state<unknown>(null);
  let allVerbs = $state<WinetricksVerbMeta[]>([]);
  let search = $state("");

  let filteredVerbs = $derived.by(() => {
    const q = search.trim().toLowerCase();
    const list = q
      ? allVerbs.filter((v) => v.id.toLowerCase().includes(q) || v.title.toLowerCase().includes(q))
      : allVerbs;
    return [...list].sort((a, b) => a.title.localeCompare(b.title));
  });

  onMount(() => {
    refreshRunners().catch((e) => (error = e));
    refreshInstalled();
  });

  async function refreshInstalled() {
    try {
      installed = new Set(await listInstalledWinetricksVerbs(prefixPath));
    } catch {
      // Best-effort — just leaves the ✓ badges off if this fails.
    }
  }

  async function toggleCatalogue() {
    catalogueOpen = !catalogueOpen;
    if (catalogueOpen && allVerbs.length === 0 && !catalogueLoading) {
      catalogueLoading = true;
      catalogueError = null;
      try {
        allVerbs = await listAllWinetricksVerbs();
      } catch (e) {
        catalogueError = e;
      } finally {
        catalogueLoading = false;
      }
    }
  }

  function toggle(id: string) {
    if (!selected.delete(id)) selected.add(id);
  }

  async function handleInstall() {
    if (!runnerId || selected.size === 0) return;
    installing = true;
    error = null;
    successLogPath = "";
    try {
      successLogPath = await installWinetricksVerbs(prefixPath, runnerId, [...selected]);
      await refreshInstalled();
    } catch (e) {
      error = e;
    } finally {
      installing = false;
    }
  }
</script>

{#snippet prefix()}<code>{prefixPath}</code>{/snippet}

<div class="winetricks">
  <p class="hint">
    <Message message={m.winetricksInstaller_hint} parts={{ prefix }} />
  </p>

  <label>
    {m.winetricksInstaller_runnerLabel()}
    <RunnerSelect bind:value={runnerId} />
  </label>

  <div class="verb-list">
    {#each WINETRICKS_VERBS as verb (verb.id)}
      <label class="verb-row">
        <input type="checkbox" checked={selected.has(verb.id)} onchange={() => toggle(verb.id)} />
        <div>
          <span class="verb-label">
            {pickMsg(msgGroup.winetricksVerbs_label, verb.id)}
            {#if installed.has(verb.id)}
              <span class="badge">{m.winetricksInstaller_installedBadge()}</span>
            {/if}
          </span>
          <p class="verb-desc">{pickMsg(msgGroup.winetricksVerbs_description, verb.id)}</p>
        </div>
      </label>
    {/each}
  </div>

  <div class="catalogue">
    <button type="button" class="ghost disclosure" onclick={toggleCatalogue}>
      {catalogueOpen ? "▾" : "▸"}
      {m.winetricksInstaller_browseMore()}
    </button>

    {#if catalogueOpen}
      <div class="catalogue-body">
        {#if catalogueLoading}
          <p class="hint">{m.winetricksInstaller_loadingCatalogue()}</p>
        {:else if catalogueError}
          <p class="error">{backendError(catalogueError)}</p>
        {:else}
          <input
            class="search"
            type="search"
            placeholder={m.winetricksInstaller_searchPlaceholder()}
            bind:value={search}
          />
          <div class="catalogue-list">
            {#each filteredVerbs as verb (verb.id)}
              <label class="catalogue-row">
                <input
                  type="checkbox"
                  checked={selected.has(verb.id)}
                  onchange={() => toggle(verb.id)}
                />
                <span class="catalogue-title">
                  {verb.title}
                  {#if installed.has(verb.id)}<span class="badge">✓</span>{/if}
                </span>
                <span class="catalogue-id">{verb.id}</span>
              </label>
            {:else}
              <p class="hint">{m.winetricksInstaller_noMatches()}</p>
            {/each}
          </div>
        {/if}
      </div>
    {/if}
  </div>

  {#if error}
    <div class="toast">
      <span>{backendError(error)}</span>
    </div>
  {/if}

  {#if successLogPath}
    <div class="success">
      <span>{m.winetricksInstaller_installSuccess()}</span>
      <button type="button" class="ghost" onclick={() => onShowLog(successLogPath)}>
        {m.winetricksInstaller_showLog()}
      </button>
    </div>
  {/if}

  <button
    type="button"
    class="primary"
    disabled={installing || !runnerId || selected.size === 0}
    onclick={handleInstall}
  >
    {installing
      ? m.winetricksInstaller_installing()
      : m.winetricksInstaller_installButton({ count: selected.size })}
  </button>
</div>

<style>
  .winetricks {
    display: flex;
    flex-direction: column;
    gap: 1em;
  }

  .hint {
    color: var(--text-muted);
    font-size: 0.85em;
  }

  .hint code {
    word-break: break-all;
  }

  label {
    display: flex;
    flex-direction: column;
    gap: 0.3em;
    font-size: 0.9em;
  }

  .verb-list {
    display: flex;
    flex-direction: column;
    gap: 0.6em;
  }

  .verb-row {
    display: flex;
    align-items: flex-start;
    gap: 0.6em;
    padding: 0.6em 0.7em;
    border-radius: 8px;
    background: var(--surface-raised);
    border: 1px solid var(--border);
    cursor: pointer;
  }

  .verb-row input {
    margin-top: 0.2em;
    width: auto;
  }

  .verb-label {
    font-weight: 600;
    display: flex;
    align-items: center;
    gap: 0.5em;
  }

  .verb-desc {
    color: var(--text-muted);
    font-size: 0.82em;
    margin: 0.2em 0 0;
  }

  .badge {
    font-size: 0.75em;
    font-weight: 500;
    color: var(--accent);
    white-space: nowrap;
  }

  .catalogue {
    border-top: 1px solid var(--border);
    padding-top: 0.8em;
  }

  .disclosure {
    width: 100%;
    text-align: left;
    font-size: 0.85em;
    color: var(--text-muted);
  }

  .catalogue-body {
    display: flex;
    flex-direction: column;
    gap: 0.6em;
    margin-top: 0.6em;
  }

  .search {
    width: 100%;
  }

  .catalogue-list {
    display: flex;
    flex-direction: column;
    gap: 0.2em;
    max-height: 220px;
    overflow-y: auto;
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 0.3em;
  }

  .catalogue-row {
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 0.5em;
    padding: 0.35em 0.5em;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.85em;
  }

  .catalogue-row:hover {
    background: var(--surface-raised);
  }

  .catalogue-row input {
    width: auto;
    flex-shrink: 0;
  }

  .catalogue-title {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 0.4em;
  }

  .catalogue-id {
    color: var(--text-muted);
    font-size: 0.85em;
    flex-shrink: 0;
  }

  .toast {
    background: var(--danger-bg);
    color: var(--danger);
    border-radius: 8px;
    padding: 0.5em 0.7em;
    font-size: 0.85em;
  }

  .error {
    color: var(--danger);
    font-size: 0.85em;
  }

  .success {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1em;
    background: var(--surface-raised);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 0.5em 0.7em;
    font-size: 0.85em;
  }
</style>
