<script lang="ts">
  import * as m from "$lib/paraglide/messages";
  import { onMount } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { prefixes, refreshPrefixes, addPrefix } from "$lib/stores/prefixes";
  import { runners, refreshRunners } from "$lib/stores/runners";
  import { runInstaller } from "$lib/stores/games";
  import { prettifyExeName } from "$lib/gameName";
  import { showLog } from "$lib/logViewer";
  import GameForm from "./GameForm.svelte";
  import type { DetectedShortcut } from "$lib/types";
  import { backendError } from "$lib/i18n/index.svelte";

  let { exePath, onClose }: { exePath: string; onClose: () => void } = $props();

  let exeName = $derived(exePath.split(/[/\\]/).pop() ?? exePath);

  let prefixPath = $state("");
  let runnerId = $state("");
  let busy = $state(false);
  let creatingPrefix = $state(false);
  let error = $state<unknown>(null);

  // "setup": choose prefix/runner and run the installer.
  // "review": installer finished — pick a detected shortcut, or fall back
  //           to picking the exe by hand.
  // "add": hand the chosen exe off to GameForm to actually add the game.
  let phase = $state<"setup" | "review" | "add">("setup");
  let candidates = $state<DetectedShortcut[]>([]);
  let chosenExePath = $state("");
  let chosenName = $state("");
  let logPath = $state("");

  onMount(() => {
    refreshPrefixes();
    refreshRunners();
  });

  async function createNewPrefix() {
    const selected = await open({ directory: true });
    if (typeof selected !== "string") return;
    creatingPrefix = true;
    error = null;
    try {
      prefixPath = await addPrefix(selected);
    } catch (e) {
      error = e;
    } finally {
      creatingPrefix = false;
    }
  }

  function selectCandidate(candidate: DetectedShortcut) {
    chosenExePath = candidate.exe_path;
    chosenName = prettifyExeName(candidate.name);
  }

  async function handleStart(event: Event) {
    event.preventDefault();
    if (!runnerId || !prefixPath) return;
    busy = true;
    error = null;
    try {
      const result = await runInstaller(prefixPath, runnerId, exePath);
      candidates = result.shortcuts;
      logPath = result.log_path;
      if (candidates.length === 1) {
        // Only one plausible shortcut — skip straight to confirming it
        // instead of making the user pick from a list of one.
        selectCandidate(candidates[0]);
        phase = "add";
      } else {
        phase = "review";
      }
    } catch (e) {
      error = e;
    } finally {
      busy = false;
    }
  }

  async function pickExeManually() {
    const selected = await open({
      filters: [{ name: m.gameForm_exeFilterName(), extensions: ["exe"] }],
    });
    if (typeof selected === "string") {
      chosenExePath = selected;
      chosenName = prettifyExeName(selected.split(/[/\\]/).pop() ?? "");
      phase = "add";
    }
  }
</script>

<div class="install-dialog">
  {#if phase === "setup"}
    <p class="hint">
      <strong>{exeName}</strong>
      {m.installDialog_introRest()}
    </p>

    <form onsubmit={handleStart}>
      <label>
        {m.gameForm_prefixLabel()}
        <div class="row">
          <select bind:value={prefixPath} disabled={creatingPrefix || busy}>
            <option value="" disabled selected>{m.gameForm_prefixChoose()}</option>
            {#each $prefixes as prefix (prefix.path)}
              <option value={prefix.path}>{prefix.path}</option>
            {/each}
          </select>
          <button
            type="button"
            class="icon-button"
            title={m.installDialog_newPrefixTitle()}
            disabled={creatingPrefix || busy}
            onclick={createNewPrefix}
          >
            {creatingPrefix ? "…" : "+"}
          </button>
        </div>
      </label>

      <label>
        {m.gameForm_runnerLabel()}
        <select bind:value={runnerId} disabled={busy}>
          <option value="" disabled selected>{m.gameForm_runnerChoose()}</option>
          {#each $runners as runner (runner.id)}
            <option value={runner.id}>{runner.name} ({runner.kind})</option>
          {/each}
        </select>
      </label>

      <button type="submit" class="primary" disabled={busy || !runnerId || !prefixPath}>
        {busy ? m.installDialog_runBusy() : m.installDialog_runStart()}
      </button>
    </form>

    {#if busy}
      <p class="hint">{m.installDialog_closeHint()}</p>
    {/if}
  {:else if phase === "review"}
    {#if candidates.length > 0}
      <p class="hint">{m.installDialog_candidatesHint()}</p>
      <div class="candidates">
        {#each candidates as candidate (candidate.exe_path)}
          <label class="candidate">
            <input
              type="radio"
              name="candidate"
              checked={chosenExePath === candidate.exe_path}
              onchange={() => selectCandidate(candidate)}
            />
            <span class="candidate-name">{candidate.name}</span>
            <span class="candidate-path">{candidate.exe_path}</span>
          </label>
        {/each}
      </div>
      <div class="row">
        <button
          type="button"
          class="primary"
          disabled={!chosenExePath}
          onclick={() => (phase = "add")}
        >
          {m.installDialog_continueButton()}
        </button>
        <button type="button" onclick={pickExeManually}>{m.installDialog_pickOtherFile()}</button
        >
      </div>
    {:else}
      <p class="hint">{m.installDialog_noCandidatesHint()}</p>
      <div class="row">
        <button type="button" class="primary" onclick={pickExeManually}
          >{m.installDialog_pickFile()}</button
        >
        {#if logPath}
          <button type="button" onclick={() => showLog(logPath)}
            >{m.installDialog_showSetupLog()}</button
          >
        {/if}
      </div>
    {/if}
  {:else if phase === "add"}
    <p class="hint">{m.installDialog_addToLibraryHint()}</p>
    <GameForm
      initialName={chosenName}
      initialExePath={chosenExePath}
      initialPrefixPath={prefixPath}
      initialRunnerId={runnerId}
      onSuccess={onClose}
    />
  {/if}

  {#if error}
    <p class="error">{backendError(error)}</p>
  {/if}
</div>

<style>
  .install-dialog {
    display: flex;
    flex-direction: column;
    gap: 1em;
  }

  .hint {
    color: var(--text-muted);
    font-size: 0.9em;
  }

  .hint strong {
    color: var(--text);
    word-break: break-all;
  }

  form {
    display: flex;
    flex-direction: column;
    gap: 1em;
  }

  label {
    display: flex;
    flex-direction: column;
    gap: 0.3em;
    font-size: 0.9em;
  }

  .row {
    display: flex;
    gap: 0.5em;
  }

  .row select {
    flex: 1;
  }

  .icon-button {
    flex-shrink: 0;
    width: 2.2em;
    padding: 0;
    font-size: 1.1em;
    line-height: 1;
  }

  .candidates {
    display: flex;
    flex-direction: column;
    gap: 0.5em;
  }

  .candidate {
    display: flex;
    align-items: baseline;
    gap: 0.5em;
    font-size: 0.9em;
  }

  .candidate-name {
    color: var(--text);
    font-weight: 600;
  }

  .candidate-path {
    color: var(--text-muted);
    font-size: 0.85em;
    word-break: break-all;
  }

  .error {
    color: var(--danger);
  }
</style>
