# PS Vita native shader ABI

These programs use GXM Cg/GXP. The canonical shared 2D WGSL is lowered through
Naga's typed HLSL representation into Cg. HLSL is a build intermediate only.
There is no OpenGL/vitaGL runtime. Reduced 3D and software BVH ray tracing use
engine-owned Cg sources.

Compilation records and named bindings belong in `manifest.json`. GXP programs
are exposed by `crysirror_shaders::native::PSV_PROGRAMS` with 16-byte alignment.
A compiled shader catalog does not implement the shared GXM renderer or certify
pipeline linkage, pixels, performance or hardware behavior.

Native GXM validation admitted all 237 programs and linked all 218 fragment
programs. Representative offscreen pixels currently pass 6 of 10 checks:
presentation, plain quads, hardware-sampled filters, kind-0 transitions, basic
mesh lighting and unoccluded BVH direct lighting. Four encoded/byte filter
sampling cases still fail. See the
[observed results](../../../../docs/validation/psv-shared-shaders-2026-11-15.md#native-gxm-validation).
Compilation success must not be used as visual acceptance.

| Rust API | Purpose |
| --- | --- |
| `PSV_PROGRAMS`, `psv_program(name)` | Aligned compiled GXP catalog and name lookup. |
| `PSV_MANIFEST` | Compiler receipts, hashes, source routes and named bindings. |
| `psv_shared_sources()` | Expanded native Cg for the shared 2D programs. |
| `psv_lite_sources()` | Expanded reduced 3D Cg sources. |
| `ray_lite_sources("psv")` | Expanded native BVH vertex/fragment sources. |
| `psv_quad_filter_program(entry, distortion, flags)` | Select the original filter's distortion/sampling specialization without allocation. |
| `psv_wipe_program(kind)` | Select the original transition specialization without allocation. |
| `PSV_LOOP_LIMITS` | Required loop counts for radial, convolution and explosion sampling. |

## Shared 2D entries

| Canonical module / entries | Native operation |
| --- | --- |
| `quad` · `vs_main`, `vs_plain` | Original clip-space vertices, UVs, tint, sample bounds and projective clipping. |
| `quad` · `fs_main`, `fs_premultiply` | Texture sampling with clip bounds; straight or premultiplied output. |
| `quad` · `fs_filtered`, `fs_filtered_premultiply` | Native distortion, byte sampling, tone curves, luminance, colour filters, masks and alpha policy. |
| `quad` · `fs_stored` | All twenty stored-byte blend profiles and original output-alpha rules. |
| `present` · `vs_main`, `fs_copy`, `fs_decode` | Fullscreen copy and encoded-byte decode before sRGB presentation. |
| `puppet` · `vs_part`, `fs_part`, `fs_mask`, `vs_full`, `fs_unpremultiply` | Part meshes, premultiplication, alpha-tested stencil mask and final straight-alpha conversion. |
| `wipe` · `vs_main`, `fs_main` | Original shared transition modes, masks, moves, scales, mosaics, raster and sixteen-tap explosion. |
| `ordered_dither` · `vs_main`, `fs_main` | Bayer cell coordinates and explicit byte-space rounding. |
| `composite` · `vs_main`, `fs_main` | Crossfade, universal-rule weights, byte blur blend and premultiplied colour conversion. |
| `composite_filter` · `cs_main` for both output formats | Equivalent GXM fragment passes, described below. |

The 23 canonical entries retain their operations. The three filter fragments
have dedicated sampling/distortion combinations to keep native compiler
expansion and shader cost bounded. Select them with
`psv_quad_filter_program(entry, distortion, flags)` using the original
`LayerFilter.d.z` enum and `LayerFilter.c.w` flags:

| Distortion | Operation |
| --- | --- |
| 0 | None |
| 1 | Mosaic |
| 2 / 3 | Horizontal / vertical raster |
| 4 | Ten-tap radial blur |
| 5 / 6 | Threshold / inverted threshold fade |
| 7 / 8 | Horizontal / vertical wave |
| 9 | 3x3 convolution |
| 10 | Grid sampling |

| Sampling | Original flags |
| --- | --- |
| Hardware sampler | bit 128 clear, bit 32 clear |
| Encoded point | bit 128 clear, bits 32 and 64 set |
| Encoded bilinear | bit 128 clear, bit 32 set, bit 64 clear |
| Integer-byte point | bits 128 and 64 set |
| Integer-byte bilinear | bit 128 set, bit 64 clear |

Each variant retains address, transparency and other effect flags. Selection
changes neither sample counts nor quality. The host rejects unknown distortion
enums rather than selecting a different effect. Integer-byte RGB interpolation
uses exact F32 integer arithmetic: RGB values are 0..255 and products are at
most 32640 in magnitude, well inside F32's exact integer range. Division by 128
is exact and `floor` matches the original arithmetic right shift. Alpha keeps
its original 32-bit integer path. Bilinear neighbours share address calculations
without changing clamp/repeat/mirror behavior.

Transition fragments specialize the original kind: 0, 1, 2, 50, 200,
210–215, 220–221, 230–231, 240–243, 900 and 901. Mask kinds 5–199 except 50
share kind 900's identical branch. Other kinds retain the canonical default
crossfade. `psv_wipe_program(kind)` performs this routing; keep the original
uniform and all options. The 22 transition variants and 165 quad variants give
206 native 2D programs for the 23 canonical entries. Together with 31 native
3D/ray programs, the compiled catalog has 237 programs.

The wipe vertex stage also reads `wipe` and evaluates the original seed-based
random centre on the GPU. It passes the draw-constant result at `TEXCOORD1` to
the random-centre explosion fragments. Bind the same wipe record in both
stages. This moves uniform arithmetic out of the fragment loop without changing
the seed formula or reducing taps.

## Binding and lifetime contract

| Input/state | Host requirement |
| --- | --- |
| Vertex inputs | Bind the generated `ATTRn` parameters by reflection. Fullscreen entries consume a float vertex index 0, 1, 2 at `ATTR0`. Other layouts follow the original WGSL locations. |
| Fragment position | Configure a top-left viewport/pixel-centre convention matching the original WGSL `position.xy`; fragments use native `WPOS`. |
| Texture bindings | Use the manifest's texture-unit mapping and named `cg_size_<texture>` float4: width, height, reciprocal width, reciprocal height. Dimensions must be positive. |
| Sample/color state | Retain each canonical texture's sRGB/raw/linear role, address and filtering modes. Point loads use texel centres and explicitly return zero outside the texture. |
| `selected_filter` | Bind one complete original `LayerFilter` record per draw. Split batches whenever its flat slot, clip or sample-bound values differ. The native helpers read that immutable uniform directly. |
| `cg_loop_limits` | Bind exactly `[10, 3, 3, 16]` with the reflected integer type. This retains radial taps, blur rows/columns and explosion taps while preventing compile-time expansion. These are mandatory constants, not quality settings. |
| Integers | Shared unsigned inputs are bounded dimensions, flags, byte weights and box sums below `INT_MAX`. Cg uses signed 32-bit arithmetic. Arbitrary division uses a biased float estimate followed by integer remainder correction, including signed extrema. |
| Blending/stencil | Preserve the original pipeline's straight/premultiplied blend policy. Puppet mask passes write stencil where alpha is at least 0.5, with colour writes disabled. |
| Ownership | Retain GXP memory, patched programs, textures, uniform buffers and render targets through GPU completion. Perform resource replacement/retirement on the owning host thread. |

Uniform fields and original bind-group/binding identities are recorded beside
the native texture slots in the manifest. Use the GXP parameter reflection for
native register offsets and types; WGSL byte offsets are not GXM registers.

## Separable box filter

GXM has no compute stage and no four-channel F32 colour target. Preserve the
horizontal integer sums with two F32x2 targets with matching logical RG texture views:

1. Bind `params.config.x = 0`, original horizontal radius/mode/rect, and
   `params.extra.y = 0`. Store the horizontal RG sum in the first F32x2 target.
2. Repeat with `params.extra.y = 1`. Store BA in the second F32x2 target.
3. Bind the RG target as `source`, BA as `horizontal_ba`, and the original
   vertical parameters (`config.x = 1`, `config.w = 0`). Use integer vertical
   accumulation/division, then write the requested RGBA8 or RGBA16F output.

Use point sampling, no blending/gamma conversion/MSAA for the sum targets.
Keep the original positive dimensions, rect, radius limit 128 and pass order.
The two-channel intermediates retain integer sums exactly; half-float
horizontal targets would lose byte precision. Bind `cg_loop_limits` only when
present in the compiled program.

## Reduced 3D and BVH

The `lite_*` programs retain the existing `crysirror-render-tier::Tier::Vita`
policy: PBR factors, two shadow cascades, four selected lights, two reflection
probes, SH/volume lighting, skinning, particles, VFX, bloom and tone mapping.
CPU-generated BRDF/probe data and substituted SSAO/SSR/TAA/VSM paths remain
explicit tier decisions. See [console shaders](../../../../docs/CONSOLE_SHADERS.md).

`ray_lite_v/f` consume the existing `crysirror-raytrace::PackedScene` stackless
BVH ABI. Bind nodes and triangles as point-sampled RG32F textures, dimensions
and counts in `ray_nodes` / `ray_triangles`, and the named camera/light/sky
vectors declared in `ray_lite_f.cg`. The fragment traces complete preorder /
escape links, up to four triangles per leaf, nearest visible hits and a
secondary directional-light shadow ray. It retains the native ray bias and
uses triangle colours, Lambert lighting, ambient light and sky; this is the
reduced software-ray profile, not hardware ray queries or the desktop PBR RT
pipeline. Empty scenes/counts must follow the packed-scene admission contract.

Regenerate and compile through [the shared compiler utility](../../../../platform/psv/shaderc/README.md).
