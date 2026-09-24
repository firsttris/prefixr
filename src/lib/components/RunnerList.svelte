<script lang="ts">
  import { onMount } from "svelte";
  import { runners, refreshRunners, deleteRunner } from "$lib/stores/runners";
  import { games, refreshGames } from "$lib/stores/games";
  import type { Runner } from "$lib/types";
  import ConfirmDialog from "./ConfirmDialog.svelte";
  import Modal from "./Modal.svelte";
  import RunnerDownloads from "./RunnerDownloads.svelte";
  import GitHubSettings from "./GitHubSettings.svelte";
  import UmuSettings from "./UmuSettings.svelte";
  import { backendError, t } from "$lib/i18n/index.svelte";

  let showDownloads = $state(false);
  let deleting = $state<Runner | null>(null);
  let error = $state<unknown>(null);

  // How many games use each runner. The library may not have been loaded
  // yet when this view opens first, hence the refresh below.
  let usage = $derived(
    $games.reduce<Record<string, number>>((counts, game) => {
      counts[game.runner_id] = (counts[game.runner_id] ?? 0) + 1;
      return counts;
    }, {}),
  );

  function usageLabel(count: number): string {
    if (count === 0) return t("runnerList.unused");
    return count === 1 ? t("runnerList.usedByOne") : t("runnerList.usedByMany", { count });
  }

  async function confirmDelete() {
    const runner = deleting;
    deleting = null;
    if (!runner) return;
    error = null;
    try {
      await deleteRunner(runner.id);
    } catch (e) {
      error = e;
    }
  }

  onMount(() => {
    refreshRunners();
    refreshGames();
  });
</script>

<section class="panel">
  <p class="explainer">
    {t("runnerList.explainerBefore")} <strong>{t("runnerList.explainerProton")}</strong>
    {t("runnerList.explainerOr")} <strong>{t("runnerList.explainerWine")}</strong>.
    {t("runnerList.explainerAfter")}
  </p>

  <div class="panel-header">
    <h2>{t("runnerList.heading")}</h2>
    <div class="actions">
      <button type="button" class="ghost" onclick={refreshRunners}>{t("runnerList.refresh")}</button>
      <button type="button" class="primary" onclick={() => (showDownloads = true)}>
        {t("runnerList.downloadRunner")}
      </button>
    </div>
  </div>

  <GitHubSettings />
  <UmuSettings />

  {#if $runners.length === 0}
    <p class="hint">{t("runnerList.empty")}</p>
  {:else}
    <ul>
      {#each $runners as runner (runner.id)}
        {@const count = usage[runner.id] ?? 0}
        <li>
          <span class="name">{runner.name}</span>
          <span class="meta">
            <span class="usage" class:unused={count === 0}>{usageLabel(count)}</span>
            <span class="kind">{runner.kind}</span>
            {#if count === 0}
              <button type="button" class="ghost small" onclick={() => (deleting = runner)}>
                {t("runnerList.delete")}
              </button>
            {/if}
          </span>
        </li>
      {/each}
    </ul>
  {/if}
  {#if error}
    <p class="error">{backendError(error)}</p>
  {/if}
</section>

<ConfirmDialog
  open={deleting !== null}
  title={t("runnerList.deleteConfirmTitle")}
  message={t("runnerList.deleteConfirmMessage", { name: deleting?.name ?? "" })}
  confirmLabel={t("runnerList.delete")}
  onConfirm={confirmDelete}
  onCancel={() => (deleting = null)}
/>

<Modal
  open={showDownloads}
  title={t("runnerList.downloadRunner")}
  onClose={() => (showDownloads = false)}
>
  <RunnerDownloads />
</Modal>

<style>
  .panel {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 1.2em;
    display: flex;
    flex-direction: column;
    gap: 0.8em;
    max-width: 780px;
  }

  .explainer {
    color: var(--text-muted);
    font-size: 0.9em;
    max-width: 60ch;
  }

  .explainer strong {
    color: var(--text);
  }

  .panel-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.8em;
    flex-wrap: wrap;
  }

  .actions {
    display: flex;
    gap: 0.5em;
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.4em;
  }

  li {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.5em 0.7em;
    background: var(--surface-raised);
    border-radius: 8px;
    font-size: 0.9em;
  }

  .meta {
    display: flex;
    align-items: center;
    gap: 0.8em;
  }

  .usage {
    color: var(--text-muted);
    font-size: 0.85em;
  }

  .usage.unused {
    font-style: italic;
  }

  .kind {
    color: var(--text-muted);
    font-size: 0.85em;
    text-transform: uppercase;
  }

  .small {
    padding: 0.2em 0.6em;
    font-size: 0.85em;
  }

  .error {
    color: var(--danger);
    font-size: 0.85em;
  }

  .hint {
    color: var(--text-muted);
    font-size: 0.9em;
  }
</style>
