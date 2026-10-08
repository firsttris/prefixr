<script lang="ts">
  import * as m from "$lib/paraglide/messages";
  import Message from "$lib/components/Message.svelte";
  import { onMount } from "svelte";
  import { loadAll } from "$lib/load";
  import { open } from "@tauri-apps/plugin-dialog";
  import { prefixes, refreshPrefixes, addPrefix, deletePrefix } from "$lib/stores/prefixes";
  import { games, refreshGames } from "$lib/stores/games";
  import { showLog } from "$lib/logViewer";
  import Modal from "$lib/components/Modal.svelte";
  import WinetricksInstaller from "$lib/components/WinetricksInstaller.svelte";
  import WineToolsLauncher from "$lib/components/WineToolsLauncher.svelte";
  import { backendError } from "$lib/i18n/index.svelte";

  let path = $state("");
  let busy = $state(false);
  let error = $state<unknown>(null);
  let loadError = $state<unknown>(null);
  let winetricksFor = $state<string | null>(null);
  let wineToolsFor = $state<string | null>(null);
  let deleting = $state<string | null>(null);
  let deleteBusy = $state(false);
  let deleteError = $state<unknown>(null);

  // How many games use each prefix. Unlike runners, a used prefix may still
  // be removed from Prefixr, since this only unregisters it and leaves the
  // directory on disk untouched.
  let usage = $derived(
    $games.reduce<Record<string, number>>((counts, game) => {
      counts[game.prefix_path] = (counts[game.prefix_path] ?? 0) + 1;
      return counts;
    }, {}),
  );

  function usageLabel(count: number): string {
    if (count === 0) return m.prefixManager_unused();
    return count === 1
      ? m.prefixManager_usedByOne()
      : m.prefixManager_usedByMany( { count });
  }

  // The games that use the prefix about to be deleted, so the dialog can
  // name them.
  let affectedGames = $derived(
    deleting ? $games.filter((g) => g.prefix_path === deleting).map((g) => g.name) : [],
  );

  onMount(() => {
    loadAll(refreshPrefixes(), refreshGames()).catch((e) => (loadError = e));
  });

  async function pickFolder() {
    const selected = await open({ directory: true });
    if (typeof selected === "string") {
      path = selected;
    }
  }

  async function handleSubmit(event: Event) {
    event.preventDefault();
    if (!path) return;
    busy = true;
    error = null;
    try {
      await addPrefix(path);
      path = "";
    } catch (e) {
      error = e;
    } finally {
      busy = false;
    }
  }

  function askDelete(prefixPath: string) {
    deleteError = null;
    deleting = prefixPath;
  }

  async function confirmDelete() {
    if (!deleting) return;
    deleteBusy = true;
    deleteError = null;
    try {
      await deletePrefix(deleting);
      deleting = null;
    } catch (e) {
      deleteError = e;
    } finally {
      deleteBusy = false;
    }
  }
</script>

{#snippet drive()}<strong>{m.prefixManager_explainerDrive()}</strong>{/snippet}
{#snippet gameNames()}<strong>{affectedGames.join(", ")}</strong>{/snippet}

<section class="panel">
  <p class="explainer">
    <Message message={m.prefixManager_explainer} parts={{ drive }} />
  </p>

  <p class="hint">{m.prefixManager_hint()}</p>

  <form onsubmit={handleSubmit}>
    <label>
      {m.prefixManager_pathLabel()}
      <div class="row">
        <input placeholder={m.prefixManager_pathPlaceholder()} bind:value={path} />
        <button type="button" onclick={pickFolder}>{m.prefixManager_chooseFolder()}</button>
      </div>
    </label>

    <button type="submit" class="primary" disabled={busy || !path}>
      {busy ? m.prefixManager_adding() : m.prefixManager_addPrefix()}
    </button>
  </form>

  {#if error}
    <p class="error">{backendError(error)}</p>
  {/if}

  {#if loadError}
    <p class="error">{backendError(loadError)}</p>
  {:else if $prefixes.length > 0}
    <ul>
      {#each $prefixes as prefix (prefix.path)}
        {@const count = usage[prefix.path] ?? 0}
        <li>
          <div class="prefix-info">
            <span>{prefix.path}</span>
            <span class="usage" class:unused={count === 0}>{usageLabel(count)}</span>
          </div>
          <div class="actions">
            <button
              type="button"
              class="ghost"
              onclick={() => (winetricksFor = prefix.path)}
              aria-label={m.prefixManager_installDeps()}
            >
              📦
            </button>
            <button
              type="button"
              class="ghost"
              onclick={() => (wineToolsFor = prefix.path)}
              aria-label={m.prefixManager_openWineTools()}
            >
              🛠️
            </button>
            <button type="button" class="ghost" onclick={() => askDelete(prefix.path)}>
              {m.prefixManager_remove()}
            </button>
          </div>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<Modal
  open={deleting !== null}
  title={m.prefixManager_removeTitle()}
  onClose={() => (deleting = null)}
>
  {#if deleting}
    <p class="path">{deleting}</p>
    {#if affectedGames.length > 0}
      <p>
        <Message message={m.prefixManager_usedBy} parts={{ games: gameNames }} />
      </p>
    {/if}
    <p>{m.prefixManager_deleteExplain()}</p>
    {#if deleteError}
      <p class="error">{backendError(deleteError)}</p>
    {/if}
    <div class="dialog-actions">
      <button type="button" class="ghost" onclick={() => (deleting = null)}
        >{m.common_cancel()}</button
      >
      <button type="button" class="danger" disabled={deleteBusy} onclick={confirmDelete}>
        {m.prefixManager_removeFromAppOnly()}
      </button>
    </div>
  {/if}
</Modal>

<Modal
  open={winetricksFor !== null}
  title={m.prefixManager_installDeps()}
  onClose={() => (winetricksFor = null)}
>
  {#if winetricksFor}
    <WinetricksInstaller prefixPath={winetricksFor} onShowLog={showLog} />
  {/if}
</Modal>

<Modal
  open={wineToolsFor !== null}
  title={m.prefixManager_wineToolsTitle()}
  onClose={() => (wineToolsFor = null)}
>
  {#if wineToolsFor}
    <WineToolsLauncher prefixPath={wineToolsFor} />
  {/if}
</Modal>

<style>
  .panel {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 1.2em;
    display: flex;
    flex-direction: column;
    gap: 1em;
    max-width: 780px;
  }

  .explainer {
    color: var(--text-muted);
    font-size: 0.9em;
    max-width: 60ch;
  }

  .explainer strong {
    color: var(--text);
  }

  .hint {
    color: var(--text-muted);
    font-size: 0.85em;
  }

  form {
    display: flex;
    flex-direction: column;
    gap: 1em;
  }

  .row {
    display: flex;
    gap: 0.5em;
  }

  .row input {
    flex: 1;
  }

  .error {
    color: var(--danger);
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.4em;
  }

  li {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1em;
    padding: 0.5em 0.7em;
    background: var(--surface-raised);
    border-radius: 8px;
    font-size: 0.9em;
  }

  li span {
    word-break: break-all;
  }

  .prefix-info {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 0.2em;
  }

  .usage {
    color: var(--text-muted);
    font-size: 0.82em;
  }

  .usage.unused {
    font-style: italic;
  }

  .actions {
    display: flex;
    gap: 0.3em;
    flex-shrink: 0;
  }

  .path {
    font-family: monospace;
    word-break: break-all;
  }

  .dialog-actions {
    display: flex;
    justify-content: flex-end;
    flex-wrap: wrap;
    gap: 0.6em;
    margin-top: 1.2em;
  }
</style>
