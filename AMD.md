# AMD notes (`amd-work` branch)

Private working branch on top of [fxceless/victory-fork](https://github.com/fxceless/victory-fork)
(merged up to its commit `9376fd6`),
tested on an **RX 9070 XT (RDNA 4, 16 GB), Adrenalin 26.11.2, Ryzen 7 9800X3D, 32 GB, Windows 11**.
No game files, logs or dumps are in this repository.

## Highlights

- **0** compute shaders crash AMD's driver compiler (66+ before): the `getDimensions` fix.
- **0** shaders fail to translate during play, and **0** dispatches are dropped: the
  native-wave mask fix brings in the water volumetrics, the last one that failed.
- Writable raw views off 16 bytes now get their own copy, instead of silently losing every
  shader write through them.

## Status

| | |
|---|---|
| Driver compiler crashes (`amdxc64.dll+0x861b9f`) | **fixed**: every compute pipeline of the title builds |
| Shader translation in play | **every shader used translates**; no dispatch dropped |
| Stable mode | `fast`: ~6 fps in the world |
| Fast mode | `fast2`: 5–8 fps, up to ~10 in light scenes, 25+ min with no device loss |
| Furthest point | the first boat mission: the boat sinks and the player cannot swim (open, see below) |

## Changes on top of the fork

### Translator (`victory-shader`)
- **`getDimensions` of a heap-indexed raw buffer gets its own non-uniform handle.**
  AMD's DXIL compiler dereferences null lowering it on a uniform-marked handle; this took the
  count of compute shaders that crash the driver compiler from 66+ to 0.
  After commit 50190fb in fork PR #697 (by Kyle). `VICTORY_DIMS_UNIFORM=1` restores the old handle.
- **Lane masks holding lanes outside EXEC, on a host whose wave is the console's.** A ballot taken
  in a block every lane runs is exact there too (it was only allowed for two exchanging 32-lane
  waves). The water volumetrics compute (`water_shading:CS_TransmissionVolumetricsSubmerged`)
  translates now. `VICTORY_MASK_NATIVE=0` refuses it as before.

### GPU layer (`victory-gpu`)
- **Unordered raw buffer views that do not start on 16 bytes get a copy of their own bytes**
  (the fork's own-copy path) instead of being left null, so shader writes through them are no
  longer dropped. `VICTORY_RAW_OWN_COPY`: unset or `uav` (default), `all`, `srv`, `0`.
  Read-only views are left out by default: with them on, the title overran a stack buffer just
  before the boat mission.
- Fork PRs merged in: **#665** (dedicated copy queue for relay fetches, parallel merge) and
  **#220** (cheaper indirect draws).
- Clean upstream (marvisak/VIctory) commits that touch no file the fork changed (19 files).

### Skipped shaders
`fc5e66f8f873d3bb` and `0452fc2d2e2fc4c3` (`meshlet_culling:CS_BatchCullMeshlets_Prev` and its
sibling) page-fault in their indirect dispatch on AMD (fork issue #3). `play.ps1` skips them through
`crashers_list.txt`.

## Diagnostics (diagnostics builds)

| Switch | What it does |
|---|---|
| `VICTORY_PIPELINE_WORKERS=n` | pipeline build threads |
| `VICTORY_SKIP_PIPELINES=h,...` | pipelines never created (not only their draws skipped) |
| `VICTORY_ABLATE=name[:op],@op` / `VICTORY_ABLATE_RANGE=a-b` | leave DXIL operations out, to bisect a driver compiler crash |
| `VICTORY_DXIL_FLAGS_OR`, `VICTORY_NO_WAVE_SIZE`, `VICTORY_SPLIT_FMAD` | compiler-path experiments |
| `VICTORY_TRANSLATE_ONLY` / `_SKIP` / `_SKIP_FILE` / `_NAME` / `_CS_ONLY` / `_TRACE` | `translate` filters and tracing |
| `VICTORY_FAKE_STORE_HASH=h,...` + `VICTORY_FAKE_STORE_VALUE=f` | those shaders' raw buffer stores, texture stores and pixel outputs write a fixed value (does a GPU result reach the title?) |
| `VICTORY_SETTLE_INDIRECT=1`, `VICTORY_MERGE_CARD_ONLY=1` | relay stage-2 experiments |

## Play modes (`play.ps1 -Mode …`)

| Mode | Switches | Result here |
|---|---|---|
| `default` | the fork's defaults (relay stage 2, late wait stage 3) | faults on AMD in the `_Prev` culling dispatch without the skips |
| `fast` | `VICTORY_RELAY=1`, `VICTORY_LATE_WAIT=1`, `VICTORY_WIDE_MERGE=1`, `VICTORY_INDIRECT_WINDOW=1` | stable, ~6 fps |
| `fast2` | as `fast` with `VICTORY_LATE_WAIT=2` | 5–8 fps (up to ~10); stable with the two culling shaders skipped |
| `fast2debug` | `fast2` with the D3D12 debug layer and GPU-based validation | too slow: the title gives up while loading |

Every mode turns on DRED (`VICTORY_D3D12_DRED=1`) and skips the shaders in `crashers_list.txt`.

## Warming the shader cache

Translates every shader of the pack into the run's cache ahead of play (about 3–4 minutes; the
host values are what a run's log reports for this card):

```powershell
$env:VICTORY_HOST_WAVE = "64"
$env:VICTORY_VIEW_LIMIT = "2097152"
.\target\diag\victory.exe translate --warm all --jobs 16 "<the title's shaders.rpf>"
```

## Scripts

| Script | Use |
|---|---|
| `play.ps1 -Mode fast\|fast2\|…` | runs the title with a set of the fork's speed switches; log in `play_<mode>.txt` |
| `trial.ps1 [-Hashes h,...] [-Knobs @{...}]` | builds shaders' pipelines offline, reports built / crash / does not translate |
| `sweep2.ps1` | builds every compute pipeline one by one, listing driver compiler crashers (resumes) |
| `bisect.ps1 -Hash h` | narrows a driver compiler crash to DXIL operations |
| `dump.ps1`, `dumpname.ps1`, `dumpall.ps1`, `disas.ps1` | DXIL and disassembly of shaders by hash or name |

Run scripts with `Set-ExecutionPolicy -Scope Process Bypass` first. **Turn the network off before
running the title**: its crash reporter tries to upload dumps.

## Open: the first boat mission

The boat sits low, floods and fails the mission; the player cannot swim. Seen on NVIDIA and AMD
(fork #279, #372, #618; upstream #2248). Ruled out so far: untranslated or dropped shaders (none
left), relay timing (stage 1 too), the water height look-up results, the top-down coarse water
height map and the simulation time step (each forced to a fixed value with `VICTORY_FAKE_STORE_*`:
no change). Likely on the CPU side.
