<script lang="ts">
  import SettingToggle from "$lib/components/SettingToggle.svelte";
  import type { OverrideHooks } from "$lib/settings";
  import type { PerformanceConfig } from "$lib/types";

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

  const ROWS: {
    key: keyof PerformanceConfig;
    label: string;
    description: string;
    info: string;
  }[] = [
    {
      key: "gamemode_enabled",
      label: "GameMode",
      description: "Optimiert CPU-Takt und Priorität, solange das Spiel läuft.",
      info: "Empfehlung: Wenn installiert, ruhig aktivieren — bringt oft spürbar mehr Leistung, besonders auf Laptops oder mit Energiesparmodus. Kein Nachteil, wenn GameMode fehlt. Läuft gerade ein konkurrierender Scheduler-Daemon (z. B. ananicy-cpp, scx), wird GameMode automatisch übersprungen und stattdessen auf das Performance-Energieprofil ausgewichen, um Konflikte um CPU-Priorität zu vermeiden.",
    },
    {
      key: "power_profile_enabled",
      label: "Performance-Energieprofil",
      description: "Hält das Energieprofil auf „Leistung“, solange ein Spiel läuft.",
      info: "Empfehlung: Auf Laptops praktisch immer sinnvoll, auf Desktops ohne Energieprofile meist wirkungslos. Ergänzt GameMode statt es zu ersetzen — GameMode setzt den CPU-Governor selbst, was der Power-Profile-Dienst auf vielen Distros wieder überschreibt. Braucht power-profiles-daemon; ohne das passiert einfach nichts.",
    },
    {
      key: "inhibit_sleep_enabled",
      label: "Ruhezustand verhindern",
      description: "Hält System und Bildschirm wach, solange ein Spiel läuft.",
      info: "Empfehlung: Praktisch immer sinnvoll — verhindert, dass der Bildschirmschoner oder der Energiesparmodus mitten im Spiel zuschlägt. Braucht systemd-inhibit; ohne das passiert einfach nichts.",
    },
  ];
</script>

<div class="list">
  {#each ROWS as row (row.key)}
    <SettingToggle
      label={row.label}
      description={row.description}
      info={row.info}
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
