# CMVS developer reference

[All engines](README.md) · [CMVS native ABI](../../crates/crysirror-cmvs/NATIVE_ABI.md)

The reviewed profile is `Pc20160201X64`. The main dispatcher has 18 explicit
non-system instruction IDs. The system dispatcher has 976 case slots from
`0x2000` to `0x23cf`, including 255 entries that return `0x4000` without
operands or effects. The case slots share entry points in several places.
Selectors beyond that table reach a non-advancing fallback in the native
dispatcher. CrySirror reports those unrecognized selectors with their PC.
On 2026-11-18, a direct read at executable RVA `0x84cf8` matched all 976
recorded case RVAs, including the 255 slots pointing to `0x823e7`.
[Recorded x64 case table](../../tools/native-api/cmvs-20160201.json)

CrySirror's structural scan decoded all 2,957,772 instructions in the 87
archived scripts and the loose startup script. The scan found 14 distinct
non-system instruction IDs and 431 system selectors. Counts describe
the installed corpus, including repeated common libraries and unexecuted
branches. They supply a reproducible order for implementation review.
See [the sample validation record](../validation/cmvs-amatsutsumi.md).

Each connected entry still requires review of its operands, native flags,
failure paths, timed continuations and host services. The ABI describes
the examined contracts for implemented families. Missing routes are
listed against their recorded x64 entry point.
Track unresolved work in
[SATD-cmvs-x64-dispatch](../../crates/crysirror-cmvs/SATD.md#satd-cmvs-x64-dispatch).

<!-- BEGIN GENERATED DISPATCH TABLES -->

## Opcode and API implementation tables

`Handled` means an executable source branch exists. It is not a claim that every argument/subselector or a complete game has been audited. Explicit gaps are shown per row.

| Status | Entries |
| --- | ---: |
| Handled | 579 |
| Open | 62 |
| Original no-op | 484 |
| Partial | 3 |
| Unreviewed | 410 |

These tables cover the checked-in catalogues and explicit dispatch branches. An engine-wide upstream denominator is not inferred from these rows.

Open reports an unresolved native syscall contract and is excluded from the implemented entries.

System selectors in the supplied corpus, by CrySirror route status.

| Status | Selectors |
| --- | ---: |
| Handled | 363 |
| Open | 33 |
| Unreviewed | 35 |

## CVNS 20040515 x86 syscalls

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x2000` | Advance and yield | Handled | Guard: x86 case RVA `0x32563` | [machine.rs:758](../../crates/crysirror-cmvs/src/vm/machine.rs#L758) |
| `0x2001` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2002` | Set the native frame period in milliseconds | Handled | Guard: x86 case RVA `0x3256a` | [display.rs:135](../../crates/crysirror-cmvs/src/adapter/display.rs#L135) |
| `0x2003` | Read the native frame period | Handled | Guard: x86 case RVA `0x32572` | [adapter.rs:1626](../../crates/crysirror-cmvs/src/adapter.rs#L1626) |
| `0x2004` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x3257a` | [cvns-20040515.json:6](../../tools/native-api/cvns-20040515.json#L6) |
| `0x2005` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32582` | [cvns-20040515.json:6](../../tools/native-api/cvns-20040515.json#L6) |
| `0x2006` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x3258d` | [cvns-20040515.json:6](../../tools/native-api/cvns-20040515.json#L6) |
| `0x2007` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32595` | [cvns-20040515.json:6](../../tools/native-api/cvns-20040515.json#L6) |
| `0x2008` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x325a5` | [cvns-20040515.json:6](../../tools/native-api/cvns-20040515.json#L6) |
| `0x2009` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x325ad` | [cvns-20040515.json:6](../../tools/native-api/cvns-20040515.json#L6) |
| `0x200A` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x325b5` | [cvns-20040515.json:6](../../tools/native-api/cvns-20040515.json#L6) |
| `0x200B` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x325bd` | [cvns-20040515.json:6](../../tools/native-api/cvns-20040515.json#L6) |
| `0x200C` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x325c5` | [cvns-20040515.json:6](../../tools/native-api/cvns-20040515.json#L6) |
| `0x200D` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x325cd` | [cvns-20040515.json:6](../../tools/native-api/cvns-20040515.json#L6) |
| `0x200E` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x200F` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2010` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x325df` | [cvns-20040515.json:7](../../tools/native-api/cvns-20040515.json#L7) |
| `0x2011` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2012` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x325e7` | [cvns-20040515.json:7](../../tools/native-api/cvns-20040515.json#L7) |
| `0x2013` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x325ef` | [cvns-20040515.json:7](../../tools/native-api/cvns-20040515.json#L7) |
| `0x2014` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x325f7` | [cvns-20040515.json:7](../../tools/native-api/cvns-20040515.json#L7) |
| `0x2015` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2016` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2017` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2018` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x325ff` | [cvns-20040515.json:7](../../tools/native-api/cvns-20040515.json#L7) |
| `0x2019` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32607` | [cvns-20040515.json:7](../../tools/native-api/cvns-20040515.json#L7) |
| `0x201A` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x3260f` | [cvns-20040515.json:7](../../tools/native-api/cvns-20040515.json#L7) |
| `0x201B` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32617` | [cvns-20040515.json:7](../../tools/native-api/cvns-20040515.json#L7) |
| `0x201C` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x3261f` | [cvns-20040515.json:7](../../tools/native-api/cvns-20040515.json#L7) |
| `0x201D` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x201E` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x201F` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32627` | [cvns-20040515.json:7](../../tools/native-api/cvns-20040515.json#L7) |
| `0x2020` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x3262f` | [cvns-20040515.json:8](../../tools/native-api/cvns-20040515.json#L8) |
| `0x2021` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32637` | [cvns-20040515.json:8](../../tools/native-api/cvns-20040515.json#L8) |
| `0x2022` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x3263f` | [cvns-20040515.json:8](../../tools/native-api/cvns-20040515.json#L8) |
| `0x2023` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32647` | [cvns-20040515.json:8](../../tools/native-api/cvns-20040515.json#L8) |
| `0x2024` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x3264f` | [cvns-20040515.json:8](../../tools/native-api/cvns-20040515.json#L8) |
| `0x2025` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32657` | [cvns-20040515.json:8](../../tools/native-api/cvns-20040515.json#L8) |
| `0x2026` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x3265f` | [cvns-20040515.json:8](../../tools/native-api/cvns-20040515.json#L8) |
| `0x2027` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2028` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32667` | [cvns-20040515.json:8](../../tools/native-api/cvns-20040515.json#L8) |
| `0x2029` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x3266f` | [cvns-20040515.json:8](../../tools/native-api/cvns-20040515.json#L8) |
| `0x202A` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32677` | [cvns-20040515.json:8](../../tools/native-api/cvns-20040515.json#L8) |
| `0x202B` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x3267f` | [cvns-20040515.json:8](../../tools/native-api/cvns-20040515.json#L8) |
| `0x202C` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32687` | [cvns-20040515.json:8](../../tools/native-api/cvns-20040515.json#L8) |
| `0x202D` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x202E` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x202F` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2030` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x3268f` | [cvns-20040515.json:9](../../tools/native-api/cvns-20040515.json#L9) |
| `0x2031` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32697` | [cvns-20040515.json:9](../../tools/native-api/cvns-20040515.json#L9) |
| `0x2032` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x3269f` | [cvns-20040515.json:9](../../tools/native-api/cvns-20040515.json#L9) |
| `0x2033` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x326a7` | [cvns-20040515.json:9](../../tools/native-api/cvns-20040515.json#L9) |
| `0x2034` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x326af` | [cvns-20040515.json:9](../../tools/native-api/cvns-20040515.json#L9) |
| `0x2035` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2036` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2037` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2038` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x326b7` | [cvns-20040515.json:9](../../tools/native-api/cvns-20040515.json#L9) |
| `0x2039` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x326ca` | [cvns-20040515.json:9](../../tools/native-api/cvns-20040515.json#L9) |
| `0x203A` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x326db` | [cvns-20040515.json:9](../../tools/native-api/cvns-20040515.json#L9) |
| `0x203B` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x326ee` | [cvns-20040515.json:9](../../tools/native-api/cvns-20040515.json#L9) |
| `0x203C` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x326ff` | [cvns-20040515.json:9](../../tools/native-api/cvns-20040515.json#L9) |
| `0x203D` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32710` | [cvns-20040515.json:9](../../tools/native-api/cvns-20040515.json#L9) |
| `0x203E` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x203F` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2040` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x3272a` | [cvns-20040515.json:10](../../tools/native-api/cvns-20040515.json#L10) |
| `0x2041` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32732` | [cvns-20040515.json:10](../../tools/native-api/cvns-20040515.json#L10) |
| `0x2042` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x3273a` | [cvns-20040515.json:10](../../tools/native-api/cvns-20040515.json#L10) |
| `0x2043` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2044` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2045` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2046` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2047` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32742` | [cvns-20040515.json:10](../../tools/native-api/cvns-20040515.json#L10) |
| `0x2048` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x3274a` | [cvns-20040515.json:10](../../tools/native-api/cvns-20040515.json#L10) |
| `0x2049` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32752` | [cvns-20040515.json:10](../../tools/native-api/cvns-20040515.json#L10) |
| `0x204A` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x3275a` | [cvns-20040515.json:10](../../tools/native-api/cvns-20040515.json#L10) |
| `0x204B` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32762` | [cvns-20040515.json:10](../../tools/native-api/cvns-20040515.json#L10) |
| `0x204C` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x3276a` | [cvns-20040515.json:10](../../tools/native-api/cvns-20040515.json#L10) |
| `0x204D` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x204E` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x204F` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2050` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32772` | [cvns-20040515.json:11](../../tools/native-api/cvns-20040515.json#L11) |
| `0x2051` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x3277a` | [cvns-20040515.json:11](../../tools/native-api/cvns-20040515.json#L11) |
| `0x2052` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32782` | [cvns-20040515.json:11](../../tools/native-api/cvns-20040515.json#L11) |
| `0x2053` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x3278a` | [cvns-20040515.json:11](../../tools/native-api/cvns-20040515.json#L11) |
| `0x2054` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2055` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32792` | [cvns-20040515.json:11](../../tools/native-api/cvns-20040515.json#L11) |
| `0x2056` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x3279a` | [cvns-20040515.json:11](../../tools/native-api/cvns-20040515.json#L11) |
| `0x2057` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x327a2` | [cvns-20040515.json:11](../../tools/native-api/cvns-20040515.json#L11) |
| `0x2058` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x327aa` | [cvns-20040515.json:11](../../tools/native-api/cvns-20040515.json#L11) |
| `0x2059` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x327b2` | [cvns-20040515.json:11](../../tools/native-api/cvns-20040515.json#L11) |
| `0x205A` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x327ba` | [cvns-20040515.json:11](../../tools/native-api/cvns-20040515.json#L11) |
| `0x205B` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x327c2` | [cvns-20040515.json:11](../../tools/native-api/cvns-20040515.json#L11) |
| `0x205C` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x327ca` | [cvns-20040515.json:11](../../tools/native-api/cvns-20040515.json#L11) |
| `0x205D` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x327d2` | [cvns-20040515.json:11](../../tools/native-api/cvns-20040515.json#L11) |
| `0x205E` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x327da` | [cvns-20040515.json:11](../../tools/native-api/cvns-20040515.json#L11) |
| `0x205F` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x327e2` | [cvns-20040515.json:11](../../tools/native-api/cvns-20040515.json#L11) |
| `0x2060` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2061` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2062` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x327ea` | [cvns-20040515.json:12](../../tools/native-api/cvns-20040515.json#L12) |
| `0x2063` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x327f2` | [cvns-20040515.json:12](../../tools/native-api/cvns-20040515.json#L12) |
| `0x2064` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2065` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2066` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x327fa` | [cvns-20040515.json:12](../../tools/native-api/cvns-20040515.json#L12) |
| `0x2067` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32802` | [cvns-20040515.json:12](../../tools/native-api/cvns-20040515.json#L12) |
| `0x2068` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x3280a` | [cvns-20040515.json:12](../../tools/native-api/cvns-20040515.json#L12) |
| `0x2069` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32812` | [cvns-20040515.json:12](../../tools/native-api/cvns-20040515.json#L12) |
| `0x206A` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x206B` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32825` | [cvns-20040515.json:12](../../tools/native-api/cvns-20040515.json#L12) |
| `0x206C` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32836` | [cvns-20040515.json:12](../../tools/native-api/cvns-20040515.json#L12) |
| `0x206D` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32847` | [cvns-20040515.json:12](../../tools/native-api/cvns-20040515.json#L12) |
| `0x206E` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x206F` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2070` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32858` | [cvns-20040515.json:13](../../tools/native-api/cvns-20040515.json#L13) |
| `0x2071` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32860` | [cvns-20040515.json:13](../../tools/native-api/cvns-20040515.json#L13) |
| `0x2072` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32868` | [cvns-20040515.json:13](../../tools/native-api/cvns-20040515.json#L13) |
| `0x2073` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2074` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32870` | [cvns-20040515.json:13](../../tools/native-api/cvns-20040515.json#L13) |
| `0x2075` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32878` | [cvns-20040515.json:13](../../tools/native-api/cvns-20040515.json#L13) |
| `0x2076` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32880` | [cvns-20040515.json:13](../../tools/native-api/cvns-20040515.json#L13) |
| `0x2077` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32888` | [cvns-20040515.json:13](../../tools/native-api/cvns-20040515.json#L13) |
| `0x2078` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32890` | [cvns-20040515.json:13](../../tools/native-api/cvns-20040515.json#L13) |
| `0x2079` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32898` | [cvns-20040515.json:13](../../tools/native-api/cvns-20040515.json#L13) |
| `0x207A` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x328a0` | [cvns-20040515.json:13](../../tools/native-api/cvns-20040515.json#L13) |
| `0x207B` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x328a8` | [cvns-20040515.json:13](../../tools/native-api/cvns-20040515.json#L13) |
| `0x207C` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x328b0` | [cvns-20040515.json:13](../../tools/native-api/cvns-20040515.json#L13) |
| `0x207D` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x328b8` | [cvns-20040515.json:13](../../tools/native-api/cvns-20040515.json#L13) |
| `0x207E` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x328c0` | [cvns-20040515.json:13](../../tools/native-api/cvns-20040515.json#L13) |
| `0x207F` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x328c8` | [cvns-20040515.json:13](../../tools/native-api/cvns-20040515.json#L13) |
| `0x2080` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x328d0` | [cvns-20040515.json:14](../../tools/native-api/cvns-20040515.json#L14) |
| `0x2081` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x328d8` | [cvns-20040515.json:14](../../tools/native-api/cvns-20040515.json#L14) |
| `0x2082` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x328e0` | [cvns-20040515.json:14](../../tools/native-api/cvns-20040515.json#L14) |
| `0x2083` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x328e8` | [cvns-20040515.json:14](../../tools/native-api/cvns-20040515.json#L14) |
| `0x2084` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x328f0` | [cvns-20040515.json:14](../../tools/native-api/cvns-20040515.json#L14) |
| `0x2085` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x328f8` | [cvns-20040515.json:14](../../tools/native-api/cvns-20040515.json#L14) |
| `0x2086` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32900` | [cvns-20040515.json:14](../../tools/native-api/cvns-20040515.json#L14) |
| `0x2087` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2088` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32908` | [cvns-20040515.json:14](../../tools/native-api/cvns-20040515.json#L14) |
| `0x2089` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x208A` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x208B` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x208C` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x208D` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x208E` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x208F` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2090` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32910` | [cvns-20040515.json:15](../../tools/native-api/cvns-20040515.json#L15) |
| `0x2091` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32918` | [cvns-20040515.json:15](../../tools/native-api/cvns-20040515.json#L15) |
| `0x2092` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32920` | [cvns-20040515.json:15](../../tools/native-api/cvns-20040515.json#L15) |
| `0x2093` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32928` | [cvns-20040515.json:15](../../tools/native-api/cvns-20040515.json#L15) |
| `0x2094` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32930` | [cvns-20040515.json:15](../../tools/native-api/cvns-20040515.json#L15) |
| `0x2095` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2096` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2097` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2098` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2099` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x209A` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x209B` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x209C` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x209D` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x209E` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x209F` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x20A0` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32938` | [cvns-20040515.json:16](../../tools/native-api/cvns-20040515.json#L16) |
| `0x20A1` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32940` | [cvns-20040515.json:16](../../tools/native-api/cvns-20040515.json#L16) |
| `0x20A2` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32952` | [cvns-20040515.json:16](../../tools/native-api/cvns-20040515.json#L16) |
| `0x20A3` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x20A4` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x3295a` | [cvns-20040515.json:16](../../tools/native-api/cvns-20040515.json#L16) |
| `0x20A5` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32962` | [cvns-20040515.json:16](../../tools/native-api/cvns-20040515.json#L16) |
| `0x20A6` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x3296a` | [cvns-20040515.json:16](../../tools/native-api/cvns-20040515.json#L16) |
| `0x20A7` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x20A8` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x20A9` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x20AA` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x20AB` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x20AC` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x20AD` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32972` | [cvns-20040515.json:16](../../tools/native-api/cvns-20040515.json#L16) |
| `0x20AE` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x3297a` | [cvns-20040515.json:16](../../tools/native-api/cvns-20040515.json#L16) |
| `0x20AF` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32982` | [cvns-20040515.json:16](../../tools/native-api/cvns-20040515.json#L16) |
| `0x20B0` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x3298a` | [cvns-20040515.json:17](../../tools/native-api/cvns-20040515.json#L17) |
| `0x20B1` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32992` | [cvns-20040515.json:17](../../tools/native-api/cvns-20040515.json#L17) |
| `0x20B2` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x3299a` | [cvns-20040515.json:17](../../tools/native-api/cvns-20040515.json#L17) |
| `0x20B3` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x329a2` | [cvns-20040515.json:17](../../tools/native-api/cvns-20040515.json#L17) |
| `0x20B4` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x329aa` | [cvns-20040515.json:17](../../tools/native-api/cvns-20040515.json#L17) |
| `0x20B5` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x329b2` | [cvns-20040515.json:17](../../tools/native-api/cvns-20040515.json#L17) |
| `0x20B6` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x329ba` | [cvns-20040515.json:17](../../tools/native-api/cvns-20040515.json#L17) |
| `0x20B7` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x329c2` | [cvns-20040515.json:17](../../tools/native-api/cvns-20040515.json#L17) |
| `0x20B8` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x329ca` | [cvns-20040515.json:17](../../tools/native-api/cvns-20040515.json#L17) |
| `0x20B9` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x329d2` | [cvns-20040515.json:17](../../tools/native-api/cvns-20040515.json#L17) |
| `0x20BA` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x329da` | [cvns-20040515.json:17](../../tools/native-api/cvns-20040515.json#L17) |
| `0x20BB` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x329e2` | [cvns-20040515.json:17](../../tools/native-api/cvns-20040515.json#L17) |
| `0x20BC` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x329ea` | [cvns-20040515.json:17](../../tools/native-api/cvns-20040515.json#L17) |
| `0x20BD` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x329f2` | [cvns-20040515.json:17](../../tools/native-api/cvns-20040515.json#L17) |
| `0x20BE` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x329fa` | [cvns-20040515.json:17](../../tools/native-api/cvns-20040515.json#L17) |
| `0x20BF` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x20C0` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32a02` | [cvns-20040515.json:18](../../tools/native-api/cvns-20040515.json#L18) |
| `0x20C1` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x20C2` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x20C3` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x20C4` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x20C5` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32a0a` | [cvns-20040515.json:18](../../tools/native-api/cvns-20040515.json#L18) |
| `0x20C6` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32a12` | [cvns-20040515.json:18](../../tools/native-api/cvns-20040515.json#L18) |
| `0x20C7` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32a1a` | [cvns-20040515.json:18](../../tools/native-api/cvns-20040515.json#L18) |
| `0x20C8` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32a22` | [cvns-20040515.json:18](../../tools/native-api/cvns-20040515.json#L18) |
| `0x20C9` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32a2a` | [cvns-20040515.json:18](../../tools/native-api/cvns-20040515.json#L18) |
| `0x20CA` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32a32` | [cvns-20040515.json:18](../../tools/native-api/cvns-20040515.json#L18) |
| `0x20CB` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32a3a` | [cvns-20040515.json:18](../../tools/native-api/cvns-20040515.json#L18) |
| `0x20CC` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32a42` | [cvns-20040515.json:18](../../tools/native-api/cvns-20040515.json#L18) |
| `0x20CD` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x20CE` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x20CF` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x20D0` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32a4a` | [cvns-20040515.json:19](../../tools/native-api/cvns-20040515.json#L19) |
| `0x20D1` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32a52` | [cvns-20040515.json:19](../../tools/native-api/cvns-20040515.json#L19) |
| `0x20D2` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32a5a` | [cvns-20040515.json:19](../../tools/native-api/cvns-20040515.json#L19) |
| `0x20D3` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32a62` | [cvns-20040515.json:19](../../tools/native-api/cvns-20040515.json#L19) |
| `0x20D4` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32a6a` | [cvns-20040515.json:19](../../tools/native-api/cvns-20040515.json#L19) |
| `0x20D5` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32a72` | [cvns-20040515.json:19](../../tools/native-api/cvns-20040515.json#L19) |
| `0x20D6` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32a7a` | [cvns-20040515.json:19](../../tools/native-api/cvns-20040515.json#L19) |
| `0x20D7` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32a82` | [cvns-20040515.json:19](../../tools/native-api/cvns-20040515.json#L19) |
| `0x20D8` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32a8a` | [cvns-20040515.json:19](../../tools/native-api/cvns-20040515.json#L19) |
| `0x20D9` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32a92` | [cvns-20040515.json:19](../../tools/native-api/cvns-20040515.json#L19) |
| `0x20DA` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32a9a` | [cvns-20040515.json:19](../../tools/native-api/cvns-20040515.json#L19) |
| `0x20DB` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32aa2` | [cvns-20040515.json:19](../../tools/native-api/cvns-20040515.json#L19) |
| `0x20DC` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32aaa` | [cvns-20040515.json:19](../../tools/native-api/cvns-20040515.json#L19) |
| `0x20DD` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32ab2` | [cvns-20040515.json:19](../../tools/native-api/cvns-20040515.json#L19) |
| `0x20DE` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32aba` | [cvns-20040515.json:19](../../tools/native-api/cvns-20040515.json#L19) |
| `0x20DF` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32ac2` | [cvns-20040515.json:19](../../tools/native-api/cvns-20040515.json#L19) |
| `0x20E0` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32aca` | [cvns-20040515.json:20](../../tools/native-api/cvns-20040515.json#L20) |
| `0x20E1` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32ad2` | [cvns-20040515.json:20](../../tools/native-api/cvns-20040515.json#L20) |
| `0x20E2` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32ada` | [cvns-20040515.json:20](../../tools/native-api/cvns-20040515.json#L20) |
| `0x20E3` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32ae2` | [cvns-20040515.json:20](../../tools/native-api/cvns-20040515.json#L20) |
| `0x20E4` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32aea` | [cvns-20040515.json:20](../../tools/native-api/cvns-20040515.json#L20) |
| `0x20E5` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32af2` | [cvns-20040515.json:20](../../tools/native-api/cvns-20040515.json#L20) |
| `0x20E6` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32afa` | [cvns-20040515.json:20](../../tools/native-api/cvns-20040515.json#L20) |
| `0x20E7` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32b02` | [cvns-20040515.json:20](../../tools/native-api/cvns-20040515.json#L20) |
| `0x20E8` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32b0a` | [cvns-20040515.json:20](../../tools/native-api/cvns-20040515.json#L20) |
| `0x20E9` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32b12` | [cvns-20040515.json:20](../../tools/native-api/cvns-20040515.json#L20) |
| `0x20EA` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32b1a` | [cvns-20040515.json:20](../../tools/native-api/cvns-20040515.json#L20) |
| `0x20EB` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32b22` | [cvns-20040515.json:20](../../tools/native-api/cvns-20040515.json#L20) |
| `0x20EC` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x20ED` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x20EE` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x20EF` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x20F0` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32b2a` | [cvns-20040515.json:21](../../tools/native-api/cvns-20040515.json#L21) |
| `0x20F1` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32b32` | [cvns-20040515.json:21](../../tools/native-api/cvns-20040515.json#L21) |
| `0x20F2` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32b3a` | [cvns-20040515.json:21](../../tools/native-api/cvns-20040515.json#L21) |
| `0x20F3` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32b42` | [cvns-20040515.json:21](../../tools/native-api/cvns-20040515.json#L21) |
| `0x20F4` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32b4a` | [cvns-20040515.json:21](../../tools/native-api/cvns-20040515.json#L21) |
| `0x20F5` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32b52` | [cvns-20040515.json:21](../../tools/native-api/cvns-20040515.json#L21) |
| `0x20F6` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32b5a` | [cvns-20040515.json:21](../../tools/native-api/cvns-20040515.json#L21) |
| `0x20F7` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32b62` | [cvns-20040515.json:21](../../tools/native-api/cvns-20040515.json#L21) |
| `0x20F8` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32b6a` | [cvns-20040515.json:21](../../tools/native-api/cvns-20040515.json#L21) |
| `0x20F9` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32b72` | [cvns-20040515.json:21](../../tools/native-api/cvns-20040515.json#L21) |
| `0x20FA` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32b7a` | [cvns-20040515.json:21](../../tools/native-api/cvns-20040515.json#L21) |
| `0x20FB` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32b82` | [cvns-20040515.json:21](../../tools/native-api/cvns-20040515.json#L21) |
| `0x20FC` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32b8a` | [cvns-20040515.json:21](../../tools/native-api/cvns-20040515.json#L21) |
| `0x20FD` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32b92` | [cvns-20040515.json:21](../../tools/native-api/cvns-20040515.json#L21) |
| `0x20FE` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x20FF` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2100` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32b9a` | [cvns-20040515.json:22](../../tools/native-api/cvns-20040515.json#L22) |
| `0x2101` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32ba2` | [cvns-20040515.json:22](../../tools/native-api/cvns-20040515.json#L22) |
| `0x2102` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32baa` | [cvns-20040515.json:22](../../tools/native-api/cvns-20040515.json#L22) |
| `0x2103` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2104` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2105` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2106` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2107` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2108` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32bb2` | [cvns-20040515.json:22](../../tools/native-api/cvns-20040515.json#L22) |
| `0x2109` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32bba` | [cvns-20040515.json:22](../../tools/native-api/cvns-20040515.json#L22) |
| `0x210A` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32bc2` | [cvns-20040515.json:22](../../tools/native-api/cvns-20040515.json#L22) |
| `0x210B` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32bca` | [cvns-20040515.json:22](../../tools/native-api/cvns-20040515.json#L22) |
| `0x210C` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32bd2` | [cvns-20040515.json:22](../../tools/native-api/cvns-20040515.json#L22) |
| `0x210D` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32bda` | [cvns-20040515.json:22](../../tools/native-api/cvns-20040515.json#L22) |
| `0x210E` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32be2` | [cvns-20040515.json:22](../../tools/native-api/cvns-20040515.json#L22) |
| `0x210F` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2110` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32bf3` | [cvns-20040515.json:23](../../tools/native-api/cvns-20040515.json#L23) |
| `0x2111` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32bfb` | [cvns-20040515.json:23](../../tools/native-api/cvns-20040515.json#L23) |
| `0x2112` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32c03` | [cvns-20040515.json:23](../../tools/native-api/cvns-20040515.json#L23) |
| `0x2113` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32c0b` | [cvns-20040515.json:23](../../tools/native-api/cvns-20040515.json#L23) |
| `0x2114` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32c13` | [cvns-20040515.json:23](../../tools/native-api/cvns-20040515.json#L23) |
| `0x2115` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2116` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2117` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32c1b` | [cvns-20040515.json:23](../../tools/native-api/cvns-20040515.json#L23) |
| `0x2118` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32c23` | [cvns-20040515.json:23](../../tools/native-api/cvns-20040515.json#L23) |
| `0x2119` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32c2b` | [cvns-20040515.json:23](../../tools/native-api/cvns-20040515.json#L23) |
| `0x211A` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32c33` | [cvns-20040515.json:23](../../tools/native-api/cvns-20040515.json#L23) |
| `0x211B` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32c3b` | [cvns-20040515.json:23](../../tools/native-api/cvns-20040515.json#L23) |
| `0x211C` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x211D` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32c43` | [cvns-20040515.json:23](../../tools/native-api/cvns-20040515.json#L23) |
| `0x211E` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32c4b` | [cvns-20040515.json:23](../../tools/native-api/cvns-20040515.json#L23) |
| `0x211F` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32c53` | [cvns-20040515.json:23](../../tools/native-api/cvns-20040515.json#L23) |
| `0x2120` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32c5b` | [cvns-20040515.json:24](../../tools/native-api/cvns-20040515.json#L24) |
| `0x2121` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32c63` | [cvns-20040515.json:24](../../tools/native-api/cvns-20040515.json#L24) |
| `0x2122` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32c6b` | [cvns-20040515.json:24](../../tools/native-api/cvns-20040515.json#L24) |
| `0x2123` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32c73` | [cvns-20040515.json:24](../../tools/native-api/cvns-20040515.json#L24) |
| `0x2124` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32c7b` | [cvns-20040515.json:24](../../tools/native-api/cvns-20040515.json#L24) |
| `0x2125` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32c83` | [cvns-20040515.json:24](../../tools/native-api/cvns-20040515.json#L24) |
| `0x2126` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32c8b` | [cvns-20040515.json:24](../../tools/native-api/cvns-20040515.json#L24) |
| `0x2127` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2128` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32c93` | [cvns-20040515.json:24](../../tools/native-api/cvns-20040515.json#L24) |
| `0x2129` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32c9b` | [cvns-20040515.json:24](../../tools/native-api/cvns-20040515.json#L24) |
| `0x212A` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32ca3` | [cvns-20040515.json:24](../../tools/native-api/cvns-20040515.json#L24) |
| `0x212B` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32cab` | [cvns-20040515.json:24](../../tools/native-api/cvns-20040515.json#L24) |
| `0x212C` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32cb3` | [cvns-20040515.json:24](../../tools/native-api/cvns-20040515.json#L24) |
| `0x212D` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32cbb` | [cvns-20040515.json:24](../../tools/native-api/cvns-20040515.json#L24) |
| `0x212E` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32cc3` | [cvns-20040515.json:24](../../tools/native-api/cvns-20040515.json#L24) |
| `0x212F` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32ccb` | [cvns-20040515.json:24](../../tools/native-api/cvns-20040515.json#L24) |
| `0x2130` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32cdd` | [cvns-20040515.json:25](../../tools/native-api/cvns-20040515.json#L25) |
| `0x2131` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32ce5` | [cvns-20040515.json:25](../../tools/native-api/cvns-20040515.json#L25) |
| `0x2132` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2133` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2134` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2135` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2136` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32ced` | [cvns-20040515.json:25](../../tools/native-api/cvns-20040515.json#L25) |
| `0x2137` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2138` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32cf5` | [cvns-20040515.json:25](../../tools/native-api/cvns-20040515.json#L25) |
| `0x2139` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32cfd` | [cvns-20040515.json:25](../../tools/native-api/cvns-20040515.json#L25) |
| `0x213A` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32d05` | [cvns-20040515.json:25](../../tools/native-api/cvns-20040515.json#L25) |
| `0x213B` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32d0d` | [cvns-20040515.json:25](../../tools/native-api/cvns-20040515.json#L25) |
| `0x213C` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32d15` | [cvns-20040515.json:25](../../tools/native-api/cvns-20040515.json#L25) |
| `0x213D` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32d1d` | [cvns-20040515.json:25](../../tools/native-api/cvns-20040515.json#L25) |
| `0x213E` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32d25` | [cvns-20040515.json:25](../../tools/native-api/cvns-20040515.json#L25) |
| `0x213F` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32d2d` | [cvns-20040515.json:25](../../tools/native-api/cvns-20040515.json#L25) |
| `0x2140` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32d35` | [cvns-20040515.json:26](../../tools/native-api/cvns-20040515.json#L26) |
| `0x2141` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32d3d` | [cvns-20040515.json:26](../../tools/native-api/cvns-20040515.json#L26) |
| `0x2142` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32d45` | [cvns-20040515.json:26](../../tools/native-api/cvns-20040515.json#L26) |
| `0x2143` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32d4d` | [cvns-20040515.json:26](../../tools/native-api/cvns-20040515.json#L26) |
| `0x2144` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32d55` | [cvns-20040515.json:26](../../tools/native-api/cvns-20040515.json#L26) |
| `0x2145` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32d5d` | [cvns-20040515.json:26](../../tools/native-api/cvns-20040515.json#L26) |
| `0x2146` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32d65` | [cvns-20040515.json:26](../../tools/native-api/cvns-20040515.json#L26) |
| `0x2147` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32d6d` | [cvns-20040515.json:26](../../tools/native-api/cvns-20040515.json#L26) |
| `0x2148` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32d75` | [cvns-20040515.json:26](../../tools/native-api/cvns-20040515.json#L26) |
| `0x2149` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x214A` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x214B` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x214C` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x214D` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x214E` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x214F` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32d7d` | [cvns-20040515.json:26](../../tools/native-api/cvns-20040515.json#L26) |
| `0x2150` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32d85` | [cvns-20040515.json:27](../../tools/native-api/cvns-20040515.json#L27) |
| `0x2151` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32d8d` | [cvns-20040515.json:27](../../tools/native-api/cvns-20040515.json#L27) |
| `0x2152` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32d95` | [cvns-20040515.json:27](../../tools/native-api/cvns-20040515.json#L27) |
| `0x2153` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32d9d` | [cvns-20040515.json:27](../../tools/native-api/cvns-20040515.json#L27) |
| `0x2154` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32da5` | [cvns-20040515.json:27](../../tools/native-api/cvns-20040515.json#L27) |
| `0x2155` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32dad` | [cvns-20040515.json:27](../../tools/native-api/cvns-20040515.json#L27) |
| `0x2156` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32db5` | [cvns-20040515.json:27](../../tools/native-api/cvns-20040515.json#L27) |
| `0x2157` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2158` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32dbd` | [cvns-20040515.json:27](../../tools/native-api/cvns-20040515.json#L27) |
| `0x2159` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32dc5` | [cvns-20040515.json:27](../../tools/native-api/cvns-20040515.json#L27) |
| `0x215A` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32dcd` | [cvns-20040515.json:27](../../tools/native-api/cvns-20040515.json#L27) |
| `0x215B` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x215C` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x215D` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x215E` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x215F` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2160` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32dd5` | [cvns-20040515.json:28](../../tools/native-api/cvns-20040515.json#L28) |
| `0x2161` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2162` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2163` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32ddd` | [cvns-20040515.json:28](../../tools/native-api/cvns-20040515.json#L28) |
| `0x2164` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2165` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2166` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2167` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2168` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32de5` | [cvns-20040515.json:28](../../tools/native-api/cvns-20040515.json#L28) |
| `0x2169` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x216A` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x216B` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x216C` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x216D` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x216E` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x216F` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2170` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2171` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2172` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2173` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2174` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2175` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2176` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2177` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2178` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32ded` | [cvns-20040515.json:29](../../tools/native-api/cvns-20040515.json#L29) |
| `0x2179` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32df5` | [cvns-20040515.json:29](../../tools/native-api/cvns-20040515.json#L29) |
| `0x217A` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32dfd` | [cvns-20040515.json:29](../../tools/native-api/cvns-20040515.json#L29) |
| `0x217B` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32e05` | [cvns-20040515.json:29](../../tools/native-api/cvns-20040515.json#L29) |
| `0x217C` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32e0d` | [cvns-20040515.json:29](../../tools/native-api/cvns-20040515.json#L29) |
| `0x217D` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32e15` | [cvns-20040515.json:29](../../tools/native-api/cvns-20040515.json#L29) |
| `0x217E` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x217F` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32e1d` | [cvns-20040515.json:29](../../tools/native-api/cvns-20040515.json#L29) |
| `0x2180` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32e25` | [cvns-20040515.json:30](../../tools/native-api/cvns-20040515.json#L30) |
| `0x2181` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32e2d` | [cvns-20040515.json:30](../../tools/native-api/cvns-20040515.json#L30) |
| `0x2182` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32e35` | [cvns-20040515.json:30](../../tools/native-api/cvns-20040515.json#L30) |
| `0x2183` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32e3d` | [cvns-20040515.json:30](../../tools/native-api/cvns-20040515.json#L30) |
| `0x2184` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2185` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2186` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2187` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2188` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32e45` | [cvns-20040515.json:30](../../tools/native-api/cvns-20040515.json#L30) |
| `0x2189` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32e4d` | [cvns-20040515.json:30](../../tools/native-api/cvns-20040515.json#L30) |
| `0x218A` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32e55` | [cvns-20040515.json:30](../../tools/native-api/cvns-20040515.json#L30) |
| `0x218B` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32e5d` | [cvns-20040515.json:30](../../tools/native-api/cvns-20040515.json#L30) |
| `0x218C` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32e65` | [cvns-20040515.json:30](../../tools/native-api/cvns-20040515.json#L30) |
| `0x218D` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32e6d` | [cvns-20040515.json:30](../../tools/native-api/cvns-20040515.json#L30) |
| `0x218E` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32e75` | [cvns-20040515.json:30](../../tools/native-api/cvns-20040515.json#L30) |
| `0x218F` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32e7d` | [cvns-20040515.json:30](../../tools/native-api/cvns-20040515.json#L30) |
| `0x2190` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32e85` | [cvns-20040515.json:31](../../tools/native-api/cvns-20040515.json#L31) |
| `0x2191` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32e8d` | [cvns-20040515.json:31](../../tools/native-api/cvns-20040515.json#L31) |
| `0x2192` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32e95` | [cvns-20040515.json:31](../../tools/native-api/cvns-20040515.json#L31) |
| `0x2193` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32e9d` | [cvns-20040515.json:31](../../tools/native-api/cvns-20040515.json#L31) |
| `0x2194` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32ea5` | [cvns-20040515.json:31](../../tools/native-api/cvns-20040515.json#L31) |
| `0x2195` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32ead` | [cvns-20040515.json:31](../../tools/native-api/cvns-20040515.json#L31) |
| `0x2196` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32eb5` | [cvns-20040515.json:31](../../tools/native-api/cvns-20040515.json#L31) |
| `0x2197` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2198` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2199` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x219A` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x219B` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x219C` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x219D` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x219E` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x219F` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21A0` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32ebd` | [cvns-20040515.json:32](../../tools/native-api/cvns-20040515.json#L32) |
| `0x21A1` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32ec5` | [cvns-20040515.json:32](../../tools/native-api/cvns-20040515.json#L32) |
| `0x21A2` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32ecd` | [cvns-20040515.json:32](../../tools/native-api/cvns-20040515.json#L32) |
| `0x21A3` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32ed5` | [cvns-20040515.json:32](../../tools/native-api/cvns-20040515.json#L32) |
| `0x21A4` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32edd` | [cvns-20040515.json:32](../../tools/native-api/cvns-20040515.json#L32) |
| `0x21A5` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32ee5` | [cvns-20040515.json:32](../../tools/native-api/cvns-20040515.json#L32) |
| `0x21A6` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32eed` | [cvns-20040515.json:32](../../tools/native-api/cvns-20040515.json#L32) |
| `0x21A7` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32ef5` | [cvns-20040515.json:32](../../tools/native-api/cvns-20040515.json#L32) |
| `0x21A8` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21A9` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32efd` | [cvns-20040515.json:32](../../tools/native-api/cvns-20040515.json#L32) |
| `0x21AA` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32f05` | [cvns-20040515.json:32](../../tools/native-api/cvns-20040515.json#L32) |
| `0x21AB` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32f0d` | [cvns-20040515.json:32](../../tools/native-api/cvns-20040515.json#L32) |
| `0x21AC` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32f15` | [cvns-20040515.json:32](../../tools/native-api/cvns-20040515.json#L32) |
| `0x21AD` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21AE` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32f1d` | [cvns-20040515.json:32](../../tools/native-api/cvns-20040515.json#L32) |
| `0x21AF` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32f25` | [cvns-20040515.json:32](../../tools/native-api/cvns-20040515.json#L32) |
| `0x21B0` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32f31` | [cvns-20040515.json:33](../../tools/native-api/cvns-20040515.json#L33) |
| `0x21B1` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32f39` | [cvns-20040515.json:33](../../tools/native-api/cvns-20040515.json#L33) |
| `0x21B2` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32f41` | [cvns-20040515.json:33](../../tools/native-api/cvns-20040515.json#L33) |
| `0x21B3` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21B4` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21B5` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21B6` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32f49` | [cvns-20040515.json:33](../../tools/native-api/cvns-20040515.json#L33) |
| `0x21B7` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32f55` | [cvns-20040515.json:33](../../tools/native-api/cvns-20040515.json#L33) |
| `0x21B8` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21B9` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21BA` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21BB` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21BC` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21BD` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21BE` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21BF` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21C0` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32f5d` | [cvns-20040515.json:34](../../tools/native-api/cvns-20040515.json#L34) |
| `0x21C1` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32f69` | [cvns-20040515.json:34](../../tools/native-api/cvns-20040515.json#L34) |
| `0x21C2` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32f75` | [cvns-20040515.json:34](../../tools/native-api/cvns-20040515.json#L34) |
| `0x21C3` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32f86` | [cvns-20040515.json:34](../../tools/native-api/cvns-20040515.json#L34) |
| `0x21C4` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32f97` | [cvns-20040515.json:34](../../tools/native-api/cvns-20040515.json#L34) |
| `0x21C5` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21C6` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21C7` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21C8` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32fa8` | [cvns-20040515.json:34](../../tools/native-api/cvns-20040515.json#L34) |
| `0x21C9` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32fb0` | [cvns-20040515.json:34](../../tools/native-api/cvns-20040515.json#L34) |
| `0x21CA` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32fb8` | [cvns-20040515.json:34](../../tools/native-api/cvns-20040515.json#L34) |
| `0x21CB` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32fc0` | [cvns-20040515.json:34](../../tools/native-api/cvns-20040515.json#L34) |
| `0x21CC` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32fce` | [cvns-20040515.json:34](../../tools/native-api/cvns-20040515.json#L34) |
| `0x21CD` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21CE` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21CF` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21D0` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21D1` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21D2` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21D3` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21D4` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32fdc` | [cvns-20040515.json:35](../../tools/native-api/cvns-20040515.json#L35) |
| `0x21D5` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32fe4` | [cvns-20040515.json:35](../../tools/native-api/cvns-20040515.json#L35) |
| `0x21D6` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21D7` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21D8` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21D9` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21DA` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21DB` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21DC` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21DD` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21DE` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21DF` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21E0` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21E1` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21E2` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21E3` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21E4` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21E5` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21E6` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21E7` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21E8` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21E9` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21EA` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21EB` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21EC` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32fec` | [cvns-20040515.json:36](../../tools/native-api/cvns-20040515.json#L36) |
| `0x21ED` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32ff4` | [cvns-20040515.json:36](../../tools/native-api/cvns-20040515.json#L36) |
| `0x21EE` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21EF` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21F0` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21F1` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21F2` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21F3` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21F4` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21F5` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21F6` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21F7` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21F8` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21F9` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21FA` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21FB` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21FC` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21FD` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21FE` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x21FF` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2200` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x32ffc` | [cvns-20040515.json:38](../../tools/native-api/cvns-20040515.json#L38) |
| `0x2201` | Original system dispatcher entry | Unreviewed | No CVNS execution route; Guard: x86 case RVA `0x33004` | [cvns-20040515.json:38](../../tools/native-api/cvns-20040515.json#L38) |
| `0x2202` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2203` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2204` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2205` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2206` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2207` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2208` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2209` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x220A` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x220B` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x220C` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x220D` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x220E` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x220F` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2210` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2211` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2212` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2213` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2214` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2215` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2216` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2217` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2218` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x2219` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x221A` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x221B` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x221C` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x221D` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x221E` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |
| `0x221F` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:6](../../crates/crysirror-cmvs/src/vm/cvns.rs#L6) |

## PC 20160201 x64 VM instructions

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x200` | Integer expression | Partial | Expression tag and operator restrictions in the ABI | [machine.rs:655](../../crates/crysirror-cmvs/src/vm/machine.rs#L655) |
| `0x201` | String expression | Partial | Expression tag and operator restrictions in the ABI | [machine.rs:655](../../crates/crysirror-cmvs/src/vm/machine.rs#L655) |
| `0x202` | Float expression | Partial | Expression tag and operator restrictions in the ABI | [machine.rs:655](../../crates/crysirror-cmvs/src/vm/machine.rs#L655) |
| `0x400` | Direct jump | Handled | — | [machine.rs:655](../../crates/crysirror-cmvs/src/vm/machine.rs#L655) |
| `0x401` | Jump on clear condition | Handled | — | [machine.rs:655](../../crates/crysirror-cmvs/src/vm/machine.rs#L655) |
| `0x402` | Jump on set condition | Handled | — | [machine.rs:655](../../crates/crysirror-cmvs/src/vm/machine.rs#L655) |
| `0x403` | Jump on raw result equality | Handled | — | [machine.rs:655](../../crates/crysirror-cmvs/src/vm/machine.rs#L655) |
| `0x405` | Jump to label | Handled | — | [machine.rs:655](../../crates/crysirror-cmvs/src/vm/machine.rs#L655) |
| `0x407` | Long jump | Handled | — | [machine.rs:655](../../crates/crysirror-cmvs/src/vm/machine.rs#L655) |
| `0x410` | Call label | Handled | — | [machine.rs:655](../../crates/crysirror-cmvs/src/vm/machine.rs#L655) |
| `0x411` | Return and discard literal byte count | Handled | — | [machine.rs:655](../../crates/crysirror-cmvs/src/vm/machine.rs#L655) |
| `0x412` | Discard literal byte count | Handled | — | [machine.rs:655](../../crates/crysirror-cmvs/src/vm/machine.rs#L655) |
| `0x413` | Return and discard result-scaled byte count | Handled | — | [machine.rs:655](../../crates/crysirror-cmvs/src/vm/machine.rs#L655) |
| `0x414` | Cross-script return | Handled | — | [machine.rs:655](../../crates/crysirror-cmvs/src/vm/machine.rs#L655) |
| `0x416` | Call result offset in current script | Handled | — | [machine.rs:655](../../crates/crysirror-cmvs/src/vm/machine.rs#L655) |
| `0x430` | Push result | Handled | — | [machine.rs:655](../../crates/crysirror-cmvs/src/vm/machine.rs#L655) |
| `0x440` | Write immediate and advance stack | Handled | — | [machine.rs:655](../../crates/crysirror-cmvs/src/vm/machine.rs#L655) |
| `0x442` | Write immediate and advance stack | Handled | — | [machine.rs:655](../../crates/crysirror-cmvs/src/vm/machine.rs#L655) |

## PC 20160201 x64 syscalls

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x2000` | Advance and yield | Handled | Guard: x64 case RVA `0x823c6`, present in the supplied script corpus | [machine.rs:758](../../crates/crysirror-cmvs/src/vm/machine.rs#L758) |
| `0x2001` | Execute advance branch | Handled | Guard: x64 case RVA `0x823d1`, present in the supplied script corpus | [adapter.rs:1673](../../crates/crysirror-cmvs/src/adapter.rs#L1673) |
| `0x2002` | Execute advance branch | Open | Frame-window counter latch recovered. Its native host update boundaries are pending; Guard: x64 case RVA `0x823de` | [adapter.rs:1625](../../crates/crysirror-cmvs/src/adapter.rs#L1625) |
| `0x2003` | Execute advance branch | Handled | Guard: x64 case RVA `0x823f2`, present in the supplied script corpus | [adapter.rs:1626](../../crates/crysirror-cmvs/src/adapter.rs#L1626) |
| `0x2004` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x82484` | [cmvs-20160201.json:6](../../tools/native-api/cmvs-20160201.json#L6) |
| `0x2005` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x82407` | [cmvs-20160201.json:6](../../tools/native-api/cmvs-20160201.json#L6) |
| `0x2006` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x82414` | [cmvs-20160201.json:6](../../tools/native-api/cmvs-20160201.json#L6) |
| `0x2007` | Execute advance branch | Handled | Guard: x64 case RVA `0x82421` | [adapter.rs:1658](../../crates/crysirror-cmvs/src/adapter.rs#L1658) |
| `0x2008` | Control flow (command) | Handled | Guard: x64 case RVA `0x82437`, present in the supplied script corpus | [control.rs:174](../../crates/crysirror-cmvs/src/vm/control.rs#L174) |
| `0x2009` | Control flow (command) | Handled | Guard: x64 case RVA `0x8244d` | [control.rs:174](../../crates/crysirror-cmvs/src/vm/control.rs#L174) |
| `0x200A` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x82463` | [cmvs-20160201.json:6](../../tools/native-api/cmvs-20160201.json#L6) |
| `0x200B` | Control flow (command) | Handled | Guard: x64 case RVA `0x8248f`, present in the supplied script corpus | [control.rs:186](../../crates/crysirror-cmvs/src/vm/control.rs#L186) |
| `0x200C` | Control flow (command) | Handled | Guard: x64 case RVA `0x824b8`, present in the supplied script corpus | [control.rs:186](../../crates/crysirror-cmvs/src/vm/control.rs#L186) |
| `0x200D` | Execute advance branch | Handled | Guard: x64 case RVA `0x824e3` | [adapter.rs:1664](../../crates/crysirror-cmvs/src/adapter.rs#L1664) |
| `0x200E` | Control flow (command) | Handled | Guard: x64 case RVA `0x824f0`, present in the supplied script corpus | [control.rs:202](../../crates/crysirror-cmvs/src/vm/control.rs#L202) |
| `0x200F` | Control flow (command) | Handled | Guard: x64 case RVA `0x82506`, present in the supplied script corpus | [control.rs:174](../../crates/crysirror-cmvs/src/vm/control.rs#L174) |
| `0x2010` | Required parameter | Handled | Guard: x64 case RVA `0x82522`, present in the supplied script corpus | [startup.rs:1573](../../crates/crysirror-cmvs/src/vm/startup.rs#L1573) |
| `0x2011` | Required parameter | Handled | Guard: x64 case RVA `0x8252f`, present in the supplied script corpus | [startup.rs:1573](../../crates/crysirror-cmvs/src/vm/startup.rs#L1573) |
| `0x2012` | Required parameter | Handled | Guard: x64 case RVA `0x8253c`, present in the supplied script corpus | [startup.rs:1573](../../crates/crysirror-cmvs/src/vm/startup.rs#L1573) |
| `0x2013` | Required parameter | Handled | Guard: x64 case RVA `0x82549`, present in the supplied script corpus | [startup.rs:1573](../../crates/crysirror-cmvs/src/vm/startup.rs#L1573) |
| `0x2014` | Required parameter | Handled | Guard: x64 case RVA `0x82556`, present in the supplied script corpus | [startup.rs:1573](../../crates/crysirror-cmvs/src/vm/startup.rs#L1573) |
| `0x2015` | Required parameter | Handled | Guard: x64 case RVA `0x82563`, present in the supplied script corpus | [startup.rs:1593](../../crates/crysirror-cmvs/src/vm/startup.rs#L1593) |
| `0x2016` | Required parameter | Handled | Guard: x64 case RVA `0x82570`, present in the supplied script corpus | [startup.rs:1593](../../crates/crysirror-cmvs/src/vm/startup.rs#L1593) |
| `0x2017` | Required parameter | Handled | Guard: x64 case RVA `0x8257d` | [startup.rs:1573](../../crates/crysirror-cmvs/src/vm/startup.rs#L1573) |
| `0x2018` | Required parameter | Handled | Guard: x64 case RVA `0x8258a` | [startup.rs:1573](../../crates/crysirror-cmvs/src/vm/startup.rs#L1573) |
| `0x2019` | The 2021 dispatch table returns from these cases after reclaiming the arguments | Handled | Guard: x64 case RVA `0x82597`, present in the supplied script corpus | [startup.rs:1539](../../crates/crysirror-cmvs/src/vm/startup.rs#L1539) |
| `0x201A` | Required parameter | Handled | Guard: x64 case RVA `0x825a4`, present in the supplied script corpus | [startup.rs:1573](../../crates/crysirror-cmvs/src/vm/startup.rs#L1573) |
| `0x201B` | Control flow (command) | Handled | Guard: x64 case RVA `0x825b1` | [control.rs:167](../../crates/crysirror-cmvs/src/vm/control.rs#L167) |
| `0x201C` | Update job | Handled | Guard: x64 case RVA `0x825c6` | [adapter.rs:3570](../../crates/crysirror-cmvs/src/adapter.rs#L3570) |
| `0x201D` | Required parameter | Handled | Guard: x64 case RVA `0x825d3`, present in the supplied script corpus | [startup.rs:1516](../../crates/crysirror-cmvs/src/vm/startup.rs#L1516) |
| `0x201E` | Required parameter | Handled | Guard: x64 case RVA `0x825f6`, present in the supplied script corpus | [startup.rs:1516](../../crates/crysirror-cmvs/src/vm/startup.rs#L1516) |
| `0x201F` | Required parameter | Handled | Guard: x64 case RVA `0x82619`, present in the supplied script corpus | [startup.rs:1516](../../crates/crysirror-cmvs/src/vm/startup.rs#L1516) |
| `0x2020` | Update job | Handled | Guard: x64 case RVA `0x8263b`, present in the supplied script corpus | [adapter.rs:3996](../../crates/crysirror-cmvs/src/adapter.rs#L3996) |
| `0x2021` | Update job | Handled | Guard: x64 case RVA `0x82648`, present in the supplied script corpus | [adapter.rs:3996](../../crates/crysirror-cmvs/src/adapter.rs#L3996) |
| `0x2022` | Update job | Handled | Guard: x64 case RVA `0x82655`, present in the supplied script corpus | [adapter.rs:3996](../../crates/crysirror-cmvs/src/adapter.rs#L3996) |
| `0x2023` | Update job | Handled | Guard: x64 case RVA `0x82662`, present in the supplied script corpus | [adapter.rs:3996](../../crates/crysirror-cmvs/src/adapter.rs#L3996) |
| `0x2024` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2025` | Execute advance branch | Handled | Guard: x64 case RVA `0x8266f` | [adapter.rs:3748](../../crates/crysirror-cmvs/src/adapter.rs#L3748) |
| `0x2026` | Execute advance branch | Handled | Guard: x64 case RVA `0x8267c` | [adapter.rs:3748](../../crates/crysirror-cmvs/src/adapter.rs#L3748) |
| `0x2027` | Execute advance branch | Handled | Guard: x64 case RVA `0x82689`, present in the supplied script corpus | [adapter.rs:3818](../../crates/crysirror-cmvs/src/adapter.rs#L3818) |
| `0x2028` | Execute advance branch | Handled | Guard: x64 case RVA `0x82696`, present in the supplied script corpus | [adapter.rs:3939](../../crates/crysirror-cmvs/src/adapter.rs#L3939) |
| `0x2029` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x826a3` | [cmvs-20160201.json:8](../../tools/native-api/cmvs-20160201.json#L8) |
| `0x202A` | Execute advance branch | Handled | Guard: x64 case RVA `0x826b0`, present in the supplied script corpus | [adapter.rs:3978](../../crates/crysirror-cmvs/src/adapter.rs#L3978) |
| `0x202B` | Select the render target used as an object's texture | Open | 2016 target-texture source selection requires native offscreen routing; 2021 uses its object error branch; Guard: x64 case RVA `0x826bd` | [adapter.rs:2770](../../crates/crysirror-cmvs/src/adapter.rs#L2770) |
| `0x202C` | Execute advance branch | Handled | Guard: x64 case RVA `0x826ca` | [adapter.rs:3748](../../crates/crysirror-cmvs/src/adapter.rs#L3748) |
| `0x202D` | Execute advance branch | Handled | Guard: x64 case RVA `0x826d7`, present in the supplied script corpus | [adapter.rs:3748](../../crates/crysirror-cmvs/src/adapter.rs#L3748) |
| `0x202E` | Execute advance branch | Handled | Guard: x64 case RVA `0x826e4`, present in the supplied script corpus | [adapter.rs:3978](../../crates/crysirror-cmvs/src/adapter.rs#L3978) |
| `0x202F` | Update job | Handled | Guard: x64 case RVA `0x826f1`, present in the supplied script corpus | [adapter.rs:3996](../../crates/crysirror-cmvs/src/adapter.rs#L3996) |
| `0x2030` | Update job | Handled | Guard: x64 case RVA `0x826fe`, present in the supplied script corpus | [adapter.rs:3996](../../crates/crysirror-cmvs/src/adapter.rs#L3996) |
| `0x2031` | Release the image owned by a scene object | Handled | Guard: x64 case RVA `0x8270b` | [adapter.rs:3953](../../crates/crysirror-cmvs/src/adapter.rs#L3953) |
| `0x2032` | Update job | Handled | Guard: x64 case RVA `0x82718` | [adapter.rs:2632](../../crates/crysirror-cmvs/src/adapter.rs#L2632) |
| `0x2033` | Execute advance branch | Handled | Guard: x64 case RVA `0x82725`, present in the supplied script corpus | [adapter.rs:3964](../../crates/crysirror-cmvs/src/adapter.rs#L3964) |
| `0x2034` | Execute advance branch | Handled | Guard: x64 case RVA `0x82732`, present in the supplied script corpus | [adapter.rs:3493](../../crates/crysirror-cmvs/src/adapter.rs#L3493) |
| `0x2035` | Update job | Handled | Guard: x64 case RVA `0x8273f`, present in the supplied script corpus | [adapter.rs:3534](../../crates/crysirror-cmvs/src/adapter.rs#L3534) |
| `0x2036` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2037` | Update job | Handled | Guard: x64 case RVA `0x8274c` | [adapter.rs:2632](../../crates/crysirror-cmvs/src/adapter.rs#L2632) |
| `0x2038` | Update job | Handled | Guard: x64 case RVA `0x82759`, present in the supplied script corpus | [adapter.rs:3171](../../crates/crysirror-cmvs/src/adapter.rs#L3171) |
| `0x2039` | Execute advance branch | Handled | Guard: x64 case RVA `0x82766` | [adapter.rs:3748](../../crates/crysirror-cmvs/src/adapter.rs#L3748) |
| `0x203A` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x203B` | Update job | Handled | Guard: x64 case RVA `0x82773`, present in the supplied script corpus | [adapter.rs:2632](../../crates/crysirror-cmvs/src/adapter.rs#L2632) |
| `0x203C` | Execute advance branch | Handled | Guard: x64 case RVA `0x82780`, present in the supplied script corpus | [adapter.rs:2741](../../crates/crysirror-cmvs/src/adapter.rs#L2741) |
| `0x203D` | Execute advance branch | Handled | Guard: x64 case RVA `0x8278d` | [adapter.rs:2741](../../crates/crysirror-cmvs/src/adapter.rs#L2741) |
| `0x203E` | Execute advance branch | Handled | Guard: x64 case RVA `0x8279a`, present in the supplied script corpus | [adapter.rs:2741](../../crates/crysirror-cmvs/src/adapter.rs#L2741) |
| `0x203F` | Execute advance branch | Handled | Guard: x64 case RVA `0x827a7`, present in the supplied script corpus | [adapter.rs:3412](../../crates/crysirror-cmvs/src/adapter.rs#L3412) |
| `0x2040` | Update job | Handled | Guard: x64 case RVA `0x827be`, present in the supplied script corpus | [adapter.rs:3996](../../crates/crysirror-cmvs/src/adapter.rs#L3996) |
| `0x2041` | Execute advance branch | Handled | Guard: x64 case RVA `0x827cb` | [adapter.rs:3748](../../crates/crysirror-cmvs/src/adapter.rs#L3748) |
| `0x2042` | Execute advance branch | Handled | Guard: x64 case RVA `0x827d8`, present in the supplied script corpus | [adapter.rs:3748](../../crates/crysirror-cmvs/src/adapter.rs#L3748) |
| `0x2043` | Execute advance branch | Handled | Guard: x64 case RVA `0x827e5`, present in the supplied script corpus | [adapter.rs:3748](../../crates/crysirror-cmvs/src/adapter.rs#L3748) |
| `0x2044` | Update job | Handled | Guard: x64 case RVA `0x827f2`, present in the supplied script corpus | [adapter.rs:3996](../../crates/crysirror-cmvs/src/adapter.rs#L3996) |
| `0x2045` | Update job | Handled | Guard: x64 case RVA `0x827ff`, present in the supplied script corpus | [adapter.rs:3996](../../crates/crysirror-cmvs/src/adapter.rs#L3996) |
| `0x2046` | Update job | Handled | Guard: x64 case RVA `0x8280c`, present in the supplied script corpus | [adapter.rs:3996](../../crates/crysirror-cmvs/src/adapter.rs#L3996) |
| `0x2047` | Update job | Handled | Guard: x64 case RVA `0x82819`, present in the supplied script corpus | [adapter.rs:3996](../../crates/crysirror-cmvs/src/adapter.rs#L3996) |
| `0x2048` | Update job | Handled | Guard: x64 case RVA `0x82826`, present in the supplied script corpus | [adapter.rs:3996](../../crates/crysirror-cmvs/src/adapter.rs#L3996) |
| `0x2049` | Execute advance branch | Handled | Guard: x64 case RVA `0x82833`, present in the supplied script corpus | [adapter.rs:3748](../../crates/crysirror-cmvs/src/adapter.rs#L3748) |
| `0x204A` | Execute advance branch | Handled | Guard: x64 case RVA `0x82840`, present in the supplied script corpus | [adapter.rs:3748](../../crates/crysirror-cmvs/src/adapter.rs#L3748) |
| `0x204B` | Execute advance branch | Handled | Guard: x64 case RVA `0x8284d`, present in the supplied script corpus | [adapter.rs:3748](../../crates/crysirror-cmvs/src/adapter.rs#L3748) |
| `0x204C` | Execute advance branch | Handled | Guard: x64 case RVA `0x8285a`, present in the supplied script corpus | [adapter.rs:3748](../../crates/crysirror-cmvs/src/adapter.rs#L3748) |
| `0x204D` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x204E` | Execute advance branch | Handled | Guard: x64 case RVA `0x82867` | [adapter.rs:3748](../../crates/crysirror-cmvs/src/adapter.rs#L3748) |
| `0x204F` | Execute advance branch | Handled | Guard: x64 case RVA `0x82874` | [adapter.rs:3748](../../crates/crysirror-cmvs/src/adapter.rs#L3748) |
| `0x2050` | Update job | Handled | Guard: x64 case RVA `0x82881`, present in the supplied script corpus | [adapter.rs:3996](../../crates/crysirror-cmvs/src/adapter.rs#L3996) |
| `0x2051` | Execute advance branch | Handled | Guard: x64 case RVA `0x8288e` | [adapter.rs:3748](../../crates/crysirror-cmvs/src/adapter.rs#L3748) |
| `0x2052` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2053` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2054` | Execute advance branch | Handled | Guard: x64 case RVA `0x8289b` | [adapter.rs:3748](../../crates/crysirror-cmvs/src/adapter.rs#L3748) |
| `0x2055` | Execute advance branch | Handled | Guard: x64 case RVA `0x828a8` | [adapter.rs:3748](../../crates/crysirror-cmvs/src/adapter.rs#L3748) |
| `0x2056` | Execute advance branch | Handled | Guard: x64 case RVA `0x828b5`, present in the supplied script corpus | [adapter.rs:3197](../../crates/crysirror-cmvs/src/adapter.rs#L3197) |
| `0x2057` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2058` | Execute command branch | Open | Retains native registers; only the implemented 2D paths consume them; Guard: x64 case RVA `0x828c2`, present in the supplied script corpus | [screen.rs:245](../../crates/crysirror-cmvs/src/vm/screen.rs#L245) |
| `0x2059` | Execute command branch | Open | Retains native registers; only the implemented 2D paths consume them; Guard: x64 case RVA `0x828cf`, present in the supplied script corpus | [screen.rs:245](../../crates/crysirror-cmvs/src/vm/screen.rs#L245) |
| `0x205A` | Execute command branch | Open | Retains native registers; only the implemented 2D paths consume them; Guard: x64 case RVA `0x828dc`, present in the supplied script corpus | [screen.rs:245](../../crates/crysirror-cmvs/src/vm/screen.rs#L245) |
| `0x205B` | Execute command branch | Open | Retains native registers; only the implemented 2D paths consume them; Guard: x64 case RVA `0x828e9`, present in the supplied script corpus | [screen.rs:245](../../crates/crysirror-cmvs/src/vm/screen.rs#L245) |
| `0x205C` | Execute command branch | Open | Retains native registers; only the implemented 2D paths consume them; Guard: x64 case RVA `0x828f6`, present in the supplied script corpus | [screen.rs:245](../../crates/crysirror-cmvs/src/vm/screen.rs#L245) |
| `0x205D` | Execute command branch | Open | Retains native registers; only the implemented 2D paths consume them; Guard: x64 case RVA `0x82903` | [screen.rs:245](../../crates/crysirror-cmvs/src/vm/screen.rs#L245) |
| `0x205E` | Execute command branch | Open | Retains native registers; only the implemented 2D paths consume them; Guard: x64 case RVA `0x82910`, present in the supplied script corpus | [screen.rs:245](../../crates/crysirror-cmvs/src/vm/screen.rs#L245) |
| `0x205F` | Execute command branch | Open | Retains native registers; only the implemented 2D paths consume them; Guard: x64 case RVA `0x8291d`, present in the supplied script corpus | [screen.rs:245](../../crates/crysirror-cmvs/src/vm/screen.rs#L245) |
| `0x2060` | Execute command branch | Open | Retains native registers; only the implemented 2D paths consume them; Guard: x64 case RVA `0x8292a` | [screen.rs:245](../../crates/crysirror-cmvs/src/vm/screen.rs#L245) |
| `0x2061` | Execute command branch | Open | Retains native registers; only the implemented 2D paths consume them; Guard: x64 case RVA `0x82937` | [screen.rs:245](../../crates/crysirror-cmvs/src/vm/screen.rs#L245) |
| `0x2062` | Execute command branch | Open | Retains native registers; only the implemented 2D paths consume them; Guard: x64 case RVA `0x82944`, present in the supplied script corpus | [screen.rs:245](../../crates/crysirror-cmvs/src/vm/screen.rs#L245) |
| `0x2063` | Execute command branch | Open | Retains native registers; only the implemented 2D paths consume them; Guard: x64 case RVA `0x82951` | [screen.rs:245](../../crates/crysirror-cmvs/src/vm/screen.rs#L245) |
| `0x2064` | Execute command branch | Open | Retains native registers; only the implemented 2D paths consume them; Guard: x64 case RVA `0x8295e` | [screen.rs:245](../../crates/crysirror-cmvs/src/vm/screen.rs#L245) |
| `0x2065` | Execute command branch | Open | Retains native registers; only the implemented 2D paths consume them; Guard: x64 case RVA `0x8296b`, present in the supplied script corpus | [screen.rs:245](../../crates/crysirror-cmvs/src/vm/screen.rs#L245) |
| `0x2066` | Execute command branch | Open | Retains native registers; only the implemented 2D paths consume them; Guard: x64 case RVA `0x82978`, present in the supplied script corpus | [screen.rs:245](../../crates/crysirror-cmvs/src/vm/screen.rs#L245) |
| `0x2067` | Execute command branch | Open | Retains native registers; only the implemented 2D paths consume them; Guard: x64 case RVA `0x82985`, present in the supplied script corpus | [screen.rs:245](../../crates/crysirror-cmvs/src/vm/screen.rs#L245) |
| `0x2068` | Execute command branch | Open | Retains native registers; only the implemented 2D paths consume them; Guard: x64 case RVA `0x82992`, present in the supplied script corpus | [screen.rs:245](../../crates/crysirror-cmvs/src/vm/screen.rs#L245) |
| `0x2069` | Execute advance branch | Handled | Guard: x64 case RVA `0x8299f`, present in the supplied script corpus | [adapter.rs:2587](../../crates/crysirror-cmvs/src/adapter.rs#L2587) |
| `0x206A` | Execute command branch | Handled | Guard: x64 case RVA `0x829ac`, present in the supplied script corpus | [screen.rs:221](../../crates/crysirror-cmvs/src/vm/screen.rs#L221) |
| `0x206B` | Start tween | Handled | Guard: x64 case RVA `0x829b9` | [screen.rs:220](../../crates/crysirror-cmvs/src/vm/screen.rs#L220) |
| `0x206C` | Start tween | Handled | Guard: x64 case RVA `0x829c6`, present in the supplied script corpus | [screen.rs:220](../../crates/crysirror-cmvs/src/vm/screen.rs#L220) |
| `0x206D` | Start tween | Handled | Guard: x64 case RVA `0x829d3` | [screen.rs:220](../../crates/crysirror-cmvs/src/vm/screen.rs#L220) |
| `0x206E` | Start tween | Handled | Guard: x64 case RVA `0x829e0` | [screen.rs:220](../../crates/crysirror-cmvs/src/vm/screen.rs#L220) |
| `0x206F` | Start tween | Handled | Guard: x64 case RVA `0x829ed` | [screen.rs:220](../../crates/crysirror-cmvs/src/vm/screen.rs#L220) |
| `0x2070` | Execute advance branch | Handled | Guard: x64 case RVA `0x829fa`, present in the supplied script corpus | [adapter.rs:3748](../../crates/crysirror-cmvs/src/adapter.rs#L3748) |
| `0x2071` | Execute advance branch | Handled | Guard: x64 case RVA `0x82a07`, present in the supplied script corpus | [adapter.rs:3748](../../crates/crysirror-cmvs/src/adapter.rs#L3748) |
| `0x2072` | Execute advance branch | Handled | Guard: x64 case RVA `0x82a14`, present in the supplied script corpus | [adapter.rs:3748](../../crates/crysirror-cmvs/src/adapter.rs#L3748) |
| `0x2073` | Execute advance branch | Handled | Guard: x64 case RVA `0x82a21`, present in the supplied script corpus | [adapter.rs:3748](../../crates/crysirror-cmvs/src/adapter.rs#L3748) |
| `0x2074` | Execute advance branch | Handled | Guard: x64 case RVA `0x82a2e`, present in the supplied script corpus | [adapter.rs:3748](../../crates/crysirror-cmvs/src/adapter.rs#L3748) |
| `0x2075` | Execute advance branch | Handled | Guard: x64 case RVA `0x82a3b`, present in the supplied script corpus | [adapter.rs:3748](../../crates/crysirror-cmvs/src/adapter.rs#L3748) |
| `0x2076` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2077` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2078` | Execute advance branch | Handled | Guard: x64 case RVA `0x82a48`, present in the supplied script corpus | [adapter.rs:3748](../../crates/crysirror-cmvs/src/adapter.rs#L3748) |
| `0x2079` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x207A` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x207B` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x207C` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x207D` | Start tween | Handled | Guard: x64 case RVA `0x82a55` | [screen.rs:220](../../crates/crysirror-cmvs/src/vm/screen.rs#L220) |
| `0x207E` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x207F` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2080` | Update job | Handled | Guard: x64 case RVA `0x82a62`, present in the supplied script corpus | [adapter.rs:4049](../../crates/crysirror-cmvs/src/adapter.rs#L4049) |
| `0x2081` | Update job | Handled | Guard: x64 case RVA `0x82a6f`, present in the supplied script corpus | [adapter.rs:4049](../../crates/crysirror-cmvs/src/adapter.rs#L4049) |
| `0x2082` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x82a7c` | [cmvs-20160201.json:14](../../tools/native-api/cmvs-20160201.json#L14) |
| `0x2083` | Control flow (command) | Handled | Guard: x64 case RVA `0x82a89` | [control.rs:355](../../crates/crysirror-cmvs/src/vm/control.rs#L355) |
| `0x2084` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2085` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2086` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2087` | Execute advance branch | Handled | Guard: x64 case RVA `0x82ab3` | [adapter.rs:2359](../../crates/crysirror-cmvs/src/adapter.rs#L2359) |
| `0x2088` | Control flow (command) | Handled | Guard: x64 case RVA `0x82ac0`, present in the supplied script corpus | [control.rs:361](../../crates/crysirror-cmvs/src/vm/control.rs#L361) |
| `0x2089` | Control flow (command) | Handled | Guard: x64 case RVA `0x82acd`, present in the supplied script corpus | [control.rs:393](../../crates/crysirror-cmvs/src/vm/control.rs#L393) |
| `0x208A` | Invoke a registered callback | Handled | Guard: x64 case RVA `0x82af9`, present in the supplied script corpus | [startup.rs:1492](../../crates/crysirror-cmvs/src/vm/startup.rs#L1492) |
| `0x208B` | Control flow (command) | Handled | Guard: x64 case RVA `0x82b06`, present in the supplied script corpus | [control.rs:361](../../crates/crysirror-cmvs/src/vm/control.rs#L361) |
| `0x208C` | Control flow (command) | Handled | Guard: x64 case RVA `0x82b13`, present in the supplied script corpus | [control.rs:393](../../crates/crysirror-cmvs/src/vm/control.rs#L393) |
| `0x208D` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x208E` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x208F` | Update job | Handled | Guard: x64 case RVA `0x82b3f`, present in the supplied script corpus | [adapter.rs:1956](../../crates/crysirror-cmvs/src/adapter.rs#L1956) |
| `0x2090` | Update settings / engine field 12084 | Handled | Guard: x64 case RVA `0x82b4c`, present in the supplied script corpus | [startup.rs:1526](../../crates/crysirror-cmvs/src/vm/startup.rs#L1526) |
| `0x2091` | Fill variables | Handled | Guard: x64 case RVA `0x82b61`, present in the supplied script corpus | [startup.rs:1530](../../crates/crysirror-cmvs/src/vm/startup.rs#L1530) |
| `0x2092` | Fill variables | Handled | Guard: x64 case RVA `0x82b6e`, present in the supplied script corpus | [startup.rs:1530](../../crates/crysirror-cmvs/src/vm/startup.rs#L1530) |
| `0x2093` | Fill variables | Handled | Guard: x64 case RVA `0x82b7b`, present in the supplied script corpus | [startup.rs:1530](../../crates/crysirror-cmvs/src/vm/startup.rs#L1530) |
| `0x2094` | Fill variables | Handled | Guard: x64 case RVA `0x82b88`, present in the supplied script corpus | [startup.rs:1530](../../crates/crysirror-cmvs/src/vm/startup.rs#L1530) |
| `0x2095` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2096` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2097` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2098` | Required parameter | Handled | Guard: x64 case RVA `0x82b95`, present in the supplied script corpus | [startup.rs:1602](../../crates/crysirror-cmvs/src/vm/startup.rs#L1602) |
| `0x2099` | Update job | Handled | Guard: x64 case RVA `0x82ba2`, present in the supplied script corpus | [adapter.rs:3570](../../crates/crysirror-cmvs/src/adapter.rs#L3570) |
| `0x209A` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x209B` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x209C` | Execute command branch | Open | Retains native registers; only the implemented 2D paths consume them; Guard: x64 case RVA `0x82baf`, present in the supplied script corpus | [screen.rs:245](../../crates/crysirror-cmvs/src/vm/screen.rs#L245) |
| `0x209D` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x209E` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x209F` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x20A0` | Update job | Handled | Guard: x64 case RVA `0x82bbc`, present in the supplied script corpus | [adapter.rs:2054](../../crates/crysirror-cmvs/src/adapter.rs#L2054) |
| `0x20A1` | Execute advance branch | Handled | Guard: x64 case RVA `0x82bc9`, present in the supplied script corpus | [adapter.rs:2072](../../crates/crysirror-cmvs/src/adapter.rs#L2072) |
| `0x20A2` | Execute advance branch | Handled | Guard: x64 case RVA `0x82be0`, present in the supplied script corpus | [adapter.rs:2076](../../crates/crysirror-cmvs/src/adapter.rs#L2076) |
| `0x20A3` | Execute advance branch | Handled | Guard: x64 case RVA `0x82bed` | [adapter.rs:3213](../../crates/crysirror-cmvs/src/adapter.rs#L3213) |
| `0x20A4` | Update job | Handled | Guard: x64 case RVA `0x82bfa`, present in the supplied script corpus | [adapter.rs:2054](../../crates/crysirror-cmvs/src/adapter.rs#L2054) |
| `0x20A5` | Execute advance branch | Open | Device attenuation affects software PCM and subsequent hardware sample starts; driver parity is pending; Guard: x64 case RVA `0x82c07` | [adapter.rs:2081](../../crates/crysirror-cmvs/src/adapter.rs#L2081) |
| `0x20A6` | Update job | Open | MOG length result unit is unverified; Guard: x64 case RVA `0x82c14`, present in the supplied script corpus | [adapter.rs:2088](../../crates/crysirror-cmvs/src/adapter.rs#L2088) |
| `0x20A7` | Activate the allocated voice at its current decoder cursor | Handled | Guard: x64 case RVA `0x82c21` | [adapter.rs:2116](../../crates/crysirror-cmvs/src/adapter.rs#L2116) |
| `0x20A8` | Execute advance branch | Handled | Guard: x64 case RVA `0x82c2e`, present in the supplied script corpus | [adapter.rs:2143](../../crates/crysirror-cmvs/src/adapter.rs#L2143) |
| `0x20A9` | Execute advance branch | Handled | Guard: x64 case RVA `0x82c3b`, present in the supplied script corpus | [adapter.rs:2143](../../crates/crysirror-cmvs/src/adapter.rs#L2143) |
| `0x20AA` | Execute advance branch | Open | Elapsed mixer frames are available; original device cursor and buffer lead are pending; Guard: x64 case RVA `0x82c48`, present in the supplied script corpus | [adapter.rs:2143](../../crates/crysirror-cmvs/src/adapter.rs#L2143) |
| `0x20AB` | Execute advance branch | Handled | Guard: x64 case RVA `0x82c55` | [adapter.rs:3217](../../crates/crysirror-cmvs/src/adapter.rs#L3217) |
| `0x20AC` | Execute advance branch | Open | Quantized per-voice gain is connected; native fade and device PCM timing need parity evidence; Guard: x64 case RVA `0x82c62` | [adapter.rs:2220](../../crates/crysirror-cmvs/src/adapter.rs#L2220) |
| `0x20AD` | Execute advance branch | Open | Quantized per-voice gain is connected; native fade and device PCM timing need parity evidence; Guard: x64 case RVA `0x82c6f` | [adapter.rs:2220](../../crates/crysirror-cmvs/src/adapter.rs#L2220) |
| `0x20AE` | Execute advance branch | Open | Buffered source levels are connected; output resampling can differ from DirectSound; Guard: x64 case RVA `0x82c7c` | [adapter.rs:2247](../../crates/crysirror-cmvs/src/adapter.rs#L2247) |
| `0x20AF` | Update job | Open | MOG length and rate-conversion parity are unverified; Guard: x64 case RVA `0x82c89`, present in the supplied script corpus | [adapter.rs:2088](../../crates/crysirror-cmvs/src/adapter.rs#L2088) |
| `0x20B0` | Update job | Handled | Guard: x64 case RVA `0x82c96`, present in the supplied script corpus | [adapter.rs:1984](../../crates/crysirror-cmvs/src/adapter.rs#L1984) |
| `0x20B1` | Update job | Handled | Guard: x64 case RVA `0x82ca3`, present in the supplied script corpus | [adapter.rs:1984](../../crates/crysirror-cmvs/src/adapter.rs#L1984) |
| `0x20B2` | Pause the effect slot and retain its decoder | Handled | Guard: x64 case RVA `0x82cb0` | [adapter.rs:2018](../../crates/crysirror-cmvs/src/adapter.rs#L2018) |
| `0x20B3` | Update job | Handled | Guard: x64 case RVA `0x82cbd`, present in the supplied script corpus | [adapter.rs:1984](../../crates/crysirror-cmvs/src/adapter.rs#L1984) |
| `0x20B4` | Rewind and activate the allocated effect | Handled | Guard: x64 case RVA `0x82cca` | [adapter.rs:2018](../../crates/crysirror-cmvs/src/adapter.rs#L2018) |
| `0x20B5` | Execute advance branch | Handled | Guard: x64 case RVA `0x82cd7`, present in the supplied script corpus | [adapter.rs:2012](../../crates/crysirror-cmvs/src/adapter.rs#L2012) |
| `0x20B6` | Execute advance branch | Handled | Guard: x64 case RVA `0x82ce4`, present in the supplied script corpus | [adapter.rs:2041](../../crates/crysirror-cmvs/src/adapter.rs#L2041) |
| `0x20B7` | Execute advance branch | Open | Elapsed mixer frames are available; original device cursor and buffer lead are pending; Guard: x64 case RVA `0x82cf1`, present in the supplied script corpus | [adapter.rs:2041](../../crates/crysirror-cmvs/src/adapter.rs#L2041) |
| `0x20B8` | Execute advance branch | Open | Quantized per-voice gain is connected; native fade and device PCM timing need parity evidence; Guard: x64 case RVA `0x82cfe` | [adapter.rs:2220](../../crates/crysirror-cmvs/src/adapter.rs#L2220) |
| `0x20B9` | Execute advance branch | Open | Quantized per-voice gain is connected; native fade and device PCM timing need parity evidence; Guard: x64 case RVA `0x82d0b` | [adapter.rs:2220](../../crates/crysirror-cmvs/src/adapter.rs#L2220) |
| `0x20BA` | Update job | Handled | Guard: x64 case RVA `0x82d18`, present in the supplied script corpus | [adapter.rs:1984](../../crates/crysirror-cmvs/src/adapter.rs#L1984) |
| `0x20BB` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x20BC` | Mark the allocated voice for mixer retirement | Handled | Guard: x64 case RVA `0x82d25` | [adapter.rs:2127](../../crates/crysirror-cmvs/src/adapter.rs#L2127) |
| `0x20BD` | Wait for the mixer to release the voice slot | Handled | Guard: x64 case RVA `0x82d32` | [adapter.rs:2127](../../crates/crysirror-cmvs/src/adapter.rs#L2127) |
| `0x20BE` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x20BF` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x20C0` | Update job | Handled | Guard: x64 case RVA `0x82d3f`, present in the supplied script corpus | [adapter.rs:2325](../../crates/crysirror-cmvs/src/adapter.rs#L2325) |
| `0x20C1` | Execute advance branch | Handled | Guard: x64 case RVA `0x82d4c`, present in the supplied script corpus | [adapter.rs:2339](../../crates/crysirror-cmvs/src/adapter.rs#L2339) |
| `0x20C2` | Execute advance branch | Handled | Guard: x64 case RVA `0x82d59`, present in the supplied script corpus | [adapter.rs:2343](../../crates/crysirror-cmvs/src/adapter.rs#L2343) |
| `0x20C3` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x20C4` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x20C5` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x20C6` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x20C7` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x20C8` | Input (command) | Handled | Guard: x64 case RVA `0x82d66`, present in the supplied script corpus | [input.rs:214](../../crates/crysirror-cmvs/src/vm/input.rs#L214) |
| `0x20C9` | Input (command) | Handled | Guard: x64 case RVA `0x82d73` | [input.rs:219](../../crates/crysirror-cmvs/src/vm/input.rs#L219) |
| `0x20CA` | Execute advance branch | Handled | Guard: x64 case RVA `0x82d80`, present in the supplied script corpus | [adapter.rs:2550](../../crates/crysirror-cmvs/src/adapter.rs#L2550) |
| `0x20CB` | Update job | Handled | Guard: x64 case RVA `0x82d8d` | [adapter.rs:3570](../../crates/crysirror-cmvs/src/adapter.rs#L3570) |
| `0x20CC` | Update job | Handled | Guard: x64 case RVA `0x82d9a` | [adapter.rs:3570](../../crates/crysirror-cmvs/src/adapter.rs#L3570) |
| `0x20CD` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x82da7` | [cmvs-20160201.json:18](../../tools/native-api/cmvs-20160201.json#L18) |
| `0x20CE` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x20CF` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x20D0` | Control flow (command) | Handled | Guard: x64 case RVA `0x82db4`, present in the supplied script corpus | [control.rs:224](../../crates/crysirror-cmvs/src/vm/control.rs#L224) |
| `0x20D1` | Control flow (command) | Handled | Guard: x64 case RVA `0x82ddc` | [control.rs:224](../../crates/crysirror-cmvs/src/vm/control.rs#L224) |
| `0x20D2` | Control flow (command) | Handled | Guard: x64 case RVA `0x82e1f` | [control.rs:224](../../crates/crysirror-cmvs/src/vm/control.rs#L224) |
| `0x20D3` | Control flow (command) | Handled | Guard: x64 case RVA `0x82e4f` | [control.rs:224](../../crates/crysirror-cmvs/src/vm/control.rs#L224) |
| `0x20D4` | Control flow (command) | Handled | Guard: x64 case RVA `0x82e7f`, present in the supplied script corpus | [control.rs:206](../../crates/crysirror-cmvs/src/vm/control.rs#L206) |
| `0x20D5` | Control flow (command) | Handled | Guard: x64 case RVA `0x82e8c` | [control.rs:206](../../crates/crysirror-cmvs/src/vm/control.rs#L206) |
| `0x20D6` | Control flow (command) | Handled | Guard: x64 case RVA `0x82e99` | [control.rs:247](../../crates/crysirror-cmvs/src/vm/control.rs#L247) |
| `0x20D7` | Control flow (command) | Handled | Guard: x64 case RVA `0x82ea6` | [control.rs:257](../../crates/crysirror-cmvs/src/vm/control.rs#L257) |
| `0x20D8` | Control flow (command) | Open | Native trigonometric results at truncation boundaries need verification; Guard: x64 case RVA `0x82eb3` | [control.rs:280](../../crates/crysirror-cmvs/src/vm/control.rs#L280) |
| `0x20D9` | Control flow (command) | Open | Native trigonometric results at truncation boundaries need verification; Guard: x64 case RVA `0x82ec0` | [control.rs:280](../../crates/crysirror-cmvs/src/vm/control.rs#L280) |
| `0x20DA` | Control flow (command) | Open | Native trigonometric results at truncation boundaries need verification; Guard: x64 case RVA `0x82ecd` | [control.rs:303](../../crates/crysirror-cmvs/src/vm/control.rs#L303) |
| `0x20DB` | Control flow (command) | Open | Native trigonometric results at truncation boundaries need verification; Guard: x64 case RVA `0x82eda` | [control.rs:303](../../crates/crysirror-cmvs/src/vm/control.rs#L303) |
| `0x20DC` | Control flow (command) | Open | Native trigonometric results at truncation boundaries need verification; Guard: x64 case RVA `0x82ee7` | [control.rs:280](../../crates/crysirror-cmvs/src/vm/control.rs#L280) |
| `0x20DD` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x20DE` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x20DF` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x20E0` | Control flow (command) | Open | Native trigonometric results at truncation boundaries need verification; Guard: x64 case RVA `0x82ef4` | [control.rs:333](../../crates/crysirror-cmvs/src/vm/control.rs#L333) |
| `0x20E1` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x20E2` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x20E3` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x20E4` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x20E5` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x20E6` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x20E7` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x20E8` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x82f01` | [cmvs-20160201.json:20](../../tools/native-api/cmvs-20160201.json#L20) |
| `0x20E9` | Control flow (command) | Handled | Guard: x64 case RVA `0x825b1` | [control.rs:167](../../crates/crysirror-cmvs/src/vm/control.rs#L167) |
| `0x20EA` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x20EB` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x20EC` | Execute advance branch | Handled | Guard: x64 case RVA `0x82f24`, present in the supplied script corpus | [adapter.rs:2369](../../crates/crysirror-cmvs/src/adapter.rs#L2369) |
| `0x20ED` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x20EE` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x20EF` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x20F0` | Execute startup command branch | Handled | Guard: x64 case RVA `0x82f31` | [startup.rs:1644](../../crates/crysirror-cmvs/src/vm/startup.rs#L1644) |
| `0x20F1` | Execute startup command branch | Handled | Guard: x64 case RVA `0x82f3e`, present in the supplied script corpus | [startup.rs:1644](../../crates/crysirror-cmvs/src/vm/startup.rs#L1644) |
| `0x20F2` | Execute startup command branch | Handled | Guard: x64 case RVA `0x82f4b` | [startup.rs:1644](../../crates/crysirror-cmvs/src/vm/startup.rs#L1644) |
| `0x20F3` | Execute startup command branch | Handled | Guard: x64 case RVA `0x82f58`, present in the supplied script corpus | [startup.rs:1644](../../crates/crysirror-cmvs/src/vm/startup.rs#L1644) |
| `0x20F4` | Execute startup command branch | Handled | Guard: x64 case RVA `0x82f65` | [startup.rs:1644](../../crates/crysirror-cmvs/src/vm/startup.rs#L1644) |
| `0x20F5` | Execute startup command branch | Handled | Guard: x64 case RVA `0x82f72` | [startup.rs:1644](../../crates/crysirror-cmvs/src/vm/startup.rs#L1644) |
| `0x20F6` | Execute startup command branch | Handled | Guard: x64 case RVA `0x82f7f` | [startup.rs:1644](../../crates/crysirror-cmvs/src/vm/startup.rs#L1644) |
| `0x20F7` | Execute startup command branch | Handled | Guard: x64 case RVA `0x82f8c` | [startup.rs:1644](../../crates/crysirror-cmvs/src/vm/startup.rs#L1644) |
| `0x20F8` | Execute startup command branch | Handled | Guard: x64 case RVA `0x82f99`, present in the supplied script corpus | [startup.rs:1644](../../crates/crysirror-cmvs/src/vm/startup.rs#L1644) |
| `0x20F9` | Execute startup command branch | Handled | Guard: x64 case RVA `0x82fa6` | [startup.rs:1644](../../crates/crysirror-cmvs/src/vm/startup.rs#L1644) |
| `0x20FA` | Execute startup command branch | Handled | Guard: x64 case RVA `0x82fb3` | [startup.rs:1644](../../crates/crysirror-cmvs/src/vm/startup.rs#L1644) |
| `0x20FB` | Execute startup command branch | Handled | Guard: x64 case RVA `0x82fc0` | [startup.rs:1644](../../crates/crysirror-cmvs/src/vm/startup.rs#L1644) |
| `0x20FC` | Execute startup command branch | Open | Overlapping format buffers and native pointer reinterpretation are rejected; Guard: x64 case RVA `0x82fcd`, present in the supplied script corpus | [startup.rs:1644](../../crates/crysirror-cmvs/src/vm/startup.rs#L1644) |
| `0x20FD` | Execute startup command branch | Open | Overlapping format buffers and native pointer reinterpretation are rejected; Guard: x64 case RVA `0x82fda` | [startup.rs:1644](../../crates/crysirror-cmvs/src/vm/startup.rs#L1644) |
| `0x20FE` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x20FF` | Execute startup command branch | Handled | Guard: x64 case RVA `0x82fe7` | [startup.rs:1644](../../crates/crysirror-cmvs/src/vm/startup.rs#L1644) |
| `0x2100` | Execute advance branch | Handled | Guard: x64 case RVA `0x82ff4`, present in the supplied script corpus | [adapter.rs:3773](../../crates/crysirror-cmvs/src/adapter.rs#L3773) |
| `0x2101` | Execute advance branch | Handled | Guard: x64 case RVA `0x83001`, present in the supplied script corpus | [adapter.rs:3773](../../crates/crysirror-cmvs/src/adapter.rs#L3773) |
| `0x2102` | Execute advance branch | Handled | Guard: x64 case RVA `0x8300e`, present in the supplied script corpus | [adapter.rs:3773](../../crates/crysirror-cmvs/src/adapter.rs#L3773) |
| `0x2103` | Execute advance branch | Handled | Guard: x64 case RVA `0x8301b`, present in the supplied script corpus | [adapter.rs:3773](../../crates/crysirror-cmvs/src/adapter.rs#L3773) |
| `0x2104` | Execute advance branch | Handled | Guard: x64 case RVA `0x83028`, present in the supplied script corpus | [adapter.rs:3773](../../crates/crysirror-cmvs/src/adapter.rs#L3773) |
| `0x2105` | Execute advance branch | Handled | Guard: x64 case RVA `0x83035`, present in the supplied script corpus | [adapter.rs:3773](../../crates/crysirror-cmvs/src/adapter.rs#L3773) |
| `0x2106` | Execute advance branch | Open | Scoped font aliases are connected; native GDI font metrics remain open; Guard: x64 case RVA `0x83042`, present in the supplied script corpus | [adapter.rs:2540](../../crates/crysirror-cmvs/src/adapter.rs#L2540) |
| `0x2107` | Execute advance branch | Handled | Guard: x64 case RVA `0x8304f`, present in the supplied script corpus | [adapter.rs:3773](../../crates/crysirror-cmvs/src/adapter.rs#L3773) |
| `0x2108` | Execute advance branch | Handled | Guard: x64 case RVA `0x8305c` | [adapter.rs:3773](../../crates/crysirror-cmvs/src/adapter.rs#L3773) |
| `0x2109` | Execute advance branch | Handled | Guard: x64 case RVA `0x83069` | [adapter.rs:3773](../../crates/crysirror-cmvs/src/adapter.rs#L3773) |
| `0x210A` | Execute advance branch | Handled | Guard: x64 case RVA `0x83076` | [adapter.rs:3773](../../crates/crysirror-cmvs/src/adapter.rs#L3773) |
| `0x210B` | Execute advance branch | Handled | Guard: x64 case RVA `0x83083`, present in the supplied script corpus | [adapter.rs:3773](../../crates/crysirror-cmvs/src/adapter.rs#L3773) |
| `0x210C` | Execute advance branch | Handled | Guard: x64 case RVA `0x83090`, present in the supplied script corpus | [adapter.rs:3773](../../crates/crysirror-cmvs/src/adapter.rs#L3773) |
| `0x210D` | Execute advance branch | Handled | Guard: x64 case RVA `0x8309d`, present in the supplied script corpus | [adapter.rs:3773](../../crates/crysirror-cmvs/src/adapter.rs#L3773) |
| `0x210E` | Execute advance branch | Handled | Guard: x64 case RVA `0x830aa`, present in the supplied script corpus | [adapter.rs:3773](../../crates/crysirror-cmvs/src/adapter.rs#L3773) |
| `0x210F` | Execute advance branch | Handled | Guard: x64 case RVA `0x830b7`, present in the supplied script corpus | [adapter.rs:3773](../../crates/crysirror-cmvs/src/adapter.rs#L3773) |
| `0x2110` | Execute advance branch | Handled | Guard: x64 case RVA `0x830c4`, present in the supplied script corpus | [adapter.rs:3773](../../crates/crysirror-cmvs/src/adapter.rs#L3773) |
| `0x2111` | Execute advance branch | Handled | Guard: x64 case RVA `0x830d1` | [adapter.rs:3773](../../crates/crysirror-cmvs/src/adapter.rs#L3773) |
| `0x2112` | Execute advance branch | Handled | Guard: x64 case RVA `0x830de` | [adapter.rs:2369](../../crates/crysirror-cmvs/src/adapter.rs#L2369) |
| `0x2113` | Execute advance branch | Handled | Guard: x64 case RVA `0x830eb`, present in the supplied script corpus | [adapter.rs:3773](../../crates/crysirror-cmvs/src/adapter.rs#L3773) |
| `0x2114` | Execute advance branch | Handled | Guard: x64 case RVA `0x830f8`, present in the supplied script corpus | [adapter.rs:2369](../../crates/crysirror-cmvs/src/adapter.rs#L2369) |
| `0x2115` | Execute advance branch | Handled | Guard: x64 case RVA `0x83105`, present in the supplied script corpus | [adapter.rs:2369](../../crates/crysirror-cmvs/src/adapter.rs#L2369) |
| `0x2116` | Execute advance branch | Handled | Guard: x64 case RVA `0x83112`, present in the supplied script corpus | [adapter.rs:2369](../../crates/crysirror-cmvs/src/adapter.rs#L2369) |
| `0x2117` | Execute advance branch | Handled | Guard: x64 case RVA `0x8311f`, present in the supplied script corpus | [adapter.rs:2369](../../crates/crysirror-cmvs/src/adapter.rs#L2369) |
| `0x2118` | Execute advance branch | Handled | Guard: x64 case RVA `0x8312c` | [adapter.rs:3773](../../crates/crysirror-cmvs/src/adapter.rs#L3773) |
| `0x2119` | Execute advance branch | Handled | Guard: x64 case RVA `0x83139` | [adapter.rs:2369](../../crates/crysirror-cmvs/src/adapter.rs#L2369) |
| `0x211A` | Execute advance branch | Handled | Guard: x64 case RVA `0x83146`, present in the supplied script corpus | [adapter.rs:3773](../../crates/crysirror-cmvs/src/adapter.rs#L3773) |
| `0x211B` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x211C` | Execute advance branch | Handled | Guard: x64 case RVA `0x83153` | [adapter.rs:3773](../../crates/crysirror-cmvs/src/adapter.rs#L3773) |
| `0x211D` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x211E` | Execute advance branch | Handled | Guard: x64 case RVA `0x83160`, present in the supplied script corpus | [adapter.rs:2369](../../crates/crysirror-cmvs/src/adapter.rs#L2369) |
| `0x211F` | Execute advance branch | Handled | Guard: x64 case RVA `0x8316d`, present in the supplied script corpus | [adapter.rs:2369](../../crates/crysirror-cmvs/src/adapter.rs#L2369) |
| `0x2120` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x8317a` | [cmvs-20160201.json:24](../../tools/native-api/cmvs-20160201.json#L24) |
| `0x2121` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x83187` | [cmvs-20160201.json:24](../../tools/native-api/cmvs-20160201.json#L24) |
| `0x2122` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x83194` | [cmvs-20160201.json:24](../../tools/native-api/cmvs-20160201.json#L24) |
| `0x2123` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x831a9` | [cmvs-20160201.json:24](../../tools/native-api/cmvs-20160201.json#L24) |
| `0x2124` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2125` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2126` | Execute command branch | Handled | Guard: x64 case RVA `0x831c1`, present in the supplied script corpus | [screen.rs:228](../../crates/crysirror-cmvs/src/vm/screen.rs#L228) |
| `0x2127` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2128` | Update job | Handled | Guard: x64 case RVA `0x831ce`, present in the supplied script corpus | [adapter.rs:1935](../../crates/crysirror-cmvs/src/adapter.rs#L1935) |
| `0x2129` | Update job | Handled | Guard: x64 case RVA `0x831e1`, present in the supplied script corpus | [adapter.rs:3053](../../crates/crysirror-cmvs/src/adapter.rs#L3053) |
| `0x212A` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x831f4` | [cmvs-20160201.json:24](../../tools/native-api/cmvs-20160201.json#L24) |
| `0x212B` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x83209` | [cmvs-20160201.json:24](../../tools/native-api/cmvs-20160201.json#L24) |
| `0x212C` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x8321e` | [cmvs-20160201.json:24](../../tools/native-api/cmvs-20160201.json#L24) |
| `0x212D` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x212E` | Execute startup command branch | Handled | Guard: x64 case RVA `0x83233` | [startup.rs:1616](../../crates/crysirror-cmvs/src/vm/startup.rs#L1616) |
| `0x212F` | Execute startup command branch | Handled | Guard: x64 case RVA `0x83240`, present in the supplied script corpus | [startup.rs:1616](../../crates/crysirror-cmvs/src/vm/startup.rs#L1616) |
| `0x2130` | Execute startup command branch | Handled | Guard: x64 case RVA `0x83257`, present in the supplied script corpus | [startup.rs:1616](../../crates/crysirror-cmvs/src/vm/startup.rs#L1616) |
| `0x2131` | Execute startup command branch | Handled | Guard: x64 case RVA `0x83264`, present in the supplied script corpus | [startup.rs:1616](../../crates/crysirror-cmvs/src/vm/startup.rs#L1616) |
| `0x2132` | Execute startup command branch | Handled | Guard: x64 case RVA `0x83271`, present in the supplied script corpus | [startup.rs:1616](../../crates/crysirror-cmvs/src/vm/startup.rs#L1616) |
| `0x2133` | Execute startup command branch | Handled | Guard: x64 case RVA `0x8327e`, present in the supplied script corpus | [startup.rs:1616](../../crates/crysirror-cmvs/src/vm/startup.rs#L1616) |
| `0x2134` | Execute startup command branch | Handled | Guard: x64 case RVA `0x8328b` | [startup.rs:1616](../../crates/crysirror-cmvs/src/vm/startup.rs#L1616) |
| `0x2135` | Execute startup command branch | Handled | Guard: x64 case RVA `0x83298`, present in the supplied script corpus | [startup.rs:1616](../../crates/crysirror-cmvs/src/vm/startup.rs#L1616) |
| `0x2136` | Execute startup command branch | Handled | Guard: x64 case RVA `0x832a5`, present in the supplied script corpus | [startup.rs:1616](../../crates/crysirror-cmvs/src/vm/startup.rs#L1616) |
| `0x2137` | Execute startup command branch | Handled | Guard: x64 case RVA `0x832b2` | [startup.rs:1616](../../crates/crysirror-cmvs/src/vm/startup.rs#L1616) |
| `0x2138` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2139` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x213A` | Execute advance branch | Handled | Guard: x64 case RVA `0x832bf` | [adapter.rs:3705](../../crates/crysirror-cmvs/src/adapter.rs#L3705) |
| `0x213B` | Execute advance branch | Handled | Guard: x64 case RVA `0x832cc` | [adapter.rs:3705](../../crates/crysirror-cmvs/src/adapter.rs#L3705) |
| `0x213C` | Execute advance branch | Handled | Guard: x64 case RVA `0x832d9` | [adapter.rs:3705](../../crates/crysirror-cmvs/src/adapter.rs#L3705) |
| `0x213D` | Execute advance branch | Handled | Guard: x64 case RVA `0x832e6`, present in the supplied script corpus | [adapter.rs:3705](../../crates/crysirror-cmvs/src/adapter.rs#L3705) |
| `0x213E` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x213F` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2140` | Execute advance branch | Handled | Guard: x64 case RVA `0x832f9`, present in the supplied script corpus | [adapter.rs:2369](../../crates/crysirror-cmvs/src/adapter.rs#L2369) |
| `0x2141` | Execute advance branch | Handled | Guard: x64 case RVA `0x83306`, present in the supplied script corpus | [adapter.rs:2369](../../crates/crysirror-cmvs/src/adapter.rs#L2369) |
| `0x2142` | Execute advance branch | Handled | Guard: x64 case RVA `0x83313`, present in the supplied script corpus | [adapter.rs:2369](../../crates/crysirror-cmvs/src/adapter.rs#L2369) |
| `0x2143` | Execute advance branch | Handled | Guard: x64 case RVA `0x83320`, present in the supplied script corpus | [adapter.rs:2369](../../crates/crysirror-cmvs/src/adapter.rs#L2369) |
| `0x2144` | Execute advance branch | Handled | Guard: x64 case RVA `0x8332d`, present in the supplied script corpus | [adapter.rs:2369](../../crates/crysirror-cmvs/src/adapter.rs#L2369) |
| `0x2145` | Execute advance branch | Handled | Guard: x64 case RVA `0x8333a`, present in the supplied script corpus | [adapter.rs:2369](../../crates/crysirror-cmvs/src/adapter.rs#L2369) |
| `0x2146` | Execute advance branch | Handled | Guard: x64 case RVA `0x83347`, present in the supplied script corpus | [adapter.rs:2369](../../crates/crysirror-cmvs/src/adapter.rs#L2369) |
| `0x2147` | Execute advance branch | Handled | Guard: x64 case RVA `0x83354`, present in the supplied script corpus | [adapter.rs:2369](../../crates/crysirror-cmvs/src/adapter.rs#L2369) |
| `0x2148` | Execute advance branch | Handled | Guard: x64 case RVA `0x83361`, present in the supplied script corpus | [adapter.rs:2369](../../crates/crysirror-cmvs/src/adapter.rs#L2369) |
| `0x2149` | Execute advance branch | Handled | Guard: x64 case RVA `0x8336e`, present in the supplied script corpus | [adapter.rs:2369](../../crates/crysirror-cmvs/src/adapter.rs#L2369) |
| `0x214A` | Execute advance branch | Handled | Guard: x64 case RVA `0x8337b`, present in the supplied script corpus | [adapter.rs:2369](../../crates/crysirror-cmvs/src/adapter.rs#L2369) |
| `0x214B` | Pc | Handled | Guard: x64 case RVA `0x83388`, present in the supplied script corpus | [startup.rs:1645](../../crates/crysirror-cmvs/src/vm/startup.rs#L1645) |
| `0x214C` | Execute advance branch | Handled | Guard: x64 case RVA `0x83395`, present in the supplied script corpus | [adapter.rs:2369](../../crates/crysirror-cmvs/src/adapter.rs#L2369) |
| `0x214D` | Execute advance branch | Handled | Guard: x64 case RVA `0x833a2`, present in the supplied script corpus | [adapter.rs:2369](../../crates/crysirror-cmvs/src/adapter.rs#L2369) |
| `0x214E` | Execute advance branch | Handled | Guard: x64 case RVA `0x833af`, present in the supplied script corpus | [adapter.rs:2369](../../crates/crysirror-cmvs/src/adapter.rs#L2369) |
| `0x214F` | Execute advance branch | Handled | Guard: x64 case RVA `0x833bc`, present in the supplied script corpus | [adapter.rs:2369](../../crates/crysirror-cmvs/src/adapter.rs#L2369) |
| `0x2150` | Execute advance branch | Open | Message move modes with an unsupported branch remain open; Guard: x64 case RVA `0x833c9`, present in the supplied script corpus | [adapter.rs:2369](../../crates/crysirror-cmvs/src/adapter.rs#L2369) |
| `0x2151` | Execute advance branch | Handled | Guard: x64 case RVA `0x833d6`, present in the supplied script corpus | [adapter.rs:2369](../../crates/crysirror-cmvs/src/adapter.rs#L2369) |
| `0x2152` | Execute advance branch | Open | Gradient and exact native font fidelity remain open; Guard: x64 case RVA `0x833e3`, present in the supplied script corpus | [adapter.rs:2399](../../crates/crysirror-cmvs/src/adapter.rs#L2399) |
| `0x2153` | Execute advance branch | Handled | Guard: x64 case RVA `0x833f0`, present in the supplied script corpus | [adapter.rs:2399](../../crates/crysirror-cmvs/src/adapter.rs#L2399) |
| `0x2154` | Execute advance branch | Handled | Guard: x64 case RVA `0x833fd`, present in the supplied script corpus | [adapter.rs:2619](../../crates/crysirror-cmvs/src/adapter.rs#L2619) |
| `0x2155` | Execute advance branch | Handled | Guard: x64 case RVA `0x8340a`, present in the supplied script corpus | [adapter.rs:2369](../../crates/crysirror-cmvs/src/adapter.rs#L2369) |
| `0x2156` | Execute advance branch | Handled | Guard: x64 case RVA `0x83417` | [adapter.rs:2619](../../crates/crysirror-cmvs/src/adapter.rs#L2619) |
| `0x2157` | Execute advance branch | Open | Gradient and exact native font fidelity remain open; Guard: x64 case RVA `0x83424`, present in the supplied script corpus | [adapter.rs:2399](../../crates/crysirror-cmvs/src/adapter.rs#L2399) |
| `0x2158` | Execute advance branch | Handled | Guard: x64 case RVA `0x83431`, present in the supplied script corpus | [adapter.rs:2399](../../crates/crysirror-cmvs/src/adapter.rs#L2399) |
| `0x2159` | Execute advance branch | Handled | Guard: x64 case RVA `0x8343e`, present in the supplied script corpus | [adapter.rs:2619](../../crates/crysirror-cmvs/src/adapter.rs#L2619) |
| `0x215A` | Execute advance branch | Handled | Guard: x64 case RVA `0x8344b`, present in the supplied script corpus | [adapter.rs:2369](../../crates/crysirror-cmvs/src/adapter.rs#L2369) |
| `0x215B` | Execute advance branch | Handled | Guard: x64 case RVA `0x83458`, present in the supplied script corpus | [adapter.rs:2369](../../crates/crysirror-cmvs/src/adapter.rs#L2369) |
| `0x215C` | Execute advance branch | Handled | Guard: x64 case RVA `0x83465` | [adapter.rs:2399](../../crates/crysirror-cmvs/src/adapter.rs#L2399) |
| `0x215D` | Execute advance branch | Handled | Guard: x64 case RVA `0x83472`, present in the supplied script corpus | [adapter.rs:2369](../../crates/crysirror-cmvs/src/adapter.rs#L2369) |
| `0x215E` | Execute advance branch | Handled | Guard: x64 case RVA `0x8347f` | [adapter.rs:2619](../../crates/crysirror-cmvs/src/adapter.rs#L2619) |
| `0x215F` | Execute advance branch | Handled | Guard: x64 case RVA `0x8348c`, present in the supplied script corpus | [adapter.rs:2369](../../crates/crysirror-cmvs/src/adapter.rs#L2369) |
| `0x2160` | Execute advance branch | Handled | Guard: x64 case RVA `0x83499`, present in the supplied script corpus | [adapter.rs:1947](../../crates/crysirror-cmvs/src/adapter.rs#L1947) |
| `0x2161` | Execute advance branch | Handled | Guard: x64 case RVA `0x834a6`, present in the supplied script corpus | [adapter.rs:1947](../../crates/crysirror-cmvs/src/adapter.rs#L1947) |
| `0x2162` | Pc | Handled | Guard: x64 case RVA `0x834b3`, present in the supplied script corpus | [startup.rs:1634](../../crates/crysirror-cmvs/src/vm/startup.rs#L1634) |
| `0x2163` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x834c0`, present in the supplied script corpus | [cmvs-20160201.json:28](../../tools/native-api/cmvs-20160201.json#L28) |
| `0x2164` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x834cd`, present in the supplied script corpus | [cmvs-20160201.json:28](../../tools/native-api/cmvs-20160201.json#L28) |
| `0x2165` | Execute advance branch | Handled | Guard: x64 case RVA `0x834da` | [adapter.rs:2576](../../crates/crysirror-cmvs/src/adapter.rs#L2576) |
| `0x2166` | Execute advance branch | Handled | Guard: x64 case RVA `0x834e7` | [adapter.rs:2576](../../crates/crysirror-cmvs/src/adapter.rs#L2576) |
| `0x2167` | Execute advance branch | Handled | Guard: x64 case RVA `0x8350a` | [adapter.rs:2576](../../crates/crysirror-cmvs/src/adapter.rs#L2576) |
| `0x2168` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x83517` | [cmvs-20160201.json:28](../../tools/native-api/cmvs-20160201.json#L28) |
| `0x2169` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x83524` | [cmvs-20160201.json:28](../../tools/native-api/cmvs-20160201.json#L28) |
| `0x216A` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x216B` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x83531`, present in the supplied script corpus | [cmvs-20160201.json:28](../../tools/native-api/cmvs-20160201.json#L28) |
| `0x216C` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x8353e`, present in the supplied script corpus | [cmvs-20160201.json:28](../../tools/native-api/cmvs-20160201.json#L28) |
| `0x216D` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x8354b`, present in the supplied script corpus | [cmvs-20160201.json:28](../../tools/native-api/cmvs-20160201.json#L28) |
| `0x216E` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x83558`, present in the supplied script corpus | [cmvs-20160201.json:28](../../tools/native-api/cmvs-20160201.json#L28) |
| `0x216F` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x83578`, present in the supplied script corpus | [cmvs-20160201.json:28](../../tools/native-api/cmvs-20160201.json#L28) |
| `0x2170` | Update job | Handled | Guard: x64 case RVA `0x83585`, present in the supplied script corpus | [adapter.rs:3793](../../crates/crysirror-cmvs/src/adapter.rs#L3793) |
| `0x2171` | Execute advance branch | Handled | Guard: x64 case RVA `0x83592`, present in the supplied script corpus | [adapter.rs:2369](../../crates/crysirror-cmvs/src/adapter.rs#L2369) |
| `0x2172` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2173` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2174` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2175` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2176` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x8359f` | [cmvs-20160201.json:29](../../tools/native-api/cmvs-20160201.json#L29) |
| `0x2177` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x835ac` | [cmvs-20160201.json:29](../../tools/native-api/cmvs-20160201.json#L29) |
| `0x2178` | Execute advance branch | Handled | Guard: x64 case RVA `0x835b9`, present in the supplied script corpus | [adapter.rs:2369](../../crates/crysirror-cmvs/src/adapter.rs#L2369) |
| `0x2179` | Execute advance branch | Handled | Guard: x64 case RVA `0x835c6`, present in the supplied script corpus | [adapter.rs:2369](../../crates/crysirror-cmvs/src/adapter.rs#L2369) |
| `0x217A` | Execute advance branch | Handled | Guard: x64 case RVA `0x835d3`, present in the supplied script corpus | [adapter.rs:2369](../../crates/crysirror-cmvs/src/adapter.rs#L2369) |
| `0x217B` | Execute advance branch | Handled | Guard: x64 case RVA `0x835e0`, present in the supplied script corpus | [adapter.rs:2369](../../crates/crysirror-cmvs/src/adapter.rs#L2369) |
| `0x217C` | Execute advance branch | Handled | Guard: x64 case RVA `0x835ed`, present in the supplied script corpus | [adapter.rs:2369](../../crates/crysirror-cmvs/src/adapter.rs#L2369) |
| `0x217D` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x217E` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x835fa` | [cmvs-20160201.json:29](../../tools/native-api/cmvs-20160201.json#L29) |
| `0x217F` | Execute advance branch | Handled | Guard: x64 case RVA `0x83607`, present in the supplied script corpus | [adapter.rs:2369](../../crates/crysirror-cmvs/src/adapter.rs#L2369) |
| `0x2180` | Execute advance branch | Handled | Guard: x64 case RVA `0x83614`, present in the supplied script corpus | [adapter.rs:2369](../../crates/crysirror-cmvs/src/adapter.rs#L2369) |
| `0x2181` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x83621` | [cmvs-20160201.json:30](../../tools/native-api/cmvs-20160201.json#L30) |
| `0x2182` | Execute advance branch | Handled | Guard: x64 case RVA `0x8362e`, present in the supplied script corpus | [adapter.rs:2369](../../crates/crysirror-cmvs/src/adapter.rs#L2369) |
| `0x2183` | Execute advance branch | Handled | Guard: x64 case RVA `0x8363b`, present in the supplied script corpus | [adapter.rs:2369](../../crates/crysirror-cmvs/src/adapter.rs#L2369) |
| `0x2184` | Execute advance branch | Handled | Guard: x64 case RVA `0x83648`, present in the supplied script corpus | [adapter.rs:2369](../../crates/crysirror-cmvs/src/adapter.rs#L2369) |
| `0x2185` | Execute advance branch | Handled | Guard: x64 case RVA `0x83655`, present in the supplied script corpus | [adapter.rs:2369](../../crates/crysirror-cmvs/src/adapter.rs#L2369) |
| `0x2186` | Execute advance branch | Handled | Guard: x64 case RVA `0x83662`, present in the supplied script corpus | [adapter.rs:2369](../../crates/crysirror-cmvs/src/adapter.rs#L2369) |
| `0x2187` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x8366f` | [cmvs-20160201.json:30](../../tools/native-api/cmvs-20160201.json#L30) |
| `0x2188` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2189` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x8367c` | [cmvs-20160201.json:30](../../tools/native-api/cmvs-20160201.json#L30) |
| `0x218A` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x218B` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x83689` | [cmvs-20160201.json:30](../../tools/native-api/cmvs-20160201.json#L30) |
| `0x218C` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x218D` | Execute advance branch | Handled | Guard: x64 case RVA `0x83696`, present in the supplied script corpus | [adapter.rs:2369](../../crates/crysirror-cmvs/src/adapter.rs#L2369) |
| `0x218E` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x218F` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2190` | Execute advance branch | Handled | Guard: x64 case RVA `0x836a3`, present in the supplied script corpus | [adapter.rs:2369](../../crates/crysirror-cmvs/src/adapter.rs#L2369) |
| `0x2191` | Execute advance branch | Handled | Guard: x64 case RVA `0x836b0`, present in the supplied script corpus | [adapter.rs:2369](../../crates/crysirror-cmvs/src/adapter.rs#L2369) |
| `0x2192` | Execute advance branch | Handled | Guard: x64 case RVA `0x836bd`, present in the supplied script corpus | [adapter.rs:2369](../../crates/crysirror-cmvs/src/adapter.rs#L2369) |
| `0x2193` | Execute advance branch | Handled | Guard: x64 case RVA `0x836ca` | [adapter.rs:2369](../../crates/crysirror-cmvs/src/adapter.rs#L2369) |
| `0x2194` | Execute advance branch | Handled | Guard: x64 case RVA `0x836d7`, present in the supplied script corpus | [adapter.rs:2369](../../crates/crysirror-cmvs/src/adapter.rs#L2369) |
| `0x2195` | Execute advance branch | Handled | Guard: x64 case RVA `0x836e4`, present in the supplied script corpus | [adapter.rs:2369](../../crates/crysirror-cmvs/src/adapter.rs#L2369) |
| `0x2196` | Execute advance branch | Handled | Guard: x64 case RVA `0x836f1`, present in the supplied script corpus | [adapter.rs:2369](../../crates/crysirror-cmvs/src/adapter.rs#L2369) |
| `0x2197` | Execute advance branch | Handled | Guard: x64 case RVA `0x836fe` | [adapter.rs:2369](../../crates/crysirror-cmvs/src/adapter.rs#L2369) |
| `0x2198` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2199` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x219A` | Input (command) | Handled | Guard: x64 case RVA `0x8370b`, present in the supplied script corpus | [input.rs:220](../../crates/crysirror-cmvs/src/vm/input.rs#L220) |
| `0x219B` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x219C` | Read or consume an input edge | Handled | Guard: x64 case RVA `0x83722`, present in the supplied script corpus | [input.rs:9](../../crates/crysirror-cmvs/src/vm/input.rs#L9) |
| `0x219D` | Execute advance branch | Handled | Guard: x64 case RVA `0x8372f`, present in the supplied script corpus | [adapter.rs:2543](../../crates/crysirror-cmvs/src/adapter.rs#L2543) |
| `0x219E` | Read or consume an input edge | Handled | Guard: x64 case RVA `0x83746`, present in the supplied script corpus | [input.rs:10](../../crates/crysirror-cmvs/src/vm/input.rs#L10) |
| `0x219F` | Execute advance branch | Handled | Guard: x64 case RVA `0x83753`, present in the supplied script corpus | [adapter.rs:2543](../../crates/crysirror-cmvs/src/adapter.rs#L2543) |
| `0x21A0` | Read or consume an input edge | Handled | Guard: x64 case RVA `0x8376a`, present in the supplied script corpus | [input.rs:11](../../crates/crysirror-cmvs/src/vm/input.rs#L11) |
| `0x21A1` | Execute advance branch | Handled | Guard: x64 case RVA `0x83777`, present in the supplied script corpus | [adapter.rs:2543](../../crates/crysirror-cmvs/src/adapter.rs#L2543) |
| `0x21A2` | Read or consume an input edge | Handled | Guard: x64 case RVA `0x8378e`, present in the supplied script corpus | [input.rs:12](../../crates/crysirror-cmvs/src/vm/input.rs#L12) |
| `0x21A3` | Execute advance branch | Handled | Guard: x64 case RVA `0x8379b`, present in the supplied script corpus | [adapter.rs:2543](../../crates/crysirror-cmvs/src/adapter.rs#L2543) |
| `0x21A4` | Read or consume an input edge | Handled | Guard: x64 case RVA `0x837b2`, present in the supplied script corpus | [input.rs:13](../../crates/crysirror-cmvs/src/vm/input.rs#L13) |
| `0x21A5` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x21A6` | Read or consume an input edge | Handled | Guard: x64 case RVA `0x837bf`, present in the supplied script corpus | [input.rs:14](../../crates/crysirror-cmvs/src/vm/input.rs#L14) |
| `0x21A7` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x21A8` | Read or consume an input edge | Handled | Guard: x64 case RVA `0x837cc`, present in the supplied script corpus | [input.rs:15](../../crates/crysirror-cmvs/src/vm/input.rs#L15) |
| `0x21A9` | Execute advance branch | Handled | Guard: x64 case RVA `0x837d9`, present in the supplied script corpus | [adapter.rs:2543](../../crates/crysirror-cmvs/src/adapter.rs#L2543) |
| `0x21AA` | Read or consume an input edge | Handled | Guard: x64 case RVA `0x837f0`, present in the supplied script corpus | [input.rs:16](../../crates/crysirror-cmvs/src/vm/input.rs#L16) |
| `0x21AB` | Execute advance branch | Handled | Guard: x64 case RVA `0x837fd`, present in the supplied script corpus | [adapter.rs:2543](../../crates/crysirror-cmvs/src/adapter.rs#L2543) |
| `0x21AC` | Read or consume an input edge | Handled | Guard: x64 case RVA `0x83814`, present in the supplied script corpus | [input.rs:17](../../crates/crysirror-cmvs/src/vm/input.rs#L17) |
| `0x21AD` | Execute advance branch | Handled | Guard: x64 case RVA `0x83821`, present in the supplied script corpus | [adapter.rs:2543](../../crates/crysirror-cmvs/src/adapter.rs#L2543) |
| `0x21AE` | Read or consume an input edge | Handled | Guard: x64 case RVA `0x83838`, present in the supplied script corpus | [input.rs:18](../../crates/crysirror-cmvs/src/vm/input.rs#L18) |
| `0x21AF` | Execute advance branch | Handled | Guard: x64 case RVA `0x83845`, present in the supplied script corpus | [adapter.rs:2543](../../crates/crysirror-cmvs/src/adapter.rs#L2543) |
| `0x21B0` | Read or consume an input edge | Handled | Guard: x64 case RVA `0x8385c` | [input.rs:19](../../crates/crysirror-cmvs/src/vm/input.rs#L19) |
| `0x21B1` | Execute advance branch | Handled | Guard: x64 case RVA `0x83869` | [adapter.rs:2543](../../crates/crysirror-cmvs/src/adapter.rs#L2543) |
| `0x21B2` | Read or consume an input edge | Handled | Guard: x64 case RVA `0x83880` | [input.rs:20](../../crates/crysirror-cmvs/src/vm/input.rs#L20) |
| `0x21B3` | Execute advance branch | Handled | Guard: x64 case RVA `0x8388d` | [adapter.rs:2543](../../crates/crysirror-cmvs/src/adapter.rs#L2543) |
| `0x21B4` | Read or consume an input edge | Handled | Guard: x64 case RVA `0x838a4` | [input.rs:21](../../crates/crysirror-cmvs/src/vm/input.rs#L21) |
| `0x21B5` | Execute advance branch | Handled | Guard: x64 case RVA `0x838b1` | [adapter.rs:2543](../../crates/crysirror-cmvs/src/adapter.rs#L2543) |
| `0x21B6` | Read or consume an input edge | Handled | Guard: x64 case RVA `0x838c8` | [input.rs:22](../../crates/crysirror-cmvs/src/vm/input.rs#L22) |
| `0x21B7` | Execute advance branch | Handled | Guard: x64 case RVA `0x838d5` | [adapter.rs:2543](../../crates/crysirror-cmvs/src/adapter.rs#L2543) |
| `0x21B8` | Read or consume an input edge | Handled | Guard: x64 case RVA `0x838ec` | [input.rs:23](../../crates/crysirror-cmvs/src/vm/input.rs#L23) |
| `0x21B9` | Execute advance branch | Handled | Guard: x64 case RVA `0x838f9` | [adapter.rs:2543](../../crates/crysirror-cmvs/src/adapter.rs#L2543) |
| `0x21BA` | Read or consume an input edge | Handled | Guard: x64 case RVA `0x83910` | [input.rs:24](../../crates/crysirror-cmvs/src/vm/input.rs#L24) |
| `0x21BB` | Execute advance branch | Handled | Guard: x64 case RVA `0x8391d` | [adapter.rs:2543](../../crates/crysirror-cmvs/src/adapter.rs#L2543) |
| `0x21BC` | Read or consume an input edge | Handled | Guard: x64 case RVA `0x83934` | [input.rs:25](../../crates/crysirror-cmvs/src/vm/input.rs#L25) |
| `0x21BD` | Execute advance branch | Handled | Guard: x64 case RVA `0x83941` | [adapter.rs:2543](../../crates/crysirror-cmvs/src/adapter.rs#L2543) |
| `0x21BE` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x21BF` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x21C0` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x83958` | [cmvs-20160201.json:34](../../tools/native-api/cmvs-20160201.json#L34) |
| `0x21C1` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x83965` | [cmvs-20160201.json:34](../../tools/native-api/cmvs-20160201.json#L34) |
| `0x21C2` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x83972` | [cmvs-20160201.json:34](../../tools/native-api/cmvs-20160201.json#L34) |
| `0x21C3` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x8397f` | [cmvs-20160201.json:34](../../tools/native-api/cmvs-20160201.json#L34) |
| `0x21C4` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x21C5` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x8398c` | [cmvs-20160201.json:34](../../tools/native-api/cmvs-20160201.json#L34) |
| `0x21C6` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x83999` | [cmvs-20160201.json:34](../../tools/native-api/cmvs-20160201.json#L34) |
| `0x21C7` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x839a6` | [cmvs-20160201.json:34](../../tools/native-api/cmvs-20160201.json#L34) |
| `0x21C8` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x839b3` | [cmvs-20160201.json:34](../../tools/native-api/cmvs-20160201.json#L34) |
| `0x21C9` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x21CA` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x839c0` | [cmvs-20160201.json:34](../../tools/native-api/cmvs-20160201.json#L34) |
| `0x21CB` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x839cd` | [cmvs-20160201.json:34](../../tools/native-api/cmvs-20160201.json#L34) |
| `0x21CC` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x839da` | [cmvs-20160201.json:34](../../tools/native-api/cmvs-20160201.json#L34) |
| `0x21CD` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x839e7` | [cmvs-20160201.json:34](../../tools/native-api/cmvs-20160201.json#L34) |
| `0x21CE` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x21CF` | Execute advance branch | Handled | Guard: x64 case RVA `0x839f4`, present in the supplied script corpus | [adapter.rs:2369](../../crates/crysirror-cmvs/src/adapter.rs#L2369) |
| `0x21D0` | Execute advance branch | Handled | Guard: x64 case RVA `0x83a01`, present in the supplied script corpus | [adapter.rs:2369](../../crates/crysirror-cmvs/src/adapter.rs#L2369) |
| `0x21D1` | Execute advance branch | Handled | Guard: x64 case RVA `0x83a0e`, present in the supplied script corpus | [adapter.rs:2369](../../crates/crysirror-cmvs/src/adapter.rs#L2369) |
| `0x21D2` | Execute advance branch | Handled | Guard: x64 case RVA `0x83a1b` | [adapter.rs:2369](../../crates/crysirror-cmvs/src/adapter.rs#L2369) |
| `0x21D3` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x21D4` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x83a28` | [cmvs-20160201.json:35](../../tools/native-api/cmvs-20160201.json#L35) |
| `0x21D5` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x83a35` | [cmvs-20160201.json:35](../../tools/native-api/cmvs-20160201.json#L35) |
| `0x21D6` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x83a42` | [cmvs-20160201.json:35](../../tools/native-api/cmvs-20160201.json#L35) |
| `0x21D7` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x83a4f` | [cmvs-20160201.json:35](../../tools/native-api/cmvs-20160201.json#L35) |
| `0x21D8` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x21D9` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x21DA` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x21DB` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x21DC` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x21DD` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x21DE` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x83a5c` | [cmvs-20160201.json:35](../../tools/native-api/cmvs-20160201.json#L35) |
| `0x21DF` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x83a69` | [cmvs-20160201.json:35](../../tools/native-api/cmvs-20160201.json#L35) |
| `0x21E0` | Execute advance branch | Handled | Guard: x64 case RVA `0x83a80`, present in the supplied script corpus | [adapter.rs:3833](../../crates/crysirror-cmvs/src/adapter.rs#L3833) |
| `0x21E1` | Execute advance branch | Handled | Guard: x64 case RVA `0x83a8d`, present in the supplied script corpus | [adapter.rs:3833](../../crates/crysirror-cmvs/src/adapter.rs#L3833) |
| `0x21E2` | Execute advance branch | Handled | Guard: x64 case RVA `0x83a9a`, present in the supplied script corpus | [adapter.rs:3833](../../crates/crysirror-cmvs/src/adapter.rs#L3833) |
| `0x21E3` | Execute advance branch | Handled | Guard: x64 case RVA `0x83aa7`, present in the supplied script corpus | [adapter.rs:3833](../../crates/crysirror-cmvs/src/adapter.rs#L3833) |
| `0x21E4` | Execute advance branch | Handled | Guard: x64 case RVA `0x83ab4`, present in the supplied script corpus | [adapter.rs:3833](../../crates/crysirror-cmvs/src/adapter.rs#L3833) |
| `0x21E5` | Execute advance branch | Handled | Guard: x64 case RVA `0x83ac1` | [adapter.rs:3833](../../crates/crysirror-cmvs/src/adapter.rs#L3833) |
| `0x21E6` | Execute advance branch | Handled | Guard: x64 case RVA `0x83ace` | [adapter.rs:3833](../../crates/crysirror-cmvs/src/adapter.rs#L3833) |
| `0x21E7` | Execute advance branch | Handled | Guard: x64 case RVA `0x83adb` | [adapter.rs:3833](../../crates/crysirror-cmvs/src/adapter.rs#L3833) |
| `0x21E8` | Execute advance branch | Handled | Guard: x64 case RVA `0x83ae8` | [adapter.rs:3833](../../crates/crysirror-cmvs/src/adapter.rs#L3833) |
| `0x21E9` | Execute advance branch | Handled | Guard: x64 case RVA `0x83af5` | [adapter.rs:3833](../../crates/crysirror-cmvs/src/adapter.rs#L3833) |
| `0x21EA` | Execute advance branch | Handled | Guard: x64 case RVA `0x83b02`, present in the supplied script corpus | [adapter.rs:3828](../../crates/crysirror-cmvs/src/adapter.rs#L3828) |
| `0x21EB` | Execute advance branch | Handled | Guard: x64 case RVA `0x83b0f`, present in the supplied script corpus | [adapter.rs:3828](../../crates/crysirror-cmvs/src/adapter.rs#L3828) |
| `0x21EC` | Execute advance branch | Handled | Guard: x64 case RVA `0x83b1c`, present in the supplied script corpus | [adapter.rs:3828](../../crates/crysirror-cmvs/src/adapter.rs#L3828) |
| `0x21ED` | Execute advance branch | Handled | Guard: x64 case RVA `0x83b29`, present in the supplied script corpus | [adapter.rs:3828](../../crates/crysirror-cmvs/src/adapter.rs#L3828) |
| `0x21EE` | Execute advance branch | Handled | Guard: x64 case RVA `0x83b36`, present in the supplied script corpus | [adapter.rs:3828](../../crates/crysirror-cmvs/src/adapter.rs#L3828) |
| `0x21EF` | Execute advance branch | Handled | Guard: x64 case RVA `0x83b43`, present in the supplied script corpus | [adapter.rs:3833](../../crates/crysirror-cmvs/src/adapter.rs#L3833) |
| `0x21F0` | Execute advance branch | Handled | Guard: x64 case RVA `0x83b50`, present in the supplied script corpus | [adapter.rs:3833](../../crates/crysirror-cmvs/src/adapter.rs#L3833) |
| `0x21F1` | Execute advance branch | Handled | Guard: x64 case RVA `0x83b5d`, present in the supplied script corpus | [adapter.rs:3833](../../crates/crysirror-cmvs/src/adapter.rs#L3833) |
| `0x21F2` | Execute advance branch | Handled | Guard: x64 case RVA `0x83b6a` | [adapter.rs:3833](../../crates/crysirror-cmvs/src/adapter.rs#L3833) |
| `0x21F3` | Execute advance branch | Handled | Guard: x64 case RVA `0x83b77` | [adapter.rs:3833](../../crates/crysirror-cmvs/src/adapter.rs#L3833) |
| `0x21F4` | Execute advance branch | Handled | Guard: x64 case RVA `0x83b84`, present in the supplied script corpus | [adapter.rs:3833](../../crates/crysirror-cmvs/src/adapter.rs#L3833) |
| `0x21F5` | Execute advance branch | Handled | Guard: x64 case RVA `0x83b91`, present in the supplied script corpus | [adapter.rs:3833](../../crates/crysirror-cmvs/src/adapter.rs#L3833) |
| `0x21F6` | Execute advance branch | Handled | Guard: x64 case RVA `0x83b9e`, present in the supplied script corpus | [adapter.rs:3833](../../crates/crysirror-cmvs/src/adapter.rs#L3833) |
| `0x21F7` | Execute advance branch | Handled | Guard: x64 case RVA `0x83bab` | [adapter.rs:3833](../../crates/crysirror-cmvs/src/adapter.rs#L3833) |
| `0x21F8` | Execute advance branch | Handled | Guard: x64 case RVA `0x83bb8` | [adapter.rs:3833](../../crates/crysirror-cmvs/src/adapter.rs#L3833) |
| `0x21F9` | Execute advance branch | Handled | Guard: x64 case RVA `0x83bc5`, present in the supplied script corpus | [adapter.rs:3833](../../crates/crysirror-cmvs/src/adapter.rs#L3833) |
| `0x21FA` | Execute advance branch | Handled | Guard: x64 case RVA `0x83bd2`, present in the supplied script corpus | [adapter.rs:3833](../../crates/crysirror-cmvs/src/adapter.rs#L3833) |
| `0x21FB` | Execute advance branch | Handled | Guard: x64 case RVA `0x83bdf`, present in the supplied script corpus | [adapter.rs:3833](../../crates/crysirror-cmvs/src/adapter.rs#L3833) |
| `0x21FC` | Execute advance branch | Handled | Guard: x64 case RVA `0x83bec`, present in the supplied script corpus | [adapter.rs:3833](../../crates/crysirror-cmvs/src/adapter.rs#L3833) |
| `0x21FD` | Execute advance branch | Handled | Guard: x64 case RVA `0x83bf9`, present in the supplied script corpus | [adapter.rs:3833](../../crates/crysirror-cmvs/src/adapter.rs#L3833) |
| `0x21FE` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x21FF` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2200` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2201` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2202` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2203` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2204` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2205` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2206` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2207` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2208` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2209` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x220A` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x220B` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x220C` | Execute advance branch | Handled | Guard: x64 case RVA `0x83c06`, present in the supplied script corpus | [adapter.rs:3843](../../crates/crysirror-cmvs/src/adapter.rs#L3843) |
| `0x220D` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x220E` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x220F` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2210` | Execute advance branch | Handled | Guard: x64 case RVA `0x83c13`, present in the supplied script corpus | [adapter.rs:2359](../../crates/crysirror-cmvs/src/adapter.rs#L2359) |
| `0x2211` | Execute advance branch | Handled | Guard: x64 case RVA `0x83c20`, present in the supplied script corpus | [adapter.rs:2359](../../crates/crysirror-cmvs/src/adapter.rs#L2359) |
| `0x2212` | Execute advance branch | Handled | Guard: x64 case RVA `0x83c2d`, present in the supplied script corpus | [adapter.rs:2359](../../crates/crysirror-cmvs/src/adapter.rs#L2359) |
| `0x2213` | Execute advance branch | Handled | Guard: x64 case RVA `0x83c3a`, present in the supplied script corpus | [adapter.rs:2359](../../crates/crysirror-cmvs/src/adapter.rs#L2359) |
| `0x2214` | Execute advance branch | Handled | Guard: x64 case RVA `0x83c47`, present in the supplied script corpus | [adapter.rs:2359](../../crates/crysirror-cmvs/src/adapter.rs#L2359) |
| `0x2215` | Execute advance branch | Handled | Guard: x64 case RVA `0x83c54`, present in the supplied script corpus | [adapter.rs:2359](../../crates/crysirror-cmvs/src/adapter.rs#L2359) |
| `0x2216` | Execute advance branch | Handled | Guard: x64 case RVA `0x83c61`, present in the supplied script corpus | [adapter.rs:2359](../../crates/crysirror-cmvs/src/adapter.rs#L2359) |
| `0x2217` | Execute advance branch | Handled | Guard: x64 case RVA `0x83c6e`, present in the supplied script corpus | [adapter.rs:2359](../../crates/crysirror-cmvs/src/adapter.rs#L2359) |
| `0x2218` | Execute advance branch | Handled | Guard: x64 case RVA `0x83c7b` | [adapter.rs:2359](../../crates/crysirror-cmvs/src/adapter.rs#L2359) |
| `0x2219` | Execute advance branch | Handled | Guard: x64 case RVA `0x83c88` | [adapter.rs:2359](../../crates/crysirror-cmvs/src/adapter.rs#L2359) |
| `0x221A` | Execute advance branch | Handled | Guard: x64 case RVA `0x83c95`, present in the supplied script corpus | [adapter.rs:2359](../../crates/crysirror-cmvs/src/adapter.rs#L2359) |
| `0x221B` | Execute advance branch | Handled | Guard: x64 case RVA `0x83ca2`, present in the supplied script corpus | [adapter.rs:2359](../../crates/crysirror-cmvs/src/adapter.rs#L2359) |
| `0x221C` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x221D` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x221E` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x221F` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2220` | Execute advance branch | Handled | Guard: x64 case RVA `0x83caf`, present in the supplied script corpus | [adapter.rs:1922](../../crates/crysirror-cmvs/src/adapter.rs#L1922) |
| `0x2221` | Execute advance branch | Handled | Guard: x64 case RVA `0x83cbc`, present in the supplied script corpus | [adapter.rs:1922](../../crates/crysirror-cmvs/src/adapter.rs#L1922) |
| `0x2222` | Execute advance branch | Handled | Guard: x64 case RVA `0x83cc9` | [adapter.rs:1922](../../crates/crysirror-cmvs/src/adapter.rs#L1922) |
| `0x2223` | Execute advance branch | Handled | Guard: x64 case RVA `0x83cd6` | [adapter.rs:1922](../../crates/crysirror-cmvs/src/adapter.rs#L1922) |
| `0x2224` | Execute advance branch | Handled | Guard: x64 case RVA `0x83ce3`, present in the supplied script corpus | [adapter.rs:1922](../../crates/crysirror-cmvs/src/adapter.rs#L1922) |
| `0x2225` | Execute advance branch | Open | Additional native quake waveform and random direction semantics remain open; Guard: x64 case RVA `0x83cf0`, present in the supplied script corpus | [adapter.rs:3843](../../crates/crysirror-cmvs/src/adapter.rs#L3843) |
| `0x2226` | Execute advance branch | Handled | Guard: x64 case RVA `0x83cfd`, present in the supplied script corpus | [adapter.rs:3843](../../crates/crysirror-cmvs/src/adapter.rs#L3843) |
| `0x2227` | Execute advance branch | Handled | Guard: x64 case RVA `0x83d0a`, present in the supplied script corpus | [adapter.rs:3843](../../crates/crysirror-cmvs/src/adapter.rs#L3843) |
| `0x2228` | Execute advance branch | Handled | Guard: x64 case RVA `0x83d17`, present in the supplied script corpus | [adapter.rs:3843](../../crates/crysirror-cmvs/src/adapter.rs#L3843) |
| `0x2229` | Execute advance branch | Handled | Guard: x64 case RVA `0x83d24`, present in the supplied script corpus | [adapter.rs:3843](../../crates/crysirror-cmvs/src/adapter.rs#L3843) |
| `0x222A` | Update job | Open | Shared shader-equivalent coverage; native CPU-device path is pending; Guard: x64 case RVA `0x83d31`, present in the supplied script corpus | [adapter.rs:3850](../../crates/crysirror-cmvs/src/adapter.rs#L3850) |
| `0x222B` | Execute advance branch | Handled | Guard: x64 case RVA `0x83d3e`, present in the supplied script corpus | [adapter.rs:3884](../../crates/crysirror-cmvs/src/adapter.rs#L3884) |
| `0x222C` | Execute advance branch | Handled | Guard: x64 case RVA `0x83d4b` | [adapter.rs:3909](../../crates/crysirror-cmvs/src/adapter.rs#L3909) |
| `0x222D` | Execute advance branch | Handled | Guard: x64 case RVA `0x83d58` | [adapter.rs:3916](../../crates/crysirror-cmvs/src/adapter.rs#L3916) |
| `0x222E` | Update job | Handled | Guard: x64 case RVA `0x83d65` | [adapter.rs:3452](../../crates/crysirror-cmvs/src/adapter.rs#L3452) |
| `0x222F` | Execute advance branch | Handled | Guard: x64 case RVA `0x83d72`, present in the supplied script corpus | [adapter.rs:2560](../../crates/crysirror-cmvs/src/adapter.rs#L2560) |
| `0x2230` | Execute advance branch | Handled | Guard: x64 case RVA `0x83d7f` | [adapter.rs:2580](../../crates/crysirror-cmvs/src/adapter.rs#L2580) |
| `0x2231` | Execute advance branch | Handled | Guard: x64 case RVA `0x83d8c`, present in the supplied script corpus | [adapter.rs:2580](../../crates/crysirror-cmvs/src/adapter.rs#L2580) |
| `0x2232` | Execute advance branch | Handled | Guard: x64 case RVA `0x83d99` | [adapter.rs:2570](../../crates/crysirror-cmvs/src/adapter.rs#L2570) |
| `0x2233` | Execute advance branch | Handled | Guard: x64 case RVA `0x83da6`, present in the supplied script corpus | [adapter.rs:2587](../../crates/crysirror-cmvs/src/adapter.rs#L2587) |
| `0x2234` | Update job | Open | Shared coverage; CPU-device path and old-target shader retention are pending; Guard: x64 case RVA `0x83db3`, present in the supplied script corpus | [adapter.rs:3850](../../crates/crysirror-cmvs/src/adapter.rs#L3850) |
| `0x2235` | Execute advance branch | Handled | Guard: x64 case RVA `0x83dc0`, present in the supplied script corpus | [adapter.rs:3884](../../crates/crysirror-cmvs/src/adapter.rs#L3884) |
| `0x2236` | Execute advance branch | Handled | Guard: x64 case RVA `0x83dcd` | [adapter.rs:3909](../../crates/crysirror-cmvs/src/adapter.rs#L3909) |
| `0x2237` | Execute advance branch | Handled | Guard: x64 case RVA `0x83dda` | [adapter.rs:3916](../../crates/crysirror-cmvs/src/adapter.rs#L3916) |
| `0x2238` | Execute advance branch | Handled | Guard: x64 case RVA `0x83de7`, present in the supplied script corpus | [adapter.rs:3227](../../crates/crysirror-cmvs/src/adapter.rs#L3227) |
| `0x2239` | Execute advance branch | Handled | Guard: x64 case RVA `0x83df4`, present in the supplied script corpus | [adapter.rs:3227](../../crates/crysirror-cmvs/src/adapter.rs#L3227) |
| `0x223A` | Execute advance branch | Handled | Guard: x64 case RVA `0x83e01`, present in the supplied script corpus | [adapter.rs:3227](../../crates/crysirror-cmvs/src/adapter.rs#L3227) |
| `0x223B` | Execute advance branch | Handled | Guard: x64 case RVA `0x83e0e`, present in the supplied script corpus | [adapter.rs:3227](../../crates/crysirror-cmvs/src/adapter.rs#L3227) |
| `0x223C` | Execute advance branch | Handled | Guard: x64 case RVA `0x83e1b`, present in the supplied script corpus | [adapter.rs:3227](../../crates/crysirror-cmvs/src/adapter.rs#L3227) |
| `0x223D` | Execute advance branch | Handled | Guard: x64 case RVA `0x83e28`, present in the supplied script corpus | [adapter.rs:3227](../../crates/crysirror-cmvs/src/adapter.rs#L3227) |
| `0x223E` | Execute advance branch | Handled | Guard: x64 case RVA `0x83e35` | [adapter.rs:3227](../../crates/crysirror-cmvs/src/adapter.rs#L3227) |
| `0x223F` | Execute advance branch | Handled | Guard: x64 case RVA `0x83e42` | [adapter.rs:3227](../../crates/crysirror-cmvs/src/adapter.rs#L3227) |
| `0x2240` | Update job | Open | Scoped playback is connected; native invalid-slot reporting occurs at the frame boundary; Guard: x64 case RVA `0x83e4f` | [adapter.rs:2160](../../crates/crysirror-cmvs/src/adapter.rs#L2160) |
| `0x2241` | Update job | Handled | Guard: x64 case RVA `0x83e5c`, present in the supplied script corpus | [adapter.rs:2160](../../crates/crysirror-cmvs/src/adapter.rs#L2160) |
| `0x2242` | Update job | Open | Shared elapsed transport time replaces the DirectSound device cursor; Guard: x64 case RVA `0x83e69` | [adapter.rs:2160](../../crates/crysirror-cmvs/src/adapter.rs#L2160) |
| `0x2243` | Execute advance branch | Open | Buffered source levels are connected; output resampling can differ from DirectSound; Guard: x64 case RVA `0x83e76` | [adapter.rs:2247](../../crates/crysirror-cmvs/src/adapter.rs#L2247) |
| `0x2244` | Update job | Open | Quantized multiplier is connected; the renderer uses its own fade clock; Guard: x64 case RVA `0x83e83` | [adapter.rs:2160](../../crates/crysirror-cmvs/src/adapter.rs#L2160) |
| `0x2245` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2246` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2247` | Execute advance branch | Handled | Guard: x64 case RVA `0x83e90` | [adapter.rs:2565](../../crates/crysirror-cmvs/src/adapter.rs#L2565) |
| `0x2248` | Update job | Open | Resident sample loading is connected; native codec compatibility needs validation; Guard: x64 case RVA `0x83e9d` | [adapter.rs:2265](../../crates/crysirror-cmvs/src/adapter.rs#L2265) |
| `0x2249` | Update job | Handled | Guard: x64 case RVA `0x83eaa` | [adapter.rs:2265](../../crates/crysirror-cmvs/src/adapter.rs#L2265) |
| `0x224A` | Update job | Open | Retains the cursor on stop/resume; DirectSound EOF cursor parity is open; Guard: x64 case RVA `0x83eb7` | [adapter.rs:2265](../../crates/crysirror-cmvs/src/adapter.rs#L2265) |
| `0x224B` | Update job | Handled | Guard: x64 case RVA `0x83ec4` | [adapter.rs:2265](../../crates/crysirror-cmvs/src/adapter.rs#L2265) |
| `0x224C` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x224D` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x224E` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x224F` | Update job | Open | Uses 32 duplicate positions; driver gain and voice-cap parity are open; Guard: x64 case RVA `0x83ed1` | [adapter.rs:2265](../../crates/crysirror-cmvs/src/adapter.rs#L2265) |
| `0x2250` | Update depth clear | Handled | Guard: x64 case RVA `0x83ede`, present in the supplied script corpus | [screen.rs:237](../../crates/crysirror-cmvs/src/vm/screen.rs#L237) |
| `0x2251` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2252` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2253` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2254` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2255` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2256` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2257` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2258` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2259` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x225A` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x225B` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x225C` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x225D` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x225E` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x225F` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2260` | Execute advance branch | Handled | Guard: x64 case RVA `0x83eeb`, present in the supplied script corpus | [adapter.rs:2584](../../crates/crysirror-cmvs/src/adapter.rs#L2584) |
| `0x2261` | Execute advance branch | Handled | Guard: x64 case RVA `0x83ef8`, present in the supplied script corpus | [adapter.rs:2584](../../crates/crysirror-cmvs/src/adapter.rs#L2584) |
| `0x2262` | Execute advance branch | Handled | Guard: x64 case RVA `0x83f05`, present in the supplied script corpus | [adapter.rs:2584](../../crates/crysirror-cmvs/src/adapter.rs#L2584) |
| `0x2263` | Execute advance branch | Handled | Guard: x64 case RVA `0x83f12`, present in the supplied script corpus | [adapter.rs:2584](../../crates/crysirror-cmvs/src/adapter.rs#L2584) |
| `0x2264` | Execute advance branch | Handled | Guard: x64 case RVA `0x83f1f`, present in the supplied script corpus | [adapter.rs:2584](../../crates/crysirror-cmvs/src/adapter.rs#L2584) |
| `0x2265` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2266` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2267` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2268` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2269` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x226A` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x226B` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x226C` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x226D` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x226E` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x226F` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2270` | Execute advance branch | Handled | Guard: x64 case RVA `0x83f2c` | [adapter.rs:2584](../../crates/crysirror-cmvs/src/adapter.rs#L2584) |
| `0x2271` | Execute advance branch | Handled | Guard: x64 case RVA `0x83f39` | [adapter.rs:2587](../../crates/crysirror-cmvs/src/adapter.rs#L2587) |
| `0x2272` | Execute advance branch | Handled | Guard: x64 case RVA `0x83f46` | [adapter.rs:2584](../../crates/crysirror-cmvs/src/adapter.rs#L2584) |
| `0x2273` | Execute advance branch | Handled | Guard: x64 case RVA `0x83f53` | [adapter.rs:2584](../../crates/crysirror-cmvs/src/adapter.rs#L2584) |
| `0x2274` | Execute advance branch | Handled | Guard: x64 case RVA `0x83f60` | [adapter.rs:2584](../../crates/crysirror-cmvs/src/adapter.rs#L2584) |
| `0x2275` | Execute advance branch | Handled | Guard: x64 case RVA `0x83f6d` | [adapter.rs:2584](../../crates/crysirror-cmvs/src/adapter.rs#L2584) |
| `0x2276` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2277` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2278` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2279` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x227A` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x227B` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x227C` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x227D` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x227E` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x227F` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2280` | Execute startup command branch | Handled | Guard: x64 case RVA `0x83f7a`, present in the supplied script corpus | [startup.rs:1670](../../crates/crysirror-cmvs/src/vm/startup.rs#L1670) |
| `0x2281` | Pc | Handled | Guard: x64 case RVA `0x83f91`, present in the supplied script corpus | [startup.rs:1674](../../crates/crysirror-cmvs/src/vm/startup.rs#L1674) |
| `0x2282` | Execute advance branch | Handled | Guard: x64 case RVA `0x83f9e`, present in the supplied script corpus | [adapter.rs:2873](../../crates/crysirror-cmvs/src/adapter.rs#L2873) |
| `0x2283` | Execute startup command branch | Handled | Guard: x64 case RVA `0x83fb5` | [startup.rs:1683](../../crates/crysirror-cmvs/src/vm/startup.rs#L1683) |
| `0x2284` | Update job | Handled | Guard: x64 case RVA `0x83fcc`, present in the supplied script corpus | [adapter.rs:2878](../../crates/crysirror-cmvs/src/adapter.rs#L2878) |
| `0x2285` | Pc | Handled | Guard: x64 case RVA `0x83fd9`, present in the supplied script corpus | [startup.rs:1674](../../crates/crysirror-cmvs/src/vm/startup.rs#L1674) |
| `0x2286` | Required parameter | Handled | Guard: x64 case RVA `0x83fe6`, present in the supplied script corpus | [startup.rs:1687](../../crates/crysirror-cmvs/src/vm/startup.rs#L1687) |
| `0x2287` | Required parameter | Handled | Guard: x64 case RVA `0x83ff3`, present in the supplied script corpus | [startup.rs:1687](../../crates/crysirror-cmvs/src/vm/startup.rs#L1687) |
| `0x2288` | Required parameter | Handled | Guard: x64 case RVA `0x84000`, present in the supplied script corpus | [startup.rs:1687](../../crates/crysirror-cmvs/src/vm/startup.rs#L1687) |
| `0x2289` | Required parameter | Handled | Guard: x64 case RVA `0x8400d`, present in the supplied script corpus | [startup.rs:1687](../../crates/crysirror-cmvs/src/vm/startup.rs#L1687) |
| `0x228A` | Required parameter | Handled | Guard: x64 case RVA `0x8401a`, present in the supplied script corpus | [startup.rs:1687](../../crates/crysirror-cmvs/src/vm/startup.rs#L1687) |
| `0x228B` | Required parameter | Handled | Guard: x64 case RVA `0x84027` | [startup.rs:1687](../../crates/crysirror-cmvs/src/vm/startup.rs#L1687) |
| `0x228C` | Update job | Handled | Guard: x64 case RVA `0x84034`, present in the supplied script corpus | [adapter.rs:2775](../../crates/crysirror-cmvs/src/adapter.rs#L2775) |
| `0x228D` | Update job | Handled | Guard: x64 case RVA `0x84041` | [adapter.rs:2775](../../crates/crysirror-cmvs/src/adapter.rs#L2775) |
| `0x228E` | Update job | Handled | Guard: x64 case RVA `0x8404e`, present in the supplied script corpus | [adapter.rs:2775](../../crates/crysirror-cmvs/src/adapter.rs#L2775) |
| `0x228F` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2290` | Create a textured ground and water scene | Handled | Guard: x64 case RVA `0x8405b` | [adapter.rs:3247](../../crates/crysirror-cmvs/src/adapter.rs#L3247) |
| `0x2291` | Create the walking camera and resident sound scene | Handled | Guard: x64 case RVA `0x84068` | [adapter.rs:3247](../../crates/crysirror-cmvs/src/adapter.rs#L3247) |
| `0x2292` | Create the rotating quad scene with staggered fades and resident cues | Handled | Guard: x64 case RVA `0x84075` | [adapter.rs:3247](../../crates/crysirror-cmvs/src/adapter.rs#L3247) |
| `0x2293` | Create projected panels with width-selected textures and rule-image reveals | Handled | Guard: x64 case RVA `0x84082` | [adapter.rs:3247](../../crates/crysirror-cmvs/src/adapter.rs#L3247) |
| `0x2294` | Create the rising planes, drifting billboards and resident cue | Handled | Guard: x64 case RVA `0x8408f` | [adapter.rs:3247](../../crates/crysirror-cmvs/src/adapter.rs#L3247) |
| `0x2295` | Create the three projected atlas actors and resident sound choreography | Handled | Guard: x64 case RVA `0x8409c` | [adapter.rs:3247](../../crates/crysirror-cmvs/src/adapter.rs#L3247) |
| `0x2296` | Create the fixed ground and seven rotating quads | Handled | Guard: x64 case RVA `0x840a9` | [adapter.rs:3247](../../crates/crysirror-cmvs/src/adapter.rs#L3247) |
| `0x2297` | Create the orbiting atlas scene and its triggerable overlays | Handled | Guard: x64 case RVA `0x840b6` | [adapter.rs:3247](../../crates/crysirror-cmvs/src/adapter.rs#L3247) |
| `0x2298` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2299` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x229A` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x229B` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x229C` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x840c3` | [cmvs-20160201.json:47](../../tools/native-api/cmvs-20160201.json#L47) |
| `0x229D` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x840ef` | [cmvs-20160201.json:47](../../tools/native-api/cmvs-20160201.json#L47) |
| `0x229E` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x229F` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x22A0` | Execute advance branch | Handled | Guard: x64 case RVA `0x8411b`, present in the supplied script corpus | [adapter.rs:3312](../../crates/crysirror-cmvs/src/adapter.rs#L3312) |
| `0x22A1` | Update job | Handled | Guard: x64 case RVA `0x84128` | [adapter.rs:3376](../../crates/crysirror-cmvs/src/adapter.rs#L3376) |
| `0x22A2` | Update job | Handled | Guard: x64 case RVA `0x84135`, present in the supplied script corpus | [adapter.rs:3376](../../crates/crysirror-cmvs/src/adapter.rs#L3376) |
| `0x22A3` | Update job | Handled | Guard: x64 case RVA `0x84142` | [adapter.rs:3376](../../crates/crysirror-cmvs/src/adapter.rs#L3376) |
| `0x22A4` | Update job | Handled | Guard: x64 case RVA `0x8414f` | [adapter.rs:3376](../../crates/crysirror-cmvs/src/adapter.rs#L3376) |
| `0x22A5` | Update job | Handled | Guard: x64 case RVA `0x8415c` | [adapter.rs:3376](../../crates/crysirror-cmvs/src/adapter.rs#L3376) |
| `0x22A6` | Update job | Handled | Guard: x64 case RVA `0x84169` | [adapter.rs:3376](../../crates/crysirror-cmvs/src/adapter.rs#L3376) |
| `0x22A7` | Update job | Handled | Guard: x64 case RVA `0x84176` | [adapter.rs:3376](../../crates/crysirror-cmvs/src/adapter.rs#L3376) |
| `0x22A8` | Execute advance branch | Handled | Guard: x64 case RVA `0x84183`, present in the supplied script corpus | [adapter.rs:3329](../../crates/crysirror-cmvs/src/adapter.rs#L3329) |
| `0x22A9` | Update job | Handled | Guard: x64 case RVA `0x84190` | [adapter.rs:3376](../../crates/crysirror-cmvs/src/adapter.rs#L3376) |
| `0x22AA` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x22AB` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x22AC` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x22AD` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x22AE` | Execute advance branch | Handled | Guard: x64 case RVA `0x8419d`, present in the supplied script corpus | [adapter.rs:3318](../../crates/crysirror-cmvs/src/adapter.rs#L3318) |
| `0x22AF` | Trigger a scene or change its native mode | Handled | Guard: x64 case RVA `0x841aa` | [adapter.rs:3347](../../crates/crysirror-cmvs/src/adapter.rs#L3347) |
| `0x22B0` | Required parameter | Handled | Guard: x64 case RVA `0x841b7`, present in the supplied script corpus | [startup.rs:1658](../../crates/crysirror-cmvs/src/vm/startup.rs#L1658) |
| `0x22B1` | Set the maximum number of history entries captured in a save | Handled | Guard: x64 case RVA `0x841c4` | [startup.rs:1665](../../crates/crysirror-cmvs/src/vm/startup.rs#L1665) |
| `0x22B2` | Execute advance branch | Open | In-memory logical capture; persistent gameplay save/restore is pending; Guard: x64 case RVA `0x841e7`, present in the supplied script corpus | [adapter.rs:1693](../../crates/crysirror-cmvs/src/adapter.rs#L1693) |
| `0x22B3` | Update job | Handled | Guard: x64 case RVA `0x841f4`, present in the supplied script corpus | [adapter.rs:1681](../../crates/crysirror-cmvs/src/adapter.rs#L1681) |
| `0x22B4` | Execute advance branch | Handled | Guard: x64 case RVA `0x84201`, present in the supplied script corpus | [adapter.rs:1698](../../crates/crysirror-cmvs/src/adapter.rs#L1698) |
| `0x22B5` | Restore a native CSV slot | Open | The connected reader accepts CrySirror checkpoints. Original CSV2 native state reconstruction is pending; Guard: x64 case RVA `0x8420e`, present in the supplied script corpus | [adapter.rs:2940](../../crates/crysirror-cmvs/src/adapter.rs#L2940) |
| `0x22B6` | Write a native CSV slot | Open | The connected writer stores a CrySirror checkpoint in CSV2 framing. Native checkpoint records are pending; Guard: x64 case RVA `0x8421b`, present in the supplied script corpus | [adapter.rs:2893](../../crates/crysirror-cmvs/src/adapter.rs#L2893) |
| `0x22B7` | Find the newest native save header in a slot range | Handled | Guard: x64 case RVA `0x84228`, present in the supplied script corpus | [adapter.rs:1713](../../crates/crysirror-cmvs/src/adapter.rs#L1713) |
| `0x22B8` | Capture the thumbnail used by the next native save | Handled | Guard: x64 case RVA `0x84235`, present in the supplied script corpus | [adapter.rs:1739](../../crates/crysirror-cmvs/src/adapter.rs#L1739) |
| `0x22B9` | Required parameter | Handled | Guard: x64 case RVA `0x84248`, present in the supplied script corpus | [startup.rs:1719](../../crates/crysirror-cmvs/src/vm/startup.rs#L1719) |
| `0x22BA` | Required parameter | Handled | Guard: x64 case RVA `0x8426b`, present in the supplied script corpus | [startup.rs:1727](../../crates/crysirror-cmvs/src/vm/startup.rs#L1727) |
| `0x22BB` | Test whether a native CSV save slot exists | Handled | Guard: x64 case RVA `0x84298`, present in the supplied script corpus | [adapter.rs:1744](../../crates/crysirror-cmvs/src/adapter.rs#L1744) |
| `0x22BC` | Update the native save slot comment | Handled | Guard: x64 case RVA `0x842a5` | [adapter.rs:1855](../../crates/crysirror-cmvs/src/adapter.rs#L1855) |
| `0x22BD` | Read native save metadata and replace a scene object's preview | Handled | Guard: x64 case RVA `0x842b2`, present in the supplied script corpus | [adapter.rs:1765](../../crates/crysirror-cmvs/src/adapter.rs#L1765) |
| `0x22BE` | Update two native save metadata words | Handled | Guard: x64 case RVA `0x842bf` | [adapter.rs:1855](../../crates/crysirror-cmvs/src/adapter.rs#L1855) |
| `0x22BF` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x22C0` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x842cc` | [cmvs-20160201.json:50](../../tools/native-api/cmvs-20160201.json#L50) |
| `0x22C1` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x842d9` | [cmvs-20160201.json:50](../../tools/native-api/cmvs-20160201.json#L50) |
| `0x22C2` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x22C3` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x22C4` | Update job | Handled | Guard: x64 case RVA `0x842e6`, present in the supplied script corpus | [adapter.rs:2632](../../crates/crysirror-cmvs/src/adapter.rs#L2632) |
| `0x22C5` | Execute advance branch | Handled | Guard: x64 case RVA `0x842f3`, present in the supplied script corpus | [adapter.rs:2671](../../crates/crysirror-cmvs/src/adapter.rs#L2671) |
| `0x22C6` | Execute advance branch | Handled | Guard: x64 case RVA `0x84300`, present in the supplied script corpus | [adapter.rs:2676](../../crates/crysirror-cmvs/src/adapter.rs#L2676) |
| `0x22C7` | Execute advance branch | Handled | Guard: x64 case RVA `0x8430d` | [adapter.rs:2681](../../crates/crysirror-cmvs/src/adapter.rs#L2681) |
| `0x22C8` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x8431a`, present in the supplied script corpus | [cmvs-20160201.json:50](../../tools/native-api/cmvs-20160201.json#L50) |
| `0x22C9` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x84327`, present in the supplied script corpus | [cmvs-20160201.json:50](../../tools/native-api/cmvs-20160201.json#L50) |
| `0x22CA` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x84334` | [cmvs-20160201.json:50](../../tools/native-api/cmvs-20160201.json#L50) |
| `0x22CB` | Execute advance branch | Handled | Guard: x64 case RVA `0x84341`, present in the supplied script corpus | [adapter.rs:1904](../../crates/crysirror-cmvs/src/adapter.rs#L1904) |
| `0x22CC` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x22CD` | Execute advance branch | Handled | Guard: x64 case RVA `0x8434e`, present in the supplied script corpus | [adapter.rs:1884](../../crates/crysirror-cmvs/src/adapter.rs#L1884) |
| `0x22CE` | Pc | Handled | Guard: x64 case RVA `0x8437a`, present in the supplied script corpus | [startup.rs:1759](../../crates/crysirror-cmvs/src/vm/startup.rs#L1759) |
| `0x22CF` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x84391` | [cmvs-20160201.json:50](../../tools/native-api/cmvs-20160201.json#L50) |
| `0x22D0` | Execute advance branch | Handled | Guard: x64 case RVA `0x843a6`, present in the supplied script corpus | [adapter.rs:2410](../../crates/crysirror-cmvs/src/adapter.rs#L2410) |
| `0x22D1` | Update visual effects | Handled | Guard: x64 case RVA `0x843b3`, present in the supplied script corpus | [adapter.rs:3927](../../crates/crysirror-cmvs/src/adapter.rs#L3927) |
| `0x22D2` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x843c0` | [cmvs-20160201.json:51](../../tools/native-api/cmvs-20160201.json#L51) |
| `0x22D3` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x843cd`, present in the supplied script corpus | [cmvs-20160201.json:51](../../tools/native-api/cmvs-20160201.json#L51) |
| `0x22D4` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x843da`, present in the supplied script corpus | [cmvs-20160201.json:51](../../tools/native-api/cmvs-20160201.json#L51) |
| `0x22D5` | Execute advance branch | Handled | Guard: x64 case RVA `0x843e7`, present in the supplied script corpus | [adapter.rs:2411](../../crates/crysirror-cmvs/src/adapter.rs#L2411) |
| `0x22D6` | Update job | Handled | Guard: x64 case RVA `0x843f4`, present in the supplied script corpus | [adapter.rs:2423](../../crates/crysirror-cmvs/src/adapter.rs#L2423) |
| `0x22D7` | Update job | Open | Effect reopen restores per-slot multiplier and pan; native settings persistence is pending; Guard: x64 case RVA `0x84401`, present in the supplied script corpus | [adapter.rs:2423](../../crates/crysirror-cmvs/src/adapter.rs#L2423) |
| `0x22D8` | Update job | Handled | Guard: x64 case RVA `0x8440e`, present in the supplied script corpus | [adapter.rs:2423](../../crates/crysirror-cmvs/src/adapter.rs#L2423) |
| `0x22D9` | Execute advance branch | Handled | Guard: x64 case RVA `0x8441b`, present in the supplied script corpus | [adapter.rs:2536](../../crates/crysirror-cmvs/src/adapter.rs#L2536) |
| `0x22DA` | Execute advance branch | Handled | Guard: x64 case RVA `0x84428`, present in the supplied script corpus | [adapter.rs:2536](../../crates/crysirror-cmvs/src/adapter.rs#L2536) |
| `0x22DB` | Execute advance branch | Handled | Guard: x64 case RVA `0x84435`, present in the supplied script corpus | [adapter.rs:2536](../../crates/crysirror-cmvs/src/adapter.rs#L2536) |
| `0x22DC` | Execute advance branch | Handled | Guard: x64 case RVA `0x84442`, present in the supplied script corpus | [adapter.rs:2536](../../crates/crysirror-cmvs/src/adapter.rs#L2536) |
| `0x22DD` | Execute advance branch | Handled | Guard: x64 case RVA `0x8444f`, present in the supplied script corpus | [adapter.rs:2536](../../crates/crysirror-cmvs/src/adapter.rs#L2536) |
| `0x22DE` | Execute advance branch | Open | Font alias selection is connected; GDI font discovery and INI writeback remain open; Guard: x64 case RVA `0x8445c`, present in the supplied script corpus | [adapter.rs:2537](../../crates/crysirror-cmvs/src/adapter.rs#L2537) |
| `0x22DF` | Execute advance branch | Open | Both nonzero outline modes currently share the same glyph outline; Guard: x64 case RVA `0x84469`, present in the supplied script corpus | [adapter.rs:2537](../../crates/crysirror-cmvs/src/adapter.rs#L2537) |
| `0x22E0` | Execute advance branch | Open | Glyph weight follows the setting; exact native font metrics remain open; Guard: x64 case RVA `0x84476`, present in the supplied script corpus | [adapter.rs:2537](../../crates/crysirror-cmvs/src/adapter.rs#L2537) |
| `0x22E1` | Execute advance branch | Handled | Guard: x64 case RVA `0x84483`, present in the supplied script corpus | [adapter.rs:2537](../../crates/crysirror-cmvs/src/adapter.rs#L2537) |
| `0x22E2` | Execute advance branch | Handled | Guard: x64 case RVA `0x84490` | [adapter.rs:2537](../../crates/crysirror-cmvs/src/adapter.rs#L2537) |
| `0x22E3` | Execute advance branch | Handled | Guard: x64 case RVA `0x8449d` | [adapter.rs:2537](../../crates/crysirror-cmvs/src/adapter.rs#L2537) |
| `0x22E4` | Execute advance branch | Handled | Guard: x64 case RVA `0x844aa`, present in the supplied script corpus | [adapter.rs:2537](../../crates/crysirror-cmvs/src/adapter.rs#L2537) |
| `0x22E5` | Pc | Handled | Guard: x64 case RVA `0x844b7`, present in the supplied script corpus | [startup.rs:1733](../../crates/crysirror-cmvs/src/vm/startup.rs#L1733) |
| `0x22E6` | Execute advance branch | Handled | Guard: x64 case RVA `0x844c4` | [adapter.rs:2537](../../crates/crysirror-cmvs/src/adapter.rs#L2537) |
| `0x22E7` | Execute advance branch | Handled | Guard: x64 case RVA `0x844d1`, present in the supplied script corpus | [adapter.rs:2537](../../crates/crysirror-cmvs/src/adapter.rs#L2537) |
| `0x22E8` | Execute advance branch | Handled | Guard: x64 case RVA `0x844de`, present in the supplied script corpus | [adapter.rs:2537](../../crates/crysirror-cmvs/src/adapter.rs#L2537) |
| `0x22E9` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x844eb`, present in the supplied script corpus | [cmvs-20160201.json:52](../../tools/native-api/cmvs-20160201.json#L52) |
| `0x22EA` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x844f8` | [cmvs-20160201.json:52](../../tools/native-api/cmvs-20160201.json#L52) |
| `0x22EB` | Pc | Handled | Guard: x64 case RVA `0x84505`, present in the supplied script corpus | [startup.rs:1733](../../crates/crysirror-cmvs/src/vm/startup.rs#L1733) |
| `0x22EC` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x84512` | [cmvs-20160201.json:52](../../tools/native-api/cmvs-20160201.json#L52) |
| `0x22ED` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x22EE` | Pc | Handled | Guard: x64 case RVA `0x8451f`, present in the supplied script corpus | [startup.rs:1759](../../crates/crysirror-cmvs/src/vm/startup.rs#L1759) |
| `0x22EF` | Input (command) | Handled | Guard: x64 case RVA `0x8453f`, present in the supplied script corpus | [input.rs:223](../../crates/crysirror-cmvs/src/vm/input.rs#L223) |
| `0x22F0` | Read or consume an input edge | Handled | Guard: x64 case RVA `0x8454c` | [input.rs:29](../../crates/crysirror-cmvs/src/vm/input.rs#L29) |
| `0x22F1` | Execute advance branch | Handled | Guard: x64 case RVA `0x84559` | [adapter.rs:2543](../../crates/crysirror-cmvs/src/adapter.rs#L2543) |
| `0x22F2` | Read or consume an input edge | Handled | Guard: x64 case RVA `0x84570` | [input.rs:30](../../crates/crysirror-cmvs/src/vm/input.rs#L30) |
| `0x22F3` | Execute advance branch | Handled | Guard: x64 case RVA `0x8457d` | [adapter.rs:2543](../../crates/crysirror-cmvs/src/adapter.rs#L2543) |
| `0x22F4` | Read or consume an input edge | Handled | Guard: x64 case RVA `0x84594` | [input.rs:31](../../crates/crysirror-cmvs/src/vm/input.rs#L31) |
| `0x22F5` | Execute advance branch | Handled | Guard: x64 case RVA `0x845a1` | [adapter.rs:2543](../../crates/crysirror-cmvs/src/adapter.rs#L2543) |
| `0x22F6` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x22F7` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x22F8` | Execute advance branch | Handled | Guard: x64 case RVA `0x845b8`, present in the supplied script corpus | [adapter.rs:2537](../../crates/crysirror-cmvs/src/adapter.rs#L2537) |
| `0x22F9` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x22FA` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x22FB` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x22FC` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x845c5`, present in the supplied script corpus | [cmvs-20160201.json:53](../../tools/native-api/cmvs-20160201.json#L53) |
| `0x22FD` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x845d2`, present in the supplied script corpus | [cmvs-20160201.json:53](../../tools/native-api/cmvs-20160201.json#L53) |
| `0x22FE` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x22FF` | Read the device's other-display setting | Handled | Guard: x64 case RVA `0x845df` | [display.rs:293](../../crates/crysirror-cmvs/src/adapter/display.rs#L293) |
| `0x2300` | Create a native offscreen render target | Open | 2016 requires native group-8/9 offscreen targets; 2021 uses its object error branch; Guard: x64 case RVA `0x845ec` | [adapter.rs:2770](../../crates/crysirror-cmvs/src/adapter.rs#L2770) |
| `0x2301` | Release a native offscreen render target | Open | 2016 requires native group-8/9 offscreen targets; 2021 uses its object error branch; Guard: x64 case RVA `0x845f9` | [adapter.rs:2770](../../crates/crysirror-cmvs/src/adapter.rs#L2770) |
| `0x2302` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2303` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2304` | Execute advance branch | Handled | Guard: x64 case RVA `0x84606`, present in the supplied script corpus | [adapter.rs:2726](../../crates/crysirror-cmvs/src/adapter.rs#L2726) |
| `0x2305` | Execute advance branch | Handled | Guard: x64 case RVA `0x84619`, present in the supplied script corpus | [adapter.rs:2731](../../crates/crysirror-cmvs/src/adapter.rs#L2731) |
| `0x2306` | Execute advance branch | Handled | Guard: x64 case RVA `0x8462c`, present in the supplied script corpus | [adapter.rs:2736](../../crates/crysirror-cmvs/src/adapter.rs#L2736) |
| `0x2307` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2308` | Update job | Handled | Guard: x64 case RVA `0x84639`, present in the supplied script corpus | [adapter.rs:3069](../../crates/crysirror-cmvs/src/adapter.rs#L3069) |
| `0x2309` | Execute advance branch | Handled | Guard: x64 case RVA `0x84646`, present in the supplied script corpus | [adapter.rs:3110](../../crates/crysirror-cmvs/src/adapter.rs#L3110) |
| `0x230A` | Execute advance branch | Open | Releases owned slot; scene movie creation is pending; Guard: x64 case RVA `0x84653`, present in the supplied script corpus | [adapter.rs:2347](../../crates/crysirror-cmvs/src/adapter.rs#L2347) |
| `0x230B` | Execute advance branch | Handled | Guard: x64 case RVA `0x84660`, present in the supplied script corpus | [adapter.rs:3110](../../crates/crysirror-cmvs/src/adapter.rs#L3110) |
| `0x230C` | Execute advance branch | Handled | Guard: x64 case RVA `0x8466d`, present in the supplied script corpus | [adapter.rs:3110](../../crates/crysirror-cmvs/src/adapter.rs#L3110) |
| `0x230D` | Execute advance branch | Handled | Guard: x64 case RVA `0x8467a` | [adapter.rs:3110](../../crates/crysirror-cmvs/src/adapter.rs#L3110) |
| `0x230E` | Update job | Handled | Guard: x64 case RVA `0x84687` | [adapter.rs:3069](../../crates/crysirror-cmvs/src/adapter.rs#L3069) |
| `0x230F` | Execute advance branch | Handled | Guard: x64 case RVA `0x84694` | [adapter.rs:3110](../../crates/crysirror-cmvs/src/adapter.rs#L3110) |
| `0x2310` | Update job | Handled | Guard: x64 case RVA `0x846a1` | [adapter.rs:3583](../../crates/crysirror-cmvs/src/adapter.rs#L3583) |
| `0x2311` | Update job | Handled | Guard: x64 case RVA `0x846ae` | [adapter.rs:3583](../../crates/crysirror-cmvs/src/adapter.rs#L3583) |
| `0x2312` | Update job | Handled | Guard: x64 case RVA `0x846bb` | [adapter.rs:3583](../../crates/crysirror-cmvs/src/adapter.rs#L3583) |
| `0x2313` | Update job | Handled | Guard: x64 case RVA `0x846c8` | [adapter.rs:3583](../../crates/crysirror-cmvs/src/adapter.rs#L3583) |
| `0x2314` | Update job | Handled | Guard: x64 case RVA `0x846d5` | [adapter.rs:3583](../../crates/crysirror-cmvs/src/adapter.rs#L3583) |
| `0x2315` | Update job | Handled | Guard: x64 case RVA `0x846e2` | [adapter.rs:3583](../../crates/crysirror-cmvs/src/adapter.rs#L3583) |
| `0x2316` | Update job | Handled | Guard: x64 case RVA `0x846ef` | [adapter.rs:3583](../../crates/crysirror-cmvs/src/adapter.rs#L3583) |
| `0x2317` | Update job | Handled | Guard: x64 case RVA `0x846fc` | [adapter.rs:3583](../../crates/crysirror-cmvs/src/adapter.rs#L3583) |
| `0x2318` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2319` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x231A` | Execute advance branch | Handled | Guard: x64 case RVA `0x84709`, present in the supplied script corpus | [adapter.rs:3110](../../crates/crysirror-cmvs/src/adapter.rs#L3110) |
| `0x231B` | Execute advance branch | Handled | Guard: x64 case RVA `0x84716` | [adapter.rs:3110](../../crates/crysirror-cmvs/src/adapter.rs#L3110) |
| `0x231C` | Execute advance branch | Handled | Guard: x64 case RVA `0x84723` | [adapter.rs:3110](../../crates/crysirror-cmvs/src/adapter.rs#L3110) |
| `0x231D` | Execute advance branch | Handled | Guard: x64 case RVA `0x84730` | [adapter.rs:3110](../../crates/crysirror-cmvs/src/adapter.rs#L3110) |
| `0x231E` | Execute advance branch | Handled | Guard: x64 case RVA `0x8473d` | [adapter.rs:3110](../../crates/crysirror-cmvs/src/adapter.rs#L3110) |
| `0x231F` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2320` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x8474a` | [cmvs-20160201.json:56](../../tools/native-api/cmvs-20160201.json#L56) |
| `0x2321` | Set a draw record's second-texture blend mode | Handled | Guard: x64 case RVA `0x84757`, present in the supplied script corpus | [adapter.rs:3748](../../crates/crysirror-cmvs/src/adapter.rs#L3748) |
| `0x2322` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x84764`, present in the supplied script corpus | [cmvs-20160201.json:56](../../tools/native-api/cmvs-20160201.json#L56) |
| `0x2323` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2324` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2325` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2326` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2327` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2328` | Execute advance branch | Handled | Guard: x64 case RVA `0x84771`, present in the supplied script corpus | [adapter.rs:2838](../../crates/crysirror-cmvs/src/adapter.rs#L2838) |
| `0x2329` | Execute advance branch | Handled | Guard: x64 case RVA `0x8477e`, present in the supplied script corpus | [adapter.rs:2838](../../crates/crysirror-cmvs/src/adapter.rs#L2838) |
| `0x232A` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x232B` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x232C` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x232D` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x232E` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x232F` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2330` | Execute advance branch | Handled | Guard: x64 case RVA `0x8478b` | [adapter.rs:2838](../../crates/crysirror-cmvs/src/adapter.rs#L2838) |
| `0x2331` | Execute advance branch | Handled | Guard: x64 case RVA `0x84798` | [adapter.rs:2838](../../crates/crysirror-cmvs/src/adapter.rs#L2838) |
| `0x2332` | Execute advance branch | Handled | Guard: x64 case RVA `0x847a5` | [adapter.rs:2838](../../crates/crysirror-cmvs/src/adapter.rs#L2838) |
| `0x2333` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2334` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2335` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2336` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2337` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2338` | Execute advance branch | Handled | Guard: x64 case RVA `0x847b2` | [adapter.rs:3416](../../crates/crysirror-cmvs/src/adapter.rs#L3416) |
| `0x2339` | Execute advance branch | Handled | Guard: x64 case RVA `0x847bf` | [adapter.rs:3440](../../crates/crysirror-cmvs/src/adapter.rs#L3440) |
| `0x233A` | Execute advance branch | Handled | Guard: x64 case RVA `0x847e3` | [adapter.rs:3440](../../crates/crysirror-cmvs/src/adapter.rs#L3440) |
| `0x233B` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x233C` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x233D` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x233E` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x233F` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2340` | Required parameter | Handled | Guard: x64 case RVA `0x847fa`, present in the supplied script corpus | [startup.rs:1723](../../crates/crysirror-cmvs/src/vm/startup.rs#L1723) |
| `0x2341` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2342` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2343` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2344` | Read or consume an input edge | Handled | Guard: x64 case RVA `0x84826`, present in the supplied script corpus | [input.rs:26](../../crates/crysirror-cmvs/src/vm/input.rs#L26) |
| `0x2345` | Execute advance branch | Handled | Guard: x64 case RVA `0x84833` | [adapter.rs:2543](../../crates/crysirror-cmvs/src/adapter.rs#L2543) |
| `0x2346` | Read or consume an input edge | Handled | Guard: x64 case RVA `0x8484a`, present in the supplied script corpus | [input.rs:27](../../crates/crysirror-cmvs/src/vm/input.rs#L27) |
| `0x2347` | Execute advance branch | Handled | Guard: x64 case RVA `0x84857` | [adapter.rs:2543](../../crates/crysirror-cmvs/src/adapter.rs#L2543) |
| `0x2348` | Read or consume an input edge | Handled | Guard: x64 case RVA `0x8486e` | [input.rs:32](../../crates/crysirror-cmvs/src/vm/input.rs#L32) |
| `0x2349` | Execute advance branch | Handled | Guard: x64 case RVA `0x8487b` | [adapter.rs:2543](../../crates/crysirror-cmvs/src/adapter.rs#L2543) |
| `0x234A` | Read or consume an input edge | Handled | Guard: x64 case RVA `0x84892`, present in the supplied script corpus | [input.rs:28](../../crates/crysirror-cmvs/src/vm/input.rs#L28) |
| `0x234B` | Execute advance branch | Handled | Guard: x64 case RVA `0x8489f` | [adapter.rs:2543](../../crates/crysirror-cmvs/src/adapter.rs#L2543) |
| `0x234C` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x234D` | Input (command) | Handled | Guard: x64 case RVA `0x848b6`, present in the supplied script corpus | [input.rs:224](../../crates/crysirror-cmvs/src/vm/input.rs#L224) |
| `0x234E` | Input (command) | Handled | Guard: x64 case RVA `0x848cd`, present in the supplied script corpus | [input.rs:225](../../crates/crysirror-cmvs/src/vm/input.rs#L225) |
| `0x234F` | Input (command) | Handled | Guard: x64 case RVA `0x848da`, present in the supplied script corpus | [input.rs:226](../../crates/crysirror-cmvs/src/vm/input.rs#L226) |
| `0x2350` | Required parameter | Handled | Guard: x64 case RVA `0x848f1`, present in the supplied script corpus | [startup.rs:1617](../../crates/crysirror-cmvs/src/vm/startup.rs#L1617) |
| `0x2351` | Required parameter | Handled | Guard: x64 case RVA `0x848fe`, present in the supplied script corpus | [startup.rs:1617](../../crates/crysirror-cmvs/src/vm/startup.rs#L1617) |
| `0x2352` | Required parameter | Handled | Guard: x64 case RVA `0x8490b`, present in the supplied script corpus | [startup.rs:1617](../../crates/crysirror-cmvs/src/vm/startup.rs#L1617) |
| `0x2353` | Required parameter | Handled | Guard: x64 case RVA `0x84918`, present in the supplied script corpus | [startup.rs:1617](../../crates/crysirror-cmvs/src/vm/startup.rs#L1617) |
| `0x2354` | Required parameter | Handled | Guard: x64 case RVA `0x84925` | [startup.rs:1617](../../crates/crysirror-cmvs/src/vm/startup.rs#L1617) |
| `0x2355` | Required parameter | Handled | Guard: x64 case RVA `0x8493c` | [startup.rs:1617](../../crates/crysirror-cmvs/src/vm/startup.rs#L1617) |
| `0x2356` | Required parameter | Handled | Guard: x64 case RVA `0x84949`, present in the supplied script corpus | [startup.rs:1617](../../crates/crysirror-cmvs/src/vm/startup.rs#L1617) |
| `0x2357` | Required parameter | Handled | Guard: x64 case RVA `0x84956`, present in the supplied script corpus | [startup.rs:1617](../../crates/crysirror-cmvs/src/vm/startup.rs#L1617) |
| `0x2358` | Required parameter | Handled | Guard: x64 case RVA `0x84963`, present in the supplied script corpus | [startup.rs:1617](../../crates/crysirror-cmvs/src/vm/startup.rs#L1617) |
| `0x2359` | Required parameter | Handled | Guard: x64 case RVA `0x84970`, present in the supplied script corpus | [startup.rs:1617](../../crates/crysirror-cmvs/src/vm/startup.rs#L1617) |
| `0x235A` | Required parameter | Handled | Guard: x64 case RVA `0x8497d`, present in the supplied script corpus | [startup.rs:1617](../../crates/crysirror-cmvs/src/vm/startup.rs#L1617) |
| `0x235B` | Required parameter | Handled | Guard: x64 case RVA `0x8498a`, present in the supplied script corpus | [startup.rs:1617](../../crates/crysirror-cmvs/src/vm/startup.rs#L1617) |
| `0x235C` | Input (command) | Handled | Guard: x64 case RVA `0x84997`, present in the supplied script corpus | [input.rs:227](../../crates/crysirror-cmvs/src/vm/input.rs#L227) |
| `0x235D` | Input (command) | Handled | Guard: x64 case RVA `0x849a4` | [input.rs:227](../../crates/crysirror-cmvs/src/vm/input.rs#L227) |
| `0x235E` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x235F` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2360` | Required parameter | Handled | Guard: x64 case RVA `0x849b1`, present in the supplied script corpus | [startup.rs:1617](../../crates/crysirror-cmvs/src/vm/startup.rs#L1617) |
| `0x2361` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x82484` | [cmvs-20160201.json:60](../../tools/native-api/cmvs-20160201.json#L60) |
| `0x2362` | Required parameter | Handled | Guard: x64 case RVA `0x849be`, present in the supplied script corpus | [startup.rs:1617](../../crates/crysirror-cmvs/src/vm/startup.rs#L1617) |
| `0x2363` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2364` | Select the pointer gesture mode and reset tracking | Handled | Guard: x64 case RVA `0x849cb`, present in the supplied script corpus | [startup.rs:1617](../../crates/crysirror-cmvs/src/vm/startup.rs#L1617) |
| `0x2365` | Required parameter | Handled | Guard: x64 case RVA `0x849d8`, present in the supplied script corpus | [startup.rs:1617](../../crates/crysirror-cmvs/src/vm/startup.rs#L1617) |
| `0x2366` | Required parameter | Handled | Guard: x64 case RVA `0x849e5`, present in the supplied script corpus | [startup.rs:1617](../../crates/crysirror-cmvs/src/vm/startup.rs#L1617) |
| `0x2367` | Required parameter | Handled | Guard: x64 case RVA `0x849f2`, present in the supplied script corpus | [startup.rs:1617](../../crates/crysirror-cmvs/src/vm/startup.rs#L1617) |
| `0x2368` | Required parameter | Handled | Guard: x64 case RVA `0x84a09`, present in the supplied script corpus | [startup.rs:1617](../../crates/crysirror-cmvs/src/vm/startup.rs#L1617) |
| `0x2369` | Required parameter | Handled | Guard: x64 case RVA `0x84a16`, present in the supplied script corpus | [startup.rs:1617](../../crates/crysirror-cmvs/src/vm/startup.rs#L1617) |
| `0x236A` | Read unrestricted-drag release status, angle and speed | Handled | Guard: x64 case RVA `0x84a23` | [startup.rs:1617](../../crates/crysirror-cmvs/src/vm/startup.rs#L1617) |
| `0x236B` | Set the unrestricted-drag movement threshold | Handled | Guard: x64 case RVA `0x84a30` | [startup.rs:1617](../../crates/crysirror-cmvs/src/vm/startup.rs#L1617) |
| `0x236C` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x236D` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x236E` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x236F` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2370` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x84a3d`, present in the supplied script corpus | [cmvs-20160201.json:61](../../tools/native-api/cmvs-20160201.json#L61) |
| `0x2371` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x84a4a`, present in the supplied script corpus | [cmvs-20160201.json:61](../../tools/native-api/cmvs-20160201.json#L61) |
| `0x2372` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x84a57`, present in the supplied script corpus | [cmvs-20160201.json:61](../../tools/native-api/cmvs-20160201.json#L61) |
| `0x2373` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x84a64`, present in the supplied script corpus | [cmvs-20160201.json:61](../../tools/native-api/cmvs-20160201.json#L61) |
| `0x2374` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x84a71`, present in the supplied script corpus | [cmvs-20160201.json:61](../../tools/native-api/cmvs-20160201.json#L61) |
| `0x2375` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x84a7e`, present in the supplied script corpus | [cmvs-20160201.json:61](../../tools/native-api/cmvs-20160201.json#L61) |
| `0x2376` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x84a8b`, present in the supplied script corpus | [cmvs-20160201.json:61](../../tools/native-api/cmvs-20160201.json#L61) |
| `0x2377` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x84a98`, present in the supplied script corpus | [cmvs-20160201.json:61](../../tools/native-api/cmvs-20160201.json#L61) |
| `0x2378` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x84aa5` | [cmvs-20160201.json:61](../../tools/native-api/cmvs-20160201.json#L61) |
| `0x2379` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x84ab2` | [cmvs-20160201.json:61](../../tools/native-api/cmvs-20160201.json#L61) |
| `0x237A` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x84abf` | [cmvs-20160201.json:61](../../tools/native-api/cmvs-20160201.json#L61) |
| `0x237B` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x84acc` | [cmvs-20160201.json:61](../../tools/native-api/cmvs-20160201.json#L61) |
| `0x237C` | Execute advance branch | Handled | Guard: x64 case RVA `0x84ad9`, present in the supplied script corpus | [adapter.rs:2354](../../crates/crysirror-cmvs/src/adapter.rs#L2354) |
| `0x237D` | Execute advance branch | Handled | Guard: x64 case RVA `0x84ae6`, present in the supplied script corpus | [adapter.rs:2354](../../crates/crysirror-cmvs/src/adapter.rs#L2354) |
| `0x237E` | Execute advance branch | Handled | Guard: x64 case RVA `0x84af3`, present in the supplied script corpus | [adapter.rs:2354](../../crates/crysirror-cmvs/src/adapter.rs#L2354) |
| `0x237F` | Execute advance branch | Handled | Guard: x64 case RVA `0x84b00` | [adapter.rs:2354](../../crates/crysirror-cmvs/src/adapter.rs#L2354) |
| `0x2380` | Execute advance branch | Handled | Guard: x64 case RVA `0x84b0d`, present in the supplied script corpus | [adapter.rs:1632](../../crates/crysirror-cmvs/src/adapter.rs#L1632) |
| `0x2381` | Execute advance branch | Handled | Guard: x64 case RVA `0x84b1a` | [adapter.rs:1633](../../crates/crysirror-cmvs/src/adapter.rs#L1633) |
| `0x2382` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x84b27`, present in the supplied script corpus | [cmvs-20160201.json:62](../../tools/native-api/cmvs-20160201.json#L62) |
| `0x2383` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x84b34`, present in the supplied script corpus | [cmvs-20160201.json:62](../../tools/native-api/cmvs-20160201.json#L62) |
| `0x2384` | Execute advance branch | Handled | Guard: x64 case RVA `0x84b41` | [adapter.rs:2713](../../crates/crysirror-cmvs/src/adapter.rs#L2713) |
| `0x2385` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x84b4e`, present in the supplied script corpus | [cmvs-20160201.json:62](../../tools/native-api/cmvs-20160201.json#L62) |
| `0x2386` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x84b5b`, present in the supplied script corpus | [cmvs-20160201.json:62](../../tools/native-api/cmvs-20160201.json#L62) |
| `0x2387` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x84b68`, present in the supplied script corpus | [cmvs-20160201.json:62](../../tools/native-api/cmvs-20160201.json#L62) |
| `0x2388` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x84b75` | [cmvs-20160201.json:62](../../tools/native-api/cmvs-20160201.json#L62) |
| `0x2389` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x84b82`, present in the supplied script corpus | [cmvs-20160201.json:62](../../tools/native-api/cmvs-20160201.json#L62) |
| `0x238A` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x84b8f` | [cmvs-20160201.json:62](../../tools/native-api/cmvs-20160201.json#L62) |
| `0x238B` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x84b9c`, present in the supplied script corpus | [cmvs-20160201.json:62](../../tools/native-api/cmvs-20160201.json#L62) |
| `0x238C` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x84ba9`, present in the supplied script corpus | [cmvs-20160201.json:62](../../tools/native-api/cmvs-20160201.json#L62) |
| `0x238D` | Execute advance branch | Handled | Guard: x64 case RVA `0x84bb6` | [adapter.rs:2713](../../crates/crysirror-cmvs/src/adapter.rs#L2713) |
| `0x238E` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x84bc3`, present in the supplied script corpus | [cmvs-20160201.json:62](../../tools/native-api/cmvs-20160201.json#L62) |
| `0x238F` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x84bd0`, present in the supplied script corpus | [cmvs-20160201.json:62](../../tools/native-api/cmvs-20160201.json#L62) |
| `0x2390` | Execute advance branch | Handled | Guard: x64 case RVA `0x84bdd`, present in the supplied script corpus | [adapter.rs:2700](../../crates/crysirror-cmvs/src/adapter.rs#L2700) |
| `0x2391` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x84bea` | [cmvs-20160201.json:63](../../tools/native-api/cmvs-20160201.json#L63) |
| `0x2392` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x84bf7`, present in the supplied script corpus | [cmvs-20160201.json:63](../../tools/native-api/cmvs-20160201.json#L63) |
| `0x2393` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x84c04`, present in the supplied script corpus | [cmvs-20160201.json:63](../../tools/native-api/cmvs-20160201.json#L63) |
| `0x2394` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x84c11` | [cmvs-20160201.json:63](../../tools/native-api/cmvs-20160201.json#L63) |
| `0x2395` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2396` | Execute advance branch | Handled | Guard: x64 case RVA `0x84c1e`, present in the supplied script corpus | [adapter.rs:2700](../../crates/crysirror-cmvs/src/adapter.rs#L2700) |
| `0x2397` | Update job | Handled | Guard: x64 case RVA `0x84c2b`, present in the supplied script corpus | [adapter.rs:3510](../../crates/crysirror-cmvs/src/adapter.rs#L3510) |
| `0x2398` | Execute advance branch | Handled | Guard: x64 case RVA `0x84c38`, present in the supplied script corpus | [adapter.rs:2713](../../crates/crysirror-cmvs/src/adapter.rs#L2713) |
| `0x2399` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x239A` | Execute advance branch | Handled | Guard: x64 case RVA `0x84c45`, present in the supplied script corpus | [adapter.rs:2700](../../crates/crysirror-cmvs/src/adapter.rs#L2700) |
| `0x239B` | Execute advance branch | Handled | Guard: x64 case RVA `0x84c52`, present in the supplied script corpus | [adapter.rs:2700](../../crates/crysirror-cmvs/src/adapter.rs#L2700) |
| `0x239C` | Execute advance branch | Handled | Guard: x64 case RVA `0x84c5f` | [adapter.rs:2700](../../crates/crysirror-cmvs/src/adapter.rs#L2700) |
| `0x239D` | Execute advance branch | Handled | Guard: x64 case RVA `0x84c6c`, present in the supplied script corpus | [adapter.rs:2700](../../crates/crysirror-cmvs/src/adapter.rs#L2700) |
| `0x239E` | Execute advance branch | Handled | Guard: x64 case RVA `0x84c79`, present in the supplied script corpus | [adapter.rs:2700](../../crates/crysirror-cmvs/src/adapter.rs#L2700) |
| `0x239F` | Execute advance branch | Handled | Guard: x64 case RVA `0x84c86`, present in the supplied script corpus | [adapter.rs:2700](../../crates/crysirror-cmvs/src/adapter.rs#L2700) |
| `0x23A0` | Execute advance branch | Handled | Guard: x64 case RVA `0x84c93`, present in the supplied script corpus | [adapter.rs:2700](../../crates/crysirror-cmvs/src/adapter.rs#L2700) |
| `0x23A1` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x23A2` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x23A3` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x23A4` | Waiting | Handled | Guard: x64 case RVA `0x84ca0` | [adapter.rs:3674](../../crates/crysirror-cmvs/src/adapter.rs#L3674) |
| `0x23A5` | Waiting | Handled | Guard: x64 case RVA `0x84cad` | [adapter.rs:3674](../../crates/crysirror-cmvs/src/adapter.rs#L3674) |
| `0x23A6` | Waiting | Handled | Guard: x64 case RVA `0x84cba` | [adapter.rs:3674](../../crates/crysirror-cmvs/src/adapter.rs#L3674) |
| `0x23A7` | Waiting | Handled | Guard: x64 case RVA `0x84cc7` | [adapter.rs:3674](../../crates/crysirror-cmvs/src/adapter.rs#L3674) |
| `0x23A8` | Waiting | Handled | Guard: x64 case RVA `0x84cd4` | [adapter.rs:3674](../../crates/crysirror-cmvs/src/adapter.rs#L3674) |
| `0x23A9` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x23AA` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x23AB` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x23AC` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x23AD` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x23AE` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x23AF` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x23B0` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x23B1` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x23B2` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x84ce1` | [cmvs-20160201.json:65](../../tools/native-api/cmvs-20160201.json#L65) |
| `0x23B3` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x23B4` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x23B5` | Control flow (command) | Handled | Guard: x64 case RVA `0x825b1` | [control.rs:167](../../crates/crysirror-cmvs/src/vm/control.rs#L167) |
| `0x23B6` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x23B7` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x82484` | [cmvs-20160201.json:65](../../tools/native-api/cmvs-20160201.json#L65) |
| `0x23B8` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x23B9` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x23BA` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x23BB` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x23BC` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x23BD` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x23BE` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x23BF` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x23C0` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x23C1` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x23C2` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x23C3` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x23C4` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x23C5` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x23C6` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x23C7` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x23C8` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x23C9` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x23CA` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x23CB` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x23CC` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x23CD` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x23CE` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x23CF` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |

## Subselectors and conditional dispatch

Patterns retain their source namespace and guards. `_` is the fallback of that table. Boolean/optional results follow the function's return contract. Call-site names identify the implementation to inspect, including indirect service calls.

### adapter.rs / read_scene_image

```rust
fn read_scene_image(
    sources: &ScriptSources,
    name: &str,
    context: &DecodeContext,
) -> Result<Option<visual::DecodedImage>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `visual::prepare_image(sources, name, context)` | `Ok(image)` | Local state/value operations | Handled | [920](../../crates/crysirror-cmvs/src/adapter.rs#L920) |
| `visual::prepare_image(sources, name, context)` | `Err(error) if error.to_string().starts_with("CMVS image missing")` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [921](../../crates/crysirror-cmvs/src/adapter.rs#L921) |
| `visual::prepare_image(sources, name, context)` | `Err(error)` | Local state/value operations | Unsupported: Unconditional rejection | [922](../../crates/crysirror-cmvs/src/adapter.rs#L922) |

### adapter.rs / Adapter::advance

```rust
fn advance(
        &mut self,
        host: &mut dyn HostServices,
        input: InputFrame,
        budget: u32,
    ) -> Result<HostYield>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under step.1=0x237c..=0x237f` | `!session.buttons.list_command(step.1, &mut session.vm)?` | Local state/value operations | Handled | [2355](../../crates/crysirror-cmvs/src/adapter.rs#L2355) |
| `condition; under step.1=0x22d5` | `mode == 0` | u32::from | Handled | [2413](../../crates/crysirror-cmvs/src/adapter.rs#L2413) |
| `condition; under step.1=0x22d6..=0x22d8` | `mode == 0` | u32::from | Handled | [2430](../../crates/crysirror-cmvs/src/adapter.rs#L2430) |
| `opcode; under step.1=0x228c..=0x228e` | `0x228c if first != second` | Local state/value operations | Handled | [2806](../../crates/crysirror-cmvs/src/adapter.rs#L2806) |
| `opcode; under step.1=0x228c..=0x228e` | `0x228d` | Local state/value operations | Handled | [2821](../../crates/crysirror-cmvs/src/adapter.rs#L2821) |
| `opcode; under step.1=0x228c..=0x228e` | `0x228e if first != second` | Local state/value operations | Handled | [2822](../../crates/crysirror-cmvs/src/adapter.rs#L2822) |
| `opcode; under step.1=0x228c..=0x228e` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [2831](../../crates/crysirror-cmvs/src/adapter.rs#L2831) |
| `visual::prepare_image(&sources, &record.name, &context.clone().with_cancellation(cancel.clone())); under step.1=0x22b5` | `Ok(image)` | Local state/value operations | Handled | [3023](../../crates/crysirror-cmvs/src/adapter.rs#L3023) |
| `visual::prepare_image(&sources, &record.name, &context.clone().with_cancellation(cancel.clone())); under step.1=0x22b5` | `Err(error) if error.to_string().starts_with("CMVS image missing")` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [3024](../../crates/crysirror-cmvs/src/adapter.rs#L3024) |
| `visual::prepare_image(&sources, &record.name, &context.clone().with_cancellation(cancel.clone())); under step.1=0x22b5` | `Err(error)` | Local state/value operations | Unsupported: Unconditional rejection | [3025](../../crates/crysirror-cmvs/src/adapter.rs#L3025) |
| `visual::prepare_image(&sources, name, &context.clone().with_cancellation(cancel.clone())); under step.1=0x22b5` | `Ok(image)` | Local state/value operations | Handled | [3031](../../crates/crysirror-cmvs/src/adapter.rs#L3031) |
| `visual::prepare_image(&sources, name, &context.clone().with_cancellation(cancel.clone())); under step.1=0x22b5` | `Err(error) if error.to_string().starts_with("CMVS image missing")` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [3032](../../crates/crysirror-cmvs/src/adapter.rs#L3032) |
| `visual::prepare_image(&sources, name, &context.clone().with_cancellation(cancel.clone())); under step.1=0x22b5` | `Err(error)` | Local state/value operations | Unsupported: Unconditional rejection | [3033](../../crates/crysirror-cmvs/src/adapter.rs#L3033) |
| `opcode; under step.1=0x2290..=0x2297` | `0x2290` | Local state/value operations | Handled | [3254](../../crates/crysirror-cmvs/src/adapter.rs#L3254) |
| `opcode; under step.1=0x2290..=0x2297` | `0x2292` | scene_effect::TOWER_TEXTURES.map | Handled | [3258](../../crates/crysirror-cmvs/src/adapter.rs#L3258) |
| `opcode; under step.1=0x2290..=0x2297` | `0x2294` | scene_effect::PARTICLE_TEXTURES.map | Handled | [3259](../../crates/crysirror-cmvs/src/adapter.rs#L3259) |
| `opcode; under step.1=0x2290..=0x2297` | `0x2295` | scene_effect::ACTOR_TEXTURES.map | Handled | [3260](../../crates/crysirror-cmvs/src/adapter.rs#L3260) |
| `opcode; under step.1=0x2290..=0x2297` | `0x2296` | scene_effect::SPIN_TEXTURES.map | Handled | [3261](../../crates/crysirror-cmvs/src/adapter.rs#L3261) |
| `opcode; under step.1=0x2290..=0x2297` | `0x2297` | scene_effect::ORBIT_TEXTURES.map | Handled | [3262](../../crates/crysirror-cmvs/src/adapter.rs#L3262) |
| `opcode; under step.1=0x2290..=0x2297` | `_` | Vec::new | Handled | [3263](../../crates/crysirror-cmvs/src/adapter.rs#L3263) |
| `opcode; under step.1=0x2290..=0x2297` | `0x2291` | Local state/value operations | Handled | [3266](../../crates/crysirror-cmvs/src/adapter.rs#L3266) |
| `opcode; under step.1=0x2290..=0x2297` | `0x2292` | Local state/value operations | Handled | [3267](../../crates/crysirror-cmvs/src/adapter.rs#L3267) |
| `opcode; under step.1=0x2290..=0x2297` | `0x2294` | Local state/value operations | Handled | [3268](../../crates/crysirror-cmvs/src/adapter.rs#L3268) |
| `opcode; under step.1=0x2290..=0x2297` | `0x2295` | Local state/value operations | Handled | [3269](../../crates/crysirror-cmvs/src/adapter.rs#L3269) |
| `opcode; under step.1=0x2290..=0x2297` | `_` | Local state/value operations | Handled | [3270](../../crates/crysirror-cmvs/src/adapter.rs#L3270) |
| `condition; under step.1=0x2290..=0x2297` | `opcode == 0x2293 \|\| !names.is_empty()` | ScriptSources::prepare, scene_effect::DOOR_PANELS.into_iter | Handled | [3280](../../crates/crysirror-cmvs/src/adapter.rs#L3280) |
| `condition; under step.1=0x2290..=0x2297` | `opcode == 0x2293` | scene_effect::DOOR_PANELS.into_iter | Handled | [3282](../../crates/crysirror-cmvs/src/adapter.rs#L3282) |
| `visual::prepare_image(&sources, &name, &context.with_cancellation(cancel.clone())); under step.1=0x22a1..=0x22a7 \| 0x22a9` | `Ok(image)` | Local state/value operations | Handled | [3404](../../crates/crysirror-cmvs/src/adapter.rs#L3404) |
| `visual::prepare_image(&sources, &name, &context.with_cancellation(cancel.clone())); under step.1=0x22a1..=0x22a7 \| 0x22a9` | `Err(error) if error.to_string().starts_with("CMVS image missing")` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [3405](../../crates/crysirror-cmvs/src/adapter.rs#L3405) |
| `visual::prepare_image(&sources, &name, &context.with_cancellation(cancel.clone())); under step.1=0x22a1..=0x22a7 \| 0x22a9` | `Err(error)` | Local state/value operations | Unsupported: Unconditional rejection | [3406](../../crates/crysirror-cmvs/src/adapter.rs#L3406) |
| `visual::prepare_image(&sources, &request.name, &context.with_cancellation(cancel.clone())); under step.1=0x222e` | `Ok(image)` | Local state/value operations | Handled | [3485](../../crates/crysirror-cmvs/src/adapter.rs#L3485) |
| `visual::prepare_image(&sources, &request.name, &context.with_cancellation(cancel.clone())); under step.1=0x222e` | `Err(error) if error.to_string().starts_with("CMVS image missing")` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [3486](../../crates/crysirror-cmvs/src/adapter.rs#L3486) |
| `visual::prepare_image(&sources, &request.name, &context.with_cancellation(cancel.clone())); under step.1=0x222e` | `Err(error)` | Local state/value operations | Unsupported: Unconditional rejection | [3487](../../crates/crysirror-cmvs/src/adapter.rs#L3487) |
| `visual::prepare_image_with(&sources, &name, prefetched, &context.with_cancellation(cancel.clone())); under step.1=0x2397` | `Ok(image)` | Local state/value operations | Handled | [3527](../../crates/crysirror-cmvs/src/adapter.rs#L3527) |
| `visual::prepare_image_with(&sources, &name, prefetched, &context.with_cancellation(cancel.clone())); under step.1=0x2397` | `Err(error) if error.to_string().starts_with("CMVS image missing")` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [3528](../../crates/crysirror-cmvs/src/adapter.rs#L3528) |
| `visual::prepare_image_with(&sources, &name, prefetched, &context.with_cancellation(cancel.clone())); under step.1=0x2397` | `Err(error)` | Local state/value operations | Unsupported: Unconditional rejection | [3529](../../crates/crysirror-cmvs/src/adapter.rs#L3529) |
| `visual::prepare_image_with( &sources, &name, prefetched, &context.with_cancellation(cancel.clone()), ); under step.1=0x2035` | `Ok(image)` | Local state/value operations | Handled | [3557](../../crates/crysirror-cmvs/src/adapter.rs#L3557) |
| `visual::prepare_image_with( &sources, &name, prefetched, &context.with_cancellation(cancel.clone()), ); under step.1=0x2035` | `Err(error) if error.to_string().starts_with("CMVS image missing")` | Local state/value operations | Handled | [3558](../../crates/crysirror-cmvs/src/adapter.rs#L3558) |
| `visual::prepare_image_with( &sources, &name, prefetched, &context.with_cancellation(cancel.clone()), ); under step.1=0x2035` | `Err(error)` | Local state/value operations | Unsupported: Unconditional rejection | [3563](../../crates/crysirror-cmvs/src/adapter.rs#L3563) |
| `visual::prepare_image(&sources, &name, &context.with_cancellation(cancel.clone())); under step.1=0x2310..=0x2317 / step.1=0x2310` | `Ok(image)` | Local state/value operations | Handled | [3611](../../crates/crysirror-cmvs/src/adapter.rs#L3611) |
| `visual::prepare_image(&sources, &name, &context.with_cancellation(cancel.clone())); under step.1=0x2310..=0x2317 / step.1=0x2310` | `Err(error) if error.to_string().starts_with("CMVS image missing")` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [3612](../../crates/crysirror-cmvs/src/adapter.rs#L3612) |
| `visual::prepare_image(&sources, &name, &context.with_cancellation(cancel.clone())); under step.1=0x2310..=0x2317 / step.1=0x2310` | `Err(error)` | Local state/value operations | Unsupported: Unconditional rejection | [3613](../../crates/crysirror-cmvs/src/adapter.rs#L3613) |
| `visual::prepare_image(&sources, &name, &context.with_cancellation(cancel.clone())); under step.1=0x23c0` | `Ok(image)` | Local state/value operations | Handled | [3666](../../crates/crysirror-cmvs/src/adapter.rs#L3666) |
| `visual::prepare_image(&sources, &name, &context.with_cancellation(cancel.clone())); under step.1=0x23c0` | `Err(error) if error.to_string().starts_with("CMVS image missing")` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [3667](../../crates/crysirror-cmvs/src/adapter.rs#L3667) |
| `visual::prepare_image(&sources, &name, &context.with_cancellation(cancel.clone())); under step.1=0x23c0` | `Err(error)` | Local state/value operations | Unsupported: Unconditional rejection | [3668](../../crates/crysirror-cmvs/src/adapter.rs#L3668) |
| `condition; under step.1=opcode if session.display.command( opcode, &mut session.vm, host, self.client, self.canvas, )?` | `opcode == 0x22f9` | Local state/value operations | Handled | [3731](../../crates/crysirror-cmvs/src/adapter.rs#L3731) |
| `condition; under step.1=opcode if session.display.command( opcode, &mut session.vm, host, self.client, self.canvas, )?` | `opcode == 0x22f9 && session.menu.published()` | Local state/value operations | Handled | [3735](../../crates/crysirror-cmvs/src/adapter.rs#L3735) |
| `condition; under step.1=0x2025 \| 0x2039 \| 0x2026 \| 0x202c..=0x202d \| 0x2041..=0x2043 \| 0x2049..=0x204c \| 0x204e \| 0x204f \| 0x2051 \| 0x2054 \| 0x2055 \| 0x2070..=0x2075 \| 0x2078 \| 0x2321` | `let Some(bit) = session.objects.property_command( step.1, host, self.client, &mut session.vm, )?` | Local state/value operations | Handled | [3764](../../crates/crysirror-cmvs/src/adapter.rs#L3764) |
| `condition; under step.1=0x21e0..=0x21e9 \| 0x21ef..=0x21fd` | `let Some(bit) = session.objects.track_command( step.1, host, self.client, &mut session.vm, )?` | Local state/value operations | Handled | [3834](../../crates/crysirror-cmvs/src/adapter.rs#L3834) |
| `visual::prepare_image(&sources, &request.name, &context.with_cancellation(cancel.clone())); under step.1=0x222a \| 0x2234` | `Ok(image) if request.reveal` | Local state/value operations | Handled | [3875](../../crates/crysirror-cmvs/src/adapter.rs#L3875) |
| `visual::prepare_image(&sources, &request.name, &context.with_cancellation(cancel.clone())); under step.1=0x222a \| 0x2234` | `Ok(image)` | Local state/value operations | Handled | [3876](../../crates/crysirror-cmvs/src/adapter.rs#L3876) |
| `visual::prepare_image(&sources, &request.name, &context.with_cancellation(cancel.clone())); under step.1=0x222a \| 0x2234` | `Err(error) if error.to_string().starts_with("CMVS image missing")` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [3877](../../crates/crysirror-cmvs/src/adapter.rs#L3877) |
| `visual::prepare_image(&sources, &request.name, &context.with_cancellation(cancel.clone())); under step.1=0x222a \| 0x2234` | `Err(error)` | Local state/value operations | Unsupported: Unconditional rejection | [3878](../../crates/crysirror-cmvs/src/adapter.rs#L3878) |

### adapter.rs / Session::create_effect

```rust
fn create_effect(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        opcode: u16,
        index: usize,
        name: String,
        parameters: Vec<u32>,
        image: Option<visual::DecodedImage>,
    ) -> Result<u16>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `opcode` | `0x22a2` | Local state/value operations | Handled | [304](../../crates/crysirror-cmvs/src/adapter.rs#L304) |
| `opcode` | `0x22a5` | Local state/value operations | Handled | [308](../../crates/crysirror-cmvs/src/adapter.rs#L308) |
| `opcode` | `0x22a1 \| 0x22a4` | Local state/value operations | Handled | [312](../../crates/crysirror-cmvs/src/adapter.rs#L312) |
| `opcode` | `0x22a3` | Local state/value operations | Handled | [316](../../crates/crysirror-cmvs/src/adapter.rs#L316) |
| `opcode` | `0x22a9` | Local state/value operations | Handled | [320](../../crates/crysirror-cmvs/src/adapter.rs#L320) |
| `opcode` | `_` | Local state/value operations | Handled | [324](../../crates/crysirror-cmvs/src/adapter.rs#L324) |

### adapter.rs / Session::install_special_scene

```rust
fn install_special_scene(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        opcode: u16,
        index: usize,
        images: SceneImages,
        sounds: Option<sample::PreparedGroup>,
    ) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `opcode == 0x2291` | scene_effect::Scene::Walk, scene_effect::Walk::start | Handled | [229](../../crates/crysirror-cmvs/src/adapter.rs#L229) |
| `opcode` | `0x2293` | self.light.set_view | Handled | [246](../../crates/crysirror-cmvs/src/adapter.rs#L246) |
| `opcode` | `0x2294` | self.light.set_view | Handled | [247](../../crates/crysirror-cmvs/src/adapter.rs#L247) |
| `opcode` | `0x2297` | self.light.set_view | Handled | [248](../../crates/crysirror-cmvs/src/adapter.rs#L248) |
| `opcode` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [249](../../crates/crysirror-cmvs/src/adapter.rs#L249) |

### adapter.rs / Session::spawn_sequence_images

```rust
fn spawn_sequence_images(
        &self,
        pool: &TaskPool,
        client: ClientId,
        loads: Vec<(SequenceTarget, String, u32)>,
    ) -> Result<Task<T>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `visual::prepare_image(&sources, &name, &context.clone().with_cancellation(cancel.clone()))` | `Ok(image)` | Local state/value operations | Handled | [384](../../crates/crysirror-cmvs/src/adapter.rs#L384) |
| `visual::prepare_image(&sources, &name, &context.clone().with_cancellation(cancel.clone()))` | `Err(error) if error.to_string().starts_with("CMVS image missing")` | Local state/value operations | Empty handler: Original no-op status not established | [385](../../crates/crysirror-cmvs/src/adapter.rs#L385) |
| `visual::prepare_image(&sources, &name, &context.clone().with_cancellation(cancel.clone()))` | `Err(error)` | Local state/value operations | Unsupported: Unconditional rejection | [386](../../crates/crysirror-cmvs/src/adapter.rs#L386) |

### adapter.rs / Session::volume_command

```rust
fn volume_command(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        opcode: u16,
    ) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `mode == 0` | self.vm.complete_owner_call | Handled | [600](../../crates/crysirror-cmvs/src/adapter.rs#L600) |
| `condition` | `matches!(opcode, 0x22d9 \| 0x22dd)` | Local state/value operations | Handled | [611](../../crates/crysirror-cmvs/src/adapter.rs#L611) |
| `condition` | `matches!(opcode, 0x22d9 \| 0x22dd)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [611](../../crates/crysirror-cmvs/src/adapter.rs#L611) |

### adapter/buttons.rs / Groups::apply

```rust
fn apply(
        &mut self,
        opcode: u16,
        vm: &mut CmvsVm,
        objects: &mut Objects,
        host: &mut dyn HostServices,
        client: ClientId,
        canvas: [u32; 2],
    ) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `opcode == 0x2087` | Local state/value operations | Handled | [86](../../crates/crysirror-cmvs/src/adapter/buttons.rs#L86) |
| `condition; under opcode=0x2210 \| 0x221b` | `opcode == 0x221b` | Local state/value operations | Handled | [99](../../crates/crysirror-cmvs/src/adapter/buttons.rs#L99) |
| `opcode; under opcode=opcode` | `0x2212` | ChildImage::default | Handled | [136](../../crates/crysirror-cmvs/src/adapter/buttons.rs#L136) |
| `opcode; under opcode=opcode` | `0x2213` | Local state/value operations | Handled | [156](../../crates/crysirror-cmvs/src/adapter/buttons.rs#L156) |
| `opcode; under opcode=opcode` | `0x2214` | Local state/value operations | Handled | [163](../../crates/crysirror-cmvs/src/adapter/buttons.rs#L163) |
| `opcode; under opcode=opcode` | `0x2215` | Local state/value operations | Handled | [186](../../crates/crysirror-cmvs/src/adapter/buttons.rs#L186) |
| `opcode; under opcode=opcode` | `0x2216` | Local state/value operations | Handled | [196](../../crates/crysirror-cmvs/src/adapter/buttons.rs#L196) |
| `opcode; under opcode=opcode` | `0x2218` | Local state/value operations | Handled | [200](../../crates/crysirror-cmvs/src/adapter/buttons.rs#L200) |
| `opcode; under opcode=opcode` | `0x2219` | Local state/value operations | Handled | [205](../../crates/crysirror-cmvs/src/adapter/buttons.rs#L205) |
| `opcode; under opcode=opcode` | `0x2217` | Local state/value operations | Handled | [213](../../crates/crysirror-cmvs/src/adapter/buttons.rs#L213) |
| `opcode; under opcode=opcode` | `0x221a` | u32::from | Handled | [214](../../crates/crysirror-cmvs/src/adapter/buttons.rs#L214) |
| `opcode; under opcode=opcode` | `_` | Local state/value operations | Handled | [239](../../crates/crysirror-cmvs/src/adapter/buttons.rs#L239) |
| `opcode` | `0x2210 \| 0x221b` | Vec::new | Handled | [98](../../crates/crysirror-cmvs/src/adapter/buttons.rs#L98) |
| `opcode` | `0x2211` | Local state/value operations | Handled | [129](../../crates/crysirror-cmvs/src/adapter/buttons.rs#L129) |
| `opcode` | `opcode` | ChildImage::default, u32::from | Handled | [133](../../crates/crysirror-cmvs/src/adapter/buttons.rs#L133) |

### adapter/buttons.rs / Groups::list_command

```rust
fn list_command(&mut self, opcode: u16, vm: &mut CmvsVm) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `opcode` | `0x237d` | Local state/value operations | Handled | [252](../../crates/crysirror-cmvs/src/adapter/buttons.rs#L252) |
| `opcode` | `0x237f` | Local state/value operations | Handled | [253](../../crates/crysirror-cmvs/src/adapter/buttons.rs#L253) |
| `opcode` | `_` | Local state/value operations | Handled | [254](../../crates/crysirror-cmvs/src/adapter/buttons.rs#L254) |
| `opcode` | `0x237c` | Local state/value operations | Handled | [266](../../crates/crysirror-cmvs/src/adapter/buttons.rs#L266) |
| `opcode` | `0x237d` | Local state/value operations | Handled | [270](../../crates/crysirror-cmvs/src/adapter/buttons.rs#L270) |
| `opcode` | `0x237e` | Local state/value operations | Handled | [274](../../crates/crysirror-cmvs/src/adapter/buttons.rs#L274) |
| `opcode` | `_` | Local state/value operations | Handled | [278](../../crates/crysirror-cmvs/src/adapter/buttons.rs#L278) |

### adapter/composite.rs / Composites::adjust

```rust
fn adjust(&mut self, slot: usize, opcode: u16, layer: usize, values: [u32; 2]) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `opcode` | `0x2313` | Local state/value operations | Handled | [64](../../crates/crysirror-cmvs/src/adapter/composite.rs#L64) |
| `opcode` | `0x2314` | Local state/value operations | Handled | [67](../../crates/crysirror-cmvs/src/adapter/composite.rs#L67) |
| `opcode` | `_` | Local state/value operations | Handled | [70](../../crates/crysirror-cmvs/src/adapter/composite.rs#L70) |

### adapter/dialog.rs / Dialogs::command

```rust
fn command(
        &mut self,
        opcode: u16,
        vm: &mut CmvsVm,
        host: &mut dyn HostServices,
        client: ClientId,
        canvas: [u32; 2],
        font: &str,
    ) -> Result<Outcome>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under opcode=0x213b` | `(mode as i32).unsigned_abs() == 0` | self.open_choice | Handled | [98](../../crates/crysirror-cmvs/src/adapter/dialog.rs#L98) |
| `opcode` | `0x213a` | self.open_choice | Handled | [89](../../crates/crysirror-cmvs/src/adapter/dialog.rs#L89) |
| `opcode` | `0x213b` | self.open_choice | Handled | [93](../../crates/crysirror-cmvs/src/adapter/dialog.rs#L93) |
| `opcode` | `0x213c` | self.open_edit | Handled | [105](../../crates/crysirror-cmvs/src/adapter/dialog.rs#L105) |
| `opcode` | `0x213d` | host.font_library, self.open_choice | Handled | [114](../../crates/crysirror-cmvs/src/adapter/dialog.rs#L114) |
| `opcode` | `_` | Local state/value operations | Handled | [124](../../crates/crysirror-cmvs/src/adapter/dialog.rs#L124) |

### adapter/dialog.rs / Dialogs::poll

```rust
fn poll(&mut self, vm: &mut CmvsVm, host: &mut dyn HostServices, client: ClientId) -> Result<Outcome>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `event["kind"] == "activate" && matches!(id, Some(EDIT_OK \| EDIT_FIELD))` | Local state/value operations | Handled | [194](../../crates/crysirror-cmvs/src/adapter/dialog.rs#L194) |
| `condition` | `event["kind"] == "activate" && id == Some(EDIT_CANCEL)` | Local state/value operations | Handled | [196](../../crates/crysirror-cmvs/src/adapter/dialog.rs#L196) |
| `condition` | `event["kind"] == "dialog_result" && event["widget"]["id"].as_u64() == Some(u64::from(DIALOG))` | Local state/value operations | Handled | [224](../../crates/crysirror-cmvs/src/adapter/dialog.rs#L224) |
| `condition; under pending.kind=_` | `opcode == 0x213b` | Local state/value operations | Handled | [242](../../crates/crysirror-cmvs/src/adapter/dialog.rs#L242) |

### adapter/display.rs / Display::alpha_command

```rust
fn alpha_command(&mut self, vm: &mut CmvsVm) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `mode == 0` | u32::from | Handled | [191](../../crates/crysirror-cmvs/src/adapter/display.rs#L191) |

### adapter/display.rs / Display::command

```rust
fn command(
        &mut self,
        opcode: u16,
        vm: &mut CmvsVm,
        host: &mut dyn HostServices,
        client: ClientId,
        canvas: [u32; 2],
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under opcode=0x22fd` | `mode <= 3` | Local state/value operations | Handled | [442](../../crates/crysirror-cmvs/src/adapter/display.rs#L442) |
| `opcode` | `0x22f9 if revision` | self.alpha_command | Handled | [292](../../crates/crysirror-cmvs/src/adapter/display.rs#L292) |
| `opcode` | `0x22ff if self.profile == VmProfile::Pc20160201X64` | u32::from | Handled | [293](../../crates/crysirror-cmvs/src/adapter/display.rs#L293) |
| `opcode` | `0x2005` | self.frame_period | Handled | [296](../../crates/crysirror-cmvs/src/adapter/display.rs#L296) |
| `opcode` | `0x20cd` | Self::submit_ui | Handled | [297](../../crates/crysirror-cmvs/src/adapter/display.rs#L297) |
| `opcode` | `0x2163` | self.observe, u32::from | Handled | [304](../../crates/crysirror-cmvs/src/adapter/display.rs#L304) |
| `opcode` | `0x2164` | self.write, u32::from, self.apply_mode | Handled | [308](../../crates/crysirror-cmvs/src/adapter/display.rs#L308) |
| `opcode` | `0x2168` | Self::submit_ui | Handled | [315](../../crates/crysirror-cmvs/src/adapter/display.rs#L315) |
| `opcode` | `0x2169` | host.window_minimized | Handled | [326](../../crates/crysirror-cmvs/src/adapter/display.rs#L326) |
| `opcode` | `0x216c` | Local state/value operations | Handled | [331](../../crates/crysirror-cmvs/src/adapter/display.rs#L331) |
| `opcode` | `0x216d` | self.write | Handled | [332](../../crates/crysirror-cmvs/src/adapter/display.rs#L332) |
| `opcode` | `0x216e` | self.observe, Self::submit_ui | Handled | [344](../../crates/crysirror-cmvs/src/adapter/display.rs#L344) |
| `opcode` | `0x216f \| 0x22ca` | u32::from | Handled | [356](../../crates/crysirror-cmvs/src/adapter/display.rs#L356) |
| `opcode` | `0x22c8` | Local state/value operations | Handled | [357](../../crates/crysirror-cmvs/src/adapter/display.rs#L357) |
| `opcode` | `0x22c9` | Self::submit_ui, self.apply_mode | Handled | [358](../../crates/crysirror-cmvs/src/adapter/display.rs#L358) |
| `opcode` | `0x22d2` | self.write, u32::from | Handled | [375](../../crates/crysirror-cmvs/src/adapter/display.rs#L375) |
| `opcode` | `0x22d3 \| 0x22d4` | usize::from, self.write, u32::from | Handled | [385](../../crates/crysirror-cmvs/src/adapter/display.rs#L385) |
| `opcode` | `0x22e9` | self.write | Handled | [397](../../crates/crysirror-cmvs/src/adapter/display.rs#L397) |
| `opcode` | `0x22ea` | self.write | Handled | [407](../../crates/crysirror-cmvs/src/adapter/display.rs#L407) |
| `opcode` | `0x22ec` | self.apply_mode, u32::from | Handled | [417](../../crates/crysirror-cmvs/src/adapter/display.rs#L417) |
| `opcode` | `0x22ed if revision` | self.write, u32::from | Handled | [429](../../crates/crysirror-cmvs/src/adapter/display.rs#L429) |
| `opcode` | `0x22fc` | Local state/value operations | Handled | [439](../../crates/crysirror-cmvs/src/adapter/display.rs#L439) |
| `opcode` | `0x22fd` | self.write, self.apply_mode | Handled | [440](../../crates/crysirror-cmvs/src/adapter/display.rs#L440) |
| `opcode` | `0x23c5 if revision` | Local state/value operations | Handled | [452](../../crates/crysirror-cmvs/src/adapter/display.rs#L452) |
| `opcode` | `0x23c6 if revision` | self.write | Handled | [453](../../crates/crysirror-cmvs/src/adapter/display.rs#L453) |
| `opcode` | `0x23c7 if revision` | Local state/value operations | Handled | [462](../../crates/crysirror-cmvs/src/adapter/display.rs#L462) |
| `opcode` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [463](../../crates/crysirror-cmvs/src/adapter/display.rs#L463) |

### adapter/effect.rs / depth_band

```rust
fn depth_band(mode: u32) -> f32
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `(mode >> 24) & 0xf` | `1` | Local state/value operations | Handled | [620](../../crates/crysirror-cmvs/src/adapter/effect.rs#L620) |
| `(mode >> 24) & 0xf` | `2` | Local state/value operations | Handled | [621](../../crates/crysirror-cmvs/src/adapter/effect.rs#L621) |
| `(mode >> 24) & 0xf` | `3` | Local state/value operations | Handled | [622](../../crates/crysirror-cmvs/src/adapter/effect.rs#L622) |
| `(mode >> 24) & 0xf` | `4` | Local state/value operations | Handled | [623](../../crates/crysirror-cmvs/src/adapter/effect.rs#L623) |
| `(mode >> 24) & 0xf` | `5` | Local state/value operations | Handled | [624](../../crates/crysirror-cmvs/src/adapter/effect.rs#L624) |
| `(mode >> 24) & 0xf` | `_` | Local state/value operations | Handled | [625](../../crates/crysirror-cmvs/src/adapter/effect.rs#L625) |

### adapter/effect.rs / projected_size

```rust
fn projected_size(mode: u32, depth: f32) -> f32
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `mode >> 28` | `4` | Local state/value operations | Handled | [608](../../crates/crysirror-cmvs/src/adapter/effect.rs#L608) |
| `mode >> 28` | `5` | Local state/value operations | Handled | [609](../../crates/crysirror-cmvs/src/adapter/effect.rs#L609) |
| `mode >> 28` | `6` | Local state/value operations | Handled | [610](../../crates/crysirror-cmvs/src/adapter/effect.rs#L610) |
| `mode >> 28` | `7` | Local state/value operations | Handled | [611](../../crates/crysirror-cmvs/src/adapter/effect.rs#L611) |
| `mode >> 28` | `8` | Local state/value operations | Handled | [612](../../crates/crysirror-cmvs/src/adapter/effect.rs#L612) |
| `mode >> 28` | `_` | Local state/value operations | Handled | [613](../../crates/crysirror-cmvs/src/adapter/effect.rs#L613) |

### adapter/effect.rs / Effects::set

```rust
fn set(&mut self, index: usize, key: u32, value: u32) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let (Some(Some(Kind::Fall(fall))), 1) = (self.slots.get_mut(index), key)` | Local state/value operations | Handled | [379](../../crates/crysirror-cmvs/src/adapter/effect.rs#L379) |

### adapter/effect.rs / Fade::new

```rust
fn new(mode: u32, now: u64) -> Self
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `kind == 1` | Local state/value operations | Handled | [789](../../crates/crysirror-cmvs/src/adapter/effect.rs#L789) |

### adapter/effect.rs / Fade::step

```rust
fn step(&mut self, now: u64) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `(self.mode, progress)` | `(1, Some(at))` | Local state/value operations | Handled | [798](../../crates/crysirror-cmvs/src/adapter/effect.rs#L798) |
| `(self.mode, progress)` | `(1, None) if elapsed < self.delay` | Local state/value operations | Handled | [799](../../crates/crysirror-cmvs/src/adapter/effect.rs#L799) |
| `(self.mode, progress)` | `(1, None)` | Local state/value operations | Handled | [800](../../crates/crysirror-cmvs/src/adapter/effect.rs#L800) |
| `(self.mode, progress)` | `(2, Some(at))` | Local state/value operations | Handled | [801](../../crates/crysirror-cmvs/src/adapter/effect.rs#L801) |
| `(self.mode, progress)` | `(2, None) if elapsed < self.delay` | Local state/value operations | Handled | [802](../../crates/crysirror-cmvs/src/adapter/effect.rs#L802) |
| `(self.mode, progress)` | `(2, None)` | Local state/value operations | Handled | [803](../../crates/crysirror-cmvs/src/adapter/effect.rs#L803) |
| `(self.mode, progress)` | `_` | Local state/value operations | Handled | [804](../../crates/crysirror-cmvs/src/adapter/effect.rs#L804) |

### adapter/effect.rs / Streaks::advance

```rust
fn advance(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        objects: &mut Objects,
        rand: &mut Rand,
        slot: usize,
        now: u64,
    ) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `self.large && self.mode & 0x1000 != 0` | self.spawn | Handled | [875](../../crates/crysirror-cmvs/src/adapter/effect.rs#L875) |

### adapter/effect.rs / Streaks::spawn

```rust
fn spawn(&mut self, rand: &mut Rand, time: u64) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `self.mode & 0xf0` | `0x00` | Local state/value operations | Handled | [819](../../crates/crysirror-cmvs/src/adapter/effect.rs#L819) |
| `self.mode & 0xf0` | `0x10` | Local state/value operations | Handled | [820](../../crates/crysirror-cmvs/src/adapter/effect.rs#L820) |
| `self.mode & 0xf0` | `0x20` | Local state/value operations | Handled | [821](../../crates/crysirror-cmvs/src/adapter/effect.rs#L821) |
| `self.mode & 0xf0` | `0x30` | Local state/value operations | Handled | [822](../../crates/crysirror-cmvs/src/adapter/effect.rs#L822) |
| `self.mode & 0xf0` | `0x40` | Local state/value operations | Handled | [823](../../crates/crysirror-cmvs/src/adapter/effect.rs#L823) |
| `self.mode & 0xf0` | `0x50` | Local state/value operations | Handled | [824](../../crates/crysirror-cmvs/src/adapter/effect.rs#L824) |
| `self.mode & 0xf0` | `0x60` | Local state/value operations | Handled | [825](../../crates/crysirror-cmvs/src/adapter/effect.rs#L825) |
| `self.mode & 0xf0` | `0x70` | Local state/value operations | Handled | [826](../../crates/crysirror-cmvs/src/adapter/effect.rs#L826) |
| `self.mode & 0xf0` | `0x80` | Local state/value operations | Handled | [827](../../crates/crysirror-cmvs/src/adapter/effect.rs#L827) |
| `self.mode & 0xf0` | `0xa0` | Local state/value operations | Handled | [828](../../crates/crysirror-cmvs/src/adapter/effect.rs#L828) |
| `self.mode & 0xf0` | `_` | Local state/value operations | Handled | [829](../../crates/crysirror-cmvs/src/adapter/effect.rs#L829) |
| `self.mode & 0xf00` | `0x000` | Local state/value operations | Handled | [832](../../crates/crysirror-cmvs/src/adapter/effect.rs#L832) |
| `self.mode & 0xf00` | `0x100` | Local state/value operations | Handled | [833](../../crates/crysirror-cmvs/src/adapter/effect.rs#L833) |
| `self.mode & 0xf00` | `0x200` | Local state/value operations | Handled | [834](../../crates/crysirror-cmvs/src/adapter/effect.rs#L834) |
| `self.mode & 0xf00` | `0x300` | Local state/value operations | Handled | [835](../../crates/crysirror-cmvs/src/adapter/effect.rs#L835) |
| `self.mode & 0xf00` | `0x400` | Local state/value operations | Handled | [836](../../crates/crysirror-cmvs/src/adapter/effect.rs#L836) |
| `self.mode & 0xf00` | `0x500` | Local state/value operations | Handled | [837](../../crates/crysirror-cmvs/src/adapter/effect.rs#L837) |
| `self.mode & 0xf00` | `0x600` | Local state/value operations | Handled | [838](../../crates/crysirror-cmvs/src/adapter/effect.rs#L838) |
| `self.mode & 0xf00` | `_` | Local state/value operations | Handled | [839](../../crates/crysirror-cmvs/src/adapter/effect.rs#L839) |

### adapter/host.rs / HostState::command

```rust
fn command(
        &mut self,
        opcode: u16,
        vm: &mut CmvsVm,
        host: &mut dyn HostServices,
        client: ClientId,
    ) -> Result<Option<Host>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `kind; under opcode=0x20cb` | `1` | Local state/value operations | Handled | [74](../../crates/crysirror-cmvs/src/adapter/host.rs#L74) |
| `kind; under opcode=0x20cb` | `2` | Local state/value operations | Handled | [75](../../crates/crysirror-cmvs/src/adapter/host.rs#L75) |
| `kind; under opcode=0x20cb` | `3` | Local state/value operations | Handled | [76](../../crates/crysirror-cmvs/src/adapter/host.rs#L76) |
| `kind; under opcode=0x20cb` | `4` | Vec::with_capacity, usize::try_from | Handled | [78](../../crates/crysirror-cmvs/src/adapter/host.rs#L78) |
| `kind; under opcode=0x20cb` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [93](../../crates/crysirror-cmvs/src/adapter/host.rs#L93) |
| `opcode` | `0x201c` | host.open_document, encoding_rs::SHIFT_JIS.decode, host.append_log | Handled | [51](../../crates/crysirror-cmvs/src/adapter/host.rs#L51) |
| `opcode` | `0x20cb` | Vec::with_capacity, usize::try_from, u32::from | Handled | [70](../../crates/crysirror-cmvs/src/adapter/host.rs#L70) |
| `opcode` | `0x20cc` | host.warp_pointer | Handled | [98](../../crates/crysirror-cmvs/src/adapter/host.rs#L98) |
| `opcode` | `0x2099` | host.submit_engine, serde_json::to_vec, Host::LoadCursor | Handled | [107](../../crates/crysirror-cmvs/src/adapter/host.rs#L107) |
| `opcode` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [125](../../crates/crysirror-cmvs/src/adapter/host.rs#L125) |

### adapter/light.rs / Light::advance

```rust
fn advance(&mut self, delta: u32) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `tween.mode` | `1` | Local state/value operations | Handled | [74](../../crates/crysirror-cmvs/src/adapter/light.rs#L74) |
| `tween.mode` | `2` | Local state/value operations | Handled | [75](../../crates/crysirror-cmvs/src/adapter/light.rs#L75) |
| `tween.mode` | `_` | Local state/value operations | Handled | [76](../../crates/crysirror-cmvs/src/adapter/light.rs#L76) |

### adapter/light.rs / Light::command

```rust
fn command(&mut self, opcode: u16, vm: &mut CmvsVm) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `opcode` | `0x2260` | Local state/value operations | Handled | [100](../../crates/crysirror-cmvs/src/adapter/light.rs#L100) |
| `opcode` | `0x2261..=0x2264` | usize::from | Handled | [105](../../crates/crysirror-cmvs/src/adapter/light.rs#L105) |
| `opcode` | `0x2270` | self.finish | Handled | [110](../../crates/crysirror-cmvs/src/adapter/light.rs#L110) |
| `opcode` | `0x2272` | u32::from, self.active | Handled | [115](../../crates/crysirror-cmvs/src/adapter/light.rs#L115) |
| `opcode` | `0x2273` | self.start | Handled | [116](../../crates/crysirror-cmvs/src/adapter/light.rs#L116) |
| `opcode` | `0x2274` | self.start | Handled | [121](../../crates/crysirror-cmvs/src/adapter/light.rs#L121) |
| `opcode` | `0x2275` | self.start | Handled | [126](../../crates/crysirror-cmvs/src/adapter/light.rs#L126) |
| `opcode` | `_` | Local state/value operations | Handled | [132](../../crates/crysirror-cmvs/src/adapter/light.rs#L132) |

### adapter/menu.rs / apply_slots

```rust
fn apply_slots(items: &mut [Value], slots: &SaveMenu) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Some(id) = item["id"].as_u64().map(\|id\| id as u32)` | Value::Bool, Value::String, encoding_rs::SHIFT_JIS.decode | Handled | [154](../../crates/crysirror-cmvs/src/adapter/menu.rs#L154) |

### adapter/menu.rs / collect_ids

```rust
fn collect_ids(items: &[MenuTemplateItem], used: &mut BTreeSet<u32>) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Some(id) = item.id.filter(\|&id\| id != 0)` | Local state/value operations | Handled | [104](../../crates/crysirror-cmvs/src/adapter/menu.rs#L104) |

### adapter/menu.rs / items

```rust
fn items(native: &[MenuTemplateItem], ids: &mut impl Iterator<Item = u32>) -> Result<Vec<Value>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `item.id` | `Some(id) if id != 0` | Local state/value operations | Handled | [120](../../crates/crysirror-cmvs/src/adapter/menu.rs#L120) |
| `item.id` | `_` | Local state/value operations | Handled | [121](../../crates/crysirror-cmvs/src/adapter/menu.rs#L121) |

### adapter/message.rs / window_object

```rust
fn window_object(
    opcode: u16,
    host: &mut dyn HostServices,
    client: ClientId,
    vm: &CmvsVm,
    objects: &mut Objects,
    object: usize,
) -> Result<u16>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `opcode == 0x2178` | DrawProperty::Shown | Handled | [2178](../../crates/crysirror-cmvs/src/adapter/message.rs#L2178) |
| `opcode` | `0x217a` | DrawProperty::Shown | Handled | [2184](../../crates/crysirror-cmvs/src/adapter/message.rs#L2184) |
| `opcode` | `0x217c` | DrawProperty::LogicalSize | Handled | [2188](../../crates/crysirror-cmvs/src/adapter/message.rs#L2188) |
| `opcode` | `0x217f` | Local state/value operations | Handled | [2192](../../crates/crysirror-cmvs/src/adapter/message.rs#L2192) |
| `opcode` | `0x2180` | Local state/value operations | Handled | [2193](../../crates/crysirror-cmvs/src/adapter/message.rs#L2193) |
| `opcode` | `0x2182` | DrawProperty::Crop | Handled | [2194](../../crates/crysirror-cmvs/src/adapter/message.rs#L2194) |
| `opcode` | `0x2183` | DrawProperty::Position | Handled | [2198](../../crates/crysirror-cmvs/src/adapter/message.rs#L2198) |
| `opcode` | `0x2184` | DrawProperty::Priority | Handled | [2202](../../crates/crysirror-cmvs/src/adapter/message.rs#L2202) |
| `opcode` | `0x2185` | DrawProperty::Origin | Handled | [2206](../../crates/crysirror-cmvs/src/adapter/message.rs#L2206) |
| `opcode` | `0x2186` | DrawProperty::Opacity | Handled | [2210](../../crates/crysirror-cmvs/src/adapter/message.rs#L2210) |
| `opcode` | `_` | Local state/value operations | Handled | [2214](../../crates/crysirror-cmvs/src/adapter/message.rs#L2214) |

### adapter/message.rs / Messages::apply

```rust
fn apply(
        &mut self,
        opcode: u16,
        host: &mut dyn HostServices,
        client: ClientId,
        vm: &mut CmvsVm,
        objects: &mut Objects,
        canvas: [u32; 2],
    ) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(opcode, 0x2112 \| 0x2114..=0x2117 \| 0x2119 \| 0x211e..=0x211f)` | self.text_command | Handled | [549](../../crates/crysirror-cmvs/src/adapter/message.rs#L549) |
| `condition` | `matches!(opcode, 0x2112 \| 0x2114..=0x2117 \| 0x2119 \| 0x211e..=0x211f)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [549](../../crates/crysirror-cmvs/src/adapter/message.rs#L549) |
| `condition` | `matches!(opcode, 0x2140 \| 0x215d)` | self.default_face, Arc::from, DrawProperty::Shown, Text::new, WindowButton::default | Handled | [556](../../crates/crysirror-cmvs/src/adapter/message.rs#L556) |
| `condition` | `matches!(opcode, 0x2140 \| 0x215d)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [556](../../crates/crysirror-cmvs/src/adapter/message.rs#L556) |
| `condition` | `opcode == 0x215d` | Local state/value operations | Handled | [557](../../crates/crysirror-cmvs/src/adapter/message.rs#L557) |
| `condition` | `opcode == 0x2141` | Local state/value operations | Handled | [608](../../crates/crysirror-cmvs/src/adapter/message.rs#L608) |
| `condition` | `opcode == 0x218d` | Local state/value operations | Handled | [630](../../crates/crysirror-cmvs/src/adapter/message.rs#L630) |
| `condition` | `opcode == 0x2194` | std::array::from_fn, i32::from, crysirror_core::region_map::contains_rect, u32::from | Handled | [640](../../crates/crysirror-cmvs/src/adapter/message.rs#L640) |
| `condition; under opcode=0x2179 \| 0x217b` | `opcode == 0x217b` | u32::from | Handled | [724](../../crates/crysirror-cmvs/src/adapter/message.rs#L724) |
| `condition; under opcode=0x214c \| 0x214e` | `opcode == 0x214c` | Local state/value operations | Handled | [833](../../crates/crysirror-cmvs/src/adapter/message.rs#L833) |
| `condition; under opcode=0x214c \| 0x214e` | `opcode == 0x214c` | Local state/value operations | Handled | [839](../../crates/crysirror-cmvs/src/adapter/message.rs#L839) |
| `condition; under opcode=0x2150` | `mode == 1` | std::array::from_fn | Handled | [876](../../crates/crysirror-cmvs/src/adapter/message.rs#L876) |
| `mode as u16; under opcode=0x2151` | `0 \| 2` | Local state/value operations | Handled | [899](../../crates/crysirror-cmvs/src/adapter/message.rs#L899) |
| `mode as u16; under opcode=0x2151` | `1 \| 3` | Local state/value operations | Handled | [900](../../crates/crysirror-cmvs/src/adapter/message.rs#L900) |
| `mode as u16; under opcode=0x2151` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [901](../../crates/crysirror-cmvs/src/adapter/message.rs#L901) |
| `opcode; under opcode=0x2190..=0x2193 \| 0x2195..=0x2197` | `0x2190` | Local state/value operations | Handled | [934](../../crates/crysirror-cmvs/src/adapter/message.rs#L934) |
| `opcode; under opcode=0x2190..=0x2193 \| 0x2195..=0x2197` | `0x2191` | Local state/value operations | Handled | [943](../../crates/crysirror-cmvs/src/adapter/message.rs#L943) |
| `opcode; under opcode=0x2190..=0x2193 \| 0x2195..=0x2197` | `0x2195` | u32::from | Handled | [948](../../crates/crysirror-cmvs/src/adapter/message.rs#L948) |
| `opcode; under opcode=0x2190..=0x2193 \| 0x2195..=0x2197` | `0x2196` | DrawProperty::Crop | Handled | [952](../../crates/crysirror-cmvs/src/adapter/message.rs#L952) |
| `opcode; under opcode=0x2190..=0x2193 \| 0x2195..=0x2197` | `0x2197` | u32::from | Handled | [974](../../crates/crysirror-cmvs/src/adapter/message.rs#L974) |
| `opcode; under opcode=0x2190..=0x2193 \| 0x2195..=0x2197` | `_` | DrawProperty::Shown | Handled | [978](../../crates/crysirror-cmvs/src/adapter/message.rs#L978) |
| `condition; under opcode=_` | `matches!(opcode, 0x2178 \| 0x2180)` | Local state/value operations | Handled | [1000](../../crates/crysirror-cmvs/src/adapter/message.rs#L1000) |
| `condition; under opcode=_` | `matches!(opcode, 0x2178 \| 0x2180)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [1000](../../crates/crysirror-cmvs/src/adapter/message.rs#L1000) |
| `condition; under opcode=_` | `opcode == 0x2178 && child.is_none()` | Local state/value operations | Handled | [1003](../../crates/crysirror-cmvs/src/adapter/message.rs#L1003) |
| `opcode` | `0x20ec` | f32::from_bits, DrawProperty::Scale | Handled | [695](../../crates/crysirror-cmvs/src/adapter/message.rs#L695) |
| `opcode` | `0x2171` | Local state/value operations | Handled | [717](../../crates/crysirror-cmvs/src/adapter/message.rs#L717) |
| `opcode` | `0x2179 \| 0x217b` | u32::from | Handled | [721](../../crates/crysirror-cmvs/src/adapter/message.rs#L721) |
| `opcode` | `0x215f` | std::array::from_fn, u32::from | Handled | [742](../../crates/crysirror-cmvs/src/adapter/message.rs#L742) |
| `opcode` | `0x21cf..=0x21d2` | Local state/value operations | Handled | [759](../../crates/crysirror-cmvs/src/adapter/message.rs#L759) |
| `opcode` | `0x2142` | Local state/value operations | Handled | [763](../../crates/crysirror-cmvs/src/adapter/message.rs#L763) |
| `opcode` | `0x2143` | Local state/value operations | Handled | [768](../../crates/crysirror-cmvs/src/adapter/message.rs#L768) |
| `opcode` | `0x2144` | Local state/value operations | Handled | [775](../../crates/crysirror-cmvs/src/adapter/message.rs#L775) |
| `opcode` | `0x2145` | Local state/value operations | Handled | [784](../../crates/crysirror-cmvs/src/adapter/message.rs#L784) |
| `opcode` | `0x2146` | Local state/value operations | Handled | [790](../../crates/crysirror-cmvs/src/adapter/message.rs#L790) |
| `opcode` | `0x2147` | Local state/value operations | Handled | [799](../../crates/crysirror-cmvs/src/adapter/message.rs#L799) |
| `opcode` | `0x2148` | Local state/value operations | Handled | [804](../../crates/crysirror-cmvs/src/adapter/message.rs#L804) |
| `opcode` | `0x2149` | Local state/value operations | Handled | [809](../../crates/crysirror-cmvs/src/adapter/message.rs#L809) |
| `opcode` | `0x214a` | Local state/value operations | Handled | [816](../../crates/crysirror-cmvs/src/adapter/message.rs#L816) |
| `opcode` | `0x214c \| 0x214e` | DrawProperty::Shown | Handled | [830](../../crates/crysirror-cmvs/src/adapter/message.rs#L830) |
| `opcode` | `0x214d` | u32::from | Handled | [866](../../crates/crysirror-cmvs/src/adapter/message.rs#L866) |
| `opcode` | `0x214f` | Local state/value operations | Handled | [867](../../crates/crysirror-cmvs/src/adapter/message.rs#L867) |
| `opcode` | `0x2150` | std::array::from_fn, DrawProperty::Position | Open: unsupported CMVS message move mode {mode} | [872](../../crates/crysirror-cmvs/src/adapter/message.rs#L872) |
| `opcode` | `0x2151` | Local state/value operations | Handled | [895](../../crates/crysirror-cmvs/src/adapter/message.rs#L895) |
| `opcode` | `0x2155` | Local state/value operations | Handled | [905](../../crates/crysirror-cmvs/src/adapter/message.rs#L905) |
| `opcode` | `0x215a` | Local state/value operations | Handled | [910](../../crates/crysirror-cmvs/src/adapter/message.rs#L910) |
| `opcode` | `0x215b` | usize::from, DrawProperty::Position | Handled | [915](../../crates/crysirror-cmvs/src/adapter/message.rs#L915) |
| `opcode` | `0x2190..=0x2193 \| 0x2195..=0x2197` | u32::from, DrawProperty::Crop, DrawProperty::Shown | Handled | [927](../../crates/crysirror-cmvs/src/adapter/message.rs#L927) |
| `opcode` | `_` | Local state/value operations | Handled | [998](../../crates/crysirror-cmvs/src/adapter/message.rs#L998) |

### adapter/message.rs / Messages::configure

```rust
fn configure(&mut self, bytes: &[u8]) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `key == "FONT"` | encoding_rs::SHIFT_JIS.encode | Handled | [324](../../crates/crysirror-cmvs/src/adapter/message.rs#L324) |
| `key.as_str()` | `"TEXT_WAIT"` | Local state/value operations | Handled | [336](../../crates/crysirror-cmvs/src/adapter/message.rs#L336) |
| `key.as_str()` | `"TEXT_GANG"` | Local state/value operations | Handled | [337](../../crates/crysirror-cmvs/src/adapter/message.rs#L337) |
| `key.as_str()` | `"TEXT_BOLD"` | Local state/value operations | Handled | [338](../../crates/crysirror-cmvs/src/adapter/message.rs#L338) |
| `key.as_str()` | `"EFFECT_TYPE"` | Local state/value operations | Handled | [339](../../crates/crysirror-cmvs/src/adapter/message.rs#L339) |
| `key.as_str()` | `"EFFECT_TIMER"` | Local state/value operations | Handled | [340](../../crates/crysirror-cmvs/src/adapter/message.rs#L340) |
| `key.as_str()` | `"READ_SKIP"` | Local state/value operations | Handled | [341](../../crates/crysirror-cmvs/src/adapter/message.rs#L341) |
| `key.as_str()` | `"VOICE_CUT_MODE"` | Local state/value operations | Handled | [342](../../crates/crysirror-cmvs/src/adapter/message.rs#L342) |
| `key.as_str()` | `"AUTO_MODE_1"` | Local state/value operations | Handled | [343](../../crates/crysirror-cmvs/src/adapter/message.rs#L343) |
| `key.as_str()` | `"AUTO_MODE_2"` | Local state/value operations | Handled | [344](../../crates/crysirror-cmvs/src/adapter/message.rs#L344) |
| `key.as_str()` | `"AUTO_MODE_3"` | Local state/value operations | Handled | [345](../../crates/crysirror-cmvs/src/adapter/message.rs#L345) |
| `key.as_str()` | `"AUTO_MODE_4"` | Local state/value operations | Handled | [346](../../crates/crysirror-cmvs/src/adapter/message.rs#L346) |
| `key.as_str()` | `"AUTO_VOICE_SYNC"` | Local state/value operations | Handled | [347](../../crates/crysirror-cmvs/src/adapter/message.rs#L347) |
| `key.as_str()` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [348](../../crates/crysirror-cmvs/src/adapter/message.rs#L348) |

### adapter/message.rs / Messages::field_command

```rust
fn field_command(
        &mut self,
        opcode: u16,
        host: &mut dyn HostServices,
        client: ClientId,
        vm: &mut CmvsVm,
        objects: &Objects,
        canvas: [u32; 2],
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `opcode` | `0x239b \| 0x23a0` | Local state/value operations | Handled | [1542](../../crates/crysirror-cmvs/src/adapter/message.rs#L1542) |
| `opcode` | `0x2390 \| 0x239d \| 0x23bb` | Local state/value operations | Handled | [1543](../../crates/crysirror-cmvs/src/adapter/message.rs#L1543) |
| `opcode` | `0x20e4 \| 0x239e \| 0x23ba` | Local state/value operations | Handled | [1544](../../crates/crysirror-cmvs/src/adapter/message.rs#L1544) |
| `opcode` | `0x239c \| 0x23b8` | Local state/value operations | Handled | [1545](../../crates/crysirror-cmvs/src/adapter/message.rs#L1545) |
| `opcode` | `0x2396 \| 0x239a` | Local state/value operations | Handled | [1546](../../crates/crysirror-cmvs/src/adapter/message.rs#L1546) |
| `opcode` | `_` | Local state/value operations | Handled | [1547](../../crates/crysirror-cmvs/src/adapter/message.rs#L1547) |
| `condition` | `opcode == 0x20e4` | Local state/value operations | Handled | [1550](../../crates/crysirror-cmvs/src/adapter/message.rs#L1550) |
| `condition` | `opcode == 0x20e4` | Local state/value operations | Handled | [1551](../../crates/crysirror-cmvs/src/adapter/message.rs#L1551) |
| `condition` | `matches!(opcode, 0x20e4 \| 0x23ba)` | host.text_measurement, field::wrap_words | Handled | [1556](../../crates/crysirror-cmvs/src/adapter/message.rs#L1556) |
| `condition` | `matches!(opcode, 0x20e4 \| 0x23ba)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [1556](../../crates/crysirror-cmvs/src/adapter/message.rs#L1556) |
| `opcode` | `0x2390` | field::count_columns | Handled | [1568](../../crates/crysirror-cmvs/src/adapter/message.rs#L1568) |
| `opcode` | `0x23bb` | host.text_measurement, measure::measure_pixels | Handled | [1573](../../crates/crysirror-cmvs/src/adapter/message.rs#L1573) |
| `opcode` | `0x2396` | Local state/value operations | Handled | [1581](../../crates/crysirror-cmvs/src/adapter/message.rs#L1581) |
| `opcode` | `0x239a` | Local state/value operations | Handled | [1587](../../crates/crysirror-cmvs/src/adapter/message.rs#L1587) |
| `opcode` | `0x239b` | u32::from | Handled | [1592](../../crates/crysirror-cmvs/src/adapter/message.rs#L1592) |
| `opcode` | `0x239c` | Local state/value operations | Handled | [1595](../../crates/crysirror-cmvs/src/adapter/message.rs#L1595) |
| `opcode` | `0x239f` | Local state/value operations | Handled | [1600](../../crates/crysirror-cmvs/src/adapter/message.rs#L1600) |
| `opcode` | `0x239d` | Local state/value operations | Handled | [1605](../../crates/crysirror-cmvs/src/adapter/message.rs#L1605) |
| `opcode` | `0x239e` | Local state/value operations | Handled | [1610](../../crates/crysirror-cmvs/src/adapter/message.rs#L1610) |
| `opcode` | `0x23a0` | Local state/value operations | Handled | [1616](../../crates/crysirror-cmvs/src/adapter/message.rs#L1616) |
| `opcode` | `0x23b8` | Local state/value operations | Handled | [1620](../../crates/crysirror-cmvs/src/adapter/message.rs#L1620) |
| `opcode` | `_` | Local state/value operations | Handled | [1627](../../crates/crysirror-cmvs/src/adapter/message.rs#L1627) |

### adapter/message.rs / Messages::field_position

```rust
fn field_position(
        &mut self,
        opcode: u16,
        host: &mut dyn HostServices,
        client: ClientId,
        vm: &mut CmvsVm,
        objects: &Objects,
        canvas: [u32; 2],
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `opcode == 0x2398` | Local state/value operations | Handled | [1674](../../crates/crysirror-cmvs/src/adapter/message.rs#L1674) |
| `condition` | `opcode == 0x2398` | Local state/value operations | Handled | [1679](../../crates/crysirror-cmvs/src/adapter/message.rs#L1679) |
| `condition` | `opcode == 0x2384` | Local state/value operations | Handled | [1686](../../crates/crysirror-cmvs/src/adapter/message.rs#L1686) |

### adapter/message.rs / Messages::image_font_command

```rust
fn image_font_command(&mut self, opcode: u16, vm: &mut CmvsVm) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `opcode == 0x23c1` | Local state/value operations | Handled | [1486](../../crates/crysirror-cmvs/src/adapter/message.rs#L1486) |

### adapter/message.rs / Messages::object_command

```rust
fn object_command(
        &mut self,
        opcode: u16,
        host: &mut dyn HostServices,
        client: ClientId,
        vm: &mut CmvsVm,
        objects: &Objects,
        canvas: [u32; 2],
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `opcode` | `0x2101 \| 0x2104 \| 0x2109 \| 0x2111` | Local state/value operations | Handled | [1707](../../crates/crysirror-cmvs/src/adapter/message.rs#L1707) |
| `opcode` | `0x2100 \| 0x2102 \| 0x2107 \| 0x210c \| 0x210d` | Local state/value operations | Handled | [1708](../../crates/crysirror-cmvs/src/adapter/message.rs#L1708) |
| `opcode` | `0x2103 \| 0x211c` | Local state/value operations | Handled | [1709](../../crates/crysirror-cmvs/src/adapter/message.rs#L1709) |
| `opcode` | `0x2105` | Local state/value operations | Handled | [1710](../../crates/crysirror-cmvs/src/adapter/message.rs#L1710) |
| `opcode` | `_` | Local state/value operations | Handled | [1711](../../crates/crysirror-cmvs/src/adapter/message.rs#L1711) |
| `condition` | `matches!(opcode, 0x2100 \| 0x211c)` | self.default_face, Arc::from, Text::object | Handled | [1722](../../crates/crysirror-cmvs/src/adapter/message.rs#L1722) |
| `condition` | `matches!(opcode, 0x2100 \| 0x211c)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [1722](../../crates/crysirror-cmvs/src/adapter/message.rs#L1722) |
| `condition` | `opcode == 0x211c` | Local state/value operations | Handled | [1724](../../crates/crysirror-cmvs/src/adapter/message.rs#L1724) |
| `condition` | `opcode == 0x2101` | Local state/value operations | Handled | [1744](../../crates/crysirror-cmvs/src/adapter/message.rs#L1744) |
| `opcode` | `0x2102` | Local state/value operations | Handled | [1758](../../crates/crysirror-cmvs/src/adapter/message.rs#L1758) |
| `opcode` | `0x2103` | Local state/value operations | Handled | [1763](../../crates/crysirror-cmvs/src/adapter/message.rs#L1763) |
| `opcode` | `0x2104` | Local state/value operations | Handled | [1768](../../crates/crysirror-cmvs/src/adapter/message.rs#L1768) |
| `opcode` | `0x2105` | Local state/value operations | Handled | [1773](../../crates/crysirror-cmvs/src/adapter/message.rs#L1773) |
| `opcode` | `0x2107` | Local state/value operations | Handled | [1785](../../crates/crysirror-cmvs/src/adapter/message.rs#L1785) |
| `opcode` | `0x2108` | Local state/value operations | Handled | [1793](../../crates/crysirror-cmvs/src/adapter/message.rs#L1793) |
| `opcode` | `0x2109` | Local state/value operations | Handled | [1797](../../crates/crysirror-cmvs/src/adapter/message.rs#L1797) |
| `opcode` | `0x210a \| 0x210b` | host.text_measurement, Arc::make_mut | Handled | [1800](../../crates/crysirror-cmvs/src/adapter/message.rs#L1800) |
| `opcode` | `0x210c` | Local state/value operations | Handled | [1816](../../crates/crysirror-cmvs/src/adapter/message.rs#L1816) |
| `opcode` | `0x210d` | Local state/value operations | Handled | [1820](../../crates/crysirror-cmvs/src/adapter/message.rs#L1820) |
| `opcode` | `0x210e` | Local state/value operations | Handled | [1824](../../crates/crysirror-cmvs/src/adapter/message.rs#L1824) |
| `opcode` | `0x210f` | Local state/value operations | Handled | [1828](../../crates/crysirror-cmvs/src/adapter/message.rs#L1828) |
| `opcode` | `0x2110` | Local state/value operations | Handled | [1832](../../crates/crysirror-cmvs/src/adapter/message.rs#L1832) |
| `opcode` | `0x2111` | Local state/value operations | Handled | [1837](../../crates/crysirror-cmvs/src/adapter/message.rs#L1837) |
| `opcode` | `0x2113` | Local state/value operations | Handled | [1843](../../crates/crysirror-cmvs/src/adapter/message.rs#L1843) |
| `opcode` | `0x2118` | Local state/value operations | Handled | [1847](../../crates/crysirror-cmvs/src/adapter/message.rs#L1847) |
| `opcode` | `0x211a` | Local state/value operations | Handled | [1851](../../crates/crysirror-cmvs/src/adapter/message.rs#L1851) |
| `opcode` | `0x20e7` | f32::from_bits | Handled | [1855](../../crates/crysirror-cmvs/src/adapter/message.rs#L1855) |
| `opcode` | `0x211d` | host.text_measurement, measure::measure_pixels | Handled | [1861](../../crates/crysirror-cmvs/src/adapter/message.rs#L1861) |
| `opcode` | `_` | Local state/value operations | Handled | [1870](../../crates/crysirror-cmvs/src/adapter/message.rs#L1870) |

### adapter/message.rs / Messages::output

```rust
fn output(
        &mut self,
        opcode: u16,
        host: &mut dyn HostServices,
        client: ClientId,
        vm: &mut CmvsVm,
        objects: &mut Objects,
        canvas: [u32; 2],
        delta: u32,
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(opcode, 0x2152 \| 0x2153 \| 0x215c)` | Local state/value operations | Handled | [1037](../../crates/crysirror-cmvs/src/adapter/message.rs#L1037) |
| `condition` | `matches!(opcode, 0x2152 \| 0x2153 \| 0x215c)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [1037](../../crates/crysirror-cmvs/src/adapter/message.rs#L1037) |
| `condition` | `matches!(opcode, 0x2153 \| 0x2158)` | self.settings.wait_elapsed, i32::from, DrawProperty::Position, DrawProperty::Shown | Handled | [1044](../../crates/crysirror-cmvs/src/adapter/message.rs#L1044) |
| `condition` | `matches!(opcode, 0x2153 \| 0x2158)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [1044](../../crates/crysirror-cmvs/src/adapter/message.rs#L1044) |
| `condition` | `matches!(opcode, 0x2152 \| 0x215c)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [1149](../../crates/crysirror-cmvs/src/adapter/message.rs#L1149) |
| `condition` | `matches!(opcode, 0x2152 \| 0x215c)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [1150](../../crates/crysirror-cmvs/src/adapter/message.rs#L1150) |

### adapter/message.rs / Messages::text_command

```rust
fn text_command(&mut self, opcode: u16, vm: &mut CmvsVm) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `opcode` | `0x2112` | output::ruby_layout | Handled | [1195](../../crates/crysirror-cmvs/src/adapter/message.rs#L1195) |
| `opcode` | `0x2119` | output::encoded_length | Handled | [1206](../../crates/crysirror-cmvs/src/adapter/message.rs#L1206) |
| `opcode` | `0x2114` | Local state/value operations | Handled | [1213](../../crates/crysirror-cmvs/src/adapter/message.rs#L1213) |
| `opcode` | `0x2115` | Local state/value operations | Handled | [1217](../../crates/crysirror-cmvs/src/adapter/message.rs#L1217) |
| `opcode` | `0x2116` | Local state/value operations | Handled | [1221](../../crates/crysirror-cmvs/src/adapter/message.rs#L1221) |
| `opcode` | `0x2117` | Local state/value operations | Handled | [1225](../../crates/crysirror-cmvs/src/adapter/message.rs#L1225) |
| `opcode` | `0x211e` | Local state/value operations | Handled | [1229](../../crates/crysirror-cmvs/src/adapter/message.rs#L1229) |
| `opcode` | `_` | measure::measure | Handled | [1240](../../crates/crysirror-cmvs/src/adapter/message.rs#L1240) |

### adapter/message.rs / Messages::text_setting

```rust
fn text_setting(&mut self, vm: &mut CmvsVm, opcode: u16) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `opcode == 0x22de` | self.default_face | Handled | [419](../../crates/crysirror-cmvs/src/adapter/message.rs#L419) |
| `condition` | `mode == 0` | self.default_face | Handled | [420](../../crates/crysirror-cmvs/src/adapter/message.rs#L420) |
| `condition` | `mode != 0 \|\| (0x22e1..=0x22e3).contains(&opcode)` | Local state/value operations | Handled | [433](../../crates/crysirror-cmvs/src/adapter/message.rs#L433) |
| `condition` | `matches!(opcode, 0x22e0 \| 0x22e4 \| 0x22e6)` | u32::from | Handled | [438](../../crates/crysirror-cmvs/src/adapter/message.rs#L438) |
| `condition` | `matches!(opcode, 0x22e0 \| 0x22e4 \| 0x22e6)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [438](../../crates/crysirror-cmvs/src/adapter/message.rs#L438) |
| `opcode` | `0x22e0` | Local state/value operations | Handled | [440](../../crates/crysirror-cmvs/src/adapter/message.rs#L440) |
| `opcode` | `0x22e4` | Local state/value operations | Handled | [441](../../crates/crysirror-cmvs/src/adapter/message.rs#L441) |
| `opcode` | `_` | Local state/value operations | Handled | [442](../../crates/crysirror-cmvs/src/adapter/message.rs#L442) |
| `condition` | `mode != 0` | Local state/value operations | Handled | [445](../../crates/crysirror-cmvs/src/adapter/message.rs#L445) |
| `condition` | `opcode == 0x22e0` | Local state/value operations | Handled | [447](../../crates/crysirror-cmvs/src/adapter/message.rs#L447) |
| `opcode` | `0x22df` | Local state/value operations | Handled | [461](../../crates/crysirror-cmvs/src/adapter/message.rs#L461) |
| `opcode` | `0x22e1` | Local state/value operations | Handled | [465](../../crates/crysirror-cmvs/src/adapter/message.rs#L465) |
| `opcode` | `0x22e2` | Local state/value operations | Handled | [469](../../crates/crysirror-cmvs/src/adapter/message.rs#L469) |
| `opcode` | `0x22e3` | Local state/value operations | Handled | [473](../../crates/crysirror-cmvs/src/adapter/message.rs#L473) |
| `opcode` | `0x22e7` | Local state/value operations | Handled | [477](../../crates/crysirror-cmvs/src/adapter/message.rs#L477) |
| `opcode` | `0x22e8` | Local state/value operations | Handled | [478](../../crates/crysirror-cmvs/src/adapter/message.rs#L478) |
| `opcode` | `0x22f8` | Local state/value operations | Handled | [479](../../crates/crysirror-cmvs/src/adapter/message.rs#L479) |
| `opcode` | `_` | Local state/value operations | Handled | [480](../../crates/crysirror-cmvs/src/adapter/message.rs#L480) |
| `condition` | `mode != 0` | Local state/value operations | Handled | [483](../../crates/crysirror-cmvs/src/adapter/message.rs#L483) |

### adapter/message.rs / Messages::window_text

```rust
fn window_text(
        &mut self,
        opcode: u16,
        host: &mut dyn HostServices,
        client: ClientId,
        vm: &mut CmvsVm,
        objects: &Objects,
        canvas: [u32; 2],
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `opcode` | `0x2154` | Local state/value operations | Handled | [1419](../../crates/crysirror-cmvs/src/adapter/message.rs#L1419) |
| `opcode` | `0x2159` | Local state/value operations | Handled | [1420](../../crates/crysirror-cmvs/src/adapter/message.rs#L1420) |
| `opcode` | `_` | Local state/value operations | Handled | [1421](../../crates/crysirror-cmvs/src/adapter/message.rs#L1421) |
| `condition` | `opcode == 0x215e` | Local state/value operations | Handled | [1429](../../crates/crysirror-cmvs/src/adapter/message.rs#L1429) |
| `condition; under opcode=0x2154 \| 0x2159` | `opcode == 0x2154` | Local state/value operations | Handled | [1446](../../crates/crysirror-cmvs/src/adapter/message.rs#L1446) |
| `condition; under opcode=0x2154 \| 0x2159` | `opcode == 0x2159` | u32::from | Handled | [1461](../../crates/crysirror-cmvs/src/adapter/message.rs#L1461) |
| `opcode` | `0x2154 \| 0x2159` | Arc::make_mut, u32::from | Handled | [1445](../../crates/crysirror-cmvs/src/adapter/message.rs#L1445) |
| `opcode` | `_` | Local state/value operations | Handled | [1465](../../crates/crysirror-cmvs/src/adapter/message.rs#L1465) |

### adapter/message.rs / Window::animation

```rust
fn animation(
        &mut self,
        opcode: u16,
        host: &mut dyn HostServices,
        client: ClientId,
        vm: &CmvsVm,
        objects: &mut Objects,
        object: usize,
    ) -> Result<u16>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `opcode` | `0x21cf` | self.animations.insert, Animation::default | Handled | [2122](../../crates/crysirror-cmvs/src/adapter/message.rs#L2122) |
| `opcode` | `0x21d0` | self.animations.get_mut | Handled | [2126](../../crates/crysirror-cmvs/src/adapter/message.rs#L2126) |
| `opcode` | `0x21d1` | self.animations.get_mut | Handled | [2144](../../crates/crysirror-cmvs/src/adapter/message.rs#L2144) |
| `opcode` | `_` | self.animations.remove | Handled | [2157](../../crates/crysirror-cmvs/src/adapter/message.rs#L2157) |

### adapter/message/field.rs / Field::advance

```rust
fn advance(&mut self, delta: u32) -> Option<[i32; 2]>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `motion.mode == 1` | Local state/value operations | Handled | [63](../../crates/crysirror-cmvs/src/adapter/message/field.rs#L63) |

### adapter/message/measure.rs / line_break_kind

```rust
fn line_break_kind(text: &Text, code: u16) -> Result<u8, &'static str>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `bytes .as_chunks::<2>() .0 .iter() .any(\|b\| u16::from_be_bytes([b[0], b[1]]) == code)` | Local state/value operations | Handled | [15](../../crates/crysirror-cmvs/src/adapter/message/measure.rs#L15) |

### adapter/message/measure.rs / Layout<'_>::ruby_wrap

```rust
fn ruby_wrap(&mut self) -> Result<(), &'static str>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `kind == 1 \|\| (kind == 2 && self.previous == 0)` | self.overflow.wrapping_add | Handled | [127](../../crates/crysirror-cmvs/src/adapter/message/measure.rs#L127) |

### adapter/message/measure.rs / Layout<'_>::wide_wrap

```rust
fn wide_wrap(&mut self, code: u16) -> Result<(), &'static str>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `kind` | `1` | Local state/value operations | Handled | [91](../../crates/crysirror-cmvs/src/adapter/message/measure.rs#L91) |
| `kind` | `2` | Local state/value operations | Handled | [92](../../crates/crysirror-cmvs/src/adapter/message/measure.rs#L92) |
| `kind` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [93](../../crates/crysirror-cmvs/src/adapter/message/measure.rs#L93) |

### adapter/message/output.rs / append

```rust
fn append(
    text: &mut Text,
    bytes: &[u8],
    skip: bool,
    settings: OutputSettings,
    metrics: Option<&dyn TextMeasurement>,
) -> Result<(), &'static str>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `code == 0x814a && metrics.is_none()` | Local state/value operations | Handled | [648](../../crates/crysirror-cmvs/src/adapter/message/output.rs#L648) |
| `condition` | `code == 0x814a` | Local state/value operations | Handled | [657](../../crates/crysirror-cmvs/src/adapter/message/output.rs#L657) |

### adapter/message/output.rs / decode

```rust
fn decode(code: u16) -> Result<char, &'static str>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `code < 0x100 \|\| data[0] == 0xff` | Local state/value operations | Handled | [300](../../crates/crysirror-cmvs/src/adapter/message/output.rs#L300) |

### adapter/message/output.rs / wraps

```rust
fn wraps(text: &mut Text, code: u16, x: u32, limit: u32) -> Result<bool, &'static str>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `line_break_kind(text, code)?; under text.break_configured=true` | `1` | Local state/value operations | Handled | [340](../../crates/crysirror-cmvs/src/adapter/message/output.rs#L340) |
| `line_break_kind(text, code)?; under text.break_configured=true` | `2` | u32::from | Handled | [341](../../crates/crysirror-cmvs/src/adapter/message/output.rs#L341) |
| `line_break_kind(text, code)?; under text.break_configured=true` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [342](../../crates/crysirror-cmvs/src/adapter/message/output.rs#L342) |

### adapter/movie.rs / ScenePlayer::advance

```rust
fn advance(&mut self, now: u64, audio: Option<u64>) -> Option<u32>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `self.mode` | `0` | self.steps, self.pending.take | Handled | [354](../../crates/crysirror-cmvs/src/adapter/movie.rs#L354) |
| `self.mode` | `10` | self.steps | Handled | [378](../../crates/crysirror-cmvs/src/adapter/movie.rs#L378) |
| `self.mode` | `15` | u64::from | Handled | [393](../../crates/crysirror-cmvs/src/adapter/movie.rs#L393) |
| `self.mode` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [400](../../crates/crysirror-cmvs/src/adapter/movie.rs#L400) |

### adapter/quake.rs / command

```rust
fn command(state: &mut Option<Quake>, vm: &mut CmvsVm, opcode: u16) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `opcode` | `0x220c` | u32::from | Handled | [116](../../crates/crysirror-cmvs/src/adapter/quake.rs#L116) |
| `opcode` | `0x2225` | Local state/value operations | Open: CMVS unsupported quake waveform or random direction | [121](../../crates/crysirror-cmvs/src/adapter/quake.rs#L121) |
| `opcode` | `0x2226` | Local state/value operations | Handled | [140](../../crates/crysirror-cmvs/src/adapter/quake.rs#L140) |
| `opcode` | `0x2227` | Local state/value operations | Handled | [147](../../crates/crysirror-cmvs/src/adapter/quake.rs#L147) |
| `opcode` | `0x2228` | Local state/value operations | Handled | [154](../../crates/crysirror-cmvs/src/adapter/quake.rs#L154) |
| `opcode` | `0x2229` | Local state/value operations | Handled | [163](../../crates/crysirror-cmvs/src/adapter/quake.rs#L163) |
| `opcode` | `_` | Local state/value operations | Handled | [167](../../crates/crysirror-cmvs/src/adapter/quake.rs#L167) |

### adapter/quake.rs / Quake::advance

```rust
fn advance(&mut self, delta: u32) -> bool
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `self.mode & 0xf00 == 0x200` | self.damped | Handled | [48](../../crates/crysirror-cmvs/src/adapter/quake.rs#L48) |
| `self.mode & 0xf0` | `0x00` | Local state/value operations | Handled | [57](../../crates/crysirror-cmvs/src/adapter/quake.rs#L57) |
| `self.mode & 0xf0` | `0x10` | Local state/value operations | Handled | [58](../../crates/crysirror-cmvs/src/adapter/quake.rs#L58) |
| `self.mode & 0xf0` | `0x30` | Local state/value operations | Handled | [59](../../crates/crysirror-cmvs/src/adapter/quake.rs#L59) |
| `self.mode & 0xf0` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [69](../../crates/crysirror-cmvs/src/adapter/quake.rs#L69) |

### adapter/quake.rs / Quake::damped

```rust
fn damped(&mut self) -> bool
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `self.mode & 0xf0` | `0x00` | Local state/value operations | Handled | [106](../../crates/crysirror-cmvs/src/adapter/quake.rs#L106) |
| `self.mode & 0xf0` | `0x10` | Local state/value operations | Handled | [107](../../crates/crysirror-cmvs/src/adapter/quake.rs#L107) |
| `self.mode & 0xf0` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [108](../../crates/crysirror-cmvs/src/adapter/quake.rs#L108) |

### adapter/quake.rs / Quake::offset

```rust
fn offset(&self, ui: bool) -> [i32; 2]
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `self.paused \|\| self.mode & 0xf == excluded` | Local state/value operations | Handled | [35](../../crates/crysirror-cmvs/src/adapter/quake.rs#L35) |

### adapter/raster.rs / Raster::advance

```rust
fn advance(&mut self, delta: u32) -> bool
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `self.mode == 1` | self.phase.wrapping_add | Handled | [110](../../crates/crysirror-cmvs/src/adapter/raster.rs#L110) |

### adapter/raster.rs / Raster::rows

```rust
fn rows(&self, parent: Parent) -> Vec<Row>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `self.mode` | `1` | self.word, self.angle, f64::from, i32::from | Handled | [65](../../crates/crysirror-cmvs/src/adapter/raster.rs#L65) |
| `self.mode` | `16` | self.word, Vec::new | Handled | [85](../../crates/crysirror-cmvs/src/adapter/raster.rs#L85) |
| `self.mode` | `_` | Vec::new | Handled | [104](../../crates/crysirror-cmvs/src/adapter/raster.rs#L104) |

### adapter/retained.rs / aliased_command

```rust
fn aliased_command(opcode: u16) -> Option<(u16, AliasTarget)>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `opcode` | `0x2378` | Local state/value operations | Handled | [19](../../crates/crysirror-cmvs/src/adapter/retained.rs#L19) |
| `opcode` | `0x2379` | Local state/value operations | Handled | [20](../../crates/crysirror-cmvs/src/adapter/retained.rs#L20) |
| `opcode` | `0x237a` | Local state/value operations | Handled | [21](../../crates/crysirror-cmvs/src/adapter/retained.rs#L21) |
| `opcode` | `0x2370` | Local state/value operations | Handled | [22](../../crates/crysirror-cmvs/src/adapter/retained.rs#L22) |
| `opcode` | `0x2371` | Local state/value operations | Handled | [23](../../crates/crysirror-cmvs/src/adapter/retained.rs#L23) |
| `opcode` | `0x2372` | Local state/value operations | Handled | [24](../../crates/crysirror-cmvs/src/adapter/retained.rs#L24) |
| `opcode` | `0x2373` | Local state/value operations | Handled | [25](../../crates/crysirror-cmvs/src/adapter/retained.rs#L25) |
| `opcode` | `0x2374` | Local state/value operations | Handled | [26](../../crates/crysirror-cmvs/src/adapter/retained.rs#L26) |
| `opcode` | `0x2375` | Local state/value operations | Handled | [27](../../crates/crysirror-cmvs/src/adapter/retained.rs#L27) |
| `opcode` | `0x2376` | Local state/value operations | Handled | [28](../../crates/crysirror-cmvs/src/adapter/retained.rs#L28) |
| `opcode` | `0x2377` | Local state/value operations | Handled | [29](../../crates/crysirror-cmvs/src/adapter/retained.rs#L29) |
| `opcode` | `0x237b` | Local state/value operations | Handled | [30](../../crates/crysirror-cmvs/src/adapter/retained.rs#L30) |
| `opcode` | `0x2380 \| 0x2381` | Local state/value operations | Handled | [31](../../crates/crysirror-cmvs/src/adapter/retained.rs#L31) |
| `opcode` | `0x2382` | Local state/value operations | Handled | [32](../../crates/crysirror-cmvs/src/adapter/retained.rs#L32) |
| `opcode` | `0x2383` | Local state/value operations | Handled | [33](../../crates/crysirror-cmvs/src/adapter/retained.rs#L33) |
| `opcode` | `0x2385` | Local state/value operations | Handled | [34](../../crates/crysirror-cmvs/src/adapter/retained.rs#L34) |
| `opcode` | `0x2386` | Local state/value operations | Handled | [35](../../crates/crysirror-cmvs/src/adapter/retained.rs#L35) |
| `opcode` | `0x2387` | Local state/value operations | Handled | [36](../../crates/crysirror-cmvs/src/adapter/retained.rs#L36) |
| `opcode` | `0x2388` | Local state/value operations | Handled | [37](../../crates/crysirror-cmvs/src/adapter/retained.rs#L37) |
| `opcode` | `0x2389` | Local state/value operations | Handled | [38](../../crates/crysirror-cmvs/src/adapter/retained.rs#L38) |
| `opcode` | `0x238a` | Local state/value operations | Handled | [39](../../crates/crysirror-cmvs/src/adapter/retained.rs#L39) |
| `opcode` | `0x238b` | Local state/value operations | Handled | [40](../../crates/crysirror-cmvs/src/adapter/retained.rs#L40) |
| `opcode` | `0x238c` | Local state/value operations | Handled | [41](../../crates/crysirror-cmvs/src/adapter/retained.rs#L41) |
| `opcode` | `0x238e` | Local state/value operations | Handled | [42](../../crates/crysirror-cmvs/src/adapter/retained.rs#L42) |
| `opcode` | `0x238f` | Local state/value operations | Handled | [43](../../crates/crysirror-cmvs/src/adapter/retained.rs#L43) |
| `opcode` | `0x2391` | Local state/value operations | Handled | [44](../../crates/crysirror-cmvs/src/adapter/retained.rs#L44) |
| `opcode` | `0x2392` | Local state/value operations | Handled | [45](../../crates/crysirror-cmvs/src/adapter/retained.rs#L45) |
| `opcode` | `0x2393` | Local state/value operations | Handled | [46](../../crates/crysirror-cmvs/src/adapter/retained.rs#L46) |
| `opcode` | `0x2394` | Local state/value operations | Handled | [47](../../crates/crysirror-cmvs/src/adapter/retained.rs#L47) |
| `opcode` | `0x23b9` | Local state/value operations | Handled | [48](../../crates/crysirror-cmvs/src/adapter/retained.rs#L48) |
| `opcode` | `_` | Local state/value operations | Handled | [49](../../crates/crysirror-cmvs/src/adapter/retained.rs#L49) |

### adapter/retained.rs / cleared_action

```rust
fn cleared_action(opcode: u16) -> Option<usize>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `opcode` | `0x219d` | Local state/value operations | Handled | [72](../../crates/crysirror-cmvs/src/adapter/retained.rs#L72) |
| `opcode` | `0x219f` | Local state/value operations | Handled | [73](../../crates/crysirror-cmvs/src/adapter/retained.rs#L73) |
| `opcode` | `0x21a1` | Local state/value operations | Handled | [74](../../crates/crysirror-cmvs/src/adapter/retained.rs#L74) |
| `opcode` | `0x21a3` | Local state/value operations | Handled | [75](../../crates/crysirror-cmvs/src/adapter/retained.rs#L75) |
| `opcode` | `0x21a9` | Local state/value operations | Handled | [76](../../crates/crysirror-cmvs/src/adapter/retained.rs#L76) |
| `opcode` | `0x21ab` | Local state/value operations | Handled | [77](../../crates/crysirror-cmvs/src/adapter/retained.rs#L77) |
| `opcode` | `0x21ad` | Local state/value operations | Handled | [78](../../crates/crysirror-cmvs/src/adapter/retained.rs#L78) |
| `opcode` | `0x21af` | Local state/value operations | Handled | [79](../../crates/crysirror-cmvs/src/adapter/retained.rs#L79) |
| `opcode` | `0x21b1` | Local state/value operations | Handled | [80](../../crates/crysirror-cmvs/src/adapter/retained.rs#L80) |
| `opcode` | `0x21b3` | Local state/value operations | Handled | [81](../../crates/crysirror-cmvs/src/adapter/retained.rs#L81) |
| `opcode` | `0x21b5` | Local state/value operations | Handled | [82](../../crates/crysirror-cmvs/src/adapter/retained.rs#L82) |
| `opcode` | `0x21b7` | Local state/value operations | Handled | [83](../../crates/crysirror-cmvs/src/adapter/retained.rs#L83) |
| `opcode` | `0x21b9` | Local state/value operations | Handled | [84](../../crates/crysirror-cmvs/src/adapter/retained.rs#L84) |
| `opcode` | `0x21bb` | Local state/value operations | Handled | [85](../../crates/crysirror-cmvs/src/adapter/retained.rs#L85) |
| `opcode` | `0x21bd` | Local state/value operations | Handled | [86](../../crates/crysirror-cmvs/src/adapter/retained.rs#L86) |
| `opcode` | `0x22f1` | Local state/value operations | Handled | [87](../../crates/crysirror-cmvs/src/adapter/retained.rs#L87) |
| `opcode` | `0x22f3` | Local state/value operations | Handled | [88](../../crates/crysirror-cmvs/src/adapter/retained.rs#L88) |
| `opcode` | `0x22f5` | Local state/value operations | Handled | [89](../../crates/crysirror-cmvs/src/adapter/retained.rs#L89) |
| `opcode` | `0x2345` | Local state/value operations | Handled | [90](../../crates/crysirror-cmvs/src/adapter/retained.rs#L90) |
| `opcode` | `0x2347` | Local state/value operations | Handled | [91](../../crates/crysirror-cmvs/src/adapter/retained.rs#L91) |
| `opcode` | `0x2349` | Local state/value operations | Handled | [92](../../crates/crysirror-cmvs/src/adapter/retained.rs#L92) |
| `opcode` | `0x234b` | Local state/value operations | Handled | [93](../../crates/crysirror-cmvs/src/adapter/retained.rs#L93) |
| `opcode` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [94](../../crates/crysirror-cmvs/src/adapter/retained.rs#L94) |

### adapter/retained.rs / window_child_command

```rust
fn window_child_command(opcode: u16) -> Option<u16>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `opcode` | `0x2176` | Local state/value operations | Handled | [56](../../crates/crysirror-cmvs/src/adapter/retained.rs#L56) |
| `opcode` | `0x2177` | Local state/value operations | Handled | [57](../../crates/crysirror-cmvs/src/adapter/retained.rs#L57) |
| `opcode` | `0x217e` | Local state/value operations | Handled | [58](../../crates/crysirror-cmvs/src/adapter/retained.rs#L58) |
| `opcode` | `0x2181` | Local state/value operations | Handled | [59](../../crates/crysirror-cmvs/src/adapter/retained.rs#L59) |
| `opcode` | `0x2187` | Local state/value operations | Handled | [60](../../crates/crysirror-cmvs/src/adapter/retained.rs#L60) |
| `opcode` | `0x2189` | Local state/value operations | Handled | [61](../../crates/crysirror-cmvs/src/adapter/retained.rs#L61) |
| `opcode` | `0x218b` | Local state/value operations | Handled | [62](../../crates/crysirror-cmvs/src/adapter/retained.rs#L62) |
| `opcode` | `0x21c0..=0x21c3 \| 0x21c5..=0x21c8 \| 0x21ca..=0x21cd` | Local state/value operations | Handled | [63](../../crates/crysirror-cmvs/src/adapter/retained.rs#L63) |
| `opcode` | `0x21d4..=0x21d7` | Local state/value operations | Handled | [64](../../crates/crysirror-cmvs/src/adapter/retained.rs#L64) |
| `opcode` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [65](../../crates/crysirror-cmvs/src/adapter/retained.rs#L65) |

### adapter/scene_effect.rs / words

```rust
fn words(opcode: u16) -> u16
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `opcode` | `0x2290` | Local state/value operations | Handled | [29](../../crates/crysirror-cmvs/src/adapter/scene_effect.rs#L29) |
| `opcode` | `0x2296` | Local state/value operations | Handled | [30](../../crates/crysirror-cmvs/src/adapter/scene_effect.rs#L30) |
| `opcode` | `_` | Local state/value operations | Handled | [31](../../crates/crysirror-cmvs/src/adapter/scene_effect.rs#L31) |

### adapter/scene_effect.rs / MeshScene::new

```rust
fn new(
        host: &mut dyn HostServices,
        client: ClientId,
        opcode: u16,
        index: usize,
        mut images: Vec<Option<DecodedImage>>,
        rand: &mut Rand,
        door_fallback: [bool; 2],
    ) -> Result<Self>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `opcode == 0x2293` | Local state/value operations | Handled | [132](../../crates/crysirror-cmvs/src/adapter/scene_effect.rs#L132) |
| `opcode` | `0x2290` | MeshMode::Track | Handled | [139](../../crates/crysirror-cmvs/src/adapter/scene_effect.rs#L139) |
| `opcode` | `0x2292` | MeshMode::Tower, Tower::default | Handled | [145](../../crates/crysirror-cmvs/src/adapter/scene_effect.rs#L145) |
| `opcode` | `0x2293` | MeshMode::Door, Door::new | Handled | [146](../../crates/crysirror-cmvs/src/adapter/scene_effect.rs#L146) |
| `opcode` | `0x2294` | MeshMode::Particles, Particles::new | Handled | [147](../../crates/crysirror-cmvs/src/adapter/scene_effect.rs#L147) |
| `opcode` | `0x2295` | MeshMode::Actors, Actors::default | Handled | [148](../../crates/crysirror-cmvs/src/adapter/scene_effect.rs#L148) |
| `opcode` | `0x2296` | MeshMode::Spin, Spin::default | Handled | [149](../../crates/crysirror-cmvs/src/adapter/scene_effect.rs#L149) |
| `opcode` | `0x2297` | MeshMode::Orbit, Orbit::new | Handled | [150](../../crates/crysirror-cmvs/src/adapter/scene_effect.rs#L150) |
| `opcode` | `_` | Local state/value operations | Handled | [151](../../crates/crysirror-cmvs/src/adapter/scene_effect.rs#L151) |

### adapter/scene_effect.rs / MeshScene::requested_images

```rust
fn requested_images(&self, key: u32) -> Option<&'static [&'static str]>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `(&self.mode, key)` | `(MeshMode::Orbit(_), 10)` | Local state/value operations | Handled | [159](../../crates/crysirror-cmvs/src/adapter/scene_effect.rs#L159) |
| `(&self.mode, key)` | `(MeshMode::Particles(_), 50)` | Local state/value operations | Handled | [160](../../crates/crysirror-cmvs/src/adapter/scene_effect.rs#L160) |
| `(&self.mode, key)` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [161](../../crates/crysirror-cmvs/src/adapter/scene_effect.rs#L161) |

### adapter/scene_effect.rs / Tower::trigger

```rust
fn trigger(&mut self, key: u32) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(key, 1 \| 2)` | Local state/value operations | Handled | [603](../../crates/crysirror-cmvs/src/adapter/scene_effect.rs#L603) |

### adapter/scene_effect/actors.rs / Actors::advance

```rust
fn advance(&mut self, delta: u32) -> SceneSound
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `self.phase == 0 && self.queued.is_some_and(\|key\| key != 0)` | self.queued.take | Handled | [314](../../crates/crysirror-cmvs/src/adapter/scene_effect/actors.rs#L314) |

### adapter/scene_effect/actors.rs / Actors::trigger

```rust
fn trigger(&mut self, key: u32) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `key` | `5` | Local state/value operations | Handled | [252](../../crates/crysirror-cmvs/src/adapter/scene_effect/actors.rs#L252) |
| `key` | `15` | Local state/value operations | Handled | [253](../../crates/crysirror-cmvs/src/adapter/scene_effect/actors.rs#L253) |
| `key` | `16` | Local state/value operations | Handled | [254](../../crates/crysirror-cmvs/src/adapter/scene_effect/actors.rs#L254) |
| `key` | `1..=4 \| 6..=8 \| 10..=14 \| 17` | Local state/value operations | Handled | [255](../../crates/crysirror-cmvs/src/adapter/scene_effect/actors.rs#L255) |
| `key` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [256](../../crates/crysirror-cmvs/src/adapter/scene_effect/actors.rs#L256) |

### adapter/scene_effect/actors.rs / Rotation::advance

```rust
fn advance(&mut self, delta: u32, angle: &mut i32) -> bool
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `mode <= 1` | f64::from | Handled | [78](../../crates/crysirror-cmvs/src/adapter/scene_effect/actors.rs#L78) |
| `condition` | `mode == 0` | Local state/value operations | Handled | [86](../../crates/crysirror-cmvs/src/adapter/scene_effect/actors.rs#L86) |
| `condition` | `mode < 128` | Local state/value operations | Handled | [97](../../crates/crysirror-cmvs/src/adapter/scene_effect/actors.rs#L97) |

### adapter/scene_effect/door.rs / Door::trigger

```rust
fn trigger(&mut self, key: u32) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `(1..=5).contains(&key)` | Local state/value operations | Handled | [60](../../crates/crysirror-cmvs/src/adapter/scene_effect/door.rs#L60) |

### adapter/scene_effect/particles.rs / Particles::trigger

```rust
fn trigger(&mut self, key: u32) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(key, 10 \| 11 \| 20 \| 22)` | Local state/value operations | Handled | [121](../../crates/crysirror-cmvs/src/adapter/scene_effect/particles.rs#L121) |

### adapter/sequence.rs / run

```rust
fn run(vm: &mut CmvsVm, instant: bool, now: u32, exists: impl Fn(usize) -> bool) -> Vec<Action>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `step.sub; under step.kind=SequenceKind::Object if exists(step.slot as usize)` | `0` | Local state/value operations | Handled | [39](../../crates/crysirror-cmvs/src/adapter/sequence.rs#L39) |
| `step.sub; under step.kind=SequenceKind::Object if exists(step.slot as usize)` | `1 \| 2 if !instant \|\| step.sub == 1` | Local state/value operations | Handled | [45](../../crates/crysirror-cmvs/src/adapter/sequence.rs#L45) |
| `step.sub; under step.kind=SequenceKind::Object if exists(step.slot as usize)` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [52](../../crates/crysirror-cmvs/src/adapter/sequence.rs#L52) |
| `step.kind` | `SequenceKind::Wait if !instant` | Local state/value operations | Handled | [29](../../crates/crysirror-cmvs/src/adapter/sequence.rs#L29) |
| `step.kind` | `SequenceKind::Object if exists(step.slot as usize)` | Local state/value operations | Handled | [36](../../crates/crysirror-cmvs/src/adapter/sequence.rs#L36) |
| `step.kind` | `SequenceKind::Camera` | Local state/value operations | Handled | [55](../../crates/crysirror-cmvs/src/adapter/sequence.rs#L55) |
| `step.kind` | `SequenceKind::Window if step.sub == 0` | Local state/value operations | Handled | [59](../../crates/crysirror-cmvs/src/adapter/sequence.rs#L59) |
| `step.kind` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [65](../../crates/crysirror-cmvs/src/adapter/sequence.rs#L65) |

### adapter/sound.rs / fader_update

```rust
fn fader_update(opcode: u16, word: u32) -> AudioPcm16GainUpdate
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(opcode, 0x20ad \| 0x20b9)` | AudioPcm16GainUpdate::Stereo | Handled | [125](../../crates/crysirror-cmvs/src/adapter/sound.rs#L125) |
| `condition` | `matches!(opcode, 0x20ad \| 0x20b9)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [125](../../crates/crysirror-cmvs/src/adapter/sound.rs#L125) |

### adapter/sound.rs / Effects::set_fader

```rust
fn set_fader(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        slot: usize,
        opcode: u16,
        word: u32,
    ) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `opcode == 0x20b8` | Local state/value operations | Handled | [416](../../crates/crysirror-cmvs/src/adapter/sound.rs#L416) |

### adapter/sound.rs / Levels::live_channels

```rust
fn live_channels(self, opcode: u16, effects: &Effects) -> Vec<(u32, f32)>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `self.music.enabled && matches!(opcode, 0x22d9 \| 0x22da)` | self.music.gain | Handled | [1142](../../crates/crysirror-cmvs/src/adapter/sound.rs#L1142) |
| `condition` | `matches!(opcode, 0x22d9 \| 0x22da)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [1142](../../crates/crysirror-cmvs/src/adapter/sound.rs#L1142) |
| `condition` | `matches!(opcode, 0x22d9 \| 0x22dc)` | Local state/value operations | Handled | [1145](../../crates/crysirror-cmvs/src/adapter/sound.rs#L1145) |
| `condition` | `matches!(opcode, 0x22d9 \| 0x22dc)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [1145](../../crates/crysirror-cmvs/src/adapter/sound.rs#L1145) |
| `condition` | `matches!(opcode, 0x22d9 \| 0x22db)` | Local state/value operations | Handled | [1149](../../crates/crysirror-cmvs/src/adapter/sound.rs#L1149) |
| `condition` | `matches!(opcode, 0x22d9 \| 0x22db)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [1149](../../crates/crysirror-cmvs/src/adapter/sound.rs#L1149) |
| `condition` | `opcode == 0x22db && volume > 0` | Local state/value operations | Handled | [1153](../../crates/crysirror-cmvs/src/adapter/sound.rs#L1153) |

### adapter/sound.rs / Levels::read

```rust
fn read(self, opcode: u16) -> u32
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `opcode` | `0x22d9` | Local state/value operations | Handled | [1111](../../crates/crysirror-cmvs/src/adapter/sound.rs#L1111) |
| `opcode` | `0x22da` | Local state/value operations | Handled | [1112](../../crates/crysirror-cmvs/src/adapter/sound.rs#L1112) |
| `opcode` | `0x22db` | Local state/value operations | Handled | [1113](../../crates/crysirror-cmvs/src/adapter/sound.rs#L1113) |
| `opcode` | `0x22dc` | Local state/value operations | Handled | [1114](../../crates/crysirror-cmvs/src/adapter/sound.rs#L1114) |
| `opcode` | `0x22dd` | Local state/value operations | Handled | [1115](../../crates/crysirror-cmvs/src/adapter/sound.rs#L1115) |
| `opcode` | `_` | Local state/value operations | Handled | [1116](../../crates/crysirror-cmvs/src/adapter/sound.rs#L1116) |

### adapter/sound.rs / Levels::updated

```rust
fn updated(mut self, opcode: u16, raw: u32) -> Self
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `opcode` | `0x22d9` | Local state/value operations | Handled | [1123](../../crates/crysirror-cmvs/src/adapter/sound.rs#L1123) |
| `opcode` | `0x22da` | Local state/value operations | Handled | [1128](../../crates/crysirror-cmvs/src/adapter/sound.rs#L1128) |
| `opcode` | `0x22db` | Local state/value operations | Handled | [1129](../../crates/crysirror-cmvs/src/adapter/sound.rs#L1129) |
| `opcode` | `0x22dc` | Local state/value operations | Handled | [1130](../../crates/crysirror-cmvs/src/adapter/sound.rs#L1130) |
| `opcode` | `0x22dd` | Local state/value operations | Handled | [1131](../../crates/crysirror-cmvs/src/adapter/sound.rs#L1131) |
| `opcode` | `_` | Local state/value operations | Handled | [1132](../../crates/crysirror-cmvs/src/adapter/sound.rs#L1132) |
| `condition` | `matches!(opcode, 0x22d9 \| 0x22dd)` | Local state/value operations | Handled | [1134](../../crates/crysirror-cmvs/src/adapter/sound.rs#L1134) |
| `condition` | `matches!(opcode, 0x22d9 \| 0x22dd)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [1134](../../crates/crysirror-cmvs/src/adapter/sound.rs#L1134) |

### adapter/sound.rs / MusicRequest::read

```rust
fn read(vm: &CmvsVm, opcode: u16) -> Result<Self>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `opcode == 0x20a4` | Local state/value operations | Handled | [1180](../../crates/crysirror-cmvs/src/adapter/sound.rs#L1180) |

### adapter/sound.rs / VoiceRequest::read

```rust
fn read(vm: &CmvsVm, opcode: u16) -> Result<Self>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `opcode == 0x20af` | f32::from_bits | Handled | [688](../../crates/crysirror-cmvs/src/adapter/sound.rs#L688) |

### adapter/text_debug.rs / TextDebug::command

```rust
fn command(
        &mut self,
        opcode: u16,
        vm: &mut CmvsVm,
        host: &mut dyn HostServices,
        client: ClientId,
        canvas: [u32; 2],
        font: &str,
        prefix: &str,
    ) -> Result<Request>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `opcode` | `0x23a4` | Request::Load, self.file_name | Handled | [236](../../crates/crysirror-cmvs/src/adapter/text_debug.rs#L236) |
| `opcode` | `0x23a5` | Local state/value operations | Handled | [237](../../crates/crysirror-cmvs/src/adapter/text_debug.rs#L237) |
| `opcode` | `0x23a6` | self.open.is_some, self.poll, self.find, self.open, Vec::new | Handled | [248](../../crates/crysirror-cmvs/src/adapter/text_debug.rs#L248) |
| `opcode` | `0x23a7` | self.find, u32::from | Handled | [259](../../crates/crysirror-cmvs/src/adapter/text_debug.rs#L259) |
| `opcode` | `_` | Local state/value operations | Handled | [269](../../crates/crysirror-cmvs/src/adapter/text_debug.rs#L269) |

### adapter/text_debug.rs / TextDebug::poll

```rust
fn poll(&mut self, vm: &mut CmvsVm, host: &mut dyn HostServices, client: ClientId, font: &str, prefix: &str) -> Result<Request>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `events.iter().any(\|event\| event["kind"] == "dialog_result")` | Local state/value operations | Handled | [361](../../crates/crysirror-cmvs/src/adapter/text_debug.rs#L361) |
| `condition` | `event["kind"] != "activate"` | Local state/value operations | Handled | [376](../../crates/crysirror-cmvs/src/adapter/text_debug.rs#L376) |
| `condition; under event["widget"]["id"].as_u64().map(\|id\| id as u32)=Some(id) if id == KINDS[0] \|\| id == KINDS[3]` | `id == KINDS[0]` | Local state/value operations | Handled | [381](../../crates/crysirror-cmvs/src/adapter/text_debug.rs#L381) |
| `event["widget"]["id"].as_u64().map(\|id\| id as u32)` | `Some(id) if id == KINDS[0] \|\| id == KINDS[3]` | Local state/value operations | Handled | [380](../../crates/crysirror-cmvs/src/adapter/text_debug.rs#L380) |
| `event["widget"]["id"].as_u64().map(\|id\| id as u32)` | `Some(CONFIRM)` | Local state/value operations | Handled | [383](../../crates/crysirror-cmvs/src/adapter/text_debug.rs#L383) |
| `event["widget"]["id"].as_u64().map(\|id\| id as u32)` | `Some(ABORT)` | Local state/value operations | Handled | [384](../../crates/crysirror-cmvs/src/adapter/text_debug.rs#L384) |
| `event["widget"]["id"].as_u64().map(\|id\| id as u32)` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [385](../../crates/crysirror-cmvs/src/adapter/text_debug.rs#L385) |

### adapter/visual.rs / plan

```rust
fn plan(
    slot: usize,
    object: &mut Object,
    canvas: [u32; 2],
    quake: [[i32; 2]; 2],
) -> Result<Vec<serde_json::Value>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under (child_image, child.draw.as_ref())=(image, Some(draw)) if (image.is_some() \|\| draw.fill.is_some()) && object.flags.shown && child.flags.shown && draw.order >= 0` | `parent.mode == 4` | Local state/value operations | Handled | [1985](../../crates/crysirror-cmvs/src/adapter/visual.rs#L1985) |

### adapter/visual.rs / project

```rust
fn project(camera: &[u32; 40], draw: &Draw) -> Option<Projected>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under camera[0]=mode @ (2 \| 3)` | `mode == 2` | Local state/value operations | Handled | [164](../../crates/crysirror-cmvs/src/adapter/visual.rs#L164) |
| `condition; under camera[0]=mode @ (2 \| 3)` | `mode == 2` | Local state/value operations | Handled | [170](../../crates/crysirror-cmvs/src/adapter/visual.rs#L170) |

### adapter/visual.rs / put

```rust
fn put(
    id: u32,
    slot: usize,
    tie: u64,
    draw: &Draw,
    image: Option<&UploadedImage>,
    second: Option<&UploadedImage>,
    logical_size: [u32; 2],
    canvas: [u32; 2],
    offset: [i32; 2],
    position: [f32; 2],
) -> Result<Option<serde_json::Value>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `draw.kind < 2 && second.is_some() && draw.mode != 4` | Local state/value operations | Handled | [2018](../../crates/crysirror-cmvs/src/adapter/visual.rs#L2018) |
| `condition` | `draw.invert && (draw.kind >= 2 \|\| second.is_none())` | Local state/value operations | Handled | [2064](../../crates/crysirror-cmvs/src/adapter/visual.rs#L2064) |

### adapter/visual.rs / Objects::anchor

```rust
fn anchor(&self, slot: usize) -> [i32; 2]
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `draw.kind == 5` | Local state/value operations | Handled | [1075](../../crates/crysirror-cmvs/src/adapter/visual.rs#L1075) |

### adapter/visual.rs / Objects::point_command

```rust
fn point_command(&mut self, slot: usize, command: u16, point: [f32; 3]) -> bool
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `command` | `0x2330` | Vec::new | Handled | [1349](../../crates/crysirror-cmvs/src/adapter/visual.rs#L1349) |
| `command` | `0x2331` | Local state/value operations | Handled | [1350](../../crates/crysirror-cmvs/src/adapter/visual.rs#L1350) |
| `command` | `_` | Local state/value operations | Handled | [1351](../../crates/crysirror-cmvs/src/adapter/visual.rs#L1351) |

### adapter/visual.rs / Objects::property_command

```rust
fn property_command(
        &mut self,
        opcode: u16,
        host: &mut dyn HostServices,
        client: ClientId,
        vm: &mut CmvsVm,
    ) -> Result<Option<u32>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `opcode` | `0x2041` | Local state/value operations | Handled | [523](../../crates/crysirror-cmvs/src/adapter/visual.rs#L523) |
| `opcode` | `0x2025 \| 0x202c \| 0x202d \| 0x2039 \| 0x2042 \| 0x204f \| 0x2051 \| 0x2078` | Local state/value operations | Handled | [524](../../crates/crysirror-cmvs/src/adapter/visual.rs#L524) |
| `opcode` | `0x2049 \| 0x2054` | Local state/value operations | Handled | [525](../../crates/crysirror-cmvs/src/adapter/visual.rs#L525) |
| `opcode` | `0x2043` | Local state/value operations | Handled | [526](../../crates/crysirror-cmvs/src/adapter/visual.rs#L526) |
| `opcode` | `_` | Local state/value operations | Handled | [527](../../crates/crysirror-cmvs/src/adapter/visual.rs#L527) |
| `condition` | `opcode == 0x2041` | Local state/value operations | Handled | [530](../../crates/crysirror-cmvs/src/adapter/visual.rs#L530) |
| `(opcode, vm.profile())` | `(0x2321, VmProfile::Pc20160201X64)` | Local state/value operations | Handled | [541](../../crates/crysirror-cmvs/src/adapter/visual.rs#L541) |
| `(opcode, vm.profile())` | `_` | Local state/value operations | Handled | [542](../../crates/crysirror-cmvs/src/adapter/visual.rs#L542) |
| `condition` | `child >= child_limit \|\| (matches!(opcode, 0x202c \| 0x2039 \| 0x2051) && child < 0)` | Local state/value operations | Handled | [544](../../crates/crysirror-cmvs/src/adapter/visual.rs#L544) |
| `condition` | `matches!(opcode, 0x202c \| 0x2039 \| 0x2051)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [544](../../crates/crysirror-cmvs/src/adapter/visual.rs#L544) |
| `condition; under opcode=_ / opcode=0x2055` | `kind <= 3` | Local state/value operations | Handled | [594](../../crates/crysirror-cmvs/src/adapter/visual.rs#L594) |
| `opcode; under opcode=_` | `0x2042` | Local state/value operations | Handled | [589](../../crates/crysirror-cmvs/src/adapter/visual.rs#L589) |
| `opcode; under opcode=_` | `0x204c` | u32::from | Handled | [590](../../crates/crysirror-cmvs/src/adapter/visual.rs#L590) |
| `opcode; under opcode=_` | `0x204e` | u32::from | Handled | [591](../../crates/crysirror-cmvs/src/adapter/visual.rs#L591) |
| `opcode; under opcode=_` | `0x2055` | Local state/value operations | Handled | [592](../../crates/crysirror-cmvs/src/adapter/visual.rs#L592) |
| `opcode; under opcode=_` | `0x2043` | Local state/value operations | Handled | [598](../../crates/crysirror-cmvs/src/adapter/visual.rs#L598) |
| `opcode; under opcode=_` | `0x2049` | Local state/value operations | Handled | [603](../../crates/crysirror-cmvs/src/adapter/visual.rs#L603) |
| `opcode; under opcode=_` | `0x204a` | Local state/value operations | Handled | [604](../../crates/crysirror-cmvs/src/adapter/visual.rs#L604) |
| `opcode; under opcode=_` | `0x204b` | Local state/value operations | Handled | [605](../../crates/crysirror-cmvs/src/adapter/visual.rs#L605) |
| `opcode; under opcode=_` | `0x2054` | Local state/value operations | Handled | [611](../../crates/crysirror-cmvs/src/adapter/visual.rs#L611) |
| `opcode; under opcode=_` | `0x2070` | Local state/value operations | Handled | [612](../../crates/crysirror-cmvs/src/adapter/visual.rs#L612) |
| `opcode; under opcode=_` | `0x2071` | Local state/value operations | Handled | [613](../../crates/crysirror-cmvs/src/adapter/visual.rs#L613) |
| `opcode; under opcode=_` | `0x2072` | Local state/value operations | Handled | [614](../../crates/crysirror-cmvs/src/adapter/visual.rs#L614) |
| `opcode; under opcode=_` | `0x2073` | Local state/value operations | Handled | [615](../../crates/crysirror-cmvs/src/adapter/visual.rs#L615) |
| `opcode; under opcode=_` | `0x2074` | Local state/value operations | Handled | [616](../../crates/crysirror-cmvs/src/adapter/visual.rs#L616) |
| `opcode; under opcode=_` | `0x2075` | Local state/value operations | Handled | [617](../../crates/crysirror-cmvs/src/adapter/visual.rs#L617) |
| `opcode; under opcode=_` | `0x2320` | Local state/value operations | Handled | [618](../../crates/crysirror-cmvs/src/adapter/visual.rs#L618) |
| `opcode; under opcode=_` | `0x2321` | Local state/value operations | Handled | [619](../../crates/crysirror-cmvs/src/adapter/visual.rs#L619) |
| `opcode; under opcode=_` | `0x2322` | Local state/value operations | Handled | [620](../../crates/crysirror-cmvs/src/adapter/visual.rs#L620) |
| `opcode; under opcode=_` | `0x204f` | Local state/value operations | Handled | [621](../../crates/crysirror-cmvs/src/adapter/visual.rs#L621) |
| `opcode; under opcode=_` | `0x2078` | Local state/value operations | Handled | [631](../../crates/crysirror-cmvs/src/adapter/visual.rs#L631) |
| `opcode; under opcode=_` | `_` | Local state/value operations | Handled | [640](../../crates/crysirror-cmvs/src/adapter/visual.rs#L640) |
| `opcode` | `0x2025 \| 0x202d` | Local state/value operations | Empty handler: Original no-op status not established | [557](../../crates/crysirror-cmvs/src/adapter/visual.rs#L557) |
| `opcode` | `0x202c` | Local state/value operations | Handled | [558](../../crates/crysirror-cmvs/src/adapter/visual.rs#L558) |
| `opcode` | `0x2026` | Local state/value operations | Handled | [561](../../crates/crysirror-cmvs/src/adapter/visual.rs#L561) |
| `opcode` | `0x2039` | host.release_image_resource, self.slots.insert | Handled | [568](../../crates/crysirror-cmvs/src/adapter/visual.rs#L568) |
| `opcode` | `0x2041 \| 0x2051` | Local state/value operations | Handled | [578](../../crates/crysirror-cmvs/src/adapter/visual.rs#L578) |
| `opcode` | `_` | u32::from, Self::reproject | Handled | [582](../../crates/crysirror-cmvs/src/adapter/visual.rs#L582) |

### adapter/visual.rs / Objects::saved_images

```rust
fn saved_images(value: &serde_json::Value) -> Vec<(usize, Option<usize>, String)>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Some(name) = object["source"].as_str()` | Local state/value operations | Handled | [1460](../../crates/crysirror-cmvs/src/adapter/visual.rs#L1460) |
| `condition` | `let (Some(index), Some(name)) = (child["index"].as_u64(), child["source"].as_str())` | Local state/value operations | Handled | [1464](../../crates/crysirror-cmvs/src/adapter/visual.rs#L1464) |

### adapter/visual.rs / Objects::track_command

```rust
fn track_command(
        &mut self,
        opcode: u16,
        host: &mut dyn HostServices,
        client: ClientId,
        vm: &mut CmvsVm,
    ) -> Result<Option<u32>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `command` | `0` | f32::from_bits, Track::new | Handled | [461](../../crates/crysirror-cmvs/src/adapter/visual.rs#L461) |
| `command` | `1` | usize::from | Handled | [477](../../crates/crysirror-cmvs/src/adapter/visual.rs#L477) |
| `command` | `2` | Local state/value operations | Handled | [486](../../crates/crysirror-cmvs/src/adapter/visual.rs#L486) |
| `command` | `3` | Local state/value operations | Handled | [495](../../crates/crysirror-cmvs/src/adapter/visual.rs#L495) |
| `command` | `_` | Local state/value operations | Handled | [503](../../crates/crysirror-cmvs/src/adapter/visual.rs#L503) |

### adapter/visual_opacity.rs / Opacity::advance

```rust
fn advance(&mut self, delta: u32, opacity: &mut i32) -> Option<i32>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `key.next < 0` | Local state/value operations | Handled | [104](../../crates/crysirror-cmvs/src/adapter/visual_opacity.rs#L104) |

### adapter/visual_opacity.rs / Opacity::command

```rust
fn command(
        opcode: u16,
        vm: &CmvsVm,
        track: &mut Option<Self>,
        opacity: &mut i32,
    ) -> Result<(u16, Option<u32>)>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `opcode` | `0x21ea` | Key::default | Handled | [35](../../crates/crysirror-cmvs/src/adapter/visual_opacity.rs#L35) |
| `opcode` | `0x21eb` | Local state/value operations | Handled | [44](../../crates/crysirror-cmvs/src/adapter/visual_opacity.rs#L44) |
| `opcode` | `0x21ec` | Local state/value operations | Handled | [60](../../crates/crysirror-cmvs/src/adapter/visual_opacity.rs#L60) |
| `opcode` | `0x21ed` | Local state/value operations | Handled | [68](../../crates/crysirror-cmvs/src/adapter/visual_opacity.rs#L68) |
| `opcode` | `_` | Local state/value operations | Handled | [76](../../crates/crysirror-cmvs/src/adapter/visual_opacity.rs#L76) |

### adapter/visual_track.rs / Channel::of

```rust
fn of(opcode: u16) -> Option<(Self, u16)>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `opcode` | `0x21e0..=0x21e4` | Local state/value operations | Handled | [19](../../crates/crysirror-cmvs/src/adapter/visual_track.rs#L19) |
| `opcode` | `0x21e5..=0x21e9` | Local state/value operations | Handled | [20](../../crates/crysirror-cmvs/src/adapter/visual_track.rs#L20) |
| `opcode` | `0x21ef..=0x21f3` | Local state/value operations | Handled | [21](../../crates/crysirror-cmvs/src/adapter/visual_track.rs#L21) |
| `opcode` | `0x21f4..=0x21f8` | Local state/value operations | Handled | [22](../../crates/crysirror-cmvs/src/adapter/visual_track.rs#L22) |
| `opcode` | `0x21f9..=0x21fd` | Local state/value operations | Handled | [23](../../crates/crysirror-cmvs/src/adapter/visual_track.rs#L23) |
| `opcode` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [24](../../crates/crysirror-cmvs/src/adapter/visual_track.rs#L24) |

### adapter/visual_track.rs / Channel::words

```rust
fn words(self, command: u16) -> u16
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `(self, command)` | `(Self::Scale, 0)` | Local state/value operations | Handled | [40](../../crates/crysirror-cmvs/src/adapter/visual_track.rs#L40) |
| `(self, command)` | `(Self::Rotation, 0)` | Local state/value operations | Handled | [41](../../crates/crysirror-cmvs/src/adapter/visual_track.rs#L41) |
| `(self, command)` | `(_, 0)` | Local state/value operations | Handled | [42](../../crates/crysirror-cmvs/src/adapter/visual_track.rs#L42) |
| `(self, command)` | `(Self::Scale, 1)` | Local state/value operations | Handled | [43](../../crates/crysirror-cmvs/src/adapter/visual_track.rs#L43) |
| `(self, command)` | `(Self::Rotation, 1)` | Local state/value operations | Handled | [44](../../crates/crysirror-cmvs/src/adapter/visual_track.rs#L44) |
| `(self, command)` | `(Self::Frame, 1)` | Local state/value operations | Handled | [45](../../crates/crysirror-cmvs/src/adapter/visual_track.rs#L45) |
| `(self, command)` | `(Self::Position, 1)` | Local state/value operations | Handled | [46](../../crates/crysirror-cmvs/src/adapter/visual_track.rs#L46) |
| `(self, command)` | `(Self::World, 1)` | Local state/value operations | Handled | [47](../../crates/crysirror-cmvs/src/adapter/visual_track.rs#L47) |
| `(self, command)` | `(_, 2)` | Local state/value operations | Handled | [48](../../crates/crysirror-cmvs/src/adapter/visual_track.rs#L48) |
| `(self, command)` | `_` | Local state/value operations | Handled | [49](../../crates/crysirror-cmvs/src/adapter/visual_track.rs#L49) |

### adapter/visual_track.rs / Track::advance

```rust
fn advance(&mut self, delta: u32) -> (Option<[f32; 6]>, Option<i32>)
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `self.channel == Channel::Rotation && key.mode & 0xff == 0x80` | Local state/value operations | Handled | [171](../../crates/crysirror-cmvs/src/adapter/visual_track.rs#L171) |
| `condition` | `key.mode & 0x8000 != 0` | std::array::from_fn | Handled | [191](../../crates/crysirror-cmvs/src/adapter/visual_track.rs#L191) |
| `condition` | `key.next < 0` | Local state/value operations | Handled | [198](../../crates/crysirror-cmvs/src/adapter/visual_track.rs#L198) |

### adapter/visual_track.rs / Track::interpolate

```rust
fn interpolate(&self, key: &Key, elapsed: u32) -> [f32; 6]
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `key.mode & 0xff` | `1` | Local state/value operations | Handled | [155](../../crates/crysirror-cmvs/src/adapter/visual_track.rs#L155) |
| `key.mode & 0xff` | `2` | Local state/value operations | Handled | [156](../../crates/crysirror-cmvs/src/adapter/visual_track.rs#L156) |
| `key.mode & 0xff` | `_` | Local state/value operations | Handled | [157](../../crates/crysirror-cmvs/src/adapter/visual_track.rs#L157) |

### vm.rs / decode_at

```rust
fn decode_at(code: &[u8], pc: usize) -> Result<Instruction, DecodeError>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `opcode` | `0x200..=0x202` | InstructionKind::Expression | Handled | [319](../../crates/crysirror-cmvs/src/vm.rs#L319) |
| `opcode` | `0x400` | Local state/value operations | Handled | [323](../../crates/crysirror-cmvs/src/vm.rs#L323) |
| `opcode` | `0x401 \| 0x402` | Local state/value operations | Handled | [326](../../crates/crysirror-cmvs/src/vm.rs#L326) |
| `opcode` | `0x403` | Local state/value operations | Handled | [329](../../crates/crysirror-cmvs/src/vm.rs#L329) |
| `opcode` | `0x407` | Local state/value operations | Handled | [333](../../crates/crysirror-cmvs/src/vm.rs#L333) |
| `opcode` | `0x405 \| 0x410` | Local state/value operations | Handled | [337](../../crates/crysirror-cmvs/src/vm.rs#L337) |
| `opcode` | `0x411 \| 0x412` | Local state/value operations | Handled | [340](../../crates/crysirror-cmvs/src/vm.rs#L340) |
| `opcode` | `0x414` | Local state/value operations | Handled | [343](../../crates/crysirror-cmvs/src/vm.rs#L343) |
| `opcode` | `0x413 \| 0x416 \| 0x430` | Local state/value operations | Handled | [346](../../crates/crysirror-cmvs/src/vm.rs#L346) |
| `opcode` | `0x440 \| 0x442` | Local state/value operations | Handled | [347](../../crates/crysirror-cmvs/src/vm.rs#L347) |
| `opcode` | `0x2000..=0x27ff` | Local state/value operations | Handled | [351](../../crates/crysirror-cmvs/src/vm.rs#L351) |
| `opcode` | `_` | Local state/value operations | Unsupported: unsupported instruction | [354](../../crates/crysirror-cmvs/src/vm.rs#L354) |

### vm.rs / Reader<'_>::expression

```rust
fn expression(&mut self, depth: usize) -> Result<Expression, DecodeError>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!(0x200..=0x202).contains(&kind)` | self.error | Unsupported: expected expression prefix | [184](../../crates/crysirror-cmvs/src/vm.rs#L184) |
| `condition; under tag=0x101 \| 0x102 \| 0x12b if kind != 0x201` | `expression.kind != 0x200` | self.error | Unsupported: index must be integer expression | [220](../../crates/crysirror-cmvs/src/vm.rs#L220) |
| `condition; under tag=0x121 if kind == 0x201` | `expression.kind != 0x200` | self.error | Unsupported: string index must be integer | [231](../../crates/crysirror-cmvs/src/vm.rs#L231) |
| `condition; under tag=0x105 \| 0x109 \| 0x110 \| 0x112 \| 0x12d \| 0x12f \| 0x134 \| 0x136 if kind != 0x201 && (tag < 0x130 \|\| kind == 0x202)` | `expression.kind != 0x200` | self.error | Unsupported: index must be integer expression | [245](../../crates/crysirror-cmvs/src/vm.rs#L245) |
| `condition; under tag=0x107 \| 0x10b \| 0x111 \| 0x113 \| 0x131 \| 0x133 \| 0x135 \| 0x137 if kind != 0x201 && (tag < 0x130 \|\| kind == 0x202)` | `indices.iter().any(\|e\| e.kind != 0x200)` | self.error | Unsupported: index must be integer expression | [265](../../crates/crysirror-cmvs/src/vm.rs#L265) |

### vm/control.rs / Control::command

```rust
fn command(
        &mut self,
        machine: &mut Machine,
        opcode: u16,
    ) -> Result<Option<u16>, DecodeError>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `opcode; under opcode=0x2008 \| 0x2009 \| 0x200f` | `0x2008` | Local state/value operations | Handled | [176](../../crates/crysirror-cmvs/src/vm/control.rs#L176) |
| `opcode; under opcode=0x2008 \| 0x2009 \| 0x200f` | `0x2009` | Local state/value operations | Handled | [177](../../crates/crysirror-cmvs/src/vm/control.rs#L177) |
| `opcode; under opcode=0x2008 \| 0x2009 \| 0x200f` | `_` | self.clock.wrapping_sub | Handled | [178](../../crates/crysirror-cmvs/src/vm/control.rs#L178) |
| `condition; under opcode=0x200b \| 0x200c` | `opcode == 0x200b` | Local state/value operations | Handled | [192](../../crates/crysirror-cmvs/src/vm/control.rs#L192) |
| `condition; under opcode=0x20d4 \| 0x20d5` | `opcode == 0x20d4` | Local state/value operations | Handled | [208](../../crates/crysirror-cmvs/src/vm/control.rs#L208) |
| `opcode; under opcode=0x20d0..=0x20d3` | `0x20d0` | Local state/value operations | Handled | [226](../../crates/crysirror-cmvs/src/vm/control.rs#L226) |
| `opcode; under opcode=0x20d0..=0x20d3` | `0x20d1` | Local state/value operations | Handled | [227](../../crates/crysirror-cmvs/src/vm/control.rs#L227) |
| `opcode; under opcode=0x20d0..=0x20d3` | `_` | Local state/value operations | Handled | [228](../../crates/crysirror-cmvs/src/vm/control.rs#L228) |
| `opcode; under opcode=0x20d0..=0x20d3` | `0x20d0` | Local state/value operations | Handled | [236](../../crates/crysirror-cmvs/src/vm/control.rs#L236) |
| `opcode; under opcode=0x20d0..=0x20d3` | `0x20d1` | Local state/value operations | Handled | [237](../../crates/crysirror-cmvs/src/vm/control.rs#L237) |
| `opcode; under opcode=0x20d0..=0x20d3` | `0x20d2` | Local state/value operations | Handled | [238](../../crates/crysirror-cmvs/src/vm/control.rs#L238) |
| `opcode; under opcode=0x20d0..=0x20d3` | `_` | Local state/value operations | Handled | [239](../../crates/crysirror-cmvs/src/vm/control.rs#L239) |
| `condition; under opcode=0x20d7 / mode=1 \| 2` | `mode == 1` | Local state/value operations | Handled | [266](../../crates/crysirror-cmvs/src/vm/control.rs#L266) |
| `mode; under opcode=0x20d7` | `0` | Local state/value operations | Handled | [262](../../crates/crysirror-cmvs/src/vm/control.rs#L262) |
| `mode; under opcode=0x20d7` | `1 \| 2` | f64::from | Handled | [263](../../crates/crysirror-cmvs/src/vm/control.rs#L263) |
| `mode; under opcode=0x20d7` | `_` | Local state/value operations | Handled | [272](../../crates/crysirror-cmvs/src/vm/control.rs#L272) |
| `condition; under opcode=0x20d8 \| 0x20d9 \| 0x20dc` | `opcode == 0x20dc` | Local state/value operations | Handled | [282](../../crates/crysirror-cmvs/src/vm/control.rs#L282) |
| `condition; under opcode=0x20d8 \| 0x20d9 \| 0x20dc` | `opcode == 0x20d8` | Local state/value operations | Handled | [287](../../crates/crysirror-cmvs/src/vm/control.rs#L287) |
| `condition; under opcode=0x20da \| 0x20db` | `opcode == 0x20da` | Local state/value operations | Handled | [324](../../crates/crysirror-cmvs/src/vm/control.rs#L324) |
| `condition; under opcode=0x2088 \| 0x208b` | `opcode == 0x208b` | Local state/value operations | Handled | [367](../../crates/crysirror-cmvs/src/vm/control.rs#L367) |
| `condition; under opcode=0x2089 \| 0x208c` | `opcode == 0x2089` | Local state/value operations | Handled | [395](../../crates/crysirror-cmvs/src/vm/control.rs#L395) |
| `opcode` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [402](../../crates/crysirror-cmvs/src/vm/control.rs#L402) |

### vm/cvns.rs / returns_without_effect

```rust
fn returns_without_effect(opcode: u16) -> bool
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(opcode, 0x2001 \| 0x200e..=0x200f \| 0x2011 \| 0x2015..=0x2017 \| 0x201d..=0x201e \| 0x2027 \| 0x202d..=0x202f \| 0x2035..=0x2037 \| 0x203e..=0x203f \| 0x2043..=0x2046 \| 0x204d..=0x204f \| 0x2054 \| 0x2060..=0x2061 \| 0x2064..=0x2065 \| 0x206a \| 0x206e..=0x206f \| 0x2073 \| 0x2087 \| 0x2089..=0x208f \| 0x2095..=0x209f \| 0x20a3 \| 0x20a7..=0x20ac \| 0x20bf \| 0x20c1..=0x20c4 \| 0x20cd..=0x20cf \| 0x20ec..=0x20ef \| 0x20fe..=0x20ff \| 0x2103..=0x2107 \| 0x210f \| 0x2115..=0x2116 \| 0x211c \| 0x2127 \| 0x2132..=0x2135 \| 0x2137 \| 0x2149..=0x214e \| 0x2157 \| 0x215b..=0x215f \| 0x2161..=0x2162 \| 0x2164..=0x2167 \| 0x2169..=0x2177 \| 0x217e \| 0x2184..=0x2187 \| 0x2197..=0x219f \| 0x21a8 \| 0x21ad \| 0x21b3..=0x21b5 \| 0x21b8..=0x21bf \| 0x21c5..=0x21c7 \| 0x21cd..=0x21d3 \| 0x21d6..=0x21eb \| 0x21ee..=0x21ff \| 0x2202..=0x221f )` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [7](../../crates/crysirror-cmvs/src/vm/cvns.rs#L7) |

### vm/evaluate.rs / binary_value

```rust
fn binary_value(
    pc: usize,
    opcode: u16,
    left: i32,
    right: i32,
) -> Result<i32, DecodeError>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `opcode` | `0x160` | Local state/value operations | Handled | [1075](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1075) |
| `opcode` | `0x161` | Local state/value operations | Handled | [1076](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1076) |
| `opcode` | `0x162` | Local state/value operations | Handled | [1079](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1079) |
| `opcode` | `0x163` | Local state/value operations | Handled | [1082](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1082) |
| `opcode` | `0x164` | Local state/value operations | Handled | [1083](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1083) |
| `opcode` | `0x165` | Local state/value operations | Handled | [1084](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1084) |
| `opcode` | `0x166` | Local state/value operations | Handled | [1085](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1085) |
| `opcode` | `0x167` | Local state/value operations | Handled | [1086](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1086) |
| `opcode` | `0x168` | Local state/value operations | Handled | [1087](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1087) |
| `opcode` | `0x169` | Local state/value operations | Handled | [1088](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1088) |
| `opcode` | `0x16a` | i32::from | Handled | [1089](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1089) |
| `opcode` | `0x16b` | i32::from | Handled | [1090](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1090) |
| `opcode` | `0x16c` | i32::from | Handled | [1091](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1091) |
| `opcode` | `0x16d` | i32::from | Handled | [1092](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1092) |
| `opcode` | `0x16e` | i32::from | Handled | [1093](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1093) |
| `opcode` | `0x16f` | i32::from | Handled | [1094](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1094) |
| `opcode` | `0x171` | i32::from | Handled | [1095](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1095) |
| `opcode` | `0x172` | i32::from | Handled | [1096](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1096) |
| `opcode` | `_` | Local state/value operations | Unsupported: unsupported integer operator | [1097](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1097) |

### vm/evaluate.rs / float_binary

```rust
fn float_binary(pc: usize, opcode: u16, left: f32, right: f32) -> Result<f32, DecodeError>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `opcode` | `0x160` | Local state/value operations | Handled | [1036](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1036) |
| `opcode` | `0x161` | Local state/value operations | Handled | [1037](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1037) |
| `opcode` | `0x163` | Local state/value operations | Handled | [1038](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1038) |
| `opcode` | `0x164` | Local state/value operations | Handled | [1039](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1039) |
| `opcode` | `0x162 \| 0x165..=0x169` | Local state/value operations | Handled | [1040](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1040) |
| `opcode` | `0x16a` | Local state/value operations | Handled | [1041](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1041) |
| `opcode` | `0x16b` | Local state/value operations | Handled | [1042](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1042) |
| `opcode` | `0x16c` | Local state/value operations | Handled | [1043](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1043) |
| `opcode` | `0x16d` | Local state/value operations | Handled | [1044](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1044) |
| `opcode` | `0x16e` | Local state/value operations | Handled | [1045](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1045) |
| `opcode` | `0x16f` | Local state/value operations | Handled | [1046](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1046) |
| `opcode` | `0x171` | Local state/value operations | Handled | [1047](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1047) |
| `opcode` | `0x172` | Local state/value operations | Handled | [1048](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1048) |
| `opcode` | `_` | Local state/value operations | Unsupported: unsupported float operator | [1049](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1049) |

### vm/evaluate.rs / State::evaluate

```rust
fn evaluate(
        &mut self,
        expr: &Expression,
        depth: usize,
        env: &mut Env,
        journal: &mut Journal,
    ) -> Result<i32, DecodeError>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `expr.kind != 0x200` | Local state/value operations | Unsupported: not an integer expression | [621](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L621) |
| `condition; under term=Term::Operator { pc, opcode: op @ (0x173 \| 0x174), }` | `*op == 0x173` | Local state/value operations | Handled | [704](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L704) |
| `condition; under term=Term::Operator { pc, opcode }` | `*opcode == 0x170` | self.read, self.write | Handled | [719](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L719) |
| `opcode; under term=Term::Operator { pc, opcode }` | `0x175` | Local state/value operations | Handled | [728](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L728) |
| `opcode; under term=Term::Operator { pc, opcode }` | `0x176` | Local state/value operations | Handled | [729](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L729) |
| `opcode; under term=Term::Operator { pc, opcode }` | `0x177` | Local state/value operations | Handled | [730](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L730) |
| `opcode; under term=Term::Operator { pc, opcode }` | `0x178` | Local state/value operations | Handled | [731](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L731) |
| `opcode; under term=Term::Operator { pc, opcode }` | `0x179` | Local state/value operations | Handled | [732](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L732) |
| `opcode; under term=Term::Operator { pc, opcode }` | `0x17a` | Local state/value operations | Handled | [733](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L733) |
| `opcode; under term=Term::Operator { pc, opcode }` | `0x17b` | Local state/value operations | Handled | [734](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L734) |
| `opcode; under term=Term::Operator { pc, opcode }` | `0x17c` | Local state/value operations | Handled | [735](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L735) |
| `opcode; under term=Term::Operator { pc, opcode }` | `0x17d` | Local state/value operations | Handled | [736](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L736) |
| `opcode; under term=Term::Operator { pc, opcode }` | `0x17e` | Local state/value operations | Handled | [737](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L737) |
| `opcode; under term=Term::Operator { pc, opcode }` | `other` | Local state/value operations | Handled | [738](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L738) |
| `condition; under term=Term::Operator { pc, opcode }` | `(0x175..=0x17e).contains(opcode)` | self.write | Handled | [741](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L741) |

### vm/evaluate.rs / State::evaluate_float_in

```rust
fn evaluate_float_in(
        &mut self,
        expr: &Expression,
        env: &mut Env,
        journal: &mut Journal,
    ) -> Result<f32, DecodeError>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `expr.kind != 0x202` | Local state/value operations | Unsupported: not a float expression | [873](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L873) |
| `condition; under term=Term::Operator { pc, opcode: op @ (0x173 \| 0x174), }` | `*op == 0x173` | Local state/value operations | Handled | [957](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L957) |
| `condition; under term=Term::Operator { pc, opcode }` | `*opcode == 0x170` | self.read_float, self.write_float | Handled | [968](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L968) |
| `condition; under term=Term::Operator { pc, opcode }` | `(0x175..=0x17e).contains(opcode)` | self.read_float, self.write_float | Handled | [972](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L972) |
| `opcode; under term=Term::Operator { pc, opcode }` | `0x175` | Local state/value operations | Handled | [977](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L977) |
| `opcode; under term=Term::Operator { pc, opcode }` | `0x176` | Local state/value operations | Handled | [978](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L978) |
| `opcode; under term=Term::Operator { pc, opcode }` | `0x177` | Local state/value operations | Handled | [979](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L979) |
| `opcode; under term=Term::Operator { pc, opcode }` | `0x178` | Local state/value operations | Handled | [980](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L980) |
| `opcode; under term=Term::Operator { pc, opcode }` | `0x179` | Local state/value operations | Handled | [981](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L981) |
| `opcode; under term=Term::Operator { pc, opcode }` | `0x17a` | Local state/value operations | Handled | [982](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L982) |
| `opcode; under term=Term::Operator { pc, opcode }` | `0x17b` | Local state/value operations | Handled | [983](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L983) |
| `opcode; under term=Term::Operator { pc, opcode }` | `0x17c` | Local state/value operations | Handled | [984](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L984) |
| `opcode; under term=Term::Operator { pc, opcode }` | `0x17d` | Local state/value operations | Handled | [985](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L985) |
| `opcode; under term=Term::Operator { pc, opcode }` | `_` | Local state/value operations | Handled | [986](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L986) |

### vm/gesture.rs / PointerGesture::command

```rust
fn command(
        &mut self,
        machine: &mut Machine,
        opcode: u16,
        input: &Actions,
    ) -> Result<Option<u16>, DecodeError>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `opcode; under opcode=0x2350 \| 0x2352 \| 0x2356..=0x2358 \| 0x2364` | `0x2350` | Local state/value operations | Handled | [331](../../crates/crysirror-cmvs/src/vm/gesture.rs#L331) |
| `opcode; under opcode=0x2350 \| 0x2352 \| 0x2356..=0x2358 \| 0x2364` | `0x2352` | Local state/value operations | Handled | [332](../../crates/crysirror-cmvs/src/vm/gesture.rs#L332) |
| `opcode; under opcode=0x2350 \| 0x2352 \| 0x2356..=0x2358 \| 0x2364` | `0x2364` | Local state/value operations | Handled | [333](../../crates/crysirror-cmvs/src/vm/gesture.rs#L333) |
| `opcode; under opcode=0x2350 \| 0x2352 \| 0x2356..=0x2358 \| 0x2364` | `_` | Local state/value operations | Handled | [334](../../crates/crysirror-cmvs/src/vm/gesture.rs#L334) |
| `condition; under opcode=0x2353 \| 0x2360` | `opcode == 0x2353` | Local state/value operations | Handled | [372](../../crates/crysirror-cmvs/src/vm/gesture.rs#L372) |
| `condition; under opcode=0x235a \| 0x235b` | `opcode == 0x235a && (1..=90).contains(&value)` | self.set | Handled | [385](../../crates/crysirror-cmvs/src/vm/gesture.rs#L385) |
| `condition; under opcode=0x235a \| 0x235b` | `opcode == 0x235b && value as i32 >= 0` | self.set | Handled | [387](../../crates/crysirror-cmvs/src/vm/gesture.rs#L387) |
| `opcode` | `0x2350 \| 0x2352 \| 0x2356..=0x2358 \| 0x2364` | self.set, i32::from, self.reset_tracking | Handled | [329](../../crates/crysirror-cmvs/src/vm/gesture.rs#L329) |
| `opcode` | `0x2351` | self.set, i32::from, self.reset_tracking | Handled | [348](../../crates/crysirror-cmvs/src/vm/gesture.rs#L348) |
| `opcode` | `0x2354` | self.reset_tracking | Handled | [356](../../crates/crysirror-cmvs/src/vm/gesture.rs#L356) |
| `opcode` | `0x2355` | self.get, i32::from | Handled | [361](../../crates/crysirror-cmvs/src/vm/gesture.rs#L361) |
| `opcode` | `0x2353 \| 0x2360` | self.get | Handled | [371](../../crates/crysirror-cmvs/src/vm/gesture.rs#L371) |
| `opcode` | `0x2359` | self.get, self.set, i32::from | Handled | [375](../../crates/crysirror-cmvs/src/vm/gesture.rs#L375) |
| `opcode` | `0x235a \| 0x235b` | self.set | Handled | [383](../../crates/crysirror-cmvs/src/vm/gesture.rs#L383) |
| `opcode` | `0x2362` | i32::from, self.get | Handled | [392](../../crates/crysirror-cmvs/src/vm/gesture.rs#L392) |
| `opcode` | `0x2365` | self.get, i32::from | Handled | [396](../../crates/crysirror-cmvs/src/vm/gesture.rs#L396) |
| `opcode` | `0x2366` | u32::from, self.set | Handled | [407](../../crates/crysirror-cmvs/src/vm/gesture.rs#L407) |
| `opcode` | `0x2367` | self.set | Handled | [418](../../crates/crysirror-cmvs/src/vm/gesture.rs#L418) |
| `opcode` | `0x2368` | self.get | Handled | [422](../../crates/crysirror-cmvs/src/vm/gesture.rs#L422) |
| `opcode` | `0x2369` | i32::from, self.drag_press | Handled | [429](../../crates/crysirror-cmvs/src/vm/gesture.rs#L429) |
| `opcode` | `0x236a` | self.get | Handled | [434](../../crates/crysirror-cmvs/src/vm/gesture.rs#L434) |
| `opcode` | `0x236b` | self.set | Handled | [443](../../crates/crysirror-cmvs/src/vm/gesture.rs#L443) |
| `opcode` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [450](../../crates/crysirror-cmvs/src/vm/gesture.rs#L450) |

### vm/history.rs / History::command

```rust
fn command(
        &mut self,
        machine: &mut Machine,
        opcode: u16,
    ) -> Result<u16, DecodeError>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under opcode=0x2131..=0x2135` | `opcode == 0x2135` | Local state/value operations | Handled | [117](../../crates/crysirror-cmvs/src/vm/history.rs#L117) |
| `opcode` | `0x212e` | Local state/value operations | Handled | [103](../../crates/crysirror-cmvs/src/vm/history.rs#L103) |
| `opcode` | `0x2137` | Local state/value operations | Handled | [104](../../crates/crysirror-cmvs/src/vm/history.rs#L104) |
| `opcode` | `0x212f` | self.entries.clear | Handled | [108](../../crates/crysirror-cmvs/src/vm/history.rs#L108) |
| `opcode` | `0x2130` | self.entries.len | Handled | [112](../../crates/crysirror-cmvs/src/vm/history.rs#L112) |
| `opcode` | `0x2131..=0x2135` | self.entries.get, StringRef::decode, usize::from, Journal::default | Handled | [113](../../crates/crysirror-cmvs/src/vm/history.rs#L113) |
| `opcode` | `0x2136` | Default::default, self.entries.len, self.entries.pop_back, self.entries.push_front | Handled | [138](../../crates/crysirror-cmvs/src/vm/history.rs#L138) |
| `opcode` | `_` | Local state/value operations | Handled | [158](../../crates/crysirror-cmvs/src/vm/history.rs#L158) |

### vm/input.rs / host_key

```rust
fn host_key(code: u16) -> u64
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `code` | `1` | Local state/value operations | Handled | [258](../../crates/crysirror-cmvs/src/vm/input.rs#L258) |
| `code` | `2` | Local state/value operations | Handled | [259](../../crates/crysirror-cmvs/src/vm/input.rs#L259) |
| `code` | `3` | Local state/value operations | Handled | [260](../../crates/crysirror-cmvs/src/vm/input.rs#L260) |
| `code` | `4` | Local state/value operations | Handled | [261](../../crates/crysirror-cmvs/src/vm/input.rs#L261) |
| `code` | `5` | Local state/value operations | Handled | [262](../../crates/crysirror-cmvs/src/vm/input.rs#L262) |
| `code` | `20` | Local state/value operations | Handled | [263](../../crates/crysirror-cmvs/src/vm/input.rs#L263) |
| `code` | `21` | Local state/value operations | Handled | [264](../../crates/crysirror-cmvs/src/vm/input.rs#L264) |
| `code` | `22` | Local state/value operations | Handled | [265](../../crates/crysirror-cmvs/src/vm/input.rs#L265) |
| `code` | `23` | Local state/value operations | Handled | [266](../../crates/crysirror-cmvs/src/vm/input.rs#L266) |
| `code` | `24` | Local state/value operations | Handled | [267](../../crates/crysirror-cmvs/src/vm/input.rs#L267) |
| `code` | `25` | Local state/value operations | Handled | [268](../../crates/crysirror-cmvs/src/vm/input.rs#L268) |
| `code` | `26` | Local state/value operations | Handled | [269](../../crates/crysirror-cmvs/src/vm/input.rs#L269) |
| `code` | `27` | Local state/value operations | Handled | [270](../../crates/crysirror-cmvs/src/vm/input.rs#L270) |
| `code` | `28` | Local state/value operations | Handled | [271](../../crates/crysirror-cmvs/src/vm/input.rs#L271) |
| `code` | `29` | Local state/value operations | Handled | [272](../../crates/crysirror-cmvs/src/vm/input.rs#L272) |
| `code` | `30` | Local state/value operations | Handled | [273](../../crates/crysirror-cmvs/src/vm/input.rs#L273) |
| `code` | `31` | Local state/value operations | Handled | [274](../../crates/crysirror-cmvs/src/vm/input.rs#L274) |
| `code` | `32` | Local state/value operations | Handled | [275](../../crates/crysirror-cmvs/src/vm/input.rs#L275) |
| `code` | `33` | Local state/value operations | Handled | [276](../../crates/crysirror-cmvs/src/vm/input.rs#L276) |
| `code` | `34` | Local state/value operations | Handled | [277](../../crates/crysirror-cmvs/src/vm/input.rs#L277) |
| `code` | `35` | Local state/value operations | Handled | [278](../../crates/crysirror-cmvs/src/vm/input.rs#L278) |
| `code` | `36` | Local state/value operations | Handled | [279](../../crates/crysirror-cmvs/src/vm/input.rs#L279) |
| `code` | `52..=63` | Local state/value operations | Handled | [280](../../crates/crysirror-cmvs/src/vm/input.rs#L280) |
| `code` | `76..=85` | Local state/value operations | Handled | [281](../../crates/crysirror-cmvs/src/vm/input.rs#L281) |
| `code` | `_` | Local state/value operations | Handled | [282](../../crates/crysirror-cmvs/src/vm/input.rs#L282) |

### vm/input.rs / Actions::begin_frame

```rust
fn begin_frame(&mut self) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `self.function_keys_enabled && pressed & ((key::F12 << 1) - key::F1) != 0` | key::F1.trailing_zeros | Handled | [140](../../crates/crysirror-cmvs/src/vm/input.rs#L140) |

### vm/input.rs / Actions::command

```rust
fn command(
        &mut self,
        machine: &mut Machine,
        opcode: u16,
        script_skip: bool,
    ) -> Result<bool, DecodeError>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `opcode` | `_` | u32::from, self.consume | Handled | [232](../../crates/crysirror-cmvs/src/vm/input.rs#L232) |

### vm/input.rs / Actions::configure

```rust
fn configure(&mut self, bytes: &[u8]) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Some(index) = name .trim() .strip_prefix("KEY_DATA_") .and_then(parse_index::<5>)` | Local state/value operations | Handled | [79](../../crates/crysirror-cmvs/src/vm/input.rs#L79) |

### vm/machine.rs / Machine::expression

```rust
fn expression(&mut self, expr: &Expression) -> Result<(), DecodeError>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `expr.kind` | `0x200` | self.numeric | Handled | [606](../../crates/crysirror-cmvs/src/vm/machine.rs#L606) |
| `expr.kind` | `0x201` | self.string_expression | Handled | [609](../../crates/crysirror-cmvs/src/vm/machine.rs#L609) |
| `expr.kind` | `0x202` | self.numeric | Handled | [610](../../crates/crysirror-cmvs/src/vm/machine.rs#L610) |
| `expr.kind` | `_` | Local state/value operations | Unsupported: expression type/operation not implemented | [613](../../crates/crysirror-cmvs/src/vm/machine.rs#L613) |

### vm/machine.rs / Machine::step

```rust
fn step(&mut self) -> Result<Step, DecodeError>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under &insn.kind=InstructionKind::ConditionalJump { target }` | `(op == 0x401 && !condition) \|\| (op == 0x402 && condition)` | self.branch_target | Handled | [682](../../crates/crysirror-cmvs/src/vm/machine.rs#L682) |
| `condition; under &insn.kind=InstructionKind::CompareJump { value, target }` | `self .state .result .ok_or_else(\|\| error(pc, op, "result register is unknown"))? == *value` | self.branch_target | Handled | [687](../../crates/crysirror-cmvs/src/vm/machine.rs#L687) |
| `condition; under &insn.kind=InstructionKind::Label { index }` | `op == 0x410` | self.enter_frame | Handled | [704](../../crates/crysirror-cmvs/src/vm/machine.rs#L704) |
| `&insn.kind` | `InstructionKind::Expression(expr)` | self.expression | Handled | [673](../../crates/crysirror-cmvs/src/vm/machine.rs#L673) |
| `&insn.kind` | `InstructionKind::Jump { target } \| InstructionKind::LongJump { target, .. }` | self.branch_target | Handled | [674](../../crates/crysirror-cmvs/src/vm/machine.rs#L674) |
| `&insn.kind` | `InstructionKind::ConditionalJump { target }` | self.branch_target | Handled | [677](../../crates/crysirror-cmvs/src/vm/machine.rs#L677) |
| `&insn.kind` | `InstructionKind::CompareJump { value, target }` | self.branch_target | Handled | [686](../../crates/crysirror-cmvs/src/vm/machine.rs#L686) |
| `&insn.kind` | `InstructionKind::Label { index }` | self.enter_frame | Handled | [696](../../crates/crysirror-cmvs/src/vm/machine.rs#L696) |
| `&insn.kind` | `InstructionKind::StackAdjust { bytes } if op == 0x412` | Local state/value operations | Handled | [708](../../crates/crysirror-cmvs/src/vm/machine.rs#L708) |
| `&insn.kind` | `InstructionKind::StackAdjust { bytes } if op == 0x411` | self.return_from_frame | Handled | [713](../../crates/crysirror-cmvs/src/vm/machine.rs#L713) |
| `&insn.kind` | `InstructionKind::Simple if op == 0x413` | self.return_from_frame | Handled | [716](../../crates/crysirror-cmvs/src/vm/machine.rs#L716) |
| `&insn.kind` | `InstructionKind::Simple if op == 0x416` | self.branch_target, self.enter_frame | Handled | [723](../../crates/crysirror-cmvs/src/vm/machine.rs#L723) |
| `&insn.kind` | `InstructionKind::Simple if op == 0x430` | self.stack.push_u32 | Handled | [731](../../crates/crysirror-cmvs/src/vm/machine.rs#L731) |
| `&insn.kind` | `InstructionKind::StackImmediate { bytes, raw }` | usize::from | Handled | [738](../../crates/crysirror-cmvs/src/vm/machine.rs#L738) |
| `&insn.kind` | `InstructionKind::ScriptReturn { .. }` | self.stack.peek_u32, self.is_boundary, self.cross_calls.last, self.stack.discard, self.cross_calls.pop | Handled | [743](../../crates/crysirror-cmvs/src/vm/machine.rs#L743) |
| `&insn.kind` | `InstructionKind::System { command: 0 }` | Local state/value operations | Handled | [758](../../crates/crysirror-cmvs/src/vm/machine.rs#L758) |
| `&insn.kind` | `&InstructionKind::System { command }` | Local state/value operations | Handled | [763](../../crates/crysirror-cmvs/src/vm/machine.rs#L763) |
| `&insn.kind` | `_` | Local state/value operations | Unsupported: instruction execution is not implemented | [767](../../crates/crysirror-cmvs/src/vm/machine.rs#L767) |

### vm/machine.rs / Machine::string_expression_in

```rust
fn string_expression_in(
        &mut self,
        expr: &Expression,
        journal: &mut Journal,
    ) -> Result<(), DecodeError>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under term=Term::Operator { pc, opcode } if (0x160..=0x172).contains(opcode)` | `*opcode == 0x170` | self.append_operand, self.assign_string | Handled | [532](../../crates/crysirror-cmvs/src/vm/machine.rs#L532) |

### vm/native_state.rs / NativeState::command

```rust
fn command(
        &mut self,
        machine: &mut Machine,
        lists: &mut [Vec<Vec<u8>>; 2],
        opcode: u16,
    ) -> Result<Option<SystemResult>, DecodeError>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `opcode` | `0x2006` | Local state/value operations | Handled | [114](../../crates/crysirror-cmvs/src/vm/native_state.rs#L114) |
| `opcode` | `0x200a` | Local state/value operations | Handled | [118](../../crates/crysirror-cmvs/src/vm/native_state.rs#L118) |
| `opcode` | `0x20e8` | Local state/value operations | Handled | [122](../../crates/crysirror-cmvs/src/vm/native_state.rs#L122) |
| `opcode` | `0x2082` | Local state/value operations | Handled | [126](../../crates/crysirror-cmvs/src/vm/native_state.rs#L126) |
| `opcode` | `0x2120 \| 0x2121` | usize::from, u32::from | Handled | [132](../../crates/crysirror-cmvs/src/vm/native_state.rs#L132) |
| `opcode` | `0x2122 \| 0x2123` | usize::from | Handled | [139](../../crates/crysirror-cmvs/src/vm/native_state.rs#L139) |
| `opcode` | `0x212a \| 0x212b` | usize::from | Handled | [143](../../crates/crysirror-cmvs/src/vm/native_state.rs#L143) |
| `opcode` | `0x212c` | Local state/value operations | Handled | [147](../../crates/crysirror-cmvs/src/vm/native_state.rs#L147) |
| `opcode` | `0x229c \| 0x229d` | usize::from | Handled | [151](../../crates/crysirror-cmvs/src/vm/native_state.rs#L151) |
| `opcode` | `0x22cf` | Local state/value operations | Handled | [155](../../crates/crysirror-cmvs/src/vm/native_state.rs#L155) |
| `opcode` | `0x23b2` | Local state/value operations | Handled | [159](../../crates/crysirror-cmvs/src/vm/native_state.rs#L159) |
| `opcode` | `0x216b if machine.profile() == VmProfile::Pc20211007X64` | Local state/value operations | Handled | [164](../../crates/crysirror-cmvs/src/vm/native_state.rs#L164) |
| `opcode` | `0x22c0 \| 0x22c1 if machine.profile() == VmProfile::Pc20211007X64` | Local state/value operations | Handled | [168](../../crates/crysirror-cmvs/src/vm/native_state.rs#L168) |
| `opcode` | `0x22ff if machine.profile() == VmProfile::Pc20211007X64` | Local state/value operations | Handled | [172](../../crates/crysirror-cmvs/src/vm/native_state.rs#L172) |
| `opcode` | `0x216b` | Local state/value operations | Handled | [178](../../crates/crysirror-cmvs/src/vm/native_state.rs#L178) |
| `opcode` | `0x22c0 \| 0x22c1` | Local state/value operations | Handled | [185](../../crates/crysirror-cmvs/src/vm/native_state.rs#L185) |
| `opcode` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [189](../../crates/crysirror-cmvs/src/vm/native_state.rs#L189) |

### vm/noop.rs / implemented_in_20211007

```rust
fn implemented_in_20211007(opcode: u16) -> bool
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!( opcode, 0x20c4..=0x20c6 \| 0x20e4 \| 0x20e7 \| 0x20ef \| 0x211d \| 0x2268..=0x226e \| 0x22ed \| 0x22f9 \| 0x23ac \| 0x23ae \| 0x23b8..=0x23bb \| 0x23c0..=0x23c2 \| 0x23c5..=0x23c7 )` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [8](../../crates/crysirror-cmvs/src/vm/noop.rs#L8) |

### vm/noop.rs / returns_without_effect

```rust
fn returns_without_effect(opcode: u16) -> bool
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!( opcode, 0x2024 \| 0x2036 \| 0x203a \| 0x204d \| 0x2052..=0x2053 \| 0x2057 \| 0x2076..=0x2077 \| 0x2079..=0x207c \| 0x207e..=0x207f \| 0x2084..=0x2086 \| 0x208d..=0x208e \| 0x2095..=0x2097 \| 0x209a..=0x209b \| 0x209d..=0x209f \| 0x20bb \| 0x20be..=0x20bf \| 0x20c3..=0x20c7 \| 0x20ce..=0x20cf \| 0x20dd..=0x20df \| 0x20e1..=0x20e7 \| 0x20ea..=0x20eb \| 0x20ed..=0x20ef \| 0x20fe \| 0x211b \| 0x211d \| 0x2124..=0x2125 \| 0x2127 \| 0x212d \| 0x2138..=0x2139 \| 0x213e..=0x213f \| 0x216a \| 0x2172..=0x2175 \| 0x217d \| 0x2188 \| 0x218a \| 0x218c \| 0x218e..=0x218f \| 0x2198..=0x2199 \| 0x219b \| 0x21a5 \| 0x21a7 \| 0x21be..=0x21bf \| 0x21c4 \| 0x21c9 \| 0x21ce \| 0x21d3 \| 0x21d8..=0x21dd \| 0x21fe..=0x220b \| 0x220d..=0x220f \| 0x221c..=0x221f \| 0x2245..=0x2246 \| 0x224c..=0x224e \| 0x2251..=0x225f \| 0x2265..=0x226f \| 0x2276..=0x227f \| 0x228f \| 0x2298..=0x229b \| 0x229e..=0x229f \| 0x22aa..=0x22ad \| 0x22bf \| 0x22c2..=0x22c3 \| 0x22cc \| 0x22ed \| 0x22f6..=0x22f7 \| 0x22f9..=0x22fb \| 0x22fe \| 0x2302..=0x2303 \| 0x2307 \| 0x2318..=0x2319 \| 0x231f \| 0x2323..=0x2327 \| 0x232a..=0x232f \| 0x2333..=0x2337 \| 0x233b..=0x233f \| 0x2341..=0x2343 \| 0x234c \| 0x235e..=0x235f \| 0x2363 \| 0x236c..=0x236f \| 0x2395 \| 0x2399 \| 0x23a1..=0x23a3 \| 0x23a9..=0x23b1 \| 0x23b3..=0x23b4 \| 0x23b6 \| 0x23b8..=0x23cf )` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [27](../../crates/crysirror-cmvs/src/vm/noop.rs#L27) |

### vm/resources.rs / ResourceBindings::set_root

```rust
fn set_root(&mut self, opcode: u16, path: &[u8]) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `opcode == 0x2015` | VfsPath::new | Handled | [93](../../crates/crysirror-cmvs/src/vm/resources.rs#L93) |

### vm/screen.rs / Screen::advance_tween

```rust
fn advance_tween(&mut self, delta: u32) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `tween.mode` | `1` | Local state/value operations | Handled | [138](../../crates/crysirror-cmvs/src/vm/screen.rs#L138) |
| `tween.mode` | `2` | Local state/value operations | Handled | [139](../../crates/crysirror-cmvs/src/vm/screen.rs#L139) |
| `tween.mode` | `_` | Local state/value operations | Handled | [140](../../crates/crysirror-cmvs/src/vm/screen.rs#L140) |

### vm/screen.rs / Screen::command

```rust
fn command(
        &mut self,
        machine: &mut Machine,
        opcode: u16,
    ) -> Result<Option<u16>, DecodeError>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `opcode` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [246](../../crates/crysirror-cmvs/src/vm/screen.rs#L246) |
| `condition` | `matches!(opcode, 0x2060..=0x2065 \| 0x2068)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [248](../../crates/crysirror-cmvs/src/vm/screen.rs#L248) |
| `opcode` | `_` | Local state/value operations | Handled | [252](../../crates/crysirror-cmvs/src/vm/screen.rs#L252) |
| `opcode` | `_` | Local state/value operations | Handled | [303](../../crates/crysirror-cmvs/src/vm/screen.rs#L303) |

### vm/screen.rs / Screen::start_tween

```rust
fn start_tween(
        &mut self,
        machine: &Machine,
        opcode: u16,
    ) -> Result<u16, DecodeError>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `opcode` | `0x206b` | Local state/value operations | Handled | [175](../../crates/crysirror-cmvs/src/vm/screen.rs#L175) |
| `opcode` | `0x206c` | Local state/value operations | Handled | [180](../../crates/crysirror-cmvs/src/vm/screen.rs#L180) |
| `opcode` | `0x206d` | Local state/value operations | Handled | [184](../../crates/crysirror-cmvs/src/vm/screen.rs#L184) |
| `opcode` | `0x206e` | Local state/value operations | Handled | [188](../../crates/crysirror-cmvs/src/vm/screen.rs#L188) |
| `opcode` | `0x206f` | Local state/value operations | Handled | [192](../../crates/crysirror-cmvs/src/vm/screen.rs#L192) |
| `opcode` | `_` | f32::from_bits | Handled | [196](../../crates/crysirror-cmvs/src/vm/screen.rs#L196) |

### vm/sequence.rs / Kind::of

```rust
fn of(code: u32) -> Self
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `code` | `1` | Local state/value operations | Handled | [38](../../crates/crysirror-cmvs/src/vm/sequence.rs#L38) |
| `code` | `3` | Local state/value operations | Handled | [39](../../crates/crysirror-cmvs/src/vm/sequence.rs#L39) |
| `code` | `4` | Local state/value operations | Handled | [40](../../crates/crysirror-cmvs/src/vm/sequence.rs#L40) |
| `code` | `5` | Local state/value operations | Handled | [41](../../crates/crysirror-cmvs/src/vm/sequence.rs#L41) |
| `code` | `other` | Self::Other | Handled | [42](../../crates/crysirror-cmvs/src/vm/sequence.rs#L42) |

### vm/startup.rs / CmvsVm::complete_title

```rust
fn complete_title(&mut self) -> Result<(), DecodeError>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `command == 0x160` | Local state/value operations | Handled | [439](../../crates/crysirror-cmvs/src/vm/startup.rs#L439) |

### vm/startup.rs / CmvsVm::fill_variables

```rust
fn fill_variables(&mut self, opcode: u16) -> Result<(), DecodeError>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `opcode` | `0x2091` | self.machine.state_mut | Handled | [1810](../../crates/crysirror-cmvs/src/vm/startup.rs#L1810) |
| `opcode` | `0x2092` | Local state/value operations | Handled | [1811](../../crates/crysirror-cmvs/src/vm/startup.rs#L1811) |
| `opcode` | `0x2093` | self.machine.state_mut | Handled | [1815](../../crates/crysirror-cmvs/src/vm/startup.rs#L1815) |
| `opcode` | `0x2094` | self.machine.resolve_string, self.machine.state_mut | Handled | [1816](../../crates/crysirror-cmvs/src/vm/startup.rs#L1816) |
| `opcode` | `_` | Local state/value operations | Handled | [1820](../../crates/crysirror-cmvs/src/vm/startup.rs#L1820) |

### vm/startup.rs / CmvsVm::image_name

```rust
fn image_name(&self, opcode: u16, reference: u32) -> Result<String, DecodeError>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `name.len() >= 64` | self.pc | Unsupported: CMVS image name byte limit | [767](../../crates/crysirror-cmvs/src/vm/startup.rs#L767) |

### vm/startup.rs / CmvsVm::owner_arguments

```rust
fn owner_arguments(
        &self,
        opcode: u16,
    ) -> Result<[u32; N], DecodeError>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `alias.is_none() && !matches!(self.machine.pending_system(), Some((_, op)) if op + 0x2000 == opcode)` | self.pc | Unsupported: CMVS owner command differs | [1017](../../crates/crysirror-cmvs/src/vm/startup.rs#L1017) |

### vm/startup.rs / CmvsVm::pending_effect_sound

```rust
fn pending_effect_sound(&self) -> Result<EffectSound, DecodeError>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under command=0xb0 \| 0xba` | `name.len() >= 32` | Local state/value operations | Unsupported: CMVS effect sound name byte limit | [842](../../crates/crysirror-cmvs/src/vm/startup.rs#L842) |
| `condition; under command=0xb0 \| 0xba` | `command == 0xba` | Local state/value operations | Handled | [852](../../crates/crysirror-cmvs/src/vm/startup.rs#L852) |
| `condition; under command=0xb0 \| 0xba` | `command == 0xba` | Local state/value operations | Handled | [855](../../crates/crysirror-cmvs/src/vm/startup.rs#L855) |
| `condition; under command=0xb0 \| 0xba` | `command == 0xba` | self.required_parameter | Handled | [857](../../crates/crysirror-cmvs/src/vm/startup.rs#L857) |
| `command` | `0xb0 \| 0xba` | self.required_parameter | Handled | [838](../../crates/crysirror-cmvs/src/vm/startup.rs#L838) |
| `command` | `0xb1` | self.required_parameter | Handled | [864](../../crates/crysirror-cmvs/src/vm/startup.rs#L864) |
| `command` | `_` | Local state/value operations | Handled | [868](../../crates/crysirror-cmvs/src/vm/startup.rs#L868) |

### vm/startup.rs / CmvsVm::pending_fade

```rust
fn pending_fade(&self) -> Result<FadeCommand, DecodeError>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `command` | `0x220` | Local state/value operations | Handled | [1305](../../crates/crysirror-cmvs/src/vm/startup.rs#L1305) |
| `command` | `0x221` | Local state/value operations | Handled | [1318](../../crates/crysirror-cmvs/src/vm/startup.rs#L1318) |
| `command` | `0x222` | Local state/value operations | Handled | [1319](../../crates/crysirror-cmvs/src/vm/startup.rs#L1319) |
| `command` | `0x223` | Local state/value operations | Handled | [1320](../../crates/crysirror-cmvs/src/vm/startup.rs#L1320) |
| `command` | `_` | Local state/value operations | Handled | [1321](../../crates/crysirror-cmvs/src/vm/startup.rs#L1321) |

### vm/startup.rs / CmvsVm::pending_scene_object

```rust
fn pending_scene_object(&self) -> Result<(SceneObject, u16), DecodeError>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `command == 0x2f` | Local state/value operations | Handled | [911](../../crates/crysirror-cmvs/src/vm/startup.rs#L911) |
| `condition` | `matches!(command, 0x22 \| 0x23 \| 0x44..=0x48 \| 0x50)` | Local state/value operations | Handled | [920](../../crates/crysirror-cmvs/src/vm/startup.rs#L920) |
| `condition` | `value < 0 && matches!(command, 0x44..=0x48)` | Local state/value operations | Handled | [922](../../crates/crysirror-cmvs/src/vm/startup.rs#L922) |
| `command; under command=_` | `0x44` | DrawProperty::Crop | Handled | [960](../../crates/crysirror-cmvs/src/vm/startup.rs#L960) |
| `command; under command=_` | `0x45` | DrawProperty::Position | Handled | [969](../../crates/crysirror-cmvs/src/vm/startup.rs#L969) |
| `command; under command=_` | `0x46` | DrawProperty::Origin | Handled | [973](../../crates/crysirror-cmvs/src/vm/startup.rs#L973) |
| `command; under command=_` | `0x47` | DrawProperty::Priority | Handled | [974](../../crates/crysirror-cmvs/src/vm/startup.rs#L974) |
| `command; under command=_` | `_` | DrawProperty::Opacity | Handled | [975](../../crates/crysirror-cmvs/src/vm/startup.rs#L975) |
| `command` | `0x20..=0x23` | Local state/value operations | Handled | [933](../../crates/crysirror-cmvs/src/vm/startup.rs#L933) |
| `command` | `0x30` | self.machine.resolve_string | Handled | [941](../../crates/crysirror-cmvs/src/vm/startup.rs#L941) |
| `command` | `0x40 \| 0x50` | Local state/value operations | Handled | [950](../../crates/crysirror-cmvs/src/vm/startup.rs#L950) |
| `command` | `_` | DrawProperty::Crop, DrawProperty::Position, DrawProperty::Origin, DrawProperty::Priority, DrawProperty::Opacity | Handled | [958](../../crates/crysirror-cmvs/src/vm/startup.rs#L958) |

### vm/startup.rs / CmvsVm::pending_script

```rust
fn pending_script(&self) -> Result<Option<ScriptRequest>, DecodeError>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `command == 0x81` | self.required_parameter | Handled | [475](../../crates/crysirror-cmvs/src/vm/startup.rs#L475) |
| `condition` | `command == 0x03` | Local state/value operations | Handled | [484](../../crates/crysirror-cmvs/src/vm/startup.rs#L484) |
| `condition` | `name.is_empty() \|\| name.len() >= 64` | Local state/value operations | Unsupported: script basename exceeds native bounds | [490](../../crates/crysirror-cmvs/src/vm/startup.rs#L490) |

### vm/startup.rs / CmvsVm::pending_title

```rust
fn pending_title(&self) -> Result<Option<String>, DecodeError>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `command == 0x161` | Local state/value operations | Handled | [411](../../crates/crysirror-cmvs/src/vm/startup.rs#L411) |

### vm/startup.rs / CmvsVm::prepare_message_read

```rust
fn prepare_message_read(
        &mut self,
        id: u32,
        only_read: bool,
    ) -> Result<bool, DecodeError>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `only_read && id as i32 >= 0` | self.pc | Handled | [721](../../crates/crysirror-cmvs/src/vm/startup.rs#L721) |

### vm/startup.rs / CmvsVm::run_slice

```rust
fn run_slice(&mut self, budget: &mut u32) -> Result<VmYield, DecodeError>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `opcode; under self.machine.step()?=Step::System { pc, command }` | `0x2002 \| 0x2003` | Local state/value operations | Handled | [1488](../../crates/crysirror-cmvs/src/vm/startup.rs#L1488) |
| `opcode; under self.machine.step()?=Step::System { pc, command }` | `_` | Local state/value operations | Unsupported: unreviewed CVNS native syscall | [1489](../../crates/crysirror-cmvs/src/vm/startup.rs#L1489) |
| `condition; under self.machine.step()?=Step::System { pc, command }` | `opcode == 0x208a` | self.control.invoke | Handled | [1492](../../crates/crysirror-cmvs/src/vm/startup.rs#L1492) |

### vm/startup.rs / CmvsVm::sound_name

```rust
fn sound_name(&self, raw: u32) -> Result<Option<String>, DecodeError>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `name.is_empty() \|\| (matches!(opcode, 0x2240 \| 0x2248) && name.len() >= 32)` | Local state/value operations | Handled | [1116](../../crates/crysirror-cmvs/src/vm/startup.rs#L1116) |
| `condition` | `matches!(opcode, 0x2240 \| 0x2248)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [1116](../../crates/crysirror-cmvs/src/vm/startup.rs#L1116) |
| `condition` | `name.len() >= 32` | self.pc | Unsupported: CMVS sound name byte limit | [1119](../../crates/crysirror-cmvs/src/vm/startup.rs#L1119) |

### vm/startup.rs / CmvsVm::startup_command

```rust
fn startup_command(&mut self, opcode: u16) -> Result<Option<SystemResult>, DecodeError>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `opcode; under opcode=0x201d..=0x201f` | `0x201d` | Local state/value operations | Handled | [1519](../../crates/crysirror-cmvs/src/vm/startup.rs#L1519) |
| `opcode; under opcode=0x201d..=0x201f` | `0x201e` | Local state/value operations | Handled | [1520](../../crates/crysirror-cmvs/src/vm/startup.rs#L1520) |
| `opcode; under opcode=0x201d..=0x201f` | `_` | Local state/value operations | Handled | [1521](../../crates/crysirror-cmvs/src/vm/startup.rs#L1521) |
| `opcode; under opcode=0x2268..=0x226e if self.machine.profile() == VmProfile::Pc20211007X64` | `0x2268` | Local state/value operations | Handled | [1552](../../crates/crysirror-cmvs/src/vm/startup.rs#L1552) |
| `opcode; under opcode=0x2268..=0x226e if self.machine.profile() == VmProfile::Pc20211007X64` | `0x2269` | Local state/value operations | Handled | [1553](../../crates/crysirror-cmvs/src/vm/startup.rs#L1553) |
| `opcode; under opcode=0x2268..=0x226e if self.machine.profile() == VmProfile::Pc20211007X64` | `0x226a` | Local state/value operations | Handled | [1554](../../crates/crysirror-cmvs/src/vm/startup.rs#L1554) |
| `opcode; under opcode=0x2268..=0x226e if self.machine.profile() == VmProfile::Pc20211007X64` | `0x226b` | Local state/value operations | Handled | [1555](../../crates/crysirror-cmvs/src/vm/startup.rs#L1555) |
| `opcode; under opcode=0x2268..=0x226e if self.machine.profile() == VmProfile::Pc20211007X64` | `0x226c` | Local state/value operations | Handled | [1556](../../crates/crysirror-cmvs/src/vm/startup.rs#L1556) |
| `opcode; under opcode=0x2268..=0x226e if self.machine.profile() == VmProfile::Pc20211007X64` | `0x226d` | Local state/value operations | Handled | [1557](../../crates/crysirror-cmvs/src/vm/startup.rs#L1557) |
| `opcode; under opcode=0x2268..=0x226e if self.machine.profile() == VmProfile::Pc20211007X64` | `_` | Local state/value operations | Handled | [1558](../../crates/crysirror-cmvs/src/vm/startup.rs#L1558) |
| `condition; under opcode=0x2010..=0x2014 \| 0x2017 \| 0x2018 \| 0x2019 \| 0x201a if opcode != 0x2019 \|\| self.machine.profile() == VmProfile::Pc20160201X64` | `opcode == 0x2011` | Local state/value operations | Handled | [1579](../../crates/crysirror-cmvs/src/vm/startup.rs#L1579) |
| `condition; under opcode=0x2098` | `name.len() >= 128 \|\| self.cursors.len() >= 1024` | Local state/value operations | Unsupported: cursor registration limit | [1606](../../crates/crysirror-cmvs/src/vm/startup.rs#L1606) |
| `condition; under opcode=0x2350..=0x235b \| 0x2360 \| 0x2362 \| 0x2364..=0x236b` | `opcode == 0x2350` | self.required_parameter | Handled | [1618](../../crates/crysirror-cmvs/src/vm/startup.rs#L1618) |
| `condition; under opcode=0x2281 \| 0x2285` | `opcode == 0x2285` | self.sequence.count | Handled | [1675](../../crates/crysirror-cmvs/src/vm/startup.rs#L1675) |
| `condition; under opcode=0x2286..=0x228b / opcode=0x2288 \| 0x228b` | `opcode == 0x2288` | Local state/value operations | Handled | [1706](../../crates/crysirror-cmvs/src/vm/startup.rs#L1706) |
| `condition; under opcode=0x2286..=0x228b / opcode=_` | `opcode == 0x2289` | Local state/value operations | Handled | [1712](../../crates/crysirror-cmvs/src/vm/startup.rs#L1712) |
| `opcode; under opcode=0x2286..=0x228b` | `0x2286` | self.sequence.push | Handled | [1693](../../crates/crysirror-cmvs/src/vm/startup.rs#L1693) |
| `opcode; under opcode=0x2286..=0x228b` | `0x2287` | self.sequence.push | Handled | [1698](../../crates/crysirror-cmvs/src/vm/startup.rs#L1698) |
| `opcode; under opcode=0x2286..=0x228b` | `0x2288 \| 0x228b` | self.sequence.push | Handled | [1703](../../crates/crysirror-cmvs/src/vm/startup.rs#L1703) |
| `opcode; under opcode=0x2286..=0x228b` | `_` | self.sequence.push | Handled | [1710](../../crates/crysirror-cmvs/src/vm/startup.rs#L1710) |
| `condition; under opcode=0x22e5 \| 0x22eb` | `opcode == 0x22e5` | Local state/value operations | Handled | [1741](../../crates/crysirror-cmvs/src/vm/startup.rs#L1741) |
| `condition; under opcode=0x22e5 \| 0x22eb` | `opcode == 0x22e5` | Local state/value operations | Handled | [1747](../../crates/crysirror-cmvs/src/vm/startup.rs#L1747) |
| `condition; under opcode=0x22ce \| 0x22ee` | `opcode == 0x22ce` | Local state/value operations | Handled | [1761](../../crates/crysirror-cmvs/src/vm/startup.rs#L1761) |
| `condition; under opcode=_` | `(revision && (0x23d0..=0x23df).contains(&opcode)) \|\| (super::noop::returns_without_effect(opcode) && !(revision && super::noop::implemented_in_20211007(opcode))) \|\| self .actions .command(&mut self.machine, opcode, self.skipping)?` | Local state/value operations | Handled | [1774](../../crates/crysirror-cmvs/src/vm/startup.rs#L1774) |
| `opcode` | `0x23ac if self.machine.profile() == VmProfile::Pc20211007X64` | self.required_parameter | Handled | [1534](../../crates/crysirror-cmvs/src/vm/startup.rs#L1534) |
| `opcode` | `0x2019 \| 0x209c if self.machine.profile() == VmProfile::Pc20211007X64` | Local state/value operations | Handled | [1539](../../crates/crysirror-cmvs/src/vm/startup.rs#L1539) |
| `opcode` | `0x23ae if self.machine.profile() == VmProfile::Pc20211007X64` | Local state/value operations | Handled | [1540](../../crates/crysirror-cmvs/src/vm/startup.rs#L1540) |
| `opcode` | `0x2268..=0x226e if self.machine.profile() == VmProfile::Pc20211007X64` | self.required_parameter, self.pc, self.machine.resolve_string | Handled | [1541](../../crates/crysirror-cmvs/src/vm/startup.rs#L1541) |
| `opcode` | `0x2010..=0x2014 \| 0x2017 \| 0x2018 \| 0x2019 \| 0x201a if opcode != 0x2019 \|\| self.machine.profile() == VmProfile::Pc20160201X64` | self.required_parameter, self.pc, self.machine.resolve_string | Handled | [1573](../../crates/crysirror-cmvs/src/vm/startup.rs#L1573) |
| `opcode` | `_` | self.machine.profile, super::noop::returns_without_effect, super::noop::implemented_in_20211007, self.control.command, self.screen.command | Handled | [1772](../../crates/crysirror-cmvs/src/vm/startup.rs#L1772) |

### vm/strings.rs / command

```rust
fn command(vm: &mut Machine, opcode: u16) -> Result<u16, DecodeError>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under opcode=0x20f0 \| 0x20f2` | `opcode == 0x20f2` | Local state/value operations | Handled | [17](../../crates/crysirror-cmvs/src/vm/strings.rs#L17) |
| `condition; under opcode=0x20f7 \| 0x20f9` | `opcode == 0x20f7` | Local state/value operations | Handled | [94](../../crates/crysirror-cmvs/src/vm/strings.rs#L94) |
| `opcode; under opcode=0x20fa \| 0x20fc \| 0x20fd` | `0x20fa` | FormatArg::Integer | Handled | [105](../../crates/crysirror-cmvs/src/vm/strings.rs#L105) |
| `opcode; under opcode=0x20fa \| 0x20fc \| 0x20fd` | `0x20fc` | FormatArg::Integer | Handled | [106](../../crates/crysirror-cmvs/src/vm/strings.rs#L106) |
| `opcode; under opcode=0x20fa \| 0x20fc \| 0x20fd` | `_` | FormatArg::String | Handled | [107](../../crates/crysirror-cmvs/src/vm/strings.rs#L107) |
| `condition; under opcode=0x20fa \| 0x20fc \| 0x20fd` | `opcode == 0x20fa` | Local state/value operations | Handled | [109](../../crates/crysirror-cmvs/src/vm/strings.rs#L109) |
| `condition; under opcode=0x20fa \| 0x20fc \| 0x20fd` | `(opcode != 0x20fa && overlaps(target, 0, output.len() + 1, arg(8)?, 0, pattern.len() + 1)) \|\| matches!(value, FormatArg::String(source) if overlaps(target, 0, output.len() + 1, arg(12)?, 0, source.len() + 1))` | Local state/value operations | Unsupported: overlapping CMVS format operands | [115](../../crates/crysirror-cmvs/src/vm/strings.rs#L115) |
| `condition; under opcode=0x20fa \| 0x20fc \| 0x20fd` | `opcode == 0x20fa` | Local state/value operations | Handled | [123](../../crates/crysirror-cmvs/src/vm/strings.rs#L123) |
| `opcode` | `0x20f0 \| 0x20f2` | Local state/value operations | Handled | [13](../../crates/crysirror-cmvs/src/vm/strings.rs#L13) |
| `opcode` | `0x20f1` | u32::from | Handled | [30](../../crates/crysirror-cmvs/src/vm/strings.rs#L30) |
| `opcode` | `0x20f3` | usize::try_from | Handled | [34](../../crates/crysirror-cmvs/src/vm/strings.rs#L34) |
| `opcode` | `0x20f4` | usize::from | Handled | [73](../../crates/crysirror-cmvs/src/vm/strings.rs#L73) |
| `opcode` | `0x20f5` | u32::from | Handled | [85](../../crates/crysirror-cmvs/src/vm/strings.rs#L85) |
| `opcode` | `0x20f6` | Local state/value operations | Handled | [89](../../crates/crysirror-cmvs/src/vm/strings.rs#L89) |
| `opcode` | `0x20f7 \| 0x20f9` | Local state/value operations | Handled | [90](../../crates/crysirror-cmvs/src/vm/strings.rs#L90) |
| `opcode` | `0x20f8` | Local state/value operations | Handled | [101](../../crates/crysirror-cmvs/src/vm/strings.rs#L101) |
| `opcode` | `0x20fa \| 0x20fc \| 0x20fd` | FormatArg::Integer, FormatArg::String | Handled | [102](../../crates/crysirror-cmvs/src/vm/strings.rs#L102) |
| `opcode` | `0x20fb` | i32::from | Handled | [125](../../crates/crysirror-cmvs/src/vm/strings.rs#L125) |
| `opcode` | `0x20ff` | Local state/value operations | Handled | [143](../../crates/crysirror-cmvs/src/vm/strings.rs#L143) |
| `opcode` | `_` | Local state/value operations | Handled | [159](../../crates/crysirror-cmvs/src/vm/strings.rs#L159) |

### vm/strings.rs / decimal

```rust
fn decimal(kind: u8, value: u32, sign: Option<u8>, width: usize) -> Result<Vec<u8>, &'static str>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `kind == b'd' && sign.is_some() && signed < 0` | Local state/value operations | Handled | [474](../../crates/crysirror-cmvs/src/vm/strings.rs#L474) |

### vm/strings.rs / sprintf

```rust
fn sprintf(
    pattern: &[u8],
    argument: FormatArg<'_>,
    read_destination: impl Fn(usize) -> Result<u8, &'static str>,
) -> Result<Vec<u8>, &'static str>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(kind, b'd' \| b'u' \| b'z' \| b'x' \| b'o' \| b's' \| b'f' \| b't')` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [280](../../crates/crysirror-cmvs/src/vm/strings.rs#L280) |
| `condition` | `matches!(kind, b's' \| b'c')` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [292](../../crates/crysirror-cmvs/src/vm/strings.rs#L292) |
