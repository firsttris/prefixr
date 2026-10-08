<!-- A game controller, for a game with neither cover nor exe icon. -->
<script lang="ts">
  let { class: className = "" }: { class?: string } = $props();
</script>

<svg class={className} viewBox="0 0 64 40" aria-hidden="true">
  <path
    fill="#64748b"
    d="M19 6h26a12 12 0 0 1 12 12v5a7 7 0 0 1-12.5 4.5L40 24H24l-4.5 3.5A7 7 0 0 1 7 23v-5A12 12 0 0 1 19 6z"
  />
  <ellipse cx="32" cy="11" rx="16" ry="4" fill="#fff" opacity="0.1" />
  <rect x="15.5" y="11" width="3" height="12" rx="1.5" fill="#e2e8f0" />
  <rect x="11" y="15.5" width="12" height="3" rx="1.5" fill="#e2e8f0" />
  <circle cx="47" cy="12" r="2.2" fill="#22c55e" />
  <circle cx="52" cy="17" r="2.2" fill="#ef4444" />
  <circle cx="47" cy="22" r="2.2" fill="#eab308" />
  <circle cx="42" cy="17" r="2.2" fill="#3b82f6" />
</svg>
