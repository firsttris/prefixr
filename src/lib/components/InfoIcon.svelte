<script lang="ts">
  let { text }: { text: string } = $props();

  let open = $state(false);
  let root: HTMLElement | undefined;

  function toggle() {
    open = !open;
  }

  function handleWindowClick(e: MouseEvent) {
    if (open && root && !root.contains(e.target as Node)) {
      open = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") open = false;
  }
</script>

<svelte:window onclick={handleWindowClick} onkeydown={handleKeydown} />

<span class="info-wrap" bind:this={root}>
  <button
    type="button"
    class="info-icon"
    class:active={open}
    onclick={toggle}
    aria-expanded={open}
    aria-label="Mehr Informationen"
  >
    ⓘ
  </button>
  {#if open}
    <div class="info-popup" role="tooltip">{text}</div>
  {/if}
</span>

<style>
  .info-wrap {
    position: relative;
    display: inline-block;
  }

  .info-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 1.4em;
    height: 1.4em;
    margin-left: 0.3em;
    padding: 0;
    border: none;
    border-radius: 50%;
    background: transparent;
    color: var(--text-muted);
    font-size: 0.85em;
    line-height: 1;
    vertical-align: middle;
    cursor: pointer;
    transition:
      background-color 0.15s,
      color 0.15s;
  }

  .info-icon:hover,
  .info-icon.active {
    color: var(--accent);
    background: var(--surface-raised);
  }

  .info-popup {
    position: absolute;
    top: calc(100% + 0.4em);
    left: 0;
    z-index: 20;
    width: max-content;
    max-width: 280px;
    padding: 0.65em 0.85em;
    border-radius: 8px;
    background: var(--surface-raised);
    border: 1px solid var(--border);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.3);
    color: var(--text);
    font-size: 0.8em;
    font-weight: 400;
    line-height: 1.45;
  }
</style>
