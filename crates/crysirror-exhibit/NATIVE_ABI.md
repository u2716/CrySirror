# ExHibit native dispatch

## Evidence and counting

The unit in this inventory is a distinct 16-bit RLD opcode. A record can also carry numeric operands, NUL-terminated strings, a format nibble and an operation-specific subtype. Counting a handler once does not establish its operand coverage. The retail modules below were read as data in IDA, with their exported symbols and the original x86 dispatch. The interpreter rejects unmatched operations and reports the scenario and record position.

| Local profile | Original dispatch RVA in resident.dll | Static script evidence |
| --- | --- | --- |
| RLD v1, Famisupi | `liteExec` 0x70c10, `cmdExHIBIT` 0x68b20 | 357 files, 57,255 records, 91 unique top-level IDs |
| RLD v3, Midori no Umi | `liteExec` 0x145e10, `createMethodTable` 0x13acf0, `cmdAction` 0x104930 | 379 files, 51,797 records, 97 unique top-level IDs |
| RLD v3, Stray Sheep volume 1 | `liteExec` 0x1d0ba0, `createMethodTable` 0x19ecf0 | 65 file headers. Ordinary payload initialization remains open. |
| RLD v3, Stray Sheep volume 2 | Header version 3 | 76 file headers. Separate dispatch parity and ordinary payload initialization remain open. |

The two v3 resident implementations use a direct switch and a method table. Their upper opcode indices are 0x10a and 0x11e respectively. The two table constructors contain at least 209 and 234 distinct slots assigned named handler addresses. These bounds and assignments include neither a census of nested calls nor assurance that handlers have identical semantics. Midori `0x18/0x8c` also resets R6 and a branch field. The volume 1 handler additionally calls `rdhExitScene`.

## RLD v1

The original top-level switch has 118 case labels. `0x2f` is an additional label record indexed by `blockId2Index` (RVA 0x62090) and falls through the original execution switch. These are the original case IDs in hex.

```text
05 06 07 08 0a 0b 0c 0d 11 12 13 14 15 16 17 18 1c 1d 1e 20
21 22 23 24 25 26 27 28 29 2a 2b 2c 2d 2e 30 31 32 33 34 35
36 37 38 39 3a 41 42 44 45 46 47 48 49 4a 4c 4e 4f 50 51 52
53 54 55 56 57 58 5a 5b 5c 5d 5e 5f 61 62 63 64 65 66 67 69
6a 6e 70 71 72 73 74 75 76 77 78 79 7a 7b 7d 7e 7f 80 81 82
83 88 8b 8c 8d 8e 8f 90 91 93 94 95 96 97 98 99 9a 9b
```

The adapter has handlers for these IDs and specific operand forms.

| IDs (hex) | Behavior in the adapter | Native evidence and boundaries |
| --- | --- | --- |
| 05 | Block numbering, marker entry, conditional skip, and ordinary v1 block checkpoint | Flushes numeric overrides, draw and sound registers, read history, and title. Full original save files remain outstanding. |
| 0a | Conditional text register assignment | CP932 byte strings, with tested replacement and numeric formatting. |
| 06, 08 | Conditional numeric assignment to a local or system register | `liteSetReg` 0x608b0. Six-field CSV, local override lookup, indirect system destination and numeric operations. Persisted system writes and other layouts remain open. |
| 07 | Binary register arithmetic | `liteCalcReg` 0x614f0. Immediate/local/system sources and conditional writeback by destination kind. |
| 0b | Nested calls 26, 27, 28, 29, 33, 37, 38 and 39 (decimal) | `cmdExHIBIT`. CG, scene and sound experience counters and the local flag. Call 26 creates the once-per-session timed logo layer. Calls 27/28 clear the startup skip/auto state. Call 29 resolves the parent and decrements the control enable count. The first transition to −1 redraws the third cell of an attached visible kind-1 image (0x87690, 0x994a0, 0xb1420, 0xb1ab0). Character creation and assignment to a layer remain open. Call 37 reads user/auto/quick slot indices. Persistent experience files are outstanding. |
| 0d | Timed waits | `liteExec`. Host deadlines cover elapsed time. Native input/skip wakeups remain open. |
| 11 | Scenario transfer for the two-string form | The second string is a condition. The transfer commits local numeric overrides, resets R2 and clears T1. Scene capture, persisted register flush, read-history and other transfer forms remain open. |
| 14, 16, 17, 2f | Conditional block jump, call, return and label | `liteCheckCondition` 0x61750/0x61f80, `blockId2Index` 0x62090. First matching explicit or implicit ID, resume after call, five binary comparisons or CP932 comparison strings where accepted. Missing terms, block entry side effects and native return overlays remain open. |
| 18, 8c | End scenario execution | `liteExec`. Session return to the surrounding native engine remains open. |
| 32–3a, 46 | Music, environment lanes, loop/one-shot effects, voice selection, and environment definitions | Shared mixer and resident PCM preparation. Alternate render-synchronous fades, mode 4 environment lanes, and full definition bootstrap are open. |
| 4a | Mode-9 fullscreen CG and canvas-sized source crops | `cmdDraw` 0x6da70. Other draw modes, clip state and exact GPU blending remain open. |
| 51 | Timed pan/zoom of the current CG without blur or orientation flags. Rectangles use four independently interpolated source-pixel edges. | `enterClip` 0x397c0, `TmAsyncMagnify::exec` 0x11f00 and `FCAsyncEffect::calcInt` 0x253e0. The VM continues during the animation. Other flags, blur and composited sources remain open. |
| 52 | Leave a completed magnification at its remembered destination and retire the native handle. | Six/ten-word forms with both flags set and zero target width. A valid object waits one tick. See `liteExec` case 82 and `leaveClip` 0x39cf0. Replacement motion and explicit alternative destinations remain open. |
| 5b | Prepare an image select item in a native group and parent slot. The three-cell horizontal atlas supplies the idle frame on attachment. | `cmdPrepareSelectItem` 0x64d00, image constructor 0xb17f0, binder 0xb1190 and redraw 0xb1ab0. Admitted form is the extended CSV kind 1, unkeyed copy, 256-entry indexed GYU palettes, no extra sound, tooltip or mask. Native indices are expanded with the parent palette, replace its colour plane and preserve its alpha. Input execution is separate (`0x5c`). |
| 5e | Create an image billboard at a literal position on its numbered layer. Effect 255 uses the supplied level, 519 binds the surface at level zero. | `cmdBillboard` 0x65a20, path overload 0x36720, initializer 0x80ba0, placement 0x978e0. The bitmap supplies the dimensions. Cropping metadata for other source types has no effect in this overload. Dynamic/relative layer IDs, attached text, other effects and properties remain open. |
| 64, 65 | Start and leave six-lane literal billboard movement, absolute or relative. | `cmdEnterMove` 0x63fe0, update 0x24260. The x86 mode-2 case falls through into the logarithmic branch. Negative-duration paths and overlapping lanes remain open. |
| 69, 6a | Start and leave eight-lane literal billboard fades. Levels 0–16 update shared layer opacity during scenario waits. | `cmdEnterFade` 0x64830, prepare/delay/exec/exit 0x46530–0x46610. The last positive delay on a prepared lane sets the channel delay. Append on a retained channel and non-billboard targets remain open. |
| 8b | Increment or release input-block counters on a single numbered layer or its complement. | `cmdInputLayerControl` 0x67000. Counters cover 127 slots before surface creation, independently of per-control enable counts. Input hit detection consumes a zero count at 0x98d76. Other range forms and event dispatch remain open. |
| 71, 72 | Path/ID movie start and leave, optional effect, one-second startup interval | `cmdEnterMovie` 0x68960, `enterMovie` 0x3c2f0/0x3c3c0, `leaveMovie` 0x3c450. Default layer and flags only. Playback and presentation require a player run. |

Of 91 sample opcode IDs, 33 reach an adapter dispatch arm for at least one form, and 58 have no handler. The unhandled sample IDs are:

```text
0c 12 13 15 1c 1e 20 21 23 24 25 26 27 28 29 2a 2b 2c 2e
30 31 42 47 48 49 4c 4e 4f 50
53 54 55 56 57 58 5a 5c 5d 5f 61 62
70 73 74 75 76 77 79 88 93 94 95 96 97 98 99 9a 9b
```

Native `0x0b` has 23 primary subcommands (decimal 0–3 and 24–42). Famisupi calls 18 of them in 426 records. Primary handlers 31 and 42 themselves dispatch further. The sample calls are:

```text
2 3 24 25 26 27 28 29 30 32 35 36 37 38 39 40 41 42
```

## RLD v3

The verified 2011 initializer takes the ordinary scenario seed from the installed configuration and the initialization bitmap, and obtains the definition seed from the installed resident module. The transform starts after the 16-byte header, processes at most 0x3ff0 words and leaves the trailing bytes intact. The record origin is the header offset plus four. All 379 decoded Midori files consumed exactly the declared records and ended at EOF in an independent static reader.

Midori's binary arithmetic export `cmdCalcReg` (RVA 0x94060) forwards to `liteCalcReg` (0xb2030), with the same source types and operations as v1. At `0xbf`, the 2011 script action dispatcher splits six numeric fields and a final byte string. Implemented literal-operand actions are 190 (set local flag 7), 262 (set/clear T206), 346 (clear a register range), 348 (replace bits 8–10 of S23), and 360 (update system flag 5 and T208/T209). The respective RVAs are 0xb8d50, 0xb9780, 0xba790, 0xba980 and 0xbab30. The action result cell starts at zero and a nonnegative destination writes it back to the local holder. Other actions, expression forms and opcodes fail explicitly.

| Nested dispatch | Midori sample IDs (decimal) | Raw records |
| --- | --- | ---: |
| `0x0b` | 25, 27, 28, 30, 51, 57, 60 | 20 |
| `0xbf` | 151, 164, 166, 190, 250, 262, 265, 338, 345, 346, 348, 350, 360 | 388 |

The 97 unique top-level IDs used in Midori are:

```text
05 06 08 0b 0c 0d 11 12 13 14 15 16 17 1c 1e 20 21 23 24 25
26 27 2c 2e 2f 30 31 32 33 34 35 36 37 38 39 3a 3b 3c 3d 3e
40 42 46 47 48 49 4a 4c 4e 50 51 52 57 58 5a 5b 5c 5d 5e 5f
61 64 65 66 69 6a 6d 70 71 72 73 74 75 84 88 93 94 a0 a2 a7
bd bf cb cc cd ce cf ef f2 f3 f8 f9 fa fb fc fd ff
```

Ten of the 97 sampled IDs reach at least one dispatch form: 05, 06, 08, 0d, 14, 16, 17, 2f, 46 and bf. `cmdDefEnv` (RVA 0xdd1f0) and its four-lane initializer (0xb2480) store the environment definition. Conditions, calls, assignments and passive block markers have execution paths. Entering a live block still requires the native camera, layer and printer checkpoint. The installed entry reaches that check at start.rld+0x114. The 87 other sampled IDs lack a dispatch arm. The counts exclude nested methods and additional operand variants.
