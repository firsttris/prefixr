# Configuration & files

Everything Prefixr stores, the command-line options it understands, and what a game launch sets up.
Prefixr's app ID is `com.tristan.prefixr`, so its folders follow the XDG base directories under that
name.

- [Configuration file](#configuration-file)
- [Data](#data)
- [Elsewhere on your system](#elsewhere-on-your-system)
- [Command-line options](#command-line-options)
- [API keys](#api-keys)
- [What a launch sets up](#what-a-launch-sets-up)
- [Backing up and resetting](#backing-up-and-resetting)

## Configuration file

`~/.config/com.tristan.prefixr/config.json` holds the library and all settings: the games, the prefix
list, the global Performance, Graphics, Overlay and Proton settings, the SteamGridDB API key and the
GitHub token.

Prefixr saves it atomically (written to a temporary file, then renamed), so a crash or a full disk
never leaves half a file. If the file can't be read when Prefixr starts, Prefixr shows the error and
quits **without touching it**, rather than starting empty and overwriting your library on the next
save. Fix the JSON or move the file away to start fresh.

The runners folder is the one setting without a UI: `runners_dir` in `config.json`. Quit Prefixr
before editing the file.

## Data

`~/.local/share/com.tristan.prefixr/` holds everything Prefixr downloads or generates:

| Path | Contents |
| --- | --- |
| `runners/` | Downloaded runners, one folder each. Any Proton or Wine build you put or symlink here is listed too. |
| `umu-launcher/` | Prefixr's own copy of umu-launcher, with its version. |
| `directx-layers/` | DXVK and VKD3D-Proton for Wine runners, with their versions, and the cached wine-mono installers. |
| `winetricks/` | The winetricks script, refreshed weekly. |
| `artwork/` | Images chosen from SteamGridDB, named after the game's ID. |
| `exe-icons/` | Icons extracted from the games' `.exe` files. |
| `mangohud/`, `vkbasalt/` | The MangoHud and vkBasalt config of each game, written on every launch. |
| `logs/<game-id>/` | The last 10 launch logs of each game. |
| `logs/prefixes/<prefix>/` | The last 10 logs of setup programs, winetricks and Wine tools run in a prefix. |
| `shortcut-icons/` | The icon of the *Install Prefixr* file-manager entry. |
| `WebKitCache/`, `storage/`, `localstorage/` | The window's own browser data (see [Troubleshooting](troubleshooting.md#clearing-the-webkit-cache)). |

`~/.cache/com.tristan.prefixr/umu-database.json` caches the umu database for the protonfixes search
for a day.

Your prefixes live wherever you created or found them; Prefixr only remembers their paths.

## Elsewhere on your system

| Path | Written when |
| --- | --- |
| `~/.local/share/umu/` | umu-launcher downloads the Steam Linux Runtime there, shared with other umu-based launchers. Follows `UMU_FOLDERS_PATH` and `XDG_DATA_HOME` as umu does. |
| `~/.local/share/applications/com.tristan.prefixr.install.desktop` | On every start, to register *Install Prefixr* in your file manager's *Open With* menu for `.exe` files. Hidden from the app menu. |
| `~/.local/share/applications/<game>-<id>.desktop` | **To start menu** on a game. |
| Your desktop folder | **To desktop** on a game. Prefixr honors a localized `XDG_DESKTOP_DIR`, such as `~/Schreibtisch`. |
| `~/.steam/root/userdata/<user>/config/shortcuts.vdf` and `grid/` | **Add to Steam**, **Update in Steam** and **Remove from Steam**. The previous file is kept as `shortcuts.vdf.bak`. |
| `$XDG_RUNTIME_DIR/prefixr/<game-id>.lock` | While a game launches or runs, so two Prefixr processes (an open window and one Steam started) never start the same game twice. |
| `/etc/sysctl.d/99-prefixr-max-map-count.conf` | Only when you click **Increase now** for `vm.max_map_count`, after a password prompt. |

## Command-line options

| Option | Effect |
| --- | --- |
| `prefixr --launch <game-id>` | Starts the game. If Prefixr is already open, that window starts it instead. Desktop and menu shortcuts use this. |
| `prefixr --run <game-id>` | Starts the game without any window and exits when the game does. A failure before the game starts is shown in a dialog. Steam uses this for games added to it. |
| `prefixr --install <path/to/setup.exe>` | Opens the install dialog for that setup program. The *Install Prefixr* file-manager entry uses this. |

A game's ID is the `id` of its entry in `config.json`. It also appears in the `Exec=` line of the
game's shortcuts.

## API keys

Both are optional and stored in `config.json`.

| Key | Where to enter it | What it's for |
| --- | --- | --- |
| [SteamGridDB API key](https://www.steamgriddb.com/profile/preferences/api) | **SteamGridDB** page | Searching artwork, and finding a game's Steam app ID for the protonfixes match |
| [GitHub personal access token](https://github.com/settings/tokens), no scopes | **Runners** page, *GitHub* section | Raising GitHub's API limit from 60 to 5000 requests per hour for runner, umu and DXVK downloads |

## What a launch sets up

For reference when reading a game's log or reproducing a launch by hand.

**The command**, from the outside in, each part only when it's enabled and available:

1. `gamescope [-w W] [-h H] [-r FPS] [-f] [--mangoapp] --`
2. `systemd-inhibit --what=idle:sleep --`, for *Prevent sleep*
3. `powerprofilesctl launch --profile performance`, for the performance power profile
4. `umu-run` for a Proton runner, or the runner's `wine` for a Wine runner
5. the game's `.exe` and its launch arguments

**The environment:**

| Variable | Set to |
| --- | --- |
| `WINEPREFIX` | The game's prefix |
| `WINEDLLOVERRIDES` | Always `winemenubuilder.exe=`, so Wine doesn't add menu entries and file associations for what runs in the prefix. For a Wine runner also the DXVK and VKD3D-Proton DLLs as native-then-builtin. |
| `PROTONPATH` | The Proton runner (Proton only) |
| `GAMEID`, `STORE` | The game's protonfixes match (Proton only) |
| `PROTON_*` | The Proton switches set to On (`1`) or Off (`0`) |
| `WINEDEBUG` | `-all` for a Wine runner, which keeps Wine's debug output from slowing games down |
| `MANGOHUD`, `MANGOHUD_CONFIGFILE` | With the overlay enabled |
| `ENABLE_VKBASALT`, `VKBASALT_CONFIG_FILE` | With vkBasalt enabled |
| `LD_PRELOAD` | `libgamemodeauto.so.0` with GameMode. An `LD_PRELOAD` Prefixr itself was started with (such as Steam's overlay) is kept. |

With Gamescope, the MangoHud and vkBasalt variables only go to the game inside it, so they don't
apply to Gamescope's own output a second time.

The game's own environment variables come last and win, except `WINEDLLOVERRIDES` and `LD_PRELOAD`,
which are appended to.

Before a Proton game's first launch, Prefixr creates the prefix with umu's `createprefix` so the
download of the Steam Runtime shows as *Starting…*. Before a Wine game's first launch, it runs
`wineboot`, installs the wine-mono version that Wine build expects, and links in DXVK and
VKD3D-Proton.

## Backing up and resetting

- **To back up your library**, copy `~/.config/com.tristan.prefixr/config.json`. With
  `~/.local/share/com.tristan.prefixr/artwork/`, your chosen artwork comes along too. Save games live
  in the prefixes, so back those up separately.
- **To start over**, quit Prefixr and remove `~/.config/com.tristan.prefixr/`. To free the space
  runners and caches take, also remove `~/.local/share/com.tristan.prefixr/`. Neither touches your
  prefixes or games.
