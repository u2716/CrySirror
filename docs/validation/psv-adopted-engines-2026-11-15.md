# PS Vita adopted engines on the shared renderer, 2026-11-15

Engines other than Siglus/BGI run the shared runtime and the unchanged shared
renderer on [`crysirror-wgpu-gxm`](../../crates/crysirror-wgpu-gxm/README.md).
All results below come from Vita3K v0.2.1 4340-da5d2e5a (Linux, Vulkan,
offscreen, audio disabled). Nothing here has run on Vita hardware.

## Harness

`run_game.py` installs the release VPK and links one installed game under
`ux0:data/CrySirror/games`. It presses Enter every 120 frames and dumps the
displayed frame at chosen frame numbers. With a dump requested, the driver
also writes every resident colour image (`<frame>.images/`). The default font
for these runs was DroidSans (Latin) merged with DroidSansJapanese and a
DroidSansFallbackFull subset (CJK punctuation, fullwidth forms). That font
was built on the test machine only and is not distributed.

## Defects found and fixed

| Symptom | Cause | Fix |
| --- | --- | --- |
| Link error `sena_vita_image_allocation` | Softpal called its GXM driver's diagnostic without that driver linked | Call only with `vita-gxm` |
| KiriKiri heap exhausted (320 MiB used) | Self-copies (`copyRect` from a layer onto itself) duplicated the layer's lazy image-history graph in both branches, doubling each time (13 copies: 6.3 M nodes) | Shared bound `IMAGE_GRAPH_MAX_NODES/DEPTH`; KiriKiri drops a history past it (it could never be admitted) and recovers through canvas capture |
| Motion textures resident three times | `upload_rgba` copied and PNG-encoded the model's own pixels | `publish_image_resource(RgbaImage)` shares the model's `Arc` |
| ~24 MiB of task queues | Bounded channels preallocate every slot | Unbounded queue; `reserve` still admits `capacity` jobs |
| Fonts resident once per face | `ab_glyph::FontVec` copies the file per face | One interned copy per distinct font file |
| `Rgba16Float` puppet target refused | GXM colour targets are RGBA8 | Driver reports renderable formats; puppet falls back to `Rgba8Unorm` |
| 256-byte buffer allocation failed | One kernel memblock per buffer | 1 MiB sub-allocated arenas for buffers ≤ 64 KiB |
| 1080p targets exhausted CDRAM | vita2d allocated a depth/stencil surface per render target | Own render targets; depth only when a pass binds it |
| 595 textures pinned after a burst | Retirement advanced only on present | Reclaim retired resources and retry on allocation failure |
| Every frame black | Fullscreen passes read `cg_vertex_index : ATTR0`, which was never fed | Extra U16 stream over the sequential index buffer |
| E-mote parts invisible | Struct vertex inputs keep `TEXCOORDn` semantics | Fall back to the `TEXCOORDn` attribute parameter |
| Readback of render targets read zeros | Vita3K writes a surface back only when a later scene begins on it | One empty scene on the target before CPU reads |
| Circus "not found" | Case-sensitive host lookups (`circus2.CRX` vs `Circus2.CRX`) | Asset mount resolves components case-insensitively |
| Vita3K terminated (KiriKiri) | Stat of an over-long XP3 protection-dummy name raised an uncaught host exception | Components over 255 bytes are absent (capfs, VFS) |
| System-NNN stack overflow | 290 KiB constructor state moved by value; 2 MiB player stack | Boxed state; 8 MiB player thread |
| SugarPot not detected | std `off_t` is 32-bit on Vita; `grp.aos` is 2.8 GB | `crysirror-vita-io` 64-bit SceIo reads in capfs and launch |
| Blurred transitions failed on GXM (renderer error: no storage support) | No compute stage | Box filter fragment lowering; per-format `storage_format`/`blendable` capabilities |
| RGBA16F/RG32F textures half-allocated | vita2d sizes 64-bit formats as 4 bytes per pixel | Allocate as double-width U8x4 and re-describe |
| F32x2 surface held half-float data; F16x4 filter output sampled as zero (Vita3K) | Float colour surfaces in the emulator | Lowering uses U8x4 high/low-byte sums and an `Rgba8Unorm` output |
| Box filter output zero (integer division), then zero in row 0 / column 0 | Integer divide and integer `clamp` in the lowered program under Vita3K | f32 long division with operands below 2^24; f32 coordinate clamp |
| KiriKiri title illustration, footer and logo missing (desktop and Vita) | Skipping the title seeks the motion to `lastTime`; the type-0 end key held the state from before the skipped keys | A type-0 key reached by a seek holds the preceding key's state, as stepping would |

## Renderer self-test (Vita3K)

The `selftest-gxm` scene (8x4 strip: black, grey, white columns; box radius
1x0) gives red `[42, 127, 212, 255, 255, 255, 255, 255]` with alpha 255 in all
four rows, through the box filter lowering. This equals the desktop GPU
test's contract (byte mean 127, replicated edge 42). The plain capture of the
same scene gives black, grey and white.

## Ray tracing (Vita3K)

The `ComputeRaytrace` path on GXM now runs the lite fragment profile. The
self-test scene is 26 triangles at 64x36. Its 16 tile means (16x9 tiles)
equal those of the same profile on the desktop GPU (Apple Metal):
`[[40,60,90] x4, [42,62,91], [96,81,98], [96,80,97], [48,66,94],
[136,141,149], [165,152,152], [112,118,165], [113,122,134], [83,96,116],
[169,169,169], [155,158,168], [91,103,121]]`. The desktop compute profile
differs from the lite profile in one tile by 1 (`[164,151,151]`). Defects
found on the way:

| Symptom | Cause | Fix |
| --- | --- | --- |
| Floor missing, shapes distorted | `ray_lite_f` compiled at optimization level 2 | Level 1 (level 3 lost direct light; level 0 fails to compile) |
| (checked, not a defect) | RG32F data swizzle | `F32F32_GR`; all 216 node/triangle texels read back equal to the CPU build |

The GoW3 preview and the PBR/lite 3D programs are not connected to the GXM
renderer path yet. No game in the runs uses a 3D scene.

## Behaviour (Vita3K), by engine

The cooked shader index covers the seven modules these sessions create. The
only refused module is the simple 3D mesh module, which no 2D title below
drew.

| Engine | Title | Result |
| --- | --- | --- |
| WillPlus | OTOCHIKA | Title menu at frame 900 matches the desktop capture at the same inputs. 33 fps. |
| System4 | Rance 7 | Opening scenes with effects, 4:3 pillarboxed. 33 fps, heap 120 MiB. |
| KiriKiri | game-kr | Title at frame 5100: E-mote background and characters, badge and menu. Heap peak 235–260 MiB of 300 MiB. 6–7 fps. The earlier desktop reference lacked the characters (the end-key seek defect above); with that defect fixed, the desktop title at frame 5100 shows the illustration, logo and menu. |
| KiriKiri | haruno, syarin | Title screens at frame 1200. |
| KiriKiri | RiddleJoker, CafeStella | Run until a glyph absent from the test font (`※`). |
| KiriKiri | Sabbat of the Witch | Heap exhausted near 263 MiB. |
| KiriKiri | ExE, natsuzora, ousama | Unsupported plugin DLLs (`cxdec`, game-specific); adapter scope, not PSV. |
| CatSystem2 | Nekopara vol. 3 | Language setup screen. Heap 176 MiB. |
| CatSystem2 | Grisaia | Runs to frame 1200 with only clears drawn; heap 283 MiB. Desktop (Linux, same inputs) is also black at 600 and 1200. |
| Majiro | Akatsuki, Scarlett, Yukiiro | Title menus. |
| QLIE | QLIE-5 | Runs to frame 1200; near-black frame; 7 fps. Desktop (same inputs) is black at 600 and uniform grey 16.9 at 1200; Vita peaks at 16. |
| SugarPot | TSUKUMO | In-game text window at frame 1200. 4 fps. |
| System-NNN | MUGEN2 | Runs to frame 1200 (noise-effect screen). |
| AVG32 | Kanon | Title menu. 40 fps. |
| RealLive | AIR SE, Little Busters, After | Title menus. |
| Circus | DC3, DC3WY, DC4PHDL | Window-size prompt, title art, player-information form. |
| Circus | DC1, DC2 | Black at frames 600/1200, as on desktop with the same inputs. An earlier DC2 run that stopped on "not found" predates the case-insensitive asset lookup. |

Re-run after the box filter, float target and readback changes: OTOCHIKA,
Rance 7, Nekopara vol. 3, Akatsuki, DC3, TSUKUMO, haruno and Kanon reach
frame 1200 without driver errors. Shader captures of 12 titles over 1500
frames contain no blurred transition, so the self-test is the only box
filter evidence.

## Not established

- Hardware behaviour, timing or memory: only Vita3K was used.
- Pixel equality beyond the OTOCHIKA comparison. The other rows are visual
  checks of one or two frames.
- Audio: every run used `disable-audio`.
- Frame pacing. Vita3K fps is an emulator figure, not a hardware estimate.
