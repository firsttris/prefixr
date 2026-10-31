<script lang="ts">
  import { onMount } from "svelte";
  import * as m from "#lib/paraglide/messages.js";
  import { loadMissingExtensions, missingExtensions } from "#lib/stores/flatpakExtensions.ts";

  // Shown below an enabled tool whose Flathub extension the Flatpak lacks.
  let { tool, name }: { tool: "gamescope" | "mangohud" | "vkbasalt"; name: string } = $props();

  onMount(() => {
    loadMissingExtensions().catch(() => {});
  });

  const missing = $derived($missingExtensions.find((extension) => extension.tool === tool));
</script>

{#if missing?.command}
  <p class="hint">
    {m.flatpakExtension_hint({ tool: name })}
    <code>{missing.command}</code>
  </p>
{:else if missing}
  <p class="hint">{m.flatpakExtension_unavailable({ tool: name })}</p>
{/if}

<style>
  .hint {
    color: var(--text-muted);
    font-size: 0.85em;
    max-width: 60ch;
    margin: 0.6em 0;
  }
  code {
    display: block;
    margin-top: 0.3em;
    user-select: all;
    word-break: break-all;
  }
</style>
