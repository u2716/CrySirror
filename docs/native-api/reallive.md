# RealLive developer reference

[All engines](README.md)

Mode: `adopt`.

## ABI and host contract

A native command is identified by **(modtype, module, opcode, overload)**,
not opcode alone. Arguments are native expressions: integer/string values,
memory references, special parameters and selection records. String banks and
byte-based CP932 measurements retain their native interpretation.

| Native area | CrySirror service / implementation | Return / completion |
| --- | --- | --- |
| Flow, strings, arrays and object properties | `vm/modules` | Native banks, store register and object identity. |
| Host commands | `adapter/commands.rs` and `adapter/extended.rs` | VM builtins delegate unresolved commands to the adapter; unsupported final routes error. |
| DC graphics / transitions | Canvas, retained Layers and shared image workers | Native DC IDs, manual refresh and transition timing remain native. |
| Message windows / choices / name entry | GlyphRun, GlyphReveal and retained UI | Native grids, ruby, read flags, hit/result indexes and waits. |
| Music / sound / voice / movie | Audio channels, cue/transport and Media | Native channel policy and long operations retain their continuation. |
| Files / saves | ClientStorage and native save codecs | Savepoint restore is distinct from arbitrary runtime snapshots. |

`Next` selects advancement, transfer or retained continuation. Host work and
long operations must complete before the native command retires. The module
routing table below documents which dispatcher owns each module; opcode and
overload predicates are preserved in the selector rules.

Limits: unknown command/overload pairs fail. Optical-disc checking uses the
explicit native-host policy in `adapter/commands.rs`. Device, font and pixel
limits remain host capabilities. Native save admission is checked independently
of parser/resource recognition.

Source: [VM module routing](../../crates/crysirror-reallive/src/vm/modules/mod.rs),
[adapter routing](../../crates/crysirror-reallive/src/adapter/commands.rs),
[source audit](../reallive-source-audit.json).
Validation: [native UI audit](../validation/native-ui-audit.md).

<!-- BEGIN GENERATED DISPATCH TABLES -->

## Opcode and API implementation tables

`Handled` means an executable source branch exists. It is not a claim that every argument/subselector or a complete game has been audited. Explicit gaps are shown per row.

| Status | Entries |
| --- | ---: |
| Empty handler | 22 |
| Handled | 1479 |
| Partial | 2 |
| Unsupported | 1 |

These tables cover the checked-in catalogues and explicit dispatch branches. An engine-wide upstream denominator is not inferred from these rows.

Module routing uses `(modtype, module)` IDs. Command tables keep opcode/overload pairs and parent module conditions separate; a bare opcode is scoped by its linked dispatcher.

## Adapter module/opcode pairs / sound_extended

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `module=20, opcode=0` | Record what the first bridge starts, for the queries below | Handled | — | [extended.rs:290](../../crates/crysirror-reallive/src/adapter/extended.rs#L290) |
| `module=20, opcode=1` | Record what the first bridge starts, for the queries below | Handled | — | [extended.rs:290](../../crates/crysirror-reallive/src/adapter/extended.rs#L290) |
| `module=20, opcode=1000` | DEBUG_MPLAY(_WAIT, _ONESHOT)_MILLISECOND / _SECOND / _SAMPLE | Handled | — | [extended.rs:363](../../crates/crysirror-reallive/src/adapter/extended.rs#L363) |
| `module=20, opcode=1001` | DEBUG_MPLAY(_WAIT, _ONESHOT)_MILLISECOND / _SECOND / _SAMPLE | Handled | — | [extended.rs:363](../../crates/crysirror-reallive/src/adapter/extended.rs#L363) |
| `module=20, opcode=1002` | DEBUG_MPLAY(_WAIT, _ONESHOT)_MILLISECOND / _SECOND / _SAMPLE | Handled | — | [extended.rs:363](../../crates/crysirror-reallive/src/adapter/extended.rs#L363) |
| `module=20, opcode=105` | bgmFadeOut(time = 1000) / bgmFadeOutEx (waits) | Handled | — | [extended.rs:345](../../crates/crysirror-reallive/src/adapter/extended.rs#L345) |
| `module=20, opcode=106` | bgmFadeOut(time = 1000) / bgmFadeOutEx (waits) | Handled | — | [extended.rs:345](../../crates/crysirror-reallive/src/adapter/extended.rs#L345) |
| `module=20, opcode=107` | bgmStatus: 0 stopped, 1 playing, 2 fading out | Handled | — | [extended.rs:308](../../crates/crysirror-reallive/src/adapter/extended.rs#L308) |
| `module=20, opcode=11` | Audio playback/control (sound_extended) | Handled | — | [extended.rs:323](../../crates/crysirror-reallive/src/adapter/extended.rs#L323) |
| `module=20, opcode=12` | Audio playback/control (sound_extended) | Handled | — | [extended.rs:324](../../crates/crysirror-reallive/src/adapter/extended.rs#L324) |
| `module=20, opcode=13` | Audio playback/control (sound_extended) | Handled | — | [extended.rs:330](../../crates/crysirror-reallive/src/adapter/extended.rs#L330) |
| `module=20, opcode=14` | Audio playback/control (sound_extended) | Handled | — | [extended.rs:330](../../crates/crysirror-reallive/src/adapter/extended.rs#L330) |
| `module=20, opcode=15` | MCHANGELOOP / MCHANGEONESHOT | Handled | — | [extended.rs:337](../../crates/crysirror-reallive/src/adapter/extended.rs#L337) |
| `module=20, opcode=16` | MCHANGELOOP / MCHANGEONESHOT | Handled | — | [extended.rs:337](../../crates/crysirror-reallive/src/adapter/extended.rs#L337) |
| `module=20, opcode=2` | Record what the first bridge starts, for the queries below | Handled | — | [extended.rs:290](../../crates/crysirror-reallive/src/adapter/extended.rs#L290) |
| `module=20, opcode=200` | bgmTimer: milliseconds since the track started | Handled | — | [extended.rs:354](../../crates/crysirror-reallive/src/adapter/extended.rs#L354) |
| `module=20, opcode=2000` | DEBUG_MPLAY(_WAIT, _ONESHOT)_MILLISECOND / _SECOND / _SAMPLE | Handled | — | [extended.rs:363](../../crates/crysirror-reallive/src/adapter/extended.rs#L363) |
| `module=20, opcode=2001` | DEBUG_MPLAY(_WAIT, _ONESHOT)_MILLISECOND / _SECOND / _SAMPLE | Handled | — | [extended.rs:363](../../crates/crysirror-reallive/src/adapter/extended.rs#L363) |
| `module=20, opcode=2002` | DEBUG_MPLAY(_WAIT, _ONESHOT)_MILLISECOND / _SECOND / _SAMPLE | Handled | — | [extended.rs:363](../../crates/crysirror-reallive/src/adapter/extended.rs#L363) |
| `module=20, opcode=300` | MCHANGE(name): another track, looping as the current one does | Handled | — | [extended.rs:356](../../crates/crysirror-reallive/src/adapter/extended.rs#L356) |
| `module=20, opcode=3000` | DEBUG_MPLAY(_WAIT, _ONESHOT)_MILLISECOND / _SECOND / _SAMPLE | Handled | — | [extended.rs:363](../../crates/crysirror-reallive/src/adapter/extended.rs#L363) |
| `module=20, opcode=3001` | DEBUG_MPLAY(_WAIT, _ONESHOT)_MILLISECOND / _SECOND / _SAMPLE | Handled | — | [extended.rs:363](../../crates/crysirror-reallive/src/adapter/extended.rs#L363) |
| `module=20, opcode=3002` | DEBUG_MPLAY(_WAIT, _ONESHOT)_MILLISECOND / _SECOND / _SAMPLE | Handled | — | [extended.rs:363](../../crates/crysirror-reallive/src/adapter/extended.rs#L363) |
| `module=20, opcode=5` | bgmStop (both forms) | Handled | — | [extended.rs:303](../../crates/crysirror-reallive/src/adapter/extended.rs#L303) |
| `module=20, opcode=6` | bgmStop (both forms) | Handled | — | [extended.rs:303](../../crates/crysirror-reallive/src/adapter/extended.rs#L303) |
| `module=20, opcode=7` | bgmStatus: 0 stopped, 1 playing, 2 fading out | Handled | — | [extended.rs:308](../../crates/crysirror-reallive/src/adapter/extended.rs#L308) |
| `module=20, opcode=8` | bgmRewind: the track again from the start | Handled | — | [extended.rs:317](../../crates/crysirror-reallive/src/adapter/extended.rs#L317) |
| `module=21, opcode=0` | Audio playback/control (sound_extended) | Handled | — | [extended.rs:295](../../crates/crysirror-reallive/src/adapter/extended.rs#L295) |
| `module=21, opcode=1` | Audio playback/control (sound_extended) | Handled | — | [extended.rs:295](../../crates/crysirror-reallive/src/adapter/extended.rs#L295) |
| `module=21, opcode=1000` | PCMBUF_LOAD / PCMBUF_FREE / PCMBUF_FREEALL: preloading; files are read when played (as in the original) | Empty handler | Original no-op status not established | [extended.rs:489](../../crates/crysirror-reallive/src/adapter/extended.rs#L489) |
| `module=21, opcode=1001` | PCMBUF_LOAD / PCMBUF_FREE / PCMBUF_FREEALL: preloading; files are read when played (as in the original) | Empty handler | Original no-op status not established | [extended.rs:489](../../crates/crysirror-reallive/src/adapter/extended.rs#L489) |
| `module=21, opcode=1002` | PCMBUF_LOAD / PCMBUF_FREE / PCMBUF_FREEALL: preloading; files are read when played (as in the original) | Empty handler | Original no-op status not established | [extended.rs:489](../../crates/crysirror-reallive/src/adapter/extended.rs#L489) |
| `module=21, opcode=105` | wavFadeOut(channel[, time = 1000]) / wavStopEx | Handled | — | [extended.rs:491](../../crates/crysirror-reallive/src/adapter/extended.rs#L491) |
| `module=21, opcode=106` | wavFadeOut(channel[, time = 1000]) / wavStopEx | Handled | — | [extended.rs:491](../../crates/crysirror-reallive/src/adapter/extended.rs#L491) |
| `module=21, opcode=107` | PCMFADECHECK(channel) | Handled | — | [extended.rs:498](../../crates/crysirror-reallive/src/adapter/extended.rs#L498) |
| `module=21, opcode=11` | Audio playback/control (sound_extended) | Handled | — | [extended.rs:384](../../crates/crysirror-reallive/src/adapter/extended.rs#L384) |
| `module=21, opcode=12` | Audio playback/control (sound_extended) | Handled | — | [extended.rs:388](../../crates/crysirror-reallive/src/adapter/extended.rs#L388) |
| `module=21, opcode=13` | Audio playback/control (sound_extended) | Handled | — | [extended.rs:395](../../crates/crysirror-reallive/src/adapter/extended.rs#L395) |
| `module=21, opcode=14` | Audio playback/control (sound_extended) | Handled | — | [extended.rs:395](../../crates/crysirror-reallive/src/adapter/extended.rs#L395) |
| `module=21, opcode=15` | PCMCHANGELOOP / PCMCHANGEONESHOT(channel) | Handled | — | [extended.rs:403](../../crates/crysirror-reallive/src/adapter/extended.rs#L403) |
| `module=21, opcode=16` | PCMCHANGELOOP / PCMCHANGEONESHOT(channel) | Handled | — | [extended.rs:403](../../crates/crysirror-reallive/src/adapter/extended.rs#L403) |
| `module=21, opcode=2` | Audio playback/control (sound_extended) | Handled | — | [extended.rs:295](../../crates/crysirror-reallive/src/adapter/extended.rs#L295) |
| `module=21, opcode=201` | PCMPLAYWAITKEY(name, channel) / PCMWAITKEY(channel) | Handled | — | [extended.rs:506](../../crates/crysirror-reallive/src/adapter/extended.rs#L506) |
| `module=21, opcode=203` | Audio playback/control (sound_extended) | Handled | — | [extended.rs:516](../../crates/crysirror-reallive/src/adapter/extended.rs#L516) |
| `module=21, opcode=21` | PCMSTOPALLMAIN / PCMSTOPALLEXTRA: the first / second half | Handled | — | [extended.rs:415](../../crates/crysirror-reallive/src/adapter/extended.rs#L415) |
| `module=21, opcode=22` | PCMSTOPALLMAIN / PCMSTOPALLEXTRA: the first / second half | Handled | — | [extended.rs:415](../../crates/crysirror-reallive/src/adapter/extended.rs#L415) |
| `module=21, opcode=32` | PCMVOLSETALL(volume) / PCMVOLMAXALL(time) / PCMVOLMINALL(time) | Handled | — | [extended.rs:423](../../crates/crysirror-reallive/src/adapter/extended.rs#L423) |
| `module=21, opcode=33` | PCMVOLSETALL(volume) / PCMVOLMAXALL(time) / PCMVOLMINALL(time) | Handled | — | [extended.rs:423](../../crates/crysirror-reallive/src/adapter/extended.rs#L423) |
| `module=21, opcode=34` | PCMVOLSETALL(volume) / PCMVOLMAXALL(time) / PCMVOLMINALL(time) | Handled | — | [extended.rs:423](../../crates/crysirror-reallive/src/adapter/extended.rs#L423) |
| `module=21, opcode=40` | PCMEVENT_LOOP / _RANDOM / _ONESHOT(event, (name, [weight,] min, max)...) | Handled | — | [extended.rs:435](../../crates/crysirror-reallive/src/adapter/extended.rs#L435) |
| `module=21, opcode=41` | PCMEVENT_LOOP / _RANDOM / _ONESHOT(event, (name, [weight,] min, max)...) | Handled | — | [extended.rs:435](../../crates/crysirror-reallive/src/adapter/extended.rs#L435) |
| `module=21, opcode=42` | PCMEVENT_LOOP / _RANDOM / _ONESHOT(event, (name, [weight,] min, max)...) | Handled | — | [extended.rs:435](../../crates/crysirror-reallive/src/adapter/extended.rs#L435) |
| `module=21, opcode=50` | PCMEVENT_STOP(event, stop sound) / PCMEVENT_STOPALL(stop sound) | Handled | — | [extended.rs:466](../../crates/crysirror-reallive/src/adapter/extended.rs#L466) |
| `module=21, opcode=51` | PCMEVENT_STOP(event, stop sound) / PCMEVENT_STOPALL(stop sound) | Handled | — | [extended.rs:466](../../crates/crysirror-reallive/src/adapter/extended.rs#L466) |
| `module=21, opcode=52` | Audio playback/control (sound_extended) | Handled | — | [extended.rs:479](../../crates/crysirror-reallive/src/adapter/extended.rs#L479) |
| `module=21, opcode=53` | Audio playback/control (sound_extended) | Handled | — | [extended.rs:483](../../crates/crysirror-reallive/src/adapter/extended.rs#L483) |
| `module=21, opcode=8` | wavRewind(channel) | Handled | — | [extended.rs:375](../../crates/crysirror-reallive/src/adapter/extended.rs#L375) |
| `module=22, opcode=11` | SEVOLGET / SEVOLSET(volume) / SEVOLMAX(time) / SEVOLMIN(time) | Handled | — | [extended.rs:521](../../crates/crysirror-reallive/src/adapter/extended.rs#L521) |
| `module=22, opcode=12` | Audio playback/control (sound_extended) | Handled | — | [extended.rs:522](../../crates/crysirror-reallive/src/adapter/extended.rs#L522) |
| `module=22, opcode=13` | Audio playback/control (sound_extended) | Handled | — | [extended.rs:522](../../crates/crysirror-reallive/src/adapter/extended.rs#L522) |
| `module=22, opcode=14` | Audio playback/control (sound_extended) | Handled | — | [extended.rs:522](../../crates/crysirror-reallive/src/adapter/extended.rs#L522) |
| `module=23, opcode=10` | koePlayExC / koeDoPlayExC: a click also ends the wait | Handled | — | [extended.rs:547](../../crates/crysirror-reallive/src/adapter/extended.rs#L547) |
| `module=23, opcode=100` | SET_KOEFILEMODE(mode): which voice archives to read; any is read | Handled | — | [extended.rs:567](../../crates/crysirror-reallive/src/adapter/extended.rs#L567) |
| `module=23, opcode=101` | KOE_REPLAY_CLEAR: drop the voice-replay markers | Handled | — | [extended.rs:565](../../crates/crysirror-reallive/src/adapter/extended.rs#L565) |
| `module=23, opcode=11` | Audio playback/control (sound_extended) | Handled | — | [extended.rs:533](../../crates/crysirror-reallive/src/adapter/extended.rs#L533) |
| `module=23, opcode=12` | Audio playback/control (sound_extended) | Handled | — | [extended.rs:534](../../crates/crysirror-reallive/src/adapter/extended.rs#L534) |
| `module=23, opcode=13` | Audio playback/control (sound_extended) | Handled | — | [extended.rs:540](../../crates/crysirror-reallive/src/adapter/extended.rs#L540) |
| `module=23, opcode=14` | Audio playback/control (sound_extended) | Handled | — | [extended.rs:540](../../crates/crysirror-reallive/src/adapter/extended.rs#L540) |
| `module=23, opcode=15` | KOE_DONT_MEMORY / KOEPLAYWAIT_ / KOEPLAYWAITKEY_ and the EX forms: a voice not remembered for replay or the backlog | Handled | — | [extended.rs:553](../../crates/crysirror-reallive/src/adapter/extended.rs#L553) |
| `module=23, opcode=16` | KOE_DONT_MEMORY / KOEPLAYWAIT_ / KOEPLAYWAITKEY_ and the EX forms: a voice not remembered for replay or the backlog | Handled | — | [extended.rs:553](../../crates/crysirror-reallive/src/adapter/extended.rs#L553) |
| `module=23, opcode=17` | KOE_DONT_MEMORY / KOEPLAYWAIT_ / KOEPLAYWAITKEY_ and the EX forms: a voice not remembered for replay or the backlog | Handled | — | [extended.rs:553](../../crates/crysirror-reallive/src/adapter/extended.rs#L553) |
| `module=23, opcode=18` | KOE_DONT_MEMORY / KOEPLAYWAIT_ / KOEPLAYWAITKEY_ and the EX forms: a voice not remembered for replay or the backlog | Handled | — | [extended.rs:553](../../crates/crysirror-reallive/src/adapter/extended.rs#L553) |
| `module=23, opcode=19` | KOE_DONT_MEMORY / KOEPLAYWAIT_ / KOEPLAYWAITKEY_ and the EX forms: a voice not remembered for replay or the backlog | Handled | — | [extended.rs:553](../../crates/crysirror-reallive/src/adapter/extended.rs#L553) |
| `module=23, opcode=20` | KOE_DONT_MEMORY / KOEPLAYWAIT_ / KOEPLAYWAITKEY_ and the EX forms: a voice not remembered for replay or the backlog | Handled | — | [extended.rs:553](../../crates/crysirror-reallive/src/adapter/extended.rs#L553) |
| `module=23, opcode=7` | koePlayExC / koeDoPlayExC: a click also ends the wait | Handled | — | [extended.rs:547](../../crates/crysirror-reallive/src/adapter/extended.rs#L547) |

## Module commands / adapter/commands / command / (c.op.modtype, c.op.module)=(1, 30)

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x00` | Execute command branch | Handled | — | [commands.rs:45](../../crates/crysirror-reallive/src/adapter/commands.rs#L45) |
| `0x03` | Update store | Handled | — | [commands.rs:46](../../crates/crysirror-reallive/src/adapter/commands.rs#L46) |
| `0x14` | Auto / semi-auto show every draw; manual shows refreshes only | Handled | — | [commands.rs:59](../../crates/crysirror-reallive/src/adapter/commands.rs#L59) |
| `0x15` | Auto / semi-auto show every draw; manual shows refreshes only | Handled | — | [commands.rs:59](../../crates/crysirror-reallive/src/adapter/commands.rs#L59) |
| `0x16` | Set manual | Handled | — | [commands.rs:60](../../crates/crysirror-reallive/src/adapter/commands.rs#L60) |
| `0x1F` | Query | Handled | — | [commands.rs:50](../../crates/crysirror-reallive/src/adapter/commands.rs#L50) |

## Module commands / adapter/commands / dll

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x0A` | Execute dll branch | Handled | — | [commands.rs:1810](../../crates/crysirror-reallive/src/adapter/commands.rs#L1810) |
| `0x0B` | Execute dll branch | Handled | — | [commands.rs:1815](../../crates/crysirror-reallive/src/adapter/commands.rs#L1815) |
| `0x0C` | Update sys / dlls, store | Handled | — | [commands.rs:1819](../../crates/crysirror-reallive/src/adapter/commands.rs#L1819) |

## Module commands / adapter/commands / graphics

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x0F` | Canvas | Handled | — | [commands.rs:217](../../crates/crysirror-reallive/src/adapter/commands.rs#L217) |
| `0x10` | freeDC(dc): DC 0 stays, DC 1 is emptied at screen size, others are released (upstream `Graphics::free_dc`) | Handled | — | [commands.rs:398](../../crates/crysirror-reallive/src/adapter/commands.rs#L398) |
| `0x1F` | Canvas | Handled | — | [commands.rs:226](../../crates/crysirror-reallive/src/adapter/commands.rs#L226) |
| `0x20` | shake(spec): `#SHAKE.spec` (else `#SHAKE.000`), waited for | Handled | — | [commands.rs:528](../../crates/crysirror-reallive/src/adapter/commands.rs#L528) |
| `0x32` | Resource | Handled | — | [commands.rs:231](../../crates/crysirror-reallive/src/adapter/commands.rs#L231) |
| `0x33` | Resource | Handled | — | [commands.rs:231](../../crates/crysirror-reallive/src/adapter/commands.rs#L231) |
| `0x46` | Resource | Handled | — | [commands.rs:231](../../crates/crysirror-reallive/src/adapter/commands.rs#L231) |
| `0x47` | Resource | Handled | — | [commands.rs:231](../../crates/crysirror-reallive/src/adapter/commands.rs#L231) |
| `0x48` | Hide text | Handled | — | [commands.rs:291](../../crates/crysirror-reallive/src/adapter/commands.rs#L291) |
| `0x49` | Hide text | Handled | — | [commands.rs:291](../../crates/crysirror-reallive/src/adapter/commands.rs#L291) |
| `0x4A` | Hide text | Handled | — | [commands.rs:291](../../crates/crysirror-reallive/src/adapter/commands.rs#L291) |
| `0x4B` | grpMulti(file \\| dc, effect..., compositors...) — upstream grp 75/77 | Handled | — | [commands.rs:461](../../crates/crysirror-reallive/src/adapter/commands.rs#L461) |
| `0x4C` | Hide text | Handled | — | [commands.rs:291](../../crates/crysirror-reallive/src/adapter/commands.rs#L291) |
| `0x4D` | grpMulti(file \\| dc, effect..., compositors...) — upstream grp 75/77 | Handled | — | [commands.rs:461](../../crates/crysirror-reallive/src/adapter/commands.rs#L461) |
| `0x64` | Extent | Handled | — | [commands.rs:366](../../crates/crysirror-reallive/src/adapter/commands.rs#L366) |
| `0x65` | Extent | Handled | — | [commands.rs:366](../../crates/crysirror-reallive/src/adapter/commands.rs#L366) |
| `0x66` | Extent | Handled | — | [commands.rs:366](../../crates/crysirror-reallive/src/adapter/commands.rs#L366) |
| `0xC8` | grpOutline / grpFill(([area,] dc, r, g, b[, alpha])), upstream grp 200/201 | Handled | — | [commands.rs:547](../../crates/crysirror-reallive/src/adapter/commands.rs#L547) |
| `0xC9` | grpOutline / grpFill(([area,] dc, r, g, b[, alpha])), upstream grp 200/201 | Handled | — | [commands.rs:547](../../crates/crysirror-reallive/src/adapter/commands.rs#L547) |
| `0x12C` | Execute graphics branch | Handled | — | [commands.rs:427](../../crates/crysirror-reallive/src/adapter/commands.rs#L427) |
| `0x12D` | Execute graphics branch | Handled | — | [commands.rs:427](../../crates/crysirror-reallive/src/adapter/commands.rs#L427) |
| `0x12E` | Execute graphics branch | Handled | — | [commands.rs:428](../../crates/crysirror-reallive/src/adapter/commands.rs#L428) |
| `0x12F` | Extent | Handled | — | [commands.rs:425](../../crates/crysirror-reallive/src/adapter/commands.rs#L425) |
| `0x191` | grpStretchBlt / grpMaskStretchBlt(src rect, src, dst rect, dst[, alpha]) | Handled | — | [commands.rs:408](../../crates/crysirror-reallive/src/adapter/commands.rs#L408) |
| `0x192` | grpZoom(from, to, src, dest, time) | Handled | — | [commands.rs:538](../../crates/crysirror-reallive/src/adapter/commands.rs#L538) |
| `0x193` | Canvas | Handled | — | [commands.rs:583](../../crates/crysirror-reallive/src/adapter/commands.rs#L583) |
| `0x199` | grpStretchBlt / grpMaskStretchBlt(src rect, src, dst rect, dst[, alpha]) | Handled | — | [commands.rs:408](../../crates/crysirror-reallive/src/adapter/commands.rs#L408) |

## Module commands / adapter/commands / haikei

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x0A` | Execute haikei branch | Handled | — | [commands.rs:1516](../../crates/crysirror-reallive/src/adapter/commands.rs#L1516) |
| `0x0B` | Execute haikei branch | Handled | — | [commands.rs:1516](../../crates/crysirror-reallive/src/adapter/commands.rs#L1516) |
| `0x14` | Execute haikei branch | Handled | — | [commands.rs:1516](../../crates/crysirror-reallive/src/adapter/commands.rs#L1516) |
| `0x15` | Execute haikei branch | Handled | — | [commands.rs:1516](../../crates/crysirror-reallive/src/adapter/commands.rs#L1516) |
| `0x64` | bgrMulti(file[, sel], (file[, x, y])...): overload 1 carries the `#SEL` number before the overlays | Handled | — | [commands.rs:1533](../../crates/crysirror-reallive/src/adapter/commands.rs#L1533) |
| `0x65` | Execute haikei branch | Handled | — | [commands.rs:1516](../../crates/crysirror-reallive/src/adapter/commands.rs#L1516) |

## Module commands / adapter/commands / message

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x03 par` | br: the next line (upstream `hard_break`); par: a new line only when the current one has text (upstream `paragraph`) | Handled | — | [commands.rs:953](../../crates/crysirror-reallive/src/adapter/commands.rs#L953) |
| `0x0F spause3` | spause3 | Handled | — | [commands.rs:909](../../crates/crysirror-reallive/src/adapter/commands.rs#L909) |
| `0x11 pause` | Wait for input | Handled | — | [commands.rs:909](../../crates/crysirror-reallive/src/adapter/commands.rs#L909) |
| `0x64 SetFontColour` | Set Font Colour | Handled | — | [commands.rs:918](../../crates/crysirror-reallive/src/adapter/commands.rs#L918) |
| `0x65 FontSize` | FontSize([size]): no size returns to the window's default | Handled | — | [commands.rs:920](../../crates/crysirror-reallive/src/adapter/commands.rs#L920) |
| `0x66 TextWindow` | Text Window | Handled | — | [commands.rs:904](../../crates/crysirror-reallive/src/adapter/commands.rs#L904) |
| `0x67 FastText` | Fast Text | Handled | — | [commands.rs:916](../../crates/crysirror-reallive/src/adapter/commands.rs#L916) |
| `0x68 NormalText` | Normal Text | Handled | — | [commands.rs:917](../../crates/crysirror-reallive/src/adapter/commands.rs#L917) |
| `0x69 FontColour` | Font Colour | Handled | — | [commands.rs:918](../../crates/crysirror-reallive/src/adapter/commands.rs#L918) |
| `0x97 msgHide` | Upstream msg: 151 closes the active window (frame included), 152 clears its text, 161 closes every window, 162 clears a… | Handled | — | [commands.rs:932](../../crates/crysirror-reallive/src/adapter/commands.rs#L932) |
| `0x98 msgClear` | msg Clear | Handled | — | [commands.rs:936](../../crates/crysirror-reallive/src/adapter/commands.rs#L936) |
| `0xA1 msgHideAll` | msg Hide All | Handled | — | [commands.rs:940](../../crates/crysirror-reallive/src/adapter/commands.rs#L940) |
| `0xA2 msgClearAll` | msg Clear All | Handled | — | [commands.rs:944](../../crates/crysirror-reallive/src/adapter/commands.rs#L944) |
| `0xC9 br` | br: the next line (upstream `hard_break`); par: a new line only when the current one has text (upstream `paragraph`) | Handled | — | [commands.rs:953](../../crates/crysirror-reallive/src/adapter/commands.rs#L953) |
| `0xCA par2` | br: the next line (upstream `hard_break`); par: a new line only when the current one has text (upstream `paragraph`) | Handled | — | [commands.rs:953](../../crates/crysirror-reallive/src/adapter/commands.rs#L953) |
| `0xCD spause` | spause | Handled | — | [commands.rs:909](../../crates/crysirror-reallive/src/adapter/commands.rs#L909) |
| `0xCE spause2` | spause2 | Handled | — | [commands.rs:909](../../crates/crysirror-reallive/src/adapter/commands.rs#L909) |
| `0xCF pause_all` | pause all | Handled | — | [commands.rs:909](../../crates/crysirror-reallive/src/adapter/commands.rs#L909) |
| `0xD2 page` | page | Handled | — | [commands.rs:909](../../crates/crysirror-reallive/src/adapter/commands.rs#L909) |

## Module commands / adapter/commands / movie

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x00` | Movie playback (movie) | Handled | — | [commands.rs:615](../../crates/crysirror-reallive/src/adapter/commands.rs#L615) |
| `0x01` | Movie playback (movie) | Handled | — | [commands.rs:615](../../crates/crysirror-reallive/src/adapter/commands.rs#L615) |
| `0x02` | Movie playback (movie) | Handled | — | [commands.rs:615](../../crates/crysirror-reallive/src/adapter/commands.rs#L615) |
| `0x03` | Movie playback (movie) | Handled | — | [commands.rs:646](../../crates/crysirror-reallive/src/adapter/commands.rs#L646) |
| `0x04` | Movie playback (movie) | Handled | — | [commands.rs:651](../../crates/crysirror-reallive/src/adapter/commands.rs#L651) |
| `0x05` | Movie playback (movie) | Handled | — | [commands.rs:657](../../crates/crysirror-reallive/src/adapter/commands.rs#L657) |
| `0x14` | Movie playback (movie) | Handled | — | [commands.rs:615](../../crates/crysirror-reallive/src/adapter/commands.rs#L615) |
| `0x15` | Movie playback (movie) | Handled | — | [commands.rs:646](../../crates/crysirror-reallive/src/adapter/commands.rs#L646) |

## Module commands / adapter/commands / system

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x00` | System commands (system) | Handled | — | [commands.rs:92](../../crates/crysirror-reallive/src/adapter/commands.rs#L92) |
| `0x02` | System commands (system) | Handled | — | [commands.rs:96](../../crates/crysirror-reallive/src/adapter/commands.rs#L96) |
| `0x64` | System commands (system) | Handled | — | [commands.rs:100](../../crates/crysirror-reallive/src/adapter/commands.rs#L100) |
| `0x65` | System commands (system) | Handled | — | [commands.rs:100](../../crates/crysirror-reallive/src/adapter/commands.rs#L100) |
| `0x6A` | System commands (system) | Handled | — | [commands.rs:132](../../crates/crysirror-reallive/src/adapter/commands.rs#L132) |
| `0x6E` | System commands (system) | Handled | — | [commands.rs:106](../../crates/crysirror-reallive/src/adapter/commands.rs#L106) |
| `0x6F` | System commands (system) | Handled | — | [commands.rs:111](../../crates/crysirror-reallive/src/adapter/commands.rs#L111) |
| `0x70` | System commands (system) | Handled | — | [commands.rs:111](../../crates/crysirror-reallive/src/adapter/commands.rs#L111) |
| `0x71` | System commands (system) | Handled | — | [commands.rs:111](../../crates/crysirror-reallive/src/adapter/commands.rs#L111) |
| `0x72` | System commands (system) | Handled | — | [commands.rs:111](../../crates/crysirror-reallive/src/adapter/commands.rs#L111) |
| `0x78` | System commands (system) | Handled | — | [commands.rs:106](../../crates/crysirror-reallive/src/adapter/commands.rs#L106) |
| `0x79` | System commands (system) | Handled | — | [commands.rs:111](../../crates/crysirror-reallive/src/adapter/commands.rs#L111) |
| `0x7A` | System commands (system) | Handled | — | [commands.rs:111](../../crates/crysirror-reallive/src/adapter/commands.rs#L111) |
| `0x7B` | System commands (system) | Handled | — | [commands.rs:111](../../crates/crysirror-reallive/src/adapter/commands.rs#L111) |
| `0x7C` | System commands (system) | Handled | — | [commands.rs:111](../../crates/crysirror-reallive/src/adapter/commands.rs#L111) |
| `0x82` | System commands (system) | Handled | — | [commands.rs:133](../../crates/crysirror-reallive/src/adapter/commands.rs#L133) |
| `0x83` | System commands (system) | Handled | — | [commands.rs:134](../../crates/crysirror-reallive/src/adapter/commands.rs#L134) |
| `0x85` | System commands (system) | Handled | — | [commands.rs:141](../../crates/crysirror-reallive/src/adapter/commands.rs#L141) |
| `0x8A` | System commands (system) | Handled | — | [commands.rs:141](../../crates/crysirror-reallive/src/adapter/commands.rs#L141) |
| `0xCA` | System commands (system) | Handled | — | [commands.rs:141](../../crates/crysirror-reallive/src/adapter/commands.rs#L141) |
| `0xCC` | System commands (system) | Handled | — | [commands.rs:158](../../crates/crysirror-reallive/src/adapter/commands.rs#L158) |
| `0xCD` | System commands (system) | Handled | — | [commands.rs:158](../../crates/crysirror-reallive/src/adapter/commands.rs#L158) |
| `0xD2` | System commands (system) | Handled | — | [commands.rs:165](../../crates/crysirror-reallive/src/adapter/commands.rs#L165) |
| `0xD3` | System commands (system) | Handled | — | [commands.rs:165](../../crates/crysirror-reallive/src/adapter/commands.rs#L165) |
| `0xD7` | System commands (system) | Handled | — | [commands.rs:166](../../crates/crysirror-reallive/src/adapter/commands.rs#L166) |
| `0xD8` | System commands (system) | Handled | — | [commands.rs:167](../../crates/crysirror-reallive/src/adapter/commands.rs#L167) |
| `0x140` | System commands (system) | Handled | — | [commands.rs:168](../../crates/crysirror-reallive/src/adapter/commands.rs#L168) |
| `0x4BB` | System commands (system) | Handled | — | [commands.rs:185](../../crates/crysirror-reallive/src/adapter/commands.rs#L185) |
| `0x4BC` | System commands (system) | Handled | — | [commands.rs:185](../../crates/crysirror-reallive/src/adapter/commands.rs#L185) |
| `0x4BD` | System commands (system) | Handled | — | [commands.rs:185](../../crates/crysirror-reallive/src/adapter/commands.rs#L185) |
| `0x514` | System commands (system) | Handled | — | [commands.rs:174](../../crates/crysirror-reallive/src/adapter/commands.rs#L174) |
| `0x515` | System commands (system) | Handled | — | [commands.rs:179](../../crates/crysirror-reallive/src/adapter/commands.rs#L179) |
| `0x516` | System commands (system) | Handled | — | [commands.rs:184](../../crates/crysirror-reallive/src/adapter/commands.rs#L184) |
| `0x517` | System commands (system) | Handled | — | [commands.rs:184](../../crates/crysirror-reallive/src/adapter/commands.rs#L184) |
| `0x51E` | System commands (system) | Handled | — | [commands.rs:174](../../crates/crysirror-reallive/src/adapter/commands.rs#L174) |
| `0x51F` | System commands (system) | Handled | — | [commands.rs:179](../../crates/crysirror-reallive/src/adapter/commands.rs#L179) |
| `0x520` | System commands (system) | Handled | — | [commands.rs:184](../../crates/crysirror-reallive/src/adapter/commands.rs#L184) |
| `0x521` | System commands (system) | Handled | — | [commands.rs:184](../../crates/crysirror-reallive/src/adapter/commands.rs#L184) |

## Module commands / adapter/extended / event_loop_extended

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0xA3` | PAGE: wait, then start a new page | Handled | — | [extended.rs:1356](../../crates/crysirror-reallive/src/adapter/extended.rs#L1356) |
| `0xC8` | CANCELCALL: the right-click routine, as if right-clicked | Handled | — | [extended.rs:1361](../../crates/crysirror-reallive/src/adapter/extended.rs#L1361) |
| `0xC9` | Update store | Handled | — | [extended.rs:1370](../../crates/crysirror-reallive/src/adapter/extended.rs#L1370) |
| `0x121` | MPLAY(name): music | Handled | — | [extended.rs:1372](../../crates/crysirror-reallive/src/adapter/extended.rs#L1372) |
| `0x173` | PCMPLAY(name, channel) | Handled | — | [extended.rs:1377](../../crates/crysirror-reallive/src/adapter/extended.rs#L1377) |
| `0x190` | MsgBox(title, text) with OK, Yes/No, Yes/No/Cancel buttons | Handled | — | [extended.rs:1385](../../crates/crysirror-reallive/src/adapter/extended.rs#L1385) |
| `0x191` | MsgBox(title, text) with OK, Yes/No, Yes/No/Cancel buttons | Handled | — | [extended.rs:1385](../../crates/crysirror-reallive/src/adapter/extended.rs#L1385) |
| `0x192` | MsgBox(title, text) with OK, Yes/No, Yes/No/Cancel buttons | Handled | — | [extended.rs:1385](../../crates/crysirror-reallive/src/adapter/extended.rs#L1385) |
| `0x193` | MsgBox(title, text) with OK, Yes/No, Yes/No/Cancel buttons | Handled | — | [extended.rs:1385](../../crates/crysirror-reallive/src/adapter/extended.rs#L1385) |
| `0x3E8` | ShowBackground: hide the text windows until a click | Handled | — | [extended.rs:1402](../../crates/crysirror-reallive/src/adapter/extended.rs#L1402) |
| `0x4B0` | CCOM_MESSAGEWINDOW_ON / _OFF([window \\| first, last]) | Handled | — | [extended.rs:1407](../../crates/crysirror-reallive/src/adapter/extended.rs#L1407) |
| `0x4B1` | CCOM_MESSAGEWINDOW_ON / _OFF([window \\| first, last]) | Handled | — | [extended.rs:1407](../../crates/crysirror-reallive/src/adapter/extended.rs#L1407) |
| `0x4B2` | CCOM_BTNSEL_ON / _OFF | Handled | — | [extended.rs:1418](../../crates/crysirror-reallive/src/adapter/extended.rs#L1418) |
| `0x4B3` | Update ext / ccom hide buttons | Handled | — | [extended.rs:1419](../../crates/crysirror-reallive/src/adapter/extended.rs#L1419) |
| `0x4B4` | CCOM_OBJECT_ON / _OFF(buffer \\| first, last) | Handled | — | [extended.rs:1421](../../crates/crysirror-reallive/src/adapter/extended.rs#L1421) |
| `0x4B5` | CCOM_OBJECT_ON / _OFF(buffer \\| first, last) | Handled | — | [extended.rs:1421](../../crates/crysirror-reallive/src/adapter/extended.rs#L1421) |
| `0x7DA` | CCOM_SET_WINDOW_EXBTN_ON / _OFF (_NOW, _SAVEPOINT)([button]) | Handled | — | [extended.rs:1433](../../crates/crysirror-reallive/src/adapter/extended.rs#L1433) |
| `0x7DB` | CCOM_SET_WINDOW_EXBTN_ON / _OFF (_NOW, _SAVEPOINT)([button]) | Handled | — | [extended.rs:1433](../../crates/crysirror-reallive/src/adapter/extended.rs#L1433) |
| `0x7DC` | CCOM_SET_WINDOW_EXBTN_ON / _OFF (_NOW, _SAVEPOINT)([button]) | Handled | — | [extended.rs:1433](../../crates/crysirror-reallive/src/adapter/extended.rs#L1433) |
| `0x7DD` | CCOM_SET_WINDOW_EXBTN_ON / _OFF (_NOW, _SAVEPOINT)([button]) | Handled | — | [extended.rs:1433](../../crates/crysirror-reallive/src/adapter/extended.rs#L1433) |
| `0x7DE` | CCOM_SET_WINDOW_EXBTN_ON / _OFF (_NOW, _SAVEPOINT)([button]) | Handled | — | [extended.rs:1433](../../crates/crysirror-reallive/src/adapter/extended.rs#L1433) |
| `0x7DF` | CCOM_SET_WINDOW_EXBTN_ON / _OFF (_NOW, _SAVEPOINT)([button]) | Handled | — | [extended.rs:1433](../../crates/crysirror-reallive/src/adapter/extended.rs#L1433) |

## Module commands / adapter/extended / event_loop_extended / opcode=400..=403

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x190` | Execute event loop extended branch | Handled | — | [extended.rs:1393](../../crates/crysirror-reallive/src/adapter/extended.rs#L1393) |
| `0x191` | Execute event loop extended branch | Handled | Guard: c.op.overload == 1 | [extended.rs:1394](../../crates/crysirror-reallive/src/adapter/extended.rs#L1394) |
| `0x192` | Execute event loop extended branch | Handled | — | [extended.rs:1396](../../crates/crysirror-reallive/src/adapter/extended.rs#L1396) |

## Module commands / adapter/extended / g00_buffers

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x3E8` | Execute g00 buffers branch | Handled | — | [extended.rs:772](../../crates/crysirror-reallive/src/adapter/extended.rs#L772) |
| `0x3E9` | Execute g00 buffers branch | Handled | — | [extended.rs:779](../../crates/crysirror-reallive/src/adapter/extended.rs#L779) |
| `0x3EA` | Execute g00 buffers branch | Handled | — | [extended.rs:783](../../crates/crysirror-reallive/src/adapter/extended.rs#L783) |

## Module commands / adapter/extended / graphics_extended

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x194` | grpFlash(r, g, b[, time]) / (area, r, g, b[, time]): the colour covers the scene for `time` (original `FlashOp`) | Handled | — | [extended.rs:1612](../../crates/crysirror-reallive/src/adapter/extended.rs#L1612) |
| `0x195` | BOXPIKACHU_OLD(r, g, b, count): the screen blinks; waits | Handled | — | [extended.rs:1621](../../crates/crysirror-reallive/src/adapter/extended.rs#L1621) |
| `0x196` | grpPan / grpShift / grpSlide | Handled | — | [extended.rs:1629](../../crates/crysirror-reallive/src/adapter/extended.rs#L1629) |
| `0x197` | grpPan / grpShift / grpSlide | Handled | — | [extended.rs:1629](../../crates/crysirror-reallive/src/adapter/extended.rs#L1629) |
| `0x198` | grpPan / grpShift / grpSlide | Handled | — | [extended.rs:1629](../../crates/crysirror-reallive/src/adapter/extended.rs#L1629) |

## Module commands / adapter/extended / message_extended

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x6A SetFontColourAll` | SetFontColourAll / FontColour for every window | Handled | — | [extended.rs:925](../../crates/crysirror-reallive/src/adapter/extended.rs#L925) |
| `0x6B FontSizeAll` | FontSizeAll([size]) | Handled | — | [extended.rs:927](../../crates/crysirror-reallive/src/adapter/extended.rs#L927) |
| `0x6C` | MESSAGE_SPEED(speed): this message's speed | Handled | — | [extended.rs:939](../../crates/crysirror-reallive/src/adapter/extended.rs#L939) |
| `0x6D` | Text and message windows (message_extended) | Handled | — | [extended.rs:940](../../crates/crysirror-reallive/src/adapter/extended.rs#L940) |
| `0x6E` | Text and message windows (message_extended) | Handled | — | [extended.rs:941](../../crates/crysirror-reallive/src/adapter/extended.rs#L941) |
| `0x6F` | Text and message windows (message_extended) | Handled | — | [extended.rs:942](../../crates/crysirror-reallive/src/adapter/extended.rs#L942) |
| `0x70` | MULTI_MESSAGE(window...): the text goes to the first window named | Handled | — | [extended.rs:944](../../crates/crysirror-reallive/src/adapter/extended.rs#L944) |
| `0x78 __doruby` | doruby | Handled | Guard: Machine::param_count(c) == 0 | [extended.rs:950](../../crates/crysirror-reallive/src/adapter/extended.rs#L950) |
| `0xAA msgHideAllTemp` | msgHideAllTemp: hide until a click | Handled | — | [extended.rs:961](../../crates/crysirror-reallive/src/adapter/extended.rs#L961) |
| `0x12C SetIndent` | SetIndent / ClearIndent | Handled | — | [extended.rs:966](../../crates/crysirror-reallive/src/adapter/extended.rs#L966) |
| `0x12D ClearIndent` | Clear Indent | Handled | — | [extended.rs:970](../../crates/crysirror-reallive/src/adapter/extended.rs#L970) |
| `0x136 TextPos` | TextPos(x, y) / TextPosX / TextPosY, TextOffset (relative) | Handled | — | [extended.rs:974](../../crates/crysirror-reallive/src/adapter/extended.rs#L974) |
| `0x137 TextPosX` | TextPos(x, y) / TextPosX / TextPosY, TextOffset (relative) | Handled | — | [extended.rs:974](../../crates/crysirror-reallive/src/adapter/extended.rs#L974) |
| `0x138 TextPosY` | TextPos(x, y) / TextPosX / TextPosY, TextOffset (relative) | Handled | — | [extended.rs:974](../../crates/crysirror-reallive/src/adapter/extended.rs#L974) |
| `0x140 TextOffset` | TextPos(x, y) / TextPosX / TextPosY, TextOffset (relative) | Handled | — | [extended.rs:974](../../crates/crysirror-reallive/src/adapter/extended.rs#L974) |
| `0x141 TextOffsetX` | TextPos(x, y) / TextPosX / TextPosY, TextOffset (relative) | Handled | — | [extended.rs:974](../../crates/crysirror-reallive/src/adapter/extended.rs#L974) |
| `0x142 TextOffsetY` | TextPos(x, y) / TextPosX / TextPosY, TextOffset (relative) | Handled | — | [extended.rs:974](../../crates/crysirror-reallive/src/adapter/extended.rs#L974) |
| `0x14A GetTextPos` | GetTextPos(x, y) / GetTextPos(y) | Handled | — | [extended.rs:989](../../crates/crysirror-reallive/src/adapter/extended.rs#L989) |
| `0x154 WindowLen` | Characters shown in a window / all windows | Handled | — | [extended.rs:1003](../../crates/crysirror-reallive/src/adapter/extended.rs#L1003) |
| `0x155 WindowLenAll` | Window Len All | Handled | — | [extended.rs:1007](../../crates/crysirror-reallive/src/adapter/extended.rs#L1007) |
| `0x190` | SYSBTN_OPEN / _CLOSE and the button pattern switches: the original's floating system buttons, remembered as set | Handled | — | [extended.rs:1016](../../crates/crysirror-reallive/src/adapter/extended.rs#L1016) |
| `0x191` | SYSBTN_OPEN / _CLOSE and the button pattern switches: the original's floating system buttons, remembered as set | Handled | — | [extended.rs:1016](../../crates/crysirror-reallive/src/adapter/extended.rs#L1016) |
| `0x192` | Text and message windows (message_extended) | Handled | — | [extended.rs:1019](../../crates/crysirror-reallive/src/adapter/extended.rs#L1019) |
| `0x193` | Text and message windows (message_extended) | Handled | — | [extended.rs:1019](../../crates/crysirror-reallive/src/adapter/extended.rs#L1019) |
| `0x194` | Text and message windows (message_extended) | Handled | — | [extended.rs:1019](../../crates/crysirror-reallive/src/adapter/extended.rs#L1019) |
| `0x195` | Text and message windows (message_extended) | Handled | — | [extended.rs:1019](../../crates/crysirror-reallive/src/adapter/extended.rs#L1019) |
| `0x196` | Text and message windows (message_extended) | Handled | — | [extended.rs:1019](../../crates/crysirror-reallive/src/adapter/extended.rs#L1019) |
| `0x197` | Text and message windows (message_extended) | Handled | — | [extended.rs:1019](../../crates/crysirror-reallive/src/adapter/extended.rs#L1019) |
| `0x198` | Text and message windows (message_extended) | Handled | — | [extended.rs:1019](../../crates/crysirror-reallive/src/adapter/extended.rs#L1019) |
| `0x199` | Text and message windows (message_extended) | Handled | — | [extended.rs:1019](../../crates/crysirror-reallive/src/adapter/extended.rs#L1019) |
| `0x1F6` | Text and message windows (message_extended) | Handled | — | [extended.rs:1019](../../crates/crysirror-reallive/src/adapter/extended.rs#L1019) |
| `0x1F7` | Text and message windows (message_extended) | Handled | — | [extended.rs:1019](../../crates/crysirror-reallive/src/adapter/extended.rs#L1019) |
| `0x1F8` | Text and message windows (message_extended) | Handled | — | [extended.rs:1019](../../crates/crysirror-reallive/src/adapter/extended.rs#L1019) |
| `0x1F9` | Text and message windows (message_extended) | Handled | — | [extended.rs:1019](../../crates/crysirror-reallive/src/adapter/extended.rs#L1019) |
| `0x1FA` | Text and message windows (message_extended) | Handled | — | [extended.rs:1019](../../crates/crysirror-reallive/src/adapter/extended.rs#L1019) |
| `0x1FB` | Text and message windows (message_extended) | Handled | — | [extended.rs:1019](../../crates/crysirror-reallive/src/adapter/extended.rs#L1019) |
| `0x1FC` | Text and message windows (message_extended) | Handled | — | [extended.rs:1019](../../crates/crysirror-reallive/src/adapter/extended.rs#L1019) |
| `0x1FD` | Text and message windows (message_extended) | Handled | — | [extended.rs:1019](../../crates/crysirror-reallive/src/adapter/extended.rs#L1019) |
| `0x3E8 FaceOpen` | Face Open | Handled | — | [extended.rs:1043](../../crates/crysirror-reallive/src/adapter/extended.rs#L1043) |
| `0x3E9 FaceClear` | Face Clear | Handled | — | [extended.rs:1048](../../crates/crysirror-reallive/src/adapter/extended.rs#L1048) |
| `0x7D0` | FLAGINDEX(var): a debugging aid of the original's window | Empty handler | Original no-op status not established | [extended.rs:1053](../../crates/crysirror-reallive/src/adapter/extended.rs#L1053) |

## Module commands / adapter/extended / message_extended / c.op.opcode=310 \| 311 \| 312 \| 320 \| 321 \| 322

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x136 TextPos` | Text Pos | Handled | — | [extended.rs:980](../../crates/crysirror-reallive/src/adapter/extended.rs#L980) |
| `0x137 TextPosX` | Text Pos X | Handled | — | [extended.rs:981](../../crates/crysirror-reallive/src/adapter/extended.rs#L981) |
| `0x138 TextPosY` | Text Pos Y | Handled | — | [extended.rs:982](../../crates/crysirror-reallive/src/adapter/extended.rs#L982) |
| `0x140 TextOffset` | Text Offset | Handled | — | [extended.rs:983](../../crates/crysirror-reallive/src/adapter/extended.rs#L983) |
| `0x141 TextOffsetX` | Text Offset X | Handled | — | [extended.rs:984](../../crates/crysirror-reallive/src/adapter/extended.rs#L984) |

## Module commands / adapter/extended / os

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x00` | Execute os branch | Handled | — | [extended.rs:813](../../crates/crysirror-reallive/src/adapter/extended.rs#L813) |
| `0x0A` | Execute os branch | Handled | — | [extended.rs:813](../../crates/crysirror-reallive/src/adapter/extended.rs#L813) |
| `0x78` | Update store | Handled | — | [extended.rs:826](../../crates/crysirror-reallive/src/adapter/extended.rs#L826) |

## Module commands / adapter/extended / save_query

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x578` | GET_SAVE_YEAR..GET_SAVE_MILLISECOND(slot), GET_SAVE_TITLE | Handled | — | [extended.rs:1819](../../crates/crysirror-reallive/src/adapter/extended.rs#L1819) |
| `0x579` | GET_SAVE_YEAR..GET_SAVE_MILLISECOND(slot), GET_SAVE_TITLE | Handled | — | [extended.rs:1819](../../crates/crysirror-reallive/src/adapter/extended.rs#L1819) |
| `0x57A` | GET_SAVE_YEAR..GET_SAVE_MILLISECOND(slot), GET_SAVE_TITLE | Handled | — | [extended.rs:1819](../../crates/crysirror-reallive/src/adapter/extended.rs#L1819) |
| `0x57B` | GET_SAVE_YEAR..GET_SAVE_MILLISECOND(slot), GET_SAVE_TITLE | Handled | — | [extended.rs:1819](../../crates/crysirror-reallive/src/adapter/extended.rs#L1819) |
| `0x57C` | GET_SAVE_YEAR..GET_SAVE_MILLISECOND(slot), GET_SAVE_TITLE | Handled | — | [extended.rs:1819](../../crates/crysirror-reallive/src/adapter/extended.rs#L1819) |
| `0x57D` | GET_SAVE_YEAR..GET_SAVE_MILLISECOND(slot), GET_SAVE_TITLE | Handled | — | [extended.rs:1819](../../crates/crysirror-reallive/src/adapter/extended.rs#L1819) |
| `0x57E` | GET_SAVE_YEAR..GET_SAVE_MILLISECOND(slot), GET_SAVE_TITLE | Handled | — | [extended.rs:1819](../../crates/crysirror-reallive/src/adapter/extended.rs#L1819) |
| `0x57F` | GET_SAVE_YEAR..GET_SAVE_MILLISECOND(slot), GET_SAVE_TITLE | Handled | — | [extended.rs:1819](../../crates/crysirror-reallive/src/adapter/extended.rs#L1819) |
| `0x580` | GET_SAVE_YEAR..GET_SAVE_MILLISECOND(slot), GET_SAVE_TITLE | Handled | — | [extended.rs:1819](../../crates/crysirror-reallive/src/adapter/extended.rs#L1819) |
| `0x581` | Save/load (save_query) | Handled | — | [extended.rs:1817](../../crates/crysirror-reallive/src/adapter/extended.rs#L1817) |
| `0x582` | Save/load (save_query) | Handled | — | [extended.rs:1817](../../crates/crysirror-reallive/src/adapter/extended.rs#L1817) |
| `0x583` | Save/load (save_query) | Handled | — | [extended.rs:1817](../../crates/crysirror-reallive/src/adapter/extended.rs#L1817) |
| `0x584` | Save/load (save_query) | Handled | — | [extended.rs:1817](../../crates/crysirror-reallive/src/adapter/extended.rs#L1817) |
| `0x585` | Save/load (save_query) | Handled | — | [extended.rs:1817](../../crates/crysirror-reallive/src/adapter/extended.rs#L1817) |
| `0x586` | Save/load (save_query) | Handled | — | [extended.rs:1817](../../crates/crysirror-reallive/src/adapter/extended.rs#L1817) |
| `0x58C` | Save/load (save_query) | Handled | — | [extended.rs:1830](../../crates/crysirror-reallive/src/adapter/extended.rs#L1830) |
| `0x58D` | Save/load (save_query) | Handled | — | [extended.rs:1817](../../crates/crysirror-reallive/src/adapter/extended.rs#L1817) |
| `0x58F` | Save/load (save_query) | Handled | — | [extended.rs:1831](../../crates/crysirror-reallive/src/adapter/extended.rs#L1831) |
| `0x590` | Save/load (save_query) | Handled | — | [extended.rs:1835](../../crates/crysirror-reallive/src/adapter/extended.rs#L1835) |
| `0x591` | Save/load (save_query) | Handled | — | [extended.rs:1836](../../crates/crysirror-reallive/src/adapter/extended.rs#L1836) |
| `0x5B3` | GET_SAVE_MESSAGE(slot, str) | Handled | — | [extended.rs:1838](../../crates/crysirror-reallive/src/adapter/extended.rs#L1838) |

## Module commands / adapter/extended / shake

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x64` | Execute shake branch | Handled | — | [extended.rs:1477](../../crates/crysirror-reallive/src/adapter/extended.rs#L1477) |
| `0x65` | Execute shake branch | Handled | — | [extended.rs:1478](../../crates/crysirror-reallive/src/adapter/extended.rs#L1478) |
| `0xC8` | Execute shake branch | Handled | — | [extended.rs:1479](../../crates/crysirror-reallive/src/adapter/extended.rs#L1479) |
| `0xC9` | Execute shake branch | Handled | — | [extended.rs:1480](../../crates/crysirror-reallive/src/adapter/extended.rs#L1480) |
| `0xCA` | Execute shake branch | Handled | — | [extended.rs:1481](../../crates/crysirror-reallive/src/adapter/extended.rs#L1481) |
| `0xCB` | Execute shake branch | Handled | — | [extended.rs:1482](../../crates/crysirror-reallive/src/adapter/extended.rs#L1482) |
| `0x190` | Execute shake branch | Handled | — | [extended.rs:1483](../../crates/crysirror-reallive/src/adapter/extended.rs#L1483) |

## Module commands / adapter/extended / sound_extended / (c.op.module, op)=(21, 32..=34)

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x20` | Audio playback/control (sound_extended) | Handled | — | [extended.rs:425](../../crates/crysirror-reallive/src/adapter/extended.rs#L425) |
| `0x21` | Audio playback/control (sound_extended) | Handled | — | [extended.rs:426](../../crates/crysirror-reallive/src/adapter/extended.rs#L426) |

## Module commands / adapter/extended / sound_extended / (c.op.module, op)=(21, 40..=42)

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x28` | Audio playback/control (sound_extended) | Handled | — | [extended.rs:438](../../crates/crysirror-reallive/src/adapter/extended.rs#L438) |
| `0x29` | Audio playback/control (sound_extended) | Handled | — | [extended.rs:439](../../crates/crysirror-reallive/src/adapter/extended.rs#L439) |

## Module commands / adapter/extended / sound_extended / (c.op.module, op)=(22, 12..=14)

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x0C` | Audio playback/control (sound_extended) | Handled | — | [extended.rs:524](../../crates/crysirror-reallive/src/adapter/extended.rs#L524) |
| `0x0D` | Audio playback/control (sound_extended) | Handled | — | [extended.rs:525](../../crates/crysirror-reallive/src/adapter/extended.rs#L525) |

## Module commands / adapter/extended / system_extended

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x66` | TIMEWAITKEY_NOMOUSECLEAR / DONTJUMP_TIMEWAIT(KEY)(_NOMOUSECLEAR) | Handled | — | [extended.rs:1158](../../crates/crysirror-reallive/src/adapter/extended.rs#L1158) |
| `0x67` | TIMEWAITKEY_NOMOUSECLEAR / DONTJUMP_TIMEWAIT(KEY)(_NOMOUSECLEAR) | Handled | — | [extended.rs:1158](../../crates/crysirror-reallive/src/adapter/extended.rs#L1158) |
| `0x68` | TIMEWAITKEY_NOMOUSECLEAR / DONTJUMP_TIMEWAIT(KEY)(_NOMOUSECLEAR) | Handled | — | [extended.rs:1158](../../crates/crysirror-reallive/src/adapter/extended.rs#L1158) |
| `0x69` | TIMEWAITKEY_NOMOUSECLEAR / DONTJUMP_TIMEWAIT(KEY)(_NOMOUSECLEAR) | Handled | — | [extended.rs:1158](../../crates/crysirror-reallive/src/adapter/extended.rs#L1158) |
| `0x6B` | KEYWAIT_NOMOUSECLEAR | Handled | — | [extended.rs:1163](../../crates/crysirror-reallive/src/adapter/extended.rs#L1163) |
| `0x73` | CmpTimer / CmpExTimer, SetTimer / SetExTimer | Handled | — | [extended.rs:1165](../../crates/crysirror-reallive/src/adapter/extended.rs#L1165) |
| `0x74` | CmpTimer / CmpExTimer, SetTimer / SetExTimer | Handled | — | [extended.rs:1165](../../crates/crysirror-reallive/src/adapter/extended.rs#L1165) |
| `0x7D` | CmpTimer / CmpExTimer, SetTimer / SetExTimer | Handled | — | [extended.rs:1165](../../crates/crysirror-reallive/src/adapter/extended.rs#L1165) |
| `0x7E` | CmpTimer / CmpExTimer, SetTimer / SetExTimer | Handled | — | [extended.rs:1165](../../crates/crysirror-reallive/src/adapter/extended.rs#L1165) |
| `0xB4` | AUTOMODEWAIT...: in auto mode, the pause auto mode gives | Handled | — | [extended.rs:1178](../../crates/crysirror-reallive/src/adapter/extended.rs#L1178) |
| `0xB5` | AUTOMODEWAIT...: in auto mode, the pause auto mode gives | Handled | — | [extended.rs:1178](../../crates/crysirror-reallive/src/adapter/extended.rs#L1178) |
| `0xB6` | AUTOMODEWAIT...: in auto mode, the pause auto mode gives | Handled | — | [extended.rs:1178](../../crates/crysirror-reallive/src/adapter/extended.rs#L1178) |
| `0xB7` | AUTOMODEWAIT...: in auto mode, the pause auto mode gives | Handled | — | [extended.rs:1178](../../crates/crysirror-reallive/src/adapter/extended.rs#L1178) |
| `0xB8` | AUTOMODEWAIT...: in auto mode, the pause auto mode gives | Handled | — | [extended.rs:1178](../../crates/crysirror-reallive/src/adapter/extended.rs#L1178) |
| `0xB9` | AUTOMODEWAIT...: in auto mode, the pause auto mode gives | Handled | — | [extended.rs:1178](../../crates/crysirror-reallive/src/adapter/extended.rs#L1178) |
| `0xD4` | SET/GET_WAIP_WINDOWCLOSE_MOD, SET/GET_GRPCOM_WINDOWCLOSE_MOD | Handled | — | [extended.rs:1188](../../crates/crysirror-reallive/src/adapter/extended.rs#L1188) |
| `0xD5` | System commands (system_extended) | Handled | — | [extended.rs:1189](../../crates/crysirror-reallive/src/adapter/extended.rs#L1189) |
| `0xD9` | System commands (system_extended) | Handled | — | [extended.rs:1190](../../crates/crysirror-reallive/src/adapter/extended.rs#L1190) |
| `0xDA` | System commands (system_extended) | Handled | — | [extended.rs:1191](../../crates/crysirror-reallive/src/adapter/extended.rs#L1191) |
| `0x1A1` | SET_WINDOW_WAKUMOD(window, mod) / SET_WINDOW_ATTRMOD(window, mod) | Handled | — | [extended.rs:1193](../../crates/crysirror-reallive/src/adapter/extended.rs#L1193) |
| `0x1AC` | SET_WINDOW_WAKUMOD(window, mod) / SET_WINDOW_ATTRMOD(window, mod) | Handled | — | [extended.rs:1193](../../crates/crysirror-reallive/src/adapter/extended.rs#L1193) |
| `0x1CC` | EnableWindowAnm / DisableWindowAnm([window]) | Handled | — | [extended.rs:1199](../../crates/crysirror-reallive/src/adapter/extended.rs#L1199) |
| `0x1CD` | EnableWindowAnm / DisableWindowAnm([window]) | Handled | — | [extended.rs:1199](../../crates/crysirror-reallive/src/adapter/extended.rs#L1199) |
| `0x1CE` | Get/Set Open/Close AnmMod/AnmTime(window[, value]) | Handled | — | [extended.rs:1205](../../crates/crysirror-reallive/src/adapter/extended.rs#L1205) |
| `0x1CF` | Get/Set Open/Close AnmMod/AnmTime(window[, value]) | Handled | — | [extended.rs:1205](../../crates/crysirror-reallive/src/adapter/extended.rs#L1205) |
| `0x1D0` | Get/Set Open/Close AnmMod/AnmTime(window[, value]) | Handled | — | [extended.rs:1205](../../crates/crysirror-reallive/src/adapter/extended.rs#L1205) |
| `0x1D1` | Get/Set Open/Close AnmMod/AnmTime(window[, value]) | Handled | — | [extended.rs:1205](../../crates/crysirror-reallive/src/adapter/extended.rs#L1205) |
| `0x1D2` | Get/Set Open/Close AnmMod/AnmTime(window[, value]) | Handled | — | [extended.rs:1205](../../crates/crysirror-reallive/src/adapter/extended.rs#L1205) |
| `0x1D3` | Get/Set Open/Close AnmMod/AnmTime(window[, value]) | Handled | — | [extended.rs:1205](../../crates/crysirror-reallive/src/adapter/extended.rs#L1205) |
| `0x1D4` | Get/Set Open/Close AnmMod/AnmTime(window[, value]) | Handled | — | [extended.rs:1205](../../crates/crysirror-reallive/src/adapter/extended.rs#L1205) |
| `0x1D5` | Get/Set Open/Close AnmMod/AnmTime(window[, value]) | Handled | — | [extended.rs:1205](../../crates/crysirror-reallive/src/adapter/extended.rs#L1205) |
| `0x1D6` | INIT_WINDOW_OPEN_ANIME / _CLOSE_ANIME([window]); GET/SET_WINDOW_ANIME_MOD | Handled | — | [extended.rs:1216](../../crates/crysirror-reallive/src/adapter/extended.rs#L1216) |
| `0x1D7` | INIT_WINDOW_OPEN_ANIME / _CLOSE_ANIME([window]); GET/SET_WINDOW_ANIME_MOD | Handled | — | [extended.rs:1216](../../crates/crysirror-reallive/src/adapter/extended.rs#L1216) |
| `0x1D8` | System commands (system_extended) | Handled | — | [extended.rs:1230](../../crates/crysirror-reallive/src/adapter/extended.rs#L1230) |
| `0x1D9` | System commands (system_extended) | Handled | — | [extended.rs:1234](../../crates/crysirror-reallive/src/adapter/extended.rs#L1234) |
| `0x1E0` | SET_WINDOW_DISP_ON / _OFF([window]) | Handled | — | [extended.rs:1240](../../crates/crysirror-reallive/src/adapter/extended.rs#L1240) |
| `0x1E1` | SET_WINDOW_DISP_ON / _OFF([window]) | Handled | — | [extended.rs:1240](../../crates/crysirror-reallive/src/adapter/extended.rs#L1240) |
| `0x2EE` | MESSAGE_ERASE(_CHECK): hide the windows until a click | Handled | — | [extended.rs:1251](../../crates/crysirror-reallive/src/adapter/extended.rs#L1251) |
| `0x2EF` | System commands (system_extended) | Handled | — | [extended.rs:1255](../../crates/crysirror-reallive/src/adapter/extended.rs#L1255) |
| `0x31B` | KOE_REPLAY / KOE_REPLAY_CHECK | Handled | — | [extended.rs:1257](../../crates/crysirror-reallive/src/adapter/extended.rs#L1257) |
| `0x31C` | System commands (system_extended) | Handled | — | [extended.rs:1263](../../crates/crysirror-reallive/src/adapter/extended.rs#L1263) |
| `0x4B1` | MenuReturn / MenuReturn2 / ReturnMenu | Handled | — | [extended.rs:1265](../../crates/crysirror-reallive/src/adapter/extended.rs#L1265) |
| `0x4B2` | MenuReturn / MenuReturn2 / ReturnMenu | Handled | — | [extended.rs:1265](../../crates/crysirror-reallive/src/adapter/extended.rs#L1265) |
| `0x4B3` | MenuReturn / MenuReturn2 / ReturnMenu | Handled | — | [extended.rs:1265](../../crates/crysirror-reallive/src/adapter/extended.rs#L1265) |
| `0x578` | System commands (system_extended) | Handled | — | [extended.rs:1276](../../crates/crysirror-reallive/src/adapter/extended.rs#L1276) |
| `0x579` | System commands (system_extended) | Handled | — | [extended.rs:1276](../../crates/crysirror-reallive/src/adapter/extended.rs#L1276) |
| `0x57A` | System commands (system_extended) | Handled | — | [extended.rs:1276](../../crates/crysirror-reallive/src/adapter/extended.rs#L1276) |
| `0x57B` | System commands (system_extended) | Handled | — | [extended.rs:1276](../../crates/crysirror-reallive/src/adapter/extended.rs#L1276) |
| `0x57C` | System commands (system_extended) | Handled | — | [extended.rs:1276](../../crates/crysirror-reallive/src/adapter/extended.rs#L1276) |
| `0x57D` | System commands (system_extended) | Handled | — | [extended.rs:1276](../../crates/crysirror-reallive/src/adapter/extended.rs#L1276) |
| `0x57E` | System commands (system_extended) | Handled | — | [extended.rs:1276](../../crates/crysirror-reallive/src/adapter/extended.rs#L1276) |
| `0x57F` | System commands (system_extended) | Handled | — | [extended.rs:1276](../../crates/crysirror-reallive/src/adapter/extended.rs#L1276) |
| `0x580` | System commands (system_extended) | Handled | — | [extended.rs:1276](../../crates/crysirror-reallive/src/adapter/extended.rs#L1276) |
| `0x581` | System commands (system_extended) | Handled | — | [extended.rs:1276](../../crates/crysirror-reallive/src/adapter/extended.rs#L1276) |
| `0x582` | System commands (system_extended) | Handled | — | [extended.rs:1276](../../crates/crysirror-reallive/src/adapter/extended.rs#L1276) |
| `0x583` | System commands (system_extended) | Handled | — | [extended.rs:1276](../../crates/crysirror-reallive/src/adapter/extended.rs#L1276) |
| `0x584` | System commands (system_extended) | Handled | — | [extended.rs:1276](../../crates/crysirror-reallive/src/adapter/extended.rs#L1276) |
| `0x585` | System commands (system_extended) | Handled | — | [extended.rs:1276](../../crates/crysirror-reallive/src/adapter/extended.rs#L1276) |
| `0x586` | System commands (system_extended) | Handled | — | [extended.rs:1276](../../crates/crysirror-reallive/src/adapter/extended.rs#L1276) |
| `0x58C` | System commands (system_extended) | Handled | — | [extended.rs:1276](../../crates/crysirror-reallive/src/adapter/extended.rs#L1276) |
| `0x58D` | System commands (system_extended) | Handled | — | [extended.rs:1276](../../crates/crysirror-reallive/src/adapter/extended.rs#L1276) |
| `0x58F` | System commands (system_extended) | Handled | — | [extended.rs:1276](../../crates/crysirror-reallive/src/adapter/extended.rs#L1276) |
| `0x590` | System commands (system_extended) | Handled | — | [extended.rs:1276](../../crates/crysirror-reallive/src/adapter/extended.rs#L1276) |
| `0x591` | System commands (system_extended) | Handled | — | [extended.rs:1276](../../crates/crysirror-reallive/src/adapter/extended.rs#L1276) |
| `0x5B3` | System commands (system_extended) | Handled | — | [extended.rs:1276](../../crates/crysirror-reallive/src/adapter/extended.rs#L1276) |
| `0x6A4` | Text input boxes (adopted `menu::text_input`) | Handled | — | [extended.rs:1275](../../crates/crysirror-reallive/src/adapter/extended.rs#L1275) |
| `0x6A5` | Text input boxes (adopted `menu::text_input`) | Handled | — | [extended.rs:1275](../../crates/crysirror-reallive/src/adapter/extended.rs#L1275) |
| `0x6A6` | Text input boxes (adopted `menu::text_input`) | Handled | — | [extended.rs:1275](../../crates/crysirror-reallive/src/adapter/extended.rs#L1275) |
| `0x6A7` | Text input boxes (adopted `menu::text_input`) | Handled | — | [extended.rs:1275](../../crates/crysirror-reallive/src/adapter/extended.rs#L1275) |
| `0x6A8` | Text input boxes (adopted `menu::text_input`) | Handled | — | [extended.rs:1275](../../crates/crysirror-reallive/src/adapter/extended.rs#L1275) |
| `0x6A9` | Text input boxes (adopted `menu::text_input`) | Handled | — | [extended.rs:1275](../../crates/crysirror-reallive/src/adapter/extended.rs#L1275) |
| `0x6AA` | Text input boxes (adopted `menu::text_input`) | Handled | — | [extended.rs:1275](../../crates/crysirror-reallive/src/adapter/extended.rs#L1275) |
| `0x6AB` | Text input boxes (adopted `menu::text_input`) | Handled | — | [extended.rs:1275](../../crates/crysirror-reallive/src/adapter/extended.rs#L1275) |
| `0x6AC` | Text input boxes (adopted `menu::text_input`) | Handled | — | [extended.rs:1275](../../crates/crysirror-reallive/src/adapter/extended.rs#L1275) |
| `0x6AD` | Text input boxes (adopted `menu::text_input`) | Handled | — | [extended.rs:1275](../../crates/crysirror-reallive/src/adapter/extended.rs#L1275) |
| `0x6AE` | Text input boxes (adopted `menu::text_input`) | Handled | — | [extended.rs:1275](../../crates/crysirror-reallive/src/adapter/extended.rs#L1275) |
| `0x6AF` | Text input boxes (adopted `menu::text_input`) | Handled | — | [extended.rs:1275](../../crates/crysirror-reallive/src/adapter/extended.rs#L1275) |

## Module commands / adapter/extended / text_input

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x6A4` | Text and message windows (text_input) | Handled | — | [extended.rs:1302](../../crates/crysirror-reallive/src/adapter/extended.rs#L1302) |
| `0x6A5` | Text and message windows (text_input) | Handled | — | [extended.rs:1314](../../crates/crysirror-reallive/src/adapter/extended.rs#L1314) |
| `0x6A6` | Text and message windows (text_input) | Handled | — | [extended.rs:1320](../../crates/crysirror-reallive/src/adapter/extended.rs#L1320) |
| `0x6A7` | Text and message windows (text_input) | Handled | — | [extended.rs:1324](../../crates/crysirror-reallive/src/adapter/extended.rs#L1324) |
| `0x6AE` | Text and message windows (text_input) | Handled | — | [extended.rs:1331](../../crates/crysirror-reallive/src/adapter/extended.rs#L1331) |
| `0x6AF` | Text and message windows (text_input) | Handled | — | [extended.rs:1340](../../crates/crysirror-reallive/src/adapter/extended.rs#L1340) |

## Module commands / adapter/selection / selection

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x00` | Choices (selection) | Handled | — | [selection.rs:294](../../crates/crysirror-reallive/src/adapter/selection.rs#L294) |
| `0x02` | Choices (selection) | Handled | — | [selection.rs:293](../../crates/crysirror-reallive/src/adapter/selection.rs#L293) |
| `0x03` | Choices (selection) | Handled | — | [selection.rs:293](../../crates/crysirror-reallive/src/adapter/selection.rs#L293) |

## Module commands / vm/modules/jmp / dispatch

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `(0x0A, _) ret` | Return from a call | Handled | — | [jmp.rs:116](../../crates/crysirror-reallive/src/vm/modules/jmp.rs#L116) |
| `(0x0B, _) jump` | Transfer to another scenario | Handled | — | [jmp.rs:130](../../crates/crysirror-reallive/src/vm/modules/jmp.rs#L130) |
| `(0x0C, _) farcall` | Call another scenario | Handled | — | [jmp.rs:137](../../crates/crysirror-reallive/src/vm/modules/jmp.rs#L137) |
| `(0x0D, _) rtl` | rtl / rtl_with | Handled | — | [jmp.rs:144](../../crates/crysirror-reallive/src/vm/modules/jmp.rs#L144) |
| `(0x11, _) ret_with` | ret with | Handled | — | [jmp.rs:120](../../crates/crysirror-reallive/src/vm/modules/jmp.rs#L120) |
| `(0x12, _) farcall_with` | farcall_with | Handled | — | [jmp.rs:158](../../crates/crysirror-reallive/src/vm/modules/jmp.rs#L158) |
| `(0x13, _) rtl_with` | rtl with | Handled | — | [jmp.rs:148](../../crates/crysirror-reallive/src/vm/modules/jmp.rs#L148) |
| `(0x64, _)` | RETURN_L_FLAG_SET(index, value) | Handled | — | [jmp.rs:167](../../crates/crysirror-reallive/src/vm/modules/jmp.rs#L167) |
| `(0x65, _) pushStringValueUp` | pushStringValueUp | Handled | — | [jmp.rs:196](../../crates/crysirror-reallive/src/vm/modules/jmp.rs#L196) |
| `(0x66, _)` | RETURN_L_FLAG_SETS((index, value)...), RETURN_K_FLAG_SETS((index, string)... or (index, value)...) | Handled | — | [jmp.rs:175](../../crates/crysirror-reallive/src/vm/modules/jmp.rs#L175) |
| `(0x67, _)` | RETURN_L_FLAG_SETS((index, value)...), RETURN_K_FLAG_SETS((index, string)... or (index, value)...) | Handled | — | [jmp.rs:175](../../crates/crysirror-reallive/src/vm/modules/jmp.rs#L175) |

## Module commands / vm/modules/mem / dispatch

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `(0x00, _) setarray` | setarray(origin, values+) | Handled | — | [mem.rs:14](../../crates/crysirror-reallive/src/vm/modules/mem.rs#L14) |
| `(0x01, _) setrng` | setrng(first, last[, value]) | Handled | — | [mem.rs:25](../../crates/crysirror-reallive/src/vm/modules/mem.rs#L25) |
| `(0x02, _) cpyrng` | cpyrng(source, dest, count) | Handled | — | [mem.rs:34](../../crates/crysirror-reallive/src/vm/modules/mem.rs#L34) |
| `(0x03, _) setarray_stepped` | setarray_stepped(origin, step, values+) | Handled | — | [mem.rs:47](../../crates/crysirror-reallive/src/vm/modules/mem.rs#L47) |
| `(0x04, _) setrng_stepped` | setrng_stepped(origin, step, count[, value]) | Handled | — | [mem.rs:56](../../crates/crysirror-reallive/src/vm/modules/mem.rs#L56) |
| `(0x05, _)` | FLAGCOPY_INDEX(source, source offset, dest, dest offset, count) | Handled | — | [mem.rs:153](../../crates/crysirror-reallive/src/vm/modules/mem.rs#L153) |
| `(0x06, _) cpyvars` | cpyvars(dest, offset, sources+) | Handled | — | [mem.rs:66](../../crates/crysirror-reallive/src/vm/modules/mem.rs#L66) |
| `(0x12C, _)` | Execute dispatch branch | Handled | — | [mem.rs:162](../../crates/crysirror-reallive/src/vm/modules/mem.rs#L162) |
| `(0x12D, _)` | Execute dispatch branch | Handled | — | [mem.rs:162](../../crates/crysirror-reallive/src/vm/modules/mem.rs#L162) |
| `(0x12E, _)` | Execute dispatch branch | Handled | — | [mem.rs:162](../../crates/crysirror-reallive/src/vm/modules/mem.rs#L162) |
| `(0x12F, _)` | Execute dispatch branch | Handled | — | [mem.rs:162](../../crates/crysirror-reallive/src/vm/modules/mem.rs#L162) |
| `(0x130, _)` | Execute dispatch branch | Handled | — | [mem.rs:162](../../crates/crysirror-reallive/src/vm/modules/mem.rs#L162) |
| `(0x131, _)` | Execute dispatch branch | Handled | — | [mem.rs:162](../../crates/crysirror-reallive/src/vm/modules/mem.rs#L162) |
| `(0x132, _)` | Execute dispatch branch | Handled | — | [mem.rs:162](../../crates/crysirror-reallive/src/vm/modules/mem.rs#L162) |
| `(0x133, _)` | Execute dispatch branch | Handled | — | [mem.rs:162](../../crates/crysirror-reallive/src/vm/modules/mem.rs#L162) |
| `(0x64, _) sum` | sum(first, last) | Handled | — | [mem.rs:82](../../crates/crysirror-reallive/src/vm/modules/mem.rs#L82) |
| `(0x65, _) sums` | sums((first, last)+) | Handled | — | [mem.rs:88](../../crates/crysirror-reallive/src/vm/modules/mem.rs#L88) |
| `(0xC8, overload)` | FLAGSORT_LARGE / FLAGSORT_SMALL(count, keys, values...): sorts the keys (descending / ascending) and reorders the value… | Handled | — | [mem.rs:103](../../crates/crysirror-reallive/src/vm/modules/mem.rs#L103) |
| `(0xC9, overload)` | FLAGSORT_LARGE / FLAGSORT_SMALL(count, keys, values...): sorts the keys (descending / ascending) and reorders the value… | Handled | — | [mem.rs:103](../../crates/crysirror-reallive/src/vm/modules/mem.rs#L103) |
| `(0xD2, _)` | FLAGINDEXSORT_LARGE / _SMALL(count, keys[, dest[, first index]]): the key indices in sorted order | Handled | — | [mem.rs:138](../../crates/crysirror-reallive/src/vm/modules/mem.rs#L138) |
| `(0xD3, _)` | FLAGINDEXSORT_LARGE / _SMALL(count, keys[, dest[, first index]]): the key indices in sorted order | Handled | — | [mem.rs:138](../../crates/crysirror-reallive/src/vm/modules/mem.rs#L138) |

## Module commands / vm/modules/obj / animation

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x00` | Execute animation branch | Handled | — | [obj.rs:1108](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1108) |
| `0x01` | OBJFRONTANM_PAUSE / _RESUME | Handled | — | [obj.rs:1036](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1036) |
| `0x02` | OBJFRONTANM_PAUSE / _RESUME | Handled | — | [obj.rs:1036](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1036) |
| `0x03` | Update store | Handled | — | [obj.rs:1114](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1114) |
| `0x04` | Execute animation branch | Handled | — | [obj.rs:1121](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1121) |
| `0x05` | OBJFRONTANM_KEEP / _RELEASE | Empty handler | Original no-op status not established | [obj.rs:1048](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1048) |
| `0x06` | OBJFRONTANM_KEEP / _RELEASE | Empty handler | Original no-op status not established | [obj.rs:1048](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1048) |
| `0x64` | Execute animation branch | Handled | — | [obj.rs:993](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L993) |
| `0x65` | Execute animation branch | Handled | — | [obj.rs:1000](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1000) |
| `0x66` | Execute animation branch | Handled | — | [obj.rs:1001](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1001) |
| `0x67` | Update store | Handled | — | [obj.rs:1002](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1002) |
| `0x3E8` | Execute animation branch | Handled | — | [obj.rs:1108](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1108) |
| `0xBEA` | *GANANM_ONESHOTRELEASE / _ONESHOTWAITRELEASE(set) | Handled | — | [obj.rs:1088](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1088) |
| `0xBEB` | *GANANM_ONESHOTRELEASE / _ONESHOTWAITRELEASE(set) | Handled | — | [obj.rs:1088](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1088) |
| `0xC1D` | *GANANM_NEXT_LOOP / _ONESHOT / _ONESHOTFREE / _ONESHOTWAIT / _ONESHOTWAITFREE / _EYE / _KOESYNC(set): after the running… | Handled | — | [obj.rs:1051](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1051) |
| `0xC1E` | *GANANM_NEXT_LOOP / _ONESHOT / _ONESHOTFREE / _ONESHOTWAIT / _ONESHOTWAITFREE / _EYE / _KOESYNC(set): after the running… | Handled | — | [obj.rs:1051](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1051) |
| `0xC1F` | *GANANM_NEXT_LOOP / _ONESHOT / _ONESHOTFREE / _ONESHOTWAIT / _ONESHOTWAITFREE / _EYE / _KOESYNC(set): after the running… | Handled | — | [obj.rs:1051](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1051) |
| `0xC20` | *GANANM_NEXT_LOOP / _ONESHOT / _ONESHOTFREE / _ONESHOTWAIT / _ONESHOTWAITFREE / _EYE / _KOESYNC(set): after the running… | Handled | — | [obj.rs:1051](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1051) |
| `0xC21` | *GANANM_NEXT_LOOP / _ONESHOT / _ONESHOTFREE / _ONESHOTWAIT / _ONESHOTWAITFREE / _EYE / _KOESYNC(set): after the running… | Handled | — | [obj.rs:1051](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1051) |
| `0xC22` | *GANANM_NEXT_LOOP / _ONESHOT / _ONESHOTFREE / _ONESHOTWAIT / _ONESHOTWAITFREE / _EYE / _KOESYNC(set): after the running… | Handled | — | [obj.rs:1051](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1051) |
| `0xC23` | *GANANM_NEXT_LOOP / _ONESHOT / _ONESHOTFREE / _ONESHOTWAIT / _ONESHOTWAITFREE / _EYE / _KOESYNC(set): after the running… | Handled | — | [obj.rs:1051](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1051) |
| `0xC24` | *GANANM_NEXT_LOOP / _ONESHOT / _ONESHOTFREE / _ONESHOTWAIT / _ONESHOTWAITFREE / _EYE / _KOESYNC(set): after the running… | Handled | — | [obj.rs:1051](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1051) |
| `0xC25` | *GANANM_NEXT_LOOP / _ONESHOT / _ONESHOTFREE / _ONESHOTWAIT / _ONESHOTWAITFREE / _EYE / _KOESYNC(set): after the running… | Handled | — | [obj.rs:1051](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1051) |

## Module commands / vm/modules/obj / animation / opcode=3101..=3109

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0xC1D` | Execute animation branch | Handled | — | [obj.rs:1053](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1053) |
| `0xC1E` | Execute animation branch | Handled | — | [obj.rs:1053](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1053) |
| `0xC1F` | Execute animation branch | Handled | — | [obj.rs:1054](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1054) |
| `0xC20` | Execute animation branch | Handled | — | [obj.rs:1054](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1054) |
| `0xC21` | Execute animation branch | Handled | — | [obj.rs:1055](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1055) |
| `0xC22` | Execute animation branch | Handled | — | [obj.rs:1054](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1054) |
| `0xC23` | Execute animation branch | Handled | — | [obj.rs:1055](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1055) |

## Module commands / vm/modules/obj / creation

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x3E8` | Execute creation branch | Handled | — | [obj.rs:905](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L905) |
| `0x3E9` | Execute creation branch | Handled | — | [obj.rs:905](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L905) |
| `0x3EA` | Execute creation branch | Handled | — | [obj.rs:905](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L905) |
| `0x3EB` | objOfFileGan; LOADGANCOPYPARAM (1004) keeps the parameters too, as every load does here | Handled | — | [obj.rs:921](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L921) |
| `0x3EC` | objOfFileGan; LOADGANCOPYPARAM (1004) keeps the parameters too, as every load does here | Handled | — | [obj.rs:921](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L921) |
| `0x3ED` | LOADTHUMBNAIL(slot): save thumbnails are not captured yet | Unsupported | reallive: save thumbnails (LOADTHUMBNAIL) are not supported | [obj.rs:918](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L918) |
| `0x44C` | Execute creation branch | Handled | — | [obj.rs:935](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L935) |
| `0x44D` | Execute creation branch | Handled | — | [obj.rs:935](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L935) |
| `0x4B0` | Execute creation branch | Handled | — | [obj.rs:951](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L951) |
| `0x514` | Execute creation branch | Handled | — | [obj.rs:905](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L905) |
| `0x578` | Execute creation branch | Handled | — | [obj.rs:905](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L905) |
| `0x5DC` | Execute creation branch | Handled | — | [obj.rs:958](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L958) |

## Module commands / vm/modules/obj / creation / command.op.opcode=1000..=1002 \| 1300 \| 1400

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x514` | Execute creation branch | Handled | — | [obj.rs:910](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L910) |
| `0x578` | Execute creation branch | Handled | — | [obj.rs:911](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L911) |

## Module commands / vm/modules/obj / getters

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x3E8` | Execute getters branch | Handled | — | [obj.rs:1175](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1175) |
| `0x3E9` | Update store | Handled | — | [obj.rs:1181](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1181) |
| `0x3EA` | Update store | Handled | — | [obj.rs:1182](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1182) |
| `0x3EB` | Update store | Handled | — | [obj.rs:1183](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1183) |
| `0x3EC` | Update store | Handled | — | [obj.rs:1184](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1184) |
| `0x3ED` | GET_ERIA / GET_BOX / GET_BOXERIA / GET_BOXOWNERIA(x1, y1, x2, y2), GET_RECT / GET_RECTERIA / GET_RECTOWNERIA(x, y, w, h) | Handled | — | [obj.rs:1205](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1205) |
| `0x3EE` | Execute getters branch | Handled | — | [obj.rs:1185](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1185) |
| `0x3EF` | Update store | Handled | — | [obj.rs:1192](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1192) |
| `0x3F0` | Update store | Handled | — | [obj.rs:1192](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1192) |
| `0x3F1` | Update store | Handled | — | [obj.rs:1200](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1200) |
| `0x3F2` | Update store | Handled | — | [obj.rs:1201](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1201) |
| `0x3F3` | Update store | Handled | — | [obj.rs:1202](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1202) |
| `0x3F4` | Execute getters branch | Handled | — | [obj.rs:1222](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1222) |
| `0x3F5` | Update store | Handled | — | [obj.rs:1223](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1223) |
| `0x3F6` | Update store | Handled | — | [obj.rs:1223](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1223) |
| `0x3F7` | Update store | Handled | — | [obj.rs:1223](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1223) |
| `0x3F8` | Execute getters branch | Handled | — | [obj.rs:1224](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1224) |
| `0x3F9` | Update store | Handled | — | [obj.rs:1225](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1225) |
| `0x3FA` | Update store | Handled | — | [obj.rs:1225](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1225) |
| `0x3FB` | Update store | Handled | — | [obj.rs:1225](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1225) |
| `0x3FC` | Update store | Handled | — | [obj.rs:1225](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1225) |
| `0x3FD` | Update store | Handled | — | [obj.rs:1226](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1226) |
| `0x3FE` | GET_ERIA / GET_BOX / GET_BOXERIA / GET_BOXOWNERIA(x1, y1, x2, y2), GET_RECT / GET_RECTERIA / GET_RECTOWNERIA(x, y, w, h) | Handled | — | [obj.rs:1205](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1205) |
| `0x3FF` | GET_ERIA / GET_BOX / GET_BOXERIA / GET_BOXOWNERIA(x1, y1, x2, y2), GET_RECT / GET_RECTERIA / GET_RECTOWNERIA(x, y, w, h) | Handled | — | [obj.rs:1205](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1205) |
| `0x400` | GET_MOJI(str): a text object's text | Handled | — | [obj.rs:1228](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1228) |
| `0x401` | Execute getters branch | Handled | — | [obj.rs:1232](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1232) |
| `0x402` | Update store | Handled | — | [obj.rs:1237](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1237) |
| `0x403` | Update store | Handled | — | [obj.rs:1238](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1238) |
| `0x404` | Execute getters branch | Handled | — | [obj.rs:1239](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1239) |
| `0x405` | Update store | Handled | — | [obj.rs:1240](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1240) |
| `0x406` | Update store | Handled | — | [obj.rs:1241](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1241) |
| `0x408` | Update store | Handled | — | [obj.rs:1242](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1242) |
| `0x409` | Update store | Handled | — | [obj.rs:1243](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1243) |
| `0x40A` | GET_ERIA / GET_BOX / GET_BOXERIA / GET_BOXOWNERIA(x1, y1, x2, y2), GET_RECT / GET_RECTERIA / GET_RECTOWNERIA(x, y, w, h) | Handled | — | [obj.rs:1205](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1205) |
| `0x40B` | GET_ERIA / GET_BOX / GET_BOXERIA / GET_BOXOWNERIA(x1, y1, x2, y2), GET_RECT / GET_RECTERIA / GET_RECTOWNERIA(x, y, w, h) | Handled | — | [obj.rs:1205](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1205) |
| `0x40C` | Update store | Handled | — | [obj.rs:1244](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1244) |
| `0x40D` | Update store | Handled | — | [obj.rs:1245](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1245) |
| `0x40E` | Execute getters branch | Handled | — | [obj.rs:1246](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1246) |
| `0x40F` | Update store | Handled | — | [obj.rs:1255](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1255) |
| `0x410` | Update store | Handled | — | [obj.rs:1256](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1256) |
| `0x415` | Execute getters branch | Handled | — | [obj.rs:1260](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1260) |
| `0x416` | Execute getters branch | Handled | — | [obj.rs:1268](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1268) |
| `0x417` | Update store | Handled | — | [obj.rs:1269](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1269) |
| `0x418` | Update store | Handled | — | [obj.rs:1270](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1270) |
| `0x419` | Update store | Handled | — | [obj.rs:1271](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1271) |
| `0x41A` | Execute getters branch | Handled | — | [obj.rs:1272](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1272) |
| `0x41B` | Update store | Handled | — | [obj.rs:1273](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1273) |
| `0x41C` | Update store | Handled | — | [obj.rs:1274](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1274) |
| `0x41D` | Execute getters branch | Handled | — | [obj.rs:1275](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1275) |
| `0x41E` | Update store | Handled | — | [obj.rs:1276](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1276) |
| `0x41F` | Update store | Handled | — | [obj.rs:1277](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1277) |
| `0x422` | Update store | Handled | — | [obj.rs:1289](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1289) |
| `0x424` | GET_MMX_USE / GET_QUAKE_USE / GET_FILTER_MOD: nothing special here | Handled | — | [obj.rs:1288](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1288) |
| `0x425` | Execute getters branch | Handled | — | [obj.rs:1278](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1278) |
| `0x426` | Update store | Handled | — | [obj.rs:1279](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1279) |
| `0x427` | Update store | Handled | — | [obj.rs:1280](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1280) |
| `0x428` | Execute getters branch | Handled | — | [obj.rs:1281](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1281) |
| `0x42A` | Update store | Handled | — | [obj.rs:1285](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1285) |
| `0x42B` | Update store | Handled | — | [obj.rs:1286](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1286) |
| `0x42E` | GET_ERIA / GET_BOX / GET_BOXERIA / GET_BOXOWNERIA(x1, y1, x2, y2), GET_RECT / GET_RECTERIA / GET_RECTOWNERIA(x, y, w, h) | Handled | — | [obj.rs:1205](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1205) |
| `0x42F` | GET_ERIA / GET_BOX / GET_BOXERIA / GET_BOXOWNERIA(x1, y1, x2, y2), GET_RECT / GET_RECTERIA / GET_RECTOWNERIA(x, y, w, h) | Handled | — | [obj.rs:1205](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1205) |
| `0x430` | GET_ERIA / GET_BOX / GET_BOXERIA / GET_BOXOWNERIA(x1, y1, x2, y2), GET_RECT / GET_RECTERIA / GET_RECTOWNERIA(x, y, w, h) | Handled | — | [obj.rs:1205](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1205) |
| `0x431` | GET_MMX_USE / GET_QUAKE_USE / GET_FILTER_MOD: nothing special here | Handled | — | [obj.rs:1288](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1288) |
| `0x432` | Update store | Handled | — | [obj.rs:1290](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1290) |
| `0x44C` | Execute getters branch | Handled | — | [obj.rs:1329](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1329) |
| `0x44D` | GET_TYPE: 0 none, then by kind of contents | Handled | — | [obj.rs:1292](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1292) |
| `0x44E` | GET_SELCENTER(x, y): the centre of the object on screen | Handled | — | [obj.rs:1305](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1305) |
| `0x1774` | EVE_DISP_ENDALL(speed up): every running change ends at once | Handled | — | [obj.rs:1324](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1324) |

## Module commands / vm/modules/obj / getters / command.op.opcode=1005 \| 1022 \| 1035 \| 1070 \| 1072 \| 1023 \| 1034 \| …

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x3FE` | Execute getters branch | Handled | — | [obj.rs:1208](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1208) |
| `0x3FF` | Execute getters branch | Handled | — | [obj.rs:1208](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1208) |
| `0x42E` | Execute getters branch | Handled | — | [obj.rs:1207](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1207) |
| `0x42F` | Execute getters branch | Handled | — | [obj.rs:1207](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1207) |
| `0x430` | Execute getters branch | Handled | — | [obj.rs:1207](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1207) |

## Module commands / vm/modules/obj / management

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x00` | Execute management branch | Handled | — | [obj.rs:1454](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1454) |
| `0x01` | Execute management branch | Handled | — | [obj.rs:1454](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1454) |
| `0x02` | objCopy family: (source, destination) | Handled | — | [obj.rs:1435](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1435) |
| `0x03` | objCopy family: (source, destination) | Handled | — | [obj.rs:1435](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1435) |
| `0x04` | Execute management branch | Handled | — | [obj.rs:1454](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1454) |
| `0x05` | Execute management branch | Handled | — | [obj.rs:1454](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1454) |
| `0x06` | SAVECHILD_ON / _OFF: children are always saved here | Empty handler | Original no-op status not established | [obj.rs:1393](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1393) |
| `0x07` | SAVECHILD_ON / _OFF: children are always saved here | Empty handler | Original no-op status not established | [obj.rs:1393](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1393) |
| `0x0A` | Execute management branch | Handled | — | [obj.rs:1454](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1454) |
| `0x0B` | Execute management branch | Handled | — | [obj.rs:1454](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1454) |
| `0x0C` | Execute management branch | Handled | — | [obj.rs:1454](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1454) |
| `0x0D` | Execute management branch | Handled | — | [obj.rs:1454](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1454) |
| `0x0E` | objCopy family: (source, destination) | Handled | — | [obj.rs:1435](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1435) |
| `0x0F` | GET_FILENAME / GET_GAN_FILENAME(buf, var); the _ALL forms have nothing to write to | Handled | — | [obj.rs:1396](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1396) |
| `0x10` | GET_FILENAME / GET_GAN_FILENAME(buf, var); the _ALL forms have nothing to write to | Handled | — | [obj.rs:1396](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1396) |
| `0x64` | Execute management branch | Handled | — | [obj.rs:1454](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1454) |
| `0x65` | Execute management branch | Handled | — | [obj.rs:1454](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1454) |
| `0x66` | OBJCOPYALL: every foreground object to the background | Handled | — | [obj.rs:1429](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1429) |
| `0x6E` | Execute management branch | Handled | — | [obj.rs:1454](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1454) |
| `0x6F` | Execute management branch | Handled | — | [obj.rs:1454](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1454) |
| `0x70` | Execute management branch | Handled | — | [obj.rs:1454](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1454) |
| `0x71` | Execute management branch | Handled | — | [obj.rs:1454](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1454) |
| `0x73` | Execute management branch | Empty handler | Original no-op status not established | [obj.rs:1427](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1427) |
| `0x74` | Execute management branch | Empty handler | Original no-op status not established | [obj.rs:1427](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1427) |

## Module commands / vm/modules/obj / management / opcode=0 \| 1 \| 4 \| 5 \| 10..=13 \| 100 \| 101 \| 110..=113

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x00` | Execute management branch | Handled | — | [obj.rs:1471](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1471) |
| `0x01` | Execute management branch | Handled | — | [obj.rs:1472](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1472) |
| `0x04` | Execute management branch | Handled | — | [obj.rs:1473](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1473) |
| `0x05` | Execute management branch | Handled | — | [obj.rs:1474](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1474) |
| `0x0A` | Execute management branch | Handled | — | [obj.rs:1475](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1475) |
| `0x0C` | INITREP: the repetition origin back to zero | Handled | — | [obj.rs:1477](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1477) |
| `0x0D` | WAIPERASE_INITPARAM | Handled | — | [obj.rs:1479](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1479) |
| `0x64` | Execute management branch | Handled | — | [obj.rs:1471](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1471) |
| `0x65` | Execute management branch | Handled | — | [obj.rs:1472](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1472) |
| `0x6E` | Execute management branch | Handled | — | [obj.rs:1475](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1475) |
| `0x70` | INITREP: the repetition origin back to zero | Handled | — | [obj.rs:1477](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1477) |
| `0x71` | WAIPERASE_INITPARAM | Handled | — | [obj.rs:1479](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1479) |

## Module commands / vm/modules/obj / object_event

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x898` | Object methods (object_event) | Handled | — | [obj.rs:710](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L710) |
| `0x899` | Object methods (object_event) | Handled | — | [obj.rs:711](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L711) |
| `0x89A` | Object methods (object_event) | Handled | — | [obj.rs:711](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L711) |
| `0x89B` | Object methods (object_event) | Handled | — | [obj.rs:719](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L719) |
| `0x89C` | Object methods (object_event) | Handled | — | [obj.rs:724](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L724) |
| `0x89D` | Object methods (object_event) | Handled | — | [obj.rs:729](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L729) |
| `0x89E` | Object methods (object_event) | Handled | — | [obj.rs:730](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L730) |
| `0x89F` | Object methods (object_event) | Handled | — | [obj.rs:739](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L739) |
| `0x8A0` | Object methods (object_event) | Handled | — | [obj.rs:746](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L746) |
| `0x8A1` | Object methods (object_event) | Handled | — | [obj.rs:747](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L747) |
| `0x8A2` | BLINK_PAL / _COL / _REVERSE / _TR: on and off, undamped | Handled | — | [obj.rs:749](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L749) |
| `0x8A3` | Object methods (object_event) | Handled | — | [obj.rs:758](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L758) |
| `0x8A4` | Object methods (object_event) | Handled | — | [obj.rs:765](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L765) |
| `0x8A5` | Object methods (object_event) | Handled | — | [obj.rs:766](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L766) |
| `0x8A6` | REPEAT_MOVE / _ROTATE / _SCALE: out and back, again and again | Handled | — | [obj.rs:768](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L768) |
| `0x8A7` | Object methods (object_event) | Handled | — | [obj.rs:776](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L776) |
| `0x8A8` | Object methods (object_event) | Handled | — | [obj.rs:794](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L794) |

## Module commands / vm/modules/obj / properties

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x3ED` | Execute properties branch | Handled | Guard: v.len() >= 4 | [obj.rs:483](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L483) |
| `0x3FD` | Execute properties branch | Handled | — | [obj.rs:492](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L492) |
| `0x3FE` | Execute properties branch | Handled | — | [obj.rs:493](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L493) |
| `0x3FF` | OBJFRONTSET_RECT(x, y, w, h): a rectangle object's area | Handled | — | [obj.rs:637](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L637) |
| `0x400` | Execute properties branch | Handled | — | [obj.rs:505](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L505) |
| `0x401` | Execute properties branch | Handled | — | [obj.rs:513](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L513) |
| `0x402` | Execute properties branch | Handled | — | [obj.rs:522](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L522) |
| `0x403` | Execute properties branch | Handled | — | [obj.rs:523](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L523) |
| `0x404` | Execute properties branch | Handled | — | [obj.rs:524](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L524) |
| `0x405` | Execute properties branch | Handled | — | [obj.rs:527](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L527) |
| `0x406` | Execute properties branch | Handled | — | [obj.rs:528](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L528) |
| `0x407` | Execute properties branch | Handled | — | [obj.rs:529](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L529) |
| `0x408` | Execute properties branch | Handled | — | [obj.rs:546](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L546) |
| `0x409` | Execute properties branch | Handled | — | [obj.rs:547](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L547) |
| `0x40A` | Execute properties branch | Handled | Guard: v.is_empty() | [obj.rs:484](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L484) |
| `0x40B` | Execute properties branch | Handled | Guard: v.is_empty() | [obj.rs:484](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L484) |
| `0x40D` | Execute properties branch | Handled | — | [obj.rs:548](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L548) |
| `0x40E` | Execute properties branch | Handled | — | [obj.rs:549](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L549) |
| `0x40F` | Execute properties branch | Handled | — | [obj.rs:557](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L557) |
| `0x411` | objAdjustAll(x, y) / X / Y: every repno's offset | Handled | — | [obj.rs:559](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L559) |
| `0x412` | Execute properties branch | Handled | — | [obj.rs:565](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L565) |
| `0x413` | Execute properties branch | Handled | — | [obj.rs:569](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L569) |
| `0x414` | OBJFRONTSET_ALLREPTR(alpha): every repno's alpha | Handled | — | [obj.rs:574](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L574) |
| `0x415` | OBJFRONTSET_GANPATNO(set, pattern): show one GAN frame | Handled | — | [obj.rs:576](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L576) |
| `0x420` | Execute properties branch | Handled | — | [obj.rs:651](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L651) |
| `0x421` | OBJFRONTSET_WEATHER_REPCNT(count) / _WEATHERPARAM_TYPEB(...) | Handled | — | [obj.rs:608](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L608) |
| `0x422` | OBJFRONTSET_GAN_CUTNO_REP(n): the pattern offset of a GAN object | Handled | — | [obj.rs:650](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L650) |
| `0x423` | Execute properties branch | Handled | — | [obj.rs:609](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L609) |
| `0x424` | OBJFRONTSET_FILTER_MOD / _MMX_USE / _QUAKE_USE: rendering switches of the original with no counterpart here | Empty handler | Original no-op status not established | [obj.rs:625](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L625) |
| `0x428` | Execute properties branch | Handled | — | [obj.rs:656](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L656) |
| `0x429` | OBJFRONTSET_BTNOFF / _BTNPUSHKEEP / _BTNACTIONNO | Handled | — | [obj.rs:627](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L627) |
| `0x42A` | Execute properties branch | Handled | — | [obj.rs:664](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L664) |
| `0x42B` | Execute properties branch | Handled | — | [obj.rs:628](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L628) |
| `0x42C` | Execute properties branch | Handled | — | [obj.rs:631](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L631) |
| `0x42E` | Execute properties branch | Handled | Guard: v.is_empty() | [obj.rs:665](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L665) |
| `0x42F` | Execute properties branch | Handled | Guard: v.is_empty() | [obj.rs:665](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L665) |
| `0x430` | OBJFRONTSET_ERIA / _OWNERIA / _BOXOWNERIA(x1, y1, x2, y2), _RECTOWNERIA(x, y, w, h); no arguments clear | Handled | Guard: v.is_empty() | [obj.rs:634](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L634) |
| `0x431` | OBJFRONTSET_FILTER_MOD / _MMX_USE / _QUAKE_USE: rendering switches of the original with no counterpart here | Empty handler | Original no-op status not established | [obj.rs:625](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L625) |
| `0x432` | OBJFRONTSET_FILTER_MOD / _MMX_USE / _QUAKE_USE: rendering switches of the original with no counterpart here | Empty handler | Original no-op status not established | [obj.rs:625](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L625) |

## Module commands / vm/modules/sel / dispatch

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x04` | Execute dispatch branch | Handled | — | [sel.rs:20](../../crates/crysirror-reallive/src/vm/modules/sel.rs#L20) |
| `0x0E` | Execute dispatch branch | Handled | — | [sel.rs:20](../../crates/crysirror-reallive/src/vm/modules/sel.rs#L20) |
| `0x14` | objbtn_init: nothing to prepare | Empty handler | Original no-op status not established | [sel.rs:27](../../crates/crysirror-reallive/src/vm/modules/sel.rs#L27) |
| `0x15` | select_btnobjinitall / select_btnobjend | Handled | — | [sel.rs:29](../../crates/crysirror-reallive/src/vm/modules/sel.rs#L29) |
| `0x16` | select_btnobjstart(group) / _with_right_click(group) | Handled | — | [sel.rs:35](../../crates/crysirror-reallive/src/vm/modules/sel.rs#L35) |
| `0x17` | select_btnobjinitall / select_btnobjend | Handled | — | [sel.rs:29](../../crates/crysirror-reallive/src/vm/modules/sel.rs#L29) |
| `0x1E` | select_btnobjnow_hit / select_btnobjnow_decide | Handled | — | [sel.rs:46](../../crates/crysirror-reallive/src/vm/modules/sel.rs#L46) |
| `0x1F` | select_btnobjnow_push (_left, _right) / _decide (_left, _right) | Handled | — | [sel.rs:54](../../crates/crysirror-reallive/src/vm/modules/sel.rs#L54) |
| `0x20` | Update store | Handled | — | [sel.rs:62](../../crates/crysirror-reallive/src/vm/modules/sel.rs#L62) |
| `0x21` | select_btnobjnow_push (_left, _right) / _decide (_left, _right) | Handled | — | [sel.rs:54](../../crates/crysirror-reallive/src/vm/modules/sel.rs#L54) |
| `0x22` | Update store | Handled | — | [sel.rs:62](../../crates/crysirror-reallive/src/vm/modules/sel.rs#L62) |
| `0x23` | select_btnobjnow_push (_left, _right) / _decide (_left, _right) | Handled | — | [sel.rs:54](../../crates/crysirror-reallive/src/vm/modules/sel.rs#L54) |
| `0x24` | Update store | Handled | — | [sel.rs:62](../../crates/crysirror-reallive/src/vm/modules/sel.rs#L62) |
| `0x7A` | select_btnobjstart(group) / _with_right_click(group) | Handled | — | [sel.rs:35](../../crates/crysirror-reallive/src/vm/modules/sel.rs#L35) |

## Module commands / vm/modules/str / dispatch

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `(0x00, overload) strcpy` | Copy a string | Handled | — | [str.rs:82](../../crates/crysirror-reallive/src/vm/modules/str.rs#L82) |
| `(0x01, 0) strclear` | Clear a string | Handled | — | [str.rs:92](../../crates/crysirror-reallive/src/vm/modules/str.rs#L92) |
| `(0x01, _) strclear` | Clear a string | Handled | — | [str.rs:96](../../crates/crysirror-reallive/src/vm/modules/str.rs#L96) |
| `(0x02, _) strcat` | Append a string | Handled | — | [str.rs:107](../../crates/crysirror-reallive/src/vm/modules/str.rs#L107) |
| `(0x03, _) strlen` | String length | Handled | — | [str.rs:114](../../crates/crysirror-reallive/src/vm/modules/str.rs#L114) |
| `(0x04, _) strcmp` | Compare strings | Handled | — | [str.rs:119](../../crates/crysirror-reallive/src/vm/modules/str.rs#L119) |
| `(0x05, overload) strsub` | strsub(dest, src, offset[, length]) / strrsub | Handled | — | [str.rs:130](../../crates/crysirror-reallive/src/vm/modules/str.rs#L130) |
| `(0x06, overload) strrsub` | strsub(dest, src, offset[, length]) / strrsub | Handled | — | [str.rs:130](../../crates/crysirror-reallive/src/vm/modules/str.rs#L130) |
| `(0x07, _) strcharlen` | strcharlen(s) | Handled | — | [str.rs:152](../../crates/crysirror-reallive/src/vm/modules/str.rs#L152) |
| `(0x08, _) strtrunc` | strtrunc(dest, chars) | Handled | — | [str.rs:157](../../crates/crysirror-reallive/src/vm/modules/str.rs#L157) |
| `(0x0A, overload) hantozen` | hantozen / zentohan / Uppercase / Lowercase: (dest) or (src, dest) | Handled | — | [str.rs:164](../../crates/crysirror-reallive/src/vm/modules/str.rs#L164) |
| `(0x0B, overload) zentohan` | hantozen / zentohan / Uppercase / Lowercase: (dest) or (src, dest) | Handled | — | [str.rs:164](../../crates/crysirror-reallive/src/vm/modules/str.rs#L164) |
| `(0x0C, overload) Uppercase` | hantozen / zentohan / Uppercase / Lowercase: (dest) or (src, dest) | Handled | — | [str.rs:164](../../crates/crysirror-reallive/src/vm/modules/str.rs#L164) |
| `(0x0D, overload) Lowercase` | hantozen / zentohan / Uppercase / Lowercase: (dest) or (src, dest) | Handled | — | [str.rs:164](../../crates/crysirror-reallive/src/vm/modules/str.rs#L164) |
| `(0x0E, overload) itoa_ws` | itoa_ws / itoa_s / itoa_w / itoa: (value, dest[, length]) | Handled | — | [str.rs:183](../../crates/crysirror-reallive/src/vm/modules/str.rs#L183) |
| `(0x0F, overload) itoa_s` | itoa_ws / itoa_s / itoa_w / itoa: (value, dest[, length]) | Handled | — | [str.rs:183](../../crates/crysirror-reallive/src/vm/modules/str.rs#L183) |
| `(0x10, overload) itoa_w` | itoa_ws / itoa_s / itoa_w / itoa: (value, dest[, length]) | Handled | — | [str.rs:183](../../crates/crysirror-reallive/src/vm/modules/str.rs#L183) |
| `(0x11, overload) itoa` | itoa_ws / itoa_s / itoa_w / itoa: (value, dest[, length]) | Handled | — | [str.rs:183](../../crates/crysirror-reallive/src/vm/modules/str.rs#L183) |
| `(0x12, _) atoi` | atoi(s) | Handled | — | [str.rs:203](../../crates/crysirror-reallive/src/vm/modules/str.rs#L203) |
| `(0x13, _) digits` | digits(n) | Handled | — | [str.rs:208](../../crates/crysirror-reallive/src/vm/modules/str.rs#L208) |
| `(0x14, _) digit` | digit(n, dest, index): the index-th digit from the right | Handled | — | [str.rs:213](../../crates/crysirror-reallive/src/vm/modules/str.rs#L213) |
| `(0x1E, _) strpos` | strpos / strlpos | Handled | — | [str.rs:226](../../crates/crysirror-reallive/src/vm/modules/str.rs#L226) |
| `(0x1F, _) strlpos` | strpos / strlpos | Handled | — | [str.rs:226](../../crates/crysirror-reallive/src/vm/modules/str.rs#L226) |
| `(0x20, _)` | Update store | Handled | — | [str.rs:242](../../crates/crysirror-reallive/src/vm/modules/str.rs#L242) |
| `(0x21, _)` | Update store | Handled | — | [str.rs:242](../../crates/crysirror-reallive/src/vm/modules/str.rs#L242) |
| `(0x28, _)` | STRCOPY_BUFSIZE(dest, src, size) / STRSET_BUFSIZE(buf, size): as if through a C buffer of `size` bytes (terminator incl… | Handled | — | [str.rs:270](../../crates/crysirror-reallive/src/vm/modules/str.rs#L270) |
| `(0x29, _)` | STRCOPY_BUFSIZE(dest, src, size) / STRSET_BUFSIZE(buf, size): as if through a C buffer of `size` bytes (terminator incl… | Handled | — | [str.rs:270](../../crates/crysirror-reallive/src/vm/modules/str.rs#L270) |
| `(0x64, _) strout` | Text output is an external engine operation. Keep the original command/overload and PC until the semantic adapter ackno… | Handled | — | [str.rs:296](../../crates/crysirror-reallive/src/vm/modules/str.rs#L296) |
| `(0xC8, _) strused` | strused(s) | Handled | — | [str.rs:298](../../crates/crysirror-reallive/src/vm/modules/str.rs#L298) |

## Module commands / vm/modules/str / dispatch / (op.opcode, op.overload)=(10..=13, overload)

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x0A hantozen` | hantozen | Handled | — | [str.rs:175](../../crates/crysirror-reallive/src/vm/modules/str.rs#L175) |
| `0x0B zentohan` | zentohan | Handled | — | [str.rs:176](../../crates/crysirror-reallive/src/vm/modules/str.rs#L176) |
| `0x0C Uppercase` | Uppercase | Handled | — | [str.rs:177](../../crates/crysirror-reallive/src/vm/modules/str.rs#L177) |

## Module commands / vm/modules/sys / dispatch

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0xC8` | KeyMouseOn / KeyMouseOff | Handled | — | [sys.rs:192](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L192) |
| `0xC9` | System commands (dispatch) | Handled | — | [sys.rs:193](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L193) |
| `0xCE` | System commands (dispatch) | Handled | — | [sys.rs:194](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L194) |
| `0xCF` | System commands (dispatch) | Handled | — | [sys.rs:195](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L195) |
| `0x12C` | CLEAR_MSGBK / SET_MSGBK_ON / SET_MSGBK_OFF | Empty handler | Original no-op status not established | [sys.rs:178](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L178) |
| `0x12D` | System commands (dispatch) | Handled | — | [sys.rs:179](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L179) |
| `0x12E` | System commands (dispatch) | Handled | — | [sys.rs:180](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L180) |
| `0x141` | CallStackNop([count]) | Handled | — | [sys.rs:197](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L197) |
| `0x142` | CallStackPop([count]): returns `count` levels | Handled | — | [sys.rs:211](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L211) |
| `0x143` | System commands (dispatch) | Handled | — | [sys.rs:217](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L217) |
| `0x144` | CallStackTrunc(length) | Handled | — | [sys.rs:219](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L219) |
| `0x14A` | EnableSkipMode / DisableSkipMode | Handled | — | [sys.rs:227](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L227) |
| `0x14B` | System commands (dispatch) | Handled | — | [sys.rs:228](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L228) |
| `0x14C` | LocalSkipMode / SetLocalSkipMode / ClearLocalSkipMode | Handled | — | [sys.rs:230](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L230) |
| `0x14D` | System commands (dispatch) | Handled | — | [sys.rs:231](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L231) |
| `0x14E` | System commands (dispatch) | Handled | — | [sys.rs:232](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L232) |
| `0x14F` | SET_READJUMPCANCEL_ON / _OFF | Handled | — | [sys.rs:182](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L182) |
| `0x150` | System commands (dispatch) | Handled | — | [sys.rs:183](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L183) |
| `0x15E` | CtrlKeySkip / On / Off / CtrlPressed / ShiftPressed | Handled | — | [sys.rs:234](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L234) |
| `0x15F` | System commands (dispatch) | Handled | — | [sys.rs:235](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L235) |
| `0x160` | System commands (dispatch) | Handled | — | [sys.rs:236](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L236) |
| `0x161` | System commands (dispatch) | Handled | — | [sys.rs:237](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L237) |
| `0x162` | System commands (dispatch) | Handled | — | [sys.rs:238](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L238) |
| `0x166` | SET_CTRLKEY_ENABLE / SET_CTRLKEY_DISENABLE | Handled | — | [sys.rs:185](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L185) |
| `0x167` | System commands (dispatch) | Handled | — | [sys.rs:186](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L186) |
| `0x16C` | PauseCursor(index) | Handled | — | [sys.rs:240](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L240) |
| `0x16E` | SET_P_CURSORNO(id) | Handled | — | [sys.rs:188](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L188) |
| `0x19A` | GetWakuAll / SetWakuAll | Handled | — | [sys.rs:242](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L242) |
| `0x19B` | System commands (dispatch) | Handled | — | [sys.rs:243](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L243) |
| `0x1A4` | GetWindowAttr2 / SetWindowAttr2: the global attributes | Handled | — | [sys.rs:245](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L245) |
| `0x1A5` | System commands (dispatch) | Handled | — | [sys.rs:249](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L249) |
| `0x1AE` | Undocumented get/enable/disable flag triples | Handled | — | [sys.rs:256](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L256) |
| `0x1AF` | Undocumented get/enable/disable flag triples | Handled | — | [sys.rs:256](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L256) |
| `0x1B0` | Undocumented get/enable/disable flag triples | Handled | — | [sys.rs:256](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L256) |
| `0x1B1` | Undocumented get/enable/disable flag triples | Handled | — | [sys.rs:256](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L256) |
| `0x1B2` | Undocumented get/enable/disable flag triples | Handled | — | [sys.rs:256](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L256) |
| `0x1B3` | Undocumented get/enable/disable flag triples | Handled | — | [sys.rs:256](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L256) |
| `0x1B4` | Undocumented get/enable/disable flag triples | Handled | — | [sys.rs:256](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L256) |
| `0x1B5` | Undocumented get/enable/disable flag triples | Handled | — | [sys.rs:256](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L256) |
| `0x1B6` | Undocumented get/enable/disable flag triples | Handled | — | [sys.rs:256](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L256) |
| `0x1B7` | Undocumented get/enable/disable flag triples | Handled | — | [sys.rs:256](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L256) |
| `0x1B8` | Undocumented get/enable/disable flag triples | Handled | — | [sys.rs:256](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L256) |
| `0x1B9` | Undocumented get/enable/disable flag triples | Handled | — | [sys.rs:256](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L256) |
| `0x1BA` | Undocumented get/enable/disable flag triples | Handled | — | [sys.rs:256](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L256) |
| `0x1BB` | Undocumented get/enable/disable flag triples | Handled | — | [sys.rs:256](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L256) |
| `0x1BC` | Undocumented get/enable/disable flag triples | Handled | — | [sys.rs:256](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L256) |
| `0x1BD` | Undocumented get/enable/disable flag triples | Handled | — | [sys.rs:256](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L256) |
| `0x1BE` | Undocumented get/enable/disable flag triples | Handled | — | [sys.rs:256](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L256) |
| `0x1BF` | Undocumented get/enable/disable flag triples | Handled | — | [sys.rs:256](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L256) |
| `0x1C0` | Undocumented get/enable/disable flag triples | Handled | — | [sys.rs:256](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L256) |
| `0x1C1` | Undocumented get/enable/disable flag triples | Handled | — | [sys.rs:256](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L256) |
| `0x1C2` | Undocumented get/enable/disable flag triples | Handled | — | [sys.rs:256](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L256) |
| `0x1C3` | Undocumented get/enable/disable flag triples | Handled | — | [sys.rs:256](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L256) |
| `0x1C4` | Undocumented get/enable/disable flag triples | Handled | — | [sys.rs:256](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L256) |
| `0x1C5` | Undocumented get/enable/disable flag triples | Handled | — | [sys.rs:256](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L256) |
| `0x1C6` | Undocumented get/enable/disable flag triples | Handled | — | [sys.rs:256](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L256) |
| `0x1C7` | Undocumented get/enable/disable flag triples | Handled | — | [sys.rs:256](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L256) |
| `0x1C8` | Undocumented get/enable/disable flag triples | Handled | — | [sys.rs:256](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L256) |
| `0x1C9` | Undocumented get/enable/disable flag triples | Handled | — | [sys.rs:256](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L256) |
| `0x1F4` | InitFrame* / InitExFrame* | Handled | — | [sys.rs:20](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L20) |
| `0x1F5` | InitFrame* / InitExFrame* | Handled | — | [sys.rs:20](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L20) |
| `0x1F6` | InitFrame* / InitExFrame* | Handled | — | [sys.rs:20](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L20) |
| `0x1F7` | InitFrame* / InitExFrame* | Handled | — | [sys.rs:20](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L20) |
| `0x1F8` | InitFrame* / InitExFrame* | Handled | — | [sys.rs:20](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L20) |
| `0x1FE` | ReadFrame / FrameActive / AnyFrameActive (and Ex) | Handled | — | [sys.rs:33](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L33) |
| `0x1FF` | System commands (dispatch) | Handled | — | [sys.rs:40](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L40) |
| `0x200` | System commands (dispatch) | Handled | — | [sys.rs:48](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L48) |
| `0x201` | ClearFrame(counter[, value]) / ClearAllFrames([value]) | Handled | — | [sys.rs:52](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L52) |
| `0x202` | System commands (dispatch) | Handled | — | [sys.rs:59](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L59) |
| `0x208` | InitFrame* / InitExFrame* | Handled | — | [sys.rs:20](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L20) |
| `0x209` | InitFrame* / InitExFrame* | Handled | — | [sys.rs:20](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L20) |
| `0x20A` | InitFrame* / InitExFrame* | Handled | — | [sys.rs:20](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L20) |
| `0x20B` | InitFrame* / InitExFrame* | Handled | — | [sys.rs:20](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L20) |
| `0x20C` | InitFrame* / InitExFrame* | Handled | — | [sys.rs:20](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L20) |
| `0x212` | ReadFrame / FrameActive / AnyFrameActive (and Ex) | Handled | — | [sys.rs:33](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L33) |
| `0x213` | System commands (dispatch) | Handled | — | [sys.rs:40](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L40) |
| `0x214` | System commands (dispatch) | Handled | — | [sys.rs:48](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L48) |
| `0x215` | ClearFrame(counter[, value]) / ClearAllFrames([value]) | Handled | — | [sys.rs:52](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L52) |
| `0x216` | System commands (dispatch) | Handled | — | [sys.rs:59](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L59) |
| `0x258` | System commands (dispatch) | Handled | — | [sys.rs:64](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L64) |
| `0x259` | System commands (dispatch) | Handled | — | [sys.rs:64](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L64) |
| `0x25A` | System commands (dispatch) | Handled | — | [sys.rs:64](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L64) |
| `0x25B` | System commands (dispatch) | Handled | — | [sys.rs:64](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L64) |
| `0x25C` | System commands (dispatch) | Handled | — | [sys.rs:64](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L64) |
| `0x262` | System commands (dispatch) | Handled | — | [sys.rs:82](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L82) |
| `0x26C` | System commands (dispatch) | Handled | — | [sys.rs:64](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L64) |
| `0x26D` | System commands (dispatch) | Handled | — | [sys.rs:64](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L64) |
| `0x26E` | System commands (dispatch) | Handled | — | [sys.rs:64](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L64) |
| `0x26F` | System commands (dispatch) | Handled | — | [sys.rs:64](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L64) |
| `0x270` | System commands (dispatch) | Handled | — | [sys.rs:64](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L64) |
| `0x276` | System commands (dispatch) | Handled | — | [sys.rs:82](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L82) |
| `0x320` | System commands (dispatch) | Handled | — | [sys.rs:98](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L98) |
| `0x32A` | TIMETABLE2 / TIMETABLELEN2 | Handled | — | [sys.rs:190](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L190) |
| `0x32B` | TIMETABLE2 / TIMETABLELEN2 | Handled | — | [sys.rs:190](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L190) |
| `0x3E8` | rnd([min], max) | Handled | — | [sys.rs:100](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L100) |
| `0x3E9` | pcnt(n, d) | Handled | — | [sys.rs:109](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L109) |
| `0x3EA` | System commands (dispatch) | Handled | — | [sys.rs:118](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L118) |
| `0x3EB` | power(base[, exponent]) | Handled | — | [sys.rs:120](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L120) |
| `0x3EC` | sin / cos (value[, divisor]) and the undocumented variants | Handled | — | [sys.rs:130](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L130) |
| `0x3ED` | modulus(x1, y1, x2, y2) / angle(...) | Handled | — | [sys.rs:141](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L141) |
| `0x3EE` | modulus(x1, y1, x2, y2) / angle(...) | Handled | — | [sys.rs:141](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L141) |
| `0x3EF` | System commands (dispatch) | Handled | — | [sys.rs:153](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L153) |
| `0x3F0` | System commands (dispatch) | Handled | — | [sys.rs:158](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L158) |
| `0x3F1` | constrain(min, value, max) | Handled | — | [sys.rs:164](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L164) |
| `0x3F2` | sin / cos (value[, divisor]) and the undocumented variants | Handled | — | [sys.rs:130](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L130) |
| `0x3F3` | System commands (dispatch) | Handled | — | [sys.rs:170](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L170) |
| `0x3F4` | sin / cos (value[, divisor]) and the undocumented variants | Handled | — | [sys.rs:130](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L130) |
| `0x3F5` | sin / cos (value[, divisor]) and the undocumented variants | Handled | — | [sys.rs:130](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L130) |
| `0x44C` | System commands (dispatch) | Handled | — | [sys.rs:171](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L171) |
| `0x44D` | System commands (dispatch) | Handled | — | [sys.rs:171](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L171) |
| `0x44E` | System commands (dispatch) | Handled | — | [sys.rs:171](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L171) |
| `0x44F` | System commands (dispatch) | Handled | — | [sys.rs:171](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L171) |
| `0x450` | System commands (dispatch) | Handled | — | [sys.rs:171](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L171) |
| `0x451` | System commands (dispatch) | Handled | — | [sys.rs:171](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L171) |
| `0x452` | System commands (dispatch) | Handled | — | [sys.rs:171](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L171) |
| `0x453` | System commands (dispatch) | Handled | — | [sys.rs:171](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L171) |
| `0x456` | GetDate / GetTime / GetDateTime | Handled | — | [sys.rs:173](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L173) |
| `0x457` | System commands (dispatch) | Handled | — | [sys.rs:174](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L174) |
| `0x458` | System commands (dispatch) | Handled | — | [sys.rs:175](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L175) |
| `0x460` | System commands (dispatch) | Handled | — | [sys.rs:176](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L176) |
| `0x46B` | System commands (dispatch) | Handled | — | [sys.rs:352](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L352) |
| `0x46D` | System commands (dispatch) | Handled | — | [sys.rs:353](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L353) |
| `0x4B0` | end() | Handled | — | [sys.rs:348](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L348) |
| `0x4BB` | EnableSyscom([n]) / HideSyscom([n]) / DisableSyscom(n) | Handled | — | [sys.rs:278](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L278) |
| `0x4BC` | EnableSyscom([n]) / HideSyscom([n]) / DisableSyscom(n) | Handled | — | [sys.rs:278](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L278) |
| `0x4BD` | System commands (dispatch) | Handled | — | [sys.rs:288](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L288) |
| `0x4BE` | System commands (dispatch) | Handled | — | [sys.rs:292](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L292) |
| `0x4C0` | System commands (dispatch) | Handled | — | [sys.rs:296](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L296) |
| `0x4C2` | CALL_SYSTEMMENU_MOVIESUPPORT / CALL_SYSTEMMENU_MMXSUPPORT: the original's hardware-support dialogs; nothing to configur… | Empty handler | Original no-op status not established | [sys.rs:272](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L272) |
| `0x4C5` | SET_SELPOINTMOD_ON / _OFF, SELPOINT, SET_SELPOINT_CLEAR | Handled | — | [sys.rs:274](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L274) |
| `0x4C6` | System commands (dispatch) | Handled | — | [sys.rs:275](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L275) |
| `0x4CF` | System commands (dispatch) | Handled | — | [sys.rs:276](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L276) |
| `0x4EC` | CALL_SYSTEMMENU_MOVIESUPPORT / CALL_SYSTEMMENU_MMXSUPPORT: the original's hardware-support dialogs; nothing to configur… | Empty handler | Original no-op status not established | [sys.rs:272](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L272) |
| `0x7D0` | Miscellaneous flags: getters 2000..2009, setters 2050..2059 | Handled | — | [sys.rs:304](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L304) |
| `0x7D1` | Miscellaneous flags: getters 2000..2009, setters 2050..2059 | Handled | — | [sys.rs:304](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L304) |
| `0x7D2` | Miscellaneous flags: getters 2000..2009, setters 2050..2059 | Handled | — | [sys.rs:304](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L304) |
| `0x7D3` | Miscellaneous flags: getters 2000..2009, setters 2050..2059 | Handled | — | [sys.rs:304](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L304) |
| `0x7D4` | Miscellaneous flags: getters 2000..2009, setters 2050..2059 | Handled | — | [sys.rs:304](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L304) |
| `0x7D5` | Miscellaneous flags: getters 2000..2009, setters 2050..2059 | Handled | — | [sys.rs:304](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L304) |
| `0x7D6` | Miscellaneous flags: getters 2000..2009, setters 2050..2059 | Handled | — | [sys.rs:304](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L304) |
| `0x7D7` | Miscellaneous flags: getters 2000..2009, setters 2050..2059 | Handled | — | [sys.rs:304](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L304) |
| `0x7D8` | Miscellaneous flags: getters 2000..2009, setters 2050..2059 | Handled | — | [sys.rs:304](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L304) |
| `0x7D9` | Miscellaneous flags: getters 2000..2009, setters 2050..2059 | Handled | — | [sys.rs:304](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L304) |
| `0x802` | Miscellaneous flags: getters 2000..2009, setters 2050..2059 | Handled | — | [sys.rs:304](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L304) |
| `0x803` | Miscellaneous flags: getters 2000..2009, setters 2050..2059 | Handled | — | [sys.rs:304](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L304) |
| `0x804` | Miscellaneous flags: getters 2000..2009, setters 2050..2059 | Handled | — | [sys.rs:304](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L304) |
| `0x805` | Miscellaneous flags: getters 2000..2009, setters 2050..2059 | Handled | — | [sys.rs:304](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L304) |
| `0x806` | Miscellaneous flags: getters 2000..2009, setters 2050..2059 | Handled | — | [sys.rs:304](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L304) |
| `0x807` | Miscellaneous flags: getters 2000..2009, setters 2050..2059 | Handled | — | [sys.rs:304](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L304) |
| `0x808` | Miscellaneous flags: getters 2000..2009, setters 2050..2059 | Handled | — | [sys.rs:304](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L304) |
| `0x809` | Miscellaneous flags: getters 2000..2009, setters 2050..2059 | Handled | — | [sys.rs:304](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L304) |
| `0x80A` | Miscellaneous flags: getters 2000..2009, setters 2050..2059 | Handled | — | [sys.rs:304](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L304) |
| `0x80B` | Miscellaneous flags: getters 2000..2009, setters 2050..2059 | Handled | — | [sys.rs:304](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L304) |
| `0x8AD` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x8AE` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x8AF` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x8B0` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x8B1` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x8B2` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x8B3` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x8B4` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x8B5` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x8B6` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x8B7` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x8B8` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x8B9` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x8BA` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x8BB` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x8BC` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x8BD` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x8BE` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x8BF` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x8C0` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x8C1` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x8C2` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x8C3` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x8C4` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x8C5` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x8C6` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x8C7` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x8C8` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x8C9` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x8CA` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x8CB` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x8CC` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x8CD` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x8CE` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x8CF` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x8D0` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x8D1` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x8D2` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x8D3` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x8D4` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x8D5` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x8D6` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x8D7` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x8D8` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x8D9` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x8DA` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x8DB` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x8DC` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x8DD` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x8DE` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x8DF` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x8E0` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x8E1` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x8E2` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x8E3` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x8E4` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x911` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x912` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x913` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x914` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x915` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x916` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x917` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x918` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x919` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x91A` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x91B` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x91C` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x91D` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x91E` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x91F` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x920` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x921` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x922` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x923` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x924` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x925` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x926` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x927` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x928` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x929` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x92A` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x92B` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x92C` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x92D` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x92E` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x92F` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x930` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x931` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x932` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x933` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x934` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x935` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x936` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x937` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x938` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x939` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x93A` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x93B` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x93C` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x93D` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x93E` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x93F` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x940` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x941` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x942` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x943` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x944` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x945` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x946` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x947` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x948` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0x962` | ENABLE_SYSTEMMENU_ANIME / DISENABLE_SYSTEMMENU_ANIME | Handled | — | [sys.rs:301](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L301) |
| `0x9C6` | System commands (dispatch) | Handled | — | [sys.rs:302](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L302) |
| `0xA28` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0xA29` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0xA2A` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0xA2B` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0xA2C` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0xA2D` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0xA2E` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0xA2F` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0xA30` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0xA31` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0xA32` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0xA33` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0xA34` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0xA35` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0xA36` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0xA37` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0xA38` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0xA39` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0xA3A` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0xA3B` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0xA3C` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0xA3D` | System commands (dispatch) | Handled | — | [sys.rs:354](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L354) |
| `0xDAC` | Savepoint / EnableAutoSavepoints / DisableAutoSavepoints | Handled | — | [sys.rs:344](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L344) |
| `0xDAD` | System commands (dispatch) | Handled | — | [sys.rs:345](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L345) |
| `0xDAE` | System commands (dispatch) | Handled | — | [sys.rs:346](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L346) |
| `0xDAF` | SAVEPOINT_CLEAR: forget the old savepoint; saving now keeps this position | Handled | — | [sys.rs:342](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L342) |

## Module commands / vm/modules/sys / dispatch / op.opcode=430..=457

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x00` | System commands (dispatch) | Handled | — | [sys.rs:264](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L264) |
| `0x01` | System commands (dispatch) | Handled | — | [sys.rs:265](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L265) |
| `0x02` | System commands (dispatch) | Handled | — | [sys.rs:266](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L266) |

## Module commands / vm/modules/sys / settings_op

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x8AD` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8AE` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8AF` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8B0` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8B1` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8B2` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8B3` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8B4` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8B5` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8B6` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8B7` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8B8` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8B9` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8BA` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8BB` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8BC` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8BD` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8BE` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8BF` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8C0` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8C1` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8C2` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8C3` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8C4` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8C5` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8C6` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8C7` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8C8` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8C9` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8CA` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8CB` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8CC` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8CD` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8CE` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8CF` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8D0` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8D1` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8D2` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8D3` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8D4` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8D5` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8D6` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8D7` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8D8` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8D9` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8DA` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8DB` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8DC` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8DD` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8DE` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8DF` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8E0` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8E1` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8E2` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8E3` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8E4` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8E5` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8E6` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8E7` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8E8` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8E9` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8EA` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8EB` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8EC` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8ED` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8EE` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8EF` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8F0` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8F1` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8F2` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8F3` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8F4` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8F5` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8F6` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8F7` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8F8` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8F9` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8FA` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x8FB` | System commands (settings_op) | Handled | — | [sys.rs:534](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L534) |
| `0x911` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x912` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x913` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x914` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x915` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x916` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x917` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x918` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x919` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x91A` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x91B` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x91C` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x91D` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x91E` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x91F` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x920` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x921` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x922` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x923` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x924` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x925` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x926` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x927` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x928` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x929` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x92A` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x92B` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x92C` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x92D` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x92E` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x92F` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x930` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x931` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x932` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x933` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x934` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x935` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x936` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x937` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x938` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x939` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x93A` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x93B` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x93C` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x93D` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x93E` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x93F` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x940` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x941` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x942` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x943` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x944` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x945` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x946` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x947` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x948` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x949` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x94A` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x94B` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x94C` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x94D` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x94E` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x94F` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x950` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x951` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x952` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x953` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x954` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x955` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x956` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x957` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x958` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x959` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x95A` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x95B` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x95C` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x95D` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x95E` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |
| `0x95F` | System commands (settings_op) | Handled | — | [sys.rs:535](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L535) |

## Module commands / vm/modules/sys / settings_op / receiver=kind == 2

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0xA28` | System commands (settings_op) | Handled | — | [sys.rs:541](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L541) |
| `0xA29` | System commands (settings_op) | Handled | — | [sys.rs:542](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L542) |
| `0xA2A` | System commands (settings_op) | Handled | — | [sys.rs:543](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L543) |
| `0xA2B` | System commands (settings_op) | Handled | — | [sys.rs:544](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L544) |
| `0xA2C` | System commands (settings_op) | Handled | — | [sys.rs:545](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L545) |
| `0xA2D` | System commands (settings_op) | Handled | — | [sys.rs:546](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L546) |
| `0xA2E` | System commands (settings_op) | Handled | — | [sys.rs:547](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L547) |
| `0xA32` | System commands (settings_op) | Handled | — | [sys.rs:548](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L548) |
| `0xA33` | System commands (settings_op) | Handled | — | [sys.rs:548](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L548) |
| `0xA34` | System commands (settings_op) | Handled | — | [sys.rs:548](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L548) |
| `0xA35` | System commands (settings_op) | Handled | — | [sys.rs:548](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L548) |
| `0xA36` | System commands (settings_op) | Handled | — | [sys.rs:548](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L548) |
| `0xA37` | System commands (settings_op) | Handled | — | [sys.rs:549](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L549) |
| `0xA38` | System commands (settings_op) | Handled | — | [sys.rs:549](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L549) |
| `0xA39` | System commands (settings_op) | Handled | — | [sys.rs:549](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L549) |
| `0xA3C` | System commands (settings_op) | Handled | — | [sys.rs:550](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L550) |
| `0xA3D` | System commands (settings_op) | Handled | — | [sys.rs:551](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L551) |

## Module commands / vm/modules/sys_max / cg_query

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x5DC` | System commands (cg_query) | Handled | — | [sys_max.rs:459](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L459) |
| `0x5DD` | System commands (cg_query) | Handled | — | [sys_max.rs:460](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L460) |
| `0x5DE` | System commands (cg_query) | Handled | — | [sys_max.rs:461](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L461) |

## Module commands / vm/modules/sys_max / cg_table

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x5E1` | System commands (cg_table) | Handled | — | [sys_max.rs:488](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L488) |
| `0x5E2` | System commands (cg_table) | Handled | — | [sys_max.rs:488](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L488) |
| `0x5E3` | System commands (cg_table) | Handled | — | [sys_max.rs:500](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L500) |
| `0x5E4` | System commands (cg_table) | Handled | — | [sys_max.rs:500](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L500) |
| `0x5E5` | GET_CGTABLE(count, names, flags) / GET_CGTABLE_FILENAME(flag, name) | Handled | — | [sys_max.rs:533](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L533) |
| `0x708` | System commands (cg_table) | Handled | — | [sys_max.rs:488](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L488) |
| `0x709` | System commands (cg_table) | Handled | — | [sys_max.rs:488](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L488) |
| `0x70A` | System commands (cg_table) | Handled | — | [sys_max.rs:488](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L488) |
| `0x70B` | System commands (cg_table) | Handled | — | [sys_max.rs:488](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L488) |
| `0x70C` | System commands (cg_table) | Handled | — | [sys_max.rs:488](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L488) |
| `0x70D` | System commands (cg_table) | Handled | — | [sys_max.rs:488](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L488) |
| `0x70E` | System commands (cg_table) | Handled | — | [sys_max.rs:488](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L488) |
| `0x712` | System commands (cg_table) | Handled | — | [sys_max.rs:501](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L501) |
| `0x713` | System commands (cg_table) | Handled | — | [sys_max.rs:501](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L501) |
| `0x714` | System commands (cg_table) | Handled | — | [sys_max.rs:501](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L501) |
| `0x715` | System commands (cg_table) | Handled | — | [sys_max.rs:501](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L501) |
| `0x716` | System commands (cg_table) | Handled | — | [sys_max.rs:501](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L501) |
| `0x717` | System commands (cg_table) | Handled | — | [sys_max.rs:501](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L501) |
| `0x718` | System commands (cg_table) | Handled | — | [sys_max.rs:501](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L501) |
| `0x71C` | System commands (cg_table) | Handled | — | [sys_max.rs:508](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L508) |
| `0x71D` | System commands (cg_table) | Handled | — | [sys_max.rs:508](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L508) |
| `0x71E` | System commands (cg_table) | Handled | — | [sys_max.rs:508](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L508) |
| `0x71F` | System commands (cg_table) | Handled | — | [sys_max.rs:508](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L508) |
| `0x720` | System commands (cg_table) | Handled | — | [sys_max.rs:508](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L508) |
| `0x721` | System commands (cg_table) | Handled | — | [sys_max.rs:508](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L508) |
| `0x722` | System commands (cg_table) | Handled | — | [sys_max.rs:508](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L508) |
| `0x726` | System commands (cg_table) | Handled | — | [sys_max.rs:515](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L515) |
| `0x727` | System commands (cg_table) | Handled | — | [sys_max.rs:515](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L515) |
| `0x728` | System commands (cg_table) | Handled | — | [sys_max.rs:515](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L515) |
| `0x729` | System commands (cg_table) | Handled | — | [sys_max.rs:515](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L515) |
| `0x72A` | System commands (cg_table) | Handled | — | [sys_max.rs:515](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L515) |
| `0x72B` | System commands (cg_table) | Handled | — | [sys_max.rs:515](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L515) |
| `0x72C` | System commands (cg_table) | Handled | — | [sys_max.rs:515](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L515) |
| `0x72D` | System commands (cg_table) | Handled | — | [sys_max.rs:515](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L515) |
| `0x72E` | System commands (cg_table) | Handled | — | [sys_max.rs:515](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L515) |
| `0x72F` | System commands (cg_table) | Handled | — | [sys_max.rs:515](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L515) |
| `0x730` | System commands (cg_table) | Handled | — | [sys_max.rs:515](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L515) |
| `0x731` | System commands (cg_table) | Handled | — | [sys_max.rs:515](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L515) |
| `0x732` | System commands (cg_table) | Handled | — | [sys_max.rs:515](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L515) |
| `0x733` | System commands (cg_table) | Handled | — | [sys_max.rs:515](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L515) |
| `0x734` | System commands (cg_table) | Handled | — | [sys_max.rs:515](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L515) |
| `0x735` | System commands (cg_table) | Handled | — | [sys_max.rs:515](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L515) |
| `0x736` | System commands (cg_table) | Handled | — | [sys_max.rs:515](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L515) |
| `0x737` | System commands (cg_table) | Handled | — | [sys_max.rs:515](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L515) |
| `0x738` | System commands (cg_table) | Handled | — | [sys_max.rs:515](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L515) |
| `0x739` | System commands (cg_table) | Handled | — | [sys_max.rs:515](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L515) |
| `0x73A` | System commands (cg_table) | Handled | — | [sys_max.rs:515](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L515) |
| `0x73B` | System commands (cg_table) | Handled | — | [sys_max.rs:515](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L515) |
| `0x73C` | System commands (cg_table) | Handled | — | [sys_max.rs:515](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L515) |
| `0x73D` | System commands (cg_table) | Handled | — | [sys_max.rs:515](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L515) |

## Module commands / vm/modules/sys_max / dispatch

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x01` | SLEEP: give up the rest of the frame | Handled | — | [sys_max.rs:133](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L133) |
| `0x86` | System commands (dispatch) | Handled | — | [sys_max.rs:137](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L137) |
| `0x87` | System commands (dispatch) | Handled | — | [sys_max.rs:138](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L138) |
| `0x88` | System commands (dispatch) | Handled | — | [sys_max.rs:138](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L138) |
| `0x89` | System commands (dispatch) | Handled | — | [sys_max.rs:138](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L138) |
| `0x8B` | MOUSEWHEELCLEAR / MOUSEWHEELCHECK (-1 up, 1 down) / ..CNT | Handled | — | [sys_max.rs:156](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L156) |
| `0x8C` | System commands (dispatch) | Handled | — | [sys_max.rs:157](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L157) |
| `0x8D` | System commands (dispatch) | Handled | — | [sys_max.rs:158](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L158) |
| `0x96` | CLEAR_KEYTABLE([code...]): forget presses (of those keys) | Handled | — | [sys_max.rs:160](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L160) |
| `0x97` | GET_KEYTABLE_DATA / GET_KEYTABLE_REAL(buffer, code...): per key, pressed since cleared / held now, into consecutive var… | Handled | — | [sys_max.rs:173](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L173) |
| `0x98` | GET_KEYTABLE_DATA / GET_KEYTABLE_REAL(buffer, code...): per key, pressed since cleared / held now, into consecutive var… | Handled | — | [sys_max.rs:173](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L173) |
| `0xA0` | SET_MOUSEAREA / CLR_MOUSEAREA: the pointer is confined there | Handled | — | [sys_max.rs:200](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L200) |
| `0xA1` | System commands (dispatch) | Handled | — | [sys_max.rs:206](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L206) |
| `0xCB` | SetCursorPos(x, y): the host warps the pointer | Handled | — | [sys_max.rs:208](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L208) |
| `0x136` | ALLSET_READFLAG / ALLCLR_READFLAG: every line read / unread | Handled | — | [sys_max.rs:215](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L215) |
| `0x137` | System commands (dispatch) | Handled | — | [sys_max.rs:216](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L216) |
| `0x156` | GET_RETURNWAIT (the second number for it) | Handled | — | [sys_max.rs:289](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L289) |
| `0x163` | GET_JUMPSTATE / SET_JUMP_ENABLE / SET_JUMP_DISENABLE (skip mode) | Handled | — | [sys_max.rs:221](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L221) |
| `0x164` | System commands (dispatch) | Handled | — | [sys_max.rs:222](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L222) |
| `0x165` | System commands (dispatch) | Handled | — | [sys_max.rs:223](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L223) |
| `0x16B` | GET_R_CURSORNO / GET_P_CURSORNO | Handled | — | [sys_max.rs:225](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L225) |
| `0x16D` | GET_R_CURSORNO / GET_P_CURSORNO | Handled | — | [sys_max.rs:225](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L225) |
| `0x1CA` | GET/SET_WINDOW_EXBTN_MOD(button[, mod]): extra window buttons | Handled | — | [sys_max.rs:227](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L227) |
| `0x1CB` | System commands (dispatch) | Handled | — | [sys_max.rs:231](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L231) |
| `0x2EA` | READJUMP_START_CHECK: skip mode is on | Handled | — | [sys_max.rs:250](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L250) |
| `0x30C` | GET/SET_LAST_KOE_COMMAND(koe, ctrl), GET_KOE_REPLAY(koe, ctrl): the last voice, for replaying | Handled | — | [sys_max.rs:253](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L253) |
| `0x30D` | System commands (dispatch) | Handled | — | [sys_max.rs:258](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L258) |
| `0x31D` | GET/SET_LAST_KOE_COMMAND(koe, ctrl), GET_KOE_REPLAY(koe, ctrl): the last voice, for replaying | Handled | — | [sys_max.rs:253](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L253) |
| `0x321` | TIMETABLELEN: index_series with lengths | Handled | — | [sys_max.rs:264](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L264) |
| `0x3F6` | LOG2 / BITSET / BITCLR / BITCHECK(var, bit) | Handled | — | [sys_max.rs:266](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L266) |
| `0x3F7` | System commands (dispatch) | Handled | — | [sys_max.rs:274](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L274) |
| `0x3F8` | System commands (dispatch) | Handled | — | [sys_max.rs:274](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L274) |
| `0x3F9` | System commands (dispatch) | Handled | — | [sys_max.rs:274](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L274) |
| `0x461` | GET_LINENO | Handled | — | [sys_max.rs:287](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L287) |
| `0x4C4` | GET_SELPOINTMOD / SELPOINT_CHECK | Handled | — | [sys_max.rs:301](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L301) |
| `0x4D0` | System commands (dispatch) | Handled | — | [sys_max.rs:302](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L302) |
| `0x518` | NAMEGET_DEFAULT / NAMESET_DEFAULT(index) and local forms: the names #NAME / #LOCALNAME give | Handled | — | [sys_max.rs:305](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L305) |
| `0x519` | NAMEGET_DEFAULT / NAMESET_DEFAULT(index) and local forms: the names #NAME / #LOCALNAME give | Handled | — | [sys_max.rs:305](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L305) |
| `0x522` | NAMEGET_DEFAULT / NAMESET_DEFAULT(index) and local forms: the names #NAME / #LOCALNAME give | Handled | — | [sys_max.rs:305](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L305) |
| `0x523` | NAMEGET_DEFAULT / NAMESET_DEFAULT(index) and local forms: the names #NAME / #LOCALNAME give | Handled | — | [sys_max.rs:305](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L305) |
| `0x528` | GET_NAME_MAXLEN | Handled | — | [sys_max.rs:317](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L317) |
| `0x5DC` | CG mode (original cgtable::sys_query) and CGTABLE_ON / _OFF | Handled | — | [sys_max.rs:319](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L319) |
| `0x5DD` | CG mode (original cgtable::sys_query) and CGTABLE_ON / _OFF | Handled | — | [sys_max.rs:319](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L319) |
| `0x5DE` | CG mode (original cgtable::sys_query) and CGTABLE_ON / _OFF | Handled | — | [sys_max.rs:319](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L319) |
| `0x5DF` | CG mode (original cgtable::sys_query) and CGTABLE_ON / _OFF | Handled | — | [sys_max.rs:319](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L319) |
| `0x5E0` | CG mode (original cgtable::sys_query) and CGTABLE_ON / _OFF | Handled | — | [sys_max.rs:319](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L319) |
| `0x5E1` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x5E2` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x5E3` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x5E4` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x5E5` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x5E6` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x5F0` | System commands (dispatch) | Handled | — | [sys_max.rs:320](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L320) |
| `0x5F1` | System commands (dispatch) | Handled | — | [sys_max.rs:321](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L321) |
| `0x640` | CURSORAREA_*: keyboard navigation areas of the original's UI | Empty handler | Original no-op status not established | [sys_max.rs:325](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L325) |
| `0x641` | CURSORAREA_*: keyboard navigation areas of the original's UI | Empty handler | Original no-op status not established | [sys_max.rs:325](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L325) |
| `0x64A` | System commands (dispatch) | Handled | — | [sys_max.rs:326](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L326) |
| `0x654` | System commands (dispatch) | Handled | — | [sys_max.rs:331](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L331) |
| `0x6B0` | EDITBOX_*_GET: key events of text boxes | Handled | — | [sys_max.rs:333](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L333) |
| `0x6B1` | EDITBOX_*_GET: key events of text boxes | Handled | — | [sys_max.rs:333](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L333) |
| `0x6B2` | EDITBOX_*_GET: key events of text boxes | Handled | — | [sys_max.rs:333](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L333) |
| `0x6B3` | EDITBOX_*_GET: key events of text boxes | Handled | — | [sys_max.rs:333](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L333) |
| `0x708` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x709` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x70A` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x70B` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x70C` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x70D` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x70E` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x70F` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x710` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x711` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x712` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x713` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x714` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x715` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x716` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x717` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x718` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x719` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x71A` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x71B` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x71C` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x71D` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x71E` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x71F` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x720` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x721` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x722` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x723` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x724` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x725` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x726` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x727` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x728` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x729` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x72A` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x72B` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x72C` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x72D` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x72E` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x72F` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x730` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x731` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x732` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x733` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x734` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x735` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x736` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x737` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x738` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x739` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x73A` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x73B` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x73C` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x73D` | CG table | Handled | — | [sys_max.rs:323](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L323) |
| `0x80C` | SET_SYSTEM_FLAG_UNREADJUMP / _SYSBTNHIDE | Handled | — | [sys_max.rs:335](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L335) |
| `0x80D` | SET_SYSTEM_FLAG_UNREADJUMP / _SYSBTNHIDE | Handled | — | [sys_max.rs:335](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L335) |
| `0x948` | GET_SYSTEM_FLAG_WAKUMOD | Handled | — | [sys_max.rs:340](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L340) |
| `0xA3E` | GET_SYSTEM_FLAG_DEFAULT_* | Handled | — | [sys_max.rs:342](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L342) |
| `0xA3F` | GET_SYSTEM_FLAG_DEFAULT_* | Handled | — | [sys_max.rs:342](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L342) |
| `0xA40` | GET_SYSTEM_FLAG_DEFAULT_* | Handled | — | [sys_max.rs:342](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L342) |
| `0xA41` | GET_SYSTEM_FLAG_DEFAULT_* | Handled | — | [sys_max.rs:342](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L342) |
| `0xA42` | GET_SYSTEM_FLAG_DEFAULT_* | Handled | — | [sys_max.rs:342](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L342) |
| `0xA43` | GET_SYSTEM_FLAG_DEFAULT_* | Handled | — | [sys_max.rs:342](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L342) |
| `0xA44` | GET_SYSTEM_FLAG_DEFAULT_* | Handled | — | [sys_max.rs:342](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L342) |
| `0xA45` | GET_SYSTEM_FLAG_DEFAULT_* | Handled | — | [sys_max.rs:342](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L342) |
| `0xA46` | GET_SYSTEM_FLAG_DEFAULT_* | Handled | — | [sys_max.rs:342](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L342) |
| `0xA47` | GET_SYSTEM_FLAG_DEFAULT_* | Handled | — | [sys_max.rs:342](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L342) |
| `0xA48` | GET_SYSTEM_FLAG_DEFAULT_* | Handled | — | [sys_max.rs:342](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L342) |
| `0xA49` | GET_SYSTEM_FLAG_DEFAULT_* | Handled | — | [sys_max.rs:342](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L342) |
| `0xA4A` | GET_SYSTEM_FLAG_DEFAULT_* | Handled | — | [sys_max.rs:342](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L342) |
| `0xA4B` | GET_SYSTEM_FLAG_DEFAULT_* | Handled | — | [sys_max.rs:342](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L342) |
| `0xA4C` | GET_SYSTEM_FLAG_DEFAULT_* | Handled | — | [sys_max.rs:342](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L342) |
| `0xA4D` | GET_SYSTEM_FLAG_DEFAULT_* | Handled | — | [sys_max.rs:342](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L342) |
| `0xA4E` | GET_SYSTEM_FLAG_DEFAULT_* | Handled | — | [sys_max.rs:342](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L342) |
| `0xA4F` | GET_SYSTEM_FLAG_DEFAULT_* | Handled | — | [sys_max.rs:342](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L342) |
| `0xA50` | GET_SYSTEM_FLAG_DEFAULT_* | Handled | — | [sys_max.rs:342](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L342) |
| `0xA51` | GET_SYSTEM_FLAG_DEFAULT_* | Handled | — | [sys_max.rs:342](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L342) |
| `0xA52` | GET_SYSTEM_FLAG_DEFAULT_* | Handled | — | [sys_max.rs:342](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L342) |
| `0xA53` | GET_SYSTEM_FLAG_DEFAULT_* | Handled | — | [sys_max.rs:342](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L342) |
| `0xA54` | GET_SYSTEM_FLAG_DEFAULT_* | Handled | — | [sys_max.rs:342](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L342) |
| `0xA55` | GET_SYSTEM_FLAG_DEFAULT_* | Handled | — | [sys_max.rs:342](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L342) |
| `0xA56` | GET_SYSTEM_FLAG_DEFAULT_* | Handled | — | [sys_max.rs:342](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L342) |
| `0xA57` | GET_SYSTEM_FLAG_DEFAULT_* | Handled | — | [sys_max.rs:342](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L342) |
| `0xA58` | GET_SYSTEM_FLAG_DEFAULT_* | Handled | — | [sys_max.rs:342](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L342) |
| `0xA59` | GET_SYSTEM_FLAG_DEFAULT_* | Handled | — | [sys_max.rs:342](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L342) |
| `0xA5A` | GET_SYSTEM_FLAG_DEFAULT_* | Handled | — | [sys_max.rs:342](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L342) |
| `0xA5B` | GET_SYSTEM_FLAG_DEFAULT_* | Handled | — | [sys_max.rs:342](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L342) |
| `0xA5C` | GET_SYSTEM_FLAG_DEFAULT_* | Handled | — | [sys_max.rs:342](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L342) |
| `0xA5D` | GET_SYSTEM_FLAG_DEFAULT_* | Handled | — | [sys_max.rs:342](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L342) |
| `0xA5E` | GET_SYSTEM_FLAG_DEFAULT_* | Handled | — | [sys_max.rs:342](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L342) |
| `0xA5F` | GET_SYSTEM_FLAG_DEFAULT_* | Handled | — | [sys_max.rs:342](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L342) |
| `0xA60` | GET_SYSTEM_FLAG_DEFAULT_* | Handled | — | [sys_max.rs:342](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L342) |
| `0xA61` | GET_SYSTEM_FLAG_DEFAULT_* | Handled | — | [sys_max.rs:342](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L342) |
| `0xA62` | GET_SYSTEM_FLAG_DEFAULT_* | Handled | — | [sys_max.rs:342](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L342) |
| `0xA63` | GET_SYSTEM_FLAG_DEFAULT_* | Handled | — | [sys_max.rs:342](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L342) |
| `0xA64` | GET_SYSTEM_FLAG_DEFAULT_* | Handled | — | [sys_max.rs:342](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L342) |
| `0xA65` | GET_SYSTEM_FLAG_DEFAULT_* | Handled | — | [sys_max.rs:342](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L342) |
| `0xA66` | GET_SYSTEM_FLAG_DEFAULT_* | Handled | — | [sys_max.rs:342](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L342) |
| `0xA67` | GET_SYSTEM_FLAG_DEFAULT_* | Handled | — | [sys_max.rs:342](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L342) |
| `0xA68` | GET_SYSTEM_FLAG_DEFAULT_* | Handled | — | [sys_max.rs:342](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L342) |
| `0xA69` | GET_SYSTEM_FLAG_DEFAULT_* | Handled | — | [sys_max.rs:342](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L342) |
| `0xA6A` | GET_SYSTEM_FLAG_DEFAULT_* | Handled | — | [sys_max.rs:342](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L342) |
| `0xA6B` | GET_SYSTEM_FLAG_DEFAULT_* | Handled | — | [sys_max.rs:342](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L342) |
| `0xA6C` | GET_SYSTEM_FLAG_DEFAULT_* | Handled | — | [sys_max.rs:342](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L342) |
| `0xA6D` | GET_SYSTEM_FLAG_DEFAULT_* | Handled | — | [sys_max.rs:342](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L342) |
| `0xA6E` | GET_SYSTEM_FLAG_DEFAULT_* | Handled | — | [sys_max.rs:342](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L342) |
| `0xA6F` | GET_SYSTEM_FLAG_DEFAULT_* | Handled | — | [sys_max.rs:342](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L342) |
| `0xA8C` | CHECK_SYSTEM_FLAG_DISPLAYMOD(mode): every mode is possible | Handled | — | [sys_max.rs:344](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L344) |
| `0xBB9` | CHECK_JUST_AFTER_LOAD | Handled | — | [sys_max.rs:346](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L346) |
| `0xE10` | System commands (dispatch) | Handled | — | [sys_max.rs:293](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L293) |
| `0xE11` | System commands (dispatch) | Handled | — | [sys_max.rs:297](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L297) |
| `0xE1A` | System commands (dispatch) | Handled | — | [sys_max.rs:293](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L293) |
| `0xE1B` | System commands (dispatch) | Handled | — | [sys_max.rs:297](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L297) |
| `0xE24` | System commands (dispatch) | Handled | — | [sys_max.rs:293](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L293) |
| `0xE25` | System commands (dispatch) | Handled | — | [sys_max.rs:297](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L297) |

## Module commands / vm/modules/sys_max / dispatch / opcode=1015..=1017

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x3F7` | System commands (dispatch) | Handled | — | [sys_max.rs:280](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L280) |
| `0x3F8` | System commands (dispatch) | Handled | — | [sys_max.rs:281](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L281) |

## Module commands / vm/modules/sys_max / dispatch / opcode=135..=137

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x87` | System commands (dispatch) | Handled | — | [sys_max.rs:145](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L145) |
| `0x88` | System commands (dispatch) | Handled | — | [sys_max.rs:149](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L149) |

## Module commands / vm/modules/sys_max / event_loop

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x78` | SetInterrupt(scenario, entrypoint) / ClearInterrupt() | Handled | — | [sys_max.rs:641](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L641) |
| `0x79` | System commands (event_loop) | Handled | — | [sys_max.rs:646](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L646) |
| `0x7A` | INTERRUPT_CHANGE(scenario, entrypoint) | Handled | — | [sys_max.rs:648](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L648) |
| `0x7B` | INTERRUPT_GET_STATE(scenario, entrypoint, set) | Handled | — | [sys_max.rs:654](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L654) |
| `0x82` | INTERRUPTCOM_GET_PROCESS / _DISAPPEAR / _APPEAR | Handled | — | [sys_max.rs:660](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L660) |
| `0x83` | System commands (event_loop) | Handled | — | [sys_max.rs:661](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L661) |
| `0x84` | System commands (event_loop) | Handled | — | [sys_max.rs:662](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L662) |
| `0xD2` | GET_MOUSEACTIONCALL_AREA(id, x1, y1, x2, y2): `#MOUSEACTIONCALL` | Handled | — | [sys_max.rs:664](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L664) |
| `0xD3` | SET_MOUSEACTIONCALL(id) / _ON / _OFF; CCOM_SET_MOUSEACTIONCALL_* | Handled | — | [sys_max.rs:671](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L671) |
| `0xD4` | SET_MOUSEACTIONCALL(id) / _ON / _OFF; CCOM_SET_MOUSEACTIONCALL_* | Handled | — | [sys_max.rs:671](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L671) |
| `0xD5` | System commands (event_loop) | Handled | — | [sys_max.rs:679](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L679) |
| `0x12C` | rtlButton / rtlCancel / rtlSystem, LOADCALL_ / MOUSEACTIONCALL_ / EXAFTERCALL_FARRETURN: return from a routine the syst… | Handled | — | [sys_max.rs:689](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L689) |
| `0x12D` | rtlButton / rtlCancel / rtlSystem, LOADCALL_ / MOUSEACTIONCALL_ / EXAFTERCALL_FARRETURN: return from a routine the syst… | Handled | — | [sys_max.rs:689](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L689) |
| `0x12E` | rtlButton / rtlCancel / rtlSystem, LOADCALL_ / MOUSEACTIONCALL_ / EXAFTERCALL_FARRETURN: return from a routine the syst… | Handled | — | [sys_max.rs:689](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L689) |
| `0x12F` | yield: return from an interrupt handler | Handled | — | [sys_max.rs:694](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L694) |
| `0x130` | rtlButton / rtlCancel / rtlSystem, LOADCALL_ / MOUSEACTIONCALL_ / EXAFTERCALL_FARRETURN: return from a routine the syst… | Handled | — | [sys_max.rs:689](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L689) |
| `0x131` | rtlButton / rtlCancel / rtlSystem, LOADCALL_ / MOUSEACTIONCALL_ / EXAFTERCALL_FARRETURN: return from a routine the syst… | Handled | — | [sys_max.rs:689](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L689) |
| `0x132` | rtlButton / rtlCancel / rtlSystem, LOADCALL_ / MOUSEACTIONCALL_ / EXAFTERCALL_FARRETURN: return from a routine the syst… | Handled | — | [sys_max.rs:689](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L689) |
| `0x140` | GET_EXCALL_MOD / GET_LOADCALL_MOD / GET_EXAFTERCALL_MOD | Handled | — | [sys_max.rs:703](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L703) |
| `0x144` | GET_EXCALL_MOD / GET_LOADCALL_MOD / GET_EXAFTERCALL_MOD | Handled | — | [sys_max.rs:703](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L703) |
| `0x146` | GET_EXCALL_MOD / GET_LOADCALL_MOD / GET_EXAFTERCALL_MOD | Handled | — | [sys_max.rs:703](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L703) |
| `0x3F2` | CCOM_MSGBK_START_READY: the backlog opens on demand here | Empty handler | Original no-op status not established | [sys_max.rs:716](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L716) |
| `0x44C` | SetSkipMode / ClearSkipMode / SkipMode | Handled | — | [sys_max.rs:712](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L712) |
| `0x44D` | System commands (event_loop) | Handled | — | [sys_max.rs:713](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L713) |
| `0x44E` | System commands (event_loop) | Handled | — | [sys_max.rs:714](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L714) |
| `0x4B6` | CCOM_QUAKE_ON / _OFF([buf \\| first, last]) | Handled | — | [sys_max.rs:718](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L718) |
| `0x4B7` | CCOM_QUAKE_ON / _OFF([buf \\| first, last]) | Handled | — | [sys_max.rs:718](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L718) |
| `0x4B8` | CCOM_FULLQUAKE / _SYSBTN / _HINT / _SELBTN _ON / _OFF | Handled | — | [sys_max.rs:733](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L733) |
| `0x4B9` | CCOM_FULLQUAKE / _SYSBTN / _HINT / _SELBTN _ON / _OFF | Handled | — | [sys_max.rs:733](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L733) |
| `0x4BA` | CCOM_FULLQUAKE / _SYSBTN / _HINT / _SELBTN _ON / _OFF | Handled | — | [sys_max.rs:733](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L733) |
| `0x4BB` | CCOM_FULLQUAKE / _SYSBTN / _HINT / _SELBTN _ON / _OFF | Handled | — | [sys_max.rs:733](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L733) |
| `0x4BC` | CCOM_FULLQUAKE / _SYSBTN / _HINT / _SELBTN _ON / _OFF | Handled | — | [sys_max.rs:733](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L733) |
| `0x4BD` | CCOM_FULLQUAKE / _SYSBTN / _HINT / _SELBTN _ON / _OFF | Handled | — | [sys_max.rs:733](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L733) |
| `0x4BE` | CCOM_FULLQUAKE / _SYSBTN / _HINT / _SELBTN _ON / _OFF | Handled | — | [sys_max.rs:733](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L733) |
| `0x4BF` | CCOM_FULLQUAKE / _SYSBTN / _HINT / _SELBTN _ON / _OFF | Handled | — | [sys_max.rs:733](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L733) |
| `0x7D0` | CCOM_LOCAL_FLAG_EXCOPY(var) / _SAVEPOINT: the value at the last savepoint (what a load would restore); _NOW keeps the c… | Handled | — | [sys_max.rs:742](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L742) |
| `0x7D1` | CCOM_LOCAL_NAME_EXCOPY(index): the local name as it was at the savepoint; _NOW keeps the current one (a no-op here) | Handled | — | [sys_max.rs:754](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L754) |
| `0x7D2` | System commands (event_loop) | Empty handler | Original no-op status not established | [sys_max.rs:763](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L763) |
| `0x7D3` | System commands (event_loop) | Empty handler | Original no-op status not established | [sys_max.rs:763](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L763) |
| `0x7D4` | CCOM_LOCAL_FLAG_EXCOPY(var) / _SAVEPOINT: the value at the last savepoint (what a load would restore); _NOW keeps the c… | Handled | — | [sys_max.rs:742](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L742) |
| `0x7D5` | CCOM_LOCAL_NAME_EXCOPY(index): the local name as it was at the savepoint; _NOW keeps the current one (a no-op here) | Handled | — | [sys_max.rs:754](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L754) |
| `0x7E4` | CCOM_SET_WINDOW_EXBTN_MOD (_NOW, _SAVEPOINT)([button,] mod) | Handled | — | [sys_max.rs:765](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L765) |
| `0x7E5` | CCOM_SET_WINDOW_EXBTN_MOD (_NOW, _SAVEPOINT)([button,] mod) | Handled | — | [sys_max.rs:765](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L765) |
| `0x7E6` | CCOM_SET_WINDOW_EXBTN_MOD (_NOW, _SAVEPOINT)([button,] mod) | Handled | — | [sys_max.rs:765](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L765) |
| `0x7EE` | SET_MOUSEACTIONCALL(id) / _ON / _OFF; CCOM_SET_MOUSEACTIONCALL_* | Handled | — | [sys_max.rs:671](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L671) |
| `0x7EF` | System commands (event_loop) | Handled | — | [sys_max.rs:679](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L679) |
| `0x7F0` | SET_MOUSEACTIONCALL(id) / _ON / _OFF; CCOM_SET_MOUSEACTIONCALL_* | Handled | — | [sys_max.rs:671](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L671) |
| `0x7F1` | System commands (event_loop) | Handled | — | [sys_max.rs:679](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L679) |
| `0x7F2` | SET_MOUSEACTIONCALL(id) / _ON / _OFF; CCOM_SET_MOUSEACTIONCALL_* | Handled | — | [sys_max.rs:671](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L671) |
| `0x7F3` | System commands (event_loop) | Handled | — | [sys_max.rs:679](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L679) |
| `0xBB8` | LOADEVENT_G_FLAG_SET(use[, flag, id]) | Handled | — | [sys_max.rs:780](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L780) |
| `0xBB9` | CHECK_JUST_AFTER_LOAD | Handled | — | [sys_max.rs:787](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L787) |

## Module commands / vm/modules/sys_max / event_loop / opcode=320 \| 324 \| 326

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x140` | System commands (event_loop) | Handled | — | [sys_max.rs:705](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L705) |
| `0x144` | System commands (event_loop) | Handled | — | [sys_max.rs:706](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L706) |

## Module commands / vm/modules/sys_max / system_default

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0xA3E` | System commands (system_default) | Handled | — | [sys_max.rs:359](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L359) |
| `0xA3F` | System commands (system_default) | Handled | — | [sys_max.rs:360](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L360) |
| `0xA40` | System commands (system_default) | Handled | — | [sys_max.rs:361](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L361) |
| `0xA41` | System commands (system_default) | Handled | — | [sys_max.rs:362](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L362) |
| `0xA42` | System commands (system_default) | Handled | — | [sys_max.rs:363](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L363) |
| `0xA43` | System commands (system_default) | Handled | — | [sys_max.rs:364](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L364) |
| `0xA44` | character voices on, full volume, normal speed, the rest off | Handled | — | [sys_max.rs:375](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L375) |
| `0xA45` | System commands (system_default) | Handled | — | [sys_max.rs:365](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L365) |
| `0xA46` | System commands (system_default) | Handled | — | [sys_max.rs:366](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L366) |
| `0xA47` | System commands (system_default) | Handled | — | [sys_max.rs:366](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L366) |
| `0xA48` | System commands (system_default) | Handled | — | [sys_max.rs:366](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L366) |
| `0xA49` | System commands (system_default) | Handled | — | [sys_max.rs:366](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L366) |
| `0xA4A` | System commands (system_default) | Handled | — | [sys_max.rs:367](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L367) |
| `0xA4B` | System commands (system_default) | Handled | — | [sys_max.rs:367](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L367) |
| `0xA4C` | System commands (system_default) | Handled | — | [sys_max.rs:367](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L367) |
| `0xA4D` | System commands (system_default) | Handled | — | [sys_max.rs:367](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L367) |
| `0xA4F` | System commands (system_default) | Handled | — | [sys_max.rs:376](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L376) |
| `0xA53` | System commands (system_default) | Handled | — | [sys_max.rs:377](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L377) |
| `0xA5A` | System commands (system_default) | Handled | — | [sys_max.rs:368](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L368) |
| `0xA5B` | System commands (system_default) | Handled | — | [sys_max.rs:369](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L369) |
| `0xA5C` | System commands (system_default) | Handled | — | [sys_max.rs:370](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L370) |
| `0xA64` | character voices on, full volume, normal speed, the rest off | Handled | — | [sys_max.rs:375](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L375) |
| `0xA65` | System commands (system_default) | Handled | — | [sys_max.rs:371](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L371) |
| `0xA67` | System commands (system_default) | Handled | — | [sys_max.rs:372](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L372) |
| `0xA68` | character voices on, full volume, normal speed, the rest off | Handled | — | [sys_max.rs:375](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L375) |
| `0xA69` | character voices on, full volume, normal speed, the rest off | Handled | — | [sys_max.rs:375](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L375) |
| `0xA6A` | System commands (system_default) | Handled | — | [sys_max.rs:373](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L373) |

## Module routing / adapter/commands

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `(0, 0x03)` | Message | Handled | — | [commands.rs:70](../../crates/crysirror-reallive/src/adapter/commands.rs#L70) |
| `(0, 0x04)` | Event loop extended | Handled | — | [commands.rs:38](../../crates/crysirror-reallive/src/adapter/commands.rs#L38) |
| `(1 \| 2, 0x47)` | Object creation | Handled | — | [commands.rs:16](../../crates/crysirror-reallive/src/adapter/commands.rs#L16) |
| `(1 \| 2, 0x48)` | Object creation | Handled | — | [commands.rs:16](../../crates/crysirror-reallive/src/adapter/commands.rs#L16) |
| `(1, 0x04)` | System extended | Handled | — | [commands.rs:17](../../crates/crysirror-reallive/src/adapter/commands.rs#L17) |
| `(1, 0x05)` | Update optical disc checks, store | Partial | DUMMYCHECK_DISC is unsupported under strict optical-disc policy; Guard: c.op.opcode == 114 && c.op.overload == 0 | [commands.rs:23](../../crates/crysirror-reallive/src/adapter/commands.rs#L23) |
| `(1, 0x0A)` | Text | Handled | Guard: c.op.opcode == 100 | [commands.rs:71](../../crates/crysirror-reallive/src/adapter/commands.rs#L71) |
| `(1, 0x0C)` | Shake | Handled | — | [commands.rs:40](../../crates/crysirror-reallive/src/adapter/commands.rs#L40) |
| `(1, 0x0D)` | Shake | Handled | — | [commands.rs:40](../../crates/crysirror-reallive/src/adapter/commands.rs#L40) |
| `(1, 0x0E)` | Execute command branch | Handled | — | [commands.rs:41](../../crates/crysirror-reallive/src/adapter/commands.rs#L41) |
| `(1, 0x14)` | Sound | Handled | — | [commands.rs:75](../../crates/crysirror-reallive/src/adapter/commands.rs#L75) |
| `(1, 0x15)` | Sound | Handled | — | [commands.rs:75](../../crates/crysirror-reallive/src/adapter/commands.rs#L75) |
| `(1, 0x16)` | Sound | Handled | — | [commands.rs:75](../../crates/crysirror-reallive/src/adapter/commands.rs#L75) |
| `(1, 0x17)` | Sound | Handled | — | [commands.rs:75](../../crates/crysirror-reallive/src/adapter/commands.rs#L75) |
| `(1, 0x1A)` | Movie | Handled | — | [commands.rs:76](../../crates/crysirror-reallive/src/adapter/commands.rs#L76) |
| `(1, 0x1E)` | Query | Partial | unsupported screen operation | [commands.rs:44](../../crates/crysirror-reallive/src/adapter/commands.rs#L44) |
| `(1, 0x1F)` | Refresh manual | Handled | — | [commands.rs:63](../../crates/crysirror-reallive/src/adapter/commands.rs#L63) |
| `(1, 0x21)` | Graphics | Handled | — | [commands.rs:69](../../crates/crysirror-reallive/src/adapter/commands.rs#L69) |
| `(1, 0x28)` | Haikei | Handled | — | [commands.rs:78](../../crates/crysirror-reallive/src/adapter/commands.rs#L78) |
| `(1, 0x29)` | Flash | Handled | — | [commands.rs:39](../../crates/crysirror-reallive/src/adapter/commands.rs#L39) |
| `(1, 0xFF)` | Debug | Handled | — | [commands.rs:42](../../crates/crysirror-reallive/src/adapter/commands.rs#L42) |
| `(2, 0x01)` | Dll | Handled | — | [commands.rs:77](../../crates/crysirror-reallive/src/adapter/commands.rs#L77) |

## Module routing / vm/modules/mod

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `(0, 0x01)` | Execute dispatch branch | Handled | — | [mod.rs:16](../../crates/crysirror-reallive/src/vm/modules/mod.rs#L16) |
| `(0, 0x02)` | Execute dispatch branch | Handled | Guard: !matches!(command.kind, super::bytecode::CommandKind::Select(_)) | [mod.rs:17](../../crates/crysirror-reallive/src/vm/modules/mod.rs#L17) |
| `(0, 0x04)` | Execute dispatch branch | Handled | — | [mod.rs:33](../../crates/crysirror-reallive/src/vm/modules/mod.rs#L33) |
| `(0, 0x05)` | Execute dispatch branch | Handled | — | [mod.rs:16](../../crates/crysirror-reallive/src/vm/modules/mod.rs#L16) |
| `(0, 0x06)` | Execute dispatch branch | Handled | — | [mod.rs:16](../../crates/crysirror-reallive/src/vm/modules/mod.rs#L16) |
| `(1 \| 2, 0x3C)` | Execute dispatch branch | Handled | — | [mod.rs:20](../../crates/crysirror-reallive/src/vm/modules/mod.rs#L20) |
| `(1 \| 2, 0x3D)` | Execute dispatch branch | Handled | — | [mod.rs:20](../../crates/crysirror-reallive/src/vm/modules/mod.rs#L20) |
| `(1 \| 2, 0x3E)` | Execute dispatch branch | Handled | — | [mod.rs:20](../../crates/crysirror-reallive/src/vm/modules/mod.rs#L20) |
| `(1 \| 2, 0x47)` | Execute dispatch branch | Handled | Guard: matches!(command.op.opcode,1000..=1004\\|1300\\|1400) && machine.sys.gfx.loader.as_ref().is_some_and(\\|l\\|l.owner_image_preparation()) | [mod.rs:21](../../crates/crysirror-reallive/src/vm/modules/mod.rs#L21) |
| `(1 \| 2, 0x48)` | Execute dispatch branch | Handled | Guard: matches!(command.op.opcode,1000..=1004\\|1300\\|1400) && machine.sys.gfx.loader.as_ref().is_some_and(\\|l\\|l.owner_image_preparation()) | [mod.rs:21](../../crates/crysirror-reallive/src/vm/modules/mod.rs#L21) |
| `(1 \| 2, 0x49)` | Execute dispatch branch | Handled | — | [mod.rs:23](../../crates/crysirror-reallive/src/vm/modules/mod.rs#L23) |
| `(1 \| 2, 0x4A)` | Execute dispatch branch | Handled | — | [mod.rs:23](../../crates/crysirror-reallive/src/vm/modules/mod.rs#L23) |
| `(1 \| 2, 0x51)` | Execute dispatch branch | Handled | — | [mod.rs:24](../../crates/crysirror-reallive/src/vm/modules/mod.rs#L24) |
| `(1 \| 2, 0x52)` | Execute dispatch branch | Handled | — | [mod.rs:24](../../crates/crysirror-reallive/src/vm/modules/mod.rs#L24) |
| `(1 \| 2, 0x54)` | Execute dispatch branch | Handled | — | [mod.rs:25](../../crates/crysirror-reallive/src/vm/modules/mod.rs#L25) |
| `(1 \| 2, 0x55)` | Execute dispatch branch | Handled | — | [mod.rs:25](../../crates/crysirror-reallive/src/vm/modules/mod.rs#L25) |
| `(1 \| 2, 0x57)` | Execute dispatch branch | Handled | — | [mod.rs:24](../../crates/crysirror-reallive/src/vm/modules/mod.rs#L24) |
| `(1 \| 2, 0x58)` | Execute dispatch branch | Handled | — | [mod.rs:24](../../crates/crysirror-reallive/src/vm/modules/mod.rs#L24) |
| `(1 \| 2, 0x5A)` | Execute dispatch branch | Handled | — | [mod.rs:24](../../crates/crysirror-reallive/src/vm/modules/mod.rs#L24) |
| `(1 \| 2, 0x5B)` | Execute dispatch branch | Handled | — | [mod.rs:24](../../crates/crysirror-reallive/src/vm/modules/mod.rs#L24) |
| `(1 \| 2, 0x5D)` | Execute dispatch branch | Handled | — | [mod.rs:24](../../crates/crysirror-reallive/src/vm/modules/mod.rs#L24) |
| `(1 \| 2, 0x5E)` | Execute dispatch branch | Handled | — | [mod.rs:24](../../crates/crysirror-reallive/src/vm/modules/mod.rs#L24) |
| `(1, 0x04)` | Execute dispatch branch | Handled | — | [mod.rs:26](../../crates/crysirror-reallive/src/vm/modules/mod.rs#L26) |
| `(1, 0x0A)` | Execute dispatch branch | Handled | Guard: command.op.opcode != 100 | [mod.rs:18](../../crates/crysirror-reallive/src/vm/modules/mod.rs#L18) |
| `(1, 0x0B)` | Execute dispatch branch | Handled | Guard: !(300..=307).contains(&command.op.opcode) | [mod.rs:19](../../crates/crysirror-reallive/src/vm/modules/mod.rs#L19) |

## Subselectors and conditional dispatch

Patterns retain their source namespace and guards. `_` is the fallback of that table. Boolean/optional results follow the function's return contract. Call-site names identify the implementation to inspect, including indirect service calls.

### adapter.rs / RealLiveVm::boot

```rust
fn boot(&mut self, h: &mut dyn HostServices) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `mode` | `1` | Local state/value operations | Handled | [310](../../crates/crysirror-reallive/src/adapter.rs#L310) |
| `mode` | `2` | Local state/value operations | Handled | [311](../../crates/crysirror-reallive/src/adapter.rs#L311) |
| `mode` | `3` | Local state/value operations | Handled | [312](../../crates/crysirror-reallive/src/adapter.rs#L312) |
| `mode` | `4` | Local state/value operations | Handled | [313](../../crates/crysirror-reallive/src/adapter.rs#L313) |
| `mode` | `_` | Local state/value operations | Handled | [314](../../crates/crysirror-reallive/src/adapter.rs#L314) |
| `condition` | `let Some(name) = exe.str("CGTABLE_FILENAME").map(str::to_owned).filter(\|n\| !n.is_empty())` | self.load_cg_table, Rc::new | Handled | [330](../../crates/crysirror-reallive/src/adapter.rs#L330) |
| `self.load_cg_table(&name)` | `Ok(table)` | Rc::new | Handled | [333](../../crates/crysirror-reallive/src/adapter.rs#L333) |
| `self.load_cg_table(&name)` | `Err(e) if format!("{e:#}").contains("not found")` | Local state/value operations | Empty handler: Original no-op status not established | [334](../../crates/crysirror-reallive/src/adapter.rs#L334) |
| `self.load_cg_table(&name)` | `Err(e)` | Local state/value operations | Unsupported: Unconditional rejection | [335](../../crates/crysirror-reallive/src/adapter.rs#L335) |

### adapter/commands.rs / RealLiveVm::command

```rust
fn command(
        &mut self,
        m: &mut Machine,
        h: &mut dyn HostServices,
        input: InputFrame,
        c: &Command,
    ) -> Result<Next>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `c.op.opcode; under (c.op.modtype, c.op.module)=(1, 30)` | `_` | Local state/value operations | Unsupported: unsupported screen operation | [61](../../crates/crysirror-reallive/src/adapter/commands.rs#L61) |
| `(c.op.modtype, c.op.module)` | `(1, 5) if c.op.opcode == 114 && c.op.overload == 0` | Local state/value operations | Partial: DUMMYCHECK_DISC is unsupported under strict optical-disc policy | [23](../../crates/crysirror-reallive/src/adapter/commands.rs#L23) |
| `(c.op.modtype, c.op.module)` | `(1, 5) if c.op.opcode != 114` | self.os | Handled | [43](../../crates/crysirror-reallive/src/adapter/commands.rs#L43) |
| `(c.op.modtype, c.op.module)` | `(1, 10) if c.op.opcode == 100` | self.text | Handled | [71](../../crates/crysirror-reallive/src/adapter/commands.rs#L71) |
| `(c.op.modtype, c.op.module)` | `_` | Local state/value operations | Unsupported: unsupported native syscall {} | [79](../../crates/crysirror-reallive/src/adapter/commands.rs#L79) |

### adapter/commands.rs / RealLiveVm::dll

```rust
fn dll(&mut self, m: &mut Machine, c: &Command) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `c.op.opcode` | `_` | Local state/value operations | Unsupported: unsupported native DLL operation {} | [1837](../../crates/crysirror-reallive/src/adapter/commands.rs#L1837) |

### adapter/commands.rs / RealLiveVm::finish_pause

```rust
fn finish_pause(
        &mut self,
        m: &mut Machine,
        h: &mut dyn HostServices,
        kind: u16,
        window: u32,
    ) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(kind, 15 \| 205 \| 206)` | Local state/value operations | Handled | [1073](../../crates/crysirror-reallive/src/adapter/commands.rs#L1073) |
| `condition` | `matches!(kind, 15 \| 205 \| 206)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [1073](../../crates/crysirror-reallive/src/adapter/commands.rs#L1073) |
| `condition` | `matches!(kind, 17 \| 207) && r` | self.font_size, self.pen.entry | Handled | [1076](../../crates/crysirror-reallive/src/adapter/commands.rs#L1076) |
| `condition` | `matches!(kind, 17 \| 207)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [1076](../../crates/crysirror-reallive/src/adapter/commands.rs#L1076) |
| `condition` | `kind != 207` | self.pen.keys, self.clear_window | Handled | [1085](../../crates/crysirror-reallive/src/adapter/commands.rs#L1085) |

### adapter/commands.rs / RealLiveVm::graphics

```rust
fn graphics(&mut self, m: &mut Machine, h: &mut dyn HostServices, c: &Command) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under op=50 \| 51 \| 70 \| 71` | `matches!(op, 51 \| 71)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [236](../../crates/crysirror-reallive/src/adapter/commands.rs#L236) |
| `condition; under op=72 \| 73 \| 74 \| 76` | `name == "?"` | self.extent | Handled | [308](../../crates/crysirror-reallive/src/adapter/commands.rs#L308) |
| `condition; under op=72 \| 73 \| 74 \| 76` | `name == "???"` | Local state/value operations | Handled | [312](../../crates/crysirror-reallive/src/adapter/commands.rs#L312) |
| `condition; under op=72 \| 73 \| 74 \| 76` | `op == 73 && name != "???"` | Local state/value operations | Handled | [342](../../crates/crysirror-reallive/src/adapter/commands.rs#L342) |
| `condition; under op=72 \| 73 \| 74 \| 76` | `matches!(op, 72 \| 73)` | Local state/value operations | Handled | [349](../../crates/crysirror-reallive/src/adapter/commands.rs#L349) |
| `condition; under op=72 \| 73 \| 74 \| 76` | `matches!(op, 72 \| 73)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [349](../../crates/crysirror-reallive/src/adapter/commands.rs#L349) |
| `op; under op=300..=303` | `_` | Local state/value operations | Handled | [429](../../crates/crysirror-reallive/src/adapter/commands.rs#L429) |
| `condition; under op=300..=303 / op=300 \| 301` | `op == 300` | Local state/value operations | Handled | [436](../../crates/crysirror-reallive/src/adapter/commands.rs#L436) |
| `condition; under op=300..=303 / op=_` | `op == 302` | Local state/value operations | Handled | [439](../../crates/crysirror-reallive/src/adapter/commands.rs#L439) |
| `op; under op=300..=303` | `_` | Local state/value operations | Handled | [438](../../crates/crysirror-reallive/src/adapter/commands.rs#L438) |
| `condition; under op=75 \| 77` | `op == 75` | self.resource, h.image_size | Handled | [484](../../crates/crysirror-reallive/src/adapter/commands.rs#L484) |
| `condition; under op=75 \| 77` | `name == "???"` | Local state/value operations | Handled | [486](../../crates/crysirror-reallive/src/adapter/commands.rs#L486) |
| `condition; under op=200 \| 201` | `op == 201` | Local state/value operations | Handled | [565](../../crates/crysirror-reallive/src/adapter/commands.rs#L565) |
| `op` | `_` | self.graphics_extended | Partial: unsupported native graphics operation {} | [601](../../crates/crysirror-reallive/src/adapter/commands.rs#L601) |

### adapter/commands.rs / RealLiveVm::haikei

```rust
fn haikei(&mut self, m: &mut Machine, h: &mut dyn HostServices, c: &Command) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under op=10 \| 11 \| 20 \| 21 \| 101` | `matches!(op, 20 \| 21 \| 101) && count >= 2` | Local state/value operations | Handled | [1518](../../crates/crysirror-reallive/src/adapter/commands.rs#L1518) |
| `condition; under op=10 \| 11 \| 20 \| 21 \| 101` | `matches!(op, 20 \| 21 \| 101)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [1518](../../crates/crysirror-reallive/src/adapter/commands.rs#L1518) |
| `condition; under op=10 \| 11 \| 20 \| 21 \| 101` | `matches!(op, 11 \| 21)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [1529](../../crates/crysirror-reallive/src/adapter/commands.rs#L1529) |
| `op` | `_` | Local state/value operations | Unsupported: unsupported native background operation {} | [1538](../../crates/crysirror-reallive/src/adapter/commands.rs#L1538) |

### adapter/commands.rs / RealLiveVm::message

```rust
fn message(&mut self, m: &mut Machine, h: &mut dyn HostServices, c: &Command) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under c.op.opcode=201 \| 3 \| 202` | `c.op.opcode == 201 \|\| pen[0] > 0` | Local state/value operations | Handled | [956](../../crates/crysirror-reallive/src/adapter/commands.rs#L956) |
| `c.op.opcode` | `_` | self.message_extended | Partial: unsupported native message operation {} | [961](../../crates/crysirror-reallive/src/adapter/commands.rs#L961) |

### adapter/commands.rs / RealLiveVm::movie

```rust
fn movie(&mut self, m: &mut Machine, h: &mut dyn HostServices, c: &Command) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under c.op.opcode=0 \| 1 \| 2 \| 20` | `matches!(c.op.opcode, 1 \| 20)` | Local state/value operations | Handled | [640](../../crates/crysirror-reallive/src/adapter/commands.rs#L640) |
| `c.op.opcode` | `_` | Local state/value operations | Unsupported: unsupported native movie operation | [658](../../crates/crysirror-reallive/src/adapter/commands.rs#L658) |

### adapter/commands.rs / RealLiveVm::name_entry

```rust
fn name_entry(&mut self, m: &mut Machine, h: &mut dyn HostServices, c: &Command) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(c.op.opcode, 1302 \| 1312)` | Local state/value operations | Handled | [1098](../../crates/crysirror-reallive/src/adapter/commands.rs#L1098) |

### adapter/commands.rs / RealLiveVm::sound

```rust
fn sound(&mut self, m: &mut Machine, h: &mut dyn HostServices, c: &Command) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under module=21` | `op <= 2` | Local state/value operations | Handled | [667](../../crates/crysirror-reallive/src/adapter/commands.rs#L667) |
| `condition; under (module, op)=(20, 0..=2) \| (21, 0..=2)` | `strings .get(1) .or(strings.first()) .is_some_and(\|s\| s.eq_ignore_ascii_case(&name))` | Local state/value operations | Handled | [685](../../crates/crysirror-reallive/src/adapter/commands.rs#L685) |
| `condition; under (module, op)=(20, 105 \| 106)` | `op == 106` | Local state/value operations | Handled | [793](../../crates/crysirror-reallive/src/adapter/commands.rs#L793) |
| `condition; under (module, op)=(23, 0 \| 1 \| 7..=10)` | `matches!(op, 0 \| 1 \| 7)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [802](../../crates/crysirror-reallive/src/adapter/commands.rs#L802) |
| `condition; under (module, op)=(23, 0 \| 1 \| 7..=10)` | `matches!(op, 1 \| 7 \| 9 \| 10)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [813](../../crates/crysirror-reallive/src/adapter/commands.rs#L813) |
| `condition; under (module, op)=(20, 13 \| 14) \| (21, 13 \| 14)` | `op == 13` | Local state/value operations | Handled | [843](../../crates/crysirror-reallive/src/adapter/commands.rs#L843) |
| `(module, op)` | `(20, 0..=2) \| (21, 0..=2)` | self.resource, VfsPath::new, Default::default, self.poll_audio | Handled | [677](../../crates/crysirror-reallive/src/adapter/commands.rs#L677) |
| `(module, op)` | `(20 \| 21, 3)` | Local state/value operations | Handled | [745](../../crates/crysirror-reallive/src/adapter/commands.rs#L745) |
| `(module, op)` | `(20 \| 21, 4 \| 7)` | i32::from, h.audio_playing | Handled | [746](../../crates/crysirror-reallive/src/adapter/commands.rs#L746) |
| `(module, op)` | `(20 \| 21, 5) \| (23, 5)` | h.submit_audio | Handled | [749](../../crates/crysirror-reallive/src/adapter/commands.rs#L749) |
| `(module, op)` | `(21, 20)` | h.submit_audio | Handled | [772](../../crates/crysirror-reallive/src/adapter/commands.rs#L772) |
| `(module, op)` | `(20, 105 \| 106)` | h.submit_audio | Handled | [784](../../crates/crysirror-reallive/src/adapter/commands.rs#L784) |
| `(module, op)` | `(23, 0 \| 1 \| 7..=10)` | Machine::param_count, h.submit_audio, self.play_koe | Handled | [799](../../crates/crysirror-reallive/src/adapter/commands.rs#L799) |
| `(module, op)` | `(22, 0)` | self.play_numbered_se | Handled | [818](../../crates/crysirror-reallive/src/adapter/commands.rs#L818) |
| `(module, op)` | `(23, 3 \| 6)` | Local state/value operations | Handled | [823](../../crates/crysirror-reallive/src/adapter/commands.rs#L823) |
| `(module, op)` | `(23, 4)` | i32::from, h.audio_playing | Handled | [824](../../crates/crysirror-reallive/src/adapter/commands.rs#L824) |
| `(module, op)` | `(21, 9 \| 10)` | h.submit_audio | Handled | [826](../../crates/crysirror-reallive/src/adapter/commands.rs#L826) |
| `(module, op)` | `(21, 105 \| 106)` | h.submit_audio | Handled | [829](../../crates/crysirror-reallive/src/adapter/commands.rs#L829) |
| `(module, op)` | `(20, 12) \| (21, 12)` | usize::from, h.submit_audio | Handled | [834](../../crates/crysirror-reallive/src/adapter/commands.rs#L834) |
| `(module, op)` | `(20, 13 \| 14) \| (21, 13 \| 14)` | usize::from, h.submit_audio | Handled | [840](../../crates/crysirror-reallive/src/adapter/commands.rs#L840) |
| `(module, op)` | `(20, 9 \| 10)` | h.submit_audio | Handled | [847](../../crates/crysirror-reallive/src/adapter/commands.rs#L847) |
| `(module, op)` | `_` | Local state/value operations | Unsupported: unsupported native audio operation {} | [850](../../crates/crysirror-reallive/src/adapter/commands.rs#L850) |

### adapter/commands.rs / RealLiveVm::system

```rust
fn system(
        &mut self,
        m: &mut Machine,
        h: &mut dyn HostServices,
        input: InputFrame,
        c: &Command,
    ) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under c.op.opcode=133 \| 202 \| 138` | `c.op.opcode == 133` | Local state/value operations | Handled | [152](../../crates/crysirror-reallive/src/adapter/commands.rs#L152) |
| `condition; under c.op.opcode=1211 \| 1212 \| 1213` | `c.op.opcode == 1211` | Local state/value operations | Handled | [192](../../crates/crysirror-reallive/src/adapter/commands.rs#L192) |
| `condition; under c.op.opcode=1211 \| 1212 \| 1213` | `c.op.opcode == 1212` | Local state/value operations | Handled | [194](../../crates/crysirror-reallive/src/adapter/commands.rs#L194) |
| `c.op.opcode` | `_` | Local state/value operations | Unsupported: unsupported native system operation {} | [202](../../crates/crysirror-reallive/src/adapter/commands.rs#L202) |

### adapter/extended.rs / RealLiveVm::begin_bgm

```rust
fn begin_bgm(
        &mut self,
        m: &mut Machine,
        h: &mut dyn HostServices,
        name: &str,
        looped: bool,
        from: Option<(u64, Option<u64>)>,
        wait: bool,
    ) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `strings.get(1).or(strings.first()).is_some_and(\|s\| s.eq_ignore_ascii_case(&name))` | Local state/value operations | Handled | [220](../../crates/crysirror-reallive/src/adapter/extended.rs#L220) |

### adapter/extended.rs / RealLiveVm::event_loop_extended

```rust
fn event_loop_extended(&mut self, m: &mut Machine, h: &mut dyn HostServices, c: &Command) -> Result<Next>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `opcode; under opcode=400..=403` | `401 if c.op.overload == 1` | Local state/value operations | Handled | [1394](../../crates/crysirror-reallive/src/adapter/extended.rs#L1394) |
| `opcode; under opcode=400..=403` | `_` | Local state/value operations | Handled | [1397](../../crates/crysirror-reallive/src/adapter/extended.rs#L1397) |
| `condition; under opcode=1200 \| 1201` | `opcode == 1201` | self.ext.windows_off.insert | Handled | [1409](../../crates/crysirror-reallive/src/adapter/extended.rs#L1409) |
| `condition; under opcode=1204 \| 1205` | `opcode == 1205` | self.ext.ccom_hidden_objects.insert | Handled | [1425](../../crates/crysirror-reallive/src/adapter/extended.rs#L1425) |
| `opcode` | `_` | Local state/value operations | Unsupported: unsupported native event-loop operation {} | [1440](../../crates/crysirror-reallive/src/adapter/extended.rs#L1440) |

### adapter/extended.rs / RealLiveVm::finish_pan

```rust
fn finish_pan(&mut self, h: &mut dyn HostServices, pan: &Pan) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `pan.kind != 406` | self.canvas | Handled | [1712](../../crates/crysirror-reallive/src/adapter/extended.rs#L1712) |

### adapter/extended.rs / RealLiveVm::flash

```rust
fn flash(&mut self, m: &mut Machine, h: &mut dyn HostServices, c: &Command) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(opcode, 100..=102 \| 110..=112)` | self.ext.flash.as_ref, self.update_flash, i32::from | Handled | [725](../../crates/crysirror-reallive/src/adapter/extended.rs#L725) |
| `condition` | `matches!(opcode, 100..=102 \| 110..=112)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [725](../../crates/crysirror-reallive/src/adapter/extended.rs#L725) |
| `opcode % 10` | `0` | self.update_flash | Handled | [729](../../crates/crysirror-reallive/src/adapter/extended.rs#L729) |
| `opcode % 10` | `1` | Local state/value operations | Handled | [735](../../crates/crysirror-reallive/src/adapter/extended.rs#L735) |
| `opcode % 10` | `_` | i32::from | Handled | [736](../../crates/crysirror-reallive/src/adapter/extended.rs#L736) |

### adapter/extended.rs / RealLiveVm::g00_buffers

```rust
fn g00_buffers(&mut self, m: &mut Machine, c: &Command) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `c.op.opcode` | `_` | Local state/value operations | Unsupported: unsupported native g00 buffer operation {} | [784](../../crates/crysirror-reallive/src/adapter/extended.rs#L784) |

### adapter/extended.rs / RealLiveVm::graphics_extended

```rust
fn graphics_extended(&mut self, m: &mut Machine, h: &mut dyn HostServices, c: &Command, op: u16, rec: bool) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under op=406..=408` | `op == 406` | Local state/value operations | Handled | [1634](../../crates/crysirror-reallive/src/adapter/extended.rs#L1634) |
| `condition; under op=406..=408` | `op != 406 && !rec` | Local state/value operations | Handled | [1640](../../crates/crysirror-reallive/src/adapter/extended.rs#L1640) |
| `condition; under op=406..=408` | `op != 406` | self.canvas | Handled | [1641](../../crates/crysirror-reallive/src/adapter/extended.rs#L1641) |
| `op` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [1655](../../crates/crysirror-reallive/src/adapter/extended.rs#L1655) |

### adapter/extended.rs / RealLiveVm::message_extended

```rust
fn message_extended(&mut self, m: &mut Machine, h: &mut dyn HostServices, c: &Command) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under c.op.opcode=310 \| 311 \| 312 \| 320 \| 321 \| 322` | `matches!(c.op.opcode, 310 \| 320)` | Local state/value operations | Handled | [977](../../crates/crysirror-reallive/src/adapter/extended.rs#L977) |
| `c.op.opcode; under c.op.opcode=310 \| 311 \| 312 \| 320 \| 321 \| 322` | `_` | Local state/value operations | Handled | [985](../../crates/crysirror-reallive/src/adapter/extended.rs#L985) |
| `condition; under c.op.opcode=402..=409 \| 502..=509` | `opcode >= 500` | Local state/value operations | Handled | [1021](../../crates/crysirror-reallive/src/adapter/extended.rs#L1021) |
| `opcode % 100; under c.op.opcode=402..=409 \| 502..=509` | `2 \| 4 \| 6` | Local state/value operations | Handled | [1023](../../crates/crysirror-reallive/src/adapter/extended.rs#L1023) |
| `opcode % 100; under c.op.opcode=402..=409 \| 502..=509` | `3 \| 5` | Local state/value operations | Handled | [1027](../../crates/crysirror-reallive/src/adapter/extended.rs#L1027) |
| `opcode % 100; under c.op.opcode=402..=409 \| 502..=509` | `7` | Local state/value operations | Handled | [1031](../../crates/crysirror-reallive/src/adapter/extended.rs#L1031) |
| `opcode % 100; under c.op.opcode=402..=409 \| 502..=509` | `8` | Local state/value operations | Handled | [1034](../../crates/crysirror-reallive/src/adapter/extended.rs#L1034) |
| `opcode % 100; under c.op.opcode=402..=409 \| 502..=509` | `_` | Local state/value operations | Handled | [1037](../../crates/crysirror-reallive/src/adapter/extended.rs#L1037) |
| `c.op.opcode` | `120 if Machine::param_count(c) == 0` | self.pen.get, self.ext.ruby_start.insert | Handled | [950](../../crates/crysirror-reallive/src/adapter/extended.rs#L950) |
| `c.op.opcode` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [1054](../../crates/crysirror-reallive/src/adapter/extended.rs#L1054) |

### adapter/extended.rs / RealLiveVm::os

```rust
fn os(&mut self, m: &mut Machine, h: &mut dyn HostServices, c: &Command) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `c.op.opcode` | `_` | Local state/value operations | Unsupported: unsupported native os operation {} | [827](../../crates/crysirror-reallive/src/adapter/extended.rs#L827) |

### adapter/extended.rs / RealLiveVm::save_query

```rust
fn save_query(&mut self, m: &mut Machine, h: &mut dyn HostServices, c: &Command) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `(crate::vm::save::any_slot_header(m, &mut store, slot), opcode); under opcode=1400..=1408` | `(Some(header), 1408)` | crate::vm::modules::sys_max::write_str | Handled | [1822](../../crates/crysirror-reallive/src/adapter/extended.rs#L1822) |
| `(crate::vm::save::any_slot_header(m, &mut store, slot), opcode); under opcode=1400..=1408` | `(Some(header), _)` | usize::from | Handled | [1826](../../crates/crysirror-reallive/src/adapter/extended.rs#L1826) |
| `(crate::vm::save::any_slot_header(m, &mut store, slot), opcode); under opcode=1400..=1408` | `(None, _)` | Local state/value operations | Handled | [1827](../../crates/crysirror-reallive/src/adapter/extended.rs#L1827) |
| `opcode` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [1844](../../crates/crysirror-reallive/src/adapter/extended.rs#L1844) |

### adapter/extended.rs / RealLiveVm::shake

```rust
fn shake(&mut self, m: &mut Machine, h: &mut dyn HostServices, c: &Command) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `opcode == 0` | self.apply_shake | Handled | [1457](../../crates/crysirror-reallive/src/adapter/extended.rs#L1457) |
| `opcode` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [1484](../../crates/crysirror-reallive/src/adapter/extended.rs#L1484) |
| `sub` | `103 if layered` | Local state/value operations | Handled | [1488](../../crates/crysirror-reallive/src/adapter/extended.rs#L1488) |
| `sub` | `102` | Local state/value operations | Handled | [1494](../../crates/crysirror-reallive/src/adapter/extended.rs#L1494) |
| `sub` | `300` | Local state/value operations | Handled | [1500](../../crates/crysirror-reallive/src/adapter/extended.rs#L1500) |
| `sub` | `other` | Local state/value operations | Partial: unsupported native shake operation {} | [1504](../../crates/crysirror-reallive/src/adapter/extended.rs#L1504) |

### adapter/extended.rs / RealLiveVm::sound_extended

```rust
fn sound_extended(&mut self, m: &mut Machine, h: &mut dyn HostServices, c: &Command) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under (c.op.module, op)=(20, 8)` | `!track.name.is_empty() && h.audio_playing(self.client, 0).map_err(err)?` | self.begin_bgm | Handled | [319](../../crates/crysirror-reallive/src/adapter/extended.rs#L319) |
| `condition; under (c.op.module, op)=(20, 13 \| 14)` | `op == 13` | Local state/value operations | Handled | [332](../../crates/crysirror-reallive/src/adapter/extended.rs#L332) |
| `condition; under (c.op.module, op)=(20, 105 \| 106)` | `op == 106` | Local state/value operations | Handled | [349](../../crates/crysirror-reallive/src/adapter/extended.rs#L349) |
| `op / 1000; under (c.op.module, op)=(20, 1000..=1002 \| 2000..=2002 \| 3000..=3002)` | `1` | Local state/value operations | Handled | [367](../../crates/crysirror-reallive/src/adapter/extended.rs#L367) |
| `op / 1000; under (c.op.module, op)=(20, 1000..=1002 \| 2000..=2002 \| 3000..=3002)` | `2` | Local state/value operations | Handled | [368](../../crates/crysirror-reallive/src/adapter/extended.rs#L368) |
| `op / 1000; under (c.op.module, op)=(20, 1000..=1002 \| 2000..=2002 \| 3000..=3002)` | `_` | Local state/value operations | Handled | [369](../../crates/crysirror-reallive/src/adapter/extended.rs#L369) |
| `condition; under (c.op.module, op)=(21, 13 \| 14)` | `op == 13` | Local state/value operations | Handled | [398](../../crates/crysirror-reallive/src/adapter/extended.rs#L398) |
| `condition; under (c.op.module, op)=(21, 21 \| 22)` | `op == 21` | Local state/value operations | Handled | [417](../../crates/crysirror-reallive/src/adapter/extended.rs#L417) |
| `op; under (c.op.module, op)=(21, 32..=34)` | `_` | Local state/value operations | Handled | [427](../../crates/crysirror-reallive/src/adapter/extended.rs#L427) |
| `op; under (c.op.module, op)=(21, 40..=42)` | `_` | Local state/value operations | Handled | [440](../../crates/crysirror-reallive/src/adapter/extended.rs#L440) |
| `condition; under (c.op.module, op)=(21, 40..=42)` | `mode == Mode::Random && ints.len() >= 3` | Local state/value operations | Handled | [448](../../crates/crysirror-reallive/src/adapter/extended.rs#L448) |
| `condition; under (c.op.module, op)=(21, 50 \| 51)` | `op == 50` | Local state/value operations | Handled | [467](../../crates/crysirror-reallive/src/adapter/extended.rs#L467) |
| `op; under (c.op.module, op)=(22, 12..=14)` | `_` | Local state/value operations | Handled | [526](../../crates/crysirror-reallive/src/adapter/extended.rs#L526) |
| `condition; under (c.op.module, op)=(23, 13 \| 14)` | `op == 13` | Local state/value operations | Handled | [542](../../crates/crysirror-reallive/src/adapter/extended.rs#L542) |
| `(c.op.module, op)` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [571](../../crates/crysirror-reallive/src/adapter/extended.rs#L571) |

### adapter/extended.rs / RealLiveVm::step_pan

```rust
fn step_pan(&mut self, h: &mut dyn HostServices, pan: &Pan, now: u64, fast: bool) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `pan.kind == 406` | f64::from, self.canvas | Handled | [1668](../../crates/crysirror-reallive/src/adapter/extended.rs#L1668) |
| `condition` | `pan.kind == 407` | Local state/value operations | Handled | [1692](../../crates/crysirror-reallive/src/adapter/extended.rs#L1692) |

### adapter/extended.rs / RealLiveVm::system_extended

```rust
fn system_extended(&mut self, m: &mut Machine, h: &mut dyn HostServices, c: &Command) -> Result<Option<Next>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under opcode=115 \| 125 \| 116 \| 126` | `opcode % 10 == 5` | i32::from | Handled | [1170](../../crates/crysirror-reallive/src/adapter/extended.rs#L1170) |
| `condition; under opcode=180..=185` | `matches!(opcode, 181 \| 182 \| 184 \| 185)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [1182](../../crates/crysirror-reallive/src/adapter/extended.rs#L1182) |
| `condition; under opcode=462..=469` | `opcode % 2 == 1` | self.window_anm_mut | Handled | [1208](../../crates/crysirror-reallive/src/adapter/extended.rs#L1208) |
| `condition; under opcode=470 \| 471` | `opcode == 470` | Local state/value operations | Handled | [1221](../../crates/crysirror-reallive/src/adapter/extended.rs#L1221) |
| `condition; under opcode=480 \| 481` | `opcode == 481` | self.ext.windows_off.insert | Handled | [1242](../../crates/crysirror-reallive/src/adapter/extended.rs#L1242) |
| `condition; under opcode=1201..=1203` | `opcode != 1203` | self.stop_channel | Handled | [1266](../../crates/crysirror-reallive/src/adapter/extended.rs#L1266) |
| `opcode` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [1279](../../crates/crysirror-reallive/src/adapter/extended.rs#L1279) |

### adapter/extended.rs / RealLiveVm::text_input

```rust
fn text_input(&mut self, m: &mut Machine, h: &mut dyn HostServices, c: &Command) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `c.op.opcode` | `_` | Local state/value operations | Unsupported: unsupported native text input operation {} | [1346](../../crates/crysirror-reallive/src/adapter/extended.rs#L1346) |

### adapter/objects.rs / ObjectAssets::gan

```rust
fn gan(&self, name: &str) -> Result<Vec<u8>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `name.to_ascii_lowercase().ends_with(".gan")` | Local state/value operations | Handled | [65](../../crates/crysirror-reallive/src/adapter/objects.rs#L65) |

### adapter/selection.rs / RealLiveVm::frame_geometry

```rust
fn frame_geometry(&self, m: &Machine, window: u32, content: Option<[i32; 2]>) -> Result<FrameGeometry>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(kind, Some(3 \| 4))` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [178](../../crates/crysirror-reallive/src/adapter/selection.rs#L178) |

### adapter/selection.rs / RealLiveVm::poll_selection

```rust
fn poll_selection(&mut self, m: &mut Machine, h: &mut dyn HostServices, input: InputFrame, s: &mut Selection) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(event.kind.as_str(), "activate" \| "click")` | Local state/value operations | Handled | [608](../../crates/crysirror-reallive/src/adapter/selection.rs#L608) |

### adapter/selection.rs / RealLiveVm::selection

```rust
fn selection(&mut self, m: &mut Machine, h: &mut dyn HostServices, c: &Command) -> Result<Next>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `c.op.opcode` | `_` | self.window_selection | Handled | [300](../../crates/crysirror-reallive/src/adapter/selection.rs#L300) |

### adapter/window_buttons.rs / RealLiveVm::tick_window_buttons

```rust
fn tick_window_buttons(
        &mut self,
        m: &mut Machine,
        h: &mut dyn HostServices,
        input: InputFrame,
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under kind=Kind::Voice` | `id >= 0` | self.play_koe | Handled | [380](../../crates/crysirror-reallive/src/adapter/window_buttons.rs#L380) |

### adapter/window_buttons.rs / RealLiveVm::window_button_specs

```rust
fn window_button_specs(&self, m: &Machine) -> Result<Vec<(u32, Kind, Value, Value)>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(kind, Kind::Extra(_))` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [173](../../crates/crysirror-reallive/src/adapter/window_buttons.rs#L173) |
| `condition` | `m.gameexe.int(&global).unwrap_or(1) == 0 \|\| m.gameexe.int(&format!("{prefix}{name}_USE")) == Some(0)` | Local state/value operations | Handled | [178](../../crates/crysirror-reallive/src/adapter/window_buttons.rs#L178) |

### resources.rs / Resources::find

```rust
fn find(&mut self, exe: &Gameexe, kind: &str, name: &str) -> Result<String>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `kind` | `"image"` | Local state/value operations | Handled | [109](../../crates/crysirror-reallive/src/resources.rs#L109) |
| `kind` | `"bgm"` | Local state/value operations | Handled | [110](../../crates/crysirror-reallive/src/resources.rs#L110) |
| `kind` | `"wav"` | Local state/value operations | Handled | [111](../../crates/crysirror-reallive/src/resources.rs#L111) |
| `kind` | `"koe"` | Local state/value operations | Handled | [112](../../crates/crysirror-reallive/src/resources.rs#L112) |
| `kind` | `"movie"` | Local state/value operations | Handled | [113](../../crates/crysirror-reallive/src/resources.rs#L113) |
| `kind` | `_` | Local state/value operations | Unsupported: unknown resource kind | [114](../../crates/crysirror-reallive/src/resources.rs#L114) |
| `condition` | `name.rsplit('/').next().unwrap_or("").contains('.')` | Local state/value operations | Handled | [129](../../crates/crysirror-reallive/src/resources.rs#L129) |

### vm/bytecode.rs / flow_kind

```rust
fn flow_kind(module: u8, opcode: u16) -> Option<Flow>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `(module, opcode)` | `(1, 0 \| 5) \| (5, 1 \| 5) \| (6, 1 \| 5)` | Local state/value operations | Handled | [279](../../crates/crysirror-reallive/src/vm/bytecode.rs#L279) |
| `(module, opcode)` | `(1, 1 \| 2 \| 6 \| 7) \| (5, 2 \| 6 \| 7) \| (6, 0 \| 2 \| 6 \| 7)` | Local state/value operations | Handled | [280](../../crates/crysirror-reallive/src/vm/bytecode.rs#L280) |
| `(module, opcode)` | `(1 \| 5 \| 6, 3 \| 8)` | Local state/value operations | Handled | [281](../../crates/crysirror-reallive/src/vm/bytecode.rs#L281) |
| `(module, opcode)` | `(1 \| 5 \| 6, 4 \| 9)` | Local state/value operations | Handled | [282](../../crates/crysirror-reallive/src/vm/bytecode.rs#L282) |
| `(module, opcode)` | `(1 \| 6, 16)` | Local state/value operations | Handled | [283](../../crates/crysirror-reallive/src/vm/bytecode.rs#L283) |
| `(module, opcode)` | `(2, 0..=3 \| 16)` | Local state/value operations | Handled | [284](../../crates/crysirror-reallive/src/vm/bytecode.rs#L284) |
| `(module, opcode)` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [285](../../crates/crysirror-reallive/src/vm/bytecode.rs#L285) |

### vm/bytecode.rs / Reader<'_>::command

```rust
fn command(&mut self) -> Result<Element>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `op.modtype == 0` | Local state/value operations | Handled | [399](../../crates/crysirror-reallive/src/vm/bytecode.rs#L399) |

### vm/clock.rs / FrameKind::from_opcode

```rust
fn from_opcode(opcode: u16) -> Self
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `opcode % 10` | `1` | Local state/value operations | Handled | [18](../../crates/crysirror-reallive/src/vm/clock.rs#L18) |
| `opcode % 10` | `2` | Local state/value operations | Handled | [19](../../crates/crysirror-reallive/src/vm/clock.rs#L19) |
| `opcode % 10` | `3` | Local state/value operations | Handled | [20](../../crates/crysirror-reallive/src/vm/clock.rs#L20) |
| `opcode % 10` | `4` | Local state/value operations | Handled | [21](../../crates/crysirror-reallive/src/vm/clock.rs#L21) |
| `opcode % 10` | `_` | Local state/value operations | Handled | [22](../../crates/crysirror-reallive/src/vm/clock.rs#L22) |

### vm/machine.rs / Machine::apply_memory_initializers

```rust
fn apply_memory_initializers(&mut self, local: bool) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Some(letters) = key.strip_prefix(if local { "LOCALNAME." } else { "NAME." })` | Memory::name_index, self.memory.set_name | Handled | [229](../../crates/crysirror-reallive/src/vm/machine.rs#L229) |

### vm/machine.rs / Machine::eval_int

```rust
fn eval_int(&mut self, expression: &Expr) -> Result<i32>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under expression=Expr::Unary(operation, operand)` | `*operation == 0x01` | Local state/value operations | Handled | [721](../../crates/crysirror-reallive/src/vm/machine.rs#L721) |

### vm/machine.rs / Machine::new

```rust
fn new(archive: Rc<Archive>, gameexe: Rc<Gameexe>) -> Result<Self>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let (Some(slot), Some(name)) = (entry.key_number(1), entry.str(0))` | Local state/value operations | Handled | [204](../../crates/crysirror-reallive/src/vm/machine.rs#L204) |

### vm/modules/jmp.rs / dispatch

```rust
fn dispatch(machine: &mut Machine, command: &Command) -> Result<Next>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under (op.opcode, op.overload)=(17, _)` | `op.module != 6 && op.overload == 0` | Local state/value operations | Handled | [121](../../crates/crysirror-reallive/src/vm/modules/jmp.rs#L121) |
| `condition; under (op.opcode, op.overload)=(19, _)` | `op.module != 6 && op.overload == 0` | Local state/value operations | Handled | [149](../../crates/crysirror-reallive/src/vm/modules/jmp.rs#L149) |
| `condition; under (op.opcode, op.overload)=(102 \| 103, _)` | `op.opcode == 103 && value.is_string()` | Local state/value operations | Handled | [182](../../crates/crysirror-reallive/src/vm/modules/jmp.rs#L182) |
| `condition; under (op.opcode, op.overload)=(102 \| 103, _)` | `op.opcode == 103` | Local state/value operations | Handled | [185](../../crates/crysirror-reallive/src/vm/modules/jmp.rs#L185) |
| `(op.opcode, op.overload)` | `_` | Local state/value operations | Handled | [202](../../crates/crysirror-reallive/src/vm/modules/jmp.rs#L202) |

### vm/modules/mem.rs / dispatch

```rust
fn dispatch(machine: &mut Machine, command: &Command) -> Result<Next>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under (op.opcode, op.overload)=(210 \| 211, _)` | `command.params.len() > 2` | Local state/value operations | Handled | [144](../../crates/crysirror-reallive/src/vm/modules/mem.rs#L144) |
| `(op.opcode, op.overload)` | `_` | Local state/value operations | Handled | [163](../../crates/crysirror-reallive/src/vm/modules/mem.rs#L163) |

### vm/modules/mod.rs / dispatch

```rust
fn dispatch(machine: &mut Machine, command: &Command) -> anyhow::Result<Next>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `(command.op.modtype, command.op.module)` | `(0, 2) if !matches!(command.kind, super::bytecode::CommandKind::Select(_))` | sel::dispatch | Handled | [17](../../crates/crysirror-reallive/src/vm/modules/mod.rs#L17) |
| `(command.op.modtype, command.op.module)` | `(1, 10) if command.op.opcode != 100` | str::dispatch | Handled | [18](../../crates/crysirror-reallive/src/vm/modules/mod.rs#L18) |
| `(command.op.modtype, command.op.module)` | `(1, 11) if !(300..=307).contains(&command.op.opcode)` | mem::dispatch | Handled | [19](../../crates/crysirror-reallive/src/vm/modules/mod.rs#L19) |
| `(command.op.modtype, command.op.module)` | `(1 \| 2, 71 \| 72) if matches!(command.op.opcode,1000..=1004\|1300\|1400) && machine.sys.gfx.loader.as_ref().is_some_and(\|l\|l.owner_image_preparation())` | Local state/value operations | Handled | [21](../../crates/crysirror-reallive/src/vm/modules/mod.rs#L21) |
| `(command.op.modtype, command.op.module)` | `_` | Local state/value operations | Handled | [37](../../crates/crysirror-reallive/src/vm/modules/mod.rs#L37) |

### vm/modules/obj.rs / add_or_stop

```rust
fn add_or_stop(machine: &mut Machine, command: &Command, t: &Targets) -> Result<Next>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `(1200..=1216).contains(&opcode)` | i32::from | Handled | [680](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L680) |

### vm/modules/obj.rs / animation

```rust
fn animation(machine: &mut Machine, command: &Command) -> Result<Next>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `(100..=106).contains(&opcode) && opcode != 104` | i32::from | Handled | [987](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L987) |
| `opcode` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [1010](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1010) |
| `condition` | `opcode == 104` | Box::new | Handled | [1014](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1014) |
| `condition; under opcode=1 \| 2` | `opcode == 1` | Local state/value operations | Handled | [1039](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1039) |
| `opcode; under opcode=3101..=3109` | `_` | Local state/value operations | Handled | [1056](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1056) |
| `condition; under opcode=3101..=3109` | `matches!(opcode, 3106 \| 3107)` | Box::new | Handled | [1078](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1078) |
| `condition; under opcode=3101..=3109` | `matches!(opcode, 3106 \| 3107)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [1078](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1078) |
| `condition; under opcode=3050 \| 3051` | `opcode == 3051` | Box::new | Handled | [1099](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1099) |
| `opcode % 1000; under opcode=_` | `1 \| 2` | Local state/value operations | Handled | [1129](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1129) |
| `opcode % 1000; under opcode=_` | `3 \| 4` | Local state/value operations | Handled | [1130](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1130) |
| `opcode % 1000; under opcode=_` | `5` | Local state/value operations | Handled | [1131](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1131) |
| `opcode % 1000; under opcode=_` | `6` | Local state/value operations | Handled | [1132](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1132) |
| `opcode % 1000; under opcode=_` | `7` | Local state/value operations | Handled | [1133](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1133) |
| `opcode % 1000; under opcode=_` | `8 \| 9` | Local state/value operations | Handled | [1134](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1134) |
| `opcode % 1000; under opcode=_` | `_` | Local state/value operations | Handled | [1135](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1135) |
| `opcode` | `_` | Box::new | Handled | [1127](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1127) |

### vm/modules/obj.rs / creation

```rust
fn creation(machine: &mut Machine, command: &Command) -> Result<Next>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `command.op.opcode; under command.op.opcode=1000..=1002 \| 1300 \| 1400` | `_` | Local state/value operations | Handled | [912](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L912) |
| `command.op.opcode` | `_` | Local state/value operations | Handled | [976](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L976) |

### vm/modules/obj.rs / creation_image_name

```rust
fn creation_image_name(machine:&mut Machine,command:&Command) -> Result<Option<String>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(command.op.opcode,1000..=1004\|1300\|1400)` | Local state/value operations | Handled | [898](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L898) |

### vm/modules/obj.rs / getters

```rust
fn getters(machine: &mut Machine, command: &Command) -> Result<Next>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under command.op.opcode=1007 \| 1008` | `command.op.opcode == 1007` | Local state/value operations | Handled | [1194](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1194) |
| `command.op.opcode; under command.op.opcode=1005 \| 1022 \| 1035 \| 1070 \| 1072 \| 1023 \| 1034 \| 1071` | `_` | Local state/value operations | Handled | [1212](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1212) |
| `condition; under command.op.opcode=1005 \| 1022 \| 1035 \| 1070 \| 1072 \| 1023 \| 1034 \| 1071` | `matches!(command.op.opcode, 1023 \| 1034 \| 1071)` | Local state/value operations | Handled | [1215](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1215) |
| `command.op.opcode` | `_` | Local state/value operations | Handled | [1336](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1336) |

### vm/modules/obj.rs / management

```rust
fn management(machine: &mut Machine, command: &Command) -> Result<Next>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(opcode, 100..=102 \| 110..=113 \| 115 \| 116)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [1366](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1366) |
| `condition; under opcode=15 \| 16 / machine.sys.gfx.object(r).and_then(\|o\| o.data.as_ref())=Some(ObjectData::Gan { image_name, gan_name, .. })` | `opcode == 15` | Local state/value operations | Handled | [1408](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1408) |
| `condition; under opcode=15 \| 16` | `opcode == 16 && !matches!( machine.sys.gfx.object(r).and_then(\|o\| o.data.as_ref()), Some(ObjectData::Gan { .. }) )` | String::new | Handled | [1416](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1416) |
| `(module, opcode); under opcode=2 \| 3 \| 14` | `(60, _)` | Local state/value operations | Handled | [1440](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1440) |
| `(module, opcode); under opcode=2 \| 3 \| 14` | `(61, 2) \| (_, 14) if module == 61` | Local state/value operations | Handled | [1441](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1441) |
| `(module, opcode); under opcode=2 \| 3 \| 14` | `(61, _)` | Local state/value operations | Handled | [1442](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1442) |
| `(module, opcode); under opcode=2 \| 3 \| 14` | `(62, 2)` | Local state/value operations | Handled | [1443](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1443) |
| `(module, opcode); under opcode=2 \| 3 \| 14` | `_` | Local state/value operations | Handled | [1444](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1444) |
| `condition; under opcode=0 \| 1 \| 4 \| 5 \| 10..=13 \| 100 \| 101 \| 110..=113` | `everything \|\| (module == 60 && opcode == 1 && count <= first)` | Local state/value operations | Handled | [1455](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1455) |
| `condition; under opcode=0 \| 1 \| 4 \| 5 \| 10..=13 \| 100 \| 101 \| 110..=113` | `machine.sys.gfx.object(r).is_none() && !matches!(opcode, 4 \| 5)` | Local state/value operations | Handled | [1466](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1466) |
| `condition; under opcode=0 \| 1 \| 4 \| 5 \| 10..=13 \| 100 \| 101 \| 110..=113` | `matches!(opcode, 4 \| 5)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [1466](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1466) |
| `opcode; under opcode=0 \| 1 \| 4 \| 5 \| 10..=13 \| 100 \| 101 \| 110..=113` | `_` | Local state/value operations | Handled | [1483](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1483) |
| `opcode` | `_` | Local state/value operations | Handled | [1488](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L1488) |

### vm/modules/obj.rs / object_event

```rust
fn object_event(machine: &mut Machine, t: &Targets, opcode: u16, v: &[i32]) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `opcode` | `_` | Local state/value operations | Handled | [777](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L777) |
| `opcode` | `_ if flash` | Local state/value operations | Handled | [795](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L795) |
| `opcode` | `_` | Local state/value operations | Handled | [796](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L796) |

### vm/modules/obj.rs / properties

```rust
fn properties(machine: &mut Machine, command: &Command) -> Result<Next>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(command.op.module, 87 \| 88 \| 93 \| 94)` | Local state/value operations | Handled | [468](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L468) |
| `condition` | `(2200..=2216).contains(&command.op.opcode)` | Local state/value operations | Handled | [478](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L478) |
| `command.op.opcode` | `1005 if v.len() >= 4` | Local state/value operations | Handled | [483](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L483) |
| `command.op.opcode` | `1005 \| 1034 \| 1035 if v.is_empty()` | Local state/value operations | Handled | [484](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L484) |
| `command.op.opcode` | `1035 if v.len() >= 4` | Local state/value operations | Handled | [488](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L488) |
| `command.op.opcode` | `1072 if v.is_empty()` | Local state/value operations | Handled | [634](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L634) |
| `command.op.opcode` | `1070 \| 1071 if v.is_empty()` | Local state/value operations | Handled | [665](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L665) |
| `command.op.opcode` | `_` | Local state/value operations | Handled | [670](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L670) |

### vm/modules/obj.rs / property_function

```rust
fn property_function(machine: &mut Machine, command: &Command, t: &Targets) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!(1..=6).contains(&kind) \|\| (family(base).is_none() && !matches!(base, 4 \| 21) && !event)` | Local state/value operations | Handled | [412](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L412) |
| `condition` | `(base == 21 \|\| event) && !(3..=6).contains(&kind)` | Local state/value operations | Handled | [415](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L415) |
| `condition` | `kind == 1 && matches!(base, 5 \| 34 \| 35 \| 70 \| 71 \| 72)` | Local state/value operations | Handled | [419](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L419) |
| `kind` | `1` | Local state/value operations | Handled | [434](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L434) |
| `kind` | `2 if base == 4` | Local state/value operations | Handled | [435](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L435) |
| `kind` | `2` | Local state/value operations | Handled | [436](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L436) |
| `kind` | `3` | i32::from | Handled | [437](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L437) |
| `kind` | `4 \| 5` | Box::new | Handled | [446](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L446) |
| `kind` | `_` | Local state/value operations | Handled | [452](../../crates/crysirror-reallive/src/vm/modules/obj.rs#L452) |

### vm/modules/sel.rs / dispatch

```rust
fn dispatch(machine: &mut Machine, command: &Command) -> Result<Next>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `command.op.opcode` | `_` | Local state/value operations | Handled | [70](../../crates/crysirror-reallive/src/vm/modules/sel.rs#L70) |

### vm/modules/str.rs / dispatch

```rust
fn dispatch(machine: &mut Machine, command: &Command) -> Result<Next>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under (op.opcode, op.overload)=(5 \| 6, overload)` | `op.opcode == 6` | Local state/value operations | Handled | [140](../../crates/crysirror-reallive/src/vm/modules/str.rs#L140) |
| `op.opcode; under (op.opcode, op.overload)=(10..=13, overload)` | `_` | Local state/value operations | Handled | [178](../../crates/crysirror-reallive/src/vm/modules/str.rs#L178) |
| `condition; under (op.opcode, op.overload)=(14..=17, overload)` | `matches!(op.opcode, 14 \| 15)` | Local state/value operations | Handled | [191](../../crates/crysirror-reallive/src/vm/modules/str.rs#L191) |
| `condition; under (op.opcode, op.overload)=(14..=17, overload)` | `matches!(op.opcode, 14 \| 16)` | nls::han_to_zen | Handled | [197](../../crates/crysirror-reallive/src/vm/modules/str.rs#L197) |
| `condition; under (op.opcode, op.overload)=(30 \| 31, _)` | `op.opcode == 30` | Local state/value operations | Handled | [229](../../crates/crysirror-reallive/src/vm/modules/str.rs#L229) |
| `condition; under (op.opcode, op.overload)=(40 \| 41, _)` | `command.op.opcode == 40` | Local state/value operations | Handled | [272](../../crates/crysirror-reallive/src/vm/modules/str.rs#L272) |
| `(op.opcode, op.overload)` | `_` | Local state/value operations | Handled | [302](../../crates/crysirror-reallive/src/vm/modules/str.rs#L302) |

### vm/modules/sys.rs / dispatch

```rust
fn dispatch(machine: &mut Machine, command: &Command) -> Result<Option<Next>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under op.opcode=1004 \| 1010 \| 1012 \| 1013` | `matches!(op.opcode, 1004 \| 1012)` | Local state/value operations | Handled | [133](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L133) |
| `condition; under op.opcode=1005 \| 1006` | `op.opcode == 1005` | Local state/value operations | Handled | [145](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L145) |
| `op.opcode - base; under op.opcode=430..=457` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [267](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L267) |
| `condition; under op.opcode=1211 \| 1212` | `op.opcode == 1211` | Local state/value operations | Handled | [279](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L279) |
| `op.opcode` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [355](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L355) |

### vm/modules/sys.rs / frame_layer

```rust
fn frame_layer(opcode: u16) -> usize
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(opcode, 520..=534 \| 620..=634)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [361](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L361) |

### vm/modules/sys.rs / interpolate

```rust
fn interpolate(start: i32, current: i32, end: i32, amount: i32, mode: i32) -> i32
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `mode` | `1` | Local state/value operations | Handled | [381](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L381) |
| `mode` | `2` | Local state/value operations | Handled | [382](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L382) |
| `mode` | `3` | Local state/value operations | Handled | [383](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L383) |
| `mode` | `4` | Local state/value operations | Handled | [384](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L384) |
| `mode` | `_` | Local state/value operations | Handled | [385](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L385) |

### vm/modules/sys.rs / settings_op

```rust
fn settings_op(machine: &mut Machine, command: &Command) -> Result<Next>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `opcode` | `_` | Local state/value operations | Handled | [536](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L536) |
| `condition` | `kind == 2` | Local state/value operations | Handled | [539](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L539) |
| `opcode; under receiver=kind == 2` | `_` | Local state/value operations | Handled | [552](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L552) |
| `condition` | `kind == 0` | Local state/value operations | Handled | [557](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L557) |
| `condition` | `kind == 2` | Local state/value operations | Handled | [568](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L568) |
| `kind` | `1 \| 2` | usize::from, i32::from | Handled | [599](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L599) |
| `kind` | `_` | usize::from | Handled | [622](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L622) |

### vm/modules/sys.rs / timetable2

```rust
fn timetable2(machine: &mut Machine, command: &Command, lengths: bool) -> Result<i32>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `lengths && kind != 49 && !v.is_empty()` | Local state/value operations | Handled | [708](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L708) |
| `kind; under kind=51..=54` | `51` | Local state/value operations | Handled | [748](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L748) |
| `kind; under kind=51..=54` | `52 \| 53` | Local state/value operations | Handled | [750](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L750) |
| `kind; under kind=51..=54` | `_` | Local state/value operations | Handled | [752](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L752) |
| `condition; under kind=51..=54` | `kind != 51` | Local state/value operations | Handled | [756](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L756) |
| `kind` | `48` | Local state/value operations | Handled | [723](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L723) |
| `kind` | `49` | Local state/value operations | Handled | [732](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L732) |
| `kind` | `50` | Local state/value operations | Handled | [734](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L734) |
| `kind` | `51..=54` | f64::from | Handled | [741](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L741) |
| `kind` | `55` | Local state/value operations | Handled | [762](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L762) |
| `kind` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [769](../../crates/crysirror-reallive/src/vm/modules/sys.rs#L769) |

### vm/modules/sys_max.rs / cg_query

```rust
fn cg_query(machine: &mut Machine, command: &Command) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `cg_flag(machine, &name); under command.op.opcode=opcode` | `None` | Local state/value operations | Handled | [471](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L471) |
| `cg_flag(machine, &name); under command.op.opcode=opcode` | `Some(flag) if opcode == 1503` | Local state/value operations | Handled | [472](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L472) |
| `cg_flag(machine, &name); under command.op.opcode=opcode` | `Some(flag)` | i32::from | Handled | [473](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L473) |
| `command.op.opcode` | `opcode` | i32::from | Handled | [468](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L468) |

### vm/modules/sys_max.rs / cg_table

```rust
fn cg_table(machine: &mut Machine, command: &Command) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `opcode` | `_` | i32::from | Handled | [559](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L559) |
| `kind; under opcode=_` | `2` | Local state/value operations | Handled | [597](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L597) |
| `kind; under opcode=_` | `4` | Local state/value operations | Handled | [598](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L598) |
| `kind; under opcode=_` | `3` | Local state/value operations | Handled | [600](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L600) |
| `kind; under opcode=_` | `5` | Local state/value operations | Handled | [602](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L602) |
| `kind; under opcode=_` | `_` | Local state/value operations | Handled | [606](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L606) |
| `opcode` | `_` | Local state/value operations | Handled | [593](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L593) |

### vm/modules/sys_max.rs / dispatch

```rust
fn dispatch(machine: &mut Machine, command: &Command) -> Result<Option<Next>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `opcode; under opcode=135..=137` | `_` | Local state/value operations | Handled | [150](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L150) |
| `condition; under opcode=151 \| 152` | `opcode == 151` | Local state/value operations | Handled | [179](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L179) |
| `condition; under opcode=459` | `command.params.len() >= 2` | Local state/value operations | Handled | [232](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L232) |
| `opcode; under opcode=1015..=1017` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [282](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L282) |
| `condition; under opcode=1304 \| 1305 \| 1314 \| 1315` | `opcode % 10 == 4` | Local state/value operations | Handled | [310](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L310) |
| `opcode` | `_` | Local state/value operations | Handled | [347](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L347) |

### vm/modules/sys_max.rs / event_loop

```rust
fn event_loop(machine: &mut Machine, command: &Command) -> Result<Option<Next>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `opcode; under opcode=320 \| 324 \| 326` | `_` | Local state/value operations | Handled | [707](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L707) |
| `opcode` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [788](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L788) |

### vm/modules/sys_max.rs / strip_extension

```rust
fn strip_extension(name: &str) -> &str
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `name.rfind('.')` | `Some(dot) if name.len() - dot <= 4` | Local state/value operations | Handled | [395](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L395) |
| `name.rfind('.')` | `_` | Local state/value operations | Handled | [396](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L396) |

### vm/modules/sys_max.rs / system_default

```rust
fn system_default(machine: &Machine, opcode: u16) -> i32
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `opcode` | `_` | Local state/value operations | Handled | [378](../../crates/crysirror-reallive/src/vm/modules/sys_max.rs#L378) |

### vm/nls.rs / Nls::from_rldev

```rust
fn from_rldev(code: u8) -> Option<Nls>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `code` | `0` | Local state/value operations | Handled | [66](../../crates/crysirror-reallive/src/vm/nls.rs#L66) |
| `code` | `1` | Local state/value operations | Handled | [67](../../crates/crysirror-reallive/src/vm/nls.rs#L67) |
| `code` | `2` | Local state/value operations | Handled | [68](../../crates/crysirror-reallive/src/vm/nls.rs#L68) |
| `code` | `3` | Local state/value operations | Handled | [69](../../crates/crysirror-reallive/src/vm/nls.rs#L69) |
| `code` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [70](../../crates/crysirror-reallive/src/vm/nls.rs#L70) |

### vm/nls.rs / Nls::from_str

```rust
fn from_str(text: &str) -> anyhow::Result<Self>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `key.as_str()` | `"sjis" \| "shiftjis" \| "cp932" \| "ms932" \| "ja"` | Local state/value operations | Handled | [159](../../crates/crysirror-reallive/src/vm/nls.rs#L159) |
| `key.as_str()` | `"gbk" \| "gb2312" \| "gb18030" \| "cp936" \| "chs" \| "zhcn"` | Local state/value operations | Handled | [160](../../crates/crysirror-reallive/src/vm/nls.rs#L160) |
| `key.as_str()` | `"big5" \| "cp950" \| "cht" \| "zhtw"` | Local state/value operations | Handled | [161](../../crates/crysirror-reallive/src/vm/nls.rs#L161) |
| `key.as_str()` | `"utf8" \| "utf" \| "unicode"` | Local state/value operations | Handled | [162](../../crates/crysirror-reallive/src/vm/nls.rs#L162) |
| `key.as_str()` | `"western" \| "cp1252" \| "latin1" \| "en"` | Local state/value operations | Handled | [163](../../crates/crysirror-reallive/src/vm/nls.rs#L163) |
| `key.as_str()` | `"korean" \| "cp949" \| "euckr" \| "ko"` | Local state/value operations | Handled | [164](../../crates/crysirror-reallive/src/vm/nls.rs#L164) |
| `key.as_str()` | `_` | Local state/value operations | Handled | [165](../../crates/crysirror-reallive/src/vm/nls.rs#L165) |

### vm/opcodes.rs / name

```rust
fn name(op: Opcode) -> Option<&'static str>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `(op.modtype, op.module)` | `(0, 5 \| 6)` | Local state/value operations | Handled | [105](../../crates/crysirror-reallive/src/vm/opcodes.rs#L105) |
| `(op.modtype, op.module)` | `(_, module)` | Local state/value operations | Handled | [106](../../crates/crysirror-reallive/src/vm/opcodes.rs#L106) |

### vm/save.rs / load_global

```rust
fn load_global(machine: &mut Machine, store: &mut dyn Store) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `name.as_str()` | `"memory"` | Local state/value operations | Handled | [241](../../crates/crysirror-reallive/src/vm/save.rs#L241) |
| `name.as_str()` | `"kidoku"` | Local state/value operations | Handled | [247](../../crates/crysirror-reallive/src/vm/save.rs#L247) |
| `name.as_str()` | `"misc"` | Local state/value operations | Handled | [256](../../crates/crysirror-reallive/src/vm/save.rs#L256) |
| `name.as_str()` | `"settings"` | crate::vm::settings_io::read | Handled | [257](../../crates/crysirror-reallive/src/vm/save.rs#L257) |
| `name.as_str()` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [258](../../crates/crysirror-reallive/src/vm/save.rs#L258) |

### vm/save.rs / read_header

```rust
fn read_header(bytes: &[u8]) -> Result<SaveHeader>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `name == "header"` | Reader::new | Handled | [141](../../crates/crysirror-reallive/src/vm/save.rs#L141) |

### vm/save.rs / sys_query

```rust
fn sys_query(machine: &mut Machine, store: &mut dyn Store, command: &Command) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `opcode == 1421` | Local state/value operations | Handled | [279](../../crates/crysirror-reallive/src/vm/save.rs#L279) |
| `condition` | `opcode == 1409` | i32::from | Handled | [284](../../crates/crysirror-reallive/src/vm/save.rs#L284) |
| `condition` | `opcode == 1414` | LocalMemory::default, Reader::new, Memory::default, crate::vm::memory::FrameMemory::default, Machine::int_range | Handled | [288](../../crates/crysirror-reallive/src/vm/save.rs#L288) |
| `condition; under opcode=_` | `command.params.len() > 9` | Local state/value operations | Handled | [361](../../crates/crysirror-reallive/src/vm/save.rs#L361) |
| `opcode` | `1410` | Local state/value operations | Handled | [356](../../crates/crysirror-reallive/src/vm/save.rs#L356) |
| `opcode` | `1411` | Local state/value operations | Handled | [357](../../crates/crysirror-reallive/src/vm/save.rs#L357) |
| `opcode` | `1412` | Local state/value operations | Handled | [358](../../crates/crysirror-reallive/src/vm/save.rs#L358) |
| `opcode` | `_` | Local state/value operations | Handled | [359](../../crates/crysirror-reallive/src/vm/save.rs#L359) |
