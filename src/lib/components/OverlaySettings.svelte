<script lang="ts">
  import { onMount } from "svelte";
  import OverlayEditor from "$lib/components/OverlayEditor.svelte";
  import SettingsPanel from "$lib/components/SettingsPanel.svelte";
  import { mangoHudConfig, refreshMangoHudConfig, saveMangoHudConfig } from "$lib/stores/mangohud";
  import type { MangoHudConfig } from "$lib/types";

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
  title="Overlay"
  hint="Das MangoHud-Overlay im Spiel. An/Aus und Aussehen gelten für alle Spiele und lassen sich pro Spiel überschreiben (Spiel bearbeiten → Overlay)."
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
