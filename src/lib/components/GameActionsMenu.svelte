<!--
  A library entry's menu. The card puts every action in it; the list row
  has buttons of its own for editing and removing, and only needs the
  shortcut and Steam entries (`full` off).
-->
<script lang="ts">
  import * as m from "$lib/paraglide/messages";
  import type { Snippet } from "svelte";
  import Menu from "./Menu.svelte";
  import type { GameItem } from "./gameItem.svelte";

  let {
    item,
    inSteam,
    full = false,
    label,
    triggerClass,
    trigger,
    onEdit,
    onEditArtwork,
    onExportToSteam,
    onRemoveFromSteam,
  }: {
    item: GameItem;
    inSteam: boolean;
    full?: boolean;
    label: string;
    triggerClass?: string;
    trigger: Snippet;
    onEdit: () => void;
    onEditArtwork: () => void;
    onExportToSteam: () => void;
    onRemoveFromSteam: () => void;
  } = $props();
</script>

{#snippet linkIcon()}
  <svg class="menu-icon" viewBox="0 0 20 20" fill="none" aria-hidden="true">
    <path d="M8 12l4-4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
    <path
      d="M7.2 13.3L5.6 15a3 3 0 01-4.2-4.2l2.3-2.3a3 3 0 014.2 0"
      stroke="currentColor"
      stroke-width="1.5"
      stroke-linecap="round"
    />
    <path
      d="M12.8 6.7L14.4 5a3 3 0 014.2 4.2l-2.3 2.3a3 3 0 01-4.2 0"
      stroke="currentColor"
      stroke-width="1.5"
      stroke-linecap="round"
    />
  </svg>
{/snippet}

<Menu
  {label}
  {triggerClass}
  disabled={!full && item.shortcutState === "creating"}
  {trigger}
>
  {#snippet items(close)}
    {#if full}
      <button
        type="button"
        role="menuitem"
        onclick={() => {
          close();
          onEditArtwork();
        }}
      >
        <svg class="menu-icon" viewBox="0 0 20 20" fill="none" aria-hidden="true">
          <rect x="3" y="4" width="14" height="12" rx="2" stroke="currentColor" stroke-width="1.5" />
          <circle cx="7.5" cy="8.5" r="1.4" stroke="currentColor" stroke-width="1.5" />
          <path
            d="M4 14.5l4-4 3 3 2-2 3.5 3.5"
            stroke="currentColor"
            stroke-width="1.5"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
        </svg>
        {m.library_selectArtworkTitle()}
      </button>
      <button
        type="button"
        role="menuitem"
        onclick={() => {
          close();
          onEdit();
        }}
      >
        <svg class="menu-icon" viewBox="0 0 20 20" fill="none" aria-hidden="true">
          <path
            d="M13.4 3.6l3 3L6.3 16.7l-3.6.9.9-3.6L13.4 3.6z"
            stroke="currentColor"
            stroke-width="1.5"
            stroke-linejoin="round"
          />
        </svg>
        {m.library_editGameTitle()}
      </button>
      <span class="divider" role="separator"></span>
    {/if}
    <button
      type="button"
      role="menuitem"
      disabled={item.shortcutState === "creating"}
      onclick={() => {
        close();
        item.createShortcut("desktop");
      }}
    >
      {@render linkIcon()}
      {m.gameCard_menuDesktopShortcut()}
    </button>
    <button
      type="button"
      role="menuitem"
      disabled={item.shortcutState === "creating"}
      onclick={() => {
        close();
        item.createShortcut("menu");
      }}
    >
      {@render linkIcon()}
      {m.gameCard_menuMenuShortcut()}
    </button>
    <button
      type="button"
      role="menuitem"
      onclick={() => {
        close();
        onExportToSteam();
      }}
    >
      <svg class="menu-icon" viewBox="0 0 20 20" fill="none" aria-hidden="true">
        <rect x="3" y="3" width="14" height="14" rx="3" stroke="currentColor" stroke-width="1.5" />
        <path d="M10 6.5v7M6.5 10h7" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
      </svg>
      {inSteam ? m.gameCard_menuUpdateInSteam() : m.gameCard_menuAddToSteam()}
    </button>
    {#if inSteam}
      <button
        type="button"
        role="menuitem"
        onclick={() => {
          close();
          onRemoveFromSteam();
        }}
      >
        <svg class="menu-icon" viewBox="0 0 20 20" fill="none" aria-hidden="true">
          <rect x="3" y="3" width="14" height="14" rx="3" stroke="currentColor" stroke-width="1.5" />
          <path d="M6.5 10h7" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
        </svg>
        {m.gameCard_menuRemoveFromSteam()}
      </button>
    {/if}
    {#if full}
      <span class="divider" role="separator"></span>
      <button
        type="button"
        role="menuitem"
        class="danger"
        onclick={() => {
          close();
          item.confirmingRemove = true;
        }}
      >
        <svg class="menu-icon" viewBox="0 0 20 20" fill="none" aria-hidden="true">
          <path
            d="M5 6h10M8 6V4.5a1 1 0 011-1h2a1 1 0 011 1V6M6.5 6l.6 8.6a1 1 0 001 .9h3.8a1 1 0 001-.9L13.5 6"
            stroke="currentColor"
            stroke-width="1.5"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
        </svg>
        {m.common_remove()}
      </button>
    {/if}
  {/snippet}
</Menu>

<style>
  .menu-icon {
    width: 1.05em;
    height: 1.05em;
    flex-shrink: 0;
  }
</style>
