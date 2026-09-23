<script lang="ts">
  import "../app.css";
  import type { Snippet } from "svelte";
  import { getLocale } from "$lib/i18n/index.svelte";

  let { children }: { children: Snippet } = $props();

  $effect(() => {
    document.documentElement.lang = getLocale();
  });
</script>

{@render children()}
