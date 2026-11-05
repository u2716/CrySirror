# Architecture decision records

ADRs record design choices, ownership boundaries and their evolution. IDs and
filenames remain stable so source references, test receipts and later decisions
continue to resolve. An implemented ADR remains useful design documentation.

- **Accepted**: the decision is binding; this does not by itself establish implementation.
- **Implemented scope/profile**: the described slice has an implementation and its
  recorded validation; wider capabilities and current regressions remain separate.
- **Partial/proposed**: required work or a design remains unimplemented.
- **Superseded**: follow the named replacement; the old recipe is retired.
- **Historical audit**: evidence for a dated source checkpoint, not a current backlog.

The 2026-11-03 review reconciles old missing-feature claims with follow-up ADRs.
It does not establish fresh builds, game compatibility or hardware results.
Implementation, compilation and observed behavior remain separately recorded in
[STATUS.md](../../STATUS.md). Current interfaces are indexed in
[the documentation guide](../README.md); [ADR 0041](0041-engine-capability-roadmap.md)
tracks remaining engine capabilities.

## Active decisions and implementation profiles

| ADR | Decision | Status / follow-up |
| --- | --- | --- |
| [0001](0001-service-boundary.md) | data-driven core, engine-specific compatibility frontends | accepted service boundary; native consumers tracked separately |
| [0002](0002-modern-engine-and-optional-typescript.md) | a modern 3D engine with independent compatibility frontends | accepted architecture; World and runtime scripts implemented in 0004/0005 |
| [0003](0003-siglus-monorepo-and-rendering-capabilities.md) | Siglus resource compatibility and desktop rendering capabilities | implemented resource/compute profile; hardware RT extended by 0051–0060 |
| [0004](0004-object-model-and-lifecycle.md) | engine object model, lifecycle and plugins | implemented object/lifecycle profile; broader tooling remains separate |
| [0005](0005-script-hosts.md) | pluggable script hosts — JS/TS by default, native VMs as peers | implemented runtime JS/TS profile; desktop always links the host |
| [0006](0006-virtual-filesystem.md) | virtual filesystem providers and mount tables | implemented capability VFS/mount profile; async extensions in 0049/0065 |
| [0007](0007-modern-rendering-and-assets.md) | modern rendering features and Blender/glTF assets | implemented asset-import scope; extended by 0010/0012/0015/0054 and 0051–0060; upscalers remain contract-only |
| [0010](0010-aaa-renderer.md) | AAA rendering: physically based HDR pipeline and showcase ports | implemented (CSM/SSAO/TAA added) |
| [0011](0011-game-ports.md) | porting real games to CrySirror to drive engine requirements | accepted |
| [0012](0012-animation-physics-input.md) | skeletal animation, physics/character control and input actions | implemented (first slice) |
| [0013](0013-unreal-demo-port-assessment.md) | assessing the "Ashen Vow" Unreal demo as a port target | accepted assessment; packaged-content conversion partial, not awaiting owner authorization |
| [0014](0014-gameplay-systems.md) | gameplay systems surfaced by the Unreal port assessment | implemented (first slice) |
| [0015](0015-engine-materials-transparency.md) | Engine material data and transparent PBR rendering | implemented (locally validated first slice) |
| [0016](0016-shader-effects.md) | Preset effects and developer surface shaders | implemented (surface ABI v1) |
| [0017](0017-gow3-scene-capabilities.md) | Native scene trials drive shared asset and rendering APIs | implemented first preview; further APIs proposed |
| [0018](0018-local-lights.md) | Clustered local lights | implemented bounded lighting/shadow profile; local validation recorded |
| [0019](0019-height-fog-auto-exposure.md) | Exponential height fog and auto exposure | implemented height-fog/exposure scope; local validation recorded |
| [0020](0020-video-service.md) | Engine video service over the Siglus decoders | implemented codec/media profile; original validation limits remain explicit |
| [0021](0021-siglus-2d-layer-filters.md) | Siglus-style 2D layer filters | implemented layer filter scope; wipes followed in 0022 |
| [0022](0022-screen-transitions.md) | Screen transitions (Siglus wipe compositor) | implemented; page curl/VM export pending |
| [0023](0023-screen-space-reflections.md) | Screen-space reflections | implemented SSR scope; probes followed in 0024/0025; glossy RT is separate in 0060 |
| [0024](0024-reflection-probes.md) | Box-projected reflection probes | implemented engine probe capture; authored GoW cubes followed in 0025; realtime/sphere scope remains open |
| [0025](0025-authored-probes-gow3-cubemaps.md) | Authored reflection probes and GoW III environment cubemaps | implemented; per-material cubes pending |
| [0026](0026-cluster-lod.md) | Cluster LOD (virtualized geometry, step 1) | GPU selection + frustum culling; HZB/streaming pending |
| [0027](0027-virtual-shadow-maps.md) | Virtual shadow maps for the sun (step 1) | static/dynamic split done; GPU allocation pending |
| [0028](0028-irradiance-volume-gi.md) | Dynamic diffuse GI with rasterized irradiance probes (step 1) | step 2: visibility, classification, idle refresh |
| [0029](0029-puppets.md) | 2D deformable puppets (engine rig format, PuppetSource, eluna bridge) | implemented synthetic-rig scope; later real E-mote work tracked separately in STATUS |
| [0030](0030-audio-mixer-and-music.md) | Shared SFX, spatial audio and streaming music | implemented first slice |
| [0031](0031-rich-text-shaping-and-sdf.md) | Shared rich text shaping, layout, reveal and CPU SDF atlases | implemented; SDF GPU integration pending |
| [0032](0032-input-rebinding-gamepads.md) | Runtime action rebinding and desktop gamepads | implemented; no hardware pad validation |
| [0033](0033-localization-rich-text-surfaces.md) | Localization catalogs and shaped text surfaces | implemented first slice |
| [0038](0038-shared-bitmap-operations.md) | native image operations become shared Bitmap capabilities | implemented shared CPU bitmap scope; native consumers require separate acceptance |
| [0040](0040-level-sequences.md) | Level sequences | implemented; GoW ANM not decoded |
| [0041](0041-engine-capability-roadmap.md) | Engine capability roadmap (what a complete engine still needs) | living list |
| [0042](0042-nfs-carbon-import.md) | Need for Speed: Carbon (PC) as an import target, starting with the canyons | canyon renders; instances/cars/vehicle pending |
| [0043](0043-arcade-vehicles.md) | Arcade vehicle controller | implemented; state not saved |
| [0044](0044-archive-format-resource-plugins.md) | Capability archive resources and explicit format plugins | implemented initial archive contract; expanded migration and variant limits in 0065 |
| [0045](0045-rapier-shared-physics.md) | Persistent shared Rapier physics | implemented bounded Rapier backend/API scope; CPU/runtime evidence recorded |
| [0046](0046-native-image-resource-pipeline.md) | Native image resources through the shared engine | implemented named image-pipeline scope; per-variant limits in resource ledgers |
| [0047](0047-engine-api-capability-extraction.md) | engine API capability extraction, no hosted engines | accepted active ownership rule; former hosted paths removed |
| [0048](0048-reallive-vm-engine-apis.md) | original RealLive VM calls shared engine APIs | API bridge implemented; opening-profile evidence separate |
| [0049](0049-multithreaded-tasks-and-async-assets.md) | Bounded engine workers and asynchronous resources | implemented stage-one workers; does not complete 0050 |
| [0050](0050-engine-wide-async-execution.md) | Asynchronous, multithreaded execution across the entire engine | accepted engine-wide requirement; partially implemented |
| [0051](0051-hardware-ray-query.md) | Hardware ray queries with capability-based backend selection | implemented hardware-query foundation; extended by 0055–0060; device validation remains scoped |
| [0052](0052-frame-pipeline-streaming-and-profiling.md) | Frame extraction graph, profiling, streamed cells and mesh RT instances | implemented slice; engine-wide async migration remains partial |
| [0053](0053-krkr-tjs2-adapter.md) | KiriKiri (TJS2) adapter over engine Layer/Canvas services | implemented first-class TJS2/KAG adapter profile; KAG3 extensions in 0088, full compatibility partial |
| [0054](0054-fbx-pmx-vrm-import.md) | FBX, PMX and VRM model import through the glTF pipeline | implemented bounded FBX/PMX/VRM and morph scope; remaining losses recorded |
| [0055](0055-rt-gpu-geometry-dynamic-blas-alpha.md) | RT from renderer GPU streams; dynamic BLAS; alpha-tested casters | implemented GPU-stream/dynamic/alpha-test scope; recorded Metal validation |
| [0056](0056-rt-soft-shadows-denoising.md) | Soft ray-traced sun shadows with temporal/spatial denoising | implemented soft-sun/denoising scope; recorded Metal validation |
| [0057](0057-rt-local-light-shadows.md) | Ray-traced point/spot light shadows | implemented bounded local-light shadow scope; recorded Metal validation |
| [0058](0058-rt-reflections-material-atlas.md) | Ray-traced reflections; material atlas and corner attributes | implemented hit-atlas/mirror scope; glossy extension in 0060; recorded Metal validation |
| [0059](0059-rt-irradiance-probes.md) | Ray-traced irradiance probe updates | implemented RT probe scope; per-pixel extension in 0060; recorded Metal validation |
| [0060](0060-glossy-and-per-pixel-ray-lighting.md) | Glossy reflections and per-pixel ray-traced diffuse GI | implemented bounded glossy/per-pixel/local-hit-lighting scope; recorded Metal validation |
| [0061](0061-siglus-plain-message-engine-apis.md) | Siglus plain message commands through shared text APIs | explicit plain profile; full native MWND/game compatibility pending |
| [0063](0063-native-font-measurement.md) | Native font callbacks use engine text measurement | implemented immutable metrics/native callback scope; recorded independent/native tests |
| [0064](0064-modular-vfx-cpu-gpu.md) | Engine-owned modular VFX on asynchronous CPU and resident GPU paths | implemented modular CPU/GPU profile; full Niagara-level scope remains incomplete |
| [0065](0065-native-resource-codecs-and-async-decode.md) | Shared native resource codecs and owner-polled decode work | implemented named decoder migration scope; variant limits remain explicit |
| [0066](0066-awaitable-services-and-native-plugin-facades.md) | Awaitable services and native plugin facades | implemented awaitable service/menu profile; full KAG3/engine async remains partial |
| [0067](0067-shared-acoustics-and-cpu-bvh.md) | Shared acoustic geometry and asynchronous direct sound | implemented direct-sound scope; HRTF/reverb/reflections extended in 0068/0073–0075 |
| [0068](0068-hrtf-resources-and-binaural-voices.md) | Engine HRTF resources and worker-prepared binaural voices | implemented bounded SOFA/HRIR convolution scope; later DSP/output workers in 0079/0080 |
| [0069](0069-worker-audio-decoding-and-playback.md) | Worker-owned audio decoding and owner-published playback | implemented buffered decode scope; native/canonical/media follow-ups in 0070–0072, DSP/output in 0079/0080 |
| [0070](0070-native-channel-audio-readiness.md) | Native channel audio readiness and shared capability streams | implemented native readiness scope; canonical/media follow-ups in 0071/0072; DSP/output in 0079/0080 |
| [0071](0071-canonical-audio-worker-bridge.md) | Canonical SceneState audio worker bridge | implemented canonical voice bridge; media/DSP/output follow-ups in 0072/0079/0080 |
| [0072](0072-worker-prepared-media-pcm.md) | worker-prepared media PCM and bounded decoder lifetimes | implemented bounded media PCM scope; final DSP/output follow-ups in 0079/0080 |
| [0073](0073-shared-parametric-reverb-buses.md) | shared worker-prepared late-reverb buses | implemented parametric reverb scope; geometric/tracked reflections followed in 0074/0075 |
| [0074](0074-captured-geometric-early-reflections.md) | captured geometric early reflections | implemented captured first-order reflections; tracking in 0075 and instance snapshots in 0082 |
| [0075](0075-tracked-reflection-plans.md) | tracked reflection plans and persistent sample history | implemented first-order tracking/crossfade scope; direct Doppler in 0084, reflected-path Doppler remains open |
| [0076](0076-shared-audio-transport-and-siglus-bgm.md) | shared audio transport and original Siglus BGM | implemented native BGM scope; retained replay/audio families extended in 0077–0086 |
| [0077](0077-prepared-audio-resources-and-native-replay.md) | immutable audio resources and native retained-source replay | implemented prepared-source replay scope; later native audio/DSP extensions in 0078–0086 |
| [0078](0078-mix-groups-and-siglus-pcm-se.md) | shared gain groups, playback age and Siglus PCM/SE | implemented gain-group/PCM/SE scope; voice/PCMCH/cue extensions in 0081/0083/0086 |
| [0079](0079-worker-final-audio-frames.md) | exclusive worker DSP frames and owner publication | implemented worker final-DSP scope; independent audio render clock remains open |
| [0080](0080-async-output-worker-and-bounded-pcm-transfer.md) | asynchronous output worker and bounded PCM transfer | implemented worker/device-transfer scope; local CoreAudio evidence, other platforms unverified |
| [0081](0081-prepared-voice-compression-and-siglus-koe.md) | prepared PCM voice compression and original global Siglus KOE | implemented global profile; full speech/mouth/routing/discovery remains open |
| [0082](0082-acoustic-instance-snapshots.md) | reusable acoustic mesh BVHs and moving instance snapshots | implemented affine/enabled profile; deformation, prototypes and TLAS refit remain open |
| [0083](0083-shared-transport-and-siglus-pcmch.md) | shared transport clocks and original Siglus PCMCH | implemented bounded profile; full native routing/events/discovery remain open |
| [0084](0084-kinematic-doppler-and-motion-capture.md) | shared direct Doppler and owner motion capture | implemented direct model; propagation delay and per-path Doppler remain open |
| [0085](0085-owner-clock-cue-programs.md) | Owner-clock cue programs and dependency receipts | implemented bounded cue scheduling; native consumer in 0086 |
| [0086](0086-native-pcmevent-and-portable-cue-cursors.md) | Native PCMEVENT over shared cue programs | implemented bounded native cue profile; exact broader native timing remains separate |
| [0087](0087-logical-input-journals-and-native-continuations.md) | Logical input journals and native audio continuations | implemented framed input journal; raw OS/IME/analog coverage remains open |
| [0088](0088-kag3-semantic-adapter-and-shared-prerequisites.md) | KAG3 semantic adapter and shared engine prerequisites | implemented bounded KAG3 facade; full games/plugins/saves remain partial |
| [0089](0089-shared-opna-and-pmd-sources.md) | Shared OPNA synthesis and bounded PMD sources | implemented bounded OPNA/PMD sources; full chip/driver and UK2 VM remain open |
| [0090](0090-shared-worker-image-resources.md) | Shared worker image resources and native effect facades | implemented bounded worker image/effect scope; later presenter capture tracked in STATUS |
| [0091](0091-authenticated-hxv4-resources-and-bootstrap.md) | Authenticated HXV4 resources and explicit executable bootstrap | implemented bounded HXV4/bootstrap scope; arbitrary protected callbacks remain separate |
| [0092](0092-shared-documents-and-native-json-facade.md) | Shared document grammar and native JSON facade | implemented bounded document/JSON scope; synchronous read boundaries remain |
| [0093](0093-shared-vector-and-native-draw-facade.md) | Shared vector geometry and native LayerExDraw facade | implemented solid-linear vector/native facade; full GDI semantics remain open |
| [0094](0094-isolated-layer-subtree-crossfades.md) | Isolated retained-layer subtree crossfades | implemented bounded isolated subtree transitions; immutable-resource limits remain explicit |
| [0095](0095-avg32-installed-games-shared-raster.md) | Installed AVG32 games over a shared raster device | implemented first-class installed-game AVG32 runtime; replaces 0062 runtime, not full compatibility |
| [0096](0096-cxdec-xp3-content-filter-and-special-index.md) | CXDEC XP3 content filter and indirect Special filename index | implemented bounded static CXDEC/Special scope; interpreted filters remain proposed in 0097 |
| [0097](0097-krkr-protection-interpreted-and-lifted-filters.md) | KiriKiri archive protection without JIT, brute force or bulk extraction | proposed interpreted/lifted filter design; static CXDEC/HXV4 remain the implemented paths |
| [0098](0098-client-scoped-gpu-mesh-frame-history.md) | Client-scoped GPU mesh frame history | implemented client-scoped mesh-frame history profile; remaining async scope explicit |
| [0099](0099-staged-original-save-restore.md) | Staged original-save restoration | implemented bounded original-restore coordinator; native consumers remain partial |

| [0100](0100-browser-host-and-capability-runtime.md) | Browser host and injected Runtime capabilities | 2D/PBR host and shared CPU workers; native adapters and broader migration remain partial |

## Superseded decisions

| ADR | Decision | Status / follow-up |
| --- | --- | --- |
| [0008](0008-native-engine-integration.md) | Native engine platform-seam hosting (superseded) | superseded by 0047; hosted acceptance withdrawn |
| [0009](0009-siglus-full-engine.md) | Siglus native renderer hosting (superseded) | superseded by 0047; original renderer/plugin recipe retired |
| [0037](0037-bgi-local-runtime-trial.md) | local BGI trial and a device-free CPU embedding experiment | superseded by 0047; local hosting experiment retired |
| [0039](0039-bgi-shared-service-adapter.md) | BGI as a native VM calling CrySirror services | superseded by 0047; old integration acceptance withdrawn |
| [0062](0062-avg32-portable-vm-engine-apis.md) | Original AVG32 VM library with engine service commands | runtime superseded by 0095; shared arithmetic/source evidence retained |

## Historical verification and source audits

| ADR | Decision | Status / follow-up |
| --- | --- | --- |
| [0034](0034-siglus-runtime-verification.md) | Verify Siglus execution separately from shared engine features | historical verification checkpoint; later native messages in 0061 |
| [0035](0035-native-family-coverage.md) | RealLive / AVG32 source coverage and integration boundary | historical source audit; later adapters in 0048/0062/0095 |
| [0036](0036-siglus-service-boundary-audit.md) | Siglus shared-service coverage must be verified through the native VM | historical boundary audit; later per-family native mappings in NATIVE_ADAPTERS |

## Requirement backlog

Engine requirements tracked by these ADRs. Completion refers to the defined
scope and its recorded behavior validation.

| ID | Requirement | ADR | State |
|---|---|---|---|
| R1 | General-purpose (Unity-like) engine; VN engines are clients, not the core | 0002, 0004 | in progress |
| R2 | Extract shared features (world, transforms, scaling, text, UI) into CrySirror | 0004 | world/layer/UI/text exist; object API wraps world |
| R3 | Everything reachable through an OOP API (classes, properties, methods) | 0004 | class registry + World/Entity classes |
| R4 | Engine lifecycle abstracted (init/start/update/fixed/late/pause/stop) | 0004 | implemented in crysirror-engine |
| R5 | Not script-centric, but can export OOP API to scripts | 0004, 0005 | implemented for JS |
| R6 | Default scripting: Rust-implemented TS/JS | 0005 | Boa runtime host + Oxc transpile |
| R7 | Other VMs (siglus_rs SceneVm, RFVP) plug in as script hosts | 0005, 0008, 0048, 0061, 0062 | independent first-class native adapters; per-profile implementation and acceptance tracked in NATIVE_ADAPTERS |
| R8 | Abstract filesystem; native engines keep their own FS semantics | 0006 | mount table + providers |
| R9 | Load Blender models | 0007, 0010 | glTF 2.0 import incl. textured PBR; .blend via Blender CLI export |
| R10 | Ray tracing | 0003, 0007, 0051, 0055–0060 | native primary/shadows, GGX reflections, probe and per-pixel GI; broader hit shading/denoising and multi-device validation remain |
| R11 | DLSS-class upscaling | 0007 | capability contract only; vendor SDKs not distributed |
| R12 | Asynchronous/multithreaded execution across every engine subsystem | 0049, 0050, 0052 | required; resource/pose/extraction, text/audio and staged restore worker paths exist; full subsystem migration remains partial |

- [0101 Native packages and console seams](0101-native-host-packages-and-console-seams.md)
