<!-- A failed store install, under its card or row, with its log when there is one. -->
<script lang="ts">
  import * as m from "#lib/paraglide/messages.js";
  import { showLog } from "#lib/logViewer.ts";

  let {
    message,
    error,
    inline = false,
    onDismiss,
  }: {
    message: string;
    error: unknown;
    inline?: boolean;
    onDismiss: () => void;
  } = $props();

  let logPath = $derived(
    typeof error === "object" && error !== null && "log_path" in error
      ? String((error as { log_path: unknown }).log_path)
      : null,
  );
</script>

<div class="toast" class:inline role="alert">
  <span>{message}</span>
  <div class="actions">
    {#if logPath}
      <button type="button" class="ghost" onclick={() => showLog(logPath!)}>
        {m.gameCard_showLog()}
      </button>
    {/if}
    <button type="button" class="ghost" onclick={onDismiss}>{m.common_close()}</button>
  </div>
</div>

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
    justify-content: space-between;
    gap: 0.6em;
    margin: 0.4em 0 0;
  }

  .actions {
    display: flex;
    gap: 0.3em;
    flex-shrink: 0;
  }

  .actions button {
    padding: 0.3em 0.6em;
    font-size: 0.95em;
    color: inherit;
  }
</style>
