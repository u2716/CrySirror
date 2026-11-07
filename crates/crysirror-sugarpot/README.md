# SugarPot / Tsukumo-2012

Mode: **adopt**. This is a first-class workspace implementation of the text
language observed in the original 2012 *Tsukumo no Tsuki* executable. SugarPot
identifies this observed native profile; AOS2's LiLiM resource-family attribution
alone does not establish that other LiLiM games share this VM.

`program` retains CP932 source positions, section/local labels and macro bodies.
`machine` owns native values, calls and control flow. `adapter` translates staged
sprites and `ud` publication into engine Layer operations, button waits into
engine UI widgets, default instant text into engine grid layout/GlyphRun, and
sound requests into engine-owned preparation receipts and channels. `resources`
uses bounded client storage on one engine worker. Construction is dormant;
resource workers start at the preparation fence. No original executable, native
plugin, renderer, composed-screen upload or audio device is hosted.

`Config` requires `script`, `canvas`, `assets` and `font`. The admitted canvas is
1024×600, entry is the original `start.scr`, assets map `scr/`, `grp/`, `bgm/`, `se/`, `cv/` plus lower-case native
member names to mounted client asset paths, and font is an authorized project alias.
AOS2 archives use shared `lilim/aos2`; scripts use shared `lilim/scr`; image
preparation uses the shared ABM/BMP registry. Original image precedence is
ABM, BMP, DAT within `grp/`. Ogg/WAVE use shared audio; BGM uses `bgm/`,
ENV/SE use `se/`, and character voice uses `cv/`. Banks never overwrite each
other when members share a basename. Resource names cannot escape the client
root. Native `system.dat`, `global.dat` and `readal.dat` use their original codecs
through scoped storage tasks. Missing preferences install native defaults;
malformed files fail. Global mutation waits for atomic write completion. The
native slot codec is available internally, but adapter slot reconstruction and
its separate thumbnail are still incomplete; framework snapshots fail explicitly.

The original command-name/function tables are at VA 0x4a3cd0/0x4a38d0 in the
owner-provided Tsukumo-2012 executable. Relevant native bodies are 44C880
(control/value dispatch), 44FAF0/450150/450670 (source loading/indexing/macros),
447090/447180 (sprite declarations), 44AA40/44AB00/44AC30 (character operations),
44AD40/44AE50/44AFC0 (background/CG), 449870 (update), 446220/445F40 (window and
backlog declarations), 44B1E0/44B410/44B4B0/44B500/44B600 (buttons),
445DF0 (clock-seeded rand), 447AC0/447C90/447EE0/447F40/45B130 (audio), and
41F0E0/453F00 (fresh-profile preferences). These are native virtual addresses,
not archive offsets. Static analysis and derived proprietary resources remain
in the ignored local workspace; no game assets or binary code are shipped.

The implementation is original Rust, under the workspace licence. It reuses
existing shared format plugins with their retained arc_unpacker GPL-3.0-or-later
notices; their code has not been copied into this VM. The original game's
Xiph/Ogg notices apply to its distribution; its DLLs are not linked here.

This remains a partial native implementation. Source includes all 17 control
selectors and macro calls, scalar/list commands, staged sprites/buttons, shared
text pagination/reveal, destructive mono tables, EFEX movement requests, default
movie/AVI playback and window fades. `ud(2,mask,time)` uses shared immutable scene
captures and the explicitly selected scalar stored-mask profile. It preserves
native threshold timing, including the first phase of 1 and half-duration end;
it does not claim MMX byte arithmetic equivalence. Original identity-palette
BMP8 masks are validated before shared decoding.

The portable value profile bounds `max/min` to initialized arguments (repairing
the native best-index/scan-index overrun). Float `str` retains the native
float-bits-as-integer conversion and zero fractional value, while encoding the
fullwidth dot once instead of widening its CP932 bytes into an undersized buffer.
These fixes are explicit compatibility policy, not recovery of undefined memory.

ABM1/3 use the public `lilim/abm-frames` archive plugin: ordinary PNG frame
entries and a timeline entry are available independently of this VM. Original
sequence indices are retained; image publication, cancellation and decoding use
shared workers. The portable presentation clock uses the native 33ms paint tick;
`anmnum` preserves its boundary wait and 150×64ms upper bound.

`float(4)` owns the native CHMode modal lifetime over shared retained layers,
widgets and asynchronous image resources. Raw `scr/hmlist00..09.dat` records
supply unlock keys and result IDs; the GRP bank supplies `hm_bkN.bmp`,
`hmNPP.bmp`, `hm_cp.abm` and `hm_no.abm`. Native category/page/thumbnail
rectangles and three-state atlas coordinates are retained. Global-node presence
unlocks an entry, independently of its value. Selection returns the record's
ID in `reg`, stops BGM and resumes the caller's `MODE_H` dispatch; cancellation
returns -1 and retains BGM. Both exits clear native history/text state and stop
voice. Shared crossfade presents the close ramp; exact native palette-step
quantization and the native /256 alpha-composition profile remain visual gaps.
Thumbnail RGB tint uses the exact normal/hover/press /256 lookup through shared
image workers; old generated page leases retire after replacement.
This implementation has not received new behavior validation.

`float(3)` reads the distinct CG-list format (48-byte variant records and two
trailing counts), groups consecutive variants by native group ID, and retains
per-variant global unlock state. Its viewer uses native BMP-before-ABM priority,
shared ABM frame resources, 33ms/4px vertical pans and independent next/back
waits. Exhausted unlocked variants return to the album; album exit preserves
caller reg/history/BGM. Category/page controls share the byte-identical native
CHMode table. The same palette/alpha visual-profile gaps apply.

All UD kinds 1..39 of 449870/435AB0 are implemented. UD19/21/23 apply the
40FAE0/410830 zoom helpers to the old stage over the new one; UD25/26 are
410B10 midpoint-circle irises (shared RectMap masks); UD32 is 412E20's row
mean whose window runs one pixel past the row (shared `TruncatedRowBlur`);
UD37/38 are 4137D0/413980 random runs drawn from the process C runtime
`rand()` stream, which `rand` reseeds from the clock (shared `RandomRuns`);
UD39 is 411190's 1024 moving discs (shared `DiscCopies`) whose tables are
read from the identified executable at VA 4A72B8..4AA2B8, not copied here.
During a UD, an active fall and filter are re-applied to every presented
sample in native order (43F430: 414A10, then 415400) while the unfiltered
sample remains the accumulation base; an active raster distorts the new
stage once below its order-999 boundary (4383B0). `filter` loads only
`grp/<name>.abm` (415740). Mono/sepia copies made by `tone_image` are
retired with the decoded-image cache rule.

Incomplete paths include the slot-menu save/load UI (quick save/load is
exercised; see docs/validation/sugarpot-2026-11-07.md), BGM
album dialogs and remaining settings controls, remaining EF/raster/particle/filter
families, nondefault movie flags, full text escape/novel/backlog policy and
animated/percentage-add gauges. Unsupported branches fail with their native
script position. Named native dummy slots are explicit no-operations and never
serve as a fallback for unknown commands.

| Target | State |
| --- | --- |
| macOS / Windows / Linux | Source implementation over the existing host services; release compilation and actual-game behavior require separate validation |
| iOS / Android | Not connected; packaging, touch, suspend and storage policy unverified |
| Browser | Not connected; injected resource/catalog boot and worker/output lifecycle unverified |
| Vita / Switch | Deferred engine host/task/graphics/audio backends |

No original executable has been executed. Compilation and behavior results must
be reported separately from these source mappings.
