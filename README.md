# VIctory on an RX 9070 XT (dexyl7's copy)

A fork of [VIctory](https://github.com/Marvisak/VIctory), a compatibility layer
that runs an Xbox Series title natively on Windows from files the user supplies,
by way of [avatarmanz/VIctory-7900XTX](https://github.com/avatarmanz/VIctory-7900XTX).
This copy is tuned on an **AMD RX 9070 XT (RDNA 4), Ryzen 7 9800X3D, 32 GB RAM**.
The goal is a steady 30 frames a second in the world. It is not there yet.

**This repository contains no game files, code or assets.** You need your own
copy of the game. It is not affiliated with or endorsed by the game's developer
or publisher.

## Where it stands (2026-11-28)

One machine: Windows 11, Ryzen 7 9800X3D, 32 GB RAM (page file raised to ~80 GB),
RX 9070 XT. Series S profile, driving in the world, ~4,000-7,000 draws a frame:

| Settings | Frame time | Notes |
|---|---|---|
| `-Console series-s -Ports` (THIN on, the default) | ~38 ms (~26 fps) | lamps can flicker, trees can pop |
| `-Console series-s` (THIN on, no `-Ports`) | ~43 ms (~23 fps) | lights stable |
| Either, before THIN was on | ~48-50 ms | |
| Series X `-Fsr4K quality -Ports` | ~60 ms (15-17 fps) | sharper picture, heavier |

What is known to be wrong:

- A little geometry flicker remains (characters' clothing, single frames).
- With `-Ports`, lamp-lit rooms can flicker and trees and palms can pop for a few
  frames (patch 1's ordering is off: on, it cost the card ~36 ms a frame).
- Textures of roads and buildings can be late or black while the title streams.
- The Series S profile with 1440p output draws the interface large; use 4K output.
- Frame generation (`-FrameGen`) doubles the shown frames but flickers at today's
  real frame rate; best left off until the real rate is near 30.

## How to run it

You need Windows 10/11 x64, a CPU with AVX2, a Direct3D 12 GPU with shader
model 6.6 and mesh shaders, Rust (the toolchain is pinned), the MSVC build
tools, and your own dump of the title.

```powershell
git clone https://github.com/dexyl7/VIctorious -b 7900-base VIctory7900
cd VIctory7900
copy victory.example.toml victory.toml      # then set your paths, outside this folder
cargo build --locked --profile diag -p victory --features diagnostics
```

Then, from a normal (not administrator) PowerShell window:

```powershell
cd <the folder>
Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass -Force
# Fastest today (32 GB RAM): Series S profile with the renderer patches
scripts\play.ps1 -Exe <path of your copy's executable> -Console series-s -Ports
# Lights without flicker, a little slower
scripts\play.ps1 -Exe <path of your copy's executable> -Console series-s
# Series X with PC FSR 4.1.2 to 4K (needs the AMD DLLs below)
scripts\play.ps1 -Exe <path of your copy's executable> -Fsr4K quality -Ports
```

Add `*> play.txt` to keep the log. If `cargo build` fails, do not start the game:
the script would run the previous build. Turn the network off before the game runs.

Before the first run, ahead of time (both can be stopped and go on later):

```powershell
target\diag\victory.exe shaders <the shader archives>   # every shader translated, minutes
target\diag\victory.exe warm --used                     # pipelines earlier runs drew with, for a new driver or build
```

### `scripts\play.ps1` switches

| Switch | What it does |
|---|---|
| `-Exe <path>` | your copy's executable (required) |
| `-Console series-x\|series-s` | the console the title takes this PC for. `series-s` needs less memory (32 GB PCs) and lays the interface plane over the game |
| `-Ports` | renderer patches 1, 5 and 6: the title's constants and constant buffers read in place, the write guards' lock shared. Faster; patch 1's ordering is left off (see above) |
| `-Fsr4K quality\|balanced\|performance` | PC FidelityFX FSR 4.1.2 upscaling to 3840x2160 (Series X only) |
| `-FfxDir <folder>` | where AMD's DLLs are (default `C:\FidelityFX`): `amd_fidelityfx_upscaler_dx12.dll` for `-Fsr4K`; for `-FrameGen` also `amd_fidelityfx_framegeneration_dx12.dll` and `amd_fidelityfx_loader_dx12.dll`, all from the FSR SDK 2.4.0 release (GPUOpen-LibrariesAndSDKs/FidelityFX-SDK, `Kits/FidelityFX/signedbin`) |
| `-FrameGen` | with `-Fsr4K`: AMD frame generation, 2x shown frames |
| `-Output <size>` | the title's output size (Series X): 3840x2160, 2560x1440, 1920x1080, 1664x936, 1280x720; with `-Fsr4K`, what FSR upscales to |
| `-RenderScale 0.2-1` | the title's render size as a share of the output |
| `-Sharpen 0-1` | sharpening of the shown frame |
| `-RayTracing` | the title's ray-traced indirect light (about a third slower) |
| `-Safe` | without the newer, faster ways: the first thing to try when something is wrong |
| `-Build diag\|release` | which build to start (`diag`, the default, is the one measured) |
| `-More @{NAME='value'}` | any other `VICTORY_...` setting, e.g. `-More @{VICTORY_THIN='0'}` |

Alt+Enter or F11 switches borderless full screen. The frame rate, memory and run
time are drawn on the picture (`VICTORY_FPS_COUNTER=0`: not).

### Settings added in this copy (for `-More`)

| Setting | Default | What it does |
|---|---|---|
| `VICTORY_THIN` | 1 (play.ps1) | list calls written down on the title's threads, translated by threads of ours (~20 % shorter frames) |
| `VICTORY_MAP_DRAIN` | on | the title's queue page mappings wait for the lists submitted before them (THIN's stretched geometry) |
| `VICTORY_PORT_CONSTANTS_ORDER` | 0 with `-Ports` | 1: patch 1's in-place indirect arguments checked one by one (lights and trees right, the card ~36 ms slower a frame) |
| `VICTORY_TABLE_BATCH` | 1 (play.ps1) | a table's descriptor copies in one driver call |
| `VICTORY_FIND_BY_RANGE` | 1 (play.ps1) | buffer lookups forgotten only where buffers changed |
| `VICTORY_FRAMEGEN`, `VICTORY_FFX_FG_DIR` | off | what `-FrameGen` sets |
| `VICTORY_LAZY_HOSTS`, `VICTORY_LIST_SIZES`, `VICTORY_UPLOAD_POOL_MB` | on, on, 512 | command and upload memory reused instead of made anew |
| `VICTORY_WIDE_THREADS` | 4 | threads that copy title memory for a submission (1-15) |

### Measuring

- `VICTORY_AB='<seconds>,<SWITCH>[,<SWITCH>..]'`: switches turned on and off in
  turn while the title runs; each phase's frame times are in the log
  (`victory_gpu::ab`). One run compares like with like.
- `VICTORY_TIMELINE='<s>[+<s>..],<frames>,<file.json>'`: a timeline of a few
  frames, with a `.txt` beside it naming the chain of work each frame waited on.
- `VICTORY_PROFILE='<from s>,<to s>,<ms>,<file>'`: a sampling profile of every
  thread.

Commands, debugging switches and upstream's own description:
[docs/README-fxceless.md](docs/README-fxceless.md). Every `VICTORY_...` switch is
described in a comment where it is read.

## Upstream: where it stood on an RX 7900 XTX (avatarmanz, 2026-11-25)

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
- With the line of submissions (`VICTORY_THIN=1` turns it on), the title's queue page mappings
  (`CopyPageMappingsBatchX`) wait until the lists submitted before them are translated and prepared
  (`VICTORY_MAP_DRAIN=0`: not): the call is a queue's, in its order, and was done at once on the
  assumption that every earlier submission had run. Suspected of THIN's stretched-geometry frames.
- AMD frame generation with FSR (`-Fsr4K ... -FrameGen`, `VICTORY_FRAMEGEN=1`): `framegen.rs` and its
  hooks from TaynanGT/Vice-Tay 3236af66 (Taynan Piza, with beloooo), whose ABI and swap-chain flow follow
  PastiiJay/klbf-lab 793c56c3; without its DLSS path and panel changes.
- The title's output size (`-Output 2560x1440`, `VICTORY_OUTPUT`; with `-Fsr4K`, FSR upscales to it):
  `resolution.rs` and `frame_rate.rs` from this repository's later main.
- Renderer patch 6 (`VICTORY_PORT_GUARD=1`, off by default): the write guards' lock shared, with a
  claim per 64 KiB block, and refresh planning beside the submitting thread; this repository's
  later main (9d723e8, 2edf844, 65f9707, 00ab2ac, 8f02bf7, 88b63a2) without the rest of it.
- Renderer patches 1 and 5 (`VICTORY_PORT_CONSTANTS`, `VICTORY_PORT_CBV`) and patch 1's ordering fix
  (`VICTORY_PORT_CONSTANTS_ORDER`), from this repository's later main (86dc109, 4b82f6c, 9c23e8d,
  631400f, 3e04ca0, 15c3d0e, 3920dc2); `scripts\play.ps1 -Ports` turns on 1, 5 and 6, with patch 1's
  ordering off: checking each in-place indirect execution alone took about 36 ms of the card a frame
  here (61 ms busy against 25), at the cost of trees and palms that can drop out for a few frames.
- Descriptor tables planned once per pipeline (`TablePlan`): codec-d/cool_fox 641835a4 by Graham,
  via this repository's later main (46e3297).
- Tried and taken out again: renderer port RT part 1 (this repository's later main, 84917c3 and
  014d52d; it removed the submitting thread's wait but the frame was no shorter here), the fallback
  heaps that came with 46e3297, and the tool that found which module asked for big memory blocks
  (`VICTORY_ALLOC_OWNERS`; the RAM investigation is done).
- A descriptor slot that keeps showing the same resource no longer clones it on every draw (a
  reference count shared by the recording threads).
- A table made afresh copies its descriptors in one call and retires what its slots let go of
  together (`VICTORY_TABLE_BATCH=1`, off by default); a thread's memory of which host copy holds a
  buffer is forgotten only for the MiB a buffer changed in, not for every buffer anywhere
  (`VICTORY_FIND_BY_RANGE=1`, off by default; `scripts\play.ps1` turns both on). Both from the
  recording threads' profile of this copy: about 15 times fewer copy calls, and some 8-16 % less time
  per draw with both on in one A/B run (four phases each, noisy).

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
