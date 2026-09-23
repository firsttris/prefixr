<script lang="ts">
  import { onMount } from "svelte";
  import { umuStatus, refreshUmuStatus, installUmu } from "$lib/stores/umu";

  let installing = $state(false);
  let error = $state("");
  let updated = $state(false);

  onMount(() => {
    refreshUmuStatus();
  });

  async function handleInstall() {
    installing = true;
    error = "";
    updated = false;
    try {
      await installUmu();
      updated = true;
    } catch (e) {
      error = String(e);
    } finally {
      installing = false;
    }
  }
</script>

<details class="umu">
  <summary>
    umu-launcher
    {#if $umuStatus?.installed}
      <span class="status">{$umuStatus.version ?? "installiert"}</span>
    {:else if $umuStatus}
      <span class="status">nicht installiert</span>
    {/if}
  </summary>
  <p class="hint">
    Proton-Runner starten über umu — so, wie Steam selbst Proton startet: in der Steam Linux
    Runtime, mit allen <code>PROTON_*</code>-Optionen und automatischen Fixes pro Spiel
    (protonfixes). umu wird beim ersten Start eines Proton-Spiels automatisch geladen; die
    Steam Runtime (einige hundert MB) ebenfalls, einmalig, und wird mit Lutris, Heroic & Co.
    geteilt.
  </p>

  <div class="row">
    <button type="button" class="ghost" disabled={installing} onclick={handleInstall}>
      {#if installing}
        Lädt…
      {:else if $umuStatus?.installed}
        Auf neueste Version aktualisieren
      {:else}
        Jetzt installieren
      {/if}
    </button>
  </div>

  {#if error}
    <p class="error">{error}</p>
  {:else if updated}
    <p class="saved-hint">umu {$umuStatus?.version ?? ""} ist installiert.</p>
  {/if}
</details>

<style>
  .umu {
    background: var(--surface-raised);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 0.5em 0.8em;
    margin-bottom: 0.8em;
    font-size: 0.9em;
  }

  summary {
    cursor: pointer;
    color: var(--text-muted);
    font-weight: 600;
    font-size: 0.85em;
  }

  .status {
    font-weight: 400;
    margin-left: 0.4em;
  }

  .hint {
    color: var(--text-muted);
    font-size: 0.85em;
    max-width: 60ch;
    margin: 0.6em 0;
  }

  .row {
    display: flex;
    gap: 0.5em;
  }

  .error {
    color: var(--danger);
    font-size: 0.85em;
    margin: 0.5em 0 0;
  }

  .saved-hint {
    color: var(--text-muted);
    font-size: 0.85em;
    margin: 0.5em 0 0;
  }
</style>
