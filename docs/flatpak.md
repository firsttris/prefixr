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

The spike (phase 0) is done, see [Spike results](#spike-results): **go**. Points still marked
**open** need a setup the spike didn't have.

Two rules for all phases:

- **Bottles, Lutris and Heroic are the best sources.** They already run Wine, Proton and umu
  inside a Flatpak on Flathub. For every problem, look first at how they solve it, in their Flathub
  manifests and their code, and do the same where it fits. The plan names the launcher it copies.
- **The native builds must not break.** The AppImage, `.deb` and `.rpm` keep working exactly as
  today. Flatpak-only behaviour goes behind `is_flatpak()`; a shared code path only where the spike
  showed it is safe for both.

- [Why Flatpak](#why-flatpak)
- [The phases](#the-phases)
- [Phase 0: spike](#phase-0-spike)
- [Spike results](#spike-results)
- [Phase 1: the manifest](#phase-1-the-manifest)
- [Phase 2: Prefixr knows it runs as a Flatpak](#phase-2-prefixr-knows-it-runs-as-a-flatpak)
- [Phase 3: launching and stopping games](#phase-3-launching-and-stopping-games)
- [Phase 4: desktop, Steam and the command line](#phase-4-desktop-steam-and-the-command-line)
- [Phase 5: building from source and releasing](#phase-5-building-from-source-and-releasing)
- [Phase 6: Flathub](#phase-6-flathub)
- [Decisions](#decisions)

## Why Flatpak

Bazzite, SteamOS, Fedora Silverblue and the other immutable distributions are where many Linux
players are, and there an app comes from Flathub through Discover or GNOME Software. Today those
users get the AppImage, with its WebKitGTK problems (blank window, `EGL_BAD_PARAMETER`) and the
environment clean-up in `host_env.rs`. A Flatpak brings its own WebKitGTK from a shared runtime,
updates itself and shows up in the app stores next to Heroic, Lutris and Bottles.

What it costs: a second build to maintain, a few features that need another way inside the sandbox
(below), and a longer command for agents (`flatpak run … game add`).

## The phases

| Phase | What | Result | Status |
| --- | --- | --- | --- |
| 0 | Spike: repackage the `.deb`, test the risky parts | Go or no-go | Done: go (#48) |
| 1 | The manifest: runtime, permissions, extensions | A Flatpak that starts | Done (#48, #55) |
| 2 | Flatpak detection and the paths that move | Library, umu, shortcuts in the right places | Done (#49, #56, #59, #60) |
| 3 | Launching, tracking and stopping games | Games run and stop like in the native build | Done (#49, #52, #58) |
| 4 | Desktop shortcuts, Steam, Open With, CLI, migration | Feature parity, or a clear message where there is none | Done (#51, #53, #54), except the skill's command, the alias in the docs and the migration hint |
| 5 | Building from source, CI, `.flatpak` bundle in releases | Users can install it | Done (#55); the first bundle comes with the next release |
| 6 | Flathub submission | Prefixr on Flathub | Open: screenshots, a `<release>` per version, submission |

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

## Spike results

**Go.** Proton and Wine games run in the sandbox, and Prefixr can see every process of a game it
started. The biggest problem is *Stop*: today's kill sequence leaves a hung game behind in the
Flatpak. Phase 3 below is adjusted for it.

| | |
| --- | --- |
| Date | 2026-10-21 |
| Distribution | Bazzite 44.20260902 (Fedora 44 base, kernel 7.2.1-ogc4.1.fc44) |
| Desktop | KDE Plasma 6.7.4, Wayland |
| GPU / driver | AMD Navi 32 (`1002:747e`), amdgpu, Mesa from `GL.default` 26.08 |
| Flatpak | 1.18.1, `bwrap` not setuid, unprivileged user namespaces on |
| Prefixr | 0.1.10, `.deb` repackaged by [`flatpak/spike/io.github.firsttris.Prefixr.yml`](https://github.com/firsttris/prefixr/blob/main/flatpak/spike/io.github.firsttris.Prefixr.yml) |
| Tested with | umu-launcher 1.4.4, GE-Proton11-7 (steamrt4), Wine 11.20 (Kron4ek) with DXVK 3.1.1 |

How it was built: Bazzite has no `flatpak-builder` on the host, so the Flathub app
`org.flatpak.Builder` does it (`flatpak run org.flatpak.Builder --user --install --force-clean
flatpak/spike/build-dir flatpak/spike/io.github.firsttris.Prefixr.yml`). The `.deb` came from an
Ubuntu 24.04 distrobox and runs on the GNOME 51 runtime without problems. The library was copied
over with `prefixr backup create` (native) and `backup restore` (Flatpak). The Flatpak downloaded
umu, GE-Proton11-7 and Wine 11.20 itself (`prefixr runner download …`), which works.

### 1. A Proton game through umu

Works, started from the window and with `flatpak run io.github.firsttris.Prefixr game launch
<id>`. pressure-vessel starts its sub-sandbox through the Flatpak portal, the log is normal:

```
INFO: umu-launcher version 1.4.4
INFO: steamrt4 is up to date
INFO: Running 'GE-Proton11-7-x86_64' using runtime 'steamrt4'
```

umu uses the **host's** `~/.local/share/umu` (it detects the Flatpak itself). The existing
`steamrt4` from the native build was reused, nothing was downloaded again. `~/.var/app/<id>/data/umu`
stays empty. `HOST_XDG_DATA_HOME` isn't set on Bazzite (the host doesn't set `XDG_DATA_HOME`), so
the fallback to `~/.local/share/umu` in phase 2 is needed.

### 2. A Wine game

Works (Far Cry 5, Wine 11.20, DXVK). DXVK and the 32-bit extensions load. No Wine-specific problem.

### 3. Which game processes Prefixr can see

The most important result. Host view while Quake runs, started from the window:

```
2576388    1455 4026531836  \_ bwrap --args 78 -- prefixr                       (app instance)
2576399 2576398 4026535006  |   \_ prefixr
2599357 2576399 4026535006  |       \_ python3 …/umu-run …
2599370 2599357 4026535006  |           \_ steam-runtime-launch-client …
   3417    1455 4026531836  \_ /usr/libexec/flatpak-portal
2599438 2599428 4026535006      \_ bwrap --args 90 -- pv-adverb …              (sub-sandbox)
2599439 2599438 4026535006          \_ pv-adverb …
2599463 2599439 4026535006              \_ python3 …/proton waitforexitandrun …
2599496 2599439 4026535006              \_ …/GE-Proton11-7/files/bin/wineserver
2599581 2599439 4026535006              \_ S:\Games\…\Quake_x64_steam.exe
```

- **Prefixr sees the game's processes.** The sub-sandbox shares the PID namespace of the instance
  that started it (`4026535006`). From inside, all of them are in `/proc`.
- **The `PPid` chain is broken.** Inside the sandbox, the sub-sandbox's `bwrap` has `PPid: 0`: its
  real parent is `flatpak-portal` on the host. Walking down from `umu-run` (`process_descendants`)
  only finds `steam-runtime-launch-client`, never the game.
- **The environment marker works.** A per-game variable (`PREFIXR_GAME_ID=<id>`, set with `game
  edit --env`) is readable in `/proc/<pid>/environ` of every game process: proton, wineserver,
  the Wine services and the game itself.
- **Wine games:** `wineserver`, `services.exe`, `explorer.exe` and so on daemonize and end up below
  PID 1 of the app sandbox (`bwrap`), not below Prefixr. Natively they end up below the systemd
  user manager, so this is the same behaviour.
- **Play time and the running state work.** The card shows *läuft*, the tray lists the game, and
  after the end *Weiterspielen* and *Zuletzt gespielt* are updated. Prefixr waits for its own child
  (`umu-run` or the Wine process), which is still a real child.
- **Stop leaves a hung game behind** (Proton and Wine). Trace of the processes around a click on
  *Stop*, sampled every 0.5 s:

  ```
  08:41:31.997  umu-run, launch-client, pv-adverb, proton, wineserver, services.exe, Quake  running
  08:41:32.539  umu-run, launch-client, bwrap, pv-adverb, proton, wineserver               gone
  08:41:33.085  services.exe, explorer.exe … (PPid = app's bwrap)  stay
                [Quake_x64_steam] <defunct>, window "(Reagiert nicht)"                       stays
  ```

  The cause is `end_game_session`: `pkill -9 -f <exe path>` also matches `pv-adverb` and the
  proton script, whose command lines contain the exe path, and `wineserver -k9` kills the
  wineserver. Natively `pv-adverb` runs as PID 1 of its own namespace, and killing it takes the
  whole container down. In the Flatpak the container shares the app's namespace, so the Wine
  processes survive without their wineserver and hang. Far Cry 5 (Wine) leaves the same orphaned
  Wine services behind, and so does the native build: on the test machine `services.exe` and
  `explorer.exe` from native Far Cry sessions were still running hours later. Which stop sequence
  works instead is in [Stopping, measured](#stopping-measured).

### Stopping, measured

After the first results, the stop sequences of Lutris and Bottles were tried by hand on a game
started from the window, without changing Prefixr. A script sampled every process with the game's
marker every 0.5 s while one step ran (`flatpak enter <instance> …`, so the step runs inside the
app's sandbox). "Clean" means: no marked process left, no zombie, no window left.

| Test | Step | Result |
| --- | --- | --- |
| P1 (Lutris) | `umu-run wineboot -k` with `PROTON_VERB=runinprefix`, `GAMEID`, `PROTONPATH`, `WINEPREFIX` of the game | Clean: 16 processes, 0 after about 1.5 s. Same result twice |
| P1b | The same, after `umu-run` was already gone | Clean after about 1 s |
| P2 (Bottles) | SIGTERM to `umu-run` only | **Not enough:** `umu-run` and `steam-runtime-launch-client` end, the container with the game keeps running (14 processes after 15 s). Prefixr shows "Game exited with status 143" while the game still runs |
| P3 | SIGTERM to every marked process | Clean after about 2 s |
| W1 | SIGTERM to the Wine game's process (Prefixr's child) | Clean after about 4 s: `wineserver` ends the Wine services itself |
| W2 (Lutris) | `wineserver -k`, without `-9` | Clean after about 1 s |
| today | `wineserver -k9` and `pkill -9 -f <exe>` | Hangs (point 3) |

Conclusions:

- The hang comes from the hard kills, not from the sandbox. The soft steps of Lutris all work.
- For Proton, the reliable first step is `umu-run wineboot -k` (Lutris). Signalling `umu-run`
  (Bottles) isn't enough in the Flatpak, because the portal, not umu, owns the container. That
  P2 result differs from the very first run in point 3, where the outer `flatpak run` had been
  ended before. P2 is the clean measurement.
- When `umu-run` ends but marked processes remain, the game still runs. The end of a game has to
  look at the marked processes, not only at Prefixr's child.
- `wineboot -k` has to run with the game's `WINEPREFIX`, `GAMEID` and `PROTONPATH`. Without them
  umu falls back to `umu-default` and downloads UMU-Proton (happened once during the test; the
  download was removed).

### 4. A second `flatpak run` instance

- **Processes: no.** Every `flatpak run` gets its own PID namespace (`--command=sh` saw 4 processes).
- **`/tmp` and `$XDG_RUNTIME_DIR`: yes, shared.** Inside, `XDG_RUNTIME_DIR` is `/run/user/1000`,
  but it is not the host's: it is backed by `/run/user/1000/.flatpak/<id>/xdg-run` (and `…/tmp`),
  shared by all instances of the app. The first instance's lock file
  (`$XDG_RUNTIME_DIR/prefixr/<id>.lock`) is visible in a new one. Native and Flatpak locks don't
  see each other.
- **`prefixr game stop <id>` from a second instance does nothing, and says it worked:**
  `Stopped "Quake Enhanced"` after 0.1 s, the game keeps running. Its `pkill` finds nothing in its
  own namespace.
- **`game list --json`** works (186 games), it has no running state.
- **Ending the outer `flatpak run`** of a `game launch` (SIGTERM to the unit) doesn't reach Prefixr
  inside: `prefixr game launch` and the game kept running.

### 5. Tray, single instance, WebKitGTK

Tested combination: **KDE Plasma 6.7 Wayland, AMD/Mesa** and the same with X11 forced (`flatpak run
--nosocket=wayland --socket=x11`, XWayland). **Open:** GNOME and NVIDIA, which weren't available.

- **WebKitGTK:** renders correctly on Wayland and X11, with `WEBKIT_DISABLE_DMABUF_RENDERER=1`.
- **Window identity:** on Wayland the window's `app_id` is `prefixr`, on X11 the `WM_CLASS` is
  `Prefixr`. Neither matches `io.github.firsttris.Prefixr.desktop`, so the desktop can't connect the
  window to its desktop file (taskbar icon, pinning).
- **Tray:** the item registers with `org.kde.StatusNotifierWatcher` and has a menu. Its `IconName`
  is `/run/user/1000/tray-icon/tray-icon-main-tray-0.png`, a path inside the sandbox. KDE only
  showed the right icon because the native build had left a file with the same name in the host's
  `/run/user/1000/tray-icon`. On a clean system the icon would be missing. This confirms the
  `temp_dir_path` change in phase 4.
- **Single instance:** as expected, it doesn't work. A second start opens a second full app with a
  second window and tray icon. No `…SingleInstance` name shows up on the session bus.

### 6. Tools in the runtime

`flatpak run --command=sh org.gnome.Platform//51`:

| Present | Missing |
| --- | --- |
| `python3` (3.14), `pkill`, `pgrep`, `unzip`, `curl`, `wget`, `xdg-open` (the portal wrapper), `gdbus`, `update-desktop-database`, `gamemoderun`, `tar`, `xz`, `zstd` | `perl`, `powerprofilesctl`, `systemd-inhibit`, `systemctl`, `udisksctl`, `pkexec`, `7z`, `cabextract`, `unrar`, `git` |

Prefixr itself never calls `perl`. In the app, `mangohud` and `gamescope` are found through the
`PATH` from the manifest; `vkbasalt` isn't (its 26.08 extension wasn't installed). The portals
`OpenURI` (v5), `Inhibit` (v3) and `GameMode` answer.

### Steam Deck variables

Simulated on the desktop (not on a Deck): `SteamDeck`, `SteamGamepadUI`, `SteamOS`,
`XDG_CURRENT_DESKTOP=gamescope`, `GAMESCOPE_WAYLAND_DISPLAY` and `SteamAppId` all reach the
sandbox unchanged.

### Side findings

- Clicks on the *Weiterspielen* row sometimes started nothing, or a different game. The row sorts
  itself by last played after every launch, so a card moves while the pointer stays. *Spielen* in
  the details panel always worked. Not Flatpak-specific.
- `--socket=fallback-x11` only falls back when the host has no Wayland session. To test X11 on a
  Wayland desktop, add `--socket=x11`.

## Phase 1: the manifest

### Runtime and ID

- **Runtime `org.gnome.Platform` 51.** It includes WebKitGTK 4.1, which Tauri needs, and python3
  for umu and the store tools. Cinny uses the same. Heroic uses `org.freedesktop.Platform` because
  it's an Electron app; Lutris and Bottles also build on `org.gnome.Platform`.
- **App ID `io.github.firsttris.Prefixr`.** Flathub's rules: an app hosted on GitHub must use
  `io.github.<user>.<App>`, and `com.tristan.prefixr` would need the domain `tristan.com`. The Tauri
  `identifier` can stay `com.tristan.prefixr` for the native builds (see
  [single instance](#tray-and-single-instance)).

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
sandbox entirely). Only Lutris has it. Prefixr doesn't get it either
([first decision](#decisions)).

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
| Lock files (`games.rs`) | `$XDG_RUNTIME_DIR/prefixr` | Unchanged: the sandbox's `$XDG_RUNTIME_DIR` is shared by all instances of the app (spike, point 4). Native and Flatpak locks don't see each other | Heroic's `heroic-run` |

Without the umu change Prefixr would look for the runtime in the sandbox while umu puts it on the
host, and would show "setting up" before every launch.

### Features that work differently

| Feature | In the sandbox | Plan | Done the same way by |
| --- | --- | --- | --- |
| `vm.max_map_count` fix (`performance.rs`) | `pkexec` doesn't exist | Read the value as before, but `can_fix = false`, show the command to copy | none of them touch it; Lutris only warns about its own limit |
| Power profile (`powerprofilesctl`, `games.rs`) | not in the runtime (spike, point 6) | First hide it in the Flatpak; later `HoldProfile` over D-Bus on `org.freedesktop.UPower.PowerProfiles` | none of them have it |
| Keep the screen on (`systemd-inhibit`, `games.rs`) | no host systemd | The Inhibit portal (`org.freedesktop.portal.Inhibit`), done in #56 | Faugus |
| Check for a competing scheduler (`systemctl`, `games.rs`) | no host systemd | Skip it in the Flatpak | — |
| GameMode (`gamemoderun`) | the runtime has it, and the GameMode portal answers (spike, point 6) | Unchanged. **Open:** whether GameMode really turns on; the test machine has no `gamemoded` | Heroic, turned on by default |
| MangoHud, Gamescope, vkBasalt | only with the extension | `command_on_path` finds them through `PATH`; if missing, a hint with `flatpak install …VulkanLayer.<name>//26.08`, done in #60. vkBasalt has no 26.08 build on Flathub yet, so its hint says so | Bottles, Heroic (reads the version from `/app/manifest.json`) |
| Mounting a disc image (`udisksctl`, `cli/install.rs`) | not in the runtime | UDisks2 over D-Bus (`Manager.LoopSetup`, `Filesystem.Mount`), done in #59 | all four have the UDisks2 permission |
| `distrobox-host-exec` (`runners.rs`) | not applicable | Skip it in the Flatpak | — |
| `update-desktop-database` | may be missing | Best effort; ignore errors | — |
| Opening links (`xdg_open`) | the runtime's `xdg-open` is the portal wrapper, OpenURI v5 answers (spike, point 6) | Unchanged | — |

## Phase 3: launching and stopping games

The spike settled how this works (see [Spike results](#spike-results), points 3 and 4).

**What the spike showed.** Today `run_game` starts the game in its own process group, and
`process_descendants` (`games.rs`) finds its processes along `PPid`. In the Flatpak:

- pressure-vessel's sub-sandbox shares the PID namespace of the Prefixr instance that started it.
  That instance sees every game process in `/proc`. (Bottles' remark that such processes "run in a
  separate sandbox instance the host cannot see by scanning /proc" is about its own sandbox mode
  with `flatpak-spawn`, not about pressure-vessel.)
- The `PPid` chain breaks at the sub-sandbox's `bwrap` (`PPid: 0`). `process_descendants(umu-run)`
  never reaches the game.
- An environment marker in the game's environment is readable in every game process.
- Prefixr's own child (`umu-run`, or the Wine process) stays alive until the game ends, so waiting
  on it still gives the end of the game and the play time.
- Today's `kill_running_game` uses `wineserver -k9` and `pkill -9 -f <exe>`. The `pkill` also
  kills `pv-adverb`, and the game hangs. The soft steps of Lutris (`umu-run wineboot -k`,
  `wineserver -k`, SIGTERM to the marked processes) each end the game cleanly. A SIGTERM to
  `umu-run` alone does not: the container keeps running ([Stopping, measured](#stopping-measured)).
- A second `flatpak run` instance sees none of this.

**What the others do:**

- **Lutris** puts `LUTRIS_GAME_UUID` in the game's environment and finds the game's processes by
  reading `/proc/*/environ`, then keeps those whose command line contains the game folder or
  `pressure-vessel` (`runners/wine.py`, `filter_game_pids`). To stop an umu game it runs `umu-run
  wineboot -k` with `PROTON_VERB=runinprefix`, a Wine game gets `wineserver -k` (without `-9`). It
  waits about 2 s, sends SIGTERM to what's left, polls for 5 s and only then sends SIGKILL
  (`game.py`, `force_stop` and `force_kill_delayed`). The end of a game is its heartbeat: no
  thread left and no marked process left (`Game.beat`).
- **Bottles** starts `umu-run` in a session of its own and stops it with SIGTERM to that process
  group, waits up to 5 s, then SIGKILL (`backend/umu/executor.py`, `_terminate`). For Wine it uses
  `wineboot -k`. It also finds processes by `BOTTLE=<path>` in `/proc/*/environ`.
- **Faugus** puts `FAUGUSID=<id>` in the environment and kills by `/proc/*/environ` too
  (`utils.py`). It detects the end with `GLib.child_watch_add`.
- **Heroic** waits for its child to exit and stops with `pkill -f` plus `wineserver -k`. It has the
  same risk as Prefixr today, so it isn't the model here.
- **umu** itself sets `PR_SET_CHILD_SUBREAPER`, starts a new session and passes SIGTERM and SIGINT
  on to its process tree (`umu_run.py`).

**Plan (Lutris for finding and stopping, measured in the spike):**

1. `run_game` sets `PREFIXR_GAME_ID=<id>` in the game's environment, in both builds. It is only an
   extra variable, so the native build behaves as before.
2. A game's processes are those whose `/proc/<pid>/environ` holds that marker, plus the process
   group.
3. Stopping, in this order (done in #49 for the Flatpak and #52 natively):
   1. Proton: `umu-run wineboot -k` with `PROTON_VERB=runinprefix` and the game's `WINEPREFIX`,
      `GAMEID` and `PROTONPATH` (Lutris; test P1). Wine: `wineserver -k` without `-9` (Lutris;
      test W2).
   2. Wait up to 5 s for the marked processes to end (Lutris, Bottles).
   3. SIGTERM to the marked processes that are left (test P3), wait 5 s.
   4. Only then SIGKILL to what's still left.

   Not first: a SIGTERM to `umu-run` (Bottles). In the Flatpak it ends umu but not the container
   (test P2). No `pkill -f <exe>` and no `wineserver -k9`: they cause the hang.
4. The native build uses the same sequence whenever it can see the marked processes (#52, after
   the [third decision](#decisions) changed). Today's `kill_running_game` and `end_game_session`
   stay as the fallback where it can't: a distrobox, whose rootless container can't read the host's
   `/proc/<pid>/environ`, and games started by an older Prefixr.
5. The end of a game stays as today: when Prefixr's own child exits. umu waits for the game
   (`waitforexitandrun`), so this is the end of the game. A first version also waited for the last
   marked process (Lutris' heartbeat). Review of #50 showed that a Wine game sharing its prefix with
   another running game then stays "running" for that whole game, because `wineserver` and the wine
   services carry the first game's marker. It was removed again.

**A second instance.** `prefixr game stop`, a desktop shortcut or `--run` from a Steam shortcut
starts a second `flatpak run` instance with its own PID namespace (spike, point 4). It can't see the
games of another instance. In the Flatpak:

- Desktop shortcuts (`--launch`) and "Open with" (`--install`) go to the window over the
  single-instance plugin with `dbus_id` set to the app ID (#51, see
  [single instance](#tray-and-single-instance)). Lutris and Bottles do the same with `GApplication`
  on their own app ID, Heroic with Electron's single-instance lock.
- Whichever instance runs a game (the window, a CLI `game launch`, a Steam `--run`) owns a D-Bus name
  for it while it runs, `io.github.firsttris.Prefixr.Game.g<id>`, below the app ID (#58). `game stop`
  calls its `Stop` method, which ends the game like the Stop button, and waits until the game's lock
  is released. A first version handed the stop to the window only (#51), which missed games started
  by `game launch` or Steam.
- If no instance holds the game's lock file, `game stop` says the game isn't running. If one does but
  doesn't take the stop (still preparing, or an older Prefixr), it says that instead of `Stopped`.
- Ending the outer `flatpak run` doesn't reach Prefixr inside (spike, point 4). **Open:** what Steam
  does when it stops a Prefixr shortcut.

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
(`ensure_install_desktop_entry`). The Flatpak keeps doing that (#54), into the host's
`~/.local/share/applications` (see `host_data_home`), with `Exec="flatpak" "run"
"io.github.firsttris.Prefixr" --install %f` and a file name of its own,
`io.github.firsttris.Prefixr.install.desktop`, so it doesn't replace the native build's entry. With
access to home, `/mnt`, `/media` and `/run/media`, the path a file manager passes arrives as it is.

Not done for now: a second desktop file in the manifest that Flatpak exports with `flatpak run
--file-forwarding … @@ %f @@`, as Bottles has. Paths would then arrive through the document portal
as `/run/user/<uid>/doc/…`, which Bottles turns back with `GetHostPaths` on
`org.freedesktop.portal.Documents` (`resolve_portal_path` in `utils/manager.py`). If Flathub's
review asks for an exported entry instead of one written at runtime, that is the way, and then
`find_install_arg` (`lib.rs`) and `add_prefix` have to resolve such paths, since a prefix must never
end up under `/run/user/…/doc`.

### Steam

- **Shortcut:** `Exe="flatpak"`, `LaunchOptions="run io.github.firsttris.Prefixr --run <id>"`,
  `StartDir=$HOME`. Heroic (`nonesteamgame.ts`) and Lutris (`shortcut.py`) do the same, and native
  Steam starts it without problems. `find_shortcut` matches on `--run <id>` and keeps working.
- **Steam as a Flatpak** can't start another Flatpak, unless the user gives it
  `--talk-name=org.freedesktop.Flatpak` (Faugus documents that in its README; Steam's manifest
  doesn't have it). `SteamOnlyFlatpak` stays, with that hint added to the message.
- **Closing and restarting Steam** (`stop_steam`, `start_steam`, `steam_running`, `steam.rs`) needs
  the `steam` command and the host's processes, neither of which exists in the sandbox. Prefixr
  takes the first of these two ways ([first decision](#decisions)):
  - Without extra permissions: Prefixr asks the user to close Steam, writes `shortcuts.vdf` and
    tells them to start Steam again. Whether Steam is running can't be checked with `kill(pid, 0)`
    on `~/.steam/steam.pid` as Heroic does: the spike confirmed that the sandbox only sees its own
    PID namespace (point 4). Heroic's Flatpak has exactly this check and silently gets "not
    running". Lutris uses `pgrep`, which doesn't see the host either. **Open:** whether anything
    works without extra permissions. Steam wasn't running during the spike. Until then, Prefixr
    treats the state as unknown in the Flatpak and always asks the user to close Steam.
  - Not chosen: with `--talk-name=org.freedesktop.Flatpak`, as Lutris has on Flathub,
    `flatpak-spawn --host steam -shutdown` and so on would work exactly as today. A user who adds
    the permission with `flatpak override` could get this later.

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

- **Tray:** in the Flatpak, `TrayIconBuilder::temp_dir_path($XDG_RUNTIME_DIR/app/$FLATPAK_ID)`
  (`tray.rs`). That folder is the same path on the host, so the tray host can read the icon file.
  The spike showed that the default (`$XDG_RUNTIME_DIR/tray-icon`) is private to the sandbox.
  Heroic's `heroic-run` sets `TMPDIR` to the same folder. Lutris passes the app ID as an icon
  name instead, which the `tray-icon` crate doesn't support. The native build keeps the default.
- **Single instance:** the plugin takes the D-Bus name `<identifier>.SingleInstance`
  (`platform_impl/linux.rs`). An app may own names below its own ID without a permission, but
  `com.tristan.prefixr…` isn't below `io.github.firsttris.Prefixr`, and Flathub never grants an
  `--own-name` outside the ID. The spike confirmed it: a second start opens a second app. So in the
  Flatpak: `Builder::new().dbus_id(flatpak_id())`. 2.4.5, the version in `Cargo.lock`, has
  `dbus_id`. The native build keeps its current name.
- **Window identity:** the window reports `app_id` `prefixr` (Wayland) and `WM_CLASS` `Prefixr`
  (X11), which doesn't match the Flatpak's desktop file (spike, point 5). As in Lutris: in the
  Flatpak, `glib::set_prgname(FLATPAK_ID)` before the window is created, and
  `StartupWMClass=io.github.firsttris.Prefixr` in the Flatpak's desktop file only. The native
  desktop file and program name stay as they are.

## Phase 5: building from source and releasing

Flathub requires a build from source without network access, as Cinny does. The manifest is
[`flatpak/io.github.firsttris.Prefixr.yml`](https://github.com/firsttris/prefixr/blob/main/flatpak/io.github.firsttris.Prefixr.yml)
(#55); the spike manifest stays for repackaging a local `.deb`.

- **Rust:** `flatpak/cargo-sources.json` from `flatpak-cargo-generator.py src-tauri/Cargo.lock`,
  SDK extension `rust-stable`, `cargo --offline fetch`.
- **Frontend:** `flatpak-node-generator` supports npm, yarn and pnpm, but not `bun.lock`, and
  Flathub makes no exceptions for uncommon build setups. So a `package-lock.json` is kept next to
  `bun.lock`, with exactly bun's versions: npm writes the tree bun installed, and
  `flatpak/lock-from-bun.py` adds the download URLs and checksums from `bun.lock`, failing on any
  difference. (A fresh `npm install --package-lock-only` resolved 77 packages to other versions.)
  `flatpak/node-sources.json` comes from that lock. In the Flatpak build, `npm ci --offline`, SDK
  extension `node24`.
- **One script for all three:** `flatpak/update-sources.sh`. The `Flatpak` workflow runs it on every
  pull request that touches a lock file or `flatpak/`, and fails when the committed files differ.
  bump.yml's shared workflow already raises the version in `package-lock.json` along with
  `package.json`.
- **Build:** `tauri build --no-bundle` with `beforeBuildCommand` overridden to `npm run build`
  (`tauri.conf.json` calls `bun run build`). `npm run build` no longer calls bun itself. Then the
  binary, icons, desktop file and metainfo go into `/app` (as Cinny does).
- **Modules:** libayatana-appindicator from flathub/shared-modules; 7zip, cabextract and unrar as
  Heroic builds them, for winetricks with a Wine runner.
- **Metainfo:** `flatpak/io.github.firsttris.Prefixr.metainfo.xml` with description, links, content
  rating, branding and releases. Screenshots are still missing; Flathub needs them (phase 6).
- **CI:** `.github/workflows/flatpak.yml` builds the bundle with
  [flatpak-github-actions](https://github.com/flatpak/flatpak-github-actions) in the `gnome-51`
  image. `release.yml` calls it and attaches `prefixr_<version>_x86_64.flatpak` to the release, next
  to the AppImage, `.deb` and `.rpm`. Installation: `flatpak install --user
  ./prefixr_<version>_x86_64.flatpak`.

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

## Decisions

Made on 2026-10-21, after the spike.

1. **No `--talk-name=org.freedesktop.Flatpak`.** The sandbox stays closed. Prefixr asks the user to
   close and restart Steam themselves and shows the `sysctl` command for `vm.max_map_count` to copy
   (Heroic, Bottles). A user can still add the permission with `flatpak override`. Lutris is the
   only one of the four with it.
2. **`--filesystem=home`.** Games and prefixes can be anywhere, and an agent passes any path
   (Lutris, Faugus).
3. **The marker and the Lutris stop in both builds** (changed on the same day). First the new stop
   sequence was meant for the Flatpak only. The native build turned out to leave orphaned wine
   services behind too (`services.exe` from killed sessions still running 18 hours later), and the
   Lutris sequence tested clean natively as well. So it runs wherever the game's marked processes
   can be seen; the old sequence stays as the fallback where they can't (a distrobox, games started
   by an older Prefixr). Done in #49 (Flatpak) and #52 (native).
4. **Releases first, Flathub later.** A `.flatpak` bundle in the GitHub releases next to the
   AppImage, so sandbox issues show up with users who installed it on purpose before Flathub's
   review.
