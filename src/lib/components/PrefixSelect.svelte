<!--
  The prefix picker, as every form that puts a game in a prefix has it.
  With an agent's prefix per game there are easily a couple of hundred, all
  of them a long path that differs only near its end. So each is named after
  the games in it (an unused one by its shortened path), the unused ones,
  the likely pick for a new game, come first, and the rest are by name, for
  typing a game's name in the open list to jump to its prefix.
  `first` renders the options before them (a placeholder, "new prefix").
-->
<script lang="ts">
  import * as m from "$lib/paraglide/messages";
  import { onMount, type Snippet } from "svelte";
  import { get } from "svelte/store";
  import { prefixes } from "$lib/stores/prefixes";
  import { games, refreshGames } from "$lib/stores/games";
  import { listedPath, pickerFallback, prefixOptions } from "$lib/prefixView";

  let {
    value = $bindable(),
    disabled = false,
    invalid = false,
    first,
    beside,
  }: {
    value: string;
    disabled?: boolean;
    invalid?: boolean;
    first?: Snippet;
    // Next to the list, e.g. a button for a new prefix.
    beside?: Snippet;
  } = $props();

  let options = $derived(prefixOptions($prefixes, $games));
  let unused = $derived(options.filter((o) => !o.used));
  let used = $derived(options.filter((o) => o.used));
  let fallback = $derived(pickerFallback(value, options));

  // A listed prefix written another way (a trailing slash) is taken as the
  // list spells it, so its option shows as chosen: to the backend it's the
  // same folder.
  $effect(() => {
    const listed = value ? listedPath(value, $prefixes) : null;
    if (listed && listed !== value) value = listed;
  });

  // The names come from the library's games; without them (a dialog opened
  // before the library was shown) every prefix would pass as unused.
  onMount(() => {
    if (get(games).length === 0) refreshGames().catch(() => {});
  });
</script>

<div class="prefix-select">
  <div class="select-row">
    <select bind:value {disabled} aria-invalid={invalid}>
      {@render first?.()}
      {#if unused.length > 0}
        <optgroup label={m.prefixSelect_unusedGroup({ count: unused.length })}>
          {#each unused as option (option.path)}
            <option value={option.path} title={option.path}>{option.label}</option>
          {/each}
        </optgroup>
      {/if}
      {#if used.length > 0}
        <optgroup label={m.prefixSelect_usedGroup({ count: used.length })}>
          {#each used as option (option.path)}
            <option value={option.path} title={option.path}>{option.label}</option>
          {/each}
        </optgroup>
      {/if}
      <!-- A prefix removed from the list is still the game's own. -->
      {#if fallback}
        <option value={fallback}>{fallback}</option>
      {/if}
    </select>
    {@render beside?.()}
  </div>
  <!-- The options leave the path out, so the chosen one's is shown here. -->
  {#if value}
    <span class="path">{value}</span>
  {/if}
</div>

<style>
  .prefix-select {
    display: flex;
    flex-direction: column;
    gap: 0.3em;
    flex: 1;
    min-width: 0;
  }

  .select-row {
    display: flex;
    gap: 0.5em;
  }

  .select-row select {
    flex: 1;
    min-width: 0;
  }

  .path {
    color: var(--text-muted);
    font-family: monospace;
    font-size: 0.78em;
    word-break: break-all;
  }
</style>
