<script lang="ts">
  import { onMount } from "svelte";
  import GraphicsEditor from "$lib/components/GraphicsEditor.svelte";
  import SettingsPanel from "$lib/components/SettingsPanel.svelte";
  import { graphicsConfig, refreshGraphicsConfig, saveGraphicsConfig } from "$lib/stores/graphics";
  import type { GraphicsConfig } from "$lib/types";

  let draft = $state<GraphicsConfig | null>(null);
  let saving = $state(false);
  let error = $state("");
  let saved = $state(false);

  onMount(() => {
    refreshGraphicsConfig();
  });

  $effect(() => {
    if (!draft && $graphicsConfig) {
      draft = {
        gamescope: { ...$graphicsConfig.gamescope },
        vkbasalt: { ...$graphicsConfig.vkbasalt },
      };
    }
  });

  async function handleSave() {
    if (!draft) return;
    saving = true;
    error = "";
    saved = false;
    try {
      await saveGraphicsConfig(draft);
      saved = true;
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }
</script>

<SettingsPanel
  title="Bild"
  hint="Was mit dem Bild des Spiels passiert, bevor es auf dem Bildschirm landet. Gilt für alle Spiele und lässt sich pro Spiel überschreiben (Spiel bearbeiten → Bild)."
  {saving}
  {saved}
  {error}
  onSave={handleSave}
>
  {#if draft}
    <GraphicsEditor
      value={draft}
      onchange={(patch) => {
        draft = {
          gamescope: { ...draft!.gamescope, ...patch.gamescope },
          vkbasalt: { ...draft!.vkbasalt, ...patch.vkbasalt },
        };
        saved = false;
      }}
    />
  {/if}
</SettingsPanel>
