# Native console shaders

The shared shader library supplies canonical WGSL, native console sources and
compiled program catalogs. Compilation and renderer/device behavior are separate
requirements. The PSV shader ABI is documented beside its
[programs](../crates/crysirror-shaders/native/psv/README.md); validation evidence
belongs in [focused notes](validation/psv-shared-shaders-2026-11-15.md).

## Source ownership

`crysirror-shaders/wgsl` now owns the shared renderer's canonical WGSL files.
The desktop wgpu backend imports those same constants. `material_modules`
assembles color/prepass/shadow material hooks for both the backend and tooling.
`crysirror-shader-tools` contains a per-entry Naga-to-GLSL 460 source exporter
with deko3d resource namespaces and binding reflection. This is compiler input,
not an OpenGL runtime. The exporter records translation failures and exits with
an error for an incomplete export. It does not invoke UAM or create DKSH.

The PSV exporter lowers the shared 2D entries through Naga's typed HLSL
representation to native GXM Cg and supplies reduced 3D/BVH programs. HLSL is
only a build intermediate. Hardware ray-query programs are recorded as requiring that
capability; Lite primary rays are not equivalent replacements for glossy
reflection, GI, alpha-tested material baking or hardware AS operations.

## Lite software rays, ABI v1

The first source profile is opaque flat-color perspective primary rays, one
directional hard-shadow ray, ambient light and a sky gradient. It has no textures,
alpha masks, skinning, GI, denoiser or reflections. It writes linear RGBA;
tonemapping, output transfer and composition remain the host renderer's work.
Vita's fragment source returns color without a world-depth attachment, so it
cannot yet be mixed with arbitrary rasterized 3D by depth.

- Shared traversal: `crysirror-shaders/native/ray_lite.cgh`.
- PSV: native Cg VP/FP entry `main`, GXM parameter semantics and `tex2Dlod`.
  There is no OpenGL/vitaGL dependency or runtime in this source path.
- Switch: GLSL compute input for deko3d/UAM, 8x8 workgroups, bounds-checked image
  stores. The exporter expands the engine-owned common include before UAM input.
- `crysirror-raytrace` is safe `no_std + alloc` CPU code. Its builder rejects
  non-finite or out-of-range geometry, unsupported texture widths, colors outside
  linear 0..1 and scenes above 1024 triangles. It never silently truncates.

The builder median-splits the longest bounds axis into preorder BVH nodes with
at most four triangles per leaf. Descending advances to the next node; missing
a box or completing a leaf follows its escape index. Traversal visits at most
the full node count and uses no per-pixel traversal stack. A shadow ray returns
on its first hit. This reduces unnecessary triangle tests; it is not a measured
performance result. Low-resolution rendering can reduce the number of rays,
but a host must still implement reconstruction and measure quality/cost.

### Data textures

Both targets use logical **RG32F**, with two full-precision floats per texel.
The inspected VitaSDK `psp2/gxm.h` defines
`SCE_GXM_TEXTURE_BASE_FORMAT_F32F32` and two-channel swizzles; it does not define
RGBA32F texture storage. Select a native swizzle that returns the packed pair
as `.xy`, and verify the upload channel order before device acceptance.
Do not substitute half floats: node/triangle indices and scene coordinates
must remain exactly representable within this profile.

Sampling is nearest, clamp, mip zero, without sRGB conversion or compression.
`DataTexture::texels()` is row-major logical data; the host owns native texture
layout, pitch/alignment, memory mapping, upload lifetime and resource fences.
Widths are powers of two up to 1024. Empty scenes have zero counts and an
allocated zero row; the shader must not dereference a nonexistent root.

| Table | RG texels per record | Values in order |
|---|---:|---|
| Node | 4 | min.x/min.y, min.z/escape, max.x/max.y, max.z/leaf |
| Triangle | 6 | a.x/a.y, a.z/red, b.x/b.y, b.z/green, c.x/c.y, c.z/blue |

For an internal node `leaf=0`. For a leaf `leaf=first_triangle*5+count`, with
count 1..4. Escape indices increase strictly; the root escapes to node_count.
Only immutable builder results should be submitted, not arbitrary serialized
tables. These buffers are derived render resources, not original-save state.

### Uniform and resource binding

| Parameter | Values |
|---|---|
| ray_nodes | node texture width, height, node count, 0 |
| ray_triangles | triangle texture width, height, triangle count, 0 |
| ray_eye | eye xyz, positive world-space ray bias |
| ray_corner | unnormalized top-left ray xyz, 0 |
| ray_dx | unnormalized ray delta over UV width xyz, 0 |
| ray_dy | unnormalized ray delta over UV height xyz, 0 |
| ray_light | nonzero direction toward sun xyz, maximum ray distance |
| ray_sun | linear directional RGB, 0 |
| ray_ambient | linear ambient RGB, 0 |
| ray_sky_low | linear lower-sky RGB, 0 |
| ray_sky_high | linear upper-sky RGB, 0 |

All uniform components must be finite. Bias must be positive and large enough
for scene-scale f32 precision; max distance must exceed it. Corner/deltas must
describe a valid perspective camera without zero-length screen rays. This
profile does not accept orthographic cameras by silently reusing a common eye.
Native host validation of these camera/light constraints remains to implement.

On PSV samplers are TEXUNIT0 (nodes) and TEXUNIT1 (triangles). Resolve named
uniforms through the compiled GXP reflection, not guessed parameter offsets.
The VP takes POSITION float2 and TEXCOORD0 float2; supply an oversized fullscreen
triangle with UV orientation matching top-left screen coordinates. Use native
FP/VP compiler profiles and preserve full float precision.

### PSV host (renderer)

`crysirror-wgpu` runs this profile for `RenderingPath::ComputeRaytrace` on
GPUs without `Rgba16Float` storage (`ray_lite.rs`, `ray_lite.wgsl`). It is the
reduced tier of the compute profile, not an equivalent:

- **Data**: it builds the PackedScene from the 3D scene's triangles (at most
  1024 triangles) and uploads both textures as `Rg32Float`, 256 texels wide.
- **Pass**: it draws one fullscreen triangle into an `Rgba8UnormSrgb` target.
- **Lighting**: the constants are those of `raytrace.wgsl`: sun
  (0.45, 0.85, 0.3), direct 0.82, ambient 0.18, sky = background.
- **Desktop preview**: `Renderer::use_lite_rays` selects the same profile on
  desktop.

`cook-vita` joins `ray_lite.wgsl` to `ray_lite_v`/`ray_lite_f`:

- Vertex attributes are bound by parameter name (`position`, `uv`).
- Uniforms are bare `ray_*` names.
- 32-bit float textures are always point-sampled.
- The GXM `F32F32_GR` swizzle returns the packed pair as `.xy`. It was verified
  by reading back every node and triangle texel of a 15-node, 26-triangle
  scene.

`ray_lite_f` is compiled at optimization level 1. Under Vita3K 0.2.1, the
same source compiled at level 2 drew wrong primary hits (the floor missing,
shapes distorted), level 3 lost direct light, and level 0 failed in
SceShaccCg (internal error 403).

On Switch samplers occupy 0/1, output RGBA16F image occupies image slot 0, and
the eleven vec4 uniforms occupy std140 UBO slot 0 in the order above (176 bytes).
Dispatch ceil(width/8), ceil(height/8), 1 and provide image/texture ownership
barriers in the native renderer before composition. These binding namespaces
are distinct. The host has not yet been implemented.

## Switch UAM compilation, 2026-11-13

Release `crysirror-shader-tools switch` exported 95 entries with no
translation failure: 89 translated raster/compute entries, the native
ray_lite compute source and 5 hardware ray-query entries (not exported).
UAM 1.1.0 (`/opt/devkitpro/tools/bin/uam`) compiled all 90 exported
sources to DKSH. Two vfx vertex entries first failed because Naga emulated
WGSL `instance_index` with a `naga_vs_first_instance` driver constant
buffer uniform; the exporter now selects `WriterFlags::DRAW_PARAMETERS`
(`gl_InstanceID + gl_BaseInstanceARB`). Eight modules compile only with
UAM's floating-point emulation of non-constant integer division/modulo
(quad fs_filtered/fs_filtered_premultiply/fs_stored, ordered_dither,
vfx_render vs_sprite, gi_project, both composite_filter variants); their
results can lose precision for large operands and need native integer
replacements before device acceptance. This is shader compilation evidence,
separate from host pipeline and game validation.

## Platform render tiers (2026-11-14)

`crysirror-render-tier` reduces the shared 3D extraction for each console
tier. It reports every native, reduced, substituted or disabled feature. VN
composition is outside its input type and is never reduced. The substitution
table and budgets are in that crate's README. The integrated runtimes keep their
own console shaders (`crysirror-siglus-integrated/src/native_vm/render/vita`,
`.../render/horizon`, `crysirror-bgi-integrated/native/vita`). Those shaders
are not part of this tier.

### Switch

Switch runs the same exported WGSL entries through UAM/deko3d. Its reductions
are parameters only: steps, resolutions, counts and substituted features. They
need no separate shader source. On 2026-11-14, `crysirror-shader-tools switch`
exported 95 entries with 0 failures. UAM 1.1.0 then compiled all 90 exported
sources to DKSH (90/90; the 5 hardware ray-query entries are not exported).
The earlier note about float-emulated integer division still applies. This
result covers compilation only.

### PSV tier programs

`crysirror-shaders/native/psv/lite` holds native Cg VP/FP sources for GXM, with
no OpenGL/vitaGL. `native::psv_lite_sources()` expands their engine-owned
includes. Each program cites the desktop WGSL entry it ports. The shared math
(`lite_common.cgh`) ports GGX/Smith/Schlick, height fog, PBR Neutral/ACES and
the equirect mapping line for line.

| Desktop | PSV | Reduction |
|---|---|---|
| pbr vs_main/vs_skinned/fs_main | lite_mesh_v, lite_skinned_v, lite_mesh_f | 2 cascades with 4-tap compare, 4 CPU-selected lights, 2 probes, per-draw volume L1, glTF factors only |
| shadow | lite_shadow_v/_skinned_v/_f | depth written to an F32_R atlas |
| sky, bloom, tonemap, copy_hdr | lite_sky_f, lite_bloom_down/up_f, lite_tonemap_f, lite_copy_f | none |
| exposure | lite_luminance_f → lite_reduce_f ×2 → lite_exposure_f | same metering and adaptation in fragment passes |
| prefilter | lite_prefilter_f | 64 instead of 256 samples, CPU Hammersley |
| probe | lite_probe_capture_f | faces in a 6x1 strip (GXM has no 2D arrays) |
| particle, vfx_render | lite_particle_*, lite_vfx_* | CPU-expanded corners, no soft-particle depth fade |
| mesh (legacy Scene3d) | lite_legacy_mesh_v/f | exact |
| brdf, gi_project | `crysirror_render_tier::derived` | CPU, exact except moments |

Matrices are uploaded as four column float4s (glam order), as in the Siglus
Vita shaders. The host provides the conventions these programs assume: GXM
color-surface gamma for sRGB output, texture gamma for sRGB material inputs,
4x MSAA on the 3D target, an atlas cleared to 1.0 with a LESS test, and a
0..1 clip depth matching the wgpu matrices.

`crysirror-shader-tools psv` records routes for desktop 3D entries and exports
the 23 canonical shared 2D entries: quad, wipe, composite, composite_filter,
ordered_dither, present and puppet. Three quad filter fragments specialize
eleven distortions and five sampling policies. Transitions have 22 kind
variants. Together these produce 206 native 2D programs, preserving sample
counts and byte-space rules. The box-filter
compute entry becomes three fragment passes with two F32x2 sum targets.

The native catalog also contains 29 reduced raster/effect programs and two BVH
ray stages. `native::psv_program` resolves a compiled program by name;
`native::psv_quad_filter_program` and `native::psv_wipe_program` select
filter/transition variants without allocating.
The GXM library joins the quad v2 module to its dedicated continuous-mask
and canvas-stencil variants for undistorted GPU blends. The filtered and
premultiplied fragments each have all five sampling variants. Draws without
those mask flags use the existing distortion and sampling catalogue.
The [ABI reference](../crates/crysirror-shaders/native/psv/README.md) describes
uniforms, batch boundaries, texture roles, lifetime and pass requirements.

The standalone [compiler utility](../platform/psv/shaderc/README.md) invokes
the local SceShaccCg module with full-precision options. It records exact
compiler input, versions, options, hashes and sizes before importing GXP.
The module and SDK binaries are not distributed with the project.
On 2026-11-15 all 237 programs compiled to GXP and were imported with matching
source/precision receipts. The shader library and tools passed release checks.
This establishes compilation, not shared GXM renderer or pixel validation.

## Work remaining

1. Connect the shared GXM renderer to the compiled catalog and validate
   pipeline linkage, resource bindings, pass ordering and pixels against desktop.
2. Replace the eight float-emulated integer division sites with native
   integer code and validate the DKSH binaries on a deko3d pipeline.
3. Validate the shared 3D/BVH native pipelines and their cost on each device.
   Shader compiler success alone establishes neither visual parity nor FPS.
4. Finish shared GXM renderer ownership and integration with shared scene
   extraction. Existing integrated-runtime console players and their own
   renderers do not establish this shared-renderer boundary.

These are lower-level Rust/source APIs, not additions to the reflective public
class registry; no docs/api class-schema fingerprint change is claimed.
