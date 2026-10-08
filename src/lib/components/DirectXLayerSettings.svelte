<script lang="ts">
  import * as m from "$lib/paraglide/messages";
  import { onMount } from "svelte";
  import {
    directXLayers,
    refreshDirectXLayers,
    updateDirectXLayers,
  } from "$lib/stores/directxLayers";
  import { backendError } from "$lib/i18n/index.svelte";

  let updating = $state(false);
  let error = $state<unknown>(null);
  let updated = $state(false);

  let anyInstalled = $derived($directXLayers.some((layer) => layer.installed));

  onMount(() => {
    refreshDirectXLayers().catch((e) => (error = e));
  });

  async function handleUpdate() {
    updating = true;
    error = null;
    updated = false;
    try {
      await updateDirectXLayers();
      updated = true;
    } catch (e) {
      error = e;
    } finally {
      updating = false;
    }
  }
</script>

<details class="layers">
  <summary>
    DXVK / VKD3D-Proton
    {#each $directXLayers as layer (layer.label)}
      <span class="status">
        {layer.label}
        {layer.installed
          ? (layer.version ?? m.directXLayers_installed())
          : m.directXLayers_notInstalled()}
      </span>
    {/each}
  </summary>
  <p class="hint">{m.directXLayers_hint()}</p>

  {#if anyInstalled}
    <div class="row">
      <button type="button" class="ghost" disabled={updating} onclick={handleUpdate}>
        {updating ? m.directXLayers_updating() : m.directXLayers_update()}
      </button>
    </div>
  {/if}

  {#if error}
    <p class="error">{backendError(error)}</p>
  {:else if updated}
    <p class="saved-hint">{m.directXLayers_updated()}</p>
  {/if}
</details>

<style>
  .layers {
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
