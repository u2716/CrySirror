# CatSystem2 developer reference

[All engines](README.md)

Mode: `adopt`.

## ABI and host contract

KCS v2 instructions and FES commands have separate dispatch namespaces.
`NativeCall` carries a numeric gcall ID and VM-owned argument words; class calls
also require the native receiver/class selected by `native_classes`. Addresses
are bounded guest-memory offsets, never host pointers. FES command identity
includes kind, name, arguments and object context.

| Native area | CrySirror service / implementation | Return / completion |
| --- | --- | --- |
| KCS values, memory and arithmetic | `kcs.rs` / `native_interpreter.rs` | Native stack/flags and checked guest memory. |
| Gcalls / class calls | `adapter.rs` / `native_classes` | Commit native return values and guest-memory outputs only after admitted work. |
| FES tasks, aliases, motions and callbacks | `fes_runtime.rs` and `fes_*` | Retain native task/result and wait continuations. |
| HG3, image lists, planes and E-mote | Shared Image/Canvas/Layer/Puppet services | Worker receipts publish scoped resources before pose/root completion. |
| Text / controls / settings | Shared Text, UI, InputRouting, XML and Storage | Native event/receiver policy remains separate from resource creation. |
| Sound | Shared Audio workers, sources and channel state | Native loop/status/fade completion follows the owner clock. |

Class-call tables are scoped by their source module; gcall ID presence does not
imply every class/argument profile is admitted. INT/HG3/CST format recognition
is independent of executing a particular KCS/FES/CST route. Unsupported class,
message, save, input or script profiles fail at the linked native handler.
Original saves and complete game compatibility have no engine-wide acceptance
claim. Source-only implementation is not rendered-title evidence.

Source/provenance: [SOURCE.json](../../crates/crysirror-catsystem2/SOURCE.json),
[KCS ABI](../../crates/crysirror-catsystem2/src/kcs.rs),
[FES runtime](../../crates/crysirror-catsystem2/src/fes_runtime.rs).
Historical source audits remain separately identified in `docs/*source-audit.json`;
this reference replaces the obsolete handoff/frontier narratives.

<!-- BEGIN GENERATED DISPATCH TABLES -->

## Opcode and API implementation tables

`Handled` means an executable source branch exists. It is not a claim that every argument/subselector or a complete game has been audited. Explicit gaps are shown per row.

| Status | Entries |
| --- | ---: |
| Empty handler | 30 |
| Handled | 682 |
| Partial | 19 |
| Unsupported | 1 |

These tables cover the checked-in catalogues and explicit dispatch branches. An engine-wide upstream denominator is not inferred from these rows.

## Class calls / native_classes/cg_register

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x1A3` | Execute handle branch | Handled | — | [cg_register.rs:184](../../crates/crysirror-catsystem2/src/native_classes/cg_register.rs#L184) |
| `0x1A4` | Execute handle branch | Handled | — | [cg_register.rs:190](../../crates/crysirror-catsystem2/src/native_classes/cg_register.rs#L190) |
| `0x1A5` | Execute handle branch | Handled | — | [cg_register.rs:196](../../crates/crysirror-catsystem2/src/native_classes/cg_register.rs#L196) |
| `0x1A6` | Execute handle branch | Handled | — | [cg_register.rs:203](../../crates/crysirror-catsystem2/src/native_classes/cg_register.rs#L203) |
| `0x1A7` | Execute handle branch | Handled | — | [cg_register.rs:215](../../crates/crysirror-catsystem2/src/native_classes/cg_register.rs#L215) |
| `0x1A8` | → under key a1 | Handled | — | [cg_register.rs:226](../../crates/crysirror-catsystem2/src/native_classes/cg_register.rs#L226) |
| `0x1A9` | a record replaces what the register held | Handled | — | [cg_register.rs:240](../../crates/crysirror-catsystem2/src/native_classes/cg_register.rs#L240) |
| `0x1CA` | the list file; a missing one leaves the register's list as it was | Handled | — | [cg_register.rs:255](../../crates/crysirror-catsystem2/src/native_classes/cg_register.rs#L255) |
| `0x1CB` | → : every string of a named group becomes seen | Handled | — | [cg_register.rs:286](../../crates/crysirror-catsystem2/src/native_classes/cg_register.rs#L286) |

## Class calls / native_classes/containers

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0xF0` | container, key, task, first word, word count. copies the clipped range; appends it (an empty range is still a record) | Handled | — | [containers.rs:38](../../crates/crysirror-catsystem2/src/native_classes/containers.rs#L38) |
| `0xFA` | container, key, task, first word. The record's size >> 2 words are written back by , which also marks the bank changed | Handled | — | [containers.rs:60](../../crates/crysirror-catsystem2/src/native_classes/containers.rs#L60) |
| `0x181` | container, key, scene, list (0 first, else second), include data. sizes and writes both lists; one becomes the record | Handled | — | [containers.rs:79](../../crates/crysirror-catsystem2/src/native_classes/containers.rs#L79) |
| `0x182` | Execute handle branch | Handled | — | [containers.rs:97](../../crates/crysirror-catsystem2/src/native_classes/containers.rs#L97) |
| `0x261` | / : with an interpreter (VM+104), copies the flags into the backup allocation and copies them back | Handled | — | [containers.rs:24](../../crates/crysirror-catsystem2/src/native_classes/containers.rs#L24) |
| `0x262` | / : with an interpreter (VM+104), copies the flags into the backup allocation and copies them back | Handled | — | [containers.rs:24](../../crates/crysirror-catsystem2/src/native_classes/containers.rs#L24) |
| `0x3D2` | > + : container a1 nested becomes the last history entry's data | Handled | — | [containers.rs:117](../../crates/crysirror-catsystem2/src/native_classes/containers.rs#L117) |
| `0x3D3` | > + : history entry a1's data as a new container; none answers 0 | Handled | — | [containers.rs:127](../../crates/crysirror-catsystem2/src/native_classes/containers.rs#L127) |
| `0x3D5` | > : data of scene a1 onto scene a0, entry by entry over both lists | Handled | — | [containers.rs:143](../../crates/crysirror-catsystem2/src/native_classes/containers.rs#L143) |
| `0x3E1` | > : scene a0's history becomes scene a1's names and values only | Handled | — | [containers.rs:156](../../crates/crysirror-catsystem2/src/native_classes/containers.rs#L156) |

## Class calls / native_classes/database

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x365` | the handle exists whether or not the file does; only fills it | Handled | — | [database.rs:268](../../crates/crysirror-catsystem2/src/native_classes/database.rs#L268) |
| `0x366` | Execute handle branch | Handled | — | [database.rs:287](../../crates/crysirror-catsystem2/src/native_classes/database.rs#L287) |
| `0x367` | a[2] is a format string naming the entry, a[1] the bit | Handled | — | [database.rs:293](../../crates/crysirror-catsystem2/src/native_classes/database.rs#L293) |
| `0x368` | answers the byte size; with a buffer, also writes the entries | Handled | — | [database.rs:303](../../crates/crysirror-catsystem2/src/native_classes/database.rs#L303) |
| `0x369` | Execute handle branch | Handled | — | [database.rs:317](../../crates/crysirror-catsystem2/src/native_classes/database.rs#L317) |
| `0x36E` | clears a dirty flag no native reads | Handled | — | [database.rs:331](../../crates/crysirror-catsystem2/src/native_classes/database.rs#L331) |

## Class calls / native_classes/effect_tasks

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0xC4` | message 13 asks the class, then its parameter message takes a VM block | Handled | — | [effect_tasks.rs:169](../../crates/crysirror-catsystem2/src/native_classes/effect_tasks.rs#L169) |
| `0xC5` | → 257: running, start = now, speed by effspd (live config 77780) | Handled | — | [effect_tasks.rs:183](../../crates/crysirror-catsystem2/src/native_classes/effect_tasks.rs#L183) |
| `0xC8` | → 259: the running flag | Handled | — | [effect_tasks.rs:218](../../crates/crysirror-catsystem2/src/native_classes/effect_tasks.rs#L218) |
| `0xCB` | → 261: a running task finishes on its final picture | Handled | — | [effect_tasks.rs:224](../../crates/crysirror-catsystem2/src/native_classes/effect_tasks.rs#L224) |

## Class calls / native_classes/emote_messages

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x38B` | → 526 → : inertia on or off | Handled | — | [emote_messages.rs:28](../../crates/crysirror-catsystem2/src/native_classes/emote_messages.rs#L28) |
| `0x38C` | → : every player's macro queue resets | Handled | — | [emote_messages.rs:57](../../crates/crysirror-catsystem2/src/native_classes/emote_messages.rs#L57) |
| `0x38D` | → 528: 0 without an M2 entry, else (busy) | Handled | — | [emote_messages.rs:22](../../crates/crysirror-catsystem2/src/native_classes/emote_messages.rs#L22) |
| `0x38E` | Text and message windows (handle) | Handled | — | [emote_messages.rs:63](../../crates/crysirror-catsystem2/src/native_classes/emote_messages.rs#L63) |
| `0x39C` | → 540: a2 is the voice channel, a1 on/off | Handled | — | [emote_messages.rs:69](../../crates/crysirror-catsystem2/src/native_classes/emote_messages.rs#L69) |
| `0x3A1` | → 543 → / → 544 → | Handled | — | [emote_messages.rs:76](../../crates/crysirror-catsystem2/src/native_classes/emote_messages.rs#L76) |
| `0x3A2` | Timeline | Handled | — | [emote_messages.rs:83](../../crates/crysirror-catsystem2/src/native_classes/emote_messages.rs#L83) |
| `0x3A3` | Reset On Motion | Handled | — | [emote_messages.rs:37](../../crates/crysirror-catsystem2/src/native_classes/emote_messages.rs#L37) |
| `0x3C5` | Waiting | Handled | — | [emote_messages.rs:43](../../crates/crysirror-catsystem2/src/native_classes/emote_messages.rs#L43) |
| `0x401` | 548/549 → : variables are resolved by name, so registration remains valid across a render-resource rebind | Handled | — | [emote_messages.rs:52](../../crates/crysirror-catsystem2/src/native_classes/emote_messages.rs#L52) |
| `0x403` | → 551 → : start a macro; the native always answers its out word 0 | Handled | — | [emote_messages.rs:89](../../crates/crysirror-catsystem2/src/native_classes/emote_messages.rs#L89) |
| `0x404` | → 553 → | Handled | — | [emote_messages.rs:97](../../crates/crysirror-catsystem2/src/native_classes/emote_messages.rs#L97) |
| `0x405` | → 554 → : rows of `#touch` colour a1, table a2 | Handled | — | [emote_messages.rs:103](../../crates/crysirror-catsystem2/src/native_classes/emote_messages.rs#L103) |
| `0x406` | → 554 → : one row's action and voice names | Handled | — | [emote_messages.rs:110](../../crates/crysirror-catsystem2/src/native_classes/emote_messages.rs#L110) |
| `0x417` | → 552 → | Handled | — | [emote_messages.rs:128](../../crates/crysirror-catsystem2/src/native_classes/emote_messages.rs#L128) |
| `0x418` | → : every player's macro queue resets | Handled | — | [emote_messages.rs:57](../../crates/crysirror-catsystem2/src/native_classes/emote_messages.rs#L57) |

## Class calls / native_classes/export_task

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x2B2` | / → 507: the bundle (692 passes the second name) in the export folder | Handled | — | [export_task.rs:54](../../crates/crysirror-catsystem2/src/native_classes/export_task.rs#L54) |
| `0x2B3` | → 508: 1 once the worker ended; its failure surfaces here | Handled | — | [export_task.rs:86](../../crates/crysirror-catsystem2/src/native_classes/export_task.rs#L86) |
| `0x2B4` | / → 507: the bundle (692 passes the second name) in the export folder | Handled | — | [export_task.rs:54](../../crates/crysirror-catsystem2/src/native_classes/export_task.rs#L54) |

## Class calls / native_classes/host_misc

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x4C` | the calling task's parent → its id (+0xC, then +4) | Handled | — | [host_misc.rs:37](../../crates/crysirror-catsystem2/src/native_classes/host_misc.rs#L37) |
| `0xC9` | → 13: class 18 takes 264 and class 25 takes 273 when a1 == 0. Scripts create no class 25 | Handled | — | [host_misc.rs:44](../../crates/crysirror-catsystem2/src/native_classes/host_misc.rs#L44) |
| `0xF1` | Execute handle branch | Handled | — | [host_misc.rs:56](../../crates/crysirror-catsystem2/src/native_classes/host_misc.rs#L56) |
| `0x13B` | Execute handle branch | Handled | — | [host_misc.rs:143](../../crates/crysirror-catsystem2/src/native_classes/host_misc.rs#L143) |
| `0x13C` | Execute handle branch | Handled | — | [host_misc.rs:154](../../crates/crysirror-catsystem2/src/native_classes/host_misc.rs#L154) |
| `0x13D` | Execute handle branch | Handled | — | [host_misc.rs:159](../../crates/crysirror-catsystem2/src/native_classes/host_misc.rs#L159) |
| `0x202` | → : whether the set holds that motion; output pose resets either way | Handled | — | [host_misc.rs:166](../../crates/crysirror-catsystem2/src/native_classes/host_misc.rs#L166) |
| `0x240` | Execute handle branch | Handled | — | [host_misc.rs:71](../../crates/crysirror-catsystem2/src/native_classes/host_misc.rs#L71) |
| `0x241` | → root message 40: stores root+652, which nothing in the program reads | Handled | — | [host_misc.rs:179](../../crates/crysirror-catsystem2/src/native_classes/host_misc.rs#L179) |
| `0x256` | → : comma separated text; the first non-empty part goes to the buffer | Handled | — | [host_misc.rs:184](../../crates/crysirror-catsystem2/src/native_classes/host_misc.rs#L184) |
| `0x274` | 0x466 rebuilds a menu the host window does not have | Handled | — | [host_misc.rs:200](../../crates/crysirror-catsystem2/src/native_classes/host_misc.rs#L200) |
| `0x289` | → 343 → : a running FES child's local variables under key a1 | Handled | — | [host_misc.rs:75](../../crates/crysirror-catsystem2/src/native_classes/host_misc.rs#L75) |
| `0x28A` | → 344 → : the variables come back, and a child with a `_LOAD_` section resumes there | Handled | — | [host_misc.rs:90](../../crates/crysirror-catsystem2/src/native_classes/host_misc.rs#L90) |
| `0x2B0` | Execute handle branch | Handled | — | [host_misc.rs:71](../../crates/crysirror-catsystem2/src/native_classes/host_misc.rs#L71) |
| `0x2B1` | Execute handle branch | Handled | — | [host_misc.rs:71](../../crates/crysirror-catsystem2/src/native_classes/host_misc.rs#L71) |
| `0x2E4` | where the pointer goes to leave the window, under its client area in canvas units (+63432 selects the height source; al… | Handled | — | [host_misc.rs:214](../../crates/crysirror-catsystem2/src/native_classes/host_misc.rs#L214) |
| `0x2E5` | +0xF78C is set only for the exclusive fullscreen mode, which this host never enters, so the answer is the windowed 0 | Handled | — | [host_misc.rs:208](../../crates/crysirror-catsystem2/src/native_classes/host_misc.rs#L208) |
| `0x34E` | 0x46A → WM_COMMAND 0x9C55 → SC_MINIMIZE | Handled | — | [host_misc.rs:219](../../crates/crysirror-catsystem2/src/native_classes/host_misc.rs#L219) |
| `0x387` | → sound message 214 | Handled | — | [host_misc.rs:107](../../crates/crysirror-catsystem2/src/native_classes/host_misc.rs#L107) |
| `0x3F1` | encode by extension: bmp type 1, png type 7, anything else JPEG (type 5) | Handled | — | [host_misc.rs:227](../../crates/crysirror-catsystem2/src/native_classes/host_misc.rs#L227) |
| `0x419` | parse the format string as message text and keep its plain glyphs | Handled | — | [host_misc.rs:248](../../crates/crysirror-catsystem2/src/native_classes/host_misc.rs#L248) |

## Class calls / native_classes/image_ops

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x156` | → 292: this[35] = 1; with a manager marks every page dirty and recomposes (), then this[31] = 0 | Handled | — | [image_ops.rs:24](../../crates/crysirror-catsystem2/src/native_classes/image_ops.rs#L24) |
| `0x1D0` | → 300 → (key): restart the draw's frame motion () and mark its old and new rectangles dirty for the next composition | Handled | — | [image_ops.rs:35](../../crates/crysirror-catsystem2/src/native_classes/image_ops.rs#L35) |
| `0x216` | → 304: this[35] = argument | Handled | — | [image_ops.rs:49](../../crates/crysirror-catsystem2/src/native_classes/image_ops.rs#L49) |
| `0x271` | Execute handle branch | Handled | — | [image_ops.rs:45](../../crates/crysirror-catsystem2/src/native_classes/image_ops.rs#L45) |

## Class calls / native_classes/input_task

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x6A` | Input (handle) | Handled | — | [input_task.rs:51](../../crates/crysirror-catsystem2/src/native_classes/input_task.rs#L51) |
| `0x6D` | Input (handle) | Handled | — | [input_task.rs:65](../../crates/crysirror-catsystem2/src/native_classes/input_task.rs#L65) |
| `0x95` | → INPUT 125: pointer, to virtual canvas, into two VM words | Handled | — | [input_task.rs:73](../../crates/crysirror-catsystem2/src/native_classes/input_task.rs#L73) |
| `0x96` | Input (handle) | Handled | — | [input_task.rs:84](../../crates/crysirror-catsystem2/src/native_classes/input_task.rs#L84) |
| `0x11E` | → INPUT 145: clear every hover marker, forget the last pointer position (this[1233..1234] = -1) so the next tick re-eva… | Handled | — | [input_task.rs:97](../../crates/crysirror-catsystem2/src/native_classes/input_task.rs#L97) |
| `0x18A` | Input (handle) | Handled | — | [input_task.rs:106](../../crates/crysirror-catsystem2/src/native_classes/input_task.rs#L106) |
| `0x191` | → INPUT 149: shows/hides the software cursor; this[1309] keeps the accepted state for the idle auto-hide in | Handled | — | [input_task.rs:142](../../crates/crysirror-catsystem2/src/native_classes/input_task.rs#L142) |
| `0x194` | → INPUT 151: every key slot is polled again (memset32 of 80 ones) | Handled | — | [input_task.rs:151](../../crates/crysirror-catsystem2/src/native_classes/input_task.rs#L151) |
| `0x3A9` | → INPUT 157 → → : the mouse device forgets its accumulated wheel direction/notches (+0x604/+0x608, 6 up, 7 down) | Handled | — | [input_task.rs:160](../../crates/crysirror-catsystem2/src/native_classes/input_task.rs#L160) |

## Class calls / native_classes/name_table

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x279` | and read the file; a missing file answers 0 | Handled | — | [name_table.rs:159](../../crates/crysirror-catsystem2/src/native_classes/name_table.rs#L159) |
| `0x27A` | Execute handle branch | Handled | — | [name_table.rs:194](../../crates/crysirror-catsystem2/src/native_classes/name_table.rs#L194) |
| `0x27B` | → : first row with an alias equal to the name, else -1 | Handled | — | [name_table.rs:200](../../crates/crysirror-catsystem2/src/native_classes/name_table.rs#L200) |
| `0x27C` | → : row id, else -1 | Handled | — | [name_table.rs:207](../../crates/crysirror-catsystem2/src/native_classes/name_table.rs#L207) |
| `0x27F` | the VM name, cut to the length of cell 2, equals it ( ignores case) | Handled | — | [name_table.rs:213](../../crates/crysirror-catsystem2/src/native_classes/name_table.rs#L213) |
| `0x280` | expanded name into a VM buffer, answering 1 | Handled | — | [name_table.rs:226](../../crates/crysirror-catsystem2/src/native_classes/name_table.rs#L226) |

## Class calls / native_classes/os_window

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x09` | a1 != 0 posts WM_CLOSE ( case 0x10: beep, MessageBox ""/"" yes/no, then + 0x410); a1 == 0 posts 0x433, which runs (hide… | Handled | — | [os_window.rs:34](../../crates/crysirror-catsystem2/src/native_classes/os_window.rs#L34) |
| `0x1B9` | → (mode, folder, counter): a nonzero mode makes save every presented frame as "%s\%s_%010d.bmp" (folder, folder, counte… | Handled | — | [os_window.rs:44](../../crates/crysirror-catsystem2/src/native_classes/os_window.rs#L44) |
| `0x235` | → : the frame dump mode | Handled | — | [os_window.rs:56](../../crates/crysirror-catsystem2/src/native_classes/os_window.rs#L56) |

## Class calls / native_classes/particles

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x00` | Particle commands (native) | Handled | — | [particles.rs:173](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L173) |
| `0x01` | Particle commands (native) | Handled | — | [particles.rs:178](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L178) |
| `0x02` | Particle commands (native) | Handled | — | [particles.rs:179](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L179) |
| `0x03` | milliseconds to frames: 0 stays, negative stays, else at least 1 | Handled | — | [particles.rs:181](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L181) |
| `0x04` | Particle commands (native) | Handled | — | [particles.rs:185](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L185) |
| `0x05` | Particle commands (native) | Handled | — | [particles.rs:186](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L186) |
| `0x06` | Particle commands (native) | Handled | — | [particles.rs:190](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L190) |
| `0x07` | Particle commands (native) | Handled | — | [particles.rs:190](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L190) |
| `0x08` | Particle commands (native) | Handled | — | [particles.rs:190](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L190) |
| `0x09` | Particle commands (native) | Handled | — | [particles.rs:190](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L190) |
| `0x0A` | Particle commands (native) | Handled | — | [particles.rs:190](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L190) |
| `0x0B` | Particle commands (native) | Handled | — | [particles.rs:190](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L190) |
| `0x0C` | Particle commands (native) | Handled | — | [particles.rs:190](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L190) |
| `0x0D` | Particle commands (native) | Handled | — | [particles.rs:190](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L190) |
| `0x0E` | Particle commands (native) | Handled | — | [particles.rs:190](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L190) |
| `0x0F` | Particle commands (native) | Handled | — | [particles.rs:190](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L190) |
| `0x10` | Particle commands (native) | Handled | — | [particles.rs:190](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L190) |
| `0x11` | Particle commands (native) | Handled | — | [particles.rs:190](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L190) |
| `0x12` | Particle commands (native) | Handled | — | [particles.rs:190](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L190) |
| `0x13` | Particle commands (native) | Handled | — | [particles.rs:190](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L190) |
| `0x14` | Particle commands (native) | Handled | — | [particles.rs:190](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L190) |
| `0x15` | Particle commands (native) | Handled | — | [particles.rs:190](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L190) |
| `0x16` | Particle commands (native) | Handled | — | [particles.rs:199](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L199) |
| `0x17` | Particle commands (native) | Handled | — | [particles.rs:200](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L200) |
| `0x18` | Particle commands (native) | Handled | — | [particles.rs:201](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L201) |
| `0x19` | Particle commands (native) | Handled | — | [particles.rs:202](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L202) |
| `0x1A` | Rotations, translation and scale of a VM quad (, , , ) | Handled | — | [particles.rs:192](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L192) |
| `0x1B` | Particle commands (native) | Handled | — | [particles.rs:212](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L212) |
| `0x1C` | Particle commands (native) | Handled | — | [particles.rs:216](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L216) |
| `0x1D` | reads only words 0 and 2, half a texel in, for both axes | Handled | — | [particles.rs:222](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L222) |
| `0x1E` | Particle commands (native) | Handled | — | [particles.rs:233](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L233) |
| `0x1F` | Particle commands (native) | Handled | — | [particles.rs:238](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L238) |
| `0x20` | running off; handler result 4 ends this script run | Handled | — | [particles.rs:245](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L245) |
| `0x21` | Particle commands (native) | Handled | — | [particles.rs:250](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L250) |
| `0x22` | Particle commands (native) | Handled | — | [particles.rs:251](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L251) |
| `0x23` | Particle commands (native) | Handled | — | [particles.rs:252](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L252) |
| `0x24` | Particle commands (native) | Handled | — | [particles.rs:256](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L256) |
| `0x25` | Particle commands (native) | Handled | — | [particles.rs:257](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L257) |
| `0x26` | the rectangle the whole screen covers at depth a0 | Handled | — | [particles.rs:262](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L262) |
| `0x27` | Particle commands (native) | Handled | — | [particles.rs:271](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L271) |
| `0x28` | Particle commands (native) | Handled | — | [particles.rs:277](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L277) |
| `0x29` | Particle commands (native) | Handled | — | [particles.rs:278](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L278) |
| `0x2A` | Particle commands (native) | Handled | — | [particles.rs:279](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L279) |
| `0x2B` | Particle commands (native) | Handled | — | [particles.rs:280](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L280) |
| `0x2C` | restores the quads from the copy took (version 0x20000 scripts) | Handled | — | [particles.rs:282](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L282) |
| `0x2D` | Particle commands (native) | Handled | — | [particles.rs:286](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L286) |
| `0x2E` | Particle commands (native) | Handled | — | [particles.rs:290](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L290) |
| `0x2F` | → and → : the attached object's position (x, y) and base (x, y, z); no object or another class leaves zeros | Handled | — | [particles.rs:293](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L293) |
| `0x30` | → → answers object word +4, which no Plane field models yet | Unsupported | CatSystem2 particle native 48 (attached object word +4) is not implemented | [particles.rs:302](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L302) |
| `0x31` | → and → : the attached object's position (x, y) and base (x, y, z); no object or another class leaves zeros | Handled | — | [particles.rs:293](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L293) |
| `0x32` | 1..5 select a blend, anything else leaves it; 0 is normal | Handled | — | [particles.rs:304](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L304) |
| `0x10F` | / → 314: script or plugin, image, count, z, parameters. A task of another class ignores the message; the native still a… | Handled | — | [particles.rs:399](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L399) |
| `0x110` | → 315: reset, run; a plugin that did not load stops at once | Handled | — | [particles.rs:459](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L459) |
| `0x111` | → 316: stop and take the draw callback away () | Handled | — | [particles.rs:469](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L469) |
| `0x112` | → 318 | Handled | — | [particles.rs:482](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L482) |
| `0x113` | → 319: new script parameters | Handled | — | [particles.rs:488](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L488) |
| `0x1AA` | → 320: alpha, 255 and above meaning 256 | Handled | — | [particles.rs:498](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L498) |
| `0x1AB` | → 321: attached object, 0 meaning none | Handled | — | [particles.rs:507](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L507) |
| `0x2B5` | / → 314: script or plugin, image, count, z, parameters. A task of another class ignores the message; the native still a… | Handled | — | [particles.rs:399](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L399) |

## Class calls / native_classes/plane_effects

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x51` | → 69 → : a `kx2` model. A missing file leaves the Plane empty | Handled | — | [plane_effects.rs:167](../../crates/crysirror-catsystem2/src/native_classes/plane_effects.rs#L167) |
| `0x158` | → 57 → : the object's world matrix (D3D rows, translation last) | Handled | — | [plane_effects.rs:181](../../crates/crysirror-catsystem2/src/native_classes/plane_effects.rs#L181) |
| `0x18E` | → 79: a1 == 0 removes the trail (); otherwise builds it | Handled | — | [plane_effects.rs:213](../../crates/crysirror-catsystem2/src/native_classes/plane_effects.rs#L213) |
| `0x18F` | → 80: the script supplies the newest record | Handled | — | [plane_effects.rs:263](../../crates/crysirror-catsystem2/src/native_classes/plane_effects.rs#L263) |
| `0x1B1` | → 46: a grid of "ptn plane" children, each cell a target quad | Handled | — | [plane_effects.rs:277](../../crates/crysirror-catsystem2/src/native_classes/plane_effects.rs#L277) |
| `0x2BD` | Visual effects (handle) | Handled | — | [plane_effects.rs:304](../../crates/crysirror-catsystem2/src/native_classes/plane_effects.rs#L304) |
| `0x2BE` | / → 87 / 88: the Plane's video object, 0 without one | Handled | — | [plane_effects.rs:324](../../crates/crysirror-catsystem2/src/native_classes/plane_effects.rs#L324) |
| `0x3DC` | Visual effects (handle) | Handled | — | [plane_effects.rs:304](../../crates/crysirror-catsystem2/src/native_classes/plane_effects.rs#L304) |
| `0x3DD` | / → 87 / 88: the Plane's video object, 0 without one | Handled | — | [plane_effects.rs:324](../../crates/crysirror-catsystem2/src/native_classes/plane_effects.rs#L324) |

## Class calls / native_classes/plane_messages

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x5F` | Text and message windows (handle) | Handled | — | [plane_messages.rs:68](../../crates/crysirror-catsystem2/src/native_classes/plane_messages.rs#L68) |
| `0x15A` | Text and message windows (handle) | Handled | — | [plane_messages.rs:22](../../crates/crysirror-catsystem2/src/native_classes/plane_messages.rs#L22) |
| `0x192` | Text and message windows (handle) | Handled | — | [plane_messages.rs:41](../../crates/crysirror-catsystem2/src/native_classes/plane_messages.rs#L41) |
| `0x28B` | (msg 24, -1) and (msg 29): forwards 24..40 to , whose switch has neither case (default returns 1); no other class handl… | Handled | — | [plane_messages.rs:30](../../crates/crysirror-catsystem2/src/native_classes/plane_messages.rs#L30) |
| `0x28D` | Text and message windows (handle) | Handled | — | [plane_messages.rs:57](../../crates/crysirror-catsystem2/src/native_classes/plane_messages.rs#L57) |
| `0x28E` | Text and message windows (handle) | Handled | — | [plane_messages.rs:22](../../crates/crysirror-catsystem2/src/native_classes/plane_messages.rs#L22) |
| `0x28F` | Text and message windows (handle) | Handled | — | [plane_messages.rs:22](../../crates/crysirror-catsystem2/src/native_classes/plane_messages.rs#L22) |
| `0x2B6` | (msg 24, -1) and (msg 29): forwards 24..40 to , whose switch has neither case (default returns 1); no other class handl… | Handled | — | [plane_messages.rs:30](../../crates/crysirror-catsystem2/src/native_classes/plane_messages.rs#L30) |

## Class calls / native_classes/render_settings

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x21D` | → / → : free the VRAM tile textures under a rectangle or all of them. Tiles only cache; the Canvas keeps every draw it … | Empty handler | Original no-op status not established | [render_settings.rs:23](../../crates/crysirror-catsystem2/src/native_classes/render_settings.rs#L23) |
| `0x21E` | → / → : free the VRAM tile textures under a rectangle or all of them. Tiles only cache; the Canvas keeps every draw it … | Empty handler | Original no-op status not established | [render_settings.rs:23](../../crates/crysirror-catsystem2/src/native_classes/render_settings.rs#L23) |
| `0x238` | / → : depth range of the device viewport (and 859's extra flag). Layer order is explicit here, so the depth range decid… | Empty handler | Original no-op status not established | [render_settings.rs:26](../../crates/crysirror-catsystem2/src/native_classes/render_settings.rs#L26) |
| `0x2A9` | → : drop a decoded image from the cache; resources are shared | Empty handler | Original no-op status not established | [render_settings.rs:28](../../crates/crysirror-catsystem2/src/native_classes/render_settings.rs#L28) |
| `0x35A` | → : queries pixel shader support and the result is dropped | Empty handler | Original no-op status not established | [render_settings.rs:30](../../crates/crysirror-catsystem2/src/native_classes/render_settings.rs#L30) |
| `0x35B` | / → : depth range of the device viewport (and 859's extra flag). Layer order is explicit here, so the depth range decid… | Empty handler | Original no-op status not established | [render_settings.rs:26](../../crates/crysirror-catsystem2/src/native_classes/render_settings.rs#L26) |
| `0x36F` | Execute handle branch | Handled | — | [render_settings.rs:34](../../crates/crysirror-catsystem2/src/native_classes/render_settings.rs#L34) |
| `0x371` | → 313: the link's cutoff time | Handled | — | [render_settings.rs:44](../../crates/crysirror-catsystem2/src/native_classes/render_settings.rs#L44) |

## Class calls / native_classes/scene_records

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x3DF` | → : prepend a record to the last history entry | Handled | — | [scene_records.rs:120](../../crates/crysirror-catsystem2/src/native_classes/scene_records.rs#L120) |
| `0x3E0` | → : search back from *a1 for a record at or before *a2 in that entry, restore it, and write where it was found | Handled | — | [scene_records.rs:145](../../crates/crysirror-catsystem2/src/native_classes/scene_records.rs#L145) |
| `0x3E2` | Scene transfer/state (handle) | Handled | — | [scene_records.rs:190](../../crates/crysirror-catsystem2/src/native_classes/scene_records.rs#L190) |

## Class calls / native_classes/shader_tasks

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0xC4` | 263 → | Handled | — | [shader_tasks.rs:267](../../crates/crysirror-catsystem2/src/native_classes/shader_tasks.rs#L267) |
| `0xC5` | Native call (raster_native) | Handled | — | [shader_tasks.rs:280](../../crates/crysirror-catsystem2/src/native_classes/shader_tasks.rs#L280) |
| `0xC8` | Native call (raster_native) | Handled | — | [shader_tasks.rs:301](../../crates/crysirror-catsystem2/src/native_classes/shader_tasks.rs#L301) |
| `0xC9` | 264 when a1 == 0: new amplitude, wave and period while it runs | Handled | — | [shader_tasks.rs:288](../../crates/crysirror-catsystem2/src/native_classes/shader_tasks.rs#L288) |

## Class calls / native_classes/subsystems

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0xA8` | System commands (handle) | Handled | Guard: a.len() >= 4 | [subsystems.rs:66](../../crates/crysirror-catsystem2/src/native_classes/subsystems.rs#L66) |
| `0x117` | System commands (handle) | Handled | — | [subsystems.rs:82](../../crates/crysirror-catsystem2/src/native_classes/subsystems.rs#L82) |
| `0x118` | System commands (handle) | Handled | — | [subsystems.rs:82](../../crates/crysirror-catsystem2/src/native_classes/subsystems.rs#L82) |
| `0x119` | System commands (handle) | Handled | — | [subsystems.rs:82](../../crates/crysirror-catsystem2/src/native_classes/subsystems.rs#L82) |
| `0x11A` | System commands (handle) | Handled | — | [subsystems.rs:82](../../crates/crysirror-catsystem2/src/native_classes/subsystems.rs#L82) |
| `0x11B` | System commands (handle) | Handled | — | [subsystems.rs:82](../../crates/crysirror-catsystem2/src/native_classes/subsystems.rs#L82) |
| `0x11F` | System commands (handle) | Handled | — | [subsystems.rs:82](../../crates/crysirror-catsystem2/src/native_classes/subsystems.rs#L82) |
| `0x120` | System commands (handle) | Handled | — | [subsystems.rs:82](../../crates/crysirror-catsystem2/src/native_classes/subsystems.rs#L82) |
| `0x140` | System commands (handle) | Handled | — | [subsystems.rs:38](../../crates/crysirror-catsystem2/src/native_classes/subsystems.rs#L38) |
| `0x15B` | System commands (handle) | Handled | — | [subsystems.rs:59](../../crates/crysirror-catsystem2/src/native_classes/subsystems.rs#L59) |
| `0x15C` | System commands (handle) | Handled | Guard: a.len() >= 2 | [subsystems.rs:67](../../crates/crysirror-catsystem2/src/native_classes/subsystems.rs#L67) |
| `0x237` | append the formatted text and CR LF to a file in the game folder | Handled | — | [subsystems.rs:105](../../crates/crysirror-catsystem2/src/native_classes/subsystems.rs#L105) |
| `0x23A` | System commands (handle) | Handled | — | [subsystems.rs:82](../../crates/crysirror-catsystem2/src/native_classes/subsystems.rs#L82) |
| `0x258` | System commands (handle) | Handled | — | [subsystems.rs:138](../../crates/crysirror-catsystem2/src/native_classes/subsystems.rs#L138) |
| `0x259` | → 448: 1 once no worker runs | Handled | — | [subsystems.rs:144](../../crates/crysirror-catsystem2/src/native_classes/subsystems.rs#L144) |
| `0x263` | System commands (handle) | Handled | — | [subsystems.rs:93](../../crates/crysirror-catsystem2/src/native_classes/subsystems.rs#L93) |
| `0x264` | System commands (handle) | Handled | — | [subsystems.rs:93](../../crates/crysirror-catsystem2/src/native_classes/subsystems.rs#L93) |
| `0x265` | System commands (handle) | Handled | — | [subsystems.rs:93](../../crates/crysirror-catsystem2/src/native_classes/subsystems.rs#L93) |
| `0x266` | System commands (handle) | Handled | — | [subsystems.rs:100](../../crates/crysirror-catsystem2/src/native_classes/subsystems.rs#L100) |
| `0x267` | System commands (handle) | Handled | — | [subsystems.rs:100](../../crates/crysirror-catsystem2/src/native_classes/subsystems.rs#L100) |
| `0x269` | System commands (handle) | Handled | — | [subsystems.rs:100](../../crates/crysirror-catsystem2/src/native_classes/subsystems.rs#L100) |
| `0x26B` | System commands (handle) | Handled | — | [subsystems.rs:100](../../crates/crysirror-catsystem2/src/native_classes/subsystems.rs#L100) |
| `0x26C` | System commands (handle) | Handled | — | [subsystems.rs:100](../../crates/crysirror-catsystem2/src/native_classes/subsystems.rs#L100) |
| `0x26D` | System commands (handle) | Handled | — | [subsystems.rs:100](../../crates/crysirror-catsystem2/src/native_classes/subsystems.rs#L100) |
| `0x388` | comma list a1 is written over list a0; an "@" keeps a0's token at that place, or "0" when a0 has none. drops empty toke… | Handled | — | [subsystems.rs:123](../../crates/crysirror-catsystem2/src/native_classes/subsystems.rs#L123) |
| `0x3FA` | Tweet account, attachment and text fields only feed that worker | Handled | — | [subsystems.rs:149](../../crates/crysirror-catsystem2/src/native_classes/subsystems.rs#L149) |
| `0x3FB` | Tweet account, attachment and text fields only feed that worker | Handled | — | [subsystems.rs:149](../../crates/crysirror-catsystem2/src/native_classes/subsystems.rs#L149) |
| `0x3FC` | Tweet account, attachment and text fields only feed that worker | Handled | — | [subsystems.rs:149](../../crates/crysirror-catsystem2/src/native_classes/subsystems.rs#L149) |
| `0x3FD` | System commands (handle) | Handled | — | [subsystems.rs:138](../../crates/crysirror-catsystem2/src/native_classes/subsystems.rs#L138) |
| `0x3FE` | → 578: the worker's result this[541], which stays 0 | Handled | — | [subsystems.rs:154](../../crates/crysirror-catsystem2/src/native_classes/subsystems.rs#L154) |
| `0x3FF` | Tweet account, attachment and text fields only feed that worker | Handled | — | [subsystems.rs:149](../../crates/crysirror-catsystem2/src/native_classes/subsystems.rs#L149) |
| `0x40C` | main and sub text language prefixes into two VM buffers | Handled | — | [subsystems.rs:159](../../crates/crysirror-catsystem2/src/native_classes/subsystems.rs#L159) |

## Class calls / native_classes/tables_3d

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x252` | a missing file answers 0; its readers then answer 0 | Handled | — | [tables_3d.rs:138](../../crates/crysirror-catsystem2/src/native_classes/tables_3d.rs#L138) |
| `0x253` | Execute handle branch | Handled | — | [tables_3d.rs:169](../../crates/crysirror-catsystem2/src/native_classes/tables_3d.rs#L169) |
| `0x254` | → by tone name, → by image name; R, G, B into VM words | Handled | — | [tables_3d.rs:175](../../crates/crysirror-catsystem2/src/native_classes/tables_3d.rs#L175) |
| `0x255` | → by tone name, → by image name; R, G, B into VM words | Handled | — | [tables_3d.rs:175](../../crates/crysirror-catsystem2/src/native_classes/tables_3d.rs#L175) |
| `0x2AB` | kind 19 names `archive/member` directly; a missing file is 0 | Handled | — | [tables_3d.rs:191](../../crates/crysirror-catsystem2/src/native_classes/tables_3d.rs#L191) |
| `0x2AC` | Execute handle branch | Handled | — | [tables_3d.rs:222](../../crates/crysirror-catsystem2/src/native_classes/tables_3d.rs#L222) |
| `0x2AD` | copies the value, terminated, and answers 1 | Handled | — | [tables_3d.rs:229](../../crates/crysirror-catsystem2/src/native_classes/tables_3d.rs#L229) |

## Class calls / native_classes/text_window

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x7B` | (418) / (123) → 101: 96 config bytes, then one (123) or two (418) kinsoku sets converted to big-endian SJIS WORDs | Handled | — | [text_window.rs:456](../../crates/crysirror-catsystem2/src/native_classes/text_window.rs#L456) |
| `0x7C` | → 103: append formatted text, then draw it unless the window reveals on its timer (+0x308 == 1) | Partial | CatSystem2 text window timed reveal (+0x308 == 1) is not implemented | [text_window.rs:510](../../crates/crysirror-catsystem2/src/native_classes/text_window.rs#L510) |
| `0x7D` | → 104: forget every record, cursor to +0x2F0/+0x2F4, colour reset; clears and recopies the bitmap | Handled | — | [text_window.rs:524](../../crates/crysirror-catsystem2/src/native_classes/text_window.rs#L524) |
| `0x1A2` | (418) / (123) → 101: 96 config bytes, then one (123) or two (418) kinsoku sets converted to big-endian SJIS WORDs | Handled | — | [text_window.rs:456](../../crates/crysirror-catsystem2/src/native_classes/text_window.rs#L456) |
| `0x22D` | → 121: this+0x758 edge colour | Handled | — | [text_window.rs:537](../../crates/crysirror-catsystem2/src/native_classes/text_window.rs#L537) |

## FES command names

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `(PendingKind::Native, _redirect)` | redirect | Handled | — | [fes_runtime.rs:590](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L590) |
| `(PendingKind::Native, _redirect_clear)` | redirect clear | Handled | — | [fes_runtime.rs:590](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L590) |
| `(PendingKind::Native, _redirect_erase)` | redirect erase | Handled | — | [fes_runtime.rs:590](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L590) |
| `(PendingKind::Native, automove)` | // -> messages9/10/30 ->INPUT137 | Handled | — | [fes_runtime.rs:610](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L610) |
| `(PendingKind::Native, automoveex)` | // -> messages9/10/30 ->INPUT137 | Handled | — | [fes_runtime.rs:610](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L610) |
| `(PendingKind::Native, basebtnpri)` | , , : button-pass requests that take effect on the next frame | Handled | — | [fes_runtime.rs:639](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L639) |
| `(PendingKind::Native, baseimg)` | baseimg | Handled | — | [fes_runtime.rs:868](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L868) |
| `(PendingKind::Native, baseimg_pri)` | baseimg pri | Handled | — | [fes_runtime.rs:872](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L872) |
| `(PendingKind::Native, bitcount)` | the set bits of a value into a flag | Handled | — | [fes_runtime.rs:766](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L766) |
| `(PendingKind::Native, break)` | break | Handled | — | [fes_runtime.rs:851](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L851) |
| `(PendingKind::Native, btnsimulate)` | , , : button-pass requests that take effect on the next frame | Handled | — | [fes_runtime.rs:639](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L639) |
| `(PendingKind::Native, cgreg)` | `dest name,variant,flags` | Handled | — | [fes_runtime.rs:468](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L468) |
| `(PendingKind::Native, cgreg_getlist)` | with no list entry found the string variable is emptied | Handled | — | [fes_runtime.rs:757](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L757) |
| `(PendingKind::Native, cgreg_init)` | `cgreg_init file key list`: the record under key in that save file, plus the group list, on a worker | Handled | — | [fes_runtime.rs:384](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L384) |
| `(PendingKind::Native, cgreg_t)` | , , : `dest [list] group`; a group starting with $, \ or a digit is a record number, anything else a name in that list.… | Handled | — | [fes_runtime.rs:446](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L446) |
| `(PendingKind::Native, cgregmax)` | , , : `dest [list] group`; a group starting with $, \ or a digit is a record number, anything else a name in that list.… | Handled | — | [fes_runtime.rs:446](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L446) |
| `(PendingKind::Native, cgregnum)` | , , : `dest [list] group`; a group starting with $, \ or a digit is a record number, anything else a name in that list.… | Handled | — | [fes_runtime.rs:446](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L446) |
| `(PendingKind::Native, cgregtbl)` | `dest list name` | Handled | — | [fes_runtime.rs:477](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L477) |
| `(PendingKind::Native, check_install)` | check install | Handled | — | [fes_runtime.rs:776](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L776) |
| `(PendingKind::Native, checktitle)` | checktitle | Handled | — | [fes_runtime.rs:818](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L818) |
| `(PendingKind::Native, clipboard)` | , , : a browser and the system clipboard are outside this player | Handled | — | [fes_runtime.rs:837](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L837) |
| `(PendingKind::Native, config)` | a volume into a flag; other names leave it | Handled | — | [fes_runtime.rs:825](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L825) |
| `(PendingKind::Native, config_open)` | config open | Handled | — | [fes_runtime.rs:842](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L842) |
| `(PendingKind::Native, cursormove)` | cursormove | Handled | — | [fes_runtime.rs:818](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L818) |
| `(PendingKind::Native, datasave)` | , , , , , , , , : slot files | Handled | — | [fes_runtime.rs:534](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L534) |
| `(PendingKind::Native, dbgstr)` | Developer output and Steam overlays: nothing for a player to see on this host | Empty handler | Original no-op status not established | [fes_runtime.rs:381](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L381) |
| `(PendingKind::Native, dbgstru8)` | Developer output and Steam overlays: nothing for a player to see on this host | Empty handler | Original no-op status not established | [fes_runtime.rs:381](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L381) |
| `(PendingKind::Native, dbgtime)` | dbgtime | Handled | — | [fes_runtime.rs:818](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L818) |
| `(PendingKind::Native, debug)` | Developer output and Steam overlays: nothing for a player to see on this host | Empty handler | Original no-op status not established | [fes_runtime.rs:381](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L381) |
| `(PendingKind::Native, decode)` | Window chrome and developer hooks with no effect on this host's scene | Empty handler | Original no-op status not established | [fes_runtime.rs:407](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L407) |
| `(PendingKind::Native, deletebgbutton)` | Window chrome and developer hooks with no effect on this host's scene | Empty handler | Original no-op status not established | [fes_runtime.rs:407](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L407) |
| `(PendingKind::Native, deletetask)` | `send target a b [wait]` posts a message the owning KCS reads from native71 | Handled | — | [fes_runtime.rs:409](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L409) |
| `(PendingKind::Native, dic_appear)` | the read-marks database of KCS native 869, by the handle in the first word | Handled | — | [fes_runtime.rs:672](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L672) |
| `(PendingKind::Native, dic_find)` | the read-marks database of KCS native 869, by the handle in the first word | Handled | — | [fes_runtime.rs:672](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L672) |
| `(PendingKind::Native, dic_id)` | the read-marks database of KCS native 869, by the handle in the first word | Handled | — | [fes_runtime.rs:672](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L672) |
| `(PendingKind::Native, dic_ind)` | the read-marks database of KCS native 869, by the handle in the first word | Handled | — | [fes_runtime.rs:672](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L672) |
| `(PendingKind::Native, dic_max_appear)` | the read-marks database of KCS native 869, by the handle in the first word | Handled | — | [fes_runtime.rs:672](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L672) |
| `(PendingKind::Native, dic_title)` | the read-marks database of KCS native 869, by the handle in the first word | Handled | — | [fes_runtime.rs:672](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L672) |
| `(PendingKind::Native, dic_update)` | the read-marks database of KCS native 869, by the handle in the first word | Handled | — | [fes_runtime.rs:672](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L672) |
| `(PendingKind::Native, dic_update_clear)` | the read-marks database of KCS native 869, by the handle in the first word | Handled | — | [fes_runtime.rs:672](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L672) |
| `(PendingKind::Native, enablebgbutton)` | enablebgbutton | Handled | — | [fes_runtime.rs:818](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L818) |
| `(PendingKind::Native, endfes)` | endfes | Handled | — | [fes_runtime.rs:942](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L942) |
| `(PendingKind::Native, endkcs)` | endkcs | Handled | — | [fes_runtime.rs:942](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L942) |
| `(PendingKind::Native, execfes)` | execfes | Handled | — | [fes_runtime.rs:931](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L931) |
| `(PendingKind::Native, execkcs)` | execkcs | Handled | — | [fes_runtime.rs:935](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L935) |
| `(PendingKind::Native, exit)` | exit | Handled | — | [fes_runtime.rs:886](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L886) |
| `(PendingKind::Native, exit_boot)` | exit boot | Handled | — | [fes_runtime.rs:818](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L818) |
| `(PendingKind::Native, fontcom)` | the system font list. This player does not enumerate host fonts, so the list is empty: count answers 0 and a search fin… | Handled | — | [fes_runtime.rs:743](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L743) |
| `(PendingKind::Native, force_redraw)` | Window chrome and developer hooks with no effect on this host's scene | Empty handler | Original no-op status not established | [fes_runtime.rs:407](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L407) |
| `(PendingKind::Native, fps_enable)` | , : the frame counters report the configured rate | Empty handler | Original no-op status not established | [fes_runtime.rs:786](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L786) |
| `(PendingKind::Native, fps_get)` | fps get | Handled | — | [fes_runtime.rs:787](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L787) |
| `(PendingKind::Native, fw_actcode)` | The fw_* disc and activation commands do nothing, by owner decision: the scripts set their own status flag ($700=2) str… | Empty handler | Original no-op status not established | [fes_runtime.rs:823](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L823) |
| `(PendingKind::Native, fw_activate)` | The fw_* disc and activation commands do nothing, by owner decision: the scripts set their own status flag ($700=2) str… | Empty handler | Original no-op status not established | [fes_runtime.rs:823](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L823) |
| `(PendingKind::Native, fw_auth)` | The fw_* disc and activation commands do nothing, by owner decision: the scripts set their own status flag ($700=2) str… | Empty handler | Original no-op status not established | [fes_runtime.rs:823](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L823) |
| `(PendingKind::Native, fw_destroy)` | The fw_* disc and activation commands do nothing, by owner decision: the scripts set their own status flag ($700=2) str… | Empty handler | Original no-op status not established | [fes_runtime.rs:823](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L823) |
| `(PendingKind::Native, fw_hash)` | The fw_* disc and activation commands do nothing, by owner decision: the scripts set their own status flag ($700=2) str… | Empty handler | Original no-op status not established | [fes_runtime.rs:823](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L823) |
| `(PendingKind::Native, fw_init)` | The fw_* disc and activation commands do nothing, by owner decision: the scripts set their own status flag ($700=2) str… | Empty handler | Original no-op status not established | [fes_runtime.rs:823](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L823) |
| `(PendingKind::Native, fw_limitpos)` | The fw_* disc and activation commands do nothing, by owner decision: the scripts set their own status flag ($700=2) str… | Empty handler | Original no-op status not established | [fes_runtime.rs:823](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L823) |
| `(PendingKind::Native, fw_status)` | The fw_* disc and activation commands do nothing, by owner decision: the scripts set their own status flag ($700=2) str… | Empty handler | Original no-op status not established | [fes_runtime.rs:823](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L823) |
| `(PendingKind::Native, fw_uniquecode)` | The fw_* disc and activation commands do nothing, by owner decision: the scripts set their own status flag ($700=2) str… | Empty handler | Original no-op status not established | [fes_runtime.rs:823](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L823) |
| `(PendingKind::Native, getexvar)` | getexvar | Handled | — | [fes_runtime.rs:805](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L805) |
| `(PendingKind::Native, getstbl)` | getstbl | Handled | — | [fes_runtime.rs:879](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L879) |
| `(PendingKind::Native, gettime)` | gettime | Handled | — | [fes_runtime.rs:913](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L913) |
| `(PendingKind::Native, getweek)` | getweek | Handled | — | [fes_runtime.rs:900](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L900) |
| `(PendingKind::Native, input_text)` | input text | Handled | — | [fes_runtime.rs:842](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L842) |
| `(PendingKind::Native, inputdevice)` | inputdevice | Handled | — | [fes_runtime.rs:776](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L776) |
| `(PendingKind::Native, isdebug)` | isdebug | Handled | — | [fes_runtime.rs:776](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L776) |
| `(PendingKind::Native, lockactive)` | posts message 22 about the window's active lock; the host owns focus | Empty handler | Original no-op status not established | [fes_runtime.rs:841](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L841) |
| `(PendingKind::Native, m2_getconf)` | , : E-mote options, keyed by word; the labelled ones take a last word | Handled | — | [fes_runtime.rs:650](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L650) |
| `(PendingKind::Native, m2_resetview)` | m2 resetview | Handled | — | [fes_runtime.rs:818](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L818) |
| `(PendingKind::Native, m2_setconf)` | m2 setconf | Handled | — | [fes_runtime.rs:660](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L660) |
| `(PendingKind::Native, m2_waiteff)` | m2 waiteff | Handled | — | [fes_runtime.rs:818](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L818) |
| `(PendingKind::Native, maketask)` | `send target a b [wait]` posts a message the owning KCS reads from native71 | Handled | — | [fes_runtime.rs:409](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L409) |
| `(PendingKind::Native, messagebox)` | messagebox | Handled | — | [fes_runtime.rs:842](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L842) |
| `(PendingKind::Native, newsave)` | , , , , , , , , : slot files | Handled | — | [fes_runtime.rs:534](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L534) |
| `(PendingKind::Native, opensite)` | , , : a browser and the system clipboard are outside this player | Handled | — | [fes_runtime.rs:837](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L837) |
| `(PendingKind::Native, opensiteenc)` | , , : a browser and the system clipboard are outside this player | Handled | — | [fes_runtime.rs:837](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L837) |
| `(PendingKind::Native, psversion)` | psversion | Handled | — | [fes_runtime.rs:776](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L776) |
| `(PendingKind::Native, rand)` | rand | Handled | — | [fes_runtime.rs:860](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L860) |
| `(PendingKind::Native, randtbl)` | randtbl | Handled | — | [fes_runtime.rs:860](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L860) |
| `(PendingKind::Native, rdraw)` | rdraw | Handled | — | [fes_runtime.rs:493](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L493) |
| `(PendingKind::Native, react)` | , , : button-pass requests that take effect on the next frame | Handled | — | [fes_runtime.rs:639](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L639) |
| `(PendingKind::Native, replacespace)` | replacespace | Handled | — | [fes_runtime.rs:883](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L883) |
| `(PendingKind::Native, resetactive)` | Window chrome and developer hooks with no effect on this host's scene | Empty handler | Original no-op status not established | [fes_runtime.rs:407](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L407) |
| `(PendingKind::Native, restoremove)` | // -> messages9/10/30 ->INPUT137 | Handled | — | [fes_runtime.rs:610](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L610) |
| `(PendingKind::Native, result)` | result | Handled | — | [fes_runtime.rs:886](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L886) |
| `(PendingKind::Native, rettitle)` | event 19: the owner's script returns to its title | Handled | — | [fes_runtime.rs:846](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L846) |
| `(PendingKind::Native, rwipe)` | rwipe | Handled | — | [fes_runtime.rs:493](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L493) |
| `(PendingKind::Native, saveapend_str)` | , , , , , , , , : slot files | Handled | — | [fes_runtime.rs:534](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L534) |
| `(PendingKind::Native, savecopy)` | , , , , , , , , : slot files | Handled | — | [fes_runtime.rs:534](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L534) |
| `(PendingKind::Native, savedelete)` | , , , , , , , , : slot files | Handled | — | [fes_runtime.rs:534](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L534) |
| `(PendingKind::Native, saveexchg)` | , , , , , , , , : slot files | Handled | — | [fes_runtime.rs:534](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L534) |
| `(PendingKind::Native, saveexist)` | , , , , , , , , : slot files | Handled | — | [fes_runtime.rs:534](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L534) |
| `(PendingKind::Native, saveget_date)` | , , , , , , , , : slot files | Handled | — | [fes_runtime.rs:534](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L534) |
| `(PendingKind::Native, saveget_str)` | , , , , , , , , : slot files | Handled | — | [fes_runtime.rs:534](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L534) |
| `(PendingKind::Native, send)` | send | Handled | — | [fes_runtime.rs:437](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L437) |
| `(PendingKind::Native, sendbtn)` | Window chrome and developer hooks with no effect on this host's scene | Empty handler | Original no-op status not established | [fes_runtime.rs:407](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L407) |
| `(PendingKind::Native, setexvar)` | , : values kept by name for the session | Handled | — | [fes_runtime.rs:797](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L797) |
| `(PendingKind::Native, setstbl)` | setstbl | Handled | — | [fes_runtime.rs:879](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L879) |
| `(PendingKind::Native, sortsave)` | , : slots ordered newest file first into consecutive flags; a slot with no file has the zero time. Equal times keep the… | Handled | — | [fes_runtime.rs:720](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L720) |
| `(PendingKind::Native, sortsave2)` | , : slots ordered newest file first into consecutive flags; a slot with no file has the zero time. Equal times keep the… | Handled | — | [fes_runtime.rs:720](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L720) |
| `(PendingKind::Native, steam_achievement)` | Developer output and Steam overlays: nothing for a player to see on this host | Empty handler | Original no-op status not established | [fes_runtime.rs:381](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L381) |
| `(PendingKind::Native, steam_achievement_reset)` | steam achievement reset | Handled | — | [fes_runtime.rs:818](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L818) |
| `(PendingKind::Native, steam_enablescreenshot)` | Developer output and Steam overlays: nothing for a player to see on this host | Empty handler | Original no-op status not established | [fes_runtime.rs:381](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L381) |
| `(PendingKind::Native, steam_isbroadcast)` | steam isbroadcast | Handled | — | [fes_runtime.rs:485](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L485) |
| `(PendingKind::Native, strcmp)` | Compare strings | Handled | — | [fes_runtime.rs:876](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L876) |
| `(PendingKind::Native, strvar)` | strvar | Handled | — | [fes_runtime.rs:857](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L857) |
| `(PendingKind::Native, sysvoice)` | , : the configured system voice: none is registered, so -1 | Handled | — | [fes_runtime.rs:644](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L644) |
| `(PendingKind::Native, sysvoiceex)` | , : the configured system voice: none is registered, so -1 | Handled | — | [fes_runtime.rs:644](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L644) |
| `(PendingKind::Native, touch_enable)` | touch enable | Handled | — | [fes_runtime.rs:818](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L818) |
| `(PendingKind::Native, touch_kill)` | touch kill | Handled | — | [fes_runtime.rs:818](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L818) |
| `(PendingKind::Native, vram_get)` | vram get | Handled | — | [fes_runtime.rs:776](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L776) |
| `(PendingKind::Native, wait)` | wait | Handled | — | [fes_runtime.rs:350](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L350) |
| `(PendingKind::Native, wndmenu)` | Window chrome and developer hooks with no effect on this host's scene | Empty handler | Original no-op status not established | [fes_runtime.rs:407](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L407) |
| `(PendingKind::Native, xchgcursor)` | Developer output and Steam overlays: nothing for a player to see on this host | Empty handler | Original no-op status not established | [fes_runtime.rs:381](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L381) |
| `(crate::fes_program::PendingKind::Native, fileexist)` | fileexist | Handled | — | [fes_runtime.rs:990](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L990) |
| `(crate::fes_program::PendingKind::Native, getconf)` | getconf | Handled | — | [fes_runtime.rs:966](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L966) |
| `(crate::fes_program::PendingKind::Native, getsndconfex)` | getsndconfex | Handled | — | [fes_runtime.rs:949](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L949) |
| `(crate::fes_program::PendingKind::Native, setconf)` | setconf | Handled | — | [fes_runtime.rs:966](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L966) |
| `(crate::fes_program::PendingKind::Native, setsndconfex)` | setsndconfex | Handled | — | [fes_runtime.rs:949](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L949) |
| `(crate::fes_program::PendingKind::Native, steam_init)` | steam init | Handled | — | [fes_runtime.rs:1011](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L1011) |
| `(crate::fes_program::PendingKind::Native, steam_isenable)` | steam isenable | Handled | — | [fes_runtime.rs:1003](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L1003) |

## KCS v2 opcodes

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x00` | Execute instruction branch | Empty handler | Original no-op status not established | [kcs.rs:484](../../crates/crysirror-catsystem2/src/kcs.rs#L484) |
| `0x01` | Execute instruction branch | Handled | Guard: float | [kcs.rs:501](../../crates/crysirror-catsystem2/src/kcs.rs#L501) |
| `0x02` | Execute instruction branch | Handled | Guard: float | [kcs.rs:503](../../crates/crysirror-catsystem2/src/kcs.rs#L503) |
| `0x03` | Execute instruction branch | Handled | Guard: float | [kcs.rs:505](../../crates/crysirror-catsystem2/src/kcs.rs#L505) |
| `0x04` | Execute instruction branch | Handled | Guard: float | [kcs.rs:507](../../crates/crysirror-catsystem2/src/kcs.rs#L507) |
| `0x05` | Execute instruction branch | Handled | — | [kcs.rs:531](../../crates/crysirror-catsystem2/src/kcs.rs#L531) |
| `0x06` | Execute instruction branch | Handled | — | [kcs.rs:543](../../crates/crysirror-catsystem2/src/kcs.rs#L543) |
| `0x07` | Execute instruction branch | Handled | — | [kcs.rs:544](../../crates/crysirror-catsystem2/src/kcs.rs#L544) |
| `0x08` | Execute instruction branch | Handled | — | [kcs.rs:545](../../crates/crysirror-catsystem2/src/kcs.rs#L545) |
| `0x09` | Execute instruction branch | Handled | — | [kcs.rs:546](../../crates/crysirror-catsystem2/src/kcs.rs#L546) |
| `0x0A` | Execute instruction branch | Handled | — | [kcs.rs:547](../../crates/crysirror-catsystem2/src/kcs.rs#L547) |
| `0x0B` | Execute instruction branch | Handled | — | [kcs.rs:548](../../crates/crysirror-catsystem2/src/kcs.rs#L548) |
| `0x0C` | Execute instruction branch | Handled | — | [kcs.rs:549](../../crates/crysirror-catsystem2/src/kcs.rs#L549) |
| `0x0D` | Execute instruction branch | Handled | — | [kcs.rs:550](../../crates/crysirror-catsystem2/src/kcs.rs#L550) |
| `0x0E` | Execute instruction branch | Handled | — | [kcs.rs:551](../../crates/crysirror-catsystem2/src/kcs.rs#L551) |
| `0x0F` | Execute instruction branch | Handled | — | [kcs.rs:552](../../crates/crysirror-catsystem2/src/kcs.rs#L552) |
| `0x10` | Execute instruction branch | Handled | — | [kcs.rs:561](../../crates/crysirror-catsystem2/src/kcs.rs#L561) |
| `0x11` | Execute instruction branch | Handled | — | [kcs.rs:570](../../crates/crysirror-catsystem2/src/kcs.rs#L570) |
| `0x12` | Execute instruction branch | Handled | — | [kcs.rs:579](../../crates/crysirror-catsystem2/src/kcs.rs#L579) |
| `0x13` | Execute instruction branch | Handled | — | [kcs.rs:588](../../crates/crysirror-catsystem2/src/kcs.rs#L588) |
| `0x14` | Execute instruction branch | Handled | — | [kcs.rs:595](../../crates/crysirror-catsystem2/src/kcs.rs#L595) |
| `0x15` | Raw operand | Handled | — | [kcs.rs:606](../../crates/crysirror-catsystem2/src/kcs.rs#L606) |
| `0x16` | Raw operand | Partial | unsupported KCS increment width | [kcs.rs:621](../../crates/crysirror-catsystem2/src/kcs.rs#L621) |
| `0x17` | Raw operand | Partial | unsupported KCS increment width | [kcs.rs:621](../../crates/crysirror-catsystem2/src/kcs.rs#L621) |
| `0x18` | Raw operand | Partial | unsupported KCS increment width | [kcs.rs:621](../../crates/crysirror-catsystem2/src/kcs.rs#L621) |
| `0x19` | Raw operand | Partial | unsupported KCS increment width | [kcs.rs:621](../../crates/crysirror-catsystem2/src/kcs.rs#L621) |
| `0x1A` | Operand | Handled | — | [kcs.rs:648](../../crates/crysirror-catsystem2/src/kcs.rs#L648) |
| `0x1B` | Execute instruction branch | Handled | Guard: y == 0 | [kcs.rs:663](../../crates/crysirror-catsystem2/src/kcs.rs#L663) |
| `0x1C` | Execute instruction branch | Handled | — | [kcs.rs:669](../../crates/crysirror-catsystem2/src/kcs.rs#L669) |
| `0x1D` | Execute instruction branch | Handled | Guard: float | [kcs.rs:670](../../crates/crysirror-catsystem2/src/kcs.rs#L670) |
| `0x1E` | Execute instruction branch | Handled | Guard: float | [kcs.rs:672](../../crates/crysirror-catsystem2/src/kcs.rs#L672) |
| `0x1F` | Execute instruction branch | Handled | Guard: float | [kcs.rs:674](../../crates/crysirror-catsystem2/src/kcs.rs#L674) |
| `0x20` | Execute instruction branch | Handled | Guard: float | [kcs.rs:676](../../crates/crysirror-catsystem2/src/kcs.rs#L676) |
| `0x21` | Execute instruction branch | Handled | — | [kcs.rs:691](../../crates/crysirror-catsystem2/src/kcs.rs#L691) |
| `0x22` | Execute instruction branch | Handled | Guard: signed | [kcs.rs:692](../../crates/crysirror-catsystem2/src/kcs.rs#L692) |
| `0x23` | Execute instruction branch | Handled | — | [kcs.rs:694](../../crates/crysirror-catsystem2/src/kcs.rs#L694) |
| `0x24` | Execute instruction branch | Handled | — | [kcs.rs:695](../../crates/crysirror-catsystem2/src/kcs.rs#L695) |
| `0x25` | Pop | Handled | — | [kcs.rs:702](../../crates/crysirror-catsystem2/src/kcs.rs#L702) |
| `0x26` | Raw operand | Handled | — | [kcs.rs:724](../../crates/crysirror-catsystem2/src/kcs.rs#L724) |
| `0x27` | Operand | Handled | — | [kcs.rs:729](../../crates/crysirror-catsystem2/src/kcs.rs#L729) |
| `0x28` | Operand | Handled | — | [kcs.rs:729](../../crates/crysirror-catsystem2/src/kcs.rs#L729) |
| `0x29` | Operand | Handled | — | [kcs.rs:737](../../crates/crysirror-catsystem2/src/kcs.rs#L737) |
| `0x2A` | Operand | Handled | — | [kcs.rs:742](../../crates/crysirror-catsystem2/src/kcs.rs#L742) |
| `0x2B` | Native | Handled | — | [kcs.rs:756](../../crates/crysirror-catsystem2/src/kcs.rs#L756) |
| `0x2C` | Operand | Handled | — | [kcs.rs:781](../../crates/crysirror-catsystem2/src/kcs.rs#L781) |
| `0x2D` | Pop | Handled | — | [kcs.rs:809](../../crates/crysirror-catsystem2/src/kcs.rs#L809) |
| `0x2E` | Raw operand | Handled | — | [kcs.rs:812](../../crates/crysirror-catsystem2/src/kcs.rs#L812) |
| `0x2F` | Raw operand | Handled | — | [kcs.rs:816](../../crates/crysirror-catsystem2/src/kcs.rs#L816) |
| `0x30` | Raw operand | Handled | — | [kcs.rs:821](../../crates/crysirror-catsystem2/src/kcs.rs#L821) |
| `0x31` | Operand | Handled | — | [kcs.rs:835](../../crates/crysirror-catsystem2/src/kcs.rs#L835) |
| `0x32` | Operand | Handled | — | [kcs.rs:847](../../crates/crysirror-catsystem2/src/kcs.rs#L847) |
| `0x33` | Pop | Handled | — | [kcs.rs:855](../../crates/crysirror-catsystem2/src/kcs.rs#L855) |
| `0x34` | Operand | Handled | — | [kcs.rs:858](../../crates/crysirror-catsystem2/src/kcs.rs#L858) |
| `0x35` | Operand | Handled | — | [kcs.rs:865](../../crates/crysirror-catsystem2/src/kcs.rs#L865) |
| `0x36` | Execute instruction branch | Handled | — | [kcs.rs:876](../../crates/crysirror-catsystem2/src/kcs.rs#L876) |
| `0x37` | Execute instruction branch | Handled | — | [kcs.rs:877](../../crates/crysirror-catsystem2/src/kcs.rs#L877) |
| `0x38` | Execute instruction branch | Handled | — | [kcs.rs:878](../../crates/crysirror-catsystem2/src/kcs.rs#L878) |
| `0x39` | Operand | Handled | — | [kcs.rs:873](../../crates/crysirror-catsystem2/src/kcs.rs#L873) |
| `0x3A` | Pop | Handled | — | [kcs.rs:883](../../crates/crysirror-catsystem2/src/kcs.rs#L883) |
| `0x3B` | Halted | Handled | — | [kcs.rs:888](../../crates/crysirror-catsystem2/src/kcs.rs#L888) |
| `0x3C` | Update stack | Handled | — | [kcs.rs:893](../../crates/crysirror-catsystem2/src/kcs.rs#L893) |
| `0x3D` | Operand | Handled | — | [kcs.rs:894](../../crates/crysirror-catsystem2/src/kcs.rs#L894) |
| `0x3E` | Raw operand | Handled | — | [kcs.rs:898](../../crates/crysirror-catsystem2/src/kcs.rs#L898) |

## Native gcalls

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x00` | Native call (service_native) | Handled | — | [motion.rs:432](../../crates/crysirror-catsystem2/src/motion.rs#L432) |
| `0x01` | Execute run script branch | Partial | CatSystem2 unsupported consecutive-percent debug format | [adapter.rs:672](../../crates/crysirror-catsystem2/src/adapter.rs#L672) |
| `0x04` | Execute run script branch | Handled | — | [adapter.rs:2946](../../crates/crysirror-catsystem2/src/adapter.rs#L2946) |
| `0x05` | Execute run script branch | Handled | — | [adapter.rs:2946](../../crates/crysirror-catsystem2/src/adapter.rs#L2946) |
| `0x08` | Execute run script branch | Handled | — | [adapter.rs:2817](../../crates/crysirror-catsystem2/src/adapter.rs#L2817) |
| `0x0D` | Execute run script branch | Partial | unsupported overlapping native CatSystem2 string operation | [adapter.rs:2826](../../crates/crysirror-catsystem2/src/adapter.rs#L2826) |
| `0x0E` | Execute run script branch | Partial | unsupported overlapping native CatSystem2 string operation | [adapter.rs:2826](../../crates/crysirror-catsystem2/src/adapter.rs#L2826) |
| `0x0F` | Execute run script branch | Partial | unsupported overlapping native CatSystem2 string operation | [adapter.rs:2826](../../crates/crysirror-catsystem2/src/adapter.rs#L2826) |
| `0x10` | Execute run script branch | Partial | unsupported overlapping native CatSystem2 string operation | [adapter.rs:2826](../../crates/crysirror-catsystem2/src/adapter.rs#L2826) |
| `0x11` | Execute run script branch | Handled | — | [adapter.rs:2845](../../crates/crysirror-catsystem2/src/adapter.rs#L2845) |
| `0x12` | Execute run script branch | Handled | — | [adapter.rs:2845](../../crates/crysirror-catsystem2/src/adapter.rs#L2845) |
| `0x13` | Execute run script branch | Handled | — | [adapter.rs:2003](../../crates/crysirror-catsystem2/src/adapter.rs#L2003) |
| `0x14` | Execute run script branch | Handled | — | [adapter.rs:2003](../../crates/crysirror-catsystem2/src/adapter.rs#L2003) |
| `0x15` | Execute run script branch | Handled | — | [adapter.rs:2003](../../crates/crysirror-catsystem2/src/adapter.rs#L2003) |
| `0x16` | Execute run script branch | Handled | — | [adapter.rs:2003](../../crates/crysirror-catsystem2/src/adapter.rs#L2003) |
| `0x17` | Arithmetic (prepare) | Handled | — | [motion_math.rs:20](../../crates/crysirror-catsystem2/src/motion_math.rs#L20) |
| `0x18` | Arithmetic (prepare) | Handled | — | [motion_math.rs:20](../../crates/crysirror-catsystem2/src/motion_math.rs#L20) |
| `0x19` | Execute run script branch | Handled | — | [adapter.rs:2963](../../crates/crysirror-catsystem2/src/adapter.rs#L2963) |
| `0x1A` | Arithmetic (prepare) | Handled | — | [motion_math.rs:22](../../crates/crysirror-catsystem2/src/motion_math.rs#L22) |
| `0x1B` | Arithmetic (prepare) | Handled | — | [motion_math.rs:22](../../crates/crysirror-catsystem2/src/motion_math.rs#L22) |
| `0x1C` | Arithmetic (prepare) | Handled | — | [motion_math.rs:23](../../crates/crysirror-catsystem2/src/motion_math.rs#L23) |
| `0x1D` | /// over shared storage, read-only: the module only opens game data here, and a missing file answers null | Handled | — | [adapter.rs:2508](../../crates/crysirror-catsystem2/src/adapter.rs#L2508) |
| `0x1E` | Execute run script branch | Handled | — | [adapter.rs:2520](../../crates/crysirror-catsystem2/src/adapter.rs#L2520) |
| `0x1F` | Arithmetic (prepare) | Handled | — | [motion_math.rs:22](../../crates/crysirror-catsystem2/src/motion_math.rs#L22) |
| `0x20` | Execute run script branch | Handled | — | [adapter.rs:2534](../../crates/crysirror-catsystem2/src/adapter.rs#L2534) |
| `0x21` | Execute run script branch | Handled | — | [adapter.rs:2539](../../crates/crysirror-catsystem2/src/adapter.rs#L2539) |
| `0x22` | Arithmetic (prepare) | Handled | — | [motion_math.rs:22](../../crates/crysirror-catsystem2/src/motion_math.rs#L22) |
| `0x23` | Execute run script branch | Handled | — | [adapter.rs:2953](../../crates/crysirror-catsystem2/src/adapter.rs#L2953) |
| `0x24` | Execute run script branch | Handled | — | [adapter.rs:2010](../../crates/crysirror-catsystem2/src/adapter.rs#L2010) |
| `0x25` | Arithmetic (prepare) | Handled | — | [motion_math.rs:23](../../crates/crysirror-catsystem2/src/motion_math.rs#L23) |
| `0x26` | Execute run script branch | Handled | — | [adapter.rs:2963](../../crates/crysirror-catsystem2/src/adapter.rs#L2963) |
| `0x27` | Native call (service_native) | Handled | — | [motion.rs:417](../../crates/crysirror-catsystem2/src/motion.rs#L417) |
| `0x28` | Native call (service_native) | Handled | — | [motion.rs:417](../../crates/crysirror-catsystem2/src/motion.rs#L417) |
| `0x29` | Update split loading | Handled | — | [adapter.rs:2020](../../crates/crysirror-catsystem2/src/adapter.rs#L2020) |
| `0x2A` | Execute run script branch | Handled | — | [adapter.rs:2036](../../crates/crysirror-catsystem2/src/adapter.rs#L2036) |
| `0x2B` | Execute run script branch | Handled | — | [adapter.rs:2963](../../crates/crysirror-catsystem2/src/adapter.rs#L2963) |
| `0x2C` | Execute run script branch | Handled | — | [adapter.rs:2963](../../crates/crysirror-catsystem2/src/adapter.rs#L2963) |
| `0x2D` | Execute run script branch | Handled | — | [adapter.rs:2963](../../crates/crysirror-catsystem2/src/adapter.rs#L2963) |
| `0x2E` | Execute run script branch | Handled | — | [adapter.rs:2963](../../crates/crysirror-catsystem2/src/adapter.rs#L2963) |
| `0x2F` | Execute run script branch | Handled | — | [adapter.rs:2963](../../crates/crysirror-catsystem2/src/adapter.rs#L2963) |
| `0x31` | Execute run script branch | Handled | — | [adapter.rs:2963](../../crates/crysirror-catsystem2/src/adapter.rs#L2963) |
| `0x42` | Update tasks | Partial | CatSystem2 unsupported native gcall 66 task class {} at PC {:#x}; original call remains unacknowledged | [adapter.rs:2042](../../crates/crysirror-catsystem2/src/adapter.rs#L2042) |
| `0x43` | Update tasks, interpreter | Handled | — | [adapter.rs:2096](../../crates/crysirror-catsystem2/src/adapter.rs#L2096) |
| `0x44` | Execute run script branch | Handled | — | [adapter.rs:2813](../../crates/crysirror-catsystem2/src/adapter.rs#L2813) |
| `0x45` | > 12 case 172 -> : a message into a script parked in its native71. The caller waits until the target has taken it and r… | Handled | — | [adapter.rs:1714](../../crates/crysirror-catsystem2/src/adapter.rs#L1714) |
| `0x46` | > 12 case 173 -> : the same message, queued without waiting | Handled | — | [adapter.rs:1732](../../crates/crysirror-catsystem2/src/adapter.rs#L1732) |
| `0x47` | Update event wait | Handled | — | [adapter.rs:2278](../../crates/crysirror-catsystem2/src/adapter.rs#L2278) |
| `0x48` | /: bytes between two scripts' memory; all tasks share one heap | Handled | — | [adapter.rs:1741](../../crates/crysirror-catsystem2/src/adapter.rs#L1741) |
| `0x49` | /: bytes between two scripts' memory; all tasks share one heap | Handled | — | [adapter.rs:1741](../../crates/crysirror-catsystem2/src/adapter.rs#L1741) |
| `0x4D` | Execute run script branch | Handled | — | [adapter.rs:2184](../../crates/crysirror-catsystem2/src/adapter.rs#L2184) |
| `0x4E` | Execute run script branch | Handled | — | [adapter.rs:2188](../../crates/crysirror-catsystem2/src/adapter.rs#L2188) |
| `0x4F` | Execute run script branch | Partial | CatSystem2 unsupported native solid plane extent / CatSystem2 unsupported native solid replacement of an image plane | [adapter.rs:2128](../../crates/crysirror-catsystem2/src/adapter.rs#L2128) |
| `0x50` | Execute run script branch | Handled | — | [adapter.rs:2214](../../crates/crysirror-catsystem2/src/adapter.rs#L2214) |
| `0x53` | Execute run script branch | Handled | — | [adapter.rs:2214](../../crates/crysirror-catsystem2/src/adapter.rs#L2214) |
| `0x54` | Execute run script branch | Handled | — | [adapter.rs:2224](../../crates/crysirror-catsystem2/src/adapter.rs#L2224) |
| `0x55` | Execute run script branch | Handled | — | [adapter.rs:2224](../../crates/crysirror-catsystem2/src/adapter.rs#L2224) |
| `0x56` | Execute run script branch | Handled | — | [adapter.rs:2250](../../crates/crysirror-catsystem2/src/adapter.rs#L2250) |
| `0x57` | Execute run script branch | Handled | — | [adapter.rs:2250](../../crates/crysirror-catsystem2/src/adapter.rs#L2250) |
| `0x58` | Execute run script branch | Handled | — | [adapter.rs:2233](../../crates/crysirror-catsystem2/src/adapter.rs#L2233) |
| `0x5C` | Update tasks | Handled | — | [adapter.rs:2266](../../crates/crysirror-catsystem2/src/adapter.rs#L2266) |
| `0x5D` | Execute run script branch | Handled | — | [adapter.rs:2224](../../crates/crysirror-catsystem2/src/adapter.rs#L2224) |
| `0x5E` | Execute run script branch | Handled | — | [adapter.rs:2220](../../crates/crysirror-catsystem2/src/adapter.rs#L2220) |
| `0x64` | Update tasks | Handled | — | [adapter.rs:2171](../../crates/crysirror-catsystem2/src/adapter.rs#L2171) |
| `0x65` | Execute run script branch | Handled | — | [adapter.rs:2224](../../crates/crysirror-catsystem2/src/adapter.rs#L2224) |
| `0x68` | Execute run script branch | Handled | — | [adapter.rs:647](../../crates/crysirror-catsystem2/src/adapter.rs#L647) |
| `0x69` | Execute run script branch | Handled | — | [adapter.rs:647](../../crates/crysirror-catsystem2/src/adapter.rs#L647) |
| `0x6C` | Execute run script branch | Handled | — | [adapter.rs:647](../../crates/crysirror-catsystem2/src/adapter.rs#L647) |
| `0x6F` | Update layered loading | Handled | — | [adapter.rs:918](../../crates/crysirror-catsystem2/src/adapter.rs#L918) |
| `0x70` | a transparent bitmap; the current screen | Handled | — | [adapter.rs:1223](../../crates/crysirror-catsystem2/src/adapter.rs#L1223) |
| `0x71` | Update surface / job | Handled | — | [adapter.rs:1230](../../crates/crysirror-catsystem2/src/adapter.rs#L1230) |
| `0x72` | Execute run script branch | Handled | — | [adapter.rs:1277](../../crates/crysirror-catsystem2/src/adapter.rs#L1277) |
| `0x73` | Update surface / owner, surface / geometry, surface / job | Handled | — | [adapter.rs:1120](../../crates/crysirror-catsystem2/src/adapter.rs#L1120) |
| `0x74` | Update surface / owner, surface / geometry, surface / job | Handled | — | [adapter.rs:1120](../../crates/crysirror-catsystem2/src/adapter.rs#L1120) |
| `0x75` | Update texture clear, texture cache | Partial | CatSystem2 unsupported native117 bitmap target/profile | [adapter.rs:2338](../../crates/crysirror-catsystem2/src/adapter.rs#L2338) |
| `0x76` | Execute run script branch | Handled | — | [adapter.rs:1241](../../crates/crysirror-catsystem2/src/adapter.rs#L1241) |
| `0x77` | , : a bitmap's size | Handled | — | [adapter.rs:1851](../../crates/crysirror-catsystem2/src/adapter.rs#L1851) |
| `0x8F` | Execute run script branch | Handled | — | [adapter.rs:877](../../crates/crysirror-catsystem2/src/adapter.rs#L877) |
| `0x9C` | Execute run script branch | Handled | — | [adapter.rs:1585](../../crates/crysirror-catsystem2/src/adapter.rs#L1585) |
| `0x9D` | Execute run script branch | Handled | — | [adapter.rs:1594](../../crates/crysirror-catsystem2/src/adapter.rs#L1594) |
| `0x9E` | Execute run script branch | Handled | — | [adapter.rs:1594](../../crates/crysirror-catsystem2/src/adapter.rs#L1594) |
| `0x9F` | Execute run script branch | Handled | — | [adapter.rs:1594](../../crates/crysirror-catsystem2/src/adapter.rs#L1594) |
| `0xA0` | Execute run script branch | Handled | — | [adapter.rs:1602](../../crates/crysirror-catsystem2/src/adapter.rs#L1602) |
| `0xA1` | Execute run script branch | Handled | — | [adapter.rs:1608](../../crates/crysirror-catsystem2/src/adapter.rs#L1608) |
| `0xA3` | Execute run script branch | Handled | — | [adapter.rs:1608](../../crates/crysirror-catsystem2/src/adapter.rs#L1608) |
| `0xCC` | , , : matrices in VM memory; a null address is the original's scratch | Handled | — | [adapter.rs:1764](../../crates/crysirror-catsystem2/src/adapter.rs#L1764) |
| `0xCE` | , , : matrices in VM memory; a null address is the original's scratch | Handled | — | [adapter.rs:1764](../../crates/crysirror-catsystem2/src/adapter.rs#L1764) |
| `0xCF` | , , : matrices in VM memory; a null address is the original's scratch | Handled | — | [adapter.rs:1764](../../crates/crysirror-catsystem2/src/adapter.rs#L1764) |
| `0xD1` | Update interpreter | Handled | — | [adapter.rs:2643](../../crates/crysirror-catsystem2/src/adapter.rs#L2643) |
| `0xD2` | Execute run script branch | Handled | — | [adapter.rs:2651](../../crates/crysirror-catsystem2/src/adapter.rs#L2651) |
| `0xD3` | Execute run script branch | Handled | — | [adapter.rs:2655](../../crates/crysirror-catsystem2/src/adapter.rs#L2655) |
| `0xD4` | Execute run script branch | Handled | — | [adapter.rs:2655](../../crates/crysirror-catsystem2/src/adapter.rs#L2655) |
| `0xD5` | Execute run script branch | Handled | — | [adapter.rs:2664](../../crates/crysirror-catsystem2/src/adapter.rs#L2664) |
| `0xF3` | the string bank (VM+180, ) as one record of container a0 under key a1 | Handled | — | [adapter.rs:2595](../../crates/crysirror-catsystem2/src/adapter.rs#L2595) |
| `0xF5` | Update save writing | Partial | CatSystem2 unsupported save filename encoding | [adapter.rs:2561](../../crates/crysirror-catsystem2/src/adapter.rs#L2561) |
| `0xF6` | Destructors of objects never created along this path | Handled | Guard: call.arguments.first()==Some(&0) | [adapter.rs:2941](../../crates/crysirror-catsystem2/src/adapter.rs#L2941) |
| `0xF7` | Update save loading, next save | Partial | CatSystem2 unsupported save filename encoding | [adapter.rs:2489](../../crates/crysirror-catsystem2/src/adapter.rs#L2489) |
| `0xFD` | the first record under key a1 replaces the whole bank () | Handled | — | [adapter.rs:2608](../../crates/crysirror-catsystem2/src/adapter.rs#L2608) |
| `0x100` | Update image pool loading, tasks | Partial | CatSystem2 unsupported image-list reinitialization | [adapter.rs:2350](../../crates/crysirror-catsystem2/src/adapter.rs#L2350) |
| `0x101` | Execute run script branch | Handled | — | [adapter.rs:1091](../../crates/crysirror-catsystem2/src/adapter.rs#L1091) |
| `0x102` | Execute run script branch | Handled | — | [adapter.rs:1401](../../crates/crysirror-catsystem2/src/adapter.rs#L1401) |
| `0x104` | Execute run script branch | Handled | — | [adapter.rs:648](../../crates/crysirror-catsystem2/src/adapter.rs#L648) |
| `0x105` | Messages 285..313 on an image list (). A task of another class has no case | Handled | Guard: self.tasks.image_list(call.arguments[0]).is_err() | [adapter.rs:1333](../../crates/crysirror-catsystem2/src/adapter.rs#L1333) |
| `0x106` | Execute run script branch | Handled | — | [adapter.rs:648](../../crates/crysirror-catsystem2/src/adapter.rs#L648) |
| `0x107` | Messages 285..313 on an image list (). A task of another class has no case | Handled | Guard: self.tasks.image_list(call.arguments[0]).is_err() | [adapter.rs:1333](../../crates/crysirror-catsystem2/src/adapter.rs#L1333) |
| `0x109` | Execute run script branch | Handled | — | [adapter.rs:1676](../../crates/crysirror-catsystem2/src/adapter.rs#L1676) |
| `0x10A` | Execute run script branch | Handled | — | [adapter.rs:1691](../../crates/crysirror-catsystem2/src/adapter.rs#L1691) |
| `0x10B` | Execute run script branch | Handled | — | [adapter.rs:1693](../../crates/crysirror-catsystem2/src/adapter.rs#L1693) |
| `0x10C` | Execute run script branch | Handled | — | [adapter.rs:1697](../../crates/crysirror-catsystem2/src/adapter.rs#L1697) |
| `0x10D` | Execute run script branch | Handled | — | [adapter.rs:1698](../../crates/crysirror-catsystem2/src/adapter.rs#L1698) |
| `0x10E` | pumps the original's decoder graph; the shared service keeps its own clock | Handled | — | [adapter.rs:1701](../../crates/crysirror-catsystem2/src/adapter.rs#L1701) |
| `0x114` | Execute run script branch | Handled | — | [adapter.rs:2615](../../crates/crysirror-catsystem2/src/adapter.rs#L2615) |
| `0x115` | Execute run script branch | Handled | — | [adapter.rs:2622](../../crates/crysirror-catsystem2/src/adapter.rs#L2622) |
| `0x116` | Execute run script branch | Handled | — | [adapter.rs:870](../../crates/crysirror-catsystem2/src/adapter.rs#L870) |
| `0x11C` | whether the named sound can be opened | Handled | — | [adapter.rs:1840](../../crates/crysirror-catsystem2/src/adapter.rs#L1840) |
| `0x11D` | Execute run script branch | Handled | — | [adapter.rs:1433](../../crates/crysirror-catsystem2/src/adapter.rs#L1433) |
| `0x126` | Execute run script branch | Handled | — | [adapter.rs:1441](../../crates/crysirror-catsystem2/src/adapter.rs#L1441) |
| `0x127` | Execute run script branch | Handled | — | [adapter.rs:1447](../../crates/crysirror-catsystem2/src/adapter.rs#L1447) |
| `0x128` | Execute run script branch | Handled | — | [adapter.rs:1456](../../crates/crysirror-catsystem2/src/adapter.rs#L1456) |
| `0x12A` | Execute run script branch | Handled | — | [adapter.rs:1457](../../crates/crysirror-catsystem2/src/adapter.rs#L1457) |
| `0x12B` | Execute run script branch | Handled | — | [adapter.rs:1464](../../crates/crysirror-catsystem2/src/adapter.rs#L1464) |
| `0x12D` | Execute run script branch | Handled | — | [adapter.rs:1464](../../crates/crysirror-catsystem2/src/adapter.rs#L1464) |
| `0x12E` | Execute run script branch | Handled | — | [adapter.rs:1475](../../crates/crysirror-catsystem2/src/adapter.rs#L1475) |
| `0x130` | Update motion loading | Handled | — | [adapter.rs:1483](../../crates/crysirror-catsystem2/src/adapter.rs#L1483) |
| `0x133` | Execute run script branch | Handled | — | [adapter.rs:1516](../../crates/crysirror-catsystem2/src/adapter.rs#L1516) |
| `0x134` | Execute run script branch | Handled | — | [adapter.rs:1529](../../crates/crysirror-catsystem2/src/adapter.rs#L1529) |
| `0x136` | Execute run script branch | Handled | — | [adapter.rs:1534](../../crates/crysirror-catsystem2/src/adapter.rs#L1534) |
| `0x137` | Execute run script branch | Handled | — | [adapter.rs:1534](../../crates/crysirror-catsystem2/src/adapter.rs#L1534) |
| `0x139` | Execute run script branch | Handled | — | [adapter.rs:1458](../../crates/crysirror-catsystem2/src/adapter.rs#L1458) |
| `0x13A` | Execute run script branch | Handled | — | [adapter.rs:1452](../../crates/crysirror-catsystem2/src/adapter.rs#L1452) |
| `0x13E` | Execute run script branch | Handled | — | [adapter.rs:694](../../crates/crysirror-catsystem2/src/adapter.rs#L694) |
| `0x13F` | Execute run script branch | Handled | — | [adapter.rs:702](../../crates/crysirror-catsystem2/src/adapter.rs#L702) |
| `0x143` | Execute run script branch | Handled | — | [adapter.rs:708](../../crates/crysirror-catsystem2/src/adapter.rs#L708) |
| `0x144` | Execute run script branch | Handled | — | [adapter.rs:715](../../crates/crysirror-catsystem2/src/adapter.rs#L715) |
| `0x145` | Update scene loading | Handled | — | [adapter.rs:720](../../crates/crysirror-catsystem2/src/adapter.rs#L720) |
| `0x146` | Execute run script branch | Handled | — | [adapter.rs:751](../../crates/crysirror-catsystem2/src/adapter.rs#L751) |
| `0x147` | Execute run script branch | Handled | — | [adapter.rs:757](../../crates/crysirror-catsystem2/src/adapter.rs#L757) |
| `0x148` | Execute run script branch | Handled | — | [adapter.rs:779](../../crates/crysirror-catsystem2/src/adapter.rs#L779) |
| `0x149` | Execute run script branch | Handled | — | [adapter.rs:780](../../crates/crysirror-catsystem2/src/adapter.rs#L780) |
| `0x14A` | empties the first list, the second | Handled | — | [adapter.rs:808](../../crates/crysirror-catsystem2/src/adapter.rs#L808) |
| `0x14B` | empties the first list, the second | Handled | — | [adapter.rs:808](../../crates/crysirror-catsystem2/src/adapter.rs#L808) |
| `0x14C` | Execute run script branch | Handled | — | [adapter.rs:781](../../crates/crysirror-catsystem2/src/adapter.rs#L781) |
| `0x14D` | Execute run script branch | Handled | — | [adapter.rs:813](../../crates/crysirror-catsystem2/src/adapter.rs#L813) |
| `0x150` | Execute run script branch | Handled | — | [adapter.rs:790](../../crates/crysirror-catsystem2/src/adapter.rs#L790) |
| `0x152` | Execute run script branch | Handled | — | [adapter.rs:765](../../crates/crysirror-catsystem2/src/adapter.rs#L765) |
| `0x153` | > : one draw record's clip; an empty rectangle removes it | Handled | — | [adapter.rs:1917](../../crates/crysirror-catsystem2/src/adapter.rs#L1917) |
| `0x154` | Messages 285..313 on an image list (). A task of another class has no case | Handled | Guard: self.tasks.image_list(call.arguments[0]).is_err() | [adapter.rs:1333](../../crates/crysirror-catsystem2/src/adapter.rs#L1333) |
| `0x155` | Messages 285..313 on an image list (). A task of another class has no case | Handled | Guard: self.tasks.image_list(call.arguments[0]).is_err() | [adapter.rs:1333](../../crates/crysirror-catsystem2/src/adapter.rs#L1333) |
| `0x157` | Execute run script branch | Handled | — | [adapter.rs:2255](../../crates/crysirror-catsystem2/src/adapter.rs#L2255) |
| `0x159` | Execute run script branch | Handled | — | [adapter.rs:2247](../../crates/crysirror-catsystem2/src/adapter.rs#L2247) |
| `0x166` | > : D3DXVec4Transform in place, row vector times matrix | Handled | — | [adapter.rs:1782](../../crates/crysirror-catsystem2/src/adapter.rs#L1782) |
| `0x167` | Update settings loading, string variables / language prefix, settings | Handled | — | [adapter.rs:2674](../../crates/crysirror-catsystem2/src/adapter.rs#L2674) |
| `0x16A` | Execute run script branch | Handled | — | [adapter.rs:1287](../../crates/crysirror-catsystem2/src/adapter.rs#L1287) |
| `0x16B` | Execute run script branch | Handled | — | [adapter.rs:1287](../../crates/crysirror-catsystem2/src/adapter.rs#L1287) |
| `0x16C` | → /: decode frame a2 of a block into a new image (VM+152). → : frame a1's 36-byte stdinfo into a VM buffer ( layout) | Handled | — | [adapter.rs:1012](../../crates/crysirror-catsystem2/src/adapter.rs#L1012) |
| `0x16D` | Update blob pending | Handled | — | [adapter.rs:977](../../crates/crysirror-catsystem2/src/adapter.rs#L977) |
| `0x16E` | release one reference; the block goes at zero (its flag is never set) | Handled | — | [adapter.rs:1000](../../crates/crysirror-catsystem2/src/adapter.rs#L1000) |
| `0x16F` | Update surface / owner, surface / geometry, surface / job | Handled | — | [adapter.rs:1120](../../crates/crysirror-catsystem2/src/adapter.rs#L1120) |
| `0x170` | → /: decode frame a2 of a block into a new image (VM+152). → : frame a1's 36-byte stdinfo into a VM buffer ( layout) | Handled | — | [adapter.rs:1012](../../crates/crysirror-catsystem2/src/adapter.rs#L1012) |
| `0x171` | → : a 32-bit image's four stored bytes times ARGB a1 / 255 | Handled | — | [adapter.rs:1041](../../crates/crysirror-catsystem2/src/adapter.rs#L1041) |
| `0x174` | Execute run script branch | Handled | — | [adapter.rs:1309](../../crates/crysirror-catsystem2/src/adapter.rs#L1309) |
| `0x175` | Execute run script branch | Handled | — | [adapter.rs:1310](../../crates/crysirror-catsystem2/src/adapter.rs#L1310) |
| `0x188` | Execute run script branch | Handled | — | [adapter.rs:908](../../crates/crysirror-catsystem2/src/adapter.rs#L908) |
| `0x189` | Execute run script branch | Handled | — | [adapter.rs:2850](../../crates/crysirror-catsystem2/src/adapter.rs#L2850) |
| `0x18D` | Execute run script branch | Handled | — | [adapter.rs:1789](../../crates/crysirror-catsystem2/src/adapter.rs#L1789) |
| `0x190` | Update loop loading, loop catalog revision, loop entries | Handled | — | [adapter.rs:2693](../../crates/crysirror-catsystem2/src/adapter.rs#L2693) |
| `0x1A1` | Execute run script branch | Handled | — | [adapter.rs:1702](../../crates/crysirror-catsystem2/src/adapter.rs#L1702) |
| `0x1B2` | , , : the original's archive read-ahead; the engine has its own | Handled | — | [adapter.rs:1835](../../crates/crysirror-catsystem2/src/adapter.rs#L1835) |
| `0x1B3` | Execute run script branch | Handled | — | [adapter.rs:1623](../../crates/crysirror-catsystem2/src/adapter.rs#L1623) |
| `0x1B4` | , , : the original's archive read-ahead; the engine has its own | Handled | — | [adapter.rs:1835](../../crates/crysirror-catsystem2/src/adapter.rs#L1835) |
| `0x1B5` | Execute run script branch | Handled | — | [adapter.rs:1617](../../crates/crysirror-catsystem2/src/adapter.rs#L1617) |
| `0x1B6` | , , : the original's archive read-ahead; the engine has its own | Handled | — | [adapter.rs:1835](../../crates/crysirror-catsystem2/src/adapter.rs#L1835) |
| `0x1B8` | Execute run script branch | Handled | — | [adapter.rs:1516](../../crates/crysirror-catsystem2/src/adapter.rs#L1516) |
| `0x1BF` | Update fes loading, tasks | Partial | CatSystem2 unsupported FES player reinitialization / CatSystem2 unsupported FES filename encoding | [adapter.rs:2447](../../crates/crysirror-catsystem2/src/adapter.rs#L2447) |
| `0x1C0` | Update fes tick input, input routes bound | Handled | — | [adapter.rs:2387](../../crates/crysirror-catsystem2/src/adapter.rs#L2387) |
| `0x1C1` | Execute run script branch | Handled | — | [adapter.rs:2483](../../crates/crysirror-catsystem2/src/adapter.rs#L2483) |
| `0x1C2` | Update tasks | Handled | — | [adapter.rs:2418](../../crates/crysirror-catsystem2/src/adapter.rs#L2418) |
| `0x1C3` | Update interpreter | Handled | — | [adapter.rs:2427](../../crates/crysirror-catsystem2/src/adapter.rs#L2427) |
| `0x1C4` | Execute run script branch | Handled | — | [adapter.rs:2435](../../crates/crysirror-catsystem2/src/adapter.rs#L2435) |
| `0x1C5` | Execute run script branch | Handled | — | [adapter.rs:2441](../../crates/crysirror-catsystem2/src/adapter.rs#L2441) |
| `0x1C6` | this task's argument string (+712, message 26 from ); the tasks here take none, so the buffer gets an empty one | Handled | — | [adapter.rs:1838](../../crates/crysirror-catsystem2/src/adapter.rs#L1838) |
| `0x1C7` | Messages 285..313 on an image list (). A task of another class has no case | Handled | Guard: self.tasks.image_list(call.arguments[0]).is_err() | [adapter.rs:1333](../../crates/crysirror-catsystem2/src/adapter.rs#L1333) |
| `0x1D1` | Execute run script branch | Handled | — | [adapter.rs:2858](../../crates/crysirror-catsystem2/src/adapter.rs#L2858) |
| `0x1D2` | Execute run script branch | Handled | — | [adapter.rs:2261](../../crates/crysirror-catsystem2/src/adapter.rs#L2261) |
| `0x203` | Execute run script branch | Handled | — | [adapter.rs:2862](../../crates/crysirror-catsystem2/src/adapter.rs#L2862) |
| `0x204` | Execute run script branch | Handled | — | [adapter.rs:2867](../../crates/crysirror-catsystem2/src/adapter.rs#L2867) |
| `0x205` | Execute run script branch | Handled | — | [adapter.rs:2872](../../crates/crysirror-catsystem2/src/adapter.rs#L2872) |
| `0x206` | Execute run script branch | Handled | — | [adapter.rs:2880](../../crates/crysirror-catsystem2/src/adapter.rs#L2880) |
| `0x207` | Execute run script branch | Handled | — | [adapter.rs:2963](../../crates/crysirror-catsystem2/src/adapter.rs#L2963) |
| `0x217` | Update motion loading | Handled | — | [adapter.rs:1483](../../crates/crysirror-catsystem2/src/adapter.rs#L1483) |
| `0x218` | / -> : five raw ats words from the shared HG3 metadata | Handled | — | [adapter.rs:1858](../../crates/crysirror-catsystem2/src/adapter.rs#L1858) |
| `0x21A` | Execute run script branch | Handled | — | [adapter.rs:1208](../../crates/crysirror-catsystem2/src/adapter.rs#L1208) |
| `0x21B` | a bracket list into a word array; the count answers | Handled | — | [adapter.rs:1795](../../crates/crysirror-catsystem2/src/adapter.rs#L1795) |
| `0x21C` | Execute run script branch | Handled | — | [adapter.rs:2963](../../crates/crysirror-catsystem2/src/adapter.rs#L2963) |
| `0x23B` | Execute run script branch | Handled | — | [adapter.rs:2939](../../crates/crysirror-catsystem2/src/adapter.rs#L2939) |
| `0x23C` | Execute run script branch | Handled | — | [adapter.rs:2939](../../crates/crysirror-catsystem2/src/adapter.rs#L2939) |
| `0x24A` | Execute run script branch | Handled | — | [adapter.rs:1542](../../crates/crysirror-catsystem2/src/adapter.rs#L1542) |
| `0x24E` | Update xml loading, document bytes | Handled | — | [adapter.rs:2738](../../crates/crysirror-catsystem2/src/adapter.rs#L2738) |
| `0x24F` | Update document bytes | Handled | — | [adapter.rs:2788](../../crates/crysirror-catsystem2/src/adapter.rs#L2788) |
| `0x250` | Execute run script branch | Handled | — | [adapter.rs:2796](../../crates/crysirror-catsystem2/src/adapter.rs#L2796) |
| `0x251` | Execute run script branch | Handled | — | [adapter.rs:2796](../../crates/crysirror-catsystem2/src/adapter.rs#L2796) |
| `0x25C` | Execute run script branch | Handled | — | [adapter.rs:1573](../../crates/crysirror-catsystem2/src/adapter.rs#L1573) |
| `0x25D` | Execute run script branch | Handled | — | [adapter.rs:1567](../../crates/crysirror-catsystem2/src/adapter.rs#L1567) |
| `0x25E` | Execute run script branch | Handled | — | [adapter.rs:1547](../../crates/crysirror-catsystem2/src/adapter.rs#L1547) |
| `0x25F` | Execute run script branch | Handled | — | [adapter.rs:1547](../../crates/crysirror-catsystem2/src/adapter.rs#L1547) |
| `0x260` | > (4): input count, -1 before a scene loaded | Handled | — | [adapter.rs:886](../../crates/crysirror-catsystem2/src/adapter.rs#L886) |
| `0x276` | Update fes tick input | Handled | — | [adapter.rs:1962](../../crates/crysirror-catsystem2/src/adapter.rs#L1962) |
| `0x285` | Execute run script branch | Handled | — | [adapter.rs:1940](../../crates/crysirror-catsystem2/src/adapter.rs#L1940) |
| `0x286` | Execute run script branch | Handled | — | [adapter.rs:1949](../../crates/crysirror-catsystem2/src/adapter.rs#L1949) |
| `0x287` | Execute run script branch | Handled | — | [adapter.rs:1955](../../crates/crysirror-catsystem2/src/adapter.rs#L1955) |
| `0x2A2` | Execute run script branch | Handled | — | [adapter.rs:1568](../../crates/crysirror-catsystem2/src/adapter.rs#L1568) |
| `0x2B8` | and family: the message log the backlog reads | Handled | — | [adapter.rs:1868](../../crates/crysirror-catsystem2/src/adapter.rs#L1868) |
| `0x2B9` | Execute run script branch | Handled | — | [adapter.rs:1869](../../crates/crysirror-catsystem2/src/adapter.rs#L1869) |
| `0x2BA` | / -> : live message insertion; 698 supplies history_index=-1, while 977 passes it explicitly | Handled | — | [adapter.rs:1874](../../crates/crysirror-catsystem2/src/adapter.rs#L1874) |
| `0x2BC` | > : the row's speaker, which the backlog turns into its name image | Handled | — | [adapter.rs:1898](../../crates/crysirror-catsystem2/src/adapter.rs#L1898) |
| `0x2C6` | Execute run script branch | Handled | — | [adapter.rs:1068](../../crates/crysirror-catsystem2/src/adapter.rs#L1068) |
| `0x2C7` | Execute run script branch | Handled | — | [adapter.rs:1277](../../crates/crysirror-catsystem2/src/adapter.rs#L1277) |
| `0x2C8` | , : a bitmap's size | Handled | — | [adapter.rs:1851](../../crates/crysirror-catsystem2/src/adapter.rs#L1851) |
| `0x2C9` | Execute run script branch | Handled | — | [adapter.rs:1213](../../crates/crysirror-catsystem2/src/adapter.rs#L1213) |
| `0x2CA` | Execute run script branch | Handled | — | [adapter.rs:1428](../../crates/crysirror-catsystem2/src/adapter.rs#L1428) |
| `0x2CB` | Execute run script branch | Handled | — | [adapter.rs:1428](../../crates/crysirror-catsystem2/src/adapter.rs#L1428) |
| `0x2CD` | Execute run script branch | Handled | — | [adapter.rs:1423](../../crates/crysirror-catsystem2/src/adapter.rs#L1423) |
| `0x2CE` | Execute run script branch | Handled | — | [adapter.rs:1309](../../crates/crysirror-catsystem2/src/adapter.rs#L1309) |
| `0x2CF` | Execute run script branch | Handled | — | [adapter.rs:1310](../../crates/crysirror-catsystem2/src/adapter.rs#L1310) |
| `0x2D0` | Execute run script branch | Handled | — | [adapter.rs:1300](../../crates/crysirror-catsystem2/src/adapter.rs#L1300) |
| `0x2D3` | Execute run script branch | Handled | — | [adapter.rs:1209](../../crates/crysirror-catsystem2/src/adapter.rs#L1209) |
| `0x2D4` | / -> : five raw ats words from the shared HG3 metadata | Handled | — | [adapter.rs:1858](../../crates/crysirror-catsystem2/src/adapter.rs#L1858) |
| `0x2D6` | Execute run script branch | Handled | — | [adapter.rs:1210](../../crates/crysirror-catsystem2/src/adapter.rs#L1210) |
| `0x2D7` | Execute run script branch | Handled | — | [adapter.rs:1211](../../crates/crysirror-catsystem2/src/adapter.rs#L1211) |
| `0x2D8` | /: a word on the image object | Handled | — | [adapter.rs:2548](../../crates/crysirror-catsystem2/src/adapter.rs#L2548) |
| `0x2DA` | Execute run script branch | Handled | — | [adapter.rs:1212](../../crates/crysirror-catsystem2/src/adapter.rs#L1212) |
| `0x2DC` | Execute run script branch | Handled | — | [adapter.rs:1091](../../crates/crysirror-catsystem2/src/adapter.rs#L1091) |
| `0x2DD` | an empty image group of the given size | Handled | — | [adapter.rs:1186](../../crates/crysirror-catsystem2/src/adapter.rs#L1186) |
| `0x2DF` | Update surface / job | Handled | — | [adapter.rs:1196](../../crates/crysirror-catsystem2/src/adapter.rs#L1196) |
| `0x2E0` | Update surface / geometry, surface / job | Handled | — | [adapter.rs:1204](../../crates/crysirror-catsystem2/src/adapter.rs#L1204) |
| `0x2E1` | Execute run script branch | Handled | — | [adapter.rs:1214](../../crates/crysirror-catsystem2/src/adapter.rs#L1214) |
| `0x2E3` | Execute run script branch | Handled | — | [adapter.rs:2931](../../crates/crysirror-catsystem2/src/adapter.rs#L2931) |
| `0x311` | Execute run script branch | Handled | — | [adapter.rs:1871](../../crates/crysirror-catsystem2/src/adapter.rs#L1871) |
| `0x313` | the screen rectangle in script coordinates | Handled | — | [adapter.rs:1705](../../crates/crysirror-catsystem2/src/adapter.rs#L1705) |
| `0x314` | Execute run script branch | Handled | — | [adapter.rs:1710](../../crates/crysirror-catsystem2/src/adapter.rs#L1710) |
| `0x315` | Update settings loading, string variables / language prefix, settings | Handled | — | [adapter.rs:2674](../../crates/crysirror-catsystem2/src/adapter.rs#L2674) |
| `0x316` | GAME/flag local (a1 == 0) or global range a2 into two VM words; an index of 8 or more answers 0 without writing | Handled | — | [adapter.rs:2921](../../crates/crysirror-catsystem2/src/adapter.rs#L2921) |
| `0x34C` | Execute run script branch | Handled | — | [adapter.rs:1568](../../crates/crysirror-catsystem2/src/adapter.rs#L1568) |
| `0x350` | Execute run script branch | Handled | — | [adapter.rs:647](../../crates/crysirror-catsystem2/src/adapter.rs#L647) |
| `0x351` | Execute run script branch | Handled | — | [adapter.rs:647](../../crates/crysirror-catsystem2/src/adapter.rs#L647) |
| `0x352` | , : the log into and out of a save container | Handled | — | [adapter.rs:1904](../../crates/crysirror-catsystem2/src/adapter.rs#L1904) |
| `0x353` | Execute run script branch | Handled | — | [adapter.rs:1910](../../crates/crysirror-catsystem2/src/adapter.rs#L1910) |
| `0x35E` | Messages 285..313 on an image list (). A task of another class has no case | Handled | Guard: self.tasks.image_list(call.arguments[0]).is_err() | [adapter.rs:1333](../../crates/crysirror-catsystem2/src/adapter.rs#L1333) |
| `0x35F` | Execute run script branch | Handled | — | [adapter.rs:827](../../crates/crysirror-catsystem2/src/adapter.rs#L827) |
| `0x360` | Execute run script branch | Handled | — | [adapter.rs:835](../../crates/crysirror-catsystem2/src/adapter.rs#L835) |
| `0x361` | Execute run script branch | Handled | — | [adapter.rs:840](../../crates/crysirror-catsystem2/src/adapter.rs#L840) |
| `0x362` | Execute run script branch | Handled | — | [adapter.rs:850](../../crates/crysirror-catsystem2/src/adapter.rs#L850) |
| `0x363` | Execute run script branch | Handled | — | [adapter.rs:858](../../crates/crysirror-catsystem2/src/adapter.rs#L858) |
| `0x364` | Execute run script branch | Handled | — | [adapter.rs:863](../../crates/crysirror-catsystem2/src/adapter.rs#L863) |
| `0x36C` | Execute run script branch | Handled | — | [adapter.rs:1870](../../crates/crysirror-catsystem2/src/adapter.rs#L1870) |
| `0x36D` | Execute run script branch | Handled | — | [adapter.rs:2553](../../crates/crysirror-catsystem2/src/adapter.rs#L2553) |
| `0x370` | Messages 285..313 on an image list (). A task of another class has no case | Handled | Guard: self.tasks.image_list(call.arguments[0]).is_err() | [adapter.rs:1333](../../crates/crysirror-catsystem2/src/adapter.rs#L1333) |
| `0x372` | Execute run script branch | Handled | — | [adapter.rs:1930](../../crates/crysirror-catsystem2/src/adapter.rs#L1930) |
| `0x375` | Execute run script branch | Handled | — | [adapter.rs:2862](../../crates/crysirror-catsystem2/src/adapter.rs#L2862) |
| `0x376` | Execute run script branch | Handled | — | [adapter.rs:2867](../../crates/crysirror-catsystem2/src/adapter.rs#L2867) |
| `0x377` | Execute run script branch | Handled | — | [adapter.rs:2872](../../crates/crysirror-catsystem2/src/adapter.rs#L2872) |
| `0x379` | Execute run script branch | Handled | — | [adapter.rs:2886](../../crates/crysirror-catsystem2/src/adapter.rs#L2886) |
| `0x37A` | Execute run script branch | Handled | — | [adapter.rs:2891](../../crates/crysirror-catsystem2/src/adapter.rs#L2891) |
| `0x37B` | Execute run script branch | Handled | — | [adapter.rs:2896](../../crates/crysirror-catsystem2/src/adapter.rs#L2896) |
| `0x37C` | a serialized map added to a map without replacing keys | Handled | — | [adapter.rs:1819](../../crates/crysirror-catsystem2/src/adapter.rs#L1819) |
| `0x37E` | Execute run script branch | Handled | — | [adapter.rs:2862](../../crates/crysirror-catsystem2/src/adapter.rs#L2862) |
| `0x37F` | Execute run script branch | Handled | — | [adapter.rs:2867](../../crates/crysirror-catsystem2/src/adapter.rs#L2867) |
| `0x380` | Execute run script branch | Handled | — | [adapter.rs:2872](../../crates/crysirror-catsystem2/src/adapter.rs#L2872) |
| `0x382` | Execute run script branch | Handled | — | [adapter.rs:2886](../../crates/crysirror-catsystem2/src/adapter.rs#L2886) |
| `0x383` | Execute run script branch | Handled | — | [adapter.rs:2891](../../crates/crysirror-catsystem2/src/adapter.rs#L2891) |
| `0x384` | Execute run script branch | Handled | — | [adapter.rs:2896](../../crates/crysirror-catsystem2/src/adapter.rs#L2896) |
| `0x385` | Execute run script branch | Handled | — | [adapter.rs:2903](../../crates/crysirror-catsystem2/src/adapter.rs#L2903) |
| `0x386` | Execute run script branch | Handled | — | [adapter.rs:2912](../../crates/crysirror-catsystem2/src/adapter.rs#L2912) |
| `0x38A` | Execute run script branch | Handled | — | [adapter.rs:1628](../../crates/crysirror-catsystem2/src/adapter.rs#L1628) |
| `0x38F` | 64C?? messages 530..542: the plane's E-mote controller | Handled | — | [adapter.rs:1640](../../crates/crysirror-catsystem2/src/adapter.rs#L1640) |
| `0x391` | 64C?? messages 530..542: the plane's E-mote controller | Handled | — | [adapter.rs:1640](../../crates/crysirror-catsystem2/src/adapter.rs#L1640) |
| `0x392` | Parameter Scale | Handled | — | [adapter.rs:1657](../../crates/crysirror-catsystem2/src/adapter.rs#L1657) |
| `0x394` | Parameter Scale | Handled | — | [adapter.rs:1658](../../crates/crysirror-catsystem2/src/adapter.rs#L1658) |
| `0x395` | Body | Handled | — | [adapter.rs:1647](../../crates/crysirror-catsystem2/src/adapter.rs#L1647) |
| `0x396` | 64C?? messages 530..542: the plane's E-mote controller | Handled | — | [adapter.rs:1640](../../crates/crysirror-catsystem2/src/adapter.rs#L1640) |
| `0x397` | Tone | Handled | — | [adapter.rs:1664](../../crates/crysirror-catsystem2/src/adapter.rs#L1664) |
| `0x399` | Parameter | Handled | — | [adapter.rs:1665](../../crates/crysirror-catsystem2/src/adapter.rs#L1665) |
| `0x39E` | Execute run script branch | Handled | — | [adapter.rs:1653](../../crates/crysirror-catsystem2/src/adapter.rs#L1653) |
| `0x39F` | Blink | Handled | — | [adapter.rs:1670](../../crates/crysirror-catsystem2/src/adapter.rs#L1670) |
| `0x3A0` | 64C?? messages 530..542: the plane's E-mote controller | Handled | — | [adapter.rs:1640](../../crates/crysirror-catsystem2/src/adapter.rs#L1640) |
| `0x3A6` | Execute run script branch | Handled | — | [adapter.rs:770](../../crates/crysirror-catsystem2/src/adapter.rs#L770) |
| `0x3A8` | Execute run script branch | Handled | — | [adapter.rs:1573](../../crates/crysirror-catsystem2/src/adapter.rs#L1573) |
| `0x3AA` | Update sound loading, sound definition | Handled | — | [adapter.rs:2628](../../crates/crysirror-catsystem2/src/adapter.rs#L2628) |
| `0x3AB` | Execute run script branch | Handled | — | [adapter.rs:1547](../../crates/crysirror-catsystem2/src/adapter.rs#L1547) |
| `0x3AE` | Execute run script branch | Handled | — | [adapter.rs:1547](../../crates/crysirror-catsystem2/src/adapter.rs#L1547) |
| `0x3AF` | Execute run script branch | Handled | — | [adapter.rs:1567](../../crates/crysirror-catsystem2/src/adapter.rs#L1567) |
| `0x3B0` | Execute run script branch | Handled | — | [adapter.rs:1562](../../crates/crysirror-catsystem2/src/adapter.rs#L1562) |
| `0x3B2` | Execute run script branch | Handled | — | [adapter.rs:1561](../../crates/crysirror-catsystem2/src/adapter.rs#L1561) |
| `0x3B3` | Execute run script branch | Handled | — | [adapter.rs:1567](../../crates/crysirror-catsystem2/src/adapter.rs#L1567) |
| `0x3B4` | Execute run script branch | Handled | — | [adapter.rs:1563](../../crates/crysirror-catsystem2/src/adapter.rs#L1563) |
| `0x3B6` | Execute run script branch | Handled | — | [adapter.rs:1564](../../crates/crysirror-catsystem2/src/adapter.rs#L1564) |
| `0x3BC` | Execute run script branch | Handled | — | [adapter.rs:1566](../../crates/crysirror-catsystem2/src/adapter.rs#L1566) |
| `0x3BD` | Execute run script branch | Handled | — | [adapter.rs:1564](../../crates/crysirror-catsystem2/src/adapter.rs#L1564) |
| `0x3C0` | Execute run script branch | Handled | — | [adapter.rs:1565](../../crates/crysirror-catsystem2/src/adapter.rs#L1565) |
| `0x3C3` | Execute run script branch | Handled | — | [adapter.rs:1566](../../crates/crysirror-catsystem2/src/adapter.rs#L1566) |
| `0x3C4` | Execute run script branch | Handled | — | [adapter.rs:1567](../../crates/crysirror-catsystem2/src/adapter.rs#L1567) |
| `0x3C6` | Clear Parameter Scale | Handled | — | [adapter.rs:1663](../../crates/crysirror-catsystem2/src/adapter.rs#L1663) |
| `0x3D1` | / -> : live message insertion; 698 supplies history_index=-1, while 977 passes it explicitly | Handled | — | [adapter.rs:1874](../../crates/crysirror-catsystem2/src/adapter.rs#L1874) |
| `0x3D4` | Execute run script branch | Handled | — | [adapter.rs:892](../../crates/crysirror-catsystem2/src/adapter.rs#L892) |
| `0x3D6` | Execute run script branch | Handled | — | [adapter.rs:822](../../crates/crysirror-catsystem2/src/adapter.rs#L822) |
| `0x3DE` | Update save writing | Partial | CatSystem2 unsupported save filename encoding | [adapter.rs:2561](../../crates/crysirror-catsystem2/src/adapter.rs#L2561) |
| `0x3E3` | Execute run script branch | Handled | — | [adapter.rs:1567](../../crates/crysirror-catsystem2/src/adapter.rs#L1567) |
| `0x3E6` | Execute run script branch | Handled | — | [adapter.rs:1696](../../crates/crysirror-catsystem2/src/adapter.rs#L1696) |
| `0x3E7` | 999 reads a zeroed setting | Handled | — | [adapter.rs:2930](../../crates/crysirror-catsystem2/src/adapter.rs#L2930) |
| `0x3E8` | Execute run script branch | Handled | — | [adapter.rs:1547](../../crates/crysirror-catsystem2/src/adapter.rs#L1547) |
| `0x3E9` | Execute run script branch | Handled | — | [adapter.rs:1568](../../crates/crysirror-catsystem2/src/adapter.rs#L1568) |
| `0x3EA` | Execute run script branch | Handled | — | [adapter.rs:908](../../crates/crysirror-catsystem2/src/adapter.rs#L908) |
| `0x402` | Execute run script branch | Handled | — | [adapter.rs:1319](../../crates/crysirror-catsystem2/src/adapter.rs#L1319) |
| `0x407` | Update save writing | Partial | CatSystem2 unsupported save filename encoding | [adapter.rs:2561](../../crates/crysirror-catsystem2/src/adapter.rs#L2561) |
| `0x409` | > (language): only a loaded .cstl table (+0x130) changes the text, which this reader, like 1038 below, does not load | Handled | — | [adapter.rs:899](../../crates/crysirror-catsystem2/src/adapter.rs#L899) |
| `0x40D` | Execute run script branch | Handled | — | [adapter.rs:757](../../crates/crysirror-catsystem2/src/adapter.rs#L757) |
| `0x40E` | > (language): only a loaded .cstl table (+0x130) changes the text, which this reader, like 1038 below, does not load | Handled | — | [adapter.rs:899](../../crates/crysirror-catsystem2/src/adapter.rs#L899) |
| `0x410` | , : debug output | Handled | — | [adapter.rs:1828](../../crates/crysirror-catsystem2/src/adapter.rs#L1828) |
| `0x412` | , : debug output | Handled | — | [adapter.rs:1828](../../crates/crysirror-catsystem2/src/adapter.rs#L1828) |
| `0x41A` | `enable_sceneanm`, as the config screen left it | Handled | — | [adapter.rs:1758](../../crates/crysirror-catsystem2/src/adapter.rs#L1758) |
| `0x41B` | Execute run script branch | Handled | — | [adapter.rs:1806](../../crates/crysirror-catsystem2/src/adapter.rs#L1806) |
| `0x41C` | the value of the first map key that prefixes the text, ignoring case | Handled | — | [adapter.rs:1812](../../crates/crysirror-catsystem2/src/adapter.rs#L1812) |

## Subselectors and conditional dispatch

Patterns retain their source namespace and guards. `_` is the fallback of that table. Boolean/optional results follow the function's return contract. Call-site names identify the implementation to inspect, including indirect service calls.

### adapter.rs / xml_node

```rust
fn xml_node(
    document: &'a XmlDocument,
    path: &str,
) -> Result<Option<&'a crysirror_resource::xml::XmlNode>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `document.root().name == "document"` | Local state/value operations | Handled | [2995](../../crates/crysirror-catsystem2/src/adapter.rs#L2995) |
| `condition` | `parts[0] != document.root().name` | Local state/value operations | Handled | [2998](../../crates/crysirror-catsystem2/src/adapter.rs#L2998) |

### adapter.rs / CatSystem2Vm::run_script

```rust
fn run_script(&mut self,host:&mut dyn HostServices,mut remaining:u32) -> Result<VmYield>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `call.id; under event=Event::Native(call)` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [649](../../crates/crysirror-catsystem2/src/adapter.rs#L649) |
| `condition; under event=Event::Native(call)` | `matches!(call.id,447\|449\|450\|451)&&self.tasks.fes(call.arguments[0]).is_err()` | Local state/value operations | Handled | [652](../../crates/crysirror-catsystem2/src/adapter.rs#L652) |
| `condition; under event=Event::Native(call)` | `matches!(call.id,447\|449\|450\|451)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [652](../../crates/crysirror-catsystem2/src/adapter.rs#L652) |
| `condition; under event=Event::Native(call) / call.id=id if crate::native_classes::claims(id)` | `matches!(id,271\|693\|965)&&!self.fes_children.motions.ensure_random_for_kcs(host,self.client,self.tasks.current(),call.pc)?` | Local state/value operations | Handled | [666](../../crates/crysirror-catsystem2/src/adapter.rs#L666) |
| `condition; under event=Event::Native(call) / call.id=328\|329\|332\|934` | `matches!(call.id,934)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [771](../../crates/crysirror-catsystem2/src/adapter.rs#L771) |
| `condition; under event=Event::Native(call) / call.id=328\|329\|332\|934` | `call.id==934` | crate::native_values::format_bytes | Handled | [775](../../crates/crysirror-catsystem2/src/adapter.rs#L775) |
| `condition; under event=Event::Native(call) / call.id=328\|329\|332\|934 / self.handles.scenes.get_mut(&a[0])=None` | `call.id==329` | Local state/value operations | Handled | [777](../../crates/crysirror-catsystem2/src/adapter.rs#L777) |
| `call.id; under event=Event::Native(call) / call.id=328\|329\|332\|934 / self.handles.scenes.get_mut(&a[0])=Some(scene)` | `_` | crate::native_scene::stem | Handled | [782](../../crates/crysirror-catsystem2/src/adapter.rs#L782) |
| `condition; under event=Event::Native(call) / call.id=336` | `self.handles.scenes.contains_key(&a[0])` | crate::native_classes::parse_number, self.string_variables.get, self.handles.scenes.get_mut | Handled | [794](../../crates/crysirror-catsystem2/src/adapter.rs#L794) |
| `condition; under event=Event::Native(call) / call.id=330\|331` | `call.id==330` | Local state/value operations | Handled | [810](../../crates/crysirror-catsystem2/src/adapter.rs#L810) |
| `condition; under event=Event::Native(call) / call.id=364\|368` | `call.id==368` | Local state/value operations | Handled | [1016](../../crates/crysirror-catsystem2/src/adapter.rs#L1016) |
| `condition; under event=Event::Native(call) / call.id=364\|368` | `call.id==368&&out==0` | Local state/value operations | Handled | [1019](../../crates/crysirror-catsystem2/src/adapter.rs#L1019) |
| `(call.id,frame); under event=Event::Native(call) / call.id=364\|368` | `(364,Some(frame))` | self.handles.allocate, self.handles.images.insert | Handled | [1030](../../crates/crysirror-catsystem2/src/adapter.rs#L1030) |
| `(call.id,frame); under event=Event::Native(call) / call.id=364\|368` | `(368,Some(frame))` | Local state/value operations | Handled | [1031](../../crates/crysirror-catsystem2/src/adapter.rs#L1031) |
| `(call.id,frame); under event=Event::Native(call) / call.id=364\|368` | `(368,None)` | Local state/value operations | Handled | [1036](../../crates/crysirror-catsystem2/src/adapter.rs#L1036) |
| `(call.id,frame); under event=Event::Native(call) / call.id=364\|368` | `_` | Local state/value operations | Handled | [1037](../../crates/crysirror-catsystem2/src/adapter.rs#L1037) |
| `condition; under event=Event::Native(call) / call.id=369 / job.poll().map_err(anyhow::Error::from)?=State::Ready(image)` | `let Some(entry)=self.handles.images.get_mut(&a[0])` | Local state/value operations | Handled | [1062](../../crates/crysirror-catsystem2/src/adapter.rs#L1062) |
| `condition; under event=Event::Native(call) / call.id=369 / job.poll().map_err(anyhow::Error::from)?=State::Ready(image)` | `let Some(upload)=self.handles.uploads.remove(&a[0])` | host.release_image_resource | Handled | [1063](../../crates/crysirror-catsystem2/src/adapter.rs#L1063) |
| `condition; under event=Event::Native(call) / call.id=257\|732` | `self.tasks.image_list(a[0]).is_ok_and(\|list\|list.manager.is_some())` | self.handles.images.get, self.handles.uploads.contains_key, host.publish_image_resource, self.handles.uploads.insert, crate::native_image_publication::ImageList::key | Handled | [1098](../../crates/crysirror-catsystem2/src/adapter.rs#L1098) |
| `condition; under event=Event::Native(call) / call.id=257\|732` | `!self.handles.uploads.contains_key(&a[2])` | host.publish_image_resource, self.handles.uploads.insert | Handled | [1100](../../crates/crysirror-catsystem2/src/adapter.rs#L1100) |
| `condition; under event=Event::Native(call) / call.id=112\|113\|115\|116\|118\|367\|733\|736\|723\|726\|727\|730\|713\|737\|538\|735` | `matches!(call.id,112\|113\|118\|733)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [1125](../../crates/crysirror-catsystem2/src/adapter.rs#L1125) |
| `condition; under event=Event::Native(call) / call.id=112\|113\|115\|116\|118\|367\|733\|736\|723\|726\|727\|730\|713\|737\|538\|735 / call.id=733` | `a[0]==0\|\|a[1]==0` | Local state/value operations | Handled | [1187](../../crates/crysirror-catsystem2/src/adapter.rs#L1187) |
| `call.id; under event=Event::Native(call) / call.id=112\|113\|115\|116\|118\|367\|733\|736\|723\|726\|727\|730\|713\|737\|538\|735 / call.id=723\|726\|727\|730\|713\|737\|736\|538` | `_` | ops::fill | Handled | [1215](../../crates/crysirror-catsystem2/src/adapter.rs#L1215) |
| `condition; under event=Event::Native(call) / call.id=112\|113\|115\|116\|118\|367\|733\|736\|723\|726\|727\|730\|713\|737\|538\|735 / call.id=112` | `a[0]==0\|\|a[1]==0` | Local state/value operations | Handled | [1225](../../crates/crysirror-catsystem2/src/adapter.rs#L1225) |
| `condition; under event=Event::Native(call) / call.id=112\|113\|115\|116\|118\|367\|733\|736\|723\|726\|727\|730\|713\|737\|538\|735 / call.id=_` | `a[0]==0\|\|a[5]==0` | Local state/value operations | Handled | [1249](../../crates/crysirror-catsystem2/src/adapter.rs#L1249) |
| `condition; under event=Event::Native(call) / call.id=112\|113\|115\|116\|118\|367\|733\|736\|723\|726\|727\|730\|713\|737\|538\|735 / call.id=_` | `call.id==116` | Local state/value operations | Handled | [1250](../../crates/crysirror-catsystem2/src/adapter.rs#L1250) |
| `condition; under event=Event::Native(call) / call.id=112\|113\|115\|116\|118\|367\|733\|736\|723\|726\|727\|730\|713\|737\|538\|735 / call.id=_` | `a[5]&0xff00_0000==LIST_BITMAP` | self.tasks.list_publication, Vec::new, self.list_shadows.get, self.ticks.saturating_sub, surface::region_commands | Handled | [1251](../../crates/crysirror-catsystem2/src/adapter.rs#L1251) |
| `call.id; under event=Event::Native(call) / call.id=112\|113\|115\|116\|118\|367\|733\|736\|723\|726\|727\|730\|713\|737\|538\|735` | `_` | self.tasks.list_publication, Vec::new, self.list_shadows.get, self.ticks.saturating_sub, surface::region_commands | Handled | [1246](../../crates/crysirror-catsystem2/src/adapter.rs#L1246) |
| `condition; under event=Event::Native(call) / call.id=362\|363` | `call.id==363` | Local state/value operations | Handled | [1292](../../crates/crysirror-catsystem2/src/adapter.rs#L1292) |
| `call.id; under event=Event::Native(call) / call.id=372\|373\|718\|719\|720 / self.handles.images.get_mut(&a[0])=Some(frame)` | `_` | Local state/value operations | Handled | [1311](../../crates/crysirror-catsystem2/src/adapter.rs#L1311) |
| `condition; under event=Event::Native(call) / call.id=263\|341` | `call.id==263` | crate::native_image_publication::ImageList::key, self.tasks.current | Handled | [1352](../../crates/crysirror-catsystem2/src/adapter.rs#L1352) |
| `condition; under event=Event::Native(call) / call.id=263\|341 / key=Some(key)` | `publication.set_visible(key,call.arguments[2]!=0)` | Vec::new | Handled | [1355](../../crates/crysirror-catsystem2/src/adapter.rs#L1355) |
| `call.id; under event=Event::Native(call) / call.id=714\|715\|717` | `_` | Local state/value operations | Handled | [1428](../../crates/crysirror-catsystem2/src/adapter.rs#L1428) |
| `call.id; under event=Event::Native(call) / call.id=296\|298\|313\|314` | `_` | Local state/value operations | Handled | [1459](../../crates/crysirror-catsystem2/src/adapter.rs#L1459) |
| `condition; under event=Event::Native(call) / call.id=299\|301` | `call.id==299` | Local state/value operations | Handled | [1471](../../crates/crysirror-catsystem2/src/adapter.rs#L1471) |
| `condition; under event=Event::Native(call) / call.id=304\|535` | `!self.handles.motions.contains_key(&a[0])` | Local state/value operations | Handled | [1488](../../crates/crysirror-catsystem2/src/adapter.rs#L1488) |
| `condition; under event=Event::Native(call) / call.id=304\|535` | `a[5]!=0&&a[4]!=0` | Local state/value operations | Handled | [1510](../../crates/crysirror-catsystem2/src/adapter.rs#L1510) |
| `condition; under event=Event::Native(call) / call.id=304\|535` | `call.id==535` | Local state/value operations | Handled | [1511](../../crates/crysirror-catsystem2/src/adapter.rs#L1511) |
| `condition; under event=Event::Native(call) / call.id=307\|440` | `call.id==307` | Local state/value operations | Handled | [1520](../../crates/crysirror-catsystem2/src/adapter.rs#L1520) |
| `condition; under event=Event::Native(call) / call.id=310\|311 / self.handles.motions.get(&call.arguments[0])=Some(set)` | `call.id==310` | Local state/value operations | Handled | [1538](../../crates/crysirror-catsystem2/src/adapter.rs#L1538) |
| `call.id; under event=Event::Native(call) / call.id=605\|606\|607\|674\|844\|939\|942\|943\|944\|946\|947\|948\|950\|956\|957\|960\|963\|964\|995\|1000\|1001` | `_` | Local state/value operations | Handled | [1569](../../crates/crysirror-catsystem2/src/adapter.rs#L1569) |
| `condition; under event=Event::Native(call) / call.id=157\|158\|159` | `call.id==157` | Local state/value operations | Handled | [1598](../../crates/crysirror-catsystem2/src/adapter.rs#L1598) |
| `condition; under event=Event::Native(call) / call.id=161\|163` | `call.id==161` | Local state/value operations | Handled | [1613](../../crates/crysirror-catsystem2/src/adapter.rs#L1613) |
| `condition; under event=Event::Native(call) / call.id=917\|918\|919\|911\|913\|914\|916\|921\|926\|927\|928\|966 / call.id=926` | `a[1]!=0` | Local state/value operations | Handled | [1655](../../crates/crysirror-catsystem2/src/adapter.rs#L1655) |
| `call.id; under event=Event::Native(call) / call.id=917\|918\|919\|911\|913\|914\|916\|921\|926\|927\|928\|966` | `_` | Local state/value operations | Handled | [1673](../../crates/crysirror-catsystem2/src/adapter.rs#L1673) |
| `condition; under event=Event::Native(call) / call.id=72\|73` | `call.id==72` | Local state/value operations | Handled | [1744](../../crates/crysirror-catsystem2/src/adapter.rs#L1744) |
| `condition; under event=Event::Native(call) / call.id=72\|73` | `call.id==73&&remote` | self.script_memory.push | Handled | [1746](../../crates/crysirror-catsystem2/src/adapter.rs#L1746) |
| `condition; under event=Event::Native(call) / call.id=207\|204\|206` | `a[0]==0` | Local state/value operations | Handled | [1768](../../crates/crysirror-catsystem2/src/adapter.rs#L1768) |
| `condition; under event=Event::Native(call) / call.id=207\|204\|206` | `call.id==207` | Local state/value operations | Handled | [1769](../../crates/crysirror-catsystem2/src/adapter.rs#L1769) |
| `condition; under event=Event::Native(call) / call.id=207\|204\|206` | `call.id==204` | math::multiply, math::rotation | Handled | [1772](../../crates/crysirror-catsystem2/src/adapter.rs#L1772) |
| `condition; under event=Event::Native(call) / call.id=698 \| 977` | `a[0]!=0` | crate::native_values::format_bytes, std::str::from_utf8, encoding_rs::SHIFT_JIS.decode_without_bom_handling_and_without_replacement, String::new, u32::from | Handled | [1880](../../crates/crysirror-catsystem2/src/adapter.rs#L1880) |
| `condition; under event=Event::Native(call) / call.id=698 \| 977` | `call.id==698` | Local state/value operations | Handled | [1891](../../crates/crysirror-catsystem2/src/adapter.rs#L1891) |
| `condition; under event=Event::Native(call) / call.id=339` | `self.tasks.image_list(a[0]).is_ok_and(\|list\|list.manager.is_some())` | crate::native_image_publication::ImageList::key, self.tasks.current, self.tasks.list_publication, Vec::new | Handled | [1922](../../crates/crysirror-catsystem2/src/adapter.rs#L1922) |
| `condition; under event=Event::Native(call) / call.id=882` | `self.tasks.fes(call.arguments[0]).is_ok_and(\|task\|task.player_loaded)&&!name.is_empty()` | self.interpreter.as_mut | Handled | [1935](../../crates/crysirror-catsystem2/src/adapter.rs#L1935) |
| `condition; under event=Event::Native(call) / call.id=19..=22` | `matches!(call.id,20\|22)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [2005](../../crates/crysirror-catsystem2/src/adapter.rs#L2005) |
| `condition; under event=Event::Native(call) / call.id=100` | `let Some((payload,color))=proposed.set_solid_diffuse(id,call.arguments[1],call.arguments[2]).with_context(\|\|format!("CatSystem2 native100 at {} PC {:#x}",self.config.script,call.pc))?` | host.submit_engine, serde_json::to_vec | Handled | [2177](../../crates/crysirror-catsystem2/src/adapter.rs#L2177) |
| `call.id; under event=Event::Native(call) / call.id=84\|85\|88\|93\|94\|101` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [2224](../../crates/crysirror-catsystem2/src/adapter.rs#L2224) |
| `condition; under event=Event::Native(call) / call.id=84\|85\|88\|93\|94\|101` | `let Some(plane)=self.tasks.plane_mut(a[0])` | crate::native_plane::publish | Handled | [2226](../../crates/crysirror-catsystem2/src/adapter.rs#L2226) |
| `call.id; under event=Event::Native(call) / call.id=84\|85\|88\|93\|94\|101` | `_` | Local state/value operations | Handled | [2238](../../crates/crysirror-catsystem2/src/adapter.rs#L2238) |
| `call.id; under event=Event::Native(call) / call.id=86\|87\|343\|345` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [2250](../../crates/crysirror-catsystem2/src/adapter.rs#L2250) |
| `condition; under event=Event::Native(call) / call.id=86\|87\|343\|345` | `selector!=0` | Local state/value operations | Handled | [2251](../../crates/crysirror-catsystem2/src/adapter.rs#L2251) |
| `call.id; under event=Event::Native(call) / call.id=86\|87\|343\|345` | `_` | Local state/value operations | Handled | [2256](../../crates/crysirror-catsystem2/src/adapter.rs#L2256) |
| `condition; under event=Event::Native(call) / call.id=245\|1031\|990` | `call.id==990` | Local state/value operations | Handled | [2565](../../crates/crysirror-catsystem2/src/adapter.rs#L2565) |
| `condition; under event=Event::Native(call) / call.id=245\|1031\|990` | `call.id==990` | Local state/value operations | Handled | [2569](../../crates/crysirror-catsystem2/src/adapter.rs#L2569) |
| `condition; under event=Event::Native(call) / call.id=211\|212` | `call.id==211` | Local state/value operations | Handled | [2659](../../crates/crysirror-catsystem2/src/adapter.rs#L2659) |
| `condition; under event=Event::Native(call) / call.id=359\|789` | `call.id==359` | Local state/value operations | Handled | [2685](../../crates/crysirror-catsystem2/src/adapter.rs#L2685) |
| `condition; under event=Event::Native(call) / call.id=359\|789` | `call.id==359` | Local state/value operations | Handled | [2690](../../crates/crysirror-catsystem2/src/adapter.rs#L2690) |
| `condition; under event=Event::Native(call) / call.id=591` | `id!=0` | Local state/value operations | Handled | [2791](../../crates/crysirror-catsystem2/src/adapter.rs#L2791) |
| `condition; under event=Event::Native(call) / call.id=592\|593` | `id==0` | Local state/value operations | Handled | [2799](../../crates/crysirror-catsystem2/src/adapter.rs#L2799) |
| `condition; under event=Event::Native(call) / call.id=592\|593` | `call.id==592` | crate::native_values::integer | Handled | [2804](../../crates/crysirror-catsystem2/src/adapter.rs#L2804) |
| `condition; under event=Event::Native(call) / call.id=13\|14\|15\|16` | `call.id==13\|\|call.id==15` | Local state/value operations | Handled | [2828](../../crates/crysirror-catsystem2/src/adapter.rs#L2828) |
| `condition; under event=Event::Native(call) / call.id=13\|14\|15\|16` | `call.id==13\|\|call.id==15` | u64::from | Partial: unsupported overlapping native CatSystem2 string operation | [2829](../../crates/crysirror-catsystem2/src/adapter.rs#L2829) |
| `condition; under event=Event::Native(call) / call.id=13\|14\|15\|16` | `call.id==15` | Local state/value operations | Handled | [2832](../../crates/crysirror-catsystem2/src/adapter.rs#L2832) |
| `condition; under event=Event::Native(call) / call.id=13\|14\|15\|16` | `call.id==15\|\|call.id==16` | Local state/value operations | Handled | [2836](../../crates/crysirror-catsystem2/src/adapter.rs#L2836) |
| `condition; under event=Event::Native(call) / call.id=17\|18` | `call.id==17` | Local state/value operations | Handled | [2847](../../crates/crysirror-catsystem2/src/adapter.rs#L2847) |
| `condition; under event=Event::Native(call) / call.id=517\|887\|896` | `call.id==517` | Local state/value operations | Handled | [2876](../../crates/crysirror-catsystem2/src/adapter.rs#L2876) |
| `condition; under event=Event::Native(call) / call.id=517\|887\|896` | `call.id==517` | self.string_maps.insert_new | Handled | [2877](../../crates/crysirror-catsystem2/src/adapter.rs#L2877) |
| `call.id; under event=Event::Native(call)` | `id if crate::native_classes::claims(id)` | self.fes_children.motions.ensure_random_for_kcs, self.tasks.current, self.pool.as_ref, self.settings.as_mut, self.interpreter.as_mut | Handled | [664](../../crates/crysirror-catsystem2/src/adapter.rs#L664) |
| `call.id; under event=Event::Native(call)` | `261\|263\|340\|341\|455\|862\|880 if self.tasks.image_list(call.arguments[0]).is_err()` | Local state/value operations | Handled | [1333](../../crates/crysirror-catsystem2/src/adapter.rs#L1333) |
| `call.id; under event=Event::Native(call)` | `id if crate::native_misc::IDS.contains(&id)` | crate::native_misc::apply | Handled | [2543](../../crates/crysirror-catsystem2/src/adapter.rs#L2543) |
| `call.id; under event=Event::Native(call)` | `id if crate::native_store::IDS.contains(&id)` | crate::native_store::apply | Handled | [2557](../../crates/crysirror-catsystem2/src/adapter.rs#L2557) |
| `call.id; under event=Event::Native(call)` | `246 if call.arguments.first()==Some(&0)` | Local state/value operations | Handled | [2941](../../crates/crysirror-catsystem2/src/adapter.rs#L2941) |
| `call.id; under event=Event::Native(call)` | `id if crate::native_scalar::apply(id,&call.arguments).is_some()` | crate::native_scalar::apply | Handled | [2967](../../crates/crysirror-catsystem2/src/adapter.rs#L2967) |
| `call.id; under event=Event::Native(call)` | `_` | Local state/value operations | Partial: CatSystem2 unsupported native gcall {} at {} PC {:#x} ({} argument bytes); original call remains unacknowledged | [2972](../../crates/crysirror-catsystem2/src/adapter.rs#L2972) |

### fes_alias_values.rs / parameter

```rust
fn parameter(
    kind: usize,
    source: &[u8],
    fallback: &[u8],
    values: Values,
    vm: &mut Interpreter,
) -> Result<f32>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `kind == 0` | Local state/value operations | Handled | [129](../../crates/crysirror-catsystem2/src/fes_alias_values.rs#L129) |
| `condition` | `kind >= 39` | Local state/value operations | Handled | [132](../../crates/crysirror-catsystem2/src/fes_alias_values.rs#L132) |
| `kind` | `1..=18` | Local state/value operations | Handled | [136](../../crates/crysirror-catsystem2/src/fes_alias_values.rs#L136) |
| `kind` | `19..=24` | Local state/value operations | Handled | [147](../../crates/crysirror-catsystem2/src/fes_alias_values.rs#L147) |
| `kind` | `25..=30` | Local state/value operations | Handled | [151](../../crates/crysirror-catsystem2/src/fes_alias_values.rs#L151) |
| `kind` | `31..=38` | Local state/value operations | Handled | [155](../../crates/crysirror-catsystem2/src/fes_alias_values.rs#L155) |
| `kind` | `_` | Local state/value operations | Handled | [160](../../crates/crysirror-catsystem2/src/fes_alias_values.rs#L160) |

### fes_alias_values.rs / project

```rust
fn project(
    alias: &Alias,
    args: &[Vec<u8>],
    values: Values,
    vm: &mut Interpreter,
) -> Result<Vec<f32>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `name` | `b"cxr"` | Local state/value operations | Handled | [206](../../crates/crysirror-catsystem2/src/fes_alias_values.rs#L206) |
| `name` | `b"cxg"` | Local state/value operations | Handled | [207](../../crates/crysirror-catsystem2/src/fes_alias_values.rs#L207) |
| `name` | `b"cxb"` | Local state/value operations | Handled | [208](../../crates/crysirror-catsystem2/src/fes_alias_values.rs#L208) |
| `name` | `b"cxa" \| b"CA"` | Local state/value operations | Handled | [209](../../crates/crysirror-catsystem2/src/fes_alias_values.rs#L209) |
| `name` | `_` | Local state/value operations | Handled | [210](../../crates/crysirror-catsystem2/src/fes_alias_values.rs#L210) |

### fes_bg_table.rs / applies

```rust
fn applies(bank: &crate::fes_object_bank::Bank, target: &[u8], method: &[u8], vm: &mut crate::native_interpreter::Interpreter, strings: &crate::native_values::StringVariables) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!matches!(method, b"exist" \| b"load")` | Local state/value operations | Handled | [25](../../crates/crysirror-catsystem2/src/fes_bg_table.rs#L25) |
| `condition` | `matches!(method, b"exist" \| b"load")` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [25](../../crates/crysirror-catsystem2/src/fes_bg_table.rs#L25) |

### fes_bg_table.rs / apply

```rust
fn apply(bank: &mut crate::fes_object_bank::Bank, target: &[u8], arguments: &[Vec<u8>], vm: &mut crate::native_interpreter::Interpreter, strings: &crate::native_values::StringVariables) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `method == b"exist"` | crate::fes_references::evaluate | Handled | [34](../../crates/crysirror-catsystem2/src/fes_bg_table.rs#L34) |
| `condition` | `method == b"exist"` | Local state/value operations | Handled | [42](../../crates/crysirror-catsystem2/src/fes_bg_table.rs#L42) |
| `condition` | `method == b"exist"` | u32::from | Handled | [47](../../crates/crysirror-catsystem2/src/fes_bg_table.rs#L47) |

### fes_bg_table.rs / parse

```rust
fn parse(kind: u8, bytes: Vec<u8>) -> Result<Table>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `kind == 16` | u32::from_le_bytes, Vec::with_capacity, core::array::from_fn, Table::Fixed | Handled | [63](../../crates/crysirror-catsystem2/src/fes_bg_table.rs#L63) |

### fes_button.rs / applies

```rust
fn applies(bank: &Bank, target: &[u8], method: &[u8], vm: &Interpreter, strings: &StringVariables) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under &bank.objects[&id].state=State::Image(image)` | `matches!(method, b"setid" \| b"getid")` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [125](../../crates/crysirror-catsystem2/src/fes_button.rs#L125) |

### fes_button.rs / apply

```rust
fn apply(bank: &mut Bank, target: &[u8], arguments: &[Vec<u8>], vm: &mut Interpreter, strings: &StringVariables) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `reference.index == -1 && matches!(method.as_slice(), b"getid" \| b"noact")` | Local state/value operations | Handled | [136](../../crates/crysirror-catsystem2/src/fes_button.rs#L136) |
| `condition` | `method==b"attach"` | crate::fes_references::evaluate | Handled | [140](../../crates/crysirror-catsystem2/src/fes_button.rs#L140) |
| `method.as_slice()` | `b"setid"` | Local state/value operations | Handled | [155](../../crates/crysirror-catsystem2/src/fes_button.rs#L155) |
| `method.as_slice()` | `b"getid"` | Local state/value operations | Handled | [161](../../crates/crysirror-catsystem2/src/fes_button.rs#L161) |
| `method.as_slice()` | `b"enable"` | Local state/value operations | Handled | [164](../../crates/crysirror-catsystem2/src/fes_button.rs#L164) |
| `method.as_slice()` | `b"drag"` | Local state/value operations | Handled | [171](../../crates/crysirror-catsystem2/src/fes_button.rs#L171) |
| `method.as_slice()` | `b"noact"` | Local state/value operations | Handled | [172](../../crates/crysirror-catsystem2/src/fes_button.rs#L172) |
| `method.as_slice()` | `b"lock"` | Local state/value operations | Handled | [173](../../crates/crysirror-catsystem2/src/fes_button.rs#L173) |
| `method.as_slice()` | `_` | Local state/value operations | Handled | [174](../../crates/crysirror-catsystem2/src/fes_button.rs#L174) |

### fes_button.rs / supports

```rust
fn supports(method: &[u8]) -> bool
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(method, b"setid" \| b"getid" \| b"enable" \| b"noact" \| b"lock" \| b"attach" \| b"drag")` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [114](../../crates/crysirror-catsystem2/src/fes_button.rs#L114) |

### fes_declarations.rs / header

```rust
fn header(
    tokens: &[Vec<u8>],
    integer: &impl Fn(&[u8]) -> Result<u32, String>,
) -> Result<Vec<u8>, String>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `name.is_empty() \|\| name.len() >= 32` | Local state/value operations | Unsupported: native OBJECT field name extent | [276](../../crates/crysirror-catsystem2/src/fes_declarations.rs#L276) |
| `condition` | `code >= 66` | Local state/value operations | Unsupported: native OBJECT field outside reviewed projection | [298](../../crates/crysirror-catsystem2/src/fes_declarations.rs#L298) |

### fes_declarations.rs / reference

```rust
fn reference(token: &[u8]) -> Result<Reference, String>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `name.is_empty() \|\| name.len() >= 32 \|\| name.contains(&b']')` | Local state/value operations | Unsupported: native object name extent/layout | [229](../../crates/crysirror-catsystem2/src/fes_declarations.rs#L229) |

### fes_file_list.rs / Lists::apply

```rust
fn apply(&mut self, storage: Arc<dyn ClientStorage>, pool: &TaskPool, owner: u32, pc: u32, bank: &Bank, target: &[u8], arguments: &[Vec<u8>], vm: &mut Interpreter, strings: &mut StringVariables) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `method == b"make"` | crate::fes_references::evaluate, String::new | Handled | [89](../../crates/crysirror-catsystem2/src/fes_file_list.rs#L89) |
| `condition` | `bank.objects[&id].declared_type != 35` | Local state/value operations | Handled | [99](../../crates/crysirror-catsystem2/src/fes_file_list.rs#L99) |
| `condition` | `kind == b"count"` | Local state/value operations | Handled | [110](../../crates/crysirror-catsystem2/src/fes_file_list.rs#L110) |
| `condition; under receiver=else: kind == b"count"` | `kind == b"fullpath"` | Local state/value operations | Handled | [121](../../crates/crysirror-catsystem2/src/fes_file_list.rs#L121) |

### fes_file_probe.rs / Pending::begin

```rust
fn begin(
        pool: &TaskPool,
        storage: Arc<dyn ClientStorage>,
        owner: u32,
        pc: u32,
        operation: crate::fes_program::PendingOperation,
        flag: u32,
    ) -> Result<Option<Self>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `fields .iter() .find(\|n\| n.name == "int") .is_some_and(\|n\| n.text.eq_ignore_ascii_case(archive))` | VfsPath::new | Handled | [54](../../crates/crysirror-catsystem2/src/fes_file_probe.rs#L54) |
| `condition` | `let Some(dir) = fields .iter() .find(\|n\| n.name == "dir" && !n.text.is_empty())` | VfsPath::new | Handled | [59](../../crates/crysirror-catsystem2/src/fes_file_probe.rs#L59) |

### fes_image_methods.rs / apply

```rust
fn apply(bank: &mut Bank, target: &[u8], arguments: &[Vec<u8>], vm: &mut Interpreter, strings: &StringVariables) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under method.as_slice()=b"clip" \| b"tone" \| b"filter" \| b"coordmode" \| b"blur"` | `method == b"tone"` | Local state/value operations | Handled | [133](../../crates/crysirror-catsystem2/src/fes_image_methods.rs#L133) |
| `condition; under method.as_slice()=b"clip" \| b"tone" \| b"filter" \| b"coordmode" \| b"blur"` | `method == b"blur"` | Local state/value operations | Handled | [134](../../crates/crysirror-catsystem2/src/fes_image_methods.rs#L134) |
| `condition; under method.as_slice()=b"clip" \| b"tone" \| b"filter" \| b"coordmode" \| b"blur"` | `method == b"filter"` | Local state/value operations | Handled | [135](../../crates/crysirror-catsystem2/src/fes_image_methods.rs#L135) |
| `condition; under method.as_slice()=b"clip" \| b"tone" \| b"filter" \| b"coordmode" \| b"blur"` | `name.eq_ignore_ascii_case(b"mono")` | Local state/value operations | Handled | [137](../../crates/crysirror-catsystem2/src/fes_image_methods.rs#L137) |
| `condition; under method.as_slice()=b"clip" \| b"tone" \| b"filter" \| b"coordmode" \| b"blur"` | `name.eq_ignore_ascii_case(b"sepia")` | Local state/value operations | Handled | [137](../../crates/crysirror-catsystem2/src/fes_image_methods.rs#L137) |
| `condition; under method.as_slice()=b"clip" \| b"tone" \| b"filter" \| b"coordmode" \| b"blur"` | `name.eq_ignore_ascii_case(b"np")` | Local state/value operations | Handled | [137](../../crates/crysirror-catsystem2/src/fes_image_methods.rs#L137) |
| `condition; under method.as_slice()=b"clip" \| b"tone" \| b"filter" \| b"coordmode" \| b"blur"` | `method == b"clip"` | Local state/value operations | Handled | [139](../../crates/crysirror-catsystem2/src/fes_image_methods.rs#L139) |
| `condition; under method.as_slice()=b"clip" \| b"tone" \| b"filter" \| b"coordmode" \| b"blur"` | `method == b"tone"` | Local state/value operations | Handled | [139](../../crates/crysirror-catsystem2/src/fes_image_methods.rs#L139) |
| `condition; under method.as_slice()=b"clip" \| b"tone" \| b"filter" \| b"coordmode" \| b"blur"` | `method==b"blur"` | Local state/value operations | Handled | [139](../../crates/crysirror-catsystem2/src/fes_image_methods.rs#L139) |
| `condition; under method.as_slice()=b"clip" \| b"tone" \| b"filter" \| b"coordmode" \| b"blur"` | `method==b"clip" && reference.index == -1` | Local state/value operations | Handled | [145](../../crates/crysirror-catsystem2/src/fes_image_methods.rs#L145) |
| `method.as_slice(); under method.as_slice()=b"clip" \| b"tone" \| b"filter" \| b"coordmode" \| b"blur"` | `b"clip"` | Local state/value operations | Handled | [149](../../crates/crysirror-catsystem2/src/fes_image_methods.rs#L149) |
| `method.as_slice(); under method.as_slice()=b"clip" \| b"tone" \| b"filter" \| b"coordmode" \| b"blur"` | `b"tone"` | Local state/value operations | Handled | [158](../../crates/crysirror-catsystem2/src/fes_image_methods.rs#L158) |
| `method.as_slice(); under method.as_slice()=b"clip" \| b"tone" \| b"filter" \| b"coordmode" \| b"blur"` | `b"filter"` | Local state/value operations | Handled | [159](../../crates/crysirror-catsystem2/src/fes_image_methods.rs#L159) |
| `method.as_slice(); under method.as_slice()=b"clip" \| b"tone" \| b"filter" \| b"coordmode" \| b"blur"` | `b"blur"` | Local state/value operations | Handled | [160](../../crates/crysirror-catsystem2/src/fes_image_methods.rs#L160) |
| `method.as_slice(); under method.as_slice()=b"clip" \| b"tone" \| b"filter" \| b"coordmode" \| b"blur"` | `_` | Local state/value operations | Handled | [161](../../crates/crysirror-catsystem2/src/fes_image_methods.rs#L161) |
| `condition; under method.as_slice()=b"getbase" \| b"getoffset" / &bank.objects.get(id).context("CatSystem2 pose object")?.state=State::Image(image) \| State::CImage(image)` | `method == b"getbase"` | Local state/value operations | Handled | [197](../../crates/crysirror-catsystem2/src/fes_image_methods.rs#L197) |
| `method.as_slice()` | `b"keypos"` | Local state/value operations | Handled | [117](../../crates/crysirror-catsystem2/src/fes_image_methods.rs#L117) |
| `method.as_slice()` | `b"clip" \| b"tone" \| b"filter" \| b"coordmode" \| b"blur"` | crate::fes_references::evaluate | Handled | [131](../../crates/crysirror-catsystem2/src/fes_image_methods.rs#L131) |
| `method.as_slice()` | `b"settbl"` | Local state/value operations | Handled | [168](../../crates/crysirror-catsystem2/src/fes_image_methods.rs#L168) |
| `method.as_slice()` | `b"pl"` | crate::fes_references::evaluate | Handled | [182](../../crates/crysirror-catsystem2/src/fes_image_methods.rs#L182) |
| `method.as_slice()` | `b"getbase" \| b"getoffset"` | Local state/value operations | Handled | [192](../../crates/crysirror-catsystem2/src/fes_image_methods.rs#L192) |
| `method.as_slice()` | `other` | Local state/value operations | Partial: CatSystem2 image method {} is not implemented | [206](../../crates/crysirror-catsystem2/src/fes_image_methods.rs#L206) |

### fes_image_methods.rs / supports

```rust
fn supports(method: &[u8]) -> bool
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(method, b"settbl" \| b"pl" \| b"getbase" \| b"getoffset" \| b"anm" \| b"clip" \| b"tone" \| b"filter" \| b"coordmode" \| b"blur" \| b"keypos")` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [15](../../crates/crysirror-catsystem2/src/fes_image_methods.rs#L15) |

### fes_live_motions.rs / Motions::apply_common

```rust
fn apply_common(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        pool: &TaskPool,
        owner: u32,
        root: u32,
        pc: u32,
        operation: &PendingOperation,
        alias: &Alias,
        bank: &mut Bank,
        vm: &mut Interpreter,
        strings: &StringVariables,
        builtin_blend: bool,
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `self.cache.len() < 256 && self .cache .values() .map(\|p\| p.code.len()) .sum::<usize>() .saturating_add(program.code.len()) <= 64 * 1024 * 1024` | Local state/value operations | Handled | [457](../../crates/crysirror-catsystem2/src/fes_live_motions.rs#L457) |

### fes_live_motions.rs / Motions::builtin_path

```rust
fn builtin_path(&mut self,host:&mut dyn HostServices,client:ClientId,pool:&TaskPool,owner:u32,root:u32,pc:u32,operation:&PendingOperation,bank:&mut Bank,vm:&mut Interpreter,strings:&StringVariables) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `method.as_slice()` | `b"arc"` | Local state/value operations | Handled | [218](../../crates/crysirror-catsystem2/src/fes_live_motions.rs#L218) |
| `method.as_slice()` | `b"mcarc"` | Local state/value operations | Handled | [218](../../crates/crysirror-catsystem2/src/fes_live_motions.rs#L218) |
| `method.as_slice()` | `b"pend"` | Local state/value operations | Handled | [218](../../crates/crysirror-catsystem2/src/fes_live_motions.rs#L218) |
| `method.as_slice()` | `b"mcpend"` | Local state/value operations | Handled | [218](../../crates/crysirror-catsystem2/src/fes_live_motions.rs#L218) |
| `method.as_slice()` | `_` | Local state/value operations | Handled | [218](../../crates/crysirror-catsystem2/src/fes_live_motions.rs#L218) |

### fes_loading.rs / load

```rust
fn load(
    storage: &dyn ClientStorage,
    client: ClientId,
    name: &str,
    cancel: crysirror_tasks::Cancellation,
) -> Result<Prepared>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Some(n) = xml.children(node)?.find(\|n\| n.name == "name")` | Local state/value operations | Handled | [39](../../crates/crysirror-catsystem2/src/fes_loading.rs#L39) |

### fes_object_bank.rs / Bank::active

```rust
fn active(bank:&Bank,id:u32,depth:usize) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `active(bank,child.id,depth+1)?` | Local state/value operations | Handled | [61](../../crates/crysirror-catsystem2/src/fes_object_bank.rs#L61) |

### fes_object_bank.rs / Bank::direct

```rust
fn direct(&mut self,target:&[u8],arguments:&[Vec<u8>],vm:&mut Interpreter,strings:&StringVariables) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `command.as_slice()` | `b"disp"` | Local state/value operations | Handled | [110](../../crates/crysirror-catsystem2/src/fes_object_bank.rs#L110) |
| `command.as_slice()` | `b"size"` | Local state/value operations | Handled | [111](../../crates/crysirror-catsystem2/src/fes_object_bank.rs#L111) |
| `command.as_slice()` | `b"pri"` | Local state/value operations | Handled | [112](../../crates/crysirror-catsystem2/src/fes_object_bank.rs#L112) |
| `command.as_slice()` | `_` | Local state/value operations | Handled | [113](../../crates/crysirror-catsystem2/src/fes_object_bank.rs#L113) |
| `condition` | `command==b"size"` | Vec::new | Handled | [115](../../crates/crysirror-catsystem2/src/fes_object_bank.rs#L115) |
| `condition` | `command==b"disp"` | u32::from | Handled | [123](../../crates/crysirror-catsystem2/src/fes_object_bank.rs#L123) |

### fes_object_resources.rs / Pending::poll_inner

```rust
fn poll_inner(&mut self, host: &mut dyn HostServices) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under &object.state=State::Image(image) if !image.filename.is_empty()` | `name.len()>4&&name[name.len()-4..].eq_ignore_ascii_case(b".hg3")` | Local state/value operations | Handled | [411](../../crates/crysirror-catsystem2/src/fes_object_resources.rs#L411) |

### fes_object_resources.rs / Pending::publish_frame

```rust
fn publish_frame(&mut self,host:&mut dyn HostServices,client:ClientId,root:u32,list:&ImageList,final_layers:Vec<L>) -> Result<Receipt>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `depth > 64 \|\| parent == id` | Local state/value operations | Handled | [495](../../crates/crysirror-catsystem2/src/fes_object_resources.rs#L495) |

### fes_object_values.rs / project

```rust
fn project(
    fields: &[(u8, Vec<u8>)],
    target_index: i32,
    interpreter: &mut Interpreter,
    reference: &mut impl FnMut(&[u8], &mut Interpreter) -> Result<IndexedName>,
    parent: &mut impl FnMut(&IndexedName) -> Result<Option<u32>>,
) -> Result<ObjectValues>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under *field=49..=52` | `value.name == b"0"` | Local state/value operations | Handled | [134](../../crates/crysirror-catsystem2/src/fes_object_values.rs#L134) |

### fes_panel_parts.rs / source

```rust
fn source(raw: &[u8], vm: &mut Interpreter) -> Result<(Source, u32, u32)>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `name` | `b"FLAG"` | Source::Flag | Handled | [90](../../crates/crysirror-catsystem2/src/fes_panel_parts.rs#L90) |
| `name` | `b"DATE_Y"` | Source::Date | Handled | [91](../../crates/crysirror-catsystem2/src/fes_panel_parts.rs#L91) |
| `name` | `b"DATE_M"` | Source::Date | Handled | [91](../../crates/crysirror-catsystem2/src/fes_panel_parts.rs#L91) |
| `name` | `b"DATE_D"` | Source::Date | Handled | [91](../../crates/crysirror-catsystem2/src/fes_panel_parts.rs#L91) |
| `name` | `b"DATE_H"` | Source::Date | Handled | [92](../../crates/crysirror-catsystem2/src/fes_panel_parts.rs#L92) |
| `name` | `b"DATE_N"` | Source::Date | Handled | [92](../../crates/crysirror-catsystem2/src/fes_panel_parts.rs#L92) |
| `name` | `b"DATE_S"` | Source::Date | Handled | [92](../../crates/crysirror-catsystem2/src/fes_panel_parts.rs#L92) |
| `name` | `_` | Local state/value operations | Handled | [93](../../crates/crysirror-catsystem2/src/fes_panel_parts.rs#L93) |

### fes_program.rs / initialize_player_bank

```rust
fn initialize_player_bank(owner:u32, interpreter:&mut Interpreter) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `name == b"_EXITCODE_"` | Local state/value operations | Handled | [259](../../crates/crysirror-catsystem2/src/fes_program.rs#L259) |

### fes_program.rs / Machine::step_at_clock_with_strings

```rust
fn step_at_clock_with_strings(&mut self,interpreter:&mut Interpreter,budget:u32,clock:u32,
        strings:Option<&crate::native_values::StringVariables>) -> Result<Option<PendingOperation>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `object_name==b"_TOPOBJ_" \|\| self.program.declarations.definitions.iter() .any(\|(_,definition)\|definition.reference.name==object_name)` | self.section.clone | Handled | [537](../../crates/crysirror-catsystem2/src/fes_program.rs#L537) |

### fes_references.rs / evaluate

```rust
fn evaluate(
    source: &[u8],
    vm: &mut Interpreter,
    strings: &StringVariables,
    count: &impl Fn(&[u8]) -> Option<u32>,
) -> Result<IndexedName>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `index == -1 && count(&name).unwrap_or(0) <= 1` | Local state/value operations | Handled | [53](../../crates/crysirror-catsystem2/src/fes_references.rs#L53) |

### fes_runtime.rs / Services<'_>::run_frame

```rust
fn run_frame(
        &mut self,
        host: &mut dyn HostServices,
        owner: u32,
        origin_pc: u32,
        observation: InputObservation,
        budget: &mut u32,
    ) -> Result<Tick>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under (op.kind.clone(),op.name.as_slice())=(PendingKind::Object,_)` | `method==b"setanm"` | host.storage, self.tasks.fes_object_bank_mut, self.fes_children.animations.set | Handled | [122](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L122) |
| `condition; under (op.kind.clone(),op.name.as_slice())=(PendingKind::Object,_)` | `method==b"anm"` | self.tasks.fes_object_bank_mut, self.fes_children.animations.run | Handled | [127](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L127) |
| `condition; under (op.kind.clone(),op.name.as_slice())=(PendingKind::Object,_)` | `method==b"loadlist2"` | self.interpreter.clone, Group::Number, Group::Name, self.fes_children.cg.as_ref, self.tasks.fes_object_bank_mut | Handled | [130](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L130) |
| `condition; under (op.kind.clone(),op.name.as_slice())=(PendingKind::Object,_)` | `matches!(method.as_slice(),b"ref"\|b"refex"\|b"copy")` | self.interpreter.clone, self.tasks.fes_object_bank_mut, self.handles.and_then, h.images.get, crate::fes_references::evaluate | Handled | [153](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L153) |
| `condition; under (op.kind.clone(),op.name.as_slice())=(PendingKind::Object,_)` | `method==b"refex"` | self.handles.and_then, h.images.get | Handled | [160](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L160) |
| `condition; under (op.kind.clone(),op.name.as_slice())=(PendingKind::Object,_)` | `method==b"copy"` | Local state/value operations | Handled | [169](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L169) |
| `condition; under (op.kind.clone(),op.name.as_slice())=(PendingKind::Object,_)` | `(method==b"refex"&&matches!(object.declared_type,1\|2\|25))\|\|(method!=b"refex"&&matches!(object.declared_type,2\|6\|11\|12\|22))` | Local state/value operations | Handled | [176](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L176) |
| `condition; under (op.kind.clone(),op.name.as_slice())=(PendingKind::Object,_)` | `method==b"refex"\|\|source_id.is_some()` | Local state/value operations | Handled | [178](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L178) |
| `condition; under (op.kind.clone(),op.name.as_slice())=(PendingKind::Object,_)` | `method==b"attr"` | self.interpreter.clone, self.tasks.fes_object_bank_mut, self.fes_children.frames.ready.get | Handled | [186](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L186) |
| `condition; under (op.kind.clone(),op.name.as_slice())=(PendingKind::Object,_)` | `method==b"sndtag"` | self.interpreter.clone, self.tasks.fes_object_bank_mut, crate::fes_references::evaluate, self.string_variables.expand, self.fes_children.sound_tags.init | Partial: CatSystem2 unsupported sndtag command {} | [201](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L201) |
| `condition; under (op.kind.clone(),op.name.as_slice())=(PendingKind::Object,_)` | `command==b"init"` | self.string_variables.expand, self.fes_children.sound_tags.init, host.storage, self.fes_children.sound_tags.finish | Handled | [208](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L208) |
| `condition; under (op.kind.clone(),op.name.as_slice())=(PendingKind::Object,_)` | `matches!(command,b"wait"\|b"remain")&&reference.index!=-1` | self.string_variables.expand, self.fes_children.sound_tags.remaining, self.tasks.begin_fes_wait | Handled | [216](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L216) |
| `condition; under (op.kind.clone(),op.name.as_slice())=(PendingKind::Object,_)` | `command==b"remain"` | Local state/value operations | Handled | [219](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L219) |
| `condition; under (op.kind.clone(),op.name.as_slice())=(PendingKind::Object,_)` | `!matches!(command,b"init"\|b"wait"\|b"remain")` | Local state/value operations | Partial: CatSystem2 unsupported sndtag command {} | [226](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L226) |
| `condition; under (op.kind.clone(),op.name.as_slice())=(PendingKind::Object,_)` | `method==b"time"` | self.interpreter.clone, self.tasks.fes_object_bank_mut, host.audio_info, u64::from | Handled | [228](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L228) |
| `condition; under (op.kind.clone(),op.name.as_slice())=(PendingKind::Object,_)` | `method==b"ismot"` | self.interpreter.clone, self.tasks.fes_object_bank_mut, u32::from | Handled | [239](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L239) |
| `condition; under (op.kind.clone(),op.name.as_slice())=(PendingKind::Object,_)` | `let Some(source)=op.arguments.get(1)` | self.tasks.fes_object_bank_mut, u32::from | Handled | [241](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L241) |
| `condition; under (op.kind.clone(),op.name.as_slice())=(PendingKind::Object,_)` | `matches!(method.as_slice(),b"arc"\|b"mcarc"\|b"pend"\|b"mcpend")` | self.tasks.fes, self.tasks.fes_object_bank_mut, self.fes_children.motions.builtin_path | Partial: CatSystem2 original arc motion capability is not linked | [249](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L249) |
| `condition; under (op.kind.clone(),op.name.as_slice())=(PendingKind::Object,_)` | `method==b"blend"` | self.tasks.fes, self.tasks.fes_object_bank_mut, self.fes_children.motions.blend | Partial: CatSystem2 original builtin motion capability is not linked | [258](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L258) |
| `condition; under (op.kind.clone(),op.name.as_slice())=(PendingKind::Object,_)` | `method==b"stop"` | self.tasks.fes_object_bank_mut, self.fes_children.motions.stop | Partial: CatSystem2 original stop motion capability is not linked | [267](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L267) |
| `condition; under (op.kind.clone(),op.name.as_slice())=(PendingKind::Object,_)` | `crate::fes_text::is_log_method(method)&&{ let logs=self.tasks.logs.clone(); let bank=self.tasks.fes_object_bank_mut(owner)?.context("CatSystem2 object command has no native object bank")?; crate::fes_text::log_command(bank,&op.name,&op.arguments,self.interpreter,self.string_variables,&logs)? }` | Local state/value operations | Empty handler: Original no-op status not established | [275](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L275) |
| `condition; under (op.kind.clone(),op.name.as_slice())=(PendingKind::Object,_)` | `{ let bank=self.tasks.fes_object_bank_mut(owner)?.context("CatSystem2 object command has no native object bank")?; crate::fes_text::applies(bank,&op.name,method,self.interpreter,self.string_variables)? }` | self.tasks.fes_object_bank_mut, crate::fes_text::apply | Handled | [280](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L280) |
| `condition; under (op.kind.clone(),op.name.as_slice())=(PendingKind::Object,_)` | `crate::fes_button::supports(method)&&{ let bank=self.tasks.fes_object_bank_mut(owner)?.context("CatSystem2 object command has no native object bank")?; crate::fes_button::applies(bank,&op.name,method,self.interpreter,self.string_variables)? }` | self.tasks.fes_object_bank_mut, crate::fes_button::apply | Handled | [286](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L286) |
| `condition; under (op.kind.clone(),op.name.as_slice())=(PendingKind::Object,_)` | `crate::fes_sound_table::supports(method)&&{ let bank=self.tasks.fes_object_bank_mut(owner)?.context("CatSystem2 object command has no native object bank")?; crate::fes_sound_table::is_table(bank,&op.name,self.interpreter,self.string_variables)? }` | self.fes_children.motions.ensure_random, self.fes_children.motions.random, self.tasks.fes_object_bank_mut, crate::fes_sound_table::apply | Handled | [292](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L292) |
| `condition; under (op.kind.clone(),op.name.as_slice())=(PendingKind::Object,_)` | `method==b"shuffle"&&!self.fes_children.motions.ensure_random(host,self.client,owner,origin_pc,&op)?` | Local state/value operations | Handled | [297](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L297) |
| `condition; under (op.kind.clone(),op.name.as_slice())=(PendingKind::Object,_)` | `method==b"shuffle"` | self.fes_children.motions.random | Handled | [299](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L299) |
| `condition; under (op.kind.clone(),op.name.as_slice())=(PendingKind::Object,_)` | `matches!(method.as_slice(),b"disp"\|b"size"\|b"pri")` | self.tasks.fes_object_bank_mut | Handled | [307](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L307) |
| `condition; under (op.kind.clone(),op.name.as_slice())=(PendingKind::Object,_)` | `matches!(method.as_slice(),b"m2eff"\|b"m2att"\|b"m2fspeed"\|b"m2prmscale"\|b"m2iner"\|b"m2enable_blink"\|b"m2enable_lip"\|b"m2splay"\|b"m2draw")` | self.tasks.acknowledge_fes_object | Handled | [312](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L312) |
| `condition; under (op.kind.clone(),op.name.as_slice())=(PendingKind::Object,_)` | `method==b"delete"` | self.tasks.fes_object_bank_mut, self.tasks.acknowledge_fes_object | Handled | [318](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L318) |
| `condition; under (op.kind.clone(),op.name.as_slice())=(PendingKind::Native,b"wait")` | `op.arguments[0]==b"frame"` | Local state/value operations | Handled | [356](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L356) |
| `condition; under (op.kind.clone(),op.name.as_slice())=(PendingKind::Native,b"wait")` | `let Some(source)=op.arguments.get(1)` | Local state/value operations | Handled | [358](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L358) |
| `op.arguments.len(); under (op.kind.clone(),op.name.as_slice())=(PendingKind::Native,b"cgreg_t"\|b"cgregnum"\|b"cgregmax")` | `3` | Local state/value operations | Handled | [450](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L450) |
| `op.arguments.len(); under (op.kind.clone(),op.name.as_slice())=(PendingKind::Native,b"cgreg_t"\|b"cgregnum"\|b"cgregmax")` | `_` | Local state/value operations | Handled | [450](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L450) |
| `op.name.as_slice(); under (op.kind.clone(),op.name.as_slice())=(PendingKind::Native,b"cgreg_t"\|b"cgregnum"\|b"cgregmax") / (&self.fes_children.cg,group)=(Some(register),Some(group))` | `_` | Local state/value operations | Handled | [460](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L460) |
| `condition; under (op.kind.clone(),op.name.as_slice())=(PendingKind::Native,b"saveexist"\|b"savedelete"\|b"savecopy"\|b"saveexchg"\|b"newsave"\|b"datasave"\|b"saveapend_str"\|b"saveget_str"\|b"saveget_date") / op.name.as_slice()=b"saveexist"` | `op.arguments.len()>3` | Local state/value operations | Handled | [546](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L546) |
| `op.name.as_slice(); under (op.kind.clone(),op.name.as_slice())=(PendingKind::Native,b"saveexist"\|b"savedelete"\|b"savecopy"\|b"saveexchg"\|b"newsave"\|b"datasave"\|b"saveapend_str"\|b"saveget_str"\|b"saveget_date")` | `_` | Job::begin, save::date, Answer::default | Handled | [570](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L570) |
| `op.name.as_slice(); under (op.kind.clone(),op.name.as_slice())=(PendingKind::Native,b"_redirect"\|b"_redirect_erase"\|b"_redirect_clear")` | `_` | Local state/value operations | Handled | [606](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L606) |
| `op.name.as_slice(); under (op.kind.clone(),op.name.as_slice())=(PendingKind::Native,b"automove"\|b"restoremove"\|b"automoveex")` | `_` | self.tasks.fes_object_bank_mut, crate::fes_references::evaluate, crate::fes_button::key_position, i32::from | Handled | [622](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L622) |
| `condition; under (op.kind.clone(),op.name.as_slice())=(PendingKind::Native,b"automove"\|b"restoremove"\|b"automoveex")` | `op.name!=b"restoremove"` | self.tasks.save_fes_cursor, self.input_state.fes_position | Handled | [631](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L631) |
| `condition; under (op.kind.clone(),op.name.as_slice())=(PendingKind::Native,b"m2_getconf")` | `let (Some(value),Some(source))=(settings.m2.get(key,&label),op.arguments.get(1))` | self.interpreter.clone | Handled | [654](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L654) |
| `op.name.as_slice(); under (op.kind.clone(),op.name.as_slice())=(PendingKind::Native,b"dic_find"\|b"dic_appear"\|b"dic_title"\|b"dic_ind"\|b"dic_max_appear"\|b"dic_update"\|b"dic_update_clear"\|b"dic_id") / op.name.as_slice()=b"dic_appear"\|b"dic_max_appear"\|b"dic_id"` | `_` | Local state/value operations | Handled | [690](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L690) |
| `op.name.as_slice(); under (op.kind.clone(),op.name.as_slice())=(PendingKind::Native,b"dic_find"\|b"dic_appear"\|b"dic_title"\|b"dic_ind"\|b"dic_max_appear"\|b"dic_update"\|b"dic_update_clear"\|b"dic_id")` | `_` | Local state/value operations | Handled | [711](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L711) |
| `condition; under (op.kind.clone(),op.name.as_slice())=(PendingKind::Native,b"sortsave"\|b"sortsave2")` | `op.name==b"sortsave"` | Vec::new | Handled | [723](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L723) |
| `condition; under (op.kind.clone(),op.name.as_slice())=(PendingKind::Native,b"fontcom") / op.arguments.first().map(Vec::as_slice)=Some(b"count")` | `let Some(source)=op.arguments.get(1)` | Local state/value operations | Handled | [747](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L747) |
| `condition; under (op.kind.clone(),op.name.as_slice())=(PendingKind::Native,b"fontcom") / op.arguments.first().map(Vec::as_slice)=Some(b"search")` | `let Some(source)=op.arguments.get(2)` | self.string_variables.assign, Vec::new | Handled | [750](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L750) |
| `op.arguments.first().map(Vec::as_slice); under (op.kind.clone(),op.name.as_slice())=(PendingKind::Native,b"fontcom")` | `Some(b"count")` | Local state/value operations | Handled | [746](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L746) |
| `op.arguments.first().map(Vec::as_slice); under (op.kind.clone(),op.name.as_slice())=(PendingKind::Native,b"fontcom")` | `Some(b"search")` | self.string_variables.assign, Vec::new | Handled | [749](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L749) |
| `op.arguments.first().map(Vec::as_slice); under (op.kind.clone(),op.name.as_slice())=(PendingKind::Native,b"fontcom")` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [752](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L752) |
| `condition; under (op.kind.clone(),op.name.as_slice())=(PendingKind::Native,b"bitcount")` | `let (Some(target),Some(source))=(op.arguments.first(),op.arguments.get(1))` | Local state/value operations | Handled | [768](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L768) |
| `condition; under (op.kind.clone(),op.name.as_slice())=(PendingKind::Native,b"isdebug"\|b"psversion"\|b"inputdevice"\|b"check_install"\|b"vram_get")` | `op.name==b"psversion"` | Local state/value operations | Handled | [777](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L777) |
| `condition; under (op.kind.clone(),op.name.as_slice())=(PendingKind::Native,b"isdebug"\|b"psversion"\|b"inputdevice"\|b"check_install"\|b"vram_get")` | `op.name==b"vram_get"` | Local state/value operations | Handled | [779](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L779) |
| `condition; under (op.kind.clone(),op.name.as_slice())=(PendingKind::Native,b"getexvar")` | `let (Some(name),Some(source))=(op.arguments.first(),op.arguments.get(1))` | self.settings.as_ref, self.interpreter.clone | Handled | [806](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L806) |
| `name.as_slice(); under (op.kind.clone(),op.name.as_slice())=(PendingKind::Native,b"config")` | `b"vol_bgm"` | Local state/value operations | Handled | [828](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L828) |
| `name.as_slice(); under (op.kind.clone(),op.name.as_slice())=(PendingKind::Native,b"config")` | `b"vol_se"` | Local state/value operations | Handled | [828](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L828) |
| `name.as_slice(); under (op.kind.clone(),op.name.as_slice())=(PendingKind::Native,b"config")` | `b"vol_pcm"` | Local state/value operations | Handled | [828](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L828) |
| `name.as_slice(); under (op.kind.clone(),op.name.as_slice())=(PendingKind::Native,b"config")` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [828](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L828) |
| `condition; under (op.kind.clone(),op.name.as_slice())=(PendingKind::Native,b"exit"\|b"result")` | `op.name==b"exit"` | self.tasks.set_task_result | Handled | [890](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L890) |
| `condition; under (op.kind.clone(),op.name.as_slice())=(PendingKind::Native,b"execfes")` | `let Some(wait)=self.exec_fes(host,owner,origin_pc,&op,observation,budget) .with_context(\|\|format!("CatSystem2 execfes at source byte {:#x}; owner native PC {:#x} remains unacknowledged",op.byte_offset,origin_pc))?` | Local state/value operations | Handled | [932](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L932) |
| `condition; under (op.kind.clone(),op.name.as_slice())=(crate::fes_program::PendingKind::Native,b"getsndconfex"\|b"setsndconfex")` | `op.name==b"getsndconfex"` | Local state/value operations | Handled | [954](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L954) |
| `condition; under (op.kind.clone(),op.name.as_slice())=(crate::fes_program::PendingKind::Native,b"getconf"\|b"setconf")` | `let Some(v)=op.arguments.get(2)` | Local state/value operations | Handled | [972](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L972) |
| `condition; under (op.kind.clone(),op.name.as_slice())=(crate::fes_program::PendingKind::Native,b"getconf"\|b"setconf")` | `let Some(v)=op.arguments.get(3)` | Local state/value operations | Handled | [973](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L973) |
| `condition; under (op.kind.clone(),op.name.as_slice())=(crate::fes_program::PendingKind::Native,b"getconf"\|b"setconf")` | `op.name==b"getconf"` | Local state/value operations | Handled | [976](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L976) |
| `condition; under (op.kind.clone(),op.name.as_slice())=(crate::fes_program::PendingKind::Native,b"getconf"\|b"setconf")` | `name.starts_with(b"vol_")\|\|name.starts_with(b"mute_")` | host.submit_audio | Handled | [982](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L982) |
| `(op.kind.clone(),op.name.as_slice())` | `(PendingKind::Object,_)` | crate::fes_save_panel::supports, self.interpreter.clone, self.tasks.fes_object_bank_mut, crate::fes_bg_table::applies, crate::fes_bg_table::apply | Partial: CatSystem2 unsupported sndtag command {} / CatSystem2 original arc motion capability is not linked / CatSystem2 original builtin motion capability is… | [105](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L105) |
| `(op.kind.clone(),op.name.as_slice())` | `(PendingKind::BlankWait\|PendingKind::SectionRestart,_)` | self.tasks.begin_fes_wait | Handled | [346](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L346) |
| `(op.kind.clone(),op.name.as_slice())` | `_` | Local state/value operations | Partial: CatSystem2 unsupported FES operation {} ({:?}) at {} section {} row {} physical line {} byte {:#x}; owner native PC {:#x} remains unacknowledged | [1016](../../crates/crysirror-catsystem2/src/fes_runtime.rs#L1016) |

### fes_save_panel.rs / apply

```rust
fn apply(
    host: &mut dyn HostServices,
    client: ClientId,
    pool: &TaskPool,
    owner: u32,
    native_pc: u32,
    op: &PendingOperation,
    panels: &mut Panels,
    tasks: &mut NativeTaskPolicy,
    vm: &mut Interpreter,
    strings: &mut StringVariables,
) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(method.as_slice(), b"make" \| b"get")` | host.storage | Handled | [123](../../crates/crysirror-catsystem2/src/fes_save_panel.rs#L123) |
| `condition` | `matches!(method.as_slice(), b"getflag" \| b"setsave")` | Local state/value operations | Handled | [128](../../crates/crysirror-catsystem2/src/fes_save_panel.rs#L128) |
| `condition` | `bank.objects.get(id).is_none_or(\|object\| object.declared_type != 12)` | Local state/value operations | Handled | [132](../../crates/crysirror-catsystem2/src/fes_save_panel.rs#L132) |
| `condition; under method.as_slice()=b"ptype"` | `bank.objects.get(&id).is_none_or(\|object\| object.declared_type != 12)` | Local state/value operations | Handled | [141](../../crates/crysirror-catsystem2/src/fes_save_panel.rs#L141) |
| `condition; under method.as_slice()=b"setnum"` | `kind == 12` | Local state/value operations | Handled | [160](../../crates/crysirror-catsystem2/src/fes_save_panel.rs#L160) |
| `condition; under method.as_slice()=b"getflag"` | `bank.objects.get(id).is_some_and(\|object\| object.declared_type == 12)` | Local state/value operations | Handled | [215](../../crates/crysirror-catsystem2/src/fes_save_panel.rs#L215) |
| `method.as_slice()` | `b"ptype"` | crate::fes_panel_parts::parse | Handled | [137](../../crates/crysirror-catsystem2/src/fes_save_panel.rs#L137) |
| `method.as_slice()` | `b"setnum"` | Local state/value operations | Handled | [152](../../crates/crysirror-catsystem2/src/fes_save_panel.rs#L152) |
| `method.as_slice()` | `b"dispid"` | Local state/value operations | Handled | [174](../../crates/crysirror-catsystem2/src/fes_save_panel.rs#L174) |
| `method.as_slice()` | `b"setsave"` | Local state/value operations | Handled | [198](../../crates/crysirror-catsystem2/src/fes_save_panel.rs#L198) |
| `method.as_slice()` | `b"getflag"` | Local state/value operations | Handled | [209](../../crates/crysirror-catsystem2/src/fes_save_panel.rs#L209) |
| `method.as_slice()` | `b"getsavestr"` | Vec::new | Handled | [221](../../crates/crysirror-catsystem2/src/fes_save_panel.rs#L221) |
| `method.as_slice()` | `b"getsavedate"` | Local state/value operations | Handled | [230](../../crates/crysirror-catsystem2/src/fes_save_panel.rs#L230) |
| `method.as_slice()` | `other` | Local state/value operations | Partial: CatSystem2 SAVEPANEL method {} is not implemented | [238](../../crates/crysirror-catsystem2/src/fes_save_panel.rs#L238) |

### fes_save_panel.rs / supports

```rust
fn supports(method: &[u8]) -> bool
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(method, b"make" \| b"get" \| b"setnum" \| b"setsave" \| b"getflag" \| b"getsavestr" \| b"getsavedate" \| b"ptype" \| b"dispid")` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [22](../../crates/crysirror-catsystem2/src/fes_save_panel.rs#L22) |

### fes_sound_commands.rs / apply

```rust
fn apply(bank:&mut Bank,target:&[u8],arguments:&[Vec<u8>],vm:&mut Interpreter,strings:&StringVariables) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `reference.index==-1&&matches!(method.as_slice(),b"play"\|b"loop"\|b"pause")` | Local state/value operations | Handled | [47](../../crates/crysirror-catsystem2/src/fes_sound_commands.rs#L47) |
| `condition` | `matches!(method.as_slice(),b"channel"\|b"stype"\|b"crossfade")` | Local state/value operations | Handled | [60](../../crates/crysirror-catsystem2/src/fes_sound_commands.rs#L60) |
| `method.as_slice()` | `b"channel"` | Local state/value operations | Handled | [61](../../crates/crysirror-catsystem2/src/fes_sound_commands.rs#L61) |
| `method.as_slice()` | `b"stype"` | Local state/value operations | Handled | [61](../../crates/crysirror-catsystem2/src/fes_sound_commands.rs#L61) |
| `method.as_slice()` | `_` | Local state/value operations | Handled | [61](../../crates/crysirror-catsystem2/src/fes_sound_commands.rs#L61) |
| `method.as_slice()` | `b"channel"` | Local state/value operations | Handled | [62](../../crates/crysirror-catsystem2/src/fes_sound_commands.rs#L62) |
| `method.as_slice()` | `b"stype"` | Local state/value operations | Handled | [62](../../crates/crysirror-catsystem2/src/fes_sound_commands.rs#L62) |
| `method.as_slice()` | `_` | Local state/value operations | Handled | [62](../../crates/crysirror-catsystem2/src/fes_sound_commands.rs#L62) |

### fes_sound_commands.rs / mutate

```rust
fn mutate(sound:&mut SoundState,method:&[u8],args:&[Vec<u8>],vm:&mut Interpreter,strings:&StringVariables) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `method` | `b"load"` | Vec::new | Handled | [14](../../crates/crysirror-catsystem2/src/fes_sound_commands.rs#L14) |
| `method` | `b"play"` | Local state/value operations | Handled | [24](../../crates/crysirror-catsystem2/src/fes_sound_commands.rs#L24) |
| `method` | `b"channel"` | Local state/value operations | Handled | [32](../../crates/crysirror-catsystem2/src/fes_sound_commands.rs#L32) |
| `method` | `b"stype"` | Local state/value operations | Handled | [33](../../crates/crysirror-catsystem2/src/fes_sound_commands.rs#L33) |
| `method` | `b"loop"` | Local state/value operations | Handled | [34](../../crates/crysirror-catsystem2/src/fes_sound_commands.rs#L34) |
| `method` | `b"pause"` | Local state/value operations | Handled | [35](../../crates/crysirror-catsystem2/src/fes_sound_commands.rs#L35) |
| `method` | `b"crossfade"` | Local state/value operations | Handled | [36](../../crates/crysirror-catsystem2/src/fes_sound_commands.rs#L36) |
| `method` | `_` | Local state/value operations | Handled | [37](../../crates/crysirror-catsystem2/src/fes_sound_commands.rs#L37) |

### fes_sound_commands.rs / supports

```rust
fn supports(method:&[u8]) -> bool
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(method,b"load"\|b"play"\|b"channel"\|b"stype"\|b"loop"\|b"pause"\|b"crossfade")` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [7](../../crates/crysirror-catsystem2/src/fes_sound_commands.rs#L7) |

### fes_sound_table.rs / apply

```rust
fn apply(
    bank: &mut Bank,
    target: &[u8],
    arguments: &[Vec<u8>],
    vm: &mut Interpreter,
    strings: &StringVariables,
    random: Option<&mut RandomStreams>,
    client: ClientId,
) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `reference.index == -1 && matches!(method.as_slice(), b"getid" \| b"next" \| b"rev" \| b"repeat" \| b"shuffle")` | Local state/value operations | Handled | [180](../../crates/crysirror-catsystem2/src/fes_sound_table.rs#L180) |
| `condition` | `method == b"sndtbl"` | Vec::new | Handled | [186](../../crates/crysirror-catsystem2/src/fes_sound_table.rs#L186) |
| `condition` | `method == b"setid"` | Local state/value operations | Handled | [195](../../crates/crysirror-catsystem2/src/fes_sound_table.rs#L195) |
| `condition` | `method == b"getid"` | Local state/value operations | Handled | [196](../../crates/crysirror-catsystem2/src/fes_sound_table.rs#L196) |
| `condition` | `method == b"shuffle"` | Local state/value operations | Handled | [197](../../crates/crysirror-catsystem2/src/fes_sound_table.rs#L197) |
| `condition` | `method == b"repeat"` | Local state/value operations | Handled | [202](../../crates/crysirror-catsystem2/src/fes_sound_table.rs#L202) |
| `method.as_slice()` | `b"sndtbl"` | usize::try_from | Handled | [214](../../crates/crysirror-catsystem2/src/fes_sound_table.rs#L214) |
| `method.as_slice()` | `b"setid"` | Local state/value operations | Handled | [220](../../crates/crysirror-catsystem2/src/fes_sound_table.rs#L220) |
| `method.as_slice()` | `b"getid"` | Local state/value operations | Handled | [221](../../crates/crysirror-catsystem2/src/fes_sound_table.rs#L221) |
| `method.as_slice()` | `b"next"` | Local state/value operations | Handled | [224](../../crates/crysirror-catsystem2/src/fes_sound_table.rs#L224) |
| `method.as_slice()` | `b"rev"` | Local state/value operations | Handled | [228](../../crates/crysirror-catsystem2/src/fes_sound_table.rs#L228) |
| `method.as_slice()` | `b"repeat"` | Local state/value operations | Handled | [229](../../crates/crysirror-catsystem2/src/fes_sound_table.rs#L229) |
| `method.as_slice()` | `b"shuffle"` | Local state/value operations | Handled | [230](../../crates/crysirror-catsystem2/src/fes_sound_table.rs#L230) |
| `method.as_slice()` | `_` | Local state/value operations | Handled | [238](../../crates/crysirror-catsystem2/src/fes_sound_table.rs#L238) |

### fes_sound_table.rs / supports

```rust
fn supports(method: &[u8]) -> bool
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(method, b"sndtbl" \| b"setid" \| b"getid" \| b"next" \| b"rev" \| b"repeat" \| b"shuffle")` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [152](../../crates/crysirror-catsystem2/src/fes_sound_table.rs#L152) |

### fes_strings.rs / StringVariables::expand

```rust
fn expand(&self, source: &[u8], limit: usize) -> Result<Vec<u8>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Some((n, id)) = crate::native_values::integer_prefix(&source[at + 4..])` | self.get | Handled | [19](../../crates/crysirror-catsystem2/src/fes_strings.rs#L19) |

### fes_text.rs / apply

```rust
fn apply(bank: &mut Bank, target: &[u8], arguments: &[Vec<u8>], raw: &[u8], vm: &mut Interpreter, strings: &StringVariables) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `method==b"mag"` | Local state/value operations | Handled | [219](../../crates/crysirror-catsystem2/src/fes_text.rs#L219) |
| `method.as_slice(); under method.as_slice()=b"page" \| b"pos" \| b"getpos" if matches!(&object.state, State::Image(image) if image.slider.is_some())` | `b"page"` | Local state/value operations | Handled | [243](../../crates/crysirror-catsystem2/src/fes_text.rs#L243) |
| `method.as_slice(); under method.as_slice()=b"page" \| b"pos" \| b"getpos" if matches!(&object.state, State::Image(image) if image.slider.is_some())` | `b"pos"` | Local state/value operations | Handled | [248](../../crates/crysirror-catsystem2/src/fes_text.rs#L248) |
| `method.as_slice(); under method.as_slice()=b"page" \| b"pos" \| b"getpos" if matches!(&object.state, State::Image(image) if image.slider.is_some())` | `_` | Local state/value operations | Handled | [254](../../crates/crysirror-catsystem2/src/fes_text.rs#L254) |
| `condition; under method.as_slice()=b"pos" \| b"getpos" \| b"getpos2" \| b"getsize"` | `method == b"pos"` | Local state/value operations | Handled | [269](../../crates/crysirror-catsystem2/src/fes_text.rs#L269) |
| `condition; under method.as_slice()=b"pos" \| b"getpos" \| b"getpos2" \| b"getsize"` | `method != b"getsize"` | Local state/value operations | Handled | [274](../../crates/crysirror-catsystem2/src/fes_text.rs#L274) |
| `method.as_slice(); under method.as_slice()=_` | `b"layout"` | Local state/value operations | Handled | [290](../../crates/crysirror-catsystem2/src/fes_text.rs#L290) |
| `method.as_slice(); under method.as_slice()=_` | `b"strid"` | Local state/value operations | Handled | [291](../../crates/crysirror-catsystem2/src/fes_text.rs#L291) |
| `method.as_slice(); under method.as_slice()=_` | `b"mag"` | Local state/value operations | Handled | [298](../../crates/crysirror-catsystem2/src/fes_text.rs#L298) |
| `method.as_slice(); under method.as_slice()=_` | `b"str"` | Local state/value operations | Handled | [304](../../crates/crysirror-catsystem2/src/fes_text.rs#L304) |
| `method.as_slice(); under method.as_slice()=_` | `b"apend"` | Local state/value operations | Handled | [308](../../crates/crysirror-catsystem2/src/fes_text.rs#L308) |
| `method.as_slice(); under method.as_slice()=_` | `b"apendmark"` | Local state/value operations | Handled | [314](../../crates/crysirror-catsystem2/src/fes_text.rs#L314) |
| `method.as_slice(); under method.as_slice()=_` | `b"insert"` | core::mem::take | Handled | [315](../../crates/crysirror-catsystem2/src/fes_text.rs#L315) |
| `method.as_slice(); under method.as_slice()=_` | `b"clear"` | Local state/value operations | Handled | [329](../../crates/crysirror-catsystem2/src/fes_text.rs#L329) |
| `method.as_slice(); under method.as_slice()=_` | `b"reset"` | Vec::new, core::mem::take, Default::default | Handled | [334](../../crates/crysirror-catsystem2/src/fes_text.rs#L334) |
| `method.as_slice(); under method.as_slice()=_` | `b"draw"` | Local state/value operations | Handled | [338](../../crates/crysirror-catsystem2/src/fes_text.rs#L338) |
| `method.as_slice(); under method.as_slice()=_` | `b"skip"` | Local state/value operations | Handled | [343](../../crates/crysirror-catsystem2/src/fes_text.rs#L343) |
| `method.as_slice(); under method.as_slice()=_` | `b"isdraw"` | u32::from | Handled | [344](../../crates/crysirror-catsystem2/src/fes_text.rs#L344) |
| `method.as_slice(); under method.as_slice()=_` | `b"endpos"` | Local state/value operations | Handled | [347](../../crates/crysirror-catsystem2/src/fes_text.rs#L347) |
| `method.as_slice(); under method.as_slice()=_` | `b"linecnt"` | Local state/value operations | Handled | [353](../../crates/crysirror-catsystem2/src/fes_text.rs#L353) |
| `method.as_slice(); under method.as_slice()=_` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [361](../../crates/crysirror-catsystem2/src/fes_text.rs#L361) |
| `method.as_slice()` | `b"pos2"` | crate::native_class_state::MetadataReference::parse | Handled | [223](../../crates/crysirror-catsystem2/src/fes_text.rs#L223) |
| `method.as_slice()` | `b"page" \| b"pos" \| b"getpos" if matches!(&object.state, State::Image(image) if image.slider.is_some())` | Local state/value operations | Handled | [239](../../crates/crysirror-catsystem2/src/fes_text.rs#L239) |
| `method.as_slice()` | `b"page"` | Local state/value operations | Empty handler: Original no-op status not established | [262](../../crates/crysirror-catsystem2/src/fes_text.rs#L262) |
| `method.as_slice()` | `b"pos" \| b"getpos" \| b"getpos2" \| b"getsize"` | Local state/value operations | Handled | [263](../../crates/crysirror-catsystem2/src/fes_text.rs#L263) |
| `method.as_slice()` | `_` | core::mem::take, Vec::new, Default::default, u32::from | Handled | [286](../../crates/crysirror-catsystem2/src/fes_text.rs#L286) |

### fes_text.rs / is_common_method

```rust
fn is_common_method(method: &[u8]) -> bool
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(method, b"pos" \| b"pos2" \| b"getpos" \| b"getpos2" \| b"getsize" \| b"page")` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [72](../../crates/crysirror-catsystem2/src/fes_text.rs#L72) |

### fes_text.rs / is_log_method

```rust
fn is_log_method(method: &[u8]) -> bool
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(method, b"setlog" \| b"remake" \| b"max" \| b"lineinfo" \| b"pathinfo" \| b"blockinfo" \| b"blockcolor" \| b"getpos" \| b"pos" \| b"strid")` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [85](../../crates/crysirror-catsystem2/src/fes_text.rs#L85) |

### fes_text.rs / is_text_method

```rust
fn is_text_method(method: &[u8]) -> bool
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(method, b"layout" \| b"str" \| b"apend" \| b"apendmark" \| b"clear" \| b"reset" \| b"draw" \| b"skip" \| b"insert" \| b"isdraw" \| b"endpos" \| b"linecnt" \| b"userfont" \| b"userfontobj" \| b"delay" \| b"mag" \| b"strid")` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [68](../../crates/crysirror-catsystem2/src/fes_text.rs#L68) |

### fes_text.rs / layout

```rust
fn layout(text: &mut TextObject, args: &[Vec<u8>], vm: &mut Interpreter) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `key.as_slice()` | `b"size"` | Local state/value operations | Handled | [384](../../crates/crysirror-catsystem2/src/fes_text.rs#L384) |
| `key.as_slice()` | `b"margin"` | Local state/value operations | Handled | [391](../../crates/crysirror-catsystem2/src/fes_text.rs#L391) |
| `key.as_slice()` | `b"frame"` | Local state/value operations | Handled | [398](../../crates/crysirror-catsystem2/src/fes_text.rs#L398) |
| `key.as_slice()` | `b"move"` | Local state/value operations | Handled | [399](../../crates/crysirror-catsystem2/src/fes_text.rs#L399) |
| `key.as_slice()` | `b"color"` | Local state/value operations | Handled | [406](../../crates/crysirror-catsystem2/src/fes_text.rs#L406) |
| `key.as_slice()` | `b"indent"` | Local state/value operations | Handled | [413](../../crates/crysirror-catsystem2/src/fes_text.rs#L413) |
| `key.as_slice()` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [416](../../crates/crysirror-catsystem2/src/fes_text.rs#L416) |

### fes_text.rs / log_command

```rust
fn log_command(bank: &mut Bank, target: &[u8], arguments: &[Vec<u8>], vm: &mut Interpreter, strings: &StringVariables, logs: &crate::native_log::Logs) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under method.as_slice()=b"remake" \| b"pos"` | `method == b"pos"` | Local state/value operations | Handled | [132](../../crates/crysirror-catsystem2/src/fes_text.rs#L132) |
| `condition; under method.as_slice()=b"max" \| b"getpos"` | `method == b"max"` | Local state/value operations | Handled | [162](../../crates/crysirror-catsystem2/src/fes_text.rs#L162) |
| `condition; under method.as_slice()=b"lineinfo" \| b"pathinfo" \| b"blockinfo"` | `method == b"blockinfo"` | Local state/value operations | Handled | [170](../../crates/crysirror-catsystem2/src/fes_text.rs#L170) |
| `method.as_slice()` | `b"setlog"` | Local state/value operations | Handled | [127](../../crates/crysirror-catsystem2/src/fes_text.rs#L127) |
| `method.as_slice()` | `b"remake" \| b"pos"` | Local state/value operations | Handled | [131](../../crates/crysirror-catsystem2/src/fes_text.rs#L131) |
| `method.as_slice()` | `b"strid"` | Vec::new, crysirror_resource::text_stream::decode_auto | Handled | [144](../../crates/crysirror-catsystem2/src/fes_text.rs#L144) |
| `method.as_slice()` | `b"max" \| b"getpos"` | Local state/value operations | Handled | [161](../../crates/crysirror-catsystem2/src/fes_text.rs#L161) |
| `method.as_slice()` | `b"lineinfo" \| b"pathinfo" \| b"blockinfo"` | Local state/value operations | Handled | [167](../../crates/crysirror-catsystem2/src/fes_text.rs#L167) |
| `method.as_slice()` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [174](../../crates/crysirror-catsystem2/src/fes_text.rs#L174) |

### kcs.rs / Machine::instruction

```rust
fn instruction(&mut self, program: &Program) -> Result<Option<Event>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under op=1..=0x14` | `matches!(op, 8 \| 0xc)` | Local state/value operations | Handled | [487](../../crates/crysirror-catsystem2/src/kcs.rs#L487) |
| `condition; under op=1..=0x14` | `matches!(op, 8 \| 0xc)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [487](../../crates/crysirror-catsystem2/src/kcs.rs#L487) |
| `condition; under op=1..=0x14` | `op <= 4` | Local state/value operations | Handled | [494](../../crates/crysirror-catsystem2/src/kcs.rs#L494) |
| `op; under op=1..=0x14` | `1 if float` | Local state/value operations | Handled | [501](../../crates/crysirror-catsystem2/src/kcs.rs#L501) |
| `op; under op=1..=0x14` | `2 if float` | Local state/value operations | Handled | [503](../../crates/crysirror-catsystem2/src/kcs.rs#L503) |
| `op; under op=1..=0x14` | `3 if float` | Local state/value operations | Handled | [505](../../crates/crysirror-catsystem2/src/kcs.rs#L505) |
| `op; under op=1..=0x14` | `4 if float` | Local state/value operations | Handled | [507](../../crates/crysirror-catsystem2/src/kcs.rs#L507) |
| `op; under op=1..=0x14` | `4 if signed` | Local state/value operations | Handled | [514](../../crates/crysirror-catsystem2/src/kcs.rs#L514) |
| `op; under op=1..=0x14` | `_` | Local state/value operations | Handled | [602](../../crates/crysirror-catsystem2/src/kcs.rs#L602) |
| `condition; under op=0x16..=0x19` | `kind == 4` | f32::from_bits | Handled | [629](../../crates/crysirror-catsystem2/src/kcs.rs#L629) |
| `kind; under op=0x16..=0x19` | `1` | Local state/value operations | Handled | [637](../../crates/crysirror-catsystem2/src/kcs.rs#L637) |
| `kind; under op=0x16..=0x19` | `2` | Local state/value operations | Handled | [638](../../crates/crysirror-catsystem2/src/kcs.rs#L638) |
| `kind; under op=0x16..=0x19` | `_` | Local state/value operations | Handled | [639](../../crates/crysirror-catsystem2/src/kcs.rs#L639) |
| `condition; under op=0x16..=0x19` | `op >= 0x18` | Local state/value operations | Handled | [642](../../crates/crysirror-catsystem2/src/kcs.rs#L642) |
| `op; under op=0x1b..=0x24` | `0x1b if y == 0` | Local state/value operations | Handled | [663](../../crates/crysirror-catsystem2/src/kcs.rs#L663) |
| `op; under op=0x1b..=0x24` | `0x1b if signed` | Local state/value operations | Handled | [664](../../crates/crysirror-catsystem2/src/kcs.rs#L664) |
| `op; under op=0x1b..=0x24` | `0x1d if float` | f32::from_bits | Handled | [670](../../crates/crysirror-catsystem2/src/kcs.rs#L670) |
| `op; under op=0x1b..=0x24` | `0x1e if float` | f32::from_bits | Handled | [672](../../crates/crysirror-catsystem2/src/kcs.rs#L672) |
| `op; under op=0x1b..=0x24` | `0x1f if float` | f32::from_bits | Handled | [674](../../crates/crysirror-catsystem2/src/kcs.rs#L674) |
| `op; under op=0x1b..=0x24` | `0x20 if float` | f32::from_bits | Handled | [676](../../crates/crysirror-catsystem2/src/kcs.rs#L676) |
| `op; under op=0x1b..=0x24` | `0x20 if y == 0` | Local state/value operations | Handled | [685](../../crates/crysirror-catsystem2/src/kcs.rs#L685) |
| `op; under op=0x1b..=0x24` | `0x20 if signed` | Local state/value operations | Handled | [686](../../crates/crysirror-catsystem2/src/kcs.rs#L686) |
| `op; under op=0x1b..=0x24` | `0x22 if signed` | Local state/value operations | Handled | [692](../../crates/crysirror-catsystem2/src/kcs.rs#L692) |
| `op; under op=0x1b..=0x24` | `_` | Local state/value operations | Handled | [696](../../crates/crysirror-catsystem2/src/kcs.rs#L696) |
| `condition; under op=0x27 \| 0x28` | `(op == 0x27 && cond == 0) \|\| (op == 0x28 && cond != 0)` | self.jump | Handled | [732](../../crates/crysirror-catsystem2/src/kcs.rs#L732) |
| `op; under op=0x36..=0x39` | `_` | f32::from_bits | Handled | [879](../../crates/crysirror-catsystem2/src/kcs.rs#L879) |
| `op` | `_` | Local state/value operations | Unsupported: unsupported KCS opcode {op:#04x} at native PC {original:#x} | [908](../../crates/crysirror-catsystem2/src/kcs.rs#L908) |

### kcs.rs / Machine::operand

```rust
fn operand(&mut self, code: &[u8]) -> Result<Operand>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `op.flags & 0x10 != 0` | self.region, u16::from_le_bytes, i16::from_le_bytes, self.read_word | Handled | [347](../../crates/crysirror-catsystem2/src/kcs.rs#L347) |
| `(op.flags as i8 >= 0, op.flags & 15, op.flags & 0x20 != 0)` | `(true, 1, false)` | self.region | Handled | [349](../../crates/crysirror-catsystem2/src/kcs.rs#L349) |
| `(op.flags as i8 >= 0, op.flags & 15, op.flags & 0x20 != 0)` | `(true, 1, true)` | self.region | Handled | [350](../../crates/crysirror-catsystem2/src/kcs.rs#L350) |
| `(op.flags as i8 >= 0, op.flags & 15, op.flags & 0x20 != 0)` | `(true, 2, false)` | u16::from_le_bytes, self.region | Handled | [351](../../crates/crysirror-catsystem2/src/kcs.rs#L351) |
| `(op.flags as i8 >= 0, op.flags & 15, op.flags & 0x20 != 0)` | `(true, 2, true)` | i16::from_le_bytes, self.region | Handled | [354](../../crates/crysirror-catsystem2/src/kcs.rs#L354) |
| `(op.flags as i8 >= 0, op.flags & 15, op.flags & 0x20 != 0)` | `_` | self.read_word | Handled | [357](../../crates/crysirror-catsystem2/src/kcs.rs#L357) |

### kcs.rs / Machine::store

```rust
fn store(&mut self, op: Operand, value: u32) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `op.flags as i8 >= 0` | Local state/value operations | Handled | [363](../../crates/crysirror-catsystem2/src/kcs.rs#L363) |
| `op.flags & 15` | `1` | Local state/value operations | Handled | [365](../../crates/crysirror-catsystem2/src/kcs.rs#L365) |
| `op.flags & 15` | `2` | Local state/value operations | Handled | [366](../../crates/crysirror-catsystem2/src/kcs.rs#L366) |
| `op.flags & 15` | `_` | Local state/value operations | Handled | [367](../../crates/crysirror-catsystem2/src/kcs.rs#L367) |

### loop_policy.rs / walk_rules

```rust
fn walk_rules(
    doc: &XmlDocument,
    node: &XmlNode,
    rules: &mut Vec<(String, String)>,
    depth: usize,
) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Some(part) = doc.children(node)?.find(\|n\| n.name == "part")` | Local state/value operations | Handled | [50](../../crates/crysirror-catsystem2/src/loop_policy.rs#L50) |

### loop_policy.rs / SoundPaths::legacy

```rust
fn legacy(startup: &[u8]) -> Result<Self>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Some(letter) = name.strip_prefix("pcm_")` | Local state/value operations | Handled | [84](../../crates/crysirror-catsystem2/src/loop_policy.rs#L84) |
| `condition` | `matches!(name.as_str(), "bgm" \| "se")` | Local state/value operations | Handled | [86](../../crates/crysirror-catsystem2/src/loop_policy.rs#L86) |

### loop_policy.rs / SoundPaths::parse_archives

```rust
fn parse_archives(startup: &[u8]) -> Result<Self>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!row.name.starts_with("int")` | Local state/value operations | Handled | [116](../../crates/crysirror-catsystem2/src/loop_policy.rs#L116) |

### motion.rs / scalar

```rust
fn scalar(id: u32, words: &[u32]) -> Result<u32>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `id` | `1..=6\|8..=10\|12\|14..=17` | Local state/value operations | Handled | [498](../../crates/crysirror-catsystem2/src/motion.rs#L498) |
| `id` | `7\|11\|13\|42` | Local state/value operations | Handled | [498](../../crates/crysirror-catsystem2/src/motion.rs#L498) |
| `id` | `41` | Local state/value operations | Handled | [498](../../crates/crysirror-catsystem2/src/motion.rs#L498) |
| `id` | `_` | Local state/value operations | Partial: unsupported original motion native family {id}; shared RNG/debug mappings are not fabricated | [499](../../crates/crysirror-catsystem2/src/motion.rs#L499) |
| `id` | `1` | f64::from | Handled | [507](../../crates/crysirror-catsystem2/src/motion.rs#L507) |
| `id` | `2` | f64::from | Handled | [508](../../crates/crysirror-catsystem2/src/motion.rs#L508) |
| `id` | `3` | f64::from | Handled | [509](../../crates/crysirror-catsystem2/src/motion.rs#L509) |
| `id` | `4` | f64::from | Handled | [510](../../crates/crysirror-catsystem2/src/motion.rs#L510) |
| `id` | `5` | f64::from | Handled | [511](../../crates/crysirror-catsystem2/src/motion.rs#L511) |
| `id` | `6` | f64::from | Handled | [512](../../crates/crysirror-catsystem2/src/motion.rs#L512) |
| `id` | `7` | f64::from | Handled | [513](../../crates/crysirror-catsystem2/src/motion.rs#L513) |
| `id` | `8` | f64::from | Handled | [514](../../crates/crysirror-catsystem2/src/motion.rs#L514) |
| `id` | `9` | Local state/value operations | Handled | [515](../../crates/crysirror-catsystem2/src/motion.rs#L515) |
| `id` | `10` | f64::from | Handled | [516](../../crates/crysirror-catsystem2/src/motion.rs#L516) |
| `id` | `11` | f64::from | Handled | [517](../../crates/crysirror-catsystem2/src/motion.rs#L517) |
| `id` | `12` | f64::from | Handled | [518](../../crates/crysirror-catsystem2/src/motion.rs#L518) |
| `id` | `13` | f64::from | Handled | [519](../../crates/crysirror-catsystem2/src/motion.rs#L519) |
| `id` | `14` | Local state/value operations | Handled | [520](../../crates/crysirror-catsystem2/src/motion.rs#L520) |
| `id` | `15` | Local state/value operations | Handled | [521](../../crates/crysirror-catsystem2/src/motion.rs#L521) |
| `id` | `16` | Local state/value operations | Handled | [522](../../crates/crysirror-catsystem2/src/motion.rs#L522) |
| `id` | `17` | f64::from | Handled | [523](../../crates/crysirror-catsystem2/src/motion.rs#L523) |
| `id` | `41` | Local state/value operations | Handled | [524](../../crates/crysirror-catsystem2/src/motion.rs#L524) |
| `id` | `42` | f64::from | Handled | [539](../../crates/crysirror-catsystem2/src/motion.rs#L539) |
| `id` | `_` | Local state/value operations | Handled | [569](../../crates/crysirror-catsystem2/src/motion.rs#L569) |

### motion.rs / Controller::service_native

```rust
fn service_native(
        &mut self,
        call: &NativeCall,
        services: Option<&mut Services<'_>>,
    ) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under call.id=39 \| 40` | `call.id == 39` | Local state/value operations | Handled | [425](../../crates/crysirror-catsystem2/src/motion.rs#L425) |
| `call.id` | `_` | Local state/value operations | Handled | [470](../../crates/crysirror-catsystem2/src/motion.rs#L470) |

### motion.rs / Controller::tick_inner

```rust
fn tick_inner(
        &mut self,
        clock: u32,
        mut budget: u32,
        mut services: Option<Services<'_>>,
        terminal: bool,
    ) -> Result<Tick>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under event=Event::Native(call)` | `matches!(call.id, 0 \| 39 \| 40)` | self.service_native | Handled | [372](../../crates/crysirror-catsystem2/src/motion.rs#L372) |
| `condition; under event=Event::Native(call)` | `matches!(call.id, 0 \| 39 \| 40)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [372](../../crates/crysirror-catsystem2/src/motion.rs#L372) |
| `condition; under event=Event::Native(call)` | `call.id == 38` | self.machine.preflight_native, self.machine.acknowledge, self.ready | Handled | [383](../../crates/crysirror-catsystem2/src/motion.rs#L383) |

### motion_math.rs / prepare

```rust
fn prepare(machine: &Machine, call: &NativeCall) -> Result<(Option<u32>, Writes)>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `call.id` | `_` | Local state/value operations | Partial: unsupported CatSystem2 motion geometric native {} | [25](../../crates/crysirror-catsystem2/src/motion_math.rs#L25) |
| `call.id; under call.id=18..=20` | `_` | Mat3::from_rotation_z | Handled | [52](../../crates/crysirror-catsystem2/src/motion_math.rs#L52) |
| `condition; under call.id=21..=22` | `call.id == 21` | Local state/value operations | Handled | [73](../../crates/crysirror-catsystem2/src/motion_math.rs#L73) |
| `call.id; under call.id=23..=25 \| 43` | `_` | Mat4::from_rotation_y, Mat4::from_rotation_x, Mat4::from_rotation_z | Handled | [98](../../crates/crysirror-catsystem2/src/motion_math.rs#L98) |
| `condition; under call.id=29..=30` | `call.id == 29` | Local state/value operations | Handled | [128](../../crates/crysirror-catsystem2/src/motion_math.rs#L128) |
| `condition; under call.id=32..=33` | `call.id == 32` | Local state/value operations | Handled | [168](../../crates/crysirror-catsystem2/src/motion_math.rs#L168) |
| `condition; under call.id=34..=35` | `call.id == 34` | f64::from | Handled | [181](../../crates/crysirror-catsystem2/src/motion_math.rs#L181) |
| `call.id` | `_` | Local state/value operations | Handled | [217](../../crates/crysirror-catsystem2/src/motion_math.rs#L217) |

### motion_set.rs / MotionSet::set_start

```rust
fn set_start(&mut self, id: u32, clock: u32) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `id == 0` | self.entries.values_mut | Handled | [200](../../crates/crysirror-catsystem2/src/motion_set.rs#L200) |

### motion_set.rs / MotionSet::skip

```rust
fn skip(&mut self, id: u32) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Some(e) = self.entries.get_mut(&id).filter(\|_\| id != 0)` | Local state/value operations | Handled | [247](../../crates/crysirror-catsystem2/src/motion_set.rs#L247) |

### native_class_state.rs / State::new

```rust
fn new(kind: u8) -> Result<Self>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `kind` | `0 \| 5 \| 35` | Local state/value operations | Handled | [390](../../crates/crysirror-catsystem2/src/native_class_state.rs#L390) |
| `kind` | `1` | Self::CImage, Default::default | Handled | [391](../../crates/crysirror-catsystem2/src/native_class_state.rs#L391) |
| `kind` | `2` | Self::Image, Default::default | Handled | [392](../../crates/crysirror-catsystem2/src/native_class_state.rs#L392) |
| `kind` | `6` | Self::Image, Default::default | Handled | [393](../../crates/crysirror-catsystem2/src/native_class_state.rs#L393) |
| `kind` | `3` | Self::CPlane, Default::default | Handled | [394](../../crates/crysirror-catsystem2/src/native_class_state.rs#L394) |
| `kind` | `4` | Self::Plane, Default::default | Handled | [395](../../crates/crysirror-catsystem2/src/native_class_state.rs#L395) |
| `kind` | `25 \| 11 \| 12` | Self::Image, Default::default | Handled | [397](../../crates/crysirror-catsystem2/src/native_class_state.rs#L397) |
| `kind` | `16 \| 17` | Self::Image, Default::default | Handled | [398](../../crates/crysirror-catsystem2/src/native_class_state.rs#L398) |
| `kind` | `13 \| 14 \| 15` | Self::Image, Default::default, crate::fes_button::Slider::new | Handled | [400](../../crates/crysirror-catsystem2/src/native_class_state.rs#L400) |
| `kind` | `24 \| 38` | Self::Plane, Default::default | Handled | [401](../../crates/crysirror-catsystem2/src/native_class_state.rs#L401) |
| `kind` | `21 \| 29 \| 37` | Self::Plane, Default::default | Handled | [402](../../crates/crysirror-catsystem2/src/native_class_state.rs#L402) |
| `kind` | `9` | Self::Sound, Default::default | Handled | [403](../../crates/crysirror-catsystem2/src/native_class_state.rs#L403) |
| `kind` | `10` | Self::Sound, Default::default | Handled | [404](../../crates/crysirror-catsystem2/src/native_class_state.rs#L404) |
| `kind` | `_` | Local state/value operations | Handled | [405](../../crates/crysirror-catsystem2/src/native_class_state.rs#L405) |

### native_classes/cg_register.rs / handle

```rust
fn handle(cx: &mut Ctx) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under cx.call.id=421` | `let Some(register) = cx.state.cg_register.registers.get_mut(&a[0])` | Local state/value operations | Handled | [198](../../crates/crysirror-catsystem2/src/native_classes/cg_register.rs#L198) |
| `condition; under cx.call.id=422` | `let (Some(register), Some((key, flags))) = (cx.state.cg_register.registers.get_mut(&a[0]), parse_key(&text))` | Local state/value operations | Handled | [207](../../crates/crysirror-catsystem2/src/native_classes/cg_register.rs#L207) |
| `(a[0], cx.state.cg_register.registers.get(&a[2])); under cx.call.id=424` | `(0, _) \| (_, None)` | Local state/value operations | Handled | [230](../../crates/crysirror-catsystem2/src/native_classes/cg_register.rs#L230) |
| `(a[0], cx.state.cg_register.registers.get(&a[2])); under cx.call.id=424` | `(container, Some(register))` | serde_json::to_vec | Handled | [231](../../crates/crysirror-catsystem2/src/native_classes/cg_register.rs#L231) |
| `condition; under cx.call.id=458` | `a[0] == 0` | Local state/value operations | Handled | [258](../../crates/crysirror-catsystem2/src/native_classes/cg_register.rs#L258) |
| `condition; under cx.call.id=458` | `let (Some(bytes), Some(register)) = (bytes, cx.state.cg_register.registers.get_mut(&a[0]))` | Local state/value operations | Handled | [280](../../crates/crysirror-catsystem2/src/native_classes/cg_register.rs#L280) |
| `condition; under cx.call.id=459` | `let Some(register) = cx.state.cg_register.registers.get_mut(&a[0])` | Local state/value operations | Handled | [290](../../crates/crysirror-catsystem2/src/native_classes/cg_register.rs#L290) |
| `cx.call.id` | `id` | Local state/value operations | Partial: CatSystem2 gallery native {id} is not implemented | [299](../../crates/crysirror-catsystem2/src/native_classes/cg_register.rs#L299) |

### native_classes/cg_register.rs / Register::group

```rust
fn group(&self, list: u32, name: &[u8]) -> Option<Vec<Vec<u8>>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `terminated(self.list.get(at + 4..at + 68)?).eq_ignore_ascii_case(name)` | Vec::new, self.list.get | Handled | [149](../../crates/crysirror-catsystem2/src/native_classes/cg_register.rs#L149) |

### native_classes/containers.rs / handle

```rust
fn handle(cx: &mut Ctx) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under cx.call.id=609 \| 610` | `cx.call.id == 609` | Local state/value operations | Handled | [28](../../crates/crysirror-catsystem2/src/native_classes/containers.rs#L28) |
| `(a[0], words); under cx.call.id=240` | `(0, _) \| (_, None)` | Local state/value operations | Handled | [43](../../crates/crysirror-catsystem2/src/native_classes/containers.rs#L43) |
| `(a[0], words); under cx.call.id=240` | `(container, Some(words))` | Local state/value operations | Handled | [44](../../crates/crysirror-catsystem2/src/native_classes/containers.rs#L44) |
| `condition; under cx.call.id=385` | `a[3] != 0` | Local state/value operations | Handled | [83](../../crates/crysirror-catsystem2/src/native_classes/containers.rs#L83) |
| `condition; under cx.call.id=386 / (record, cx.handles.scenes.get_mut(&a[2]))=(Some(bytes), Some(scene))` | `a[3] != 0` | Local state/value operations | Handled | [104](../../crates/crysirror-catsystem2/src/native_classes/containers.rs#L104) |
| `condition; under cx.call.id=978` | `let (Some(bytes), Some(entry)) = (nested, cx.handles.scenes.get_mut(&a[0]).and_then(\|scene\| scene.history.last_mut()))` | Local state/value operations | Handled | [121](../../crates/crysirror-catsystem2/src/native_classes/containers.rs#L121) |
| `condition; under cx.call.id=981` | `let Some(source) = cx.handles.scenes.get(&a[1]).cloned()` | Local state/value operations | Handled | [146](../../crates/crysirror-catsystem2/src/native_classes/containers.rs#L146) |
| `condition; under cx.call.id=981` | `let Some(target) = cx.handles.scenes.get_mut(&a[0])` | Local state/value operations | Handled | [147](../../crates/crysirror-catsystem2/src/native_classes/containers.rs#L147) |
| `condition; under cx.call.id=993` | `a[0] == a[1]` | Vec::new | Handled | [159](../../crates/crysirror-catsystem2/src/native_classes/containers.rs#L159) |
| `condition; under cx.call.id=993` | `let (Some(copy), Some(target)) = (copy, cx.handles.scenes.get_mut(&a[0]))` | Local state/value operations | Handled | [164](../../crates/crysirror-catsystem2/src/native_classes/containers.rs#L164) |
| `cx.call.id` | `id` | Local state/value operations | Handled | [169](../../crates/crysirror-catsystem2/src/native_classes/containers.rs#L169) |

### native_classes/database.rs / fold

```rust
fn fold(name: &mut [u8]) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `lead_byte(name[at]) && at + 1 < name.len()` | Local state/value operations | Handled | [40](../../crates/crysirror-catsystem2/src/native_classes/database.rs#L40) |
| `condition` | `let Some((_, to)) = FOLDED.iter().find(\|(from, _)\| name[at..at + 2] == *from)` | Local state/value operations | Handled | [41](../../crates/crysirror-catsystem2/src/native_classes/database.rs#L41) |

### native_classes/database.rs / handle

```rust
fn handle(cx: &mut Ctx) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under cx.call.id=871` | `let Some(database) = cx.settings.as_mut().context("CatSystem2 database settings")?.databases.get_mut(&a[0])` | Local state/value operations | Handled | [297](../../crates/crysirror-catsystem2/src/native_classes/database.rs#L297) |
| `condition; under cx.call.id=873` | `a[1] != 0 && cx.settings.as_mut().context("CatSystem2 database settings")?.databases.contains_key(&a[0])` | u32::from_le_bytes | Handled | [321](../../crates/crysirror-catsystem2/src/native_classes/database.rs#L321) |
| `cx.call.id` | `id` | Local state/value operations | Handled | [335](../../crates/crysirror-catsystem2/src/native_classes/database.rs#L335) |

### native_classes/effect_tasks.rs / handle

```rust
fn handle(cx: &mut Ctx) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `cx.call.id` | `other` | Local state/value operations | Unsupported: CatSystem2 native {other} has no handler in the effect family | [231](../../crates/crysirror-catsystem2/src/native_classes/effect_tasks.rs#L231) |

### native_classes/emote_messages.rs / handle

```rust
fn handle(cx: &mut Ctx) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `cx.call.id` | `id` | Local state/value operations | Partial: CatSystem2 E-mote native {id} is not implemented | [133](../../crates/crysirror-catsystem2/src/native_classes/emote_messages.rs#L133) |

### native_classes/export_task.rs / handle

```rust
fn handle(cx: &mut Ctx) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under cx.call.id=690 \| 692` | `cx.call.id == 690` | Local state/value operations | Handled | [56](../../crates/crysirror-catsystem2/src/native_classes/export_task.rs#L56) |
| `condition; under cx.call.id=690 \| 692` | `!cx.state.export_task.workers.contains_key(&a[0])` | cx.host.storage, crate::native_resource_paths::Catalog::load, crate::native_resource_paths::resolve, crysirror_resource::read_all, Vec::new | Handled | [58](../../crates/crysirror-catsystem2/src/native_classes/export_task.rs#L58) |
| `cx.call.id` | `id` | Local state/value operations | Partial: CatSystem2 export native {id} is not implemented | [97](../../crates/crysirror-catsystem2/src/native_classes/export_task.rs#L97) |

### native_classes/export_task.rs / walk

```rust
fn walk(bytes: &[u8], mut at: usize, path: &str, out: &mut Vec<(String, Vec<u8>)>) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `kind == 0` | Vec::new, flate2::read::ZlibDecoder::new | Handled | [33](../../crates/crysirror-catsystem2/src/native_classes/export_task.rs#L33) |

### native_classes/host_misc.rs / handle

```rust
fn handle(cx: &mut Ctx) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under cx.call.id=201` | `cx.tasks.opaque_class(a[0]) == Some(18)` | super::shader_tasks::raster_native | Handled | [47](../../crates/crysirror-catsystem2/src/native_classes/host_misc.rs#L47) |
| `(a[0], image); under cx.call.id=241` | `(0, _) \| (_, None)` | Local state/value operations | Handled | [61](../../crates/crysirror-catsystem2/src/native_classes/host_misc.rs#L61) |
| `(a[0], image); under cx.call.id=241` | `(container, Some(image))` | crysirror_image::encode_png | Handled | [62](../../crates/crysirror-catsystem2/src/native_classes/host_misc.rs#L62) |
| `condition; under cx.call.id=650 / record=Some(bytes)` | `vm.restore_player_bank(a[2], &bytes)` | Local state/value operations | Handled | [97](../../crates/crysirror-catsystem2/src/native_classes/host_misc.rs#L97) |
| `condition; under cx.call.id=315` | `a[0] == 0` | Local state/value operations | Handled | [146](../../crates/crysirror-catsystem2/src/native_classes/host_misc.rs#L146) |
| `condition; under cx.call.id=1009` | `a[0] == 0` | Local state/value operations | Handled | [230](../../crates/crysirror-catsystem2/src/native_classes/host_misc.rs#L230) |
| `condition; under cx.call.id=1049` | `a[0] == 0` | Local state/value operations | Handled | [251](../../crates/crysirror-catsystem2/src/native_classes/host_misc.rs#L251) |
| `cx.call.id` | `id` | Local state/value operations | Unsupported: CatSystem2 native {id} is not implemented | [262](../../crates/crysirror-catsystem2/src/native_classes/host_misc.rs#L262) |

### native_classes/image_ops.rs / handle

```rust
fn handle(cx: &mut Ctx) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `cx.tasks.image_list(a[0]).is_err()` | Local state/value operations | Handled | [17](../../crates/crysirror-catsystem2/src/native_classes/image_ops.rs#L17) |
| `condition; under cx.call.id=464` | `cx.tasks.image_list(a[0])?.manager.is_some()` | Local state/value operations | Handled | [37](../../crates/crysirror-catsystem2/src/native_classes/image_ops.rs#L37) |
| `cx.call.id` | `id` | Local state/value operations | Handled | [53](../../crates/crysirror-catsystem2/src/native_classes/image_ops.rs#L53) |

### native_classes/input_task.rs / handle

```rust
fn handle(cx: &mut Ctx) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under cx.call.id=106` | `bound && receiver(cx, a[0])?.is_some()` | Local state/value operations | Handled | [55](../../crates/crysirror-catsystem2/src/native_classes/input_task.rs#L55) |
| `condition; under cx.call.id=106` | `bound && a[1] != 0` | Local state/value operations | Handled | [57](../../crates/crysirror-catsystem2/src/native_classes/input_task.rs#L57) |
| `condition; under cx.call.id=394` | `a[1] == 0` | Local state/value operations | Handled | [113](../../crates/crysirror-catsystem2/src/native_classes/input_task.rs#L113) |
| `cx.call.id` | `id` | Local state/value operations | Handled | [167](../../crates/crysirror-catsystem2/src/native_classes/input_task.rs#L167) |

### native_classes/name_table.rs / handle

```rust
fn handle(cx: &mut Ctx) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `cx.call.id` | `id` | Local state/value operations | Unsupported: CatSystem2 native {id} has no handler in the name table family | [241](../../crates/crysirror-catsystem2/src/native_classes/name_table.rs#L241) |

### native_classes/name_table.rs / parse

```rust
fn parse(bytes: &[u8]) -> Vec<Row>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `name.len() >= 256` | Local state/value operations | Handled | [91](../../crates/crysirror-catsystem2/src/native_classes/name_table.rs#L91) |

### native_classes/os_window.rs / handle

```rust
fn handle(cx: &mut Ctx) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `cx.call.id` | `id` | Local state/value operations | Handled | [61](../../crates/crysirror-catsystem2/src/native_classes/os_window.rs#L61) |

### native_classes/particle_plugins.rs / Plugin::kind_of

```rust
fn kind_of(name: &[u8]) -> Option<Kind>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `name` | `b"em_note"` | Local state/value operations | Handled | [228](../../crates/crysirror-catsystem2/src/native_classes/particle_plugins.rs#L228) |
| `name` | `b"em_double"` | Local state/value operations | Handled | [229](../../crates/crysirror-catsystem2/src/native_classes/particle_plugins.rs#L229) |
| `name` | `b"em_question"` | Local state/value operations | Handled | [230](../../crates/crysirror-catsystem2/src/native_classes/particle_plugins.rs#L230) |
| `name` | `b"em_question_a"` | Local state/value operations | Handled | [231](../../crates/crysirror-catsystem2/src/native_classes/particle_plugins.rs#L231) |
| `name` | `b"em_sweat"` | Local state/value operations | Handled | [232](../../crates/crysirror-catsystem2/src/native_classes/particle_plugins.rs#L232) |
| `name` | `b"em_angry"` | Local state/value operations | Handled | [233](../../crates/crysirror-catsystem2/src/native_classes/particle_plugins.rs#L233) |
| `name` | `b"rain"` | Local state/value operations | Handled | [234](../../crates/crysirror-catsystem2/src/native_classes/particle_plugins.rs#L234) |
| `name` | `b"dash"` | Local state/value operations | Handled | [235](../../crates/crysirror-catsystem2/src/native_classes/particle_plugins.rs#L235) |
| `name` | `b"filmnoise"` | Local state/value operations | Handled | [236](../../crates/crysirror-catsystem2/src/native_classes/particle_plugins.rs#L236) |
| `name` | `b"blur"` | Local state/value operations | Handled | [237](../../crates/crysirror-catsystem2/src/native_classes/particle_plugins.rs#L237) |
| `name` | `b"steam"` | Local state/value operations | Handled | [238](../../crates/crysirror-catsystem2/src/native_classes/particle_plugins.rs#L238) |
| `name` | `b"cmnptcl"` | Local state/value operations | Handled | [239](../../crates/crysirror-catsystem2/src/native_classes/particle_plugins.rs#L239) |
| `name` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [240](../../crates/crysirror-catsystem2/src/native_classes/particle_plugins.rs#L240) |

### native_classes/particle_plugins.rs / Plugin::update_angry

```rust
fn update_angry(&mut self, index: usize) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `sub.phase` | `0` | Local state/value operations | Handled | [624](../../crates/crysirror-catsystem2/src/native_classes/particle_plugins.rs#L624) |
| `sub.phase` | `1` | Local state/value operations | Handled | [633](../../crates/crysirror-catsystem2/src/native_classes/particle_plugins.rs#L633) |
| `sub.phase` | `2` | Local state/value operations | Handled | [641](../../crates/crysirror-catsystem2/src/native_classes/particle_plugins.rs#L641) |
| `sub.phase` | `3` | Local state/value operations | Handled | [647](../../crates/crysirror-catsystem2/src/native_classes/particle_plugins.rs#L647) |
| `sub.phase` | `4` | Local state/value operations | Handled | [655](../../crates/crysirror-catsystem2/src/native_classes/particle_plugins.rs#L655) |
| `sub.phase` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [663](../../crates/crysirror-catsystem2/src/native_classes/particle_plugins.rs#L663) |

### native_classes/particle_plugins.rs / Plugin::update_note

```rust
fn update_note(&mut self, index: usize, random: &mut RandomStreams, client: ClientId) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under phase=0` | `e > sub.times[0]` | Local state/value operations | Handled | [476](../../crates/crysirror-catsystem2/src/native_classes/particle_plugins.rs#L476) |
| `condition; under phase=1` | `e > sub.times[1]` | Local state/value operations | Handled | [481](../../crates/crysirror-catsystem2/src/native_classes/particle_plugins.rs#L481) |
| `condition; under phase=_` | `e > sub.times[2]` | Local state/value operations | Handled | [487](../../crates/crysirror-catsystem2/src/native_classes/particle_plugins.rs#L487) |

### native_classes/particle_plugins.rs / Plugin::update_question

```rust
fn update_question(&mut self, index: usize, _random: &mut RandomStreams, _client: ClientId) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `sub.phase` | `0` | Local state/value operations | Handled | [546](../../crates/crysirror-catsystem2/src/native_classes/particle_plugins.rs#L546) |
| `sub.phase` | `1` | Local state/value operations | Handled | [554](../../crates/crysirror-catsystem2/src/native_classes/particle_plugins.rs#L554) |
| `sub.phase` | `2` | Local state/value operations | Handled | [562](../../crates/crysirror-catsystem2/src/native_classes/particle_plugins.rs#L562) |
| `sub.phase` | `3` | Local state/value operations | Handled | [569](../../crates/crysirror-catsystem2/src/native_classes/particle_plugins.rs#L569) |
| `sub.phase` | `4` | Local state/value operations | Handled | [574](../../crates/crysirror-catsystem2/src/native_classes/particle_plugins.rs#L574) |
| `sub.phase` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [582](../../crates/crysirror-catsystem2/src/native_classes/particle_plugins.rs#L582) |

### native_classes/particle_plugins.rs / Plugin::update_sweat

```rust
fn update_sweat(&mut self, index: usize) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `sub.phase` | `0` | Local state/value operations | Handled | [597](../../crates/crysirror-catsystem2/src/native_classes/particle_plugins.rs#L597) |
| `sub.phase` | `1` | Local state/value operations | Handled | [603](../../crates/crysirror-catsystem2/src/native_classes/particle_plugins.rs#L603) |
| `sub.phase` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [608](../../crates/crysirror-catsystem2/src/native_classes/particle_plugins.rs#L608) |

### native_classes/particles.rs / handle

```rust
fn handle(cx: &mut Ctx) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under cx.call.id=271 \| 693` | `cx.tasks.opaque_class(a[0]) != Some(16)` | Local state/value operations | Handled | [402](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L402) |
| `condition; under cx.call.id=271 \| 693 / load.script.flatten()=None` | `cx.call.id == 271` | cx.host.append_log | Handled | [442](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L442) |
| `condition; under cx.call.id=272` | `let Some(particles) = state_of(cx, a[0])` | Local state/value operations | Handled | [461](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L461) |
| `condition; under cx.call.id=275` | `cx.state.particles.tasks.contains_key(&a[0])` | Local state/value operations | Handled | [491](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L491) |
| `condition; under cx.call.id=426` | `let Some(particles) = state_of(cx, a[0])` | Local state/value operations | Handled | [501](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L501) |
| `condition; under cx.call.id=426` | `a[1] as i32 >= 255` | Local state/value operations | Handled | [502](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L502) |
| `condition; under cx.call.id=427` | `let Some(particles) = state_of(cx, a[0])` | Local state/value operations | Handled | [510](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L510) |
| `condition; under cx.call.id=427` | `a[1] == 0` | Local state/value operations | Handled | [511](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L511) |
| `cx.call.id` | `id` | Local state/value operations | Unsupported: CatSystem2 native {id} has no handler in the particle family | [515](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L515) |

### native_classes/particles.rs / tick

```rust
fn tick(host: &mut dyn HostServices, client: ClientId, tasks: &crate::task_policy::NativeTaskPolicy, canvas: [u32; 2], state: &mut State, random: Option<&mut crate::motion::RandomStreams>, now_ms: u64) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under event=Event::Native(call)` | `particles.native(host, client, tasks, canvas, random, machine, &call).with_context(\|\| format!("CatSystem2 particle native {} at PC {:#x}", call.id, call.pc))?` | Local state/value operations | Handled | [568](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L568) |

### native_classes/particles.rs / Particles::native

```rust
fn native(&mut self, host: &mut dyn HostServices, client: ClientId, tasks: &crate::task_policy::NativeTaskPolicy, canvas: [u32; 2], random: &mut crate::motion::RandomStreams, machine: &mut Machine, call: &NativeCall) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `call.id; under call.id=22..=26` | `_` | Local state/value operations | Handled | [203](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L203) |
| `condition; under call.id=39` | `(a[0] as usize) < self.count` | Local state/value operations | Handled | [272](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L272) |
| `condition; under call.id=47 \| 49` | `call.id == 47` | Local state/value operations | Handled | [295](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L295) |
| `condition; under call.id=50` | `a[0] <= 5` | Local state/value operations | Handled | [305](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L305) |
| `call.id` | `id` | Local state/value operations | Unsupported: CatSystem2 particle script native {id} at PC {:#x} is outside A40778 | [310](../../crates/crysirror-catsystem2/src/native_classes/particles.rs#L310) |

### native_classes/plane_effects.rs / handle

```rust
fn handle(cx: &mut Ctx) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under cx.call.id=81` | `a[0] != 0` | crysirror_core::path::VfsPath::new, cx.host.storage | Handled | [170](../../crates/crysirror-catsystem2/src/native_classes/plane_effects.rs#L170) |
| `condition; under cx.call.id=81` | `let Some(found) = file_exists(cx, "kx2", &name)?` | crysirror_core::path::VfsPath::new, cx.host.storage | Handled | [171](../../crates/crysirror-catsystem2/src/native_classes/plane_effects.rs#L171) |
| `condition; under cx.call.id=344` | `a[0] != 0 && a[1] != 0 && renderable` | crate::native_plane::publish_source, cx.host.query_engine, EngineQuery::Layer, cx.host.submit_engine, serde_json::to_vec | Handled | [184](../../crates/crysirror-catsystem2/src/native_classes/plane_effects.rs#L184) |
| `condition; under cx.call.id=344` | `let Some(payload) = cx.tasks.plane(a[0])?.payload` | cx.host.query_engine, EngineQuery::Layer, cx.host.submit_engine, serde_json::to_vec | Handled | [203](../../crates/crysirror-catsystem2/src/native_classes/plane_effects.rs#L203) |
| `condition; under cx.call.id=398` | `a[0] != 0` | u32::from, f32::from_bits | Handled | [216](../../crates/crysirror-catsystem2/src/native_classes/plane_effects.rs#L216) |
| `condition; under cx.call.id=398` | `a[1] != 0 && cx.tasks.plane(a[0]).is_ok()` | u32::from, f32::from_bits | Handled | [218](../../crates/crysirror-catsystem2/src/native_classes/plane_effects.rs#L218) |
| `a[10]; under cx.call.id=398` | `0` | Local state/value operations | Handled | [222](../../crates/crysirror-catsystem2/src/native_classes/plane_effects.rs#L222) |
| `a[10]; under cx.call.id=398` | `1` | Local state/value operations | Handled | [223](../../crates/crysirror-catsystem2/src/native_classes/plane_effects.rs#L223) |
| `a[10]; under cx.call.id=398` | `_` | Local state/value operations | Handled | [224](../../crates/crysirror-catsystem2/src/native_classes/plane_effects.rs#L224) |
| `condition; under cx.call.id=399` | `cx.state.plane_effects.trails.contains_key(&a[0])` | Local state/value operations | Handled | [266](../../crates/crysirror-catsystem2/src/native_classes/plane_effects.rs#L266) |
| `condition; under cx.call.id=433` | `a[0] != 0 && renderable` | f32::from_bits, cx.host.submit_engine, serde_json::to_vec, crate::native_plane::create_target, crate::native_plane::publish | Handled | [280](../../crates/crysirror-catsystem2/src/native_classes/plane_effects.rs#L280) |
| `condition; under cx.call.id=701 \| 988` | `a[0] != 0 && renderable` | crate::native_values::format_bytes, String::from_utf8, crate::native_scene::stem | Handled | [307](../../crates/crysirror-catsystem2/src/native_classes/plane_effects.rs#L307) |
| `condition; under cx.call.id=701 \| 988` | `cx.call.id == 701` | Local state/value operations | Handled | [310](../../crates/crysirror-catsystem2/src/native_classes/plane_effects.rs#L310) |
| `cx.call.id` | `id` | Local state/value operations | Unsupported: CatSystem2 Plane object native {id} is not implemented | [328](../../crates/crysirror-catsystem2/src/native_classes/plane_effects.rs#L328) |

### native_classes/plane_effects.rs / indices

```rust
fn indices(trail: &Trail) -> Vec<u32>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `trail.mode` | `0` | Local state/value operations | Handled | [103](../../crates/crysirror-catsystem2/src/native_classes/plane_effects.rs#L103) |
| `trail.mode` | `2 \| 3` | Local state/value operations | Handled | [104](../../crates/crysirror-catsystem2/src/native_classes/plane_effects.rs#L104) |
| `trail.mode` | `_` | Local state/value operations | Handled | [105](../../crates/crysirror-catsystem2/src/native_classes/plane_effects.rs#L105) |

### native_classes/plane_effects.rs / tick

```rust
fn tick(host: &mut dyn HostServices, client: ClientId, tasks: &mut crate::task_policy::NativeTaskPolicy, state: &mut State) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under trail.mode=1 \| 2 \| 3 if trail.mode != 1 \|\| index % trail.stride != 0` | `trail.mode == 2` | Local state/value operations | Handled | [425](../../crates/crysirror-catsystem2/src/native_classes/plane_effects.rs#L425) |
| `trail.mode` | `1 \| 2 \| 3 if trail.mode != 1 \|\| index % trail.stride != 0` | Local state/value operations | Handled | [422](../../crates/crysirror-catsystem2/src/native_classes/plane_effects.rs#L422) |
| `trail.mode` | `4 \| 5` | Local state/value operations | Handled | [429](../../crates/crysirror-catsystem2/src/native_classes/plane_effects.rs#L429) |
| `trail.mode` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [440](../../crates/crysirror-catsystem2/src/native_classes/plane_effects.rs#L440) |
| `condition` | `trail.mode >= 4` | Local state/value operations | Handled | [448](../../crates/crysirror-catsystem2/src/native_classes/plane_effects.rs#L448) |
| `condition` | `trail.mode == 4` | Local state/value operations | Handled | [458](../../crates/crysirror-catsystem2/src/native_classes/plane_effects.rs#L458) |

### native_classes/plane_messages.rs / handle

```rust
fn handle(cx: &mut Ctx) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under cx.call.id=402` | `let Some(payload) = cx.tasks.plane(a[0]).ok().and_then(\|plane\| plane.payload)` | cx.host.submit_engine, serde_json::to_vec | Handled | [44](../../crates/crysirror-catsystem2/src/native_classes/plane_messages.rs#L44) |
| `a[1]; under cx.call.id=402` | `1` | Local state/value operations | Handled | [46](../../crates/crysirror-catsystem2/src/native_classes/plane_messages.rs#L46) |
| `a[1]; under cx.call.id=402` | `2` | Local state/value operations | Handled | [47](../../crates/crysirror-catsystem2/src/native_classes/plane_messages.rs#L47) |
| `a[1]; under cx.call.id=402` | `3` | Local state/value operations | Handled | [48](../../crates/crysirror-catsystem2/src/native_classes/plane_messages.rs#L48) |
| `a[1]; under cx.call.id=402` | `4` | Local state/value operations | Handled | [49](../../crates/crysirror-catsystem2/src/native_classes/plane_messages.rs#L49) |
| `a[1]; under cx.call.id=402` | `5` | Local state/value operations | Handled | [50](../../crates/crysirror-catsystem2/src/native_classes/plane_messages.rs#L50) |
| `a[1]; under cx.call.id=402` | `_` | Local state/value operations | Handled | [51](../../crates/crysirror-catsystem2/src/native_classes/plane_messages.rs#L51) |
| `condition; under cx.call.id=653` | `let Some(plane) = cx.tasks.plane_mut(a[0])` | Local state/value operations | Handled | [60](../../crates/crysirror-catsystem2/src/native_classes/plane_messages.rs#L60) |
| `condition; under cx.call.id=95` | `proposed.bind_plane_button(id, target, native_rect, a[1], full_canvas)?` | cx.host.submit_engine, serde_json::to_vec | Handled | [94](../../crates/crysirror-catsystem2/src/native_classes/plane_messages.rs#L94) |
| `cx.call.id` | `id` | Local state/value operations | Handled | [101](../../crates/crysirror-catsystem2/src/native_classes/plane_messages.rs#L101) |

### native_classes/render_settings.rs / handle

```rust
fn handle(cx: &mut Ctx) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under cx.call.id=879` | `a[2] == u32::MAX` | Local state/value operations | Handled | [37](../../crates/crysirror-catsystem2/src/native_classes/render_settings.rs#L37) |
| `cx.call.id` | `id` | Local state/value operations | Partial: CatSystem2 render setting native {id} is not implemented | [51](../../crates/crysirror-catsystem2/src/native_classes/render_settings.rs#L51) |

### native_classes/scene_log.rs / read_inputs

```rust
fn read_inputs(storage: &dyn crysirror_core::service::ClientStorage, catalog: &crate::native_resource_paths::Catalog, scene: &crate::scene::Scene, count: u32, voice: &[u8], novel_command: &[u8], novel_mode: &mut bool) -> Result<Vec<(Vec<u8>, Vec<u8>, String, bool)>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `line.kind` | `2` | Local state/value operations | Handled | [56](../../crates/crysirror-catsystem2/src/native_classes/scene_log.rs#L56) |
| `line.kind` | `3 if *novel_mode` | Local state/value operations | Handled | [57](../../crates/crysirror-catsystem2/src/native_classes/scene_log.rs#L57) |
| `line.kind` | `32` | Local state/value operations | Handled | [58](../../crates/crysirror-catsystem2/src/native_classes/scene_log.rs#L58) |
| `line.kind` | `33` | Local state/value operations | Handled | [59](../../crates/crysirror-catsystem2/src/native_classes/scene_log.rs#L59) |
| `line.kind` | `48` | std::str::from_utf8 | Handled | [60](../../crates/crysirror-catsystem2/src/native_classes/scene_log.rs#L60) |
| `line.kind` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [67](../../crates/crysirror-catsystem2/src/native_classes/scene_log.rs#L67) |

### native_classes/scene_records.rs / handle

```rust
fn handle(cx: &mut Ctx) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under cx.call.id=991` | `!cx.handles.scenes.contains_key(&a[0])` | Local state/value operations | Handled | [123](../../crates/crysirror-catsystem2/src/native_classes/scene_records.rs#L123) |
| `condition; under cx.call.id=992` | `index < 0 \|\| !cx.handles.scenes.contains_key(&a[0])` | Local state/value operations | Handled | [149](../../crates/crysirror-catsystem2/src/native_classes/scene_records.rs#L149) |
| `condition; under cx.call.id=994` | `let Some(scene) = cx.handles.scenes.get_mut(&a[0])` | Local state/value operations | Handled | [193](../../crates/crysirror-catsystem2/src/native_classes/scene_records.rs#L193) |
| `condition; under cx.call.id=994` | `let Some(entry) = scene.history.get_mut(a[1] as usize)` | Local state/value operations | Handled | [201](../../crates/crysirror-catsystem2/src/native_classes/scene_records.rs#L201) |
| `cx.call.id` | `id` | Local state/value operations | Partial: CatSystem2 scene record native {id} is not implemented | [208](../../crates/crysirror-catsystem2/src/native_classes/scene_records.rs#L208) |

### native_classes/shader_tasks.rs / handle

```rust
fn handle(cx: &mut Ctx) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `id == 520` | cx.host.now_ms, u32::from | Handled | [355](../../crates/crysirror-catsystem2/src/native_classes/shader_tasks.rs#L355) |
| `condition` | `let Some(effect) = cx.state.shader_tasks.effects.get_mut(&a[0])` | Local state/value operations | Handled | [359](../../crates/crysirror-catsystem2/src/native_classes/shader_tasks.rs#L359) |
| `id` | `521 \| 522` | Local state/value operations | Handled | [371](../../crates/crysirror-catsystem2/src/native_classes/shader_tasks.rs#L371) |
| `id` | `523 \| 524` | Local state/value operations | Handled | [372](../../crates/crysirror-catsystem2/src/native_classes/shader_tasks.rs#L372) |
| `id` | `527 \| 528` | Local state/value operations | Handled | [373](../../crates/crysirror-catsystem2/src/native_classes/shader_tasks.rs#L373) |
| `id` | `530 \| 531 \| 905` | Local state/value operations | Handled | [374](../../crates/crysirror-catsystem2/src/native_classes/shader_tasks.rs#L374) |
| `id` | `853..=855` | Local state/value operations | Handled | [375](../../crates/crysirror-catsystem2/src/native_classes/shader_tasks.rs#L375) |
| `id` | `856 \| 857` | Local state/value operations | Handled | [376](../../crates/crysirror-catsystem2/src/native_classes/shader_tasks.rs#L376) |
| `id` | `860 \| 861` | Local state/value operations | Handled | [377](../../crates/crysirror-catsystem2/src/native_classes/shader_tasks.rs#L377) |
| `id` | `967 \| 968` | Local state/value operations | Handled | [378](../../crates/crysirror-catsystem2/src/native_classes/shader_tasks.rs#L378) |
| `id` | `_` | Local state/value operations | Handled | [379](../../crates/crysirror-catsystem2/src/native_classes/shader_tasks.rs#L379) |
| `condition; under id=967 \| 971` | `cx.call.id == 967` | f32::from_bits | Handled | [435](../../crates/crysirror-catsystem2/src/native_classes/shader_tasks.rs#L435) |
| `condition; under id=968 \| 972 if cx.state.shader_tasks.grids.contains_key(&a[0])` | `a[1] != 0` | cx.host.submit_engine, serde_json::to_vec | Handled | [452](../../crates/crysirror-catsystem2/src/native_classes/shader_tasks.rs#L452) |
| `condition; under id=861 if cx.state.shader_tasks.shatters.contains_key(&a[0])` | `a[1] != 0` | cx.host.submit_engine, serde_json::to_vec | Handled | [478](../../crates/crysirror-catsystem2/src/native_classes/shader_tasks.rs#L478) |
| `id` | `527` | Local state/value operations | Handled | [388](../../crates/crysirror-catsystem2/src/native_classes/shader_tasks.rs#L388) |
| `id` | `530 \| 905` | Local state/value operations | Handled | [393](../../crates/crysirror-catsystem2/src/native_classes/shader_tasks.rs#L393) |
| `id` | `853 \| 856` | Local state/value operations | Handled | [399](../../crates/crysirror-catsystem2/src/native_classes/shader_tasks.rs#L399) |
| `id` | `854` | Local state/value operations | Empty handler: Original no-op status not established | [404](../../crates/crysirror-catsystem2/src/native_classes/shader_tasks.rs#L404) |
| `id` | `528 \| 531 \| 855 \| 857` | cx.host.now_ms | Handled | [406](../../crates/crysirror-catsystem2/src/native_classes/shader_tasks.rs#L406) |
| `id` | `860` | f32::from_bits, u64::from | Handled | [414](../../crates/crysirror-catsystem2/src/native_classes/shader_tasks.rs#L414) |
| `id` | `967 \| 971` | f32::from_bits, u64::from | Handled | [432](../../crates/crysirror-catsystem2/src/native_classes/shader_tasks.rs#L432) |
| `id` | `968 \| 972 if cx.state.shader_tasks.grids.contains_key(&a[0])` | cx.host.now_ms, cx.host.submit_engine, serde_json::to_vec | Handled | [445](../../crates/crysirror-catsystem2/src/native_classes/shader_tasks.rs#L445) |
| `id` | `970` | Local state/value operations | Empty handler: Original no-op status not established | [468](../../crates/crysirror-catsystem2/src/native_classes/shader_tasks.rs#L468) |
| `id` | `861 if cx.state.shader_tasks.shatters.contains_key(&a[0])` | cx.host.now_ms, cx.host.submit_engine, serde_json::to_vec | Handled | [471](../../crates/crysirror-catsystem2/src/native_classes/shader_tasks.rs#L471) |
| `id` | `861 \| 968 \| 972` | cx.host.now_ms | Handled | [494](../../crates/crysirror-catsystem2/src/native_classes/shader_tasks.rs#L494) |
| `id` | `521` | f32::from_le_bytes, f32::from_bits | Handled | [502](../../crates/crysirror-catsystem2/src/native_classes/shader_tasks.rs#L502) |
| `id` | `523` | f32::from_bits | Handled | [518](../../crates/crysirror-catsystem2/src/native_classes/shader_tasks.rs#L518) |
| `id` | `522 \| 524` | cx.host.submit_engine, serde_json::to_vec | Handled | [533](../../crates/crysirror-catsystem2/src/native_classes/shader_tasks.rs#L533) |
| `id` | `_` | Local state/value operations | Unsupported: CatSystem2 screen effect native {id} is not implemented | [545](../../crates/crysirror-catsystem2/src/native_classes/shader_tasks.rs#L545) |

### native_classes/shader_tasks.rs / raster_native

```rust
fn raster_native(cx: &mut Ctx) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under cx.call.id=201` | `a[1] == 0` | Local state/value operations | Handled | [290](../../crates/crysirror-catsystem2/src/native_classes/shader_tasks.rs#L290) |
| `cx.call.id` | `_` | cx.host.submit_engine, serde_json::to_vec | Handled | [308](../../crates/crysirror-catsystem2/src/native_classes/shader_tasks.rs#L308) |

### native_classes/subsystems.rs / handle

```rust
fn handle(cx: &mut Ctx) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `cx.call.id; under cx.call.id=168 \| 347 \| 348` | `168 if a.len() >= 4` | f32::from_bits | Handled | [66](../../crates/crysirror-catsystem2/src/native_classes/subsystems.rs#L66) |
| `cx.call.id; under cx.call.id=168 \| 347 \| 348` | `348 if a.len() >= 2` | f32::from_bits | Handled | [67](../../crates/crysirror-catsystem2/src/native_classes/subsystems.rs#L67) |
| `cx.call.id; under cx.call.id=168 \| 347 \| 348` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [68](../../crates/crysirror-catsystem2/src/native_classes/subsystems.rs#L68) |
| `condition; under cx.call.id=168 \| 347 \| 348` | `cx.call.id != 347 && cx.host.audio_info(cx.client, channel)?.is_some()` | cx.host.submit_audio | Handled | [71](../../crates/crysirror-catsystem2/src/native_classes/subsystems.rs#L71) |
| `cx.call.id` | `id` | Local state/value operations | Handled | [169](../../crates/crysirror-catsystem2/src/native_classes/subsystems.rs#L169) |

### native_classes/tables_3d.rs / handle

```rust
fn handle(cx: &mut Ctx) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under cx.call.id=596 \| 597` | `cx.call.id == 596` | Local state/value operations | Handled | [179](../../crates/crysirror-catsystem2/src/native_classes/tables_3d.rs#L179) |
| `(value, a[4]); under cx.call.id=685` | `(Some(mut value), target) if target != 0` | Local state/value operations | Handled | [235](../../crates/crysirror-catsystem2/src/native_classes/tables_3d.rs#L235) |
| `(value, a[4]); under cx.call.id=685` | `_` | Local state/value operations | Handled | [239](../../crates/crysirror-catsystem2/src/native_classes/tables_3d.rs#L239) |
| `cx.call.id` | `id` | Local state/value operations | Partial: CatSystem2 section table native {id} is not implemented | [245](../../crates/crysirror-catsystem2/src/native_classes/tables_3d.rs#L245) |

### native_classes/text_window.rs / decode

```rust
fn decode(code: u16) -> Option<char>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `code & 0xff00 != 0` | Local state/value operations | Handled | [372](../../crates/crysirror-catsystem2/src/native_classes/text_window.rs#L372) |

### native_classes/text_window.rs / message

```rust
fn message(cx: &mut Ctx, id: u32, a: &[u32], window: &mut Window) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under cx.call.id=123 \| 418` | `cx.call.id == 418` | Local state/value operations | Handled | [481](../../crates/crysirror-catsystem2/src/native_classes/text_window.rs#L481) |
| `condition; under cx.call.id=124` | `added && window.config.as_ref().is_some_and(\|c\| c.mode() != 1)` | Local state/value operations | Handled | [515](../../crates/crysirror-catsystem2/src/native_classes/text_window.rs#L515) |
| `cx.call.id` | `other` | Local state/value operations | Unsupported: CatSystem2 native {other} has no handler in the text window family | [543](../../crates/crysirror-catsystem2/src/native_classes/text_window.rs#L543) |

### native_classes/text_window.rs / present

```rust
fn present(host: &mut dyn HostServices, client: ClientId, font: &str, tasks: &crate::task_policy::NativeTaskPolicy, id: u32, window: &mut Window) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `glyph.drawn.is_none() && glyph.code != 0 && glyph.code != 0xffff` | Local state/value operations | Handled | [390](../../crates/crysirror-catsystem2/src/native_classes/text_window.rs#L390) |
| `config.kind()` | `2 \| 3` | Local state/value operations | Handled | [413](../../crates/crysirror-catsystem2/src/native_classes/text_window.rs#L413) |
| `config.kind()` | `1` | Local state/value operations | Unsupported: CatSystem2 text window shadow type 1 (6A40D0) is not implemented | [414](../../crates/crysirror-catsystem2/src/native_classes/text_window.rs#L414) |
| `config.kind()` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [415](../../crates/crysirror-catsystem2/src/native_classes/text_window.rs#L415) |

### native_classes/text_window.rs / Window::align

```rust
fn align(&mut self, config: &Config) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `end < count && self.glyphs[end].code != 0` | Local state/value operations | Handled | [190](../../crates/crysirror-catsystem2/src/native_classes/text_window.rs#L190) |

### native_classes/text_window.rs / Window::place

```rust
fn place(&mut self, config: &Config, code: u16) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `self.hanging_enabled && self.glyphs.is_empty() && code == 0x8175` | Local state/value operations | Handled | [160](../../crates/crysirror-catsystem2/src/native_classes/text_window.rs#L160) |
| `condition` | `code == 0` | self.break_line | Handled | [163](../../crates/crysirror-catsystem2/src/native_classes/text_window.rs#L163) |
| `condition` | `code & 0xff00 != 0` | Local state/value operations | Handled | [167](../../crates/crysirror-catsystem2/src/native_classes/text_window.rs#L167) |

### native_classes/view3d.rs / save

```rust
fn save(cx: &mut Ctx<'_>) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `a[0] == 0` | Local state/value operations | Handled | [68](../../crates/crysirror-catsystem2/src/native_classes/view3d.rs#L68) |
| `condition` | `cx.call.id == 688` | Local state/value operations | Handled | [70](../../crates/crysirror-catsystem2/src/native_classes/view3d.rs#L70) |
| `condition` | `cx.call.id == 689` | Local state/value operations | Handled | [76](../../crates/crysirror-catsystem2/src/native_classes/view3d.rs#L76) |

### native_interpreter.rs / Interpreter::eval

```rust
fn eval(&mut self, tree: &Tree, depth: usize) -> Result<Value>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `token.kind == 13` | Value::immediate | Handled | [271](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L271) |
| `condition` | `token.kind == 7` | self.eval, self.resolved, self.locals.get, Value::immediate | Handled | [274](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L274) |
| `condition` | `(1..=4).contains(&token.kind)` | self.resolved | Handled | [301](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L301) |
| `condition; under token.kind=137 \| 141` | `token.kind == 137` | Local state/value operations | Handled | [325](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L325) |
| `condition; under token.kind=138 \| 142` | `token.kind == 138` | Local state/value operations | Handled | [334](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L334) |
| `token.kind; under token.kind=153..=159 \| 161..=163` | `153` | Local state/value operations | Handled | [363](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L363) |
| `token.kind; under token.kind=153..=159 \| 161..=163` | `154` | Local state/value operations | Handled | [367](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L367) |
| `token.kind; under token.kind=153..=159 \| 161..=163` | `155` | Local state/value operations | Handled | [368](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L368) |
| `token.kind; under token.kind=153..=159 \| 161..=163` | `156` | Local state/value operations | Handled | [369](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L369) |
| `token.kind; under token.kind=153..=159 \| 161..=163` | `157` | Local state/value operations | Handled | [370](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L370) |
| `token.kind; under token.kind=153..=159 \| 161..=163` | `158` | Local state/value operations | Handled | [371](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L371) |
| `token.kind; under token.kind=153..=159 \| 161..=163` | `159` | Local state/value operations | Handled | [372](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L372) |
| `token.kind; under token.kind=153..=159 \| 161..=163` | `161` | Local state/value operations | Handled | [373](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L373) |
| `token.kind; under token.kind=153..=159 \| 161..=163` | `162` | Local state/value operations | Handled | [374](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L374) |
| `token.kind; under token.kind=153..=159 \| 161..=163` | `163` | Local state/value operations | Handled | [375](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L375) |
| `token.kind; under token.kind=153..=159 \| 161..=163` | `_` | Local state/value operations | Handled | [376](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L376) |
| `token.kind` | `128` | Local state/value operations | Handled | [317](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L317) |
| `token.kind` | `129` | Local state/value operations | Handled | [318](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L318) |
| `token.kind` | `130` | Local state/value operations | Handled | [319](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L319) |
| `token.kind` | `131` | Local state/value operations | Handled | [320](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L320) |
| `token.kind` | `132` | Local state/value operations | Handled | [321](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L321) |
| `token.kind` | `135` | Local state/value operations | Handled | [322](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L322) |
| `token.kind` | `136` | Local state/value operations | Handled | [323](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L323) |
| `token.kind` | `137 \| 141` | self.assign | Handled | [324](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L324) |
| `token.kind` | `138 \| 142` | self.assign | Handled | [333](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L333) |
| `token.kind` | `140` | Local state/value operations | Handled | [342](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L342) |
| `token.kind` | `146` | Local state/value operations | Handled | [343](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L343) |
| `token.kind` | `149` | Local state/value operations | Handled | [350](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L350) |
| `token.kind` | `150` | Local state/value operations | Handled | [351](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L351) |
| `token.kind` | `151` | Local state/value operations | Handled | [352](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L352) |
| `token.kind` | `152` | self.assign | Handled | [353](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L353) |
| `token.kind` | `153..=159 \| 161..=163` | self.assign | Handled | [361](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L361) |
| `token.kind` | `160` | Local state/value operations | Handled | [385](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L385) |
| `token.kind` | `164` | Local state/value operations | Handled | [386](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L386) |
| `token.kind` | `165` | Local state/value operations | Handled | [387](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L387) |
| `token.kind` | `166` | Local state/value operations | Handled | [388](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L388) |
| `token.kind` | `170` | Local state/value operations | Handled | [389](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L389) |
| `token.kind` | `172` | Local state/value operations | Handled | [390](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L390) |
| `token.kind` | `173` | Local state/value operations | Handled | [391](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L391) |
| `token.kind` | `175` | Local state/value operations | Handled | [392](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L392) |
| `token.kind` | `176` | Value::immediate | Handled | [393](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L393) |
| `token.kind` | `177` | Local state/value operations | Handled | [394](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L394) |
| `token.kind` | `_` | Local state/value operations | Unsupported: CatSystem2 unsupported expression token {} | [395](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L395) |

### native_interpreter.rs / Interpreter::evaluate

```rust
fn evaluate(&mut self, source: &[u8]) -> Result<u32>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `token.kind == 6 \|\| token.kind == 5` | Local state/value operations | Handled | [192](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L192) |
| `condition` | `token.kind == 39` | Token::number | Handled | [195](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L195) |
| `condition` | `tree.token.kind == 176` | Local state/value operations | Handled | [213](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L213) |

### native_interpreter.rs / Tree::compile

```rust
fn compile(tokens: &[Token]) -> Result<Option<Self>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(token.kind, 136 \| 140) && previous.is_none_or(\|t\| operator(t.kind) && !matches!(t.kind, 134 \| 169))` | Local state/value operations | Handled | [463](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L463) |
| `condition` | `token.kind == 136` | Local state/value operations | Handled | [466](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L466) |
| `condition` | `matches!(token.kind, 137 \| 141) && previous.is_some_and(\|t\| t.kind == 7 \|\| matches!(t.kind, 134 \| 169))` | Local state/value operations | Handled | [468](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L468) |
| `condition` | `matches!(token.kind, 7 \| 13)` | Local state/value operations | Handled | [473](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L473) |
| `condition` | `token.kind == 168` | Local state/value operations | Handled | [481](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L481) |
| `previous.map(\|t\|t.kind); under receiver=token.kind == 168` | `Some(1..=4)` | Local state/value operations | Handled | [487](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L487) |
| `previous.map(\|t\|t.kind); under receiver=token.kind == 168` | `Some(7\|13\|134\|169)` | Local state/value operations | Handled | [488](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L488) |
| `previous.map(\|t\|t.kind); under receiver=token.kind == 168` | `_` | Local state/value operations | Unsupported: CatSystem2 indexed value missing | [489](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L489) |
| `condition` | `precedence(top.kind).0 < precedence(token.kind).1` | Local state/value operations | Handled | [498](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L498) |
| `condition` | `matches!((top.kind, token.kind), (133, 134) \| (168, 169))` | Box::new | Handled | [501](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L501) |
| `condition` | `token.kind == 169 && opening.value==1` | Box::new | Handled | [503](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L503) |
| `condition` | `!matches!(token.kind, 134 \| 169)` | Local state/value operations | Handled | [516](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L516) |

### native_interpreter.rs / Tree::reduce

```rust
fn reduce(values: &mut Vec<Tree>, token: Token) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `token.kind` | `1..=4 \| 128 \| 137 \| 141 \| 175..=177` | Box::new | Handled | [529](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L529) |
| `token.kind` | `138 \| 142` | Box::new | Handled | [537](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L537) |
| `token.kind` | `133 \| 168` | Local state/value operations | Unsupported: CatSystem2 unmatched expression bracket | [543](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L543) |
| `token.kind` | `_` | Box::new | Handled | [544](../../crates/crysirror-catsystem2/src/native_interpreter.rs#L544) |

### native_m2.rs / parse_actions

```rust
fn parse_actions(text: &str) -> BTreeMap<String, Vec<ActionStep>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under *first="var"` | `let (Some(ActionStep::Frame { vars, .. }), Some(name), Some(value)) = (steps.last_mut(), words.get(1), words.get(2).and_then(\|w\| w.parse::<f32>().ok()))` | Local state/value operations | Handled | [240](../../crates/crysirror-catsystem2/src/native_m2.rs#L240) |

### native_m2.rs / parse_waiting

```rust
fn parse_waiting(text: &str) -> Result<BTreeMap<String, BTreeMap<String, WaveSpec>>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `mode == "time"` | Local state/value operations | Handled | [508](../../crates/crysirror-catsystem2/src/native_m2.rs#L508) |
| `condition` | `mode == "set" && row.len() >= 3` | Local state/value operations | Partial: CatSystem2 unsupported waiting waveform {waveform} | [509](../../crates/crysirror-catsystem2/src/native_m2.rs#L509) |

### native_m2.rs / Dif::parse

```rust
fn parse(&mut self, text: &str) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under name.as_str()="pattern"` | `let (Some((position, key)), Some(value)) = (&current, row.get(1).and_then(\|v\| v.parse().ok()))` | self.patterns.get_mut | Handled | [275](../../crates/crysirror-catsystem2/src/native_m2.rs#L275) |
| `condition; under name.as_str()="pattern_shift"` | `let (Some((position, key)), Some(value)) = (&current, row.get(1).and_then(\|value\| value.parse().ok()))` | self.shifts.entry | Handled | [285](../../crates/crysirror-catsystem2/src/native_m2.rs#L285) |
| `name.as_str(); under name.as_str()="material" \| "material_min" \| "material_max"` | `"material"` | Local state/value operations | Handled | [293](../../crates/crysirror-catsystem2/src/native_m2.rs#L293) |
| `name.as_str(); under name.as_str()="material" \| "material_min" \| "material_max"` | `"material_min"` | Local state/value operations | Handled | [293](../../crates/crysirror-catsystem2/src/native_m2.rs#L293) |
| `name.as_str(); under name.as_str()="material" \| "material_min" \| "material_max"` | `_` | Local state/value operations | Handled | [293](../../crates/crysirror-catsystem2/src/native_m2.rs#L293) |
| `condition; under name.as_str()="inertia_lv" \| "inertia_enable"` | `name == "inertia_lv"` | Local state/value operations | Handled | [312](../../crates/crysirror-catsystem2/src/native_m2.rs#L312) |
| `condition; under name.as_str()="lip_table"` | `let (Some(position), Some(key), true) = (row[0].parse::<usize>().ok(), row.get(1), row.len() > 5)` | self.lips.entry | Handled | [333](../../crates/crysirror-catsystem2/src/native_m2.rs#L333) |
| `condition; under name.as_str()="lip_variable"` | `let (Some(position), Some(key), Some(variable)) = (row[0].parse::<usize>().ok(), row.get(1), row.get(2))` | self.lips.entry | Handled | [340](../../crates/crysirror-catsystem2/src/native_m2.rs#L340) |
| `condition; under name.as_str()="lip_flow"` | `let (Some(position), Some(key), true) = (row[0].parse::<usize>().ok(), row.get(1), row.len() > 2)` | self.lips.entry | Handled | [347](../../crates/crysirror-catsystem2/src/native_m2.rs#L347) |
| `condition; under name.as_str()="touch"` | `let Some(name) = row[0].strip_prefix('$')` | self.touch_tables.entry | Handled | [369](../../crates/crysirror-catsystem2/src/native_m2.rs#L369) |
| `name.as_str()` | `"pattern"` | self.patterns.entry, Vec::new, self.patterns.get_mut | Handled | [267](../../crates/crysirror-catsystem2/src/native_m2.rs#L267) |
| `name.as_str()` | `"pattern_shift"` | self.shifts.entry | Handled | [280](../../crates/crysirror-catsystem2/src/native_m2.rs#L280) |
| `name.as_str()` | `"chain"` | self.parse_chains | Handled | [290](../../crates/crysirror-catsystem2/src/native_m2.rs#L290) |
| `name.as_str()` | `"waiting"` | self.waiting_files.extend | Handled | [291](../../crates/crysirror-catsystem2/src/native_m2.rs#L291) |
| `name.as_str()` | `"material" \| "material_min" \| "material_max"` | Local state/value operations | Handled | [292](../../crates/crysirror-catsystem2/src/native_m2.rs#L292) |
| `name.as_str()` | `"inertia_lr"` | self.inertia_horizontal.extend | Handled | [309](../../crates/crysirror-catsystem2/src/native_m2.rs#L309) |
| `name.as_str()` | `"inertia_ud"` | self.inertia_vertical.extend | Handled | [310](../../crates/crysirror-catsystem2/src/native_m2.rs#L310) |
| `name.as_str()` | `"inertia_lv" \| "inertia_enable"` | String::new | Handled | [311](../../crates/crysirror-catsystem2/src/native_m2.rs#L311) |
| `name.as_str()` | `"inertia_dif"` | self.inertia_links.push | Handled | [321](../../crates/crysirror-catsystem2/src/native_m2.rs#L321) |
| `name.as_str()` | `"lip_table"` | self.lips.entry | Handled | [331](../../crates/crysirror-catsystem2/src/native_m2.rs#L331) |
| `name.as_str()` | `"lip_variable"` | self.lips.entry | Handled | [338](../../crates/crysirror-catsystem2/src/native_m2.rs#L338) |
| `name.as_str()` | `"lip_flow"` | self.lips.entry | Handled | [345](../../crates/crysirror-catsystem2/src/native_m2.rs#L345) |
| `name.as_str()` | `"attribute"` | self.attributes.insert | Handled | [355](../../crates/crysirror-catsystem2/src/native_m2.rs#L355) |
| `name.as_str()` | `"touch"` | u32::from_str_radix, self.touch.insert, self.touch_tables.entry, self.touch_tables.get_mut | Handled | [364](../../crates/crysirror-catsystem2/src/native_m2.rs#L364) |
| `name.as_str()` | `"blink"` | self.blinks.push | Handled | [377](../../crates/crysirror-catsystem2/src/native_m2.rs#L377) |
| `name.as_str()` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [386](../../crates/crysirror-catsystem2/src/native_m2.rs#L386) |

### native_m2.rs / M2Config::get

```rust
fn get(&self, key: &[u8], label: &str) -> Option<u32>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `key` | `b"motion"` | Local state/value operations | Handled | [109](../../crates/crysirror-catsystem2/src/native_m2.rs#L109) |
| `key` | `b"eff_dif"` | Local state/value operations | Handled | [110](../../crates/crysirror-catsystem2/src/native_m2.rs#L110) |
| `key` | `b"eff_iner"` | Local state/value operations | Handled | [111](../../crates/crysirror-catsystem2/src/native_m2.rs#L111) |
| `key` | `b"eff_blink"` | Local state/value operations | Handled | [112](../../crates/crysirror-catsystem2/src/native_m2.rs#L112) |
| `key` | `b"eff_lip"` | Local state/value operations | Handled | [113](../../crates/crysirror-catsystem2/src/native_m2.rs#L113) |
| `key` | `b"fade"` | Local state/value operations | Handled | [114](../../crates/crysirror-catsystem2/src/native_m2.rs#L114) |
| `key` | `b"interleave"` | Local state/value operations | Handled | [115](../../crates/crysirror-catsystem2/src/native_m2.rs#L115) |
| `key` | `b"iner_lv"` | self.levels.get | Handled | [116](../../crates/crysirror-catsystem2/src/native_m2.rs#L116) |
| `key` | `b"iner_enable"` | self.enables.get | Handled | [117](../../crates/crysirror-catsystem2/src/native_m2.rs#L117) |
| `key` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [118](../../crates/crysirror-catsystem2/src/native_m2.rs#L118) |

### native_m2.rs / M2Config::set

```rust
fn set(&mut self, key: &[u8], value: Option<u32>, label: &str) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `key` | `b"motion"` | Local state/value operations | Handled | [125](../../crates/crysirror-catsystem2/src/native_m2.rs#L125) |
| `key` | `b"eff_dif"` | Local state/value operations | Handled | [126](../../crates/crysirror-catsystem2/src/native_m2.rs#L126) |
| `key` | `b"eff_iner"` | Local state/value operations | Handled | [127](../../crates/crysirror-catsystem2/src/native_m2.rs#L127) |
| `key` | `b"eff_blink"` | Local state/value operations | Handled | [128](../../crates/crysirror-catsystem2/src/native_m2.rs#L128) |
| `key` | `b"eff_lip"` | Local state/value operations | Handled | [129](../../crates/crysirror-catsystem2/src/native_m2.rs#L129) |
| `key` | `b"fade"` | Local state/value operations | Handled | [130](../../crates/crysirror-catsystem2/src/native_m2.rs#L130) |
| `key` | `b"interleave"` | Local state/value operations | Handled | [131](../../crates/crysirror-catsystem2/src/native_m2.rs#L131) |
| `key` | `b"iner_lv"` | self.levels.insert | Handled | [132](../../crates/crysirror-catsystem2/src/native_m2.rs#L132) |
| `key` | `b"iner_enable"` | self.enables.insert | Handled | [135](../../crates/crysirror-catsystem2/src/native_m2.rs#L135) |
| `key` | `b"reset"` | Self::default | Handled | [138](../../crates/crysirror-catsystem2/src/native_m2.rs#L138) |
| `key` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [139](../../crates/crysirror-catsystem2/src/native_m2.rs#L139) |

### native_m2.rs / Profile::apply_dif

```rust
fn apply_dif(&mut self, text: &str) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `name.as_str()` | `"scale"` | Local state/value operations | Handled | [66](../../crates/crysirror-catsystem2/src/native_m2.rs#L66) |
| `name.as_str()` | `"position"` | Local state/value operations | Handled | [67](../../crates/crysirror-catsystem2/src/native_m2.rs#L67) |
| `name.as_str()` | `"scaleoffset"` | self.scale_offset.unwrap_or | Handled | [68](../../crates/crysirror-catsystem2/src/native_m2.rs#L68) |
| `name.as_str()` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [77](../../crates/crysirror-catsystem2/src/native_m2.rs#L77) |

### native_m2.rs / Profile::parse_pfl

```rust
fn parse_pfl(text: &str) -> Self
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `(name.as_str(), row.as_slice())` | `("charaname", [id, _, file, ..])` | Local state/value operations | Handled | [43](../../crates/crysirror-catsystem2/src/native_m2.rs#L43) |
| `(name.as_str(), row.as_slice())` | `("motname", [body, wait, motion, dif, rest @ ..])` | Local state/value operations | Handled | [48](../../crates/crysirror-catsystem2/src/native_m2.rs#L48) |
| `(name.as_str(), row.as_slice())` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [56](../../crates/crysirror-catsystem2/src/native_m2.rs#L56) |

### native_menu.rs / collect

```rust
fn collect(items: &[MenuTemplateItem], used: &mut BTreeSet<u32>) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Some(id) = i.id.filter(\|id\| *id != 0)` | Local state/value operations | Partial: unsupported duplicate/out-of-range native menu command | [98](../../crates/crysirror-catsystem2/src/native_menu.rs#L98) |

### native_menu.rs / convert

```rust
fn convert(
        items: &[MenuTemplateItem],
        used: &mut BTreeSet<u32>,
        next: &mut u32,
    ) -> Result<Vec<MenuItemSpec>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Some(id) = item.id.filter(\|id\| *id != 0)` | Local state/value operations | Handled | [127](../../crates/crysirror-catsystem2/src/native_menu.rs#L127) |

### native_misc.rs / apply

```rust
fn apply(id: u32, args: &[u32], machine: &Machine) -> Result<Outcome>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `id` | `11` | std::time::SystemTime::now | Handled | [55](../../crates/crysirror-catsystem2/src/native_misc.rs#L55) |
| `id` | `218` | i32::from_le_bytes | Handled | [59](../../crates/crysirror-catsystem2/src/native_misc.rs#L59) |
| `id` | `786` | Local state/value operations | Handled | [67](../../crates/crysirror-catsystem2/src/native_misc.rs#L67) |
| `id` | `875` | Local state/value operations | Handled | [68](../../crates/crysirror-catsystem2/src/native_misc.rs#L68) |
| `id` | `1045` | Local state/value operations | Handled | [69](../../crates/crysirror-catsystem2/src/native_misc.rs#L69) |
| `id` | `10 \| 1039 \| 445 \| 629 \| 932 \| 933 \| 845 \| 847` | Local state/value operations | Handled | [83](../../crates/crysirror-catsystem2/src/native_misc.rs#L83) |
| `id` | `852 \| 566` | Local state/value operations | Handled | [85](../../crates/crysirror-catsystem2/src/native_misc.rs#L85) |
| `id` | `_` | Local state/value operations | Handled | [86](../../crates/crysirror-catsystem2/src/native_misc.rs#L86) |

### native_misc.rs / null_handle

```rust
fn null_handle(id: u32, args: &[u32]) -> Option<Option<u32>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `id` | `269 \| 266 \| 998 \| 635 \| 636 \| 639 \| 640 \| 608 \| 980 \| 1033 \| 872 \| 873 \| 877 \| 685` | Local state/value operations | Handled | [46](../../crates/crysirror-catsystem2/src/native_misc.rs#L46) |
| `id` | `267 \| 270 \| 417 \| 788 \| 684 \| 697 \| 700 \| 785 \| 876 \| 977 \| 870 \| 878 \| 634 \| 723 \| 726 \| 727 \| 730 \| 737 \| 713 \| 711` | Local state/value operations | Handled | [47](../../crates/crysirror-catsystem2/src/native_misc.rs#L47) |
| `id` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [48](../../crates/crysirror-catsystem2/src/native_misc.rs#L48) |

### native_motion.rs / pose_writes

```rust
fn pose_writes(a: &[u32], pose: PoseWords) -> Vec<(u32, Vec<u8>)>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `a[9] != 0` | Local state/value operations | Handled | [24](../../crates/crysirror-catsystem2/src/native_motion.rs#L24) |

### native_scalar.rs / apply

```rust
fn apply(id: u32, arguments: &[u32]) -> Option<Option<u32>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `id` | `439 \| 543..=548 \| 550 \| 552 \| 556 \| 559..=563 \| 573 \| 578 \| 579 \| 581 \| 642..=644 \| 652 \| 670..=673 \| 686 \| 687 \| 974..=976` | Local state/value operations | Handled | [11](../../crates/crysirror-catsystem2/src/native_scalar.rs#L11) |
| `id` | `549 \| 551 \| 554 \| 555 \| 575 \| 580` | Local state/value operations | Handled | [14](../../crates/crysirror-catsystem2/src/native_scalar.rs#L14) |
| `id` | `925` | Local state/value operations | Handled | [16](../../crates/crysirror-catsystem2/src/native_scalar.rs#L16) |
| `id` | `6` | Local state/value operations | Handled | [18](../../crates/crysirror-catsystem2/src/native_scalar.rs#L18) |
| `id` | `874` | Local state/value operations | Handled | [20](../../crates/crysirror-catsystem2/src/native_scalar.rs#L20) |
| `id` | `50` | Local state/value operations | Handled | [24](../../crates/crysirror-catsystem2/src/native_scalar.rs#L24) |
| `id` | `51` | Local state/value operations | Handled | [25](../../crates/crysirror-catsystem2/src/native_scalar.rs#L25) |
| `id` | `54` | Local state/value operations | Handled | [26](../../crates/crysirror-catsystem2/src/native_scalar.rs#L26) |
| `id` | `57` | Local state/value operations | Handled | [27](../../crates/crysirror-catsystem2/src/native_scalar.rs#L27) |
| `id` | `62` | Local state/value operations | Handled | [28](../../crates/crysirror-catsystem2/src/native_scalar.rs#L28) |
| `id` | `63` | Local state/value operations | Handled | [29](../../crates/crysirror-catsystem2/src/native_scalar.rs#L29) |
| `id` | `461` | Local state/value operations | Handled | [30](../../crates/crysirror-catsystem2/src/native_scalar.rs#L30) |
| `id` | `61` | Local state/value operations | Handled | [31](../../crates/crysirror-catsystem2/src/native_scalar.rs#L31) |
| `id` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [35](../../crates/crysirror-catsystem2/src/native_scalar.rs#L35) |

### native_settings.rs / configuration_offset

```rust
fn configuration_offset(name: &[u8]) -> Option<usize>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `name` | `b"vol_master"` | Local state/value operations | Handled | [435](../../crates/crysirror-catsystem2/src/native_settings.rs#L435) |
| `name` | `b"vol_bgm"` | Local state/value operations | Handled | [436](../../crates/crysirror-catsystem2/src/native_settings.rs#L436) |
| `name` | `b"vol_se"` | Local state/value operations | Handled | [437](../../crates/crysirror-catsystem2/src/native_settings.rs#L437) |
| `name` | `b"vol_voice"` | Local state/value operations | Handled | [438](../../crates/crysirror-catsystem2/src/native_settings.rs#L438) |
| `name` | `b"vol_play"` | Local state/value operations | Handled | [439](../../crates/crysirror-catsystem2/src/native_settings.rs#L439) |
| `name` | `b"mute_master"` | Local state/value operations | Handled | [440](../../crates/crysirror-catsystem2/src/native_settings.rs#L440) |
| `name` | `b"mute_bgm"` | Local state/value operations | Handled | [441](../../crates/crysirror-catsystem2/src/native_settings.rs#L441) |
| `name` | `b"mute_se"` | Local state/value operations | Handled | [442](../../crates/crysirror-catsystem2/src/native_settings.rs#L442) |
| `name` | `b"mute_voice"` | Local state/value operations | Handled | [443](../../crates/crysirror-catsystem2/src/native_settings.rs#L443) |
| `name` | `b"mute_play"` | Local state/value operations | Handled | [444](../../crates/crysirror-catsystem2/src/native_settings.rs#L444) |
| `name` | `b"messpd"` | Local state/value operations | Handled | [445](../../crates/crysirror-catsystem2/src/native_settings.rs#L445) |
| `name` | `b"autospd"` | Local state/value operations | Handled | [446](../../crates/crysirror-catsystem2/src/native_settings.rs#L446) |
| `name` | `b"autoind"` | Local state/value operations | Handled | [447](../../crates/crysirror-catsystem2/src/native_settings.rs#L447) |
| `name` | `b"autospd_b"` | Local state/value operations | Handled | [448](../../crates/crysirror-catsystem2/src/native_settings.rs#L448) |
| `name` | `b"autospd_c"` | Local state/value operations | Handled | [449](../../crates/crysirror-catsystem2/src/native_settings.rs#L449) |
| `name` | `b"stillread"` | Local state/value operations | Handled | [450](../../crates/crysirror-catsystem2/src/native_settings.rs#L450) |
| `name` | `b"contauto"` | Local state/value operations | Handled | [451](../../crates/crysirror-catsystem2/src/native_settings.rs#L451) |
| `name` | `b"waitpcm"` | Local state/value operations | Handled | [452](../../crates/crysirror-catsystem2/src/native_settings.rs#L452) |
| `name` | `b"fullskip"` | Local state/value operations | Handled | [453](../../crates/crysirror-catsystem2/src/native_settings.rs#L453) |
| `name` | `b"enable_readcol"` | Local state/value operations | Handled | [454](../../crates/crysirror-catsystem2/src/native_settings.rs#L454) |
| `name` | `b"readcol"` | Local state/value operations | Handled | [455](../../crates/crysirror-catsystem2/src/native_settings.rs#L455) |
| `name` | `b"skip_speed"` | Local state/value operations | Handled | [456](../../crates/crysirror-catsystem2/src/native_settings.rs#L456) |
| `name` | `b"contskip"` | Local state/value operations | Handled | [457](../../crates/crysirror-catsystem2/src/native_settings.rs#L457) |
| `name` | `b"strscroll"` | Local state/value operations | Handled | [458](../../crates/crysirror-catsystem2/src/native_settings.rs#L458) |
| `name` | `b"ctrlmode"` | Local state/value operations | Handled | [459](../../crates/crysirror-catsystem2/src/native_settings.rs#L459) |
| `name` | `b"autoleft"` | Local state/value operations | Handled | [460](../../crates/crysirror-catsystem2/src/native_settings.rs#L460) |
| `name` | `b"enable_readselcol"` | Local state/value operations | Handled | [461](../../crates/crysirror-catsystem2/src/native_settings.rs#L461) |
| `name` | `b"effspd"` | Local state/value operations | Handled | [462](../../crates/crysirror-catsystem2/src/native_settings.rs#L462) |
| `name` | `b"effcut"` | Local state/value operations | Handled | [463](../../crates/crysirror-catsystem2/src/native_settings.rs#L463) |
| `name` | `b"autocur"` | Local state/value operations | Handled | [464](../../crates/crysirror-catsystem2/src/native_settings.rs#L464) |
| `name` | `b"autodel"` | Local state/value operations | Handled | [465](../../crates/crysirror-catsystem2/src/native_settings.rs#L465) |
| `name` | `b"dorama"` | Local state/value operations | Handled | [466](../../crates/crysirror-catsystem2/src/native_settings.rs#L466) |
| `name` | `b"motcut"` | Local state/value operations | Handled | [467](../../crates/crysirror-catsystem2/src/native_settings.rs#L467) |
| `name` | `b"wndcolor"` | Local state/value operations | Handled | [468](../../crates/crysirror-catsystem2/src/native_settings.rs#L468) |
| `name` | `b"dragwnd"` | Local state/value operations | Handled | [469](../../crates/crysirror-catsystem2/src/native_settings.rs#L469) |
| `name` | `b"wndframe"` | Local state/value operations | Handled | [470](../../crates/crysirror-catsystem2/src/native_settings.rs#L470) |
| `name` | `b"contvoice"` | Local state/value operations | Handled | [471](../../crates/crysirror-catsystem2/src/native_settings.rs#L471) |
| `name` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [472](../../crates/crysirror-catsystem2/src/native_settings.rs#L472) |

### native_settings.rs / optional_native_asset

```rust
fn optional_native_asset(
    storage: &dyn ClientStorage,
    startup: &XmlDocument,
    path: &str,
) -> Result<Option<Vec<u8>>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Some(dir) = fields .iter() .find(\|n\| n.name == "dir") .filter(\|n\| !n.text.is_empty())` | Local state/value operations | Handled | [512](../../crates/crysirror-catsystem2/src/native_settings.rs#L512) |

### native_settings.rs / NativeSettings::configuration

```rust
fn configuration(&self, name: &[u8], first: u32, second: u32) -> Result<u32>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `name` | `b"lang_ui"` | Local state/value operations | Handled | [305](../../crates/crysirror-catsystem2/src/native_settings.rs#L305) |
| `name` | `b"lang_text_main"` | Local state/value operations | Handled | [306](../../crates/crysirror-catsystem2/src/native_settings.rs#L306) |
| `name` | `b"lang_text_sub"` | Local state/value operations | Handled | [307](../../crates/crysirror-catsystem2/src/native_settings.rs#L307) |
| `name` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [308](../../crates/crysirror-catsystem2/src/native_settings.rs#L308) |
| `condition` | `name == b"play"` | self.get | Handled | [310](../../crates/crysirror-catsystem2/src/native_settings.rs#L310) |
| `condition` | `name == b"sysplay"` | Local state/value operations | Handled | [318](../../crates/crysirror-catsystem2/src/native_settings.rs#L318) |

### native_settings.rs / NativeSettings::set_configuration

```rust
fn set_configuration(
        &mut self,
        name: &[u8],
        value: u32,
        first: u32,
        second: u32,
    ) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Some(at) = [b"lang_ui".as_slice(), b"lang_text_main", b"lang_text_sub"].iter().position(\|n\| *n == name)` | Local state/value operations | Handled | [342](../../crates/crysirror-catsystem2/src/native_settings.rs#L342) |
| `condition` | `name == b"play"` | self.put | Handled | [344](../../crates/crysirror-catsystem2/src/native_settings.rs#L344) |
| `condition` | `name == b"sysplay"` | Local state/value operations | Handled | [349](../../crates/crysirror-catsystem2/src/native_settings.rs#L349) |
| `condition` | `matches!(name, b"autospd_b" \| b"autospd_c" \| b"autoind")` | self.put | Handled | [363](../../crates/crysirror-catsystem2/src/native_settings.rs#L363) |
| `condition` | `matches!(name, b"autospd_b" \| b"autospd_c" \| b"autoind")` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [363](../../crates/crysirror-catsystem2/src/native_settings.rs#L363) |
| `condition` | `name == b"autospd"` | Local state/value operations | Handled | [367](../../crates/crysirror-catsystem2/src/native_settings.rs#L367) |

### native_settings.rs / NativeSettings::startup

```rust
fn startup(&mut self, d: &XmlDocument) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Some(v) = value(d, &format!("{path}/{name}"))?` | self.string | Handled | [156](../../crates/crysirror-catsystem2/src/native_settings.rs#L156) |
| `condition` | `name == "int"` | Local state/value operations | Handled | [157](../../crates/crysirror-catsystem2/src/native_settings.rs#L157) |

### native_sound_policy.rs / SoundPolicy::get

```rust
fn get(
        &self,
        selector: &[u8],
        target: &[Vec<u8>],
        interpreter: &mut crate::native_interpreter::Interpreter,
    ) -> Result<u32>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Some(s) = selector.strip_suffix("_id")` | Local state/value operations | Handled | [263](../../crates/crysirror-catsystem2/src/native_sound_policy.rs#L263) |
| `condition` | `let Some(s) = selector.strip_suffix("_type")` | Local state/value operations | Handled | [265](../../crates/crysirror-catsystem2/src/native_sound_policy.rs#L265) |
| `condition` | `mode == 1` | self.parts.as_slice | Handled | [284](../../crates/crysirror-catsystem2/src/native_sound_policy.rs#L284) |
| `condition` | `mode == 0 && target == "master" && matches!(field, 0 \| 1 \| 5)` | Local state/value operations | Handled | [302](../../crates/crysirror-catsystem2/src/native_sound_policy.rs#L302) |
| `condition` | `mode == 2` | Local state/value operations | Handled | [308](../../crates/crysirror-catsystem2/src/native_sound_policy.rs#L308) |

### native_sound_policy.rs / SoundPolicy::set

```rust
fn set(
        &mut self,
        selector: &[u8],
        target: &[Vec<u8>],
        value: u32,
        interpreter: &mut crate::native_interpreter::Interpreter,
    ) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Some(s) = selector.strip_suffix("_id")` | Local state/value operations | Handled | [399](../../crates/crysirror-catsystem2/src/native_sound_policy.rs#L399) |
| `condition` | `let Some(s) = selector.strip_suffix("_type")` | Local state/value operations | Handled | [401](../../crates/crysirror-catsystem2/src/native_sound_policy.rs#L401) |
| `condition` | `mode == 1` | Local state/value operations | Handled | [415](../../crates/crysirror-catsystem2/src/native_sound_policy.rs#L415) |
| `condition` | `mode == 2` | Local state/value operations | Handled | [441](../../crates/crysirror-catsystem2/src/native_sound_policy.rs#L441) |

### native_store.rs / apply

```rust
fn apply(
    id: u32,
    args: &[u32],
    machine: &mut Machine,
    saves: &mut BTreeMap<u32, Container>,
    next_save: &mut u32,
    stores: &mut Stores,
) -> Result<Outcome>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `id` | `238` | Container::new | Handled | [34](../../crates/crysirror-catsystem2/src/native_store.rs#L34) |
| `id` | `246` | Local state/value operations | Handled | [35](../../crates/crysirror-catsystem2/src/native_store.rs#L35) |
| `id` | `248` | Local state/value operations | Handled | [39](../../crates/crysirror-catsystem2/src/native_store.rs#L39) |
| `id` | `244 \| 835` | Local state/value operations | Handled | [40](../../crates/crysirror-catsystem2/src/native_store.rs#L40) |
| `id` | `242` | Local state/value operations | Handled | [48](../../crates/crysirror-catsystem2/src/native_store.rs#L48) |
| `id` | `252` | Local state/value operations | Handled | [58](../../crates/crysirror-catsystem2/src/native_store.rs#L58) |
| `id` | `254` | Local state/value operations | Handled | [62](../../crates/crysirror-catsystem2/src/native_store.rs#L62) |
| `id` | `388` | Local state/value operations | Handled | [69](../../crates/crysirror-catsystem2/src/native_store.rs#L69) |
| `id` | `387` | Local state/value operations | Handled | [77](../../crates/crysirror-catsystem2/src/native_store.rs#L77) |
| `id` | `389` | Local state/value operations | Handled | [84](../../crates/crysirror-catsystem2/src/native_store.rs#L84) |
| `id` | `390` | Local state/value operations | Handled | [91](../../crates/crysirror-catsystem2/src/native_store.rs#L91) |
| `id` | `469` | Local state/value operations | Handled | [95](../../crates/crysirror-catsystem2/src/native_store.rs#L95) |
| `id` | `833` | Local state/value operations | Handled | [101](../../crates/crysirror-catsystem2/src/native_store.rs#L101) |
| `id` | `834` | Local state/value operations | Handled | [102](../../crates/crysirror-catsystem2/src/native_store.rs#L102) |
| `id` | `836` | Local state/value operations | Handled | [106](../../crates/crysirror-catsystem2/src/native_store.rs#L106) |
| `id` | `_` | Local state/value operations | Handled | [114](../../crates/crysirror-catsystem2/src/native_store.rs#L114) |

### native_strings.rs / apply

```rust
fn apply(id: u32, a: &[u32], machine: &Machine) -> Result<Option<Outcome>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under id=44` | `a[3] == u32::MAX` | Local state/value operations | Handled | [115](../../crates/crysirror-catsystem2/src/native_strings.rs#L115) |
| `condition; under id=540` | `word >= a[1]` | Local state/value operations | Handled | [180](../../crates/crysirror-catsystem2/src/native_strings.rs#L180) |
| `id` | `25` | crate::native_values::format_bytes, crate::native_values::integer_prefix, Vec::new | Handled | [89](../../crates/crysirror-catsystem2/src/native_strings.rs#L89) |
| `id` | `38` | Local state/value operations | Handled | [94](../../crates/crysirror-catsystem2/src/native_strings.rs#L94) |
| `id` | `43` | Vec::new | Handled | [104](../../crates/crysirror-catsystem2/src/native_strings.rs#L104) |
| `id` | `44` | Vec::new | Handled | [113](../../crates/crysirror-catsystem2/src/native_strings.rs#L113) |
| `id` | `45` | Local state/value operations | Handled | [133](../../crates/crysirror-catsystem2/src/native_strings.rs#L133) |
| `id` | `46` | Vec::new | Handled | [134](../../crates/crysirror-catsystem2/src/native_strings.rs#L134) |
| `id` | `47` | Vec::new | Handled | [141](../../crates/crysirror-catsystem2/src/native_strings.rs#L141) |
| `id` | `49` | Vec::new | Handled | [150](../../crates/crysirror-catsystem2/src/native_strings.rs#L150) |
| `id` | `519` | Local state/value operations | Handled | [163](../../crates/crysirror-catsystem2/src/native_strings.rs#L163) |
| `id` | `540` | Vec::new | Handled | [174](../../crates/crysirror-catsystem2/src/native_strings.rs#L174) |
| `id` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [196](../../crates/crysirror-catsystem2/src/native_strings.rs#L196) |

### native_surface_ops.rs / colour_mode

```rust
fn colour_mode(frame: &Prepared, mode: u32) -> Option<Edit>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `mode` | `1` | Local state/value operations | Handled | [34](../../crates/crysirror-catsystem2/src/native_surface_ops.rs#L34) |
| `mode` | `2` | Local state/value operations | Handled | [35](../../crates/crysirror-catsystem2/src/native_surface_ops.rs#L35) |
| `mode` | `3` | Local state/value operations | Handled | [36](../../crates/crysirror-catsystem2/src/native_surface_ops.rs#L36) |
| `mode` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [37](../../crates/crysirror-catsystem2/src/native_surface_ops.rs#L37) |

### native_task_result.rs / Results::construct

```rust
fn construct(&mut self, id: u32) -> Result<(), &'static str>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `id >= 65536 \|\| self.slots.contains_key(&id)` | Local state/value operations | Unsupported: native task result slot ownership | [11](../../crates/crysirror-catsystem2/src/native_task_result.rs#L11) |

### sound_definition.rs / load

```rust
fn load(
    storage: &dyn ClientStorage,
    check: &dyn Fn() -> Result<()>,
) -> Result<SoundDefinition>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `fields .iter() .any(\|n\| n.name == "name" && n.text.eq_ignore_ascii_case("config"))` | Local state/value operations | Handled | [76](../../crates/crysirror-catsystem2/src/sound_definition.rs#L76) |
