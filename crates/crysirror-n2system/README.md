# N2System / NsScript

Mode: **adopt**. This crate owns NSS parsing, execution, native names, argument
policy, waits and save layouts. CrySirror owns image processing, retained scene
composition, text, UI, audio, video, storage and presentation. It has no private
renderer or window loop.

`Config` selects the entry module, script root, encoding, canvas, font bindings,
content platform and caller-owned archive material. Game files remain in their
original mounted layout. Archive keys and user assets are not distributed.

| Area | Implementation |
| --- | --- |
| NSS execution | `source.rs`, `program.rs`, `vm.rs`, `array.rs`; lowered instructions are internal IR, not original binary opcode numbers. |
| Images and surfaces | `adapter.rs`, `adapter/native_bitmap.rs`, `adapter/surface.rs`, `adapter/stencil.rs`; client-owned shared resources and render targets. |
| Motion and cameras | `adapter/graphics.rs`, `adapter/camera.rs`, `adapter/blur.rs`; retained motion, paths, inherited camera projection and delayed poses. |
| Cube scenes | `adapter/cube.rs`; six immutable faces and native pose policy over shared homogeneous mesh projection and GPU snapshot composition. |
| Bitmap processing | `adapter/frame_image.rs`, `adapter/bitmap_pair.rs`, `adapter/shade.rs`, `adapter/ripple.rs`; shared asynchronous image jobs. |
| Text and glyph preparation | `adapter/text_host.rs`, `adapter/font_cache.rs`, `text.rs`; native markup/layout policy over scoped fonts and shared glyph workers. `LoadFont` retains non-drawing cache entries. |
| Input and UI | `adapter/choice.rs`, `adapter/scrollbar.rs`, `adapter/backlog.rs`, `adapter/system.rs`. |
| Audio and video | `adapter/media.rs`, `adapter/modal_movie.rs`; scoped decoder/transport services. `MoviePlay` holds all script threads and restores previously running audio/video after playback. |
| Persistence | `adapter/storage.rs`, `native_save.rs`, `adapter/native_file.rs`; original NPSG/NPSL variable tables and shared NPSF packets, including indexed values and associative labels. `Save(0)` writes native common variables; the JSON sidecar retains portable read history. |

Desktop and console hosts use the same shared scene/render services. No native
console build or device behavior is established by desktop compilation.

Encoded NPSF files use caller-owned material. `archive_material.npf_permutation`
can name an explicit 256-byte inverse table in the asset mount. Otherwise this
profile accepts only a mathematically unique inverse derived from the supplied
effective NPA permutation. Ambiguous tables require explicit material. Packet
substitution and bounded zlib expansion live in `crysirror-resource::npf`.

## Reference identity

- NSS/reference VM: CommitteeOfZero/nitrosharp
  `e3869842bb5bdd5ed95344027297620cd54454a6` and legacy
  `36701328b3167a2eef754a624b73b7d1e54fb2bb` (MIT).
- Additional reference implementation: Aether N2System
  `77a93db4a6e83cddd60a969476b3e1239f68092c` (MIT). Its private checkpoint
  envelope is not an original N2 save format.
- Primary native findings: authorized Totono trial 1.0.0.0, SHA-256
  `59c9afd545a492e0d14806baaa3adc1ce35b6da6a6e4a5f9a0c778a4866e605c`.
  Function addresses and recovered contracts are recorded beside their handlers.

## Remaining boundaries

The catalogue contains 154 reference call names, including compatibility
helpers. A dispatch branch does not establish every native subtype or behavior.
The following handlers are connected in source; compilation is separate from
observed game behavior.

| Call / area | Connection and native policy |
| --- | --- |
| `SetComposite(query,x,y,source)` | Shared immutable bitmap composition, with clipped RGBA copy into visual/resource/surface destinations. This is an explicit compatibility extension: trial 413410/457730 writes incrementing process-address values into a quarter-width loop instead of reading source pixels. The portable copy is not equivalent to that buggy native result. |
| `DrawLegacyTransition(query,time,mask,masked,wait[,dynamic])` | Aether-only translator helper, not an original NSS command. Duration is quantized to 128 ms, with 128-step cumulative mask coverage or incremental dynamic feedback. Uses detached shared Layer captures and shared rule-table image transitions; one active transition at a time. |
| Nested / rotated cameras | Nearest camera ownership replaces the enclosing camera, with compensated parent translation. Trial 4534C0/492AC0 subtracts camera Euler channels before the sprite's model rotation, through shared `CanvasLens.model_rotation_offset`; it does not rotate the completed view matrix. Position/rotation tracks retain independent waits. |
| Alpha movie planes | Trial 472B80/473AB0: top-half colour plus lower-half green-channel alpha, with half-height display geometry. Shared media decoder workers unpack frames before publication; seek, loop and restart retain the layout. |
| `SoundEffect` | Echo, Chorus and WavesReverb, plus native removal for other names (454470). Chorus maps wet/depth/feedback/delay with sine 1.3 Hz and +90-degree stereo phase; WavesReverb uses the native fixed gain/decay/HF settings. Shared streaming DSP retains bounded history, is replaced live and reconnected after preparation/load. Chorus/reverb use portable DSP, not sample-identical proprietary DirectSound algorithms. |
| `SetTone` | Monochrome, Sepia, KitanoBlue and NegaPosi through shared immutable image workers. The first operation retains a backup, later operations change current pixels, and NULL restores/releases the backup. Uses native selector byte sums and 46E7B0's truncated 77B/151G/28R luma tables; unknown hashes leave the bitmap unchanged. |
| `LoadFont(size=0)` | Maps to a portable 16-pixel default cell independently of dialogue size/HDPI. Original 444680 passes zero to the Windows font mapper; its installed-face-dependent metrics are not a fixed cross-platform value. [LOGFONT contract](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/ns-wingdi-logfonta). |
| Plain reconstruction / effect replacement | Plain/colour callbacks can be rebuilt over a retained visual. Prefix captures exclude the callback's own output and later overlays. Switching callbacks cancels obsolete captures/process jobs, freezes the current source for bitmap effects, and rebuilds the full-frame source view when returning to Plain. |

`DrawEffect` has Plain, Ripple, Mosaic, LowWave/MiddleWave/HighWave/SuperWave
and the four frame-colour callbacks (NegaPosi 443C10, Monochrome 4435C0, Sepia
4439D0, KitanoBlue 4437C0, over shared `invert`/`fixed_luma`/`channel_lookup`,
requiring a Plain capture). Bitmap effects share one source/progress/restore
path (`adapter/ripple.rs`): Mosaic uses `full_block_average` (433630), the waves
use `row_shift` (433E80, 32 ms ticks, sine table, amplitude `(w>>1)*amount/1000`).
`CreateEffect "Lens"` (4368F0/443DE0) samples the layer prefix below its own
order through shared `lens::prepare`/`FieldWarpSpec::Lens`.

`DrawEffect` also covers SoftSplit/Split/HardSplit (4/8/12 tiles per side;
alternating downward 2..6 and upward 4..12 speeds, group slide `(canvas
height - y) * amount / 1000`) and TurnOver (32 x 32 tiles, columns flipped by
the eased 0..64 counter, +4 px and +4 degrees per step, angle capped at 180)
through shared `tile_compose`. `CreateEffect` covers Rain (128 streaks, 196-point
alpha ramp, speed table 152..64), Drop (256 sprites, speed/swing/rotation
variables), FloatBubble (16 `data/particle.png` bubbles) through
`streak_field`/`tile_compose`. The original seeds `rand()` from wall-clock time,
so a deterministic generator seeded from the entity id is used: motion is
statistically equivalent, never frame-identical. Documented approximations:
the Split velocity pairing, TurnOver's flip axis and the unlabelled per-tile
fields (depth/shade at +204/+208), Drop's z-rotation and Bezier swing, and the
native speed unit (path points per 16 ms) come from decompiled arithmetic, not
from observed frames.

`Spiral` is an explicit compatibility extension: 4344C0 installs the wave
updater 433E80 over an incompatible circle-list state. Instead of following
those invalid accesses, shared `FieldWarpSpec::PolarTwist` rotates concentric
source pixels around the bitmap centre. Amount 0..1000 is zero to one centre
turn, tapering linearly to zero at half the shorter image edge. It follows the
existing eased duration, restores the original at a zero endpoint and retains
the final image at a nonzero endpoint. This is not an official native result.

`LookupRain` follows 4390D0/439360: it allocates its internal stripe with
46EC10(1,16,-1,0x80), whose four-pixel row alignment makes it 4x16. The packed
colour is `-1 OR (0x80<<24)`, so the actual pixels are opaque white. 491B80
binds this bitmap through ESI; there is no external texture name. One initial
random Z angle is retained, and the canvas-centred strip is cleared/redrawn
every 32 ms without advancing that angle. Shared `tile_compose` handles rotation
and coverage. The portable rasterizer/random sequence is not a pixel-parity
claim against the Windows renderer.

Remaining boundaries include some bitmap/mask combinations and original-save
resource persistence described below. Portable font coverage does not promise
byte-identical Windows GDI rasterization. The handlers above have scoped release
compilation evidence; their real-game frames, audio and route behavior have not
been validated in this change.

Slots 1..9999 use the CrySirror checkpoint of ADR 0104
(`n2system-v1/slots/NNNN.json`); original slot files are raw process-memory dumps
and are not imported or exported. `Save(n)` writes script threads, variables,
entity tree, retained layers, text pages, backlog, choices and sound parameters;
`Load(n)` validates and prepares modules on a worker, then commits once at the
frame boundary and continues after the saved `Save`. Scenes showing
runtime-generated bitmaps (captures, surfaces, effect output, movies) are
refused at save. Thumbnails and generated-bitmap persistence are not stored.
Framework `snapshot`/`restore` remains explicitly unsupported.
