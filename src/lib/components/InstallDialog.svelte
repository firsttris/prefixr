<script lang="ts">
  import { onMount } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { prefixes, refreshPrefixes, addPrefix } from "$lib/stores/prefixes";
  import { runners, refreshRunners } from "$lib/stores/runners";
  import { runInstaller } from "$lib/stores/games";
  import { prettifyExeName } from "$lib/gameName";
  import GameForm from "./GameForm.svelte";
  import type { DetectedShortcut } from "$lib/types";

  let { exePath, onClose }: { exePath: string; onClose: () => void } = $props();

  let exeName = $derived(exePath.split(/[/\\]/).pop() ?? exePath);

  let prefixPath = $state("");
  let runnerId = $state("");
  let busy = $state(false);
  let creatingPrefix = $state(false);
  let error = $state("");

  // "setup": choose prefix/runner and run the installer.
  // "review": installer finished — pick a detected shortcut, or fall back
  //           to picking the exe by hand.
  // "add": hand the chosen exe off to GameForm to actually add the game.
  let phase = $state<"setup" | "review" | "add">("setup");
  let candidates = $state<DetectedShortcut[]>([]);
  let chosenExePath = $state("");
  let chosenName = $state("");

  onMount(() => {
    refreshPrefixes();
    refreshRunners();
  });

  async function createNewPrefix() {
    const selected = await open({ directory: true });
    if (typeof selected !== "string") return;
    creatingPrefix = true;
    error = "";
    try {
      await addPrefix(selected);
      prefixPath = selected;
    } catch (e) {
      error = String(e);
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
    error = "";
    try {
      candidates = await runInstaller(prefixPath, runnerId, exePath);
      if (candidates.length === 1) {
        // Only one plausible shortcut — skip straight to confirming it
        // instead of making the user pick from a list of one.
        selectCandidate(candidates[0]);
        phase = "add";
      } else {
        phase = "review";
      }
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function pickExeManually() {
    const selected = await open({ filters: [{ name: "Programme", extensions: ["exe"] }] });
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
      <strong>{exeName}</strong> in einem Prefix ausführen, um es zu installieren. Prefixr
      versucht danach automatisch, die Spiel-Exe anhand angelegter Verknüpfungen zu finden.
    </p>

    <form onsubmit={handleStart}>
      <label>
        Prefix
        <div class="row">
          <select bind:value={prefixPath} disabled={creatingPrefix || busy}>
            <option value="" disabled selected>Prefix wählen</option>
            {#each $prefixes as prefix (prefix.path)}
              <option value={prefix.path}>{prefix.path}</option>
            {/each}
          </select>
          <button
            type="button"
            class="icon-button"
            title="Neuen Prefix erstellen"
            disabled={creatingPrefix || busy}
            onclick={createNewPrefix}
          >
            {creatingPrefix ? "…" : "+"}
          </button>
        </div>
      </label>

      <label>
        Runner
        <select bind:value={runnerId} disabled={busy}>
          <option value="" disabled selected>Runner wählen</option>
          {#each $runners as runner (runner.id)}
            <option value={runner.id}>{runner.name} ({runner.kind})</option>
          {/each}
        </select>
      </label>

      <button type="submit" class="primary" disabled={busy || !runnerId || !prefixPath}>
        {busy ? "Installation läuft…" : "Setup ausführen"}
      </button>
    </form>

    {#if busy}
      <p class="hint">Schließe das Setup-Fenster, sobald die Installation abgeschlossen ist.</p>
    {/if}
  {:else if phase === "review"}
    {#if candidates.length > 0}
      <p class="hint">Nach der Installation gefundene Verknüpfung(en) — welche startet das Spiel?</p>
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
          Weiter
        </button>
        <button type="button" onclick={pickExeManually}>Andere Datei wählen…</button>
      </div>
    {:else}
      <p class="hint">
        Es wurde keine passende Verknüpfung gefunden. Wähle die Spiel-Exe manuell aus.
      </p>
      <button type="button" class="primary" onclick={pickExeManually}>Datei wählen…</button>
    {/if}
  {:else if phase === "add"}
    <p class="hint">Spiel zur Bibliothek hinzufügen:</p>
    <GameForm
      initialName={chosenName}
      initialExePath={chosenExePath}
      initialPrefixPath={prefixPath}
      initialRunnerId={runnerId}
      onSuccess={onClose}
    />
  {/if}

  {#if error}
    <p class="error">{error}</p>
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
