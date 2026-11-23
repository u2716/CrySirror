# The Rain Court movement and lighting on Apple M4 Max

Measured 2026-11-23 with the release script and raster PBR renderer on Metal at 1920 × 1080. The winit player created a 1920 × 1080 BGRA sRGB surface on the locked Mac. A 14-second bounded window probe with per-process Metal HUD logging produced no presentation profile. All frame times below come from the release offscreen benchmark. Each frame advances the same 16 ms simulation, waits for the assets required by that view, submits rendering, and fences GPU completion. Cold means the first lap of a fresh process. OS file caches were not flushed. Warm is the second lap in that process. Each configuration ran once per route.

## Movement

The short walk uses the original 180-frame input track, resets to the approach and repeats it. The T route presses the real tour action and runs two full circuits for 2,628 frames. All timing comparisons use the same instrumented benchmark binary. Shader, lighting or game data changes are identified in the configuration column.

| Route and configuration | Frame 0 (ms) | P95, both laps (ms) | Worst after frame 0 (ms) | Warm P95 / worst (ms) |
| --- | ---: | ---: | ---: | ---: |
| Walk, baseline | 1994.56 | 13.39 | 101.70 | 13.37 / 14.12 |
| Walk, prefetch only | 2028.22 | 13.31 | 23.21 | 13.26 / 14.31 |
| Walk, final default lighting | 2058.40 | 14.11 | 24.86 | 14.36 / 14.86 |
| Walk, optional live probes | 2085.14 | 19.50 | 24.35 | 19.61 / 21.59 |
| T, baseline | 1996.01 | 13.92 | 106.87 | 14.05 / 16.14 |
| T, prefetch only | 2007.52 | 14.00 | 21.63 | 14.25 / 16.40 |
| T, final default lighting | 2087.48 | 14.25 | 21.69 | 14.64 / 16.51 |
| T, optional live probes | 2081.83 | 18.23 | 24.89 | 18.24 / 21.46 |

The fourth cell became visible on walk frame 50 and T frame 439. Baseline readiness fences took 70.89 and 72.67 ms. A separate 54-frame trace covering the walk spike measured 72.52 ms in `asset.Level.read_prepare` and 6.04 ms in `pbr.model_upload`. A prepared level carries both its hierarchy and its CPU PBR payload. The existing streamer reuses an earlier client-scoped request for that same level.

The demo starts preparation within 40 m, activates at 34 m and unloads at 41 m. The explicit prefetch owner releases beyond 44 m after two observations so the world has time to drop its lease. It skips prefetch on a zero-timestep startup frame. The sequence of resident cells matched the baseline for every frame of both routes. Readiness at each measured boundary fell below 0.01 ms. Walk frame 50 took 19.44 ms with final lighting. The final walk's worst later frame was frame 1. The separate leave-and-return run observed 3 → 4 → 3 → 4 committed cells. Whole-model upload still happens on entry. First-frame startup is still about two seconds.

The level retains its original four cells, 20,401 small groundcover plants, 640 ferns/shrubs and two pairs of 4096² colour/normal sources. The prepared CPU asset budget remained 1200 MiB. End-of-walk prepared payload grew from 842.7 to 901.5 MiB after adding native-resolution occlusion variants. This counts the prepared CPU cache only.

## Lighting controls

The default bake projects masonry and the strangler trunk toward 20 sky directions. Face orientation weights the visible sky. Four accessibility bands modulate the scanned ambient occlusion channel at its original resolution. Colour, normals, roughness and direct sun parameters remain intact. Six finite mist shafts use independent 4 × 6 maps of first-hit depth traced through the masonry, terrain and trunk along the fixed sun. Each integration sample filters the visibility of neighbouring columns. The fixed bake excludes moving foliage and characters.

A player-rendered A/B held the camera, exposure (−0.31), shaders, water pose and frame count fixed. Restoring only the original material occlusion made 486,160 pixels brighter by over four mean RGB code values at the approach. Removing only the shaft depth clipping brightened 77,371 pixels by the same threshold. The three normal camera shots passed their state and image checks. The optional 4 key enables the renderer's 3 × 2 × 3 irradiance volume. A stationary capture reported 18 probes, 54 updates and three converged sweeps. Pressing 4 again returned the active count to zero. The creek now plays a rigid skinned motion clip so its drift no longer registers as a moving static mesh in the GI capture signature.

## Reproduction

From the repository root, with the release player present, run

```sh
demo=examples/aaa-raincourt
out=target/raincourt/recheck
python3.11 "$demo/prepare.py"
cargo build --locked --release --manifest-path "$demo/bench/Cargo.toml" --target-dir target
python3.11 "$demo/bench/measure.py" "$out/default"
python3.11 "$demo/bench/measure.py" "$out/probes" --live-probes
python3.11 "$demo/verify.py" --output "$out/cameras"
python3.11 "$demo/bench/check_lighting.py" "$out/lighting-controls"
```

The original local evidence is in `target/raincourt/lighting-pass/`. Look under `before/` and `baked/` for matching camera images and player state, under `controls/` for isolated image comparisons, and at `baseline-*.json`, `performance-*.json`, `baked-*.json` and `optional/*.json` for per-frame traces. `window.log` records surface creation. The focused release build and check passed. Display frame pacing, driver-reported GPU residency, I/O after a cold OS boot and other devices were outside the measurements.
