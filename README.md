# VIctory on an RX 7900 XTX

A fork of [VIctory](https://github.com/Marvisak/VIctory), a compatibility layer
that runs an Xbox Series title natively on Windows from files the user supplies.
This one is about **speed on AMD RDNA 3**: the world at 30 frames a second on
one PC, without giving up the picture. It is not there yet.

**This repository contains no game files, code or assets.** You need your own
copy of the game. It is not affiliated with or endorsed by the game's developer
or publisher.

## Where it stands (2026-11-25)

Tested on one machine only: Windows 11, Intel i9-12900K, 64 GB RAM, Radeon
RX 7900 XTX, 1440p, the Series X profile.

| | |
|---|---|
| Menus, videos, loading screens | 30 fps |
| Prologue, indoors | 50-75 ms a frame (13-20 fps) |
| First mission with the boat | 46-53 ms (19-22 fps) |
| Street chase with many cars and people | 100-165 ms (6-10 fps), single frames far longer |
| Outdoors on foot, the camp | 60-90 ms |
| Flying | 42-57 ms |
| Longest session so far | 41 minutes, no crash |
| Memory | 40-46 GB of private memory: **64 GB of RAM recommended** |

It started at 280 ms a frame in the same scenes two days earlier.

What is known to be wrong:

- Trees, plants and far buildings flicker outdoors in some places.
- Rooms with little direct light are too dark without `-RayTracing`.
- Textures of what just came into view appear late while the title streams.
- The first run with a new driver or a new build stutters while the driver makes
  pipelines (`victory warm` makes them ahead).
- Recording a frame in the world makes the title's own watchdog end it.

## How to run it

You need Windows 10/11 x64, a CPU with AVX2, a Direct3D 12 GPU with shader
model 6.6 and mesh shaders, Rust (the toolchain is pinned), the MSVC build
tools, and your own dump of the title.

```powershell
git clone https://github.com/avatarmanz/VIctory-7900XTX
cd VIctory-7900XTX
copy victory.example.toml victory.toml      # then set your paths, outside this folder
cargo build --locked --profile diag -p victory --features diagnostics
scripts\play.ps1 -Exe <path of your copy's executable>
```

`scripts\play.ps1 -Safe` leaves out the newest, faster ways; try it first when
something is wrong. `-RayTracing` turns the title's ray-traced indirect light on.

Before the first run, ahead of time (both can be stopped and go on later):

```powershell
target\diag\victory.exe shaders <the shader archives>   # every shader translated, minutes
target\diag\victory.exe warm --used                     # pipelines earlier runs drew with, for a new driver or build
```

Commands, debugging switches and upstream's own description:
[docs/README-fxceless.md](docs/README-fxceless.md). Every `VICTORY_...` switch is
described in a comment where it is read.

## What is different here

In short; the reasons and the measurements are in [docs/findings.md](docs/findings.md).

- **List calls are written down, not carried out, on the title's threads**
  (`VICTORY_THIN`): eight threads of ours translate them.
- **Submissions are prepared by a thread of ours and joined**
  (`VICTORY_ASYNC_SUBMIT`, `VICTORY_JOIN_PARTS`), with what can be spread over
  threads spread: refreshes, compares (AVX2), descriptor records, landings.
- **What shaders wrote comes back beside the write guards**, by the dwords that
  changed, on a thread of its own (`VICTORY_RELAY`, `VICTORY_LAND`).
- **Buffer fills that need bytes still on the card are done on the card**
  (`VICTORY_CARD_STEPS`) instead of waiting for it.
- **The title's writes are caught more cheaply**: one view opened at a fault,
  blocks that rest, places in the title's code that look before they write.
- **Textures go up on a copy queue of their own** and a slot shows one only after
  its upload ran, which also ended a device loss that came every 15 minutes or so.
- **Boats float**: textures that shaders write and the title's CPU reads go back
  to title memory (from fxceless and Geckomacabre, on by default here).
- **Frame record and replay** with per-pass times and shader variants compared
  turn by turn ([docs/replay.md](docs/replay.md)).

## Changes in this copy (dexyl7, RX 9070 XT, 32 GB)

- AMD's shader compiler (RDNA4) no longer crashes on size queries: Belobug's PR #324 and
  dexyl7/VIctorious' separate handle for `getDimensions`.
- Series S (`scripts\play.ps1 -Console series-s`): the interface plane laid over the game
  (`VICTORY_COMPOSE`), the title's own upscaler, card room for textures; from
  dexyl7/VIctorious, ported there from trustno1-coder/VIctory-NVIDIA (fxceless PR #1189).
- Render scale and the title's own upscaler (`-RenderScale`, `VICTORY_RENDER_SCALE`,
  `VICTORY_OWN_UPSCALER`) and the watchdog patch (`VICTORY_NO_WATCHDOG`): trustno1-coder,
  vitutu1/victory-perf and ZigZagBoi1/ScarlettPort, via dexyl7/VIctorious.
- Sharpening of the shown frame (`-Sharpen`, `VICTORY_SHARPEN`), from dexyl7/VIctorious.
- A submitted list's host side is not replaced until the list's reset (`VICTORY_LAZY_HOSTS=0`:
  as before): fewer host lists alive, less command memory.
- Borderless fullscreen on Alt+Enter or F11, and the frame rate, memory and run time drawn on
  the picture (`VICTORY_FPS_COUNTER=0`: none), from dexyl7/VIctorious.
- Idle host lists are kept by size (`VICTORY_LIST_SIZES=0`: one pool): a recording takes the idle list
  that fits it best, only 64 large ones wait idle (`VICTORY_BIG_IDLE`, `VICTORY_BIG_WORDS`),
  and a full pool lets its largest go first: the driver's command memory stops growing to the
  largest recording in every list.
- Lists' upload buffers made by size class, and host copies' buffers by size, kept for the next
  ones (`VICTORY_UPLOAD_POOL_MB`, 512; 0: off; after dexyl7/VIctorious' `VICTORY_BUFFER_POOL`)
  instead of made new every time; `VICTORY_THIN` left off in `scripts\play.ps1`
  (single frames of stretched geometry with it on, on the RX 9070 XT).
- PC FidelityFX FSR 4.1.2 to 4K (`scripts\play.ps1 -Fsr4K quality|balanced|performance`), Belobug's
  port (cd8dfc0) of DexNom/VIctory ab34a8b2.
- The title's output size (`-Output 2560x1440`, `VICTORY_OUTPUT`; with `-Fsr4K`, FSR upscales to it):
  `resolution.rs` and `frame_rate.rs` from this repository's later main.
- Renderer patch 6 (`VICTORY_PORT_GUARD=1`, off by default): the write guards' lock shared, with a
  claim per 64 KiB block, and refresh planning beside the submitting thread; this repository's
  later main (9d723e8, 2edf844, 65f9707, 00ab2ac, 8f02bf7, 88b63a2) without the rest of it.
- Renderer patches 1 and 5 (`VICTORY_PORT_CONSTANTS`, `VICTORY_PORT_CBV`) and patch 1's ordering fix
  (`VICTORY_PORT_CONSTANTS_ORDER`), from this repository's later main (86dc109, 4b82f6c, 9c23e8d,
  631400f, 3e04ca0, 15c3d0e, 3920dc2); `scripts\play.ps1 -Ports` turns on 1, 5 and 6.

## Credits

This is a tree of other people's work with more on top. The history is kept, and
commits taken from elsewhere name the commit and its author.

- **[VIctory](https://github.com/Marvisak/VIctory)** by Marvisak and its
  contributors: the loader, the runtime services, the Direct3D 12 back end and
  the shader recompiler.
- **[fxceless/victory-fork](https://github.com/fxceless/victory-fork)**: the fork
  this one started from, and the people whose pull requests it carries.
- **[DexNom/VIctory](https://github.com/DexNom/VIctory)**: ray tracing fixes,
  cube gradients, sparse textures.
- **[Belobug/VIctory-Belobug-AMD](https://github.com/Belobug/VIctory-Belobug-AMD)**:
  the ambient light idea, pipeline workers, and work by romainhedouin.
- **[FQY55kai/ZJOU-VICTORY](https://github.com/FQY55kai/ZJOU-VICTORY)**.
- **[myokex/victory-amd](https://github.com/myokex/victory-amd)**: the guard over
  every view of a page (with Spexivus, joroyt and tightfall), and Joshua Odom's
  reads into title memory.
- **Geckomacabre** and **cxy0072019-sketch**: the texture read-back.
- **[XCL](https://github.com/ivall/xcl)** (GPL-2.0-only) was read for how the same
  title behaves under another layer; no code of it is here.
- This fork's changes were made with [Claude Code](https://claude.com/claude-code).

## License

GPL-3.0-or-later, as VIctory. See [LICENSE](LICENSE); the one file that is not
covered is named in [docs/asset-policy.md](docs/asset-policy.md).

ZETTROX
