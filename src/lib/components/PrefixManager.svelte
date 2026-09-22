<script lang="ts">
  import { onMount } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { prefixes, refreshPrefixes, addPrefix, deletePrefix } from "$lib/stores/prefixes";
  import { showLog } from "$lib/logViewer";
  import Modal from "$lib/components/Modal.svelte";
  import WinetricksInstaller from "$lib/components/WinetricksInstaller.svelte";

  let path = $state("");
  let busy = $state(false);
  let error = $state("");
  let winetricksFor = $state<string | null>(null);

  onMount(() => {
    refreshPrefixes();
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
    error = "";
    try {
      await addPrefix(path);
      path = "";
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function handleDelete(prefixPath: string) {
    await deletePrefix(prefixPath);
  }
</script>

<section class="panel">
  <p class="explainer">
    Ein Prefix ist ein eigenständiges, virtuelles Windows-Dateisystem — mit eigenem
    <strong>C:-Laufwerk</strong> und eigener Registry — für installierte Programme. Jeder Prefix ist
    komplett getrennt von deinem echten Linux-System und von anderen Prefixen, so wie eine eigene
    kleine Windows-Installation nur für dieses eine Spiel.
  </p>

  <p class="hint">
    Neuer, leerer Ordner oder ein bereits vorhandener Wine-Prefix (z. B. aus PortProton, Lutris
    oder Bottles) — initialisiert wird er automatisch beim ersten Spielstart, mit dem Runner, den
    du für dieses Spiel wählst.
  </p>

  <form onsubmit={handleSubmit}>
    <label>
      Pfad
      <div class="row">
        <input placeholder="z. B. /home/du/prefixes/mein-spiel" bind:value={path} />
        <button type="button" onclick={pickFolder}>Ordner wählen</button>
      </div>
    </label>

    <button type="submit" class="primary" disabled={busy || !path}>
      {busy ? "Füge hinzu…" : "Prefix hinzufügen"}
    </button>
  </form>

  {#if error}
    <p class="error">{error}</p>
  {/if}

  {#if $prefixes.length > 0}
    <ul>
      {#each $prefixes as prefix (prefix.path)}
        <li>
          <span>{prefix.path}</span>
          <div class="actions">
            <button
              type="button"
              class="ghost"
              onclick={() => (winetricksFor = prefix.path)}
              aria-label="Abhängigkeiten installieren"
            >
              📦
            </button>
            <button type="button" class="ghost" onclick={() => handleDelete(prefix.path)}>
              Löschen
            </button>
          </div>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<Modal
  open={winetricksFor !== null}
  title="Abhängigkeiten installieren"
  onClose={() => (winetricksFor = null)}
>
  {#if winetricksFor}
    <WinetricksInstaller prefixPath={winetricksFor} onShowLog={showLog} />
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

  .actions {
    display: flex;
    gap: 0.3em;
    flex-shrink: 0;
  }
</style>
