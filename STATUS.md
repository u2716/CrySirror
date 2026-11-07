# Implementation and validation status

## 2026-11-08 — Majiro playable bring-up (adopt)

- Implementation: `crysirror-majiro` now executes the owner game 暁の護衛 through the title, scenario text, choices UI, save and load. VM: arrays, click/AIP/destructor targets (0x83B..0x847), destructors on return and kill, argument stores, native string storage, `RunScript`, same-update `wait(-2)` rounds. Pages/sprites are retained native pictures composed with the new shared `crysirror_image::transparency_planes` kernels and shared RCT/RC8 plane readers; the primary picture is published as an engine image layer. Text uses engine glyph jobs with native PROC layout and hooks. Input/hit regions, music/sound/voice, buffered file readers, message history recording/replay, portable settings and portable save records (not MajiroSavV1.103) are connected. Contract and gaps: `crates/crysirror-majiro/README.md`.
- Compilation: `cargo build --release -p crysirror-desktop --bin crysirror` passed. API reference regenerated with the release example; fingerprint unchanged (`4c6395d28206ec40`).
- Tests: `cargo test --release -p crysirror-majiro --features runtime` passed (4 unit, 1 bridge, 5 native; 2 owner-corpus tests remain ignored). Two settings unit tests and the bridge test were rewritten for the new contracts; the native division test now uses the defined fault case. `crysirror-image --lib transparency_planes` passed (2). The existing `crysirror-image` integration test `tests/effects.rs` does not compile (missing `rule_table`), unrelated to this change.
- Behavior (local, headless `play --headless --no-audio` with input tracks): logos, title (frame ~1470), START into scenario 001 with dialogue and event CGs through 8000 frames, SAVE menu, slot save, LOAD menu with slot date/label, load confirmation and restore, backlog opened by wheel-up showing replayed history pages. A 40 s windowed run with audio decoded the Ogg streams without errors. Not exercised: backlog voice replay, movies (skipped with a warning), save thumbnails, auto/skip modes, endings.

## 2026-11-03 — Original-pixel desktop window and free resizing

- Implementation: desktop startup requests the original canvas size in physical pixels instead of logical desktop points; reported display dimensions use the same units. The host permits resizing the presentation independently of the native script's retained window style. Existing aspect-fit rendering and original-canvas pointer mapping are unchanged.
- Compilation: `cargo build --release -p crysirror-desktop --bin crysirror` passed using the existing target tree; no debug build. No public engine API changed.

## 2026-11-03 — Fractional-scale clipped atlas sampling

- Implementation: shared `crysirror-wgpu` layer vertices carry exact fragment clip bounds and isolated UV sampling bounds. Clamp-to-edge atlas views, including quarter-turn and snapshot layers, no longer sample neighbouring texels across an axis-aligned clip. Repeat/mirror addressing and distortion retain their sampling policy. Public engine APIs and `Vertex2d` are unchanged.
- Tests: the two explicitly enabled `image_sampling` GPU tests passed on Apple M5/Metal, covering RGBA/BGRA, quarter-turn, filtered and snapshot atlas views; Python tooling tests passed (148). A four-frame helmet hardware-raytrace render also exited successfully with `hardware_raytracing: true`.

## 2026-11-02 — RealLive / NScripter implementation and validation

- RealLive: `adapter/selection.rs` implements `select_w` in its own window with others closed, `select`, `select_s` with SELBTN images, colour/title/hide/blank/cursor effects, hover, keyboard and SE. `frame_geometry` adds right/bottom origins and content-fitted type 3 frames; nine-slice type 3/4 frames are engine layers. Text windows in type 3 frames use the full text area and do not shrink.

- NScripter: `commands.rs` adds about 250 ONScripter commands and syntax: arrays, `~` jumps, math and strings, sprite2, sprite animation, buttons, csel, dialogs, input, movies, audio aliases, private-format save slots, right-click menu, lookback, skip and auto modes, ruby, `%n`/`$n` in text, quake, monocro, nega, draw overlay and screen copies. Save slots do not interchange with original `.dat` files. `drawsp3`, `exec_dll`, `luacall` and `luasub` fail visibly.

- crysirror-nscripter: 13 passed, 1 ignored. These are authored scripts; `~/Downloads` holds no NScripter title, so no real game ran. Screen-copy commands compile but were not executed.

## 2026-11-01 — RealLive implementation and validation

- RealLive LB: windowed exit 1, `asset g00/TT_TTA_RP00.g00 Failed: image dimensions must be 1..8192`. Original sheets reach 1200x12000 (10 files in LB; After also has files above 8192). `crysirror_image::rgba_len` now allows 16384; the 64 MiB budget and the renderer's device-limit check still apply. A GPU limited to 8192 fails visibly on these images.

## 2026-11-01 — Desktop links every engine; no per-engine build features

Implementation:
- Owner request: plain `cargo build` produces the complete player. Removed desktop features `reallive`, `avg32`, `nscripter`, `circus`, `majiro`, `system-nnn`, `system4`, `scripting`, `emote-import` and `arc-formats`, with their `cfg` arms and "feature not linked" errors. Desktop features remaining: `audio` and `gamepad` (default), `acoustics-obvhs`. Library-crate features (`runtime`, `arc-formats`, `blackcyc-formats`) are unchanged; the desktop enables them.

Validation (local):
- `inspect` reports the adapters as registered. Release binary `play --headless --frames 120 --no-audio` exited 0 for `~/Downloads/Game_RUSUR` (RealLive). Boot/exit only; pixels, audio and interactive windowed play were not checked.
- Not run: AVG32 (`~/Downloads/game_kanon` is absent), Circus, Majiro, System-NNN, NScripter and System4 titles.

## 2026-11-01 — AVG32 implementation and validation

- AVG32 (separate agent): original input/rng split adopted; AVI/Cinepak demux/decode added to shared crysirror-video (`Container::Avi`, oxideav-cinepak =0.0.3, MIT) and used by the original movie policy on the engine clock; `0x66` text into non-zero PDT work buffers now uses the engine glyph service plus PdtManager coverage blending. Details and remaining gaps in crates/crysirror-avg32/UPSTREAM.md.

- crysirror-avg32: 21 unit + 8 integration passed; crysirror-video 27 passed, 4 ignored. Kanon (local/work/avg32-kanon/project/assets/game) headless runs reached the title, main menu, scene 70 choice and scene 501 with zero warnings; the new Cinepak and work-buffer text paths were not exercised by Kanon.

## 2026-11-01 — System4 implementation and validation

- Alice original Rance7 owner test passed1/1,11256 instructions to A_PUSHBACK0x84 at pc0x1007c/mes_mark_metrics_t@AddCG; main and desktop factory were still absent in that frozen version. Shared ALD owner test passed1/1 with6606 original numeric entries across two volumes6506+100. Agent continuation is newer source and has separate verification requirements.

## 2026-11-01 — Circus implementation and validation

- Circus accepted falling type1 now mirrors original save words only after shared VFX completion; type0 preserves the previous intensity while clearing the native effect-family flags. This completes another save-state prerequisite, without admitting an incomplete original slot or bypassing autosave0.

- Circus release library passed47 with1ignored (native-falling-save-tests.log), including pending shared-effect receipt preserving PC/body and native intensity retention. Original autosave/thumbnail admission remains blocked.

## 2026-11-01 — Circus native save projection prerequisites

Implementation:
- Inspected the pinned DC4PHDL dispatcher and original402A10 byte-difference history routine with local static IDA analysis. Accepted4f/50 scene transfers now record changes only after the target resource is ready, in native index order. The1000-record limit retains the comparison-bank updates even when no further history entry fits; it does not overwrite the adjacent native241 field.
- Native opcode63 now appends to the actual save-visible version bytes using strict Shift-JIS capacity<128. Oversized suffixes preserve the old string, matching the original conditional append. Existing assignment/comparison/palette branches have audited live-body coverage; this does not certify unaudited scene cleanup or resources.

Compilation and independent behavior:
- Release Circus library tests passed44/44 with1 owner-corpus test ignored (target/verification/circus-execution/native-save-completion2-tests.log). New coverage exercises1100changed bytes/history saturation, retained tails after reset, encoded Japanese version limits, and an original instruction scene transfer that remains Pending with unchanged history until resource completion.
- Actual DC4PHDL prologue compatibility remains incomplete at DC4_SEN20181213B pc0x83e opcode18 selector4 (mandatory original autosave0). The original writer uses the earlier captured mirror, not a later live-state approximation. Full scene-reset/sprite/movie projection and the original selected128x72thumbnail still need completion. No new real-game progress, skipped save, fabricated body or successful original-save claim is made by these tests.

## 2026-11-01 — RealLive LB system-settings empty clip

Implementation:
- Fixed the native object-to-Layer clip projection. RealLive screen-space clips are intersected with the destination screen; empty intersections emit no drawable/clip group. Original object and rectangle state remain intact, so a later nonempty clip restores the same object. Shared Layer validation stays strict; no original renderer or private pixel operation was added. The inspected source pin and exact failing slider are recorded in docs/reallive-empty-clip-source-audit.json.

Compilation and independent validation:
- Release RealLive library tests passed48/48 and engine bridge tests passed15/15 (target/verification/reallive-lb-clip/reallive-clip-tests.log). The authored original-bytecode empty/visible/empty test first reproduced the same invalid-layer-clip error before the fix, then passed. Additional coverage includes zero/negative dimensions, offscreen and bounded positive clips. Relevant Python schema tests passed8/8 (python-schema-tests.log).
- Combined release player27 built successfully with defaults plus nscripter,arc-formats,circus,majiro,system-nnn. Its immutable SHA256 is 8d7c607fb330c43779744e80924979b0085c3d624ab233e907db2a982ec6fdd7. No public signature changed, so no new API fingerprint is required.

Actual game behavior:
- Player25 reproduced the owner's game-rl-LB failure by opening Config/system settings: the native slider submitted clip[351,208,0,8] for MN_CF_SB03 and exited with invalid layer clip. This is separate from the earlier title/story routes, which did not reproduce it.
- Player27 opened the actual settings page at5000frames (scene9001:865,1714780original instructions, error=null). A7500frame input track including slider changes and subsequent menu interactions also completed with error=null; its actual settings capture was visually inspected. Both retained127layers/21strictly positive clips. Evidence: after-settings-report.json/png and after-roundtrip-report.json/png under target/verification/reallive-lb-clip. Captures used Apple M5/Metal offscreen, without a physical window/audio device; this does not claim full-game/save compatibility.
- A separate6300frame capture confirms the intervening EXIT actually returned to the title (scene9012:541,2370441instructions, error=null; after-exit-report.json/png), followed by the7500frame capture's actual re-entry into settings. This verifies the settings/slider/exit/reopen route rather than inferring it solely from an input track.

## 2026-10-31 — shared raster jobs, immutable Canvas images and installed NNN admission

Implementation:
- Added client/runtime-scoped persistent pattern resources and nonblocking worker receipts. Create/Advance/Restore preserve immutable generations and return portable RNG candidates; rendering uses the common image pipeline and memory reservations. Native policy commits its RNG only after its own receipt succeeds.
- Added HostServices.begin_canvas_image and explicit Runtime presenter admission. A request freezes engine Canvas nodes, published image leases and font membership. The existing wgpu compositor prepares a retained texture; readback waiting, depadding and immutable ImageResource construction execute on a worker. Desktop window, render and in-place offscreen paths install/poll the same service. Live Puppet and replaceable surface/history dependencies fail explicitly. Explicit readback snapshot cache generations are bounded; automatic PNG compaction remains a separate synchronous path.
- Image release now ends caller ownership while retained Canvas/Layer/filter/transition/mesh and World/VFX references keep the published path alive. Owner collection retires unreferenced paths without serializing mesh/particle payloads; open Layer transactions defer collection. Frozen frame/capture leases remain usable after live lookup ends.
- Added a dormant System-NNN installed factory and detection profile. Original MSI installed directories can be mounted at a selected shared windows/msi root alongside loose game resources, with original record IDs retained. Boot/config/script work uses scoped resource workers and the original reviewed callback module guard. Clock, image, text/UI and audio preparation use public engine services. The native controller remains the default entry; no data VM or original executable is substituted. This is partial integration; registration is not a playable-title or original-save claim.

Compilation and independent behavior:
- Release Runtime/WGPU check passed (canvas-image-check.log); combined desktop check with nscripter,arc-formats,circus,majiro,system-nnn passed (desktop-installed-check.log). Later NNN window-resource wiring is also being checked by the combined player build; no newer whole-workspace success is claimed.
- Shared resource release tests passed6/6: Canvas jobs3, pattern resources2, RGB lookup/publication1 (shared-resources-final.log). Coverage includes worker-only short reads, frozen Canvas mutation, retained terminal receipts, client isolation, publication release/retirement, hidden nested snapshots, World material references and frozen render leases.
- Explicit offscreen Metal Canvas capture test passed1/1 on Apple M5 (canvas-image-gpu-final.log). It uses engine-owned nested groups and retained copies, mutates/removes live input afterward, then verifies straight RGBA[255,0,0,128] from the immutable image job. This is an engine-only GPU test, not an original-game title result.
- Shared MSI tests passed3/3 with1 owner test ignored by default (msi-root-tests.log); independent dormant factory test passed1/1 (installed-tests-final.log). Original MUGEN2 detection tests passed2/2 including its explicitly invoked owner-media test: actual catalog/module selects system-nnn and a scoped installer root without extraction or executable execution (detect-installed-tests.log). Original NNN gameplay/title/save compatibility remains unverified by these tests.

Follow-up startup validation:
- Actual player17 MUGEN2 controller invocation detected/mounted the installer, but20,000 fast offscreen frames ended with bootstrap_pending=true/phase=preparing. Its error=null and empty capture are **not** a successful native-game run: the simulated clock outran the cold worker before any original instruction started. Evidence: player17-controller-report.json/log.
- Added the opt-in VmAdapter.preparation_pending boundary and Runtime.poll_vm_preparations. Headless startup now polls only dormant preparing VMs before counting game frames, with no clock advance, peer replay or input consumption. Window hosts remain nonblocking. Independent release test passed1/1 and proves first normal input still reaches both peers exactly once (vm-preparation-tests.log). Combined player18 build and the next actual native run remain pending at this checkpoint.

Follow-up source precedence and device-free media preparation:
- Shared fallback mounts now keep existing overlays and actual loose files ahead of the mounted provider. Missing files alone fall back; decoder failure never selects a different source. Per-path origin queries report the bytes actually served. Resources.mountDecode adds an optional fallback flag while preserving the previous two-argument behavior. Independent release fallback test passed1/1 (fallback-mount-tests.log), and combined desktop release check passed (fallback-desktop-check2.log).
- Actual MUGEN2 media contains four-byte MSI GPK/GTB placeholders alongside full loose resources. Correcting source priority gets past those placeholders; weakening the shared BlackCyc parser was unnecessary. Player19 then exposed the original controller's repeated StoryName/SUB1/END sequence, confirming the missing native outer startup/title controller. This is not a successful title or gameplay result; the native adapter work continues separately.
- Added client-scoped HostServices.begin_media_probe plus Media.beginProbe/probeStatus/releaseProbe. Header/decoder preparation runs on bounded shared workers, retains completion and supports cancellation without creating a playback slot, picture, Layer, PCM or clock advance. Unknown audio identity/duration stays unknown; unsupported AlphaMovie header-only preparation fails explicitly. Public tickets become obsolete after Runtime restore.
- Release Engine check passed (probe-routing-engine-check.log). Independent Runtime MediaProbe tests passed2/2 (media-probe-tests3.log), including a sparse100MiB MPEG resource, retained malformed-header failure and subsequent admission. Public Media facade/restore-ticket test passed1/1 (media-probe-api-tests.log). These establish engine capability behavior, not original NNN movie or title compatibility.
- Added reflective InputRoutes methods/global over the canonical shared Scene router, with no private engine-API queue. Independent public receiver test passed1/1 and command API coverage passed1/1 (input-routes-api-tests.log): signed priority, retained leases, unhandled propagation, foreign/stale receipt rejection, registry retention and transient input retirement after restore. Native callback semantics remain the adapter's responsibility; this public API result does not establish game compatibility.
- Regenerated docs/api for MediaProbe and InputRoutes; public reference tests passed8/8 (api-probe-routes-verified.log), fingerprint1dbdf921cc54e571 logged in docs/api/CHANGES.md. New API source and documentation diff checks passed. The next combined player22 build and native-game behavior checks are separate pending validation.

Snapshot: 2026-10-25. Owner priority: the new AAA engine first;
compatibility layers are secondary (ADR 0010).

## In-place launch, native menu, window/fast-forward fixes (2026-10-30)

- `crysirror play [game-folder|file]` detects an installed game and constructs its resource roots. `--detect-only` prints the plan. `Runtime::with_asset_root` takes the chosen folder; saves use a per-user state directory keyed by that folder.

## AVG32 installed games: Kanon through shared services (ADR 0095, 2026-10-30)

- Implementation: `crysirror-avg32` now vendors the original AVG32
  interpreter from the original project instead of the portable-vm Git build. The PDT bank,
  copy/mask/fade/stretch primitives and all transitions moved to
  `crysirror_image::avg32` (plus buffer-0 write damage and a present
  revision). The screen is one engine surface layer; buffer-0 text is a
  retained GlyphRun; menus/backlog/name entry are solid + GlyphRun layers;
  audio is engine channels; files and `SAVE.INI` use client storage and the
  save mount. Config is `{root, font, encoding?}` (breaking, API fingerprint
  `c6204fad405840ca`, logged).
- Install: owner-supplied Kanon CloneCD Disc 1 extracted per its SETUP.INI
  copy list to `~/Downloads/game_kanon`, CD-DA tracks 02-23 ripped to
  `BGM/TRACKnn.WAV`; Disc 2 (12 audio tracks, not referenced by
  GAMEEXE.INI) ripped to `DISC2/`. Unmodified upstream `avg32_probe` on the
  install reached SEEN050 prologue in 3000 frames.
- Compilation: `cargo check --release --workspace --all-targets` passed
  (after giving the vendored `reallive-dll` tests a test-only `game_fs`
  shim). Desktop release build with default features passed.
- Tests: avg32/image/reallive-dll/reallive/desktop/runtime release suites
  330 passed, 0 failed, 5 ignored; 4 new synthetic-install AVG32 tests
  (screen surface, GlyphRun text, `#CDTRACK` BGM, glyph removal on
  overwrite, movie warning, read-only game files/save mount), plus a fifth
  added afterwards for choice-row highlight (glyphs recoloured, not removed);
  `crysirror-avg32` then passed 25 (20 vendored unit + 5 integration). Python 3.11
  tooling 195 passed. API docs 7 passed.
- Behaviour (Kanon, release `render`/`trace`, Apple M5/Metal, scripted
  input): KEY logo and title, the graphical main menu, prologue text in the
  message frame, the right-click system menu, save to slot 5 and load from
  the title (prologue restored, 29 glyphs), and a Ctrl-skip trace into
  SEEN501 "１月８日 金曜日" after 12000 frames with 52 audio requests and
  no warnings. Images in `target/verification/avg32-kanon/`. A 900-frame
  windowed `run` exited cleanly; audible output was not verified. Local
  launcher: `~/Downloads/Run-kanon.command` (system Hiragino font, saves in
  `~/Downloads/CrySirror-kanon-state`).
- Limits: not played to completion; choices were only clicked blindly
  during skip. Text into PDT buffers other than 0, Cinepak movies,
  `FN.DAT`/`＊００` glyphs as engine glyphs, framework snapshots and script
  `Bitmap` exposure of the AVG32 raster primitives remain open. Glyph
  visibility follows buffer-0 writes, not native present timing.
- RealLive DLLs: the owner's EF00/PT00/DT00 emulation (`reallive-dll`) is
  wired to `LoadDLL`/`CallDLL`; its upstream tests now run (4 passed; the two
  game-DLL cases return early without a Little Busters!/Tomoyo After
  install). No DLL-using game has been run.

## RT step 6: ray-traced irradiance probes (ADR 0059)

- `irradiance_volume.ray_traced`: probes refreshed by 256 rays each against
  the RT scene (sun + RT shadow, emission, volume multi-bounce, environment
  on misses) into the same SH/visibility data as raster captures.
- Behaviour (Apple M5, Metal): closed red-emissive room floor black without
  GI, (245, 60, 60) raster probes, (211, 50, 50) ray-traced probes. Hardware
  RT GPU tests **8 passed**; whole `crysirror-wgpu` ignored GPU suite
  `--test-threads=1` **43 passed**. Cost relative to raster captures not
  measured.

## RT step 5: ray-traced reflections (ADR 0058)

- `reflections.ray_traced`: mirror rays traced on the hardware path; hits
  visible this frame reuse the previous frame's colour, others are shaded
  from a baked all-material atlas, GPU-gathered UV/normal attributes, the
  sun with an RT shadow and SH irradiance. Shadow queries now use the
  closest-hit flag (fixes a possible missed occluder behind alpha holes).
- Behaviour (Apple M5, Metal): hardware RT GPU tests **7 passed**; whole
  `crysirror-wgpu` ignored GPU suite `--test-threads=1` **42 passed**.
  Off-screen box reflected: RT 2348 red pixels, SSR 0, none 0.

## RT step 4: ray-traced point/spot light shadows (ADR 0057)

- Nearest two shadowed point/spot lights traced into the spare B/A
  channels of the RT visibility target (16-texture binding budget kept);
  `LightSpec.source_radius` gives denoised penumbrae; others keep atlas
  shadows.
- Behaviour (Apple M5, Metal): hardware RT GPU tests **6 passed**; whole
  `crysirror-wgpu` ignored GPU suite `--test-threads=1` **41 passed**.
  Point light: 1389 shadowed floor pixels; 0.35 m emitter: sharp floor
  shadow edges 119 → 79.

## RT step 3: soft sun shadows and denoising (ADR 0056)

- `Sun.angular_diameter_degrees` (default 0): 2 cone-sampled shadow rays per
  pixel and frame, temporal accumulation with depth-validated reprojection
  (≤ 32 samples) and a 5×5 edge-aware spatial filter. 0 keeps the exact
  hard ray.
- Behaviour (Apple M5, Metal): hardware RT GPU tests **5 passed**
  (release, `--test-threads=1`); soft test: sharp floor edges 211 → 146 px,
  converged frame-to-frame mean |ΔG| 0.008. Sponza (local) with a 2° sun
  renders clean penumbrae.

## RT step 2: GPU-stream BLAS, dynamic BLAS, alpha-tested shadows (ADR 0055)

- PBR hardware RT builds BLAS from the renderer's GPU vertex/index streams
  (no CPU triangle copies); skinned/morphed instances own resident BLAS fed
  by a GPU skinning pass and rebuilt only when pose/morphs change. wgpu
  29.0.x performs `PreferUpdate` as a full rebuild: no refit yet.
- MASK casters: GPU-gathered UVs, a 2048² alpha atlas baked with the raster
  coverage function, shadow rays re-traced past transparent hits (≤ 8
  layers). Blended surfaces cast nothing. The old CPU triangle instance
  path was removed.
- Behaviour (Apple M5, Metal):
  - Hardware RT GPU tests, release, `--test-threads=1`: **4 passed**
    (instancing/rigid motion, uniform-alpha mask, checker-alpha texture
    shadow 410 → 205 pixels, skinned dynamic BLAS reuse/rebuild).
  - Whole `crysirror-wgpu` ignored GPU suite, `--test-threads=1`: **39
    passed**.
  - Khronos Sponza (local), `--renderer hardware-raytrace`: renders; 103
    instances, 14 alpha-tested, 3 atlas tiles. Before this step hardware RT
    rejected the scene's masked materials.

## Morph targets (ADR 0054, second step)

- glTF/VRM morph targets load (sparse, `targetNames`); PMX vertex and
  group morphs convert. `Entity.setMorph/setMorphs` store weights in
  `AnimationState.morphs`; whole-scene drawables carry them; the renderer
  blends positions/normals on the CPU into a per-draw vertex buffer used by
  every raster pass, re-uploaded only when that primitive's weights change.
- Not applied: per-mesh `pbr: true` references (reported), glTF weight
  animation channels, FBX blend shapes, PMX non-vertex morphs, hardware RT
  shadow geometry (unmorphed).
- Behaviour:
  - VRM 1.0 Seed-san, release raster on Apple M5/Metal: `eye_smile`,
    `lip_a`, `cheek` at 1.0 close the eyes, open the mouth and add blush
    (local images `target/verification/vrm-face0.png`/`vrm-face1.png`).
  - Rust workspace `--no-fail-fast`: **601 passed / 0 failed / 56 ignored**.
  - Explicit Metal GPU smoke suite, `--test-threads=1`: **29 passed**. Run
    in parallel, 21 of them panic with a wgpu validation error; the
    unmodified previous commit fails identically (pre-existing, not
    investigated).
  - Python 3.11: **169 passed**.

## FBX, PMX and VRM import (ADR 0054)

- glTF/GLB was already loading (DamagedHelmet, Sponza, Fox); a render of
  `examples/aaa-helmet/project.json` without `--script game.ts` is empty
  because the scene is created by that script.
- New `crysirror-asset::interchange`: own binary FBX 7.x and PMX 2.0/2.1
  parsers, converted in memory to GLB and consumed by every glTF path.
- Import reports now list morph targets and unsupported glTF extensions;
  PBR references no longer report skins/animations as dropped; triangle
  counts are filled in.
- Behaviour:
  - `crysirror-asset` tests: **23 passed** (5 new interchange tests, one of
    them comparing local Fox glTF/FBX copies when present).
  - Fox glTF → Blender 5.2.2 FBX → CrySirror: relative bounds error
    2·10⁻⁵ (Survey), 1.9·10⁻⁴ (Walk), < 10⁻⁵ (Run). Release raster render
    of `root|Run` vs the glTF `Run` at the same frame: 1.05% of pixels
    differ by more than 8/255.
  - VRM 1.0 Seed-san (VRM Public License 1.0, local only) renders with
    the unlit fallback; reported: 129 morph targets, VRMC_vrm,
    VRMC_springBone, VRMC_materials_mtoon, VRMC_node_constraint,
    KHR_texture_transform.
  - No real PMX model was rendered (none with a redistributable licence was
    fetched); PMX is validated by a synthetic file only.

## Latest: NFS Carbon canyon decoded and driven; arcade vehicles (ADR 0042, 0043)

- **NFS Carbon geometry and textures** (`tools/nfsc_geometry.py`,
  `tools/nfsc_textures.py`, our own decoders).
  - All 962 Palmont stream sections decode: 40,677 solids and 4.62 million
    triangles, with every vertex normal unit length.
  - 3,496 textures (DXT1/3/5, RGBA8) are indexed and go through the
    engine's DDS codec.
  - The TNDRIFTB canyon was identified from evidence (solid names match the
    disc's canyon movies). Its 17 sections, with 98,431 triangles and 55
    textured materials, render in the engine.
- **Arcade vehicles (ADR 0043).** `Entity.setVehicle` and `Entity.drive`
  provide wheel-ray suspension, bicycle steering, grip and handbrake drift,
  and swept wall collision. They are tested for acceleration, wall stops,
  steering, drift and a 10° ramp.
- **`tools/road_centerline.py`** derives a centred road centreline (tested
  on a synthetic road).
- **Canyon drive.** An autopilot drove about 3 km down the decoded canyon in
  142 s, off the road in 0 of 8,880 frames (`canyon-drive.mp4`).
- Behavior after this step:
  - Rust workspace `--no-fail-fast`: **312 passed / 0 failed / 34 ignored**.
  - Python 3.11: **152 passed**.
  - GPU suite not re-run; no renderer code changed.

## clipping fixes, local data layout, NFS Carbon start (ADR 0041, 0042)

- **Clipping.** New engine APIs, each with tests:
  - `World.sphereCast`;
  - `World.groundPose`;
  - `World.modelInfo(..).bounds` (rest pose, skinned vertices posed
    through their joints).

  The walking slice now sizes its collider from the model, conforms the
  body to the ground and sweeps the camera. A per-frame body-axis sweep
  over 1,500 frames counted 216 clipping frames (14%) with the old capsule
  and 0 with the fitted one.
- **Local data.**
  - Game sources (GoW III PS3, Ashen Vow, the NFS Carbon ISO), converted
    work projects, references and the private .NET SDK are in the
    gitignored `local/` (see local/README.md).
  - Representative renders are in `images/`; `images/third-party/` is
    gitignored.
  - The cleanup of `~/.cache/crysirror` kept only Codex's directories.
- **NFS Carbon (ADR 0042).** `tools/nfsc_bun.py` reads the disc zips
  directly. It walks BUN chunks and parses the 962 streaming sections of
  Palmont (`L5RA`). Geometry and texture decoding are next.
- Behavior after this step:
  - Rust workspace `--no-fail-fast`: **311 passed / 0 failed / 34 ignored**.
  - Python 3.11: **148 passed**. The policy test now skips the gitignored
    `local/` and `images/third-party/` and also forbids `.psb`.
  - GPU suite not re-run; no renderer code changed since the last run of
    29 passed.

## walking character in the GoW hall, puppets, sequences, capability roadmap (ADR 0029, 0040, 0041)

- **One scene a character walks in.** The imported OLY100 hall
  (`hall-walk.ts`, user data, outside the repository) has:
  - level mesh collision;
  - a capsule character controller;
  - the skinned Fox stand-in with walk/run/idle clips and cross-fades;
  - navigation and A* to random goals;
  - an orbit camera;
  - WASD control in `crysirror run`.

  A 320-frame autopilot movie (`hall-walk.mp4`) rendered in 69 s.
  Kratos' own skeleton, skin weights and `ANM_hero` (10.5 MB) are not
  decoded yet.
- **Navigation fix.** Small steep bumps no longer erode the floor. The
  OLY100 hall went from 1,015 to 4,153 walkable cells, with a regression
  test.
- **Puppets (ADR 0029).**
  - The engine owns the rig format, evaluator and `PuppetSource`
    interface, plus bake.
  - The eluna E-mote bridge and `crysirror puppet-import` exist; PSB is
    rejected for layers and packages.
  - Puppet layers render with stencil masks, and the `Puppets` API drives
    them.
  - Not validated on a real E-mote file (none available locally).
- **Level sequences (ADR 0040).** Camera, transform and light tracks with
  spline interpolation, flicker noise and events. In the OLY100 hall: a
  dolly shot and 102 flickering warm lights with embers (`hall-sequence.mp4`).
  - `render --movie-frames/--movie-dir` writes consecutive frames; scripts
    keep running while it captures.
- **Capability roadmap (ADR 0041)** lists done/partial/missing
  capabilities, with priorities and an order of work.
- Behavior after this step:
  - Rust workspace `--no-fail-fast`: **309 passed / 0 failed / 34 ignored**.
  - Explicit Metal GPU (release): **29 passed** (run before the navigation
    change; no GPU code changed since).
  - Python 3.11: **145 passed**.

## generated API reference; GI visibility and refresh cost (ADR 0028 step 2)

- **GI step 2.**
  - Octahedral distance moments per probe with Chebyshev visibility.
  - Probes inside geometry are disabled, detected by their back-face
    fraction.
  - Captures use 32² faces with per-face draw culling and a nearest-first
    sweep order.
  - Converged volumes idle until something changes.
  - Measured:
    - Dark-room leak (mean red): 78.2 without visibility, 4.3 with it.
    - The three buried probes are exactly the disabled ones.
    - An idle capture refreshes 0 probes and reproduces the image.
    - OLY100 GI capture: 46 s → 8.8 s; 2.6 s without GI.
  - A wgpu discard-tracking cost that was quadratic in the number of
    capture passes was found by sampling and avoided by storing instead of
    discarding.
  - New optional fields: `visibility` and `idle_probes_per_frame`.

## GPU cluster selection, VSM static/dynamic split, irradiance GI (ADR 0026–0028)

- **Cluster LOD on the GPU.**
  - A compute pass per draw selects clusters, frustum-culls the main view,
    and writes compacted indices plus indirect args. Every pass draws
    indirectly; shadows use an unculled cut.
  - The GPU cut equals the CPU cut exactly.
  - OLY100: 0.5% of pixels differ from the CPU path; the headless render
    drops from 3.57 s to 2.71 s.
- **VSM static/dynamic split.** Moved, added, removed and skinned casters
  dirty only the pages under their old and new footprints. The depth range
  only grows. A moving box re-rendered 4 of 8 pages, and the static
  pillar's shadow stayed identical.
- **Irradiance-volume GI** (`irradiance_volume` component).
  - Probes are refreshed round-robin by 6-face captures with full shading,
    projected to SH L1 on the GPU, and stored in the shadow-data buffer.
    Bounces accumulate.
  - A red emissive wall lights a closed room that is black without GI.
  - OLY100 smoke run: 46 s for a 16-frame capture, dominated by probe
    refreshes.
  - Not done: light-leak visibility and sun shadows in captures.
- Behavior after this step:
  - Rust `--no-fail-fast`: **226 passed / 0 failed / 31 ignored**.
  - Explicit Metal GPU: **26 passed**.
  - Python 3.11: **126 passed**.
  - Desktop release build passed.

## Virtual shadow maps for the sun (ADR 0027)

- `Environment.virtual_shadows`: a camera-centred clipmap of 8192² virtual
  levels. Pages are marked by a compute pass over the prepass, read back,
  allocated and LRU-cached in the right half of the 8192×4096 shadow atlas,
  and sampled with a coarser-level fallback. Only new pages are rendered.
- The GPU test shows a shadow beyond the 10 m cascade range, 0 pages
  re-rendered on the second capture, and a byte-identical image.
- Ashen castle: 55 pages requested, 0 dropped; 0.84% of pixels differ from
  the cascade render. A first-frame auto-exposure seeding bug (99.6%
  difference) was found and fixed.
- Behavior after this step:
  - Rust `--no-fail-fast`: **225 passed / 0 failed / 28 ignored**.
  - Explicit Metal GPU: **23 passed**.
  - Python 3.11: **126 passed**.
  - Desktop release build passed.

## GoW III cubemap probes and cluster LOD (ADR 0025, ADR 0026)

- Authored reflection probes (`source` `.hdr`, `parallax`).
  `tools/gow3_cubemaps.py` decodes the GoW III slot-22 cubemaps; their face
  convention is verified by seam continuity. 3 of 4 are converted and 1 is
  refused.
  - OLY100 with the native hall cube and with the engine's own capture
    differ in 0.7% of pixels.
- Cluster LOD DAG (own QEM simplifier, crack-free cuts, seam welding),
  drawn through every PBR pass with `Environment.geometry_lod`.
  - OLY100 at 1 px: 574,234 of 999,296 triangles drawn.
  - GPU-driven culling and virtual shadow maps are not implemented yet.
- Behavior after this step (shared tree; Codex was editing audio, text and
  input concurrently):
  - Rust `--no-fail-fast`: **217 passed / 1 failed / 27 ignored**. The
    failure is `crysirror-audio`
    `compressed_music_streams_decode_and_seek_without_whole_song_pcm`, which
    is in-progress Codex work.
  - Explicit Metal GPU: **22 passed**.
  - Python 3.11: **125 passed / 1 failed**. The failure is
    `test_engine_audio_and_text_are_independent_subsystems`: the desktop
    crate's new `rodio` dependency conflicts with the subsystem-boundary
    rule, also in-progress Codex work.

## Box reflection probes (ADR 0024)

- `reflection_probe` entity component / `Entity.setReflectionProbe`.
  - The engine captures six faces with the real surface pipelines, including
    local lights and shadows, then converts them to an equirect map and runs
    the environment GGX prefilter.
  - Up to 8 slots. Static probes are captured once; bumping `revision`
    recaptures.
  - Shading uses box projection, blend fade and environment fallback, with
    SSR applied on top.
- The GPU test puts coloured off-screen walls on the probe box faces and
  checks that each appears in the mirror floor on the correct side.
- OLY100 hall with one probe: 40% of pixels change by more than 8/255
  against the sky-only reflection. No original reference image exists to
  compare against.
- Behavior after this step:
  - Rust: **182 passed / 0 failed / 22 ignored**.
  - Explicit Metal GPU: **20 passed**.
  - Python 3.11: **123 passed**.
  - Desktop release build passed.

## Screen transitions and screen-space reflections (ADR 0022, ADR 0023)

- Screen-space reflections (`Environment.reflections`): prepass ray march
  with bisection and self-intersection rejection, previous-frame HDR
  radiance, roughness fade.
  - On OLY100 the import's flat roughness of 0.6 equals the default fade-out,
    so the default settings change nothing there. That is reported, not
    tuned away.
- Regression found and fixed in this step: a class member that was both a
  property and a method broke every JS binding. The binding generator now
  rejects such collisions visibly.
- Behavior after this step:
  - Rust: **181 passed / 0 failed / 21 ignored**.
  - Explicit Metal GPU: **19 passed**, including the new transition and SSR tests.
  - Python 3.11: **123 passed**.
  - Desktop release build passed.

### Atmosphere, exposure and video (ADR 0019, ADR 0020)

- Behavior: Rust **179 passed / 0 failed / 18 ignored**; explicit Metal GPU
  **16 passed**; Python 3.11 **123 passed**. Desktop release build passed.

## Clustered local lights (ADR 0018)

- Update 2: budgeted point/spot shadows from a 64-tile atlas; material
  `normal_encoding` (rgb/rg/ag) and `specular_factor`/`specular_texture`
  (ADR 0015 addendum). GoW III OLY100: complete mip chains from `OLYMPUS.TS`
  (byte offset at wrapper +60; the streamed sizes of all textures sum exactly to the
  TS length), **241** textures decode with 0 errors, and sampler slots 0/5/9 are
  mapped as base/DXT5nm normal/specular from texture content (other slots
  reported). Actual Metal renders: `~/.cache/crysirror/gow3/olympus-hd/preview`
  and the castle with shadows. Behavior: Rust **175 passed / 0 failed / 17
  ignored**; explicit Metal GPU **15 passed**; Python 3.11 **123 passed**.

- Update: range-limited `directional` and `ambient` volume lights plus
  `falloff: {kind:"exponent"}`. GoW III OLY100: `tools/gow3_lights.py` converts
  all **207** native light records (103 point / 87 directional / 17 ambient)
  with documented approximations; actual Metal render of the interior with them.
  Behavior after this update: Rust **174 passed / 0 failed / 15 ignored**;
  explicit Metal GPU **13 passed**; Python 3.11 **122 passed**.

- Implementation: `LightSpec` entity component (point/spot/rect, candela or
  lumens, range, enabled, stored shadow flag), `WorldCommand::SetLight`,
  `World.spawn({light})`, `Entity.light`/`Entity.setLight`, Python schema
  mirror. The renderer uses CPU clusters (16×9×24) with storage buffers, and
  the PBR shader shades lights with the sun's BRDF. glTF `KHR_lights_punctual`
  point/spot lights import; directional lights are reported as dropped.
- UE path: pure-Rust `ue-import level` (recovered schemas + container
  header + archetype merge) extracts placements and lights without CUE4Parse.
  Its FantasyCastle mesh positions agree with the CUE4Parse preview glTF for
  **4,683 / 4,815** instances (the rest include omitted sky/fog meshes).
  `tools/ue_lights.py` converts **284** castle and **200** village lights;
  sky lights are skipped and reported.
- Compilation: `cargo check --workspace --tests` and desktop release build passed.
  Behavior: Rust **174 passed / 0 failed / 14 ignored**; explicit Metal GPU
  **12 passed** (new: clustered point/spot test); Python 3.11 **120 passed**.
  Ignored tests are not counted as passes.
- Actual Metal captures of the castle hall with and without the converted
  lights are in `~/.cache/crysirror/ashen/castle-lit` (external, not
  distributed). No image-equivalence or frame-time claim.
- Not implemented: local-light shadows (light leaks through walls), rect-light
  area specular, sky-light capture, auto exposure, height/volumetric fog.

## God of War III OLY100 rest-space scene trial (ADR 0017)

- Actual Apple M5 / Metal capture from the local OLY100.WAD: **247 resource
  nodes / 1,474 surface groups / 548,191 triangles**, with 75 used texture
  previews and no selected MG geometry omitted. This is the selected WAD's first
  surface table in encoded rest space, **not the complete original scene/game**.
  Entity/bind/animation composition remains unresolved; applying entity matrices
  directly double-transforms some geometry. Native matrices remain diagnostic
  metadata. 1,380 surfaces use preview textures; 94 remain neutral, including
  seven unresolved UV-remap groups. Original shaders,
  lighting and animation poses are not reproduced.
- Reusable safe CPU APIs: `crysirror-asset::indices` (raw U16 and bounded EDGE
  profile) and DDS BC1/2/3 + explicit raw formats in `crysirror-image`.
  Actual conversion uses Rust codecs: all **2,118** topology streams agree with
  the independent Python implementation; **97** low-mip DDS previews decode.
  This is not a comparison against an original/proprietary native decoder.
- `SystemsSpec.environment` initializes validated, client-owned lighting from
  project data. The native preview renders from JSON/glTF/PNG without TS/JS.
  Python validation/inventory includes scoped HDR assets. Existing shared glTF
  PBR and Metal rendering are reused; no title-specific GPU path or console host.
- Compilation: workspace/all-targets, desktop no-default, desktop release and
  CPU transcode example passed. Behavior: Rust **170 passed / 0 failed / 13
  ignored**; explicit Metal GPU **11 passed**; Python 3.11 **117 passed**.
  Ignored GPU/font/game cases remain excluded from the workspace pass count.
  A separate CPU transcode capability-path test also passed. Actual CLI
  validation/render and MCP stdio validate/trace/capture succeeded.
- Capability audit proposes local point/spot lights, per-slot material overrides,
  extra vertex attributes/UV sets/baked lighting and dependency/mip streaming.
  Native ANM/transform and UV decoding are frontend work; engine animation
  already exists. These proposed APIs are not implemented by this trial.
- Evidence and exact limits: [ADR 0017](docs/adr/0017-gow3-scene-capabilities.md),
  docs/gow3-scene-results.json and docs/gow3-scene-mcp.json. Game assets/captures
  stay external. No full-scene fidelity, hardware RT, Windows/Linux or frame-time
  result is claimed.

## Earlier: shader presets and developer WGSL (ADR 0016)

- `Material.effect` uses safe shared CPU `ShaderEffect` data: Fog (moving UV
  noise/density/edges), Fresnel emission, Dissolve coverage/glowing edges, or a
  client-scoped custom WGSL path plus four vec4 parameters. Code stays in separate
  assets; snapshots contain only paths/parameters. No code/device pointers in data.
- Public surface ABI v1 changes linear color/opacity, emission and PBR factors.
  One implementation runs in color, prepass and shadow variants, including
  skinned meshes. The engine retains depth/blend/lighting ownership. Diagnostics
  cover bad WGSL/ABI/resources and driver pipeline validation; no silent fallback.
- CPU `shader-check` CLI and MCP `shader_validate` check all assembled stages
  without a GPU. Schema, inventory, Rust/TS APIs, saves and external import
  profiles support effect data/code assets. `examples/shader-effects` runs as
  a script-free project; dist/shader-effects.png is its actual Metal capture.
- The native castle's **42 fog sheets / 7,709 nodes / 13,426,500 scene triangles**
  render using the Fog preset and source color/opacity/depth parameters, replacing
  the prior self-authored opacity mask. Noise/flow/edges are engine approximations;
  full UE material/lighting/gameplay fidelity is not claimed. User assets stay
  external. The source sky sphere remains the only omitted geometry instance.
- Limits: surface hooks only, no custom bindings/vertex/post/compute replacement,
  hot reload or shader graph. Fog is a sheet effect, not global/volumetric fog.
  No new hardware RT, GoW geometry, Windows/Linux or GPU frame-time result.
- Contract and evidence: [ADR 0016](docs/adr/0016-shader-effects.md),
  docs/shader-effects-results.json and docs/shader-effects-mcp.json.

## Earlier: engine material system and transparent PBR (ADR 0015)

- Shared, device-free `MaterialSpec` in crysirror-scene: PBR factors/maps,
  opaque/mask/blend/additive/premultiplied, unlit, independent opacity,
  shared UV0 transform and soft depth intersection. Project entities, Rust
  commands, `Entity.setMaterial` in JS/TS, snapshots and MCP use this data.
  Canonical texture paths and GPU override caches remain client-scoped.
- Renderer: distinct transparent static/skinned pipelines, back-to-front sorting,
  opaque depth tests without writes, transparent HDR composition after opaque TAA
  and before bloom/tonemap. Mask cutoff now also applies in shadow rendering.
  Standard glTF BLEND and unlit import correctly; a versioned engine-owned extras
  envelope carries additional shared material data. No title-specific renderer code.
- Behavior: script-free `examples/materials` rendered all five alpha modes;
  actual MCP stdio validated/traced/captured it. Native castle capture retained
  **all 42 fog sheets: 7,709 nodes / 13,426,500 submitted triangles**. Only the
  original sky sphere is omitted. Fog uses recorded source color/opacity/depth
  parameters and a self-authored mask; UE noise/projection/panning remains an
  explicit approximation. User content/profile/capture stays outside the repo.
- Limits: center sorting, single-sample transparency outside TAA history; no OIT,
  refraction, complete UE shader graphs or jointly sorted particles. No new GoW
  mesh decoder, hardware RT, Windows/Linux or frame-time result.
- Contract/limits: [ADR 0015](docs/adr/0015-engine-materials-transparency.md).
  Evidence: docs/engine-materials-results.json and docs/engine-materials-mcp.json;
  native demo: dist/engine-materials.png.

The sections below retain earlier checkpoints; their missing-feature statements
and test counts describe those checkpoints, not the latest implementation.

## Earlier local game asset trial (2026-10-25, before transparent materials)

- Ashen Vow's actual FantasyCastle static geometry was exported from the local
  UE 5.7.4 shipping build and rendered by CrySirror on Apple M5/Metal. Native
  extraction: 132 distinct meshes / 7,710 instances; preview: **7,667 nodes,
  13,426,416 submitted conventional-LOD triangles, 31 base-color textures**.
  One sky sphere and 42 unsupported fog-sheet instances are explicitly omitted.
  Materials are approximate, original lights/gameplay/landscape/characters are
  absent; this is partial scene rendering, not full-game compatibility.
- Existing static reflection recovery now exports interoperable `.usmap` data
  (7,312 schemas / 1,276 enums); 100 sampled native mesh components parsed with
  expected tails. Developer-only CUE4Parse bridge, preview converter, bounded
  PSARC reader and partial WAD inspector are retained as tools.
- God of War III (Japan): 16 PSARC directories / 6,041 names read; hero WAD
  decompressed and all 885 records inspected. The selected Olympus level fails
  explicitly on unframed `EntityCount` at `0xd60`; no mesh decoder or scene
  rendering is implemented. No console backend was added.
- Compilation: workspace/all-targets check and external bridge build passed.
  Behavior: workspace **158 passed / 0 failed / 8 ignored**, Python **106 passed**,
  actual Metal screenshot produced. GPU smoke suite was not rerun; ignored tests
  remain excluded. No GPU frame-time or original-image-equivalence result.
- Extracted content and screenshot remain outside the repository. Commands,
  limitations and metadata evidence: [LOCAL_GAME_IMPORT.md](docs/LOCAL_GAME_IMPORT.md)
  and docs/local-game-import-results.json.

## Earlier: gameplay systems (ADR 0014)

- GAS-style gameplay (attributes, effects, tags, abilities, cues), animation
  state machines/blend spaces/montages with notifies, reactive behaviour-tree
  AI with a nav grid baked from collision (A* plus string pulling), CPU
  particles in the HDR pass with warm-up, procedural/heightmap terrain with
  heightfield collision, level streaming with hysteresis, spatial audio
  (core `SetAudioGain`), and the IoStore/Zen reader for the Ashen Vow
  assessment (ADR 0013).
- examples/aaa-systems (valley hunt): AI hunters path 58 m → 1.4 m and
  bite on cooldown; player HP 100→10 with bonfire healing;
  dist/aaa-systems.png. Found and fixed a parry heightfield vertical-ray
  miss (regression-tested).

## Earlier: animation, physics, input, playable slice (ADR 0012)

- glTF skeletal animation (skins, T/R/S channels incl. cubic spline,
  4-layer blending, cross-fades as saved world state) with GPU skinning in
  the opaque, prepass and shadow passes, plus previous-frame palettes for TAA.
- Rapier 0.22 physics derived from World colliders: trimesh/box/sphere/
  capsule, a kinematic character controller (slide, autostep, slopes, ground
  snap) and raycasts. Input action maps over 19 named keys. `run --script`
  makes projects playable in a window.
- examples/aaa-fox-sponza: third-person Fox in Sponza (WASD/Shift/Space,
  orbit camera with raycast collision, Survey/Walk/Run blends) →
  dist/fox-sponza.png and dist/fox-sponza-gaits.png. Release CPU simulation
  about 0.3 ms/frame; GPU frame time not measured yet.
- Evidence: `cargo test --workspace` **144 passed / 0 failed / 7 ignored**;
  explicit GPU 5 passed; Python 97 passed; workspace and no-default checks
  pass.

## Earlier additions (2026-10-24, late night)

- Renderer: 4 stable cascaded sun shadows, a depth/normal/motion prepass,
  SSAO with a depth-aware blur, and TAA (Halton jitter, reprojection,
  YCoCg clamp). The DamagedHelmet and Sponza demos were re-rendered on
  Apple M5/Metal (dist/aaa-*.png; release wall time about 1 s each, which
  includes asset loading and 16 TAA accumulation frames, not frame time).
- API coverage: Layers/Ui/Audio/Save classes; World.camera/environment and
  Entity.angularVelocity/color/geometry. `COMMAND_API` and
  tests/api_coverage.rs require an API method for every World/Layer/UI
  command variant, and that each method appears in the generated .d.ts. A
  JS test drives layers, UI, camera, environment and save/load rollback.
- ADR 0011: game-port research. OpenLara (BSD-2) is the Tomb Raider target;
  Zelda projects lack permissive licenses and are reference-only.
- Evidence: `cargo test --workspace` 138 passed / 0 failed / 7 ignored;
  explicit GPU tests 5 passed; Python 97 passed; workspace and
  no-default-features checks pass.

## AAA rendering implementation and validation

- **AAA rendering (ADR 0010), implemented:** glTF metallic-roughness
  materials with textures, a 4×MSAA HDR pipeline, GGX/Smith/Schlick shading,
  normal/AO/emissive maps, alpha mask, double-sided materials, IBL
  (GGX-prefiltered HDR, SH9 irradiance, split-sum LUT), HDR sky, a 4096²
  PCF sun shadow map, dual-filter bloom and PBR Neutral/ACES tonemapping.
  Script API: `world.setEnvironment`, `world.setCamera`,
  `importModel(..., {pbr:true})`.
- **Showcase ports executed on Apple M5/Metal:** examples/aaa-helmet (port of
  three.js webgl_loader_gltf: DamagedHelmet + Royal Esplanade HDR) →
  dist/aaa-helmet.png; examples/aaa-sponza (Sponza, 103 primitives, about
  70 textures, sun shadows) → dist/aaa-sponza.png. Assets are fetched with
  tools/fetch_demo_assets.py (pinned SHA-256, licenses recorded) and not
  committed.
- Core gained `ClientStorage`, `present_surface`, `service::buttons`
  and the `VfsProvider` mounts.

## Earlier iteration: engine object model, scripting, VFS, Blender assets

Decisions and backlog: docs/adr/README.md (ADR 0004–0007 new).

- Implemented: crysirror-engine (reflective Class/Object API, App lifecycle
  Created/Initialized/Running/Paused/Stopped/Faulted, fixed/update/late hooks,
  plugins, ScriptHost contract; built-in Engine/World/Entity/Fs/Client
  classes over shared runtime services); crysirror-script-js (Boa runtime +
  Oxc TS lowering, generated JS bindings, catchable engine errors, loop/
  recursion limits); `crysirror script-types` emits registry-generated .d.ts;
  core `VfsProvider` + per-client provider mounts in DiskVfs/Runtime;
  crysirror-asset glTF/GLB importer and `world.importModel`;
  tools/blender_export_gltf.py; core `Upscaler`/`UpscalerSupport` contract.
- Compilation: `cargo check --workspace --all-targets` passed
  (docs/engine-api-cargo-check.txt); `cargo check -p crysirror-desktop
  --no-default-features` passed (docs/engine-api-no-default-check.txt);
  clippy reports no warnings in the three new crates.
- Behavior: `cargo test --workspace` **131 passed, 0 failed, 5 ignored**
  (docs/engine-api-rust-tests.txt; ignored = 4 GPU + 1 caller-font test,
  not counted as passes). Explicit GPU run `cargo test -p crysirror-wgpu --
  --ignored`: **4 passed** (docs/engine-api-gpu-tests.txt). Python tools:
  **97 passed** (docs/engine-api-python-tests.txt).
- Executed end to end on Apple M5/Metal: Blender 5.2.2 headless exported
  examples/engine/assets/lantern.glb; `crysirror render
  examples/engine/project.json --script examples/engine/game.ts` ran a TS
  behaviour for 30 frames, imported the model three times (13 entities,
  nothing dropped), and produced dist/engine-api-raster.png and
  dist/engine-api-compute-raytrace.png (docs/engine-api-render.json).
  hardware_raytracing=false.

## Previous snapshot (shared CPU source migration)

## Implemented source

- Thirteen workspace crates; crysirror-desktop remains the default executable.
  Core remains no_std/dependency-free; core/compat stay safe and device-free.
- World owns scoped generational entity handles, validated transactional
  mutations, cascading removal, parent transforms/visibility, velocity/angular
  velocity, fixed 16 ms steps, fractional remainder and game/real-time tweens.
  World extraction bakes dynamic hierarchy transforms into renderable triangles.
- Layer owns retained hierarchy, mutation batches, pivot/anchor/rotation/scale,
  inherited opacity/visibility, clips, explicit order pairs, UVs and original
  camera/billboard/culling projection. CPU data includes homogeneous W; GPU
  interpolation is perspective-correct. Rotated clips use four-corner AABBs.
- UI owns Panel/Label/Button/Toggle/Slider trees, layout/padding/gap, clipping,
  focus/navigation, pointer capture/release, modal input routing, value/event
  state, typed World callbacks and transactional mutations. UI consumes owned
  input before VMs, preventing Enter from also resuming a background wait.
- ClientSpec.systems installs those systems independently of VM progression;
  World continues updating after data VMs halt. Desktop/capture extract the
  same scene and overlays for raster and compute rendering.
- Renderer has executed normal/add/subtract/multiply/screen pipelines and
  scissors; it remains independent of winit. Desktop owns windows/event loop.
- Runtime snapshots include engine state and fixed-step/animation clocks.
  Systems projects use services schema 2; legacy flat projects retain schema 1.
  Restore validates scopes/hierarchies/render extraction before live swaps;
  transient UI capture is cleared and held input must first release.
- MCP project validation/replacement and asset inventory include systems.
  With a configured host, engine_trace returns actual fresh-session world/UI
  state. scene_capture accepts raster/compute plus bounded frames/step_ms and
  Enter pulses. There is no in-process live game control API.
- Existing crysirror-text owns fonts/simple layout/positioned glyph rasterization;
  crysirror-audio owns voice lifecycle/buses/AudioDevice with optional rodio.
- Optional crysirror-author retains Rust Oxc TS/JS compilation plus Boa offline
  buildProject execution. Runtime JSON needs no JS engine. No per-frame JS was
  added; original native VMs remain separate from the built-in data VM.

## Compilation evidence

Local host: macOS arm64, Apple M5 Metal; Rust/cargo 1.98.1 and Python 3.11.
Desktop targets remain Windows/Linux/macOS. A minimum Rust 1.88 is declared from
resolved requirements; it was not independently tested here.

- `cargo check --workspace --all-targets`: passed;
  docs/engine-systems-cargo-check.txt.
- `cargo check -p crysirror-desktop --no-default-features`: passed;
  docs/engine-systems-no-default-check.txt.
- `cargo test --workspace`: **103 passed, 4 ignored**, no failures;
  docs/engine-systems-rust-tests.txt. Ignored tests are three GPU tests and one
  caller-supplied font test; those are not counted as passes in this run.
- Python tools: **91 passed**;
  docs/engine-systems-python-tests.txt.
- Cargo reports the existing block 0.1.6 future-incompatibility warning.
  No successful Windows/Linux build or hardware-RT execution is claimed.

## Behavior evidence

- CPU tests exercise stale/scoped handles, hierarchy/cycle rollback, negative
  scale winding, fixed-step chunking and save continuity, pause versus real
  time, UI capture/release/focus/modal/slider, engine mutation rollback, input
  consumption, post-halt simulation and transactional engine restore.
- Explicit GPU run: **3 passed, 0 ignored**;
  docs/engine-systems-gpu-tests.txt. Tests check actual legacy/native-image
  pixels, dynamic World extraction on raster AND compute paths, UI preservation,
  scissor exclusion, and all five blend equations in linear color.
- Explicit local-font CPU run: **1 passed, 0 ignored**;
  docs/engine-systems-font-test.txt. Arial was supplied from the local OS for
  this run only. No font binary was copied into the repository or packages.
- Real GPU captures: dist/engine-systems-raster.png and
  dist/engine-systems-raytrace.png. The latter includes actual Text/UI services
  and dynamic World at frame 64; docs/engine-systems-render.json records the
  real adapter/runtime. Compute traces primary/shadow rays; hardware RT is false.
- Actual MCP stdio engine_trace executed 64 frames and its Enter pulse activated
  the pause-world UI button. MCP scene_capture returned real compute PNG bytes;
  asset inventory included the layer's client-scoped PNG. Evidence:
  docs/engine-systems-mcp.json.
- Previous executed G00 original-decoder differential and hardware-RT rejection
  evidence remain in the resource/renderer records referenced by ADR 0003.
  This iteration adds no original VM hooks, game runs or original-save claims.

## Remaining work and limits

Only crysirror-script-v1 is registered as a VM. Unknown native adapters and text
profiles fail visibly. A successful common subsystem test is not evidence of
native game compatibility. Snapshot identity hashes project data; bump
content_revision for changed external assets. Device pointers/code do not enter
project/VM data, GPU state is rebuilt and audio restores at sample zero.

Complete reference checkouts remain outside the deliverable at
/Users/xmoe/.cache/crysirror/upstream-20261024, with actual pins in upstream-lock.json.
Only attributed selected source was migrated; no game data, font, credential,
SDK or proprietary binary was imported. Workspace has no .git; no remote or PR
was created. Older bundle/hash/missing-toolchain records are historical and do
not validate this source. tools/verify.py/release recovery were not rerun.

## Codex parallel service work: audio, text, input and localization (2026-10-25)

This appended checkpoint owns ADR 0030–0033. Renderer, scene view/components,
asset mesh clustering, GoW tools and probe work remain with the parallel author.
Earlier historical descriptions above, including the source-export Git state,
retain their original checkpoint meaning.

- Implementation: shared CPU SFX pools with scoped generation handles and
  priority stealing; seeded weighted variants, stereo spatial attenuation/pan,
  occlusion lowpass, client buses/ducking, sample-level fades/crossfades/loops,
  WAV/MP3/Vorbis/FLAC streaming and rate/pitch. The Audio class and App feed one
  mixer; optional device output stays inside crysirror-audio, not desktop rodio.
- Text: rustybuzz shaping, bidi, UAX #14 + documented CJK tailoring, fallback,
  ruby/vertical layout, rich styles/icons/outline/shadow, grapheme reveal/events
  and CPU SDF atlas data. The engine Text class publishes scoped RGBA surfaces;
  localization catalogs choose CLDR plurals and font aliases. Input adds runtime
  rebinding/profiles, analog/dead zones, optional GilRs and prompt tokens.
- Compilation: workspace/all-targets, desktop release and no-default checks
  passed; audio/text strict clippy passed. Existing no-default unused_mut and
  dependency future-incompatibility warnings remain.
- Limits: CPU SDF data only, no SDF GPU/3D pipeline. Advanced audio/session
  configuration is not automatically in Runtime snapshots; explicit text,
  localization and binding checkpoints are documented. Audio decode/mixing
  remains synchronous on App, with resampling pitch affecting duration; no
  HRTF, hard-real-time, physical soundcard/gamepad or new GPU validation claim.
- At owner request Git was initialized and origin set to
  `git@github.com:xmoezzz/CrySirror.git`; SSH read succeeded, with no remote
  branches at configuration. No commit or push was performed on parallel work.
  Fonts are excluded by Git patterns; no font files were added.
- Evidence: docs/codex-services-results.json, codex-services-*.txt, and
  ADR 0030–0033. Actual CPU typography/SDF previews remain under
  ~/.cache/crysirror/codex-text, outside the source export.

## 2026-10-25 — RealLive / AVG32 implementation and validation

- RealLive/AVG32: both independent VMs already exist in the original VM project. External AVG32 tests passed
  33 library + 1 NLS + 4 native synthetic Engine cases; RealLive passed
  58 library tests. Missing-installed-data early returns were excluded. Neither
  family is vendored/registered in CrySirror; no installed game was run for them.

## Codex shared capability extraction: Bitmap API (2026-10-25)

- Implementation: crysirror-image now owns checked CPU bitmap fill, clipped
  copy/blit, opacity/mask math, RGB color-key and nearest stretch. RealLive
  /255 and AVG32 premultiplied /256 arithmetic are explicit profiles alongside
  standard source-over. Native VM/presentation/buffer policies stay separate.
- API: new Bitmap class/global supplies create/load/fill/blit/stretch/info/remove
  to Rust and JS/TS through the existing registry. Results use the existing
  client-scoped surface store, Layer transform/filter/composition and budgets.
  Asset/provider reads and generated native images are supported. Operations
  stage pixels before publication, including source snapshots for self-blits.
- Behavior: comparison against unchanged pinned upstream matched all
  **19,200** RealLive Copy/Mask cases, **9,216** AVG32 premultiplied cases and
  **12** nearest stretch cases. Rust and actual Oxc/Boa tests verify provider
  loading, generated images, alpha policies, self-copy, clipping, failed-write
  rollback, client isolation and existing shared Layer composition parameters.
  tools/compare_bitmap_upstream.py and tools/bitmap_reference.rs reproduce the
  original-source comparison, with pin/dirty-source/output-path validation.
- Limits: no new GPU execution, complete native VM port, bilinear/external mask,
  AVG32 fade or automatic bitmap snapshot/save compatibility claim. No original
  game assets/fonts or protected renderer/scene/asset/GoW code were changed.

## 2026-10-25 Codex: shared resource formats and Rapier physics

- Implementation — resources: added the safe CPU `crysirror-resource` contracts,
  registry, random-access archive streams, bounded decoding and scoped key/transform
  pipelines. The optional GPL `crysirror-formats-arc` crate registers Majiro ARC, SAR/NSA and XP3. Its complete pinned inventory records 289 decoder
  IDs: 3 ported, 2 partial and 284 pending. Existing native VM integration remains
  the ADR 0039 adapter; a resource plugin does not register another VM.
- Implementation — engine integration: `Resources` exposes metadata, profiles
  and archive mounts through the shared object API. Bitmap/Fs/Assets consume
  those mounts. Client storage handles observe later mounts; callbacks release
  the mount lock first. Mounts invalidate client asset/physics caches and retain
  generated resource identities. Generic encryption interfaces do not imply
  universal game-key or crypt-filter support; keys are not saved in project JSON.
- Implementation — physics: `crysirror-physics` directly adapts Rapier 3D 0.22.0;
  Rapier performs integration, collisions, CCD, joints and sleeping. Runtime owns
  the persistent service, called once inside each 16 ms World step, including
  pure JSON projects. Scene declarations and Physics/PhysicsJoint/Entity APIs
  cover rigid bodies, materials/groups/sensors, forces/impulses, queries/events,
  joints and kinematic targets. Client/generation checks and incremental caches
  retain ownership and stable handles. Logical snapshots preserve velocities,
  forces, targets and joints, including explicit impulses while paused.
- Compilation: `cargo check --workspace --all-targets --features
  crysirror-desktop/arc-formats` passed on local macOS aarch64. API docs were
  regenerated and the signature fingerprint was logged in docs/api/CHANGES.md.
- Behavior: final workspace tests with desktop/arc-formats passed 368 tests,
  0 failed and 35 ignored. The ignored GPU/font/external-corpus cases were not
  counted as passes. Python tooling passed 157 tests. Focused physics evidence
  separately records 25 backend tests, 13 Runtime/object/legacy regressions and
  one real Oxc/Boa TypeScript test; these are subsets of the workspace result.
  Resource tests cover limits/truncation/key isolation, cross-block AES seeks,
  synthetic large streams, actual archive-to-Bitmap pixels, provider visibility
  and client cache invalidation. A separately invoked external-source test passed
  against 14 upstream synthetic fixtures and original declared expected bytes;
  the C++ executable was not built or run and no fixture binaries were copied.
- Measured performance: release Rapier-only primitive workloads on this Apple
  M5 host averaged 0.682 ms per active falling step for 512 cubes and 2.380 ms
  for 2048; unchanged sync recreated zero bodies/colliders. This does not measure
  whole-engine frames or a commercial level. Raw and summarized evidence is in
  docs/physics-benchmark-results.json and target/verification/.
- Limits: 284 arc_unpacker decoders remain pending, with variant boundaries
  recorded for the five registered IDs. No new commercial-game, GPU or other-OS
  behavior is claimed. Compressed entries are currently materialized under
  decode limits; retained-byte accounting is not a bound on all process memory.
  Physics tooling, cloth/soft bodies/fluids and exact solver-cache snapshots
  remain future work. Original native opcode/wait/save policies remain adapters.
- Evidence: docs/RESOURCES.md, docs/PHYSICS.md, ADR 0044/0045,
  docs/physics-integration-validation.json and docs/resource-physics-validation.json.

## 2026-10-25 Codex: all pinned image decoder IDs on the shared resource path

- Implementation: all 96 image decoder IDs in the complete pinned arc_unpacker
  inventory now have concrete registered Rust implementations/adapters, with
  80 ported and 16 partial entries. These include TLG5/6, QNT/PMS, G00/PDT, PB3,
  ERI, CBG, RCT/RC8, PGD, NVSG, GIM/GTF/XTX, wrappers and palette/mask codecs.
  Standard entropy codecs reuse shared libraries; no original VM, renderer or
  unpacker executable is embedded. The five archive IDs remain implemented;
  188 other archive/audio/file decoders remain pending in the complete ledger.
- Implementation: the safe CPU ImageFormat/ImageDecodeContext registry handles
  specificity/path hints, explicit ambiguity, scoped bases/palettes/masks,
  bounded recursive decoding and caller key/transform services. Bitmap, text
  icons, retained layers and renderer uploads share Runtime Assets decoding.
  glTF/PBR accepts that decoder for native textures and engine mip generation.
  Resources adds imageFormats/probeImage/decodeImage; API docs were regenerated
  and the fingerprint logged. Native VM cuts/pivots/messages/saves stay adapters.
- Compilation: final cargo check --workspace --all-targets with
  crysirror-desktop/arc-formats passed on macOS aarch64. Optional format plugins
  remain explicitly enabled by arc-formats.
- Behavior: final workspace tests passed 464 tests, 0 failed, 42 ignored.
  Python 3.11 tooling passed 162 tests. Ignored cases are not passes. The image
  groups separately executed seven external tests with 177 output comparisons:
  174 exact dimensions/RGBA, three documented AJP/JPEG/WebP lossy rounding
  differences. Eighty-eight ordinary image tests use generated resources;
  original external fixture binaries are not distributed.
- Behavior: three end-to-end engine tests verify SAR→QNT/TEYL→Bitmap/Resources,
  native glTF textures and exact sRGB/linear mips, caller key/client isolation,
  same-client reference/mask reads, strict denied/corrupt errors and failed
  decode preserving the prior surface revision. Common registry tests cover
  priority/ambiguity, recursion/path escapes and invalid outputs. Dispatch
  tests cover shared raw-mask layouts and strong PNG signatures over weak hints.
- Actual GPU behavior: an Apple M5 Metal raster capture loads self-authored
  TEYL, PGX and NVSG data from SAR through Resources.mount and ordinary retained
  layers. All 49,152 native pixels match the PNG reference region, and 16
  interior samples equal their source RGBA. Reproduce using
  tools/make_native_image_smoke.py and the command in docs/RESOURCES.md.
  images/native-image-registry.png contains only these generated test colors.
- Limits: named variants, key/filename/dependency constraints and fidelity
  boundaries remain in docs/arc-image-support.json; 96 registered decoders do
  not imply every game/format variant works. This does not register RealLive or
  AVG32 VMs or claim a commercial level, other GPU/OS or hardware-RT result.
- Evidence: ADR 0046, docs/arc-image-validation.json, per-group source/hash and
  behavior records, docs/RESOURCES.md and target/verification/arc-image-*.log.

## 2026-10-26 — Original RealLive through shared CPU services

- Implementation: added dormant `crysirror-reallive` and the attributed
  original `reallive-original` VM from the original VM project. Desktop registers `reallive`
  by default. Original Seen instructions, values, waits, resource metadata,
  text layout and savepoints remain independent of the data VM.
- Shared implementation: CPU Surface kernels, PDT10/11, glyph coverage and
  painting, channel audio, incremental MPEG video/audio, logical input and
  scoped resource/save services. No native window/GPU/audio device is created.
  Native loops can update the shared mixer without replaying the sound.
  Unknown operations, unsupported host requests and worker faults fail visibly.
- Native savepoints: Runtime now supports explicitly transactional native
  imports without requiring a whole-VM snapshot. RealLive stages all fallible
  work once, commits original restore fields, and preserves non-slot global,
  settings, clock/timer and sound state. Invalid late sections leave the live
  VM and queued audio unchanged. RLRSSAV1 is the upstream Rust savepoint format;
  it does not establish Windows binary-save or exact-frame snapshot support.
- Compilation: workspace check, minimal desktop check, and optimized desktop
  build completed successfully. The generated API reference and change log
  agree on fingerprint `22687066e1f5f9c5`.
- Automated behavior: `cargo test --workspace` exited 0, **551 passed / 0 failed /
  50 ignored** across 112 reported suites. Python tooling: **162 passed**.
  Separately executed adapter tests passed all 8 cases, including four normally
  ignored system-font cases; the explicit glyph differential matched 36 cells.
  Shared Surface differential covers 1,920 cases. A read-only local MPEG prefix
  matched 12 video RGBA/PTS frames and two PCM/PTS chunks. Ignored tests remain
  distinct from these explicit executions; original DLL tests were not run.
- Actual game/GPU behavior: the owner-supplied Game_RUSUR ran from disclaimer
  through title/name settings, the opening movie, and the first story pause at
  scene 6030 line 65. Final optimized run: 10,000 ticks, zero unknown operations
  and errors, 51 shared glyph rasters, and 7,679,232 offline stereo audio frames
  with no rejected voices. Both 800×600 title and first-story captures match the
  pinned original VM's reference pixels exactly. A real winit/Metal window ran
  180 ticks with a diagnostic scene-10 start override and muted audio; its title
  capture also matches exactly. The original game's 975 files were rehashed
  after the final runs: no content/size/mtime changes or added/missing files.
- Limits: bounded opening profile only; no complete-game, Windows/Linux,
  hardware-RT or device-audio claim. Committed text/IME and whole-runtime
  snapshots remain unsupported. Native movie scheduling and some effect
  policies remain in compatibility code; movie audio still accumulates bounded
  PCM rather than streaming a long track. Private game/font/captures stay ignored.
- Evidence: ADR 0047, docs/REALLIVE.md, docs/reallive-validation.json,
  docs/reallive-source-audit.json and the vendored UPSTREAM/source manifest.

## 2026-10-26 — Hosted engines removed at owner request

- The owner explicitly prohibits engine hosting: integration work exists to
  expand CrySirror APIs and engine-owned state so different original VMs can
  use those capabilities through thin semantic/syscall adapters. A running
  original engine and matching composed screen do not satisfy that goal.
- Deleted the earlier RealLive hosted adapter package, dedicated embedded
  engine source trees, Cargo members/dependencies/features, desktop factories,
  configuration schemas and hosted integration tests. Old project names fail
  visibly. Previous hosted-integration completion claims are withdrawn.
- Recorded the requirement in AGENTS.md, ARCHITECTURE.md, NATIVE_ADAPTERS.md and
  ADR 0047. Removed hosted setup instructions and regenerated API documentation:
  current fingerprint d233217062201fa1.
- Compilation: cargo check --workspace passed. Behavior: cargo test --workspace
  passed 474 tests, zero failures, 42 ignored; Python tooling passed 157 tests.
  Ignored tests were not counted as passes. These results validate the remaining
  engine and removal, not native game compatibility.

## 2026-10-26 — RealLive VM calls engine-owned APIs

- Implementation: added `crysirror-reallive`, retaining the original Seen
  parser/interpreter, values/banks, call stack and pure builtins from the original VM project. Engine operations
  remain pending until CrySirror service calls succeed. No original Engine,
  System, Graphics, Sound, renderer or framebuffer is linked. The desktop
  default enables the new factory; the bare Runtime remains host controlled.
- Shared capabilities: retained Canvas drawing/copy/presentation, positioned
  GlyphRun and engine-clock reveal, ordered-dither GPU transitions, Unicode UI
  TextField editing/query, common PDT/NWA/KOEPAC decoding and engine Media
  resources. Ordinary engine/data/JS clients use the same APIs. Unsupported
  copying/layout/audio/native variants fail visibly. API documentation was
  regenerated; fingerprint `8d83111697a3c377` is logged.
- Compilation: workspace check and minimal desktop check with only the
  `reallive` feature passed; optimized desktop/native GPU test compiled.
- Automated behavior: the clean workspace test run exited 0: **561 passed,
  zero failed, 47 ignored**. Python tooling: **165 passed**. Ignored cases are
  not counted as successful executions. A first workspace attempt encountered
  a rustdoc StableCrateId artifact collision; refreshing affected dev artifacts
  and rerunning the workspace completed successfully.
- Explicit behavior: shared media tests, including the complete local movie,
  passed **6/6** without a VM/GPU/device. The separate opt-in native GPU test
  passed on Apple M5/Metal: title, name input, choice, complete opening movie
  (2,995 decoded frames), then scene 6030 line 65, with 22 retained glyphs and
  five engine layers. Removing foreground/shadow text with the VM PC and clock
  unchanged changed GPU pixels. EOF retired the media overlay. This establishes
  engine-owned state and rendering, beyond merely starting an original VM.
- Source integrity: all 975 original game files were rehashed; no content,
  size, mtime, added-file or missing-file changes. Game assets/fonts/captures
  remain ignored. Only a self-authored synthetic media fixture is committed.
- Limits: bounded opening profile, not a full playthrough or pixel-perfect
  native layout claim. Original save files, VM snapshots, DLLs, unsupported
  window skins/advanced layout, preserving DC growth, transparent non-mask
  copies and queued outgoing music replacement remain unsupported. Media audio
  uses bounded full-track PCM; media resources reject snapshots until a real
  reconstruction contract exists. No device-audio, Windows/Linux or hardware
  ray-tracing result is claimed.
- Evidence: ADR 0048, docs/REALLIVE.md and docs/reallive-source-audit.json;
  ignored logs in target/verification/reallive-api-{acceptance.log,
  workspace-tests-final.log,python-final.log}. Private captures and before/after
  engine inspection are in images/third-party/reallive-api/.

## 2026-10-26 — Engine CPU workers and asynchronous assets

- Implementation: added device-free `crysirror-tasks` with bounded worker
  queues, wakeable Rust Futures, cooperative cancellation, panic reporting,
  worker counters and same-pool blocking-wait rejection. Runtime Assets shares
  an asset lane and a CPU lane across clients; no thread is spawned per asset.
- Actual integration: scoped image decoding and glTF dependency/model imports
  execute on asset workers and publish on the owner thread. Desktop data-VM
  projects defer pending draws while continuing frames. Offline capture and
  native compatibility projects retain explicit first-use waits. Independent
  PBR scene poses execute in bounded CPU batches and are collected in instance
  order before GPU writes. Loaded-resource revisions invalidate stale tickets
  and GPU keys; late cancelled results cannot overwrite refreshed resources.
- API: Rust awaitable reads and `Assets.load/status/cancel/release/stats` with
  the `assets` global. Requests are client/store scoped, deduplicated and
  bounded; renderer overload defers work rather than growing unbounded queues.
  API docs regenerated, fingerprint `df17e49431e56860` logged. ADR 0049 records
  thread ownership, limits and remaining synchronous work.
- Compilation: workspace check and the final desktop check passed.
- Automated behavior: `cargo test --workspace` exited 0, **571 passed / zero
  failed / 49 ignored**. Python tooling passed **165 tests**. Its source-data
  boundary scan now prunes private/generated directories before walking them,
  preventing races with rustc temporary directories during parallel validation.
- Explicit GPU behavior: **2/2** async tests passed on Apple M5/Metal. A blocked
  image provider did not block frame capture or logical ticks; completed pixels
  matched offline resolution. A glTF scene used CPU pose workers and matched
  blocking-mode GPU pixels. A separately executed reflection-probe regression
  passed **1/1**. Pending model imports do not bake empty reflection probes;
  resource-generation changes invalidate capture/GI identities. These explicit
  GPU executions are separate from ignored workspace tests.
- Other behavior: two barrier-synchronized jobs ran on distinct workers;
  Future wakeup, bounded admission, cancellation, failure/panic recovery and
  nested-wait rejection passed. Tests cover slow I/O with advancing Runtime,
  stale-generation rejection, client isolation, shared decode/import caches,
  overload deferral and 37 animated pose results matching serial evaluation.
- Limits: World/VM/script/fixed-step physics remain ordered on their owner
  thread. This is not a general frame task graph or complete simulation/render
  overlap. GPU uploads/compilation, cluster-LOD construction, collision cooking,
  HDR/probe loading and some shader/material/puppet/native paths retain
  synchronous work. Cancellation cannot forcibly interrupt a provider syscall;
  prepared-cache accounting does not bound all transient or GPU memory. No
  whole-engine speedup, other-OS hardware result or hardware-RT result is claimed.
- Evidence: target/verification/async-{workspace-check.log,workspace-tests.log,
  final-desktop-check.log,python-tests.log,gpu-tests.log,probe-regression.log};
  the new focused tests and ADR 0049 are committed without external game data.

## 2026-10-26 — Engine-wide async requirement

- Added accepted ADR 0050: asynchronous/multithreaded execution is required
  across every engine subsystem. It defines task/dependency, completion,
  ownership, backpressure and cancellation boundaries plus subsystem migration
  and acceptance criteria. ADR 0049 is explicitly stage one; the present
  asset/pose work does not complete the engine-wide requirement.

## 2026-10-26 — Native hardware ray queries and shared PBR sun visibility

- Implementation: migrated the default host to pinned wgpu 29.0.3 (resolved
  core/HAL/types 29.0.4), retaining renderer/event-loop separation. Native
  Vulkan/DX12/Metal adapters are selected by actual ray-query feature bits,
  surface compatibility and AS limits, without vendor-name allowlists. Raster
  devices do not opt into experimental features. Unsupported explicit hardware
  requests fail with adapter/backend reasons. ADR 0051 records this contract.
- Implementation: renderer-owned BLAS/TLAS, opaque primary rays and hard sun
  queries; position-based AS cache, metadata-only uploads, move/removal handling
  and recovery from abandoned encoders. PBR uses shared world/model/terrain/skin
  geometry prepared on bounded workers, its existing depth/normal prepass and a
  combined AO/visibility target. Other PBR shading remains in CrySirror's existing
  pipeline. Engine overlays still compose after the primary-ray output.
- Implementation: CLI reports adapter availability separately from device
  enablement and exposes encoded build/query counters. Counters are not GPU
  completion or performance evidence. Portable API support does not certify
  dedicated RT silicon; that report field remains unverified/null.
- Compilation/CPU validation: renderer unit suite passed 26 tests, with three
  GPU tests ignored in that invocation. The synthetic capability matrix covers
  native backends and vendor identifiers, missing features/limits, software
  rejection and device opt-in. Geometry tests verify shared transformed/skinned
  vertices. Shader validation uses Naga. API documentation regeneration/check
  passed five tests without a signature/fingerprint change. Python tooling
  passed 165 tests. `cargo check --locked --workspace --all-targets` passed.
  The generated API fingerprint remains `df17e49431e56860`.
- GPU backend behavior: explicit unit invocation on Apple M5 / Metal passed
  all six tests (including two actual GPU pixel tests): primary hits/occlusion,
  metadata recoloring without rebuild, moved/removed geometry, abandoned-build
  recovery, and AO-preserving directional shadow/empty-scene output. Evidence:
  `target/verification/hardware-rt-backend-gpu-tests.log`. The pinned Naga Metal
  ready-flag loop issue was reproduced and avoided with single-proceed queries
  for force-opaque triangles; this is not a candidate-processing implementation.
- GPU integration behavior: on Apple M5 / Metal, the 120-triangle primary scene
  matched the existing compute reference within the asserted pixel tolerance,
  composited the engine's green overlay and reused its AS on a repeated frame.
  The 24-triangle PBR floor/cube scene produced a measured shadow-on/off pixel
  difference through native sun queries and rejected a masked material.
  Evidence: `hardware-rt-{primary,pbr}-after-reset.log` and the rendered images
  in `target/verification/hardware-rt/`. Initial renderer tests timed out while
  even an empty GPU submission stalled; those failed runs are not passes.
  Following device/session recovery, the above explicit tests passed. Capture
  now tracks its submitted readback and returns a finite timeout with context.
  The final-source integration invocation passed both tests together in 7.23s:
  `target/verification/hardware-rt-renderer-gpu-final.log`.
- GPU CLI/regression behavior: the real CLI raster/hardware selection and
  encoded-work-report test passed on the recovered Metal device. The existing
  raster PBR sky/lighting/sun-shadow pixel regression also passed. Evidence:
  `hardware-rt-cli-render-final.log` and `hardware-rt-raster-pbr-final.log`.
- Default workspace validation: `cargo test --locked --workspace` passed,
  with 581 passed, zero failed and 55 ignored, respecting the documented
  upstream `lib.test=false` settings. Ignored tests are not counted as passes;
  the explicit Metal runs above provide separate GPU evidence. Evidence:
  `target/verification/hardware-rt-engine-workspace-tests.log`.
- Limits: single world-space BLAS geometry/identity TLAS instance, at most
  1,000,000 primitives; no refit/compaction/per-mesh instancing, RT reflections,
  RT GI or denoiser. Masked/transparent casters and custom fragment-discard
  materials fail visibly. Fixed world-space ray bias is an initial profile.
  Windows/Vulkan/DX12 and NVIDIA/AMD/Intel physical execution remain unverified.
  Core/compat remain device-free and forbid unsafe code; the renderer isolates
  wgpu's required unsafe experimental-consent token at explicit device opt-in.
  No game data, fonts, proprietary SDKs were added.

## 2026-10-26 — Frame DAG, profiler, leased streaming and instanced RT

- Implementation: ADR 0052 adds a bounded dependency executor over immutable
  results. Independent PBR/simple/layer extraction runs on workers; desktop
  polls one pending snapshot while retaining one ready snapshot. World/VM/input
  ordering stays on its owner. Dynamic surface generations and transition state
  freeze with snapshots; scoped resource lookup restores live access on error
  or unwind. Restore/store identity rejects stale results.
- Implementation: shared Profiler class/global exports bounded CPU traces and
  stage percentiles. GPU timing uses four asynchronous slots, pass-boundary
  markers, frame-completion callbacks, a later resolve/map submission and
  validated nonzero samples. Capture/shutdown have explicit bounded completion
  fences. CPU/GPU clocks remain uncalibrated; capture wall time is not FPS.
- Implementation: streaming volumes prepare hierarchy/PBR data on workers,
  atomically publish ready cells, reject obsolete work and hold scoped resource
  leases. The renderer shares decoded models and releases unused model/RT
  caches. Explicit caller-owned asset requests retain their lifetime. Host CPU
  prepared-cache budgets default to 256 MiB and are explicitly configurable
  from 16 to 4096 MiB; this is not a total RAM/VRAM limit. Offline readiness
  fences commit resources without another script/physics/time step. Ticket
  validation also rejects forged client fields on another client's request ID.
- Implementation: PBR shadows share local-space BLAS and use affine TLAS
  instances. Cached source/pose data avoids repeated rigid-mesh preparation;
  source Arcs prevent pointer-ID reuse. Static frames reuse TLAS; rigid moves
  rebuild only TLAS. Budgets are 4096 instances/1024 unique meshes/1M unique
  triangles, bounded by device limits. The simple primary path stays separate.
- Compilation/CPU validation: workspace all-target check passed. Default
  workspace tests passed 590, failed zero, ignored 56. Focused tests cover real
  concurrent graph execution/dependencies, pressure retry/failure cancellation,
  delayed workers with advancing simulation, immutable geometry/surface data,
  restore rejection, leased/shared cache reclamation, budget policy, forged
  tickets, streaming hierarchy/collision/save behavior and offline clock fences.
  Python tooling passed 165 tests. Generated API checks passed five tests;
  fingerprint changed `df17e49431e56860` → `e3a0831acf720cb7`, logged with the
  streaming readiness migration in docs/api/CHANGES.md.
- GPU validation: explicit Apple M5 / Metal primary/PBR integration and GPU
  profiler tests passed (2 + 1). Assertions cover matching primary pixels and
  overlays, shared BLAS, static TLAS reuse, moved-instance pixels without BLAS
  rebuild, opaque-profile rejection, explicit raster recovery and actual
  nonzero timestamps. The final profiler run recorded eight valid samples,
  zero failed/dropped/pending. Earlier immediate timestamp resolution produced
  invalid/zero samples; those were rejected and are not successful measurements.
- Public showcase behavior: existing Khronos resource files matched all local
  manifest hashes (2 helmet/HDR files; 73 Fox/Sponza/HDR files). Nine textured
  helmets plus floor rendered as 10 native RT instances/2 BLAS, 139,080
  instance-expanded triangles; eight rotating captures built BLAS only twice
  and TLAS eight times. All eight final GPU timing samples were valid. The
  streaming movie produced entity counts 1 → 3 → 1 → 3 at the expected logical
  radii/time, reclaimed prepared data to zero after unload and reloaded it.
  The prepared cell was 112,737,441 bytes; no game VM/renderer was hosted.
- Public larger-scene behavior: Sponza failed visibly under the default
  256 MiB prepared budget. With an explicit 1024 MiB host budget it rendered
  through CrySirror's raster material path, retaining 392,730,673 prepared bytes.
  Four final GPU samples were valid, with no failed/dropped/pending samples.
  Source alpha-tested materials were not forced into the opaque RT profile.
- Window behavior: an earlier 1800-presented-frame run completed with actual
  10-instance/2-BLAS window rendering and a trace spanning owner/worker threads.
  Its profiler trace predates the final completion-order fix and includes
  rejected samples; use the final GPU tests/offscreen reports for timing
  evidence. A later repeat did not reach its presentation count and was stopped;
  sampling showed continuing owner/surface-acquisition activity. That repeat
  is not a pass. Window visibility/surface timing is not an offscreen result.
- Evidence: target/verification/frame-pipeline-{workspace-check-final.log,
  workspace-tests-final.log,api-docs-final.log,python-tests.log,
  lifecycle-final.log,gpu-final.log}; frame-pipeline/{helmet-report.json,
  streaming-report.json,sponza-report.json,*.png,
  interactive-pre-completion-fix-trace.json,interactive-hang.sample.txt}.
  Recipes/attribution links are in docs/FRAME_PIPELINE.md; downloaded assets,
  captures and traces stay ignored.
- Limits: ADR 0050 remains partial. General simulation parallelism, budgeted
  GPU uploads/pipeline compilation, collision cooking, full resource GUID/cook
  databases and chunk/mip/world-partition streaming remain. World freezing still
  clones logical state; skinned geometry changes rebuild BLAS. RT alpha
  candidates/refit/reflections/GI/denoising, an editor/profiler overlay, complete
  GPU memory accounting and other desktop hardware/platform validation remain.
  No proprietary data, fonts or SDKs were added.

## Native VM API expansion and ray lighting (2026-10-26, ADR 0060–0063)

- Implementation: native GGX glossy reflections now sample the actual material
  roughness. Optional per-pixel diffuse GI shares a two-layer array with the
  reflection target, retaining the forward pass's 16 sampled-texture bindings.
  Reflection, probe-GI and pixel-GI hits use the shared local-light buffer.
  Probe dispatches read an immutable history copy, removing inter-workgroup
  read/write races. Native device negotiation checks six storage-buffer
  bindings. Nonuniform-scale hit normals and environment-zero reflection
  attenuation were corrected. These extend the earlier mirror/probe-only limits.
- Implementation: AVG32 is a pinned Git library dependency on original
  the original project portable-vm branch. Its portable build
  compiles the same Scenario/parser/control-flow source; no VM source was copied
  into CrySirror. The dependency tree contains no original System, renderer,
  font/image painter or audio device. The thin adapter issues scoped engine
  Canvas/GlyphRun/UI/audio operations, preserving original pending events,
  waits, hidden choice ordinals, initial RNG state and VM-relative clock.
  Both native saves and framework snapshots remain visibly unsupported here.
- Implementation: Bitmap.composite/filterRgb add engine-owned associated-byte
  composition, separate coverage and integer fade/multiply. Associated output
  is intermediate data, not an implicit conversion to straight-alpha display.
  The new AVG32 profile currently accepts only opaque full-size image loads;
  these primitives do not establish complete PDT effect/mask compatibility.
- Compilation: locked workspace all-target check and the desktop build with
  selected adapter features passed. Default workspace tests completed: 623 passed, zero
  failed, 67 ignored. Python tooling passed 170 tests. Generated API checks
  passed 5/5; final fingerprint changed 3ff1c210d5f93836 → 1050976234c41b17
  and is logged in docs/api/CHANGES.md. The CLI registry assertion was updated
  to verify the actual feature set, including AVG32.
- CPU behavior: AVG32's native bridge passed five tests; its licensed-font test
  was then explicitly executed and passed. The original upstream native engine
  passed 33 library, one NLS and four synthetic tests (one installed-game audio
  case filtered); portable-vm passed 11 tests. Shared bitmap differential tests
  matched the unchanged original pin for 1,776,384 associated cases and 2,304
  signed/clipped fades, plus existing 19,200 / 9,216 / 12 cases. Explicit system
  font tests also passed for engine measurement and script rich text.
  Ignored font tests in the default suite are not counted as default passes.
- GPU behavior: Apple M5 / Metal explicitly passed all 46 wgpu GPU tests in
  one serialized run. New tests distinguish a rough reflection from the mirror
  footprint and isolate local-light GI with sun/environment/emission disabled.
  Floor red-channel excess was 0 without GI, 29.67 with RT probes, 25.11 with
  per-pixel GI and 0 with the local light disabled. Raster requests for pixel
  RT GI fail visibly. A first GPU-suite setup lacked the requested system font
  and failed that test; the later complete font-supplied run passed.
- GPU performance: same 160×90 room, 24 valid steady GPU frame samples per
  mode, mean 7.578 ms raster probes / 0.810 ms RT probes / 0.644 ms pixel GI.
  Probe modes refresh 16 probes/frame; pixel GI uses 64 rays/pixel. These are
  different representations/sample densities, not equal-quality speedup claims
  or isolated GI timings. RT-probe warmup dropped three timing-ring samples;
  no failed/pending measurements remained. ADR 0060 records p50/p95 and limits.
- Render behavior: the public Khronos DamagedHelmet/HDR files matched both
  recorded hashes and rendered with three roughness panels, a local light,
  native reflections and pixel GI: four instances, two BLAS, eight reflection
  and eight pixel-GI dispatches. The AVG32 TPC32
  example rendered background/text/wait icon and a subsequent native input
  trace reached Halted with 81 engine API calls and no private framebuffer.
  Configured font/cell metrics are explicit; these captures are not commercial
  game completion or proof of matching a title's original fonts/layout.
- Remaining limits: pixel GI is one diffuse bounce, without dedicated GI or
  glossy denoising. Hit shading retains atlas/geometric-normal/constant-emission
  approximations; transparent/transmission and full path tracing remain out.
  Other GPU vendors/platforms were not physically tested. AVG32 complete
  native drawing/layout/save support remains unfinished. New native
  VM steps publish on the owner; bounded first-use resource reads still need the
  remaining ADR 0050 async migration. No old engine host, game
  data, font file, credential or proprietary SDK was added.
- Evidence: target/verification/vn-rt-{workspace-tests-complete.log,
  all-targets-verified.log,python-final.log,api-verified.log,
  avg32-dependencies.log,*-report.json,avg32-trace.json};
  target/verification/avg32/{cpu-tests-final.log,font-test-final.log};
  target/associated-upstream.log; rt0060-all-gpu-font.log and
  rt0060/gi-benchmark.json; rendered PNGs in target/verification/vn-rt/.

## Modular CPU/GPU VFX (2026-10-26, ADR 0064)

- Implementation: added safe, device-independent `crysirror-vfx` definitions,
  compilation, deterministic CPU simulation, typed parameters, multi-emitter
  systems, five spawn shapes, ten update modules, checked lifetime curves,
  world/local space, analytic collisions and bounded deferred CPU child events.
  Engine `Vfx`/`Entity` APIs and data `EntitySpec.vfx` use the same state.
  Legacy `particles` lowers into this core; its renderer-private CPU simulator
  was deleted. No Unreal source, VM host, game data or fonts were added.
- Async implementation: runtime moves simulation ownership into the bounded
  worker pool and publishes immutable frame products. Live frames poll without
  waiting. A bounded input journal retains intermediate parameters/transforms
  while workers are busy; fixed-step debt is drained before the next sample.
  Full journals fail visibly and an explicit offline fence can drain/recover
  them without advancing World/VM clocks. Epochs reject stale restore products.
- GPU implementation: resident 96-byte particle state, parallel update/death,
  deterministic free-slot prefix allocation and parallel birth initialization,
  active compaction, bounded device event records and indirect drawing. GPU
  targets do not run CPU particle loops. Historical tick packets include the
  parameters/transforms at that tick. The owner retains unsubmitted commands
  under packet/byte budgets; receipts advance after queue submission. A second
  direct PBR encode with pending work fails instead of resimulating a tick.
- Render implementation: texture/flipbook sprites, camera/velocity/axis facing,
  soft depth intersections, four blend modes, static mesh instances, CPU ribbon
  bands, per-view culling and GPU alpha sorting. Resource access uses ordinary
  scoped Assets; no extra texture bindings enter the PBR material pipeline.
  Inspection separates CPU counts, GPU capacity/submissions, resident buffer
  bytes and pending commands. Live GPU counts stay null without readback.
- Compilation: local `cargo check --locked --workspace --all-targets` and the
  default desktop build succeeded. Existing image/RealLive warnings remain.
  API reference/types regenerated with 32 distinct VFX data aliases; signature
  fingerprint changed from `1050976234c41b17` to `ff5f3d2050247846` and is logged.
- Workspace behavior: the normal locked workspace suite completed with 659
  passed, zero failed and 76 explicitly ignored tests. The subsequently added
  end-to-end GPU integration test was run explicitly, including its BGRA
  desktop-capture path. Ignored tests are not counted as hardware passes.
- CPU behavior: core 18 tests, runtime 10 tests, engine VFX API 2 tests and API
  documentation 6 tests passed. Tests include paused/stopped/restarted state,
  scoped events, time-zero initialization, transactional type errors, retained
  GPU histories beyond 32 ticks, hidden GPU simulation and submission receipts.
  A deliberately blocked worker produced exactly the same particle arrays as
  a per-frame offline baseline across changing parameters/transforms. Queue
  overflow and subsequent offline recovery also passed. Python 3.11 tooling
  passed 181 tests, including 11 new VFX schema/resource/MCP tests. Strict
  tooling rejects unknown modules, invalid bindings/curves, unsupported GPU
  profiles and competing components. Asset inventory covers initial entities
  and widget spawn/set-VFX textures/static meshes; client/symlink isolation and
  rejected MCP writes were exercised without a GPU or VM.
- Actual GPU behavior: Apple M5 / Metal explicitly passed all five GPU compute
  tests, three render pixel tests, the private cull/indirect test, the engine
  timeline/PBR/submission integration test and the existing terrain/HDR legacy
  particle test (11 device tests total). The integration test submitted every
  tick of a 61-tick history, compared paused pixels, rejected duplicate pending
  encoding, then abandoned/retried encoding without advancing a receipt early.
  Device tests remain ignored in the default suite; their explicit runs are
  separate evidence. Other desktop GPUs/platforms were not physically tested.
- GPU performance: 100,000 resident particles, 24 timestamp samples per scope.
  Update/compact with acceleration, drag and curl: mean 66.448 us, p50 71.250 us,
  p95 95.000 us. Independent parallel 100k birth/allocation/compaction at dt=0:
  mean 358.286 us, p50 115.875 us, p95 761.083 us, maximum 1.991 ms. These
  GPU-only intervals exclude draw, CPU encoding, allocation and image/readback
  cost; the wide burst distribution is retained in the raw JSON. No whole-frame
  or equal-quality Niagara performance equivalence is claimed.
- Integrated behavior: the public DamagedHelmet/HDR showcase rendered through
  the actual desktop JS/TS API with GPU fire, textured flipbooks, CPU soft smoke,
  instanced shards, collision child sparks and a moving world-space ribbon. Four
  consecutive captured frames kept one renderer. The 100k stress script also
  rendered with zero CPU particle arrays, 116 submitted ticks, all 100,000 birth
  requests, zero unreplayed ticks and zero pending commands after capture.
  The same PBR/effects showcase also rendered on the native hardware RT path
  with three RT scene instances, two static BLAS and one TLAS build. Particles
  coexist with that pipeline but do not themselves enter the RT structures.
  Desktop captures now reuse the live renderer, retain resident particle state
  and convert BGRA surfaces into RGBA PNGs; the BGRA pixel test passed explicitly.
  A real winit window also ran the 100k script for 60 presented frames, captured
  its retained GPU particles on exit and exported a trace. Its 60 completed
  whole-frame GPU samples (including startup; simple additive-particle scene)
  measured mean 1.347 ms, p50 0.609 ms, p95 2.378 ms. One timing-ring sample was
  dropped, none failed or remained pending. The explicit offline VFX fence
  occurred only for the final capture, not once per interactive tick.
- Limits: this is a substantial modular VFX profile, not complete Niagara
  parity. Visual graphs, arbitrary custom compute, skeletal/scene/physics data
  interfaces, fluid simulation and effect-level significance/LOD scheduling
  remain unfinished. GPU ribbon/child emitters/replace-oldest fail explicitly;
  CPU supports them. Mesh particles are static unlit geometry with texture/color.
  Transparency is not sorted jointly with ordinary scene meshes. Particle
  probe/RT visibility and exact save/device particle checkpoints remain out.
  Replacing a renderer after its history is retired cannot reconstruct a paused
  GPU simulation; this fails visibly. Canonical restore resets derived visuals.
- Evidence: target/verification/vfx-{core-tests-final,runtime-tests,
  api-runtime-final,integration-gpu,legacy-particles-gpu,all-targets,
  desktop-final-build,desktop-capture-build,integration-bgra-gpu,
  changed-crates-final,all-targets-final,python-complete}.log;
  vfx-gpu-parallel-final.log, vfx-gpu/benchmark.json,
  vfx-render-{gpu,cull-gpu,geometry,cpu}.log, vfx-demo-final.json,
  vfx-stress-final.json, vfx-demo-rt.json, vfx-live.log and PNGs/movies plus
  live-profile.json in target/verification/vfx-demo/.

## 2026-10-26 — Native resource migration and asynchronous conversion (ADR 0065)

Implementation:
- Continued the unfinished arc_unpacker batches and wired every concrete named
  decoder from clean pin `456834ecf2e5686813802c37efd829310485c57d`: 152 archives,
  96 images, 16 audio and 25 file converters. The reviewed ledger records
  **258 ported / 31 partial**, including original-source unsupported branches,
  explicit encrypted-title profiles and bounded Unity/PSB/pickle/XP3 variants.
  There is no pending ID registered as a success stub. This is resource-codec
  migration, not a new native VM or full-game compatibility claim.
- Shared engine contracts now include FileRegistry/DecodedFile, capability-only
  companions, cycle/depth checks, LZSS, generic crypto primitives, WAVE/loop
  serialization, PNG output, channel masks and nontransparent pixel copying.
  Format-specific layouts/codecs stay in the optional GPL plugin. HCA uses
  safe Rust cridecoder; no original-engine process or C/C++ codec is hosted.
- Resources exposes file/audio discovery, paging, converted-file mounts and
  bounded client-scoped decode jobs. Provider-backed source opening and codec
  work both run on CPU workers; owner polling/publication never joins them.
  Cancellation discards late results; mounted readers retain their original
  profile/budget. Synchronous file probe/conversion APIs are offline-only.
- Fixed companion authority lifetime without a VFS/provider reference cycle;
  checked WARC short-tail/index/region-image limits and S25 aggregate row and
  predictor-work limits. Native keys remain host aliases, not bundled tables.
- Regenerated docs/api with fingerprint `d4f4c6edc250ee72`; CHANGES records the
  new APIs, data specs and Rust PixelMath exhaustive-match compatibility change.

Compilation:
- Local `cargo check --workspace --all-targets`: passed.
- Local engine + desktop `arc-formats` all-target check: passed.
- Latest arc-enabled API/file-resource tests compiled the realtime probe gate
  and worker-side mounted-source opening. Existing unrelated warnings remain.

Behavior validation:
- Full local `cargo test --workspace`, including documentation tests: **780
  passed, 0 failed, 85 ignored**. Ignored GPU/font/external tests are not passes.
- Arc-enabled engine resource/image/file suites plus API docs: **19 passed**;
  final realtime-source/gate/API rerun: **13 passed**. These overlapping runs
  are not added together as distinct tests. No VM, GPU or audio device required.
- Final i-family review: **8 native unit tests passed** and **4 archive tests
  passed with the external S25 corpus explicitly enabled**.
- Final external pinned corpus: **15 test groups passed**. The separate FC01
  external-key test was excluded and is not claimed as a pass. Fixture media
  remains outside Git. Existing lossy-image tolerances and alpha0 hidden-RGB
  comparison rules remain explicit in the ledgers.
- Original pinned WARC C++ table-cipher functions compiled and reproduced
  **3 portable vectors**; Rust matches them. This excludes title-key branches
  and upstream's undefined negative floating-to-unsigned conversion behavior.
- Python 3.11 tooling: **187 tests passed**, including inventory/source-linkage
  checks.

No commercial-game run, console validation or hardware-render result is claimed
from these CPU codec/API checks. Per-format limits remain in RESOURCES.md,
arc-resource-support.json and arc-image-support.json.

## 2026-10-27 — Awaitable services, shared menus and native plugin facades (ADR 0066)

Implementation:
- Added engine-owned bounded OperationTable tickets with client/restore-generation
  checks, retained completion/failure, cooperative cancellation, dependent-phase
  backpressure and owner publication. Native resource jobs share its ID namespace.
- Resources.beginProbe/beginImage open scoped inputs and prepare results on
  I/O/CPU workers. Ready image pixels publish by shared immutable Arc ownership;
  both client-profile and host-wide 256 MiB staging reservations retain their
  lifetime. Existing dynamic surfaces retain their independent 256 MiB budget.
- Text parsing, shaping, icon loading and rasterization now have worker APIs.
  Reveal events publish with their corresponding pixels, and failed replacement
  keeps the previous block. Restored generations discard stale work.
- Save captures native checkpoints/logical state at an ordered safepoint, then
  workers encode, perform I/O and validate. Ready loads remain staged until
  atomic commitLoad. Native capture/reconstruction and unsupported-save behavior
  remain explicit; the existing snapshot wire format is preserved.
- JS/TS has generated CancellablePromise helpers for Assets/Resources/Text/Save,
  client-scoped continuations, bounded Boa microtask/future turns, async init/start
  fences, pending-hook reentry prevention and bounded shutdown draining.
  Blocking legacy resource/text/save operations reject live-host use.
- Engine MenuSpec/MenuItemSpec owns trees, radio groups, popup/bar layout,
  mouse/keyboard input, paint and saved logical state. Ui.drainEvents consumes
  client-scoped events once; ordinary reads retain them. Menus share the existing
  last-256 UI event policy. Snapshot validation includes menu/focus/event owners.
- Native MenuItem translates handles/properties/callbacks onto these APIs.
  Plugins distinguishes engine_facade, explicit registration_only shells and
  unsupported modules. menu.dll is a real facade; shells export no fake APIs.
  No native DLL, extra engine implementation, compositor or device is hosted.
- Added a standalone TS menu/async-text/save example, ADR 0066 and migration
  documentation. Regenerated docs/api: d4f4c6edc250ee72 -> 5394949c5999cde6.
  Edited sources use full license notices while retaining third-party licenses
  and authors; pinned upstream files/hash manifests remain unchanged.
- Cleaned rebuildable target/debug after it reached about 313 GB (165 GB
  incremental and 145 GB dependencies). Available disk increased from about
  24 GiB to 336 GiB immediately after cleanup; after release verification about
  329 GiB remains. Release output (~11 GB), local data (~44 GB) and verification
  evidence were preserved; debug output is now about 621 MB.

Compilation:
- cargo check --release --locked --workspace --all-targets: passed on macOS.
- Formatting of the 38 edited Rust files and git diff whitespace checks: passed.
  No claim that unrelated existing whole-workspace formatting is clean.

Behavior validation:
- Explicitly executed all three system-font tests using a runtime system font;
  these cover offline/worker pixel equality, reveal publication, native font
  callbacks and the standalone TS example. No font is committed. The example
  verifies a real menu click saves once and stays single after 30 more frames.
- Gates verify live frames progress while CPU/I/O workers are blocked, failed or
  cancelled work does not mutate live state, prepared loads require commit,
  mounted sources open on workers and foreign/stale tickets cannot publish.
  Tests also cover foreign menu/event owners, event consumption, held-button
  dismissal and malformed saved menu paths.
- Python 3.11 tooling: **189 passed**, including pinned-source policy and menu
  authoring validation. The external complete-movie test remains ignored; an
  accidental include-ignored attempt lacked its explicit MPEG path and is not
  counted as a pass.

Limits:
- ADR 0050 remains partial: native interpretation/storage callbacks and startup
  compilation still have synchronous boundaries; native checkpoint capture and
  reconstruction remain owner operations. Remaining renderer/audio/media/editor
  scheduling gaps keep their prior status. This is not all-engine async completion.
- Text blocks require explicit Text.state/restoreAsync reconstruction; media
  snapshots and unsupported original-VM checkpoints still fail visibly. No new
  commercial-game, GPU, hardware-RT or cross-platform result is claimed here.

## Shared acoustic scenes and worker direct sound (ADR 0067, 2026-10-27)

Implementation:
- Added device-free `crysirror-acoustics`: engine-owned acoustic mesh/material
  resources, per-triangle materials, affine transform baking, default Parry
  TriMesh/QBVH and optional obvhs 0.4.0 BVH2. The old physics occlusion path
  already used Rapier/Parry QBVH; this adds distinct acoustic geometry and
  propagation, not the engine's first BVH.
- Reviewed pinned phonon_rs direct/scene/model/DSP sources; adapted volumetric
  visibility, exponential air absorption and weighted-dipole directivity with
  original Apache/Valve/phonon attribution. Source inventory and semantic
  differences are in crysirror-acoustics/UPSTREAM.md. No native SDK or HRTF data
  is bundled. BVH build/traversal use existing libraries.
- Added multi-ray source-volume visibility, three-band per-surface amplitude
  transmission, air loss, directivity and reported travel time. Existing mixer
  consumes smoothed three-band coefficients without querying geometry, retaining
  its voice pool, distance, pan, buses, ducking and envelopes. New acoustic
  voices are muted until their first result rather than briefly leaking sound.
- Audio API prepares scene assets and probes on bounded workers, publishes
  immutable client/revision resources on the owner, and exposes status/cancel/
  clear/stats/per-voice results plus JS/TS cancellable async helpers. Automatic
  jobs coalesce poses and use bounded round-robin dispatch. Reservations follow
  pending/captured scenes through cancellation/clear; restore invalidates them.
- Corrected original-origin ray intervals and an obvhs shared-edge signed-zero
  miss (use Parry triangle intersection inside obvhs traversal). Legacy physics
  occlusion now explicitly queries the voice's client. One existing archive
  test buffer needed an explicit Vec<u8> annotation with the additional glam
  dependency; no archive behavior was changed.
- Regenerated docs/api and logged fingerprint `5394949c5999cde6` ->
  `0d7b882595278ffb`. Added ADR 0067 and a synthesized offline acoustic-room demo.

Compilation:

Behavior validation:
- Release targeted suites: **56 passed, zero failed, three ignored** across
  default acoustic propagation (6), audio library/channels/offline PCM (25),
  engine acoustic lifecycle (6), engine async services (5), engine mixer (4),
  JS async services (4) and generated API checks (6). The ignored cases need
  explicit external NWA/system-font input and are not counted as passes.
  The obvhs-enabled propagation suite separately passed its five tests.
- Worker gates prove frame progress while builds/probes are pending; tests
  cover foreign owners, cancellation, failed replacement, retained-result
  capacity, restore invalidation and immutable probes surviving scene clear.
  Native physics occlusion for client 2 uses its own wall. A blocked acoustic
  voice produces zero actual PCM, including before initial query completion;
  clearing the scene restores audible direct samples through the same mixer.
- JS awaits prepare/probe, checks scene revision and attenuation, and catches
  missing-asset rejection. Successful helper completion releases tickets while
  leaving scene resources installed. No native VM is involved in these tests.
- Reproducible BVH2 microbenchmark: 32768 terraced triangles, 100000 seeded
  downward rays with misses and interval clipping; all hits checked against an
  analytic oracle. This host measured Parry build/query medians 4.891/13.605 ms
  and obvhs 3.885/21.748 ms. Single-thread workload only; no general speed or
  CWBVH/GPU claim. Parry remains default.
- The synthesized nine-second 48 kHz stereo WAV has clear/wall/clear sections.
  Measured wall/clear amplitudes at 200/3000/10000 Hz were 0.683/0.319/0.212;
  the final clear section matches the initial section. Crossover responses
  overlap; these are sample measurements, not exact material-band gains.
- Python 3.11 tooling: **189 passed**. Evidence: target/verification/
  acoustics-{library-tests,regression-tests,api-verify,core-tests,python-tests}.log,
  acoustics-workspace-check.log, acoustics-bvh-compare.json,
  acoustics-audio-demo-analysis.json and acoustics-room.wav. Output/media stays
  ignored under target; all demo audio is synthesized from authored tones.

Remaining boundaries:
- HRTF, diffraction, early reflections, late reverb and dynamic mesh refit are
  not implemented by this profile. Absorption/scattering remain validated
  metadata reserved for reflections; travel time is not applied to playback.
  Source forward is a world direction, without automatic entity rotation.
- Audio source opening/decoding, streaming decoder reads and mix/output still
  have synchronous boundaries; ADR 0050 is not complete. BVH library builds
  are bounded non-preemptible phases with cancellation between phases. Ready
  coefficients can be stale while workers are busy; there is no latency SLA.

## HRTF resources and engine binaural voices (ADR 0068, 2026-10-27)

Implementation:
- Added immutable engine HrtfDataset/HrtfOptions/HrtfEarOrder resources in
  crysirror-acoustics. Inputs are SOFA SimpleFreeFieldHRIR/FIR or prepared
  HrirAsset JSON. SOFA resamples through sofar; JSON requires the mixer rate.
  Native SOFA uses library neighborhood interpolation; prepared JSON weights
  the nearest three directions. No measured human HRIR is bundled.
- Integrated the standalone sofar 0.4.0 DSP library, pinned at
  303532bd609db2d74c1b9e7f7483666dad1210fa. Its SOFA reader, FFT plans,
  partitioned convolution, per-ear delays, crossfades and filter mailbox are
  library components, not another game engine or mixer. vendor/sofar retains
  original licenses/source hashes and one audited HDF5 v3 range correction.
  No native libmysofa SDK, audio-device demo or workflow is imported.
- Added preflight checks for original/resampled dimensions, sample rates,
  required coordinate Type/Units, little-endian numeric encodings, finite FIRs,
  delay axes and supported listener frames. Fixed the M=R=2 global-ear-delay
  ambiguity using Data.Delay dimensions. Ear order is explicit: authored
  left_right by default, right_left or receiver_positions when requested.
  The local CIPIC profile demonstrates why metadata must not silently flip
  conventional response-channel order.
- Audio beginHrtf/status/cancel/clear/stats/voice controls and prepareHrtfAsync
  prepare datasets, voice buffers and filters on bounded workers. Ready
  resources publish on the owner; failed replacements preserve the prior
  profile. Latest direction updates coalesce. Limits are 32 voices/client,
  64/host and 16 in-flight voice/filter jobs. Epoch checks and storage roots
  remain client-scoped. Reservations follow actual retained stream buffers.
- playSfx accepts binaural:true with spatial/follow. The existing mixer replaces
  spatial panning with convolution while retaining acoustic bands, distance,
  gain/envelopes, buses and ducking. Pending initial preparation freezes short
  sources; arbitrary block sizes retain history; natural EOF drains FIR/ear
  delay tails. Pause/seek and disabling have explicit behavior. The default
  bridge adds 64 frames (1.33 ms at 48 kHz), separate from device/queue latency.
- hrtfVoice reports published_direction and pending preparation; it does not
  pretend the audible crossfade has already completed. Resource replacement
  re-prepares enabled voices; seamless profile replacement is not claimed.
- Added authored netCDF4 SOFA impulses, including HDF5 v3, reversed receiver
  records, bad units and a missing coordinate type. Generator dependencies
  are development-only. Added an offline synthesized-source orbit example.
  Regenerated docs/api; fingerprint 0d7b882595278ffb -> c619eaa9bacd0fc3.

Compilation:
- The vendored source audit verified all 33 original Rust-source hashes after
  reversing the single declared parser fix; zero unexpected source changes.
  Manifest changes exclude upstream native/device/data test targets. This
  does not count the dependency's omitted external-data suites as passes.

Behavior validation:
- Release library suites: **41 passed, zero failed, two ignored** (external
  SOFA and owner NWA fixtures). Engine/JS/API suites: **31 passed, zero failed,
  two system-font tests ignored**. Thus default targeted suites passed 72;
  ignored tests are not counted as default passes.
- The external SOFA case was separately executed successfully with the explicit
  local CIPIC path. The complete HRTF kernel suite, including that normally
  ignored case, passed **8 tests**: 1250 measurements resampled to 48 kHz,
  218 taps, finite nonzero stereo output. This input remains local/ignored.
- Analytic FIR/sample-point oracles verify partition latency, tail preservation,
  ear delays, crossfades, reset and three ear-order policies. The M=R=2 fixtures
  verify constant ear delays independently of measurement index; the HDF5 v3
  case verifies the dependency fix. Bad/missing coordinate metadata fails.
- Engine worker gates prove dataset and initial voice work leave frames free
  to progress. Tests verify cancellation, failed replacement, bounded results/
  voice admission, idempotent enable, listener rotation and real PCM changes,
  restore invalidation, distinct same-named profiles under two client roots,
  foreign handles and clearing one client without changing the other's PCM.
- JS awaits resource preparation, checks metadata, verifies automatic ticket
  release with the installed resource retained, and catches missing assets.
  Native audio predictor/channel/mixer regressions remain in the library suite;
  no new VN opcode or complete game boot is claimed by HRTF.
- Python 3.11 tooling: **189 passed**. Evidence in target/verification:
  hrtf-{library-release-tests,engine-js-release-tests,core-and-sofa-tests}.log,
  hrtf-workspace-release-check.log, hrtf-python-tests.log and
  hrtf-source-audit.json. Offline orbit output and timings are in
  hrtf-orbit.wav / hrtf-orbit.log; all source audio is synthesized. The Apple M5
  single-voice offline run measured dataset read/plan 62.244 ms, eight seconds
  of mixer rendering 16.414 ms, and 80 filter updates 0.151 ms. Source allocation
  and final tail drain are outside the render timing; this is no multi-voice
  deadline claim. hrtf-orbit-analysis.json verifies stereo energy and final zeros.

Remaining boundaries:
- Fixed-reference HRTF is not dynamic near-field propagation, diffraction,
  early reflections or late reverb. Direct travel time remains metadata only.
  Unsupported SOFA conventions, legacy object-header v1 and some generic HDF5
  group layouts still fail visibly. No all-format or subjective localization
  quality claim is made from the impulse tests or generated WAV.
- Full audio async remains open: source opening, decoder reads, mixing/output
  and teardown retain their prior synchronous boundaries. DSP processing
  preallocates its own buffers, but the existing mixer still allocates scratch
  and drops ended voices during render. This is not a hard real-time or
  all-engine async completion claim. Host reservations are estimates, not a
  process-memory sandbox; library phases are not preemptible mid-call.
- No new GPU, device-output or Windows/Linux behavior result is claimed. Native
  VM syscall/wait/save mappings retain their previous integration status.

## Worker audio decoding and playback (ADR 0069, 2026-10-27)

Implementation:
- Added reusable TaskPermit admission so a caller reserves bounded queue
  capacity before moving a decoder/resource into a closure. Unused permits
  release their slots. TaskStats exposes reserved admission separately.
- Added device-free BufferedSource and explicit Frames/Pending/End, service,
  poll_seek, read_position and buffering status to shared AudioSource. One
  idle owner or one worker owns the decoder; immutable PCM chunks cross the
  boundary. Short positive reads are not mistaken for EOF. Regular read-ahead
  is bounded; initial/custom loop prefixes remain resident. Existing blocking
  decoder/seek contracts remain explicit and available to offline/native paths.
- Mixer interpolation consumes ready PCM, retains the previous chunk's edge
  sample and stalls only the starved voice's source/envelope/DSP clocks. Other
  voices continue. Stats report per-voice underrun frames. Exact seek sequences
  freeze until ready, discard stale generations and support cancellation back
  to the previous fractional clock. Synchronous seek rejects buffered sources.
- Added engine beginPreloadSfx/beginPlaySfx/beginPlayMusic/beginSeekVoice,
  playbackStatus/cancelPlaybackJob/sourceStatus/clearSfxCache and corresponding
  JS/TS await helpers. Capability reads, source probing, decode and priming run
  on bounded I/O workers. Owner publication revalidates spatial dependencies
  and uses the same mixer/acoustics/binaural attachment APIs. Named music only
  replaces/fades previous playback after successful preparation and admission.
- Retained jobs are limited to 16/client and 64/host. Shared residency receipts
  follow cached/live clips and worker decoders; the 512 MiB estimated audio
  budget survives reset while references remain. The cache has its own 128 MiB
  bound. These estimates do not comprehensively account provider/codec/transient
  allocations and are not a process-memory sandbox (see ADR 0069).
- Realtime cold synchronous SFX and music now reject with the async migration
  path; preloaded SFX/random variants and explicit offline calls remain usable.
  Cache clearing does not stop playing clips. Added generation checks at Audio
  API entry, fixing restore before the first frame leaving a prior voice alive.
- Regenerated docs/api and recorded fingerprint c619eaa9bacd0fc3 ->
  2d945b181a39e802 with realtime behavior migration instructions.

Compilation:
- Local release workspace/all-target check passed with desktop
  arc-formats and acoustics-obvhs features. Existing CBG dead-code and RealLive
  unused-variable warnings remain.

Behavior validation:
- Shared task/audio release validation passed 45 distinct tests (including
  seven buffered-source cases), with one owner-asset NWA test ignored. Engine/
  audio/acoustics/binaural/JS suites passed 31 tests, with one system-font demo
  ignored. API documentation/signature checks passed six tests. Total: 82
  distinct Rust tests passed; the two ignored cases are not passes.
- Analytic PCM and worker gates verify blocked I/O leaves other voices/frames
  progressing, pending PCM is not EOF, pool backpressure retains decoder
  ownership, fractional chunk interpolation matches resident PCM without
  backward decoder seeks, custom loops match, pinned heads obey queue limits,
  exact seek generations supersede stale work, and cancellation restores the
  prior fractional clock. Decoder read/seek probes execute off the owner.
  A worker packet failure retains queued PCM and then reports its error,
  rather than reporting a successful EOF or discarding the valid prefix.
- Engine tests verify cancellation before and after worker completion cannot
  start unpublished playback, per-client same-named assets/foreign tickets,
  bounded result admission, shared cache lifetime, failed music replacement,
  uncached seek readiness with different sample values at its destination,
  stop-during-seek ticket release and restore before the first audio frame.
- An authored NWA stream is opened through capability storage and decoded/
  looped by the shared Audio API without a native VM. JS/TS awaits preload,
  playback and seek, verifies helper ticket release and catches missing files.
  This does not establish a new native game boot or opcode/wait/save profile.
- Python 3.11 tooling passed 189 tests. Evidence is under target/verification:
  audio-worker-final-library-tests.log, audio-worker-final-engine-js-tests.log,
  audio-worker-workspace-check.log, audio-worker-python-tests.log,
  audio-buffered-failure-tests.log and audio-worker-api-tests.log.

Remaining boundaries:
- Native SceneState/channel source opening and AudioRequest decoding still
  retain synchronous boundaries; changing original native waits requires
  explicit pending events and continuations. Final mix/output, scratch/voice
  allocation/disposal and final task-pool joins remain synchronous/owner work.
  This stage is not a hard realtime mixer or all-engine async completion.
- Acoustic reflections, diffraction, near-field propagation and room reverb
  remain open. No GPU/device-output/Windows/Linux behavior or additional
  original VN compatibility is inferred from these offline results.

## Native channel audio readiness (ADR 0070, 2026-10-27)

Implementation:
- Core now declares deferred AudioAsset/PlayAsset and explicit preparation
  phases. HostServices begin_audio/audio_preparation/release_audio distinguish
  request admission, owner publication and actual voice completion. Core and
  compat retain their dependency-free/device-free boundaries.
- Shared ChannelAudio opens and probes capabilities, prepares source/intro,
  declared limits and exact startup/loop prefixes on bounded workers. It uses
  the existing standard/NWA decoder, source wrappers, BufferedSource and mixer.
  A native adapter owns no decoded PCM or private general audio algorithms.
- Mapped RealLive BGM/WAV commands retain the original pending VmEvent/PC until
  publication; AVG32 music/wave/SE/voice events retain the original portable
  event. Both install/continue native sound waits after readiness. Failure
  leaves later native instructions unacknowledged. VM parser/opcode source and
  previous unsupported original-save/profile behavior remain unchanged.
- Added OperationTable::insert_ready for explicit offline products and
  submit_deferred for bounded queued inputs. Full shared pools retain Pending
  requests and retry slot reservation on owner polling; cancellation before
  admission performs no I/O. Retained-result/memory limits still fail visibly.
- Channel preparation retains at most 16 receipts/client and 64/host. A separate
  512 MiB estimated channel budget follows worker/stream storage. Encoded inputs
  are bounded at 64 MiB per supplied memory segment and 512 MiB per streamed
  segment, subject to provider limits. Estimates do not account all provider,
  transient or codec memory and are not a process-memory sandbox.
- Stop/supersession prevent unpublished playback. Failed preparation preserves
  the old live voice. restart:false can reuse an active channel without asset
  I/O. Restore clears channel voices, queued PCM and receipts after successful
  state validation; late completions cannot revive prior runtime playback.
- Fixed stale stopped/ended channel handles blocking later owner publication.
  Added prepare_at_cancellable to prime exact nonzero startup positions, while
  retaining intro and loop caches. Source frame units remain explicit.
- Moved the existing common AssetIo cursor to crysirror-io (core + std, no
  codecs/devices). Resource retains its reexport; modern and native audio use
  decoder::open_asset over the same cursor, removing the engine-private wrapper.
- Regenerated docs/api and logged fingerprint 2d945b181a39e802 ->
  d36d62998490cf17, including the exhaustive AudioRequest enum migration.

Compilation:
- Local release workspace/all-target check passed with desktop
  arc-formats and acoustics-obvhs features. Existing CBG/RealLive warnings
  remain. Exact-file rustfmt and git diff checks passed.

Behavior validation:
- Shared I/O/task/audio/resource suites passed **79 tests**, with one owner NWA
  fixture ignored. Native/engine/JS bridge suites passed **38 tests**, with
  three font-dependent cases ignored. API documentation/signature checks
  passed **six tests**. Total: **123 distinct Rust tests passed**, four ignored
  cases excluded. The separately rerun six-case channel suite is not counted
  twice. Python 3.11 tooling passed **189 tests**.
- Synthetic capability readers record opening on workers. Gates verify pending
  preparation leaves owner frames free, including when every pool slot is
  reserved, without losing inputs. Tests verify cancellation before publication,
  foreign receipts, result bounds, no reopen with restart:false, newer request
  precedence, failed replacement, Stop invalidation and Ready receipts surviving
  actual voice completion. Runtime restore invalidates pending channel work.
- Authored PCM oracles verify intro/tail declared limits, decode/playback gain,
  nonzero exact startup and native loop offsets at sample points. The new
  cursor tests cover partial reads, end-relative seeks, underflow/overflow and
  invalid provider read sizes. Existing resource transform/reader tests pass
  through the preserved AssetIo reexport.
- Original RealLive bytecode and AVG32 TPC32 fixtures verify unchanged instruction
  counters while all I/O workers are blocked; following title/arithmetic state
  remains untouched. Once ready, playback and native sound waits complete.
  Invalid decoder input fails before following native instructions execute.
  These are original instruction/event paths, not VM aliases or hosted engines.
- Revalidated all 90 complete-reference RealLive Rust hashes at
  5f358ef1360b0439e92aed2b749e8f25d8721954 and inventoried 31 AVG32 Rust files at
  256c398e06fbcbae8e956eb046c831b3d6c8a593. Inspected native sound/event/wait
  branches at those pins. This source review is not blanket opcode coverage.
- Evidence in target/verification: channel-final-library-tests.log,
  channel-final-bridges-tests.log, channel-final-coordinate-tests.log,
  channel-api-tests.log, channel-final-workspace-check.log,
  channel-python-tests.log and channel-native-source-review.json.

Remaining boundaries:
- Native resource-name directory discovery, boot/scenario and other unmigrated
  image/video reads remain synchronous. SceneState/data-VM voice mirroring and
  encoded media AudioRequest submission retain synchronous opening. The legacy
  encoded submit API remains explicit; PlayAsset requires preparation.
- Final mix/output, allocation/disposal and final pool destruction still have
  owner/synchronous boundaries. No hard realtime, complete audio/engine async,
  additional native opcode/save/game profile or commercial game boot is claimed.
  No new device, GPU, hardware-RT or Windows/Linux behavior was measured.
- Near-field propagation, diffraction, acoustic reflections and room reverb
  remain open. These results advance the shared foundation without completing
  the broader modern 3D engine/VN integration objective.

## Canonical SceneState audio worker bridge (ADR 0071, 2026-10-27)

Implementation:
- Added device-free SceneAudio over a caller-owned Mixer. Engine App and plain
  desktop playback now use the same declaration/epoch reconciliation and worker
  preparation. Removed the engine-private canonical voice bridge and the plain
  desktop's live AudioSystem::sync/read_for path. The library's explicit
  synchronous compatibility API remains available and is documented as such.
- Reused native channel capability opening and source preparation, existing
  BufferedSource and deferred task admission. Canonical cold assets are opened,
  probed and primed off the realtime owner. No codec, mixer or general-purpose
  algorithm was added to a VM. Canonical and pooled App voices retain their
  existing shared Mixer, limits, buses and final PCM composition.
- Stops/replacements/restore reconcile before result publication. Published
  epochs remain distinct from desired epochs, so natural EOF/stolen handles do
  not automatically restart. Gain/routing preserve the timeline. Pause includes
  voices that finish preparation later. Failed preparation retains the prior
  voice, records the error and produces the existing visible frame failure.
- Added Audio.channelStatus, SceneVoicePhase and SceneVoiceStatus for scoped
  queued/loading/playing/paused/ended/failed state, position and failures.
  Streaming faults are visible while a valid predecoded prefix can still drain.
  Logical time waits remain separate from audio readiness; save/trace voice data
  contains no decoder, task or device handle.
- Preparation is limited to 16 jobs/client and 64/bridge. Additional declarations
  stay Queued and retry. A separate 512 MiB estimated bridge budget reserves
  4 MiB per realtime source and follows actual worker/source ownership through
  cancellation. These estimates are not comprehensive process-memory accounting.
  Offline playback retains same-frame preparation at the canonical 128-voice
  limit instead of inheriting realtime admission delays.
- Added Runtime audio execution override and effective mode inspection. Desktop
  Run enables audio workers before initialization, independently of native
  image/scenario Assets mode. Setter also configures direct channel calls before
  the first runtime tick. Explicit offline/test hosts inherit the Assets policy
  unless overridden. Cold modern SFX/music guards now use the audio policy.
- Regenerated docs/api and logged d36d62998490cf17 -> 102009774599dc55.
  ADR 0071 records initialization migration and remaining subsystem boundaries.

Compilation:
- Release workspace/all-target check passed with desktop  arc-formats and
  acoustics-obvhs features. The desktop no-default-features reallive+avg32 check
  also passed, without scripting or audio-device output enabled. Existing CBG,
  RealLive and minimal-CLI warnings remain.

Behavior validation:
- Audio library suites passed **47 tests**, one owner-asset NWA case ignored.
  Engine/native/JS bridge suites passed **41 tests**, three font-dependent cases
  ignored. API documentation/signature checks passed **six tests**. Total:
  **94 distinct Rust tests passed**, four ignored cases excluded. Separate
  focused reruns are not counted again. Python 3.11 tooling passed **189 tests**.
- Six standalone SceneAudio tests verify full-pool pressure, cancellation before
  admission/publication, progress of unrelated voices, same-asset replay and
  superseded results, gain/bus changes without epoch reset, restore at frame
  zero, EOF without implicit replay, failure with old playback retained, pause
  and all 128 offline declarations in one sync. Excess realtime declarations
  eventually publish in bounded batches. Reader probes remain off the owner.
- Authored NWA data with an injected read fault proves streaming status records
  the error while already-valid PCM remains playable. No owner game/font asset
  or audio device was used for this case.
- Engine tests execute data VM Play/Wait/Stop while workers are blocked, check
  Stop-before-ready cannot leak playback, verify two client roots/readiness,
  inspect visible failed channel state, and reconstruct logical voices after
  restore without saved handles. Existing pooled/canonical output and replay
  regressions pass through the new bridge.
- Original RealLive/AVG32 bytecode fixtures now explicitly keep native Assets
  mode blocking while overriding audio async. Their original pending events,
  sound waits and failure checks pass with that independent mode. Direct native
  channel preparation before the first tick is also covered by restore tests.
- Corrected two test timing assumptions: canceled worker inputs retain storage
  until actually dropped, and an asynchronous onInit rejection may be observed
  at init, start or frame. The tests retain their reservation/fault assertions;
  production memory limits and error contracts were not weakened.
- Evidence in target/verification: scene-audio-final-library-tests.log,
  scene-audio-final-bridge-tests.log, scene-audio-final-api-tests.log,
  scene-audio-core-tests.log, scene-audio-workspace-check.log,
  scene-audio-minimal-check.log and scene-audio-python-tests.log.

Remaining boundaries:
- Final mixing, scratch/allocation/disposal, device output/queuing and final pool
  destruction remain owner/synchronous work. Media's encoded AudioRequest path
  still opens/decodes on the owner. No all-audio/all-engine async completion or
  hard realtime guarantee is claimed by this stage.
- Native directory discovery, boot/scenario and unmigrated image/video work
  retain their existing boundaries. No new original opcode/save/game profile,
  commercial-game run, device/GPU/hardware-RT or Windows/Linux result is claimed.
- Reflection/reverb, diffraction and near-field propagation remain acoustic
  work; the larger modern engine/VN capability objective remains active.

## Worker-prepared media PCM (ADR 0072, 2026-10-27)

Implementation:
- Added dependency-free immutable PcmAudio/PcmAudioInfo and AudioRequest::PlayPcm
  in core, plus shared audio PcmBuilder and resident-source playback. These work
  without any native VM, movie service or output device. Prepared publication
  performs no codec opening or sample validation scan. Core retains no device
  dependency; resources/receipts are derived and excluded from logical saves.
- Media workers now prepare PCM directly instead of encoding WAVE for the owner
  to reopen/decode. Conversion preserves i16 /32768, mono duplication and the
  existing first-two-channel projection. This is not surround downmix support.
  Start offsets, sample-rate conversion, loop points, gain, pan and client scope
  use the existing shared mixer. Movie PTS lead/delay and late visits retain
  their clock/skipping policy.
- Moved media capability opening and length queries off the owner. Audio/video
  share one lazy scoped capability with independent AssetIo cursors. Missing
  assets now produce asynchronous Loading -> Failed errors; invalid authority,
  options and worker admission still reject immediately.
- Added 16 live/starting decoder-worker reservations. Blocked canceled workers
  retain admission until they actually exit; replacement rejects before changing
  the current slot if no capacity remains. Loop-restart admission failures mark
  Failed and retain emitted Stop controls for the next owner audio drain.
- PCM sample-budget receipts follow actual queued/mixer references past resource
  Stop. Staging and final samples are reserved together during Arc conversion.
  The 128 MiB budget excludes allocator spare capacity, codec/provider buffers
  and other process overhead. Inspection adds decoder_worker_reservations.
- Generated API metadata now includes PcmAudioInfo; fingerprint changes from
  102009774599dc55 to f55ff4c1819aaa78. The change log records the Rust enum
  extension and asynchronous media opening errors. No JS sample-upload API was
  added and no native opcode or renderer implementation moved into an adapter.

Compilation:
- Release audio/runtime checks and the final workspace/all-target check passed
  with desktop  arc-formats and acoustics-obvhs enabled. Existing CBG and
  RealLive warnings remain. No new device/GPU validation ran.

Behavior validation:
- Audio suites passed **51 tests**, one unrelated owner-NWA test ignored. Media
  regression tests passed **11 tests**; its local-file case was explicitly run
  separately and passed. API signature/reference checks passed **six tests**.
  Total: **69 distinct Rust tests passed**. The explicitly executed movie is not
  counted again from the earlier full-media run. Python 3.11: **189 passed**.
- Four independent PCM tests verify invalid resources, signed extrema, mono and
  multichannel projection, partial/oversized append atomicity, shared storage
  lifetime, exact seek/EOF beyond the SFX Clip ceiling, sample-level start/rate/
  loop/gain/pan controls, foreign-client Stop isolation and final voice release.
- Media probes block opening while the owner continues ticking, assert open/len/
  read occur off-owner and both decoders share one capability, retain admission
  across canceled blocked providers, preserve the current slot on failed
  replacement and release leases after actual worker exit. Queued/live PCM keeps
  its sample reservation after resource Stop. Missing-open faults are visible;
  synthetic PTS cases verify early audio, delayed audio and late-visit sample
  skipping. Loop admission failure retains the completed voice's Stop.
- Explicit owner-supplied RUSUR.MPG run reached Ended without error: **2,995
  decoded frames / 2,987 published updates**, one PCM resource with **4,402,944
  stereo frames at 44,100 Hz**, **35,223,552 sample bytes**, nonzero shared mixer
  output (peak 0.99996948), and unchanged source size/mtime. This is CPU media/
  offline-mixer evidence without any native VM, GPU or audio output device;
  published updates are not a guarantee every source picture was presented.
- Evidence: target/verification/pcm-audio-tests.log, pcm-media-final-tests.log,
  pcm-media-regression-tests.log, pcm-api-final-tests.log,
  pcm-workspace-final-check.log and pcm-python-tests.log.

Remaining boundaries:
- Movie audio still decodes a bounded complete track. Long-track streaming,
  codec/provider/transient memory accounting and live device behavior remain
  open. Final mix/output, control/scratch allocation, last-reference disposal
  and final worker-pool destruction retain synchronous boundaries.
- Native boot/discovery/images, original save/opcode coverage, other-platform
  behavior, acoustic reflections/reverb and complete engine-wide async execution
  are not completed by this phase. No additional whole-game compatibility or
  hardware RT result is claimed; the broader engine objective remains active.

## Shared parametric late-reverb buses (ADR 0073, 2026-10-27)

Implementation:
- Integrated standalone FunDSP 0.23.0's 32-channel FDN behind engine-owned
  LateReverb/PreparedReverb. Original source revision is
  8479a70d813d539c67a908f88e65f6b0b344122e. Audited the complete reference checkout
  inventory (70 Rust files); all 52 published registry sources match after CRLF
  normalization, with four raw newline differences and both hash sets recorded.
  Reverb/feedback/delay/FIR/allocation hooks were inspected. Cargo pins the crate
  and checksum; upstream MIT/Apache terms remain intact. Optional file loading
  and FFT convolution are disabled. No other engine or mixer was imported.
- Shared ReverbSpec/ReverbSettings/ReverbSend provide authored room scale,
  approximate decay, damping, source-bus sends and wet-return routing. Sends are
  post source fader/duck/spatial/direct/HRTF and pre-Master. Wet uses its return
  bus and Master once, without feeding itself again. Dry output remains present.
  Tails survive source EOF/Stop/stealing; clear/replacement cuts old history.
- Added Audio beginReverb/reverbStatus/cancelReverbJob/clearReverb/reverbSettings
  and prepareReverbAsync. Workers construct/preallocate matched DSP objects for
  ordinary/canonical and native/media mixers; owner publication rolls back the
  first install if the second fails. Both call the same engine DSP/bus APIs.
  Native adapter implementations/opcodes are unchanged; only test dependencies
  and fixtures add shared effect configuration and output assertions.
- Deferred worker admission, cancellation/supersession, scoped tickets and
  restore invalidation reuse OperationTable. A 64 MiB estimated pair reservation
  follows actual worker/DSP owners against 512 MiB/host; canceled work does not
  release its receipt early. Tables retain at most 8 jobs/client and 32/host.
  Published receipt release keeps installed DSP. Effects/history are derived,
  never serialized; Mixer::clear removes them after restore/reset.
- Added source audit/attribution, ADR 0073, async/API usage notes and a standalone
  room_reverb example. Generated docs/api fingerprint changed from
  f55ff4c1819aaa78 to 98b40f5f4c7c26d0 and is logged in CHANGES.md.

Compilation:
- Final local release workspace/all-target check passed with desktop
  arc-formats and acoustics-obvhs. Desktop no-default-features reallive+avg32
  check also passed without scripting or device output. Existing CBG/RealLive
  and minimal-CLI warnings remain.

Behavior validation:
- Acoustics/audio suites with obvhs passed **68 tests**, two owner-resource cases
  ignored. Engine reverb passed **4**, JS async services **7** (one font case
  ignored), original RealLive/AVG32 bridge suites **12** (one font case ignored),
  and API reference/signature checks **6**. Total: **97 distinct Rust tests
  passed**, four ignored excluded. Focused reruns are not counted twice.
  Python 3.11 tooling: **189 passed**.
- Independent impulse tests verify source-EOF tails and decay, sample-identical
  arbitrary block boundaries, Master linear scaling without squaring, input
  mute, wet-return mute with dry preserved, client isolation, native PCM channel
  tails, invalid formats/routes and actual storage release after Mixer::clear.
- Engine tests verify both mixer publications, native PCM tail in App output,
  rollback preserving the old first-mixer room when the second cannot publish,
  foreign ticket access rejection, full-pool owner progress, cancel/supersede
  before publication, invalid replacement preserving old settings, ticket release
  preserving installed DSP and restore removing pending/installed histories.
  JS/TS executes prepareReverbAsync, checks publication and receipt release,
  catches invalid settings and clears the client room.
- Original RealLive/AVG32 sound fixtures retain their original source-completion
  waits and instruction/arithmetic boundaries with reverb enabled. Subsequent
  native instructions execute while a nonzero shared bus tail remains. Worker
  failure tests also pass. This is original-bytecode/service-boundary evidence,
  not additional opcode, native-save or commercial-game compatibility.
- Authored six-second dry/room/hall demo rendered stereo 48 kHz WAVs without
  assets/devices. Mix wall times: dry 0.006363 s, room 0.031424 s, hall 0.032521 s;
  post-source tail energy: 0, 0.01223493, 0.51438753; all peaks < 0.578. This is a
  single-effect offline CPU sample on this host, not full-engine performance or
  audio-device latency. Output stays ignored under target/verification/reverb-demo.
- Evidence: reverb-library-tests.log, reverb-engine-final-tests.log,
  reverb-js-tests.log, reverb-native-tests.log, reverb-api-final-tests.log,
  reverb-workspace-final-check.log, reverb-minimal-check.log,
  reverb-python-tests.log and reverb-demo.log in target/verification.

Remaining boundaries:
- Geometric early/specular/diffuse reflections, material-dependent IRs, physical
  room/RT60 inference, diffraction/pathing and Ambisonic return rendering remain
  open. The late FDN consumes authored parameters, not BVH materials. Its decay
  is approximate; sends follow existing direct processing rather than a separate
  physically simulated unoccluded indirect source path.
- Idle installed effects continue processing; replacement/clear does not crossfade
  or preserve old tails. Final DSP/mixing, control/scratch allocation, old-effect
  disposal, device queues and pool teardown remain synchronous boundaries.
  No complete engine/audio async migration, new GPU/hardware RT or Windows/Linux
  validation is claimed. The larger modern engine/VN objective remains active.

## Captured geometric early reflections (ADR 0074, 2026-10-27)

Implementation:
- Added engine-owned finite-triangle, first-order image-source queries over the
  existing pinned Parry/optional obvhs scene. Mirror candidates use Parry's
  primitive intersection; both path legs use the shared BVH and direct-path
  per-crossing amplitude transmission kernel. No legacy engine/renderer or new
  handwritten BVH is imported. Scientific/source boundaries are in ADR 0074;
  the pinned phonon_rs reverb parameter placeholder is not presented as a solver.
- Absorption, transmission and scattering now determine specular pressure via
  sqrt(max(0,1-absorption-transmission²)*(1-scattering)), with air/range/source
  directivity and both legs' transmission. Shared-edge duplicates are removed;
  finite surfaces, backfaces, units and ignored meshes retain explicit policies.
  Whole-scene triangle, audible-path and crossing overflow fail visibly.
- Added worker-prepared EarlyReflections with sparse linear fractional delays,
  equal-power stereo arrivals and shared complementary-band history. No geometry
  or ring allocation runs in sample processing. Seek uses a constant-time filled-
  history reset. Natural EOF drains echoes; explicit stop/steal/clear cuts history.
  Excess delay aligns to the existing dry clock and prepared direct HRTF bridge.
  Echoes receive raw pre-direct-filter input, so direct occlusion alone does not
  erase a separately visible indirect path.
- Added scoped engine reflection probe/voice/status/cancel/clear/inspection APIs,
  traceReflectionsAsync and prepareVoiceReflectionsAsync. Work uses immutable
  scene/pose snapshots and bounded deferred CPU jobs. Initial preparation holds
  only that source's clock. Obsolete pose/scene/latency work fails, releases the
  hold and cannot publish; failed replacement keeps old valid paths. These are
  captured-pose resources: automatic tracking is not implemented in this phase.
- Added OperationTable::fail for observable owner/generation-scoped invalidation
  without starting queued work. Worker/DSP leases retain actual ownership. Voice
  staging reserves 8 MiB, then shrinks to ring/tap/path storage plus 8 KiB against
  a 512 MiB estimated host budget. Jobs retain 16/client and 64/host. Restore
  invalidates derived effects/history/tickets without serializing pointers.
- Added Mixer::source_is_playing and Audio.voice.source_playing. Modern playing
  includes processing tails; native channel waits use source completion. Shared
  Rust channel/mixer effect APIs work without App/VM/device. Native adapter
  implementations/opcodes/save formats are unchanged; bridge tests configure the
  same engine resources rather than adding private native algorithms.
- Generated docs/api changed 98b40f5f4c7c26d0 -> 079531ae354ff224; CHANGES records
  APIs, snapshot semantics, source/tail distinction and initial paused-play recipe.

Compilation:
- Local release workspace/all-target check passed with desktop  arc-formats
  and acoustics-obvhs; desktop no-default-features reallive+avg32 also passed
  without scripting or device output. Existing CBG/RealLive/minimal-CLI warnings
  remain.

Behavior validation:
- Acoustics/audio/tasks suites with obvhs passed **91 tests** (two owner-resource
  cases ignored); engine acoustics/HRTF/reverb/reflections suites **20**; JS async
  services **8** (one font case ignored); original RealLive/AVG32 bridges **12**
  (one font case ignored); API reference checks **6**. Total: **137 distinct Rust
  tests passed**, four ignored excluded. Focused reruns are not counted twice.
  Python 3.11 tooling: **189 passed**.
- Six geometry tests cover finite-wall analytic points, 1/r pressure, absolute/
  excess delays, shared-edge/backface identity on both BVHs, both-leg occlusion,
  air/units/dipole/ignore policies, per-band transmission and crossing limits,
  cancellation/overflow, and an opaque direct-path blocker with a visible echo.
- Five sample-level tests verify fractional-delay/pan oracles, reset without old
  history leakage, actual resource lifetime, pre-direct echo routing, source EOF
  with remaining echo, native source completion, initial hold/cancel, pose/seek
  invalidation and direct HRTF bridge alignment. Task tests prove invalidated
  queued work never starts and failed receipts preserve scopes/errors.
- Engine tests verify real isolated echo samples, shared scene queries, full-pool
  progress of other voices, scoped/superseded/canceled work, pose/scene failure,
  old-path preservation, restore invalidation and **64 installed voice effects
  plus successful replacement** after staging reservations shrink. JS uses both
  Promise helpers, checks actual receipt release, rejects incomplete queries,
  clears/reprepares a paused short source, then starts it.
- Original RealLive/AVG32 audio fixtures now install real geometric echo resources
  through shared channel APIs. At original source EOF their next instructions/
  arithmetic execute while the mixer still retains an active echo voice. Worker
  failure/PC-boundary tests also pass. This is bounded native bytecode/service
  evidence, not additional native opcode/save or commercial-game coverage.
- Apple M5 dense-grid oracle: **262,088 triangles**, ten checked source positions
  per backend. Parry build 0.027020 s / mean query 2.579733 ms; obvhs build
  0.026347 s / mean query 2.530567 ms. Candidate enumeration dominates this flat
  workload; these close timings do not establish a backend advantage or engine
  frame-rate ranking. All ten analytic path/pressure/delay checks passed each.
- Authored dry/hard-wall/absorber/blocked WAV demo: hard-wall excess delay
  **0.069970846 s**, post-source energy **2.638427589**; absorber energy
  **0.026384251** (about 1/100); dry/blocked post-source energy zero. All peaks
  0.528611. One-second mixing: 0.002065–0.002616 s on this host. No game data,
  measured HRIR, font, GPU or audio output device was used. WAVs remain ignored.
- Evidence in target/verification: reflection-library-final-tests.log,
  reflection-engine-final-tests.log, reflection-native-final-tests.log,
  reflection-js-final-tests.log, reflection-api-final-tests.log,
  reflection-workspace-check.log, reflection-minimal-check.log,
  reflection-python-tests.log, reflection-large-scene.log and
  reflection-demo-final.log.

Remaining boundaries:
- Automatic pose refresh, dynamic acoustic geometry/refit, higher-order/diffuse
  paths, diffraction, Doppler, reflection HRIR/Ambisonics, physical room/RT60
  inference and seamless tap transitions remain open. Echo DSP uses linear
  fractional delay/one-pole bands, not a bandlimited wave solver. Its 10 ms
  post-delay band-history horizon truncates tiny remaining IIR residue.
- Final DSP/mixing, control/scratch allocation, last-reference disposal and device/
  pool teardown retain owner/synchronous boundaries. No full engine-wide async
  completion, new GPU/hardware RT or Windows/Linux behavior is claimed. The
  broader modern engine/VN capability objective remains active.

## Automatic reflection tracking (ADR 0075, 2026-10-27)

Implementation:
- Added engine-owned ReflectionTrackingSpec and Audio.trackVoiceReflections,
  plus playback `reflections` registration before source consumption. Updates
  run after listener/camera and entity-follow poses. Initial short sounds wait
  for enabled HRTF then reflection preparation. Static captured APIs retain
  explicit invalidation/re-preparation semantics; mode/settings replacement
  clears history, while ordinary pose updates preserve it.
- Split immutable worker-prepared ReflectionPlan from mixer-owned EarlyReflections
  history. Tracked effects reserve configured maximum delay once and record even
  empty plans. At most two plans render with sample-bounded linear crossfades.
  Completed previous plans/leases are reclaimed by the control owner, not in the
  sample loop. Seek resets filled history without stale samples. Shared Rust
  Mixer/ChannelAudio publication works without App, VM, script or device.
- Coalesced immutable source/listener/revision/latency snapshots use round-robin
  admission: 64 configured voices, 32 tracked flights globally, eight/client,
  four launches/frame and one flight or ready result/voice. The cap is checked
  before advancing the queue, covering the five-voice starvation boundary.
  Full pools defer; residency exhaustion records waiting with interval retry.
  Initial effects reserve 8 MiB, updates 64 KiB, shrinking to prepared capacities
  plus 8 KiB against the existing 512 MiB estimated host reflection budget.
  Actual workers, results and installed/retired objects retain their leases.
- Publication accepts a completed captured pose behind continuous movement,
  then schedules the coalesced desired pose; scene revision and direct HRTF
  latency remain strict. Incompatible old taps mute immediately and compatible
  replacements fade from silence using retained input history. Worker failure
  releases initial holds and retains a still-valid old effect. Clear, stop and
  restore prevent stale publication. Source EOF stops new geometry jobs and
  drains the installed target; native waits still use source completion.
- reflectionsVoice exposes tracking settings, desired/submitted/published
  snapshots, capture time/published age, pending/waiting/error, muted/draining
  and successful update count. Targets may lag under contention; publication
  is separate from audible fade completion. API docs regenerated and logged
  079531ae354ff224 -> 1ff0b7ce813fb81e. No legacy engine or VM-private algorithm
  was imported; original native adapter implementations/opcodes are unchanged.

Compilation:
- Local release workspace/all-target check passed with desktop  arc-formats
  and acoustics-obvhs. Minimal desktop no-default reallive+avg32 check passed.
  Existing CBG/RealLive/minimal-CLI warnings remain; no new warning was observed.

Behavior validation:
- Acoustics/audio/tasks with obvhs: **95 passed**, two ignored. Engine audio,
  acoustics/HRTF/reverb/static/tracked reflections: **41 passed**. JS async
  services: **8 passed**, one font case ignored. Original RealLive/AVG32 bridges:
  **12 passed**, one font case ignored. API reference: **6 passed**. Total:
  **162 distinct Rust tests passed**, four ignored excluded; the final five
  tracked-engine reruns are not counted twice. Python 3.11: **189 passed**.
- Four new device-free DSP/channel cases verify past samples survive changed
  delays, initially empty plans retain history, exact fade samples, busy-plan
  rejection, owner-side retirement/receipt lifetime, obsolete-geometry silence,
  seek isolation/capacity rejection, scoped channel publication and source EOF
  independent of retained echoes.
- Five engine cases verify continuous moving-source publication, listener
  orientation, blocked workers retaining audible old history, client scopes,
  clear/restore cancellation, scene replacement mute/recovery, both five-voice
  and twelve-voice fair admission, and a **one-frame sound** with HRTF-before-
  reflections dependency. Its real isolated echo reaches the output after
  source EOF. The same playback case verifies entity following, camera listener
  movement and entity destruction. Latest tracked-engine rerun passed after
  capture-age/draining-status additions.
- JS now exercises playSfxAsync with reflection settings, the explicit tracking
  method, listener movement, observed worker publication and resource clear.
  The initial JS failure was a fixture missing existing required listener
  forward/up fields; the corrected fixture passed. API blessing initially
  required logging the new fingerprint; the logged final reference test passed.
- Authored moving_reflections example exports 48 kHz stereo, three-second
  stationary/moving wet-only WAVs. Moving output uses **40 plan updates**, peak
  **0.121842**, versus stationary **0.127668**; squared PCM difference
  **71.441991051**. Worker wall time (including offline fences) 0.000328 s for
  moving plans; three-second mixing 0.005390 s on this Apple M5 run. These tiny
  authored-scene timings do not rank backends or establish live-scene throughput.
  An independent exported-PCM check verifies left-dominant early moving energy
  (4.388710 / 3.508156), right-dominant late energy (2.249148 / 2.863898), and
  a silent final half-second. Stationary output stays left-dominant. No game
  data, measured HRIR, font, GPU or audio output device was used.
- Evidence under target/verification: tracking-library-tests.log,
  tracking-engine-final-tests.log, tracking-native-tests.log,
  tracking-js-tests.log, tracking-api-final-tests.log,
  tracking-workspace-check.log, tracking-minimal-check.log,
  tracking-python-tests.log, tracking-demo.log, tracking-demo-oracle.log and
  moving-reflection-demo/*.wav. Generated WAVs/evidence remain ignored.

Remaining boundaries:
- Geometry remains baked/static; no mesh refit, higher/diffuse bounce,
  diffraction, reflection HRIR/Ambisonics, continuous Doppler or physical waves
  already in flight. Discrete captured delays crossfade; they are not continuous
  physical resampling. Source directivity stays authored in world space rather
  than implicitly following entity rotation. Explicit reconfiguration cuts tails.
- Final DSP/mixing, control/scratch allocation, last-reference disposal and device/
  pool teardown retain synchronous owner boundaries. No complete engine-wide
  async migration, new GPU/hardware RT, other-platform/audio-device behavior or
  additional commercial VN opcode/save compatibility is claimed. The broader
  modern engine/VN capability objective remains active.

## 2026-10-27 — shared mix groups, playback age and original PCM/global SE (ADR 0078)

Implementation:
- Added engine-owned client-scoped named gain groups, independent of any VM.
  Core supplies plain AudioMixGroupSettings/Info and SetMixGroup/SetMixGroupGain
  requests, without dependencies. VoiceSpec/AudioParameters gains optional
  mix_group; publication resolves a dense index in the owning mixer's namespace.
- Candidate graph validation checks parent authority, cycles, descendant depth,
  gains/fades/names and limits (64 groups/client, 256/mixer, depth eight) before
  mutation. Parent-before-child evaluation performs no name search/allocation
  per output frame. Removal rejects children and live source/effect references;
  pending playback revalidates its logical name at publication. Leaf slots reuse
  safely without moving indices referenced by another client/voice.
- Extracted the existing shared output-frame Ramp into one internal module.
  Group fades progress during silence, pauses, dependency holds and starvation;
  independent source positions/envelopes remain separate. Group/ancestor gains
  apply once before existing reverb sends/Master. Previously committed late wet
  history stays on the return bus. Dialogue duck admission evaluates group level
  per sample and remains invariant across block partitions.
- Public Audio setMixGroup/mixGroup/nativeMixGroup/removeMixGroup/
  setVoiceMixGroup reaches pooled and native/media control graphs. Both validate
  before either changes. Partial fields retain values; parent:null detaches,
  while null/nonfinite gain, null mute and unknown fields fail. Gain-only native
  updates preserve host parent/mute. Host group controls persist through playback
  clear/restore; they are not automatically serialized VM/project DSP graphs.
- AudioPlaybackInfo.age_frames is a shared output-frame publication age, distinct
  from decoded position/read-ahead. Pause, holds, starvation and effects count;
  seek does not reset age. Native PCM/SE resource preparation measures unknown
  lengths, then independent voice preparation captures a lease. Temporary lookup
  handles release at admission; workers/live instances retain actual encoded bytes.
- Canonical resources aliases are mandatory. Decoded Gameexe numbered SE uses
  native count default16/min8/max256; SE voice numbers use explicit CrySirror
  aliases koe/{number}, without guessing native packs/filenames. Unknown forms,
  missing entries/aliases and property fallbacks fail visibly. BGM/PCM/SE pending
  operations carry ephemeral ordering stamps so original import stops PCM/SE
  before a later BGM preparation can block; no stamp/ticket enters saved data.

Compilation:
- Final release workspace/all-target check passed with desktop  arc-formats
  and acoustics-obvhs; minimal no-default desktop reallive+avg32 check passed.
  Existing CBG/RealLive/minimal-CLI warnings remain; new fixture warning was fixed.
  No output device, GPU, Windows/Linux or
  console validation ran. Normal VM dependency audit is in the local evidence.

Behavior validation:
- Final local release scopes: shared audio/core/compat/tasks **115 passed**;
  joint Runtime suites **114 passed**; engine audio/acoustics/HRTF/reflection/reverb and
  API reference **54 passed**; RealLive/AVG32 bridges **12 passed**; JS async
  services **10 passed**. **305 distinct Rust passed**, four ignored excluded.
  Focused/earlier runs are not counted twice. Python 3.11 tooling: **189 passed**.
- Six independent group tests assert hierarchy/client routing, live re-routing
  without seek, bus/Master scaling, atomic graph/cycle/depth/capacity failure,
  silence/pause fade clocks, sample-boundary duck release and block partition
  invariance. Native gain requests preserve host parent/mute. A grouped impulse
  matches independent voice-gain dry/late-reverb PCM; committed tail survives
  group mute/removal. Transport assertions separate starved/paused source position
  from output publication age. Public API tests cover both mixers, invalid/null
  changes, parent detach/partial retention and second-mixer admission rollback.
- **20 authored original SceneVm audio cases** pass. New cases exercise native
  PCM/SE independent fade samples, file/SE-number/explicit voice-number playback,
  true worker gates and pending snapshots, failure preserving the old source and
  original PC followed by repair/retry, active rebind, native zero-return key wait,
  original file volume/stop behavior and cross-category stop ordering while BGM
  workers are blocked. A whole-ms wait releases while physical PCM is still live.
  Existing BGM/source/effect/save/unknown-op coverage remains passing.
- Ran standalone mix_groups example in release, producing a three-second stereo
  48 kHz WAV from authored tones. Parent music fades leave the UI stem unchanged;
  Rust asserts its samples and restored level. Clear/muted/restored RMS:
  **0.158114 / 0.070711 / 0.158114**, peak **0.286419**. No game data/fonts/device.

Remaining boundaries:
- These are gain/control hierarchies, not full Sound Class/submix effect graphs,
  group concurrency or virtualization. PCMCH/PCM events, general KOE speech/
  JITAN/mouth tables, native UI sound slots, automatic resource/append discovery,
  encrypted Gameexe, all save variants and whole-game compatibility remain open.
  Native polling follows the engine's current VM-before-mix frame phase and is
  not an exact wall-clock/device timing match under starvation. Halted snapshots
  can retain the last observed native declaration. Final async DSP/mixing/output,
  dynamic geometry/Doppler and higher/diffuse acoustics remain active goals.

## 2026-10-27 — worker final DSP frames and desktop dependencies (ADR 0079)

Implementation:
- Added shared CPU MixJob/MixFrame over the bounded TaskPool. A worker owns the
  actual mixer/decoder/effect histories for a frame; owner slots retain scoped
  readonly observations. Completion validates all leases, restores DSP state and
  publishes combined unclipped PCM once. This works without a VM/device/window.
- Jobs admit 1–8 same-rate mixers and at most two seconds per frame, with per-mixer
  counts and 4096-frame cancellation blocks. Prepared frames retain PCM/counts
  while capacity is full. Ordinary controls fail during a lease; infallible
  client pause/stop applies at publication. Reset/restore invalidates old tokens;
  failures/panics release playback slots and publish no failed PCM.
- Runtime.begin_tick/poll_audio_frame and App.begin_frame/poll_frame expose the
  dependency. Callbacks, VM/world time and accepted input execute once. App joins
  pooled/canonical and native/media audio in one worker product; engine frame
  completion/input commit follows publication. Pause defers, shutdown cancels,
  and validated restore discards obsolete PCM. Synchronous frame/tick remain
  explicit offline/reference paths; tick_control requires an external coordinator.
- Plain and scripted desktop now use worker final DSP. The pending-frame gate
  precedes elapsed-time/gamepad/input-edge acquisition. CPU mixing remains
  available with the optional output device disabled; muted checks open no device.
- Audio.rendering includes prepared capacity waits and running work. Playback
  and group observations remain at the last publication. Prepared playback
  receipts cannot report ready before owner publication during a DSP lease.
- Extended the authored gain-group demo with --worker for sample and exported WAV
  comparison. Added independent worker/sample/transport/failure/reverb-history
  oracles, App lifecycle/backpressure/restore tests, desktop driver tests and an
  original SceneVm WAIT/restore test. No new VM hooks or legacy DSP were imported.

Compilation:
- Release workspace/all-target check passed with desktop  arc-formats and
  acoustics-obvhs. Minimal no-default desktop reallive+avg32 check also passed.
  Existing CBG/RealLive/minimal-CLI warnings remain. API reference regenerated;
  signature fingerprint f2af02453b2f74f8 -> bec05c019d833cb0 is logged in CHANGES.
-  No live window, GPU,
  output device, Windows/Linux or console behavior was tested by this phase.

Behavior validation:
- Final local release scopes: shared audio + full Runtime **192 passed**;
  engine App/audio/frame/API reference **42 passed**; JS async services **10
  passed**; desktop CPU driver/input/mix tests **6 passed**. **250 distinct Rust
  passed**, three ignored excluded. Earlier/focused reruns are not counted again.
  Python 3.11 tooling: **189 passed**.
- Eight shared worker cases assert actual source reads on a worker thread,
  synchronous-reference PCM, frozen queries/transport, rejected concurrent DSP
  mutations, capacity retries retaining PCM, independent producer frame counts,
  deferred client controls, stale generation/reset cancellation, source failures,
  task panic, queued cancellation and bounded validation. Multi-frame reverb
  history produces identical samples to a continuous synchronous render.
- Five App/frame cases cover callbacks/time/input once under backpressure,
  readonly Audio.rendering and native observations, paired PCM equivalence,
  deferred pause, restore/shutdown discarding old output, standalone Runtime
  completion and completion errors faulting the App. Both default desktop core
  variants enforce the gate before another tick and explicitly cancel at exit.
- All **21 authored original SceneVm audio cases** pass. The new case blocks CPU
  workers after native BGM preparation, verifies unchanged original VM diagnostic
  state and published age while DSP is pending, then observes publication and
  WAIT completion. Restore cancels the obsolete audio and rebinds resources via
  the same engine API; no native audio runtime or renderer is hosted.
- Full regression caught the new two-second admission check incorrectly rejecting
  a long idle synchronous tick. Fixed the no-audio path to retain its original
  contract. Runtime save rewind and the four-second VFX history test now pass;
  active audio/worker admission keeps its bounded profile.
- Ran the standalone mix_groups demo in synchronous and --worker release modes.
  Both authored assertions pass and exported WAV files are byte-identical:
  stereo 48 kHz, 16-bit, 144000 frames/three seconds, 576044 bytes; SHA-256
  a6cb3e72588260026d4b6a5789f47dcda27969d0f1694d01a236aa551849e0eb.
  Clear/muted/restored RMS 0.158114 / 0.070711 / 0.158114; peak 0.286419.
- Evidence: target/verification/mix-worker-{shared-runtime-final,engine-final,
  js-final,desktop-tests,workspace-check,minimal-check,api-bless,demo-reference,
  demo,python-final,diff-check}.log; mix-worker-validation.json,
  mix-worker-demo.json and mix-worker{,-reference}.wav. The earlier broad
  regression logs include the idle-tick failure; final scopes above pass.

Remaining boundaries:
- Final DSP is worker-owned but frame-dependent. Shared-pool contention can
  postpone simulation/render publication and starve device output. There is no
  dedicated device-clock renderer, sample-timestamped future command queue,
  allocation-free owner handoff or realtime throughput/latency guarantee.
- The existing two mixer instances retain independent duck/effect histories;
  final PCM addition does not create cross-mixer sidechains or a submix graph.
  PcmOutput still opens devices on submission and uses a mutex for its bounded
  queue/refill callback. Device lifecycle/loss, lock-free transfer and pool
  teardown/join remain work. No hardware performance claim follows from CPU tests.
- General native startup/I/O and VM scheduling, PCMCH/KOE, exact legacy timing,
  dynamic acoustic geometry, Doppler and diffuse/higher-order propagation remain
  open. This phase adds reusable scheduling and completion capabilities, not
  whole-game compatibility or completion of the engine-wide async requirement.

## 2026-10-27 — device worker, bounded PCM transfer and startup dependency (ADR 0080)

Implementation:
- Modern PcmOutput now owns a shared fixed-capacity stereo packet queue and an
  OutputBackend factory. A dedicated worker opens/builds/starts/destroys the
  platform stream on that same thread; it never borrows a VM/window/EngineContext.
  Streams need not be Send. submit admits work without device I/O or thread join
  on the caller. Opening/readiness, retained failure and actual worker-resource
  close completion are separate OutputWatch/OutputStatus observations.
- Crossbeam ArrayQueue 0.3.14 replaces the CrySirror Mutex<VecDeque>/512-sample
  refill. It stores at most 12000 complete stereo 48 kHz frames, keeps newest
  frames on overflow and records drops. Final clamp/nonfinite sanitization stays
  after engine mixing. Pause preserves source state; epochs/reset invalidate old
  queue/interpolation data. Callback transfer and its fixed UTF-8 error lane use
  no control mutex or growable buffer. This is lock-free, not wait-free or a
  claim about CPAL/OS internal locks, allocations or realtime deadlines.
- Native stereo 48 kHz bypasses rate conversion. Other device rates use fixed
  linear interpolation; mono averages L/R, channels 0/1 carry stereo and extra
  channels are silent. Misaligned callback buffers fail visibly without consuming
  input. CPAL sample conversion covers its integer/float formats. There is no
  bandlimited resampling or surround speaker-layout implementation in this bridge.
- begin_close and Drop request cancellation without joining the owner. Completion
  follows stream destructor/unwind return; cleanup panic remains a failure.
  Opening calls can only observe cancellation after returning. Explicit retry
  requires failed/closed completion and isolates a fresh session/queue/epoch.
  A callback failure retains its diagnostic even when close starts before cleanup.
- Desktop now enables cpal-output, directly creating a CPAL stream rather than a
  Rodio Sink. It polls failures without subsequent PCM, gates startup before time/
  gamepad/input acquisition while the device opens, excludes opening wall time
  from catch-up audio and requests close even if lifecycle shutdown fails. The
  explicit rodio-output compatibility feature still compiles its older synchronous
  AudioDevice API; default desktop does not use that endpoint.
- Independently checked current CPAL 0.18.2, but Cargo resolution rejected it:
  existing WMV CPAL 0.15/alsa-sys 0.3 conflicts with 0.18/alsa-sys 0.4 native links.
  The implemented backend pins existing CPAL 0.15.3. Source revisions/checksums
  are in docs/audio-output-source-audit.json. No upstream implementation, device
  code or private legacy mixer was copied into a native adapter.

Compilation:
- Release workspace/all-target check passed with desktop  arc-formats and
  acoustics-obvhs plus the optional rodio-output compatibility feature. Minimal
  no-default desktop reallive+avg32 check also passed. API docs regenerated;
  reflective fingerprint remains bec05c019d833cb0, with Rust host migration logged.
- Fixed a new unused test import and a probe example's missing local binding;
  final validation passes. Existing CBG/RealLive/minimal-CLI warnings remain.
   Windows/Linux/console/GPU
  behavior and a live desktop window were not tested in this phase.

Behavior validation:
- Final release scopes: shared audio including optional device/compatibility
  code **102 passed**; desktop CPU driver/input/output **8 passed** ; original RealLive/AVG32 bridges **12 passed**;
  engine frame/API reference **11 passed**. **154 distinct Rust passed**, two
  ignored excluded. Earlier/focused reruns are not counted again. Python 3.11
  tooling: **189 passed**.
- Eleven independent output cases exercise lazy silence, truly blocked worker
  open/Drop and gated destruction receipts, stereo overflow/order/clamping,
  pause/underrun, reset of cached interpolation, partial-frame rejection,
  continuous-signal 24/44.1/96 kHz oracles across callback partitions, mono/extra
  channels, failed-session retry, callback failure/open panic, concurrent reset/
  overflow and failure retention through close. Callback/error transfer also
  completes while both control mutexes are deliberately held. CPAL integer/full-
  scale/unsigned-silence conversion has a separate device-free test.
- A rate oracle initially omitted the lookahead input frame at its authored
  signal endpoint; supplying that input makes the continuous linear oracle
  valid. Its corrected sample and partition assertions pass. Desktop tests
  verify no second simulation tick during gated opening, failure polling without
  PCM, plain/scripted worker completion and shutdown cancellation.
- Ran the release output_probe against the local macOS CoreAudio device:
  stereo **48 kHz f32** opened on the worker and produced callbacks. Paused
  state rendered **558 device frames**, retained all **128 queued frames** and
  reported zero underruns. After reset/resume, another **128 MIN_POSITIVE-level
  stereo frames** were dequeued by the real callback, reaching **1115 rendered
  device frames**, then close completed with no error and an empty queue. The
  rendered counter remained unchanged for the following **20 ms** after close.
- This short burst intentionally exhausts its input and pads silence: its **429
  active underrun frames** are recorded, not reported as a streaming benchmark or
  performance pass. No listening-quality, capture-loopback, latency or sustained
  throughput assertion was made; test values were below audibility.

Remaining boundaries:
- Engine DSP still follows simulation frames/shared CPU admission; device callback
  execution is not an independent engine audio render clock. Timestamped future
  commands, clock drift correction, realtime scheduling/latency tuning,
  bandlimited output resampling, surround/ambisonics, cross-mixer sidechains and
  default-device monitoring remain open. Stream failure has explicit retry;
  automatic recovery is not implemented. OS open/close cancellation is cooperative.
- CPAL 0.18 migration requires resolving the existing video/compatibility native
  dependency graph first. The local CoreAudio result establishes neither other
  hardware/platform behavior nor a complete implementation of ADR 0050. Native
  opcode/save/timing/resource discovery gaps and broader acoustics remain active.

## 2026-10-27 — prepared voice compression and original global KOE (ADR 0081)

Implementation:
- Added plain core AudioResourceTransform and transformed-resource preparation,
  plus public AudioSourcePrepareSpec/AudioTransformSpec options for the existing
  beginPrepareSource / JS prepareSourceAsync API. Shared audio owns capability
  reads, explicit decoder layout, bounded finite PCM conversion, processing,
  WAVE serialization, memory charges, worker products and owner publication.
  Voices retain independent shared decoder leases. Percent 100 preserves the
  encoded source; frame_limit applies to decoded input before processing.
- Nonidentity processing admits at most 4194304 input frames, mono/stereo and
  8–192 kHz, reserving 128 MiB scratch inside the shared 512 MiB resource budget.
  Temporary output overlap is charged; final storage shrinks to actual WAVE
  bytes plus 8 KiB. Existing receipt/handle scopes, cancellation and restore
  invalidation remain. Processing runs in the bounded asset/preparation pool,
  not a separate CPU DAG stage or a realtime streaming time-stretch effect.
- Original replacement order stops the old voice before resource preparation,
  including failure/negative numbers. Readiness gates the original instruction
  commit. Failure is visible and repaired resources can retry. Native integer-ms
  completion uses shared output publication age; DECIDE releases a key wait
  without stopping the voice. Source completion excludes derived effect tails.

Compilation:
- Final release workspace/all-target check passed with desktop  arc-formats,
  acoustics-obvhs and optional rodio-output compatibility; minimal no-default
  desktop reallive+avg32 check also passed after the native read-flag guard.
- Generated API reference/JSON/TypeScript and logged fingerprint change from
  bec05c019d833cb0 to **24e44c371ca52eb7**. Final API reference tests pass.
  Existing CBG/RealLive/minimal CLI warnings remain. No font/game data or proprietary SDK was added.

Behavior validation:
- A temporary standalone harness executed the complete unmodified pinned JITAN
  converter and compared full PCM samples for **784 authored vectors**: four
  rates, two durations, mono/stereo, seven patterns and seven percentages. All
  matched after fixing an exclusive search-window boundary in the new profile.
  Permanent length/FNV-1a regression fixtures retain the independent results;
  the reference implementation/harness is not shipped in the engine.
- Worker-gated tests verify no early resource/playback publication, canceled work,
  strict transform options, client ownership, lease lifetime/file removal, exact
  transformed PCM hash, identity samples/encoded length, input-range semantics and measured output
  frames. Public Audio and real JS Promise calls exercise the same capability
  without a native VM or output device.
- Native Scene.pck tests additionally cover Voice bus PCM mute/unmute, JITAN
  config/cache replay after file deletion, shortened-source integer-ms wait,
  EXKOE named flags and preserved message identity, completed DECIDE versus
  unrelated direction input, replacement failure/repair, both save paths and
  repeated refusal of the newer unmapped read-flag ABI before any sound/resource.
  Early test assumptions about Space and anyhow's outer error text were fixed.
- The independent release voice_compression example exported authored 16-bit
  mono 48 kHz WAVE: **48000 input frames -> 24839 output frames** at 200 percent.
  Python's WAVE reader confirmed both headers/counts. No listening-quality,
  device, GPU, cross-platform or whole-game result is implied.
- Evidence: target/verification/koe-{audio-final,engine-final,js-final,native-final,
  workspace-check,minimal-check,api-final,demo,python-final}.log, koe-summary.json,
  koe-final-run.json and koe-reference-comparison-final.log. The initial API
  generation log retains the expected missing-fingerprint entry failure.

Remaining boundaries:

## 2026-10-27 — moving acoustic instances and World bindings (ADR 0082)

Implementation:
- Replaced the whole-scene flattened index with immutable per-mesh local BVHs
  and a Parry Qbvh world instance index. Local Parry/optional obvhs BVHs share
  across pose snapshots; unchanged meshes also share world-surface pages. Only
  changed surface pages and the top-level index rebuild on a pose commit. No
  handwritten BVH builder/refitter or Steam Audio SDK is added. Exact registry
  archive checksums, matching source-file hashes and inspected source scope are
  in docs/acoustic-instances-source-audit.json.
- Added AcousticMeshUpdate and immutable/cancellable instance commits for affine
  transforms and enabled flags. Queries retain normalized world-distance units
  through inverse transforms and nonuniform scale; winding normals include
  negative scale. Instance bounds use coordinate-scaled padding. Disabled meshes
  keep memory/stable triangle offsets and remain in complete-scene budgets.
- Public Audio.beginUpdateAcousticScene / updateAcousticsAsync gates explicit
  batches with scoped task receipts and owner publication. Unknown/duplicate IDs,
  empty/invalid batches, pending conflicting commits and manual edits to bound
  meshes fail visibly. Old probes keep immutable scenes; failed/canceled updates
  retain prior geometry. Local/world coordinates are now both bounded at +/-1e6.
- Audio.bindAcousticMesh / unbindAcousticMesh connects owned World entities via
  global matrix times explicit local offset, with optional inherited visibility.
  One automatic job/client coalesces latest poses at 50 ms minimum submissions;
  limits are 256 bindings/client, 1024/host. Missing entities disable geometry
  with diagnostics; invalid poses retain prior geometry. Unchanged failed poses
  do not spin/rebuild until changed or rebound. Unbind, successful full scene
  replacement, clear and restore cancel/remove derived binding work.
- Scene generation identifies full replacements; revision changes on every
  instance commit. Automatic direct and tracked reflection queries may publish
  completed older instance snapshots, expose geometry_stale/current_revision and
  then refresh. Full scene generations and HRTF latency retain strict invalidation.
  This avoids cancellation starvation during continued movement and preserves
  sample history/tap crossfades. Explicit static reflections remain revision exact.
- New geometry and automatic-binding stats expose actual reuse/work counts,
  desired/submitted/publication state, ages, pending/waiting/errors. Staging
  remains bounded by 128 MiB reservations / 512 MiB host budget; snapshot estimates
  include per-mesh and duplicated local/world storage. Shared allocations can be
  conservatively charged more than once, not reported as allocator measurements.
- Audio follow-Entity now uses the existing validated object resolver, preserving
  the object's actual client instead of reinterpreting a foreign ID locally.
  Native adapter opcodes/layouts and save schemas are unchanged; no new native
  spatial opcode or HostServices facade is claimed.

Compilation:
- Final release workspace/all-target check passed with desktop  arc-formats,
  acoustics-obvhs and optional rodio-output; minimal no-default desktop
  reallive+avg32 check passed. Generated API reference/JSON/TypeScript and logged
  fingerprint change **24e44c371ca52eb7 -> e73bda1334c12a7f**. Final API and command
  coverage checks pass. Existing CBG/RealLive/minimal CLI warnings remain.
- Initial compile corrected the BoundingVolume trait import and the registry's
  List type hint. Release-only validation was used;  No game, font, SOFA dataset or proprietary SDK bytes were added.

Behavior validation:
- Final release scopes: acoustic library/backend tests **23 passed** (including
  one distinct default-feature refusal case); engine acoustic/frame/async/API/
  reflection tests **43 passed**; JS async services **12 passed**. **78 distinct
  Rust passed**, three ignored excluded. Earlier/focused reruns are not counted
  again. Python 3.11 tooling: **189 passed**.
- Both local BVH backends pass analytic affine/nonuniform/negative-scale ray
  distance/normal cases, ignore/bounds/material behavior, immutable update errors,
  disabled reflection identity/budgets and existing direct/reflection tests.
  Unit tests assert shared Arc allocation identity for local BVHs and unchanged
  world pages; counters alone are not the evidence for reuse.
- Engine tests gate workers to verify no early commit, old captured probes,
  coalescing without resubmission starvation, parent transforms/visibility,
  stale-entity disable, unbind/cancel/restore and scope isolation. Invalid worker
  geometry retains the old scene/reservation, avoids identical retry spin and
  recovers after a valid pose change. A public obvhs scene retains that backend
  through an instance commit. Foreign Entity objects are rejected even when
  an entity with the same ID exists in the current client.
- Actual PCM goes from blocked silence to audible after geometry moves while
  the source age continues. Reflection tests remove a finite reflector, publish
  new paths and verify the output settles to silence after the tap transition.
  A first assertion incorrectly expected the crossfade prefix to be silent;
  the corrected oracle checks its settled suffix. Worker-gated direct/reflection
  tests require older instance snapshots to publish with geometry_stale=true,
  then converge to the current revision instead of discarding every completion.
- Real JS Promise/Entity facades prepare, update, probe and automatically follow
  World geometry. The authored moving_acoustic_door release demo exports stereo
  PCM16 **48 kHz / 72000 frames**, with closed/open/closed phase RMS approximately
  **0 / 0.1000 / 0** and independent reflected-path counts **0 / 1 / 0**. The WAV
  contains direct occlusion audio; the path counts separately verify echo
  visibility. Python WAVE decoding confirms PCM RMS **0 / 0.09998 / 0**. Geometry
  work runs on workers; no native VM or audio device is used.
- Evidence: target/verification/instances-{acoustics-final,engine-final,
  default-backend,workspace-check,minimal-check,api-final,door-demo,python-final,
  diff-check}.log, instances-final-run.json, instances-summary.json and
  moving-acoustic-door.wav. Initial API generation retains the expected
  missing-fingerprint failure.

Remaining boundaries:
- This is pose/enabled updates of fixed topology. It does not implement vertex
  deformation/skinning, cross-ID prototype sharing, TLAS refit/quality maintenance,
  automatic model/collider acoustic extraction or material mutation. Automatic
  geometry/result age has no deadline or extrapolation guarantee; invalid poses
  and capacity pressure remain observable.
- Doppler, diffraction/diffuse/higher-order sound, independent device clock,
  native spatial mappings and wider async/3D/VN capabilities remain open. No
  listening-quality, performance ranking, GPU/hardware-RT, cross-platform or
  whole-game result is implied by this CPU release validation.

## 2026-10-27 — Shared transport and original PCMCH (ADR 0083)

Implementation:
- Added shared AudioPlaybackInfo transport_frames, stopping and pausing. Running
  input intent excludes fully paused/freezing-delay time and effect tails while
  counting decoder/seek/effect dependency holds. Playback position and publication
  age remain separate. Unknown EOF can count its discovery frame.
- Shared AudioResumeSettings / ResumeWithOptions preserve an ongoing pause fade
  until an output-clock deadline, or retain the existing freezing policy.
  reset_gain restarts a gain fade toward the latest requested target at that
  deadline instead of multiplying an unfinished ramp. Stop deadlines are now
  independent of later pause envelopes and remain observable through short input
  EOF. Input EOF cancels pending resume even when HRTF/echo state is retained;
  a new resume on already-ended input leaves its tail alone. All processing and
  observations use the existing worker-owned DSP/publication boundary.
- Public Audio.scheduleVoiceResume retains its three-argument signature/freeze
  policy, including generated TypeScript. scheduleVoiceResumeWithOptions adds
  {policy:"freeze"|"preserve",reset_gain?:boolean}; unknown policies/options fail.
  The initial nullable fourth-argument design was replaced after checking that
  the generated TypeScript would require it and break old three-argument callers.
- Original PCMCH chains now translate PLAY/READY variants, pause/resume/delay,
  independent slot volume, source/fade/DECIDE waits and checks onto engine-owned
  prepared sources and shared channels 4+index. Global PCM remains channel 1.
  PLAY initial fades use shared gain so later volume commands replace them;
  native resume selects a fresh gain fade. There is no private native decoder,
  source buffer, envelope, sample timer or output device.
- Explicit resource aliases preserve PCM/BGM/KOE/SE selection. BGM registration
  supplies its filename only; KOE uses koe/{number} without JITAN; SE forces an
  actual one-shot/no-fade while preserving requested native persistent flags.
  Declared list count defaults to 16 and is bounded at 256, subject to shared
  voice/memory/receipt capacity. The pinned upstream Rust player allocates only
  16 PCMCH slots; honoring a larger declared list is an explicit extension.
  Native queues are bounded per slot and globally serial-sorted with other audio.
  Sparse BGM registration indices are bounded at 4096 before lookup.
- Original save codecs/layouts are unchanged. Corrected false empty-slot volume
  1000 to native 255, retained original loop/READY/delay restart predicates and
  PCM-first then independent BGM/KOE/SE restore order. Delay is only an original
  file restart predicate. Runtime payload schema advances 4 -> 5 without old
  payload migration; logical cues rebuild at source start, with READY/paused and
  remaining integer-ms delay preserved. Pause fades rebuild fully paused and
  pending stops rebuild stopped; exact sample/envelope/DSP history is not saved.

Compilation:
- Final release workspace/all-target check passed with desktop  arc-formats,
  acoustics-obvhs and optional rodio-output. Minimal no-default desktop
  reallive+avg32 check passed.
- Regenerated reference/JSON/TypeScript and logged fingerprint
  **e73bda1334c12a7f -> 0d9cec36638123d1**. Final API reference/documentation and
  typed three-argument compatibility checks passed. New Rust request/inspection
  members require exhaustive matches/struct literals to be updated.
- Initial compilation corrected a missing async-control match arm and a type-hint
  name; initial API generation retained its expected missing-fingerprint guard
  failure before the real fingerprint was logged. Existing CBG/RealLive/minimal
  CLI warnings remain. All cargo validation used --release.

Behavior validation:
- Sample oracles cover preserved versus frozen delay, pause fades before/after
  resume deadlines, running clocks through starvation/holds, latest gain targets,
  short-source EOF with retained stop completion, pause-versus-stop deadlines,
  scope and worker publication. An actual delayed HRTF tail keeps its full PCM
  after input EOF cancels resume; a new source resume does not freeze/restart it.
- Authored original Scene.pck cases verify independent global/PCMCH mixing,
  per-slot gain, declared channel 255, integer-ms source completion, named source
  priority, BGM filename-only mapping, KOE/SE policy, READY and delayed resume,
  source/fade/key return values, STOP/EOF source release and failed replacement
  retaining the previous voice/original PC before successful resource repair.
  Resource worker gates leave original commands uncommitted until publication.
- Both save contracts are exercised: READY/paused/looped framework cues rebuild,
  remaining 80 ms resume delay reconstructs; original load does not reschedule
  saved delay or restart ordinary one-shots. A stopped READY flag can restart a
  paused cue on original load while a stopped framework snapshot stays stopped.
  Raw original PCMCH records assert actual 255 and unchanged 37-byte field order.
  Unknown routing/opcodes, missing aliases and sparse BGM overflow fail visibly.
- The independent transport_timeline release example exports authored stereo
  PCM16 **8 kHz / 2400 frames** and checks every float sample without a VM/device.
  Python WAVE inspection confirms initial RMS 0.25, paused-delay silence and
  post-EOF silence. Final sample/DSP checks include the input-EOF/HRTF and sparse
  lookup follow-ups, with final workspace/minimal compilation afterward.
- Evidence: target/verification/pcmch-final-run.json, pcmch-summary.json,
  pcmch-{audio-eof-final,bounds-final,completion-check,completion-minimal}.log,
  pcmch-demo-inspection.json and transport-timeline.wav. No game, font, measured
  HRIR/SOFA data, credential or proprietary SDK was added.

Remaining boundaries:
- Doppler/diffraction/higher-order sound, independent device clock, native spatial
  mappings and broader engine-wide async/3D/VN requirements remain active work.
  No listening-quality, performance ranking, GPU/hardware-RT, other-platform or
  complete-engine result is claimed by this local CPU validation.

## 2026-10-27 — Direct Doppler and motion capture (ADR 0084)

Implementation:
- Added engine-owned radial Doppler calculation, opt-in Spatial.doppler and
  optional source/listener world velocity. The bounded target composes with
  existing authored rate/semitone pitch through an independent output-frame ramp;
  it does not overwrite user controls, restart input or reset downstream DSP.
  Paused/starved/held input does not consume PCM while interpolation can settle.
- New DopplerSettings controls factor, medium speed, symmetric target ratio and
  transition time. Finite vector/range validation, bilateral 99%-of-sound-speed
  radial bounds and target-ratio bounds have separate diagnostic flags. Existing
  non-opted-in playback retains unity; disabled observations skip the calculation.
- Shared MotionTracker infers finite differences from caller dt, not wall time.
  Explicit velocities override it. First observations, gaps, teleports and changed
  zero-time poses reset inference; unchanged zero-time poses retain velocity.
  App captures Entity global parent motion and camera/manual listener motion once
  after world/late updates. Full client/slot/generation keys prevent stale reuse;
  ended/disabled histories are pruned. Restore clears derived history while
  retaining client motion settings as host policy.
- Added Audio.dopplerVoice, motionSettings, setMotionSettings and resetMotion;
  setListener accepts optional velocity. Desired target, installed ratio/remaining
  ramp, effective rate and capture/reset reasons are inspectable. target_pending
  distinguishes a desired change awaiting render-block setup. Mixer ownership
  and read observations retain the existing final DSP worker publication gate;
  motion controls cannot mutate during a pending frame. No new task pool or
  callback lock is added; small bounded pose capture remains owner control work.
- Reviewed oddio's complete pinned spatial/speed/public modules at registry
  version 0.7.4 / Git 85ad0723b460bc8c641198ef02bf114ef04f04f9, including its
  propagation/scene ownership. The standard radial formula and velocity/unit
  conventions were checked against OpenAL 1.1 section 3.5.2 and X3DAudio docs.
  Exact archive/source/PDF hashes and the selection rationale are recorded in
  docs/doppler-source-audit.json. No oddio implementation, new dependency, SDK,
  native opcode or legacy audio host is added. Native two-dimensional sound
  does not opt into Doppler automatically; save schemas/layouts are unchanged.

Compilation:
- Release workspace/all-target check passed with desktop  arc-formats,
  acoustics-obvhs and optional rodio-output. Minimal no-default desktop
  reallive+avg32 check passed. All cargo validation used --release. Existing CBG/RealLive/minimal CLI warnings remain.
- Regenerated reference/JSON/TypeScript and logged API fingerprint
  **0d9cec36638123d1 -> 768ebdf0e5892411**. API reference/command coverage checks
  passed. Added explicit public data descriptions/defaults for Spatial,
  VoiceSpec, Doppler and capture state. Old JSON remains default-disabled;
  fully specified Rust Listener/Spatial literals need their new fields/defaults.
- Initial compilation corrected the context-client accessor. Initial API
  generation retained its expected missing-fingerprint guard failure before the
  actual fingerprint was logged. These attempts are not behavior passes.

Behavior validation:
- Independent analytic cases cover approaching/receding source and listener,
  equal/transverse motion, coincidence, factor zero, extreme/clamped velocities
  and invalid input. Actual PCM and decoded-source positions verify resampling,
  composition with authored pitch/rate and chunk-invariant interpolation. Paused
  input keeps its position while Doppler settles. Listener scope and invalid
  update atomicity are checked.
- An actual HRIR case verifies Doppler resamples input before HRTF: input ends
  earlier while ear delays and filter tails keep their output-frame positions.
  Existing acoustic/reflection/reverb/resource/decoder/output tests remain green.
- Public engine cases follow a parent's Entity transform, camera motion, shared
  velocity, explicit overrides, teleport/zero-time/gap resets and host policy
  across restore. Worker-capacity gates keep ratio/observations frozen and prove
  polling does not capture movement or advance clocks twice. Unknown settings
  and foreign/stale handles fail. Real JS/TS Promise playback observes actual
  engine Doppler and capture reasons through the class registry.
- The initial parent-motion oracle confused 48 kHz output frames with the
  fixture's 8 kHz decoded-source units; it was corrected to 384 source frames
  after 16 ms at unity plus 16 ms at 2x. The production behavior was unchanged.
- The independent doppler_passby release demo exports stereo PCM16 **48 kHz /
  96000 frames** from an authored 440 Hz source moving past a listener. Windowed
  positive-crossing estimates are **500 Hz approaching / 395 Hz receding**;
  these are coarse 0.2-second-window estimates, not instantaneous-frequency
  claims. Worker and synchronous exports are byte-identical. Python independently
  decodes the WAVE, checks metadata/frequencies and records its SHA-256. No VM,
  output device, game, font or measured HRIR/SOFA dataset is used by this demo.
- Evidence: target/verification/doppler-{audio-completion,api-engine-final,
  regression-final,workspace-check,minimal-check,python-final,demo,demo-sync}.log,
  doppler-summary.json, doppler-demo-inspection.json and doppler-passby*.wav.

Remaining boundaries:
- This is a direct instantaneous radial model feeding existing linear source
  resampling. It adds no retarded-time/direct propagation delay, independent
  reflected-path Doppler, wind/moving medium, diffraction, shock-wave simulation,
  predictive motion, band-limited resampling or independent device clock.
  Automatic frame-difference velocity is not an exact physics-body velocity;
  explicit input is available. Existing interpolation may settle after reset.
- Native spatial opcodes/HostServices facade and broader async/3D/VN capability
  requirements remain open. No full native game, cross-platform/hardware result,
  performance ranking, listening-quality comparison or complete-engine claim is
  established by this CPU release validation.

## 2026-10-27: Shared cue programs, PCMEVENT prerequisite (ADR 0085)

Implementation:
- Added engine-owned once/loop/weighted cue programs to crysirror-audio, with
  device-free Core DTOs and HostServices methods. SharedHost and the public Audio
  class use the same scoped ChannelAudio bank. Six Audio operations create,
  advance, poll, acknowledge, inspect and release scheduling state.
- Programs select game/real owner clocks, signed/exclusive interval ranges,
  optional acknowledged source duration, reset/carry timing and softened repeats.
  Default uniform rejection sampling is separate from explicit original modulo32
  RNG semantics. Native shared-RNG injection is possible at a new due selection;
  interval RNG runs only once at acknowledgement. Keys convey no file authority.
- Pending emission serials freeze scheduling time across worker resource/voice
  dependencies; repeated polling is idempotent. Stale/foreign handles/receipts
  fail. Zero/negative intervals have a visible 1024-emission budget per explicit
  owner advance. Bank/program/key/weight/time bounds are documented in ADR 0085.
- Restore/clear invalidates scheduling state while retaining generation counters.
  Final DSP controls are fenced; immutable info stays readable. Scheduling release
  leaves independently published voices alone. App does not implicitly advance
  caller-owned sequence clocks, and working cursors are not implicitly saved.

Compilation and reference validation:
- Release workspace/all-targets check passed with desktop  arc-formats,
  acoustics-obvhs and audio rodio-output enabled. Minimal desktop no-default
  reallive+avg32 release check also passed. Existing image/RealLive/CLI warnings
  remain.
- Generated reference/JSON/TypeScript and logged actual API fingerprint
  **768ebdf0e5892411 -> 2077f8fd45fb4b36**. Reference and command coverage checks
  passed. Initial generation attempts hit the expected changed-reference and
  missing-fingerprint guards; those attempts are not counted as passes.

Behavior validation:
- Shared audio library/integration scopes: **124 passed**, one owner-NWA-data
  test ignored and excluded. Engine cue/API/reference/frame scopes: **16 passed**.
  Real JS/TS cue Promise scope: **1 passed**. **141 distinct Rust tests passed**;
  focused/initial reruns are not counted again. Python 3.11 tooling: **189 passed**.
- Independent sequence cases verify real/game clocks, pending time freeze, exact
  original repeat-weight/RNG draw order, deterministic uniform distributions,
  reset versus overdue carry, last-interval completion, malformed-loop bounds,
  zero-weight fallback, capacity, invalid update atomicity and generation scope.
- Public engine tests prove native HostServices and scripts inspect the same bank,
  restore invalidates identities, and real DSP capacity backpressure admits only
  frozen observation without advancing sequence clocks. Real JS resolves an
  emitted key through actual shared resource and voice Promises, acknowledges
  decoded duration, and verifies releasing scheduling does not stop that voice.
- Independent authored cue_sequence demo renders actual stereo PCM at **8000 Hz /
  160 frames**: positive cue starts frame 0, negative cue starts frame 56 (7 ms),
  inputs end at frames 24/72, and scheduling finishes at 13 ms. Every PCM sample
  is checked. Sync and final-worker exports are byte-identical; Python independently
  decodes the WAVE and checks amplitudes, timing and SHA-256. No VM, game, font,
  hardware output device or proprietary data is used by this demo.
- Evidence: target/verification/cue-sequence-{audio-regression,engine-final,js,
  workspace-check,minimal-check,api-final,python,demo-worker,demo-sync}.log,
  cue-sequence-{summary,demo-inspection}.json and cue-sequence-{worker,sync}.wav.

Remaining boundaries:
- Program clocks/acknowledged durations are caller-supplied, not automatic sample
  timestamps. Scheduling is owner-frame granular; voice DSP remains sample-based.
  No hardware, cross-platform playback, performance ranking, propagation-delay or
  complete-engine claim is established. Broader 3D/VN/async work remains active.

## 2026-10-27: Original PCMEVENT over shared cue services (ADR 0086)

Implementation:
- Extended the pinned source audit with native clock feed and complete selected
  MATH.RAND/xorshift branches at commit 5f358ef1360b0439e92aed2b749e8f25d8721954.
  Six reference blobs match exactly; runtime/mod.rs has a pre-existing wipe_mask
  visibility export patch. Its reviewed clock feed matches the actual pinned blob.
  The final audit assertion caught this difference; the manifest records it, without
  claiming whole-file equality. No original PCMEVENT/player module is copied/imported.
  Shared Core admission validates event declarations and cursors.
- Added public Audio.captureCueSequence/restoreCueSequence and typed HostServices
  equivalents. Portable cursors preserve clock/line/RNG/emission/frame-budget data,
  reject outstanding dependencies and invalid input, and restore to a fresh scoped
  identity. These operations work independently of any VM and reconstruct no voice.
- Native script-global RNG is injected before selection and published after exact
  resource/voice acknowledgement, including an immediate final interval. Native
  math and event choice share the same stream. START/STOP release the old engine
  program in the command's commit batch, instead of leaving it alive another frame.
- Runtime payload schema **6** stores pre-selection cursors and native frame pass
  continuation, never worker/engine handles. Async resume does not tick input/VM
  time or draw twice. A clock stamp avoids charging dependency time on the next
  frame. Active events keep their owner service ticking after script EOF, without
  replaying the script. Original file codecs remain unchanged: LOOP/RANDOM restart;
  ONESHOT is discarded. Voices retain the existing reprepare/restart save policy.

Compilation and reference validation:
- Release workspace/all-targets check passed with desktop  arc-formats,
  acoustics-obvhs and audio rodio-output. Minimal no-default desktop reallive+avg32
  check also passed. Existing image/RealLive/CLI warnings remain.
- Regenerated API reference/JSON/TypeScript and logged actual fingerprint
  **2077f8fd45fb4b36 -> 1c682b7ad1fb38cd**. API/reference coverage passed.
  Initial generation retained the expected drift/missing-fingerprint guard failures;
  those attempts are not counted as passes.

Behavior validation:
- Shared audio scopes: **126 passed**; original VM unit/save scopes: **16 passed**;
  native audio/PCMCH/PCMEVENT scopes: **55 passed** (including 11 PCMEVENT cases);
  engine cursor/frame/reference scopes: **17 passed**; actual JS/TS Promise scopes:
  **14 passed**. **228 distinct Rust tests passed**. Two ignored tests (owner NWA
  data and an explicitly supplied system font) are excluded; earlier/focused
  reruns are not counted again. Python 3.11 tooling: **189 passed**.
- Shared cases test portable cursor roundtrips, fresh identity, malformed staging,
  deterministic RNG continuation and preservation of a spent frame budget. Public
  engine/JS cases use actual source/voice Promises and keep voice lifetime distinct
  from scheduler restore/release. Read-only observations remain frozen under DSP
  capacity backpressure.
- Authored original Scene.pck cases inspect shared sequence identities, actual
  decoded PCM, global/slot routing, interval-vs-source completion, weighted line
  order, MATH.RAND interleaving, STOP with/without stopping PCM, and natural/key
  waits. WAIT_KEY leaves the event running, including a later cue after script
  EOF; loop events continue while the halted script's stack remains unchanged.
- A real I/O-capacity gate lets host time advance while preparation is pending.
  Snapshot/restore reproduces one selection/acknowledgement, the same RNG and
  interval, without duplicate playback or retroactive dependency-time advancement.
  Runtime cursor restoration and original file restart/discard policies are checked
  separately. Unknown operations/parameters/layouts/routes and missing aliases fail.
- The initial native run exposed a second cleanup from an original-load marker;
  explicit import now consumes that marker once. Code review also corrected deferred
  interval catch-up and immediate-final-interval RNG publication; stronger boundary
  cases verify the resulting behavior. The failed initial run is not a pass.
- Evidence: target/verification/pcmevent-{shared-regression,vm-published,
  native-published,engine-final,js,workspace-published,minimal-published,
  python-final}.log and pcmevent-summary.json. No game, font or proprietary data is
  committed; no audio device or hardware result is claimed.

Remaining boundaries:
- This is the explicit resource/default PCMCH routing profile, bounded to 128
  native event declarations and existing shared limits. Native START uses real
  time; saved game-clock events use the current unscaled native game clock.
  Skip acceleration, broader SCRIPT/SYSCOM routing and resource discovery remain.
- New input edges while native audio admission is pending are not yet buffered.
  Exact ordering of native frame-action callbacks versus this event pass also needs
  further work; these straight-line fixtures do not establish callback compatibility.

## 2026-10-27: Shared logical input journals and native continuations (ADR 0087)

Implementation:

Compilation and reference validation:
- Release workspace/all-target check passed with  arc-formats,
  acoustics-obvhs and rodio-output; minimal desktop reallive+avg32 also passed.
  Existing warnings remain.
- Regenerated API reference/JSON/TypeScript; actual fingerprint changed
  1c682b7ad1fb38cd -> 4b42007e15453574 and is logged. Final coverage/docs passed.

Behavior validation:
- Release scopes: Core input 5, Scene 22, original VM 16, native audio/PCMCH/
  PCMEVENT 58, engine input/frame/reference 23, actual JS async services 15:
  **139 distinct Rust tests passed**; one external-font test ignored, excluded.
  Python 3.11 tooling: **189 passed**. Repeated focused runs are not added again.
- Cases cover capacity/owner/epoch errors, immutable motion leases, exact high
  mask bits in actual JS, multi-client atomic ingress and restore invalidation.
  Native worker gates retain two clicks across preparation and snapshots, route
  down/up after event readiness, and discard pre-load clicks on original import.
- An initial command was rejected for duplicate --lib flags; corrected scopes
  ran successfully. Expected generation guards caught drift/missing fingerprint.
  The full JS run exposed an old zero-time assumption at Promise completion;
  single/full diagnostics passed but did not erase that failure. The test now
  checks stable source/pause/stop state and transport <= age, allowing a mixer
  publication before Promise polling. The corrected final suite passed.
- Evidence: target/verification/input-inbox-{core-tests,vm-scene,native-final,
  engine-final,js-published,workspace,minimal,python}.log and input-inbox-summary.json.

Remaining boundaries:
- Framed digital/pointer data only; no raw OS/IME/analog/sub-frame capture or
  hardware restoration. Public journals are opt-in; native default capacity256
  fails visibly on overflow. Other native adapters have not been migrated.
- Original frame-action versus audio-event ordering remains open. No complete
  game, hardware or whole-engine result is claimed. The broader goal remains.

## 2026-10-27 — shared Yamaha sources and worker image capabilities (ADR 0089–0090)

Implementation:
- Extracted the pinned UK2 pure YM2608 DSP and PMD score driver into shared
  crysirror-audio, retaining MPL provenance; no UK2 VM, Kira output or second
  mixer/renderer was imported. Existing Audio resources, decoder workers, buses
  and worker final mixing consume .opna register scores and bounded PMD-M
  .pmd/.mmm streams independently of any VM. Unsupported chip/driver modes,
  malformed directory/voice reads and zero-time loops fail visibly. Exact rational
  chip clocks, partition-invariant reads and replay seek bounds are explicit.
- Shared crysirror-image owns gamma/gray/invert/flips/noise, premultiplied Gaussian
  blur, tiny-skia vector paths/strokes, inverse projective sampling, mask/blur
  transitions and preserved-edge sine row copying. Core immutable image leases
  and owner-polled completion contain no device handles. Runtime reserves the
  existing CPU pool and bounded processing memory before accepting work.
- Added self-authored examples/shared-effects and audio chiptune examples.
  Sources/pins/hashes and exclusions are in docs/synth-image-source-audit.json.
  No fonts, ROMs, sample banks, game files, captures or compiled outputs added.

Compilation:
- Release workspace --all-targets check with desktop arc-formats/
  acoustics-obvhs and audio rodio-output passed. Existing CBG/RealLive warnings
  remain. An initial command incorrectly named the acoustics feature on audio;
  feature resolution failed before compilation, then the corrected command passed.

Behavior validation completed:
- Release synth integration: 4 passed; shared image-effect pixels: 7 passed;
  Core regions: 3 passed; public shared-effect/audio ownership tests: 3 passed.
  Earlier library regressions: audio 15 passed, 1 ignored; image 63 passed.
  The ignored environment-dependent audio test is not counted as a pass.
- Release JS/TypeScript Bitmap regressions: 2 passed, including awaited shared
  effects and transitions followed by ordinary retained-layer presentation.
- Release generated API docs: 7 passed; command coverage: 1 passed. Fingerprint
  be619476a3bcddb4 -> 4fa7e1d7f355d61a regenerated and logged.
- Python 3.11 tooling suite: 189 passed. Default Python lacked tomllib during
  source-audit generation; that failed invocation was rerun with Python 3.11.
- Actual public-API showcase captured at 780x520 on Apple M5 / Metal, raster
  renderer. All eight panels and labels visually inspected: original, gamma,
  Gaussian, vector, perspective, row distortion, mask and blur transition. This
  is one local raster capture, not GPU effect timing, hardware RT or cross-platform
  evidence. Self-authored WAV contains 96000 stereo PCM16 frames at 48000 Hz;
  worker mixing asserted finite/nonzero output with peak 0.3520. No live device
  or physical chip parity is claimed.
- Initial DSP edition/borrow issues and the expected generated-API fingerprint
  guard were corrected before final validation. Evidence stays ignored under
  target/verification/synth-effects-*.log, shared-effects.png and synth-demo/.

Remaining boundaries:
- This adds reusable capabilities, not a complete UK2 VM or all PMD/chip dialects.
  ADPCM/ROM banks, FM3 special mode and unsupported driver modes fail explicitly.
  This is a sound model with procedural rhythm, not silicon-accurate emulation.
- ADR 0088's former nonidentity gamma/gray/flip gaps are superseded only for the
  image-resource profile above. Full native save/media/blend/clip/mask semantics,
  general Canvas readback, GDI/metafiles and arbitrary plugin DLL compatibility
  remain open. Engine mask/blur transitions do not claim KaichoTrans equivalence.
- Processing is CPU-worker based. GPU shader variants, metadata/discovery async,
  every other ADR 0050 boundary and broader 3D engine completion remain open.

### Implementation

- Explicit `void` native storage strings convert to the empty string, distinct
  from a missing argument. `windowEx.getCursorPos` reads the client engine input
  snapshot in virtual window coordinates (not the OS desktop cursor).
- Shared text-file encoding and scoped VFS metadata back `Array.save2` and the
  fstat read/query profile. Saves await actual atomic-write completion; failure
  does not resume the script as success. Unsupported mutation/saveStruct2 modes
  remain explicit errors.
- Shared M2 motion resources, worker samples, validated dynamic PuppetFrames and
  immutable textures back the Motion native profile. This does not imply all
  Motion timeline, physics, masking or feedback modes are implemented.
- MP4 AVC/HEVC/AV1 software decoding is in `crysirror-video`, with pinned Rust
  OxideAV decoders and provenance in its SOURCE.json. The unused `crysirror-mp4`
  prototype was moved to ignored local research storage, removed from the
  workspace. The dependency baseline is now Rust 1.95 (tested host: 1.98.1).

### Compilation and behavior evidence recorded so far

- Release storage-plugin tests: 3 passed. Independent runtime text-encoding and
  scoped metadata tests: 1 passed each. Exact CP932/UTF-8 bytes, newline modes,
  worker failure, client isolation and absent archive timestamps are covered.
- Release video tests: 16 passed, 1 existing local-file test ignored. Six
  generated MP4 samples cover AVC Baseline/Main with B frames, HEVC Main/Main10,
  AV1 8/10 bit; decoded pixels, ordered PTS, seek, rewind, cancel and EOF are
  checked. Old MPEG/WMV/AC3 tests also ran. No FFmpeg subprocess is used for
  runtime decode. These fixtures are synthetic, not user game data.
- Earlier in this phase, the native bridge suite passed 22 tests with 1 ignored,
  and Python tooling passed 191 tests. Those results precede the new crossfade
  and Motion changes and are not reported as verification of the final state.

- Subsequent native bridge release run: **23 passed, 1 ignored**, including
  actual engine-clock crossfade completion, self-bound native object identity,
  order/visibility exchange and exactly one callback.
- Runtime Media release behavior run: **13 passed, 1 ignored** (47 unrelated
  tests filtered). AVC/HEVC/AV1 frames publish through the existing bounded media
  workers; an MP4 audio track with audio enabled fails explicitly. MP4 audio
  decode, HDR tone mapping and hardware video decode are not implemented.

- Shared subtree state tests: **3 passed**. Explicit release GPU test on
  **Apple M5 / Metal: 1 passed, 0 ignored**. Pixels verify overlapping descendants,
  gamma blend, transparent alpha, clipping, endpoints, unchanged external layers
  and stop restoration; invalid progress and oversized worksets are rejected.
  An initial cache-key review concern was withdrawn after tracing the actual
  ordinary-image path; the original namespace is correct and is GPU-tested.

- Motion release native profile test: **1 passed**; public frame-validation and
  rectangle-clipping/UV-color interpolation tests: **2 passed**. Explicit local
  logo native tests both passed (yuzulogo: 5 parts/14 immutable textures at
  500 ms; m2logo: 13 parts/38 textures). They verify engine-owned 1920x1080
  presentation, real decoded texture resources, retained resource lifetime and
  one onSync event, with `owns_framebuffer=false`. These are headless native-to-
  engine checks, not whole-game or GPU visual validation.

- Canvas lifecycle release tests: **14 passed, 0 ignored** (3 new regressions
  plus 11 existing Canvas tests). Cleanup of missing derived nodes is scoped to
  Canvas ownership; ordinary Layer Remove still rejects an absent ID.
- API reference regenerated, command coverage passed (1 test), and the release
  API documentation binary passed all 7 tests after logging fingerprint
  `cca12b45f54104e4`. Public command coverage also now records the existing
  Canvas resize/fill/transform_copy commands.

- The next actual game replay passed the earlier Canvas failure and executed to
  `custom.ks:110` (`loadfadein prepare`, after the logo sequence). It then failed
  explicitly at `Layer.piledCopy`. No title image was captured. The run exited
  itself before a proposed termination signal was sent; it was not a timeout or
  a successful render. Evidence: local `title-bringup/render6.log`.
- Sampling this replay exposed repeated rebuilding of every Canvas presentation.
  Dirty-source/root filtering now updates only affected bindings. Its release
  tests plus lifecycle/existing Canvas coverage: **17 passed, 0 ignored**.
  This is behavior verification; actual post-change game timing is pending.

- CopySubtree now freezes one `Paint::Snapshot { size, layers }` rather than
  expanding its draw tree into the destination. Scene release coverage is
  **25 passed, 0 ignored**. Explicit Snapshot GPU tests on Apple M5/Metal:
  **2 passed, 0 ignored**; overall/ancestor opacity, scaling and source mutation
  preserve the frozen product with exactly one rasterization. Immutable-image
  validation and asynchronous readiness are tested; ordinary paths, dynamic
  surfaces and uncaptured font aliases are rejected in this first profile.
- Replay 7 reached the same prepare point but rejected a type-1 source. Upstream
  inspection identified that raw-image Complete is a direct RGBA copy independent
  of type/opacity; the native fast path is being corrected. Composite opaque
  alpha is not universally 255 in upstream and is not silently normalized here.

### Final verified checkpoint for this batch

- MP4: pure Rust AVC/HEVC/AV1 video is wired through the shared decoder and Media
  workers. The old prototype is outside the source tree. MP4 audio, hardware
  decode, HDR tone mapping and exhaustive codec conformance are not claimed.
- Native piledCopy release tests: **3 passed, 0 ignored**. The raw-image branch
  preserves RGBA (including hidden RGB in the synthetic PNG) independently of
  opaque type/opacity; the composed branch uses the shared Snapshot profile.
- Explicit GPU regression across Snapshot and subtree crossfade: **3 passed,
  0 ignored, Apple M5/Metal**. Image readiness is gated deterministically; an
  unfinished dependency never becomes a cached blank product. Crossfade's
  alternating-input snapshot cache retention remains a performance follow-up.
- Final API fingerprint: `08381e9cfceae9bf`. Reference regeneration and command
  coverage ran; after the fingerprint log update, all **7 API documentation
  checks passed** from the newly built release test executable.
- No commits or pushes were made.  Verification used local
  release builds, synthetic redistributable fixtures, existing local game data
  and runtime system fonts.

## 2026-10-29 — RealLive: AIR SE and Game_RUSUR play through the story

Owner-authorized games: `~/Downloads/AIR Standard Edition` (a RealLive title,
not AVG32: Seen.txt scene index, g00 art, RealLive.exe) and
`~/Downloads/Game_RUSUR`. Scripts: `target/verification/reallive/scripts/`.

- RealLive adapter VM includes settings/syscom, frame counters, GAN, the object model
  (params, properties, mutators, pattern/GAN animation, promotion) and
  `modules/obj.rs`, and the VM-state 1:4 system functions (arithmetic, date,
  settings 22xx/23xx/26xx, syscom states, flags, savepoints, call stack,
  `end`). The extension DLLs (EF00, PT00 Little Busters, DT00 Tomoyo After)
  are vendored as `crates/reallive-upstream/reallive-dll` and wired to
  LoadDLL/CallDLL; no authorized game uses them, so they are compiled and
  wired but not game-tested. Unknown DLLs fail visibly.
- Engine additions: RealLive g00 atlas decoding with pattern regions
  (`crysirror_image::g00`; the shared g00 decoders now return multi-pattern
  files as that atlas), `#SEL` reveal rule images
  (`crysirror_image::reallive_wipe`, used with the engine mask transition),
  canvas `filter` (region mono/invert/light/colour), snapshots of
  self-contained plain project images (content-pinned), present_surface
  release, raw NWA with an unset packed size.
- Adapter mapping onto engine services: objects as retained layers (geometry
  from upstream blit_region: origin/rep origin, scale, rotation, clip,
  alpha, add/subtract, filters; digits; child objects), haikei backgrounds
  (1:40), grpMulti, grpZoom, grpStretchBlt, freeDC, grpFill/Outline,
  region colour effects, masked copies, manual draw mode (held batches until
  refresh), window frames (WAKU types other than nine-slice: BACK tinted by
  the window attribute, NAME on top), choices laid out in the text window,
  kinsoku and page-full continuation, koePlay (loose files, or voice
  archives mounted as engine containers), SE table, BGM replacement fades,
  volumes, screen shake (whole scene under one root layer), name entry OK.
- Behaviour (headless render, scripted clicks): AIR — title, chapter
  select, prologue, opening sequence, story into day 2 (July 19) with
  backgrounds, sprites, calendar object, choices, voices; 150,000 frames, no
  error, no approximated transition. RUSUR — title, name entry, story to day
  14; 300,000 frames, no error.
- Not done / known gaps: save/load and the system-command menu (right-click
  does nothing yet; RealLive saves need graphics-stack replay); save
  thumbnails; motion transition styles (scroll/squash/slide/zoom/ripple)
  are shown as crossfades and counted in diagnostics
  (`approximated_transitions`); nine-slice window frames, per-channel tint,
  drift (weather) objects, HIK animated backgrounds and filter-area objects
  fail visibly; the text-object control syntax (`#c` etc.) is not
  interpreted; window open/close animations, backlog and auto mode are not
  implemented. Windowed play not verified.
- Tests: reallive/reallive-dll/image/scene/runtime/engine/audio/
  formats-arc 907 passed, 0 failed; policy tests OK; api_docs pass
  (fingerprint 0a86cec0fc22dca2, logged).

Follow-up (same day, owner report: kr crashes after a while, AIR `play` would not start):
- `run.py … play` passed a nonexistent `play` subcommand; it now runs `crysirror run`.
- AIR window exit ("manual-mode draw backlog limit") fixed: manual draw mode
  now presents a frozen copy of DC 0 that `refresh` replaces, instead of
  holding batches.
- Canvas `restore` (replace a surface with a queried state) added for native
  saves. Tests: scene pass; wgpu with ignored GPU tests (Metal, font set)
  60 passed 0 failed (new: text inside a frozen canvas).
- Owner report "the text window stays up during AIR's OP": the adapter had
  msg 151/152/162 inverted. Now as upstream: 151 closes the active window
  (frame included), 152 clears it, 161 closes all, 162 clears all. AIR's
  opening shows no window; story text still shows it; RUSUR 300k frames pass.

## 2026-10-30 — Parallel native API profiles and shared Media

- Implementation: shared Media workers now route capability readers through
  ASF/WMV/WMA and OMV/Ogg Theora/Vorbis. Float PCM appends preserve decoded
  samples without integer quantization. ASF metadata/packets/reassembly and
  Theora dimensions/Vorbis formats have explicit bounds. `media_frame` exposes
  immutable engine pictures whose clones retain memory leases; foreign clients
  cannot read another resource. Stop releases the manager's picture. No VM,
  decoder process, window or private native mixer owns this pipeline.
- Implementation: NScripter is a first-class source interpreter using shared Canvas/Layer/GlyphRun/UI/resource/audio APIs, worker loading and explicit unsupported operations.

- Compilation: final desktop release build with `nscripter,arc-formats`
  passed (`parallel-final2-desktop-build.log`); release desktop checks passed.
  Source-generated adapter metadata and strict Python config/declared-inventory
  validation now includes NScripter. API fingerprint is
  `810d2690fcbf11b5` (formerly `26edfb80137ff54a`), logged in docs/api/CHANGES.md.
- Tests: NScripter syntax 2 and native integration 4 passed, including hidden choice ordinals and unsupported-save failure at the native PC. ASF reader tests 2 passed/1 ignored; the owner WMV prefix separately passed 1. Theora/Vorbis reader test 1 passed/1 ignored. Audio release 16 passed/1 ignored. API docs 7 and Python tooling 200 passed. Ignored tests are not counted as passes.

- Behaviour: the authored NScripter source renders shared image/text layers and waits for input. This is a native-format fixture, not real-game validation.

- Remaining scope: the recorded NScripter plain-nscript-v1 profile lacked complex windows/markup/effects, archive mounting, numbered script concatenation, skippable delay, original saves and real-game validation. This is the historical result for that source batch.

## 2026-10-30 — Desktop native menu bar and title-bar policy

- Implementation: desktop continues mirroring the engine-owned MenuSystem into
  AppKit's application menu bar on macOS and the HWND menu bar on Windows.
  Native selections and accelerators use the same scoped activation validation
  as shared UI, with diagnostic activation logs. Windows menu replacement now
  tracks attachment separately from its dirty cache, so a selection-triggered
  rebuild detaches the previous bar. macOS explicitly uses Regular application
  activation and disables winit's competing default menu. Linux keeps a visible
  engine-drawn fallback; this is explicitly not a native Linux menu bar.
- Implementation: ordinary desktop windows retain their OS title bar by default.
  `--allow-borderless` permits a game's undecorated-window request; fullscreen
  remains independent. The original native borderStyle and shared window-policy
  request are retained. Unchanged window styles are no longer reapplied every
  frame. No bundle was added.
- Compilation: full-feature desktop release check passed (nscripter,
  arc-formats; native-launch-round/check-v2.log). An offline release
  cross-check for x86_64-pc-windows-gnu passed
  using the already installed target/toolchain. Its small ignored harness
  directly includes the production native_menu.rs with real Scene/winit/muda
  dependencies and deny(unsafe_code). This checks the Windows presenter source,
  not the full desktop application or Windows runtime behaviour.

## 2026-10-30 — NScripter installed Tsukihime native bring-up

- Implementation: native named variable indices, backtick English strings,
  multiline selections, inline click/page/speed/wait controls, physical-line
  `skip`, date/timers/delay, square horizontal `setwindow`, staged sprite moves,
  character placement and native atlas buttons now call shared engine services.
  Image alpha/key/paired-mask extraction and stripe/stagger rule generation are
  shared `crysirror-image` worker effects. Sprite-sheet timing/UV selection is
  shared `crysirror-scene::sprite`; retained widget hover/focus/press inspection
  is a client-scoped engine query. No ONScripter runtime/renderer is linked.
- Implementation: original game archives stay packed. Native resource policy
  searches loose files then explicit engine archive prefixes, with scoped case
  matching; the desktop bootstrap owns shared mounts. Native global indices
  >=200 and accessed-resource metadata use worker-backed atomic Save writes to
  `nscripter-state.json`, a separate engine format, not an original game save.
  Framework snapshot schema is 2; pending work and prepared image reconstruction
  are explicitly rejected. Native savegame/loadgame remains unsupported.
- Compilation: the release owner_game target compiled and executed. Earlier
  failed iterations (private module access and native `trap` label tokenization)
  are retained as failures in the verification directory, not counted as passes.
  The final combined desktop/check/test run is recorded separately by the root
  task after merging all parallel changes.
- Behaviour (device-free installed game): owner-provided Downloads/Tsukihime
  nscript.dat, arc.sar, arc.nsa, arc1.nsa, default.ttf and CD Ogg tracks run in
  place. The ignored owner test reached the real title's btnwait at native line
  12792/PC10802 after 2616 ticks, asserting title1.png/titlebtn1.png resources.
  Reusing the same release test executable with CRYSIRROR_NSCRIPT_STORY=1 used
  the title and Start submenu buttons, passed the native Prologue chapter
  sequence and reached s20's hospital sentence at line14118/PC12003 after3152
  ticks; the background is image/bg/bg_51b.jpg. No script patch or PC jump was
  used. This proves native control/shared state, not a GPU screenshot or audible
  device output; the desktop render verification is separate.
- Validation: syntax tests 3 passed. A preceding native suite had 3 passes and
  one obsolete direct-file-asset assertion after worker surface extraction;
  that assertion now checks engine-owned RGBA and retained native source policy.
  Added a cold-instance shared metadata persistence test; its final execution,
  plus shared key/mask/sequence/interaction tests, belongs to the combined run.
- Limits: full original save/global-file interchange, system menus/backlog/skip,
  rich markup, general animated sprites, numbered source concatenation and full
  window enter/leave animation remain unfinished. Stored window/menu definitions
  do not establish those features. Rule transitions use 8-bit masks and have no
  original integer-pixel parity claim. Image metadata admission is synchronous;
  full ADR0050 staging remains open. No whole-game completion claim. Evidence:
  target/verification/nscripter-tsukihime-owner-third.log,
  nscripter-tsukihime-story-first.log and nscripter-tsukihime/*.json.

## 2026-10-30 — NScripter implementation and validation

NScripter rendering follow-up (same round): the desktop hospital capture exposed
an adapter error that logical background checks alone missed. The original
bg_51b.jpg is a complete 640x480 image; the right half was covered by a stale
right-character layer from the chapter sequence. Pinned ONScripter bgCommand
removes all three character slots. The native adapter now clears their retained
and unpublished staged declarations on bg, while retaining ordinary lsp sprites.
An independent regression covers both published and staged character removal.
The corrected desktop image and final release test results are recorded by the
root task's final validation; the earlier half-black capture is rejected visual
acceptance evidence.

## 2026-10-30 — Shared pure-Rust content identification

- Implementation: `crysirror-resource::sniff` exposes FileDetector and owned
  FileIdentity evidence using pinned pure-magic 0.4.1 and magic-db 0.6.0. A
  OnceLock caches one verified immutable Arc-backed embedded database and its
  initialization failure, with a PNG rule/MIME sentinel. The public API uses
  caller slices or AssetReader capabilities; it has no libmagic FFI, file(1)
  subprocess, ambient file/mmap entry point or runtime rules installation.
  No filename extension is passed to the matcher. Source/version/archive hashes,
  licence provenance and the database's existing DER exclusion are recorded in
  crates/crysirror-resource/SNIFF_SOURCE.json.
- Implementation: identification requests at most a 64-KiB logical prefix.
  Partial reads are completed, while premature zero reads, invalid read counts
  and changed source lengths fail visibly. Generic empty/text/data fallback is
  None; positive MIME/description/rule/strength observations are explicitly
  evidence, not structural validation, authentication or decoder/VM support.
  The detector is Send+Sync; matching owns its result and retains no VM or asset.
  Desktop uses it from the named boot-detection worker before Runtime creation.
- Compilation: the unified full-feature desktop release build passed
  (native-launch-round/build-final.log). No libmagic library was required.
- Tests: six independent resource release tests passed, covering embedded PNG/
  WAVE rules, unknown/empty/short headers, partial and malformed capability
  reads, the fixed prefix budget, renamed extensions and deterministic shared
  concurrent queries. The same unified run passed the desktop's renamed-PNG
  native-script rejection test. Logs: native-launch-round/unified-tests.log.
- Limits: classification cannot inspect rules beyond the sampled prefix, verify
  complete files, authenticate bytes or detect arbitrary same-size mutation.
  Engine-specific signatures and parsers still establish native layouts;
  recognition does not add an unsupported format decoder or engine adapter.
  No bundle, commit or push was added.

## 2026-10-30 — Installed-game bootstrap, content evidence and final offscreen verification

- Implementation: `play <folder-or-file>` now constructs typed native projects
  and archive mounts internally. NScripter metadata paths join
  the existing RealLive/AVG32 detection. A named boot worker performs
  inspection; Resources `beginDecode`/completion/`mountDecode` indexes on engine
  workers and publishes scoped containers before VM execution. No generated
  `play-mount.ts` is needed. `--headless`, frame bounds, capture and input tracks
  exercise the same launch plan without a window/audio device. This boot fence
  still waits before the event loop; it is not a loading-screen/full-async claim.
- Implementation: shared `resource::sniff::FileDetector` uses pinned pure-magic
  0.4.1 and embedded magic-db 0.6.0, with immutable shared rules and at most 64 KiB
  per capability/slice. It has no C libmagic dependency or filename hint.
  Partial reads, premature EOF, invalid read counts and changing lengths fail;
  unknown fallback stays unknown. Detection records MIME/rule evidence while
  native parsers still validate signatureless/encrypted layouts. Renaming a PNG
  to 0.txt is rejected. This does not add format decoders by itself.
- Implementation: NS source/resource/UI/audio policy calls shared engine
  services. GPU inspection caught an additional real native-policy bug after
  successful control-flow tests: `bg` left a chapter's right-side TACHI in the
  scene, obscuring half the hospital background. The adapter now removes all
  three published/staged character slots together with background presentation,
  retaining ordinary sprites. A native regression covers both published and
  staged slots so a later print cannot resurrect them. The original half-black
  capture is marked `rejected-retained-character-story.*`, not accepted evidence.
- Compilation: the final full-feature desktop release build passed
  (`native-launch-round/build-final2.log`, features nscripter,
  arc-formats). API docs regenerated and verified: fingerprint
  `8ff9a0e655c6babe`, previous `8e6792f059054768`, with migration notes.
- Tests: unified release validation passed 181 tests / 1 ignored across 12
  targets, including desktop 17, engine API docs 7, native menu bridge/popup
  failure 2, resource sniff 6, NS syntax 3/native integration 5, image 94
  (1 ignored), and shared UI/sprite state. After the final NS background fix,
  desktop 17, NS syntax 3 and native integration 6 passed again; these overlap
  the unified totals. Explicit Metal GPU canvas tests passed 3, including
  RGB-only copy, compaction/materialization parity and permanent resize crop.
  Python tooling passed 200. Ignored tests are not passes. No full-workspace
  or whole-game validation is claimed.
- Behaviour (final offscreen path, Apple M5/Metal): automatic original-directory
  launch reaches Tsukihime's real title (line 12792) and Start -> chapter curtain
  -> hospital dialogue (final PC 12004 / line 14119). The final picture shows the
  full hospital background and the English first sentence, with the stale TACHI
  absent. Engine audio is mixed (active 1, peak 1.0); device output is not tested.
  Title and final story captures are `native-launch-round/tsukihime-title.png`
  and `tsukihime-story-final.png`. SAR/NSA patches and local font are used in
  place; no user game data or font was added to the repository.

## 2026-10-30 — Circus MES/CRXG/XPCM groundwork and explicit execution gap

- Implementation: new first-class crysirror-circus parses native offset2 MES
  versions 0xaab6/0xabb6. It preserves signed word/byte/string operands, original
  label metadata and instruction boundaries; native 0x77 continuation operands
  are consumed with their preceding command. Byte/string/instruction budgets,
  malformed headers, truncation and non-boundary PC lookups fail explicitly.
  Pinned MesTextTool format reference/provenance and GPL notice are preserved.
  This crate has no executable VM/factory. Circus instruction/system/variable,
  wait, UI event, scenario and save policy remain unimplemented; DC4PHDL cannot
  run. Automatic detection now reports the recognized unsupported profile,
  instead of a generic unrecognized-engine error or a pretend running client.
- Implementation: shared Image's content registry decodes CRXG compression
  1/2/3 with bounded LZSS/zlib, row predictors, palette/alpha conversion and
  origin metadata, independently of a VM. Shared AudioSource paths unwrap
  XPCM mode 0 PCM and mode 5 Ogg through a seekable bounded payload view and
  existing packet decoding; songs are not decoded/allocated in full at open.
  Spectral modes 1/3 remain explicit unsupported errors. GARbro source commit,
  hashes and MIT notices are retained; no original engine host was adopted.
- Compilation: final full-feature desktop release build passed with features
  nscripter,arc-formats (dc4ph/desktop-final-build.log).
  Binary sha256 706f94778ceea33672577a4fc1384990bec3ebf6e429857543ef64a132094812.
  crysirror-circus and crysirror-audio release library builds/tests also passed.
- Tests: MES authored format/invalid-boundary tests 2 passed; XPCM sample/exact
  seek/EOF and malformed/unsupported mode tests 2 passed; CRX tests 3 passed,
  including row predictors, LZSS literals/block tables, palette/alpha/planar
  runs and malformed/bomb rejection. The final full Image release suite passed
  100 tests / 2 ignored, overlapping the CRX/color/cursor tests. Desktop release
  tests passed 19, including explicit recognized-unsupported Circus reporting.
  API docs regenerated/verified (7 passed), fingerprint remains 2663a3650d0f8656;
  host-only API additions are logged. Python tooling passed 200. Failed initial
  XPCM type inference was repaired before accepted builds; no failure is counted
  as a pass. Ignored tests are not counted unless explicitly run below.
- Behaviour/resources: explicitly invoked ignored local-corpus tests decoded
  all 1339 CRXG images, parsed all 536 MES scripts and opened/read/seeked 148 PCM
  and 52 Ogg XPCM resources from the owner's original DC4PHDL directory. Original
  assets were read in place. This is resource/format validation, not whole-file
  reference pixel/audio parity or native game execution. New binary --detect-only
  returns exit 1 with recognized Circus dc4ph / 0xabb6 and explicit missing
  execution support; no game title/frame is claimed. Logs and local executable
  analysis are ignored under target/verification/dc4ph/. No game data/font/key
  was added to the repository, no native executable was run, no window created,
  and no commit, push, bundle was added.

### Native Circus execution and shared capabilities (2026-10-31)

- Implemented: first-class `crysirror-circus` native0xabb6 interpreter retains original operands, labels/metadata, variable banks, branches/calls, resource/wait policy and explicit unsupported errors. A worker-backed thin adapter publishes shared CRX/PCK/cursor/image/layer/UI/audio resources; no original executable, renderer, mixer or composed-screen engine is hosted.0xaab6 remains parser-only and rejects execution.
- Shared implementation: image `pad_to_extent`; explicit scene ImageSampling with clamp/repeat/mirror addressing and a bounded UV contract; wgpu per-sampling texture aliases over the same base texture; owner-scoped `Ui.snapshotLayers` / logical UI snapshots; configurable text color/inset and native press activation; resource `PeData` safe read-only PE32/64 views, bounded RVA/VA slices and immutable SHA256; VFX `screen::pack_screen_emitter` converts existing immutable CPU particle snapshots into ordinary PuppetFrame quads with explicit atlas UVs. UI/particle snapshots are logical engine resources, not a private VM rasterizer.
- Native implementation: bootstrap reaches real name/mail input using shared TextField/Button/Dialog state, normal pointer events and empty-input confirmation; local CLEARNAME/CLEARMAIL preferences commit on a worker. The original `system.dat` two-block21368-byte preference format is imported/written separately, preserving unknown bytes and native read-range layout; ordinary game-slot saves and VM snapshots remain explicit unsupported. Music loop points are read from the exact recognized original executable as data, not guessed or shipped; source43C390 points are decoded frames. Native centibel gain is mapped into common client audio groups. Scene fades now use public externally-clocked crossfades so resource waits do not manufacture native render frames.
- Compilation: release Circus/runtime examples check passed, including the external-clock changes (`check-music-clock.log`). Native release unit tests passed16, with the separate installed-corpus test ignored in that invocation (`tests-settings-music.log`). The installed MES corpus test was then explicitly invoked against the original536 MES files and passed1/1 (`corpus-final.log`).
- Shared behavior: UI snapshot/text-style/press tests3, shared TextField tests4 and interaction test1 passed (`ui-snapshot-final-tests.log`); PE32/64/bounds tests2 passed (`pe-tests.log`); VFX screen packing tests3 passed, including independent common mesh CPU capture without a VM (`vfx-screen-tests.log`). Parent separately ran real shared image-sampling/frame-history GPU tests2/2 and the original20396-entry PCK voice range test; ignored GPU tests are not counted as passes here.
- Installed behavior: a fresh private Save capability plus authored ordinary input (click submit, then confirm empty fields) executes original START/WIN_SIZE through the complete playerinput callback, writes native preferences and enters MAIN. `headless-music1.log` reaches MAIN pc0x1193/op28 after loading TITLEBACK02/TITLECHIP01/TITLECHIP02/EFFSAKURA01 and preparing original music with its42 native loop records. This is an honest first failure at the not-yet-mapped falling-atlas particle effect; a complete visible/interactable title or story is not claimed. The subsequently checked native external frame clock has not yet been behavior-revalidated by this log. No original game process/window was opened and no files were committed.

### Shared screen VFX and native Circus execution progress (2026-10-31)

- Implemented in shared engine crates: CPU screen-sprite declarations publish
  ordinary owner-scoped dynamic Puppet layers from the existing VFX worker.
  The reusable falling/atlas node uses explicit integration ticks and wall time,
  a serializable integer sequence, bounded atlas rows and bottom cropping.
  Completed worker receipts and fenced random-sequence exchanges keep the
  submitted declaration distinct from completed simulation state. Empty screen
  declarations avoid cloning the entire layer tree. Existing/foreign targets,
  unsupported GPU modes and active screen-producer snapshots fail explicitly.
- Implemented in the thin Circus adapter: native28 type1 density0..5 declares
  five ordered depth emitters in one shared system; native25 borrows the same
  process stream and waits for exchange acknowledgment before assigning99/2
  and recording original random history. The virtual millisecond seed is fixed
  at adapter startup, before I/O latency. Native1e/19 performs the original
 356324-byte save-prefix availability probe on a capability worker; it does not
  load original slot saves. Native78/32 preserve group movement/flag data;
  they do not pretend to activate or animate an unsupported group.
- Compilation: release Circus/example metadata check passed
  (`native-effects-check.log`). The latest release native library tests passed
 20 with one installed-corpus test ignored (`native-effects-final-tests.log`);
  that ignored item is not counted as a pass here. Both actual headless examples
  compiled and executed with the normal runtime, without window/audio devices.
- Shared behavior: final VFX tests passed8 library tests and4 CpuWorld tests,
  including one stream across emitters, independent external clocks, completed
  tick accounting, fenced exchanges and birth/reset atlas cells
  (`vfx-falling-final-tests.log`). Runtime screen publication/ownership/cleanup/
  snapshot gates passed5 tests (`vfx-publication-final2-tests.log`); the separate
  default-budget/client-pause receipt test passed1
  (`vfx-sequence-runtime-tests.log`). These are device-free behavior results.
- Installed behavior: fresh private save storage plus ordinary submit/empty-field
  confirmation input executes original START/WIN_SIZE into MAIN. The first probe
  passed native28 and stopped at the then-unmapped save-availability query.
  The corrected second probe reaches MAIN pc0x14f9/op0b at frame806, where title
  group activation still fails visibly (`headless-effects2.log`). The completed
  engine snapshot contains5 emitters/60 particles and five owner1 dynamic mesh
  layers, each with12 parts and one texture. Tick0/epoch0 random state3672816812
  is the completed shared stream after540 draws from virtual startup seed16.
  This establishes actual simulation/publication, not a visible/interactable
  title, story, GPU rendering or60-FPS result.
- Remaining boundaries: title group activation, its20-native-frame layer/UI
  slide and original button policy; immutable freezing of dynamic mesh producers;
  storm first-birth/reset differences, other/concurrent/replaced effect pools,
  nonempty native text/markup and original slot-save reconstruction. No native
  executable was run, no game/font data was committed, and no commits were made.

### 2026-10-31 — shared media clocks, VFX acknowledgements and verified desktop title

- Implementation: MediaInfo/MediaStatus now publish validated display size before
  the first picture and declared duration where available. OMV packet/page hints
  remain unknown duration. Incremental ASF/Theora header callbacks precede frame
  publication. External media clock samples retain engine-owned origin, pause,
  seek and loop offsets; backward samples fail. Decoded audio plus an external
  clock explicitly rejects until audio-master mapping, rather than desynchronizing.
  Remove releases the scoped resource/surface; Stop retains reconstruction debt.
  Retained resource bounds are now16/client32/Runtime, with unchanged128MiB video
  and128MiB audio byte budgets and bounded cancellation leases.
- Implementation: VFX completion queries validate entity lifetime, authored-source
  fingerprint, revision, completed tick and exchange epoch. Declared RNG seeds are
  never returned as ready work. Both screen producer and sequence/exchange
  snapshots explicitly reject missing worker/resource reconstruction. Python
  authoring validation now accepts the actual serde defaults and enforces the
  supported screen/clock/sequence/falling profiles without rewriting declarations.
- Compilation/behavior: shared Media release suite passed18, ignored1 separate
  owner-corpus case (`shared-media-external-clock-tests.log`); public Media
  resource/clock/remove object tests passed2/2 (`shared-media-clock-object-api-tests.log`).
  The earlier shared metadata suite passed16/1ignored; independent real HF logo
  OMV decoding with header-before-picture assertions was explicitly invoked and
  passed1/1,0ignored (`shared-real-logo-metadata-test.log`). No game media was
  copied into the repository. The newly increased capacity test is queued.
- Behavior: VFX object API really waits for CPU worker initialization and exchange
  acknowledgements, rejects same-revision stale declarations, and rejects both
  pending and completed sequence snapshots:1/1 passed
  (`vfx-sequence-snapshot-gate-test.log`). Public API generation/tests passed8/8
  at fingerprint4094eacf6dc444ee; earlier validated stages9e71ed5151241ed2 and
  497d019748fb64d4 remain separately logged in docs/api/CHANGES.md. Python3.11
  full tooling passed201/201 before the last native-only edits; default Python's
  missing tomllib was an environment error, not a passing run.

### Circus original system preference reload26 (2026-10-31)

- Matched the reported original START failure exactly: native pc0x2b1d has
  bytes18/1a/00, opcode18 selector26. It is40EB40 system.dat reload, entered
  on a later boot when the original100/10 preference flag is set; earlier
  fresh-state probes skipped this branch. This was a real missing handler,
  independently reproduced with the prior binary and copied existing private
  preferences at frame41 (`reload26-before.log`).
- Implemented: the native instruction waits for a client-scoped Save worker,
  then replaces both validated original blocks and configured-name buffers.
  Runtime script variables/names/PC remain distinct. A missing file follows
 40E9C0 full first-block reset while preserving the separate2040-byte source
  block. The reset's retained-name music replay uses the existing shared Audio
  path; an empty name emits nothing. No instruction is skipped and no shared
  renderer or second engine is added.
- Compilation/behavior: release Circus library tests passed22 with the separate
  installed-corpus test ignored (`reload26-tests.log`). The new authored tests
  cover pending-PC stability, loaded unknown bytes/names, ordinary script state
  preservation and missing-file reset/source-tail behavior. Further inspection
  of43CE40 corrected its retained-name music replay; final validation of that
  additional branch is recorded separately below.
- Installed behavior: the rebuilt release headless example uses the same
  copied existing preferences, passes START18/26 and follows original boot
  control flow into MAIN; this saved-state run needs no new profile input.
  It next stops visibly at native title-group
  activation, pc0x14f9/op0b, frame786 (`reload26-after.log`). This fixes the
  reported reload wall; it does not establish a working title/menu/story.
  Original files were not changed, no native executable/window was opened,
  no Core/Runtime/public object API changed, and no commit was made.

### 2026-10-31 — Circus implementation and validation

- Circus reload26 final follow-up: inspection of43CE40 corrected the reset's
  retained-name music action to looping replay, not an unconditional stop.
  Empty names emit no request. A previously nonlooping resource acquires its
  actual loop metadata before replay; the completed storage result is retained
  while that second worker is pending. Final release tests passed23 with the
  separate corpus ignored (`reload26-corrected-tests.log`), asserting one reload,
  stable PC/preferences while pending and the prepared100..200 authored loop.
  The latest rebuilt example again passes the user's existing-preferences18/26
  path and reaches MAIN pc0x14f9/op0b, at headless frame803
  (`reload26-fenced-boot.log`). No working title or original slot save is claimed.

### 2026-10-31 — published reload26 fix and current installed-game boundaries

- Compilation: combined desktop release build passed in
  `circus-execution/reload26-player-build.log`. All four managed local launchers
  now have SHA256
  `09c492c843372cec49f5ea5acd0344f433730e7bdb47cded2f2e7fb174df1f80`.
  The prior title player remains retained under ignored verification files.
- Behavior: the normal detected DC4PHDL desktop player, offscreen and with a copy
  of the same existing original system.dat, passes START pc0x2b1d/op18 and
  reaches MAIN pc0x14f9/op0b (`circus-execution/desktop-reload26.log`).
  The next original title-group/menu instruction remains under implementation;
  the reported reload instruction is fixed, not skipped. Original game data
  was not modified and no title/menu completion is claimed here.
- No original executable, window, commit or push was created.

### Circus process11 and reusable Layer/UI motion/coverage (2026-10-31)

- Implementation: owner-scoped retained Layer/UI motion tracks use shared
  `ScalarTween`, with explicit external caller units or automatic engine
  milliseconds, typed query/status, monotonic elapsed, transactional rollback,
  serialized logical state, Put/Remove cancellation and track quotas. Linear
  progress retains f64 precision until its final coordinate conversion. UI
  immutable `WidgetHitMask` samples engine-owned alpha coverage, supports
  explicit strict local edges and threshold, and retains normal keyboard
  behavior. `ImageResource::alpha_region` performs bounded checked extraction;
  UI masks have per-resource, owner and global byte limits. The native VM
  performs neither interpolation nor pixel hit sampling.
- Native implementation: DC4PH0x0b now finalizes its real group and waits in
  process11. Shared motion drives the original20-tick slide and15-tick hover
  scale. Native34[0] belongs to each button record; shared masks use the first
  normal-state image's source-cut origin and alpha>=1. Input helper442D60 /
  4428E0 confirms release activation; retained shared UI events preserve that
  action through asynchronous preparation. Only a real action resolves a MES
  label. Keyboard/controller/page navigation remains explicitly unsupported.
- Compilation/behavior: `cargo check --release -p crysirror-scene` and
  `cargo check --release -p crysirror-circus --features runtime` passed.
  `motion-alpha-final-tests.log` passed9/9, including20 exact coordinates,
  parent motion before hit layout, owner isolation, rollback/restore,
  transparency pass-through, mask validation and delayed event consumption.
  `menu-native-tests.log` passed24 with one separate installed corpus test
  ignored; the authored process11 test cannot fall through on a raw click and
  resolves its supplied real label only after engine activation.
- Installed behavior: using a private copy of the existing original preferences,
  the release headless example waits for completed shared UI motion and presses
  an actual covered pixel at1014,201 on frame830; release activates the native
  `prologue` action. Execution then fails explicitly at MAIN pc0x1512 opcode13
  on frame838 (`menu-title1-boot.log`). No PC, VM flag or native event was
  injected. Inspection also found SCENE_ROOT still hidden by the prior fade,
  so this interaction result is not a verified visible title screenshot.
- Follow-up implementation: original49→425C20→41F2B0/407BC0 explicitly clears
  current UI groups and the old transition draw state. A native ResetScene
  event now maps that cleanup to shared layers/canvas/menu state before new
  background publication, correcting the stale hidden scene/color overlay.
  Its new release fixture and actual offscreen verification are pending;
  later native13, complex animations and original slot restoration remain
  unsupported. No original exe was run and no commit was made. Root owns
  public API generation and its integration validation.

### Circus audio-release contract draft, paused before admission (2026-10-31)

- Static analysis identifies native13's owner-update db100 release and retired
  PCM channel transfer; its200ms refill timer does not drive the fade. Default
 30-step f32 accumulation stops on update29 or30 depending on starting level.
  This is a single target selected by a threshold, not an automatic two-stage
  release. The ignored `audio-release-plan.json` and arithmetic oracle record
  sources and distinguish static simulation from native execution.
- Added device-free Core release spec/handle/actual-receipt contracts and an
  audio owner/controller draft. At the owner's request to free an agent slot,
  the public admission method explicitly rejects unverified release producers;
  native13 remains unsupported. No PCM behavior, native13 progression or
  script/API integration is claimed. `cargo check --release -p crysirror-audio`
  passed at this guarded boundary (`audio-release-paused-check.log`). Remaining
  gain capture, preparation/cancellation fences and required sample tests are
  recorded in ignored `AUDIO_RELEASE_HANDOFF.json`. No commit was made.

## RealLive AIR clip, native cursors and installed after/LB regressions (2026-10-31)

- Implementation: native object screen-space clips now belong to transparent
  identity parent layers; the image keeps its original rect/UV/transform. Parent
  object scale and adjustment offsets also reach child sprites. Shared
  `crysirror-image::cursor` sprite/header kernels prepare Gameexe cursor sheets,
  and existing ImageJob workers own full resource reads/decoding; the native PC
  awaits completion before shared UI cursor publication. No private pointer
  renderer/clock is added. Header reads, source/output pixels and frame counts
  have explicit bounds; source provenance is in
  `docs/reallive-cursor-source-audit.json`.
- Shared Canvas `sample_pixel` provides a bounded opaque-solid profile with
  exact local clips and terminal opaque coverage. Unknown image/text/group,
  transformed, filtered, blended or translucent coverage fails explicitly.
  Native GetDCPixel reads that service and preflights all three RGB destinations.
  DSTRACK nonzero starts use exact decoded-source frame boundaries and existing
  audio seek/loop services. Native optical checks have explicit portable/strict
  policy; portable diagnostics report no authentication or verification result.
- Compilation/tests: initial RealLive release suite 45 unit + 9 bridge passed;
  later targeted release checks passed: shared cursor/header 3, shared Canvas
  sampling 3, native RGB atomic write 1, cursor/query bridges 3 and original
  Gameexe DLL-slot preload 1. The cursor fence test was corrected to gate the
  actual CPU worker pool rather than the unrelated I/O pool, then passed.
- Behaviour: immutable combined player4
  (`a8a1c108b7162dad0161c8a27c9e1f4fb187e2571546019452603a7a3a2ce83f`),
  offscreen Metal: AIR's 400-frame Key logo is complete and centered; logical
  custom cursor 0 is selected. Physical OS cursor presentation was not verified.
  Ordinary Ctrl down/up, Enter and mouse input reach Tomoyo After's complete
  title and Start enters scene628:28 with the June 28 calendar, room and Japanese
  dialogue. Further clicks stop at scene628:33 because `koe/z0629.nwk` is not
  mounted by installed-game launch. Little Busters! reaches its complete title
  (scene9012:541), with the optical step explicitly policy-disabled; NewGame
  stops at scene9012:1204 because configured EF00 slot 1 was not preloaded.
  The original pinned Machine::new DLL.nnn preload policy is now restored and
  independently tested; installed-game behaviour after that change awaits the
  next combined binary. No original executable/DLL or window was run; headless
  playback emitted no sound. Evidence: `target/verification/reallive-air-after-lb/`.

## Circus prologue frontier, shared audio release and mesh freeze (2026-10-31)

- Implementation: shared `AudioReleaseSpec/Handle/Info` transfers an actual live
  decoder/cursor to an independent nonlooping tail, freezes current native group
  attenuation and advances native f32 db100 release on explicit owner updates.
  Public Audio release methods and Runtime snapshot/restore gates use that actor.
  Shared `Paint::Mesh` freezes an owner-completed `PuppetFrame` and reuses the
  existing mesh renderer in Canvas snapshots; live surface/history dependencies
  fail explicitly. Circus maps native13, stage scale/pivot declarations and1c
  transient checkpoints to these engine-owned resources and retains native policy.
- Validation: audio release/all audio release suites **150 passed, 0 failed,
  4 ignored**; public Audio/Runtime PCM/worker/snapshot acceptance **2 passed**;
  shared frozen Canvas **9 passed**; explicitly executed Metal frozen Canvas GPU
  suite **9 passed, 0 failed, 0 ignored**. Native tests **29 passed, 0 failed,
  1 installed-corpus test ignored**. GPU assertions include source removal,
  changed mesh content under the same source ID, serialization, immutable resource
  rejection and later outer opacity. Audible device output was not verified.
- Actual installed behavior: normal title pointer press/release passes native13
  and fade capture, enters `DC4_SEN20181213B`, completes initial stage/placement
  declarations and1c's actual shared visual/resource checkpoint. It stops at
  **pc0x83e, opcode18 selector4 arg1=1**, the mandatory original autosave0.
  `native13-checkpoint-installed-boot.log` records this headless run. This is
  prologue initialization evidence, not rendered dialogue or complete playability.
- Remaining: native autosave requires the complete **356324-byte** original body
  plus its **128x72** thumbnail; no fake/JSON/zero-filled original save was written.
  Original1d/slot restoration, later dialogue, PCK sequence playback and numerous
  native profiles remain unsupported. Checkpoint UI paints are not reconstructed
  interactive UI. Exact layout and remaining fields are in the local ignored
  `target/verification/circus-execution/CIRCUS_AUTOSAVE_HANDOFF.json`.

## First-class native RealLive DLL semantics and shared spatial extraction (2026-10-31)

- Implementation: `crates/crysirror-reallive-dll` replaces the former nested
  helper; package, imports and workspace dependency now use that first-class
  name. Original MPL-2.0 licence, pinned hashes and the previous adoption
  manifest remain beside the maintained source. This is not only a move:
  EF00 cubic B-spline/perspective sampling and PT00 f64 geometry, matrix and
  parabolic kernels now live in public `crysirror-vfx::spatial`; native argument,
  bank, RNG, addressing, actor/dungeon and numeric-profile policy stay native.
  The private PE parser is removed in favour of shared Resource `PeData`.
  DT00's `image` means PE table/data memory, never a rendered screen. Production
  environment-selected trace/dump filesystem writes and the test-only fake
  game_fs package are removed. Source/extraction ledger:
  `crates/crysirror-reallive-dll/SOURCE.json` and `SOURCE_NOTES.md`.
- Compilation/tests: one release Cargo invocation compiled VFX, native DLL and
  RealLive crates. Shared spatial tests 3 passed; EF00 golden native-bank and
  unknown-DLL tests 2 passed. The release DLL test binary then explicitly ran
  both otherwise-ignored owner-file cases against original PT00.dll/dt00.dll
  static tables: 2 passed. Those calls execute Rust translations only, not PE
  code; they do not establish full minigame gameplay or hardware behaviour.
- Behaviour remains separate: player5 attempted installed after/LB voice mounts
  but both failed at the existing provider-mount quota before native boot.
  The new extraction and a revised shared mount budget require the next
  combined player for installed-game validation. Logs are under
  `target/verification/reallive-air-after-lb/`.

RealLive installed follow-up, immutable player6
(`00f1b6ef71ac4471cc45e0e37d16c6d3456e91f475bedea977eff3c8e3e37304`):
Little Busters! NewGame now executes 4,176 EF00 calls through the first-class
DLL/shared spatial path, advances through scene8516 and reaches scene513:51.
At 7,000 frames the actual GPU capture shows the first Japanese dialogue line
in its native message window. The black background is the script's explicit
KURO image, not a claim of a missing render. Further ordinary clicks stop at
scene513:58, voice51300042; after similarly stops at scene628:33, voice62900017.
Both now fail with `Not a directory` while listing a voice-container root
backed by an original archive file. The shared VFS fix needs the next binary;
these failed voice preparations are not successful voice-playback evidence.

### Codex 2026-10-31 — new native program foundations and shared installation media

Implementation:
- Added first-class `crysirror-system4` AIN v5 program reader. Legacy MT19937-1998 byte XOR is a reusable `crysirror-resource::legacy_stream` kernel; AI2 has bounded exact zlib framing. Original byte strings, functions/globals/structures/HLL declarations/switches and function-type size metadata are retained. The original reader does not use that size metadata as a serialized span; an initial invented equality check was removed after actual-game validation exposed it. Native operand-layout decoding is separate from instruction execution.
- Added first-class `crysirror-system-nnn` bounded original SPT data/instruction reader. The generic rolling 32-bit XOR/add transform lives in shared resources; the adapter data preserves word PCs and table/text addresses.
- Added shared read-only `windows/msi` resource archive and `InstallerCatalog`: MSI File/Component/Directory/Media metadata maps embedded CAB members to installed paths. MSI/CAB readers are local to worker invocations, decoded members use common memory leases and cancellation checks. External cabinets require an explicit future scoped dependency mapping; no installer action, COM/native EXE execution or filesystem extraction occurs.
- These two native crates do **not** yet provide executable VM factories or playable adapters. System 4 stack/heap/calls/HLL mappings and System-NNN expressions/control flow/native configuration/syscalls/save semantics remain unfinished. Recognized instruction layouts and successfully read game assets are not integration completion.

Compilation and local tests:
- `cargo check -p crysirror-system4 --release` passed. AIN authored tests: 2 passed, 1 owner-data test ignored by default; latest full instruction-layout validation is recorded separately when actually run.
- Shared MSI authored tests: 2 passed, 1 owner-data test ignored by default. Native System-NNN authored tests: 2 passed, 1 owner-data test ignored by default. No game fixtures, fonts, extracted files or original code are distributed.
- Python 3.11 tooling: 201 passed (`target/verification/majiro/python311-final-tests.log`).

Behavior:
- Explicitly executed the owner AIN test against `game-rance7/GAMEDATA`: parsed version5, 5,750,996 code bytes, 6,580 functions, 219 globals, 209 structures, 10,281 strings, 92,120 messages, 15 HLL libraries and native main function16. This establishes the data reader, not gameplay (`system4-rance7-program3.log`).
- Explicitly executed the owner MSI test: read 385 installed files and decoded original `mugen2/spt/preprologue.spt` directly from the embedded CAB. Explicit System-NNN owner test then validated 11,248 words, 78 messages, 176 strings and first original instruction PC0x20/identity0x66660007/code0x33330051 (`systemnnn-owner-program.log`). No command was acknowledged or executed by that data-only test.

## Majiro original MJO execution and shared Canvas frontier (2026-10-31)

- Implementation: first-class `crysirror-majiro` owns bounded X/V1.000 MJO
  instruction/function/jump validation, native stack/local/control semantics and
  pending syscall receipts. MJO's fixed CRC-table XOR is separate from RCT image
  passwords. Native resources are read on bounded capability workers; there is
  no original EXE execution, hosted engine, private renderer/text algorithm or
  mixer. The thin facade maps reviewed true-colour page creation into shared
  Canvas state; actual native allocation is opaque 24-bit BGR, not the public
  wiki's 32-bit description. Native auto-mode configuration and explicit release
  startup-mode policy stay in the adapter. Source pins/licences/module SHA and
  static inspection boundaries are in `crates/crysirror-majiro/SOURCE.json`.
- Compilation/independent validation: release native tests passed5/5; the
  authored original-MJO-to-shared-Runtime Canvas bridge passed1/1. It verifies
  real engine resource dimensions and preservation of an unknown following
  syscall at its native PC. Final tests strengthen its opaque-black assertion;
  that final assertion awaits the next compiled test binary. No visual/headless
  screenshot or physical device output is claimed by these CPU tests.
- Actual owner behavior: explicitly executed local owner tests passed2/2 in
  `target/verification/majiro/owner-runtime-final.log`. All181 original MJO modules
  parse, covering496549 instructions. The unchanged `start.mjo` entry0x203b runs
  through its first native policy calls under shared Runtime and a read-only
  SHA-guarded native module, then stops after15 instructions at **pc0x213f,
  line446, syscall0xc6282f19 `rand(100)`**. Its original pending operands/PC remain
  intact; engine_api_calls=0/pages=0 for that game run. This is native execution
  evidence, not a rendered title or completed Majiro integration.
- Installed-game boot: the optional desktop `majiro` feature has an original
  ARC/ENV/start helper, shared private archive mounts and root/bin executable
  metadata lookup, with bounded header probes and capability-root checks before
  reading. Conflicting patch providers and unlinked builds fail explicitly.
  Parent coordinates final desktop compilation/public API references; no pass is
  claimed for the combined player at this entry. Cross-module linking, arrays,
  full value/call profiles, text/control, image/audio/video and saves remain open.

RealLive tooling follow-up: the Python authoring validator now accepts exactly
`optical_disc_check = disabled_for_native_host | strict`, matching native Config;
unknown values and non-string substitutes fail. Native API schema tests 8 passed.

### 2026-10-31 — RealLive / System4 / Majiro implementation and validation

- AIN v5 full code-layout/entry-boundary validation explicitly passed on Rance7: 1,062,488 original instructions, main PC0x73a. All 6,580 function entries lie on decoded boundaries; validation uses a bounded bitset rather than an unbounded tree (`system4-rance7-layout-final2.log`). The initially omitted renamed EOF/MSG metadata rows were fixed from their original declarations before this pass. Authored AIN tests remain 2 passed; the actual owner test was explicitly run, not counted as an ignored pass.

- Majiro follow-up: the thin native RNG policy now calls the existing shared
  `SequenceRng::msvc` kernel, with original `srand(time(0))` Unix-second seeding
  captured once by the startup I/O worker. Signed modulo and zero-bound error
  policy stay native; state is explicit seed/accumulator/draw-count data and can
  be captured/restored independently, with no global Random. Release tests now
  pass1 RNG/capture +1 shared Canvas (including exact opaque-black sampling)
  +5 native/ENV tests. The two explicit owner tests also pass again in
  `owner-runtime-rng.log`: actual startup advances33 instructions and2 CRT draws,
  then stops at **pc0x214, line58, syscall0x111bd910 `get_effectspeed`** inside an
  original script function. Native ENV/settings getter mapping remains missing;
  engine_api_calls is still0 for the real game and no title is claimed.

RealLive final player7 behavior and read-only next-wall diagnosis:
- Immutable player7 SHA256 `3af2da91bce72920c9aa54651f8667e13e1587b215d74c574f96ed6081dc51ea` crossed actual after/LB voice through shared NWK decoding/mixing after the shared mounted-file directory fix. Headless GPU only; no original EXE/window or physical OS cursor claim. after at frame5000 reaches scene628:43 Pause17 without error,4 acknowledged KOE operations,3,120,384 mixed PCM frames,peak1.3376832,rejected0/underrun0 (active1 is looping BGM). LB at frame7800 reaches scene513:59 Pause17 without error,1 acknowledged KOE,5,226,240 PCM frames,peak0.9996352,rejected0/underrun0,active0 after voice completion; screenshot shows original May13 bedroom narration. EF00 original execution retains4,176 successful samples. Evidence/screenshots/logs stay in ignored `target/verification/reallive-air-after-lb`.
- Further normal LB clicks at frame10000 reach SEEN9030:82 element133 byte1425, native `1:72:1000` overload1 `objOfFile(110,strS1001,1)` and visibly fail loading `CGMS12?030`. Original preceding string branch appends that suffix. Complete pinned RLVM lookup strips `?` only for the resource filename, then SDL graphics applies tone curve `suffix/10-1`:030 selects table2. It loads a3×256 RGB lookup from configured TCC and preserves alpha. Existing shared G00 atlas/pattern expresses geometry, not this lookup; current ImageEffect has no arbitrary RGB LUT input. This is the explicit next shared capability/native policy gap; no guessed suffix stripping or production expansion was made. RealLive source is frozen.

- Player7 already explicitly consumed after/LB first voice samples through shared NWK/OVK mounts and mixer. after reached scene628:43 with visible room/Tomoyo/dialogue. LB reached scene513:59 after the voice naturally ended. LB later stops at original `CGMS12?030`: the pinned native suffix selects tone-curve table2, not an atlas frame. Arbitrary RGB LUT support remains a shared-engine gap; the suffix was not stripped to fake success.

- Majiro ENV/settings follow-up: detection records the actual unique ENV resource
  path. The startup worker reads it and an explicitly labelled `portable_client`
  settings file from that client's Save capability. Native REG_SZ decimal text,
  ENV defaults and current speed globals remain distinct; generic reg_write does
  not overwrite the current speed global. Dedicated setters stage a candidate,
  call shared begin_write_atomic, retain their native PC while Pending, and only
  publish/ACK after Ready. Strict original registry import is unsupported and
  diagnostics report import_performed=false; this is not an original game save.

Codex follow-up validation (same work, actual later results):
- AIN v5 full code-layout/entry-boundary validation explicitly passed on Rance7: 1,062,488 original instructions, main PC0x73a. All 6,580 function entries lie on decoded boundaries; validation uses a bounded bitset rather than an unbounded tree (`system4-rance7-layout-final2.log`). The initially omitted renamed EOF/MSG metadata rows were fixed from their original declarations before this pass. Authored AIN tests remain 2 passed; the actual owner test was explicitly run, not counted as an ignored pass.
- Shared MT19937-1998 profile matched an independently compiled, unmodified reference C oracle: 1 passed (`legacy-stream-tests-final.log`). The C oracle is research tooling, not an engine dependency.
- Runtime VFS quota test passed at 256 mounts per client. Separate mounted-archive-file directory/read/client-isolation test passed. Late-installed static default fonts and shared dynamic font publication passed the explicit runtime-system-font regression; no font file was committed.
- API reference regenerated and all 8 checks passed at fingerprint `d1da391dfe839e4a`, including the partial Majiro config (`target/verification/majiro/api-final-verified.log`).
- Combined player7 compiled in release and was copied immutably: SHA256 `3af2da91bce72920c9aa54651f8667e13e1587b215d74c574f96ed6081dc51ea`. Normal installed-game detection selected Majiro, all six original ARC mounts, 800x600 ENV and `bin/暁の護衛.exe` as read-only ABI metadata. Normal headless boot then reached the real unsupported rand(100), start.mjo PC0x213f/line446/hash0xc6282f19; no page/title/engine API call had yet occurred. A short 100-frame run was still loading, and was not called successful integration. The longer actual run is `desktop7-native-1200.log`.
- Majiro follow-up: the thin native RNG policy now calls the existing shared
  `SequenceRng::msvc` kernel, with original `srand(time(0))` Unix-second seeding
  captured once by the startup I/O worker. Signed modulo and zero-bound error
  policy stay native; state is explicit seed/accumulator/draw-count data and can
  be captured/restored independently, with no global Random. Release tests now
  pass1 RNG/capture +1 shared Canvas (including exact opaque-black sampling)
  +5 native/ENV tests. The two explicit owner tests also pass again in
  `owner-runtime-rng.log`: actual startup advances33 instructions and2 CRT draws,
  then stops at **pc0x214, line58, syscall0x111bd910 `get_effectspeed`** inside an
  original script function. Native ENV/settings getter mapping remains missing;
  engine_api_calls is still0 for the real game and no title is claimed.

- Player7 already explicitly consumed after/LB first voice samples through shared NWK/OVK mounts and mixer. after reached scene628:43 with visible room/Tomoyo/dialogue. LB reached scene513:59 after the voice naturally ended. LB later stops at original `CGMS12?030`: the pinned native suffix selects tone-curve table2, not an atlas frame. Arbitrary RGB LUT support remains a shared-engine gap; the suffix was not stripped to fake success.
- Majiro ENV/settings follow-up: detection records the actual unique ENV resource
  path. The startup worker reads it and an explicitly labelled `portable_client`
  settings file from that client's Save capability. Native REG_SZ decimal text,
  ENV defaults and current speed globals remain distinct; generic reg_write does
  not overwrite the current speed global. Dedicated setters stage a candidate,
  call shared begin_write_atomic, retain their native PC while Pending, and only
  publish/ACK after Ready. Strict original registry import is unsupported and
  diagnostics report import_performed=false; this is not an original game save.
- Final release validation passed4 unit +1 shared Canvas +5 native tests.
  These include Pending/Failed write receipts, a stale receipt, one admission,
  unchanged PC/operands/current values on failure, portable override/restart,
  foreign scopes and duplicate/corrupt JSON. A first failure-injection assertion
  inspected only anyhow's outer message; the test now checks its complete cause
  chain and passes. Local owner tests additionally passed2/2 in
  `owner-runtime-settings.log`: the original script advances40 instructions,
  completes an actual shared storage commit, and stops at **pc0x23e, line60,
  syscall0x81ce0485 `fast_mode_disable`**. It still has no Canvas/title image.
  That native call also conditionally clears CapsLock through original keyboard
  state; no fake void ACK or private keyboard controller was substituted. Shared
  keyboard lock-state input/policy is the next capability requirement.

### 2026-10-31 — System4 implementation and validation

System4 native VM implementation (first-class crate, source frozen):
- `crysirror-system4` now contains a bounded original AIN v5 stack/heap/page executor, native i32/f32 bit values, slot identity/reference counts/sequences, two-word variable references, original argument/return convention, and resumable constructor/destructor/array continuations. Startup allocates all global storage, applies initializers/references, runs original function `0`, then global constructors and main. Lazy member reads and sequential struct-array construction preserve mutation and allocation order; local/copy identity is reserved before child allocation. No C engine, renderer, text layout, image/effect/audio mechanism or private logging system was adopted.
- The actual VmAdapter port owns capability reads, AIN decode/layout validation, heap allocation and native instruction batches on shared TaskPool workers. Owner completion uses existing HostServices clock for GetTime/Sleep. Unknown HLL/syscalls retain native PC/arguments; no factory is registered, no title/playable integration is claimed, and serialized/framework/original saves remain explicitly unsupported. SOURCE.json/README/NATIVE_SEMANTICS retain GPL2+ provenance, fixed commits and exact source hashes.

System4 release compilation and validation:
- `cargo check --release -p crysirror-system4` passed. Final `cargo test --release -p crysirror-system4` passed **2 AIN +13 native VM/bridge/worker tests**; two installed-game tests remain ignored by default. Tests cover native references, constructor/destructor order, array trees and sequential object identity, local/copy allocation order, HLL cleanup without destructors, same-PC pending capture, unknown-op failure, real shared-clock deadlines and worker/owner thread ownership. Final log: `target/verification/system4-vm-freeze-verified.log`.
- Explicit installed Rance7 bootstrap test separately passed1/1 (`system4-rance7-vm-final-verified.log`). Original OutputLog.dll was inspected statically only (SHA256 `8c023f38776994db5c7cc36a5ce409b433efc251a879fcf832b607b62fee04c8`): reviewed Create RVA0x1000 queries an already-loaded DPLogViewer.ade/GetInterface; absent interface returns -1. The native adapter takes only that verified disconnected branch, twice in the actual constructors. No original DLL/EXE was loaded or executed; no game bytes/fonts were added.

System4 actual behavior and next shared boundary:
- Actual original Rance7 initialization executes1,540 native instructions and stops at `CSactSystemLog@Output`, PC0xa4a, original CALLSYS6 (`system.Output`), with the original string still on stack,3 call frames,7,325 live heap slots and212,108 accounted values. Main has not yet been entered. Repeated pending polling/capture preserves the exact PC and stack; no GPU/audio result is claimed. Evidence: `target/verification/system4-vm-evidence.json`.
- Next requirement is a client-scoped HostServices logging operation backed by existing EngineContext.log state/limits, also usable without System4. CALLSYS6 must emit the original encoded text through that shared operation and leave the caller's following S_POP responsible for native string release. Connected named OutputLog buffers/persistence, if later supported, must likewise be shared engine resources. Shared source remained frozen; this missing API was not replaced with a second VM log or silent ACK.

### Codex 2026-10-31 — final local publication and truthful native boundaries

Implementation and compilation:
- The final combined player9 release build passed, immutable binary `target/verification/majiro/player9`, SHA256 `3af4e6303e6dd320afea76fd89009bd4d3143333dab83a1c3f3d8c804140973b`. The four existing managed local launchers now atomically point to player9. No commit or push was added.
- Public API/config reference passed all 8 tests at `61f9f936ff0989a4`, including explicit Majiro ENV and portable-client settings policy. Final Python tooling run: 201 passed (`python311-final2-tests.log`). `git diff --check` was clean.
- System-NNN partial machine authored tests: 3 passed; native program authored tests: 2 passed, one owner test ignored by default. The owner test was explicitly executed afterward and passed its **partial VM boundary assertions**, not a gameplay acceptance test.

Actual behavior:
- Root independently ran/viewed ExE player9 via normal installed-game bootstrap and original Start input: original first dialogue, last_tag=p/click_waiting=true/error=null, actual shared PCM consumption, no window or audio output device. This does not establish full-game, save or audible playback acceptance.
- Root ran Majiro player9 normally from the unmodified installed directory. Shared atomic persistence actually wrote the portable named EffectSpeed value; the original script then failed visibly at start.mjo PC0x23e/line60/hash0x81ce0485, before any game page/title. No CapsLock operation was swallowed.
- System-NNN owner media probe executed eight original internal/metadata instructions and retained the original story name, then explicitly stopped at original IF PC0x41/code0x55560002. Expressions, native resource/graphics/audio mapping and a playable factory remain unfinished (`systemnnn-owner-final2.log`).

Shared-production scope is frozen according to the earlier Claude coordination. The next concrete missing capabilities are shared keyboard-lock policy/state (Majiro), a client-scoped admission/completion path into the existing EngineContext log sink (System4), arbitrary RGB tone-curve LUTs (RealLive LB), and complete original Circus save-body/thumbnail/restore mapping. No private VM renderer, image algorithm or second logging implementation was introduced to evade those boundaries. Original save support has not been claimed for the new partial profiles.

RealLive RGB tone curves: shared implementation after owner resumed shared work:
- Added reusable `crysirror_image::rgb_lut::{RgbLookup,RgbLookupProfile}` and `ImageEffect::RgbLookup`, automatically accepted by existing file/Asset and immutable-resource ImageJobs and public Bitmap.beginEffect. Three arbitrary256-byte RGB tables transform all straight RGB (including hidden RGB at alpha0), preserving alpha exactly. Premultiplied profiles explicitly fail: ImageSurface/resources have no association metadata. Processing is immutable, row-cancellable and capped at4,194,304 pixels/768 table bytes; shared worker admission/memory reservation remains in force. No native LUT/compositor implementation exists.
- The RealLive adapter retains native `?NNN -> NNN/10-1` selection, case-resolved configured dat/TCC source and a bounded immutable first-use tone-bank snapshot. TCC reading/metadata/source fingerprint run on a bounded worker; shared ImageJob owns decoding and RGB execution. Original object creation stays pending at its PC until Ready, then invokes original creation semantics with original atlas/pattern metadata. Each variant gets its own immutable shared publication, never a reused surface slot; cache limits are32 variants/128MiB plus<=1MiB TCC metadata. New publication uses HostServices.publish_image_resource, retaining RGBA Arc/resource lease without owner PNG/scan/hash/copy. Source hashes/provenance and limits: `docs/reallive-tone-source-audit.json`; generic API notes: `crates/crysirror-image/RGB_LOOKUP.md`.

RealLive RGB tone curves: actual release validation:
- Final targeted tests passed8/8: shared arbitrary-table/alpha/strict-profile/cancellation4; native suffix/TCC1; native pending/immutable variants/bad-TCC bridge2; independent runtime file worker/client isolation/publication/release1. Native test removes the TCC after first load and verifies a second variant uses the retained original bank, both pixel results remain distinct and the first stays unchanged. Invalid TCC does not install an object or acknowledge the original opcode. Runtime rejects foreign input/publish/release, rejects premultiplied admission, and verifies a frozen Surface retains pixels after valid release. Logs under ignored `target/verification/reallive-air-after-lb`: `tone-straight-unit-final.log`, `tone-native-published-final.log`, `tone-runtime-published-final.log`.
- New combined-player LB behavior remains to be tested separately; these independent passes do not claim that the installed game has crossed SEEN9030:82.

## 2026-10-31 — shared host capability extraction after owner lifted coordination freeze

Implementation:
- Owner cancelled the earlier05:50 Claude restart/freeze. Native Output and Engine.log now use one bounded client-scoped canonical LogBook; existing App deque is a derived view. Engine.logs exposes bounded records with exact decimal-string u64 metadata. Owner append returns an actual commit receipt; no native log file/device output is implied.
- Added shared logical Caps/Num/Scroll state/revisions. Physical import is unknown and control unsupported; patch validation is atomic, client scoped, and only a committed Runtime restore resets session locks. Majiro fast_mode_disable maps the reviewed depth/zero result and logical CapsLock clearing, retaining Num/Scroll. Explicit physical policy fails.
- Added an actual local civil-time dependency on cancellable host workers, distinct from virtual clock. Added immutable client-scoped ImageResource publication/release with unique generation paths and retained resource leases; no owner PNG encoding or pixel copy. Frozen frames retain released resources. Runtime capture/restore rejects live publications pending reconstruction support.
- System4 CALLSYS6 CP932 Output now commits to the common log before native acknowledgement, preserving the native string result/stack ownership. No System4 factory or native presentation is claimed. Shared image format contexts expose cancellation checks for decoders.

Compilation:
- Release check of crysirror-runtime/crysirror-engine/crysirror-wgpu passed (shared-capabilities-final-check.log); Majiro runtime release check passed (logical-lock-check.log).
- Combined release desktop build passed with nscripter,arc-formats,circus,majiro features. Immutable player10 SHA256 is913f4b795b1b38f5cc9920ea5d9fc8cc84fb353cc290eabba90970f9b3a7d45c. Four managed local launchers were atomically refreshed only after confirming their previous player9 identity. This binary does not register unfinished System4/System-NNN factories.

Behavior validation:
- Independent host_services3/3 passed: canonical native/public log, scope/atomic logical-lock policy/restore boundaries, and actual civil worker completion. Core log ring test1/1 passed. These tests need no native game or hardware device.
- Actual Rance7 constructor with shared Runtime committed2 Output records and advanced past0xa4a to unsupported SACT2.Init at0x6f6 after1,586 original instructions. Original HLL call stays pending; main has not been entered. Explicit ignored owner test was run and passed1/1 (system4-shared-log-owner2.log).
- Explicit Majiro owner tests passed2/2:181 original modules/496,549 instructions parsed; Runtime advanced past fast_mode_disable0x23e to unsupported syscall0x0c070535 at0x24a/line62 after42 instructions. Depth1/shared API completion confirmed, original next syscall not acknowledged (majiro/logical-lock-owner.log). No title or full game claim.
- Python tooling tests201 passed (shared-capabilities-python.log). Shared log/input/civil Rust API tests and generated reference validation are distinct from game behavior. RealLive LUT tests and installed LB behavior are recorded separately below/by their owner.

### Circus original save-body state: audited partial live model (2026-10-31)

Implementation:
- Added lossless original356324-byte slot-body codec (`native_save`) and a separate `State.native_live` model. Initial body bytes follow40EC50; all150x960-byte live sprite records follow complete444BC0/444180 constructor defaults, including original nonzero scale/viewport/visibility and integer clipping fields. No generic renderer or image algorithm was added to the VM.
- Successful native operations now retain both1100-byte reset banks,40D240 live selectors233/241, original configured-name copy order, full14-byte read-history fields/counters/contiguous extension/dedup, bounded random history, completed shared texture backing/logical sizes, sprite transforms, placement rows, music filename/id and voice timing. Sprite texture ownership is semantic metadata; host/device identities are never serialized. Imported opaque slot bytes remain lossless.
- Transient1c capture now additionally constructs original native body/sprite snapshot bytes from that live model. Unaudited operations record explicit coverage gaps; `admitted_checkpoint` rejects any gap. This remains an internal partial native image, not a writable original save. The forced autosave operation has no success acknowledgement, no skipped PC and no fabricated body/PNG.

Compilation and behavior validation:
- `cargo check -p crysirror-circus --release --features runtime` passed. Native slot codec5/5; newly added live defaults/banks/names/read-history/transforms/random tests4/4; added music/placement/voice-memory and metadata-snapshot validation2/2 passed. The freshly rebuilt release unit-test binary then passed23/23 existing native-machine regression tests after the State metadata hooks. Logs: `target/verification/circus-execution/native-save-codec-tests.log`, `native-live-tests.log`, `native-live-new-tables-tests.log`, `native-live-machine-regression.log`. Tests use authored bytes; they do not establish original save reconstruction.
- A newly built actual headless engine accepted original title press/release and executed1,563 native instructions intoDC4_SEN20181213B. At frame876 it stopped at the unchanged forced autosave0, pc0x83e/opcode0x18/selector4. The native1c checkpoint retained1,082,140 bytes of logical/native metadata and172,372 bytes of completed shared resource declarations. Inspection listed43 conservative opcode coverage gaps. This binary precedes the last audit narrowing for13/47/70, native music-stop id255 correction and logical opcode11 random-history counter fix; no new game progress is inferred from those changes. Evidence: `native-live-installed-boot.log`; exact commands and remaining offsets: `CIRCUS_AUTOSAVE_HANDOFF.json`.

Explicit remaining boundaries:
- Native49/425C20 sprite/resource resets and animated final-state metadata;4f/425F50/425DC0/423070 scene activation, object740/908/904 and conditional string fields;3c background-root reset and resource alias/presentation lifecycle;28 falling-wrapper saved policy;4c/71 native voice/SE filenames/loops;ab backend1 saved movie tail. The live image cannot yet be admitted as a complete slot, and original1d/slot restoration remains unsupported.
- Original save thumbnail4250B0 enables6276/6277, calls4226B0(1), enables native layer1 and stops the render loop at native index80. Native UI at70 has additional suppression/selection rules, including5970/2599 under a2=1. Generic scene plus all UI is not equivalent. ImageInput::Raster consumes image resources; MeshFrameHistory captures a single mesh. A client-scoped asynchronous capture of the proven selected complete snapshot must reuse existing desktop materialize_snapshot/read_snapshot and shared resize/PNG facilities before128x72 thumbnail persistence can be accepted. No private screenshot renderer was added.

## Shared API reference final validation

- Final generated API reference validation passed8/8 after adding shared host data and opaque Circus NativeLiveState; current fingerprint f59d9a19867f34c9 is logged in docs/api/CHANGES.md (shared-capabilities-api-final2.log). Python tooling rerun after regeneration passed201/201; git diff --check passed. No commit, push was performed.

- Final small native11 correction separately passed1/1 in release: resetting the original random-history count also clears its logical Vec, while original native raw history tail is retained. `native-live-random-reset-tests.log` records this current-source result; no further game behavior was claimed.

RealLive LB actual GPU validation of the shared RGB LUT fix:
- Immutable player10 SHA256 `913f4b795b1b38f5cc9920ea5d9fc8cc84fb353cc290eabba90970f9b3a7d45c` was verified locally and run through normal installed-game detection/bootstrap, original Seen, voice mounts and ordinary Ctrl/mouse down-up input. Apple M5/Metal offscreen capture created no window or audio device. At10,000 frames LB crosses the former SEEN9030:82 wall and reaches scene513:91, showing the May13 dormitory corridor and original Japanese narration. At16,000 frames it reaches scene513:203, showing the cafeteria and original 【理樹】「…恭介は！？」 dialogue. Both screenshots were viewed; both reports have error=null and normal Pause17 input wait. No new unsupported native wall was observed within this bounded run.
- Native tone-image cache is ready2/pendingfalse at both10k and16k; repeated original object-file calls rose7→21 without new variants. The16k report acknowledges4 KOE calls and actual shared PCM mixing11,523,840 frames (peak1.7876658,rejected0,underrun0; active1 at final capture). This is decoder/mixer consumption, not a hardware audio-output claim. Reports/screenshots/logs are ignored local evidence under `target/verification/reallive-air-after-lb/lb/player10-story-{10000,16000}*`. Actual shared publication byte/count statistics are not exposed in player10 inspection; ordinary asset-cache resident_bytes was not misreported as LUT-resource memory. The native count2 remained stable; source limits32 variants/128MiB RGBA/1MiB TCC still apply.
- Pixel execution, immutable RGBA, publication identity/lease and renderer access belong to shared engine services. The native adapter retains filename/table choice, original atlas/pattern metadata, pending opcode and bounded policy state. Runtime snapshot/restore rejects live image publications, and RealLive checkpoint/original-save export remains unsupported; ephemeral published paths/Arcs are not persistent save identities. Future save support must reconstruct these shared resources from original native resource/table policy. Production source is frozen after the verified tests; no additional game runs, commits were made.

### System-NNN execution and shared Black Cyc formats (2026-10-31)

Implementation:
- First-class `crysirror-system-nnn` now independently expresses bounded postfix
  i32 expressions, reference-preserving assignment/compound/unary operations,
  native positive-only conditionals, IF/ELSIF/ELSE/WHILE/block control, exact
  original operand consumption and instruction-local variable commit. Direct
  and postfix constant profiles preserve their distinct original decoding.
  Native user calls/control boundaries retain pending receipts and original PCs.
- Native encoded-name tables retain empty comma slots and byte names. Explicit
  Legacy1000/Expanded2200 profiles preserve original persistent/nonpersistent
  ranges and actual configured NewGame initial data. Unresolved system/calculated
  variables and Layer-affecting writes fail visibly. No System-NNN runnable
  factory, selection UI, private image/text/renderer/audio engine or original
  save compatibility was introduced. Unlicensed SystemNNN source was used only
  for reviewed format/behaviour facts, independently implemented in Rust.
- New public MIT `crysirror-formats-blackcyc` implements Resource ArchiveFormat
  GPK/GTB and VPK/VTB, ImageFormat DWQ, and FileFormat VAW/WGQ→standard WAVE/Ogg.
  Root workspace changes are only its member/dependency rows. Companion reads
  use scoped resolvers; entry readers are checked subranges, not archive copies.
  Index/name expansion, placement, dimensions, mask palettes and row runs are
  bounded. DWQ preserves plain RGB versus packed BGR and top-down rows; loops
  use ImageDecodeContext cancellation. VAW delta output reserves a shared lease
  before allocation, rejects truncated bits and feeds existing shared Audio.
  Over-2GiB/64-bit-footer archive variants and unsupported image layouts fail.
  GARbro's four complete source files were inspected at real MIT commit
  `b09ee4570ccb1daf6ac56710ee8934dc0b8baeb0`; hashes/licence remain in the crate.
  Public registries are callable independently of a VM; Runtime auto-registration
  and desktop/System-NNN playable support are not claimed for player10.

Compilation and local validation:
- Release checks/tests compiled both crates. Final authored tests passed13:
  Black Cyc3, System-NNN expressions/config5, machine3 and program2. Two
  installed-data tests are ignored by default and were explicitly executed
  separately, each passed1/1. `target/verification/systemnnn/final-authored.log`.
- Black Cyc owner test opened all6 GPK and35 VPK companion tables. All1625 DWQ
  decoded:1122 PNG8A,464 JPEG5,35 plain BMP0,2 packed BMP+mask3A and2 BMP+mask2A.
  Five actual decoded profile images were inspected. The 2A source is the game's
  own `ta_error.dwq`/UNKNOWN DATA artwork, not a CrySirror fallback. Original
  entry names/encoded hashes/decoded RGBA hashes/alpha ranges are recorded in
  `owner-image-profile-evidence.json`; actual mask extrema are3A0..252,2A0..255.
-156 selected VPK entries actually decoded through shared Audio.open_asset,
  producing2,555,904 finite PCM frames, peak0.9739774. This exceeds header-only
  unwrap validation; no audible output device, playback/seek/loop acceptance or
  complete voice-corpus decoding is claimed. Evidence:
  `target/verification/systemnnn/owner-blackcyc-final.log` and `dwq-captures/`.

Actual original-script boundary:
- Shared InstallerCatalog reads the original MSI without installation/extraction.
  Its `init2/game.fxf` has varInitFlag=1; `var.fxf` has1000 slots, and original
  `varinitdata.xtx` supplies variable310=1. The probe applies that original
  NewGame policy/data; it does not inject a PC, branch flag or replacement script.
- Original `preprologue.spt` now executes16 instructions rather than stopping
  after8 at IF0x41. It retains story18bytes and film16bytes and yields the exact
  pending FilmTypeTime(0) receipt at PC0x173/code0x33330052, next0x178, serial4.
  Repeated polling preserves that receipt/PC/executed count. The reviewed modern
  source contains an empty FilmTypeTime handler, but equivalence to this legacy
  callback ABI remains unverified; no generic no-op acknowledgement was used.
  Next known script operation is ClearAllEffect at0x178, which still requires
  a real semantic/shared-engine mapping. Evidence: `owner-control-final.log`.
- These are executable native-control and reusable resource results, not a title
  screen, rendered System-NNN gameplay, completed integration or original saves.
  Production source is frozen for this bounded round; no full player build, commit or game/font/key distribution was performed by this agent.

### 2026-10-31 — System4 shared SACT initialization and text-format extraction

Implementation:
- The first-class native worker Config now carries optional logical canvas and
  explicit Gothic/Mincho font aliases. SACT2.Init requires published shared font
  faces/measurement and device-free channel audio, then commits the shared
  opaque-black hidden main Canvas1 and engine Layers through one atomic owner
  batch. Repeated same-client Init is idempotent; cross-client reuse fails.
  Missing font/canvas admission does not acknowledge the original HLL. Native
  screen/main-surface/message-count/clock queries preserve their reviewed ABI.
- Added public safe CPU crysirror_text::format::format_one_bytes, independent of
  VMs/fonts: first compatible `%d`/`%s`, bounded width/zero padding, encoded-byte
  preservation, original ignored integer/string precision and declared-size
  string copying (including value NUL). Template/output<=1MiB and fields<=510;
  unsupported conversions/template NUL fail visibly. No private VM formatter.
- S_MOD maps only native type/refcount/PC policy to that shared kernel. The format
  slot is released; a string argument is released only if a matching `%s` used
  it. Unsupported formatting leaves the original stack, heap and PC unchanged.
  Provenance and scope remain in crysirror-system4/SOURCE.json; no original
  executable, C engine, renderer, font engine, SDL mixer or hosted screen runs.

Compilation:
- Final `cargo check -p crysirror-system4 --release` passed
  (system4-final-check.log). Release VM/host and shared-format tests compiled.

Behavior validation:
- Shared byte formatter passed5 targeted tests (three unit tests plus two
  independent integration tests); literal `%` overlap, sign/zero/precision,
  encoded byte width, embedded value NUL, unused string and output limits are
  covered (system4-format-final-tests.log, system4-format-byte-final.log).
- Original VM release tests15 passed/1 default ignored; includes new native
  S_MOD reference-release and unsupported-operation atomicity tests. Shared-host
  missing-font/no-resource-commit test1 passed/2 default ignored. Both ignored
  runtime-input tests were then explicitly executed:2/2 passed using a local
  system font published through engine FontLoadJobs. Idempotent initialization,
  foreign-client rejection and actual engine Canvas pixel/hidden Layer state
  were verified (system4-format-vm-tests.log, system4-sact-format-owner-final.log).
- Rance7's real original constructors now execute11,053 instructions and commit
  five canonical Output records, advancing through SACT2.Init0x6f6 and
  S_MOD0x17e0e. Current explicit frontier is SR_ASSIGN opcode0x4e at0x384a in
  menu_metrics_t@0. It is original struct-page assignment, not string formatting;
  old-page deletion/deep copy/destructor/slot identity must be implemented.
  Main remains false. No factory, native title/gameplay, complete HLL, physical
  audio/window/GPU or original-save support is claimed. Root player13 does not
  link System4; this evidence is library/VM plus shared Runtime owner execution.
- Next-step source/commands are in ignored target/verification/system4/HANDOFF.json.
  Source is frozen after this bounded result; no commit, push was added.

### System-NNN: real Core VmAdapter worker/owner seam (2026-10-31)

Implementation:
- Added first-class `adapter::NativeAdapter::from_prepared`. Original VM instruction batches own a bounded TaskPool worker; original pending receipts return to an owner-only `Bridge`. No HostServices/window/GPU/audio pointer reaches that worker. Bootstrap read/decrypt/config/NewGame data also runs on a cancellable scoped resource worker in the owner test. There is no factory/desktop automatic-play registration.
- Proved the legacy FilmTypeTime no-op by static original-module inspection: moduleSHA e85e92ec538d50eae08d27286177149972d443a96d1415277c1ae1f9ee108bdb, CGameCallBack RTTI/vtable0x564f04→complete ReceiveScriptCommand0x541af0 has no branch; original ExecScript0x4ea210 passes its scalar then advances. The original executable was read from MSI but never run/distributed. Exact legacy Fill defaults/RGB behavior are additionally proven by CEffectFill0x4aba20/0x4abb10. Public unlicensed source was used for facts only, not copied/linked.
- Native policy now maps original effect clear/fill into shared retained Layers, PrepareOverlap into shared Canvas copy_subtree, and the bounded initial absent-music stop into shared Audio. No VM pixel/composition/audio algorithm was added. Existing opaque caller backdrop is explicit; it is not a reconstructed native title. Unsupported frame/read/music-fade/message/effect/service/save semantics remain fenced at native locations.

Compilation and behavior validation:
- Release check passed and final `cargo test -p crysirror-system-nnn --release` passed13 authored tests (bridge/Core-port3, expression/config5, machine3, program2), with2 installed-data tests ignored by default. Independent host tests verify frozen snapshots survive live layer removal, stale receipts/owner errors do not acknowledge or mutate, and the actual Core worker port commits only through owner services. Log: `target/verification/systemnnn/core-port-final-tests.log`.
- Explicit original-data owner test1/1 passed on its fresh binary (`owner-core-port-actual.log`). Original NewGame config/Legacy1000 variables execute unchanged original preprologue through both direct bridge and real VmAdapter port:25 instructions,6 shared owner API calls,1 Canvas snapshot and actual opaque800x600 shared Fill Layer100. Original story18B/film16B remain. It now stops at originalPC0x1a1 / SetEffect29(SUNA), rather than16 instructions at0x173. Exact pending receipt persists; Core diagnostics worker_pending=false/owns_framebuffer=false/factory_registered=false; repeated terminal step makes no owner mutation. No original title, picture render, playable game, audio device, full compatibility or saves are claimed.
- Existing ImageEffect WhiteNoise/Noise and VFX CurlNoise/SequenceRng were checked. They do not express SUNA's persistent packed2x2 mono state, original CRT15-bit draw/update order, alternating half-bank XOR,8x2 region alignment and temporal separate-term/256 blending. No static-noise substitution/private pixel loop was used. Required reusable temporal-raster API and exact next operands are recorded in `crates/crysirror-system-nnn/HANDOFF.md`. Production is frozen; no shared runtime/desktop/root-Cargo edits, full player build, commit were made by this agent.

### System-NNN native frame policy and real MSI resource diagnosis (2026-10-31)

Implementation:
- Native DRAW now uses shared frozen Canvas images, Pattern jobs and image
  resources for every original print. Zero-frame DRAW prints once without
  aging effect counts; positive frame waits count after Print and retain the
  immutable base capture. FILL54's reserved first parameter does not replace
  its native9999 maximum; NOP0 retains native99999 lifetime. Loading an image
  preserves its current Fill effect, and replacing Fill restores the assigned
  image. Active-picture PRELOAD remains an explicit unsupported boundary.
- Native resource errors retain the actual GTB path and byte length. No parser
  bounds were relaxed, corrupt high-priority pack skipped or private renderer
  added. Full frame-debt scheduling, ongoing effects during message waits and
  original VM/resource save restoration remain incomplete.

Compilation:
- Release check passed (`native-draw-effect-policy-check.log`). The current
  release bridge test binary compiled successfully (`native-draw-tests.log`).

Behavior:
- All8 current authored bridge tests passed, including DRAW0/2-frame exact
  shared Pattern steps/global RNG consumption, effect counters, immutable image
  display and retention of the next unsupported native PC. These use an
  authored known-pixel capture provider, not an original-game GPU capture.
  Evidence: `target/verification/systemnnn/native-all-current-bridge-tests.log`.
- The explicit system-font native window50 fixture passed using shared image
  jobs plus separate GlyphRuns; source masks0/128/255 yielded exact50-percent
  RGB255/190/128 and the reveal/advance click contract was asserted
  (`native-window50-font-actual.log`). This is not original dialogue completion.
- Actual player18 now waits for startup work and reports a real GTB error before
  any original instruction. Reading the MSI through the shared InstallerCatalog
  proved all6 GPK and6 GTB records are four-byte placeholders with values0..11;
  the external distribution has full same-named packs. The MSI mount currently
  shadows them. This is a source-priority issue, not a new GTB profile. Parent
  is correcting shared mount precedence; original title/play is not claimed.
  Evidence: `target/verification/systemnnn/probe-gtb.log` and
  `player18-controller.log`.
- After extracting the shared detached GpkIndex parser, its explicit owner-pack
  release regression passed1/1 again: all1625 DWQ decoded,156 sampled audio
  wrappers decoded2,555,904 PCM frames with finite nonzero samples. Evidence:
  `target/verification/systemnnn/owner-blackcyc-index-regression.log`.

### System-NNN native outer startup and picture transparency — 2026-10-31

Implementation: first-class Controller now follows reviewed Nothing -> real
shared MediaProbe -> Logo -> original configured Opening background SPT, with
native mode3 and original variables (no forced NewGame/VarInit). Shared image
jobs/Layers implement Logo; native click edges survive admitted source/image
work. Separate NativeAdapter/InstalledAdapter completion pumps drain dependencies
without advancing clock, sampling input or scheduling a new VM batch. Core-wide
maintenance routing is separate parent work. Native six-rectangle geometry maps
to Layer crop/UV, LayerGrey uses public grouped fixed-luma, and CTRANS9 maps its
actual legacy14 fields to public clipped masked/unkeyed Q8 image composition.
Raw/opaque/display images are separate engine-owned immutable leases; no VM
pixel loop, renderer, private decoder or original EXE is hosted. Movie/Title
continuation and original save restoration remain explicit incomplete boundaries.

Compilation: release checks passed in native-rectangle-check.log,
native-ctrans-check.log and installed-dependency-check.log under
`target/verification/systemnnn`. No desktop binary was independently rebuilt.

Behavior: fixed player22 SHA256
`5d0a93d5800a806edd22f5ca080aa691f8d2ad0456a7e19904251068c519acd2`
ran ordinary `play Downloads/game-MUGEN2 --headless --no-audio --frames 20000`.
It passed original Nothing/Logo and entered actual Opening background, then
stopped at original PC0x37d/instruction22/CTRANS9 with the pending receipt
retained. See player22-controller.log. CTRANS source landed after22 and is not
an actual-game pass in that binary. Independent native-ctrans-tests.log passed
lib5/5 + bridge12/12 (exact mask0/128/255 and unkeyed RGB, immutable source/base,
unchanged RNG, existing Draw/Overlap/image/audio regression). First new shell
click test exposed removal before first publication; fixed only-presented-layer
retirement and reran native-shell-completion-tests2.log: installed2/2 + shell4/4
passed, including no-clock completion/failure retention and pending click edge.
No title screenshot, Start/story, long input or complete original save is claimed.

### System-NNN Opening effect expansion and reusable sampling — 2026-10-31

Implementation: reviewed original-module OldLamp109 now keeps only native
Calcu/Print trigger/count/global-RNG policy; public image Composite owns drawing.
All lamp Calcu candidates precede layer Print and commit with shared-frame/RNG
receipts. Bura74 maps native quadrant parameters into the public immutable
PiecewiseResample image operation (signed16.16 nearest sampling, independent Q8
weights, checked endpoints/work budget and row cancellation). Public ProgressCurve
owns bounded f64 envelopes/right-continuous steps; native numeric curve mapping
covers original time maps0/4 and quadratic/sqrt/cosine shapes. Other native warp
and random families still explicitly fail. Explicit MAE1 preserves current state
as proven by actual SetEffectScript0x4c18e0; no unknown-operation fallback exists.

Behavior/compilation: release native-oldlamp-tests.log passed lib6/6+bridge13/13;
native-bura-tests.log passed lib8/8+bridge13/13; standalone piecewise sampler2/2
and progress envelope2/2 passed. Native Bura shared ImageJob/count fixture and
MAE state-preservation fixture each passed1/1. These are authored fixtures and
static original-module facts; actual game frontier remains player22 PC0x37d
until the single coordinated player23 is built/run. No title/play/save pass.

### Shared Canvas capacity and public query (2026-10-31)

Compilation/tests: independent release Scene capacity/atomic-overflow/client
isolation/slot-replacement/restore1/1 passed; public reflected getLimits1/1
passed; native2048-layer admission/explicit overflow/invalidated-slot reuse1/1
passed. No device, system font or game data was needed. API docs8/8 passed and
fingerprint bd71afbaa37a19da was recorded by the coordinated System-NNN worker.
Logs: shared-canvas-capacity-tests.log, public-canvas-capacity-test.log,
native-canvas-capacity-test.log. Unified full-feature player23 release build
passed3m09s, SHA256
`2c7a0ac497d068d207b846ddbaa63c0b4f173342a2d74a0077e28762c557caf8`.

### Published worker images in Canvas snapshots — 2026-11-01

Actual game: coordinated player23 (SHA256
`2c7a0ac497d068d207b846ddbaa63c0b4f173342a2d74a0077e28762c557caf8`)
advanced MUGEN2 original Controller/Opening beyond CTRANS to instruction25 /
PC0x397 at the first OVERRAP. It then explicitly failed because WGPU snapshot
preflight attempted a filesystem read for `__image_resource/16-100`; source
image resources were still engine-owned. player23-controller.log records this,
not a completed title, final screenshot or playable game.

Implementation: common WGPU snapshot dependency preflight now resolves actual
client-scoped published Surface entries directly. It retains admission-time
catalog leases and does not schedule PNG encoding/decoding or a file AssetLease.
Fake published names, foreign owners and released live paths still fail. No
NNN private renderer/screenshot/provider or resource-format parser was added;
no Core/Runtime/public signatures changed, so API fingerprint stays unchanged.

Validation: release CPU preflight1/1passed verifies direct in-memory resolution,
cross-client rejection and frozen/live release behavior. Explicit real GPU
fixture1/1passed on Apple M5/Metal in published-snapshot-gpu-test3.log: shared
ImageJob -> publication -> capture admission -> clear/remove/release live path
-> existing engine compositor -> worker4x4 red readback with clock0. First GPU
fixture had incorrectly combined Layer creation and Canvas copy in one ordered
service batch; it was corrected to create the source first and rerun. This fix
is after player23 and still needs the next coordinated actual-game run.

## 2026-11-01 — System-NNN actual Controller crosses first overlap

Actual behavior: immutable unified player24 SHA256
`f6cc7fc86de4ed35650c510ecdda07cc2007900f0876500727295da611ef8d98`
ran ordinary `play ~/Downloads/game-MUGEN2 --headless --no-audio --frames 20000`.
The original Nothing/Logo/Opening chain passed player23's25instructions/PC0x397
capture failure and reached141original instructions/PC0x6c2. It then explicitly
failed on interleaved picture/effect bands. Log: ignored
`target/verification/systemnnn/player24-controller.log`. No final screenshot,
title, Start/story or complete integration result is claimed.

Implementation after that artifact: native ordered Paint DTOs place shared
ImageInput composition between original effect slots. Shared MaskSeparate256
models the reviewed x86 mask rule (each product floors separately, zero/full
endpoints, destination display alpha retained). Generic raster work stays in
crysirror-image/Runtime jobs, not the VM. Release compilation, independent
behavior tests and API regeneration are being validated separately.

System-NNN ordered painter validation follow-up: release lib8/8 plus bridge16/16
passed (`ordered-paint-native-tests.log`), including two consecutive native frames
with grey → upper red fill → grey yielding exact RGB75 and unchanged RNG.
Independent shared bitmap mask kernel1/1 and Runtime image graph1/1 passed
(`mask-separate-kernel-tests.log`, `mask-separate-service-tests.log`): mask0/1/128/254/255,
separate-product floors, clipping, retained receipt, client isolation and invalid
opacity rejection. Final releasecheck passed (`ordered-paint-check-final.log`);
first check had an Error-vs-anyhow type mismatch corrected before these results.
API generation recorded `d22690a23af1bf9f` → `6781d4170ef6e9f2`; initial bless7/8
required the CHANGES entry, then no-bless docs8/8 plus command coverage1/1 passed
(`mask-separate-api-docs-verify.log`). The actual game has not yet run this new
source; coordinated next desktop artifact is still required.

### System-NNN native FadeOut11 — 2026-11-01

Implementation: complete pinned FadeOut source plus original static callbacks
0x4aab70/0x4aac80/0x4aab10 establish argument defaults, reverse Q15 weights and
CountIsMax -> Fill54. The adapter uses existing public RgbLookup jobs. Terminal
fill preserves native count/max, suppresses picture underlay, uses exact bytes
and remains present during DRAW0. Native g/b=-1 fade inheritance deliberately
changes to independent byte wrapping when command becomes Fill. No native pixel
loop, new public API or original-engine execution was introduced.

Independent release behavior: native-fadeout-final-tests.log passed9/9 library
and17/17 bridge tests. The new cases prove intermediate Q15 weights, negative
channel transition, exact255 terminal colour, frame/count/RNG policy, DRAW0 and
unknown-PC retention. The earlier native-fadeout-tests.log passed8+17 before the
negative-channel follow-up; the final run is authoritative. Dedicated release
check is separate. Actual MUGEN2 remains at the player24 observed141/0x6c2
frontier until the next unique desktop build; static0x86d is not execution proof.

### 2026-11-01 — System-NNN implementation and validation

System-NNN FadeOut compilation follow-up: dedicated `cargo check --release -p
crysirror-system-nnn --features runtime` passed in9.79s
(`native-fadeout-final-check.log`). Actual player continuation remains pending.

## 2026-11-01 — System-NNN player26 original startup continuation

Actual ordinary Controller run with immutable player26 SHA256
`c67df6c247e15f6b4ec72a222d3c4d9c87b33828bfbbca2f7d6ad8d531166897`
passed the old interleaved-painter frontier and executed the new native FadeOut,
then retained an unsupported SETFILM callback at211instructions/PC0x8a7,
code0x3333001a/parameter1. Evidence: player26-controller.log. No final screenshot,
Title/Start/story or full engine/save completion is claimed. GPU was released
after the natural error; no owner game process was killed.

Public documentation: final AlphaClipRect/source/method registration, options
example and no-bless release cargo docs8/8+command coverage1/1 passed in
alpha-clip-api-docs-final-cargo.log. Current fingerprint66c74a7ee0a69bf1 replaces
6c31b5c44d887345 solely because rustfmt changed reflected array type whitespace;
both the reason and unchanged wire semantics are recorded in CHANGES. NNN source
is frozen at player26 while Root prepares the separate unified player27.

### Final ExE regression after shared alpha/lazy/cache changes — 2026-11-01

Frozen player27 SHA256
8d7c607fb330c43779744e80924979b0085c3d624ab233e907db2a982ec6fdd7
(Root's unique all-feature build, adding only its RealLive settings clip fix to
our shared26 changes) completed110000 requested/runtime frames, engine1759984ms,
native97464 frames, error:null, queued_input0,8 ordinary clicks and two Ctrl
down/up phases. Native Layers101. Actual captured image was viewed: original
Day03 night street/standing girl/two Japanese choice buttons, normal date/menu
UI; parser03.ks *day_03 line1475. Evidence: exe-regression27.log/png/report.json.

### System-NNN background read-mark and control-frame policy — 2026-11-01

Implementation: exact SETFILM suppression is limited to original background mode
with backScriptSetCGOkFlag0. Public source and original module0x5324b0/0x543061
prove this condition/default; foreground and enabled-background persistent flags
still fail explicitly. Foreground constructors remove carried background policy.
Background ENDFILM/ENDKOMA/ENDIF now retain their own original receipts until one
shared FrameWork/ImageJob/Layer frame, CountUp and native duration complete.
Foreground Yoyaku behavior is preserved. No synthetic DRAW or unknown-code ACK,
new private bitmap/render implementation or new shared API was introduced.

Compilation: final dedicated releasecheck passed6.37s
(native-background-frame-final-check.log). Independent release behavior passed
41/41 (native-background-frame-tests.log): lib9/bridge18/machine3/control6/shell5.
New cases prove disabled-background-only suppression, error/PC retention in the
other contexts, three real shared frames, native counts/time and stable RNG.
Actual game remains at previously observed player26211/PC0x8a7; this newer
source still needs the next coordinated player. Title/Start/story/full native
restore are not completed.

### 2026-11-01 System-NNN shared three-input mask composition and background returns

Implementation: `crysirror-image::masked_pair::MaskedPairBlend`, Core
`ImageInput::Ternary`, the existing Runtime image worker and public
`Bitmap.beginMaskedPair` now support three immutable inputs. Independent14bit
mask contributions and individually truncated RGB products retain destination
alpha; no ordinary crossfade approximation or VM-private pixel loop is used.
The same scoped owner/epoch/revision/cancel and existing graph/output/shared
memory limits apply. Source facts and arithmetic are recorded separately in
`docs/nnn-masked-pair-source-audit.json`; this is not a Title integration claim.

Native implementation: background SETFILM suppresses history writes only in
the explicitly bound original `backScriptSetCGOkFlag=0` profile. Foreground and
enabled-background writes still fail visibly. ENDFILM/ENDKOMA/ENDIF background
returns now retain their original receipts until one shared native frame and
its CountUp/wait complete; foreground control flow remains distinct.

Compilation/independent behavior: release Runtime check9.85s and Engine
check11.20s passed. MaskedPair kernel3/3, Runtime service/composite/mask6/6 and
public bitmap effects5/5 passed. Background native tests41/41 and final release
check6.37s passed. Full logs are under ignored
`target/verification/systemnnn/{masked-pair-*,native-background-frame-*}`.

API reference: final merged fingerprint `1171d6b86d446b00` (previous
`c4f0841c769d7a18`) includes this shared operation, the shared CG catalogue types
and shared paged-image types. Actual no-bless release `api_docs`8/8 plus
`api_coverage`1/1 passed (`masked-pair-cgm-paged-api-docs-verify.log`).

Actual game behavior remains the prior player26 frontier:211 original
instructions/PC0x8a7 (SETFILM), before these new source changes. Root coordinates
the next single player28 build. No new original-game Title/Start/full integration
success is claimed; Movie/Title continuation, live message/effect recomposition
and complete original saves remain unfinished.

### 2026-11-01 System-NNN original Opening completion and outer Movie transport

Actual game: immutable player28 (`b6d6b7479ebad8d8d599b1a0a47c8251eb59daa204168864ad230979713dee8f`)
ran ordinary `play /Users/xmoe/Downloads/game-MUGEN2 --headless --no-audio
--frames 20000`. It naturally reached the explicit original Opening-completed /
missing printmovie-and-Title continuation error. Thus the actual background SPT
passed the former SETFILM/frame-return boundary and reached its native NEXT.
The log has no final instruction count, so none is claimed. It exited1; no final
Title capture or playable-game result is claimed. Log: ignored
`target/verification/systemnnn/player28-controller.log`; GPU released afterward.

Implementation after28: `native_movie.rs` maps original outer printmovie
selection, optional original movielist pair lookup/fallback, calculated mode27,
transport, pointer-skip/noSkip and post-stop native wait to shared Media and
ordinary image Layers. Missing selectors retain the original default movie;
only the existing zero-selector / selectByVar=-1 branch skips. Completed Opening
Machine/Bridge/configuration/scoped loader remain owner-held; NewGame is not
forced. Actual legacy Init/Calcu were checked separately from later source:
there is no invented immediate black clear, and original saved exit-buffer
colour is not misrepresented as an immediate live-screen operation.

Compilation/independent behavior: final release check passed; authored
`native-movie-final-tests.log` passed12/12 (installed2, movie5, shell5), including
calculated-selector values, frozen transport lifecycle and native wait/input
policy. These transport fixtures do not establish actual decoder/game success.
Source evidence is in `docs/nnn-native-movie-source-audit.json`.

Limitations: this source is prepared for the next unified player29, not present
in immutable28. Title UI/background continuation remains an explicit error.
Original exit-buffer/Title handoff, stepped100ms decibel music fade, active
message/effect recomposition and original-save reconstruction remain unfinished.
The existing audio policy rejects the unsupported stepped fade instead of
substituting the shared linear fade. No complete System-NNN integration claim.

API reference follow-up for the independently implemented shared Solid colour
command: actual fingerprint `c8c180ba4ff65706` (previous
`1171d6b86d446b00`). `Layers.setSolidColor` / `LayerCommand::SetSolidColor` retain
all unrelated layer state and reject non-Solid/foreign targets atomically.
No-bless release api_docs8/8 plus api_coverage1/1 passed in ignored
`target/verification/systemnnn/solid-color-api-docs-verify.log`. All source
is frozen for Root's single player29 build; no new System-NNN Title claim.

### 2026-11-01 System-NNN real mouse mapping and player29 continuation

Actual immutable29 (`24218d166db484803755533a3518625cc7f6e50747007c692753b1d72a0b4850`)
ordinary MUGEN2 Controller20k naturally exited1 at `original Movie completed to
Title; native Title UI/background continuation is not yet installed` (ignored
`player29-controller.log`). It passed the old Movie-entry guard without forcing
NewGame. That log lacks runtime JSON, so it does not distinguish original
selector suppression from decoded EOF; actual OP playback is not claimed.

Own-crate input wiring after29 now samples the shared `MOUSE_LEFT`/`MOUSE_RIGHT`
bits for original trig/click fields and Logo/Movie pointer skips. Enter, Space
and the Left arrow no longer masquerade as mouse buttons. Installed message
advance maps left mouse plus Enter; original Space is window hiding, not an
invented advance binding. Missing middle-button fields remain unbound and fail
when read, with explicit input-capability diagnostics.

Release check passed6.28s; Movie5 and Shell5 release tests passed, including a
real Core VM worker sampling contrasting pointer/keyboard states and retained
pointer edges during image dependencies (`native-input-step-tests.log`).
Native Title configuration/animation layout and immutable current-frame/context
helpers are preparatory own-crate code only. No Title presenter or title success
is claimed. All production source frozen for Root's single player30 build.

### 2026-11-01 System-NNN native Title implementation / independent validation

Implementation after30: installed Movie completion now prepares original Title
sprites through scoped source workers and shared DWQ/ImageJobs, restarts the
actual mode4 SPT with the original variable/RNG context, and drives engine-owned
widgets/hit masks plus immutable complete-frame image composition. Native
button copy defaults, frame indices, counters, intro policy and clickTime remain
in the adapter. All pixel arithmetic is the existing shared ImageInput
Composite/Ternary/MaskedPair operations. Outer BGM uses the same native audio
policy and shared audio preparation receipts, without a fake SPT call.

Compilation: dedicated release check passed11.29s. Independent authored release
`native_title`2/2 passed (`native-title-tests.log`): actual common image workers
verify exact masked pixels and immutable old frames; retained UI verifies alpha
hit coverage, press delivery and clickTime1 before action return. These tests
use no GPU, audio device or proprietary game data. Source audit is
`docs/nnn-native-title-source-audit.json`.

Actual original-game frontier remains immutable30: frame5862/time93792ms,
231instructions/PC0x1d0, legal selector0 Movie suppression, then the old explicit
Title guard. Its generic failure JSON confirms no video/audio resource was
created for that selector branch. New Title source is frozen for Root's single
player31 build and has not yet rendered the original Title. System sound,
L/C hardware accelerators, completed Start/exit policy, ongoing message/effect
recomposition and original saves remain unfinished and explicit.

Shared API documentation: integer_fields source/four Rust functions documented;
actual signature fingerprint remains `c8c180ba4ff65706`. Regeneration8/8 and
no-bless release api_docs8/8+api_coverage1/1 passed separately
(`integer-fields-api-docs-{bless,verify}.log`). No new reflective method or DTO.

### 2026-11-01 System-NNN actual Title VM frontier / Move policy

Immutable31 original MUGEN2 entered native Title mode4, prepared actual sprites
and shared BGM, and created engine widgets. It then failed at frame8220,
261instructions/PC0xe39: native effect52 parameters[0,52,300]. Title counter70
had not reached the original menuWait200; final capture was not written and
complete Title/Start is not claimed (`player31-title.log`).

Implementation after31: native MOVE52 rectangle/count semantics now call the
existing shared Composite graph. Native signed rounding/wrapping, hidden
Calcu, post-Print CountUp, endpoint source/destination copy and command0 remain
adapter policy. No private pixel loop or renderer was added. Dedicated release
checks passed; independent release lib11+Bridge18+shared CompositeMove1 passed30
(`native-move-tests.log`). This source has not yet run the original game.

Further source-only work maps original system-sound lookup/table/two-channel
policy to shared Audio preparation and gates native Title frame/action on its
receipt. That new leaf and later input/hiddenMove regression additions await
Root's owner-scheduled combined release run; they are not reported as verified.
The queued player32 build was cancelled by its owner before producing an
artifact. No new standalone Cargo queues, commits were added by this work.

### 2026-11-01 System-NNN unified release verification

Owner-scheduled seven-package `--release --lib --tests` run compiled the final
NNN runtime profile and passed all67 non-ignored NNN tests;8 owner/font tests
remained ignored and are not counted as passes
(`target/verification/native-source-batch/combined-release-tests.log`). This
includes the new hidden-MOVE Calcu/native post-Print policy, C-key accelerator,
and original system-UI-sound lookup/two alternating channels through real shared
WAVE/audio preparation, without changing native music policy. The earlier
intermediate Title dependency-pump signature failure is superseded by this
successful complete crate run.

Actual original-game evidence is still immutable31 at Title counter70/MOVE52.
The verified newer source has not yet been linked into an actual-game artifact;
full Title, Start, long story and original-save reconstruction are not claimed.
Static original-module Title/NewGame evidence is separately recorded in
`docs/nnn-native-newgame-source-audit.json`; it is not behavioral acceptance.

### 2026-11-01 Alice System4 native source and shared ALD validation

Implementation: native SR_ASSIGN/SH_SR_ASSIGN now retain heap-slot identity and
run original destructors before deep copy; ASSERT preserves fatal false checks.
A larger scalar/string/reference opcode batch reuses the shared byte formatter.
ALD numeric multi-volume catalogue/header parsing lives in shared GPL formats-arc;
legacy filename extraction delegates the same parser. Source provenance and the
MIT footer-count reference are retained in ald_catalog.NOTICE and source audit.

Compilation/ordinary behavior: Root's combined release run passed System4 VM
21 tests (1 owner test remained ignored), AIN2 and shared-init1. Shared ALD3
and legacy archives12 passed;16 Python arc tooling tests passed separately.
Ignored tests are excluded from these passes. Logs: native-source-batch/combined-
release-tests.log and system4-ald-python.log under target/verification.

Actual behavior: explicitly invoked frozen release binaries passed both owner
checks1/1 with0ignored. Rance7 reaches11,256 instructions at native A_PUSHBACK
0x84/PC0x1007c in mes_mark_metrics_t@AddCG, after real SR_ASSIGN and ASSERT;
main=false and factory_registered=false. Shared numbered CG catalogue reads
6,606 entries across original GA/GB volumes [6506,100] without unpacking. See
native-source-batch/alice-owner.log, ald-owner.log and owner-artifacts.json.

Next implemented source batch (not yet validated): bounded CP932 System40.ini
syntax/bootstrap, native class/ID-to-shared-mount policy, optional runtime
factory, native sprite properties through shared Layers and clearable input
flags through shared InputObservation; native array replacement/reallocation,
push/insert/erase/pop/reverse and struct pushback shorthand; shared QNT0/1/2
headers and bounded complete zlib planes. The unchanged RGB-only native blend
profiles, full SACT HLL/text/audio, saves and title/gameplay remain incomplete.
The new desktop detection leaf is not yet registered or linked into a player;
no working play command is claimed. No original engine is hosted or executed.

### 2026-11-01 System-NNN native Title exit and NewGame source batch

Implementation now retains the full completed Title frame as an engine image,
quiesces an explicit background-mode exit at its admitted shared frame without
forging a native DRAW/OVERRAP acknowledgement, and prepares the original sys
controller on the scoped loader worker. The native variable initialization,
retained picture-cache distinction and controller restart follow the reviewed
legacy module. Widgets/window/glyph presentation is removed through engine
operations; original message rows remain available for APPEND. Original menu
group first-click locking, one returned sound and animation adjustment policy
stay native, while input hit ownership and pixels remain engine services.

Owner-scheduled release validation passed the selected45 NNN tests:
lib16 (`native-batch3-lib-fixes.log`) plus Bridge18/Title4/Shell5/installed2
(frozen binaries and logs under `native-source-batch/nnn-batch3`). The new hide
fixture initially omitted native name-color setup, then mixed a16px canvas with
native100px print defaults; both authored fixture preconditions were corrected
without weakening production validation. No proprietary font/device was used
by these tests. This is a selected suite, not a complete-suite rerun.

Actual original-game evidence still ends at immutable31's Title MOVE52 frontier.
Source is frozen for the owner's player32 build; complete Title, original Start,
long-story and original-save behavior remain unverified. Configured exit fades,
other Title modes, foreground SystemFile/read-mark persistence and full native
save reconstruction still contain explicit gaps. No commit was added.

### 2026-11-01 System-NNN immutable32 original Controller validation

Ran the original MUGEN2 Controller once with immutable release player32 SHA
`783940ef4a493cdebe59c04172191887952257a5072b19200d97f897c0abcd0c`,
Metal on Apple M5,20,000 requested headless/no-device-audio frames. It executed
348 original instructions, naturally completed the original nonlooping Title
background script at PC0x1d0, and passed the original200+100 Title intro:
Title counter2568, all5 engine widgets enabled, actual nonempty hit masks and
shared BGM active. The adapter still reports owns_framebuffer=false.

It then failed at frame17150/time274400ms on the already explicit missing
Title autoRestart outer-mode transition. No final capture was written; visual
Title acceptance, Start/story and complete compatibility are not claimed.
Logs and reduced facts are `target/verification/systemnnn/player32-title.log`
and `player32-title-summary.json`. No source patch or repeated frontier run
followed this result; GPU was released. Complete SystemFile/Boot/original-save
modules remain the next source-based work, distinct from this runtime check.

### 2026-11-01 — Final source-batch player and public API verification

Compilation: rebuilt the full-feature desktop release. Its inspect command reported 13 linked adapters, including the opt-in System4 factory. No source commit was implied.

Independent verification: Python201 passed; public api_docs8 and api_coverage1 passed. Other native-batch results were NNN45 passed, System4 ordinary44 passed/5 ignored, and shared formats76 passed/4 ignored. Logs are under target/verification/native-source-batch. Ignored cases are not passes and factory registration is not completed compatibility.

### 2026-11-01 Alice System4 full-source batch and shared byte-text extraction

Implementation: the first-class `crysirror-text-legacy` workspace crate adds
bounded cancellable byte-character/number operations, separate from the base
MPL text package. GPL-2.0-or-later source provenance, original notices and
licence text are retained. The public character profiles distinguish stored
byte extent, NUL prefix, native broad lead masks, signed single bytes and
explicit numeric-compaction tail behavior. No VM, renderer, font or device is
embedded. New System4 source delegates eleven string/character/numeric opcodes
to this shared crate; only native stack/heap lifetime policy remains in the VM.
Its worker now supplies actual cancellation to the per-operation checks.

Compilation/behavior: shared byte-text `cargo test --release --lib` passed all
6 authored tests with no ignored cases (`system4-text-legacy-tests.log`).
System4's subsequent release test batch is still running at this entry; the new
VM string source is not claimed validated yet. Earlier owner-scheduled batch
passed44 System4 ordinary tests/5 ignored, including31 VM tests/1 ignored;
shared formats passed76/4 ignored after correcting duplicate ALK registration.
The unique original ALK registration delegates the shared parser and preserves
its old sequential aliases while `AlkCatalog` exposes native numeric IDs/holes.

Actual game evidence remains19,108 original instructions at `tagTitleBar@0`,
PC0x19286e/A_FILL, before main. That prior frozen owner binary does not contain
the later A_FILL or string family; no new title/gameplay/save/60fps claim.
Original save reconstruction, complete HLL coverage, encoded CG plane metadata,
exact native blend/text profiles, audio control gaps and real AliceLogo2
animation remain explicit work. No original-engine hosting, code execution,
commit was added.

System4 string-family follow-up actual result: the release command completed
successfully, nine targets49 passed/5 ignored, including new string-family5/0
and existing VM31/1. Log: `system4-text-module-release.log`. Shared byte-text
remains6/0. Immutable player33 predates this source; it is not evidence of these
new operations in a desktop build. No original-game rerun was performed.

## 2026-11-01 — System-NNN complete SystemFile layout, bootstrap namespace and native host close

Implementation:
- Independently expressed the reviewed legacy10-record58432-byte SystemFile
  layout, preserving opaque words/labels/padding, native grouped limits,
  film/message/CG/movie/voice/music flags and100 persistent variables200..299.
  Original layout/default/index/save policy stays in the first-class adapter;
  reusable bit operations use Core integer_fields, storage uses the existing
  scoped asynchronous atomic-write service. No legacy host or renderer runs.
- Installed bootstrap first obtains real client-scoped civil time, then prepares
  defaults/contiguous CG/project limits, original file overlay and persistent
  variables on its resource worker. MainControl and game configurations now have
  separate namespaces; host initRandomSeed policy preserves fixed/wall-clock/
  no-srand branches. Native constructor font-name metadata is read from the
  authenticated PE as data, with loader-zero-filled name defaults verified.
  Empty-name initialization/clear flags preserve original copy-through-NUL
  padding semantics. SETFILM uses actual project script indices and both original
  suppression gates rather than acknowledging foreground writes without state.
- Native host close stops new VM batches, drains admitted resource/frame work,
  cancels known text waits without inventing a result/ACK, and releases/stops
  only its own audio channels. Separate civil samples and retained writes
  preserve both original Game-loop Save and End Save before halting. Exact
  original lifecycle was statically checked at52d6b0/52c6f0; no EXE execution.
- Added shared pure Core civil_weekday/CivilTime::weekday (Gregorian Sunday0,
  invalid dates unavailable). No new clock sampling or device dependency is introduced.

Compilation and independent behavior:
- Final local release selected NNN targets passed57 with1ignored: library28,
  Bridge18, installed2, nativeShell5, nativeTitle4
  (target/verification/systemnnn/native-system-file-close-final2-tests.log).
  Tests verify two real scoped writes/saveCount2, exact persistent-variable
  range, unchanged scene/clock/no script load, native text cancellation retaining
  PC/receipt, and another client's same-numbered audio channel surviving close.
- Shared calendar3/3 passed. The ignored constructor test was explicitly invoked
  separately and passed1/1 against the authenticated owner module's metadata
  (native-system-file-original-constructor.log), without executing the module or
  loading/bundling a font. Ignored tests are not otherwise counted as passed.
- The first bootstrap test run had an authored atoi expectation failure
  (23passed/1failed); corrected to original nonnumeric→0. A later strengthened
  foreground-close check exposed a real background-only guard (26passed/1failed);
  production now has an explicit shutdown boundary, with final57 above passing.
- Shared metadata/motion/text-legacy/MT documentation regeneration and no-bless
  verification passed8 API tests plus1coverage; actual fingerprint
  1cae8f9410b1b5b9 is logged. Python tooling201/201 passed and scoped diff check
  passed. Peers' shared metadata/motion tests remain separately recorded.

Limits:
- This source is newer than immutable player33; no new desktop build, original
  game run or visual Title/Start/native-close acceptance is claimed. Existing
  single player32 observations remain separate. Full progress callback/getter
  consumers, outer mode families and original dataN.sav VM/resource restoration
  are still incomplete. SystemFile is not a substitute for original game saves.
- No commit, push was added.

## 2026-11-01 — System-NNN original game-slot context preparation

Implementation:
- Reviewed the complete original status get/set and variable-load routines,
  comparing the public source with legacy53ad20/53b630. Added worker-only native
  four-context candidates over the original7088-byte GAMESTATUS record, with
  original project script identities,249-call limits and script/command native
  continuation tables. Adjusted parent PCs derive from native IDs; raw return
  PCs use one bounded cancellable record scan to recover CALL identity.
- Original SetVarByLoad preserves current200..299 while restoring0..199 and
  300..999. The detached bank retains current RNG/calculated bindings; no native
  slot is treated as a framework snapshot. Machine now records acknowledged
  script/command continuation IDs for future original save production. Fixed
  context/name/ABI checks precede script loader admission.
- Preparation does not install a live adapter or enable original-save import.
  EFFECT/image/cache, message/selection, audio work, thumbnail and slot UI
  reconstruction are still pending; no missing consumer is acknowledged.

Compilation and independent validation:
- Local release selected targets passed45 with5ignored: library32, machine3,
  native_control6, native_services4
  (target/verification/systemnnn/native-game-save-context-final-tests.log).
  Authored native records verify four actual detached VM contexts/program-table
  destinations, persistent variables/current RNG, exact ACK-only native IDs,
  general command adjustment and corrupt/cancelled input leaving source state
  unchanged. Ignored owner/font cases were not run or counted in this batch.
- Python tooling rerun passed201/201 (system-file-python-final.log); scoped
  diff check passed. No shared public API shape changed after verified
  1cae8f9410b1b5b9, and these new native helpers are private Rust policy.
- No desktop rebuild, actual game run, original-slot restoration or complete
  System-NNN integration is claimed from this preparation.

## 2026-11-01 — System-NNN saved EFFECT images through the ordinary shared image workflow

Implementation:
- Reviewed complete original EFFECT layout and four save/load routines, with
  legacy534390/5342f0/534640/541220 static comparison. Native16x1024-byte records
  retain every rectangle/count/parameter/flag/padding word;16x256-byte names
  retain original double-NUL rules. Unknown commands are opaque metadata, not
  treated as implemented effects.
- Saved-image preparation and ordinary LOADDWQ now reuse one native slot/alpha
  workflow over shared resource registries and ImageJobs. No synthetic VM call,
  private decoder/pixel loop, native composed screen or second renderer exists.
  Enabled named slots prepare sequentially; inactive/empty slots preserve warm
  cache policy. Scope/name validation precedes I/O; Ready/errors are retained.

Compilation and independent behavior:
- Local release library34 and Bridge18 passed52 tests, with1ignored, exit0
  (target/verification/systemnnn/native-game-save-image-tests.log).
  New authored save records actually decode two pictures through shared
  DWQ/PNG/jobs and verify raw/background/masked alpha, retained native fields,
  no scene/clock/VM instruction changes, pre-I/O foreign/corrupt rejection and
  missing-resource failures without repeated I/O. Existing normal image/control/
  effect Bridge behavior remains passing after workflow reuse.
- Complete live original-slot restore is still not admitted: effect semantic
  state, text/audio/UI and a common asynchronous native restore commit boundary
  remain necessary. No desktop rebuild/original-game run is claimed here.

### 2026-11-01 Alice System4 shared image metadata, native source families and real main

Implementation: Core now has typed client-scoped encoded-image metadata queries.
Image decoder selection is shared with pixel decoding; formats-arc QNT uses one
header parser for versions0/1/2 and reports actual color/alpha plane declarations
instead of inferring them from opacity. Runtime workers use bounded fixed-length
reads with cancellation/source-length checks and exact fingerprints. Native SACT
CG metrics and file/numbered image publication use those shared receipts and
ImageProcess/ImageResource/retained Layers, without a private raster loop.

System4 Math random words reuse Resource's cloneable MT19937-1998 kernel; original
seed/discard/biased shuffle/table policy stays worker-owned, and real host-time
seeding awaits scoped CivilTime rather than virtual time. Reviewed Sys42VM static
evidence establishes actual nested peek-lock depth and OJMP/SJUMP semantics,
which differ from public reference stubs/explicit guesses. Native events use
shared InputBuffer receipts. MSG invokes original AIN code; FT_ASSIGNS preserves
its owned string, SLBL tag presence is retained, and scene jumps unwind native
frames/destructors resumably. Both find instructions preserve original key
ownership and run actual AIN callbacks across service waits. Invalidated callback
page identity/layout fails, including a matching callback. No hosted engine or
original DLL execution was added; public source notices/licences remain.

Compilation/independent behavior: a full release System4 batch passed77 ordinary
tests/5ignored across11targets (system4-functions-find-final.log). Final optional
input-startup and callback-pointer guards then passed43/3 across find7,VM35/1,
sact1/2 (system4-final-scoped-guards.log). Shared byte-text6/0, Resource31/3,
Image135/2, QNT5/0 and final Runtime bounded-worker1 plus integration2 passed
separately. Metadata tests distinguish absent versus all255 alpha despite equal
decoded opacity, preserve typed NotFound, reject changed fingerprints and do not
equate a valid header with valid compressed pixels. Ignored cases are not passes.
Shared API no-bless8+coverage1 passed at1cae8f9410b1b5b9 via NNN; the AIN AST
SLBL flag has no fabricated reflective class. Python tooling201/201 and final
format-ledger16/16 passed.

Actual owner behavior: frozen CPU test artifact SHA256
3d925d77960e67e2fb7457f29aa3bc9fd10502ed2655c72f18a3bbd80b02151d
explicitly passed1/1 in0.51s. Original Rance7 now executes435040instructions,
enters main, and retains DrawPluginManager.Load(string12)->int10 at PC0x88c in
SACT_LOAD_DRAW_PLUGIN. Shared font/audio/Canvas/Layer initialization is real;
7canonical logs/8debug/14clock calls,14254heap slots/274623words. This manual
probe is not factory registration or title/render/gameplay acceptance. The first
new owner attempt failed before bootstrap because an optional InputObservation
NotFound was wrongly fatal; production now uses the actually supplied VM input
frame, while pointer/focus still require observations. That failed artifact/log
is retained separately; the corrected passing result is above.

Separate frozen formats artifact762cd155ed11a13ac814612e460d401720fee87bc0fd661770e524db6caf5207
explicitly decoded all37 nonempty AliceLogo2 ALK QNTs, allversion2, totaling
11220069pixels;1/1passed. It does not establish the seasonal logo animation.
Prior ALD verification read6606 CG entries across GA/GB6506+100.

Limits: immutable desktop player33 predates all these additions; no full desktop
build or visual title/Start/60fps run is claimed. DrawPluginManager's public
fixed-success stub is not adopted. Actual DrawGraph shared mutable surfaces,
exact native blend/text/effects/audio profiles, genuine plugin mapping, complete
array sort/delegates, original global/group/resume saves, AliceLogo2 animation,
manual per-client presentation fencing and precise callback cadence still need
work. Next-source order and exact reproduction are in the ignored System4
HANDOFF.json; source audit is docs/system4-native-source-audit.json. No commit or push was added.

### 2026-11-01 Alice DrawGraph prerequisite: shared alpha-channel copy

Implementation: generic Core ImageCompositeMath::CopyAlpha and Image
PixelMath::CopyAlpha (serde copy_alpha) retain destination RGB and copy/opacity-
interpolate only alpha with existing rounded/255 arithmetic. Runtime maps it
through the same bounded image graph, clipping, row cancellation and ownership
path. No native VM pixel loop or new renderer was added.

Compilation/behavior: Image selected release1/1 passed (six opacity values,
negative destination and RGB retention); Runtime new image_copy_alpha2/2 plus
existing image_composite_clip3/3 passed, all0ignored. The latter independently
verify negative-source/destination clipping phase, stored alpha including0,
foreign client rejection before admission, retained Ready resource identity and
input cleanup after dropping the query. Logs: system4-copy-alpha-pixel.log and
system4-copy-alpha-runtime.log. Source is frozen pending NNN's single shared API
documentation regeneration. This source is newer than the frozen435040-
instruction owner; it does not claim native DrawGraph/plugin/visual acceptance.
DrawPluginManager.Load is still explicitly unsupported; the public reference's
success stub is not used. No new desktop build, game rerun, commit.

### 2026-11-01 — System-NNN native builtin persistence and visibility

- Implementation: ordinary foreground ENDFILM/ENDKOMA/ENDIF follows the original
  immediate-resume policy, distinct from CBackExec's actual shared frame. Native
  savesystemfile now waits for scoped CivilTime and atomic ClientStorage writes
  before committing the exact expression candidate; layer changes shared Layer
  visibility and the next retained Canvas contents. Native SystemFile bytes,
  timestamps and mask policy stay in the adapter. ResultData, resource-bound
  argument owner plans and scene-replay transitions remain explicit failures.
- Compilation/behavior: foreground selected release67passed/1ignored
  (`native-foreground-control-family-tests.log`); final builtin release lib
  39passed/1ignored (`native-builtins-system-save-layer-final-lib.log`). The
  first builtin run36/2failed/1ignored had authored fixture errors (same-game
  save directory and backend-less Snapshot pixel sampling), corrected without
  relaxing production behavior. Final tests cover actual save bytes, retained
  pending PC, one-time argument commit, preflight failure, real shared visibility
  and retained frame content, unchanged clock/other game save directory.
- Limits: no newer Desktop artifact/GPU or owner-game acceptance; original
  dataN live restore and complete outer mode/function families are unfinished.

### 2026-11-01 — Shared procedural layer motion and alpha-only image API reference

- API registration/docs: registered shared ExternalMotionPose/Stamp/Status and
  the external-motion source module; generated real Layer producer methods,
  setRect, MotionStatus.drivers and SpriteTransform.local_matrix. The same
  reference includes Core/Image CopyAlpha. Previous verified fingerprint
  `9dac24fecb3e8275` became actual `42a8c3604407b796`. No fake native-only
  ClassRegistry entries were created for Rust kernels.
- Validation: regeneration initially7/8 with only missing actual CHANGES entry;
  after logging it, independent no-bless `api_docs`8/8 and `api_coverage`1/1
  passed (`systemnnn/api-external-matrix-copy-alpha-verify.log`). Implementation
  owners separately validated shared Scene75/0 and Engine public APIs, and
  CopyAlpha pixel1/Runtime2+existingclip3; these are not native game acceptance.
- Contract: generation/revision strings are exact u64; native external producers
  commit real pose/completion atomically, skip request is not completion, and
  active host continuations must retire before snapshots. Affine local matrices
  preserve supported shear/projection instead of guessed Euler conversions.
  Alpha-only composition retains RGB and existing clipping/opacity rules.

CopyAlpha API follow-up: NNN's final common regeneration produced actual
42a8c3604407b796 (including concurrent external-motion/matrix changes); no-bless
api_docs8/8 plus coverage1/1 passed. This records generic engine API availability,
not native DrawGraph/plugin completion. System4 owner artifact remains the
separately frozen435040/main=true/Load-retained source point.

### 2026-11-01 — System-NNN complete script weekday/holiday family

- Implementation: original current-bank date/year bindings and optional exact
  holiday tuples now drive both week/holiday calculated variables. Pure
  Gregorian arithmetic comes from Core civil_weekday; capability resource reads
  run in the existing bootstrap worker. No private calendar algorithm, ambient
  file read, wall-clock substitution or batch-stale calculated constant.
- Compilation/behavior: release native lib42passed/1ignored
  (`systemnnn/native-calendar-family-tests.log`), including three native calendar
  fixtures plus builtin save/Layer/foreground/close regressions. Python tooling
  201passed (`native-calendar-python-tooling.log`). Static authenticated original
  GetCalcuVarData/GetWeek/holiday gate/constructor/matcher reviewed independently
  of later public-source differences, recorded in native calendar source audit.
- Limits: invalid Gregorian script fields explicitly fail; system-clock fields
  and remaining calculated service groups still need their real dependencies.
  No Desktop rebuild or owner-game run and no native live-save importer claim.

### 2026-11-01 — System-NNN original calculated preference groups

- Implementation: actual legacy GetSystemParam/constructor source maps only
  `expmode1..5` and `expcheck1..5` to original SystemFile data, although both
  stored arrays have16 slots. The owner freezes these logical native values
  into the existing worker admission; ordinary stored words remain separate.
- Compilation/behavior: release lib43passed/1ignored
  (`systemnnn/native-calculated-preference-family-tests2.log`). The new fixture
  runs all10 original postfix assignments on the actual VM worker from encoded
  native save bytes, checks later-name ordinary fallback, unchanged original
  stored words, and no Scene/clock/host operations. Initial authored compile
  only corrected mutation of immutable test snapshot bytes via to_vec.
- Limits: wheel, per-read host-time and progress getters remain explicit gaps;
  complete native UI/outer modes/original live saves and actual game acceptance
  are still pending. No Desktop artifact or GPU result is implied.

### 2026-11-01 System4 real DrawGraph channel binding and native MsgSkip family

Implementation: DrawPluginManager now requires a real scoped engine Image
interface, keeps its exact native loaded-name table, and translates six bounded
RGB/alpha/RGBA copy/fill exports through common Image workers, immutable engine
ImageResources and retained Layers. Shared CopyAlpha was independently verified
before this adapter mapping. Remaining draw exports, unknown plugins and
alpha-less destination profiles explicitly fail; no private compositor is used.
Native System10/12/14/16 file queries/location metadata use scoped capability
storage and case-aware worker receipts. SaveFolder is a native logical alias
for that client's Save mount, with readonly original SaveData fallback.

The complete reviewed2004 MsgSkip10-export family uses exact DWORD-count/MSB
bit records and shared Core integer_fields. UseFlag, padding counts, repeat Init
and malformed engine-save reset follow original DLL evidence instead of public
warning stubs/migration guesses. Normal halt, close during Sleep and fatal
unsupported HLL wait for actual atomic flag-write completion; original pending
PC/wait stays unacknowledged. Read/image/time dependencies are cancelled at
shutdown; original game files are never modified. This is read-history policy,
not original global/group/resume save or VM/resource reconstruction.

Compilation/independent behavior: Draw channel library17/17, sact1passed2ignored,
sprites2/2; later NativeFiles library20/20; final MsgSkip library26/26 with six
MsgSkip cases including actual VM normal-halt/close/error paths. Logs:
system4-draw-channel-family.log, system4-native-files-module.log,
system4-native-msgskip-final.log. First MsgSkip run25passed1failed because the
authored invalid-layout fixture was rejected before bootstrap; corrected only
the fixture to a valid unknown HLL, leaving production checks strict.

Original media behavior remains separately frozen: e4201dab... owner executes
435060 original instructions/main=true with real DrawGraph loaded, then retains
System12 at pc0x1167e/SACT_MSG_SKIP_INIT; explicit1passed0ignored/0.56s. That
artifact predates NativeFiles/MsgSkip and proves no title/render/gameplay result.
The newer owner build is still pending at this entry. No Desktop rebuild,
commit, push. Public API shape unchanged since independently verified
42a8c3604407b796; private module changes need no fake script classes.

### 2026-11-01 — Shared byte text kernel API reference

- Documentation/registration: included Text byte_replace.rs and printf32.rs;
  registered Rust-only Printf32Argument/Printf32Output with their actual Word(u32)
  and borrowed Bytes tuple types. README documents replace_spans_bytes and
  format_printf32_bytes boundaries while preserving format_one_bytes semantics;
  no script reflection class or VM-specific text service was fabricated.
- Generated fingerprint actually changed from `42a8c3604407b796` to
  `ceb843236c48cb69`, recorded in CHANGES. Regeneration initially7/8 with only
  the missing hash entry; final independent no-bless release api_docs8/8 and
  api_coverage1/1 passed (`systemnnn/api-byte-text-kernels-verify.log`). The
  existing capability test also checks both tuple payloads and absence of a
  fake Printf32 script class. git diff --check passed.
- Independent shared Text validation has42passed/3ignored. Those source
  tests and Python tooling do not establish new owner-game/GUI acceptance.
  This documentation batch changes no NNN/native source, Desktop, GPU, repository history.

### 2026-11-01 System4 original MsgSkip lifecycle acceptance

Behavior: newer frozen CPU owner e0abb3dad2f1c62bdb75dd1fdfdb32e4ee8956f8f01accfcbb88b1961a94c883
explicitly passed1/1 in0.53s. Actual Rance7 executes435187instructions/main=true,
initializes native92120-message MsgSkip state, and commits its read-history
atomic save before exposing later unsupported SACT2.SetWP at pc0x15bde in
WP_SET_COLOR. Native pending args/PC remain intact. The game files are readonly;
test writes use its private temporary engine state. Log:
system4-native-msgskip/owner-sact-host.log. Separately, scoped VM35passed1ignored
and sact1passed2ignored passed in system4-native-msgskip-owner-build.log.
This is no title/render/gameplay or full native-save acceptance. Next complete
scene.c background family is source work in progress, not part of that artifact.

### 2026-11-01 System4 source-family background and array-sort migration

Implementation: complete scene.c SetWP/SetWP_Color/WP_SetSP background setters
now use shared Layer/Image services. Numbered CG waits shared header/pixel
fingerprint receipts; missing/corrupt inputs preserve previous wallpaper.
WP_SetSP creates an immutable opaque RGB snapshot through common CopyRgb,
independent of source position/show/opacity and later source delete/repaint.
No private pixels/compositor or original renderer is linked.

A_SORT/A_SORT_MEM rank1 scalar/string/member/custom families execute as bounded
VM continuations. Owned string copies/borrowed struct refs, original stable-index
fallback, genuine AIN callbacks across HLL waits and heap sequence/layout guards
are preserved. Cancellation leaves native values unpublished. The deterministic
merge callback schedule does not emulate platform qsort invocation order or
unstable equal-key permutations; side-effect-sensitive ordering is not claimed.
Multi-rank and unreviewed callback value profiles fail explicitly.

Compilation/independent behavior: combined release82passed3ignored across
library30/find7/sort9/sact1/VM35 (system4-native-scene-sort-final.log). Final
narrow comparator-type guards plus member-string/raw-float-word/cancel/reused-slot
checks passed11/11 (system4-native-array-sort-guards.log). First background
compile had an authored double-reference contains typo, corrected before tests.
Bundled Python ran201tests OK(1skip), native-msgskip-scene-python-runtime.log;
default XcodePython3.9 first attempt had6 tomllib import errors and is not a pass.

Behavior limits: latest actual original owner remains frozen e0abb3...,
435187instructions/main=true/92120-message MsgSkip, predating this scene/sort
source. No repeated per-opcode original game probe, Desktop build, GPU, title,
full native save or gameplay claim. Remaining DrawGraph/text/UI/effects/audio,
AliceLogo seasonal animation, delegates and manual presentation fence require
further full-source migration. No public API shape changes in these private
modules; previous shared docs receipt remains separately verified. No commit,
push.

### 2026-11-01 — Shared audio decibel API documentation

- Documentation/registration: included Audio levels.rs as shared Rust source and
  documented all four finite/legacy dB conversion functions. Audio.setVoice's
  actual registry description now exposes gain_db100 and its mutual exclusion
  with gain, exact finite -100dB versus explicit silence, and linear fade units.
  README/CHANGES preserve those distinctions and the prior Text history.
- Actual generation8/8 passed and fingerprint stayed `ceb843236c48cb69`:
  parameter/return Map types are unchanged and Rust free functions add no DTO or
  reflected class. No new signature hash was invented. Independent release
  no-bless api_docs8/8 plus api_coverage1/1 passed
  (`systemnnn/api-audio-levels-verify.log`); git diff --check passed.
- Implementation evidence is separate: Audio owner's shared-audio-levels-tests
  has36passed/2ignored and actual public Audio tests3/3passed. This reference
  update adds no new NNN/native source, Desktop/GPU run, commit or hardware
  claim.

### 2026-11-01 — Live isolated Canvas API reference

- Documentation: generated CanvasCommand.present_isolated and the real
  Canvases.apply mapping, with README/CHANGES explaining live immutable snapshot
  refresh, post-composition parent opacity/transform, preserved ordinary Present
  and identity-only snapshot inlining. Source's missing COMMAND_API mapping was
  found during registration review and added by the implementation owner before
  validation; no placeholder class or private renderer was introduced.
- Actual fingerprint changed `ceb843236c48cb69` -> `7139e6a8e9650eac`.
  Regeneration7/8 initially needed the actual CHANGES hash; final independent
  release no-bless api_docs8/8 and api_coverage1/1 passed
  (`systemnnn/api-canvas-isolation-verify.log`), and git diff --check passed.
- Implementation evidence is separate: Scene lib50+isolation4+subtree9+matrix3
  totals66passed/0ignored; actual independent public Engine API1/1passed.
  These are CPU resource/state/geometry and typed-public-API checks, not GPU
  visual acceptance or a native-game completion claim. This documentation batch
  changes no NNN/native/renderer code, Desktop, repository history.

### 2026-11-01 — Independent stereo gain API reference

- Documentation/registration: listed the complete existing Rust AudioRequest
  capability enum, including SetStereoGain's real channel/gains/fade_ms payload.
  It is not a JSON command or reflected class. README documents Mixer control/
  observation and shared validation, rendered voice-frame ramps, atomic mixed
  options validation, worker ownership and post-pan/pre-Master/reverb placement.
  Existing Audio.setVoice keeps its options Map and adds stereo_gain semantics.
- Actual fingerprint changed `7139e6a8e9650eac` -> `5ba5be1c88e5f542` for the
  newly documented typed enum; the reflected Map signature itself did not
  change. Initial generation had coverage1pass/docs7pass with only the missing
  hash entry; independent release no-bless api_docs8/8 and api_coverage1/1 then
  passed (`systemnnn/api-stereo-gain-verify.log`). git diff --check passed.
  Existing capability tests validate the exact typed stereo payload and ensure
  no fake AudioRequest class is generated.
- Implementation evidence remains separate: Audio8 targets78passed/2ignored
  and real Engine public audio4/4passed, including actual PCM and worker-lease
  behavior. These do not establish new device/native-game/GPU acceptance. This
  documentation batch changes no other production code, NNN, Desktop, repository history.

### 2026-11-01 System4 shared finite audio transport and metadata source family

Implementation: common Core/Audio now owns positive finite loop-pass counts,
dynamic decoded-frame loop ranges and decoded L/R exchange. Remaining pass
counts come from the real mixer, including fractional/high-rate boundaries;
existing unbounded-loop defaults remain unchanged. Own gain/current target/
remaining fade frames are observable. A real gain-to-target ramp can retire its
voice without adding a second zero envelope; pause/starvation, source completion,
independent transport deadlines and worker publication remain distinct. Public
Audio setVoiceLoop/fadeVoiceGain/finishVoiceGainFade and setVoice swap_channels
reach these same resources. No System4 mixer/DLL is hosted.

Compilation/independent behavior: common release Audio55passed0ignored across
8targets (system4-shared-gain-loop-atomic-tests.log); public Engine13passed0ignored
across audio_loops2/audio_frame5/audio_mixer6 (system4-shared-gain-public-tests.log).
Shared metadata3passed1ignored (system4-audio-metadata-tests.log). Explicit original
metadata owner1/1 reads44old-format music records and2219version3 wave routes;
frozen77798a33... at system4-audio-metadata/owner.log. No game bytes are committed.
The first public test command typo named nonexistent async_channels; it did not
compile. Another public attempt met a concurrently edited adapter compile error,
fixed by that owner before final passing verification; neither is counted as pass.

Native source migration: new NativeAudio is currently test-only, not routed into
NativeAdapter. Its resource/finite-loop/swap/fade fixtures plus prior lib tests
passed33/33 after correcting the authored Runtime/Host setup method; final native
export-ABI fixture guard verification is still pending. Stop/seek/replay must
preserve actual decoder/fader state and terminal receipts before production
routing. Remaining native audio/sample/FFT, DrawGraph/text/effects, AliceLogo,
delegates and original saves still need complete semantics. Original game/title
receipt remains e0abb3... from before this batch. NNN joint API gen measured
feb2ec9be29523ba; final no-bless verification is pending at this entry. No Desktop
build by this child, GPU/gameplay claim, commit, push.

### Shared original-save staged restoration and unified API reference (2026-11-01)

Implementation: Core now exposes an opt-in native restore preparation/commit
contract; Runtime owns instance/client/epoch/serial-scoped receipts, bounded
resource admission and scene-validation workers. Polling holds only the target
VM and never advances VM/input/clock. The preparation host grants read-only
storage and engine-owned immutable image/pattern/probe work. Published images
reserve shared quotas without becoming live; Ready commit validates latest
scene state, atomically installs generations/retirements and swaps native VM
plus scene. Cancellation, failure, framework/native restore and close obsolete
preparations without partial live changes. ADR0099 records the ownership model.
Audio replacement, World replacement and remaining restored-client external
producers explicitly fail in this initial profile; complete NNN dataN live
restoration is not claimed. Existing synchronous adapters remain compatible.

Compilation: release shared check passed in native-restore-worker-check.log.
Independent behavior: native-restore-worker-tests.log contains6passed/0ignored,
using synthetic first-class VMs and actual ImageJobs without game/device data.
They cover pending invisibility, resource/scene-validation dependencies,
read-only preparation, atomic commit and old immutable generation retention,
wrong client/runtime, cancellation/epochs, retained failures, bad audio/leases,
and preservation of another client's external motion producer. Python tooling
201passed (native-restore-python-tooling.log). No Desktop/GPU/game run by this
batch, no original save compatibility or full NNN completion claim.

Unified API regeneration measured feb2ec9be29523ba, preserving previous5ba5
history. It includes the host-only restore Rust contract, Media gain,
pointerActivity, ActivityTimer/integer interpolation, Audio finite-loop/channel
swap/gain completion and shared Alice metadata contracts. HG3 existing entry
metadata is documented without inventing a native completion/API class.
No-bless api_docs8/8 and api_coverage1/1 actually passed, exit0 in
api-native-restore-audio-media-input-verify.log. No commit, push.

### Alice shared retained channel seek and native mixer source batch (2026-11-01)

Implementation: shared Audio/Core/Runtime/public Audio now provide scoped async
channel seeks over the existing decoder. Retained completion survives same-frame
EOF, preserves gain/loop/pause state, and is fenced by actual owner publication.
Cancelled/failed worker frames reject their unpublished seek results. Pending
cancellation realigns the prior logical cursor; old IDs cannot cancel newer
sequences. Receipts share bounded play-operation admission and reset invalidates
their IDs. This capability is usable without System4 or any native VM.

Compilation: release Audio/Runtime/Engine check passed in
system4-channel-seek-atomic-check2.log. Independent behavior: dedicated seek5/5
passed; final release Audio library41passed2ignored plus eight target55passed,
96passed2ignored total in system4-channel-seek-audio-regressions.log. The two
ignored tests were not executed. Public Engine audio_loops third test is authored
and awaiting the root's combined run; previous shared-gain Engine13/13 remains
historical evidence. NNN will regenerate Font/seek API references together;
feb2ec9be29523ba is the previous verified fingerprint, not this new shape.

### 2026-11-01 — System4 implementation and validation

Alice mixer/seek source-batch validation follow-up: root retry96018 actually
passed87 tests with1ignored across System4 runtime library34/34, public Engine
library3/3 and audio_loops3/3, and NNN47/1ignored. The native mixer fixture tests
all8 original methods, reference and invalid-index policy, exact C-name behavior,
client rejection and real shared PCM mute/gain. Public audio_loops includes the
new retained channel seek. Initial compile failure was an authored fixture cfg
missing `test`; it was corrected to `cfg(all(test, feature="runtime"))`, retaining
the existing dev-only tempfile dependency and every test. Joint Font/Seek API
reference measured f52fcaec3a9ab322 and no-bless8+coverage1 passed. Independent
bundled Python tooling ran201tests OK(1skip), system4-audio-seek-python.log.
Prepared audio replacement is a later source point with release check only so
far; these87 tests do not validate it. No original game/player/render result.

### Shared prepared per-client audio replacement (2026-11-01)

Implementation: `crysirror-audio::channel_replacement` captures bounded pure
Play/PCM/Asset/Resource recipes and scoped immutable resource leases, then opens,
decodes, seeks and prewarms detached sources on workers. It registers no live
voice. Final owner staging checks latest slots, generations, group bindings and
client capacity, and holds an exclusive borrow until install or rollback.
Infallible install moves the prepared voices into prevalidated slots and retires
only that client's voices/tails/queued play/seek/sequence state. The constructor
is shared with ordinary playback; live Mixer/decoder/history is never cloned.
Mixer instance, reset epoch and per-client commit revision reject stale results.
Peer configuration, stream leases and DSP histories remain intact. Existing
resource catalog/group/bus/listener configuration stays valid; reconstruction of
the restored client's own reverb history explicitly fails in this initial profile.

Compilation: release Audio check passed, system4-audio-replacement/check.log.
Independent behavior: nine new release tests passed. Final combined regression
actually passed118tests with2ignored: Audio library50/2ignored and eight target55,
Tasks library7 and operations6, system4-audio-replacement/regressions.log. It
covers actual WAVE capability reads/codec seek/prewarm on workers, accounting
until the last stream lease drops, scoped and obsolete results, invalid late
resources/groups/capacity without partial change, dropped preflight, same-client
revision and peer-commit isolation, real PCM controls and bit-exact peer reverb
tails. The ignored cases were not run. This is a standalone shared audio result;
NNN owns the still-pending Core/Runtime restore wrapper and next API regeneration.
It does not yet establish original dataN or Alice native stop/replay/save support.
No new game, Desktop executable, device/GPU run, commit, push.

### Shared resident pixel access and Alice stored-pixel queries (2026-11-01)

Implementation: Core now has an opt-in resident ImageResource lookup and checked
allocation-free RGBA helper, with scoped Solid/Resource ImageInput sampling.
Unsupported providers and nonresident graphs fail explicitly; there is no
fallback to a potentially materialized full rgba image. Runtime/Engine resource
implementations directly borrow their existing resident bytes. Alice's complete
four stored-pixel HLL wrappers use that API, retaining native try-vs-lazy handle
allocation, RGB-only alpha=-1, checked reference outputs and original failure PC.
No adapter raster loop, original GL readback or second renderer is introduced.

Compilation: System4+Engine release check passed. Independent behavior: release
Core2/2 and NativeDraw6/6 actually passed,0ignored, in
system4-resident-pixel-tests.log and system4-native-pixel-tests.log. The latter
includes two new actual shared-resource/reference tests and four prior
Copy/Fill/Load regressions. Core uses a provider whose full rgba method panics,
so these cases establish that lookup does not silently materialize it. Stored
bytes are checked independently of display composition. Source review records
complete pinned DrawGraph/SACT2/sprite/video modules; original undefined outside
RGB readback remains a visible unsupported profile. Next API regeneration is
coordinated with restore/replacement work; f52fcaec3a9ab322 remains the previous
verified reference. No original game/title/GPU/player result in this batch.

### 2026-11-01 Shared styled unit flow and glyph-bound preparation

Compilation: the joint release invocation flow-bounds-native-batch-tests.log included Engine3, public-font/flow2/1ignored, Runtime80/1ignored, precomputed2, System-NNN49/1ignored, native-shell5 and Text45/3ignored. Standalone unit-flow3/3 also passed. Engine cases exercise the scoped glyph worker, retained Ready metadata, foreign-client denial, baseline-derived bounds and public layout. This batch did not run a game, GPU or desktop build.

### System-NNN outer return / legacy Title initialization source family (2026-11-01)

Implementation: Title auto-restart/script-END and native button precedence now
wait for their corresponding completed shared frame. Restart transfers the real
variables/RNG/Bridge into Nothing/Logo/Opening without VarInit or SystemFile
reload; Opening uses real left/right click or background END and drains admitted
work without forged NEXT/ACK. Shared ImageJobs/immutable covers retain visible
handoffs and the exact terminal Logo fill. The reviewed zero General fade
profile selects ordinary/special capture conditions; unsupported nonzero effects,
delayed voice and other mode consumers still fail explicitly.

Actual source review corrected a prior inference: Game vtable+240 targets534b00,
which posts WM_CLOSE; it does not directly call EndGame. Native close uses the
existing drain/two-SystemFile-write protocol. Title.Init follows actual52f5f0's
once-per-process saturated launch count, actual52f4c0's8sound/4voice stops and
53fa30's work-enable reset. It does not copy later variable-clearing loops or
stop another client's music/DSP. Provenance/evidence is recorded in the native
Title source audit; the PE was only read statically and remains outside Git.

Compilation/behavior: native-title-init-return-final-tests.log exit0,
67passed1ignored across lib49/installed2/movie5/shell5/title6. Root's later
flow-bounds-native-batch-tests.log revalidated current lib49/1ignored +shell5,
including the final terminal-fill/reentry path. Earlier complete restart fixture
used only4sprite pixels for4original2pixel states and correctly failed; only
its authored width/header were corrected to8, no native bounds relaxed.
No new Desktop/GPU/Title screenshot, Start/story or full System-NNN claim.

### Shared audio preparation joins native restore transaction (2026-11-01)

Implementation: Core AudioReplacementJob returns a retained, scoped, opaque
one-shot resource; preparation only grants pure audio recipes and changes no
live playback. Runtime validates actual instance/client provenance and holds
Audio's exclusive prepared commit token after all final checks. Immutable image
publication, infallible channel replacement and VM/scene swap have no later
fallible action. Empty replacement clears only target channels/tails/intents;
peer voices and DSP are preserved. Arbitrary sequential audio commands and
remaining target Media/World/classic-audio/external-motion ownership remain
explicitly unsupported, including retained EOF/media updates.

Compilation: native-restore-audio-check2.log release passed. Independent behavior:
native-restore-audio-tests2.log9passed0ignored, exit0; includes real PCM equality
against independently installed sources, peer retention, empty clear, cancelled/
foreign/consumed handles and late scene/image failures without partial install.
An initial World query used an iterator method on its Vec result; it failed compile and was corrected before
this passing receipt. NNN original dataN still has no live importer enabled;
this common capability alone is not engine/game completion.

Font/seek API previous batch measured f52fcaec3a9ab322 and passed unblessed
api_docs8/8+api_coverage1/1 (api-font-channel-seek-verify.log). Current shared
font/input restore guards and newer public capabilities await their own final
behavior/API generation; that older fingerprint is retained as history.

## 2026-11-01 — Alice reviewed optional music interface policy

Implementation:
- Static original MusicSystem QIF analysis confirms the three optional pause queries are absent in the exact Rance7 module pair. The first-class adapter reads the configured DLL directory through scoped startup workers and requires both exact module SHA256/lengths before preserving the original facade return0. Missing/changed modules admit no profile; initialization, ABI and client checks remain. No DLL/original engine is executed. Full Sound/Music playback remains unregistered pending canonical retained-resource semantics.

Compilation and independent behavior:
- Release native_audio filtered run passed6 with1ignored (system4-reviewed-music-profile-tests2.log). Parent explicitly ran the frozen original-owner readonly profile test:1passed0ignored, system4-owner-music-profile-test.log. This only verifies scoped INI/module reads and exact profile binding; no new original-game or renderer result is claimed.

### 2026-11-01 — Original restore: static fonts and target-scoped input

Implementation: shared typed font-dependency visitors cover retained Layer,
Canvas (including unpresented groups/snapshots) and UI resources. The original
restore profile admits exact static project aliases; it explicitly rejects
unknown/dynamic fonts rather than treating a transient is_loaded query as an
atomic lease. Staged UI/input-routing reset retires only the restored client's
old delivery epochs, events, focus and captures, preserving physical held state
and peer delivery. No live reset occurs on preparation/failure.

Compilation: release font/input checks passed (native-restore-font-check.log,
native-restore-input-check.log). Behavior: native-restore-font-input-tests3.log
passed14 with1ignored: Runtime original_restore11/1ignored plus Scene client
input2/font visitor1. The system-font ignored case was then explicitly executed
and passed1/1 (native-restore-static-font-owner.log); Arial.ttf was read at runtime
only. Python tooling rerun201/201 passed (native-restore-python-tests.log).
This validates shared restore mechanisms, not completed System-NNN originaldataN
consumer wiring, game/title/Start acceptance, dynamic font transactions or GPU
rendering.

### 2026-11-01 — Public restore/audio/font-flow reference synchronized

Implementation: registered the actual Core detached AudioReplacementInfo/State
and Audio's three opaque replacement values, updated OriginalRestoreCommit's
prepared_audio field, and registered UnitFlow's four and GlyphBounds' four
public data contracts. Rust traits, typed font/input visitors and opt-in
resident pixel lookup are described as capabilities rather than invented
script classes. Existing HG3 metadata documentation now includes the native
pixel profile. Native consumers/game completion remain separate.

Generation: one actual bless produced a7e4bc50985744b6 from f52fcaec3a9ab322;
the initial run passed coverage1 and API7/8, failing only the required missing
CHANGES entry. After recording the observed fingerprint, release no-bless
api_docs8/8 and api_coverage1/1 passed in
systemnnn/api-restore-font-flow-verify.log. The opaque-field and exact typed
Failed payload assertions passed. No Desktop/GPU/device/native-game result is
implied. Python tooling201/201 also passed this batch.

## 2026-11-01 — Shared positioned glyphs to immutable image resources

Implementation:
- Added device-free GlyphImageSpec and optional ImageMetadata.begin_glyph_image. Runtime freezes only the authorized font and prepares ordinary ImageResource pixels on existing bounded image workers; publication remains explicit and client-scoped. Output/scratch reservations retire with resources/cancelled closures. TextSystem uses one regular coverage kernel through a new cancellable entry point; outline/precomputed paint checks work and cancellation between glyphs/rows. No VM, renderer or font binaries are required.
- This is the stored-text-image prerequisite for native adapters. Alice DrawGraph/SACT text is not routed yet: native bold/block metrics, RGB blending and per-glyph AMap overwrite cannot be replaced by a whole-run union mask or overlay.

Compilation and independent behavior:
- Core/Text/Runtime release check passed. Text release library passed46 with3ignored. Runtime first library run passed81 with1ignored and one new authored test race; the test had equated scheduler counter retirement with the later closure/resource drop. After checking the actual Task lifecycle, only its wait assertion was corrected, and both exact cancellation/admission tests passed2/2. Production was not loosened.
- The same compiled production point's frozen glyph_images and precomputed_fonts binaries passed2/2 each, including actual coverage pixels, CopyAlpha, font release/frozen retention, cross-client rejection, invalid glyph failure and explicit resource publication. Artifact SHAs/logs are in target/verification/system4-glyph-image/. Python tooling passed201 with1skip. Public docs are being regenerated separately; no unverified new fingerprint or original-game success is claimed.

### 2026-11-01 — System-NNN canonical saved audio work and shared preparation

Implementation: independently expressed original16-word loop-work layout with
explicit12-word callback reordering; preserve native11..15 and64-byte saved
names. STOP/Title clear native loop word0. SavedAudioPreparation validates the
complete original file profile, resolves sources on a cancellable worker and
prepares shared Core AudioReplacement resources; it does not synthesize a VM
receipt or alter live playback. Reviewed single-source music0/1/N uses shared
counted loops. MML/stepped native fades/spatial profiles remain explicit gaps.

Source evidence: read-only actual536740 proves8sound/4voice restart;531dc0 and
531370 prove direct legacy clamp and saved-name replay, distinct from later
public-source delta=-101 behavior and newly received62-byte names. Pinned source
and actual-module hashes recorded in nnn-native-audio-save-source-audit.json.
Compilation and behavior: release final native-audio-save-work-final-tests.log
passed57 with1ignored (8new cases), including real PCM count completion and
actual engine WAV replacement readiness, peer/old playback preservation,
zero-clock/late failure/profile rejection, raw names/work and63-byte replay.
Initial56/1 was before final module-profile/name/profile guards; not the final
receipt. No original game/GPU/Desktop was run. Native dataN complete VM/Bridge/
message/selection/effect import remains unsupported, not marked complete.

## 2026-11-01 — System4 complete native delegate instruction family

Implementation:
- Migrated the complete pinned16-op DG_* source family into the first-class VM. Weak object/function/sequence triples preserve order, duplicate policy, expired-object collection and original self-subtraction iteration; they never retain a method instance as a strong owner. Callback locals/typed argument copies follow original allocation order, retain native bytecode/external-wait continuations, and clean caller arguments only after the last callback. Shared engine APIs and the public Core shape did not change.

Compilation and independent behavior:
- Local release check passed. A single release --no-fail-fast combination passed80 with2ignored: System4 runtime library38/1ignored, existing VM35/1ignored, new authored native-bytecode delegates7/0ignored (system4-delegates/tests.log). Tests exercise weak generation reuse, bound/static callbacks, unchanged instance ownership, clone during pending original HLL, per-callback owned argument isolation/final release, last-return semantics and invalid native PCs. There is no new original-game/GPU/save acceptance. Other AIN versions and incompatible callback declarations remain explicit failures.

### 2026-11-01 — Shared glyph-image / prepared-layer batch API final receipt

Implementation: documented the actual GlyphImageSpec optional Rust capability,
ServiceBatch six-section DTO and Canvases.applyBatch with prepare_layers before
Canvas inside the existing engine transaction. No fake reflective glyph class.

Generation: Glyph-only observed081fcf5a46b10d77 could not pass no-bless after
concurrent applyBatch source landed; that failed7/8 run was not counted as a
pass. Final actual regeneration produced1bea1d048c6dd276. After recording that value, release no-bless
api_docs8/8+api_coverage1/1 passed(api-glyph-service-batch-verify.log). Prior
verifieda7e4bc50985744b6 history is retained. No Desktop/GPU/native-game result
is inferred from these documents.

### 2026-11-01 — System-NNN detached audio bridge and saved name boundaries

Implementation: native restoration now seeds immutable Arc name tables and
shared worker/resolver capabilities, prepares real Core replacement sources,
then constructs a detached AudioBridge without copying a live mixer. Original
export merges canonical current fields with retained raw work/name tails.
Native x86 volume addition wraps before clamp. Restored63-byte CP932 names use
direct replay policy, so a final double-byte character is not cut as if a new
62-byte callback had been received. Whole dataN import remains in progress.

Compilation/behavior: release native-audio-restore-seed-tests.log58passed1ignored;
then native-audio-restore-name-boundary-tests.log58passed1ignored after adding
the second CP932 name fixture to the same case. This includes actual shared
WAV preparation, detached bridge/raw export, old/peer isolation, PCM repeats,
failed preparation retention and wrap boundaries. No original-game run here.

## 2026-11-01 — System4 canonical audio and shared retained transport

Implementation:
- Shared Audio now supports opt-in retained canonical sources and an independent input/DSP activity switch. Natural EOF or an own-gain stop keeps its actual source, cursor, gain and loop observations; retained sources consume pool capacity and cannot be stolen. Inactive control clocks continue, while full Pause still freezes controls. Replaying EOF requires a completed real seek. Explicit Stop and per-client replacement retire the same ownership; there is no second held-source bank.
- System4 preparation now acquires an engine AudioResource and asynchronously prewarms one inactive shared voice. Play/Stop reuse it through retained seek receipts. Native fade/finish/observations use common Mixer state. File preparation uses scoped case-resolution workers. Cleanup retains each authority until its release succeeds. Exact original MusicSystem optional-interface policy is guarded by both module hashes, not inferred from public warning stubs.

Compilation and independent behavior:
- Final release combo compiled and passed137 tests,3ignored, across9 targets: Audio lib51/2ignored, channel_async6, counted_loops8, gain_fades6, retained_transport8, transport9; Engine lib3/audio_loops4; System4 runtime lib42/1ignored. New PCM tests verify no source reopen, inactive-versus-pause clocks, worker publication, real EOF/seek, ownership/pool behavior and replacement clearing target held sources while preserving peer PCM. Native tests verify prepare/stop/replay/EOF/file/partial-cleanup lifetime. Log: `target/verification/system4-retained-audio/tests.log`.
- Full native worker routing and halt/close/error cleanup have now been connected, with two additional original-HLL-to-shared-PCM tests queued separately. They are not counted in137. New public API generation is pending the coordinated NNN batch.
- Previous shared GlyphImage8192 negative case has now actually passed in Root4529 (`glyph_images`2/2); Glyph+ServiceBatch public fingerprint1bea1d048c6dd276 passed no-bless docs8+coverage1. These facts supersede earlier pending notes, without changing their historical source-point results.

Original media:
- No new game/title/render acceptance. Remaining Sound/Music optional/sample-data profiles, native control-only instance creation, text/effects, real logo animation and original saves remain explicit gaps. No original engine, EXE or DLL was executed.

System4 canonical actor routing follow-up: the separately compiled release library now passed44/44 ordinary tests,1ignored. Both new end-to-end original-HLL cases passed: Prepare/Play produced actual shared PCM before native halt or close, and unsupported-HLL failure released audio while retaining the original PC/arguments. `target/verification/system4-retained-audio/worker-tests.log`; this is authored CPU acceptance, not original game/title acceptance.

### 2026-11-01 — System-NNN original text/selection wait consumers

Implementation: fixed original message rows now prepare shared GlyphRuns and
native window ImageJobs without replaying PRINT. Saved selection uses original
total-minus-supplemental counts and shared UI; a real activation resumes CASE
from saved nextPC without a fake callback, acknowledgement or SELECT count.
Core VM scheduling holds instructions until the restored wait completes and
blocks the load's triggering input. Native word180 is window variant selection,
not visibility. Name-colour tables are immutable Arc-shared across preparation.

Compilation/behavior: native-original-wait-consumers-tests4.log actually passed
65/1ignored in release, including original envelope/count64+4, real Core VM wait
and CASE/END path, no preparation Scene mutation, worker execution, two-phase
reveal and bad-CASE PC/variable/count preservation. Earlier fixture failures
(single-slot cancellation backpressure, expected END nextPC instead of its
retained PC47) were corrected without relaxing production. Complete dataN entry,
timed/special/window-variant consumers and original-game/GPU acceptance remain
in progress. Subsequent saved-effect/source-view work is a new unverified source
batch, not included in this65-test receipt.

### 2026-11-01 — Public viewport / retained-voice API receipt

Generation: documented Root's final viewport clip geometry and Alice's final
retained/active voice transport with the real Rust fields/enum payloads and
existing public facades. First coverage run correctly rejected missing
layer.set_viewport_clip COMMAND_API mapping; Root added its real facade map,
without relaxing the check. Actual regeneration then produced531a48a7ce30ee00.
After recording the observed value and retaining prior1bea history, release
no-bless api_docs8/8 plus api_coverage1/1 passed in
systemnnn/api-viewport-retained-audio-verify.log. Root's198/2 and Alice's137/3
independent behavior receipts remain separate from document generation. No new Desktop/GPU/game acceptance is implied.

## 2026-11-01 — Shared normalized/scalar RGBA and native Graph families

Implementation:
- Added reusable Core RGBA factor/equation/fragment/write-mask descriptors and explicit binary32-nearest or integer255 product arithmetic. Image's fixed-factor method shares the original checked blit visitor; immutable Runtime jobs retain ordinary scope, budget, cancellation and publication rules. Integer transforms and constants compile once per operation. No general renderer or VM pixel loop was added.
- Native DrawGraph now maps32 reference Copy/Blend/Fill/alpha operations to those engine APIs. Original scalar Graph policy adds ScreenDA with complementary-multiply table semantics, distinct from saturated AddDA. Native startup worker binds both reviewed original module hashes; unknown installed versions fail Load. Source metadata, filenames and wait/ABI policy stay in the adapter. No original DLL is loaded or executed.

Compilation and independent behavior:
- Final release combo passed311 tests,5ignored, across6 targets: Core33; Image136/2ignored and fixed_blend6; Runtime86/1ignored and fixed_blend3; System4 runtime47/2ignored. New tests include all65,536 multiply and screen byte-pairs, all-alpha separate-product truncation, immutable/scoped worker results, cancellation/clipping,32 native reference mappings,17 original scalar difference oracles and module guard failures. `target/verification/system4-normalized-blend/integer-tests.log`.
- An earlier307-pass generic batch included an authored SrcBright mistake that missed opacity in the thin native mapping. Complete source review found it; the mapping and independently asserted zero-opacity edge were corrected in the final311 batch. Historical pass counts do not turn that earlier branch into a correct native implementation.
- New API regeneration is coordinated with NNN and pending. Prior retained Audio/viewport fingerprint531a48a7ce30ee00 passed no-bless docs8+coverage1, independently of this new math batch.

Original media:
- Only static source facts and an as-yet-ignored double-hash owner test; no new original game/title/render result. Scalar table semantics do not claim identical historical MMX behavior. Native text/styles, filters/blur/gradation/overlay, transforms/wrap/effects, logo animation, presentation cadence and original saves remain work in progress.

Normalized RGBA follow-up evidence: the production `Bitmap.beginFixedBlend`
frontend now passed the separate Engine `shared_effects`6/6 release target,
including actual integer pixels, frozen source generation, foreign/revision/
cancel/invalid guards and TypeScript exposure. Root also explicitly ran the
immutable owner double-module profile test1/1 against Rance7 data; this only
verifies read-only original module identity, not game execution. Final full
Python ran205 tests successfully,1skip (`python-final.log`); the earlier live
`src/export.rs` ledger failure remains recorded in `python.log`. No native
title/GPU result has been added.

## 2026-11-01 — Source geometry cropping and complete native vector setters

Implementation:
- Added engine-owned LayerSpec.source_crop, SetSourceCrop and scoped Layers.setSourceCrop/sourceCrop. Cropping changes only the payload geometry/UVs before projection, preserving original content size, child transforms and live motion. Shared Scene CropRect/crop_rect/subrect_quad handle finite/empty rectangles, atlas or reversed/repeated mathematical UVs and homogeneous projected subrectangles. Existing retained-layer UV validation is unchanged.
- Adopted all five original vector setters (SET_POS48, SET_SCALE49, SET_ROTATE50, SET_CENTER158 and SET_CENTER_REP159), including both XY/XYZ signatures. The original early setter path preserves omitted Z; layout/width/signature preflight completes before mutation. Final review also matched original integer-center summation before f32 conversion, including cancellation across2^24. Original save format/schema16 remain unchanged.

Compilation and independent behavior validation:
- Explicitly invoked real offscreen GPU test passed1/1 (source-crop-gpu-final.log), comparing every framebuffer pixel with the original image inside the crop and background outside, while retaining immutable coverage keys. The initial GPU run already passed its pixel oracle but incorrectly required a fully hidden glyph to remain cached; only that fixture assertion changed. Earlier release attempts exposed an invalid reversed-UV fixture and the genuine missing native SET_CENTER command; both histories are retained in the receipt rather than described as passes.
- Unified generated API no-bless verification passed8 plus command coverage1 (systemnnn/api-seed-blend-crop-verify.log), actual fingerprint531a48a7ce30ee00 ->0154abd7b61f34df, covering source crop, Alice fixed/normalized blends and shared session seed material. Implementation, compilation, GPU pixels and native execution remain separate evidence.

Actual original-data behavior and limits:

### 2026-11-01 — Unified seed, fixed-blend and source-crop public contracts

Generated docs/api from the actual frozen public source: Core session seed material/query, six normalized/fixed RGBA blend types with the real qualified tuple payload, Bitmap.beginFixedBlend, and Scene source geometry cropping. The historical531a48a7ce30ee00 receipt is retained; this actual generated fingerprint is0154abd7b61f34df. The first generation correctly rejected the missing CHANGES entry and an authored test that expected an unqualified type instead of the real crate::image_blend::RgbaBlendState declaration. The declaration and coverage constraints were not weakened. After correcting the assertion and recording the generated hash, release no-bless api_docs passed8/8 and command coverage1/1 (systemnnn/api-seed-blend-crop-verify.log). The README now describes System4/System-NNN bounded native factories rather than the obsolete parser-only boundary; it explicitly does not claim complete compatibility. Underlying seed/blend/crop and GPU behavior are recorded in their owners’ separate receipts; no desktop build, commit was produced here.

### 2026-11-01 — System-NNN original dataN entry through shared atomic restore

Implemented: NativeAdapter and the installed first-class wrapper now admit original dataN bytes into the generic Core/Runtime asynchronous prepare/status/commit boundary. Original fixed context/variables, native status/player-name policy, effect records, DWQ picture preparation, fixed message/selection records and detached channel replacement are prepared through CrySirror workers/resources; no old native engine or composed legacy framebuffer is hosted. Preparation does not execute script, mutate Scene/time or start playback. Commit replaces only the target VM/2D state/immutable resources/audio after shared preflight. The installed wrapper preserves its outer-mode/close/SystemFile ownership. Native END still restarts controller context0 rather than inheriting data-VM halt semantics. Unscaled native picture clipping now uses Core's existing clip_region_copy; an empty copy is a genuine empty engine source crop, including after visibility toggles.

Compilation/behavior: native-original-entry-check5.log releasecheck passed. Frozen native-original-entry/lib-89191493c4f8c016 (full SHA in artifacts-actual5.json) ordinary library regression passed70/0failed/2ignored. Explicit licensed system-font Runtime test passed1/1 (runtime-actual5.log), exercising both NativeAdapter and InstalledAdapter: Ready keeps old VM/Scene/clock/owner+peer playback unchanged, commit replaces target audio while preserving peer playback, first load click only arms the restored message wait, subsequent release/press advances the saved nextPC. The earlier authored test incorrectly waited for all VMs to halt despite native END restarting sys; diagnostic runs showed43million valid controller instructions, and only the test expectation/input trace changed. Earlier wrong authored factory-key and timeout logs remain failures, not counted as passes. Python205tests passed in native-original-entry/python-tests2.log.

Limits: this is a synthetic original-format/runtime transaction, not a new real game/GPU/desktop receipt. Original slot-selection UI/export, Omake/Cutin/extensions, nonmessage/nonselection saved actors, active effect/window refresh while waiting, timed/special selection, nondefault native cursor/window variants and automatic-message consumers remain explicit incomplete boundaries. Full System-NNN migration remains active. Complete static InitLoadGame/effect-vtable audits record the actual old-module SUNA reset/global-RNG policy, distinct from the later public version. No commit, Desktop replacement was produced.

### 2026-11-01 Alice native font controls and atomic glyph-cell preparation

Implementation:

- Core/Text/Runtime now expose explicit glyph coverage style (EM/line-height, top/baseline, bounded supersampling, binary lookup reduction, the existing disk morphology, actual face decorations) and cached OS/2 style/glyph-ID observations. Existing default raster output is unchanged. Public `Text.glyphId` and `Text.fontStyle` use the same scoped font capability.
- A whole-cell batch freezes one authorized face and runs one worker. Every immutable cell retains its own output budget lease; full admission precedes queueing, and no partial cell list is published. Cancellation never joins. Native text prepares the whole string and commits one shared composition, rather than waiting one frame per character.
- Native DrawGraph's fourteen font controls and PMap/AMap text calls are production-routed. The adapter retains native CP932 byte units, fixed cells, original spacing remainder, raw font state/exact-one flags, and original 3x monochrome reduction lookup. Generic coverage, morphology, composition, resource budgets and workers live in the engine. Startup reads the reviewed DrawText2 module hash as data; unknown installed versions fail explicitly. No DLL code runs.
- Font matching is an explicit portable mapped-outline profile: actual OS/2 face weight is checked, and requested bold on a mapped regular face uses the engine's declared synthetic disk profile. This does not claim GDI face matching/hinting or historical byte-identical Windows glyphs. Missing metrics/unknown weights or unsupported bitmap-only profiles fail visibly.

Compilation:

- Coherent release check for Core dependencies, Runtime, Engine and System4(runtime) passed. The final nine-target release test invocation also exited0.

Behavior validation:

- Final `--no-fail-fast` combo:232 passed,9 ignored. Core33; Engine3 plus dynamic-font API2/1ignored; Runtime88/1ignored plus glyph2/precomputed2/styled5; System4runtime48/3ignored; Text49/4ignored. Batch tests cover one-task cancellation, full admission, late-cell failure rollback, immutable scope and independent retained cell leases.
- Three frozen binaries were then explicitly run against a runtime-selected system Arial font: original AIN HLL PMap/AMap1/1, public font API1/1, styled outline EM/top/decorations1/1; zero ignored in those filtered runs. The native test checks pending original PC/arguments, absence of partial sprite publication, PMap destination-alpha preservation, full AMap blank-cell overwrite and font-release-safe immutable output. No font files are committed.
- Python tooling:205 tests OK,1 skipped. Logs and executable hashes: `target/verification/system4-styled-glyph/`. This is shared/native API behavior, not a new game/title/GPU acceptance. The previous original-game instruction/main evidence remains historical. Remaining filters/transforms/effects, AliceLogo animation, original saves and further SACT coverage are still incomplete. New public shape is frozen for the next unified docs/API generation.

## 2026-11-01 — Owner SVG and transparent desktop icon assets

Implementation:
- Added the owner’s Glass Clover SVG unchanged as `icon/crysirror.svg` (source SHA256 `456627f43bf1ea1af2000f40870b9a6fb70f9671badd83500c5d73fa4ab3cd8f`). Added16 vector-rendered RGBA PNG sizes16–1024, a10-representation Windows ICO, and macOS ICNS/iconset with10 standard/Retina representations. Linux can use the shared SVG/PNG exports. All assets preserve transparent backgrounds and original composition.
- Added local `icon/generate.py` (Python standard library plus librsvg) and export hashes in `icon/manifest.json`; `icon/README.md` documents regeneration and platform packaging paths. These are prepared icon assets; no runtime, executable-resource or app-bundle wiring was changed. Previous engine tasks remain stopped.

Compilation:
- Generator syntax compilation passed. No Rust source, dependency or public API changed; no Cargo build/test was run for this asset-only change.

Behavior validation:
- Generated28 icon files with librsvg2.62.3. All16 PNG sizes passed RGBA/dimension checks and alpha inspection (transparent corners; alpha range0..1). Every ICO/ICNS embedded PNG matched the corresponding standalone PNG byte-for-byte. macOS `iconutil` decoded all10 ICNS representations successfully. Regeneration produced identical manifest/output hashes; the copied SVG remains byte-identical to the owner’s source. The1024px export was visually inspected. Windows/Linux installed-shell presentation was not exercised.

## 2026-11-03 — Documentation cleanup and current desktop entry points

Implementation:
- Rewrote root README from266 to97 lines around build/play, engine capabilities and documentation navigation; removed the obsolete source-export/publication instructions and claims that only the data VM, static 3D or offline JS existed. Added docs/README.md as the guide/evidence index.
- Updated desktop/MCP, architecture/systems, authoring, migration, saves and NScripter/Majiro guides; corrected the roadmap's obsolete missing physics/RT/audio rows. Recorded new source as implementation, without promoting it to complete-game or hardware acceptance.

Compilation:
- Not run: Cargo builds/tests and API generation. This task changes documentation while another implementation task is active. No new compilation, GPU or game result is claimed.

Validation:
- Checked173 Markdown files (excluding STATUS contents, generated API and vendor): no missing relative link targets. Documented CLI command variants, current desktop feature names/default player target and clipping cross-reference match source; documentation whitespace check passed.

## 2026-11-03 — Developer-facing README and project logo

Implementation:
- Corrected the README audience to engine developers/contributors: design, shared capability ownership, workspace architecture, development and technical references. Installed-game launch instructions remain in the desktop guide. Added the unchanged local transparent SVG as a centered160px logo above the project title.

Compilation:
- Not run: documentation-only change; no Rust source, build configuration, API or concurrent implementation work changed.

Validation:
- README relative file/image references and the engine example path resolve; the referenced SVG parses as SVG XML. Markdown whitespace check passed. No new runtime, browser-rendering or GPU test result is claimed.

## 2026-11-03 — ADR status reconciliation and retired hosting recipes

Implementation:
- Reorganized all99 ADR IDs into one index with91 active decisions/profiles,5 superseded decisions and3 historical audits; each ID appears once and existing filenames remain stable. Removed the parallel-task table fragmentation and reconciled follow-up statuses.
- Added explicit bounded implementation states to0085–0096 where recorded source/evidence already existed. Completed scopes remain as design records.0050 full-engine async,0097 interpreted filters and0099 incomplete native consumers remain open. No new engine, game or hardware completion is asserted. Only ADR Markdown and this appended STATUS section changed in this task; concurrent implementation, source ledgers and generated API docs were left untouched.

Compilation:
- No Cargo/API generation/GPU/game run: documentation-only maintenance while parallel implementation is active.

Validation:
- Focused Python desktop/system/menu tooling:65 passed. Logs, checked counts and document hashes: target/verification/adr-cleanup-20261103/{python-focused.log,receipt.json}. No full-workspace success is claimed; the previous full-Python source-manifest/dependency failures were not rerun or resolved by this prose change. No commit/push/subagents.

## 2026-11-03 — README capability/API/compatibility presentation

Implementation: removed internal verification and repository-policy prose from the README. Kept the logo; presented engine features, direct Rust/JS/TS/MCP API links, twelve native engine families with explicit profile scope, and a short build/example entry.
Compilation: not run; README prose only, no implementation changes.
Validation: README file/image links resolve, the SVG parses, and its native-profile anchor exists. Internal AGENTS/verify/evidence instructions are absent from the homepage. No new compatibility, runtime or hardware result is claimed.
