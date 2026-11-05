# Native console shader sources

This is source work, not a completed console renderer. GXP/DKSH compilation and
device behavior are separate requirements. No new compilation or execution was
performed after native compilation was deferred. In particular, the current
source revision has not passed Rust, Naga, SceShaccCg or UAM validation.

## Source ownership

`crysirror-shaders/wgsl` now owns the shared renderer's canonical WGSL files.
The desktop wgpu backend imports those same constants. `material_modules`
assembles color/prepass/shadow material hooks for both the backend and tooling.
`crysirror-shader-tools` contains a per-entry Naga-to-GLSL 460 source exporter
with deko3d resource namespaces and binding reflection. This is compiler input,
not an OpenGL runtime. The exporter records translation failures and exits with
an error for an incomplete export. It does not invoke UAM or create DKSH.

The PSV exporter currently writes the native Lite ray sources but reports the
remaining WGSL raster/compute entries as unported. It must not be described as a
complete Cg translation backend. The Switch exporter also has unresolved source
translation cases. Hardware ray-query programs are recorded as requiring that
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

On Switch samplers occupy 0/1, output RGBA16F image occupies image slot 0, and
the eleven vec4 uniforms occupy std140 UBO slot 0 in the order above (176 bytes).
Dispatch ceil(width/8), ceil(height/8), 1 and provide image/texture ownership
barriers in the native renderer before composition. These binding namespaces
are distinct. The host has not yet been implemented.

## Work remaining

1. Finish native Cg raster/material/effect equivalents and explicit fragment/CPU
   alternatives for compute passes. Do not label their absence a working fallback.
2. Resolve Switch source translation failures, then compile source with UAM.
3. Supply the owner-provided native compiler module for GXP compilation. The
   inspected Vita3K filesystem did not contain libshacccg.suprx. No module or SDK
   binary is distributed here; no emulator is automatically launched.
4. Compile/check the current Rust and shader source revisions, run meaningful
   CPU BVH tests, then validate native bindings, pixels and cost separately.
5. Implement native GXM/deko3d renderer ownership and integration with shared
   scene extraction. No console application, VPK/NRO or measured FPS exists yet.

The CPU BVH tests are authored but have not been compiled or run for this source
revision. Shader tools and a local host UAM executable were built during earlier
exploration; that does not validate these later source changes. Moving shared
shader files also needs a subsequent desktop release check.

These are lower-level Rust/source APIs, not additions to the reflective public
class registry; no docs/api class-schema fingerprint change is claimed.
