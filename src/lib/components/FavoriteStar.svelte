<!--
  Marks a game as a favorite and shows that it is one. On a card it floats
  on the cover, shown on hover or focus unless the game is a favorite.
-->
<script lang="ts">
  import * as m from "$lib/paraglide/messages";

  let {
    favorite,
    onToggle,
    onImage = false,
  }: { favorite: boolean; onToggle: () => void; onImage?: boolean } = $props();
</script>

<button
  type="button"
  class="star"
  class:on={favorite}
  class:on-image={onImage}
  aria-pressed={favorite}
  aria-label={favorite ? m.favorite_remove() : m.favorite_add()}
  title={favorite ? m.favorite_remove() : m.favorite_add()}
  onclick={(e) => {
    e.stopPropagation();
    onToggle();
  }}
>
  <svg viewBox="0 0 20 20" aria-hidden="true">
    <path
      d="M10 2.8l2.2 4.6 5 .7-3.6 3.5.9 5-4.5-2.4-4.5 2.4.9-5L2.8 8.1l5-.7L10 2.8z"
      fill={favorite ? "currentColor" : "none"}
      stroke="currentColor"
      stroke-width="1.4"
      stroke-linejoin="round"
    />
  </svg>
</button>

<style>
  .star {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 2em;
    height: 2em;
    padding: 0;
    border-radius: 8px;
    background: transparent;
    border: 1px solid transparent;
    color: var(--text-muted);
    flex-shrink: 0;
  }

  .star svg {
    width: 1.15em;
    height: 1.15em;
  }

  .star.on {
    color: #f5c451;
  }

  .star:hover:not(:disabled) {
    color: #f5c451;
    border-color: var(--border);
  }

  .star.on-image {
    background: rgba(15, 16, 20, 0.55);
    border-color: rgba(255, 255, 255, 0.18);
    backdrop-filter: blur(8px);
    color: #fff;
  }

  .star.on-image.on {
    color: #f5c451;
  }
</style>
