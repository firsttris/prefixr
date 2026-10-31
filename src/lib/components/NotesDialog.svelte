<script lang="ts">
  import * as m from "#lib/paraglide/messages.js";
  import { parseNotes } from "#lib/notes.ts";
  import type { Game } from "#lib/types.ts";
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

  let blocks = $derived(game ? parseNotes(game.notes) : []);
</script>

<Modal
  open={game !== null}
  title={game ? m.notesDialog_title({ name: game.name }) : ""}
  wide
  {onClose}
>
  {#if game}
    <div class="notes">
      {#each blocks as block, i (i)}
        {#if block.kind === "heading"}
          <h4>{block.text}</h4>
        {:else if block.kind === "list"}
          <ul>
            {#each block.items as item, j (j)}
              <li>{item}</li>
            {/each}
          </ul>
        {:else}
          <p>{block.text}</p>
        {/if}
      {/each}
    </div>
    <div class="actions">
      <button type="button" class="ghost" onclick={onClose}>{m.common_close()}</button>
      <button type="button" class="primary" onclick={() => onEdit(game)}>
        {m.notesDialog_edit()}
      </button>
    </div>
  {/if}
</Modal>

<style>
  /* Selectable, so a login in the notes can be copied. */
  .notes {
    user-select: text;
    -webkit-user-select: text;
    line-height: 1.55;
    overflow-wrap: anywhere;
    max-height: 60vh;
    overflow-y: auto;
    margin-bottom: 1.2em;
    display: flex;
    flex-direction: column;
    gap: 0.6em;
  }

  .notes p {
    white-space: pre-wrap;
  }

  .notes h4 {
    margin-top: 0.6em;
    font-size: 0.8em;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted);
  }

  .notes h4:first-child {
    margin-top: 0;
  }

  .notes ul {
    margin: 0;
    padding-left: 1.2em;
    display: flex;
    flex-direction: column;
    gap: 0.35em;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.6em;
  }
</style>
