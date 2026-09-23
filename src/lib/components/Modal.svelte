<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    open,
    title,
    onClose,
    wide = false,
    children,
  }: {
    open: boolean;
    title: string;
    onClose: () => void;
    // For content with side-by-side layouts, like the game settings tabs.
    wide?: boolean;
    children: Snippet;
  } = $props();

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") onClose();
  }
</script>

<svelte:window onkeydown={open ? handleKeydown : undefined} />

{#if open}
  <div class="backdrop" onclick={onClose} role="presentation">
    <div
      class="panel"
      class:wide
      role="dialog"
      aria-modal="true"
      aria-label={title}
      tabindex="-1"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
    >
      <header>
        <h2>{title}</h2>
        <button type="button" class="ghost" onclick={onClose} aria-label="Schließen">✕</button>
      </header>
      <div class="body">
        {@render children()}
      </div>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.55);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
    padding: 1em;
  }

  .panel {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    width: min(480px, 100%);
    max-height: 85vh;
    overflow-y: auto;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
  }

  .panel.wide {
    width: min(760px, 100%);
  }

  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1em 1.2em;
    border-bottom: 1px solid var(--border);
  }

  .body {
    padding: 1.2em;
  }
</style>
