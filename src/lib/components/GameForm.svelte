<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { runners, refreshRunners } from "$lib/stores/runners";
  import { prefixes, refreshPrefixes } from "$lib/stores/prefixes";
  import { addGame, updateGame } from "$lib/stores/games";
  import { performanceConfig, refreshPerformanceConfig } from "$lib/stores/performance";
  import { graphicsConfig, refreshGraphicsConfig } from "$lib/stores/graphics";
  import { mangoHudConfig, refreshMangoHudConfig } from "$lib/stores/mangohud";
  import { listProtonOptions, protonConfig, refreshProtonConfig } from "$lib/stores/proton";
  import { prettifyExeName } from "$lib/gameName";
  import { PRESETS } from "$lib/mangohudPresets";
  import { onOff, sameBlock, sameFields, type OverrideHooks } from "$lib/settings";
  import { backendError, t } from "$lib/i18n/index.svelte";
  import PerformanceEditor from "$lib/components/PerformanceEditor.svelte";
  import GraphicsEditor from "$lib/components/GraphicsEditor.svelte";
  import OverlayEditor from "$lib/components/OverlayEditor.svelte";
  import ProtonEditor from "$lib/components/ProtonEditor.svelte";
  import UmuIdPicker from "$lib/components/UmuIdPicker.svelte";
  import type {
    Game,
    GameInput,
    GamescopeSettings,
    GraphicsConfig,
    GraphicsOverrides,
    MangoHudConfig,
    MangoHudLayout,
    OverlayOverrides,
    PerformanceConfig,
    PerformanceOverrides,
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
    umuId: existingGame?.umu_id ?? null,
    umuStore: existingGame?.umu_store ?? null,
    overrides: existingGame?.overrides,
  }));

  let name = $state(defaults.name);
  let exePath = $state(defaults.exePath);
  let prefixPath = $state(defaults.prefixPath);
  let runnerId = $state(defaults.runnerId);
  let envVarsText = $state(defaults.envVarsText);
  let launchArgs = $state(defaults.launchArgs);
  let umuId = $state<string | null>(defaults.umuId);
  let umuStore = $state<string | null>(defaults.umuStore);
  let error = $state<unknown>(null);
  let submitting = $state(false);

  // Per-game overrides, one block per settings category; `null` (or a
  // missing Proton key) inherits the global setting. The editors always get
  // the effective values; a change is stored as an override, which is
  // dropped again as soon as it matches the global setting.
  const saved = defaults.overrides;
  let perf = $state<PerformanceOverrides>({
    gamemode_enabled: saved?.performance.gamemode_enabled ?? null,
    power_profile_enabled: saved?.performance.power_profile_enabled ?? null,
    inhibit_sleep_enabled: saved?.performance.inhibit_sleep_enabled ?? null,
  });
  let gfx = $state<GraphicsOverrides>({
    gamescope: saved?.graphics.gamescope ? { ...saved.graphics.gamescope } : null,
    vkbasalt: saved?.graphics.vkbasalt ? { ...saved.graphics.vkbasalt } : null,
  });
  let overlay = $state<OverlayOverrides>({
    enabled: saved?.overlay.enabled ?? null,
    layout: saved?.overlay.layout ? { ...saved.overlay.layout } : null,
  });
  // Kept when switching to a Wine runner (which ignores them), so switching
  // back doesn't lose them.
  let proton = $state<Record<string, boolean>>({ ...(saved?.proton ?? {}) });

  onMount(() => {
    refreshRunners();
    refreshPrefixes();
    refreshPerformanceConfig();
    refreshGraphicsConfig();
    refreshMangoHudConfig();
    refreshProtonConfig();
  });

  // --- Leistung ---

  const PERF_KEYS = ["gamemode_enabled", "power_profile_enabled", "inhibit_sleep_enabled"] as const;
  const globalPerf = $derived<PerformanceConfig>(
    $performanceConfig ?? {
      gamemode_enabled: false,
      power_profile_enabled: false,
      inhibit_sleep_enabled: false,
    },
  );
  const perfShown = $derived<PerformanceConfig>({
    gamemode_enabled: perf.gamemode_enabled ?? globalPerf.gamemode_enabled,
    power_profile_enabled: perf.power_profile_enabled ?? globalPerf.power_profile_enabled,
    inhibit_sleep_enabled: perf.inhibit_sleep_enabled ?? globalPerf.inhibit_sleep_enabled,
  });
  const perfCount = $derived(PERF_KEYS.filter((k) => perf[k] !== null).length);

  function changePerf(patch: Partial<PerformanceConfig>) {
    for (const key of PERF_KEYS) {
      const value = patch[key];
      if (value !== undefined) perf[key] = value === globalPerf[key] ? null : value;
    }
  }

  const perfHooks: OverrideHooks<keyof PerformanceConfig> = {
    isOverridden: (key) => perf[key] !== null,
    reset: (key) => (perf[key] = null),
    resetTitle: (key) => t("gameForm.resetToGlobal", { state: onOff(globalPerf[key]) }),
  };

  // --- Bild ---

  const globalGfx = $derived<GraphicsConfig>(
    $graphicsConfig ?? {
      gamescope: { enabled: false, width: null, height: null, fps_limit: null, fullscreen: false },
      vkbasalt: { enabled: false, sharpen: true, sharpness: 0.4, smaa: false },
    },
  );
  const gfxShown = $derived<GraphicsConfig>({
    gamescope: gfx.gamescope ?? globalGfx.gamescope,
    vkbasalt: gfx.vkbasalt ?? globalGfx.vkbasalt,
  });
  const gfxCount = $derived([gfx.gamescope, gfx.vkbasalt].filter((o) => o !== null).length);

  function changeGfx(patch: {
    gamescope?: Partial<GamescopeSettings>;
    vkbasalt?: Partial<VkBasaltSettings>;
  }) {
    if (patch.gamescope) {
      const next = { ...gfxShown.gamescope, ...patch.gamescope };
      gfx.gamescope = sameBlock(next, globalGfx.gamescope) ? null : next;
    }
    if (patch.vkbasalt) {
      const next = { ...gfxShown.vkbasalt, ...patch.vkbasalt };
      gfx.vkbasalt = sameBlock(next, globalGfx.vkbasalt) ? null : next;
    }
  }

  const gfxHooks: OverrideHooks<keyof GraphicsConfig> = {
    isOverridden: (key) => gfx[key] !== null,
    reset: (key) => (gfx[key] = null),
    resetTitle: (key) => t("gameForm.resetToGlobal", { state: onOff(globalGfx[key].enabled) }),
  };

  // --- Overlay ---

  function layoutOf(config: MangoHudConfig): MangoHudLayout {
    const layout: Partial<MangoHudConfig> = { ...config };
    delete layout.enabled;
    return layout as MangoHudLayout;
  }

  const standardPreset = PRESETS.find((p) => p.key === "standard")!;
  const globalOverlay = $derived<MangoHudConfig>(
    $mangoHudConfig ?? { enabled: false, preset: "standard", ...standardPreset.values },
  );
  const overlayShown = $derived<MangoHudConfig>({
    ...(overlay.layout ?? layoutOf(globalOverlay)),
    enabled: overlay.enabled ?? globalOverlay.enabled,
  });
  const overlayCount = $derived(
    [overlay.enabled, overlay.layout].filter((o) => o !== null).length,
  );

  function changeOverlay(patch: Partial<MangoHudConfig>) {
    const { enabled, ...layoutPatch } = patch;
    if (enabled !== undefined) {
      overlay.enabled = enabled === globalOverlay.enabled ? null : enabled;
    }
    if (Object.keys(layoutPatch).length > 0) {
      const next = { ...layoutOf(overlayShown), ...layoutPatch };
      // The preset name alone ("custom" after a manual tweak) is no reason
      // to keep an override whose values match the global look.
      const same = sameFields(
        { ...next, preset: "" },
        { ...layoutOf(globalOverlay), preset: "" },
      );
      overlay.layout = same ? null : next;
    }
  }

  const overlayHooks: OverrideHooks<"enabled" | "layout"> = {
    isOverridden: (key) => overlay[key] !== null,
    reset: (key) => (overlay[key] = null),
    resetTitle: (key) =>
      key === "enabled"
        ? t("gameForm.resetToGlobal", { state: onOff(globalOverlay.enabled) })
        : t("gameForm.resetOverlayLook"),
  };

  // --- Proton ---

  const isProton = $derived($runners.find((r) => r.id === runnerId)?.kind === "proton");
  let protonOptions = $state<ProtonOption[]>([]);
  let protonOptionsError = $state<unknown>(null);
  const protonCount = $derived(Object.keys(proton).length);

  $effect(() => {
    const id = runnerId;
    if (!isProton) {
      protonOptions = [];
      protonOptionsError = null;
      return;
    }
    let cancelled = false;
    listProtonOptions(id)
      .then((options) => {
        if (cancelled) return;
        protonOptions = options;
        protonOptionsError = null;
      })
      .catch((e) => {
        if (cancelled) return;
        protonOptions = [];
        protonOptionsError = e;
      });
    return () => {
      cancelled = true;
    };
  });

  // --- Tabs ---

  type Tab = "general" | "performance" | "graphics" | "overlay" | "proton";
  let tab = $state<Tab>("general");
  const tabs = $derived(
    [
      { id: "general" as Tab, label: t("gameForm.tabGeneral"), count: 0 },
      { id: "performance" as Tab, label: t("nav.performance"), count: perfCount },
      { id: "graphics" as Tab, label: t("nav.graphics"), count: gfxCount },
      { id: "overlay" as Tab, label: t("nav.overlay"), count: overlayCount },
      { id: "proton" as Tab, label: t("nav.proton"), count: protonCount },
    ].filter((tabItem) => tabItem.id !== "proton" || isProton),
  );
  const activeTab = $derived(tabs.some((t) => t.id === tab) ? tab : "general");

  async function pickExe() {
    const selected = await open({
      filters: [{ name: t("gameForm.exeFilterName"), extensions: ["exe"] }],
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
    if (!name || !exePath || !prefixPath || !runnerId) {
      tab = "general";
      return;
    }
    error = null;
    submitting = true;
    const input: GameInput = {
      name,
      exe_path: exePath,
      prefix_path: prefixPath,
      runner_id: runnerId,
      env_vars: parseEnvVars(envVarsText),
      launch_args: launchArgs.trim(),
      umu_id: umuId,
      umu_store: umuStore,
      overrides: $state.snapshot({ performance: perf, graphics: gfx, overlay, proton }),
    };
    try {
      if (existingGame) {
        await updateGame(existingGame.id, input);
      } else {
        await addGame(input);
      }
      onSuccess?.();
    } catch (e) {
      error = e;
    } finally {
      submitting = false;
    }
  }
</script>

{#snippet inheritHint(page: string)}
  <p class="hint">{t("gameForm.inheritHint", { page })}</p>
{/snippet}

<form onsubmit={handleSubmit}>
  <div class="tabs" role="tablist">
    {#each tabs as tabItem (tabItem.id)}
      <button
        type="button"
        role="tab"
        aria-selected={activeTab === tabItem.id}
        class:active={activeTab === tabItem.id}
        onclick={() => (tab = tabItem.id)}
      >
        {tabItem.label}
        {#if tabItem.count > 0}
          <span class="count" title={t("gameForm.tabCount", { count: tabItem.count })}
            >{tabItem.count}</span
          >
        {/if}
      </button>
    {/each}
  </div>

  {#if activeTab === "general"}
    <label>
      {t("gameForm.nameLabel")}
      <input bind:value={name} placeholder={t("gameForm.namePlaceholder")} />
    </label>

    <label>
      {t("gameForm.exeLabel")}
      <div class="row">
        <input bind:value={exePath} readonly placeholder={t("gameForm.exePlaceholder")} />
        <button type="button" onclick={pickExe}>{t("gameForm.exeChoose")}</button>
      </div>
    </label>

    <label>
      {t("gameForm.prefixLabel")}
      <select bind:value={prefixPath}>
        <option value="" disabled selected>{t("gameForm.prefixChoose")}</option>
        {#each $prefixes as prefix (prefix.path)}
          <option value={prefix.path}>{prefix.path}</option>
        {/each}
        <!-- A prefix removed from the list is still the game's own. -->
        {#if prefixPath && !$prefixes.some((p) => p.path === prefixPath)}
          <option value={prefixPath}>{prefixPath}</option>
        {/if}
      </select>
      {#if $prefixes.length === 0}
        <span class="hint">{t("gameForm.prefixEmptyHint")}</span>
      {/if}
    </label>

    <label>
      {t("gameForm.runnerLabel")}
      <select bind:value={runnerId}>
        <option value="" disabled selected>{t("gameForm.runnerChoose")}</option>
        {#each $runners as runner (runner.id)}
          <option value={runner.id}>{runner.name} ({runner.kind})</option>
        {/each}
      </select>
      {#if isProton && !umuId}
        <span class="hint">
          {t("gameForm.runnerNoProtonfixesHint")}
          <button type="button" class="link" onclick={() => (tab = "proton")}
            >{t("nav.proton")}</button
          >
          {t("gameForm.runnerNoProtonfixesHintSuffix")}
        </span>
      {/if}
    </label>

    <label>
      {t("gameForm.envVarsLabel")}
      <textarea placeholder={t("gameForm.envVarsPlaceholder")} bind:value={envVarsText}
      ></textarea>
      <span class="hint">{t("gameForm.envVarsHint")}</span>
    </label>

    <label>
      {t("gameForm.launchArgsLabel")}
      <input placeholder={t("gameForm.launchArgsPlaceholder")} bind:value={launchArgs} />
    </label>
  {:else if activeTab === "performance"}
    {@render inheritHint(t("nav.performance"))}
    <PerformanceEditor value={perfShown} onchange={changePerf} overrides={perfHooks} />
  {:else if activeTab === "graphics"}
    {@render inheritHint(t("nav.graphics"))}
    <GraphicsEditor value={gfxShown} onchange={changeGfx} overrides={gfxHooks} compact />
  {:else if activeTab === "overlay"}
    {@render inheritHint(t("nav.overlay"))}
    <OverlayEditor value={overlayShown} onchange={changeOverlay} overrides={overlayHooks} />
  {:else if activeTab === "proton"}
    <UmuIdPicker
      {umuId}
      {umuStore}
      {name}
      steamgriddbId={existingGame?.steamgriddb_id ?? null}
      onchange={(id, store) => {
        umuId = id;
        umuStore = store;
      }}
    />
    {@render inheritHint(t("nav.proton"))}
    <ProtonEditor
      options={protonOptions}
      values={proton}
      inherited={$protonConfig?.options ?? {}}
      error={backendError(protonOptionsError)}
      onchange={(next) => (proton = next)}
    />
  {/if}

  {#if error}
    <p class="error">{backendError(error)}</p>
  {/if}

  <button type="submit" class="primary" disabled={submitting}>
    {#if submitting}
      {existingGame ? t("gameForm.submitSaving") : t("gameForm.submitAdding")}
    {:else}
      {existingGame ? t("common.save") : t("gameForm.submitAdd")}
    {/if}
  </button>
</form>

<style>
  form {
    display: flex;
    flex-direction: column;
    gap: 1em;
  }

  .tabs {
    display: flex;
    flex-wrap: wrap;
    gap: 0.3em;
    border-bottom: 1px solid var(--border);
    padding-bottom: 0.5em;
  }

  .tabs button {
    background: none;
    border: 1px solid transparent;
    color: var(--text-muted);
    padding: 0.4em 0.8em;
    border-radius: 8px;
    display: flex;
    align-items: center;
    gap: 0.4em;
  }

  .tabs button:hover {
    background: var(--surface-raised);
  }

  .tabs button.active {
    background: var(--surface-raised);
    color: var(--text);
    font-weight: 600;
  }

  .count {
    min-width: 1.4em;
    padding: 0 0.4em;
    border-radius: 999px;
    background: var(--accent);
    color: #fff;
    font-size: 0.75em;
    font-weight: 600;
    text-align: center;
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
    margin: 0;
  }

  .link {
    background: none;
    border: none;
    padding: 0;
    color: var(--accent);
    font-size: inherit;
    text-decoration: underline;
    cursor: pointer;
  }
</style>
