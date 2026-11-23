# NScripter developer reference

[All engines](README.md)

Mode: `adopt`.

## ABI and host contract

The VM executes text commands, labels and expressions. A command name and its
argument grammar are the API identity; aliases are listed separately. Builtin
lookup, user subroutines and expression operators use different namespaces.
Native numeric/string/array variables and call/loop stacks stay in the VM.

| Native area | CrySirror service / implementation | Return / completion |
| --- | --- | --- |
| Commands, labels and expressions | `lib.rs`, `commands.rs`, `program.rs` | Interpreter control flow; unknown final command routes fail. |
| Images, sprites, screen copies and effects | Shared image jobs, Canvas and retained Layers | Resource preparation and captures retain native pending state. |
| Text, ruby, buttons and menus | GlyphRun and retained UI | Native reveal, csel/button result, lookback, skip/auto policy. |
| Sound / voice / movies | Shared Audio and Media | Channel status and waits resume through native command state. |
| Archives / files / slots | Shared resources and client Save storage | Portable slot format, independent of original `.dat` saves. |

SAR/NSA resources are mounted without bulk extraction; loose resources and
numbered patch archives retain their native priority. `exec_dll`, `luacall`,
`luasub` and unsupported `drawsp3` profiles do not execute external native/Lua
code. Other unsupported argument profiles remain explicit in their handlers.
The registration of an alias is not evidence for every ONScripter extension.

Source/provenance: [SOURCE.json](../../crates/crysirror-nscripter/SOURCE.json),
[commands](../../crates/crysirror-nscripter/src/commands.rs),
[interpreter](../../crates/crysirror-nscripter/src/lib.rs).
No new execution evidence is implied by this command inventory.

<!-- BEGIN GENERATED DISPATCH TABLES -->

## Opcode and API implementation tables

`Handled` means an executable source branch exists. It is not a claim that every argument/subselector or a complete game has been audited. Explicit gaps are shown per row.

| Status | Entries |
| --- | ---: |
| Empty handler | 3 |
| Handled | 335 |
| Unsupported | 3 |

These tables cover the checked-in catalogues and explicit dispatch branches. An engine-wide upstream denominator is not inferred from these rows.

## Script commands

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `add` | add | Handled | — | [lib.rs:891](../../crates/crysirror-nscripter/src/lib.rs#L891) |
| `dec` | dec | Handled | — | [lib.rs:892](../../crates/crysirror-nscripter/src/lib.rs#L892) |
| `bdef` | bdef | Handled | — | [commands.rs:1307](../../crates/crysirror-nscripter/src/commands.rs#L1307) |
| `abssetcursor` | abssetcursor | Handled | — | [native.rs:631](../../crates/crysirror-nscripter/src/native.rs#L631) |
| `addkinsoku` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `allsp2hide` | allsp2hide | Handled | — | [commands.rs:987](../../crates/crysirror-nscripter/src/commands.rs#L987) |
| `allsp2resume` | allsp2resume | Handled | — | [commands.rs:987](../../crates/crysirror-nscripter/src/commands.rs#L987) |
| `allsphide` | allsphide | Handled | — | [commands.rs:983](../../crates/crysirror-nscripter/src/commands.rs#L983) |
| `allspresume` | allspresume | Handled | — | [commands.rs:983](../../crates/crysirror-nscripter/src/commands.rs#L983) |
| `amsp` | sprites | Handled | — | [commands.rs:971](../../crates/crysirror-nscripter/src/commands.rs#L971) |
| `amsp2` | amsp2 | Handled | — | [commands.rs:1006](../../crates/crysirror-nscripter/src/commands.rs#L1006) |
| `arc` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `atoi` | arithmetic and strings | Handled | — | [commands.rs:634](../../crates/crysirror-nscripter/src/commands.rs#L634) |
| `autoclick` | autoclick | Handled | — | [native.rs:711](../../crates/crysirror-nscripter/src/native.rs#L711) |
| `automode` | automode | Handled | — | [commands.rs:623](../../crates/crysirror-nscripter/src/commands.rs#L623) |
| `automode_time` | automode time | Handled | — | [commands.rs:624](../../crates/crysirror-nscripter/src/commands.rs#L624) |
| `autosaveoff` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `avi` | movies | Handled | — | [commands.rs:1520](../../crates/crysirror-nscripter/src/commands.rs#L1520) |
| `bar` | bar | Handled | — | [commands.rs:1137](../../crates/crysirror-nscripter/src/commands.rs#L1137) |
| `barclear` | barclear | Handled | — | [commands.rs:1146](../../crates/crysirror-nscripter/src/commands.rs#L1146) |
| `bclear` | bclear | Handled | — | [commands.rs:1307](../../crates/crysirror-nscripter/src/commands.rs#L1307) |
| `bexec` | bexec | Handled | — | [commands.rs:1322](../../crates/crysirror-nscripter/src/commands.rs#L1322) |
| `bg` | bg | Handled | — | [lib.rs:1024](../../crates/crysirror-nscripter/src/lib.rs#L1024) |
| `bgcopy` | bgcopy | Handled | — | [commands.rs:1606](../../crates/crysirror-nscripter/src/commands.rs#L1606) |
| `bgcpy` | bgcpy | Handled | — | [commands.rs:1606](../../crates/crysirror-nscripter/src/commands.rs#L1606) |
| `bgm` | bgm | Handled | — | [commands.rs:1509](../../crates/crysirror-nscripter/src/commands.rs#L1509) |
| `bgm_mp3` | audio | Handled | — | [commands.rs:1428](../../crates/crysirror-nscripter/src/commands.rs#L1428) |
| `bgmfadein` | bgmfadein | Handled | — | [commands.rs:1445](../../crates/crysirror-nscripter/src/commands.rs#L1445) |
| `bgmfadeout` | bgmfadeout | Handled | — | [commands.rs:1445](../../crates/crysirror-nscripter/src/commands.rs#L1445) |
| `bgmonce` | bgmonce | Handled | — | [commands.rs:1509](../../crates/crysirror-nscripter/src/commands.rs#L1509) |
| `bgmstop` | bgmstop | Handled | — | [commands.rs:1514](../../crates/crysirror-nscripter/src/commands.rs#L1514) |
| `bgmvol` | bgmvol | Handled | — | [commands.rs:1458](../../crates/crysirror-nscripter/src/commands.rs#L1458) |
| `blt` | blt | Handled | — | [commands.rs:1261](../../crates/crysirror-nscripter/src/commands.rs#L1261) |
| `br` | br | Handled | — | [lib.rs:1020](../../crates/crysirror-nscripter/src/lib.rs#L1020) |
| `break` | break | Handled | — | [lib.rs:995](../../crates/crysirror-nscripter/src/lib.rs#L995) |
| `btime` | btime | Handled | — | [commands.rs:1317](../../crates/crysirror-nscripter/src/commands.rs#L1317) |
| `btn` | btn | Handled | — | [native.rs:948](../../crates/crysirror-nscripter/src/native.rs#L948) |
| `btndef` | btndef | Handled | — | [native.rs:942](../../crates/crysirror-nscripter/src/native.rs#L942) |
| `btndown` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `btntime` | btntime | Handled | — | [commands.rs:1317](../../crates/crysirror-nscripter/src/commands.rs#L1317) |
| `btntime2` | btntime2 | Handled | — | [commands.rs:1317](../../crates/crysirror-nscripter/src/commands.rs#L1317) |
| `btnwait` | btnwait | Handled | — | [native.rs:965](../../crates/crysirror-nscripter/src/native.rs#L965) |
| `btnwait2` | btnwait2 | Handled | — | [commands.rs:1322](../../crates/crysirror-nscripter/src/commands.rs#L1322) |
| `caption` | caption | Handled | — | [lib.rs:836](../../crates/crysirror-nscripter/src/lib.rs#L836) |
| `cell` | cell | Handled | — | [commands.rs:1047](../../crates/crysirror-nscripter/src/commands.rs#L1047) |
| `cellcheckexbtn` | cellcheckexbtn | Handled | — | [commands.rs:1292](../../crates/crysirror-nscripter/src/commands.rs#L1292) |
| `cellcheckspbtn` | cellcheckspbtn | Handled | — | [commands.rs:1292](../../crates/crysirror-nscripter/src/commands.rs#L1292) |
| `checkkey` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `checkpage` | checkpage | Handled | — | [commands.rs:867](../../crates/crysirror-nscripter/src/commands.rs#L867) |
| `chvol` | chvol | Handled | — | [commands.rs:1466](../../crates/crysirror-nscripter/src/commands.rs#L1466) |
| `cl` | cl | Handled | — | [native.rs:841](../../crates/crysirror-nscripter/src/native.rs#L841) |
| `click` | click | Handled | — | [lib.rs:1006](../../crates/crysirror-nscripter/src/lib.rs#L1006) |
| `clickstr` | clickstr | Handled | — | [native.rs:498](../../crates/crysirror-nscripter/src/native.rs#L498) |
| `clickvoice` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `cmp` | cmp | Handled | — | [native.rs:891](../../crates/crysirror-nscripter/src/native.rs#L891) |
| `cos` | cos | Handled | — | [commands.rs:724](../../crates/crysirror-nscripter/src/commands.rs#L724) |
| `csel` | csel | Handled | — | [commands.rs:1349](../../crates/crysirror-nscripter/src/commands.rs#L1349) |
| `cselbtn` | cselbtn | Handled | — | [commands.rs:1366](../../crates/crysirror-nscripter/src/commands.rs#L1366) |
| `cselgoto` | cselgoto | Handled | — | [commands.rs:1380](../../crates/crysirror-nscripter/src/commands.rs#L1380) |
| `csp` | csp | Handled | — | [lib.rs:1099](../../crates/crysirror-nscripter/src/lib.rs#L1099) |
| `csp2` | csp2 | Handled | — | [commands.rs:1035](../../crates/crysirror-nscripter/src/commands.rs#L1035) |
| `date` | date | Handled | — | [native.rs:684](../../crates/crysirror-nscripter/src/native.rs#L684) |
| `defaultfont` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `defaultspeed` | defaultspeed | Handled | — | [commands.rs:615](../../crates/crysirror-nscripter/src/commands.rs#L615) |
| `definereset` | definereset | Handled | — | [commands.rs:834](../../crates/crysirror-nscripter/src/commands.rs#L834) |
| `defmp3vol` | defmp3vol | Handled | — | [commands.rs:1458](../../crates/crysirror-nscripter/src/commands.rs#L1458) |
| `defsevol` | defsevol | Handled | — | [commands.rs:1462](../../crates/crysirror-nscripter/src/commands.rs#L1462) |
| `defsub` | defsub | Handled | — | [lib.rs:854](../../crates/crysirror-nscripter/src/lib.rs#L854) |
| `defvoicevol` | defvoicevol | Handled | — | [commands.rs:1454](../../crates/crysirror-nscripter/src/commands.rs#L1454) |
| `delay` | delay | Handled | — | [native.rs:706](../../crates/crysirror-nscripter/src/native.rs#L706) |
| `dim` | dim | Handled | — | [commands.rs:751](../../crates/crysirror-nscripter/src/commands.rs#L751) |
| `div` | div | Handled | — | [lib.rs:894](../../crates/crysirror-nscripter/src/lib.rs#L894) |
| `draw` | draw | Handled | — | [commands.rs:1257](../../crates/crysirror-nscripter/src/commands.rs#L1257) |
| `drawbg` | drawbg | Handled | — | [commands.rs:1225](../../crates/crysirror-nscripter/src/commands.rs#L1225) |
| `drawbg2` | drawbg2 | Handled | — | [commands.rs:1225](../../crates/crysirror-nscripter/src/commands.rs#L1225) |
| `drawclear` | draw family, as an overlay composed from engine layers | Handled | — | [commands.rs:1204](../../crates/crysirror-nscripter/src/commands.rs#L1204) |
| `drawfill` | drawfill | Handled | — | [commands.rs:1212](../../crates/crysirror-nscripter/src/commands.rs#L1212) |
| `drawsp` | drawsp | Handled | — | [commands.rs:1230](../../crates/crysirror-nscripter/src/commands.rs#L1230) |
| `drawsp2` | drawsp2 | Handled | — | [commands.rs:1234](../../crates/crysirror-nscripter/src/commands.rs#L1234) |
| `drawsp3` | drawsp3 | Handled | — | [commands.rs:1220](../../crates/crysirror-nscripter/src/commands.rs#L1220) |
| `drawtext` | drawtext | Handled | — | [commands.rs:1253](../../crates/crysirror-nscripter/src/commands.rs#L1253) |
| `dwave` | dwave | Handled | — | [commands.rs:1482](../../crates/crysirror-nscripter/src/commands.rs#L1482) |
| `dwaveload` | dwaveload | Handled | — | [commands.rs:1490](../../crates/crysirror-nscripter/src/commands.rs#L1490) |
| `dwaveloop` | dwaveloop | Handled | — | [commands.rs:1482](../../crates/crysirror-nscripter/src/commands.rs#L1482) |
| `dwaveplay` | dwaveplay | Handled | — | [commands.rs:1498](../../crates/crysirror-nscripter/src/commands.rs#L1498) |
| `dwaveplayloop` | dwaveplayloop | Handled | — | [commands.rs:1498](../../crates/crysirror-nscripter/src/commands.rs#L1498) |
| `dwavestop` | dwavestop | Handled | — | [commands.rs:1504](../../crates/crysirror-nscripter/src/commands.rs#L1504) |
| `effect` | effect | Handled | — | [native.rs:557](../../crates/crysirror-nscripter/src/native.rs#L557) |
| `effectblank` | effectblank | Handled | — | [native.rs:498](../../crates/crysirror-nscripter/src/native.rs#L498) |
| `effectcut` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `end` | end | Handled | — | [lib.rs:1129](../../crates/crysirror-nscripter/src/lib.rs#L1129) |
| `english` | english | Handled | — | [commands.rs:614](../../crates/crysirror-nscripter/src/commands.rs#L614) |
| `erasetextwindow` | erasetextwindow | Handled | — | [commands.rs:941](../../crates/crysirror-nscripter/src/commands.rs#L941) |
| `exbtn` | exbtn | Handled | — | [commands.rs:1279](../../crates/crysirror-nscripter/src/commands.rs#L1279) |
| `exbtn_d` | exbtn d | Handled | — | [commands.rs:1279](../../crates/crysirror-nscripter/src/commands.rs#L1279) |
| `exec_dll` | exec dll | Unsupported | native `{command}` hosts native code or Lua, which CrySirror never loads | [commands.rs:1624](../../crates/crysirror-nscripter/src/commands.rs#L1624) |
| `exec_dll_decl` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `existspbtn` | existspbtn | Handled | — | [commands.rs:1298](../../crates/crysirror-nscripter/src/commands.rs#L1298) |
| `fileexist` | fileexist | Handled | — | [commands.rs:826](../../crates/crysirror-nscripter/src/commands.rs#L826) |
| `filelog` | filelog | Handled | — | [native.rs:534](../../crates/crysirror-nscripter/src/native.rs#L534) |
| `for` | for | Handled | — | [lib.rs:950](../../crates/crysirror-nscripter/src/lib.rs#L950) |
| `game` | game | Handled | — | [lib.rs:828](../../crates/crysirror-nscripter/src/lib.rs#L828) |
| `getbgmvol` | getbgmvol | Handled | — | [commands.rs:1471](../../crates/crysirror-nscripter/src/commands.rs#L1471) |
| `getbtntimer` | getbtntimer | Handled | — | [commands.rs:847](../../crates/crysirror-nscripter/src/commands.rs#L847) |
| `getcselnum` | getcselnum | Handled | — | [commands.rs:1335](../../crates/crysirror-nscripter/src/commands.rs#L1335) |
| `getcselstr` | getcselstr | Handled | — | [commands.rs:1341](../../crates/crysirror-nscripter/src/commands.rs#L1341) |
| `getcursor` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `getcursorpos` | getcursorpos | Handled | — | [commands.rs:853](../../crates/crysirror-nscripter/src/commands.rs#L853) |
| `getcursorpos2` | getcursorpos2 | Handled | — | [commands.rs:853](../../crates/crysirror-nscripter/src/commands.rs#L853) |
| `getenter` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `getfunction` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `getinsert` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `getinsert_decl` | getinsert decl | Empty handler | Original no-op status not established | [commands.rs:1625](../../crates/crysirror-nscripter/src/commands.rs#L1625) |
| `getlog` | getlog | Handled | — | [commands.rs:885](../../crates/crysirror-nscripter/src/commands.rs#L885) |
| `getlogtext` | getlogtext | Handled | — | [commands.rs:885](../../crates/crysirror-nscripter/src/commands.rs#L885) |
| `getmclick` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `getmouseover` | getmouseover | Empty handler | Original no-op status not established | [commands.rs:628](../../crates/crysirror-nscripter/src/commands.rs#L628) |
| `getmousepos` | getmousepos | Handled | — | [commands.rs:853](../../crates/crysirror-nscripter/src/commands.rs#L853) |
| `getmp3vol` | getmp3vol | Handled | — | [commands.rs:1471](../../crates/crysirror-nscripter/src/commands.rs#L1471) |
| `getpage` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `getpageup` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `getparam` | getparam | Handled | — | [lib.rs:918](../../crates/crysirror-nscripter/src/lib.rs#L918) |
| `getparam2` | getparam2 | Handled | — | [commands.rs:805](../../crates/crysirror-nscripter/src/commands.rs#L805) |
| `getreadlang` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `getreg` | getreg | Handled | — | [commands.rs:817](../../crates/crysirror-nscripter/src/commands.rs#L817) |
| `getret` | getret | Handled | — | [commands.rs:812](../../crates/crysirror-nscripter/src/commands.rs#L812) |
| `getsavestr` | getsavestr | Handled | — | [commands.rs:1585](../../crates/crysirror-nscripter/src/commands.rs#L1585) |
| `getscreenshot` | getscreenshot | Handled | — | [commands.rs:1606](../../crates/crysirror-nscripter/src/commands.rs#L1606) |
| `getsevol` | getsevol | Handled | — | [commands.rs:1475](../../crates/crysirror-nscripter/src/commands.rs#L1475) |
| `getspmode` | getspmode | Handled | — | [commands.rs:1083](../../crates/crysirror-nscripter/src/commands.rs#L1083) |
| `getsppos` | getsppos | Handled | — | [commands.rs:1072](../../crates/crysirror-nscripter/src/commands.rs#L1072) |
| `getspsize` | getspsize | Handled | — | [commands.rs:1054](../../crates/crysirror-nscripter/src/commands.rs#L1054) |
| `gettab` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `gettag` | gettag | Handled | — | [commands.rs:900](../../crates/crysirror-nscripter/src/commands.rs#L900) |
| `gettaglog` | gettaglog | Handled | — | [commands.rs:900](../../crates/crysirror-nscripter/src/commands.rs#L900) |
| `gettext` | gettext | Handled | — | [commands.rs:894](../../crates/crysirror-nscripter/src/commands.rs#L894) |
| `gettimer` | input state | Handled | — | [commands.rs:841](../../crates/crysirror-nscripter/src/commands.rs#L841) |
| `getversion` | getversion | Handled | — | [commands.rs:741](../../crates/crysirror-nscripter/src/commands.rs#L741) |
| `getvoicevol` | getvoicevol | Handled | — | [commands.rs:1476](../../crates/crysirror-nscripter/src/commands.rs#L1476) |
| `getzxc` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `globalon` | globalon | Handled | — | [native.rs:526](../../crates/crysirror-nscripter/src/native.rs#L526) |
| `gosub` | Call a scenario subroutine | Handled | — | [lib.rs:913](../../crates/crysirror-nscripter/src/lib.rs#L913) |
| `goto` | Jump within the scenario | Handled | — | [lib.rs:907](../../crates/crysirror-nscripter/src/lib.rs#L907) |
| `humanorder` | humanorder | Empty handler | Original no-op status not established | [commands.rs:627](../../crates/crysirror-nscripter/src/commands.rs#L627) |
| `humanz` | humanz | Handled | — | [commands.rs:1091](../../crates/crysirror-nscripter/src/commands.rs#L1091) |
| `if` | if | Handled | — | [lib.rs:947](../../crates/crysirror-nscripter/src/lib.rs#L947) |
| `inc` | inc | Handled | — | [lib.rs:891](../../crates/crysirror-nscripter/src/lib.rs#L891) |
| `indent` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `input` | input | Handled | — | [commands.rs:1404](../../crates/crysirror-nscripter/src/commands.rs#L1404) |
| `intlimit` | intlimit | Handled | — | [commands.rs:746](../../crates/crysirror-nscripter/src/commands.rs#L746) |
| `isdown` | isdown | Handled | — | [commands.rs:870](../../crates/crysirror-nscripter/src/commands.rs#L870) |
| `isfull` | isfull | Handled | — | [commands.rs:871](../../crates/crysirror-nscripter/src/commands.rs#L871) |
| `ispage` | ispage | Handled | — | [commands.rs:872](../../crates/crysirror-nscripter/src/commands.rs#L872) |
| `isskip` | isskip | Handled | — | [commands.rs:873](../../crates/crysirror-nscripter/src/commands.rs#L873) |
| `itoa` | itoa | Handled | — | [commands.rs:645](../../crates/crysirror-nscripter/src/commands.rs#L645) |
| `itoa2` | itoa2 | Handled | — | [commands.rs:645](../../crates/crysirror-nscripter/src/commands.rs#L645) |
| `jumpb` | jumpb | Handled | — | [commands.rs:795](../../crates/crysirror-nscripter/src/commands.rs#L795) |
| `jumpf` | jumpf | Handled | — | [commands.rs:795](../../crates/crysirror-nscripter/src/commands.rs#L795) |
| `kidokumode` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `kidokuskip` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `kinsoku` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `labellog` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `langen` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `langjp` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `ld` | ld | Handled | — | [native.rs:818](../../crates/crysirror-nscripter/src/native.rs#L818) |
| `len` | len | Handled | — | [commands.rs:663](../../crates/crysirror-nscripter/src/commands.rs#L663) |
| `linepage` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `linepage2` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `loadgame` | loadgame | Handled | — | [commands.rs:1554](../../crates/crysirror-nscripter/src/commands.rs#L1554) |
| `loadgosub` | loadgosub | Handled | — | [commands.rs:1559](../../crates/crysirror-nscripter/src/commands.rs#L1559) |
| `locate` | locate | Handled | — | [commands.rs:921](../../crates/crysirror-nscripter/src/commands.rs#L921) |
| `logsp` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `logsp2` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `lookbackbutton` | lookbackbutton | Handled | — | [native.rs:498](../../crates/crysirror-nscripter/src/native.rs#L498) |
| `lookbackcolor` | lookbackcolor | Handled | — | [native.rs:498](../../crates/crysirror-nscripter/src/native.rs#L498) |
| `lookbackcolor_set` | lookbackcolor set | Handled | — | [commands.rs:965](../../crates/crysirror-nscripter/src/commands.rs#L965) |
| `lookbackflush` | lookbackflush | Handled | — | [commands.rs:621](../../crates/crysirror-nscripter/src/commands.rs#L621) |
| `lookbacksp` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `loopbgm` | loopbgm | Handled | — | [commands.rs:1437](../../crates/crysirror-nscripter/src/commands.rs#L1437) |
| `loopbgmstop` | loopbgmstop | Handled | — | [commands.rs:1433](../../crates/crysirror-nscripter/src/commands.rs#L1433) |
| `lr_trap` | lr trap | Handled | — | [native.rs:716](../../crates/crysirror-nscripter/src/native.rs#L716) |
| `lsp` | lsp | Handled | — | [lib.rs:1062](../../crates/crysirror-nscripter/src/lib.rs#L1062) |
| `lsp2` | lsp2 | Handled | — | [commands.rs:991](../../crates/crysirror-nscripter/src/commands.rs#L991) |
| `lsp2add` | lsp2add | Handled | — | [commands.rs:991](../../crates/crysirror-nscripter/src/commands.rs#L991) |
| `lsp2sub` | lsp2sub | Handled | — | [commands.rs:991](../../crates/crysirror-nscripter/src/commands.rs#L991) |
| `lsph` | lsph | Handled | — | [lib.rs:1062](../../crates/crysirror-nscripter/src/lib.rs#L1062) |
| `lsph2` | lsph2 | Handled | — | [commands.rs:991](../../crates/crysirror-nscripter/src/commands.rs#L991) |
| `lsph2add` | lsph2add | Handled | — | [commands.rs:991](../../crates/crysirror-nscripter/src/commands.rs#L991) |
| `lsph2sub` | lsph2sub | Handled | — | [commands.rs:991](../../crates/crysirror-nscripter/src/commands.rs#L991) |
| `luacall` | luacall | Unsupported | native `{command}` hosts native code or Lua, which CrySirror never loads | [commands.rs:1624](../../crates/crysirror-nscripter/src/commands.rs#L1624) |
| `luasub` | luasub | Unsupported | native `{command}` hosts native code or Lua, which CrySirror never loads | [commands.rs:1624](../../crates/crysirror-nscripter/src/commands.rs#L1624) |
| `maxkaisoupage` | maxkaisoupage | Handled | — | [commands.rs:622](../../crates/crysirror-nscripter/src/commands.rs#L622) |
| `menu_automode` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `menu_click_def` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `menu_click_page` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `menu_full` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `menu_window` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `menuselectcolor` | menuselectcolor | Handled | — | [native.rs:498](../../crates/crysirror-nscripter/src/native.rs#L498) |
| `menuselectvoice` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `menusetwindow` | menusetwindow | Handled | — | [native.rs:498](../../crates/crysirror-nscripter/src/native.rs#L498) |
| `mid` | mid | Handled | — | [commands.rs:670](../../crates/crysirror-nscripter/src/commands.rs#L670) |
| `mod` | mod | Handled | — | [lib.rs:898](../../crates/crysirror-nscripter/src/lib.rs#L898) |
| `mode_ext` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `mode_saya` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `monocro` | whole-screen effects | Handled | — | [commands.rs:1173](../../crates/crysirror-nscripter/src/commands.rs#L1173) |
| `mov` | mov | Handled | — | [lib.rs:861](../../crates/crysirror-nscripter/src/lib.rs#L861) |
| `mov10` | mov10 | Handled | — | [commands.rs:759](../../crates/crysirror-nscripter/src/commands.rs#L759) |
| `mov3` | mov3 | Handled | — | [commands.rs:759](../../crates/crysirror-nscripter/src/commands.rs#L759) |
| `mov4` | mov4 | Handled | — | [commands.rs:759](../../crates/crysirror-nscripter/src/commands.rs#L759) |
| `mov5` | mov5 | Handled | — | [commands.rs:759](../../crates/crysirror-nscripter/src/commands.rs#L759) |
| `mov6` | mov6 | Handled | — | [commands.rs:759](../../crates/crysirror-nscripter/src/commands.rs#L759) |
| `mov7` | mov7 | Handled | — | [commands.rs:759](../../crates/crysirror-nscripter/src/commands.rs#L759) |
| `mov8` | mov8 | Handled | — | [commands.rs:759](../../crates/crysirror-nscripter/src/commands.rs#L759) |
| `mov9` | mov9 | Handled | — | [commands.rs:759](../../crates/crysirror-nscripter/src/commands.rs#L759) |
| `movemousecursor` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `movie` | movies | Handled | — | [commands.rs:1520](../../crates/crysirror-nscripter/src/commands.rs#L1520) |
| `movl` | movl | Handled | — | [commands.rs:772](../../crates/crysirror-nscripter/src/commands.rs#L772) |
| `mp3` | audio | Handled | — | [commands.rs:1428](../../crates/crysirror-nscripter/src/commands.rs#L1428) |
| `mp3fadein` | mp3fadein | Handled | — | [commands.rs:1445](../../crates/crysirror-nscripter/src/commands.rs#L1445) |
| `mp3fadeout` | mp3fadeout | Handled | — | [commands.rs:1445](../../crates/crysirror-nscripter/src/commands.rs#L1445) |
| `mp3loop` | audio | Handled | — | [commands.rs:1428](../../crates/crysirror-nscripter/src/commands.rs#L1428) |
| `mp3save` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `mp3stop` | mp3stop | Handled | — | [commands.rs:1433](../../crates/crysirror-nscripter/src/commands.rs#L1433) |
| `mp3vol` | mp3vol | Handled | — | [commands.rs:1458](../../crates/crysirror-nscripter/src/commands.rs#L1458) |
| `mpegplay` | movies | Handled | — | [commands.rs:1520](../../crates/crysirror-nscripter/src/commands.rs#L1520) |
| `msp` | msp | Handled | — | [native.rs:862](../../crates/crysirror-nscripter/src/native.rs#L862) |
| `msp2` | msp2 | Handled | — | [commands.rs:1006](../../crates/crysirror-nscripter/src/commands.rs#L1006) |
| `mul` | mul | Handled | — | [lib.rs:893](../../crates/crysirror-nscripter/src/lib.rs#L893) |
| `nega` | whole-screen effects | Handled | — | [commands.rs:1173](../../crates/crysirror-nscripter/src/commands.rs#L1173) |
| `newpage` | newpage | Handled | — | [native.rs:697](../../crates/crysirror-nscripter/src/native.rs#L697) |
| `next` | next | Handled | — | [lib.rs:979](../../crates/crysirror-nscripter/src/lib.rs#L979) |
| `notif` | notif | Handled | — | [lib.rs:947](../../crates/crysirror-nscripter/src/lib.rs#L947) |
| `ns2` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `ns3` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `nsa` | nsa | Handled | — | [native.rs:541](../../crates/crysirror-nscripter/src/native.rs#L541) |
| `nsadir` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `numalias` | numalias | Handled | — | [lib.rs:841](../../crates/crysirror-nscripter/src/lib.rs#L841) |
| `ofscopy` | ofscopy | Handled | — | [commands.rs:1606](../../crates/crysirror-nscripter/src/commands.rs#L1606) |
| `ofscpy` | ofscpy | Handled | — | [commands.rs:1606](../../crates/crysirror-nscripter/src/commands.rs#L1606) |
| `okcancelbox` | okcancelbox | Handled | — | [commands.rs:1386](../../crates/crysirror-nscripter/src/commands.rs#L1386) |
| `pagetag` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `play` | play | Handled | — | [native.rs:914](../../crates/crysirror-nscripter/src/native.rs#L914) |
| `playonce` | playonce | Handled | — | [native.rs:914](../../crates/crysirror-nscripter/src/native.rs#L914) |
| `playstop` | playstop | Handled | — | [native.rs:924](../../crates/crysirror-nscripter/src/native.rs#L924) |
| `pretextgosub` | pretextgosub | Handled | — | [commands.rs:910](../../crates/crysirror-nscripter/src/commands.rs#L910) |
| `print` | print | Handled | — | [lib.rs:1120](../../crates/crysirror-nscripter/src/lib.rs#L1120) |
| `prnum` | prnum | Handled | — | [commands.rs:1153](../../crates/crysirror-nscripter/src/commands.rs#L1153) |
| `prnumclear` | prnumclear | Handled | — | [commands.rs:1165](../../crates/crysirror-nscripter/src/commands.rs#L1165) |
| `puttext` | puttext | Handled | — | [commands.rs:928](../../crates/crysirror-nscripter/src/commands.rs#L928) |
| `quake` | quake | Handled | — | [commands.rs:1191](../../crates/crysirror-nscripter/src/commands.rs#L1191) |
| `quakex` | quakex | Handled | — | [commands.rs:1196](../../crates/crysirror-nscripter/src/commands.rs#L1196) |
| `quakey` | quakey | Handled | — | [commands.rs:1197](../../crates/crysirror-nscripter/src/commands.rs#L1197) |
| `r_trap` | r trap | Handled | — | [native.rs:716](../../crates/crysirror-nscripter/src/native.rs#L716) |
| `repaint` | repaint | Handled | — | [native.rs:910](../../crates/crysirror-nscripter/src/native.rs#L910) |
| `reset` | reset | Handled | — | [commands.rs:1601](../../crates/crysirror-nscripter/src/commands.rs#L1601) |
| `resettimer` | resettimer | Handled | — | [native.rs:735](../../crates/crysirror-nscripter/src/native.rs#L735) |
| `return` | return | Handled | — | [lib.rs:937](../../crates/crysirror-nscripter/src/lib.rs#L937) |
| `rmenu` | rmenu | Handled | — | [native.rs:498](../../crates/crysirror-nscripter/src/native.rs#L498) |
| `rmode` | rmode | Handled | — | [commands.rs:626](../../crates/crysirror-nscripter/src/commands.rs#L626) |
| `rnd` | rnd | Handled | — | [commands.rs:701](../../crates/crysirror-nscripter/src/commands.rs#L701) |
| `rnd2` | rnd2 | Handled | — | [commands.rs:701](../../crates/crysirror-nscripter/src/commands.rs#L701) |
| `roff` | roff | Handled | — | [commands.rs:625](../../crates/crysirror-nscripter/src/commands.rs#L625) |
| `rubyoff` | rubyoff | Handled | — | [commands.rs:950](../../crates/crysirror-nscripter/src/commands.rs#L950) |
| `rubyon` | rubyon | Handled | — | [commands.rs:950](../../crates/crysirror-nscripter/src/commands.rs#L950) |
| `sar` | sar | Handled | — | [native.rs:541](../../crates/crysirror-nscripter/src/native.rs#L541) |
| `savedir` | savedir | Handled | — | [commands.rs:834](../../crates/crysirror-nscripter/src/commands.rs#L834) |
| `savefileexist` | savefileexist | Handled | — | [commands.rs:1565](../../crates/crysirror-nscripter/src/commands.rs#L1565) |
| `savegame` | savegame | Handled | — | [commands.rs:1545](../../crates/crysirror-nscripter/src/commands.rs#L1545) |
| `savegame2` | savegame2 | Handled | — | [commands.rs:1545](../../crates/crysirror-nscripter/src/commands.rs#L1545) |
| `savename` | savename | Handled | — | [native.rs:498](../../crates/crysirror-nscripter/src/native.rs#L498) |
| `savename_declared` | savename declared | Handled | — | [commands.rs:834](../../crates/crysirror-nscripter/src/commands.rs#L834) |
| `savenumber` | savenumber | Handled | — | [native.rs:498](../../crates/crysirror-nscripter/src/native.rs#L498) |
| `saveoff` | save and load | Handled | — | [commands.rs:1538](../../crates/crysirror-nscripter/src/commands.rs#L1538) |
| `saveon` | save and load | Handled | — | [commands.rs:1538](../../crates/crysirror-nscripter/src/commands.rs#L1538) |
| `savepoint` | savepoint | Handled | — | [commands.rs:1542](../../crates/crysirror-nscripter/src/commands.rs#L1542) |
| `savescreenshot` | savescreenshot | Handled | — | [commands.rs:1612](../../crates/crysirror-nscripter/src/commands.rs#L1612) |
| `savescreenshot2` | savescreenshot2 | Handled | — | [commands.rs:1612](../../crates/crysirror-nscripter/src/commands.rs#L1612) |
| `savetime` | savetime | Handled | — | [commands.rs:1573](../../crates/crysirror-nscripter/src/commands.rs#L1573) |
| `select` | Present a choice | Handled | — | [lib.rs:827](../../crates/crysirror-nscripter/src/lib.rs#L827) |
| `selectbtnwait` | selectbtnwait | Handled | — | [commands.rs:1322](../../crates/crysirror-nscripter/src/commands.rs#L1322) |
| `selectcolor` | selectcolor | Handled | — | [native.rs:498](../../crates/crysirror-nscripter/src/native.rs#L498) |
| `selectvoice` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `selgosub` | selgosub | Handled | — | [lib.rs:827](../../crates/crysirror-nscripter/src/lib.rs#L827) |
| `selnum` | selnum | Handled | — | [lib.rs:827](../../crates/crysirror-nscripter/src/lib.rs#L827) |
| `setcursor` | setcursor | Handled | — | [native.rs:631](../../crates/crysirror-nscripter/src/native.rs#L631) |
| `setkinsoku` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `setwindow` | setwindow | Handled | — | [native.rs:601](../../crates/crysirror-nscripter/src/native.rs#L601) |
| `setwindow2` | setwindow2 | Handled | — | [commands.rs:954](../../crates/crysirror-nscripter/src/commands.rs#L954) |
| `setwindow3` | setwindow3 | Handled | — | [native.rs:601](../../crates/crysirror-nscripter/src/native.rs#L601) |
| `sevol` | sevol | Handled | — | [commands.rs:1462](../../crates/crysirror-nscripter/src/commands.rs#L1462) |
| `shadedistance` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `showlangen` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `showlangjp` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `sin` | sin | Handled | — | [commands.rs:723](../../crates/crysirror-nscripter/src/commands.rs#L723) |
| `skip` | skip | Handled | — | [native.rs:744](../../crates/crysirror-nscripter/src/native.rs#L744) |
| `soundpressplgin` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `spbtn` | buttons | Handled | — | [commands.rs:1273](../../crates/crysirror-nscripter/src/commands.rs#L1273) |
| `spi` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `split` | split | Handled | — | [commands.rs:683](../../crates/crysirror-nscripter/src/commands.rs#L683) |
| `spreload` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `spstr` | spstr | Handled | — | [commands.rs:1102](../../crates/crysirror-nscripter/src/commands.rs#L1102) |
| `stop` | stop | Handled | — | [native.rs:924](../../crates/crysirror-nscripter/src/native.rs#L924) |
| `stralias` | stralias | Handled | — | [lib.rs:841](../../crates/crysirror-nscripter/src/lib.rs#L841) |
| `strsp` | strsp | Handled | — | [commands.rs:1102](../../crates/crysirror-nscripter/src/commands.rs#L1102) |
| `strsph` | strsph | Handled | — | [commands.rs:1102](../../crates/crysirror-nscripter/src/commands.rs#L1102) |
| `sub` | sub | Handled | — | [lib.rs:892](../../crates/crysirror-nscripter/src/lib.rs#L892) |
| `systemcall` | systemcall | Handled | — | [commands.rs:1593](../../crates/crysirror-nscripter/src/commands.rs#L1593) |
| `tablegoto` | tablegoto | Handled | — | [commands.rs:785](../../crates/crysirror-nscripter/src/commands.rs#L785) |
| `tan` | tan | Handled | — | [commands.rs:717](../../crates/crysirror-nscripter/src/commands.rs#L717) |
| `tateyoko` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `textbtnwait` | textbtnwait | Handled | — | [commands.rs:1322](../../crates/crysirror-nscripter/src/commands.rs#L1322) |
| `textclear` | textclear | Handled | — | [lib.rs:1016](../../crates/crysirror-nscripter/src/lib.rs#L1016) |
| `textcolor` | textcolor | Handled | — | [native.rs:701](../../crates/crysirror-nscripter/src/native.rs#L701) |
| `textgosub` | textgosub | Handled | — | [commands.rs:910](../../crates/crysirror-nscripter/src/commands.rs#L910) |
| `texthide` | texthide | Handled | — | [commands.rs:933](../../crates/crysirror-nscripter/src/commands.rs#L933) |
| `textoff` | textoff | Handled | — | [native.rs:752](../../crates/crysirror-nscripter/src/native.rs#L752) |
| `texton` | texton | Handled | — | [native.rs:756](../../crates/crysirror-nscripter/src/native.rs#L756) |
| `textshow` | textshow | Handled | — | [commands.rs:937](../../crates/crysirror-nscripter/src/commands.rs#L937) |
| `textspeed` | textspeed | Handled | — | [lib.rs:1010](../../crates/crysirror-nscripter/src/lib.rs#L1010) |
| `textspeeddefault` | textspeeddefault | Handled | — | [commands.rs:946](../../crates/crysirror-nscripter/src/commands.rs#L946) |
| `time` | time | Handled | — | [commands.rs:729](../../crates/crysirror-nscripter/src/commands.rs#L729) |
| `transbtn` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `transmode` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `trap` | trap | Handled | — | [native.rs:716](../../crates/crysirror-nscripter/src/native.rs#L716) |
| `underline` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `useescspc` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `usewheel` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `versionstr` | versionstr | Handled | — | [native.rs:498](../../crates/crysirror-nscripter/src/native.rs#L498) |
| `voicevol` | voicevol | Handled | — | [commands.rs:1454](../../crates/crysirror-nscripter/src/commands.rs#L1454) |
| `vsp` | vsp | Handled | — | [lib.rs:1086](../../crates/crysirror-nscripter/src/lib.rs#L1086) |
| `vsp2` | vsp2 | Handled | — | [commands.rs:1028](../../crates/crysirror-nscripter/src/commands.rs#L1028) |
| `wait` | wait | Handled | — | [lib.rs:1000](../../crates/crysirror-nscripter/src/lib.rs#L1000) |
| `waittimer` | waittimer | Handled | — | [native.rs:739](../../crates/crysirror-nscripter/src/native.rs#L739) |
| `wave` | wave | Handled | — | [native.rs:914](../../crates/crysirror-nscripter/src/native.rs#L914) |
| `waveloop` | waveloop | Handled | — | [native.rs:914](../../crates/crysirror-nscripter/src/native.rs#L914) |
| `wavestop` | wavestop | Handled | — | [native.rs:924](../../crates/crysirror-nscripter/src/native.rs#L924) |
| `windowback` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `windowchip` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |
| `windoweffect` | windoweffect | Handled | — | [native.rs:498](../../crates/crysirror-nscripter/src/native.rs#L498) |
| `yesnobox` | yesnobox | Handled | — | [commands.rs:1386](../../crates/crysirror-nscripter/src/commands.rs#L1386) |
| `zenkakko` | declarations with no runtime effect beyond being remembered | Handled | — | [commands.rs:587](../../crates/crysirror-nscripter/src/commands.rs#L587) |

## Subselectors and conditional dispatch

Patterns retain their source namespace and guards. `_` is the fallback of that table. Boolean/optional results follow the function's return contract. Call-site names identify the implementation to inspect, including indirect service calls.

### commands.rs / NscripterVm::extended_command

```rust
fn extended_command(&mut self, command: &str, t: &mut Tokens, host: &mut dyn HostServices) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `command; under command="addkinsoku" \| "setkinsoku" \| "kinsoku" \| "shadedistance" \| "underline" \| "zenkakko" \| "soundpressplgin" \| "windowchip" \| "menu_automode" \| "menu_click_def" \| "menu_click_page" \| "menu_full" \| "menu_window" \| "maxkaisoupage" \| "spi" \| "arc" \| "ns2" \| "ns3" \| "nsadir" \| "mode_ext" \| "mode_saya" \| "labellog" \| "kidokuskip" \| "kidokumode" \| "linepage" \| "linepage2" \| "pagetag" \| "checkkey" \| "defaultfont" \| "useescspc" \| "usewheel" \| "lookbacksp" \| "logsp" \| "logsp2" \| "exec_dll_decl" \| "effectcut" \| "autosaveoff" \| "tateyoko" \| "indent" \| "defaultspeed" \| "windowback" \| "automode" \| "automode_time" \| "rmode" \| "roff" \| "clickvoice" \| "selectvoice" \| "menuselectvoice" \| "langen" \| "langjp" \| "showlangen" \| "showlangjp" \| "getmclick" \| "getmouseover" \| "getinsert" \| "getzxc" \| "getfunction" \| "getpage" \| "getpageup" \| "getcursor" \| "getenter" \| "gettab" \| "btndown" \| "transbtn" \| "transmode" \| "movemousecursor" \| "getreadlang" \| "english" \| "humanorder" \| "spreload" \| "lookbackflush" \| "mp3save"` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [629](../../crates/crysirror-nscripter/src/commands.rs#L629) |
| `condition; under command="itoa" \| "itoa2"` | `command == "itoa2"` | Local state/value operations | Handled | [656](../../crates/crysirror-nscripter/src/commands.rs#L656) |
| `condition; under command="rnd" \| "rnd2"` | `command == "rnd2"` | Local state/value operations | Handled | [705](../../crates/crysirror-nscripter/src/commands.rs#L705) |
| `command; under command="sin" \| "cos" \| "tan"` | `_` | Local state/value operations | Handled | [725](../../crates/crysirror-nscripter/src/commands.rs#L725) |
| `condition; under command="jumpf" \| "jumpb"` | `command == "jumpf"` | Local state/value operations | Handled | [799](../../crates/crysirror-nscripter/src/commands.rs#L799) |
| `condition; under command="savedir" \| "definereset" \| "savename_declared"` | `command == "definereset"` | Local state/value operations | Handled | [836](../../crates/crysirror-nscripter/src/commands.rs#L836) |
| `condition; under command="getmousepos" \| "getcursorpos" \| "getcursorpos2"` | `command == "getmousepos"` | Local state/value operations | Handled | [858](../../crates/crysirror-nscripter/src/commands.rs#L858) |
| `command; under command="isdown" \| "isfull" \| "ispage" \| "isskip" \| "checkpage"` | `_` | i32::from, usize::try_from, self.state.native.history.len | Handled | [876](../../crates/crysirror-nscripter/src/commands.rs#L876) |
| `condition; under command="textgosub" \| "pretextgosub"` | `command == "textgosub"` | Local state/value operations | Handled | [914](../../crates/crysirror-nscripter/src/commands.rs#L914) |
| `condition; under command="lsp2" \| "lsph2" \| "lsp2add" \| "lsph2add" \| "lsp2sub" \| "lsph2sub"` | `command.ends_with("add")` | Local state/value operations | Handled | [999](../../crates/crysirror-nscripter/src/commands.rs#L999) |
| `condition; under command="lsp2" \| "lsph2" \| "lsp2add" \| "lsph2add" \| "lsp2sub" \| "lsph2sub"` | `command.ends_with("sub")` | Local state/value operations | Handled | [999](../../crates/crysirror-nscripter/src/commands.rs#L999) |
| `condition; under command="strsp" \| "strsph" \| "spstr"` | `command == "spstr"` | self.state.native.declarations.insert | Handled | [1103](../../crates/crysirror-nscripter/src/commands.rs#L1103) |
| `condition; under command="monocro" \| "nega"` | `command == "nega"` | Local state/value operations | Handled | [1176](../../crates/crysirror-nscripter/src/commands.rs#L1176) |
| `command; under command="quake" \| "quakex" \| "quakey"` | `_` | Local state/value operations | Handled | [1198](../../crates/crysirror-nscripter/src/commands.rs#L1198) |
| `condition; under command="drawsp" \| "drawsp2" \| "drawsp3" \| "drawbg" \| "drawbg2" / command="drawbg" \| "drawbg2"` | `command == "drawbg2"` | Local state/value operations | Handled | [1227](../../crates/crysirror-nscripter/src/commands.rs#L1227) |
| `command; under command="drawsp" \| "drawsp2" \| "drawsp3" \| "drawbg" \| "drawbg2"` | `_` | Local state/value operations | Unsupported: native drawsp3 needs a shared affine matrix mapping | [1239](../../crates/crysirror-nscripter/src/commands.rs#L1239) |
| `condition; under command="exbtn" \| "exbtn_d"` | `command == "exbtn_d"` | Local state/value operations | Handled | [1281](../../crates/crysirror-nscripter/src/commands.rs#L1281) |
| `condition; under command="existspbtn"` | `a[0] >= 0 && self.state.native.buttons.iter().all(\|b\| b.sprite != Some(SPRITE + a[0] as u32))` | self.sprite_rect, self.state.native.buttons.push | Handled | [1301](../../crates/crysirror-nscripter/src/commands.rs#L1301) |
| `condition; under command="existspbtn"` | `let Some(rect) = self.sprite_rect(SPRITE + a[0] as u32)` | self.state.native.buttons.push | Handled | [1302](../../crates/crysirror-nscripter/src/commands.rs#L1302) |
| `condition; under command="bclear" \| "bdef"` | `command == "bdef"` | self.resolve_asset | Handled | [1308](../../crates/crysirror-nscripter/src/commands.rs#L1308) |
| `condition; under command="btnwait2" \| "textbtnwait" \| "selectbtnwait" \| "bexec"` | `command != "textbtnwait"` | self.hide_text | Handled | [1327](../../crates/crysirror-nscripter/src/commands.rs#L1327) |
| `condition; under command="yesnobox" \| "okcancelbox"` | `command == "yesnobox"` | Local state/value operations | Handled | [1399](../../crates/crysirror-nscripter/src/commands.rs#L1399) |
| `condition; under command="bgmfadeout" \| "mp3fadeout" \| "bgmfadein" \| "mp3fadein"` | `command.ends_with("out")` | Local state/value operations | Handled | [1448](../../crates/crysirror-nscripter/src/commands.rs#L1448) |
| `command; under command="bgmvol" \| "mp3vol" \| "sevol" \| "voicevol" \| "defmp3vol" \| "defsevol" \| "defvoicevol"` | `_` | Local state/value operations | Handled | [1463](../../crates/crysirror-nscripter/src/commands.rs#L1463) |
| `command; under command="getbgmvol" \| "getmp3vol" \| "getsevol" \| "getvoicevol"` | `_` | Local state/value operations | Handled | [1477](../../crates/crysirror-nscripter/src/commands.rs#L1477) |
| `condition; under command="mpegplay" \| "avi" \| "movie"` | `command == "movie"` | Local state/value operations | Handled | [1523](../../crates/crysirror-nscripter/src/commands.rs#L1523) |
| `condition; under command="savescreenshot" \| "savescreenshot2"` | `command == "savescreenshot"` | Local state/value operations | Handled | [1620](../../crates/crysirror-nscripter/src/commands.rs#L1620) |
| `command` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [1626](../../crates/crysirror-nscripter/src/commands.rs#L1626) |

### commands.rs / NscripterVm::poll_extended

```rust
fn poll_extended(&mut self, wait: Wait, host: &mut dyn HostServices, input: &mut InputFrame) -> Result<Option<VmYield>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under wait=Wait::Input { variable }` | `!events.iter().any(\|e\| e.kind == "activate" && e.widget.owner == self.client.0 && e.widget.id == INPUT + 2)` | Local state/value operations | Handled | [476](../../crates/crysirror-nscripter/src/commands.rs#L476) |

### commands.rs / NscripterVm::update_animations

```rust
fn update_animations(&mut self, host: &mut dyn HostServices) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!self.state.published.contains(id) \|\| animation.sequence.frame_ms == 0` | Local state/value operations | Handled | [561](../../crates/crysirror-nscripter/src/commands.rs#L561) |

### lib.rs / NscripterVm::condition

```rust
fn condition(
        &mut self,
        t: &mut Tokens,
        invert: bool,
        host: &mut dyn HostServices,
    ) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!matches!(op.as_str(), "&" \| "&&" \| "\|" \| "\|\|")` | Local state/value operations | Handled | [1274](../../crates/crysirror-nscripter/src/lib.rs#L1274) |
| `op.as_str()` | `"=" \| "=="` | Local state/value operations | Handled | [1297](../../crates/crysirror-nscripter/src/lib.rs#L1297) |
| `op.as_str()` | `"!=" \| "<>"` | Local state/value operations | Handled | [1298](../../crates/crysirror-nscripter/src/lib.rs#L1298) |
| `op.as_str()` | `">"` | Local state/value operations | Handled | [1299](../../crates/crysirror-nscripter/src/lib.rs#L1299) |
| `op.as_str()` | `"<"` | Local state/value operations | Handled | [1300](../../crates/crysirror-nscripter/src/lib.rs#L1300) |
| `op.as_str()` | `">="` | Local state/value operations | Handled | [1301](../../crates/crysirror-nscripter/src/lib.rs#L1301) |
| `op.as_str()` | `"<="` | Local state/value operations | Handled | [1302](../../crates/crysirror-nscripter/src/lib.rs#L1302) |
| `op.as_str()` | `_` | Local state/value operations | Unsupported: invalid native comparison | [1303](../../crates/crysirror-nscripter/src/lib.rs#L1303) |
| `condition` | `!matches!(op.as_str(), "&" \| "&&" \| "\|" \| "\|\|")` | Local state/value operations | Handled | [1314](../../crates/crysirror-nscripter/src/lib.rs#L1314) |

### lib.rs / NscripterVm::execute

```rust
fn execute(&mut self, body: &str, host: &mut dyn HostServices) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under command.as_str()="numalias" \| "stralias"` | `command == "numalias"` | Local state/value operations | Handled | [847](../../crates/crysirror-nscripter/src/lib.rs#L847) |
| `condition; under command.as_str()="inc" \| "dec" \| "add" \| "sub" \| "mul" \| "div" \| "mod"` | `command == "inc" \|\| command == "dec"` | Value::Int | Handled | [875](../../crates/crysirror-nscripter/src/lib.rs#L875) |
| `condition; under command.as_str()="inc" \| "dec" \| "add" \| "sub" \| "mul" \| "div" \| "mod"` | `command == "add" && matches!(left, Value::Str(_))` | Value::Str | Handled | [882](../../crates/crysirror-nscripter/src/lib.rs#L882) |
| `command.as_str(); under command.as_str()="inc" \| "dec" \| "add" \| "sub" \| "mul" \| "div" \| "mod"` | `_` | Local state/value operations | Handled | [902](../../crates/crysirror-nscripter/src/lib.rs#L902) |
| `command.as_str()` | `name if self.state.subroutines.contains(name)` | self.call | Handled | [1133](../../crates/crysirror-nscripter/src/lib.rs#L1133) |
| `command.as_str()` | `other` | self.extended_command | Partial: unsupported NScripter native command `{command}` | [1134](../../crates/crysirror-nscripter/src/lib.rs#L1134) |

### lib.rs / NscripterVm::select

```rust
fn select(&mut self, t: &mut Tokens, host: &mut dyn HostServices, mode: &str) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `mode == "selnum"` | self.state.vars.resolve | Handled | [1162](../../crates/crysirror-nscripter/src/lib.rs#L1162) |

### lib.rs / NscripterVm::skip_loop

```rust
fn skip_loop(&mut self) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `command == "for"` | Local state/value operations | Handled | [1234](../../crates/crysirror-nscripter/src/lib.rs#L1234) |
| `condition` | `command == "next"` | Local state/value operations | Handled | [1236](../../crates/crysirror-nscripter/src/lib.rs#L1236) |

### native.rs / NscripterVm::effect

```rust
fn effect(&mut self, id: i32, host: &mut dyn HostServices) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `id == 0` | Local state/value operations | Handled | [233](../../crates/crysirror-nscripter/src/native.rs#L233) |
| `condition` | `id == 1` | Local state/value operations | Handled | [237](../../crates/crysirror-nscripter/src/native.rs#L237) |
| `effect.kind` | `2..=9` | Local state/value operations | Handled | [248](../../crates/crysirror-nscripter/src/native.rs#L248) |
| `effect.kind` | `10` | Local state/value operations | Handled | [251](../../crates/crysirror-nscripter/src/native.rs#L251) |
| `effect.kind` | `11` | Local state/value operations | Handled | [252](../../crates/crysirror-nscripter/src/native.rs#L252) |
| `effect.kind` | `12` | Local state/value operations | Handled | [253](../../crates/crysirror-nscripter/src/native.rs#L253) |
| `effect.kind` | `13` | Local state/value operations | Handled | [254](../../crates/crysirror-nscripter/src/native.rs#L254) |
| `effect.kind` | `14` | Local state/value operations | Handled | [255](../../crates/crysirror-nscripter/src/native.rs#L255) |
| `effect.kind` | `15 \| 18` | Local state/value operations | Handled | [256](../../crates/crysirror-nscripter/src/native.rs#L256) |
| `effect.kind` | `other` | Local state/value operations | Unsupported: native effect kind {other} needs exact shared transition mapping | [259](../../crates/crysirror-nscripter/src/native.rs#L259) |

### native.rs / NscripterVm::native_command

```rust
fn native_command(
        &mut self,
        command: &str,
        t: &mut Tokens,
        host: &mut dyn HostServices,
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under command="effect"` | `(2..=9).contains(&kind) && !self.state.native.reveal_masks.contains_key(&kind)` | host.image_metadata, Arc::from, serde_json::to_vec | Handled | [579](../../crates/crysirror-nscripter/src/native.rs#L579) |
| `condition; under command="effect"` | `kind >= 6` | Local state/value operations | Handled | [580](../../crates/crysirror-nscripter/src/native.rs#L580) |
| `a[2]; under command="setcursor" \| "abssetcursor"` | `0` | Local state/value operations | Handled | [659](../../crates/crysirror-nscripter/src/native.rs#L659) |
| `a[2]; under command="setcursor" \| "abssetcursor"` | `1` | Local state/value operations | Handled | [660](../../crates/crysirror-nscripter/src/native.rs#L660) |
| `a[2]; under command="setcursor" \| "abssetcursor"` | `2` | Local state/value operations | Handled | [661](../../crates/crysirror-nscripter/src/native.rs#L661) |
| `a[2]; under command="setcursor" \| "abssetcursor"` | `3` | Local state/value operations | Handled | [662](../../crates/crysirror-nscripter/src/native.rs#L662) |
| `a[2]; under command="setcursor" \| "abssetcursor"` | `_` | Local state/value operations | Unsupported: native cursor loop mode | [663](../../crates/crysirror-nscripter/src/native.rs#L663) |
| `condition; under command="trap" \| "lr_trap" \| "r_trap"` | `command == "r_trap"` | Local state/value operations | Handled | [726](../../crates/crysirror-nscripter/src/native.rs#L726) |
| `condition; under command="trap" \| "lr_trap" \| "r_trap"` | `command == "lr_trap"` | Local state/value operations | Handled | [729](../../crates/crysirror-nscripter/src/native.rs#L729) |
| `condition; under command="play" \| "playonce" \| "wave" \| "waveloop"` | `command.starts_with("play")` | Local state/value operations | Handled | [919](../../crates/crysirror-nscripter/src/native.rs#L919) |
| `condition; under command="playstop" \| "wavestop" \| "stop"` | `command == "stop"` | Local state/value operations | Handled | [926](../../crates/crysirror-nscripter/src/native.rs#L926) |
| `condition; under command="playstop" \| "wavestop" \| "stop"` | `command == "playstop"` | Local state/value operations | Handled | [928](../../crates/crysirror-nscripter/src/native.rs#L928) |
| `command` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [974](../../crates/crysirror-nscripter/src/native.rs#L974) |

### native.rs / NscripterVm::poll_buttons

```rust
fn poll_buttons(
        &mut self,
        host: &mut dyn HostServices,
        input: &mut InputFrame,
        variable: u16,
    ) -> Result<Option<VmYield>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `declared("getenter") && key(buttons::ENTER)` | Local state/value operations | Handled | [1114](../../crates/crysirror-nscripter/src/native.rs#L1114) |
| `condition` | `declared("gettab") && key(buttons::TAB)` | Local state/value operations | Handled | [1116](../../crates/crysirror-nscripter/src/native.rs#L1116) |
| `condition` | `declared("getcursor") && key(buttons::UP)` | Local state/value operations | Handled | [1118](../../crates/crysirror-nscripter/src/native.rs#L1118) |
| `condition` | `declared("getcursor") && key(buttons::RIGHT)` | Local state/value operations | Handled | [1120](../../crates/crysirror-nscripter/src/native.rs#L1120) |
| `condition` | `declared("getcursor") && key(buttons::DOWN)` | Local state/value operations | Handled | [1122](../../crates/crysirror-nscripter/src/native.rs#L1122) |
| `condition` | `declared("getcursor") && key(buttons::LEFT)` | Local state/value operations | Handled | [1124](../../crates/crysirror-nscripter/src/native.rs#L1124) |
| `condition` | `declared("getpageup") && key(buttons::PAGE_UP)` | Local state/value operations | Handled | [1126](../../crates/crysirror-nscripter/src/native.rs#L1126) |
| `condition` | `declared("getpageup") && key(buttons::PAGE_DOWN)` | Local state/value operations | Handled | [1128](../../crates/crysirror-nscripter/src/native.rs#L1128) |
| `condition` | `declared("usewheel") && key(buttons::WHEEL_UP)` | Local state/value operations | Handled | [1130](../../crates/crysirror-nscripter/src/native.rs#L1130) |
| `condition` | `declared("usewheel") && key(buttons::WHEEL_DOWN)` | Local state/value operations | Handled | [1132](../../crates/crysirror-nscripter/src/native.rs#L1132) |

### native.rs / NscripterVm::poll_image

```rust
fn poll_image(
        &mut self,
        host: &mut dyn HostServices,
        next: usize,
    ) -> Result<Option<VmYield>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under pending.job.poll()?=ImageProcessState::Ready(resource)` | `(MASK + 2..=MASK + 9).contains(&pending.id)` | Local state/value operations | Handled | [376](../../crates/crysirror-nscripter/src/native.rs#L376) |
| `condition; under pending.job.poll()?=ImageProcessState::Ready(resource)` | `pending.id >= TACHI && pending.id < TACHI + 3` | Local state/value operations | Handled | [387](../../crates/crysirror-nscripter/src/native.rs#L387) |
| `condition; under pending.job.poll()?=ImageProcessState::Ready(resource)` | `pending.id >= SPRITE2 && pending.id < SPRITE2 + 1000` | i64::from | Handled | [407](../../crates/crysirror-nscripter/src/native.rs#L407) |

### program.rs / Tokens::binary

```rust
fn binary(&mut self, vars: &Variables, min: u8) -> Result<Value>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `op.as_str()` | `"+" \| "-"` | Local state/value operations | Handled | [496](../../crates/crysirror-nscripter/src/program.rs#L496) |
| `op.as_str()` | `"*" \| "/"` | Local state/value operations | Handled | [497](../../crates/crysirror-nscripter/src/program.rs#L497) |
| `op.as_str()` | `_` | Local state/value operations | Handled | [498](../../crates/crysirror-nscripter/src/program.rs#L498) |
| `op; under (left, right, op.as_str())=(Value::Int(a), Value::Int(b), op)` | `"+"` | Local state/value operations | Handled | [512](../../crates/crysirror-nscripter/src/program.rs#L512) |
| `op; under (left, right, op.as_str())=(Value::Int(a), Value::Int(b), op)` | `"-"` | Local state/value operations | Handled | [513](../../crates/crysirror-nscripter/src/program.rs#L513) |
| `op; under (left, right, op.as_str())=(Value::Int(a), Value::Int(b), op)` | `"*"` | Local state/value operations | Handled | [514](../../crates/crysirror-nscripter/src/program.rs#L514) |
| `op; under (left, right, op.as_str())=(Value::Int(a), Value::Int(b), op)` | `"/"` | Local state/value operations | Handled | [515](../../crates/crysirror-nscripter/src/program.rs#L515) |
| `op; under (left, right, op.as_str())=(Value::Int(a), Value::Int(b), op)` | `_` | Local state/value operations | Handled | [519](../../crates/crysirror-nscripter/src/program.rs#L519) |
| `(left, right, op.as_str())` | `(Value::Str(a), Value::Str(b), "+")` | Value::Str | Handled | [507](../../crates/crysirror-nscripter/src/program.rs#L507) |
| `(left, right, op.as_str())` | `(Value::Int(a), Value::Int(b), op)` | Value::Int | Handled | [511](../../crates/crysirror-nscripter/src/program.rs#L511) |
| `(left, right, op.as_str())` | `_` | Local state/value operations | Unsupported: mixed native string/integer arithmetic | [521](../../crates/crysirror-nscripter/src/program.rs#L521) |

### program.rs / Tokens::new

```rust
fn new(text: &str) -> Result<Self>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `c.is_ascii_alphabetic() \|\| c == '_' \|\| c == '*' && values.last().is_none_or(\|previous\| match previous { Token::Comma => true, Token::Word(w) => matches!( w.as_str(), "goto" \| "gosub" \| "return" \| "trap" \| "lr_trap" \| "r_trap" ), Token::Op(op) => op != ")", _ => false, }) && s.as_bytes() .get(1) .is_some_and(\|b\| b.is_ascii_alphanumeric() \|\| *b == b'_')` | usize::from, Token::Label, Token::Word | Handled | [363](../../crates/crysirror-nscripter/src/program.rs#L363) |
