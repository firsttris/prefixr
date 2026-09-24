<script lang="ts">
  import { onMount } from "svelte";
  import { runners, refreshRunners } from "$lib/stores/runners";
  import { WINE_TOOLS, launchWineTool } from "$lib/stores/wineTools";
  import { backendError, t, type TranslationKey } from "$lib/i18n/index.svelte";

  let { prefixPath }: { prefixPath: string } = $props();

  let runnerId = $state("");
  let launching = $state("");
  let error = $state<unknown>(null);

  onMount(() => {
    refreshRunners();
  });

  async function handleLaunch(tool: string) {
    if (!runnerId) return;
    launching = tool;
    error = null;
    try {
      await launchWineTool(prefixPath, runnerId, tool);
    } catch (e) {
      error = e;
    } finally {
      launching = "";
    }
  }
</script>

<div class="wine-tools">
  <p class="hint">
    {t("wineToolsLauncher.hintBefore")}<code>{prefixPath}</code>{t("wineToolsLauncher.hintAfter")}
  </p>

  <label>
    {t("wineToolsLauncher.runnerLabel")}
    <select bind:value={runnerId}>
      <option value="" disabled selected>{t("gameForm.runnerChoose")}</option>
      {#each $runners as runner (runner.id)}
        <option value={runner.id}>{runner.name} ({runner.kind})</option>
      {/each}
    </select>
  </label>

  <div class="tool-list">
    {#each WINE_TOOLS as tool (tool.id)}
      <div class="tool-row">
        <div>
          <span class="tool-label">{t(`wineTools.${tool.id}.label` as TranslationKey)}</span>
          <p class="tool-desc">{t(`wineTools.${tool.id}.description` as TranslationKey)}</p>
        </div>
        <button
          type="button"
          class="ghost"
          disabled={!runnerId || launching !== ""}
          onclick={() => handleLaunch(tool.id)}
        >
          {launching === tool.id ? t("wineToolsLauncher.opening") : t("wineToolsLauncher.open")}
        </button>
      </div>
    {/each}
  </div>

  {#if error}
    <div class="toast">
      <span>{backendError(error)}</span>
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
