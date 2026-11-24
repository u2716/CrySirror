# Naughty Dog script ABI (BCAS20097 01.00)

Findings from the owner-supplied decrypted `EBOOT.elf` (not distributed) and the installed `dc1.psarc`. Addresses are PPU virtual addresses of that ELF. String references go through per-module constant pools that a TOC slot points to (for example the slot at 0xe0ff78 for the script VM module).

## DC modules (`dc1.psarc/*.bin`)

| Offset | Meaning |
| --- | --- |
| 0x00 | `"DC00"` |
| 0x04 | version 1 |
| 0x08 | payload size; the fix-up table follows it |
| 0x10 | pointer to the root value |
| size | fix-up count, then payload offsets of pointer words |

Every value is preceded by its type `StringId` (CRC-32, polynomial 0x04C11DB7, MSB first, initial 0). The root is a `script-file` `{1, count, [(name, value)]}`. A pointer may equal the payload size (an empty trailing array). `startup.bin`'s `top-level` lambda calls `add-import-module` (0xc66a1ce6) and `add-load-module` with `'script`; 86 modules load in that order. The reference SID database at icemesh `3e3025c` maps 0xc66a1ce6 to an unrelated multiplayer name.

## Script lambdas

`script-lambda` is `{code, statics, 'function}`. The interpreter is `sub_1594E0` ("Eval: Unknown opcode"); `sub_158288` ("RunScript") is a second entry and `sub_154F00` the tracing disassembler whose format strings name the opcodes. An instruction is a big-endian word `[op][a][b][c]`; registers are 32-bit.

| Op | Name | Semantics (`r` = registers, `s` = statics) |
| --- | --- | --- |
| 0x00 | Return | return `r[b]` to the caller's destination register |
| 0x01–0x04 | IAdd ISub IMul IDiv | `r[a] = r[b] op r[c]` (signed) |
| 0x05–0x08 | FAdd FSub FMul FDiv | float |
| 0x09–0x0b | LoadStaticInt/Float/Pointer | `r[a] = s[r[b]]` |
| 0x0c | LoadU16Imm | `r[a] = c << 8 \| b` |
| 0x0d–0x0f | LoadU32/Float/Pointer | `r[a] = *r[b]` |
| 0x10–0x12 | Store… | `*r[b] = r[c]; r[a] = r[c]` |
| 0x13–0x15 | Lookup… | `r[a]` = value of symbol `s[b]`; unknown: error and a null-symbol value |
| 0x16–0x18 | Move… | `r[a] = r[b]` |
| 0x19 / 0x1a | CastInteger / CastFloat | convert `r[a]` in place |
| 0x1b | CallFf | DC lambda `r[b]`, `c` arguments from r24…, result to `r[a]` |
| 0x1c | Call | native `r[b]` as `fn(result*, c, args[16] = r24…r39)` |
| 0x1d | IEqual | `r[a] = 1` or 0 |
| 0x1e–0x21 | IGT IGTE ILT ILTE | −1 or 0 |
| 0x22 | FEqual | 1.0f or 0.0f (`fsel` on −\|b−c\|) |
| 0x23–0x26 | FGT FGTE FLT FLTE | −1 or 0 |
| 0x27–0x2a | IMod FMod IAbs FAbs | |
| 0x2d / 0x2e / 0x2f | Branch / BranchIf / BranchIfNot | target `c << 8 \| a`, condition `r[b]` |
| 0x30–0x37 | logic/bitwise NOT AND NOT OR XOR NOR, logic AND OR | logic results −1 or 0 |
| 0x38 / 0x39 | INeg / FNeg | |
| 0x3a | LoadParamCnt | caller argument count |
| 0x3b–0x3e | IAddImm ISubImm IMulImm IDivImm | immediate `c`; IDivImm divides the sign-extended value as unsigned 64-bit |
| 0x3f–0x41 | LoadStatic…Imm | `r[a] = s[b]` |

Other opcodes report "Unknown opcode". After a native call the interpreter reads the status word before the argument block: 1 suspends the lambda, keeping the frame chain, and resumes at the next instruction; 2 abandons it. `sub_153618` sets 1 (waits) and `sub_15362C` sets 2 (`go`). A census of the 177 modules finds 52,401 lambdas and 582 distinct natives called through `Call`.

## Natives

`src/native_names.rs` lists 833 natives: the 782 records `{opd, object, "DcName", "script-name"}` of the ELF with their entry points, plus names that only appear as `Call` targets. Implemented semantics in `src/ss.rs` follow these entry points:

| Native | PPU | Behaviour |
| --- | --- | --- |
| `go` | 0x7d4884 | posts event `go` (0xc2c05f3a) with the state to the caller's process; status 2 |
| `wait` / `wait-frames` | 0x7d6b34 / 0x7d6914 | suspend for seconds / frames × 1/30 s when positive |
| `wait-one-frame` / `wait-for` / `signal` | 0x7d5f90 / 0x7d617c / 0x7d6d38 | |
| `set-/get-int32` etc. | 0x7d7b5c / 0x7d83a0 | `(var, value, [object])` on the process variables |
| `tag-symbol` etc. | 0x7d5664 | the caller's spawner property |
| `send-event` | 0x83da8 | `(event, target, args…)` |
| `spawn-script` | 0x7d4260 | start a process |
| `spawn-entity` / `kill-entity` / `object-spawned?` | 0x150674 / 0x140484 / 0x14032c | |
| `get-spawner-locator` | 0x14bf58 | heap locator |
| `task-find`, `task-subnode-active?`, `task-subnode-complete`, `task-subnode-play` | 0x143358, 0x140efc, 0x13f5e4, 0x141e50 | task state below |
| `fade-to-black` / `fade-to-white` | 0x13d3f4 / 0x13d3ac | `(alpha, seconds)` screen fade |
| `set-black-fade` / `set-white-fade` | 0x13d344 / 0x14aee4 | `(seconds, on)` |
| `player-disable-controls` | — | a control mask; `player-controls-all`/`-none` (`script-funcs.bin`) return 0xffff / 0 |

`camera-fade-out` (0x13b210) fades a named camera object, not the screen.

## State scripts

`state-script {name, ?, initial state, ?, options, state count, states}`; state `{name, block count, blocks}`; block `{kind, event, 0, track group}` with kinds 0 begin, 1 end, 2 event, 3 update (strings of `sub_7DEB70`); track group `{count, tracks}`; track `{name, lambda count, [(label, lambda)]}`. Spawners name their script with the string property `state-script` (0x45c51f9f).

## Tasks

`tasks.bin` `*task-graph*` is `{node map, node edges}` with `%start`/`%end` edges. A node is `{name, world, ss-module, subnode map, subnode edges}`; a subnode holds name, title, level, a combined name, a record at +0x1c whose word 0 is 3 when complete (`task-subnode-complete?` in `task-defines.bin`), and a `continue` record naming the player and companion continue spawners. A subnode's +0x38 is a start lambda and +0x40 a task-manager record `{?, type, flag, params, name string, name}` with types `TaskManagerPlayMovie` (movie name), `TaskManagerMessageDialog` (text key), `TaskManagerMainMenu` and `none`. Text catalog IDs in `text1.psarc` are the `StringId` of the key.

## Main menu

`TaskManagerMainMenu`'s update (`sub_98478`) loads level `menu-temple` once through the level manager (`sub_BA9D0`) and `sub_98634` sets a `CameraControlFixed` camera; the subnode's own level `menu` is a flat board at the origin with the localized logos (`general/menu/logo2[-ita|-spa|-jpn|-kor].tga` with `-alpha` masks, stored bottom row first) and chapter/movie shots, drawn as 2D art. `menu-temple`'s `CAMERA_TABLE_1` start camera `|_cam_start_1` sits at the `menu-camera-1` spawner, whose `fov` property is 75. `dc1.psarc/menu.bin` `*menu-pages*` is `{count, pages}`: a page (0x44 bytes) has its name, title text key at +0x10, flags at +0x1c and buttons `{count, array}` at +0x20; a button (0x48 bytes) has its name, type +8, action +0xc, text key +0x14 and a condition +0x1c. `PAGE_ROOT_MAIN` (title `MENU_LEGAL_COPYRIGHT`, button `MENU_PRESS_START`) leads, without a save, to `PAGE_NEWPROFILE` (New Game, Load Game [0x13], Multiplayer, Options, Medals [0x13]) and New Game to `PAGE_NEWGAME` (Ultra-Easy [0x1d], Easy, Normal, Hard, Ultra-Hard [0x13]). Catalog text uses `\` for line breaks. The condition codes and the button-to-page dispatch live in the EBOOT and are not decoded; the player hides conditional buttons and maps page buttons by name.

## Load regions

`sub_BF890` (`game/level-mgr.cpp`, "[Load Regions] region '%s', task '%s'") reads `REGION_TREE` regions: name at +0, entry count (`i16`) at +0x1c, tag count at +0x1e, tags at +0x20, entries `{key, type, value}` at +0x24. Tag 0xf131b5e1 holds a script lambda stored in the package; its result selects scheme N (default 1). Keys `level`/`level-N` load and display, `level-no-display[-N]` fill the second list, 0x4f807b48 applies to every scheme. Each name was confirmed by its `StringId`.

## Sky

`sky-…` level packages hold one mesh (a closed dome about 120 m across on `sky-train-wreck`) whose first material binds the sky image to sampler 0 (`sky-train-wreck-color`, 4096 × 2048 DXT1) and two cloud layers with masks to samplers 1–4. The material table's shader array (+0x1c, 16-byte `{0, 0, vertex program, fragment program}`) gives shader 0 for the main pass. Its fragment program normalises the view vector (interpolant TEX2) and computes `u = atan2(z, x) / 2π` (an atan polynomial −0.0134805…0.999996 with the |z| > |x|, x < 0 and z < 0 fix-ups) and `v = acos(y) · 2/π` (the −0.0187293, 0.074261, −0.212114, 1.5707288 polynomial): a hemispherical latitude–longitude image with the zenith at the top and the horizon on the bottom row. The cloud layers use scrolled coordinates from interpolant TEX1 and mask textures; runtime constants (time, layer scales) are patched in and not read here. The player renders the sky image as the environment (`crysirror_asset::sky::ImageSky`) instead of drawing the dome, which otherwise blocks the sun's shadow map and hides geometry beyond 50 m.

## Baked lighting

`LIGHT_TABLE_1` holds a light record at +0x28, the probe count at +0x18 and the probe array at +0x30. A probe is 240 bytes: a position in metres and two sets of nine RGB spherical-harmonic coefficients (coefficient-major, RGB interleaved), then padding. The `ice-cave-temple` probes (635) lie on a 2 m grid. The player sums both sets, evaluates them as radiance with cosine convolution per vertex and draws the result through `baked_vertex_irradiance`. Which lighting each set holds and the brightness scale are not established. The `default-lightmap0/1` textures in level packages are 32 × 32 defaults, not baked maps.

Texture descriptors carry the source file name (`…/name-color.tga/….ndb`); materials are matched to their colour map by that name.

## Trigger regions (in progress)

`player-in-region?` (0x674a04) and `object-in-region?` (0x140b58) ask `sub_F7D94` whether an object is in a region; that calls `sub_115EE0(region manager, region, position)`, which tests each region of that name with `sub_114D60`. A region record holds its name (+0), `StringId` (+4), a parent spawner (+0x10, so the volume moves with that spawner, for example `train-wreck-car-b-1`), a shape list (+0x18), entry and tag counts (+0x1c/+0x1e), tags (+0x20, including script lambdas that send events) and entries (+0x24). `sub_114D60` walks the runtime region's shape list (`next` at +0, a `u16` type at +0xc with types 0, 1 and 3, a `u16` count at +0xe, and for type 1 112-byte face records from +0x50) and runs AltiVec plane tests. Those runtime objects are built at login from the package record, whose +0x18 block does not have this layout, so the login conversion still has to be decoded.

## Vertex dequantization and skinning

Each descriptor attribute (`id`, kind << 4 | components, parameter offset, bit depths) is its own MSB-first bit stream. Kind 4: N floats `a`, N floats `b`, `value = a · (1 + v · 2⁻²¹) + b`; kind 6 adds N `u32` offsets `I`, `value = a · (1 + (4v + I) · 2⁻²³) + b`; kind 0 is signed normalised `v · 2 / (2ᵈ − 1) − 1` (normals, tangents, attribute 0x15). Positions are metres in the prototype's frame, texture coordinates are in textures. Source: the public Uncharted 3 PS3 actor decoder `bc160201983/Uncharted-The-Nathan-Drake-Collection-PC-port` (`u3mp/paktools/ps3geom.py`, commit `1294635`), checked there against the PS4 assets. On Uncharted 2 it puts the 1 m crate on the floor (y 0.004…0.999), the hero's feet at y = 0 (1.90 m tall, fingertips beside the `npc-normal-base` finger joints), the `train-wreck-b` box car roof on its Havok collision, and the `train-wreck-wide` valley floor at the river (−125.4 against −124.6 m). The earlier data fits (a 2^(N − 22) grid unit, origins fitted to the per-fragment world spheres at instance +0x14, UVs centred by +0.5, art groups moved to their `vis_sphere`) approximated these formulas and are replaced.

Fragment byte +0x20 counts descriptor rows and +0x2f all streams. Fragment +0x0c points to skin information on skinned fragments: `u16` sizes of streams s1–s4, the palette range count, s1–s4 pointers at +0xc and palette ranges `{u16 first joint, u16 count}` at +0x20. Groups of four vertices take one s1 code: 4/6 a joint list shared by the lanes from s2 (`u16` palette offsets × 48, +4 while more follow), 2/10 per-lane rows of four `u16` from s3 (lane order 1, 0, 3, 2), 0 repeats the previous group, 8 keeps its joints; 6/8/10 read four `u8` weights per joint row from s4, 2/4 weight the first joint fully. Every installed skinned fragment consumes its streams exactly.

Skeletons: `JOINT_HIERARCHY` `{8, joint count, JH block, flip data, names, 0, …, skeleton id}`; the JH block (magic `0x4a48010a`) holds the joint count at +0xc, offsets at +0x28 (per-joint `{flags, parent, first child, next sibling}`), +0x2c (default local poses, 48 bytes `scale | quat | translation`) and +0x44 (3 × 4 inverse bind matrices, 48 bytes each); names are `{StringId, string}` pairs. `npc-normal-base` `skel` has 124 joints rooted at the feet (ankles 0.12 m, pelvis 1.06 m, A-pose).

## Animation clips

`ANIM` items hold the clip at +8 (ICE "CD" clip data, magic `0x43440107`; `ANIM_RAWCHANNELS` carry only the `align` and `apReference` channels). Clip header: hierarchy hash +4 (the skeleton's JH hash: the player clips name `0xef94465d`, the 230-joint rig in `actor27.psarc/proto.pak`, 204 animated joints, with 1,548 clips), size +8, frame count `u16` +0xe, frames per second +0x1c, program block +0x30, first frame record +0x38. The program after the block's 16-byte mask is a list of `{u8 argument count, u8 opcode, u16 arguments}` with the EBOOT's animation command numbering (`sub_2D7058` names: 11 `CopyUniformDataToKeyCache`, 15/16 `KeyCacheUniformLerp/Slerp`, 21–24 constant vector/quaternion decompression, 26 `Dec3VecFloat16`, 27 `Dec3VecRange`, 28 `DecompressQuatSmallestThree`, 29/30 `PostDecompressQuatLog[Pca]`); data offsets are relative to the program (block + 0x10: 0x60 in the player clips, 0x50 in the `anim-train-wreck-climb` prop clips), lists are padded to multiples of 8 with a repeated output, and outputs are offsets into a 48-byte-per-joint buffer (scale, quaternion, translation). Decoding rules and their evidence are in `src/anim.rs`. `Dec3VecRange` records are 16 bytes (a 21-bit unsigned step, two signed steps, a three-float minimum, field starts at bits 21, 42, 64, 86 and 107) followed by a 4-byte `[a, b, a + b, c]` width table per item; `DecompressQuatSmallestThree` descriptors use the same width form.

The SPU handlers are in the `CalcInfluences` job (loaded at local store 0x4000; dispatcher at listing 0x946C, 62-entry handler table at 0xA850 indexed by the opcode above; `Dec3VecRange` 0x30C0, `PostDecompressQuatLog` 0x6C80, `PostDecompressQuatLogPca` 0x6E90), and their formulas were checked by executing those handlers on controlled inputs:

- `Dec3VecRange` widths use the low six bits of bytes 0, 1 and 3. The top two bits of each are a predictor `f`: when non-zero, component `f − 1` of the previous item's decoded vector of the same frame is added to that component.
- `PostDecompressQuatLog(count, keys, parameters, key slot)` turns each key vector `v` into `e = (s·v, c)` with `t = v·v`, `c = 1 − t/2 + 0.04152372 t² − 0.0012714077 t³` and `s = 1 − t/6 + 0.0083172908 t² − 0.00018526176 t³` (constants at listing 0xA7F0/0xA800), then `q = q0 ⊗ e`. The parameters are 16 bytes per pair of items with interleaved `s16 / 32768` halfwords `[x0 x1 y0 y1 z0 z1 w0 w1]`. Nothing is relative to the rest pose and the result is not renormalised.
- `PostDecompressQuatLogPca` gives `q = P ⊗ e ⊗ Q`, with 16 bytes per item `[Px Qx Py Qy Pz Qz Pw Qw]` (`s16 / 32768`).

Every opcode of `idle`, `walk`, `move` and `walk-hurt` decodes.

## Player animation state machine

`dc1.psarc/anim-player.bin` `anim-actor` `anim-player`: state count +0x30, inline 0x48-byte states +0x34, initial state +0x38 (`idle`); layout in `src/anim_state.rs`. 988 states; transitions `{request, flags, blend record, target, 0, 0, fade, -1, 0}` with `auto` taken at the end of the state's animation. Blend trees are nested blend nodes (type `0xb249ac72`), layer/select nodes (`0x1c3f471c`) and `anim-node-animation` leaves; the `run` tree's select node lists clips by movement type (`run-tense`, `walk-balance`, `walk-hurt`, `run-hurt-long`, …), the options of the `move` `player-overlay-info` that `set-walk-type`/`set-run-type` choose. Hurt clips live in `actor27.psarc/anim-proto-hurt.pak`. The `align` raw channel gives root motion (`move` 2.8 m/s, `walk` 1.5, `walk-hurt` 0.8, `run-hurt-long` 3.9).

Blend-tree nodes (`src/anim_tree.rs`):

- **Blend** `{left, right, flags, lambda, 1.0}`: the lambda `(node, info, phase, instance)` returns the right child's weight.
- **Select** `{default, alternatives, flags, lambda, 0}`: the lambda `(node, info, instance)` returns the child to evaluate. DC arrays carry no length.

The 6,131 tree lambdas only read two per-actor blocks and call `lerp-scale` (PPU 0x1416cc: `t = clamp((x − a)/(b − a), 0, 1)`, `c(1 − t) + d t`) and `minmax` (0x14ab64: `min(max(x, lo), hi)`).

- **Instance block:** +48 is the movement-type index and +52 the weapon index.
  - Movement types: the 39 records of `#b35a5e92`, 0x6c bytes each, with the `StringId` at +0x40, the name at +0x48 and the index at +0x4c. `run` 0, `walk` 2, `run-tense` 3, `run-stealth` 7, `run-mo` 9, `run-hurt` 11, `walk-hurt` 14, `relaxed-run` 38.
  - Weapons: the `weapon-base` `player-overlay-info`: binoculars, no-weapon, pistol, long-gun, grenade, crossbow, rpg.
- **Info block (locomotion states):** +28/+32 lean right/left, +40 uphill (`run-incline`), +44 downhill spine, +48/+52 stairs up/down. +0x228 is a cycle phase. These meanings are inferred from the clips the blends lead to.

Every state evaluates. With zero inputs: `idle` is `idle` plus the additive `idle-base-big-add`, `walk-hurt` turns it into `idle-hurt` and `run` into `walk-hurt-long`, and a lean of 0.5 blends `move-lean-r`. Clips named `…-add` hold local deltas (near-identity rotations, untouched joints unanimated) and play as additive layers.

The PPU locomotion controller is reproduced from the transition table in `src/player.rs`. Jumps, falls and landings have no incoming transitions and are entered directly.

Animation leaves are `{clip, ?, ?, phase lambda, ?, clip lambda}`. The phase lambda `(node, info, instance, state phase, 0)` returns the sample phase; look sweeps read it from the info block (NPC +140 left–right, +144 up–down; aim +92/+96). The clip lambda `(node, info, instance)` replaces the clip (`face-reaction`).

`anim-npc.bin` has the same `anim-actor` layout: 264 states, initial `s_ambient-gunout-idle`. Its lambdas also call `angle-mod` (PPU 0x2db59c: `fmod(x, 360)` brought into [−180, 180] with the TOC constants 360, 180 and −180).

NPC leaves are logical animations that `anim-sets.bin` `anim-set`s (`{array, count}` of `{logical, clip}`) map to the character's clips. Example: `*flynn-anim-set*` maps 274 of its 276 leaves into `anim-flynn.pak`, and `*flynn-heist-anim-set*` overrides 4 of them. No module or EBOOT string links characters to sets. The `*NAME-anim-set*` names match 11 art-group and character names, which is the rule used here.

Task continuation records (subnode +0x28) are `{string, StringId}` slots: the player's spawner at +0, allies' at +0x08 to +0x48 (`continue-flynn-…` in 24 records, `continue-rescuer-…`, `continue-ally-…`) and the camera's at +0x50.

## Script animation

- **`animate` / `wait-animate`:** `(animate object clip key value …)` plays a clip once on an object: a process (it plays on the process's spawner entities), a spawner name, or `player`. The level scripts use three keys: 2 (blend-in seconds), 4 (blend-out seconds) and 7 (a locator record from `get-ap-ref-locator`/`get-spawner-locator`). Other keys are ignored. `wait-animate` suspends the track until the clip ends.
- **`get-animation-phase`:** `(get-animation-phase object)` returns the phase (0–1).
- **`get-object-locator` / `get-ap-ref-locator`:** `(get-object-locator object)` and `(get-ap-ref-locator ? object)` return that object's locator record `{x, y, z, 1, qx, qy, qz, qw}`.
- **Where clips come from:** the entity rig's packages, then the `pak22.txt` `actor` packages of the task's levels (`anim-train-wreck-climb` for the prologue cars). The clip is decoded for that rig's skeleton and played from a single-clip library.
- **Prologue:** `train-wreck-climb` plays `idle-00-car-a`, `idle-00-car-b`, `idle-00-undercarriage` and `idle-00-ladder` (10.07 s), the 1.2 s `drake-platform-drop-door` and the single-frame `broken` pose. Props hold the last frame of a finished animation.

## Foreground (art-group) meshes

Art-group meshes use the same descriptors as level geometry (above); their `vis_sphere` tags (`TAG_VEC4`) bound the model, and spheres of animated objects cover their motion. Skinned player models carry no `JOINT_HIERARCHY`; they bind to the `proto.pak` rig (230 joints), whose `idle` clip the player renders.

## Spawners

Spawner (+0x40) points to a parent spawner; locators of children are in the parent's frame. Flag 0x40000000 is set on continue points, companion placements and cameras and is treated as no-auto-birth; the flag test in the ELF has not been located.
