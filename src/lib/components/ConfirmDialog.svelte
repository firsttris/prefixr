<script lang="ts">
  import Modal from "./Modal.svelte";

  let {
    open,
    title,
    message,
    confirmLabel = "Entfernen",
    onConfirm,
    onCancel,
  }: {
    open: boolean;
    title: string;
    message: string;
    confirmLabel?: string;
    onConfirm: () => void;
    onCancel: () => void;
  } = $props();
</script>

<Modal {open} {title} onClose={onCancel}>
  <p>{message}</p>
  <div class="actions">
    <button type="button" class="ghost" onclick={onCancel}>Abbrechen</button>
    <button type="button" class="danger" onclick={onConfirm}>{confirmLabel}</button>
  </div>
</Modal>

<style>
  p {
    color: var(--text-muted);
    margin-bottom: 1.2em;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.6em;
  }
</style>
