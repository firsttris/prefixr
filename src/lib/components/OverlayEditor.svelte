<script lang="ts">
  import SettingToggle from "$lib/components/SettingToggle.svelte";
  import { COLORS, POSITIONS, PRESETS, type MangoHudPreset } from "$lib/mangohudPresets";
  import type { OverrideHooks } from "$lib/settings";
  import type { MangoHudConfig, MangoHudLayout } from "$lib/types";

  // Overlay — see settings.ts for how the editors are shared between the
  // global page and the game dialog. On/off and the look (`layout`) are
  // overridden separately, see `OverlayOverrides` in models.rs.
  let {
    value,
    onchange,
    overrides,
  }: {
    value: MangoHudConfig;
    onchange: (patch: Partial<MangoHudConfig>) => void;
    overrides?: OverrideHooks<"enabled" | "layout">;
  } = $props();

  function applyPreset(preset: MangoHudPreset) {
    onchange({ ...preset.values, preset: preset.key, enabled: true });
  }

  // Any hand-made change turns the preset selection into "custom".
  function setLayout(patch: Partial<MangoHudLayout>) {
    onchange({ ...patch, preset: "custom" });
  }
</script>

<div class="overlay-editor">
  <SettingToggle
    label="MangoHud"
    description="Zeigt FPS, Auslastung und Temperaturen direkt im Spiel an. Setzt voraus, dass MangoHud installiert ist."
    checked={value.enabled}
    onToggle={(enabled) => onchange({ enabled })}
    overridden={overrides?.isOverridden("enabled")}
    resetTitle={overrides?.resetTitle("enabled")}
    onReset={overrides && (() => overrides.reset("enabled"))}
  />

  <div class="look" class:overridden={overrides?.isOverridden("layout")}>
    <div class="look-header">
      <span class="look-title">Aussehen</span>
      {#if overrides?.isOverridden("layout")}
        <button
          type="button"
          class="reset"
          title={overrides.resetTitle("layout")}
          onclick={() => overrides.reset("layout")}
        >
          Zurücksetzen
        </button>
      {/if}
    </div>
    <div class="presets">
      {#each PRESETS as preset (preset.key)}
        <button
          type="button"
          class="preset-card"
          class:active={value.preset === preset.key}
          onclick={() => applyPreset(preset)}
        >
          <span class="preset-label">{preset.label}</span>
          <span class="preset-description">{preset.description}</span>
        </button>
      {/each}
    </div>

    <div class="tune-grid">
      <div class="preview-wrap">
        <span class="tune-title">Vorschau</span>
        <div class="preview-screen">
          <div
            class="overlay-box pos-{value.position}"
            class:rounded={value.round_corners}
            class:horizontal={value.horizontal}
            style={`--overlay-color:#${value.theme_color}; --overlay-alpha:${value.background_alpha};`}
          >
            {#if value.show_gpu_name}
              <div class="stat stat-muted">AMD Radeon RX 7900 XTX</div>
            {/if}
            {#if value.show_driver}
              <div class="stat stat-muted">RADV</div>
            {/if}
            {#if value.show_engine_version}
              <div class="stat stat-muted">DXVK 2.3</div>
            {/if}
            {#if value.show_wine}
              <div class="stat stat-muted">Proton-GE 9-20</div>
            {/if}
            {#if value.show_resolution}
              <div class="stat stat-muted">2560x1440</div>
            {/if}
            {#if value.show_fps}
              <div class="stat stat-fps">144 FPS</div>
            {/if}
            {#if value.show_frametime}
              <div class="graph">
                {#each [40, 70, 55, 90, 65, 50, 80, 60] as h (h)}
                  <span style={`height:${h}%`}></span>
                {/each}
              </div>
            {/if}
            {#if value.show_cpu}
              <div class="stat">CPU 46%{value.show_temps ? " · 65°C" : ""}</div>
            {/if}
            {#if value.show_gpu}
              <div class="stat">GPU 62%{value.show_temps ? " · 71°C" : ""}</div>
            {/if}
            {#if value.show_ram}
              <div class="stat">RAM 8.2/16 GB</div>
            {/if}
            {#if value.show_vram}
              <div class="stat">VRAM 4.1/8 GB</div>
            {/if}
            {#if value.show_gamemode || value.show_vkbasalt || value.show_hdr}
              <div class="badges">
                {#if value.show_gamemode}<span class="badge">GameMode</span>{/if}
                {#if value.show_vkbasalt}<span class="badge">vkBasalt</span>{/if}
                {#if value.show_hdr}<span class="badge">HDR</span>{/if}
              </div>
            {/if}
          </div>
        </div>
      </div>

      <div class="controls">
        <div class="control-group">
          <span class="tune-title">Position</span>
          <div class="position-grid">
            {#each POSITIONS as pos (pos.value)}
              <button
                type="button"
                class="position-btn pos-{pos.value}"
                class:active={value.position === pos.value}
                title={pos.label}
                onclick={() => setLayout({ position: pos.value })}
              ></button>
            {/each}
          </div>
        </div>

        <div class="control-group">
          <span class="tune-title">Farbe</span>
          <div class="colors">
            {#each COLORS as color (color.hex)}
              <button
                type="button"
                class="color-swatch"
                class:active={value.theme_color === color.hex}
                title={color.name}
                style={`--swatch-color:#${color.hex}`}
                onclick={() => setLayout({ theme_color: color.hex })}
              ></button>
            {/each}
          </div>
        </div>

        <label class="control-group">
          <span class="tune-title">Durchsichtigkeit des Hintergrunds</span>
          <input
            type="range"
            min="0"
            max="1"
            step="0.05"
            value={value.background_alpha}
            oninput={(e) => setLayout({ background_alpha: e.currentTarget.valueAsNumber })}
          />
        </label>

        <label class="checkbox-row">
          <input type="checkbox" checked={value.round_corners}
                onchange={(e) => setLayout({ round_corners: e.currentTarget.checked })} />
          Abgerundete Ecken
        </label>

        <label class="checkbox-row">
          <input type="checkbox" checked={value.horizontal}
                onchange={(e) => setLayout({ horizontal: e.currentTarget.checked })} />
          Horizontales Layout
        </label>

        <div class="control-group">
          <span class="tune-title">Leistungswerte</span>
          <div class="checkbox-list">
            <label class="checkbox-row">
              <input type="checkbox" checked={value.show_fps}
                onchange={(e) => setLayout({ show_fps: e.currentTarget.checked })} />
              FPS-Zähler
            </label>
            <label class="checkbox-row">
              <input type="checkbox" checked={value.show_frametime}
                onchange={(e) => setLayout({ show_frametime: e.currentTarget.checked })} />
              Verlaufsgrafik
            </label>
            <label class="checkbox-row">
              <input type="checkbox" checked={value.show_cpu}
                onchange={(e) => setLayout({ show_cpu: e.currentTarget.checked })} />
              Prozessor
            </label>
            <label class="checkbox-row">
              <input type="checkbox" checked={value.show_gpu}
                onchange={(e) => setLayout({ show_gpu: e.currentTarget.checked })} />
              Grafikkarte
            </label>
            <label class="checkbox-row">
              <input type="checkbox" checked={value.show_ram}
                onchange={(e) => setLayout({ show_ram: e.currentTarget.checked })} />
              Arbeitsspeicher
            </label>
            <label class="checkbox-row">
              <input type="checkbox" checked={value.show_vram}
                onchange={(e) => setLayout({ show_vram: e.currentTarget.checked })} />
              Grafikspeicher
            </label>
            <label class="checkbox-row">
              <input type="checkbox" checked={value.show_temps}
                onchange={(e) => setLayout({ show_temps: e.currentTarget.checked })} />
              Temperaturen
            </label>
          </div>
        </div>

        <div class="control-group">
          <span class="tune-title">Status-Icons</span>
          <div class="checkbox-list">
            <label class="checkbox-row">
              <input type="checkbox" checked={value.show_gamemode}
                onchange={(e) => setLayout({ show_gamemode: e.currentTarget.checked })} />
              GameMode aktiv
            </label>
            <label class="checkbox-row">
              <input type="checkbox" checked={value.show_vkbasalt}
                onchange={(e) => setLayout({ show_vkbasalt: e.currentTarget.checked })} />
              vkBasalt aktiv
            </label>
            <label class="checkbox-row">
              <input type="checkbox" checked={value.show_hdr}
                onchange={(e) => setLayout({ show_hdr: e.currentTarget.checked })} />
              HDR aktiv
            </label>
          </div>
        </div>

        <div class="control-group">
          <span class="tune-title">Technische Infos</span>
          <div class="checkbox-list">
            <label class="checkbox-row">
              <input type="checkbox" checked={value.show_driver}
                onchange={(e) => setLayout({ show_driver: e.currentTarget.checked })} />
              Grafiktreiber
            </label>
            <label class="checkbox-row">
              <input type="checkbox" checked={value.show_engine_version}
                onchange={(e) => setLayout({ show_engine_version: e.currentTarget.checked })} />
              DXVK/VKD3D-Version
            </label>
            <label class="checkbox-row">
              <input type="checkbox" checked={value.show_wine}
                onchange={(e) => setLayout({ show_wine: e.currentTarget.checked })} />
              Wine/Proton-Version
            </label>
            <label class="checkbox-row">
              <input type="checkbox" checked={value.show_gpu_name}
                onchange={(e) => setLayout({ show_gpu_name: e.currentTarget.checked })} />
              Grafikkarten-Name
            </label>
            <label class="checkbox-row">
              <input type="checkbox" checked={value.show_resolution}
                onchange={(e) => setLayout({ show_resolution: e.currentTarget.checked })} />
              Auflösung
            </label>
          </div>
        </div>
      </div>
    </div>
  </div>
</div>

<style>
  .overlay-editor {
    display: flex;
    flex-direction: column;
    gap: 1.2em;
  }

  .look {
    display: flex;
    flex-direction: column;
    gap: 1.2em;
  }

  .look.overridden {
    border-left: 3px solid var(--accent);
    padding-left: 0.9em;
  }

  .look-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1em;
  }

  .look-title {
    font-weight: 600;
  }

  .reset {
    padding: 0.25em 0.6em;
    font-size: 0.8em;
  }

  .presets {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
    gap: 0.7em;
  }

  .preset-card {
    text-align: left;
    display: flex;
    flex-direction: column;
    gap: 0.3em;
    padding: 0.8em 0.9em;
    border-radius: 10px;
    border: 1px solid var(--border);
    background: var(--surface-raised);
  }

  .preset-card.active {
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent);
  }

  .preset-label {
    font-weight: 600;
  }

  .preset-description {
    font-size: 0.8em;
    color: var(--text-muted);
  }

  .tune-grid {
    display: grid;
    grid-template-columns: 1.1fr 1fr;
    gap: 1.4em;
    align-items: start;
  }

  @media (max-width: 640px) {
    .tune-grid {
      grid-template-columns: 1fr;
    }
  }

  .tune-title {
    display: block;
    font-size: 0.85em;
    color: var(--text-muted);
    margin-bottom: 0.5em;
  }

  .preview-wrap {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .preview-screen {
    position: relative;
    width: 100%;
    height: 260px;
    border-radius: var(--radius);
    overflow: hidden;
    background: radial-gradient(circle at 30% 20%, #2c3550, #0b0d13 70%);
    border: 1px solid var(--border);
  }

  .overlay-box {
    position: absolute;
    display: flex;
    flex-direction: column;
    gap: 0.25em;
    padding: 0.5em 0.7em;
    background: rgba(0, 0, 0, var(--overlay-alpha, 0.4));
    color: var(--overlay-color, #fff);
    font-family: "Courier New", monospace;
    font-size: 0.72em;
    line-height: 1.4;
  }

  .overlay-box.rounded {
    border-radius: 8px;
  }

  .overlay-box.horizontal {
    flex-direction: row;
    align-items: center;
  }

  .overlay-box.pos-top-left {
    top: 0.7em;
    left: 0.7em;
  }

  .overlay-box.pos-top-right {
    top: 0.7em;
    right: 0.7em;
    text-align: right;
  }

  .overlay-box.pos-bottom-left {
    bottom: 0.7em;
    left: 0.7em;
  }

  .overlay-box.pos-bottom-right {
    bottom: 0.7em;
    right: 0.7em;
    text-align: right;
  }

  .stat-fps {
    font-size: 1.4em;
    font-weight: 700;
  }

  .stat-muted {
    opacity: 0.7;
    font-size: 0.9em;
  }

  .badges {
    display: flex;
    gap: 0.3em;
    margin-top: 0.2em;
  }

  .badge {
    font-size: 0.65em;
    padding: 0.15em 0.5em;
    border-radius: 999px;
    border: 1px solid currentColor;
    opacity: 0.85;
  }

  .graph {
    display: flex;
    align-items: flex-end;
    gap: 2px;
    height: 18px;
  }

  .graph span {
    width: 4px;
    background: var(--overlay-color, #fff);
    opacity: 0.7;
    border-radius: 1px;
  }

  .controls {
    display: flex;
    flex-direction: column;
    gap: 1.1em;
    min-width: 0;
  }

  .control-group {
    display: flex;
    flex-direction: column;
  }

  .position-grid {
    width: 72px;
    height: 54px;
    border: 1px solid var(--border);
    border-radius: 8px;
    position: relative;
    background: var(--surface-raised);
  }

  .position-btn {
    position: absolute;
    width: 14px;
    height: 14px;
    border-radius: 4px;
    border: 1px solid var(--border);
    background: var(--bg);
    padding: 0;
  }

  .position-btn.active {
    background: var(--accent);
    border-color: var(--accent);
  }

  .position-btn.pos-top-left {
    top: 5px;
    left: 5px;
  }

  .position-btn.pos-top-right {
    top: 5px;
    right: 5px;
  }

  .position-btn.pos-bottom-left {
    bottom: 5px;
    left: 5px;
  }

  .position-btn.pos-bottom-right {
    bottom: 5px;
    right: 5px;
  }

  .colors {
    display: flex;
    gap: 0.5em;
  }

  .color-swatch {
    width: 26px;
    height: 26px;
    border-radius: 50%;
    background: var(--swatch-color);
    border: 2px solid var(--border);
    padding: 0;
  }

  .color-swatch.active {
    border-color: var(--accent);
  }

  .checkbox-list {
    display: flex;
    flex-direction: column;
    gap: 0.5em;
  }

  .checkbox-row {
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 0.5em;
    font-size: 0.9em;
    color: var(--text);
  }

  .checkbox-row input {
    width: auto;
  }
</style>
