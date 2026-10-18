<script lang="ts">
  import * as m from "$lib/paraglide/messages";
  import type { Snippet } from "svelte";
  import { isTopLayer, openLayer, trapTab } from "$lib/layers";

  let {
    open,
    title,
    onClose,
    wide = false,
    closable = true,
    children,
  }: {
    open: boolean;
    title: string;
    onClose: () => void;
    // For content with side-by-side layouts, like the game settings tabs.
    wide?: boolean;
    // False while the content is in the middle of something it has to
    // see through (e.g. a running installer): Escape, a backdrop click and
    // the close button then do nothing.
    closable?: boolean;
    children: Snippet;
  } = $props();

  function close() {
    if (closable) onClose();
  }

  const titleId = $props.id();
  let panel = $state<HTMLDivElement>();

  // Focus moves into the dialog when it opens, and back to whatever had it
  // (usually the button that opened it) when it closes.
  $effect(() => {
    if (!panel) return;
    const opener = document.activeElement;
    panel.focus();
    return () => {
      if (opener instanceof HTMLElement && opener.isConnected) opener.focus();
    };
  });

  // Only the topmost dialog answers Escape and Tab (see `layers`).
  let layer: ReturnType<typeof openLayer> | null = null;
  $effect(() => {
    if (!open) return;
    layer = openLayer();
    return () => {
      layer?.close();
      layer = null;
    };
  });

  function handleKeydown(event: KeyboardEvent) {
    if (!layer || !isTopLayer(layer.id)) return;
    if (event.key === "Escape") {
      close();
      return;
    }
    if (panel) trapTab(event, panel);
  }
</script>

<svelte:window onkeydown={open ? handleKeydown : undefined} />

{#if open}
  <!-- Only a click on the backdrop itself closes, not one inside the panel. -->
  <div
    class="backdrop"
    onclick={(e) => e.target === e.currentTarget && close()}
    role="presentation"
  >
    <div
      bind:this={panel}
      class="panel"
      class:wide
      role="dialog"
      aria-modal="true"
      aria-labelledby={titleId}
      tabindex="-1"
    >
      <header>
        <h2 id={titleId}>{title}</h2>
        <button
          type="button"
          class="ghost"
          onclick={close}
          disabled={!closable}
          aria-label={m.common_close()}>✕</button
        >
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
    outline: none;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    width: min(480px, 100%);
    /* As tall as the window allows; only the body scrolls, so the title
       and close button stay in view however long the content is. */
    max-height: 100%;
    display: flex;
    flex-direction: column;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
  }

  .panel.wide {
    width: min(760px, 100%);
  }

  header {
    flex-shrink: 0;
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1em 1.2em;
    border-bottom: 1px solid var(--border);
  }

  .body {
    padding: 1.2em;
    overflow-y: auto;
    min-height: 0;
  }
</style>
