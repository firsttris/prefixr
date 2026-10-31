<script lang="ts">
  import { msgGroup } from "#lib/i18n/msg-groups.ts";
  import SettingToggle from "#lib/components/SettingToggle.svelte";
  import type { OverrideHooks } from "#lib/settings.ts";
  import type { PerformanceConfig } from "#lib/types.ts";
  import { pickMsg } from "#lib/i18n/index.svelte.ts";

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
      label={pickMsg(msgGroup.performanceEditor_label, row.key)}
      description={pickMsg(msgGroup.performanceEditor_description, row.key)}
      info={pickMsg(msgGroup.performanceEditor_info, row.key)}
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

  @container settings-panel (min-width: 960px) {
    .list {
      display: grid;
      grid-template-columns: repeat(auto-fill, minmax(380px, 1fr));
    }
  }
</style>
