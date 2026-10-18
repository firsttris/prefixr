# User guide

A tour of everything Prefixr does. New to Prefixr? Start with [Getting started](getting-started.md).

- [The library](#the-library)
- [Adding and editing games](#adding-and-editing-games)
- [Installing a game from its setup program](#installing-a-game-from-its-setup-program)
- [Stores (Epic Games, GOG, Amazon)](#stores-epic-games-gog-amazon)
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
- [Backup and restore](#backup-and-restore)
- [Language](#language)

## The library

The **Library** shows your games as cover cards (grid view) or as a compact list (list view);
switch with the two buttons at the right of the toolbar, and set the size of the cards with the
slider next to them. Search filters by name (`/` jumps to it). You can sort by name, by when you last
played, by when a game was added, by play time, or by runner. Prefixr remembers the view, the sort,
the filters and the card size.

The library has up to three sections: **Continue playing** (the games you played last), **Installed**
and **In your stores** (the stores' games you haven't installed yet). A click on a section's title
folds it away.

### Favorites, categories and hidden games

The chips above the games pick what you see: **All**, **Favorites**, each of your categories, the
test results (once a game has one), and **Hidden** (once you've hidden a game). The numbers say how many games each holds.

- **Favorites**: the star on a card or row marks a game as a favorite.
- **Categories**: **+ Category** creates one, such as *Kinder* or *Koop*; the gear renames and
  deletes them. Add a game to categories in its details. An AI agent can sort a whole library into
  categories, see [Command line](cli.md#library-categories-and-favorites).
- **Hidden**: hide a game in its details, say a store's giveaway you won't play. It then only shows
  under **Hidden**.
- **Test status**: a small symbol at the top left of a cover says how the game ran when it was
  tested: ✓ works, ! works with issues, ✕ broken. Hover it for the note and the date. Set it in
  the card's ⋮ menu under **Test status**, or let an AI agent test your games and set it (see
  [Testing games on screen](cli.md#testing-games-on-screen)). Once the game's runner or program
  changes, the symbol fades: the result may no longer hold. Chips such as **✓ Works** show only
  the games with that result.

### Game details

A click on a game's cover (or a right-click on a card or row) opens its details at the window's
right edge: play time, when it was last played and added, its size (for a store game), runner,
prefix (with **Show in prefixes**, see [The prefixes page](#the-prefixes-page)) and program, its
categories, and how well it runs:

- **ProtonDB**: the game's rating on [ProtonDB](https://www.protondb.com), from Platinum to Borked,
  for a game whose UMU ID is a Steam app ID.
- **umu fix**: umu-protonfixes has fixes for the game and applies them at launch.

Both also show as small badges on the cards, so you can see before installing a store game whether
it's likely to run. Play time is counted for every launch, also from Steam.

### Downloads

**Downloads** in the sidebar lists the store installs that are running, with their speed and the
time left, and the ones that failed, to try again. The number next to it counts both.

Each game shows its SteamGridDB cover if you picked one, otherwise the icon from its `.exe`.

With a store connected (see [Stores](#stores-epic-games-gog-amazon)), the library also shows that store's
games you haven't installed yet, with an **Install** button instead of the play button, in the
**In your stores** section. Their covers are muted until you point at them. Chips above the library
then filter by source: all, your own, or one store. Store games carry the store's name on their
card.

| State | What you see |
| --- | --- |
| **Starting…** | The launch is being prepared. On first use this includes downloads and setting up the prefix, which can take a few minutes. |
| **Running** | The game's process runs. The play button turns into a stop button. |
| **Error** | The launch failed or the game exited with an error. The message and a **Show log** button appear below the game. |

**Stop** ends the game's whole session: its process tree, the umu container for a Proton game, and
the Wine session for a Wine game. Use it when a game hangs. It is the same as closing a game the
hard way, so unsaved progress is lost.

The **⋮** menu on a card or in a list row (where editing, artwork and removing also have buttons of their own) has:

- **Show notes**: only for a game with notes; see [Adding and editing games](#adding-and-editing-games).
- **Select artwork**: see [Artwork](#artwork).
- **Edit game**: the same dialog as when adding it.
- **To desktop** / **To start menu**: see [Shortcuts](#shortcuts).
- **Add to Steam** / **Update in Steam** / **Remove from Steam**: see
  [Steam integration](#steam-integration).
- **Remove**: removes the game from the library together with its logs, artwork, shortcuts and
  settings files. The prefix and the game's files stay. A running game can't be removed.
- **Uninstall** (instead of **Remove**, for a game installed from a store): deletes the game's
  files as well. It stays in your store library and can be installed again.

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
| **Notes** (own tab) | Anything worth remembering about the game: an offline login, why a setting is needed, where its config file lives. **Show notes** in the game's **⋮** menu opens them to read in full, copy from, or go on to edit. A line ending in `:` shows as a heading, lines starting with `- ` as a list. They're stored as plain text in `config.json`, so keep real passwords out. |

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

## Stores (Epic Games, GOG, Amazon)

Prefixr can install and start the games of your **Epic Games Store**, **GOG** and **Amazon Games**
(Prime Gaming) accounts. They appear in the library next to the games you added yourself and run
with the same prefixes, runners and settings.

Prefixr talks to each store through the command line tool the
[Heroic Games Launcher](https://heroicgameslauncher.com/) uses for it:
[Legendary](https://github.com/Heroic-Games-Launcher/legendary) for Epic and
[gogdl](https://github.com/Heroic-Games-Launcher/heroic-gogdl) for GOG and
[Nile](https://github.com/imLinguin/nile) for Amazon. A tool is downloaded when you connect its
store, at a version pinned in Prefixr and checked against its SHA-256, and deleted again when you
disconnect. Like umu-launcher, Legendary and gogdl need **Python 3**, which almost every
distribution has installed; Nile brings its own.

### Connecting your account

1. Open **Stores** in the sidebar and click **Connect** next to the store.
2. Your browser opens the store's login page. Sign in.
3. Bring the login back to Prefixr and click **Sign in**:
   - **Epic** then shows a short page of text. Copy all of it (or only the value of
     `authorizationCode`).
   - **GOG** then shows an almost empty page whose address contains `on_login_success?…code=…`.
     Copy the whole address from the address bar.
   - **Amazon** then shows its homepage, with an address containing
     `openid.oa2.authorization_code=…`. Copy the whole address from the address bar.

   The code works once and only for a few minutes; if it's refused, open the login page again for
   a new one.

Prefixr then loads your game list and the covers. **Refresh library** loads it again, for example
after you bought a game. **Disconnect** signs you out; the games you installed stay installed.

### Installing a game

Click **Install** on a game in the library. The dialog shows the download and installed size and
has everything filled in:

| Field | Default |
| --- | --- |
| **Install folder** | `~/Games/Epic`, `~/Games/GOG` or `~/Games/Amazon` on a store's first install, after that the folder you used last for that store. The game gets a folder of its own in it. |
| **Prefix** | A new prefix for this game alone (recommended), under Prefixr's data folder. Any of your prefixes works too. |
| **Runner** | Your newest Proton runner. |

The download runs in the background; the card shows its progress and has a **Cancel** button.
What was downloaded is kept, so installing again continues where it stopped. Games of one store
install one after another. If an install fails, the card shows why, with the install's log.

Once installed, the game is an ordinary library game: you can change its prefix, runner,
settings and artwork in the game dialog. Its cover comes from the store; pick another one under
**Select artwork** if you like.

### Starting a store game

An **Epic** game's start asks Legendary for its command line first, because online games need
fresh login tokens on it. Without a connection, a game that can run offline still starts. A game's
**launch arguments** go between the game's own arguments and the account's, as in Epic's launcher.

A **GOG** game needs no login to start: Prefixr starts the program the game's own
`goggame-<id>.info` names as its main one, with its arguments and working folder, just as GOG
Galaxy does. Games are installed in English; most let you switch the language in their settings.

An **Amazon** game starts with what its `fuel.json` names, in the working folder it asks for. Nile
adds the Amazon Games SDK's environment (the entitlement and product the game checks), so Prime
Gaming games that ask for it start without the Amazon Games app.

On its first start, a game installs what it lists as prerequisites (Visual C++ runtimes, DirectX
and the like) into its prefix. For GOG these are GOG's shared redistributables, downloaded once
into Prefixr's data folder for all GOG games; for Amazon, the installers its `fuel.json` lists
under `PostInstall`. That runs once per game; the launch log shows what
happened.

**Uninstall** removes an Epic game through Legendary, an Amazon game through Nile (the files it
installed), and a GOG game's folder (only once it's certain to be the game's: its
`goggame-<id>.info` is in it).

Not supported yet: updating installed games, cloud saves (use your own backup), DLC picked one by
one (the DLC you own is installed with the game), GOG's install scripts for a few older games and
Galaxy features such as achievements, choosing a game's language, and importing games installed by
another launcher.

## Prefixes

A prefix is a self-contained Windows environment: its own `C:` drive, registry and installed
programs, separate from your Linux system and from other prefixes.

### The prefixes page

The **Prefixes** page lists every prefix by the games that use it. A click on a game's name opens
its settings, and a prefix no game uses says *Unused*. Prefixes that sit in the same folder (say
`/mnt/games/Prefixr/<game>/prefix` for each game) are grouped under that folder, shown once as a
heading, so each row only shows the part of the path that sets it apart. Hover over it for the
full path.

- **Search** finds a prefix by a game's name or by its path.
- **Filters** show only the prefixes that are *Unused*, *Shared by several games*, *Not set up
  yet* (Wine sets a prefix up the first time a game in it starts), *Folder missing* (nothing at
  its path, for example on a drive that isn't mounted) or *Not readable* (the folder can't be read,
  or didn't answer within a few seconds, like a network share whose server is gone). Each one
  shows how many prefixes it matches, and the rarer ones only appear when there are any.
- **Sort** by game, by path or by the number of games using each prefix. The folder headings
  follow the sort too.
- Each row has **Dependencies** (winetricks, see below) and **Wine tools** buttons. Its **⋮** menu
  has **Show folder**, which opens the folder in your file manager, and **Remove**.

**Show in prefixes** in a game's details opens this page at the game's prefix. If the prefix was
removed from the list, the page says so and offers to **Add it again**.

### Adding a prefix

Click **+ Add prefix**, choose a folder and click **Add prefix**:

- **A new or empty folder** becomes a new prefix. Nothing is written until the first game or tool
  runs in it.
- **An existing prefix** (a folder with a `drive_c` inside) is used as it is. That works for
  prefixes from Lutris, Bottles, PortProton, Heroic or plain Wine.
- **A Steam `compatdata/<appid>` folder** works too: Prefixr uses the `pfx` folder inside it.

A folder that already contains other files is refused, so a prefix never ends up mixed into
unrelated data.

In the game dialog and the install dialogs, the prefix list names each prefix after its games, in
alphabetical order, so typing a game's name in the open list jumps to its prefix. Unused prefixes
come first, under their own heading. The chosen prefix's full path is shown below the list.

### Installing dependencies (winetricks)

**Dependencies** opens the winetricks dialog for a prefix. The common packages are
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

**Wine tools** starts one of Wine's own programs in the prefix:

| Tool | Use it for |
| --- | --- |
| Wine configuration (`winecfg`) | Windows version, DLL overrides, graphics and drive settings |
| Registry editor (`regedit`) | Editing the prefix's registry directly |
| Command prompt (`cmd`) | A Windows command line in the prefix |
| File explorer (`winefile`) | Browsing the virtual `C:` drive |
| Task manager (`taskmgr`) | Seeing and ending Windows processes in the prefix |
| Uninstall programs (`uninstaller`) | Removing programs installed in the prefix |

### Removing a prefix

**Remove** (in a prefix's **⋮** menu) only takes a prefix off Prefixr's list. The folder, its programs and save games stay on
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

## Backup and restore

**Backup** in the sidebar keeps your library safe: the games, the prefix list, every setting, your
categories, favorites and play times, and the covers and icons, in one `.tar.gz` file. The prefixes
and the games' own files stay where they are and aren't in it (save games live in the prefixes, so
back those up separately), nor are your store logins.

- **Create backup…** saves one to a file you pick, to keep or to take to another computer. The
  SteamGridDB API key and the GitHub token only go in when you tick the box.
- **Restore from file…** replaces the library and all settings with a backup's. Prefixr shows what's
  in it first and asks.
- **Automatic backups**: Prefixr backs up by itself once a day when it starts (when something changed
  since the last one), and right before every restore. The last 7 are listed with **Restore**, so a
  restore, or a change an agent made, can always be undone. Each takes about as much space as your
  covers and icons together.

A restore keeps your API keys when the backup has none, and this computer's runners folder. On
another computer, the prefixes and
games have to be at the same paths for the games to start, and the stores need connecting again.

## Language

Prefixr is available in English and German. Switch with **DE** / **EN** at the bottom of the
sidebar; the tray menu follows. On first start, Prefixr uses your system language.

Below the language switch, the sidebar shows Prefixr's version. Click it to see what changed in that
version on GitHub.
