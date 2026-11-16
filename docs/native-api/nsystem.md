# N-System developer reference

[All engines](README.md)

Mode: `adopt`. Evidence: HANANYU.EXE (sha256 `81b9063f…e1ee`, crate
`SOURCE.json`), static IDA 9.0 analysis; function names are the strings the
binary passes to its own trace routine 0x440850.

## ABI profile `hananyu-2015`

| Item | Value | Evidence |
| --- | --- | --- |
| Detection | `HANANYU.EXE` + extensionless FJSYS banks `MSD`, `MGD` (+ `BGM`, `VOICE`, `SE`, `DATA`) | launcher `detect/nsystem.rs` |
| MSD key | CP932 window title; block n XOR = hex MD5(title ‖ decimal n), 32-byte blocks; verified on `start.MSD`, never searched | `CSnrBuffer::Read` 0x442B60 |
| MSD layout | magic(16), version @0x10, labels @0x14, 256 load-point counts @0x18, header end 0x458 | `CSnrBuffer` 0x442540.. |
| Command | u16 id, u16 operand bytes; tags 1 int, 2 var, 3 CP932 string (`&&`, `&`+u16 string var), 4 RPN, 5 string var | `NextParam` 0x442980 |
| MGD | compression = **u16** @+24 (0 raw, 1 SGD, 2 PNG); trailing region table `[x,y,w,h]` | `DecodeMain` 0x43C2F0, `UpdateWndImage` 0x404180 |
| Layers | 2 pages × 16; the back page is committed at OVERLAP and reset afterwards | `CDIBSnrVVram::Move` 0x43B2A0 |
| Layer defaults | blend on, colour 0xFFFFFFFF, action 0, camera on, scale 1 | `.data` 0x5C88E8 |
| Variables | 0..999 per slot, 1000..1999 global (CONFIG.SAV) | `SaveGame` / `SaveConfig` |

Other builds need a profile row (exe, title); unknown builds are not detected.

## Dispatch

`CDIBSnrBaseScreen::Move` 0x435B20 (vtable 0x47EA40) dispatches MSD command
ids; native screens dispatch on control index (vfunc 116). Both are generated
below. Unhandled ids fail visibly with name and PC.

## Native screens

| Screen | Contract |
| --- | --- |
| Title (`CDIBTitleScreen`) | logos, OP.MPG, name fades, logo zoom, menu start/load/option/omake/end, idle replay |
| Message window (`CDIBMesWnd`) | typewriter (`MovePutWord`), click wait (`OnClickWait`: skip read/all, Ctrl, auto with MESWND key 92), hide, backlog, QSave/QLoad (999), Save/Load, Skip/Auto, Config, Title/Exit |
| Select | slide + fade, pointer/keyboard |
| Save/Load (`CDIBSaveLoadScreen`) | 5 pages × 10 slots, captions, CONF save/load |
| Confirm (`CDIBConfScreen`) | 0 exit, 1 to title, 2 save, 3 load; `.data` button defaults (MSD records use screen 0) |
| Option (`CDIBOptionScreen`) | two pages, radios, sliders 0..255, tests, per-speaker voice/colour |
| Omake (`CDIBOmakeScreen`) | CG / scene modes, grid from flags, music, CGVIEW / scene replay (CHKSCEXEC) |

## Persistent files (save capability)

| File | Format | Evidence |
| --- | --- | --- |
| `GAMEnn.SAV`, `GAME999.SAV` | title-MD5 header, plain system block, XOR screen blob (LPT snapshot) | 0x418A30 / 0x419070; an original slot round-trips byte-identically |
| `CONFIG.SAV` | globals, read history, CG/scene flags, settings; written at each LPT | 0x4196B0 / 0x419950; the original round-trips byte-identically |
| `Novel.ini` | [ConfigEx], [charNNN] voice/colour | 0x417F70 / 0x4181A0 |

Files live in CrySirror's per-game save directory; copy original files there to use them.

## Inferred or approximated

- Volume: a fit of the DirectSound table `dword_5C8950` (mB ≈ −6224 + 2310·log10 v);
  BGMSmallVolume is stored, not applied.
- Slot dates use the host timezone rules, not `FileTimeToLocalFileTime`.
- Scene-mode grid sheets use (character, page) like CG mode.
- Backlog keeps 256 pages; page-turn wipe and MWO/MWC slide are not drawn.
- New CONFIG.SAV read-history names are zero-padded (native copies 256 bytes past its 72-byte entry).
- Thumbnails use the shared bilinear resize.

<!-- BEGIN GENERATED DISPATCH TABLES -->

## Opcode and API implementation tables

`Handled` means an executable source branch exists. It is not a claim that every argument/subselector or a complete game has been audited. Explicit gaps are shown per row.

| Status | Entries |
| --- | ---: |
| Handled | 134 |

These tables cover the checked-in catalogues and explicit dispatch branches. An engine-wide upstream denominator is not inferred from these rows.

## MSD commands

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x01 LPT` | save point marker (native also refreshes the quick-save availability); execution continues | Handled | — | [adapter.rs:1099](../../crates/crysirror-nsystem/src/adapter.rs#L1099) |
| `0x02 LS` | replace the running script and start at its first command | Handled | — | [adapter.rs:1107](../../crates/crysirror-nsystem/src/adapter.rs#L1107) |
| `0x03 JP` | JP | Handled | — | [adapter.rs:1114](../../crates/crysirror-nsystem/src/adapter.rs#L1114) |
| `0x04 ONJP` | jump to the n-th label operand, else fall through | Handled | — | [adapter.rs:1119](../../crates/crysirror-nsystem/src/adapter.rs#L1119) |
| `0x05 MOV` | variable index, value | Handled | — | [adapter.rs:1129](../../crates/crysirror-nsystem/src/adapter.rs#L1129) |
| `0x06 IF` | (a, op, b) groups, all true → jump to the final label | Handled | — | [adapter.rs:1135](../../crates/crysirror-nsystem/src/adapter.rs#L1135) |
| `0x07 ATIMES` | arm a timer (ms; negative = forever) | Handled | — | [adapter.rs:1179](../../crates/crysirror-nsystem/src/adapter.rs#L1179) |
| `0x08 AWAIT` | (skippable, five flag variables): wait for the timer (or any input when skippable), then store the input flags | Handled | — | [adapter.rs:1187](../../crates/crysirror-nsystem/src/adapter.rs#L1187) |
| `0x64 IMREAD` | image cache slot ← sprite name | Handled | — | [adapter.rs:1203](../../crates/crysirror-nsystem/src/adapter.rs#L1203) |
| `0x65 IMFREE` | IMFREE | Handled | — | [adapter.rs:1211](../../crates/crysirror-nsystem/src/adapter.rs#L1211) |
| `0x66 LYIMAGE` | layer image from a name or an IMREAD slot (back page) | Handled | — | [adapter.rs:1217](../../crates/crysirror-nsystem/src/adapter.rs#L1217) |
| `0x67 LYPOS` | / CDIBSnrLayer::SetPos : x, y, camera depth | Handled | — | [adapter.rs:1231](../../crates/crysirror-nsystem/src/adapter.rs#L1231) |
| `0x69 LYCOLOR` | (layer, a, r, g, b), each clamped to 0..=255 | Handled | — | [adapter.rs:1533](../../crates/crysirror-nsystem/src/adapter.rs#L1533) |
| `0x6A LYBLEND` | LYBLEND | Handled | — | [adapter.rs:1237](../../crates/crysirror-nsystem/src/adapter.rs#L1237) |
| `0x6B LYANTI` | LYANTI | Handled | — | [adapter.rs:1237](../../crates/crysirror-nsystem/src/adapter.rs#L1237) |
| `0x6C LYACTION` | attach action slot value+1 (SetAction ) | Handled | — | [adapter.rs:1249](../../crates/crysirror-nsystem/src/adapter.rs#L1249) |
| `0x6E OVERLAP` | OVERLAP | Handled | — | [adapter.rs:1255](../../crates/crysirror-nsystem/src/adapter.rs#L1255) |
| `0xC9 BGMPLAY` | name, wait, flag, fade ms. Loops; fades are not mapped | Handled | — | [adapter.rs:1259](../../crates/crysirror-nsystem/src/adapter.rs#L1259) |
| `0xCA BGMSTOP` | wait flag, fade ms | Handled | — | [adapter.rs:1269](../../crates/crysirror-nsystem/src/adapter.rs#L1269) |
| `0xD0 SEPLAY` | slot, fade ms, wait, loop | Handled | — | [adapter.rs:1294](../../crates/crysirror-nsystem/src/adapter.rs#L1294) |
| `0xD1 SESTOP` | slot, fade ms, wait | Handled | — | [adapter.rs:1316](../../crates/crysirror-nsystem/src/adapter.rs#L1316) |
| `0xD2 SEWAIT` | mode 2 waits until the slot stops | Handled | — | [adapter.rs:1324](../../crates/crysirror-nsystem/src/adapter.rs#L1324) |
| `0xD3 SEREAD` | SE slot ← name; yields one frame | Handled | — | [adapter.rs:1281](../../crates/crysirror-nsystem/src/adapter.rs#L1281) |
| `0x3E9 MWO` | / : open / close the message window | Handled | — | [adapter.rs:1334](../../crates/crysirror-nsystem/src/adapter.rs#L1334) |
| `0x3EA MWC` | / : open / close the message window | Handled | — | [adapter.rs:1334](../../crates/crysirror-nsystem/src/adapter.rs#L1334) |
| `0x3EC CHKSCEXEC` | variable ← scene-replay mode (never in normal play) | Handled | — | [adapter.rs:1349](../../crates/crysirror-nsystem/src/adapter.rs#L1349) |
| `0x3ED SUBTITLE` | chapter title kept for save slots | Handled | — | [adapter.rs:1356](../../crates/crysirror-nsystem/src/adapter.rs#L1356) |
| `0x3EE SETCGFLAG` | SETCGFLAG | Handled | — | [adapter.rs:1361](../../crates/crysirror-nsystem/src/adapter.rs#L1361) |
| `0x3EF SETSCENEFLAG` | SETSCENEFLAG | Handled | — | [adapter.rs:1361](../../crates/crysirror-nsystem/src/adapter.rs#L1361) |
| `0x3F0 SETMESSKIP` | → SetMesSkip | Handled | — | [adapter.rs:1614](../../crates/crysirror-nsystem/src/adapter.rs#L1614) |
| `0x7D0 MESFTSIZE` | MESFTSIZE | Handled | — | [adapter.rs:1386](../../crates/crysirror-nsystem/src/adapter.rs#L1386) |
| `0x7D1 MESFTCOLOR` | index 0 text colour, 1 shadow colour | Handled | — | [adapter.rs:1375](../../crates/crysirror-nsystem/src/adapter.rs#L1375) |
| `0x7D3 MESFTSHADOW` | MESFTSHADOW | Handled | — | [adapter.rs:1381](../../crates/crysirror-nsystem/src/adapter.rs#L1381) |
| `0x7D5 MESTXKINSOKU` | MESTXKINSOKU | Handled | — | [adapter.rs:1394](../../crates/crysirror-nsystem/src/adapter.rs#L1394) |
| `0x7D6 MESTXDOWN` | indentation after an opening quote; recorded only | Handled | — | [adapter.rs:1403](../../crates/crysirror-nsystem/src/adapter.rs#L1403) |
| `0x7D7 MESNAME` | speaker name plate number | Handled | — | [adapter.rs:1409](../../crates/crysirror-nsystem/src/adapter.rs#L1409) |
| `0x7D8 MESVPLAY` | read-flag id, voice name ('' = none) | Handled | — | [adapter.rs:1431](../../crates/crysirror-nsystem/src/adapter.rs#L1431) |
| `0x7D9 MESVSTOP` | stop the voice when the message advances | Handled | — | [adapter.rs:1453](../../crates/crysirror-nsystem/src/adapter.rs#L1453) |
| `0x7DA MESRUN` | read-flag id, then strings and function codes. → CDIBMesWnd::OutStart / MoveText | Handled | — | [adapter.rs:1460](../../crates/crysirror-nsystem/src/adapter.rs#L1460) |
| `0x7DD SELMAKE` | → MakeSelData: (index, enabled, x, y, five state cells) | Handled | — | [adapter.rs:1581](../../crates/crysirror-nsystem/src/adapter.rs#L1581) |
| `0x7DE SELRUN` | SELRUN | Handled | — | [adapter.rs:1590](../../crates/crysirror-nsystem/src/adapter.rs#L1590) |
| `0x7DF ACTMAKE` | (slot, type, ...). Type 1 is Scroll: (duration ms, loop, spline, then x,y,z points); other types are not used here | Handled | — | [adapter.rs:1542](../../crates/crysirror-nsystem/src/adapter.rs#L1542) |
| `0x7E0 ACTFREE` | free the slot and detach layers that used it | Handled | — | [adapter.rs:1568](../../crates/crysirror-nsystem/src/adapter.rs#L1568) |
| `0x7E1 ACTPLAY` | / : start or stop the listed slots | Handled | — | [adapter.rs:1557](../../crates/crysirror-nsystem/src/adapter.rs#L1557) |
| `0x7E2 ACTSTOP` | / : start or stop the listed slots | Handled | — | [adapter.rs:1557](../../crates/crysirror-nsystem/src/adapter.rs#L1557) |
| `0x7E4 SAVESAM` | save-thumbnail sampling settings only | Handled | — | [adapter.rs:1257](../../crates/crysirror-nsystem/src/adapter.rs#L1257) |
| `0x1B58 SETCHARID` | speaker id selecting the text colour preset. : speaker id; ids < 256 take their Novel.ini colour | Handled | — | [adapter.rs:1422](../../crates/crysirror-nsystem/src/adapter.rs#L1422) |
| `0x2710 END` | back to the native title screen (not adopted yet). → CRootFlow: back to the title menu (Initialize a2 ≠ 0) | Handled | — | [adapter.rs:1628](../../crates/crysirror-nsystem/src/adapter.rs#L1628) |

## Native screen controls / adapter / window_button

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x00` | OnClickVOICEBTN : replay the current voice | Handled | — | [adapter.rs:970](../../crates/crysirror-nsystem/src/adapter.rs#L970) |
| `0x01` | OnClickBLPREVBTN / OnClickBLNEXTBTN | Handled | — | [adapter.rs:977](../../crates/crysirror-nsystem/src/adapter.rs#L977) |
| `0x02` | Update window / bl view | Handled | — | [adapter.rs:985](../../crates/crysirror-nsystem/src/adapter.rs#L985) |
| `0x03` | OnClickQSAVEBTN : slot 999 without a dialog | Handled | — | [adapter.rs:989](../../crates/crysirror-nsystem/src/adapter.rs#L989) |
| `0x04` | Begin load | Handled | — | [adapter.rs:993](../../crates/crysirror-nsystem/src/adapter.rs#L993) |
| `0x05` | Open save load | Handled | — | [adapter.rs:994](../../crates/crysirror-nsystem/src/adapter.rs#L994) |
| `0x06` | Open save load | Handled | — | [adapter.rs:995](../../crates/crysirror-nsystem/src/adapter.rs#L995) |
| `0x07` | Update window / skip mode, window / auto mode | Handled | — | [adapter.rs:996](../../crates/crysirror-nsystem/src/adapter.rs#L996) |
| `0x08` | Update window / auto mode, window / skip mode | Handled | — | [adapter.rs:1000](../../crates/crysirror-nsystem/src/adapter.rs#L1000) |
| `0x09` | OnClickSYSTEMBTN | Handled | — | [adapter.rs:1005](../../crates/crysirror-nsystem/src/adapter.rs#L1005) |
| `0x0A` | Open conf | Handled | — | [adapter.rs:1006](../../crates/crysirror-nsystem/src/adapter.rs#L1006) |
| `0x0B` | Open conf | Handled | — | [adapter.rs:1007](../../crates/crysirror-nsystem/src/adapter.rs#L1007) |

## Native screen controls / adapter/omake / omake_mode

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x00` | Execute omake mode branch | Handled | — | [omake.rs:118](../../crates/crysirror-nsystem/src/adapter/omake.rs#L118) |
| `0x01` | Execute omake mode branch | Handled | — | [omake.rs:118](../../crates/crysirror-nsystem/src/adapter/omake.rs#L118) |
| `0x02` | Execute omake mode branch | Handled | — | [omake.rs:118](../../crates/crysirror-nsystem/src/adapter/omake.rs#L118) |
| `0x03` | Execute omake mode branch | Handled | — | [omake.rs:118](../../crates/crysirror-nsystem/src/adapter/omake.rs#L118) |
| `0x04` | Execute omake mode branch | Handled | — | [omake.rs:118](../../crates/crysirror-nsystem/src/adapter/omake.rs#L118) |
| `0x05` | Execute omake mode branch | Handled | — | [omake.rs:118](../../crates/crysirror-nsystem/src/adapter/omake.rs#L118) |
| `0x06` | Execute omake mode branch | Handled | — | [omake.rs:119](../../crates/crysirror-nsystem/src/adapter/omake.rs#L119) |
| `0x07` | Execute omake mode branch | Handled | — | [omake.rs:119](../../crates/crysirror-nsystem/src/adapter/omake.rs#L119) |
| `0x08` | Execute omake mode branch | Handled | — | [omake.rs:119](../../crates/crysirror-nsystem/src/adapter/omake.rs#L119) |
| `0x09` | Execute omake mode branch | Handled | — | [omake.rs:119](../../crates/crysirror-nsystem/src/adapter/omake.rs#L119) |
| `0x0A` | Execute omake mode branch | Handled | — | [omake.rs:119](../../crates/crysirror-nsystem/src/adapter/omake.rs#L119) |
| `0x0B` | Execute omake mode branch | Handled | — | [omake.rs:119](../../crates/crysirror-nsystem/src/adapter/omake.rs#L119) |
| `0x0C` | Execute omake mode branch | Handled | — | [omake.rs:119](../../crates/crysirror-nsystem/src/adapter/omake.rs#L119) |
| `0x0D` | Execute omake mode branch | Handled | — | [omake.rs:119](../../crates/crysirror-nsystem/src/adapter/omake.rs#L119) |
| `0x0E` | Execute omake mode branch | Handled | — | [omake.rs:119](../../crates/crysirror-nsystem/src/adapter/omake.rs#L119) |
| `0x0F` | Execute omake mode branch | Handled | — | [omake.rs:119](../../crates/crysirror-nsystem/src/adapter/omake.rs#L119) |
| `0x10` | Execute omake mode branch | Handled | — | [omake.rs:120](../../crates/crysirror-nsystem/src/adapter/omake.rs#L120) |
| `0x11` | Execute omake mode branch | Handled | — | [omake.rs:120](../../crates/crysirror-nsystem/src/adapter/omake.rs#L120) |
| `0x12` | Execute omake mode branch | Handled | — | [omake.rs:120](../../crates/crysirror-nsystem/src/adapter/omake.rs#L120) |
| `0x13` | Execute omake mode branch | Handled | — | [omake.rs:120](../../crates/crysirror-nsystem/src/adapter/omake.rs#L120) |
| `0x14` | Execute omake mode branch | Handled | — | [omake.rs:120](../../crates/crysirror-nsystem/src/adapter/omake.rs#L120) |
| `0x15` | Execute omake mode branch | Handled | — | [omake.rs:120](../../crates/crysirror-nsystem/src/adapter/omake.rs#L120) |
| `0x16` | Execute omake mode branch | Handled | — | [omake.rs:120](../../crates/crysirror-nsystem/src/adapter/omake.rs#L120) |
| `0x17` | Execute omake mode branch | Handled | — | [omake.rs:120](../../crates/crysirror-nsystem/src/adapter/omake.rs#L120) |
| `0x18` | Execute omake mode branch | Handled | — | [omake.rs:120](../../crates/crysirror-nsystem/src/adapter/omake.rs#L120) |
| `0x19` | Execute omake mode branch | Handled | — | [omake.rs:120](../../crates/crysirror-nsystem/src/adapter/omake.rs#L120) |
| `0x1A` | Execute omake mode branch | Handled | — | [omake.rs:120](../../crates/crysirror-nsystem/src/adapter/omake.rs#L120) |
| `0x1B` | Execute omake mode branch | Handled | — | [omake.rs:120](../../crates/crysirror-nsystem/src/adapter/omake.rs#L120) |
| `0x1C` | Execute omake mode branch | Handled | — | [omake.rs:120](../../crates/crysirror-nsystem/src/adapter/omake.rs#L120) |
| `0x1D` | Execute omake mode branch | Handled | — | [omake.rs:120](../../crates/crysirror-nsystem/src/adapter/omake.rs#L120) |
| `0x1E` | Execute omake mode branch | Handled | — | [omake.rs:120](../../crates/crysirror-nsystem/src/adapter/omake.rs#L120) |
| `0x1F` | Execute omake mode branch | Handled | — | [omake.rs:120](../../crates/crysirror-nsystem/src/adapter/omake.rs#L120) |
| `0x20` | Execute omake mode branch | Handled | — | [omake.rs:120](../../crates/crysirror-nsystem/src/adapter/omake.rs#L120) |
| `0x21` | Execute omake mode branch | Handled | — | [omake.rs:120](../../crates/crysirror-nsystem/src/adapter/omake.rs#L120) |
| `0x22` | Execute omake mode branch | Handled | — | [omake.rs:120](../../crates/crysirror-nsystem/src/adapter/omake.rs#L120) |
| `0x23` | Execute omake mode branch | Handled | — | [omake.rs:120](../../crates/crysirror-nsystem/src/adapter/omake.rs#L120) |

## Native screen controls / adapter/option / option_step

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x00` | Submit | Handled | — | [option.rs:197](../../crates/crysirror-nsystem/src/adapter/option.rs#L197) |
| `0x01` | Submit | Handled | — | [option.rs:197](../../crates/crysirror-nsystem/src/adapter/option.rs#L197) |
| `0x02` | Execute option step branch | Handled | — | [option.rs:202](../../crates/crysirror-nsystem/src/adapter/option.rs#L202) |
| `0x03` | Execute option step branch | Handled | — | [option.rs:202](../../crates/crysirror-nsystem/src/adapter/option.rs#L202) |
| `0x04` | Execute option step branch | Handled | — | [option.rs:203](../../crates/crysirror-nsystem/src/adapter/option.rs#L203) |
| `0x05` | Execute option step branch | Handled | — | [option.rs:204](../../crates/crysirror-nsystem/src/adapter/option.rs#L204) |
| `0x06` | Execute option step branch | Handled | — | [option.rs:205](../../crates/crysirror-nsystem/src/adapter/option.rs#L205) |
| `0x07` | Execute option step branch | Handled | — | [option.rs:205](../../crates/crysirror-nsystem/src/adapter/option.rs#L205) |
| `0x08` | Execute option step branch | Handled | — | [option.rs:206](../../crates/crysirror-nsystem/src/adapter/option.rs#L206) |
| `0x09` | Execute option step branch | Handled | — | [option.rs:207](../../crates/crysirror-nsystem/src/adapter/option.rs#L207) |
| `0x0A` | Execute option step branch | Handled | — | [option.rs:207](../../crates/crysirror-nsystem/src/adapter/option.rs#L207) |
| `0x0B` | Asset | Handled | — | [option.rs:211](../../crates/crysirror-nsystem/src/adapter/option.rs#L211) |
| `0x0C` | Execute option step branch | Handled | — | [option.rs:208](../../crates/crysirror-nsystem/src/adapter/option.rs#L208) |
| `0x0D` | Execute option step branch | Handled | — | [option.rs:209](../../crates/crysirror-nsystem/src/adapter/option.rs#L209) |
| `0x0E` | Execute option step branch | Handled | — | [option.rs:209](../../crates/crysirror-nsystem/src/adapter/option.rs#L209) |
| `0x0F` | Asset | Handled | — | [option.rs:211](../../crates/crysirror-nsystem/src/adapter/option.rs#L211) |
| `0x10` | Execute option step branch | Handled | — | [option.rs:210](../../crates/crysirror-nsystem/src/adapter/option.rs#L210) |
| `0x11` | Write ini | Handled | — | [option.rs:216](../../crates/crysirror-nsystem/src/adapter/option.rs#L216) |
| `0x12` | Apply volumes | Handled | — | [option.rs:224](../../crates/crysirror-nsystem/src/adapter/option.rs#L224) |
| `0x13` | Execute option step branch | Handled | — | [option.rs:231](../../crates/crysirror-nsystem/src/adapter/option.rs#L231) |
| `0x14` | Execute option step branch | Handled | — | [option.rs:232](../../crates/crysirror-nsystem/src/adapter/option.rs#L232) |
| `0x15` | Execute option step branch | Handled | — | [option.rs:232](../../crates/crysirror-nsystem/src/adapter/option.rs#L232) |
| `0x16` | Execute option step branch | Handled | — | [option.rs:233](../../crates/crysirror-nsystem/src/adapter/option.rs#L233) |
| `0x17` | Execute option step branch | Handled | — | [option.rs:233](../../crates/crysirror-nsystem/src/adapter/option.rs#L233) |
| `0x18` | Execute option step branch | Handled | — | [option.rs:233](../../crates/crysirror-nsystem/src/adapter/option.rs#L233) |
| `0x19` | Execute option step branch | Handled | — | [option.rs:233](../../crates/crysirror-nsystem/src/adapter/option.rs#L233) |
| `0x1A` | Execute option step branch | Handled | — | [option.rs:237](../../crates/crysirror-nsystem/src/adapter/option.rs#L237) |
| `0x1B` | Execute option step branch | Handled | — | [option.rs:237](../../crates/crysirror-nsystem/src/adapter/option.rs#L237) |
| `0x1C` | Execute option step branch | Handled | — | [option.rs:238](../../crates/crysirror-nsystem/src/adapter/option.rs#L238) |
| `0x1D` | Execute option step branch | Handled | — | [option.rs:239](../../crates/crysirror-nsystem/src/adapter/option.rs#L239) |
| `0x1E` | Execute option step branch | Handled | — | [option.rs:240](../../crates/crysirror-nsystem/src/adapter/option.rs#L240) |
| `0x1F` | Execute option step branch | Handled | — | [option.rs:240](../../crates/crysirror-nsystem/src/adapter/option.rs#L240) |
| `0x20` | Execute option step branch | Handled | — | [option.rs:240](../../crates/crysirror-nsystem/src/adapter/option.rs#L240) |
| `0x21` | Execute option step branch | Handled | — | [option.rs:241](../../crates/crysirror-nsystem/src/adapter/option.rs#L241) |
| `0x22` | Execute option step branch | Handled | — | [option.rs:241](../../crates/crysirror-nsystem/src/adapter/option.rs#L241) |
| `0x23` | Write ini | Handled | — | [option.rs:216](../../crates/crysirror-nsystem/src/adapter/option.rs#L216) |
| `0x24` | Apply volumes | Handled | — | [option.rs:224](../../crates/crysirror-nsystem/src/adapter/option.rs#L224) |
| `0x25` | Execute option step branch | Handled | — | [option.rs:231](../../crates/crysirror-nsystem/src/adapter/option.rs#L231) |

## Subselectors and conditional dispatch

Patterns retain their source namespace and guards. `_` is the fallback of that table. Boolean/optional results follow the function's return contract. Call-site names identify the implementation to inspect, including indirect service calls.

### adapter.rs / load_window_regions

```rust
fn load_window_regions(
    storage: Arc<dyn ClientStorage>,
    scripts: &BTreeMap<String, Arc<Script>>,
    images: &BTreeMap<String, String>,
    cancel: &crysirror_tasks::Cancellation,
) -> Result<BTreeMap<String, Arc<Vec<[u16; 4]>>>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Some([crate::script::Operand::Text(parts), ..]) = setup.setup(3, key)` | Local state/value operations | Handled | [1874](../../crates/crysirror-nsystem/src/adapter.rs#L1874) |
| `condition` | `!name.ends_with(".mgd")` | Local state/value operations | Handled | [1922](../../crates/crysirror-nsystem/src/adapter.rs#L1922) |
| `condition` | `out.contains_key(&format!("mgd/{name}"))` | Local state/value operations | Handled | [1925](../../crates/crysirror-nsystem/src/adapter.rs#L1925) |

### adapter.rs / name

```rust
fn name(id: u16) -> &'static str
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `id` | `LPT` | Local state/value operations | Handled | [334](../../crates/crysirror-nsystem/src/adapter.rs#L334) |
| `id` | `LS` | Local state/value operations | Handled | [334](../../crates/crysirror-nsystem/src/adapter.rs#L334) |
| `id` | `JP` | Local state/value operations | Handled | [334](../../crates/crysirror-nsystem/src/adapter.rs#L334) |
| `id` | `ONJP` | Local state/value operations | Handled | [334](../../crates/crysirror-nsystem/src/adapter.rs#L334) |
| `id` | `MOV` | Local state/value operations | Handled | [334](../../crates/crysirror-nsystem/src/adapter.rs#L334) |
| `id` | `IF` | Local state/value operations | Handled | [334](../../crates/crysirror-nsystem/src/adapter.rs#L334) |
| `id` | `ATIMES` | Local state/value operations | Handled | [335](../../crates/crysirror-nsystem/src/adapter.rs#L335) |
| `id` | `AWAIT` | Local state/value operations | Handled | [335](../../crates/crysirror-nsystem/src/adapter.rs#L335) |
| `id` | `RANDOM` | Local state/value operations | Handled | [335](../../crates/crysirror-nsystem/src/adapter.rs#L335) |
| `id` | `CMPSTR` | Local state/value operations | Handled | [335](../../crates/crysirror-nsystem/src/adapter.rs#L335) |
| `id` | `IMREAD` | Local state/value operations | Handled | [336](../../crates/crysirror-nsystem/src/adapter.rs#L336) |
| `id` | `IMFREE` | Local state/value operations | Handled | [336](../../crates/crysirror-nsystem/src/adapter.rs#L336) |
| `id` | `LYIMAGE` | Local state/value operations | Handled | [336](../../crates/crysirror-nsystem/src/adapter.rs#L336) |
| `id` | `LYPOS` | Local state/value operations | Handled | [336](../../crates/crysirror-nsystem/src/adapter.rs#L336) |
| `id` | `LYSIZE` | Local state/value operations | Handled | [337](../../crates/crysirror-nsystem/src/adapter.rs#L337) |
| `id` | `LYCOLOR` | Local state/value operations | Handled | [337](../../crates/crysirror-nsystem/src/adapter.rs#L337) |
| `id` | `LYBLEND` | Local state/value operations | Handled | [337](../../crates/crysirror-nsystem/src/adapter.rs#L337) |
| `id` | `LYANTI` | Local state/value operations | Handled | [337](../../crates/crysirror-nsystem/src/adapter.rs#L337) |
| `id` | `LYACTION` | Local state/value operations | Handled | [338](../../crates/crysirror-nsystem/src/adapter.rs#L338) |
| `id` | `LYCAMSTATE` | Local state/value operations | Handled | [338](../../crates/crysirror-nsystem/src/adapter.rs#L338) |
| `id` | `OVERLAP` | Local state/value operations | Handled | [338](../../crates/crysirror-nsystem/src/adapter.rs#L338) |
| `id` | `CAMMOVE` | Local state/value operations | Handled | [339](../../crates/crysirror-nsystem/src/adapter.rs#L339) |
| `id` | `CURSOR` | Local state/value operations | Handled | [339](../../crates/crysirror-nsystem/src/adapter.rs#L339) |
| `id` | `MOVIE` | Local state/value operations | Handled | [339](../../crates/crysirror-nsystem/src/adapter.rs#L339) |
| `id` | `BGMLIST` | Local state/value operations | Handled | [339](../../crates/crysirror-nsystem/src/adapter.rs#L339) |
| `id` | `BGMPLAY` | Local state/value operations | Handled | [340](../../crates/crysirror-nsystem/src/adapter.rs#L340) |
| `id` | `BGMSTOP` | Local state/value operations | Handled | [340](../../crates/crysirror-nsystem/src/adapter.rs#L340) |
| `id` | `BGMWAIT` | Local state/value operations | Handled | [340](../../crates/crysirror-nsystem/src/adapter.rs#L340) |
| `id` | `VOICELIST` | Local state/value operations | Handled | [340](../../crates/crysirror-nsystem/src/adapter.rs#L340) |
| `id` | `VOICEPLAY` | Local state/value operations | Handled | [341](../../crates/crysirror-nsystem/src/adapter.rs#L341) |
| `id` | `VOICESTOP` | Local state/value operations | Handled | [341](../../crates/crysirror-nsystem/src/adapter.rs#L341) |
| `id` | `VOICEWAIT` | Local state/value operations | Handled | [341](../../crates/crysirror-nsystem/src/adapter.rs#L341) |
| `id` | `SEPLAY` | Local state/value operations | Handled | [342](../../crates/crysirror-nsystem/src/adapter.rs#L342) |
| `id` | `SESTOP` | Local state/value operations | Handled | [342](../../crates/crysirror-nsystem/src/adapter.rs#L342) |
| `id` | `SEWAIT` | Local state/value operations | Handled | [342](../../crates/crysirror-nsystem/src/adapter.rs#L342) |
| `id` | `SEREAD` | Local state/value operations | Handled | [342](../../crates/crysirror-nsystem/src/adapter.rs#L342) |
| `id` | `SEFREE` | Local state/value operations | Handled | [343](../../crates/crysirror-nsystem/src/adapter.rs#L343) |
| `id` | `MSG` | Local state/value operations | Handled | [343](../../crates/crysirror-nsystem/src/adapter.rs#L343) |
| `id` | `MWO` | Local state/value operations | Handled | [343](../../crates/crysirror-nsystem/src/adapter.rs#L343) |
| `id` | `MWC` | Local state/value operations | Handled | [343](../../crates/crysirror-nsystem/src/adapter.rs#L343) |
| `id` | `SELECT` | Local state/value operations | Handled | [343](../../crates/crysirror-nsystem/src/adapter.rs#L343) |
| `id` | `CHKSCEXEC` | Local state/value operations | Handled | [344](../../crates/crysirror-nsystem/src/adapter.rs#L344) |
| `id` | `SUBTITLE` | Local state/value operations | Handled | [344](../../crates/crysirror-nsystem/src/adapter.rs#L344) |
| `id` | `SETCGFLAG` | Local state/value operations | Handled | [344](../../crates/crysirror-nsystem/src/adapter.rs#L344) |
| `id` | `SETSCENEFLAG` | Local state/value operations | Handled | [345](../../crates/crysirror-nsystem/src/adapter.rs#L345) |
| `id` | `SETMESSKIP` | Local state/value operations | Handled | [345](../../crates/crysirror-nsystem/src/adapter.rs#L345) |
| `id` | `SELITEM` | Local state/value operations | Handled | [345](../../crates/crysirror-nsystem/src/adapter.rs#L345) |
| `id` | `MESFTSIZE` | Local state/value operations | Handled | [346](../../crates/crysirror-nsystem/src/adapter.rs#L346) |
| `id` | `MESFTCOLOR` | Local state/value operations | Handled | [346](../../crates/crysirror-nsystem/src/adapter.rs#L346) |
| `id` | `MESFTBLOD` | Local state/value operations | Handled | [346](../../crates/crysirror-nsystem/src/adapter.rs#L346) |
| `id` | `MESFTSHADOW` | Local state/value operations | Handled | [347](../../crates/crysirror-nsystem/src/adapter.rs#L347) |
| `id` | `MESTXAREA` | Local state/value operations | Handled | [347](../../crates/crysirror-nsystem/src/adapter.rs#L347) |
| `id` | `MESTXKINSOKU` | Local state/value operations | Handled | [347](../../crates/crysirror-nsystem/src/adapter.rs#L347) |
| `id` | `MESTXDOWN` | Local state/value operations | Handled | [348](../../crates/crysirror-nsystem/src/adapter.rs#L348) |
| `id` | `MESNAME` | Local state/value operations | Handled | [348](../../crates/crysirror-nsystem/src/adapter.rs#L348) |
| `id` | `MESVPLAY` | Local state/value operations | Handled | [348](../../crates/crysirror-nsystem/src/adapter.rs#L348) |
| `id` | `MESVSTOP` | Local state/value operations | Handled | [349](../../crates/crysirror-nsystem/src/adapter.rs#L349) |
| `id` | `MESRUN` | Local state/value operations | Handled | [349](../../crates/crysirror-nsystem/src/adapter.rs#L349) |
| `id` | `BLOGSAVEBEGIN` | Local state/value operations | Handled | [349](../../crates/crysirror-nsystem/src/adapter.rs#L349) |
| `id` | `BLOGSAVEEND` | Local state/value operations | Handled | [350](../../crates/crysirror-nsystem/src/adapter.rs#L350) |
| `id` | `SELMAKE` | Local state/value operations | Handled | [350](../../crates/crysirror-nsystem/src/adapter.rs#L350) |
| `id` | `SELRUN` | Local state/value operations | Handled | [350](../../crates/crysirror-nsystem/src/adapter.rs#L350) |
| `id` | `ACTMAKE` | Local state/value operations | Handled | [351](../../crates/crysirror-nsystem/src/adapter.rs#L351) |
| `id` | `ACTFREE` | Local state/value operations | Handled | [351](../../crates/crysirror-nsystem/src/adapter.rs#L351) |
| `id` | `ACTPLAY` | Local state/value operations | Handled | [351](../../crates/crysirror-nsystem/src/adapter.rs#L351) |
| `id` | `ACTSTOP` | Local state/value operations | Handled | [351](../../crates/crysirror-nsystem/src/adapter.rs#L351) |
| `id` | `SCREENACT` | Local state/value operations | Handled | [352](../../crates/crysirror-nsystem/src/adapter.rs#L352) |
| `id` | `SAVESAM` | Local state/value operations | Handled | [352](../../crates/crysirror-nsystem/src/adapter.rs#L352) |
| `id` | `FACE` | Local state/value operations | Handled | [352](../../crates/crysirror-nsystem/src/adapter.rs#L352) |
| `id` | `MOVIEPAUSE` | Local state/value operations | Handled | [352](../../crates/crysirror-nsystem/src/adapter.rs#L352) |
| `id` | `MOVIERATE` | Local state/value operations | Handled | [353](../../crates/crysirror-nsystem/src/adapter.rs#L353) |
| `id` | `MOVIELOOP` | Local state/value operations | Handled | [353](../../crates/crysirror-nsystem/src/adapter.rs#L353) |
| `id` | `SETCHARID` | Local state/value operations | Handled | [353](../../crates/crysirror-nsystem/src/adapter.rs#L353) |
| `id` | `END` | Local state/value operations | Handled | [353](../../crates/crysirror-nsystem/src/adapter.rs#L353) |
| `id` | `0` | Local state/value operations | Handled | [354](../../crates/crysirror-nsystem/src/adapter.rs#L354) |
| `id` | `_` | Local state/value operations | Handled | [355](../../crates/crysirror-nsystem/src/adapter.rs#L355) |

### adapter.rs / NSystemVm::execute

```rust
fn execute(&mut self, h: &mut dyn HostServices, c: &Command, now: u64) -> Result<Flow>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `op; under c.id=IF` | `0` | Local state/value operations | Handled | [1158](../../crates/crysirror-nsystem/src/adapter.rs#L1158) |
| `op; under c.id=IF` | `1` | Local state/value operations | Handled | [1159](../../crates/crysirror-nsystem/src/adapter.rs#L1159) |
| `op; under c.id=IF` | `2` | Local state/value operations | Handled | [1160](../../crates/crysirror-nsystem/src/adapter.rs#L1160) |
| `op; under c.id=IF` | `3` | Local state/value operations | Handled | [1161](../../crates/crysirror-nsystem/src/adapter.rs#L1161) |
| `op; under c.id=IF` | `4` | Local state/value operations | Handled | [1162](../../crates/crysirror-nsystem/src/adapter.rs#L1162) |
| `op; under c.id=IF` | `5` | Local state/value operations | Handled | [1163](../../crates/crysirror-nsystem/src/adapter.rs#L1163) |
| `op; under c.id=IF` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [1164](../../crates/crysirror-nsystem/src/adapter.rs#L1164) |

### adapter.rs / NSystemVm::overlap

```rust
fn overlap(&mut self, h: &mut dyn HostServices, c: &Command, now: u64) -> Result<Flow>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `mode; under self.phase=1 / kind=3` | `1` | Local state/value operations | Handled | [1672](../../crates/crysirror-nsystem/src/adapter.rs#L1672) |
| `mode; under self.phase=1 / kind=3` | `2 if vague > 0` | Local state/value operations | Handled | [1673](../../crates/crysirror-nsystem/src/adapter.rs#L1673) |
| `mode; under self.phase=1 / kind=3` | `2` | Local state/value operations | Handled | [1674](../../crates/crysirror-nsystem/src/adapter.rs#L1674) |
| `mode; under self.phase=1 / kind=3` | `other` | Local state/value operations | Unsupported: OVERLAP alpha mode {other} (DrawAlpha0{other}) is not mapped | [1675](../../crates/crysirror-nsystem/src/adapter.rs#L1675) |
| `kind; under self.phase=1` | `1` | Local state/value operations | Handled | [1656](../../crates/crysirror-nsystem/src/adapter.rs#L1656) |
| `kind; under self.phase=1` | `2` | self.ints | Handled | [1658](../../crates/crysirror-nsystem/src/adapter.rs#L1658) |
| `kind; under self.phase=1` | `3` | self.value, self.string, self.asset | Handled | [1664](../../crates/crysirror-nsystem/src/adapter.rs#L1664) |
| `kind; under self.phase=1` | `5` | self.ints | Handled | [1680](../../crates/crysirror-nsystem/src/adapter.rs#L1680) |
| `kind; under self.phase=1` | `8` | self.ints, i16::try_from | Handled | [1686](../../crates/crysirror-nsystem/src/adapter.rs#L1686) |
| `kind; under self.phase=1` | `14` | self.ints | Handled | [1695](../../crates/crysirror-nsystem/src/adapter.rs#L1695) |
| `kind; under self.phase=1` | `other` | Local state/value operations | Unsupported: OVERLAP effect type {other} is not mapped | [1700](../../crates/crysirror-nsystem/src/adapter.rs#L1700) |

### adapter.rs / NSystemVm::submit

```rust
fn submit(&mut self, h: &mut dyn HostServices, mut layers: Vec<Json>, ui: Vec<Json>) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `(op["op"].as_str(), op["id"].as_u64(), op["layer"]["id"].as_u64())` | `(Some("remove"), Some(id), _)` | self.live.remove | Handled | [560](../../crates/crysirror-nsystem/src/adapter.rs#L560) |
| `(op["op"].as_str(), op["id"].as_u64(), op["layer"]["id"].as_u64())` | `(Some("put"), _, Some(id))` | self.live.insert | Handled | [561](../../crates/crysirror-nsystem/src/adapter.rs#L561) |
| `(op["op"].as_str(), op["id"].as_u64(), op["layer"]["id"].as_u64())` | `_` | Local state/value operations | Handled | [565](../../crates/crysirror-nsystem/src/adapter.rs#L565) |

### adapter.rs / NSystemVm::value

```rust
fn value(&self, o: &Operand) -> Result<i32>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `op; under o=Operand::Expr(pairs) / op=2..=6` | `2` | Local state/value operations | Handled | [513](../../crates/crysirror-nsystem/src/adapter.rs#L513) |
| `op; under o=Operand::Expr(pairs) / op=2..=6` | `3` | Local state/value operations | Handled | [514](../../crates/crysirror-nsystem/src/adapter.rs#L514) |
| `op; under o=Operand::Expr(pairs) / op=2..=6` | `4` | Local state/value operations | Handled | [515](../../crates/crysirror-nsystem/src/adapter.rs#L515) |
| `op; under o=Operand::Expr(pairs) / op=2..=6` | `5` | Local state/value operations | Handled | [516](../../crates/crysirror-nsystem/src/adapter.rs#L516) |
| `op; under o=Operand::Expr(pairs) / op=2..=6` | `_` | Local state/value operations | Handled | [517](../../crates/crysirror-nsystem/src/adapter.rs#L517) |
| `op; under o=Operand::Expr(pairs)` | `0` | Local state/value operations | Handled | [507](../../crates/crysirror-nsystem/src/adapter.rs#L507) |
| `op; under o=Operand::Expr(pairs)` | `1` | self.var | Handled | [508](../../crates/crysirror-nsystem/src/adapter.rs#L508) |
| `op; under o=Operand::Expr(pairs)` | `2..=6` | Local state/value operations | Handled | [509](../../crates/crysirror-nsystem/src/adapter.rs#L509) |
| `op; under o=Operand::Expr(pairs)` | `other` | Local state/value operations | Unsupported: N-System expression operator {other} | [520](../../crates/crysirror-nsystem/src/adapter.rs#L520) |

### adapter/omake.rs / NSystemVm::entries

```rust
fn entries(&self, mode: usize, character: usize) -> usize
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `mode == 0` | self.lists.cg.iter | Handled | [84](../../crates/crysirror-nsystem/src/adapter/omake.rs#L84) |

### adapter/omake.rs / NSystemVm::entry_open

```rust
fn entry_open(&self, mode: usize, character: usize, entry: usize) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `mode == 0` | self.system.cg.get | Handled | [90](../../crates/crysirror-nsystem/src/adapter/omake.rs#L90) |

### adapter/omake.rs / NSystemVm::grid_sheet

```rust
fn grid_sheet(&self, mode: usize, character: usize, page: usize) -> Result<Option<String>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `mode == 0` | Local state/value operations | Handled | [72](../../crates/crysirror-nsystem/src/adapter/omake.rs#L72) |
| `condition` | `c.operands[0] == Operand::Int(5) && c.operands[1] == Operand::Int(key)` | self.value, self.string | Handled | [74](../../crates/crysirror-nsystem/src/adapter/omake.rs#L74) |

### adapter/omake.rs / NSystemVm::omake_layers

```rust
fn omake_layers(&mut self, h: &mut dyn HostServices, o: &Omake) -> Result<Vec<Json>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `o.mode == 0` | Local state/value operations | Handled | [151](../../crates/crysirror-nsystem/src/adapter/omake.rs#L151) |

### adapter/omake.rs / NSystemVm::omake_mode

```rust
fn omake_mode(&mut self, o: &mut Omake, mode: usize) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `mode == 0` | Local state/value operations | Handled | [106](../../crates/crysirror-nsystem/src/adapter/omake.rs#L106) |
| `condition` | `mode == 0` | Local state/value operations | Handled | [107](../../crates/crysirror-nsystem/src/adapter/omake.rs#L107) |
| `condition` | `mode == 0` | Local state/value operations | Handled | [108](../../crates/crysirror-nsystem/src/adapter/omake.rs#L108) |

### adapter/omake.rs / NSystemVm::omake_replay

```rust
fn omake_replay(&mut self, h: &mut dyn HostServices, o: Box<Omake>, entry: usize) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `o.mode == 0` | self.omake_script, self.value, self.lists.cg.iter, self.slots.clear, self.system.cg.get | Handled | [240](../../crates/crysirror-nsystem/src/adapter/omake.rs#L240) |

### adapter/slots.rs / NSystemVm::apply_slot

```rust
fn apply_slot(&mut self, h: &mut dyn HostServices, slot: &Slot) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `a.kind` | `0` | Local state/value operations | Empty handler: Original no-op status not established | [221](../../crates/crysirror-nsystem/src/adapter/slots.rs#L221) |
| `a.kind` | `2` | i32::from_le_bytes, usize::try_from, u64::from | Handled | [222](../../crates/crysirror-nsystem/src/adapter/slots.rs#L222) |
| `a.kind` | `k` | Local state/value operations | Unsupported: N-System slot action family {k} is not mapped | [230](../../crates/crysirror-nsystem/src/adapter/slots.rs#L230) |

### save.rs / read_screen

```rust
fn read_screen(b: &[u8]) -> Result<Screen>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `kind` | `0` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [221](../../crates/crysirror-nsystem/src/save.rs#L221) |
| `kind` | `2` | Local state/value operations | Handled | [222](../../crates/crysirror-nsystem/src/save.rs#L222) |
| `kind` | `other` | Local state/value operations | Unsupported: N-System save action family {other} has no reviewed layout | [223](../../crates/crysirror-nsystem/src/save.rs#L223) |

### save.rs / write_screen

```rust
fn write_screen(s: &Screen) -> Result<Vec<u8>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `(a.kind, a.elapsed)` | `(0, None)` | Local state/value operations | Empty handler: Original no-op status not established | [271](../../crates/crysirror-nsystem/src/save.rs#L271) |
| `(a.kind, a.elapsed)` | `(2, Some(e))` | Local state/value operations | Handled | [272](../../crates/crysirror-nsystem/src/save.rs#L272) |
| `(a.kind, a.elapsed)` | `(k, _)` | Local state/value operations | Unsupported: N-System save action family {k} has no reviewed layout | [273](../../crates/crysirror-nsystem/src/save.rs#L273) |

### script.rs / operand

```rust
fn operand(b: &[u8], mut i: usize, end: usize) -> Result<(Operand, usize)>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under tag=4` | `op == -1` | Operand::Expr | Handled | [130](../../crates/crysirror-nsystem/src/script.rs#L130) |
