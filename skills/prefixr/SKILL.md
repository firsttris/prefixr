---
name: prefixr
description: Run the user's Linux game library through Prefixr's command line. Use for anything about Windows games on Linux with Prefixr - adding, installing (setup files, discs, Epic/GOG/Amazon), launching, fixing or testing games, Wine/Proton prefixes and runners, protonfixes (UMU ids), winetricks, artwork, Steam shortcuts, sorting the library into categories, and backups.
---

# Prefixr

Prefixr is a Linux launcher for Windows games. Everything its window does, its command line
`prefixr` does too, with the same checks and the same launch. The user usually keeps the window
open: it shows every change within a second, so they can watch and try a game right away.

## Ground rules

- **Use `--json`** for anything you read back: `prefixr game list --json`. Results go to stdout,
  progress and notes to stderr. Exit code `0` is success, `1` a failed command (the error is on
  stderr), `2` wrong arguments.
- **Find options with `--help`**: `prefixr --help`, `prefixr game add --help`. Don't guess flags.
- **Back up before a larger change** (many games, a restore, moving prefixes):
  `prefixr backup create`. `prefixr backup restore <file>` undoes it, and a restore is itself
  undoable with `prefixr backup restore 1`.
- **Run it on the user's desktop**, not over SSH: launches and installers need the desktop session.
  With the AppImage, call `./prefixr_*.AppImage` instead of `prefixr`; with the Flatpak,
  `flatpak run io.github.firsttris.Prefixr`.
- **Long commands run in the background**: `game launch` (waits until the game exits),
  `game install`, `store install`, `runner download`. Ctrl+C or your timeout cancels them cleanly.
- **Nothing here deletes game files or prefixes**, except `store uninstall` (deletes a store
  game's files, like Uninstall in the window). `game remove` and `prefix remove` only take things
  off Prefixr's lists.
- **Ask the user** for what only they can do: store logins (the code from the browser), games
  that need an online account or a controller, and whether a game really runs, unless you have a
  computer-use tool to look at the screen.

## Naming things

- A **game** in `game …` commands: its id (the first 8 characters `game list` shows are enough),
  its name, or a part of the name only one game has.
- A game in `library …` and `category …` commands: its **key** from `library list`: `epic:<app>`,
  `gog:<id>`, `amazon:<id>` for store games (installed or not), the id for games added by hand. A
  name works too.
- A **runner** by its id from `runner list` (for example `GE-Proton10-30`), a **prefix** by its
  folder.

## Look around first

```bash
prefixr game list --json
prefixr library list --json
prefixr prefix list --json
prefixr runner list --json
prefixr store list --json
```

## Setting a game up

1. Make sure there's a Proton runner: `prefixr runner list`, else
   `prefixr runner download proton-ge`.
2. Find its protonfixes match. With the right UMU id, umu applies the known fixes for the game,
   and many games need nothing else: `prefixr umu search "Fallout 4"`.
3. Add it, one prefix per game:

```bash
prefixr game add --name "Fallout 4" --exe "/games/Fallout 4/Fallout4.exe" --prefix ~/Games/prefixes/fallout4 --runner GE-Proton10-30 --umu-id 377160
```

4. Give it artwork (needs the user's SteamGridDB key): `prefixr game artwork "Fallout 4"`.
5. Write down what you found, for the next person or agent:
   `prefixr game edit "Fallout 4" --notes "Physics need DXVK_FRAME_RATE=58"`.

Work **one game at a time**: add it, have it tested, then the next.

### From a setup file or a disc

`game install` runs the installer in the prefix, waits for it, and adds the game its shortcut
points to. Disc images are mounted with `udisksctl`. Pass silent switches after `--`: Inno Setup
(GOG) `/VERYSILENT /SUPPRESSMSGBOXES /DIR="C:\Games\<name>"`, NSIS `/S`, MSI `/qn`.

```bash
prefixr game install ~/Downloads/setup_game_1.0.exe --prefix ~/Games/prefixes/game -- /VERYSILENT /SUPPRESSMSGBOXES
prefixr game install ~/ISOs/GAME_DISC1.iso --prefix ~/Games/prefixes/game
```

If the installer creates no shortcut or several, the command names them and stops: then use
`game add`, or pass `--exe` the next time.

### From a store

```bash
prefixr store login gog
prefixr store connect gog "<what the page shows>"
prefixr library list --json
prefixr store install gog:1207658924
```

The user signs in in the browser and gives you what the page shows (Epic: the page's text, GOG and
Amazon: the address from the address bar). The code works once, for a few minutes. Only one install
per store runs at a time.

## Fixing a game

1. Read the log: `prefixr game log <game> --print`. It's usually where the fix is.
2. Check the protonfixes match (`--umu-id`) before anything else.
3. Carry over what launch scripts or other launchers set. Generic tweaks (DXVK, d3dx9, esync,
   fsync) are built into Proton already; game-specific ones become:
   - environment variables: `prefixr game edit <game> --env WINEDLLOVERRIDES="dinput8=n,b"`
   - Proton switches: `prefixr game edit <game> --proton PROTON_USE_WINED3D=on`
   - winetricks packages: `prefixr prefix winetricks <prefix> vcrun2019`
   - registry keys: `prefixr prefix run <prefix> regedit /S fix.reg`
   - launch arguments: `prefixr game edit <game> --args "-dx11"`
4. Note the fix with `--notes`.

**Save games live in the prefix.** When a game moves to a new prefix, copy its saves from the old
prefix's `drive_c/users/…` (`Documents`, `Saved Games`, `AppData`) to the new one's
`drive_c/users/steamuser/…`.

## Testing games

`game launch` tells you whether the process started and how it ended; whether the game shows its
menu or a black window you can only see on screen. With a computer-use tool:

1. `prefixr game launch <game> &`, wait a little, take a screenshot and judge it.
2. `prefixr game stop <game>`, and read the log if it looked wrong.
3. Fix and try again.
4. Record the result:

```bash
prefixr library test-status "Fallout 4" works
prefixr library test-status "Hades" issues --note "Videos stay black"
prefixr library test-status "Old Game" broken --note "Crashes on start with VK_ERROR_DEVICE_LOST; tried WINED3D"
```

`prefixr library list --tested untested --json` lists the games still to test, including those
whose runner or exe changed since their last test.

Without a computer-use tool, launch the game and ask the user how it looked.

## Organizing the library

Read `prefixr library list --json` (each game's key, name, source, categories, play time), decide
per game, then:

```bash
prefixr category add Kids
prefixr category assign Kids epic:Fortnite gog:1207658924
prefixr library favorite "Fallout 4"
prefixr library hide amazon:amzn1.adg.product.1234
```

## Other commands

- `prefixr game shortcut <game> --place desktop`: a desktop or start menu shortcut.
- `prefixr game steam-add <game>`: add it to Steam as a non-Steam game.
- `prefixr prefix tool <prefix> winecfg`: also `regedit`, `cmd`, `winefile`, `taskmgr`,
  `uninstaller`.
- `prefixr umu install`: install or update umu-launcher.

The full reference is <https://firsttris.github.io/prefixr/cli.html>.
