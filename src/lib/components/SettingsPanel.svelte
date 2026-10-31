<script lang="ts">
  import * as m from "#lib/paraglide/messages.js";
  import type { Snippet } from "svelte";

  // Frame for a global settings page: heading, intro, the category's
  // editor, and a save button for the page's draft.
  let {
    title,
    hint,
    saving,
    saved,
    error,
    onSave,
    children,
  }: {
    title: string;
    hint: string;
    saving: boolean;
    saved: boolean;
    error: string;
    onSave: () => void;
    children: Snippet;
  } = $props();
</script>

<section class="panel">
  <div>
    <h2>{title}</h2>
    <p class="hint">{hint}</p>
  </div>

  {@render children()}

  {#if error}
    <p class="error">{error}</p>
  {/if}

  <div class="save-row">
    <button type="button" class="primary" disabled={saving} onclick={onSave}>
      {saving ? m.common_saving() : m.settingsPanel_saveChanges()}
    </button>
    {#if saved}
      <span class="saved-hint">{m.common_saved()}</span>
    {/if}
  </div>
</section>

<style>
  .panel {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 1.2em;
    display: flex;
    flex-direction: column;
    gap: 1.4em;
    max-width: var(--page-max);
    /* The editors inside lay themselves out in columns once the panel is wide. */
    container: settings-panel / inline-size;
  }

  .hint {
    color: var(--text-muted);
    font-size: 0.85em;
    margin-top: 0.3em;
    max-width: 60ch;
  }

  .error {
    color: var(--danger);
  }

  .save-row {
    display: flex;
    align-items: center;
    gap: 1em;
  }

  .saved-hint {
    color: var(--text-muted);
    font-size: 0.85em;
  }
</style>
