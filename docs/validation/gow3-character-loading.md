# GoW III character loading and retained regression assets

Date: 2026-11-04. Local PS3 owner-media conversion and offscreen Metal validation.
This is an asset frontend using shared engine glTF/material/scene rendering,
not execution of the original game or a PS3 VM.

## Implemented source changes

- `tools/gow3_scene.py` walks every declared MG draw set, preserving each set's
  native joint binding. The previous importer only walked the first set.
- Shared `crysirror-asset::indices` and the independent Python cross-check accept
  zero-bit EDGE delta streams. These encode the predictor without packed bits;
  version, size, index ranges, reuse counts and eight-lane totals remain checked.
- The observed 0x12 skin-input UV binding is decoded as its declared float2
  stream. The previous UV selector only recognized 0x02.
- `tools/gow3_character.py` decodes the native hierarchy, names and inverse-bind
  matrices. Deform joints obtain bind-world transforms from their inverse bind;
  non-deform attachments compose their native local matrix with their parent's
  bind-world matrix. MG joint bindings place the rigid eyes/armor pieces. No
  external model, fitted transform or guessed/nearest bone is used.
- `tools/ue_preview.py` exports each material primitive's referenced vertices,
  remapping its indices and retaining position/normal/UV/tangent correspondence.
  Previously every material primitive referenced its entire mesh's vertex
  accessor, causing this engine's independently owned primitives to duplicate
  unused vertices in prepared CPU/GPU resources.
- `tools/gow3_fixture.py` retains the checked dependency closure of one scene
  and one character, selected native inputs, private source reports, and SHA-256
  receipts. Base-color previews are capped at 512 pixels and detail maps at 256
  through the shared Rust image codec. Original mips remain in the native inputs.
- Tool outputs may live externally or in the ignored `local/work` subtree.
  They remain prohibited in the source-export directories.

The character is a **static bind-pose preview**. Native animation clips,
compressed skin weights, runtime cloth and original material/shader behavior
are not implemented by this conversion. Exported glTF contains ordinary static
geometry, not a fabricated animated rig. The private skeleton diagnostic is
retained for future work. This result establishes character geometry loading,
texturing and native rigid-part placement, not complete character/game support.

## Actual owner-media results

| Resource | Draw sets / surfaces | Vertices | Triangles |
|---|---:|---:|---:|
| R_HERO00.WAD | 22 / 71 | 24,242 | 32,513 |
| OLY100.WAD | all declared sets / 1,532 | 526,975 | 585,990 |

The character has 254 native hierarchy nodes. All 71 surfaces have verified UV
streams and textures, with no missing texture dependency or neutral-material
fallback. Source texture roles are still a reduced material policy; secondary
color slot 17 is explicit when a character material lacks slot 0. Source masks
use the existing shared engine opacity/masked-material path.

The level now includes the additional MG sets formerly omitted by the first-set
profile. All 247 MG resources decode. Seven level surface groups still have
unresolved UV remapping/cardinality; 81 surfaces lack a verified UV binding and
92 surfaces use the documented neutral preview policy. Native level entity /
animation composition and original lighting remain incomplete. Existing native
light conversions are retained as preview approximations.

The character scale of 3.28 in the combined scene is an explicit inherited
preview convention, not physical-unit calibration. At x=155,z=-312 the selected
level geometry has a floor at y=-26; the combined character is placed there.

## Compilation and tests

Successful local commands:

```sh
cargo build --release -p crysirror-asset --example transcode
cargo test --release -p crysirror-asset --lib indices::tests
cargo build --release -p crysirror-desktop --bin crysirror
python3 -m unittest discover -s tests -p 'test_gow3*.py'
python3 -m unittest discover -s tests -p 'test_local_game_import.py'
```

- Shared Rust index tests: 2 passed, 0 failed, 0 ignored; other asset tests were
  filtered out in this targeted invocation.
- Python GoW/fixture tests: 17 passed; local import/export tests: 12 passed.
  These fixtures are authored and contain no game bytes.
- The desktop release build succeeded with existing dependency/workspace
  warnings. No debug build was used.
- No reflective class/schema API changed, so this work does not claim a changed
  docs/api registry fingerprint.

## Behavior validation

The actual release player loaded/rendered the character alone, then the combined
scene + character through the existing offscreen raster/PBR backend on Apple M5
/ Metal. Four warm-up frames were used. The combined project loads within the
default 256 MiB prepared-asset budget after vertex compaction and texture caps.
The earlier uncompact export failed that budget; it was not counted as a pass.

```sh
target/release/crysirror render \
  local/work/gow3/scene-character-regression/project.json \
  --frames 4 --output target/verification/gow3-scene-character.png \
  --data-dir target/verification/gow3-regression-state
python3 tools/gow3_fixture.py verify local/work/gow3/scene-character-regression
```

The generated PNG was inspected: Kratos is visible, textured and placed on the
hall floor with his native rigid parts. This is a static bind pose. No game
window or live screen was read, no sound was emitted, and no original game,
emulator or PSV shader compiler was launched. No FPS or native-animation result
is claimed from this capture.

Private local receipts:

- `target/verification/gow3-character-render.json` / `gow3-character.png`.
- `target/verification/gow3-scene-character-render.json` / `gow3-scene-character.png`.
- `target/verification/gow3-character-python-tests.log` and
  `gow3-import-python-tests.log`.
- `target/verification/gow3-native-retention.json`, `gow3-cleanup.json`.

## Retained data and cleanup

The only retained GoW work-material directory is
`local/work/gow3/scene-character-regression`. Its 240 recorded files total
373,102,625 bytes (about 356 MiB), excluding the small receipt itself. It contains
one runtime project, the two models' referenced assets, and:

| Selected original input | Bytes |
|---|---:|
| OLY100.WAD | 48,310,336 |
| OLY100.TS | 211,599,360 |
| R_HERO00.WAD | 44,147,008 |

Before deleting the complete disc directory, all three retained inputs were
stream-compared by SHA-256 against their original entries in OLY.PSARC and
RES.PSARC. All matched. The complete retained dependency receipt passed again
after cleanup. The owner explicitly authorized deletion of the full original
disc directory after seeing the retained-pack scope.

`local/sources/gow3-ps3` and the old GoW conversion/intermediate material were
removed. Those removed paths accounted for approximately 35.49 GiB allocated;
this is not a claim that unrelated concurrent workspace cleanup was ours.
The retained content is private owner game data and must never be committed,
packaged into a distributable or published.

The retained inputs can rebuild the pair without the full disc:

```sh
python3 tools/gow3_scene.py \
  local/work/gow3/scene-character-regression/native/R_HERO00.WAD \
  local/work/gow3-rebuild/character --character \
  --codec target/release/examples/transcode --base-size 512 --detail-size 256
python3 tools/gow3_scene.py \
  local/work/gow3/scene-character-regression/native/OLY100.WAD \
  local/work/gow3-rebuild/scene \
  --ts local/work/gow3/scene-character-regression/native/OLY100.TS \
  --codec target/release/examples/transcode --base-size 512 --detail-size 256
python3 tools/gow3_lights.py \
  local/work/gow3/scene-character-regression/native/OLY100.WAD \
  local/work/gow3-rebuild/scene/preview/project.json
```

Then `gow3_fixture.py build` accepts these two preview projects, the three
retained native inputs and the shared `--codec`, writing a new destination.
The generator never deletes its source inputs.

## Format-reference provenance

The existing native scene profile retains the format-source provenance in
[ADR 0017](../adr/0017-gow3-scene-capabilities.md). An additional layout lead was
the public [Gamer JP Blender add-on release v2.6.6](https://github.com/jpthebaby/GOW3-PS4-Remastered-Modding-Tool-Blender-Addon/releases/tag/v2.6.6),
repository/tag commit `0054cbb0e4a7dd55cfe6c0d983c3e70e61705c3d`.
The downloaded release ZIP SHA-256 was
`8e63ce1031fe0bea33cf2211130584dfd018e162c519d36d6423fa5f3261e119`.
Its MG/subobject, rigid-joint and preview format routines were read as leads;
PS4 vertex/weight layouts were not substituted for the observed PS3 data.

No add-on implementation, fitted transform tables, guessed/nearest-bone policy,
Blender dependency or bundled external skeleton was adopted or executed. The
retained implementation validates native PS3 fields against the local WAD and
feeds existing engine resources. The external reference stays ignored/private;
its release/licence notices are not removed or relicensed.
