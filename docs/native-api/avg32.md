# AVG32 developer reference

[All engines](README.md)

Mode: `adopt`.

## ABI and host contract

AVG32 scenarios use byte commands and command-specific subcommands, not the
RealLive module ABI. Native values, flags, strings, Seen transitions and return
stack live in `vm/scenario.rs`. The decoder's boolean result controls whether
the current interpreter pass yields; `self.cmd` retains an unfinished command.
Some unrecognized subcommands warn and continue: those branches are shown as
partial rather than original no-ops.

| Native area | CrySirror service / implementation | Return / completion |
| --- | --- | --- |
| Scenario control, flags and native windows | `vm/scenario.rs`, `vm/meswin.rs` | Native command/subcommand and wait state. |
| PDT buffers and graphics operations | `crysirror_image::avg32` / PDT decoder | Shared CPU raster resources; presented dynamic surface becomes an engine Layer. |
| Text, choices, menus and backlog | Engine GlyphRun / solid Layers | Native message grid, reveal and selection policy. |
| `0x66` work-buffer text | Glyph-image job and PDT coverage blending | Coverage completes before scenario continuation. |
| Sound and movies | Shared Audio; AVI/Cinepak through shared Video | Playback follows engine clock and native waits. |
| Files and `SAVE.INI` | Scoped Storage and original slot layout | Framework snapshots are unsupported. |

Graphics composition here uses shared CPU PDT raster operations. It is not a
claim of native GPU composition. Original external bitmap glyphs remain image
resources. Project font metrics differ from native GDI; buffer-0 glyph damage
and copy timing have their recorded limitations. AVI video outside Cinepak
fails; compressed AVI audio is not decoded by this path.

Source/provenance: [AVG32 implementation](../../crates/crysirror-avg32/src/vm/),
[scenario executor](../../crates/crysirror-avg32/src/vm/scenario.rs).
Behavior receipts are independent of this source inventory; no new game run is
claimed by generating these tables.

<!-- BEGIN GENERATED DISPATCH TABLES -->

## Opcode and API implementation tables

`Handled` means an executable source branch exists. It is not a claim that every argument/subselector or a complete game has been audited. Explicit gaps are shown per row.

| Status | Entries |
| --- | ---: |
| Empty handler | 5 |
| Handled | 283 |
| Partial | 9 |

These tables cover the checked-in catalogues and explicit dispatch branches. An engine-wide upstream denominator is not inferred from these rows.

## Scenario opcodes

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x01` | Click wait | Handled | — | [scenario.rs:637](../../crates/crysirror-avg32/src/vm/scenario.rs#L637) |
| `0x02` | Insert message line break | Handled | — | [scenario.rs:638](../../crates/crysirror-avg32/src/vm/scenario.rs#L638) |
| `0x03` | Click wait | Handled | — | [scenario.rs:645](../../crates/crysirror-avg32/src/vm/scenario.rs#L645) |
| `0x04` | Text window | Handled | — | [scenario.rs:646](../../crates/crysirror-avg32/src/vm/scenario.rs#L646) |
| `0x05` | Set message font size | Partial | Unknown subselectors warn and continue | [scenario.rs:647](../../crates/crysirror-avg32/src/vm/scenario.rs#L647) |
| `0x06` | End command and yield | Handled | — | [scenario.rs:656](../../crates/crysirror-avg32/src/vm/scenario.rs#L656) |
| `0x08` | Consume control operands | Partial | Operands consumed without a connected runtime effect | [scenario.rs:660](../../crates/crysirror-avg32/src/vm/scenario.rs#L660) |
| `0x0B` | Graphics load | Handled | — | [scenario.rs:667](../../crates/crysirror-avg32/src/vm/scenario.rs#L667) |
| `0x0C` | Animation | Handled | — | [scenario.rs:668](../../crates/crysirror-avg32/src/vm/scenario.rs#L668) |
| `0x0E` | Sound | Handled | — | [scenario.rs:669](../../crates/crysirror-avg32/src/vm/scenario.rs#L669) |
| `0x10` | Value text | Handled | — | [scenario.rs:670](../../crates/crysirror-avg32/src/vm/scenario.rs#L670) |
| `0x13` | Fade | Handled | — | [scenario.rs:671](../../crates/crysirror-avg32/src/vm/scenario.rs#L671) |
| `0x15` | Branch when condition is false | Handled | — | [scenario.rs:672](../../crates/crysirror-avg32/src/vm/scenario.rs#L672) |
| `0x16` | Call or transfer to another Seen scenario | Handled | — | [scenario.rs:681](../../crates/crysirror-avg32/src/vm/scenario.rs#L681) |
| `0x17` | Start/wait for screen shake | Handled | — | [scenario.rs:694](../../crates/crysirror-avg32/src/vm/scenario.rs#L694) |
| `0x18` | Set font colour | Handled | — | [scenario.rs:709](../../crates/crysirror-avg32/src/vm/scenario.rs#L709) |
| `0x19` | Wait | Handled | — | [scenario.rs:717](../../crates/crysirror-avg32/src/vm/scenario.rs#L717) |
| `0x1A` | End command and yield | Handled | — | [scenario.rs:656](../../crates/crysirror-avg32/src/vm/scenario.rs#L656) |
| `0x1B` | Call local subroutine | Handled | — | [scenario.rs:718](../../crates/crysirror-avg32/src/vm/scenario.rs#L718) |
| `0x1C` | Jump | Handled | — | [scenario.rs:725](../../crates/crysirror-avg32/src/vm/scenario.rs#L725) |
| `0x1D` | Call selected jump-table entry | Handled | — | [scenario.rs:731](../../crates/crysirror-avg32/src/vm/scenario.rs#L731) |
| `0x1E` | Jump to selected jump-table entry | Handled | — | [scenario.rs:731](../../crates/crysirror-avg32/src/vm/scenario.rs#L731) |
| `0x20` | Return or change call stack | Partial | Unknown subselectors warn and continue | [scenario.rs:751](../../crates/crysirror-avg32/src/vm/scenario.rs#L751) |
| `0x22` | End command and yield | Handled | — | [scenario.rs:656](../../crates/crysirror-avg32/src/vm/scenario.rs#L656) |
| `0x23` | End command and yield | Handled | — | [scenario.rs:656](../../crates/crysirror-avg32/src/vm/scenario.rs#L656) |
| `0x24` | End command and yield | Handled | — | [scenario.rs:656](../../crates/crysirror-avg32/src/vm/scenario.rs#L656) |
| `0x25` | End command and yield | Handled | — | [scenario.rs:656](../../crates/crysirror-avg32/src/vm/scenario.rs#L656) |
| `0x26` | End command and yield | Handled | — | [scenario.rs:656](../../crates/crysirror-avg32/src/vm/scenario.rs#L656) |
| `0x27` | End command and yield | Handled | — | [scenario.rs:656](../../crates/crysirror-avg32/src/vm/scenario.rs#L656) |
| `0x28` | End command and yield | Handled | — | [scenario.rs:656](../../crates/crysirror-avg32/src/vm/scenario.rs#L656) |
| `0x29` | End command and yield | Handled | — | [scenario.rs:656](../../crates/crysirror-avg32/src/vm/scenario.rs#L656) |
| `0x2C` | Legacy | Handled | — | [scenario.rs:779](../../crates/crysirror-avg32/src/vm/scenario.rs#L779) |
| `0x2D` | Legacy | Handled | — | [scenario.rs:779](../../crates/crysirror-avg32/src/vm/scenario.rs#L779) |
| `0x2E` | Legacy | Handled | — | [scenario.rs:779](../../crates/crysirror-avg32/src/vm/scenario.rs#L779) |
| `0x2F` | Legacy | Handled | — | [scenario.rs:779](../../crates/crysirror-avg32/src/vm/scenario.rs#L779) |
| `0x30` | Legacy | Handled | — | [scenario.rs:779](../../crates/crysirror-avg32/src/vm/scenario.rs#L779) |
| `0x31` | Legacy | Handled | — | [scenario.rs:779](../../crates/crysirror-avg32/src/vm/scenario.rs#L779) |
| `0x37` | Variable | Handled | — | [scenario.rs:784](../../crates/crysirror-avg32/src/vm/scenario.rs#L784) |
| `0x39` | Variable | Handled | — | [scenario.rs:784](../../crates/crysirror-avg32/src/vm/scenario.rs#L784) |
| `0x3B` | Variable | Handled | — | [scenario.rs:784](../../crates/crysirror-avg32/src/vm/scenario.rs#L784) |
| `0x3C` | Variable | Handled | — | [scenario.rs:784](../../crates/crysirror-avg32/src/vm/scenario.rs#L784) |
| `0x3D` | Variable | Handled | — | [scenario.rs:784](../../crates/crysirror-avg32/src/vm/scenario.rs#L784) |
| `0x3E` | Variable | Handled | — | [scenario.rs:784](../../crates/crysirror-avg32/src/vm/scenario.rs#L784) |
| `0x3F` | Variable | Handled | — | [scenario.rs:784](../../crates/crysirror-avg32/src/vm/scenario.rs#L784) |
| `0x40` | Variable | Handled | — | [scenario.rs:784](../../crates/crysirror-avg32/src/vm/scenario.rs#L784) |
| `0x41` | Variable | Handled | — | [scenario.rs:784](../../crates/crysirror-avg32/src/vm/scenario.rs#L784) |
| `0x42` | Variable | Handled | — | [scenario.rs:784](../../crates/crysirror-avg32/src/vm/scenario.rs#L784) |
| `0x43` | Variable | Handled | — | [scenario.rs:784](../../crates/crysirror-avg32/src/vm/scenario.rs#L784) |
| `0x49` | Variable | Handled | — | [scenario.rs:784](../../crates/crysirror-avg32/src/vm/scenario.rs#L784) |
| `0x4A` | Variable | Handled | — | [scenario.rs:784](../../crates/crysirror-avg32/src/vm/scenario.rs#L784) |
| `0x4B` | Variable | Handled | — | [scenario.rs:784](../../crates/crysirror-avg32/src/vm/scenario.rs#L784) |
| `0x4C` | Variable | Handled | — | [scenario.rs:784](../../crates/crysirror-avg32/src/vm/scenario.rs#L784) |
| `0x4D` | Variable | Handled | — | [scenario.rs:784](../../crates/crysirror-avg32/src/vm/scenario.rs#L784) |
| `0x4E` | Variable | Handled | — | [scenario.rs:784](../../crates/crysirror-avg32/src/vm/scenario.rs#L784) |
| `0x4F` | Variable | Handled | — | [scenario.rs:784](../../crates/crysirror-avg32/src/vm/scenario.rs#L784) |
| `0x50` | Variable | Handled | — | [scenario.rs:784](../../crates/crysirror-avg32/src/vm/scenario.rs#L784) |
| `0x51` | Variable | Handled | — | [scenario.rs:784](../../crates/crysirror-avg32/src/vm/scenario.rs#L784) |
| `0x56` | Variable | Handled | — | [scenario.rs:784](../../crates/crysirror-avg32/src/vm/scenario.rs#L784) |
| `0x57` | Variable | Handled | — | [scenario.rs:784](../../crates/crysirror-avg32/src/vm/scenario.rs#L784) |
| `0x58` | Present choices | Handled | — | [scenario.rs:789](../../crates/crysirror-avg32/src/vm/scenario.rs#L789) |
| `0x59` | String operations | Handled | — | [scenario.rs:790](../../crates/crysirror-avg32/src/vm/scenario.rs#L790) |
| `0x5B` | Write variable/flag sequence | Handled | — | [scenario.rs:795](../../crates/crysirror-avg32/src/vm/scenario.rs#L795) |
| `0x5C` | Fill variable/flag range | Handled | — | [scenario.rs:811](../../crates/crysirror-avg32/src/vm/scenario.rs#L811) |
| `0x5D` | Copy variable/flag range | Handled | — | [scenario.rs:826](../../crates/crysirror-avg32/src/vm/scenario.rs#L826) |
| `0x5E` | Query date/time or current Seen | Handled | — | [scenario.rs:843](../../crates/crysirror-avg32/src/vm/scenario.rs#L843) |
| `0x5F` | Add to variable ranges | Handled | — | [scenario.rs:855](../../crates/crysirror-avg32/src/vm/scenario.rs#L855) |
| `0x60` | System command dispatcher | Handled | — | [scenario.rs:860](../../crates/crysirror-avg32/src/vm/scenario.rs#L860) |
| `0x61` | Name-entry control | Handled | — | [scenario.rs:861](../../crates/crysirror-avg32/src/vm/scenario.rs#L861) |
| `0x63` | Copy graphics regions | Partial | Unknown subselectors warn and continue | [scenario.rs:862](../../crates/crysirror-avg32/src/vm/scenario.rs#L862) |
| `0x64` | Graphics | Handled | — | [scenario.rs:883](../../crates/crysirror-avg32/src/vm/scenario.rs#L883) |
| `0x65` | End command and yield | Handled | — | [scenario.rs:656](../../crates/crysirror-avg32/src/vm/scenario.rs#L656) |
| `0x66` | Write graphics pixel | Handled | — | [scenario.rs:884](../../crates/crysirror-avg32/src/vm/scenario.rs#L884) |
| `0x67` | Graphics | Handled | — | [scenario.rs:883](../../crates/crysirror-avg32/src/vm/scenario.rs#L883) |
| `0x68` | Graphics | Handled | — | [scenario.rs:883](../../crates/crysirror-avg32/src/vm/scenario.rs#L883) |
| `0x69` | Graphics | Handled | — | [scenario.rs:883](../../crates/crysirror-avg32/src/vm/scenario.rs#L883) |
| `0x6A` | Graphics | Handled | — | [scenario.rs:883](../../crates/crysirror-avg32/src/vm/scenario.rs#L883) |
| `0x6C` | Area control | Handled | — | [scenario.rs:894](../../crates/crysirror-avg32/src/vm/scenario.rs#L894) |
| `0x6D` | Mouse control | Handled | — | [scenario.rs:895](../../crates/crysirror-avg32/src/vm/scenario.rs#L895) |
| `0x6E` | Cg mode | Handled | — | [scenario.rs:896](../../crates/crysirror-avg32/src/vm/scenario.rs#L896) |
| `0x6F` | Update cmd | Handled | — | [scenario.rs:897](../../crates/crysirror-avg32/src/vm/scenario.rs#L897) |
| `0x70` | Window config | Handled | — | [scenario.rs:904](../../crates/crysirror-avg32/src/vm/scenario.rs#L904) |
| `0x72` | Window position | Handled | — | [scenario.rs:905](../../crates/crysirror-avg32/src/vm/scenario.rs#L905) |
| `0x73` | System values | Handled | — | [scenario.rs:906](../../crates/crysirror-avg32/src/vm/scenario.rs#L906) |
| `0x74` | Update cmd | Partial | Unknown subselectors warn and continue | [scenario.rs:907](../../crates/crysirror-avg32/src/vm/scenario.rs#L907) |
| `0x75` | Update cmd | Partial | Unknown subselectors warn and continue | [scenario.rs:935](../../crates/crysirror-avg32/src/vm/scenario.rs#L935) |
| `0x76` | Update cmd | Partial | Unknown subselectors warn and continue | [scenario.rs:948](../../crates/crysirror-avg32/src/vm/scenario.rs#L948) |
| `0x7F` | Update cmd | Handled | — | [scenario.rs:963](../../crates/crysirror-avg32/src/vm/scenario.rs#L963) |
| `0xFE` | Text packet | Handled | — | [scenario.rs:968](../../crates/crysirror-avg32/src/vm/scenario.rs#L968) |
| `0xFF` | Text packet | Handled | — | [scenario.rs:968](../../crates/crysirror-avg32/src/vm/scenario.rs#L968) |

## Subcommands / animation / subcommand

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x10` | Name | Handled | — | [scenario.rs:1185](../../crates/crysirror-avg32/src/vm/scenario.rs#L1185) |
| `0x12` | Name | Handled | — | [scenario.rs:1202](../../crates/crysirror-avg32/src/vm/scenario.rs#L1202) |
| `0x13` | Update cmd | Handled | — | [scenario.rs:1212](../../crates/crysirror-avg32/src/vm/scenario.rs#L1212) |
| `0x16` | Name | Handled | — | [scenario.rs:1185](../../crates/crysirror-avg32/src/vm/scenario.rs#L1185) |
| `0x18` | Name | Handled | — | [scenario.rs:1185](../../crates/crysirror-avg32/src/vm/scenario.rs#L1185) |
| `0x20` | Text | Handled | — | [scenario.rs:1213](../../crates/crysirror-avg32/src/vm/scenario.rs#L1213) |
| `0x21` | Update multi anm, cmd | Handled | — | [scenario.rs:1222](../../crates/crysirror-avg32/src/vm/scenario.rs#L1222) |
| `0x24` | Text | Handled | — | [scenario.rs:1213](../../crates/crysirror-avg32/src/vm/scenario.rs#L1213) |
| `0x25` | Update multi anm, cmd | Handled | — | [scenario.rs:1222](../../crates/crysirror-avg32/src/vm/scenario.rs#L1222) |
| `0x30` | Name | Handled | — | [scenario.rs:1202](../../crates/crysirror-avg32/src/vm/scenario.rs#L1202) |

## Subcommands / area_control / self.byte()

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x02` | Control flow (area_control) | Handled | — | [scenario.rs:2474](../../crates/crysirror-avg32/src/vm/scenario.rs#L2474) |
| `0x03` | Control flow (area_control) | Handled | — | [scenario.rs:2497](../../crates/crysirror-avg32/src/vm/scenario.rs#L2497) |
| `0x04` | Control flow (area_control) | Handled | — | [scenario.rs:2502](../../crates/crysirror-avg32/src/vm/scenario.rs#L2502) |
| `0x05` | Control flow (area_control) | Handled | — | [scenario.rs:2502](../../crates/crysirror-avg32/src/vm/scenario.rs#L2502) |
| `0x10` | Control flow (area_control) | Handled | — | [scenario.rs:2517](../../crates/crysirror-avg32/src/vm/scenario.rs#L2517) |
| `0x11` | Control flow (area_control) | Handled | — | [scenario.rs:2526](../../crates/crysirror-avg32/src/vm/scenario.rs#L2526) |
| `0x15` | Control flow (area_control) | Handled | — | [scenario.rs:2535](../../crates/crysirror-avg32/src/vm/scenario.rs#L2535) |
| `0x20` | Control flow (area_control) | Handled | — | [scenario.rs:2541](../../crates/crysirror-avg32/src/vm/scenario.rs#L2541) |

## Subcommands / cg_mode / self.byte()

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x01` | Execute cg mode branch | Handled | — | [scenario.rs:2599](../../crates/crysirror-avg32/src/vm/scenario.rs#L2599) |
| `0x02` | Execute cg mode branch | Handled | — | [scenario.rs:2603](../../crates/crysirror-avg32/src/vm/scenario.rs#L2603) |
| `0x03` | Execute cg mode branch | Handled | — | [scenario.rs:2608](../../crates/crysirror-avg32/src/vm/scenario.rs#L2608) |
| `0x04` | Auto cg mode | Handled | — | [scenario.rs:2613](../../crates/crysirror-avg32/src/vm/scenario.rs#L2613) |
| `0x05` | Execute cg mode branch | Handled | — | [scenario.rs:2621](../../crates/crysirror-avg32/src/vm/scenario.rs#L2621) |

## Subcommands / fade / subcommand

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x01` | Update fade step, fade colour | Handled | — | [scenario.rs:1465](../../crates/crysirror-avg32/src/vm/scenario.rs#L1465) |
| `0x02` | Update fade step, fade colour | Handled | — | [scenario.rs:1465](../../crates/crysirror-avg32/src/vm/scenario.rs#L1465) |
| `0x03` | Colour | Handled | — | [scenario.rs:1475](../../crates/crysirror-avg32/src/vm/scenario.rs#L1475) |
| `0x04` | Colour | Handled | — | [scenario.rs:1475](../../crates/crysirror-avg32/src/vm/scenario.rs#L1475) |
| `0x10` | Update fade step, fade colour, fade count | Handled | — | [scenario.rs:1484](../../crates/crysirror-avg32/src/vm/scenario.rs#L1484) |
| `0x11` | Colour | Handled | — | [scenario.rs:1490](../../crates/crysirror-avg32/src/vm/scenario.rs#L1490) |

## Subcommands / formatted_text / self.byte()

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x01` | Text and message windows (formatted_text) | Handled | — | [scenario.rs:345](../../crates/crysirror-avg32/src/vm/scenario.rs#L345) |
| `0x02` | Text and message windows (formatted_text) | Handled | — | [scenario.rs:349](../../crates/crysirror-avg32/src/vm/scenario.rs#L349) |
| `0x03` | Text and message windows (formatted_text) | Handled | — | [scenario.rs:354](../../crates/crysirror-avg32/src/vm/scenario.rs#L354) |
| `0x11` | Text and message windows (formatted_text) | Handled | — | [scenario.rs:358](../../crates/crysirror-avg32/src/vm/scenario.rs#L358) |

## Subcommands / graphics / self.byte()

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x01` | Colour | Handled | — | [scenario.rs:2256](../../crates/crysirror-avg32/src/vm/scenario.rs#L2256) |
| `0x10` | Colour | Handled | — | [scenario.rs:2262](../../crates/crysirror-avg32/src/vm/scenario.rs#L2262) |

## Subcommands / graphics / subcommand

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x00` | Rect | Handled | — | [scenario.rs:2135](../../crates/crysirror-avg32/src/vm/scenario.rs#L2135) |
| `0x01` | Execute graphics branch | Handled | — | [scenario.rs:2147](../../crates/crysirror-avg32/src/vm/scenario.rs#L2147) |
| `0x02` | Execute graphics branch | Handled | — | [scenario.rs:2072](../../crates/crysirror-avg32/src/vm/scenario.rs#L2072) |
| `0x03` | Rect | Handled | — | [scenario.rs:2160](../../crates/crysirror-avg32/src/vm/scenario.rs#L2160) |
| `0x04` | Execute graphics branch | Handled | — | [scenario.rs:2073](../../crates/crysirror-avg32/src/vm/scenario.rs#L2073) |
| `0x05` | Rect | Handled | — | [scenario.rs:2168](../../crates/crysirror-avg32/src/vm/scenario.rs#L2168) |
| `0x07` | Execute graphics branch | Handled | — | [scenario.rs:2085](../../crates/crysirror-avg32/src/vm/scenario.rs#L2085) |
| `0x08` | Rect | Handled | — | [scenario.rs:2141](../../crates/crysirror-avg32/src/vm/scenario.rs#L2141) |
| `0x10` | Execute graphics branch | Handled | — | [scenario.rs:2074](../../crates/crysirror-avg32/src/vm/scenario.rs#L2074) |
| `0x11` | Execute graphics branch | Handled | — | [scenario.rs:2086](../../crates/crysirror-avg32/src/vm/scenario.rs#L2086) |
| `0x12` | Execute graphics branch | Handled | — | [scenario.rs:2087](../../crates/crysirror-avg32/src/vm/scenario.rs#L2087) |
| `0x15` | Rect | Handled | — | [scenario.rs:2067](../../crates/crysirror-avg32/src/vm/scenario.rs#L2067) |
| `0x20` | Rect | Handled | — | [scenario.rs:2081](../../crates/crysirror-avg32/src/vm/scenario.rs#L2081) |
| `0x21` | Execute graphics branch | Handled | — | [scenario.rs:2203](../../crates/crysirror-avg32/src/vm/scenario.rs#L2203) |
| `0x22` | Colour | Handled | — | [scenario.rs:2204](../../crates/crysirror-avg32/src/vm/scenario.rs#L2204) |
| `0x30` | Rect | Handled | — | [scenario.rs:2091](../../crates/crysirror-avg32/src/vm/scenario.rs#L2091) |
| `0x32` | Start effect | Handled | — | [scenario.rs:2099](../../crates/crysirror-avg32/src/vm/scenario.rs#L2099) |

## Subcommands / graphics_load / subcommand

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x01` | Text | Handled | — | [scenario.rs:1055](../../crates/crysirror-avg32/src/vm/scenario.rs#L1055) |
| `0x02` | Text | Handled | — | [scenario.rs:1062](../../crates/crysirror-avg32/src/vm/scenario.rs#L1062) |
| `0x03` | Text | Handled | — | [scenario.rs:1055](../../crates/crysirror-avg32/src/vm/scenario.rs#L1055) |
| `0x04` | Text | Handled | — | [scenario.rs:1062](../../crates/crysirror-avg32/src/vm/scenario.rs#L1062) |
| `0x05` | Text | Handled | — | [scenario.rs:1055](../../crates/crysirror-avg32/src/vm/scenario.rs#L1055) |
| `0x06` | Text | Handled | — | [scenario.rs:1062](../../crates/crysirror-avg32/src/vm/scenario.rs#L1062) |
| `0x09` | Text | Handled | — | [scenario.rs:1091](../../crates/crysirror-avg32/src/vm/scenario.rs#L1091) |
| `0x10` | Text | Handled | — | [scenario.rs:1091](../../crates/crysirror-avg32/src/vm/scenario.rs#L1091) |
| `0x11` | Text | Handled | — | [scenario.rs:1096](../../crates/crysirror-avg32/src/vm/scenario.rs#L1096) |
| `0x13` | Execute graphics load branch | Empty handler | Original no-op status not established | [scenario.rs:1099](../../crates/crysirror-avg32/src/vm/scenario.rs#L1099) |
| `0x22` | Text | Partial | Unknown subselectors warn and continue | [scenario.rs:1100](../../crates/crysirror-avg32/src/vm/scenario.rs#L1100) |
| `0x24` | Text | Partial | Unknown subselectors warn and continue | [scenario.rs:1100](../../crates/crysirror-avg32/src/vm/scenario.rs#L1100) |
| `0x30` | Execute graphics load branch | Empty handler | Original no-op status not established | [scenario.rs:1151](../../crates/crysirror-avg32/src/vm/scenario.rs#L1151) |
| `0x31` | Execute graphics load branch | Handled | — | [scenario.rs:1152](../../crates/crysirror-avg32/src/vm/scenario.rs#L1152) |
| `0x32` | Execute graphics load branch | Handled | — | [scenario.rs:1152](../../crates/crysirror-avg32/src/vm/scenario.rs#L1152) |
| `0x33` | Execute graphics load branch | Handled | — | [scenario.rs:1155](../../crates/crysirror-avg32/src/vm/scenario.rs#L1155) |
| `0x50` | Execute graphics load branch | Handled | — | [scenario.rs:1160](../../crates/crysirror-avg32/src/vm/scenario.rs#L1160) |
| `0x52` | Start effect | Handled | — | [scenario.rs:1161](../../crates/crysirror-avg32/src/vm/scenario.rs#L1161) |
| `0x54` | Text | Handled | — | [scenario.rs:1091](../../crates/crysirror-avg32/src/vm/scenario.rs#L1091) |

## Subcommands / mouse_control / self.byte()

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x01` | Control flow (mouse_control) | Handled | — | [scenario.rs:2567](../../crates/crysirror-avg32/src/vm/scenario.rs#L2567) |
| `0x02` | Control flow (mouse_control) | Handled | — | [scenario.rs:2580](../../crates/crysirror-avg32/src/vm/scenario.rs#L2580) |
| `0x03` | Control flow (mouse_control) | Handled | — | [scenario.rs:2587](../../crates/crysirror-avg32/src/vm/scenario.rs#L2587) |
| `0x20` | Control flow (mouse_control) | Handled | — | [scenario.rs:2588](../../crates/crysirror-avg32/src/vm/scenario.rs#L2588) |
| `0x21` | Control flow (mouse_control) | Handled | — | [scenario.rs:2589](../../crates/crysirror-avg32/src/vm/scenario.rs#L2589) |

## Subcommands / multi_add / self.byte()

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x01` | Peek | Handled | — | [scenario.rs:1815](../../crates/crysirror-avg32/src/vm/scenario.rs#L1815) |
| `0x10` | Execute multi add branch | Handled | — | [scenario.rs:1854](../../crates/crysirror-avg32/src/vm/scenario.rs#L1854) |
| `0x20` | Execute multi add branch | Handled | — | [scenario.rs:1865](../../crates/crysirror-avg32/src/vm/scenario.rs#L1865) |

## Subcommands / name_control / self.byte()

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x01` | Control flow (name_control) | Handled | — | [scenario.rs:1977](../../crates/crysirror-avg32/src/vm/scenario.rs#L1977) |
| `0x02` | Control flow (name_control) | Handled | — | [scenario.rs:1991](../../crates/crysirror-avg32/src/vm/scenario.rs#L1991) |
| `0x03` | Control flow (name_control) | Handled | — | [scenario.rs:2000](../../crates/crysirror-avg32/src/vm/scenario.rs#L2000) |
| `0x04` | Control flow (name_control) | Handled | — | [scenario.rs:2010](../../crates/crysirror-avg32/src/vm/scenario.rs#L2010) |
| `0x10` | Control flow (name_control) | Handled | — | [scenario.rs:2011](../../crates/crysirror-avg32/src/vm/scenario.rs#L2011) |
| `0x11` | Control flow (name_control) | Handled | — | [scenario.rs:2017](../../crates/crysirror-avg32/src/vm/scenario.rs#L2017) |
| `0x12` | Control flow (name_control) | Handled | — | [scenario.rs:2011](../../crates/crysirror-avg32/src/vm/scenario.rs#L2011) |
| `0x20` | Control flow (name_control) | Handled | — | [scenario.rs:2023](../../crates/crysirror-avg32/src/vm/scenario.rs#L2023) |
| `0x21` | Control flow (name_control) | Handled | — | [scenario.rs:2029](../../crates/crysirror-avg32/src/vm/scenario.rs#L2029) |
| `0x24` | Control flow (name_control) | Handled | — | [scenario.rs:2036](../../crates/crysirror-avg32/src/vm/scenario.rs#L2036) |
| `0x30` | Control flow (name_control) | Empty handler | Original no-op status not established | [scenario.rs:2052](../../crates/crysirror-avg32/src/vm/scenario.rs#L2052) |
| `0x31` | Control flow (name_control) | Empty handler | Original no-op status not established | [scenario.rs:2052](../../crates/crysirror-avg32/src/vm/scenario.rs#L2052) |

## Subcommands / sound / subcommand

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x01` | Audio playback/control (sound) | Handled | — | [scenario.rs:1277](../../crates/crysirror-avg32/src/vm/scenario.rs#L1277) |
| `0x02` | Audio playback/control (sound) | Handled | — | [scenario.rs:1277](../../crates/crysirror-avg32/src/vm/scenario.rs#L1277) |
| `0x03` | Audio playback/control (sound) | Handled | — | [scenario.rs:1277](../../crates/crysirror-avg32/src/vm/scenario.rs#L1277) |
| `0x05` | Audio playback/control (sound) | Handled | — | [scenario.rs:1285](../../crates/crysirror-avg32/src/vm/scenario.rs#L1285) |
| `0x06` | Audio playback/control (sound) | Handled | — | [scenario.rs:1285](../../crates/crysirror-avg32/src/vm/scenario.rs#L1285) |
| `0x07` | Audio playback/control (sound) | Handled | — | [scenario.rs:1285](../../crates/crysirror-avg32/src/vm/scenario.rs#L1285) |
| `0x10` | Audio playback/control (sound) | Handled | — | [scenario.rs:1294](../../crates/crysirror-avg32/src/vm/scenario.rs#L1294) |
| `0x11` | Audio playback/control (sound) | Handled | — | [scenario.rs:1298](../../crates/crysirror-avg32/src/vm/scenario.rs#L1298) |
| `0x12` | Audio playback/control (sound) | Handled | — | [scenario.rs:1298](../../crates/crysirror-avg32/src/vm/scenario.rs#L1298) |
| `0x16` | Audio playback/control (sound) | Handled | — | [scenario.rs:1298](../../crates/crysirror-avg32/src/vm/scenario.rs#L1298) |
| `0x20` | Audio playback/control (sound) | Handled | — | [scenario.rs:1299](../../crates/crysirror-avg32/src/vm/scenario.rs#L1299) |
| `0x21` | Audio playback/control (sound) | Handled | — | [scenario.rs:1299](../../crates/crysirror-avg32/src/vm/scenario.rs#L1299) |
| `0x22` | Audio playback/control (sound) | Handled | — | [scenario.rs:1299](../../crates/crysirror-avg32/src/vm/scenario.rs#L1299) |
| `0x30` | Audio playback/control (sound) | Handled | — | [scenario.rs:1324](../../crates/crysirror-avg32/src/vm/scenario.rs#L1324) |
| `0x31` | Audio playback/control (sound) | Handled | — | [scenario.rs:1324](../../crates/crysirror-avg32/src/vm/scenario.rs#L1324) |
| `0x32` | Audio playback/control (sound) | Handled | — | [scenario.rs:1324](../../crates/crysirror-avg32/src/vm/scenario.rs#L1324) |
| `0x33` | Audio playback/control (sound) | Handled | — | [scenario.rs:1324](../../crates/crysirror-avg32/src/vm/scenario.rs#L1324) |
| `0x34` | Audio playback/control (sound) | Handled | — | [scenario.rs:1324](../../crates/crysirror-avg32/src/vm/scenario.rs#L1324) |
| `0x35` | Audio playback/control (sound) | Handled | — | [scenario.rs:1324](../../crates/crysirror-avg32/src/vm/scenario.rs#L1324) |
| `0x36` | Audio playback/control (sound) | Handled | — | [scenario.rs:1333](../../crates/crysirror-avg32/src/vm/scenario.rs#L1333) |
| `0x37` | Audio playback/control (sound) | Handled | — | [scenario.rs:1334](../../crates/crysirror-avg32/src/vm/scenario.rs#L1334) |
| `0x38` | Audio playback/control (sound) | Handled | — | [scenario.rs:1333](../../crates/crysirror-avg32/src/vm/scenario.rs#L1333) |
| `0x39` | Audio playback/control (sound) | Handled | — | [scenario.rs:1334](../../crates/crysirror-avg32/src/vm/scenario.rs#L1334) |
| `0x40` | Audio playback/control (sound) | Handled | — | [scenario.rs:1338](../../crates/crysirror-avg32/src/vm/scenario.rs#L1338) |
| `0x44` | Audio playback/control (sound) | Handled | — | [scenario.rs:1338](../../crates/crysirror-avg32/src/vm/scenario.rs#L1338) |
| `0x50` | Audio playback/control (sound) | Handled | — | [scenario.rs:1342](../../crates/crysirror-avg32/src/vm/scenario.rs#L1342) |
| `0x51` | Audio playback/control (sound) | Handled | — | [scenario.rs:1342](../../crates/crysirror-avg32/src/vm/scenario.rs#L1342) |
| `0x52` | Audio playback/control (sound) | Handled | — | [scenario.rs:1342](../../crates/crysirror-avg32/src/vm/scenario.rs#L1342) |
| `0x53` | Audio playback/control (sound) | Handled | — | [scenario.rs:1342](../../crates/crysirror-avg32/src/vm/scenario.rs#L1342) |
| `0x54` | Audio playback/control (sound) | Handled | — | [scenario.rs:1342](../../crates/crysirror-avg32/src/vm/scenario.rs#L1342) |
| `0x55` | Audio playback/control (sound) | Handled | — | [scenario.rs:1342](../../crates/crysirror-avg32/src/vm/scenario.rs#L1342) |
| `0x60` | Audio playback/control (sound) | Empty handler | Original no-op status not established | [scenario.rs:1354](../../crates/crysirror-avg32/src/vm/scenario.rs#L1354) |

## Subcommands / strings / subcommand

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x01` | String operations (strings) | Handled | — | [scenario.rs:1750](../../crates/crysirror-avg32/src/vm/scenario.rs#L1750) |
| `0x02` | String operations (strings) | Handled | — | [scenario.rs:1754](../../crates/crysirror-avg32/src/vm/scenario.rs#L1754) |
| `0x03` | String operations (strings) | Handled | — | [scenario.rs:1759](../../crates/crysirror-avg32/src/vm/scenario.rs#L1759) |
| `0x04` | String operations (strings) | Handled | — | [scenario.rs:1765](../../crates/crysirror-avg32/src/vm/scenario.rs#L1765) |
| `0x05` | String operations (strings) | Handled | — | [scenario.rs:1771](../../crates/crysirror-avg32/src/vm/scenario.rs#L1771) |
| `0x06` | String operations (strings) | Handled | — | [scenario.rs:1776](../../crates/crysirror-avg32/src/vm/scenario.rs#L1776) |
| `0x07` | String operations (strings) | Handled | — | [scenario.rs:1789](../../crates/crysirror-avg32/src/vm/scenario.rs#L1789) |
| `0x08` | String operations (strings) | Handled | — | [scenario.rs:1793](../../crates/crysirror-avg32/src/vm/scenario.rs#L1793) |

## Subcommands / system_command / self.byte()

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x02` | System commands (system_command) | Handled | — | [scenario.rs:1890](../../crates/crysirror-avg32/src/vm/scenario.rs#L1890) |
| `0x03` | System commands (system_command) | Handled | — | [scenario.rs:1900](../../crates/crysirror-avg32/src/vm/scenario.rs#L1900) |
| `0x04` | System commands (system_command) | Handled | — | [scenario.rs:1910](../../crates/crysirror-avg32/src/vm/scenario.rs#L1910) |
| `0x05` | System commands (system_command) | Handled | — | [scenario.rs:1914](../../crates/crysirror-avg32/src/vm/scenario.rs#L1914) |
| `0x20` | System commands (system_command) | Handled | — | [scenario.rs:1915](../../crates/crysirror-avg32/src/vm/scenario.rs#L1915) |
| `0x30` | System commands (system_command) | Handled | — | [scenario.rs:1916](../../crates/crysirror-avg32/src/vm/scenario.rs#L1916) |
| `0x31` | System commands (system_command) | Handled | — | [scenario.rs:1942](../../crates/crysirror-avg32/src/vm/scenario.rs#L1942) |
| `0x35` | System commands (system_command) | Handled | — | [scenario.rs:1916](../../crates/crysirror-avg32/src/vm/scenario.rs#L1916) |
| `0x36` | System commands (system_command) | Handled | — | [scenario.rs:1942](../../crates/crysirror-avg32/src/vm/scenario.rs#L1942) |
| `0x37` | System commands (system_command) | Handled | — | [scenario.rs:1942](../../crates/crysirror-avg32/src/vm/scenario.rs#L1942) |

## Subcommands / system_values / subcommand

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x01` | Variables and values (system_values) | Handled | — | [scenario.rs:2774](../../crates/crysirror-avg32/src/vm/scenario.rs#L2774) |
| `0x02` | Variables and values (system_values) | Handled | — | [scenario.rs:2785](../../crates/crysirror-avg32/src/vm/scenario.rs#L2785) |
| `0x05` | Variables and values (system_values) | Handled | — | [scenario.rs:2774](../../crates/crysirror-avg32/src/vm/scenario.rs#L2774) |
| `0x06` | Variables and values (system_values) | Handled | — | [scenario.rs:2789](../../crates/crysirror-avg32/src/vm/scenario.rs#L2789) |
| `0x10` | Variables and values (system_values) | Handled | — | [scenario.rs:2813](../../crates/crysirror-avg32/src/vm/scenario.rs#L2813) |
| `0x11` | Variables and values (system_values) | Handled | — | [scenario.rs:2813](../../crates/crysirror-avg32/src/vm/scenario.rs#L2813) |
| `0x12` | Variables and values (system_values) | Handled | — | [scenario.rs:2814](../../crates/crysirror-avg32/src/vm/scenario.rs#L2814) |
| `0x13` | Variables and values (system_values) | Handled | — | [scenario.rs:2814](../../crates/crysirror-avg32/src/vm/scenario.rs#L2814) |
| `0x14` | Variables and values (system_values) | Handled | — | [scenario.rs:2810](../../crates/crysirror-avg32/src/vm/scenario.rs#L2810) |
| `0x15` | Variables and values (system_values) | Handled | — | [scenario.rs:2810](../../crates/crysirror-avg32/src/vm/scenario.rs#L2810) |
| `0x16` | Variables and values (system_values) | Handled | — | [scenario.rs:2815](../../crates/crysirror-avg32/src/vm/scenario.rs#L2815) |
| `0x17` | Variables and values (system_values) | Handled | — | [scenario.rs:2815](../../crates/crysirror-avg32/src/vm/scenario.rs#L2815) |
| `0x18` | Variables and values (system_values) | Handled | — | [scenario.rs:2816](../../crates/crysirror-avg32/src/vm/scenario.rs#L2816) |
| `0x19` | Variables and values (system_values) | Handled | — | [scenario.rs:2816](../../crates/crysirror-avg32/src/vm/scenario.rs#L2816) |
| `0x1A` | Variables and values (system_values) | Handled | — | [scenario.rs:2817](../../crates/crysirror-avg32/src/vm/scenario.rs#L2817) |
| `0x1B` | Variables and values (system_values) | Handled | — | [scenario.rs:2817](../../crates/crysirror-avg32/src/vm/scenario.rs#L2817) |
| `0x1C` | Variables and values (system_values) | Handled | — | [scenario.rs:2818](../../crates/crysirror-avg32/src/vm/scenario.rs#L2818) |
| `0x1D` | Variables and values (system_values) | Handled | — | [scenario.rs:2818](../../crates/crysirror-avg32/src/vm/scenario.rs#L2818) |
| `0x1E` | Variables and values (system_values) | Handled | — | [scenario.rs:2819](../../crates/crysirror-avg32/src/vm/scenario.rs#L2819) |
| `0x1F` | Variables and values (system_values) | Handled | — | [scenario.rs:2819](../../crates/crysirror-avg32/src/vm/scenario.rs#L2819) |
| `0x20` | Variables and values (system_values) | Handled | — | [scenario.rs:2820](../../crates/crysirror-avg32/src/vm/scenario.rs#L2820) |
| `0x21` | Variables and values (system_values) | Handled | — | [scenario.rs:2820](../../crates/crysirror-avg32/src/vm/scenario.rs#L2820) |
| `0x22` | Variables and values (system_values) | Handled | — | [scenario.rs:2821](../../crates/crysirror-avg32/src/vm/scenario.rs#L2821) |
| `0x23` | Variables and values (system_values) | Handled | — | [scenario.rs:2821](../../crates/crysirror-avg32/src/vm/scenario.rs#L2821) |
| `0x24` | Variables and values (system_values) | Handled | — | [scenario.rs:2822](../../crates/crysirror-avg32/src/vm/scenario.rs#L2822) |
| `0x25` | Variables and values (system_values) | Handled | — | [scenario.rs:2822](../../crates/crysirror-avg32/src/vm/scenario.rs#L2822) |
| `0x26` | Variables and values (system_values) | Handled | — | [scenario.rs:2823](../../crates/crysirror-avg32/src/vm/scenario.rs#L2823) |
| `0x27` | Variables and values (system_values) | Handled | — | [scenario.rs:2823](../../crates/crysirror-avg32/src/vm/scenario.rs#L2823) |
| `0x28` | Variables and values (system_values) | Handled | — | [scenario.rs:2793](../../crates/crysirror-avg32/src/vm/scenario.rs#L2793) |
| `0x29` | Variables and values (system_values) | Handled | — | [scenario.rs:2799](../../crates/crysirror-avg32/src/vm/scenario.rs#L2799) |
| `0x2A` | Variables and values (system_values) | Handled | — | [scenario.rs:2793](../../crates/crysirror-avg32/src/vm/scenario.rs#L2793) |
| `0x2B` | Variables and values (system_values) | Handled | — | [scenario.rs:2799](../../crates/crysirror-avg32/src/vm/scenario.rs#L2799) |
| `0x2C` | Variables and values (system_values) | Handled | — | [scenario.rs:2824](../../crates/crysirror-avg32/src/vm/scenario.rs#L2824) |
| `0x2D` | Variables and values (system_values) | Handled | — | [scenario.rs:2824](../../crates/crysirror-avg32/src/vm/scenario.rs#L2824) |
| `0x2E` | Variables and values (system_values) | Handled | — | [scenario.rs:2825](../../crates/crysirror-avg32/src/vm/scenario.rs#L2825) |
| `0x2F` | Variables and values (system_values) | Handled | — | [scenario.rs:2825](../../crates/crysirror-avg32/src/vm/scenario.rs#L2825) |
| `0x30` | Variables and values (system_values) | Handled | — | [scenario.rs:2810](../../crates/crysirror-avg32/src/vm/scenario.rs#L2810) |
| `0x31` | Variables and values (system_values) | Handled | — | [scenario.rs:2803](../../crates/crysirror-avg32/src/vm/scenario.rs#L2803) |
| `0x32` | Variables and values (system_values) | Handled | — | [scenario.rs:2807](../../crates/crysirror-avg32/src/vm/scenario.rs#L2807) |
| `0x33` | Variables and values (system_values) | Handled | — | [scenario.rs:2810](../../crates/crysirror-avg32/src/vm/scenario.rs#L2810) |
| `0x34` | Variables and values (system_values) | Handled | — | [scenario.rs:2807](../../crates/crysirror-avg32/src/vm/scenario.rs#L2807) |

## Subcommands / text_window / subcommand

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x01` | Text and message windows (text_window) | Handled | — | [scenario.rs:1002](../../crates/crysirror-avg32/src/vm/scenario.rs#L1002) |
| `0x02` | Text and message windows (text_window) | Handled | — | [scenario.rs:1002](../../crates/crysirror-avg32/src/vm/scenario.rs#L1002) |
| `0x03` | Text and message windows (text_window) | Handled | — | [scenario.rs:1008](../../crates/crysirror-avg32/src/vm/scenario.rs#L1008) |
| `0x04` | Text and message windows (text_window) | Handled | — | [scenario.rs:1013](../../crates/crysirror-avg32/src/vm/scenario.rs#L1013) |
| `0x05` | Text and message windows (text_window) | Handled | — | [scenario.rs:1008](../../crates/crysirror-avg32/src/vm/scenario.rs#L1008) |

## Subcommands / value_text / self.byte()

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x01` | Text and message windows (value_text) | Handled | — | [scenario.rs:1411](../../crates/crysirror-avg32/src/vm/scenario.rs#L1411) |
| `0x02` | Text and message windows (value_text) | Handled | — | [scenario.rs:1415](../../crates/crysirror-avg32/src/vm/scenario.rs#L1415) |
| `0x03` | Text and message windows (value_text) | Handled | — | [scenario.rs:1420](../../crates/crysirror-avg32/src/vm/scenario.rs#L1420) |

## Subcommands / window_config / self.byte()

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x01` | Execute window config branch | Handled | — | [scenario.rs:2670](../../crates/crysirror-avg32/src/vm/scenario.rs#L2670) |
| `0x02` | Colour | Handled | — | [scenario.rs:2688](../../crates/crysirror-avg32/src/vm/scenario.rs#L2688) |
| `0x03` | Execute window config branch | Handled | — | [scenario.rs:2693](../../crates/crysirror-avg32/src/vm/scenario.rs#L2693) |
| `0x04` | Execute window config branch | Handled | — | [scenario.rs:2703](../../crates/crysirror-avg32/src/vm/scenario.rs#L2703) |
| `0x05` | Execute window config branch | Handled | — | [scenario.rs:2693](../../crates/crysirror-avg32/src/vm/scenario.rs#L2693) |
| `0x06` | Execute window config branch | Handled | — | [scenario.rs:2703](../../crates/crysirror-avg32/src/vm/scenario.rs#L2703) |
| `0x10` | Execute window config branch | Handled | — | [scenario.rs:2712](../../crates/crysirror-avg32/src/vm/scenario.rs#L2712) |
| `0x11` | Execute window config branch | Handled | — | [scenario.rs:2721](../../crates/crysirror-avg32/src/vm/scenario.rs#L2721) |

## Subcommands / window_position / subcommand

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x01` | Execute window position branch | Handled | — | [scenario.rs:2739](../../crates/crysirror-avg32/src/vm/scenario.rs#L2739) |
| `0x02` | Execute window position branch | Handled | — | [scenario.rs:2740](../../crates/crysirror-avg32/src/vm/scenario.rs#L2740) |
| `0x03` | Execute window position branch | Handled | — | [scenario.rs:2741](../../crates/crysirror-avg32/src/vm/scenario.rs#L2741) |
| `0x04` | Execute window position branch | Handled | — | [scenario.rs:2742](../../crates/crysirror-avg32/src/vm/scenario.rs#L2742) |
| `0x05` | Execute window position branch | Handled | — | [scenario.rs:2735](../../crates/crysirror-avg32/src/vm/scenario.rs#L2735) |
| `0x11` | Execute window position branch | Handled | — | [scenario.rs:2751](../../crates/crysirror-avg32/src/vm/scenario.rs#L2751) |
| `0x12` | Execute window position branch | Handled | — | [scenario.rs:2752](../../crates/crysirror-avg32/src/vm/scenario.rs#L2752) |
| `0x13` | Execute window position branch | Handled | — | [scenario.rs:2756](../../crates/crysirror-avg32/src/vm/scenario.rs#L2756) |
| `0x14` | Execute window position branch | Handled | — | [scenario.rs:2757](../../crates/crysirror-avg32/src/vm/scenario.rs#L2757) |
| `0x15` | Execute window position branch | Handled | — | [scenario.rs:2748](../../crates/crysirror-avg32/src/vm/scenario.rs#L2748) |

## Subselectors and conditional dispatch

Patterns retain their source namespace and guards. `_` is the fallback of that table. Boolean/optional results follow the function's return contract. Call-site names identify the implementation to inspect, including indirect service calls.

### vm/ini.rs / Ini::item

```rust
fn item(&mut self, text: &str, sel_number: &mut usize) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Some(kind) = text.strip_prefix("#DIRC.")` | String::new, self.directories.insert | Handled | [314](../../crates/crysirror-avg32/src/vm/ini.rs#L314) |

### vm/meswin.rs / System::mes_set

```rust
fn mes_set(&mut self, packet: &[u8]) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `kind` | `0xfe` | self.mes.buf.extend_from_slice | Handled | [525](../../crates/crysirror-avg32/src/vm/meswin.rs#L525) |
| `kind` | `0xff` | nls::char_at, self.mes.buf.extend_from_slice, self.mes.buf.push, self.name | Handled | [526](../../crates/crysirror-avg32/src/vm/meswin.rs#L526) |
| `kind` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [559](../../crates/crysirror-avg32/src/vm/meswin.rs#L559) |

### vm/nls.rs / Nls::from_str

```rust
fn from_str(text: &str) -> anyhow::Result<Self>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `key.as_str()` | `"sjis" \| "shiftjis" \| "cp932" \| "ms932" \| "jis" \| "ja"` | Local state/value operations | Handled | [108](../../crates/crysirror-avg32/src/vm/nls.rs#L108) |
| `key.as_str()` | `"gbk" \| "gb2312" \| "gb18030" \| "cp936" \| "chs" \| "zhcn"` | Local state/value operations | Handled | [109](../../crates/crysirror-avg32/src/vm/nls.rs#L109) |
| `key.as_str()` | `"big5" \| "cp950" \| "cht" \| "zhtw"` | Local state/value operations | Handled | [110](../../crates/crysirror-avg32/src/vm/nls.rs#L110) |
| `key.as_str()` | `"utf8" \| "utf" \| "unicode"` | Local state/value operations | Handled | [111](../../crates/crysirror-avg32/src/vm/nls.rs#L111) |
| `key.as_str()` | `_` | Local state/value operations | Handled | [112](../../crates/crysirror-avg32/src/vm/nls.rs#L112) |

### vm/resource.rs / with_extension

```rust
fn with_extension(name: &str, extension: &str) -> String
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `extension == "***" \|\| Path::new(name).extension().is_some()` | Local state/value operations | Handled | [138](../../crates/crysirror-avg32/src/vm/resource.rs#L138) |

### vm/scenario.rs / Scenario::animation

```rust
fn animation(&mut self, sys: &mut System) -> bool
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under subcommand=0x10 \| 0x16 \| 0x18` | `subcommand == 0x18` | Local state/value operations | Handled | [1187](../../crates/crysirror-avg32/src/vm/scenario.rs#L1187) |
| `condition; under subcommand=0x10 \| 0x16 \| 0x18` | `subcommand != 0x10` | self.value | Handled | [1192](../../crates/crysirror-avg32/src/vm/scenario.rs#L1192) |
| `condition; under subcommand=0x12 \| 0x30` | `subcommand == 0x30` | Local state/value operations | Handled | [1209](../../crates/crysirror-avg32/src/vm/scenario.rs#L1209) |
| `subcommand` | `other` | Local state/value operations | Handled | [1227](../../crates/crysirror-avg32/src/vm/scenario.rs#L1227) |

### vm/scenario.rs / Scenario::area_control

```rust
fn area_control(&mut self, sys: &mut System) -> bool
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under self.byte()=sub @ (0x04 \| 0x05)` | `sub == 0x04` | i32::from, self.area_find | Handled | [2507](../../crates/crysirror-avg32/src/vm/scenario.rs#L2507) |

### vm/scenario.rs / Scenario::choice

```rust
fn choice(&mut self, sys: &mut System) -> bool
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under self.select_flag=0x01 \| 0x02` | `self.peek() == 0x23 \|\| self.pos as usize >= self.code().len()` | Local state/value operations | Handled | [1720](../../crates/crysirror-avg32/src/vm/scenario.rs#L1720) |

### vm/scenario.rs / Scenario::conditions

```rust
fn conditions(&mut self, sys: &System) -> i32
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `kind; under c=ATTR` | `0x20 \| 0x22` | self.value | Handled | [398](../../crates/crysirror-avg32/src/vm/scenario.rs#L398) |
| `kind; under c=ATTR` | `_` | Local state/value operations | Handled | [399](../../crates/crysirror-avg32/src/vm/scenario.rs#L399) |

### vm/scenario.rs / Scenario::decode

```rust
fn decode(&mut self, sys: &mut System) -> bool
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under self.cmd=0x16` | `subcommand != 1` | Local state/value operations | Handled | [684](../../crates/crysirror-avg32/src/vm/scenario.rs#L684) |
| `condition; under self.cmd=0x1d \| 0x1e` | `self.cmd == 0x1d` | Local state/value operations | Handled | [743](../../crates/crysirror-avg32/src/vm/scenario.rs#L743) |
| `condition; under self.cmd=0x5b` | `subcommand == 1` | Local state/value operations | Handled | [800](../../crates/crysirror-avg32/src/vm/scenario.rs#L800) |
| `condition; under self.cmd=0x5b` | `subcommand == 2` | Local state/value operations | Handled | [802](../../crates/crysirror-avg32/src/vm/scenario.rs#L802) |
| `condition; under self.cmd=0x5c` | `subcommand == 1` | Local state/value operations | Handled | [817](../../crates/crysirror-avg32/src/vm/scenario.rs#L817) |
| `condition; under self.cmd=0x5c` | `subcommand == 2` | Local state/value operations | Handled | [819](../../crates/crysirror-avg32/src/vm/scenario.rs#L819) |
| `condition; under self.cmd=0x5d` | `subcommand == 1` | Local state/value operations | Handled | [832](../../crates/crysirror-avg32/src/vm/scenario.rs#L832) |
| `condition; under self.cmd=0x5d` | `subcommand == 2` | Local state/value operations | Handled | [835](../../crates/crysirror-avg32/src/vm/scenario.rs#L835) |
| `subcommand; under self.cmd=0x5e` | `1..=4` | i32::from | Handled | [847](../../crates/crysirror-avg32/src/vm/scenario.rs#L847) |
| `subcommand; under self.cmd=0x5e` | `0x10` | Local state/value operations | Handled | [848](../../crates/crysirror-avg32/src/vm/scenario.rs#L848) |
| `subcommand; under self.cmd=0x5e` | `_` | Local state/value operations | Handled | [849](../../crates/crysirror-avg32/src/vm/scenario.rs#L849) |
| `subcommand >> 4; under self.cmd=0x75` | `0` | Local state/value operations | Handled | [940](../../crates/crysirror-avg32/src/vm/scenario.rs#L940) |
| `subcommand >> 4; under self.cmd=0x75` | `1` | Local state/value operations | Handled | [941](../../crates/crysirror-avg32/src/vm/scenario.rs#L941) |
| `subcommand >> 4; under self.cmd=0x75` | `2` | Local state/value operations | Handled | [942](../../crates/crysirror-avg32/src/vm/scenario.rs#L942) |
| `subcommand >> 4; under self.cmd=0x75` | `_` | Local state/value operations | Handled | [943](../../crates/crysirror-avg32/src/vm/scenario.rs#L943) |
| `self.cmd` | `_` | self.unknown | Handled | [969](../../crates/crysirror-avg32/src/vm/scenario.rs#L969) |

### vm/scenario.rs / Scenario::ending

```rust
fn ending(&mut self, sys: &mut System) -> bool
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under mode=0x10 \| 0x20 \| 0x30` | `mode == 0x30` | self.value | Handled | [2327](../../crates/crysirror-avg32/src/vm/scenario.rs#L2327) |
| `mode` | `0x10 \| 0x20 \| 0x30` | self.byte, self.value, self.text, nls::decode, self.ending.offsets.push | Handled | [2321](../../crates/crysirror-avg32/src/vm/scenario.rs#L2321) |
| `mode` | `0x03 \| 0x04` | self.byte, self.value, self.text, self.ending.files.push, self.ending.offsets.push | Handled | [2352](../../crates/crysirror-avg32/src/vm/scenario.rs#L2352) |
| `mode` | `0x05` | Local state/value operations | Handled | [2370](../../crates/crysirror-avg32/src/vm/scenario.rs#L2370) |
| `mode` | `other` | Local state/value operations | Handled | [2374](../../crates/crysirror-avg32/src/vm/scenario.rs#L2374) |
| `condition; under self.ending.cmd=0x10 \| 0x20 \| 0x30` | `self.ending.cmd == 0x30 && (sys.mouse.button(now) \|\| sys.check_skip())` | Ending::default | Handled | [2385](../../crates/crysirror-avg32/src/vm/scenario.rs#L2385) |
| `self.ending.cmd` | `0x10 \| 0x20 \| 0x30` | Ending::default, self.ending.wait.max, self.ending.files.len, nls::decode, crysirror_image::avg32::buffer::PdtBuffer::from_image | Handled | [2384](../../crates/crysirror-avg32/src/vm/scenario.rs#L2384) |
| `self.ending.cmd` | `_` | self.ending.wait.max, self.ending.files.len, Ending::default, self.show_immediately | Handled | [2436](../../crates/crysirror-avg32/src/vm/scenario.rs#L2436) |

### vm/scenario.rs / Scenario::fade

```rust
fn fade(&mut self, sys: &mut System) -> bool
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under subcommand=0x01 \| 0x02` | `subcommand == 0x02` | self.value | Handled | [1467](../../crates/crysirror-avg32/src/vm/scenario.rs#L1467) |
| `condition; under subcommand=0x03 \| 0x04` | `subcommand == 0x04` | self.value | Handled | [1477](../../crates/crysirror-avg32/src/vm/scenario.rs#L1477) |
| `subcommand` | `other` | Local state/value operations | Handled | [1495](../../crates/crysirror-avg32/src/vm/scenario.rs#L1495) |

### vm/scenario.rs / Scenario::graphics

```rust
fn graphics(&mut self, sys: &mut System) -> bool
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `subcommand; under self.cmd=0x64 / subcommand=0x02 \| 0x04 \| 0x10 \| 0x15` | `_` | self.value | Handled | [2075](../../crates/crysirror-avg32/src/vm/scenario.rs#L2075) |
| `subcommand; under self.cmd=0x64 / subcommand=0x07 \| 0x11 \| 0x12 \| 0x20` | `_` | Local state/value operations | Handled | [2088](../../crates/crysirror-avg32/src/vm/scenario.rs#L2088) |
| `subcommand; under self.cmd=0x64` | `other` | Local state/value operations | Handled | [2127](../../crates/crysirror-avg32/src/vm/scenario.rs#L2127) |
| `subcommand; under self.cmd=0x67 / subcommand=0x01 \| 0x02 \| 0x08` | `_` | Local state/value operations | Handled | [2149](../../crates/crysirror-avg32/src/vm/scenario.rs#L2149) |
| `subcommand; under self.cmd=0x67 / subcommand=0x01 \| 0x02 \| 0x08` | `_` | Local state/value operations | Handled | [2155](../../crates/crysirror-avg32/src/vm/scenario.rs#L2155) |
| `condition; under self.cmd=0x67 / subcommand=0x11 \| 0x12` | `subcommand == 0x11` | Local state/value operations | Handled | [2178](../../crates/crysirror-avg32/src/vm/scenario.rs#L2178) |
| `condition; under self.cmd=0x67 / subcommand=0x11 \| 0x12` | `subcommand == 0x11` | Local state/value operations | Handled | [2184](../../crates/crysirror-avg32/src/vm/scenario.rs#L2184) |
| `subcommand; under self.cmd=0x67 / subcommand=0x20..=0x22` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [2205](../../crates/crysirror-avg32/src/vm/scenario.rs#L2205) |
| `(subcommand, extra); under self.cmd=0x67 / subcommand=0x20..=0x22` | `(0x21, Some([flag, _, _]))` | Local state/value operations | Handled | [2217](../../crates/crysirror-avg32/src/vm/scenario.rs#L2217) |
| `(subcommand, extra); under self.cmd=0x67 / subcommand=0x20..=0x22` | `(0x22, Some(key))` | Local state/value operations | Handled | [2220](../../crates/crysirror-avg32/src/vm/scenario.rs#L2220) |
| `(subcommand, extra); under self.cmd=0x67 / subcommand=0x20..=0x22` | `_` | Local state/value operations | Handled | [2223](../../crates/crysirror-avg32/src/vm/scenario.rs#L2223) |
| `subcommand; under self.cmd=0x67` | `other` | Local state/value operations | Handled | [2231](../../crates/crysirror-avg32/src/vm/scenario.rs#L2231) |
| `self.cmd` | `0x64` | self.byte, self.rect, self.value, self.colour, Effect::default | Partial: Unknown subselectors warn and continue | [2064](../../crates/crysirror-avg32/src/vm/scenario.rs#L2064) |
| `self.cmd` | `0x67` | self.byte, self.rect, self.value, self.colour, Rect::full | Partial: Unknown subselectors warn and continue | [2132](../../crates/crysirror-avg32/src/vm/scenario.rs#L2132) |
| `self.cmd` | `0x68` | Rect::full, self.byte, self.value, self.colour | Partial: Unknown subselectors warn and continue | [2236](../../crates/crysirror-avg32/src/vm/scenario.rs#L2236) |
| `self.cmd` | `0x69` | self.byte, i32::from, self.rect, self.value, Effect::default | Handled | [2283](../../crates/crysirror-avg32/src/vm/scenario.rs#L2283) |
| `self.cmd` | `_` | self.ending | Handled | [2308](../../crates/crysirror-avg32/src/vm/scenario.rs#L2308) |

### vm/scenario.rs / Scenario::graphics_load

```rust
fn graphics_load(&mut self, sys: &mut System) -> bool
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under subcommand=0x22 \| 0x24` | `subcommand == 0x22` | self.text | Handled | [1102](../../crates/crysirror-avg32/src/vm/scenario.rs#L1102) |
| `condition; under subcommand=0x22 \| 0x24 / method=0x03 \| 0x04` | `method == 0x04` | self.value | Handled | [1136](../../crates/crysirror-avg32/src/vm/scenario.rs#L1136) |
| `method; under subcommand=0x22 \| 0x24` | `0x01` | Rect::full | Handled | [1120](../../crates/crysirror-avg32/src/vm/scenario.rs#L1120) |
| `method; under subcommand=0x22 \| 0x24` | `0x02` | self.value, Rect::new | Handled | [1121](../../crates/crysirror-avg32/src/vm/scenario.rs#L1121) |
| `method; under subcommand=0x22 \| 0x24` | `0x03 \| 0x04` | self.rect, self.value | Handled | [1133](../../crates/crysirror-avg32/src/vm/scenario.rs#L1133) |
| `method; under subcommand=0x22 \| 0x24` | `other` | Local state/value operations | Handled | [1143](../../crates/crysirror-avg32/src/vm/scenario.rs#L1143) |
| `subcommand` | `other` | Local state/value operations | Handled | [1168](../../crates/crysirror-avg32/src/vm/scenario.rs#L1168) |

### vm/scenario.rs / Scenario::legacy

```rust
fn legacy(&mut self, sys: &mut System) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `(self.cmd, subcommand)` | `(0x2c, 1) \| (0x2d, 1) \| (0x2d, 2) \| (0x31, 1)` | self.value | Handled | [1585](../../crates/crysirror-avg32/src/vm/scenario.rs#L1585) |
| `(self.cmd, subcommand)` | `(0x2d, 3) \| (0x2d, 4)` | self.value | Handled | [1588](../../crates/crysirror-avg32/src/vm/scenario.rs#L1588) |
| `(self.cmd, subcommand)` | `(0x2e, 1)` | self.value | Handled | [1592](../../crates/crysirror-avg32/src/vm/scenario.rs#L1592) |
| `(self.cmd, subcommand)` | `(0x2e, 2)` | self.value | Handled | [1597](../../crates/crysirror-avg32/src/vm/scenario.rs#L1597) |
| `(self.cmd, subcommand)` | `(0x2f, 1)` | self.value | Handled | [1603](../../crates/crysirror-avg32/src/vm/scenario.rs#L1603) |
| `(self.cmd, subcommand)` | `(0x30, 1) \| (0x31, 2)` | Local state/value operations | Empty handler: Original no-op status not established | [1608](../../crates/crysirror-avg32/src/vm/scenario.rs#L1608) |
| `(self.cmd, subcommand)` | `(command, other)` | Local state/value operations | Handled | [1609](../../crates/crysirror-avg32/src/vm/scenario.rs#L1609) |

### vm/scenario.rs / Scenario::multi_add

```rust
fn multi_add(&mut self, sys: &mut System) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `code; under self.byte()=0x01` | `0x01` | self.value | Handled | [1821](../../crates/crysirror-avg32/src/vm/scenario.rs#L1821) |
| `code; under self.byte()=0x01` | `0x02` | self.value | Handled | [1825](../../crates/crysirror-avg32/src/vm/scenario.rs#L1825) |
| `code; under self.byte()=0x01` | `0x11` | self.value, i32::from | Handled | [1832](../../crates/crysirror-avg32/src/vm/scenario.rs#L1832) |
| `code; under self.byte()=0x01` | `0x12` | self.value, i32::from | Handled | [1836](../../crates/crysirror-avg32/src/vm/scenario.rs#L1836) |
| `code; under self.byte()=0x01` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [1843](../../crates/crysirror-avg32/src/vm/scenario.rs#L1843) |
| `condition; under self.byte()=0x01` | `code == 0 \|\| self.peek() == 0` | Local state/value operations | Handled | [1845](../../crates/crysirror-avg32/src/vm/scenario.rs#L1845) |
| `condition; under self.byte()=0x01` | `code != 0` | Local state/value operations | Handled | [1846](../../crates/crysirror-avg32/src/vm/scenario.rs#L1846) |

### vm/scenario.rs / Scenario::run

```rust
fn run(&mut self, sys: &mut System) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `self.multi_anm != 0 && self.cmd != 0x13 && (self.multi_anm == 1 \|\| sys.sound.koe_playing())` | Local state/value operations | Handled | [582](../../crates/crysirror-avg32/src/vm/scenario.rs#L582) |
| `condition` | `self.cmd == 0` | self.code, self.byte | Handled | [588](../../crates/crysirror-avg32/src/vm/scenario.rs#L588) |
| `condition` | `self.pos < 0 \|\| self.pos as usize >= self.code().len()` | Local state/value operations | Handled | [589](../../crates/crysirror-avg32/src/vm/scenario.rs#L589) |

### vm/scenario.rs / Scenario::scenario_menu

```rust
fn scenario_menu(&mut self, sys: &mut System) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `menu.id & self.menu.bit != 0` | Local state/value operations | Handled | [2994](../../crates/crysirror-avg32/src/vm/scenario.rs#L2994) |
| `condition` | `submenu.id & self.menu.sub_bit != 0` | Local state/value operations | Handled | [3006](../../crates/crysirror-avg32/src/vm/scenario.rs#L3006) |

### vm/scenario.rs / Scenario::sound

```rust
fn sound(&mut self, sys: &mut System) -> bool
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under subcommand=0x01..=0x03` | `subcommand == 0x02` | Local state/value operations | Handled | [1280](../../crates/crysirror-avg32/src/vm/scenario.rs#L1280) |
| `condition; under subcommand=0x05..=0x07` | `subcommand == 0x06` | Local state/value operations | Handled | [1289](../../crates/crysirror-avg32/src/vm/scenario.rs#L1289) |
| `condition; under subcommand=0x20..=0x22` | `subcommand == 0x22` | self.value | Handled | [1301](../../crates/crysirror-avg32/src/vm/scenario.rs#L1301) |
| `condition; under subcommand=0x20..=0x22` | `subcommand == 0x20` | Local state/value operations | Handled | [1319](../../crates/crysirror-avg32/src/vm/scenario.rs#L1319) |
| `condition; under subcommand=0x30..=0x35` | `matches!(subcommand, 0x34 \| 0x35)` | Local state/value operations | Handled | [1328](../../crates/crysirror-avg32/src/vm/scenario.rs#L1328) |
| `condition; under subcommand=0x50..=0x55` | `subcommand >= 0x54` | self.text | Handled | [1344](../../crates/crysirror-avg32/src/vm/scenario.rs#L1344) |
| `condition; under subcommand=0x50..=0x55` | `subcommand >= 0x52 && self.movie.is_some()` | Local state/value operations | Handled | [1349](../../crates/crysirror-avg32/src/vm/scenario.rs#L1349) |
| `subcommand` | `other` | Local state/value operations | Handled | [1355](../../crates/crysirror-avg32/src/vm/scenario.rs#L1355) |

### vm/scenario.rs / Scenario::strings

```rust
fn strings(&mut self, sys: &mut System) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `subcommand` | `other` | Local state/value operations | Handled | [1808](../../crates/crysirror-avg32/src/vm/scenario.rs#L1808) |

### vm/scenario.rs / Scenario::system_command

```rust
fn system_command(&mut self, sys: &mut System) -> bool
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under self.byte()=sub @ (0x30 \| 0x35)` | `sub == 0x30` | Local state/value operations | Handled | [1925](../../crates/crysirror-avg32/src/vm/scenario.rs#L1925) |
| `sub; under self.byte()=sub @ (0x31 \| 0x36 \| 0x37)` | `0x31` | i32::from | Handled | [1951](../../crates/crysirror-avg32/src/vm/scenario.rs#L1951) |
| `sub; under self.byte()=sub @ (0x31 \| 0x36 \| 0x37)` | `0x36` | Local state/value operations | Handled | [1952](../../crates/crysirror-avg32/src/vm/scenario.rs#L1952) |
| `sub; under self.byte()=sub @ (0x31 \| 0x36 \| 0x37)` | `_` | Local state/value operations | Handled | [1953](../../crates/crysirror-avg32/src/vm/scenario.rs#L1953) |

### vm/scenario.rs / Scenario::system_values

```rust
fn system_values(&mut self, sys: &mut System) -> bool
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under subcommand=0x01 \| 0x05` | `subcommand == 0x01` | Local state/value operations | Handled | [2777](../../crates/crysirror-avg32/src/vm/scenario.rs#L2777) |
| `subcommand; under subcommand=0x10..=0x27 \| 0x2c..=0x30 \| 0x33` | `_` | Local state/value operations | Handled | [2826](../../crates/crysirror-avg32/src/vm/scenario.rs#L2826) |
| `key; under subcommand=0x10..=0x27 \| 0x2c..=0x30 \| 0x33` | `"MOJI_COLOR"` | Local state/value operations | Handled | [2831](../../crates/crysirror-avg32/src/vm/scenario.rs#L2831) |
| `key; under subcommand=0x10..=0x27 \| 0x2c..=0x30 \| 0x33` | `"FADE_TIME"` | Local state/value operations | Handled | [2832](../../crates/crysirror-avg32/src/vm/scenario.rs#L2832) |
| `key; under subcommand=0x10..=0x27 \| 0x2c..=0x30 \| 0x33` | `"CTRL_KEY"` | i32::from | Handled | [2833](../../crates/crysirror-avg32/src/vm/scenario.rs#L2833) |
| `key; under subcommand=0x10..=0x27 \| 0x2c..=0x30 \| 0x33` | `_` | Local state/value operations | Handled | [2834](../../crates/crysirror-avg32/src/vm/scenario.rs#L2834) |
| `key; under subcommand=0x10..=0x27 \| 0x2c..=0x30 \| 0x33` | `"MOJI_COLOR"` | Local state/value operations | Handled | [2840](../../crates/crysirror-avg32/src/vm/scenario.rs#L2840) |
| `key; under subcommand=0x10..=0x27 \| 0x2c..=0x30 \| 0x33` | `"FADE_TIME"` | Local state/value operations | Handled | [2841](../../crates/crysirror-avg32/src/vm/scenario.rs#L2841) |
| `key; under subcommand=0x10..=0x27 \| 0x2c..=0x30 \| 0x33` | `"CTRL_KEY"` | Local state/value operations | Handled | [2842](../../crates/crysirror-avg32/src/vm/scenario.rs#L2842) |
| `key; under subcommand=0x10..=0x27 \| 0x2c..=0x30 \| 0x33` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [2843](../../crates/crysirror-avg32/src/vm/scenario.rs#L2843) |
| `subcommand` | `other` | Local state/value operations | Handled | [2848](../../crates/crysirror-avg32/src/vm/scenario.rs#L2848) |

### vm/scenario.rs / Scenario::text_window

```rust
fn text_window(&mut self, sys: &mut System) -> bool
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `subcommand` | `other` | Local state/value operations | Handled | [1031](../../crates/crysirror-avg32/src/vm/scenario.rs#L1031) |

### vm/scenario.rs / Scenario::unknown

```rust
fn unknown(&mut self, sys: &mut System) -> bool
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `self.cmd == 0` | self.scenario_menu, self.code | Handled | [2892](../../crates/crysirror-avg32/src/vm/scenario.rs#L2892) |
| `condition` | `self.pos as usize >= self.code().len().saturating_sub(1)` | Local state/value operations | Handled | [2902](../../crates/crysirror-avg32/src/vm/scenario.rs#L2902) |

### vm/scenario.rs / Scenario::variable

```rust
fn variable(&mut self, sys: &mut System) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `self.cmd == 0x56` | Local state/value operations | Handled | [1616](../../crates/crysirror-avg32/src/vm/scenario.rs#L1616) |
| `self.cmd` | `0x37` | Local state/value operations | Handled | [1626](../../crates/crysirror-avg32/src/vm/scenario.rs#L1626) |
| `self.cmd` | `0x39` | Local state/value operations | Handled | [1630](../../crates/crysirror-avg32/src/vm/scenario.rs#L1630) |
| `self.cmd` | `0x3b` | Local state/value operations | Handled | [1635](../../crates/crysirror-avg32/src/vm/scenario.rs#L1635) |
| `self.cmd` | `0x3c` | Local state/value operations | Handled | [1636](../../crates/crysirror-avg32/src/vm/scenario.rs#L1636) |
| `self.cmd` | `0x3d` | Local state/value operations | Handled | [1637](../../crates/crysirror-avg32/src/vm/scenario.rs#L1637) |
| `self.cmd` | `0x3e` | Local state/value operations | Handled | [1638](../../crates/crysirror-avg32/src/vm/scenario.rs#L1638) |
| `self.cmd` | `0x3f` | Local state/value operations | Handled | [1639](../../crates/crysirror-avg32/src/vm/scenario.rs#L1639) |
| `self.cmd` | `0x40` | Local state/value operations | Handled | [1640](../../crates/crysirror-avg32/src/vm/scenario.rs#L1640) |
| `self.cmd` | `0x41` | Local state/value operations | Handled | [1641](../../crates/crysirror-avg32/src/vm/scenario.rs#L1641) |
| `self.cmd` | `0x42` | Local state/value operations | Handled | [1642](../../crates/crysirror-avg32/src/vm/scenario.rs#L1642) |
| `self.cmd` | `0x43` | Local state/value operations | Handled | [1643](../../crates/crysirror-avg32/src/vm/scenario.rs#L1643) |
| `self.cmd` | `0x49` | Local state/value operations | Handled | [1644](../../crates/crysirror-avg32/src/vm/scenario.rs#L1644) |
| `self.cmd` | `0x4a` | Local state/value operations | Handled | [1645](../../crates/crysirror-avg32/src/vm/scenario.rs#L1645) |
| `self.cmd` | `0x4b` | Local state/value operations | Handled | [1646](../../crates/crysirror-avg32/src/vm/scenario.rs#L1646) |
| `self.cmd` | `0x4c` | Local state/value operations | Handled | [1647](../../crates/crysirror-avg32/src/vm/scenario.rs#L1647) |
| `self.cmd` | `0x4d` | Local state/value operations | Handled | [1648](../../crates/crysirror-avg32/src/vm/scenario.rs#L1648) |
| `self.cmd` | `0x4e` | Local state/value operations | Handled | [1649](../../crates/crysirror-avg32/src/vm/scenario.rs#L1649) |
| `self.cmd` | `0x4f` | Local state/value operations | Handled | [1650](../../crates/crysirror-avg32/src/vm/scenario.rs#L1650) |
| `self.cmd` | `0x50` | Local state/value operations | Handled | [1651](../../crates/crysirror-avg32/src/vm/scenario.rs#L1651) |
| `self.cmd` | `0x51` | Local state/value operations | Handled | [1652](../../crates/crysirror-avg32/src/vm/scenario.rs#L1652) |
| `self.cmd` | `0x57` | self.value | Handled | [1653](../../crates/crysirror-avg32/src/vm/scenario.rs#L1653) |
| `self.cmd` | `_` | Local state/value operations | Handled | [1659](../../crates/crysirror-avg32/src/vm/scenario.rs#L1659) |

### vm/scenario.rs / Scenario::window_config

```rust
fn window_config(&mut self, sys: &mut System) -> bool
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under self.byte()=sub @ (0x03 \| 0x05)` | `sub == 0x03` | Local state/value operations | Handled | [2695](../../crates/crysirror-avg32/src/vm/scenario.rs#L2695) |
| `condition; under self.byte()=sub @ (0x04 \| 0x06)` | `sub == 0x04` | Local state/value operations | Handled | [2705](../../crates/crysirror-avg32/src/vm/scenario.rs#L2705) |

### vm/scenario.rs / Scenario::window_position

```rust
fn window_position(&mut self, sys: &mut System) -> bool
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `subcommand; under subcommand=0x01..=0x05` | `_` | Local state/value operations | Handled | [2743](../../crates/crysirror-avg32/src/vm/scenario.rs#L2743) |
| `subcommand; under subcommand=0x11..=0x15` | `_` | Local state/value operations | Handled | [2758](../../crates/crysirror-avg32/src/vm/scenario.rs#L2758) |
| `subcommand` | `other` | Local state/value operations | Handled | [2764](../../crates/crysirror-avg32/src/vm/scenario.rs#L2764) |

### vm/sound.rs / decode_koe_entry

```rust
fn decode_koe_entry(bytes: &[u8], entry: KoeEntry) -> Result<Vec<i16>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `code & 1 != 0` | Local state/value operations | Handled | [673](../../crates/crysirror-avg32/src/vm/sound.rs#L673) |
| `condition; under size=_` | `code == 15` | Local state/value operations | Handled | [699](../../crates/crysirror-avg32/src/vm/sound.rs#L699) |

### vm/sound.rs / wav_clip

```rust
fn wav_clip(bytes: &[u8], linear: bool) -> Result<Clip>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `id == b"fmt " && end - body >= 16` | u16::from_le_bytes, u32::from_le_bytes | Handled | [551](../../crates/crysirror-avg32/src/vm/sound.rs#L551) |
| `condition` | `id == b"data"` | Local state/value operations | Handled | [557](../../crates/crysirror-avg32/src/vm/sound.rs#L557) |

### vm/sound.rs / Sound::bgm_clip

```rust
fn bgm_clip(&mut self, ini: &Ini, resources: &Avg32Resources, name: &str) -> Option<Clip>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Ok(bytes) = resources.read("BGM", name)` | Local state/value operations | Handled | [311](../../crates/crysirror-avg32/src/vm/sound.rs#L311) |

### vm/system.rs / System::date_time

```rust
fn date_time(&self, kind: i32) -> i32
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `kind` | `1` | Local state/value operations | Handled | [429](../../crates/crysirror-avg32/src/vm/system.rs#L429) |
| `kind` | `2` | Local state/value operations | Handled | [430](../../crates/crysirror-avg32/src/vm/system.rs#L430) |
| `kind` | `3` | Local state/value operations | Handled | [431](../../crates/crysirror-avg32/src/vm/system.rs#L431) |
| `kind` | `4` | Local state/value operations | Handled | [432](../../crates/crysirror-avg32/src/vm/system.rs#L432) |
| `kind` | `_` | Local state/value operations | Handled | [433](../../crates/crysirror-avg32/src/vm/system.rs#L433) |

### vm/system.rs / System::redraw_macros

```rust
fn redraw_macros(&mut self) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under item.cmd=0x96 \| 0x97` | `item.cmd == 0x96` | self.load_named, self.gfx.load_base_file | Handled | [719](../../crates/crysirror-avg32/src/vm/system.rs#L719) |
| `item.cmd` | `0x00` | self.load_named, self.gfx.load_file, Rect::new | Handled | [663](../../crates/crysirror-avg32/src/vm/system.rs#L663) |
| `item.cmd` | `0x02` | self.load_named, self.gfx.load_file | Handled | [669](../../crates/crysirror-avg32/src/vm/system.rs#L669) |
| `item.cmd` | `0x32` | Rect::new | Handled | [673](../../crates/crysirror-avg32/src/vm/system.rs#L673) |
| `item.cmd` | `0x4c` | self.gfx.fade_color, Rect::new | Handled | [678](../../crates/crysirror-avg32/src/vm/system.rs#L678) |
| `item.cmd` | `0x64` | self.gfx.copy, Rect::new | Handled | [684](../../crates/crysirror-avg32/src/vm/system.rs#L684) |
| `item.cmd` | `0x66` | self.gfx.mask_copy, Rect::new | Handled | [692](../../crates/crysirror-avg32/src/vm/system.rs#L692) |
| `item.cmd` | `0x68` | self.gfx.color_key_copy, Rect::new | Handled | [700](../../crates/crysirror-avg32/src/vm/system.rs#L700) |
| `item.cmd` | `0x6a` | self.gfx.swap, Rect::new | Handled | [711](../../crates/crysirror-avg32/src/vm/system.rs#L711) |
| `item.cmd` | `0x96 \| 0x97` | self.load_named, self.gfx.load_base_file, self.gfx.all_copy, self.load_image, self.gfx.load_copy | Handled | [718](../../crates/crysirror-avg32/src/vm/system.rs#L718) |
| `item.cmd` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [746](../../crates/crysirror-avg32/src/vm/system.rs#L746) |

### vm/system.rs / System::screen_fade

```rust
fn screen_fade(&mut self, cmd: u8, count: i32, colour: [i32; 3]) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(cmd, 0x10 \| 0x11)` | Local state/value operations | Handled | [779](../../crates/crysirror-avg32/src/vm/system.rs#L779) |

### vm/system.rs / System::snr_load_copy

```rust
fn snr_load_copy(
        &mut self,
        name: &[u8],
        rect: Rect,
        dx: i32,
        dy: i32,
        destination: i32,
        method: u8,
    ) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `method == 1` | Local state/value operations | Handled | [875](../../crates/crysirror-avg32/src/vm/system.rs#L875) |

### vm/voicepatch.rs / VoicePatch::discover

```rust
fn discover(game: &Avg32Game) -> Option<Self>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!(name.starts_with("voicepat") && name.ends_with(".txt"))` | Local state/value operations | Handled | [59](../../crates/crysirror-avg32/src/vm/voicepatch.rs#L59) |
