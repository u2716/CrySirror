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
| Handled | 900 |
| Open | 62 |
| Original no-op | 484 |
| Partial | 3 |
| Unreviewed | 98 |

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
| `0x2000` | Advance and yield | Handled | Guard: x86 case RVA `0x32563` | [machine.rs:800](../../crates/crysirror-cmvs/src/vm/machine.rs#L800) |
| `0x2001` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2002` | Set the native frame period in milliseconds | Handled | Guard: x86 case RVA `0x3256a` | [display.rs:147](../../crates/crysirror-cmvs/src/adapter/display.rs#L147) |
| `0x2003` | Read the native frame period | Handled | Guard: x86 case RVA `0x32572` | [adapter.rs:1943](../../crates/crysirror-cmvs/src/adapter.rs#L1943) |
| `0x2004` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x3257a` | [mod.rs:1500](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1500) |
| `0x2005` | Soft reset the native scene and enter main.pss while preserving the game banks | Handled | Guard: x86 case RVA `0x32582` | [adapter.rs:2230](../../crates/crysirror-cmvs/src/adapter.rs#L2230) |
| `0x2006` | Shares the reviewed 2016 route 0x2001 | Handled | Guard: x86 case RVA `0x3258d` | [cvns.rs:82](../../crates/crysirror-cmvs/src/vm/cvns.rs#L82) |
| `0x2007` | Read the completed one-second frame counter | Handled | Guard: x86 case RVA `0x32595` | [adapter.rs:1900](../../crates/crysirror-cmvs/src/adapter.rs#L1900) |
| `0x2008` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x325a5` | [mod.rs:537](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L537) |
| `0x2009` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x325ad` | [mod.rs:542](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L542) |
| `0x200A` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x325b5` | [mod.rs:549](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L549) |
| `0x200B` | Fill the retained primary and schedule full damage | Handled | Guard: x86 case RVA `0x325bd` | [mod.rs:554](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L554) |
| `0x200C` | Submit a clipped redraw rectangle | Handled | Guard: x86 case RVA `0x325c5` | [mod.rs:559](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L559) |
| `0x200D` | Cancel queued redraw rectangles | Handled | Guard: x86 case RVA `0x325cd` | [mod.rs:564](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L564) |
| `0x200E` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x200F` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2010` | Replace the main script and capture its entry state in CSP1 | Handled | Guard: x86 case RVA `0x325df` | [adapter.rs:2231](../../crates/crysirror-cmvs/src/adapter.rs#L2231) |
| `0x2011` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2012` | Call a script in image slot 1 | Handled | Guard: x86 case RVA `0x325e7` | [adapter.rs:2232](../../crates/crysirror-cmvs/src/adapter.rs#L2232) |
| `0x2013` | Shares the reviewed 2016 route 0x2082 | Handled | Guard: x86 case RVA `0x325ef` | [cvns.rs:17](../../crates/crysirror-cmvs/src/vm/cvns.rs#L17) |
| `0x2014` | VM-owned CVNS selector | Handled | Guard: x86 case RVA `0x325f7` | [cvns.rs:163](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L163) |
| `0x2015` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2016` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2017` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2018` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x325ff` | [mod.rs:1513](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1513) |
| `0x2019` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32607` | [mod.rs:1514](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1514) |
| `0x201A` | Shares the reviewed 2016 route 0x200B | Handled | Guard: x86 case RVA `0x3260f` | [cvns.rs:61](../../crates/crysirror-cmvs/src/vm/cvns.rs#L61) |
| `0x201B` | Shares the reviewed 2016 route 0x200C | Handled | Guard: x86 case RVA `0x32617` | [cvns.rs:62](../../crates/crysirror-cmvs/src/vm/cvns.rs#L62) |
| `0x201C` | Shares the reviewed 2016 route 0x200D | Handled | Guard: x86 case RVA `0x3261f` | [cvns.rs:19](../../crates/crysirror-cmvs/src/vm/cvns.rs#L19) |
| `0x201D` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x201E` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x201F` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32627` | [mod.rs:1505](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1505) |
| `0x2020` | VM-owned CVNS selector | Handled | Guard: x86 case RVA `0x3262f` | [cvns.rs:140](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L140) |
| `0x2021` | VM-owned CVNS selector | Handled | Guard: x86 case RVA `0x32637` | [cvns.rs:140](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L140) |
| `0x2022` | VM-owned CVNS selector | Handled | Guard: x86 case RVA `0x3263f` | [cvns.rs:140](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L140) |
| `0x2023` | VM-owned CVNS selector | Handled | Guard: x86 case RVA `0x32647` | [cvns.rs:140](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L140) |
| `0x2024` | VM-owned CVNS selector | Handled | Guard: x86 case RVA `0x3264f` | [cvns.rs:140](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L140) |
| `0x2025` | VM-owned CVNS selector | Handled | Guard: x86 case RVA `0x32657` | [cvns.rs:152](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L152) |
| `0x2026` | VM-owned CVNS selector | Handled | Guard: x86 case RVA `0x3265f` | [cvns.rs:152](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L152) |
| `0x2027` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2028` | VM-owned CVNS selector | Handled | Guard: x86 case RVA `0x32667` | [cvns.rs:19](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L19) |
| `0x2029` | VM-owned CVNS selector | Handled | Guard: x86 case RVA `0x3266f` | [cvns.rs:19](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L19) |
| `0x202A` | VM-owned CVNS selector | Handled | Guard: x86 case RVA `0x32677` | [cvns.rs:19](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L19) |
| `0x202B` | Shares the reviewed 2016 route 0x208A | Handled | Guard: x86 case RVA `0x3267f` | [cvns.rs:53](../../crates/crysirror-cmvs/src/vm/cvns.rs#L53) |
| `0x202C` | VM-owned CVNS selector | Handled | Guard: x86 case RVA `0x32687` | [cvns.rs:19](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L19) |
| `0x202D` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x202E` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x202F` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2030` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x3268f` | [mod.rs:568](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L568) |
| `0x2031` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32697` | [mod.rs:573](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L573) |
| `0x2032` | Shares the reviewed 2016 route 0x20CB | Handled | Guard: x86 case RVA `0x3269f` | [cvns.rs:92](../../crates/crysirror-cmvs/src/vm/cvns.rs#L92) |
| `0x2033` | Shares the reviewed 2016 route 0x20CC | Handled | Guard: x86 case RVA `0x326a7` | [cvns.rs:90](../../crates/crysirror-cmvs/src/vm/cvns.rs#L90) |
| `0x2034` | Shares the reviewed 2016 route 0x20CD | Handled | Guard: x86 case RVA `0x326af` | [cvns.rs:94](../../crates/crysirror-cmvs/src/vm/cvns.rs#L94) |
| `0x2035` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2036` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2037` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2038` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x326b7` | [mod.rs:580](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L580) |
| `0x2039` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x326ca` | [mod.rs:581](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L581) |
| `0x203A` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x326db` | [mod.rs:580](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L580) |
| `0x203B` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x326ee` | [mod.rs:581](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L581) |
| `0x203C` | Shares the reviewed 2016 route 0x23B2 | Handled | Guard: x86 case RVA `0x326ff` | [cvns.rs:23](../../crates/crysirror-cmvs/src/vm/cvns.rs#L23) |
| `0x203D` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32710` | [mod.rs:585](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L585) |
| `0x203E` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x203F` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2040` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x3272a` | [mod.rs:626](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L626) |
| `0x2041` | Set native fault bit 0 after consuming the selector argument | Handled | Guard: x86 case RVA `0x32732` | [adapter.rs:2215](../../crates/crysirror-cmvs/src/adapter.rs#L2215) |
| `0x2042` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x3273a` | [mod.rs:586](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L586) |
| `0x2043` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2044` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2045` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2046` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2047` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32742` | [mod.rs:607](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L607) |
| `0x2048` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x3274a` | [mod.rs:496](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L496) |
| `0x2049` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32752` | [mod.rs:501](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L501) |
| `0x204A` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x3275a` | [mod.rs:501](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L501) |
| `0x204B` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32762` | [mod.rs:501](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L501) |
| `0x204C` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x3276a` | [mod.rs:501](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L501) |
| `0x204D` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x204E` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x204F` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2050` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32772` | [mod.rs:662](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L662) |
| `0x2051` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x3277a` | [mod.rs:631](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L631) |
| `0x2052` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32782` | [mod.rs:631](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L631) |
| `0x2053` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x3278a` | [mod.rs:631](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L631) |
| `0x2054` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2055` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32792` | [mod.rs:711](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L711) |
| `0x2056` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x3279a` | [mod.rs:717](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L717) |
| `0x2057` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x327a2` | [mod.rs:662](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L662) |
| `0x2058` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x327aa` | [mod.rs:662](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L662) |
| `0x2059` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x327b2` | [mod.rs:723](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L723) |
| `0x205A` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x327ba` | [mod.rs:754](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L754) |
| `0x205B` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x327c2` | [mod.rs:763](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L763) |
| `0x205C` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x327ca` | [mod.rs:662](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L662) |
| `0x205D` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x327d2` | [mod.rs:662](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L662) |
| `0x205E` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x327da` | [mod.rs:662](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L662) |
| `0x205F` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x327e2` | [mod.rs:662](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L662) |
| `0x2060` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2061` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2062` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x327ea` | [mod.rs:662](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L662) |
| `0x2063` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x327f2` | [mod.rs:662](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L662) |
| `0x2064` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2065` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2066` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x327fa` | [mod.rs:662](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L662) |
| `0x2067` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32802` | [mod.rs:662](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L662) |
| `0x2068` | Shares the reviewed 2016 route 0x22CD | Handled | Guard: x86 case RVA `0x3280a` | [cvns.rs:59](../../crates/crysirror-cmvs/src/vm/cvns.rs#L59) |
| `0x2069` | VM-owned CVNS selector | Handled | Guard: x86 case RVA `0x32812` | [cvns.rs:104](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L104) |
| `0x206A` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x206B` | VM-owned CVNS selector | Handled | Guard: x86 case RVA `0x32825` | [cvns.rs:39](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L39) |
| `0x206C` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32836` | [mod.rs:611](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L611) |
| `0x206D` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32847` | [mod.rs:611](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L611) |
| `0x206E` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x206F` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2070` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32858` | [mod.rs:796](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L796) |
| `0x2071` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32860` | [mod.rs:796](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L796) |
| `0x2072` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32868` | [mod.rs:796](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L796) |
| `0x2073` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2074` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32870` | [mod.rs:796](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L796) |
| `0x2075` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32878` | [mod.rs:820](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L820) |
| `0x2076` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32880` | [mod.rs:824](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L824) |
| `0x2077` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32888` | [mod.rs:848](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L848) |
| `0x2078` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32890` | [mod.rs:890](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L890) |
| `0x2079` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32898` | [mod.rs:895](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L895) |
| `0x207A` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x328a0` | [mod.rs:911](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L911) |
| `0x207B` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x328a8` | [mod.rs:928](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L928) |
| `0x207C` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x328b0` | [mod.rs:928](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L928) |
| `0x207D` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x328b8` | [mod.rs:928](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L928) |
| `0x207E` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x328c0` | [mod.rs:947](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L947) |
| `0x207F` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x328c8` | [mod.rs:911](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L911) |
| `0x2080` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x328d0` | [mod.rs:928](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L928) |
| `0x2081` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x328d8` | [mod.rs:911](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L911) |
| `0x2082` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x328e0` | [mod.rs:928](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L928) |
| `0x2083` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x328e8` | [mod.rs:952](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L952) |
| `0x2084` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x328f0` | [mod.rs:957](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L957) |
| `0x2085` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x328f8` | [mod.rs:968](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L968) |
| `0x2086` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32900` | [mod.rs:963](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L963) |
| `0x2087` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2088` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32908` | [mod.rs:977](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L977) |
| `0x2089` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x208A` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x208B` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x208C` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x208D` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x208E` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x208F` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2090` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32910` | [mod.rs:985](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L985) |
| `0x2091` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32918` | [mod.rs:985](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L985) |
| `0x2092` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32920` | [mod.rs:1015](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1015) |
| `0x2093` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32928` | [mod.rs:1026](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1026) |
| `0x2094` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32930` | [mod.rs:1037](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1037) |
| `0x2095` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2096` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2097` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2098` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2099` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x209A` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x209B` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x209C` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x209D` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x209E` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x209F` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x20A0` | Shares the reviewed 2016 route 0x20A0 | Handled | Guard: x86 case RVA `0x32938` | [cvns.rs:79](../../crates/crysirror-cmvs/src/vm/cvns.rs#L79) |
| `0x20A1` | Shares the reviewed 2016 route 0x20A1 | Handled | Guard: x86 case RVA `0x32940` | [cvns.rs:79](../../crates/crysirror-cmvs/src/vm/cvns.rs#L79) |
| `0x20A2` | Shares the reviewed 2016 route 0x20A2 | Handled | Guard: x86 case RVA `0x32952` | [cvns.rs:79](../../crates/crysirror-cmvs/src/vm/cvns.rs#L79) |
| `0x20A3` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x20A4` | Read music playback time after the native buffer lead | Handled | Guard: x86 case RVA `0x3295a` | [adapter.rs:2180](../../crates/crysirror-cmvs/src/adapter.rs#L2180) |
| `0x20A5` | VM-owned CVNS selector | Handled | Guard: x86 case RVA `0x32962` | [cvns.rs:22](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L22) |
| `0x20A6` | VM-owned CVNS selector | Handled | Guard: x86 case RVA `0x3296a` | [cvns.rs:31](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L31) |
| `0x20A7` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x20A8` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x20A9` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x20AA` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x20AB` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x20AC` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x20AD` | Shares the reviewed 2016 route 0x20BA | Handled | Guard: x86 case RVA `0x32972` | [cvns.rs:67](../../crates/crysirror-cmvs/src/vm/cvns.rs#L67) |
| `0x20AE` | Shares the reviewed 2016 route 0x20B4 | Handled | Guard: x86 case RVA `0x3297a` | [cvns.rs:68](../../crates/crysirror-cmvs/src/vm/cvns.rs#L68) |
| `0x20AF` | Set signed percentage balance of one SE stream | Handled | Guard: x86 case RVA `0x32982` | [adapter.rs:2122](../../crates/crysirror-cmvs/src/adapter.rs#L2122) |
| `0x20B0` | Shares the reviewed 2016 route 0x20B0 | Handled | Guard: x86 case RVA `0x3298a` | [cvns.rs:66](../../crates/crysirror-cmvs/src/vm/cvns.rs#L66) |
| `0x20B1` | Shares the reviewed 2016 route 0x20B1 | Handled | Guard: x86 case RVA `0x32992` | [cvns.rs:66](../../crates/crysirror-cmvs/src/vm/cvns.rs#L66) |
| `0x20B2` | Shares the reviewed 2016 route 0x20B2 | Handled | Guard: x86 case RVA `0x3299a` | [cvns.rs:66](../../crates/crysirror-cmvs/src/vm/cvns.rs#L66) |
| `0x20B3` | Shares the reviewed 2016 route 0x20B3 | Handled | Guard: x86 case RVA `0x329a2` | [cvns.rs:66](../../crates/crysirror-cmvs/src/vm/cvns.rs#L66) |
| `0x20B4` | Shares the reviewed 2016 route 0x20B5 | Handled | Guard: x86 case RVA `0x329aa` | [cvns.rs:80](../../crates/crysirror-cmvs/src/vm/cvns.rs#L80) |
| `0x20B5` | Shares the reviewed 2016 route 0x20B6 | Handled | Guard: x86 case RVA `0x329b2` | [cvns.rs:69](../../crates/crysirror-cmvs/src/vm/cvns.rs#L69) |
| `0x20B6` | Read active SE playback milliseconds after the two-buffer lead | Handled | Guard: x86 case RVA `0x329ba` | [adapter.rs:2157](../../crates/crysirror-cmvs/src/adapter.rs#L2157) |
| `0x20B7` | Set secondary percentage gain of one SE stream | Handled | Guard: x86 case RVA `0x329c2` | [adapter.rs:2122](../../crates/crysirror-cmvs/src/adapter.rs#L2122) |
| `0x20B8` | Load or play a voice with a percentage gain and loop flag | Handled | Guard: x86 case RVA `0x329ca` | [sound.rs:806](../../crates/crysirror-cmvs/src/adapter/sound.rs#L806) |
| `0x20B9` | Start a prepared voice | Handled | Guard: x86 case RVA `0x329d2` | [adapter.rs:2136](../../crates/crysirror-cmvs/src/adapter.rs#L2136) |
| `0x20BA` | Stop and release a voice | Handled | Guard: x86 case RVA `0x329da` | [adapter.rs:2136](../../crates/crysirror-cmvs/src/adapter.rs#L2136) |
| `0x20BB` | Fade a voice to retirement | Handled | Guard: x86 case RVA `0x329e2` | [adapter.rs:2136](../../crates/crysirror-cmvs/src/adapter.rs#L2136) |
| `0x20BC` | Read active voice playback milliseconds after the two-buffer lead | Handled | Guard: x86 case RVA `0x329ea` | [adapter.rs:2157](../../crates/crysirror-cmvs/src/adapter.rs#L2157) |
| `0x20BD` | Set secondary percentage gain of one voice | Handled | Guard: x86 case RVA `0x329f2` | [adapter.rs:2122](../../crates/crysirror-cmvs/src/adapter.rs#L2122) |
| `0x20BE` | Set signed percentage balance of one voice | Handled | Guard: x86 case RVA `0x329fa` | [adapter.rs:2122](../../crates/crysirror-cmvs/src/adapter.rs#L2122) |
| `0x20BF` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x20C0` | Play a fullscreen movie and suspend the script until EOF or input | Handled | Guard: x86 case RVA `0x32a02` | [adapter.rs:2188](../../crates/crysirror-cmvs/src/adapter.rs#L2188) |
| `0x20C1` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x20C2` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x20C3` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x20C4` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x20C5` | Prepare or play one of four independent sound streams | Handled | Guard: x86 case RVA `0x32a0a` | [adapter.rs:2189](../../crates/crysirror-cmvs/src/adapter.rs#L2189) |
| `0x20C6` | Read the active sample stream cursor | Handled | Guard: x86 case RVA `0x32a12` | [adapter.rs:2157](../../crates/crysirror-cmvs/src/adapter.rs#L2157) |
| `0x20C7` | Release a sample stream | Handled | Guard: x86 case RVA `0x32a1a` | [adapter.rs:2198](../../crates/crysirror-cmvs/src/adapter.rs#L2198) |
| `0x20C8` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32a22` | [mod.rs:768](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L768) |
| `0x20C9` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32a2a` | [mod.rs:768](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L768) |
| `0x20CA` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32a32` | [mod.rs:768](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L768) |
| `0x20CB` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32a3a` | [mod.rs:768](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L768) |
| `0x20CC` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32a42` | [mod.rs:792](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L792) |
| `0x20CD` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x20CE` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x20CF` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x20D0` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32a4a` | [mod.rs:1059](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1059) |
| `0x20D1` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32a52` | [mod.rs:1059](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1059) |
| `0x20D2` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32a5a` | [mod.rs:1065](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1065) |
| `0x20D3` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32a62` | [mod.rs:1065](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1065) |
| `0x20D4` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32a6a` | [mod.rs:1065](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1065) |
| `0x20D5` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32a72` | [mod.rs:1065](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1065) |
| `0x20D6` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32a7a` | [mod.rs:1065](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1065) |
| `0x20D7` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32a82` | [mod.rs:1065](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1065) |
| `0x20D8` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32a8a` | [mod.rs:1065](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1065) |
| `0x20D9` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32a92` | [mod.rs:1043](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1043) |
| `0x20DA` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32a9a` | [mod.rs:1043](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1043) |
| `0x20DB` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32aa2` | [mod.rs:1043](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1043) |
| `0x20DC` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32aaa` | [mod.rs:1065](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1065) |
| `0x20DD` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32ab2` | [mod.rs:1065](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1065) |
| `0x20DE` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32aba` | [mod.rs:1065](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1065) |
| `0x20DF` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32ac2` | [mod.rs:1065](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1065) |
| `0x20E0` | Shares the reviewed 2016 route 0x20D0 | Handled | Guard: x86 case RVA `0x32aca` | [cvns.rs:72](../../crates/crysirror-cmvs/src/vm/cvns.rs#L72) |
| `0x20E1` | Shares the reviewed 2016 route 0x20D1 | Handled | Guard: x86 case RVA `0x32ad2` | [cvns.rs:73](../../crates/crysirror-cmvs/src/vm/cvns.rs#L73) |
| `0x20E2` | Shares the reviewed 2016 route 0x20D2 | Handled | Guard: x86 case RVA `0x32ada` | [cvns.rs:74](../../crates/crysirror-cmvs/src/vm/cvns.rs#L74) |
| `0x20E3` | Shares the reviewed 2016 route 0x20D3 | Handled | Guard: x86 case RVA `0x32ae2` | [cvns.rs:75](../../crates/crysirror-cmvs/src/vm/cvns.rs#L75) |
| `0x20E4` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32aea` | [mod.rs:524](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L524) |
| `0x20E5` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32af2` | [mod.rs:524](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L524) |
| `0x20E6` | Shares the reviewed 2016 route 0x20D6 | Handled | Guard: x86 case RVA `0x32afa` | [cvns.rs:76](../../crates/crysirror-cmvs/src/vm/cvns.rs#L76) |
| `0x20E7` | VM-owned CVNS selector | Handled | Guard: x86 case RVA `0x32b02` | [cvns.rs:44](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L44) |
| `0x20E8` | VM-owned CVNS selector | Handled | Guard: x86 case RVA `0x32b0a` | [cvns.rs:44](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L44) |
| `0x20E9` | VM-owned CVNS selector | Handled | Guard: x86 case RVA `0x32b12` | [cvns.rs:44](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L44) |
| `0x20EA` | VM-owned CVNS selector | Handled | Guard: x86 case RVA `0x32b1a` | [cvns.rs:77](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L77) |
| `0x20EB` | Clear the vector result and set native fault bit 0 | Handled | Guard: x86 case RVA `0x32b22` | [adapter.rs:2208](../../crates/crysirror-cmvs/src/adapter.rs#L2208) |
| `0x20EC` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x20ED` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x20EE` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x20EF` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x20F0` | VM-owned CVNS selector | Handled | Guard: x86 case RVA `0x32b2a` | [cvns.rs:76](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L76) |
| `0x20F1` | VM-owned CVNS selector | Handled | Guard: x86 case RVA `0x32b32` | [cvns.rs:76](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L76) |
| `0x20F2` | VM-owned CVNS selector | Handled | Guard: x86 case RVA `0x32b3a` | [cvns.rs:76](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L76) |
| `0x20F3` | VM-owned CVNS selector | Handled | Guard: x86 case RVA `0x32b42` | [cvns.rs:76](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L76) |
| `0x20F4` | VM-owned CVNS selector | Handled | Guard: x86 case RVA `0x32b4a` | [cvns.rs:76](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L76) |
| `0x20F5` | VM-owned CVNS selector | Handled | Guard: x86 case RVA `0x32b52` | [cvns.rs:76](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L76) |
| `0x20F6` | VM-owned CVNS selector | Handled | Guard: x86 case RVA `0x32b5a` | [cvns.rs:76](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L76) |
| `0x20F7` | VM-owned CVNS selector | Handled | Guard: x86 case RVA `0x32b62` | [cvns.rs:76](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L76) |
| `0x20F8` | VM-owned CVNS selector | Handled | Guard: x86 case RVA `0x32b6a` | [cvns.rs:76](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L76) |
| `0x20F9` | VM-owned CVNS selector | Handled | Guard: x86 case RVA `0x32b72` | [cvns.rs:76](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L76) |
| `0x20FA` | VM-owned CVNS selector | Handled | Guard: x86 case RVA `0x32b7a` | [cvns.rs:76](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L76) |
| `0x20FB` | VM-owned CVNS selector | Handled | Guard: x86 case RVA `0x32b82` | [cvns.rs:76](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L76) |
| `0x20FC` | VM-owned CVNS selector | Handled | Guard: x86 case RVA `0x32b8a` | [cvns.rs:76](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L76) |
| `0x20FD` | VM-owned CVNS selector | Handled | Guard: x86 case RVA `0x32b92` | [cvns.rs:76](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L76) |
| `0x20FE` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x20FF` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2100` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32b9a` | [mod.rs:616](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L616) |
| `0x2101` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32ba2` | [mod.rs:616](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L616) |
| `0x2102` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32baa` | [mod.rs:1043](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1043) |
| `0x2103` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2104` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2105` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2106` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2107` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2108` | VM-owned CVNS selector | Handled | Guard: x86 case RVA `0x32bb2` | [cvns.rs:118](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L118) |
| `0x2109` | VM-owned CVNS selector | Handled | Guard: x86 case RVA `0x32bba` | [cvns.rs:118](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L118) |
| `0x210A` | VM-owned CVNS selector | Handled | Guard: x86 case RVA `0x32bc2` | [cvns.rs:118](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L118) |
| `0x210B` | VM-owned CVNS selector | Handled | Guard: x86 case RVA `0x32bca` | [cvns.rs:159](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L159) |
| `0x210C` | VM-owned CVNS selector | Handled | Guard: x86 case RVA `0x32bd2` | [cvns.rs:159](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L159) |
| `0x210D` | VM-owned CVNS selector | Handled | Guard: x86 case RVA `0x32bda` | [cvns.rs:159](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L159) |
| `0x210E` | VM-owned CVNS selector | Handled | Guard: x86 case RVA `0x32be2` | [cvns.rs:113](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L113) |
| `0x210F` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2110` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32bf3` | [mod.rs:1125](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1125) |
| `0x2111` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32bfb` | [mod.rs:1140](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1140) |
| `0x2112` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32c03` | [mod.rs:1158](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1158) |
| `0x2113` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32c0b` | [mod.rs:1177](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1177) |
| `0x2114` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32c13` | [mod.rs:1183](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1183) |
| `0x2115` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2116` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2117` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32c1b` | [mod.rs:1196](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1196) |
| `0x2118` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32c23` | [mod.rs:1203](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1203) |
| `0x2119` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32c2b` | [mod.rs:1183](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1183) |
| `0x211A` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32c33` | [mod.rs:1263](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1263) |
| `0x211B` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32c3b` | [mod.rs:1274](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1274) |
| `0x211C` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x211D` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32c43` | [mod.rs:1284](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1284) |
| `0x211E` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32c4b` | [mod.rs:1203](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1203) |
| `0x211F` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32c53` | [mod.rs:1289](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1289) |
| `0x2120` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32c5b` | [mod.rs:1294](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1294) |
| `0x2121` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32c63` | [mod.rs:1183](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1183) |
| `0x2122` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32c6b` | [mod.rs:1203](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1203) |
| `0x2123` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32c73` | [mod.rs:1203](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1203) |
| `0x2124` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32c7b` | [mod.rs:1183](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1183) |
| `0x2125` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32c83` | [mod.rs:1183](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1183) |
| `0x2126` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32c8b` | [mod.rs:1228](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1228) |
| `0x2127` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2128` | VM-owned CVNS selector | Handled | Guard: x86 case RVA `0x32c93` | [cvns.rs:112](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L112) |
| `0x2129` | VM-owned CVNS selector | Handled | Guard: x86 case RVA `0x32c9b` | [cvns.rs:112](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L112) |
| `0x212A` | VM-owned CVNS selector | Handled | Guard: x86 case RVA `0x32ca3` | [cvns.rs:112](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L112) |
| `0x212B` | VM-owned CVNS selector | Handled | Guard: x86 case RVA `0x32cab` | [cvns.rs:112](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L112) |
| `0x212C` | VM-owned CVNS selector | Handled | Guard: x86 case RVA `0x32cb3` | [cvns.rs:112](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L112) |
| `0x212D` | VM-owned CVNS selector | Handled | Guard: x86 case RVA `0x32cbb` | [cvns.rs:112](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L112) |
| `0x212E` | VM-owned CVNS selector | Handled | Guard: x86 case RVA `0x32cc3` | [cvns.rs:112](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L112) |
| `0x212F` | VM-owned CVNS selector | Handled | Guard: x86 case RVA `0x32ccb` | [cvns.rs:112](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L112) |
| `0x2130` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32cdd` | [mod.rs:631](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L631) |
| `0x2131` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32ce5` | [mod.rs:859](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L859) |
| `0x2132` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2133` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2134` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2135` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2136` | VM-owned CVNS selector | Handled | Guard: x86 case RVA `0x32ced` | [cvns.rs:112](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L112) |
| `0x2137` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2138` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32cf5` | [mod.rs:1309](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1309) |
| `0x2139` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32cfd` | [mod.rs:1255](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1255) |
| `0x213A` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32d05` | [mod.rs:824](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L824) |
| `0x213B` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32d0d` | [mod.rs:895](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L895) |
| `0x213C` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32d15` | [mod.rs:911](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L911) |
| `0x213D` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32d1d` | [mod.rs:928](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L928) |
| `0x213E` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32d25` | [mod.rs:867](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L867) |
| `0x213F` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32d2d` | [mod.rs:867](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L867) |
| `0x2140` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32d35` | [mod.rs:911](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L911) |
| `0x2141` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32d3d` | [mod.rs:867](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L867) |
| `0x2142` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32d45` | [mod.rs:867](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L867) |
| `0x2143` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32d4d` | [mod.rs:867](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L867) |
| `0x2144` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32d55` | [mod.rs:985](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L985) |
| `0x2145` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32d5d` | [mod.rs:985](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L985) |
| `0x2146` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32d65` | [mod.rs:1015](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1015) |
| `0x2147` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32d6d` | [mod.rs:1026](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1026) |
| `0x2148` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32d75` | [mod.rs:848](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L848) |
| `0x2149` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x214A` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x214B` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x214C` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x214D` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x214E` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x214F` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32d7d` | [mod.rs:867](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L867) |
| `0x2150` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32d85` | [mod.rs:1318](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1318) |
| `0x2151` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32d8d` | [mod.rs:1329](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1329) |
| `0x2152` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32d95` | [mod.rs:1350](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1350) |
| `0x2153` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32d9d` | [mod.rs:1336](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1336) |
| `0x2154` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32da5` | [mod.rs:1345](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1345) |
| `0x2155` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32dad` | [mod.rs:1358](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1358) |
| `0x2156` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32db5` | [mod.rs:1365](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1365) |
| `0x2157` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2158` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32dbd` | [mod.rs:450](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L450) |
| `0x2159` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32dc5` | [mod.rs:450](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L450) |
| `0x215A` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32dcd` | [mod.rs:450](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L450) |
| `0x215B` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x215C` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x215D` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x215E` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x215F` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2160` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32dd5` | [mod.rs:738](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L738) |
| `0x2161` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2162` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2163` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32ddd` | [mod.rs:743](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L743) |
| `0x2164` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2165` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2166` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2167` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2168` | Shares the reviewed 2016 route 0x208F | Handled | Guard: x86 case RVA `0x32de5` | [cvns.rs:34](../../crates/crysirror-cmvs/src/vm/cvns.rs#L34) |
| `0x2169` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x216A` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x216B` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x216C` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x216D` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x216E` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x216F` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2170` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2171` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2172` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2173` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2174` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2175` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2176` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2177` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2178` | Shares the reviewed 2016 route 0x2160 | Handled | Guard: x86 case RVA `0x32ded` | [cvns.rs:30](../../crates/crysirror-cmvs/src/vm/cvns.rs#L30) |
| `0x2179` | Shares the reviewed 2016 route 0x2161 | Handled | Guard: x86 case RVA `0x32df5` | [cvns.rs:56](../../crates/crysirror-cmvs/src/vm/cvns.rs#L56) |
| `0x217A` | Shares the reviewed 2016 route 0x2162 | Handled | Guard: x86 case RVA `0x32dfd` | [cvns.rs:57](../../crates/crysirror-cmvs/src/vm/cvns.rs#L57) |
| `0x217B` | Return zero in a window and the active native pixel-depth mode in fullscreen | Handled | Guard: x86 case RVA `0x32e05` | [display.rs:392](../../crates/crysirror-cmvs/src/adapter/display.rs#L392) |
| `0x217C` | Switch between windowed and fullscreen presentation | Handled | Guard: x86 case RVA `0x32e0d` | [adapter.rs:1908](../../crates/crysirror-cmvs/src/adapter.rs#L1908) |
| `0x217D` | Read the backing canvas dimensions into registers 1 and 2 | Handled | Guard: x86 case RVA `0x32e15` | [adapter.rs:2226](../../crates/crysirror-cmvs/src/adapter.rs#L2226) |
| `0x217E` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x217F` | Read the primary display dimensions into registers 1 and 2 | Handled | Guard: x86 case RVA `0x32e1d` | [adapter.rs:2221](../../crates/crysirror-cmvs/src/adapter.rs#L2221) |
| `0x2180` | Read the window position in desktop coordinates | Handled | Guard: x86 case RVA `0x32e25` | [adapter.rs:1931](../../crates/crysirror-cmvs/src/adapter.rs#L1931) |
| `0x2181` | Move the window in desktop coordinates and yield a native frame | Handled | Guard: x86 case RVA `0x32e2d` | [adapter.rs:1936](../../crates/crysirror-cmvs/src/adapter.rs#L1936) |
| `0x2182` | Read the minimized or maximized window state | Handled | Guard: x86 case RVA `0x32e35` | [adapter.rs:1925](../../crates/crysirror-cmvs/src/adapter.rs#L1925) |
| `0x2183` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32e3d` | [mod.rs:1512](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1512) |
| `0x2184` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2185` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2186` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2187` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2188` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32e45` | [mod.rs:1380](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1380) |
| `0x2189` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32e4d` | [mod.rs:1380](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1380) |
| `0x218A` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32e55` | [mod.rs:1398](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1398) |
| `0x218B` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32e5d` | [mod.rs:1399](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1399) |
| `0x218C` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32e65` | [mod.rs:1403](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1403) |
| `0x218D` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32e6d` | [mod.rs:1415](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1415) |
| `0x218E` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32e75` | [mod.rs:1424](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1424) |
| `0x218F` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32e7d` | [mod.rs:1442](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1442) |
| `0x2190` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32e85` | [mod.rs:1456](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1456) |
| `0x2191` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32e8d` | [mod.rs:1452](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1452) |
| `0x2192` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32e95` | [mod.rs:1462](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1462) |
| `0x2193` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32e9d` | [mod.rs:1479](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1479) |
| `0x2194` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32ea5` | [mod.rs:1483](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1483) |
| `0x2195` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32ead` | [mod.rs:1494](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1494) |
| `0x2196` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32eb5` | [mod.rs:1499](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1499) |
| `0x2197` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2198` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2199` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x219A` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x219B` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x219C` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x219D` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x219E` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x219F` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21A0` | Capture VM and resource records at the next instruction in CSP1 | Handled | Guard: x86 case RVA `0x32ebd` | [adapter.rs:669](../../crates/crysirror-cmvs/src/adapter.rs#L669) |
| `0x21A1` | Scan CSP gameplay slot headers and update load/save menu entries | Handled | Guard: x86 case RVA `0x32ec5` | [save_menu.rs:548](../../crates/crysirror-cmvs/src/vm/save_menu.rs#L548) |
| `0x21A2` | Shares the reviewed 2016 route 0x22B4 | Handled | Guard: x86 case RVA `0x32ecd` | [cvns.rs:21](../../crates/crysirror-cmvs/src/vm/cvns.rs#L21) |
| `0x21A3` | Restore a numbered CSP1 slot and resume its embedded PSS0 | Handled | Guard: x86 case RVA `0x32ed5` | [adapter.rs:1947](../../crates/crysirror-cmvs/src/adapter.rs#L1947) |
| `0x21A4` | Write the retained checkpoint, thumbnail and live main image to a numbered slot | Handled | Guard: x86 case RVA `0x32edd` | [slot.rs:51](../../crates/crysirror-cmvs/src/adapter/cvns/slot.rs#L51) |
| `0x21A5` | Shares the reviewed 2016 route 0x22B1 | Handled | Guard: x86 case RVA `0x32ee5` | [cvns.rs:40](../../crates/crysirror-cmvs/src/vm/cvns.rs#L40) |
| `0x21A6` | Shares the reviewed 2016 route 0x22B0 | Handled | Guard: x86 case RVA `0x32eed` | [cvns.rs:45](../../crates/crysirror-cmvs/src/vm/cvns.rs#L45) |
| `0x21A7` | Find the newest CSP slot after the starting index | Handled | Guard: x86 case RVA `0x32ef5` | [save_menu.rs:103](../../crates/crysirror-cmvs/src/vm/save_menu.rs#L103) |
| `0x21A8` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21A9` | Shares the reviewed 2016 route 0x22B9 | Handled | Guard: x86 case RVA `0x32efd` | [cvns.rs:42](../../crates/crysirror-cmvs/src/vm/cvns.rs#L42) |
| `0x21AA` | Shares the reviewed 2016 route 0x22BA | Handled | Guard: x86 case RVA `0x32f05` | [cvns.rs:43](../../crates/crysirror-cmvs/src/vm/cvns.rs#L43) |
| `0x21AB` | Replace the CSP comment while preserving the payload | Handled | Guard: x86 case RVA `0x32f0d` | [save_menu.rs:303](../../crates/crysirror-cmvs/src/vm/save_menu.rs#L303) |
| `0x21AC` | Read CSP metadata and optionally install its compressed thumbnail bitmap | Handled | Guard: x86 case RVA `0x32f15` | [adapter.rs:2496](../../crates/crysirror-cmvs/src/adapter.rs#L2496) |
| `0x21AD` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21AE` | Probe a numbered CSP gameplay slot signature | Handled | Guard: x86 case RVA `0x32f1d` | [save_menu.rs:163](../../crates/crysirror-cmvs/src/vm/save_menu.rs#L163) |
| `0x21AF` | Capture the displayed frame as a bottom-up BGR24 save thumbnail using the x86 row interval | Handled | Guard: x86 case RVA `0x32f25` | [thumbnail.rs:135](../../crates/crysirror-cmvs/src/adapter/thumbnail.rs#L135) |
| `0x21B0` | Shares the reviewed 2016 route 0x213A | Handled | Guard: x86 case RVA `0x32f31` | [cvns.rs:85](../../crates/crysirror-cmvs/src/vm/cvns.rs#L85) |
| `0x21B1` | Show the retry/cancel prompt and return the selected result | Handled | Guard: x86 case RVA `0x32f39` | [dialog.rs:107](../../crates/crysirror-cmvs/src/adapter/dialog.rs#L107) |
| `0x21B2` | Edit a CP932 name and copy the accepted or original text to the script destination | Handled | Guard: x86 case RVA `0x32f41` | [dialog.rs:119](../../crates/crysirror-cmvs/src/adapter/dialog.rs#L119) |
| `0x21B3` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21B4` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21B5` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21B6` | Choose the default face and rebind message windows that inherit it | Handled | Guard: x86 case RVA `0x32f49` | [dialog.rs:139](../../crates/crysirror-cmvs/src/adapter/dialog.rs#L139) |
| `0x21B7` | Choose an RGB colour using two initial custom swatches | Handled | Guard: x86 case RVA `0x32f55` | [dialog.rs:130](../../crates/crysirror-cmvs/src/adapter/dialog.rs#L130) |
| `0x21B8` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21B9` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21BA` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21BB` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21BC` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21BD` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21BE` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21BF` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21C0` | Load the CSS0 persistent bank suffixes and name lists | Handled | Guard: x86 case RVA `0x32f5d` | [css0.rs:50](../../crates/crysirror-cmvs/src/vm/css0.rs#L50) |
| `0x21C1` | Write save000.dat in CSS0 format | Handled | Guard: x86 case RVA `0x32f69` | [css0.rs:143](../../crates/crysirror-cmvs/src/vm/css0.rs#L143) |
| `0x21C2` | Shares the reviewed 2016 route 0x212A | Handled | Guard: x86 case RVA `0x32f75` | [cvns.rs:50](../../crates/crysirror-cmvs/src/vm/cvns.rs#L50) |
| `0x21C3` | Shares the reviewed 2016 route 0x212B | Handled | Guard: x86 case RVA `0x32f86` | [cvns.rs:51](../../crates/crysirror-cmvs/src/vm/cvns.rs#L51) |
| `0x21C4` | VM-owned CVNS selector | Handled | Guard: x86 case RVA `0x32f97` | [cvns.rs:35](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L35) |
| `0x21C5` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21C6` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21C7` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21C8` | Shares the reviewed 2016 route 0x2120 | Handled | Guard: x86 case RVA `0x32fa8` | [cvns.rs:25](../../crates/crysirror-cmvs/src/vm/cvns.rs#L25) |
| `0x21C9` | Shares the reviewed 2016 route 0x2121 | Handled | Guard: x86 case RVA `0x32fb0` | [cvns.rs:26](../../crates/crysirror-cmvs/src/vm/cvns.rs#L26) |
| `0x21CA` | VM-owned CVNS selector | Handled | Guard: x86 case RVA `0x32fb8` | [cvns.rs:12](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L12) |
| `0x21CB` | Shares the reviewed 2016 route 0x2122 | Handled | Guard: x86 case RVA `0x32fc0` | [cvns.rs:27](../../crates/crysirror-cmvs/src/vm/cvns.rs#L27) |
| `0x21CC` | Shares the reviewed 2016 route 0x2123 | Handled | Guard: x86 case RVA `0x32fce` | [cvns.rs:28](../../crates/crysirror-cmvs/src/vm/cvns.rs#L28) |
| `0x21CD` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21CE` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21CF` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21D0` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21D1` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21D2` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21D3` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21D4` | Shares the reviewed 2016 route 0x2098 | Handled | Guard: x86 case RVA `0x32fdc` | [cvns.rs:36](../../crates/crysirror-cmvs/src/vm/cvns.rs#L36) |
| `0x21D5` | Shares the reviewed 2016 route 0x2099 | Handled | Guard: x86 case RVA `0x32fe4` | [cvns.rs:38](../../crates/crysirror-cmvs/src/vm/cvns.rs#L38) |
| `0x21D6` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21D7` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21D8` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21D9` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21DA` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21DB` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21DC` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21DD` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21DE` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21DF` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21E0` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21E1` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21E2` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21E3` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21E4` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21E5` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21E6` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21E7` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21E8` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21E9` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21EA` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21EB` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21EC` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32fec` | [mod.rs:1515](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1515) |
| `0x21ED` | Write the indexed CVNS setting and update its live owners and menu | Handled | Guard: x86 case RVA `0x32ff4` | [adapter.rs:1859](../../crates/crysirror-cmvs/src/adapter.rs#L1859) |
| `0x21EE` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21EF` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21F0` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21F1` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21F2` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21F3` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21F4` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21F5` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21F6` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21F7` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21F8` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21F9` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21FA` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21FB` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21FC` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21FD` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21FE` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x21FF` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2200` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x32ffc` | [mod.rs:512](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L512) |
| `0x2201` | CVNS scene, text, input or settings selector | Handled | Guard: x86 case RVA `0x33004` | [mod.rs:519](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L519) |
| `0x2202` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2203` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2204` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2205` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2206` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2207` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2208` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2209` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x220A` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x220B` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x220C` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x220D` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x220E` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x220F` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2210` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2211` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2212` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2213` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2214` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2215` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2216` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2217` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2218` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x2219` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x221A` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x221B` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x221C` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x221D` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x221E` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |
| `0x221F` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x86 case RVA `0x32fd5` | [cvns.rs:106](../../crates/crysirror-cmvs/src/vm/cvns.rs#L106) |

## PC 20160201 x64 VM instructions

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x200` | Integer expression | Partial | Expression tag and operator restrictions in the ABI | [machine.rs:695](../../crates/crysirror-cmvs/src/vm/machine.rs#L695) |
| `0x201` | String expression | Partial | Expression tag and operator restrictions in the ABI | [machine.rs:695](../../crates/crysirror-cmvs/src/vm/machine.rs#L695) |
| `0x202` | Float expression | Partial | Expression tag and operator restrictions in the ABI | [machine.rs:695](../../crates/crysirror-cmvs/src/vm/machine.rs#L695) |
| `0x400` | Direct jump | Handled | — | [machine.rs:695](../../crates/crysirror-cmvs/src/vm/machine.rs#L695) |
| `0x401` | Jump on clear condition | Handled | — | [machine.rs:695](../../crates/crysirror-cmvs/src/vm/machine.rs#L695) |
| `0x402` | Jump on set condition | Handled | — | [machine.rs:695](../../crates/crysirror-cmvs/src/vm/machine.rs#L695) |
| `0x403` | Jump on raw result equality | Handled | — | [machine.rs:695](../../crates/crysirror-cmvs/src/vm/machine.rs#L695) |
| `0x405` | Jump to label | Handled | — | [machine.rs:695](../../crates/crysirror-cmvs/src/vm/machine.rs#L695) |
| `0x407` | Long jump | Handled | — | [machine.rs:695](../../crates/crysirror-cmvs/src/vm/machine.rs#L695) |
| `0x410` | Call label | Handled | — | [machine.rs:695](../../crates/crysirror-cmvs/src/vm/machine.rs#L695) |
| `0x411` | Return and discard literal byte count | Handled | — | [machine.rs:695](../../crates/crysirror-cmvs/src/vm/machine.rs#L695) |
| `0x412` | Discard literal byte count | Handled | — | [machine.rs:695](../../crates/crysirror-cmvs/src/vm/machine.rs#L695) |
| `0x413` | Return and discard result-scaled byte count | Handled | — | [machine.rs:695](../../crates/crysirror-cmvs/src/vm/machine.rs#L695) |
| `0x414` | Cross-script return | Handled | — | [machine.rs:695](../../crates/crysirror-cmvs/src/vm/machine.rs#L695) |
| `0x416` | Call result offset in current script | Handled | — | [machine.rs:695](../../crates/crysirror-cmvs/src/vm/machine.rs#L695) |
| `0x430` | Push result | Handled | — | [machine.rs:695](../../crates/crysirror-cmvs/src/vm/machine.rs#L695) |
| `0x440` | Write immediate and advance stack | Handled | — | [machine.rs:695](../../crates/crysirror-cmvs/src/vm/machine.rs#L695) |
| `0x442` | Write immediate and advance stack | Handled | — | [machine.rs:695](../../crates/crysirror-cmvs/src/vm/machine.rs#L695) |

## PC 20160201 x64 syscalls

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x2000` | Advance and yield | Handled | Guard: x64 case RVA `0x823c6`, present in the supplied script corpus | [machine.rs:800](../../crates/crysirror-cmvs/src/vm/machine.rs#L800) |
| `0x2001` | Update job | Handled | Guard: x64 case RVA `0x823d1`, present in the supplied script corpus | [adapter.rs:2389](../../crates/crysirror-cmvs/src/adapter.rs#L2389) |
| `0x2002` | Original system dispatcher entry | Open | Frame-window counter latch recovered. Its native host update boundaries are pending; Guard: x64 case RVA `0x823de` | [cmvs-20160201.json:6](../../tools/native-api/cmvs-20160201.json#L6) |
| `0x2003` | Update job | Handled | Guard: x64 case RVA `0x823f2`, present in the supplied script corpus | [adapter.rs:4814](../../crates/crysirror-cmvs/src/adapter.rs#L4814) |
| `0x2004` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x82484` | [cmvs-20160201.json:6](../../tools/native-api/cmvs-20160201.json#L6) |
| `0x2005` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x82407` | [cmvs-20160201.json:6](../../tools/native-api/cmvs-20160201.json#L6) |
| `0x2006` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x82414` | [cmvs-20160201.json:6](../../tools/native-api/cmvs-20160201.json#L6) |
| `0x2007` | Execute advance branch | Handled | Guard: x64 case RVA `0x82421` | [adapter.rs:2374](../../crates/crysirror-cmvs/src/adapter.rs#L2374) |
| `0x2008` | Control flow (command) | Handled | Guard: x64 case RVA `0x82437`, present in the supplied script corpus | [control.rs:353](../../crates/crysirror-cmvs/src/vm/control.rs#L353) |
| `0x2009` | Control flow (command) | Handled | Guard: x64 case RVA `0x8244d` | [control.rs:353](../../crates/crysirror-cmvs/src/vm/control.rs#L353) |
| `0x200A` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x82463` | [cmvs-20160201.json:6](../../tools/native-api/cmvs-20160201.json#L6) |
| `0x200B` | Control flow (command) | Handled | Guard: x64 case RVA `0x8248f`, present in the supplied script corpus | [control.rs:365](../../crates/crysirror-cmvs/src/vm/control.rs#L365) |
| `0x200C` | Control flow (command) | Handled | Guard: x64 case RVA `0x824b8`, present in the supplied script corpus | [control.rs:365](../../crates/crysirror-cmvs/src/vm/control.rs#L365) |
| `0x200D` | Execute advance branch | Handled | Guard: x64 case RVA `0x824e3` | [adapter.rs:2380](../../crates/crysirror-cmvs/src/adapter.rs#L2380) |
| `0x200E` | Control flow (command) | Handled | Guard: x64 case RVA `0x824f0`, present in the supplied script corpus | [control.rs:381](../../crates/crysirror-cmvs/src/vm/control.rs#L381) |
| `0x200F` | Control flow (command) | Handled | Guard: x64 case RVA `0x82506`, present in the supplied script corpus | [control.rs:353](../../crates/crysirror-cmvs/src/vm/control.rs#L353) |
| `0x2010` | Required parameter | Handled | Guard: x64 case RVA `0x82522`, present in the supplied script corpus | [startup.rs:1655](../../crates/crysirror-cmvs/src/vm/startup.rs#L1655) |
| `0x2011` | Required parameter | Handled | Guard: x64 case RVA `0x8252f`, present in the supplied script corpus | [startup.rs:1655](../../crates/crysirror-cmvs/src/vm/startup.rs#L1655) |
| `0x2012` | Required parameter | Handled | Guard: x64 case RVA `0x8253c`, present in the supplied script corpus | [startup.rs:1655](../../crates/crysirror-cmvs/src/vm/startup.rs#L1655) |
| `0x2013` | Required parameter | Handled | Guard: x64 case RVA `0x82549`, present in the supplied script corpus | [startup.rs:1655](../../crates/crysirror-cmvs/src/vm/startup.rs#L1655) |
| `0x2014` | Required parameter | Handled | Guard: x64 case RVA `0x82556`, present in the supplied script corpus | [startup.rs:1655](../../crates/crysirror-cmvs/src/vm/startup.rs#L1655) |
| `0x2015` | Required parameter | Handled | Guard: x64 case RVA `0x82563`, present in the supplied script corpus | [startup.rs:1675](../../crates/crysirror-cmvs/src/vm/startup.rs#L1675) |
| `0x2016` | Required parameter | Handled | Guard: x64 case RVA `0x82570`, present in the supplied script corpus | [startup.rs:1675](../../crates/crysirror-cmvs/src/vm/startup.rs#L1675) |
| `0x2017` | Required parameter | Handled | Guard: x64 case RVA `0x8257d` | [startup.rs:1655](../../crates/crysirror-cmvs/src/vm/startup.rs#L1655) |
| `0x2018` | Required parameter | Handled | Guard: x64 case RVA `0x8258a` | [startup.rs:1655](../../crates/crysirror-cmvs/src/vm/startup.rs#L1655) |
| `0x2019` | The 2021 dispatch table returns from these cases after reclaiming the arguments | Handled | Guard: x64 case RVA `0x82597`, present in the supplied script corpus | [startup.rs:1621](../../crates/crysirror-cmvs/src/vm/startup.rs#L1621) |
| `0x201A` | Required parameter | Handled | Guard: x64 case RVA `0x825a4`, present in the supplied script corpus | [startup.rs:1655](../../crates/crysirror-cmvs/src/vm/startup.rs#L1655) |
| `0x201B` | Control flow (command) | Handled | Guard: x64 case RVA `0x825b1` | [control.rs:346](../../crates/crysirror-cmvs/src/vm/control.rs#L346) |
| `0x201C` | Update job | Handled | Guard: x64 case RVA `0x825c6` | [adapter.rs:4331](../../crates/crysirror-cmvs/src/adapter.rs#L4331) |
| `0x201D` | Required parameter | Handled | Guard: x64 case RVA `0x825d3`, present in the supplied script corpus | [startup.rs:1598](../../crates/crysirror-cmvs/src/vm/startup.rs#L1598) |
| `0x201E` | Required parameter | Handled | Guard: x64 case RVA `0x825f6`, present in the supplied script corpus | [startup.rs:1598](../../crates/crysirror-cmvs/src/vm/startup.rs#L1598) |
| `0x201F` | Required parameter | Handled | Guard: x64 case RVA `0x82619`, present in the supplied script corpus | [startup.rs:1598](../../crates/crysirror-cmvs/src/vm/startup.rs#L1598) |
| `0x2020` | Update job | Handled | Guard: x64 case RVA `0x8263b`, present in the supplied script corpus | [adapter.rs:4761](../../crates/crysirror-cmvs/src/adapter.rs#L4761) |
| `0x2021` | Update job | Handled | Guard: x64 case RVA `0x82648`, present in the supplied script corpus | [adapter.rs:4761](../../crates/crysirror-cmvs/src/adapter.rs#L4761) |
| `0x2022` | Update job | Handled | Guard: x64 case RVA `0x82655`, present in the supplied script corpus | [adapter.rs:4761](../../crates/crysirror-cmvs/src/adapter.rs#L4761) |
| `0x2023` | Update job | Handled | Guard: x64 case RVA `0x82662`, present in the supplied script corpus | [adapter.rs:4761](../../crates/crysirror-cmvs/src/adapter.rs#L4761) |
| `0x2024` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2025` | Execute advance branch | Handled | Guard: x64 case RVA `0x8266f` | [adapter.rs:4513](../../crates/crysirror-cmvs/src/adapter.rs#L4513) |
| `0x2026` | Execute advance branch | Handled | Guard: x64 case RVA `0x8267c` | [adapter.rs:4513](../../crates/crysirror-cmvs/src/adapter.rs#L4513) |
| `0x2027` | Execute advance branch | Handled | Guard: x64 case RVA `0x82689`, present in the supplied script corpus | [adapter.rs:4583](../../crates/crysirror-cmvs/src/adapter.rs#L4583) |
| `0x2028` | Execute advance branch | Handled | Guard: x64 case RVA `0x82696`, present in the supplied script corpus | [adapter.rs:4704](../../crates/crysirror-cmvs/src/adapter.rs#L4704) |
| `0x2029` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x826a3` | [cmvs-20160201.json:8](../../tools/native-api/cmvs-20160201.json#L8) |
| `0x202A` | Execute advance branch | Handled | Guard: x64 case RVA `0x826b0`, present in the supplied script corpus | [adapter.rs:4743](../../crates/crysirror-cmvs/src/adapter.rs#L4743) |
| `0x202B` | Select the render target used as an object's texture | Open | 2016 target-texture source selection requires native offscreen routing; 2021 uses its object error branch; Guard: x64 case RVA `0x826bd` | [adapter.rs:3531](../../crates/crysirror-cmvs/src/adapter.rs#L3531) |
| `0x202C` | Execute advance branch | Handled | Guard: x64 case RVA `0x826ca` | [adapter.rs:4513](../../crates/crysirror-cmvs/src/adapter.rs#L4513) |
| `0x202D` | Execute advance branch | Handled | Guard: x64 case RVA `0x826d7`, present in the supplied script corpus | [adapter.rs:4513](../../crates/crysirror-cmvs/src/adapter.rs#L4513) |
| `0x202E` | Execute advance branch | Handled | Guard: x64 case RVA `0x826e4`, present in the supplied script corpus | [adapter.rs:4743](../../crates/crysirror-cmvs/src/adapter.rs#L4743) |
| `0x202F` | Update job | Handled | Guard: x64 case RVA `0x826f1`, present in the supplied script corpus | [adapter.rs:4761](../../crates/crysirror-cmvs/src/adapter.rs#L4761) |
| `0x2030` | Update job | Handled | Guard: x64 case RVA `0x826fe`, present in the supplied script corpus | [adapter.rs:4761](../../crates/crysirror-cmvs/src/adapter.rs#L4761) |
| `0x2031` | Release the image owned by a scene object | Handled | Guard: x64 case RVA `0x8270b` | [adapter.rs:4718](../../crates/crysirror-cmvs/src/adapter.rs#L4718) |
| `0x2032` | Update job | Handled | Guard: x64 case RVA `0x82718` | [adapter.rs:3393](../../crates/crysirror-cmvs/src/adapter.rs#L3393) |
| `0x2033` | Execute advance branch | Handled | Guard: x64 case RVA `0x82725`, present in the supplied script corpus | [adapter.rs:4729](../../crates/crysirror-cmvs/src/adapter.rs#L4729) |
| `0x2034` | Execute advance branch | Handled | Guard: x64 case RVA `0x82732`, present in the supplied script corpus | [adapter.rs:4254](../../crates/crysirror-cmvs/src/adapter.rs#L4254) |
| `0x2035` | Update job | Handled | Guard: x64 case RVA `0x8273f`, present in the supplied script corpus | [adapter.rs:4295](../../crates/crysirror-cmvs/src/adapter.rs#L4295) |
| `0x2036` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2037` | Update job | Handled | Guard: x64 case RVA `0x8274c` | [adapter.rs:3393](../../crates/crysirror-cmvs/src/adapter.rs#L3393) |
| `0x2038` | Update job | Handled | Guard: x64 case RVA `0x82759`, present in the supplied script corpus | [adapter.rs:3932](../../crates/crysirror-cmvs/src/adapter.rs#L3932) |
| `0x2039` | Execute advance branch | Handled | Guard: x64 case RVA `0x82766` | [adapter.rs:4513](../../crates/crysirror-cmvs/src/adapter.rs#L4513) |
| `0x203A` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x203B` | Update job | Handled | Guard: x64 case RVA `0x82773`, present in the supplied script corpus | [adapter.rs:3393](../../crates/crysirror-cmvs/src/adapter.rs#L3393) |
| `0x203C` | Execute advance branch | Handled | Guard: x64 case RVA `0x82780`, present in the supplied script corpus | [adapter.rs:3502](../../crates/crysirror-cmvs/src/adapter.rs#L3502) |
| `0x203D` | Execute advance branch | Handled | Guard: x64 case RVA `0x8278d` | [adapter.rs:3502](../../crates/crysirror-cmvs/src/adapter.rs#L3502) |
| `0x203E` | Execute advance branch | Handled | Guard: x64 case RVA `0x8279a`, present in the supplied script corpus | [adapter.rs:3502](../../crates/crysirror-cmvs/src/adapter.rs#L3502) |
| `0x203F` | Execute advance branch | Handled | Guard: x64 case RVA `0x827a7`, present in the supplied script corpus | [adapter.rs:4173](../../crates/crysirror-cmvs/src/adapter.rs#L4173) |
| `0x2040` | Update job | Handled | Guard: x64 case RVA `0x827be`, present in the supplied script corpus | [adapter.rs:4761](../../crates/crysirror-cmvs/src/adapter.rs#L4761) |
| `0x2041` | Execute advance branch | Handled | Guard: x64 case RVA `0x827cb` | [adapter.rs:4513](../../crates/crysirror-cmvs/src/adapter.rs#L4513) |
| `0x2042` | Execute advance branch | Handled | Guard: x64 case RVA `0x827d8`, present in the supplied script corpus | [adapter.rs:4513](../../crates/crysirror-cmvs/src/adapter.rs#L4513) |
| `0x2043` | Execute advance branch | Handled | Guard: x64 case RVA `0x827e5`, present in the supplied script corpus | [adapter.rs:4513](../../crates/crysirror-cmvs/src/adapter.rs#L4513) |
| `0x2044` | Update job | Handled | Guard: x64 case RVA `0x827f2`, present in the supplied script corpus | [adapter.rs:4761](../../crates/crysirror-cmvs/src/adapter.rs#L4761) |
| `0x2045` | Update job | Handled | Guard: x64 case RVA `0x827ff`, present in the supplied script corpus | [adapter.rs:4761](../../crates/crysirror-cmvs/src/adapter.rs#L4761) |
| `0x2046` | Update job | Handled | Guard: x64 case RVA `0x8280c`, present in the supplied script corpus | [adapter.rs:4761](../../crates/crysirror-cmvs/src/adapter.rs#L4761) |
| `0x2047` | Update job | Handled | Guard: x64 case RVA `0x82819`, present in the supplied script corpus | [adapter.rs:4761](../../crates/crysirror-cmvs/src/adapter.rs#L4761) |
| `0x2048` | Update job | Handled | Guard: x64 case RVA `0x82826`, present in the supplied script corpus | [adapter.rs:4761](../../crates/crysirror-cmvs/src/adapter.rs#L4761) |
| `0x2049` | Execute advance branch | Handled | Guard: x64 case RVA `0x82833`, present in the supplied script corpus | [adapter.rs:4513](../../crates/crysirror-cmvs/src/adapter.rs#L4513) |
| `0x204A` | Execute advance branch | Handled | Guard: x64 case RVA `0x82840`, present in the supplied script corpus | [adapter.rs:4513](../../crates/crysirror-cmvs/src/adapter.rs#L4513) |
| `0x204B` | Execute advance branch | Handled | Guard: x64 case RVA `0x8284d`, present in the supplied script corpus | [adapter.rs:4513](../../crates/crysirror-cmvs/src/adapter.rs#L4513) |
| `0x204C` | Execute advance branch | Handled | Guard: x64 case RVA `0x8285a`, present in the supplied script corpus | [adapter.rs:4513](../../crates/crysirror-cmvs/src/adapter.rs#L4513) |
| `0x204D` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x204E` | Execute advance branch | Handled | Guard: x64 case RVA `0x82867` | [adapter.rs:4513](../../crates/crysirror-cmvs/src/adapter.rs#L4513) |
| `0x204F` | Execute advance branch | Handled | Guard: x64 case RVA `0x82874` | [adapter.rs:4513](../../crates/crysirror-cmvs/src/adapter.rs#L4513) |
| `0x2050` | Update job | Handled | Guard: x64 case RVA `0x82881`, present in the supplied script corpus | [adapter.rs:4761](../../crates/crysirror-cmvs/src/adapter.rs#L4761) |
| `0x2051` | Execute advance branch | Handled | Guard: x64 case RVA `0x8288e` | [adapter.rs:4513](../../crates/crysirror-cmvs/src/adapter.rs#L4513) |
| `0x2052` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2053` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2054` | Execute advance branch | Handled | Guard: x64 case RVA `0x8289b` | [adapter.rs:4513](../../crates/crysirror-cmvs/src/adapter.rs#L4513) |
| `0x2055` | Execute advance branch | Handled | Guard: x64 case RVA `0x828a8` | [adapter.rs:4513](../../crates/crysirror-cmvs/src/adapter.rs#L4513) |
| `0x2056` | Execute advance branch | Handled | Guard: x64 case RVA `0x828b5`, present in the supplied script corpus | [adapter.rs:3958](../../crates/crysirror-cmvs/src/adapter.rs#L3958) |
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
| `0x2069` | Execute advance branch | Handled | Guard: x64 case RVA `0x8299f`, present in the supplied script corpus | [adapter.rs:3348](../../crates/crysirror-cmvs/src/adapter.rs#L3348) |
| `0x206A` | Execute command branch | Handled | Guard: x64 case RVA `0x829ac`, present in the supplied script corpus | [screen.rs:221](../../crates/crysirror-cmvs/src/vm/screen.rs#L221) |
| `0x206B` | Start tween | Handled | Guard: x64 case RVA `0x829b9` | [screen.rs:220](../../crates/crysirror-cmvs/src/vm/screen.rs#L220) |
| `0x206C` | Start tween | Handled | Guard: x64 case RVA `0x829c6`, present in the supplied script corpus | [screen.rs:220](../../crates/crysirror-cmvs/src/vm/screen.rs#L220) |
| `0x206D` | Start tween | Handled | Guard: x64 case RVA `0x829d3` | [screen.rs:220](../../crates/crysirror-cmvs/src/vm/screen.rs#L220) |
| `0x206E` | Start tween | Handled | Guard: x64 case RVA `0x829e0` | [screen.rs:220](../../crates/crysirror-cmvs/src/vm/screen.rs#L220) |
| `0x206F` | Start tween | Handled | Guard: x64 case RVA `0x829ed` | [screen.rs:220](../../crates/crysirror-cmvs/src/vm/screen.rs#L220) |
| `0x2070` | Execute advance branch | Handled | Guard: x64 case RVA `0x829fa`, present in the supplied script corpus | [adapter.rs:4513](../../crates/crysirror-cmvs/src/adapter.rs#L4513) |
| `0x2071` | Execute advance branch | Handled | Guard: x64 case RVA `0x82a07`, present in the supplied script corpus | [adapter.rs:4513](../../crates/crysirror-cmvs/src/adapter.rs#L4513) |
| `0x2072` | Execute advance branch | Handled | Guard: x64 case RVA `0x82a14`, present in the supplied script corpus | [adapter.rs:4513](../../crates/crysirror-cmvs/src/adapter.rs#L4513) |
| `0x2073` | Execute advance branch | Handled | Guard: x64 case RVA `0x82a21`, present in the supplied script corpus | [adapter.rs:4513](../../crates/crysirror-cmvs/src/adapter.rs#L4513) |
| `0x2074` | Execute advance branch | Handled | Guard: x64 case RVA `0x82a2e`, present in the supplied script corpus | [adapter.rs:4513](../../crates/crysirror-cmvs/src/adapter.rs#L4513) |
| `0x2075` | Execute advance branch | Handled | Guard: x64 case RVA `0x82a3b`, present in the supplied script corpus | [adapter.rs:4513](../../crates/crysirror-cmvs/src/adapter.rs#L4513) |
| `0x2076` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2077` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2078` | Execute advance branch | Handled | Guard: x64 case RVA `0x82a48`, present in the supplied script corpus | [adapter.rs:4513](../../crates/crysirror-cmvs/src/adapter.rs#L4513) |
| `0x2079` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x207A` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x207B` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x207C` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x207D` | Start tween | Handled | Guard: x64 case RVA `0x82a55` | [screen.rs:220](../../crates/crysirror-cmvs/src/vm/screen.rs#L220) |
| `0x207E` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x207F` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2080` | Update job | Handled | Guard: x64 case RVA `0x82a62`, present in the supplied script corpus | [adapter.rs:4814](../../crates/crysirror-cmvs/src/adapter.rs#L4814) |
| `0x2081` | Update job | Handled | Guard: x64 case RVA `0x82a6f`, present in the supplied script corpus | [adapter.rs:4814](../../crates/crysirror-cmvs/src/adapter.rs#L4814) |
| `0x2082` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x82a7c` | [cmvs-20160201.json:14](../../tools/native-api/cmvs-20160201.json#L14) |
| `0x2083` | Control flow (command) | Handled | Guard: x64 case RVA `0x82a89` | [control.rs:534](../../crates/crysirror-cmvs/src/vm/control.rs#L534) |
| `0x2084` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2085` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2086` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2087` | Execute advance branch | Handled | Guard: x64 case RVA `0x82ab3` | [adapter.rs:3114](../../crates/crysirror-cmvs/src/adapter.rs#L3114) |
| `0x2088` | Control flow (command) | Handled | Guard: x64 case RVA `0x82ac0`, present in the supplied script corpus | [control.rs:540](../../crates/crysirror-cmvs/src/vm/control.rs#L540) |
| `0x2089` | Control flow (command) | Handled | Guard: x64 case RVA `0x82acd`, present in the supplied script corpus | [control.rs:572](../../crates/crysirror-cmvs/src/vm/control.rs#L572) |
| `0x208A` | Invoke a registered callback | Handled | Guard: x64 case RVA `0x82af9`, present in the supplied script corpus | [startup.rs:1574](../../crates/crysirror-cmvs/src/vm/startup.rs#L1574) |
| `0x208B` | Control flow (command) | Handled | Guard: x64 case RVA `0x82b06`, present in the supplied script corpus | [control.rs:540](../../crates/crysirror-cmvs/src/vm/control.rs#L540) |
| `0x208C` | Control flow (command) | Handled | Guard: x64 case RVA `0x82b13`, present in the supplied script corpus | [control.rs:572](../../crates/crysirror-cmvs/src/vm/control.rs#L572) |
| `0x208D` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x208E` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x208F` | Update job | Handled | Guard: x64 case RVA `0x82b3f`, present in the supplied script corpus | [adapter.rs:2700](../../crates/crysirror-cmvs/src/adapter.rs#L2700) |
| `0x2090` | Update settings / engine field 12084 | Handled | Guard: x64 case RVA `0x82b4c`, present in the supplied script corpus | [startup.rs:1608](../../crates/crysirror-cmvs/src/vm/startup.rs#L1608) |
| `0x2091` | Fill variables | Handled | Guard: x64 case RVA `0x82b61`, present in the supplied script corpus | [startup.rs:1612](../../crates/crysirror-cmvs/src/vm/startup.rs#L1612) |
| `0x2092` | Fill variables | Handled | Guard: x64 case RVA `0x82b6e`, present in the supplied script corpus | [startup.rs:1612](../../crates/crysirror-cmvs/src/vm/startup.rs#L1612) |
| `0x2093` | Fill variables | Handled | Guard: x64 case RVA `0x82b7b`, present in the supplied script corpus | [startup.rs:1612](../../crates/crysirror-cmvs/src/vm/startup.rs#L1612) |
| `0x2094` | Fill variables | Handled | Guard: x64 case RVA `0x82b88`, present in the supplied script corpus | [startup.rs:1612](../../crates/crysirror-cmvs/src/vm/startup.rs#L1612) |
| `0x2095` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2096` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2097` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2098` | Required parameter | Handled | Guard: x64 case RVA `0x82b95`, present in the supplied script corpus | [startup.rs:1684](../../crates/crysirror-cmvs/src/vm/startup.rs#L1684) |
| `0x2099` | Update job | Handled | Guard: x64 case RVA `0x82ba2`, present in the supplied script corpus | [adapter.rs:4331](../../crates/crysirror-cmvs/src/adapter.rs#L4331) |
| `0x209A` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x209B` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x209C` | Execute command branch | Open | Retains native registers; only the implemented 2D paths consume them; Guard: x64 case RVA `0x82baf`, present in the supplied script corpus | [screen.rs:245](../../crates/crysirror-cmvs/src/vm/screen.rs#L245) |
| `0x209D` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x209E` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x209F` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x20A0` | Update job | Handled | Guard: x64 case RVA `0x82bbc`, present in the supplied script corpus | [adapter.rs:2796](../../crates/crysirror-cmvs/src/adapter.rs#L2796) |
| `0x20A1` | Execute advance branch | Handled | Guard: x64 case RVA `0x82bc9`, present in the supplied script corpus | [adapter.rs:2814](../../crates/crysirror-cmvs/src/adapter.rs#L2814) |
| `0x20A2` | Execute advance branch | Handled | Guard: x64 case RVA `0x82be0`, present in the supplied script corpus | [adapter.rs:2818](../../crates/crysirror-cmvs/src/adapter.rs#L2818) |
| `0x20A3` | Execute advance branch | Handled | Guard: x64 case RVA `0x82bed` | [adapter.rs:3974](../../crates/crysirror-cmvs/src/adapter.rs#L3974) |
| `0x20A4` | Update job | Handled | Guard: x64 case RVA `0x82bfa`, present in the supplied script corpus | [adapter.rs:2796](../../crates/crysirror-cmvs/src/adapter.rs#L2796) |
| `0x20A5` | Execute advance branch | Open | Device attenuation affects software PCM and subsequent hardware sample starts; driver parity is pending; Guard: x64 case RVA `0x82c07` | [adapter.rs:2823](../../crates/crysirror-cmvs/src/adapter.rs#L2823) |
| `0x20A6` | Update job | Open | MOG length result unit is unverified; Guard: x64 case RVA `0x82c14`, present in the supplied script corpus | [adapter.rs:2830](../../crates/crysirror-cmvs/src/adapter.rs#L2830) |
| `0x20A7` | Activate the allocated voice at its current decoder cursor | Handled | Guard: x64 case RVA `0x82c21` | [adapter.rs:2871](../../crates/crysirror-cmvs/src/adapter.rs#L2871) |
| `0x20A8` | Execute advance branch | Handled | Guard: x64 case RVA `0x82c2e`, present in the supplied script corpus | [adapter.rs:2898](../../crates/crysirror-cmvs/src/adapter.rs#L2898) |
| `0x20A9` | Execute advance branch | Handled | Guard: x64 case RVA `0x82c3b`, present in the supplied script corpus | [adapter.rs:2898](../../crates/crysirror-cmvs/src/adapter.rs#L2898) |
| `0x20AA` | Execute advance branch | Open | Elapsed mixer frames are available; original device cursor and buffer lead are pending; Guard: x64 case RVA `0x82c48`, present in the supplied script corpus | [adapter.rs:2898](../../crates/crysirror-cmvs/src/adapter.rs#L2898) |
| `0x20AB` | Execute advance branch | Handled | Guard: x64 case RVA `0x82c55` | [adapter.rs:3978](../../crates/crysirror-cmvs/src/adapter.rs#L3978) |
| `0x20AC` | Execute advance branch | Open | Quantized per-voice gain is connected; native fade and device PCM timing need parity evidence; Guard: x64 case RVA `0x82c62` | [adapter.rs:2975](../../crates/crysirror-cmvs/src/adapter.rs#L2975) |
| `0x20AD` | Execute advance branch | Open | Quantized per-voice gain is connected; native fade and device PCM timing need parity evidence; Guard: x64 case RVA `0x82c6f` | [adapter.rs:2975](../../crates/crysirror-cmvs/src/adapter.rs#L2975) |
| `0x20AE` | Execute advance branch | Open | Buffered source levels are connected; output resampling can differ from DirectSound; Guard: x64 case RVA `0x82c7c` | [adapter.rs:3002](../../crates/crysirror-cmvs/src/adapter.rs#L3002) |
| `0x20AF` | Update job | Open | MOG length and rate-conversion parity are unverified; Guard: x64 case RVA `0x82c89`, present in the supplied script corpus | [adapter.rs:2830](../../crates/crysirror-cmvs/src/adapter.rs#L2830) |
| `0x20B0` | Execute advance branch | Handled | Guard: x64 case RVA `0x82c96`, present in the supplied script corpus | [adapter.rs:2354](../../crates/crysirror-cmvs/src/adapter.rs#L2354) |
| `0x20B1` | Execute advance branch | Handled | Guard: x64 case RVA `0x82ca3`, present in the supplied script corpus | [adapter.rs:2356](../../crates/crysirror-cmvs/src/adapter.rs#L2356) |
| `0x20B2` | Pause the effect slot and retain its decoder | Handled | Guard: x64 case RVA `0x82cb0` | [adapter.rs:2357](../../crates/crysirror-cmvs/src/adapter.rs#L2357) |
| `0x20B3` | Execute advance branch | Handled | Guard: x64 case RVA `0x82cbd`, present in the supplied script corpus | [adapter.rs:2358](../../crates/crysirror-cmvs/src/adapter.rs#L2358) |
| `0x20B4` | Rewind and activate the allocated effect | Handled | Guard: x64 case RVA `0x82cca` | [adapter.rs:2359](../../crates/crysirror-cmvs/src/adapter.rs#L2359) |
| `0x20B5` | Execute advance branch | Handled | Guard: x64 case RVA `0x82cd7`, present in the supplied script corpus | [adapter.rs:2756](../../crates/crysirror-cmvs/src/adapter.rs#L2756) |
| `0x20B6` | Execute advance branch | Handled | Guard: x64 case RVA `0x82ce4`, present in the supplied script corpus | [adapter.rs:2360](../../crates/crysirror-cmvs/src/adapter.rs#L2360) |
| `0x20B7` | Execute advance branch | Open | Elapsed mixer frames are available; original device cursor and buffer lead are pending; Guard: x64 case RVA `0x82cf1`, present in the supplied script corpus | [adapter.rs:2783](../../crates/crysirror-cmvs/src/adapter.rs#L2783) |
| `0x20B8` | Execute advance branch | Open | Quantized per-voice gain is connected; native fade and device PCM timing need parity evidence; Guard: x64 case RVA `0x82cfe` | [adapter.rs:2975](../../crates/crysirror-cmvs/src/adapter.rs#L2975) |
| `0x20B9` | Execute advance branch | Open | Quantized per-voice gain is connected; native fade and device PCM timing need parity evidence; Guard: x64 case RVA `0x82d0b` | [adapter.rs:2975](../../crates/crysirror-cmvs/src/adapter.rs#L2975) |
| `0x20BA` | Execute advance branch | Handled | Guard: x64 case RVA `0x82d18`, present in the supplied script corpus | [adapter.rs:2355](../../crates/crysirror-cmvs/src/adapter.rs#L2355) |
| `0x20BB` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x20BC` | Mark the allocated voice for mixer retirement | Handled | Guard: x64 case RVA `0x82d25` | [adapter.rs:2882](../../crates/crysirror-cmvs/src/adapter.rs#L2882) |
| `0x20BD` | Wait for the mixer to release the voice slot | Handled | Guard: x64 case RVA `0x82d32` | [adapter.rs:2882](../../crates/crysirror-cmvs/src/adapter.rs#L2882) |
| `0x20BE` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x20BF` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x20C0` | Update job | Handled | Guard: x64 case RVA `0x82d3f`, present in the supplied script corpus | [adapter.rs:3080](../../crates/crysirror-cmvs/src/adapter.rs#L3080) |
| `0x20C1` | Execute advance branch | Handled | Guard: x64 case RVA `0x82d4c`, present in the supplied script corpus | [adapter.rs:3094](../../crates/crysirror-cmvs/src/adapter.rs#L3094) |
| `0x20C2` | Execute advance branch | Handled | Guard: x64 case RVA `0x82d59`, present in the supplied script corpus | [adapter.rs:3098](../../crates/crysirror-cmvs/src/adapter.rs#L3098) |
| `0x20C3` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x20C4` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x20C5` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x20C6` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x20C7` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x20C8` | Input (command) | Handled | Guard: x64 case RVA `0x82d66`, present in the supplied script corpus | [input.rs:214](../../crates/crysirror-cmvs/src/vm/input.rs#L214) |
| `0x20C9` | Input (command) | Handled | Guard: x64 case RVA `0x82d73` | [input.rs:219](../../crates/crysirror-cmvs/src/vm/input.rs#L219) |
| `0x20CA` | Execute advance branch | Handled | Guard: x64 case RVA `0x82d80`, present in the supplied script corpus | [adapter.rs:3311](../../crates/crysirror-cmvs/src/adapter.rs#L3311) |
| `0x20CB` | Update job | Handled | Guard: x64 case RVA `0x82d8d` | [adapter.rs:4331](../../crates/crysirror-cmvs/src/adapter.rs#L4331) |
| `0x20CC` | Update job | Handled | Guard: x64 case RVA `0x82d9a` | [adapter.rs:4331](../../crates/crysirror-cmvs/src/adapter.rs#L4331) |
| `0x20CD` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x82da7` | [cmvs-20160201.json:18](../../tools/native-api/cmvs-20160201.json#L18) |
| `0x20CE` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x20CF` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x20D0` | Control flow (command) | Handled | Guard: x64 case RVA `0x82db4`, present in the supplied script corpus | [control.rs:403](../../crates/crysirror-cmvs/src/vm/control.rs#L403) |
| `0x20D1` | Control flow (command) | Handled | Guard: x64 case RVA `0x82ddc` | [control.rs:403](../../crates/crysirror-cmvs/src/vm/control.rs#L403) |
| `0x20D2` | Control flow (command) | Handled | Guard: x64 case RVA `0x82e1f` | [control.rs:403](../../crates/crysirror-cmvs/src/vm/control.rs#L403) |
| `0x20D3` | Control flow (command) | Handled | Guard: x64 case RVA `0x82e4f` | [control.rs:403](../../crates/crysirror-cmvs/src/vm/control.rs#L403) |
| `0x20D4` | Control flow (command) | Handled | Guard: x64 case RVA `0x82e7f`, present in the supplied script corpus | [control.rs:385](../../crates/crysirror-cmvs/src/vm/control.rs#L385) |
| `0x20D5` | Control flow (command) | Handled | Guard: x64 case RVA `0x82e8c` | [control.rs:385](../../crates/crysirror-cmvs/src/vm/control.rs#L385) |
| `0x20D6` | Control flow (command) | Handled | Guard: x64 case RVA `0x82e99` | [control.rs:426](../../crates/crysirror-cmvs/src/vm/control.rs#L426) |
| `0x20D7` | Control flow (command) | Handled | Guard: x64 case RVA `0x82ea6` | [control.rs:436](../../crates/crysirror-cmvs/src/vm/control.rs#L436) |
| `0x20D8` | Control flow (command) | Open | Native trigonometric results at truncation boundaries need verification; Guard: x64 case RVA `0x82eb3` | [control.rs:459](../../crates/crysirror-cmvs/src/vm/control.rs#L459) |
| `0x20D9` | Control flow (command) | Open | Native trigonometric results at truncation boundaries need verification; Guard: x64 case RVA `0x82ec0` | [control.rs:459](../../crates/crysirror-cmvs/src/vm/control.rs#L459) |
| `0x20DA` | Control flow (command) | Open | Native trigonometric results at truncation boundaries need verification; Guard: x64 case RVA `0x82ecd` | [control.rs:482](../../crates/crysirror-cmvs/src/vm/control.rs#L482) |
| `0x20DB` | Control flow (command) | Open | Native trigonometric results at truncation boundaries need verification; Guard: x64 case RVA `0x82eda` | [control.rs:482](../../crates/crysirror-cmvs/src/vm/control.rs#L482) |
| `0x20DC` | Control flow (command) | Open | Native trigonometric results at truncation boundaries need verification; Guard: x64 case RVA `0x82ee7` | [control.rs:459](../../crates/crysirror-cmvs/src/vm/control.rs#L459) |
| `0x20DD` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x20DE` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x20DF` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x20E0` | Control flow (command) | Open | Native trigonometric results at truncation boundaries need verification; Guard: x64 case RVA `0x82ef4` | [control.rs:512](../../crates/crysirror-cmvs/src/vm/control.rs#L512) |
| `0x20E1` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x20E2` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x20E3` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x20E4` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x20E5` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x20E6` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x20E7` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x20E8` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x82f01` | [cmvs-20160201.json:20](../../tools/native-api/cmvs-20160201.json#L20) |
| `0x20E9` | Control flow (command) | Handled | Guard: x64 case RVA `0x825b1` | [control.rs:346](../../crates/crysirror-cmvs/src/vm/control.rs#L346) |
| `0x20EA` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x20EB` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x20EC` | Execute advance branch | Handled | Guard: x64 case RVA `0x82f24`, present in the supplied script corpus | [adapter.rs:3124](../../crates/crysirror-cmvs/src/adapter.rs#L3124) |
| `0x20ED` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x20EE` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x20EF` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x20F0` | Execute startup command branch | Handled | Guard: x64 case RVA `0x82f31` | [startup.rs:1726](../../crates/crysirror-cmvs/src/vm/startup.rs#L1726) |
| `0x20F1` | Execute startup command branch | Handled | Guard: x64 case RVA `0x82f3e`, present in the supplied script corpus | [startup.rs:1726](../../crates/crysirror-cmvs/src/vm/startup.rs#L1726) |
| `0x20F2` | Execute startup command branch | Handled | Guard: x64 case RVA `0x82f4b` | [startup.rs:1726](../../crates/crysirror-cmvs/src/vm/startup.rs#L1726) |
| `0x20F3` | Execute startup command branch | Handled | Guard: x64 case RVA `0x82f58`, present in the supplied script corpus | [startup.rs:1726](../../crates/crysirror-cmvs/src/vm/startup.rs#L1726) |
| `0x20F4` | Execute startup command branch | Handled | Guard: x64 case RVA `0x82f65` | [startup.rs:1726](../../crates/crysirror-cmvs/src/vm/startup.rs#L1726) |
| `0x20F5` | Execute startup command branch | Handled | Guard: x64 case RVA `0x82f72` | [startup.rs:1726](../../crates/crysirror-cmvs/src/vm/startup.rs#L1726) |
| `0x20F6` | Execute startup command branch | Handled | Guard: x64 case RVA `0x82f7f` | [startup.rs:1726](../../crates/crysirror-cmvs/src/vm/startup.rs#L1726) |
| `0x20F7` | Execute startup command branch | Handled | Guard: x64 case RVA `0x82f8c` | [startup.rs:1726](../../crates/crysirror-cmvs/src/vm/startup.rs#L1726) |
| `0x20F8` | Execute startup command branch | Handled | Guard: x64 case RVA `0x82f99`, present in the supplied script corpus | [startup.rs:1726](../../crates/crysirror-cmvs/src/vm/startup.rs#L1726) |
| `0x20F9` | Execute startup command branch | Handled | Guard: x64 case RVA `0x82fa6` | [startup.rs:1726](../../crates/crysirror-cmvs/src/vm/startup.rs#L1726) |
| `0x20FA` | Execute startup command branch | Handled | Guard: x64 case RVA `0x82fb3` | [startup.rs:1726](../../crates/crysirror-cmvs/src/vm/startup.rs#L1726) |
| `0x20FB` | Execute startup command branch | Handled | Guard: x64 case RVA `0x82fc0` | [startup.rs:1726](../../crates/crysirror-cmvs/src/vm/startup.rs#L1726) |
| `0x20FC` | Execute startup command branch | Open | Overlapping format buffers and native pointer reinterpretation are rejected; Guard: x64 case RVA `0x82fcd`, present in the supplied script corpus | [startup.rs:1726](../../crates/crysirror-cmvs/src/vm/startup.rs#L1726) |
| `0x20FD` | Execute startup command branch | Open | Overlapping format buffers and native pointer reinterpretation are rejected; Guard: x64 case RVA `0x82fda` | [startup.rs:1726](../../crates/crysirror-cmvs/src/vm/startup.rs#L1726) |
| `0x20FE` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x20FF` | Execute startup command branch | Handled | Guard: x64 case RVA `0x82fe7` | [startup.rs:1726](../../crates/crysirror-cmvs/src/vm/startup.rs#L1726) |
| `0x2100` | Execute advance branch | Handled | Guard: x64 case RVA `0x82ff4`, present in the supplied script corpus | [adapter.rs:4538](../../crates/crysirror-cmvs/src/adapter.rs#L4538) |
| `0x2101` | Execute advance branch | Handled | Guard: x64 case RVA `0x83001`, present in the supplied script corpus | [adapter.rs:4538](../../crates/crysirror-cmvs/src/adapter.rs#L4538) |
| `0x2102` | Execute advance branch | Handled | Guard: x64 case RVA `0x8300e`, present in the supplied script corpus | [adapter.rs:4538](../../crates/crysirror-cmvs/src/adapter.rs#L4538) |
| `0x2103` | Execute advance branch | Handled | Guard: x64 case RVA `0x8301b`, present in the supplied script corpus | [adapter.rs:4538](../../crates/crysirror-cmvs/src/adapter.rs#L4538) |
| `0x2104` | Execute advance branch | Handled | Guard: x64 case RVA `0x83028`, present in the supplied script corpus | [adapter.rs:4538](../../crates/crysirror-cmvs/src/adapter.rs#L4538) |
| `0x2105` | Execute advance branch | Handled | Guard: x64 case RVA `0x83035`, present in the supplied script corpus | [adapter.rs:4538](../../crates/crysirror-cmvs/src/adapter.rs#L4538) |
| `0x2106` | Execute advance branch | Open | Scoped font aliases are connected; native GDI font metrics remain open; Guard: x64 case RVA `0x83042`, present in the supplied script corpus | [adapter.rs:3301](../../crates/crysirror-cmvs/src/adapter.rs#L3301) |
| `0x2107` | Execute advance branch | Handled | Guard: x64 case RVA `0x8304f`, present in the supplied script corpus | [adapter.rs:4538](../../crates/crysirror-cmvs/src/adapter.rs#L4538) |
| `0x2108` | Execute advance branch | Handled | Guard: x64 case RVA `0x8305c` | [adapter.rs:4538](../../crates/crysirror-cmvs/src/adapter.rs#L4538) |
| `0x2109` | Execute advance branch | Handled | Guard: x64 case RVA `0x83069` | [adapter.rs:4538](../../crates/crysirror-cmvs/src/adapter.rs#L4538) |
| `0x210A` | Execute advance branch | Handled | Guard: x64 case RVA `0x83076` | [adapter.rs:4538](../../crates/crysirror-cmvs/src/adapter.rs#L4538) |
| `0x210B` | Execute advance branch | Handled | Guard: x64 case RVA `0x83083`, present in the supplied script corpus | [adapter.rs:4538](../../crates/crysirror-cmvs/src/adapter.rs#L4538) |
| `0x210C` | Execute advance branch | Handled | Guard: x64 case RVA `0x83090`, present in the supplied script corpus | [adapter.rs:4538](../../crates/crysirror-cmvs/src/adapter.rs#L4538) |
| `0x210D` | Execute advance branch | Handled | Guard: x64 case RVA `0x8309d`, present in the supplied script corpus | [adapter.rs:4538](../../crates/crysirror-cmvs/src/adapter.rs#L4538) |
| `0x210E` | Execute advance branch | Handled | Guard: x64 case RVA `0x830aa`, present in the supplied script corpus | [adapter.rs:4538](../../crates/crysirror-cmvs/src/adapter.rs#L4538) |
| `0x210F` | Execute advance branch | Handled | Guard: x64 case RVA `0x830b7`, present in the supplied script corpus | [adapter.rs:4538](../../crates/crysirror-cmvs/src/adapter.rs#L4538) |
| `0x2110` | Execute advance branch | Handled | Guard: x64 case RVA `0x830c4`, present in the supplied script corpus | [adapter.rs:4538](../../crates/crysirror-cmvs/src/adapter.rs#L4538) |
| `0x2111` | Execute advance branch | Handled | Guard: x64 case RVA `0x830d1` | [adapter.rs:4538](../../crates/crysirror-cmvs/src/adapter.rs#L4538) |
| `0x2112` | Execute advance branch | Handled | Guard: x64 case RVA `0x830de` | [adapter.rs:3124](../../crates/crysirror-cmvs/src/adapter.rs#L3124) |
| `0x2113` | Execute advance branch | Handled | Guard: x64 case RVA `0x830eb`, present in the supplied script corpus | [adapter.rs:4538](../../crates/crysirror-cmvs/src/adapter.rs#L4538) |
| `0x2114` | Execute advance branch | Handled | Guard: x64 case RVA `0x830f8`, present in the supplied script corpus | [adapter.rs:3124](../../crates/crysirror-cmvs/src/adapter.rs#L3124) |
| `0x2115` | Execute advance branch | Handled | Guard: x64 case RVA `0x83105`, present in the supplied script corpus | [adapter.rs:3124](../../crates/crysirror-cmvs/src/adapter.rs#L3124) |
| `0x2116` | Execute advance branch | Handled | Guard: x64 case RVA `0x83112`, present in the supplied script corpus | [adapter.rs:3124](../../crates/crysirror-cmvs/src/adapter.rs#L3124) |
| `0x2117` | Execute advance branch | Handled | Guard: x64 case RVA `0x8311f`, present in the supplied script corpus | [adapter.rs:3124](../../crates/crysirror-cmvs/src/adapter.rs#L3124) |
| `0x2118` | Execute advance branch | Handled | Guard: x64 case RVA `0x8312c` | [adapter.rs:4538](../../crates/crysirror-cmvs/src/adapter.rs#L4538) |
| `0x2119` | Execute advance branch | Handled | Guard: x64 case RVA `0x83139` | [adapter.rs:3124](../../crates/crysirror-cmvs/src/adapter.rs#L3124) |
| `0x211A` | Execute advance branch | Handled | Guard: x64 case RVA `0x83146`, present in the supplied script corpus | [adapter.rs:4538](../../crates/crysirror-cmvs/src/adapter.rs#L4538) |
| `0x211B` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x211C` | Execute advance branch | Handled | Guard: x64 case RVA `0x83153` | [adapter.rs:4538](../../crates/crysirror-cmvs/src/adapter.rs#L4538) |
| `0x211D` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x211E` | Execute advance branch | Handled | Guard: x64 case RVA `0x83160`, present in the supplied script corpus | [adapter.rs:3124](../../crates/crysirror-cmvs/src/adapter.rs#L3124) |
| `0x211F` | Execute advance branch | Handled | Guard: x64 case RVA `0x8316d`, present in the supplied script corpus | [adapter.rs:3124](../../crates/crysirror-cmvs/src/adapter.rs#L3124) |
| `0x2120` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x8317a` | [cmvs-20160201.json:24](../../tools/native-api/cmvs-20160201.json#L24) |
| `0x2121` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x83187` | [cmvs-20160201.json:24](../../tools/native-api/cmvs-20160201.json#L24) |
| `0x2122` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x83194` | [cmvs-20160201.json:24](../../tools/native-api/cmvs-20160201.json#L24) |
| `0x2123` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x831a9` | [cmvs-20160201.json:24](../../tools/native-api/cmvs-20160201.json#L24) |
| `0x2124` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2125` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2126` | Execute command branch | Handled | Guard: x64 case RVA `0x831c1`, present in the supplied script corpus | [screen.rs:228](../../crates/crysirror-cmvs/src/vm/screen.rs#L228) |
| `0x2127` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2128` | Update job | Handled | Guard: x64 case RVA `0x831ce`, present in the supplied script corpus | [adapter.rs:2679](../../crates/crysirror-cmvs/src/adapter.rs#L2679) |
| `0x2129` | Update job | Handled | Guard: x64 case RVA `0x831e1`, present in the supplied script corpus | [adapter.rs:3814](../../crates/crysirror-cmvs/src/adapter.rs#L3814) |
| `0x212A` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x831f4` | [cmvs-20160201.json:24](../../tools/native-api/cmvs-20160201.json#L24) |
| `0x212B` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x83209` | [cmvs-20160201.json:24](../../tools/native-api/cmvs-20160201.json#L24) |
| `0x212C` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x8321e` | [cmvs-20160201.json:24](../../tools/native-api/cmvs-20160201.json#L24) |
| `0x212D` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x212E` | Execute startup command branch | Handled | Guard: x64 case RVA `0x83233` | [startup.rs:1698](../../crates/crysirror-cmvs/src/vm/startup.rs#L1698) |
| `0x212F` | Execute startup command branch | Handled | Guard: x64 case RVA `0x83240`, present in the supplied script corpus | [startup.rs:1698](../../crates/crysirror-cmvs/src/vm/startup.rs#L1698) |
| `0x2130` | Execute startup command branch | Handled | Guard: x64 case RVA `0x83257`, present in the supplied script corpus | [startup.rs:1698](../../crates/crysirror-cmvs/src/vm/startup.rs#L1698) |
| `0x2131` | Execute startup command branch | Handled | Guard: x64 case RVA `0x83264`, present in the supplied script corpus | [startup.rs:1698](../../crates/crysirror-cmvs/src/vm/startup.rs#L1698) |
| `0x2132` | Execute startup command branch | Handled | Guard: x64 case RVA `0x83271`, present in the supplied script corpus | [startup.rs:1698](../../crates/crysirror-cmvs/src/vm/startup.rs#L1698) |
| `0x2133` | Execute startup command branch | Handled | Guard: x64 case RVA `0x8327e`, present in the supplied script corpus | [startup.rs:1698](../../crates/crysirror-cmvs/src/vm/startup.rs#L1698) |
| `0x2134` | Execute startup command branch | Handled | Guard: x64 case RVA `0x8328b` | [startup.rs:1698](../../crates/crysirror-cmvs/src/vm/startup.rs#L1698) |
| `0x2135` | Execute startup command branch | Handled | Guard: x64 case RVA `0x83298`, present in the supplied script corpus | [startup.rs:1698](../../crates/crysirror-cmvs/src/vm/startup.rs#L1698) |
| `0x2136` | Execute startup command branch | Handled | Guard: x64 case RVA `0x832a5`, present in the supplied script corpus | [startup.rs:1698](../../crates/crysirror-cmvs/src/vm/startup.rs#L1698) |
| `0x2137` | Execute startup command branch | Handled | Guard: x64 case RVA `0x832b2` | [startup.rs:1698](../../crates/crysirror-cmvs/src/vm/startup.rs#L1698) |
| `0x2138` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2139` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x213A` | Execute advance branch | Handled | Guard: x64 case RVA `0x832bf` | [adapter.rs:4466](../../crates/crysirror-cmvs/src/adapter.rs#L4466) |
| `0x213B` | Execute advance branch | Handled | Guard: x64 case RVA `0x832cc` | [adapter.rs:4466](../../crates/crysirror-cmvs/src/adapter.rs#L4466) |
| `0x213C` | Execute advance branch | Handled | Guard: x64 case RVA `0x832d9` | [adapter.rs:4466](../../crates/crysirror-cmvs/src/adapter.rs#L4466) |
| `0x213D` | Execute advance branch | Handled | Guard: x64 case RVA `0x832e6`, present in the supplied script corpus | [adapter.rs:4466](../../crates/crysirror-cmvs/src/adapter.rs#L4466) |
| `0x213E` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x213F` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2140` | Execute advance branch | Handled | Guard: x64 case RVA `0x832f9`, present in the supplied script corpus | [adapter.rs:3124](../../crates/crysirror-cmvs/src/adapter.rs#L3124) |
| `0x2141` | Execute advance branch | Handled | Guard: x64 case RVA `0x83306`, present in the supplied script corpus | [adapter.rs:3124](../../crates/crysirror-cmvs/src/adapter.rs#L3124) |
| `0x2142` | Execute advance branch | Handled | Guard: x64 case RVA `0x83313`, present in the supplied script corpus | [adapter.rs:3124](../../crates/crysirror-cmvs/src/adapter.rs#L3124) |
| `0x2143` | Execute advance branch | Handled | Guard: x64 case RVA `0x83320`, present in the supplied script corpus | [adapter.rs:3124](../../crates/crysirror-cmvs/src/adapter.rs#L3124) |
| `0x2144` | Execute advance branch | Handled | Guard: x64 case RVA `0x8332d`, present in the supplied script corpus | [adapter.rs:3124](../../crates/crysirror-cmvs/src/adapter.rs#L3124) |
| `0x2145` | Execute advance branch | Handled | Guard: x64 case RVA `0x8333a`, present in the supplied script corpus | [adapter.rs:3124](../../crates/crysirror-cmvs/src/adapter.rs#L3124) |
| `0x2146` | Execute advance branch | Handled | Guard: x64 case RVA `0x83347`, present in the supplied script corpus | [adapter.rs:3124](../../crates/crysirror-cmvs/src/adapter.rs#L3124) |
| `0x2147` | Execute advance branch | Handled | Guard: x64 case RVA `0x83354`, present in the supplied script corpus | [adapter.rs:3124](../../crates/crysirror-cmvs/src/adapter.rs#L3124) |
| `0x2148` | Execute advance branch | Handled | Guard: x64 case RVA `0x83361`, present in the supplied script corpus | [adapter.rs:3124](../../crates/crysirror-cmvs/src/adapter.rs#L3124) |
| `0x2149` | Execute advance branch | Handled | Guard: x64 case RVA `0x8336e`, present in the supplied script corpus | [adapter.rs:3124](../../crates/crysirror-cmvs/src/adapter.rs#L3124) |
| `0x214A` | Execute advance branch | Handled | Guard: x64 case RVA `0x8337b`, present in the supplied script corpus | [adapter.rs:3124](../../crates/crysirror-cmvs/src/adapter.rs#L3124) |
| `0x214B` | Pc | Handled | Guard: x64 case RVA `0x83388`, present in the supplied script corpus | [startup.rs:1727](../../crates/crysirror-cmvs/src/vm/startup.rs#L1727) |
| `0x214C` | Execute advance branch | Handled | Guard: x64 case RVA `0x83395`, present in the supplied script corpus | [adapter.rs:3124](../../crates/crysirror-cmvs/src/adapter.rs#L3124) |
| `0x214D` | Execute advance branch | Handled | Guard: x64 case RVA `0x833a2`, present in the supplied script corpus | [adapter.rs:3124](../../crates/crysirror-cmvs/src/adapter.rs#L3124) |
| `0x214E` | Execute advance branch | Handled | Guard: x64 case RVA `0x833af`, present in the supplied script corpus | [adapter.rs:3124](../../crates/crysirror-cmvs/src/adapter.rs#L3124) |
| `0x214F` | Execute advance branch | Handled | Guard: x64 case RVA `0x833bc`, present in the supplied script corpus | [adapter.rs:3124](../../crates/crysirror-cmvs/src/adapter.rs#L3124) |
| `0x2150` | Execute advance branch | Open | Message move modes with an unsupported branch remain open; Guard: x64 case RVA `0x833c9`, present in the supplied script corpus | [adapter.rs:3124](../../crates/crysirror-cmvs/src/adapter.rs#L3124) |
| `0x2151` | Execute advance branch | Handled | Guard: x64 case RVA `0x833d6`, present in the supplied script corpus | [adapter.rs:3124](../../crates/crysirror-cmvs/src/adapter.rs#L3124) |
| `0x2152` | Execute advance branch | Open | Gradient and exact native font fidelity remain open; Guard: x64 case RVA `0x833e3`, present in the supplied script corpus | [adapter.rs:3154](../../crates/crysirror-cmvs/src/adapter.rs#L3154) |
| `0x2153` | Execute advance branch | Handled | Guard: x64 case RVA `0x833f0`, present in the supplied script corpus | [adapter.rs:3154](../../crates/crysirror-cmvs/src/adapter.rs#L3154) |
| `0x2154` | Execute advance branch | Handled | Guard: x64 case RVA `0x833fd`, present in the supplied script corpus | [adapter.rs:3380](../../crates/crysirror-cmvs/src/adapter.rs#L3380) |
| `0x2155` | Execute advance branch | Handled | Guard: x64 case RVA `0x8340a`, present in the supplied script corpus | [adapter.rs:3124](../../crates/crysirror-cmvs/src/adapter.rs#L3124) |
| `0x2156` | Execute advance branch | Handled | Guard: x64 case RVA `0x83417` | [adapter.rs:3380](../../crates/crysirror-cmvs/src/adapter.rs#L3380) |
| `0x2157` | Execute advance branch | Open | Gradient and exact native font fidelity remain open; Guard: x64 case RVA `0x83424`, present in the supplied script corpus | [adapter.rs:3154](../../crates/crysirror-cmvs/src/adapter.rs#L3154) |
| `0x2158` | Execute advance branch | Handled | Guard: x64 case RVA `0x83431`, present in the supplied script corpus | [adapter.rs:3154](../../crates/crysirror-cmvs/src/adapter.rs#L3154) |
| `0x2159` | Execute advance branch | Handled | Guard: x64 case RVA `0x8343e`, present in the supplied script corpus | [adapter.rs:3380](../../crates/crysirror-cmvs/src/adapter.rs#L3380) |
| `0x215A` | Execute advance branch | Handled | Guard: x64 case RVA `0x8344b`, present in the supplied script corpus | [adapter.rs:3124](../../crates/crysirror-cmvs/src/adapter.rs#L3124) |
| `0x215B` | Execute advance branch | Handled | Guard: x64 case RVA `0x83458`, present in the supplied script corpus | [adapter.rs:3124](../../crates/crysirror-cmvs/src/adapter.rs#L3124) |
| `0x215C` | Execute advance branch | Handled | Guard: x64 case RVA `0x83465` | [adapter.rs:3154](../../crates/crysirror-cmvs/src/adapter.rs#L3154) |
| `0x215D` | Execute advance branch | Handled | Guard: x64 case RVA `0x83472`, present in the supplied script corpus | [adapter.rs:3124](../../crates/crysirror-cmvs/src/adapter.rs#L3124) |
| `0x215E` | Execute advance branch | Handled | Guard: x64 case RVA `0x8347f` | [adapter.rs:3380](../../crates/crysirror-cmvs/src/adapter.rs#L3380) |
| `0x215F` | Execute advance branch | Handled | Guard: x64 case RVA `0x8348c`, present in the supplied script corpus | [adapter.rs:3124](../../crates/crysirror-cmvs/src/adapter.rs#L3124) |
| `0x2160` | Execute advance branch | Handled | Guard: x64 case RVA `0x83499`, present in the supplied script corpus | [adapter.rs:2691](../../crates/crysirror-cmvs/src/adapter.rs#L2691) |
| `0x2161` | Execute advance branch | Handled | Guard: x64 case RVA `0x834a6`, present in the supplied script corpus | [adapter.rs:2691](../../crates/crysirror-cmvs/src/adapter.rs#L2691) |
| `0x2162` | Pc | Handled | Guard: x64 case RVA `0x834b3`, present in the supplied script corpus | [startup.rs:1716](../../crates/crysirror-cmvs/src/vm/startup.rs#L1716) |
| `0x2163` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x834c0`, present in the supplied script corpus | [cmvs-20160201.json:28](../../tools/native-api/cmvs-20160201.json#L28) |
| `0x2164` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x834cd`, present in the supplied script corpus | [cmvs-20160201.json:28](../../tools/native-api/cmvs-20160201.json#L28) |
| `0x2165` | Execute advance branch | Handled | Guard: x64 case RVA `0x834da` | [adapter.rs:3337](../../crates/crysirror-cmvs/src/adapter.rs#L3337) |
| `0x2166` | Execute advance branch | Handled | Guard: x64 case RVA `0x834e7` | [adapter.rs:3337](../../crates/crysirror-cmvs/src/adapter.rs#L3337) |
| `0x2167` | Execute advance branch | Handled | Guard: x64 case RVA `0x8350a` | [adapter.rs:3337](../../crates/crysirror-cmvs/src/adapter.rs#L3337) |
| `0x2168` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x83517` | [cmvs-20160201.json:28](../../tools/native-api/cmvs-20160201.json#L28) |
| `0x2169` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x83524` | [cmvs-20160201.json:28](../../tools/native-api/cmvs-20160201.json#L28) |
| `0x216A` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x216B` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x83531`, present in the supplied script corpus | [cmvs-20160201.json:28](../../tools/native-api/cmvs-20160201.json#L28) |
| `0x216C` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x8353e`, present in the supplied script corpus | [cmvs-20160201.json:28](../../tools/native-api/cmvs-20160201.json#L28) |
| `0x216D` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x8354b`, present in the supplied script corpus | [cmvs-20160201.json:28](../../tools/native-api/cmvs-20160201.json#L28) |
| `0x216E` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x83558`, present in the supplied script corpus | [cmvs-20160201.json:28](../../tools/native-api/cmvs-20160201.json#L28) |
| `0x216F` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x83578`, present in the supplied script corpus | [cmvs-20160201.json:28](../../tools/native-api/cmvs-20160201.json#L28) |
| `0x2170` | Update job | Handled | Guard: x64 case RVA `0x83585`, present in the supplied script corpus | [adapter.rs:4558](../../crates/crysirror-cmvs/src/adapter.rs#L4558) |
| `0x2171` | Execute advance branch | Handled | Guard: x64 case RVA `0x83592`, present in the supplied script corpus | [adapter.rs:3124](../../crates/crysirror-cmvs/src/adapter.rs#L3124) |
| `0x2172` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2173` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2174` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2175` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2176` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x8359f` | [cmvs-20160201.json:29](../../tools/native-api/cmvs-20160201.json#L29) |
| `0x2177` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x835ac` | [cmvs-20160201.json:29](../../tools/native-api/cmvs-20160201.json#L29) |
| `0x2178` | Execute advance branch | Handled | Guard: x64 case RVA `0x835b9`, present in the supplied script corpus | [adapter.rs:3124](../../crates/crysirror-cmvs/src/adapter.rs#L3124) |
| `0x2179` | Execute advance branch | Handled | Guard: x64 case RVA `0x835c6`, present in the supplied script corpus | [adapter.rs:3124](../../crates/crysirror-cmvs/src/adapter.rs#L3124) |
| `0x217A` | Execute advance branch | Handled | Guard: x64 case RVA `0x835d3`, present in the supplied script corpus | [adapter.rs:3124](../../crates/crysirror-cmvs/src/adapter.rs#L3124) |
| `0x217B` | Execute advance branch | Handled | Guard: x64 case RVA `0x835e0`, present in the supplied script corpus | [adapter.rs:3124](../../crates/crysirror-cmvs/src/adapter.rs#L3124) |
| `0x217C` | Execute advance branch | Handled | Guard: x64 case RVA `0x835ed`, present in the supplied script corpus | [adapter.rs:3124](../../crates/crysirror-cmvs/src/adapter.rs#L3124) |
| `0x217D` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x217E` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x835fa` | [cmvs-20160201.json:29](../../tools/native-api/cmvs-20160201.json#L29) |
| `0x217F` | Execute advance branch | Handled | Guard: x64 case RVA `0x83607`, present in the supplied script corpus | [adapter.rs:3124](../../crates/crysirror-cmvs/src/adapter.rs#L3124) |
| `0x2180` | Execute advance branch | Handled | Guard: x64 case RVA `0x83614`, present in the supplied script corpus | [adapter.rs:3124](../../crates/crysirror-cmvs/src/adapter.rs#L3124) |
| `0x2181` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x83621` | [cmvs-20160201.json:30](../../tools/native-api/cmvs-20160201.json#L30) |
| `0x2182` | Execute advance branch | Handled | Guard: x64 case RVA `0x8362e`, present in the supplied script corpus | [adapter.rs:3124](../../crates/crysirror-cmvs/src/adapter.rs#L3124) |
| `0x2183` | Execute advance branch | Handled | Guard: x64 case RVA `0x8363b`, present in the supplied script corpus | [adapter.rs:3124](../../crates/crysirror-cmvs/src/adapter.rs#L3124) |
| `0x2184` | Execute advance branch | Handled | Guard: x64 case RVA `0x83648`, present in the supplied script corpus | [adapter.rs:3124](../../crates/crysirror-cmvs/src/adapter.rs#L3124) |
| `0x2185` | Execute advance branch | Handled | Guard: x64 case RVA `0x83655`, present in the supplied script corpus | [adapter.rs:3124](../../crates/crysirror-cmvs/src/adapter.rs#L3124) |
| `0x2186` | Execute advance branch | Handled | Guard: x64 case RVA `0x83662`, present in the supplied script corpus | [adapter.rs:3124](../../crates/crysirror-cmvs/src/adapter.rs#L3124) |
| `0x2187` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x8366f` | [cmvs-20160201.json:30](../../tools/native-api/cmvs-20160201.json#L30) |
| `0x2188` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2189` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x8367c` | [cmvs-20160201.json:30](../../tools/native-api/cmvs-20160201.json#L30) |
| `0x218A` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x218B` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x83689` | [cmvs-20160201.json:30](../../tools/native-api/cmvs-20160201.json#L30) |
| `0x218C` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x218D` | Execute advance branch | Handled | Guard: x64 case RVA `0x83696`, present in the supplied script corpus | [adapter.rs:3124](../../crates/crysirror-cmvs/src/adapter.rs#L3124) |
| `0x218E` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x218F` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2190` | Execute advance branch | Handled | Guard: x64 case RVA `0x836a3`, present in the supplied script corpus | [adapter.rs:3124](../../crates/crysirror-cmvs/src/adapter.rs#L3124) |
| `0x2191` | Execute advance branch | Handled | Guard: x64 case RVA `0x836b0`, present in the supplied script corpus | [adapter.rs:3124](../../crates/crysirror-cmvs/src/adapter.rs#L3124) |
| `0x2192` | Execute advance branch | Handled | Guard: x64 case RVA `0x836bd`, present in the supplied script corpus | [adapter.rs:3124](../../crates/crysirror-cmvs/src/adapter.rs#L3124) |
| `0x2193` | Execute advance branch | Handled | Guard: x64 case RVA `0x836ca` | [adapter.rs:3124](../../crates/crysirror-cmvs/src/adapter.rs#L3124) |
| `0x2194` | Execute advance branch | Handled | Guard: x64 case RVA `0x836d7`, present in the supplied script corpus | [adapter.rs:3124](../../crates/crysirror-cmvs/src/adapter.rs#L3124) |
| `0x2195` | Execute advance branch | Handled | Guard: x64 case RVA `0x836e4`, present in the supplied script corpus | [adapter.rs:3124](../../crates/crysirror-cmvs/src/adapter.rs#L3124) |
| `0x2196` | Execute advance branch | Handled | Guard: x64 case RVA `0x836f1`, present in the supplied script corpus | [adapter.rs:3124](../../crates/crysirror-cmvs/src/adapter.rs#L3124) |
| `0x2197` | Execute advance branch | Handled | Guard: x64 case RVA `0x836fe` | [adapter.rs:3124](../../crates/crysirror-cmvs/src/adapter.rs#L3124) |
| `0x2198` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2199` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x219A` | Input (command) | Handled | Guard: x64 case RVA `0x8370b`, present in the supplied script corpus | [input.rs:220](../../crates/crysirror-cmvs/src/vm/input.rs#L220) |
| `0x219B` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x219C` | Read or consume an input edge | Handled | Guard: x64 case RVA `0x83722`, present in the supplied script corpus | [input.rs:9](../../crates/crysirror-cmvs/src/vm/input.rs#L9) |
| `0x219D` | Execute advance branch | Handled | Guard: x64 case RVA `0x8372f`, present in the supplied script corpus | [adapter.rs:3304](../../crates/crysirror-cmvs/src/adapter.rs#L3304) |
| `0x219E` | Read or consume an input edge | Handled | Guard: x64 case RVA `0x83746`, present in the supplied script corpus | [input.rs:10](../../crates/crysirror-cmvs/src/vm/input.rs#L10) |
| `0x219F` | Execute advance branch | Handled | Guard: x64 case RVA `0x83753`, present in the supplied script corpus | [adapter.rs:3304](../../crates/crysirror-cmvs/src/adapter.rs#L3304) |
| `0x21A0` | Read or consume an input edge | Handled | Guard: x64 case RVA `0x8376a`, present in the supplied script corpus | [input.rs:11](../../crates/crysirror-cmvs/src/vm/input.rs#L11) |
| `0x21A1` | Execute advance branch | Handled | Guard: x64 case RVA `0x83777`, present in the supplied script corpus | [adapter.rs:3304](../../crates/crysirror-cmvs/src/adapter.rs#L3304) |
| `0x21A2` | Read or consume an input edge | Handled | Guard: x64 case RVA `0x8378e`, present in the supplied script corpus | [input.rs:12](../../crates/crysirror-cmvs/src/vm/input.rs#L12) |
| `0x21A3` | Execute advance branch | Handled | Guard: x64 case RVA `0x8379b`, present in the supplied script corpus | [adapter.rs:3304](../../crates/crysirror-cmvs/src/adapter.rs#L3304) |
| `0x21A4` | Read or consume an input edge | Handled | Guard: x64 case RVA `0x837b2`, present in the supplied script corpus | [input.rs:13](../../crates/crysirror-cmvs/src/vm/input.rs#L13) |
| `0x21A5` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x21A6` | Read or consume an input edge | Handled | Guard: x64 case RVA `0x837bf`, present in the supplied script corpus | [input.rs:14](../../crates/crysirror-cmvs/src/vm/input.rs#L14) |
| `0x21A7` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x21A8` | Read or consume an input edge | Handled | Guard: x64 case RVA `0x837cc`, present in the supplied script corpus | [input.rs:15](../../crates/crysirror-cmvs/src/vm/input.rs#L15) |
| `0x21A9` | Execute advance branch | Handled | Guard: x64 case RVA `0x837d9`, present in the supplied script corpus | [adapter.rs:3304](../../crates/crysirror-cmvs/src/adapter.rs#L3304) |
| `0x21AA` | Read or consume an input edge | Handled | Guard: x64 case RVA `0x837f0`, present in the supplied script corpus | [input.rs:16](../../crates/crysirror-cmvs/src/vm/input.rs#L16) |
| `0x21AB` | Execute advance branch | Handled | Guard: x64 case RVA `0x837fd`, present in the supplied script corpus | [adapter.rs:3304](../../crates/crysirror-cmvs/src/adapter.rs#L3304) |
| `0x21AC` | Read or consume an input edge | Handled | Guard: x64 case RVA `0x83814`, present in the supplied script corpus | [input.rs:17](../../crates/crysirror-cmvs/src/vm/input.rs#L17) |
| `0x21AD` | Execute advance branch | Handled | Guard: x64 case RVA `0x83821`, present in the supplied script corpus | [adapter.rs:3304](../../crates/crysirror-cmvs/src/adapter.rs#L3304) |
| `0x21AE` | Read or consume an input edge | Handled | Guard: x64 case RVA `0x83838`, present in the supplied script corpus | [input.rs:18](../../crates/crysirror-cmvs/src/vm/input.rs#L18) |
| `0x21AF` | Execute advance branch | Handled | Guard: x64 case RVA `0x83845`, present in the supplied script corpus | [adapter.rs:3304](../../crates/crysirror-cmvs/src/adapter.rs#L3304) |
| `0x21B0` | Read or consume an input edge | Handled | Guard: x64 case RVA `0x8385c` | [input.rs:19](../../crates/crysirror-cmvs/src/vm/input.rs#L19) |
| `0x21B1` | Execute advance branch | Handled | Guard: x64 case RVA `0x83869` | [adapter.rs:3304](../../crates/crysirror-cmvs/src/adapter.rs#L3304) |
| `0x21B2` | Read or consume an input edge | Handled | Guard: x64 case RVA `0x83880` | [input.rs:20](../../crates/crysirror-cmvs/src/vm/input.rs#L20) |
| `0x21B3` | Execute advance branch | Handled | Guard: x64 case RVA `0x8388d` | [adapter.rs:3304](../../crates/crysirror-cmvs/src/adapter.rs#L3304) |
| `0x21B4` | Read or consume an input edge | Handled | Guard: x64 case RVA `0x838a4` | [input.rs:21](../../crates/crysirror-cmvs/src/vm/input.rs#L21) |
| `0x21B5` | Execute advance branch | Handled | Guard: x64 case RVA `0x838b1` | [adapter.rs:3304](../../crates/crysirror-cmvs/src/adapter.rs#L3304) |
| `0x21B6` | Read or consume an input edge | Handled | Guard: x64 case RVA `0x838c8` | [input.rs:22](../../crates/crysirror-cmvs/src/vm/input.rs#L22) |
| `0x21B7` | Execute advance branch | Handled | Guard: x64 case RVA `0x838d5` | [adapter.rs:3304](../../crates/crysirror-cmvs/src/adapter.rs#L3304) |
| `0x21B8` | Read or consume an input edge | Handled | Guard: x64 case RVA `0x838ec` | [input.rs:23](../../crates/crysirror-cmvs/src/vm/input.rs#L23) |
| `0x21B9` | Execute advance branch | Handled | Guard: x64 case RVA `0x838f9` | [adapter.rs:3304](../../crates/crysirror-cmvs/src/adapter.rs#L3304) |
| `0x21BA` | Read or consume an input edge | Handled | Guard: x64 case RVA `0x83910` | [input.rs:24](../../crates/crysirror-cmvs/src/vm/input.rs#L24) |
| `0x21BB` | Execute advance branch | Handled | Guard: x64 case RVA `0x8391d` | [adapter.rs:3304](../../crates/crysirror-cmvs/src/adapter.rs#L3304) |
| `0x21BC` | Read or consume an input edge | Handled | Guard: x64 case RVA `0x83934` | [input.rs:25](../../crates/crysirror-cmvs/src/vm/input.rs#L25) |
| `0x21BD` | Execute advance branch | Handled | Guard: x64 case RVA `0x83941` | [adapter.rs:3304](../../crates/crysirror-cmvs/src/adapter.rs#L3304) |
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
| `0x21CF` | Execute advance branch | Handled | Guard: x64 case RVA `0x839f4`, present in the supplied script corpus | [adapter.rs:3124](../../crates/crysirror-cmvs/src/adapter.rs#L3124) |
| `0x21D0` | Execute advance branch | Handled | Guard: x64 case RVA `0x83a01`, present in the supplied script corpus | [adapter.rs:3124](../../crates/crysirror-cmvs/src/adapter.rs#L3124) |
| `0x21D1` | Execute advance branch | Handled | Guard: x64 case RVA `0x83a0e`, present in the supplied script corpus | [adapter.rs:3124](../../crates/crysirror-cmvs/src/adapter.rs#L3124) |
| `0x21D2` | Execute advance branch | Handled | Guard: x64 case RVA `0x83a1b` | [adapter.rs:3124](../../crates/crysirror-cmvs/src/adapter.rs#L3124) |
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
| `0x21E0` | Execute advance branch | Handled | Guard: x64 case RVA `0x83a80`, present in the supplied script corpus | [adapter.rs:4598](../../crates/crysirror-cmvs/src/adapter.rs#L4598) |
| `0x21E1` | Execute advance branch | Handled | Guard: x64 case RVA `0x83a8d`, present in the supplied script corpus | [adapter.rs:4598](../../crates/crysirror-cmvs/src/adapter.rs#L4598) |
| `0x21E2` | Execute advance branch | Handled | Guard: x64 case RVA `0x83a9a`, present in the supplied script corpus | [adapter.rs:4598](../../crates/crysirror-cmvs/src/adapter.rs#L4598) |
| `0x21E3` | Execute advance branch | Handled | Guard: x64 case RVA `0x83aa7`, present in the supplied script corpus | [adapter.rs:4598](../../crates/crysirror-cmvs/src/adapter.rs#L4598) |
| `0x21E4` | Execute advance branch | Handled | Guard: x64 case RVA `0x83ab4`, present in the supplied script corpus | [adapter.rs:4598](../../crates/crysirror-cmvs/src/adapter.rs#L4598) |
| `0x21E5` | Execute advance branch | Handled | Guard: x64 case RVA `0x83ac1` | [adapter.rs:4598](../../crates/crysirror-cmvs/src/adapter.rs#L4598) |
| `0x21E6` | Execute advance branch | Handled | Guard: x64 case RVA `0x83ace` | [adapter.rs:4598](../../crates/crysirror-cmvs/src/adapter.rs#L4598) |
| `0x21E7` | Execute advance branch | Handled | Guard: x64 case RVA `0x83adb` | [adapter.rs:4598](../../crates/crysirror-cmvs/src/adapter.rs#L4598) |
| `0x21E8` | Execute advance branch | Handled | Guard: x64 case RVA `0x83ae8` | [adapter.rs:4598](../../crates/crysirror-cmvs/src/adapter.rs#L4598) |
| `0x21E9` | Execute advance branch | Handled | Guard: x64 case RVA `0x83af5` | [adapter.rs:4598](../../crates/crysirror-cmvs/src/adapter.rs#L4598) |
| `0x21EA` | Execute advance branch | Handled | Guard: x64 case RVA `0x83b02`, present in the supplied script corpus | [adapter.rs:4593](../../crates/crysirror-cmvs/src/adapter.rs#L4593) |
| `0x21EB` | Execute advance branch | Handled | Guard: x64 case RVA `0x83b0f`, present in the supplied script corpus | [adapter.rs:4593](../../crates/crysirror-cmvs/src/adapter.rs#L4593) |
| `0x21EC` | Execute advance branch | Handled | Guard: x64 case RVA `0x83b1c`, present in the supplied script corpus | [adapter.rs:4593](../../crates/crysirror-cmvs/src/adapter.rs#L4593) |
| `0x21ED` | Execute advance branch | Handled | Guard: x64 case RVA `0x83b29`, present in the supplied script corpus | [adapter.rs:4593](../../crates/crysirror-cmvs/src/adapter.rs#L4593) |
| `0x21EE` | Execute advance branch | Handled | Guard: x64 case RVA `0x83b36`, present in the supplied script corpus | [adapter.rs:4593](../../crates/crysirror-cmvs/src/adapter.rs#L4593) |
| `0x21EF` | Execute advance branch | Handled | Guard: x64 case RVA `0x83b43`, present in the supplied script corpus | [adapter.rs:4598](../../crates/crysirror-cmvs/src/adapter.rs#L4598) |
| `0x21F0` | Execute advance branch | Handled | Guard: x64 case RVA `0x83b50`, present in the supplied script corpus | [adapter.rs:4598](../../crates/crysirror-cmvs/src/adapter.rs#L4598) |
| `0x21F1` | Execute advance branch | Handled | Guard: x64 case RVA `0x83b5d`, present in the supplied script corpus | [adapter.rs:4598](../../crates/crysirror-cmvs/src/adapter.rs#L4598) |
| `0x21F2` | Execute advance branch | Handled | Guard: x64 case RVA `0x83b6a` | [adapter.rs:4598](../../crates/crysirror-cmvs/src/adapter.rs#L4598) |
| `0x21F3` | Execute advance branch | Handled | Guard: x64 case RVA `0x83b77` | [adapter.rs:4598](../../crates/crysirror-cmvs/src/adapter.rs#L4598) |
| `0x21F4` | Execute advance branch | Handled | Guard: x64 case RVA `0x83b84`, present in the supplied script corpus | [adapter.rs:4598](../../crates/crysirror-cmvs/src/adapter.rs#L4598) |
| `0x21F5` | Execute advance branch | Handled | Guard: x64 case RVA `0x83b91`, present in the supplied script corpus | [adapter.rs:4598](../../crates/crysirror-cmvs/src/adapter.rs#L4598) |
| `0x21F6` | Execute advance branch | Handled | Guard: x64 case RVA `0x83b9e`, present in the supplied script corpus | [adapter.rs:4598](../../crates/crysirror-cmvs/src/adapter.rs#L4598) |
| `0x21F7` | Execute advance branch | Handled | Guard: x64 case RVA `0x83bab` | [adapter.rs:4598](../../crates/crysirror-cmvs/src/adapter.rs#L4598) |
| `0x21F8` | Execute advance branch | Handled | Guard: x64 case RVA `0x83bb8` | [adapter.rs:4598](../../crates/crysirror-cmvs/src/adapter.rs#L4598) |
| `0x21F9` | Execute advance branch | Handled | Guard: x64 case RVA `0x83bc5`, present in the supplied script corpus | [adapter.rs:4598](../../crates/crysirror-cmvs/src/adapter.rs#L4598) |
| `0x21FA` | Execute advance branch | Handled | Guard: x64 case RVA `0x83bd2`, present in the supplied script corpus | [adapter.rs:4598](../../crates/crysirror-cmvs/src/adapter.rs#L4598) |
| `0x21FB` | Execute advance branch | Handled | Guard: x64 case RVA `0x83bdf`, present in the supplied script corpus | [adapter.rs:4598](../../crates/crysirror-cmvs/src/adapter.rs#L4598) |
| `0x21FC` | Execute advance branch | Handled | Guard: x64 case RVA `0x83bec`, present in the supplied script corpus | [adapter.rs:4598](../../crates/crysirror-cmvs/src/adapter.rs#L4598) |
| `0x21FD` | Execute advance branch | Handled | Guard: x64 case RVA `0x83bf9`, present in the supplied script corpus | [adapter.rs:4598](../../crates/crysirror-cmvs/src/adapter.rs#L4598) |
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
| `0x220C` | Execute advance branch | Handled | Guard: x64 case RVA `0x83c06`, present in the supplied script corpus | [adapter.rs:4608](../../crates/crysirror-cmvs/src/adapter.rs#L4608) |
| `0x220D` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x220E` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x220F` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2210` | Execute advance branch | Handled | Guard: x64 case RVA `0x83c13`, present in the supplied script corpus | [adapter.rs:3114](../../crates/crysirror-cmvs/src/adapter.rs#L3114) |
| `0x2211` | Execute advance branch | Handled | Guard: x64 case RVA `0x83c20`, present in the supplied script corpus | [adapter.rs:3114](../../crates/crysirror-cmvs/src/adapter.rs#L3114) |
| `0x2212` | Execute advance branch | Handled | Guard: x64 case RVA `0x83c2d`, present in the supplied script corpus | [adapter.rs:3114](../../crates/crysirror-cmvs/src/adapter.rs#L3114) |
| `0x2213` | Execute advance branch | Handled | Guard: x64 case RVA `0x83c3a`, present in the supplied script corpus | [adapter.rs:3114](../../crates/crysirror-cmvs/src/adapter.rs#L3114) |
| `0x2214` | Execute advance branch | Handled | Guard: x64 case RVA `0x83c47`, present in the supplied script corpus | [adapter.rs:3114](../../crates/crysirror-cmvs/src/adapter.rs#L3114) |
| `0x2215` | Execute advance branch | Handled | Guard: x64 case RVA `0x83c54`, present in the supplied script corpus | [adapter.rs:3114](../../crates/crysirror-cmvs/src/adapter.rs#L3114) |
| `0x2216` | Execute advance branch | Handled | Guard: x64 case RVA `0x83c61`, present in the supplied script corpus | [adapter.rs:3114](../../crates/crysirror-cmvs/src/adapter.rs#L3114) |
| `0x2217` | Execute advance branch | Handled | Guard: x64 case RVA `0x83c6e`, present in the supplied script corpus | [adapter.rs:3114](../../crates/crysirror-cmvs/src/adapter.rs#L3114) |
| `0x2218` | Execute advance branch | Handled | Guard: x64 case RVA `0x83c7b` | [adapter.rs:3114](../../crates/crysirror-cmvs/src/adapter.rs#L3114) |
| `0x2219` | Execute advance branch | Handled | Guard: x64 case RVA `0x83c88` | [adapter.rs:3114](../../crates/crysirror-cmvs/src/adapter.rs#L3114) |
| `0x221A` | Execute advance branch | Handled | Guard: x64 case RVA `0x83c95`, present in the supplied script corpus | [adapter.rs:3114](../../crates/crysirror-cmvs/src/adapter.rs#L3114) |
| `0x221B` | Execute advance branch | Handled | Guard: x64 case RVA `0x83ca2`, present in the supplied script corpus | [adapter.rs:3114](../../crates/crysirror-cmvs/src/adapter.rs#L3114) |
| `0x221C` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x221D` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x221E` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x221F` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2220` | Execute advance branch | Handled | Guard: x64 case RVA `0x83caf`, present in the supplied script corpus | [adapter.rs:2666](../../crates/crysirror-cmvs/src/adapter.rs#L2666) |
| `0x2221` | Execute advance branch | Handled | Guard: x64 case RVA `0x83cbc`, present in the supplied script corpus | [adapter.rs:2666](../../crates/crysirror-cmvs/src/adapter.rs#L2666) |
| `0x2222` | Execute advance branch | Handled | Guard: x64 case RVA `0x83cc9` | [adapter.rs:2666](../../crates/crysirror-cmvs/src/adapter.rs#L2666) |
| `0x2223` | Execute advance branch | Handled | Guard: x64 case RVA `0x83cd6` | [adapter.rs:2666](../../crates/crysirror-cmvs/src/adapter.rs#L2666) |
| `0x2224` | Execute advance branch | Handled | Guard: x64 case RVA `0x83ce3`, present in the supplied script corpus | [adapter.rs:2666](../../crates/crysirror-cmvs/src/adapter.rs#L2666) |
| `0x2225` | Execute advance branch | Open | Additional native quake waveform and random direction semantics remain open; Guard: x64 case RVA `0x83cf0`, present in the supplied script corpus | [adapter.rs:4608](../../crates/crysirror-cmvs/src/adapter.rs#L4608) |
| `0x2226` | Execute advance branch | Handled | Guard: x64 case RVA `0x83cfd`, present in the supplied script corpus | [adapter.rs:4608](../../crates/crysirror-cmvs/src/adapter.rs#L4608) |
| `0x2227` | Execute advance branch | Handled | Guard: x64 case RVA `0x83d0a`, present in the supplied script corpus | [adapter.rs:4608](../../crates/crysirror-cmvs/src/adapter.rs#L4608) |
| `0x2228` | Execute advance branch | Handled | Guard: x64 case RVA `0x83d17`, present in the supplied script corpus | [adapter.rs:4608](../../crates/crysirror-cmvs/src/adapter.rs#L4608) |
| `0x2229` | Execute advance branch | Handled | Guard: x64 case RVA `0x83d24`, present in the supplied script corpus | [adapter.rs:4608](../../crates/crysirror-cmvs/src/adapter.rs#L4608) |
| `0x222A` | Update job | Open | Shared shader-equivalent coverage; native CPU-device path is pending; Guard: x64 case RVA `0x83d31`, present in the supplied script corpus | [adapter.rs:4615](../../crates/crysirror-cmvs/src/adapter.rs#L4615) |
| `0x222B` | Execute advance branch | Handled | Guard: x64 case RVA `0x83d3e`, present in the supplied script corpus | [adapter.rs:4649](../../crates/crysirror-cmvs/src/adapter.rs#L4649) |
| `0x222C` | Execute advance branch | Handled | Guard: x64 case RVA `0x83d4b` | [adapter.rs:4674](../../crates/crysirror-cmvs/src/adapter.rs#L4674) |
| `0x222D` | Execute advance branch | Handled | Guard: x64 case RVA `0x83d58` | [adapter.rs:4681](../../crates/crysirror-cmvs/src/adapter.rs#L4681) |
| `0x222E` | Update job | Handled | Guard: x64 case RVA `0x83d65` | [adapter.rs:4213](../../crates/crysirror-cmvs/src/adapter.rs#L4213) |
| `0x222F` | Execute advance branch | Handled | Guard: x64 case RVA `0x83d72`, present in the supplied script corpus | [adapter.rs:3321](../../crates/crysirror-cmvs/src/adapter.rs#L3321) |
| `0x2230` | Execute advance branch | Handled | Guard: x64 case RVA `0x83d7f` | [adapter.rs:3341](../../crates/crysirror-cmvs/src/adapter.rs#L3341) |
| `0x2231` | Execute advance branch | Handled | Guard: x64 case RVA `0x83d8c`, present in the supplied script corpus | [adapter.rs:3341](../../crates/crysirror-cmvs/src/adapter.rs#L3341) |
| `0x2232` | Execute advance branch | Handled | Guard: x64 case RVA `0x83d99` | [adapter.rs:3331](../../crates/crysirror-cmvs/src/adapter.rs#L3331) |
| `0x2233` | Execute advance branch | Handled | Guard: x64 case RVA `0x83da6`, present in the supplied script corpus | [adapter.rs:3348](../../crates/crysirror-cmvs/src/adapter.rs#L3348) |
| `0x2234` | Update job | Open | Shared coverage; CPU-device path and old-target shader retention are pending; Guard: x64 case RVA `0x83db3`, present in the supplied script corpus | [adapter.rs:4615](../../crates/crysirror-cmvs/src/adapter.rs#L4615) |
| `0x2235` | Execute advance branch | Handled | Guard: x64 case RVA `0x83dc0`, present in the supplied script corpus | [adapter.rs:4649](../../crates/crysirror-cmvs/src/adapter.rs#L4649) |
| `0x2236` | Execute advance branch | Handled | Guard: x64 case RVA `0x83dcd` | [adapter.rs:4674](../../crates/crysirror-cmvs/src/adapter.rs#L4674) |
| `0x2237` | Execute advance branch | Handled | Guard: x64 case RVA `0x83dda` | [adapter.rs:4681](../../crates/crysirror-cmvs/src/adapter.rs#L4681) |
| `0x2238` | Execute advance branch | Handled | Guard: x64 case RVA `0x83de7`, present in the supplied script corpus | [adapter.rs:3988](../../crates/crysirror-cmvs/src/adapter.rs#L3988) |
| `0x2239` | Execute advance branch | Handled | Guard: x64 case RVA `0x83df4`, present in the supplied script corpus | [adapter.rs:3988](../../crates/crysirror-cmvs/src/adapter.rs#L3988) |
| `0x223A` | Execute advance branch | Handled | Guard: x64 case RVA `0x83e01`, present in the supplied script corpus | [adapter.rs:3988](../../crates/crysirror-cmvs/src/adapter.rs#L3988) |
| `0x223B` | Execute advance branch | Handled | Guard: x64 case RVA `0x83e0e`, present in the supplied script corpus | [adapter.rs:3988](../../crates/crysirror-cmvs/src/adapter.rs#L3988) |
| `0x223C` | Execute advance branch | Handled | Guard: x64 case RVA `0x83e1b`, present in the supplied script corpus | [adapter.rs:3988](../../crates/crysirror-cmvs/src/adapter.rs#L3988) |
| `0x223D` | Execute advance branch | Handled | Guard: x64 case RVA `0x83e28`, present in the supplied script corpus | [adapter.rs:3988](../../crates/crysirror-cmvs/src/adapter.rs#L3988) |
| `0x223E` | Execute advance branch | Handled | Guard: x64 case RVA `0x83e35` | [adapter.rs:3988](../../crates/crysirror-cmvs/src/adapter.rs#L3988) |
| `0x223F` | Execute advance branch | Handled | Guard: x64 case RVA `0x83e42` | [adapter.rs:3988](../../crates/crysirror-cmvs/src/adapter.rs#L3988) |
| `0x2240` | Update job | Open | Scoped playback is connected; native invalid-slot reporting occurs at the frame boundary; Guard: x64 case RVA `0x83e4f` | [adapter.rs:2915](../../crates/crysirror-cmvs/src/adapter.rs#L2915) |
| `0x2241` | Update job | Handled | Guard: x64 case RVA `0x83e5c`, present in the supplied script corpus | [adapter.rs:2915](../../crates/crysirror-cmvs/src/adapter.rs#L2915) |
| `0x2242` | Update job | Open | Shared elapsed transport time replaces the DirectSound device cursor; Guard: x64 case RVA `0x83e69` | [adapter.rs:2915](../../crates/crysirror-cmvs/src/adapter.rs#L2915) |
| `0x2243` | Execute advance branch | Open | Buffered source levels are connected; output resampling can differ from DirectSound; Guard: x64 case RVA `0x83e76` | [adapter.rs:3002](../../crates/crysirror-cmvs/src/adapter.rs#L3002) |
| `0x2244` | Update job | Open | Quantized multiplier is connected; the renderer uses its own fade clock; Guard: x64 case RVA `0x83e83` | [adapter.rs:2915](../../crates/crysirror-cmvs/src/adapter.rs#L2915) |
| `0x2245` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2246` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2247` | Execute advance branch | Handled | Guard: x64 case RVA `0x83e90` | [adapter.rs:3326](../../crates/crysirror-cmvs/src/adapter.rs#L3326) |
| `0x2248` | Update job | Open | Resident sample loading is connected; native codec compatibility needs validation; Guard: x64 case RVA `0x83e9d` | [adapter.rs:3020](../../crates/crysirror-cmvs/src/adapter.rs#L3020) |
| `0x2249` | Update job | Handled | Guard: x64 case RVA `0x83eaa` | [adapter.rs:3020](../../crates/crysirror-cmvs/src/adapter.rs#L3020) |
| `0x224A` | Update job | Open | Retains the cursor on stop/resume; DirectSound EOF cursor parity is open; Guard: x64 case RVA `0x83eb7` | [adapter.rs:3020](../../crates/crysirror-cmvs/src/adapter.rs#L3020) |
| `0x224B` | Update job | Handled | Guard: x64 case RVA `0x83ec4` | [adapter.rs:3020](../../crates/crysirror-cmvs/src/adapter.rs#L3020) |
| `0x224C` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x224D` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x224E` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x224F` | Update job | Open | Uses 32 duplicate positions; driver gain and voice-cap parity are open; Guard: x64 case RVA `0x83ed1` | [adapter.rs:3020](../../crates/crysirror-cmvs/src/adapter.rs#L3020) |
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
| `0x2260` | Execute advance branch | Handled | Guard: x64 case RVA `0x83eeb`, present in the supplied script corpus | [adapter.rs:3345](../../crates/crysirror-cmvs/src/adapter.rs#L3345) |
| `0x2261` | Execute advance branch | Handled | Guard: x64 case RVA `0x83ef8`, present in the supplied script corpus | [adapter.rs:3345](../../crates/crysirror-cmvs/src/adapter.rs#L3345) |
| `0x2262` | Execute advance branch | Handled | Guard: x64 case RVA `0x83f05`, present in the supplied script corpus | [adapter.rs:3345](../../crates/crysirror-cmvs/src/adapter.rs#L3345) |
| `0x2263` | Execute advance branch | Handled | Guard: x64 case RVA `0x83f12`, present in the supplied script corpus | [adapter.rs:3345](../../crates/crysirror-cmvs/src/adapter.rs#L3345) |
| `0x2264` | Execute advance branch | Handled | Guard: x64 case RVA `0x83f1f`, present in the supplied script corpus | [adapter.rs:3345](../../crates/crysirror-cmvs/src/adapter.rs#L3345) |
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
| `0x2270` | Execute advance branch | Handled | Guard: x64 case RVA `0x83f2c` | [adapter.rs:3345](../../crates/crysirror-cmvs/src/adapter.rs#L3345) |
| `0x2271` | Execute advance branch | Handled | Guard: x64 case RVA `0x83f39` | [adapter.rs:3348](../../crates/crysirror-cmvs/src/adapter.rs#L3348) |
| `0x2272` | Execute advance branch | Handled | Guard: x64 case RVA `0x83f46` | [adapter.rs:3345](../../crates/crysirror-cmvs/src/adapter.rs#L3345) |
| `0x2273` | Execute advance branch | Handled | Guard: x64 case RVA `0x83f53` | [adapter.rs:3345](../../crates/crysirror-cmvs/src/adapter.rs#L3345) |
| `0x2274` | Execute advance branch | Handled | Guard: x64 case RVA `0x83f60` | [adapter.rs:3345](../../crates/crysirror-cmvs/src/adapter.rs#L3345) |
| `0x2275` | Execute advance branch | Handled | Guard: x64 case RVA `0x83f6d` | [adapter.rs:3345](../../crates/crysirror-cmvs/src/adapter.rs#L3345) |
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
| `0x2280` | Execute startup command branch | Handled | Guard: x64 case RVA `0x83f7a`, present in the supplied script corpus | [startup.rs:1752](../../crates/crysirror-cmvs/src/vm/startup.rs#L1752) |
| `0x2281` | Pc | Handled | Guard: x64 case RVA `0x83f91`, present in the supplied script corpus | [startup.rs:1756](../../crates/crysirror-cmvs/src/vm/startup.rs#L1756) |
| `0x2282` | Execute advance branch | Handled | Guard: x64 case RVA `0x83f9e`, present in the supplied script corpus | [adapter.rs:3634](../../crates/crysirror-cmvs/src/adapter.rs#L3634) |
| `0x2283` | Execute startup command branch | Handled | Guard: x64 case RVA `0x83fb5` | [startup.rs:1765](../../crates/crysirror-cmvs/src/vm/startup.rs#L1765) |
| `0x2284` | Update job | Handled | Guard: x64 case RVA `0x83fcc`, present in the supplied script corpus | [adapter.rs:3639](../../crates/crysirror-cmvs/src/adapter.rs#L3639) |
| `0x2285` | Pc | Handled | Guard: x64 case RVA `0x83fd9`, present in the supplied script corpus | [startup.rs:1756](../../crates/crysirror-cmvs/src/vm/startup.rs#L1756) |
| `0x2286` | Required parameter | Handled | Guard: x64 case RVA `0x83fe6`, present in the supplied script corpus | [startup.rs:1769](../../crates/crysirror-cmvs/src/vm/startup.rs#L1769) |
| `0x2287` | Required parameter | Handled | Guard: x64 case RVA `0x83ff3`, present in the supplied script corpus | [startup.rs:1769](../../crates/crysirror-cmvs/src/vm/startup.rs#L1769) |
| `0x2288` | Required parameter | Handled | Guard: x64 case RVA `0x84000`, present in the supplied script corpus | [startup.rs:1769](../../crates/crysirror-cmvs/src/vm/startup.rs#L1769) |
| `0x2289` | Required parameter | Handled | Guard: x64 case RVA `0x8400d`, present in the supplied script corpus | [startup.rs:1769](../../crates/crysirror-cmvs/src/vm/startup.rs#L1769) |
| `0x228A` | Required parameter | Handled | Guard: x64 case RVA `0x8401a`, present in the supplied script corpus | [startup.rs:1769](../../crates/crysirror-cmvs/src/vm/startup.rs#L1769) |
| `0x228B` | Required parameter | Handled | Guard: x64 case RVA `0x84027` | [startup.rs:1769](../../crates/crysirror-cmvs/src/vm/startup.rs#L1769) |
| `0x228C` | Update job | Handled | Guard: x64 case RVA `0x84034`, present in the supplied script corpus | [adapter.rs:3536](../../crates/crysirror-cmvs/src/adapter.rs#L3536) |
| `0x228D` | Update job | Handled | Guard: x64 case RVA `0x84041` | [adapter.rs:3536](../../crates/crysirror-cmvs/src/adapter.rs#L3536) |
| `0x228E` | Update job | Handled | Guard: x64 case RVA `0x8404e`, present in the supplied script corpus | [adapter.rs:3536](../../crates/crysirror-cmvs/src/adapter.rs#L3536) |
| `0x228F` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2290` | Create a textured ground and water scene | Handled | Guard: x64 case RVA `0x8405b` | [adapter.rs:4008](../../crates/crysirror-cmvs/src/adapter.rs#L4008) |
| `0x2291` | Create the walking camera and resident sound scene | Handled | Guard: x64 case RVA `0x84068` | [adapter.rs:4008](../../crates/crysirror-cmvs/src/adapter.rs#L4008) |
| `0x2292` | Create the rotating quad scene with staggered fades and resident cues | Handled | Guard: x64 case RVA `0x84075` | [adapter.rs:4008](../../crates/crysirror-cmvs/src/adapter.rs#L4008) |
| `0x2293` | Create projected panels with width-selected textures and rule-image reveals | Handled | Guard: x64 case RVA `0x84082` | [adapter.rs:4008](../../crates/crysirror-cmvs/src/adapter.rs#L4008) |
| `0x2294` | Create the rising planes, drifting billboards and resident cue | Handled | Guard: x64 case RVA `0x8408f` | [adapter.rs:4008](../../crates/crysirror-cmvs/src/adapter.rs#L4008) |
| `0x2295` | Create the three projected atlas actors and resident sound choreography | Handled | Guard: x64 case RVA `0x8409c` | [adapter.rs:4008](../../crates/crysirror-cmvs/src/adapter.rs#L4008) |
| `0x2296` | Create the fixed ground and seven rotating quads | Handled | Guard: x64 case RVA `0x840a9` | [adapter.rs:4008](../../crates/crysirror-cmvs/src/adapter.rs#L4008) |
| `0x2297` | Create the orbiting atlas scene and its triggerable overlays | Handled | Guard: x64 case RVA `0x840b6` | [adapter.rs:4008](../../crates/crysirror-cmvs/src/adapter.rs#L4008) |
| `0x2298` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2299` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x229A` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x229B` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x229C` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x840c3` | [cmvs-20160201.json:47](../../tools/native-api/cmvs-20160201.json#L47) |
| `0x229D` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x840ef` | [cmvs-20160201.json:47](../../tools/native-api/cmvs-20160201.json#L47) |
| `0x229E` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x229F` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x22A0` | Execute advance branch | Handled | Guard: x64 case RVA `0x8411b`, present in the supplied script corpus | [adapter.rs:4073](../../crates/crysirror-cmvs/src/adapter.rs#L4073) |
| `0x22A1` | Update job | Handled | Guard: x64 case RVA `0x84128` | [adapter.rs:4137](../../crates/crysirror-cmvs/src/adapter.rs#L4137) |
| `0x22A2` | Update job | Handled | Guard: x64 case RVA `0x84135`, present in the supplied script corpus | [adapter.rs:4137](../../crates/crysirror-cmvs/src/adapter.rs#L4137) |
| `0x22A3` | Update job | Handled | Guard: x64 case RVA `0x84142` | [adapter.rs:4137](../../crates/crysirror-cmvs/src/adapter.rs#L4137) |
| `0x22A4` | Update job | Handled | Guard: x64 case RVA `0x8414f` | [adapter.rs:4137](../../crates/crysirror-cmvs/src/adapter.rs#L4137) |
| `0x22A5` | Update job | Handled | Guard: x64 case RVA `0x8415c` | [adapter.rs:4137](../../crates/crysirror-cmvs/src/adapter.rs#L4137) |
| `0x22A6` | Update job | Handled | Guard: x64 case RVA `0x84169` | [adapter.rs:4137](../../crates/crysirror-cmvs/src/adapter.rs#L4137) |
| `0x22A7` | Update job | Handled | Guard: x64 case RVA `0x84176` | [adapter.rs:4137](../../crates/crysirror-cmvs/src/adapter.rs#L4137) |
| `0x22A8` | Execute advance branch | Handled | Guard: x64 case RVA `0x84183`, present in the supplied script corpus | [adapter.rs:4090](../../crates/crysirror-cmvs/src/adapter.rs#L4090) |
| `0x22A9` | Update job | Handled | Guard: x64 case RVA `0x84190` | [adapter.rs:4137](../../crates/crysirror-cmvs/src/adapter.rs#L4137) |
| `0x22AA` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x22AB` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x22AC` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x22AD` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x22AE` | Execute advance branch | Handled | Guard: x64 case RVA `0x8419d`, present in the supplied script corpus | [adapter.rs:4079](../../crates/crysirror-cmvs/src/adapter.rs#L4079) |
| `0x22AF` | Trigger a scene or change its native mode | Handled | Guard: x64 case RVA `0x841aa` | [adapter.rs:4108](../../crates/crysirror-cmvs/src/adapter.rs#L4108) |
| `0x22B0` | Required parameter | Handled | Guard: x64 case RVA `0x841b7`, present in the supplied script corpus | [startup.rs:1740](../../crates/crysirror-cmvs/src/vm/startup.rs#L1740) |
| `0x22B1` | Set the maximum number of history entries captured in a save | Handled | Guard: x64 case RVA `0x841c4` | [startup.rs:1747](../../crates/crysirror-cmvs/src/vm/startup.rs#L1747) |
| `0x22B2` | Execute advance branch | Open | In-memory logical capture; persistent gameplay save/restore is pending; Guard: x64 case RVA `0x841e7`, present in the supplied script corpus | [adapter.rs:2420](../../crates/crysirror-cmvs/src/adapter.rs#L2420) |
| `0x22B3` | Update job | Handled | Guard: x64 case RVA `0x841f4`, present in the supplied script corpus | [adapter.rs:2408](../../crates/crysirror-cmvs/src/adapter.rs#L2408) |
| `0x22B4` | Execute advance branch | Handled | Guard: x64 case RVA `0x84201`, present in the supplied script corpus | [adapter.rs:2429](../../crates/crysirror-cmvs/src/adapter.rs#L2429) |
| `0x22B5` | Restore a native CSV slot | Open | The connected reader accepts CrySirror checkpoints. Original CSV2 native state reconstruction is pending; Guard: x64 case RVA `0x8420e`, present in the supplied script corpus | [adapter.rs:3701](../../crates/crysirror-cmvs/src/adapter.rs#L3701) |
| `0x22B6` | Write a native CSV slot | Open | The connected writer stores a CrySirror checkpoint in CSV2 framing. Native checkpoint records are pending; Guard: x64 case RVA `0x8421b`, present in the supplied script corpus | [adapter.rs:3654](../../crates/crysirror-cmvs/src/adapter.rs#L3654) |
| `0x22B7` | Find the newest native save header in a slot range | Handled | Guard: x64 case RVA `0x84228`, present in the supplied script corpus | [adapter.rs:2444](../../crates/crysirror-cmvs/src/adapter.rs#L2444) |
| `0x22B8` | Capture the thumbnail used by the next native save | Handled | Guard: x64 case RVA `0x84235`, present in the supplied script corpus | [adapter.rs:2470](../../crates/crysirror-cmvs/src/adapter.rs#L2470) |
| `0x22B9` | Required parameter | Handled | Guard: x64 case RVA `0x84248`, present in the supplied script corpus | [startup.rs:1801](../../crates/crysirror-cmvs/src/vm/startup.rs#L1801) |
| `0x22BA` | Required parameter | Handled | Guard: x64 case RVA `0x8426b`, present in the supplied script corpus | [startup.rs:1809](../../crates/crysirror-cmvs/src/vm/startup.rs#L1809) |
| `0x22BB` | Test whether a native CSV save slot exists | Handled | Guard: x64 case RVA `0x84298`, present in the supplied script corpus | [adapter.rs:2475](../../crates/crysirror-cmvs/src/adapter.rs#L2475) |
| `0x22BC` | Update the native save slot comment | Handled | Guard: x64 case RVA `0x842a5` | [adapter.rs:2598](../../crates/crysirror-cmvs/src/adapter.rs#L2598) |
| `0x22BD` | Read native save metadata and replace a scene object's preview | Handled | Guard: x64 case RVA `0x842b2`, present in the supplied script corpus | [adapter.rs:2496](../../crates/crysirror-cmvs/src/adapter.rs#L2496) |
| `0x22BE` | Update two native save metadata words | Handled | Guard: x64 case RVA `0x842bf` | [adapter.rs:2598](../../crates/crysirror-cmvs/src/adapter.rs#L2598) |
| `0x22BF` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x22C0` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x842cc` | [cmvs-20160201.json:50](../../tools/native-api/cmvs-20160201.json#L50) |
| `0x22C1` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x842d9` | [cmvs-20160201.json:50](../../tools/native-api/cmvs-20160201.json#L50) |
| `0x22C2` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x22C3` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x22C4` | Update job | Handled | Guard: x64 case RVA `0x842e6`, present in the supplied script corpus | [adapter.rs:3393](../../crates/crysirror-cmvs/src/adapter.rs#L3393) |
| `0x22C5` | Execute advance branch | Handled | Guard: x64 case RVA `0x842f3`, present in the supplied script corpus | [adapter.rs:3432](../../crates/crysirror-cmvs/src/adapter.rs#L3432) |
| `0x22C6` | Execute advance branch | Handled | Guard: x64 case RVA `0x84300`, present in the supplied script corpus | [adapter.rs:3437](../../crates/crysirror-cmvs/src/adapter.rs#L3437) |
| `0x22C7` | Execute advance branch | Handled | Guard: x64 case RVA `0x8430d` | [adapter.rs:3442](../../crates/crysirror-cmvs/src/adapter.rs#L3442) |
| `0x22C8` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x8431a`, present in the supplied script corpus | [cmvs-20160201.json:50](../../tools/native-api/cmvs-20160201.json#L50) |
| `0x22C9` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x84327`, present in the supplied script corpus | [cmvs-20160201.json:50](../../tools/native-api/cmvs-20160201.json#L50) |
| `0x22CA` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x84334` | [cmvs-20160201.json:50](../../tools/native-api/cmvs-20160201.json#L50) |
| `0x22CB` | Execute advance branch | Handled | Guard: x64 case RVA `0x84341`, present in the supplied script corpus | [adapter.rs:2648](../../crates/crysirror-cmvs/src/adapter.rs#L2648) |
| `0x22CC` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x22CD` | Execute advance branch | Handled | Guard: x64 case RVA `0x8434e`, present in the supplied script corpus | [adapter.rs:2628](../../crates/crysirror-cmvs/src/adapter.rs#L2628) |
| `0x22CE` | Pc | Handled | Guard: x64 case RVA `0x8437a`, present in the supplied script corpus | [startup.rs:1841](../../crates/crysirror-cmvs/src/vm/startup.rs#L1841) |
| `0x22CF` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x84391` | [cmvs-20160201.json:50](../../tools/native-api/cmvs-20160201.json#L50) |
| `0x22D0` | Execute advance branch | Handled | Guard: x64 case RVA `0x843a6`, present in the supplied script corpus | [adapter.rs:3165](../../crates/crysirror-cmvs/src/adapter.rs#L3165) |
| `0x22D1` | Update visual effects | Handled | Guard: x64 case RVA `0x843b3`, present in the supplied script corpus | [adapter.rs:4692](../../crates/crysirror-cmvs/src/adapter.rs#L4692) |
| `0x22D2` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x843c0` | [cmvs-20160201.json:51](../../tools/native-api/cmvs-20160201.json#L51) |
| `0x22D3` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x843cd`, present in the supplied script corpus | [cmvs-20160201.json:51](../../tools/native-api/cmvs-20160201.json#L51) |
| `0x22D4` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x843da`, present in the supplied script corpus | [cmvs-20160201.json:51](../../tools/native-api/cmvs-20160201.json#L51) |
| `0x22D5` | Execute advance branch | Handled | Guard: x64 case RVA `0x843e7`, present in the supplied script corpus | [adapter.rs:3166](../../crates/crysirror-cmvs/src/adapter.rs#L3166) |
| `0x22D6` | Update job | Handled | Guard: x64 case RVA `0x843f4`, present in the supplied script corpus | [adapter.rs:3178](../../crates/crysirror-cmvs/src/adapter.rs#L3178) |
| `0x22D7` | Update job | Open | Effect reopen restores per-slot multiplier and pan; native settings persistence is pending; Guard: x64 case RVA `0x84401`, present in the supplied script corpus | [adapter.rs:3178](../../crates/crysirror-cmvs/src/adapter.rs#L3178) |
| `0x22D8` | Update job | Handled | Guard: x64 case RVA `0x8440e`, present in the supplied script corpus | [adapter.rs:3178](../../crates/crysirror-cmvs/src/adapter.rs#L3178) |
| `0x22D9` | Execute advance branch | Handled | Guard: x64 case RVA `0x8441b`, present in the supplied script corpus | [adapter.rs:3297](../../crates/crysirror-cmvs/src/adapter.rs#L3297) |
| `0x22DA` | Execute advance branch | Handled | Guard: x64 case RVA `0x84428`, present in the supplied script corpus | [adapter.rs:3297](../../crates/crysirror-cmvs/src/adapter.rs#L3297) |
| `0x22DB` | Execute advance branch | Handled | Guard: x64 case RVA `0x84435`, present in the supplied script corpus | [adapter.rs:3297](../../crates/crysirror-cmvs/src/adapter.rs#L3297) |
| `0x22DC` | Execute advance branch | Handled | Guard: x64 case RVA `0x84442`, present in the supplied script corpus | [adapter.rs:3297](../../crates/crysirror-cmvs/src/adapter.rs#L3297) |
| `0x22DD` | Execute advance branch | Handled | Guard: x64 case RVA `0x8444f`, present in the supplied script corpus | [adapter.rs:3297](../../crates/crysirror-cmvs/src/adapter.rs#L3297) |
| `0x22DE` | Execute advance branch | Open | Font alias selection is connected; GDI font discovery and INI writeback remain open; Guard: x64 case RVA `0x8445c`, present in the supplied script corpus | [adapter.rs:3298](../../crates/crysirror-cmvs/src/adapter.rs#L3298) |
| `0x22DF` | Execute advance branch | Open | Both nonzero outline modes currently share the same glyph outline; Guard: x64 case RVA `0x84469`, present in the supplied script corpus | [adapter.rs:3298](../../crates/crysirror-cmvs/src/adapter.rs#L3298) |
| `0x22E0` | Execute advance branch | Open | Glyph weight follows the setting; exact native font metrics remain open; Guard: x64 case RVA `0x84476`, present in the supplied script corpus | [adapter.rs:3298](../../crates/crysirror-cmvs/src/adapter.rs#L3298) |
| `0x22E1` | Execute advance branch | Handled | Guard: x64 case RVA `0x84483`, present in the supplied script corpus | [adapter.rs:3298](../../crates/crysirror-cmvs/src/adapter.rs#L3298) |
| `0x22E2` | Execute advance branch | Handled | Guard: x64 case RVA `0x84490` | [adapter.rs:3298](../../crates/crysirror-cmvs/src/adapter.rs#L3298) |
| `0x22E3` | Execute advance branch | Handled | Guard: x64 case RVA `0x8449d` | [adapter.rs:3298](../../crates/crysirror-cmvs/src/adapter.rs#L3298) |
| `0x22E4` | Execute advance branch | Handled | Guard: x64 case RVA `0x844aa`, present in the supplied script corpus | [adapter.rs:3298](../../crates/crysirror-cmvs/src/adapter.rs#L3298) |
| `0x22E5` | Pc | Handled | Guard: x64 case RVA `0x844b7`, present in the supplied script corpus | [startup.rs:1815](../../crates/crysirror-cmvs/src/vm/startup.rs#L1815) |
| `0x22E6` | Execute advance branch | Handled | Guard: x64 case RVA `0x844c4` | [adapter.rs:3298](../../crates/crysirror-cmvs/src/adapter.rs#L3298) |
| `0x22E7` | Execute advance branch | Handled | Guard: x64 case RVA `0x844d1`, present in the supplied script corpus | [adapter.rs:3298](../../crates/crysirror-cmvs/src/adapter.rs#L3298) |
| `0x22E8` | Execute advance branch | Handled | Guard: x64 case RVA `0x844de`, present in the supplied script corpus | [adapter.rs:3298](../../crates/crysirror-cmvs/src/adapter.rs#L3298) |
| `0x22E9` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x844eb`, present in the supplied script corpus | [cmvs-20160201.json:52](../../tools/native-api/cmvs-20160201.json#L52) |
| `0x22EA` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x844f8` | [cmvs-20160201.json:52](../../tools/native-api/cmvs-20160201.json#L52) |
| `0x22EB` | Pc | Handled | Guard: x64 case RVA `0x84505`, present in the supplied script corpus | [startup.rs:1815](../../crates/crysirror-cmvs/src/vm/startup.rs#L1815) |
| `0x22EC` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x84512` | [cmvs-20160201.json:52](../../tools/native-api/cmvs-20160201.json#L52) |
| `0x22ED` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x22EE` | Pc | Handled | Guard: x64 case RVA `0x8451f`, present in the supplied script corpus | [startup.rs:1841](../../crates/crysirror-cmvs/src/vm/startup.rs#L1841) |
| `0x22EF` | Input (command) | Handled | Guard: x64 case RVA `0x8453f`, present in the supplied script corpus | [input.rs:223](../../crates/crysirror-cmvs/src/vm/input.rs#L223) |
| `0x22F0` | Read or consume an input edge | Handled | Guard: x64 case RVA `0x8454c` | [input.rs:29](../../crates/crysirror-cmvs/src/vm/input.rs#L29) |
| `0x22F1` | Execute advance branch | Handled | Guard: x64 case RVA `0x84559` | [adapter.rs:3304](../../crates/crysirror-cmvs/src/adapter.rs#L3304) |
| `0x22F2` | Read or consume an input edge | Handled | Guard: x64 case RVA `0x84570` | [input.rs:30](../../crates/crysirror-cmvs/src/vm/input.rs#L30) |
| `0x22F3` | Execute advance branch | Handled | Guard: x64 case RVA `0x8457d` | [adapter.rs:3304](../../crates/crysirror-cmvs/src/adapter.rs#L3304) |
| `0x22F4` | Read or consume an input edge | Handled | Guard: x64 case RVA `0x84594` | [input.rs:31](../../crates/crysirror-cmvs/src/vm/input.rs#L31) |
| `0x22F5` | Execute advance branch | Handled | Guard: x64 case RVA `0x845a1` | [adapter.rs:3304](../../crates/crysirror-cmvs/src/adapter.rs#L3304) |
| `0x22F6` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x22F7` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x22F8` | Execute advance branch | Handled | Guard: x64 case RVA `0x845b8`, present in the supplied script corpus | [adapter.rs:3298](../../crates/crysirror-cmvs/src/adapter.rs#L3298) |
| `0x22F9` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x22FA` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x22FB` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x22FC` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x845c5`, present in the supplied script corpus | [cmvs-20160201.json:53](../../tools/native-api/cmvs-20160201.json#L53) |
| `0x22FD` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x845d2`, present in the supplied script corpus | [cmvs-20160201.json:53](../../tools/native-api/cmvs-20160201.json#L53) |
| `0x22FE` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x22FF` | Read the device's other-display setting | Handled | Guard: x64 case RVA `0x845df` | [display.rs:379](../../crates/crysirror-cmvs/src/adapter/display.rs#L379) |
| `0x2300` | Create a native offscreen render target | Open | 2016 requires native group-8/9 offscreen targets; 2021 uses its object error branch; Guard: x64 case RVA `0x845ec` | [adapter.rs:3531](../../crates/crysirror-cmvs/src/adapter.rs#L3531) |
| `0x2301` | Release a native offscreen render target | Open | 2016 requires native group-8/9 offscreen targets; 2021 uses its object error branch; Guard: x64 case RVA `0x845f9` | [adapter.rs:3531](../../crates/crysirror-cmvs/src/adapter.rs#L3531) |
| `0x2302` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2303` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2304` | Execute advance branch | Handled | Guard: x64 case RVA `0x84606`, present in the supplied script corpus | [adapter.rs:3487](../../crates/crysirror-cmvs/src/adapter.rs#L3487) |
| `0x2305` | Execute advance branch | Handled | Guard: x64 case RVA `0x84619`, present in the supplied script corpus | [adapter.rs:3492](../../crates/crysirror-cmvs/src/adapter.rs#L3492) |
| `0x2306` | Execute advance branch | Handled | Guard: x64 case RVA `0x8462c`, present in the supplied script corpus | [adapter.rs:3497](../../crates/crysirror-cmvs/src/adapter.rs#L3497) |
| `0x2307` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2308` | Update job | Handled | Guard: x64 case RVA `0x84639`, present in the supplied script corpus | [adapter.rs:3830](../../crates/crysirror-cmvs/src/adapter.rs#L3830) |
| `0x2309` | Execute advance branch | Handled | Guard: x64 case RVA `0x84646`, present in the supplied script corpus | [adapter.rs:3871](../../crates/crysirror-cmvs/src/adapter.rs#L3871) |
| `0x230A` | Execute advance branch | Open | Releases owned slot; scene movie creation is pending; Guard: x64 case RVA `0x84653`, present in the supplied script corpus | [adapter.rs:3102](../../crates/crysirror-cmvs/src/adapter.rs#L3102) |
| `0x230B` | Execute advance branch | Handled | Guard: x64 case RVA `0x84660`, present in the supplied script corpus | [adapter.rs:3871](../../crates/crysirror-cmvs/src/adapter.rs#L3871) |
| `0x230C` | Execute advance branch | Handled | Guard: x64 case RVA `0x8466d`, present in the supplied script corpus | [adapter.rs:3871](../../crates/crysirror-cmvs/src/adapter.rs#L3871) |
| `0x230D` | Execute advance branch | Handled | Guard: x64 case RVA `0x8467a` | [adapter.rs:3871](../../crates/crysirror-cmvs/src/adapter.rs#L3871) |
| `0x230E` | Update job | Handled | Guard: x64 case RVA `0x84687` | [adapter.rs:3830](../../crates/crysirror-cmvs/src/adapter.rs#L3830) |
| `0x230F` | Execute advance branch | Handled | Guard: x64 case RVA `0x84694` | [adapter.rs:3871](../../crates/crysirror-cmvs/src/adapter.rs#L3871) |
| `0x2310` | Update job | Handled | Guard: x64 case RVA `0x846a1` | [adapter.rs:4344](../../crates/crysirror-cmvs/src/adapter.rs#L4344) |
| `0x2311` | Update job | Handled | Guard: x64 case RVA `0x846ae` | [adapter.rs:4344](../../crates/crysirror-cmvs/src/adapter.rs#L4344) |
| `0x2312` | Update job | Handled | Guard: x64 case RVA `0x846bb` | [adapter.rs:4344](../../crates/crysirror-cmvs/src/adapter.rs#L4344) |
| `0x2313` | Update job | Handled | Guard: x64 case RVA `0x846c8` | [adapter.rs:4344](../../crates/crysirror-cmvs/src/adapter.rs#L4344) |
| `0x2314` | Update job | Handled | Guard: x64 case RVA `0x846d5` | [adapter.rs:4344](../../crates/crysirror-cmvs/src/adapter.rs#L4344) |
| `0x2315` | Update job | Handled | Guard: x64 case RVA `0x846e2` | [adapter.rs:4344](../../crates/crysirror-cmvs/src/adapter.rs#L4344) |
| `0x2316` | Update job | Handled | Guard: x64 case RVA `0x846ef` | [adapter.rs:4344](../../crates/crysirror-cmvs/src/adapter.rs#L4344) |
| `0x2317` | Update job | Handled | Guard: x64 case RVA `0x846fc` | [adapter.rs:4344](../../crates/crysirror-cmvs/src/adapter.rs#L4344) |
| `0x2318` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2319` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x231A` | Execute advance branch | Handled | Guard: x64 case RVA `0x84709`, present in the supplied script corpus | [adapter.rs:3871](../../crates/crysirror-cmvs/src/adapter.rs#L3871) |
| `0x231B` | Execute advance branch | Handled | Guard: x64 case RVA `0x84716` | [adapter.rs:3871](../../crates/crysirror-cmvs/src/adapter.rs#L3871) |
| `0x231C` | Execute advance branch | Handled | Guard: x64 case RVA `0x84723` | [adapter.rs:3871](../../crates/crysirror-cmvs/src/adapter.rs#L3871) |
| `0x231D` | Execute advance branch | Handled | Guard: x64 case RVA `0x84730` | [adapter.rs:3871](../../crates/crysirror-cmvs/src/adapter.rs#L3871) |
| `0x231E` | Execute advance branch | Handled | Guard: x64 case RVA `0x8473d` | [adapter.rs:3871](../../crates/crysirror-cmvs/src/adapter.rs#L3871) |
| `0x231F` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2320` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x8474a` | [cmvs-20160201.json:56](../../tools/native-api/cmvs-20160201.json#L56) |
| `0x2321` | Set a draw record's second-texture blend mode | Handled | Guard: x64 case RVA `0x84757`, present in the supplied script corpus | [adapter.rs:4513](../../crates/crysirror-cmvs/src/adapter.rs#L4513) |
| `0x2322` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x84764`, present in the supplied script corpus | [cmvs-20160201.json:56](../../tools/native-api/cmvs-20160201.json#L56) |
| `0x2323` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2324` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2325` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2326` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2327` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2328` | Execute advance branch | Handled | Guard: x64 case RVA `0x84771`, present in the supplied script corpus | [adapter.rs:3599](../../crates/crysirror-cmvs/src/adapter.rs#L3599) |
| `0x2329` | Execute advance branch | Handled | Guard: x64 case RVA `0x8477e`, present in the supplied script corpus | [adapter.rs:3599](../../crates/crysirror-cmvs/src/adapter.rs#L3599) |
| `0x232A` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x232B` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x232C` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x232D` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x232E` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x232F` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2330` | Execute advance branch | Handled | Guard: x64 case RVA `0x8478b` | [adapter.rs:3599](../../crates/crysirror-cmvs/src/adapter.rs#L3599) |
| `0x2331` | Execute advance branch | Handled | Guard: x64 case RVA `0x84798` | [adapter.rs:3599](../../crates/crysirror-cmvs/src/adapter.rs#L3599) |
| `0x2332` | Execute advance branch | Handled | Guard: x64 case RVA `0x847a5` | [adapter.rs:3599](../../crates/crysirror-cmvs/src/adapter.rs#L3599) |
| `0x2333` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2334` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2335` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2336` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2337` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2338` | Execute advance branch | Handled | Guard: x64 case RVA `0x847b2` | [adapter.rs:4177](../../crates/crysirror-cmvs/src/adapter.rs#L4177) |
| `0x2339` | Execute advance branch | Handled | Guard: x64 case RVA `0x847bf` | [adapter.rs:4201](../../crates/crysirror-cmvs/src/adapter.rs#L4201) |
| `0x233A` | Execute advance branch | Handled | Guard: x64 case RVA `0x847e3` | [adapter.rs:4201](../../crates/crysirror-cmvs/src/adapter.rs#L4201) |
| `0x233B` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x233C` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x233D` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x233E` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x233F` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2340` | Required parameter | Handled | Guard: x64 case RVA `0x847fa`, present in the supplied script corpus | [startup.rs:1805](../../crates/crysirror-cmvs/src/vm/startup.rs#L1805) |
| `0x2341` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2342` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2343` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2344` | Read or consume an input edge | Handled | Guard: x64 case RVA `0x84826`, present in the supplied script corpus | [input.rs:26](../../crates/crysirror-cmvs/src/vm/input.rs#L26) |
| `0x2345` | Execute advance branch | Handled | Guard: x64 case RVA `0x84833` | [adapter.rs:3304](../../crates/crysirror-cmvs/src/adapter.rs#L3304) |
| `0x2346` | Read or consume an input edge | Handled | Guard: x64 case RVA `0x8484a`, present in the supplied script corpus | [input.rs:27](../../crates/crysirror-cmvs/src/vm/input.rs#L27) |
| `0x2347` | Execute advance branch | Handled | Guard: x64 case RVA `0x84857` | [adapter.rs:3304](../../crates/crysirror-cmvs/src/adapter.rs#L3304) |
| `0x2348` | Read or consume an input edge | Handled | Guard: x64 case RVA `0x8486e` | [input.rs:32](../../crates/crysirror-cmvs/src/vm/input.rs#L32) |
| `0x2349` | Execute advance branch | Handled | Guard: x64 case RVA `0x8487b` | [adapter.rs:3304](../../crates/crysirror-cmvs/src/adapter.rs#L3304) |
| `0x234A` | Read or consume an input edge | Handled | Guard: x64 case RVA `0x84892`, present in the supplied script corpus | [input.rs:28](../../crates/crysirror-cmvs/src/vm/input.rs#L28) |
| `0x234B` | Execute advance branch | Handled | Guard: x64 case RVA `0x8489f` | [adapter.rs:3304](../../crates/crysirror-cmvs/src/adapter.rs#L3304) |
| `0x234C` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x234D` | Input (command) | Handled | Guard: x64 case RVA `0x848b6`, present in the supplied script corpus | [input.rs:224](../../crates/crysirror-cmvs/src/vm/input.rs#L224) |
| `0x234E` | Input (command) | Handled | Guard: x64 case RVA `0x848cd`, present in the supplied script corpus | [input.rs:225](../../crates/crysirror-cmvs/src/vm/input.rs#L225) |
| `0x234F` | Input (command) | Handled | Guard: x64 case RVA `0x848da`, present in the supplied script corpus | [input.rs:226](../../crates/crysirror-cmvs/src/vm/input.rs#L226) |
| `0x2350` | Required parameter | Handled | Guard: x64 case RVA `0x848f1`, present in the supplied script corpus | [startup.rs:1699](../../crates/crysirror-cmvs/src/vm/startup.rs#L1699) |
| `0x2351` | Required parameter | Handled | Guard: x64 case RVA `0x848fe`, present in the supplied script corpus | [startup.rs:1699](../../crates/crysirror-cmvs/src/vm/startup.rs#L1699) |
| `0x2352` | Required parameter | Handled | Guard: x64 case RVA `0x8490b`, present in the supplied script corpus | [startup.rs:1699](../../crates/crysirror-cmvs/src/vm/startup.rs#L1699) |
| `0x2353` | Required parameter | Handled | Guard: x64 case RVA `0x84918`, present in the supplied script corpus | [startup.rs:1699](../../crates/crysirror-cmvs/src/vm/startup.rs#L1699) |
| `0x2354` | Required parameter | Handled | Guard: x64 case RVA `0x84925` | [startup.rs:1699](../../crates/crysirror-cmvs/src/vm/startup.rs#L1699) |
| `0x2355` | Required parameter | Handled | Guard: x64 case RVA `0x8493c` | [startup.rs:1699](../../crates/crysirror-cmvs/src/vm/startup.rs#L1699) |
| `0x2356` | Required parameter | Handled | Guard: x64 case RVA `0x84949`, present in the supplied script corpus | [startup.rs:1699](../../crates/crysirror-cmvs/src/vm/startup.rs#L1699) |
| `0x2357` | Required parameter | Handled | Guard: x64 case RVA `0x84956`, present in the supplied script corpus | [startup.rs:1699](../../crates/crysirror-cmvs/src/vm/startup.rs#L1699) |
| `0x2358` | Required parameter | Handled | Guard: x64 case RVA `0x84963`, present in the supplied script corpus | [startup.rs:1699](../../crates/crysirror-cmvs/src/vm/startup.rs#L1699) |
| `0x2359` | Required parameter | Handled | Guard: x64 case RVA `0x84970`, present in the supplied script corpus | [startup.rs:1699](../../crates/crysirror-cmvs/src/vm/startup.rs#L1699) |
| `0x235A` | Required parameter | Handled | Guard: x64 case RVA `0x8497d`, present in the supplied script corpus | [startup.rs:1699](../../crates/crysirror-cmvs/src/vm/startup.rs#L1699) |
| `0x235B` | Required parameter | Handled | Guard: x64 case RVA `0x8498a`, present in the supplied script corpus | [startup.rs:1699](../../crates/crysirror-cmvs/src/vm/startup.rs#L1699) |
| `0x235C` | Input (command) | Handled | Guard: x64 case RVA `0x84997`, present in the supplied script corpus | [input.rs:227](../../crates/crysirror-cmvs/src/vm/input.rs#L227) |
| `0x235D` | Input (command) | Handled | Guard: x64 case RVA `0x849a4` | [input.rs:227](../../crates/crysirror-cmvs/src/vm/input.rs#L227) |
| `0x235E` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x235F` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2360` | Required parameter | Handled | Guard: x64 case RVA `0x849b1`, present in the supplied script corpus | [startup.rs:1699](../../crates/crysirror-cmvs/src/vm/startup.rs#L1699) |
| `0x2361` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x82484` | [cmvs-20160201.json:60](../../tools/native-api/cmvs-20160201.json#L60) |
| `0x2362` | Required parameter | Handled | Guard: x64 case RVA `0x849be`, present in the supplied script corpus | [startup.rs:1699](../../crates/crysirror-cmvs/src/vm/startup.rs#L1699) |
| `0x2363` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2364` | Select the pointer gesture mode and reset tracking | Handled | Guard: x64 case RVA `0x849cb`, present in the supplied script corpus | [startup.rs:1699](../../crates/crysirror-cmvs/src/vm/startup.rs#L1699) |
| `0x2365` | Required parameter | Handled | Guard: x64 case RVA `0x849d8`, present in the supplied script corpus | [startup.rs:1699](../../crates/crysirror-cmvs/src/vm/startup.rs#L1699) |
| `0x2366` | Required parameter | Handled | Guard: x64 case RVA `0x849e5`, present in the supplied script corpus | [startup.rs:1699](../../crates/crysirror-cmvs/src/vm/startup.rs#L1699) |
| `0x2367` | Required parameter | Handled | Guard: x64 case RVA `0x849f2`, present in the supplied script corpus | [startup.rs:1699](../../crates/crysirror-cmvs/src/vm/startup.rs#L1699) |
| `0x2368` | Required parameter | Handled | Guard: x64 case RVA `0x84a09`, present in the supplied script corpus | [startup.rs:1699](../../crates/crysirror-cmvs/src/vm/startup.rs#L1699) |
| `0x2369` | Required parameter | Handled | Guard: x64 case RVA `0x84a16`, present in the supplied script corpus | [startup.rs:1699](../../crates/crysirror-cmvs/src/vm/startup.rs#L1699) |
| `0x236A` | Read unrestricted-drag release status, angle and speed | Handled | Guard: x64 case RVA `0x84a23` | [startup.rs:1699](../../crates/crysirror-cmvs/src/vm/startup.rs#L1699) |
| `0x236B` | Set the unrestricted-drag movement threshold | Handled | Guard: x64 case RVA `0x84a30` | [startup.rs:1699](../../crates/crysirror-cmvs/src/vm/startup.rs#L1699) |
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
| `0x237C` | Execute advance branch | Handled | Guard: x64 case RVA `0x84ad9`, present in the supplied script corpus | [adapter.rs:3109](../../crates/crysirror-cmvs/src/adapter.rs#L3109) |
| `0x237D` | Execute advance branch | Handled | Guard: x64 case RVA `0x84ae6`, present in the supplied script corpus | [adapter.rs:3109](../../crates/crysirror-cmvs/src/adapter.rs#L3109) |
| `0x237E` | Execute advance branch | Handled | Guard: x64 case RVA `0x84af3`, present in the supplied script corpus | [adapter.rs:3109](../../crates/crysirror-cmvs/src/adapter.rs#L3109) |
| `0x237F` | Execute advance branch | Handled | Guard: x64 case RVA `0x84b00` | [adapter.rs:3109](../../crates/crysirror-cmvs/src/adapter.rs#L3109) |
| `0x2380` | Execute advance branch | Handled | Guard: x64 case RVA `0x84b0d`, present in the supplied script corpus | [adapter.rs:2328](../../crates/crysirror-cmvs/src/adapter.rs#L2328) |
| `0x2381` | Execute advance branch | Handled | Guard: x64 case RVA `0x84b1a` | [adapter.rs:2329](../../crates/crysirror-cmvs/src/adapter.rs#L2329) |
| `0x2382` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x84b27`, present in the supplied script corpus | [cmvs-20160201.json:62](../../tools/native-api/cmvs-20160201.json#L62) |
| `0x2383` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x84b34`, present in the supplied script corpus | [cmvs-20160201.json:62](../../tools/native-api/cmvs-20160201.json#L62) |
| `0x2384` | Execute advance branch | Handled | Guard: x64 case RVA `0x84b41` | [adapter.rs:3474](../../crates/crysirror-cmvs/src/adapter.rs#L3474) |
| `0x2385` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x84b4e`, present in the supplied script corpus | [cmvs-20160201.json:62](../../tools/native-api/cmvs-20160201.json#L62) |
| `0x2386` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x84b5b`, present in the supplied script corpus | [cmvs-20160201.json:62](../../tools/native-api/cmvs-20160201.json#L62) |
| `0x2387` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x84b68`, present in the supplied script corpus | [cmvs-20160201.json:62](../../tools/native-api/cmvs-20160201.json#L62) |
| `0x2388` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x84b75` | [cmvs-20160201.json:62](../../tools/native-api/cmvs-20160201.json#L62) |
| `0x2389` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x84b82`, present in the supplied script corpus | [cmvs-20160201.json:62](../../tools/native-api/cmvs-20160201.json#L62) |
| `0x238A` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x84b8f` | [cmvs-20160201.json:62](../../tools/native-api/cmvs-20160201.json#L62) |
| `0x238B` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x84b9c`, present in the supplied script corpus | [cmvs-20160201.json:62](../../tools/native-api/cmvs-20160201.json#L62) |
| `0x238C` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x84ba9`, present in the supplied script corpus | [cmvs-20160201.json:62](../../tools/native-api/cmvs-20160201.json#L62) |
| `0x238D` | Execute advance branch | Handled | Guard: x64 case RVA `0x84bb6` | [adapter.rs:3474](../../crates/crysirror-cmvs/src/adapter.rs#L3474) |
| `0x238E` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x84bc3`, present in the supplied script corpus | [cmvs-20160201.json:62](../../tools/native-api/cmvs-20160201.json#L62) |
| `0x238F` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x84bd0`, present in the supplied script corpus | [cmvs-20160201.json:62](../../tools/native-api/cmvs-20160201.json#L62) |
| `0x2390` | Execute advance branch | Handled | Guard: x64 case RVA `0x84bdd`, present in the supplied script corpus | [adapter.rs:3461](../../crates/crysirror-cmvs/src/adapter.rs#L3461) |
| `0x2391` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x84bea` | [cmvs-20160201.json:63](../../tools/native-api/cmvs-20160201.json#L63) |
| `0x2392` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x84bf7`, present in the supplied script corpus | [cmvs-20160201.json:63](../../tools/native-api/cmvs-20160201.json#L63) |
| `0x2393` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x84c04`, present in the supplied script corpus | [cmvs-20160201.json:63](../../tools/native-api/cmvs-20160201.json#L63) |
| `0x2394` | Original system dispatcher entry | Unreviewed | No connected CrySirror dispatch route; Guard: x64 case RVA `0x84c11` | [cmvs-20160201.json:63](../../tools/native-api/cmvs-20160201.json#L63) |
| `0x2395` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x2396` | Execute advance branch | Handled | Guard: x64 case RVA `0x84c1e`, present in the supplied script corpus | [adapter.rs:3461](../../crates/crysirror-cmvs/src/adapter.rs#L3461) |
| `0x2397` | Update job | Handled | Guard: x64 case RVA `0x84c2b`, present in the supplied script corpus | [adapter.rs:4271](../../crates/crysirror-cmvs/src/adapter.rs#L4271) |
| `0x2398` | Execute advance branch | Handled | Guard: x64 case RVA `0x84c38`, present in the supplied script corpus | [adapter.rs:3474](../../crates/crysirror-cmvs/src/adapter.rs#L3474) |
| `0x2399` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x239A` | Execute advance branch | Handled | Guard: x64 case RVA `0x84c45`, present in the supplied script corpus | [adapter.rs:3461](../../crates/crysirror-cmvs/src/adapter.rs#L3461) |
| `0x239B` | Execute advance branch | Handled | Guard: x64 case RVA `0x84c52`, present in the supplied script corpus | [adapter.rs:3461](../../crates/crysirror-cmvs/src/adapter.rs#L3461) |
| `0x239C` | Execute advance branch | Handled | Guard: x64 case RVA `0x84c5f` | [adapter.rs:3461](../../crates/crysirror-cmvs/src/adapter.rs#L3461) |
| `0x239D` | Execute advance branch | Handled | Guard: x64 case RVA `0x84c6c`, present in the supplied script corpus | [adapter.rs:3461](../../crates/crysirror-cmvs/src/adapter.rs#L3461) |
| `0x239E` | Execute advance branch | Handled | Guard: x64 case RVA `0x84c79`, present in the supplied script corpus | [adapter.rs:3461](../../crates/crysirror-cmvs/src/adapter.rs#L3461) |
| `0x239F` | Execute advance branch | Handled | Guard: x64 case RVA `0x84c86`, present in the supplied script corpus | [adapter.rs:3461](../../crates/crysirror-cmvs/src/adapter.rs#L3461) |
| `0x23A0` | Execute advance branch | Handled | Guard: x64 case RVA `0x84c93`, present in the supplied script corpus | [adapter.rs:3461](../../crates/crysirror-cmvs/src/adapter.rs#L3461) |
| `0x23A1` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x23A2` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x23A3` | Advance with no operands or effects | Original no-op | Original handler returns 0x4000; Guard: x64 case RVA `0x823e7` | [noop.rs:26](../../crates/crysirror-cmvs/src/vm/noop.rs#L26) |
| `0x23A4` | Waiting | Handled | Guard: x64 case RVA `0x84ca0` | [adapter.rs:4435](../../crates/crysirror-cmvs/src/adapter.rs#L4435) |
| `0x23A5` | Waiting | Handled | Guard: x64 case RVA `0x84cad` | [adapter.rs:4435](../../crates/crysirror-cmvs/src/adapter.rs#L4435) |
| `0x23A6` | Waiting | Handled | Guard: x64 case RVA `0x84cba` | [adapter.rs:4435](../../crates/crysirror-cmvs/src/adapter.rs#L4435) |
| `0x23A7` | Waiting | Handled | Guard: x64 case RVA `0x84cc7` | [adapter.rs:4435](../../crates/crysirror-cmvs/src/adapter.rs#L4435) |
| `0x23A8` | Waiting | Handled | Guard: x64 case RVA `0x84cd4` | [adapter.rs:4435](../../crates/crysirror-cmvs/src/adapter.rs#L4435) |
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
| `0x23B5` | Control flow (command) | Handled | Guard: x64 case RVA `0x825b1` | [control.rs:346](../../crates/crysirror-cmvs/src/vm/control.rs#L346) |
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
| `0x28B8` | Update job | Handled | — | [adapter.rs:2830](../../crates/crysirror-cmvs/src/adapter.rs#L2830) |
| `0x28C5` | Update job | Handled | — | [adapter.rs:2728](../../crates/crysirror-cmvs/src/adapter.rs#L2728) |
| `0x29AB` | Update job | Handled | — | [adapter.rs:2598](../../crates/crysirror-cmvs/src/adapter.rs#L2598) |
| `0x29AC` | Update job | Handled | — | [adapter.rs:2496](../../crates/crysirror-cmvs/src/adapter.rs#L2496) |
| `0x29B1` | Execute advance branch | Handled | — | [adapter.rs:4466](../../crates/crysirror-cmvs/src/adapter.rs#L4466) |
| `0x29B2` | Execute advance branch | Handled | — | [adapter.rs:4466](../../crates/crysirror-cmvs/src/adapter.rs#L4466) |
| `0x29B6` | Execute advance branch | Handled | — | [adapter.rs:4466](../../crates/crysirror-cmvs/src/adapter.rs#L4466) |
| `0x29B7` | Execute advance branch | Handled | — | [adapter.rs:4466](../../crates/crysirror-cmvs/src/adapter.rs#L4466) |
| `0x29ED` | Update job | Handled | — | [adapter.rs:3178](../../crates/crysirror-cmvs/src/adapter.rs#L3178) |

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
| `visual::prepare_image(sources, name, context)` | `Ok(image)` | Local state/value operations | Handled | [1007](../../crates/crysirror-cmvs/src/adapter.rs#L1007) |
| `visual::prepare_image(sources, name, context)` | `Err(error) if error.to_string().starts_with("CMVS image missing")` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [1008](../../crates/crysirror-cmvs/src/adapter.rs#L1008) |
| `visual::prepare_image(sources, name, context)` | `Err(error)` | Local state/value operations | Unsupported: Unconditional rejection | [1009](../../crates/crysirror-cmvs/src/adapter.rs#L1009) |

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
| `condition; under crate::vm::cvns_selector(step.1)=Some(0x21ed)` | `(1..=31).contains(&key) && key != 6 && (changed \|\| key == 15)` | Local state/value operations | Handled | [1865](../../crates/crysirror-cmvs/src/adapter.rs#L1865) |
| `condition; under crate::vm::cvns_selector(step.1)=Some(0x21ed)` | `key == 2` | host.now_ms | Handled | [1868](../../crates/crysirror-cmvs/src/adapter.rs#L1868) |
| `condition; under crate::vm::cvns_selector(step.1)=Some(0x21ed)` | `key == 30` | Local state/value operations | Handled | [1871](../../crates/crysirror-cmvs/src/adapter.rs#L1871) |
| `condition; under crate::vm::cvns_selector(step.1)=Some(0x21ed)` | `key == 29` | Local state/value operations | Handled | [1872](../../crates/crysirror-cmvs/src/adapter.rs#L1872) |
| `condition; under crate::vm::cvns_selector(step.1)=Some(0x21ed)` | `key == 5` | host.window_focused | Handled | [1875](../../crates/crysirror-cmvs/src/adapter.rs#L1875) |
| `condition; under crate::vm::cvns_selector(step.1)=Some(0x21ed)` | `(10..=14).contains(&key)` | Local state/value operations | Handled | [1880](../../crates/crysirror-cmvs/src/adapter.rs#L1880) |
| `condition; under crate::vm::cvns_selector(step.1)=Some(0x21ed)` | `!matches!(key, 6..=9) \|\| changed` | Local state/value operations | Handled | [1883](../../crates/crysirror-cmvs/src/adapter.rs#L1883) |
| `condition; under crate::vm::cvns_selector(step.1)=Some(0x21ed)` | `matches!(key, 7..=9) && changed` | Local state/value operations | Handled | [1893](../../crates/crysirror-cmvs/src/adapter.rs#L1893) |
| `condition; under crate::vm::cvns_selector(step.1)=Some(0x217c)` | `mode as i32 > 3` | Local state/value operations | Handled | [1910](../../crates/crysirror-cmvs/src/adapter.rs#L1910) |
| `condition; under crate::vm::cvns_selector(step.1)=Some(selector @ (0x21c0 \| 0x21c1))` | `selector == 0x21c0` | crate::vm::css0::SystemData::decode, Completion::CvnsSystemData | Handled | [2064](../../crates/crysirror-cmvs/src/adapter.rs#L2064) |
| `condition; under crate::vm::cvns_selector(step.1)=Some(selector @ (0x20af \| 0x20b7 \| 0x20bd \| 0x20be))` | `matches!(selector, 0x20bd \| 0x20be)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [2124](../../crates/crysirror-cmvs/src/adapter.rs#L2124) |
| `condition; under crate::vm::cvns_selector(step.1)=Some(selector @ 0x20b9..=0x20bb)` | `selector == 0x20ba` | Local state/value operations | Handled | [2138](../../crates/crysirror-cmvs/src/adapter.rs#L2138) |
| `selector; under crate::vm::cvns_selector(step.1)=Some(selector @ 0x20b9..=0x20bb)` | `0x20b9` | Local state/value operations | Handled | [2143](../../crates/crysirror-cmvs/src/adapter.rs#L2143) |
| `selector; under crate::vm::cvns_selector(step.1)=Some(selector @ 0x20b9..=0x20bb)` | `0x20ba` | Local state/value operations | Handled | [2147](../../crates/crysirror-cmvs/src/adapter.rs#L2147) |
| `selector; under crate::vm::cvns_selector(step.1)=Some(selector @ 0x20b9..=0x20bb)` | `_` | Local state/value operations | Handled | [2148](../../crates/crysirror-cmvs/src/adapter.rs#L2148) |
| `condition; under crate::vm::cvns_selector(step.1)=Some(selector @ (0x20b6 \| 0x20bc \| 0x20c6))` | `selector == 0x20b6` | Local state/value operations | Handled | [2160](../../crates/crysirror-cmvs/src/adapter.rs#L2160) |
| `condition; under crate::vm::cvns_selector(step.1)=Some(selector @ (0x20b6 \| 0x20bc \| 0x20c6))` | `selector == 0x20c6` | Local state/value operations | Handled | [2168](../../crates/crysirror-cmvs/src/adapter.rs#L2168) |
| `crate::vm::cvns_selector(step.1)` | `Some(0x21ed)` | host.now_ms, host.window_focused | Handled | [1859](../../crates/crysirror-cmvs/src/adapter.rs#L1859) |
| `crate::vm::cvns_selector(step.1)` | `Some(0x2007)` | Local state/value operations | Handled | [1900](../../crates/crysirror-cmvs/src/adapter.rs#L1900) |
| `crate::vm::cvns_selector(step.1)` | `Some(0x2002)` | Local state/value operations | Handled | [1904](../../crates/crysirror-cmvs/src/adapter.rs#L1904) |
| `crate::vm::cvns_selector(step.1)` | `Some(0x217c)` | Local state/value operations | Handled | [1908](../../crates/crysirror-cmvs/src/adapter.rs#L1908) |
| `crate::vm::cvns_selector(step.1)` | `Some(0x2182)` | host.window_maximized, host.window_minimized | Handled | [1925](../../crates/crysirror-cmvs/src/adapter.rs#L1925) |
| `crate::vm::cvns_selector(step.1)` | `Some(0x2180)` | Local state/value operations | Handled | [1931](../../crates/crysirror-cmvs/src/adapter.rs#L1931) |
| `crate::vm::cvns_selector(step.1)` | `Some(0x2181)` | Local state/value operations | Handled | [1936](../../crates/crysirror-cmvs/src/adapter.rs#L1936) |
| `crate::vm::cvns_selector(step.1)` | `Some(0x2003)` | Local state/value operations | Handled | [1943](../../crates/crysirror-cmvs/src/adapter.rs#L1943) |
| `crate::vm::cvns_selector(step.1)` | `Some(0x21a3)` | cvns::slot::Write::capture, sound::Effects::new, sound::Music::new, crate::vm::open_save_slot, ScriptSources::prepare | Handled | [1947](../../crates/crysirror-cmvs/src/adapter.rs#L1947) |
| `crate::vm::cvns_selector(step.1)` | `Some(0x21a4)` | cvns::slot::Write::capture, crate::vm::scan_cvns_menu, Completion::CvnsWritten | Handled | [2031](../../crates/crysirror-cmvs/src/adapter.rs#L2031) |
| `crate::vm::cvns_selector(step.1)` | `Some(selector @ (0x21c0 \| 0x21c1))` | VfsPath::new, crate::vm::css0::SystemData::decode, Completion::CvnsSystemData | Handled | [2060](../../crates/crysirror-cmvs/src/adapter.rs#L2060) |
| `crate::vm::cvns_selector(step.1)` | `Some(0x20b8 \| 0x21b1 \| 0x21b2 \| 0x21b6 \| 0x21b7 \| 0x21ab \| 0x21ac)` | Local state/value operations | Handled | [2089](../../crates/crysirror-cmvs/src/adapter.rs#L2089) |
| `crate::vm::cvns_selector(step.1)` | `Some(selector @ (0x21a7 \| 0x21ae))` | crate::vm::latest_cvns_slot, Completion::LatestSaveSlot, crate::vm::probe_cvns_slot, Completion::ProbeSaveSlot | Handled | [2090](../../crates/crysirror-cmvs/src/adapter.rs#L2090) |
| `crate::vm::cvns_selector(step.1)` | `Some(0x21a1)` | Completion::CvnsSaveMenu, crate::vm::scan_cvns_menu | Handled | [2110](../../crates/crysirror-cmvs/src/adapter.rs#L2110) |
| `crate::vm::cvns_selector(step.1)` | `Some(selector @ (0x20af \| 0x20b7 \| 0x20bd \| 0x20be))` | host.update_audio_pcm16_gain, sound::cvns_fader_update | Handled | [2122](../../crates/crysirror-cmvs/src/adapter.rs#L2122) |
| `crate::vm::cvns_selector(step.1)` | `Some(selector @ 0x20b9..=0x20bb)` | Local state/value operations | Handled | [2136](../../crates/crysirror-cmvs/src/adapter.rs#L2136) |
| `crate::vm::cvns_selector(step.1)` | `Some(selector @ (0x20b6 \| 0x20bc \| 0x20c6))` | sound::position | Handled | [2157](../../crates/crysirror-cmvs/src/adapter.rs#L2157) |
| `crate::vm::cvns_selector(step.1)` | `Some(0x20a4)` | host.now_ms | Handled | [2180](../../crates/crysirror-cmvs/src/adapter.rs#L2180) |
| `crate::vm::cvns_selector(step.1)` | `Some(0x20c0)` | Local state/value operations | Handled | [2188](../../crates/crysirror-cmvs/src/adapter.rs#L2188) |
| `crate::vm::cvns_selector(step.1)` | `Some(0x20c5)` | Local state/value operations | Handled | [2189](../../crates/crysirror-cmvs/src/adapter.rs#L2189) |
| `crate::vm::cvns_selector(step.1)` | `Some(0x20c7)` | Local state/value operations | Handled | [2198](../../crates/crysirror-cmvs/src/adapter.rs#L2198) |
| `crate::vm::cvns_selector(step.1)` | `Some(0x20eb)` | Local state/value operations | Handled | [2208](../../crates/crysirror-cmvs/src/adapter.rs#L2208) |
| `crate::vm::cvns_selector(step.1)` | `Some(0x2041)` | Local state/value operations | Handled | [2215](../../crates/crysirror-cmvs/src/adapter.rs#L2215) |
| `crate::vm::cvns_selector(step.1)` | `Some(0x217f)` | host.display_info | Handled | [2221](../../crates/crysirror-cmvs/src/adapter.rs#L2221) |
| `crate::vm::cvns_selector(step.1)` | `Some(0x217d)` | Local state/value operations | Handled | [2226](../../crates/crysirror-cmvs/src/adapter.rs#L2226) |
| `crate::vm::cvns_selector(step.1)` | `Some(0x2005)` | Local state/value operations | Handled | [2230](../../crates/crysirror-cmvs/src/adapter.rs#L2230) |
| `crate::vm::cvns_selector(step.1)` | `Some(0x2010)` | Local state/value operations | Handled | [2231](../../crates/crysirror-cmvs/src/adapter.rs#L2231) |
| `crate::vm::cvns_selector(step.1)` | `Some(0x2012)` | Local state/value operations | Handled | [2232](../../crates/crysirror-cmvs/src/adapter.rs#L2232) |
| `crate::vm::cvns_selector(step.1)` | `Some(selector)` | DecodeContext::new, ScriptSources::prepare, visual::prepare_image, cvns::rule::Active::new, cvns::BitmapSlot::Global | Unsupported: unsupported CVNS syscall 0x{selector:04x} at {}+0x{:x} | [2233](../../crates/crysirror-cmvs/src/adapter.rs#L2233) |
| `crate::vm::cvns_selector(step.1)` | `None` | Local state/value operations | Handled | [2325](../../crates/crysirror-cmvs/src/adapter.rs#L2325) |
| `condition; under step.1=0x237c..=0x237f` | `!session.buttons.list_command(step.1, &mut session.vm)?` | Local state/value operations | Handled | [3110](../../crates/crysirror-cmvs/src/adapter.rs#L3110) |
| `condition; under step.1=0x22d5` | `mode == 0` | u32::from | Handled | [3168](../../crates/crysirror-cmvs/src/adapter.rs#L3168) |
| `condition; under step.1=opcode @ (0x22d6..=0x22d8 \| 0x29ed)` | `opcode == 0x29ed` | Local state/value operations | Handled | [3179](../../crates/crysirror-cmvs/src/adapter.rs#L3179) |
| `condition; under step.1=opcode @ (0x22d6..=0x22d8 \| 0x29ed)` | `mode == 0` | u32::from | Handled | [3191](../../crates/crysirror-cmvs/src/adapter.rs#L3191) |
| `opcode; under step.1=0x228c..=0x228e` | `0x228c if first != second` | Local state/value operations | Handled | [3567](../../crates/crysirror-cmvs/src/adapter.rs#L3567) |
| `opcode; under step.1=0x228c..=0x228e` | `0x228d` | Local state/value operations | Handled | [3582](../../crates/crysirror-cmvs/src/adapter.rs#L3582) |
| `opcode; under step.1=0x228c..=0x228e` | `0x228e if first != second` | Local state/value operations | Handled | [3583](../../crates/crysirror-cmvs/src/adapter.rs#L3583) |
| `opcode; under step.1=0x228c..=0x228e` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [3592](../../crates/crysirror-cmvs/src/adapter.rs#L3592) |
| `visual::prepare_image(&sources, &record.name, &context.clone().with_cancellation(cancel.clone())); under step.1=0x22b5` | `Ok(image)` | Local state/value operations | Handled | [3784](../../crates/crysirror-cmvs/src/adapter.rs#L3784) |
| `visual::prepare_image(&sources, &record.name, &context.clone().with_cancellation(cancel.clone())); under step.1=0x22b5` | `Err(error) if error.to_string().starts_with("CMVS image missing")` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [3785](../../crates/crysirror-cmvs/src/adapter.rs#L3785) |
| `visual::prepare_image(&sources, &record.name, &context.clone().with_cancellation(cancel.clone())); under step.1=0x22b5` | `Err(error)` | Local state/value operations | Unsupported: Unconditional rejection | [3786](../../crates/crysirror-cmvs/src/adapter.rs#L3786) |
| `visual::prepare_image(&sources, name, &context.clone().with_cancellation(cancel.clone())); under step.1=0x22b5` | `Ok(image)` | Local state/value operations | Handled | [3792](../../crates/crysirror-cmvs/src/adapter.rs#L3792) |
| `visual::prepare_image(&sources, name, &context.clone().with_cancellation(cancel.clone())); under step.1=0x22b5` | `Err(error) if error.to_string().starts_with("CMVS image missing")` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [3793](../../crates/crysirror-cmvs/src/adapter.rs#L3793) |
| `visual::prepare_image(&sources, name, &context.clone().with_cancellation(cancel.clone())); under step.1=0x22b5` | `Err(error)` | Local state/value operations | Unsupported: Unconditional rejection | [3794](../../crates/crysirror-cmvs/src/adapter.rs#L3794) |
| `opcode; under step.1=0x2290..=0x2297` | `0x2290` | Local state/value operations | Handled | [4015](../../crates/crysirror-cmvs/src/adapter.rs#L4015) |
| `opcode; under step.1=0x2290..=0x2297` | `0x2292` | scene_effect::TOWER_TEXTURES.map | Handled | [4019](../../crates/crysirror-cmvs/src/adapter.rs#L4019) |
| `opcode; under step.1=0x2290..=0x2297` | `0x2294` | scene_effect::PARTICLE_TEXTURES.map | Handled | [4020](../../crates/crysirror-cmvs/src/adapter.rs#L4020) |
| `opcode; under step.1=0x2290..=0x2297` | `0x2295` | scene_effect::ACTOR_TEXTURES.map | Handled | [4021](../../crates/crysirror-cmvs/src/adapter.rs#L4021) |
| `opcode; under step.1=0x2290..=0x2297` | `0x2296` | scene_effect::SPIN_TEXTURES.map | Handled | [4022](../../crates/crysirror-cmvs/src/adapter.rs#L4022) |
| `opcode; under step.1=0x2290..=0x2297` | `0x2297` | scene_effect::ORBIT_TEXTURES.map | Handled | [4023](../../crates/crysirror-cmvs/src/adapter.rs#L4023) |
| `opcode; under step.1=0x2290..=0x2297` | `_` | Vec::new | Handled | [4024](../../crates/crysirror-cmvs/src/adapter.rs#L4024) |
| `opcode; under step.1=0x2290..=0x2297` | `0x2291` | Local state/value operations | Handled | [4027](../../crates/crysirror-cmvs/src/adapter.rs#L4027) |
| `opcode; under step.1=0x2290..=0x2297` | `0x2292` | Local state/value operations | Handled | [4028](../../crates/crysirror-cmvs/src/adapter.rs#L4028) |
| `opcode; under step.1=0x2290..=0x2297` | `0x2294` | Local state/value operations | Handled | [4029](../../crates/crysirror-cmvs/src/adapter.rs#L4029) |
| `opcode; under step.1=0x2290..=0x2297` | `0x2295` | Local state/value operations | Handled | [4030](../../crates/crysirror-cmvs/src/adapter.rs#L4030) |
| `opcode; under step.1=0x2290..=0x2297` | `_` | Local state/value operations | Handled | [4031](../../crates/crysirror-cmvs/src/adapter.rs#L4031) |
| `condition; under step.1=0x2290..=0x2297` | `opcode == 0x2293 \|\| !names.is_empty()` | ScriptSources::prepare, scene_effect::DOOR_PANELS.into_iter | Handled | [4041](../../crates/crysirror-cmvs/src/adapter.rs#L4041) |
| `condition; under step.1=0x2290..=0x2297` | `opcode == 0x2293` | scene_effect::DOOR_PANELS.into_iter | Handled | [4043](../../crates/crysirror-cmvs/src/adapter.rs#L4043) |
| `visual::prepare_image(&sources, &name, &context.with_cancellation(cancel.clone())); under step.1=0x22a1..=0x22a7 \| 0x22a9` | `Ok(image)` | Local state/value operations | Handled | [4165](../../crates/crysirror-cmvs/src/adapter.rs#L4165) |
| `visual::prepare_image(&sources, &name, &context.with_cancellation(cancel.clone())); under step.1=0x22a1..=0x22a7 \| 0x22a9` | `Err(error) if error.to_string().starts_with("CMVS image missing")` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [4166](../../crates/crysirror-cmvs/src/adapter.rs#L4166) |
| `visual::prepare_image(&sources, &name, &context.with_cancellation(cancel.clone())); under step.1=0x22a1..=0x22a7 \| 0x22a9` | `Err(error)` | Local state/value operations | Unsupported: Unconditional rejection | [4167](../../crates/crysirror-cmvs/src/adapter.rs#L4167) |
| `visual::prepare_image(&sources, &request.name, &context.with_cancellation(cancel.clone())); under step.1=0x222e` | `Ok(image)` | Local state/value operations | Handled | [4246](../../crates/crysirror-cmvs/src/adapter.rs#L4246) |
| `visual::prepare_image(&sources, &request.name, &context.with_cancellation(cancel.clone())); under step.1=0x222e` | `Err(error) if error.to_string().starts_with("CMVS image missing")` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [4247](../../crates/crysirror-cmvs/src/adapter.rs#L4247) |
| `visual::prepare_image(&sources, &request.name, &context.with_cancellation(cancel.clone())); under step.1=0x222e` | `Err(error)` | Local state/value operations | Unsupported: Unconditional rejection | [4248](../../crates/crysirror-cmvs/src/adapter.rs#L4248) |
| `visual::prepare_image_with(&sources, &name, prefetched, &context.with_cancellation(cancel.clone())); under step.1=0x2397` | `Ok(image)` | Local state/value operations | Handled | [4288](../../crates/crysirror-cmvs/src/adapter.rs#L4288) |
| `visual::prepare_image_with(&sources, &name, prefetched, &context.with_cancellation(cancel.clone())); under step.1=0x2397` | `Err(error) if error.to_string().starts_with("CMVS image missing")` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [4289](../../crates/crysirror-cmvs/src/adapter.rs#L4289) |
| `visual::prepare_image_with(&sources, &name, prefetched, &context.with_cancellation(cancel.clone())); under step.1=0x2397` | `Err(error)` | Local state/value operations | Unsupported: Unconditional rejection | [4290](../../crates/crysirror-cmvs/src/adapter.rs#L4290) |
| `visual::prepare_image_with( &sources, &name, prefetched, &context.with_cancellation(cancel.clone()), ); under step.1=0x2035` | `Ok(image)` | Local state/value operations | Handled | [4318](../../crates/crysirror-cmvs/src/adapter.rs#L4318) |
| `visual::prepare_image_with( &sources, &name, prefetched, &context.with_cancellation(cancel.clone()), ); under step.1=0x2035` | `Err(error) if error.to_string().starts_with("CMVS image missing")` | Local state/value operations | Handled | [4319](../../crates/crysirror-cmvs/src/adapter.rs#L4319) |
| `visual::prepare_image_with( &sources, &name, prefetched, &context.with_cancellation(cancel.clone()), ); under step.1=0x2035` | `Err(error)` | Local state/value operations | Unsupported: Unconditional rejection | [4324](../../crates/crysirror-cmvs/src/adapter.rs#L4324) |
| `visual::prepare_image(&sources, &name, &context.with_cancellation(cancel.clone())); under step.1=0x2310..=0x2317 / step.1=0x2310` | `Ok(image)` | Local state/value operations | Handled | [4372](../../crates/crysirror-cmvs/src/adapter.rs#L4372) |
| `visual::prepare_image(&sources, &name, &context.with_cancellation(cancel.clone())); under step.1=0x2310..=0x2317 / step.1=0x2310` | `Err(error) if error.to_string().starts_with("CMVS image missing")` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [4373](../../crates/crysirror-cmvs/src/adapter.rs#L4373) |
| `visual::prepare_image(&sources, &name, &context.with_cancellation(cancel.clone())); under step.1=0x2310..=0x2317 / step.1=0x2310` | `Err(error)` | Local state/value operations | Unsupported: Unconditional rejection | [4374](../../crates/crysirror-cmvs/src/adapter.rs#L4374) |
| `visual::prepare_image(&sources, &name, &context.with_cancellation(cancel.clone())); under step.1=0x23c0` | `Ok(image)` | Local state/value operations | Handled | [4427](../../crates/crysirror-cmvs/src/adapter.rs#L4427) |
| `visual::prepare_image(&sources, &name, &context.with_cancellation(cancel.clone())); under step.1=0x23c0` | `Err(error) if error.to_string().starts_with("CMVS image missing")` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [4428](../../crates/crysirror-cmvs/src/adapter.rs#L4428) |
| `visual::prepare_image(&sources, &name, &context.with_cancellation(cancel.clone())); under step.1=0x23c0` | `Err(error)` | Local state/value operations | Unsupported: Unconditional rejection | [4429](../../crates/crysirror-cmvs/src/adapter.rs#L4429) |
| `session.dialogs.command( step.1, &mut session.vm, host, self.client, self.canvas, &font, )?; under step.1=0x213a..=0x213d \| 0x29b1 \| 0x29b2 \| 0x29b6 \| 0x29b7` | `dialog::Outcome::Waiting` | Local state/value operations | Handled | [4477](../../crates/crysirror-cmvs/src/adapter.rs#L4477) |
| `session.dialogs.command( step.1, &mut session.vm, host, self.client, self.canvas, &font, )?; under step.1=0x213a..=0x213d \| 0x29b1 \| 0x29b2 \| 0x29b6 \| 0x29b7` | `dialog::Outcome::Done` | Local state/value operations | Empty handler: Original no-op status not established | [4478](../../crates/crysirror-cmvs/src/adapter.rs#L4478) |
| `session.dialogs.command( step.1, &mut session.vm, host, self.client, self.canvas, &font, )?; under step.1=0x213a..=0x213d \| 0x29b1 \| 0x29b2 \| 0x29b6 \| 0x29b7` | `dialog::Outcome::Face(face) if step.1 == 0x29b6` | Local state/value operations | Handled | [4479](../../crates/crysirror-cmvs/src/adapter.rs#L4479) |
| `session.dialogs.command( step.1, &mut session.vm, host, self.client, self.canvas, &font, )?; under step.1=0x213a..=0x213d \| 0x29b1 \| 0x29b2 \| 0x29b6 \| 0x29b7` | `dialog::Outcome::Face(face)` | Local state/value operations | Handled | [4482](../../crates/crysirror-cmvs/src/adapter.rs#L4482) |
| `condition; under step.1=opcode if session.display.command( opcode, &mut session.vm, host, self.client, self.canvas, )?` | `opcode == 0x22f9` | Local state/value operations | Handled | [4496](../../crates/crysirror-cmvs/src/adapter.rs#L4496) |
| `condition; under step.1=opcode if session.display.command( opcode, &mut session.vm, host, self.client, self.canvas, )?` | `opcode == 0x22f9 && session.menu.published()` | Local state/value operations | Handled | [4500](../../crates/crysirror-cmvs/src/adapter.rs#L4500) |
| `condition; under step.1=0x2025 \| 0x2039 \| 0x2026 \| 0x202c..=0x202d \| 0x2041..=0x2043 \| 0x2049..=0x204c \| 0x204e \| 0x204f \| 0x2051 \| 0x2054 \| 0x2055 \| 0x2070..=0x2075 \| 0x2078 \| 0x2321` | `let Some(bit) = session.objects.property_command( step.1, host, self.client, &mut session.vm, )?` | Local state/value operations | Handled | [4529](../../crates/crysirror-cmvs/src/adapter.rs#L4529) |
| `condition; under step.1=0x21e0..=0x21e9 \| 0x21ef..=0x21fd` | `let Some(bit) = session.objects.track_command( step.1, host, self.client, &mut session.vm, )?` | Local state/value operations | Handled | [4599](../../crates/crysirror-cmvs/src/adapter.rs#L4599) |
| `visual::prepare_image(&sources, &request.name, &context.with_cancellation(cancel.clone())); under step.1=0x222a \| 0x2234` | `Ok(image) if request.reveal` | Local state/value operations | Handled | [4640](../../crates/crysirror-cmvs/src/adapter.rs#L4640) |
| `visual::prepare_image(&sources, &request.name, &context.with_cancellation(cancel.clone())); under step.1=0x222a \| 0x2234` | `Ok(image)` | Local state/value operations | Handled | [4641](../../crates/crysirror-cmvs/src/adapter.rs#L4641) |
| `visual::prepare_image(&sources, &request.name, &context.with_cancellation(cancel.clone())); under step.1=0x222a \| 0x2234` | `Err(error) if error.to_string().starts_with("CMVS image missing")` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [4642](../../crates/crysirror-cmvs/src/adapter.rs#L4642) |
| `visual::prepare_image(&sources, &request.name, &context.with_cancellation(cancel.clone())); under step.1=0x222a \| 0x2234` | `Err(error)` | Local state/value operations | Unsupported: Unconditional rejection | [4643](../../crates/crysirror-cmvs/src/adapter.rs#L4643) |

### adapter.rs / Adapter::begin_boot

```rust
fn begin_boot(&mut self, host: &mut dyn HostServices) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `(7..=9).contains(&key)` | Local state/value operations | Handled | [1209](../../crates/crysirror-cmvs/src/adapter.rs#L1209) |

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
| `opcode` | `0x22a2` | Local state/value operations | Handled | [312](../../crates/crysirror-cmvs/src/adapter.rs#L312) |
| `opcode` | `0x22a5` | Local state/value operations | Handled | [316](../../crates/crysirror-cmvs/src/adapter.rs#L316) |
| `opcode` | `0x22a1 \| 0x22a4` | Local state/value operations | Handled | [320](../../crates/crysirror-cmvs/src/adapter.rs#L320) |
| `opcode` | `0x22a3` | Local state/value operations | Handled | [324](../../crates/crysirror-cmvs/src/adapter.rs#L324) |
| `opcode` | `0x22a9` | Local state/value operations | Handled | [328](../../crates/crysirror-cmvs/src/adapter.rs#L328) |
| `opcode` | `_` | Local state/value operations | Handled | [332](../../crates/crysirror-cmvs/src/adapter.rs#L332) |

### adapter.rs / Session::cvns_volume

```rust
fn cvns_volume(&mut self, host: &mut dyn HostServices, client: ClientId, key: u32) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(key, 10 \| 11)` | self.cvns.music_gain, sound::MUSIC_CHANNELS.map | Handled | [604](../../crates/crysirror-cmvs/src/adapter.rs#L604) |
| `condition` | `matches!(key, 10 \| 12)` | self.effects.cvns_live_gains, self.cvns.effect_levels | Handled | [609](../../crates/crysirror-cmvs/src/adapter.rs#L609) |
| `condition` | `matches!(key, 10 \| 13)` | self.cvns.voice_settings, sound::cvns_gain | Handled | [612](../../crates/crysirror-cmvs/src/adapter.rs#L612) |
| `condition` | `matches!(key, 10 \| 14)` | self.cvns.movie_gain, self.movies.gain_targets, self.cvns.movies.gain_targets | Handled | [617](../../crates/crysirror-cmvs/src/adapter.rs#L617) |

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
| `condition` | `opcode == 0x2291` | scene_effect::Scene::Walk, scene_effect::Walk::start | Handled | [237](../../crates/crysirror-cmvs/src/adapter.rs#L237) |
| `opcode` | `0x2293` | self.light.set_view | Handled | [254](../../crates/crysirror-cmvs/src/adapter.rs#L254) |
| `opcode` | `0x2294` | self.light.set_view | Handled | [255](../../crates/crysirror-cmvs/src/adapter.rs#L255) |
| `opcode` | `0x2297` | self.light.set_view | Handled | [256](../../crates/crysirror-cmvs/src/adapter.rs#L256) |
| `opcode` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [257](../../crates/crysirror-cmvs/src/adapter.rs#L257) |

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
| `visual::prepare_image(&sources, &name, &context.clone().with_cancellation(cancel.clone()))` | `Ok(image)` | Local state/value operations | Handled | [392](../../crates/crysirror-cmvs/src/adapter.rs#L392) |
| `visual::prepare_image(&sources, &name, &context.clone().with_cancellation(cancel.clone()))` | `Err(error) if error.to_string().starts_with("CMVS image missing")` | Local state/value operations | Empty handler: Original no-op status not established | [393](../../crates/crysirror-cmvs/src/adapter.rs#L393) |
| `visual::prepare_image(&sources, &name, &context.clone().with_cancellation(cancel.clone()))` | `Err(error)` | Local state/value operations | Unsupported: Unconditional rejection | [394](../../crates/crysirror-cmvs/src/adapter.rs#L394) |

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
| `condition` | `mode == 0` | self.vm.complete_owner_call | Handled | [639](../../crates/crysirror-cmvs/src/adapter.rs#L639) |
| `condition` | `matches!(opcode, 0x22d9 \| 0x22dd)` | Local state/value operations | Handled | [650](../../crates/crysirror-cmvs/src/adapter.rs#L650) |
| `condition` | `matches!(opcode, 0x22d9 \| 0x22dd)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [650](../../crates/crysirror-cmvs/src/adapter.rs#L650) |

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

### adapter/cvns/bitmap.rs / Job::apply

```rust
fn apply(
        mut self, client: ClientId, load: impl FnOnce(&str, u32) -> Result<DecodedImage>,
        publish: bool,
    ) -> Result<Prepared>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `mode; under self.edit=Edit::Filter(layer, mode, parameter)` | `0` | Local state/value operations | Handled | [266](../../crates/crysirror-cmvs/src/adapter/cvns/bitmap.rs#L266) |
| `mode; under self.edit=Edit::Filter(layer, mode, parameter)` | `1` | Local state/value operations | Handled | [267](../../crates/crysirror-cmvs/src/adapter/cvns/bitmap.rs#L267) |
| `mode; under self.edit=Edit::Filter(layer, mode, parameter)` | `3 \| 4 \| 9` | Local state/value operations | Handled | [268](../../crates/crysirror-cmvs/src/adapter/cvns/bitmap.rs#L268) |
| `mode; under self.edit=Edit::Filter(layer, mode, parameter)` | `10..=16` | Local state/value operations | Handled | [269](../../crates/crysirror-cmvs/src/adapter/cvns/bitmap.rs#L269) |
| `mode; under self.edit=Edit::Filter(layer, mode, parameter)` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [270](../../crates/crysirror-cmvs/src/adapter/cvns/bitmap.rs#L270) |

### adapter/cvns/fx.rs / Effect::advance

```rust
fn advance(&mut self, delta: u32, base: Base, random: &mut impl FnMut() -> i32) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under kind=1 \| 257 \| 26` | `kind == 257` | Local state/value operations | Handled | [174](../../crates/crysirror-cmvs/src/adapter/cvns/fx.rs#L174) |
| `condition; under kind=1 \| 257 \| 26` | `kind == 257` | Local state/value operations | Handled | [177](../../crates/crysirror-cmvs/src/adapter/cvns/fx.rs#L177) |
| `condition; under kind=_ / low=10..=17` | `kind >= 0x100` | Local state/value operations | Handled | [266](../../crates/crysirror-cmvs/src/adapter/cvns/fx.rs#L266) |
| `condition; under kind=_ / low=18 \| 19` | `kind >= 0x100` | Local state/value operations | Handled | [284](../../crates/crysirror-cmvs/src/adapter/cvns/fx.rs#L284) |
| `kind; under kind=_ / low=18 \| 19` | `275` | Local state/value operations | Handled | [287](../../crates/crysirror-cmvs/src/adapter/cvns/fx.rs#L287) |
| `kind; under kind=_ / low=18 \| 19` | `19` | Local state/value operations | Handled | [288](../../crates/crysirror-cmvs/src/adapter/cvns/fx.rs#L288) |
| `kind; under kind=_ / low=18 \| 19` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [289](../../crates/crysirror-cmvs/src/adapter/cvns/fx.rs#L289) |
| `kind; under kind=_ / low=24 \| 25` | `24` | Local state/value operations | Handled | [316](../../crates/crysirror-cmvs/src/adapter/cvns/fx.rs#L316) |
| `kind; under kind=_ / low=24 \| 25` | `280` | Local state/value operations | Handled | [317](../../crates/crysirror-cmvs/src/adapter/cvns/fx.rs#L317) |
| `kind; under kind=_ / low=24 \| 25` | `25` | Local state/value operations | Handled | [318](../../crates/crysirror-cmvs/src/adapter/cvns/fx.rs#L318) |
| `kind; under kind=_ / low=24 \| 25` | `_` | Local state/value operations | Handled | [319](../../crates/crysirror-cmvs/src/adapter/cvns/fx.rs#L319) |
| `condition; under kind=_` | `matches!(kind, 19 \| 275)` | Local state/value operations | Handled | [332](../../crates/crysirror-cmvs/src/adapter/cvns/fx.rs#L332) |
| `condition; under kind=_` | `matches!(kind, 19 \| 275)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [332](../../crates/crysirror-cmvs/src/adapter/cvns/fx.rs#L332) |
| `kind` | `1 \| 257 \| 26` | Local state/value operations | Handled | [172](../../crates/crysirror-cmvs/src/adapter/cvns/fx.rs#L172) |
| `kind` | `764` | Local state/value operations | Handled | [184](../../crates/crysirror-cmvs/src/adapter/cvns/fx.rs#L184) |
| `kind` | `306` | self.removed.iter | Handled | [198](../../crates/crysirror-cmvs/src/adapter/cvns/fx.rs#L198) |
| `kind` | `_` | Self::clipped | Handled | [220](../../crates/crysirror-cmvs/src/adapter/cvns/fx.rs#L220) |

### adapter/cvns/fx.rs / Effect::new

```rust
fn new(kind: u32, duration: u32, a6: i32, a7: i32, a8: i32, base: Base, random: &mut impl FnMut() -> i32) -> Result<Self>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `kind` | `1 \| 2..=19 \| 24 \| 25 \| 26 \| 764` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [75](../../crates/crysirror-cmvs/src/adapter/cvns/fx.rs#L75) |
| `kind` | `_` | Local state/value operations | Handled | [76](../../crates/crysirror-cmvs/src/adapter/cvns/fx.rs#L76) |
| `condition` | `kind == 764` | Local state/value operations | Handled | [143](../../crates/crysirror-cmvs/src/adapter/cvns/fx.rs#L143) |

### adapter/cvns/fx.rs / Effect::record

```rust
fn record(&self) -> [u32;10]
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `self.kind==1 \|\| self.kind==257` | Local state/value operations | Handled | [65](../../crates/crysirror-cmvs/src/adapter/cvns/fx.rs#L65) |

### adapter/cvns/input.rs / Input::consume_movie_skip

```rust
fn consume_movie_skip(&mut self) -> bool
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `skip && self.host.buttons_down & key::ENTER != 0` | Local state/value operations | Handled | [64](../../crates/crysirror-cmvs/src/adapter/cvns/input.rs#L64) |

### adapter/cvns/mod.rs / Scene::command

```rust
fn command(
        &mut self,
        selector: u16,
        canvas: [u32; 2],
        muted: bool,
        vm: &mut CmvsVm,
        host: &mut dyn HostServices,
        client: ClientId,
    ) -> Result<Option<Step>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `selector; under selector=0x2158..=0x215a` | `0x2158` | self.capture_frame | Handled | [454](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L454) |
| `selector; under selector=0x2158..=0x215a` | `0x2159` | Local state/value operations | Handled | [475](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L475) |
| `selector; under selector=0x2158..=0x215a` | `_` | encoding_rs::SHIFT_JIS.decode | Handled | [476](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L476) |
| `condition; under selector=0x2158..=0x215a` | `selector == 0x2158` | Local state/value operations | Handled | [493](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L493) |
| `selector; under selector=0x2049..=0x204c` | `0x2049` | registry::Action::ReadText | Handled | [505](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L505) |
| `selector; under selector=0x2049..=0x204c` | `0x204a` | Local state/value operations | Handled | [506](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L506) |
| `selector; under selector=0x2049..=0x204c` | `0x204b` | registry::Action::Write | Handled | [507](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L507) |
| `selector; under selector=0x2049..=0x204c` | `_` | registry::Action::Write | Handled | [508](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L508) |
| `condition; under selector=0x20e4 \| 0x20e5` | `selector == 0x20e4` | Local state/value operations | Handled | [525](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L525) |
| `condition; under selector=0x206c \| 0x206d` | `selector == 0x206c` | Local state/value operations | Handled | [613](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L613) |
| `condition; under selector=0x2100 \| 0x2101` | `selector == 0x2100` | Local state/value operations | Handled | [619](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L619) |
| `selector; under selector=0x2051..=0x2053 \| 0x2130` | `0x2051` | BitmapSlot::Global | Handled | [633](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L633) |
| `selector; under selector=0x2051..=0x2053 \| 0x2130` | `0x2052` | BitmapSlot::Global | Handled | [637](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L637) |
| `selector; under selector=0x2051..=0x2053 \| 0x2130` | `0x2053` | BitmapSlot::Global | Handled | [641](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L641) |
| `selector; under selector=0x2051..=0x2053 \| 0x2130` | `_` | self.adv, BitmapSlot::Adv | Handled | [645](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L645) |
| `condition; under selector=0x2051..=0x2053 \| 0x2130` | `let (0x2053, BitmapSlot::Global(index)) = (selector, slot)` | self.picture | Handled | [655](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L655) |
| `condition; under selector=0x2050 \| 0x2057 \| 0x2058 \| 0x205c..=0x205f \| 0x2062 \| 0x2063 \| 0x2066 \| 0x2067` | `selector == 0x2062` | self.picture | Handled | [666](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L666) |
| `condition; under selector=0x2050 \| 0x2057 \| 0x2058 \| 0x205c..=0x205f \| 0x2062 \| 0x2063 \| 0x2066 \| 0x2067` | `matches!(selector, 0x2050 \| 0x205c \| 0x205d)` | bitmap::Picture::default | Handled | [674](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L674) |
| `condition; under selector=0x2050 \| 0x2057 \| 0x2058 \| 0x205c..=0x205f \| 0x2062 \| 0x2063 \| 0x2066 \| 0x2067` | `matches!(selector, 0x2050 \| 0x205c \| 0x205d)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [674](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L674) |
| `condition; under selector=0x2050 \| 0x2057 \| 0x2058 \| 0x205c..=0x205f \| 0x2062 \| 0x2063 \| 0x2066 \| 0x2067 / selector=0x205c \| 0x205d` | `selector == 0x205c` | self.release, BitmapSlot::Global | Handled | [687](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L687) |
| `selector; under selector=0x2050 \| 0x2057 \| 0x2058 \| 0x205c..=0x205f \| 0x2062 \| 0x2063 \| 0x2066 \| 0x2067` | `0x2050` | Edit::Create | Handled | [676](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L676) |
| `selector; under selector=0x2050 \| 0x2057 \| 0x2058 \| 0x205c..=0x205f \| 0x2062 \| 0x2063 \| 0x2066 \| 0x2067` | `0x2057` | Edit::Release | Handled | [677](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L677) |
| `selector; under selector=0x2050 \| 0x2057 \| 0x2058 \| 0x205c..=0x205f \| 0x2062 \| 0x2063 \| 0x2066 \| 0x2067` | `0x2058` | Local state/value operations | Handled | [682](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L682) |
| `selector; under selector=0x2050 \| 0x2057 \| 0x2058 \| 0x205c..=0x205f \| 0x2062 \| 0x2063 \| 0x2066 \| 0x2067` | `0x205c \| 0x205d` | self.picture, self.release, BitmapSlot::Global | Handled | [683](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L683) |
| `selector; under selector=0x2050 \| 0x2057 \| 0x2058 \| 0x205c..=0x205f \| 0x2062 \| 0x2063 \| 0x2066 \| 0x2067` | `0x205e` | Edit::Tint | Handled | [698](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L698) |
| `selector; under selector=0x2050 \| 0x2057 \| 0x2058 \| 0x205c..=0x205f \| 0x2062 \| 0x2063 \| 0x2066 \| 0x2067` | `0x205f` | Edit::Filter | Handled | [699](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L699) |
| `selector; under selector=0x2050 \| 0x2057 \| 0x2058 \| 0x205c..=0x205f \| 0x2062 \| 0x2063 \| 0x2066 \| 0x2067` | `0x2063` | Edit::Position | Handled | [700](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L700) |
| `selector; under selector=0x2050 \| 0x2057 \| 0x2058 \| 0x205c..=0x205f \| 0x2062 \| 0x2063 \| 0x2066 \| 0x2067` | `0x2066` | Edit::Fill | Handled | [701](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L701) |
| `selector; under selector=0x2050 \| 0x2057 \| 0x2058 \| 0x205c..=0x205f \| 0x2062 \| 0x2063 \| 0x2066 \| 0x2067` | `_` | self.picture | Handled | [702](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L702) |
| `condition; under selector=0x20c8..=0x20cb` | `selector == 0x20c8` | Local state/value operations | Handled | [771](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L771) |
| `selector; under selector=0x20c8..=0x20cb` | `0x20c8` | self.movies.remove, Step::Movie | Handled | [774](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L774) |
| `selector; under selector=0x20c8..=0x20cb` | `0x20c9` | self.movies.start, self.movie_gain | Handled | [781](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L781) |
| `selector; under selector=0x20c8..=0x20cb` | `0x20ca` | Local state/value operations | Handled | [785](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L785) |
| `selector; under selector=0x20c8..=0x20cb` | `_` | self.movies.remove | Handled | [786](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L786) |
| `condition; under selector=0x2070..=0x2072 \| 0x2074 / selector=0x2070 \| 0x2071` | `selector == 0x2071` | Local state/value operations | Handled | [800](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L800) |
| `selector; under selector=0x2070..=0x2072 \| 0x2074` | `0x2070 \| 0x2071` | self.sprites.insert, Sprite::new | Handled | [799](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L799) |
| `selector; under selector=0x2070..=0x2072 \| 0x2074` | `0x2072` | Sprite::new, self.sprites.insert | Handled | [804](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L804) |
| `selector; under selector=0x2070..=0x2072 \| 0x2074` | `_` | self.sprites.remove | Handled | [814](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L814) |
| `condition; under selector=0x2076 \| 0x213a` | `selector == 0x2076` | Local state/value operations | Handled | [825](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L825) |
| `condition; under selector=0x2076 \| 0x213a` | `selector == 0x2076` | Local state/value operations | Handled | [837](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L837) |
| `condition; under selector=0x2077 \| 0x2148` | `selector == 0x2077` | Local state/value operations | Handled | [849](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L849) |
| `selector; under selector=0x213e \| 0x213f \| 0x2141..=0x2143 \| 0x214f` | `0x214f` | Local state/value operations | Handled | [870](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L870) |
| `selector; under selector=0x213e \| 0x213f \| 0x2141..=0x2143 \| 0x214f` | `0x213f \| 0x2142` | Local state/value operations | Handled | [871](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L871) |
| `selector; under selector=0x213e \| 0x213f \| 0x2141..=0x2143 \| 0x214f` | `_` | Local state/value operations | Handled | [872](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L872) |
| `condition; under selector=0x213e \| 0x213f \| 0x2141..=0x2143 \| 0x214f` | `selector == 0x214f` | Local state/value operations | Handled | [878](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L878) |
| `selector; under selector=0x213e \| 0x213f \| 0x2141..=0x2143 \| 0x214f` | `0x213e` | Local state/value operations | Handled | [882](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L882) |
| `selector; under selector=0x213e \| 0x213f \| 0x2141..=0x2143 \| 0x214f` | `0x213f` | Local state/value operations | Handled | [883](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L883) |
| `selector; under selector=0x213e \| 0x213f \| 0x2141..=0x2143 \| 0x214f` | `0x2141` | Local state/value operations | Handled | [884](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L884) |
| `selector; under selector=0x213e \| 0x213f \| 0x2141..=0x2143 \| 0x214f` | `0x2142` | Local state/value operations | Handled | [885](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L885) |
| `selector; under selector=0x213e \| 0x213f \| 0x2141..=0x2143 \| 0x214f` | `_` | Local state/value operations | Handled | [886](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L886) |
| `condition; under selector=0x2079 \| 0x213b` | `selector == 0x2079` | Local state/value operations | Handled | [896](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L896) |
| `condition; under selector=0x207a \| 0x207f \| 0x2081 \| 0x213c \| 0x2140` | `selector < 0x2100` | Local state/value operations | Handled | [912](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L912) |
| `selector; under selector=0x207a \| 0x207f \| 0x2081 \| 0x213c \| 0x2140` | `0x207a \| 0x213c` | Local state/value operations | Handled | [922](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L922) |
| `selector; under selector=0x207a \| 0x207f \| 0x2081 \| 0x213c \| 0x2140` | `0x207f \| 0x2140` | Local state/value operations | Handled | [923](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L923) |
| `selector; under selector=0x207a \| 0x207f \| 0x2081 \| 0x213c \| 0x2140` | `_` | Local state/value operations | Handled | [924](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L924) |
| `condition; under selector=0x207b..=0x207d \| 0x2080 \| 0x2082 \| 0x213d` | `selector == 0x213d` | Local state/value operations | Handled | [929](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L929) |
| `selector; under selector=0x207b..=0x207d \| 0x2080 \| 0x2082 \| 0x213d` | `0x207b \| 0x213d if value < 128` | Local state/value operations | Handled | [938](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L938) |
| `selector; under selector=0x207b..=0x207d \| 0x2080 \| 0x2082 \| 0x213d` | `0x207b \| 0x213d` | Local state/value operations | Empty handler: Original no-op status not established | [939](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L939) |
| `selector; under selector=0x207b..=0x207d \| 0x2080 \| 0x2082 \| 0x213d` | `0x207c` | Local state/value operations | Handled | [940](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L940) |
| `selector; under selector=0x207b..=0x207d \| 0x2080 \| 0x2082 \| 0x213d` | `0x207d` | Local state/value operations | Handled | [941](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L941) |
| `selector; under selector=0x207b..=0x207d \| 0x2080 \| 0x2082 \| 0x213d` | `0x2080` | Local state/value operations | Handled | [942](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L942) |
| `selector; under selector=0x207b..=0x207d \| 0x2080 \| 0x2082 \| 0x213d` | `_` | Local state/value operations | Handled | [943](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L943) |
| `selector; under selector=0x2090 \| 0x2091 \| 0x2144 \| 0x2145` | `0x2090` | Local state/value operations | Handled | [988](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L988) |
| `selector; under selector=0x2090 \| 0x2091 \| 0x2144 \| 0x2145` | `0x2091` | Local state/value operations | Handled | [989](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L989) |
| `selector; under selector=0x2090 \| 0x2091 \| 0x2144 \| 0x2145` | `0x2144` | Local state/value operations | Handled | [990](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L990) |
| `selector; under selector=0x2090 \| 0x2091 \| 0x2144 \| 0x2145` | `_` | Local state/value operations | Handled | [991](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L991) |
| `condition; under selector=0x2090 \| 0x2091 \| 0x2144 \| 0x2145` | `matches!(selector, 0x2090 \| 0x2144)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [1011](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1011) |
| `condition; under selector=0x2092 \| 0x2146` | `selector == 0x2092` | Local state/value operations | Handled | [1016](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1016) |
| `condition; under selector=0x2093 \| 0x2147` | `selector == 0x2093` | Local state/value operations | Handled | [1027](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1027) |
| `condition; under selector=0x20d9..=0x20db \| 0x2102` | `matches!(selector, 0x20d9 \| 0x20db)` | Local state/value operations | Handled | [1045](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1045) |
| `condition; under selector=0x20d9..=0x20db \| 0x2102` | `matches!(selector, 0x20d9 \| 0x20db)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [1045](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1045) |
| `condition; under selector=0x20d9..=0x20db \| 0x2102` | `selector == 0x20db` | Local state/value operations | Handled | [1047](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1047) |
| `selector; under selector=0x20d9..=0x20db \| 0x2102` | `0x20d9` | Local state/value operations | Handled | [1052](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1052) |
| `selector; under selector=0x20d9..=0x20db \| 0x2102` | `0x20da` | Local state/value operations | Handled | [1053](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1053) |
| `selector; under selector=0x20d9..=0x20db \| 0x2102` | `0x20db` | Local state/value operations | Handled | [1054](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1054) |
| `selector; under selector=0x20d9..=0x20db \| 0x2102` | `_` | Local state/value operations | Handled | [1055](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1055) |
| `condition; under selector=0x20d0 \| 0x20d1` | `selector == 0x20d0` | text::Window::new | Handled | [1062](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1062) |
| `selector; under selector=0x20d2..=0x20d8 \| 0x20dc..=0x20df` | `0x20d5` | Local state/value operations | Handled | [1070](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1070) |
| `selector; under selector=0x20d2..=0x20d8 \| 0x20dc..=0x20df` | `0x20dd` | Local state/value operations | Handled | [1071](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1071) |
| `selector; under selector=0x20d2..=0x20d8 \| 0x20dc..=0x20df` | `0x20d7 \| 0x20df` | Local state/value operations | Handled | [1072](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1072) |
| `selector; under selector=0x20d2..=0x20d8 \| 0x20dc..=0x20df` | `0x20d6` | Local state/value operations | Handled | [1073](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1073) |
| `selector; under selector=0x20d2..=0x20d8 \| 0x20dc..=0x20df` | `0x20d4` | Local state/value operations | Handled | [1074](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1074) |
| `selector; under selector=0x20d2..=0x20d8 \| 0x20dc..=0x20df` | `_` | Local state/value operations | Handled | [1075](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1075) |
| `condition; under selector=0x20d2..=0x20d8 \| 0x20dc..=0x20df` | `matches!(selector, 0x20d7 \| 0x20df)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [1081](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1081) |
| `condition; under selector=0x20d2..=0x20d8 \| 0x20dc..=0x20df` | `selector == 0x20d7` | Local state/value operations | Handled | [1082](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1082) |
| `selector; under selector=0x20d2..=0x20d8 \| 0x20dc..=0x20df` | `0x20d2` | Local state/value operations | Handled | [1089](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1089) |
| `selector; under selector=0x20d2..=0x20d8 \| 0x20dc..=0x20df` | `0x20d3` | Local state/value operations | Handled | [1090](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1090) |
| `selector; under selector=0x20d2..=0x20d8 \| 0x20dc..=0x20df` | `0x20d4` | Local state/value operations | Handled | [1091](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1091) |
| `selector; under selector=0x20d2..=0x20d8 \| 0x20dc..=0x20df` | `0x20d5` | Local state/value operations | Handled | [1092](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1092) |
| `selector; under selector=0x20d2..=0x20d8 \| 0x20dc..=0x20df` | `0x20d6` | Local state/value operations | Handled | [1093](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1093) |
| `selector; under selector=0x20d2..=0x20d8 \| 0x20dc..=0x20df` | `0x20d7` | Local state/value operations | Handled | [1104](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1104) |
| `selector; under selector=0x20d2..=0x20d8 \| 0x20dc..=0x20df` | `0x20d8` | Local state/value operations | Handled | [1109](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1109) |
| `selector; under selector=0x20d2..=0x20d8 \| 0x20dc..=0x20df` | `0x20dc` | Local state/value operations | Handled | [1117](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1117) |
| `selector; under selector=0x20d2..=0x20d8 \| 0x20dc..=0x20df` | `0x20dd` | Local state/value operations | Handled | [1118](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1118) |
| `selector; under selector=0x20d2..=0x20d8 \| 0x20dc..=0x20df` | `0x20de` | Local state/value operations | Handled | [1119](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1119) |
| `selector; under selector=0x20d2..=0x20d8 \| 0x20dc..=0x20df` | `_` | Local state/value operations | Handled | [1120](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1120) |
| `condition; under selector=0x2114 \| 0x2119 \| 0x2121 \| 0x2124 \| 0x2125` | `selector == 0x2121` | Local state/value operations | Handled | [1184](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1184) |
| `selector; under selector=0x2114 \| 0x2119 \| 0x2121 \| 0x2124 \| 0x2125` | `0x2114 if (value as i32) > 0` | Local state/value operations | Handled | [1187](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1187) |
| `selector; under selector=0x2114 \| 0x2119 \| 0x2121 \| 0x2124 \| 0x2125` | `0x2114` | Local state/value operations | Empty handler: Original no-op status not established | [1188](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1188) |
| `selector; under selector=0x2114 \| 0x2119 \| 0x2121 \| 0x2124 \| 0x2125` | `0x2119` | Local state/value operations | Handled | [1189](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1189) |
| `selector; under selector=0x2114 \| 0x2119 \| 0x2121 \| 0x2124 \| 0x2125` | `0x2121` | Local state/value operations | Handled | [1190](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1190) |
| `selector; under selector=0x2114 \| 0x2119 \| 0x2121 \| 0x2124 \| 0x2125` | `0x2124` | Local state/value operations | Handled | [1191](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1191) |
| `selector; under selector=0x2114 \| 0x2119 \| 0x2121 \| 0x2124 \| 0x2125` | `_` | Local state/value operations | Handled | [1192](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1192) |
| `selector; under selector=0x2118 \| 0x211e \| 0x2122 \| 0x2123` | `0x2118` | Local state/value operations | Handled | [1207](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1207) |
| `selector; under selector=0x2118 \| 0x211e \| 0x2122 \| 0x2123` | `0x211e` | Local state/value operations | Handled | [1215](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1215) |
| `selector; under selector=0x2118 \| 0x211e \| 0x2122 \| 0x2123` | `0x2122` | Local state/value operations | Handled | [1223](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1223) |
| `selector; under selector=0x2118 \| 0x211e \| 0x2122 \| 0x2123` | `_` | Local state/value operations | Handled | [1224](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1224) |
| `condition; under selector=0x2126` | `mode == 0` | Local state/value operations | Handled | [1235](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1235) |
| `condition; under selector=0x2126` | `mode == 1` | Local state/value operations | Handled | [1238](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1238) |
| `mode; under selector=0x211a` | `0` | Local state/value operations | Handled | [1268](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1268) |
| `mode; under selector=0x211a` | `1` | Local state/value operations | Handled | [1269](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1269) |
| `mode; under selector=0x211a` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [1270](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1270) |
| `condition; under selector=0x2188 \| 0x2189` | `selector == 0x2189` | self.capture_frame | Handled | [1381](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1381) |
| `selector` | `0x2158..=0x215a` | self.clipboard.is_none, self.capture_frame, encoding_rs::SHIFT_JIS.decode, host.begin_desktop_request | Handled | [450](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L450) |
| `selector` | `0x2048` | registry::name | Handled | [496](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L496) |
| `selector` | `0x2049..=0x204c` | registry::name, registry::Action::ReadText, registry::Action::Write, Step::Registry, registry::Request::new | Handled | [501](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L501) |
| `selector` | `0x2200` | encoding_rs::SHIFT_JIS.decode_without_bom_handling, Step::MaskSheet | Handled | [512](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L512) |
| `selector` | `0x2201` | self.masks.register | Handled | [519](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L519) |
| `selector` | `0x20e4 \| 0x20e5` | Local state/value operations | Handled | [524](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L524) |
| `selector` | `0x2008` | Local state/value operations | Handled | [537](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L537) |
| `selector` | `0x2009` | self.primary.invalidate_all | Handled | [542](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L542) |
| `selector` | `0x200a` | Local state/value operations | Handled | [549](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L549) |
| `selector` | `0x200b` | self.primary.fill | Handled | [554](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L554) |
| `selector` | `0x200c` | self.primary.invalidate | Handled | [559](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L559) |
| `selector` | `0x200d` | self.primary.cancel | Handled | [564](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L564) |
| `selector` | `0x2030` | self.input.pointer | Handled | [568](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L568) |
| `selector` | `0x2031` | self.input.pointer | Handled | [573](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L573) |
| `selector` | `0x2038 \| 0x203a` | Local state/value operations | Handled | [580](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L580) |
| `selector` | `0x2039 \| 0x203b` | usize::from | Handled | [581](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L581) |
| `selector` | `0x203d` | Local state/value operations | Handled | [585](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L585) |
| `selector` | `0x2042` | Local state/value operations | Handled | [586](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L586) |
| `selector` | `0x2047` | Local state/value operations | Handled | [607](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L607) |
| `selector` | `0x206c \| 0x206d` | Local state/value operations | Handled | [611](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L611) |
| `selector` | `0x2100 \| 0x2101` | self.message | Handled | [616](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L616) |
| `selector` | `0x2040` | self.input.mask | Handled | [626](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L626) |
| `selector` | `0x2051..=0x2053 \| 0x2130` | BitmapSlot::Global, self.adv, BitmapSlot::Adv, encoding_rs::SHIFT_JIS.decode, self.picture | Handled | [631](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L631) |
| `selector` | `0x2050 \| 0x2057 \| 0x2058 \| 0x205c..=0x205f \| 0x2062 \| 0x2063 \| 0x2066 \| 0x2067` | self.picture, bitmap::Picture::default, Edit::Create, Edit::Release, self.release | Handled | [662](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L662) |
| `selector` | `0x2055` | self.release, BitmapSlot::Global | Handled | [711](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L711) |
| `selector` | `0x2056` | self.release, BitmapSlot::Global | Handled | [717](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L717) |
| `selector` | `0x2059` | Local state/value operations | Handled | [723](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L723) |
| `selector` | `0x2160` | Local state/value operations | Handled | [738](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L738) |
| `selector` | `0x2163` | bitmap::Edit::Capture, bitmap::Edit::Create, Step::Bitmap, BitmapSlot::Global, Default::default | Handled | [743](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L743) |
| `selector` | `0x205a` | BitmapSlot::Global, self.capture_frame, bitmap::Edit::Capture, Step::Bitmap, Default::default | Handled | [754](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L754) |
| `selector` | `0x205b` | Local state/value operations | Handled | [763](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L763) |
| `selector` | `0x20c8..=0x20cb` | self.movies.remove, Step::Movie, self.movies.start, self.movie_gain | Handled | [768](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L768) |
| `selector` | `0x20cc` | self.movies.remove | Handled | [792](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L792) |
| `selector` | `0x2070..=0x2072 \| 0x2074` | self.sprites.insert, Sprite::new, self.sprites.remove | Handled | [796](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L796) |
| `selector` | `0x2075` | self.sprites.retain | Handled | [820](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L820) |
| `selector` | `0x2076 \| 0x213a` | Kind::Part, self.bitmap_size | Handled | [824](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L824) |
| `selector` | `0x2077 \| 0x2148` | Local state/value operations | Handled | [848](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L848) |
| `selector` | `0x2131` | self.adv, self.release, BitmapSlot::Adv | Handled | [859](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L859) |
| `selector` | `0x213e \| 0x213f \| 0x2141..=0x2143 \| 0x214f` | self.adv, self.sprites.get_mut | Handled | [867](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L867) |
| `selector` | `0x2078` | Local state/value operations | Handled | [890](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L890) |
| `selector` | `0x2079 \| 0x213b` | self.sprites.get_mut | Handled | [895](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L895) |
| `selector` | `0x207a \| 0x207f \| 0x2081 \| 0x213c \| 0x2140` | Local state/value operations | Handled | [911](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L911) |
| `selector` | `0x207b..=0x207d \| 0x2080 \| 0x2082 \| 0x213d` | Local state/value operations | Handled | [928](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L928) |
| `selector` | `0x207e` | Local state/value operations | Handled | [947](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L947) |
| `selector` | `0x2083` | self.sprites.contains_key | Handled | [952](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L952) |
| `selector` | `0x2084` | Local state/value operations | Handled | [957](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L957) |
| `selector` | `0x2086` | Local state/value operations | Handled | [963](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L963) |
| `selector` | `0x2085` | self.sprites.get | Handled | [968](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L968) |
| `selector` | `0x2088` | self.sprites.remove, self.sprites.insert | Handled | [977](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L977) |
| `selector` | `0x2090 \| 0x2091 \| 0x2144 \| 0x2145` | usize::from | Handled | [985](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L985) |
| `selector` | `0x2092 \| 0x2146` | Local state/value operations | Handled | [1015](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1015) |
| `selector` | `0x2093 \| 0x2147` | Local state/value operations | Handled | [1026](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1026) |
| `selector` | `0x2094` | Local state/value operations | Handled | [1037](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1037) |
| `selector` | `0x20d9..=0x20db \| 0x2102` | Vec::new, self.messages.get_mut | Handled | [1043](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1043) |
| `selector` | `0x20d0 \| 0x20d1` | text::Window::new | Handled | [1059](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1059) |
| `selector` | `0x20d2..=0x20d8 \| 0x20dc..=0x20df` | self.message | Handled | [1065](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1065) |
| `selector` | `0x2110` | self.destroy_adv, text::Window::new, adv::Adv::new | Handled | [1125](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1125) |
| `selector` | `0x2111` | self.adv, self.adv_message, self.show_parts | Handled | [1140](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1140) |
| `selector` | `0x2112` | self.read_state, self.style, self.adv_message, self.print_done | Handled | [1158](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1158) |
| `selector` | `0x2113` | self.adv, self.destroy_adv | Handled | [1177](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1177) |
| `selector` | `0x2114 \| 0x2119 \| 0x2121 \| 0x2124 \| 0x2125` | self.adv_message | Handled | [1183](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1183) |
| `selector` | `0x2117` | self.style, self.adv_message | Handled | [1196](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1196) |
| `selector` | `0x2118 \| 0x211e \| 0x2122 \| 0x2123` | self.adv_message | Handled | [1203](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1203) |
| `selector` | `0x2126` | self.advs.get_mut, self.adv_message | Handled | [1228](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1228) |
| `selector` | `0x2139` | self.advs.get, u32::from, self.sprites.remove, SpriteId::Part | Handled | [1255](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1255) |
| `selector` | `0x211a` | self.adv | Handled | [1263](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1263) |
| `selector` | `0x211b` | self.adv, self.adv_message, self.show_parts | Handled | [1274](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1274) |
| `selector` | `0x211d` | self.adv_message | Handled | [1284](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1284) |
| `selector` | `0x211f` | self.adv | Handled | [1289](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1289) |
| `selector` | `0x2120` | self.read_state, self.wait_done | Handled | [1294](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1294) |
| `selector` | `0x2138` | self.adv, Sprite::new, Kind::Part, self.sprites.insert | Handled | [1309](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1309) |
| `selector` | `0x2150` | self.button | Handled | [1318](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1318) |
| `selector` | `0x2151` | self.adv, self.button | Handled | [1329](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1329) |
| `selector` | `0x2153` | self.input.pointer, self.adv, u32::from | Handled | [1336](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1336) |
| `selector` | `0x2154` | self.button | Handled | [1345](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1345) |
| `selector` | `0x2152` | self.advs.get, self.button | Handled | [1350](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1350) |
| `selector` | `0x2155` | self.button | Handled | [1358](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1358) |
| `selector` | `0x2156` | self.button, u32::from | Handled | [1365](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1365) |
| `selector` | `0x2188 \| 0x2189` | self.capture_frame, self.sprites.get_mut, encoding_rs::SHIFT_JIS.decode, Step::Rule | Handled | [1380](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1380) |
| `selector` | `0x218a` | Local state/value operations | Handled | [1398](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1398) |
| `selector` | `0x218b` | self.finish_rule | Handled | [1399](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1399) |
| `selector` | `0x218c` | self.rule.is_none, self.input.take, self.finish_rule | Handled | [1403](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1403) |
| `selector` | `0x218d` | fx::Effect::new | Handled | [1415](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1415) |
| `selector` | `0x218e` | self.sprites.get_mut, self.input.take | Handled | [1424](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1424) |
| `selector` | `0x218f` | Local state/value operations | Handled | [1442](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1442) |
| `selector` | `0x2191` | self.finish_transition | Handled | [1452](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1452) |
| `selector` | `0x2190` | self.transition.as_ref | Handled | [1456](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1456) |
| `selector` | `0x2192` | self.transition.is_none, self.input.take, self.finish_transition | Handled | [1462](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1462) |
| `selector` | `0x2193` | shake::Shake::new | Handled | [1479](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1479) |
| `selector` | `0x2194` | self.shake.is_none | Handled | [1483](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1483) |
| `selector` | `0x2195` | Local state/value operations | Handled | [1494](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1494) |
| `selector` | `0x2196` | Local state/value operations | Handled | [1499](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1499) |
| `selector` | `0x2004` | self.reset_clock, host.now_ms | Handled | [1500](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1500) |
| `selector` | `0x201f` | Local state/value operations | Handled | [1505](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1505) |
| `selector` | `0x2183` | Local state/value operations | Handled | [1512](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1512) |
| `selector` | `0x2018` | Local state/value operations | Handled | [1513](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1513) |
| `selector` | `0x2019` | Local state/value operations | Handled | [1514](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1514) |
| `selector` | `0x21ec` | self.setting | Handled | [1515](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1515) |
| `selector` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [1524](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1524) |

### adapter/cvns/mod.rs / Scene::frame

```rust
fn frame(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        canvas: [u32; 2],
    ) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `sprite.kind != Kind::Fill && sprite.tint.1 != 256` | self.tint_curves.entry, u16::from, crysirror_image::rgb_lut::RgbLookup::low_word_affine, DecodedImage::from_surface | Handled | [1894](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1894) |
| `condition` | `sprite.rule_blit && sprite.kind == Kind::Bitmap && sprite.scale == [1000; 2] && sprite.rotation == 0` | self.rule_image.as_ref | Handled | [1911](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1911) |
| `condition` | `let Some(layer) = message.layer(id, sprite::order(message.layer, 1064 + index as i32), offset, \|ink, generation\| self.masks.image(host, client, ink, generation, &mut used_masks))?` | Local state/value operations | Handled | [1928](../../crates/crysirror-cmvs/src/adapter/cvns/mod.rs#L1928) |

### adapter/cvns/settings.rs / Scene::menu_setting

```rust
fn menu_setting(&self, key: u32) -> Vec<(u32, bool, bool)>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `key; under key=_` | `1` | Local state/value operations | Handled | [91](../../crates/crysirror-cmvs/src/adapter/cvns/settings.rs#L91) |
| `key; under key=_` | `3` | Local state/value operations | Handled | [92](../../crates/crysirror-cmvs/src/adapter/cvns/settings.rs#L92) |
| `key; under key=_` | `4` | Local state/value operations | Handled | [93](../../crates/crysirror-cmvs/src/adapter/cvns/settings.rs#L93) |
| `key; under key=_` | `5` | Local state/value operations | Handled | [94](../../crates/crysirror-cmvs/src/adapter/cvns/settings.rs#L94) |
| `key; under key=_` | `7..=9` | Local state/value operations | Handled | [95](../../crates/crysirror-cmvs/src/adapter/cvns/settings.rs#L95) |
| `key; under key=_` | `16` | Local state/value operations | Handled | [96](../../crates/crysirror-cmvs/src/adapter/cvns/settings.rs#L96) |
| `key; under key=_` | `21` | Local state/value operations | Handled | [97](../../crates/crysirror-cmvs/src/adapter/cvns/settings.rs#L97) |
| `key; under key=_` | `22` | Local state/value operations | Handled | [98](../../crates/crysirror-cmvs/src/adapter/cvns/settings.rs#L98) |
| `key; under key=_` | `30` | Local state/value operations | Handled | [99](../../crates/crysirror-cmvs/src/adapter/cvns/settings.rs#L99) |
| `key; under key=_` | `_` | Local state/value operations | Handled | [100](../../crates/crysirror-cmvs/src/adapter/cvns/settings.rs#L100) |
| `key` | `2` | Local state/value operations | Handled | [79](../../crates/crysirror-cmvs/src/adapter/cvns/settings.rs#L79) |
| `key` | `6` | Local state/value operations | Handled | [80](../../crates/crysirror-cmvs/src/adapter/cvns/settings.rs#L80) |
| `key` | `10..=14` | Local state/value operations | Handled | [81](../../crates/crysirror-cmvs/src/adapter/cvns/settings.rs#L81) |
| `key` | `17` | Local state/value operations | Handled | [82](../../crates/crysirror-cmvs/src/adapter/cvns/settings.rs#L82) |
| `key` | `18` | Local state/value operations | Handled | [83](../../crates/crysirror-cmvs/src/adapter/cvns/settings.rs#L83) |
| `key` | `19` | Local state/value operations | Handled | [84](../../crates/crysirror-cmvs/src/adapter/cvns/settings.rs#L84) |
| `key` | `20` | Local state/value operations | Handled | [85](../../crates/crysirror-cmvs/src/adapter/cvns/settings.rs#L85) |
| `key` | `28` | Local state/value operations | Handled | [86](../../crates/crysirror-cmvs/src/adapter/cvns/settings.rs#L86) |
| `key` | `29` | Local state/value operations | Handled | [87](../../crates/crysirror-cmvs/src/adapter/cvns/settings.rs#L87) |
| `key` | `31` | Local state/value operations | Handled | [88](../../crates/crysirror-cmvs/src/adapter/cvns/settings.rs#L88) |
| `key` | `_` | Local state/value operations | Handled | [89](../../crates/crysirror-cmvs/src/adapter/cvns/settings.rs#L89) |
| `key` | `6` | Local state/value operations | Handled | [108](../../crates/crysirror-cmvs/src/adapter/cvns/settings.rs#L108) |
| `key` | `28` | Local state/value operations | Handled | [109](../../crates/crysirror-cmvs/src/adapter/cvns/settings.rs#L109) |
| `key` | `29` | Local state/value operations | Handled | [110](../../crates/crysirror-cmvs/src/adapter/cvns/settings.rs#L110) |
| `key` | `_` | Local state/value operations | Handled | [111](../../crates/crysirror-cmvs/src/adapter/cvns/settings.rs#L111) |

### adapter/cvns/settings.rs / Scene::setting

```rust
fn setting(&self, key: u32) -> Option<u32>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `key` | `6` | Local state/value operations | Handled | [17](../../crates/crysirror-cmvs/src/adapter/cvns/settings.rs#L17) |
| `key` | `15` | Local state/value operations | Handled | [18](../../crates/crysirror-cmvs/src/adapter/cvns/settings.rs#L18) |
| `key` | `27` | u32::from | Handled | [19](../../crates/crysirror-cmvs/src/adapter/cvns/settings.rs#L19) |
| `key` | `999` | u32::from | Handled | [20](../../crates/crysirror-cmvs/src/adapter/cvns/settings.rs#L20) |
| `key` | `1 \| 3..=5 \| 7..=9 \| 16 \| 21..=24` | u32::from | Handled | [21](../../crates/crysirror-cmvs/src/adapter/cvns/settings.rs#L21) |
| `key` | `1..=31` | Local state/value operations | Handled | [24](../../crates/crysirror-cmvs/src/adapter/cvns/settings.rs#L24) |
| `key` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [25](../../crates/crysirror-cmvs/src/adapter/cvns/settings.rs#L25) |

### adapter/cvns/settings.rs / Scene::write_setting

```rust
fn write_setting(&mut self, key: u32, value: u32, text: &[u8]) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `key` | `3 \| 4 if signed < 0` | Local state/value operations | Handled | [33](../../crates/crysirror-cmvs/src/adapter/cvns/settings.rs#L33) |
| `key` | `1 \| 3..=5 \| 7..=9 \| 16 \| 21 \| 23 \| 24 \| 30` | Local state/value operations | Handled | [34](../../crates/crysirror-cmvs/src/adapter/cvns/settings.rs#L34) |
| `key` | `2` | Local state/value operations | Handled | [35](../../crates/crysirror-cmvs/src/adapter/cvns/settings.rs#L35) |
| `key` | `6` | Local state/value operations | Handled | [36](../../crates/crysirror-cmvs/src/adapter/cvns/settings.rs#L36) |
| `key` | `10..=14` | Local state/value operations | Handled | [40](../../crates/crysirror-cmvs/src/adapter/cvns/settings.rs#L40) |
| `key` | `15` | Local state/value operations | Handled | [41](../../crates/crysirror-cmvs/src/adapter/cvns/settings.rs#L41) |
| `key` | `17 \| 19` | Local state/value operations | Handled | [45](../../crates/crysirror-cmvs/src/adapter/cvns/settings.rs#L45) |
| `key` | `18 \| 20 \| 31` | Local state/value operations | Handled | [46](../../crates/crysirror-cmvs/src/adapter/cvns/settings.rs#L46) |
| `key` | `22` | Local state/value operations | Handled | [47](../../crates/crysirror-cmvs/src/adapter/cvns/settings.rs#L47) |
| `key` | `25 \| 26 \| 28` | Local state/value operations | Handled | [51](../../crates/crysirror-cmvs/src/adapter/cvns/settings.rs#L51) |
| `key` | `27` | Local state/value operations | Handled | [52](../../crates/crysirror-cmvs/src/adapter/cvns/settings.rs#L52) |
| `key` | `29` | Local state/value operations | Handled | [56](../../crates/crysirror-cmvs/src/adapter/cvns/settings.rs#L56) |
| `key` | `999` | Local state/value operations | Handled | [63](../../crates/crysirror-cmvs/src/adapter/cvns/settings.rs#L63) |
| `key` | `_` | Local state/value operations | Handled | [67](../../crates/crysirror-cmvs/src/adapter/cvns/settings.rs#L67) |

### adapter/cvns/sprite.rs / Sprite::finish_effect

```rust
fn finish_effect(&mut self) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `effect.kind == 1` | Local state/value operations | Handled | [282](../../crates/crysirror-cmvs/src/adapter/cvns/sprite.rs#L282) |

### adapter/cvns/text.rs / Window::advance

```rust
fn advance(&mut self, delta: u32) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `step == 0 && reveal.mode != 0` | Local state/value operations | Handled | [317](../../crates/crysirror-cmvs/src/adapter/cvns/text.rs#L317) |
| `reveal.mode` | `1 if reveal.elapsed < reveal.duration` | Local state/value operations | Handled | [323](../../crates/crysirror-cmvs/src/adapter/cvns/text.rs#L323) |
| `reveal.mode` | `1` | Local state/value operations | Handled | [327](../../crates/crysirror-cmvs/src/adapter/cvns/text.rs#L327) |
| `reveal.mode` | `2 if reveal.elapsed < reveal.duration` | Local state/value operations | Handled | [331](../../crates/crysirror-cmvs/src/adapter/cvns/text.rs#L331) |
| `reveal.mode` | `2` | Local state/value operations | Handled | [335](../../crates/crysirror-cmvs/src/adapter/cvns/text.rs#L335) |
| `reveal.mode` | `_` | Local state/value operations | Handled | [339](../../crates/crysirror-cmvs/src/adapter/cvns/text.rs#L339) |

### adapter/cvns/text.rs / Window::print

```rust
fn print(&mut self, text: &[u8], style: Option<Style>, reveal: bool, generation: usize) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `code` | `0x8141` | Local state/value operations | Handled | [235](../../crates/crysirror-cmvs/src/adapter/cvns/text.rs#L235) |
| `code` | `0x8142` | Local state/value operations | Handled | [236](../../crates/crysirror-cmvs/src/adapter/cvns/text.rs#L236) |
| `code` | `0x8149 if next_space` | Local state/value operations | Handled | [237](../../crates/crysirror-cmvs/src/adapter/cvns/text.rs#L237) |
| `code` | `0x8148 if next_space` | Local state/value operations | Handled | [238](../../crates/crysirror-cmvs/src/adapter/cvns/text.rs#L238) |
| `code` | `_` | Local state/value operations | Handled | [239](../../crates/crysirror-cmvs/src/adapter/cvns/text.rs#L239) |
| `mode` | `1` | Local state/value operations | Handled | [265](../../crates/crysirror-cmvs/src/adapter/cvns/text.rs#L265) |
| `mode` | `2` | Local state/value operations | Handled | [266](../../crates/crysirror-cmvs/src/adapter/cvns/text.rs#L266) |
| `mode` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [267](../../crates/crysirror-cmvs/src/adapter/cvns/text.rs#L267) |

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
| `condition; under opcode=0x213b \| 0x29b1` | `opcode == 0x213b && mode == 0` | self.open_choice | Handled | [112](../../crates/crysirror-cmvs/src/adapter/dialog.rs#L112) |
| `condition; under opcode=0x213c \| 0x29b2` | `opcode == 0x29b2` | Local state/value operations | Handled | [124](../../crates/crysirror-cmvs/src/adapter/dialog.rs#L124) |
| `opcode` | `0x213a` | self.open_choice | Handled | [103](../../crates/crysirror-cmvs/src/adapter/dialog.rs#L103) |
| `opcode` | `0x213b \| 0x29b1` | self.open_choice | Handled | [107](../../crates/crysirror-cmvs/src/adapter/dialog.rs#L107) |
| `opcode` | `0x213c \| 0x29b2` | self.open_edit | Handled | [119](../../crates/crysirror-cmvs/src/adapter/dialog.rs#L119) |
| `opcode` | `0x29b7` | super::cvns::rgb, Kind::Color, crysirror_scene::ui::color_picker::ColorPicker::open | Handled | [130](../../crates/crysirror-cmvs/src/adapter/dialog.rs#L130) |
| `opcode` | `0x29b6` | host.font_library, self.open_font_page | Handled | [139](../../crates/crysirror-cmvs/src/adapter/dialog.rs#L139) |
| `opcode` | `0x213d` | host.font_library, self.open_choice | Handled | [146](../../crates/crysirror-cmvs/src/adapter/dialog.rs#L146) |
| `opcode` | `_` | Local state/value operations | Handled | [156](../../crates/crysirror-cmvs/src/adapter/dialog.rs#L156) |

### adapter/dialog.rs / Dialogs::poll

```rust
fn poll(&mut self, vm: &mut CmvsVm, host: &mut dyn HostServices, client: ClientId) -> Result<Outcome>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `event["kind"] == "activate" && matches!(id, Some(EDIT_OK \| EDIT_FIELD))` | Local state/value operations | Handled | [252](../../crates/crysirror-cmvs/src/adapter/dialog.rs#L252) |
| `condition` | `event["kind"] == "activate" && id == Some(EDIT_CANCEL)` | Local state/value operations | Handled | [254](../../crates/crysirror-cmvs/src/adapter/dialog.rs#L254) |
| `condition` | `event["kind"] == "dialog_result" && event["widget"]["id"].as_u64() == Some(u64::from(DIALOG))` | Local state/value operations | Handled | [288](../../crates/crysirror-cmvs/src/adapter/dialog.rs#L288) |
| `condition; under pending.kind=_` | `matches!(opcode, 0x213b \| 0x29b1)` | Local state/value operations | Handled | [318](../../crates/crysirror-cmvs/src/adapter/dialog.rs#L318) |
| `condition; under pending.kind=_` | `matches!(opcode, 0x213b \| 0x29b1)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [318](../../crates/crysirror-cmvs/src/adapter/dialog.rs#L318) |

### adapter/display.rs / Display::alpha_command

```rust
fn alpha_command(&mut self, vm: &mut CmvsVm) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `mode == 0` | u32::from | Handled | [271](../../crates/crysirror-cmvs/src/adapter/display.rs#L271) |

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
| `condition; under opcode=0x22fd` | `mode <= 3` | Local state/value operations | Handled | [529](../../crates/crysirror-cmvs/src/adapter/display.rs#L529) |
| `opcode` | `0x22f9 if revision` | self.alpha_command | Handled | [378](../../crates/crysirror-cmvs/src/adapter/display.rs#L378) |
| `opcode` | `0x22ff if self.profile == VmProfile::Pc20160201X64` | u32::from | Handled | [379](../../crates/crysirror-cmvs/src/adapter/display.rs#L379) |
| `opcode` | `0x2005` | self.frame_period | Handled | [382](../../crates/crysirror-cmvs/src/adapter/display.rs#L382) |
| `opcode` | `0x20cd` | Self::submit_ui | Handled | [383](../../crates/crysirror-cmvs/src/adapter/display.rs#L383) |
| `opcode` | `0x2163` | self.observe | Handled | [390](../../crates/crysirror-cmvs/src/adapter/display.rs#L390) |
| `opcode` | `0x2164` | self.write, u32::from, self.apply_mode | Handled | [395](../../crates/crysirror-cmvs/src/adapter/display.rs#L395) |
| `opcode` | `0x2168` | Self::submit_ui | Handled | [402](../../crates/crysirror-cmvs/src/adapter/display.rs#L402) |
| `opcode` | `0x2169` | host.window_minimized | Handled | [413](../../crates/crysirror-cmvs/src/adapter/display.rs#L413) |
| `opcode` | `0x216c` | Local state/value operations | Handled | [418](../../crates/crysirror-cmvs/src/adapter/display.rs#L418) |
| `opcode` | `0x216d` | self.write | Handled | [419](../../crates/crysirror-cmvs/src/adapter/display.rs#L419) |
| `opcode` | `0x216e` | self.observe, Self::submit_ui | Handled | [431](../../crates/crysirror-cmvs/src/adapter/display.rs#L431) |
| `opcode` | `0x216f \| 0x22ca` | u32::from | Handled | [443](../../crates/crysirror-cmvs/src/adapter/display.rs#L443) |
| `opcode` | `0x22c8` | Local state/value operations | Handled | [444](../../crates/crysirror-cmvs/src/adapter/display.rs#L444) |
| `opcode` | `0x22c9` | Self::submit_ui, self.apply_mode | Handled | [445](../../crates/crysirror-cmvs/src/adapter/display.rs#L445) |
| `opcode` | `0x22d2` | self.write, u32::from | Handled | [462](../../crates/crysirror-cmvs/src/adapter/display.rs#L462) |
| `opcode` | `0x22d3 \| 0x22d4` | usize::from, self.write, u32::from | Handled | [472](../../crates/crysirror-cmvs/src/adapter/display.rs#L472) |
| `opcode` | `0x22e9` | self.write | Handled | [484](../../crates/crysirror-cmvs/src/adapter/display.rs#L484) |
| `opcode` | `0x22ea` | self.write | Handled | [494](../../crates/crysirror-cmvs/src/adapter/display.rs#L494) |
| `opcode` | `0x22ec` | self.apply_mode, u32::from | Handled | [504](../../crates/crysirror-cmvs/src/adapter/display.rs#L504) |
| `opcode` | `0x22ed if revision` | self.write, u32::from | Handled | [516](../../crates/crysirror-cmvs/src/adapter/display.rs#L516) |
| `opcode` | `0x22fc` | Local state/value operations | Handled | [526](../../crates/crysirror-cmvs/src/adapter/display.rs#L526) |
| `opcode` | `0x22fd` | self.write, self.apply_mode | Handled | [527](../../crates/crysirror-cmvs/src/adapter/display.rs#L527) |
| `opcode` | `0x23c5 if revision` | Local state/value operations | Handled | [539](../../crates/crysirror-cmvs/src/adapter/display.rs#L539) |
| `opcode` | `0x23c6 if revision` | self.write | Handled | [540](../../crates/crysirror-cmvs/src/adapter/display.rs#L540) |
| `opcode` | `0x23c7 if revision` | Local state/value operations | Handled | [549](../../crates/crysirror-cmvs/src/adapter/display.rs#L549) |
| `opcode` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [550](../../crates/crysirror-cmvs/src/adapter/display.rs#L550) |

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
| `condition` | `let Some(id) = item["id"].as_u64().map(\|id\| id as u32)` | Value::Bool, Value::String, encoding_rs::SHIFT_JIS.decode | Handled | [210](../../crates/crysirror-cmvs/src/adapter/menu.rs#L210) |

### adapter/menu.rs / collect_ids

```rust
fn collect_ids(items: &[MenuTemplateItem], used: &mut BTreeSet<u32>) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Some(id) = item.id.filter(\|&id\| id != 0)` | Local state/value operations | Handled | [160](../../crates/crysirror-cmvs/src/adapter/menu.rs#L160) |

### adapter/menu.rs / cvns_slots

```rust
fn cvns_slots(items: &mut [Value], slots: &SaveMenu) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Some(id) = item["id"].as_u64()` | encoding_rs::SHIFT_JIS.decode | Handled | [135](../../crates/crysirror-cmvs/src/adapter/menu.rs#L135) |

### adapter/menu.rs / items

```rust
fn items(native: &[MenuTemplateItem], ids: &mut impl Iterator<Item = u32>) -> Result<Vec<Value>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `item.id` | `Some(id) if id != 0` | Local state/value operations | Handled | [176](../../crates/crysirror-cmvs/src/adapter/menu.rs#L176) |
| `item.id` | `_` | Local state/value operations | Handled | [177](../../crates/crysirror-cmvs/src/adapter/menu.rs#L177) |

### adapter/menu.rs / NativeMenu::update

```rust
fn update(items: &mut [Value], changes: &[(u32, bool, bool)]) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Some((_, enabled, checked)) = changes.iter().find(\|(id, _, _)\| item["id"].as_u64() == Some(u64::from(*id)))` | Local state/value operations | Handled | [110](../../crates/crysirror-cmvs/src/adapter/menu.rs#L110) |

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

### adapter/movie.rs / Movies::begin

```rust
fn begin(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        vm: &mut CmvsVm,
        source: Option<Arc<dyn AssetReader>>,
        gain: f32,
        frame_control: Option<u32>,
    ) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `mode % 3` | `1` | Local state/value operations | Handled | [50](../../crates/crysirror-cmvs/src/adapter/movie.rs#L50) |
| `mode % 3` | `2` | Local state/value operations | Handled | [50](../../crates/crysirror-cmvs/src/adapter/movie.rs#L50) |
| `mode % 3` | `_` | Local state/value operations | Handled | [50](../../crates/crysirror-cmvs/src/adapter/movie.rs#L50) |

### adapter/movie.rs / ScenePlayer::advance

```rust
fn advance(&mut self, now: u64, audio: Option<u64>) -> Option<u32>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `self.mode` | `0` | self.steps, self.pending.take | Handled | [353](../../crates/crysirror-cmvs/src/adapter/movie.rs#L353) |
| `self.mode` | `10` | self.steps | Handled | [377](../../crates/crysirror-cmvs/src/adapter/movie.rs#L377) |
| `self.mode` | `15` | u64::from | Handled | [392](../../crates/crysirror-cmvs/src/adapter/movie.rs#L392) |
| `self.mode` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [399](../../crates/crysirror-cmvs/src/adapter/movie.rs#L399) |

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

### adapter/sound.rs / cvns_fader_update

```rust
fn cvns_fader_update(selector: u16, word: u32) -> AudioPcm16GainUpdate
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(selector, 0x20af \| 0x20be)` | AudioPcm16GainUpdate::Stereo | Handled | [149](../../crates/crysirror-cmvs/src/adapter/sound.rs#L149) |
| `condition` | `matches!(selector, 0x20af \| 0x20be)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [149](../../crates/crysirror-cmvs/src/adapter/sound.rs#L149) |

### adapter/sound.rs / fader_update

```rust
fn fader_update(opcode: u16, word: u32) -> AudioPcm16GainUpdate
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(opcode, 0x20ad \| 0x20b9)` | AudioPcm16GainUpdate::Stereo | Handled | [141](../../crates/crysirror-cmvs/src/adapter/sound.rs#L141) |
| `condition` | `matches!(opcode, 0x20ad \| 0x20b9)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [141](../../crates/crysirror-cmvs/src/adapter/sound.rs#L141) |

### adapter/sound.rs / Levels::live_channels

```rust
fn live_channels(self, opcode: u16, effects: &Effects) -> Vec<(u32, f32)>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `self.music.enabled && matches!(opcode, 0x22d9 \| 0x22da)` | self.music.gain | Handled | [1267](../../crates/crysirror-cmvs/src/adapter/sound.rs#L1267) |
| `condition` | `matches!(opcode, 0x22d9 \| 0x22da)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [1267](../../crates/crysirror-cmvs/src/adapter/sound.rs#L1267) |
| `condition` | `matches!(opcode, 0x22d9 \| 0x22dc)` | Local state/value operations | Handled | [1270](../../crates/crysirror-cmvs/src/adapter/sound.rs#L1270) |
| `condition` | `matches!(opcode, 0x22d9 \| 0x22dc)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [1270](../../crates/crysirror-cmvs/src/adapter/sound.rs#L1270) |
| `condition` | `matches!(opcode, 0x22d9 \| 0x22db)` | Local state/value operations | Handled | [1274](../../crates/crysirror-cmvs/src/adapter/sound.rs#L1274) |
| `condition` | `matches!(opcode, 0x22d9 \| 0x22db)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [1274](../../crates/crysirror-cmvs/src/adapter/sound.rs#L1274) |
| `condition` | `opcode == 0x22db && volume > 0` | Local state/value operations | Handled | [1278](../../crates/crysirror-cmvs/src/adapter/sound.rs#L1278) |

### adapter/sound.rs / Levels::read

```rust
fn read(self, opcode: u16) -> u32
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `opcode` | `0x22d9` | Local state/value operations | Handled | [1236](../../crates/crysirror-cmvs/src/adapter/sound.rs#L1236) |
| `opcode` | `0x22da` | Local state/value operations | Handled | [1237](../../crates/crysirror-cmvs/src/adapter/sound.rs#L1237) |
| `opcode` | `0x22db` | Local state/value operations | Handled | [1238](../../crates/crysirror-cmvs/src/adapter/sound.rs#L1238) |
| `opcode` | `0x22dc` | Local state/value operations | Handled | [1239](../../crates/crysirror-cmvs/src/adapter/sound.rs#L1239) |
| `opcode` | `0x22dd` | Local state/value operations | Handled | [1240](../../crates/crysirror-cmvs/src/adapter/sound.rs#L1240) |
| `opcode` | `_` | Local state/value operations | Handled | [1241](../../crates/crysirror-cmvs/src/adapter/sound.rs#L1241) |

### adapter/sound.rs / Levels::updated

```rust
fn updated(mut self, opcode: u16, raw: u32) -> Self
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `opcode` | `0x22d9` | Local state/value operations | Handled | [1248](../../crates/crysirror-cmvs/src/adapter/sound.rs#L1248) |
| `opcode` | `0x22da` | Local state/value operations | Handled | [1253](../../crates/crysirror-cmvs/src/adapter/sound.rs#L1253) |
| `opcode` | `0x22db` | Local state/value operations | Handled | [1254](../../crates/crysirror-cmvs/src/adapter/sound.rs#L1254) |
| `opcode` | `0x22dc` | Local state/value operations | Handled | [1255](../../crates/crysirror-cmvs/src/adapter/sound.rs#L1255) |
| `opcode` | `0x22dd` | Local state/value operations | Handled | [1256](../../crates/crysirror-cmvs/src/adapter/sound.rs#L1256) |
| `opcode` | `_` | Local state/value operations | Handled | [1257](../../crates/crysirror-cmvs/src/adapter/sound.rs#L1257) |
| `condition` | `matches!(opcode, 0x22d9 \| 0x22dd)` | Local state/value operations | Handled | [1259](../../crates/crysirror-cmvs/src/adapter/sound.rs#L1259) |
| `condition` | `matches!(opcode, 0x22d9 \| 0x22dd)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [1259](../../crates/crysirror-cmvs/src/adapter/sound.rs#L1259) |

### adapter/sound.rs / MusicRequest::read

```rust
fn read(vm: &CmvsVm, opcode: u16) -> Result<Self>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `opcode == 0x20a4` | Local state/value operations | Handled | [1305](../../crates/crysirror-cmvs/src/adapter/sound.rs#L1305) |

### adapter/sound.rs / VoiceRequest::read

```rust
fn read(vm: &CmvsVm, opcode: u16) -> Result<Self>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `opcode == 0x20af` | f32::from_bits | Handled | [778](../../crates/crysirror-cmvs/src/adapter/sound.rs#L778) |

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
| `opcode` | `0x200..=0x202` | InstructionKind::Expression | Handled | [338](../../crates/crysirror-cmvs/src/vm.rs#L338) |
| `opcode` | `0x400` | Local state/value operations | Handled | [342](../../crates/crysirror-cmvs/src/vm.rs#L342) |
| `opcode` | `0x401 \| 0x402` | Local state/value operations | Handled | [345](../../crates/crysirror-cmvs/src/vm.rs#L345) |
| `opcode` | `0x403` | Local state/value operations | Handled | [348](../../crates/crysirror-cmvs/src/vm.rs#L348) |
| `opcode` | `0x407` | Local state/value operations | Handled | [352](../../crates/crysirror-cmvs/src/vm.rs#L352) |
| `opcode` | `0x405 \| 0x410` | Local state/value operations | Handled | [356](../../crates/crysirror-cmvs/src/vm.rs#L356) |
| `opcode` | `0x411 \| 0x412` | Local state/value operations | Handled | [359](../../crates/crysirror-cmvs/src/vm.rs#L359) |
| `opcode` | `0x414` | Local state/value operations | Handled | [362](../../crates/crysirror-cmvs/src/vm.rs#L362) |
| `opcode` | `0x413 \| 0x416 \| 0x430` | Local state/value operations | Handled | [365](../../crates/crysirror-cmvs/src/vm.rs#L365) |
| `opcode` | `0x440 \| 0x442` | Local state/value operations | Handled | [366](../../crates/crysirror-cmvs/src/vm.rs#L366) |
| `opcode` | `0x2000..=0x27ff` | Local state/value operations | Handled | [370](../../crates/crysirror-cmvs/src/vm.rs#L370) |
| `opcode` | `_` | Local state/value operations | Unsupported: unsupported instruction | [373](../../crates/crysirror-cmvs/src/vm.rs#L373) |

### vm.rs / Reader<'_>::expression

```rust
fn expression(&mut self, depth: usize) -> Result<Expression, DecodeError>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!(0x200..=0x202).contains(&kind)` | self.error | Unsupported: expected expression prefix | [203](../../crates/crysirror-cmvs/src/vm.rs#L203) |
| `condition; under tag=0x101 \| 0x102 \| 0x12b if kind != 0x201` | `expression.kind != 0x200` | self.error | Unsupported: index must be integer expression | [239](../../crates/crysirror-cmvs/src/vm.rs#L239) |
| `condition; under tag=0x121 if kind == 0x201` | `expression.kind != 0x200` | self.error | Unsupported: string index must be integer | [250](../../crates/crysirror-cmvs/src/vm.rs#L250) |
| `condition; under tag=0x105 \| 0x109 \| 0x110 \| 0x112 \| 0x12d \| 0x12f \| 0x134 \| 0x136 if kind != 0x201 && (tag < 0x130 \|\| kind == 0x202)` | `expression.kind != 0x200` | self.error | Unsupported: index must be integer expression | [264](../../crates/crysirror-cmvs/src/vm.rs#L264) |
| `condition; under tag=0x107 \| 0x10b \| 0x111 \| 0x113 \| 0x131 \| 0x133 \| 0x135 \| 0x137 if kind != 0x201 && (tag < 0x130 \|\| kind == 0x202)` | `indices.iter().any(\|e\| e.kind != 0x200)` | self.error | Unsupported: index must be integer expression | [284](../../crates/crysirror-cmvs/src/vm.rs#L284) |

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
| `opcode; under opcode=0x2008 \| 0x2009 \| 0x200f` | `0x2008` | Local state/value operations | Handled | [355](../../crates/crysirror-cmvs/src/vm/control.rs#L355) |
| `opcode; under opcode=0x2008 \| 0x2009 \| 0x200f` | `0x2009` | Local state/value operations | Handled | [356](../../crates/crysirror-cmvs/src/vm/control.rs#L356) |
| `opcode; under opcode=0x2008 \| 0x2009 \| 0x200f` | `_` | self.clock.wrapping_sub | Handled | [357](../../crates/crysirror-cmvs/src/vm/control.rs#L357) |
| `condition; under opcode=0x200b \| 0x200c` | `opcode == 0x200b` | Local state/value operations | Handled | [371](../../crates/crysirror-cmvs/src/vm/control.rs#L371) |
| `condition; under opcode=0x20d4 \| 0x20d5` | `opcode == 0x20d4` | Local state/value operations | Handled | [387](../../crates/crysirror-cmvs/src/vm/control.rs#L387) |
| `opcode; under opcode=0x20d0..=0x20d3` | `0x20d0` | Local state/value operations | Handled | [405](../../crates/crysirror-cmvs/src/vm/control.rs#L405) |
| `opcode; under opcode=0x20d0..=0x20d3` | `0x20d1` | Local state/value operations | Handled | [406](../../crates/crysirror-cmvs/src/vm/control.rs#L406) |
| `opcode; under opcode=0x20d0..=0x20d3` | `_` | Local state/value operations | Handled | [407](../../crates/crysirror-cmvs/src/vm/control.rs#L407) |
| `opcode; under opcode=0x20d0..=0x20d3` | `0x20d0` | Local state/value operations | Handled | [415](../../crates/crysirror-cmvs/src/vm/control.rs#L415) |
| `opcode; under opcode=0x20d0..=0x20d3` | `0x20d1` | Local state/value operations | Handled | [416](../../crates/crysirror-cmvs/src/vm/control.rs#L416) |
| `opcode; under opcode=0x20d0..=0x20d3` | `0x20d2` | Local state/value operations | Handled | [417](../../crates/crysirror-cmvs/src/vm/control.rs#L417) |
| `opcode; under opcode=0x20d0..=0x20d3` | `_` | Local state/value operations | Handled | [418](../../crates/crysirror-cmvs/src/vm/control.rs#L418) |
| `condition; under opcode=0x20d7 / mode=1 \| 2` | `mode == 1` | Local state/value operations | Handled | [445](../../crates/crysirror-cmvs/src/vm/control.rs#L445) |
| `mode; under opcode=0x20d7` | `0` | Local state/value operations | Handled | [441](../../crates/crysirror-cmvs/src/vm/control.rs#L441) |
| `mode; under opcode=0x20d7` | `1 \| 2` | f64::from | Handled | [442](../../crates/crysirror-cmvs/src/vm/control.rs#L442) |
| `mode; under opcode=0x20d7` | `_` | Local state/value operations | Handled | [451](../../crates/crysirror-cmvs/src/vm/control.rs#L451) |
| `condition; under opcode=0x20d8 \| 0x20d9 \| 0x20dc` | `opcode == 0x20dc` | Local state/value operations | Handled | [461](../../crates/crysirror-cmvs/src/vm/control.rs#L461) |
| `condition; under opcode=0x20d8 \| 0x20d9 \| 0x20dc` | `opcode == 0x20d8` | Local state/value operations | Handled | [466](../../crates/crysirror-cmvs/src/vm/control.rs#L466) |
| `condition; under opcode=0x20da \| 0x20db` | `opcode == 0x20da` | Local state/value operations | Handled | [503](../../crates/crysirror-cmvs/src/vm/control.rs#L503) |
| `condition; under opcode=0x2088 \| 0x208b` | `opcode == 0x208b` | Local state/value operations | Handled | [546](../../crates/crysirror-cmvs/src/vm/control.rs#L546) |
| `condition; under opcode=0x2089 \| 0x208c` | `opcode == 0x2089` | Local state/value operations | Handled | [574](../../crates/crysirror-cmvs/src/vm/control.rs#L574) |
| `opcode` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [581](../../crates/crysirror-cmvs/src/vm/control.rs#L581) |

### vm/control.rs / Control::cvns_command

```rust
fn cvns_command(
        &mut self,
        machine: &Machine,
        opcode: u16,
    ) -> Result<u16, DecodeError>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `opcode` | `0x2028` | Self::argument | Handled | [286](../../crates/crysirror-cmvs/src/vm/control.rs#L286) |
| `opcode` | `0x2029` | Local state/value operations | Handled | [310](../../crates/crysirror-cmvs/src/vm/control.rs#L310) |
| `opcode` | `0x202a` | Local state/value operations | Handled | [314](../../crates/crysirror-cmvs/src/vm/control.rs#L314) |
| `opcode` | `_` | self.callbacks.get_mut | Handled | [323](../../crates/crysirror-cmvs/src/vm/control.rs#L323) |

### vm/cvns.rs / returns_without_effect

```rust
fn returns_without_effect(opcode: u16) -> bool
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(opcode, 0x2001 \| 0x200e..=0x200f \| 0x2011 \| 0x2015..=0x2017 \| 0x201d..=0x201e \| 0x2027 \| 0x202d..=0x202f \| 0x2035..=0x2037 \| 0x203e..=0x203f \| 0x2043..=0x2046 \| 0x204d..=0x204f \| 0x2054 \| 0x2060..=0x2061 \| 0x2064..=0x2065 \| 0x206a \| 0x206e..=0x206f \| 0x2073 \| 0x2087 \| 0x2089..=0x208f \| 0x2095..=0x209f \| 0x20a3 \| 0x20a7..=0x20ac \| 0x20bf \| 0x20c1..=0x20c4 \| 0x20cd..=0x20cf \| 0x20ec..=0x20ef \| 0x20fe..=0x20ff \| 0x2103..=0x2107 \| 0x210f \| 0x2115..=0x2116 \| 0x211c \| 0x2127 \| 0x2132..=0x2135 \| 0x2137 \| 0x2149..=0x214e \| 0x2157 \| 0x215b..=0x215f \| 0x2161..=0x2162 \| 0x2164..=0x2167 \| 0x2169..=0x2177 \| 0x217e \| 0x2184..=0x2187 \| 0x2197..=0x219f \| 0x21a8 \| 0x21ad \| 0x21b3..=0x21b5 \| 0x21b8..=0x21bf \| 0x21c5..=0x21c7 \| 0x21cd..=0x21d3 \| 0x21d6..=0x21eb \| 0x21ee..=0x21ff \| 0x2202..=0x221f )` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [107](../../crates/crysirror-cmvs/src/vm/cvns.rs#L107) |

### vm/cvns.rs / route

```rust
fn route(command: u16) -> u16
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `opcode` | `0x2013` | Local state/value operations | Handled | [17](../../crates/crysirror-cmvs/src/vm/cvns.rs#L17) |
| `opcode` | `0x201c` | Local state/value operations | Handled | [19](../../crates/crysirror-cmvs/src/vm/cvns.rs#L19) |
| `opcode` | `0x21a2` | Local state/value operations | Handled | [21](../../crates/crysirror-cmvs/src/vm/cvns.rs#L21) |
| `opcode` | `0x203c` | Local state/value operations | Handled | [23](../../crates/crysirror-cmvs/src/vm/cvns.rs#L23) |
| `opcode` | `0x21c8` | Local state/value operations | Handled | [25](../../crates/crysirror-cmvs/src/vm/cvns.rs#L25) |
| `opcode` | `0x21c9` | Local state/value operations | Handled | [26](../../crates/crysirror-cmvs/src/vm/cvns.rs#L26) |
| `opcode` | `0x21cb` | Local state/value operations | Handled | [27](../../crates/crysirror-cmvs/src/vm/cvns.rs#L27) |
| `opcode` | `0x21cc` | Local state/value operations | Handled | [28](../../crates/crysirror-cmvs/src/vm/cvns.rs#L28) |
| `opcode` | `0x2178` | Local state/value operations | Handled | [30](../../crates/crysirror-cmvs/src/vm/cvns.rs#L30) |
| `opcode` | `0x217b` | Local state/value operations | Handled | [32](../../crates/crysirror-cmvs/src/vm/cvns.rs#L32) |
| `opcode` | `0x2168` | Local state/value operations | Handled | [34](../../crates/crysirror-cmvs/src/vm/cvns.rs#L34) |
| `opcode` | `0x21d4` | Local state/value operations | Handled | [36](../../crates/crysirror-cmvs/src/vm/cvns.rs#L36) |
| `opcode` | `0x21d5` | Local state/value operations | Handled | [38](../../crates/crysirror-cmvs/src/vm/cvns.rs#L38) |
| `opcode` | `0x21a5` | Local state/value operations | Handled | [40](../../crates/crysirror-cmvs/src/vm/cvns.rs#L40) |
| `opcode` | `0x21a9` | Local state/value operations | Handled | [42](../../crates/crysirror-cmvs/src/vm/cvns.rs#L42) |
| `opcode` | `0x21aa` | Local state/value operations | Handled | [43](../../crates/crysirror-cmvs/src/vm/cvns.rs#L43) |
| `opcode` | `0x21a6` | Local state/value operations | Handled | [45](../../crates/crysirror-cmvs/src/vm/cvns.rs#L45) |
| `opcode` | `0x21af` | Local state/value operations | Handled | [47](../../crates/crysirror-cmvs/src/vm/cvns.rs#L47) |
| `opcode` | `0x21c2` | Local state/value operations | Handled | [50](../../crates/crysirror-cmvs/src/vm/cvns.rs#L50) |
| `opcode` | `0x21c3` | Local state/value operations | Handled | [51](../../crates/crysirror-cmvs/src/vm/cvns.rs#L51) |
| `opcode` | `0x202b` | Local state/value operations | Handled | [53](../../crates/crysirror-cmvs/src/vm/cvns.rs#L53) |
| `opcode` | `0x2179` | Local state/value operations | Handled | [56](../../crates/crysirror-cmvs/src/vm/cvns.rs#L56) |
| `opcode` | `0x217a` | Local state/value operations | Handled | [57](../../crates/crysirror-cmvs/src/vm/cvns.rs#L57) |
| `opcode` | `0x2068` | Local state/value operations | Handled | [59](../../crates/crysirror-cmvs/src/vm/cvns.rs#L59) |
| `opcode` | `0x201a` | Local state/value operations | Handled | [61](../../crates/crysirror-cmvs/src/vm/cvns.rs#L61) |
| `opcode` | `0x201b` | Local state/value operations | Handled | [62](../../crates/crysirror-cmvs/src/vm/cvns.rs#L62) |
| `opcode` | `0x20b0..=0x20b3` | Local state/value operations | Handled | [66](../../crates/crysirror-cmvs/src/vm/cvns.rs#L66) |
| `opcode` | `0x20ad` | Local state/value operations | Handled | [67](../../crates/crysirror-cmvs/src/vm/cvns.rs#L67) |
| `opcode` | `0x20ae` | Local state/value operations | Handled | [68](../../crates/crysirror-cmvs/src/vm/cvns.rs#L68) |
| `opcode` | `0x20b5` | Local state/value operations | Handled | [69](../../crates/crysirror-cmvs/src/vm/cvns.rs#L69) |
| `opcode` | `0x20e0` | Local state/value operations | Handled | [72](../../crates/crysirror-cmvs/src/vm/cvns.rs#L72) |
| `opcode` | `0x20e1` | Local state/value operations | Handled | [73](../../crates/crysirror-cmvs/src/vm/cvns.rs#L73) |
| `opcode` | `0x20e2` | Local state/value operations | Handled | [74](../../crates/crysirror-cmvs/src/vm/cvns.rs#L74) |
| `opcode` | `0x20e3` | Local state/value operations | Handled | [75](../../crates/crysirror-cmvs/src/vm/cvns.rs#L75) |
| `opcode` | `0x20e6` | Local state/value operations | Handled | [76](../../crates/crysirror-cmvs/src/vm/cvns.rs#L76) |
| `opcode` | `0x20a0..=0x20a2` | Local state/value operations | Handled | [79](../../crates/crysirror-cmvs/src/vm/cvns.rs#L79) |
| `opcode` | `0x20b4` | Local state/value operations | Handled | [80](../../crates/crysirror-cmvs/src/vm/cvns.rs#L80) |
| `opcode` | `0x2006` | Local state/value operations | Handled | [82](../../crates/crysirror-cmvs/src/vm/cvns.rs#L82) |
| `opcode` | `0x21b0` | Local state/value operations | Handled | [85](../../crates/crysirror-cmvs/src/vm/cvns.rs#L85) |
| `opcode` | `0x21a0` | Local state/value operations | Handled | [88](../../crates/crysirror-cmvs/src/vm/cvns.rs#L88) |
| `opcode` | `0x2033` | Local state/value operations | Handled | [90](../../crates/crysirror-cmvs/src/vm/cvns.rs#L90) |
| `opcode` | `0x2032` | Local state/value operations | Handled | [92](../../crates/crysirror-cmvs/src/vm/cvns.rs#L92) |
| `opcode` | `0x2034` | Local state/value operations | Handled | [94](../../crates/crysirror-cmvs/src/vm/cvns.rs#L94) |
| `opcode` | `_` | Local state/value operations | Handled | [95](../../crates/crysirror-cmvs/src/vm/cvns.rs#L95) |

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
| `opcode` | `0x160` | Local state/value operations | Handled | [1114](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1114) |
| `opcode` | `0x161` | Local state/value operations | Handled | [1115](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1115) |
| `opcode` | `0x162` | Local state/value operations | Handled | [1118](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1118) |
| `opcode` | `0x163` | Local state/value operations | Handled | [1121](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1121) |
| `opcode` | `0x164` | Local state/value operations | Handled | [1122](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1122) |
| `opcode` | `0x165` | Local state/value operations | Handled | [1123](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1123) |
| `opcode` | `0x166` | Local state/value operations | Handled | [1124](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1124) |
| `opcode` | `0x167` | Local state/value operations | Handled | [1125](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1125) |
| `opcode` | `0x168` | Local state/value operations | Handled | [1126](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1126) |
| `opcode` | `0x169` | Local state/value operations | Handled | [1127](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1127) |
| `opcode` | `0x16a` | i32::from | Handled | [1128](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1128) |
| `opcode` | `0x16b` | i32::from | Handled | [1129](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1129) |
| `opcode` | `0x16c` | i32::from | Handled | [1130](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1130) |
| `opcode` | `0x16d` | i32::from | Handled | [1131](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1131) |
| `opcode` | `0x16e` | i32::from | Handled | [1132](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1132) |
| `opcode` | `0x16f` | i32::from | Handled | [1133](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1133) |
| `opcode` | `0x171` | i32::from | Handled | [1134](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1134) |
| `opcode` | `0x172` | i32::from | Handled | [1135](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1135) |
| `opcode` | `_` | Local state/value operations | Unsupported: unsupported integer operator | [1136](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1136) |

### vm/evaluate.rs / float_binary

```rust
fn float_binary(pc: usize, opcode: u16, left: f32, right: f32) -> Result<f32, DecodeError>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `opcode` | `0x160` | Local state/value operations | Handled | [1075](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1075) |
| `opcode` | `0x161` | Local state/value operations | Handled | [1076](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1076) |
| `opcode` | `0x163` | Local state/value operations | Handled | [1077](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1077) |
| `opcode` | `0x164` | Local state/value operations | Handled | [1078](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1078) |
| `opcode` | `0x162 \| 0x165..=0x169` | Local state/value operations | Handled | [1079](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1079) |
| `opcode` | `0x16a` | Local state/value operations | Handled | [1080](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1080) |
| `opcode` | `0x16b` | Local state/value operations | Handled | [1081](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1081) |
| `opcode` | `0x16c` | Local state/value operations | Handled | [1082](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1082) |
| `opcode` | `0x16d` | Local state/value operations | Handled | [1083](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1083) |
| `opcode` | `0x16e` | Local state/value operations | Handled | [1084](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1084) |
| `opcode` | `0x16f` | Local state/value operations | Handled | [1085](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1085) |
| `opcode` | `0x171` | Local state/value operations | Handled | [1086](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1086) |
| `opcode` | `0x172` | Local state/value operations | Handled | [1087](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1087) |
| `opcode` | `_` | Local state/value operations | Unsupported: unsupported float operator | [1088](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1088) |

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
| `condition` | `expr.kind != 0x200` | Local state/value operations | Unsupported: not an integer expression | [660](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L660) |
| `condition; under term=Term::Operator { pc, opcode: op @ (0x173 \| 0x174), }` | `*op == 0x173` | Local state/value operations | Handled | [743](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L743) |
| `condition; under term=Term::Operator { pc, opcode }` | `*opcode == 0x170` | self.read, self.write | Handled | [758](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L758) |
| `opcode; under term=Term::Operator { pc, opcode }` | `0x175` | Local state/value operations | Handled | [767](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L767) |
| `opcode; under term=Term::Operator { pc, opcode }` | `0x176` | Local state/value operations | Handled | [768](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L768) |
| `opcode; under term=Term::Operator { pc, opcode }` | `0x177` | Local state/value operations | Handled | [769](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L769) |
| `opcode; under term=Term::Operator { pc, opcode }` | `0x178` | Local state/value operations | Handled | [770](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L770) |
| `opcode; under term=Term::Operator { pc, opcode }` | `0x179` | Local state/value operations | Handled | [771](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L771) |
| `opcode; under term=Term::Operator { pc, opcode }` | `0x17a` | Local state/value operations | Handled | [772](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L772) |
| `opcode; under term=Term::Operator { pc, opcode }` | `0x17b` | Local state/value operations | Handled | [773](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L773) |
| `opcode; under term=Term::Operator { pc, opcode }` | `0x17c` | Local state/value operations | Handled | [774](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L774) |
| `opcode; under term=Term::Operator { pc, opcode }` | `0x17d` | Local state/value operations | Handled | [775](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L775) |
| `opcode; under term=Term::Operator { pc, opcode }` | `0x17e` | Local state/value operations | Handled | [776](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L776) |
| `opcode; under term=Term::Operator { pc, opcode }` | `other` | Local state/value operations | Handled | [777](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L777) |
| `condition; under term=Term::Operator { pc, opcode }` | `(0x175..=0x17e).contains(opcode)` | self.write | Handled | [780](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L780) |

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
| `condition` | `expr.kind != 0x202` | Local state/value operations | Unsupported: not a float expression | [912](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L912) |
| `condition; under term=Term::Operator { pc, opcode: op @ (0x173 \| 0x174), }` | `*op == 0x173` | Local state/value operations | Handled | [996](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L996) |
| `condition; under term=Term::Operator { pc, opcode }` | `*opcode == 0x170` | self.read_float, self.write_float | Handled | [1007](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1007) |
| `condition; under term=Term::Operator { pc, opcode }` | `(0x175..=0x17e).contains(opcode)` | self.read_float, self.write_float | Handled | [1011](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1011) |
| `opcode; under term=Term::Operator { pc, opcode }` | `0x175` | Local state/value operations | Handled | [1016](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1016) |
| `opcode; under term=Term::Operator { pc, opcode }` | `0x176` | Local state/value operations | Handled | [1017](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1017) |
| `opcode; under term=Term::Operator { pc, opcode }` | `0x177` | Local state/value operations | Handled | [1018](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1018) |
| `opcode; under term=Term::Operator { pc, opcode }` | `0x178` | Local state/value operations | Handled | [1019](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1019) |
| `opcode; under term=Term::Operator { pc, opcode }` | `0x179` | Local state/value operations | Handled | [1020](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1020) |
| `opcode; under term=Term::Operator { pc, opcode }` | `0x17a` | Local state/value operations | Handled | [1021](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1021) |
| `opcode; under term=Term::Operator { pc, opcode }` | `0x17b` | Local state/value operations | Handled | [1022](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1022) |
| `opcode; under term=Term::Operator { pc, opcode }` | `0x17c` | Local state/value operations | Handled | [1023](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1023) |
| `opcode; under term=Term::Operator { pc, opcode }` | `0x17d` | Local state/value operations | Handled | [1024](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1024) |
| `opcode; under term=Term::Operator { pc, opcode }` | `_` | Local state/value operations | Handled | [1025](../../crates/crysirror-cmvs/src/vm/evaluate.rs#L1025) |

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
| `condition; under opcode=0x2131..=0x2135` | `opcode == 0x2135` | Local state/value operations | Handled | [228](../../crates/crysirror-cmvs/src/vm/history.rs#L228) |
| `opcode` | `0x212e` | Local state/value operations | Handled | [214](../../crates/crysirror-cmvs/src/vm/history.rs#L214) |
| `opcode` | `0x2137` | Local state/value operations | Handled | [215](../../crates/crysirror-cmvs/src/vm/history.rs#L215) |
| `opcode` | `0x212f` | self.entries.clear | Handled | [219](../../crates/crysirror-cmvs/src/vm/history.rs#L219) |
| `opcode` | `0x2130` | self.entries.len | Handled | [223](../../crates/crysirror-cmvs/src/vm/history.rs#L223) |
| `opcode` | `0x2131..=0x2135` | self.entries.get, StringRef::decode, usize::from, Journal::default | Handled | [224](../../crates/crysirror-cmvs/src/vm/history.rs#L224) |
| `opcode` | `0x2136` | Default::default, self.entries.len, self.entries.pop_back, self.entries.push_front | Handled | [249](../../crates/crysirror-cmvs/src/vm/history.rs#L249) |
| `opcode` | `_` | Local state/value operations | Handled | [269](../../crates/crysirror-cmvs/src/vm/history.rs#L269) |

### vm/history.rs / History::cvns_command

```rust
fn cvns_command(
        &mut self,
        machine: &mut Machine,
        opcode: u16,
    ) -> Result<u16, DecodeError>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `opcode` | `0x2128` | self.entries.len | Handled | [92](../../crates/crysirror-cmvs/src/vm/history.rs#L92) |
| `opcode` | `0x2129..=0x212b` | usize::from, self.entries.get | Handled | [93](../../crates/crysirror-cmvs/src/vm/history.rs#L93) |
| `opcode` | `0x212c` | self.entries.is_empty, self.entries.truncate, self.entries.push_front | Handled | [101](../../crates/crysirror-cmvs/src/vm/history.rs#L101) |
| `opcode` | `0x212d` | Local state/value operations | Handled | [119](../../crates/crysirror-cmvs/src/vm/history.rs#L119) |
| `opcode` | `0x212e` | Local state/value operations | Handled | [123](../../crates/crysirror-cmvs/src/vm/history.rs#L123) |
| `opcode` | `0x212f` | self.entries.clear | Handled | [124](../../crates/crysirror-cmvs/src/vm/history.rs#L124) |
| `opcode` | `0x2136` | self.entries.get | Handled | [128](../../crates/crysirror-cmvs/src/vm/history.rs#L128) |
| `opcode` | `_` | Local state/value operations | Handled | [133](../../crates/crysirror-cmvs/src/vm/history.rs#L133) |

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
| `expr.kind` | `0x200` | self.numeric | Handled | [646](../../crates/crysirror-cmvs/src/vm/machine.rs#L646) |
| `expr.kind` | `0x201` | self.string_expression | Handled | [649](../../crates/crysirror-cmvs/src/vm/machine.rs#L649) |
| `expr.kind` | `0x202` | self.numeric | Handled | [650](../../crates/crysirror-cmvs/src/vm/machine.rs#L650) |
| `expr.kind` | `_` | Local state/value operations | Unsupported: expression type/operation not implemented | [653](../../crates/crysirror-cmvs/src/vm/machine.rs#L653) |

### vm/machine.rs / Machine::step

```rust
fn step(&mut self) -> Result<Step, DecodeError>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under &insn.kind=InstructionKind::ConditionalJump { target }` | `(op == 0x401 && !condition) \|\| (op == 0x402 && condition)` | self.branch_target | Handled | [722](../../crates/crysirror-cmvs/src/vm/machine.rs#L722) |
| `condition; under &insn.kind=InstructionKind::CompareJump { value, target }` | `self .state .result .ok_or_else(\|\| error(pc, op, "result register is unknown"))? == *value` | self.branch_target | Handled | [727](../../crates/crysirror-cmvs/src/vm/machine.rs#L727) |
| `condition; under &insn.kind=InstructionKind::Label { index }` | `op == 0x410` | self.enter_frame | Handled | [744](../../crates/crysirror-cmvs/src/vm/machine.rs#L744) |
| `condition; under &insn.kind=&InstructionKind::System { command } / self.profile()=VmProfile::Cvns20040515X86` | `command==0xe6 && self.cvns_arguments[1] as i32>1` | Local state/value operations | Handled | [815](../../crates/crysirror-cmvs/src/vm/machine.rs#L815) |
| `&insn.kind` | `InstructionKind::Expression(expr)` | self.expression | Handled | [713](../../crates/crysirror-cmvs/src/vm/machine.rs#L713) |
| `&insn.kind` | `InstructionKind::Jump { target } \| InstructionKind::LongJump { target, .. }` | self.branch_target | Handled | [714](../../crates/crysirror-cmvs/src/vm/machine.rs#L714) |
| `&insn.kind` | `InstructionKind::ConditionalJump { target }` | self.branch_target | Handled | [717](../../crates/crysirror-cmvs/src/vm/machine.rs#L717) |
| `&insn.kind` | `InstructionKind::CompareJump { value, target }` | self.branch_target | Handled | [726](../../crates/crysirror-cmvs/src/vm/machine.rs#L726) |
| `&insn.kind` | `InstructionKind::Label { index }` | self.enter_frame | Handled | [736](../../crates/crysirror-cmvs/src/vm/machine.rs#L736) |
| `&insn.kind` | `InstructionKind::StackAdjust { bytes } if op == 0x412` | Local state/value operations | Handled | [748](../../crates/crysirror-cmvs/src/vm/machine.rs#L748) |
| `&insn.kind` | `InstructionKind::StackAdjust { bytes } if op == 0x411` | self.return_from_frame | Handled | [753](../../crates/crysirror-cmvs/src/vm/machine.rs#L753) |
| `&insn.kind` | `InstructionKind::Simple if op == 0x413` | self.return_from_frame | Handled | [756](../../crates/crysirror-cmvs/src/vm/machine.rs#L756) |
| `&insn.kind` | `InstructionKind::Simple if op == 0x416` | self.branch_target, self.enter_frame | Handled | [763](../../crates/crysirror-cmvs/src/vm/machine.rs#L763) |
| `&insn.kind` | `InstructionKind::Simple if op == 0x430` | self.stack.push_u32 | Handled | [771](../../crates/crysirror-cmvs/src/vm/machine.rs#L771) |
| `&insn.kind` | `InstructionKind::StackImmediate { bytes, raw }` | usize::from | Handled | [778](../../crates/crysirror-cmvs/src/vm/machine.rs#L778) |
| `&insn.kind` | `InstructionKind::ScriptReturn { .. }` | self.stack.peek_u32, self.is_boundary, self.cross_calls.is_empty, self.cross_calls.last, self.stack.discard | Handled | [783](../../crates/crysirror-cmvs/src/vm/machine.rs#L783) |
| `&insn.kind` | `InstructionKind::System { command: 0 }` | Local state/value operations | Handled | [800](../../crates/crysirror-cmvs/src/vm/machine.rs#L800) |
| `&insn.kind` | `&InstructionKind::System { command }` | self.profile, super::cvns_scratch::ARGUMENT_WORDS.get, usize::from, self.stack.peek_u32, super::cvns::route | Handled | [805](../../crates/crysirror-cmvs/src/vm/machine.rs#L805) |
| `&insn.kind` | `_` | Local state/value operations | Unsupported: instruction execution is not implemented | [826](../../crates/crysirror-cmvs/src/vm/machine.rs#L826) |

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
| `condition; under term=Term::Operator { pc, opcode } if (0x160..=0x172).contains(opcode)` | `*opcode == 0x170` | self.append_operand, self.assign_string | Handled | [572](../../crates/crysirror-cmvs/src/vm/machine.rs#L572) |

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
| `condition` | `command == 0x160` | Local state/value operations | Handled | [460](../../crates/crysirror-cmvs/src/vm/startup.rs#L460) |

### vm/startup.rs / CmvsVm::fill_variables

```rust
fn fill_variables(&mut self, opcode: u16) -> Result<(), DecodeError>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `opcode` | `0x2091` | self.machine.state_mut | Handled | [1892](../../crates/crysirror-cmvs/src/vm/startup.rs#L1892) |
| `opcode` | `0x2092` | Local state/value operations | Handled | [1893](../../crates/crysirror-cmvs/src/vm/startup.rs#L1893) |
| `opcode` | `0x2093` | self.machine.state_mut | Handled | [1897](../../crates/crysirror-cmvs/src/vm/startup.rs#L1897) |
| `opcode` | `0x2094` | self.machine.resolve_string, self.machine.state_mut | Handled | [1898](../../crates/crysirror-cmvs/src/vm/startup.rs#L1898) |
| `opcode` | `_` | Local state/value operations | Handled | [1902](../../crates/crysirror-cmvs/src/vm/startup.rs#L1902) |

### vm/startup.rs / CmvsVm::image_name

```rust
fn image_name(&self, opcode: u16, reference: u32) -> Result<String, DecodeError>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `name.len() >= 64` | self.pc | Unsupported: CMVS image name byte limit | [829](../../crates/crysirror-cmvs/src/vm/startup.rs#L829) |

### vm/startup.rs / CmvsVm::owner_arguments

```rust
fn owner_arguments(
        &self,
        opcode: u16,
    ) -> Result<[u32; N], DecodeError>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `alias.is_none() && !matches!(self.machine.pending_system(), Some((_, op)) if op + 0x2000 == opcode)` | self.pc | Unsupported: CMVS owner command differs | [1083](../../crates/crysirror-cmvs/src/vm/startup.rs#L1083) |

### vm/startup.rs / CmvsVm::pending_effect_sound

```rust
fn pending_effect_sound(&self) -> Result<EffectSound, DecodeError>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under command=0xb0 \| 0xba \| 0x8c5` | `!sample && name.len() >= if cvns { 64 } else { 32 }` | Local state/value operations | Unsupported: CMVS effect sound name byte limit | [908](../../crates/crysirror-cmvs/src/vm/startup.rs#L908) |
| `condition; under command=0xb0 \| 0xba \| 0x8c5` | `command == 0xb0` | Local state/value operations | Handled | [918](../../crates/crysirror-cmvs/src/vm/startup.rs#L918) |
| `condition; under command=0xb0 \| 0xba \| 0x8c5` | `command == 0xb0` | Local state/value operations | Handled | [921](../../crates/crysirror-cmvs/src/vm/startup.rs#L921) |
| `condition; under command=0xb0 \| 0xba \| 0x8c5` | `command != 0xb0` | self.required_parameter | Handled | [923](../../crates/crysirror-cmvs/src/vm/startup.rs#L923) |
| `command` | `0xb0 \| 0xba \| 0x8c5` | self.required_parameter | Handled | [903](../../crates/crysirror-cmvs/src/vm/startup.rs#L903) |
| `command` | `0xb1` | self.required_parameter | Handled | [930](../../crates/crysirror-cmvs/src/vm/startup.rs#L930) |
| `command` | `_` | Local state/value operations | Handled | [934](../../crates/crysirror-cmvs/src/vm/startup.rs#L934) |

### vm/startup.rs / CmvsVm::pending_fade

```rust
fn pending_fade(&self) -> Result<FadeCommand, DecodeError>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `command` | `0x220` | Local state/value operations | Handled | [1383](../../crates/crysirror-cmvs/src/vm/startup.rs#L1383) |
| `command` | `0x221` | Local state/value operations | Handled | [1396](../../crates/crysirror-cmvs/src/vm/startup.rs#L1396) |
| `command` | `0x222` | Local state/value operations | Handled | [1397](../../crates/crysirror-cmvs/src/vm/startup.rs#L1397) |
| `command` | `0x223` | Local state/value operations | Handled | [1398](../../crates/crysirror-cmvs/src/vm/startup.rs#L1398) |
| `command` | `_` | Local state/value operations | Handled | [1399](../../crates/crysirror-cmvs/src/vm/startup.rs#L1399) |

### vm/startup.rs / CmvsVm::pending_scene_object

```rust
fn pending_scene_object(&self) -> Result<(SceneObject, u16), DecodeError>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `command == 0x2f` | Local state/value operations | Handled | [977](../../crates/crysirror-cmvs/src/vm/startup.rs#L977) |
| `condition` | `matches!(command, 0x22 \| 0x23 \| 0x44..=0x48 \| 0x50)` | Local state/value operations | Handled | [986](../../crates/crysirror-cmvs/src/vm/startup.rs#L986) |
| `condition` | `value < 0 && matches!(command, 0x44..=0x48)` | Local state/value operations | Handled | [988](../../crates/crysirror-cmvs/src/vm/startup.rs#L988) |
| `command; under command=_` | `0x44` | DrawProperty::Crop | Handled | [1026](../../crates/crysirror-cmvs/src/vm/startup.rs#L1026) |
| `command; under command=_` | `0x45` | DrawProperty::Position | Handled | [1035](../../crates/crysirror-cmvs/src/vm/startup.rs#L1035) |
| `command; under command=_` | `0x46` | DrawProperty::Origin | Handled | [1039](../../crates/crysirror-cmvs/src/vm/startup.rs#L1039) |
| `command; under command=_` | `0x47` | DrawProperty::Priority | Handled | [1040](../../crates/crysirror-cmvs/src/vm/startup.rs#L1040) |
| `command; under command=_` | `_` | DrawProperty::Opacity | Handled | [1041](../../crates/crysirror-cmvs/src/vm/startup.rs#L1041) |
| `command` | `0x20..=0x23` | Local state/value operations | Handled | [999](../../crates/crysirror-cmvs/src/vm/startup.rs#L999) |
| `command` | `0x30` | self.machine.resolve_string | Handled | [1007](../../crates/crysirror-cmvs/src/vm/startup.rs#L1007) |
| `command` | `0x40 \| 0x50` | Local state/value operations | Handled | [1016](../../crates/crysirror-cmvs/src/vm/startup.rs#L1016) |
| `command` | `_` | DrawProperty::Crop, DrawProperty::Position, DrawProperty::Origin, DrawProperty::Priority, DrawProperty::Opacity | Handled | [1024](../../crates/crysirror-cmvs/src/vm/startup.rs#L1024) |

### vm/startup.rs / CmvsVm::pending_script

```rust
fn pending_script(&self) -> Result<Option<ScriptRequest>, DecodeError>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `command == 0x812` | Local state/value operations | Handled | [497](../../crates/crysirror-cmvs/src/vm/startup.rs#L497) |
| `condition` | `command == 0x81` | self.required_parameter | Handled | [499](../../crates/crysirror-cmvs/src/vm/startup.rs#L499) |
| `condition` | `command == 0x03` | Local state/value operations | Handled | [508](../../crates/crysirror-cmvs/src/vm/startup.rs#L508) |
| `condition` | `command == 0x805` | Local state/value operations | Handled | [510](../../crates/crysirror-cmvs/src/vm/startup.rs#L510) |
| `condition` | `name.is_empty() \|\| name.len() >= if command >= 0x800 { 32 } else { 64 }` | Local state/value operations | Unsupported: script basename exceeds native bounds | [516](../../crates/crysirror-cmvs/src/vm/startup.rs#L516) |
| `condition` | `command >= 0x800` | Local state/value operations | Handled | [516](../../crates/crysirror-cmvs/src/vm/startup.rs#L516) |

### vm/startup.rs / CmvsVm::pending_title

```rust
fn pending_title(&self) -> Result<Option<String>, DecodeError>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `command == 0x161` | Local state/value operations | Handled | [432](../../crates/crysirror-cmvs/src/vm/startup.rs#L432) |

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
| `condition` | `only_read && id as i32 >= 0` | self.pc | Handled | [783](../../crates/crysirror-cmvs/src/vm/startup.rs#L783) |

### vm/startup.rs / CmvsVm::run_slice

```rust
fn run_slice(&mut self, budget: &mut u32) -> Result<VmYield, DecodeError>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under self.machine.step()?=Step::System { pc, command }` | `opcode == 0x208a` | self.control.invoke | Handled | [1574](../../crates/crysirror-cmvs/src/vm/startup.rs#L1574) |

### vm/startup.rs / CmvsVm::sound_name

```rust
fn sound_name(&self, raw: u32) -> Result<Option<String>, DecodeError>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `self.profile() == VmProfile::Cvns20040515X86 && opcode == 0x28b8` | Local state/value operations | Handled | [1182](../../crates/crysirror-cmvs/src/vm/startup.rs#L1182) |
| `condition` | `name.is_empty() \|\| name.len() >= 64` | Local state/value operations | Handled | [1183](../../crates/crysirror-cmvs/src/vm/startup.rs#L1183) |
| `condition` | `name.is_empty() \|\| (matches!(opcode, 0x2240 \| 0x2248) && name.len() >= 32)` | Local state/value operations | Handled | [1186](../../crates/crysirror-cmvs/src/vm/startup.rs#L1186) |
| `condition` | `matches!(opcode, 0x2240 \| 0x2248)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [1186](../../crates/crysirror-cmvs/src/vm/startup.rs#L1186) |
| `condition` | `name.len() >= 32` | self.pc | Unsupported: CMVS sound name byte limit | [1188](../../crates/crysirror-cmvs/src/vm/startup.rs#L1188) |

### vm/startup.rs / CmvsVm::startup_command

```rust
fn startup_command(&mut self, opcode: u16) -> Result<Option<SystemResult>, DecodeError>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `opcode; under opcode=0x201d..=0x201f` | `0x201d` | Local state/value operations | Handled | [1601](../../crates/crysirror-cmvs/src/vm/startup.rs#L1601) |
| `opcode; under opcode=0x201d..=0x201f` | `0x201e` | Local state/value operations | Handled | [1602](../../crates/crysirror-cmvs/src/vm/startup.rs#L1602) |
| `opcode; under opcode=0x201d..=0x201f` | `_` | Local state/value operations | Handled | [1603](../../crates/crysirror-cmvs/src/vm/startup.rs#L1603) |
| `opcode; under opcode=0x2268..=0x226e if self.machine.profile() == VmProfile::Pc20211007X64` | `0x2268` | Local state/value operations | Handled | [1634](../../crates/crysirror-cmvs/src/vm/startup.rs#L1634) |
| `opcode; under opcode=0x2268..=0x226e if self.machine.profile() == VmProfile::Pc20211007X64` | `0x2269` | Local state/value operations | Handled | [1635](../../crates/crysirror-cmvs/src/vm/startup.rs#L1635) |
| `opcode; under opcode=0x2268..=0x226e if self.machine.profile() == VmProfile::Pc20211007X64` | `0x226a` | Local state/value operations | Handled | [1636](../../crates/crysirror-cmvs/src/vm/startup.rs#L1636) |
| `opcode; under opcode=0x2268..=0x226e if self.machine.profile() == VmProfile::Pc20211007X64` | `0x226b` | Local state/value operations | Handled | [1637](../../crates/crysirror-cmvs/src/vm/startup.rs#L1637) |
| `opcode; under opcode=0x2268..=0x226e if self.machine.profile() == VmProfile::Pc20211007X64` | `0x226c` | Local state/value operations | Handled | [1638](../../crates/crysirror-cmvs/src/vm/startup.rs#L1638) |
| `opcode; under opcode=0x2268..=0x226e if self.machine.profile() == VmProfile::Pc20211007X64` | `0x226d` | Local state/value operations | Handled | [1639](../../crates/crysirror-cmvs/src/vm/startup.rs#L1639) |
| `opcode; under opcode=0x2268..=0x226e if self.machine.profile() == VmProfile::Pc20211007X64` | `_` | Local state/value operations | Handled | [1640](../../crates/crysirror-cmvs/src/vm/startup.rs#L1640) |
| `condition; under opcode=0x2010..=0x2014 \| 0x2017 \| 0x2018 \| 0x2019 \| 0x201a if opcode != 0x2019 \|\| self.machine.profile() == VmProfile::Pc20160201X64` | `opcode == 0x2011` | Local state/value operations | Handled | [1661](../../crates/crysirror-cmvs/src/vm/startup.rs#L1661) |
| `condition; under opcode=0x2098` | `name.len() >= 128 \|\| self.cursors.len() >= 1024` | Local state/value operations | Unsupported: cursor registration limit | [1688](../../crates/crysirror-cmvs/src/vm/startup.rs#L1688) |
| `condition; under opcode=0x2350..=0x235b \| 0x2360 \| 0x2362 \| 0x2364..=0x236b` | `opcode == 0x2350` | self.required_parameter | Handled | [1700](../../crates/crysirror-cmvs/src/vm/startup.rs#L1700) |
| `condition; under opcode=0x2281 \| 0x2285` | `opcode == 0x2285` | self.sequence.count | Handled | [1757](../../crates/crysirror-cmvs/src/vm/startup.rs#L1757) |
| `condition; under opcode=0x2286..=0x228b / opcode=0x2288 \| 0x228b` | `opcode == 0x2288` | Local state/value operations | Handled | [1788](../../crates/crysirror-cmvs/src/vm/startup.rs#L1788) |
| `condition; under opcode=0x2286..=0x228b / opcode=_` | `opcode == 0x2289` | Local state/value operations | Handled | [1794](../../crates/crysirror-cmvs/src/vm/startup.rs#L1794) |
| `opcode; under opcode=0x2286..=0x228b` | `0x2286` | self.sequence.push | Handled | [1775](../../crates/crysirror-cmvs/src/vm/startup.rs#L1775) |
| `opcode; under opcode=0x2286..=0x228b` | `0x2287` | self.sequence.push | Handled | [1780](../../crates/crysirror-cmvs/src/vm/startup.rs#L1780) |
| `opcode; under opcode=0x2286..=0x228b` | `0x2288 \| 0x228b` | self.sequence.push | Handled | [1785](../../crates/crysirror-cmvs/src/vm/startup.rs#L1785) |
| `opcode; under opcode=0x2286..=0x228b` | `_` | self.sequence.push | Handled | [1792](../../crates/crysirror-cmvs/src/vm/startup.rs#L1792) |
| `condition; under opcode=0x22e5 \| 0x22eb` | `opcode == 0x22e5` | Local state/value operations | Handled | [1823](../../crates/crysirror-cmvs/src/vm/startup.rs#L1823) |
| `condition; under opcode=0x22e5 \| 0x22eb` | `opcode == 0x22e5` | Local state/value operations | Handled | [1829](../../crates/crysirror-cmvs/src/vm/startup.rs#L1829) |
| `condition; under opcode=0x22ce \| 0x22ee` | `opcode == 0x22ce` | Local state/value operations | Handled | [1843](../../crates/crysirror-cmvs/src/vm/startup.rs#L1843) |
| `condition; under opcode=_` | `(revision && (0x23d0..=0x23df).contains(&opcode)) \|\| (super::noop::returns_without_effect(opcode) && !(revision && super::noop::implemented_in_20211007(opcode))) \|\| self .actions .command(&mut self.machine, opcode, self.skipping)?` | Local state/value operations | Handled | [1856](../../crates/crysirror-cmvs/src/vm/startup.rs#L1856) |
| `opcode` | `0x23ac if self.machine.profile() == VmProfile::Pc20211007X64` | self.required_parameter | Handled | [1616](../../crates/crysirror-cmvs/src/vm/startup.rs#L1616) |
| `opcode` | `0x2019 \| 0x209c if self.machine.profile() == VmProfile::Pc20211007X64` | Local state/value operations | Handled | [1621](../../crates/crysirror-cmvs/src/vm/startup.rs#L1621) |
| `opcode` | `0x23ae if self.machine.profile() == VmProfile::Pc20211007X64` | Local state/value operations | Handled | [1622](../../crates/crysirror-cmvs/src/vm/startup.rs#L1622) |
| `opcode` | `0x2268..=0x226e if self.machine.profile() == VmProfile::Pc20211007X64` | self.required_parameter, self.pc, self.machine.resolve_string | Handled | [1623](../../crates/crysirror-cmvs/src/vm/startup.rs#L1623) |
| `opcode` | `0x2010..=0x2014 \| 0x2017 \| 0x2018 \| 0x2019 \| 0x201a if opcode != 0x2019 \|\| self.machine.profile() == VmProfile::Pc20160201X64` | self.required_parameter, self.pc, self.machine.resolve_string | Handled | [1655](../../crates/crysirror-cmvs/src/vm/startup.rs#L1655) |
| `opcode` | `_` | self.machine.profile, super::noop::returns_without_effect, super::noop::implemented_in_20211007, self.control.command, self.screen.command | Handled | [1854](../../crates/crysirror-cmvs/src/vm/startup.rs#L1854) |

### vm/startup/cvns.rs / CmvsVm::cvns_command

```rust
fn cvns_command(&mut self, opcode: u16) -> Result<Option<SystemResult>, DecodeError>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under opcode=0x21ca` | `name.len() < 64` | self.native.record | Handled | [14](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L14) |
| `condition; under opcode=0x20a5` | `number < 64 && name.len() < 64` | self.cvns.cd_tracks.insert | Handled | [25](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L25) |
| `condition; under opcode=0x20e7 \| 0x20e8 \| 0x20e9` | `opcode != 0x20e8` | f64::from | Handled | [45](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L45) |
| `condition; under opcode=0x20e7 \| 0x20e8 \| 0x20e9` | `opcode == 0x20e7` | f64::from | Handled | [47](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L47) |
| `condition; under opcode=0x20ea / mode=1 \| 2` | `mode == 1` | Local state/value operations | Handled | [94](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L94) |
| `mode; under opcode=0x20ea` | `0` | Local state/value operations | Handled | [82](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L82) |
| `mode; under opcode=0x20ea` | `1 \| 2` | f64::from | Handled | [83](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L83) |
| `mode; under opcode=0x20ea` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [96](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L96) |
| `opcode; under opcode=0x2108..=0x210a` | `0x210a` | self.machine.resolve_string | Handled | [125](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L125) |
| `opcode; under opcode=0x2108..=0x210a` | `_` | Vec::new | Handled | [126](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L126) |
| `opcode; under opcode=0x2108..=0x210a` | `0x2108` | Local state/value operations | Handled | [130](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L130) |
| `opcode; under opcode=0x2108..=0x210a` | `0x2109` | Local state/value operations | Handled | [131](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L131) |
| `opcode; under opcode=0x2108..=0x210a` | `_` | Local state/value operations | Handled | [132](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L132) |
| `opcode` | `0x21ca` | self.machine.resolve_string, self.native.record | Handled | [12](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L12) |
| `opcode` | `0x2028 \| 0x2029 \| 0x202a \| 0x202c` | self.control.cvns_command | Handled | [19](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L19) |
| `opcode` | `0x20a5` | self.machine.resolve_string, self.cvns.cd_tracks.insert, self.cvns.last_cd_track.max | Handled | [22](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L22) |
| `opcode` | `0x20a6` | Local state/value operations | Handled | [31](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L31) |
| `opcode` | `0x21c4` | Local state/value operations | Handled | [35](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L35) |
| `opcode` | `0x206b` | Local state/value operations | Handled | [39](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L39) |
| `opcode` | `0x20e7 \| 0x20e8 \| 0x20e9` | f64::from, self.machine.state_mut | Handled | [44](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L44) |
| `opcode` | `0x20f0..=0x20fd` | crate::vm::strings::command | Handled | [76](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L76) |
| `opcode` | `0x20ea` | f64::from, self.machine.state_mut | Handled | [77](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L77) |
| `opcode` | `0x2069` | u32::from | Handled | [104](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L104) |
| `opcode` | `0x2128..=0x212f \| 0x2136` | self.history.cvns_command | Handled | [112](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L112) |
| `opcode` | `0x210e` | Local state/value operations | Handled | [113](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L113) |
| `opcode` | `0x2108..=0x210a` | self.machine.resolve_string, Vec::new, self.machine.state_mut | Handled | [118](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L118) |
| `opcode` | `0x2020..=0x2024` | self.machine.resolve_string | Handled | [140](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L140) |
| `opcode` | `0x2025 \| 0x2026` | self.machine.resolve_string | Handled | [152](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L152) |
| `opcode` | `0x210b..=0x210d` | usize::from | Handled | [159](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L159) |
| `opcode` | `0x2014` | Local state/value operations | Handled | [163](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L163) |
| `opcode` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [168](../../crates/crysirror-cmvs/src/vm/startup/cvns.rs#L168) |

### vm/strings.rs / command

```rust
fn command(vm: &mut Machine, opcode: u16) -> Result<u16, DecodeError>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `opcode` | `0x20f0` | Local state/value operations | Handled | [15](../../crates/crysirror-cmvs/src/vm/strings.rs#L15) |
| `opcode` | `0x20f1` | Local state/value operations | Handled | [16](../../crates/crysirror-cmvs/src/vm/strings.rs#L16) |
| `opcode` | `0x20f8..=0x20fc` | Local state/value operations | Handled | [17](../../crates/crysirror-cmvs/src/vm/strings.rs#L17) |
| `opcode` | `0x20fd` | Local state/value operations | Handled | [18](../../crates/crysirror-cmvs/src/vm/strings.rs#L18) |
| `opcode` | `_` | Local state/value operations | Handled | [19](../../crates/crysirror-cmvs/src/vm/strings.rs#L19) |
| `condition; under operation=0x20f0 \| 0x20f2` | `operation == 0x20f2` | Local state/value operations | Handled | [31](../../crates/crysirror-cmvs/src/vm/strings.rs#L31) |
| `condition; under operation=0x20f7 \| 0x20f9` | `operation == 0x20f7` | Local state/value operations | Handled | [111](../../crates/crysirror-cmvs/src/vm/strings.rs#L111) |
| `operation; under operation=0x20fa \| 0x20fc \| 0x20fd` | `0x20fa` | FormatArg::Integer | Handled | [122](../../crates/crysirror-cmvs/src/vm/strings.rs#L122) |
| `operation; under operation=0x20fa \| 0x20fc \| 0x20fd` | `0x20fc` | FormatArg::Integer | Handled | [123](../../crates/crysirror-cmvs/src/vm/strings.rs#L123) |
| `operation; under operation=0x20fa \| 0x20fc \| 0x20fd` | `_` | FormatArg::String | Handled | [124](../../crates/crysirror-cmvs/src/vm/strings.rs#L124) |
| `condition; under operation=0x20fa \| 0x20fc \| 0x20fd` | `operation == 0x20fa` | Local state/value operations | Handled | [126](../../crates/crysirror-cmvs/src/vm/strings.rs#L126) |
| `condition; under operation=0x20fa \| 0x20fc \| 0x20fd` | `(operation != 0x20fa && overlaps(target, 0, output.len() + 1, arg(8)?, 0, pattern.len() + 1)) \|\| matches!(value, FormatArg::String(source) if overlaps(target, 0, output.len() + 1, arg(12)?, 0, source.len() + 1))` | Local state/value operations | Unsupported: overlapping CMVS format operands | [132](../../crates/crysirror-cmvs/src/vm/strings.rs#L132) |
| `condition; under operation=0x20fa \| 0x20fc \| 0x20fd` | `operation == 0x20fa` | Local state/value operations | Handled | [140](../../crates/crysirror-cmvs/src/vm/strings.rs#L140) |
| `operation` | `0x20f0 \| 0x20f2` | Local state/value operations | Handled | [27](../../crates/crysirror-cmvs/src/vm/strings.rs#L27) |
| `operation` | `0x20f1` | u32::from | Handled | [44](../../crates/crysirror-cmvs/src/vm/strings.rs#L44) |
| `operation` | `0x20f3` | usize::try_from | Handled | [48](../../crates/crysirror-cmvs/src/vm/strings.rs#L48) |
| `operation` | `0x20f4` | usize::from | Handled | [87](../../crates/crysirror-cmvs/src/vm/strings.rs#L87) |
| `operation` | `0x20f5` | u32::from | Handled | [102](../../crates/crysirror-cmvs/src/vm/strings.rs#L102) |
| `operation` | `0x20f6` | Local state/value operations | Handled | [106](../../crates/crysirror-cmvs/src/vm/strings.rs#L106) |
| `operation` | `0x20f7 \| 0x20f9` | Local state/value operations | Handled | [107](../../crates/crysirror-cmvs/src/vm/strings.rs#L107) |
| `operation` | `0x20f8` | Local state/value operations | Handled | [118](../../crates/crysirror-cmvs/src/vm/strings.rs#L118) |
| `operation` | `0x20fa \| 0x20fc \| 0x20fd` | FormatArg::Integer, FormatArg::String | Handled | [119](../../crates/crysirror-cmvs/src/vm/strings.rs#L119) |
| `operation` | `0x20fb` | i32::from | Handled | [142](../../crates/crysirror-cmvs/src/vm/strings.rs#L142) |
| `operation` | `0x20ff` | Local state/value operations | Handled | [160](../../crates/crysirror-cmvs/src/vm/strings.rs#L160) |
| `operation` | `_` | Local state/value operations | Handled | [176](../../crates/crysirror-cmvs/src/vm/strings.rs#L176) |

### vm/strings.rs / decimal

```rust
fn decimal(kind: u8, value: u32, sign: Option<u8>, width: usize) -> Result<Vec<u8>, &'static str>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `kind == b'd' && sign.is_some() && signed < 0` | Local state/value operations | Handled | [495](../../crates/crysirror-cmvs/src/vm/strings.rs#L495) |

### vm/strings.rs / sprintf

```rust
fn sprintf(
    pattern: &[u8],
    argument: FormatArg<'_>,
    single_x86_word: bool,
    read_destination: impl Fn(usize) -> Result<u8, &'static str>,
) -> Result<Vec<u8>, &'static str>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(kind, b'd' \| b'u' \| b'z' \| b'x' \| b'o' \| b's' \| b'f' \| b't')` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [298](../../crates/crysirror-cmvs/src/vm/strings.rs#L298) |
| `condition` | `matches!(kind, b'f' \| b't')` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [302](../../crates/crysirror-cmvs/src/vm/strings.rs#L302) |
| `condition` | `matches!(kind, b's' \| b'c')` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [313](../../crates/crysirror-cmvs/src/vm/strings.rs#L313) |
