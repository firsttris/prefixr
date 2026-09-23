<script lang="ts">
  import { onMount } from "svelte";
  import { runners, refreshRunners } from "$lib/stores/runners";
  import Modal from "./Modal.svelte";
  import RunnerDownloads from "./RunnerDownloads.svelte";
  import GitHubSettings from "./GitHubSettings.svelte";
  import UmuSettings from "./UmuSettings.svelte";

  let showDownloads = $state(false);

  onMount(() => {
    refreshRunners();
  });
</script>

<section class="panel">
  <p class="explainer">
    Ein Runner ist die Kompatibilitätsschicht, die ein Windows-Spiel unter Linux überhaupt zum
    Laufen bringt — z. B. <strong>Proton</strong> (bekannt von Steam) oder <strong>Wine</strong>.
    Er übersetzt die Windows-Programmaufrufe des Spiels ins Linux-System, quasi eine
    Übersetzungsschicht zwischen den beiden Welten.
  </p>

  <div class="panel-header">
    <h2>Installierte Runner</h2>
    <div class="actions">
      <button type="button" class="ghost" onclick={refreshRunners}>Aktualisieren</button>
      <button type="button" class="primary" onclick={() => (showDownloads = true)}>
        Runner herunterladen
      </button>
    </div>
  </div>

  <GitHubSettings />
  <UmuSettings />

  {#if $runners.length === 0}
    <p class="hint">Keine Runner gefunden. Lege Proton- oder Wine-Builds im Runner-Verzeichnis ab.</p>
  {:else}
    <ul>
      {#each $runners as runner (runner.id)}
        <li>
          <span class="name">{runner.name}</span>
          <span class="kind">{runner.kind}</span>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<Modal
  open={showDownloads}
  title="Runner herunterladen"
  onClose={() => (showDownloads = false)}
>
  <RunnerDownloads />
</Modal>

<style>
  .panel {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 1.2em;
    display: flex;
    flex-direction: column;
    gap: 0.8em;
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

  .panel-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.8em;
    flex-wrap: wrap;
  }

  .actions {
    display: flex;
    gap: 0.5em;
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
    padding: 0.5em 0.7em;
    background: var(--surface-raised);
    border-radius: 8px;
    font-size: 0.9em;
  }

  .kind {
    color: var(--text-muted);
    font-size: 0.85em;
    text-transform: uppercase;
  }

  .hint {
    color: var(--text-muted);
    font-size: 0.9em;
  }
</style>
