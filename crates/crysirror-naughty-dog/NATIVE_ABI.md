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

`tasks.bin` `*task-graph*` is `{node map, node edges}` with `%start`/`%end` edges. A node is `{name, world, ss-module, subnode map, subnode edges}`; a subnode holds name, title, level, a combined name, a record at +0x1c whose word 0 is 3 when complete (`task-subnode-complete?` in `task-defines.bin`), and a `continue` record naming the player and companion continue spawners. Movie subnodes are named `movie-NAME` and the disc holds `movie1/NAME.m2v`.

## Load regions

`sub_BF890` (`game/level-mgr.cpp`, "[Load Regions] region '%s', task '%s'") reads `REGION_TREE` regions: name at +0, entry count (`i16`) at +0x1c, tag count at +0x1e, tags at +0x20, entries `{key, type, value}` at +0x24. Tag 0xf131b5e1 holds a script lambda stored in the package; its result selects scheme N (default 1). Keys `level`/`level-N` load and display, `level-no-display[-N]` fill the second list, 0x4f807b48 applies to every scheme. Each name was confirmed by its `StringId`.

## Spawners

Spawner (+0x40) points to a parent spawner; locators of children are in the parent's frame. Flag 0x40000000 is set on continue points, companion placements and cameras and is treated as no-auto-birth; the flag test in the ELF has not been located.
