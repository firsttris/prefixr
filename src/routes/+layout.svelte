<script lang="ts">
  import "../app.css";
  import type { Snippet } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { getLocale } from "$lib/i18n/index.svelte";

  let { children }: { children: Snippet } = $props();

  $effect(() => {
    const locale = getLocale();
    document.documentElement.lang = locale;
    // Keeps the tray menu and the few native dialogs Rust renders itself
    // (see src-tauri/src/locale.rs) in sync with the chosen language —
    // fires once on mount and again on every switch.
    invoke("set_ui_locale", { locale }).catch(() => {});
  });
</script>

{@render children()}
