<script lang="ts">
  import SettingToggle from "$lib/components/SettingToggle.svelte";
  import type { OverrideHooks } from "$lib/settings";
  import type { PerformanceConfig } from "$lib/types";
  import { t } from "$lib/i18n/index.svelte";

  // Leistung — see settings.ts for how the editors are shared between the
  // global page and the game dialog.
  let {
    value,
    onchange,
    overrides,
  }: {
    value: PerformanceConfig;
    onchange: (patch: Partial<PerformanceConfig>) => void;
    overrides?: OverrideHooks<keyof PerformanceConfig>;
  } = $props();

  const ROWS: { key: keyof PerformanceConfig }[] = [
    { key: "gamemode_enabled" },
    { key: "power_profile_enabled" },
    { key: "inhibit_sleep_enabled" },
  ];
</script>

<div class="list">
  {#each ROWS as row (row.key)}
    <SettingToggle
      label={t(`performanceEditor.${row.key}.label`)}
      description={t(`performanceEditor.${row.key}.description`)}
      info={t(`performanceEditor.${row.key}.info`)}
      checked={value[row.key]}
      onToggle={(checked) => onchange({ [row.key]: checked })}
      overridden={overrides?.isOverridden(row.key)}
      resetTitle={overrides?.resetTitle(row.key)}
      onReset={overrides && (() => overrides.reset(row.key))}
    />
  {/each}
</div>

<style>
  .list {
    display: flex;
    flex-direction: column;
    gap: 0.7em;
  }
</style>
