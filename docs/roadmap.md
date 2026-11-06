# Roadmap

Features Prefixr doesn't have yet but could, collected so they aren't forgotten, in the order they
would be worth doing: most use for the least work first. Each one says what it would do, how it
could work, and what to watch out for. Nothing here is promised; pull requests are welcome.

Some ideas come from [Prefixer](https://github.com/wojtmic/prefixer), a command-line tool for Steam's
Proton prefixes (also GPL-3.0): its declarative tweaks write registry values and drop mod loaders
into a game's folder.

| # | Idea | Effort | Why this place |
| --- | --- | --- | --- |
| 1 | [Prefix switches](#prefix-switches) | small | Fixes common game problems (mouse, Windows version, crash dialogs) without `winecfg` or `regedit` |
| 2 | [Upscaling with Gamescope](#upscaling-with-gamescope) | small | Gamescope is wrapped already; three settings more |
| 3 | [A dark theme for Wine's windows](#a-dark-theme-for-wines-windows) | small | Launchers, installers and `winecfg` stop looking like Windows 98 next to a dark desktop |
| 4 | [The FSR 4 upgrade in Proton](#the-fsr-4-upgrade-in-proton) | small | The switch exists already; it's about finding it |
| 5 | [Mods per game](#mods-per-game) | medium | One mechanism for mod loaders (BepInEx, Steamodded) and [OptiScaler](#optiscaler) |
| 6 | [Shareable game recipes](#shareable-game-recipes) | large | Fixes for games umu-protonfixes doesn't know, shared as a file |
| 7 | [Frame generation with lsfg-vk](#spatial-and-temporal-upscaling) | medium | Needs a paid app; an optional extra like MangoHud |

## Prefix switches

**What:** the registry settings games most often need, as switches in a prefix's (or a game's)
settings instead of a trip through `winecfg` or `regedit`:

| Switch | Registry (`user.reg`) | Helps when |
| --- | --- | --- |
| Windows version (XP … 11) | `Software\Wine` `Version` | An old game refuses a new Windows, or a new one an old Windows |
| Virtual desktop, with a resolution | `Software\Wine\Explorer` `Desktop`, `…\Explorer\Desktops` | A game changes the resolution and leaves the desktop broken |
| Capture the mouse in fullscreen | `Software\Wine\X11 Driver` `GrabFullscreen` | The cursor leaves the game onto another monitor |
| Mouse warp | `Software\Wine\DirectInput` `MouseWarpOverride` | The camera spins or jumps in older games |
| Window decorations | `Software\Wine\X11 Driver` `Decorated` | A borderless game gets a title bar |
| No crash dialog | `Software\Wine\WineDbg` `ShowCrashDialog` | A crashing game leaves Wine's dialog behind |
| Video memory | `Software\Wine\Direct3D` `VideoMemorySize` | A game misjudges the graphics memory (WineD3D only) |
| Hide Wine | `Software\Wine` `HideWineExports` | A game or launcher refuses to run under Wine |

**How:** read and write `user.reg` directly, as Prefixer does, instead of starting `regedit` in the
prefix: that takes milliseconds rather than seconds, and the switches can show the current value
without starting Wine. A small parser for Wine's `.reg` format (sections, quoted strings, `dword:`,
`hex:`) in the backend, the switches in the prefix dialog and as per-game overrides, and `prefixr
prefix set <prefix> <switch>=<value>` on the command line.

**Watch out:** `wineserver` keeps the registry in memory and writes it back when the prefix's last
program exits, overwriting changes made meanwhile; only write while nothing runs in the prefix
(Prefixr knows its running games, `wineserver -k` ends the rest). `HideWineExports` exists only in
Wine Staging and some Proton builds. Virtual desktops and the X11 driver's keys do nothing with
Proton's Wayland driver. `VideoMemorySize` is WineD3D's; DXVK reads `dxgi.maxDeviceMemory` from
`dxvk.conf` instead.

## Upscaling with Gamescope

**What:** let any game, old ones included, render at a lower resolution and have
[Gamescope](https://github.com/ValveSoftware/gamescope) scale it up to the screen with FSR 1, NIS,
integer or nearest-neighbour scaling. That helps games that only offer a few fixed resolutions,
ports that run poorly at native resolution, and pixel-art games that should stay sharp.

**How:** Prefixr already wraps a game in Gamescope (*Graphics* settings: width, height, FPS limit,
fullscreen). Gamescope tells the game's resolution (`-w`/`-h`) apart from the output resolution
(`-W`/`-H`) and takes a scaling filter (`-F fsr|nis|linear|nearest|pixel`) and a sharpness
(`--sharpness`). The *Graphics* settings would get:

- **Render resolution** (what the game sees) and **output resolution** (the screen), or a scale
  factor such as 75 %
- **Scaling filter:** FSR, NIS, integer, pixel, linear
- **Sharpness** for FSR and NIS

`GamescopeSettings` in `models.rs` gains the fields, `run_game` the arguments. Because the settings
are overridable per game, an old game can get integer scaling while the rest keep native resolution.

**Watch out:** Gamescope only scales what the game renders. A game that ignores the resolution it
is told still renders at its own. Inside Steam's Game Mode, Prefixr already skips Gamescope; the
session's own scaling applies there.

## A dark theme for Wine's windows

**What:** Wine draws its own windows (`winecfg`, installers, many launchers, message boxes) in the
grey of Windows 98. A switch per prefix turns them dark, in colours that match the desktop's.

**How:** Wine takes its colours from the registry: `Control Panel\Colors` (about 30 values such as
`ButtonFace`, `Window`, `WindowText`), with the theme switched off under
`Software\Microsoft\Windows\CurrentVersion\ThemeManager` and dark mode set under
`…\Themes\Personalize` (`AppsUseLightTheme` = 0). Prefixer ships such a set for Breeze Dark. Built
on the [prefix switches](#prefix-switches)' registry writing; a preset follows the desktop's light or
dark mode at prefix creation.

**Watch out:** only Wine's own controls follow it. Games and launchers that draw themselves (Qt,
Electron, Chromium-based ones) stay as they are. "Reset" has to restore Wine's defaults, so the
values from before are kept.

## The FSR 4 upgrade in Proton

**What:** games that ship FSR 3.1 can run AMD's newer, machine-learning based FSR 4 instead, which
looks considerably better at the same resolution. Recent Proton builds can swap the upscaler when
asked to through an environment variable.

**How:** Prefixr already lists every `PROTON_*` switch a runner's `proton` script understands (the
*Proton* settings and tab), so a runner that supports the upgrade offers it there today. What's
missing is making it discoverable:

- A description for the switch, like the common switches have, saying which games it applies to
  (those with FSR 3.1) and which GPUs support it
- Possibly a hint on a game whose files contain an FSR 3.1 DLL (`amd_fidelityfx_dx12.dll`)

**Watch out:** FSR 4 officially needs an RDNA 4 GPU (Radeon RX 9000 series). Ways to run it on
RDNA 3 exist in some Proton builds but are unofficial and slower. The switch names have changed
between Proton releases, which is why Prefixr reads them from the runner instead of hard-coding
them.

## Mods per game

**What:** install a mod loader into a game with one click: [BepInEx](https://github.com/BepInEx/BepInEx)
for Unity games, [Steamodded](https://github.com/Steamodded/smods) for Balatro, an ASI loader, and
[OptiScaler](#optiscaler). Each is a few files next to the game's exe plus a DLL override, which is
where people get stuck under Wine.

**How:** one mechanism for all of them, described per mod as data:

1. download a pinned release, verify its checksum and cache it (as for runners, umu and DXVK),
2. extract it into the game's folder, setting aside any game file of the same name,
3. add the DLL override to the game's launch (`WINEDLLOVERRIDES`, e.g. `winhttp=n,b` for BepInEx),
4. record what was placed, so switching the mod off removes exactly that and restores the rest.

The game's settings get a *Mods* section listing what's installed, with an update when the pinned
release changes.

**Watch out:** two mods can claim the same proxy DLL (`winhttp`, `dxgi`, `version`, `winmm`); the
second one must be refused rather than overwrite the first. Online games with anti-cheat can treat
any of them as tampering. Store updates may restore files a mod replaced.

## OptiScaler

**What:** [OptiScaler](https://github.com/optiscaler/OptiScaler) is a mod that intercepts a game's
own upscaler (DLSS, XeSS or FSR 2/3) and replaces it with another one, for example FSR 3.1 or
FSR 4. A game that only offers DLSS thereby gets a good upscaler on an AMD or Intel GPU, and some
setups add frame generation. It works under Proton.

**How:** a per-game option that

1. downloads OptiScaler's latest release (like Prefixr already does for runners, umu and DXVK,
   checksum-verified) and caches it,
2. copies it into the game's folder under the name the game loads (usually `dxgi.dll`, sometimes
   `winmm.dll` or another proxy DLL), keeping a game file of that name aside,
3. adds the matching `WINEDLLOVERRIDES` entry (for example `dxgi=n,b`) to the launch,
4. and removes all of it again when the option is switched off.

OptiScaler's own overlay (Insert key) then picks the upscaler in the game.

**Watch out:** it only helps games that already have an upscaler: it needs the motion vectors those
pass on. The proxy DLL must not collide with one the game or another mod already uses (a DXVK copy
next to the exe, ReShade, an ASI loader). Online games with anti-cheat can treat it as tampering.

## Shareable game recipes

**What:** what a game needs to run well, as a small file people can share: winetricks packages, DLL
overrides, prefix switches, environment variables, Proton switches, a mod. Prefixer does this with
json5 files per game (Cyberpunk 2077: `d3dcompiler_47`, `vcrun2022` and two DLL overrides).

**How:** most of it is a game's settings already. Export a game's settings (without paths) as a
recipe, import one onto a game, and look up a recipe by the game's umu or Steam ID at add time.
Recipes could live in a repository that takes pull requests.

**Watch out:** [umu-protonfixes](https://github.com/Open-Wine-Components/umu-protonfixes) already
applies fixes for many games automatically, so recipes are for the rest, and must not fight it.
An imported recipe runs installers and downloads, so it has to show what it will do and ask first.

## Spatial and temporal upscaling

Why these are three separate features:

| | Works with | Quality | Needs from the game |
| --- | --- | --- | --- |
| **FSR 1** (spatial), via Gamescope | every game | fine at small factors, soft at large ones | nothing, it scales the finished image |
| **FSR 2/3/4** (temporal) | games that ship it, or OptiScaler | close to native resolution | motion vectors and depth from the engine |
| **Frame generation** | games that ship it, OptiScaler in some cases | more frames, a little more latency | the same as temporal upscaling |

Temporal upscalers and frame generation can't be added to a game from the outside, because only the
engine knows how each pixel moved. For games without any upscaler, Gamescope's FSR 1 is the option.
Frame generation for any game exists in [lsfg-vk](https://github.com/PancakeTAS/lsfg-vk), a Vulkan
layer for the frame generation of the paid app *Lossless Scaling*; it would be a candidate for an
optional setting like MangoHud or vkBasalt.
