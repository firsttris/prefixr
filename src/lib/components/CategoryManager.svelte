<!-- Renames and deletes the library's categories. -->
<script lang="ts">
  import * as m from "#lib/paraglide/messages.js";
  import type { Category } from "#lib/types.ts";
  import { createCategory, deleteCategory, renameCategory } from "#lib/stores/libraryMeta.ts";
  import { backendError } from "#lib/i18n/index.svelte.ts";
  import Modal from "./Modal.svelte";
  import ConfirmDialog from "./ConfirmDialog.svelte";

  let {
    open,
    categories,
    onClose,
  }: { open: boolean; categories: Category[]; onClose: () => void } = $props();

  let renaming = $state<string | null>(null);
  let renameTo = $state("");
  let newName = $state("");
  let deleting = $state<Category | null>(null);
  let error = $state<unknown>(null);

  async function attempt(action: () => Promise<void>) {
    try {
      await action();
      error = null;
    } catch (e) {
      error = e;
    }
  }
</script>

<Modal {open} title={m.categories_title()} {onClose}>
  <p class="intro">{m.categories_intro()}</p>

  {#if categories.length === 0}
    <p class="muted">{m.categories_empty()}</p>
  {:else}
    <ul class="categories">
      {#each categories as category (category.id)}
        <li>
          {#if renaming === category.id}
            <form
              class="rename"
              onsubmit={(e) => {
                e.preventDefault();
                attempt(async () => {
                  await renameCategory(category.id, renameTo);
                  renaming = null;
                });
              }}
            >
              <input bind:value={renameTo} aria-label={m.categories_rename()} />
              <button type="submit" class="primary">{m.categories_save()}</button>
              <button type="button" class="ghost" onclick={() => (renaming = null)}
                >{m.common_cancel()}</button
              >
            </form>
          {:else}
            <span class="name">{category.name}</span>
            <span class="muted">{m.categories_gameCount({ count: category.games.length })}</span>
            <span class="actions">
              <button
                type="button"
                class="ghost"
                onclick={() => {
                  renaming = category.id;
                  renameTo = category.name;
                }}>{m.categories_rename()}</button
              >
              <button type="button" class="ghost danger-text" onclick={() => (deleting = category)}
                >{m.categories_delete()}</button
              >
            </span>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}

  <form
    class="add"
    onsubmit={(e) => {
      e.preventDefault();
      attempt(async () => {
        await createCategory(newName);
        newName = "";
      });
    }}
  >
    <input
      bind:value={newName}
      placeholder={m.gameList_chips_newCategoryPlaceholder()}
      aria-label={m.gameList_chips_newCategoryPlaceholder()}
    />
    <button type="submit" class="primary" disabled={!newName.trim()}>{m.categories_add()}</button>
  </form>

  {#if error}
    <p class="error">{backendError(error)}</p>
  {/if}
</Modal>

<ConfirmDialog
  open={deleting !== null}
  title={m.categories_deleteConfirmTitle()}
  message={deleting ? m.categories_deleteConfirmMessage({ name: deleting.name }) : ""}
  confirmLabel={m.categories_delete()}
  onConfirm={() => {
    const category = deleting;
    deleting = null;
    if (category) attempt(() => deleteCategory(category.id));
  }}
  onCancel={() => (deleting = null)}
/>

<style>
  .intro {
    color: var(--text-muted);
    font-size: 0.9em;
    margin-bottom: 1em;
  }

  .muted {
    color: var(--text-muted);
    font-size: 0.85em;
  }

  .categories {
    list-style: none;
    margin: 0 0 1em;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.4em;
  }

  li {
    display: flex;
    align-items: center;
    gap: 0.7em;
    padding: 0.4em 0.6em;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface-raised);
  }

  .name {
    font-weight: 600;
  }

  .actions {
    margin-left: auto;
    display: flex;
    gap: 0.2em;
  }

  .actions button {
    padding: 0.3em 0.6em;
    font-size: 0.85em;
  }

  .danger-text {
    color: var(--danger);
  }

  .rename,
  .add {
    display: flex;
    gap: 0.4em;
    flex: 1;
  }

  .rename input,
  .add input {
    flex: 1;
  }

  .error {
    color: var(--danger);
    font-size: 0.85em;
    margin-top: 0.6em;
  }
</style>
