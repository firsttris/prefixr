<script lang="ts">
  import { onMount } from "svelte";
  import GameList from "$lib/components/GameList.svelte";
  import GameForm from "$lib/components/GameForm.svelte";
  import RunnerList from "$lib/components/RunnerList.svelte";
  import PrefixManager from "$lib/components/PrefixManager.svelte";
  import MangoHudSettings from "$lib/components/MangoHudSettings.svelte";
  import PerformanceSettings from "$lib/components/PerformanceSettings.svelte";
  import SteamGridDbSettings from "$lib/components/SteamGridDbSettings.svelte";
  import ArtworkPicker from "$lib/components/ArtworkPicker.svelte";
  import Modal from "$lib/components/Modal.svelte";
  import { initGameEvents, launchGame, takePendingLaunch } from "$lib/stores/games";
  import type { Game } from "$lib/types";

  let view = $state<"library" | "prefixes" | "runners" | "mangohud" | "performance" | "steamgriddb">(
    "library",
  );
  let editing = $state<Game | "new" | null>(null);
  let pickingCoverFor = $state<Game | null>(null);

  let modalGame = $derived(editing && editing !== "new" ? editing : undefined);
  let modalTitle = $derived(editing === "new" ? "Spiel hinzufügen" : "Spiel bearbeiten");

  // If the app was started via a desktop shortcut (--launch <id>), jump
  // straight into starting that game instead of just showing the library.
  onMount(async () => {
    const pendingGameId = await takePendingLaunch();
    if (pendingGameId) {
      view = "library";
      initGameEvents();
      launchGame(pendingGameId);
    }
  });
</script>

<div class="shell">
  <nav class="sidebar">
    <div class="brand">Prefixr</div>
    <button
      type="button"
      class="nav-item"
      class:active={view === "library"}
      onclick={() => (view = "library")}
    >
      Bibliothek
    </button>
    <button
      type="button"
      class="nav-item"
      class:active={view === "prefixes"}
      onclick={() => (view = "prefixes")}
    >
      Prefixe
    </button>
    <button
      type="button"
      class="nav-item"
      class:active={view === "runners"}
      onclick={() => (view = "runners")}
    >
      Runner
    </button>
    <button
      type="button"
      class="nav-item"
      class:active={view === "mangohud"}
      onclick={() => (view = "mangohud")}
    >
      MangoHud
    </button>
    <button
      type="button"
      class="nav-item"
      class:active={view === "performance"}
      onclick={() => (view = "performance")}
    >
      Performance
    </button>
    <button
      type="button"
      class="nav-item"
      class:active={view === "steamgriddb"}
      onclick={() => (view = "steamgriddb")}
    >
      SteamGridDB
    </button>
  </nav>

  <main>
    {#if view === "library"}
      <div class="page-header">
        <h1>Bibliothek</h1>
        <button type="button" class="primary" onclick={() => (editing = "new")}>
          + Spiel hinzufügen
        </button>
      </div>
      <GameList onEdit={(game) => (editing = game)} onEditCover={(game) => (pickingCoverFor = game)} />
    {:else if view === "prefixes"}
      <div class="page-header">
        <h1>Prefixe</h1>
      </div>
      <PrefixManager />
    {:else if view === "runners"}
      <div class="page-header">
        <h1>Runner</h1>
      </div>
      <RunnerList />
    {:else if view === "mangohud"}
      <div class="page-header">
        <h1>MangoHud</h1>
      </div>
      <MangoHudSettings />
    {:else if view === "performance"}
      <div class="page-header">
        <h1>Performance</h1>
      </div>
      <PerformanceSettings />
    {:else}
      <div class="page-header">
        <h1>SteamGridDB</h1>
      </div>
      <SteamGridDbSettings />
    {/if}
  </main>
</div>

<Modal open={editing !== null} title={modalTitle} onClose={() => (editing = null)}>
  <GameForm existingGame={modalGame} onSuccess={() => (editing = null)} />
</Modal>

<Modal
  open={pickingCoverFor !== null}
  title="Cover auswählen"
  onClose={() => (pickingCoverFor = null)}
>
  {#if pickingCoverFor}
    <ArtworkPicker game={pickingCoverFor} onDone={() => (pickingCoverFor = null)} />
  {/if}
</Modal>

<style>
  .shell {
    display: flex;
    height: 100vh;
  }

  .sidebar {
    width: 220px;
    flex-shrink: 0;
    background: var(--surface);
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    padding: 1em 0.8em;
    gap: 0.3em;
    overflow-y: auto;
  }

  .brand {
    font-weight: 700;
    font-size: 1.2em;
    padding: 0.5em 0.6em 1em;
  }

  .nav-item {
    text-align: left;
    background: transparent;
    border: 1px solid transparent;
    color: var(--text-muted);
    padding: 0.6em 0.8em;
    border-radius: 8px;
  }

  .nav-item:hover {
    background: var(--surface-raised);
    border-color: transparent;
  }

  .nav-item.active {
    background: var(--surface-raised);
    color: var(--text);
    font-weight: 600;
  }

  main {
    flex: 1;
    padding: 2em;
    max-width: 1100px;
    overflow-y: auto;
  }

  .page-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1.5em;
  }

</style>
