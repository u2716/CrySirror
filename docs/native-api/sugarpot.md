# SugarPot / Tsukumo-2012 developer reference

[All engines](README.md)

Mode: `adopt`.

## ABI and host contract

The admitted language is the observed 2012 *Tsukumo no Tsuki* profile, canvas
1024×600, entry `start.scr`. AOS2 resource-family recognition does not establish
that another LiLiM/SugarPot title uses this VM. Commands, `^` control selectors,
value operators, macro names and UD effect kinds are separate namespaces.
Sources retain CP932 positions, section/local labels and macro bodies.

| Native area | CrySirror service / implementation | Return / completion |
| --- | --- | --- |
| Calls, macros, values and control | `machine.rs` / `values.rs` | Native registers/lists and stack; unknown builtins fail after macro lookup. |
| Sprites, backgrounds and UD 1..39 | Image jobs, scene capture, Layer/Canvas/VFX | Staged state publishes through native UD timing; pending jobs retain the call. |
| Text, choices, recollection and galleries | Shared glyph jobs and retained UI | Native result IDs/modal lifetime, 33ms paint policy and read state. |
| BGM / ENV / SE / voice / movie | Shared Audio/Media | Banks remain distinct; native channel and movie flags still apply. |
| Resources and native preferences | Scoped workers / Storage | ABM/BMP/DAT precedence; atomic global writes must complete before acknowledgement. |

UD39 obtains its disc tables from the identified installed executable as data;
no proprietary table is distributed. UD2 selects the documented scalar
stored-mask policy; MMX byte parity is not claimed. Native named dummy commands
remain explicit no-ops, not a catchall for unknown commands. Portable max/min
bounds and fullwidth float formatting are intentional fixes to native memory
errors. Original slot-menu reconstruction/thumbnail handling, settings/BGM album,
remaining EF/raster/filter families and some text/movie/gauge profiles have
explicit gaps; slot codec availability alone is not complete save support.

Source/provenance: [crate](../../crates/crysirror-sugarpot/),
[UD execution](../../crates/crysirror-sugarpot/src/adapter/update.rs),
[value/control ABI](../../crates/crysirror-sugarpot/src/machine.rs).
Behavior: [focused validation](../validation/sugarpot-2026-11-07.md).

<!-- BEGIN GENERATED DISPATCH TABLES -->

## Opcode and API implementation tables

`Handled` means an executable source branch exists. It is not a claim that every argument/subselector or a complete game has been audited. Explicit gaps are shown per row.

| Status | Entries |
| --- | ---: |
| Empty handler | 1 |
| Handled | 180 |
| Original no-op | 2 |
| Partial | 1 |

These tables cover the checked-in catalogues and explicit dispatch branches. An engine-wide upstream denominator is not inferred from these rows.

## Command selectors / adapter.rs / selector

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x01` | Start slot menu | Handled | Guard: self.save_enabled | [adapter.rs:2972](../../crates/crysirror-sugarpot/src/adapter.rs#L2972) |
| `0x02` | Start slot menu | Handled | Guard: self.load_enabled | [adapter.rs:2973](../../crates/crysirror-sugarpot/src/adapter.rs#L2973) |
| `0x03` | / disabled native gates return0 | Handled | — | [adapter.rs:2975](../../crates/crysirror-sugarpot/src/adapter.rs#L2975) |
| `0x04` | Recollection | Handled | — | [adapter.rs:2976](../../crates/crysirror-sugarpot/src/adapter.rs#L2976) |
| `0x05` | Bgm mode | Handled | — | [adapter.rs:2977](../../crates/crysirror-sugarpot/src/adapter.rs#L2977) |
| `0x06` | End mode | Handled | — | [adapter.rs:2978](../../crates/crysirror-sugarpot/src/adapter.rs#L2978) |
| `0x07` | Start native save | Handled | — | [adapter.rs:2979](../../crates/crysirror-sugarpot/src/adapter.rs#L2979) |
| `0x08` | Start native load | Handled | — | [adapter.rs:2987](../../crates/crysirror-sugarpot/src/adapter.rs#L2987) |
| `0x09` | Settings menu | Handled | — | [adapter.rs:2995](../../crates/crysirror-sugarpot/src/adapter.rs#L2995) |
| `0x0B` | String | Handled | — | [adapter.rs:2996](../../crates/crysirror-sugarpot/src/adapter.rs#L2996) |

## Source control prefixes

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `!` | Native case 0x21; advance source line | Original no-op | — | [machine.rs:375](../../crates/crysirror-sugarpot/src/machine.rs#L375) |

## Tsukumo no Tsuki script commands

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `df` | df | Handled | — | [machine.rs:479](../../crates/crysirror-sugarpot/src/machine.rs#L479) |
| `ef` | ef | Handled | — | [adapter.rs:3052](../../crates/crysirror-sugarpot/src/adapter.rs#L3052) |
| `def` | def | Handled | — | [values.rs:14](../../crates/crysirror-sugarpot/src/values.rs#L14) |
| `anm` | anm | Handled | — | [values.rs:218](../../crates/crysirror-sugarpot/src/values.rs#L218) |
| `anmdisable` | anmdisable | Handled | — | [values.rs:218](../../crates/crysirror-sugarpot/src/values.rs#L218) |
| `anmenable` | anmenable | Handled | — | [values.rs:218](../../crates/crysirror-sugarpot/src/values.rs#L218) |
| `anmnum` | anmnum | Handled | — | [adapter.rs:3030](../../crates/crysirror-sugarpot/src/adapter.rs#L3030) |
| `anmoff` | anmoff | Handled | — | [values.rs:218](../../crates/crysirror-sugarpot/src/values.rs#L218) |
| `anmon` | anmon | Handled | — | [values.rs:218](../../crates/crysirror-sugarpot/src/values.rs#L218) |
| `array` | array | Handled | — | [values.rs:29](../../crates/crysirror-sugarpot/src/values.rs#L29) |
| `avi` | avi | Handled | — | [adapter.rs:3071](../../crates/crysirror-sugarpot/src/adapter.rs#L3071) |
| `backtitle` | backtitle | Handled | — | [adapter.rs:2883](../../crates/crysirror-sugarpot/src/adapter.rs#L2883) |
| `bg` | bg | Handled | — | [adapter.rs:2577](../../crates/crysirror-sugarpot/src/adapter.rs#L2577) |
| `bgm` | bgm | Handled | — | [values.rs:218](../../crates/crysirror-sugarpot/src/values.rs#L218) |
| `bgmoff` | bgmoff | Handled | — | [adapter.rs:2789](../../crates/crysirror-sugarpot/src/adapter.rs#L2789) |
| `bgmon` | bgmon | Handled | — | [adapter.rs:2710](../../crates/crysirror-sugarpot/src/adapter.rs#L2710) |
| `bgoff` | bgoff | Handled | — | [adapter.rs:2578](../../crates/crysirror-sugarpot/src/adapter.rs#L2578) |
| `bgon` | bgon | Handled | — | [adapter.rs:2578](../../crates/crysirror-sugarpot/src/adapter.rs#L2578) |
| `bin` | bin | Handled | — | [adapter.rs:2578](../../crates/crysirror-sugarpot/src/adapter.rs#L2578) |
| `bmp` | bmp | Handled | — | [adapter.rs:2908](../../crates/crysirror-sugarpot/src/adapter.rs#L2908) |
| `bout` | bout | Handled | — | [adapter.rs:2578](../../crates/crysirror-sugarpot/src/adapter.rs#L2578) |
| `brk` | brk | Handled | — | [machine.rs:508](../../crates/crysirror-sugarpot/src/machine.rs#L508) |
| `btn` | btn | Handled | — | [adapter.rs:2577](../../crates/crysirror-sugarpot/src/adapter.rs#L2577) |
| `btndisable` | btndisable | Handled | — | [adapter.rs:2652](../../crates/crysirror-sugarpot/src/adapter.rs#L2652) |
| `btnenable` | btnenable | Handled | — | [adapter.rs:2635](../../crates/crysirror-sugarpot/src/adapter.rs#L2635) |
| `btnoff` | btnoff | Handled | — | [adapter.rs:2578](../../crates/crysirror-sugarpot/src/adapter.rs#L2578) |
| `btnon` | btnon | Handled | — | [adapter.rs:2578](../../crates/crysirror-sugarpot/src/adapter.rs#L2578) |
| `btnreset` | btnreset | Handled | — | [adapter.rs:2578](../../crates/crysirror-sugarpot/src/adapter.rs#L2578) |
| `btnset` | btnset | Handled | — | [adapter.rs:2578](../../crates/crysirror-sugarpot/src/adapter.rs#L2578) |
| `call` | Call a function | Handled | — | [values.rs:218](../../crates/crysirror-sugarpot/src/values.rs#L218) |
| `case` | case | Handled | — | [machine.rs:479](../../crates/crysirror-sugarpot/src/machine.rs#L479) |
| `cgoff` | cgoff | Handled | — | [adapter.rs:2578](../../crates/crysirror-sugarpot/src/adapter.rs#L2578) |
| `cgon` | cgon | Handled | — | [adapter.rs:2578](../../crates/crysirror-sugarpot/src/adapter.rs#L2578) |
| `ch` | ch | Handled | — | [adapter.rs:2577](../../crates/crysirror-sugarpot/src/adapter.rs#L2577) |
| `choff` | choff | Handled | — | [adapter.rs:2578](../../crates/crysirror-sugarpot/src/adapter.rs#L2578) |
| `chon` | chon | Handled | — | [adapter.rs:2578](../../crates/crysirror-sugarpot/src/adapter.rs#L2578) |
| `clrbacklog` | clrbacklog | Handled | — | [adapter.rs:3113](../../crates/crysirror-sugarpot/src/adapter.rs#L3113) |
| `cls` | cls | Handled | — | [adapter.rs:2842](../../crates/crysirror-sugarpot/src/adapter.rs#L2842) |
| `cont` | cont | Handled | — | [machine.rs:508](../../crates/crysirror-sugarpot/src/machine.rs#L508) |
| `cv` | cv | Handled | — | [adapter.rs:2675](../../crates/crysirror-sugarpot/src/adapter.rs#L2675) |
| `cvoff` | cvoff | Handled | — | [adapter.rs:2776](../../crates/crysirror-sugarpot/src/adapter.rs#L2776) |
| `cvon` | cvon | Handled | — | [adapter.rs:2712](../../crates/crysirror-sugarpot/src/adapter.rs#L2712) |
| `debug` | debug | Handled | — | [adapter.rs:2943](../../crates/crysirror-sugarpot/src/adapter.rs#L2943) |
| `delp` | delp | Handled | — | [values.rs:218](../../crates/crysirror-sugarpot/src/values.rs#L218) |
| `deque` | deque | Handled | — | [values.rs:85](../../crates/crysirror-sugarpot/src/values.rs#L85) |
| `dummy` | Native named dummy command | Original no-op | Explicit named slot; not an unknown-command fallback | [values.rs:10](../../crates/crysirror-sugarpot/src/values.rs#L10) |
| `efex` | efex | Handled | — | [adapter.rs:3060](../../crates/crysirror-sugarpot/src/adapter.rs#L3060) |
| `elif` | elif | Handled | — | [machine.rs:445](../../crates/crysirror-sugarpot/src/machine.rs#L445) |
| `else` | else | Handled | — | [machine.rs:445](../../crates/crysirror-sugarpot/src/machine.rs#L445) |
| `endi` | endi | Handled | — | [machine.rs:451](../../crates/crysirror-sugarpot/src/machine.rs#L451) |
| `ends` | ends | Handled | — | [machine.rs:451](../../crates/crysirror-sugarpot/src/machine.rs#L451) |
| `endw` | endw | Handled | — | [machine.rs:508](../../crates/crysirror-sugarpot/src/machine.rs#L508) |
| `enque` | enque | Handled | — | [values.rs:50](../../crates/crysirror-sugarpot/src/values.rs#L50) |
| `env` | env | Handled | — | [values.rs:218](../../crates/crysirror-sugarpot/src/values.rs#L218) |
| `envoff` | envoff | Handled | — | [adapter.rs:2790](../../crates/crysirror-sugarpot/src/adapter.rs#L2790) |
| `envon` | envon | Handled | — | [adapter.rs:2711](../../crates/crysirror-sugarpot/src/adapter.rs#L2711) |
| `exit` | exit | Handled | — | [adapter.rs:3121](../../crates/crysirror-sugarpot/src/adapter.rs#L3121) |
| `fall` | fall | Handled | — | [adapter.rs:3108](../../crates/crysirror-sugarpot/src/adapter.rs#L3108) |
| `fill` | fill | Handled | — | [adapter.rs:2828](../../crates/crysirror-sugarpot/src/adapter.rs#L2828) |
| `filter` | filter | Handled | — | [adapter.rs:3107](../../crates/crysirror-sugarpot/src/adapter.rs#L3107) |
| `float` | float | Handled | — | [adapter.rs:2969](../../crates/crysirror-sugarpot/src/adapter.rs#L2969) |
| `fnt` | fnt | Handled | — | [adapter.rs:2593](../../crates/crysirror-sugarpot/src/adapter.rs#L2593) |
| `gg` | gg | Handled | — | [adapter.rs:3051](../../crates/crysirror-sugarpot/src/adapter.rs#L3051) |
| `ggoff` | ggoff | Handled | — | [adapter.rs:3051](../../crates/crysirror-sugarpot/src/adapter.rs#L3051) |
| `ggon` | ggon | Handled | — | [adapter.rs:3051](../../crates/crysirror-sugarpot/src/adapter.rs#L3051) |
| `global` | global | Handled | — | [adapter.rs:3110](../../crates/crysirror-sugarpot/src/adapter.rs#L3110) |
| `go` | go | Handled | — | [machine.rs:387](../../crates/crysirror-sugarpot/src/machine.rs#L387) |
| `gsb` | gsb | Handled | — | [machine.rs:387](../../crates/crysirror-sugarpot/src/machine.rs#L387) |
| `if` | if | Handled | — | [machine.rs:412](../../crates/crysirror-sugarpot/src/machine.rs#L412) |
| `insp` | insp | Handled | — | [values.rs:218](../../crates/crysirror-sugarpot/src/values.rs#L218) |
| `jp` | jp | Handled | — | [machine.rs:387](../../crates/crysirror-sugarpot/src/machine.rs#L387) |
| `jsb` | jsb | Handled | — | [machine.rs:387](../../crates/crysirror-sugarpot/src/machine.rs#L387) |
| `layer` | layer | Handled | — | [values.rs:218](../../crates/crysirror-sugarpot/src/values.rs#L218) |
| `load` | load | Handled | — | [values.rs:218](../../crates/crysirror-sugarpot/src/values.rs#L218) |
| `loaddisable` | loaddisable | Handled | — | [adapter.rs:2622](../../crates/crysirror-sugarpot/src/adapter.rs#L2622) |
| `loadenable` | loadenable | Handled | — | [adapter.rs:2617](../../crates/crysirror-sugarpot/src/adapter.rs#L2617) |
| `max` | max | Handled | — | [values.rs:114](../../crates/crysirror-sugarpot/src/values.rs#L114) |
| `memo` | memo | Handled | — | [adapter.rs:2588](../../crates/crysirror-sugarpot/src/adapter.rs#L2588) |
| `menu` | menu | Handled | — | [values.rs:218](../../crates/crysirror-sugarpot/src/values.rs#L218) |
| `min` | min | Handled | — | [values.rs:114](../../crates/crysirror-sugarpot/src/values.rs#L114) |
| `mode` | mode | Handled | — | [adapter.rs:2602](../../crates/crysirror-sugarpot/src/adapter.rs#L2602) |
| `mono` | mono | Handled | — | [adapter.rs:3109](../../crates/crysirror-sugarpot/src/adapter.rs#L3109) |
| `move` | move | Handled | — | [values.rs:218](../../crates/crysirror-sugarpot/src/values.rs#L218) |
| `movie` | movie | Handled | — | [adapter.rs:3071](../../crates/crysirror-sugarpot/src/adapter.rs#L3071) |
| `msg` | msg | Handled | — | [values.rs:218](../../crates/crysirror-sugarpot/src/values.rs#L218) |
| `nop` | No-operation instruction | Empty handler | Original no-op status not established | [adapter.rs:3127](../../crates/crysirror-sugarpot/src/adapter.rs#L3127) |
| `num` | num | Handled | — | [values.rs:148](../../crates/crysirror-sugarpot/src/values.rs#L148) |
| `ofs` | ofs | Handled | — | [values.rs:218](../../crates/crysirror-sugarpot/src/values.rs#L218) |
| `pause` | Wait for input | Handled | — | [adapter.rs:2634](../../crates/crysirror-sugarpot/src/adapter.rs#L2634) |
| `pick` | pick | Handled | — | [values.rs:218](../../crates/crysirror-sugarpot/src/values.rs#L218) |
| `pool` | only parses arguments; no array is allocated | Handled | — | [values.rs:34](../../crates/crysirror-sugarpot/src/values.rs#L34) |
| `pop` | pop | Handled | — | [values.rs:85](../../crates/crysirror-sugarpot/src/values.rs#L85) |
| `print` | print | Handled | — | [values.rs:218](../../crates/crysirror-sugarpot/src/values.rs#L218) |
| `push` | push | Handled | — | [values.rs:50](../../crates/crysirror-sugarpot/src/values.rs#L50) |
| `que` | only parses arguments; no array is allocated | Handled | — | [values.rs:34](../../crates/crysirror-sugarpot/src/values.rs#L34) |
| `rand` | rand | Handled | — | [adapter.rs:2662](../../crates/crysirror-sugarpot/src/adapter.rs#L2662) |
| `raster` | raster | Handled | — | [adapter.rs:3106](../../crates/crysirror-sugarpot/src/adapter.rs#L3106) |
| `rbtn` | rbtn | Handled | — | [adapter.rs:3112](../../crates/crysirror-sugarpot/src/adapter.rs#L3112) |
| `readal` | readal | Handled | — | [adapter.rs:3111](../../crates/crysirror-sugarpot/src/adapter.rs#L3111) |
| `ret` | Return from a call | Handled | — | [machine.rs:396](../../crates/crysirror-sugarpot/src/machine.rs#L396) |
| `save` | save | Handled | — | [values.rs:218](../../crates/crysirror-sugarpot/src/values.rs#L218) |
| `savedisable` | savedisable | Handled | — | [adapter.rs:2612](../../crates/crysirror-sugarpot/src/adapter.rs#L2612) |
| `saveenable` | saveenable | Handled | — | [adapter.rs:2607](../../crates/crysirror-sugarpot/src/adapter.rs#L2607) |
| `scroll` | scroll | Handled | — | [values.rs:218](../../crates/crysirror-sugarpot/src/values.rs#L218) |
| `se` | se | Handled | — | [values.rs:218](../../crates/crysirror-sugarpot/src/values.rs#L218) |
| `seoff` | seoff | Handled | — | [adapter.rs:2776](../../crates/crysirror-sugarpot/src/adapter.rs#L2776) |
| `seon` | seon | Handled | — | [adapter.rs:2686](../../crates/crysirror-sugarpot/src/adapter.rs#L2686) |
| `set` | set | Handled | — | [values.rs:218](../../crates/crysirror-sugarpot/src/values.rs#L218) |
| `skipdisable` | skipdisable | Handled | — | [adapter.rs:2628](../../crates/crysirror-sugarpot/src/adapter.rs#L2628) |
| `skipenable` | skipenable | Handled | — | [adapter.rs:2627](../../crates/crysirror-sugarpot/src/adapter.rs#L2627) |
| `slct` | slct | Handled | — | [values.rs:218](../../crates/crysirror-sugarpot/src/values.rs#L218) |
| `snap` | snap | Handled | — | [values.rs:218](../../crates/crysirror-sugarpot/src/values.rs#L218) |
| `spr` | spr | Handled | — | [adapter.rs:2577](../../crates/crysirror-sugarpot/src/adapter.rs#L2577) |
| `ss` | ss | Handled | — | [values.rs:218](../../crates/crysirror-sugarpot/src/values.rs#L218) |
| `ssoff` | ssoff | Handled | — | [values.rs:218](../../crates/crysirror-sugarpot/src/values.rs#L218) |
| `sson` | sson | Handled | — | [values.rs:218](../../crates/crysirror-sugarpot/src/values.rs#L218) |
| `ssreset` | ssreset | Handled | — | [values.rs:218](../../crates/crysirror-sugarpot/src/values.rs#L218) |
| `ssset` | ssset | Handled | — | [values.rs:218](../../crates/crysirror-sugarpot/src/values.rs#L218) |
| `stack` | only parses arguments; no array is allocated | Handled | — | [values.rs:34](../../crates/crysirror-sugarpot/src/values.rs#L34) |
| `str` | str | Handled | — | [values.rs:191](../../crates/crysirror-sugarpot/src/values.rs#L191) |
| `sw` | sw | Handled | — | [machine.rs:459](../../crates/crysirror-sugarpot/src/machine.rs#L459) |
| `swap` | swap | Handled | — | [values.rs:218](../../crates/crysirror-sugarpot/src/values.rs#L218) |
| `title` | title | Handled | — | [adapter.rs:2582](../../crates/crysirror-sugarpot/src/adapter.rs#L2582) |
| `ud` | ud | Handled | — | [adapter.rs:2578](../../crates/crysirror-sugarpot/src/adapter.rs#L2578) |
| `udwait` | udwait | Handled | — | [values.rs:218](../../crates/crysirror-sugarpot/src/values.rs#L218) |
| `var` | var | Handled | — | [adapter.rs:2576](../../crates/crysirror-sugarpot/src/adapter.rs#L2576) |
| `wait` | wait | Handled | — | [adapter.rs:2629](../../crates/crysirror-sugarpot/src/adapter.rs#L2629) |
| `wbl` | wbl | Handled | — | [adapter.rs:2577](../../crates/crysirror-sugarpot/src/adapter.rs#L2577) |
| `wh` | wh | Handled | — | [machine.rs:485](../../crates/crysirror-sugarpot/src/machine.rs#L485) |
| `win` | win | Handled | — | [adapter.rs:2578](../../crates/crysirror-sugarpot/src/adapter.rs#L2578) |
| `wnd` | wnd | Handled | — | [adapter.rs:2577](../../crates/crysirror-sugarpot/src/adapter.rs#L2577) |
| `wndoff` | wndoff | Handled | — | [adapter.rs:2578](../../crates/crysirror-sugarpot/src/adapter.rs#L2578) |
| `wndon` | wndon | Handled | — | [adapter.rs:2578](../../crates/crysirror-sugarpot/src/adapter.rs#L2578) |
| `wout` | wout | Handled | — | [adapter.rs:2578](../../crates/crysirror-sugarpot/src/adapter.rs#L2578) |

## UD effect kinds

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `1` | Crossfade | Handled | — | [update.rs:157](../../crates/crysirror-sugarpot/src/adapter/update.rs#L157) |
| `2` | Stored-mask transition | Partial | Scalar stored-mask policy; no MMX bit-parity claim | [update.rs:157](../../crates/crysirror-sugarpot/src/adapter/update.rs#L157) |
| `3` | Instant target copy; hide native windows | Handled | — | [adapter.rs:1542](../../crates/crysirror-sugarpot/src/adapter.rs#L1542) |
| `4` | Instant target copy; hide native windows | Handled | — | [adapter.rs:1542](../../crates/crysirror-sugarpot/src/adapter.rs#L1542) |
| `5` | Instant target copy; hide native windows | Handled | — | [adapter.rs:1542](../../crates/crysirror-sugarpot/src/adapter.rs#L1542) |
| `6` | Instant target copy; hide native windows | Handled | — | [adapter.rs:1542](../../crates/crysirror-sugarpot/src/adapter.rs#L1542) |
| `7` | Instant target copy; hide native windows | Handled | — | [adapter.rs:1542](../../crates/crysirror-sugarpot/src/adapter.rs#L1542) |
| `8` | Instant target copy; hide native windows | Handled | — | [adapter.rs:1542](../../crates/crysirror-sugarpot/src/adapter.rs#L1542) |
| `9` | Instant target copy; hide native windows | Handled | — | [adapter.rs:1542](../../crates/crysirror-sugarpot/src/adapter.rs#L1542) |
| `10` | Native line/column scan transition | Handled | — | [update.rs:157](../../crates/crysirror-sugarpot/src/adapter/update.rs#L157) |
| `11` | Native line/column scan transition | Handled | — | [update.rs:157](../../crates/crysirror-sugarpot/src/adapter/update.rs#L157) |
| `12` | Native line/column scan transition | Handled | — | [update.rs:157](../../crates/crysirror-sugarpot/src/adapter/update.rs#L157) |
| `13` | Native line/column scan transition | Handled | — | [update.rs:157](../../crates/crysirror-sugarpot/src/adapter/update.rs#L157) |
| `14` | Instant target copy; hide native windows | Handled | — | [adapter.rs:1542](../../crates/crysirror-sugarpot/src/adapter.rs#L1542) |
| `15` | Native line/column scan transition | Handled | — | [update.rs:157](../../crates/crysirror-sugarpot/src/adapter/update.rs#L157) |
| `16` | Native line/column scan transition | Handled | — | [update.rs:157](../../crates/crysirror-sugarpot/src/adapter/update.rs#L157) |
| `17` | Native line/column scan transition | Handled | — | [update.rs:157](../../crates/crysirror-sugarpot/src/adapter/update.rs#L157) |
| `18` | Native line/column scan transition | Handled | — | [update.rs:157](../../crates/crysirror-sugarpot/src/adapter/update.rs#L157) |
| `19` | Zoom old stage in | Handled | — | [update.rs:157](../../crates/crysirror-sugarpot/src/adapter/update.rs#L157) |
| `20` | Zoom target stage toward native size | Handled | — | [update.rs:157](../../crates/crysirror-sugarpot/src/adapter/update.rs#L157) |
| `21` | Shrink old stage | Handled | — | [update.rs:157](../../crates/crysirror-sugarpot/src/adapter/update.rs#L157) |
| `22` | Expand target stage | Handled | — | [update.rs:157](../../crates/crysirror-sugarpot/src/adapter/update.rs#L157) |
| `23` | Rotate/zoom old stage | Handled | — | [update.rs:157](../../crates/crysirror-sugarpot/src/adapter/update.rs#L157) |
| `24` | Rotate/zoom target stage | Handled | — | [update.rs:157](../../crates/crysirror-sugarpot/src/adapter/update.rs#L157) |
| `25` | Circle iris | Handled | — | [update.rs:157](../../crates/crysirror-sugarpot/src/adapter/update.rs#L157) |
| `26` | Inverse circle iris | Handled | — | [update.rs:157](../../crates/crysirror-sugarpot/src/adapter/update.rs#L157) |
| `27` | Angular transition | Handled | — | [update.rs:157](../../crates/crysirror-sugarpot/src/adapter/update.rs#L157) |
| `28` | Angular transition | Handled | — | [update.rs:157](../../crates/crysirror-sugarpot/src/adapter/update.rs#L157) |
| `29` | Band transition | Handled | — | [update.rs:157](../../crates/crysirror-sugarpot/src/adapter/update.rs#L157) |
| `30` | Band transition | Handled | — | [update.rs:157](../../crates/crysirror-sugarpot/src/adapter/update.rs#L157) |
| `31` | Mosaic transition | Handled | — | [update.rs:157](../../crates/crysirror-sugarpot/src/adapter/update.rs#L157) |
| `32` | Truncated row blur | Handled | — | [update.rs:157](../../crates/crysirror-sugarpot/src/adapter/update.rs#L157) |
| `33` | 4x4 cell reveal | Handled | — | [update.rs:157](../../crates/crysirror-sugarpot/src/adapter/update.rs#L157) |
| `34` | Wave transition | Handled | — | [update.rs:157](../../crates/crysirror-sugarpot/src/adapter/update.rs#L157) |
| `35` | Squeeze transition | Handled | — | [update.rs:157](../../crates/crysirror-sugarpot/src/adapter/update.rs#L157) |
| `36` | Squeeze transition | Handled | — | [update.rs:157](../../crates/crysirror-sugarpot/src/adapter/update.rs#L157) |
| `37` | Random runs / middle colour | Handled | — | [update.rs:157](../../crates/crysirror-sugarpot/src/adapter/update.rs#L157) |
| `38` | Random runs / mean colour | Handled | — | [update.rs:157](../../crates/crysirror-sugarpot/src/adapter/update.rs#L157) |
| `39` | Moving-disc copies | Handled | — | [update.rs:157](../../crates/crysirror-sugarpot/src/adapter/update.rs#L157) |

## Subselectors and conditional dispatch

Patterns retain their source namespace and guards. `_` is the fallback of that table. Boolean/optional results follow the function's return contract. Call-site names identify the implementation to inspect, including indirect service calls.

### adapter.rs / SugarPotVm::begin_effect_gates

```rust
fn begin_effect_gates(&mut self) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `self.mode == 0` | Local state/value operations | Handled | [490](../../crates/crysirror-sugarpot/src/adapter.rs#L490) |

### adapter.rs / SugarPotVm::command

```rust
fn command(&mut self, h: &mut dyn HostServices, name: &str, args: &[String]) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under name="saveenable"` | `self.mode == 0` | Local state/value operations | Handled | [2608](../../crates/crysirror-sugarpot/src/adapter.rs#L2608) |
| `condition; under name="savedisable"` | `self.mode == 0` | Local state/value operations | Handled | [2613](../../crates/crysirror-sugarpot/src/adapter.rs#L2613) |
| `condition; under name="loadenable"` | `self.mode == 0` | Local state/value operations | Handled | [2618](../../crates/crysirror-sugarpot/src/adapter.rs#L2618) |
| `condition; under name="loaddisable"` | `self.mode == 0` | Local state/value operations | Handled | [2623](../../crates/crysirror-sugarpot/src/adapter.rs#L2623) |
| `condition; under name="bgmon" \| "envon" \| "seon" \| "cvon"` | `name == "bgmon"` | self.value, Value::Integer | Handled | [2687](../../crates/crysirror-sugarpot/src/adapter.rs#L2687) |
| `condition; under name="bgmon" \| "envon" \| "seon" \| "cvon"` | `name == "cvon"` | Local state/value operations | Handled | [2699](../../crates/crysirror-sugarpot/src/adapter.rs#L2699) |
| `condition; under name="bgmon" \| "envon" \| "seon" \| "cvon" / name=_` | `mode == 1` | Local state/value operations | Handled | [2714](../../crates/crysirror-sugarpot/src/adapter.rs#L2714) |
| `name; under name="bgmon" \| "envon" \| "seon" \| "cvon"` | `_` | Local state/value operations | Handled | [2713](../../crates/crysirror-sugarpot/src/adapter.rs#L2713) |
| `condition; under name="bgmon" \| "envon" \| "seon" \| "cvon"` | `name == "bgmon" && source == self.last_music` | Local state/value operations | Handled | [2719](../../crates/crysirror-sugarpot/src/adapter.rs#L2719) |
| `condition; under name="bgmon" \| "envon" \| "seon" \| "cvon"` | `name == "cvon"` | Local state/value operations | Handled | [2726](../../crates/crysirror-sugarpot/src/adapter.rs#L2726) |
| `condition; under name="bgmon" \| "envon" \| "seon" \| "cvon"` | `name == "bgmon"` | Value::String | Handled | [2731](../../crates/crysirror-sugarpot/src/adapter.rs#L2731) |
| `condition; under name="bgmon" \| "envon" \| "seon" \| "cvon"` | `name == "seon" && mode == 1` | self.stop_audio | Handled | [2739](../../crates/crysirror-sugarpot/src/adapter.rs#L2739) |
| `condition; under name="bgmon" \| "envon" \| "seon" \| "cvon"` | `name == "bgmon"` | Local state/value operations | Handled | [2750](../../crates/crysirror-sugarpot/src/adapter.rs#L2750) |
| `condition; under name="bgmon" \| "envon" \| "seon" \| "cvon"` | `name == "envon"` | Local state/value operations | Handled | [2756](../../crates/crysirror-sugarpot/src/adapter.rs#L2756) |
| `condition; under name="bgmon" \| "envon" \| "seon" \| "cvon"` | `name == "seon"` | Local state/value operations | Handled | [2762](../../crates/crysirror-sugarpot/src/adapter.rs#L2762) |
| `mode; under name="bgmon" \| "envon" \| "seon" \| "cvon"` | `1` | Local state/value operations | Handled | [2764](../../crates/crysirror-sugarpot/src/adapter.rs#L2764) |
| `mode; under name="bgmon" \| "envon" \| "seon" \| "cvon"` | `2` | Local state/value operations | Handled | [2765](../../crates/crysirror-sugarpot/src/adapter.rs#L2765) |
| `mode; under name="bgmon" \| "envon" \| "seon" \| "cvon"` | `_` | Local state/value operations | Handled | [2766](../../crates/crysirror-sugarpot/src/adapter.rs#L2766) |
| `condition; under name="bgmon" \| "envon" \| "seon" \| "cvon"` | `name == "seon" && mode == 1` | Wait::Audio | Handled | [2772](../../crates/crysirror-sugarpot/src/adapter.rs#L2772) |
| `condition; under name="bgmoff" \| "envoff" \| "seoff" \| "cvoff"` | `name == "seoff"` | self.number, self.stop_audio | Handled | [2777](../../crates/crysirror-sugarpot/src/adapter.rs#L2777) |
| `name; under name="bgmoff" \| "envoff" \| "seoff" \| "cvoff"` | `_` | Local state/value operations | Handled | [2791](../../crates/crysirror-sugarpot/src/adapter.rs#L2791) |
| `condition; under name="bgmoff" \| "envoff" \| "seoff" \| "cvoff"` | `name != "cvoff" && self.number(args, 0, 0)? == 0` | self.number | Handled | [2793](../../crates/crysirror-sugarpot/src/adapter.rs#L2793) |
| `condition; under name="bgmoff" \| "envoff" \| "seoff" \| "cvoff"` | `name == "bgmoff"` | self.last_music.clear, self.native_audio.bgm.clear, Value::Integer, Value::String, String::new | Handled | [2800](../../crates/crysirror-sugarpot/src/adapter.rs#L2800) |
| `condition; under name="bgmoff" \| "envoff" \| "seoff" \| "cvoff"` | `name == "envoff"` | self.last_env.clear, self.native_audio.env.clear | Handled | [2818](../../crates/crysirror-sugarpot/src/adapter.rs#L2818) |
| `selector; under name="float"` | `1 if self.save_enabled` | self.start_slot_menu | Handled | [2972](../../crates/crysirror-sugarpot/src/adapter.rs#L2972) |
| `selector; under name="float"` | `2 if self.load_enabled` | self.start_slot_menu | Handled | [2973](../../crates/crysirror-sugarpot/src/adapter.rs#L2973) |
| `selector; under name="float"` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [3027](../../crates/crysirror-sugarpot/src/adapter.rs#L3027) |
| `condition; under name="movie" \| "avi"` | `name == "movie"` | Local state/value operations | Handled | [3073](../../crates/crysirror-sugarpot/src/adapter.rs#L3073) |
| `condition; under name="movie" \| "avi"` | `name == "avi"` | Local state/value operations | Handled | [3084](../../crates/crysirror-sugarpot/src/adapter.rs#L3084) |
| `name` | `_` | Local state/value operations | Unsupported: unsupported SugarPot native command {name}({}) / , | [3128](../../crates/crysirror-sugarpot/src/adapter.rs#L3128) |

### adapter.rs / SugarPotVm::declaration

```rust
fn declaration(
        &mut self,
        h: &mut dyn HostServices,
        name: &str,
        args: &[String],
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under name="ch" \| "bg"` | `name == "bg"` | Local state/value operations | Handled | [1197](../../crates/crysirror-sugarpot/src/adapter.rs#L1197) |
| `condition; under name="ch" \| "bg"` | `id == -1` | self.sprites.len | Handled | [1217](../../crates/crysirror-sugarpot/src/adapter.rs#L1217) |
| `condition; under name="ch" \| "bg"` | `selector == "size"` | self.allocate_sprite_buffers | Handled | [1222](../../crates/crysirror-sugarpot/src/adapter.rs#L1222) |
| `selector; under name="ch" \| "bg"` | `"pos"` | Local state/value operations | Handled | [1231](../../crates/crysirror-sugarpot/src/adapter.rs#L1231) |
| `selector; under name="ch" \| "bg"` | `"sys"` | Local state/value operations | Handled | [1232](../../crates/crysirror-sugarpot/src/adapter.rs#L1232) |
| `selector; under name="ch" \| "bg"` | `"pnt"` | Local state/value operations | Handled | [1233](../../crates/crysirror-sugarpot/src/adapter.rs#L1233) |
| `selector; under name="ch" \| "bg"` | `"atr"` | Local state/value operations | Handled | [1234](../../crates/crysirror-sugarpot/src/adapter.rs#L1234) |
| `selector; under name="ch" \| "bg"` | `_` | Local state/value operations | Unsupported: unsupported SugarPot {name} selector {selector} | [1235](../../crates/crysirror-sugarpot/src/adapter.rs#L1235) |
| `condition; under name="wnd"` | `id == -1 && selector == "btn"` | Local state/value operations | Handled | [1267](../../crates/crysirror-sugarpot/src/adapter.rs#L1267) |
| `condition; under name="wnd"` | `["frm", "fp", "pp", "btn"].contains(&selector.as_str())` | self.image, self.declared_images.insert | Handled | [1272](../../crates/crysirror-sugarpot/src/adapter.rs#L1272) |
| `condition; under name="wnd"` | `selector == "btn"` | self.image, self.declared_images.insert | Handled | [1284](../../crates/crysirror-sugarpot/src/adapter.rs#L1284) |
| `condition; under name="wnd"` | `id == -1` | self.windows.len | Handled | [1296](../../crates/crysirror-sugarpot/src/adapter.rs#L1296) |
| `condition; under name="wnd"` | `selector == "txt"` | Local state/value operations | Handled | [1313](../../crates/crysirror-sugarpot/src/adapter.rs#L1313) |
| `condition; under name="wnd"` | `selector == "btn"` | Local state/value operations | Handled | [1317](../../crates/crysirror-sugarpot/src/adapter.rs#L1317) |
| `name` | `"spr"` | self.number | Handled | [1171](../../crates/crysirror-sugarpot/src/adapter.rs#L1171) |
| `name` | `"btn" if args.len() == 1` | self.number | Handled | [1187](../../crates/crysirror-sugarpot/src/adapter.rs#L1187) |
| `name` | `"ch" \| "bg"` | self.number, self.sprites.len, self.allocate_sprite_buffers | Partial: SugarPot bg unsupported selector / unsupported SugarPot {name} selector {selector} | [1196](../../crates/crysirror-sugarpot/src/adapter.rs#L1196) |
| `name` | `"wnd"` | self.number, Window::default, self.image, self.declared_images.insert, self.windows.len | Partial: unsupported SugarPot wnd selector {selector} | [1239](../../crates/crysirror-sugarpot/src/adapter.rs#L1239) |
| `name` | `"wbl"` | self.configure_backlog | Handled | [1334](../../crates/crysirror-sugarpot/src/adapter.rs#L1334) |
| `name` | `_` | Local state/value operations | Unsupported: unsupported SugarPot declaration {name} | [1335](../../crates/crysirror-sugarpot/src/adapter.rs#L1335) |

### adapter.rs / SugarPotVm::finish_effect_gates

```rust
fn finish_effect_gates(&mut self) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `self.mode == 0` | Local state/value operations | Handled | [498](../../crates/crysirror-sugarpot/src/adapter.rs#L498) |

### adapter.rs / SugarPotVm::poll_wait

```rust
fn poll_wait(
        &mut self,
        h: &mut dyn HostServices,
        input: &mut InputFrame,
    ) -> Result<Option<VmYield>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under wait=Wait::Input \| Wait::Text` | `self.mode == 0 && self.native_text.waiting == 16` | Local state/value operations | Handled | [2192](../../crates/crysirror-sugarpot/src/adapter.rs#L2192) |
| `condition; under wait=Wait::Input \| Wait::Text` | `self.mode == 0 && self.native_text.waiting == 16` | Local state/value operations | Handled | [2288](../../crates/crysirror-sugarpot/src/adapter.rs#L2288) |
| `condition; under wait=Wait::Dialog` | `!events.iter().any(\|event\| { event["kind"] == "dialog_result" && event["widget"]["id"] == 65500 && event["widget"]["owner"] == self.client.0 })` | Local state/value operations | Handled | [2473](../../crates/crysirror-sugarpot/src/adapter.rs#L2473) |
| `condition; under wait=Wait::Buttons` | `let Some(layer) = Self::sprite_layer(1000 + *id as u32, &b.sprite, 1000 + *id)?` | Local state/value operations | Handled | [2535](../../crates/crysirror-sugarpot/src/adapter.rs#L2535) |

### adapter.rs / SugarPotVm::publish

```rust
fn publish(&mut self, h: &mut dyn HostServices, transition: Option<(Json, u64)>) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `id != 0 && s.system != 0 && !system_visible` | Local state/value operations | Handled | [1003](../../crates/crysirror-sugarpot/src/adapter.rs#L1003) |
| `condition` | `let Some(mut layer) = Self::sprite_layer( id as u32 + 1, s, if id == 0 { 0 } else if s.system != 0 { 2000 + id as i32 } else { 100 + id as i32 }, )?` | Local state/value operations | Handled | [1006](../../crates/crysirror-sugarpot/src/adapter.rs#L1006) |
| `condition` | `id == 0` | Local state/value operations | Handled | [1009](../../crates/crysirror-sugarpot/src/adapter.rs#L1009) |
| `condition` | `let Some(layer) = Self::sprite_layer(1000 + *id as u32, &b.sprite, 1000 + *id)?` | Vec::new, String::new, crysirror_text::grid::layout_cells, crysirror_text::grid::CellCursor::at_start | Handled | [1028](../../crates/crysirror-sugarpot/src/adapter.rs#L1028) |

### adapter.rs / SugarPotVm::render_text

```rust
fn render_text(
        &mut self,
        h: &mut dyn HostServices,
        prepared: crate::text::Prepared,
        record: bool,
        continuation: bool,
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `self.mode == 0` | Local state/value operations | Handled | [1708](../../crates/crysirror-sugarpot/src/adapter.rs#L1708) |
| `condition` | `self.mode == 2 && self.text_remainder.is_empty()` | Local state/value operations | Handled | [1873](../../crates/crysirror-sugarpot/src/adapter.rs#L1873) |

### adapter.rs / SugarPotVm::resolve

```rust
fn resolve(&self, bank: &str, name: &str, extensions: &[&str]) -> Result<String>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Some(path) = self.config.assets.get(&format!("{bank}/{name}.{ext}"))` | Local state/value operations | Handled | [535](../../crates/crysirror-sugarpot/src/adapter.rs#L535) |

### adapter.rs / SugarPotVm::sprite_layer

```rust
fn sprite_layer(id: u32, s: &Sprite, order: i32) -> Result<Option<Json>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `s.native_depth == 24 && id != 1` | Local state/value operations | Handled | [947](../../crates/crysirror-sugarpot/src/adapter.rs#L947) |

### adapter.rs / SugarPotVm::step_inner

```rust
fn step_inner(
        &mut self,
        h: &mut dyn HostServices,
        mut input: InputFrame,
        budget: u32,
    ) -> Result<VmYield>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under event=Event::Blank` | `self.native_text.blank(line, self.mode == 2)?` | self.capture_history_voice, self.advance_native_line | Handled | [3199](../../crates/crysirror-sugarpot/src/adapter.rs#L3199) |

### adapter.rs / SugarPotVm::submit

```rust
fn submit(&mut self, h: &mut dyn HostServices, layers: Vec<Json>, ui: Vec<Json>) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `command["op"] == "transition"` | h.now_ms | Handled | [469](../../crates/crysirror-sugarpot/src/adapter.rs#L469) |

### adapter.rs / SugarPotVm::visual

```rust
fn visual(&mut self, h: &mut dyn HostServices, name: &str, args: &[String]) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `name == "cgon"` | Local state/value operations | Handled | [1340](../../crates/crysirror-sugarpot/src/adapter.rs#L1340) |
| `condition; under name="bgon" \| "cgon" \| "chon"` | `name == "chon"` | usize::try_from, self.number | Handled | [1351](../../crates/crysirror-sugarpot/src/adapter.rs#L1351) |
| `condition; under name="bgon" \| "cgon" \| "chon"` | `name == "chon" && self.sprites.get(id).context("SugarPot sprite ID")?.system == 3` | Local state/value operations | Handled | [1356](../../crates/crysirror-sugarpot/src/adapter.rs#L1356) |
| `condition; under name="bgon" \| "cgon" \| "chon"` | `name == "chon"` | Local state/value operations | Handled | [1361](../../crates/crysirror-sugarpot/src/adapter.rs#L1361) |
| `condition; under name="bgon" \| "cgon" \| "chon"` | `self.sprites.get(id).context("SugarPot sprite ID")?.system == 0` | self.tone_animation, self.tone_image | Handled | [1366](../../crates/crysirror-sugarpot/src/adapter.rs#L1366) |
| `condition; under name="bgon" \| "cgon" \| "chon"` | `name == "chon"` | Local state/value operations | Handled | [1385](../../crates/crysirror-sugarpot/src/adapter.rs#L1385) |
| `condition; under name="bgon" \| "cgon" \| "chon"` | `name == "chon"` | Local state/value operations | Handled | [1386](../../crates/crysirror-sugarpot/src/adapter.rs#L1386) |
| `condition; under name="bgon" \| "cgon" \| "chon"` | `name == "chon"` | Local state/value operations | Handled | [1388](../../crates/crysirror-sugarpot/src/adapter.rs#L1388) |
| `condition; under name="bgon" \| "cgon" \| "chon"` | `name == "chon"` | Local state/value operations | Handled | [1389](../../crates/crysirror-sugarpot/src/adapter.rs#L1389) |
| `condition; under name="bgon" \| "cgon" \| "chon"` | `name == "chon"` | Local state/value operations | Handled | [1390](../../crates/crysirror-sugarpot/src/adapter.rs#L1390) |
| `condition; under name="bgon" \| "cgon" \| "chon"` | `name == "chon"` | Local state/value operations | Handled | [1398](../../crates/crysirror-sugarpot/src/adapter.rs#L1398) |
| `condition; under name="bgon" \| "cgon" \| "chon"` | `id == 0` | self.clear_stage_underlay | Handled | [1423](../../crates/crysirror-sugarpot/src/adapter.rs#L1423) |
| `condition; under name="bgon" \| "cgon" \| "chon"` | `name == "cgon"` | self.globals.insert, Value::Integer, self.write_globals | Handled | [1426](../../crates/crysirror-sugarpot/src/adapter.rs#L1426) |
| `condition; under name="choff"` | `id == 0` | self.clear_stage_underlay | Handled | [1436](../../crates/crysirror-sugarpot/src/adapter.rs#L1436) |
| `condition; under name="choff"` | `id == -1` | self.sprites.iter_mut | Handled | [1439](../../crates/crysirror-sugarpot/src/adapter.rs#L1439) |
| `condition; under name="btnon" \| "btnoff" \| "btnreset"` | `name == "btnreset" && id == -1` | Local state/value operations | Handled | [1507](../../crates/crysirror-sugarpot/src/adapter.rs#L1507) |
| `condition; under name="btnon" \| "btnoff" \| "btnreset"` | `id == -1 \|\| id == *n` | Local state/value operations | Handled | [1511](../../crates/crysirror-sugarpot/src/adapter.rs#L1511) |
| `condition; under name="btnon" \| "btnoff" \| "btnreset"` | `name == "btnon"` | Local state/value operations | Handled | [1512](../../crates/crysirror-sugarpot/src/adapter.rs#L1512) |
| `condition; under name="ud"` | `matches!(kind, 1 \| 2 \| 10..=13 \| 15..=39)` | self.mask_update, self.finish_effect_gates | Handled | [1525](../../crates/crysirror-sugarpot/src/adapter.rs#L1525) |
| `condition; under name="ud"` | `matches!(kind, 1 \| 2 \| 10..=13 \| 15..=39)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [1525](../../crates/crysirror-sugarpot/src/adapter.rs#L1525) |
| `condition; under name="ud"` | `kind != 0` | Local state/value operations | Handled | [1537](../../crates/crysirror-sugarpot/src/adapter.rs#L1537) |
| `condition; under name="ud"` | `kind == 0 && self .windows .get(self.current_window) .is_some_and(\|window\| window.visibility == 1) && !self.native_text.committed.is_empty()` | self.native_text.committed.clone, self.wait.take, self.text_inner | Handled | [1546](../../crates/crysirror-sugarpot/src/adapter.rs#L1546) |
| `condition; under name="wout" \| "bout" \| "win" \| "bin"` | `name.ends_with("out")` | Local state/value operations | Handled | [1568](../../crates/crysirror-sugarpot/src/adapter.rs#L1568) |
| `name` | `"bgon" \| "cgon" \| "chon"` | usize::try_from, self.number, self.sprites.get, self.string, self.image | Handled | [1350](../../crates/crysirror-sugarpot/src/adapter.rs#L1350) |
| `name` | `"choff"` | self.number, self.clear_stage_underlay, self.sprites.iter_mut, usize::try_from | Handled | [1434](../../crates/crysirror-sugarpot/src/adapter.rs#L1434) |
| `name` | `"bgoff" \| "cgoff"` | self.clear_stage_underlay | Handled | [1452](../../crates/crysirror-sugarpot/src/adapter.rs#L1452) |
| `name` | `"btnset"` | self.number, self.string, self.image, self.depths.get, Default::default | Handled | [1459](../../crates/crysirror-sugarpot/src/adapter.rs#L1459) |
| `name` | `"btnon" \| "btnoff" \| "btnreset"` | self.number | Handled | [1505](../../crates/crysirror-sugarpot/src/adapter.rs#L1505) |
| `name` | `"ud"` | self.begin_effect_gates, self.number, self.mask_update, self.finish_effect_gates, self.clear_stage_underlay | Partial: unsupported SugarPot update effect {kind} | [1522](../../crates/crysirror-sugarpot/src/adapter.rs#L1522) |
| `name` | `"wout" \| "bout" \| "win" \| "bin"` | self.begin_effect_gates, self.number, self.publish, Wait::Clock, h.now_ms | Handled | [1563](../../crates/crysirror-sugarpot/src/adapter.rs#L1563) |
| `name` | `"wndoff" \| "wndon"` | self.number, self.windows.get, self.begin_effect_gates, self.clear_window_controls, self.window_layers.is_empty | Handled | [1583](../../crates/crysirror-sugarpot/src/adapter.rs#L1583) |
| `name` | `_` | Local state/value operations | Unsupported: unsupported SugarPot visual command {name} | [1630](../../crates/crysirror-sugarpot/src/adapter.rs#L1630) |

### adapter/animation.rs / SugarPotVm::animate

```rust
fn animate(&mut self, h: &mut dyn HostServices) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!sprite.visible \|\| !self.published_layers.contains(&(id as u32 + 1))` | Local state/value operations | Handled | [124](../../crates/crysirror-sugarpot/src/adapter/animation.rs#L124) |

### adapter/backlog.rs / History::record

```rust
fn record(
        &mut self,
        raw: &str,
        mode: i32,
        voice: &str,
        window: &Window,
        size: i32,
    ) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `mode == 2` | i32::from | Handled | [74](../../crates/crysirror-sugarpot/src/adapter/backlog.rs#L74) |
| `condition` | `mode == 2` | Local state/value operations | Handled | [103](../../crates/crysirror-sugarpot/src/adapter/backlog.rs#L103) |
| `condition` | `mode == 2 && page_end(raw)` | Local state/value operations | Handled | [104](../../crates/crysirror-sugarpot/src/adapter/backlog.rs#L104) |
| `condition` | `mode != 2` | self.bytes.saturating_sub | Handled | [107](../../crates/crysirror-sugarpot/src/adapter/backlog.rs#L107) |
| `condition` | `mode == 2` | Local state/value operations | Handled | [120](../../crates/crysirror-sugarpot/src/adapter/backlog.rs#L120) |

### adapter/backlog.rs / SugarPotVm::backlog_step

```rust
fn backlog_step(
        &mut self,
        h: &mut dyn HostServices,
        input: &mut InputFrame,
        view: &mut View,
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `id` | `0` | Local state/value operations | Handled | [701](../../crates/crysirror-sugarpot/src/adapter/backlog.rs#L701) |
| `id` | `1` | Local state/value operations | Handled | [702](../../crates/crysirror-sugarpot/src/adapter/backlog.rs#L702) |
| `id` | `2` | Local state/value operations | Handled | [703](../../crates/crysirror-sugarpot/src/adapter/backlog.rs#L703) |
| `id` | `3` | Local state/value operations | Handled | [704](../../crates/crysirror-sugarpot/src/adapter/backlog.rs#L704) |
| `id` | `4` | Local state/value operations | Handled | [705](../../crates/crysirror-sugarpot/src/adapter/backlog.rs#L705) |
| `id` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [706](../../crates/crysirror-sugarpot/src/adapter/backlog.rs#L706) |
| `event["kind"].as_str().unwrap_or("")` | `"bl_close"` | Local state/value operations | Handled | [728](../../crates/crysirror-sugarpot/src/adapter/backlog.rs#L728) |
| `event["kind"].as_str().unwrap_or("")` | `"bl_previous"` | Local state/value operations | Handled | [729](../../crates/crysirror-sugarpot/src/adapter/backlog.rs#L729) |
| `event["kind"].as_str().unwrap_or("")` | `"bl_next"` | Local state/value operations | Handled | [730](../../crates/crysirror-sugarpot/src/adapter/backlog.rs#L730) |
| `event["kind"].as_str().unwrap_or("")` | `"bl_top"` | Local state/value operations | Handled | [731](../../crates/crysirror-sugarpot/src/adapter/backlog.rs#L731) |
| `event["kind"].as_str().unwrap_or("")` | `"bl_bottom"` | Local state/value operations | Handled | [732](../../crates/crysirror-sugarpot/src/adapter/backlog.rs#L732) |
| `event["kind"].as_str().unwrap_or("")` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [733](../../crates/crysirror-sugarpot/src/adapter/backlog.rs#L733) |

### adapter/backlog.rs / SugarPotVm::configure_backlog

```rust
fn configure_backlog(
        &mut self,
        h: &mut dyn HostServices,
        args: &[String],
        values: &[Value],
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `selector == 0` | self.open_backlog | Handled | [206](../../crates/crysirror-sugarpot/src/adapter/backlog.rs#L206) |
| `selector` | `1` | Local state/value operations | Handled | [211](../../crates/crysirror-sugarpot/src/adapter/backlog.rs#L211) |
| `selector` | `2` | Local state/value operations | Handled | [212](../../crates/crysirror-sugarpot/src/adapter/backlog.rs#L212) |
| `selector` | `4` | Local state/value operations | Handled | [213](../../crates/crysirror-sugarpot/src/adapter/backlog.rs#L213) |
| `selector` | `5` | Local state/value operations | Handled | [214](../../crates/crysirror-sugarpot/src/adapter/backlog.rs#L214) |
| `selector` | `6` | Local state/value operations | Handled | [215](../../crates/crysirror-sugarpot/src/adapter/backlog.rs#L215) |
| `selector` | `9` | Local state/value operations | Handled | [216](../../crates/crysirror-sugarpot/src/adapter/backlog.rs#L216) |
| `selector` | `10` | Local state/value operations | Handled | [217](../../crates/crysirror-sugarpot/src/adapter/backlog.rs#L217) |
| `selector` | `_` | Local state/value operations | Handled | [219](../../crates/crysirror-sugarpot/src/adapter/backlog.rs#L219) |
| `condition` | `matches!(name, "frm" \| "btn")` | self.image | Handled | [224](../../crates/crysirror-sugarpot/src/adapter/backlog.rs#L224) |
| `condition` | `matches!(name, "frm" \| "btn")` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [224](../../crates/crysirror-sugarpot/src/adapter/backlog.rs#L224) |
| `condition` | `name == "btn"` | self.backlog.buttons.push | Handled | [238](../../crates/crysirror-sugarpot/src/adapter/backlog.rs#L238) |
| `condition` | `!(0..=4).contains(&id)` | Local state/value operations | Handled | [240](../../crates/crysirror-sugarpot/src/adapter/backlog.rs#L240) |
| `condition` | `name == "mod"` | self.backlog.properties.insert | Handled | [247](../../crates/crysirror-sugarpot/src/adapter/backlog.rs#L247) |
| `condition` | `name == "cel"` | Local state/value operations | Handled | [255](../../crates/crysirror-sugarpot/src/adapter/backlog.rs#L255) |
| `condition` | `name == "txt"` | Local state/value operations | Handled | [257](../../crates/crysirror-sugarpot/src/adapter/backlog.rs#L257) |

### adapter/backlog.rs / SugarPotVm::paint_backlog

```rust
fn paint_backlog(
        &mut self,
        h: &mut dyn HostServices,
        view: &mut View,
        background: bool,
        replace_widgets: bool,
    ) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `record.mode != 2` | Local state/value operations | Handled | [601](../../crates/crysirror-sugarpot/src/adapter/backlog.rs#L601) |

### adapter/bgm_mode.rs / direction

```rust
fn direction(id: usize, key: &str) -> Option<usize>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `id; under key="bgm_up"` | `0..=2` | Local state/value operations | Handled | [80](../../crates/crysirror-sugarpot/src/adapter/bgm_mode.rs#L80) |
| `id; under key="bgm_up"` | `3` | Local state/value operations | Handled | [81](../../crates/crysirror-sugarpot/src/adapter/bgm_mode.rs#L81) |
| `id; under key="bgm_up"` | `4 \| 12` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [82](../../crates/crysirror-sugarpot/src/adapter/bgm_mode.rs#L82) |
| `id; under key="bgm_up"` | `5..=21` | Local state/value operations | Handled | [83](../../crates/crysirror-sugarpot/src/adapter/bgm_mode.rs#L83) |
| `id; under key="bgm_up"` | `_` | Local state/value operations | Handled | [84](../../crates/crysirror-sugarpot/src/adapter/bgm_mode.rs#L84) |
| `id; under key="bgm_down"` | `0..=2` | Local state/value operations | Handled | [87](../../crates/crysirror-sugarpot/src/adapter/bgm_mode.rs#L87) |
| `id; under key="bgm_down"` | `3` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [88](../../crates/crysirror-sugarpot/src/adapter/bgm_mode.rs#L88) |
| `id; under key="bgm_down"` | `11 \| 19` | Local state/value operations | Handled | [89](../../crates/crysirror-sugarpot/src/adapter/bgm_mode.rs#L89) |
| `id; under key="bgm_down"` | `4..=21` | Local state/value operations | Handled | [90](../../crates/crysirror-sugarpot/src/adapter/bgm_mode.rs#L90) |
| `id; under key="bgm_down"` | `_` | Local state/value operations | Handled | [91](../../crates/crysirror-sugarpot/src/adapter/bgm_mode.rs#L91) |
| `id; under key="bgm_left"` | `0 \| 3` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [94](../../crates/crysirror-sugarpot/src/adapter/bgm_mode.rs#L94) |
| `id; under key="bgm_left"` | `1` | Local state/value operations | Handled | [95](../../crates/crysirror-sugarpot/src/adapter/bgm_mode.rs#L95) |
| `id; under key="bgm_left"` | `2` | Local state/value operations | Handled | [96](../../crates/crysirror-sugarpot/src/adapter/bgm_mode.rs#L96) |
| `id; under key="bgm_left"` | `4..=11` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [97](../../crates/crysirror-sugarpot/src/adapter/bgm_mode.rs#L97) |
| `id; under key="bgm_left"` | `12..=21` | Local state/value operations | Handled | [98](../../crates/crysirror-sugarpot/src/adapter/bgm_mode.rs#L98) |
| `id; under key="bgm_left"` | `_` | Local state/value operations | Handled | [99](../../crates/crysirror-sugarpot/src/adapter/bgm_mode.rs#L99) |
| `id; under key="bgm_right"` | `0` | Local state/value operations | Handled | [102](../../crates/crysirror-sugarpot/src/adapter/bgm_mode.rs#L102) |
| `id; under key="bgm_right"` | `1` | Local state/value operations | Handled | [103](../../crates/crysirror-sugarpot/src/adapter/bgm_mode.rs#L103) |
| `id; under key="bgm_right"` | `2 \| 3` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [104](../../crates/crysirror-sugarpot/src/adapter/bgm_mode.rs#L104) |
| `id; under key="bgm_right"` | `4..=11` | Local state/value operations | Handled | [105](../../crates/crysirror-sugarpot/src/adapter/bgm_mode.rs#L105) |
| `id; under key="bgm_right"` | `12..=21` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [106](../../crates/crysirror-sugarpot/src/adapter/bgm_mode.rs#L106) |
| `id; under key="bgm_right"` | `_` | Local state/value operations | Handled | [107](../../crates/crysirror-sugarpot/src/adapter/bgm_mode.rs#L107) |
| `key` | `"bgm_up"` | Local state/value operations | Handled | [79](../../crates/crysirror-sugarpot/src/adapter/bgm_mode.rs#L79) |
| `key` | `"bgm_down"` | Local state/value operations | Handled | [86](../../crates/crysirror-sugarpot/src/adapter/bgm_mode.rs#L86) |
| `key` | `"bgm_left"` | Local state/value operations | Handled | [93](../../crates/crysirror-sugarpot/src/adapter/bgm_mode.rs#L93) |
| `key` | `"bgm_right"` | Local state/value operations | Handled | [101](../../crates/crysirror-sugarpot/src/adapter/bgm_mode.rs#L101) |
| `key` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [109](../../crates/crysirror-sugarpot/src/adapter/bgm_mode.rs#L109) |

### adapter/bgm_mode.rs / row

```rust
fn row(id: usize) -> [i32; 12]
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `id` | `0` | Local state/value operations | Handled | [48](../../crates/crysirror-sugarpot/src/adapter/bgm_mode.rs#L48) |
| `id` | `1` | Local state/value operations | Handled | [49](../../crates/crysirror-sugarpot/src/adapter/bgm_mode.rs#L49) |
| `id` | `2` | Local state/value operations | Handled | [50](../../crates/crysirror-sugarpot/src/adapter/bgm_mode.rs#L50) |
| `id` | `3` | Local state/value operations | Handled | [51](../../crates/crysirror-sugarpot/src/adapter/bgm_mode.rs#L51) |
| `id` | `_` | Local state/value operations | Handled | [52](../../crates/crysirror-sugarpot/src/adapter/bgm_mode.rs#L52) |

### adapter/bgm_mode.rs / SugarPotVm::bgm_icon

```rust
fn bgm_icon(&self, mode: &BgmMode, id: usize, cell: u8) -> Result<Json>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `id < 4` | Local state/value operations | Handled | [197](../../crates/crysirror-sugarpot/src/adapter/bgm_mode.rs#L197) |

### adapter/bgm_mode.rs / SugarPotVm::bgm_mode_step

```rust
fn bgm_mode_step(
        &mut self,
        h: &mut dyn HostServices,
        input: &mut InputFrame,
        mode: &mut BgmMode,
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `mode.stage != 2` | self.bgm_palette | Handled | [302](../../crates/crysirror-sugarpot/src/adapter/bgm_mode.rs#L302) |
| `condition` | `!self.play_audio(h, &name, 0, PcmBus::Music, mode.play_mode == 2)?` | Local state/value operations | Handled | [307](../../crates/crysirror-sugarpot/src/adapter/bgm_mode.rs#L307) |
| `condition` | `mode.play_mode == 2` | Local state/value operations | Handled | [315](../../crates/crysirror-sugarpot/src/adapter/bgm_mode.rs#L315) |
| `condition` | `self.settings.header[1] != 1 && mode.selected.is_some()` | self.stop_audio | Handled | [317](../../crates/crysirror-sugarpot/src/adapter/bgm_mode.rs#L317) |
| `condition` | `mode.playing && !h .audio_info(self.client, 0)? .is_some_and(\|info\| info.source_playing)` | self.stop_audio | Handled | [324](../../crates/crysirror-sugarpot/src/adapter/bgm_mode.rs#L324) |
| `condition` | `mode.play_mode == 1` | Local state/value operations | Handled | [329](../../crates/crysirror-sugarpot/src/adapter/bgm_mode.rs#L329) |
| `condition` | `mode.play_mode == 0` | self.stop_audio | Handled | [334](../../crates/crysirror-sugarpot/src/adapter/bgm_mode.rs#L334) |
| `event["kind"].as_str().unwrap_or("")` | `"bgm_close"` | Local state/value operations | Handled | [377](../../crates/crysirror-sugarpot/src/adapter/bgm_mode.rs#L377) |
| `event["kind"].as_str().unwrap_or("")` | `"bgm_activate" if mode.focused < 22` | Local state/value operations | Handled | [378](../../crates/crysirror-sugarpot/src/adapter/bgm_mode.rs#L378) |
| `event["kind"].as_str().unwrap_or("")` | `key @ ("bgm_up" \| "bgm_down" \| "bgm_left" \| "bgm_right")` | Local state/value operations | Handled | [379](../../crates/crysirror-sugarpot/src/adapter/bgm_mode.rs#L379) |
| `event["kind"].as_str().unwrap_or("")` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [382](../../crates/crysirror-sugarpot/src/adapter/bgm_mode.rs#L382) |
| `condition; under id=0 \| 1` | `mode.play_mode == 2` | Local state/value operations | Handled | [440](../../crates/crysirror-sugarpot/src/adapter/bgm_mode.rs#L440) |
| `id` | `0 \| 1` | h.submit_audio | Handled | [422](../../crates/crysirror-sugarpot/src/adapter/bgm_mode.rs#L422) |
| `id` | `2` | self.stop_audio | Handled | [443](../../crates/crysirror-sugarpot/src/adapter/bgm_mode.rs#L443) |
| `id` | `4..=21` | Local state/value operations | Handled | [449](../../crates/crysirror-sugarpot/src/adapter/bgm_mode.rs#L449) |
| `id` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [455](../../crates/crysirror-sugarpot/src/adapter/bgm_mode.rs#L455) |

### adapter/bgm_mode.rs / SugarPotVm::bgm_palette

```rust
fn bgm_palette(
        &mut self,
        h: &mut dyn HostServices,
        input: &mut InputFrame,
        mode: &mut BgmMode,
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `mode.stage == 3 && mode.capture.is_some()` | Local state/value operations | Handled | [473](../../crates/crysirror-sugarpot/src/adapter/bgm_mode.rs#L473) |
| `condition; under job.poll()?=ImageProcessState::Ready(resource)` | `mode.stage == 0 \|\| mode.stage == 3` | Local state/value operations | Handled | [491](../../crates/crysirror-sugarpot/src/adapter/bgm_mode.rs#L491) |
| `condition` | `mode.stage == 0 \|\| mode.stage == 3` | Local state/value operations | Handled | [501](../../crates/crysirror-sugarpot/src/adapter/bgm_mode.rs#L501) |
| `condition` | `(mode.stage == 0 \|\| mode.stage == 3) && mode.weight == 0` | self.submit, self.stop_audio, self.paint_bgm_mode, h.begin_layer_prefix_image_over, self.play_audio | Handled | [511](../../crates/crysirror-sugarpot/src/adapter/bgm_mode.rs#L511) |
| `condition` | `mode.stage == 0` | self.stop_audio, self.paint_bgm_mode, h.begin_layer_prefix_image_over | Handled | [520](../../crates/crysirror-sugarpot/src/adapter/bgm_mode.rs#L520) |
| `condition` | `(mode.stage == 1 \|\| mode.stage == 4) && mode.weight >= 256` | self.submit, h.release_image_resource, self.stop_audio | Handled | [567](../../crates/crysirror-sugarpot/src/adapter/bgm_mode.rs#L567) |
| `condition` | `mode.stage == 1` | self.submit, h.release_image_resource | Handled | [568](../../crates/crysirror-sugarpot/src/adapter/bgm_mode.rs#L568) |
| `condition` | `mode.stage == 0 \|\| mode.stage == 3` | self.bgm_attenuate | Handled | [594](../../crates/crysirror-sugarpot/src/adapter/bgm_mode.rs#L594) |
| `condition` | `mode.stage == 0 \|\| mode.stage == 4` | Local state/value operations | Handled | [597](../../crates/crysirror-sugarpot/src/adapter/bgm_mode.rs#L597) |

### adapter/choice.rs / SugarPotVm::poll_choices

```rust
fn poll_choices(
        &mut self,
        h: &mut dyn HostServices,
        input: &mut InputFrame,
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `event["widget"]["owner"] != self.client.0 \|\| event["widget"]["id"] != ROOT` | Local state/value operations | Handled | [225](../../crates/crysirror-sugarpot/src/adapter/choice.rs#L225) |
| `event["kind"].as_str().unwrap_or("")` | `"sel_activate"` | Local state/value operations | Handled | [229](../../crates/crysirror-sugarpot/src/adapter/choice.rs#L229) |
| `event["kind"].as_str().unwrap_or("")` | `"sel_up"` | Local state/value operations | Handled | [230](../../crates/crysirror-sugarpot/src/adapter/choice.rs#L230) |
| `event["kind"].as_str().unwrap_or("")` | `"sel_down"` | Local state/value operations | Handled | [231](../../crates/crysirror-sugarpot/src/adapter/choice.rs#L231) |
| `event["kind"].as_str().unwrap_or("")` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [232](../../crates/crysirror-sugarpot/src/adapter/choice.rs#L232) |
| `condition` | `self.mode == 2` | Local state/value operations | Handled | [259](../../crates/crysirror-sugarpot/src/adapter/choice.rs#L259) |

### adapter/endmode.rs / SugarPotVm::end_mode_step

```rust
fn end_mode_step(
        &mut self,
        h: &mut dyn HostServices,
        input: &mut InputFrame,
        mode: &mut EndMode,
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under job.poll()?=ImageProcessState::Ready(resource)` | `mode.stage % 2 == 0` | Local state/value operations | Handled | [194](../../crates/crysirror-sugarpot/src/adapter/endmode.rs#L194) |
| `condition` | `mode.stage % 2 == 0` | Local state/value operations | Handled | [205](../../crates/crysirror-sugarpot/src/adapter/endmode.rs#L205) |
| `condition` | `mode.stage % 2 == 0` | Local state/value operations | Handled | [208](../../crates/crysirror-sugarpot/src/adapter/endmode.rs#L208) |
| `condition` | `mode.stage % 2 == 0 && mode.weight == 0` | self.submit, self.paint_end_mode, h.begin_layer_prefix_image_over | Handled | [213](../../crates/crysirror-sugarpot/src/adapter/endmode.rs#L213) |
| `condition` | `mode.stage == 0` | self.paint_end_mode, h.begin_layer_prefix_image_over | Handled | [216](../../crates/crysirror-sugarpot/src/adapter/endmode.rs#L216) |
| `condition` | `mode.stage % 2 == 1 && mode.weight >= 256` | self.end_overlay, self.submit, h.release_image_resource | Handled | [246](../../crates/crysirror-sugarpot/src/adapter/endmode.rs#L246) |
| `condition` | `mode.stage == 1` | Local state/value operations | Handled | [248](../../crates/crysirror-sugarpot/src/adapter/endmode.rs#L248) |
| `condition` | `mode.stage == 1` | Local state/value operations | Handled | [256](../../crates/crysirror-sugarpot/src/adapter/endmode.rs#L256) |
| `condition` | `mode.stage == 0 \|\| mode.stage == 3` | Local state/value operations | Handled | [270](../../crates/crysirror-sugarpot/src/adapter/endmode.rs#L270) |

### adapter/fall.rs / SugarPotVm::fall_command

```rust
fn fall_command(
        &mut self,
        h: &mut dyn HostServices,
        args: &[String],
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `selector == 0 && self.fall.is_none()` | Local state/value operations | Handled | [299](../../crates/crysirror-sugarpot/src/adapter/fall.rs#L299) |
| `condition` | `!(0..=3).contains(&selector)` | Local state/value operations | Handled | [302](../../crates/crysirror-sugarpot/src/adapter/fall.rs#L302) |
| `condition` | `selector == 0` | Local state/value operations | Handled | [315](../../crates/crysirror-sugarpot/src/adapter/fall.rs#L315) |

### adapter/filter.rs / SugarPotVm::filter_command

```rust
fn filter_command(
        &mut self,
        h: &mut dyn HostServices,
        args: &[String],
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `selector == 0` | Local state/value operations | Handled | [240](../../crates/crysirror-sugarpot/src/adapter/filter.rs#L240) |
| `condition` | `selector == 1` | self.string, self.resolve, self.image, self.filter.take, self.submit | Handled | [249](../../crates/crysirror-sugarpot/src/adapter/filter.rs#L249) |

### adapter/gauge.rs / SugarPotVm::gauge

```rust
fn gauge(
        &mut self,
        h: &mut dyn HostServices,
        name: &str,
        args: &[String],
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `name == "gg"` | self.number, self.gauges.len, self.gauges.get_mut | Handled | [13](../../crates/crysirror-sugarpot/src/adapter/gauge.rs#L13) |
| `condition` | `!(1..=8).contains(&selector)` | Local state/value operations | Handled | [15](../../crates/crysirror-sugarpot/src/adapter/gauge.rs#L15) |
| `condition` | `id == -1` | self.gauges.len | Handled | [24](../../crates/crysirror-sugarpot/src/adapter/gauge.rs#L24) |
| `selector` | `1` | Local state/value operations | Handled | [32](../../crates/crysirror-sugarpot/src/adapter/gauge.rs#L32) |
| `selector` | `2` | Local state/value operations | Handled | [36](../../crates/crysirror-sugarpot/src/adapter/gauge.rs#L36) |
| `selector` | `3` | Local state/value operations | Handled | [40](../../crates/crysirror-sugarpot/src/adapter/gauge.rs#L40) |
| `selector` | `4` | Local state/value operations | Handled | [41](../../crates/crysirror-sugarpot/src/adapter/gauge.rs#L41) |
| `selector` | `5` | Local state/value operations | Handled | [42](../../crates/crysirror-sugarpot/src/adapter/gauge.rs#L42) |
| `selector` | `6` | Local state/value operations | Handled | [43](../../crates/crysirror-sugarpot/src/adapter/gauge.rs#L43) |
| `selector` | `7` | Local state/value operations | Handled | [44](../../crates/crysirror-sugarpot/src/adapter/gauge.rs#L44) |
| `selector` | `8` | Local state/value operations | Handled | [45](../../crates/crysirror-sugarpot/src/adapter/gauge.rs#L45) |
| `selector` | `_` | Local state/value operations | Handled | [46](../../crates/crysirror-sugarpot/src/adapter/gauge.rs#L46) |
| `condition` | `name == "ggoff"` | self.gauges.len, self.gauges.get_mut, self.submit | Handled | [51](../../crates/crysirror-sugarpot/src/adapter/gauge.rs#L51) |
| `condition` | `id == -1` | self.gauges.len | Handled | [52](../../crates/crysirror-sugarpot/src/adapter/gauge.rs#L52) |

### adapter/geometry.rs / single

```rust
fn single(selector: i32, p: i32, position: [i32; 2], size: [u32; 2]) -> Option<RectMapRegion>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `selector` | `20` | Local state/value operations | Handled | [97](../../crates/crysirror-sugarpot/src/adapter/geometry.rs#L97) |
| `selector` | `21` | Local state/value operations | Handled | [98](../../crates/crysirror-sugarpot/src/adapter/geometry.rs#L98) |
| `selector` | `22` | Local state/value operations | Handled | [99](../../crates/crysirror-sugarpot/src/adapter/geometry.rs#L99) |
| `selector` | `23` | Local state/value operations | Handled | [100](../../crates/crysirror-sugarpot/src/adapter/geometry.rs#L100) |
| `selector` | `24` | Local state/value operations | Handled | [101](../../crates/crysirror-sugarpot/src/adapter/geometry.rs#L101) |
| `selector` | `25` | Local state/value operations | Handled | [102](../../crates/crysirror-sugarpot/src/adapter/geometry.rs#L102) |
| `selector` | `26` | Local state/value operations | Handled | [103](../../crates/crysirror-sugarpot/src/adapter/geometry.rs#L103) |
| `selector` | `27` | Local state/value operations | Handled | [104](../../crates/crysirror-sugarpot/src/adapter/geometry.rs#L104) |
| `selector` | `_` | Local state/value operations | Handled | [105](../../crates/crysirror-sugarpot/src/adapter/geometry.rs#L105) |

### adapter/geometry.rs / whole

```rust
fn whole(selector: i32, p: i32) -> RectMapRegion
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `selector` | `20` | Local state/value operations | Handled | [37](../../crates/crysirror-sugarpot/src/adapter/geometry.rs#L37) |
| `selector` | `21` | Local state/value operations | Handled | [38](../../crates/crysirror-sugarpot/src/adapter/geometry.rs#L38) |
| `selector` | `22` | Local state/value operations | Handled | [39](../../crates/crysirror-sugarpot/src/adapter/geometry.rs#L39) |
| `selector` | `23` | Local state/value operations | Handled | [40](../../crates/crysirror-sugarpot/src/adapter/geometry.rs#L40) |
| `selector` | `24` | Local state/value operations | Handled | [41](../../crates/crysirror-sugarpot/src/adapter/geometry.rs#L41) |
| `selector` | `25` | Local state/value operations | Handled | [42](../../crates/crysirror-sugarpot/src/adapter/geometry.rs#L42) |
| `selector` | `26` | Local state/value operations | Handled | [43](../../crates/crysirror-sugarpot/src/adapter/geometry.rs#L43) |
| `selector` | `27` | Local state/value operations | Handled | [44](../../crates/crysirror-sugarpot/src/adapter/geometry.rs#L44) |
| `selector` | `_` | Local state/value operations | Handled | [45](../../crates/crysirror-sugarpot/src/adapter/geometry.rs#L45) |

### adapter/geometry.rs / SugarPotVm::begin_geometry

```rust
fn begin_geometry(
        &mut self,
        h: &mut dyn HostServices,
        args: &[String],
        selector: i32,
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `selector == 28` | Local state/value operations | Handled | [148](../../crates/crysirror-sugarpot/src/adapter/geometry.rs#L148) |

### adapter/geometry.rs / SugarPotVm::poll_geometry

```rust
fn poll_geometry(
        &mut self,
        h: &mut dyn HostServices,
        work: &mut Geometry,
        skip: bool,
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `work.selector == 28` | Local state/value operations | Handled | [285](../../crates/crysirror-sugarpot/src/adapter/geometry.rs#L285) |

### adapter/menu.rs / SugarPotVm::poll_settings

```rust
fn poll_settings(
        &mut self,
        h: &mut dyn HostServices,
        input: &mut InputFrame,
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `id == CLOSE as u64 && event["kind"] == "activate"` | Local state/value operations | Handled | [70](../../crates/crysirror-sugarpot/src/adapter/menu.rs#L70) |
| `condition` | `event["kind"] != "change" \|\| !(64200..64200 + SETTINGS.len() as u64).contains(&id)` | Local state/value operations | Handled | [74](../../crates/crysirror-sugarpot/src/adapter/menu.rs#L74) |

### adapter/motion.rs / SugarPotVm::ef

```rust
fn ef(&mut self, h: &mut dyn HostServices, args: &[String]) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(selector, 1..=9)` | self.ef_movement | Handled | [369](../../crates/crysirror-sugarpot/src/adapter/motion.rs#L369) |
| `condition` | `matches!(selector, 1..=9)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [369](../../crates/crysirror-sugarpot/src/adapter/motion.rs#L369) |
| `condition` | `matches!(selector, 20..=28)` | self.begin_geometry | Handled | [372](../../crates/crysirror-sugarpot/src/adapter/motion.rs#L372) |
| `condition` | `matches!(selector, 20..=28)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [372](../../crates/crysirror-sugarpot/src/adapter/motion.rs#L372) |
| `condition` | `selector == 30` | self.begin_rotation | Handled | [375](../../crates/crysirror-sugarpot/src/adapter/motion.rs#L375) |
| `condition` | `selector == 29` | self.begin_warp | Handled | [378](../../crates/crysirror-sugarpot/src/adapter/motion.rs#L378) |
| `condition` | `matches!(selector, 11 \| 12 \| 16 \| 17)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [388](../../crates/crysirror-sugarpot/src/adapter/motion.rs#L388) |
| `condition` | `matches!(selector, 11 \| 12)` | Local state/value operations | Handled | [390](../../crates/crysirror-sugarpot/src/adapter/motion.rs#L390) |
| `condition` | `matches!(selector, 11 \| 12)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [390](../../crates/crysirror-sugarpot/src/adapter/motion.rs#L390) |
| `condition` | `target != -1 && target != id as i32` | Local state/value operations | Handled | [422](../../crates/crysirror-sugarpot/src/adapter/motion.rs#L422) |
| `condition` | `target == -1 && sprite.system != 0 && !(fixed && id == 0)` | Local state/value operations | Handled | [425](../../crates/crysirror-sugarpot/src/adapter/motion.rs#L425) |
| `selector` | `11` | Local state/value operations | Handled | [438](../../crates/crysirror-sugarpot/src/adapter/motion.rs#L438) |
| `selector` | `12` | Local state/value operations | Handled | [439](../../crates/crysirror-sugarpot/src/adapter/motion.rs#L439) |
| `selector` | `16` | Local state/value operations | Handled | [440](../../crates/crysirror-sugarpot/src/adapter/motion.rs#L440) |
| `selector` | `17` | Local state/value operations | Handled | [441](../../crates/crysirror-sugarpot/src/adapter/motion.rs#L441) |
| `selector` | `_` | Local state/value operations | Handled | [442](../../crates/crysirror-sugarpot/src/adapter/motion.rs#L442) |
| `condition` | `target == -1 && selector == 19` | Local state/value operations | Handled | [467](../../crates/crysirror-sugarpot/src/adapter/motion.rs#L467) |

### adapter/motion.rs / SugarPotVm::ef_movement

```rust
fn ef_movement(
        &mut self,
        h: &mut dyn HostServices,
        args: &[String],
        selector: i32,
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(selector, 6 \| 8)` | self.visual | Handled | [169](../../crates/crysirror-sugarpot/src/adapter/motion.rs#L169) |
| `condition` | `matches!(selector, 6 \| 8)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [169](../../crates/crysirror-sugarpot/src/adapter/motion.rs#L169) |
| `condition` | `matches!(selector, 8 \| 9)` | Local state/value operations | Handled | [194](../../crates/crysirror-sugarpot/src/adapter/motion.rs#L194) |
| `condition` | `matches!(selector, 8 \| 9)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [194](../../crates/crysirror-sugarpot/src/adapter/motion.rs#L194) |
| `condition` | `matches!(selector, 8 \| 9)` | Local state/value operations | Handled | [201](../../crates/crysirror-sugarpot/src/adapter/motion.rs#L201) |
| `condition` | `matches!(selector, 8 \| 9)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [201](../../crates/crysirror-sugarpot/src/adapter/motion.rs#L201) |
| `condition` | `selector <= 4` | usize::from | Handled | [203](../../crates/crysirror-sugarpot/src/adapter/motion.rs#L203) |
| `condition` | `matches!(selector, 1 \| 3)` | Local state/value operations | Handled | [206](../../crates/crysirror-sugarpot/src/adapter/motion.rs#L206) |
| `condition` | `matches!(selector, 1 \| 3)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [206](../../crates/crysirror-sugarpot/src/adapter/motion.rs#L206) |

### adapter/motion.rs / SugarPotVm::ef_movement_frame

```rust
fn ef_movement_frame(
        &mut self,
        h: &mut dyn HostServices,
        motion: &EfMovement,
        skip: bool,
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `motion.selector <= 4` | Local state/value operations | Handled | [252](../../crates/crysirror-sugarpot/src/adapter/motion.rs#L252) |
| `condition` | `matches!(motion.selector, 7 \| 9)` | Local state/value operations | Handled | [256](../../crates/crysirror-sugarpot/src/adapter/motion.rs#L256) |
| `condition` | `motion.selector <= 4` | usize::from, self.submit | Handled | [271](../../crates/crysirror-sugarpot/src/adapter/motion.rs#L271) |
| `condition` | `matches!(motion.selector, 7 \| 9)` | Local state/value operations | Handled | [284](../../crates/crysirror-sugarpot/src/adapter/motion.rs#L284) |
| `condition` | `matches!(motion.selector, 8 \| 9)` | Self::sprite_opacity, Vec::new, Self::sprite_layer, self.submit, self.published_layers.contains | Handled | [289](../../crates/crysirror-sugarpot/src/adapter/motion.rs#L289) |
| `condition` | `motion.selector == 8` | Local state/value operations | Handled | [296](../../crates/crysirror-sugarpot/src/adapter/motion.rs#L296) |
| `condition` | `motion.selector == 8` | Local state/value operations | Handled | [301](../../crates/crysirror-sugarpot/src/adapter/motion.rs#L301) |
| `condition` | `motion.id == 0` | Local state/value operations | Handled | [307](../../crates/crysirror-sugarpot/src/adapter/motion.rs#L307) |
| `condition` | `motion.id == 0 && sprite.native_depth == 24` | Local state/value operations | Handled | [325](../../crates/crysirror-sugarpot/src/adapter/motion.rs#L325) |
| `motion.selector` | `6` | Local state/value operations | Handled | [349](../../crates/crysirror-sugarpot/src/adapter/motion.rs#L349) |
| `motion.selector` | `7` | Local state/value operations | Handled | [350](../../crates/crysirror-sugarpot/src/adapter/motion.rs#L350) |
| `motion.selector` | `_` | Local state/value operations | Handled | [351](../../crates/crysirror-sugarpot/src/adapter/motion.rs#L351) |

### adapter/motion.rs / SugarPotVm::efex

```rust
fn efex(&mut self, h: &mut dyn HostServices, args: &[String]) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under selector=11 \| 12 \| 13` | `selector == 12` | self.visual | Handled | [17](../../crates/crysirror-sugarpot/src/adapter/motion.rs#L17) |
| `selector` | `11 \| 12 \| 13` | usize::try_from, self.number, self.visual, self.sprites.get, self.moves.len | Handled | [15](../../crates/crysirror-sugarpot/src/adapter/motion.rs#L15) |
| `selector` | `1 \| 2 \| 3` | self.number, self.publish, h.now_ms, self.motion_frame | Handled | [49](../../crates/crysirror-sugarpot/src/adapter/motion.rs#L49) |
| `selector` | `_` | Local state/value operations | Unsupported: SugarPot efex selector {selector} has no reviewed portable execution profile | [70](../../crates/crysirror-sugarpot/src/adapter/motion.rs#L70) |

### adapter/motion.rs / SugarPotVm::motion_frame

```rust
fn motion_frame(
        &mut self,
        h: &mut dyn HostServices,
        elapsed: u64,
        duration: u64,
        kind: i32,
    ) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `kind` | `2` | Local state/value operations | Handled | [110](../../crates/crysirror-sugarpot/src/adapter/motion.rs#L110) |
| `kind` | `3` | Local state/value operations | Handled | [111](../../crates/crysirror-sugarpot/src/adapter/motion.rs#L111) |
| `kind` | `_` | Local state/value operations | Handled | [112](../../crates/crysirror-sugarpot/src/adapter/motion.rs#L112) |
| `condition` | `movement.kind == 13` | Local state/value operations | Handled | [133](../../crates/crysirror-sugarpot/src/adapter/motion.rs#L133) |

### adapter/raster.rs / SugarPotVm::raster_command

```rust
fn raster_command(
        &mut self,
        args: &[String],
        h: &mut dyn HostServices,
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `selector == 0` | Local state/value operations | Handled | [171](../../crates/crysirror-sugarpot/src/adapter/raster.rs#L171) |
| `condition` | `selector == 1` | Local state/value operations | Handled | [175](../../crates/crysirror-sugarpot/src/adapter/raster.rs#L175) |
| `condition` | `selector == 2` | Local state/value operations | Handled | [177](../../crates/crysirror-sugarpot/src/adapter/raster.rs#L177) |

### adapter/recollection.rs / Recollection::available

```rust
fn available(&self, id: usize) -> bool
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `id` | `0..=9` | self.groups.len | Handled | [82](../../crates/crysirror-sugarpot/src/adapter/recollection.rs#L82) |
| `id` | `10..=19` | self.pages | Handled | [83](../../crates/crysirror-sugarpot/src/adapter/recollection.rs#L83) |
| `id` | `20` | Local state/value operations | Handled | [84](../../crates/crysirror-sugarpot/src/adapter/recollection.rs#L84) |
| `id` | `21..=29` | Local state/value operations | Handled | [85](../../crates/crysirror-sugarpot/src/adapter/recollection.rs#L85) |
| `id` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [86](../../crates/crysirror-sugarpot/src/adapter/recollection.rs#L86) |

### adapter/recollection.rs / Recollection::source

```rust
fn source(&self, id: usize, state: u8) -> (String, [i32; 2])
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `id >= 21` | Local state/value operations | Handled | [91](../../crates/crysirror-sugarpot/src/adapter/recollection.rs#L91) |

### adapter/recollection.rs / SugarPotVm::h_direction

```rust
fn h_direction(mode: &Recollection, id: usize, direction: &str) -> Option<usize>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `id == 255` | Local state/value operations | Handled | [413](../../crates/crysirror-sugarpot/src/adapter/recollection.rs#L413) |
| `id; under direction="h_up"` | `1..=9` | Local state/value operations | Handled | [418](../../crates/crysirror-sugarpot/src/adapter/recollection.rs#L418) |
| `id; under direction="h_up"` | `10..=20` | Local state/value operations | Handled | [419](../../crates/crysirror-sugarpot/src/adapter/recollection.rs#L419) |
| `id; under direction="h_up"` | `24..=29` | Local state/value operations | Handled | [420](../../crates/crysirror-sugarpot/src/adapter/recollection.rs#L420) |
| `id; under direction="h_up"` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [421](../../crates/crysirror-sugarpot/src/adapter/recollection.rs#L421) |
| `condition; under direction="h_down" / id=27..=29` | `mode.pages() > 1` | Local state/value operations | Handled | [428](../../crates/crysirror-sugarpot/src/adapter/recollection.rs#L428) |
| `id; under direction="h_down"` | `0..=8` | Local state/value operations | Handled | [424](../../crates/crysirror-sugarpot/src/adapter/recollection.rs#L424) |
| `id; under direction="h_down"` | `10..=19` | Local state/value operations | Handled | [425](../../crates/crysirror-sugarpot/src/adapter/recollection.rs#L425) |
| `id; under direction="h_down"` | `21..=26` | Local state/value operations | Handled | [426](../../crates/crysirror-sugarpot/src/adapter/recollection.rs#L426) |
| `id; under direction="h_down"` | `27..=29` | Local state/value operations | Handled | [427](../../crates/crysirror-sugarpot/src/adapter/recollection.rs#L427) |
| `id; under direction="h_down"` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [434](../../crates/crysirror-sugarpot/src/adapter/recollection.rs#L434) |
| `id; under direction="h_left"` | `11..=19` | Local state/value operations | Handled | [437](../../crates/crysirror-sugarpot/src/adapter/recollection.rs#L437) |
| `id; under direction="h_left"` | `21..=29 if (id - 21) % 3 != 0` | Local state/value operations | Handled | [438](../../crates/crysirror-sugarpot/src/adapter/recollection.rs#L438) |
| `id; under direction="h_left"` | `21..=29` | Local state/value operations | Handled | [439](../../crates/crysirror-sugarpot/src/adapter/recollection.rs#L439) |
| `id; under direction="h_left"` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [440](../../crates/crysirror-sugarpot/src/adapter/recollection.rs#L440) |
| `id; under direction="h_right"` | `0..=9` | Local state/value operations | Handled | [443](../../crates/crysirror-sugarpot/src/adapter/recollection.rs#L443) |
| `id; under direction="h_right"` | `10..=18` | Local state/value operations | Handled | [444](../../crates/crysirror-sugarpot/src/adapter/recollection.rs#L444) |
| `id; under direction="h_right"` | `21..=29 if (id - 21) % 3 != 2` | Local state/value operations | Handled | [445](../../crates/crysirror-sugarpot/src/adapter/recollection.rs#L445) |
| `id; under direction="h_right"` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [446](../../crates/crysirror-sugarpot/src/adapter/recollection.rs#L446) |

### adapter/recollection.rs / SugarPotVm::paint_recollection

```rust
fn paint_recollection(
        &mut self,
        h: &mut dyn HostServices,
        mode: &mut Recollection,
    ) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `mode.gallery && id >= 21` | Local state/value operations | Handled | [385](../../crates/crysirror-sugarpot/src/adapter/recollection.rs#L385) |

### adapter/recollection.rs / SugarPotVm::poll_recollection_inner

```rust
fn poll_recollection_inner(
        &mut self,
        h: &mut dyn HostServices,
        input: &mut InputFrame,
        mode: &mut Recollection,
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `event["kind"].as_str().unwrap_or("")` | `"h_cancel"` | Local state/value operations | Handled | [579](../../crates/crysirror-sugarpot/src/adapter/recollection.rs#L579) |
| `event["kind"].as_str().unwrap_or("")` | `"h_activate"` | Local state/value operations | Handled | [580](../../crates/crysirror-sugarpot/src/adapter/recollection.rs#L580) |
| `event["kind"].as_str().unwrap_or("")` | `key @ ("h_up" \| "h_down" \| "h_left" \| "h_right")` | Self::h_direction | Handled | [585](../../crates/crysirror-sugarpot/src/adapter/recollection.rs#L585) |
| `event["kind"].as_str().unwrap_or("")` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [588](../../crates/crysirror-sugarpot/src/adapter/recollection.rs#L588) |
| `id` | `0..=9 if id != mode.category` | Local state/value operations | Handled | [611](../../crates/crysirror-sugarpot/src/adapter/recollection.rs#L611) |
| `id` | `10..=19 if id - 10 != mode.page` | Local state/value operations | Handled | [616](../../crates/crysirror-sugarpot/src/adapter/recollection.rs#L616) |
| `id` | `21..=29` | gallery::Viewer::new, self.close_recollection | Handled | [620](../../crates/crysirror-sugarpot/src/adapter/recollection.rs#L620) |
| `id` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [631](../../crates/crysirror-sugarpot/src/adapter/recollection.rs#L631) |

### adapter/recollection.rs / SugarPotVm::recollection_icon

```rust
fn recollection_icon(&self, mode: &Recollection, id: usize, state: u8) -> Result<Json>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `name == "@thumbnails"` | Local state/value operations | Handled | [320](../../crates/crysirror-sugarpot/src/adapter/recollection.rs#L320) |
| `condition` | `id >= 21 && state < 2` | Local state/value operations | Handled | [325](../../crates/crysirror-sugarpot/src/adapter/recollection.rs#L325) |

### adapter/recollection/gallery.rs / SugarPotVm::gallery_view

```rust
fn gallery_view(
        &mut self,
        h: &mut dyn HostServices,
        input: &mut InputFrame,
        view: &mut Viewer,
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `self .config .assets .contains_key(&format!("grp/{}.bmp", key.to_ascii_lowercase()))` | Local state/value operations | Handled | [185](../../crates/crysirror-sugarpot/src/adapter/recollection/gallery.rs#L185) |

### adapter/restore.rs / Restore::advance

```rust
fn advance(&mut self, h: &mut dyn HostServices) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `vm.mode == 2` | Local state/value operations | Handled | [616](../../crates/crysirror-sugarpot/src/adapter/restore.rs#L616) |
| `condition` | `vm.mode == 0` | Local state/value operations | Handled | [661](../../crates/crysirror-sugarpot/src/adapter/restore.rs#L661) |

### adapter/restore.rs / SugarPotVm::prepare_original_restore

```rust
fn prepare_original_restore(&self, bytes: Arc<[u8]>) -> Result<Restore>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `*id` | `60001` | self.raster.as_ref | Handled | [119](../../crates/crysirror-sugarpot/src/adapter/restore.rs#L119) |
| `*id` | `60002` | self.filter.as_ref | Handled | [120](../../crates/crysirror-sugarpot/src/adapter/restore.rs#L120) |
| `*id` | `_` | self.fall.as_ref | Handled | [121](../../crates/crysirror-sugarpot/src/adapter/restore.rs#L121) |

### adapter/storage.rs / set_window

```rust
fn set_window(
    window: &mut SlotWindow,
    selector: &str,
    values: &[Value],
    image_size: Option<[u32; 2]>,
    blend: i32,
) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under selector="fp" \| "pp"` | `selector == "fp"` | Local state/value operations | Handled | [70](../../crates/crysirror-sugarpot/src/adapter/storage.rs#L70) |
| `condition; under selector="fp" \| "pp"` | `selector == "fp"` | Local state/value operations | Handled | [82](../../crates/crysirror-sugarpot/src/adapter/storage.rs#L82) |
| `selector` | `"size"` | Local state/value operations | Handled | [25](../../crates/crysirror-sugarpot/src/adapter/storage.rs#L25) |
| `selector` | `"pos"` | Local state/value operations | Handled | [29](../../crates/crysirror-sugarpot/src/adapter/storage.rs#L29) |
| `selector` | `"txt"` | Local state/value operations | Handled | [33](../../crates/crysirror-sugarpot/src/adapter/storage.rs#L33) |
| `selector` | `"cel"` | Local state/value operations | Handled | [41](../../crates/crysirror-sugarpot/src/adapter/storage.rs#L41) |
| `selector` | `"frm"` | Local state/value operations | Handled | [52](../../crates/crysirror-sugarpot/src/adapter/storage.rs#L52) |
| `selector` | `"fp" \| "pp"` | Local state/value operations | Handled | [67](../../crates/crysirror-sugarpot/src/adapter/storage.rs#L67) |
| `selector` | `"btn"` | Local state/value operations | Handled | [107](../../crates/crysirror-sugarpot/src/adapter/storage.rs#L107) |
| `selector` | `_` | Local state/value operations | Unsupported: SugarPot original window selector {selector} | [121](../../crates/crysirror-sugarpot/src/adapter/storage.rs#L121) |

### adapter/update.rs / SugarPotVm::finish_update

```rust
fn finish_update(&mut self, h: &mut dyn HostServices, work: &mut MaskUpdate) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(work.kind, 10..=13)` | Local state/value operations | Handled | [48](../../crates/crysirror-sugarpot/src/adapter/update.rs#L48) |
| `condition` | `matches!(work.kind, 15..=18)` | Local state/value operations | Handled | [51](../../crates/crysirror-sugarpot/src/adapter/update.rs#L51) |

### adapter/update.rs / SugarPotVm::mask_update

```rust
fn mask_update(
        &mut self,
        h: &mut dyn HostServices,
        args: &[String],
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(kind,10..=13\|15..=18)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [101](../../crates/crysirror-sugarpot/src/adapter/update.rs#L101) |
| `condition` | `kind == 2` | Local state/value operations | Handled | [103](../../crates/crysirror-sugarpot/src/adapter/update.rs#L103) |
| `condition` | `kind <= 13` | Local state/value operations | Handled | [128](../../crates/crysirror-sugarpot/src/adapter/update.rs#L128) |

### adapter/update.rs / SugarPotVm::mask_update_step

```rust
fn mask_update_step(
        &mut self,
        h: &mut dyn HostServices,
        args: &[String],
        work: &mut MaskUpdate,
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `work.kind <= 13` | Local state/value operations | Handled | [280](../../crates/crysirror-sugarpot/src/adapter/update.rs#L280) |
| `condition` | `(scan && (skipping \|\| update_scan::complete(work.kind, work.scan_index, work.duration as i32))) \|\| (matches!(work.kind, 1 \| 19..=39) && skipping)` | self.finish_update | Handled | [297](../../crates/crysirror-sugarpot/src/adapter/update.rs#L297) |
| `condition` | `(work.kind == 2 \|\| scan) && work.mask.is_none()` | h.now_ms, update_scan::add, update_scan::mask, h.image_metadata, serde_json::to_vec | Handled | [304](../../crates/crysirror-sugarpot/src/adapter/update.rs#L304) |
| `condition` | `work.kind == 2 && skipping` | self.finish_update | Handled | [365](../../crates/crysirror-sugarpot/src/adapter/update.rs#L365) |
| `condition` | `matches!(work.kind, 2 \| 31 \| 32 \| 34..=38)` | Local state/value operations | Handled | [380](../../crates/crysirror-sugarpot/src/adapter/update.rs#L380) |
| `condition` | `phase >= if matches!(work.kind, 2 \| 31 \| 32 \| 34..=38) { 512 } else { 256 }` | self.finish_update | Handled | [385](../../crates/crysirror-sugarpot/src/adapter/update.rs#L385) |
| `condition` | `matches!(work.kind, 2 \| 31 \| 32 \| 34..=38)` | Local state/value operations | Handled | [386](../../crates/crysirror-sugarpot/src/adapter/update.rs#L386) |
| `condition` | `matches!(work.kind, 32 \| 37 \| 38)` | f64::from, h.image_metadata, ImageInput::Resource, serde_json::to_vec | Handled | [428](../../crates/crysirror-sugarpot/src/adapter/update.rs#L428) |
| `condition` | `work.kind == 32` | Local state/value operations | Handled | [435](../../crates/crysirror-sugarpot/src/adapter/update.rs#L435) |
| `condition; under receiver=else: work.kind == 32` | `work.kind == 37` | Local state/value operations | Handled | [451](../../crates/crysirror-sugarpot/src/adapter/update.rs#L451) |
| `condition` | `work.kind == 39` | self.disc_tables.clone, self.disc_load.is_none, self.storage.as_ref, self.pool.as_ref, crate::resources::disc_tables | Handled | [469](../../crates/crysirror-sugarpot/src/adapter/update.rs#L469) |
| `condition` | `matches!(work.kind, 25 \| 26)` | update_iris::radius, update_iris::mask, h.image_metadata, serde_json::to_vec | Handled | [518](../../crates/crysirror-sugarpot/src/adapter/update.rs#L518) |
| `condition` | `work.kind == 25` | Local state/value operations | Handled | [539](../../crates/crysirror-sugarpot/src/adapter/update.rs#L539) |
| `condition` | `work.kind == 1` | h.image_metadata, serde_json::to_vec | Handled | [565](../../crates/crysirror-sugarpot/src/adapter/update.rs#L565) |
| `condition` | `matches!(work.kind, 19..=24)` | update_zoom::input, h.image_metadata, serde_json::to_vec | Handled | [594](../../crates/crysirror-sugarpot/src/adapter/update.rs#L594) |
| `condition` | `matches!(work.kind, 19 \| 21 \| 23)` | Local state/value operations | Handled | [597](../../crates/crysirror-sugarpot/src/adapter/update.rs#L597) |
| `condition` | `matches!(work.kind, 29 \| 30)` | update_bands::input, h.image_metadata, serde_json::to_vec | Handled | [619](../../crates/crysirror-sugarpot/src/adapter/update.rs#L619) |
| `condition` | `matches!(work.kind, 27 \| 28)` | update_angular::input, h.image_metadata, serde_json::to_vec | Handled | [637](../../crates/crysirror-sugarpot/src/adapter/update.rs#L637) |
| `condition` | `work.kind == 31` | update_mosaic::input, h.image_metadata, serde_json::to_vec | Handled | [655](../../crates/crysirror-sugarpot/src/adapter/update.rs#L655) |
| `condition` | `matches!(work.kind, 35 \| 36)` | update_squeeze::input, h.image_metadata, serde_json::to_vec | Handled | [672](../../crates/crysirror-sugarpot/src/adapter/update.rs#L672) |
| `condition` | `work.kind == 34` | update_wave::input, h.image_metadata, serde_json::to_vec | Handled | [690](../../crates/crysirror-sugarpot/src/adapter/update.rs#L690) |
| `condition` | `work.kind == 33` | update_checker::input, h.image_metadata, serde_json::to_vec | Handled | [709](../../crates/crysirror-sugarpot/src/adapter/update.rs#L709) |

### adapter/update_iris.rs / radius

```rust
fn radius(kind: i32, phase: u32) -> i32
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `kind == 25` | f64::from | Handled | [18](../../crates/crysirror-sugarpot/src/adapter/update_iris.rs#L18) |

### adapter/update_scan.rs / add

```rust
fn add(kind: i32, index: i32, stride: i32, stripes: &mut Stripes) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `kind <= 13` | Local state/value operations | Handled | [20](../../crates/crysirror-sugarpot/src/adapter/update_scan.rs#L20) |
| `condition` | `matches!(kind, 11 \| 13)` | Local state/value operations | Handled | [23](../../crates/crysirror-sugarpot/src/adapter/update_scan.rs#L23) |
| `condition` | `matches!(kind, 11 \| 13)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [23](../../crates/crysirror-sugarpot/src/adapter/update_scan.rs#L23) |
| `condition` | `kind <= 16` | Local state/value operations | Handled | [41](../../crates/crysirror-sugarpot/src/adapter/update_scan.rs#L41) |
| `condition` | `matches!(kind, 16 \| 18)` | i64::from | Handled | [44](../../crates/crysirror-sugarpot/src/adapter/update_scan.rs#L44) |
| `condition` | `matches!(kind, 16 \| 18)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [44](../../crates/crysirror-sugarpot/src/adapter/update_scan.rs#L44) |
| `condition` | `(matches!(kind, 16 \| 18) && at < 0) \|\| (!matches!(kind, 16 \| 18) && at >= i64::from(extent))` | Local state/value operations | Handled | [52](../../crates/crysirror-sugarpot/src/adapter/update_scan.rs#L52) |
| `condition` | `matches!(kind, 16 \| 18)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [52](../../crates/crysirror-sugarpot/src/adapter/update_scan.rs#L52) |
| `condition` | `matches!(kind, 16 \| 18)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [53](../../crates/crysirror-sugarpot/src/adapter/update_scan.rs#L53) |
| `condition` | `kind <= 16` | Local state/value operations | Handled | [59](../../crates/crysirror-sugarpot/src/adapter/update_scan.rs#L59) |
| `condition` | `kind == 15 && (-1024..0).contains(&at)` | Local state/value operations | Handled | [64](../../crates/crysirror-sugarpot/src/adapter/update_scan.rs#L64) |

### adapter/update_scan.rs / complete

```rust
fn complete(kind: i32, index: i32, stride: i32) -> bool
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `kind <= 13` | Local state/value operations | Handled | [9](../../crates/crysirror-sugarpot/src/adapter/update_scan.rs#L9) |
| `condition` | `kind <= 16` | Local state/value operations | Handled | [12](../../crates/crysirror-sugarpot/src/adapter/update_scan.rs#L12) |

### adapter/update_squeeze.rs / input

```rust
fn input(
    kind: i32,
    phase: u32,
    old: Arc<dyn ImageResource>,
    target: Arc<dyn ImageResource>,
) -> Result<ImageInput>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `kind == 35` | Local state/value operations | Handled | [25](../../crates/crysirror-sugarpot/src/adapter/update_squeeze.rs#L25) |

### adapter/update_zoom.rs / input

```rust
fn input(
    kind: i32,
    phase: u32,
    old: Arc<dyn ImageResource>,
    target: Arc<dyn ImageResource>,
) -> Result<ImageInput>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(kind, 19 \| 21 \| 23)` | Local state/value operations | Handled | [24](../../crates/crysirror-sugarpot/src/adapter/update_zoom.rs#L24) |
| `condition` | `matches!(kind, 19 \| 21 \| 23)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [24](../../crates/crysirror-sugarpot/src/adapter/update_zoom.rs#L24) |
| `condition` | `matches!(kind, 23 \| 24)` | f64::from, Box::new, ImageInput::Resource, serde_json::to_vec | Handled | [25](../../crates/crysirror-sugarpot/src/adapter/update_zoom.rs#L25) |
| `condition` | `matches!(kind, 23 \| 24)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [25](../../crates/crysirror-sugarpot/src/adapter/update_zoom.rs#L25) |
| `kind` | `19 \| 23` | f64::from | Handled | [60](../../crates/crysirror-sugarpot/src/adapter/update_zoom.rs#L60) |
| `kind` | `21` | f64::from | Handled | [61](../../crates/crysirror-sugarpot/src/adapter/update_zoom.rs#L61) |
| `kind` | `22` | f64::from | Handled | [62](../../crates/crysirror-sugarpot/src/adapter/update_zoom.rs#L62) |
| `kind` | `_` | f64::from | Handled | [63](../../crates/crysirror-sugarpot/src/adapter/update_zoom.rs#L63) |

### adapter/window_system.rs / SugarPotVm::system_window_layers

```rust
fn system_window_layers(&self, visible: bool) -> Result<Vec<Json>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Some(mut layer) = Self::sprite_layer(id, sprite, 2000 + index as i32)?` | Local state/value operations | Handled | [14](../../crates/crysirror-sugarpot/src/adapter/window_system.rs#L14) |

### adapter/window_ui.rs / SugarPotVm::poll_window_controls

```rust
fn poll_window_controls(
        &mut self,
        h: &mut dyn HostServices,
        input: &mut InputFrame,
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Some(id) = op["layer"]["id"].as_u64()` | self.window_ui.layers.insert | Handled | [255](../../crates/crysirror-sugarpot/src/adapter/window_ui.rs#L255) |
| `condition` | `let Some(id) = op["id"].as_u64()` | self.window_ui.layers.remove | Handled | [257](../../crates/crysirror-sugarpot/src/adapter/window_ui.rs#L257) |

### expression.rs / condition_banks

```rust
fn condition_banks(
    source: &str,
    vars: &BTreeMap<String, Value>,
    defs: &BTreeMap<String, Value>,
) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `op` | `"=="` | Local state/value operations | Handled | [182](../../crates/crysirror-sugarpot/src/expression.rs#L182) |
| `op` | `"!="` | Local state/value operations | Handled | [183](../../crates/crysirror-sugarpot/src/expression.rs#L183) |
| `op` | `">="` | Local state/value operations | Handled | [184](../../crates/crysirror-sugarpot/src/expression.rs#L184) |
| `op` | `"<="` | Local state/value operations | Handled | [185](../../crates/crysirror-sugarpot/src/expression.rs#L185) |
| `op` | `">"` | Local state/value operations | Handled | [186](../../crates/crysirror-sugarpot/src/expression.rs#L186) |
| `op` | `"<"` | Local state/value operations | Handled | [187](../../crates/crysirror-sugarpot/src/expression.rs#L187) |
| `op` | `_` | Local state/value operations | Handled | [188](../../crates/crysirror-sugarpot/src/expression.rs#L188) |

### expression.rs / Parser<'_>::atom

```rust
fn atom(&mut self) -> Result<i32>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Some(hex) = name.strip_prefix("0x").or_else(\|\| name.strip_prefix("0X"))` | Local state/value operations | Handled | [294](../../crates/crysirror-sugarpot/src/expression.rs#L294) |

### machine.rs / Machine::assignment

```rust
fn assignment(&mut self, line: &str) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `op == "="` | Local state/value operations | Handled | [306](../../crates/crysirror-sugarpot/src/machine.rs#L306) |
| `condition` | `op == "+=" && line.starts_with('$')` | Value::String, String::new | Handled | [308](../../crates/crysirror-sugarpot/src/machine.rs#L308) |
| `op` | `"+="` | Local state/value operations | Handled | [324](../../crates/crysirror-sugarpot/src/machine.rs#L324) |
| `op` | `"-="` | Local state/value operations | Handled | [325](../../crates/crysirror-sugarpot/src/machine.rs#L325) |
| `op` | `"*="` | Local state/value operations | Handled | [326](../../crates/crysirror-sugarpot/src/machine.rs#L326) |
| `op` | `"/="` | Local state/value operations | Handled | [327](../../crates/crysirror-sugarpot/src/machine.rs#L327) |
| `op` | `"%="` | Local state/value operations | Handled | [331](../../crates/crysirror-sugarpot/src/machine.rs#L331) |
| `op` | `"&="` | Local state/value operations | Handled | [335](../../crates/crysirror-sugarpot/src/machine.rs#L335) |
| `op` | `"\|="` | Local state/value operations | Handled | [336](../../crates/crysirror-sugarpot/src/machine.rs#L336) |
| `op` | `"^="` | Local state/value operations | Handled | [337](../../crates/crysirror-sugarpot/src/machine.rs#L337) |
| `op` | `_` | Local state/value operations | Unsupported: unsupported SugarPot assignment | [338](../../crates/crysirror-sugarpot/src/machine.rs#L338) |

### machine.rs / Machine::event

```rust
fn event(&mut self) -> Result<Option<Event>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `self.state.frame.script.starts_with("#macro:") && self.state.frame.pc >= self.programs[&self.state.frame.script].instructions.len()` | self.state.macro_stack.len | Handled | [354](../../crates/crysirror-sugarpot/src/machine.rs#L354) |
| `condition; under name="go" \| "gsb" \| "jp" \| "jsb"` | `matches!(name, "jp" \| "jsb")` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [391](../../crates/crysirror-sugarpot/src/machine.rs#L391) |
| `condition; under name="go" \| "gsb" \| "jp" \| "jsb"` | `matches!(name, "gsb" \| "jsb")` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [392](../../crates/crysirror-sugarpot/src/machine.rs#L392) |
| `condition; under name="if"` | `name != "elif" \|\| expression::condition_banks( args.first().context("SugarPot elif condition")?, &self.state.variables, &self.state.definitions, )?` | Local state/value operations | Handled | [429](../../crates/crysirror-sugarpot/src/machine.rs#L429) |
| `condition; under name="sw"` | `name != "case" \|\| self.value(args.first().context("SugarPot case value")?)? == value` | Local state/value operations | Handled | [468](../../crates/crysirror-sugarpot/src/machine.rs#L468) |
| `name` | `_` | Local state/value operations | Unsupported: unsupported SugarPot native control ^{name} | [543](../../crates/crysirror-sugarpot/src/machine.rs#L543) |

### machine.rs / Machine::key

```rust
fn key(name: &str) -> Result<String>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `name.ends_with(".scr")` | Local state/value operations | Handled | [660](../../crates/crysirror-sugarpot/src/machine.rs#L660) |

### machine.rs / Machine::scan

```rust
fn scan(&self, start: usize, opening: &str, closing: &str, branches: &[&str]) -> Result<usize>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `depth == 0 && branches.contains(&name)` | Local state/value operations | Handled | [270](../../crates/crysirror-sugarpot/src/machine.rs#L270) |

### native_storage/slot.rs / variable_block

```rust
fn variable_block(c: &mut Cursor<'_>, end: &[u8; 4]) -> Result<Vec<Variable>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `kind == 42` | Vec::new | Handled | [520](../../crates/crysirror-sugarpot/src/native_storage/slot.rs#L520) |

### program.rs / Program::parse

```rust
fn parse(name: &str, bytes: &[u8]) -> Result<Self>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Some(key) = source.strip_prefix("::")` | Local state/value operations | Handled | [192](../../crates/crysirror-sugarpot/src/program.rs#L192) |

### resources.rs / programs

```rust
fn programs(
    storage: Arc<dyn ClientStorage>,
    config: Config,
    client: ClientId,
    cancel: &Cancellation,
) -> Result<BTreeMap<String, Arc<Program>>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!name.ends_with(".scr")` | Local state/value operations | Handled | [75](../../crates/crysirror-sugarpot/src/resources.rs#L75) |

### tone.rs / effects

```rust
fn effects(mode: i32) -> Vec<ImageEffect>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(mode, 1 \| 2) \|\| !(3..=8).contains(&mode)` | Local state/value operations | Handled | [9](../../crates/crysirror-sugarpot/src/tone.rs#L9) |
| `condition` | `mode == 2` | Local state/value operations | Handled | [12](../../crates/crysirror-sugarpot/src/tone.rs#L12) |
| `(mode, channel)` | `(3, 1 \| 2)` | Local state/value operations | Handled | [39](../../crates/crysirror-sugarpot/src/tone.rs#L39) |
| `(mode, channel)` | `(4, 0 \| 1)` | Local state/value operations | Handled | [40](../../crates/crysirror-sugarpot/src/tone.rs#L40) |
| `(mode, channel)` | `(4, 2)` | Local state/value operations | Handled | [41](../../crates/crysirror-sugarpot/src/tone.rs#L41) |
| `(mode, channel)` | `(5, _)` | Local state/value operations | Handled | [42](../../crates/crysirror-sugarpot/src/tone.rs#L42) |
| `(mode, channel)` | `(6, 0)` | Local state/value operations | Handled | [43](../../crates/crysirror-sugarpot/src/tone.rs#L43) |
| `(mode, channel)` | `(6, 1)` | Local state/value operations | Handled | [44](../../crates/crysirror-sugarpot/src/tone.rs#L44) |
| `(mode, channel)` | `(7, 0 \| 2)` | Local state/value operations | Handled | [45](../../crates/crysirror-sugarpot/src/tone.rs#L45) |
| `(mode, channel)` | `(7, 1)` | Local state/value operations | Handled | [46](../../crates/crysirror-sugarpot/src/tone.rs#L46) |
| `(mode, channel)` | `(8, 0)` | Local state/value operations | Handled | [47](../../crates/crysirror-sugarpot/src/tone.rs#L47) |
| `(mode, channel)` | `(8, 1 \| 2)` | Local state/value operations | Handled | [48](../../crates/crysirror-sugarpot/src/tone.rs#L48) |
| `(mode, channel)` | `_` | Local state/value operations | Handled | [49](../../crates/crysirror-sugarpot/src/tone.rs#L49) |

### values.rs / Machine::value_command

```rust
fn value_command(&mut self, name: &str, args: &[String]) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `["nop", "pick", "insp", "delp", "swap", "dummy"].contains(&name)` | Local state/value operations | Handled | [10](../../crates/crysirror-sugarpot/src/values.rs#L10) |
| `condition; under name="enque" \| "push"` | `name == "push"` | Local state/value operations | Handled | [76](../../crates/crysirror-sugarpot/src/values.rs#L76) |
| `condition; under name="deque" \| "pop"` | `name == "pop"` | Local state/value operations | Handled | [97](../../crates/crysirror-sugarpot/src/values.rs#L97) |
| `condition; under name="deque" \| "pop"` | `name == "pop"` | Local state/value operations | Handled | [107](../../crates/crysirror-sugarpot/src/values.rs#L107) |
| `condition; under name="max" \| "min"` | `extreme.is_none_or(\|old\| { if name == "max" { word > old } else { word < old } })` | Local state/value operations | Handled | [135](../../crates/crysirror-sugarpot/src/values.rs#L135) |
| `condition; under name="max" \| "min"` | `name == "max"` | Local state/value operations | Handled | [136](../../crates/crysirror-sugarpot/src/values.rs#L136) |
| `name` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [226](../../crates/crysirror-sugarpot/src/values.rs#L226) |
