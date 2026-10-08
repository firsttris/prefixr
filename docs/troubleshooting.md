# Troubleshooting & FAQ

- [A game doesn't start](#a-game-doesnt-start)
- [Common messages](#common-messages)
- [The app itself](#the-app-itself)
  - [AppImage crashes (EGL_BAD_PARAMETER) or shows a blank window](#appimage-crashes-egl_bad_parameter-or-shows-a-blank-window)
  - [Clearing the WebKit cache](#clearing-the-webkit-cache)
  - [No tray icon](#no-tray-icon)
- [FAQ](#faq)
- [Reporting a bug](#reporting-a-bug)

## A game doesn't start

1. **Read the log.** After a failed start the game shows the error and a **Show log** button. The
   log contains everything Wine/Proton and the game printed. The last lines usually name the
   problem, such as a missing DLL.
2. **Wait out the first launch.** The first start in a new prefix sets it up. With Proton it also
   downloads umu-launcher and the Steam Linux Runtime (a few hundred MB). The card shows
   *Starting…* until then.
3. **Set the protonfixes match** (Proton runners). In the game's **Proton** tab, search for the game
   so umu applies its known fixes.
4. **Try another runner.** Some games need a newer Proton-GE, some an older one, and some run better
   on plain Wine or the other way round.
5. **Install missing dependencies.** A log mentioning `MSVCP140.dll`, `VCRUNTIME140.dll`,
   `d3dx9_43.dll`, `xinput1_3.dll` or similar means a runtime is missing. Install it in the prefix
   with **Install dependencies** (see [winetricks](user-guide.md#installing-dependencies-winetricks)).
6. **Check the Proton switches.** For old DirectX 8/9 games with graphical glitches, try
   **OpenGL instead of DXVK (WineD3D)**. For crashes related to Nvidia detection, try
   **Disable NVAPI**.
7. **Look the game up on [ProtonDB](https://www.protondb.com/).** Other players' reports often name
   the exact runner, switches or launch arguments that work. Most of them translate directly into a
   game's settings in Prefixr.

## Common messages

| Message | What to do |
| --- | --- |
| *Don't run Prefixr as root* | Start Prefixr as your normal user. Files in a prefix used as root belong to root, and your session can't use them afterwards. |
| *Could not read the configuration* | `config.json` is damaged. Prefixr left it untouched; fix the JSON or move the file away (see [Configuration file](configuration.md#configuration-file)). |
| *The game is already starting or running* | The game runs, possibly in a Prefixr that Steam started. Use **Stop**, or end it from Steam. |
| *GitHub's rate limit for unauthenticated requests is exhausted* | Add a GitHub token on the Runners page (see [GitHub token](user-guide.md#github-token)) or wait an hour. |
| *… already contains files but doesn't look like a Wine or Proton prefix* | Prefixr only adds empty folders or existing prefixes (with a `drive_c` inside). Choose an empty folder or the prefix folder itself. |
| *Steam is only installed as a Flatpak* | The Flatpak Steam can't start programs outside its sandbox. Install Steam from your distribution's packages to add games to Steam. |
| *Restart Steam?* | Steam must be closed while Prefixr changes its shortcuts. Confirm to let Prefixr quit and restart it (a running Steam game closes too), or quit Steam yourself first. |
| *vm.max_map_count too low* | Click **Increase now** on the Performance page, or run the command shown there. |

**The overlay, Gamescope or vkBasalt don't show up.** Prefixr leaves each of them out at launch when
the tool isn't installed. Gamescope is also left out when Prefixr itself runs inside a Gamescope
session, as in the Steam Deck's Game Mode.

**GameMode seems to have no effect.** If a scheduler daemon such as ananicy or scx runs, Prefixr
skips GameMode on purpose and uses the performance power profile instead. The game's log says so.

## The app itself

### AppImage crashes (EGL_BAD_PARAMETER) or shows a blank window

**Symptom:** Launching the AppImage immediately fails with

```text
Could not create default EGL display: EGL_BAD_PARAMETER. Aborting...
```

or the window opens but stays completely white. This was seen with an AMD GPU on a very recent
distribution (Bazzite/Fedora 44, Mesa 26.2.2), but not on another PC with an NVIDIA GPU.

**Cause:** The AppImage bundles the WebKitGTK version that was current when it was built (in an
Ubuntu 24.04 build container). That version doesn't work with very recent Mesa and graphics driver
versions. Neither `WEBKIT_DISABLE_DMABUF_RENDERER=1`, `WEBKIT_DISABLE_COMPOSITING_MODE=1`,
`GDK_BACKEND=x11` nor `LIBGL_ALWAYS_SOFTWARE=1` fix it reliably: at best they prevent the crash, but
the window stays white.

**Workaround:** Use the `.deb` or `.rpm` package where your distribution supports it, or build
Prefixr yourself. Both link against your system's WebKitGTK, which renders normally on the same
machine:

```bash
bun install
bun run tauri build   # not "cargo build --release": that doesn't embed the frontend,
                      # and the app then tries to reach the dev server
./src-tauri/target/release/prefixr
```

See [Building from source](development.md#building-from-source) for the build requirements.

**Proper fix (still open):** Build the AppImage without bundling WebKitGTK and GTK, so it always uses
the host's libraries. Swapping only the bundled `libwebkit2gtk-4.1.so.0` for the system's failed on a
further incompatibility with the also-bundled GLib, so the bundle needs to be reworked as a whole.

On crashing, the process sometimes stays behind as a zombie and has to be ended with `kill -9`.

### Clearing the WebKit cache

The window is a WebKitGTK webview, whose cache lives in
`~/.local/share/com.tristan.prefixr/WebKitCache/`. Next to it are `CacheStorage/` (Cache API),
`storage/` (IndexedDB) and `localstorage/` (LocalStorage).

Quit Prefixr first (otherwise WebKit writes the files back on exit), then:

```bash
rm -rf ~/.local/share/com.tristan.prefixr/WebKitCache
```

Deleting `localstorage/` as well resets the remembered language and library view. Your library and
settings are not stored there and stay.

### No tray icon

GNOME shows no tray icons without the
[AppIndicator extension](https://extensions.gnome.org/extension/615/appindicator-support/). Prefixr
notices that: closing the window then quits Prefixr, or only minimizes it while a game is running,
so you can always get back to it to stop the game.

## FAQ

**Is Prefixr a replacement for Steam's own Proton?**
No. For games you own on Steam, Steam's Proton integration is the simplest choice. Prefixr is for
everything else: GOG, Epic, itch.io, EA, Ubisoft and Battle.net installers, standalone games and old
CD games. You can still add those to Steam through Prefixr.

**Can I use my existing prefixes from Lutris, Bottles, Heroic or PortProton?**
Yes. Add the prefix folder under **Prefixes** and point the game at it. Prefixr uses the prefix as it
is and doesn't move or convert anything.

**Proton or Wine: which runner should I use?**
Start with Proton-GE. It runs through umu in the Steam Linux Runtime, gets protonfixes, and is what
most compatibility reports refer to. Use a plain Wine build for games or tools that misbehave under
Proton, or when you need a specific Wine version.

**Does Prefixr work on the Steam Deck?**
Prefixr is a desktop app. On SteamOS, use the AppImage in Desktop Mode, then add your games to Steam
to play them in Game Mode.

**Does Prefixr send any data anywhere?**
Only what its features need: release lookups and downloads from GitHub (runners, umu, DXVK,
VKD3D-Proton, winetricks), wine-mono from WineHQ, the umu database for the protonfixes search, and
SteamGridDB searches and images when you set an API key. There is no telemetry.

**Where are my save games?**
Inside the game's prefix, usually in `drive_c/users/steamuser/` (`Documents`, `AppData`,
`Saved Games`). Removing a game or a prefix from Prefixr never deletes them.

## Reporting a bug

[Open an issue](https://github.com/firsttris/prefixr/issues) with:

- your distribution and GPU,
- how you installed Prefixr (AppImage, .deb, .rpm or built from source) and its version,
- the runner and the game,
- the log from **Show log**, if a game is involved.
