<script lang="ts">
  import * as m from "$lib/paraglide/messages";
  import RunnerSelect from "$lib/components/RunnerSelect.svelte";
  import Message from "$lib/components/Message.svelte";
  import { msgGroup } from "$lib/i18n/msg-groups";
  import { onMount, untrack } from "svelte";
  import { prefixRunner, refreshRunners } from "$lib/stores/runners";
  import { games } from "$lib/stores/games";
  import { WINE_TOOLS, launchWineTool } from "$lib/stores/wineTools";
  import { backendError, pickMsg } from "$lib/i18n/index.svelte";

  let { prefixPath }: { prefixPath: string } = $props();

  // Started with the runner the prefix's games use, if they agree on one.
  let runnerId = $state(untrack(() => prefixRunner($games, prefixPath)));
  let launching = $state("");
  let error = $state<unknown>(null);

  onMount(() => {
    refreshRunners().catch((e) => (error = e));
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

{#snippet prefix()}<code>{prefixPath}</code>{/snippet}

<div class="wine-tools">
  <p class="hint">
    <Message message={m.wineToolsLauncher_hint} parts={{ prefix }} />
  </p>

  <label>
    {m.wineToolsLauncher_runnerLabel()}
    <RunnerSelect bind:value={runnerId} />
  </label>

  <div class="tool-list">
    {#each WINE_TOOLS as tool (tool.id)}
      <div class="tool-row">
        <div>
          <span class="tool-label">{pickMsg(msgGroup.wineTools_label, tool.id)}</span>
          <p class="tool-desc">{pickMsg(msgGroup.wineTools_description, tool.id)}</p>
        </div>
        <button
          type="button"
          class="ghost"
          disabled={!runnerId || launching !== ""}
          onclick={() => handleLaunch(tool.id)}
        >
          {launching === tool.id ? m.wineToolsLauncher_opening() : m.wineToolsLauncher_open()}
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
