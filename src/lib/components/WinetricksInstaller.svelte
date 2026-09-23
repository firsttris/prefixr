<script lang="ts">
  import { onMount } from "svelte";
  import { runners, refreshRunners } from "$lib/stores/runners";
  import {
    WINETRICKS_VERBS,
    installWinetricksVerbs,
    listAllWinetricksVerbs,
    listInstalledWinetricksVerbs,
    type WinetricksVerbMeta,
  } from "$lib/stores/winetricks";
  import { t, type TranslationKey } from "$lib/i18n/index.svelte";

  let { prefixPath, onShowLog }: { prefixPath: string; onShowLog: (path: string) => void } =
    $props();

  let runnerId = $state("");
  let selected = $state<Set<string>>(new Set());
  let installing = $state(false);
  let error = $state("");
  let successLogPath = $state("");

  let installed = $state<Set<string>>(new Set());

  let catalogueOpen = $state(false);
  let catalogueLoading = $state(false);
  let catalogueError = $state("");
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
    refreshRunners();
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
      catalogueError = "";
      try {
        allVerbs = await listAllWinetricksVerbs();
      } catch (e) {
        catalogueError = String(e);
      } finally {
        catalogueLoading = false;
      }
    }
  }

  function toggle(id: string) {
    const next = new Set(selected);
    if (next.has(id)) {
      next.delete(id);
    } else {
      next.add(id);
    }
    selected = next;
  }

  async function handleInstall() {
    if (!runnerId || selected.size === 0) return;
    installing = true;
    error = "";
    successLogPath = "";
    try {
      successLogPath = await installWinetricksVerbs(prefixPath, runnerId, [...selected]);
      await refreshInstalled();
    } catch (e) {
      error = String(e);
    } finally {
      installing = false;
    }
  }
</script>

<div class="winetricks">
  <p class="hint">
    {t("winetricksInstaller.hintBefore")}<code>{prefixPath}</code
    >{t("winetricksInstaller.hintAfter")}
  </p>

  <label>
    {t("winetricksInstaller.runnerLabel")}
    <select bind:value={runnerId}>
      <option value="" disabled selected>{t("gameForm.runnerChoose")}</option>
      {#each $runners as runner (runner.id)}
        <option value={runner.id}>{runner.name} ({runner.kind})</option>
      {/each}
    </select>
  </label>

  <div class="verb-list">
    {#each WINETRICKS_VERBS as verb (verb.id)}
      <label class="verb-row">
        <input
          type="checkbox"
          checked={selected.has(verb.id)}
          onchange={() => toggle(verb.id)}
        />
        <div>
          <span class="verb-label">
            {t(`winetricksVerbs.${verb.id}.label` as TranslationKey)}
            {#if installed.has(verb.id)}
              <span class="badge">{t("winetricksInstaller.installedBadge")}</span>
            {/if}
          </span>
          <p class="verb-desc">{t(`winetricksVerbs.${verb.id}.description` as TranslationKey)}</p>
        </div>
      </label>
    {/each}
  </div>

  <div class="catalogue">
    <button type="button" class="ghost disclosure" onclick={toggleCatalogue}>
      {catalogueOpen ? "▾" : "▸"}
      {t("winetricksInstaller.browseMore")}
    </button>

    {#if catalogueOpen}
      <div class="catalogue-body">
        {#if catalogueLoading}
          <p class="hint">{t("winetricksInstaller.loadingCatalogue")}</p>
        {:else if catalogueError}
          <p class="error">{catalogueError}</p>
        {:else}
          <input
            class="search"
            type="search"
            placeholder={t("winetricksInstaller.searchPlaceholder")}
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
              <p class="hint">{t("winetricksInstaller.noMatches")}</p>
            {/each}
          </div>
        {/if}
      </div>
    {/if}
  </div>

  {#if error}
    <div class="toast">
      <span>{error}</span>
    </div>
  {/if}

  {#if successLogPath}
    <div class="success">
      <span>{t("winetricksInstaller.installSuccess")}</span>
      <button type="button" class="ghost" onclick={() => onShowLog(successLogPath)}>
        {t("winetricksInstaller.showLog")}
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
      ? t("winetricksInstaller.installing")
      : t("winetricksInstaller.installButton", { count: selected.size })}
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
