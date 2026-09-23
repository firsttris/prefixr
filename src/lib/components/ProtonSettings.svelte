<script lang="ts">
  import { onMount } from "svelte";
  import ProtonEditor from "$lib/components/ProtonEditor.svelte";
  import SettingsPanel from "$lib/components/SettingsPanel.svelte";
  import {
    listProtonOptions,
    protonConfig,
    refreshProtonConfig,
    saveProtonConfig,
  } from "$lib/stores/proton";
  import { refreshRunners, runners } from "$lib/stores/runners";
  import type { ProtonOption } from "$lib/types";
  import { t } from "$lib/i18n/index.svelte";

  let draft = $state<Record<string, boolean> | null>(null);
  let saving = $state(false);
  let error = $state("");
  let saved = $state(false);

  // Which switches exist depends on the Proton version, so the list comes
  // from one installed runner — the newest by name unless picked otherwise.
  const protonRunners = $derived($runners.filter((r) => r.kind === "proton"));
  let pickedRunnerId = $state("");
  const runnerId = $derived(
    protonRunners.some((r) => r.id === pickedRunnerId)
      ? pickedRunnerId
      : (protonRunners.at(-1)?.id ?? ""),
  );
  let options = $state<ProtonOption[]>([]);
  let optionsError = $state("");

  onMount(() => {
    refreshRunners();
    refreshProtonConfig();
  });

  $effect(() => {
    if (!draft && $protonConfig) draft = { ...$protonConfig.options };
  });

  $effect(() => {
    const id = runnerId;
    if (!id) {
      options = [];
      return;
    }
    let cancelled = false;
    listProtonOptions(id)
      .then((result) => {
        if (cancelled) return;
        options = result;
        optionsError = "";
      })
      .catch((e) => {
        if (cancelled) return;
        options = [];
        optionsError = String(e);
      });
    return () => {
      cancelled = true;
    };
  });

  async function handleSave() {
    if (!draft) return;
    saving = true;
    error = "";
    saved = false;
    try {
      await saveProtonConfig({ options: draft });
      saved = true;
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }
</script>

<SettingsPanel
  title={t("protonSettings.title")}
  hint={t("protonSettings.hint")}
  {saving}
  {saved}
  {error}
  onSave={handleSave}
>
  {#if protonRunners.length === 0}
    <p class="hint">{t("protonSettings.noRunner")}</p>
  {:else}
    <label class="runner">
      {t("protonSettings.availableSwitches")}
      <select
        value={runnerId}
        onchange={(e) => (pickedRunnerId = e.currentTarget.value)}
      >
        {#each protonRunners as runner (runner.id)}
          <option value={runner.id}>{runner.name}</option>
        {/each}
      </select>
    </label>
    {#if draft}
      <ProtonEditor
        {options}
        values={draft}
        error={optionsError}
        onchange={(next) => {
          draft = next;
          saved = false;
        }}
      />
    {/if}
  {/if}
</SettingsPanel>

<style>
  .hint {
    color: var(--text-muted);
    font-size: 0.85em;
  }

  .runner {
    max-width: 22em;
  }
</style>
