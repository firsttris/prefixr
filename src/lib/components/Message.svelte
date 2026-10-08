<!--
  A message whose placeholders hold markup rather than text — a <code> path,
  a <strong> word, a link button — so the whole sentence stays one message
  and a translation can put them wherever its word order needs. `parts`
  maps each placeholder to the snippet rendered in its place.
-->
<script lang="ts" generics="K extends string">
  import type { Snippet } from "svelte";

  let {
    message,
    parts,
  }: {
    message: (inputs: Record<K, string>) => string;
    parts: Record<K, Snippet>;
  } = $props();

  // Each placeholder is filled with a marker the text around it can't
  // contain (private-use characters), then the message is split at them.
  const segments = $derived.by(() => {
    const keys = Object.keys(parts) as K[];
    const markers = Object.fromEntries(keys.map((key, i) => [key, `\uE000${i}\uE001`]));
    return message(markers as Record<K, string>)
      .split(/\uE000(\d+)\uE001/)
      .map((piece, i) => (i % 2 === 1 ? { key: keys[Number(piece)] } : { text: piece }));
  });
</script>

{#each segments as segment, i (i)}{#if segment.key !== undefined}{@render parts[segment.key]()}{:else}{segment.text}{/if}{/each}
