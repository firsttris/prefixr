<!--
  A game's test result as a small round symbol: ✓ works, ! issues,
  ✕ broken. Faded once the game has another runner or exe than it was
  tested with. The whole result, its note too, is its tooltip.
-->
<script lang="ts">
  import type { GameTest } from "#lib/stores/libraryMeta.ts";
  import { TEST_SYMBOL, testSummary } from "#lib/testStatus.ts";

  let { test, onImage = false }: { test: GameTest; onImage?: boolean } = $props();

  let summary = $derived(testSummary(test));
</script>

<span
  class="test-dot {test.result.status}"
  class:outdated={!test.current}
  class:on-image={onImage}
  role="img"
  aria-label={summary}
  title={summary}>{TEST_SYMBOL[test.result.status]}</span
>

<style>
  .test-dot {
    display: inline-grid;
    place-items: center;
    flex-shrink: 0;
    width: 1.35em;
    height: 1.35em;
    border-radius: 50%;
    font-size: 0.8em;
    font-weight: 700;
    line-height: 1;
    cursor: default;
  }

  .test-dot.on-image {
    box-shadow: 0 0 0 2px rgba(14, 15, 19, 0.8);
  }

  .works {
    background: var(--success);
    color: #08130d;
  }

  .issues {
    background: #f5b43c;
    color: #1d1404;
  }

  .broken {
    background: var(--danger);
    color: #fff;
  }

  .outdated {
    opacity: 0.45;
  }
</style>
