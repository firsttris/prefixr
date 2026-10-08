# Development

How to build Prefixr, how the code is organized, and what CI checks before a change is merged.

- [Building from source](#building-from-source)
- [Everyday commands](#everyday-commands)
- [Project layout](#project-layout)
- [Architecture](#architecture)
- [Texts and languages](#texts-and-languages)
- [Generated TypeScript types](#generated-typescript-types)
- [Tests and CI](#tests-and-ci)
- [Releases](#releases)

## Building from source

Prefixr is a [Tauri 2](https://v2.tauri.app/) app: a Rust backend with a
[SvelteKit](https://svelte.dev/docs/kit) frontend (Svelte 5, TypeScript) rendered by WebKitGTK.

**You need:**

- Rust 1.89 or newer, via [rustup](https://rustup.rs/):

  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```

- [Bun](https://bun.sh/) as the package manager and script runner
- Tauri's system libraries. On Debian/Ubuntu:

  ```bash
  sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
    libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev patchelf
  ```

  For Fedora, Arch and others, see [Tauri's prerequisites](https://v2.tauri.app/start/prerequisites/#linux).

**Then:**

```bash
bun install
bun run tauri dev     # development build with hot reload
bun run tauri build   # release binary and bundles
```

`bun run tauri build` writes the binary to `src-tauri/target/release/prefixr` and the AppImage,
`.deb` and `.rpm` to `src-tauri/target/release/bundle/`. Use it rather than
`cargo build --release`, which doesn't embed the frontend.

**Recommended editor setup:** [VS Code](https://code.visualstudio.com/) with the
[Svelte](https://marketplace.visualstudio.com/items?itemName=svelte.svelte-vscode),
[Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) and
[rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer) extensions.

## Everyday commands

| Command | Does |
| --- | --- |
| `bun run tauri dev` | Runs the app with the Vite dev server and hot reload |
| `bun run check` | Compiles the texts, then type-checks the frontend with svelte-check |
| `bun run lint` | Prettier format check and ESLint |
| `bun run format` | Formats the frontend with Prettier |
| `bun run test` | Frontend tests (Vitest) |
| `cargo test` | Backend tests; also regenerates `src/lib/bindings` |
| `cargo clippy --all-targets -- -D warnings` | Backend lints, as CI runs them |
| `cargo fmt` | Formats the backend |

Run the `cargo` commands in `src-tauri/`, or from the repository root with
`--manifest-path src-tauri/Cargo.toml`.

## Project layout

```text
messages/               UI texts, de.json and en.json (Paraglide JS)
src/
  routes/               The single page: sidebar, views and dialogs
  lib/
    components/         Svelte components (GameCard, GameForm, Modal, Menu, …)
    stores/             One module per backend area: Svelte stores plus the invoke() calls
    bindings/           TypeScript types generated from Rust (don't edit)
    i18n/               Locale switching and backendError()
    types.ts            Re-exports the generated types
tests/                  Vitest tests
src-tauri/
  src/
    lib.rs              App setup, single-instance handoff, command registration
    commands/           One module per area, each with its tests in commands/<name>/tests.rs
    config.rs           Loading, migrating and atomically saving config.json
    models.rs           The persisted data model (games, settings)
    error.rs            AppError, the structured error every command returns
    tray.rs, locale.rs  Tray menu, and the texts Rust renders itself
  tauri.conf.json       Window, CSP, asset protocol scope, bundling
.cargo/config.toml      Where ts-rs writes the TypeScript bindings
```

### Backend modules

| Module | Responsibility |
| --- | --- |
| `games` | The library, launching and stopping games, installers, shortcuts, the `--run` headless mode |
| `prefixes` | The prefix list, and finding the real prefix inside a Steam compatdata folder |
| `runners` | Detecting runners and building the commands that run them |
| `runner_downloads` | Listing, downloading, verifying and unpacking runner releases |
| `umu`, `umu_database` | The managed umu-launcher copy, and the protonfixes (UMU ID) search |
| `graphics_layers` | DXVK, VKD3D-Proton and wine-mono for Wine runners |
| `winetricks`, `wine_tools` | Installing dependencies, and Wine's own tools |
| `performance`, `graphics`, `mangohud`, `proton_options` | The settings categories, and the config files and environment they turn into |
| `steamgriddb` | Artwork search, download and cache |
| `steam`, `binary_vdf` | Adding games to Steam through its binary `shortcuts.vdf` |
| `icons`, `shell_link`, `logs` | Exe icons, reading `.lnk` shortcuts, and log rotation |
| `github`, `locale` | The GitHub token, and the UI language as Rust needs it |

## Architecture

**Commands and state.** The frontend calls Rust through Tauri commands (`invoke()` in
`src/lib/stores/*`). The config lives in one `Mutex<AppConfig>`. Commands take it through
`LockExt::locked()`, which also accepts a poisoned lock, and never hold it across an `.await`.
Every change is saved to `config.json` with a write to a temporary file and a rename.

**Errors.** Every command returns `AppError`, serialized as `{ code, ...fields }`. The frontend turns
it into text with `backendError()`, so an error stays in the chosen language even after switching.
`AppError::Other` carries text that stays untranslated, for technical failures.

**Events.** Long-running work reports through events: `game-initializing`, `game-started`,
`game-exited` and `game-launch-error` for launches, `runner-download-progress` and
`runner-download-done` for downloads, and `pending-launch` and `pending-install` when a second
instance hands its arguments over (see `tauri_plugin_single_instance` in `lib.rs`). The handed-over
value waits in the backend until the frontend takes it, so it isn't lost before the page listens.

**Launching.** `prepare_prefix` readies a prefix the same way for a game, an installer, winetricks or
a Wine tool, so a fresh prefix ends up identical whichever comes first. `run_game` then builds the
wrapper chain (see [What a launch sets up](configuration.md#what-a-launch-sets-up)), spawns it in
its own process group, and tracks it in `RunningGames`. A game in progress is claimed in
`LaunchingGames`, which also holds a lock file in `$XDG_RUNTIME_DIR`, so a Prefixr that Steam started
and the open window never start the same game twice. Stopping a game ends its process tree, the umu
session and the wineserver.

**Downloads.** Runners are streamed to disk, checked against the release's checksum file, extracted
on the blocking thread pool into a scratch folder and moved into place. umu and the DirectX layers
are swapped in with `replace_dir` (an atomic `renameat2` exchange), so a running launch never finds
them missing.

**Security.** The webview runs under a Content Security Policy (`tauri.conf.json`) that allows only
the app's own scripts, IPC, and images from the app, the asset protocol and SteamGridDB. Covers and
icons load through the asset protocol, scoped to `artwork/` and `exe-icons/`. Commands that run
something in a prefix only accept prefixes Prefixr knows, and artwork downloads only come from
SteamGridDB.

**Frontend.** A single-page SvelteKit app (`adapter-static`) with Svelte 5 runes. Each backend area
has a store module in `src/lib/stores`. Shared pieces: `Modal` (focus handling, Escape), `Menu`
(keyboard navigation), `Message` (a message with markup in a placeholder), `loadAll` (showing load
errors instead of empty views).

## Texts and languages

All texts live in `messages/de.json` and `messages/en.json`, compiled by
[Paraglide JS](https://inlang.com/m/gerre34r/library-inlang-paraglideJs) into `src/lib/paraglide`
(`dev`, `build`, `check` and `test` do that themselves; `bun run i18n` does it alone).

- Components call messages directly: `m.gameForm_tabCount({ count })`. A message picked by an ID at
  runtime goes through `pickMsg(msgGroup.wineTools_label, id)`.
- A sentence with markup in it (a `<code>` path, a link) stays one message with a placeholder,
  rendered by `<Message message={m.…} parts={{ name: snippet }} />`, so translations can reorder it.
- Plurals are Paraglide variants in the message file, not a choice in the code.
- Backend errors render as `backendErrors_<code>` with the error's fields as placeholders. The few
  texts Rust shows itself (tray menu, native dialogs) come from the same files, built into the
  binary (`locale::text`).

`bun run test` checks that both languages have the same keys and placeholders, that every message
is used and every used key exists. `cargo test` checks that every `AppError` variant has its message.

To add a language, add `messages/<code>.json` with every key, list the code in
`project.inlang/settings.json`, extend the `Locale` types in `src/lib/i18n` and
`src-tauri/src/locale.rs`, and add it to the language switcher in `src/routes/+page.svelte`.

## Generated TypeScript types

The types the frontend receives and sends (games, settings, payloads, `AppError`, …) are generated
from the Rust types by [ts-rs](https://github.com/Aleph-Alpha/ts-rs). A type marked with
`#[cfg_attr(test, derive(ts_rs::TS), ts(export))]` is written to `src/lib/bindings/<Type>.ts` when
`cargo test` runs; `src/lib/types.ts` re-exports them. ts-rs is a dev-dependency only, so the app
doesn't carry it.

After changing such a type, run `cargo test` and commit the updated bindings. CI fails when they
differ from what the Rust types generate.

## Tests and CI

- **Backend:** each command module has unit tests in `src-tauri/src/commands/<name>/tests.rs`. Tests
  don't change the process environment: functions that read variables take an `Env`, and tests pass
  `env::fake(...)`. Temporary folders are `TestDir`s, removed when the test ends.
- **Frontend:** Vitest tests in `tests/` cover the pure helpers, the games store with a mocked Tauri
  API, and the message files.

CI (`.github/workflows/ci.yml`) runs on every pull request to `main` that isn't a draft:

1. `bun run check`
2. `bun run lint`
3. `bun run test`
4. `cargo fmt --check`
5. `cargo clippy --all-targets -- -D warnings`
6. `cargo test`, then a check that `src/lib/bindings` is up to date
7. `bun run build` and `cargo build --release`

CI uses the latest stable Rust, so a new Clippy lint can turn a pull request red even if nothing
else changed.

## Releases

Releases are built by CI from a tag `vX.Y.Z`, with the shared workflows from
[firsttris/workflows](https://github.com/firsttris/workflows):

1. *Actions → Bump version → Run workflow*, choosing patch, minor or major.
2. That raises the version in `package.json`, `src-tauri/tauri.conf.json` and
   `src-tauri/Cargo.toml` (and `Cargo.lock`), commits, tags, and starts `release.yml`.
3. `release.yml` runs the checks and tests, builds the AppImage, `.deb` and `.rpm` on Ubuntu 24.04,
   and attaches them to a GitHub release.

The AppImage is published as `*-ubuntu-compat.AppImage`. It bundles WebKitGTK from its build
environment, which can clash with very new graphics drivers; see
[Troubleshooting](troubleshooting.md#appimage-crashes-egl_bad_parameter-or-shows-a-blank-window).
