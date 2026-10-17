# Command line & AI agents

Everything you set up in Prefixr's window can also be done from a terminal: add and edit games,
create prefixes, install winetricks packages, download runners, pick artwork and launch games. That
makes Prefixr scriptable, and it lets an AI coding agent such as
[Claude Code](https://claude.com/claude-code) set up your games for you.

- [How it works](#how-it-works)
- [Games](#games)
- [Prefixes](#prefixes)
- [Runners and umu-launcher](#runners-and-umu-launcher)
- [JSON output and exit codes](#json-output-and-exit-codes)
- [Setting up games with an AI agent](#setting-up-games-with-an-ai-agent)

## How it works

Call Prefixr with a command instead of no arguments:

```bash
prefixr game list
prefixr prefix list
prefixr runner list
prefixr --help            # every command
prefixr game add --help   # one command's options
```

With the AppImage, that's `./prefixr_*.AppImage game list`.

The commands do exactly what the matching buttons do: the same checks, the same prefix setup, the
same launch. They work while Prefixr's window is open, too. The window notices the change within a
second and shows it, so you can watch a script fill your library.

Prefixr runs without a window for these commands, but it still needs your desktop session (as for
any launch), so run them in a terminal on your desktop rather than over SSH.

A game is named by its ID (`prefixr game list` shows the first 8 characters, which are enough), by
its name, or by a part of its name that only one game has: `prefixr game launch quake`.

## Games

| Command | Does |
| --- | --- |
| `game list` | Lists the games with ID, runner, UMU ID and prefix |
| `game show <game>` | Shows all of a game's settings |
| `game add --exe <exe> --prefix <dir> --runner <id> [settings]` | Adds a game. A prefix Prefixr doesn't know yet is added (and created) first. Without `--name`, the name comes from the `.exe` |
| `game edit <game> [--name] [--exe] [--prefix] [--runner] [--clear-env] [settings]` | Changes a game. Whatever you don't pass stays as it is |
| `game remove <game>` | Removes the game from the library, like **Remove** in the window. Prefix and game files stay |
| `game launch <game>` | Starts the game and waits until it exits; prints the log's path |
| `game stop <game>` | Ends the game's whole session, also one the window or Steam started |
| `game log <game> [--print]` | Prints the path of the latest log, or with `--print` the log itself |
| `game artwork <game> [--query] [--steamgriddb-id] [--kinds cover,icon,wide,hero,logo]` | Picks the best-rated SteamGridDB images (needs the API key) |
| `game shortcut <game> [--place menu\|desktop]` | Creates a start menu or desktop shortcut |
| `game steam-add <game> [--restart-steam]` / `game steam-remove <game>` | Adds the game to Steam, or removes it |

The settings `game add` and `game edit` take:

| Option | Sets |
| --- | --- |
| `--env KEY=VALUE` | An environment variable (repeatable). `--unset-env KEY` removes one |
| `--args "…"` | The launch arguments, quoted as on a Windows command line |
| `--notes "…"` | The game's notes, shown on its card; `""` removes them |
| `--umu-id <id>` | The [protonfixes match](user-guide.md#proton). A Steam app ID such as `377160` becomes `umu-377160`; `""` removes it. `--umu-store gog` sets the store |
| `--proton NAME=on\|off\|default` | A Proton switch for this game (repeatable), for example `--proton PROTON_USE_WINED3D=on` |
| `--gamemode`, `--power-profile`, `--inhibit-sleep`, `--mangohud` | `on`, `off`, or `default` to follow the global setting again |

For example, Fallout 4 with Proton-GE, its protonfixes, and the frame rate cap its physics need:

```bash
prefixr game add --name "Fallout 4" \
  --exe "/games/Fallout 4/Fallout4.exe" \
  --prefix ~/Games/prefixes/fallout4 \
  --runner GE-Proton10-30 \
  --umu-id 377160 \
  --env DXVK_FRAME_RATE=58
```

## Prefixes

| Command | Does |
| --- | --- |
| `prefix list` | Lists the prefixes, whether they're set up yet, and their games |
| `prefix add <dir>` | Adds an empty or new folder, or an existing prefix (also a Steam `compatdata` folder) |
| `prefix remove <dir>` | Takes a prefix off the list; the folder stays |
| `prefix winetricks <dir> [--runner <id>] <package>…` | Installs [winetricks](user-guide.md#installing-dependencies-winetricks) packages. Without packages, lists the installed ones |
| `prefix tool <dir> [--runner <id>] <tool>` | Opens `winecfg`, `regedit`, `cmd`, `winefile`, `taskmgr` or `uninstaller` |
| `prefix run <dir> [--runner <id>] <program> [args…]` | Runs a program in the prefix and waits for it: an installer, or one of Wine's builtins such as `regedit /S settings.reg` |

Without `--runner`, a prefix command uses the runner all of the prefix's games use.

## Runners and umu-launcher

| Command | Does |
| --- | --- |
| `runner list` | Lists the installed runners and how many games use each |
| `runner sources` | Lists where runners can be downloaded from (`proton-ge`, …) |
| `runner releases <source>` | Lists a source's recent releases |
| `runner download <source> [<tag>]` | Downloads and verifies a release, the latest without a tag |
| `runner remove <id>` | Deletes a runner no game uses |
| `umu status` / `umu install` | Shows or installs/updates umu-launcher |
| `umu search <name>` | Finds the UMU ID (protonfixes match) for a game |

## JSON output and exit codes

Add `--json` to any command to get its result as JSON on stdout, for example
`prefixr game list --json`. Messages for people (progress, notes) always go to stderr, so they
never mix into it.

A command exits with `0` on success, `1` when it fails (the error is printed to stderr), and `2`
when its arguments are wrong. `game launch` exits with `1` when the game exits with an error, and
`prefix run` when the program does.

## Setting up games with an AI agent

The command line is made to be driven by an AI coding agent like
[Claude Code](https://claude.com/claude-code): it can look at a game's files, find out what the game
needs, and set it up in Prefixr while you test. Some things you can ask for:

- *"Add every game under `~/Games` to Prefixr, with Proton-GE, one prefix per game, the right
  protonfixes match and artwork."*
- *"These games come with their own Wine and start scripts. Move each game out of its bundled
  prefix, add it to Prefixr with Proton, and carry over the fixes from the scripts and my save
  games."*
- *"Fallout 4 crashes on start. Read its latest Prefixr log and fix it."*

A good way to work with an agent:

1. **One game at a time.** The agent adds a game; you start it from the window and say whether it
   works. The window shows the changes right away.
2. **Let it read the logs.** `prefixr game log <game> --print` shows the agent what Wine and Proton
   printed, which is usually where the fix is.
3. **Protonfixes first.** With the right `--umu-id` (`prefixr umu search <name>`), umu applies the
   fixes known for that game automatically. Many games then need nothing else.
4. **Carry over what other launchers set.** Launch scripts and other launchers often set
   environment variables (`DXVK_FRAME_RATE`, `WINEDLLOVERRIDES`, …), winetricks packages or
   registry keys. Most generic ones (DXVK, d3dx9, esync and fsync) are built into Proton already; the
   game-specific ones become `--env`, `prefix winetricks` and `prefix run … regedit /S file.reg`.
5. **Write down what you found.** `--notes` keeps it with the game: an offline login, why an
   override is set, which config file holds the resolution. The next person (or agent) sees it on
   the card.
6. **Save games live in the prefix.** When a game moves to a new prefix, have the agent copy its
   saves from the old prefix's `drive_c/users/…` (`Documents`, `Saved Games`, `AppData`) into the
   new one's `drive_c/users/steamuser/…`.

Moving and copying files is up to the agent's own tools; Prefixr's command line takes over from
there. Nothing an agent does through it deletes game files or prefixes: removing a game or a prefix
only takes it off Prefixr's lists.
