# VIctory on an RX 9070 XT (randomguy12000's copy)

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
| `-Console series-s -Ports` (THIN on, the default) | ~33.6 ms on average while driving, at the 30 fps cap much of the time; slowest tenth ~35 ms | lamps can flicker, trees can pop |
| `-Console series-s` (THIN on, no `-Ports`) | ~43 ms (~23 fps) | lights stable |
| Either, before THIN was on | ~48-50 ms | |
| Series X `-Fsr4K quality -Ports` | ~60 ms (15-17 fps) | sharper picture, heavier |

Long sessions (Series S, `-Ports`, same machine):

| Run | Length | Frame time early → late | Free RAM late | Notes |
|---|---|---|---|---|
| 2026-11-27, before the fixes below | 43 min | ~35-47 ms → ~54-89 ms (preparing ~25-34 → ~40-65 ms) | 0.2-0.6 GB | process heap 2.6 → 11.4 GB; over 500,000 title textures alive; the once-a-second texture sweep grew to 313 ms; audio dropped out as it slowed |
| 2026-11-28, hosted-only walk | 11 min | ~33-44 ms | 0.3 GB | 166,000 title textures alive; 8.6 GB of host textures on the card, ~2.7 GB of them for memory the title had unmapped; 9.3 GB of the PC's memory shared with the card |
| 2026-11-28, plus the dead-texture sweep | 14 min | ~33-40 ms → ~44-54 ms | 0.1-0.5 GB | 113,000 host copies (11.4 GB) let go as their memory was unmapped; host textures on the card ~4.5-5 GB instead of 8.6 GB |

None of these runs crashed: each ended with the window closed by hand.

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
git clone https://github.com/randomguy12000/VIctorious -b 7900-base VIctory7900
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
| `-SkipIntro` | skips the logo and intro movies (`VICTORY_SKIP_BINK=1`; `all` skips looping menu movies too) |
| `-Output <size>` | the title's output size (Series X): 3840x2160, 2560x1440, 1920x1080, 1664x936, 1280x720; with `-Fsr4K`, what FSR upscales to |
| `-RenderScale 0.2-1` | the title's render size as a share of the output |
| `-Sharpen 0-1` | sharpening of the shown frame |
| `-RayTracing` | the title's ray-traced indirect light (about a third slower) |
| `-Safe` | without the newer, faster ways: the first thing to try when something is wrong |
| `-Build diag\|release` | which build to start (`diag`, the default, is the one measured) |
| `-More @{NAME='value'}` | any other `VICTORY_...` setting, e.g. `-More @{VICTORY_THIN='0'}` |

The title caps itself at 30 frames a second (its own setting; menus and videos run a flat
33.3 ms), so frames that finish sooner wait: that is the headroom. `VICTORY_FRAME_INTERVAL_MS`
changes the cap (16.667 for 60; 0 for none); left unset, the title's 30 stays.

Alt+Enter or F11 switches borderless full screen. The frame rate, VRAM, RAM and run
time are drawn in the corners of the picture; **F3 hides and shows them**
(`VICTORY_FPS_COUNTER=0`: never drawn).

### Settings added in this copy (for `-More`)

| Setting | Default | What it does |
|---|---|---|
| `VICTORY_THIN` | 1 (play.ps1) | list calls written down on the title's threads, translated by threads of ours (~20 % shorter frames) |
| `VICTORY_MAP_DRAIN` | on | the title's queue page mappings wait for the lists submitted before them (THIN's stretched geometry) |
| `VICTORY_PORT_CONSTANTS_ORDER` | 0 with `-Ports` | 1: patch 1's in-place indirect arguments checked one by one (lights and trees right, the card ~36 ms slower a frame) |
| `VICTORY_TABLE_BATCH` | 1 (play.ps1) | a table's descriptor copies in one driver call |
| `VICTORY_FIND_BY_RANGE` | 1 (play.ps1) | buffer lookups forgotten only where buffers changed |
| `VICTORY_SYNC_AT_SUBMIT` | 1 (play.ps1) | the descriptor records round a submission waits for is asked for when the title submits it, not when it is prepared |
| `VICTORY_CARD_BUFFERS_MIN_KB` | 64 (play.ps1; 4 MB on a 16 GB card without it) | title buffers of that size and more copied to the card and compared there, not on the CPU (the method from fxceless/victory-fork, the KiB setting after its c36971b2; trustno1-coder/victory-nvidia uses 1 MB). At 1 MB: the sending thread's snapshots 4.0 → 1.6 ms a frame; at 64 KiB: 0, 3,426 copies (802 MB) on the card, frames mean 36.8 → 33.6 ms, slowest tenth 38.6 → 35.0 |
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

## What this builds on

This copy starts from [avatarmanz/VIctory-7900XTX](https://github.com/avatarmanz/VIctory-7900XTX)
(itself from [fxceless/victory-fork](https://github.com/fxceless/victory-fork) and
[VIctory](https://github.com/Marvisak/VIctory)), which already translates list calls on threads of its
own, prepares submissions on a thread of its own, sends them ahead without waiting for the card, copies
what shaders wrote back beside the write guards, and uploads textures on a copy queue of their own. Its
README has the 7900 XTX measurements and the reasons, and [docs/findings.md](docs/findings.md) here
keeps them. Everything below is what this copy changed on top, measured on the RX 9070 XT.

## Changes in this copy (randomguy12000, RX 9070 XT, 32 GB)

- AMD's shader compiler (RDNA4) no longer crashes on size queries: Belobug's PR #324 and
  randomguy12000/VIctorious' separate handle for `getDimensions`.
- Series S (`scripts\play.ps1 -Console series-s`): the interface plane laid over the game
  (`VICTORY_COMPOSE`), the title's own upscaler, card room for textures; from
  randomguy12000/VIctorious, ported there from trustno1-coder/VIctory-NVIDIA (fxceless PR #1189).
- Render scale and the title's own upscaler (`-RenderScale`, `VICTORY_RENDER_SCALE`,
  `VICTORY_OWN_UPSCALER`) and the watchdog patch (`VICTORY_NO_WATCHDOG`): trustno1-coder,
  vitutu1/victory-perf and ZigZagBoi1/ScarlettPort, via randomguy12000/VIctorious.
- Sharpening of the shown frame (`-Sharpen`, `VICTORY_SHARPEN`), from randomguy12000/VIctorious.
- A submitted list's host side is not replaced until the list's reset (`VICTORY_LAZY_HOSTS=0`:
  as before): fewer host lists alive, less command memory.
- Borderless fullscreen on Alt+Enter or F11, and the frame rate, memory and run time drawn on
  the picture (`VICTORY_FPS_COUNTER=0`: none), from randomguy12000/VIctorious.
- Idle host lists are kept by size (`VICTORY_LIST_SIZES=0`: one pool): a recording takes the idle list
  that fits it best, only 64 large ones wait idle (`VICTORY_BIG_IDLE`, `VICTORY_BIG_WORDS`),
  and a full pool lets its largest go first: the driver's command memory stops growing to the
  largest recording in every list.
- Lists' upload buffers made by size class, and host copies' buffers by size, kept for the next
  ones (`VICTORY_UPLOAD_POOL_MB`, 512; 0: off; after randomguy12000/VIctorious' `VICTORY_BUFFER_POOL`)
  instead of made new every time; `VICTORY_THIN` left off in `scripts\play.ps1`
  (single frames of stretched geometry with it on, on the RX 9070 XT).
- PC FidelityFX FSR 4.1.2 to 4K (`scripts\play.ps1 -Fsr4K quality|balanced|performance`), Belobug's
  port (cd8dfc0) of DexNom/VIctory ab34a8b2.
- Patch 1's ordering in batches that end at every barrier and value wait of the title's
  (`VICTORY_PORT_BATCH_AT_SYNC=1`, off: tried once, the batches ended at nearly every execution and
  their memory ran the card past its budget; used with `VICTORY_PORT_CONSTANTS_ORDER=1` and
  `VICTORY_PORT_CONSTANTS_ORDER_ALONE=0`): batches that ended only after a known buffer writer still
  let trees and cars drop out, here as in upstream's run pop-15; every in-place execution (2,400-4,600
  a frame) is a single record without a count, so batches take all of them.
- The texture sweep and the memory lines walk only the textures that were given a host copy
  (`VICTORY_HOSTED_WALK=0`: all, as before): over a 43-minute run the title had more than 500,000
  textures alive, and walking all of them each second under the lock the title needs to make and free
  textures grew to 313 ms at a time while the frame went from about 35 to about 88 ms.
- Tried and left off: landed lists reset aside (`VICTORY_LAND_ASIDE=1` turns it on; A/B-able). The
  host lists of flights the card has run were reset and pooled by the thread that settles flights, not
  by the submitting thread (~9 % of its sampled time in an earlier profile). In an A/B drive
  (2026-11-28) the submitting thread's work stayed at ~24-28 ms a frame either way.
- Heaps' records asked for at submit (`VICTORY_SYNC_AT_SUBMIT=1`, on in play.ps1, A/B-able): the round that mirrors the descriptor records the title copied into its
  heap is asked for when the title submits, and made beside the line while it gets there, instead of
  when the submitting thread prepares the submission. That thread waited 6-9 ms a frame for those
  rounds, which also took in what the title had written since for the submissions behind. In an A/B
  drive (2026-11-28, 20 s each way) the wait went from ~6-9 to ~1-4 ms a frame, the slowest tenth of
  frames from mostly ~40 to mostly ~38 ms, with no new visual faults seen.
- Slot marks (`VICTORY_SLOT_MARKS=0`: off): a slot update that would only take a slot out of the
  queue of textures picked by index, or out of the placed-texture spans, skips that structure's lock
  when a lock-free mark says the slot is not there. The threads that make the heaps' records spent
  ~20 % of their time queued for those two locks in a profile while driving, mostly for slots in
  neither; the wait for the records did not measurably change with it (2026-11-28).
- Tried and left off: page mappings in the line (`VICTORY_MAP_IN_LINE=1` turns it on; A/B-able). The
  title's page-mapping calls became a job in their queue's order, done by the submitting thread, instead
  of the title's streaming thread waiting for every submission before them (`VICTORY_MAP_DRAIN`; that
  thread stood ~58 % of its sampled time in that wait in a profile while driving). In an A/B drive
  (2026-11-28) frames were worse with it (means 50-58 ms against 38-43 ms off) and textures no faster.
  Turning the drain off instead was no faster either and was followed by a device removal.
- Read-back buffers for textures the title reads back are kept after their texels land and handed
  out again by size (`VICTORY_READBACK_POOL=0`: made each time, as before): making one at every
  list's close took 3-5 % of the translating threads' time in a profile while driving.
- `VICTORY_TABLE_REUSE_COUNT=1` counts how many descriptor tables a list makes that an earlier list
  made with the same records (nothing changes): building tables took ~47 % of the translating threads'
  time in that profile, every list making each of its tables anew.
- The sampling profiler (`VICTORY_PROFILE`) stops a thread's stack walk where a function's frame
  register does not point into that thread's stack: a sample of a title thread taken mid-function
  read through a garbage frame pointer and ended the process.
- Tried and left off (`VICTORY_CRITICAL_PRIORITY=1` turns it on): the threads a submission waits
  for (the heaps' records, the relay and its landing threads) at above-normal priority. The wait for
  the heaps' records fell from ~8 to ~4 ms a frame, but the picture froze for seconds at a time, often,
  while the game went on: with 13 of our threads above the title's own, those stood until Windows
  lifted them.
- Host copies of placed textures whose console memory is no longer mapped are let go once idle for
  30 seconds (2 seconds made 16,800 of 78,000 come back, uploaded again), whether or not the card is near its limit (`VICTORY_DEAD_SWEEP=0`: kept, as before):
  11 minutes into a Series S run 2.7 GB of them were still on the card while 9.3 GB of the PC's
  memory was shared with it. The same checks as the existing sweep (no list, no slot, nothing else
  holds it); a later use makes the copy again.
- Skipping the logo and intro movies (`-SkipIntro`, `VICTORY_SKIP_BINK`): written by C-GDev
  (fxceless/victory-fork pull request 613; the idea from XCL pull request #1053), as carried in
  CuteBooty/victory-8gb and trustno1-coder/victory-nvidia e31f4740.
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
  (`VICTORY_ALLOC_OWNERS`; the RAM investigation is done). The diagnostics panel
  (`VICTORY_PANEL`) is gone too: F3 now hides and shows the corner boxes instead.
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
  avatarmanz's started from, the card-buffer comparisons on the card, and the people whose pull
  requests it carries.
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
- **[avatarmanz/VIctory-7900XTX](https://github.com/avatarmanz/VIctory-7900XTX)**: the fork this
  copy is built on, and the renderer patches behind `-Ports`.
- **[trustno1-coder/victory-nvidia](https://github.com/trustno1-coder/victory-nvidia)**: the 1 MB
  card-buffer setting, and the intro skip as carried there.
- **[TaynanGT/Vice-Tay](https://github.com/TaynanGT/Vice-Tay)**: the frame generation code.
- **ZETTROX**: credited in avatarmanz's README.
- This copy's changes were made with [Claude Code](https://claude.com/claude-code).

## License

GPL-3.0-or-later, as VIctory. See [LICENSE](LICENSE); the one file that is not
covered is named in [docs/asset-policy.md](docs/asset-policy.md).
