# ADR 0007: modern rendering features and Blender/glTF assets

Status: implemented initial asset-import scope, extended by ADRs 0010/0012/0015/
0054. Hardware ray queries are implemented in ADRs 0051–0060. Vendor upscalers
remain contract-only; this record does not establish their implementation.

## Requirement

R9–R11: integrate current AAA-engine technology: ray tracing, DLSS-class
upscaling, and loading Blender models.

## Decisions

### Blender and model import (implemented)

- glTF 2.0 is the interchange format. `crysirror-asset::import_gltf` uses the
  pure-Rust `gltf` parser. It reads GLB or external-buffer glTF through the
  engine VFS and keeps the node hierarchy as entities. It converts TRS to
  engine transforms (glTF +Y up, as exported by Blender), turns multi-primitive
  meshes into child entities, and converts base-color factors from linear to
  sRGB. The initial importer reported textures, animation, skins, cameras, morphs
  and transparency as dropped. PBR materials/textures, skins/animation and morphs
  were subsequently added through ADRs 0010/0012/0015/0054. Actual unsupported
  formats/extensions remain in `ImportReport.dropped`; this is not a claim of
  complete glTF or DCC interchange coverage.
- `.blend` is not parsed in-engine, because it is an undocumented
  memory-layout dump that changes per Blender version.
  `tools/blender_export_gltf.py` runs inside Blender to convert. Run with
  `--build-fixture`, it regenerates the original lantern model in
  `examples/engine/assets/lantern.glb`.
- Scripts call `world.importModel(path, {parent, prefix})`.

### Upscalers (contract only)

`crysirror_core::graphics::{Upscaler, UpscalerSupport}` covers Native, DLSS,
FSR, XeSS and MetalFX. A required upscaler that is not implemented returns
`Unsupported`, with no silent fallback to native. Every non-native upscaler
needs a compatible motion/depth/jitter and temporal integration contract. The
renderer now has depth, jitter and temporal processing, but those mechanisms do
not implement a vendor upscaler. No non-native upscaler backend is established
by this ADR.

Integration rules:
- DLSS requires NVIDIA's SDK and an NVIDIA RTX GPU. It runs through a
  backend-specific graphics interop path; eligibility must be established against
  the actual SDK and renderer rather than the retired wgpu 0.20 baseline. The SDK
  is not redistributed in this repository (AGENTS.md); a backend plugin would
  load it from the user's install. It cannot run on the macOS/M5 host used
  here.
- FSR (MIT-licensed shaders) is the portable first target. MetalFX is the
  native macOS target.
- Prerequisites, in order: a frame graph with motion vectors and jittered
  projection, then a temporal accumulation path, then the vendor plugin.

### Ray tracing

The compute prototype remains a separate rendering choice. The hardware path
now negotiates actual device features and owns BLAS/TLAS over renderer geometry;
ADRs 0051–0060 add dynamic geometry, alpha tests, shadows, reflections and GI.
The workspace's current wgpu baseline is declared in Cargo.toml. Logical scenes
and saves do not contain acceleration-structure/device handles. The initial
prototype captures below predate those changes and are not hardware-RT evidence.

## Evidence

- `crysirror-asset` unit tests: hierarchy/TRS/material/external buffer and
  visible failures.
- `script_imports_blender_gltf_model`: a Blender 5.2.2-exported GLB with 3
  nodes and 3 primitives, nothing dropped, hierarchy preserved, and the roof
  apex at y=1.6 after the Z-up to Y-up conversion.
- GPU captures `dist/engine-api-raster.png` and
  `dist/engine-api-compute-raytrace.png`, made on Apple M5/Metal with
  hardware_raytracing=false.
