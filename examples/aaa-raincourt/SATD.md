# Rain Court technical debt

## SATD-raincourt-residency

**Location** — `game.ts`, `prepare.py`, `groundcover.py` and the raster PBR asset path.
The four authored understory cells load by camera distance. Their material batches include individual plants baked into meshes. The two tree representations share a visibility switch and both remain resident. Four 4K base/normal images have full RGBA mip chains. Whole-model CPU preparation and first GPU upload can cause stalls.

**2026-11-23** — Added cell-managed understory, two-distance tree LOD, a batched horizon and a repeatable walking benchmark. Extended the same cells with 20,401 rooted groundcover clumps. Full-res image sources and normal rendering are validated on Metal. Per-mip GPU admission, block-compressed sampling, incremental upload, measured GPU residency and hardware instancing for the ground layer remain open. Close after a 200 m repeatable route exercises loading, eviction and re-entry, proves the active device texture formats, shows bounded upload/retirement under a configured GPU budget, and records P95 plus worst traversal frame.

**2026-11-23, follow-up** — A bounded cell preparation band now overlaps worker decoding with movement. The measured fourth-cell wait fell below 0.01 ms on both routes. Verified camera-driven unload and re-entry. Whole-model GPU upload and cold-start limits remain. [Measurements](../../docs/validation/raincourt-streaming-lighting.md)

## SATD-raincourt-light-transport

**Location** — `sunshaft.wgsl`, `leaf.wgsl`, `wetstone.wgsl`, `court.py` and the environment in `game.ts`.

**2026-11-23** — Added fixed-sun broadleaf backlighting, a world-height wetting band, six local mist shafts, HDR environment lighting, raster sun shadows and screen-space reflections. The mist integrates eight samples inside authored canopy openings. It has no shadow-map sampling along the ray. Its density is local to the proxy. Leaf backlight uses a fixed sun direction and has no shadow visibility input. The creek has a moving rippled surface. Off-screen water reflections, refraction through depth, changing light/occluder response in mist and leaves, and dynamic indirect illumination remain open. Close with moving-light and moving-occluder image tests plus an off-screen reflected-object and submerged-object test.

**2026-11-23, follow-up** — Added static sky visibility in `sky_visibility.py`, first-blocker shaft depth in `shaft_depth.py`, and optional live irradiance probes around the court. Isolated image controls passed for the default lighting and optional probe activation/convergence was verified separately. Moving canopy or character occlusion in the mist, off-screen reflections, submerged refraction and general dynamic light transport remain open.

## SATD-raincourt-scale

**Location** — `game.ts`, `bench/src/main.rs`, `bench/tour.json`.

**2026-11-23** — Added a three-view walkthrough with a 56 m × 48 m camera envelope, material-batched geometry and a fenced, no-readback Metal benchmark. Moved the locally authored benchmark from `crates/crysirror-desktop/examples/scene_frame_bench.rs` to `examples/aaa-raincourt/bench/src/main.rs` before delivery to isolate it from unrelated adapter builds. The desktop present loop, longer traversal, true GPU-driven foliage culling and continuous geometry LOD need their own measurements. The camera follows terrain and steps. Full character/wall collision is outside this walkthrough. Close the performance item with a repeatable 200 m traversal, GPU draw/residency evidence and cold plus steady-state presentation frame times on named hardware.

**2026-11-23, follow-up** — Extended the offscreen harness to report per-frame engine, readiness, encoding, fence and committed cells, and replayed cold and warm walk and T routes. A locked 1080p window produced no presentation timing. The 200 m route and actual display pacing remain open.

**2026-11-23, canyon route** — Added and exercised a 36.5 m continuous gallery, cliffside city and waterfall route, with an animated three-joint chain per hero broadleaf. The 1080p walk exceeded the 16.7 ms P95 goal. Cold upload, live display measurements and the longer 200 m acceptance route remain open. The cataracts use animated ribbon surfaces. Water simulation and dynamic spray are unverified.
