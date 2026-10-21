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
- [Documentation website](#documentation-website)

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
| `bun run screenshots` | Takes the screenshots in `docs/` from a demo library (see [Screenshots](#screenshots)) |
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
skills/prefixr/         The Agent Skill (SKILL.md) built into `prefixr skill install`
src-tauri/
  src/
    lib.rs              App setup, single-instance handoff, command registration
    cli.rs              The command line (`prefixr game …`), with its tests in cli/tests.rs;
                        library, store, backup and skill commands in cli/<name>.rs
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
| `backup` | Backups and restoring them, the automatic backups and their settings |
| `games` | The library, launching and stopping games, installers, shortcuts, the `--run` headless mode |
| `prefixes` | The prefix list, and finding the real prefix inside a Steam compatdata folder |
| `runners` | Detecting runners and building the commands that run them |
| `runner_downloads` | Listing, downloading, verifying and unpacking runner releases |
| `umu`, `umu_database` | The managed umu-launcher copy, and the protonfixes (UMU ID) search |
| `graphics_layers` | DXVK, VKD3D-Proton and wine-mono for Wine runners |
| `winetricks`, `wine_tools` | Installing dependencies, and Wine's own tools |
| `performance`, `graphics`, `mangohud`, `proton_options` | The settings categories, and the config files and environment they turn into |
| `steamgriddb` | Artwork search, download and cache |
| `stores` (`epic`, `gog`, `amazon`, `helpers`) | Store accounts: the pinned store tools, the library cache and covers, installs, and the command line a store game starts with |
| `steam`, `binary_vdf` | Adding games to Steam through its binary `shortcuts.vdf` |
| `icons`, `shell_link`, `logs` | Exe icons, reading `.lnk` shortcuts, and log rotation |
| `github`, `locale` | The GitHub token, and the UI language as Rust needs it |

## Architecture

**Commands and state.** The frontend calls Rust through Tauri commands (`invoke()` in
`src/lib/stores/*`). The config lives in one `Mutex<AppConfig>`. Commands take it through
`LockExt::locked()`, which also accepts a poisoned lock, and never hold it across an `.await`.
Every change is saved to `config.json` with a write to a temporary file and a rename.

**Command line.** `prefixr game …`, `library …`, `prefix …`, `store …` and the rest are parsed with
[clap](https://docs.rs/clap) in `cli.rs`. They start the app without a window, as `--run` does
(all but `skill`, which needs no app), and call the same Tauri command functions the frontend
invokes, passing `app.state()` for their `State` parameters, so there is no second implementation
to keep in sync. Progress comes from the same events the frontend listens to. To add a command, add
a variant to the matching subcommand enum and call the existing command function. When you add,
rename or change a command an agent uses, update `skills/prefixr/SKILL.md` too: a test parses
every example command in it. The window's process watches `config.json` (`watch_config_file`):
when another process changed it, the config is reloaded and the frontend gets `config-changed` and
refreshes the library.

**Errors.** Every command returns `AppError`, serialized as `{ code, ...fields }`. The frontend turns
it into text with `backendError()`, so an error stays in the chosen language even after switching.
`AppError::Other` carries text that stays untranslated, for technical failures.

**Events.** Long-running work reports through events: `game-initializing`, `game-started`,
`game-exited` and `game-launch-error` for launches, `config-changed` when another process changed
`config.json`, `runner-download-progress` and
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

The Tauri system packages come from the Actions cache (`apt-install` from
[firsttris/workflows](https://github.com/firsttris/workflows)), because the Ubuntu mirror is
sometimes very slow.

CI uses the latest stable Rust, so a new Clippy lint can turn a pull request red even if nothing
else changed.

### Screenshots

The pictures in `docs/screenshot-*.png` (README, the Flatpak's metainfo) are taken by
`bun run screenshots`: it builds the frontend and runs `scripts/screenshots.mjs`, which serves
`build/` and opens it in a headless Chromium with [Playwright](https://playwright.dev/). There is no
Rust backend: a stand-in for Tauri's IPC (`scripts/screenshots-demo.mjs`) answers every command
from a made-up library with drawn covers, so the pictures never show anyone's real games. A
command without an answer is logged and fails the run; add its demo data there.

The first run needs Playwright's browser: `bunx playwright install --only-shell chromium`. After a
change to the look, run *Actions → Screenshots → Run workflow*: it takes the pictures in the
official Playwright image, so fonts and rendering are the same every time, and commits the ones
that changed.

## Flatpak

Prefixr is also built as a Flatpak, `io.github.firsttris.Prefixr`, on the GNOME 51 runtime.
`release.yml` attaches `prefixr_<version>_x86_64.flatpak` to every release. The launchers already
on Flathub ([Lutris](https://github.com/flathub/net.lutris.Lutris),
[Heroic](https://github.com/flathub/com.heroicgameslauncher.hgl),
[Bottles](https://github.com/flathub/com.usebottles.bottles)) were the model for every decision
below; when something in the sandbox needs a new answer, look at how they solve it first. The native
builds (AppImage, `.deb`, `.rpm`) must keep working as before: Flatpak-only behaviour goes behind
`host_env::is_flatpak()`.

### Building it

```bash
flatpak run org.flatpak.Builder --user --install --force-clean --disable-rofiles-fuse \
  flatpak/build-dir flatpak/io.github.firsttris.Prefixr.yml
```

(`flatpak-builder` instead of `flatpak run org.flatpak.Builder` where it's installed on the host.)
The manifest builds from the checkout without network access, as Flathub requires. That needs:

- `flatpak/cargo-sources.json` (from `Cargo.lock`) and `flatpak/node-sources.json` (from
  `package-lock.json`). `flatpak-node-generator` can't read `bun.lock`, so `package-lock.json` is
  kept with exactly bun's versions: `flatpak/lock-from-bun.py` fills in the download URLs and
  checksums from `bun.lock` and fails on any difference.
- After changing `bun.lock` or `Cargo.lock`, run `flatpak/update-sources.sh` (with
  [flatpak-builder-tools](https://github.com/flatpak/flatpak-builder-tools) in `FBT`) and commit the
  results. The `Flatpak` workflow checks that they match.
- `flatpak/spike/io.github.firsttris.Prefixr.yml` repackages a local `.deb` instead
  (`bun run tauri build --bundles deb` first). It's quicker for trying a change, not for releases.

Running it: `flatpak run io.github.firsttris.Prefixr`, and the CLI as
`flatpak run io.github.firsttris.Prefixr game list`.

### What works differently in the sandbox

- **Paths:** the library and settings live in `~/.var/app/io.github.firsttris.Prefixr`. umu's
  runtime, the app menu and Steam are the host's (`host_data_home`). With an empty library, the
  window points to a native library in `~/.config/com.tristan.prefixr` and explains moving it over
  with *Backup*.
- **Stopping games:** pressure-vessel's container runs below the Flatpak portal, in the app's PID
  namespace, so the `PPid` chain from umu-run breaks. Every game gets `PREFIXR_GAME_ID` in its
  environment, and `stop_game_gracefully` ends its marked processes as Lutris does:
  `umu-run wineboot -k` or `wineserver -k`, then SIGTERM, then SIGKILL. The native build does the
  same wherever it can see the marked processes. Hard kills (`pkill -9`, `wineserver -k9`) left hung
  games and orphaned Wine services behind.
- **Other instances:** every `flatpak run` is its own PID namespace. The window owns the
  single-instance name below the app ID (`--launch`, `--install`), and whichever instance runs a
  game owns `io.github.firsttris.Prefixr.Game.g<id>`, whose `Stop` method `game stop` calls.
- **No host commands:** the Flatpak doesn't get `--talk-name=org.freedesktop.Flatpak`. So Prefixr
  asks the user to close Steam before changing its shortcuts, and shows the `sysctl` command for
  `vm.max_map_count` instead of running `pkexec`. A user can add the permission with
  `flatpak override`.
- **Portals and D-Bus instead of tools:** keeping the system awake goes through the Inhibit portal,
  mounting a disc image through UDisks2. MangoHud and Gamescope are Flathub extensions
  (`org.freedesktop.Platform.VulkanLayer.<Name>//26.08`); the settings show the install command
  when one is missing. vkBasalt has no 26.08 build on Flathub yet.
- **Desktop:** shortcuts and the Steam entry run `flatpak run io.github.firsttris.Prefixr`. The tray
  icon goes to `$XDG_RUNTIME_DIR/app/<id>`, the one folder with the same path on the host, and the
  program name is the app ID, so the window matches the desktop file.

### Still open

- Flathub: a `<release>` entry in the metainfo per version, then the submission (screenshots come
  from `bun run screenshots`).
- Not tested yet: GNOME, NVIDIA, the Steam Deck, GameMode with `gamemoded`, and what Steam does when
  it stops a Prefixr shortcut (ending the outer `flatpak run` doesn't reach Prefixr inside).

## Releases

Releases are built by CI from a tag `vX.Y.Z`, with the shared workflows from
[firsttris/workflows](https://github.com/firsttris/workflows):

1. *Actions → Bump version → Run workflow*, choosing patch, minor or major.
2. That raises the version in `package.json`, `src-tauri/tauri.conf.json` and
   `src-tauri/Cargo.toml` (and `Cargo.lock`), commits, tags, and starts `release.yml`.
3. `release.yml` runs the checks and tests (skipped when `main`'s CI already passed for the code,
   since the release commit only changes versions), builds the AppImage, `.deb` and `.rpm` on
   Ubuntu 24.04, and attaches them to a GitHub release.

The AppImage is published as `prefixr_<version>_amd64.AppImage` (up to 0.1.10 as
`…_amd64-ubuntu-compat.AppImage`). It bundles WebKitGTK from its build environment, which can clash
with very new graphics drivers; see
[Troubleshooting](troubleshooting.md#appimage-crashes-egl_bad_parameter-or-shows-a-blank-window).

## Documentation website

These pages are published at [firsttris.github.io/prefixr](https://firsttris.github.io/prefixr/),
built from `docs/` with [MkDocs Material](https://squidfunk.github.io/mkdocs-material/). The
navigation and theme are in `mkdocs.yml`.

`.github/workflows/docs.yml` builds the site with `mkdocs build --strict` for every pull request that
changes the docs, which fails on broken links and anchors, and publishes it to GitHub Pages on every
push to `main`. Changes to the docs alone don't run the main CI.

To preview the site locally:

```bash
python3 -m venv .venv && . .venv/bin/activate
pip install -r requirements-docs.txt
mkdocs serve   # http://127.0.0.1:8000, reloads on every change
```

Links between pages stay plain relative Markdown links (`user-guide.md#artwork`), so the docs read the
same on GitHub and on the website.
