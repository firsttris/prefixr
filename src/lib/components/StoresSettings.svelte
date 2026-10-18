<!--
  Connecting store accounts. A connected store's games show up in the
  library, where they're installed like any other download.
-->
<script lang="ts">
  import * as m from "$lib/paraglide/messages";
  import { onMount } from "svelte";
  import {
    storeAccounts,
    refreshStoreAccounts,
    openStoreLogin,
    loginStoreInApp,
    connectStore,
    disconnectStore,
    syncStoreLibrary,
    storeName,
  } from "$lib/stores/gameStores";
  import { backendError } from "$lib/i18n/index.svelte";
  import ConfirmDialog from "./ConfirmDialog.svelte";
  import type { StoreAccount, StoreKind } from "$lib/types";

  let loading = $state(true);
  let loadError = $state<unknown>(null);
  // Per store: what it's busy with, its last error, the login in progress.
  let busy = $state<
    Partial<Record<StoreKind, "login" | "login-app" | "connect" | "sync" | "disconnect">>
  >({});
  let errors = $state<Partial<Record<StoreKind, unknown>>>({});
  let loggingIn = $state<Partial<Record<StoreKind, boolean>>>({});
  let codes = $state<Partial<Record<StoreKind, string>>>({});
  let confirmingDisconnect = $state<StoreAccount | null>(null);

  const logos: Record<StoreKind, string> = { epic: "EG", gog: "GOG", amazon: "AG" };
  // Each store's login ends somewhere else: Epic on a JSON page, GOG and
  // Amazon on an address that carries the code.
  const loginTexts: Record<StoreKind, { step: () => string; placeholder: () => string }> = {
    epic: { step: m.stores_loginStepCode, placeholder: m.stores_codePlaceholder },
    gog: { step: m.stores_loginStepCodeGog, placeholder: m.stores_codePlaceholderGog },
    amazon: { step: m.stores_loginStepCodeAmazon, placeholder: m.stores_codePlaceholderAmazon },
  };

  onMount(() => {
    refreshStoreAccounts()
      .catch((e) => (loadError = e))
      .finally(() => (loading = false));
  });

  async function run(
    store: StoreKind,
    action: "login" | "login-app" | "connect" | "sync" | "disconnect",
    work: () => Promise<void>,
  ) {
    busy[store] = action;
    errors[store] = undefined;
    try {
      await work();
    } catch (e) {
      errors[store] = e;
    } finally {
      busy[store] = undefined;
    }
  }

  // The window closes itself once it has the code; closing it by hand just
  // ends the login.
  function loginInApp(store: StoreKind) {
    run(store, "login-app", async () => {
      await loginStoreInApp(store);
    });
  }

  function startLogin(store: StoreKind) {
    loggingIn[store] = true;
    run(store, "login", () => openStoreLogin(store));
  }

  function submitCode(event: Event, store: StoreKind) {
    event.preventDefault();
    const code = codes[store]?.trim();
    if (!code) return;
    run(store, "connect", async () => {
      await connectStore(store, code);
      loggingIn[store] = false;
      codes[store] = "";
    });
  }
</script>

<p class="intro">{m.stores_intro()}</p>
<p class="intro hint">{m.stores_loginHint()}</p>

{#if loadError}
  <p class="error">{backendError(loadError)}</p>
{:else if loading}
  <p class="muted">{m.stores_loading()}</p>
{/if}

<div class="stores">
  {#each $storeAccounts as account (account.store)}
    {@const store = account.store}
    <section class="store" class:connected={account.connected}>
      <div class="head">
        <div class="identity">
          <div class="logo" aria-hidden="true">{logos[store]}</div>
          <div>
            <div class="title-row">
              <h2>{storeName(store)}</h2>
              {#if account.connected}
                <span class="status ok"><span class="dot"></span>{m.stores_connected()}</span>
              {:else}
                <span class="status off">{m.stores_notConnected()}</span>
              {/if}
            </div>
            <p class="desc">
              {#if account.connected}
                {m.stores_connectedAs({ name: account.account_name ?? "" })} ·
                {m.stores_gameCount({ count: account.game_count })}
              {:else}
                {m.stores_toolHint({ tool: account.tool_name })}
              {/if}
            </p>
          </div>
        </div>

        <div class="actions">
          {#if account.connected}
            <button
              type="button"
              disabled={busy[store] !== undefined}
              onclick={() => run(store, "sync", () => syncStoreLibrary(store))}
            >
              {busy[store] === "sync" ? m.stores_syncing() : m.stores_sync()}
            </button>
            <button
              type="button"
              disabled={busy[store] !== undefined}
              onclick={() => (confirmingDisconnect = account)}
            >
              {m.stores_disconnect()}
            </button>
          {:else if !loggingIn[store]}
            <button
              type="button"
              class="primary"
              disabled={busy[store] !== undefined}
              onclick={() => loginInApp(store)}
            >
              {busy[store] === "login-app"
                ? m.stores_loginWindowOpen()
                : busy[store] === "connect"
                  ? m.stores_connecting()
                  : m.stores_connect()}
            </button>
            <button
              type="button"
              disabled={busy[store] !== undefined}
              onclick={() => startLogin(store)}
            >
              {m.stores_connectInBrowser()}
            </button>
          {/if}
        </div>
      </div>

      {#if !account.connected && loggingIn[store]}
        <form class="login" onsubmit={(e) => submitCode(e, store)}>
          <ol>
            <li>
              {m.stores_loginStepOpen()}
              <button
                type="button"
                class="link"
                disabled={busy[store] === "login"}
                onclick={() => run(store, "login", () => openStoreLogin(store))}
              >
                {busy[store] === "login" ? m.stores_loginPreparing() : m.stores_loginOpenAgain()}
              </button>
            </li>
            <li>{loginTexts[store].step()}</li>
          </ol>
          <div class="code-row">
            <input
              type="text"
              autocomplete="off"
              spellcheck="false"
              placeholder={loginTexts[store].placeholder()}
              aria-label={m.stores_codeLabel()}
              bind:value={codes[store]}
            />
            <button
              type="submit"
              class="primary"
              disabled={!codes[store]?.trim() || busy[store] !== undefined}
            >
              {busy[store] === "connect" ? m.stores_connecting() : m.stores_signIn()}
            </button>
            <button type="button" class="ghost" onclick={() => (loggingIn[store] = false)}>
              {m.common_cancel()}
            </button>
          </div>
        </form>
      {/if}

      {#if errors[store]}
        <p class="error">{backendError(errors[store])}</p>
      {/if}

      <p class="tool">
        {account.tool_needs_python
          ? m.stores_toolLine({ tool: account.tool_name, version: account.tool_version })
          : m.stores_toolLineStandalone({ tool: account.tool_name, version: account.tool_version })}
      </p>
    </section>
  {/each}
</div>

<ConfirmDialog
  open={confirmingDisconnect !== null}
  title={m.stores_disconnectConfirmTitle()}
  message={confirmingDisconnect
    ? m.stores_disconnectConfirmMessage({ store: storeName(confirmingDisconnect.store) })
    : ""}
  confirmLabel={m.stores_disconnect()}
  onConfirm={() => {
    const account = confirmingDisconnect;
    confirmingDisconnect = null;
    if (account) run(account.store, "disconnect", () => disconnectStore(account.store));
  }}
  onCancel={() => (confirmingDisconnect = null)}
/>

<style>
  .intro {
    color: var(--text-muted);
    font-size: 0.9em;
    max-width: 65ch;
    margin-bottom: 1.4em;
  }

  .intro:has(+ .hint) {
    margin-bottom: 0.6em;
  }

  .hint {
    font-size: 0.82em;
  }

  .stores {
    display: flex;
    flex-direction: column;
    gap: 0.8em;
    max-width: var(--page-max);
  }

  .store {
    display: flex;
    flex-direction: column;
    gap: 0.9em;
    padding: 1em 1.1em;
    border-radius: var(--radius);
    background: var(--surface-raised);
    border: 1px solid var(--border);
  }

  .store.connected {
    border-color: rgba(62, 207, 142, 0.45);
  }

  .head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1em;
    flex-wrap: wrap;
  }

  /* Takes the room the buttons leave and wraps its text in it, so the
     buttons stay on the right however long the description is. */
  .identity {
    flex: 1 1 20em;
    display: flex;
    gap: 0.9em;
    align-items: center;
    min-width: 0;
  }

  .logo {
    width: 2.8em;
    height: 2.8em;
    border-radius: var(--radius);
    background: var(--surface);
    border: 1px solid var(--border);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 0.85em;
    font-weight: 700;
    flex-shrink: 0;
  }

  .title-row {
    display: flex;
    gap: 0.6em;
    align-items: center;
    flex-wrap: wrap;
  }

  h2 {
    font-size: 1em;
  }

  .status {
    display: inline-flex;
    align-items: center;
    gap: 0.4em;
    font-size: 0.75em;
    font-weight: 600;
    border-radius: 999px;
    padding: 0.15em 0.65em;
  }

  .status.ok {
    color: var(--success);
    background: var(--success-bg);
  }

  .status.off {
    color: var(--text-muted);
    background: var(--surface);
    border: 1px solid var(--border);
  }

  .dot {
    width: 0.45em;
    height: 0.45em;
    border-radius: 50%;
    background: currentColor;
  }

  .desc {
    margin-top: 0.2em;
    font-size: 0.85em;
    color: var(--text-muted);
  }

  .actions {
    display: flex;
    gap: 0.5em;
  }

  .login {
    display: flex;
    flex-direction: column;
    gap: 0.7em;
    font-size: 0.9em;
  }

  ol {
    margin: 0;
    padding-left: 1.3em;
    color: var(--text-muted);
    display: flex;
    flex-direction: column;
    gap: 0.35em;
  }

  .link {
    background: none;
    border: none;
    padding: 0 0.2em;
    color: var(--accent);
    text-decoration: underline;
  }

  .code-row {
    display: flex;
    gap: 0.5em;
  }

  .code-row input {
    flex: 1;
    font-family: ui-monospace, monospace;
  }

  .code-row button {
    flex-shrink: 0;
  }

  .tool,
  .muted {
    font-size: 0.8em;
    color: var(--text-muted);
  }

  .error {
    color: var(--danger);
    font-size: 0.9em;
  }
</style>
