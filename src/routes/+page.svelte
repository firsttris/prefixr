<script lang="ts">
  import { onMount } from "svelte";
  import GameList from "$lib/components/GameList.svelte";
  import GameForm from "$lib/components/GameForm.svelte";
  import RunnerList from "$lib/components/RunnerList.svelte";
  import PrefixManager from "$lib/components/PrefixManager.svelte";
  import Modal from "$lib/components/Modal.svelte";
  import { initGameEvents, launchGame, takePendingLaunch } from "$lib/stores/games";
  import type { Game } from "$lib/types";

  let view = $state<"library" | "settings">("library");
  let editing = $state<Game | "new" | null>(null);

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
      class:active={view === "settings"}
      onclick={() => (view = "settings")}
    >
      Prefixe & Runner
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
      <GameList onEdit={(game) => (editing = game)} />
    {:else}
      <div class="page-header">
        <h1>Prefixe & Runner</h1>
      </div>
      <div class="settings-grid">
        <RunnerList />
        <PrefixManager />
      </div>
    {/if}
  </main>
</div>

<Modal open={editing !== null} title={modalTitle} onClose={() => (editing = null)}>
  <GameForm existingGame={modalGame} onSuccess={() => (editing = null)} />
</Modal>

<style>
  .shell {
    display: flex;
    min-height: 100vh;
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
  }

  .page-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1.5em;
  }

  .settings-grid {
    display: grid;
    grid-template-columns: 1fr 1.4fr;
    gap: 1.5em;
    align-items: start;
  }

  @media (max-width: 720px) {
    .settings-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
