# User guide

A tour of everything Prefixr does. New to Prefixr? Start with [Getting started](getting-started.md).

- [The library](#the-library)
- [Adding and editing games](#adding-and-editing-games)
- [Installing a game from its setup program](#installing-a-game-from-its-setup-program)
- [Prefixes](#prefixes)
- [Runners](#runners)
- [Game settings](#game-settings)
  - [Performance](#performance)
  - [Graphics](#graphics)
  - [Overlay](#overlay)
  - [Proton](#proton)
- [Artwork](#artwork)
- [Steam integration](#steam-integration)
- [Shortcuts](#shortcuts)
- [The tray icon](#the-tray-icon)
- [Logs](#logs)
- [Language](#language)

## The library

The **Library** shows your games as cover cards (grid view) or as a compact list (list view);
switch with the two buttons next to the sort menu. Search filters by name, and you can sort by name
or by runner. Prefixr remembers the view you chose.

Each game shows its SteamGridDB cover if you picked one, otherwise the icon from its `.exe`.

| State | What you see |
| --- | --- |
| **Starting…** | The launch is being prepared. On first use this includes downloads and setting up the prefix, which can take a few minutes. |
| **Running** | The game's process runs. The play button turns into a stop button. |
| **Error** | The launch failed or the game exited with an error. The message and a **Show log** button appear below the game. |

**Stop** ends the game's whole session: its process tree, the umu container for a Proton game, and
the Wine session for a Wine game. Use it when a game hangs. It is the same as closing a game the
hard way, so unsaved progress is lost.

The **⋮** menu on a card (in the list view, the 🔗 button plus the buttons next to it) has:

- **Select artwork**: see [Artwork](#artwork).
- **Edit game**: the same dialog as when adding it.
- **To desktop** / **To start menu**: see [Shortcuts](#shortcuts).
- **Add to Steam** / **Update in Steam** / **Remove from Steam**: see
  [Steam integration](#steam-integration).
- **Remove**: removes the game from the library together with its logs, artwork, shortcuts and
  settings files. The prefix and the game's files stay. A running game can't be removed.

## Adding and editing games

**+ Add game** opens the game dialog. The **General** tab holds what every game needs:

| Field | Notes |
| --- | --- |
| **Name** | Filled in from the `.exe`'s file name (`BaldursGate3.exe` becomes *Baldurs Gate 3*). Change it freely. |
| **Program (.exe)** | The game's executable. Its embedded icon becomes the game's icon. |
| **Prefix** | Which prefix the game runs in. See [Prefixes](#prefixes). |
| **Runner** | Proton or Wine build to run it with. See [Runners](#runners). |
| **Environment variables** | `KEY=VALUE`, one per line. These win over every switch in the other tabs. `WINEDLLOVERRIDES` and `LD_PRELOAD` are added to what Prefixr sets, not replaced. |
| **Launch arguments** | Passed to the game's `.exe`, for example `--launcher-skip -dx11`. Quote arguments with spaces: `-path "C:\My Game"`. |

The other tabs (**Performance**, **Graphics**, **Overlay**, and **Proton** for a Proton runner)
override the global settings for this game only. See [Game settings](#game-settings).

## Installing a game from its setup program

For games that come as an installer:

1. Right-click the setup `.exe` in your file manager → **Open With** → **Install Prefixr**.
   Prefixr opens the **Install with Prefixr** dialog. If Prefixr is already running, the existing
   window takes over.
2. Choose a prefix (or create one with **+**) and a runner, then click **Run setup**. The prefix is
   prepared first, exactly as for a game.
3. Go through the installer. While it runs, the dialog stays open.
4. When the installer exits, Prefixr scans the prefix's desktop and start menu for shortcuts the
   installer created. Pick the one that starts the game, or choose the `.exe` yourself.
5. The game dialog opens with everything filled in. Check it and click **Add**.

If no shortcut is found or setup failed, **Show setup log** shows what the installer printed.

## Prefixes

A prefix is a self-contained Windows environment: its own `C:` drive, registry and installed
programs, separate from your Linux system and from other prefixes.

### Adding a prefix

Under **Prefixes**, choose a folder and click **Add prefix**:

- **A new or empty folder** becomes a new prefix. Nothing is written until the first game or tool
  runs in it.
- **An existing prefix** (a folder with a `drive_c` inside) is used as it is. That works for
  prefixes from Lutris, Bottles, PortProton, Heroic or plain Wine.
- **A Steam `compatdata/<appid>` folder** works too: Prefixr uses the `pfx` folder inside it.

A folder that already contains other files is refused, so a prefix never ends up mixed into
unrelated data.

Each prefix shows how many games use it.

### Installing dependencies (winetricks)

**Install dependencies** (📦) opens the winetricks dialog for a prefix. The common packages are
listed with a short explanation: the Visual C++ runtimes 2005 to 2022, .NET Framework 4.8, the
Windows core fonts, the DirectX shader compiler, D3DX9, XAudio/XACT, DirectShow, PhysX, GDI+ and the
Visual Basic 6 runtime. **Browse more packages** searches all of winetricks' DLL and font packages.
Packages already installed in the prefix are marked with ✓.

Pick the runner to install with (it's preselected when all of the prefix's games use the same one)
and click **Install**. Installing downloads the original installers from their vendors and can take
several minutes. A Proton runner that ships protonfixes (such as Proton-GE) installs through
umu-launcher, inside the same Steam Runtime the game later runs in.

The packages go into the prefix, so they apply to every game using it.

### Wine tools

**Open Wine tools** (🛠️) starts one of Wine's own programs in the prefix:

| Tool | Use it for |
| --- | --- |
| Wine configuration (`winecfg`) | Windows version, DLL overrides, graphics and drive settings |
| Registry editor (`regedit`) | Editing the prefix's registry directly |
| Command prompt (`cmd`) | A Windows command line in the prefix |
| File explorer (`winefile`) | Browsing the virtual `C:` drive |
| Task manager (`taskmgr`) | Seeing and ending Windows processes in the prefix |
| Uninstall programs (`uninstaller`) | Removing programs installed in the prefix |

### Removing a prefix

**Remove** only takes a prefix off Prefixr's list. The folder, its programs and save games stay on
disk, and games that use it keep working. A prefix whose game is running can't be removed.

## Runners

The **Runners** page lists the runners Prefixr found in its runners folder, each with the number of
games using it.

### Downloading runners

**Download runner** shows the recent releases of each source with date and size:

- **Proton-GE** (GloriousEggroll), the most widely used Proton build for non-Steam games
- **Proton-CachyOS**, the plain x86_64 build
- **Wine (Kron4ek)**, the staging WoW64 build, which runs 32-bit games without a separate 32-bit
  Wine

Every download is checked against the checksum file published with the release before it's
unpacked.

A runner can be deleted once no game uses it. A runner that is a symlink to another folder only
loses the link.

### umu-launcher

Proton runners start through [umu-launcher](https://github.com/Open-Wine-Components/umu-launcher).
Prefixr keeps its own copy, downloaded the first time a Proton game starts, and shows its version in
the **umu-launcher** section. When a newer release exists, the section offers the update.

The Steam Linux Runtime, which umu downloads on first use (a few hundred MB), is shared with Lutris,
Heroic and other umu-based launchers. If one of them already downloaded it, Prefixr reuses it.

### DXVK and VKD3D-Proton

Plain Wine builds don't include [DXVK](https://github.com/doitsujin/dxvk) (Direct3D 8 to 11 on
Vulkan) or [VKD3D-Proton](https://github.com/HansKristian-Work/vkd3d-proton) (Direct3D 12 on
Vulkan). Prefixr downloads the latest release of each the first time a Wine game starts and links
them into the prefix on every launch. The **DXVK / VKD3D-Proton** section shows the versions in use
and updates both on request. Proton runners bring their own.

### GitHub token

Runner, umu and DXVK lookups use GitHub's API, which allows 60 anonymous requests per hour. If you
hit that limit, add a [personal access token](https://github.com/settings/tokens) (no scopes needed)
in the **GitHub** section on the Runners page. It raises the limit to 5000 requests per hour.

## Game settings

The four pages under **Default for all games** (Performance, Graphics, Overlay, Proton) set how every
game launches. Each game can override any of them in the matching tab of its game dialog:

- Changing a value in the game dialog makes it an override for that game, highlighted in the dialog.
  The tab shows how many overrides it has.
- The reset button next to an override goes back to the global setting.
- An override that ends up equal to the global setting is dropped, so the game follows future
  global changes again.

### Performance

| Setting | Effect | Needs |
| --- | --- | --- |
| **GameMode** | Raises CPU governor and process priority while the game runs | [GameMode](https://github.com/FeralInteractive/gamemode) |
| **Performance power profile** | Holds the *Performance* power profile for as long as the game runs, and releases it even if the game crashes | power-profiles-daemon |
| **Prevent sleep** | Keeps the system and screen from sleeping while the game runs | systemd-inhibit |

If a scheduler daemon that manages process priorities is active (ananicy, ananicy-cpp, scx,
scx_loader or falcond), GameMode would fight it over the same settings. Prefixr then skips GameMode
and uses the performance power profile instead, and notes that in the game's log.

The page also checks `vm.max_map_count`. Some games and anti-cheat systems (Easy Anti-Cheat titles,
Baldur's Gate 3, Elden Ring, Diablo IV and others) crash when it's low. Values from 1048576, which
current Fedora, Arch, Ubuntu and SteamOS ship, count as enough. Below that, **Increase now** sets
Steam's value permanently through a `/etc/sysctl.d` file (with a password prompt via `pkexec`). If
`pkexec` isn't available, the page shows the command to run yourself.

### Graphics

**Gamescope** runs the game in its own nested compositor. You can set the width and height it renders
at, an FPS limit, and force fullscreen. It's skipped when Gamescope isn't installed or when Prefixr
itself already runs inside a Gamescope session (as in Steam's Game Mode).

**vkBasalt** adds post-processing: **Sharpen (CAS)**, with adjustable strength, and **Anti-aliasing
(SMAA)**. Sharpening costs almost nothing and helps when rendering below native resolution.

### Overlay

The [MangoHud](https://github.com/flightlessmango/MangoHud) overlay shows FPS, frame times, CPU and
GPU load, temperatures and more inside the game. Start from a preset:

| Preset | Shows |
| --- | --- |
| **Minimal** | Just the FPS, tucked into a corner |
| **Standard** | FPS, load and temperatures at a glance |
| **Detailed** | Every value, including status icons and technical info such as driver and Wine version |
| **Competitive** | Large and bold, nothing else |

Then adjust which values appear, the position, color, background opacity, rounded corners and a
horizontal layout. The preview shows the result. With Gamescope enabled, MangoHud runs as
Gamescope's own overlay (`--mangoapp`) when available.

### Proton

These are the switches Proton reads from the environment (`PROTON_*`). Prefixr reads them from the
selected runner's own `proton` script, so you only see what that runner supports. Each switch has
three states: **Default** leaves the decision to Proton and protonfixes, **On** and **Off** force it.

The common switches have a description, for example HDR, the Wayland driver, WineD3D instead of
DXVK, updating DLSS, FSR 4 or XeSS, NVAPI, NTSync and Fsync, a per-game shader cache, more memory for
32-bit games, and controllers via SDL. All other switches the runner understands are listed under
**Advanced**.

In a game's **Proton** tab you also set its **protonfixes match** (the UMU ID). With the right ID,
umu applies the fixes known for exactly that game, and Proton its game-specific tweaks. Search by
name: Steam matches come from SteamGridDB (with an API key), the rest from the
[umu database](https://github.com/Open-Wine-Components/umu-database). Several entries for one game
belong to different stores (GOG, Epic, …); pick the store the game came from. You can also type an
ID, and a plain Steam app ID such as `1091500` becomes `umu-1091500`.

## Artwork

With a free [SteamGridDB API key](https://www.steamgriddb.com/profile/preferences/api), entered on
the **SteamGridDB** page, **Select artwork** finds images for a game:

| Kind | Used for |
| --- | --- |
| **Cover** | The card in Prefixr's library, and the portrait cover in Steam |
| **Icon** | Desktop and menu shortcuts, and the icon in Steam |
| **Wide cover**, **Hero**, **Logo** | Steam only: the wide grid image, the banner on the game's page, and the logo on it |

Search for the game, pick the matching entry, then pick an image for each kind. Images are downloaded
once and kept locally. **Remove** clears one kind again. Prefixr remembers the matched game, so the
next time the picker opens straight on its images.

## Steam integration

**Add to Steam** adds the game to Steam as a non-Steam game, with its name, icon and all chosen
artwork. It then appears in your Steam library, in Big Picture and in the Steam Deck's Game Mode.
**Update in Steam** writes changes (a new name, new artwork) to the same entry without touching what
you changed in Steam itself, such as collections or your own launch options. **Remove from Steam**
takes it out again, and removing a game from Prefixr removes it from Steam as well.

When Steam starts the game, it runs Prefixr with `--run <game-id>`: the game launches without
Prefixr's window, and that Prefixr exits with the game, so Steam shows the game as running exactly as
long as it is. This works even while Prefixr is open, and the open window won't start the game a
second time.

Steam has to be closed while its shortcuts are changed, because a running Steam would overwrite
them. If it's running, Prefixr asks whether to quit it; Steam is started again afterwards. Prefixr
keeps the previous `shortcuts.vdf` as `shortcuts.vdf.bak`.

Prefixr works with the native Steam package. The Flatpak version of Steam can't start programs
outside its sandbox, so it can't launch games through Prefixr.

## Shortcuts

**To desktop** puts a launcher for the game on your desktop; **To start menu** adds it to your
desktop environment's application menu. Both start the game directly (through
`prefixr --launch <game-id>`), opening Prefixr if it isn't running yet. They use the game's
SteamGridDB icon if it has one, otherwise the icon from its `.exe`. Creating a shortcut again
replaces the old one, and removing the game removes its shortcuts.

## The tray icon

While Prefixr runs, its tray icon offers:

- **Show window** / **Hide window**
- **Quit "*game*" (force)** for every running game, to end one that hangs
- **Quit**, which asks first if games are still running, since quitting ends them

Closing the window only hides it to the tray, so running games stay under Prefixr's control. On a
desktop without a tray (GNOME without the AppIndicator extension), closing quits Prefixr instead,
except while a game runs, when the window is minimized.

## Logs

Every game launch writes a log of everything the game and Wine/Proton print. Prefixr keeps the last
10 per game. Setup programs, winetricks and Wine tools write logs per prefix, also the last 10. Open
the latest with **Show log** after an error. All logs are plain text files; see
[Configuration & files](configuration.md#data) for where they are.

## Language

Prefixr is available in English and German. Switch with **DE** / **EN** at the bottom of the
sidebar; the tray menu follows. On first start, Prefixr uses your system language.
