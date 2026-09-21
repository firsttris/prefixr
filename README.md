# Tauri + SvelteKit + TypeScript

This template should help get you started developing with Tauri, SvelteKit and TypeScript in Vite.

## Recommended IDE Setup

[VS Code](https://code.visualstudio.com/) + [Svelte](https://marketplace.visualstudio.com/items?itemName=svelte.svelte-vscode) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer).

## Rust/Tauri Setup

Tauri benötigt Rust. Installation über rustup (Linux/macOS):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

Unter Ubuntu/Debian werden zusätzlich folgende Systemabhängigkeiten benötigt:

```bash
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

Installation prüfen:

```bash
rustc --version
cargo --version
```

Weitere Plattformen (Windows, macOS) siehe [Tauri Prerequisites](https://v2.tauri.app/start/prerequisites/).
