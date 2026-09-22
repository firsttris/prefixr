<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { runners, refreshRunners } from "$lib/stores/runners";
  import { prefixes, refreshPrefixes } from "$lib/stores/prefixes";
  import { addGame, updateGame } from "$lib/stores/games";
  import { prettifyExeName } from "$lib/gameName";
  import type { Game, GameInput } from "$lib/types";

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
  }));

  let name = $state(defaults.name);
  let exePath = $state(defaults.exePath);
  let prefixPath = $state(defaults.prefixPath);
  let runnerId = $state(defaults.runnerId);
  let envVarsText = $state(defaults.envVarsText);
  let launchArgs = $state(defaults.launchArgs);
  let error = $state("");
  let submitting = $state(false);

  onMount(() => {
    refreshRunners();
    refreshPrefixes();
  });

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
</style>
