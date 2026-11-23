# Uncharted 2 PS3 installed-resource verification

## 2026-11-23 — BCAS20097 01.00

The local `PS3_GAME/PARAM.SFO` identifies *Uncharted 2: Among Thieves*, title ID BCAS20097, APP_VER and VERSION 01.00. The following measurements concern clear on-disc resources and do not require execution of the PlayStation binary. The input directory was opened read-only. No assets were added to the repository.

| Mounted source under `PS3_GAME/USRDIR/build/main` | Named files |
| --- | ---: |
| `pak22.psarc` | 781 |
| `actor27.psarc` | 1,311 |
| `dc1.psarc` | 177 |
| `text1.psarc` | 51 |
| `sound1/banks.psarc` | 182 |
| `sound1/music.psarc` | 446 |
| `sound1/streams.psarc` | 357 |
| `sound1/speech/english.psarc` | 17,127 |
| **Total** | **20,432** |

The 5,596,444,529-byte level archive uses 40-bit big-endian entry addresses. An independent bounded-block reader provided the comparison digests. The checked first directory record in `pak22.txt` is `con-1-start`. Its exact package match is 9,350,592 bytes; SHA-256 is `27351f592bf987f6978b740c224a0c1ecbddeb2f56f7641e33936ca9009ef253`. Its sound-bank reference maps to `convoy-local.bnk`, 3,998,544 bytes; SHA-256 is `df4158e4c8645b0f7934c633a9dee8ef7064a8591fe83c61ec2959f9820c268a`. The catalog also declares `con-1-start-ingame`. The archive has a numbered `con-1-start-ingame10.pak`, whose native selection rule remains open.

The 50,442,560-byte `train-cliff.pak` yields SHA-256 `3423bea4eabc7699e5adee0169f9807e24cf666210ce8933cb28d795aae2b945`. Native item types include the background section, material and light tables and collision data. Reading their headers establishes the resource directory; traversing them into renderable level geometry is outstanding.

The 51 native localization tables cover 17 numeric language slots and three categories. The pool holds UTF-8 strings; records contain sorted big-endian IDs and offsets relative to the pool. Independent inspection counted 120,564 string records; the same ID `0x00049e15` resolves to the English “Slow Motion” and Japanese “スローモーション”.

The release `level_resources` example ran to completion on macOS for `con-1-start`, `train-cliff` and `war-1-market-coop`. Every run mounted all eight archives and parsed all localization tables. The controller DDS decoded through the shared image pipeline to 1280 × 720 with 9,076 distinct RGBA values. The full convoy and train package digests matched the independent reads above.

| Selected level | Verified inner PAK directory |
| --- | --- |
| `con-1-start` | 10 pages and 48 items, including 35 VRAM descriptors and the native camera table. The declared sound bank resolved in its separate mount. |
| `train-cliff` | 38 pages and 107 items, including 95 VRAM descriptors and the background, lights and material table. |
| `war-1-market-coop` | Physical archive offset 4,312,806,630, 21,901,568 logical bytes, six pages and 173 items, including water and two collision records. Both referenced actor packages resolved through the actor mount with 32 and 38 items. |

Six release regression checks passed. The native checks exercise duplicate/malformed level IDs, repeated cache references, localized key lookups and invalid string boundaries, and page-relative item names and broken links. Shared format checks exercise mounted seeks across three block forms, partial final reads, EOF, corrupted zlib, invalid flags, invalid hashes, invalid offsets and directory traversal. The format registry check confirms the new ID is present once.

Run from the repository root:

```sh
cargo test --release --offline --locked -p crysirror-naughty-dog --lib
cargo test --release --offline --locked -p crysirror-formats-arc --test psarc
cargo test --release --offline --locked -p crysirror-formats-arc --test archives explicit_registration_lists_only_implemented_decoders
cargo run --release --offline --locked -p crysirror-naughty-dog --example level_resources -- "$HOME/Downloads/game-Uncharted2" train-cliff
```

Runtime entry, game-script execution, geometry, texture bindings, real-time audio, playable controls and native saves have no gameplay verification. Package selection for numbered names is unresolved. [Current implementation and scoped gaps](../../crates/crysirror-naughty-dog/README.md)

The verified local release executable is available at `target/uncharted2-validation/level_resources`. The same directory holds `con-1-start.log`, `train-cliff.log` and `war-1-market-coop.log` from the three installed-data runs.

## 2026-11-24 — textures and XVAG

The release `level_resources` example decoded every `VRAM_DESC` level 0 in `menu-temple.pak` (48), `train-cliff.pak` (95, including DXT3), `con-1-start.pak` (35) and `war-1-market-coop.pak` (159), plus the menu level's actor packages. A separate PS-ADPCM reference decode of `146_JungleExplore_LOOP1.xvag` (2 channels, 48 kHz) produced no invalid filters, no clipping and an inter-channel correlation of 0.64. Census of 200 headers per archive: music and streams are 48 kHz with 1–6 channels; speech is 32 kHz mono or stereo, all codec 6. Unit tests cover descriptor bounds, Morton order, DXT3 alpha and XVAG interleave/history.

## 2026-11-24 — fragment indices

All 37 fragments of the `menu-temple` prototype mesh decoded with the header counts exactly consumed: 41,040 half-edges, 31,830 with an opposite twin, no duplicate directed edges and no degenerate triangles. The two shared-edge code meanings were selected by maximizing opposite-twin edges over all 36 candidates; the chosen pair scored 31,830 against 11,086 for the next. Positions are not yet validated.

Positions: `menu-temple` (1 mesh, 37 fragments, 12,383 vertices, 13,680 triangles) and `train-cliff` decode through `level_resources … LEVEL OUT.glb`, which writes an inspection GLB outside the repository. Kind-4 fragments form coherent shapes in wireframe inspection. `con-1-start` and `war-1-market-coop` fail the kind-6 grid bound, and actor packages fail on a different index variant; both are open. No level has been rendered in the player.

Kind-6 positions are `(q << 2) + offset`: on `con-1-start` mesh 4 and `train-cliff` mesh 3, neighbouring fragments share 147 and 1,438 seam vertices exactly with a two-bit shift and none with shifts 0, 1 or 3. With the shift, the needed grid equals the mesh's largest position depth in 96 of 97 meshes over six levels; the remaining mesh reaches `4 · (2²⁰ − 1)` exactly. The release `background_census` example decoded every one of the 334 `pak22.psarc` packages that contain prototype meshes: 19,864 meshes, 80,154 fragments and 21,612,412 triangles, with two-triangle position-less quads in `post-56` skipped.

Texture coordinates use 1/256 of the texture: uniquely mapped statue fragments of `menu-temple` span at most 256 units, and with that unit the release `texel_density` example measures 0.37–1.51 texels per native unit across the 13 `menu-temple` material groups (123 `train-cliff` groups: median 0.76). Sampler texture IDs resolve to the package's `VRAM_DESC` word 3 for every installed material visited by `background_census` (334 packages, 80,160 fragments). Images were not rendered for this check.
