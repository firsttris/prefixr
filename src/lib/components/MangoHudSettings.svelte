<script lang="ts">
  import { onMount } from "svelte";
  import { mangoHudConfig, refreshMangoHudConfig, saveMangoHudConfig } from "$lib/stores/mangohud";
  import type { MangoHudConfig, MangoHudPosition } from "$lib/types";

  const FALLBACK: MangoHudConfig = {
    enabled: false,
    preset: "standard",
    position: "top-left",
    theme_color: "ffffff",
    background_alpha: 0.4,
    round_corners: true,
    show_fps: true,
    show_frametime: true,
    show_cpu: true,
    show_gpu: true,
    show_ram: false,
    show_vram: false,
    show_temps: true,
    show_gamemode: false,
    show_vkbasalt: false,
    show_hdr: false,
    show_driver: false,
    show_engine_version: false,
    show_wine: false,
    show_gpu_name: false,
    show_resolution: false,
    horizontal: false,
  };

  interface Preset {
    key: string;
    label: string;
    description: string;
    values: Partial<MangoHudConfig>;
  }

  const PRESETS: Preset[] = [
    {
      key: "minimal",
      label: "Minimal",
      description: "Nur die FPS-Zahl, dezent in der Ecke.",
      values: {
        position: "top-right",
        theme_color: "ffffff",
        background_alpha: 0.15,
        round_corners: true,
        show_fps: true,
        show_frametime: false,
        show_cpu: false,
        show_gpu: false,
        show_ram: false,
        show_vram: false,
        show_temps: false,
        show_gamemode: false,
        show_vkbasalt: false,
        show_hdr: false,
        show_driver: false,
        show_engine_version: false,
        show_wine: false,
        show_gpu_name: false,
        show_resolution: false,
        horizontal: false,
      },
    },
    {
      key: "standard",
      label: "Standard",
      description: "FPS, Auslastung und Temperaturen im Überblick.",
      values: {
        position: "top-left",
        theme_color: "ffffff",
        background_alpha: 0.4,
        round_corners: true,
        show_fps: true,
        show_frametime: true,
        show_cpu: true,
        show_gpu: true,
        show_ram: false,
        show_vram: false,
        show_temps: true,
        show_gamemode: false,
        show_vkbasalt: false,
        show_hdr: false,
        show_driver: false,
        show_engine_version: false,
        show_wine: false,
        show_gpu_name: false,
        show_resolution: false,
        horizontal: false,
      },
    },
    {
      key: "detailed",
      label: "Ausführlich",
      description: "Alle Werte, inklusive Status-Icons und technischer Infos.",
      values: {
        position: "top-left",
        theme_color: "00e5ff",
        background_alpha: 0.55,
        round_corners: true,
        show_fps: true,
        show_frametime: true,
        show_cpu: true,
        show_gpu: true,
        show_ram: true,
        show_vram: true,
        show_temps: true,
        show_gamemode: true,
        show_vkbasalt: true,
        show_hdr: true,
        show_driver: true,
        show_engine_version: true,
        show_wine: true,
        show_gpu_name: true,
        show_resolution: true,
        horizontal: false,
      },
    },
    {
      key: "competitive",
      label: "Wettkampf",
      description: "Groß, knallig, sonst nichts — für maximale Übersicht.",
      values: {
        position: "top-right",
        theme_color: "39ff14",
        background_alpha: 0.1,
        round_corners: false,
        show_fps: true,
        show_frametime: false,
        show_cpu: false,
        show_gpu: false,
        show_ram: false,
        show_vram: false,
        show_temps: false,
        show_gamemode: false,
        show_vkbasalt: false,
        show_hdr: false,
        show_driver: false,
        show_engine_version: false,
        show_wine: false,
        show_gpu_name: false,
        show_resolution: false,
        horizontal: false,
      },
    },
  ];

  const COLORS = [
    { name: "Weiß", hex: "ffffff" },
    { name: "Cyan", hex: "00e5ff" },
    { name: "Grün", hex: "39ff14" },
    { name: "Orange", hex: "ff9100" },
    { name: "Pink", hex: "ff4da6" },
  ];

  const POSITIONS: { value: MangoHudPosition; label: string }[] = [
    { value: "top-left", label: "Oben links" },
    { value: "top-right", label: "Oben rechts" },
    { value: "bottom-left", label: "Unten links" },
    { value: "bottom-right", label: "Unten rechts" },
  ];

  let config = $state<MangoHudConfig>({ ...FALLBACK });
  let initialized = $state(false);
  let saving = $state(false);
  let error = $state("");
  let saved = $state(false);

  onMount(() => {
    refreshMangoHudConfig();
  });

  $effect(() => {
    if (!initialized && $mangoHudConfig) {
      config = { ...$mangoHudConfig };
      initialized = true;
    }
  });

  function applyPreset(preset: Preset) {
    config = { ...config, ...preset.values, preset: preset.key, enabled: true };
    saved = false;
  }

  function markCustom() {
    config.preset = "custom";
    saved = false;
  }

  async function handleSave() {
    saving = true;
    error = "";
    saved = false;
    try {
      await saveMangoHudConfig(config);
      saved = true;
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }
</script>

<section class="panel">
  <div class="header-row">
    <div>
      <h2>MangoHud-Overlay</h2>
      <p class="hint">
        Zeigt FPS, Auslastung und Temperaturen direkt im Spiel an — die Einstellungen gelten für
        alle Spiele. Setzt voraus, dass MangoHud auf deinem System installiert ist.
      </p>
    </div>
    <label class="switch">
      <input type="checkbox" bind:checked={config.enabled} onchange={() => (saved = false)} />
      <span class="track"><span class="thumb"></span></span>
      <span class="switch-label">{config.enabled ? "Aktiv" : "Aus"}</span>
    </label>
  </div>

  <div class="presets">
    {#each PRESETS as preset (preset.key)}
      <button
        type="button"
        class="preset-card"
        class:active={config.preset === preset.key}
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
          class="overlay-box pos-{config.position}"
          class:rounded={config.round_corners}
          class:horizontal={config.horizontal}
          style={`--overlay-color:#${config.theme_color}; --overlay-alpha:${config.background_alpha};`}
        >
          {#if config.show_gpu_name}
            <div class="stat stat-muted">AMD Radeon RX 7900 XTX</div>
          {/if}
          {#if config.show_driver}
            <div class="stat stat-muted">RADV</div>
          {/if}
          {#if config.show_engine_version}
            <div class="stat stat-muted">DXVK 2.3</div>
          {/if}
          {#if config.show_wine}
            <div class="stat stat-muted">Proton-GE 9-20</div>
          {/if}
          {#if config.show_resolution}
            <div class="stat stat-muted">2560x1440</div>
          {/if}
          {#if config.show_fps}
            <div class="stat stat-fps">144 FPS</div>
          {/if}
          {#if config.show_frametime}
            <div class="graph">
              {#each [40, 70, 55, 90, 65, 50, 80, 60] as h (h)}
                <span style={`height:${h}%`}></span>
              {/each}
            </div>
          {/if}
          {#if config.show_cpu}
            <div class="stat">CPU 46%{config.show_temps ? " · 65°C" : ""}</div>
          {/if}
          {#if config.show_gpu}
            <div class="stat">GPU 62%{config.show_temps ? " · 71°C" : ""}</div>
          {/if}
          {#if config.show_ram}
            <div class="stat">RAM 8.2/16 GB</div>
          {/if}
          {#if config.show_vram}
            <div class="stat">VRAM 4.1/8 GB</div>
          {/if}
          {#if config.show_gamemode || config.show_vkbasalt || config.show_hdr}
            <div class="badges">
              {#if config.show_gamemode}<span class="badge">GameMode</span>{/if}
              {#if config.show_vkbasalt}<span class="badge">vkBasalt</span>{/if}
              {#if config.show_hdr}<span class="badge">HDR</span>{/if}
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
              class:active={config.position === pos.value}
              title={pos.label}
              onclick={() => {
                config.position = pos.value;
                markCustom();
              }}
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
              class:active={config.theme_color === color.hex}
              title={color.name}
              style={`--swatch-color:#${color.hex}`}
              onclick={() => {
                config.theme_color = color.hex;
                markCustom();
              }}
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
          bind:value={config.background_alpha}
          oninput={markCustom}
        />
      </label>

      <label class="checkbox-row">
        <input type="checkbox" bind:checked={config.round_corners} onchange={markCustom} />
        Abgerundete Ecken
      </label>

      <label class="checkbox-row">
        <input type="checkbox" bind:checked={config.horizontal} onchange={markCustom} />
        Horizontales Layout
      </label>

      <div class="control-group">
        <span class="tune-title">Leistungswerte</span>
        <div class="checkbox-list">
          <label class="checkbox-row">
            <input type="checkbox" bind:checked={config.show_fps} onchange={markCustom} />
            FPS-Zähler
          </label>
          <label class="checkbox-row">
            <input type="checkbox" bind:checked={config.show_frametime} onchange={markCustom} />
            Verlaufsgrafik
          </label>
          <label class="checkbox-row">
            <input type="checkbox" bind:checked={config.show_cpu} onchange={markCustom} />
            Prozessor
          </label>
          <label class="checkbox-row">
            <input type="checkbox" bind:checked={config.show_gpu} onchange={markCustom} />
            Grafikkarte
          </label>
          <label class="checkbox-row">
            <input type="checkbox" bind:checked={config.show_ram} onchange={markCustom} />
            Arbeitsspeicher
          </label>
          <label class="checkbox-row">
            <input type="checkbox" bind:checked={config.show_vram} onchange={markCustom} />
            Grafikspeicher
          </label>
          <label class="checkbox-row">
            <input type="checkbox" bind:checked={config.show_temps} onchange={markCustom} />
            Temperaturen
          </label>
        </div>
      </div>

      <div class="control-group">
        <span class="tune-title">Status-Icons</span>
        <div class="checkbox-list">
          <label class="checkbox-row">
            <input type="checkbox" bind:checked={config.show_gamemode} onchange={markCustom} />
            GameMode aktiv
          </label>
          <label class="checkbox-row">
            <input type="checkbox" bind:checked={config.show_vkbasalt} onchange={markCustom} />
            vkBasalt aktiv
          </label>
          <label class="checkbox-row">
            <input type="checkbox" bind:checked={config.show_hdr} onchange={markCustom} />
            HDR aktiv
          </label>
        </div>
      </div>

      <div class="control-group">
        <span class="tune-title">Technische Infos</span>
        <div class="checkbox-list">
          <label class="checkbox-row">
            <input type="checkbox" bind:checked={config.show_driver} onchange={markCustom} />
            Grafiktreiber
          </label>
          <label class="checkbox-row">
            <input type="checkbox" bind:checked={config.show_engine_version} onchange={markCustom} />
            DXVK/VKD3D-Version
          </label>
          <label class="checkbox-row">
            <input type="checkbox" bind:checked={config.show_wine} onchange={markCustom} />
            Wine/Proton-Version
          </label>
          <label class="checkbox-row">
            <input type="checkbox" bind:checked={config.show_gpu_name} onchange={markCustom} />
            Grafikkarten-Name
          </label>
          <label class="checkbox-row">
            <input type="checkbox" bind:checked={config.show_resolution} onchange={markCustom} />
            Auflösung
          </label>
        </div>
      </div>
    </div>
  </div>

  {#if error}
    <p class="error">{error}</p>
  {/if}

  <div class="save-row">
    <button type="button" class="primary" disabled={saving} onclick={handleSave}>
      {saving ? "Wird gespeichert…" : "Änderungen speichern"}
    </button>
    {#if saved}
      <span class="saved-hint">Gespeichert.</span>
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
    max-width: 46ch;
  }

  .switch {
    display: flex;
    align-items: center;
    gap: 0.5em;
    flex-direction: column;
    cursor: pointer;
    flex-shrink: 0;
  }

  .switch input {
    position: absolute;
    opacity: 0;
    width: 1px;
    height: 1px;
  }

  .track {
    width: 44px;
    height: 24px;
    border-radius: 999px;
    background: var(--border);
    position: relative;
    transition: background-color 0.15s;
  }

  .switch input:checked + .track {
    background: var(--accent);
  }

  .thumb {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: #fff;
    transition: transform 0.15s;
  }

  .switch input:checked + .track .thumb {
    transform: translateX(20px);
  }

  .switch-label {
    font-size: 0.75em;
    color: var(--text-muted);
    font-weight: 600;
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
