# System-NNN developer reference

[All engines](README.md)

Mode: `adopt`.

## ABI and host contract

The installed-game bootstrap is the reviewed **MUGEN2 / Legacy1000** profile.
SPT PCs and operands are word-addressed; command identity includes native
profile, command code and payload. Source32 codecs/control layouts are explicit
source profiles, not proof of a Source32 installed-game bootstrap.

| Native area | CrySirror service / implementation | Return / completion |
| --- | --- | --- |
| SPT VM / expressions | `machine.rs` / `expression.rs` | Native words, values and retained original PC. |
| Native commands / actors | `bridge.rs` and actor modules | Payload/profile validation precedes admission; actor completion resumes the VM. |
| Picture slots, wipes, particles and geometry | Shared Image/Layer/VFX services | Native 16-slot/effect state and saved continuations remain native. |
| Text, chart, choice and configuration UI | Shared GlyphRun/UI/Input | Native result/callback semantics; some screens use shared controls. |
| BGM, script sound, voice and movies | Shared Audio/Media workers | Native channel, loop and fade policy; pending callbacks need a safe boundary before load. |
| SystemFile and game saves | Native codecs over scoped Storage | Explicit record layouts, independent banks and actor reconstruction. |

Remaining domains include complete native menu/config skins, map-editor
persistence, Source32 bootstrap, saved outer actors outside admitted profiles,
custom animation callbacks, unconnected secondary modes and SelectPlace
character/auto-load/hover/keyboard behavior. The table's unsupported/default
routes are authoritative for individual arguments. Recognition of a DWQ/VAW
resource does not establish its script/actor compatibility.

Source/provenance: [SOURCE.json](../../crates/crysirror-system-nnn/SOURCE.json),
[command bridge](../../crates/crysirror-system-nnn/src/bridge.rs).
Behavior: [focused validation](../validation/system-nnn-alice-2026-11-09.md).

<!-- BEGIN GENERATED DISPATCH TABLES -->

## Opcode and API implementation tables

`Handled` means an executable source branch exists. It is not a claim that every argument/subselector or a complete game has been audited. Explicit gaps are shown per row.

| Status | Entries |
| --- | ---: |
| Handled | 120 |

These tables cover the checked-in catalogues and explicit dispatch branches. An engine-wide upstream denominator is not inferred from these rows.

## Legacy1000 VM instructions

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x55560001` | The outer original YoyakuExecRoutine immediately resumes ENDIF/ENDKOMA without any callback or host operation | Handled | — | [machine.rs:276](../../crates/crysirror-system-nnn/src/machine.rs#L276) |
| `0x55560002` | Queue native | Handled | — | [machine.rs:278](../../crates/crysirror-system-nnn/src/machine.rs#L278) |
| `0x55560003` | Update pc | Handled | — | [machine.rs:306](../../crates/crysirror-system-nnn/src/machine.rs#L306) |
| `0x55560004` | Update pc | Handled | — | [machine.rs:306](../../crates/crysirror-system-nnn/src/machine.rs#L306) |
| `0x55560005` | Update halted, executed | Handled | — | [machine.rs:309](../../crates/crysirror-system-nnn/src/machine.rs#L309) |
| `0x55560007` | Update pc | Handled | — | [machine.rs:314](../../crates/crysirror-system-nnn/src/machine.rs#L314) |
| `0x55560008` | Update pc | Handled | — | [machine.rs:306](../../crates/crysirror-system-nnn/src/machine.rs#L306) |
| `0x55560009` | Update pc | Handled | — | [machine.rs:324](../../crates/crysirror-system-nnn/src/machine.rs#L324) |
| `0x5556000E` | The outer original YoyakuExecRoutine immediately resumes ENDIF/ENDKOMA without any callback or host operation | Handled | — | [machine.rs:276](../../crates/crysirror-system-nnn/src/machine.rs#L276) |
| `0x55560011` | Queue native | Handled | — | [machine.rs:278](../../crates/crysirror-system-nnn/src/machine.rs#L278) |
| `0x55560013` | The outer original YoyakuExecRoutine immediately resumes ENDIF/ENDKOMA without any callback or host operation | Handled | — | [machine.rs:276](../../crates/crysirror-system-nnn/src/machine.rs#L276) |
| `0x55560020` | Update pc | Handled | — | [machine.rs:354](../../crates/crysirror-system-nnn/src/machine.rs#L354) |

## Legacy1000 native commands

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `(CallbackProfile::Mugen2Legacy, 0x3333001A, Payload::Words { count, words })` | Execute commit branch | Handled | — | [bridge.rs:3430](../../crates/crysirror-system-nnn/src/bridge.rs#L3430) |
| `(CallbackProfile::Mugen2Legacy, 0x33330052, Payload::Scalar(_))` | Execute commit branch | Handled | — | [bridge.rs:3516](../../crates/crysirror-system-nnn/src/bridge.rs#L3516) |
| `(CallbackProfile::Mugen2Legacy, 0x33330053, Payload::Scalar(value))` | Update configuration mask | Handled | — | [bridge.rs:3550](../../crates/crysirror-system-nnn/src/bridge.rs#L3550) |
| `(CallbackProfile::Mugen2Legacy, 0x33330055, Payload::Scalar(value))` | Update script mouse number | Handled | — | [bridge.rs:3558](../../crates/crysirror-system-nnn/src/bridge.rs#L3558) |
| `(CallbackProfile::Mugen2Legacy, 0x33330056, Payload::Scalar(value))` | Update script cursor number | Handled | — | [bridge.rs:3565](../../crates/crysirror-system-nnn/src/bridge.rs#L3565) |
| `(CallbackProfile::Mugen2Legacy, 0x3333005B, Payload::Scalar(value))` | Update auxiliary / cutin mode | Handled | — | [bridge.rs:3577](../../crates/crysirror-system-nnn/src/bridge.rs#L3577) |
| `(CallbackProfile::Mugen2Legacy, 0x3333005C, Payload::Scalar(value))` | Update obsolete message exp wait | Handled | — | [bridge.rs:3569](../../crates/crysirror-system-nnn/src/bridge.rs#L3569) |
| `(CallbackProfile::Mugen2Legacy, 0x3333005D, Payload::Scalar(value))` | Update skip film enabled | Handled | — | [bridge.rs:3587](../../crates/crysirror-system-nnn/src/bridge.rs#L3587) |
| `(CallbackProfile::Mugen2Legacy, 0x3333005E, Payload::Scalar(_))` | Execute commit branch | Handled | — | [bridge.rs:3594](../../crates/crysirror-system-nnn/src/bridge.rs#L3594) |
| `(Payload::Voice { name, parameters, default_voice, }, 0x33330008)` | Audio playback/control (plan) | Handled | — | [native_audio.rs:319](../../crates/crysirror-system-nnn/src/native_audio.rs#L319) |
| `(Payload::Voice { parameters, .. }, 0x33330008)` | Execute admit with skip branch | Handled | — | [skip.rs:91](../../crates/crysirror-system-nnn/src/native_audio/skip.rs#L91) |
| `(Payload::Words { count, words }, 0x33330007)` | Execute admit with skip branch | Handled | — | [skip.rs:34](../../crates/crysirror-system-nnn/src/native_audio/skip.rs#L34) |
| `(Payload::Words { count, words }, 0x33330009)` | Execute admit with skip branch | Handled | — | [skip.rs:60](../../crates/crysirror-system-nnn/src/native_audio/skip.rs#L60) |
| `(_, 0x33330007, Payload::Words { count: 1, words })` | Update api calls | Handled | Guard: words == &[-1] | [bridge.rs:3500](../../crates/crysirror-system-nnn/src/bridge.rs#L3500) |
| `(_, 0x33330011, Payload::Empty)` | Submit | Handled | — | [bridge.rs:3451](../../crates/crysirror-system-nnn/src/bridge.rs#L3451) |
| `(_, 0x33330012, Payload::Empty)` | Clear all native effects | Handled | — | [bridge.rs:3450](../../crates/crysirror-system-nnn/src/bridge.rs#L3450) |
| `(_, 0x33330013, Payload::Words { count, words })` | Layer id | Handled | Guard: *count >= 2 && matches!(words[1], 2 \\| 8 \\| 12 \\| 13 \\| 14 \\| 15 \\| 19 \\| 20 \\| 22 \\| 23 \\| 24 \\| 26 \\| 27 \\| 28 \\| 32 \\| 33 \\| 36 \\| 37 \\| 40 \\| 42 \\| 43 \\| 48 \\| 50 \\| 51 \\| 55 \\| 56 \\| 57 \\| 59 \\| 60 \\| 61 \\| 62 \\| 63 \\| 64 \\| 65 \\| 66 \\| 67 \\| 68 \\| 69 \\| 70 \\| 71 \\| 72 \\| 73 \\| 75 \\| 76 \\| 77 \\| 78 \\| 79 \\| 80 \\| 81 \\| 82 \\| 83 \\| 85 \\| 86 \\| 87 \\| 88 \\| 89 \\| 90 \\| 91 \\| 92 \\| 93 \\| 94 \\| 95 \\| 96 \\| 97 \\| 98 \\| 99 \\| 101 \\| 102 \\| 103 \\| 104 \\| 105 \\| 106 \\| 107 \\| 108 \\| 110 \\| 111 \\| 112 \\| 113 \\| 114 \\| 115 \\| 116 \\| 117 \\| 118 \\| 119 \\| 120 \\| 121 \\| 122 \\| 123 \\| 124 \\| 125 \\| 126 \\| 127 \\| 130 \\| 131 \\| 132 \\| 133 \\| 136 \\| 137 \\| 138 \\| 139 \\| 140) | [bridge.rs:3700](../../crates/crysirror-system-nnn/src/bridge.rs#L3700) |
| `(_, 0x33330013, Payload::Words { words, .. })` | Layer id | Handled | Guard: code == 0x33330014 && words.len() == 1 \\|\\| code == 0x33330013 && words.len() >= 2 && words[1] == -1 | [bridge.rs:3658](../../crates/crysirror-system-nnn/src/bridge.rs#L3658) |
| `(_, 0x33330013, Payload::Words{count,words})` | Layer id | Handled | Guard: *count>=2 && (matches!(words[1],40\\|123) && words.len()<4 \\|\\| matches!(words[1],60\\|101) && words.len()<5) | [bridge.rs:3692](../../crates/crysirror-system-nnn/src/bridge.rs#L3692) |
| `(_, 0x33330014, Payload::Words { words, .. })` | Layer id | Handled | Guard: code == 0x33330014 && words.len() == 1 \\|\\| code == 0x33330013 && words.len() >= 2 && words[1] == -1 | [bridge.rs:3658](../../crates/crysirror-system-nnn/src/bridge.rs#L3658) |
| `(_, 0x33330016, Payload::Words { count, words })` | Layer id | Handled | — | [bridge.rs:3622](../../crates/crysirror-system-nnn/src/bridge.rs#L3622) |
| `(_, 0x33330017, Payload::Empty)` | Update message window visible | Handled | — | [bridge.rs:3522](../../crates/crysirror-system-nnn/src/bridge.rs#L3522) |
| `(_, 0x33330018, Payload::Words { count, words })` | Execute commit branch | Handled | — | [bridge.rs:3400](../../crates/crysirror-system-nnn/src/bridge.rs#L3400) |
| `(_, 0x33330019, Payload::Words { count: 1, words })` | Execute commit branch | Handled | Guard: words.len() == 1 | [bridge.rs:3396](../../crates/crysirror-system-nnn/src/bridge.rs#L3396) |
| `(_, 0x3333001B, Payload::Words { count, words })` | Message control | Handled | — | [bridge.rs:3465](../../crates/crysirror-system-nnn/src/bridge.rs#L3465) |
| `(_, 0x3333001C, Payload::Words { count, words })` | Message control | Handled | — | [bridge.rs:3465](../../crates/crysirror-system-nnn/src/bridge.rs#L3465) |
| `(_, 0x3333001E, Payload::Scalar(value))` | Update system file, legacy demo flag | Handled | — | [bridge.rs:3531](../../crates/crysirror-system-nnn/src/bridge.rs#L3531) |
| `(_, 0x33330028, Payload::Words { count: 1, words })` | Set frame time | Handled | Guard: words.len() == 1 | [bridge.rs:3382](../../crates/crysirror-system-nnn/src/bridge.rs#L3382) |
| `(_, 0x33330029, Payload::Empty)` | Set frame time | Handled | — | [bridge.rs:3392](../../crates/crysirror-system-nnn/src/bridge.rs#L3392) |
| `(_, 0x33330057, Payload::Scalar(value))` | Update auto message, message skip, cannot click | Handled | — | [bridge.rs:3599](../../crates/crysirror-system-nnn/src/bridge.rs#L3599) |
| `(_, 0x33330058, Payload::Scalar(value))` | Update auto message, message skip, cannot click | Handled | — | [bridge.rs:3599](../../crates/crysirror-system-nnn/src/bridge.rs#L3599) |
| `(_, 0x33330059, Payload::Scalar(value))` | Update auto message, message skip, cannot click | Handled | — | [bridge.rs:3599](../../crates/crysirror-system-nnn/src/bridge.rs#L3599) |
| `(_, 0x3333005A, Payload::Scalar(value))` | Update option off | Handled | — | [bridge.rs:3547](../../crates/crysirror-system-nnn/src/bridge.rs#L3547) |

## Picture effect selectors

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x02` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:431](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L431) |
| `0x08` | The first argument is saved but never controls countMax | Handled | — | [native_picture_effect.rs:442](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L442) |
| `0x0D` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:338](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L338) |
| `0x0F` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:315](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L315) |
| `0x13` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:345](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L345) |
| `0x14` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:413](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L413) |
| `0x16` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:336](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L336) |
| `0x1A` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:331](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L331) |
| `0x1B` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:332](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L332) |
| `0x1C` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:321](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L321) |
| `0x20` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:385](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L385) |
| `0x21` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:350](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L350) |
| `0x28` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:347](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L347) |
| `0x2A` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:319](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L319) |
| `0x30` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:342](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L342) |
| `0x33` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:329](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L329) |
| `0x37` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:351](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L351) |
| `0x38` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:337](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L337) |
| `0x3B` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:431](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L431) |
| `0x3C` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:322](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L322) |
| `0x3D` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:344](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L344) |
| `0x3E` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:401](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L401) |
| `0x3F` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:359](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L359) |
| `0x40` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:326](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L326) |
| `0x41` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:354](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L354) |
| `0x42` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:431](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L431) |
| `0x43` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:328](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L328) |
| `0x44` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:389](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L389) |
| `0x45` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:312](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L312) |
| `0x48` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:348](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L348) |
| `0x49` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:431](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L431) |
| `0x4B` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:431](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L431) |
| `0x4C` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:320](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L320) |
| `0x4D` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:405](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L405) |
| `0x4E` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:314](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L314) |
| `0x4F` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:340](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L340) |
| `0x50` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:316](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L316) |
| `0x52` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:311](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L311) |
| `0x55` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:317](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L317) |
| `0x56` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:343](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L343) |
| `0x5A` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:463](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L463) |
| `0x5B` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:335](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L335) |
| `0x5C` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:323](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L323) |
| `0x5D` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:330](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L330) |
| `0x5E` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:393](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L393) |
| `0x5F` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:339](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L339) |
| `0x60` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:349](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L349) |
| `0x62` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:325](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L325) |
| `0x63` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:478](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L478) |
| `0x65` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:322](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L322) |
| `0x66` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:318](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L318) |
| `0x69` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:324](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L324) |
| `0x6A` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:367](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L367) |
| `0x6B` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:371](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L371) |
| `0x6C` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:333](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L333) |
| `0x6E` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:363](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L363) |
| `0x6F` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:334](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L334) |
| `0x70` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:313](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L313) |
| `0x71` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:379](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L379) |
| `0x77` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:397](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L397) |
| `0x78` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:478](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L478) |
| `0x79` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:355](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L355) |
| `0x7A` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:356](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L356) |
| `0x7B` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:357](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L357) |
| `0x7C` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:449](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L449) |
| `0x7D` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:352](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L352) |
| `0x7E` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:353](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L353) |
| `0x7F` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:449](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L449) |
| `0x82` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:375](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L375) |
| `0x83` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:420](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L420) |
| `0x84` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:409](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L409) |
| `0x85` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:358](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L358) |
| `0x88` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:346](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L346) |
| `0x8B` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:341](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L341) |
| `0x8C` | Visual effects (parse) | Handled | — | [native_picture_effect.rs:327](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L327) |

## Subselectors and conditional dispatch

Patterns retain their source namespace and guards. `_` is the fallback of that table. Boolean/optional results follow the function's return contract. Call-site names identify the implementation to inspect, including indirect service calls.

### adapter.rs / NativeAdapter::advance

```rust
fn advance(
        &mut self,
        host: &mut dyn HostServices,
        input: InputFrame,
        budget: u32,
        schedule_next: bool,
    ) -> Result<VmYield>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under session.poll(1)?=Step::Native(call)` | `session.depth()==0&&matches!(call.payload,crate::machine::Payload::Control{code:0x55560006,..})` | Local state/value operations | Handled | [1017](../../crates/crysirror-system-nnn/src/adapter.rs#L1017) |

### adapter.rs / NativeAdapter::apply_menu_setting

```rust
fn apply_menu_setting(
        &mut self,
        id: i32,
        value: i32,
        host: &mut dyn HostServices,
    ) -> Result<Option<[i32; 4]>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `id` | `103 \| 203` | Local state/value operations | Handled | [99](../../crates/crysirror-system-nnn/src/adapter.rs#L99) |
| `id` | `104 \| 204` | Local state/value operations | Handled | [100](../../crates/crysirror-system-nnn/src/adapter.rs#L100) |
| `id` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [107](../../crates/crysirror-system-nnn/src/adapter.rs#L107) |

### adapter.rs / NativeAdapter::produce_native_save

```rust
fn produce_native_save(
        &mut self,
        thumbnail: crate::native_thumbnail::NativeThumbnail,
        stamp: crate::native_system_file::SystemStamp,
    ) -> Result<crate::native_save::NativeSaveFile>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Some(id) = configuration.variable_names.index(b"year")` | Local state/value operations | Handled | [377](../../crates/crysirror-system-nnn/src/adapter.rs#L377) |

### adapter/scene.rs / NativeAdapter::try_scene_boundary

```rust
fn try_scene_boundary(&mut self, host: &mut dyn HostServices) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `call.identify != 0x66660002 \|\| !matches!(call.code, 0x5556000c \| 0x55560006)` | Local state/value operations | Handled | [92](../../crates/crysirror-system-nnn/src/adapter/scene.rs#L92) |
| `condition` | `call.code == 0x5556000c` | Local state/value operations | Handled | [96](../../crates/crysirror-system-nnn/src/adapter/scene.rs#L96) |

### adapter/user_gaiji.rs / NativeAdapter::change_user_gaiji

```rust
fn change_user_gaiji(
        &mut self,
        host: &mut dyn HostServices,
        font: &[u8],
        characters: &[u8],
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `command == 0x8f49` | Local state/value operations | Handled | [77](../../crates/crysirror-system-nnn/src/adapter/user_gaiji.rs#L77) |

### bridge.rs / native_optional_cutin_and_film_latches_keep_inert_profiles_and_guard_active_consumers

```rust
fn native_optional_cutin_and_film_latches_keep_inert_profiles_and_guard_active_consumers() -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `code==0x3333005b` | Local state/value operations | Handled | [4964](../../crates/crysirror-system-nnn/src/bridge.rs#L4964) |
| `condition` | `code==0x3333005b` | Local state/value operations | Handled | [4972](../../crates/crysirror-system-nnn/src/bridge.rs#L4972) |

### bridge.rs / Bridge::adopt_draw_result

```rust
fn adopt_draw_result(
        &mut self,
        result: crate::native_draw::DrawResult,
        frames: i32,
        host: &mut dyn HostServices,
    ) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under effect=NativeFrameEffect::Move(frame)` | `frame.native_command == 0` | Local state/value operations | Handled | [2978](../../crates/crysirror-system-nnn/src/bridge.rs#L2978) |

### bridge.rs / Bridge::commit

```rust
fn commit(
        &mut self,
        machine: &mut Machine,
        call: &NativeCall,
        host: &mut dyn HostServices,
    ) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `self.skip.next_command && self.skip.effect_command && ((call.identify == 0x66660007 && matches!(call.code, 0x33330011 \| 0x33330015 \| 0x3333001f)) \|\| (call.identify == 0x66660006 && call.code == 0x22220005 && self.background_read_marks.is_none()))` | Local state/value operations | Handled | [3261](../../crates/crysirror-system-nnn/src/bridge.rs#L3261) |
| `condition` | `call.identify == 0x66660007 && call.code == 0x33330013 && self.pending_hamon.is_none() && self.pending_hahen.is_none() && self.skip.next_command && self.skip.effect_command` | Local state/value operations | Handled | [3275](../../crates/crysirror-system-nnn/src/bridge.rs#L3275) |
| `condition` | `name.eq_ignore_ascii_case(b"setresult") \|\| name.eq_ignore_ascii_case(b"getresult")` | self.result_data.as_mut | Handled | [3320](../../crates/crysirror-system-nnn/src/bridge.rs#L3320) |
| `condition` | `name.eq_ignore_ascii_case(b"stopf5")` | self.skip.stop_f5 | Handled | [3331](../../crates/crysirror-system-nnn/src/bridge.rs#L3331) |
| `condition` | `name.eq_ignore_ascii_case(b"layer")` | i32::from, self.layer_id, serde_json::from_slice, host.query_engine, crysirror_core::service::EngineQuery::Layer | Handled | [3336](../../crates/crysirror-system-nnn/src/bridge.rs#L3336) |
| `condition` | `call.identify != 0x66660007` | Local state/value operations | Unsupported: Unconditional rejection | [3377](../../crates/crysirror-system-nnn/src/bridge.rs#L3377) |
| `condition` | `matches!(call.code,0x33330013\|0x33330014)` | self.freeze_effect_backing | Handled | [3380](../../crates/crysirror-system-nnn/src/bridge.rs#L3380) |
| `condition; under (self.profile, call.code, &call.payload)=(_, code @ (0x3333001b \| 0x3333001c), Payload::Words { count, words })` | `code == 0x3333001b` | Local state/value operations | Handled | [3490](../../crates/crysirror-system-nnn/src/bridge.rs#L3490) |
| `code; under (self.profile, call.code, &call.payload)=(_, code @ (0x33330057 \| 0x33330058 \| 0x33330059), Payload::Scalar(value))` | `0x33330057` | Local state/value operations | Handled | [3602](../../crates/crysirror-system-nnn/src/bridge.rs#L3602) |
| `code; under (self.profile, call.code, &call.payload)=(_, code @ (0x33330057 \| 0x33330058 \| 0x33330059), Payload::Scalar(value))` | `0x33330058` | Local state/value operations | Handled | [3612](../../crates/crysirror-system-nnn/src/bridge.rs#L3612) |
| `code; under (self.profile, call.code, &call.payload)=(_, code @ (0x33330057 \| 0x33330058 \| 0x33330059), Payload::Scalar(value))` | `0x33330059` | Local state/value operations | Handled | [3613](../../crates/crysirror-system-nnn/src/bridge.rs#L3613) |
| `code; under (self.profile, call.code, &call.payload)=(_, code @ (0x33330057 \| 0x33330058 \| 0x33330059), Payload::Scalar(value))` | `_` | Local state/value operations | Handled | [3619](../../crates/crysirror-system-nnn/src/bridge.rs#L3619) |
| `(self.profile, call.code, &call.payload)` | `(_, 0x33330028, Payload::Words { count: 1, words }) if words.len() == 1` | self.set_frame_time | Handled | [3382](../../crates/crysirror-system-nnn/src/bridge.rs#L3382) |
| `(self.profile, call.code, &call.payload)` | `(_, 0x33330019, Payload::Words { count: 1, words }) if words.len() == 1` | Local state/value operations | Handled | [3396](../../crates/crysirror-system-nnn/src/bridge.rs#L3396) |
| `(self.profile, call.code, &call.payload)` | `(_, 0x33330007, Payload::Words { count: 1, words }) if words == &[-1]` | host.submit_audio | Handled | [3500](../../crates/crysirror-system-nnn/src/bridge.rs#L3500) |
| `(self.profile, call.code, &call.payload)` | `(_, code @ (0x33330014 \| 0x33330013), Payload::Words { words, .. }) if code == 0x33330014 && words.len() == 1 \|\| code == 0x33330013 && words.len() >= 2 && words[1] == -1` | self.layer_id, self.active_effects.contains, Self::native_layer_removal, self.submit, self.active_effects.remove | Handled | [3658](../../crates/crysirror-system-nnn/src/bridge.rs#L3658) |
| `(self.profile, call.code, &call.payload)` | `(_,0x33330013,Payload::Words{count,words}) if *count>=2 && (matches!(words[1],40\|123) && words.len()<4 \|\| matches!(words[1],60\|101) && words.len()<5)` | self.layer_id, self.previous_effects.insert | Handled | [3692](../../crates/crysirror-system-nnn/src/bridge.rs#L3692) |
| `(self.profile, call.code, &call.payload)` | `(_, 0x33330013, Payload::Words { count, words }) if *count >= 2 && matches!(words[1], 2 \| 8 \| 12 \| 13 \| 14 \| 15 \| 19 \| 20 \| 22 \| 23 \| 24 \| 26 \| 27 \| 28 \| 32 \| 33 \| 36 \| 37 \| 40 \| 42 \| 43 \| 48 \| 50 \| 51 \| 55 \| 56 \| 57 \| 59 \| 60 \| 61 \| 62 \| 63 \| 64 \| 65 \| 66 \| 67 \| 68 \| 69 \| 70 \| 71 \| 72 \| 73 \| 75 \| 76 \| 77 \| 78 \| 79 \| 80 \| 81 \| 82 \| 83 \| 85 \| 86 \| 87 \| 88 \| 89 \| 90 \| 91 \| 92 \| 93 \| 94 \| 95 \| 96 \| 97 \| 98 \| 99 \| 101 \| 102 \| 103 \| 104 \| 105 \| 106 \| 107 \| 108 \| 110 \| 111 \| 112 \| 113 \| 114 \| 115 \| 116 \| 117 \| 118 \| 119 \| 120 \| 121 \| 122 \| 123 \| 124 \| 125 \| 126 \| 127 \| 130 \| 131 \| 132 \| 133 \| 136 \| 137 \| 138 \| 139 \| 140)` | self.layer_id, crate::native_effects_mosaic::MosaicState::parse, crate::native_picture_effect::PictureEffectState::parse_for_canvas, self.previous_effects.get, self.animation.as_ref | Handled | [3700](../../crates/crysirror-system-nnn/src/bridge.rs#L3700) |
| `(self.profile, call.code, &call.payload)` | `(_, 0x33330013, Payload::Words { count, words }) if *count >= 2 && words[1] == 52` | self.layer_id, self.rectangles.get, crate::native_geometry::NativeRects::natural, crate::native_move::NativeMoveState::parse, self.restore_picture_source | Handled | [3876](../../crates/crysirror-system-nnn/src/bridge.rs#L3876) |
| `(self.profile, call.code, &call.payload)` | `(_, 0x33330013, Payload::Words { count, words }) if *count >= 2 && words[1] == 54` | self.layer_id, i32::from, self.submit, self.active_effects.insert, self.previous_effects.insert | Handled | [3907](../../crates/crysirror-system-nnn/src/bridge.rs#L3907) |
| `(self.profile, call.code, &call.payload)` | `(_, 0x33330013, Payload::Words { count, words }) if *count >= 2 && words[1] == 1` | self.layer_id | Handled | [3940](../../crates/crysirror-system-nnn/src/bridge.rs#L3940) |
| `(self.profile, call.code, &call.payload)` | `(_, 0x33330013, Payload::Words { count, words }) if *count >= 2 && words[1] == 74` | self.layer_id, crate::native_bura::BuraState::parse, self.restore_picture_source, self.fill_effects.remove, self.patterns.remove | Handled | [3950](../../crates/crysirror-system-nnn/src/bridge.rs#L3950) |
| `(self.profile, call.code, &call.payload)` | `(_, 0x33330013, Payload::Words { count, words }) if *count >= 2 && words[1] == 109` | self.layer_id, crate::native_old_lamp::OldLampState::parse, self.restore_picture_source, self.fill_effects.remove, self.patterns.remove | Handled | [3974](../../crates/crysirror-system-nnn/src/bridge.rs#L3974) |
| `(self.profile, call.code, &call.payload)` | `(_, 0x33330013, Payload::Words { count, words }) if *count >= 2 && words[1] == 9` | self.layer_id, crate::native_chara_trans::CharaTransState::parse, self.restore_picture_source, self.fill_effects.remove, self.patterns.remove | Handled | [3998](../../crates/crysirror-system-nnn/src/bridge.rs#L3998) |
| `(self.profile, call.code, &call.payload)` | `(_, 0x33330013, Payload::Words { count, words }) if *count >= 2 && words[1] == 17` | self.layer_id, crate::native_effects_flash::FlashState::parse, self.restore_picture_source, self.fill_effects.remove, self.patterns.remove | Handled | [4025](../../crates/crysirror-system-nnn/src/bridge.rs#L4025) |
| `(self.profile, call.code, &call.payload)` | `(_, 0x33330013, Payload::Words { count, words }) if *count >= 2 && matches!(words[1], 18 \| 100)` | self.layer_id, crate::native_effects_shake::ShakeState::parse, crate::native_effects_shake::ShakeState::parse_yure, self.restore_picture_source, self.fill_effects.remove | Handled | [4047](../../crates/crysirror-system-nnn/src/bridge.rs#L4047) |
| `(self.profile, call.code, &call.payload)` | `(_, 0x33330013, Payload::Words { count, words }) if *count >= 2 && words[1] == 39` | self.layer_id, crate::native_effects_scroll::ScrollState::parse, self.restore_picture_source, self.fill_effects.remove, self.patterns.remove | Handled | [4075](../../crates/crysirror-system-nnn/src/bridge.rs#L4075) |
| `(self.profile, call.code, &call.payload)` | `(_, 0x33330013, Payload::Words { count, words }) if *count >= 2 && words[1] == 84` | self.layer_id, crate::native_effects_fusuma::FusumaState::parse, self.restore_picture_source, self.fill_effects.remove, self.patterns.remove | Handled | [4102](../../crates/crysirror-system-nnn/src/bridge.rs#L4102) |
| `(self.profile, call.code, &call.payload)` | `(_, 0x33330013, Payload::Words { count, words }) if *count >= 2 && words[1] == 31` | self.layer_id, crate::native_effects::LayerGreyState::parse, self.restore_picture_source, self.fill_effects.remove, self.patterns.remove | Handled | [4129](../../crates/crysirror-system-nnn/src/bridge.rs#L4129) |
| `(self.profile, call.code, &call.payload)` | `(_, 0x33330013, Payload::Words { count, words }) if *count >= 2 && matches!(words[1], 128 \| 129)` | self.layer_id, crate::native_effects::LayerGreyState::parse_sepia, crate::native_effects::LayerGreyState::parse, self.restore_picture_source, self.fill_effects.remove | Handled | [4154](../../crates/crysirror-system-nnn/src/bridge.rs#L4154) |
| `(self.profile, call.code, &call.payload)` | `(_, 0x33330013, Payload::Words { count, words }) if *count >= 2 && matches!(words[1], 10 \| 11)` | self.layer_id, FadeInState::parse_out, FadeInState::parse, self.restore_picture_source, self.fill_effects.remove | Handled | [4182](../../crates/crysirror-system-nnn/src/bridge.rs#L4182) |
| `(self.profile, call.code, &call.payload)` | `(_, 0x33330013, Payload::Words { count, words }) if *count >= 2 && words[1] == 0` | self.layer_id, self.restore_picture_source, self.fill_effects.remove, self.patterns.remove, self.fades.remove | Handled | [4211](../../crates/crysirror-system-nnn/src/bridge.rs#L4211) |
| `(self.profile, call.code, &call.payload)` | `_` | Local state/value operations | Unsupported: unsupported System-NNN native pc={:#x} code={:#x}; exact pending call retained | [4233](../../crates/crysirror-system-nnn/src/bridge.rs#L4233) |

### bridge.rs / Bridge::frame_effects

```rust
fn frame_effects(&self) -> BTreeMap<i32, crate::native_presentation::NativeFrameEffect>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!uses_mask \|\| ((matches!(state.command(),65\|124\|125\|127\|131\|136) \|\| matches!(state.kind,crate::native_picture_effect::PictureEffectKind::Koma{parameters} if parameters[8]==2)) && !p.has_mask)` | Local state/value operations | Handled | [2401](../../crates/crysirror-system-nnn/src/bridge.rs#L2401) |

### bridge.rs / Bridge::frame_underlays

```rust
fn frame_underlays(
        &self,
        effects: &BTreeMap<i32, crate::native_presentation::NativeFrameEffect>,
        visible: &BTreeSet<i32>,
    ) -> Result<BTreeMap<i32, crysirror_core::image_processing::ImageCompositeLayer>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `self.layer_visible.get(slot) == Some(&false) \|\| self.chara_trans.contains_key(slot) \|\| self.old_lamps.contains_key(slot) \|\| self.moves.contains_key(slot) \|\| self.shakes.get(slot).is_some_and(\|state\| !state.needs_picture_underlay()) \|\| matches!( effects.get(slot), Some( NativeFrameEffect::Move(_) \| NativeFrameEffect::Scroll(_) \| NativeFrameEffect::Fusuma(_) ) ) \|\| matches!(effects.get(slot),Some(NativeFrameEffect::Picture(frame)) if !matches!(frame.state.kind,crate::native_picture_effect::PictureEffectKind::Circle{command:0,active:true,..})&&!matches!(frame.state.command(),12\|13\|14\|15\|57\|71\|83\|20\|22\|23\|78\|24\|26\|28\|32\|33\|42\|50\|51\|55\|68\|73\|85\|87\|88\|89\|103\|114\|115\|116\|117\|118))` | Local state/value operations | Handled | [2616](../../crates/crysirror-system-nnn/src/bridge.rs#L2616) |

### bridge.rs / Bridge::initialize_outer_newgame

```rust
fn initialize_outer_newgame(
        &mut self,
        host: &mut dyn HostServices,
        exit: Arc<dyn ImageResource>,
    ) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Some(op)=Self::native_layer_removal(self.client,host,self.bindings.effect_base+u32::from(self.bindings.layers)+1)?` | Local state/value operations | Handled | [828](../../crates/crysirror-system-nnn/src/bridge.rs#L828) |

### bridge.rs / Bridge::poll_replay_voice

```rust
fn poll_replay_voice(&mut self, host:&mut dyn HostServices, name:&[u8]) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `names.total_off(file)? \|\| file.system_word(196)? == 0 \|\| !names.allows_with_config(name,file,p.config_present)?` | Local state/value operations | Handled | [706](../../crates/crysirror-system-nnn/src/bridge.rs#L706) |

### bridge.rs / Bridge::try_commit

```rust
fn try_commit(
        &mut self,
        machine: &mut Machine,
        call: &NativeCall,
        host: &mut dyn HostServices,
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `self.skip.next_command && self.skip.effect_command && ((call.identify == 0x66660007 && matches!(call.code, 0x33330011 \| 0x33330015 \| 0x3333001f)) \|\| (call.identify == 0x66660006 && call.code == 0x22220005 && self.background_read_marks.is_none()))` | Local state/value operations | Handled | [1876](../../crates/crysirror-system-nnn/src/bridge.rs#L1876) |
| `condition` | `call.identify == 0x66660007 && call.code == 0x33330013 && self.skip.next_command && self.skip.effect_command` | Local state/value operations | Handled | [1890](../../crates/crysirror-system-nnn/src/bridge.rs#L1890) |
| `condition` | `call.identify==0x66660007&&call.code==0x33330013&&matches!(&call.payload,Payload::Words{count,words} if *count>=2&&words.get(1)==Some(&69))` | self.try_hamon_set | Handled | [1900](../../crates/crysirror-system-nnn/src/bridge.rs#L1900) |
| `condition` | `call.identify==0x66660007&&call.code==0x33330013&&matches!(&call.payload,Payload::Words{count,words} if *count>=2&&words.get(1)==Some(&81))` | self.try_hahen_set | Handled | [1901](../../crates/crysirror-system-nnn/src/bridge.rs#L1901) |
| `condition` | `call.identify == 0x66660007 && call.code == 0x33330054` | self.prepare_native_windows | Handled | [1908](../../crates/crysirror-system-nnn/src/bridge.rs#L1908) |
| `condition` | `matches!(call.payload, Payload::UserCall { command: false, .. }) && self .user_function_name(call)? .eq_ignore_ascii_case(b"savesystemfile")` | self.try_system_save | Handled | [1922](../../crates/crysirror-system-nnn/src/bridge.rs#L1922) |
| `condition` | `call.identify == 0x66660002 && matches!( call.payload, Payload::Control { code: 0x5556000c \| 0x5556000e \| 0x55560013, .. } )` | self.background_read_marks.is_some, self.try_draw | Handled | [1929](../../crates/crysirror-system-nnn/src/bridge.rs#L1929) |
| `condition` | `matches!( call.payload, Payload::Control { code: 0x5556000c, .. } )` | Local state/value operations | Handled | [1944](../../crates/crysirror-system-nnn/src/bridge.rs#L1944) |
| `condition` | `call.identify == 0x66660006 && call.code == 0x22220005` | self.try_overlap | Handled | [1965](../../crates/crysirror-system-nnn/src/bridge.rs#L1965) |
| `condition` | `call.identify == 0x66660006 && call.code == 0x22220004` | self.try_draw | Handled | [1968](../../crates/crysirror-system-nnn/src/bridge.rs#L1968) |
| `condition` | `matches!(call.code, 0x33330007 \| 0x33330008 \| 0x33330009)` | self.native_voice_suppressed, self.audio.as_mut, self.voice_speak.started | Handled | [1974](../../crates/crysirror-system-nnn/src/bridge.rs#L1974) |
| `condition` | `call.code == 0x33330007 && matches!(&call.payload,Payload::Words{words,..} if words.first().is_some_and(\|n\|*n>0)) && (self.must_set_music \|\| !(self.skip.next_command && self.skip.effect_command))` | Local state/value operations | Handled | [1981](../../crates/crysirror-system-nnn/src/bridge.rs#L1981) |
| `condition` | `call.code == 0x33330013 && *count >= 2 && words[1] == 29` | self.freeze_effect_backing, self.layer_id, SunaParameters::parse, host.begin_pattern, serde_json::to_vec | Handled | [2018](../../crates/crysirror-system-nnn/src/bridge.rs#L2018) |

### bridge.rs / Bridge::try_draw

```rust
fn try_draw(
        &mut self,
        machine: &mut Machine,
        call: &NativeCall,
        host: &mut dyn HostServices,
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `self.draw_wait.is_none() && frames > 0 && self.skip.next_command && self.background_read_marks.is_none()` | self.message_control, self.text.as_mut, self.adopt_message_control | Handled | [2750](../../crates/crysirror-system-nnn/src/bridge.rs#L2750) |

### bridge.rs / Bridge::try_image

```rust
fn try_image(
        &mut self,
        machine: &mut Machine,
        call: &NativeCall,
        host: &mut dyn HostServices,
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Err(error) = self.submit(host, json!({"layers":[op]}))` | host.release_image_resource | Unsupported: Unconditional rejection | [2153](../../crates/crysirror-system-nnn/src/bridge.rs#L2153) |
| `condition` | `let Err(error) = self.submit(host, json!({"layers":[operation]}))` | host.release_image_resource | Unsupported: Unconditional rejection | [2204](../../crates/crysirror-system-nnn/src/bridge.rs#L2204) |

### bridge/actor_frame.rs / Bridge::poll_message_frame

```rust
fn poll_message_frame(
        &mut self,
        machine: &mut Machine,
        call: Option<&NativeCall>,
        selection: bool,
        host: &mut dyn HostServices,
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Some(op)=Self::native_layer_removal(self.client,host,display+1)?` | Local state/value operations | Handled | [268](../../crates/crysirror-system-nnn/src/bridge/actor_frame.rs#L268) |
| `condition` | `let Err(error)=host.submit_engine(self.client,1,&serde_json::to_vec(&json!({"layers":[{"op":"put","layer":{"id":id,"parent":self.bindings.root,"rect":[0,0,w,h],"order":[i32::MAX-1,0],"paint":{"kind":"image","asset":published.path.relative()}}}]}))?)` | host.release_image_resource | Unsupported: Unconditional rejection | [403](../../crates/crysirror-system-nnn/src/bridge/actor_frame.rs#L403) |

### bridge/chart.rs / Bridge::chart_final_variables

```rust
fn chart_final_variables(
        &self,
        variables: &mut Variables,
        slot: i32,
        config: &NativeConfiguration,
        new_flags: u32,
    ) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Some(id) = config.variable_names.index( setup .parameter(b"newMarkVarName") .unwrap_or(b"newchartflag"), )` | Local state/value operations | Handled | [618](../../crates/crysirror-system-nnn/src/bridge/chart.rs#L618) |

### bridge/general_audio.rs / Bridge::poll_foreground_general_audio

```rust
fn poll_foreground_general_audio(
        &mut self,
        host: &mut dyn HostServices,
        call: Option<&NativeCall>,
        admit: bool,
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `call.map(\|c\| (&c.payload, c.identify, c.code))` | `Some((Payload::Message { .. }, _, _)) if !self.skip.next_command` | Local state/value operations | Handled | [72](../../crates/crysirror-system-nnn/src/bridge/general_audio.rs#L72) |
| `call.map(\|c\| (&c.payload, c.identify, c.code))` | `Some((Payload::Selection { .. }, _, _))` | Local state/value operations | Handled | [73](../../crates/crysirror-system-nnn/src/bridge/general_audio.rs#L73) |
| `call.map(\|c\| (&c.payload, c.identify, c.code))` | `Some((Payload::Words { words, .. }, 0x66660006, 0x22220004)) if words.first().is_some_and(\|n\| *n > 0) && !self.skip.next_command` | Local state/value operations | Handled | [74](../../crates/crysirror-system-nnn/src/bridge/general_audio.rs#L74) |
| `call.map(\|c\| (&c.payload, c.identify, c.code))` | `Some((Payload::Words { .. }, 0x66660006, 0x22220005)) if !(self.skip.next_command && self.skip.effect_command)` | Local state/value operations | Handled | [79](../../crates/crysirror-system-nnn/src/bridge/general_audio.rs#L79) |
| `call.map(\|c\| (&c.payload, c.identify, c.code))` | `None` | self.wait_kind | Handled | [84](../../crates/crysirror-system-nnn/src/bridge/general_audio.rs#L84) |
| `call.map(\|c\| (&c.payload, c.identify, c.code))` | `_` | Local state/value operations | Handled | [89](../../crates/crysirror-system-nnn/src/bridge/general_audio.rs#L89) |
| `condition` | `mode==9` | Local state/value operations | Handled | [96](../../crates/crysirror-system-nnn/src/bridge/general_audio.rs#L96) |
| `condition` | `mode == 9` | Local state/value operations | Handled | [104](../../crates/crysirror-system-nnn/src/bridge/general_audio.rs#L104) |

### bridge/saved_effects.rs / Bridge::load_saved_effect_state

```rust
fn load_saved_effect_state(
        &mut self,
        records: &[SavedEffect],
        layer_off: u32,
    ) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under words[2]=command @ (70\|71\|138)` | `command==138` | crate::native_hotaru_new::restored, crate::native_picture_effect::PictureEffectState::parse_for_canvas | Handled | [91](../../crates/crysirror-system-nnn/src/bridge/saved_effects.rs#L91) |
| `condition; under words[2]=command @ (70\|71\|138)` | `command==70` | Local state/value operations | Handled | [95](../../crates/crysirror-system-nnn/src/bridge/saved_effects.rs#L95) |

### bridge/saved_restore.rs / Bridge::initialize_saved_cpu_effect

```rust
fn initialize_saved_cpu_effect(&mut self,slot:i32,command:i32,random:&mut PatternRng) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `command` | `14\|57\|70\|71\|138` | self.picture_effects.get, self.particle_classes.as_mut, self.saved_particles.remove | Handled | [326](../../crates/crysirror-system-nnn/src/bridge/saved_restore.rs#L326) |
| `command` | `15` | self.picture_effects.get, self.sun_rays_class.initialize, self.saved_sun_rays.remove | Handled | [332](../../crates/crysirror-system-nnn/src/bridge/saved_restore.rs#L332) |
| `command` | `85` | self.picture_effects.get, Arc::new, self.saved_speed_line.remove | Handled | [338](../../crates/crysirror-system-nnn/src/bridge/saved_restore.rs#L338) |
| `command` | `42` | self.picture_effects.get, self.kira_class.clone, self.saved_kira.remove | Handled | [344](../../crates/crysirror-system-nnn/src/bridge/saved_restore.rs#L344) |
| `command` | `13` | self.picture_effects.get, Arc::new, crate::native_rain::RainClass::set, self.saved_rain.remove | Handled | [350](../../crates/crysirror-system-nnn/src/bridge/saved_restore.rs#L350) |
| `command` | `51` | self.picture_effects.get, Arc::new, self.saved_taiyo.remove | Handled | [356](../../crates/crysirror-system-nnn/src/bridge/saved_restore.rs#L356) |
| `command` | `93` | self.picture_effects.get, Arc::new, crate::native_grass_plane::GrassClass::set, self.saved_grass.remove | Handled | [362](../../crates/crysirror-system-nnn/src/bridge/saved_restore.rs#L362) |
| `command` | `108` | self.picture_effects.get_mut, crate::native_picture_hae::initialize, self.saved_hae.remove | Handled | [368](../../crates/crysirror-system-nnn/src/bridge/saved_restore.rs#L368) |
| `command` | `96` | self.initialize_saved_rainbow | Handled | [374](../../crates/crysirror-system-nnn/src/bridge/saved_restore.rs#L374) |
| `command` | `_` | Local state/value operations | Unsupported: NNN unknown queued native RNG class {command} | [375](../../crates/crysirror-system-nnn/src/bridge/saved_restore.rs#L375) |

### bridge/settings.rs / Bridge::commit_setting_file

```rust
fn commit_setting_file(
        &mut self,
        file: crate::native_system_file::NativeSystemFile,
        id: i32,
        value: i32,
        host: &mut dyn HostServices,
    ) -> Result<Option<[i32; 4]>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `id` | `4` | host.submit_engine, serde_json::to_vec | Handled | [137](../../crates/crysirror-system-nnn/src/bridge/settings.rs#L137) |
| `id` | `6` | self.text.as_mut | Handled | [142](../../crates/crysirror-system-nnn/src/bridge/settings.rs#L142) |
| `id` | `105` | self.text.as_mut | Handled | [148](../../crates/crysirror-system-nnn/src/bridge/settings.rs#L148) |
| `id` | `106` | self.text.as_mut, self.window_binding.as_mut | Handled | [153](../../crates/crysirror-system-nnn/src/bridge/settings.rs#L153) |
| `id` | `_` | Local state/value operations | Handled | [162](../../crates/crysirror-system-nnn/src/bridge/settings.rs#L162) |

### bridge/user_command.rs / default_user_commands

```rust
fn default_user_commands(
    configuration: &crate::native_bootstrap::NativeConfiguration,
) -> Result<BTreeMap<u32, i32>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `configuration.setup.integer(&key)?.unwrap_or(0) == 0` | Local state/value operations | Handled | [37](../../crates/crysirror-system-nnn/src/bridge/user_command.rs#L37) |

### config.rs / configured_variables

```rust
fn configured_variables(
    profile: VariableProfile,
    names: &Names,
    setup: &Names,
) -> Result<Variables>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Some(id) = names.index(b"date")` | Local state/value operations | Handled | [150](../../crates/crysirror-system-nnn/src/config.rs#L150) |
| `condition` | `let Some(id) = names.index(b"year")` | Local state/value operations | Handled | [155](../../crates/crysirror-system-nnn/src/config.rs#L155) |

### control.rs / case_target

```rust
fn case_target(program: &Program, mut pc: u32, selected: u16) -> Result<u32>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `case.identify != CONTROL \|\| case.code != 0x55560008` | Local state/value operations | Handled | [16](../../crates/crysirror-system-nnn/src/control.rs#L16) |

### control.rs / conditional

```rust
fn conditional(
    program: &Program,
    pc: u32,
    vars: &mut Variables,
    while_only: bool,
) -> Result<u32>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `branch.identify == CONTROL && branch.code == 0x55560004` | Local state/value operations | Handled | [78](../../crates/crysirror-system-nnn/src/control.rs#L78) |
| `condition` | `branch.identify != CONTROL \|\| branch.code != 0x55560003` | Local state/value operations | Handled | [81](../../crates/crysirror-system-nnn/src/control.rs#L81) |

### expression.rs / evaluate

```rust
fn evaluate(words: &[u32], p: &Program, vars: &mut Variables) -> Result<(i32, usize)>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `kind == 0xa0000000 && op == u16::from(b'!')` | i32::from, Cell::Value | Handled | [297](../../crates/crysirror-system-nnn/src/expression.rs#L297) |
| `(kind, op as u8)` | `(0xa0000000, b'=')` | Local state/value operations | Handled | [324](../../crates/crysirror-system-nnn/src/expression.rs#L324) |
| `(kind, op as u8)` | `(0xa2000000, op) if op != b'='` | i32::from, u16::from | Handled | [331](../../crates/crysirror-system-nnn/src/expression.rs#L331) |
| `(kind, op as u8)` | `(0xa0000000, _)` | Cell::Value | Handled | [346](../../crates/crysirror-system-nnn/src/expression.rs#L346) |
| `(kind, op as u8)` | `(0xa2000000, b'=') \| (0xa3000000, b'=') \| (0xa5000000, b'=')` | Cell::Value, i32::from | Handled | [351](../../crates/crysirror-system-nnn/src/expression.rs#L351) |
| `(kind, op as u8)` | `(0xa3000000, b'&')` | Cell::Value, i32::from | Handled | [354](../../crates/crysirror-system-nnn/src/expression.rs#L354) |
| `(kind, op as u8)` | `(0xa3000000, b'\|')` | Cell::Value, i32::from | Handled | [357](../../crates/crysirror-system-nnn/src/expression.rs#L357) |
| `(kind, op as u8)` | `(0xa3000000, b'^')` | Cell::Value, i32::from | Handled | [360](../../crates/crysirror-system-nnn/src/expression.rs#L360) |
| `(kind, op as u8)` | `(0xa4000000, b'<')` | Cell::Value, i32::from | Handled | [363](../../crates/crysirror-system-nnn/src/expression.rs#L363) |
| `(kind, op as u8)` | `(0xa4000000, b'>')` | Cell::Value, i32::from | Handled | [366](../../crates/crysirror-system-nnn/src/expression.rs#L366) |
| `(kind, op as u8)` | `(0xa5000000, b'<')` | Cell::Value, i32::from | Handled | [369](../../crates/crysirror-system-nnn/src/expression.rs#L369) |
| `(kind, op as u8)` | `(0xa5000000, b'>')` | Cell::Value, i32::from | Handled | [372](../../crates/crysirror-system-nnn/src/expression.rs#L372) |
| `(kind, op as u8)` | `(0xa5000000, b'!')` | Cell::Value, i32::from | Handled | [375](../../crates/crysirror-system-nnn/src/expression.rs#L375) |
| `(kind, op as u8)` | `_` | Local state/value operations | Unsupported: unsupported System-NNN postfix operator {token:#x} | [378](../../crates/crysirror-system-nnn/src/expression.rs#L378) |

### expression.rs / Variables::read_calculated

```rust
fn read_calculated(&self, id: u16) -> Result<i32>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(name.as_slice(), b"week" \| b"holiday")` | Local state/value operations | Handled | [166](../../crates/crysirror-system-nnn/src/expression.rs#L166) |
| `condition` | `name == b"rnd"` | self.random.borrow_mut | Handled | [173](../../crates/crysirror-system-nnn/src/expression.rs#L173) |

### expression.rs / Variables::read_stored

```rust
fn read_stored(&self, id: u16) -> i32
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `id & 0x8000 != 0` | i32::from | Handled | [157](../../crates/crysirror-system-nnn/src/expression.rs#L157) |

### installed.rs / InstalledAdapter::poll_script_movie

```rust
fn poll_script_movie(&mut self, host: &mut dyn HostServices, input: InputFrame) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `mode == 22` | self.open_menu | Handled | [1167](../../crates/crysirror-system-nnn/src/installed.rs#L1167) |

### installed/actor_entry.rs / ActorEntry::poll

```rust
fn poll(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        vm: &mut NativeAdapter,
        input: InputFrame,
        admit: bool,
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `self.display_path.is_none() && self.spec.kind != EntryKind::NoPrint && self.spec.frames > 0` | self.old.clone, self.show | Handled | [223](../../crates/crysirror-system-nnn/src/installed/actor_entry.rs#L223) |

### installed/actor_entry.rs / ActorEntry::show

```rust
fn show(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        image: Arc<dyn ImageResource>,
    ) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Err(error)=host.submit_engine(client,1,&serde_json::to_vec(&json!({"layers":[{"op":"put","layer":{"id":self.display,"rect":[0,0,800,600],"order":[i32::MAX-1,0],"paint":{"kind":"image","asset":path}}}]}))?)` | host.release_image_resource | Unsupported: Unconditional rejection | [107](../../crates/crysirror-system-nnn/src/installed/actor_entry.rs#L107) |

### installed/chart.rs / ChartPlayer::draw

```rust
fn draw(
        &mut self,
        runtime: &ChartRuntime,
        state: &ChartState,
        variables: &mut Variables,
        host: &mut dyn HostServices,
        font: &str,
    ) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `key == "unknown"` | Local state/value operations | Handled | [1131](../../crates/crysirror-system-nnn/src/installed/chart.rs#L1131) |
| `condition` | `command["widget"]["kind"] == "label"` | Local state/value operations | Handled | [1423](../../crates/crysirror-system-nnn/src/installed/chart.rs#L1423) |

### installed/chart.rs / ChartPlayer::load

```rust
fn load(&mut self, runtime: &ChartRuntime, host: &mut dyn HostServices) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `key.starts_with("slot")` | u64::from | Handled | [293](../../crates/crysirror-system-nnn/src/installed/chart.rs#L293) |

### installed/chart.rs / ChartPlayer::poll

```rust
fn poll(
        &mut self,
        vm: &mut NativeAdapter,
        host: &mut dyn HostServices,
        input: InputFrame,
        font: &str,
    ) -> Result<Option<Event>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `event["kind"] != "activate"` | Local state/value operations | Handled | [689](../../crates/crysirror-system-nnn/src/installed/chart.rs#L689) |
| `condition` | `id == ROOT + 1` | Local state/value operations | Handled | [692](../../crates/crysirror-system-nnn/src/installed/chart.rs#L692) |
| `condition` | `id == ROOT + 2 && self.confirmed.is_some()` | self.confirmed.take | Handled | [695](../../crates/crysirror-system-nnn/src/installed/chart.rs#L695) |
| `condition` | `id == ROOT + 3 && self.confirmed.is_some()` | Local state/value operations | Handled | [700](../../crates/crysirror-system-nnn/src/installed/chart.rs#L700) |
| `condition` | `id >= ROOT + 100 && id < ROOT + 100 + runtime.definition.maps.len() as u32` | Local state/value operations | Handled | [708](../../crates/crysirror-system-nnn/src/installed/chart.rs#L708) |

### installed/chart.rs / InstalledAdapter::poll_chart

```rust
fn poll_chart(
        &mut self,
        host: &mut dyn HostServices,
        input: InputFrame,
        admit: bool,
    ) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!vm.start_menu_background( script.name, script.program, script.looping, 34, host, )?` | Local state/value operations | Handled | [1578](../../crates/crysirror-system-nnn/src/installed/chart.rs#L1578) |

### installed/gallery.rs / prepare

```rust
fn prepare(
    kind: Kind,
    page: usize,
    table: &Names,
    config: &crate::native_bootstrap::NativeConfiguration,
    storage: &dyn ClientStorage,
    progress: &NativeSystemFile,
    variables: &crate::expression::Variables,
    cancel: &Cancellation,
) -> Result<Page>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under kind=Kind::Music` | `let Some(id) = config.variable_names.index(&record[6])` | Local state/value operations | Handled | [285](../../crates/crysirror-system-nnn/src/installed/gallery.rs#L285) |

### installed/gallery_viewer.rs / PictureViewer::load

```rust
fn load(&mut self) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!name[..2].eq_ignore_ascii_case(b"ta")` | Local state/value operations | Handled | [171](../../crates/crysirror-system-nnn/src/installed/gallery_viewer.rs#L171) |

### installed/gallery_viewer/difference.rs / PictureViewer::poll_difference

```rust
fn poll_difference(
        &mut self,
        host: &mut dyn HostServices,
        mut vm: Option<&mut NativeAdapter>,
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Err(error) = host.submit_engine(self.client, 1, &serde_json::to_vec(&commands)?)` | host.release_image_resource | Unsupported: Unconditional rejection | [53](../../crates/crysirror-system-nnn/src/installed/gallery_viewer/difference.rs#L53) |

### installed/menu.rs / Kind::from_native

```rust
fn from_native(mode: i32) -> Result<Self>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `mode` | `5` | Local state/value operations | Handled | [70](../../crates/crysirror-system-nnn/src/installed/menu.rs#L70) |
| `mode` | `6` | Local state/value operations | Handled | [71](../../crates/crysirror-system-nnn/src/installed/menu.rs#L71) |
| `mode` | `8` | Local state/value operations | Handled | [72](../../crates/crysirror-system-nnn/src/installed/menu.rs#L72) |
| `mode` | `12` | Local state/value operations | Handled | [73](../../crates/crysirror-system-nnn/src/installed/menu.rs#L73) |
| `mode` | `15` | Local state/value operations | Handled | [74](../../crates/crysirror-system-nnn/src/installed/menu.rs#L74) |
| `mode` | `16` | Local state/value operations | Handled | [75](../../crates/crysirror-system-nnn/src/installed/menu.rs#L75) |
| `mode` | `17` | Self::Cg | Handled | [76](../../crates/crysirror-system-nnn/src/installed/menu.rs#L76) |
| `mode` | `18` | Self::Scenes | Handled | [77](../../crates/crysirror-system-nnn/src/installed/menu.rs#L77) |
| `mode` | `20` | Local state/value operations | Handled | [78](../../crates/crysirror-system-nnn/src/installed/menu.rs#L78) |
| `mode` | `26` | Local state/value operations | Handled | [79](../../crates/crysirror-system-nnn/src/installed/menu.rs#L79) |
| `mode` | `13` | Local state/value operations | Handled | [80](../../crates/crysirror-system-nnn/src/installed/menu.rs#L80) |
| `mode` | `_` | Local state/value operations | Handled | [81](../../crates/crysirror-system-nnn/src/installed/menu.rs#L81) |

### installed/menu.rs / Menu::new

```rust
fn new(
        kind: Kind,
        config: Arc<crate::native_bootstrap::NativeConfiguration>,
        settings: Vec<(i32, i32)>,
        client: ClientId,
        host: &mut dyn HostServices,
    ) -> Result<Self>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(kind,Kind::Save\|Kind::System)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [253](../../crates/crysirror-system-nnn/src/installed/menu.rs#L253) |

### installed/menu.rs / Menu::poll

```rust
fn poll(
        &mut self,
        client: ClientId,
        font: &str,
        pool: &TaskPool,
        host: &mut dyn HostServices,
        input: InputFrame,
        admit: bool,
    ) -> Result<Option<Action>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `event["kind"]=="nnn_menu_confirm_cancel" && event["widget"]["id"]==ROOT+8 && self.confirm.is_some()` | Local state/value operations | Handled | [1213](../../crates/crysirror-system-nnn/src/installed/menu.rs#L1213) |
| `condition` | `!child_cancelled && event["kind"]=="nnn_menu_cancel" && event["widget"]["id"]==ROOT` | Local state/value operations | Handled | [1216](../../crates/crysirror-system-nnn/src/installed/menu.rs#L1216) |
| `condition; under receiver=self.kind == Kind::Config` | `matches!(event["kind"].as_str(), Some("change" \| "activate")) && id >= ROOT + 101` | self.settings.get, crate::native_settings::menu_setting, serde_json::from_slice, host.query_engine, EngineQuery::UiWidget | Handled | [1227](../../crates/crysirror-system-nnn/src/installed/menu.rs#L1227) |
| `condition; under receiver=self.kind == Kind::Config` | `(id - ROOT - 101) % 2 == 0` | self.settings.get, crate::native_settings::menu_setting, serde_json::from_slice, host.query_engine, EngineQuery::UiWidget | Handled | [1230](../../crates/crysirror-system-nnn/src/installed/menu.rs#L1230) |
| `condition` | `event["kind"] != "activate"` | Local state/value operations | Handled | [1252](../../crates/crysirror-system-nnn/src/installed/menu.rs#L1252) |
| `id - ROOT` | `1` | Local state/value operations | Handled | [1256](../../crates/crysirror-system-nnn/src/installed/menu.rs#L1256) |
| `id - ROOT` | `6 if self.confirm.is_some()` | self.confirm.take | Handled | [1257](../../crates/crysirror-system-nnn/src/installed/menu.rs#L1257) |
| `id - ROOT` | `7 if self.confirm.is_some()` | self.skin.is_some, host.submit_engine, serde_json::to_vec, self.build, self.skin.as_mut | Handled | [1258](../../crates/crysirror-system-nnn/src/installed/menu.rs#L1258) |
| `id - ROOT` | `3 if self.kind==Kind::Config&&self.settings_page>0` | Local state/value operations | Handled | [1269](../../crates/crysirror-system-nnn/src/installed/menu.rs#L1269) |
| `id - ROOT` | `4 if self.kind==Kind::Config&&(self.settings_page+1)*14<self.settings.len()` | Local state/value operations | Handled | [1270](../../crates/crysirror-system-nnn/src/installed/menu.rs#L1270) |
| `id - ROOT` | `3 if self.kind != Kind::Config && self.page > 0` | self.refresh | Handled | [1271](../../crates/crysirror-system-nnn/src/installed/menu.rs#L1271) |
| `id - ROOT` | `4 if self.kind != Kind::Config && self.ready.as_ref().is_some_and(\|p\| self.page + 1 < p.pages)` | self.refresh | Handled | [1276](../../crates/crysirror-system-nnn/src/installed/menu.rs#L1276) |
| `id - ROOT` | `i if self.kind != Kind::Config && self.confirm.is_none() && ((100..164).contains(&i) \|\| (200..264).contains(&i))` | self.kind.gallery, Action::Load, i32::try_from, Action::Save, u32::try_from | Handled | [1283](../../crates/crysirror-system-nnn/src/installed/menu.rs#L1283) |
| `id - ROOT` | `_` | Local state/value operations | Handled | [1310](../../crates/crysirror-system-nnn/src/installed/menu.rs#L1310) |

### installed/menu.rs / Menu::prepare_page

```rust
fn prepare_page(
        &mut self,
        client: ClientId,
        pool: &TaskPool,
        host: &mut dyn HostServices,
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(kind,Kind::Omake\|Kind::System)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [596](../../crates/crysirror-system-nnn/src/installed/menu.rs#L596) |
| `condition` | `matches!(kind,Kind::Omake\|Kind::System)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [656](../../crates/crysirror-system-nnn/src/installed/menu.rs#L656) |
| `condition; under receiver=kind == Kind::System` | `mode > 0 && !super::system_menu::mode_enabled(&config,variables.as_ref().context("NNN system mode variable bank absent")?,mode)?` | Local state/value operations | Handled | [688](../../crates/crysirror-system-nnn/src/installed/menu.rs#L688) |
| `condition` | `kind==Kind::System && !matches!(return_mode,9\|10)` | Local state/value operations | Handled | [704](../../crates/crysirror-system-nnn/src/installed/menu.rs#L704) |

### installed/menu.rs / Menu::update_setting

```rust
fn update_setting(&mut self, id: i32, value: i32) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Some(pair) = self.settings.iter_mut().find(\|v\| v.0 == id)` | crate::native_character_setting::Setting::decode | Handled | [1316](../../crates/crysirror-system-nnn/src/installed/menu.rs#L1316) |

### installed/menu/general_entry.rs / PlainEntry::present

```rust
fn present(
        &mut self,
        client: ClientId,
        host: &mut dyn HostServices,
        image: Arc<dyn ImageResource>,
    ) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Err(error)=host.submit_engine(client,1,&serde_json::to_vec(&json!({"layers":[{"op":"put","layer":{ "id":DISPLAY,"rect":[0,0,SIZE[0],SIZE[1]],"order":[i32::MAX-1,0],"paint":{"kind":"image","asset":path} }}]}))?)` | host.release_image_resource | Unsupported: Unconditional rejection | [76](../../crates/crysirror-system-nnn/src/installed/menu/general_entry.rs#L76) |

### installed/menu_host.rs / InstalledAdapter::activate_menu_action

```rust
fn activate_menu_action(&mut self, action: Action, host: &mut dyn HostServices) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under action=Action::System(index) / index=9..` | `mode > 0` | Action::Navigate, Kind::from_native | Handled | [521](../../crates/crysirror-system-nnn/src/installed/menu_host.rs#L521) |
| `condition; under action=Action::System(index) / index=9..` | `command <= 0` | Local state/value operations | Handled | [524](../../crates/crysirror-system-nnn/src/installed/menu_host.rs#L524) |
| `command; under action=Action::SystemCommand(command)` | `6` | self.native_save.as_ref, self.menu.as_ref, self.vm.as_ref, self.native_save.take, native_save::Controller::selected | Handled | [547](../../crates/crysirror-system-nnn/src/installed/menu_host.rs#L547) |
| `command; under action=Action::SystemCommand(command)` | `5` | self.native_load.as_ref, native_load::Controller::admit, host.now_ms, self.native_load.take | Handled | [557](../../crates/crysirror-system-nnn/src/installed/menu_host.rs#L557) |
| `command; under action=Action::SystemCommand(command)` | `_` | Local state/value operations | Handled | [566](../../crates/crysirror-system-nnn/src/installed/menu_host.rs#L566) |
| `id; under action=Action::Setting(id, value)` | `103` | Local state/value operations | Handled | [779](../../crates/crysirror-system-nnn/src/installed/menu_host.rs#L779) |
| `id; under action=Action::Setting(id, value)` | `203` | Local state/value operations | Handled | [780](../../crates/crysirror-system-nnn/src/installed/menu_host.rs#L780) |
| `id; under action=Action::Setting(id, value)` | `_` | Local state/value operations | Handled | [781](../../crates/crysirror-system-nnn/src/installed/menu_host.rs#L781) |

### installed/menu_host.rs / InstalledAdapter::apply_config_effect

```rust
fn apply_config_effect(
        &mut self,
        effect: super::menu::native_config::Effect,
        host: &mut dyn HostServices,
    ) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `id; under effect=Effect::Parameter(id, value)` | `103` | Local state/value operations | Handled | [807](../../crates/crysirror-system-nnn/src/installed/menu_host.rs#L807) |
| `id; under effect=Effect::Parameter(id, value)` | `203` | Local state/value operations | Handled | [808](../../crates/crysirror-system-nnn/src/installed/menu_host.rs#L808) |
| `id; under effect=Effect::Parameter(id, value)` | `_` | Local state/value operations | Handled | [809](../../crates/crysirror-system-nnn/src/installed/menu_host.rs#L809) |

### installed/native_load.rs / key_mask

```rust
fn key_mask(code: i32) -> Result<u64>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `code` | `0` | Local state/value operations | Handled | [44](../../crates/crysirror-system-nnn/src/installed/native_load.rs#L44) |
| `code` | `8` | Local state/value operations | Handled | [45](../../crates/crysirror-system-nnn/src/installed/native_load.rs#L45) |
| `code` | `9` | Local state/value operations | Handled | [46](../../crates/crysirror-system-nnn/src/installed/native_load.rs#L46) |
| `code` | `13` | Local state/value operations | Handled | [47](../../crates/crysirror-system-nnn/src/installed/native_load.rs#L47) |
| `code` | `16` | Local state/value operations | Handled | [48](../../crates/crysirror-system-nnn/src/installed/native_load.rs#L48) |
| `code` | `17` | Local state/value operations | Handled | [49](../../crates/crysirror-system-nnn/src/installed/native_load.rs#L49) |
| `code` | `26` | Local state/value operations | Handled | [50](../../crates/crysirror-system-nnn/src/installed/native_load.rs#L50) |
| `code` | `27` | Local state/value operations | Handled | [51](../../crates/crysirror-system-nnn/src/installed/native_load.rs#L51) |
| `code` | `28` | Local state/value operations | Handled | [52](../../crates/crysirror-system-nnn/src/installed/native_load.rs#L52) |
| `code` | `29` | Local state/value operations | Handled | [53](../../crates/crysirror-system-nnn/src/installed/native_load.rs#L53) |
| `code` | `30` | Local state/value operations | Handled | [54](../../crates/crysirror-system-nnn/src/installed/native_load.rs#L54) |
| `code` | `31` | Local state/value operations | Handled | [55](../../crates/crysirror-system-nnn/src/installed/native_load.rs#L55) |
| `code` | `32` | Local state/value operations | Handled | [56](../../crates/crysirror-system-nnn/src/installed/native_load.rs#L56) |
| `code` | `33` | Local state/value operations | Handled | [57](../../crates/crysirror-system-nnn/src/installed/native_load.rs#L57) |
| `code` | `34` | Local state/value operations | Handled | [58](../../crates/crysirror-system-nnn/src/installed/native_load.rs#L58) |
| `code` | `35` | Local state/value operations | Handled | [59](../../crates/crysirror-system-nnn/src/installed/native_load.rs#L59) |
| `code` | `36` | Local state/value operations | Handled | [60](../../crates/crysirror-system-nnn/src/installed/native_load.rs#L60) |
| `code` | `37` | Local state/value operations | Handled | [61](../../crates/crysirror-system-nnn/src/installed/native_load.rs#L61) |
| `code` | `38` | Local state/value operations | Handled | [62](../../crates/crysirror-system-nnn/src/installed/native_load.rs#L62) |
| `code` | `39` | Local state/value operations | Handled | [63](../../crates/crysirror-system-nnn/src/installed/native_load.rs#L63) |
| `code` | `40` | Local state/value operations | Handled | [64](../../crates/crysirror-system-nnn/src/installed/native_load.rs#L64) |
| `code` | `45` | Local state/value operations | Handled | [65](../../crates/crysirror-system-nnn/src/installed/native_load.rs#L65) |
| `code` | `48..=57` | Local state/value operations | Handled | [66](../../crates/crysirror-system-nnn/src/installed/native_load.rs#L66) |
| `code` | `65` | Local state/value operations | Handled | [67](../../crates/crysirror-system-nnn/src/installed/native_load.rs#L67) |
| `code` | `67` | Local state/value operations | Handled | [68](../../crates/crysirror-system-nnn/src/installed/native_load.rs#L68) |
| `code` | `68` | Local state/value operations | Handled | [69](../../crates/crysirror-system-nnn/src/installed/native_load.rs#L69) |
| `code` | `69` | Local state/value operations | Handled | [70](../../crates/crysirror-system-nnn/src/installed/native_load.rs#L70) |
| `code` | `70` | Local state/value operations | Handled | [71](../../crates/crysirror-system-nnn/src/installed/native_load.rs#L71) |
| `code` | `76` | Local state/value operations | Handled | [72](../../crates/crysirror-system-nnn/src/installed/native_load.rs#L72) |
| `code` | `81` | Local state/value operations | Handled | [73](../../crates/crysirror-system-nnn/src/installed/native_load.rs#L73) |
| `code` | `82` | Local state/value operations | Handled | [74](../../crates/crysirror-system-nnn/src/installed/native_load.rs#L74) |
| `code` | `83` | Local state/value operations | Handled | [75](../../crates/crysirror-system-nnn/src/installed/native_load.rs#L75) |
| `code` | `87` | Local state/value operations | Handled | [76](../../crates/crysirror-system-nnn/src/installed/native_load.rs#L76) |
| `code` | `90` | Local state/value operations | Handled | [77](../../crates/crysirror-system-nnn/src/installed/native_load.rs#L77) |
| `code` | `112..=123` | Local state/value operations | Handled | [78](../../crates/crysirror-system-nnn/src/installed/native_load.rs#L78) |
| `code` | `_` | Local state/value operations | Handled | [79](../../crates/crysirror-system-nnn/src/installed/native_load.rs#L79) |

### installed/native_load.rs / InstalledAdapter::poll_slot_load

```rust
fn poll_slot_load(
        &mut self,
        host: &mut dyn HostServices,
        input: InputFrame,
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `input.buttons_pressed & load.policy.key == 0` | Local state/value operations | Handled | [247](../../crates/crysirror-system-nnn/src/installed/native_load.rs#L247) |

### installed/native_load.rs / Policy::prepare

```rust
fn prepare(
        config: &NativeConfiguration,
        storage: &dyn ClientStorage,
        cancel: &Cancellation,
    ) -> Result<Option<Self>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Some(name) = setup.parameter(b"quickLoadKey")` | crate::config::integer, i32::from | Handled | [118](../../crates/crysirror-system-nnn/src/installed/native_load.rs#L118) |
| `condition` | `number == 0 && name.first().is_some_and(\|byte\| *byte != b'0')` | i32::from | Handled | [120](../../crates/crysirror-system-nnn/src/installed/native_load.rs#L120) |

### installed/native_save.rs / InstalledAdapter::poll_slot_save

```rust
fn poll_slot_save(
        &mut self,
        host: &mut dyn HostServices,
        input: InputFrame,
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `input.buttons_pressed & save.policy.key == 0` | Local state/value operations | Handled | [138](../../crates/crysirror-system-nnn/src/installed/native_save.rs#L138) |

### installed/native_save.rs / Policy::prepare

```rust
fn prepare(
        config: &crate::native_bootstrap::NativeConfiguration,
        storage: &dyn ClientStorage,
        cancel: &Cancellation,
    ) -> Result<Option<Self>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Some(name) = setup.parameter(b"quickSaveKey")` | crate::config::integer, i32::from, native_load::key_mask | Handled | [39](../../crates/crysirror-system-nnn/src/installed/native_save.rs#L39) |
| `condition` | `number == 0 && name.first().is_some_and(\|b\| *b != b'0')` | i32::from | Handled | [41](../../crates/crysirror-system-nnn/src/installed/native_save.rs#L41) |

### installed/system_menu.rs / backdrop

```rust
fn backdrop(
    table: &crate::config::Names,
    config: &crate::native_bootstrap::NativeConfiguration,
    resources: &NativeResources,
    captured: Arc<dyn ImageResource>,
    cancel: &Cancellation,
) -> Result<ImageInput>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Some(source) = resources.optional_image_source( &config .resources .picture(&[b"sys/".as_slice(), name].concat(), levels)?, cancel, )?` | Box::new | Handled | [166](../../crates/crysirror-system-nnn/src/installed/system_menu.rs#L166) |

### installed/system_menu.rs / mode_enabled

```rust
fn mode_enabled(
    config: &crate::native_bootstrap::NativeConfiguration,
    variables: &crate::expression::Variables,
    mode: i32,
) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `config .setup .integer(format!("expModeCheckByVarMode{index}").as_bytes())? .unwrap_or(0) == mode` | Local state/value operations | Handled | [27](../../crates/crysirror-system-nnn/src/installed/system_menu.rs#L27) |
| `condition` | `let Some(name) = config .setup .parameter(format!("expModeCheckByVarName{index}").as_bytes())` | Local state/value operations | Handled | [33](../../crates/crysirror-system-nnn/src/installed/system_menu.rs#L33) |

### machine.rs / Machine::poll_profile

```rust
fn poll_profile(&mut self, program: &Program, budget: u32, frame_yields: bool) -> Result<Step>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `i.identify == 0x66660002 && !matches!( i.code, 0x55560006 \| 0x5556000a..=0x5556000c \| 0x55560015 \| 0x55560016 ) && !(frame_yields && matches!(i.code, 0x5556000e \| 0x55560013))` | crate::control::conditional, self.queue_native, crate::control::skip_body, crate::control::target, self.returns.pop | Partial: unsupported System-NNN control pc={:#x} code={:#x}; PC retained | [266](../../crates/crysirror-system-nnn/src/machine.rs#L266) |
| `i.code` | `crate::control::OPEN \| crate::control::CLOSE` | Local state/value operations | Handled | [277](../../crates/crysirror-system-nnn/src/machine.rs#L277) |
| `i.code` | `_` | Local state/value operations | Unsupported: unsupported System-NNN control pc={:#x} code={:#x}; PC retained | [359](../../crates/crysirror-system-nnn/src/machine.rs#L359) |
| `condition; under (i.identify, i.code)=( 0x66660002, code @ (0x55560006 \| 0x5556000a..=0x5556000c \| 0x5556000e \| 0x55560013 \| 0x55560015 \| 0x55560016), )` | `matches!(code, 0x5556000a \| 0x55560015 \| 0x55560016)` | Local state/value operations | Handled | [380](../../crates/crysirror-system-nnn/src/machine.rs#L380) |
| `condition; under (i.identify, i.code)=( 0x66660002, code @ (0x55560006 \| 0x5556000a..=0x5556000c \| 0x5556000e \| 0x55560013 \| 0x55560015 \| 0x55560016), )` | `matches!(code, 0x5556000a \| 0x55560016)` | Local state/value operations | Handled | [385](../../crates/crysirror-system-nnn/src/machine.rs#L385) |
| `condition; under (i.identify, i.code)=(id @ (0x66660003 \| 0x66660004), _)` | `id == 0x66660003` | Local state/value operations | Handled | [412](../../crates/crysirror-system-nnn/src/machine.rs#L412) |
| `(i.identify, i.code)` | `( 0x66660002, code @ (0x55560006 \| 0x5556000a..=0x5556000c \| 0x5556000e \| 0x55560013 \| 0x55560015 \| 0x55560016), )` | Local state/value operations | Handled | [370](../../crates/crysirror-system-nnn/src/machine.rs#L370) |
| `(i.identify, i.code)` | `(id @ (0x66660003 \| 0x66660004), _)` | Vec::with_capacity | Handled | [391](../../crates/crysirror-system-nnn/src/machine.rs#L391) |
| `(i.identify, i.code)` | `(0x66660006, 0x22220004)` | Local state/value operations | Handled | [426](../../crates/crysirror-system-nnn/src/machine.rs#L426) |
| `(i.identify, i.code)` | `(0x66660006, 0x22220005)` | Local state/value operations | Handled | [430](../../crates/crysirror-system-nnn/src/machine.rs#L430) |
| `(i.identify, i.code)` | `(0x66660006, 0x22220006)` | Local state/value operations | Handled | [441](../../crates/crysirror-system-nnn/src/machine.rs#L441) |
| `(i.identify, i.code)` | `(0x66660007, 0x33330008)` | Local state/value operations | Handled | [525](../../crates/crysirror-system-nnn/src/machine.rs#L525) |
| `(i.identify, i.code)` | `(0x66660007, 0x33330042)` | Payload::DebugMessage | Handled | [547](../../crates/crysirror-system-nnn/src/machine.rs#L547) |
| `(i.identify, i.code)` | `(0x66660007, 0x33330014 \| 0x33330019 \| 0x3333001a \| 0x33330028)` | Local state/value operations | Handled | [550](../../crates/crysirror-system-nnn/src/machine.rs#L550) |
| `(i.identify, i.code)` | `(0x66660007, 0x3333001e)` | Payload::Scalar | Handled | [554](../../crates/crysirror-system-nnn/src/machine.rs#L554) |
| `(i.identify, i.code)` | `(0x66660007, 0x33330050)` | Payload::FilmName | Handled | [555](../../crates/crysirror-system-nnn/src/machine.rs#L555) |
| `(i.identify, i.code)` | `(0x66660007, 0x33330051)` | Payload::StoryName | Handled | [558](../../crates/crysirror-system-nnn/src/machine.rs#L558) |
| `(i.identify, i.code)` | `(0x66660007, code @ (0x33330015 \| 0x3333001f))` | Local state/value operations | Handled | [561](../../crates/crysirror-system-nnn/src/machine.rs#L561) |
| `(i.identify, i.code)` | `(0x66660006, kind @ 0x22220001..=0x22220003)` | Local state/value operations | Handled | [566](../../crates/crysirror-system-nnn/src/machine.rs#L566) |
| `(i.identify, i.code)` | `(0x66660007, 0x33330012 \| 0x33330011 \| 0x33330017 \| 0x33330029)` | Local state/value operations | Handled | [572](../../crates/crysirror-system-nnn/src/machine.rs#L572) |
| `(i.identify, i.code)` | `(0x66660007, 0x33330052..=0x33330060)` | Payload::Scalar | Handled | [573](../../crates/crysirror-system-nnn/src/machine.rs#L573) |
| `(i.identify, i.code)` | `( 0x66660007, 0x33330007 \| 0x33330009 \| 0x33330013 \| 0x33330016 \| 0x33330018 \| 0x3333001b \| 0x3333001c \| 0x33330061..=0x3333006a \| 0x3333006c..=0x3333006f, )` | Local state/value operations | Handled | [574](../../crates/crysirror-system-nnn/src/machine.rs#L574) |
| `(i.identify, i.code)` | `_` | Local state/value operations | Unsupported: unsupported System-NNN instruction pc={:#x} identify={:#x} code={:#x}; PC retained | [599](../../crates/crysirror-system-nnn/src/machine.rs#L599) |
| `condition` | `i.identify == 0x66660002 && i.code == 0x55560006` | Local state/value operations | Handled | [606](../../crates/crysirror-system-nnn/src/machine.rs#L606) |

### native_anime_parts.rs / AnimePartSpec::print

```rust
fn print(
        &self,
        count: u64,
        point: Option<[i32; 2]>,
        delta: bool,
        appearance: Option<PartAppearance>,
    ) -> Result<PartPrint>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `self.kind >= 5` | u64::from | Handled | [116](../../crates/crysirror-system-nnn/src/native_anime_parts.rs#L116) |
| `condition` | `a.kind & 1 != 0` | Local state/value operations | Handled | [129](../../crates/crysirror-system-nnn/src/native_anime_parts.rs#L129) |

### native_anime_parts.rs / AnimePartSpec::read

```rust
fn read(table: &Names, tag: &[u8]) -> Result<Self>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(kind, 3 \| 4 \| 7 \| 8)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [69](../../crates/crysirror-system-nnn/src/native_anime_parts.rs#L69) |
| `condition` | `matches!(kind, 2 \| 4 \| 6 \| 8)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [70](../../crates/crysirror-system-nnn/src/native_anime_parts.rs#L70) |

### native_audio.rs / NativeAudioPolicy::plan

```rust
fn plan(
        &self,
        call: &NativeCall,
        music: &Names,
        sound: &Names,
        music_playing: bool,
    ) -> Result<NativeAudioPlan>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `(&call.payload, call.code)` | `_` | Local state/value operations | Unsupported: NNN audio callback requires exact Music/Sound/Voice payload | [332](../../crates/crysirror-system-nnn/src/native_audio.rs#L332) |

### native_audio/skip.rs / NativeAudioPolicy::admit_with_skip

```rust
fn admit_with_skip(
        &self,
        call: &NativeCall,
        music: &Names,
        sound: &Names,
        playing: bool,
        skip: &NativeSkipPolicy,
        context: AudioCallbackContext,
    ) -> Result<Admission>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under (&call.payload, call.code)=(Payload::Words { count, words }, 0x33330009)` | `words[0] != -1 && (skip.next_command \|\| context.message_skipping) && !context.auto_message` | Local state/value operations | Handled | [67](../../crates/crysirror-system-nnn/src/native_audio/skip.rs#L67) |
| `condition; under (&call.payload, call.code)=(Payload::Voice { parameters, .. }, 0x33330008)` | `(skip.next_command \|\| context.message_skipping) && !context.auto_message && seed.voice_work[channel][0] != 0` | Local state/value operations | Handled | [98](../../crates/crysirror-system-nnn/src/native_audio/skip.rs#L98) |
| `(&call.payload, call.code)` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [120](../../crates/crysirror-system-nnn/src/native_audio/skip.rs#L120) |

### native_calendar.rs / NativeCalendar::value

```rust
fn value(&self, name: &[u8], variables: &Variables) -> Result<i32>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `name == b"week"` | i32::from | Handled | [66](../../crates/crysirror-system-nnn/src/native_calendar.rs#L66) |

### native_capture_effect.rs / parameters

```rust
fn parameters(command: i32, p: &[i32]) -> Result<[i32; 16]>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `command` | `59` | Local state/value operations | Handled | [49](../../crates/crysirror-system-nnn/src/native_capture_effect.rs#L49) |
| `command` | `66` | Local state/value operations | Handled | [50](../../crates/crysirror-system-nnn/src/native_capture_effect.rs#L50) |
| `command` | `_` | Local state/value operations | Handled | [51](../../crates/crysirror-system-nnn/src/native_capture_effect.rs#L51) |
| `command` | `59` | Local state/value operations | Handled | [58](../../crates/crysirror-system-nnn/src/native_capture_effect.rs#L58) |
| `command` | `2` | Local state/value operations | Handled | [70](../../crates/crysirror-system-nnn/src/native_capture_effect.rs#L70) |
| `command` | `75` | Local state/value operations | Handled | [78](../../crates/crysirror-system-nnn/src/native_capture_effect.rs#L78) |
| `command` | `66 \| 73` | Local state/value operations | Empty handler: Original no-op status not established | [83](../../crates/crysirror-system-nnn/src/native_capture_effect.rs#L83) |
| `command` | `_` | Local state/value operations | Handled | [84](../../crates/crysirror-system-nnn/src/native_capture_effect.rs#L84) |

### native_cg.rs / CgEntry::parse

```rust
fn parse(raw: &[u8], code_byte: i32) -> Result<Self>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `sub` | `0` | Local state/value operations | Handled | [155](../../crates/crysirror-system-nnn/src/native_cg.rs#L155) |
| `sub` | `1` | Local state/value operations | Handled | [159](../../crates/crysirror-system-nnn/src/native_cg.rs#L159) |
| `sub` | `_` | Local state/value operations | Handled | [160](../../crates/crysirror-system-nnn/src/native_cg.rs#L160) |

### native_cg_difference.rs / DifferenceProfile::read

```rust
fn read(table: &Names) -> Result<Self>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `selector > 0` | Local state/value operations | Handled | [18](../../crates/crysirror-system-nnn/src/native_cg_difference.rs#L18) |

### native_cg_difference.rs / DifferenceState::navigate

```rust
fn navigate(
        &mut self,
        same_block: bool,
        new_animation: bool,
        changed: bool,
        old_animation: bool,
    ) -> Option<OldDifferenceImage>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `changed && self.enabled && self.profile.selector != 0` | Local state/value operations | Handled | [77](../../crates/crysirror-system-nnn/src/native_cg_difference.rs#L77) |

### native_cg_difference.rs / DifferenceState::plan

```rust
fn plan(&self, column: u32) -> DifferencePrint
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `column != 0 \|\| !self.enabled \|\| self.profile.selector == 0` | Local state/value operations | Handled | [95](../../crates/crysirror-system-nnn/src/native_cg_difference.rs#L95) |

### native_cg_difference.rs / DifferenceState::printed

```rust
fn printed(&mut self) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `self.enabled && self.profile.selector != 0 && self.count < self.profile.frames` | self.count.wrapping_add | Handled | [111](../../crates/crysirror-system-nnn/src/native_cg_difference.rs#L111) |

### native_character_setting.rs / Setting::decode

```rust
fn decode(id: i32) -> Option<Self>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `id < 0` | Local state/value operations | Handled | [29](../../crates/crysirror-system-nnn/src/native_character_setting.rs#L29) |

### native_chart/model.rs / ChartRuntime::apply

```rust
fn apply(
        &mut self,
        state: &mut ChartState,
        name: &[u8],
        parameters: &[i32],
        variables: &mut Variables,
    ) -> Result<Option<ChartFunctionEffect>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `kind` | `0` | self.slot, self.mark | Handled | [514](../../crates/crysirror-system-nnn/src/native_chart/model.rs#L514) |
| `kind` | `1` | self.percent | Handled | [518](../../crates/crysirror-system-nnn/src/native_chart/model.rs#L518) |
| `kind` | `2` | self.slot, i32::from, self.complete | Handled | [524](../../crates/crysirror-system-nnn/src/native_chart/model.rs#L524) |
| `kind` | `3` | Local state/value operations | Handled | [531](../../crates/crysirror-system-nnn/src/native_chart/model.rs#L531) |
| `kind` | `4` | Local state/value operations | Handled | [541](../../crates/crysirror-system-nnn/src/native_chart/model.rs#L541) |
| `kind` | `5` | self.from, self.set_from, self.mark, self.slot | Handled | [542](../../crates/crysirror-system-nnn/src/native_chart/model.rs#L542) |
| `kind` | `_` | Local state/value operations | Handled | [554](../../crates/crysirror-system-nnn/src/native_chart/model.rs#L554) |

### native_config_layout.rs / Layout::read

```rust
fn read(
        table: &Names,
        config: &NativeConfiguration,
        system_voices: usize,
    ) -> Result<Self>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Some(name) = table.parameter(format!("filenameBG{page}").as_bytes())` | Local state/value operations | Handled | [195](../../crates/crysirror-system-nnn/src/native_config_layout.rs#L195) |
| `condition` | `id == 4` | Local state/value operations | Handled | [340](../../crates/crysirror-system-nnn/src/native_config_layout.rs#L340) |

### native_config_layout/appearance.rs / plan

```rust
fn plan(
    spec: &NativeButtonSpec,
    state: &mut NativeButtonState,
    picture: [u32; 2],
    count: i32,
    maximum: i32,
    kind: i32,
    delta: [i32; 2],
    number: i32,
    total: i32,
) -> Result<Option<Paint>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `kind & 1 != 0` | Local state/value operations | Handled | [42](../../crates/crysirror-system-nnn/src/native_config_layout/appearance.rs#L42) |

### native_config_layout/controls.rs / Button::calcu

```rust
fn calcu(&mut self, input: Option<ButtonInput>) -> i32
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `self.state.mode == 0` | Local state/value operations | Handled | [93](../../crates/crysirror-system-nnn/src/native_config_layout/controls.rs#L93) |
| `condition` | `self.state.mode == 1 && self.entered && !self.suppress_enter && self.state.count == self.spec.button.sound_waits[0] && (self.spec.raw_sounds[2] > 0 \|\| self.spec.voices[2] > 0)` | Local state/value operations | Handled | [103](../../crates/crysirror-system-nnn/src/native_config_layout/controls.rs#L103) |

### native_config_layout/session.rs / Session::init

```rust
fn init(
        layout: Arc<Layout>,
        table: &Names,
        configuration: &crate::native_bootstrap::NativeConfiguration,
        policy: &Policy,
        file: &NativeSystemFile,
        voices: &VoicePolicy,
        variables: &Variables,
        previous_page: usize,
        mask: i32,
    ) -> Result<Self>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under group.action=Action::SystemVoice(_)` | `let Some(id) = super::variable( table, configuration, &format!("systemVoice{index}OnVarName"), )` | Local state/value operations | Handled | [88](../../crates/crysirror-system-nnn/src/native_config_layout/session.rs#L88) |

### native_config_volume.rs / Profile::source_check_volume_exists

```rust
fn source_check_volume_exists(&self, id: i32) -> bool
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `id` | `105` | Local state/value operations | Handled | [280](../../crates/crysirror-system-nnn/src/native_config_volume.rs#L280) |
| `id` | `100` | Local state/value operations | Handled | [281](../../crates/crysirror-system-nnn/src/native_config_volume.rs#L281) |
| `id` | `104` | Local state/value operations | Handled | [282](../../crates/crysirror-system-nnn/src/native_config_volume.rs#L282) |
| `id` | `102` | Local state/value operations | Handled | [283](../../crates/crysirror-system-nnn/src/native_config_volume.rs#L283) |
| `id` | `103` | Local state/value operations | Handled | [284](../../crates/crysirror-system-nnn/src/native_config_volume.rs#L284) |
| `id` | `106` | Local state/value operations | Handled | [285](../../crates/crysirror-system-nnn/src/native_config_volume.rs#L285) |
| `id` | `107` | Local state/value operations | Handled | [286](../../crates/crysirror-system-nnn/src/native_config_volume.rs#L286) |
| `id` | `108` | Local state/value operations | Handled | [287](../../crates/crysirror-system-nnn/src/native_config_volume.rs#L287) |
| `id` | `109` | Local state/value operations | Handled | [288](../../crates/crysirror-system-nnn/src/native_config_volume.rs#L288) |
| `id` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [289](../../crates/crysirror-system-nnn/src/native_config_volume.rs#L289) |

### native_control.rs / ScriptSession::dispatch_control

```rust
fn dispatch_control(
        &mut self,
        receipt: &NativeCall,
        mut load: impl FnMut(&[u8]) -> Result<Arc<Program>>,
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `*code` | `0x5556000a \| 0x55560015 \| 0x55560016` | self.frames.last, Machine::with_variables, self.frames.last_mut, self.frames.push | Handled | [171](../../crates/crysirror-system-nnn/src/native_control.rs#L171) |
| `*code` | `0x55560006` | self.frames.last, self.frames.len, self.frames.pop | Handled | [204](../../crates/crysirror-system-nnn/src/native_control.rs#L204) |
| `*code` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [226](../../crates/crysirror-system-nnn/src/native_control.rs#L226) |

### native_curve.rs / curve

```rust
fn curve(max: i32, subframe: i32, kind: i32) -> Result<ProgressCurve>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `code / 15` | `2` | Local state/value operations | Handled | [16](../../crates/crysirror-system-nnn/src/native_curve.rs#L16) |
| `code / 15` | `4` | Local state/value operations | Handled | [17](../../crates/crysirror-system-nnn/src/native_curve.rs#L17) |
| `code / 15` | `5` | Local state/value operations | Handled | [18](../../crates/crysirror-system-nnn/src/native_curve.rs#L18) |
| `code / 15` | `_` | Local state/value operations | Handled | [19](../../crates/crysirror-system-nnn/src/native_curve.rs#L19) |

### native_curve.rs / progress

```rust
fn progress(count: i32, max: i32, subframe: i32, kind: i32, noise: f64) -> Result<f64>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `code % 15` | `1` | Local state/value operations | Handled | [92](../../crates/crysirror-system-nnn/src/native_curve.rs#L92) |
| `code % 15` | `2` | Local state/value operations | Handled | [93](../../crates/crysirror-system-nnn/src/native_curve.rs#L93) |
| `code % 15` | `3` | Local state/value operations | Handled | [94](../../crates/crysirror-system-nnn/src/native_curve.rs#L94) |
| `code % 15` | `4` | Local state/value operations | Handled | [95](../../crates/crysirror-system-nnn/src/native_curve.rs#L95) |
| `code % 15` | `5 \| 6` | Local state/value operations | Handled | [96](../../crates/crysirror-system-nnn/src/native_curve.rs#L96) |
| `code % 15` | `7 \| 8` | Local state/value operations | Handled | [100](../../crates/crysirror-system-nnn/src/native_curve.rs#L100) |
| `code % 15` | `9 \| 10` | Local state/value operations | Handled | [104](../../crates/crysirror-system-nnn/src/native_curve.rs#L104) |
| `code % 15` | `_` | Local state/value operations | Handled | [108](../../crates/crysirror-system-nnn/src/native_curve.rs#L108) |
| `code / 15` | `2` | Local state/value operations | Handled | [111](../../crates/crysirror-system-nnn/src/native_curve.rs#L111) |
| `code / 15` | `4` | Local state/value operations | Handled | [112](../../crates/crysirror-system-nnn/src/native_curve.rs#L112) |
| `code / 15` | `5` | Local state/value operations | Handled | [113](../../crates/crysirror-system-nnn/src/native_curve.rs#L113) |
| `code / 15` | `_` | Local state/value operations | Handled | [114](../../crates/crysirror-system-nnn/src/native_curve.rs#L114) |

### native_curve.rs / quantize

```rust
fn quantize(start: i32, end: i32, kind: i32, progress: f64) -> Result<i32>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `kind / 100 % 2 != 0` | Local state/value operations | Handled | [130](../../crates/crysirror-system-nnn/src/native_curve.rs#L130) |

### native_curve.rs / value_with_random

```rust
fn value_with_random(
    count: i32,
    max: i32,
    start: i32,
    end: i32,
    subframe: i32,
    kind: i32,
    random: &mut crysirror_image::temporal_pattern::PatternRng,
) -> Result<i32>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `kind / 200 != 0` | f64::from | Handled | [160](../../crates/crysirror-system-nnn/src/native_curve.rs#L160) |

### native_curve/source32.rs / SourceCurveContext::shaped_progress

```rust
fn shaped_progress(&self, count: i32, max: i32, subframe: i32, kind: i32) -> Result<f64>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `kind % 100 / 15 == 3` | f64::from, self.custom_parameter.clamp | Handled | [26](../../crates/crysirror-system-nnn/src/native_curve/source32.rs#L26) |

### native_curve/source32.rs / SourceCurveContext::value_with_random

```rust
fn value_with_random(
        &self,
        count: i32,
        max: i32,
        start: i32,
        end: i32,
        subframe: i32,
        kind: i32,
        random: &mut PatternRng,
    ) -> Result<i32>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `kind / 200 != 0` | f64::from | Handled | [72](../../crates/crysirror-system-nnn/src/native_curve/source32.rs#L72) |

### native_effects.rs / FadeInState::image_effect_with

```rust
fn image_effect_with(
        &self,
        random: Option<&mut crysirror_image::temporal_pattern::PatternRng>,
    ) -> Result<crysirror_image::effects::ImageEffect>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `self.command() == 54` | crysirror_image::rgb_lut::RgbLookup::weighted_color_fixed, self.color.map | Handled | [168](../../crates/crysirror-system-nnn/src/native_effects.rs#L168) |

### native_film_button.rs / Presenter::poll

```rust
fn poll(
        &mut self,
        host: &mut dyn HostServices,
        input: InputFrame,
        admit: bool,
        period: u64,
        sound: &mut dyn FnMut(&mut dyn HostServices, i32) -> Result<bool>,
    ) -> Result<Poll>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `state.mode == 1 && self.entered && state.count == spec.sound_waits[0] && spec.sound_waits[0] > 0` | Local state/value operations | Handled | [238](../../crates/crysirror-system-nnn/src/native_film_button.rs#L238) |
| `condition` | `state.mode == 0 && self.exited && state.count == spec.sound_waits[1] && spec.sound_waits[1] > 0` | Local state/value operations | Handled | [245](../../crates/crysirror-system-nnn/src/native_film_button.rs#L245) |

### native_flower.rs / FlowerClass::image_input

```rust
fn image_input(&self, p: &[i32; 4], base: ImageInput) -> Result<Option<ImageInput>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `(f.kind as i32) < p[1] \|\| (f.kind as i32) > p[2]` | Local state/value operations | Handled | [170](../../crates/crysirror-system-nnn/src/native_flower.rs#L170) |

### native_general.rs / ModeReturn::read

```rust
fn read(table: &Names, target: Option<NativeMode>) -> Result<Self>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `kind == 0` | Local state/value operations | Handled | [91](../../crates/crysirror-system-nnn/src/native_general.rs#L91) |
| `condition; under receiver=else: kind == 0` | `kind>0` | Local state/value operations | Handled | [101](../../crates/crysirror-system-nnn/src/native_general.rs#L101) |

### native_general_audio.rs / GeneralAudioOwner::admit_mode

```rust
fn admit_mode(&mut self, update: u64, mode: i32, omake: bool) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Some(event) = self .modes .get_mut(&mode) .ok_or_else(\|\| anyhow::anyhow!("NNN General audio object absent for mode{mode}"))? .calcu()` | self.queue.push_back | Handled | [380](../../crates/crysirror-system-nnn/src/native_general_audio.rs#L380) |

### native_general_audio.rs / GeneralAudioPolicy::for_mode

```rust
fn for_mode(mode: i32, table: &Names) -> Result<Self>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `mode` | `4 \| 5 \| 6 \| 8 \| 12 \| 15 \| 16 \| 17 \| 18 \| 20 \| 22 \| 26 \| 29 \| 31 \| 32 \| 35 \| 36 \| 38 \| 39 \| 41 \| 42 \| 43 \| 44 \| 46 \| 47 \| 48 \| 49` | Self::read | Handled | [32](../../crates/crysirror-system-nnn/src/native_general_audio.rs#L32) |
| `mode` | `1 \| 2 \| 3 \| 7 \| 9 \| 10 \| 13 \| 23 \| 24 \| 25 \| 27 \| 33 \| 34 \| 40 \| 45` | Local state/value operations | Handled | [34](../../crates/crysirror-system-nnn/src/native_general_audio.rs#L34) |
| `mode` | `_` | Local state/value operations | Handled | [43](../../crates/crysirror-system-nnn/src/native_general_audio.rs#L43) |

### native_general_entry.rs / ModeEntry::read

```rust
fn read(table: &Names) -> Result<Option<Self>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `code == 0` | Local state/value operations | Handled | [33](../../crates/crysirror-system-nnn/src/native_general_entry.rs#L33) |
| `code` | `-1` | Local state/value operations | Handled | [42](../../crates/crysirror-system-nnn/src/native_general_entry.rs#L42) |
| `code` | `n if n >= 1` | EntryKind::Wipe | Handled | [43](../../crates/crysirror-system-nnn/src/native_general_entry.rs#L43) |
| `code` | `_` | Local state/value operations | Handled | [50](../../crates/crysirror-system-nnn/src/native_general_entry.rs#L50) |
| `condition` | `code == -1` | Local state/value operations | Handled | [52](../../crates/crysirror-system-nnn/src/native_general_entry.rs#L52) |

### native_hahen_grid.rs / spec

```rust
fn spec(key: Key, canvas: [u32; 2]) -> Result<Option<FragmentGrid>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `key.0[0] != 1` | Local state/value operations | Handled | [18](../../crates/crysirror-system-nnn/src/native_hahen_grid.rs#L18) |
| `key.0[1]` | `0` | Local state/value operations | Handled | [22](../../crates/crysirror-system-nnn/src/native_hahen_grid.rs#L22) |
| `key.0[1]` | `1` | Local state/value operations | Handled | [23](../../crates/crysirror-system-nnn/src/native_hahen_grid.rs#L23) |
| `key.0[1]` | `2` | Local state/value operations | Handled | [24](../../crates/crysirror-system-nnn/src/native_hahen_grid.rs#L24) |
| `key.0[1]` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [25](../../crates/crysirror-system-nnn/src/native_hahen_grid.rs#L25) |

### native_hahen_set.rs / Preparation::begin

```rust
fn begin(
        class: &HahenClass,
        parameters: [i32; 18],
        canvas: [u32; 2],
        random: PatternRng,
        context: &ContextData,
    ) -> Result<Self>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `key.0[0] == 2` | Work::External, ExternalPreparation::begin | Handled | [42](../../crates/crysirror-system-nnn/src/native_hahen_set.rs#L42) |

### native_logo_audio.rs / Clock::calcu

```rust
fn calcu(&mut self, logo: usize) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `spec.kind == 0` | Event::SystemWave | Handled | [76](../../crates/crysirror-system-nnn/src/native_logo_audio.rs#L76) |

### native_message_control.rs / NativeMessageControl::start_koma_with_history

```rust
fn start_koma_with_history(
        &mut self,
        values: &[i32],
        skip: &mut NativeSkipPolicy,
        read: Option<bool>,
    ) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `values.first().is_some_and(\|id\| *id != -1) && skip.next_command && skip.f4 && skip.effect_command` | Local state/value operations | Handled | [84](../../crates/crysirror-system-nnn/src/native_message_control.rs#L84) |

### native_message_control.rs / NativeMessageControl::start_message_with_history

```rust
fn start_message_with_history(
        &mut self,
        values: &[i32],
        skip: &mut NativeSkipPolicy,
        read: Option<bool>,
    ) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `values.first().is_some_and(\|id\| *id != -1) && skip.next_command && skip.f4` | Local state/value operations | Handled | [47](../../crates/crysirror-system-nnn/src/native_message_control.rs#L47) |
| `condition` | `id >= 0` | Local state/value operations | Handled | [57](../../crates/crysirror-system-nnn/src/native_message_control.rs#L57) |

### native_old_lamp.rs / OldLampState::calcu

```rust
fn calcu(&mut self,rng:&mut PatternRng) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `self.mode==0` | Local state/value operations | Handled | [33](../../crates/crysirror-system-nnn/src/native_old_lamp.rs#L33) |

### native_old_lamp.rs / OldLampState::visible

```rust
fn visible(&self,has_picture:bool,rng:&mut PatternRng) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!has_picture\|\|self.mode==2` | Local state/value operations | Handled | [45](../../crates/crysirror-system-nnn/src/native_old_lamp.rs#L45) |
| `condition` | `self.mode==1` | Local state/value operations | Handled | [46](../../crates/crysirror-system-nnn/src/native_old_lamp.rs#L46) |

### native_overlap.rs / NativeOverlap::poll_admitted_frame

```rust
fn poll_admitted_frame(&mut self, host: &mut dyn HostServices) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under job.poll()?=ImageProcessState::Ready(image)` | `let Err(error)=host.submit_engine(self.client,1,&serde_json::to_vec(&serde_json::json!({"layers":[{"op":"put","layer":{"id":self.display,"rect":[0,0,w,h],"order":[self.order,0],"paint":{"kind":"image","asset":published.path.relative()}}}]}))?)` | host.release_image_resource | Unsupported: Unconditional rejection | [133](../../crates/crysirror-system-nnn/src/native_overlap.rs#L133) |

### native_particle_classes.rs / Classes::restore_setter

```rust
fn restore_setter(
        &mut self,
        command: i32,
        p: &[i32],
        slot: i32,
        canvas: [u32; 2],
        random: &mut PatternRng,
    ) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `command == 138` | crate::native_hotaru_new::restored, self.set | Handled | [167](../../crates/crysirror-system-nnn/src/native_particle_classes.rs#L167) |

### native_particle_classes.rs / Classes::set

```rust
fn set(
        &mut self,
        command: i32,
        parameters: &[i32],
        slot: i32,
        canvas: [u32; 2],
        random: &mut PatternRng,
    ) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `command` | `14` | Arc::make_mut | Handled | [125](../../crates/crysirror-system-nnn/src/native_particle_classes.rs#L125) |
| `command` | `57` | Arc::make_mut | Handled | [126](../../crates/crysirror-system-nnn/src/native_particle_classes.rs#L126) |
| `command` | `70` | Arc::make_mut | Handled | [127](../../crates/crysirror-system-nnn/src/native_particle_classes.rs#L127) |
| `command` | `71` | Arc::make_mut | Handled | [133](../../crates/crysirror-system-nnn/src/native_particle_classes.rs#L133) |
| `command` | `138` | Arc::make_mut | Handled | [139](../../crates/crysirror-system-nnn/src/native_particle_classes.rs#L139) |
| `command` | `12 \| 82 \| 83 \| 97` | Local state/value operations | Handled | [154](../../crates/crysirror-system-nnn/src/native_particle_classes.rs#L154) |
| `command` | `_` | Local state/value operations | Handled | [155](../../crates/crysirror-system-nnn/src/native_particle_classes.rs#L155) |

### native_picture_effect.rs / PictureEffectFrame::image_input

```rust
fn image_input(
        &self,
        base: ImageInput,
        random: &mut PatternRng,
    ) -> Result<Option<ImageInput>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `command; under self.state.kind=PictureEffectKind::Circle{parameters,command,..}` | `0` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [1131](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L1131) |
| `command; under self.state.kind=PictureEffectKind::Circle{parameters,command,..}` | `54` | Local state/value operations | Handled | [1132](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L1132) |
| `command; under self.state.kind=PictureEffectKind::Circle{parameters,command,..}` | `_` | crate::native_picture_circle::image_input | Handled | [1133](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L1133) |

### native_picture_effect.rs / PictureEffectState::companion_slot

```rust
fn companion_slot(&self,layers:u16) -> Option<(i32,bool)>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `self.kind` | `PictureEffectKind::Buffered{command:59,parameters}` | Local state/value operations | Handled | [901](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L901) |
| `self.kind` | `PictureEffectKind::DualMask{parameters}` | i32::from | Handled | [902](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L902) |
| `self.kind` | `PictureEffectKind::MaskAppear{parameters} if parameters[1]!=-1` | i32::from | Handled | [903](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L903) |
| `self.kind` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [904](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L904) |

### native_picture_effect.rs / PictureEffectState::parameters

```rust
fn parameters(&self) -> Vec<i32>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `command; under self.kind=PictureEffectKind::Buffered { command, parameters, }` | `59` | Local state/value operations | Handled | [828](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L828) |
| `command; under self.kind=PictureEffectKind::Buffered { command, parameters, }` | `66` | Local state/value operations | Handled | [829](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L829) |
| `command; under self.kind=PictureEffectKind::Buffered { command, parameters, }` | `_` | Local state/value operations | Handled | [830](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L830) |

### native_picture_effect.rs / PictureEffectState::parse

```rust
fn parse(command: i32, p: &[i32]) -> Result<Self>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `command==137` | crate::native_animation::parameters | Handled | [307](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L307) |
| `condition` | `matches!(command,81\|97\|70\|71\|83\|138\|12\|14\|57\|24\|104\|87\|88\|89\|114\|115\|116\|117\|118)` | Self::parse_for_canvas | Handled | [309](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L309) |
| `condition; under command=99 \| 120` | `command == 120` | Local state/value operations | Handled | [487](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L487) |
| `command` | `_` | Local state/value operations | Handled | [501](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L501) |

### native_picture_effect.rs / PictureEffectState::parse_for_canvas

```rust
fn parse_for_canvas(command: i32, p: &[i32], canvas: [u32; 2]) -> Result<Self>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `command==137` | Self::parse | Handled | [171](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L171) |
| `condition` | `command==81` | crate::native_hahen::parameters | Handled | [172](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L172) |
| `condition` | `command==97` | crate::native_bath_window::parameters | Handled | [173](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L173) |
| `condition` | `command==83` | crate::native_beach::parameters | Handled | [174](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L174) |
| `condition` | `command==70` | crate::native_hanabira::parameters | Handled | [175](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L175) |
| `condition` | `command==71` | crate::native_hotaru::parameters | Handled | [176](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L176) |
| `condition` | `command==138` | crate::native_hotaru_new::parameters | Handled | [177](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L177) |
| `condition` | `command==12` | crate::native_flower::parameters | Handled | [178](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L178) |
| `condition` | `matches!(command,14\|57)` | crate::native_snow::parameters | Handled | [179](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L179) |
| `condition` | `command==82` | crate::native_cloud::parameters | Handled | [180](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L180) |
| `condition` | `command==69` | crate::native_hamon::parameters | Handled | [181](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L181) |
| `condition` | `command==80` | crate::native_staff_roll::parameters | Handled | [182](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L182) |
| `condition` | `command==85` | crate::native_speed_line::parameters | Handled | [183](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L183) |
| `condition` | `command==102` | crate::native_rain4::parameters | Handled | [184](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L184) |
| `condition` | `command==42` | crate::native_kira::parameters | Handled | [185](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L185) |
| `condition` | `command==28` | crate::native_picture_circle::parameters | Handled | [186](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L186) |
| `condition` | `command==105` | crate::native_picture_movie_warp::parameters | Handled | [187](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L187) |
| `condition` | `command==67` | crate::native_picture_pers::parameters | Handled | [188](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L188) |
| `condition` | `command==51` | crate::native_taiyo::parameters | Handled | [189](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L189) |
| `condition` | `command==93` | crate::native_grass_plane::parameters | Handled | [190](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L190) |
| `condition` | `command==111` | crate::native_picture_fish::parameters | Handled | [191](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L191) |
| `condition` | `command==22` | crate::native_kagerou::parameters | Handled | [195](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L195) |
| `condition` | `command==24` | crate::native_water::parameters | Handled | [200](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L200) |
| `condition` | `command==104` | crate::native_picture_konoha::parameters | Handled | [205](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L205) |
| `condition` | `matches!(command,87\|88\|89\|114\|115\|116\|117\|118)` | crate::native_picture_geo::GeoParameters::parse | Handled | [210](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L210) |
| `condition` | `command==139` | crate::native_picture_chara_mosaic::parameters | Handled | [215](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L215) |
| `condition` | `command==86` | crate::native_picture_zantetsu::parameters | Handled | [220](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L220) |
| `condition` | `command==55` | crate::native_picture_old_film::parameters | Handled | [225](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L225) |
| `condition` | `command==122` | crate::native_picture_clip_zoom::parameters | Handled | [230](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L230) |
| `condition` | `command == 103` | crate::native_picture_target::parameters | Handled | [235](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L235) |
| `condition` | `command == 23` | crate::native_picture_motion::raster_parameters | Handled | [241](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L241) |
| `condition` | `command == 37` | crate::native_picture_motion::zoom_parameters | Handled | [252](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L252) |
| `condition` | `command == 50` | std::array::from_fn | Handled | [263](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L263) |
| `condition` | `command != 36` | Self::parse | Handled | [293](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L293) |

### native_picture_effect.rs / PictureEffectState::uses_mask

```rust
fn uses_mask(&self, background: bool) -> bool
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `self.kind` | `PictureEffectKind::Rain{..}\|PictureEffectKind::Kagerou{..}\|PictureEffectKind::Remember{..}\|PictureEffectKind::Taiyo{..}` | Local state/value operations | Handled | [909](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L909) |
| `self.kind` | `PictureEffectKind::Rain2{..}\|PictureEffectKind::Waterfall{..}\|PictureEffectKind::Fish{..}\|PictureEffectKind::Hae{..}\|PictureEffectKind::Ke{..}` | Local state/value operations | Handled | [910](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L910) |
| `self.kind` | `PictureEffectKind::Animation{..}\|PictureEffectKind::Hahen{..}\|PictureEffectKind::BathWindow{..}\|PictureEffectKind::Beach{..}\|PictureEffectKind::Hanabira{..}\|PictureEffectKind::Hotaru{..}\|PictureEffectKind::HotaruNew{..}\|PictureEffectKind::Flower{..}\|PictureEffectKind::Snow{..}\|PictureEffectKind::Cloud{..}\|PictureEffectKind::Hamon{..}\|PictureEffectKind::CharaMetal{..}\|PictureEffectKind::Komorebi{..}\|PictureEffectKind::SunRays{..}\|PictureEffectKind::StaffRoll{..}\|PictureEffectKind::SpeedLine{..}\|PictureEffectKind::Rain4{..}\|PictureEffectKind::Kira{..}\|PictureEffectKind::Slash{..}\|PictureEffectKind::Circle{..}\|PictureEffectKind::MaskWipe{..}\|PictureEffectKind::Soyosoyo{..}\|PictureEffectKind::MovieWarp{..}\|PictureEffectKind::FaceAnimation{..}\|PictureEffectKind::Turn{..}\|PictureEffectKind::TurnNew{..}\|PictureEffectKind::Perspective{..}\|PictureEffectKind::GrassPlane{..}\|PictureEffectKind::Water{..}\|PictureEffectKind::WaterDrop{..}\|PictureEffectKind::FullAnime{..}\|PictureEffectKind::BankAnimation{..}\|PictureEffectKind::Change{..}` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [911](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L911) |
| `self.kind` | `PictureEffectKind::Geometry{..}` | Local state/value operations | Handled | [912](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L912) |
| `self.kind` | `PictureEffectKind::Konoha{..}\|PictureEffectKind::CharaMosaic{..}\|PictureEffectKind::BossDeath{..}\|PictureEffectKind::Zantetsu{..}\|PictureEffectKind::StoppedBoss{..}` | Local state/value operations | Handled | [913](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L913) |
| `self.kind` | `PictureEffectKind::TaClip{..}\|PictureEffectKind::SpecialMove{..}` | Local state/value operations | Handled | [914](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L914) |
| `self.kind` | `PictureEffectKind::Rainbow{..}\|PictureEffectKind::Meisai{..}` | Local state/value operations | Handled | [915](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L915) |
| `self.kind` | `PictureEffectKind::Relief{..}\|PictureEffectKind::OldFilm{..}\|PictureEffectKind::BackgroundBlur{..}` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [916](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L916) |
| `self.kind` | `PictureEffectKind::CharaBlur{..}` | Local state/value operations | Handled | [917](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L917) |
| `self.kind` | `PictureEffectKind::DualMask{..}\|PictureEffectKind::MaskAppear{..}\|PictureEffectKind::Sentence{..}` | Local state/value operations | Handled | [918](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L918) |
| `self.kind` | `PictureEffectKind::StretchBlit{parameters}` | Local state/value operations | Handled | [919](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L919) |
| `self.kind` | `PictureEffectKind::ClipZoom{..}` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [920](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L920) |
| `self.kind` | `PictureEffectKind::Koma { parameters }` | Local state/value operations | Handled | [921](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L921) |
| `self.kind` | `PictureEffectKind::Ghost { .. }` | Local state/value operations | Handled | [922](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L922) |
| `self.kind` | `PictureEffectKind::Noise { .. }` | Local state/value operations | Handled | [923](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L923) |
| `self.kind` | `PictureEffectKind::Target { .. }` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [924](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L924) |
| `self.kind` | `PictureEffectKind::CharaRaster { .. }` | Local state/value operations | Handled | [925](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L925) |
| `self.kind` | `PictureEffectKind::Saike { .. }` | Local state/value operations | Handled | [926](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L926) |
| `self.kind` | `PictureEffectKind::Lupin { .. }` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [927](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L927) |
| `self.kind` | `PictureEffectKind::MeltDown { .. }` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [928](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L928) |
| `self.kind` | `PictureEffectKind::Curtain { .. }` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [929](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L929) |
| `self.kind` | `PictureEffectKind::Raster { .. } \| PictureEffectKind::ScrollLoop { .. }` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [930](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L930) |
| `self.kind` | `PictureEffectKind::Quadratic { .. }` | Local state/value operations | Handled | [931](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L931) |
| `self.kind` | `PictureEffectKind::Bezier { .. }` | Local state/value operations | Handled | [932](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L932) |
| `self.kind` | `PictureEffectKind::BoxBlur { .. }` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [933](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L933) |
| `self.kind` | `PictureEffectKind::Jitter { .. }` | Local state/value operations | Handled | [934](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L934) |
| `self.kind` | `PictureEffectKind::Zoom { .. }` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [935](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L935) |
| `self.kind` | `PictureEffectKind::ScreenGradient { .. }` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [936](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L936) |
| `self.kind` | `PictureEffectKind::ScreenLight { .. }` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [937](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L937) |
| `self.kind` | `PictureEffectKind::AddSubtract { .. }` | Local state/value operations | Handled | [938](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L938) |
| `self.kind` | `PictureEffectKind::Stretch { .. }` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [939](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L939) |
| `self.kind` | `PictureEffectKind::Buffered { command: 75, .. }` | Local state/value operations | Handled | [940](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L940) |
| `self.kind` | `PictureEffectKind::Buffered { .. }` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [941](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L941) |
| `self.kind` | `PictureEffectKind::Trans { .. }` | Local state/value operations | Handled | [942](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L942) |
| `self.kind` | `PictureEffectKind::Flash { .. } \| PictureEffectKind::Grey { .. }` | Local state/value operations | Handled | [943](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L943) |
| `self.kind` | `PictureEffectKind::ColorAdd { transparent, balance, .. }` | Local state/value operations | Handled | [944](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L944) |

### native_picture_effect.rs / PictureEffectState::validate

```rust
fn validate(&self) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let PictureEffectKind::Buffered { command: 59, parameters: p, } = self.kind` | crate::native_curve::validate | Handled | [720](../../crates/crysirror-system-nnn/src/native_picture_effect.rs#L720) |

### native_picture_geo.rs / GeoParameters::frames

```rust
fn frames(&self) -> i32
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(self.command, 88 \| 117)` | Local state/value operations | Handled | [48](../../crates/crysirror-system-nnn/src/native_picture_geo.rs#L48) |

### native_picture_geo.rs / GeoParameters::from_saved

```rust
fn from_saved(command: i32, values: &[i32]) -> Result<Self>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `command` | `114 \| 118` | Local state/value operations | Handled | [17](../../crates/crysirror-system-nnn/src/native_picture_geo.rs#L17) |
| `command` | `87 \| 115` | Local state/value operations | Handled | [18](../../crates/crysirror-system-nnn/src/native_picture_geo.rs#L18) |
| `command` | `89 \| 116` | Local state/value operations | Handled | [19](../../crates/crysirror-system-nnn/src/native_picture_geo.rs#L19) |
| `command` | `88 \| 117` | Local state/value operations | Handled | [20](../../crates/crysirror-system-nnn/src/native_picture_geo.rs#L20) |
| `command` | `_` | Local state/value operations | Handled | [21](../../crates/crysirror-system-nnn/src/native_picture_geo.rs#L21) |

### native_picture_geo.rs / GeoParameters::image_input

```rust
fn image_input(
        &self,
        frame: &crate::native_picture_effect::PictureEffectFrame,
        base: crysirror_core::image_processing::ImageInput,
        random: &mut PatternRng,
    ) -> Result<Option<crysirror_core::image_processing::ImageInput>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under self.command=114 \| 118` | `self.command == 118` | Local state/value operations | Handled | [103](../../crates/crysirror-system-nnn/src/native_picture_geo.rs#L103) |
| `self.command` | `87 \| 115` | self.words.as_slice | Handled | [80](../../crates/crysirror-system-nnn/src/native_picture_geo.rs#L80) |
| `self.command` | `89 \| 116` | self.words.as_slice | Handled | [87](../../crates/crysirror-system-nnn/src/native_picture_geo.rs#L87) |
| `self.command` | `114 \| 118` | self.words.as_slice, crysirror_core::region_map::clip_region_copy, Box::new | Handled | [95](../../crates/crysirror-system-nnn/src/native_picture_geo.rs#L95) |
| `self.command` | `88 \| 117` | self.words.as_slice, Box::new | Handled | [146](../../crates/crysirror-system-nnn/src/native_picture_geo.rs#L146) |
| `self.command` | `_` | Local state/value operations | Partial: NNN unsupported geometric command | [163](../../crates/crysirror-system-nnn/src/native_picture_geo.rs#L163) |

### native_picture_geo.rs / GeoParameters::parse

```rust
fn parse(command: i32, values: &[i32], canvas: [u32; 2]) -> Result<Self>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `command` | `114 \| 118` | Local state/value operations | Handled | [33](../../crates/crysirror-system-nnn/src/native_picture_geo.rs#L33) |
| `command` | `87 \| 115` | Local state/value operations | Handled | [34](../../crates/crysirror-system-nnn/src/native_picture_geo.rs#L34) |
| `command` | `89 \| 116` | Local state/value operations | Handled | [35](../../crates/crysirror-system-nnn/src/native_picture_geo.rs#L35) |
| `command` | `88 \| 117` | Local state/value operations | Handled | [36](../../crates/crysirror-system-nnn/src/native_picture_geo.rs#L36) |
| `command` | `_` | Local state/value operations | Handled | [37](../../crates/crysirror-system-nnn/src/native_picture_geo.rs#L37) |

### native_picture_geo.rs / GeoParameters::validate

```rust
fn validate(&self, frames: i32) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `self.command` | `114 \| 118` | Local state/value operations | Handled | [59](../../crates/crysirror-system-nnn/src/native_picture_geo.rs#L59) |
| `self.command` | `87 \| 115` | self.words.as_slice | Handled | [60](../../crates/crysirror-system-nnn/src/native_picture_geo.rs#L60) |
| `self.command` | `89 \| 116` | self.words.as_slice | Handled | [61](../../crates/crysirror-system-nnn/src/native_picture_geo.rs#L61) |
| `self.command` | `88 \| 117` | Local state/value operations | Handled | [62](../../crates/crysirror-system-nnn/src/native_picture_geo.rs#L62) |
| `self.command` | `_` | Local state/value operations | Handled | [63](../../crates/crysirror-system-nnn/src/native_picture_geo.rs#L63) |

### native_picture_mask_wipe.rs / needs_capture

```rust
fn needs_capture(command: i32, p: &[i32; 10]) -> bool
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `command == 60` | Local state/value operations | Handled | [82](../../crates/crysirror-system-nnn/src/native_picture_mask_wipe.rs#L82) |

### native_picture_mask_wipe.rs / parameters

```rust
fn parameters(command: i32, values: &[i32]) -> Result<[i32; 10]>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `command == 60 && values.len() < 4` | Local state/value operations | Handled | [23](../../crates/crysirror-system-nnn/src/native_picture_mask_wipe.rs#L23) |

### native_picture_mask_wipe.rs / validate

```rust
fn validate(command: i32, p: &[i32; 10], frames: i32) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `command == 60` | Local state/value operations | Handled | [32](../../crates/crysirror-system-nnn/src/native_picture_mask_wipe.rs#L32) |

### native_picture_pers.rs / image_input

```rust
fn image_input(
    frame: &PictureEffectFrame,
    p: &[i32; 16],
    base: ImageInput,
) -> Result<Option<ImageInput>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `mode == 2 && !picture.has_mask` | Local state/value operations | Handled | [90](../../crates/crysirror-system-nnn/src/native_picture_pers.rs#L90) |

### native_picture_stretch_blt.rs / draw_plan

```rust
fn draw_plan(
    p: [i32; 25],
    phase: StretchPhase,
    count: i32,
    frames: i32,
    picture: [u32; 2],
    point: [i32; 2],
    canvas: [u32; 2],
    has_mask: bool,
    random: &mut PatternRng,
) -> Result<Option<StretchPlan>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `mode` | `0 \| 1` | Local state/value operations | Handled | [149](../../crates/crysirror-system-nnn/src/native_picture_stretch_blt.rs#L149) |
| `mode` | `2` | Local state/value operations | Handled | [150](../../crates/crysirror-system-nnn/src/native_picture_stretch_blt.rs#L150) |
| `mode` | `_` | Local state/value operations | Handled | [154](../../crates/crysirror-system-nnn/src/native_picture_stretch_blt.rs#L154) |

### native_picture_turn.rs / image_input

```rust
fn image_input(
    frame: &PictureEffectFrame,
    p: &[i32; 36],
    base: ImageInput,
    rng: &mut PatternRng,
) -> Result<Option<ImageInput>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `mode == 0 \|\| mode == 1` | Local state/value operations | Handled | [187](../../crates/crysirror-system-nnn/src/native_picture_turn.rs#L187) |
| `condition` | `mode == 0 \|\| mode == 1` | Local state/value operations | Handled | [198](../../crates/crysirror-system-nnn/src/native_picture_turn.rs#L198) |

### native_presentation.rs / MoveFrame::command

```rust
fn command(&self) -> i32
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `self.native_command == 0` | Local state/value operations | Handled | [59](../../crates/crysirror-system-nnn/src/native_presentation.rs#L59) |

### native_presentation.rs / MoveFrame::count_up

```rust
fn count_up(&mut self, steps: u32) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `self.native_command == 0` | Local state/value operations | Handled | [66](../../crates/crysirror-system-nnn/src/native_presentation.rs#L66) |

### native_presentation.rs / NativeFrameWork::next

```rust
fn next(&mut self, host: &mut dyn HostServices) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let crate::native_picture_effect::PictureEffectKind::Circle{parameters,command:28,..}=state.kind` | self.buffers.circle_ends.get, crate::native_picture_circle::terminal | Handled | [716](../../crates/crysirror-system-nnn/src/native_presentation.rs#L716) |
| `condition; under effect=NativeFrameEffect::Picture(mut frame)` | `matches!(frame.state.kind,crate::native_picture_effect::PictureEffectKind::Circle{command:28,..})` | self.buffers.circle_ends.insert | Handled | [907](../../crates/crysirror-system-nnn/src/native_presentation.rs#L907) |
| `condition; under effect=NativeFrameEffect::Picture(mut frame)` | `command==75` | self.buffers.once_printed.contains, self.buffers.once_printed.insert, self.next, Work::Effect, host.image_metadata | Handled | [1077](../../crates/crysirror-system-nnn/src/native_presentation.rs#L1077) |
| `condition; under effect=NativeFrameEffect::Picture(mut frame)` | `command==66` | self.next | Handled | [1084](../../crates/crysirror-system-nnn/src/native_presentation.rs#L1084) |
| `condition; under effect=NativeFrameEffect::Picture(mut frame)` | `command==73` | self.current.clone, self.next | Handled | [1089](../../crates/crysirror-system-nnn/src/native_presentation.rs#L1089) |
| `condition; under effect=NativeFrameEffect::Picture(mut frame)` | `command==59&&!frame.wipe_can_print()` | self.next | Handled | [1093](../../crates/crysirror-system-nnn/src/native_presentation.rs#L1093) |
| `condition; under effect=NativeFrameEffect::Picture(mut frame)` | `command==2` | self.current.clone, ImageInput::Resource, self.buffers.capture.clone, Work::Effect, host.image_metadata | Handled | [1098](../../crates/crysirror-system-nnn/src/native_presentation.rs#L1098) |

### native_radio_group.rs / ButtonState::calcu

```rust
fn calcu(&mut self, spec: &RadioButtonSpec, input: Option<ButtonInput>) -> i32
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `self.state.mode == 0` | Local state/value operations | Handled | [242](../../crates/crysirror-system-nnn/src/native_radio_group.rs#L242) |
| `condition` | `self.state.mode == 1 && self.enter && !self.suppress_enter && self.state.count == spec.button.sound_waits[0] && (spec.raw_sounds[2] > 0 \|\| spec.voices[2] > 0)` | Local state/value operations | Handled | [252](../../crates/crysirror-system-nnn/src/native_radio_group.rs#L252) |

### native_result.rs / NativeResultData::apply

```rust
fn apply(
        &mut self,
        name: &[u8],
        parameters: &[i32],
        variables: &mut Variables,
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `name.eq_ignore_ascii_case(b"setresult")` | usize::try_from, self.notices.push_back | Handled | [80](../../crates/crysirror-system-nnn/src/native_result.rs#L80) |
| `condition` | `name.eq_ignore_ascii_case(b"getresult") && self.get_registered` | usize::try_from, self.words.get, i32::from | Handled | [119](../../crates/crysirror-system-nnn/src/native_result.rs#L119) |

### native_save.rs / LegacySaveProfile::for_configuration

```rust
fn for_configuration(
        config: &crate::native_bootstrap::NativeConfiguration,
    ) -> Result<Self>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `mode <= 0` | Local state/value operations | Handled | [102](../../crates/crysirror-system-nnn/src/native_save.rs#L102) |
| `condition` | `mode == 34` | u32::try_from, crate::native_chart::ChartLayout::read | Handled | [105](../../crates/crysirror-system-nnn/src/native_save.rs#L105) |

### native_simple_wipe.rs / needs_random

```rust
fn needs_random(kind: i32) -> bool
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(kind, 24 \| 25 \| 38 \| 39 \| 48..=53)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [23](../../crates/crysirror-system-nnn/src/native_simple_wipe.rs#L23) |

### native_simple_wipe.rs / plan

```rust
fn plan(
    kind: i32,
    old: Arc<dyn ImageResource>,
    new: Arc<dyn ImageResource>,
    count: u32,
    duration: u32,
) -> Result<(ImageInput, Vec<ImageEffect>)>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(kind, 34 \| 35)` | fragments::plan | Handled | [132](../../crates/crysirror-system-nnn/src/native_simple_wipe.rs#L132) |
| `condition` | `matches!(kind, 34 \| 35)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [132](../../crates/crysirror-system-nnn/src/native_simple_wipe.rs#L132) |
| `condition` | `kind == 0` | u64::from | Handled | [136](../../crates/crysirror-system-nnn/src/native_simple_wipe.rs#L136) |
| `condition; under kind=36 \| 37 \| 54..=59` | `kind < 54` | Local state/value operations | Handled | [197](../../crates/crysirror-system-nnn/src/native_simple_wipe.rs#L197) |
| `condition; under kind=88..=93` | `kind < 90` | Local state/value operations | Handled | [230](../../crates/crysirror-system-nnn/src/native_simple_wipe.rs#L230) |
| `condition; under kind=69..=72 \| 74..=77` | `kind < 73` | Local state/value operations | Handled | [252](../../crates/crysirror-system-nnn/src/native_simple_wipe.rs#L252) |
| `kind` | `80..=83` | Local state/value operations | Handled | [162](../../crates/crysirror-system-nnn/src/native_simple_wipe.rs#L162) |
| `kind` | `84..=87` | Local state/value operations | Handled | [167](../../crates/crysirror-system-nnn/src/native_simple_wipe.rs#L167) |
| `kind` | `28 \| 29` | Local state/value operations | Handled | [173](../../crates/crysirror-system-nnn/src/native_simple_wipe.rs#L173) |
| `kind` | `30 \| 31` | Local state/value operations | Handled | [178](../../crates/crysirror-system-nnn/src/native_simple_wipe.rs#L178) |
| `kind` | `32 \| 33 \| 78 \| 79` | Local state/value operations | Handled | [183](../../crates/crysirror-system-nnn/src/native_simple_wipe.rs#L183) |
| `kind` | `26 \| 27` | Local state/value operations | Handled | [189](../../crates/crysirror-system-nnn/src/native_simple_wipe.rs#L189) |
| `kind` | `36 \| 37 \| 54..=59` | Local state/value operations | Handled | [194](../../crates/crysirror-system-nnn/src/native_simple_wipe.rs#L194) |
| `kind` | `41` | Local state/value operations | Handled | [204](../../crates/crysirror-system-nnn/src/native_simple_wipe.rs#L204) |
| `kind` | `42..=47` | Local state/value operations | Handled | [207](../../crates/crysirror-system-nnn/src/native_simple_wipe.rs#L207) |
| `kind` | `60` | Local state/value operations | Handled | [213](../../crates/crysirror-system-nnn/src/native_simple_wipe.rs#L213) |
| `kind` | `61..=64` | Local state/value operations | Handled | [216](../../crates/crysirror-system-nnn/src/native_simple_wipe.rs#L216) |
| `kind` | `65..=68` | Local state/value operations | Handled | [221](../../crates/crysirror-system-nnn/src/native_simple_wipe.rs#L221) |
| `kind` | `88..=93` | Local state/value operations | Handled | [227](../../crates/crysirror-system-nnn/src/native_simple_wipe.rs#L227) |
| `kind` | `94 \| 95` | Local state/value operations | Handled | [237](../../crates/crysirror-system-nnn/src/native_simple_wipe.rs#L237) |
| `kind` | `1 \| 2` | Local state/value operations | Handled | [242](../../crates/crysirror-system-nnn/src/native_simple_wipe.rs#L242) |
| `kind` | `3` | Local state/value operations | Handled | [248](../../crates/crysirror-system-nnn/src/native_simple_wipe.rs#L248) |
| `kind` | `40` | Local state/value operations | Handled | [249](../../crates/crysirror-system-nnn/src/native_simple_wipe.rs#L249) |
| `kind` | `69..=72 \| 74..=77` | Local state/value operations | Handled | [250](../../crates/crysirror-system-nnn/src/native_simple_wipe.rs#L250) |
| `kind` | `22` | Local state/value operations | Handled | [255](../../crates/crysirror-system-nnn/src/native_simple_wipe.rs#L255) |
| `kind` | `23 \| 73` | Local state/value operations | Handled | [256](../../crates/crysirror-system-nnn/src/native_simple_wipe.rs#L256) |
| `kind` | `6 \| 7` | Local state/value operations | Handled | [259](../../crates/crysirror-system-nnn/src/native_simple_wipe.rs#L259) |
| `kind` | `8` | Local state/value operations | Handled | [260](../../crates/crysirror-system-nnn/src/native_simple_wipe.rs#L260) |
| `kind` | `9` | Local state/value operations | Handled | [261](../../crates/crysirror-system-nnn/src/native_simple_wipe.rs#L261) |
| `kind` | `4 \| 5` | Local state/value operations | Handled | [262](../../crates/crysirror-system-nnn/src/native_simple_wipe.rs#L262) |
| `kind` | `10..=17` | Local state/value operations | Handled | [265](../../crates/crysirror-system-nnn/src/native_simple_wipe.rs#L265) |
| `kind` | `18..=21` | Local state/value operations | Handled | [269](../../crates/crysirror-system-nnn/src/native_simple_wipe.rs#L269) |
| `kind` | `_` | Local state/value operations | Handled | [273](../../crates/crysirror-system-nnn/src/native_simple_wipe.rs#L273) |

### native_simple_wipe.rs / plan_with_random

```rust
fn plan_with_random(
    kind: i32,
    old: Arc<dyn ImageResource>,
    new: Arc<dyn ImageResource>,
    count: u32,
    duration: u32,
    random: &mut crysirror_image::temporal_pattern::PatternRng,
    tables: &mut Tables,
) -> Result<(ImageInput, Vec<ImageEffect>)>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under kind=24 \| 25 \| 48..=53` | `kind < 48` | Local state/value operations | Handled | [62](../../crates/crysirror-system-nnn/src/native_simple_wipe.rs#L62) |
| `kind` | `24 \| 25 \| 48..=53` | Local state/value operations | Handled | [45](../../crates/crysirror-system-nnn/src/native_simple_wipe.rs#L45) |
| `kind` | `38` | u64::from | Handled | [71](../../crates/crysirror-system-nnn/src/native_simple_wipe.rs#L71) |
| `kind` | `39` | Local state/value operations | Handled | [88](../../crates/crysirror-system-nnn/src/native_simple_wipe.rs#L88) |
| `kind` | `_` | Local state/value operations | Handled | [94](../../crates/crysirror-system-nnn/src/native_simple_wipe.rs#L94) |

### native_source_parameters.rs / data_offset

```rust
fn data_offset(id: i32) -> Option<usize>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `id` | `2` | Local state/value operations | Handled | [35](../../crates/crysirror-system-nnn/src/native_source_parameters.rs#L35) |
| `id` | `4` | Local state/value operations | Handled | [36](../../crates/crysirror-system-nnn/src/native_source_parameters.rs#L36) |
| `id` | `6` | Local state/value operations | Handled | [37](../../crates/crysirror-system-nnn/src/native_source_parameters.rs#L37) |
| `id` | `100` | Local state/value operations | Handled | [38](../../crates/crysirror-system-nnn/src/native_source_parameters.rs#L38) |
| `id` | `101` | Local state/value operations | Handled | [39](../../crates/crysirror-system-nnn/src/native_source_parameters.rs#L39) |
| `id` | `102` | Local state/value operations | Handled | [40](../../crates/crysirror-system-nnn/src/native_source_parameters.rs#L40) |
| `id` | `103` | Local state/value operations | Handled | [41](../../crates/crysirror-system-nnn/src/native_source_parameters.rs#L41) |
| `id` | `104` | Local state/value operations | Handled | [42](../../crates/crysirror-system-nnn/src/native_source_parameters.rs#L42) |
| `id` | `105` | Local state/value operations | Handled | [43](../../crates/crysirror-system-nnn/src/native_source_parameters.rs#L43) |
| `id` | `106` | Local state/value operations | Handled | [44](../../crates/crysirror-system-nnn/src/native_source_parameters.rs#L44) |
| `id` | `107` | Local state/value operations | Handled | [45](../../crates/crysirror-system-nnn/src/native_source_parameters.rs#L45) |
| `id` | `108` | Local state/value operations | Handled | [46](../../crates/crysirror-system-nnn/src/native_source_parameters.rs#L46) |
| `id` | `200` | Local state/value operations | Handled | [47](../../crates/crysirror-system-nnn/src/native_source_parameters.rs#L47) |
| `id` | `201` | Local state/value operations | Handled | [48](../../crates/crysirror-system-nnn/src/native_source_parameters.rs#L48) |
| `id` | `202` | Local state/value operations | Handled | [49](../../crates/crysirror-system-nnn/src/native_source_parameters.rs#L49) |
| `id` | `203` | Local state/value operations | Handled | [50](../../crates/crysirror-system-nnn/src/native_source_parameters.rs#L50) |
| `id` | `204` | Local state/value operations | Handled | [51](../../crates/crysirror-system-nnn/src/native_source_parameters.rs#L51) |
| `id` | `208` | Local state/value operations | Handled | [52](../../crates/crysirror-system-nnn/src/native_source_parameters.rs#L52) |
| `id` | `300..=315` | Local state/value operations | Handled | [53](../../crates/crysirror-system-nnn/src/native_source_parameters.rs#L53) |
| `id` | `400..=415` | Local state/value operations | Handled | [54](../../crates/crysirror-system-nnn/src/native_source_parameters.rs#L54) |
| `id` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [55](../../crates/crysirror-system-nnn/src/native_source_parameters.rs#L55) |

### native_source_parameters.rs / flag2_index

```rust
fn flag2_index(id: i32) -> Option<usize>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `id` | `109` | Local state/value operations | Handled | [60](../../crates/crysirror-system-nnn/src/native_source_parameters.rs#L60) |
| `id` | `209` | Local state/value operations | Handled | [61](../../crates/crysirror-system-nnn/src/native_source_parameters.rs#L61) |
| `id` | `210` | Local state/value operations | Handled | [62](../../crates/crysirror-system-nnn/src/native_source_parameters.rs#L62) |
| `id` | `211` | Local state/value operations | Handled | [63](../../crates/crysirror-system-nnn/src/native_source_parameters.rs#L63) |
| `id` | `212` | Local state/value operations | Handled | [64](../../crates/crysirror-system-nnn/src/native_source_parameters.rs#L64) |
| `id` | `213` | Local state/value operations | Handled | [65](../../crates/crysirror-system-nnn/src/native_source_parameters.rs#L65) |
| `id` | `214` | Local state/value operations | Handled | [66](../../crates/crysirror-system-nnn/src/native_source_parameters.rs#L66) |
| `id` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [67](../../crates/crysirror-system-nnn/src/native_source_parameters.rs#L67) |

### native_source_parameters.rs / Policy::get

```rust
fn get(&self, file: &NativeSystemFile, id: i32) -> Result<i32>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `id` | `103` | Local state/value operations | Handled | [83](../../crates/crysirror-system-nnn/src/native_source_parameters.rs#L83) |
| `id` | `203` | Local state/value operations | Handled | [89](../../crates/crysirror-system-nnn/src/native_source_parameters.rs#L89) |
| `id` | `104` | Local state/value operations | Handled | [95](../../crates/crysirror-system-nnn/src/native_source_parameters.rs#L95) |
| `id` | `204` | Local state/value operations | Handled | [100](../../crates/crysirror-system-nnn/src/native_source_parameters.rs#L100) |
| `id` | `_` | Local state/value operations | Handled | [107](../../crates/crysirror-system-nnn/src/native_source_parameters.rs#L107) |
| `condition` | `(500..572).contains(&id)` | crate::native_voice_policy::character_slider | Handled | [112](../../crates/crysirror-system-nnn/src/native_source_parameters.rs#L112) |

### native_source_parameters.rs / Policy::prepare

```rust
fn prepare(&self, file: &NativeSystemFile, id: i32, value: i32) -> Result<Write>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under id=300..=315` | `id - 300 + 1 == self.mouse_radio` | Action::MouseByConfig | Handled | [149](../../crates/crysirror-system-nnn/src/native_source_parameters.rs#L149) |
| `condition; under id=300..=315` | `id - 300 + 1 == self.cursor_radio` | Action::MessageCursorByConfig | Handled | [152](../../crates/crysirror-system-nnn/src/native_source_parameters.rs#L152) |
| `id` | `100` | Action::ChangeMusicVolume | Handled | [142](../../crates/crysirror-system-nnn/src/native_source_parameters.rs#L142) |
| `id` | `200 \| 208` | Action::SwitchMusic | Handled | [143](../../crates/crysirror-system-nnn/src/native_source_parameters.rs#L143) |
| `id` | `108` | Action::ChangeTotalVolume | Handled | [144](../../crates/crysirror-system-nnn/src/native_source_parameters.rs#L144) |
| `id` | `106` | Local state/value operations | Handled | [145](../../crates/crysirror-system-nnn/src/native_source_parameters.rs#L145) |
| `id` | `300..=315` | Action::MouseByConfig, Action::MessageCursorByConfig | Handled | [146](../../crates/crysirror-system-nnn/src/native_source_parameters.rs#L146) |
| `id` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [158](../../crates/crysirror-system-nnn/src/native_source_parameters.rs#L158) |
| `condition` | `(500..572).contains(&id)` | crate::native_voice_policy::set_character_slider | Handled | [160](../../crates/crysirror-system-nnn/src/native_source_parameters.rs#L160) |

### native_system_file/source_records.rs / NativeSystemFile::clear_source_record

```rust
fn clear_source_record(&mut self, code: u32) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `code` | `5` | Local state/value operations | Handled | [197](../../crates/crysirror-system-nnn/src/native_system_file/source_records.rs#L197) |
| `code` | `6` | Local state/value operations | Handled | [198](../../crates/crysirror-system-nnn/src/native_system_file/source_records.rs#L198) |
| `code` | `7` | Local state/value operations | Handled | [199](../../crates/crysirror-system-nnn/src/native_system_file/source_records.rs#L199) |
| `code` | `8` | Local state/value operations | Handled | [200](../../crates/crysirror-system-nnn/src/native_system_file/source_records.rs#L200) |
| `code` | `9` | Local state/value operations | Handled | [201](../../crates/crysirror-system-nnn/src/native_system_file/source_records.rs#L201) |
| `code` | `10` | Local state/value operations | Handled | [202](../../crates/crysirror-system-nnn/src/native_system_file/source_records.rs#L202) |
| `code` | `11` | Local state/value operations | Handled | [203](../../crates/crysirror-system-nnn/src/native_system_file/source_records.rs#L203) |
| `code` | `12` | Local state/value operations | Handled | [204](../../crates/crysirror-system-nnn/src/native_system_file/source_records.rs#L204) |
| `code` | `13` | Local state/value operations | Handled | [205](../../crates/crysirror-system-nnn/src/native_system_file/source_records.rs#L205) |
| `code` | `14` | Local state/value operations | Handled | [206](../../crates/crysirror-system-nnn/src/native_system_file/source_records.rs#L206) |
| `code` | `15` | Local state/value operations | Handled | [207](../../crates/crysirror-system-nnn/src/native_system_file/source_records.rs#L207) |
| `code` | `16` | Local state/value operations | Handled | [208](../../crates/crysirror-system-nnn/src/native_system_file/source_records.rs#L208) |
| `code` | `17` | Local state/value operations | Handled | [209](../../crates/crysirror-system-nnn/src/native_system_file/source_records.rs#L209) |
| `code` | `18` | Local state/value operations | Handled | [210](../../crates/crysirror-system-nnn/src/native_system_file/source_records.rs#L210) |
| `code` | `_` | Local state/value operations | Partial: NNN unsupported SystemFile clear record {code} | [211](../../crates/crysirror-system-nnn/src/native_system_file/source_records.rs#L211) |
| `condition` | `code == 17` | Local state/value operations | Handled | [214](../../crates/crysirror-system-nnn/src/native_system_file/source_records.rs#L214) |

### native_text.rs / styled_cells

```rust
fn styled_cells(
    raw: &[u8],
    single_byte_map: Option<&[char]>,
    inline: Option<&InlineProfile>,
) -> Result<(String, Vec<RubyRun>, Vec<Option<[u8; 4]>>)>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `command != 0x816d` | std::iter::repeat_n, char::from_u32, u32::from | Handled | [521](../../crates/crysirror-system-nnn/src/native_text.rs#L521) |
| `condition` | `command == 0x89fc` | Local state/value operations | Handled | [523](../../crates/crysirror-system-nnn/src/native_text.rs#L523) |
| `command` | `0x90a9` | Local state/value operations | Handled | [532](../../crates/crysirror-system-nnn/src/native_text.rs#L532) |
| `command` | `0x96bc` | Local state/value operations | Handled | [533](../../crates/crysirror-system-nnn/src/native_text.rs#L533) |
| `command` | `0x89d4` | Local state/value operations | Handled | [534](../../crates/crysirror-system-nnn/src/native_text.rs#L534) |
| `command` | `0x824f..=0x8258` | Local state/value operations | Handled | [535](../../crates/crysirror-system-nnn/src/native_text.rs#L535) |
| `command` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [536](../../crates/crysirror-system-nnn/src/native_text.rs#L536) |

### native_text.rs / InlineProfile::mugen2

```rust
fn mugen2(module: &[u8]) -> Result<Self>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `code == 0x8f49` | Local state/value operations | Handled | [134](../../crates/crysirror-system-nnn/src/native_text.rs#L134) |

### native_text.rs / NativeMessageState::layout_rows

```rust
fn layout_rows(
        mut next: Self,
        previous: usize,
        profile: &MessageProfile,
        metrics: &dyn TextMeasurement,
    ) -> Result<MessagePlan>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `next.mode == 0x22220001` | Local state/value operations | Handled | [951](../../crates/crysirror-system-nnn/src/native_text.rs#L951) |
| `condition` | `selector>0` | Local state/value operations | Handled | [976](../../crates/crysirror-system-nnn/src/native_text.rs#L976) |

### native_text.rs / NativeMessageState::prepare

```rust
fn prepare(
        &self,
        call: &NativeCall,
        profile: &MessageProfile,
        metrics: &dyn TextMeasurement,
    ) -> Result<MessagePlan>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `append && matches!(next.mode, 0x22220004 \| 0x22220005)` | Local state/value operations | Handled | [784](../../crates/crysirror-system-nnn/src/native_text.rs#L784) |
| `condition` | `next.mode == 0x22220001` | Local state/value operations | Handled | [830](../../crates/crysirror-system-nnn/src/native_text.rs#L830) |
| `condition` | `next.mode == 0x22220001` | Local state/value operations | Handled | [842](../../crates/crysirror-system-nnn/src/native_text.rs#L842) |

### native_text/window_visibility.rs / NativeTextBridge::project_window_visibility

```rust
fn project_window_visibility(
        &self,
        batch: &mut Value,
    ) -> Result<BTreeMap<u32, bool>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `node.get("op").and_then(Value::as_str) != Some("put")` | Local state/value operations | Handled | [29](../../crates/crysirror-system-nnn/src/native_text/window_visibility.rs#L29) |

### native_title.rs / extension_request

```rust
fn extension_request(
    table: &Names,
    configuration: &NativeConfiguration,
    index: usize,
    title: bool,
) -> Result<MenuRequest>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `mode > 0` | MenuRequest::Mode | Handled | [91](../../crates/crysirror-system-nnn/src/native_title.rs#L91) |

### native_title.rs / read_menu_button

```rust
fn read_menu_button(
    table: &Names,
    prefix: &str,
    configuration: &NativeConfiguration,
    old: Option<&NativeButtonSpec>,
    delta: [i32; 2],
) -> Result<NativeButtonSpec>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `mode > 0` | Local state/value operations | Handled | [306](../../crates/crysirror-system-nnn/src/native_title.rs#L306) |
| `condition` | `matches!(kind, 3 \| 4 \| 7 \| 8)` | Local state/value operations | Handled | [336](../../crates/crysirror-system-nnn/src/native_title.rs#L336) |
| `condition` | `matches!(kind, 3 \| 4 \| 7 \| 8)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [336](../../crates/crysirror-system-nnn/src/native_title.rs#L336) |

### native_title.rs / NativeButtonSpec::animation_sequence

```rust
fn animation_sequence(&self, mode: usize) -> Result<crysirror_scene::sprite_samples::SpriteSamples>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(kind,3\|4\|7\|8)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [565](../../crates/crysirror-system-nnn/src/native_title.rs#L565) |
| `condition` | `matches!(kind,2\|4\|6\|8)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [565](../../crates/crysirror-system-nnn/src/native_title.rs#L565) |

### native_title.rs / NativeButtonSpec::from_named_setup

```rust
fn from_named_setup(
        table: &Names,
        prefix: &str,
        configuration: &NativeConfiguration,
    ) -> Result<Self>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `pair[0] .get(..prefix.len()) .is_some_and(\|key\| key.eq_ignore_ascii_case(prefix.as_bytes()))` | Local state/value operations | Handled | [579](../../crates/crysirror-system-nnn/src/native_title.rs#L579) |

### native_title.rs / NativeButtonState::calcu

```rust
fn calcu(
        &mut self,
        spec: &NativeButtonSpec,
        hovered: bool,
        pressed: bool,
    ) -> NativeButtonSignal
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `self.mode == 2` | self.count.saturating_add | Handled | [493](../../crates/crysirror-system-nnn/src/native_title.rs#L493) |
| `(self.mode, hovered)` | `(0, true)` | Local state/value operations | Handled | [511](../../crates/crysirror-system-nnn/src/native_title.rs#L511) |
| `(self.mode, hovered)` | `(1, false)` | Local state/value operations | Handled | [517](../../crates/crysirror-system-nnn/src/native_title.rs#L517) |
| `(self.mode, hovered)` | `(1, true) if self.entered && self.count == spec.sound_waits[0] && (spec.sounds[2] > 0 \|\| spec.voices[2]>0)` | NativeButtonSignal::Sound | Handled | [523](../../crates/crysirror-system-nnn/src/native_title.rs#L523) |
| `(self.mode, hovered)` | `(0, false) if self.exited && self.count == spec.sound_waits[1] && (spec.sounds[3] > 0 \|\| spec.voices[3]>0)` | NativeButtonSignal::Sound | Handled | [524](../../crates/crysirror-system-nnn/src/native_title.rs#L524) |
| `(self.mode, hovered)` | `_` | Local state/value operations | Handled | [525](../../crates/crysirror-system-nnn/src/native_title.rs#L525) |

### native_title.rs / NativeButtonState::frame

```rust
fn frame(&self, spec: &NativeButtonSpec) -> ([u32; 2], u16)
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `spec.types[self.mode] >= 5` | u64::from | Handled | [543](../../crates/crysirror-system-nnn/src/native_title.rs#L543) |

### native_title.rs / NativeTitlePlayer::complete_frame

```rust
fn complete_frame(&mut self, host: &mut dyn HostServices) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under frame.final_job.as_mut().unwrap().poll()?=ImageProcessState::Ready(image)` | `let Err(error)=host.submit_engine(self.client,1,&serde_json::to_vec(&serde_json::json!({"layers":[{"op":"put","layer":{"id":self.layer,"rect":[0,0,w,h],"order":[4000,0],"paint":{"kind":"image","asset":path}}}]}))?)` | host.release_image_resource | Unsupported: Unconditional rejection | [1294](../../crates/crysirror-system-nnn/src/native_title.rs#L1294) |

### native_title.rs / NativeTitlePlayer::poll

```rust
fn poll(
        &mut self,
        host: &mut dyn HostServices,
        input: InputFrame,
        background: Arc<dyn ImageResource>,
        vm: &mut crate::adapter::NativeAdapter,
    ) -> Result<Option<usize>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `state.mode; under signal=NativeButtonSignal::Sound(_)` | `3` | Local state/value operations | Handled | [1432](../../crates/crysirror-system-nnn/src/native_title.rs#L1432) |
| `state.mode; under signal=NativeButtonSignal::Sound(_)` | `1` | Local state/value operations | Handled | [1432](../../crates/crysirror-system-nnn/src/native_title.rs#L1432) |
| `state.mode; under signal=NativeButtonSignal::Sound(_)` | `_` | Local state/value operations | Handled | [1432](../../crates/crysirror-system-nnn/src/native_title.rs#L1432) |

### native_title.rs / NativeTitlePlayer::poll_input

```rust
fn poll_input(&mut self,host:&mut dyn HostServices,input:InputFrame) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!child_cancelled && event["kind"]=="nnn_menu_cancel" && self.cancel_button.is_some() && event["widget"]["id"].as_u64()==self.widget_parent.map(u64::from)` | Local state/value operations | Handled | [1107](../../crates/crysirror-system-nnn/src/native_title.rs#L1107) |
| `condition` | `event["kind"] == "activate"` | u64::from, self.spec.buttons.len, self.activated.insert | Handled | [1111](../../crates/crysirror-system-nnn/src/native_title.rs#L1111) |
| `condition` | `let Some(id) = event["widget"]["id"].as_u64()` | u64::from, self.spec.buttons.len, self.activated.insert | Handled | [1112](../../crates/crysirror-system-nnn/src/native_title.rs#L1112) |

### native_title.rs / NativeTitlePlayer::present_return_frame

```rust
fn present_return_frame(
        &mut self,
        host: &mut dyn HostServices,
        image: Arc<dyn ImageResource>,
    ) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Err(error) = host.submit_engine(self.client, 1, &serde_json::to_vec(&serde_json::json!({"layers":[{"op":"put","layer":{"id":self.layer,"rect":[0,0,w,h],"order":[4000,0],"paint":{"kind":"image","asset":path}}}]}))?)` | host.release_image_resource | Unsupported: Unconditional rejection | [1569](../../crates/crysirror-system-nnn/src/native_title.rs#L1569) |

### native_title/minigame.rs / NativeTitlePlayer::calcu_mini_game

```rust
fn calcu_mini_game(
        &mut self,
        host: &mut dyn HostServices,
        vm: &mut crate::adapter::NativeAdapter,
        states: &mut [NativeButtonState],
        enabled: bool,
    ) -> Result<Option<crate::bridge::ButtonAudioEvent>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `states[index].mode; under signal=NativeButtonSignal::Sound(_)` | `3` | Local state/value operations | Handled | [459](../../crates/crysirror-system-nnn/src/native_title/minigame.rs#L459) |
| `states[index].mode; under signal=NativeButtonSignal::Sound(_)` | `1` | Local state/value operations | Handled | [460](../../crates/crysirror-system-nnn/src/native_title/minigame.rs#L460) |
| `states[index].mode; under signal=NativeButtonSignal::Sound(_)` | `_` | Local state/value operations | Handled | [461](../../crates/crysirror-system-nnn/src/native_title/minigame.rs#L461) |

### native_title/minigame.rs / Overlay::area_commands

```rust
fn area_commands(
        &mut self,
        base: u32,
        parent: Option<u32>,
        mode: i32,
        variables: &crate::expression::Variables,
        enabled: bool,
    ) -> Vec<serde_json::Value>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `area.mode != mode \|\| area.rect[2] <= 0 \|\| area.rect[3] <= 0` | Local state/value operations | Handled | [161](../../crates/crysirror-system-nnn/src/native_title/minigame.rs#L161) |

### native_title/minigame.rs / Setup::read

```rust
fn read(
        &self,
        open: &crate::resource_bridge::SourceOpener,
        cancel: &crysirror_tasks::Cancellation,
    ) -> Result<Overlay>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Some(kind) = table .integer(format!("miniGame{row}").as_bytes())? .filter(\|kind\| *kind > 0)` | Local state/value operations | Handled | [58](../../crates/crysirror-system-nnn/src/native_title/minigame.rs#L58) |
| `condition` | `let Some(name) = table.parameter(format!("miniGameCheckMode{i}").as_bytes())` | crate::native_mode::number | Handled | [69](../../crates/crysirror-system-nnn/src/native_title/minigame.rs#L69) |

### native_title/system_intro.rs / SystemIntro::new

```rust
fn new(
        table: &Names,
        count: usize,
        random: &mut crysirror_image::temporal_pattern::PatternRng,
    ) -> Result<Self>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `time > 0 && kind > 0` | Local state/value operations | Handled | [29](../../crates/crysirror-system-nnn/src/native_title/system_intro.rs#L29) |

### native_title/system_intro.rs / SystemIntro::view

```rust
fn view(&self, index: usize, size: [i32; 2]) -> Option<([i32; 2], [i32; 2], [i32; 2])>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `self.finished \|\| self.kind <= 0` | Local state/value operations | Handled | [64](../../crates/crysirror-system-nnn/src/native_title/system_intro.rs#L64) |
| `condition` | `self.kind > 6` | Local state/value operations | Handled | [69](../../crates/crysirror-system-nnn/src/native_title/system_intro.rs#L69) |
| `condition` | `matches!(self.kind, 3 \| 4 \| 6)` | Local state/value operations | Handled | [81](../../crates/crysirror-system-nnn/src/native_title/system_intro.rs#L81) |
| `condition` | `centered \|\| matches!(self.kind, 1 \| 3)` | Local state/value operations | Handled | [85](../../crates/crysirror-system-nnn/src/native_title/system_intro.rs#L85) |
| `condition` | `centered \|\| matches!(self.kind, 2 \| 4)` | Local state/value operations | Handled | [88](../../crates/crysirror-system-nnn/src/native_title/system_intro.rs#L88) |

### native_ui.rs / NativeSelectionBridge::poll_choice

```rust
fn poll_choice(&mut self, host: &mut dyn HostServices) -> Result<Option<usize>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `event["kind"] != "activate"` | Local state/value operations | Handled | [652](../../crates/crysirror-system-nnn/src/native_ui.rs#L652) |
