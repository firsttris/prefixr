# Getting started

This guide takes you from a fresh install to a running game in a few minutes.

- [1. Install Prefixr](#1-install-prefixr)
- [2. Get a runner](#2-get-a-runner)
- [3. Create a prefix](#3-create-a-prefix)
- [4. Add your game](#4-add-your-game)
- [5. Start it](#5-start-it)
- [Installing a game from its setup program](#installing-a-game-from-its-setup-program)
- [Optional extras](#optional-extras)
- [Next steps](#next-steps)

## Key terms

| Term | Meaning |
| --- | --- |
| **Prefix** | A folder that holds a virtual Windows installation: its own `C:` drive, registry and installed programs. Each game runs inside one. Several games can share a prefix, or each can get its own. |
| **Runner** | The compatibility layer that runs Windows programs. Either **Proton** (Valve's Wine build used by Steam, and its community variants such as Proton-GE) or a plain **Wine** build. |
| **umu-launcher** | The tool Prefixr launches Proton through. It runs Proton inside the Steam Linux Runtime, just as Steam does, and applies game-specific fixes (protonfixes). |

## 1. Install Prefixr

Download a package from the [latest release](https://github.com/firsttris/prefixr/releases/latest):

- **AppImage** (works on any distribution, including immutable ones like Bazzite and SteamOS):

  ```bash
  chmod +x prefixr_*.AppImage
  ./prefixr_*.AppImage
  ```

- **Debian, Ubuntu, Linux Mint, Pop!_OS**:

  ```bash
  sudo apt install ./prefixr_*_amd64.deb
  ```

- **Fedora, openSUSE, Nobara**:

  ```bash
  sudo dnf install ./prefixr-*.x86_64.rpm
  ```

Start Prefixr as your normal user, not with `sudo`. A prefix created as root would be owned by
root, and your desktop session couldn't use it afterwards, so Prefixr refuses to start as root.

## 2. Get a runner

Open **Runners** in the sidebar and click **Download runner**. Pick a source and a version:

| Source | Kind | Good for |
| --- | --- | --- |
| **Proton-GE** | Proton | The usual choice. It's Valve's Proton plus extra fixes and media codecs. |
| **Proton-CachyOS** | Proton | An alternative Proton build with CachyOS's patches. |
| **Wine (Kron4ek)** | Wine | Plain Wine (staging, WoW64), for games or apps that run better outside Proton. |

If you're not sure, take the newest **Proton-GE**. Prefixr verifies each download against the
checksum the release publishes.

Prefixr picks up any Proton or Wine build placed in its runners folder, so you can also copy or
symlink a build there, for example from Steam's `compatibilitytools.d`. See
[Configuration & files](configuration.md#data) for where that folder is.

## 3. Create a prefix

Open **Prefixes**, choose an empty folder (or type a path that doesn't exist yet) and click
**Add prefix**. The folder is set up the first time a game starts in it.

You can also add a prefix that already exists, for example one from Lutris, Bottles, PortProton,
Heroic, or a game's `compatdata/<appid>` folder from Steam. Prefixr uses it as it is; for Steam's
compatdata folders it picks the `pfx` folder inside automatically.

## 4. Add your game

In the **Library**, click **+ Add game** and fill in the **General** tab:

1. **Program (.exe)**: choose the game's executable. The name is filled in from the file name.
2. **Prefix**: the prefix from step 3.
3. **Runner**: the runner from step 2.

With a Proton runner, the **Proton** tab offers a **protonfixes match**. Search for your game there
so umu applies the fixes known for it. Without a match, only general fixes apply.

Click **Add**.

## 5. Start it

Click the play button on the game's card (or **▶ Start** in the list view). The first launch of
a new prefix takes a while: Prefixr downloads umu-launcher and the Steam Linux Runtime (a few
hundred MB, once) for Proton, or initializes the prefix and installs wine-mono and DXVK for Wine.
The card shows **Starting…** in the meantime and **Running** once the game is up.

If it fails, the card shows the error and a **Show log** button. The
[troubleshooting guide](troubleshooting.md) covers the common cases.

## Installing a game from its setup program

Many games come as an installer (`setup.exe`) rather than a ready-to-run folder:

1. In your file manager, right-click the installer → **Open With** → **Install Prefixr**. Prefixr
   registers this entry the first time it starts.
2. Choose a prefix (or create a new one with **+**) and a runner, then click **Run setup** and go
   through the installer as on Windows.
3. When the installer closes, Prefixr looks for the shortcuts it created and offers them as the
   game to add. Pick the right one (or choose the `.exe` yourself) and add the game.

## Optional extras

These make the matching settings work. When a tool isn't installed, Prefixr leaves its setting out
at launch instead of failing the game.

| Tool | Enables |
| --- | --- |
| [MangoHud](https://github.com/flightlessmango/MangoHud) | The in-game overlay with FPS, frame times and temperatures |
| [GameMode](https://github.com/FeralInteractive/gamemode) | CPU governor and priority tweaks while a game runs |
| power-profiles-daemon | Holding the *Performance* power profile while a game runs |
| [Gamescope](https://github.com/ValveSoftware/gamescope) | Running a game in its own compositor with a fixed resolution and FPS limit |
| [vkBasalt](https://github.com/DadSchoorse/vkBasalt) | Sharpening and anti-aliasing as post-processing |
| Steam | Importing Steam prefixes and adding games to Steam |
| A [SteamGridDB](https://www.steamgriddb.com/) API key | Cover art and other artwork |

Most gaming-focused distributions (Bazzite, Nobara, CachyOS, SteamOS) ship most of these already.

## Next steps

- Make games look good in your library with [artwork from SteamGridDB](user-guide.md#artwork).
- Tune performance and visuals [globally or per game](user-guide.md#game-settings).
- [Add your games to Steam](user-guide.md#steam-integration) to launch them from Big Picture or
  the Steam Deck's Game Mode.
