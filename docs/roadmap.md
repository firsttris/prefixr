# Ideas: upscaling

Features Prefixr doesn't have yet but could, collected so they aren't forgotten. Each one says what
it would do, how it could work, and what to watch out for. Nothing here is promised; pull requests
are welcome.

- [Upscaling with Gamescope](#upscaling-with-gamescope)
- [The FSR 4 upgrade in Proton](#the-fsr-4-upgrade-in-proton)
- [OptiScaler](#optiscaler)
- [Spatial and temporal upscaling](#spatial-and-temporal-upscaling)

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
