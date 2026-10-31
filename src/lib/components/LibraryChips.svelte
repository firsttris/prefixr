<!--
  The library's collections as chips: all games, favorites, each of the
  user's categories, the games a test found to work, have issues or be
  broken, and the hidden games (those only once there are any). A new
  category is named right in the row; renaming and deleting happen in the
  dialog behind the gear.
-->
<script lang="ts">
  import * as m from "#lib/paraglide/messages.js";
  import type { Category } from "#lib/types.ts";
  import type { Collection } from "#lib/libraryView.ts";
  import { get } from "svelte/store";
  import { createCategory, libraryMeta } from "#lib/stores/libraryMeta.ts";
  import { TEST_STATUSES, TEST_SYMBOL, testLabel } from "#lib/testStatus.ts";
  import { backendError } from "#lib/i18n/index.svelte.ts";
  import CategoryManager from "./CategoryManager.svelte";

  let {
    collection = $bindable(),
    counts,
    categories,
  }: {
    collection: Collection;
    counts: Record<Collection, number>;
    categories: Category[];
  } = $props();

  let adding = $state(false);
  let newName = $state("");
  let error = $state<unknown>(null);
  let managing = $state(false);

  async function add(event: SubmitEvent) {
    event.preventDefault();
    const name = newName.trim();
    if (!name) {
      adding = false;
      return;
    }
    try {
      await createCategory(name);
      const created = get(libraryMeta).categories.find((c) => c.name === name);
      newName = "";
      adding = false;
      error = null;
      if (created) collection = `category:${created.id}`;
    } catch (e) {
      error = e;
    }
  }

  function focus(node: HTMLInputElement) {
    node.focus();
  }
</script>

{#snippet chip(value: Collection, label: string, icon?: string, iconClass = "")}
  <button
    type="button"
    class="chip"
    class:active={collection === value}
    aria-pressed={collection === value}
    onclick={() => (collection = value)}
  >
    {#if icon}<span class="icon {iconClass}" aria-hidden="true">{icon}</span>{/if}
    {label}
    <span class="count">{counts[value] ?? 0}</span>
  </button>
{/snippet}

<div class="chips" role="group" aria-label={m.gameList_chips_label()}>
  {@render chip("all", m.gameList_chips_all())}
  {@render chip("installed", m.gameList_chips_installed(), "✓")}
  {@render chip("favorites", m.gameList_chips_favorites(), "★")}
  {#each categories as category (category.id)}
    {@render chip(`category:${category.id}`, category.name)}
  {/each}
  {#each TEST_STATUSES as status (status)}
    {#if (counts[`tested:${status}`] ?? 0) > 0 || collection === `tested:${status}`}
      {@render chip(`tested:${status}`, testLabel(status), TEST_SYMBOL[status], `test ${status}`)}
    {/if}
  {/each}
  {#if (counts.hidden ?? 0) > 0}
    {@render chip("hidden", m.gameList_chips_hidden(), "◌")}
  {/if}

  {#if adding}
    <form class="new" onsubmit={add}>
      <input
        use:focus
        bind:value={newName}
        placeholder={m.gameList_chips_newCategoryPlaceholder()}
        aria-label={m.gameList_chips_newCategoryPlaceholder()}
        onkeydown={(e) => {
          if (e.key === "Escape") {
            adding = false;
            newName = "";
            error = null;
          }
        }}
        onblur={() => {
          if (!newName.trim()) adding = false;
        }}
      />
      <button type="submit" class="primary">{m.categories_add()}</button>
    </form>
  {:else}
    <button type="button" class="chip add" onclick={() => (adding = true)}>
      {m.gameList_chips_newCategory()}
    </button>
  {/if}
  {#if categories.length > 0}
    <button
      type="button"
      class="chip manage"
      aria-label={m.gameList_chips_manage()}
      title={m.gameList_chips_manage()}
      onclick={() => (managing = true)}
    >
      ⚙
    </button>
  {/if}
</div>
{#if error}
  <p class="error">{backendError(error)}</p>
{/if}

<CategoryManager open={managing} {categories} onClose={() => (managing = false)} />

<style>
  .chips {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.4em;
    margin-bottom: 1.3em;
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: 0.35em;
    padding: 0.35em 0.85em;
    font-size: 0.85em;
    border-radius: 999px;
    background: var(--surface);
    color: var(--text-muted);
  }

  .chip.active {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
  }

  .chip.active:hover:not(:disabled) {
    background: var(--accent-hover);
  }

  .chip.add,
  .chip.manage {
    border-style: dashed;
    background: transparent;
  }

  .icon {
    font-size: 0.95em;
  }

  .icon.test {
    font-weight: 700;
  }

  .icon.works {
    color: var(--success);
  }

  .icon.issues {
    color: #f5b43c;
  }

  .icon.broken {
    color: var(--danger);
  }

  .chip.active .icon.test {
    color: inherit;
  }

  .count {
    font-size: 0.85em;
    opacity: 0.75;
  }

  .new {
    display: flex;
    gap: 0.3em;
  }

  .new input {
    width: 12em;
    padding: 0.3em 0.7em;
    font-size: 0.85em;
    border-radius: 999px;
  }

  .new button {
    padding: 0.3em 0.8em;
    font-size: 0.82em;
    border-radius: 999px;
  }

  .error {
    color: var(--danger);
    font-size: 0.85em;
    margin: -0.8em 0 1em;
  }
</style>
