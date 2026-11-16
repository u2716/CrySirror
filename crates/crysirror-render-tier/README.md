# crysirror-render-tier

Platform render tiers for the console ports (Switch, PS Vita). This is shared
engine policy, not an engine adapter, so it has no `adopt`/`integrated` mode.
The four integrated runtimes (Siglus, BGI, Softpal, RFVP) keep their own
console renderers and shaders and do not pass through this crate.

`reduce(&PbrScene, Tier)` takes the shared 3D extraction and returns a scene the
tier's native executor can draw, plus one `Decision` for each feature that is
present. A feature is kept `Native`, `Reduced`, `Substituted` by a named
`Technique`, or `Disabled` with a reason. No feature is dropped without a
decision. `Tier::Desktop` returns the input unchanged.

**VN features are out of scope by construction.** The input type holds only the
3D world. Layers, sprites, text, transitions, composite filters, puppets
(E-mote), movies and audio never reach this crate. On every tier, VN
composition runs at full output resolution with the shared algorithms. Only
the offscreen 3D target is scaled (`Budget::scene_3d`).

## Substitutions

| Feature | Switch (docked/handheld) | Vita |
|---|---|---|
| Ray-traced GI | raster irradiance volume × RT intensity, else SH9 | same |
| Ray-traced reflections | SSR (1 ray, depth march) | env + probes |
| SSR | ≤24 steps, thickness ≥ one stride | env + 2 nearest probes |
| Virtual shadows | 3 × 1024² cascades, ≤80 m | 2 × 512² cascades, ≤30 m, 4-tap PCF |
| Geometry LOD | GPU (native) | CPU selection, ≥2 px error |
| TAA | native | 4× MSAA |
| SSAO | half resolution | material occlusion only |
| Bloom / auto exposure / fog | 5 levels / native / native | 3 levels / fragment reduction / native |
| Irradiance volume | ≤512 probes, 2 updates/frame | ≤128 probes, 1/frame, no visibility, per-draw CPU sample |
| Reflection probes | 4 nearest, 128 px | 2 nearest, 64 px |
| Local lights | ≤256, 24 shadow tiles | ≤16 (4 per draw), no local shadows |
| Rect lights | native | native (same closest-point Lambertian term) |
| Particles | emission ×½, ≤32768 | emission ×¼, ≤2048, CPU |
| GPU VFX / soft particles | native | CPU simulation / no depth fade |

Console budgets are starting limits taken from each native API's
capabilities. They have not been measured on hardware or in an emulator.

`derived` reproduces desktop GPU computations on the CPU for tiers whose
programs lack integer bit operations or compute stages: Hammersley points,
the split-sum BRDF LUT (`brdf.wgsl`), probe L1 projection (`gi_project.wgsl`,
without visibility moments) and the per-draw irradiance-volume sample.

## Validation (2026-11-14)

- `cargo test --release -p crysirror-render-tier`: 10 passed. The tests cover
  desktop identity, validity of reduced environments and volumes/emitters,
  determinism, and the Switch/Vita substitution tables. They also check that
  uniform radiance L projects to irradiance πL and that A+B≈1 for a smooth
  head-on BRDF.
- Release `check` passed for `armv7-sony-vita-newlibeabihf` (build-std) and
  for `platform/switch/rust/aarch64-switch.json` (with the libc patch recipe).
  This is a check only: nothing was linked or executed.
