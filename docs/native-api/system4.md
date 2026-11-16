# AliceSoft System 4 developer reference

[All engines](README.md)

Mode: `adopt`.

## ABI and host contract

AIN **v5** is the admitted execution format. Instructions are a little-endian
16-bit opcode followed by layout-defined 32-bit words. The layout catalogue has
273 entries; the generated rows distinguish executable routes from decode-only
entries. `CALLSYS` uses a numeric system ID; `CALLHLL` uses an AIN library/function
pair and its declared argument/return signature. Export names alone do not
establish parameter or version compatibility.

| Native area | CrySirror service / implementation | Return / completion |
| --- | --- | --- |
| Stack, heap, structures, arrays and delegates | `vm/` | Original i32/f32 bits, typed pages/references and destructor ordering. |
| `CALLSYS` / `CALLHLL` | `vm/external.rs`, `Bridge::dispatch_for` | Retained `ExternalCall`; typed return and output references commit after completion. |
| SACT2 / DrawGraph | Shared Image, Canvas and retained Layers | Native handles and sprite policy remain in the adapter. |
| Text / dialogs / platform requests | Shared Text, UI, input, clock and desktop-request services | `Dispatch::Completed`, `WaitUntil` or `Presented`; pending requests retain the call. |
| Music / sample queries | Shared Audio and scoped resource workers | Source-format sampling does not replace or advance playback cursors. |
| Global/resume slots | Native serializers over scoped Storage | Original ownership and save-safe boundaries are validated before restore. |

Known domains: AIN versions other than v5 are rejected. Source sample inspection
accepts RIFF integer PCM8/16 mono/stereo; unsupported compression/depth/layout and
positions beyond EOF fail. RDFT accepts finite power-of-two lengths 2..65536;
Bezier accepts at most 13 control points. Portable floating point is not x87
bit parity. Debug-log absence is a reviewed optional-interface policy, not a
successful load of a Windows DLL.

Source: [SOURCE.json](../../crates/crysirror-system4/SOURCE.json),
[native semantic findings](../../crates/crysirror-system4/NATIVE_SEMANTICS.md),
[external-call ABI](../../crates/crysirror-system4/src/vm/external.rs).
Behavior: [focused validation](../validation/system4-shared-native-2026-11-09.md).

<!-- BEGIN GENERATED DISPATCH TABLES -->

## Opcode and API implementation tables

`Handled` means an executable source branch exists. It is not a claim that every argument/subselector or a complete game has been audited. Explicit gaps are shown per row.

| Status | Entries |
| --- | ---: |
| Decode only | 29 |
| Empty handler | 4 |
| Handled | 578 |
| Partial | 5 |

These tables cover the checked-in catalogues and explicit dispatch branches. An engine-wide upstream denominator is not inferred from these rows.

## AIN VM opcodes

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x000 PUSH` | Push an immediate value | Handled | — | [mod.rs:769](../../crates/crysirror-system4/src/vm/mod.rs#L769) |
| `0x001 POP` | Pop a stack value | Handled | — | [mod.rs:770](../../crates/crysirror-system4/src/vm/mod.rs#L770) |
| `0x002 REF` | Read a referenced value | Handled | — | [mod.rs:783](../../crates/crysirror-system4/src/vm/mod.rs#L783) |
| `0x003 REFREF` | Read a nested reference | Handled | — | [mod.rs:794](../../crates/crysirror-system4/src/vm/mod.rs#L794) |
| `0x004 PUSHGLOBALPAGE` | PUSHGLOBALPAGE | Handled | — | [mod.rs:772](../../crates/crysirror-system4/src/vm/mod.rs#L772) |
| `0x005 PUSHLOCALPAGE` | PUSHLOCALPAGE | Handled | — | [mod.rs:773](../../crates/crysirror-system4/src/vm/mod.rs#L773) |
| `0x006 INV` | INV | Handled | — | [mod.rs:1134](../../crates/crysirror-system4/src/vm/mod.rs#L1134) |
| `0x007 NOT` | NOT | Handled | — | [mod.rs:1135](../../crates/crysirror-system4/src/vm/mod.rs#L1135) |
| `0x008 COMPL` | COMPL | Handled | — | [mod.rs:1136](../../crates/crysirror-system4/src/vm/mod.rs#L1136) |
| `0x009 ADD` | Addition | Handled | — | [mod.rs:1152](../../crates/crysirror-system4/src/vm/mod.rs#L1152) |
| `0x00a SUB` | Subtraction | Handled | — | [mod.rs:1152](../../crates/crysirror-system4/src/vm/mod.rs#L1152) |
| `0x00b MUL` | Multiplication | Handled | — | [mod.rs:1152](../../crates/crysirror-system4/src/vm/mod.rs#L1152) |
| `0x00c DIV` | Division | Handled | — | [mod.rs:1152](../../crates/crysirror-system4/src/vm/mod.rs#L1152) |
| `0x00d MOD` | Remainder | Handled | — | [mod.rs:1152](../../crates/crysirror-system4/src/vm/mod.rs#L1152) |
| `0x00e AND` | AND | Handled | — | [mod.rs:1152](../../crates/crysirror-system4/src/vm/mod.rs#L1152) |
| `0x00f OR` | OR | Handled | — | [mod.rs:1152](../../crates/crysirror-system4/src/vm/mod.rs#L1152) |
| `0x010 XOR` | XOR | Handled | — | [mod.rs:1152](../../crates/crysirror-system4/src/vm/mod.rs#L1152) |
| `0x011 LSHIFT` | LSHIFT | Handled | — | [mod.rs:1152](../../crates/crysirror-system4/src/vm/mod.rs#L1152) |
| `0x012 RSHIFT` | RSHIFT | Handled | — | [mod.rs:1152](../../crates/crysirror-system4/src/vm/mod.rs#L1152) |
| `0x013 LT` | LT | Handled | — | [mod.rs:1152](../../crates/crysirror-system4/src/vm/mod.rs#L1152) |
| `0x014 GT` | GT | Handled | — | [mod.rs:1152](../../crates/crysirror-system4/src/vm/mod.rs#L1152) |
| `0x015 LTE` | LTE | Handled | — | [mod.rs:1152](../../crates/crysirror-system4/src/vm/mod.rs#L1152) |
| `0x016 GTE` | GTE | Handled | — | [mod.rs:1152](../../crates/crysirror-system4/src/vm/mod.rs#L1152) |
| `0x017 NOTE` | NOTE | Handled | — | [mod.rs:1152](../../crates/crysirror-system4/src/vm/mod.rs#L1152) |
| `0x018 EQUALE` | EQUALE | Handled | — | [mod.rs:1152](../../crates/crysirror-system4/src/vm/mod.rs#L1152) |
| `0x019 ASSIGN` | Assign a value | Handled | — | [mod.rs:802](../../crates/crysirror-system4/src/vm/mod.rs#L802) |
| `0x01a PLUSA` | PLUSA | Handled | — | [mod.rs:1175](../../crates/crysirror-system4/src/vm/mod.rs#L1175) |
| `0x01b MINUSA` | MINUSA | Handled | — | [mod.rs:1176](../../crates/crysirror-system4/src/vm/mod.rs#L1176) |
| `0x01c MULA` | MULA | Handled | — | [mod.rs:1177](../../crates/crysirror-system4/src/vm/mod.rs#L1177) |
| `0x01d DIVA` | DIVA | Handled | — | [mod.rs:1178](../../crates/crysirror-system4/src/vm/mod.rs#L1178) |
| `0x01e MODA` | MODA | Handled | — | [mod.rs:1179](../../crates/crysirror-system4/src/vm/mod.rs#L1179) |
| `0x01f ANDA` | ANDA | Handled | — | [mod.rs:1183](../../crates/crysirror-system4/src/vm/mod.rs#L1183) |
| `0x020 ORA` | ORA | Handled | — | [mod.rs:1184](../../crates/crysirror-system4/src/vm/mod.rs#L1184) |
| `0x021 XORA` | XORA | Handled | — | [mod.rs:1185](../../crates/crysirror-system4/src/vm/mod.rs#L1185) |
| `0x022 LSHIFTA` | LSHIFTA | Handled | — | [mod.rs:1186](../../crates/crysirror-system4/src/vm/mod.rs#L1186) |
| `0x023 RSHIFTA` | RSHIFTA | Handled | — | [mod.rs:1162](../../crates/crysirror-system4/src/vm/mod.rs#L1162) |
| `0x024 F_ASSIGN` | F ASSIGN | Handled | — | [mod.rs:802](../../crates/crysirror-system4/src/vm/mod.rs#L802) |
| `0x025 F_PLUSA` | F PLUSA | Handled | — | [primitives.rs:60](../../crates/crysirror-system4/src/vm/primitives.rs#L60) |
| `0x026 F_MINUSA` | F MINUSA | Handled | — | [primitives.rs:61](../../crates/crysirror-system4/src/vm/primitives.rs#L61) |
| `0x027 F_MULA` | F MULA | Handled | — | [primitives.rs:62](../../crates/crysirror-system4/src/vm/primitives.rs#L62) |
| `0x028 F_DIVA` | F DIVA | Handled | — | [primitives.rs:55](../../crates/crysirror-system4/src/vm/primitives.rs#L55) |
| `0x029 DUP2` | DUP2 | Handled | — | [mod.rs:1108](../../crates/crysirror-system4/src/vm/mod.rs#L1108) |
| `0x02a DUP_X2` | DUP X2 | Handled | — | [mod.rs:1119](../../crates/crysirror-system4/src/vm/mod.rs#L1119) |
| `0x02b CMP` | CMP | Decode only | No extracted executor route | [opcodes.rs:48](../../crates/crysirror-system4/src/opcodes.rs#L48) |
| `0x02c JUMP` | Unconditional jump | Handled | — | [mod.rs:1048](../../crates/crysirror-system4/src/vm/mod.rs#L1048) |
| `0x02d IFZ` | Jump when zero | Handled | — | [mod.rs:1049](../../crates/crysirror-system4/src/vm/mod.rs#L1049) |
| `0x02e IFNZ` | Jump when nonzero | Handled | — | [mod.rs:1049](../../crates/crysirror-system4/src/vm/mod.rs#L1049) |
| `0x02f RETURN` | Return from a function | Handled | — | [mod.rs:996](../../crates/crysirror-system4/src/vm/mod.rs#L996) |
| `0x030 CALLFUNC` | Call a script function | Handled | — | [mod.rs:978](../../crates/crysirror-system4/src/vm/mod.rs#L978) |
| `0x031 INC` | INC | Handled | — | [mod.rs:1194](../../crates/crysirror-system4/src/vm/mod.rs#L1194) |
| `0x032 DEC` | DEC | Handled | — | [mod.rs:1194](../../crates/crysirror-system4/src/vm/mod.rs#L1194) |
| `0x033 FTOI` | FTOI | Handled | — | [mod.rs:1140](../../crates/crysirror-system4/src/vm/mod.rs#L1140) |
| `0x034 ITOF` | ITOF | Handled | — | [mod.rs:1139](../../crates/crysirror-system4/src/vm/mod.rs#L1139) |
| `0x035 F_INV` | F INV | Handled | — | [mod.rs:1131](../../crates/crysirror-system4/src/vm/mod.rs#L1131) |
| `0x036 F_ADD` | F ADD | Handled | — | [mod.rs:1152](../../crates/crysirror-system4/src/vm/mod.rs#L1152) |
| `0x037 F_SUB` | F SUB | Handled | — | [mod.rs:1152](../../crates/crysirror-system4/src/vm/mod.rs#L1152) |
| `0x038 F_MUL` | F MUL | Handled | — | [mod.rs:1152](../../crates/crysirror-system4/src/vm/mod.rs#L1152) |
| `0x039 F_DIV` | F DIV | Handled | — | [mod.rs:1152](../../crates/crysirror-system4/src/vm/mod.rs#L1152) |
| `0x03a F_LT` | F LT | Handled | — | [mod.rs:1152](../../crates/crysirror-system4/src/vm/mod.rs#L1152) |
| `0x03b F_GT` | F GT | Handled | — | [mod.rs:1152](../../crates/crysirror-system4/src/vm/mod.rs#L1152) |
| `0x03c F_LTE` | F LTE | Handled | — | [mod.rs:1152](../../crates/crysirror-system4/src/vm/mod.rs#L1152) |
| `0x03d F_GTE` | F GTE | Handled | — | [mod.rs:1152](../../crates/crysirror-system4/src/vm/mod.rs#L1152) |
| `0x03e F_NOTE` | F NOTE | Handled | — | [mod.rs:1152](../../crates/crysirror-system4/src/vm/mod.rs#L1152) |
| `0x03f F_EQUALE` | F EQUALE | Handled | — | [mod.rs:1152](../../crates/crysirror-system4/src/vm/mod.rs#L1152) |
| `0x040 F_PUSH` | F PUSH | Handled | — | [mod.rs:769](../../crates/crysirror-system4/src/vm/mod.rs#L769) |
| `0x041 S_PUSH` | S PUSH | Handled | — | [mod.rs:820](../../crates/crysirror-system4/src/vm/mod.rs#L820) |
| `0x042 S_POP` | S POP | Handled | — | [mod.rs:938](../../crates/crysirror-system4/src/vm/mod.rs#L938) |
| `0x043 S_ADD` | S ADD | Handled | — | [mod.rs:922](../../crates/crysirror-system4/src/vm/mod.rs#L922) |
| `0x044 S_ASSIGN` | S ASSIGN | Handled | — | [mod.rs:830](../../crates/crysirror-system4/src/vm/mod.rs#L830) |
| `0x045 S_PLUSA` | S PLUSA | Handled | — | [primitives.rs:170](../../crates/crysirror-system4/src/vm/primitives.rs#L170) |
| `0x046 S_REF` | S REF | Handled | — | [mod.rs:787](../../crates/crysirror-system4/src/vm/mod.rs#L787) |
| `0x047 S_REFREF` | S REFREF | Decode only | No extracted executor route | [opcodes.rs:76](../../crates/crysirror-system4/src/opcodes.rs#L76) |
| `0x048 S_NOTE` | S NOTE | Handled | — | [primitives.rs:140](../../crates/crysirror-system4/src/vm/primitives.rs#L140) |
| `0x049 S_EQUALE` | S EQUALE | Handled | — | [primitives.rs:128](../../crates/crysirror-system4/src/vm/primitives.rs#L128) |
| `0x04a SF_CREATE` | SF CREATE | Decode only | No extracted executor route | [opcodes.rs:79](../../crates/crysirror-system4/src/opcodes.rs#L79) |
| `0x04b SF_CREATEPIXEL` | SF CREATEPIXEL | Decode only | No extracted executor route | [opcodes.rs:80](../../crates/crysirror-system4/src/opcodes.rs#L80) |
| `0x04c SF_CREATEALPHA` | SF CREATEALPHA | Decode only | No extracted executor route | [opcodes.rs:81](../../crates/crysirror-system4/src/opcodes.rs#L81) |
| `0x04d SR_POP` | SR POP | Handled | — | [mod.rs:938](../../crates/crysirror-system4/src/vm/mod.rs#L938) |
| `0x04e SR_ASSIGN` | SR ASSIGN | Handled | — | [mod.rs:838](../../crates/crysirror-system4/src/vm/mod.rs#L838) |
| `0x04f SR_REF` | SR REF | Handled | — | [mod.rs:788](../../crates/crysirror-system4/src/vm/mod.rs#L788) |
| `0x050 SR_REFREF` | SR REFREF | Decode only | No extracted executor route | [opcodes.rs:85](../../crates/crysirror-system4/src/opcodes.rs#L85) |
| `0x051 A_ALLOC` | A ALLOC | Handled | — | [mod.rs:1216](../../crates/crysirror-system4/src/vm/mod.rs#L1216) |
| `0x052 A_REALLOC` | A REALLOC | Handled | — | [mod.rs:1216](../../crates/crysirror-system4/src/vm/mod.rs#L1216) |
| `0x053 A_FREE` | A FREE | Handled | — | [primitives.rs:202](../../crates/crysirror-system4/src/vm/primitives.rs#L202) |
| `0x054 A_NUMOF` | A NUMOF | Handled | — | [mod.rs:1220](../../crates/crysirror-system4/src/vm/mod.rs#L1220) |
| `0x055 A_COPY` | A COPY | Handled | — | [arrays.rs:193](../../crates/crysirror-system4/src/vm/arrays.rs#L193) |
| `0x056 A_FILL` | A FILL | Handled | — | [arrays.rs:147](../../crates/crysirror-system4/src/vm/arrays.rs#L147) |
| `0x057 C_REF` | C REF | Handled | — | [strings.rs:72](../../crates/crysirror-system4/src/vm/strings.rs#L72) |
| `0x058 C_ASSIGN` | C ASSIGN | Handled | — | [strings.rs:78](../../crates/crysirror-system4/src/vm/strings.rs#L78) |
| `0x059 MSG` | MSG | Handled | — | [functions.rs:27](../../crates/crysirror-system4/src/vm/functions.rs#L27) |
| `0x05a CALLHLL` | Call a native library export | Handled | — | [mod.rs:1003](../../crates/crysirror-system4/src/vm/mod.rs#L1003) |
| `0x05b PUSHSTRUCTPAGE` | PUSHSTRUCTPAGE | Handled | — | [mod.rs:774](../../crates/crysirror-system4/src/vm/mod.rs#L774) |
| `0x05c CALLMETHOD` | Call an object method | Handled | — | [mod.rs:982](../../crates/crysirror-system4/src/vm/mod.rs#L982) |
| `0x05d SH_GLOBALREF` | SH GLOBALREF | Handled | — | [mod.rs:777](../../crates/crysirror-system4/src/vm/mod.rs#L777) |
| `0x05e SH_LOCALREF` | SH LOCALREF | Handled | — | [mod.rs:778](../../crates/crysirror-system4/src/vm/mod.rs#L778) |
| `0x05f SWITCH` | Integer switch | Handled | — | [mod.rs:1085](../../crates/crysirror-system4/src/vm/mod.rs#L1085) |
| `0x060 STRSWITCH` | String switch | Handled | — | [mod.rs:1056](../../crates/crysirror-system4/src/vm/mod.rs#L1056) |
| `0x061 FUNC` | FUNC | Empty handler | Original no-op status not established | [mod.rs:771](../../crates/crysirror-system4/src/vm/mod.rs#L771) |
| `0x062 EOF` | File-end marker | Decode only | No extracted executor route | [opcodes.rs:277](../../crates/crysirror-system4/src/opcodes.rs#L277) |
| `0x063 CALLSYS` | Call a system function | Handled | — | [mod.rs:1020](../../crates/crysirror-system4/src/vm/mod.rs#L1020) |
| `0x064 SJUMP` | SJUMP | Partial | System4 scenario ABI without SLBL/OJMP metadata is unsupported | [functions.rs:119](../../crates/crysirror-system4/src/vm/functions.rs#L119) |
| `0x065 CALLONJUMP` | CALLONJUMP | Partial | System4 scenario ABI without SLBL/OJMP metadata is unsupported; no guessed preallocated page | [functions.rs:81](../../crates/crysirror-system4/src/vm/functions.rs#L81) |
| `0x066 SWAP` | SWAP | Handled | — | [mod.rs:1114](../../crates/crysirror-system4/src/vm/mod.rs#L1114) |
| `0x067 SH_STRUCTREF` | SH STRUCTREF | Handled | — | [mod.rs:775](../../crates/crysirror-system4/src/vm/mod.rs#L775) |
| `0x068 S_LENGTH` | S LENGTH | Handled | — | [strings.rs:21](../../crates/crysirror-system4/src/vm/strings.rs#L21) |
| `0x069 S_LENGTHBYTE` | S LENGTHBYTE | Handled | — | [primitives.rs:154](../../crates/crysirror-system4/src/vm/primitives.rs#L154) |
| `0x06a I_STRING` | I STRING | Handled | — | [primitives.rs:191](../../crates/crysirror-system4/src/vm/primitives.rs#L191) |
| `0x06b CALLFUNC2` | CALLFUNC2 | Handled | — | [primitives.rs:18](../../crates/crysirror-system4/src/vm/primitives.rs#L18) |
| `0x06c DUP2_X1` | DUP2 X1 | Handled | — | [mod.rs:1119](../../crates/crysirror-system4/src/vm/mod.rs#L1119) |
| `0x06d R_ASSIGN` | R ASSIGN | Handled | — | [mod.rs:809](../../crates/crysirror-system4/src/vm/mod.rs#L809) |
| `0x06e FT_ASSIGNS` | FT ASSIGNS | Handled | — | [functions.rs:66](../../crates/crysirror-system4/src/vm/functions.rs#L66) |
| `0x06f ASSERT` | ASSERT | Handled | — | [mod.rs:1027](../../crates/crysirror-system4/src/vm/mod.rs#L1027) |
| `0x070 S_LT` | S LT | Handled | — | [primitives.rs:136](../../crates/crysirror-system4/src/vm/primitives.rs#L136) |
| `0x071 S_GT` | S GT | Handled | — | [primitives.rs:137](../../crates/crysirror-system4/src/vm/primitives.rs#L137) |
| `0x072 S_LTE` | S LTE | Handled | — | [primitives.rs:138](../../crates/crysirror-system4/src/vm/primitives.rs#L138) |
| `0x073 S_GTE` | S GTE | Handled | — | [primitives.rs:139](../../crates/crysirror-system4/src/vm/primitives.rs#L139) |
| `0x074 S_LENGTH2` | S LENGTH2 | Handled | — | [strings.rs:21](../../crates/crysirror-system4/src/vm/strings.rs#L21) |
| `0x075 S_LENGTHBYTE2` | S LENGTHBYTE2 | Decode only | No extracted executor route | [opcodes.rs:120](../../crates/crysirror-system4/src/opcodes.rs#L120) |
| `0x076 NEW` | Create an object | Handled | — | [mod.rs:956](../../crates/crysirror-system4/src/vm/mod.rs#L956) |
| `0x077 DELETE` | Delete an object | Handled | — | [mod.rs:938](../../crates/crysirror-system4/src/vm/mod.rs#L938) |
| `0x078 CHECKUDO` | CHECKUDO | Decode only | No extracted executor route | [opcodes.rs:123](../../crates/crysirror-system4/src/opcodes.rs#L123) |
| `0x079 A_REF` | A REF | Handled | — | [mod.rs:1238](../../crates/crysirror-system4/src/vm/mod.rs#L1238) |
| `0x07a DUP` | DUP | Handled | — | [mod.rs:1104](../../crates/crysirror-system4/src/vm/mod.rs#L1104) |
| `0x07b DUP_U2` | DUP U2 | Handled | — | [mod.rs:1104](../../crates/crysirror-system4/src/vm/mod.rs#L1104) |
| `0x07c SP_INC` | SP INC | Handled | — | [mod.rs:951](../../crates/crysirror-system4/src/vm/mod.rs#L951) |
| `0x07d SP_DEC` | SP DEC | Decode only | No extracted executor route | [opcodes.rs:128](../../crates/crysirror-system4/src/opcodes.rs#L128) |
| `0x07e ENDFUNC` | Function-end marker | Decode only | No extracted executor route | [opcodes.rs:129](../../crates/crysirror-system4/src/opcodes.rs#L129) |
| `0x07f R_EQUALE` | R EQUALE | Handled | — | [primitives.rs:13](../../crates/crysirror-system4/src/vm/primitives.rs#L13) |
| `0x080 R_NOTE` | R NOTE | Handled | — | [primitives.rs:13](../../crates/crysirror-system4/src/vm/primitives.rs#L13) |
| `0x081 SH_LOCALCREATE` | SH LOCALCREATE | Handled | — | [mod.rs:956](../../crates/crysirror-system4/src/vm/mod.rs#L956) |
| `0x082 SH_LOCALDELETE` | SH LOCALDELETE | Handled | — | [primitives.rs:28](../../crates/crysirror-system4/src/vm/primitives.rs#L28) |
| `0x083 STOI` | STOI | Handled | — | [strings.rs:86](../../crates/crysirror-system4/src/vm/strings.rs#L86) |
| `0x084 A_PUSHBACK` | A PUSHBACK | Handled | — | [arrays.rs:286](../../crates/crysirror-system4/src/vm/arrays.rs#L286) |
| `0x085 A_POPBACK` | A POPBACK | Handled | — | [arrays.rs:347](../../crates/crysirror-system4/src/vm/arrays.rs#L347) |
| `0x086 S_EMPTY` | S EMPTY | Handled | — | [primitives.rs:160](../../crates/crysirror-system4/src/vm/primitives.rs#L160) |
| `0x087 A_EMPTY` | A EMPTY | Handled | — | [mod.rs:1228](../../crates/crysirror-system4/src/vm/mod.rs#L1228) |
| `0x088 A_ERASE` | A ERASE | Handled | — | [arrays.rs:347](../../crates/crysirror-system4/src/vm/arrays.rs#L347) |
| `0x089 A_INSERT` | A INSERT | Handled | — | [arrays.rs:286](../../crates/crysirror-system4/src/vm/arrays.rs#L286) |
| `0x08a SH_LOCALINC` | SH LOCALINC | Handled | — | [mod.rs:1202](../../crates/crysirror-system4/src/vm/mod.rs#L1202) |
| `0x08b SH_LOCALDEC` | SH LOCALDEC | Handled | — | [mod.rs:1202](../../crates/crysirror-system4/src/vm/mod.rs#L1202) |
| `0x08c SH_LOCALASSIGN` | SH LOCALASSIGN | Handled | — | [mod.rs:1201](../../crates/crysirror-system4/src/vm/mod.rs#L1201) |
| `0x08d ITOB` | ITOB | Handled | — | [mod.rs:1137](../../crates/crysirror-system4/src/vm/mod.rs#L1137) |
| `0x08e S_FIND` | S FIND | Handled | — | [strings.rs:36](../../crates/crysirror-system4/src/vm/strings.rs#L36) |
| `0x08f S_GETPART` | S GETPART | Handled | — | [strings.rs:44](../../crates/crysirror-system4/src/vm/strings.rs#L44) |
| `0x090 A_SORT` | A SORT | Handled | — | [array_sort.rs:64](../../crates/crysirror-system4/src/vm/array_sort.rs#L64) |
| `0x091 S_PUSHBACK` | S PUSHBACK | Decode only | No extracted executor route | [opcodes.rs:148](../../crates/crysirror-system4/src/opcodes.rs#L148) |
| `0x092 S_POPBACK` | S POPBACK | Decode only | No extracted executor route | [opcodes.rs:149](../../crates/crysirror-system4/src/opcodes.rs#L149) |
| `0x093 FTOS` | FTOS | Handled | — | [strings.rs:97](../../crates/crysirror-system4/src/vm/strings.rs#L97) |
| `0x094 S_MOD` | S MOD | Partial | System4 S_MOD format value type {kind} unsupported | [mod.rs:894](../../crates/crysirror-system4/src/vm/mod.rs#L894) |
| `0x095 S_PLUSA2` | S PLUSA2 | Handled | — | [primitives.rs:170](../../crates/crysirror-system4/src/vm/primitives.rs#L170) |
| `0x096 OBJSWAP` | OBJSWAP | Handled | — | [primitives.rs:47](../../crates/crysirror-system4/src/vm/primitives.rs#L47) |
| `0x097 S_ERASE` | S ERASE | Decode only | No extracted executor route | [opcodes.rs:154](../../crates/crysirror-system4/src/opcodes.rs#L154) |
| `0x098 SR_REF2` | SR REF2 | Handled | — | [mod.rs:1238](../../crates/crysirror-system4/src/vm/mod.rs#L1238) |
| `0x099 S_ERASE2` | S ERASE2 | Handled | — | [strings.rs:66](../../crates/crysirror-system4/src/vm/strings.rs#L66) |
| `0x09a S_PUSHBACK2` | S PUSHBACK2 | Handled | — | [strings.rs:54](../../crates/crysirror-system4/src/vm/strings.rs#L54) |
| `0x09b S_POPBACK2` | S POPBACK2 | Handled | — | [strings.rs:60](../../crates/crysirror-system4/src/vm/strings.rs#L60) |
| `0x09c ITOLI` | ITOLI | Handled | — | [mod.rs:1138](../../crates/crysirror-system4/src/vm/mod.rs#L1138) |
| `0x09d LI_ADD` | LI ADD | Handled | — | [mod.rs:1152](../../crates/crysirror-system4/src/vm/mod.rs#L1152) |
| `0x09e LI_SUB` | LI SUB | Handled | — | [mod.rs:1152](../../crates/crysirror-system4/src/vm/mod.rs#L1152) |
| `0x09f LI_MUL` | LI MUL | Handled | — | [mod.rs:1152](../../crates/crysirror-system4/src/vm/mod.rs#L1152) |
| `0x0a0 LI_DIV` | LI DIV | Handled | — | [mod.rs:1152](../../crates/crysirror-system4/src/vm/mod.rs#L1152) |
| `0x0a1 LI_MOD` | LI MOD | Handled | — | [mod.rs:1152](../../crates/crysirror-system4/src/vm/mod.rs#L1152) |
| `0x0a2 LI_ASSIGN` | LI ASSIGN | Handled | — | [primitives.rs:70](../../crates/crysirror-system4/src/vm/primitives.rs#L70) |
| `0x0a3 LI_PLUSA` | LI PLUSA | Handled | — | [primitives.rs:83](../../crates/crysirror-system4/src/vm/primitives.rs#L83) |
| `0x0a4 LI_MINUSA` | LI MINUSA | Handled | — | [primitives.rs:84](../../crates/crysirror-system4/src/vm/primitives.rs#L84) |
| `0x0a5 LI_MULA` | LI MULA | Handled | — | [primitives.rs:85](../../crates/crysirror-system4/src/vm/primitives.rs#L85) |
| `0x0a6 LI_DIVA` | LI DIVA | Handled | — | [primitives.rs:86](../../crates/crysirror-system4/src/vm/primitives.rs#L86) |
| `0x0a7 LI_MODA` | LI MODA | Handled | — | [primitives.rs:93](../../crates/crysirror-system4/src/vm/primitives.rs#L93) |
| `0x0a8 LI_ANDA` | LI ANDA | Handled | — | [primitives.rs:97](../../crates/crysirror-system4/src/vm/primitives.rs#L97) |
| `0x0a9 LI_ORA` | LI ORA | Handled | — | [primitives.rs:98](../../crates/crysirror-system4/src/vm/primitives.rs#L98) |
| `0x0aa LI_XORA` | LI XORA | Handled | — | [primitives.rs:99](../../crates/crysirror-system4/src/vm/primitives.rs#L99) |
| `0x0ab LI_LSHIFTA` | LI LSHIFTA | Handled | — | [primitives.rs:100](../../crates/crysirror-system4/src/vm/primitives.rs#L100) |
| `0x0ac LI_RSHIFTA` | LI RSHIFTA | Handled | — | [primitives.rs:77](../../crates/crysirror-system4/src/vm/primitives.rs#L77) |
| `0x0ad LI_INC` | LI INC | Handled | — | [primitives.rs:120](../../crates/crysirror-system4/src/vm/primitives.rs#L120) |
| `0x0ae LI_DEC` | LI DEC | Handled | — | [primitives.rs:120](../../crates/crysirror-system4/src/vm/primitives.rs#L120) |
| `0x0af A_FIND` | A FIND | Handled | — | [array_find.rs:24](../../crates/crysirror-system4/src/vm/array_find.rs#L24) |
| `0x0b0 A_REVERSE` | A REVERSE | Handled | — | [arrays.rs:395](../../crates/crysirror-system4/src/vm/arrays.rs#L395) |
| `0x0b1 SH_SR_ASSIGN` | SH SR ASSIGN | Handled | — | [mod.rs:838](../../crates/crysirror-system4/src/vm/mod.rs#L838) |
| `0x0b2 SH_MEM_ASSIGN_LOCAL` | SH MEM ASSIGN LOCAL | Handled | — | [mod.rs:1212](../../crates/crysirror-system4/src/vm/mod.rs#L1212) |
| `0x0b3 A_NUMOF_GLOB_1` | A NUMOF GLOB 1 | Handled | — | [shorthand.rs:117](../../crates/crysirror-system4/src/vm/shorthand.rs#L117) |
| `0x0b4 A_NUMOF_STRUCT_1` | A NUMOF STRUCT 1 | Handled | — | [shorthand.rs:117](../../crates/crysirror-system4/src/vm/shorthand.rs#L117) |
| `0x0b5 SH_MEM_ASSIGN_IMM` | SH MEM ASSIGN IMM | Handled | — | [mod.rs:1211](../../crates/crysirror-system4/src/vm/mod.rs#L1211) |
| `0x0b6 SH_LOCALREFREF` | SH LOCALREFREF | Handled | — | [primitives.rs:228](../../crates/crysirror-system4/src/vm/primitives.rs#L228) |
| `0x0b7 SH_LOCALASSIGN_SUB_IMM` | SH LOCALASSIGN SUB IMM | Handled | — | [primitives.rs:234](../../crates/crysirror-system4/src/vm/primitives.rs#L234) |
| `0x0b8 SH_IF_LOC_LT_IMM` | SH IF LOC LT IMM | Handled | — | [shorthand.rs:20](../../crates/crysirror-system4/src/vm/shorthand.rs#L20) |
| `0x0b9 SH_IF_LOC_GE_IMM` | SH IF LOC GE IMM | Handled | — | [shorthand.rs:21](../../crates/crysirror-system4/src/vm/shorthand.rs#L21) |
| `0x0ba SH_LOCREF_ASSIGN_MEM` | SH LOCREF ASSIGN MEM | Handled | — | [shorthand.rs:73](../../crates/crysirror-system4/src/vm/shorthand.rs#L73) |
| `0x0bb PAGE_REF` | PAGE REF | Handled | — | [primitives.rs:239](../../crates/crysirror-system4/src/vm/primitives.rs#L239) |
| `0x0bc SH_GLOBAL_ASSIGN_LOCAL` | SH GLOBAL ASSIGN LOCAL | Handled | — | [shorthand.rs:89](../../crates/crysirror-system4/src/vm/shorthand.rs#L89) |
| `0x0bd SH_STRUCTREF_GT_IMM` | SH STRUCTREF GT IMM | Handled | — | [primitives.rs:245](../../crates/crysirror-system4/src/vm/primitives.rs#L245) |
| `0x0be SH_STRUCT_ASSIGN_LOCALREF_ITOB` | SH STRUCT ASSIGN LOCALREF ITOB | Handled | — | [primitives.rs:248](../../crates/crysirror-system4/src/vm/primitives.rs#L248) |
| `0x0bf SH_LOCAL_ASSIGN_STRUCTREF` | SH LOCAL ASSIGN STRUCTREF | Handled | — | [primitives.rs:252](../../crates/crysirror-system4/src/vm/primitives.rs#L252) |
| `0x0c0 SH_IF_STRUCTREF_NE_LOCALREF` | SH IF STRUCTREF NE LOCALREF | Handled | — | [shorthand.rs:40](../../crates/crysirror-system4/src/vm/shorthand.rs#L40) |
| `0x0c1 SH_IF_STRUCTREF_GT_IMM` | SH IF STRUCTREF GT IMM | Handled | — | [shorthand.rs:32](../../crates/crysirror-system4/src/vm/shorthand.rs#L32) |
| `0x0c2 SH_STRUCTREF_CALLMETHOD_NO_PARAM` | SH STRUCTREF CALLMETHOD NO PARAM | Handled | — | [shorthand.rs:66](../../crates/crysirror-system4/src/vm/shorthand.rs#L66) |
| `0x0c3 SH_STRUCTREF2` | SH STRUCTREF2 | Handled | — | [primitives.rs:260](../../crates/crysirror-system4/src/vm/primitives.rs#L260) |
| `0x0c4 SH_REF_STRUCTREF2` | SH REF STRUCTREF2 | Handled | — | [primitives.rs:260](../../crates/crysirror-system4/src/vm/primitives.rs#L260) |
| `0x0c5 SH_STRUCTREF3` | SH STRUCTREF3 | Handled | — | [primitives.rs:273](../../crates/crysirror-system4/src/vm/primitives.rs#L273) |
| `0x0c6 SH_STRUCTREF2_CALLMETHOD_NO_PARAM` | SH STRUCTREF2 CALLMETHOD NO PARAM | Handled | — | [shorthand.rs:60](../../crates/crysirror-system4/src/vm/shorthand.rs#L60) |
| `0x0c7 SH_IF_STRUCTREF_Z` | SH IF STRUCTREF Z | Handled | — | [shorthand.rs:45](../../crates/crysirror-system4/src/vm/shorthand.rs#L45) |
| `0x0c8 SH_IF_STRUCT_A_NOT_EMPTY` | SH IF STRUCT A NOT EMPTY | Handled | — | [shorthand.rs:50](../../crates/crysirror-system4/src/vm/shorthand.rs#L50) |
| `0x0c9 SH_IF_LOC_GT_IMM` | SH IF LOC GT IMM | Handled | — | [shorthand.rs:22](../../crates/crysirror-system4/src/vm/shorthand.rs#L22) |
| `0x0ca SH_IF_STRUCTREF_NE_IMM` | SH IF STRUCTREF NE IMM | Handled | — | [shorthand.rs:33](../../crates/crysirror-system4/src/vm/shorthand.rs#L33) |
| `0x0cb THISCALLMETHOD_NOPARAM` | THISCALLMETHOD NOPARAM | Handled | — | [shorthand.rs:65](../../crates/crysirror-system4/src/vm/shorthand.rs#L65) |
| `0x0cc SH_IF_LOC_NE_IMM` | SH IF LOC NE IMM | Handled | — | [shorthand.rs:17](../../crates/crysirror-system4/src/vm/shorthand.rs#L17) |
| `0x0cd SH_IF_STRUCTREF_EQ_IMM` | SH IF STRUCTREF EQ IMM | Handled | — | [shorthand.rs:29](../../crates/crysirror-system4/src/vm/shorthand.rs#L29) |
| `0x0ce SH_GLOBAL_ASSIGN_IMM` | SH GLOBAL ASSIGN IMM | Handled | — | [shorthand.rs:89](../../crates/crysirror-system4/src/vm/shorthand.rs#L89) |
| `0x0cf SH_LOCALSTRUCT_ASSIGN_IMM` | SH LOCALSTRUCT ASSIGN IMM | Handled | — | [primitives.rs:256](../../crates/crysirror-system4/src/vm/primitives.rs#L256) |
| `0x0d0 SH_STRUCT_A_PUSHBACK_LOCAL_STRUCT` | SH STRUCT A PUSHBACK LOCAL STRUCT | Handled | — | [arrays.rs:244](../../crates/crysirror-system4/src/vm/arrays.rs#L244) |
| `0x0d1 SH_GLOBAL_A_PUSHBACK_LOCAL_STRUCT` | SH GLOBAL A PUSHBACK LOCAL STRUCT | Handled | — | [arrays.rs:245](../../crates/crysirror-system4/src/vm/arrays.rs#L245) |
| `0x0d2 SH_LOCAL_A_PUSHBACK_LOCAL_STRUCT` | SH LOCAL A PUSHBACK LOCAL STRUCT | Handled | — | [arrays.rs:240](../../crates/crysirror-system4/src/vm/arrays.rs#L240) |
| `0x0d3 SH_IF_SREF_NE_STR0` | SH IF SREF NE STR0 | Handled | — | [shorthand.rs:283](../../crates/crysirror-system4/src/vm/shorthand.rs#L283) |
| `0x0d4 SH_S_ASSIGN_REF` | SH S ASSIGN REF | Handled | — | [shorthand.rs:220](../../crates/crysirror-system4/src/vm/shorthand.rs#L220) |
| `0x0d5 SH_A_FIND_SREF` | SH A FIND SREF | Handled | — | [array_find.rs:25](../../crates/crysirror-system4/src/vm/array_find.rs#L25) |
| `0x0d6 SH_SREF_EMPTY` | SH SREF EMPTY | Handled | — | [shorthand.rs:145](../../crates/crysirror-system4/src/vm/shorthand.rs#L145) |
| `0x0d7 SH_STRUCTSREF_EQ_LOCALSREF` | SH STRUCTSREF EQ LOCALSREF | Handled | — | [shorthand.rs:259](../../crates/crysirror-system4/src/vm/shorthand.rs#L259) |
| `0x0d8 SH_LOCALSREF_EQ_STR0` | SH LOCALSREF EQ STR0 | Handled | — | [shorthand.rs:265](../../crates/crysirror-system4/src/vm/shorthand.rs#L265) |
| `0x0d9 SH_STRUCTSREF_NE_LOCALSREF` | SH STRUCTSREF NE LOCALSREF | Handled | — | [shorthand.rs:259](../../crates/crysirror-system4/src/vm/shorthand.rs#L259) |
| `0x0da SH_LOCALSREF_NE_STR0` | SH LOCALSREF NE STR0 | Handled | — | [shorthand.rs:265](../../crates/crysirror-system4/src/vm/shorthand.rs#L265) |
| `0x0db SH_STRUCT_SR_REF` | SH STRUCT SR REF | Handled | — | [shorthand.rs:126](../../crates/crysirror-system4/src/vm/shorthand.rs#L126) |
| `0x0dc SH_STRUCT_S_REF` | SH STRUCT S REF | Handled | — | [shorthand.rs:135](../../crates/crysirror-system4/src/vm/shorthand.rs#L135) |
| `0x0dd S_REF2` | S REF2 | Handled | — | [shorthand.rs:131](../../crates/crysirror-system4/src/vm/shorthand.rs#L131) |
| `0x0de SH_REF_LOCAL_ASSIGN_STRUCTREF2` | SH REF LOCAL ASSIGN STRUCTREF2 | Handled | — | [shorthand.rs:79](../../crates/crysirror-system4/src/vm/shorthand.rs#L79) |
| `0x0df SH_GLOBAL_S_REF` | SH GLOBAL S REF | Handled | — | [shorthand.rs:133](../../crates/crysirror-system4/src/vm/shorthand.rs#L133) |
| `0x0e0 SH_LOCAL_S_REF` | SH LOCAL S REF | Handled | — | [shorthand.rs:134](../../crates/crysirror-system4/src/vm/shorthand.rs#L134) |
| `0x0e1 SH_LOCALREF_SASSIGN_LOCALSREF` | SH LOCALREF SASSIGN LOCALSREF | Handled | — | [shorthand.rs:248](../../crates/crysirror-system4/src/vm/shorthand.rs#L248) |
| `0x0e2 SH_LOCAL_APUSHBACK_LOCALSREF` | SH LOCAL APUSHBACK LOCALSREF | Handled | — | [shorthand.rs:174](../../crates/crysirror-system4/src/vm/shorthand.rs#L174) |
| `0x0e3 SH_S_ASSIGN_CALLSYS19` | SH S ASSIGN CALLSYS19 | Handled | — | [shorthand.rs:160](../../crates/crysirror-system4/src/vm/shorthand.rs#L160) |
| `0x0e4 SH_S_ASSIGN_STR0` | SH S ASSIGN STR0 | Handled | — | [shorthand.rs:227](../../crates/crysirror-system4/src/vm/shorthand.rs#L227) |
| `0x0e5 SH_SASSIGN_LOCALSREF` | SH SASSIGN LOCALSREF | Handled | — | [shorthand.rs:240](../../crates/crysirror-system4/src/vm/shorthand.rs#L240) |
| `0x0e6 SH_STRUCTREF_SASSIGN_LOCALSREF` | SH STRUCTREF SASSIGN LOCALSREF | Handled | — | [shorthand.rs:248](../../crates/crysirror-system4/src/vm/shorthand.rs#L248) |
| `0x0e7 SH_LOCALSREF_EMPTY` | SH LOCALSREF EMPTY | Handled | — | [shorthand.rs:153](../../crates/crysirror-system4/src/vm/shorthand.rs#L153) |
| `0x0e8 SH_GLOBAL_APUSHBACK_LOCALSREF` | SH GLOBAL APUSHBACK LOCALSREF | Handled | — | [shorthand.rs:175](../../crates/crysirror-system4/src/vm/shorthand.rs#L175) |
| `0x0e9 SH_STRUCT_APUSHBACK_LOCALSREF` | SH STRUCT APUSHBACK LOCALSREF | Handled | — | [shorthand.rs:170](../../crates/crysirror-system4/src/vm/shorthand.rs#L170) |
| `0x0ea SH_STRUCTSREF_EMPTY` | SH STRUCTSREF EMPTY | Handled | — | [shorthand.rs:154](../../crates/crysirror-system4/src/vm/shorthand.rs#L154) |
| `0x0eb SH_GLOBALSREF_EMPTY` | SH GLOBALSREF EMPTY | Handled | — | [shorthand.rs:151](../../crates/crysirror-system4/src/vm/shorthand.rs#L151) |
| `0x0ec SH_SASSIGN_STRUCTSREF` | SH SASSIGN STRUCTSREF | Handled | — | [shorthand.rs:241](../../crates/crysirror-system4/src/vm/shorthand.rs#L241) |
| `0x0ed SH_SASSIGN_GLOBALSREF` | SH SASSIGN GLOBALSREF | Handled | — | [shorthand.rs:238](../../crates/crysirror-system4/src/vm/shorthand.rs#L238) |
| `0x0ee SH_STRUCTSREF_NE_STR0` | SH STRUCTSREF NE STR0 | Handled | — | [shorthand.rs:270](../../crates/crysirror-system4/src/vm/shorthand.rs#L270) |
| `0x0ef SH_GLOBALSREF_NE_STR0` | SH GLOBALSREF NE STR0 | Handled | — | [shorthand.rs:271](../../crates/crysirror-system4/src/vm/shorthand.rs#L271) |
| `0x0f0 SH_LOC_LT_IMM_OR_LOC_GE_IMM` | SH LOC LT IMM OR LOC GE IMM | Handled | — | [primitives.rs:279](../../crates/crysirror-system4/src/vm/primitives.rs#L279) |
| `0x0f1 A_SORT_MEM` | A SORT MEM | Handled | — | [array_sort.rs:65](../../crates/crysirror-system4/src/vm/array_sort.rs#L65) |
| `0x0f2 DG_SET` | DG SET | Handled | — | [delegates.rs:210](../../crates/crysirror-system4/src/vm/delegates.rs#L210) |
| `0x0f3 DG_ADD` | DG ADD | Handled | — | [delegates.rs:211](../../crates/crysirror-system4/src/vm/delegates.rs#L211) |
| `0x0f4 DG_CALL` | DG CALL | Handled | — | [delegates.rs:357](../../crates/crysirror-system4/src/vm/delegates.rs#L357) |
| `0x0f5 DG_NUMOF` | DG NUMOF | Handled | — | [delegates.rs:218](../../crates/crysirror-system4/src/vm/delegates.rs#L218) |
| `0x0f6 DG_EXIST` | DG EXIST | Handled | — | [delegates.rs:197](../../crates/crysirror-system4/src/vm/delegates.rs#L197) |
| `0x0f7 DG_ERASE` | DG ERASE | Handled | — | [delegates.rs:197](../../crates/crysirror-system4/src/vm/delegates.rs#L197) |
| `0x0f8 DG_CLEAR` | DG CLEAR | Handled | — | [delegates.rs:238](../../crates/crysirror-system4/src/vm/delegates.rs#L238) |
| `0x0f9 DG_COPY` | DG COPY | Handled | — | [delegates.rs:249](../../crates/crysirror-system4/src/vm/delegates.rs#L249) |
| `0x0fa DG_ASSIGN` | DG ASSIGN | Handled | — | [delegates.rs:256](../../crates/crysirror-system4/src/vm/delegates.rs#L256) |
| `0x0fb DG_PLUSA` | DG PLUSA | Handled | — | [delegates.rs:256](../../crates/crysirror-system4/src/vm/delegates.rs#L256) |
| `0x0fc DG_POP` | DG POP | Handled | — | [delegates.rs:313](../../crates/crysirror-system4/src/vm/delegates.rs#L313) |
| `0x0fd DG_NEW_FROM_METHOD` | DG NEW FROM METHOD | Handled | — | [delegates.rs:320](../../crates/crysirror-system4/src/vm/delegates.rs#L320) |
| `0x0fe DG_MINUSA` | DG MINUSA | Handled | — | [delegates.rs:256](../../crates/crysirror-system4/src/vm/delegates.rs#L256) |
| `0x0ff DG_CALLBEGIN` | DG CALLBEGIN | Handled | — | [delegates.rs:335](../../crates/crysirror-system4/src/vm/delegates.rs#L335) |
| `0x100 DG_NEW` | DG NEW | Handled | — | [delegates.rs:329](../../crates/crysirror-system4/src/vm/delegates.rs#L329) |
| `0x101 DG_STR_TO_METHOD` | DG STR TO METHOD | Handled | — | [delegates.rs:361](../../crates/crysirror-system4/src/vm/delegates.rs#L361) |
| `0x102 OP_0X102` | OP 0X102 | Decode only | No extracted executor route | [opcodes.rs:261](../../crates/crysirror-system4/src/opcodes.rs#L261) |
| `0x103 X_GETENV` | X GETENV | Decode only | No extracted executor route | [opcodes.rs:262](../../crates/crysirror-system4/src/opcodes.rs#L262) |
| `0x104 X_SET` | X SET | Decode only | No extracted executor route | [opcodes.rs:263](../../crates/crysirror-system4/src/opcodes.rs#L263) |
| `0x105 X_ICAST` | X ICAST | Decode only | No extracted executor route | [opcodes.rs:264](../../crates/crysirror-system4/src/opcodes.rs#L264) |
| `0x106 X_OP_SET` | X OP SET | Decode only | No extracted executor route | [opcodes.rs:265](../../crates/crysirror-system4/src/opcodes.rs#L265) |
| `0x107 OP_0X107` | OP 0X107 | Decode only | No extracted executor route | [opcodes.rs:266](../../crates/crysirror-system4/src/opcodes.rs#L266) |
| `0x108 OP_0X108` | OP 0X108 | Decode only | No extracted executor route | [opcodes.rs:267](../../crates/crysirror-system4/src/opcodes.rs#L267) |
| `0x109 OP_0X109` | OP 0X109 | Decode only | No extracted executor route | [opcodes.rs:268](../../crates/crysirror-system4/src/opcodes.rs#L268) |
| `0x10a X_DUP` | X DUP | Decode only | No extracted executor route | [opcodes.rs:269](../../crates/crysirror-system4/src/opcodes.rs#L269) |
| `0x10b X_MOV` | X MOV | Decode only | No extracted executor route | [opcodes.rs:270](../../crates/crysirror-system4/src/opcodes.rs#L270) |
| `0x10c X_REF` | X REF | Decode only | No extracted executor route | [opcodes.rs:271](../../crates/crysirror-system4/src/opcodes.rs#L271) |
| `0x10d X_ASSIGN` | X ASSIGN | Decode only | No extracted executor route | [opcodes.rs:272](../../crates/crysirror-system4/src/opcodes.rs#L272) |
| `0x10e X_A_INIT` | X A INIT | Decode only | No extracted executor route | [opcodes.rs:273](../../crates/crysirror-system4/src/opcodes.rs#L273) |
| `0x10f X_A_SIZE` | X A SIZE | Decode only | No extracted executor route | [opcodes.rs:274](../../crates/crysirror-system4/src/opcodes.rs#L274) |
| `0x110 X_TO_STR` | X TO STR | Decode only | No extracted executor route | [opcodes.rs:275](../../crates/crysirror-system4/src/opcodes.rs#L275) |

## Native exports / adapter

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `(Clear, 0, [Argument::Scalar { kind: 10, .. }])` | Clear | Handled | — | [adapter.rs:275](../../crates/crysirror-system4/src/adapter.rs#L275) |
| `(Create, 10, [Argument::String { .. }])` | Create | Handled | — | [adapter.rs:269](../../crates/crysirror-system4/src/adapter.rs#L269) |
| `(DisableAutoSave, 47, [Argument::Scalar { kind: 10, .. }])` | Disable Auto Save | Handled | — | [adapter.rs:286](../../crates/crysirror-system4/src/adapter.rs#L286) |
| `(EnableAutoSave, 47, [Argument::Scalar { kind: 10, .. }, Argument::String { .. }])` | Enable Auto Save | Handled | — | [adapter.rs:281](../../crates/crysirror-system4/src/adapter.rs#L281) |
| `(JoypadQuake_Set, 0, [Argument::Scalar { kind: 10, .. }, Argument::Scalar { kind: 10, .. }, Argument::Scalar { kind: 10, .. }])` | Joypad Quake Set | Handled | — | [adapter.rs:145](../../crates/crysirror-system4/src/adapter.rs#L145) |
| `(Joypad_ClearKeyDownFlag, 0, [Argument::Scalar { kind: 10, .. }])` | Joypad Clear Key Down Flag | Handled | — | [adapter.rs:140](../../crates/crysirror-system4/src/adapter.rs#L140) |
| `(Joypad_GetAnalogStickStatus, 47, [Argument::Scalar { kind: 10, .. }, Argument::Scalar { kind: 10, .. }, Argument::Reference { kind: 19, .. }, Argument::Reference { kind: 19, .. }])` | Joypad Get Analog Stick Status | Handled | — | [adapter.rs:143](../../crates/crysirror-system4/src/adapter.rs#L143) |
| `(Joypad_GetDigitalStickStatus, 47, [Argument::Scalar { kind: 10, .. }, Argument::Scalar { kind: 10, .. }, Argument::Reference { kind: 51, .. }, Argument::Reference { kind: 51, .. }, Argument::Reference { kind: 51, .. }, Argument::Reference { kind: 51, .. }])` | Joypad Get Digital Stick Status | Handled | — | [adapter.rs:144](../../crates/crysirror-system4/src/adapter.rs#L144) |
| `(Joypad_GetNumof, 10, [])` | Joypad Get Numof | Handled | — | [adapter.rs:142](../../crates/crysirror-system4/src/adapter.rs#L142) |
| `(Joypad_IsKeyDown, 10, [Argument::Scalar { kind: 10, .. }, Argument::Scalar { kind: 10, .. }])` | Joypad Is Key Down | Handled | — | [adapter.rs:141](../../crates/crysirror-system4/src/adapter.rs#L141) |
| `(Key_ClearFlag, 10, [])` | Key Clear Flag | Handled | — | [adapter.rs:200](../../crates/crysirror-system4/src/adapter.rs#L200) |
| `(Key_ClearFlagNoCtrl, 0, [])` | Key Clear Flag No Ctrl | Handled | — | [adapter.rs:204](../../crates/crysirror-system4/src/adapter.rs#L204) |
| `(Key_ClearFlagOne, 0, [Argument::Scalar {kind:10,bits:key}])` | Key Clear Flag One | Handled | — | [adapter.rs:208](../../crates/crysirror-system4/src/adapter.rs#L208) |
| `(Key_IsDown, 10, [Argument::Scalar { kind: 10, bits }])` | Key Is Down | Handled | — | [adapter.rs:197](../../crates/crysirror-system4/src/adapter.rs#L197) |
| `(Mouse_ClearWheel, 0, [])` | Mouse Clear Wheel | Handled | — | [adapter.rs:212](../../crates/crysirror-system4/src/adapter.rs#L212) |
| `(Mouse_GetPos, 10, [Argument::Reference { kind: 18, page: xp, index: xi, }, Argument::Reference { kind: 18, page: yp, index: yi, }])` | Mouse Get Pos | Handled | — | [adapter.rs:216](../../crates/crysirror-system4/src/adapter.rs#L216) |
| `(Mouse_GetWheel, 0, [Argument::Reference { kind: 18, page: xp, index: xi, }, Argument::Reference { kind: 18, page: yp, index: yi, }])` | Mouse Get Wheel | Handled | — | [adapter.rs:235](../../crates/crysirror-system4/src/adapter.rs#L235) |
| `(Output, 0, [Argument::Scalar { kind: 10, .. }, Argument::String { .. }])` | Output | Handled | — | [adapter.rs:270](../../crates/crysirror-system4/src/adapter.rs#L270) |
| `(Save, 10 \| 47, [Argument::Scalar { kind: 10, .. }, Argument::String { .. }])` | Save | Handled | — | [adapter.rs:276](../../crates/crysirror-system4/src/adapter.rs#L276) |
| `GAME_MSG_GetNumof` | GAME MSG Get Numof | Handled | — | [adapter.rs:101](../../crates/crysirror-system4/src/adapter.rs#L101) |
| `GetMainSurfaceNumber` | Get Main Surface Number | Handled | — | [adapter.rs:100](../../crates/crysirror-system4/src/adapter.rs#L100) |
| `GetScreenHeight` | Get Screen Height | Handled | — | [adapter.rs:94](../../crates/crysirror-system4/src/adapter.rs#L94) |
| `GetScreenWidth` | Get Screen Width | Handled | — | [adapter.rs:88](../../crates/crysirror-system4/src/adapter.rs#L88) |
| `Timer_Get` | Timer Get | Handled | — | [adapter.rs:102](../../crates/crysirror-system4/src/adapter.rs#L102) |

## Native exports / bootstrap

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `CodeName` | Code Name | Handled | — | [bootstrap.rs:319](../../crates/crysirror-system4/src/bootstrap.rs#L319) |
| `DefaultVolumeRate` | Default Volume Rate | Handled | — | [bootstrap.rs:331](../../crates/crysirror-system4/src/bootstrap.rs#L331) |
| `GameName` | Game Name | Handled | — | [bootstrap.rs:320](../../crates/crysirror-system4/src/bootstrap.rs#L320) |
| `MainVM` | Main VM | Handled | — | [bootstrap.rs:322](../../crates/crysirror-system4/src/bootstrap.rs#L322) |
| `SaveFolder` | Save Folder | Handled | — | [bootstrap.rs:321](../../crates/crysirror-system4/src/bootstrap.rs#L321) |
| `UseJoypad` | Use Joypad | Handled | — | [bootstrap.rs:325](../../crates/crysirror-system4/src/bootstrap.rs#L325) |
| `ViewHeight` | View Height | Handled | — | [bootstrap.rs:324](../../crates/crysirror-system4/src/bootstrap.rs#L324) |
| `ViewWidth` | View Width | Handled | — | [bootstrap.rs:323](../../crates/crysirror-system4/src/bootstrap.rs#L323) |
| `VolumeValancer` | Volume Valancer | Handled | — | [bootstrap.rs:332](../../crates/crysirror-system4/src/bootstrap.rs#L332) |
| `false` | false | Handled | — | [bootstrap.rs:243](../../crates/crysirror-system4/src/bootstrap.rs#L243) |
| `true` | true | Handled | — | [bootstrap.rs:242](../../crates/crysirror-system4/src/bootstrap.rs#L242) |

## Native exports / math / receiver=kind == 10

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `(Abs, [x])` | Abs | Handled | — | [math.rs:100](../../crates/crysirror-system4/src/math.rs#L100) |
| `(Max, [a, b])` | Max | Handled | — | [math.rs:110](../../crates/crysirror-system4/src/math.rs#L110) |
| `(Min, [a, b])` | Min | Handled | — | [math.rs:103](../../crates/crysirror-system4/src/math.rs#L103) |

## Native exports / math / receiver=kind == 11

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `(AbsF, [x])` | Abs F | Handled | — | [math.rs:144](../../crates/crysirror-system4/src/math.rs#L144) |
| `(Acos, [x])` | Acos | Handled | — | [math.rs:141](../../crates/crysirror-system4/src/math.rs#L141) |
| `(Asin, [x])` | Asin | Handled | — | [math.rs:140](../../crates/crysirror-system4/src/math.rs#L140) |
| `(Atan, [x])` | Atan | Handled | — | [math.rs:142](../../crates/crysirror-system4/src/math.rs#L142) |
| `(Atan2, [a, b])` | Atan2 | Handled | — | [math.rs:143](../../crates/crysirror-system4/src/math.rs#L143) |
| `(Cos, [x])` | Cos | Handled | — | [math.rs:136](../../crates/crysirror-system4/src/math.rs#L136) |
| `(Log, [x])` | Log | Handled | — | [math.rs:146](../../crates/crysirror-system4/src/math.rs#L146) |
| `(Log10, [x])` | Log10 | Handled | — | [math.rs:147](../../crates/crysirror-system4/src/math.rs#L147) |
| `(MaxF, [a, b])` | Max F | Handled | — | [math.rs:155](../../crates/crysirror-system4/src/math.rs#L155) |
| `(MinF, [a, b])` | Min F | Handled | — | [math.rs:148](../../crates/crysirror-system4/src/math.rs#L148) |
| `(Pow, [a, b])` | Pow | Handled | — | [math.rs:145](../../crates/crysirror-system4/src/math.rs#L145) |
| `(Sin, [x])` | Sin | Handled | — | [math.rs:137](../../crates/crysirror-system4/src/math.rs#L137) |
| `(Sqrt, [x])` | Sqrt | Handled | — | [math.rs:139](../../crates/crysirror-system4/src/math.rs#L139) |
| `(Tan, [x])` | Tan | Handled | — | [math.rs:138](../../crates/crysirror-system4/src/math.rs#L138) |

## Native exports / math_random

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `(RandTable2, 10, [Argument::Scalar { kind: 10, bits: id }])` | Rand Table2 | Handled | — | [math_random.rs:86](../../crates/crysirror-system4/src/math_random.rs#L86) |
| `(RandTable2Init, 0, [ Argument::Scalar { kind: 10, bits: id }, Argument::Page { kind: 14, slot }, ])` | Rand Table2Init | Handled | — | [math_random.rs:78](../../crates/crysirror-system4/src/math_random.rs#L78) |
| `Rand` | Rand | Handled | — | [math_random.rs:189](../../crates/crysirror-system4/src/math_random.rs#L189) |
| `RandF` | Rand F | Handled | — | [math_random.rs:202](../../crates/crysirror-system4/src/math_random.rs#L202) |
| `RandTable` | Rand Table | Handled | — | [math_random.rs:243](../../crates/crysirror-system4/src/math_random.rs#L243) |
| `RandTableInit` | Rand Table Init | Handled | — | [math_random.rs:215](../../crates/crysirror-system4/src/math_random.rs#L215) |
| `SetRandMode` | Original Math.dll18F0 retains every integer; 1280/1300 select the four-sample distribution only for exactly1, else unif… | Handled | — | [math_random.rs:184](../../crates/crysirror-system4/src/math_random.rs#L184) |
| `SetSeed` | Set Seed | Handled | — | [math_random.rs:176](../../crates/crysirror-system4/src/math_random.rs#L176) |

## Native exports / native_audio

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `Fade` | Fade | Handled | — | [native_audio.rs:709](../../crates/crysirror-system4/src/native_audio.rs#L709) |
| `(Fade, [_, milliseconds, volume, stop])` | Fade | Partial | System4 negative native audio fade duration unsupported | [native_audio.rs:977](../../crates/crysirror-system4/src/native_audio.rs#L977) |
| `(GetGroupNum, [_])` | Get Group Num | Handled | — | [native_audio.rs:976](../../crates/crysirror-system4/src/native_audio.rs#L976) |
| `(GetLength, [_])` | Get Length | Handled | — | [native_audio.rs:970](../../crates/crysirror-system4/src/native_audio.rs#L970) |
| `(GetLoopCount, [_])` | Get Loop Count | Handled | — | [native_audio.rs:891](../../crates/crysirror-system4/src/native_audio.rs#L891) |
| `(GetMixerDefaultVolume, Argument::Reference { kind: 18, page, index, })` | Get Mixer Default Volume | Handled | — | [native_audio.rs:400](../../crates/crysirror-system4/src/native_audio.rs#L400) |
| `(GetMixerDefaultVolume, Argument::Reference { page, index: field, .. })` | Get Mixer Default Volume | Handled | — | [native_audio.rs:429](../../crates/crysirror-system4/src/native_audio.rs#L429) |
| `(GetMixerMute, Argument::Reference { kind: 18, page, index, })` | Get Mixer Mute | Handled | — | [native_audio.rs:400](../../crates/crysirror-system4/src/native_audio.rs#L400) |
| `(GetMixerMute, Argument::Reference { page, index: field, .. })` | Get Mixer Mute | Handled | — | [native_audio.rs:437](../../crates/crysirror-system4/src/native_audio.rs#L437) |
| `(GetMixerName, Argument::HeapReference { kind: 20, slot })` | Get Mixer Name | Handled | — | [native_audio.rs:397](../../crates/crysirror-system4/src/native_audio.rs#L397) |
| `(GetMixerName, Argument::HeapReference { slot, .. })` | Get Mixer Name | Handled | — | [native_audio.rs:424](../../crates/crysirror-system4/src/native_audio.rs#L424) |
| `(GetMixerVolume, Argument::Reference { kind: 18, page, index, })` | Get Mixer Volume | Handled | — | [native_audio.rs:400](../../crates/crysirror-system4/src/native_audio.rs#L400) |
| `(GetMixerVolume, Argument::Reference { page, index: field, .. })` | Get Mixer Volume | Handled | — | [native_audio.rs:437](../../crates/crysirror-system4/src/native_audio.rs#L437) |
| `(GetPos, [_])` | Get Pos | Handled | — | [native_audio.rs:966](../../crates/crysirror-system4/src/native_audio.rs#L966) |
| `(GetSampleLength, [_])` | Get Sample Length | Handled | — | [native_audio.rs:975](../../crates/crysirror-system4/src/native_audio.rs#L975) |
| `(GetSamplePos, [_])` | Get Sample Pos | Handled | — | [native_audio.rs:974](../../crates/crysirror-system4/src/native_audio.rs#L974) |
| `(GetTimeLength, [_])` | Get Time Length | Handled | — | [native_audio.rs:970](../../crates/crysirror-system4/src/native_audio.rs#L970) |
| `(GetVolume, [_])` | Get Volume | Handled | — | [native_audio.rs:965](../../crates/crysirror-system4/src/native_audio.rs#L965) |
| `(IsFade, [_])` | Is Fade | Handled | — | [native_audio.rs:1011](../../crates/crysirror-system4/src/native_audio.rs#L1011) |
| `(IsPlay, [_])` | Is Play | Handled | — | [native_audio.rs:890](../../crates/crysirror-system4/src/native_audio.rs#L890) |
| `(Play, [_])` | Play | Handled | — | [native_audio.rs:876](../../crates/crysirror-system4/src/native_audio.rs#L876) |
| `(ReverseLR, [_])` | Reverse LR | Handled | — | [native_audio.rs:951](../../crates/crysirror-system4/src/native_audio.rs#L951) |
| `(SetLoopCount, [_, count])` | Set Loop Count | Handled | — | [native_audio.rs:892](../../crates/crysirror-system4/src/native_audio.rs#L892) |
| `(SetLoopEndPos, [_, end])` | Set Loop End Pos | Handled | — | [native_audio.rs:931](../../crates/crysirror-system4/src/native_audio.rs#L931) |
| `(SetLoopStartPos, [_, start])` | Set Loop Start Pos | Handled | — | [native_audio.rs:911](../../crates/crysirror-system4/src/native_audio.rs#L911) |
| `(SetMixerMute, Argument::Scalar { bits, .. })` | Set Mixer Mute | Handled | — | [native_audio.rs:457](../../crates/crysirror-system4/src/native_audio.rs#L457) |
| `(SetMixerMute, Argument::Scalar { kind: 10, bits: _ })` | Set Mixer Mute | Empty handler | Original no-op status not established | [native_audio.rs:411](../../crates/crysirror-system4/src/native_audio.rs#L411) |
| `(SetMixerName, Argument::String { .. })` | Set Mixer Name | Empty handler | Original no-op status not established | [native_audio.rs:410](../../crates/crysirror-system4/src/native_audio.rs#L410) |
| `(SetMixerName, Argument::String { bytes, .. })` | Set Mixer Name | Handled | — | [native_audio.rs:453](../../crates/crysirror-system4/src/native_audio.rs#L453) |
| `(SetMixerVolume, Argument::Scalar { bits, .. })` | Set Mixer Volume | Handled | — | [native_audio.rs:457](../../crates/crysirror-system4/src/native_audio.rs#L457) |
| `(SetMixerVolume, Argument::Scalar { kind: 10, bits: _ })` | Set Mixer Volume | Empty handler | Original no-op status not established | [native_audio.rs:411](../../crates/crysirror-system4/src/native_audio.rs#L411) |
| `(Stop, [_])` | Stop | Handled | — | [native_audio.rs:887](../../crates/crysirror-system4/src/native_audio.rs#L887) |
| `(StopFade, [_])` | Stop Fade | Handled | — | [native_audio.rs:1000](../../crates/crysirror-system4/src/native_audio.rs#L1000) |
| `GetGroupNum` | Get Group Num | Handled | — | [native_audio.rs:701](../../crates/crysirror-system4/src/native_audio.rs#L701) |
| `GetLength` | Get Length | Handled | — | [native_audio.rs:701](../../crates/crysirror-system4/src/native_audio.rs#L701) |
| `GetLoopCount` | Get Loop Count | Handled | — | [native_audio.rs:701](../../crates/crysirror-system4/src/native_audio.rs#L701) |
| `GetPos` | Get Pos | Handled | — | [native_audio.rs:701](../../crates/crysirror-system4/src/native_audio.rs#L701) |
| `GetSampleLength` | Get Sample Length | Handled | — | [native_audio.rs:701](../../crates/crysirror-system4/src/native_audio.rs#L701) |
| `GetSamplePos` | Get Sample Pos | Handled | — | [native_audio.rs:701](../../crates/crysirror-system4/src/native_audio.rs#L701) |
| `GetTimeLength` | Get Time Length | Handled | — | [native_audio.rs:701](../../crates/crysirror-system4/src/native_audio.rs#L701) |
| `GetUnuseChannel` | Get Unuse Channel | Handled | — | [native_audio.rs:710](../../crates/crysirror-system4/src/native_audio.rs#L710) |
| `GetVolume` | Get Volume | Handled | — | [native_audio.rs:701](../../crates/crysirror-system4/src/native_audio.rs#L701) |
| `IsExist` | Is Exist | Handled | — | [native_audio.rs:701](../../crates/crysirror-system4/src/native_audio.rs#L701) |
| `IsFade` | Is Fade | Handled | — | [native_audio.rs:701](../../crates/crysirror-system4/src/native_audio.rs#L701) |
| `IsPlay` | Is Play | Handled | — | [native_audio.rs:701](../../crates/crysirror-system4/src/native_audio.rs#L701) |
| `MillisecondsToSamples` | Milliseconds To Samples | Handled | — | [native_audio.rs:704](../../crates/crysirror-system4/src/native_audio.rs#L704) |
| `Play` | Play | Handled | — | [native_audio.rs:701](../../crates/crysirror-system4/src/native_audio.rs#L701) |
| `Prepare` | Prepare | Handled | — | [native_audio.rs:704](../../crates/crysirror-system4/src/native_audio.rs#L704) |
| `ReverseLR` | Reverse LR | Handled | — | [native_audio.rs:701](../../crates/crysirror-system4/src/native_audio.rs#L701) |
| `SetLoopCount` | Set Loop Count | Handled | — | [native_audio.rs:704](../../crates/crysirror-system4/src/native_audio.rs#L704) |
| `SetLoopEndPos` | Set Loop End Pos | Handled | — | [native_audio.rs:704](../../crates/crysirror-system4/src/native_audio.rs#L704) |
| `SetLoopStartPos` | Set Loop Start Pos | Handled | — | [native_audio.rs:704](../../crates/crysirror-system4/src/native_audio.rs#L704) |
| `Stop` | Stop | Handled | — | [native_audio.rs:701](../../crates/crysirror-system4/src/native_audio.rs#L701) |
| `StopFade` | Stop Fade | Handled | — | [native_audio.rs:701](../../crates/crysirror-system4/src/native_audio.rs#L701) |
| `Unprepare` | Unprepare | Handled | — | [native_audio.rs:701](../../crates/crysirror-system4/src/native_audio.rs#L701) |

## Native exports / native_audio/analysis

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `(Music_GetFormat, [Argument::Scalar { kind: 10, bits: number, }, rest @ ..])` | Music Get Format | Handled | — | [analysis.rs:145](../../crates/crysirror-system4/src/native_audio/analysis.rs#L145) |
| `(Music_GetSampleData, [Argument::Scalar { kind: 10, bits: number, }, Argument::HeapReference { kind: 22, slot }, Argument::Scalar { kind: 10, bits: start, }])` | Music Get Sample Data | Handled | — | [analysis.rs:131](../../crates/crysirror-system4/src/native_audio/analysis.rs#L131) |

## Native exports / native_cg

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `(CG_GetMetrics, [ Argument::Scalar { kind: 10, bits: id }, Argument::HeapReference { kind: 21, slot }, ])` | CG Get Metrics | Handled | — | [native_cg.rs:360](../../crates/crysirror-system4/src/native_cg.rs#L360) |
| `(CG_IsExist, [Argument::Scalar { kind: 10, bits: id }])` | CG Is Exist | Handled | — | [native_cg.rs:351](../../crates/crysirror-system4/src/native_cg.rs#L351) |
| `(SP_SetCG, [ Argument::Scalar { kind: 10, bits: number, }, Argument::Scalar { kind: 10, bits: id }, ])` | SP Set CG | Handled | — | [native_cg.rs:377](../../crates/crysirror-system4/src/native_cg.rs#L377) |
| `(SP_SetCG, [ Argument::Scalar { kind: 10, bits: number, }, Argument::Scalar { kind: 10, bits: id }, ])` | SP Set CG | Handled | — | [native_cg.rs:98](../../crates/crysirror-system4/src/native_cg.rs#L98) |
| `(SP_SetCGFromFile, [ Argument::Scalar { kind: 10, bits: number, }, Argument::String { bytes, .. }, ])` | SP Set CGFrom File | Handled | — | [native_cg.rs:401](../../crates/crysirror-system4/src/native_cg.rs#L401) |
| `(SP_SetCGFromFile, [ Argument::Scalar { kind: 10, bits: number, }, Argument::String { .. }, ])` | SP Set CGFrom File | Handled | — | [native_cg.rs:113](../../crates/crysirror-system4/src/native_cg.rs#L113) |

## Native exports / native_confirm

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `(Init, 0, [Argument::Scalar { kind: 26, .. }])` | Init | Handled | — | [native_confirm.rs:164](../../crates/crysirror-system4/src/native_confirm.rs#L164) |
| `CheckProtectFile` | Check Protect File | Handled | — | [native_confirm.rs:174](../../crates/crysirror-system4/src/native_confirm.rs#L174) |
| `CreateKeyFile` | Create Key File | Handled | — | [native_confirm.rs:173](../../crates/crysirror-system4/src/native_confirm.rs#L173) |
| `ExistKeyFile` | Exist Key File | Handled | — | [native_confirm.rs:169](../../crates/crysirror-system4/src/native_confirm.rs#L169) |

## Native exports / native_csv

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `(CSV_CountColumns, [])` | CSV Count Columns | Handled | — | [native_csv.rs:160](../../crates/crysirror-system4/src/native_csv.rs#L160) |
| `(CSV_CountLines, [])` | CSV Count Lines | Handled | — | [native_csv.rs:160](../../crates/crysirror-system4/src/native_csv.rs#L160) |
| `(CSV_Get, [ Argument::HeapReference { slot, .. }, Argument::Scalar { bits: row, .. }, Argument::Scalar { bits: column, .. }, ])` | CSV Get | Handled | — | [native_csv.rs:232](../../crates/crysirror-system4/src/native_csv.rs#L232) |
| `(CSV_Get, [ Argument::HeapReference { kind: 20, slot }, Argument::Scalar { kind: 10, .. }, Argument::Scalar { kind: 10, .. }, ])` | CSV Get | Handled | — | [native_csv.rs:161](../../crates/crysirror-system4/src/native_csv.rs#L161) |
| `(CSV_GetInt, [ Argument::Scalar { bits: row, .. }, Argument::Scalar { bits: column, .. }, ])` | CSV Get Int | Handled | — | [native_csv.rs:244](../../crates/crysirror-system4/src/native_csv.rs#L244) |
| `(CSV_GetInt, [ Argument::Scalar { kind: 10, .. }, Argument::Scalar { kind: 10, .. }, ])` | CSV Get Int | Handled | — | [native_csv.rs:180](../../crates/crysirror-system4/src/native_csv.rs#L180) |
| `(CSV_Load, [Argument::String { .. }])` | CSV Load | Handled | — | [native_csv.rs:159](../../crates/crysirror-system4/src/native_csv.rs#L159) |
| `(CSV_Load, _)` | CSV Load | Handled | — | [native_csv.rs:296](../../crates/crysirror-system4/src/native_csv.rs#L296) |
| `(CSV_Realloc, [ Argument::Scalar { bits: rows, .. }, Argument::Scalar { bits: columns, .. }, ])` | CSV Realloc | Handled | — | [native_csv.rs:283](../../crates/crysirror-system4/src/native_csv.rs#L283) |
| `(CSV_Realloc, [ Argument::Scalar { kind: 10, .. }, Argument::Scalar { kind: 10, .. }, ])` | CSV Realloc | Handled | — | [native_csv.rs:195](../../crates/crysirror-system4/src/native_csv.rs#L195) |
| `(CSV_Save, [])` | CSV Save | Handled | — | [native_csv.rs:160](../../crates/crysirror-system4/src/native_csv.rs#L160) |
| `(CSV_Save, _)` | CSV Save | Handled | — | [native_csv.rs:332](../../crates/crysirror-system4/src/native_csv.rs#L332) |
| `(CSV_SaveAs, [Argument::String { .. }])` | CSV Save As | Handled | — | [native_csv.rs:159](../../crates/crysirror-system4/src/native_csv.rs#L159) |
| `(CSV_SaveAs, _)` | CSV Save As | Handled | — | [native_csv.rs:332](../../crates/crysirror-system4/src/native_csv.rs#L332) |
| `(CSV_Set, [ Argument::Scalar { bits: row, .. }, Argument::Scalar { bits: column, .. }, Argument::String { bytes, .. }, ])` | CSV Set | Handled | — | [native_csv.rs:254](../../crates/crysirror-system4/src/native_csv.rs#L254) |
| `(CSV_Set, [ Argument::Scalar { kind: 10, .. }, Argument::Scalar { kind: 10, .. }, Argument::String { .. }, ])` | CSV Set | Handled | — | [native_csv.rs:172](../../crates/crysirror-system4/src/native_csv.rs#L172) |
| `(CSV_SetInt, [ Argument::Scalar { bits: row, .. }, Argument::Scalar { bits: column, .. }, Argument::Scalar { bits: value, .. }, ])` | CSV Set Int | Handled | — | [native_csv.rs:267](../../crates/crysirror-system4/src/native_csv.rs#L267) |
| `(CSV_SetInt, [ Argument::Scalar { kind: 10, .. }, Argument::Scalar { kind: 10, .. }, Argument::Scalar { kind: 10, .. }, ])` | CSV Set Int | Handled | — | [native_csv.rs:187](../../crates/crysirror-system4/src/native_csv.rs#L187) |

## Native exports / native_draw

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `(Copy, [dst, dx, dy, src, sx, sy, w, h])` | Copy | Handled | — | [native_draw.rs:333](../../crates/crysirror-system4/src/native_draw.rs#L333) |
| `(CopyAMap, [dst, dx, dy, src, sx, sy, w, h])` | Copy AMap | Handled | — | [native_draw.rs:333](../../crates/crysirror-system4/src/native_draw.rs#L333) |
| `(CopyWithAlphaMap, [dst, dx, dy, src, sx, sy, w, h])` | Copy With Alpha Map | Handled | — | [native_draw.rs:333](../../crates/crysirror-system4/src/native_draw.rs#L333) |
| `(Fill, [dst, x, y, w, h, r, g, b])` | Fill | Handled | — | [native_draw.rs:357](../../crates/crysirror-system4/src/native_draw.rs#L357) |
| `(FillAMap, [dst, x, y, w, h, a])` | Fill AMap | Handled | — | [native_draw.rs:366](../../crates/crysirror-system4/src/native_draw.rs#L366) |
| `(FillWithAlpha, [dst, x, y, w, h, r, g, b, a])` | Fill With Alpha | Handled | — | [native_draw.rs:375](../../crates/crysirror-system4/src/native_draw.rs#L375) |
| `Copy` | Copy | Handled | — | [native_draw.rs:342](../../crates/crysirror-system4/src/native_draw.rs#L342) |
| `CopyAMap` | Copy AMap | Handled | — | [native_draw.rs:343](../../crates/crysirror-system4/src/native_draw.rs#L343) |
| `IsLoad` | Is Load | Handled | — | [native_draw.rs:129](../../crates/crysirror-system4/src/native_draw.rs#L129) |
| `Load` | Load | Partial | unsupported System4 native Graph/DrawGraph module arithmetic profile | [native_draw.rs:131](../../crates/crysirror-system4/src/native_draw.rs#L131) |

## Native exports / native_draw/font_state

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `(GetFontColor, refs)` | Get Font Color | Handled | Guard: refs.len() == 3 | [font_state.rs:123](../../crates/crysirror-system4/src/native_draw/font_state.rs#L123) |
| `(GetFontName, [])` | Get Font Name | Handled | — | [font_state.rs:122](../../crates/crysirror-system4/src/native_draw/font_state.rs#L122) |
| `(GetFontSize, [])` | Get Font Size | Handled | — | [font_state.rs:117](../../crates/crysirror-system4/src/native_draw/font_state.rs#L117) |
| `(GetFontSpace, [])` | Get Font Space | Handled | — | [font_state.rs:119](../../crates/crysirror-system4/src/native_draw/font_state.rs#L119) |
| `(GetFontStrikeOut, [])` | Get Font Strike Out | Handled | — | [font_state.rs:121](../../crates/crysirror-system4/src/native_draw/font_state.rs#L121) |
| `(GetFontUnderline, [])` | Get Font Underline | Handled | — | [font_state.rs:120](../../crates/crysirror-system4/src/native_draw/font_state.rs#L120) |
| `(GetFontWeight, [])` | Get Font Weight | Handled | — | [font_state.rs:118](../../crates/crysirror-system4/src/native_draw/font_state.rs#L118) |
| `(SetFontColor, [Argument::Scalar { kind: 10, bits: r }, Argument::Scalar { kind: 10, bits: g }, Argument::Scalar { kind: 10, bits: b }])` | Set Font Color | Handled | — | [font_state.rs:106](../../crates/crysirror-system4/src/native_draw/font_state.rs#L106) |
| `(SetFontName, [Argument::String { bytes, .. }])` | Set Font Name | Handled | — | [font_state.rs:100](../../crates/crysirror-system4/src/native_draw/font_state.rs#L100) |
| `(SetFontSize, [Argument::Scalar { kind: 10, bits }])` | Set Font Size | Handled | — | [font_state.rs:72](../../crates/crysirror-system4/src/native_draw/font_state.rs#L72) |
| `(SetFontSpace, [Argument::Scalar { kind: 10, bits }])` | Set Font Space | Handled | — | [font_state.rs:80](../../crates/crysirror-system4/src/native_draw/font_state.rs#L80) |
| `(SetFontStrikeOut, [Argument::Scalar { kind: 10, bits }])` | Set Font Strike Out | Handled | — | [font_state.rs:92](../../crates/crysirror-system4/src/native_draw/font_state.rs#L92) |
| `(SetFontUnderline, [Argument::Scalar { kind: 10, bits }])` | Set Font Underline | Handled | — | [font_state.rs:84](../../crates/crysirror-system4/src/native_draw/font_state.rs#L84) |
| `(SetFontWeight, [Argument::Scalar { kind: 10, bits }])` | Set Font Weight | Handled | — | [font_state.rs:76](../../crates/crysirror-system4/src/native_draw/font_state.rs#L76) |

## Native exports / native_draw/geometry

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `CopyRotZoomAMap` | Copy Rot Zoom AMap | Handled | — | [geometry.rs:279](../../crates/crysirror-system4/src/native_draw/geometry.rs#L279) |
| `CopyRotZoomUseAMap` | Copy Rot Zoom Use AMap | Handled | — | [geometry.rs:280](../../crates/crysirror-system4/src/native_draw/geometry.rs#L280) |

## Native exports / native_draw/normalized

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `(AddDA_DAxSA, [])` | Add DA DAx SA | Handled | — | [normalized.rs:177](../../crates/crysirror-system4/src/native_draw/normalized.rs#L177) |
| `(Blend, [alpha])` | Blend | Handled | — | [normalized.rs:115](../../crates/crysirror-system4/src/native_draw/normalized.rs#L115) |
| `(BlendAMap, [])` | Blend AMap | Handled | — | [normalized.rs:126](../../crates/crysirror-system4/src/native_draw/normalized.rs#L126) |
| `(BlendAMapAlpha, [alpha])` | Blend AMap Alpha | Handled | — | [normalized.rs:140](../../crates/crysirror-system4/src/native_draw/normalized.rs#L140) |
| `(BlendAMapAlphaSrcBright, [alpha, rate])` | Blend AMap Alpha Src Bright | Handled | — | [normalized.rs:149](../../crates/crysirror-system4/src/native_draw/normalized.rs#L149) |
| `(BlendAMapBright, [rate])` | Blend AMap Bright | Handled | — | [normalized.rs:144](../../crates/crysirror-system4/src/native_draw/normalized.rs#L144) |
| `(BlendAMapSrcOnly, [])` | Blend AMap Src Only | Handled | — | [normalized.rs:129](../../crates/crysirror-system4/src/native_draw/normalized.rs#L129) |
| `(BlendAddSatur, [])` | Blend Add Satur | Handled | — | [normalized.rs:125](../../crates/crysirror-system4/src/native_draw/normalized.rs#L125) |
| `(BlendDA_DAxSA, [])` | Blend DA DAx SA | Handled | — | [normalized.rs:181](../../crates/crysirror-system4/src/native_draw/normalized.rs#L181) |
| `(BlendMultiply, [])` | Blend Multiply | Handled | — | [normalized.rs:165](../../crates/crysirror-system4/src/native_draw/normalized.rs#L165) |
| `(BlendScreen, [])` | Blend Screen | Handled | — | [normalized.rs:164](../../crates/crysirror-system4/src/native_draw/normalized.rs#L164) |
| `(BlendScreenAlpha, [alpha])` | Blend Screen Alpha | Handled | — | [normalized.rs:166](../../crates/crysirror-system4/src/native_draw/normalized.rs#L166) |
| `(BlendSrcBright, [alpha, rate])` | Blend Src Bright | Handled | — | [normalized.rs:119](../../crates/crysirror-system4/src/native_draw/normalized.rs#L119) |
| `(BlendUseAMapColor, [r, g, b, threshold])` | Blend Use AMap Color | Handled | — | [normalized.rs:154](../../crates/crysirror-system4/src/native_draw/normalized.rs#L154) |
| `(BrightDestOnly, [rate])` | Bright Dest Only | Handled | — | [normalized.rs:243](../../crates/crysirror-system4/src/native_draw/normalized.rs#L243) |
| `(CopyAMapMax, [])` | Copy AMap Max | Handled | — | [normalized.rs:107](../../crates/crysirror-system4/src/native_draw/normalized.rs#L107) |
| `(CopyAMapMin, [])` | Copy AMap Min | Handled | — | [normalized.rs:111](../../crates/crysirror-system4/src/native_draw/normalized.rs#L111) |
| `(CopyBright, [rate])` | Copy Bright | Handled | — | [normalized.rs:93](../../crates/crysirror-system4/src/native_draw/normalized.rs#L93) |
| `(CopyColorReverse, [])` | Copy Color Reverse | Handled | — | [normalized.rs:94](../../crates/crysirror-system4/src/native_draw/normalized.rs#L94) |
| `(CopySprite, [r, g, b])` | Copy Sprite | Handled | — | [normalized.rs:98](../../crates/crysirror-system4/src/native_draw/normalized.rs#L98) |
| `(CopyUseAMapBorder, [threshold])` | Copy Use AMap Border | Handled | — | [normalized.rs:106](../../crates/crysirror-system4/src/native_draw/normalized.rs#L106) |
| `(CopyUseAMapUnder, [threshold])` | Copy Use AMap Under | Handled | — | [normalized.rs:105](../../crates/crysirror-system4/src/native_draw/normalized.rs#L105) |
| `(FillAlphaColor, [r, g, b, a])` | Fill Alpha Color | Handled | — | [normalized.rs:219](../../crates/crysirror-system4/src/native_draw/normalized.rs#L219) |
| `(SaturDP_DPxSA, [])` | Satur DP DPx SA | Handled | — | [normalized.rs:171](../../crates/crysirror-system4/src/native_draw/normalized.rs#L171) |
| `(ScreenDA_DAxSA, [])` | Screen DA DAx SA | Handled | — | [normalized.rs:185](../../crates/crysirror-system4/src/native_draw/normalized.rs#L185) |
| `(SpriteCopyAMap, [key])` | Sprite Copy AMap | Handled | — | [normalized.rs:101](../../crates/crysirror-system4/src/native_draw/normalized.rs#L101) |
| `(SubDA_DAxSA, [])` | Sub DA DAx SA | Handled | — | [normalized.rs:189](../../crates/crysirror-system4/src/native_draw/normalized.rs#L189) |

## Native exports / native_file

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `(Close, [])` | Close | Handled | — | [native_file.rs:300](../../crates/crysirror-system4/src/native_file.rs#L300) |
| `(Copy, [ Argument::String { bytes: a, .. }, Argument::String { bytes: b, .. }, ])` | Copy | Handled | — | [native_file.rs:303](../../crates/crysirror-system4/src/native_file.rs#L303) |
| `(Delete, [Argument::String { bytes, .. }])` | Delete | Handled | — | [native_file.rs:310](../../crates/crysirror-system4/src/native_file.rs#L310) |
| `(GetFind, [Argument::HeapReference { kind: 20, slot }])` | Get Find | Handled | — | [native_file.rs:337](../../crates/crysirror-system4/src/native_file.rs#L337) |
| `(GetTime, [ Argument::String { bytes, .. }, Argument::HeapReference { kind: 21, slot }, ])` | Get Time | Handled | — | [native_file.rs:311](../../crates/crysirror-system4/src/native_file.rs#L311) |
| `(MakeDirectory, [Argument::String { bytes, .. }])` | Make Directory | Handled | — | [native_file.rs:340](../../crates/crysirror-system4/src/native_file.rs#L340) |
| `(Open, [ Argument::String { bytes, .. }, Argument::Scalar { kind: 10, bits }, ])` | Open | Handled | — | [native_file.rs:286](../../crates/crysirror-system4/src/native_file.rs#L286) |
| `(Read, [Argument::HeapReference { kind: 21, slot }])` | Read | Handled | — | [native_file.rs:301](../../crates/crysirror-system4/src/native_file.rs#L301) |
| `(SetFind, [Argument::String { bytes, .. }])` | Set Find | Handled | — | [native_file.rs:318](../../crates/crysirror-system4/src/native_file.rs#L318) |
| `(Write, [Argument::Page { kind: 13, slot }])` | Write | Handled | — | [native_file.rs:302](../../crates/crysirror-system4/src/native_file.rs#L302) |
| `Close` | Close | Handled | — | [native_file.rs:210](../../crates/crysirror-system4/src/native_file.rs#L210) |
| `Copy` | Copy | Handled | — | [native_file.rs:213](../../crates/crysirror-system4/src/native_file.rs#L213) |
| `Delete` | Delete | Handled | — | [native_file.rs:217](../../crates/crysirror-system4/src/native_file.rs#L217) |
| `GetFind` | Get Find | Handled | — | [native_file.rs:227](../../crates/crysirror-system4/src/native_file.rs#L227) |
| `GetTime` | Get Time | Handled | — | [native_file.rs:220](../../crates/crysirror-system4/src/native_file.rs#L220) |
| `MakeDirectory` | Make Directory | Handled | — | [native_file.rs:217](../../crates/crysirror-system4/src/native_file.rs#L217) |
| `Open` | Open | Handled | — | [native_file.rs:206](../../crates/crysirror-system4/src/native_file.rs#L206) |
| `Read` | Read | Handled | — | [native_file.rs:211](../../crates/crysirror-system4/src/native_file.rs#L211) |
| `SetFind` | Set Find | Handled | — | [native_file.rs:217](../../crates/crysirror-system4/src/native_file.rs#L217) |
| `Write` | Write | Handled | — | [native_file.rs:212](../../crates/crysirror-system4/src/native_file.rs#L212) |

## Native exports / native_input_string

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `(Begin, [])` | Begin | Handled | — | [native_input_string.rs:209](../../crates/crysirror-system4/src/native_input_string.rs#L209) |
| `(CloseIME, [])` | Close IME | Handled | — | [native_input_string.rs:212](../../crates/crysirror-system4/src/native_input_string.rs#L212) |
| `(End, [])` | End | Handled | — | [native_input_string.rs:210](../../crates/crysirror-system4/src/native_input_string.rs#L210) |
| `(OpenIME, [])` | Open IME | Handled | — | [native_input_string.rs:211](../../crates/crysirror-system4/src/native_input_string.rs#L211) |
| `(SetFont, [Argument::Scalar { bits: size, .. }, Argument::String { bytes, .. }, Argument::Scalar { bits: weight, .. }])` | Set Font | Handled | — | [native_input_string.rs:189](../../crates/crysirror-system4/src/native_input_string.rs#L189) |
| `(SetPos, [Argument::Scalar { bits: x, .. }, Argument::Scalar { bits: y, .. }])` | Set Pos | Handled | — | [native_input_string.rs:206](../../crates/crysirror-system4/src/native_input_string.rs#L206) |

## Native exports / native_logo

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `(Init, [] \| [Argument::Scalar { kind: 26, .. }])` | Init0x10007310 is cdecl with no parameters. Rance7's AIN declares an IMainSystem(26) argument; the DLL never reads that… | Handled | — | [native_logo.rs:210](../../crates/crysirror-system4/src/native_logo.rs#L210) |
| `(Run, [ Argument::Scalar { kind: 10, bits: season, }, Argument::Scalar { kind: 10, bits: repeat, }, ])` | Run | Handled | — | [native_logo.rs:243](../../crates/crysirror-system4/src/native_logo.rs#L243) |
| `(SetWaveNum, [ Argument::Scalar { kind: 10, bits: index, }, Argument::Scalar { kind: 10, bits: wave, }, ])` | Set Wave Num | Handled | — | [native_logo.rs:183](../../crates/crysirror-system4/src/native_logo.rs#L183) |

## Native exports / native_logs

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `(MsgLogManager, b"AddInt", [Argument::Scalar { kind: 10, .. }, Argument::Scalar { kind: 10, .. }])` | Msg Log Manager | Handled | — | [native_logs.rs:186](../../crates/crysirror-system4/src/native_logs.rs#L186) |
| `(MsgLogManager, b"AddString", [Argument::Scalar { kind: 10, .. }, Argument::String { .. }])` | Msg Log Manager | Handled | — | [native_logs.rs:191](../../crates/crysirror-system4/src/native_logs.rs#L191) |
| `(MsgLogManager, b"GetString", [Argument::Scalar { kind: 10, .. }, Argument::HeapReference { kind: 20, slot }])` | Msg Log Manager | Handled | — | [native_logs.rs:196](../../crates/crysirror-system4/src/native_logs.rs#L196) |
| `(MsgLogManager, b"GetType" \| b"GetInt", [Argument::Scalar { kind: 10, .. }])` | Msg Log Manager | Handled | — | [native_logs.rs:184](../../crates/crysirror-system4/src/native_logs.rs#L184) |
| `(MsgLogManager, b"Numof" \| b"GetInterface", [])` | Msg Log Manager | Handled | — | [native_logs.rs:183](../../crates/crysirror-system4/src/native_logs.rs#L183) |
| `(MsgLogManager, b"Save" \| b"Load", [Argument::String { .. }])` | Msg Log Manager | Handled | — | [native_logs.rs:185](../../crates/crysirror-system4/src/native_logs.rs#L185) |
| `(MsgLogManager, b"SetLineMax", [Argument::Scalar { kind: 10, .. }])` | Msg Log Manager | Handled | — | [native_logs.rs:204](../../crates/crysirror-system4/src/native_logs.rs#L204) |
| `(MsgLogViewer, b"Add", [Argument::Page { kind: 13, .. }])` | Msg Log Viewer | Handled | — | [native_logs.rs:207](../../crates/crysirror-system4/src/native_logs.rs#L207) |
| `(MsgLogViewer, b"Clear", [])` | Msg Log Viewer | Handled | — | [native_logs.rs:206](../../crates/crysirror-system4/src/native_logs.rs#L206) |
| `(MsgLogViewer, b"ConvertFromMsgLogManager", [Argument::Scalar { kind: 10, .. }])` | Msg Log Viewer | Handled | — | [native_logs.rs:213](../../crates/crysirror-system4/src/native_logs.rs#L213) |
| `(MsgLogViewer, b"Get", [Argument::Scalar { kind: 10, .. }, Argument::HeapReference { kind: 21, .. }])` | Msg Log Viewer | Handled | — | [native_logs.rs:208](../../crates/crysirror-system4/src/native_logs.rs#L208) |
| `(MsgLogViewer, b"Numof", [])` | Msg Log Viewer | Handled | — | [native_logs.rs:205](../../crates/crysirror-system4/src/native_logs.rs#L205) |
| `(b"MsgLogManager", AddInt, [Argument::Scalar { bits: tag, .. }, Argument::Scalar { bits: value, .. }])` | Add Int | Handled | — | [native_logs.rs:264](../../crates/crysirror-system4/src/native_logs.rs#L264) |
| `(b"MsgLogManager", AddString, [Argument::Scalar { bits: tag, .. }, Argument::String { bytes, .. }])` | Add String | Handled | — | [native_logs.rs:278](../../crates/crysirror-system4/src/native_logs.rs#L278) |
| `(b"MsgLogManager", GetInt, [Argument::Scalar { bits, .. }])` | Get Int | Handled | — | [native_logs.rs:299](../../crates/crysirror-system4/src/native_logs.rs#L299) |
| `(b"MsgLogManager", GetInterface, [])` | Get Interface | Handled | — | [native_logs.rs:263](../../crates/crysirror-system4/src/native_logs.rs#L263) |
| `(b"MsgLogManager", GetString, [Argument::Scalar { bits, .. }, Argument::HeapReference { slot, .. }])` | Get String | Handled | — | [native_logs.rs:305](../../crates/crysirror-system4/src/native_logs.rs#L305) |
| `(b"MsgLogManager", GetType, [Argument::Scalar { bits, .. }])` | Get Type | Handled | — | [native_logs.rs:296](../../crates/crysirror-system4/src/native_logs.rs#L296) |
| `(b"MsgLogManager", Load, _)` | Load | Handled | — | [native_logs.rs:335](../../crates/crysirror-system4/src/native_logs.rs#L335) |
| `(b"MsgLogManager", Numof, [])` | Numof | Handled | — | [native_logs.rs:262](../../crates/crysirror-system4/src/native_logs.rs#L262) |
| `(b"MsgLogManager", Save, _)` | Save | Handled | — | [native_logs.rs:316](../../crates/crysirror-system4/src/native_logs.rs#L316) |
| `(b"MsgLogManager", SetLineMax, [Argument::Scalar { bits, .. }])` | Set Line Max | Handled | — | [native_logs.rs:292](../../crates/crysirror-system4/src/native_logs.rs#L292) |
| `(b"MsgLogViewer", Add, [Argument::Page { slot, .. }])` | Add | Handled | — | [native_logs.rs:365](../../crates/crysirror-system4/src/native_logs.rs#L365) |
| `(b"MsgLogViewer", Clear, [])` | Clear | Handled | — | [native_logs.rs:361](../../crates/crysirror-system4/src/native_logs.rs#L361) |
| `(b"MsgLogViewer", ConvertFromMsgLogManager, [Argument::Scalar { bits: interface, .. }])` | Convert From Msg Log Manager | Handled | — | [native_logs.rs:387](../../crates/crysirror-system4/src/native_logs.rs#L387) |
| `(b"MsgLogViewer", Get, [Argument::Scalar { bits, .. }, Argument::HeapReference { slot, .. }])` | Get | Handled | — | [native_logs.rs:370](../../crates/crysirror-system4/src/native_logs.rs#L370) |
| `(b"MsgLogViewer", Numof, [])` | Numof | Handled | — | [native_logs.rs:360](../../crates/crysirror-system4/src/native_logs.rs#L360) |

## Native exports / native_movie

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `Init` | Init | Handled | — | [native_movie.rs:171](../../crates/crysirror-system4/src/native_movie.rs#L171) |
| `IsPlay` | Is Play | Handled | — | [native_movie.rs:302](../../crates/crysirror-system4/src/native_movie.rs#L302) |
| `Load` | Load | Handled | — | [native_movie.rs:183](../../crates/crysirror-system4/src/native_movie.rs#L183) |
| `Play` | Play | Handled | — | [native_movie.rs:254](../../crates/crysirror-system4/src/native_movie.rs#L254) |
| `Release` | Release | Handled | — | [native_movie.rs:323](../../crates/crysirror-system4/src/native_movie.rs#L323) |
| `Stop` | Stop | Handled | — | [native_movie.rs:292](../../crates/crysirror-system4/src/native_movie.rs#L292) |

## Native exports / native_msgskip

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `GetEnable` | Get Enable | Handled | — | [native_msgskip.rs:261](../../crates/crysirror-system4/src/native_msgskip.rs#L261) |
| `GetFlag` | Get Flag | Handled | — | [native_msgskip.rs:245](../../crates/crysirror-system4/src/native_msgskip.rs#L245) |
| `GetNumofFlag` | Get Numof Flag | Handled | — | [native_msgskip.rs:255](../../crates/crysirror-system4/src/native_msgskip.rs#L255) |
| `GetNumofMsg` | Get Numof Msg | Handled | — | [native_msgskip.rs:263](../../crates/crysirror-system4/src/native_msgskip.rs#L263) |
| `GetState` | Get State | Handled | — | [native_msgskip.rs:262](../../crates/crysirror-system4/src/native_msgskip.rs#L262) |
| `SetEnable` | Set Enable | Handled | — | [native_msgskip.rs:235](../../crates/crysirror-system4/src/native_msgskip.rs#L235) |
| `SetFlag` | Set Flag | Handled | — | [native_msgskip.rs:223](../../crates/crysirror-system4/src/native_msgskip.rs#L223) |
| `SetState` | Set State | Handled | — | [native_msgskip.rs:236](../../crates/crysirror-system4/src/native_msgskip.rs#L236) |
| `UseFlag` | Use Flag | Handled | — | [native_msgskip.rs:231](../../crates/crysirror-system4/src/native_msgskip.rs#L231) |

## Native exports / native_sact_text

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `(SP_SetBrightness, _, Some([_, value]))` | SP Set Brightness | Handled | — | [native_sact_text.rs:497](../../crates/crysirror-system4/src/native_sact_text.rs#L497) |
| `(SP_SetTextPos, _, Some([_, x, y]))` | SP Set Text Pos | Handled | — | [native_sact_text.rs:481](../../crates/crysirror-system4/src/native_sact_text.rs#L481) |
| `(SP_SetZ, _, Some([_, value]))` | SP Set Z | Handled | — | [native_sact_text.rs:485](../../crates/crysirror-system4/src/native_sact_text.rs#L485) |
| `(SP_TextBackSpace, _, Some([_]))` | SP Text Back Space | Handled | — | [native_sact_text.rs:381](../../crates/crysirror-system4/src/native_sact_text.rs#L381) |
| `(SP_TextClear, _, Some([_]))` | SP Text Clear | Handled | — | [native_sact_text.rs:369](../../crates/crysirror-system4/src/native_sact_text.rs#L369) |
| `(SP_TextCopy, _, Some([_, source]))` | SP Text Copy | Handled | — | [native_sact_text.rs:316](../../crates/crysirror-system4/src/native_sact_text.rs#L316) |
| `(SP_TextDraw, [ Argument::Scalar { .. }, Argument::String { bytes, .. }, Argument::Page { kind: 13, slot }, ], _)` | SP Text Draw | Handled | — | [native_sact_text.rs:268](../../crates/crysirror-system4/src/native_sact_text.rs#L268) |
| `(SP_TextHome, _, Some([_, size]))` | SP Text Home | Handled | — | [native_sact_text.rs:454](../../crates/crysirror-system4/src/native_sact_text.rs#L454) |
| `(SP_TextNewLine, _, Some([_, size]))` | SP Text New Line | Handled | — | [native_sact_text.rs:463](../../crates/crysirror-system4/src/native_sact_text.rs#L463) |
| `(SP_TextScroll, _, Some([_, x, y]))` | SP Text Scroll | Handled | — | [native_sact_text.rs:408](../../crates/crysirror-system4/src/native_sact_text.rs#L408) |

## Native exports / native_scene

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `(SetWP, [0])` | Set WP | Handled | — | [native_scene.rs:219](../../crates/crysirror-system4/src/native_scene.rs#L219) |
| `(SetWP, [number])` | Set WP | Handled | — | [native_scene.rs:224](../../crates/crysirror-system4/src/native_scene.rs#L224) |
| `(SetWP_Color, [r, g, b])` | Set WP Color | Handled | — | [native_scene.rs:207](../../crates/crysirror-system4/src/native_scene.rs#L207) |
| `(WP_SetSP, [number])` | WP Set SP | Handled | — | [native_scene.rs:246](../../crates/crysirror-system4/src/native_scene.rs#L246) |

## Native exports / native_system

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `(true, System_GetDate)` | System Get Date | Handled | — | [native_system.rs:156](../../crates/crysirror-system4/src/native_system.rs#L156) |
| `(true, System_GetTime)` | System Get Time | Handled | — | [native_system.rs:157](../../crates/crysirror-system4/src/native_system.rs#L157) |
| `ChangeFullScreen` | Change Full Screen | Handled | — | [native_system.rs:314](../../crates/crysirror-system4/src/native_system.rs#L314) |
| `ChangeNormalScreen` | Change Normal Screen | Handled | — | [native_system.rs:314](../../crates/crysirror-system4/src/native_system.rs#L314) |
| `GetDate` | Get Date | Handled | — | [native_system.rs:435](../../crates/crysirror-system4/src/native_system.rs#L435) |
| `GetGameName` | Get Game Name | Handled | — | [native_system.rs:285](../../crates/crysirror-system4/src/native_system.rs#L285) |
| `GetGameVersion` | Get Game Version | Handled | — | [native_system.rs:281](../../crates/crysirror-system4/src/native_system.rs#L281) |
| `GetMouseCursorConfig` | Get Mouse Cursor Config | Handled | — | [native_system.rs:412](../../crates/crysirror-system4/src/native_system.rs#L412) |
| `GetTime` | Get Time | Handled | — | [native_system.rs:435](../../crates/crysirror-system4/src/native_system.rs#L435) |
| `GetViewHeight` | Get View Height | Handled | — | [native_system.rs:296](../../crates/crysirror-system4/src/native_system.rs#L296) |
| `GetViewWidth` | Get View Width | Handled | — | [native_system.rs:296](../../crates/crysirror-system4/src/native_system.rs#L296) |
| `IsExistPlayingManual` | Is Exist Playing Manual | Handled | — | [native_system.rs:522](../../crates/crysirror-system4/src/native_system.rs#L522) |
| `IsFullScreen` | Is Full Screen | Handled | — | [native_system.rs:307](../../crates/crysirror-system4/src/native_system.rs#L307) |
| `IsResetOnce` | Is Reset Once | Handled | — | [native_system.rs:234](../../crates/crysirror-system4/src/native_system.rs#L234) |
| `MoveMouseCursorPosImmediately` | Move Mouse Cursor Pos Immediately | Handled | — | [native_system.rs:361](../../crates/crysirror-system4/src/native_system.rs#L361) |
| `OpenPlayingManual` | Open Playing Manual | Handled | — | [native_system.rs:522](../../crates/crysirror-system4/src/native_system.rs#L522) |
| `RestrainScreensaver` | Restrain Screensaver | Handled | — | [native_system.rs:239](../../crates/crysirror-system4/src/native_system.rs#L239) |
| `SetHideMouseCursorByGame` | Set Hide Mouse Cursor By Game | Handled | — | [native_system.rs:372](../../crates/crysirror-system4/src/native_system.rs#L372) |
| `SetMouseCursorConfig` | Set Mouse Cursor Config | Handled | — | [native_system.rs:391](../../crates/crysirror-system4/src/native_system.rs#L391) |

## Native exports / native_system/misc

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `AddURLMenu` | Add URLMenu | Handled | — | [misc.rs:181](../../crates/crysirror-system4/src/native_system/misc.rs#L181) |
| `Debug_GetFuncStack` | Debug Get Func Stack | Handled | — | [misc.rs:164](../../crates/crysirror-system4/src/native_system/misc.rs#L164) |
| `Error` | Error | Handled | — | [misc.rs:153](../../crates/crysirror-system4/src/native_system/misc.rs#L153) |

## Native exports / sprites

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `(SP_Count, [])` | SP Count | Handled | — | [sprites.rs:302](../../crates/crysirror-system4/src/sprites.rs#L302) |
| `(SP_Create, [n, w, h, r, g, b, a])` | SP Create | Handled | — | [sprites.rs:321](../../crates/crysirror-system4/src/sprites.rs#L321) |
| `(SP_CreateCustom, [n])` | SP Create Custom | Handled | — | [sprites.rs:335](../../crates/crysirror-system4/src/sprites.rs#L335) |
| `(SP_CreatePixelOnly, [n, w, h])` | SP Create Pixel Only | Handled | — | [sprites.rs:328](../../crates/crysirror-system4/src/sprites.rs#L328) |
| `(SP_Delete, [n])` | SP Delete | Handled | — | [sprites.rs:336](../../crates/crysirror-system4/src/sprites.rs#L336) |
| `(SP_ExistAlpha, [n])` | SP Exist Alpha | Handled | — | [sprites.rs:305](../../crates/crysirror-system4/src/sprites.rs#L305) |
| `(SP_GetBlendRate, [n])` | SP Get Blend Rate | Handled | — | [sprites.rs:311](../../crates/crysirror-system4/src/sprites.rs#L311) |
| `(SP_GetBrightness, [n])` | SP Get Brightness | Handled | — | [sprites.rs:313](../../crates/crysirror-system4/src/sprites.rs#L313) |
| `(SP_GetDrawMethod, [n])` | SP Get Draw Method | Handled | — | [sprites.rs:419](../../crates/crysirror-system4/src/sprites.rs#L419) |
| `(SP_GetHeight, [n])` | SP Get Height | Handled | — | [sprites.rs:309](../../crates/crysirror-system4/src/sprites.rs#L309) |
| `(SP_GetMaxZ, [])` | SP Get Max Z | Handled | — | [sprites.rs:303](../../crates/crysirror-system4/src/sprites.rs#L303) |
| `(SP_GetPosX, [n])` | SP Get Pos X | Handled | — | [sprites.rs:306](../../crates/crysirror-system4/src/sprites.rs#L306) |
| `(SP_GetPosY, [n])` | SP Get Pos Y | Handled | — | [sprites.rs:307](../../crates/crysirror-system4/src/sprites.rs#L307) |
| `(SP_GetShow, [n])` | SP Get Show | Handled | — | [sprites.rs:312](../../crates/crysirror-system4/src/sprites.rs#L312) |
| `(SP_GetTextCharSpace, [n])` | SP Get Text Char Space | Handled | — | [sprites.rs:318](../../crates/crysirror-system4/src/sprites.rs#L318) |
| `(SP_GetTextHomeX, [n])` | SP Get Text Home X | Handled | — | [sprites.rs:314](../../crates/crysirror-system4/src/sprites.rs#L314) |
| `(SP_GetTextHomeY, [n])` | SP Get Text Home Y | Handled | — | [sprites.rs:315](../../crates/crysirror-system4/src/sprites.rs#L315) |
| `(SP_GetTextLineSpace, [n])` | SP Get Text Line Space | Handled | — | [sprites.rs:319](../../crates/crysirror-system4/src/sprites.rs#L319) |
| `(SP_GetTextPosX, [n])` | SP Get Text Pos X | Handled | — | [sprites.rs:316](../../crates/crysirror-system4/src/sprites.rs#L316) |
| `(SP_GetTextPosY, [n])` | SP Get Text Pos Y | Handled | — | [sprites.rs:317](../../crates/crysirror-system4/src/sprites.rs#L317) |
| `(SP_GetUnuseNum, [min])` | SP Get Unuse Num | Handled | — | [sprites.rs:320](../../crates/crysirror-system4/src/sprites.rs#L320) |
| `(SP_GetWidth, [n])` | SP Get Width | Handled | — | [sprites.rs:308](../../crates/crysirror-system4/src/sprites.rs#L308) |
| `(SP_GetZ, [n])` | SP Get Z | Handled | — | [sprites.rs:310](../../crates/crysirror-system4/src/sprites.rs#L310) |
| `(SP_IsUsing, [n])` | SP Is Using | Handled | — | [sprites.rs:304](../../crates/crysirror-system4/src/sprites.rs#L304) |
| `(SP_SetBlendRate, [n, a])` | SP Set Blend Rate | Handled | — | [sprites.rs:353](../../crates/crysirror-system4/src/sprites.rs#L353) |
| `(SP_SetBrightness, [n, b])` | SP Set Brightness | Handled | — | [sprites.rs:372](../../crates/crysirror-system4/src/sprites.rs#L372) |
| `(SP_SetDrawMethod, [n, method])` | SP Set Draw Method | Handled | — | [sprites.rs:420](../../crates/crysirror-system4/src/sprites.rs#L420) |
| `(SP_SetPos, [n, x, y])` | SP Set Pos | Handled | — | [sprites.rs:337](../../crates/crysirror-system4/src/sprites.rs#L337) |
| `(SP_SetShow, [n, show])` | SP Set Show | Handled | — | [sprites.rs:360](../../crates/crysirror-system4/src/sprites.rs#L360) |
| `(SP_SetTextCharSpace, [n, px])` | SP Set Text Char Space | Handled | — | [sprites.rs:390](../../crates/crysirror-system4/src/sprites.rs#L390) |
| `(SP_SetTextHome, [n, x, y])` | SP Set Text Home | Handled | — | [sprites.rs:379](../../crates/crysirror-system4/src/sprites.rs#L379) |
| `(SP_SetTextLineSpace, [n, px])` | SP Set Text Line Space | Handled | — | [sprites.rs:394](../../crates/crysirror-system4/src/sprites.rs#L394) |
| `(SP_SetTextPos, [n, x, y])` | SP Set Text Pos | Handled | — | [sprites.rs:383](../../crates/crysirror-system4/src/sprites.rs#L383) |
| `(SP_SetX, [n, x])` | SP Set X | Handled | — | [sprites.rs:341](../../crates/crysirror-system4/src/sprites.rs#L341) |
| `(SP_SetY, [n, y])` | SP Set Y | Handled | — | [sprites.rs:345](../../crates/crysirror-system4/src/sprites.rs#L345) |
| `(SP_SetZ, [n, z])` | SP Set Z | Handled | — | [sprites.rs:349](../../crates/crysirror-system4/src/sprites.rs#L349) |
| `(SP_TextHome, [n, size])` | SP Text Home | Handled | — | [sprites.rs:398](../../crates/crysirror-system4/src/sprites.rs#L398) |
| `(SP_TextNewLine, [n, size])` | SP Text New Line | Handled | — | [sprites.rs:403](../../crates/crysirror-system4/src/sprites.rs#L403) |

## Subselectors and conditional dispatch

Patterns retain their source namespace and guards. `_` is the fallback of that table. Boolean/optional results follow the function's return contract. Call-site names identify the implementation to inspect, including indirect service calls.

### adapter.rs / Bridge::dispatch_scoped

```rust
fn dispatch_scoped(
        &mut self,
        m: &mut Machine,
        call: &ExternalCall,
        host: &mut dyn HostServices,
        client: Option<crysirror_core::state::ClientId>,
    ) -> Result<Dispatch>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under call.target=ExternalTarget::Hll { library, function }` | `l.name == b"SACT2" && args.is_empty() && f.return_kind == 10` | i32::try_from, host.now_ms, HllReturn::Scalar | Handled | [86](../../crates/crysirror-system4/src/adapter.rs#L86) |
| `f.name.as_slice(); under call.target=ExternalTarget::Hll { library, function }` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [106](../../crates/crysirror-system4/src/adapter.rs#L106) |
| `condition; under call.target=ExternalTarget::Hll { library, function }` | `l.name == b"SACT2" && f.name == b"Init"` | HllReturn::Scalar | Handled | [113](../../crates/crysirror-system4/src/adapter.rs#L113) |
| `condition; under call.target=ExternalTarget::Hll { library, function }` | `l.name == b"SACT2"` | HllReturn::Scalar, self.sact.as_mut, host.input_observation, crate::input::Input::validate_scope, self.input.clone | Partial: unsupported no-joypad SACT2 {} ABI / unsupported native SACT2 input ABI | [129](../../crates/crysirror-system4/src/adapter.rs#L129) |
| `condition; under call.target=ExternalTarget::Hll { library, function }` | `f.name.starts_with(b"Joypad")` | HllReturn::Scalar | Partial: unsupported no-joypad SACT2 {} ABI | [131](../../crates/crysirror-system4/src/adapter.rs#L131) |
| `(f.name.as_slice(), f.return_kind, args.as_slice()); under call.target=ExternalTarget::Hll { library, function }` | `_` | Local state/value operations | Unsupported: unsupported no-joypad SACT2 {} ABI | [146](../../crates/crysirror-system4/src/adapter.rs#L146) |
| `condition; under call.target=ExternalTarget::Hll { library, function }` | `f.name == b"Update" && scalars.is_empty()` | Local state/value operations | Unsupported: System4 Update requires the retained native scene presentation actor | [160](../../crates/crysirror-system4/src/adapter.rs#L160) |
| `condition; under call.target=ExternalTarget::Hll { library, function }` | `matches!( f.name.as_slice(), b"Key_IsDown" \| b"Key_ClearFlag" \| b"Key_ClearFlagNoCtrl" \| b"Key_ClearFlagOne" \| b"Mouse_GetPos" \| b"Mouse_GetWheel" \| b"Mouse_ClearWheel" )` | host.input_observation, crate::input::Input::validate_scope, self.input.clone, HllReturn::Scalar, crate::input::Input::pointer | Partial: unsupported native SACT2 input ABI | [175](../../crates/crysirror-system4/src/adapter.rs#L175) |
| `condition; under call.target=ExternalTarget::Hll { library, function }` | `!self.buffered_input \|\| f.name == b"Mouse_GetPos"` | host.input_observation, crate::input::Input::validate_scope | Handled | [185](../../crates/crysirror-system4/src/adapter.rs#L185) |
| `(f.name.as_slice(), f.return_kind, args.as_slice()); under call.target=ExternalTarget::Hll { library, function }` | `_` | Local state/value operations | Unsupported: unsupported native SACT2 input ABI | [253](../../crates/crysirror-system4/src/adapter.rs#L253) |
| `condition; under call.target=ExternalTarget::Hll { library, function }` | `l.name == b"OutputLog" && self.debug_log.is_some()` | HllReturn::Scalar | Partial: unsupported reviewed OutputLog function/ABI | [263](../../crates/crysirror-system4/src/adapter.rs#L263) |
| `(f.name.as_slice(), f.return_kind, args.as_slice()); under call.target=ExternalTarget::Hll { library, function }` | `_` | Local state/value operations | Unsupported: unsupported reviewed OutputLog function/ABI | [289](../../crates/crysirror-system4/src/adapter.rs#L289) |
| `call.target` | `ExternalTarget::Hll { library, function }` | std::env::var_os, i32::try_from, host.now_ms, HllReturn::Scalar, self.sact.as_mut | Unsupported: unsupported original System4 HLL {}.{} pc={:#x}; external call retained | [78](../../crates/crysirror-system4/src/adapter.rs#L78) |
| `call.target` | `ExternalTarget::System(6)` | host.append_log | Handled | [302](../../crates/crysirror-system4/src/adapter.rs#L302) |
| `call.target` | `ExternalTarget::System(0x0d)` | host.now_ms | Handled | [316](../../crates/crysirror-system4/src/adapter.rs#L316) |
| `call.target` | `ExternalTarget::System(0x15)` | host.now_ms, Dispatch::WaitUntil | Partial: System4 negative Sleep unsupported | [322](../../crates/crysirror-system4/src/adapter.rs#L322) |
| `call.target` | `_` | Local state/value operations | Unsupported: unsupported original System4 syscall {:?} pc={:#x}; external call retained | [344](../../crates/crysirror-system4/src/adapter.rs#L344) |

### ain.rs / Program::parse

```rust
fn parse(input: &[u8]) -> Result<Self>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under &tag=b"GSET"` | `kind == 12` | Local state/value operations | Handled | [283](../../crates/crysirror-system4/src/ain.rs#L283) |

### bootstrap.rs / tokens

```rust
fn tokens(text: &str) -> Result<Vec<Token>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `name` | `_` | Token::Word | Handled | [244](../../crates/crysirror-system4/src/bootstrap.rs#L244) |

### bootstrap.rs / Bootstrap::from_document

```rust
fn from_document(doc: &Document) -> Result<Self>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `entry.name.as_str()` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [364](../../crates/crysirror-system4/src/bootstrap.rs#L364) |

### bootstrap.rs / Parser::entries

```rust
fn entries(&mut self, depth: usize) -> Result<Vec<Entry>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `name == "Formation"` | self.value, self.need, self.entries, Value::Formation | Handled | [104](../../crates/crysirror-system4/src/bootstrap.rs#L104) |

### input.rs / Input::clear_one

```rust
fn clear_one(&mut self, code: i32) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!(0..256).contains(&code)` | Local state/value operations | Handled | [71](../../crates/crysirror-system4/src/input.rs#L71) |

### input.rs / Input::key

```rust
fn key(&self, code: i32) -> Result<i32>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!(0..256).contains(&code)` | Local state/value operations | Handled | [57](../../crates/crysirror-system4/src/input.rs#L57) |

### math.rs / scalar

```rust
fn scalar(
    m: &mut Machine,
    name: &[u8],
    kind: i32,
    args: &[Argument],
) -> Result<Option<HllReturn>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `name == b"BezierCurve"` | HllReturn::Scalar, crysirror_image::progress_curve::bernstein_i32_points, f32::from_bits | Partial: native Bezier factorial-overflow degree is unsupported | [17](../../crates/crysirror-system4/src/math.rs#L17) |
| `condition` | `kind == 10` | HllReturn::Scalar | Handled | [97](../../crates/crysirror-system4/src/math.rs#L97) |
| `(name, values.as_slice()); under receiver=kind == 10` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [117](../../crates/crysirror-system4/src/math.rs#L117) |
| `condition; under receiver=kind == 10` | `let (b"Ceil", [x]) = (name, floats().unwrap_or_default().as_slice())` | HllReturn::Scalar | Handled | [121](../../crates/crysirror-system4/src/math.rs#L121) |
| `condition` | `kind == 11` | HllReturn::Scalar | Handled | [132](../../crates/crysirror-system4/src/math.rs#L132) |
| `(name, values.as_slice()); under receiver=kind == 11` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [162](../../crates/crysirror-system4/src/math.rs#L162) |
| `condition` | `matches!(name, b"Swap" \| b"SwapF")` | Local state/value operations | Handled | [167](../../crates/crysirror-system4/src/math.rs#L167) |
| `condition` | `matches!(name, b"Swap" \| b"SwapF")` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [167](../../crates/crysirror-system4/src/math.rs#L167) |
| `condition` | `name == b"Swap"` | Local state/value operations | Handled | [169](../../crates/crysirror-system4/src/math.rs#L169) |

### math_random.rs / Random::dispatch_array_checked

```rust
fn dispatch_array_checked(
        &mut self,
        name: &[u8],
        kind: i32,
        args: &[Argument],
        heap: &mut crate::vm::heap::Heap,
        check: &(dyn Fn() -> Result<()> + Sync),
    ) -> Result<Option<HllReturn>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!matches!(name, b"RandTable2Init" \| b"RandTable2")` | Local state/value operations | Handled | [73](../../crates/crysirror-system4/src/math_random.rs#L73) |
| `condition` | `matches!(name, b"RandTable2Init" \| b"RandTable2")` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [73](../../crates/crysirror-system4/src/math_random.rs#L73) |
| `(name, kind, args)` | `_` | Local state/value operations | Handled | [87](../../crates/crysirror-system4/src/math_random.rs#L87) |
| `condition` | `!(0..=256).contains(&id)` | HllReturn::Scalar | Handled | [89](../../crates/crysirror-system4/src/math_random.rs#L89) |

### math_random.rs / Random::dispatch_checked

```rust
fn dispatch_checked(
        &mut self,
        name: &[u8],
        kind: i32,
        args: &[Argument],
        check: &(dyn Fn() -> Result<()> + Sync),
    ) -> Result<Option<HllReturn>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under name=b"Rand"` | `self.mode == 1` | f64::from, self.mt.next_u32 | Handled | [191](../../crates/crysirror-system4/src/math_random.rs#L191) |
| `condition; under name=b"RandF"` | `self.mode == 1` | f64::from, self.mt.next_u32 | Handled | [204](../../crates/crysirror-system4/src/math_random.rs#L204) |
| `condition; under name=b"RandTableInit"` | `!(0..=256).contains(&id)` | Local state/value operations | Handled | [221](../../crates/crysirror-system4/src/math_random.rs#L221) |
| `condition; under name=b"RandTable"` | `!(0..=256).contains(&id)` | HllReturn::Scalar | Handled | [246](../../crates/crysirror-system4/src/math_random.rs#L246) |
| `name` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [257](../../crates/crysirror-system4/src/math_random.rs#L257) |

### native_async.rs / NativeAsync::dispatch

```rust
fn dispatch(
        &mut self,
        owner: ClientId,
        machine: &mut Machine,
        call: &ExternalCall,
        host: &mut dyn HostServices,
    ) -> Result<Option<Dispatch>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `library.name != b"Math" \|\| function.name != b"SetSeedByCurrentTime"` | Local state/value operations | Handled | [44](../../crates/crysirror-system4/src/native_async.rs#L44) |

### native_audio.rs / NativeAudio::configure

```rust
fn configure(
        &mut self,
        bootstrap: &Bootstrap,
        default_volume: i32,
        metadata: AudioMetadata,
    ) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `groups[0].name == b"Master" \|\| groups[0].name == b"\x83\x7d\x83\x58\x83\x5e\x81\x5b"` | Local state/value operations | Handled | [207](../../crates/crysirror-system4/src/native_audio.rs#L207) |
| `condition` | `group.name == b"Voice" \|\| group.name == b"\x89\xb9\x90\xba"` | Local state/value operations | Handled | [229](../../crates/crysirror-system4/src/native_audio.rs#L229) |

### native_audio.rs / NativeAudio::dispatch

```rust
fn dispatch(
        &mut self,
        owner: ClientId,
        machine: &mut Machine,
        call: &ExternalCall,
        bridge: &Bridge,
        host: &mut dyn HostServices,
    ) -> Result<Option<Dispatch>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `library.name != b"SACT2" \|\| !(function.name.starts_with(b"Music_") \|\| function.name.starts_with(b"Sound_"))` | Local state/value operations | Handled | [640](../../crates/crysirror-system4/src/native_audio.rs#L640) |
| `condition` | `function.name == b"Sound_PrepareFromFile"` | self.prepare_file | Handled | [657](../../crates/crysirror-system4/src/native_audio.rs#L657) |
| `condition` | `function.name.starts_with(b"Music_")` | Local state/value operations | Partial: unsupported original Music export {} | [672](../../crates/crysirror-system4/src/native_audio.rs#L672) |
| `condition` | `let Some(name) = function.name.strip_prefix(b"Music_")` | Local state/value operations | Handled | [717](../../crates/crysirror-system4/src/native_audio.rs#L717) |
| `condition` | `name == b"MillisecondsToSamples"` | i64::from, HllReturn::Scalar | Handled | [722](../../crates/crysirror-system4/src/native_audio.rs#L722) |
| `condition` | `name == b"IsExist"` | HllReturn::Scalar, i32::from | Handled | [730](../../crates/crysirror-system4/src/native_audio.rs#L730) |
| `condition` | `name == b"GetUnuseChannel"` | self.channels.contains_key, HllReturn::Scalar | Handled | [748](../../crates/crysirror-system4/src/native_audio.rs#L748) |
| `condition` | `name == b"Prepare"` | HllReturn::Scalar, self.retire, host.begin_audio_resource, VfsPath::new, Dispatch::WaitUntil | Handled | [764](../../crates/crysirror-system4/src/native_audio.rs#L764) |
| `condition` | `*id < 0` | HllReturn::Scalar | Handled | [768](../../crates/crysirror-system4/src/native_audio.rs#L768) |
| `condition` | `kind == Kind::Music && self.metadata.reviewed_music.is_some() && matches!( name, b"SetLoopCount" \| b"SetLoopStartPos" \| b"SetLoopEndPos" )` | HllReturn::Scalar | Partial: System4 native optional music setter requires an unimplemented empty resource declaration | [812](../../crates/crysirror-system4/src/native_audio.rs#L812) |
| `condition` | `name == b"Unprepare" && values.len() == 1` | self.retire, HllReturn::Scalar, i32::from | Handled | [832](../../crates/crysirror-system4/src/native_audio.rs#L832) |
| `condition` | `number < 0 \|\| !self.channels.contains_key(&key)` | HllReturn::Scalar | Partial: unsupported original SACT audio export {} | [841](../../crates/crysirror-system4/src/native_audio.rs#L841) |
| `(name, values.as_slice())` | `_` | Local state/value operations | Partial: unsupported original SACT audio export/ABI {} | [1013](../../crates/crysirror-system4/src/native_audio.rs#L1013) |

### native_audio.rs / NativeAudio::dispatch_mixer

```rust
fn dispatch_mixer(
        &mut self,
        owner: ClientId,
        machine: &mut Machine,
        call: &ExternalCall,
        host: &mut dyn HostServices,
    ) -> Result<Option<Dispatch>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `library.name != b"SystemService"` | Local state/value operations | Handled | [355](../../crates/crysirror-system4/src/native_audio.rs#L355) |
| `condition` | `!matches!( name, b"GetMixerNumof" \| b"GetMixerName" \| b"GetMixerVolume" \| b"GetMixerDefaultVolume" \| b"GetMixerMute" \| b"SetMixerName" \| b"SetMixerVolume" \| b"SetMixerMute" )` | Local state/value operations | Handled | [360](../../crates/crysirror-system4/src/native_audio.rs#L360) |
| `condition` | `matches!( name, b"GetMixerNumof" \| b"GetMixerName" \| b"GetMixerVolume" \| b"GetMixerDefaultVolume" \| b"GetMixerMute" \| b"SetMixerName" \| b"SetMixerVolume" \| b"SetMixerMute" )` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [360](../../crates/crysirror-system4/src/native_audio.rs#L360) |
| `condition` | `name == b"GetMixerNumof"` | HllReturn::Scalar | Handled | [383](../../crates/crysirror-system4/src/native_audio.rs#L383) |
| `(name, &args[1])` | `_` | Local state/value operations | Handled | [412](../../crates/crysirror-system4/src/native_audio.rs#L412) |
| `condition; under (name, &args[1])=( b"GetMixerVolume" \| b"GetMixerMute", Argument::Reference { page, index: field, .. }, )` | `name == b"GetMixerVolume"` | Local state/value operations | Handled | [446](../../crates/crysirror-system4/src/native_audio.rs#L446) |
| `condition; under (name, &args[1])=(b"SetMixerVolume" \| b"SetMixerMute", Argument::Scalar { bits, .. })` | `name == b"SetMixerVolume"` | Local state/value operations | Handled | [461](../../crates/crysirror-system4/src/native_audio.rs#L461) |
| `condition; under (name, &args[1])=(b"SetMixerVolume" \| b"SetMixerMute", Argument::Scalar { bits, .. })` | `name == b"SetMixerMute"` | Local state/value operations | Handled | [466](../../crates/crysirror-system4/src/native_audio.rs#L466) |
| `(name, &args[1])` | `_` | Local state/value operations | Handled | [486](../../crates/crysirror-system4/src/native_audio.rs#L486) |

### native_audio.rs / NativeAudio::dispatch_reviewed_unavailable

```rust
fn dispatch_reviewed_unavailable(
        &mut self,
        owner: ClientId,
        machine: &mut Machine,
        call: &ExternalCall,
        bridge: &Bridge,
        host: &mut dyn HostServices,
    ) -> Result<Option<Dispatch>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `library.name != b"SACT2" \|\| !matches!( function.name.as_slice(), b"Music_Pause" \| b"Music_Restart" \| b"Music_IsPause" \| b"Music_GetLoopCount" \| b"Music_GetSamplePos" \| b"Music_GetSampleLength" \| b"Music_Seek" )` | Local state/value operations | Handled | [286](../../crates/crysirror-system4/src/native_audio.rs#L286) |
| `condition` | `self.metadata.reviewed_music.is_none() && matches!( function.name.as_slice(), b"Music_GetLoopCount" \| b"Music_GetSamplePos" \| b"Music_GetSampleLength" )` | Local state/value operations | Handled | [300](../../crates/crysirror-system4/src/native_audio.rs#L300) |
| `condition` | `function.name == b"Music_Seek"` | Local state/value operations | Handled | [317](../../crates/crysirror-system4/src/native_audio.rs#L317) |

### native_audio.rs / NativeAudio::new_channel

```rust
fn new_channel(
        &self,
        key: Key,
        number: i32,
        resource: AudioResourceHandle,
        info: AudioResourceInfo,
    ) -> Result<Channel>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `number < 0 \|\| key.kind == Kind::Music` | Local state/value operations | Handled | [514](../../crates/crysirror-system4/src/native_audio.rs#L514) |

### native_audio/analysis.rs / NativeAudio::dispatch_analysis

```rust
fn dispatch_analysis(
        &mut self,
        owner: ClientId,
        m: &mut Machine,
        call: &ExternalCall,
        bridge: &Bridge,
        host: &mut dyn HostServices,
    ) -> Result<Option<Dispatch>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `lib.name != b"SACT2" \|\| !matches!( name, b"FFT_rdft" \| b"FFT_hanning_window" \| b"Music_AnalyzeSampleData" \| b"Music_GetSampleData" \| b"Music_GetFormat" )` | Local state/value operations | Handled | [36](../../crates/crysirror-system4/src/native_audio/analysis.rs#L36) |
| `condition` | `matches!( name, b"FFT_rdft" \| b"FFT_hanning_window" \| b"Music_AnalyzeSampleData" \| b"Music_GetSampleData" \| b"Music_GetFormat" )` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [37](../../crates/crysirror-system4/src/native_audio/analysis.rs#L37) |
| `condition` | `matches!(name, b"FFT_rdft" \| b"FFT_hanning_window")` | f32::from_bits, crysirror_audio::analysis::real_spectrum | Handled | [57](../../crates/crysirror-system4/src/native_audio/analysis.rs#L57) |
| `condition` | `matches!(name, b"FFT_rdft" \| b"FFT_hanning_window")` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [57](../../crates/crysirror-system4/src/native_audio/analysis.rs#L57) |
| `condition` | `name == b"FFT_rdft"` | crysirror_audio::analysis::real_spectrum | Handled | [69](../../crates/crysirror-system4/src/native_audio/analysis.rs#L69) |
| `condition` | `name == b"Music_AnalyzeSampleData"` | HllReturn::Scalar, crysirror_audio::analysis::split_pcm_words | Handled | [83](../../crates/crysirror-system4/src/native_audio/analysis.rs#L83) |
| `(name, args.as_slice())` | `_` | Local state/value operations | Handled | [166](../../crates/crysirror-system4/src/native_audio/analysis.rs#L166) |
| `condition` | `name == b"Music_GetFormat"` | i32::from | Handled | [207](../../crates/crysirror-system4/src/native_audio/analysis.rs#L207) |

### native_audio/errors.rs / NativeAudio::configure_errors

```rust
fn configure_errors(
        &mut self,
        owner: ClientId,
        m: &mut Machine,
        call: &ExternalCall,
    ) -> Result<Option<Dispatch>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `f.name != b"MusicSystem" \|\| function.name != b"SetErrorOutput"` | Local state/value operations | Handled | [23](../../crates/crysirror-system4/src/native_audio/errors.rs#L23) |

### native_audio/lifecycle.rs / NativeAudio::prepare_file

```rust
fn prepare_file(
        &mut self,
        machine: &mut Machine,
        call: &ExternalCall,
        args: &[Argument],
        host: &mut dyn HostServices,
    ) -> Result<Dispatch>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `*id < 0` | HllReturn::Scalar | Handled | [344](../../crates/crysirror-system4/src/native_audio/lifecycle.rs#L344) |

### native_audio_profile.rs / load

```rust
fn load(
    document: &Document,
    storage: &dyn ClientStorage,
    cancel: &Cancellation,
) -> Result<Option<ReviewedMusic>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `format!("{:x}", Sha256::digest(&bytes)) != hash` | Local state/value operations | Handled | [52](../../crates/crysirror-system4/src/native_audio_profile.rs#L52) |

### native_audio_profile.rs / ReviewedMusic::unavailable

```rust
fn unavailable(self, name: &[u8]) -> bool
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under self=Self::Rance7Original` | `matches!( name, b"Music_Pause" \| b"Music_Restart" \| b"Music_IsPause" \| b"Music_GetLoopCount" \| b"Music_GetSamplePos" \| b"Music_GetSampleLength" \| b"Music_Seek" )` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [65](../../crates/crysirror-system4/src/native_audio_profile.rs#L65) |

### native_cg.rs / NativeCg::copy_sprite

```rust
fn copy_sprite(
        &mut self,
        owner: ClientId,
        m: &mut Machine,
        call: &ExternalCall,
        bridge: &mut Bridge,
        host: &mut dyn HostServices,
    ) -> Result<Option<Dispatch>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `library.name != b"SACT2" \|\| f.name != b"SP_CreateCopy"` | Local state/value operations | Handled | [645](../../crates/crysirror-system4/src/native_cg.rs#L645) |

### native_cg.rs / NativeCg::dispatch_inner

```rust
fn dispatch_inner(
        &mut self,
        owner: ClientId,
        machine: &mut Machine,
        call: &ExternalCall,
        bridge: &mut Bridge,
        files: Option<&crate::native_files::NativeFiles>,
        host: &mut dyn HostServices,
    ) -> Result<Option<Dispatch>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `library.name != b"SACT2" \|\| !matches!( function.name.as_slice(), b"CG_GetMetrics" \| b"CG_IsExist" \| b"SP_SetCG" \| b"SP_SetCGFromFile" )` | Local state/value operations | Handled | [337](../../crates/crysirror-system4/src/native_cg.rs#L337) |
| `condition; under (function.name.as_slice(), arguments.as_slice())=( b"SP_SetCG", [ Argument::Scalar { kind: 10, bits: number, }, Argument::Scalar { kind: 10, bits: id }, ], )` | `*number < 1 \|\| *id < 1` | HllReturn::Scalar | Handled | [387](../../crates/crysirror-system4/src/native_cg.rs#L387) |
| `(function.name.as_slice(), arguments.as_slice())` | `_` | Local state/value operations | Handled | [439](../../crates/crysirror-system4/src/native_cg.rs#L439) |

### native_cg.rs / NativeCg::sprite_clear_target

```rust
fn sprite_clear_target(
        &self,
        m: &Machine,
        call: &ExternalCall,
    ) -> Result<Option<i32>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `library.name != b"SACT2" \|\| !matches!(function.name.as_slice(), b"SP_SetCG" \| b"SP_SetCGFromFile")` | Local state/value operations | Handled | [90](../../crates/crysirror-system4/src/native_cg.rs#L90) |
| `condition; under (function.name.as_slice(), args.as_slice())=( b"SP_SetCG", [ Argument::Scalar { kind: 10, bits: number, }, Argument::Scalar { kind: 10, bits: id }, ], )` | `*number < 1 \|\| *id < 1` | Local state/value operations | Handled | [108](../../crates/crysirror-system4/src/native_cg.rs#L108) |
| `(function.name.as_slice(), args.as_slice())` | `_` | Local state/value operations | Handled | [123](../../crates/crysirror-system4/src/native_cg.rs#L123) |

### native_cg.rs / NativeCg::surface_replacement

```rust
fn surface_replacement(
        machine: &Machine,
        call: &ExternalCall,
    ) -> Result<Option<i32>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `library.name != b"SACT2" \|\| !matches!( library.functions[function].name.as_slice(), b"SP_Create" \| b"SP_CreatePixelOnly" \| b"SP_Delete" )` | Local state/value operations | Handled | [277](../../crates/crysirror-system4/src/native_cg.rs#L277) |

### native_cg/window.rs / NativeCg::install_wallpaper_snapshot

```rust
fn install_wallpaper_snapshot(
        &mut self,
        owner: ClientId,
        number: i32,
        raw: Arc<dyn ImageResource>,
        target: Arc<dyn ImageResource>,
        bridge: &mut Bridge,
        host: &mut dyn HostServices,
    ) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Err(error) = host.submit_engine( owner, 1, &serde_json::to_vec(&serde_json::json!({"layers":[command]}))?, )` | host.release_image_resource | Unsupported: Unconditional rejection | [192](../../crates/crysirror-system4/src/native_cg/window.rs#L192) |

### native_confirm.rs / NativeConfirm::dispatch

```rust
fn dispatch(
        &mut self,
        owner: ClientId,
        machine: &mut Machine,
        call: &ExternalCall,
        files: &NativeFiles,
        host: &mut dyn HostServices,
    ) -> Result<Option<Dispatch>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `library.name != b"Confirm2"` | Local state/value operations | Handled | [147](../../crates/crysirror-system4/src/native_confirm.rs#L147) |
| `name; under (function.name.as_slice(), function.return_kind, args.as_slice())=(name, 10, [Argument::String { bytes, .. }, Argument::Scalar { kind: 10, bits: key }])` | `_` | Local state/value operations | Unsupported: unsupported Confirm2 export {} | [185](../../crates/crysirror-system4/src/native_confirm.rs#L185) |
| `(function.name.as_slice(), function.return_kind, args.as_slice())` | `_` | Local state/value operations | Unsupported: unsupported Confirm2 function/ABI {} | [187](../../crates/crysirror-system4/src/native_confirm.rs#L187) |

### native_csv.rs / run

```rust
fn run(
    original: Machine,
    state: State,
    call: ExternalCall,
    name: Vec<u8>,
    args: Vec<Argument>,
    paths: Option<Paths>,
    filename: Option<Arc<Vec<u8>>>,
    cancel: Cancellation,
) -> Completed
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `(name.as_slice(), args.as_slice())` | `_` | Local state/value operations | Handled | [357](../../crates/crysirror-system4/src/native_csv.rs#L357) |

### native_csv.rs / signature

```rust
fn signature(m: &Machine, call: &ExternalCall) -> Result<(Vec<u8>, Vec<Argument>)>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `(f.name.as_slice(), args.as_slice())` | `_` | Local state/value operations | Partial: unsupported native CSV export/ABI {} | [202](../../crates/crysirror-system4/src/native_csv.rs#L202) |

### native_csv.rs / NativeCsv::dispatch

```rust
fn dispatch(
        &mut self,
        owner: ClientId,
        machine: &mut Option<Machine>,
        call: &ExternalCall,
        files: &NativeFiles,
        host: &mut dyn HostServices,
    ) -> Result<Option<Dispatch>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(name.as_slice(), b"CSV_Load" \| b"CSV_SaveAs")` | Local state/value operations | Handled | [108](../../crates/crysirror-system4/src/native_csv.rs#L108) |
| `condition` | `name == b"CSV_Save"` | Local state/value operations | Handled | [113](../../crates/crysirror-system4/src/native_csv.rs#L113) |

### native_dialog.rs / NativeDialog::dispatch

```rust
fn dispatch(
        &mut self,
        owner: ClientId,
        machine: &mut Machine,
        call: &ExternalCall,
        bridge: &Bridge,
        host: &mut dyn HostServices,
    ) -> Result<Option<Dispatch>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!matches!(code, 7 \| 18 \| 15)` | Local state/value operations | Handled | [74](../../crates/crysirror-system4/src/native_dialog.rs#L74) |
| `condition` | `code == 18` | Local state/value operations | Handled | [81](../../crates/crysirror-system4/src/native_dialog.rs#L81) |
| `code` | `7` | Local state/value operations | Handled | [98](../../crates/crysirror-system4/src/native_dialog.rs#L98) |
| `code` | `18` | Local state/value operations | Handled | [99](../../crates/crysirror-system4/src/native_dialog.rs#L99) |
| `code` | `_` | Local state/value operations | Handled | [100](../../crates/crysirror-system4/src/native_dialog.rs#L100) |
| `code` | `7` | Local state/value operations | Handled | [103](../../crates/crysirror-system4/src/native_dialog.rs#L103) |
| `code` | `18` | Local state/value operations | Handled | [104](../../crates/crysirror-system4/src/native_dialog.rs#L104) |
| `code` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [105](../../crates/crysirror-system4/src/native_dialog.rs#L105) |
| `condition` | `code == 15` | host.append_log | Handled | [107](../../crates/crysirror-system4/src/native_dialog.rs#L107) |
| `condition` | `code == 15` | Local state/value operations | Handled | [110](../../crates/crysirror-system4/src/native_dialog.rs#L110) |
| `condition` | `code == 7` | Local state/value operations | Handled | [147](../../crates/crysirror-system4/src/native_dialog.rs#L147) |
| `(code, choice)` | `(7, _) \| (15, 0)` | Local state/value operations | Handled | [157](../../crates/crysirror-system4/src/native_dialog.rs#L157) |
| `(code, choice)` | `(18, _)` | Local state/value operations | Handled | [158](../../crates/crysirror-system4/src/native_dialog.rs#L158) |
| `(code, choice)` | `(15, 1)` | Local state/value operations | Handled | [159](../../crates/crysirror-system4/src/native_dialog.rs#L159) |
| `(code, choice)` | `_` | Local state/value operations | Unsupported: System4 unsupported modal completion | [160](../../crates/crysirror-system4/src/native_dialog.rs#L160) |

### native_draw.rs / NativeDraw::dispatch

```rust
fn dispatch(
        &mut self,
        owner: ClientId,
        machine: &mut Machine,
        call: &ExternalCall,
        bridge: &mut Bridge,
        cg: &mut NativeCg,
        host: &mut dyn HostServices,
    ) -> Result<Option<Dispatch>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `library.name != b"DrawPluginManager" && library.name != b"DrawGraph" && !sact_cg` | Local state/value operations | Handled | [102](../../crates/crysirror-system4/src/native_draw.rs#L102) |
| `condition` | `library.name == b"DrawPluginManager"` | i32::from, self.names.contains, host.image_metadata, self.names.insert, HllReturn::Scalar | Partial: unsupported System4 native Graph/DrawGraph module arithmetic profile / unsupported System4 DrawPluginManager export {} | [114](../../crates/crysirror-system4/src/native_draw.rs#L114) |
| `function.name.as_slice()` | `b"Load" if self.names.contains(name)` | Local state/value operations | Handled | [130](../../crates/crysirror-system4/src/native_draw.rs#L130) |
| `function.name.as_slice()` | `_` | Local state/value operations | Unsupported: unsupported System4 DrawPluginManager export {} | [158](../../crates/crysirror-system4/src/native_draw.rs#L158) |
| `condition` | `library.name != b"DrawGraph" && !sact_cg` | Local state/value operations | Handled | [166](../../crates/crysirror-system4/src/native_draw.rs#L166) |
| `condition` | `matches!( function.name.as_slice(), b"DrawTextToPMap" \| b"DrawTextToAMap" )` | self.text_run.is_none, host.font_library, native_text::Run::prepare, self.text_run.as_mut | Partial: unsupported original DrawText2 module coverage profile | [226](../../crates/crysirror-system4/src/native_draw.rs#L226) |
| `function.name.as_slice(); under (function.name.as_slice(), values.as_slice())=( b"Copy" \| b"CopyAMap" \| b"CopyWithAlphaMap", [dst, dx, dy, src, sx, sy, w, h], )` | `_` | Local state/value operations | Handled | [344](../../crates/crysirror-system4/src/native_draw.rs#L344) |
| `(function.name.as_slice(), values.as_slice())` | `_` | Local state/value operations | Unsupported: unsupported original DrawGraph export/ABI {} | [384](../../crates/crysirror-system4/src/native_draw.rs#L384) |

### native_draw/filtered_copy.rs / Run::begin

```rust
fn begin(
        owner: ClientId,
        call: &ExternalCall,
        name: &[u8],
        args: &[Argument],
        bridge: &Bridge,
        cg: &NativeCg,
        images: &dyn ImageMetadata,
    ) -> Result<Self>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!( name, b"CopyReverseAMapLR" \| b"CopyReverseAMapUD" \| b"CopyReverseLRWithAMap" \| b"CopyReverseUDWithAMap" )` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [70](../../crates/crysirror-system4/src/native_draw/filtered_copy.rs#L70) |
| `condition` | `matches!( name, b"CopyReverseLR" \| b"CopyReverseAMapLR" \| b"CopyReverseLRWithAMap" )` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [94](../../crates/crysirror-system4/src/native_draw/filtered_copy.rs#L94) |
| `condition` | `matches!(name, b"CopyReverseAMapLR" \| b"CopyReverseAMapUD")` | Local state/value operations | Handled | [102](../../crates/crysirror-system4/src/native_draw/filtered_copy.rs#L102) |
| `condition` | `matches!(name, b"CopyReverseAMapLR" \| b"CopyReverseAMapUD")` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [102](../../crates/crysirror-system4/src/native_draw/filtered_copy.rs#L102) |

### native_draw/filtered_copy.rs / Run::recognizes

```rust
fn recognizes(name: &[u8]) -> bool
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!( name, b"CopyReverseLR" \| b"CopyReverseUD" \| b"CopyReverseAMapLR" \| b"CopyReverseAMapUD" \| b"CopyReverseLRWithAMap" \| b"CopyReverseUDWithAMap" )` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [20](../../crates/crysirror-system4/src/native_draw/filtered_copy.rs#L20) |

### native_draw/filters.rs / operation

```rust
fn operation(
    owner: ClientId,
    name: &[u8],
    values: &[i32],
    bridge: &Bridge,
    cg: &NativeCg,
    native_tables: bool,
) -> Result<Option<(i32, ImageInput, bool)>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(name,b"FillOverlay"\|b"FillOverlayAlpha")` | crysirror_image::rgb_lut::RgbLookup::overlay_byte255 | Handled | [60](../../crates/crysirror-system4/src/native_draw/filters.rs#L60) |
| `condition` | `matches!(name,b"FillOverlay"\|b"FillOverlayAlpha")` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [60](../../crates/crysirror-system4/src/native_draw/filters.rs#L60) |
| `condition` | `name==b"FillOverlayAlpha"` | Local state/value operations | Handled | [68](../../crates/crysirror-system4/src/native_draw/filters.rs#L68) |
| `condition` | `!matches!( name, b"CopyWidthBlur" \| b"CopyHeightBlur" \| b"CopyAMapWidthBlur" \| b"CopyAMapHeightBlur" \| b"CopyGrayscale" \| b"FillAMapGradationUD" )` | Local state/value operations | Handled | [75](../../crates/crysirror-system4/src/native_draw/filters.rs#L75) |
| `condition` | `matches!( name, b"CopyWidthBlur" \| b"CopyHeightBlur" \| b"CopyAMapWidthBlur" \| b"CopyAMapHeightBlur" \| b"CopyGrayscale" \| b"FillAMapGradationUD" )` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [75](../../crates/crysirror-system4/src/native_draw/filters.rs#L75) |
| `condition` | `name == b"FillAMapGradationUD"` | Local state/value operations | Handled | [92](../../crates/crysirror-system4/src/native_draw/filters.rs#L92) |
| `condition` | `matches!(name, b"CopyAMapWidthBlur" \| b"CopyAMapHeightBlur")` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [170](../../crates/crysirror-system4/src/native_draw/filters.rs#L170) |
| `condition` | `matches!(name, b"CopyWidthBlur" \| b"CopyAMapWidthBlur")` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [179](../../crates/crysirror-system4/src/native_draw/filters.rs#L179) |

### native_draw/font_state.rs / FontState::dispatch

```rust
fn dispatch(
        &mut self,
        machine: &mut Machine,
        call: &ExternalCall,
        name: &[u8],
        return_kind: i32,
        args: &[Argument],
        native_profile: bool,
    ) -> Result<Option<Dispatch>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!( name, b"SetFontSize" \| b"SetFontName" \| b"SetFontWeight" \| b"SetFontUnderline" \| b"SetFontStrikeOut" \| b"SetFontSpace" \| b"SetFontColor" \| b"GetFontSize" \| b"GetFontName" \| b"GetFontWeight" \| b"GetFontUnderline" \| b"GetFontStrikeOut" \| b"GetFontSpace" \| b"GetFontColor" )` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [39](../../crates/crysirror-system4/src/native_draw/font_state.rs#L39) |
| `condition` | `getter && name != b"GetFontColor"` | Local state/value operations | Handled | [60](../../crates/crysirror-system4/src/native_draw/font_state.rs#L60) |
| `condition` | `name == b"GetFontName"` | Local state/value operations | Handled | [61](../../crates/crysirror-system4/src/native_draw/font_state.rs#L61) |
| `(name, args)` | `(b"GetFontColor", refs) if refs.len() == 3` | Vec::new, i32::from | Handled | [123](../../crates/crysirror-system4/src/native_draw/font_state.rs#L123) |
| `(name, args)` | `_` | Local state/value operations | Unsupported: System4 original font control argument ABI | [142](../../crates/crysirror-system4/src/native_draw/font_state.rs#L142) |

### native_draw/geometry.rs / operation

```rust
fn operation(
    owner: ClientId,
    name: &[u8],
    args: &[Argument],
    bridge: &Bridge,
    cg: &NativeCg,
    native_tables: bool,
) -> Result<Option<(i32, ImageInput, bool)>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!matches!( name, b"DrawLine" \| b"DrawLineToAMap" \| b"CopyTextureWrap" \| b"CopyTextureWrapAlpha" \| b"CopyRotZoom" \| b"CopyRotZoomAMap" \| b"CopyRotZoomUseAMap" \| b"CG_RotateRGB" \| b"CG_BlendAMapBin" \| b"CopyRotZoom2Bilinear" \| b"CopyRotateY" \| b"CopyRotateYUseAMap" \| b"CopyRotateYFixL" \| b"CopyRotateYFixR" \| b"CopyRotateYFixLUseAMap" \| b"CopyRotateYFixRUseAMap" \| b"CopyRotateX" \| b"CopyRotateXUseAMap" \| b"CopyRotateXFixU" \| b"CopyRotateXFixD" \| b"CopyRotateXFixUUseAMap" \| b"CopyRotateXFixDUseAMap" )` | Local state/value operations | Handled | [26](../../crates/crysirror-system4/src/native_draw/geometry.rs#L26) |
| `condition` | `matches!( name, b"DrawLine" \| b"DrawLineToAMap" \| b"CopyTextureWrap" \| b"CopyTextureWrapAlpha" \| b"CopyRotZoom" \| b"CopyRotZoomAMap" \| b"CopyRotZoomUseAMap" \| b"CG_RotateRGB" \| b"CG_BlendAMapBin" \| b"CopyRotZoom2Bilinear" \| b"CopyRotateY" \| b"CopyRotateYUseAMap" \| b"CopyRotateYFixL" \| b"CopyRotateYFixR" \| b"CopyRotateYFixLUseAMap" \| b"CopyRotateYFixRUseAMap" \| b"CopyRotateX" \| b"CopyRotateXUseAMap" \| b"CopyRotateXFixU" \| b"CopyRotateXFixD" \| b"CopyRotateXFixUUseAMap" \| b"CopyRotateXFixDUseAMap" )` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [26](../../crates/crysirror-system4/src/native_draw/geometry.rs#L26) |
| `condition` | `name.starts_with(b"CopyRotate")` | f32::from_bits | Partial: native axis overlapping source/output feedback profile unsupported / native axis clipped crop profile unsupported / native axis unequal output/crop e… | [57](../../crates/crysirror-system4/src/native_draw/geometry.rs#L57) |
| `condition` | `name.windows(4).any(\|n\| n == b"FixL" \|\| n == b"FixU")` | Local state/value operations | Handled | [131](../../crates/crysirror-system4/src/native_draw/geometry.rs#L131) |
| `condition` | `name.windows(4).any(\|n\| n == b"FixR" \|\| n == b"FixD")` | Local state/value operations | Handled | [133](../../crates/crysirror-system4/src/native_draw/geometry.rs#L133) |
| `condition` | `name == b"CopyRotZoom2Bilinear"` | f32::from_bits | Partial: native bilinear overlapping source/output profile unsupported | [154](../../crates/crysirror-system4/src/native_draw/geometry.rs#L154) |
| `condition` | `matches!( name, b"CopyRotZoom" \| b"CopyRotZoomAMap" \| b"CopyRotZoomUseAMap" )` | f32::from_bits | Partial: native rotate overlapping source/output profile unsupported / native rotate clipped source profile unsupported | [213](../../crates/crysirror-system4/src/native_draw/geometry.rs#L213) |
| `condition` | `matches!( name, b"CopyRotZoom" \| b"CopyRotZoomAMap" \| b"CopyRotZoomUseAMap" )` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [213](../../crates/crysirror-system4/src/native_draw/geometry.rs#L213) |
| `name` | `_` | Local state/value operations | Handled | [281](../../crates/crysirror-system4/src/native_draw/geometry.rs#L281) |
| `condition` | `name == b"CG_RotateRGB"` | Box::new, serde_json::to_vec | Handled | [303](../../crates/crysirror-system4/src/native_draw/geometry.rs#L303) |
| `condition` | `name == b"CG_BlendAMapBin"` | Local state/value operations | Partial: native alpha threshold overlapping feedback unsupported | [343](../../crates/crysirror-system4/src/native_draw/geometry.rs#L343) |
| `condition` | `matches!(name, b"DrawLine" \| b"DrawLineToAMap")` | Local state/value operations | Handled | [379](../../crates/crysirror-system4/src/native_draw/geometry.rs#L379) |
| `condition` | `matches!(name, b"DrawLine" \| b"DrawLineToAMap")` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [379](../../crates/crysirror-system4/src/native_draw/geometry.rs#L379) |

### native_draw/native_text.rs / Run::prepare

```rust
fn prepare(
        owner: ClientId,
        _machine: &Machine,
        call: &ExternalCall,
        args: &[Argument],
        state: &FontState,
        bridge: &Bridge,
        cg: &NativeCg,
        fonts: &dyn FontLibrary,
        alpha: bool,
    ) -> Result<Self>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `name == "ＭＳ 明朝"` | Local state/value operations | Handled | [74](../../crates/crysirror-system4/src/native_draw/native_text.rs#L74) |

### native_draw/normalized.rs / operation

```rust
fn operation(
    owner: ClientId,
    name: &[u8],
    values: &[i32],
    bridge: &Bridge,
    cg: &NativeCg,
    native_tables: bool,
) -> Result<Option<(i32, ImageInput, bool)>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!( name, b"CopyBright" \| b"CopyColorReverse" \| b"CopySprite" \| b"SpriteCopyAMap" \| b"CopyUseAMapUnder" \| b"CopyUseAMapBorder" \| b"CopyAMapMax" \| b"CopyAMapMin" \| b"Blend" \| b"BlendSrcBright" \| b"BlendAddSatur" \| b"BlendAMap" \| b"BlendAMapSrcOnly" \| b"BlendAMapColor" \| b"BlendAMapColorAlpha" \| b"BlendAMapAlpha" \| b"BlendAMapBright" \| b"BlendAMapAlphaSrcBright" \| b"BlendUseAMapColor" \| b"BlendScreen" \| b"BlendMultiply" \| b"BlendScreenAlpha" \| b"SaturDP_DPxSA" \| b"AddDA_DAxSA" \| b"BlendDA_DAxSA" \| b"SubDA_DAxSA" \| b"ScreenDA_DAxSA" )` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [41](../../crates/crysirror-system4/src/native_draw/normalized.rs#L41) |
| `condition` | `matches!( name, b"FillAlphaColor" \| b"FillScreen" \| b"FillMultiply" \| b"BrightDestOnly" \| b"FillAMapOverBorder" \| b"FillAMapUnderBorder" )` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [71](../../crates/crysirror-system4/src/native_draw/normalized.rs#L71) |
| `condition` | `!copy && !fill \|\| name == b"ScreenDA_DAxSA" && !native_tables` | Local state/value operations | Handled | [80](../../crates/crysirror-system4/src/native_draw/normalized.rs#L80) |
| `(name, rest)` | `_` | Local state/value operations | Unsupported: System4 normalized original copy ABI {} | [193](../../crates/crysirror-system4/src/native_draw/normalized.rs#L193) |
| `condition` | `name == b"CopyBright"` | Local state/value operations | Handled | [198](../../crates/crysirror-system4/src/native_draw/normalized.rs#L198) |
| `condition; under (name, rest)=(b"FillScreen", [r, g, b]) \| (b"FillMultiply", [r, g, b])` | `name == b"FillScreen"` | Local state/value operations | Handled | [230](../../crates/crysirror-system4/src/native_draw/normalized.rs#L230) |
| `condition; under (name, rest)=(b"FillAMapOverBorder", [alpha, border]) \| (b"FillAMapUnderBorder", [alpha, border])` | `name == b"FillAMapOverBorder"` | Local state/value operations | Handled | [257](../../crates/crysirror-system4/src/native_draw/normalized.rs#L257) |
| `(name, rest)` | `_` | Local state/value operations | Unsupported: System4 normalized original fill ABI {} | [266](../../crates/crysirror-system4/src/native_draw/normalized.rs#L266) |
| `condition` | `matches!( name, b"BlendScreen" \| b"BlendScreenAlpha" \| b"FillScreen" \| b"ScreenDA_DAxSA" )` | Local state/value operations | Handled | [281](../../crates/crysirror-system4/src/native_draw/normalized.rs#L281) |
| `condition` | `matches!( name, b"BlendScreen" \| b"BlendScreenAlpha" \| b"FillScreen" \| b"ScreenDA_DAxSA" )` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [281](../../crates/crysirror-system4/src/native_draw/normalized.rs#L281) |
| `condition` | `name == b"BlendAddSatur"` | Local state/value operations | Handled | [290](../../crates/crysirror-system4/src/native_draw/normalized.rs#L290) |

### native_draw/polygon.rs / operation

```rust
fn operation(
    owner: ClientId,
    name: &[u8],
    args: &[Argument],
    bridge: &Bridge,
    cg: &NativeCg,
    native_tables: bool,
) -> Result<Option<Plan>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!matches!(name, b"DrawPolygon" \| b"DrawColorPolygon")` | Local state/value operations | Handled | [23](../../crates/crysirror-system4/src/native_draw/polygon.rs#L23) |
| `condition` | `matches!(name, b"DrawPolygon" \| b"DrawColorPolygon")` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [23](../../crates/crysirror-system4/src/native_draw/polygon.rs#L23) |
| `condition` | `name == b"DrawPolygon"` | Local state/value operations | Handled | [48](../../crates/crysirror-system4/src/native_draw/polygon.rs#L48) |

### native_draw/stretch.rs / operation

```rust
fn operation(
    owner: ClientId,
    name: &[u8],
    values: &[i32],
    bridge: &Bridge,
    cg: &NativeCg,
    native_tables: bool,
) -> Result<Option<(i32, ImageInput, bool)>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!matches!( name, b"CopyStretch" \| b"CopyStretchBlend" \| b"CopyStretchBlendAMap" \| b"CopyStretchAMap" \| b"CopyStretchInterp" \| b"CopyStretchAMapInterp" \| b"CopyReduce" \| b"CopyReduceAMap" )` | Local state/value operations | Handled | [34](../../crates/crysirror-system4/src/native_draw/stretch.rs#L34) |
| `condition` | `matches!( name, b"CopyStretch" \| b"CopyStretchBlend" \| b"CopyStretchBlendAMap" \| b"CopyStretchAMap" \| b"CopyStretchInterp" \| b"CopyStretchAMapInterp" \| b"CopyReduce" \| b"CopyReduceAMap" )` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [34](../../crates/crysirror-system4/src/native_draw/stretch.rs#L34) |
| `condition` | `matches!( name, b"CopyStretchAMap" \| b"CopyStretchAMapInterp" \| b"CopyReduceAMap" )` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [83](../../crates/crysirror-system4/src/native_draw/stretch.rs#L83) |
| `condition` | `alpha_only \|\| name == b"CopyStretchBlendAMap"` | Local state/value operations | Handled | [87](../../crates/crysirror-system4/src/native_draw/stretch.rs#L87) |
| `condition` | `matches!(name, b"CopyStretchInterp" \| b"CopyStretchAMapInterp")` | Local state/value operations | Handled | [95](../../crates/crysirror-system4/src/native_draw/stretch.rs#L95) |
| `condition` | `matches!(name, b"CopyStretchInterp" \| b"CopyStretchAMapInterp")` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [95](../../crates/crysirror-system4/src/native_draw/stretch.rs#L95) |
| `condition` | `matches!(name, b"CopyReduce" \| b"CopyReduceAMap")` | Local state/value operations | Handled | [100](../../crates/crysirror-system4/src/native_draw/stretch.rs#L100) |
| `condition` | `matches!(name, b"CopyReduce" \| b"CopyReduceAMap")` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [100](../../crates/crysirror-system4/src/native_draw/stretch.rs#L100) |
| `condition` | `matches!(name, b"CopyStretchBlend" \| b"CopyStretchBlendAMap")` | Default::default, ImageCompositeMath::Normalized | Handled | [122](../../crates/crysirror-system4/src/native_draw/stretch.rs#L122) |
| `condition` | `matches!(name, b"CopyStretchBlend" \| b"CopyStretchBlendAMap")` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [122](../../crates/crysirror-system4/src/native_draw/stretch.rs#L122) |
| `condition` | `name == b"CopyStretchBlend"` | Local state/value operations | Handled | [123](../../crates/crysirror-system4/src/native_draw/stretch.rs#L123) |
| `condition` | `name == b"CopyStretchBlend"` | Local state/value operations | Handled | [128](../../crates/crysirror-system4/src/native_draw/stretch.rs#L128) |

### native_draw_profile.rs / load_modules

```rust
fn load_modules(
    document: &Document,
    storage: &dyn ClientStorage,
    cancel: &Cancellation,
    modules: &[(&str, u64, &str)],
) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `format!("{:x}", Sha256::digest(&bytes)) != hash` | Local state/value operations | Handled | [100](../../crates/crysirror-system4/src/native_draw_profile.rs#L100) |

### native_effect.rs / sample

```rust
fn sample(
    kind: i32,
    elapsed: u32,
    duration: u32,
    old: Arc<dyn ImageResource>,
    new: Arc<dyn ImageResource>,
    native: bool,
) -> Result<ImageInput>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `kind == 1` | Box::new, serde_json::to_vec | Handled | [574](../../crates/crysirror-system4/src/native_effect.rs#L574) |
| `condition` | `matches!(kind, 12 \| 13 \| 45 \| 46)` | Local state/value operations | Handled | [598](../../crates/crysirror-system4/src/native_effect.rs#L598) |
| `condition` | `matches!(kind, 12 \| 13 \| 45 \| 46)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [598](../../crates/crysirror-system4/src/native_effect.rs#L598) |
| `condition` | `kind == 2 \|\| kind == 4` | Local state/value operations | Handled | [606](../../crates/crysirror-system4/src/native_effect.rs#L606) |
| `condition` | `kind == 2 \|\| kind == 4` | Local state/value operations | Handled | [607](../../crates/crysirror-system4/src/native_effect.rs#L607) |

### native_effect.rs / NativeEffect::dispatch

```rust
fn dispatch(
        &mut self,
        owner: ClientId,
        m: &mut Machine,
        call: &ExternalCall,
        bridge: &mut Bridge,
        cg: &mut NativeCg,
        host: &mut dyn HostServices,
    ) -> Result<Option<Dispatch>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `l.name != b"SACT2"` | Local state/value operations | Handled | [125](../../crates/crysirror-system4/src/native_effect.rs#L125) |
| `condition` | `matches!(f.name.as_slice(), b"EffectSetMask" \| b"EffectSetMaskSP")` | self.mask.dispatch | Handled | [130](../../crates/crysirror-system4/src/native_effect.rs#L130) |
| `condition` | `matches!( f.name.as_slice(), b"QUAKE_SET_CROSS" \| b"QUAKE_SET_ROTATION" )` | self.quake.set_cross, self.quake.set_rotation | Handled | [138](../../crates/crysirror-system4/src/native_effect.rs#L138) |
| `condition` | `f.name == b"QUAKE_SET_CROSS"` | self.quake.set_cross | Handled | [157](../../crates/crysirror-system4/src/native_effect.rs#L157) |
| `condition` | `f.name != b"Effect" && !return_void` | Local state/value operations | Handled | [165](../../crates/crysirror-system4/src/native_effect.rs#L165) |
| `condition` | `kind == 27 && duration > 0` | Local state/value operations | Handled | [219](../../crates/crysirror-system4/src/native_effect.rs#L219) |
| `condition` | `matches!(kind, 18 \| 42) && !fast` | Local state/value operations | Handled | [222](../../crates/crysirror-system4/src/native_effect.rs#L222) |
| `condition` | `matches!(kind, 18 \| 42)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [222](../../crates/crysirror-system4/src/native_effect.rs#L222) |
| `condition` | `!fast && matches!(kind, 7 \| 8 \| 9)` | blind::validate | Handled | [255](../../crates/crysirror-system4/src/native_effect.rs#L255) |
| `condition` | `matches!(kind, 7 \| 8 \| 9)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [255](../../crates/crysirror-system4/src/native_effect.rs#L255) |
| `condition` | `!fast && kind == 53` | radial::validate | Handled | [261](../../crates/crysirror-system4/src/native_effect.rs#L261) |
| `condition` | `matches!(kind, 12 \| 13 \| 45 \| 46) && !fast` | usize::from | Handled | [270](../../crates/crysirror-system4/src/native_effect.rs#L270) |
| `condition` | `matches!(kind, 12 \| 13 \| 45 \| 46)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [270](../../crates/crysirror-system4/src/native_effect.rs#L270) |
| `condition` | `matches!(kind, 12 \| 13)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [271](../../crates/crysirror-system4/src/native_effect.rs#L271) |
| `condition` | `matches!(run.kind, 24 \| 27)` | SequenceRng::msvc, host.now_ms | Handled | [331](../../crates/crysirror-system4/src/native_effect.rs#L331) |
| `condition` | `run.kind == 24` | block::BlockMask::new | Handled | [334](../../crates/crysirror-system4/src/native_effect.rs#L334) |
| `condition` | `native && run.kind != 0` | i32::from | Handled | [372](../../crates/crysirror-system4/src/native_effect.rs#L372) |
| `condition` | `run.kind == 0 \|\| run.skip \|\| final_frame && run.endpoint_seen` | ImageInput::Resource | Handled | [404](../../crates/crysirror-system4/src/native_effect.rs#L404) |
| `run.kind; under receiver=else: run.kind == 0 \|\| run.skip \|\| final_frame && run.endpoint_seen` | `2 \| 32` | Local state/value operations | Handled | [417](../../crates/crysirror-system4/src/native_effect.rs#L417) |
| `run.kind; under receiver=else: run.kind == 0 \|\| run.skip \|\| final_frame && run.endpoint_seen` | `4` | Local state/value operations | Handled | [421](../../crates/crysirror-system4/src/native_effect.rs#L421) |
| `run.kind; under receiver=else: run.kind == 0 \|\| run.skip \|\| final_frame && run.endpoint_seen` | `_` | ImageInput::Resource | Handled | [425](../../crates/crysirror-system4/src/native_effect.rs#L425) |
| `condition; under receiver=else: run.kind == 0 \|\| run.skip \|\| final_frame && run.endpoint_seen` | `matches!(run.kind, 7 \| 8 \| 9)` | blind::sample | Handled | [433](../../crates/crysirror-system4/src/native_effect.rs#L433) |
| `condition; under receiver=else: run.kind == 0 \|\| run.skip \|\| final_frame && run.endpoint_seen` | `run.kind == 24` | Local state/value operations | Handled | [435](../../crates/crysirror-system4/src/native_effect.rs#L435) |
| `condition; under receiver=else: run.kind == 0 \|\| run.skip \|\| final_frame && run.endpoint_seen / receiver=else: run.kind == 24` | `run.kind == 53` | radial::sample | Handled | [440](../../crates/crysirror-system4/src/native_effect.rs#L440) |
| `condition; under receiver=else: run.kind == 0 \|\| run.skip \|\| final_frame && run.endpoint_seen / receiver=else: run.kind == 24 / receiver=else: run.kind == 53` | `run.kind == 52` | feedback::sample_double | Handled | [442](../../crates/crysirror-system4/src/native_effect.rs#L442) |
| `condition; under receiver=else: run.kind == 0 \|\| run.skip \|\| final_frame && run.endpoint_seen / receiver=else: run.kind == 24 / receiver=else: run.kind == 53 / receiver=else: run.kind == 52` | `run.kind == 27` | Local state/value operations | Handled | [474](../../crates/crysirror-system4/src/native_effect.rs#L474) |
| `condition; under receiver=else: run.kind == 0 \|\| run.skip \|\| final_frame && run.endpoint_seen / receiver=else: run.kind == 24 / receiver=else: run.kind == 53 / receiver=else: run.kind == 52 / receiver=else: run.kind == 27` | `matches!(run.kind, 18 \| 42)` | mask::sample | Handled | [482](../../crates/crysirror-system4/src/native_effect.rs#L482) |

### native_effect/blind.rs / sample

```rust
fn sample(
    kind: i32,
    time: u32,
    duration: u32,
    old: Arc<dyn ImageResource>,
    new: Arc<dyn ImageResource>,
) -> Result<ImageInput>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `kind != 9 && vertical != (kind == 7)` | Local state/value operations | Handled | [36](../../crates/crysirror-system4/src/native_effect/blind.rs#L36) |

### native_effect/blind.rs / validate

```rust
fn validate(kind: i32, size: [u32; 2], duration: u32) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `kind == 9 \|\| axis == usize::from(kind == 7)` | Local state/value operations | Handled | [10](../../crates/crysirror-system4/src/native_effect/blind.rs#L10) |

### native_effect/blur.rs / immediate

```rust
fn immediate(kind: i32, duration: u32) -> bool
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(kind, 11 \| 19 \| 40 \| 41)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [9](../../crates/crysirror-system4/src/native_effect/blur.rs#L9) |

### native_effect/blur.rs / recognizes

```rust
fn recognizes(kind: i32) -> bool
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(kind, 6 \| 11 \| 19 \| 40 \| 41 \| 50 \| 51)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [6](../../crates/crysirror-system4/src/native_effect/blur.rs#L6) |

### native_effect/blur.rs / sample

```rust
fn sample(
    kind: i32,
    time: u32,
    duration: u32,
    old: Arc<dyn ImageResource>,
    new: Arc<dyn ImageResource>,
) -> Result<ImageInput>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `kind == 6` | u64::from, ImageInput::Resource | Handled | [136](../../crates/crysirror-system4/src/native_effect/blur.rs#L136) |
| `condition` | `matches!(kind, 11 \| 19)` | ImageInput::Resource, usize::from, u64::from | Handled | [147](../../crates/crysirror-system4/src/native_effect/blur.rs#L147) |
| `condition` | `matches!(kind, 11 \| 19)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [147](../../crates/crysirror-system4/src/native_effect/blur.rs#L147) |
| `condition` | `matches!(kind, 40 \| 41)` | ImageInput::Resource, usize::from, u64::from | Handled | [164](../../crates/crysirror-system4/src/native_effect/blur.rs#L164) |
| `condition` | `matches!(kind, 40 \| 41)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [164](../../crates/crysirror-system4/src/native_effect/blur.rs#L164) |
| `condition` | `kind == 51 && wanted > 24` | Local state/value operations | Handled | [186](../../crates/crysirror-system4/src/native_effect/blur.rs#L186) |
| `condition` | `kind == 50` | Local state/value operations | Handled | [191](../../crates/crysirror-system4/src/native_effect/blur.rs#L191) |
| `condition` | `kind == 50` | ImageInput::Resource | Handled | [198](../../crates/crysirror-system4/src/native_effect/blur.rs#L198) |

### native_effect/blur.rs / validate

```rust
fn validate(kind: i32, size: [u32; 2], duration: u32) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `kind == 6` | Local state/value operations | Handled | [16](../../crates/crysirror-system4/src/native_effect/blur.rs#L16) |
| `condition` | `matches!(kind, 11 \| 19) && duration > 1` | usize::from, u64::from | Handled | [22](../../crates/crysirror-system4/src/native_effect/blur.rs#L22) |
| `condition` | `matches!(kind, 11 \| 19)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [22](../../crates/crysirror-system4/src/native_effect/blur.rs#L22) |
| `condition` | `matches!(kind, 50 \| 51) \|\| kind == 40 && duration > 1` | Local state/value operations | Handled | [32](../../crates/crysirror-system4/src/native_effect/blur.rs#L32) |
| `condition` | `matches!(kind, 50 \| 51) \|\| kind == 41 && duration > 1` | Local state/value operations | Handled | [38](../../crates/crysirror-system4/src/native_effect/blur.rs#L38) |

### native_effect/feedback.rs / recognizes

```rust
fn recognizes(kind: i32) -> bool
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(kind, 10 \| 30 \| 52)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [6](../../crates/crysirror-system4/src/native_effect/feedback.rs#L6) |

### native_effect/feedback.rs / sample

```rust
fn sample(
    kind: i32,
    time: u32,
    duration: u32,
    new: Arc<dyn ImageResource>,
    previous: Arc<dyn ImageResource>,
) -> Result<ImageInput>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `kind == 10` | f64::from, i64::from | Handled | [39](../../crates/crysirror-system4/src/native_effect/feedback.rs#L39) |

### native_effect/geometry.rs / recognizes

```rust
fn recognizes(kind: i32) -> bool
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(kind, 32 \| 33 \| 43 \| 44 \| 48)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [6](../../crates/crysirror-system4/src/native_effect/geometry.rs#L6) |

### native_effect/geometry.rs / sample

```rust
fn sample(
    kind: i32,
    time: u32,
    duration: u32,
    old: Arc<dyn ImageResource>,
    new: Arc<dyn ImageResource>,
) -> Result<ImageInput>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(kind, 43 \| 44)` | u64::from | Handled | [91](../../crates/crysirror-system4/src/native_effect/geometry.rs#L91) |
| `condition` | `matches!(kind, 43 \| 44)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [91](../../crates/crysirror-system4/src/native_effect/geometry.rs#L91) |
| `condition` | `kind == 43` | Local state/value operations | Handled | [94](../../crates/crysirror-system4/src/native_effect/geometry.rs#L94) |
| `condition` | `matches!(kind, 32 \| 33)` | u64::from | Handled | [110](../../crates/crysirror-system4/src/native_effect/geometry.rs#L110) |
| `condition` | `matches!(kind, 32 \| 33)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [110](../../crates/crysirror-system4/src/native_effect/geometry.rs#L110) |
| `condition` | `kind == 32` | u64::from | Handled | [112](../../crates/crysirror-system4/src/native_effect/geometry.rs#L112) |
| `condition` | `kind == 32` | Local state/value operations | Handled | [138](../../crates/crysirror-system4/src/native_effect/geometry.rs#L138) |

### native_effect/geometry.rs / validate

```rust
fn validate(kind: i32, size: [u32; 2], duration: u32) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(kind, 32 \| 33)` | Local state/value operations | Handled | [10](../../crates/crysirror-system4/src/native_effect/geometry.rs#L10) |
| `condition` | `matches!(kind, 32 \| 33)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [10](../../crates/crysirror-system4/src/native_effect/geometry.rs#L10) |
| `condition` | `kind != 48` | Local state/value operations | Handled | [16](../../crates/crysirror-system4/src/native_effect/geometry.rs#L16) |

### native_effect/mask.rs / sample

```rust
fn sample(
    kind: i32,
    time: u32,
    duration: u32,
    old: Arc<dyn ImageResource>,
    new: Arc<dyn ImageResource>,
    mask: Arc<dyn ImageResource>,
) -> Result<ImageInput>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `kind == 18` | Local state/value operations | Handled | [252](../../crates/crysirror-system4/src/native_effect/mask.rs#L252) |
| `condition` | `kind == 18` | Local state/value operations | Handled | [256](../../crates/crysirror-system4/src/native_effect/mask.rs#L256) |

### native_effect/mask.rs / Mask::dispatch

```rust
fn dispatch(
        &mut self,
        owner: ClientId,
        m: &mut Machine,
        call: &ExternalCall,
        bridge: &Bridge,
        cg: &NativeCg,
        host: &mut dyn HostServices,
    ) -> Result<Option<Dispatch>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `l.name != b"SACT2" \|\| !matches!(f.name.as_slice(), b"EffectSetMask" \| b"EffectSetMaskSP")` | Local state/value operations | Handled | [56](../../crates/crysirror-system4/src/native_effect/mask.rs#L56) |
| `condition` | `f.name == b"EffectSetMaskSP"` | HllReturn::Scalar, Phase::Pixels, self.resource.clone | Handled | [79](../../crates/crysirror-system4/src/native_effect/mask.rs#L79) |

### native_effect/quake.rs / Quake::sample

```rust
fn sample(
        self,
        elapsed: u32,
        duration: u32,
        old: Arc<dyn ImageResource>,
        new: Arc<dyn ImageResource>,
        rng: &mut SequenceRng,
    ) -> Result<ImageInput>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `self.mode == 0` | f64::from | Handled | [71](../../crates/crysirror-system4/src/native_effect/quake.rs#L71) |
| `condition` | `self.mode == 1` | f64::from | Handled | [80](../../crates/crysirror-system4/src/native_effect/quake.rs#L80) |

### native_effect/rotation.rs / sample

```rust
fn sample(
    kind: i32,
    time: u32,
    duration: u32,
    old: Arc<dyn ImageResource>,
    new: Arc<dyn ImageResource>,
    previous: Arc<dyn ImageResource>,
) -> Result<ImageInput>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `kind >= 22` | Local state/value operations | Handled | [44](../../crates/crysirror-system4/src/native_effect/rotation.rs#L44) |

### native_file.rs / execute

```rust
fn execute(
    m: &mut Machine,
    state: &mut State,
    request: Request,
    storage: &dyn ClientStorage,
    owner: ClientId,
    check: &dyn Fn() -> Result<()>,
) -> Result<i32>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under request=Request::GetTime(location, slot, offset)` | `kind == 10` | Local state/value operations | Handled | [604](../../crates/crysirror-system4/src/native_file.rs#L604) |

### native_file.rs / NativeFile::dispatch

```rust
fn dispatch(
        &mut self,
        owner: ClientId,
        machine: &mut Option<Machine>,
        call: &ExternalCall,
        files: &NativeFiles,
        host: &mut dyn HostServices,
    ) -> Result<Option<Dispatch>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `name` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [228](../../crates/crysirror-system4/src/native_file.rs#L228) |
| `condition` | `name == b"GetTime" && !args.iter().any(\|arg\|matches!(arg,Argument::String{bytes,..} if bytes.first().is_none_or(\|b\|*b==0)))` | self.clock.is_none, host.begin_civil_time, self.clock.as_mut, Dispatch::WaitUntil, host.now_ms | Handled | [240](../../crates/crysirror-system4/src/native_file.rs#L240) |
| `condition` | `empty_string && name != b"Open"` | Local state/value operations | Handled | [282](../../crates/crysirror-system4/src/native_file.rs#L282) |
| `(name, args.as_slice())` | `_` | Local state/value operations | Partial: System4 unsupported File export/ABI: {} | [344](../../crates/crysirror-system4/src/native_file.rs#L344) |

### native_file/codec.rs / walk

```rust
fn walk(
    m: &mut Machine,
    slot: i32,
    s: &mut Stream,
    reading: bool,
    budget: &mut usize,
    path: &mut BTreeSet<i32>,
    depth: usize,
    check: &dyn Fn() -> Result<()>,
) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(kind, 13 \| 14..=17 \| 50)` | Local state/value operations | Handled | [68](../../crates/crysirror-system4/src/native_file/codec.rs#L68) |
| `condition` | `matches!(kind, 13 \| 14..=17 \| 50)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [68](../../crates/crysirror-system4/src/native_file/codec.rs#L68) |
| `condition` | `matches!(kind, 10 \| 11 \| 47)` | i32::from_le_bytes | Handled | [73](../../crates/crysirror-system4/src/native_file/codec.rs#L73) |
| `condition` | `matches!(kind, 10 \| 11 \| 47)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [73](../../crates/crysirror-system4/src/native_file/codec.rs#L73) |
| `condition` | `kind == 12` | Arc::new, Object::String | Handled | [85](../../crates/crysirror-system4/src/native_file/codec.rs#L85) |

### native_files.rs / NativeFiles::dispatch

```rust
fn dispatch(
        &mut self,
        owner: ClientId,
        machine: &mut Machine,
        call: &ExternalCall,
        host: &mut dyn HostServices,
    ) -> Result<Option<Dispatch>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!matches!(code, 10 \| 12 \| 14 \| 16 \| 26 \| 28)` | Local state/value operations | Handled | [103](../../crates/crysirror-system4/src/native_files.rs#L103) |
| `condition` | `code == 12` | host.storage, self.folder_bytes.clone | Handled | [112](../../crates/crysirror-system4/src/native_files.rs#L112) |
| `condition` | `code == 14` | Local state/value operations | Handled | [120](../../crates/crysirror-system4/src/native_files.rs#L120) |
| `condition` | `matches!(code, 26 \| 28) && self.pending.is_none()` | self.save_path, host.storage, self.pool.spawn, crysirror_resource::capability_lookup::resolve_ascii_case_insensitive, crysirror_resource::read_all | Handled | [130](../../crates/crysirror-system4/src/native_files.rs#L130) |
| `condition` | `code == 28` | Local state/value operations | Handled | [131](../../crates/crysirror-system4/src/native_files.rs#L131) |
| `condition` | `code == 28` | Local state/value operations | Handled | [135](../../crates/crysirror-system4/src/native_files.rs#L135) |
| `condition` | `code == 28` | self.save_path | Handled | [137](../../crates/crysirror-system4/src/native_files.rs#L137) |
| `condition` | `code == 28` | Local state/value operations | Handled | [183](../../crates/crysirror-system4/src/native_files.rs#L183) |
| `condition` | `code == 10 && (bytes.is_empty() \|\| bytes.ends_with(b"/") \|\| bytes.ends_with(b"\\"))` | Local state/value operations | Handled | [189](../../crates/crysirror-system4/src/native_files.rs#L189) |
| `condition` | `code == 16` | Local state/value operations | Handled | [197](../../crates/crysirror-system4/src/native_files.rs#L197) |

### native_input_string.rs / NativeInputString::dispatch

```rust
fn dispatch(
        &mut self,
        owner: ClientId,
        m: &mut Machine,
        call: &ExternalCall,
        host: &mut dyn HostServices,
    ) -> Result<Option<Dispatch>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `library.name != b"InputString"` | Local state/value operations | Handled | [91](../../crates/crysirror-system4/src/native_input_string.rs#L91) |
| `condition` | `name == b"GetResultString"` | self.info, encoding_rs::SHIFT_JIS.encode, host.take_text_input_committed, HllReturn::String | Handled | [159](../../crates/crysirror-system4/src/native_input_string.rs#L159) |
| `condition` | `name == b"ClearResultString"` | host.replace_text_input_committed, String::new | Handled | [182](../../crates/crysirror-system4/src/native_input_string.rs#L182) |
| `(name, args.as_slice())` | `_` | Local state/value operations | Handled | [213](../../crates/crysirror-system4/src/native_input_string.rs#L213) |
| `condition` | `name == b"Begin"` | host.replace_text_input_committed, String::new | Handled | [217](../../crates/crysirror-system4/src/native_input_string.rs#L217) |

### native_logo.rs / load_catalog

```rust
fn load_catalog(
    owner: ClientId,
    storage: &dyn ClientStorage,
    ini: Option<&str>,
    cancel: &Cancellation,
) -> Result<Catalog>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!(106..=109).contains(&id)` | Local state/value operations | Handled | [374](../../crates/crysirror-system4/src/native_logo.rs#L374) |

### native_logo.rs / NativeLogo::dispatch

```rust
fn dispatch(
        &mut self,
        owner: ClientId,
        machine: &mut Machine,
        call: &ExternalCall,
        bridge: &mut Bridge,
        cg: &mut NativeCg,
        host: &mut dyn HostServices,
    ) -> Result<Option<Dispatch>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `library.name != b"AliceLogo2"` | Local state/value operations | Handled | [167](../../crates/crysirror-system4/src/native_logo.rs#L167) |
| `(name, args.as_slice())` | `_` | Local state/value operations | Partial: unsupported native AliceLogo2 export/ABI {} args={:?} return_kind={} | [319](../../crates/crysirror-system4/src/native_logo.rs#L319) |

### native_logo.rs / Run::advance

```rust
fn advance(
        &mut self,
        owner: ClientId,
        pool: &TaskPool,
        catalog: &Arc<Catalog>,
        wave: &[i32; 4],
        images: &dyn ImageMetadata,
        input: &crate::input::Input,
        host: &mut dyn HostServices,
    ) -> Result<Option<Arc<dyn ImageResource>>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under &mut self.phase=Phase::Clock` | `input.key(key)? != 0` | Local state/value operations | Handled | [672](../../crates/crysirror-system4/src/native_logo.rs#L672) |

### native_logs.rs / run

```rust
fn run(
    mut m: Machine,
    mut state: State,
    call: ExternalCall,
    library: Vec<u8>,
    name: Vec<u8>,
    paths: Option<Paths>,
    cancel: Cancellation,
) -> Completed
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!( name.as_slice(), b"AddInt" \| b"AddString" \| b"SetLineMax" \| b"Clear" \| b"Add" \| b"ConvertFromMsgLogManager" \| b"Load" )` | Local state/value operations | Handled | [245](../../crates/crysirror-system4/src/native_logs.rs#L245) |
| `(library.as_slice(), name.as_slice(), args.as_slice())` | `_` | Local state/value operations | Handled | [401](../../crates/crysirror-system4/src/native_logs.rs#L401) |

### native_logs.rs / signature

```rust
fn signature(m: &Machine, call: &ExternalCall) -> Result<(Vec<u8>, Vec<u8>, Vec<Argument>)>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `(lib.name.as_slice(), f.name.as_slice(), args.as_slice())` | `_` | Local state/value operations | Partial: unsupported original message HLL {}/{} ABI | [216](../../crates/crysirror-system4/src/native_logs.rs#L216) |

### native_logs.rs / NativeLogs::dispatch

```rust
fn dispatch(
        &mut self,
        owner: ClientId,
        machine: &mut Option<Machine>,
        call: &ExternalCall,
        files: &NativeFiles,
        host: &mut dyn HostServices,
    ) -> Result<Option<Dispatch>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `library == b"MsgLogManager" && matches!(name.as_slice(), b"Save" \| b"Load")` | VfsPath::new, host.storage | Handled | [144](../../crates/crysirror-system4/src/native_logs.rs#L144) |

### native_movie.rs / NativeMovie::dispatch

```rust
fn dispatch(
        &mut self,
        owner: ClientId,
        machine: &mut Machine,
        call: &ExternalCall,
        bridge: &Bridge,
        host: &mut dyn HostServices,
    ) -> Result<Option<Dispatch>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `library.name != b"PlayMovie"` | Local state/value operations | Handled | [127](../../crates/crysirror-system4/src/native_movie.rs#L127) |
| `condition` | `matches!(name, b"Init" \| b"Load" \| b"Play" \| b"IsPlay")` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [147](../../crates/crysirror-system4/src/native_movie.rs#L147) |
| `name` | `_` | Local state/value operations | Handled | [327](../../crates/crysirror-system4/src/native_movie.rs#L327) |

### native_msgskip.rs / NativeMsgSkip::dispatch

```rust
fn dispatch(
        &mut self,
        owner: ClientId,
        machine: &mut Machine,
        call: &ExternalCall,
        files: &NativeFiles,
        host: &mut dyn HostServices,
    ) -> Result<Option<Dispatch>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `library.name != b"MsgSkip"` | Local state/value operations | Handled | [131](../../crates/crysirror-system4/src/native_msgskip.rs#L131) |
| `condition` | `function.name == b"Init"` | self.pending.is_none, host.storage, self.flags.is_some, self.flags.as_ref, Phase::Flush | Handled | [140](../../crates/crysirror-system4/src/native_msgskip.rs#L140) |
| `function.name.as_slice(); under function.name.as_slice()=b"UseFlag" \| b"SetEnable" \| b"SetState" \| b"SetFlag"` | `_` | self.flags.as_mut | Handled | [237](../../crates/crysirror-system4/src/native_msgskip.rs#L237) |
| `function.name.as_slice(); under function.name.as_slice()=b"GetEnable" \| b"GetState" \| b"GetNumofMsg" \| b"GetNumofFlag"` | `_` | self.flags.as_ref | Handled | [264](../../crates/crysirror-system4/src/native_msgskip.rs#L264) |
| `function.name.as_slice()` | `_` | Local state/value operations | Partial: unsupported original MsgSkip export {} | [268](../../crates/crysirror-system4/src/native_msgskip.rs#L268) |

### native_pixel.rs / dispatch

```rust
fn dispatch(
    owner: ClientId,
    machine: &mut Machine,
    call: &ExternalCall,
    bridge: &mut Bridge,
    cg: &NativeCg,
    host: &mut dyn HostServices,
) -> Result<Option<Dispatch>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!(graph && matches!(name, b"GetPixelColor" \| b"GetAlphaColor") \|\| library.name == b"SACT2" && matches!( name, b"SP_GetPixelValue" \| b"SP_GetAMapValue" \| b"SP_IsPtIn" \| b"SP_IsPtInRect" ))` | Local state/value operations | Handled | [30](../../crates/crysirror-system4/src/native_pixel.rs#L30) |
| `condition` | `matches!(name, b"GetPixelColor" \| b"GetAlphaColor")` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [30](../../crates/crysirror-system4/src/native_pixel.rs#L30) |
| `condition` | `matches!( name, b"SP_GetPixelValue" \| b"SP_GetAMapValue" \| b"SP_IsPtIn" \| b"SP_IsPtInRect" )` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [32](../../crates/crysirror-system4/src/native_pixel.rs#L32) |
| `condition` | `matches!(name, b"SP_IsPtIn" \| b"SP_IsPtInRect")` | i64::from, crysirror_core::region_map::contains_rect, HllReturn::Scalar, i32::from | Handled | [44](../../crates/crysirror-system4/src/native_pixel.rs#L44) |
| `condition` | `matches!(name, b"SP_IsPtIn" \| b"SP_IsPtInRect")` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [44](../../crates/crysirror-system4/src/native_pixel.rs#L44) |
| `condition` | `name == b"SP_IsPtInRect" \|\| !sprite.alpha` | Local state/value operations | Handled | [72](../../crates/crysirror-system4/src/native_pixel.rs#L72) |
| `condition` | `matches!(name, b"GetAlphaColor" \| b"SP_GetAMapValue")` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [90](../../crates/crysirror-system4/src/native_pixel.rs#L90) |

### native_sact_intrinsics.rs / dispatch

```rust
fn dispatch(
    owner: ClientId,
    m: &mut Machine,
    call: &ExternalCall,
    bridge: &Bridge,
    host: &mut dyn HostServices,
) -> Result<Option<Dispatch>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `lib.name != b"SACT2" \|\| !matches!( name.as_slice(), b"SP_Enum" \| b"GAME_MSG_Get" \| b"IntToZenkaku" \| b"IntToHankaku" \| b"StringPopFront" \| b"Mouse_SetPos" )` | Local state/value operations | Handled | [30](../../crates/crysirror-system4/src/native_sact_intrinsics.rs#L30) |
| `condition` | `name == b"SP_Enum"` | HllReturn::Scalar | Handled | [45](../../crates/crysirror-system4/src/native_sact_intrinsics.rs#L45) |
| `condition` | `name == b"Mouse_SetPos"` | host.input_observation, crate::input::Input::validate_scope, host.warp_pointer, HllReturn::Scalar, i32::from | Handled | [79](../../crates/crysirror-system4/src/native_sact_intrinsics.rs#L79) |
| `condition` | `name == b"GAME_MSG_Get"` | usize::try_from, Arc::new | Handled | [98](../../crates/crysirror-system4/src/native_sact_intrinsics.rs#L98) |
| `condition` | `name == b"StringPopFront"` | Arc::new, HllReturn::Scalar, i32::try_from | Handled | [135](../../crates/crysirror-system4/src/native_sact_intrinsics.rs#L135) |
| `condition` | `name == b"IntToZenkaku"` | Local state/value operations | Handled | [195](../../crates/crysirror-system4/src/native_sact_intrinsics.rs#L195) |

### native_sact_text.rs / NativeText::dispatch

```rust
fn dispatch(
        &mut self,
        owner: ClientId,
        m: &mut Machine,
        call: &ExternalCall,
        bridge: &mut Bridge,
        host: &mut dyn HostServices,
    ) -> Result<Option<Dispatch>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `(f.name.as_slice(), args.as_slice(), scalars.as_deref())` | `_` | Local state/value operations | Handled | [508](../../crates/crysirror-system4/src/native_sact_text.rs#L508) |

### native_save.rs / run_resume

```rust
fn run_resume(
    mut original: Machine,
    call: ExternalCall,
    args: crate::vm::resume::ResumeArguments,
    paths: (VfsPath, VfsPath),
    storage: Arc<dyn ClientStorage>,
    cancel: Cancellation,
) -> Completed
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `args.code == 9` | codec::open_envelope, resume::Resume::decode_payload, legacy_json::parse, legacy_json::resume | Handled | [153](../../crates/crysirror-system4/src/native_save.rs#L153) |
| `condition` | `args.code == 23` | legacy_json::parse, legacy_json::comments, codec::open_envelope, resume::Resume::decode_payload | Handled | [173](../../crates/crysirror-system4/src/native_save.rs#L173) |
| `condition` | `args.code == 22` | codec::open_envelope, resume::Resume::decode_payload, codec::seal_envelope, crysirror_resource::capability_lookup::resolve_ascii_case_insensitive | Handled | [208](../../crates/crysirror-system4/src/native_save.rs#L208) |

### native_save_cg.rs / NativeSaveCg::dispatch

```rust
fn dispatch(
        &mut self,
        owner: ClientId,
        m: &mut Machine,
        call: &ExternalCall,
        bridge: &Bridge,
        cg: &NativeCg,
        files: &NativeFiles,
        host: &mut dyn HostServices,
    ) -> Result<Option<Dispatch>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `library.name != b"SACT2" \|\| f.name != b"SP_SaveCG"` | Local state/value operations | Handled | [72](../../crates/crysirror-system4/src/native_save_cg.rs#L72) |

### native_scene.rs / NativeScene::dispatch

```rust
fn dispatch(
        &mut self,
        owner: ClientId,
        machine: &mut Machine,
        call: &ExternalCall,
        bridge: &Bridge,
        cg: &NativeCg,
        host: &mut dyn HostServices,
    ) -> Result<Option<Dispatch>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `library.name != b"SACT2" \|\| !matches!( function.name.as_slice(), b"SetWP" \| b"SetWP_Color" \| b"WP_SetSP" )` | Local state/value operations | Handled | [177](../../crates/crysirror-system4/src/native_scene.rs#L177) |
| `(function.name.as_slice(), values.as_slice())` | `_` | Local state/value operations | Handled | [283](../../crates/crysirror-system4/src/native_scene.rs#L283) |

### native_scene_present.rs / NativePresent::dispatch

```rust
fn dispatch(
        &mut self,
        owner: ClientId,
        m: &mut Machine,
        call: &ExternalCall,
        bridge: &mut Bridge,
        cg: &mut NativeCg,
        host: &mut dyn HostServices,
    ) -> Result<Option<Dispatch>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `library.name != b"SACT2" \|\| f.name != b"Update"` | Local state/value operations | Handled | [79](../../crates/crysirror-system4/src/native_scene_present.rs#L79) |

### native_system.rs / NativeSystem::dispatch

```rust
fn dispatch(
        &mut self,
        owner: ClientId,
        m: &mut Machine,
        call: &ExternalCall,
        bridge: &Bridge,
        host: &mut dyn HostServices,
    ) -> Result<Option<Dispatch>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(call.target, ExternalTarget::System(11))` | self.pending.is_none, crysirror_resource::text_stream::decode_shift_jis, host.begin_desktop_request, Dispatch::WaitUntil, host.now_ms | Handled | [102](../../crates/crysirror-system4/src/native_system.rs#L102) |
| `(sact_civil, function.name.as_slice())` | `_` | Local state/value operations | Handled | [158](../../crates/crysirror-system4/src/native_system.rs#L158) |
| `condition` | `!(lib.name == b"SystemService" \|\| sact_civil) \|\| !matches!( name, b"IsResetOnce" \| b"RestrainScreensaver" \| b"GetGameVersion" \| b"GetGameName" \| b"GetViewWidth" \| b"GetViewHeight" \| b"IsFullScreen" \| b"ChangeNormalScreen" \| b"ChangeFullScreen" \| b"MoveMouseCursorPosImmediately" \| b"SetHideMouseCursorByGame" \| b"GetDate" \| b"GetTime" \| b"IsExistPlayingManual" \| b"OpenPlayingManual" \| b"SetMouseCursorConfig" \| b"GetMouseCursorConfig" )` | Local state/value operations | Handled | [160](../../crates/crysirror-system4/src/native_system.rs#L160) |
| `condition` | `matches!( name, b"IsResetOnce" \| b"RestrainScreensaver" \| b"GetGameVersion" \| b"GetGameName" \| b"GetViewWidth" \| b"GetViewHeight" \| b"IsFullScreen" \| b"ChangeNormalScreen" \| b"ChangeFullScreen" \| b"MoveMouseCursorPosImmediately" \| b"SetHideMouseCursorByGame" \| b"GetDate" \| b"GetTime" \| b"IsExistPlayingManual" \| b"OpenPlayingManual" \| b"SetMouseCursorConfig" \| b"GetMouseCursorConfig" )` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [161](../../crates/crysirror-system4/src/native_system.rs#L161) |
| `condition` | `matches!( name, b"GetGameName" \| b"GetDate" \| b"GetTime" \| b"OpenPlayingManual" \| b"RestrainScreensaver" )` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [190](../../crates/crysirror-system4/src/native_system.rs#L190) |
| `condition` | `matches!(name, b"GetGameVersion" \| b"GetViewWidth" \| b"GetViewHeight")` | Local state/value operations | Handled | [200](../../crates/crysirror-system4/src/native_system.rs#L200) |
| `condition` | `matches!(name, b"GetGameVersion" \| b"GetViewWidth" \| b"GetViewHeight")` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [200](../../crates/crysirror-system4/src/native_system.rs#L200) |
| `name` | `_` | Local state/value operations | Handled | [559](../../crates/crysirror-system4/src/native_system.rs#L559) |

### native_system/misc.rs / Misc::dispatch

```rust
fn dispatch(
        &mut self,
        owner: ClientId,
        m: &mut Machine,
        call: &ExternalCall,
        bridge: &Bridge,
        host: &mut dyn HostServices,
    ) -> Result<Option<Dispatch>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!((lib.name == b"SACT2" && matches!(name, b"Error" \| b"Debug_Pause" \| b"Debug_GetFuncStack")) \|\| (lib.name == b"SystemService" && name == b"AddURLMenu"))` | Local state/value operations | Handled | [105](../../crates/crysirror-system4/src/native_system/misc.rs#L105) |
| `condition` | `matches!(name, b"Error" \| b"Debug_Pause" \| b"Debug_GetFuncStack")` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [106](../../crates/crysirror-system4/src/native_system/misc.rs#L106) |
| `condition` | `name == b"Debug_Pause"` | serde_json::from_slice, host.submit_engine, serde_json::to_vec, Dispatch::WaitUntil, host.now_ms | Handled | [112](../../crates/crysirror-system4/src/native_system/misc.rs#L112) |
| `name` | `_` | Local state/value operations | Handled | [215](../../crates/crysirror-system4/src/native_system/misc.rs#L215) |

### native_wallpaper_get.rs / NativeWallpaperGet::dispatch

```rust
fn dispatch(
        &mut self,
        owner: ClientId,
        m: &mut Machine,
        call: &ExternalCall,
        bridge: &mut Bridge,
        cg: &mut NativeCg,
        scene: &NativeScene,
        host: &mut dyn HostServices,
    ) -> Result<Option<Dispatch>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `l.name != b"SACT2" \|\| f.name != b"WP_GetSP"` | Local state/value operations | Handled | [71](../../crates/crysirror-system4/src/native_wallpaper_get.rs#L71) |

### sprites.rs / Sprite::layer

```rust
fn layer(&self, number: i32) -> Result<Option<Value>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `self.draw_method` | `0` | Local state/value operations | Handled | [83](../../crates/crysirror-system4/src/sprites.rs#L83) |
| `self.draw_method` | `1 \| 2` | Local state/value operations | Handled | [84](../../crates/crysirror-system4/src/sprites.rs#L84) |
| `self.draw_method` | `_` | Local state/value operations | Handled | [87](../../crates/crysirror-system4/src/sprites.rs#L87) |
| `condition` | `self.draw_method == 0` | f64::from | Handled | [119](../../crates/crysirror-system4/src/sprites.rs#L119) |

### sprites.rs / Sprites::scalar

```rust
fn scalar(
        &mut self,
        client: ClientId,
        name: &[u8],
        args: &[i32],
        host: &mut dyn HostServices,
    ) -> Result<Option<i32>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `(name, args)` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [434](../../crates/crysirror-system4/src/sprites.rs#L434) |

### vm/array_find.rs / Machine::array_find_step

```rust
fn array_find_step(&mut self, mut find: Find) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `element_kind(kind)? == 12` | self.heap.string | Handled | [124](../../crates/crysirror-system4/src/vm/array_find.rs#L124) |

### vm/array_find.rs / Machine::execute_array_find

```rust
fn execute_array_find(
        &mut self,
        instruction: &crate::instruction::Instruction,
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `instruction.name` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [29](../../crates/crysirror-system4/src/vm/array_find.rs#L29) |

### vm/array_sort.rs / Machine::execute_array_sort

```rust
fn execute_array_sort(
        &mut self,
        i: &crate::instruction::Instruction,
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `i.name` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [66](../../crates/crysirror-system4/src/vm/array_sort.rs#L66) |
| `condition` | `selector != 0` | self.consume, usize::try_from | Handled | [99](../../crates/crysirror-system4/src/vm/array_sort.rs#L99) |

### vm/arrays.rs / Machine::execute_array_allocation

```rust
fn execute_array_allocation(
        &mut self,
        i: &crate::instruction::Instruction,
    ) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `var.kind` | `22..=25` | Local state/value operations | Handled | [20](../../crates/crysirror-system4/src/vm/arrays.rs#L20) |
| `var.kind` | `32` | Local state/value operations | Handled | [21](../../crates/crysirror-system4/src/vm/arrays.rs#L21) |
| `var.kind` | `52` | Local state/value operations | Handled | [22](../../crates/crysirror-system4/src/vm/arrays.rs#L22) |
| `var.kind` | `60` | Local state/value operations | Handled | [23](../../crates/crysirror-system4/src/vm/arrays.rs#L23) |
| `var.kind` | `67` | Local state/value operations | Handled | [24](../../crates/crysirror-system4/src/vm/arrays.rs#L24) |
| `var.kind` | `k if is_array(k)` | Local state/value operations | Handled | [25](../../crates/crysirror-system4/src/vm/arrays.rs#L25) |
| `var.kind` | `_` | Local state/value operations | Unsupported: System4 allocation of non-array variable | [26](../../crates/crysirror-system4/src/vm/arrays.rs#L26) |
| `condition` | `i.name == "A_REALLOC"` | Action::ClearPage, Action::Consume, self.variable, Action::Release, Action::Array | Handled | [44](../../crates/crysirror-system4/src/vm/arrays.rs#L44) |

### vm/arrays.rs / Machine::execute_array_values

```rust
fn execute_array_values(
        &mut self,
        i: &crate::instruction::Instruction,
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `i.name; under i.name="SH_STRUCT_A_PUSHBACK_LOCAL_STRUCT" \| "SH_GLOBAL_A_PUSHBACK_LOCAL_STRUCT" \| "SH_LOCAL_A_PUSHBACK_LOCAL_STRUCT"` | `_` | self.local | Handled | [246](../../crates/crysirror-system4/src/vm/arrays.rs#L246) |
| `condition; under i.name="A_PUSHBACK" \| "A_INSERT"` | `i.name == "A_INSERT"` | Local state/value operations | Handled | [288](../../crates/crysirror-system4/src/vm/arrays.rs#L288) |
| `var.kind; under i.name="A_PUSHBACK" \| "A_INSERT"` | `22..=25` | Local state/value operations | Handled | [293](../../crates/crysirror-system4/src/vm/arrays.rs#L293) |
| `var.kind; under i.name="A_PUSHBACK" \| "A_INSERT"` | `32` | Local state/value operations | Handled | [294](../../crates/crysirror-system4/src/vm/arrays.rs#L294) |
| `var.kind; under i.name="A_PUSHBACK" \| "A_INSERT"` | `52` | Local state/value operations | Handled | [295](../../crates/crysirror-system4/src/vm/arrays.rs#L295) |
| `var.kind; under i.name="A_PUSHBACK" \| "A_INSERT"` | `60` | Local state/value operations | Handled | [296](../../crates/crysirror-system4/src/vm/arrays.rs#L296) |
| `var.kind; under i.name="A_PUSHBACK" \| "A_INSERT"` | `67` | Local state/value operations | Handled | [297](../../crates/crysirror-system4/src/vm/arrays.rs#L297) |
| `var.kind; under i.name="A_PUSHBACK" \| "A_INSERT"` | `k if is_array(k)` | Local state/value operations | Handled | [298](../../crates/crysirror-system4/src/vm/arrays.rs#L298) |
| `var.kind; under i.name="A_PUSHBACK" \| "A_INSERT"` | `_` | Local state/value operations | Unsupported: System4 insertion into non-array variable | [299](../../crates/crysirror-system4/src/vm/arrays.rs#L299) |
| `condition; under i.name="A_PUSHBACK" \| "A_INSERT"` | `i.name == "A_PUSHBACK"` | Local state/value operations | Handled | [313](../../crates/crysirror-system4/src/vm/arrays.rs#L313) |
| `condition; under i.name="A_PUSHBACK" \| "A_INSERT"` | `i.name == "A_PUSHBACK"` | Local state/value operations | Handled | [330](../../crates/crysirror-system4/src/vm/arrays.rs#L330) |
| `condition; under i.name="A_POPBACK" \| "A_ERASE"` | `i.name == "A_ERASE"` | Local state/value operations | Handled | [348](../../crates/crysirror-system4/src/vm/arrays.rs#L348) |
| `condition; under i.name="A_POPBACK" \| "A_ERASE"` | `i.name == "A_ERASE"` | self.peek | Handled | [351](../../crates/crysirror-system4/src/vm/arrays.rs#L351) |
| `condition; under i.name="A_POPBACK" \| "A_ERASE"` | `i.name == "A_POPBACK"` | Local state/value operations | Handled | [361](../../crates/crysirror-system4/src/vm/arrays.rs#L361) |
| `condition; under i.name="A_POPBACK" \| "A_ERASE"` | `i.name == "A_ERASE"` | Action::Push, i32::from | Handled | [388](../../crates/crysirror-system4/src/vm/arrays.rs#L388) |
| `condition; under i.name="A_POPBACK" \| "A_ERASE"` | `i.name == "A_ERASE"` | Local state/value operations | Handled | [391](../../crates/crysirror-system4/src/vm/arrays.rs#L391) |
| `i.name` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [409](../../crates/crysirror-system4/src/vm/arrays.rs#L409) |

### vm/control.rs / Machine::complete_pure_control

```rust
fn complete_pure_control(&mut self, call: &ExternalCall) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under code=3 \| 4` | `code == 3` | Local state/value operations | Handled | [47](../../crates/crysirror-system4/src/vm/control.rs#L47) |
| `code` | `0` | self.peek, self.complete_system_exit | Handled | [26](../../crates/crysirror-system4/src/vm/control.rs#L26) |
| `code` | `0x11` | self.complete_system | Handled | [32](../../crates/crysirror-system4/src/vm/control.rs#L32) |
| `code` | `3 \| 4` | self.validate_system, self.peek_lock.saturating_sub, self.complete_system | Handled | [41](../../crates/crysirror-system4/src/vm/control.rs#L41) |
| `code` | `0x13` | self.validate_system, self.peek, usize::try_from, self.frames.len, self.heap.alloc | Handled | [58](../../crates/crysirror-system4/src/vm/control.rs#L58) |
| `code` | `0x1b` | self.validate_system, self.peek, self.heap.string, self.consume, self.release_string_now | Handled | [69](../../crates/crysirror-system4/src/vm/control.rs#L69) |
| `code` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [90](../../crates/crysirror-system4/src/vm/control.rs#L90) |

### vm/delegates.rs / Machine::delegate_call

```rust
fn delegate_call(
        &mut self,
        instruction: &crate::instruction::Instruction,
        check: &(dyn Fn() -> Result<()> + Sync),
    ) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `owns_heap(kind) && !is_reference(kind) && value != -1` | self.heap.slot, Action::Release | Handled | [177](../../crates/crysirror-system4/src/vm/delegates.rs#L177) |

### vm/delegates.rs / Machine::execute_delegates

```rust
fn execute_delegates(
        &mut self,
        instruction: &crate::instruction::Instruction,
        check: &(dyn Fn() -> Result<()> + Sync),
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under instruction.name="DG_SET" \| "DG_ADD" \| "DG_EXIST" \| "DG_ERASE"` | `instruction.name == "DG_EXIST"` | self.delegate_contains, self.consume, self.push, i32::from | Handled | [202](../../crates/crysirror-system4/src/vm/delegates.rs#L202) |
| `instruction.name; under instruction.name="DG_SET" \| "DG_ADD" \| "DG_EXIST" \| "DG_ERASE"` | `_` | Self::delegate_erase | Handled | [212](../../crates/crysirror-system4/src/vm/delegates.rs#L212) |
| `condition; under instruction.name="DG_ASSIGN" \| "DG_PLUSA" \| "DG_MINUSA"` | `instruction.name == "DG_ASSIGN"` | Local state/value operations | Handled | [261](../../crates/crysirror-system4/src/vm/delegates.rs#L261) |
| `condition; under instruction.name="DG_ASSIGN" \| "DG_PLUSA" \| "DG_MINUSA"` | `source == destination && instruction.name == "DG_MINUSA"` | self.delegate_sequence, Self::delegate_erase | Handled | [263](../../crates/crysirror-system4/src/vm/delegates.rs#L263) |
| `condition; under instruction.name="DG_ASSIGN" \| "DG_PLUSA" \| "DG_MINUSA"` | `instruction.name == "DG_PLUSA"` | self.delegate_append | Handled | [302](../../crates/crysirror-system4/src/vm/delegates.rs#L302) |
| `instruction.name` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [381](../../crates/crysirror-system4/src/vm/delegates.rs#L381) |

### vm/external.rs / Machine::complete_hll

```rust
fn complete_hll(&mut self, call: &ExternalCall, value: HllReturn) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under (kind, value)=(10 \| 11 \| 26 \| 27 \| 47 \| 55, HllReturn::Scalar(v))` | `kind == 47` | Local state/value operations | Handled | [147](../../crates/crysirror-system4/src/vm/external.rs#L147) |
| `(kind, value)` | `(0, HllReturn::Void)` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [145](../../crates/crysirror-system4/src/vm/external.rs#L145) |
| `(kind, value)` | `(10 \| 11 \| 26 \| 27 \| 47 \| 55, HllReturn::Scalar(v))` | Local state/value operations | Handled | [146](../../crates/crysirror-system4/src/vm/external.rs#L146) |
| `(kind, value)` | `(12, HllReturn::String(s))` | self.heap.alloc, Object::String, Arc::new | Handled | [149](../../crates/crysirror-system4/src/vm/external.rs#L149) |
| `(kind, value)` | `_` | Local state/value operations | Unsupported: System4 HLL return ABI mismatch | [150](../../crates/crysirror-system4/src/vm/external.rs#L150) |

### vm/functions.rs / Machine::execute_functions

```rust
fn execute_functions(
        &mut self,
        instruction: &crate::instruction::Instruction,
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `instruction.name` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [162](../../crates/crysirror-system4/src/vm/functions.rs#L162) |

### vm/heap.rs / Heap::reference

```rust
fn reference(&mut self, id: i32) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `id == -1` | Local state/value operations | Handled | [220](../../crates/crysirror-system4/src/vm/heap.rs#L220) |

### vm/mod.rs / calculate

```rust
fn calculate(op: &str, x: i32, y: i32) -> Result<i32>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `op` | `"ADD"` | Local state/value operations | Handled | [1426](../../crates/crysirror-system4/src/vm/mod.rs#L1426) |
| `op` | `"SUB"` | Local state/value operations | Handled | [1427](../../crates/crysirror-system4/src/vm/mod.rs#L1427) |
| `op` | `"MUL"` | Local state/value operations | Handled | [1428](../../crates/crysirror-system4/src/vm/mod.rs#L1428) |
| `op` | `"DIV"` | Local state/value operations | Handled | [1429](../../crates/crysirror-system4/src/vm/mod.rs#L1429) |
| `op` | `"MOD"` | Local state/value operations | Handled | [1440](../../crates/crysirror-system4/src/vm/mod.rs#L1440) |
| `op` | `"AND"` | Local state/value operations | Handled | [1451](../../crates/crysirror-system4/src/vm/mod.rs#L1451) |
| `op` | `"OR"` | Local state/value operations | Handled | [1452](../../crates/crysirror-system4/src/vm/mod.rs#L1452) |
| `op` | `"XOR"` | Local state/value operations | Handled | [1453](../../crates/crysirror-system4/src/vm/mod.rs#L1453) |
| `op` | `"LSHIFT"` | Local state/value operations | Handled | [1454](../../crates/crysirror-system4/src/vm/mod.rs#L1454) |
| `op` | `"RSHIFT"` | Local state/value operations | Handled | [1458](../../crates/crysirror-system4/src/vm/mod.rs#L1458) |
| `op` | `"LT"` | Local state/value operations | Handled | [1462](../../crates/crysirror-system4/src/vm/mod.rs#L1462) |
| `op` | `"GT"` | Local state/value operations | Handled | [1463](../../crates/crysirror-system4/src/vm/mod.rs#L1463) |
| `op` | `"LTE"` | Local state/value operations | Handled | [1464](../../crates/crysirror-system4/src/vm/mod.rs#L1464) |
| `op` | `"GTE"` | Local state/value operations | Handled | [1465](../../crates/crysirror-system4/src/vm/mod.rs#L1465) |
| `op` | `"NOTE"` | Local state/value operations | Handled | [1466](../../crates/crysirror-system4/src/vm/mod.rs#L1466) |
| `op` | `"EQUALE"` | Local state/value operations | Handled | [1467](../../crates/crysirror-system4/src/vm/mod.rs#L1467) |
| `op` | `k if k.starts_with("LI_")` | Local state/value operations | Handled | [1468](../../crates/crysirror-system4/src/vm/mod.rs#L1468) |
| `op` | `k if k.starts_with("F_")` | f32::from_bits | Handled | [1489](../../crates/crysirror-system4/src/vm/mod.rs#L1489) |
| `op` | `_` | Local state/value operations | Unsupported: System4 unsupported arithmetic {op} | [1505](../../crates/crysirror-system4/src/vm/mod.rs#L1505) |

### vm/mod.rs / Machine::action

```rust
fn action(&mut self) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under a=Action::InitStruct { index, slot, member, depth, }` | `v.kind == 13` | self.heap.get, self.actions.push | Handled | [502](../../crates/crysirror-system4/src/vm/mod.rs#L502) |

### vm/mod.rs / Machine::alloc_structure

```rust
fn alloc_structure(&mut self, index: i32, depth: usize) -> Result<i32>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `v.kind == 13` | self.alloc_structure | Handled | [255](../../crates/crysirror-system4/src/vm/mod.rs#L255) |

### vm/mod.rs / Machine::array_step

```rust
fn array_step(&mut self, mut b: ArrayBuild) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `kind == 13` | self.alloc_structure, self.actions.push, Action::Array | Handled | [1311](../../crates/crysirror-system4/src/vm/mod.rs#L1311) |

### vm/mod.rs / Machine::complete_pure_math

```rust
fn complete_pure_math(
        &mut self,
        call: &ExternalCall,
        check: &(dyn Fn() -> Result<()> + Sync),
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `library.name != b"Math"` | Local state/value operations | Handled | [694](../../crates/crysirror-system4/src/vm/mod.rs#L694) |

### vm/mod.rs / Machine::copy_native_value

```rust
fn copy_native_value(&mut self, value: i32, kind: i32) -> Result<i32>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(kind, 12 \| 13 \| 63) \|\| is_array(kind)` | self.copy_object | Handled | [1350](../../crates/crysirror-system4/src/vm/mod.rs#L1350) |
| `condition` | `matches!(kind, 12 \| 13 \| 63)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [1350](../../crates/crysirror-system4/src/vm/mod.rs#L1350) |

### vm/mod.rs / Machine::copy_page_into

```rust
fn copy_page_into(&mut self, slot: i32, destination: i32, depth: usize) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(kind, 12 \| 13 \| 63) \|\| is_array(kind)` | self.copy_object | Handled | [1391](../../crates/crysirror-system4/src/vm/mod.rs#L1391) |
| `condition` | `matches!(kind, 12 \| 13 \| 63)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [1391](../../crates/crysirror-system4/src/vm/mod.rs#L1391) |

### vm/mod.rs / Machine::execute

```rust
fn execute(
        &mut self,
        i: &crate::instruction::Instruction,
        check: &(dyn Fn() -> Result<()> + Sync),
    ) -> Result<Option<ExternalCall>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `i.name; under i.name="SH_GLOBALREF" \| "SH_LOCALREF" \| "SH_STRUCTREF"` | `_` | self.structure | Handled | [779](../../crates/crysirror-system4/src/vm/mod.rs#L779) |
| `i.name; under i.name="REF" \| "S_REF" \| "SR_REF"` | `_` | Local state/value operations | Handled | [789](../../crates/crysirror-system4/src/vm/mod.rs#L789) |
| `condition; under i.name="SR_ASSIGN" \| "SH_SR_ASSIGN"` | `i.name == "SR_ASSIGN"` | self.peek | Handled | [839](../../crates/crysirror-system4/src/vm/mod.rs#L839) |
| `condition; under i.name="SR_ASSIGN" \| "SH_SR_ASSIGN"` | `i.name == "SR_ASSIGN"` | self.push | Handled | [866](../../crates/crysirror-system4/src/vm/mod.rs#L866) |
| `condition; under i.name="S_MOD"` | `kind == 4` | self.heap.string | Handled | [900](../../crates/crysirror-system4/src/vm/mod.rs#L900) |
| `kind; under i.name="S_MOD"` | `2 \| 48 \| 56` | ByteFormatValue::Integer | Handled | [906](../../crates/crysirror-system4/src/vm/mod.rs#L906) |
| `kind; under i.name="S_MOD"` | `4` | ByteFormatValue::Bytes | Handled | [907](../../crates/crysirror-system4/src/vm/mod.rs#L907) |
| `kind; under i.name="S_MOD"` | `_` | Local state/value operations | Unsupported: System4 S_MOD format value type {kind} unsupported | [908](../../crates/crysirror-system4/src/vm/mod.rs#L908) |
| `condition; under i.name="NEW" \| "SH_LOCALCREATE"` | `i.name == "NEW"` | self.peek | Handled | [957](../../crates/crysirror-system4/src/vm/mod.rs#L957) |
| `condition; under i.name="IFZ" \| "IFNZ"` | `(x == 0) == (i.name == "IFZ")` | u32::try_from | Handled | [1051](../../crates/crysirror-system4/src/vm/mod.rs#L1051) |
| `condition; under i.name="DUP_X2" \| "DUP2_X1"` | `i.name == "DUP_X2"` | Local state/value operations | Handled | [1125](../../crates/crysirror-system4/src/vm/mod.rs#L1125) |
| `i.name; under i.name="INV" \| "NOT" \| "COMPL" \| "ITOB" \| "ITOLI" \| "ITOF" \| "FTOI" \| "F_INV"` | `_` | f32::from_bits | Handled | [1148](../../crates/crysirror-system4/src/vm/mod.rs#L1148) |
| `condition; under i.name="PLUSA" \| "MINUSA" \| "MULA" \| "DIVA" \| "MODA" \| "ANDA" \| "ORA" \| "XORA" \| "LSHIFTA" \| "RSHIFTA"` | `i.name == "DIVA" && y == 0` | self.consume, self.push | Handled | [1168](../../crates/crysirror-system4/src/vm/mod.rs#L1168) |
| `i.name; under i.name="PLUSA" \| "MINUSA" \| "MULA" \| "DIVA" \| "MODA" \| "ANDA" \| "ORA" \| "XORA" \| "LSHIFTA" \| "RSHIFTA"` | `_` | Local state/value operations | Handled | [1187](../../crates/crysirror-system4/src/vm/mod.rs#L1187) |
| `condition; under i.name="INC" \| "DEC"` | `i.name == "INC"` | Local state/value operations | Handled | [1198](../../crates/crysirror-system4/src/vm/mod.rs#L1198) |
| `condition; under i.name="SH_LOCALINC" \| "SH_LOCALDEC"` | `i.name == "SH_LOCALINC"` | Local state/value operations | Handled | [1208](../../crates/crysirror-system4/src/vm/mod.rs#L1208) |
| `i.name` | `_` | self.execute_strings, self.execute_array_values, self.execute_array_find, self.execute_array_sort, self.execute_shorthand | Unsupported: unsupported original instruction {} opcode={:#x} | [1244](../../crates/crysirror-system4/src/vm/mod.rs#L1244) |

### vm/mod.rs / Machine::new

```rust
fn new(program: Arc<Program>) -> Result<Self>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `v.kind == 13` | Local state/value operations | Handled | [177](../../crates/crysirror-system4/src/vm/mod.rs#L177) |
| `condition` | `let Some(f) = m.program.functions.iter().position(\|f\| f.name == b"0")` | Action::Call | Handled | [212](../../crates/crysirror-system4/src/vm/mod.rs#L212) |
| `condition` | `g.kind == 13` | Action::InitGlobal | Handled | [217](../../crates/crysirror-system4/src/vm/mod.rs#L217) |

### vm/primitives.rs / Machine::execute_primitives

```rust
fn execute_primitives(
        &mut self,
        i: &crate::instruction::Instruction,
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `i.name; under i.name="F_PLUSA" \| "F_MINUSA" \| "F_MULA" \| "F_DIVA"` | `_` | Local state/value operations | Handled | [63](../../crates/crysirror-system4/src/vm/primitives.rs#L63) |
| `i.name; under i.name="LI_PLUSA" \| "LI_MINUSA" \| "LI_MULA" \| "LI_DIVA" \| "LI_MODA" \| "LI_ANDA" \| "LI_ORA" \| "LI_XORA" \| "LI_LSHIFTA" \| "LI_RSHIFTA"` | `_` | Local state/value operations | Handled | [107](../../crates/crysirror-system4/src/vm/primitives.rs#L107) |
| `condition; under i.name="LI_INC" \| "LI_DEC"` | `i.name == "LI_INC"` | Local state/value operations | Handled | [123](../../crates/crysirror-system4/src/vm/primitives.rs#L123) |
| `i.name; under i.name="S_LT" \| "S_GT" \| "S_LTE" \| "S_GTE" \| "S_NOTE" \| "S_EQUALE"` | `_` | Local state/value operations | Handled | [141](../../crates/crysirror-system4/src/vm/primitives.rs#L141) |
| `condition; under i.name="SH_STRUCTREF2" \| "SH_REF_STRUCTREF2"` | `i.name == "SH_STRUCTREF2"` | self.structure | Handled | [261](../../crates/crysirror-system4/src/vm/primitives.rs#L261) |
| `condition; under i.name="SH_STRUCTREF2" \| "SH_REF_STRUCTREF2"` | `i.name == "SH_REF_STRUCTREF2"` | self.consume | Handled | [268](../../crates/crysirror-system4/src/vm/primitives.rs#L268) |
| `i.name` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [283](../../crates/crysirror-system4/src/vm/primitives.rs#L283) |

### vm/resume.rs / Machine::finish_resume_save

```rust
fn finish_resume_save(
        &mut self,
        call: &ExternalCall,
        args: &ResumeArguments,
        success: bool,
    ) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `args.code == 8` | Local state/value operations | Handled | [80](../../crates/crysirror-system4/src/vm/resume.rs#L80) |

### vm/resume.rs / Machine::resume_arguments

```rust
fn resume_arguments(&self, call: &ExternalCall) -> Result<ResumeArguments>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `code` | `8` | Local state/value operations | Handled | [23](../../crates/crysirror-system4/src/vm/resume.rs#L23) |
| `code` | `22 \| 23` | Local state/value operations | Handled | [24](../../crates/crysirror-system4/src/vm/resume.rs#L24) |
| `code` | `_` | Local state/value operations | Handled | [25](../../crates/crysirror-system4/src/vm/resume.rs#L25) |
| `condition` | `code == 8` | self.reference, self.heap.get | Handled | [29](../../crates/crysirror-system4/src/vm/resume.rs#L29) |
| `condition` | `matches!(code, 22 \| 23)` | self.peek, self.heap.page_opt | Handled | [36](../../crates/crysirror-system4/src/vm/resume.rs#L36) |
| `code` | `8` | Local state/value operations | Handled | [61](../../crates/crysirror-system4/src/vm/resume.rs#L61) |
| `code` | `22 \| 23` | Local state/value operations | Handled | [62](../../crates/crysirror-system4/src/vm/resume.rs#L62) |
| `code` | `_` | Local state/value operations | Handled | [63](../../crates/crysirror-system4/src/vm/resume.rs#L63) |

### vm/resume.rs / Machine::validate_resume_heap

```rust
fn validate_resume_heap(&self, check: &dyn Fn() -> Result<()>) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `owns_heap(var.kind) && *value != -1` | self.heap.slot | Handled | [717](../../crates/crysirror-system4/src/vm/resume.rs#L717) |
| `condition` | `matches!(var.kind, 12 \| 20)` | Local state/value operations | Handled | [719](../../crates/crysirror-system4/src/vm/resume.rs#L719) |
| `condition` | `var.kind == 13` | Local state/value operations | Handled | [733](../../crates/crysirror-system4/src/vm/resume.rs#L733) |
| `condition; under receiver=else: var.kind == 13` | `is_reference(var.kind) \|\| var.kind == 63` | Local state/value operations | Handled | [738](../../crates/crysirror-system4/src/vm/resume.rs#L738) |

### vm/shorthand.rs / Machine::execute_shorthand

```rust
fn execute_shorthand(
        &mut self,
        i: &crate::instruction::Instruction,
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `i.name; under i.name="SH_IF_LOC_LT_IMM" \| "SH_IF_LOC_GE_IMM" \| "SH_IF_LOC_GT_IMM" \| "SH_IF_LOC_NE_IMM"` | `_` | Local state/value operations | Handled | [23](../../crates/crysirror-system4/src/vm/shorthand.rs#L23) |
| `i.name; under i.name="SH_IF_STRUCTREF_GT_IMM" \| "SH_IF_STRUCTREF_NE_IMM" \| "SH_IF_STRUCTREF_EQ_IMM"` | `_` | Local state/value operations | Handled | [34](../../crates/crysirror-system4/src/vm/shorthand.rs#L34) |
| `condition; under i.name="SH_IF_STRUCTREF_NE_LOCALREF"` | `self.heap.get(self.structure()?, a[0])? != self.heap.get(self.local()?, a[1])?` | u32::try_from | Handled | [41](../../crates/crysirror-system4/src/vm/shorthand.rs#L41) |
| `condition; under i.name="SH_IF_STRUCTREF_Z"` | `self.heap.get(self.structure()?, a[0])? == 0` | u32::try_from | Handled | [46](../../crates/crysirror-system4/src/vm/shorthand.rs#L46) |
| `i.name; under i.name="THISCALLMETHOD_NOPARAM" \| "SH_STRUCTREF_CALLMETHOD_NO_PARAM" \| "SH_STRUCTREF2_CALLMETHOD_NO_PARAM"` | `_` | self.heap.get | Handled | [67](../../crates/crysirror-system4/src/vm/shorthand.rs#L67) |
| `condition; under i.name="SH_GLOBAL_ASSIGN_LOCAL" \| "SH_GLOBAL_ASSIGN_IMM"` | `i.name == "SH_GLOBAL_ASSIGN_IMM"` | Local state/value operations | Handled | [90](../../crates/crysirror-system4/src/vm/shorthand.rs#L90) |
| `condition; under i.name="SH_GLOBAL_ASSIGN_LOCAL" \| "SH_GLOBAL_ASSIGN_IMM"` | `old > 0 && (matches!(kind, 12 \| 13) \|\| is_array(kind))` | self.schedule | Handled | [97](../../crates/crysirror-system4/src/vm/shorthand.rs#L97) |
| `condition; under i.name="SH_GLOBAL_ASSIGN_LOCAL" \| "SH_GLOBAL_ASSIGN_IMM"` | `matches!(kind, 12 \| 13)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [97](../../crates/crysirror-system4/src/vm/shorthand.rs#L97) |
| `condition; under i.name="A_NUMOF_GLOB_1" \| "A_NUMOF_STRUCT_1"` | `i.name == "A_NUMOF_GLOB_1"` | Local state/value operations | Handled | [118](../../crates/crysirror-system4/src/vm/shorthand.rs#L118) |
| `i.name; under i.name="SH_GLOBAL_S_REF" \| "SH_LOCAL_S_REF" \| "SH_STRUCT_S_REF" \| "S_REF2"` | `_` | self.peek | Handled | [136](../../crates/crysirror-system4/src/vm/shorthand.rs#L136) |
| `condition; under i.name="SH_GLOBAL_S_REF" \| "SH_LOCAL_S_REF" \| "SH_STRUCT_S_REF" \| "S_REF2"` | `i.name == "S_REF2"` | self.consume | Handled | [140](../../crates/crysirror-system4/src/vm/shorthand.rs#L140) |
| `i.name; under i.name="SH_LOCALSREF_EMPTY" \| "SH_STRUCTSREF_EMPTY" \| "SH_GLOBALSREF_EMPTY"` | `_` | Local state/value operations | Handled | [155](../../crates/crysirror-system4/src/vm/shorthand.rs#L155) |
| `i.name; under i.name="SH_LOCAL_APUSHBACK_LOCALSREF" \| "SH_GLOBAL_APUSHBACK_LOCALSREF" \| "SH_STRUCT_APUSHBACK_LOCALSREF"` | `_` | self.structure | Handled | [176](../../crates/crysirror-system4/src/vm/shorthand.rs#L176) |
| `i.name; under i.name="SH_SASSIGN_LOCALSREF" \| "SH_SASSIGN_STRUCTSREF" \| "SH_SASSIGN_GLOBALSREF"` | `_` | Local state/value operations | Handled | [242](../../crates/crysirror-system4/src/vm/shorthand.rs#L242) |
| `condition; under i.name="SH_LOCALREF_SASSIGN_LOCALSREF" \| "SH_STRUCTREF_SASSIGN_LOCALSREF"` | `i.name == "SH_LOCALREF_SASSIGN_LOCALSREF"` | self.local | Handled | [249](../../crates/crysirror-system4/src/vm/shorthand.rs#L249) |
| `i.name; under i.name="SH_LOCALSREF_EQ_STR0" \| "SH_LOCALSREF_NE_STR0" \| "SH_STRUCTSREF_NE_STR0" \| "SH_GLOBALSREF_NE_STR0"` | `_` | self.local | Handled | [272](../../crates/crysirror-system4/src/vm/shorthand.rs#L272) |
| `i.name` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [296](../../crates/crysirror-system4/src/vm/shorthand.rs#L296) |

### vm/strings.rs / Machine::execute_strings

```rust
fn execute_strings(
        &mut self,
        instruction: &crate::instruction::Instruction,
        check: &(dyn Fn() -> Result<()> + Sync),
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under instruction.name="S_LENGTH" \| "S_LENGTH2"` | `instruction.name == "S_LENGTH"` | self.reference, self.heap.get | Handled | [22](../../crates/crysirror-system4/src/vm/strings.rs#L22) |
| `condition; under instruction.name="S_LENGTH" \| "S_LENGTH2"` | `instruction.name == "S_LENGTH"` | Local state/value operations | Handled | [30](../../crates/crysirror-system4/src/vm/strings.rs#L30) |
| `condition; under instruction.name="S_LENGTH" \| "S_LENGTH2"` | `instruction.name == "S_LENGTH2"` | self.release_string_now | Handled | [32](../../crates/crysirror-system4/src/vm/strings.rs#L32) |
| `instruction.name` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [105](../../crates/crysirror-system4/src/vm/strings.rs#L105) |

### worker_adapter.rs / NativeAdapter::advance

```rust
fn advance(
        &mut self,
        h: &mut dyn HostServices,
        input: InputFrame,
        budget: u32,
        offer_input: bool,
    ) -> Result<VmYield>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under self.stop.clone()=Some(Step::External(call))` | `matches!(call.target, crate::vm::ExternalTarget::System(5))` | self.advance_reset | Handled | [481](../../crates/crysirror-system4/src/worker_adapter.rs#L481) |
| `condition; under self.stop.clone()=Some(Step::External(call))` | `matches!( call.target, crate::vm::ExternalTarget::System(8 \| 9 \| 22 \| 23) )` | self.native_files.save_path, h.storage, self.native_files.io_pool, self.machine.take, crate::native_save::run_resume | Handled | [525](../../crates/crysirror-system4/src/worker_adapter.rs#L525) |
| `condition; under self.stop.clone()=Some(Step::External(call))` | `matches!( call.target, crate::vm::ExternalTarget::System(1 \| 2 \| 24 \| 25) )` | self.native_files.save_path, h.storage, self.native_files.io_pool, self.machine.take, crate::native_save::run | Handled | [546](../../crates/crysirror-system4/src/worker_adapter.rs#L546) |
| `condition; under self.stop.clone()=Some(Step::External(call))` | `matches!(call.target, crate::vm::ExternalTarget::System(7 \| 18 \| 15))` | h.input_observation, crate::input::Input::validate_scope, crate::buffered_input::Events::new | Handled | [575](../../crates/crysirror-system4/src/worker_adapter.rs#L575) |
