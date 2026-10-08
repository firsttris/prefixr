<!--
  A library entry's launch error and failed shortcut, and the question
  before it's removed — the same in the card and the list row, which only
  lay the notices out differently (`inline` for the row).
-->
<script lang="ts">
  import * as m from "$lib/paraglide/messages";
  import type { Game } from "$lib/types";
  import type { GameRunState } from "$lib/stores/games";
  import { showLog } from "$lib/logViewer";
  import { backendError } from "$lib/i18n/index.svelte";
  import ConfirmDialog from "./ConfirmDialog.svelte";
  import type { GameItem } from "./gameItem.svelte";

  let {
    game,
    item,
    runState,
    inSteam,
    inline = false,
    onRemove,
  }: {
    game: Game;
    item: GameItem;
    runState?: GameRunState;
    inSteam: boolean;
    inline?: boolean;
    onRemove: () => void;
  } = $props();
</script>

{#if runState?.error}
  <div class="toast" class:inline>
    <span>{backendError(runState.error)}</span>
    {#if runState.logPath}
      <button type="button" class="ghost" onclick={() => showLog(runState.logPath!)}>
        {m.gameCard_showLog()}
      </button>
    {/if}
  </div>
{/if}

{#if typeof item.shortcutState === "object"}
  <div class="toast" class:inline>
    <span>{m.gameCard_shortcutFailed({ error: backendError(item.shortcutState.error) })}</span>
  </div>
{/if}

<ConfirmDialog
  open={item.confirmingRemove}
  title={m.gameCard_removeConfirmTitle()}
  message={m.gameCard_removeConfirmMessage({
    name: game.name,
    steamPart: inSteam ? m.gameCard_removeConfirmSteamPart() : "",
  })}
  onConfirm={() => {
    item.confirmingRemove = false;
    onRemove();
  }}
  onCancel={() => (item.confirmingRemove = false)}
/>

<style>
  .toast {
    display: flex;
    flex-direction: column;
    gap: 0.4em;
    background: var(--danger-bg);
    color: var(--danger);
    border-radius: 8px;
    padding: 0.5em 0.7em;
    margin: 0.6em 0.9em 0.9em;
    font-size: 0.85em;
    text-align: left;
  }

  .toast.inline {
    flex-direction: row;
    align-items: center;
    gap: 0.6em;
    padding: 0.5em 0.8em;
    margin: 0.4em 0 0;
  }
</style>
