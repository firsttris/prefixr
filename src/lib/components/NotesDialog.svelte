<script lang="ts">
  import * as m from "$lib/paraglide/messages";
  import type { Game } from "$lib/types";
  import Modal from "./Modal.svelte";

  let {
    game,
    onClose,
    onEdit,
  }: {
    // The game whose notes are shown; null while the dialog is closed.
    game: Game | null;
    onClose: () => void;
    onEdit: (game: Game) => void;
  } = $props();
</script>

<Modal open={game !== null} title={game ? m.notesDialog_title({ name: game.name }) : ""} {onClose}>
  {#if game}
    <p class="notes">{game.notes}</p>
    <div class="actions">
      <button type="button" class="ghost" onclick={onClose}>{m.common_close()}</button>
      <button type="button" class="primary" onclick={() => onEdit(game)}>
        {m.notesDialog_edit()}
      </button>
    </div>
  {/if}
</Modal>

<style>
  /* Kept as written, line breaks included, and selectable, so a login in
     the notes can be copied. */
  .notes {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    user-select: text;
    -webkit-user-select: text;
    line-height: 1.5;
    margin-bottom: 1.2em;
    max-height: 60vh;
    overflow-y: auto;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.6em;
  }
</style>
