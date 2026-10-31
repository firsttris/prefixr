<script lang="ts" module>
  import * as m from "#lib/paraglide/messages.js";

  export function tierName(tier: string): string {
    switch (tier) {
      case "platinum":
        return m.compat_tier_platinum();
      case "gold":
        return m.compat_tier_gold();
      case "silver":
        return m.compat_tier_silver();
      case "bronze":
        return m.compat_tier_bronze();
      case "borked":
        return m.compat_tier_borked();
      default:
        return tier;
    }
  }
</script>

<!--
  What's known about how well a game runs, as small badges on its card or
  row: its ProtonDB rating and whether umu-protonfixes has fixes for it.
-->
<script lang="ts">
  import type { Compatibility } from "#lib/types.ts";

  let { compat }: { compat?: Compatibility } = $props();
</script>

{#if compat}
  <span class="badges">
    {#if compat.protondb_tier}
      <span
        class="tier tier-{compat.protondb_tier}"
        title={m.compat_tierTitle({ tier: tierName(compat.protondb_tier) })}
      >
        {tierName(compat.protondb_tier)}
      </span>
    {/if}
    {#if compat.umu_id}
      <span class="umu" title={m.compat_umuTitle()}>{m.compat_umuBadge()}</span>
    {/if}
  </span>
{/if}

<style>
  .badges {
    display: inline-flex;
    flex-wrap: wrap;
    gap: 0.3em;
  }

  .tier,
  .umu {
    font-size: 0.68em;
    font-weight: 600;
    padding: 0.12em 0.55em;
    border-radius: 999px;
    border: 1px solid transparent;
    white-space: nowrap;
  }

  /* ProtonDB's own tier colors, softened for the dark theme. */
  .tier-platinum {
    color: #e5e4e2;
    background: rgba(180, 199, 220, 0.22);
    border-color: rgba(180, 199, 220, 0.45);
  }

  .tier-gold {
    color: #f5d36b;
    background: rgba(207, 181, 59, 0.2);
    border-color: rgba(207, 181, 59, 0.45);
  }

  .tier-silver {
    color: #cfd3d8;
    background: rgba(166, 166, 166, 0.2);
    border-color: rgba(166, 166, 166, 0.4);
  }

  .tier-bronze {
    color: #e3a76f;
    background: rgba(205, 127, 50, 0.2);
    border-color: rgba(205, 127, 50, 0.45);
  }

  .tier-borked {
    color: var(--danger);
    background: var(--danger-bg);
    border-color: rgba(239, 83, 80, 0.4);
  }

  .umu {
    color: var(--success);
    background: var(--success-bg);
    border-color: rgba(62, 207, 142, 0.35);
  }
</style>
