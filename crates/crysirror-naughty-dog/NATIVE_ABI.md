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

## Load regions

`sub_BF890` (`game/level-mgr.cpp`, "[Load Regions] region '%s', task '%s'") reads `REGION_TREE` regions: name at +0, entry count (`i16`) at +0x1c, tag count at +0x1e, tags at +0x20, entries `{key, type, value}` at +0x24. Tag 0xf131b5e1 holds a script lambda stored in the package; its result selects scheme N (default 1). Keys `level`/`level-N` load and display, `level-no-display[-N]` fill the second list, 0x4f807b48 applies to every scheme. Each name was confirmed by its `StringId`.

## Sky

`sky-…` level packages hold one mesh (a closed dome about 120 m across on `sky-train-wreck`) whose first material binds the sky image to sampler 0 (`sky-train-wreck-color`, 4096 × 2048 DXT1) and two cloud layers with masks to samplers 1–4. The material table's shader array (+0x1c, 16-byte `{0, 0, vertex program, fragment program}`) gives shader 0 for the main pass. Its fragment program normalises the view vector (interpolant TEX2) and computes `u = atan2(z, x) / 2π` (an atan polynomial −0.0134805…0.999996 with the |z| > |x|, x < 0 and z < 0 fix-ups) and `v = acos(y) · 2/π` (the −0.0187293, 0.074261, −0.212114, 1.5707288 polynomial): a hemispherical latitude–longitude image with the zenith at the top and the horizon on the bottom row. The cloud layers use scrolled coordinates from interpolant TEX1 and mask textures; runtime constants (time, layer scales) are patched in and not read here. The player renders the sky image as the environment (`crysirror_asset::sky::ImageSky`) instead of drawing the dome, which otherwise blocks the sun's shadow map and hides geometry beyond 50 m.

## Baked lighting

`LIGHT_TABLE_1` holds a light record at +0x28, the probe count at +0x18 and the probe array at +0x30. A probe is 240 bytes: a position in metres and two sets of nine RGB spherical-harmonic coefficients (coefficient-major, RGB interleaved), then padding. The `ice-cave-temple` probes (635) lie on a 2 m grid. The player sums both sets, evaluates them as radiance with cosine convolution per vertex and draws the result through `baked_vertex_irradiance`. Which lighting each set holds and the brightness scale are not established. The `default-lightmap0/1` textures in level packages are 32 × 32 defaults, not baked maps.

Texture descriptors carry the source file name (`…/name-color.tga/….ndb`); materials are matched to their colour map by that name.

## Trigger regions (in progress)

`player-in-region?` (0x674a04) and `object-in-region?` (0x140b58) ask `sub_F7D94` whether an object is in a region; that calls `sub_115EE0(region manager, region, position)`, which tests each region of that name with `sub_114D60`. A region record holds its name (+0), `StringId` (+4), a parent spawner (+0x10, so the volume moves with that spawner, for example `train-wreck-car-b-1`), a shape list (+0x18), entry and tag counts (+0x1c/+0x1e), tags (+0x20, including script lambdas that send events) and entries (+0x24). `sub_114D60` walks the runtime region's shape list (`next` at +0, a `u16` type at +0xc with types 0, 1 and 3, a `u16` count at +0xe, and for type 1 112-byte face records from +0x50) and runs AltiVec plane tests. Those runtime objects are built at login from the package record, whose +0x18 block does not have this layout, so the login conversion still has to be decoded.

## Mesh unit and placement

Decoded positions (`min + t · (max − min)` in the descriptor's float range) are in units of 2^(N − 22) m for a mesh whose position grid has N bits (the deepest kind-4 depth or the kind-6 grid), and texture coordinates in 2^(M − 22) textures for an M-bit UV grid, centred on the texture (+0.5). Level and art-group meshes follow the same rule; centimetres and a fixed 1/256 were earlier data fits that only held for some grids. Checks: the `train-wreck-wide` meshes' half-extents equal their culling-sphere radii (bridge, 15-bit grid, 61.1 against 62.5 m; river 772 against 770; mountains 7,686 against 7,734); the tightly bounded `train-wreck-b` props fit inside their per-fragment world spheres up to 0.977–0.995 × millimetres·2^(N − 12); the 1 × 1 × 1 m crate decodes 1.025 m wide; art-group `vis_sphere` tags agree within 10 %; the 10-bit UV grids of the hero model then span one texture instead of ±8.

Decoded positions are centred on their quantization range, not on the prototype's pivot. A prototype record `{header, instance list, count, count}` lists instances with a culling sphere at +0x30, a row-major world matrix in metres at +0x40, and at +0x14 a pointer to one world-space bounding sphere per fragment. The player recovers the missing local origin by fitting a translation that puts the vertices of the first instance inside their fragment spheres (alternating projections; exact to millimetres on tight props, a few metres on large terrain chunks whose spheres are loose), then places every instance with its matrix. This replaced placing identity-matrix chunks at their culling-sphere centre, which left the `train-wreck-wide` valley floor at +403 m above the river (now −127 against −128 m). Art groups have no instances; their meshes are moved so the bounds centre lies on the `vis_sphere` tag's centre ((0, 0.5, 0) for the crate, (0, 3.57, 0) for `wrk-box-car-02`). The native origin field itself (header, record or SPU job) has not been located.

## Foreground (art-group) meshes

Art-group meshes use the same fragment descriptors, unit and centring as level geometry (above). Their `vis_sphere` tags (`TAG_VEC4`, centre and radius in the model frame) agree with the decoded extents: player and companion models, the crate, the oil drum and the chained lift within 10 %, and `ice-t-gear-8m` (18-bit grid) and `ice-t-gear-8m-bevel` (12-bit grid) both at about 6.2 m. Spheres of animated objects cover their motion and are larger. Fragment byte +0x20 counts the descriptor's attribute rows and byte +0x2f all vertex streams. Skinned fragments add joint data after the described attributes (the hero's: 6 described of 11): after the stream pointers, fragment +0x80 holds counts (108, 336, 40, 688, 3, 12 on the first hero fragment) and four pointers; one stream repeats `u16` pairs such as (0x94, 0x60). This block is not decoded yet. Skinned meshes are drawn in bind space without their joint transforms. The player model's package holds no `JOINT_HIERARCHY`; its skeleton comes from a shared animation package.

## Spawners

Spawner (+0x40) points to a parent spawner; locators of children are in the parent's frame. Flag 0x40000000 is set on continue points, companion placements and cameras and is treated as no-auto-birth; the flag test in the ELF has not been located.
