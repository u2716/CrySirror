# 0041: Engine capability roadmap (what a complete engine still needs)

Date: 2026-10-25. Status: accepted; living list. Update a row only from
actual results, as STATUS.md requires. This matrix is not a whole-engine test
report. The 2026-11-03 documentation pass corrects superseded missing-feature
claims using their owning ADRs and existing recorded evidence; it adds no new
hardware or full-game acceptance.

## Context

A general-purpose 3D engine needs a capability matrix across runtime, rendering,
content, gameplay and tooling. Subsystem availability and end-to-end integration
are separate: a playable scene can require integration fixes even when its
individual mechanisms already exist. For example:
- skeletal animation, character physics, navigation and cameras all
  existed;
- an imported level needed a navigation fix before an autopilot could
  route through it.

## Decision

Keep this matrix as the engine's capability checklist. Each row names the
owning ADR and an honest status:
- **done**: implemented and behaviour-validated;
- **partial**: implemented first slice, with known gaps;
- **missing**: not started.

Priorities:
- **P0** blocks shipping a small 3D game end to end.
- **P1** is needed for AAA-class content.
- **P2** is breadth.

### Core and runtime

| Capability | Status | ADR | Next step | P |
|---|---|---|---|---|
| Object model, reflective classes, generated bindings | done | 0004 | — | |
| Lifecycle (init/start/fixed/late update), plugins | done | 0004 | hot reload of scripts | P1 |
| JS/TS scripting (Boa + Oxc), pluggable VMs | done | 0005 | debugger protocol, profiling hooks | P1 |
| Virtual file system, capability roots | done | 0006 | async I/O and streaming reads | P1 |
| Save/load of engine state | partial | SAVE_FORMAT | versioned migrations of saves across schema changes | P0 |
| Generated API docs and change log | done | docs/api | — | |
| Engine-wide async / multithreaded frame (simulation ∥ render) | partial; mandatory across all subsystems | 0049, 0050, 0052 | bounded render-extraction DAG and retained snapshots added; simulation parallelism, budgeted uploads and remaining subsystem migration remain | P0 |
| Deterministic fixed step, replay | partial | 0012 | input recording and replay files | P2 |

### Assets and pipeline

| Capability | Status | ADR | Next step | P |
|---|---|---|---|---|
| glTF/GLB import (PBR, skins, animation), Blender export | done | 0007 | — | |
| FBX / PMX / VRM import, morph targets | partial | 0054 | weight animation channels, FBX blend shapes, MToon, spring bones, VMD | P1 |
| Texture codecs (BC/DXT, PNG, HDR) | done | 0017 | BC7/ASTC encoding and GPU compression at import | P1 |
| Asset database (GUIDs, dependencies, incremental re-import, cooking) | missing | — | content database plus cook step to a runtime format | P0 |
| Cluster LOD DAG | partial | 0026 | on-disk DAG cache, streaming | P1 |
| Native importers (GoW III, UE, E-mote) | partial | 0017, 0013, 0029 | GoW skeleton/skin/ANM; real E-mote validation | P1 |
| Packaging (desktop zip, licence notices) | partial | package_game | signed builds, platform bundles | P1 |

### Rendering

| Capability | Status | ADR | Next step | P |
|---|---|---|---|---|
| PBR forward+ (HDR, IBL, CSM, SSAO, TAA, bloom, tonemap) | done | 0010 | — | |
| Clustered local lights and shadows | done | 0018 | — | |
| Fog, auto exposure | done | 0019 | volumetric fog and light shafts | P1 |
| SSR, reflection probes | partial | 0023–0025 | glossy SSR, Hi-Z, realtime probes | P1 |
| Virtual geometry (cluster LOD, GPU culling) | partial | 0026 | HZB occlusion, software raster | P1 |
| Virtual shadow maps | partial | 0027 | GPU page allocation, local lights | P1 |
| Dynamic GI | partial | 0028, 0059, 0060 | RT probes and per-pixel single-bounce GI include local-light hit shading; broader transport, denoising and performance comparisons remain | P1 |
| Transparency, materials, shader effects | partial | 0015, 0016 | material graph and editor, OIT, refraction | P1 |
| Decals, terrain, foliage | partial | 0014 | deferred decals, foliage instancing and wind | P1 |
| Post FX (DOF, motion blur, color grading LUT) | missing | — | per-camera post stack | P1 |
| Upscaling (FSR-class) | missing | 0007 | open upscaler | P2 |
| Hardware RT | partial | 0051, 0052, 0055–0060 | GPU-stream/dynamic BLAS, alpha tests, soft/local shadows, GGX reflections and probe/per-pixel GI exist; broader hit shading/denoising, refit and multi-device validation remain | P1 |
| 2D layers, filters, transitions, puppets, video | done | 0020–0022, 0029 | — | |

### Animation

| Capability | Status | ADR | Next step | P |
|---|---|---|---|---|
| Skinned meshes, clips, cross-fades, layers | done | 0012 | — | |
| State machines, montages, 1D/2D blend spaces | done | 0014 | — | |
| Root motion | missing | — | extract and apply to character controllers | P0 |
| IK (foot placement, look-at), ragdoll | missing | — | two-bone IK, physics ragdoll blend | P1 |
| Level sequences (camera, transforms, lights, events) | done | 0040 | material/audio tracks, editor | P2 |
| Retargeting between skeletons | missing | — | humanoid mapping | P2 |
| 2D puppets (Live2D/E-mote class) | partial | 0029 | real E-mote validation, keyform grids | P1 |

### Physics, gameplay, AI

| Capability | Status | ADR | Next step | P |
|---|---|---|---|---|
| Rigid bodies, colliders, character controller (Rapier) | done | 0012 | — | |
| Triggers (sensors), contact events → scripts | implemented bounded service | 0045 | sensor/contact/force/sleep events are exposed through Physics.events; application policies remain caller-owned | P0 |
| Vehicles (arcade) | partial | 0043 | save state, dynamic bodies, wheel visuals | P1 |
| Joints, cloth | partial | 0045 | fixed/spherical/revolute/prismatic joints exist; cloth and authoring tools remain | P1 |
| Gameplay ability system, attributes, effects | done | 0014 | — | |
| Behaviour trees, perception | done | 0014 | — | |
| Navigation grid + A* | done | 0014 | navmesh polygons, dynamic obstacles, crowds | P1 |
| Streaming levels | partial | 0014, 0052 | async cell preparation/leases and offline readiness implemented; world partition, chunk paging and GPU upload budgets remain | P1 |

### Audio, input, UI, text

| Capability | Status | ADR | Next step | P |
|---|---|---|---|---|
| Mixer, buses, SFX pools, music, spatial | partial | 0030, 0067–0084 | BVH occlusion, HRTF, late reverb, reflected paths and worker decode/mix/output exist; full acoustic transport, hard-realtime guarantees and broader native fidelity remain | P1 |
| Input actions, rebinding, gamepads | partial | 0032 | hardware gamepad validation | P0 |
| UI widgets, layout, focus | partial | — | theming, scroll/list/text-input widgets, accessibility | P0 |
| Text shaping, bidi, ruby, SDF | partial | 0031 | GPU SDF text rendering | P1 |
| Localization | done | 0033 | — | |

### Tooling and platform

| Capability | Status | ADR | Next step | P |
|---|---|---|---|---|
| Editor (scene view, inspector, gizmos, undo) | missing | — | an editor on the reflective API | P0 |
| Profiler (CPU/GPU timings, memory) | partial | 0052 | CPU trace, optional GPU duration ring and resource/queue diagnostics implemented; live overlay and comprehensive GPU memory accounting remain | P0 |
| Debug draw, console, in-game stats | missing | — | debug-draw API, command console | P1 |
| MCP/automation tools | done | MCP.md | — | |
| Networking (replication, RPC) | missing | — | client/server replication of world state | P2 |
| Platforms (Windows/macOS/Linux desktop) | partial | — | local platform validation; mobile and web remain deferred | P1 |
| Consoles (Switch, Vita) | deferred | AGENTS.md | — | |

### Compatibility targets (secondary, ADR 0010)

| Target | Status | ADR |
|---|---|---|
| RFVP | partial first-class HCB adapter; original VM availability is not full integration | 0047, NATIVE_ADAPTERS |
| Siglus | partial | 0009, 0034–0036 |
| BGI | bounded first-class BP profile; old hosted acceptance superseded | 0047, BGI |
| GoW III level | partial: geometry, materials, lights, cubemaps, probes, collision, navigation | 0017, 0018, 0025 |
| GoW III characters | missing: skeleton, skinning and ANM undecoded (`ANM_hero` is 10.5 MB in hero.wad) | 0017 |
| NFS Carbon (canyons) | partial geometry/texture rendering; full instances/material/gameplay fidelity remains open | 0042 |

## Order of work

1. **P0 game loop gaps:** root motion; triggers and contact callbacks;
   UI widgets; save migrations; profiler.
2. **GoW III character pipeline**: decode the hero
   skeleton, skin weights and `ANM_hero`, and drive Kratos through the
   existing animation graph instead of the Fox stand-in.
3. **Editor** on the reflective API (largest single gap).
4. **Asset database and cook step.**
5. Then the P1 rendering rows.

## Validation of the "one walking scene" requirement (2026-10-25)

- `~/.cache/crysirror/gow3/olympus-hd/preview/hall-walk.ts` (user data,
  not in the repository) runs a third-person character in the imported
  OLY100 hall:
  - triangle-mesh collision for the whole level (~1M triangles);
  - a capsule character controller with steps, slopes and gravity;
  - the skinned Fox (CC0 model, CC-BY rigging) with Walk/Run/Survey clips
    and cross-fades, as a stand-in for Kratos;
  - a grid navigation bake with A* to random reachable goals;
  - a wall-avoiding orbit camera;
  - WASD/Shift/Space control in `crysirror run`.
- An autopilot run crossed the hall floor between seven goals in 23 s of
  game time.
- Engine fix found by this scene: navigation bakes treated small steep
  bumps (decals, trim) as obstacles, and erosion removed most of the floor.
  Walkable cells went from 1,015 to 4,153 of 14,560. The regression test is
  `navigation_steps_over_small_steep_bumps_but_not_tall_ones`.

## Clipping fixes (2026-10-25, second pass)

Clipping in the walking scene exposed collider sizing and terrain-alignment
requirements. Shared capabilities cover them, with tests in
`crysirror-engine/tests/physics.rs`:
- **`modelInfo(..).bounds`** gives rest-pose bounds, with skinned vertices
  posed through their joints. The character collider is now sized from the
  model instead of guessed: the capsule radius is half the body length, and
  the navigation agent radius matches it.
- **`groundPose`** probes the ground under the front and back feet. The
  body pitches with slopes and steps, and the feet follow the ground
  instead of a fixed offset.
- **`sphereCast`** lets the orbit camera sweep a sphere, so its near plane
  no longer grazes walls.

Measured with a per-frame body-axis sphere sweep (head and tail, 1,500
frames of the OLY100 autopilot):

| Collider | Clipping frames | Worst depth |
|---|---|---|
| Old guessed capsule (radius 0.35 m) | 216 (14%) | 2.08 units |
| Fitted collider | 0 | — |

Remaining: white fragments over the central ring are GoW III level
geometry left in rest space (unposed breakables). They need the native
entity/ANM transforms (ADR 0017 P0), not an engine change.
