# crysirror-wgpu-gxm

GXM driver (PS Vita) for the wgpu custom backend in `crysirror-wgpu-deko3d`.
The shared renderer (`crysirror-wgpu`) runs on it unchanged, so every adopted
engine the console player opens (KiriKiri, System4, Circus, CatSystem2, ...)
renders through the same code as desktop and Switch. This crate is a host
graphics driver, not an engine integration; it has no onboarding mode.

## Boundary

- **Context**: vita2d's GXM context and shader patcher, which the integrated
  Siglus/BGI Vita renderers also use. vita2d owns display buffers and the
  final present; the shared renderer draws into an offscreen RGBA8 display
  image that `present` copies to the screen.
- **Programs**: no shader is compiled on the device. `cooked/index.json` maps
  the blake3 of each WGSL module the renderer creates to compiled GXP programs
  from `crysirror_shaders::native::PSV_PROGRAMS` (`library.rs`). Per-draw
  variants are chosen as `crysirror_shaders::native` specifies: transitions by
  the bound wipe kind, quad filters by the bound `LayerFilter` record.
- **Scenes**: every render pass is one GXM scene. Scenes are synchronous;
  `submit` waits for the GPU.
- **CPU work**: buffer/texture uploads and readbacks are CPU copies into
  GPU-mapped memory after the GPU is idle (no transfer queue on this path).
  No frame is composed on the CPU.
- **Vertex index**: GXM has no vertex-index builtin. The PSV lowering turns
  WGSL `@builtin(vertex_index)` into `float cg_vertex_index : ATTR0`. When a
  program declares it, the driver feeds an extra U16 stream over its
  sequential index buffer, offset like the vertex streams. The value then
  equals `first + i` for non-indexed draws and `index + base_vertex` for
  indexed ones. Every fullscreen pass (present/decode, transitions,
  composite, puppet resolve, dither) depends on this.
- **Vertex attributes**: matched by `ATTRn` semantic. Inputs the lowering
  declared as a struct (puppet `vs_part(i: In)`) keep the `TEXCOORDn` their
  `@location(n)` became. The driver then uses the attribute-category
  parameter with that semantic. A location with neither is logged once.
- **CPU reads of render targets**: `copy_image_to_buffer`, CPU image copies
  and diagnostic dumps first wait for the GPU, then run one empty scene on the
  display surface and two on the source target. Vita3K writes a colour
  surface back to guest memory only after such later scenes. On hardware the
  GPU wait alone suffices, and empty scenes leave the pixels unchanged.
- **Compute**: GXM has no compute stage. The only compute entry the 2D
  renderer creates, the separable box filter of blurred subtree transitions
  (`composite_filter.wgsl`), runs through its native fragment lowering
  (`crysirror-shader-tools psv`, `lowering: box_filter` in the index):
  - Horizontal dispatch: two fullscreen passes write the exact 16-bit RG and
    BA sums (`extra.y` 0/1) as high/low bytes into two U8x4 surfaces. These
    are the desktop's RGBA32F intermediate.
  - Vertical dispatch: one pass reads both halves and divides the exact 2D
    sums in f32 base-256/16/16 steps. All operands stay below 2^24, and the
    pass uses no integer divide. It writes the integer average into a U8x4
    output.
  - The GPU waits between passes, because each pass samples the previous one.
  - The renderer picks `Rgba8Unorm` for the filtered pixels when `Rgba16Float`
    storage is unavailable. Each value is a byte average / 255, so U8x4 holds
    it exactly.
  - Every other compute dispatch is refused and logged.

- **Hand-written programs**: lite rays (`ray_lite_v`/`ray_lite_f`) name
  their vertex inputs instead of using ATTRn/TEXCOORDn semantics. Their
  index entry carries an `attributes` map (location to parameter name), and
  their uniform block has an empty name, so members resolve as bare `ray_*`
  uniforms. `R32F`/`RG32F` textures are data and are always point-sampled.

## Memory

- Render targets are colour-only vita2d textures with a colour surface and GXM
  render target created here. A depth/stencil surface is allocated only for a
  target whose pass binds depth (for example puppet stencil masks).
  `vita2d_create_empty_texture_rendertarget` would allocate one for every
  target, doubling every 2D canvas.
- Buffers up to 64 KiB come from 1 MiB mapped arenas in power-of-two classes.
  The process can hold only a limited number of kernel memblocks, and each is
  at least 4 KiB. Larger buffers get their own memblock.
- Destroyed textures/buffers are retired for four presents (Vita3K reads
  resources on its own thread after the GPU wait). If an allocation fails, the
  driver waits for the GPU, frees everything retired and retries once. A burst
  of destroys inside a single engine step would otherwise hold memory until
  the next presents.
- Allocation failures log live textures, retired resources, buffers and free
  user/CDRAM/phycont memory through the host log (`gxm::set_log`).

## Capabilities reported to the renderer

`Driver::renderable` and `Driver::compute` feed the adapter's format features.
The renderer can then choose a supported format before it creates a texture:

- Colour render targets: `RGBA8_UNORM`/`RGBA8_UNORM_SRGB`, plus
  `RGBA16_FLOAT` (F16x4), `RG32_FLOAT` and `R32_FLOAT` surfaces.
- Blending: U8x4 targets only, because the shader patcher blends the UCHAR4
  output register only. For example, the puppet renderer therefore falls back
  from `Rgba16Float` to `Rgba8Unorm` premultiplied targets.
- Storage images (`storage_format`): `RGBA32_FLOAT` and `RGBA8_UNORM`, the
  outputs of the box filter lowering. These have no general compute behind
  them.
- 64-bit texture formats are allocated as U8x4 textures of twice the
  (8-aligned) width and then re-described. vita2d's size table counts F16x4
  and F32x2 as 4 bytes per pixel, so it would allocate half of each texture.

## Refused visibly (logged once)

Compute dispatches other than the box filter, constant blend factors,
blending into float targets, RGBA32F render passes, mip levels above zero and
array layers. A module with no native program is also refused:
currently only the simple 3D mesh module (`mesh.wgsl`). Its draws are skipped
and logged, not drawn wrongly.

## Regenerating the shader index

Capture the renderer modules of real sessions on desktop, without a GPU:

```sh
cargo run --release -p crysirror-console-player --example null_capture -- GAME_DIR OUT FRAMES [FONT]
```

Merge the `*.wgsl`/`*.entries` files of several captures into one directory
(union the entry lines), then:

```sh
crysirror-shader-tools cook-vita CAPTURE_DIR crates/crysirror-wgpu-gxm/cooked
```

The current index joins captures of KiriKiri (game-kr), System4 (game-rance7)
and WillPlus (game-OTOCHIKA). All three produce the same seven modules.
The quad v2 hash includes the existing GXP variants and the separate
continuous-mask and canvas-stencil fragments. GXM selects the mask
variants from the draw flags and retains the sampling policy.

`cook-vita` always adds the renderer's box filter and lite ray modules,
because the renderer creates them only when a blurred transition or a
compute ray-traced scene first runs.

## Self-test

The PS Vita player runs the renderer self-test instead of a game when
`ux0:data/CrySirror/selftest-gxm` exists. The self-test is
`crysirror_console_player::selftest`: the blurred-transition scene of
crysirror-wgpu's `subtree_effects` GPU test, plus a lit ray-traced scene
(floor and two cubes, 64x36), with no game data. It logs every scene
(`gxm::trace`) and compares the box filter with the desktop contract. It
also logs per-tile means of the ray image and saves it as
`selftest-rays.png`. The desktop reference comes from
`cargo run --release -p crysirror-console-player --example selftest_gpu -- OUT`.

Validation evidence is kept separately under `docs/validation/`.
