<script lang="ts">
  import { onMount } from "svelte";
  import PerformanceEditor from "$lib/components/PerformanceEditor.svelte";
  import SettingsPanel from "$lib/components/SettingsPanel.svelte";
  import {
    performanceConfig,
    refreshPerformanceConfig,
    savePerformanceConfig,
  } from "$lib/stores/performance";
  import { maxMapCountStatus, refreshMaxMapCountStatus, fixMaxMapCount } from "$lib/stores/system";
  import type { PerformanceConfig } from "$lib/types";
  import { t, getLocale } from "$lib/i18n/index.svelte";

  let draft = $state<PerformanceConfig | null>(null);
  let saving = $state(false);
  let error = $state("");
  let saved = $state(false);

  let fixingMapCount = $state(false);
  let mapCountFixError = $state("");

  onMount(() => {
    refreshPerformanceConfig();
    refreshMaxMapCountStatus();
  });

  $effect(() => {
    if (!draft && $performanceConfig) draft = { ...$performanceConfig };
  });

  async function handleFixMapCount() {
    fixingMapCount = true;
    mapCountFixError = "";
    try {
      await fixMaxMapCount();
    } catch (e) {
      mapCountFixError = String(e);
    } finally {
      fixingMapCount = false;
    }
  }

  async function handleSave() {
    if (!draft) return;
    saving = true;
    error = "";
    saved = false;
    try {
      await savePerformanceConfig(draft);
      saved = true;
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }
</script>

<div class="page">
  {#if $maxMapCountStatus && !$maxMapCountStatus.sufficient}
    <div class="warning-banner">
      <div>
        <strong>{t("performanceSettings.mapCountWarningTitle")}</strong>
        <p>
          {t("performanceSettings.mapCountWarningBody", {
            current: $maxMapCountStatus.current.toLocaleString(
              getLocale() === "de" ? "de-DE" : "en-US",
            ),
            recommended: $maxMapCountStatus.recommended.toLocaleString(
              getLocale() === "de" ? "de-DE" : "en-US",
            ),
          })}
        </p>
        {#if mapCountFixError}
          <p class="error">{mapCountFixError}</p>
        {/if}
        {#if !$maxMapCountStatus.can_fix}
          <code
            >sudo sh -c 'echo "vm.max_map_count = {$maxMapCountStatus.recommended}" &gt;
            /etc/sysctl.d/99-prefixr-max-map-count.conf && sysctl --system'</code
          >
        {/if}
      </div>
      {#if $maxMapCountStatus.can_fix}
        <button
          type="button"
          class="primary"
          disabled={fixingMapCount}
          onclick={handleFixMapCount}
        >
          {fixingMapCount ? t("performanceSettings.fixing") : t("performanceSettings.fixNow")}
        </button>
      {/if}
    </div>
  {/if}


  <SettingsPanel
    title={t("performanceSettings.title")}
    hint={t("performanceSettings.hint")}
    {saving}
    {saved}
    {error}
    onSave={handleSave}
  >
    {#if draft}
      <PerformanceEditor
        value={draft}
        onchange={(patch) => {
          draft = { ...draft!, ...patch };
          saved = false;
        }}
      />
    {/if}
  </SettingsPanel>
</div>

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 1.2em;
    max-width: 780px;
  }

  .warning-banner {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1.2em;
    padding: 0.9em 1.1em;
    border-radius: var(--radius);
    background: var(--danger-bg);
    border: 1px solid var(--danger);
  }

  .warning-banner strong {
    display: block;
    margin-bottom: 0.3em;
  }

  .warning-banner p {
    color: var(--text-muted);
    font-size: 0.85em;
    max-width: 60ch;
    margin: 0;
  }

  .warning-banner p + p {
    margin-top: 0.4em;
  }

  .warning-banner code {
    display: block;
    margin-top: 0.5em;
    padding: 0.5em 0.7em;
    border-radius: 6px;
    background: var(--surface);
    border: 1px solid var(--border);
    font-size: 0.78em;
    white-space: pre-wrap;
    word-break: break-all;
  }

  .warning-banner button {
    flex-shrink: 0;
  }

  .error {
    color: var(--danger);
  }
</style>
