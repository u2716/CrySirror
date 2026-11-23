# The Rain Court

The default route follows a 36.5 m stone gallery and broken balcony. The narrow exit reveals a terraced city across a jungle ravine, a ruined aqueduct and three waterfall ribbons falling approximately 35 metres. The original flooded courtyard remains available as a separate route.

## Run on macOS

From the repository root with a release player available, run

```sh
./examples/aaa-raincourt/Play.command
```

Double-clicking `Play.command` also works. First launch fetches the pinned source files and cooks the local geometry. Preparation uses curl and Python 3.11 with NumPy, SciPy and Pillow. Set `CRYSIRROR_PYTHON` or `CRYSIRROR_PLAYER` to select existing installations. Later launches check the recipe fingerprint and file sizes. To verify the downloaded hashes again and regenerate the scene, run `python3.11 examples/aaa-raincourt/prepare.py --force`.

On the canyon route W/S advance or backtrack, Shift runs, A/D move across the walkway, Q/E or right drag look around, T starts or pauses the continuous walk and 1 returns to the gallery. The approach and overlook use the same sun, native-resolution AA and 4K masonry. Each of the 11 broad leaves has its own petiole, blade and tip joints. The batched groundcover remains static.

For the original courtyard use `./examples/aaa-raincourt/Play.command --court`. Its controls are listed below.

| Input | Action |
| --- | --- |
| 1 / 2 / 3 | Stone approach / brook / guardian |
| 4 | Toggle live indirect-light probes around the gate |
| T | Cycle through the three moving cameras |
| W A S D, Shift | Walk, hold Shift to move faster |
| Q / E, or hold right mouse and drag | Turn or aim |

## Scene and assets

The gate, masonry, creek, climbing plants, broadleaf geometry, palm crowns and layout are authored for this project. The four cells hold 640 scanned ferns/shrubs and 20,401 small four-leaf groundcover plants. The groundcover is baked by material and may unload with its cell. Near-camera foliage includes 17 animated ferns, 12 broadleaf plants and seven geometric palms. Forty-two canopy trees choose between a reduced mesh and two baked intersecting views at 19/23 m hysteresis. Another 200 trees form the distant horizon.

Sandstone and masonry each use native 4096 × 4096 diffuse and OpenGL normal maps. Packed occlusion/roughness/metallic maps are 1024² and 2048² respectively. The raster importer builds 13 mip levels for the 4K images. The player uses a demo-local 1200 MiB prepared CPU asset budget. Disk formats are JPEG/PNG/HDR. The current PBR path uploads uncompressed texels.

The renderer uses native-resolution temporal AA, a shadowed sun, rainforest HDR illumination, ambient occlusion and screen-space reflections. Six authored canopy openings carry finite mist volumes. The leaves respond to the fixed sun, bank stones darken and smooth near the waterline, and a moving ripple mesh breaks up the creek reflection. The bounds and completion criteria for this lighting model are in [SATD.md](SATD.md).

Scan and lighting sources include Poly Haven's Mossy Sandstone, Mossy Stone Wall, Forest Ground 04, Palm Bark, Rainforest Trail, Fern 02, Shrub 03, Island Tree 01, Root Cluster 01, Namaqualand Boulder 03, Namaqualand Cliff 02 and Lion Head. The guardian reuses the recolored lion scan. The manifest records every download with its source URL, byte count and published MD5. Downloaded and cooked assets are kept in the ignored `assets/` directory. [Source manifest](sources.json) · [Poly Haven CC0 license](https://polyhaven.com/license)

## Reproduce validation

Run these commands from the repository root. The image check exercises real key presses, verifies camera and cell state, confirms the 4K source dimensions, and checks three distinct 1920 × 1080 images. The benchmark uses the same scripted scene, PBR renderer and deterministic button track on an offscreen BGRA sRGB texture. It advances by 16 ms per simulation step, prepares the current scene and waits for the GPU to complete each frame. Its wall timings include CPU, GPU and waits. GPU timestamps cover the samples retained by the engine profiler.

```sh
demo=examples/aaa-raincourt
python3.11 "$demo/prepare.py"
python3.11 "$demo/verify.py"
cargo build --locked --release --manifest-path "$demo/bench/Cargo.toml" --target-dir target
target/release/raincourt-frame-bench "$demo/project.json" "$demo/game.ts" \
  "$PWD/target/raincourt/bench-state" "$demo/bench/tour.json" 180 \
  > target/raincourt/timings.json
```

The input track walks from the approach, switches to the brook at frame 60, sidesteps, switches to the guardian at frame 120 and turns. The interactive host and offscreen benchmark can overlap CPU and GPU work differently. The benchmark does not measure display presentation, VSync or input latency.

## Current movement and lighting results

The current scene prepares nearby cells before they enter view. The default illumination uses baked masonry sky visibility and geometry-clipped mist shafts. Press 4 to enable local raster irradiance probes. On the M4 Max the original walk boundary improved from 101.70 to 19.44 ms in the fenced 1080p run. The final walk P95 was 14.11 ms and its worst frame after startup was 24.86 ms. Window presentation remains unmeasured. [Matched routes, A/B image measurements and reproduction](../../docs/validation/raincourt-streaming-lighting.md)

## Initial 2026-11-23 baseline

Apple M4 Max, Metal raster, 1920 × 1080, release build, 180 sequential offscreen frames, with all 20,401 groundcover plants in the scene data. The input track visits all three viewpoints. The approach had three resident understory cells. The brook and guardian had four.

| Metric | Measured |
| --- | ---: |
| CPU + GPU completion latency, P50 / P95 | 11.90 / 13.75 ms |
| Frame 0, including initial preparation and uploads | 2004.06 ms |
| Longest later frame, index 50 while walking | 98.90 ms |
| GPU-only P95, 58 retained timestamp samples | 7.91 ms |
| Setup before frame 0 | 97.68 ms |
| Final prepared CPU assets | 842.69 MiB |

All three player renders passed the camera, image and residency assertions. A prior run before the extra groundcover had a 12.60 ms P95 and 96.40 ms worst traversal frame. Both tests were run once on this machine. The desktop session was locked, so display presentation latency remains unmeasured. The local output directory holds `hero.png`, `shot2.png`, `shot3.png`, the three player state reports and `timings.json`. The 200 m route, GPU compressed mip streaming and dynamic light transport remain tracked in [SATD.md](SATD.md).

## Ravine route validation

From the repository root run `python3.11 examples/aaa-raincourt/bench/check_canyon.py target/raincourt/canyon`. The release route test holds T through the passage, exit and overlook and checks the camera travel and image differences. On Apple M4 Max / Metal at 1920 × 1080 the 620-frame fenced walk measured P50 17.83 ms, P95 18.86 ms, first frame 2538.46 ms and worst subsequent frame 55.64 ms. The original walkthrough still passed its three camera checks and 180-frame replay (P95 13.95 ms). Raw screenshots and timings are in the indicated local output folder.
