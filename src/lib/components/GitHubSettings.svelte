<script lang="ts">
  import { onMount } from "svelte";
  import { githubConfig, refreshGitHubConfig, saveGitHubConfig } from "$lib/stores/github";
  import type { GitHubConfig } from "$lib/types";
  import { backendError, t } from "$lib/i18n/index.svelte";

  let { open = false }: { open?: boolean } = $props();

  const FALLBACK: GitHubConfig = { token: "" };

  let config = $state<GitHubConfig>({ ...FALLBACK });
  let initialized = $state(false);
  let saving = $state(false);
  let error = $state<unknown>(null);
  let saved = $state(false);

  onMount(() => {
    refreshGitHubConfig();
  });

  $effect(() => {
    if (!initialized && $githubConfig) {
      config = { ...$githubConfig, token: $githubConfig.token ?? "" };
      initialized = true;
    }
  });

  async function handleSave() {
    saving = true;
    error = null;
    saved = false;
    try {
      await saveGitHubConfig({ token: config.token?.trim() || null });
      saved = true;
    } catch (e) {
      error = e;
    } finally {
      saving = false;
    }
  }
</script>

<details class="github-token" {open}>
  <summary>{t("githubSettings.summary")}</summary>
  <p class="hint">{t("githubSettings.hint")}</p>

  <div class="row">
    <input
      type="password"
      class="text-input"
      placeholder={t("githubSettings.tokenPlaceholder")}
      bind:value={config.token}
      oninput={() => (saved = false)}
    />
    <button type="button" class="ghost" disabled={saving} onclick={handleSave}>
      {saving ? t("githubSettings.saving") : t("common.save")}
    </button>
  </div>

  {#if error}
    <p class="error">{backendError(error)}</p>
  {:else if saved}
    <p class="saved-hint">{t("common.saved")}</p>
  {/if}
</details>

<style>
  .github-token {
    background: var(--surface-raised);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 0.5em 0.8em;
    margin-bottom: 0.8em;
    font-size: 0.9em;
  }

  summary {
    cursor: pointer;
    color: var(--text-muted);
    font-weight: 600;
    font-size: 0.85em;
  }

  .hint {
    color: var(--text-muted);
    font-size: 0.85em;
    max-width: 60ch;
    margin: 0.6em 0;
  }

  .row {
    display: flex;
    gap: 0.5em;
  }

  .text-input {
    flex: 1;
    padding: 0.5em 0.7em;
    border-radius: 8px;
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--text);
    font-size: 0.9em;
  }

  .error {
    color: var(--danger);
    font-size: 0.85em;
    margin: 0.5em 0 0;
  }

  .saved-hint {
    color: var(--text-muted);
    font-size: 0.85em;
    margin: 0.5em 0 0;
  }
</style>
