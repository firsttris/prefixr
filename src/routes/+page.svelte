<script lang="ts">
  import { onMount } from "svelte";
  import GameList from "$lib/components/GameList.svelte";
  import GameForm from "$lib/components/GameForm.svelte";
  import RunnerList from "$lib/components/RunnerList.svelte";
  import PrefixManager from "$lib/components/PrefixManager.svelte";
  import PerformanceSettings from "$lib/components/PerformanceSettings.svelte";
  import GraphicsSettings from "$lib/components/GraphicsSettings.svelte";
  import OverlaySettings from "$lib/components/OverlaySettings.svelte";
  import ProtonSettings from "$lib/components/ProtonSettings.svelte";
  import SteamGridDbSettings from "$lib/components/SteamGridDbSettings.svelte";
  import ArtworkPicker from "$lib/components/ArtworkPicker.svelte";
  import InstallDialog from "$lib/components/InstallDialog.svelte";
  import Modal from "$lib/components/Modal.svelte";
  import { t, getLocale, setLocale, type Locale } from "$lib/i18n/index.svelte";
  import {
    games,
    initGameEvents,
    launchGame,
    listenForPendingInstall,
    takePendingInstall,
    takePendingLaunch,
    listenForPendingLaunch,
  } from "$lib/stores/games";
  import type { Game } from "$lib/types";

  type View =
    | "library"
    | "prefixes"
    | "runners"
    | "performance"
    | "graphics"
    | "overlay"
    | "proton"
    | "steamgriddb";
  let view = $state<View>("library");

  // The game settings categories: set globally here, overridable per game
  // in the game dialog's tabs of the same names.
  const SETTINGS_VIEWS: { view: View; label: () => string }[] = [
    { view: "performance", label: () => t("nav.performance") },
    { view: "graphics", label: () => t("nav.graphics") },
    { view: "overlay", label: () => t("nav.overlay") },
    { view: "proton", label: () => t("nav.proton") },
  ];
  let editing = $state<Game | "new" | null>(null);
  let pickingArtworkFor = $state<Game | null>(null);
  let installingExePath = $state<string | null>(null);

  let modalGame = $derived(editing && editing !== "new" ? editing : undefined);
  let modalTitle = $derived(
    editing === "new" ? t("library.addGameTitle") : t("library.editGameTitle"),
  );

  // If the app was started via a desktop shortcut (--launch <id>), jump
  // straight into starting that game instead of just showing the library.
  // If it was started via the "Mit Prefixr installieren" context menu entry
  // (--install <exe-path>) — or a second such click handed its path off to
  // this already-running instance — open the install dialog instead.
  // The listeners go in first: a handoff arriving while the startup
  // arguments are still being read would otherwise be lost.
  onMount(async () => {
    await Promise.all([
      listenForPendingLaunch((gameId) => {
        initGameEvents();
        launchGame(gameId);
      }),
      listenForPendingInstall((exePath) => (installingExePath = exePath)),
    ]);

    const pendingGameId = await takePendingLaunch();
    if (pendingGameId) {
      view = "library";
      initGameEvents();
      launchGame(pendingGameId);
    }

    const pendingExePath = await takePendingInstall();
    if (pendingExePath) {
      installingExePath = pendingExePath;
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
      {t("nav.library")}
    </button>
    <button
      type="button"
      class="nav-item"
      class:active={view === "prefixes"}
      onclick={() => (view = "prefixes")}
    >
      {t("nav.prefixes")}
    </button>
    <button
      type="button"
      class="nav-item"
      class:active={view === "runners"}
      onclick={() => (view = "runners")}
    >
      {t("nav.runners")}
    </button>
    <div class="nav-group" title={t("nav.perGameGroupTitle")}>
      {t("nav.perGameGroupLabel")}
    </div>
    {#each SETTINGS_VIEWS as item (item.view)}
      <button
        type="button"
        class="nav-item"
        class:active={view === item.view}
        onclick={() => (view = item.view)}
      >
        {item.label()}
      </button>
    {/each}
    <div class="nav-divider"></div>
    <button
      type="button"
      class="nav-item"
      class:active={view === "steamgriddb"}
      onclick={() => (view = "steamgriddb")}
    >
      {t("nav.steamgriddb")}
    </button>
    <div class="nav-spacer"></div>
    <div class="lang-switch" role="group" aria-label={t("nav.languageSwitcher")}>
      {#each ["de", "en"] as const as lng (lng)}
        <button
          type="button"
          class="lang-item"
          class:active={getLocale() === lng}
          onclick={() => setLocale(lng as Locale)}
        >
          {lng.toUpperCase()}
        </button>
      {/each}
    </div>
  </nav>

  <main>
    {#if view === "library"}
      <div class="page-header">
        <div>
          <h1>{t("library.heading")}</h1>
          {#if $games.length > 0}
            <p class="subtitle">
              {$games.length}
              {$games.length === 1 ? t("library.gameSingular") : t("library.gamePlural")}
            </p>
          {/if}
        </div>
        <button type="button" class="primary" onclick={() => (editing = "new")}>
          {t("library.addGame")}
        </button>
      </div>
      <GameList
        onEdit={(game) => (editing = game)}
        onEditArtwork={(game) => (pickingArtworkFor = game)}
        onAddNew={() => (editing = "new")}
      />
    {:else if view === "prefixes"}
      <div class="page-header">
        <h1>{t("library.prefixesHeading")}</h1>
      </div>
      <PrefixManager />
    {:else if view === "runners"}
      <div class="page-header">
        <h1>{t("library.runnersHeading")}</h1>
      </div>
      <RunnerList />
    {:else if view === "performance"}
      <PerformanceSettings />
    {:else if view === "graphics"}
      <GraphicsSettings />
    {:else if view === "overlay"}
      <OverlaySettings />
    {:else if view === "proton"}
      <ProtonSettings />
    {:else}
      <div class="page-header">
        <h1>{t("library.steamgriddbHeading")}</h1>
      </div>
      <SteamGridDbSettings />
    {/if}
  </main>
</div>

<Modal open={editing !== null} title={modalTitle} wide onClose={() => (editing = null)}>
  <GameForm existingGame={modalGame} onSuccess={() => (editing = null)} />
</Modal>

<Modal
  open={pickingArtworkFor !== null}
  title={t("library.selectArtworkTitle")}
  onClose={() => (pickingArtworkFor = null)}
>
  {#if pickingArtworkFor}
    <ArtworkPicker game={pickingArtworkFor} onDone={() => (pickingArtworkFor = null)} />
  {/if}
</Modal>

<Modal
  open={installingExePath !== null}
  title={t("library.installTitle")}
  onClose={() => (installingExePath = null)}
>
  {#if installingExePath}
    <InstallDialog exePath={installingExePath} onClose={() => (installingExePath = null)} />
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

  .nav-group {
    margin-top: 1em;
    padding: 0.4em 0.8em 0.2em;
    font-size: 0.72em;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted);
  }

  .nav-divider {
    margin: 0.6em 0.8em;
    border-top: 1px solid var(--border);
  }

  .nav-item.active {
    background: var(--surface-raised);
    color: var(--text);
    font-weight: 600;
  }

  .nav-spacer {
    flex: 1;
  }

  .lang-switch {
    display: flex;
    gap: 0.3em;
    padding: 0.4em 0.6em;
  }

  .lang-item {
    flex: 1;
    padding: 0.4em;
    font-size: 0.78em;
    font-weight: 600;
    background: transparent;
    border: 1px solid var(--border);
    color: var(--text-muted);
    border-radius: 6px;
  }

  .lang-item.active {
    background: var(--surface-raised);
    color: var(--text);
    border-color: var(--accent);
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

  .subtitle {
    color: var(--text-muted);
    font-size: 0.9em;
    margin-top: 0.2em;
  }

</style>
