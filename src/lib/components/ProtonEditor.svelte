<script lang="ts">
  import { PROTON_OPTION_ORDER } from "$lib/protonOptions";
  import type { ProtonOption } from "$lib/types";
  import { t, type TranslationKey } from "$lib/i18n/index.svelte";

  // Proton — see settings.ts for how the editors are shared between the
  // global page and the game dialog. Unlike the other categories every
  // switch has three states: the neutral one leaves the decision to the
  // level below (Proton's own default globally, the global setting in the
  // game dialog), "An"/"Aus" set the variable to 1/0.
  let {
    options,
    values,
    inherited,
    onchange,
    error = "",
  }: {
    // What the runner's `proton` script understands.
    options: ProtonOption[];
    // Switches set at this level, by variable name.
    values: Record<string, boolean>;
    // Game dialog only: the global switches, which the neutral state falls
    // back to.
    inherited?: Record<string, boolean>;
    onchange: (values: Record<string, boolean>) => void;
    error?: string;
  } = $props();

  const curated = $derived(
    options
      .filter((o) => PROTON_OPTION_ORDER.includes(o.config))
      .sort(
        (a, b) => PROTON_OPTION_ORDER.indexOf(a.config) - PROTON_OPTION_ORDER.indexOf(b.config),
      ),
  );
  const others = $derived(options.filter((o) => !PROTON_OPTION_ORDER.includes(o.config)));
  // Set here, but unknown to the runner — e.g. carried over from an older
  // Proton version. Harmless, but shown so they can be cleared.
  const unknown = $derived(
    Object.keys(values).filter(
      (name) => !options.some((o) => o.env === name || o.aliases.includes(name)),
    ),
  );

  // Looks under every name the runner accepts for this switch, so a value
  // saved under an alias (or under a name an older runner preferred) shows up.
  function lookup(map: Record<string, boolean> | undefined, option: ProtonOption) {
    if (!map) return null;
    for (const name of [option.env, ...option.aliases]) {
      if (name in map) return map[name];
    }
    return null;
  }

  function set(option: ProtonOption, value: boolean | null) {
    const next = { ...values };
    for (const alias of option.aliases) delete next[alias];
    if (value === null) delete next[option.env];
    else next[option.env] = value;
    onchange(next);
  }

  function remove(name: string) {
    const next = { ...values };
    delete next[name];
    onchange(next);
  }

  function neutralLabel(option: ProtonOption): string {
    if (!inherited) return t("protonEditor.neutralDefault");
    const global = lookup(inherited, option);
    return global === null
      ? t("protonEditor.neutralGlobal")
      : t("protonEditor.neutralGlobalState", { state: global ? t("common.on") : t("common.off") });
  }
</script>

{#snippet tristate(option: ProtonOption)}
  {@const value = lookup(values, option)}
  <div class="tristate" role="group" aria-label={option.env}>
    <button type="button" aria-pressed={value === null} onclick={() => set(option, null)}>
      {neutralLabel(option)}
    </button>
    <button type="button" aria-pressed={value === true} onclick={() => set(option, true)}>
      {t("protonEditor.on")}
    </button>
    <button type="button" aria-pressed={value === false} onclick={() => set(option, false)}>
      {t("protonEditor.off")}
    </button>
  </div>
{/snippet}

<div class="proton-editor">
  {#if error}
    <p class="error">{error}</p>
  {/if}

  <div class="list">
    {#each curated as option (option.config)}
      <div class="row" class:overridden={lookup(values, option) !== null}>
        <div>
          <span class="label">{t(`protonOptions.${option.config}.label` as TranslationKey)}</span>
          <p class="desc">{t(`protonOptions.${option.config}.description` as TranslationKey)}</p>
          <code class="env-name">{option.env}</code>
        </div>
        {@render tristate(option)}
      </div>
    {/each}
  </div>

  {#if others.length > 0}
    <details class="advanced">
      <summary>{t("protonEditor.advanced", { count: others.length })}</summary>
      <p class="hint">{t("protonEditor.advancedHint")}</p>
      <div class="compact-list">
        {#each others as option (option.config)}
          <div class="compact-row" class:overridden={lookup(values, option) !== null}>
            <code
              title={option.aliases.length
                ? t("protonEditor.alsoAliases", { aliases: option.aliases.join(", ") })
                : ""}
            >
              {option.env}
            </code>
            {@render tristate(option)}
          </div>
        {/each}
      </div>
    </details>
  {/if}

  {#if options.length > 0 && unknown.length > 0}
    <div class="compact-list">
      <p class="hint">{t("protonEditor.unsetKnownHint")}</p>
      {#each unknown as name (name)}
        <div class="compact-row overridden">
          <code>{name}={values[name] ? "1" : "0"}</code>
          <button type="button" class="reset" onclick={() => remove(name)}
            >{t("common.remove")}</button
          >
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .proton-editor {
    display: flex;
    flex-direction: column;
    gap: 1em;
  }

  .list {
    display: flex;
    flex-direction: column;
    gap: 0.7em;
  }

  .row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1em;
    padding: 0.7em 0.9em;
    border-radius: 10px;
    background: var(--surface-raised);
    border: 1px solid var(--border);
  }

  .row.overridden,
  .compact-row.overridden {
    border-color: var(--accent);
  }

  .label {
    font-weight: 600;
    color: var(--text);
    display: block;
  }

  .desc {
    color: var(--text-muted);
    font-size: 0.82em;
    margin: 0.25em 0 0;
    max-width: 52ch;
  }

  .env-name {
    display: inline-block;
    margin-top: 0.3em;
    color: var(--text-muted);
    font-size: 0.75em;
  }

  .tristate {
    display: flex;
    flex-shrink: 0;
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
    white-space: nowrap;
  }

  .tristate button + button {
    border-left: 1px solid var(--border);
  }

  .tristate button[aria-pressed="true"] {
    background: var(--accent);
    color: #fff;
  }

  .advanced summary {
    cursor: pointer;
    color: var(--text-muted);
    font-weight: 600;
    font-size: 0.9em;
  }

  .advanced[open] {
    display: flex;
    flex-direction: column;
    gap: 0.6em;
  }

  .hint {
    color: var(--text-muted);
    font-size: 0.85em;
    margin: 0;
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

  .reset {
    padding: 0.25em 0.6em;
    font-size: 0.8em;
  }

  .error {
    color: var(--danger);
  }
</style>
