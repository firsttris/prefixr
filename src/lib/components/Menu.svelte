<!--
  A dropdown menu behind a trigger button. It opens downward, or upward when
  there's no room below. `items` renders the entries; call the `close` it
  gets before acting on one.
-->
<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    label,
    triggerClass = "icon-btn",
    disabled = false,
    trigger,
    items,
  }: {
    label: string;
    triggerClass?: string;
    disabled?: boolean;
    trigger: Snippet;
    items: Snippet<[close: () => void]>;
  } = $props();

  let open = $state(false);
  let upward = $state(false);
  let menuEl = $state<HTMLDivElement>();

  function close() {
    open = false;
  }

  $effect(() => {
    if (!menuEl) return;
    upward = false;
    upward = menuEl.getBoundingClientRect().bottom > window.innerHeight;
  });
</script>

<div class="menu-anchor">
  <button
    type="button"
    class={triggerClass}
    onclick={() => (open = !open)}
    {disabled}
    aria-haspopup="menu"
    aria-expanded={open}
    aria-label={label}
    title={label}
  >
    {@render trigger()}
  </button>
  {#if open}
    <div class="menu-backdrop" onclick={close} role="presentation"></div>
    <div bind:this={menuEl} class="menu" class:menu-up={upward} role="menu">
      {@render items(close)}
    </div>
  {/if}
</div>

<style>
  .menu-anchor {
    position: relative;
    display: flex;
  }

  .menu-backdrop {
    position: fixed;
    inset: 0;
    z-index: 10;
  }

  .menu {
    position: absolute;
    top: calc(100% + 0.3em);
    right: 0;
    z-index: 11;
    display: flex;
    flex-direction: column;
    min-width: 12em;
    background: var(--surface-raised);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 0.3em;
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.4);
  }

  .menu.menu-up {
    top: auto;
    bottom: calc(100% + 0.3em);
  }

  .menu :global([role="menuitem"]) {
    display: flex;
    align-items: center;
    gap: 0.55em;
    text-align: left;
    background: transparent;
    border: none;
    padding: 0.5em 0.6em;
    border-radius: 6px;
    font-size: 0.85em;
    color: var(--text);
    white-space: nowrap;
  }

  .menu :global([role="menuitem"]:hover:not(:disabled)) {
    background: var(--surface);
  }

  .menu :global([role="menuitem"].danger) {
    color: var(--danger);
  }

  .menu :global([role="menuitem"].danger:hover:not(:disabled)) {
    background: var(--danger-bg);
  }

  .menu :global([role="separator"]) {
    height: 1px;
    background: var(--border);
    margin: 0.25em 0.4em;
  }
</style>
