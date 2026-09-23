<script lang="ts">
  import type { Snippet } from "svelte";
  import InfoIcon from "$lib/components/InfoIcon.svelte";
  import { t } from "$lib/i18n/index.svelte";

  // One on/off setting with a label and description, plus optional details
  // shown below it (e.g. gamescope's resolution). `overridden`/`onReset` are
  // only passed in the game dialog — see `OverrideHooks`.
  let {
    label,
    description,
    info,
    checked,
    onToggle,
    overridden = false,
    resetTitle,
    onReset,
    children,
  }: {
    label: string;
    description: string;
    info?: string;
    checked: boolean;
    onToggle: (checked: boolean) => void;
    overridden?: boolean;
    resetTitle?: string;
    onReset?: () => void;
    children?: Snippet;
  } = $props();
</script>

<div class="setting" class:overridden>
  <div class="row">
    <div>
      <span class="label">
        {label}
        {#if info}<InfoIcon text={info} />{/if}
      </span>
      <p class="desc">{description}</p>
    </div>
    <div class="controls">
      {#if overridden && onReset}
        <button type="button" class="reset" title={resetTitle} onclick={onReset}>
          {t("common.reset")}
        </button>
      {/if}
      <label class="switch">
        <input
          type="checkbox"
          aria-label={label}
          {checked}
          onchange={(e) => onToggle(e.currentTarget.checked)}
        />
        <span class="track"><span class="thumb"></span></span>
      </label>
    </div>
  </div>
  {#if children}
    <div class="details">
      {@render children()}
    </div>
  {/if}
</div>

<style>
  .setting {
    border-radius: 10px;
    background: var(--surface-raised);
    border: 1px solid var(--border);
  }

  .setting.overridden {
    border-color: var(--accent);
  }

  .row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1em;
    padding: 0.7em 0.9em;
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

  .controls {
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
    display: flex;
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

  .switch input:focus-visible + .track {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
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
    gap: 0.7em;
    padding: 0 0.9em 0.9em;
  }

  /* Details that currently render nothing (e.g. gamescope switched off). */
  .details:empty {
    display: none;
  }
</style>
