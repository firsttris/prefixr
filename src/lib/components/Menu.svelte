<!--
  A dropdown menu behind a trigger button, with the keyboard behavior the
  menu role promises: it opens with focus on its first entry, the arrow
  keys, Home and End move between entries, Escape closes it (back to the
  trigger), and Tab or a click anywhere else closes it without being
  swallowed. It opens downward, or upward when there's no room below.
  `items` renders the entries (role="menuitem"); call the `close` it gets
  before acting on one.
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
  let anchorEl: HTMLDivElement;
  let triggerEl: HTMLButtonElement;
  let menuEl = $state<HTMLDivElement>();

  function entries(): HTMLElement[] {
    return [...(menuEl?.querySelectorAll<HTMLElement>('[role="menuitem"]:not(:disabled)') ?? [])];
  }

  // Focus goes back to the trigger when it was in the menu, so a dialog an
  // entry opens returns it there too.
  function close() {
    if (menuEl?.contains(document.activeElement)) triggerEl.focus();
    open = false;
  }

  $effect(() => {
    if (!menuEl) return;
    upward = false;
    upward = menuEl.getBoundingClientRect().bottom > window.innerHeight;
    entries()[0]?.focus();
  });

  function handleMenuKeydown(event: KeyboardEvent) {
    const list = entries();
    const index = list.indexOf(document.activeElement as HTMLElement);
    const move = (to: number) => {
      event.preventDefault();
      list[(to + list.length) % list.length]?.focus();
    };
    switch (event.key) {
      case "ArrowDown":
        return move(index + 1);
      case "ArrowUp":
        return move(index - 1);
      case "Home":
        return move(0);
      case "End":
        return move(list.length - 1);
      case "Escape":
        event.preventDefault();
        event.stopPropagation();
        return close();
      case "Tab":
        open = false;
    }
  }

  function handleWindowPointerdown(event: PointerEvent) {
    if (!anchorEl.contains(event.target as Node)) open = false;
  }
</script>

<svelte:window onpointerdown={open ? handleWindowPointerdown : undefined} />

<div class="menu-anchor" bind:this={anchorEl}>
  <button
    bind:this={triggerEl}
    type="button"
    class={triggerClass}
    onclick={() => (open ? close() : (open = true))}
    {disabled}
    aria-haspopup="menu"
    aria-expanded={open}
    aria-label={label}
    title={label}
  >
    {@render trigger()}
  </button>
  {#if open}
    <div
      bind:this={menuEl}
      class="menu"
      class:menu-up={upward}
      role="menu"
      tabindex="-1"
      aria-label={label}
      onkeydown={handleMenuKeydown}
    >
      {@render items(close)}
    </div>
  {/if}
</div>

<style>
  .menu-anchor {
    position: relative;
    display: flex;
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
