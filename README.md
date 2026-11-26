# Scarlett fork (branch `scarlett-fox`), state of 2026-11-26

This branch is codec-d/cool_fox at `f4b48ad` (itself avatarmanz/VIctory-7900XTX `0a43566` plus six commits) with the
work of a local fork ("Scarlett") ported onto it. Target PC: Ryzen 7 9800X3D, 32 GB RAM, RTX 4080 SUPER 16 GB,
2560x1440, Series X mode. Goal: a stable 30 frames a second with no loss of picture. It is not there yet.

**This repository contains no game files, code or assets.**

## Measured (6-minute opening route, diagnostics build)

| | cool_fox as it was (floor) | this branch, best settings |
|---|---|---|
| ms per frame, 1,000-2,500 draws | 50-55 | 41 |
| ms per frame, 2,500-4,000 draws | 50-53 | 50 |
| ms per frame, over 4,000 draws | 59-66 | 61 |
| frames over 1.5x the usual | 7-8 % | 4.3 % |
| 5-second periods with a frame over 500 ms | 18-21 | 11 |
| frames drawn in the 6 minutes | 4,400-5,900 | 7,256 |

A heavier hand-played scene (5,500-8,600 draws) runs at about 70 ms. Memory commit is about 36.5 GB.

## Best settings so far

cool_fox's launcher set without `VICTORY_THIN` and without the stand-in ambient light, plus three switches:

```
VICTORY_RELAY=2 VICTORY_WIDE_BACK=1 VICTORY_RELAY_COPY=1 VICTORY_CARD_STEPS=2 VICTORY_INDIRECT_BATCH=2
VICTORY_MAP_THREADS=1 VICTORY_CARD_AWAKE=1 VICTORY_SHOW_AFTER=1 VICTORY_JOIN_PARTS=8 VICTORY_UPLOAD_AHEAD=1
VICTORY_CARD_BUFFERS_MIN_KB=64
VICTORY_POOL_DISCARD=1 VICTORY_FILL_TITLE=1 VICTORY_PARTIAL_DELTA=1
```

Build: `cargo build --profile diag -p victory --features diagnostics --locked`.

## What was added to cool_fox

Always on:
- Unattended runs: `VICTORY_AUTO_INPUT` (a script of pad presses keyed on draws per frame, to pass the start
  screens), a window caption with fps, ms and draws, `VICTORY_FRAME_LOG` (per-frame breakdown by thread, long frames
  listed with their cause), `VICTORY_HANG_WATCH_S`, `VICTORY_PRESENT_SKIP` for runs with the display off.
- Picture: 16-bit float values moved through unsigned-integer views (`VICTORY_HALF_VIEWS`, `VICTORY_D16_BITS`; fixes
  frozen facial animation and a blurred disc in cutscenes), a NaN guard on colour exports (`VICTORY_NAN_GUARD`),
  early depth, pipelines made for the render-target formats actually bound (`VICTORY_TARGET_VARIANTS`), an alias copy
  after an upload earlier in the same list, views keep the title's minimum LOD. 26 compute programs the base refused
  now translate (hair shadow masks, water volumetrics, particle emits, decals).
- Meters (diagnostics builds): uploads by cause, read waits by reason, mapping-lock readers, memory housekeeping,
  texture detail, the game's chunk pool.

Switches, off by default (the base's behaviour is kept unless a run showed a gain):
- `VICTORY_POOL_DISCARD=1` — unused pool pages are given back when free RAM is low; halves the cost of the game's
  page-mapping calls and the number of freezes. In the best set.
- `VICTORY_PARTIAL_DELTA=1` — when more of a texture maps, only the levels and slices that differ from the last
  upload are sent again. In the best set.
- `VICTORY_FILL_TITLE=1` — cool_fox's own switch; removes 15-24 ms a frame of waits to read title memory behind
  flights. In the best set.
- `VICTORY_MAP_DEFER`, `VICTORY_REMAP_SAME`, `VICTORY_DSTORAGE_OLDEST_FIRST`, `VICTORY_DSTORAGE_WORKERS`,
  `VICTORY_AFFINITY=free`, `VICTORY_HEAP_PACK`, `VICTORY_STAGING_SPARE_BLOCKS`, `VICTORY_REFRESH_ONCE` (+`=verify`),
  `VICTORY_CHUNK_FINAL`, `VICTORY_CHUNK_COUNT`, `VICTORY_BIND_FILTER`, `VICTORY_PREPARE_WIDE`,
  `VICTORY_WAVE_PAIRS_FIRST`, `VICTORY_ONE_MODE_PER_REGISTER`, `VICTORY_HOIST_LOADS`: ported or new; no gain shown
  yet, or untested. Tried and rejected on this base: `VICTORY_UNMAP_EXACT`, `VICTORY_TEXTURE_RAM_CAPS=0`,
  `VICTORY_VRAM_RAW_PCT`.

## Known problems

- Bushes and some trees glow purple; purple patches linger for a few seconds in some scenes (present in the base).
- Texture pop-in: about 46 GB of texture copies are let go and 10 GB made again in six minutes; the card's 64 MB
  pool blocks hold about 3 GB of holes.
- About 80 s of a 6-minute run is spent in frames over 150 ms: mostly the game's page-mapping calls under low free
  RAM, and bursts of texture uploads.
- About 820 draws a frame are dropped for two pipeline creation methods that are not implemented; ray-tracing
  acceleration structures are not built.

## Where things are

- `tools/` — the run scripts (`run-auto.sh`: hands-off 6-minute run with watchdog; `run-compare.sh`,
  `run-freezes.sh`, `run-focus.sh`), notes (`tools/docs`: what was measured, tried and rejected; `fox-port-*.md` list
  every ported switch), `tools/baseline-coolfox/README.md` (the floor and how to compare against it).
- The older Scarlett tree (soldsawn/victory-nvidia-8gb base) is on branch `scarlett`.

The base's own README follows.

---

# VIctory on an RX 7900 XTX

## Run the codec_d Windows build

Download and extract the Windows x64 ZIP from the [private releases](https://github.com/codec-d/cool_fox/releases/latest). Open PowerShell in the extracted folder and point the launcher at your own complete Xbox dump.

```powershell
$env:VICTORY_GAME_DIR = 'D:\Games\MyDump'
.\Play-VIctory.cmd pareto-residency
```

The launcher selects Series S and the RTX-tested residency settings. It builds its shader cache locally under `%LOCALAPPDATA%\VIctory\cache-pareto`. Set `VICTORY_CACHE_DIR` to reuse an existing cache outside the checkout. Windows x64, an AVX2 CPU, and a Direct3D 12 GPU with shader model 6.6 and mesh shader support are required.

To build from this checkout, install the pinned Rust toolchain and the MSVC C++ build tools with a Windows SDK, then run `cargo build --release --locked -p victory`. The same launcher finds `target\release\victory.exe`. The sections below describe the upstream AMD fork and its separate launcher.

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
