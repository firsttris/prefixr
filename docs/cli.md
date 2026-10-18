# Command line & AI agents

Most of what you do in Prefixr's window can also be done from a terminal: add and edit games,
create prefixes, install winetricks packages, download runners, pick artwork, launch games, connect
store accounts and install their games, and sort the library into categories. That makes Prefixr scriptable, and it lets an AI coding agent
such as [Claude Code](https://claude.com/claude-code) run your library for you: you say what you
want, the agent does it, and you play.

- [How it works](#how-it-works)
- [Games](#games)
- [Library, categories and favorites](#library-categories-and-favorites)
- [Stores](#stores)
- [Prefixes](#prefixes)
- [Runners and umu-launcher](#runners-and-umu-launcher)
- [JSON output and exit codes](#json-output-and-exit-codes)
- [Working with an AI agent](#working-with-an-ai-agent)

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

## Library, categories and favorites

`game` commands work on the games in the library. The `library` and `category` commands cover the
whole library: also the games of your connected stores that aren't installed yet. They name a game
by its **key**, which `library list` shows: `epic:<app>`, `gog:<id>` or `amazon:<id>` for a store's
game, the game's ID for one you added by hand. Its name, or a part of it only one game has, works
too.

| Command | Does |
| --- | --- |
| `library list [--category <name>] [--favorites] [--hidden] [--compatibility]` | Lists every game with its key, source, whether it's installed, its categories and play time. Hidden games only with `--hidden`; `--compatibility` adds each game's ProtonDB rating and umu-protonfixes match |
| `library favorite <game>… [--off]` | Marks games as favorites, or unmarks them |
| `library hide <game>… [--off]` | Hides games from the library (they show up under **Hidden**), or shows them again |
| `category list` | Lists the categories and how many games each holds |
| `category add <name>` | Creates a category, e.g. `Kinder` |
| `category rename <category> <new name>` | Renames a category |
| `category remove <category>` | Deletes a category; its games stay in the library |
| `category assign <category> <game>…` | Puts games in a category; a game can be in several |
| `category unassign <category> <game>…` | Takes games out of a category |

A store game keeps its categories, favorite mark and play time when it's installed, uninstalled
and installed again.

Sorting a whole library into categories is a job for an agent: *"Create the categories Kinder,
Koop and Rennspiele, and sort my games into them."* It reads `prefixr library list --json`, decides
per game, and calls `prefixr category assign Kinder epic:Fortnite gog:1207658924 …`.

## Stores

| Command | Does |
| --- | --- |
| `store list` | Lists Epic Games, GOG and Amazon Games: connected or not, the account, how many games |
| `store login <store>` | Opens the store's login page in the browser (`epic`, `gog` or `amazon`) |
| `store connect <store> <code>` | Finishes the login with the code the page shows, then fetches the game list |
| `store disconnect <store>` | Signs out; games installed from the store stay installed |
| `store sync <store>` | Fetches the store's game list again, for example after buying a game |
| `store install <game> [--dir <dir>] [--prefix <dir>] [--runner <id>]` | Installs a store game and adds it to the library. Without options: the store's last install folder, a new prefix of its own and the newest Proton |
| `store uninstall <game>` | Deletes a store game's files and takes it out of the library; it stays in the store's list |

The games are named as in `library list`: by key (`epic:<app>`, `gog:<id>`, …) or name. All of a
store's games, installed or not, show up there once it's connected.

Logging in still needs you: after `store login`, sign in in the browser and pass on what the page
shows, as in the window (see [Connecting your account](user-guide.md#connecting-your-account)): for
Epic the page's text, for GOG and Amazon the address from the address bar. `store connect` takes it
as it is. The code works once and only for a few minutes.

`store install` prints its progress on stderr and returns the new game (with `--json`, as JSON). An
open window shows the install on its **Downloads** page as well.

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

## Working with an AI agent

The command line is made to be driven by an AI coding agent like
[Claude Code](https://claude.com/claude-code). Start the agent in a terminal on your desktop, tell
it to use `prefixr` (it finds every command with `prefixr --help`), and keep Prefixr's window
open: it shows each change within a second, so you see what the agent does and can try a game
right away.

What you can hand over:

| Task | For example |
| --- | --- |
| **Set games up** | *"Add every game under `~/Games` to Prefixr, with Proton-GE, one prefix per game, the right protonfixes match and artwork."* |
| **Install from your stores** | *"Connect my GOG account and install all my GOG games that are rated Gold or better on ProtonDB."* |
| **Move games over** | *"These games come with their own Wine and start scripts. Move each game out of its bundled prefix, add it to Prefixr with Proton, and carry over the fixes from the scripts and my save games."* |
| **Fix games** | *"Fallout 4 crashes on start. Read its latest Prefixr log and fix it."* |
| **Organize the library** | *"Create the categories Kids and Adults and sort all my games into them. Hide the store giveaways I've never played."* |
| **Tidy up** | *"Give every game without artwork a cover, and add my favorites to Steam."* |

### Organizing the library

The agent reads the whole library, store games not installed yet included, with
`prefixr library list --json`: each game's `key`, name, source, categories and play time. Then it
creates the categories it needs and sorts the games in by key:

```bash
prefixr category add Kids
prefixr category add Adults
prefixr category assign Kids epic:Fortnite gog:1207658924
prefixr category assign Adults "Fallout 4"
prefixr library favorite "Fallout 4"
prefixr library hide amazon:amzn1.adg.product.1234
```

It can decide by the names it knows, or look games up first (their age ratings, genres, co-op
modes). The categories show up as chips above the library right away.

### Setting games up

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
