# CMVS self-admitted technical debt

## SATD-cmvs-x64-dispatch

**Scope and current location.** The `Pc20160201X64` instruction and syscall
contracts in [NATIVE_ABI.md](NATIVE_ABI.md), the implementation in
[`src/vm`](src/vm) and [`src/adapter.rs`](src/adapter.rs), and the unresolved
entries in the [generated inventory](../../docs/native-api/cmvs.md). Each
inventory row identifies its native profile and selector.

**Remaining work.** Recover and implement every unresolved instruction,
operand form, syscall, subselector, error and wait for this profile. The
inventory retains the current branch-specific gaps, including image/draw
state, native saves and the previously connected syscall paths with open
behavior. The original dispatch table is the coverage baseline.

**Closure evidence.** Verify all tracked native contracts through their
normal entry points, close the inventory gaps, and record the installed-game
acceptance in [the validation notes](../../docs/validation/cmvs-amatsutsumi.md).

| Date | Change | Remaining scope |
| --- | --- | --- |
| 2026-11-19 | Recorded the existing ABI backlog under ADR 0105. Recovered and connected the software effect pause/rewind and voice retirement/wait contracts from the IDA analysis. Resuming a retained voice also covers the native EOF and pending-retirement states. | The generated inventory records all open and unreviewed entries. The DirectSound cursor contract has its own record below. |
| 2026-11-19 | Completed unrestricted pointer gesture mode 2 and its result and threshold syscalls. | See the remaining entries in the generated inventory. |
| 2026-11-19 | Implemented `22b7` newest-slot selection, including the native range guard, ties and bounded header reads. | Original CSV save and restore and the remaining selectors in the inventory. |
| 2026-11-19 | Moved save filename resolution from `src/vm/save_menu.rs::scan_latest_slot` into `slot_paths` and shared header reads into `read_slot_headers` in the same file. Implemented the CSV slot probe and existing-slot metadata edits through those paths. | The preview syscall and whole-slot CSV capture and restore remain open. |
| 2026-11-19 | Connected `2031` object image release and the `22b1` history-save limit. Applied the limit when capturing native history. | File-backed history capture and restore still depend on the whole-slot CSV implementation. |
| 2026-11-19 | Connected and verified `22bd` native metadata and preview replacement through the normal headless player. See the [validation record](../../docs/validation/cmvs-amatsutsumi.md#save-metadata-and-preview-syscall). | Original `22b5` restore, `22b6` write and `22b8` thumbnail capture remain open. |
| 2026-11-19 | Verified `22b8` shared frame capture and retained native BMP through the headless player. See the [validation record](../../docs/validation/cmvs-amatsutsumi.md#native-thumbnail-capture). | Original `22b5` restore and `22b6` write still need native checkpoint and live-state reconstruction. |
| 2026-11-19 | Added the isolated CSV2 slot framing in `src/vm/save_slot.rs`, the expanded PS2A image path in `src/script.rs`, and the shared bytewise encoder in `crysirror-resource`. Verified against synthetic output from the original writer. | `22b2`, `22b5` and `22b6` still need native checkpoint record mapping, scoped storage and undo writes, and transactional restoration of VM, scene, text and audio. |
| 2026-11-19 | Connected `22ff` to the 2016 startup setting at `src/adapter/display.rs`. Matched CD3DDevice's field writer and checked configured results 0 and 1 through the headless player. Removed the success-only 2016 `202b` branch in `src/adapter/visual.rs::Objects::property_command`. The 2021 error branch now shares the renderer-target dispatch at `src/adapter.rs`. | Recover and connect the full 2016 texture source and destination paths before restoring `202b`. Native whole-slot save compatibility and the frame counter scheduling remain open. |

## SATD-cmvs-20211007-profile

**Scope and current location.** The `Pc20211007X64` profile in
[`src/vm.rs`](src/vm.rs), keyed archive registration in
[`src/vm/startup.rs`](src/vm/startup.rs) and
[`src/vm/resources.rs`](src/vm/resources.rs), and the gate
`unresolved_in_20211007` in [`src/vm/noop.rs`](src/vm/noop.rs). The profile
shares the 2016 handlers, which have not been reviewed against this build.

**Remaining work.** Dispatch reports unsupported for `0x2002` (the
CMainFrameWnd counter and latch are recovered, while their host scheduling
is unresolved) and, in the 2016 profile only, the render targets
`0x2300`/`0x2301` with the draw groups 8 and 9 and record +6200 (`0x202b`) that
sample them. The 2016 registration checks `0x22c0`/`0x22c1` report unregistered:
they validate a machine-bound `comap.dat` this host cannot reproduce.
The `0x2293` rule-image reveal has a compiled shader path on Switch and
Vita. Vita player linking and runtime validation were paused by the owner.
Vita's mask path supports undistorted GPU blends. Combining those masks
with shader distortion or stored-byte blending remains outside its profile.

These connected paths are known to be incomplete: text decoration gradients
(`0x210f`), outline modes, draw record bit `0x2025`/`0x202d`, draw blend modes
(`0x204b`), record `+108` (`0x2075`), camera pitch and yaw on projected sprites,
rotation channel spin and direction flags, and `0x20e7` scaling of glyph cells.
The message adapter reads the separate `MESSAGE_INFO/FONT` face name. Image
font sheets (`0x23c0`) take the red channel of decoded RGBA instead of checking for
an 8-bit source; image glyphs draw without outline. Scene movie mode 15 follows the
movie clock, not sound channel 11. Step-list window records (`0x228b`) replace a
window part without a crossfade. Baked picture stacks (`0x2312`) and the text debug
records are not part of saves, as in the original.

**Closure evidence.** Replay KUNADO through the normal headless player from
logo to title, settings and story, then record it in a validation note.

| Date | Change | Remaining scope |
| --- | --- | --- |
| 2026-11-19 | Added detection, CPZ7 V7B archives and keyed registration. Gated selectors fail visibly with their PC. | KUNADO stops at `0x23ac` in `start.ps3`. All items above remain open. |
| 2026-11-19 | Connected text objects `0x2100..=0x211d`, proportional glyph advances, draw record commands, camera projection and keyframe channels `0x21e0..=0x21fd`. | KUNADO reaches its title, load screen and story opening. The incomplete paths and open selectors above remain. |
| 2026-11-19 | Connected `0x2129` native `system.dat` writes. A unit test reads the output back through the CSS1 decoder. | No replay has reached a `0x2129` call site, so the file write is unverified in a running game. |
| 2026-11-19 | Added MGV1 playback through the shared Theora and Vorbis decoders. Connected every remaining dispatch slot except `0x22b5` and `0x22b6`. [`src/adapter/retained_table.rs`](src/adapter/retained_table.rs) reclaims arguments for 119 of them without native output. | 216 slots in the 2016 profile and 230 in the 2021 profile are placeholders. `0x22b5` and `0x22b6` remain unimplemented. MGV playback has only been decoded directly, not reached through `op.ps3`. |
| 2026-11-19 | Implemented `0x22b5` and `0x22b6` with a CrySirror payload inside native CSV2 framing, window text commands, image variants, screen capture and the `0x2282..=0x228a` step list. | Slots are not interchangeable with the original program. Step lists apply final state without timed playback. `0x22c4` replaces the image without its crossfade. Effects `0x22a0..=0x22af`, object emitters `0x2328`, list objects and text fields remain placeholders. |
| 2026-11-19 | Connected text fields `0x2380..=0x23b9` to text objects, list objects `0x2370..=0x237b` to button groups, and `0x22c4..=0x22c7` image crossfades. | Text field scrolling, clipping and frame images, list scrolling, effects `0x22a0..=0x22af`, raster effects `0x2328`, lighting `0x2260..=0x2275` and timed step lists remain open. |
| 2026-11-19 | Added CMV5 and CMV7 to the shared decoder with synthetic frame tests. Connected text field scrolling, framing and measurement `0x2390..=0x23bb` and `0x20e4`, list commands `0x2218`, `0x2219`, `0x2378..=0x237a`, native reverb limits and the world light state `0x2260..=0x2275`. | No CMV5 or CMV7 sample has been decoded. These natives were read from the 2021 build only. The light state does not reach drawing because its consumers are unrecovered. `0x2218` clears the pressed button by inference from native field +8. Effects `0x2290..=0x22af`, raster effects `0x2328..=0x2332`, video object commands `0x230b..=0x231e`, object commands `0x2310..=0x2322`, `0x2397`, `0x237c..=0x237f`, `0x23a4..=0x23a8`, `0x23c0..=0x23c2` and window settings remain placeholders. |
| 2026-11-19 | Owner chose complete syscall coverage for both profiles before game acceptance. A comparison of the two dispatch tables left 427 selectors open: unreviewed, open or new in 2021. | Implement whole families in this order: text objects `0x2100..=0x211d` with `0x23ac`, scene object properties `0x2025..=0x2078`, then the remaining families. |
| 2026-11-19 | Implemented the remaining placeholder families for both profiles: object effects `0x22a1..=0x22a9` and `0x22af` with type 6 collision on object point lists `0x2330..=0x2332`, rasters `0x2328`/`0x2329`, the timed step list `0x2280..=0x228b` in the VM with slot file commands `0x228c..=0x228e`, picture stacks `0x2310..=0x2317` on the shared `layer_stack`, the scene movie frame clock `0x2308..=0x231e`, list commands `0x237c..=0x237f`, field mattes `0x2397`, gesture `0x2354`/`0x2355`, the text debug dialog `0x23a4..=0x23a8`, the 2021 image font `0x23c0..=0x23c2` on the shared `alpha_cells`, the window, display and settings commands, dialogs `0x213a..=0x213d`, host commands `0x201c`, `0x20cb`, `0x20cc`, `0x2099` and the 2016 variants of `0x216b`, `0x22c0` and `0x22c1`. Saves keep effect records, scene movie clocks, second textures and field mattes. KUNADO and AMATSUTSUMI replays still reach their story text. | The selectors and incomplete paths listed above. Most new families have no call site in either sample game, so only unit tests cover them. |
| 2026-11-19 | Removed the incomplete success paths at `src/adapter/retained_table.rs` and `src/adapter/retained.rs::Retained`. These entries now fail at the existing unsupported-syscall branch in `src/adapter.rs`. IDA recovered the 2016 CMainFrameWnd counter and latch and the initial CD3DDevice `+144` word. | Preserve the open contracts above. Recover the counter's host call boundaries, the device word's update path, and the shared rendering paths for the native situations. |
| 2026-11-19 | Traced the 2021 font face loader and the distinct `TEXT_REVISION_ALPHA` option. Removed the old `src/adapter/display.rs` `22f9` route that overwrote the configured face with a numeric string. Recovered the 2016 targets' allocation size, group ordering, source selector and save record. | Implement the glyph raster effect and connect `22f9`. Map the target drawing and backing to the existing shared persistent-target service. |
| 2026-11-19 | Connected the 2021 `22f9` setting and both native coverage curves through the shared outline rasterizer. Headless tests confirmed the setting round trip, preserved font name and visible edge coverage change. IDA confirmed both counter latch addresses, the 3D scene slots and the distinct 2016 output and source fields. | 3D rendering and sound, native counter scheduling, 2016 render target lifecycle and native whole-slot compatibility remain open. |
| 2026-11-19 | Connected the ground/water geometry and walking-camera scene in `src/adapter/scene_effect.rs`, including texture ownership, shared projection, resident cues, status and retirement. Corrected the four-word cleanup for `22af`. Validated both ABI variants through the muted headless player. | The native fixed scenes `2292`–`2297` still need their complete asset, rendering, camera and audio contracts. |
| 2026-11-19 | Moved the type `0x50` texture ownership and layer publication into `MeshScene` in `src/adapter/scene_effect.rs`. The type `0x56` rotating-quad scene (`2296`) uses the same path and passed muted headless create, update, status and retirement checks in both profiles. | Scenes `2292`–`2295` and `2297` remain open. |
| 2026-11-19 | Moved the walk-specific resource preparation in `src/adapter.rs` into the shared special-scene loader. Connected type `0x52` (`2292`) in `src/adapter/scene_effect.rs` with its nine textures, two resident cues, queued phases and slot retirement. Muted headless scripts exercised both phases in both ABI profiles. | Scenes `2293`–`2295` and `2297` remain open. Type `0x53` additionally needs the original two-texture shader transition, width-selected resource fallback, and phase clock. |
| 2026-11-19 | Moved scoped texture publication in `src/adapter/scene_effect.rs` into a common constructor and replacement path. Connected type `0x57` (`2297`) with its atlas quads, native random sequence, four revolving planes and two-image reload on trigger 10. Verified both ABI profiles through the muted headless player. | Scenes `2293`–`2295` remain open. The type `0x53` shader samples the rule image's green channel and multiplies source alpha by `clamp(2 × elapsed / 6000 − rule_green, 0, 1)`. |
| 2026-11-19 | Connected type `0x53` (`2293`) in `src/adapter/scene_effect/door.rs` through the shared continuous mask. Connected type `0x54` (`2294`) in `src/adapter/scene_effect/particles.rs` through shared projected meshes and resident audio. The texture reload previously used by type `0x57` now also handles the type `0x54` atlas replacement. Both ABI profiles passed muted headless checks. | Type `0x55` (`2295`) remains open. The type `0x53` continuous mask requires the console quad v2 shader package for Vita and Switch. |
| 2026-11-19 | Connected type `0x55` (`2295`) in `src/adapter/scene_effect/actors.rs`. It reuses the existing projected sprite camera and atlas painting in `src/adapter/visual.rs` and the resident duplicate playback in `src/adapter/sample.rs`. All eight fixed 3D scene selectors now run in the desktop player. | Console shader packages for the `2293` reveal and the other outstanding profile work above remain open. |
| 2026-11-19 | Connected `0x2321` in `src/adapter.rs` and verified the KUNADO gallery call. Switched the shared renderer to quad v2 on both consoles. The experimental rebuild of the entire Vita quad catalogue was discarded after compiler timeouts. GXM selects the ten compiled mask fragments in `crysirror-wgpu-gxm/src/gxm.rs` and keeps the existing distortion variants for other draws. | Switch packaged the DKSH pipeline. Vita player linking and playback are paused. The Vita profile limitations and the other outstanding native work above remain open. |

## SATD-cvns-20040515-profile

**Scope and current location.** `Cvns20040515X86` in [`src/vm.rs`](src/vm.rs),
its gate in [`src/vm/startup.rs`](src/vm/startup.rs), its routes and x86
no-effect entries in [`src/vm/cvns.rs`](src/vm/cvns.rs), VM selectors in
[`src/vm/startup/cvns.rs`](src/vm/startup/cvns.rs), the sprite scene in
[`src/adapter/cvns/`](src/adapter/cvns/mod.rs) and its section in the
[native inventory](../../docs/native-api/cmvs.md#cvns-20040515-x86-syscalls).

**Remaining work.** Review the script members
that fail a linear scan and check the remaining instruction, expression and
resource variants against the 2004 ABI. Verify the remaining story routes.

**Closure evidence.** Verify each connected route against original behavior
and run menu, route and original-save acceptance through the normal muted
headless player.

| Date | Change | Remaining scope |
| --- | --- | --- |
| 2026-11-19 | Replaced `tests/cvns_probe.rs` with `tests/cvns.rs`. Connected the PSS0 container and scoped CPZ1 resources, the frame-period commands and the 229 verified no-op selectors. Recorded the full native jump table. | Unreviewed selectors stop explicitly. CVNS VM, resource and game acceptance remain open. |
| 2026-11-19 | Rechecked the installed archive and ordered the first pending startup handlers by actual PSS0 offsets. Documented the mirrored bit and integer fills, the 256-cell string bank and the resource source registration that follows the audit call. | `210e` remains the first unsupported selector. Implement it together with the native bank representation and preserve the setup order. |
| 2026-11-20 | Routed 2016-equivalent selectors and kept the others in the native `0x2800` space. Added the startup VM selectors, the 256-cell string bank, `0xfffc` padding, the CVNS callback dispatch, script loads, the sprite/bitmap/fade scene in `src/adapter/cvns.rs` and PB2A tail reads. The startup, `intproc.pss`, `main.pss` and the brand logo now run. | The ADV windows `0x2110`–`0x2125` are the first unsupported selectors after the logo. Message windows `0x20d0`–`0x20df`, mask-font glyphs, input `0x2040`, music, voices, effect channels 6–47, saves (CSP1/`save000.dat`), the save-menu scan, tint weights and the remaining recorded selectors are open. |
| 2026-11-20 | Split the adapter scene into `src/adapter/cvns/` with keyframe animation (`0x2090`–`0x2094`, ADV parts), the per-sprite transitions of `0x4176e0`/`0x418910` (`fx.rs`), message windows (`0x20d0`–`0x20df`, `0x2100`/`0x2101`), ADV windows and buttons (`0x2110`–`0x2156`), the native input block, settings queries (`0x21ec`), music, voices, the checkpoint `0x21a0` and JBP1/tile PB2A tail reads. The prologue renders and story script `1kyo00.pss` starts with scripted clicks. | Paused by the owner for the CM3D2 port. Next stop: voice audio probing (`end of stream`) at the first voice. Open: `0x20fb` formatter, bitmap sub-images (`0x2050`/`0x2053`/`0x2058`/`0x205c`–`0x2067`), full-screen rule transitions `0x2188`/`0x218c`, shake `0x2193`/`0x2194`/`0x2196`, voice volume/queries, SE channels 6–47, sample channels 60–63, video `0x20c0`, history `0x2128`–`0x2136`, saves (CSP1, `save000.dat`, `0x21c1`), `0x21b0`, `0x21ed`, gradients, tint weights, wheel input, mask-font glyphs. |
| 2026-11-20 | Connected the MV2X synthesis filter in shared audio, CVNS voice load/start/stop/fade and the voice-synchronised wait. Added per-face glyph gradients, the x86 string selector mapping, the dialogue history table, 48-slot effect storage and CSS0 system data in `vm/css0.rs`. The shared 10/6 LZSS settings remain in `vm/system_save.rs`. | Original CSP1 play slots, video, remaining bitmap operations, native playback-position timing, sample channels, mask-font cells, menu/settings and route acceptance remain open. |
| 2026-11-20 | Connected the eight-component bitmap store and screen capture, PB2A frame selection, sprite tint, x86 shake, rule wipes and fullscreen CMV3 playback. Added the CVNS SE position query and the wheel counter. The shared video decoder now traverses JBP1 MCUs in raster order for CMV3. | CSP1 slots and menu scans, settings writes, mask-font cells, remaining audio controls and the unreviewed PSS0 dispatcher entries remain open. Verify the original movie frame-control modes and rendering against native captures. |
| 2026-11-20 | Clipped overhanging sprite crops and recovered the per-sprite rule marker, sprite dimensions, message reveal queries, damage request, numeric selectors, name-list access and calendar query. Mapped native sample streams 60–63 and separate voice, sample and SE cursor queries. The shared PCM16 mixer handles the 2004 SE and voice percentage gain and balance. Verified SE slot 47 at 100%, 50% and 0% and the independence of sample slot 3 from SE stop-all. | Gameplay CSP1 slots and menu scans, the complete settings writer, mask-font glyph rendering, movie frame-control modes, the remaining dispatcher entries and full route acceptance remain open. |
| 2026-11-20 | Connected original menu 130 to the CVNS slot count and enable commands (`0x21a1/0x21a2`). Verified the newest-slot check, case-insensitive header lookup, available save commands and disabled loads for missing or invalid slots. | The dispatcher still has 60 unreviewed and four open selectors. CSP1 gameplay checkpoint, write and restore, settings writes, mask glyphs, movie frame control and route acceptance remain open. |
| 2026-11-20 | Implemented ADV bitmap release and part existence, alpha, tint, rotation, scaling and drawing enable (`0x2131`, `0x213e`, `0x213f`, `0x2141`–`0x2143`, `0x214f`). These paths use the existing sprite compositor and return the native fault bits for invalid parts and bitmaps. | 53 unreviewed and four open syscall entries remain. Gameplay slot persistence, the settings writer, mask-font rendering, movie frame control and route acceptance are still open. |
| 2026-11-20 | Connected `0x21af` to presenter capture and shared BGR24 encoding with the x86 horizontal row interval. Reopening the title from the game's CSS0 file captured a 96×72 thumbnail. Added the window mode (`0x217b`) and display dimensions (`0x217f`) queries. | 50 unreviewed and four open syscall entries remain. The first story choice returns to the title, and the title can quit cleanly. Settings tabs and the return confirmation run. Changing effects stops at `intproc.pss+0xaf1c0` on `0x21ed`. Full CSP1 gameplay slots, mask fonts, movie frame control and other routes remain open. |
| 2026-11-20 | Connected the settings writer and persistence, registry values, frame clocks, window controls, screen-copy and CSP header operations. Moved mask-font ownership from VM registration records into the scene and shared glyph renderer. Added indexed fullscreen CMV delivery and six independently addressable scene clips. | The inventory has 13 unreviewed entries and one open entry. CSP1 gameplay serialization and detached scene reconstruction, display damage, soft reset, CD music mapping, clipboard and native dialogs still need acceptance. |
| 2026-11-20 | Completed the frame counter, reset into main.pss, CD track table, clipboard routes and name/font/colour dialogs. Validated the CSP1 outer codec and mixed-width checkpoint record framing against both directions of the original x86 slot codec. Audited the shared checkpoint route and the retained-display routes. | 308 handled, 229 original no-ops, seven open. Retained primary fill, rectangle submission and cancellation (`200b/200c/200d`) require the persistent surface. Native checkpoint capture (`2010/21a0`), slot write (`21a4`) and detached restore (`21a3`) require resource-dependent record serialization and reconstruction. End-to-end gameplay save/load and route acceptance remain open. |
| 2026-11-20 | Connected the three primary-surface calls in `adapter/cvns/primary.rs`. Added checked CSP1 record construction and visiting, capture of the VM execution, bank and history records at a checkpoint, and preservation of popped stack bytes. | 311 handled, 229 original no-ops, four open. Complete native scene capture for `2010/21a0`, numbered write `21a4` and detached restore `21a3`, then verify a fresh process loading the slot and continuing. |
| 2026-11-20 | Moved the VM execution codec into `vm/machine/csp.rs`. Added detached PC/stack/register/bank reconstruction, normal checkpoint capture of callback/timer data, bitmap recipes, ADV fields, movie references, display effects and SE/music records. The bitmap worker reconstructs named frames with their filters and rebakes component stacks. | The same four selectors remain open. Complete live scene reconstruction and publication, sprite and text/glyph records, scratch and callback state, and full slot I/O. |
| 2026-11-21 | Completed the remaining four checkpoint and slot selectors. The native writer lives in `adapter/cvns/slot.rs`, the detached scene reader in `adapter/cvns/restore.rs` and the VM transfer in `vm/startup/csp.rs`. Cold restores from both checkpoint sites reproduced the same frame. The player also resumed an original x86-generated slot. | The 544-entry syscall inventory has 315 handled selectors and 229 original no-ops. The profile stays open for script-member, instruction, resource-variant and remaining story-route acceptance. |

## SATD-cmvs-software-playback-clock

**Scope and current location.** Voice and SE position queries `20aa` and
`20b7` in [`src/adapter/sound.rs`](src/adapter/sound.rs) and the
[software voice ABI](NATIVE_ABI.md#software-voice-controls).

**Remaining work.** Match the original playback cursor observation, including
the first-buffer gate, accumulated buffer count and the half-block lead in
`0x1400877b0`. The shared mixer reports elapsed output frames since
activation. Paused, absent and retiring slots return the inactive sentinel.

**Closure evidence.** Cover the first and subsequent mix buffers, a loop,
pause and resume, a rewind after more than one buffer, and natural EOF
under the supported native sound-buffer modes.

| Date | Change | Remaining scope |
| --- | --- | --- |
| 2026-11-19 | Recovered the native time query in IDA. Connected a new activation interval and fenced decoder rewind through mixer publication. | Reproduce the native device-buffer accounting in the position queries and verify their result words. |

## SATD-cmvs-20070119-profile

**Scope.** The i386 CMVS configuration in `src/vm.rs`, PS2 transfers in
`src/vm/startup.rs`, the registration result in `src/vm/native_state.rs`
and the shared handlers reached through `src/adapter.rs`.

**2026-11-23.** Connected the startup executable, PS2A loader, CPZ3 resource
reader and profile-specific no-effect dispatch. The original installation
renders its disc-check dialog. `0x22c0` and `0x22c1` query the original
registration state with argument 1. In the original, `0x22c0` can launch the
bundled Windows registration helper and poll for its result. The adapter
currently reports unregistered. A successful registration path and title,
story and save behavior remain unverified. Other nontrivial syscall handlers
shared with later CMVS builds still require a per-selector comparison.

**Closure evidence.** Validate the positive and negative registration results
using original media in their normal environment, compare native operand and
result behavior for the remaining dispatch entries, and replay title, story
and save/restore from this profile's normal entry point.
