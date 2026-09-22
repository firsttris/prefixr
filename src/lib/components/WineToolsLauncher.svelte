<script lang="ts">
  import { onMount } from "svelte";
  import { runners, refreshRunners } from "$lib/stores/runners";
  import { WINE_TOOLS, launchWineTool } from "$lib/stores/wineTools";

  let { prefixPath }: { prefixPath: string } = $props();

  let runnerId = $state("");
  let launching = $state("");
  let error = $state("");

  onMount(() => {
    refreshRunners();
  });

  async function handleLaunch(tool: string) {
    if (!runnerId) return;
    launching = tool;
    error = "";
    try {
      await launchWineTool(prefixPath, runnerId, tool);
    } catch (e) {
      error = String(e);
    } finally {
      launching = "";
    }
  }
</script>

<div class="wine-tools">
  <p class="hint">
    Wines eigene Werkzeuge für diesen Prefix (<code>{prefixPath}</code>) — nützlich, um ein
    Problem direkt zu untersuchen, statt nur ein Spiel neu zu starten.
  </p>

  <label>
    Runner (zum Ausführen des Werkzeugs)
    <select bind:value={runnerId}>
      <option value="" disabled selected>Runner wählen</option>
      {#each $runners as runner (runner.id)}
        <option value={runner.id}>{runner.name} ({runner.kind})</option>
      {/each}
    </select>
  </label>

  <div class="tool-list">
    {#each WINE_TOOLS as tool (tool.id)}
      <div class="tool-row">
        <div>
          <span class="tool-label">{tool.label}</span>
          <p class="tool-desc">{tool.description}</p>
        </div>
        <button
          type="button"
          class="ghost"
          disabled={!runnerId || launching !== ""}
          onclick={() => handleLaunch(tool.id)}
        >
          {launching === tool.id ? "Öffnet…" : "Öffnen"}
        </button>
      </div>
    {/each}
  </div>

  {#if error}
    <div class="toast">
      <span>{error}</span>
    </div>
  {/if}
</div>

<style>
  .wine-tools {
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

  .tool-list {
    display: flex;
    flex-direction: column;
    gap: 0.5em;
  }

  .tool-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1em;
    padding: 0.6em 0.7em;
    border-radius: 8px;
    background: var(--surface-raised);
    border: 1px solid var(--border);
  }

  .tool-label {
    font-weight: 600;
  }

  .tool-desc {
    color: var(--text-muted);
    font-size: 0.82em;
    margin: 0.2em 0 0;
  }

  .toast {
    background: var(--danger-bg);
    color: var(--danger);
    border-radius: 8px;
    padding: 0.5em 0.7em;
    font-size: 0.85em;
  }
</style>
