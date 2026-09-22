<script lang="ts">
  import { onMount } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { prefixes, refreshPrefixes, addPrefix } from "$lib/stores/prefixes";
  import { runners, refreshRunners } from "$lib/stores/runners";
  import { runInstaller } from "$lib/stores/games";

  let { exePath, onClose }: { exePath: string; onClose: () => void } = $props();

  let exeName = $derived(exePath.split(/[/\\]/).pop() ?? exePath);

  let prefixPath = $state("");
  let newPrefixPath = $state("");
  let runnerId = $state("");
  let busy = $state(false);
  let started = $state(false);
  let error = $state("");

  onMount(() => {
    refreshPrefixes();
    refreshRunners();
  });

  async function pickNewPrefixFolder() {
    const selected = await open({ directory: true });
    if (typeof selected === "string") {
      newPrefixPath = selected;
      prefixPath = "";
    }
  }

  async function handleStart(event: Event) {
    event.preventDefault();
    if (!runnerId) return;
    busy = true;
    error = "";
    try {
      let targetPrefix = prefixPath;
      if (!targetPrefix && newPrefixPath) {
        await addPrefix(newPrefixPath);
        targetPrefix = newPrefixPath;
      }
      if (!targetPrefix) return;
      await runInstaller(targetPrefix, runnerId, exePath);
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
    trägst du danach separat über „Spiel hinzufügen“ ein — die Setup-exe ist ja meist eine andere
    Datei als die, die das Spiel am Ende startet.
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
        <select bind:value={prefixPath} disabled={newPrefixPath !== ""}>
          <option value="" disabled selected>Prefix wählen</option>
          {#each $prefixes as prefix (prefix.path)}
            <option value={prefix.path}>{prefix.path}</option>
          {/each}
        </select>
      </label>

      <div class="or">oder</div>

      <label>
        Neuer Prefix
        <div class="row">
          <input
            placeholder="z. B. /home/du/prefixes/mein-spiel"
            bind:value={newPrefixPath}
            oninput={() => (prefixPath = "")}
          />
          <button type="button" onclick={pickNewPrefixFolder}>Ordner wählen</button>
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

      <button
        type="submit"
        class="primary"
        disabled={busy || !runnerId || (!prefixPath && !newPrefixPath)}
      >
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

  .row input {
    flex: 1;
  }

  .or {
    color: var(--text-muted);
    font-size: 0.8em;
    text-align: center;
  }

  .error {
    color: var(--danger);
  }

  .success {
    color: var(--text);
    font-size: 0.9em;
  }
</style>
