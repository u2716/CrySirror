# PS Vita shared shaders, 2026-11-15

## Implementation

| Area | Implemented output |
| --- | --- |
| Shared 2D | 23 canonical entries lowered from shared WGSL through Naga's typed HLSL representation to native GXM Cg. 165 quad filter variants, 22 transition variants and 19 other programs give 206 native programs. |
| Box filter | Three fragment passes with two F32x2 horizontal RG/BA sum targets and integer vertical accumulation/division. Both canonical output formats are catalogued. |
| Reduced 3D | 29 native raster/effect stages for the existing Vita feature tier. |
| Software rays | Two native stages using the shared packed RG32F stackless BVH, nearest hits and directional hard-shadow rays. |
| Runtime lookup | Aligned GXP catalog, named binding/receipt manifest and allocation-free filter/transition selectors in `crysirror_shaders::native`. |

Canonical WGSL remains unchanged. Specialization preserves the original branch,
sample counts and options. The random explosion centre uses the original GPU
seed formula in the fullscreen vertex stage, passed to fragments at TEXCOORD1.
The same wipe record must be bound in both stages. Byte RGB interpolation uses
exact bounded F32 integer arithmetic; alpha and box sums retain integer paths.
General integer division uses a biased estimate and exact remainder corrections.
The [native ABI](../../crates/crysirror-shaders/native/psv/README.md) specifies
bindings, required loop constants, pass ordering and resource lifetime.

## Compilation

| Check | Observed result |
| --- | --- |
| Release `crysirror-shader-tools psv` export | 308 manifest entries, 0 failures; 237 native Cg files. Other entries describe 3D routes and hardware ray-query exclusions. |
| Standalone shader compiler VPK | Release cross-build with `cargo +nightly vita build vpk -- --release` succeeded, reusing the existing release target directory. |
| Native Cg compilation | 237/237 programs available, `missing.json` empty. Only the compiler utility ran in headless Vita3K. |
| Import | All 237 compiler inputs, receipts, GXP headers and byte counts admitted before repository outputs were written. 206 shared 2D sources imported. |
| GXP payload | 7,781,292 bytes across 237 programs, retained with 16-byte alignment. |
| Rust | `cargo check --release -p crysirror-shaders -p crysirror-shader-tools` passed. |
| Host driver | `python3 -m py_compile tools/compile_psv_shaders.py` passed. |
| Shared traversal on Switch | Release source export: 95 entries, 0 failures. UAM 1.1.0 compiled the changed `ray_lite.comp.glsl` wrapper to a 7,936-byte DKSH. Other Switch programs were not recompiled in this validation. |

Naga version: 30.0.1. Native compiler identification:

```text
libshacccg_version: SDK 3.0.0
graphics/release_branches/release_03.000/shacc r13225
sys/release_branches/release_03.000 r85995
optimization=1 (110 filtered fragments), 2 (ray fragment), 3 (126 other stages)
fastmath=0, fastprecision=0, fastint=0
```

The local compiler module SHA-256 is
`188adb8c34c3983d63244c32d981fd7b518a8dcfd200f5886233fface6b5fa40`.
The imported manifest records per-program source/GXP BLAKE3 hashes, compiler
identification and options. Neither the module nor SDK binaries are bundled.

The [compiler driver](../../tools/compile_psv_shaders.py) creates compiler-only
temporary filesystems and uses a fresh process per program. Each invocation has
a 90-second default timeout, log limits and a 2 GiB free-space floor. Fatal
compiler state is retired without re-entering compiler teardown. It only stops
owned child processes. See [regeneration instructions](../../platform/psv/shaderc/README.md).

A targeted host F32 comparison of the division algorithm matched integer
division across 4,880 signed boundary/sample cases. This checks the arithmetic
construction, not native GPU ALU execution. No new test suite or game regression
was run.

## Behavior boundary

The compilation checks above validate translation and Rust catalog linkage.
The subsequent native GXM probe below adds API and pixel observations. The
shared GXM renderer is not connected yet. Hardware behavior and performance have not been validated. Actual-game
observations are recorded separately below. Existing integrated engine renderers and
their player packages do not establish those shared-renderer results.

The simplified ray profile remains flat-color opaque geometry with primary
visibility and a hard sun shadow. It does not replace desktop hardware ray
queries, textured alpha tracing, reflection, GI or denoising. The existing Vita
3D substitutions remain explicit in the feature tier.

These are Rust-only host APIs. No reflective class, command, adapter
configuration or `docs/api` signature was changed.

## Native GXM validation

The optimized VitaSDK [native diagnostic](../../platform/psv/shader-validate/README.md)
loaded only engine-owned GXP and synthetic textures/geometry. Both the local
macOS Vita3K and Linux Vita3K admitted 237/237 programs, created 19/19 vertex
programs and accepted 218/218 fragment links with their vertex families. Native
draw calls returned zero. These results do not establish pixel correctness.

macOS Vulkan reported memory mapping disabled and retained the CPU target's
initialization marker, even for the vita2d clear control. Its readback is
unavailable in this configuration, not a visual pass. The diagnostic now returns
code 2 for that condition.

The Linux run used SDL's offscreen driver, Vulkan on AMD Radeon 680M
(RADV/Mesa 25.1.9), double-buffer memory mapping, high accuracy and synchronous
pipeline compilation. No desktop capture, firmware, fonts or games were used.
An empty second scene after the first CPU read requests native surface sync.

| Synthetic case | Pixel result |
| --- | --- |
| Presentation/copy | Pass: all 4096 RGBA pixels exactly match the asymmetric source pattern. |
| Plain quad | Pass: all 4096 RGBA pixels exactly match the source. |
| Filter, hardware sampler | Fail: blue/alpha output differs; centre `[128,128,255,0]`, expected `[128,128,0,255]`. |
| Filter, encoded point | Fail: constant green/zero-alpha output. |
| Filter, encoded bilinear | Fail: constant green/zero-alpha output. |
| Filter, byte point | Fail: zero RGBA output. |
| Filter, byte bilinear | Fail: constant yellow/zero-alpha output. |
| Kind-0 transition | Fail: target retains the known clear colour instead of the source pattern. |
| Legacy 3D mesh lighting | Pass: triangle centre `[32,127,64,255]`, matching the independent Lambert calculation within two byte units. |
| Packed BVH ray profile | Fail: sky and primary red hit appear, but an unoccluded sunlit centre is `[64,0,0,255]` instead of `[255,0,0,255]`. The direct-light/normal/shadow cause is unresolved. |

The pixel comparator returns **failure: 3/10 checks pass, 7/10 fail**. The
filter, transition and ray findings are reproducible; they are not marked fixed.
Puppet, stored blend profiles, distortion variants, separable box filtering,
PBR, skinning, particles and other postprocessing were linked but not exercised
by these pixel fixtures. No full renderer or hardware acceptance is established.

The Linux AppImage SHA-256 was verified against its release asset digest:
`bc21c8f21c9e6efa778c48bf4196e292ebb0abe7e3ada337813deb12232dff96`.
The emulator also emitted a shutdown SIGSEGV diagnostic after the completed
native report; clean emulator teardown is not claimed.

Private local artifacts (ignored by Git):
`local/validation/psv-shared-shaders-2026-11-15/linux/` contains `report.txt`,
ten raw RGBA frames, `pixels.json`, `emulator.log` and `offscreen-results.png`.
The `macos/` sibling retains the API/readback report. These contain synthetic
data only. Recheck the captured pixels without launching anything:

```sh
python3 tools/validate_psv_shaders.py \
  local/validation/psv-shared-shaders-2026-11-15/linux --compare-only
```


## Follow-up diagnosis and corrected native programs

The table above retains the original all-level-3 baseline. The subsequent
canonical catalog was recompiled and imported with per-program receipt checks.
The Linux native probe then passed **6/10 pixel checks**, still returning failure
for the four encoded/byte filter cases. All 237 registrations, 19 vertex creates
and 218 fragment links continued to succeed.

| Finding | Controlled observation | Change and current boundary |
| --- | --- | --- |
| Wipe fullscreen vertex | Replacing local-array indexing by equivalent arithmetic restored every source pixel in the kind-0 probe. | Native lowering now computes the same three vertex positions/UVs arithmetically. Canonical WGSL is unchanged. Other wipe kinds have not received pixel acceptance. |
| Filter control flow | Level-3 full filter GXP triggers Vita3K `NEGP2` and unhandled branch diagnostics. The same complete hardware-sampler source at level 1 matches all 4096 pixels. | Filter fragments use level 1 with all precision shortcuts disabled. Four encoded/byte sampling cases still return incorrect pixels. Minimal standalone sampling kernels pass, so those results do not validate their full filter composition. Integer-flag and zero-flag probes did not fix the remaining cases and were not imported. Level-0 compilation produced three of four diagnostic programs, but native execution stopped before finishing the first encoded draw within the 180-second guard; those binaries were not imported either. |
| Ray lighting | A diagnostic fragment produced normal.z=1, Lambert=1 and occluded=false for the centre hit. The full level-3 program nevertheless returned ambient red. Level 2 restored direct sunlight and the expected sky. | Keep Lambert immutable across the secondary trace and compile the ray fragment at level 2. The unoccluded primary/shadow path passes; positive occluders and general BVH scenes remain unvalidated. |
| macOS readback | The emulator disables Vulkan memory mapping on Apple platforms. | Use the Linux offscreen run for pixels. macOS admission alone remains insufficient. |

Corrected canonical artifacts are private under `linux-fixed/`: `report.txt`,
ten raw RGBA images and `pixels.json`. Their expected pass set is presentation,
plain quad, hardware filter, kind-0 wipe, mesh lighting and the unoccluded ray.
No diagnostic GXP override was used in that run. The changed shared ray source
also passed UAM compilation for Switch (7,936-byte DKSH). The targeted release
Rust check passed after importing the updated catalog.

## Actual game observations

Only CrySirror and its own release PSV program ran. Original executable files
were not executed. Game data stayed in existing private installations. All
output capture used engine/GXM buffer readback, without reading the desktop.

| Host | Game | Observed scope |
| --- | --- | --- |
| Desktop, headless wgpu | Gin'iro, Haruka (`ginharu`) | 1500 paced frames and scripted Enter events reach the title menu, scene `_01menu`, line 77, with no runtime error. Private capture: `real-games/ginharu-title.png`. This uses integrated Siglus, not the shared GXM catalog. |
| Vita3K Linux, native GXM | Same installed game | Initial release exhausted its entry stack during Siglus host initialization. The corrected release creates one 2 MiB session-owner thread and reaches the title menu at frame 1500. The final 960x544 PNG has readable text and correct row layout. Private capture: `real-games/psv/frame-1500.png`. This uses integrated Siglus GXM and does not validate shared GXM shaders. |

Desktop detection also incorrectly requested common archive mounts for all 389
Siglus voice packs, exhausting the mount budget before the integrated runtime
could start. Siglus detection now leaves these archives to the integrated
runtime's scoped native resource reader. RealLive mounting is unchanged.

The PSV stack failure is established by the release ELF and emulator fault:
`new_with_renderer_sync` reserves approximately 160 KiB, and its first stack
store already targets an invalid address below the entry stack. The new thread
owns session construction, GXM and input for its entire lifetime. No debug
build was generated. The player release cross-build succeeded.

The first game capture was sheared because the readback used a 1024-pixel
framebuffer pitch. The linked libvita2d display callback uses 960 pixels,
confirmed in the installed static library and the
[libvita2d source](https://github.com/xerpi/libvita2d/blob/master/libvita2d/source/vita2d.c).
The capture code now uses that pitch. Readback correctness is checked separately
from successful VM startup; the initial sheared image is not visual acceptance.
The missing PGF system-module warning was not the cause of the stack overflow.

The final corrected PSV release was run again to frame 1500 after the pitch
change. Captures at frames 150 and 1500 were inspected as local files: the
startup notice is readable, and the title background, logo and menu labels are
correctly aligned. No desktop surface was captured. The harness waited for the
PNG IEND footer before stopping its owned emulator process. Audio devices were
disabled; script and render behavior were observed, not audio playback or real
hardware performance. Temporary compiler/profile probes remain outside the
product catalog.
