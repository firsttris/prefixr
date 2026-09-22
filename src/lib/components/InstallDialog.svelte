<script lang="ts">
  import { onMount } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { prefixes, refreshPrefixes, addPrefix } from "$lib/stores/prefixes";
  import { runners, refreshRunners } from "$lib/stores/runners";
  import { runInstaller } from "$lib/stores/games";

  let { exePath, onClose }: { exePath: string; onClose: () => void } = $props();

  let exeName = $derived(exePath.split(/[/\\]/).pop() ?? exePath);

  let prefixPath = $state("");
  let runnerId = $state("");
  let busy = $state(false);
  let creatingPrefix = $state(false);
  let started = $state(false);
  let error = $state("");

  onMount(() => {
    refreshPrefixes();
    refreshRunners();
  });

  async function createNewPrefix() {
    const selected = await open({ directory: true });
    if (typeof selected !== "string") return;
    creatingPrefix = true;
    error = "";
    try {
      await addPrefix(selected);
      prefixPath = selected;
    } catch (e) {
      error = String(e);
    } finally {
      creatingPrefix = false;
    }
  }

  async function handleStart(event: Event) {
    event.preventDefault();
    if (!runnerId || !prefixPath) return;
    busy = true;
    error = "";
    try {
      await runInstaller(prefixPath, runnerId, exePath);
      started = true;
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<div class="install-dialog">
  <p class="hint">
    <strong>{exeName}</strong> in einem Prefix ausführen, um es zu installieren. Das Spiel selbst
    trägst du danach separat über „Spiel hinzufügen“ ein.
  </p>

  {#if started}
    <p class="success">
      Setup gestartet. Sobald die Installation fertig ist, kannst du das Spiel über „Spiel
      hinzufügen“ eintragen.
    </p>
    <button type="button" class="primary" onclick={onClose}>Schließen</button>
  {:else}
    <form onsubmit={handleStart}>
      <label>
        Prefix
        <div class="row">
          <select bind:value={prefixPath} disabled={creatingPrefix}>
            <option value="" disabled selected>Prefix wählen</option>
            {#each $prefixes as prefix (prefix.path)}
              <option value={prefix.path}>{prefix.path}</option>
            {/each}
          </select>
          <button
            type="button"
            class="icon-button"
            title="Neuen Prefix erstellen"
            disabled={creatingPrefix}
            onclick={createNewPrefix}
          >
            {creatingPrefix ? "…" : "+"}
          </button>
        </div>
      </label>

      <label>
        Runner
        <select bind:value={runnerId}>
          <option value="" disabled selected>Runner wählen</option>
          {#each $runners as runner (runner.id)}
            <option value={runner.id}>{runner.name} ({runner.kind})</option>
          {/each}
        </select>
      </label>

      <button type="submit" class="primary" disabled={busy || !runnerId || !prefixPath}>
        {busy ? "Starte…" : "Setup ausführen"}
      </button>
    </form>
  {/if}

  {#if error}
    <p class="error">{error}</p>
  {/if}
</div>

<style>
  .install-dialog {
    display: flex;
    flex-direction: column;
    gap: 1em;
  }

  .hint {
    color: var(--text-muted);
    font-size: 0.9em;
  }

  .hint strong {
    color: var(--text);
    word-break: break-all;
  }

  form {
    display: flex;
    flex-direction: column;
    gap: 1em;
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

  .row select {
    flex: 1;
  }

  .icon-button {
    flex-shrink: 0;
    width: 2.2em;
    padding: 0;
    font-size: 1.1em;
    line-height: 1;
  }

  .error {
    color: var(--danger);
  }

  .success {
    color: var(--text);
    font-size: 0.9em;
  }
</style>
