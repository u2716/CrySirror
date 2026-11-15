# Majiro developer reference

[All engines](README.md)

Mode: `adopt`.

## ABI and host contract

MJO X/V1.000 scripts use 16-bit opcodes and typed operands. Native syscalls are
identified by **32-bit hashes**. Variable/array instructions encode flags,
hash and offset; scalar arithmetic and assignment families select type from
the opcode. Byte PCs, frame/local identity, string bytes and bsel targets remain
native state.

| Native area | CrySirror service / implementation | Return / completion |
| --- | --- | --- |
| Stack, arrays, frames and arithmetic | `machine.rs` / `machine/scalar.rs` | Typed values and native PC; destructors can suspend at external work. |
| Syscall hashes | `adapter` syscall modules | VM emits `Event`; adapter completion supplies the native result. |
| Pictures, masks and animation | Shared RCT/RC8/image-processing and retained Layers | Native pages/sprites and ownership policy remain in the adapter. |
| Message / choices / history | Shared glyph jobs and retained UI | Native PROC layout, hooks and read/selection state. |
| Music, sound, voice and media | Shared Audio/Media services | Native channel waits and asynchronous resources. |
| Settings / slots / files | Scoped Storage | Atomic portable records; native names do not imply original file compatibility. |

Settings use portable storage rather than importing the original registry.
Slots named `majiro_%04d`, `quick%d` and `last` are CrySirror JSON records,
**not MajiroSavV1.103**. Restore releases transient visuals, reconstructs the
captured thread/save-scope state and follows the native reset policy. Unknown
hashes and unsupported parameter types fail at their native call.

Source/provenance: [SOURCE.json](../../crates/crysirror-majiro/SOURCE.json),
[VM events](../../crates/crysirror-majiro/src/machine.rs),
[slot policy](../../crates/crysirror-majiro/src/adapter/saves.rs).
Table generation is source validation only; game behavior requires a scoped run.

<!-- BEGIN GENERATED DISPATCH TABLES -->

## Opcode and API implementation tables

`Handled` means an executable source branch exists. It is not a claim that every argument/subselector or a complete game has been audited. Explicit gaps are shown per row.

| Status | Entries |
| --- | ---: |
| Empty handler | 3 |
| Handled | 448 |
| Partial | 6 |
| Unsupported | 1 |

These tables cover the checked-in catalogues and explicit dispatch branches. An engine-wide upstream denominator is not inferred from these rows.

Reference mnemonics: [MajiroTools Opcode.cs](https://github.com/AtomCrafty/MajiroTools/blob/e0261df508b6c8065865bb51b4059263cef84394/src/MajiroLib/Script/Opcode.cs) at the revision recorded in [SOURCE.json](../../crates/crysirror-majiro/SOURCE.json). Execution status comes from CrySirror's handlers, not that reference catalogue.

## MJO VM opcodes

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x100 mul.i` | Multiply (*) | Handled | Guard: *op>=0x100 && *op<=0x188 | [machine.rs:417](../../crates/crysirror-majiro/src/machine.rs#L417) |
| `0x101 mul.r` | Multiply (*) | Handled | Guard: *op>=0x100 && *op<=0x188 | [machine.rs:417](../../crates/crysirror-majiro/src/machine.rs#L417) |
| `0x108 div.i` | Divide (/) | Handled | Guard: *op>=0x100 && *op<=0x188 | [machine.rs:417](../../crates/crysirror-majiro/src/machine.rs#L417) |
| `0x109 div.r` | Divide (/) | Handled | Guard: *op>=0x100 && *op<=0x188 | [machine.rs:417](../../crates/crysirror-majiro/src/machine.rs#L417) |
| `0x110 rem` | Remainder (%) | Handled | Guard: *op>=0x100 && *op<=0x188 | [machine.rs:417](../../crates/crysirror-majiro/src/machine.rs#L417) |
| `0x118 add.i` | Add (+) | Handled | Guard: *op>=0x100 && *op<=0x188 | [machine.rs:417](../../crates/crysirror-majiro/src/machine.rs#L417) |
| `0x119 add.r` | Add (+) | Handled | Guard: *op>=0x100 && *op<=0x188 | [machine.rs:417](../../crates/crysirror-majiro/src/machine.rs#L417) |
| `0x11A add.s` | Add (+) | Handled | Guard: *op>=0x100 && *op<=0x188 | [machine.rs:417](../../crates/crysirror-majiro/src/machine.rs#L417) |
| `0x120 sub.i` | Subtract (-) | Handled | Guard: *op>=0x100 && *op<=0x188 | [machine.rs:417](../../crates/crysirror-majiro/src/machine.rs#L417) |
| `0x121 sub.r` | Subtract (-) | Handled | Guard: *op>=0x100 && *op<=0x188 | [machine.rs:417](../../crates/crysirror-majiro/src/machine.rs#L417) |
| `0x128 shr` | Shift right (>>) | Handled | Guard: *op>=0x100 && *op<=0x188 | [machine.rs:417](../../crates/crysirror-majiro/src/machine.rs#L417) |
| `0x130 shl` | Shift left (<<) | Handled | Guard: *op>=0x100 && *op<=0x188 | [machine.rs:417](../../crates/crysirror-majiro/src/machine.rs#L417) |
| `0x138 cle.i` | Compare less than or equal (<=) | Handled | Guard: *op>=0x100 && *op<=0x188 | [machine.rs:417](../../crates/crysirror-majiro/src/machine.rs#L417) |
| `0x139 cle.r` | Compare less than or equal (<=) | Handled | Guard: *op>=0x100 && *op<=0x188 | [machine.rs:417](../../crates/crysirror-majiro/src/machine.rs#L417) |
| `0x13A cle.s` | Compare less than or equal (<=) | Handled | Guard: *op>=0x100 && *op<=0x188 | [machine.rs:417](../../crates/crysirror-majiro/src/machine.rs#L417) |
| `0x140 clt.i` | Compare less than (<) | Handled | Guard: *op>=0x100 && *op<=0x188 | [machine.rs:417](../../crates/crysirror-majiro/src/machine.rs#L417) |
| `0x141 clt.r` | Compare less than (<) | Handled | Guard: *op>=0x100 && *op<=0x188 | [machine.rs:417](../../crates/crysirror-majiro/src/machine.rs#L417) |
| `0x142 clt.s` | Compare less than (<) | Handled | Guard: *op>=0x100 && *op<=0x188 | [machine.rs:417](../../crates/crysirror-majiro/src/machine.rs#L417) |
| `0x148 cge.i` | Compare greater than or equal (>=) | Handled | Guard: *op>=0x100 && *op<=0x188 | [machine.rs:417](../../crates/crysirror-majiro/src/machine.rs#L417) |
| `0x149 cge.r` | Compare greater than or equal (>=) | Handled | Guard: *op>=0x100 && *op<=0x188 | [machine.rs:417](../../crates/crysirror-majiro/src/machine.rs#L417) |
| `0x14A cge.s` | Compare greater than or equal (>=) | Handled | Guard: *op>=0x100 && *op<=0x188 | [machine.rs:417](../../crates/crysirror-majiro/src/machine.rs#L417) |
| `0x150 cgt.i` | Compare greater than (>) | Handled | Guard: *op>=0x100 && *op<=0x188 | [machine.rs:417](../../crates/crysirror-majiro/src/machine.rs#L417) |
| `0x151 cgt.r` | Compare greater than (>) | Handled | Guard: *op>=0x100 && *op<=0x188 | [machine.rs:417](../../crates/crysirror-majiro/src/machine.rs#L417) |
| `0x152 cgt.s` | Compare greater than (>) | Handled | Guard: *op>=0x100 && *op<=0x188 | [machine.rs:417](../../crates/crysirror-majiro/src/machine.rs#L417) |
| `0x158 ceq.i` | Compare equal (==) | Handled | Guard: *op>=0x100 && *op<=0x188 | [machine.rs:417](../../crates/crysirror-majiro/src/machine.rs#L417) |
| `0x159 ceq.r` | Compare equal (==) | Handled | Guard: *op>=0x100 && *op<=0x188 | [machine.rs:417](../../crates/crysirror-majiro/src/machine.rs#L417) |
| `0x15A ceq.s` | Compare equal (==) | Handled | Guard: *op>=0x100 && *op<=0x188 | [machine.rs:417](../../crates/crysirror-majiro/src/machine.rs#L417) |
| `0x15B ceq.iarr` | Compare equal (==) | Handled | Guard: *op>=0x100 && *op<=0x188 | [machine.rs:417](../../crates/crysirror-majiro/src/machine.rs#L417) |
| `0x15C ceq.rarr` | Compare equal (==) | Handled | Guard: *op>=0x100 && *op<=0x188 | [machine.rs:417](../../crates/crysirror-majiro/src/machine.rs#L417) |
| `0x15D ceq.sarr` | Compare equal (==) | Handled | Guard: *op>=0x100 && *op<=0x188 | [machine.rs:417](../../crates/crysirror-majiro/src/machine.rs#L417) |
| `0x160 cne.i` | Compare unequal (!=) | Handled | Guard: *op>=0x100 && *op<=0x188 | [machine.rs:417](../../crates/crysirror-majiro/src/machine.rs#L417) |
| `0x161 cne.r` | Compare unequal (!=) | Handled | Guard: *op>=0x100 && *op<=0x188 | [machine.rs:417](../../crates/crysirror-majiro/src/machine.rs#L417) |
| `0x162 cne.s` | Compare unequal (!=) | Handled | Guard: *op>=0x100 && *op<=0x188 | [machine.rs:417](../../crates/crysirror-majiro/src/machine.rs#L417) |
| `0x163 cne.iarr` | Compare unequal (!=) | Handled | Guard: *op>=0x100 && *op<=0x188 | [machine.rs:417](../../crates/crysirror-majiro/src/machine.rs#L417) |
| `0x164 cne.rarr` | Compare unequal (!=) | Handled | Guard: *op>=0x100 && *op<=0x188 | [machine.rs:417](../../crates/crysirror-majiro/src/machine.rs#L417) |
| `0x165 cne.sarr` | Compare unequal (!=) | Handled | Guard: *op>=0x100 && *op<=0x188 | [machine.rs:417](../../crates/crysirror-majiro/src/machine.rs#L417) |
| `0x168 xor` | Bitwise XOR (^) | Handled | Guard: *op>=0x100 && *op<=0x188 | [machine.rs:417](../../crates/crysirror-majiro/src/machine.rs#L417) |
| `0x170 andl` | Logical AND (&&) | Handled | Guard: *op>=0x100 && *op<=0x188 | [machine.rs:417](../../crates/crysirror-majiro/src/machine.rs#L417) |
| `0x178 orl` | Logical OR (\|\|) | Handled | Guard: *op>=0x100 && *op<=0x188 | [machine.rs:417](../../crates/crysirror-majiro/src/machine.rs#L417) |
| `0x180 and` | Bitwise AND (&) | Handled | Guard: *op>=0x100 && *op<=0x188 | [machine.rs:417](../../crates/crysirror-majiro/src/machine.rs#L417) |
| `0x188 or` | Bitwise OR (\|) | Handled | Guard: *op>=0x100 && *op<=0x188 | [machine.rs:417](../../crates/crysirror-majiro/src/machine.rs#L417) |
| `0x190 notl` | Logical NOT (!) | Handled | — | [machine.rs:420](../../crates/crysirror-majiro/src/machine.rs#L420) |
| `0x191 nop.191` | No operation (reference mnemonic) | Empty handler | Original no-op status not established | [machine.rs:424](../../crates/crysirror-majiro/src/machine.rs#L424) |
| `0x198 not` | Bitwise NOT (~) | Handled | — | [machine.rs:420](../../crates/crysirror-majiro/src/machine.rs#L420) |
| `0x1A0 neg.i` | Negate (-) | Handled | — | [machine.rs:420](../../crates/crysirror-majiro/src/machine.rs#L420) |
| `0x1A1 neg.r` | Negate (-) | Handled | — | [machine.rs:421](../../crates/crysirror-majiro/src/machine.rs#L421) |
| `0x1A8 nop.1a8` | No operation (reference mnemonic) | Empty handler | Original no-op status not established | [machine.rs:424](../../crates/crysirror-majiro/src/machine.rs#L424) |
| `0x1A9 nop.1a9` | No operation (reference mnemonic) | Empty handler | Original no-op status not established | [machine.rs:424](../../crates/crysirror-majiro/src/machine.rs#L424) |
| `0x1B0 st.i` | Store variable and retain result (=) | Handled | Guard: (0x1b0..=0x265).contains(op) | [machine.rs:411](../../crates/crysirror-majiro/src/machine.rs#L411) |
| `0x1B1 st.r` | Store variable and retain result (=) | Handled | Guard: (0x1b0..=0x265).contains(op) | [machine.rs:411](../../crates/crysirror-majiro/src/machine.rs#L411) |
| `0x1B2 st.s` | Store variable and retain result (=) | Handled | Guard: (0x1b0..=0x265).contains(op) | [machine.rs:411](../../crates/crysirror-majiro/src/machine.rs#L411) |
| `0x1B3 st.iarr` | Store variable and retain result (=) | Handled | Guard: (0x1b0..=0x265).contains(op) | [machine.rs:411](../../crates/crysirror-majiro/src/machine.rs#L411) |
| `0x1B4 st.rarr` | Store variable and retain result (=) | Handled | Guard: (0x1b0..=0x265).contains(op) | [machine.rs:411](../../crates/crysirror-majiro/src/machine.rs#L411) |
| `0x1B5 st.sarr` | Store variable and retain result (=) | Handled | Guard: (0x1b0..=0x265).contains(op) | [machine.rs:411](../../crates/crysirror-majiro/src/machine.rs#L411) |
| `0x1B8 st.mul.i` | Store variable and retain result (*=) | Handled | Guard: (0x1b0..=0x265).contains(op) | [machine.rs:411](../../crates/crysirror-majiro/src/machine.rs#L411) |
| `0x1B9 st.mul.r` | Store variable and retain result (*=) | Handled | Guard: (0x1b0..=0x265).contains(op) | [machine.rs:411](../../crates/crysirror-majiro/src/machine.rs#L411) |
| `0x1C0 st.div.i` | Store variable and retain result (/=) | Handled | Guard: (0x1b0..=0x265).contains(op) | [machine.rs:411](../../crates/crysirror-majiro/src/machine.rs#L411) |
| `0x1C1 st.div.r` | Store variable and retain result (/=) | Handled | Guard: (0x1b0..=0x265).contains(op) | [machine.rs:411](../../crates/crysirror-majiro/src/machine.rs#L411) |
| `0x1C8 st.rem` | Store variable and retain result (%=) | Handled | Guard: (0x1b0..=0x265).contains(op) | [machine.rs:411](../../crates/crysirror-majiro/src/machine.rs#L411) |
| `0x1D0 st.add.i` | Store variable and retain result (+=) | Handled | Guard: (0x1b0..=0x265).contains(op) | [machine.rs:411](../../crates/crysirror-majiro/src/machine.rs#L411) |
| `0x1D1 st.add.r` | Store variable and retain result (+=) | Handled | Guard: (0x1b0..=0x265).contains(op) | [machine.rs:411](../../crates/crysirror-majiro/src/machine.rs#L411) |
| `0x1D2 st.add.s` | Store variable and retain result (+=) | Handled | Guard: (0x1b0..=0x265).contains(op) | [machine.rs:411](../../crates/crysirror-majiro/src/machine.rs#L411) |
| `0x1D8 st.sub.i` | Store variable and retain result (-=) | Handled | Guard: (0x1b0..=0x265).contains(op) | [machine.rs:411](../../crates/crysirror-majiro/src/machine.rs#L411) |
| `0x1D9 st.sub.r` | Store variable and retain result (-=) | Handled | Guard: (0x1b0..=0x265).contains(op) | [machine.rs:411](../../crates/crysirror-majiro/src/machine.rs#L411) |
| `0x1E0 st.shl` | Store variable and retain result (<<=) | Handled | Guard: (0x1b0..=0x265).contains(op) | [machine.rs:411](../../crates/crysirror-majiro/src/machine.rs#L411) |
| `0x1E8 st.shr` | Store variable and retain result (>>=) | Handled | Guard: (0x1b0..=0x265).contains(op) | [machine.rs:411](../../crates/crysirror-majiro/src/machine.rs#L411) |
| `0x1F0 st.and` | Store variable and retain result (&=) | Handled | Guard: (0x1b0..=0x265).contains(op) | [machine.rs:411](../../crates/crysirror-majiro/src/machine.rs#L411) |
| `0x1F8 st.xor` | Store variable and retain result (^=) | Handled | Guard: (0x1b0..=0x265).contains(op) | [machine.rs:411](../../crates/crysirror-majiro/src/machine.rs#L411) |
| `0x200 st.or` | Store variable and retain result (\|=) | Handled | Guard: (0x1b0..=0x265).contains(op) | [machine.rs:411](../../crates/crysirror-majiro/src/machine.rs#L411) |
| `0x210 stp.i` | Store variable and pop result (=) | Handled | Guard: (0x1b0..=0x265).contains(op) | [machine.rs:411](../../crates/crysirror-majiro/src/machine.rs#L411) |
| `0x211 stp.r` | Store variable and pop result (=) | Handled | Guard: (0x1b0..=0x265).contains(op) | [machine.rs:411](../../crates/crysirror-majiro/src/machine.rs#L411) |
| `0x212 stp.s` | Store variable and pop result (=) | Handled | Guard: (0x1b0..=0x265).contains(op) | [machine.rs:411](../../crates/crysirror-majiro/src/machine.rs#L411) |
| `0x213 stp.iarr` | Store variable and pop result (=) | Handled | Guard: (0x1b0..=0x265).contains(op) | [machine.rs:411](../../crates/crysirror-majiro/src/machine.rs#L411) |
| `0x214 stp.rarr` | Store variable and pop result (=) | Handled | Guard: (0x1b0..=0x265).contains(op) | [machine.rs:411](../../crates/crysirror-majiro/src/machine.rs#L411) |
| `0x215 stp.sarr` | Store variable and pop result (=) | Handled | Guard: (0x1b0..=0x265).contains(op) | [machine.rs:411](../../crates/crysirror-majiro/src/machine.rs#L411) |
| `0x218 stp.mul.i` | Store variable and pop result (*=) | Handled | Guard: (0x1b0..=0x265).contains(op) | [machine.rs:411](../../crates/crysirror-majiro/src/machine.rs#L411) |
| `0x219 stp.mul.r` | Store variable and pop result (*=) | Handled | Guard: (0x1b0..=0x265).contains(op) | [machine.rs:411](../../crates/crysirror-majiro/src/machine.rs#L411) |
| `0x220 stp.div.i` | Store variable and pop result (/=) | Handled | Guard: (0x1b0..=0x265).contains(op) | [machine.rs:411](../../crates/crysirror-majiro/src/machine.rs#L411) |
| `0x221 stp.div.r` | Store variable and pop result (/=) | Handled | Guard: (0x1b0..=0x265).contains(op) | [machine.rs:411](../../crates/crysirror-majiro/src/machine.rs#L411) |
| `0x228 stp.rem` | Store variable and pop result (%=) | Handled | Guard: (0x1b0..=0x265).contains(op) | [machine.rs:411](../../crates/crysirror-majiro/src/machine.rs#L411) |
| `0x230 stp.add.i` | Store variable and pop result (+=) | Handled | Guard: (0x1b0..=0x265).contains(op) | [machine.rs:411](../../crates/crysirror-majiro/src/machine.rs#L411) |
| `0x231 stp.add.r` | Store variable and pop result (+=) | Handled | Guard: (0x1b0..=0x265).contains(op) | [machine.rs:411](../../crates/crysirror-majiro/src/machine.rs#L411) |
| `0x232 stp.add.s` | Store variable and pop result (+=) | Handled | Guard: (0x1b0..=0x265).contains(op) | [machine.rs:411](../../crates/crysirror-majiro/src/machine.rs#L411) |
| `0x238 stp.sub.i` | Store variable and pop result (-=) | Handled | Guard: (0x1b0..=0x265).contains(op) | [machine.rs:411](../../crates/crysirror-majiro/src/machine.rs#L411) |
| `0x239 stp.sub.r` | Store variable and pop result (-=) | Handled | Guard: (0x1b0..=0x265).contains(op) | [machine.rs:411](../../crates/crysirror-majiro/src/machine.rs#L411) |
| `0x240 stp.shl` | Store variable and pop result (<<=) | Handled | Guard: (0x1b0..=0x265).contains(op) | [machine.rs:411](../../crates/crysirror-majiro/src/machine.rs#L411) |
| `0x248 stp.shr` | Store variable and pop result (>>=) | Handled | Guard: (0x1b0..=0x265).contains(op) | [machine.rs:411](../../crates/crysirror-majiro/src/machine.rs#L411) |
| `0x250 stp.and` | Store variable and pop result (&=) | Handled | Guard: (0x1b0..=0x265).contains(op) | [machine.rs:411](../../crates/crysirror-majiro/src/machine.rs#L411) |
| `0x258 stp.xor` | Store variable and pop result (^=) | Handled | Guard: (0x1b0..=0x265).contains(op) | [machine.rs:411](../../crates/crysirror-majiro/src/machine.rs#L411) |
| `0x260 stp.or` | Store variable and pop result (\|=) | Handled | Guard: (0x1b0..=0x265).contains(op) | [machine.rs:411](../../crates/crysirror-majiro/src/machine.rs#L411) |
| `0x270 stelem.i` | Store array element and retain result (=) | Handled | Guard: (0x270..=0x322).contains(op) | [machine.rs:402](../../crates/crysirror-majiro/src/machine.rs#L402) |
| `0x271 stelem.r` | Store array element and retain result (=) | Handled | Guard: (0x270..=0x322).contains(op) | [machine.rs:402](../../crates/crysirror-majiro/src/machine.rs#L402) |
| `0x272 stelem.s` | Store array element and retain result (=) | Handled | Guard: (0x270..=0x322).contains(op) | [machine.rs:402](../../crates/crysirror-majiro/src/machine.rs#L402) |
| `0x278 stelem.mul.i` | Store array element and retain result (*=) | Handled | Guard: (0x270..=0x322).contains(op) | [machine.rs:402](../../crates/crysirror-majiro/src/machine.rs#L402) |
| `0x279 stelem.mul.r` | Store array element and retain result (*=) | Handled | Guard: (0x270..=0x322).contains(op) | [machine.rs:402](../../crates/crysirror-majiro/src/machine.rs#L402) |
| `0x280 stelem.div.i` | Store array element and retain result (/=) | Handled | Guard: (0x270..=0x322).contains(op) | [machine.rs:402](../../crates/crysirror-majiro/src/machine.rs#L402) |
| `0x281 stelem.div.r` | Store array element and retain result (/=) | Handled | Guard: (0x270..=0x322).contains(op) | [machine.rs:402](../../crates/crysirror-majiro/src/machine.rs#L402) |
| `0x288 stelem.rem` | Store array element and retain result (%=) | Handled | Guard: (0x270..=0x322).contains(op) | [machine.rs:402](../../crates/crysirror-majiro/src/machine.rs#L402) |
| `0x290 stelem.add.i` | Store array element and retain result (+=) | Handled | Guard: (0x270..=0x322).contains(op) | [machine.rs:402](../../crates/crysirror-majiro/src/machine.rs#L402) |
| `0x291 stelem.add.r` | Store array element and retain result (+=) | Handled | Guard: (0x270..=0x322).contains(op) | [machine.rs:402](../../crates/crysirror-majiro/src/machine.rs#L402) |
| `0x292 stelem.add.s` | Store array element and retain result (+=) | Handled | Guard: (0x270..=0x322).contains(op) | [machine.rs:402](../../crates/crysirror-majiro/src/machine.rs#L402) |
| `0x298 stelem.sub.i` | Store array element and retain result (-=) | Handled | Guard: (0x270..=0x322).contains(op) | [machine.rs:402](../../crates/crysirror-majiro/src/machine.rs#L402) |
| `0x299 stelem.sub.r` | Store array element and retain result (-=) | Handled | Guard: (0x270..=0x322).contains(op) | [machine.rs:402](../../crates/crysirror-majiro/src/machine.rs#L402) |
| `0x2A0 stelem.shl` | Store array element and retain result (<<=) | Handled | Guard: (0x270..=0x322).contains(op) | [machine.rs:402](../../crates/crysirror-majiro/src/machine.rs#L402) |
| `0x2A8 stelem.shr` | Store array element and retain result (>>=) | Handled | Guard: (0x270..=0x322).contains(op) | [machine.rs:402](../../crates/crysirror-majiro/src/machine.rs#L402) |
| `0x2B0 stelem.and` | Store array element and retain result (&=) | Handled | Guard: (0x270..=0x322).contains(op) | [machine.rs:402](../../crates/crysirror-majiro/src/machine.rs#L402) |
| `0x2B8 stelem.xor` | Store array element and retain result (^=) | Handled | Guard: (0x270..=0x322).contains(op) | [machine.rs:402](../../crates/crysirror-majiro/src/machine.rs#L402) |
| `0x2C0 stelem.or` | Store array element and retain result (\|=) | Handled | Guard: (0x270..=0x322).contains(op) | [machine.rs:402](../../crates/crysirror-majiro/src/machine.rs#L402) |
| `0x2D0 stelemp.i` | Store array element and pop result (=) | Handled | Guard: (0x270..=0x322).contains(op) | [machine.rs:402](../../crates/crysirror-majiro/src/machine.rs#L402) |
| `0x2D1 stelemp.r` | Store array element and pop result (=) | Handled | Guard: (0x270..=0x322).contains(op) | [machine.rs:402](../../crates/crysirror-majiro/src/machine.rs#L402) |
| `0x2D2 stelemp.s` | Store array element and pop result (=) | Handled | Guard: (0x270..=0x322).contains(op) | [machine.rs:402](../../crates/crysirror-majiro/src/machine.rs#L402) |
| `0x2D8 stelemp.mul.i` | Store array element and pop result (*=) | Handled | Guard: (0x270..=0x322).contains(op) | [machine.rs:402](../../crates/crysirror-majiro/src/machine.rs#L402) |
| `0x2D9 stelemp.mul.r` | Store array element and pop result (*=) | Handled | Guard: (0x270..=0x322).contains(op) | [machine.rs:402](../../crates/crysirror-majiro/src/machine.rs#L402) |
| `0x2E0 stelemp.div.i` | Store array element and pop result (/=) | Handled | Guard: (0x270..=0x322).contains(op) | [machine.rs:402](../../crates/crysirror-majiro/src/machine.rs#L402) |
| `0x2E1 stelemp.div.r` | Store array element and pop result (/=) | Handled | Guard: (0x270..=0x322).contains(op) | [machine.rs:402](../../crates/crysirror-majiro/src/machine.rs#L402) |
| `0x2E8 stelemp.rem` | Store array element and pop result (%=) | Handled | Guard: (0x270..=0x322).contains(op) | [machine.rs:402](../../crates/crysirror-majiro/src/machine.rs#L402) |
| `0x2F0 stelemp.add.i` | Store array element and pop result (+=) | Handled | Guard: (0x270..=0x322).contains(op) | [machine.rs:402](../../crates/crysirror-majiro/src/machine.rs#L402) |
| `0x2F1 stelemp.add.r` | Store array element and pop result (+=) | Handled | Guard: (0x270..=0x322).contains(op) | [machine.rs:402](../../crates/crysirror-majiro/src/machine.rs#L402) |
| `0x2F2 stelemp.add.s` | Store array element and pop result (+=) | Handled | Guard: (0x270..=0x322).contains(op) | [machine.rs:402](../../crates/crysirror-majiro/src/machine.rs#L402) |
| `0x2F8 stelemp.sub.i` | Store array element and pop result (-=) | Handled | Guard: (0x270..=0x322).contains(op) | [machine.rs:402](../../crates/crysirror-majiro/src/machine.rs#L402) |
| `0x2F9 stelemp.sub.r` | Store array element and pop result (-=) | Handled | Guard: (0x270..=0x322).contains(op) | [machine.rs:402](../../crates/crysirror-majiro/src/machine.rs#L402) |
| `0x300 stelemp.shl` | Store array element and pop result (<<=) | Handled | Guard: (0x270..=0x322).contains(op) | [machine.rs:402](../../crates/crysirror-majiro/src/machine.rs#L402) |
| `0x308 stelemp.shr` | Store array element and pop result (>>=) | Handled | Guard: (0x270..=0x322).contains(op) | [machine.rs:402](../../crates/crysirror-majiro/src/machine.rs#L402) |
| `0x310 stelemp.and` | Store array element and pop result (&=) | Handled | Guard: (0x270..=0x322).contains(op) | [machine.rs:402](../../crates/crysirror-majiro/src/machine.rs#L402) |
| `0x318 stelemp.xor` | Store array element and pop result (^=) | Handled | Guard: (0x270..=0x322).contains(op) | [machine.rs:402](../../crates/crysirror-majiro/src/machine.rs#L402) |
| `0x320 stelemp.or` | Store array element and pop result (\|=) | Handled | Guard: (0x270..=0x322).contains(op) | [machine.rs:402](../../crates/crysirror-majiro/src/machine.rs#L402) |
| `0x800 ldc.i` | Load constant | Handled | — | [machine.rs:387](../../crates/crysirror-majiro/src/machine.rs#L387) |
| `0x801 ldstr` | ldstr | Handled | — | [machine.rs:388](../../crates/crysirror-majiro/src/machine.rs#L388) |
| `0x802 ld` | Load variable | Handled | — | [machine.rs:394](../../crates/crysirror-majiro/src/machine.rs#L394) |
| `0x803 ldc.r` | Load constant | Handled | — | [machine.rs:389](../../crates/crysirror-majiro/src/machine.rs#L389) |
| `0x80F call` | Call a function | Handled | — | [machine.rs:449](../../crates/crysirror-majiro/src/machine.rs#L449) |
| `0x810 callp` | Call function and discard result | Handled | — | [machine.rs:449](../../crates/crysirror-majiro/src/machine.rs#L449) |
| `0x829 alloca` | Allocate local variables | Handled | — | [machine.rs:391](../../crates/crysirror-majiro/src/machine.rs#L391) |
| `0x82B ret` | Return from a call | Handled | Guard: self.frames.last().unwrap().bsel.destructor!=0 | [machine.rs:454](../../crates/crysirror-majiro/src/machine.rs#L454) |
| `0x82C br` | Branch | Handled | — | [machine.rs:426](../../crates/crysirror-majiro/src/machine.rs#L426) |
| `0x82D brtrue` | Branch if true | Handled | — | [machine.rs:427](../../crates/crysirror-majiro/src/machine.rs#L427) |
| `0x82E brfalse` | Branch if false | Handled | — | [machine.rs:427](../../crates/crysirror-majiro/src/machine.rs#L427) |
| `0x82F pop` | pop | Handled | — | [machine.rs:425](../../crates/crysirror-majiro/src/machine.rs#L425) |
| `0x830 br.case` | Branch | Handled | — | [machine.rs:428](../../crates/crysirror-majiro/src/machine.rs#L428) |
| `0x831 bne.case` | Branch if unequal | Handled | — | [machine.rs:435](../../crates/crysirror-majiro/src/machine.rs#L435) |
| `0x832 bge.case` | Branch if greater than or equal | Handled | — | [machine.rs:435](../../crates/crysirror-majiro/src/machine.rs#L435) |
| `0x833 ble.case` | Branch if less than or equal | Handled | — | [machine.rs:435](../../crates/crysirror-majiro/src/machine.rs#L435) |
| `0x834 syscall` | Call native syscall and retain result | Handled | — | [machine.rs:468](../../crates/crysirror-majiro/src/machine.rs#L468) |
| `0x835 syscallp` | Call native syscall and discard result | Handled | — | [machine.rs:468](../../crates/crysirror-majiro/src/machine.rs#L468) |
| `0x836 argcheck` | Check argument types | Handled | — | [machine.rs:393](../../crates/crysirror-majiro/src/machine.rs#L393) |
| `0x837 ldelem` | Load array element | Handled | — | [machine.rs:401](../../crates/crysirror-majiro/src/machine.rs#L401) |
| `0x838 blt.case` | Branch if less than | Handled | — | [machine.rs:435](../../crates/crysirror-majiro/src/machine.rs#L435) |
| `0x839 bgt.case` | Branch if greater than | Handled | — | [machine.rs:435](../../crates/crysirror-majiro/src/machine.rs#L435) |
| `0x83A line` | Record source/read-history line | Handled | — | [machine.rs:390](../../crates/crysirror-majiro/src/machine.rs#L390) |
| `0x83B bsel.1` | Button-selection control | Handled | — | [machine.rs:404](../../crates/crysirror-majiro/src/machine.rs#L404) |
| `0x83C bsel.3` | Button-selection control | Handled | — | [machine.rs:406](../../crates/crysirror-majiro/src/machine.rs#L406) |
| `0x83D bsel.2` | Button-selection control | Handled | — | [machine.rs:405](../../crates/crysirror-majiro/src/machine.rs#L405) |
| `0x83E conv.i` | Convert scalar type | Handled | — | [machine.rs:422](../../crates/crysirror-majiro/src/machine.rs#L422) |
| `0x83F conv.r` | Convert scalar type | Handled | — | [machine.rs:423](../../crates/crysirror-majiro/src/machine.rs#L423) |
| `0x840 text` | Emit text | Handled | — | [machine.rs:469](../../crates/crysirror-majiro/src/machine.rs#L469) |
| `0x841 proc` | Process message | Handled | — | [machine.rs:470](../../crates/crysirror-majiro/src/machine.rs#L470) |
| `0x842 ctrl` | Process text control | Handled | — | [machine.rs:471](../../crates/crysirror-majiro/src/machine.rs#L471) |
| `0x843 bsel.x` | Button-selection control | Handled | — | [machine.rs:403](../../crates/crysirror-majiro/src/machine.rs#L403) |
| `0x844 bsel.clr` | Button-selection control | Handled | — | [machine.rs:407](../../crates/crysirror-majiro/src/machine.rs#L407) |
| `0x845 bsel.4` | Button-selection control | Handled | — | [machine.rs:408](../../crates/crysirror-majiro/src/machine.rs#L408) |
| `0x846 bsel.jmp.4` | Button-selection control | Handled | — | [machine.rs:409](../../crates/crysirror-majiro/src/machine.rs#L409) |
| `0x847 bsel.5` | Button-selection control | Handled | — | [machine.rs:410](../../crates/crysirror-majiro/src/machine.rs#L410) |
| `0x850 switch` | Select branch from jump table | Handled | — | [machine.rs:443](../../crates/crysirror-majiro/src/machine.rs#L443) |

## Native syscall hashes

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x8ACBC0` | input | Handled | — | [syscalls.rs:606](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L606) |
| `0x1B4517C` | Graphics and composition (try_gfx) | Handled | — | [gfx.rs:1690](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1690) |
| `0x2026AC6` | System commands (try_syscall) | Handled | — | [syscalls.rs:823](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L823) |
| `0x53FAC99` | YUKIIRO422730 complements the three colour channels only | Handled | — | [gfx.rs:1423](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1423) |
| `0x542E1B7` | Native call (native_flags_native) | Handled | — | [native_flags.rs:64](../../crates/crysirror-majiro/src/adapter/native_flags.rs#L64) |
| `0x5D6ED69` | Audio playback/control (audio_control) | Handled | — | [native_audio.rs:42](../../crates/crysirror-majiro/src/adapter/native_audio.rs#L42) |
| `0x5EA6E4D` | System commands (try_syscall) | Handled | — | [syscalls.rs:477](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L477) |
| `0x6906970` | strings | Handled | — | [syscalls.rs:358](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L358) |
| `0x6B3C8AC` | Execute try console branch | Handled | — | [console.rs:335](../../crates/crysirror-majiro/src/adapter/console.rs#L335) |
| `0x72E738C` | System commands (try_syscall) | Handled | — | [syscalls.rs:828](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L828) |
| `0x78A756E` | Control flow (native_thread_control) | Handled | — | [events.rs:62](../../crates/crysirror-majiro/src/machine/events.rs#L62) |
| `0x98399F2` | Native call (native) | Handled | — | [adapter.rs:283](../../crates/crysirror-majiro/src/adapter.rs#L283) |
| `0xC070535` | Audio playback/control (audio_control) | Handled | — | [native_audio.rs:38](../../crates/crysirror-majiro/src/adapter/native_audio.rs#L38) |
| `0xC7A0E97` | System commands (try_syscall) | Handled | — | [syscalls.rs:826](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L826) |
| `0xC93FCB4` | Native call (native) | Handled | — | [adapter.rs:378](../../crates/crysirror-majiro/src/adapter.rs#L378) |
| `0xDC634C4` | music | Handled | — | [syscalls.rs:691](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L691) |
| `0xE42730D` | Execute try console branch | Handled | — | [console.rs:295](../../crates/crysirror-majiro/src/adapter/console.rs#L295) |
| `0xEBC053B` | Reviewed classic dispatcher410060 branch0x0ebc053b stores the top integer in dword_49b020. This is native policy, not a… | Handled | — | [adapter.rs:337](../../crates/crysirror-majiro/src/adapter.rs#L337) |
| `0x109CA5DB` | System commands (try_syscall) | Handled | — | [syscalls.rs:414](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L414) |
| `0x111BD910` | Native call (native) | Handled | — | [adapter.rs:368](../../crates/crysirror-majiro/src/adapter.rs#L368) |
| `0x1204D7E8` | System commands (try_syscall) | Handled | — | [syscalls.rs:582](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L582) |
| `0x12877048` | System commands (try_syscall) | Handled | — | [syscalls.rs:347](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L347) |
| `0x1295BBDA` | Control flow (native_thread_control) | Handled | — | [events.rs:62](../../crates/crysirror-majiro/src/machine/events.rs#L62) |
| `0x134D5585` | System commands (try_syscall) | Handled | — | [syscalls.rs:342](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L342) |
| `0x13C305DA` | Native call (native) | Handled | — | [adapter.rs:359](../../crates/crysirror-majiro/src/adapter.rs#L359) |
| `0x15EEDEAA` | System commands (try_syscall) | Handled | — | [syscalls.rs:707](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L707) |
| `0x171AEE22` | Graphics and composition (try_gfx) | Handled | — | [gfx.rs:1305](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1305) |
| `0x19FC7CC7` | System commands (try_syscall) | Handled | — | [syscalls.rs:803](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L803) |
| `0x1BB47604` | Save/load (try_saves) | Handled | — | [saves.rs:133](../../crates/crysirror-majiro/src/adapter/saves.rs#L133) |
| `0x1D93FD7E` | Audio playback/control (native_audio_wait) | Handled | — | [audio_wait.rs:22](../../crates/crysirror-majiro/src/adapter/audio_wait.rs#L22) |
| `0x1EC3C6BB` | Execute try console branch | Handled | — | [console.rs:403](../../crates/crysirror-majiro/src/adapter/console.rs#L403) |
| `0x1F57B724` | arrays | Handled | — | [syscalls.rs:331](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L331) |
| `0x2075C300` | System commands (try_syscall) | Handled | — | [syscalls.rs:615](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L615) |
| `0x221C2CC2` | System commands (try_syscall) | Handled | — | [syscalls.rs:478](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L478) |
| `0x2539D212` | Input (read_input_file) | Handled | — | [input_files.rs:134](../../crates/crysirror-majiro/src/adapter/input_files.rs#L134) |
| `0x25401A1F` | strings | Handled | — | [syscalls.rs:358](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L358) |
| `0x25D92502` | System commands (try_syscall) | Handled | — | [syscalls.rs:524](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L524) |
| `0x2626FE02` | is idempotent pause, not a toggle. requests resume | Handled | — | [native_audio.rs:51](../../crates/crysirror-majiro/src/adapter/native_audio.rs#L51) |
| `0x265E07D7` | Graphics and composition (try_gfx) | Handled | — | [gfx.rs:1429](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1429) |
| `0x266A3C79` | Native call (native) | Handled | — | [adapter.rs:298](../../crates/crysirror-majiro/src/adapter.rs#L298) |
| `0x26DC6566` | sounds 1..4 | Handled | — | [syscalls.rs:717](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L717) |
| `0x29F1AC40` | Control flow (native_thread_control) | Handled | — | [events.rs:95](../../crates/crysirror-majiro/src/machine/events.rs#L95) |
| `0x2CB1BC41` | System commands (try_syscall) | Handled | — | [syscalls.rs:522](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L522) |
| `0x2E861C51` | System commands (try_syscall) | Handled | — | [syscalls.rs:739](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L739) |
| `0x30636D6E` | Graphics and composition (try_gfx) | Partial | Unknown subselectors warn and continue | [gfx.rs:1097](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1097) |
| `0x307F28BC` | System commands (try_syscall) | Handled | — | [syscalls.rs:515](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L515) |
| `0x30FA2A29` | Execute try console branch | Handled | — | [console.rs:307](../../crates/crysirror-majiro/src/adapter/console.rs#L307) |
| `0x32D0236C` | Execute try console branch | Handled | — | [console.rs:361](../../crates/crysirror-majiro/src/adapter/console.rs#L361) |
| `0x3378635D` | System commands (try_syscall) | Handled | — | [syscalls.rs:666](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L666) |
| `0x3670B283` | System commands (try_syscall) | Handled | — | [syscalls.rs:629](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L629) |
| `0x3704C919` | System commands (try_syscall) | Handled | — | [syscalls.rs:501](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L501) |
| `0x38721A26` | Native call (fast_mode_native) | Handled | — | [fast_mode.rs:298](../../crates/crysirror-majiro/src/adapter/fast_mode.rs#L298) |
| `0x39AE7891` | System commands (try_syscall) | Handled | — | [syscalls.rs:743](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L743) |
| `0x3A3BFB29` | System commands (try_syscall) | Handled | — | [syscalls.rs:689](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L689) |
| `0x3A3CA0F1` | Save/load (try_saves) | Handled | — | [saves.rs:198](../../crates/crysirror-majiro/src/adapter/saves.rs#L198) |
| `0x3B1D5537` | Graphics and composition (try_gfx) | Handled | — | [gfx.rs:1712](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1712) |
| `0x3C84073C` | Native call (fast_mode_native) | Handled | — | [fast_mode.rs:290](../../crates/crysirror-majiro/src/adapter/fast_mode.rs#L290) |
| `0x3CAD3884` | Graphics and composition (try_gfx) | Handled | — | [gfx.rs:1221](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1221) |
| `0x3D053317` | System commands (try_syscall) | Handled | — | [syscalls.rs:345](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L345) |
| `0x3D6CF5BE` | YUKIIRO447AF0/////; classic43A790 and early4268B0 share the Sound3 wrapper. GetPlayState4614F0 uses active+first-pass-c… | Handled | — | [syscalls.rs:773](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L773) |
| `0x3E11F2C5` | Graphics and composition (try_gfx) | Handled | — | [gfx.rs:1011](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1011) |
| `0x3EABF498` | Native call (fast_mode_native) | Handled | — | [fast_mode.rs:329](../../crates/crysirror-majiro/src/adapter/fast_mode.rs#L329) |
| `0x3F34D519` | Graphics and composition (try_gfx) | Unsupported | Majiro sprite cell cross-fade scratch/50ms update policy is not connected | [gfx.rs:1411](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1411) |
| `0x3FE05B39` | System commands (try_syscall) | Handled | — | [syscalls.rs:391](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L391) |
| `0x40258A56` | Graphics and composition (try_gfx) | Handled | — | [gfx.rs:1465](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1465) |
| `0x403960F8` | Execute try console branch | Handled | — | [console.rs:270](../../crates/crysirror-majiro/src/adapter/console.rs#L270) |
| `0x4133D7A9` | Execute try console branch | Handled | — | [console.rs:311](../../crates/crysirror-majiro/src/adapter/console.rs#L311) |
| `0x42D7C922` | System commands (try_syscall) | Handled | — | [syscalls.rs:478](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L478) |
| `0x42F6FDDF` | Clock and waits (clock_native) | Handled | — | [clock_policy.rs:108](../../crates/crysirror-majiro/src/adapter/clock_policy.rs#L108) |
| `0x4311CFA6` | System commands (try_syscall) | Handled | — | [syscalls.rs:721](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L721) |
| `0x44B5C4ED` | Save/load (try_saves) | Handled | — | [saves.rs:145](../../crates/crysirror-majiro/src/adapter/saves.rs#L145) |
| `0x44BC7555` | Native call (fast_mode_native) | Handled | — | [fast_mode.rs:356](../../crates/crysirror-majiro/src/adapter/fast_mode.rs#L356) |
| `0x45815F6D` | System commands (try_syscall) | Handled | — | [syscalls.rs:415](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L415) |
| `0x45ACD4A7` | Native call (fast_mode_native) | Handled | — | [fast_mode.rs:310](../../crates/crysirror-majiro/src/adapter/fast_mode.rs#L310) |
| `0x45AD5F4D` | System commands (try_syscall) | Handled | — | [syscalls.rs:600](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L600) |
| `0x4662E95D` | System commands (try_syscall) | Handled | — | [syscalls.rs:591](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L591) |
| `0x4980F82C` | music | Handled | — | [syscalls.rs:691](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L691) |
| `0x4A02D664` | Graphics and composition (try_gfx) | Handled | — | [gfx.rs:1552](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1552) |
| `0x4AB0CAEF` | Graphics and composition (try_gfx) | Handled | — | [gfx.rs:1622](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1622) |
| `0x4B306C5C` | Native call (fast_mode_native) | Handled | — | [fast_mode.rs:272](../../crates/crysirror-majiro/src/adapter/fast_mode.rs#L272) |
| `0x4B5AC64B` | YUKIIRO4037B0: release mode returns0 without reading42CE70's development-only message word. Other startup profiles rema… | Partial | Majiro development message word host is not connected | [adapter.rs:297](../../crates/crysirror-majiro/src/adapter.rs#L297) |
| `0x4C310B4B` | Graphics and composition (try_gfx) | Handled | — | [gfx.rs:1041](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1041) |
| `0x4C99B0EA` | Save/load (try_saves) | Handled | — | [saves.rs:197](../../crates/crysirror-majiro/src/adapter/saves.rs#L197) |
| `0x4D849AA6` | Save/load (try_saves) | Handled | — | [saves.rs:141](../../crates/crysirror-majiro/src/adapter/saves.rs#L141) |
| `0x4E1EB9D0` | System commands (try_syscall) | Handled | — | [syscalls.rs:675](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L675) |
| `0x4EF903F3` | System commands (try_syscall) | Handled | — | [syscalls.rs:495](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L495) |
| `0x4F62152A` | Native call (native) | Handled | — | [adapter.rs:294](../../crates/crysirror-majiro/src/adapter.rs#L294) |
| `0x4F6367B3` | System commands (try_syscall) | Handled | — | [syscalls.rs:707](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L707) |
| `0x4FFAB7C4` | Save/load (try_saves) | Handled | — | [saves.rs:149](../../crates/crysirror-majiro/src/adapter/saves.rs#L149) |
| `0x507F22C6` | System commands (try_syscall) | Handled | — | [syscalls.rs:562](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L562) |
| `0x5131E234` | Execute try console branch | Handled | — | [console.rs:295](../../crates/crysirror-majiro/src/adapter/console.rs#L295) |
| `0x52819912` | Graphics and composition (try_gfx) | Handled | — | [gfx.rs:1533](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1533) |
| `0x539B07BC` | Graphics and composition (try_gfx) | Handled | — | [gfx.rs:989](../../crates/crysirror-majiro/src/adapter/gfx.rs#L989) |
| `0x546F69C3` | System commands (try_syscall) | Handled | — | [syscalls.rs:739](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L739) |
| `0x55232561` | Input (read_input_file) | Handled | — | [input_files.rs:133](../../crates/crysirror-majiro/src/adapter/input_files.rs#L133) |
| `0x5548EF5E` | Graphics and composition (try_gfx) | Handled | — | [gfx.rs:1552](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1552) |
| `0x56BBBA3A` | Graphics and composition (try_gfx) | Partial | Unknown subselectors warn and continue | [gfx.rs:1097](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1097) |
| `0x5785A054` | Audio playback/control (native_audio_wait) | Handled | — | [audio_wait.rs:19](../../crates/crysirror-majiro/src/adapter/audio_wait.rs#L19) |
| `0x57AD7635` | Audio playback/control (native_audio_wait) | Handled | — | [audio_wait.rs:21](../../crates/crysirror-majiro/src/adapter/audio_wait.rs#L21) |
| `0x57EFA275` | Graphics and composition (try_gfx) | Handled | — | [gfx.rs:969](../../crates/crysirror-majiro/src/adapter/gfx.rs#L969) |
| `0x57F6310D` | Text and message windows (try_fontout) | Handled | — | [text.rs:270](../../crates/crysirror-majiro/src/adapter/text.rs#L270) |
| `0x58B31737` | Graphics and composition (try_gfx) | Handled | — | [gfx.rs:1389](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1389) |
| `0x5A981E41` | System commands (try_syscall) | Handled | — | [syscalls.rs:815](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L815) |
| `0x5B87A41D` | Execute try console branch | Handled | — | [console.rs:260](../../crates/crysirror-majiro/src/adapter/console.rs#L260) |
| `0x5BF8F175` | System commands (try_syscall) | Handled | — | [syscalls.rs:749](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L749) |
| `0x5C9ED743` | Execute try console branch | Handled | — | [console.rs:307](../../crates/crysirror-majiro/src/adapter/console.rs#L307) |
| `0x5E4E3063` | Graphics and composition (try_gfx) | Handled | — | [gfx.rs:1001](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1001) |
| `0x5EB974F7` | Graphics and composition (try_gfx) | Handled | — | [gfx.rs:1250](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1250) |
| `0x5FDCCCEE` | Execute try console branch | Handled | — | [console.rs:353](../../crates/crysirror-majiro/src/adapter/console.rs#L353) |
| `0x60085FA4` | Native call (fast_mode_native) | Handled | — | [fast_mode.rs:343](../../crates/crysirror-majiro/src/adapter/fast_mode.rs#L343) |
| `0x6008BEBB` | Input (read_input_file) | Handled | — | [input_files.rs:141](../../crates/crysirror-majiro/src/adapter/input_files.rs#L141) |
| `0x619DE833` | System commands (try_syscall) | Handled | — | [syscalls.rs:475](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L475) |
| `0x61BAE53C` | Graphics and composition (try_gfx) | Handled | — | [gfx.rs:1056](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1056) |
| `0x62374297` | System commands (try_syscall) | Handled | — | [syscalls.rs:478](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L478) |
| `0x63C341B1` | Graphics and composition (try_gfx) | Handled | — | [gfx.rs:1596](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1596) |
| `0x63F660A2` | YUKIIRO44F9E0/ return the stored sprite width/height, not the current rotated/zoomed temporary bitmap dimensions | Handled | — | [gfx.rs:1401](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1401) |
| `0x6501CC30` | Native call (native) | Handled | — | [adapter.rs:378](../../crates/crysirror-majiro/src/adapter.rs#L378) |
| `0x65075141` | Save/load (try_saves) | Handled | — | [saves.rs:189](../../crates/crysirror-majiro/src/adapter/saves.rs#L189) |
| `0x6584F13E` | / returns the owner-pass cached word, not4108C0's fresh real-time mapping. Merely querying does not advance time | Handled | — | [clock_policy.rs:114](../../crates/crysirror-majiro/src/adapter/clock_policy.rs#L114) |
| `0x65F2E980` | Graphics and composition (try_gfx) | Handled | — | [gfx.rs:983](../../crates/crysirror-majiro/src/adapter/gfx.rs#L983) |
| `0x661AFB43` | Graphics and composition (try_gfx) | Handled | — | [gfx.rs:1666](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1666) |
| `0x6621F84D` | System commands (try_syscall) | Handled | — | [syscalls.rs:521](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L521) |
| `0x68EF2987` | Graphics and composition (try_gfx) | Handled | — | [gfx.rs:1442](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1442) |
| `0x69073588` | Graphics and composition (try_gfx) | Handled | — | [gfx.rs:1552](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1552) |
| `0x69DB4512` | System commands (try_syscall) | Handled | — | [syscalls.rs:384](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L384) |
| `0x6B4A7873` | Graphics and composition (try_gfx) | Handled | — | [gfx.rs:1430](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1430) |
| `0x6CFC884A` | System commands (try_syscall) | Handled | — | [syscalls.rs:344](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L344) |
| `0x6DE6CDFC` | Graphics and composition (try_gfx) | Handled | — | [gfx.rs:1313](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1313) |
| `0x6E6C641A` | Native call (native) | Handled | — | [adapter.rs:338](../../crates/crysirror-majiro/src/adapter.rs#L338) |
| `0x6E83677A` | variables / flags | Handled | — | [syscalls.rs:424](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L424) |
| `0x6EAEC2A0` | System commands (try_syscall) | Handled | — | [syscalls.rs:827](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L827) |
| `0x6ED38886` | System commands (try_syscall) | Handled | — | [syscalls.rs:824](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L824) |
| `0x6F384A3D` | Graphics and composition (try_gfx) | Handled | — | [gfx.rs:1690](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1690) |
| `0x6F769889` | System commands (try_syscall) | Handled | — | [syscalls.rs:413](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L413) |
| `0x70612DB5` | Graphics and composition (try_gfx) | Handled | — | [gfx.rs:1036](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1036) |
| `0x708B0256` | System commands (try_syscall) | Handled | — | [syscalls.rs:720](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L720) |
| `0x718EF651` | control | Handled | — | [syscalls.rs:480](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L480) |
| `0x721FA5B7` | Graphics and composition (try_gfx) | Handled | — | [gfx.rs:1596](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1596) |
| `0x72DB7ABF` | System commands (try_syscall) | Handled | — | [syscalls.rs:620](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L620) |
| `0x76EE6C90` | Control flow (native_thread_control) | Handled | — | [events.rs:95](../../crates/crysirror-majiro/src/machine/events.rs#L95) |
| `0x77527EF5` | System commands (try_syscall) | Handled | — | [syscalls.rs:780](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L780) |
| `0x777AA894` | System commands (try_syscall) | Handled | — | [syscalls.rs:778](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L778) |
| `0x78A6A112` | System commands (try_syscall) | Handled | — | [syscalls.rs:525](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L525) |
| `0x7A7B6ED4` | Native call (native) | Handled | — | [adapter.rs:309](../../crates/crysirror-majiro/src/adapter.rs#L309) |
| `0x7BFFC5B1` | Graphics and composition (try_gfx) | Handled | — | [gfx.rs:1552](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1552) |
| `0x7C3B36C5` | Save/load (try_saves) | Handled | — | [saves.rs:137](../../crates/crysirror-majiro/src/adapter/saves.rs#L137) |
| `0x7E277952` | Graphics and composition (try_gfx) | Handled | — | [gfx.rs:1047](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1047) |
| `0x80177286` | Native call (native) | Handled | — | [adapter.rs:371](../../crates/crysirror-majiro/src/adapter.rs#L371) |
| `0x801D7AF9` | System commands (try_syscall) | Handled | — | [syscalls.rs:785](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L785) |
| `0x804FF66D` | Audio playback/control (native_audio_wait) | Handled | — | [audio_wait.rs:20](../../crates/crysirror-majiro/src/adapter/audio_wait.rs#L20) |
| `0x806261CE` | 's inline case reads the first payload as a float word; the dispatcher consumes any further operands without reading th… | Handled | — | [clock_policy.rs:96](../../crates/crysirror-majiro/src/adapter/clock_policy.rs#L96) |
| `0x818A4B92` | Native call (fast_mode_native) | Handled | — | [fast_mode.rs:274](../../crates/crysirror-majiro/src/adapter/fast_mode.rs#L274) |
| `0x81CE0485` | Native call (native) | Handled | — | [adapter.rs:339](../../crates/crysirror-majiro/src/adapter.rs#L339) |
| `0x824D124F` | System commands (try_syscall) | Handled | — | [syscalls.rs:562](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L562) |
| `0x82FD765B` | System commands (try_syscall) | Handled | — | [syscalls.rs:523](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L523) |
| `0x838DE99B` | System commands (try_syscall) | Handled | — | [syscalls.rs:556](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L556) |
| `0x83A53FFA` | Audio playback/control (audio_control) | Handled | — | [native_audio.rs:46](../../crates/crysirror-majiro/src/adapter/native_audio.rs#L46) |
| `0x83D81F59` | pages | Handled | — | [gfx.rs:957](../../crates/crysirror-majiro/src/adapter/gfx.rs#L957) |
| `0x86E933D1` | Native call (fast_mode_native) | Handled | — | [fast_mode.rs:318](../../crates/crysirror-majiro/src/adapter/fast_mode.rs#L318) |
| `0x86F0F608` | Native call (fast_mode_native) | Handled | — | [fast_mode.rs:276](../../crates/crysirror-majiro/src/adapter/fast_mode.rs#L276) |
| `0x87113971` | Graphics and composition (try_gfx) | Partial | Unknown subselectors warn and continue | [gfx.rs:1097](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1097) |
| `0x8838FD56` | YUKIIRO44DF60 sets sprite+32: release owns its source page (), independently of visibility nesting/transform state | Handled | — | [gfx.rs:1417](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1417) |
| `0x884949E0` | sounds 1..4 | Handled | — | [syscalls.rs:717](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L717) |
| `0x88B4B26C` | System commands (try_syscall) | Handled | — | [syscalls.rs:525](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L525) |
| `0x89018DD1` | Save/load (try_saves) | Handled | — | [saves.rs:171](../../crates/crysirror-majiro/src/adapter/saves.rs#L171) |
| `0x892E9B20` | input | Handled | — | [syscalls.rs:606](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L606) |
| `0x89CE8C53` | System commands (try_syscall) | Handled | — | [syscalls.rs:562](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L562) |
| `0x8AEF9167` | System commands (try_syscall) | Handled | — | [syscalls.rs:373](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L373) |
| `0x8C283F7F` | Native call (native) | Handled | — | [adapter.rs:320](../../crates/crysirror-majiro/src/adapter.rs#L320) |
| `0x8C7F4C1E` | Native call (native) | Handled | — | [adapter.rs:328](../../crates/crysirror-majiro/src/adapter.rs#L328) |
| `0x8D84B83F` | Graphics and composition (try_gfx) | Handled | — | [gfx.rs:982](../../crates/crysirror-majiro/src/adapter/gfx.rs#L982) |
| `0x8D8FB32E` | System commands (try_syscall) | Handled | — | [syscalls.rs:501](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L501) |
| `0x8E8167F2` | System commands (try_syscall) | Handled | — | [syscalls.rs:790](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L790) |
| `0x8F905B3F` | System commands (try_syscall) | Handled | — | [syscalls.rs:815](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L815) |
| `0x8FE5D6E7` | Native call (fast_mode_native) | Handled | — | [fast_mode.rs:306](../../crates/crysirror-majiro/src/adapter/fast_mode.rs#L306) |
| `0x90816C6E` | Text and message windows (try_fontout) | Handled | — | [text.rs:265](../../crates/crysirror-majiro/src/adapter/text.rs#L265) |
| `0x90D5298A` | voice | Handled | — | [syscalls.rs:761](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L761) |
| `0x9143DC9E` | System commands (try_syscall) | Handled | — | [syscalls.rs:514](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L514) |
| `0x924EE3EB` | Execute try console branch | Handled | — | [console.rs:391](../../crates/crysirror-majiro/src/adapter/console.rs#L391) |
| `0x926C13E5` | System commands (try_syscall) | Handled | — | [syscalls.rs:735](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L735) |
| `0x934D927B` | Execute try console branch | Handled | — | [console.rs:295](../../crates/crysirror-majiro/src/adapter/console.rs#L295) |
| `0x959B0A16` | Native call (native) | Handled | — | [adapter.rs:284](../../crates/crysirror-majiro/src/adapter.rs#L284) |
| `0x95FF28F2` | System commands (try_syscall) | Handled | — | [syscalls.rs:514](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L514) |
| `0x96B07AEE` | arrays | Handled | — | [syscalls.rs:331](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L331) |
| `0x979D1F16` | Execute try console branch | Handled | — | [console.rs:319](../../crates/crysirror-majiro/src/adapter/console.rs#L319) |
| `0x97BD691D` | System commands (try_syscall) | Handled | — | [syscalls.rs:823](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L823) |
| `0x99A5DE25` | System commands (try_syscall) | Handled | — | [syscalls.rs:443](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L443) |
| `0x9A1ACECA` | Native call (native) | Handled | — | [adapter.rs:378](../../crates/crysirror-majiro/src/adapter.rs#L378) |
| `0x9AC11F47` | Graphics and composition (try_gfx) | Handled | — | [gfx.rs:1212](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1212) |
| `0x9EF1DDC3` | Native call (fast_mode_native) | Handled | — | [fast_mode.rs:275](../../crates/crysirror-majiro/src/adapter/fast_mode.rs#L275) |
| `0x9FB53FBE` | Control flow (native_thread_control) | Handled | — | [events.rs:126](../../crates/crysirror-majiro/src/machine/events.rs#L126) |
| `0xA0B0FEAD` | System commands (try_syscall) | Handled | — | [syscalls.rs:779](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L779) |
| `0xA2743878` | Execute try console branch | Handled | — | [console.rs:319](../../crates/crysirror-majiro/src/adapter/console.rs#L319) |
| `0xA361D9F7` | Graphics and composition (try_gfx) | Handled | — | [gfx.rs:1180](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1180) |
| `0xA549E852` | Save/load (try_saves) | Handled | — | [saves.rs:193](../../crates/crysirror-majiro/src/adapter/saves.rs#L193) |
| `0xA589DBD1` | System commands (try_syscall) | Handled | — | [syscalls.rs:563](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L563) |
| `0xA62AA5EB` | Native call (fast_mode_native) | Handled | — | [fast_mode.rs:266](../../crates/crysirror-majiro/src/adapter/fast_mode.rs#L266) |
| `0xA79731A2` | System commands (try_syscall) | Partial | movie_play is skipped (movie decoding not connected for Majiro) | [syscalls.rs:527](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L527) |
| `0xA7C6E918` | System commands (try_syscall) | Handled | — | [syscalls.rs:681](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L681) |
| `0xA93C9856` | Control flow (native_thread_control) | Handled | — | [events.rs:78](../../crates/crysirror-majiro/src/machine/events.rs#L78) |
| `0xAB13BE44` | System commands (try_syscall) | Handled | — | [syscalls.rs:581](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L581) |
| `0xAC796720` | System commands (try_syscall) | Handled | — | [syscalls.rs:525](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L525) |
| `0xACCF4004` | Native call (native_flags_native) | Handled | — | [native_flags.rs:61](../../crates/crysirror-majiro/src/adapter/native_flags.rs#L61) |
| `0xAD74D9AC` | Native call (native) | Handled | — | [adapter.rs:298](../../crates/crysirror-majiro/src/adapter.rs#L298) |
| `0xAE497F1A` | Control flow (native_thread_control) | Handled | — | [events.rs:118](../../crates/crysirror-majiro/src/machine/events.rs#L118) |
| `0xB0C8F550` | System commands (try_syscall) | Handled | — | [syscalls.rs:383](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L383) |
| `0xB0CE081A` | System commands (try_syscall) | Handled | — | [syscalls.rs:562](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L562) |
| `0xB268F04F` | System commands (try_syscall) | Handled | — | [syscalls.rs:719](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L719) |
| `0xB5A1E3C9` | Execute try console branch | Handled | — | [console.rs:266](../../crates/crysirror-majiro/src/adapter/console.rs#L266) |
| `0xB7E3E81C` | 's absent argument is1. deliberately preserves the last input timestamp, so the350ms interval is not rearmed here | Handled | — | [native_flags.rs:45](../../crates/crysirror-majiro/src/adapter/native_flags.rs#L45) |
| `0xB84C9953` | System commands (try_syscall) | Handled | — | [syscalls.rs:621](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L621) |
| `0xBA383010` | Graphics and composition (try_gfx) | Handled | — | [gfx.rs:1712](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1712) |
| `0xBA81EF8D` | Execute try console branch | Handled | — | [console.rs:379](../../crates/crysirror-majiro/src/adapter/console.rs#L379) |
| `0xBBF35806` | System commands (try_syscall) | Handled | — | [syscalls.rs:524](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L524) |
| `0xBC42CA9A` | Native call (fast_mode_native) | Handled | — | [fast_mode.rs:283](../../crates/crysirror-majiro/src/adapter/fast_mode.rs#L283) |
| `0xC102BCC3` | Native call (fast_mode_native) | Handled | — | [fast_mode.rs:322](../../crates/crysirror-majiro/src/adapter/fast_mode.rs#L322) |
| `0xC21F8B49` | Native call (fast_mode_native) | Handled | — | [fast_mode.rs:297](../../crates/crysirror-majiro/src/adapter/fast_mode.rs#L297) |
| `0xC29B30E3` | Graphics and composition (try_gfx) | Handled | — | [gfx.rs:1519](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1519) |
| `0xC2C3C4F4` | Graphics and composition (try_gfx) | Handled | — | [gfx.rs:1358](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1358) |
| `0xC32E286F` | System commands (try_syscall) | Handled | — | [syscalls.rs:476](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L476) |
| `0xC438BA98` | System commands (try_syscall) | Handled | — | [syscalls.rs:660](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L660) |
| `0xC50DFD06` | Graphics and composition (try_gfx) | Handled | — | [gfx.rs:973](../../crates/crysirror-majiro/src/adapter/gfx.rs#L973) |
| `0xC5270429` | Execute try console branch | Handled | — | [console.rs:408](../../crates/crysirror-majiro/src/adapter/console.rs#L408) |
| `0xC6282F19` | Native call (native) | Handled | — | [adapter.rs:350](../../crates/crysirror-majiro/src/adapter.rs#L350) |
| `0xC68C011A` | Native call (fast_mode_native) | Handled | — | [fast_mode.rs:314](../../crates/crysirror-majiro/src/adapter/fast_mode.rs#L314) |
| `0xC9B362D0` | System commands (try_syscall) | Handled | — | [syscalls.rs:554](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L554) |
| `0xCA30043A` | Native call (native) | Handled | — | [adapter.rs:272](../../crates/crysirror-majiro/src/adapter.rs#L272) |
| `0xCB665AC0` | System commands (try_syscall) | Handled | — | [syscalls.rs:803](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L803) |
| `0xCC3E0342` | Execute try console branch | Handled | — | [console.rs:295](../../crates/crysirror-majiro/src/adapter/console.rs#L295) |
| `0xCD290AEF` | variables / flags | Handled | — | [syscalls.rs:424](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L424) |
| `0xCD71FE7A` | Graphics and composition (try_gfx) | Handled | — | [gfx.rs:1358](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1358) |
| `0xCE94E6CA` | Graphics and composition (try_gfx) | Handled | — | [gfx.rs:989](../../crates/crysirror-majiro/src/adapter/gfx.rs#L989) |
| `0xCF2B1E50` | Graphics and composition (try_gfx) | Handled | — | [gfx.rs:1309](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1309) |
| `0xCF35F0E3` | Native call (native) | Handled | — | [adapter.rs:327](../../crates/crysirror-majiro/src/adapter.rs#L327) |
| `0xD011FF32` | Audio playback/control (audio_control) | Handled | — | [native_audio.rs:56](../../crates/crysirror-majiro/src/adapter/native_audio.rs#L56) |
| `0xD01BE374` | Graphics and composition (try_gfx) | Handled | — | [gfx.rs:1062](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1062) |
| `0xD1F672C7` | Audio playback/control (native_audio_wait) | Handled | — | [audio_wait.rs:18](../../crates/crysirror-majiro/src/adapter/audio_wait.rs#L18) |
| `0xD3C06820` | Graphics and composition (try_gfx) | Handled | — | [gfx.rs:997](../../crates/crysirror-majiro/src/adapter/gfx.rs#L997) |
| `0xD4066E31` | Execute try console branch | Handled | — | [console.rs:311](../../crates/crysirror-majiro/src/adapter/console.rs#L311) |
| `0xD6333B3C` | Native call (native) | Handled | — | [adapter.rs:290](../../crates/crysirror-majiro/src/adapter.rs#L290) |
| `0xD7771B72` | Graphics and composition (try_gfx) | Handled | — | [gfx.rs:1465](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1465) |
| `0xD8279A9B` | Audio playback/control (native_audio_wait) | Handled | — | [audio_wait.rs:17](../../crates/crysirror-majiro/src/adapter/audio_wait.rs#L17) |
| `0xDA675785` | Graphics and composition (try_gfx) | Handled | — | [gfx.rs:1196](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1196) |
| `0xDAD96289` | Graphics and composition (try_gfx) | Handled | — | [gfx.rs:969](../../crates/crysirror-majiro/src/adapter/gfx.rs#L969) |
| `0xDCA7DA63` | YUKIIRO403950: retained media lookup with native Retry/Cancel | Handled | — | [adapter.rs:282](../../crates/crysirror-majiro/src/adapter.rs#L282) |
| `0xDCC706EF` | Graphics and composition (try_gfx) | Handled | — | [gfx.rs:1398](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1398) |
| `0xDD2056F6` | System commands (try_syscall) | Handled | — | [syscalls.rs:525](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L525) |
| `0xDE1E2ED0` | System commands (try_syscall) | Handled | — | [syscalls.rs:720](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L720) |
| `0xDF644F85` | diagnostics | Handled | — | [syscalls.rs:462](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L462) |
| `0xDF91CFC5` | Native call (native) | Handled | — | [adapter.rs:364](../../crates/crysirror-majiro/src/adapter.rs#L364) |
| `0xDF9F7749` | Native call (native_flags_native) | Handled | — | [native_flags.rs:58](../../crates/crysirror-majiro/src/adapter/native_flags.rs#L58) |
| `0xDFD5599E` | Graphics and composition (try_gfx) | Handled | — | [gfx.rs:1291](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1291) |
| `0xDFDBA8E4` | Graphics and composition (try_gfx) | Handled | — | [gfx.rs:1495](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1495) |
| `0xE06200C4` | Native call (native) | Handled | — | [adapter.rs:320](../../crates/crysirror-majiro/src/adapter.rs#L320) |
| `0xE119D5BA` | Graphics and composition (try_gfx) | Handled | — | [gfx.rs:1443](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1443) |
| `0xE186D19A` | Text and message windows (try_fontout) | Handled | — | [text.rs:251](../../crates/crysirror-majiro/src/adapter/text.rs#L251) |
| `0xE3605CFA` | System commands (try_syscall) | Handled | — | [syscalls.rs:478](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L478) |
| `0xE4E3B40D` | Native call (native) | Handled | — | [adapter.rs:289](../../crates/crysirror-majiro/src/adapter.rs#L289) |
| `0xE5C70196` | Native call (native) | Handled | — | [adapter.rs:368](../../crates/crysirror-majiro/src/adapter.rs#L368) |
| `0xE712FEC1` | System commands (try_syscall) | Handled | — | [syscalls.rs:750](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L750) |
| `0xE7B90DF5` | System commands (try_syscall) | Handled | — | [syscalls.rs:711](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L711) |
| `0xE91B0504` | System commands (try_syscall) | Handled | — | [syscalls.rs:525](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L525) |
| `0xEB5CC468` | System commands (try_syscall) | Handled | — | [syscalls.rs:443](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L443) |
| `0xED84E320` | System commands (try_syscall) | Handled | — | [syscalls.rs:721](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L721) |
| `0xEE154520` | Native call (native) | Handled | — | [adapter.rs:365](../../crates/crysirror-majiro/src/adapter.rs#L365) |
| `0xEE2B6D92` | System commands (try_syscall) | Handled | — | [syscalls.rs:346](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L346) |
| `0xEE6B328D` | System commands (try_syscall) | Handled | — | [syscalls.rs:348](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L348) |
| `0xEF4581DC` | sprites | Handled | — | [gfx.rs:1318](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1318) |
| `0xEFD49BC9` | System commands (try_syscall) | Handled | — | [syscalls.rs:619](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L619) |
| `0xF03A9A01` | System commands (try_syscall) | Handled | — | [syscalls.rs:747](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L747) |
| `0xF1097A07` | System commands (try_syscall) | Handled | — | [syscalls.rs:777](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L777) |
| `0xF1F8A206` | Execute try console branch | Handled | — | [console.rs:291](../../crates/crysirror-majiro/src/adapter/console.rs#L291) |
| `0xF2F9DAA8` | Control flow (native_thread_control) | Handled | — | [events.rs:118](../../crates/crysirror-majiro/src/machine/events.rs#L118) |
| `0xF3679C34` | diagnostics | Handled | — | [syscalls.rs:462](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L462) |
| `0xF3D276F1` | Native call (fast_mode_native) | Handled | — | [fast_mode.rs:299](../../crates/crysirror-majiro/src/adapter/fast_mode.rs#L299) |
| `0xF62E3CA7` | System commands (try_syscall) | Handled | — | [syscalls.rs:719](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L719) |
| `0xF63CB36C` | System commands (try_syscall) | Handled | — | [syscalls.rs:634](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L634) |
| `0xF6416E87` | Native call (fast_mode_native) | Partial | Majiro native read-mark profile is not connected | [fast_mode.rs:267](../../crates/crysirror-majiro/src/adapter/fast_mode.rs#L267) |
| `0xF6459905` | Graphics and composition (try_gfx) | Handled | — | [gfx.rs:1430](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1430) |
| `0xF6A05538` | sprites | Handled | — | [gfx.rs:1318](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1318) |
| `0xF8004993` | Save/load (try_saves) | Handled | — | [saves.rs:150](../../crates/crysirror-majiro/src/adapter/saves.rs#L150) |
| `0xF80BC9B6` | Graphics and composition (try_gfx) | Handled | — | [gfx.rs:1508](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1508) |
| `0xF86FC950` | System commands (try_syscall) | Handled | — | [syscalls.rs:567](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L567) |
| `0xF8D18340` | hit regions | Handled | — | [syscalls.rs:644](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L644) |
| `0xF8D8925B` | Shared source-clock observation; a completed first pass is distinct from both decoder preparation and the loop setting | Handled | — | [native_audio.rs:63](../../crates/crysirror-majiro/src/adapter/native_audio.rs#L63) |
| `0xF8FD08F6` | Graphics and composition (try_gfx) | Handled | — | [gfx.rs:983](../../crates/crysirror-majiro/src/adapter/gfx.rs#L983) |
| `0xF94B3586` | pages | Handled | — | [gfx.rs:957](../../crates/crysirror-majiro/src/adapter/gfx.rs#L957) |
| `0xF9705332` | Graphics and composition (try_gfx) | Handled | — | [gfx.rs:1012](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1012) |
| `0xF9A3B675` | variables / flags | Handled | — | [syscalls.rs:424](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L424) |
| `0xFAC4F361` | Native call (fast_mode_native) | Handled | — | [fast_mode.rs:277](../../crates/crysirror-majiro/src/adapter/fast_mode.rs#L277) |
| `0xFD128A61` | Graphics and composition (try_gfx) | Handled | — | [gfx.rs:1087](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1087) |
| `0xFDDF6C40` | Graphics and composition (try_gfx) | Handled | — | [gfx.rs:1063](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1063) |
| `0xFEF981D4` | YUKIIRO44F9E0/ return the stored sprite width/height, not the current rotated/zoomed temporary bitmap dimensions | Handled | — | [gfx.rs:1401](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1401) |
| `0xFF75CD29` | System commands (try_syscall) | Handled | — | [syscalls.rs:531](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L531) |
| `0xFF7C52E6` | Graphics and composition (try_gfx) | Handled | — | [gfx.rs:1474](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1474) |

## Subselectors and conditional dispatch

Patterns retain their source namespace and guards. `_` is the fallback of that table. Boolean/optional results follow the function's return contract. Call-site names identify the implementation to inspect, including indirect service calls.

### adapter.rs / MajiroVm::native

```rust
fn native(&mut self, h: &mut dyn HostServices, event: &Event) -> Result<Option<Vec<Value>>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under *hash=0x266a3c79\|0xad74d9ac` | `*hash==0xad74d9ac` | crate::byte_policy::lower | Handled | [306](../../crates/crysirror-majiro/src/adapter.rs#L306) |
| `*hash; under *hash=0x111bd910\|0xe5c70196\|0xee154520` | `_` | Local state/value operations | Handled | [368](../../crates/crysirror-majiro/src/adapter.rs#L368) |
| `condition; under *hash=0x0c93fcb4\|0x6501cc30\|0x9a1aceca` | `*hash==0x9a1aceca` | encoding_rs::SHIFT_JIS.decode_without_bom_handling_and_without_replacement | Handled | [380](../../crates/crysirror-majiro/src/adapter.rs#L380) |
| `condition; under *hash=0x0c93fcb4\|0x6501cc30\|0x9a1aceca` | `*hash==0x0c93fcb4` | Local state/value operations | Handled | [384](../../crates/crysirror-majiro/src/adapter.rs#L384) |
| `*hash` | `_` | Local state/value operations | Unsupported: unsupported original Majiro syscall {hash:#010x}; pending call was not acknowledged | [392](../../crates/crysirror-majiro/src/adapter.rs#L392) |

### adapter.rs / MajiroVm::run

```rust
fn run(&mut self, h: &mut dyn HostServices, budget: u32) -> Result<VmYield>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(event,Event::Native {hash:0x10db0a43\|0xcd00d280\|0x9627ddc3\|0x0edefb0b\|0xedefb0b0,..})` | self.try_library | Handled | [585](../../crates/crysirror-majiro/src/adapter.rs#L585) |
| `condition` | `let Event::Native { hash: 0xcf35f0e3, arguments, returns, .. } = &event` | Value::Integer, self.round_yields.remove, self.round_yields.insert | Handled | [594](../../crates/crysirror-majiro/src/adapter.rs#L594) |
| `condition` | `matches!(&event,Event::Native{hash:0xe4e3b40d,..})` | Local state/value operations | Handled | [667](../../crates/crysirror-majiro/src/adapter.rs#L667) |

### adapter/audio_wait.rs / MajiroVm::native_audio_wait

```rust
fn native_audio_wait(
        &mut self,
        machine: &mut Machine,
        hash: u32,
        host: &mut dyn HostServices,
    ) -> Result<Option<syscalls::Out>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `hash` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [23](../../crates/crysirror-majiro/src/adapter/audio_wait.rs#L23) |
| `condition` | `self.timed_waits.get(&key).is_some_and(\|deadline\| { (now as u32) < (*deadline as u32) && now.wrapping_sub(*deadline).wrapping_abs() <= 43_200_000 })` | Local state/value operations | Handled | [29](../../crates/crysirror-majiro/src/adapter/audio_wait.rs#L29) |

### adapter/clock_policy.rs / MajiroVm::clock_native

```rust
fn clock_native(
        &mut self,
        hash: u32,
        arguments: &[Value],
        returns: bool,
        host: &mut dyn HostServices,
    ) -> Result<Option<Vec<Value>>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `hash` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [115](../../crates/crysirror-majiro/src/adapter/clock_policy.rs#L115) |

### adapter/console.rs / MajiroVm::try_console

```rust
fn try_console(
        &mut self,
        machine: &mut Machine,
        event: &Event,
        host: &mut dyn HostServices,
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `control.is_none() && !matches!( hash, 0xb5a1e3c9 \| 0x403960f8 \| 0xf1f8a206 \| 0x5131e234 \| 0xcc3e0342 \| 0x0e42730d \| 0x934d927b \| 0x30fa2a29 \| 0x5c9ed743 \| 0xd4066e31 \| 0x4133d7a9 \| 0xa2743878 \| 0x979d1f16 \| 0x06b3c8ac \| 0x5fdcccee \| 0x924ee3eb \| 0xc5270429 \| 0x1ec3c6bb \| 0x5b87a41d \| 0xba81ef8d \| 0x32d0236c )` | Local state/value operations | Handled | [218](../../crates/crysirror-majiro/src/adapter/console.rs#L218) |
| `condition; under hash=0xd4066e31 \| 0x4133d7a9` | `hash == 0xd4066e31` | Local state/value operations | Handled | [313](../../crates/crysirror-majiro/src/adapter/console.rs#L313) |
| `hash` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [412](../../crates/crysirror-majiro/src/adapter/console.rs#L412) |

### adapter/controllers.rs / aliases

```rust
fn aliases(key: usize) -> &'static [usize]
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `key` | `27` | Local state/value operations | Handled | [59](../../crates/crysirror-majiro/src/adapter/controllers.rs#L59) |
| `key` | `13` | Local state/value operations | Handled | [60](../../crates/crysirror-majiro/src/adapter/controllers.rs#L60) |
| `key` | `38` | Local state/value operations | Handled | [61](../../crates/crysirror-majiro/src/adapter/controllers.rs#L61) |
| `key` | `40` | Local state/value operations | Handled | [62](../../crates/crysirror-majiro/src/adapter/controllers.rs#L62) |
| `key` | `37` | Local state/value operations | Handled | [63](../../crates/crysirror-majiro/src/adapter/controllers.rs#L63) |
| `key` | `39` | Local state/value operations | Handled | [64](../../crates/crysirror-majiro/src/adapter/controllers.rs#L64) |
| `key` | `_` | Local state/value operations | Handled | [65](../../crates/crysirror-majiro/src/adapter/controllers.rs#L65) |

### adapter/controllers.rs / syscalls::Input::update_sources

```rust
fn update_sources(&mut self, sources: &InputSourcesFrame, now: u64) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `self.joystick.mode` | `1` | Local state/value operations | Handled | [94](../../crates/crysirror-majiro/src/adapter/controllers.rs#L94) |
| `self.joystick.mode` | `2` | Local state/value operations | Handled | [95](../../crates/crysirror-majiro/src/adapter/controllers.rs#L95) |
| `self.joystick.mode` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [99](../../crates/crysirror-majiro/src/adapter/controllers.rs#L99) |
| `condition` | `key < 128` | Local state/value operations | Handled | [108](../../crates/crysirror-majiro/src/adapter/controllers.rs#L108) |

### adapter/fast_mode.rs / FastMode::mode

```rust
fn mode(&mut self, mut mode: u32, neko_menu: i32) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `mode == 5` | Local state/value operations | Handled | [52](../../crates/crysirror-majiro/src/adapter/fast_mode.rs#L52) |
| `mode` | `0 \| 7` | Local state/value operations | Handled | [56](../../crates/crysirror-majiro/src/adapter/fast_mode.rs#L56) |
| `mode` | `1 if self.preference != 0` | Local state/value operations | Handled | [61](../../crates/crysirror-majiro/src/adapter/fast_mode.rs#L61) |
| `mode` | `1 \| 4` | Local state/value operations | Handled | [66](../../crates/crysirror-majiro/src/adapter/fast_mode.rs#L66) |
| `mode` | `2` | Local state/value operations | Handled | [71](../../crates/crysirror-majiro/src/adapter/fast_mode.rs#L71) |
| `mode` | `3` | Local state/value operations | Handled | [75](../../crates/crysirror-majiro/src/adapter/fast_mode.rs#L75) |
| `mode` | `6` | Local state/value operations | Handled | [80](../../crates/crysirror-majiro/src/adapter/fast_mode.rs#L80) |
| `mode` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [86](../../crates/crysirror-majiro/src/adapter/fast_mode.rs#L86) |

### adapter/fast_mode.rs / MajiroVm::fast_mode_native

```rust
fn fast_mode_native(
        &mut self,
        m: &mut Machine,
        hash: u32,
        a: &[Value],
        returns: bool,
        host: &mut dyn HostServices,
    ) -> Result<Option<Vec<Value>>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!matches!( hash, 0xa62aa5eb \| 0x818a4b92 \| 0x9ef1ddc3 \| 0x86f0f608 \| 0xfac4f361 \| 0x4b306c5c \| 0xbc42ca9a \| 0x3c84073c \| 0xc21f8b49 \| 0x38721a26 \| 0xf3d276f1 \| 0x8fe5d6e7 \| 0x45acd4a7 \| 0xc102bcc3 \| 0xc68c011a \| 0x86e933d1 \| 0x3eabf498 \| 0xf6416e87 \| 0x60085fa4 \| 0x44bc7555 )` | Local state/value operations | Handled | [234](../../crates/crysirror-majiro/src/adapter/fast_mode.rs#L234) |
| `hash; under hash=0x818a4b92 \| 0x9ef1ddc3 \| 0x86f0f608 \| 0xfac4f361 \| 0x4b306c5c` | `_` | Local state/value operations | Handled | [278](../../crates/crysirror-majiro/src/adapter/fast_mode.rs#L278) |
| `hash` | `_` | Local state/value operations | Handled | [364](../../crates/crysirror-majiro/src/adapter/fast_mode.rs#L364) |

### adapter/fast_mode.rs / MajiroVm::update_fast_mode_input

```rust
fn update_fast_mode_input(&mut self, host: &mut dyn HostServices) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `key == 16` | Local state/value operations | Handled | [137](../../crates/crysirror-majiro/src/adapter/fast_mode.rs#L137) |

### adapter/gfx.rs / Gfx::redraw

```rust
fn redraw(&mut self, page: u32) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `a[0] < tile[0]` | Local state/value operations | Handled | [404](../../crates/crysirror-majiro/src/adapter/gfx.rs#L404) |
| `condition` | `a[1] < tile[1]` | Local state/value operations | Handled | [409](../../crates/crysirror-majiro/src/adapter/gfx.rs#L409) |

### adapter/gfx.rs / MajiroVm::try_gfx

```rust
fn try_gfx(
        &mut self,
        hash: u32,
        a: &[Value],
        host: &mut dyn HostServices,
    ) -> Result<Option<GfxOut>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under hash=0x83d81f59 \| 0xf94b3586` | `hash == 0xf94b3586` | Plane::new | Handled | [961](../../crates/crysirror-majiro/src/adapter/gfx.rs#L961) |
| `condition; under hash=0xf8fd08f6 \| 0x65f2e980` | `hash == 0xf8fd08f6` | Local state/value operations | Handled | [985](../../crates/crysirror-majiro/src/adapter/gfx.rs#L985) |
| `condition; under hash=0x539b07bc \| 0xce94e6ca / self.load_picture(host, &stem)=Ok(p)` | `hash == 0x539b07bc` | Local state/value operations | Handled | [993](../../crates/crysirror-majiro/src/adapter/gfx.rs#L993) |
| `condition; under hash=0x5e4e3063` | `page.plane.format == PlaneFormat::Bgr24 && page.mode != int(a, 1)` | Local state/value operations | Handled | [1004](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1004) |
| `hash; under hash=0xd01be374 \| 0xfddf6c40 \| 0x61bae53c` | `_` | u32::from | Handled | [1064](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1064) |
| `condition; under hash=0x56bbba3a \| 0x87113971 \| 0x30636d6e` | `hash==0x30636d6e` | Local state/value operations | Handled | [1103](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1103) |
| `hash; under hash=0x56bbba3a \| 0x87113971 \| 0x30636d6e` | `_` | planes::masked_blend, planes::blend | Handled | [1162](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1162) |
| `condition; under hash=0xef4581dc \| 0xf6a05538` | `hash == 0xf6a05538` | Local state/value operations | Handled | [1353](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1353) |
| `condition; under hash=0xc2c3c4f4 \| 0xcd71fe7a` | `hash == 0xcd71fe7a` | Plane::new | Handled | [1377](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1377) |
| `condition; under hash=0x63f660a2\|0xfef981d4` | `hash==0x63f660a2` | Local state/value operations | Handled | [1403](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1403) |
| `condition; under hash=0x40258a56 \| 0xd7771b72` | `hash == 0x40258a56` | Local state/value operations | Handled | [1467](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1467) |
| `hash; under hash=0xf80bc9b6 \| 0xc29b30e3 \| 0x52819912 \| 0xdfdba8e4` | `_` | Local state/value operations | Handled | [1538](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1538) |
| `condition; under hash=0x01b4517c \| 0x6f384a3d` | `hash == 0x01b4517c` | Local state/value operations | Handled | [1707](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1707) |
| `condition; under hash=0x01b4517c \| 0x6f384a3d` | `hash == 0x01b4517c` | Local state/value operations | Handled | [1708](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1708) |
| `hash` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [1732](../../crates/crysirror-majiro/src/adapter/gfx.rs#L1732) |

### adapter/input_files.rs / MajiroVm::read_input_file

```rust
fn read_input_file(
        &mut self,
        hash: u32,
        args: &[Value],
        returns: bool,
    ) -> Result<Option<Vec<Value>>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `hash == 0xd6333b3c` | self.input_files.remove | Handled | [124](../../crates/crysirror-majiro/src/adapter/input_files.rs#L124) |
| `hash` | `_` | Local state/value operations | Unsupported: Majiro unknown input-file operation | [142](../../crates/crysirror-majiro/src/adapter/input_files.rs#L142) |

### adapter/libraries.rs / ModuleCatalog::prepare

```rust
fn prepare(
        storage: &dyn ClientStorage,
        script: &str,
        cancel: &Cancellation,
    ) -> Result<Self>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `name.to_ascii_lowercase().ends_with(".arc")` | Local state/value operations | Handled | [37](../../crates/crysirror-majiro/src/adapter/libraries.rs#L37) |
| `condition` | `!key.ends_with(".mjo")` | Local state/value operations | Handled | [71](../../crates/crysirror-majiro/src/adapter/libraries.rs#L71) |

### adapter/media.rs / MajiroVm::media_check

```rust
fn media_check(
        &mut self,
        event: &Event,
        host: &mut dyn HostServices,
    ) -> Result<Option<Vec<Value>>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `kind` | `0` | Local state/value operations | Handled | [86](../../crates/crysirror-majiro/src/adapter/media.rs#L86) |
| `kind` | `2` | Local state/value operations | Handled | [87](../../crates/crysirror-majiro/src/adapter/media.rs#L87) |
| `kind` | `_` | Local state/value operations | Handled | [88](../../crates/crysirror-majiro/src/adapter/media.rs#L88) |

### adapter/native_audio.rs / MajiroVm::audio_control

```rust
fn audio_control(
        &mut self,
        hash: u32,
        arguments: &[Value],
        returns: bool,
        host: &mut dyn HostServices,
    ) -> Result<Option<Vec<Value>>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `hash` | `_` | Local state/value operations | Unsupported: Majiro unreviewed audio control | [71](../../crates/crysirror-majiro/src/adapter/native_audio.rs#L71) |
| `hash` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [80](../../crates/crysirror-majiro/src/adapter/native_audio.rs#L80) |

### adapter/native_flags.rs / MajiroVm::native_flags_native

```rust
fn native_flags_native(
        &mut self,
        hash: u32,
        arguments: &[Value],
        returns: bool,
    ) -> Result<Option<Vec<Value>>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `hash` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [67](../../crates/crysirror-majiro/src/adapter/native_flags.rs#L67) |

### adapter/native_flags.rs / MajiroVm::play_native_voice

```rust
fn play_native_voice(
        &mut self,
        name: &[u8],
        repeat: i32,
        console: Option<(i32, bool)>,
        host: &mut dyn HostServices,
    ) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `name.eq_ignore_ascii_case(b"NW")` | Local state/value operations | Handled | [113](../../crates/crysirror-majiro/src/adapter/native_flags.rs#L113) |

### adapter/saves.rs / MajiroVm::try_saves

```rust
fn try_saves(
        &mut self,
        m: &mut Machine,
        hash: u32,
        a: &[Value],
        returns: bool,
        host: &mut dyn HostServices,
    ) -> Result<Option<syscalls::Out>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `hash` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [205](../../crates/crysirror-majiro/src/adapter/saves.rs#L205) |

### adapter/syscalls.rs / MajiroVm::poll_regions

```rust
fn poll_regions(&mut self, m: &mut Machine, returns: bool) -> Result<Out>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(key, 37..=40)` | Local state/value operations | Handled | [870](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L870) |
| `key; under current=Some(cur)` | `38` | Local state/value operations | Handled | [883](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L883) |
| `key; under current=Some(cur)` | `40` | Local state/value operations | Handled | [884](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L884) |
| `key; under current=Some(cur)` | `37` | Local state/value operations | Handled | [885](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L885) |
| `key; under current=Some(cur)` | `_` | Local state/value operations | Handled | [886](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L886) |
| `condition` | `key == 13 \|\| key == 32` | Local state/value operations | Handled | [903](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L903) |
| `condition` | `regions.list[idx].key != 0` | Local state/value operations | Handled | [982](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L982) |

### adapter/syscalls.rs / MajiroVm::try_syscall

```rust
fn try_syscall(
        &mut self,
        m: &mut Machine,
        hash: u32,
        a: &[Value],
        returns: bool,
        host: &mut dyn HostServices,
    ) -> Result<Option<Out>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under hash=0x96b07aee \| 0x1f57b724` | `hash == 0x96b07aee` | Local state/value operations | Handled | [332](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L332) |
| `condition; under hash=0x06906970 \| 0x25401a1f` | `hash==0x06906970` | Local state/value operations | Handled | [369](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L369) |
| `condition; under hash=0x6e83677a \| 0xf9a3b675 \| 0xcd290aef` | `!name.get(1..).is_some_and(\|rest\| rest.contains(&b'@'))` | Local state/value operations | Handled | [426](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L426) |
| `condition; under hash=0x6e83677a \| 0xf9a3b675 \| 0xcd290aef` | `hash == 0xcd290aef` | Local state/value operations | Handled | [430](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L430) |
| `condition; under hash=0xeb5cc468 \| 0x99a5de25` | `hash == 0xeb5cc468` | self.flags.len, self.flags.insert | Handled | [450](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L450) |
| `hash; under hash=0x838de99b \| 0xc9b362d0` | `_` | Local state/value operations | Handled | [557](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L557) |
| `condition; under hash=0x892e9b20 \| 0x008acbc0` | `hash == 0x008acbc0 && bits == 0` | Local state/value operations | Handled | [610](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L610) |
| `condition; under hash=0x4980f82c \| 0x0dc634c4` | `hash == 0x0dc634c4` | Local state/value operations | Handled | [696](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L696) |
| `hash; under hash=0xf62e3ca7 \| 0x708b0256 \| 0xed84e320 \| 0x884949e0 \| 0xb268f04f \| 0xde1e2ed0 \| 0x4311cfa6 \| 0x26dc6566` | `_` | Local state/value operations | Handled | [722](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L722) |
| `condition; under hash=0x546f69c3 \| 0x2e861c51` | `hash == 0x2e861c51` | Local state/value operations | Handled | [740](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L740) |
| `hash; under hash=0x5bf8f175 \| 0xe712fec1 \| 0xf03a9a01` | `_` | Local state/value operations | Handled | [751](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L751) |
| `hash; under hash=0xf8d8925b \| 0xf1097a07 \| 0x777aa894 \| 0xa0b0fead \| 0x77527ef5 \| 0x3d6cf5be` | `_` | Local state/value operations | Handled | [781](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L781) |
| `condition; under hash=0xcb665ac0 \| 0x19fc7cc7` | `hash == 0x19fc7cc7` | Local state/value operations | Handled | [807](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L807) |
| `condition; under hash=0x5a981e41 \| 0x8f905b3f` | `hash == 0x8f905b3f` | Local state/value operations | Handled | [818](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L818) |
| `hash; under hash=0x0c7a0e97 \| 0x6eaec2a0 \| 0x072e738c \| 0x6ed38886` | `_` | Local state/value operations | Handled | [829](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L829) |
| `condition; under hash=0x0c7a0e97 \| 0x6eaec2a0 \| 0x072e738c \| 0x6ed38886` | `let Ok(candidate) = settings.candidate_integer(key, int(a, 0), false)` | Local state/value operations | Handled | [832](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L832) |
| `condition; under hash=0x0c7a0e97 \| 0x6eaec2a0 \| 0x072e738c \| 0x6ed38886` | `hash == 0x0c7a0e97` | self.volume, host.submit_audio | Handled | [837](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L837) |
| `hash` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [843](../../crates/crysirror-majiro/src/adapter/syscalls.rs#L843) |

### adapter/text.rs / MajiroVm::try_fontout

```rust
fn try_fontout(
        &mut self,
        m: &mut Machine,
        hash: u32,
        a: &[Value],
        host: &mut dyn HostServices,
    ) -> Result<Option<Option<i32>>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!matches!(hash, 0xe186d19a \| 0x90816c6e \| 0x57f6310d \| 0x35a89417)` | Local state/value operations | Handled | [241](../../crates/crysirror-majiro/src/adapter/text.rs#L241) |
| `hash` | `_` | self.warn, self.ensure_glyphs, self.draw_text, self.fontout.get_mut | Handled | [278](../../crates/crysirror-majiro/src/adapter/text.rs#L278) |

### machine.rs / integer_binary

```rust
fn integer_binary(op: u16, a: &Value, b: &Value) -> Result<Value>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `op` | `0x100` | Local state/value operations | Handled | [487](../../crates/crysirror-majiro/src/machine.rs#L487) |
| `op` | `0x108` | Local state/value operations | Handled | [491](../../crates/crysirror-majiro/src/machine.rs#L491) |
| `op` | `0x110` | Local state/value operations | Handled | [494](../../crates/crysirror-majiro/src/machine.rs#L494) |
| `op` | `0x118` | Local state/value operations | Handled | [497](../../crates/crysirror-majiro/src/machine.rs#L497) |
| `op` | `0x120` | Local state/value operations | Handled | [498](../../crates/crysirror-majiro/src/machine.rs#L498) |
| `op` | `0x128` | Local state/value operations | Handled | [499](../../crates/crysirror-majiro/src/machine.rs#L499) |
| `op` | `0x130` | Local state/value operations | Handled | [500](../../crates/crysirror-majiro/src/machine.rs#L500) |
| `op` | `0x138` | i32::from | Handled | [501](../../crates/crysirror-majiro/src/machine.rs#L501) |
| `op` | `0x140` | i32::from | Handled | [502](../../crates/crysirror-majiro/src/machine.rs#L502) |
| `op` | `0x148` | i32::from | Handled | [503](../../crates/crysirror-majiro/src/machine.rs#L503) |
| `op` | `0x150` | i32::from | Handled | [504](../../crates/crysirror-majiro/src/machine.rs#L504) |
| `op` | `0x158` | i32::from | Handled | [505](../../crates/crysirror-majiro/src/machine.rs#L505) |
| `op` | `0x160` | i32::from | Handled | [506](../../crates/crysirror-majiro/src/machine.rs#L506) |
| `op` | `0x168` | Local state/value operations | Handled | [507](../../crates/crysirror-majiro/src/machine.rs#L507) |
| `op` | `0x170` | i32::from | Handled | [508](../../crates/crysirror-majiro/src/machine.rs#L508) |
| `op` | `0x178` | i32::from | Handled | [509](../../crates/crysirror-majiro/src/machine.rs#L509) |
| `op` | `0x180` | Local state/value operations | Handled | [510](../../crates/crysirror-majiro/src/machine.rs#L510) |
| `op` | `0x188` | Local state/value operations | Handled | [511](../../crates/crysirror-majiro/src/machine.rs#L511) |
| `op` | `_` | Local state/value operations | Unsupported: Majiro integer operator | [512](../../crates/crysirror-majiro/src/machine.rs#L512) |

### machine.rs / Machine::execute

```rust
fn execute(&mut self, i: &Instruction) -> Result<Option<Event>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under (&i.opcode,&i.operand)=(op,Operand::Variable{flags,hash,offset}) if (0x1b0..=0x265).contains(op)` | `*op>=0x210` | Local state/value operations | Handled | [412](../../crates/crysirror-majiro/src/machine.rs#L412) |
| `condition; under (&i.opcode,&i.operand)=(op,Operand::Variable{flags,hash,offset}) if (0x1b0..=0x265).contains(op)` | `group==0 && ty==1 && rhs.kind()==0` | Value::Float, scalar::number | Handled | [413](../../crates/crysirror-majiro/src/machine.rs#L413) |
| `i.opcode; under (&i.opcode,&i.operand)=(0x190\|0x198\|0x1a0,Operand::None)` | `0x190` | i32::from | Handled | [420](../../crates/crysirror-majiro/src/machine.rs#L420) |
| `i.opcode; under (&i.opcode,&i.operand)=(0x190\|0x198\|0x1a0,Operand::None)` | `0x198` | Local state/value operations | Handled | [420](../../crates/crysirror-majiro/src/machine.rs#L420) |
| `i.opcode; under (&i.opcode,&i.operand)=(0x190\|0x198\|0x1a0,Operand::None)` | `_` | Local state/value operations | Handled | [420](../../crates/crysirror-majiro/src/machine.rs#L420) |
| `condition; under (&i.opcode,&i.operand)=(0x82d\|0x82e,Operand::Jump(target))` | `scalar::branch(&self.tail(1)?[0],i.opcode==0x82d)` | Local state/value operations | Handled | [427](../../crates/crysirror-majiro/src/machine.rs#L427) |
| `(&i.opcode,&i.operand)` | `(op,Operand::Variable{flags,hash,offset}) if (0x270..=0x322).contains(op)` | self.store_element | Handled | [402](../../crates/crysirror-majiro/src/machine.rs#L402) |
| `(&i.opcode,&i.operand)` | `(op,Operand::Variable{flags,hash,offset}) if (0x1b0..=0x265).contains(op)` | self.tail, Value::Float, scalar::number, self.variable, scalar::binary | Handled | [411](../../crates/crysirror-majiro/src/machine.rs#L411) |
| `(&i.opcode,&i.operand)` | `(op,Operand::None) if *op>=0x100 && *op<=0x188` | self.tail, scalar::binary, self.stack.truncate, self.stack.len, self.stack.push | Handled | [417](../../crates/crysirror-majiro/src/machine.rs#L417) |
| `(&i.opcode,&i.operand)` | `(0x82b,Operand::None) if self.frames.last().unwrap().bsel.destructor!=0` | self.frames.last_mut | Handled | [454](../../crates/crysirror-majiro/src/machine.rs#L454) |
| `(&i.opcode,&i.operand)` | `_` | Local state/value operations | Unsupported: unsupported original Majiro instruction {:#x}; execution stopped without advancing | [472](../../crates/crysirror-majiro/src/machine.rs#L472) |

### machine/arrays.rs / Machine::array_variable

```rust
fn array_variable(&mut self, flags: u16, hash: u32, offset: i16) -> Result<ArrayRef>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `scope < 3 && !self.banks[scope as usize].contains_key(&hash)` | Value::Array, ArrayRef::new | Handled | [19](../../crates/crysirror-majiro/src/machine/arrays.rs#L19) |

### machine/arrays.rs / Machine::deliver_input

```rust
fn deliver_input(&mut self, latch: u8, fast: bool) -> Result<Vec<i32>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `self.edit_thread_state(id, "input delivery", \|state, next_frame\| { state.click_input(latch, fast, next_frame) })?` | self.wake_thread | Handled | [231](../../crates/crysirror-majiro/src/machine/arrays.rs#L231) |

### machine/arrays.rs / Machine::store_element

```rust
fn store_element(
        &mut self,
        op: u16,
        flags: u16,
        hash: u32,
        offset: i16,
    ) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `op >= 0x2d0` | Local state/value operations | Handled | [108](../../crates/crysirror-majiro/src/machine/arrays.rs#L108) |

### machine/events.rs / Machine::native_thread_control

```rust
fn native_thread_control(&mut self, event: &Event) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under *hash=0x078a756e \| 0x1295bbda` | `*hash == 0x078a756e` | self.hooks.push | Handled | [72](../../crates/crysirror-majiro/src/machine/events.rs#L72) |
| `condition; under *hash=0x76ee6c90 \| 0x29f1ac40` | `id == -1` | Local state/value operations | Handled | [105](../../crates/crysirror-majiro/src/machine/events.rs#L105) |
| `*hash` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [130](../../crates/crysirror-majiro/src/machine/events.rs#L130) |

### machine/modules.rs / Machine::enter_library

```rust
fn enter_library(&mut self, name: String, program: Arc<Program>) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `*hash == 0x9627ddc3` | Local state/value operations | Handled | [129](../../crates/crysirror-majiro/src/machine/modules.rs#L129) |

### machine/scalar.rs / binary

```rust
fn binary(op: u16, left: &Value, right: &Value) -> Result<Value>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(op,0x13a\|0x142\|0x14a\|0x152\|0x15a\|0x162)` | crate::byte_policy::compare, Value::Integer, i32::from | Handled | [25](../../crates/crysirror-majiro/src/machine/scalar.rs#L25) |
| `condition` | `matches!(op,0x13a\|0x142\|0x14a\|0x152\|0x15a\|0x162)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [25](../../crates/crysirror-majiro/src/machine/scalar.rs#L25) |
| `op` | `0x13a` | Local state/value operations | Handled | [29](../../crates/crysirror-majiro/src/machine/scalar.rs#L29) |
| `op` | `0x142` | Local state/value operations | Handled | [29](../../crates/crysirror-majiro/src/machine/scalar.rs#L29) |
| `op` | `0x14a` | Local state/value operations | Handled | [29](../../crates/crysirror-majiro/src/machine/scalar.rs#L29) |
| `op` | `0x152` | Local state/value operations | Handled | [30](../../crates/crysirror-majiro/src/machine/scalar.rs#L30) |
| `op` | `0x15a` | Local state/value operations | Handled | [30](../../crates/crysirror-majiro/src/machine/scalar.rs#L30) |
| `op` | `_` | Local state/value operations | Handled | [30](../../crates/crysirror-majiro/src/machine/scalar.rs#L30) |
| `condition` | `op & 7 == 0` | super::integer_binary | Handled | [33](../../crates/crysirror-majiro/src/machine/scalar.rs#L33) |
| `condition` | `op == 0x11a` | Vec::with_capacity, Value::String | Handled | [36](../../crates/crysirror-majiro/src/machine/scalar.rs#L36) |
| `op` | `0x101` | Value::Float | Handled | [60](../../crates/crysirror-majiro/src/machine/scalar.rs#L60) |
| `op` | `0x109` | Value::Float | Handled | [63](../../crates/crysirror-majiro/src/machine/scalar.rs#L63) |
| `op` | `0x119` | Value::Float | Handled | [64](../../crates/crysirror-majiro/src/machine/scalar.rs#L64) |
| `op` | `0x121` | Value::Float | Handled | [65](../../crates/crysirror-majiro/src/machine/scalar.rs#L65) |
| `op` | `0x139` | Value::Integer, i32::from | Handled | [66](../../crates/crysirror-majiro/src/machine/scalar.rs#L66) |
| `op` | `0x141` | Value::Integer, i32::from | Handled | [67](../../crates/crysirror-majiro/src/machine/scalar.rs#L67) |
| `op` | `0x149` | Value::Integer, i32::from | Handled | [68](../../crates/crysirror-majiro/src/machine/scalar.rs#L68) |
| `op` | `0x151` | Value::Integer, i32::from | Handled | [69](../../crates/crysirror-majiro/src/machine/scalar.rs#L69) |
| `op` | `0x159` | Value::Integer, i32::from | Handled | [70](../../crates/crysirror-majiro/src/machine/scalar.rs#L70) |
| `op` | `0x161` | Value::Integer, i32::from | Handled | [71](../../crates/crysirror-majiro/src/machine/scalar.rs#L71) |
| `op` | `_` | Local state/value operations | Unsupported: Majiro unknown floating operator {op:#x} | [72](../../crates/crysirror-majiro/src/machine/scalar.rs#L72) |

### machine/scalar.rs / case_branch

```rust
fn case_branch(op: u16, selector: &Value, operand: &Value) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `op; under operand=Value::Integer(b)` | `0x831` | Local state/value operations | Handled | [105](../../crates/crysirror-majiro/src/machine/scalar.rs#L105) |
| `op; under operand=Value::Integer(b)` | `0x832` | Local state/value operations | Handled | [105](../../crates/crysirror-majiro/src/machine/scalar.rs#L105) |
| `op; under operand=Value::Integer(b)` | `0x833` | Local state/value operations | Handled | [105](../../crates/crysirror-majiro/src/machine/scalar.rs#L105) |
| `op; under operand=Value::Integer(b)` | `0x838` | Local state/value operations | Handled | [106](../../crates/crysirror-majiro/src/machine/scalar.rs#L106) |
| `op; under operand=Value::Integer(b)` | `0x839` | Local state/value operations | Handled | [106](../../crates/crysirror-majiro/src/machine/scalar.rs#L106) |
| `op; under operand=Value::Integer(b)` | `_` | Local state/value operations | Unsupported: Majiro invalid native case opcode {op:#x} | [107](../../crates/crysirror-majiro/src/machine/scalar.rs#L107) |
| `op; under operand=Value::Float(b)` | `0x831` | Local state/value operations | Handled | [115](../../crates/crysirror-majiro/src/machine/scalar.rs#L115) |
| `op; under operand=Value::Float(b)` | `0x832` | Local state/value operations | Handled | [115](../../crates/crysirror-majiro/src/machine/scalar.rs#L115) |
| `op; under operand=Value::Float(b)` | `0x833` | Local state/value operations | Handled | [116](../../crates/crysirror-majiro/src/machine/scalar.rs#L116) |
| `op; under operand=Value::Float(b)` | `0x838` | Local state/value operations | Handled | [116](../../crates/crysirror-majiro/src/machine/scalar.rs#L116) |
| `op; under operand=Value::Float(b)` | `0x839` | Local state/value operations | Handled | [117](../../crates/crysirror-majiro/src/machine/scalar.rs#L117) |
| `op; under operand=Value::Float(b)` | `_` | Local state/value operations | Unsupported: Majiro invalid native case opcode {op:#x} | [118](../../crates/crysirror-majiro/src/machine/scalar.rs#L118) |

### machine/threads.rs / Machine::retire_current

```rust
fn retire_current(&mut self) -> bool
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `id == 0` | self.scheduler.parked.clear, self.scheduler.order.clear, self.scheduler.ready.clear, self.scheduler.blocked.clear, self.scheduler.owner_deferred.clear | Handled | [419](../../crates/crysirror-majiro/src/machine/threads.rs#L419) |

### machine/threads.rs / Machine::select_ready

```rust
fn select_ready(&mut self, now: u64) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `self.thread_alive(id) && !self.scheduler.blocked.get(&id).is_some_and(\|deadline\| { if *deadline == u64::MAX { return true; } let (now, deadline) = (now as u32, *deadline as u32); now < deadline && (now.wrapping_sub(deadline) as i32).wrapping_abs() <= 43_200_000 })` | self.switch_thread | Handled | [161](../../crates/crysirror-majiro/src/machine/threads.rs#L161) |

### machine/threads.rs / Machine::spawn_thread

```rust
fn spawn_thread(
        &mut self,
        target: (usize, u32),
        arguments: Vec<Value>,
    ) -> Result<i32>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `id >= 0xffff_fffd` | Local state/value operations | Handled | [345](../../crates/crysirror-majiro/src/machine/threads.rs#L345) |
| `condition` | `id == 0xffff_fffd` | Local state/value operations | Handled | [379](../../crates/crysirror-majiro/src/machine/threads.rs#L379) |

### native_abi.rs / branch_call_compare

```rust
fn branch_call_compare(code: &[u8],base:u64,hash:u32,prefix:&[u8]) -> Option<u64>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under op=0x70..=0x7f \| 0x0f` | `op == 0x0f` | i32::from_le_bytes | Handled | [378](../../crates/crysirror-majiro/src/native_abi.rs#L378) |
| `condition; under op=0x70..=0x7f \| 0x0f` | `!(0x80..=0x8f).contains(&sub)` | Local state/value operations | Handled | [380](../../crates/crysirror-majiro/src/native_abi.rs#L380) |
| `op` | `0xe8` | i32::from_le_bytes | Handled | [356](../../crates/crysirror-majiro/src/native_abi.rs#L356) |
| `op` | `0x50..=0x57 \| 0x90` | Local state/value operations | Handled | [364](../../crates/crysirror-majiro/src/native_abi.rs#L364) |
| `op` | `0xe9` | i32::from_le_bytes, usize::try_from | Handled | [365](../../crates/crysirror-majiro/src/native_abi.rs#L365) |
| `op` | `0xeb` | i32::from, usize::try_from | Handled | [373](../../crates/crysirror-majiro/src/native_abi.rs#L373) |
| `op` | `0x70..=0x7f \| 0x0f` | i32::from_le_bytes, i32::from, usize::try_from | Handled | [377](../../crates/crysirror-majiro/src/native_abi.rs#L377) |
| `op` | `_` | Local state/value operations | Handled | [404](../../crates/crysirror-majiro/src/native_abi.rs#L404) |

### native_abi.rs / NativeAbi::inspect

```rust
fn inspect(pe: &PeData<'_>) -> Result<Self>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `[ (0x7a7b6ed4, key_address), (0x83d81f59, page_address), (0x6e6c641a, test_address), (0x266a3c79, upper_address), ] .into_iter() .all(\|(hash, target)\| branch_call(dispatch, base, hash) == Some(target))` | Local state/value operations | Handled | [169](../../crates/crysirror-majiro/src/native_abi.rs#L169) |

### native_abi/legacy.rs / inspect

```rust
fn inspect(pe: &PeData<'_>) -> Result<Option<NativeAbi>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `[(0x83d81f59, page), (0x266a3c79, upper), (0x6e6c641a, test)] .into_iter() .all(\|(hash, target)\| { branch_call_compare(code, address, hash, &[0x81, 0xfe]) == Some(target) })` | Local state/value operations | Handled | [96](../../crates/crysirror-majiro/src/native_abi/legacy.rs#L96) |

### program.rs / encoding

```rust
fn encoding(op: u16) -> Option<&'static str>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under op=_` | `(0x1b0..=0x205).contains(&op)` | Local state/value operations | Handled | [108](../../crates/crysirror-majiro/src/program.rs#L108) |
| `condition; under op=_` | `(0x210..=0x265).contains(&op)` | Local state/value operations | Handled | [110](../../crates/crysirror-majiro/src/program.rs#L110) |
| `condition; under op=_` | `(0x270..=0x2c2).contains(&op)` | Local state/value operations | Handled | [112](../../crates/crysirror-majiro/src/program.rs#L112) |
| `condition; under op=_` | `(0x2d0..=0x322).contains(&op)` | Local state/value operations | Handled | [114](../../crates/crysirror-majiro/src/program.rs#L114) |
| `op` | `0x800` | Local state/value operations | Handled | [83](../../crates/crysirror-majiro/src/program.rs#L83) |
| `op` | `0x801 \| 0x840 \| 0x842` | Local state/value operations | Handled | [84](../../crates/crysirror-majiro/src/program.rs#L84) |
| `op` | `0x802 \| 0x837` | Local state/value operations | Handled | [85](../../crates/crysirror-majiro/src/program.rs#L85) |
| `op` | `0x803` | Local state/value operations | Handled | [86](../../crates/crysirror-majiro/src/program.rs#L86) |
| `op` | `0x80f \| 0x810` | Local state/value operations | Handled | [87](../../crates/crysirror-majiro/src/program.rs#L87) |
| `op` | `0x834 \| 0x835` | Local state/value operations | Handled | [88](../../crates/crysirror-majiro/src/program.rs#L88) |
| `op` | `0x829 \| 0x836` | Local state/value operations | Handled | [89](../../crates/crysirror-majiro/src/program.rs#L89) |
| `op` | `0x83a` | Local state/value operations | Handled | [90](../../crates/crysirror-majiro/src/program.rs#L90) |
| `op` | `0x850` | Local state/value operations | Handled | [91](../../crates/crysirror-majiro/src/program.rs#L91) |
| `op` | `0x82c..=0x82e \| 0x830..=0x833 \| 0x838..=0x83d \| 0x843 \| 0x845 \| 0x847` | Local state/value operations | Handled | [92](../../crates/crysirror-majiro/src/program.rs#L92) |
| `op` | `0x82b \| 0x82f \| 0x83e \| 0x83f \| 0x841 \| 0x844 \| 0x846 \| 0x190 \| 0x191 \| 0x198 \| 0x1a0 \| 0x1a1 \| 0x1a8 \| 0x1a9` | Local state/value operations | Handled | [93](../../crates/crysirror-majiro/src/program.rs#L93) |
| `op` | `_` | Local state/value operations | Handled | [95](../../crates/crysirror-majiro/src/program.rs#L95) |
