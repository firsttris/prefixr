<script lang="ts">
  import { onMount } from "svelte";
  import {
    umuStatus,
    latestUmuVersion,
    refreshUmuStatus,
    installUmu,
    checkUmuUpdate,
  } from "$lib/stores/umu";
  import { backendError, t } from "$lib/i18n/index.svelte";

  let installing = $state(false);
  let error = $state<unknown>(null);
  let updated = $state(false);

  // Only when both are known: a missing version file (older installs) or
  // an unreachable GitHub is no reason to claim an update.
  let updateAvailable = $derived(
    !!$umuStatus?.installed &&
      !!$umuStatus.version &&
      !!$latestUmuVersion &&
      $latestUmuVersion !== $umuStatus.version,
  );

  onMount(() => {
    refreshUmuStatus();
    checkUmuUpdate();
  });

  async function handleInstall() {
    installing = true;
    error = null;
    updated = false;
    try {
      await installUmu();
      updated = true;
    } catch (e) {
      error = e;
    } finally {
      installing = false;
    }
  }
</script>

<details class="umu">
  <summary>
    umu-launcher
    {#if $umuStatus?.installed}
      <span class="status">{$umuStatus.version ?? t("umuSettings.installed")}</span>
      {#if updateAvailable}
        <span class="update"
          >{t("umuSettings.updateAvailable", { version: $latestUmuVersion ?? "" })}</span
        >
      {/if}
    {:else if $umuStatus}
      <span class="status">{t("umuSettings.notInstalled")}</span>
    {/if}
  </summary>
  <p class="hint">
    {t("umuSettings.hintBefore")} <code>PROTON_*</code>{t("umuSettings.hintAfter")}
  </p>

  <div class="row">
    <button type="button" class="ghost" disabled={installing} onclick={handleInstall}>
      {#if installing}
        {t("umuSettings.loading")}
      {:else if updateAvailable}
        {t("umuSettings.updateTo", { version: $latestUmuVersion ?? "" })}
      {:else if $umuStatus?.installed && $latestUmuVersion === $umuStatus.version}
        {t("umuSettings.reinstall")}
      {:else if $umuStatus?.installed}
        {t("umuSettings.updateToLatest")}
      {:else}
        {t("umuSettings.installNow")}
      {/if}
    </button>
  </div>

  {#if error}
    <p class="error">{backendError(error)}</p>
  {:else if updated}
    <p class="saved-hint">
      {t("umuSettings.installedHint", { version: $umuStatus?.version ?? "" })}
    </p>
  {/if}
</details>

<style>
  .umu {
    background: var(--surface-raised);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 0.5em 0.8em;
    margin-bottom: 0.8em;
    font-size: 0.9em;
  }

  summary {
    cursor: pointer;
    color: var(--text-muted);
    font-weight: 600;
    font-size: 0.85em;
  }

  .status {
    font-weight: 400;
    margin-left: 0.4em;
  }

  .update {
    font-weight: 400;
    margin-left: 0.4em;
    color: var(--accent);
  }

  .hint {
    color: var(--text-muted);
    font-size: 0.85em;
    max-width: 60ch;
    margin: 0.6em 0;
  }

  .row {
    display: flex;
    gap: 0.5em;
  }

  .error {
    color: var(--danger);
    font-size: 0.85em;
    margin: 0.5em 0 0;
  }

  .saved-hint {
    color: var(--text-muted);
    font-size: 0.85em;
    margin: 0.5em 0 0;
  }
</style>
