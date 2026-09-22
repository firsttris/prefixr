<script lang="ts">
  import { onMount } from "svelte";
  import {
    performanceConfig,
    refreshPerformanceConfig,
    savePerformanceConfig,
  } from "$lib/stores/performance";
  import { maxMapCountStatus, refreshMaxMapCountStatus, fixMaxMapCount } from "$lib/stores/system";
  import type { PerformanceConfig } from "$lib/types";
  import InfoIcon from "$lib/components/InfoIcon.svelte";

  const FALLBACK: PerformanceConfig = {
    gamemode_enabled: false,
    vkbasalt_enabled: false,
    vkbasalt_sharpen: true,
    vkbasalt_sharpness: 0.4,
    vkbasalt_smaa: false,
    vkbasalt_deband: false,
    inhibit_sleep_enabled: false,
    power_profile_enabled: false,
  };

  let config = $state<PerformanceConfig>({ ...FALLBACK });
  let initialized = $state(false);
  let saving = $state(false);
  let error = $state("");
  let saved = $state(false);

  let fixingMapCount = $state(false);
  let mapCountFixError = $state("");

  onMount(() => {
    refreshPerformanceConfig();
    refreshMaxMapCountStatus();
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

  $effect(() => {
    if (!initialized && $performanceConfig) {
      config = { ...$performanceConfig };
      initialized = true;
    }
  });

  function changed() {
    saved = false;
  }

  async function handleSave() {
    saving = true;
    error = "";
    saved = false;
    try {
      await savePerformanceConfig(config);
      saved = true;
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }

  let previewFilter = $derived.by(() => {
    let filter = "";
    if (config.vkbasalt_enabled && config.vkbasalt_sharpen) {
      filter += ` contrast(${1 + config.vkbasalt_sharpness * 0.35}) saturate(${1 + config.vkbasalt_sharpness * 0.2})`;
    }
    if (config.vkbasalt_enabled && config.vkbasalt_smaa) {
      filter += " blur(0.4px)";
    }
    return filter.trim() || "none";
  });
</script>

<section class="panel">
  {#if $maxMapCountStatus && !$maxMapCountStatus.sufficient}
    <div class="warning-banner">
      <div>
        <strong>vm.max_map_count zu niedrig</strong>
        <p>
          Aktuell {$maxMapCountStatus.current.toLocaleString("de-DE")}, empfohlen mindestens
          {$maxMapCountStatus.recommended.toLocaleString("de-DE")}. Mehrere moderne Spiele (u. a.
          mit Easy Anti-Cheat, Elden Ring, Baldur's Gate 3, Diablo IV) stürzen darunter direkt
          beim Start ab. Steam setzt diesen Wert selbst systemweit — Spiele außerhalb von Steam
          bekommen ihn ohne diesen Fix nicht.
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
          {fixingMapCount ? "Wird angewendet…" : "Jetzt erhöhen"}
        </button>
      {/if}
    </div>
  {/if}

  <div class="section">
    <h2>Performance</h2>
    <p class="hint">
      Systemweite Optimierungen, gelten für alle Spiele. Setzt jeweils voraus, dass das
      zugrunde liegende Tool auf deinem System installiert ist.
    </p>

    <div class="toggle-list">
      <div class="toggle-row">
        <div>
          <span class="toggle-label">
            GameMode
            <InfoIcon
              text="Empfehlung: Wenn installiert, ruhig aktivieren — bringt oft spürbar mehr Leistung, besonders auf Laptops oder mit Energiesparmodus. Kein Nachteil, wenn GameMode fehlt."
            />
          </span>
          <p class="toggle-desc">
            Optimiert CPU-Takt und Priorität, solange das Spiel läuft.
          </p>
        </div>
        <label class="switch">
          <input type="checkbox" bind:checked={config.gamemode_enabled} onchange={changed} />
          <span class="track"><span class="thumb"></span></span>
        </label>
      </div>

      <div class="toggle-row">
        <div>
          <span class="toggle-label">
            Performance-Energieprofil
            <InfoIcon
              text="Empfehlung: Auf Laptops praktisch immer sinnvoll, auf Desktops ohne Energieprofile meist wirkungslos. Ergänzt GameMode statt es zu ersetzen — GameMode setzt den CPU-Governor selbst, was der Power-Profile-Dienst auf vielen Distros wieder überschreibt. Braucht power-profiles-daemon; ohne das passiert einfach nichts."
            />
          </span>
          <p class="toggle-desc">
            Hält das Energieprofil auf „Leistung", solange ein Spiel läuft.
          </p>
        </div>
        <label class="switch">
          <input type="checkbox" bind:checked={config.power_profile_enabled} onchange={changed} />
          <span class="track"><span class="thumb"></span></span>
        </label>
      </div>

      <div class="toggle-row">
        <div>
          <span class="toggle-label">
            Ruhezustand verhindern
            <InfoIcon
              text="Empfehlung: Praktisch immer sinnvoll — verhindert, dass der Bildschirmschoner oder der Energiesparmodus mitten im Spiel zuschlägt. Braucht systemd-inhibit; ohne das passiert einfach nichts."
            />
          </span>
          <p class="toggle-desc">
            Hält System und Bildschirm wach, solange ein Spiel läuft.
          </p>
        </div>
        <label class="switch">
          <input type="checkbox" bind:checked={config.inhibit_sleep_enabled} onchange={changed} />
          <span class="track"><span class="thumb"></span></span>
        </label>
      </div>
    </div>
  </div>

  <div class="section">
    <h2>vkBasalt (Bildeffekte)</h2>
    <p class="hint">
      Bildnachbearbeitung direkt im Spiel — kostet etwas Leistung, im Gegensatz zu den
      Optionen oben.
    </p>

    <div class="toggle-row">
      <div>
        <span class="toggle-label">
          vkBasalt aktivieren
          <InfoIcon
            text="Empfehlung: Kostet immer etwas Leistung (zusätzlicher Bildbearbeitungsschritt) — nur aktivieren, wenn du GPU-Leistung übrig hast und dir das Ergebnis optisch wichtiger ist als die letzten FPS."
          />
        </span>
        <p class="toggle-desc">Schaltet die Effekte unten für alle Spiele frei.</p>
      </div>
      <label class="switch">
        <input type="checkbox" bind:checked={config.vkbasalt_enabled} onchange={changed} />
        <span class="track"><span class="thumb"></span></span>
      </label>
    </div>

    {#if config.vkbasalt_enabled}
      <div class="vkbasalt-grid">
        <div class="preview-wrap">
          <span class="tune-title">Vorschau (Annäherung)</span>
          <div class="preview-screen">
            <div class="preview-image" style={`filter:${previewFilter}`}></div>
          </div>
        </div>

        <div class="effects">
          <div class="checkbox-row">
            <label>
              <input type="checkbox" bind:checked={config.vkbasalt_sharpen} onchange={changed} />
              Schärfen (CAS)
            </label>
            <InfoIcon
              text="Empfehlung: Kaum Leistungseinbruch — lohnt sich besonders bei Upscaling oder niedrigerer Auflösung, um Schärfe zurückzugewinnen."
            />
          </div>
          {#if config.vkbasalt_sharpen}
            <label class="control-group sharpness">
              <span class="tune-title">Stärke</span>
              <input
                type="range"
                min="0"
                max="1"
                step="0.05"
                bind:value={config.vkbasalt_sharpness}
                oninput={changed}
              />
            </label>
          {/if}

          <div class="checkbox-row">
            <label>
              <input type="checkbox" bind:checked={config.vkbasalt_smaa} onchange={changed} />
              Kantenglättung (SMAA)
            </label>
            <InfoIcon
              text="Empfehlung: Spürbarer Leistungseinbruch möglich. Wenn das Spiel schon eigene Kantenglättung hat, hier eher weglassen — sonst kostet es nur zusätzlich Leistung."
            />
          </div>

          <div class="checkbox-row">
            <label>
              <input type="checkbox" bind:checked={config.vkbasalt_deband} onchange={changed} />
              Farbverläufe glätten (Deband)
            </label>
            <InfoIcon
              text="Empfehlung: Sehr geringe Kosten — nur sinnvoll bei sichtbaren Farbstufen (z. B. im Himmel), sonst kannst du es weglassen."
            />
          </div>
        </div>
      </div>
    {/if}
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
    gap: 1.8em;
    max-width: 780px;
  }

  .section {
    display: flex;
    flex-direction: column;
    gap: 0.9em;
  }

  .hint {
    color: var(--text-muted);
    font-size: 0.85em;
    max-width: 56ch;
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

  .toggle-list {
    display: flex;
    flex-direction: column;
    gap: 0.9em;
  }

  .toggle-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1em;
    padding: 0.7em 0.9em;
    border-radius: 10px;
    background: var(--surface-raised);
    border: 1px solid var(--border);
  }

  .toggle-label {
    font-weight: 600;
    display: block;
  }

  .toggle-desc {
    color: var(--text-muted);
    font-size: 0.82em;
    margin-top: 0.25em;
    max-width: 48ch;
  }

  .switch {
    display: flex;
    align-items: center;
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

  .vkbasalt-grid {
    display: grid;
    grid-template-columns: 1.1fr 1fr;
    gap: 1.4em;
    align-items: start;
  }

  @media (max-width: 640px) {
    .vkbasalt-grid {
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
    width: 100%;
    height: 260px;
    border-radius: var(--radius);
    overflow: hidden;
    border: 1px solid var(--border);
  }

  .preview-image {
    width: 100%;
    height: 100%;
    background:
      radial-gradient(circle at 25% 30%, #ff9f6b, transparent 45%),
      radial-gradient(circle at 70% 60%, #5b8cff, transparent 50%),
      linear-gradient(160deg, #1c2436, #0b0d13 80%);
    transition: filter 0.15s;
  }

  .effects {
    display: flex;
    flex-direction: column;
    gap: 0.8em;
    min-width: 0;
  }

  .checkbox-row {
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 0.5em;
    font-size: 0.9em;
    color: var(--text);
  }

  .checkbox-row label {
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 0.5em;
    color: inherit;
    font-size: inherit;
  }

  .checkbox-row input {
    width: auto;
  }

  .control-group {
    display: flex;
    flex-direction: column;
  }

  .sharpness {
    padding-left: 1.6em;
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
