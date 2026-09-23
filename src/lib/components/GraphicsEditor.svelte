<script lang="ts">
  import InfoIcon from "$lib/components/InfoIcon.svelte";
  import SettingToggle from "$lib/components/SettingToggle.svelte";
  import type { OverrideHooks } from "$lib/settings";
  import type { GamescopeSettings, GraphicsConfig, VkBasaltSettings } from "$lib/types";
  import { t } from "$lib/i18n/index.svelte";

  // Bild — see settings.ts for how the editors are shared between the global
  // page and the game dialog. gamescope and vkBasalt are each overridden as
  // a whole block (see `GraphicsOverrides` in models.rs).
  let {
    value,
    onchange,
    overrides,
    compact = false,
  }: {
    value: GraphicsConfig;
    onchange: (patch: {
      gamescope?: Partial<GamescopeSettings>;
      vkbasalt?: Partial<VkBasaltSettings>;
    }) => void;
    overrides?: OverrideHooks<keyof GraphicsConfig>;
    // Leaves out the vkBasalt preview, for the narrower game dialog.
    compact?: boolean;
  } = $props();

  // Emptied number inputs read as NaN; 0 is no valid size or limit either.
  function numberOrNull(input: HTMLInputElement): number | null {
    const n = input.valueAsNumber;
    return Number.isFinite(n) && n > 0 ? n : null;
  }

  const previewFilter = $derived.by(() => {
    const vk = value.vkbasalt;
    let filter = "";
    if (vk.sharpen) {
      filter += ` contrast(${1 + vk.sharpness * 0.35}) saturate(${1 + vk.sharpness * 0.2})`;
    }
    if (vk.smaa) filter += " blur(0.4px)";
    return filter.trim() || "none";
  });
</script>

<div class="list">
  <SettingToggle
    label={t("graphicsEditor.gamescopeLabel")}
    description={t("graphicsEditor.gamescopeDescription")}
    info={t("graphicsEditor.gamescopeInfo")}
    checked={value.gamescope.enabled}
    onToggle={(enabled) => onchange({ gamescope: { enabled } })}
    overridden={overrides?.isOverridden("gamescope")}
    resetTitle={overrides?.resetTitle("gamescope")}
    onReset={overrides && (() => overrides.reset("gamescope"))}
  >
    {#snippet children()}
      {#if value.gamescope.enabled}
        <div class="field-grid">
          <label>
            {t("graphicsEditor.widthLabel")}
            <input
              type="number"
              min="0"
              placeholder={t("graphicsEditor.widthPlaceholder")}
              value={value.gamescope.width ?? ""}
              oninput={(e) => onchange({ gamescope: { width: numberOrNull(e.currentTarget) } })}
            />
          </label>
          <label>
            {t("graphicsEditor.heightLabel")}
            <input
              type="number"
              min="0"
              placeholder={t("graphicsEditor.heightPlaceholder")}
              value={value.gamescope.height ?? ""}
              oninput={(e) => onchange({ gamescope: { height: numberOrNull(e.currentTarget) } })}
            />
          </label>
          <label>
            {t("graphicsEditor.fpsLimitLabel")}
            <input
              type="number"
              min="0"
              placeholder={t("graphicsEditor.fpsLimitPlaceholder")}
              value={value.gamescope.fps_limit ?? ""}
              oninput={(e) =>
                onchange({ gamescope: { fps_limit: numberOrNull(e.currentTarget) } })}
            />
          </label>
        </div>
        <label class="check">
          <input
            type="checkbox"
            checked={value.gamescope.fullscreen}
            onchange={(e) => onchange({ gamescope: { fullscreen: e.currentTarget.checked } })}
          />
          {t("graphicsEditor.forceFullscreen")}
        </label>
      {/if}
    {/snippet}
  </SettingToggle>

  <SettingToggle
    label={t("graphicsEditor.vkbasaltLabel")}
    description={t("graphicsEditor.vkbasaltDescription")}
    info={t("graphicsEditor.vkbasaltInfo")}
    checked={value.vkbasalt.enabled}
    onToggle={(enabled) => onchange({ vkbasalt: { enabled } })}
    overridden={overrides?.isOverridden("vkbasalt")}
    resetTitle={overrides?.resetTitle("vkbasalt")}
    onReset={overrides && (() => overrides.reset("vkbasalt"))}
  >
    {#snippet children()}
      {#if value.vkbasalt.enabled}
        <div class="vkbasalt" class:compact>
          {#if !compact}
            <div class="preview">
              <span class="title">{t("graphicsEditor.previewTitle")}</span>
              <div class="preview-screen">
                <div class="preview-image" style={`filter:${previewFilter}`}></div>
              </div>
            </div>
          {/if}
          <div class="effects">
            <div class="check-row">
              <label class="check">
                <input
                  type="checkbox"
                  checked={value.vkbasalt.sharpen}
                  onchange={(e) => onchange({ vkbasalt: { sharpen: e.currentTarget.checked } })}
                />
                {t("graphicsEditor.sharpenLabel")}
              </label>
              <InfoIcon text={t("graphicsEditor.sharpenInfo")} />
            </div>
            {#if value.vkbasalt.sharpen}
              <label class="sharpness">
                {t("graphicsEditor.sharpnessLabel", { value: value.vkbasalt.sharpness.toFixed(2) })}
                <input
                  type="range"
                  min="0"
                  max="1"
                  step="0.05"
                  value={value.vkbasalt.sharpness}
                  oninput={(e) =>
                    onchange({ vkbasalt: { sharpness: e.currentTarget.valueAsNumber } })}
                />
              </label>
            {/if}
            <div class="check-row">
              <label class="check">
                <input
                  type="checkbox"
                  checked={value.vkbasalt.smaa}
                  onchange={(e) => onchange({ vkbasalt: { smaa: e.currentTarget.checked } })}
                />
                {t("graphicsEditor.smaaLabel")}
              </label>
              <InfoIcon text={t("graphicsEditor.smaaInfo")} />
            </div>
          </div>
        </div>
      {/if}
    {/snippet}
  </SettingToggle>
</div>

<style>
  .list {
    display: flex;
    flex-direction: column;
    gap: 0.7em;
  }

  .field-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(8em, 1fr));
    gap: 0.7em;
  }

  .field-grid input {
    width: 100%;
  }

  .check {
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 0.5em;
    font-size: 0.9em;
    color: var(--text);
  }

  .check input {
    width: auto;
    padding: 0;
  }

  .check-row {
    display: flex;
    align-items: center;
    gap: 0.5em;
  }

  .vkbasalt {
    display: grid;
    grid-template-columns: 1.1fr 1fr;
    gap: 1.4em;
    align-items: start;
  }

  .vkbasalt.compact {
    grid-template-columns: 1fr;
  }

  @media (max-width: 640px) {
    .vkbasalt {
      grid-template-columns: 1fr;
    }
  }

  .title {
    display: block;
    font-size: 0.85em;
    color: var(--text-muted);
    margin-bottom: 0.5em;
  }

  .preview {
    min-width: 0;
  }

  .preview-screen {
    width: 100%;
    height: 200px;
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
    gap: 0.7em;
    min-width: 0;
  }

  .sharpness {
    padding-left: 1.6em;
  }
</style>
