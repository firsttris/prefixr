<script lang="ts">
  import { onMount } from "svelte";
  import OverlayEditor from "$lib/components/OverlayEditor.svelte";
  import SettingsPanel from "$lib/components/SettingsPanel.svelte";
  import { mangoHudConfig, refreshMangoHudConfig, saveMangoHudConfig } from "$lib/stores/mangohud";
  import type { MangoHudConfig } from "$lib/types";
  import { t } from "$lib/i18n/index.svelte";

  let draft = $state<MangoHudConfig | null>(null);
  let saving = $state(false);
  let error = $state("");
  let saved = $state(false);

  onMount(() => {
    refreshMangoHudConfig();
  });

  $effect(() => {
    if (!draft && $mangoHudConfig) draft = { ...$mangoHudConfig };
  });

  async function handleSave() {
    if (!draft) return;
    saving = true;
    error = "";
    saved = false;
    try {
      await saveMangoHudConfig(draft);
      saved = true;
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }
</script>

<SettingsPanel
  title={t("overlaySettings.title")}
  hint={t("overlaySettings.hint")}
  {saving}
  {saved}
  {error}
  onSave={handleSave}
>
  {#if draft}
    <OverlayEditor
      value={draft}
      onchange={(patch) => {
        draft = { ...draft!, ...patch };
        saved = false;
      }}
    />
  {/if}
</SettingsPanel>
