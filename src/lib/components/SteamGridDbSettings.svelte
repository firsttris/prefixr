<script lang="ts">
  import { onMount } from "svelte";
  import {
    steamGridDbConfig,
    refreshSteamGridDbConfig,
    saveSteamGridDbConfig,
  } from "$lib/stores/steamgriddb";
  import type { SteamGridDbConfig } from "$lib/types";
  import { backendError, t } from "$lib/i18n/index.svelte";

  const FALLBACK: SteamGridDbConfig = { api_key: "" };

  let config = $state<SteamGridDbConfig>({ ...FALLBACK });
  let initialized = $state(false);
  let saving = $state(false);
  let error = $state<unknown>(null);
  let saved = $state(false);

  onMount(() => {
    refreshSteamGridDbConfig();
  });

  $effect(() => {
    if (!initialized && $steamGridDbConfig) {
      config = { ...$steamGridDbConfig, api_key: $steamGridDbConfig.api_key ?? "" };
      initialized = true;
    }
  });

  async function handleSave() {
    saving = true;
    error = null;
    saved = false;
    try {
      await saveSteamGridDbConfig({ api_key: config.api_key?.trim() || null });
      saved = true;
    } catch (e) {
      error = e;
    } finally {
      saving = false;
    }
  }
</script>

<section class="panel">
  <div class="header-row">
    <div>
      <h2>SteamGridDB</h2>
      <p class="hint">{t("steamGridDbSettings.hint")}</p>
    </div>
  </div>

  <label class="control-group">
    <span class="tune-title">{t("steamGridDbSettings.apiKeyLabel")}</span>
    <input
      type="password"
      class="text-input"
      placeholder={t("steamGridDbSettings.apiKeyPlaceholder")}
      bind:value={config.api_key}
      oninput={() => (saved = false)}
    />
  </label>

  {#if error}
    <p class="error">{backendError(error)}</p>
  {/if}

  <div class="save-row">
    <button type="button" class="primary" disabled={saving} onclick={handleSave}>
      {saving ? t("common.saving") : t("settingsPanel.saveChanges")}
    </button>
    {#if saved}
      <span class="saved-hint">{t("common.saved")}</span>
    {/if}
  </div>
</section>

<style>
  .panel {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 1.2em;
    display: flex;
    flex-direction: column;
    gap: 1.4em;
    max-width: 780px;
  }

  .header-row {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 1em;
  }

  .hint {
    color: var(--text-muted);
    font-size: 0.85em;
    margin-top: 0.3em;
    max-width: 60ch;
  }

  .control-group {
    display: flex;
    flex-direction: column;
    gap: 0.4em;
    max-width: 360px;
  }

  .tune-title {
    display: block;
    font-size: 0.85em;
    color: var(--text-muted);
  }

  .text-input {
    padding: 0.5em 0.7em;
    border-radius: 8px;
    border: 1px solid var(--border);
    background: var(--surface-raised);
    color: var(--text);
    font-size: 0.9em;
  }

  .error {
    color: var(--danger);
  }

  .save-row {
    display: flex;
    align-items: center;
    gap: 1em;
  }

  .saved-hint {
    color: var(--text-muted);
    font-size: 0.85em;
  }
</style>
