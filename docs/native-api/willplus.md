# WillPlus / AdvHD developer reference

[All engines](README.md)

Mode: `adopt`.

## ABI and host contract

WS2 **V1/V2/V3** operand layouts are recognized separately; the runtime executor
is **V3**. Scripts use ror-2 decoding. Meanings are inferred from reviewed script
operands/call sites because the installed AdvHD executable is packed; these
inferences are not a native-handler parity claim. Lua host calls are addressed
by global name or `(class, method)`, independently of WS2 opcodes.

| Native area | CrySirror service / implementation | Return / completion |
| --- | --- | --- |
| WS2 control, variables, flags and Lua calls | `vm.rs` and shared Lua interpreter | Native scene/PC and script-held menus remain in the adapter. |
| `cfunc`, `cclass`, `os` and `io` | `system.rs` and VM host methods | Native arguments/results; class/method and library scopes remain distinct. |
| Layer sets, PNA, effects and transitions | Shared Image/Layer service | Tree flattens in native depth-first order; resources/tweens can wait. |
| Message windows, choices and backlog | Shared GlyphRun/UI | Script window keys and wait/page policy. |
| Audio / movie | Shared Audio/Media | Native channel state, volume and completion. |
| ARC resources / `save\` files | Scoped Storage and shared ARC readers | Ordered mounts, read-only game data and writable Save namespace. |

Desktop detection uses `AdvHD.exe` and Pulltop-layout ARCs, canvas 1280×720,
start scene `START`. Other callers must supply archive order, entry and profile.
`0x42` attributes and `0x46` groups 10/11 are retained without visible semantics;
weather meanings at `0x56/0x5C` remain inferred. V1/V2 decode rows are not runtime
support. Other unlisted opcodes/methods fail visibly.

Source/provenance: [crate](../../crates/crysirror-willplus/),
[WS2 layouts](../../crates/crysirror-willplus/src/tables.rs),
[executor and Lua methods](../../crates/crysirror-willplus/src/vm.rs).
Generation establishes documentation consistency, not a platform/game run.

<!-- BEGIN GENERATED DISPATCH TABLES -->

## Opcode and API implementation tables

`Handled` means an executable source branch exists. It is not a claim that every argument/subselector or a complete game has been audited. Explicit gaps are shown per row.

| Status | Entries |
| --- | ---: |
| Decode only | 340 |
| Empty handler | 7 |
| Handled | 164 |
| Partial | 8 |

These tables cover the checked-in catalogues and explicit dispatch branches. An engine-wide upstream denominator is not inferred from these rows.

## Lua class dispatch / Bridge<'_>

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `("CLegacyFontInfo", add)` | add | Handled | — | [vm.rs:1767](../../crates/crysirror-willplus/src/vm.rs#L1767) |
| `("CLegacyFontInfo", changeColor)` | change Color | Handled | — | [vm.rs:1776](../../crates/crysirror-willplus/src/vm.rs#L1776) |
| `("CLegacyFontInfo", create)` | create | Handled | — | [vm.rs:1766](../../crates/crysirror-willplus/src/vm.rs#L1766) |
| `("CLegacyFontInfo", remove)` | remove | Handled | — | [vm.rs:1772](../../crates/crysirror-willplus/src/vm.rs#L1772) |
| `("lua_AutoCursor", AutoCursor)` | Pointer warp toward a button (mouseAutoMove setting): the host does not move the OS pointer, so no auto-move is ever ru… | Handled | — | [vm.rs:1805](../../crates/crysirror-willplus/src/vm.rs#L1805) |
| `("lua_AutoCursor", GetCursorPos)` | Get Cursor Pos | Handled | — | [vm.rs:1807](../../crates/crysirror-willplus/src/vm.rs#L1807) |
| `("lua_AutoCursor", IsMove)` | Is Move | Handled | — | [vm.rs:1806](../../crates/crysirror-willplus/src/vm.rs#L1806) |
| `("lua_AutoCursor", SetCursor)` | Set Cursor | Handled | — | [vm.rs:1802](../../crates/crysirror-willplus/src/vm.rs#L1802) |
| `("lua_DataSort", DeleteData)` | Delete Data | Handled | — | [vm.rs:1829](../../crates/crysirror-willplus/src/vm.rs#L1829) |
| `("lua_DataSort", GetProtect)` | Get Protect | Handled | — | [vm.rs:1824](../../crates/crysirror-willplus/src/vm.rs#L1824) |
| `("lua_DataSort", Insert)` | Insert | Handled | — | [vm.rs:1816](../../crates/crysirror-willplus/src/vm.rs#L1816) |
| `("lua_DataSort", ResetMark)` | Save-list edit object: Insert lists a normal slot, Toggle marks it for deletion, DeleteData removes the marked slots | Handled | — | [vm.rs:1812](../../crates/crysirror-willplus/src/vm.rs#L1812) |
| `("lua_DataSort", Toggle)` | Toggle | Handled | — | [vm.rs:1817](../../crates/crysirror-willplus/src/vm.rs#L1817) |
| `("lua_DataSort", size)` | size | Handled | — | [vm.rs:1825](../../crates/crysirror-willplus/src/vm.rs#L1825) |
| `("lua_MeasureTime", diff)` | Scale for effect delays measured since construction (1 = as authored) | Handled | — | [vm.rs:1809](../../crates/crysirror-willplus/src/vm.rs#L1809) |
| `("lua_Mixer", getMasterMute)` | get Master Mute | Handled | — | [vm.rs:1847](../../crates/crysirror-willplus/src/vm.rs#L1847) |
| `("lua_Mixer", getMasterVolume)` | Master volume 0..1000 and mute (config SOUND page); applied to every voice's gain at start | Handled | — | [vm.rs:1840](../../crates/crysirror-willplus/src/vm.rs#L1840) |
| `("lua_Mixer", setMasterMute)` | set Master Mute | Handled | — | [vm.rs:1848](../../crates/crysirror-willplus/src/vm.rs#L1848) |
| `("lua_Mixer", setMasterVolume)` | set Master Volume | Handled | — | [vm.rs:1841](../../crates/crysirror-willplus/src/vm.rs#L1841) |
| `("lua_Registry", GetDWORD)` | Get DWORD | Handled | — | [vm.rs:1853](../../crates/crysirror-willplus/src/vm.rs#L1853) |
| `("lua_Registry", IsKeyDWORD)` | Is Key DWORD | Handled | — | [vm.rs:1854](../../crates/crysirror-willplus/src/vm.rs#L1854) |
| `("lua_Registry", SetDWORD)` | Set DWORD | Handled | — | [vm.rs:1855](../../crates/crysirror-willplus/src/vm.rs#L1855) |
| `("lua_Sounds", BgmPlay)` | Bgm Play | Handled | — | [vm.rs:1783](../../crates/crysirror-willplus/src/vm.rs#L1783) |
| `("lua_Sounds", BgmStop)` | Bgm Stop | Handled | — | [vm.rs:1794](../../crates/crysirror-willplus/src/vm.rs#L1794) |
| `("lua_Sounds", IsPlaying)` | Is Playing | Handled | — | [vm.rs:1801](../../crates/crysirror-willplus/src/vm.rs#L1801) |
| `("lua_Sounds", SndPlay)` | Snd Play | Handled | — | [vm.rs:1783](../../crates/crysirror-willplus/src/vm.rs#L1783) |
| `("lua_Sounds", SndStop)` | Snd Stop | Handled | — | [vm.rs:1794](../../crates/crysirror-willplus/src/vm.rs#L1794) |
| `(_, delete)` | delete | Handled | — | [vm.rs:1759](../../crates/crysirror-willplus/src/vm.rs#L1759) |
| `(_, new)` | Construct an object | Handled | — | [vm.rs:1752](../../crates/crysirror-willplus/src/vm.rs#L1752) |

## Lua host API / layers / call / Layers

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `add` | add | Partial | AdvHD lua_Layers:add with text-style arguments ({} values) is not connected | [layers.rs:376](../../crates/crysirror-willplus/src/layers.rs#L376) |
| `cancelEffect` | cancel Effect | Handled | — | [layers.rs:677](../../crates/crysirror-willplus/src/layers.rs#L677) |
| `cancelSubLayerEffect` | cancel Sub Layer Effect | Handled | — | [layers.rs:677](../../crates/crysirror-willplus/src/layers.rs#L677) |
| `clearHitTestAll` | clear Hit Test All | Handled | — | [layers.rs:630](../../crates/crysirror-willplus/src/layers.rs#L630) |
| `clearText` | clear Text | Handled | — | [layers.rs:518](../../crates/crysirror-willplus/src/layers.rs#L518) |
| `copySnap` | copy Snap | Handled | — | [layers.rs:783](../../crates/crysirror-willplus/src/layers.rs#L783) |
| `create` | create | Handled | — | [layers.rs:375](../../crates/crysirror-willplus/src/layers.rs#L375) |
| `delete` | delete | Handled | — | [layers.rs:375](../../crates/crysirror-willplus/src/layers.rs#L375) |
| `effectText` | effectText reveals the text at the message speed, then the host calls onMessageOutEnd(set) (config TEXT page sample) | Handled | — | [layers.rs:493](../../crates/crysirror-willplus/src/layers.rs#L493) |
| `forwardLayerOrder` | forward Layer Order | Handled | — | [layers.rs:719](../../crates/crysirror-willplus/src/layers.rs#L719) |
| `getAnimationState` | get Animation State | Handled | — | [layers.rs:762](../../crates/crysirror-willplus/src/layers.rs#L762) |
| `getCursorPos` | get Cursor Pos | Handled | — | [layers.rs:639](../../crates/crysirror-willplus/src/layers.rs#L639) |
| `getParam` | get Param | Handled | — | [layers.rs:368](../../crates/crysirror-willplus/src/layers.rs#L368) |
| `getScreenAspect` | get Screen Aspect | Handled | — | [layers.rs:776](../../crates/crysirror-willplus/src/layers.rs#L776) |
| `getSubLayerParam` | get Sub Layer Param | Handled | — | [layers.rs:369](../../crates/crysirror-willplus/src/layers.rs#L369) |
| `initSubLayer` | init Sub Layer | Handled | — | [layers.rs:534](../../crates/crysirror-willplus/src/layers.rs#L534) |
| `insertLayer` | insert Layer | Handled | — | [layers.rs:400](../../crates/crysirror-willplus/src/layers.rs#L400) |
| `insertLogPreviewLayer` | insert Log Preview Layer | Handled | — | [layers.rs:452](../../crates/crysirror-willplus/src/layers.rs#L452) |
| `insertPNARefLayer` | insert PNARef Layer | Handled | — | [layers.rs:428](../../crates/crysirror-willplus/src/layers.rs#L428) |
| `insertPreviewLayer` | insert Preview Layer | Handled | — | [layers.rs:463](../../crates/crysirror-willplus/src/layers.rs#L463) |
| `insertTextLayer` | insert Text Layer | Handled | — | [layers.rs:473](../../crates/crysirror-willplus/src/layers.rs#L473) |
| `insertThumbnailLayer` | insert Thumbnail Layer | Handled | — | [layers.rs:443](../../crates/crysirror-willplus/src/layers.rs#L443) |
| `isEffect` | is Effect | Handled | — | [layers.rs:693](../../crates/crysirror-willplus/src/layers.rs#L693) |
| `isMosaic` | is Mosaic | Handled | — | [layers.rs:777](../../crates/crysirror-willplus/src/layers.rs#L777) |
| `isTransition` | is Transition | Handled | — | [layers.rs:715](../../crates/crysirror-willplus/src/layers.rs#L715) |
| `isVisible` | is Visible | Handled | — | [layers.rs:619](../../crates/crysirror-willplus/src/layers.rs#L619) |
| `messageLength` | message Length | Handled | — | [layers.rs:510](../../crates/crysirror-willplus/src/layers.rs#L510) |
| `remove` | remove | Handled | — | [layers.rs:388](../../crates/crysirror-willplus/src/layers.rs#L388) |
| `removeLayer` | remove Layer | Handled | — | [layers.rs:527](../../crates/crysirror-willplus/src/layers.rs#L527) |
| `removeSnap` | remove Snap | Handled | — | [layers.rs:784](../../crates/crysirror-willplus/src/layers.rs#L784) |
| `rtSnap` | rt Snap | Handled | — | [layers.rs:783](../../crates/crysirror-willplus/src/layers.rs#L783) |
| `setAnimation` | set Animation | Handled | — | [layers.rs:734](../../crates/crysirror-willplus/src/layers.rs#L734) |
| `setBlurCount` | Snapshot blur radius for menu backgrounds (snapshots compose live) | Handled | — | [layers.rs:775](../../crates/crysirror-willplus/src/layers.rs#L775) |
| `setEffect` | set Effect | Handled | — | [layers.rs:646](../../crates/crysirror-willplus/src/layers.rs#L646) |
| `setForceBG` | set Force BG | Handled | — | [layers.rs:788](../../crates/crysirror-willplus/src/layers.rs#L788) |
| `setOverlaySet` | set Overlay Set | Handled | — | [layers.rs:767](../../crates/crysirror-willplus/src/layers.rs#L767) |
| `setParam` | set Param | Handled | — | [layers.rs:583](../../crates/crysirror-willplus/src/layers.rs#L583) |
| `setSnap` | setSnap2 re-takes an existing snapshot (log redraw) | Handled | — | [layers.rs:779](../../crates/crysirror-willplus/src/layers.rs#L779) |
| `setSnap2` | setSnap2 re-takes an existing snapshot (log redraw) | Handled | — | [layers.rs:779](../../crates/crysirror-willplus/src/layers.rs#L779) |
| `setSubLayerEffect` | set Sub Layer Effect | Handled | — | [layers.rs:646](../../crates/crysirror-willplus/src/layers.rs#L646) |
| `setSubLayerHitTest` | set Sub Layer Hit Test | Handled | — | [layers.rs:623](../../crates/crysirror-willplus/src/layers.rs#L623) |
| `setSubLayerParam` | set Sub Layer Param | Handled | — | [layers.rs:547](../../crates/crysirror-willplus/src/layers.rs#L547) |
| `setSubLayerVisible` | set Sub Layer Visible | Handled | — | [layers.rs:541](../../crates/crysirror-willplus/src/layers.rs#L541) |
| `setText` | effectText reveals the text at the message speed, then the host calls onMessageOutEnd(set) (config TEXT page sample) | Handled | — | [layers.rs:493](../../crates/crysirror-willplus/src/layers.rs#L493) |
| `setVisible` | set Visible | Handled | — | [layers.rs:613](../../crates/crysirror-willplus/src/layers.rs#L613) |
| `startAnimation` | start Animation | Handled | — | [layers.rs:743](../../crates/crysirror-willplus/src/layers.rs#L743) |
| `startEffect` | start Effect | Handled | — | [layers.rs:665](../../crates/crysirror-willplus/src/layers.rs#L665) |
| `startSubLayerEffect` | start Sub Layer Effect | Handled | — | [layers.rs:665](../../crates/crysirror-willplus/src/layers.rs#L665) |
| `startTransition` | start Transition | Handled | — | [layers.rs:700](../../crates/crysirror-willplus/src/layers.rs#L700) |
| `stopAnimation` | stop Animation | Handled | — | [layers.rs:753](../../crates/crysirror-willplus/src/layers.rs#L753) |
| `swapLayerOrder` | swap Layer Order | Handled | — | [layers.rs:719](../../crates/crysirror-willplus/src/layers.rs#L719) |
| `updateTextColor` | Text layers read their CLegacyFontInfo colours at composition | Handled | — | [layers.rs:773](../../crates/crysirror-willplus/src/layers.rs#L773) |

## Lua host API / log / call

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `getIndexLogLayerText` | get Index Log Layer Text | Handled | — | [log.rs:168](../../crates/crysirror-willplus/src/log.rs#L168) |
| `getLogLineToIndex` | get Log Line To Index | Handled | — | [log.rs:164](../../crates/crysirror-willplus/src/log.rs#L164) |
| `getLogTextDrawCount` | get Log Text Draw Count | Handled | — | [log.rs:137](../../crates/crysirror-willplus/src/log.rs#L137) |
| `getLogTextLine` | get Log Text Line | Handled | — | [log.rs:138](../../crates/crysirror-willplus/src/log.rs#L138) |
| `getLogTextRect` | get Log Text Rect | Handled | — | [log.rs:162](../../crates/crysirror-willplus/src/log.rs#L162) |
| `getLogTextRectIndex` | get Log Text Rect Index | Handled | — | [log.rs:163](../../crates/crysirror-willplus/src/log.rs#L163) |
| `hitTestLogLayerText` | hit Test Log Layer Text | Handled | — | [log.rs:151](../../crates/crysirror-willplus/src/log.rs#L151) |
| `onWheelMouseLogLayer` | on Wheel Mouse Log Layer | Handled | — | [log.rs:139](../../crates/crysirror-willplus/src/log.rs#L139) |
| `pageUpLogText` | page Up Log Text | Handled | — | [log.rs:146](../../crates/crysirror-willplus/src/log.rs#L146) |
| `stopLogVoice` | stop Log Voice | Handled | — | [log.rs:172](../../crates/crysirror-willplus/src/log.rs#L172) |

## Lua host API / vm / cfunc / Bridge<'_>

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `CSaveDataManager__lua_FindNewData` | (mode, ..) -> (mode of the newest data, its slot); slot 0 when empty | Handled | — | [vm.rs:1701](../../crates/crysirror-willplus/src/vm.rs#L1701) |
| `CSaveDataManager__lua_GetEnabled` | CSave Data Manager lua Get Enabled | Handled | — | [vm.rs:1693](../../crates/crysirror-willplus/src/vm.rs#L1693) |
| `CSaveDataManager__lua_GetSaveDataDate` | CSave Data Manager lua Get Save Data Date | Handled | — | [vm.rs:1694](../../crates/crysirror-willplus/src/vm.rs#L1694) |
| `CSaveDataManager__lua_GetSaveDataText` | CSave Data Manager lua Get Save Data Text | Handled | — | [vm.rs:1695](../../crates/crysirror-willplus/src/vm.rs#L1695) |
| `CSaveDataManager__lua_GetSavePath` | CSave Data Manager lua Get Save Path | Handled | — | [vm.rs:1707](../../crates/crysirror-willplus/src/vm.rs#L1707) |
| `CSaveDataManager__lua_GetThumbnailPath` | CSave Data Manager lua Get Thumbnail Path | Handled | — | [vm.rs:1696](../../crates/crysirror-willplus/src/vm.rs#L1696) |
| `CSaveDataManager__lua_IsData` | CSave Data Manager lua Is Data | Handled | — | [vm.rs:1693](../../crates/crysirror-willplus/src/vm.rs#L1693) |
| `CSaveDataManager__lua_IsResume` | CSave Data Manager lua Is Resume | Handled | — | [vm.rs:1692](../../crates/crysirror-willplus/src/vm.rs#L1692) |
| `D3DApp__lua_PostMouseMove` | Re-deliver the pointer next frame (hover refresh after a menu) | Handled | — | [vm.rs:1668](../../crates/crysirror-willplus/src/vm.rs#L1668) |
| `LegacyGame__lua_GetFlag` | Legacy Game lua Get Flag | Handled | — | [vm.rs:1618](../../crates/crysirror-willplus/src/vm.rs#L1618) |
| `LegacyGame__lua_GetGameState` | Legacy Game lua Get Game State | Handled | — | [vm.rs:1599](../../crates/crysirror-willplus/src/vm.rs#L1599) |
| `LegacyGame__lua_GetString` | Legacy Game lua Get String | Handled | — | [vm.rs:1643](../../crates/crysirror-willplus/src/vm.rs#L1643) |
| `LegacyGame__lua_GetVariable` | Legacy Game lua Get Variable | Handled | — | [vm.rs:1633](../../crates/crysirror-willplus/src/vm.rs#L1633) |
| `LegacyGame__lua_NeedPreview` | Config display-mode preview (window/fullscreen are host policy) | Handled | — | [vm.rs:1681](../../crates/crysirror-willplus/src/vm.rs#L1681) |
| `LegacyGame__lua_NeedProcess` | Legacy Game lua Need Process | Handled | — | [vm.rs:1648](../../crates/crysirror-willplus/src/vm.rs#L1648) |
| `LegacyGame__lua_NeedShell` | Opening a web page (survey / site links) is not done from game data; the request is reported | Handled | — | [vm.rs:1684](../../crates/crysirror-willplus/src/vm.rs#L1684) |
| `LegacyGame__lua_NeedVolumeUpdate` | Volume settings changed for a category: playing voices take their new gains after this call | Handled | — | [vm.rs:1674](../../crates/crysirror-willplus/src/vm.rs#L1674) |
| `LegacyGame__lua_SelectItem` | Legacy Game lua Select Item | Handled | — | [vm.rs:1611](../../crates/crysirror-willplus/src/vm.rs#L1611) |
| `LegacyGame__lua_SetFlag` | Legacy Game lua Set Flag | Handled | — | [vm.rs:1619](../../crates/crysirror-willplus/src/vm.rs#L1619) |
| `LegacyGame__lua_SetGameState` | One argument toggles: hide/unhide, skip on/off and the DISABLEDATA backup/restore pairs all call it that way | Handled | — | [vm.rs:1602](../../crates/crysirror-willplus/src/vm.rs#L1602) |
| `LegacyGame__lua_SetLoadProc` | Load hook names (cast-off/tips features this title disables) | Handled | — | [vm.rs:1679](../../crates/crysirror-willplus/src/vm.rs#L1679) |
| `LegacyGame__lua_SetSaveComment` | Legacy Game lua Set Save Comment | Handled | — | [vm.rs:1688](../../crates/crysirror-willplus/src/vm.rs#L1688) |
| `LegacyGame__lua_SetString` | Legacy Game lua Set String | Handled | — | [vm.rs:1644](../../crates/crysirror-willplus/src/vm.rs#L1644) |
| `LegacyGame__lua_SetVariable` | Legacy Game lua Set Variable | Handled | — | [vm.rs:1638](../../crates/crysirror-willplus/src/vm.rs#L1638) |
| `MyConv__lua_sjistoutf8` | My Conv lua sjistoutf8 | Handled | — | [vm.rs:1715](../../crates/crysirror-willplus/src/vm.rs#L1715) |
| `MyConv__lua_utf8tosjis` | My Conv lua utf8tosjis | Handled | — | [vm.rs:1708](../../crates/crysirror-willplus/src/vm.rs#L1708) |

## WS2 V1 opcodes

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x00` | Operand layout: none | Decode only | No executor for this profile | [tables.rs:17](../../crates/crysirror-willplus/src/tables.rs#L17) |
| `0x01` | Operand layout: bhfaa | Decode only | No executor for this profile | [tables.rs:18](../../crates/crysirror-willplus/src/tables.rs#L18) |
| `0x02` | Operand layout: a | Decode only | No executor for this profile | [tables.rs:19](../../crates/crysirror-willplus/src/tables.rs#L19) |
| `0x04` | Operand layout: s | Decode only | No executor for this profile | [tables.rs:20](../../crates/crysirror-willplus/src/tables.rs#L20) |
| `0x05` | Operand layout: none | Decode only | No executor for this profile | [tables.rs:21](../../crates/crysirror-willplus/src/tables.rs#L21) |
| `0x06` | Operand layout: a | Decode only | No executor for this profile | [tables.rs:22](../../crates/crysirror-willplus/src/tables.rs#L22) |
| `0x07` | Operand layout: s | Decode only | No executor for this profile | [tables.rs:23](../../crates/crysirror-willplus/src/tables.rs#L23) |
| `0x08` | Operand layout: b | Decode only | No executor for this profile | [tables.rs:24](../../crates/crysirror-willplus/src/tables.rs#L24) |
| `0x09` | Operand layout: bhf | Decode only | No executor for this profile | [tables.rs:25](../../crates/crysirror-willplus/src/tables.rs#L25) |
| `0x0A` | Operand layout: hf | Decode only | No executor for this profile | [tables.rs:26](../../crates/crysirror-willplus/src/tables.rs#L26) |
| `0x0B` | Operand layout: hb | Decode only | No executor for this profile | [tables.rs:27](../../crates/crysirror-willplus/src/tables.rs#L27) |
| `0x0C` | Operand layout: hbh* | Decode only | No executor for this profile | [tables.rs:28](../../crates/crysirror-willplus/src/tables.rs#L28) |
| `0x0D` | Operand layout: hhf | Decode only | No executor for this profile | [tables.rs:29](../../crates/crysirror-willplus/src/tables.rs#L29) |
| `0x0E` | Operand layout: hhb | Decode only | No executor for this profile | [tables.rs:30](../../crates/crysirror-willplus/src/tables.rs#L30) |
| `0x0F` | Operand layout: b | Decode only | No executor for this profile | [tables.rs:31](../../crates/crysirror-willplus/src/tables.rs#L31) |
| `0x11` | Operand layout: sf | Decode only | No executor for this profile | [tables.rs:32](../../crates/crysirror-willplus/src/tables.rs#L32) |
| `0x12` | Operand layout: sbs | Decode only | No executor for this profile | [tables.rs:33](../../crates/crysirror-willplus/src/tables.rs#L33) |
| `0x13` | Operand layout: none | Decode only | No executor for this profile | [tables.rs:34](../../crates/crysirror-willplus/src/tables.rs#L34) |
| `0x14` | Operand layout: iss | Decode only | No executor for this profile | [tables.rs:35](../../crates/crysirror-willplus/src/tables.rs#L35) |
| `0x15` | Operand layout: s | Decode only | No executor for this profile | [tables.rs:36](../../crates/crysirror-willplus/src/tables.rs#L36) |
| `0x16` | Operand layout: b | Decode only | No executor for this profile | [tables.rs:37](../../crates/crysirror-willplus/src/tables.rs#L37) |
| `0x17` | Operand layout: none | Decode only | No executor for this profile | [tables.rs:38](../../crates/crysirror-willplus/src/tables.rs#L38) |
| `0x18` | Operand layout: bs | Decode only | No executor for this profile | [tables.rs:39](../../crates/crysirror-willplus/src/tables.rs#L39) |
| `0x19` | Operand layout: none | Decode only | No executor for this profile | [tables.rs:40](../../crates/crysirror-willplus/src/tables.rs#L40) |
| `0x1A` | Operand layout: s | Decode only | No executor for this profile | [tables.rs:41](../../crates/crysirror-willplus/src/tables.rs#L41) |
| `0x1B` | Operand layout: b | Decode only | No executor for this profile | [tables.rs:42](../../crates/crysirror-willplus/src/tables.rs#L42) |
| `0x1C` | Operand layout: ssh | Decode only | No executor for this profile | [tables.rs:43](../../crates/crysirror-willplus/src/tables.rs#L43) |
| `0x1D` | Operand layout: h | Decode only | No executor for this profile | [tables.rs:44](../../crates/crysirror-willplus/src/tables.rs#L44) |
| `0x1E` | Operand layout: ssffhhb | Decode only | No executor for this profile | [tables.rs:45](../../crates/crysirror-willplus/src/tables.rs#L45) |
| `0x1F` | Operand layout: sf | Decode only | No executor for this profile | [tables.rs:46](../../crates/crysirror-willplus/src/tables.rs#L46) |
| `0x20` | Operand layout: sfh | Decode only | No executor for this profile | [tables.rs:47](../../crates/crysirror-willplus/src/tables.rs#L47) |
| `0x21` | Operand layout: shhh | Decode only | No executor for this profile | [tables.rs:48](../../crates/crysirror-willplus/src/tables.rs#L48) |
| `0x22` | Operand layout: sb | Decode only | No executor for this profile | [tables.rs:49](../../crates/crysirror-willplus/src/tables.rs#L49) |
| `0x28` | Operand layout: ssffhhbhhb | Decode only | No executor for this profile | [tables.rs:50](../../crates/crysirror-willplus/src/tables.rs#L50) |
| `0x29` | Operand layout: sf | Decode only | No executor for this profile | [tables.rs:51](../../crates/crysirror-willplus/src/tables.rs#L51) |
| `0x2A` | Operand layout: sfh | Decode only | No executor for this profile | [tables.rs:52](../../crates/crysirror-willplus/src/tables.rs#L52) |
| `0x2B` | Operand layout: s | Decode only | No executor for this profile | [tables.rs:53](../../crates/crysirror-willplus/src/tables.rs#L53) |
| `0x2C` | Operand layout: s | Decode only | No executor for this profile | [tables.rs:54](../../crates/crysirror-willplus/src/tables.rs#L54) |
| `0x2D` | Operand layout: sb | Decode only | No executor for this profile | [tables.rs:55](../../crates/crysirror-willplus/src/tables.rs#L55) |
| `0x2E` | Operand layout: none | Decode only | No executor for this profile | [tables.rs:56](../../crates/crysirror-willplus/src/tables.rs#L56) |
| `0x2F` | Operand layout: shf | Decode only | No executor for this profile | [tables.rs:57](../../crates/crysirror-willplus/src/tables.rs#L57) |
| `0x32` | Operand layout: s | Decode only | No executor for this profile | [tables.rs:58](../../crates/crysirror-willplus/src/tables.rs#L58) |
| `0x33` | Operand layout: ssbb | Decode only | No executor for this profile | [tables.rs:59](../../crates/crysirror-willplus/src/tables.rs#L59) |
| `0x34` | Operand layout: ssbb | Decode only | No executor for this profile | [tables.rs:60](../../crates/crysirror-willplus/src/tables.rs#L60) |
| `0x35` | Operand layout: ssbbb | Decode only | No executor for this profile | [tables.rs:61](../../crates/crysirror-willplus/src/tables.rs#L61) |
| `0x36` | Operand layout: sfffffffbb | Decode only | No executor for this profile | [tables.rs:62](../../crates/crysirror-willplus/src/tables.rs#L62) |
| `0x37` | Operand layout: s | Decode only | No executor for this profile | [tables.rs:63](../../crates/crysirror-willplus/src/tables.rs#L63) |
| `0x38` | Operand layout: sb | Decode only | No executor for this profile | [tables.rs:64](../../crates/crysirror-willplus/src/tables.rs#L64) |
| `0x39` | Operand layout: sbbh* | Decode only | No executor for this profile | [tables.rs:65](../../crates/crysirror-willplus/src/tables.rs#L65) |
| `0x3A` | Operand layout: sbb | Decode only | No executor for this profile | [tables.rs:66](../../crates/crysirror-willplus/src/tables.rs#L66) |
| `0x3B` | Operand layout: sshhhffffffff | Decode only | No executor for this profile | [tables.rs:67](../../crates/crysirror-willplus/src/tables.rs#L67) |
| `0x3C` | Operand layout: s | Decode only | No executor for this profile | [tables.rs:68](../../crates/crysirror-willplus/src/tables.rs#L68) |
| `0x3D` | Operand layout: h | Decode only | No executor for this profile | [tables.rs:69](../../crates/crysirror-willplus/src/tables.rs#L69) |
| `0x3E` | Operand layout: none | Decode only | No executor for this profile | [tables.rs:70](../../crates/crysirror-willplus/src/tables.rs#L70) |
| `0x3F` | Operand layout: s* | Decode only | No executor for this profile | [tables.rs:71](../../crates/crysirror-willplus/src/tables.rs#L71) |
| `0x40` | Operand layout: ssb | Decode only | No executor for this profile | [tables.rs:72](../../crates/crysirror-willplus/src/tables.rs#L72) |
| `0x41` | Operand layout: sb | Decode only | No executor for this profile | [tables.rs:73](../../crates/crysirror-willplus/src/tables.rs#L73) |
| `0x42` | Operand layout: sh | Decode only | No executor for this profile | [tables.rs:74](../../crates/crysirror-willplus/src/tables.rs#L74) |
| `0x43` | Operand layout: s | Decode only | No executor for this profile | [tables.rs:75](../../crates/crysirror-willplus/src/tables.rs#L75) |
| `0x44` | Operand layout: ssb | Decode only | No executor for this profile | [tables.rs:76](../../crates/crysirror-willplus/src/tables.rs#L76) |
| `0x45` | Operand layout: shffff | Decode only | No executor for this profile | [tables.rs:77](../../crates/crysirror-willplus/src/tables.rs#L77) |
| `0x46` | Operand layout: shbffff | Decode only | No executor for this profile | [tables.rs:78](../../crates/crysirror-willplus/src/tables.rs#L78) |
| `0x47` | Operand layout: sshbbfffffhf | Decode only | No executor for this profile | [tables.rs:79](../../crates/crysirror-willplus/src/tables.rs#L79) |
| `0x48` | Operand layout: sshbbs | Decode only | No executor for this profile | [tables.rs:80](../../crates/crysirror-willplus/src/tables.rs#L80) |
| `0x49` | Operand layout: sss | Decode only | No executor for this profile | [tables.rs:81](../../crates/crysirror-willplus/src/tables.rs#L81) |
| `0x4A` | Operand layout: ss | Decode only | No executor for this profile | [tables.rs:82](../../crates/crysirror-willplus/src/tables.rs#L82) |
| `0x4B` | Operand layout: shhffff | Decode only | No executor for this profile | [tables.rs:83](../../crates/crysirror-willplus/src/tables.rs#L83) |
| `0x4C` | Operand layout: shhbffff | Decode only | No executor for this profile | [tables.rs:84](../../crates/crysirror-willplus/src/tables.rs#L84) |
| `0x4D` | Operand layout: sshhbbfffffhf | Decode only | No executor for this profile | [tables.rs:85](../../crates/crysirror-willplus/src/tables.rs#L85) |
| `0x4E` | Operand layout: sshhbbs | Decode only | No executor for this profile | [tables.rs:86](../../crates/crysirror-willplus/src/tables.rs#L86) |
| `0x4F` | Operand layout: sshs | Decode only | No executor for this profile | [tables.rs:87](../../crates/crysirror-willplus/src/tables.rs#L87) |
| `0x50` | Operand layout: ssh | Decode only | No executor for this profile | [tables.rs:88](../../crates/crysirror-willplus/src/tables.rs#L88) |
| `0x51` | Operand layout: sshfb | Decode only | No executor for this profile | [tables.rs:89](../../crates/crysirror-willplus/src/tables.rs#L89) |
| `0x52` | Operand layout: ssfhfbs | Decode only | No executor for this profile | [tables.rs:90](../../crates/crysirror-willplus/src/tables.rs#L90) |
| `0x53` | Operand layout: ss | Decode only | No executor for this profile | [tables.rs:91](../../crates/crysirror-willplus/src/tables.rs#L91) |
| `0x54` | Operand layout: sss | Decode only | No executor for this profile | [tables.rs:92](../../crates/crysirror-willplus/src/tables.rs#L92) |
| `0x55` | Operand layout: ss | Decode only | No executor for this profile | [tables.rs:93](../../crates/crysirror-willplus/src/tables.rs#L93) |
| `0x56` | Operand layout: sbhfffffffffffbffffbhshssf | Decode only | No executor for this profile | [tables.rs:94](../../crates/crysirror-willplus/src/tables.rs#L94) |
| `0x57` | Operand layout: sh | Decode only | No executor for this profile | [tables.rs:95](../../crates/crysirror-willplus/src/tables.rs#L95) |
| `0x58` | Operand layout: ss | Decode only | No executor for this profile | [tables.rs:96](../../crates/crysirror-willplus/src/tables.rs#L96) |
| `0x59` | Operand layout: ssh | Decode only | No executor for this profile | [tables.rs:97](../../crates/crysirror-willplus/src/tables.rs#L97) |
| `0x5A` | Operand layout: sh* | Decode only | No executor for this profile | [tables.rs:98](../../crates/crysirror-willplus/src/tables.rs#L98) |
| `0x5B` | Operand layout: shb | Decode only | No executor for this profile | [tables.rs:99](../../crates/crysirror-willplus/src/tables.rs#L99) |
| `0x5C` | Operand layout: s | Decode only | No executor for this profile | [tables.rs:100](../../crates/crysirror-willplus/src/tables.rs#L100) |
| `0x5D` | Operand layout: ssb | Decode only | No executor for this profile | [tables.rs:101](../../crates/crysirror-willplus/src/tables.rs#L101) |
| `0x5E` | Operand layout: sff | Decode only | No executor for this profile | [tables.rs:102](../../crates/crysirror-willplus/src/tables.rs#L102) |
| `0x64` | Operand layout: b | Decode only | No executor for this profile | [tables.rs:103](../../crates/crysirror-willplus/src/tables.rs#L103) |
| `0x65` | Operand layout: hbffbs | Decode only | No executor for this profile | [tables.rs:104](../../crates/crysirror-willplus/src/tables.rs#L104) |
| `0x66` | Operand layout: s | Decode only | No executor for this profile | [tables.rs:105](../../crates/crysirror-willplus/src/tables.rs#L105) |
| `0x67` | Operand layout: bbhfffffb | Decode only | No executor for this profile | [tables.rs:106](../../crates/crysirror-willplus/src/tables.rs#L106) |
| `0x68` | Operand layout: b | Decode only | No executor for this profile | [tables.rs:107](../../crates/crysirror-willplus/src/tables.rs#L107) |
| `0x6E` | Operand layout: ss | Decode only | No executor for this profile | [tables.rs:108](../../crates/crysirror-willplus/src/tables.rs#L108) |
| `0x6F` | Operand layout: s | Decode only | No executor for this profile | [tables.rs:109](../../crates/crysirror-willplus/src/tables.rs#L109) |
| `0x70` | Operand layout: sh | Decode only | No executor for this profile | [tables.rs:110](../../crates/crysirror-willplus/src/tables.rs#L110) |
| `0x71` | Operand layout: none | Decode only | No executor for this profile | [tables.rs:111](../../crates/crysirror-willplus/src/tables.rs#L111) |
| `0x72` | Operand layout: shhs | Decode only | No executor for this profile | [tables.rs:112](../../crates/crysirror-willplus/src/tables.rs#L112) |
| `0x73` | Operand layout: ssh | Decode only | No executor for this profile | [tables.rs:113](../../crates/crysirror-willplus/src/tables.rs#L113) |
| `0xFA` | Operand layout: none | Decode only | No executor for this profile | [tables.rs:114](../../crates/crysirror-willplus/src/tables.rs#L114) |
| `0xFB` | Operand layout: b | Decode only | No executor for this profile | [tables.rs:115](../../crates/crysirror-willplus/src/tables.rs#L115) |
| `0xFC` | Operand layout: h | Decode only | No executor for this profile | [tables.rs:116](../../crates/crysirror-willplus/src/tables.rs#L116) |
| `0xFD` | Operand layout: none | Decode only | No executor for this profile | [tables.rs:117](../../crates/crysirror-willplus/src/tables.rs#L117) |
| `0xFE` | Operand layout: s | Decode only | No executor for this profile | [tables.rs:118](../../crates/crysirror-willplus/src/tables.rs#L118) |
| `0xFF` | Operand layout: none | Decode only | No executor for this profile | [tables.rs:119](../../crates/crysirror-willplus/src/tables.rs#L119) |

## WS2 V2 opcodes

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x00` | Operand layout: none | Decode only | No executor for this profile | [tables.rs:122](../../crates/crysirror-willplus/src/tables.rs#L122) |
| `0x01` | Operand layout: bhfaa | Decode only | No executor for this profile | [tables.rs:123](../../crates/crysirror-willplus/src/tables.rs#L123) |
| `0x02` | Operand layout: a | Decode only | No executor for this profile | [tables.rs:124](../../crates/crysirror-willplus/src/tables.rs#L124) |
| `0x04` | Operand layout: s | Decode only | No executor for this profile | [tables.rs:125](../../crates/crysirror-willplus/src/tables.rs#L125) |
| `0x05` | Operand layout: none | Decode only | No executor for this profile | [tables.rs:126](../../crates/crysirror-willplus/src/tables.rs#L126) |
| `0x06` | Operand layout: a | Decode only | No executor for this profile | [tables.rs:127](../../crates/crysirror-willplus/src/tables.rs#L127) |
| `0x07` | Operand layout: s | Decode only | No executor for this profile | [tables.rs:128](../../crates/crysirror-willplus/src/tables.rs#L128) |
| `0x08` | Operand layout: b | Decode only | No executor for this profile | [tables.rs:129](../../crates/crysirror-willplus/src/tables.rs#L129) |
| `0x09` | Operand layout: bhf | Decode only | No executor for this profile | [tables.rs:130](../../crates/crysirror-willplus/src/tables.rs#L130) |
| `0x0A` | Operand layout: hf | Decode only | No executor for this profile | [tables.rs:131](../../crates/crysirror-willplus/src/tables.rs#L131) |
| `0x0B` | Operand layout: hb | Decode only | No executor for this profile | [tables.rs:132](../../crates/crysirror-willplus/src/tables.rs#L132) |
| `0x0C` | Operand layout: hbh* | Decode only | No executor for this profile | [tables.rs:133](../../crates/crysirror-willplus/src/tables.rs#L133) |
| `0x0D` | Operand layout: hhf | Decode only | No executor for this profile | [tables.rs:134](../../crates/crysirror-willplus/src/tables.rs#L134) |
| `0x0E` | Operand layout: hhb | Decode only | No executor for this profile | [tables.rs:135](../../crates/crysirror-willplus/src/tables.rs#L135) |
| `0x0F` | Operand layout: b | Decode only | No executor for this profile | [tables.rs:136](../../crates/crysirror-willplus/src/tables.rs#L136) |
| `0x11` | Operand layout: sf | Decode only | No executor for this profile | [tables.rs:137](../../crates/crysirror-willplus/src/tables.rs#L137) |
| `0x12` | Operand layout: sbs | Decode only | No executor for this profile | [tables.rs:138](../../crates/crysirror-willplus/src/tables.rs#L138) |
| `0x13` | Operand layout: none | Decode only | No executor for this profile | [tables.rs:139](../../crates/crysirror-willplus/src/tables.rs#L139) |
| `0x14` | Operand layout: iss | Decode only | No executor for this profile | [tables.rs:140](../../crates/crysirror-willplus/src/tables.rs#L140) |
| `0x15` | Operand layout: s | Decode only | No executor for this profile | [tables.rs:141](../../crates/crysirror-willplus/src/tables.rs#L141) |
| `0x16` | Operand layout: b | Decode only | No executor for this profile | [tables.rs:142](../../crates/crysirror-willplus/src/tables.rs#L142) |
| `0x17` | Operand layout: none | Decode only | No executor for this profile | [tables.rs:143](../../crates/crysirror-willplus/src/tables.rs#L143) |
| `0x18` | Operand layout: bs | Decode only | No executor for this profile | [tables.rs:144](../../crates/crysirror-willplus/src/tables.rs#L144) |
| `0x19` | Operand layout: none | Decode only | No executor for this profile | [tables.rs:145](../../crates/crysirror-willplus/src/tables.rs#L145) |
| `0x1A` | Operand layout: s | Decode only | No executor for this profile | [tables.rs:146](../../crates/crysirror-willplus/src/tables.rs#L146) |
| `0x1B` | Operand layout: b | Decode only | No executor for this profile | [tables.rs:147](../../crates/crysirror-willplus/src/tables.rs#L147) |
| `0x1C` | Operand layout: sshb | Decode only | No executor for this profile | [tables.rs:148](../../crates/crysirror-willplus/src/tables.rs#L148) |
| `0x1D` | Operand layout: h | Decode only | No executor for this profile | [tables.rs:149](../../crates/crysirror-willplus/src/tables.rs#L149) |
| `0x1E` | Operand layout: ssffhhb | Decode only | No executor for this profile | [tables.rs:150](../../crates/crysirror-willplus/src/tables.rs#L150) |
| `0x1F` | Operand layout: sf | Decode only | No executor for this profile | [tables.rs:151](../../crates/crysirror-willplus/src/tables.rs#L151) |
| `0x20` | Operand layout: sfh | Decode only | No executor for this profile | [tables.rs:152](../../crates/crysirror-willplus/src/tables.rs#L152) |
| `0x21` | Operand layout: shhh | Decode only | No executor for this profile | [tables.rs:153](../../crates/crysirror-willplus/src/tables.rs#L153) |
| `0x22` | Operand layout: sb | Decode only | No executor for this profile | [tables.rs:154](../../crates/crysirror-willplus/src/tables.rs#L154) |
| `0x28` | Operand layout: ssffhhbhhb | Decode only | No executor for this profile | [tables.rs:155](../../crates/crysirror-willplus/src/tables.rs#L155) |
| `0x29` | Operand layout: sf | Decode only | No executor for this profile | [tables.rs:156](../../crates/crysirror-willplus/src/tables.rs#L156) |
| `0x2A` | Operand layout: sfh | Decode only | No executor for this profile | [tables.rs:157](../../crates/crysirror-willplus/src/tables.rs#L157) |
| `0x2B` | Operand layout: s | Decode only | No executor for this profile | [tables.rs:158](../../crates/crysirror-willplus/src/tables.rs#L158) |
| `0x2C` | Operand layout: s | Decode only | No executor for this profile | [tables.rs:159](../../crates/crysirror-willplus/src/tables.rs#L159) |
| `0x2D` | Operand layout: sb | Decode only | No executor for this profile | [tables.rs:160](../../crates/crysirror-willplus/src/tables.rs#L160) |
| `0x2E` | Operand layout: none | Decode only | No executor for this profile | [tables.rs:161](../../crates/crysirror-willplus/src/tables.rs#L161) |
| `0x2F` | Operand layout: shf | Decode only | No executor for this profile | [tables.rs:162](../../crates/crysirror-willplus/src/tables.rs#L162) |
| `0x32` | Operand layout: s | Decode only | No executor for this profile | [tables.rs:163](../../crates/crysirror-willplus/src/tables.rs#L163) |
| `0x33` | Operand layout: ssbb | Decode only | No executor for this profile | [tables.rs:164](../../crates/crysirror-willplus/src/tables.rs#L164) |
| `0x34` | Operand layout: ssbb | Decode only | No executor for this profile | [tables.rs:165](../../crates/crysirror-willplus/src/tables.rs#L165) |
| `0x35` | Operand layout: ssbbb | Decode only | No executor for this profile | [tables.rs:166](../../crates/crysirror-willplus/src/tables.rs#L166) |
| `0x36` | Operand layout: sfffffffbb | Decode only | No executor for this profile | [tables.rs:167](../../crates/crysirror-willplus/src/tables.rs#L167) |
| `0x37` | Operand layout: s | Decode only | No executor for this profile | [tables.rs:168](../../crates/crysirror-willplus/src/tables.rs#L168) |
| `0x38` | Operand layout: sb | Decode only | No executor for this profile | [tables.rs:169](../../crates/crysirror-willplus/src/tables.rs#L169) |
| `0x39` | Operand layout: sbbh* | Decode only | No executor for this profile | [tables.rs:170](../../crates/crysirror-willplus/src/tables.rs#L170) |
| `0x3A` | Operand layout: sbb | Decode only | No executor for this profile | [tables.rs:171](../../crates/crysirror-willplus/src/tables.rs#L171) |
| `0x3B` | Operand layout: sshhhffffffff | Decode only | No executor for this profile | [tables.rs:172](../../crates/crysirror-willplus/src/tables.rs#L172) |
| `0x3C` | Operand layout: s | Decode only | No executor for this profile | [tables.rs:173](../../crates/crysirror-willplus/src/tables.rs#L173) |
| `0x3D` | Operand layout: h | Decode only | No executor for this profile | [tables.rs:174](../../crates/crysirror-willplus/src/tables.rs#L174) |
| `0x3E` | Operand layout: none | Decode only | No executor for this profile | [tables.rs:175](../../crates/crysirror-willplus/src/tables.rs#L175) |
| `0x3F` | Operand layout: s* | Decode only | No executor for this profile | [tables.rs:176](../../crates/crysirror-willplus/src/tables.rs#L176) |
| `0x40` | Operand layout: ssb | Decode only | No executor for this profile | [tables.rs:177](../../crates/crysirror-willplus/src/tables.rs#L177) |
| `0x41` | Operand layout: sb | Decode only | No executor for this profile | [tables.rs:178](../../crates/crysirror-willplus/src/tables.rs#L178) |
| `0x42` | Operand layout: sh | Decode only | No executor for this profile | [tables.rs:179](../../crates/crysirror-willplus/src/tables.rs#L179) |
| `0x43` | Operand layout: s | Decode only | No executor for this profile | [tables.rs:180](../../crates/crysirror-willplus/src/tables.rs#L180) |
| `0x44` | Operand layout: ssb | Decode only | No executor for this profile | [tables.rs:181](../../crates/crysirror-willplus/src/tables.rs#L181) |
| `0x45` | Operand layout: shffff | Decode only | No executor for this profile | [tables.rs:182](../../crates/crysirror-willplus/src/tables.rs#L182) |
| `0x46` | Operand layout: shbffff | Decode only | No executor for this profile | [tables.rs:183](../../crates/crysirror-willplus/src/tables.rs#L183) |
| `0x47` | Operand layout: sshbbfffffhf | Decode only | No executor for this profile | [tables.rs:184](../../crates/crysirror-willplus/src/tables.rs#L184) |
| `0x48` | Operand layout: sshbbs | Decode only | No executor for this profile | [tables.rs:185](../../crates/crysirror-willplus/src/tables.rs#L185) |
| `0x49` | Operand layout: sss | Decode only | No executor for this profile | [tables.rs:186](../../crates/crysirror-willplus/src/tables.rs#L186) |
| `0x4A` | Operand layout: ss | Decode only | No executor for this profile | [tables.rs:187](../../crates/crysirror-willplus/src/tables.rs#L187) |
| `0x4B` | Operand layout: shhffff | Decode only | No executor for this profile | [tables.rs:188](../../crates/crysirror-willplus/src/tables.rs#L188) |
| `0x4C` | Operand layout: shhbffff | Decode only | No executor for this profile | [tables.rs:189](../../crates/crysirror-willplus/src/tables.rs#L189) |
| `0x4D` | Operand layout: sshhbbfffffhf | Decode only | No executor for this profile | [tables.rs:190](../../crates/crysirror-willplus/src/tables.rs#L190) |
| `0x4E` | Operand layout: sshhbbs | Decode only | No executor for this profile | [tables.rs:191](../../crates/crysirror-willplus/src/tables.rs#L191) |
| `0x4F` | Operand layout: sshs | Decode only | No executor for this profile | [tables.rs:192](../../crates/crysirror-willplus/src/tables.rs#L192) |
| `0x50` | Operand layout: ssh | Decode only | No executor for this profile | [tables.rs:193](../../crates/crysirror-willplus/src/tables.rs#L193) |
| `0x51` | Operand layout: sshfb | Decode only | No executor for this profile | [tables.rs:194](../../crates/crysirror-willplus/src/tables.rs#L194) |
| `0x52` | Operand layout: ssfhfbs | Decode only | No executor for this profile | [tables.rs:195](../../crates/crysirror-willplus/src/tables.rs#L195) |
| `0x53` | Operand layout: ss | Decode only | No executor for this profile | [tables.rs:196](../../crates/crysirror-willplus/src/tables.rs#L196) |
| `0x54` | Operand layout: sss | Decode only | No executor for this profile | [tables.rs:197](../../crates/crysirror-willplus/src/tables.rs#L197) |
| `0x55` | Operand layout: ss | Decode only | No executor for this profile | [tables.rs:198](../../crates/crysirror-willplus/src/tables.rs#L198) |
| `0x56` | Operand layout: sbhfffffffffffbffffbhshssf | Decode only | No executor for this profile | [tables.rs:199](../../crates/crysirror-willplus/src/tables.rs#L199) |
| `0x57` | Operand layout: sh | Decode only | No executor for this profile | [tables.rs:200](../../crates/crysirror-willplus/src/tables.rs#L200) |
| `0x58` | Operand layout: ss | Decode only | No executor for this profile | [tables.rs:201](../../crates/crysirror-willplus/src/tables.rs#L201) |
| `0x59` | Operand layout: ssh | Decode only | No executor for this profile | [tables.rs:202](../../crates/crysirror-willplus/src/tables.rs#L202) |
| `0x5A` | Operand layout: sh* | Decode only | No executor for this profile | [tables.rs:203](../../crates/crysirror-willplus/src/tables.rs#L203) |
| `0x5B` | Operand layout: shb | Decode only | No executor for this profile | [tables.rs:204](../../crates/crysirror-willplus/src/tables.rs#L204) |
| `0x5C` | Operand layout: s | Decode only | No executor for this profile | [tables.rs:205](../../crates/crysirror-willplus/src/tables.rs#L205) |
| `0x5D` | Operand layout: ssb | Decode only | No executor for this profile | [tables.rs:206](../../crates/crysirror-willplus/src/tables.rs#L206) |
| `0x5E` | Operand layout: sff | Decode only | No executor for this profile | [tables.rs:207](../../crates/crysirror-willplus/src/tables.rs#L207) |
| `0x5F` | Operand layout: s | Decode only | No executor for this profile | [tables.rs:208](../../crates/crysirror-willplus/src/tables.rs#L208) |
| `0x60` | Operand layout: hhhh | Decode only | No executor for this profile | [tables.rs:209](../../crates/crysirror-willplus/src/tables.rs#L209) |
| `0x61` | Operand layout: bffff | Decode only | No executor for this profile | [tables.rs:210](../../crates/crysirror-willplus/src/tables.rs#L210) |
| `0x62` | Operand layout: s | Decode only | No executor for this profile | [tables.rs:211](../../crates/crysirror-willplus/src/tables.rs#L211) |
| `0x63` | Operand layout: sb | Decode only | No executor for this profile | [tables.rs:212](../../crates/crysirror-willplus/src/tables.rs#L212) |
| `0x64` | Operand layout: b | Decode only | No executor for this profile | [tables.rs:213](../../crates/crysirror-willplus/src/tables.rs#L213) |
| `0x65` | Operand layout: hbffbs | Decode only | No executor for this profile | [tables.rs:214](../../crates/crysirror-willplus/src/tables.rs#L214) |
| `0x66` | Operand layout: s | Decode only | No executor for this profile | [tables.rs:215](../../crates/crysirror-willplus/src/tables.rs#L215) |
| `0x67` | Operand layout: bbhfffffb | Decode only | No executor for this profile | [tables.rs:216](../../crates/crysirror-willplus/src/tables.rs#L216) |
| `0x68` | Operand layout: b | Decode only | No executor for this profile | [tables.rs:217](../../crates/crysirror-willplus/src/tables.rs#L217) |
| `0x69` | Operand layout: sbbfffffhf | Decode only | No executor for this profile | [tables.rs:218](../../crates/crysirror-willplus/src/tables.rs#L218) |
| `0x6A` | Operand layout: shbbs | Decode only | No executor for this profile | [tables.rs:219](../../crates/crysirror-willplus/src/tables.rs#L219) |
| `0x6E` | Operand layout: ss | Decode only | No executor for this profile | [tables.rs:220](../../crates/crysirror-willplus/src/tables.rs#L220) |
| `0x6F` | Operand layout: s | Decode only | No executor for this profile | [tables.rs:221](../../crates/crysirror-willplus/src/tables.rs#L221) |
| `0x70` | Operand layout: sh | Decode only | No executor for this profile | [tables.rs:222](../../crates/crysirror-willplus/src/tables.rs#L222) |
| `0x71` | Operand layout: none | Decode only | No executor for this profile | [tables.rs:223](../../crates/crysirror-willplus/src/tables.rs#L223) |
| `0x72` | Operand layout: shhs | Decode only | No executor for this profile | [tables.rs:224](../../crates/crysirror-willplus/src/tables.rs#L224) |
| `0x73` | Operand layout: ssh | Decode only | No executor for this profile | [tables.rs:225](../../crates/crysirror-willplus/src/tables.rs#L225) |
| `0x74` | Operand layout: ss | Decode only | No executor for this profile | [tables.rs:226](../../crates/crysirror-willplus/src/tables.rs#L226) |
| `0x75` | Operand layout: ss | Decode only | No executor for this profile | [tables.rs:227](../../crates/crysirror-willplus/src/tables.rs#L227) |
| `0x78` | Operand layout: ssbb | Decode only | No executor for this profile | [tables.rs:228](../../crates/crysirror-willplus/src/tables.rs#L228) |
| `0x79` | Operand layout: ssf | Decode only | No executor for this profile | [tables.rs:229](../../crates/crysirror-willplus/src/tables.rs#L229) |
| `0x7A` | Operand layout: ssfbbs | Decode only | No executor for this profile | [tables.rs:230](../../crates/crysirror-willplus/src/tables.rs#L230) |
| `0x7B` | Operand layout: ss | Decode only | No executor for this profile | [tables.rs:231](../../crates/crysirror-willplus/src/tables.rs#L231) |
| `0x7C` | Operand layout: ssf | Decode only | No executor for this profile | [tables.rs:232](../../crates/crysirror-willplus/src/tables.rs#L232) |
| `0x7D` | Operand layout: sf | Decode only | No executor for this profile | [tables.rs:233](../../crates/crysirror-willplus/src/tables.rs#L233) |
| `0x7E` | Operand layout: s | Decode only | No executor for this profile | [tables.rs:234](../../crates/crysirror-willplus/src/tables.rs#L234) |
| `0xC8` | Operand layout: none | Decode only | No executor for this profile | [tables.rs:235](../../crates/crysirror-willplus/src/tables.rs#L235) |
| `0xC9` | Operand layout: sshhh | Decode only | No executor for this profile | [tables.rs:236](../../crates/crysirror-willplus/src/tables.rs#L236) |
| `0xCA` | Operand layout: ss | Decode only | No executor for this profile | [tables.rs:237](../../crates/crysirror-willplus/src/tables.rs#L237) |
| `0xCB` | Operand layout: sbb | Decode only | No executor for this profile | [tables.rs:238](../../crates/crysirror-willplus/src/tables.rs#L238) |
| `0xCC` | Operand layout: none | Decode only | No executor for this profile | [tables.rs:239](../../crates/crysirror-willplus/src/tables.rs#L239) |
| `0xCD` | Operand layout: sssssfb | Decode only | No executor for this profile | [tables.rs:240](../../crates/crysirror-willplus/src/tables.rs#L240) |
| `0xCE` | Operand layout: b | Decode only | No executor for this profile | [tables.rs:241](../../crates/crysirror-willplus/src/tables.rs#L241) |
| `0xCF` | Operand layout: ssf | Decode only | No executor for this profile | [tables.rs:242](../../crates/crysirror-willplus/src/tables.rs#L242) |
| `0xD0` | Operand layout: sh | Decode only | No executor for this profile | [tables.rs:243](../../crates/crysirror-willplus/src/tables.rs#L243) |
| `0xD1` | Operand layout: sh | Decode only | No executor for this profile | [tables.rs:244](../../crates/crysirror-willplus/src/tables.rs#L244) |
| `0xD2` | Operand layout: s | Decode only | No executor for this profile | [tables.rs:245](../../crates/crysirror-willplus/src/tables.rs#L245) |
| `0xD3` | Operand layout: s | Decode only | No executor for this profile | [tables.rs:246](../../crates/crysirror-willplus/src/tables.rs#L246) |
| `0xD4` | Operand layout: shh | Decode only | No executor for this profile | [tables.rs:247](../../crates/crysirror-willplus/src/tables.rs#L247) |
| `0xF8` | Operand layout: none | Decode only | No executor for this profile | [tables.rs:248](../../crates/crysirror-willplus/src/tables.rs#L248) |
| `0xF9` | Operand layout: bs | Decode only | No executor for this profile | [tables.rs:249](../../crates/crysirror-willplus/src/tables.rs#L249) |
| `0xFA` | Operand layout: none | Decode only | No executor for this profile | [tables.rs:250](../../crates/crysirror-willplus/src/tables.rs#L250) |
| `0xFB` | Operand layout: b | Decode only | No executor for this profile | [tables.rs:251](../../crates/crysirror-willplus/src/tables.rs#L251) |
| `0xFC` | Operand layout: h | Decode only | No executor for this profile | [tables.rs:252](../../crates/crysirror-willplus/src/tables.rs#L252) |
| `0xFD` | Operand layout: none | Decode only | No executor for this profile | [tables.rs:253](../../crates/crysirror-willplus/src/tables.rs#L253) |
| `0xFE` | Operand layout: s | Decode only | No executor for this profile | [tables.rs:254](../../crates/crysirror-willplus/src/tables.rs#L254) |
| `0xFF` | Operand layout: none | Decode only | No executor for this profile | [tables.rs:255](../../crates/crysirror-willplus/src/tables.rs#L255) |

## WS2 V3 opcodes

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x00` | Execute execute branch | Empty handler | Original no-op status not established | [vm.rs:1111](../../crates/crysirror-willplus/src/vm.rs#L1111) |
| `0x01` | Conditional branch | Partial | AdvHD 0x01 comparison operator {other} is not connected | [vm.rs:1112](../../crates/crysirror-willplus/src/vm.rs#L1112) |
| `0x02` | Jump | Handled | — | [vm.rs:1135](../../crates/crysirror-willplus/src/vm.rs#L1135) |
| `0x04` | Call a script | Handled | — | [vm.rs:1136](../../crates/crysirror-willplus/src/vm.rs#L1136) |
| `0x05` | Return | Handled | — | [vm.rs:1143](../../crates/crysirror-willplus/src/vm.rs#L1143) |
| `0x06` | Jump | Handled | — | [vm.rs:1135](../../crates/crysirror-willplus/src/vm.rs#L1135) |
| `0x07` | Transfer to another script | Handled | — | [vm.rs:1148](../../crates/crysirror-willplus/src/vm.rs#L1148) |
| `0x08` | 0x08: title EXIT | Handled | — | [vm.rs:1282](../../crates/crysirror-willplus/src/vm.rs#L1282) |
| `0x09` | Variable arithmetic/assignment | Partial | AdvHD 0x09 operator {other} is not connected | [vm.rs:1161](../../crates/crysirror-willplus/src/vm.rs#L1161) |
| `0x0A` | Operand layout: hf | Decode only | No extracted executor route | [tables.rs:267](../../crates/crysirror-willplus/src/tables.rs#L267) |
| `0x0B` | Set a flag | Handled | — | [vm.rs:1176](../../crates/crysirror-willplus/src/vm.rs#L1176) |
| `0x0C` | Operand layout: hbh* | Decode only | No extracted executor route | [tables.rs:269](../../crates/crysirror-willplus/src/tables.rs#L269) |
| `0x0D` | Fill variable range | Handled | — | [vm.rs:1184](../../crates/crysirror-willplus/src/vm.rs#L1184) |
| `0x0E` | Fill flag range | Handled | — | [vm.rs:1190](../../crates/crysirror-willplus/src/vm.rs#L1190) |
| `0x0F` | Present choices | Handled | — | [vm.rs:1225](../../crates/crysirror-willplus/src/vm.rs#L1225) |
| `0x11` | Start named timer | Handled | — | [vm.rs:1201](../../crates/crysirror-willplus/src/vm.rs#L1201) |
| `0x12` | Wait for named timer | Handled | — | [vm.rs:1206](../../crates/crysirror-willplus/src/vm.rs#L1206) |
| `0x13` | Operand layout: none | Decode only | No extracted executor route | [tables.rs:275](../../crates/crysirror-willplus/src/tables.rs#L275) |
| `0x14` | Display message | Handled | — | [vm.rs:1215](../../crates/crysirror-willplus/src/vm.rs#L1215) |
| `0x15` | Set speaker | Handled | — | [vm.rs:1223](../../crates/crysirror-willplus/src/vm.rs#L1223) |
| `0x16` | Close message window | Handled | — | [vm.rs:1224](../../crates/crysirror-willplus/src/vm.rs#L1224) |
| `0x17` | 0x17 / 0x71 close the title/menu reset sequences (after 0x68 b1): clear running tweens and pending queues / screen effe… | Handled | — | [vm.rs:1289](../../crates/crysirror-willplus/src/vm.rs#L1289) |
| `0x18` | Operand layout: bs | Decode only | No extracted executor route | [tables.rs:280](../../crates/crysirror-willplus/src/tables.rs#L280) |
| `0x19` | Operand layout: none | Decode only | No extracted executor route | [tables.rs:281](../../crates/crysirror-willplus/src/tables.rs#L281) |
| `0x1A` | Call Lua | Handled | — | [vm.rs:1321](../../crates/crysirror-willplus/src/vm.rs#L1321) |
| `0x1B` | Open system screen | Partial | AdvHD 0x1B system screen {other} is not connected | [vm.rs:1333](../../crates/crysirror-willplus/src/vm.rs#L1333) |
| `0x1C` | Call Lua | Handled | — | [vm.rs:1346](../../crates/crysirror-willplus/src/vm.rs#L1346) |
| `0x1D` | Execute execute branch | Empty handler | Original no-op status not established | [vm.rs:1271](../../crates/crysirror-willplus/src/vm.rs#L1271) |
| `0x1E` | Play BGM | Handled | — | [vm.rs:1229](../../crates/crysirror-willplus/src/vm.rs#L1229) |
| `0x1F` | Stop BGM | Handled | — | [vm.rs:1235](../../crates/crysirror-willplus/src/vm.rs#L1235) |
| `0x20` | Set volume | Handled | — | [vm.rs:1259](../../crates/crysirror-willplus/src/vm.rs#L1259) |
| `0x21` | Operand layout: shhh | Decode only | No extracted executor route | [tables.rs:289](../../crates/crysirror-willplus/src/tables.rs#L289) |
| `0x22` | Operand layout: sb | Decode only | No extracted executor route | [tables.rs:290](../../crates/crysirror-willplus/src/tables.rs#L290) |
| `0x28` | Play sound/voice | Handled | — | [vm.rs:1248](../../crates/crysirror-willplus/src/vm.rs#L1248) |
| `0x29` | Stop sound | Handled | — | [vm.rs:1235](../../crates/crysirror-willplus/src/vm.rs#L1235) |
| `0x2A` | Set volume | Handled | — | [vm.rs:1259](../../crates/crysirror-willplus/src/vm.rs#L1259) |
| `0x2B` | Operand layout: s | Decode only | No extracted executor route | [tables.rs:294](../../crates/crysirror-willplus/src/tables.rs#L294) |
| `0x2C` | Operand layout: s | Decode only | No extracted executor route | [tables.rs:295](../../crates/crysirror-willplus/src/tables.rs#L295) |
| `0x2D` | Operand layout: sb | Decode only | No extracted executor route | [tables.rs:296](../../crates/crysirror-willplus/src/tables.rs#L296) |
| `0x2E` | Execute execute branch | Empty handler | Original no-op status not established | [vm.rs:1257](../../crates/crysirror-willplus/src/vm.rs#L1257) |
| `0x2F` | Operand layout: shf | Decode only | No extracted executor route | [tables.rs:298](../../crates/crysirror-willplus/src/tables.rs#L298) |
| `0x32` | Set face graphic | Handled | — | [vm.rs:1510](../../crates/crysirror-willplus/src/vm.rs#L1510) |
| `0x33` | Load layer image | Handled | — | [vm.rs:1359](../../crates/crysirror-willplus/src/vm.rs#L1359) |
| `0x34` | Load PNA | Handled | — | [vm.rs:1365](../../crates/crysirror-willplus/src/vm.rs#L1365) |
| `0x35` | Movie control | Handled | — | [vm.rs:1299](../../crates/crysirror-willplus/src/vm.rs#L1299) |
| `0x36` | Operand layout: sfffffffbb | Decode only | No extracted executor route | [tables.rs:303](../../crates/crysirror-willplus/src/tables.rs#L303) |
| `0x37` | Erase layers | Handled | — | [vm.rs:1371](../../crates/crysirror-willplus/src/vm.rs#L1371) |
| `0x38` | Operand layout: sb | Decode only | No extracted executor route | [tables.rs:305](../../crates/crysirror-willplus/src/tables.rs#L305) |
| `0x39` | Select PNA parts | Handled | — | [vm.rs:1382](../../crates/crysirror-willplus/src/vm.rs#L1382) |
| `0x3A` | Movie control | Handled | — | [vm.rs:1312](../../crates/crysirror-willplus/src/vm.rs#L1312) |
| `0x3B` | Operand layout: sshhhffffffff | Decode only | No extracted executor route | [tables.rs:308](../../crates/crysirror-willplus/src/tables.rs#L308) |
| `0x3C` | Operand layout: s | Decode only | No extracted executor route | [tables.rs:309](../../crates/crysirror-willplus/src/tables.rs#L309) |
| `0x3D` | Operand layout: h | Decode only | No extracted executor route | [tables.rs:310](../../crates/crysirror-willplus/src/tables.rs#L310) |
| `0x3E` | Operand layout: none | Decode only | No extracted executor route | [tables.rs:311](../../crates/crysirror-willplus/src/tables.rs#L311) |
| `0x3F` | Set stacking order | Handled | — | [vm.rs:1393](../../crates/crysirror-willplus/src/vm.rs#L1393) |
| `0x40` | Set layer mask | Handled | — | [vm.rs:1498](../../crates/crysirror-willplus/src/vm.rs#L1498) |
| `0x41` | Operand layout: sb | Decode only | No extracted executor route | [tables.rs:314](../../crates/crysirror-willplus/src/tables.rs#L314) |
| `0x42` | Record layer attributes | Partial | Attributes retained; visible semantics not connected | [vm.rs:1505](../../crates/crysirror-willplus/src/vm.rs#L1505) |
| `0x43` | Execute execute branch | Empty handler | Original no-op status not established | [vm.rs:1271](../../crates/crysirror-willplus/src/vm.rs#L1271) |
| `0x44` | Operand layout: ssb | Decode only | No extracted executor route | [tables.rs:317](../../crates/crysirror-willplus/src/tables.rs#L317) |
| `0x45` | Read layer property group | Handled | — | [vm.rs:1398](../../crates/crysirror-willplus/src/vm.rs#L1398) |
| `0x46` | Set layer property group | Partial | Groups 10/11 retained without visible semantics | [vm.rs:1412](../../crates/crysirror-willplus/src/vm.rs#L1412) |
| `0x47` | Queue layer tween | Handled | — | [vm.rs:1427](../../crates/crysirror-willplus/src/vm.rs#L1427) |
| `0x48` | Start tween queue | Handled | — | [vm.rs:1439](../../crates/crysirror-willplus/src/vm.rs#L1439) |
| `0x49` | Operand layout: sss | Decode only | No extracted executor route | [tables.rs:322](../../crates/crysirror-willplus/src/tables.rs#L322) |
| `0x4A` | Operand layout: ss | Decode only | No extracted executor route | [tables.rs:323](../../crates/crysirror-willplus/src/tables.rs#L323) |
| `0x4B` | Operand layout: shhffff | Decode only | No extracted executor route | [tables.rs:324](../../crates/crysirror-willplus/src/tables.rs#L324) |
| `0x4C` | Operand layout: shhbffff | Decode only | No extracted executor route | [tables.rs:325](../../crates/crysirror-willplus/src/tables.rs#L325) |
| `0x4D` | Operand layout: sshhbbfffffhf | Decode only | No extracted executor route | [tables.rs:326](../../crates/crysirror-willplus/src/tables.rs#L326) |
| `0x4E` | Operand layout: sshhbbs | Decode only | No extracted executor route | [tables.rs:327](../../crates/crysirror-willplus/src/tables.rs#L327) |
| `0x4F` | Operand layout: sshs | Decode only | No extracted executor route | [tables.rs:328](../../crates/crysirror-willplus/src/tables.rs#L328) |
| `0x50` | Operand layout: ssh | Decode only | No extracted executor route | [tables.rs:329](../../crates/crysirror-willplus/src/tables.rs#L329) |
| `0x51` | Operand layout: sshfb | Decode only | No extracted executor route | [tables.rs:330](../../crates/crysirror-willplus/src/tables.rs#L330) |
| `0x52` | Operand layout: ssfhfbs | Decode only | No extracted executor route | [tables.rs:331](../../crates/crysirror-willplus/src/tables.rs#L331) |
| `0x53` | Operand layout: ss | Decode only | No extracted executor route | [tables.rs:332](../../crates/crysirror-willplus/src/tables.rs#L332) |
| `0x54` | Operand layout: sss | Decode only | No extracted executor route | [tables.rs:333](../../crates/crysirror-willplus/src/tables.rs#L333) |
| `0x55` | Operand layout: ss | Decode only | No extracted executor route | [tables.rs:334](../../crates/crysirror-willplus/src/tables.rs#L334) |
| `0x56` | Streak weather | Partial | Weather semantics inferred from script usage | [vm.rs:1472](../../crates/crysirror-willplus/src/vm.rs#L1472) |
| `0x57` | Wait for layer tweens | Handled | — | [vm.rs:1454](../../crates/crysirror-willplus/src/vm.rs#L1454) |
| `0x58` | Layer name | Handled | — | [vm.rs:1463](../../crates/crysirror-willplus/src/vm.rs#L1463) |
| `0x59` | Operand layout: ssh | Decode only | No extracted executor route | [tables.rs:338](../../crates/crysirror-willplus/src/tables.rs#L338) |
| `0x5A` | Operand layout: sh* | Decode only | No extracted executor route | [tables.rs:339](../../crates/crysirror-willplus/src/tables.rs#L339) |
| `0x5B` | Operand layout: shb | Decode only | No extracted executor route | [tables.rs:340](../../crates/crysirror-willplus/src/tables.rs#L340) |
| `0x5C` | Streak weather | Partial | Weather semantics inferred from script usage | [vm.rs:1481](../../crates/crysirror-willplus/src/vm.rs#L1481) |
| `0x5D` | Operand layout: ssb | Decode only | No extracted executor route | [tables.rs:342](../../crates/crysirror-willplus/src/tables.rs#L342) |
| `0x5E` | Operand layout: sff | Decode only | No extracted executor route | [tables.rs:343](../../crates/crysirror-willplus/src/tables.rs#L343) |
| `0x5F` | Operand layout: s | Decode only | No extracted executor route | [tables.rs:344](../../crates/crysirror-willplus/src/tables.rs#L344) |
| `0x60` | Operand layout: hhhh | Decode only | No extracted executor route | [tables.rs:345](../../crates/crysirror-willplus/src/tables.rs#L345) |
| `0x61` | Operand layout: bffff | Decode only | No extracted executor route | [tables.rs:346](../../crates/crysirror-willplus/src/tables.rs#L346) |
| `0x62` | Operand layout: s | Decode only | No extracted executor route | [tables.rs:347](../../crates/crysirror-willplus/src/tables.rs#L347) |
| `0x63` | Operand layout: sb | Decode only | No extracted executor route | [tables.rs:348](../../crates/crysirror-willplus/src/tables.rs#L348) |
| `0x64` | Prepare screen transition | Handled | — | [vm.rs:1527](../../crates/crysirror-willplus/src/vm.rs#L1527) |
| `0x65` | Run screen transition | Handled | — | [vm.rs:1532](../../crates/crysirror-willplus/src/vm.rs#L1532) |
| `0x66` | Set transition rule | Handled | — | [vm.rs:1531](../../crates/crysirror-willplus/src/vm.rs#L1531) |
| `0x67` | Update state / quake | Handled | — | [vm.rs:1518](../../crates/crysirror-willplus/src/vm.rs#L1518) |
| `0x68` | Update state / quake, dirty | Handled | — | [vm.rs:1523](../../crates/crysirror-willplus/src/vm.rs#L1523) |
| `0x69` | Operand layout: sbbfffffhf | Decode only | No extracted executor route | [tables.rs:354](../../crates/crysirror-willplus/src/tables.rs#L354) |
| `0x6A` | Operand layout: shbbs | Decode only | No extracted executor route | [tables.rs:355](../../crates/crysirror-willplus/src/tables.rs#L355) |
| `0x6B` | Operand layout: ss | Decode only | No extracted executor route | [tables.rs:356](../../crates/crysirror-willplus/src/tables.rs#L356) |
| `0x6C` | Operand layout: sff | Decode only | No extracted executor route | [tables.rs:357](../../crates/crysirror-willplus/src/tables.rs#L357) |
| `0x6E` | Execute execute branch | Handled | — | [vm.rs:1486](../../crates/crysirror-willplus/src/vm.rs#L1486) |
| `0x6F` | Execute execute branch | Handled | — | [vm.rs:1495](../../crates/crysirror-willplus/src/vm.rs#L1495) |
| `0x70` | Operand layout: sh | Decode only | No extracted executor route | [tables.rs:360](../../crates/crysirror-willplus/src/tables.rs#L360) |
| `0x71` | Update state / quake, state / rule, dirty | Handled | — | [vm.rs:1294](../../crates/crysirror-willplus/src/vm.rs#L1294) |
| `0x72` | Operand layout: shhs | Decode only | No extracted executor route | [tables.rs:362](../../crates/crysirror-willplus/src/tables.rs#L362) |
| `0x73` | Operand layout: ssh | Decode only | No extracted executor route | [tables.rs:363](../../crates/crysirror-willplus/src/tables.rs#L363) |
| `0x74` | Operand layout: ss | Decode only | No extracted executor route | [tables.rs:364](../../crates/crysirror-willplus/src/tables.rs#L364) |
| `0x75` | Operand layout: ss | Decode only | No extracted executor route | [tables.rs:365](../../crates/crysirror-willplus/src/tables.rs#L365) |
| `0x78` | Operand layout: ssbbb | Decode only | No extracted executor route | [tables.rs:366](../../crates/crysirror-willplus/src/tables.rs#L366) |
| `0x79` | Operand layout: ssf | Decode only | No extracted executor route | [tables.rs:367](../../crates/crysirror-willplus/src/tables.rs#L367) |
| `0x7A` | Operand layout: ssfbbs | Decode only | No extracted executor route | [tables.rs:368](../../crates/crysirror-willplus/src/tables.rs#L368) |
| `0x7B` | Operand layout: ss | Decode only | No extracted executor route | [tables.rs:369](../../crates/crysirror-willplus/src/tables.rs#L369) |
| `0x7C` | Operand layout: ssf | Decode only | No extracted executor route | [tables.rs:370](../../crates/crysirror-willplus/src/tables.rs#L370) |
| `0x7D` | Operand layout: sf | Decode only | No extracted executor route | [tables.rs:371](../../crates/crysirror-willplus/src/tables.rs#L371) |
| `0x7E` | Operand layout: s | Decode only | No extracted executor route | [tables.rs:372](../../crates/crysirror-willplus/src/tables.rs#L372) |
| `0x7F` | Operand layout: sfffff | Decode only | No extracted executor route | [tables.rs:373](../../crates/crysirror-willplus/src/tables.rs#L373) |
| `0x80` | Operand layout: s | Decode only | No extracted executor route | [tables.rs:374](../../crates/crysirror-willplus/src/tables.rs#L374) |
| `0x81` | Operand layout: sbsffb | Decode only | No extracted executor route | [tables.rs:375](../../crates/crysirror-willplus/src/tables.rs#L375) |
| `0x82` | Operand layout: ssf | Decode only | No extracted executor route | [tables.rs:376](../../crates/crysirror-willplus/src/tables.rs#L376) |
| `0x83` | Operand layout: ssff | Decode only | No extracted executor route | [tables.rs:377](../../crates/crysirror-willplus/src/tables.rs#L377) |
| `0x84` | Operand layout: sssfhf | Decode only | No extracted executor route | [tables.rs:378](../../crates/crysirror-willplus/src/tables.rs#L378) |
| `0x85` | Operand layout: ssbf | Decode only | No extracted executor route | [tables.rs:379](../../crates/crysirror-willplus/src/tables.rs#L379) |
| `0x86` | Operand layout: sfff | Decode only | No extracted executor route | [tables.rs:380](../../crates/crysirror-willplus/src/tables.rs#L380) |
| `0x87` | Operand layout: sf | Decode only | No extracted executor route | [tables.rs:381](../../crates/crysirror-willplus/src/tables.rs#L381) |
| `0x88` | Operand layout: sssfhf | Decode only | No extracted executor route | [tables.rs:382](../../crates/crysirror-willplus/src/tables.rs#L382) |
| `0x8C` | Operand layout: sssbb | Decode only | No extracted executor route | [tables.rs:383](../../crates/crysirror-willplus/src/tables.rs#L383) |
| `0x8D` | Operand layout: issbbs | Decode only | No extracted executor route | [tables.rs:384](../../crates/crysirror-willplus/src/tables.rs#L384) |
| `0x8E` | Operand layout: issbbs | Decode only | No extracted executor route | [tables.rs:385](../../crates/crysirror-willplus/src/tables.rs#L385) |
| `0x8F` | Operand layout: ss | Decode only | No extracted executor route | [tables.rs:386](../../crates/crysirror-willplus/src/tables.rs#L386) |
| `0x90` | Operand layout: s | Decode only | No extracted executor route | [tables.rs:387](../../crates/crysirror-willplus/src/tables.rs#L387) |
| `0x96` | Operand layout: hffff | Decode only | No extracted executor route | [tables.rs:388](../../crates/crysirror-willplus/src/tables.rs#L388) |
| `0x97` | Operand layout: hbffff | Decode only | No extracted executor route | [tables.rs:389](../../crates/crysirror-willplus/src/tables.rs#L389) |
| `0x98` | Operand layout: shbbfffffhf | Decode only | No extracted executor route | [tables.rs:390](../../crates/crysirror-willplus/src/tables.rs#L390) |
| `0x99` | Operand layout: shbbs | Decode only | No extracted executor route | [tables.rs:391](../../crates/crysirror-willplus/src/tables.rs#L391) |
| `0x9A` | Operand layout: none | Decode only | No extracted executor route | [tables.rs:392](../../crates/crysirror-willplus/src/tables.rs#L392) |
| `0x9B` | Operand layout: s | Decode only | No extracted executor route | [tables.rs:393](../../crates/crysirror-willplus/src/tables.rs#L393) |
| `0x9C` | Operand layout: ss | Decode only | No extracted executor route | [tables.rs:394](../../crates/crysirror-willplus/src/tables.rs#L394) |
| `0x9D` | Operand layout: s | Decode only | No extracted executor route | [tables.rs:395](../../crates/crysirror-willplus/src/tables.rs#L395) |
| `0x9E` | Operand layout: sb | Decode only | No extracted executor route | [tables.rs:396](../../crates/crysirror-willplus/src/tables.rs#L396) |
| `0x9F` | Operand layout: sb | Decode only | No extracted executor route | [tables.rs:397](../../crates/crysirror-willplus/src/tables.rs#L397) |
| `0xC8` | Operand layout: none | Decode only | No extracted executor route | [tables.rs:398](../../crates/crysirror-willplus/src/tables.rs#L398) |
| `0xC9` | Operand layout: sshhhh | Decode only | No extracted executor route | [tables.rs:399](../../crates/crysirror-willplus/src/tables.rs#L399) |
| `0xCA` | Operand layout: ss | Decode only | No extracted executor route | [tables.rs:400](../../crates/crysirror-willplus/src/tables.rs#L400) |
| `0xCB` | Operand layout: sbb | Decode only | No extracted executor route | [tables.rs:401](../../crates/crysirror-willplus/src/tables.rs#L401) |
| `0xCC` | Operand layout: none | Decode only | No extracted executor route | [tables.rs:402](../../crates/crysirror-willplus/src/tables.rs#L402) |
| `0xCD` | Operand layout: sssssfb | Decode only | No extracted executor route | [tables.rs:403](../../crates/crysirror-willplus/src/tables.rs#L403) |
| `0xCE` | Operand layout: b | Decode only | No extracted executor route | [tables.rs:404](../../crates/crysirror-willplus/src/tables.rs#L404) |
| `0xCF` | Operand layout: ssf | Decode only | No extracted executor route | [tables.rs:405](../../crates/crysirror-willplus/src/tables.rs#L405) |
| `0xD0` | Operand layout: sh | Decode only | No extracted executor route | [tables.rs:406](../../crates/crysirror-willplus/src/tables.rs#L406) |
| `0xD1` | Operand layout: sh | Decode only | No extracted executor route | [tables.rs:407](../../crates/crysirror-willplus/src/tables.rs#L407) |
| `0xD2` | Operand layout: s | Decode only | No extracted executor route | [tables.rs:408](../../crates/crysirror-willplus/src/tables.rs#L408) |
| `0xD3` | Operand layout: s | Decode only | No extracted executor route | [tables.rs:409](../../crates/crysirror-willplus/src/tables.rs#L409) |
| `0xD4` | Operand layout: shh | Decode only | No extracted executor route | [tables.rs:410](../../crates/crysirror-willplus/src/tables.rs#L410) |
| `0xE6` | Execute execute branch | Empty handler | Original no-op status not established | [vm.rs:1271](../../crates/crysirror-willplus/src/vm.rs#L1271) |
| `0xE7` | Operand layout: none | Decode only | No extracted executor route | [tables.rs:412](../../crates/crysirror-willplus/src/tables.rs#L412) |
| `0xE8` | Operand layout: none | Decode only | No extracted executor route | [tables.rs:413](../../crates/crysirror-willplus/src/tables.rs#L413) |
| `0xF0` | Operand layout: b | Decode only | No extracted executor route | [tables.rs:414](../../crates/crysirror-willplus/src/tables.rs#L414) |
| `0xF8` | Operand layout: none | Decode only | No extracted executor route | [tables.rs:415](../../crates/crysirror-willplus/src/tables.rs#L415) |
| `0xF9` | Operand layout: bs | Decode only | No extracted executor route | [tables.rs:416](../../crates/crysirror-willplus/src/tables.rs#L416) |
| `0xFA` | Operand layout: none | Decode only | No extracted executor route | [tables.rs:417](../../crates/crysirror-willplus/src/tables.rs#L417) |
| `0xFB` | 0xFB b1/b0 brackets the brand logos in start.ws2 and 0xFD precedes new-game/replay fades; neither has a visible effect … | Empty handler | Original no-op status not established | [vm.rs:1274](../../crates/crysirror-willplus/src/vm.rs#L1274) |
| `0xFC` | 0xFC h: system voice by SYSVOICE_* id (h1 = SYSVOICE_STARTUP after the brand logo), played by the Lua layer | Handled | — | [vm.rs:1277](../../crates/crysirror-willplus/src/vm.rs#L1277) |
| `0xFD` | 0xFB b1/b0 brackets the brand logos in start.ws2 and 0xFD precedes new-game/replay fades; neither has a visible effect … | Empty handler | Original no-op status not established | [vm.rs:1274](../../crates/crysirror-willplus/src/vm.rs#L1274) |
| `0xFE` | Operand layout: s | Decode only | No extracted executor route | [tables.rs:421](../../crates/crysirror-willplus/src/tables.rs#L421) |
| `0xFF` | End script | Handled | — | [vm.rs:1152](../../crates/crysirror-willplus/src/vm.rs#L1152) |

## Subselectors and conditional dispatch

Patterns retain their source namespace and guards. `_` is the fallback of that table. Boolean/optional results follow the function's return contract. Call-site names identify the implementation to inspect, including indirect service calls.

### layers.rs / ease

```rust
fn ease(kind: i64, t: f64) -> f64
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `kind` | `1` | Local state/value operations | Handled | [173](../../crates/crysirror-willplus/src/layers.rs#L173) |
| `kind` | `2` | Local state/value operations | Handled | [174](../../crates/crysirror-willplus/src/layers.rs#L174) |
| `kind` | `3` | Local state/value operations | Handled | [175](../../crates/crysirror-willplus/src/layers.rs#L175) |
| `kind` | `_` | Local state/value operations | Handled | [176](../../crates/crysirror-willplus/src/layers.rs#L176) |

### layers.rs / Layers::call

```rust
fn call(&mut self, method: &str, args: &[Value], now: u64, cursor: [f64; 2], geometry: &mut dyn Geometry) -> Result<Vec<Value>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `method` | `_` | Local state/value operations | Handled | [370](../../crates/crysirror-willplus/src/layers.rs#L370) |
| `condition; under method="forwardLayerOrder" \| "swapLayerOrder"` | `method == "swapLayerOrder"` | Local state/value operations | Handled | [725](../../crates/crysirror-willplus/src/layers.rs#L725) |
| `method` | `other` | Local state/value operations | Unsupported: AdvHD lua_Layers:{other} is not connected | [792](../../crates/crysirror-willplus/src/layers.rs#L792) |

### log.rs / call

```rust
fn call(layers: &mut Layers, log: &[LogEntry], fonts: &BTreeMap<(String, String), crate::vm::FontStyle>, method: &str, args: &[Value], redraw: &mut bool) -> Result<Option<Vec<Value>>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `method == "insertLogLayer"` | Params::default, BTreeMap::new | Handled | [109](../../crates/crysirror-willplus/src/log.rs#L109) |
| `method` | `_` | Local state/value operations | Handled | [173](../../crates/crysirror-willplus/src/log.rs#L173) |

### program.rs / Decoder<'_, F>::instruction

```rust
fn instruction(&mut self, choice_body: bool) -> Result<Instruction>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `opcode == 0x0f && choice_body` | self.operands, self.instruction | Handled | [136](../../crates/crysirror-willplus/src/program.rs#L136) |

### save.rs / file_name

```rust
fn file_name(mode: i64, slot: i64) -> Option<String>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `mode` | `MODE_NORMAL if (1..=999).contains(&slot)` | Local state/value operations | Handled | [58](../../crates/crysirror-willplus/src/save.rs#L58) |
| `mode` | `MODE_QUICK if (1..=5).contains(&slot)` | Local state/value operations | Handled | [59](../../crates/crysirror-willplus/src/save.rs#L59) |
| `mode` | `MODE_AUTO if (1..=5).contains(&slot)` | Local state/value operations | Handled | [60](../../crates/crysirror-willplus/src/save.rs#L60) |
| `mode` | `MODE_SELECT` | Local state/value operations | Handled | [61](../../crates/crysirror-willplus/src/save.rs#L61) |
| `mode` | `MODE_RESUME` | Local state/value operations | Handled | [62](../../crates/crysirror-willplus/src/save.rs#L62) |
| `mode` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [63](../../crates/crysirror-willplus/src/save.rs#L63) |

### save.rs / Machine::run_processes

```rust
fn run_processes(&mut self, host: &mut dyn HostServices) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `name` | `"LUA_PROCESS_SAVEGAME"` | self.save_to | Handled | [253](../../crates/crysirror-willplus/src/save.rs#L253) |
| `name` | `"LUA_PROCESS_QUICKSAVEGAME"` | self.newest, self.save_to | Handled | [254](../../crates/crysirror-willplus/src/save.rs#L254) |
| `name` | `"LUA_PROCESS_SELECTSAVEGAME"` | self.save_to | Handled | [258](../../crates/crysirror-willplus/src/save.rs#L258) |
| `name` | `"LUA_PROCESS_LOADGAME"` | self.load_from | Handled | [259](../../crates/crysirror-willplus/src/save.rs#L259) |
| `name` | `"LUA_PROCESS_QUICKLOADGAME"` | self.newest, self.load_from | Handled | [260](../../crates/crysirror-willplus/src/save.rs#L260) |
| `name` | `"LUA_PROCESS_AUTOLOADGAME"` | self.newest, self.load_from | Handled | [264](../../crates/crysirror-willplus/src/save.rs#L264) |
| `name` | `"LUA_PROCESS_SELECTLOADGAME"` | self.load_from | Handled | [268](../../crates/crysirror-willplus/src/save.rs#L268) |
| `name` | `"LUA_PROCESS_RESUMEGAME"` | self.load_from | Handled | [269](../../crates/crysirror-willplus/src/save.rs#L269) |
| `name` | `"LUA_PROCESS_TITLE"` | self.to_title | Handled | [270](../../crates/crysirror-willplus/src/save.rs#L270) |
| `name` | `"LUA_PROCESS_BACKGAME"` | usize::try_from, self.state.log.len, self.state.log.truncate, self.load_state | Handled | [273](../../crates/crysirror-willplus/src/save.rs#L273) |
| `name` | `"LUA_PROCESS_QUIT"` | self.save_global | Handled | [279](../../crates/crysirror-willplus/src/save.rs#L279) |
| `name` | `"LUA_PROCESS_VOICEREPEAT"` | self.state.line_voice.clone, crate::vm::sound_channel, self.play | Handled | [283](../../crates/crysirror-willplus/src/save.rs#L283) |
| `name` | `"LUA_PROCESS_LOG_VOICEPLAY"` | usize::try_from, self.state.log.get, crate::vm::sound_channel, self.play | Handled | [289](../../crates/crysirror-willplus/src/save.rs#L289) |
| `name` | `"LUA_PROCESS_UPDATEDATA" \| "LUA_PROCESS_UPDATEMSGSPEED" \| "LUA_PROCESS_UPDATEBGMCUT" \| "LUA_PROCESS_UPDATEBGV" \| "LUA_PROCESS_ASPECTMODE" \| "LUA_PROCESS_SCREENMODE" \| "LUA_PROCESS_MINIMIZED" \| "LUA_PROCESS_MOUSEMOVE" \| "LUA_PROCESS_CHANGELANGUAGE" \| "LUA_PROCESS_RTSNAP"` | Local state/value operations | Empty handler: Original no-op status not established | [299](../../crates/crysirror-willplus/src/save.rs#L299) |
| `name` | `other` | Local state/value operations | Partial: AdvHD NeedProcess({other}, {argument}) is not connected | [301](../../crates/crysirror-willplus/src/save.rs#L301) |

### vm.rs / ease

```rust
fn ease(mode: u8, t: f32) -> f32
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `mode` | `2` | Local state/value operations | Handled | [318](../../crates/crysirror-willplus/src/vm.rs#L318) |
| `mode` | `3` | Local state/value operations | Handled | [319](../../crates/crysirror-willplus/src/vm.rs#L319) |
| `mode` | `_` | Local state/value operations | Handled | [320](../../crates/crysirror-willplus/src/vm.rs#L320) |

### vm.rs / Bridge<'_>::cfunc

```rust
fn cfunc(&mut self, name: &str, args: &[crysirror_lua::Value]) -> Result<Vec<crysirror_lua::Value>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `name` | `other` | Local state/value operations | Unsupported: AdvHD cfunc.{other} is not connected | [1719](../../crates/crysirror-willplus/src/vm.rs#L1719) |

### vm.rs / Bridge<'_>::file_method

```rust
fn file_method(&mut self, this: u32, name: &str, a: &[crysirror_lua::Value]) -> Result<Vec<crysirror_lua::Value>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `name` | `"read"` | Local state/value operations | Partial: AdvHD file:read({format:?}) is not connected | [1897](../../crates/crysirror-willplus/src/vm.rs#L1897) |
| `name` | `"write"` | Local state/value operations | Handled | [1924](../../crates/crysirror-willplus/src/vm.rs#L1924) |
| `name` | `"close"` | VfsPath::new, self.services.write_atomic, self.registry.objects.remove | Handled | [1931](../../crates/crysirror-willplus/src/vm.rs#L1931) |
| `name` | `other` | Local state/value operations | Unsupported: AdvHD file:{other} is not connected | [1940](../../crates/crysirror-willplus/src/vm.rs#L1940) |

### vm.rs / Bridge<'_>::library

```rust
fn library(&mut self, (lib, name): (&str, &str), args: &[crysirror_lua::Value]) -> Result<Vec<crysirror_lua::Value>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `(lib, name)` | `("os", "time")` | Local state/value operations | Handled | [1949](../../crates/crysirror-willplus/src/vm.rs#L1949) |
| `(lib, name)` | `("os", "clock")` | Local state/value operations | Handled | [1950](../../crates/crysirror-willplus/src/vm.rs#L1950) |
| `(lib, name)` | `("os", "date")` | i64::from | Partial: AdvHD os.date format {format:?} is not connected | [1951](../../crates/crysirror-willplus/src/vm.rs#L1951) |
| `(lib, name)` | `("os", "remove")` | self.services.storage | Handled | [1960](../../crates/crysirror-willplus/src/vm.rs#L1960) |
| `(lib, name)` | `("io", "open")` | Vec::new, self.services.storage, self.registry.add | Handled | [1968](../../crates/crysirror-willplus/src/vm.rs#L1968) |
| `(lib, name)` | `other` | Local state/value operations | Unsupported: AdvHD {}.{} is not connected | [1987](../../crates/crysirror-willplus/src/vm.rs#L1987) |

### vm.rs / Bridge<'_>::method

```rust
fn method(&mut self, lua: &mut crysirror_lua::Lua, object: crate::system::Object, id: u32, name: &str, args: &[crysirror_lua::Value]) -> Result<Vec<crysirror_lua::Value>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `name; under (class, name)=("lua_Layers", _)` | `"create"` | Local state/value operations | Handled | [1745](../../crates/crysirror-willplus/src/vm.rs#L1745) |
| `name; under (class, name)=("lua_Layers", _)` | `_` | crate::log::call, self.state.ui.call | Handled | [1746](../../crates/crysirror-willplus/src/vm.rs#L1746) |
| `(class, name)` | `(class, name)` | Local state/value operations | Unsupported: AdvHD {class}:{name} is not connected | [1859](../../crates/crysirror-willplus/src/vm.rs#L1859) |

### vm.rs / Machine::execute

```rust
fn execute(&mut self, host: &mut dyn HostServices, program: &Program, i: &Instruction, now: u64) -> Result<Option<VmYield>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under i.opcode=0x01` | `op & 0x80 != 0` | self.var | Handled | [1116](../../crates/crysirror-willplus/src/vm.rs#L1116) |
| `op & 0x7f; under i.opcode=0x01` | `0` | Local state/value operations | Handled | [1122](../../crates/crysirror-willplus/src/vm.rs#L1122) |
| `op & 0x7f; under i.opcode=0x01` | `1` | Local state/value operations | Handled | [1123](../../crates/crysirror-willplus/src/vm.rs#L1123) |
| `op & 0x7f; under i.opcode=0x01` | `2` | Local state/value operations | Handled | [1124](../../crates/crysirror-willplus/src/vm.rs#L1124) |
| `op & 0x7f; under i.opcode=0x01` | `3` | Local state/value operations | Handled | [1125](../../crates/crysirror-willplus/src/vm.rs#L1125) |
| `op & 0x7f; under i.opcode=0x01` | `other` | Local state/value operations | Unsupported: AdvHD 0x01 comparison operator {other} is not connected | [1126](../../crates/crysirror-willplus/src/vm.rs#L1126) |
| `condition; under i.opcode=0x09` | `op & 0x80 != 0` | self.var | Handled | [1165](../../crates/crysirror-willplus/src/vm.rs#L1165) |
| `op & 0x7f; under i.opcode=0x09` | `0` | Local state/value operations | Handled | [1168](../../crates/crysirror-willplus/src/vm.rs#L1168) |
| `op & 0x7f; under i.opcode=0x09` | `1` | Local state/value operations | Handled | [1169](../../crates/crysirror-willplus/src/vm.rs#L1169) |
| `op & 0x7f; under i.opcode=0x09` | `2` | Local state/value operations | Handled | [1170](../../crates/crysirror-willplus/src/vm.rs#L1170) |
| `op & 0x7f; under i.opcode=0x09` | `3` | Local state/value operations | Handled | [1171](../../crates/crysirror-willplus/src/vm.rs#L1171) |
| `op & 0x7f; under i.opcode=0x09` | `other` | Local state/value operations | Unsupported: AdvHD 0x09 operator {other} is not connected | [1172](../../crates/crysirror-willplus/src/vm.rs#L1172) |
| `condition; under i.opcode=0x1f \| 0x29` | `id == "*"` | self.state.channels.iter | Handled | [1238](../../crates/crysirror-willplus/src/vm.rs#L1238) |
| `condition; under i.opcode=0x1f \| 0x29` | `id == "*"` | self.state.music.clear | Handled | [1239](../../crates/crysirror-willplus/src/vm.rs#L1239) |
| `condition; under i.opcode=0x28` | `id.starts_with("char")` | Local state/value operations | Handled | [1253](../../crates/crysirror-willplus/src/vm.rs#L1253) |
| `condition; under i.opcode=0x57` | `short(&o[1])? != 0 && self.tweens_pending(&name)` | Wait::Tweens, VmYield::WaitUntil | Handled | [1458](../../crates/crysirror-willplus/src/vm.rs#L1458) |
| `i.opcode` | `other` | Local state/value operations | Unsupported: AdvHD WS2 opcode {other:#04x} is not connected | [1551](../../crates/crysirror-willplus/src/vm.rs#L1551) |

### vm.rs / Machine::flush

```rust
fn flush(&mut self, host: &mut dyn HostServices) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Some(id) = op["layer"]["id"].as_u64()` | Local state/value operations | Handled | [544](../../crates/crysirror-willplus/src/vm.rs#L544) |

### vm.rs / Machine::flush_system

```rust
fn flush_system(&mut self, host: &mut dyn HostServices, ops: &mut Vec<Json>, live: &mut BTreeSet<u32>) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under &item.kind=Kind::Pna { path }` | `!sub.visible \|\| sub.params.color[3] <= 0.` | Local state/value operations | Handled | [701](../../crates/crysirror-willplus/src/vm.rs#L701) |
| `condition; under &item.kind=Kind::Text { size, margins, line, text, font_size, color, edge, reveal }` | `set.name == "system" && item.key == "Txt01"` | self.state.msg.page.clone, self.message_visible, host.now_ms | Handled | [768](../../crates/crysirror-willplus/src/vm.rs#L768) |
| `condition; under &item.kind=Kind::Text { size, margins, line, text, font_size, color, edge, reveal }` | `set.name == "system" && item.key == "Name01"` | self.state.speaker_raw.is_empty, self.state.speaker.clone, self.state.speaker_raw.clone | Handled | [770](../../crates/crysirror-willplus/src/vm.rs#L770) |

### vm.rs / Machine::lua_call

```rust
fn lua_call(&mut self, host: &mut dyn HostServices, name: &str, args: Vec<crysirror_lua::Value>) -> Result<Vec<crysirror_lua::Value>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!self.state.system_ready && !matches!(name, "initLegacyGame" \| "initSystemScreen" \| "checkInitVersion")` | self.lua_call, std::env::var_os | Handled | [390](../../crates/crysirror-willplus/src/vm.rs#L390) |
| `condition` | `matches!(name, "initLegacyGame" \| "initSystemScreen" \| "checkInitVersion")` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [390](../../crates/crysirror-willplus/src/vm.rs#L390) |

### vm.rs / Machine::shake

```rust
fn shake(&mut self, now: u64) -> [f32; 2]
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `quake.kind == 4` | Local state/value operations | Handled | [940](../../crates/crysirror-willplus/src/vm.rs#L940) |

### vm.rs / Machine::step

```rust
fn step(&mut self, host: &mut dyn HostServices, input: InputFrame, budget: u32) -> Result<VmYield>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `spec.split_once(':').is_some_and(\|(scene, pc)\| scene.eq_ignore_ascii_case(&self.state.scene) && u32::from_str_radix(pc.trim_start_matches("0x"), 16) == Ok(instruction.pc))` | self.flush, VmYield::WaitUntil | Handled | [1075](../../crates/crysirror-willplus/src/vm.rs#L1075) |

### vm.rs / Machine::tick_tweens

```rust
fn tick_tweens(&mut self, now: u64) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `track.mode == 1` | Local state/value operations | Handled | [901](../../crates/crysirror-willplus/src/vm.rs#L901) |
