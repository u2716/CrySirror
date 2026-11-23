# Naughty Dog engine resources

Onboarding mode is **adopt**. This crate reads the installed PS3 disc profile for *Uncharted 2: Among Thieves*, title ID BCAS20097, application/disc version 01.00. The identifiers come from the installation's `PS3_GAME/PARAM.SFO`. The Rust code keeps package data in client-readable streams and uses CrySirror's shared archive and image services. This resource profile is available independently of a game VM.

## Inspect installed resources

From the repository root, supply the directory containing `PS3_GAME` and an optional native level name.

```sh
cargo run --release --offline --locked -p crysirror-naughty-dog --example level_resources -- "$HOME/Downloads/game-Uncharted2" train-cliff
```

The command reads the level declaration, mounts the eight archives in place, decodes one installed controller image into RGBA, and reports the resource types and SHA-256 of exact package matches. It reads the localization tables and prints two language variants of the same UI string. All results go to the terminal. Some package references have numbered siblings on disc. The command reports a missing exact match until native version selection is established.

| Input | Current behavior |
| --- | --- |
| Plain PSARC 1.3, zlib, 65,536-byte blocks, flags zero | The shared `sony/psarc` archive format exposes seekable files under their original case-sensitive paths. It validates the manifest path hashes, physical and logical bounds, and zlib end/checksum. Uncompressed and compressed blocks may alternate. File positions use wide offsets. |
| `pak22.txt` | Reads 403 level declarations, including 660 package references. Retains the level ID, package, actor, sound bank and cache references in native order, including repeated cache edges. |
| PAK 0xa79 | Reads the original page and named item directories, including item type and page-relative location. Package payloads remain accessible through the shared archive mount. |
| `text1.psarc` | Validates sorted IDs and UTF-8 string boundaries in the installed common, networking and subtitles catalogs. Lookups support multiple IDs referring to one string. |
| `VRAM_DESC` | Reads the RSX format, size, level count, face count and data offset relative to the package's VRAM base (file length minus header data size). Level 0 of DXT1/DXT3/DXT5 decodes through the shared block decoder; swizzled A8R8G8B8 is de-Mortonized. |
| Background mesh indices | `geometry::decode_indices` expands the compressed fragment triangle stream (explicit references, 2-bit strip codes, emission flags). Every fragment of `menu-temple` decodes with all counts and references consistent and no degenerate triangles. |
| Background mesh positions | `mesh::prototype_meshes` relocates the package and dequantizes fragment positions (bounds mapping; offset fragments on the mesh grid with two placement bits). `examples/background_census` decodes every installed package. |
| Mesh materials and UVs | Mesh groups bind fragment ranges to materials; pass-0 samplers name textures by `VRAM_DESC` ID. Texture coordinates are 1/256 of the texture. `level_resources LEVEL OUT.glb` writes a textured inspection GLB outside the repository. |
| XVAG codec 6 | `crysirror_audio::xvag` decodes PS-ADPCM streams (music, ambience, speech) with frame-interleaved channels to WAVE. |
| Loose boot DDS | Uses the existing CrySirror image decoder. The local sample includes original 1280 × 720 DXT1 controller and gallery assets. |

## Evidence and outstanding work

The 0xa79 page/item layout was independently checked on the local installation against the [package layout research by icemesh at `3e3025c`](https://github.com/icemesh/big2-ps3-pak-research/blob/3e3025cc9afc2cbf8e6c0e5a36466d6a72004244/big2-ps3-pak-research/src/package/package-def.h). The PSARC layout was checked against its eight on-disc manifests and [libPSARC format notes](https://github.com/GaticusHax/libPSARC/wiki/Documentation-libPSARC-PSARC-File-Format). The implementation treats all offsets as checked integer data.

Native PPU behaviour is analyzed from an owner-supplied decrypted `EBOOT.elf` beside the installed `EBOOT.BIN`. Neither file is distributed. `crysirror play GAME_DIR` detects the installation and opens a level viewer (`naughty-dog` adapter, `src/adapter.rs`). It starts at `menu-temple` and uses a level's `CAMERA_TABLE_1` start camera when one is installed. It loads the packages of the level's `cache` levels and places prototype instances by their world matrices. Page Down/Page Up step through the `pak22.txt` levels, WASD/QE move the camera, arrows turn it and Shift moves faster. The viewer presents the level packages' background and prototype meshes with base-colour textures. The level's original camera, lighting, actors and scripts are not used. Native boot, animation, collision, scripting and save semantics remain open. The disc is an input format. Desktop, browser and console gameplay are unverified. See [validated samples](../../docs/validation/uncharted2.md) and [SATD-nd-retail-runtime](SATD.md).
