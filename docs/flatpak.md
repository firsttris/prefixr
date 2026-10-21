# Plan: Flatpak

How Prefixr could run as a Flatpak and, later, be published on Flathub. Prefixr is a game launcher
that drives a lot of host tools (Wine, Proton, umu, Gamescope, Steam), which is exactly what a
sandbox makes harder. So for every point where Flatpak gets in the way, this plan looks at how the
launchers already on Flathub solve it, and picks the same solution where it fits:
[Heroic](https://github.com/flathub/com.heroicgameslauncher.hgl),
[Lutris](https://github.com/flathub/net.lutris.Lutris),
[Bottles](https://github.com/flathub/com.usebottles.bottles),
[Faugus Launcher](https://github.com/flathub/io.github.Faugus.faugus-launcher) and the
[Steam Flatpak](https://github.com/flathub/com.valvesoftware.Steam), plus
[Cinny](https://github.com/flathub/in.cinny.Cinny), a Tauri app on Flathub.

Nothing here is built yet. Points marked **to verify** are assumptions the spike (phase 0) has to
confirm before the work that depends on them starts.

- [Why Flatpak](#why-flatpak)
- [The phases](#the-phases)
- [Phase 0: spike](#phase-0-spike)
- [Phase 1: the manifest](#phase-1-the-manifest)
- [Phase 2: Prefixr knows it runs as a Flatpak](#phase-2-prefixr-knows-it-runs-as-a-flatpak)
- [Phase 3: launching and stopping games](#phase-3-launching-and-stopping-games)
- [Phase 4: desktop, Steam and the command line](#phase-4-desktop-steam-and-the-command-line)
- [Phase 5: building from source and releasing](#phase-5-building-from-source-and-releasing)
- [Phase 6: Flathub](#phase-6-flathub)
- [Decisions to make](#decisions-to-make)

## Why Flatpak

Bazzite, SteamOS, Fedora Silverblue and the other immutable distributions are where many Linux
players are, and there an app comes from Flathub through Discover or GNOME Software. Today those
users get the AppImage, with its WebKitGTK problems (blank window, `EGL_BAD_PARAMETER`) and the
environment clean-up in `host_env.rs`. A Flatpak brings its own WebKitGTK from a shared runtime,
updates itself and shows up in the app stores next to Heroic, Lutris and Bottles.

What it costs: a second build to maintain, a few features that need another way inside the sandbox
(below), and a longer command for agents (`flatpak run … game add`).

## The phases

| Phase | What | Result |
| --- | --- | --- |
| 0 | Spike: repackage the `.deb`, test the risky parts | Go or no-go |
| 1 | The manifest: runtime, permissions, extensions | A Flatpak that starts |
| 2 | Flatpak detection and the paths that move | Library, umu, shortcuts in the right places |
| 3 | Launching, tracking and stopping games | Games run and stop like in the native build |
| 4 | Desktop shortcuts, Steam, Open With, CLI, migration | Feature parity, or a clear message where there is none |
| 5 | Building from source, CI, `.flatpak` bundle in releases | Users can install it |
| 6 | Flathub submission | Prefixr on Flathub |

## Phase 0: spike

Before anything else, a manifest that repackages the release `.deb` (the "closed source" route in
[Tauri's Flatpak guide](https://github.com/tauri-apps/tauri-docs/blob/v2/src/content/docs/distribute/flatpak.mdx)),
built locally with `flatpak-builder`, with the permissions from phase 1. It is not meant to be
released. With it, test:

1. **A Proton game through umu.** pressure-vessel has to start the Steam Linux Runtime as a
   sub-sandbox through the Flatpak portal. That needs Flatpak 1.12 or newer and unprivileged user
   namespaces on the host (`kernel.unprivileged_userns_clone=1`, a setuid `bwrap` doesn't work, see
   Valve's
   [known issues](https://github.com/ValveSoftware/steam-runtime/blob/master/doc/steamlinuxruntime-known-issues.md)).
2. **A Wine game** with a Kron4ek runner, DXVK and wine-mono.
3. **Which game processes Prefixr can see:** the play time, the tray's list of running games and
   *Stop* depend on it (phase 3). Write down the `PPid` chain and the PID namespace the game's
   processes end up in.
4. **A second `flatpak run` instance**, the way `prefixr game stop` or a Steam shortcut starts one:
   does it see the first instance's games, its `/tmp` and `$XDG_RUNTIME_DIR`?
5. **Tray icon, single instance and the WebKitGTK view** on GNOME and KDE, X11 and Wayland, Mesa
   and NVIDIA.
6. **Which tools the runtime has:** `flatpak run --command=sh org.gnome.Platform//51`, then check for
   `python3`, `pkill`, `unzip`, `curl`, `perl`, `xdg-open`, `gdbus`, `update-desktop-database`.

If 1 or 3 fails and can't be fixed, stop here: Flatpak would then lose what Prefixr is for.

## Phase 1: the manifest

### Runtime and ID

- **Runtime `org.gnome.Platform` 51.** It includes WebKitGTK 4.1, which Tauri needs, and python3
  for umu and the store tools. Cinny uses the same. Heroic uses `org.freedesktop.Platform` because
  it's an Electron app; Lutris and Bottles also build on `org.gnome.Platform`.
- **App ID `io.github.firsttris.Prefixr`.** Flathub's rules: an app hosted on GitHub must use
  `io.github.<user>.<App>`, and `com.tristan.prefixr` would need the domain `tristan.com`. The Tauri
  `identifier` can stay `com.tristan.prefixr` for the native builds (see
  [single instance](#single-instance)).

### Permissions

Most of these come from the four launchers' manifests, which all use the same set for Wine and
pressure-vessel:

```yaml
finish-args:
  - --share=ipc
  - --share=network
  - --socket=wayland
  - --socket=fallback-x11
  - --socket=pulseaudio
  - --device=all                      # GPU, controllers
  - --allow=multiarch                 # 32-bit
  - --allow=devel                     # launchers and anti-cheat that need ptrace
  - --allow=per-app-dev-shm           # pressure-vessel
  # Games, prefixes and game folders can be anywhere (Lutris, Faugus: home)
  - --filesystem=home
  - --filesystem=/mnt
  - --filesystem=/media
  - --filesystem=/run/media           # SD card on the Steam Deck
  - --filesystem=/run/udev:ro         # controllers (Steam, Heroic)
  # umu keeps the Steam Linux Runtime in the host's ~/.local/share/umu (umu_consts.py)
  - --filesystem=xdg-data/umu:create
  # Flatpak Steam; Lutris lists it on its own although it has home, so does Heroic
  - --filesystem=~/.var/app/com.valvesoftware.Steam
  - --filesystem=xdg-config/MangoHud:ro
  - --filesystem=xdg-run/gamescope-0
  - --talk-name=org.kde.StatusNotifierWatcher   # tray
  - --talk-name=org.freedesktop.Notifications
  - --system-talk-name=org.freedesktop.UDisks2  # Wine's drives, mounting disc images
  # Vulkan layers and drivers into the runtime's sub-sandbox (Steam, Heroic, Lutris)
  - --env=XDG_DATA_DIRS=/app/share:/usr/lib/extensions/vulkan/share:/usr/share:/usr/share/runtime/share:/run/host/user-share:/run/host/share:/usr/lib/pressure-vessel/overrides/share
  - --env=PATH=/app/bin:/usr/bin:/usr/lib/extensions/vulkan/MangoHud/bin:/usr/lib/extensions/vulkan/gamescope/bin
  - --env=LD_LIBRARY_PATH=/usr/lib/extensions/vulkan/gamescope/lib
  # Variables Steam leaves behind that confuse the runtime (Steam, Heroic)
  - --unset-env=STEAM_RUNTIME
  - --unset-env=STEAM_RUNTIME_PREFER_HOST_LIBRARIES
  - --unset-env=SDL_VIDEODRIVER
  # WebKitGTK's DMA-BUF renderer shows a blank view on some drivers (Lutris)
  - --env=WEBKIT_DISABLE_DMABUF_RENDERER=1
```

`org.freedesktop.portal.Flatpak`, the portal pressure-vessel uses for its sub-sandbox, needs no
permission: every app may talk to portals. Heroic, Bottles and Faugus manage without
`--talk-name=org.freedesktop.Flatpak` (the permission for `flatpak-spawn --host`, which leaves the
sandbox entirely). Only Lutris has it. Whether Prefixr needs it is the
[first decision](#decisions-to-make).

### 32-bit libraries

The same blocks in all four manifests; Faugus has the shortest version:

```yaml
add-extensions:
  org.freedesktop.Platform.Compat.i386:
    directory: lib/i386-linux-gnu
    version: '26.08'
  org.freedesktop.Platform.GL32:
    directory: lib/i386-linux-gnu/GL
    version: '1.4'
    versions: 26.08;26.08-extra;1.4
    subdirectories: true
    no-autodownload: true
    autodelete: false
    add-ld-path: lib
    merge-dirs: vulkan/icd.d;glvnd/egl_vendor.d;egl/egl_external_platform.d;OpenCL/vendors;lib/dri;lib/d3d;lib/gbm;vulkan/explicit_layer.d;vulkan/implicit_layer.d
    download-if: active-gl-driver
    enable-if: active-gl-driver
    autoprune-unless: active-gl-driver
```

plus a `bundle-setup` module that creates `/app/lib/i386-linux-gnu` and writes
`/app/etc/ld.so.conf` with that path. The versions follow the runtime's freedesktop base (26.08 for GNOME 51). Prefixr's
Kron4ek runner is the WoW64 build (`runner_downloads.rs`), which needs no 32-bit libraries itself,
but 32-bit OpenGL and Vulkan drivers and other runners do.

### Modules

- **Tray:** `libayatana-appindicator` from
  [flathub/shared-modules](https://github.com/flathub/shared-modules) (git submodule). The GNOME
  runtime doesn't have it. Cinny uses the older `libappindicator` from there.
- **winetricks' tools:** `cabextract` (with libmspack), `7zip` (with `7z`/`7za` links) and `unrar`,
  as Heroic builds them. `direct_winetricks_command` needs them for Wine runners. `umu-run
  winetricks` brings its own.
- **Not bundled:** MangoHud, Gamescope and vkBasalt come as Flathub extensions
  (`org.freedesktop.Platform.VulkanLayer.MangoHud` and so on), which the user installs. Lutris
  downloads the Gamescope and MangoHud ones automatically. Prefixr only puts their `bin` folders on
  `PATH` (above) and shows a hint when one is missing (phase 2).
- **Store tools and umu stay downloads.** Legendary and gogdl are Python zipapps, Nile is a
  PyInstaller binary, umu is a zipapp: they all run on the runtime's python3, as they do in Heroic's
  Flatpak. Heroic, Lutris, Faugus and Bottles all download runners or umu at runtime and are on
  Flathub. Bottles bundles umu as a module instead, which is an option if review asks for it.

## Phase 2: Prefixr knows it runs as a Flatpak

### Detection

One `is_flatpak()` in `host_env.rs`, true when `FLATPAK_ID` is set or `/.flatpak-info` exists. That
covers both checks the others use: Heroic and Bottles check `FLATPAK_ID`, Lutris checks
`/.flatpak-info`, Faugus checks both. Plus `flatpak_id()`, used for the `flatpak run` commands
below. Every Flatpak-specific branch goes through these two.

### Paths

| What | Today | In the Flatpak | Done the same way by |
| --- | --- | --- | --- |
| Library, config, artwork | `~/.local/share/com.tristan.prefixr`, `~/.config/prefixr` | `~/.var/app/<id>/data/…`, `…/config/…` (Tauri does this) | all; migration in phase 4 |
| umu's runtime (`umu_local_dir`, `umu.rs`) | `$XDG_DATA_HOME/umu` | `$HOST_XDG_DATA_HOME/umu`, otherwise `~/.local/share/umu` | umu itself, Lutris, Bottles |
| App menu shortcuts (`applications_directory`, `games.rs`) | `$XDG_DATA_HOME/applications` | `~/.local/share/applications` | Lutris |
| Steam (`steam_root`, `steam.rs`) | `$XDG_DATA_HOME/Steam` among others | `~/.local/share/Steam`, `~/.steam` | Heroic, Lutris |
| Lock files (`games.rs`) | `$XDG_RUNTIME_DIR/prefixr` | `$XDG_RUNTIME_DIR/app/<id>/prefixr` (**to verify**: shared between instances) | Heroic's `heroic-run` |

Without the umu change Prefixr would look for the runtime in the sandbox while umu puts it on the
host, and would show "setting up" before every launch.

### Features that work differently

| Feature | In the sandbox | Plan | Done the same way by |
| --- | --- | --- | --- |
| `vm.max_map_count` fix (`performance.rs`) | `pkexec` doesn't exist | Read the value as before, but `can_fix = false`, show the command to copy | none of them touch it; Lutris only warns about its own limit |
| Power profile (`powerprofilesctl`, `games.rs`) | not in the runtime (**to verify**) | First hide it in the Flatpak; later `HoldProfile` over D-Bus on `org.freedesktop.UPower.PowerProfiles` | none of them have it |
| Keep the screen on (`systemd-inhibit`, `games.rs`) | no host systemd | The Inhibit portal (`org.freedesktop.portal.Inhibit`) | Faugus |
| Check for a competing scheduler (`systemctl`, `games.rs`) | no host systemd | Skip it in the Flatpak | — |
| GameMode (`gamemoderun`) | the runtime has it, talks to the GameMode portal | Unchanged (**to verify**) | Heroic, turned on by default |
| MangoHud, Gamescope, vkBasalt | only with the extension | `command_on_path` finds them through `PATH`; if missing, a hint with `flatpak install …VulkanLayer.<name>` for the runtime's version | Bottles, Heroic (reads the version from `/app/manifest.json`) |
| Mounting a disc image (`udisksctl`, `cli/install.rs`) | not in the runtime | UDisks2 over D-Bus (`Loop.Setup`, `Filesystem.Mount`) | all four have the UDisks2 permission |
| `distrobox-host-exec` (`runners.rs`) | not applicable | Skip it in the Flatpak | — |
| `update-desktop-database` | may be missing | Best effort; ignore errors | — |
| Opening links (`xdg_open`) | the runtime's `xdg-open` goes through the OpenURI portal | Unchanged (**to verify**) | — |

## Phase 3: launching and stopping games

This is the riskiest part, and the spike decides how it is done.

**The problem.** Today `run_game` starts the game in its own process group, and `process_descendants`
(`games.rs`) finds its processes along `PPid`. Inside the Flatpak, pressure-vessel starts the game
through the portal in a sub-sandbox. Its processes are probably not children of umu-run any more,
and the `PPid` chain is broken. Bottles says outright that such processes "run in a separate
sandbox instance the host cannot see by scanning /proc" (`backend/managers/sandbox.py`).

**What the others do:**

- **Lutris** puts `LUTRIS_GAME_UUID` in the game's environment and finds the game's processes by
  reading `/proc/*/environ`, then keeps those whose command line contains the game folder or
  `pressure-vessel` (`runners/wine.py`). It stops an umu game with `umu-run wineboot -k`
  (`PROTON_VERB=runinprefix`) and only then SIGKILLs what's left (`game.py`).
- **Faugus** puts `FAUGUSID=<id>` in the environment and kills by `/proc/*/environ` too
  (`utils.py`). It detects the end with `GLib.child_watch_add`.
- **Heroic** waits for its child to exit, optionally with `wineserver --wait`, and stops with `pkill
  -f` plus `wineserver -k`.
- **umu** itself sets `PR_SET_CHILD_SUBREAPER`, starts a new session and passes SIGTERM and SIGINT
  on to its process tree (`umu_run.py`).

**Plan (the Lutris approach, for the native build too):**

1. `run_game` sets `PREFIXR_GAME_ID=<id>` in the game's environment.
2. A game's processes are those whose `/proc/<pid>/environ` holds that marker, in addition to the
   process group. This also works natively, so there is one code path, not two.
3. Stopping a Proton game: first `umu-run wineboot -k` in the game's prefix, then SIGTERM and finally
   SIGKILL to the marked processes. A Wine game keeps `wineserver -k`.
4. The game has ended when umu-run has exited and no marked process is left. Play time is counted
   until then.

**A second instance.** `prefixr game stop` or `--run` from a Steam shortcut starts a second
`flatpak run` instance, possibly in a PID namespace of its own (spike, point 4). If it can't see the
games, it hands the command to the instance with the window, like the existing single-instance
handoff (`pending-launch`) does, instead of acting itself.

## Phase 4: desktop, Steam and the command line

### Desktop shortcuts

`own_executable_path` (`games.rs`) returns `/app/bin/prefixr` in the Flatpak, a path that doesn't
exist on the host. In the Flatpak the `Exec=` line becomes:

```ini
Exec=flatpak run io.github.firsttris.Prefixr --launch <id>
```

As in Lutris (`xdgshortcuts.py`: `flatpak run net.lutris.Lutris`), Faugus and Bottles. Heroic goes
through its own URL scheme instead (`xdg-open heroic://launch?…`). `game_shortcuts` finds shortcuts
by `--launch <id>`, so it keeps working, also for shortcuts from before. `desktop_directory` reads
`~/.config/user-dirs.dirs` and keeps working too, because `HOME` is the real home.

### Open With for installers

Prefixr writes the *Install Prefixr* entry with `Exec=… --install %f` itself at runtime
(`ensure_install_desktop_entry`). In the Flatpak it is a second desktop file in the manifest
instead, `io.github.firsttris.Prefixr.Install.desktop`, with the MIME types for `.exe` and `.msi`, as
Bottles declares them in its own entry. Flatpak exports it and turns its `Exec=` into a
`flatpak run --file-forwarding … @@ %f @@`.

Paths can then arrive through the document portal as `/run/user/<uid>/doc/…`, even with access to
home. Bottles turns them back into real paths with `GetHostPaths` on
`org.freedesktop.portal.Documents` (`resolve_portal_path` in `utils/manager.py`); Heroic rejects
them. Prefixr does it like Bottles: in `find_install_arg` (`lib.rs`) and for folders from the file
dialog (`add_prefix`). A prefix must never end up under `/run/user/…/doc`.

### Steam

- **Shortcut:** `Exe="flatpak"`, `LaunchOptions="run io.github.firsttris.Prefixr --run <id>"`,
  `StartDir=$HOME`. Heroic (`nonesteamgame.ts`) and Lutris (`shortcut.py`) do the same, and native
  Steam starts it without problems. `find_shortcut` matches on `--run <id>` and keeps working.
- **Steam as a Flatpak** can't start another Flatpak, unless the user gives it
  `--talk-name=org.freedesktop.Flatpak` (Faugus documents that in its README; Steam's manifest
  doesn't have it). `SteamOnlyFlatpak` stays, with that hint added to the message.
- **Closing and restarting Steam** (`stop_steam`, `start_steam`, `steam_running`, `steam.rs`) needs
  the `steam` command and the host's processes, neither of which exists in the sandbox. Two ways, see
  the [first decision](#decisions-to-make):
  - Without extra permissions: Prefixr asks the user to close Steam, writes `shortcuts.vdf` and
    tells them to start Steam again. Whether Steam is running can be read from `~/.steam/steam.pid`,
    but probably not checked with `kill(pid, 0)` as Heroic does, since the host's PIDs aren't visible
    in the sandbox; **to verify** what works instead.
  - With `--talk-name=org.freedesktop.Flatpak`, as Lutris has on Flathub: `flatpak-spawn --host
    steam -shutdown` and so on, exactly as today.

### Command line and the skill

- In the Flatpak the command is `flatpak run io.github.firsttris.Prefixr game add …`. Bottles works
  the same way (`flatpak run --command=bottles-cli com.usebottles.bottles`), and none of them sets up
  an alias. Prefixr's docs suggest `alias prefixr='flatpak run io.github.firsttris.Prefixr'`.
- `prefixr skill install` writes the `flatpak run` form into `SKILL.md` when `is_flatpak()`, so the
  agent uses the right command from the start. `~/.claude/skills` is covered by `home`.
- Paths an agent passes must be visible in the sandbox, which `home`, `/mnt`, `/media` and
  `/run/media` cover. The window still picks up CLI changes through `watch_config_file`: both
  processes see the same `config.json` in `~/.var/app/<id>/config`.

### Moving over from the native build

None of the other launchers migrate. Prefixr already has what's needed: back up in the native build
(*Settings → Backup*), restore the file in the Flatpak. The prefixes and games stay where they are;
only the library and settings move. On the first start, if `~/.local/share/com.tristan.prefixr`
exists, the Flatpak points to this. An automatic import can come later.

### Tray and single instance

- **Tray:** `TrayIconBuilder::temp_dir_path(app_cache_dir)` (`tray.rs`), so the icon file is
  somewhere the tray host can read it without `xdg-run/tray-icon` (Tauri's guide).
- **Single instance:** the plugin takes the D-Bus name `<identifier>.SingleInstance`
  (`platform_impl/linux.rs`). An app may own names below its own ID without a permission, but
  `com.tristan.prefixr…` isn't below `io.github.firsttris.Prefixr`, and Flathub never grants an
  `--own-name` outside the ID. So in the Flatpak: `Builder::new().dbus_id(flatpak_id())`. 2.4.5,
  the version in `Cargo.lock`, has `dbus_id`.

## Phase 5: building from source and releasing

Flathub requires a build from source without network access, as Cinny does:

- **Rust:** `cargo-sources.json` from `flatpak-cargo-generator.py src-tauri/Cargo.lock`, SDK
  extension `rust-stable`.
- **Frontend:** `flatpak-node-generator` supports npm, yarn and pnpm, but not `bun.lock`, and
  Flathub makes no exceptions for uncommon build setups. So a `package-lock.json` is kept next to
  `bun.lock` (`npm install --package-lock-only`), and CI checks that both stay in step. In the
  Flatpak build, `npm ci --offline`, SDK extension `node24`.
- **Build:** `tauri build --no-bundle` with `beforeBuildCommand` overridden to `npm run build`
  (`tauri.conf.json` calls `bun run build`), then install the binary, icons, desktop files and
  metainfo into `/app` (as Cinny does).
- **Metainfo:** `io.github.firsttris.Prefixr.metainfo.xml` with description, screenshots, releases
  and content rating. Flathub checks it.
- **CI:** a job with [flatpak-github-actions](https://github.com/flatpak/flatpak-github-actions)
  that builds a `.flatpak` bundle. `release.yml` attaches it to the release, next to the AppImage,
  `.deb` and `.rpm`. Installation:
  `flatpak install --user ./prefixr.flatpak`.

## Phase 6: Flathub

- Submit `io.github.firsttris.Prefixr` through a pull request to
  [flathub/flathub](https://github.com/flathub/flathub). Flathub then hosts the manifest in its own
  repository, and `x-checker-data` keeps the sources up to date.
- The ID is verified through the GitHub login of the owner of `github.com/firsttris/prefixr`.
- Reviewers will ask about `--filesystem=home`, `--device=all` and anything beyond. The reasons are
  the same as for Lutris and Faugus, which have both. `--talk-name=org.freedesktop.Flatpak` would
  need a separate justification.
- Flathub doesn't accept apps that are only a "thin wrapper" around other tools. Prefixr's library,
  prefix management and store integration clearly go beyond that.

## Decisions to make

1. **`--talk-name=org.freedesktop.Flatpak`, yes or no?** With it, Prefixr could restart Steam and
   run `pkexec` for `vm.max_map_count` as it does today (Lutris). Without it, the sandbox stays
   closed and Prefixr asks the user to do these two things themselves (Heroic, Bottles).
   Recommendation: without, at least for Flathub. A user can add it later with `flatpak override`.
2. **`--filesystem=home` or single folders?** Heroic lists folders, Lutris and Faugus take `home`.
   Prefixr's games and prefixes can be anywhere, and an agent passes any path. Recommendation:
   `home`.
3. **Tracking processes by environment marker natively too?** Recommendation: yes, one code path
   for both builds, if the spike shows it works.
4. **Start with a `.flatpak` in the releases or go straight to Flathub?** Recommendation: releases
   first, so the sandbox issues show up with users who installed it on purpose before Flathub's
   review.
