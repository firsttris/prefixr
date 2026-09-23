<script lang="ts">
  import { onMount } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { prefixes, refreshPrefixes, addPrefix, deletePrefix } from "$lib/stores/prefixes";
  import { games, refreshGames } from "$lib/stores/games";
  import { showLog } from "$lib/logViewer";
  import Modal from "$lib/components/Modal.svelte";
  import WinetricksInstaller from "$lib/components/WinetricksInstaller.svelte";
  import WineToolsLauncher from "$lib/components/WineToolsLauncher.svelte";

  let path = $state("");
  let busy = $state(false);
  let error = $state("");
  let winetricksFor = $state<string | null>(null);
  let wineToolsFor = $state<string | null>(null);
  let deleting = $state<string | null>(null);
  let deleteBusy = $state(false);
  let deleteError = $state("");

  // The games that use the prefix about to be deleted, so the dialog can
  // name them.
  let affectedGames = $derived(
    deleting ? $games.filter((g) => g.prefix_path === deleting).map((g) => g.name) : [],
  );

  onMount(() => {
    refreshPrefixes();
    refreshGames();
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

  function askDelete(prefixPath: string) {
    deleteError = "";
    deleting = prefixPath;
  }

  async function confirmDelete(deleteFiles: boolean) {
    if (!deleting) return;
    deleteBusy = true;
    deleteError = "";
    try {
      await deletePrefix(deleting, deleteFiles);
      deleting = null;
    } catch (e) {
      deleteError = String(e);
    } finally {
      deleteBusy = false;
    }
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
            <button
              type="button"
              class="ghost"
              onclick={() => (wineToolsFor = prefix.path)}
              aria-label="Wine-Werkzeuge öffnen"
            >
              🛠️
            </button>
            <button type="button" class="ghost" onclick={() => askDelete(prefix.path)}>
              Löschen
            </button>
          </div>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<Modal open={deleting !== null} title="Prefix entfernen" onClose={() => (deleting = null)}>
  {#if deleting}
    <p class="path">{deleting}</p>
    {#if affectedGames.length > 0}
      <p>
        Verwendet von: <strong>{affectedGames.join(", ")}</strong>. Diese Spiele behalten den
        Prefix, auch wenn er nur aus Prefixr entfernt wird.
      </p>
    {/if}
    <p>
      „Ordner löschen“ löscht den Prefix endgültig, mit allen installierten Programmen und
      Spielständen darin. Bei einem Prefix aus Lutris, Bottles oder PortProton reicht meist „Nur
      aus Prefixr entfernen“.
    </p>
    {#if deleteError}
      <p class="error">{deleteError}</p>
    {/if}
    <div class="dialog-actions">
      <button type="button" class="ghost" onclick={() => (deleting = null)}>Abbrechen</button>
      <button type="button" disabled={deleteBusy} onclick={() => confirmDelete(false)}>
        Nur aus Prefixr entfernen
      </button>
      <button
        type="button"
        class="danger"
        disabled={deleteBusy}
        onclick={() => confirmDelete(true)}
      >
        Ordner löschen
      </button>
    </div>
  {/if}
</Modal>

<Modal
  open={winetricksFor !== null}
  title="Abhängigkeiten installieren"
  onClose={() => (winetricksFor = null)}
>
  {#if winetricksFor}
    <WinetricksInstaller prefixPath={winetricksFor} onShowLog={showLog} />
  {/if}
</Modal>

<Modal
  open={wineToolsFor !== null}
  title="Wine-Werkzeuge"
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
