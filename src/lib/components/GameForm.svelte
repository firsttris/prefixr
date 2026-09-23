<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import { runners, refreshRunners } from "$lib/stores/runners";
  import { prefixes, refreshPrefixes } from "$lib/stores/prefixes";
  import { addGame, updateGame } from "$lib/stores/games";
  import { performanceConfig, refreshPerformanceConfig } from "$lib/stores/performance";
  import { mangoHudConfig, refreshMangoHudConfig } from "$lib/stores/mangohud";
  import { prettifyExeName } from "$lib/gameName";
  import { PROTON_OPTION_INFO, PROTON_OPTION_ORDER } from "$lib/protonOptions";
  import InfoIcon from "$lib/components/InfoIcon.svelte";
  import type {
    Game,
    GameInput,
    GamescopeSettings,
    ProtonOption,
    VkBasaltSettings,
  } from "$lib/types";

  let {
    existingGame,
    initialName,
    initialExePath,
    initialPrefixPath,
    initialRunnerId,
    onSuccess,
  }: {
    existingGame?: Game;
    // Prefills a fresh (non-`existingGame`) form, e.g. from a shortcut
    // detected right after running an installer — see InstallDialog.svelte.
    initialName?: string;
    initialExePath?: string;
    initialPrefixPath?: string;
    initialRunnerId?: string;
    onSuccess?: () => void;
  } = $props();

  function formatEnvVars(envVars: Record<string, string>): string {
    return Object.entries(envVars)
      .map(([key, value]) => `${key}=${value}`)
      .join("\n");
  }

  // Modal fully unmounts/remounts this form on every open, so reading
  // existingGame once here to seed the fields is intentional, not a bug.
  const defaults = untrack(() => ({
    name: existingGame?.name ?? initialName ?? "",
    exePath: existingGame?.exe_path ?? initialExePath ?? "",
    prefixPath: existingGame?.prefix_path ?? initialPrefixPath ?? "",
    runnerId: existingGame?.runner_id ?? initialRunnerId ?? "",
    envVarsText: existingGame ? formatEnvVars(existingGame.env_vars) : "",
    launchArgs: existingGame?.launch_args ?? "",
    overrides: existingGame?.overrides,
  }));

  let name = $state(defaults.name);
  let exePath = $state(defaults.exePath);
  let prefixPath = $state(defaults.prefixPath);
  let runnerId = $state(defaults.runnerId);
  let envVarsText = $state(defaults.envVarsText);
  let launchArgs = $state(defaults.launchArgs);
  // Per-game overrides; `null` inherits the global setting. The toggles
  // below always show the effective value; changing one (or one of the
  // vkBasalt/gamescope details) stores an override, which is dropped again
  // as soon as it matches the global setting — see `setMangohud` etc.
  let mangohudOverride = $state<boolean | null>(defaults.overrides?.mangohud_enabled ?? null);
  let gamemodeOverride = $state<boolean | null>(defaults.overrides?.gamemode_enabled ?? null);
  let vkbasalt = $state<VkBasaltSettings | null>(
    defaults.overrides?.vkbasalt ? { ...defaults.overrides.vkbasalt } : null,
  );
  let gamescope = $state<GamescopeSettings | null>(
    defaults.overrides?.gamescope ? { ...defaults.overrides.gamescope } : null,
  );
  // Proton switches this game sets, by variable name; a missing key keeps
  // Proton's default. Kept when switching to a Wine runner (which ignores
  // them), so switching back doesn't lose them.
  let protonOptions = $state<Record<string, boolean>>({
    ...(defaults.overrides?.proton_options ?? {}),
  });
  // What the selected runner's `proton` script understands.
  let availableProtonOptions = $state<ProtonOption[]>([]);
  let protonOptionsError = $state("");
  let error = $state("");
  let submitting = $state(false);

  const mangohudOn = $derived(mangohudOverride ?? $mangoHudConfig?.enabled ?? false);
  const gamemodeOn = $derived(gamemodeOverride ?? $performanceConfig?.gamemode_enabled ?? false);
  const vkbasaltShown = $derived(vkbasalt ?? globalVkbasalt());
  const gamescopeShown = $derived(gamescope ?? globalGamescope());
  const overrideCount = $derived(
    [mangohudOverride, gamemodeOverride, vkbasalt, gamescope].filter((o) => o !== null).length,
  );
  const isProton = $derived($runners.find((r) => r.id === runnerId)?.kind === "proton");
  const curatedProtonOptions = $derived(
    availableProtonOptions
      .filter((o) => o.config in PROTON_OPTION_INFO)
      .sort(
        (a, b) => PROTON_OPTION_ORDER.indexOf(a.config) - PROTON_OPTION_ORDER.indexOf(b.config),
      ),
  );
  const otherProtonOptions = $derived(
    availableProtonOptions.filter((o) => !(o.config in PROTON_OPTION_INFO)),
  );
  // Set on this game, but unknown to the selected runner — e.g. carried over
  // from a previous Proton version. Harmless, but shown so they can be
  // cleared.
  const unknownProtonOptions = $derived(
    Object.keys(protonOptions).filter(
      (name) => !availableProtonOptions.some((o) => o.env === name || o.aliases.includes(name)),
    ),
  );
  const protonOptionCount = $derived(Object.keys(protonOptions).length);

  // Starts expanded if the game already has overrides. Bound two-way rather
  // than passed as `open={...}`: Svelte re-applies every attribute of the
  // form in one shared effect, so a one-way value would snap the section
  // back to it on every toggle click.
  let overridesOpen = $state(untrack(() => overrideCount > 0));
  let protonOptionsOpen = $state(untrack(() => protonOptionCount > 0));

  $effect(() => {
    const id = runnerId;
    if (!isProton) {
      availableProtonOptions = [];
      protonOptionsError = "";
      return;
    }
    let cancelled = false;
    invoke<ProtonOption[]>("list_proton_options", { runnerId: id })
      .then((options) => {
        if (cancelled) return;
        availableProtonOptions = options;
        protonOptionsError = "";
      })
      .catch((e) => {
        if (cancelled) return;
        availableProtonOptions = [];
        protonOptionsError = String(e);
      });
    return () => {
      cancelled = true;
    };
  });

  onMount(() => {
    refreshRunners();
    refreshPrefixes();
    refreshPerformanceConfig();
    refreshMangoHudConfig();
  });

  function globalVkbasalt(): VkBasaltSettings {
    const global = $performanceConfig;
    return {
      enabled: global?.vkbasalt_enabled ?? false,
      sharpen: global?.vkbasalt_sharpen ?? true,
      sharpness: global?.vkbasalt_sharpness ?? 0.4,
      smaa: global?.vkbasalt_smaa ?? false,
      deband: global?.vkbasalt_deband ?? false,
    };
  }

  function globalGamescope(): GamescopeSettings {
    const global = $performanceConfig;
    return {
      enabled: global?.gamescope_enabled ?? false,
      width: global?.gamescope_width ?? null,
      height: global?.gamescope_height ?? null,
      fps_limit: global?.gamescope_fps_limit ?? null,
      fullscreen: global?.gamescope_fullscreen ?? false,
    };
  }

  function setMangohud(enabled: boolean) {
    mangohudOverride = enabled === $mangoHudConfig?.enabled ? null : enabled;
  }

  function setGamemode(enabled: boolean) {
    gamemodeOverride = enabled === $performanceConfig?.gamemode_enabled ? null : enabled;
  }

  // While both are switched off, the remaining values have no effect, so
  // they don't count as a difference.
  function sameVkbasalt(a: VkBasaltSettings, b: VkBasaltSettings): boolean {
    if (!a.enabled && !b.enabled) return true;
    return (
      a.enabled === b.enabled &&
      a.sharpen === b.sharpen &&
      Math.abs(a.sharpness - b.sharpness) < 1e-6 &&
      a.smaa === b.smaa &&
      a.deband === b.deband
    );
  }

  function sameGamescope(a: GamescopeSettings, b: GamescopeSettings): boolean {
    if (!a.enabled && !b.enabled) return true;
    return (
      a.enabled === b.enabled &&
      a.width === b.width &&
      a.height === b.height &&
      a.fps_limit === b.fps_limit &&
      a.fullscreen === b.fullscreen
    );
  }

  function editVkbasalt(patch: Partial<VkBasaltSettings>) {
    const next = { ...vkbasaltShown, ...patch };
    vkbasalt = sameVkbasalt(next, globalVkbasalt()) ? null : next;
  }

  function editGamescope(patch: Partial<GamescopeSettings>) {
    const next = { ...gamescopeShown, ...patch };
    gamescope = sameGamescope(next, globalGamescope()) ? null : next;
  }

  // Looks under every name the runner accepts for this switch, so a value
  // saved under an alias (or under a name an older runner preferred) shows up.
  function protonOptionValue(option: ProtonOption): boolean | null {
    for (const name of [option.env, ...option.aliases]) {
      if (name in protonOptions) return protonOptions[name];
    }
    return null;
  }

  function setProtonOption(option: ProtonOption, value: boolean | null) {
    const next = { ...protonOptions };
    for (const alias of option.aliases) delete next[alias];
    if (value === null) delete next[option.env];
    else next[option.env] = value;
    protonOptions = next;
  }

  function clearProtonOption(name: string) {
    const next = { ...protonOptions };
    delete next[name];
    protonOptions = next;
  }

  // Emptied number inputs read as NaN; 0 is no valid size or limit either.
  function numberOrNull(input: HTMLInputElement): number | null {
    const value = input.valueAsNumber;
    return Number.isFinite(value) && value > 0 ? value : null;
  }

  function resetTitle(globalOn: boolean | undefined): string {
    if (globalOn === undefined) return "Globale Einstellung übernehmen";
    return `Globale Einstellung übernehmen (${globalOn ? "an" : "aus"})`;
  }

  async function pickExe() {
    const selected = await open({
      filters: [{ name: "Programme", extensions: ["exe"] }],
    });
    if (typeof selected === "string") {
      exePath = selected;
      if (!name) {
        const fileName = selected.split(/[/\\]/).pop() ?? "";
        name = prettifyExeName(fileName);
      }
    }
  }

  function parseEnvVars(text: string): Record<string, string> {
    const result: Record<string, string> = {};
    for (const line of text.split("\n")) {
      const trimmed = line.trim();
      if (!trimmed) continue;
      const [key, ...rest] = trimmed.split("=");
      if (key && rest.length > 0) {
        result[key.trim()] = rest.join("=").trim();
      }
    }
    return result;
  }

  async function handleSubmit(event: Event) {
    event.preventDefault();
    if (!name || !exePath || !prefixPath || !runnerId) return;
    error = "";
    submitting = true;
    const input: GameInput = {
      name,
      exe_path: exePath,
      prefix_path: prefixPath,
      runner_id: runnerId,
      env_vars: parseEnvVars(envVarsText),
      launch_args: launchArgs.trim(),
      overrides: {
        mangohud_enabled: mangohudOverride,
        gamemode_enabled: gamemodeOverride,
        vkbasalt,
        gamescope,
        proton_options: protonOptions,
      },
    };
    try {
      if (existingGame) {
        await updateGame(existingGame.id, input);
      } else {
        await addGame(input);
      }
      onSuccess?.();
    } catch (e) {
      error = String(e);
    } finally {
      submitting = false;
    }
  }
</script>

<form onsubmit={handleSubmit}>
  <label>
    Name
    <input bind:value={name} placeholder="z. B. Baldur's Gate 3" />
  </label>

  <label>
    Programm (.exe)
    <div class="row">
      <input bind:value={exePath} readonly placeholder="Noch keine Datei gewählt" />
      <button type="button" onclick={pickExe}>Wählen…</button>
    </div>
  </label>

  <label>
    Prefix
    <select bind:value={prefixPath}>
      <option value="" disabled selected>Prefix wählen</option>
      {#each $prefixes as prefix (prefix.path)}
        <option value={prefix.path}>{prefix.path}</option>
      {/each}
    </select>
    {#if $prefixes.length === 0}
      <span class="hint">Noch kein Prefix vorhanden — leg zuerst einen unter "Prefixe & Runner" an.</span>
    {/if}
  </label>

  <label>
    Runner
    <select bind:value={runnerId}>
      <option value="" disabled selected>Runner wählen</option>
      {#each $runners as runner (runner.id)}
        <option value={runner.id}>{runner.name} ({runner.kind})</option>
      {/each}
    </select>
  </label>

  <label>
    Umgebungsvariablen
    <textarea placeholder={"KEY=WERT, eine pro Zeile"} bind:value={envVarsText}></textarea>
  </label>

  <label>
    Startparameter
    <input placeholder="z. B. --launcher-skip -dx11" bind:value={launchArgs} />
  </label>

  <details class="overrides" bind:open={overridesOpen}>
    <summary>
      Leistung &amp; Overlay
      {#if overrideCount > 0}
        <span class="badge">{overrideCount} angepasst</span>
      {/if}
    </summary>
    <p class="hint">
      Standardmäßig gelten die globalen Einstellungen. Hier kannst du sie nur für dieses Spiel
      überschreiben.
    </p>

    <div class="toggle-list">
      <div class="toggle-row" class:overridden={mangohudOverride !== null}>
        <div>
          <span class="toggle-label">MangoHud</span>
          <p class="toggle-desc">Zeigt FPS, Auslastung und Temperaturen direkt im Spiel an.</p>
        </div>
        <div class="toggle-controls">
          {#if mangohudOverride !== null}
            <button
              type="button"
              class="reset"
              title={resetTitle($mangoHudConfig?.enabled)}
              onclick={() => (mangohudOverride = null)}
            >
              Zurücksetzen
            </button>
          {/if}
          <label class="switch">
            <input
              type="checkbox"
              checked={mangohudOn}
              onchange={(e) => setMangohud(e.currentTarget.checked)}
            />
            <span class="track"><span class="thumb"></span></span>
          </label>
        </div>
      </div>

      <div class="toggle-row" class:overridden={gamemodeOverride !== null}>
        <div>
          <span class="toggle-label">
            GameMode
            <InfoIcon
              text="Empfehlung: Wenn installiert, ruhig aktivieren — bringt oft spürbar mehr Leistung, besonders auf Laptops oder mit Energiesparmodus. Kein Nachteil, wenn GameMode fehlt. Läuft gerade ein konkurrierender Scheduler-Daemon (z. B. ananicy-cpp, scx), wird GameMode automatisch übersprungen und stattdessen auf das Performance-Energieprofil ausgewichen, um Konflikte um CPU-Priorität zu vermeiden."
            />
          </span>
          <p class="toggle-desc">Optimiert CPU-Takt und Priorität, solange das Spiel läuft.</p>
        </div>
        <div class="toggle-controls">
          {#if gamemodeOverride !== null}
            <button
              type="button"
              class="reset"
              title={resetTitle($performanceConfig?.gamemode_enabled)}
              onclick={() => (gamemodeOverride = null)}
            >
              Zurücksetzen
            </button>
          {/if}
          <label class="switch">
            <input
              type="checkbox"
              checked={gamemodeOn}
              onchange={(e) => setGamemode(e.currentTarget.checked)}
            />
            <span class="track"><span class="thumb"></span></span>
          </label>
        </div>
      </div>

      <div class="toggle-group" class:overridden={gamescope !== null}>
        <div class="toggle-row">
          <div>
            <span class="toggle-label">
              Gamescope
              <InfoIcon
                text="Empfehlung: Für Handhelds/TV-Setups oder um Auflösung und FPS-Limit unabhängig vom Spiel zu erzwingen. Braucht das gamescope-Paket; ohne das passiert einfach nichts. Wird übersprungen, wenn prefixr selbst schon in einer Gamescope-Session läuft."
              />
            </span>
            <p class="toggle-desc">
              Startet das Spiel in einer eigenen, verschachtelten Compositor-Session.
            </p>
          </div>
          <div class="toggle-controls">
            {#if gamescope !== null}
              <button
                type="button"
                class="reset"
                title={resetTitle($performanceConfig?.gamescope_enabled)}
                onclick={() => (gamescope = null)}
              >
                Zurücksetzen
              </button>
            {/if}
            <label class="switch">
              <input
                type="checkbox"
                checked={gamescopeShown.enabled}
                onchange={(e) => editGamescope({ enabled: e.currentTarget.checked })}
              />
              <span class="track"><span class="thumb"></span></span>
            </label>
          </div>
        </div>
        {#if gamescopeShown.enabled}
          <div class="details">
            <div class="detail-grid">
              <label>
                Breite (px)
                <input
                  type="number"
                  min="0"
                  placeholder="native"
                  value={gamescopeShown.width ?? ""}
                  oninput={(e) => editGamescope({ width: numberOrNull(e.currentTarget) })}
                />
              </label>
              <label>
                Höhe (px)
                <input
                  type="number"
                  min="0"
                  placeholder="native"
                  value={gamescopeShown.height ?? ""}
                  oninput={(e) => editGamescope({ height: numberOrNull(e.currentTarget) })}
                />
              </label>
              <label>
                FPS-Limit
                <input
                  type="number"
                  min="0"
                  placeholder="unbegrenzt"
                  value={gamescopeShown.fps_limit ?? ""}
                  oninput={(e) => editGamescope({ fps_limit: numberOrNull(e.currentTarget) })}
                />
              </label>
            </div>
            <label class="check">
              <input
                type="checkbox"
                checked={gamescopeShown.fullscreen}
                onchange={(e) => editGamescope({ fullscreen: e.currentTarget.checked })}
              />
              Vollbild erzwingen
            </label>
          </div>
        {/if}
      </div>

      <div class="toggle-group" class:overridden={vkbasalt !== null}>
        <div class="toggle-row">
          <div>
            <span class="toggle-label">
              vkBasalt
              <InfoIcon
                text="Empfehlung: Kostet immer etwas Leistung (zusätzlicher Bildbearbeitungsschritt) — nur aktivieren, wenn du GPU-Leistung übrig hast und dir das Ergebnis optisch wichtiger ist als die letzten FPS."
              />
            </span>
            <p class="toggle-desc">Bildnachbearbeitung direkt im Spiel — kostet etwas Leistung.</p>
          </div>
          <div class="toggle-controls">
            {#if vkbasalt !== null}
              <button
                type="button"
                class="reset"
                title={resetTitle($performanceConfig?.vkbasalt_enabled)}
                onclick={() => (vkbasalt = null)}
              >
                Zurücksetzen
              </button>
            {/if}
            <label class="switch">
              <input
                type="checkbox"
                checked={vkbasaltShown.enabled}
                onchange={(e) => editVkbasalt({ enabled: e.currentTarget.checked })}
              />
              <span class="track"><span class="thumb"></span></span>
            </label>
          </div>
        </div>
        {#if vkbasaltShown.enabled}
          <div class="details">
            <label class="check">
              <input
                type="checkbox"
                checked={vkbasaltShown.sharpen}
                onchange={(e) => editVkbasalt({ sharpen: e.currentTarget.checked })}
              />
              Schärfen (CAS)
            </label>
            {#if vkbasaltShown.sharpen}
              <label>
                Stärke: {vkbasaltShown.sharpness.toFixed(2)}
                <input
                  type="range"
                  min="0"
                  max="1"
                  step="0.05"
                  value={vkbasaltShown.sharpness}
                  oninput={(e) => editVkbasalt({ sharpness: e.currentTarget.valueAsNumber })}
                />
              </label>
            {/if}
            <label class="check">
              <input
                type="checkbox"
                checked={vkbasaltShown.smaa}
                onchange={(e) => editVkbasalt({ smaa: e.currentTarget.checked })}
              />
              Kantenglättung (SMAA)
            </label>
            <label class="check">
              <input
                type="checkbox"
                checked={vkbasaltShown.deband}
                onchange={(e) => editVkbasalt({ deband: e.currentTarget.checked })}
              />
              Farbverläufe glätten (Deband)
            </label>
          </div>
        {/if}
      </div>
    </div>
  </details>

  {#if isProton}
    <details class="overrides" bind:open={protonOptionsOpen}>
      <summary>
        Proton-Optionen
        {#if protonOptionCount > 0}
          <span class="badge">{protonOptionCount} gesetzt</span>
        {/if}
      </summary>
      <p class="hint">
        Schalter, die der gewählte Runner kennt. „Standard“ überlässt die Entscheidung Proton und
        den protonfixes. Einträge unter „Umgebungsvariablen“ haben Vorrang.
      </p>

      {#if protonOptionsError}
        <p class="error">{protonOptionsError}</p>
      {/if}

      {#snippet tristate(option: ProtonOption)}
        {@const value = protonOptionValue(option)}
        <div class="tristate" role="group" aria-label={option.env}>
          <button
            type="button"
            aria-pressed={value === null}
            onclick={() => setProtonOption(option, null)}
          >
            Standard
          </button>
          <button
            type="button"
            aria-pressed={value === true}
            onclick={() => setProtonOption(option, true)}
          >
            An
          </button>
          <button
            type="button"
            aria-pressed={value === false}
            onclick={() => setProtonOption(option, false)}
          >
            Aus
          </button>
        </div>
      {/snippet}

      <div class="toggle-list">
        {#each curatedProtonOptions as option (option.config)}
          {@const info = PROTON_OPTION_INFO[option.config]}
          <div class="toggle-row" class:overridden={protonOptionValue(option) !== null}>
            <div>
              <span class="toggle-label">{info.label}</span>
              <p class="toggle-desc">{info.description}</p>
              <code class="env-name">{option.env}</code>
            </div>
            <div class="toggle-controls">
              {@render tristate(option)}
            </div>
          </div>
        {/each}
      </div>

      {#if otherProtonOptions.length > 0}
        <details class="advanced">
          <summary>Erweitert ({otherProtonOptions.length})</summary>
          <p class="hint">
            Weitere Schalter aus dem Skript des Runners, ohne Beschreibung. Nur ändern, wenn ein
            Fix für ein Spiel genau das verlangt.
          </p>
          <div class="compact-list">
            {#each otherProtonOptions as option (option.config)}
              <div class="compact-row" class:overridden={protonOptionValue(option) !== null}>
                <code title={option.aliases.length ? `auch: ${option.aliases.join(", ")}` : ""}>
                  {option.env}
                </code>
                {@render tristate(option)}
              </div>
            {/each}
          </div>
        </details>
      {/if}

      {#if availableProtonOptions.length > 0 && unknownProtonOptions.length > 0}
        <div class="compact-list">
          <p class="hint">Gesetzt, aber vom gewählten Runner nicht erkannt:</p>
          {#each unknownProtonOptions as name (name)}
            <div class="compact-row overridden">
              <code>{name}={protonOptions[name] ? "1" : "0"}</code>
              <button type="button" class="reset" onclick={() => clearProtonOption(name)}>
                Entfernen
              </button>
            </div>
          {/each}
        </div>
      {/if}
    </details>
  {/if}

  {#if error}
    <p class="error">{error}</p>
  {/if}

  <button type="submit" class="primary" disabled={submitting}>
    {#if submitting}
      {existingGame ? "Wird gespeichert…" : "Wird hinzugefügt…"}
    {:else}
      {existingGame ? "Speichern" : "Hinzufügen"}
    {/if}
  </button>
</form>

<style>
  form {
    display: flex;
    flex-direction: column;
    gap: 1em;
  }

  .row {
    display: flex;
    gap: 0.5em;
  }

  .row input {
    flex: 1;
  }

  textarea {
    min-height: 4em;
    resize: vertical;
  }

  .error {
    color: var(--danger);
  }

  .hint {
    color: var(--text-muted);
    font-size: 0.85em;
  }

  .overrides {
    background: var(--surface-raised);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 0.6em 0.8em;
  }

  .overrides[open] {
    display: flex;
    flex-direction: column;
    gap: 0.8em;
  }

  summary {
    cursor: pointer;
    color: var(--text-muted);
    font-weight: 600;
    font-size: 0.9em;
  }

  .badge {
    margin-left: 0.4em;
    padding: 0.1em 0.5em;
    border-radius: 999px;
    background: var(--accent);
    color: #fff;
    font-size: 0.8em;
    font-weight: 600;
  }

  .overrides .hint {
    margin: 0;
  }

  .toggle-list {
    display: flex;
    flex-direction: column;
    gap: 0.6em;
  }

  .toggle-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1em;
    padding: 0.6em 0.8em;
    border-radius: 10px;
    background: var(--surface);
    border: 1px solid var(--border);
  }

  .toggle-group {
    border-radius: 10px;
    background: var(--surface);
    border: 1px solid var(--border);
  }

  .toggle-group .toggle-row {
    border: none;
    background: none;
  }

  .toggle-row.overridden,
  .toggle-group.overridden {
    border-color: var(--accent);
  }

  .toggle-label {
    font-weight: 600;
    color: var(--text);
    display: block;
  }

  .toggle-desc {
    color: var(--text-muted);
    font-size: 0.82em;
    margin: 0.25em 0 0;
  }

  .toggle-controls {
    display: flex;
    align-items: center;
    gap: 0.6em;
    flex-shrink: 0;
  }

  .reset {
    padding: 0.25em 0.6em;
    font-size: 0.8em;
  }

  .switch {
    flex-direction: row;
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

  .details {
    display: flex;
    flex-direction: column;
    gap: 0.6em;
    padding: 0 0.8em 0.8em;
  }

  .detail-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(8em, 1fr));
    gap: 0.6em;
  }

  .detail-grid input {
    width: 100%;
  }

  .env-name {
    display: inline-block;
    margin-top: 0.3em;
    color: var(--text-muted);
    font-size: 0.75em;
  }

  .tristate {
    display: flex;
    border: 1px solid var(--border);
    border-radius: 8px;
    overflow: hidden;
  }

  .tristate button {
    border: none;
    border-radius: 0;
    padding: 0.3em 0.7em;
    font-size: 0.8em;
    background: none;
  }

  .tristate button + button {
    border-left: 1px solid var(--border);
  }

  .tristate button[aria-pressed="true"] {
    background: var(--accent);
    color: #fff;
  }

  .advanced summary {
    font-weight: 500;
  }

  .advanced[open] {
    display: flex;
    flex-direction: column;
    gap: 0.6em;
  }

  .compact-list {
    display: flex;
    flex-direction: column;
    gap: 0.3em;
  }

  .compact-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1em;
    padding: 0.3em 0.6em;
    border-radius: 8px;
    border: 1px solid transparent;
  }

  .compact-row code {
    font-size: 0.8em;
    overflow-wrap: anywhere;
  }

  .compact-row.overridden {
    border-color: var(--accent);
  }

  .check {
    flex-direction: row;
    align-items: center;
    gap: 0.5em;
  }

  .check input {
    padding: 0;
  }
</style>
