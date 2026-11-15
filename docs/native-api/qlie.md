# QLIE developer reference

[All engines](README.md)

Mode: `adopt`.

## ABI and host contract

Native `.s` script commands, value/control commands and object methods have
distinct namespaces. An object method requires its **kind/receiver plus name**;
a short method name is not globally unique. FilePack order, script source
positions and native object/thread state remain in the adapter.

| Native area | CrySirror service / implementation | Return / completion |
| --- | --- | --- |
| Control, values and expressions | `machine.rs` / `expression.rs` | Native labels, call/thread and value semantics. |
| Object and system methods | `adapter/objects.rs` and method modules | Receiver-kind guards apply before method dispatch. |
| ABMP/DPNG, cameras, filters and effects | Shared image-processing, Canvas and Layers | Resources and captures complete before publication. |
| Messages, buttons and native dialogs | Shared Text/UI/Input | Native result and wait state; callback routing stays scoped. |
| Sound filters, voice and movies | Shared Audio/Media | Native channel/cue and worker completion. |
| Saves / thumbnails / SystemData | Scoped Storage and image capture | CrySirror state body; original saves/SystemData are not imported. |

Game/patch packs are mounted in native priority order and the installed metadata
supplies the canvas. Guest scripts cannot escape client roots. Retired Twitter
posting and unconnected object/argument profiles fail visibly. The conditional
rules below retain `kind` guards rather than merging `list`, `timer`, `sys`,
`msg` and other receivers into one flat method table.

Source/provenance: [crate](../../crates/crysirror-qlie/),
[object dispatch](../../crates/crysirror-qlie/src/adapter/objects.rs).
Behavior: [save/effect validation](../validation/qlie-saves-effects-2026-11-06.md).

<!-- BEGIN GENERATED DISPATCH TABLES -->

## Opcode and API implementation tables

`Handled` means an executable source branch exists. It is not a claim that every argument/subselector or a complete game has been audited. Explicit gaps are shown per row.

| Status | Entries |
| --- | ---: |
| Empty handler | 14 |
| Handled | 664 |
| Partial | 3 |
| Unsupported | 2 |

These tables cover the checked-in catalogues and explicit dispatch branches. An engine-wide upstream denominator is not inferred from these rows.

## Object and system methods / adapter/abmp_native / abmp_native_method

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `?flt` | ?flt | Handled | — | [abmp_native.rs:236](../../crates/crysirror-qlie/src/adapter/abmp_native.rs#L236) |
| `?fo` | ?fo | Handled | — | [abmp_native.rs:236](../../crates/crysirror-qlie/src/adapter/abmp_native.rs#L236) |
| `?gfk` | ?gfk | Handled | — | [abmp_native.rs:148](../../crates/crysirror-qlie/src/adapter/abmp_native.rs#L148) |
| `?gfs` | ?gfs | Handled | — | [abmp_native.rs:148](../../crates/crysirror-qlie/src/adapter/abmp_native.rs#L148) |
| `?hit` | ?hit | Handled | — | [abmp_native.rs:47](../../crates/crysirror-qlie/src/adapter/abmp_native.rs#L47) |
| `?li` | ?li | Handled | — | [abmp_native.rs:247](../../crates/crysirror-qlie/src/adapter/abmp_native.rs#L247) |
| `?ls` | ?ls | Handled | — | [abmp_native.rs:247](../../crates/crysirror-qlie/src/adapter/abmp_native.rs#L247) |
| `?lsk` | ?lsk | Handled | — | [abmp_native.rs:59](../../crates/crysirror-qlie/src/adapter/abmp_native.rs#L59) |
| `?lst` | ?lst | Handled | — | [abmp_native.rs:59](../../crates/crysirror-qlie/src/adapter/abmp_native.rs#L59) |
| `?lv` | ?lv | Handled | — | [abmp_native.rs:77](../../crates/crysirror-qlie/src/adapter/abmp_native.rs#L77) |
| `?tmp` | ?tmp | Handled | — | [abmp_native.rs:186](../../crates/crysirror-qlie/src/adapter/abmp_native.rs#L186) |
| `?tt` | ?tt | Handled | — | [abmp_native.rs:117](../../crates/crysirror-qlie/src/adapter/abmp_native.rs#L117) |
| `flt` | flt | Handled | — | [abmp_native.rs:230](../../crates/crysirror-qlie/src/adapter/abmp_native.rs#L230) |
| `gfk` | gfk | Handled | — | [abmp_native.rs:130](../../crates/crysirror-qlie/src/adapter/abmp_native.rs#L130) |
| `gfs` | gfs | Handled | — | [abmp_native.rs:130](../../crates/crysirror-qlie/src/adapter/abmp_native.rs#L130) |
| `lidx` | lidx | Handled | — | [abmp_native.rs:99](../../crates/crysirror-qlie/src/adapter/abmp_native.rs#L99) |
| `lsk` | lsk | Handled | — | [abmp_native.rs:48](../../crates/crysirror-qlie/src/adapter/abmp_native.rs#L48) |
| `lst` | lst | Handled | — | [abmp_native.rs:48](../../crates/crysirror-qlie/src/adapter/abmp_native.rs#L48) |
| `lv` | lv | Handled | — | [abmp_native.rs:69](../../crates/crysirror-qlie/src/adapter/abmp_native.rs#L69) |
| `mem` | mem | Handled | — | [abmp_native.rs:196](../../crates/crysirror-qlie/src/adapter/abmp_native.rs#L196) |
| `ss` | ss | Handled | — | [abmp_native.rs:191](../../crates/crysirror-qlie/src/adapter/abmp_native.rs#L191) |
| `tmp` | tmp | Handled | — | [abmp_native.rs:161](../../crates/crysirror-qlie/src/adapter/abmp_native.rs#L161) |

## Object and system methods / adapter/camera / camera_method

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `?bm` | camex ?bm / ?col answer the stored blend and colour request | Handled | — | [camera.rs:267](../../crates/crysirror-qlie/src/adapter/camera.rs#L267) |
| `?col` | ?col | Handled | — | [camera.rs:271](../../crates/crysirror-qlie/src/adapter/camera.rs#L271) |
| `?mm` | ?mm | Handled | — | [camera.rs:256](../../crates/crysirror-qlie/src/adapter/camera.rs#L256) |
| `?p` | ?p | Handled | — | [camera.rs:209](../../crates/crysirror-qlie/src/adapter/camera.rs#L209) |
| `?p2` | ?p2/?p3: x, y, scale x/y percent, rotation degrees | Handled | — | [camera.rs:247](../../crates/crysirror-qlie/src/adapter/camera.rs#L247) |
| `?p3` | ?p2/?p3: x, y, scale x/y percent, rotation degrees | Handled | — | [camera.rs:247](../../crates/crysirror-qlie/src/adapter/camera.rs#L247) |
| `bm` | bm | Handled | — | [camera.rs:202](../../crates/crysirror-qlie/src/adapter/camera.rs#L202) |
| `col` | col | Handled | — | [camera.rs:116](../../crates/crysirror-qlie/src/adapter/camera.rs#L116) |
| `g` | g | Handled | — | [camera.rs:182](../../crates/crysirror-qlie/src/adapter/camera.rs#L182) |
| `g2` | g2 | Handled | — | [camera.rs:185](../../crates/crysirror-qlie/src/adapter/camera.rs#L185) |
| `i` | i | Handled | — | [camera.rs:46](../../crates/crysirror-qlie/src/adapter/camera.rs#L46) |
| `mm` | mm | Handled | — | [camera.rs:188](../../crates/crysirror-qlie/src/adapter/camera.rs#L188) |
| `p` | p | Handled | — | [camera.rs:222](../../crates/crysirror-qlie/src/adapter/camera.rs#L222) |
| `p2` | p2 | Handled | — | [camera.rs:222](../../crates/crysirror-qlie/src/adapter/camera.rs#L222) |
| `p3` | cam p/p2/p3 ( -> ): centre-relative position with optional scale percent and rotation degrees, then a duration | Handled | Guard: self.objects[id].kind == "cam" | [camera.rs:221](../../crates/crysirror-qlie/src/adapter/camera.rs#L221) |
| `pri` | pri | Handled | — | [camera.rs:58](../../crates/crysirror-qlie/src/adapter/camera.rs#L58) |
| `tf` | tf | Handled | — | [camera.rs:202](../../crates/crysirror-qlie/src/adapter/camera.rs#L202) |

## Object and system methods / adapter/cfg_settings / cfg_setting_method

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `fcd` | fcd | Handled | — | [cfg_settings.rs:235](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L235) |
| `fdd` | fdd | Handled | — | [cfg_settings.rs:204](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L204) |
| `?bgi` | ?bgi | Handled | — | [cfg_settings.rs:269](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L269) |
| `?blp` | ?blp | Handled | — | [cfg_settings.rs:259](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L259) |
| `?crc` | ?crc | Handled | — | [cfg_settings.rs:184](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L184) |
| `?dfn` | +168 Game.txt DefaultFont (empty -> MS Gothic), resolved by against the installed faces with Tahoma as fallback | Handled | — | [cfg_settings.rs:281](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L281) |
| `?dsf` | ?dsf | Handled | — | [cfg_settings.rs:184](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L184) |
| `?fcd` | ?fcd | Handled | — | [cfg_settings.rs:184](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L184) |
| `?fcs` | ?fcs | Handled | — | [cfg_settings.rs:184](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L184) |
| `?fdd` | ?fdd | Handled | — | [cfg_settings.rs:184](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L184) |
| `?fil` | ?fil | Handled | — | [cfg_settings.rs:308](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L308) |
| `?kct` | ?kct | Handled | — | [cfg_settings.rs:184](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L184) |
| `?mam` | ?mam | Handled | — | [cfg_settings.rs:184](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L184) |
| `?mas` | ?mas | Handled | — | [cfg_settings.rs:247](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L247) |
| `?mge` | ?mge | Handled | — | [cfg_settings.rs:184](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L184) |
| `?mht` | ?mht | Handled | — | [cfg_settings.rs:184](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L184) |
| `?mrc` | ?mrc | Handled | — | [cfg_settings.rs:184](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L184) |
| `?mre` | ?mre | Handled | — | [cfg_settings.rs:184](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L184) |
| `?nct` | ?nct | Handled | — | [cfg_settings.rs:183](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L183) |
| `?nfs` | ?nfs | Handled | — | [cfg_settings.rs:184](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L184) |
| `?rfs` | ?rfs | Handled | — | [cfg_settings.rs:184](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L184) |
| `?ssd` | ?ssd | Handled | — | [cfg_settings.rs:184](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L184) |
| `?ssm` | ?ssm | Handled | — | [cfg_settings.rs:184](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L184) |
| `?swe` | ?swe | Handled | — | [cfg_settings.rs:184](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L184) |
| `?swi` | ?swi | Handled | — | [cfg_settings.rs:184](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L184) |
| `?swp` | ?swp | Handled | — | [cfg_settings.rs:184](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L184) |
| `?tdm` | ?tdm | Handled | — | [cfg_settings.rs:184](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L184) |
| `?ttm` | ?ttm | Handled | — | [cfg_settings.rs:184](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L184) |
| `?vsw` | ?vsw | Handled | — | [cfg_settings.rs:184](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L184) |
| `bgi` | bgi | Handled | — | [cfg_settings.rs:265](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L265) |
| `blp` | Only parameter 0 exists; other indices are ignored | Handled | — | [cfg_settings.rs:253](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L253) |
| `blt` | Presentation/buffer setters of the older loader table | Handled | — | [cfg_settings.rs:213](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L213) |
| `crc` | crc | Handled | — | [cfg_settings.rs:196](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L196) |
| `cs` | cs | Handled | — | [cfg_settings.rs:229](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L229) |
| `dsf` | dsf | Handled | — | [cfg_settings.rs:235](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L235) |
| `fcs` | fcs | Handled | — | [cfg_settings.rs:235](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L235) |
| `fdm` | fdm | Handled | — | [cfg_settings.rs:217](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L217) |
| `fil` | +31C[i]; Game.txt OldFontILRate (default true) exchanges the stored interline value with the legacy 100 - rate form | Handled | — | [cfg_settings.rs:297](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L297) |
| `fps` | fps | Handled | — | [cfg_settings.rs:221](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L221) |
| `gim` | Presentation/buffer setters of the older loader table | Handled | — | [cfg_settings.rs:213](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L213) |
| `kct` | kct | Handled | — | [cfg_settings.rs:235](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L235) |
| `mam` | mam | Handled | — | [cfg_settings.rs:235](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L235) |
| `mas` | mas | Handled | — | [cfg_settings.rs:240](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L240) |
| `mct` | mct | Handled | — | [cfg_settings.rs:235](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L235) |
| `mge` | mge | Handled | — | [cfg_settings.rs:235](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L235) |
| `mht` | mht | Handled | — | [cfg_settings.rs:235](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L235) |
| `mrc` | mrc | Handled | — | [cfg_settings.rs:208](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L208) |
| `mre` | mre | Handled | — | [cfg_settings.rs:235](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L235) |
| `nfs` | nfs | Handled | — | [cfg_settings.rs:196](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L196) |
| `rfs` | rfs | Handled | — | [cfg_settings.rs:196](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L196) |
| `sb` | sb | Handled | — | [cfg_settings.rs:225](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L225) |
| `sfs` | sfs is accepted and ignored by | Empty handler | Original no-op status not established | [cfg_settings.rs:234](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L234) |
| `ssd` | ssd | Handled | — | [cfg_settings.rs:235](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L235) |
| `ssm` | ssm | Handled | — | [cfg_settings.rs:235](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L235) |
| `swe` | swe | Handled | — | [cfg_settings.rs:235](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L235) |
| `swi` | swi | Handled | — | [cfg_settings.rs:235](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L235) |
| `swp` | swp | Handled | — | [cfg_settings.rs:235](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L235) |
| `tdm` | tdm | Handled | — | [cfg_settings.rs:235](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L235) |
| `ttm` | ttm | Handled | — | [cfg_settings.rs:200](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L200) |
| `vsw` | vsw | Handled | — | [cfg_settings.rs:235](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L235) |

## Object and system methods / adapter/clip / clip_method

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `cb` | cb | Unsupported | QLIE clip\\cb: the host clipboard accepts text only | [clip.rs:27](../../crates/crysirror-qlie/src/adapter/clip.rs#L27) |
| `ci` | ci | Handled | — | [clip.rs:26](../../crates/crysirror-qlie/src/adapter/clip.rs#L26) |
| `cl` | cl | Empty handler | Original no-op status not established | [clip.rs:28](../../crates/crysirror-qlie/src/adapter/clip.rs#L28) |
| `ct` | ct | Empty handler | Original no-op status not established | [clip.rs:28](../../crates/crysirror-qlie/src/adapter/clip.rs#L28) |
| `gt` | gt | Empty handler | Original no-op status not established | [clip.rs:28](../../crates/crysirror-qlie/src/adapter/clip.rs#L28) |
| `pl` | pl | Empty handler | Original no-op status not established | [clip.rs:28](../../crates/crysirror-qlie/src/adapter/clip.rs#L28) |
| `pt` | pt | Empty handler | Original no-op status not established | [clip.rs:28](../../crates/crysirror-qlie/src/adapter/clip.rs#L28) |

## Object and system methods / adapter/dialogs/choice / choice_dialog_method

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `d1` | d1 | Handled | — | [choice.rs:61](../../crates/crysirror-qlie/src/adapter/dialogs/choice.rs#L61) |
| `d2` | d2 | Handled | — | [choice.rs:61](../../crates/crysirror-qlie/src/adapter/dialogs/choice.rs#L61) |
| `d3` | d3 | Handled | — | [choice.rs:61](../../crates/crysirror-qlie/src/adapter/dialogs/choice.rs#L61) |

## Object and system methods / adapter/files / file_method

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `c` | c | Handled | — | [files.rs:229](../../crates/crysirror-qlie/src/adapter/files.rs#L229) |
| `?l` | ?l | Handled | — | [files.rs:222](../../crates/crysirror-qlie/src/adapter/files.rs#L222) |
| `g` | g | Handled | — | [files.rs:239](../../crates/crysirror-qlie/src/adapter/files.rs#L239) |
| `gl` | gl | Handled | — | [files.rs:239](../../crates/crysirror-qlie/src/adapter/files.rs#L239) |
| `i` | i | Handled | — | [files.rs:214](../../crates/crysirror-qlie/src/adapter/files.rs#L214) |
| `l` | l | Handled | — | [files.rs:148](../../crates/crysirror-qlie/src/adapter/files.rs#L148) |

## Object and system methods / adapter/files / file_method / receiver=kind == "path"

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `a` | a | Handled | — | [files.rs:111](../../crates/crysirror-qlie/src/adapter/files.rs#L111) |
| `d` | d | Handled | — | [files.rs:125](../../crates/crysirror-qlie/src/adapter/files.rs#L125) |
| `?lk` | ?lk | Handled | — | [files.rs:141](../../crates/crysirror-qlie/src/adapter/files.rs#L141) |
| `i` | i | Handled | — | [files.rs:136](../../crates/crysirror-qlie/src/adapter/files.rs#L136) |
| `lk` | lk | Handled | — | [files.rs:140](../../crates/crysirror-qlie/src/adapter/files.rs#L140) |

## Object and system methods / adapter/gesture_guide / gesture_guide_method

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `mgi` | mgi | Handled | — | [gesture_guide.rs:57](../../crates/crysirror-qlie/src/adapter/gesture_guide.rs#L57) |
| `mgl` | mgl | Handled | — | [gesture_guide.rs:68](../../crates/crysirror-qlie/src/adapter/gesture_guide.rs#L68) |
| `mgp` | mgp | Handled | — | [gesture_guide.rs:87](../../crates/crysirror-qlie/src/adapter/gesture_guide.rs#L87) |

## Object and system methods / adapter/glyph_pictures / glyph_picture_method

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `glpa` | glpa | Handled | — | [glyph_pictures.rs:303](../../crates/crysirror-qlie/src/adapter/glyph_pictures.rs#L303) |
| `glpf` | glpf | Handled | — | [glyph_pictures.rs:268](../../crates/crysirror-qlie/src/adapter/glyph_pictures.rs#L268) |
| `glpi` | glpi | Handled | — | [glyph_pictures.rs:249](../../crates/crysirror-qlie/src/adapter/glyph_pictures.rs#L249) |
| `glpo` | glpo | Handled | — | [glyph_pictures.rs:258](../../crates/crysirror-qlie/src/adapter/glyph_pictures.rs#L258) |
| `glps` | glps | Handled | — | [glyph_pictures.rs:262](../../crates/crysirror-qlie/src/adapter/glyph_pictures.rs#L262) |
| `gt` | gt | Handled | — | [glyph_pictures.rs:341](../../crates/crysirror-qlie/src/adapter/glyph_pictures.rs#L341) |

## Object and system methods / adapter/image_native / image_native_method

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `a` | a,alpha,time (): the colour animation's alpha alone, with the same linear calculator as col | Handled | — | [image_native.rs:72](../../crates/crysirror-qlie/src/adapter/image_native.rs#L72) |
| `bc` | bc | Handled | — | [image_native.rs:160](../../crates/crysirror-qlie/src/adapter/image_native.rs#L160) |
| `?a` | ?a | Handled | — | [image_native.rs:83](../../crates/crysirror-qlie/src/adapter/image_native.rs#L83) |
| `?alg` | ?alg | Handled | — | [image_native.rs:99](../../crates/crysirror-qlie/src/adapter/image_native.rs#L99) |
| `?alr` | ?alr | Handled | — | [image_native.rs:127](../../crates/crysirror-qlie/src/adapter/image_native.rs#L127) |
| `?anc` | ?anc | Handled | — | [image_native.rs:106](../../crates/crysirror-qlie/src/adapter/image_native.rs#L106) |
| `?bc` | ?bc (): 1 once the button was clicked, the counter then held at 1; bc () clicks it at its own position | Handled | — | [image_native.rs:152](../../crates/crysirror-qlie/src/adapter/image_native.rs#L152) |
| `?bm` | ?bm | Handled | — | [image_native.rs:117](../../crates/crysirror-qlie/src/adapter/image_native.rs#L117) |
| `?col` | ?col | Handled | — | [image_native.rs:110](../../crates/crysirror-qlie/src/adapter/image_native.rs#L110) |
| `?in` | ?in | Handled | — | [image_native.rs:129](../../crates/crysirror-qlie/src/adapter/image_native.rs#L129) |
| `?l` | ?l | Handled | — | [image_native.rs:146](../../crates/crysirror-qlie/src/adapter/image_native.rs#L146) |
| `?mm` | ?mm | Handled | — | [image_native.rs:122](../../crates/crysirror-qlie/src/adapter/image_native.rs#L122) |
| `?ost` | ?ost | Handled | — | [image_native.rs:136](../../crates/crysirror-qlie/src/adapter/image_native.rs#L136) |
| `?out` | ?out | Handled | — | [image_native.rs:129](../../crates/crysirror-qlie/src/adapter/image_native.rs#L129) |
| `?p2` | ?p2: x, y, priority, scale x/y, the repeat value; ?p3 adds the rotation before the last value | Handled | — | [image_native.rs:51](../../crates/crysirror-qlie/src/adapter/image_native.rs#L51) |
| `?p3` | ?p2: x, y, priority, scale x/y, the repeat value; ?p3 adds the rotation before the last value | Handled | — | [image_native.rs:51](../../crates/crysirror-qlie/src/adapter/image_native.rs#L51) |
| `?ps` | ?ps | Handled | — | [image_native.rs:130](../../crates/crysirror-qlie/src/adapter/image_native.rs#L130) |
| `?sp` | ?sp | Handled | — | [image_native.rs:127](../../crates/crysirror-qlie/src/adapter/image_native.rs#L127) |
| `?tab` | ?tab | Handled | — | [image_native.rs:127](../../crates/crysirror-qlie/src/adapter/image_native.rs#L127) |
| `?tf` | ?tf | Handled | — | [image_native.rs:127](../../crates/crysirror-qlie/src/adapter/image_native.rs#L127) |
| `?v` | ?v | Handled | — | [image_native.rs:142](../../crates/crysirror-qlie/src/adapter/image_native.rs#L142) |
| `?vtn` | ?vtn | Handled | — | [image_native.rs:127](../../crates/crysirror-qlie/src/adapter/image_native.rs#L127) |
| `alr` | alr,v / ?alr (+3113) are kept with the anchor settings | Handled | — | [image_native.rs:95](../../crates/crysirror-qlie/src/adapter/image_native.rs#L95) |
| `p2` | p2 | Handled | — | [image_native.rs:21](../../crates/crysirror-qlie/src/adapter/image_native.rs#L21) |
| `p3` | p3 | Handled | — | [image_native.rs:21](../../crates/crysirror-qlie/src/adapter/image_native.rs#L21) |

## Object and system methods / adapter/input_device / input_device_method

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `?bsv` | ?bsv | Handled | — | [input_device.rs:131](../../crates/crysirror-qlie/src/adapter/input_device.rs#L131) |
| `?hit` | ?hit | Handled | — | [input_device.rs:116](../../crates/crysirror-qlie/src/adapter/input_device.rs#L116) |
| `?swe` | ?swe | Handled | — | [input_device.rs:131](../../crates/crysirror-qlie/src/adapter/input_device.rs#L131) |
| `bs` | bs | Handled | — | [input_device.rs:117](../../crates/crysirror-qlie/src/adapter/input_device.rs#L117) |
| `bsv` | +174/+178 store two script values; ?bsv returns them | Handled | — | [input_device.rs:122](../../crates/crysirror-qlie/src/adapter/input_device.rs#L122) |
| `i` | i | Handled | — | [input_device.rs:89](../../crates/crysirror-qlie/src/adapter/input_device.rs#L89) |
| `mp` | mp/smp,x,y move the host pointer ( SetCursorPos); without coordinates they answer the pointer position | Handled | — | [input_device.rs:146](../../crates/crysirror-qlie/src/adapter/input_device.rs#L146) |
| `ms` | ms | Handled | — | [input_device.rs:101](../../crates/crysirror-qlie/src/adapter/input_device.rs#L101) |
| `sk` | sk | Handled | — | [input_device.rs:94](../../crates/crysirror-qlie/src/adapter/input_device.rs#L94) |
| `smp` | mp/smp,x,y move the host pointer ( SetCursorPos); without coordinates they answer the pointer position | Handled | — | [input_device.rs:146](../../crates/crysirror-qlie/src/adapter/input_device.rs#L146) |

## Object and system methods / adapter/math / math_method

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `abs` | abs | Handled | — | [math.rs:187](../../crates/crysirror-qlie/src/adapter/math.rs#L187) |
| `atan` | atan | Handled | — | [math.rs:129](../../crates/crysirror-qlie/src/adapter/math.rs#L129) |
| `col` | col | Handled | — | [math.rs:148](../../crates/crysirror-qlie/src/adapter/math.rs#L148) |
| `cos` | cos | Handled | — | [math.rs:115](../../crates/crysirror-qlie/src/adapter/math.rs#L115) |
| `max` | reduces all supplied operands, retaining signed integer ordering rather than converting them through a float result | Handled | — | [math.rs:92](../../crates/crysirror-qlie/src/adapter/math.rs#L92) |
| `min` | reduces all supplied operands, retaining signed integer ordering rather than converting them through a float result | Handled | — | [math.rs:92](../../crates/crysirror-qlie/src/adapter/math.rs#L92) |
| `pow` | pow | Handled | — | [math.rs:129](../../crates/crysirror-qlie/src/adapter/math.rs#L129) |
| `rnd` | / rnd ->. This is the thread's MT generator, distinct from TABMP's1024-entry Delphi noise ring | Handled | — | [math.rs:71](../../crates/crysirror-qlie/src/adapter/math.rs#L71) |
| `sat` | sat | Handled | — | [math.rs:179](../../crates/crysirror-qlie/src/adapter/math.rs#L179) |
| `sin` | sin | Handled | — | [math.rs:114](../../crates/crysirror-qlie/src/adapter/math.rs#L114) |
| `sqr` | sqr | Handled | — | [math.rs:187](../../crates/crysirror-qlie/src/adapter/math.rs#L187) |
| `sqrt` | sqrt | Handled | — | [math.rs:110](../../crates/crysirror-qlie/src/adapter/math.rs#L110) |
| `tan` | tan | Handled | — | [math.rs:116](../../crates/crysirror-qlie/src/adapter/math.rs#L116) |

## Object and system methods / adapter/math / misc_method

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `efe` | efe -> (Sysutils.ExtractFileExt): text from the last '.' when it follows the last '\\' or ':'; else empty | Handled | — | [math.rs:217](../../crates/crysirror-qlie/src/adapter/math.rs#L217) |
| `gcd` | gcd | Handled | — | [math.rs:265](../../crates/crysirror-qlie/src/adapter/math.rs#L265) |
| `glpk` | glpk | Handled | — | [math.rs:271](../../crates/crysirror-qlie/src/adapter/math.rs#L271) |
| `gsf` | gsf | Handled | — | [math.rs:283](../../crates/crysirror-qlie/src/adapter/math.rs#L283) |
| `info` | info | Partial | unsupported QLIE host-information key {query} | [math.rs:227](../../crates/crysirror-qlie/src/adapter/math.rs#L227) |
| `tgt` | tgt | Handled | — | [math.rs:299](../../crates/crysirror-qlie/src/adapter/math.rs#L299) |

## Object and system methods / adapter/message / message_method

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `cc` | cc | Handled | — | [message.rs:575](../../crates/crysirror-qlie/src/adapter/message.rs#L575) |
| `?ap` | ?ap | Handled | — | [message.rs:721](../../crates/crysirror-qlie/src/adapter/message.rs#L721) |
| `?as` | ?as | Handled | — | [message.rs:717](../../crates/crysirror-qlie/src/adapter/message.rs#L717) |
| `?kwb` | ?kwb | Handled | — | [message.rs:734](../../crates/crysirror-qlie/src/adapter/message.rs#L734) |
| `?kwd` | ?kwd | Handled | — | [message.rs:761](../../crates/crysirror-qlie/src/adapter/message.rs#L761) |
| `?kwe` | ?kwe | Handled | — | [message.rs:735](../../crates/crysirror-qlie/src/adapter/message.rs#L735) |
| `?kwl` | ?kwl | Handled | — | [message.rs:740](../../crates/crysirror-qlie/src/adapter/message.rs#L740) |
| `?mc` | ?mc | Handled | — | [message.rs:783](../../crates/crysirror-qlie/src/adapter/message.rs#L783) |
| `?ns` | ?ns | Handled | — | [message.rs:744](../../crates/crysirror-qlie/src/adapter/message.rs#L744) |
| `?nsw` | ?nsw | Handled | — | [message.rs:731](../../crates/crysirror-qlie/src/adapter/message.rs#L731) |
| `?tda` | ?tda | Handled | — | [message.rs:638](../../crates/crysirror-qlie/src/adapter/message.rs#L638) |
| `?tlt` | ?tlt | Handled | — | [message.rs:761](../../crates/crysirror-qlie/src/adapter/message.rs#L761) |
| `?tmg` | ?tmg | Handled | — | [message.rs:626](../../crates/crysirror-qlie/src/adapter/message.rs#L626) |
| `?tp` | ?tp | Handled | — | [message.rs:613](../../crates/crysirror-qlie/src/adapter/message.rs#L613) |
| `?ts` | ?ts | Handled | — | [message.rs:770](../../crates/crysirror-qlie/src/adapter/message.rs#L770) |
| `ap` | ap | Handled | — | [message.rs:707](../../crates/crysirror-qlie/src/adapter/message.rs#L707) |
| `as` | as | Handled | — | [message.rs:659](../../crates/crysirror-qlie/src/adapter/message.rs#L659) |
| `kwb` | kwb | Handled | — | [message.rs:677](../../crates/crysirror-qlie/src/adapter/message.rs#L677) |
| `kwd` | kwd,v (+3752) and tlt,v (, +3749) are kept settings | Handled | — | [message.rs:749](../../crates/crysirror-qlie/src/adapter/message.rs#L749) |
| `kwe` | kwe | Handled | — | [message.rs:677](../../crates/crysirror-qlie/src/adapter/message.rs#L677) |
| `kwl` | kwl | Handled | — | [message.rs:689](../../crates/crysirror-qlie/src/adapter/message.rs#L689) |
| `mc` | mc | Handled | — | [message.rs:775](../../crates/crysirror-qlie/src/adapter/message.rs#L775) |
| `n` | n | Handled | — | [message.rs:471](../../crates/crysirror-qlie/src/adapter/message.rs#L471) |
| `ns` | ns | Handled | — | [message.rs:698](../../crates/crysirror-qlie/src/adapter/message.rs#L698) |
| `nsw` | nsw | Handled | — | [message.rs:668](../../crates/crysirror-qlie/src/adapter/message.rs#L668) |
| `o` | o | Handled | — | [message.rs:471](../../crates/crysirror-qlie/src/adapter/message.rs#L471) |
| `sk` | sk | Handled | — | [message.rs:775](../../crates/crysirror-qlie/src/adapter/message.rs#L775) |
| `t` | t | Handled | — | [message.rs:471](../../crates/crysirror-qlie/src/adapter/message.rs#L471) |
| `tlt` | kwd,v (+3752) and tlt,v (, +3749) are kept settings | Handled | — | [message.rs:749](../../crates/crysirror-qlie/src/adapter/message.rs#L749) |
| `tmg` | tmg | Handled | — | [message.rs:650](../../crates/crysirror-qlie/src/adapter/message.rs#L650) |
| `tp` | tp | Handled | — | [message.rs:603](../../crates/crysirror-qlie/src/adapter/message.rs#L603) |
| `ts` | ts | Handled | — | [message.rs:585](../../crates/crysirror-qlie/src/adapter/message.rs#L585) |

## Object and system methods / adapter/misc_native / misc_native_method

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `cfe` | cfe | Handled | — | [misc_native.rs:208](../../crates/crysirror-qlie/src/adapter/misc_native.rs#L208) |
| `?lng` | ?lng | Handled | — | [misc_native.rs:245](../../crates/crysirror-qlie/src/adapter/misc_native.rs#L245) |
| `?now` | ?now | Handled | — | [misc_native.rs:282](../../crates/crysirror-qlie/src/adapter/misc_native.rs#L282) |
| `bsno` | bsno | Handled | — | [misc_native.rs:283](../../crates/crysirror-qlie/src/adapter/misc_native.rs#L283) |
| `code` | code,text,mode,seed,key: mode 0 encodes, otherwise decodes; a zero seed is 0xC4 () | Handled | — | [misc_native.rs:266](../../crates/crysirror-qlie/src/adapter/misc_native.rs#L266) |
| `crc` | crc,text: over the UTF-16 bytes, IntToHex(v, 4) | Handled | — | [misc_native.rs:213](../../crates/crysirror-qlie/src/adapter/misc_native.rs#L213) |
| `efn` | efn | Handled | — | [misc_native.rs:200](../../crates/crysirror-qlie/src/adapter/misc_native.rs#L200) |
| `exec` | exec,file,params: ShellExecute "open". Only the client's own assets may be offered to the platform viewer; programs, UR… | Handled | — | [misc_native.rs:258](../../crates/crysirror-qlie/src/adapter/misc_native.rs#L258) |
| `exp` | exp | Handled | — | [misc_native.rs:204](../../crates/crysirror-qlie/src/adapter/misc_native.rs#L204) |
| `fid` | fid | Handled | — | [misc_native.rs:283](../../crates/crysirror-qlie/src/adapter/misc_native.rs#L283) |
| `fkey` | fkey answers only when an external key object was attached (engine+324, set by the online activation path); never here | Empty handler | Original no-op status not established | [misc_native.rs:254](../../crates/crysirror-qlie/src/adapter/misc_native.rs#L254) |
| `fsha` | fsha | Handled | — | [misc_native.rs:283](../../crates/crysirror-qlie/src/adapter/misc_native.rs#L283) |
| `gfe` | gfe | Handled | — | [misc_native.rs:283](../../crates/crysirror-qlie/src/adapter/misc_native.rs#L283) |
| `id` | id (): a fresh GUID mixed by , four 8-digit words | Handled | — | [misc_native.rs:229](../../crates/crysirror-qlie/src/adapter/misc_native.rs#L229) |
| `lng` | lng | Handled | — | [misc_native.rs:245](../../crates/crysirror-qlie/src/adapter/misc_native.rs#L245) |
| `now` | now | Handled | — | [misc_native.rs:282](../../crates/crysirror-qlie/src/adapter/misc_native.rs#L282) |
| `ptof` | ptof | Handled | — | [misc_native.rs:281](../../crates/crysirror-qlie/src/adapter/misc_native.rs#L281) |
| `tsha` | tsha,text () hashes as many bytes of the UTF-16 buffer as the string has characters, i.e. the first half of its code un… | Handled | — | [misc_native.rs:220](../../crates/crysirror-qlie/src/adapter/misc_native.rs#L220) |

## Object and system methods / adapter/native_wait / wait_statement

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `btn` | btn | Handled | — | [native_wait.rs:101](../../crates/crysirror-qlie/src/adapter/native_wait.rs#L101) |
| `msg` | msg | Handled | — | [native_wait.rs:100](../../crates/crysirror-qlie/src/adapter/native_wait.rs#L100) |
| `sndt` | sndt | Handled | — | [native_wait.rs:89](../../crates/crysirror-qlie/src/adapter/native_wait.rs#L89) |
| `time` | time | Handled | — | [native_wait.rs:74](../../crates/crysirror-qlie/src/adapter/native_wait.rs#L74) |
| `wav` | wav | Handled | — | [native_wait.rs:96](../../crates/crysirror-qlie/src/adapter/native_wait.rs#L96) |

## Object and system methods / adapter/objects / object_method

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `d` | d | Handled | — | [objects.rs:2737](../../crates/crysirror-qlie/src/adapter/objects.rs#L2737) |
| `e` | e | Handled | — | [objects.rs:2737](../../crates/crysirror-qlie/src/adapter/objects.rs#L2737) |
| `dc` | dc | Handled | — | [objects.rs:2627](../../crates/crysirror-qlie/src/adapter/objects.rs#L2627) |
| `?ck` | ?ck | Handled | Guard: matches!(kind.as_str(), "btn" \\| "sbtn") | [objects.rs:2070](../../crates/crysirror-qlie/src/adapter/objects.rs#L2070) |
| `?dc` | ?dc | Handled | — | [objects.rs:2901](../../crates/crysirror-qlie/src/adapter/objects.rs#L2901) |
| `?e` | ?e | Handled | — | [objects.rs:2686](../../crates/crysirror-qlie/src/adapter/objects.rs#L2686) |
| `?exv` | ?exv | Handled | — | [objects.rs:2832](../../crates/crysirror-qlie/src/adapter/objects.rs#L2832) |
| `?lk` | ?lk | Handled | — | [objects.rs:2082](../../crates/crysirror-qlie/src/adapter/objects.rs#L2082) |
| `?m` | ?m | Handled | — | [objects.rs:2666](../../crates/crysirror-qlie/src/adapter/objects.rs#L2666) |
| `?mi` | ?mi | Handled | — | [objects.rs:2609](../../crates/crysirror-qlie/src/adapter/objects.rs#L2609) |
| `?nt` | ?nt | Handled | Guard: kind == "snd" | [objects.rs:2389](../../crates/crysirror-qlie/src/adapter/objects.rs#L2389) |
| `?p` | ?p | Handled | Guard: kind == "snd" | [objects.rs:2715](../../crates/crysirror-qlie/src/adapter/objects.rs#L2715) |
| `?p4` | ?p4: the ten p4 pose values (x,y,z, scale x/y/z in percent, rotation x/y/z in degrees, priority), sampled from the layer | Handled | Guard: kind != "snd" | [objects.rs:2570](../../crates/crysirror-qlie/src/adapter/objects.rs#L2570) |
| `?pri` | ?pri | Handled | Guard: kind != "snd" | [objects.rs:2526](../../crates/crysirror-qlie/src/adapter/objects.rs#L2526) |
| `?sc` | ?sc | Handled | Guard: kind != "snd" | [objects.rs:2096](../../crates/crysirror-qlie/src/adapter/objects.rs#L2096) |
| `?sk` | ?sk -> : decoder stream position, rounded ms | Handled | Guard: kind == "snd" | [objects.rs:2379](../../crates/crysirror-qlie/src/adapter/objects.rs#L2379) |
| `?tt` | ?tt | Handled | Guard: kind == "snd" | [objects.rs:2389](../../crates/crysirror-qlie/src/adapter/objects.rs#L2389) |
| `?ui` | ?ui | Handled | — | [objects.rs:2652](../../crates/crysirror-qlie/src/adapter/objects.rs#L2652) |
| `?us` | ?us | Handled | — | [objects.rs:2652](../../crates/crysirror-qlie/src/adapter/objects.rs#L2652) |
| `?v` | ?v | Handled | Guard: kind == "snd" | [objects.rs:2814](../../crates/crysirror-qlie/src/adapter/objects.rs#L2814) |
| `?wh` | ?wh -> /: the image's canvas width and height | Handled | Guard: kind != "snd" | [objects.rs:2563](../../crates/crysirror-qlie/src/adapter/objects.rs#L2563) |
| `?wim` | ?wim | Handled | Guard: kind != "snd" | [objects.rs:2115](../../crates/crysirror-qlie/src/adapter/objects.rs#L2115) |
| `alg` | alg | Handled | — | [objects.rs:2497](../../crates/crysirror-qlie/src/adapter/objects.rs#L2497) |
| `alg2` | alg2 | Handled | — | [objects.rs:2497](../../crates/crysirror-qlie/src/adapter/objects.rs#L2497) |
| `anc` | anc,obj: +132 anchor object reference (type 2); any other argument type is the native type error | Handled | — | [objects.rs:2537](../../crates/crysirror-qlie/src/adapter/objects.rs#L2537) |
| `bm` | bm | Handled | — | [objects.rs:2689](../../crates/crysirror-qlie/src/adapter/objects.rs#L2689) |
| `ck` | ck | Handled | Guard: matches!(kind.as_str(), "btn" \\| "sbtn") | [objects.rs:2042](../../crates/crysirror-qlie/src/adapter/objects.rs#L2042) |
| `cm` | cm | Handled | — | [objects.rs:2968](../../crates/crysirror-qlie/src/adapter/objects.rs#L2968) |
| `col` | col | Handled | — | [objects.rs:2864](../../crates/crysirror-qlie/src/adapter/objects.rs#L2864) |
| `ds` | ds | Handled | — | [objects.rs:2912](../../crates/crysirror-qlie/src/adapter/objects.rs#L2912) |
| `exr` | exr | Handled | — | [objects.rs:2825](../../crates/crysirror-qlie/src/adapter/objects.rs#L2825) |
| `exv` | exv | Handled | — | [objects.rs:2832](../../crates/crysirror-qlie/src/adapter/objects.rs#L2832) |
| `fo` | fo: per-channel filter stages (), applied by the next playback through a prepared shared PCM chain | Handled | Guard: kind == "snd" | [objects.rs:2315](../../crates/crysirror-qlie/src/adapter/objects.rs#L2315) |
| `g` | g | Handled | — | [objects.rs:2860](../../crates/crysirror-qlie/src/adapter/objects.rs#L2860) |
| `g2` | g2 | Handled | — | [objects.rs:2506](../../crates/crysirror-qlie/src/adapter/objects.rs#L2506) |
| `hs` | hs | Handled | — | [objects.rs:2912](../../crates/crysirror-qlie/src/adapter/objects.rs#L2912) |
| `i` | i | Handled | — | [objects.rs:2127](../../crates/crysirror-qlie/src/adapter/objects.rs#L2127) |
| `in` | in: button input group at +3053 (omitted -> 255) | Handled | — | [objects.rs:2619](../../crates/crysirror-qlie/src/adapter/objects.rs#L2619) |
| `l` | l | Handled | — | [objects.rs:2144](../../crates/crysirror-qlie/src/adapter/objects.rs#L2144) |
| `li` | li | Handled | — | [objects.rs:2211](../../crates/crysirror-qlie/src/adapter/objects.rs#L2211) |
| `lk` | lk | Handled | — | [objects.rs:2795](../../crates/crysirror-qlie/src/adapter/objects.rs#L2795) |
| `ls` | ls | Handled | — | [objects.rs:2224](../../crates/crysirror-qlie/src/adapter/objects.rs#L2224) |
| `m` | m | Handled | — | [objects.rs:2758](../../crates/crysirror-qlie/src/adapter/objects.rs#L2758) |
| `mi` | mi | Handled | — | [objects.rs:2600](../../crates/crysirror-qlie/src/adapter/objects.rs#L2600) |
| `midx` | midx | Handled | — | [objects.rs:2677](../../crates/crysirror-qlie/src/adapter/objects.rs#L2677) |
| `mm` | mm | Handled | — | [objects.rs:2956](../../crates/crysirror-qlie/src/adapter/objects.rs#L2956) |
| `ost` | ost,on,start,length (): sound-synchronised timeline; ?ost returns them ( pushes the flag only when set) | Handled | Guard: kind != "snd" | [objects.rs:2512](../../crates/crysirror-qlie/src/adapter/objects.rs#L2512) |
| `out` | out | Handled | — | [objects.rs:2627](../../crates/crysirror-qlie/src/adapter/objects.rs#L2627) |
| `p` | p | Handled | Guard: kind == "snd" | [objects.rs:2295](../../crates/crysirror-qlie/src/adapter/objects.rs#L2295) |
| `p2` | p2 | Handled | Guard: kind == "snd" | [objects.rs:2295](../../crates/crysirror-qlie/src/adapter/objects.rs#L2295) |
| `p4` | p4 | Handled | — | [objects.rs:2444](../../crates/crysirror-qlie/src/adapter/objects.rs#L2444) |
| `pan` | pan | Handled | Guard: kind == "snd" | [objects.rs:2363](../../crates/crysirror-qlie/src/adapter/objects.rs#L2363) |
| `pp` | pp | Handled | — | [objects.rs:2701](../../crates/crysirror-qlie/src/adapter/objects.rs#L2701) |
| `pri` | pri/?pri: wrapper +20 update group (see render) | Handled | Guard: kind != "snd" | [objects.rs:2518](../../crates/crysirror-qlie/src/adapter/objects.rs#L2518) |
| `ps` | ps | Handled | — | [objects.rs:2634](../../crates/crysirror-qlie/src/adapter/objects.rs#L2634) |
| `s` | s | Handled | Guard: kind == "snd" | [objects.rs:2277](../../crates/crysirror-qlie/src/adapter/objects.rs#L2277) |
| `sc` | sc | Handled | Guard: kind != "snd" | [objects.rs:2086](../../crates/crysirror-qlie/src/adapter/objects.rs#L2086) |
| `sk` | sk | Handled | Guard: kind == "snd" | [objects.rs:2325](../../crates/crysirror-qlie/src/adapter/objects.rs#L2325) |
| `skp` | skp | Handled | — | [objects.rs:2787](../../crates/crysirror-qlie/src/adapter/objects.rs#L2787) |
| `sp` | sp | Handled | — | [objects.rs:2968](../../crates/crysirror-qlie/src/adapter/objects.rs#L2968) |
| `tab` | tab | Handled | — | [objects.rs:2968](../../crates/crysirror-qlie/src/adapter/objects.rs#L2968) |
| `tf` | tf | Handled | — | [objects.rs:2968](../../crates/crysirror-qlie/src/adapter/objects.rs#L2968) |
| `tt` | tt | Handled | Guard: kind == "theora" | [objects.rs:2431](../../crates/crysirror-qlie/src/adapter/objects.rs#L2431) |
| `ui` | ui | Handled | — | [objects.rs:2946](../../crates/crysirror-qlie/src/adapter/objects.rs#L2946) |
| `us` | us | Handled | — | [objects.rs:2946](../../crates/crysirror-qlie/src/adapter/objects.rs#L2946) |
| `v` | v | Handled | Guard: kind == "snd" | [objects.rs:2799](../../crates/crysirror-qlie/src/adapter/objects.rs#L2799) |
| `vtn` | vtn | Handled | — | [objects.rs:2968](../../crates/crysirror-qlie/src/adapter/objects.rs#L2968) |
| `wim` | wim | Handled | Guard: kind != "snd" | [objects.rs:2108](../../crates/crysirror-qlie/src/adapter/objects.rs#L2108) |

## Object and system methods / adapter/objects / object_method / receiver=kind == "cfg"

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `faa` | faa | Handled | — | [objects.rs:1011](../../crates/crysirror-qlie/src/adapter/objects.rs#L1011) |
| `faa2` | faa2 | Handled | — | [objects.rs:1011](../../crates/crysirror-qlie/src/adapter/objects.rs#L1011) |
| `?blt` | ?blt | Handled | — | [objects.rs:713](../../crates/crysirror-qlie/src/adapter/objects.rs#L713) |
| `?cs` | ?cs | Handled | — | [objects.rs:713](../../crates/crysirror-qlie/src/adapter/objects.rs#L713) |
| `?cv` | +4B8/+5B8/+7B8 per-voice volume, mute and the third bank | Handled | — | [objects.rs:898](../../crates/crysirror-qlie/src/adapter/objects.rs#L898) |
| `?cvi` | +4B8/+5B8/+7B8 per-voice volume, mute and the third bank | Handled | — | [objects.rs:898](../../crates/crysirror-qlie/src/adapter/objects.rs#L898) |
| `?cvm` | +4B8/+5B8/+7B8 per-voice volume, mute and the third bank | Handled | — | [objects.rs:898](../../crates/crysirror-qlie/src/adapter/objects.rs#L898) |
| `?cvv` | +4B8/+5B8/+7B8 per-voice volume, mute and the third bank | Handled | — | [objects.rs:898](../../crates/crysirror-qlie/src/adapter/objects.rs#L898) |
| `?faa` | ?faa | Handled | — | [objects.rs:1050](../../crates/crysirror-qlie/src/adapter/objects.rs#L1050) |
| `?fdm` | ?fdm | Handled | — | [objects.rs:713](../../crates/crysirror-qlie/src/adapter/objects.rs#L713) |
| `?fn` | ?fn | Handled | — | [objects.rs:1025](../../crates/crysirror-qlie/src/adapter/objects.rs#L1025) |
| `?fps` | ?fps | Handled | — | [objects.rs:713](../../crates/crysirror-qlie/src/adapter/objects.rs#L713) |
| `?fs` | ?fs | Handled | — | [objects.rs:747](../../crates/crysirror-qlie/src/adapter/objects.rs#L747) |
| `?fsc` | ?fsc | Handled | — | [objects.rs:1025](../../crates/crysirror-qlie/src/adapter/objects.rs#L1025) |
| `?fst` | ?fst | Handled | — | [objects.rs:1025](../../crates/crysirror-qlie/src/adapter/objects.rs#L1025) |
| `?gim` | ?gim | Handled | — | [objects.rs:713](../../crates/crysirror-qlie/src/adapter/objects.rs#L713) |
| `?il` | ?il | Handled | — | [objects.rs:772](../../crates/crysirror-qlie/src/adapter/objects.rs#L772) |
| `?ma` | ?ma | Handled | — | [objects.rs:806](../../crates/crysirror-qlie/src/adapter/objects.rs#L806) |
| `?ma2` | ?ma2 | Handled | — | [objects.rs:806](../../crates/crysirror-qlie/src/adapter/objects.rs#L806) |
| `?mcw` | ?mcw | Handled | — | [objects.rs:978](../../crates/crysirror-qlie/src/adapter/objects.rs#L978) |
| `?ms` | ?ms | Handled | — | [objects.rs:978](../../crates/crysirror-qlie/src/adapter/objects.rs#L978) |
| `?mtw` | ?mtw | Handled | — | [objects.rs:978](../../crates/crysirror-qlie/src/adapter/objects.rs#L978) |
| `?sb` | ?sb | Handled | — | [objects.rs:713](../../crates/crysirror-qlie/src/adapter/objects.rs#L713) |
| `?sfs` | ?sfs | Handled | — | [objects.rs:713](../../crates/crysirror-qlie/src/adapter/objects.rs#L713) |
| `?vtm` | ?vtm | Handled | — | [objects.rs:834](../../crates/crysirror-qlie/src/adapter/objects.rs#L834) |
| `?vtv` | ?vtv | Handled | — | [objects.rs:834](../../crates/crysirror-qlie/src/adapter/objects.rs#L834) |
| `cvi` | +4B8/+5B8/+7B8 per-voice volume, mute and the third bank | Handled | — | [objects.rs:898](../../crates/crysirror-qlie/src/adapter/objects.rs#L898) |
| `cvm` | +4B8/+5B8/+7B8 per-voice volume, mute and the third bank | Handled | — | [objects.rs:898](../../crates/crysirror-qlie/src/adapter/objects.rs#L898) |
| `cvv` | +4B8/+5B8/+7B8 per-voice volume, mute and the third bank | Handled | — | [objects.rs:898](../../crates/crysirror-qlie/src/adapter/objects.rs#L898) |
| `fn` | fn | Handled | — | [objects.rs:995](../../crates/crysirror-qlie/src/adapter/objects.rs#L995) |
| `fn2` | fn2 | Handled | — | [objects.rs:995](../../crates/crysirror-qlie/src/adapter/objects.rs#L995) |
| `fnl` | fnl | Handled | — | [objects.rs:1063](../../crates/crysirror-qlie/src/adapter/objects.rs#L1063) |
| `fs` | fs | Handled | — | [objects.rs:723](../../crates/crysirror-qlie/src/adapter/objects.rs#L723) |
| `fsc` | fsc | Handled | — | [objects.rs:1011](../../crates/crysirror-qlie/src/adapter/objects.rs#L1011) |
| `fsc2` | fsc2 | Handled | — | [objects.rs:1011](../../crates/crysirror-qlie/src/adapter/objects.rs#L1011) |
| `fst` | fst | Handled | — | [objects.rs:1011](../../crates/crysirror-qlie/src/adapter/objects.rs#L1011) |
| `fst2` | fst2 | Handled | — | [objects.rs:1011](../../crates/crysirror-qlie/src/adapter/objects.rs#L1011) |
| `il` | il | Handled | — | [objects.rs:761](../../crates/crysirror-qlie/src/adapter/objects.rs#L761) |
| `ma` | ma | Handled | — | [objects.rs:784](../../crates/crysirror-qlie/src/adapter/objects.rs#L784) |
| `ma2` | ma2 | Handled | — | [objects.rs:784](../../crates/crysirror-qlie/src/adapter/objects.rs#L784) |
| `mcw` | mcw | Handled | — | [objects.rs:970](../../crates/crysirror-qlie/src/adapter/objects.rs#L970) |
| `ms` | ms | Handled | — | [objects.rs:970](../../crates/crysirror-qlie/src/adapter/objects.rs#L970) |
| `mtw` | mtw | Handled | — | [objects.rs:970](../../crates/crysirror-qlie/src/adapter/objects.rs#L970) |
| `smr` | smr | Handled | — | [objects.rs:723](../../crates/crysirror-qlie/src/adapter/objects.rs#L723) |
| `vtm` | vtm | Handled | — | [objects.rs:870](../../crates/crysirror-qlie/src/adapter/objects.rs#L870) |
| `vtv` | vtv | Handled | — | [objects.rs:849](../../crates/crysirror-qlie/src/adapter/objects.rs#L849) |

## Object and system methods / adapter/objects / object_method / receiver=kind == "dat"

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `c` | c | Handled | — | [objects.rs:1424](../../crates/crysirror-qlie/src/adapter/objects.rs#L1424) |
| `d` | d | Handled | — | [objects.rs:1410](../../crates/crysirror-qlie/src/adapter/objects.rs#L1410) |
| `?g` | ?g | Handled | — | [objects.rs:1438](../../crates/crysirror-qlie/src/adapter/objects.rs#L1438) |
| `?gd` | ?gd | Handled | — | [objects.rs:1438](../../crates/crysirror-qlie/src/adapter/objects.rs#L1438) |
| `cp` | Copy a value | Handled | — | [objects.rs:1414](../../crates/crysirror-qlie/src/adapter/objects.rs#L1414) |
| `g` | g | Handled | — | [objects.rs:1438](../../crates/crysirror-qlie/src/adapter/objects.rs#L1438) |
| `gd` | gd | Handled | — | [objects.rs:1438](../../crates/crysirror-qlie/src/adapter/objects.rs#L1438) |
| `l` | l | Handled | — | [objects.rs:1405](../../crates/crysirror-qlie/src/adapter/objects.rs#L1405) |
| `s` | s | Handled | — | [objects.rs:1392](../../crates/crysirror-qlie/src/adapter/objects.rs#L1392) |

## Object and system methods / adapter/objects / object_method / receiver=kind == "inp"

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `?mge` | ?mge | Handled | — | [objects.rs:1859](../../crates/crysirror-qlie/src/adapter/objects.rs#L1859) |
| `?mgp` | ?mgp,flag (): press xy, pointer xy, drag buttons (drag-masked when flag is 1) and the eight-way gesture direction (-1 w… | Handled | — | [objects.rs:1849](../../crates/crysirror-qlie/src/adapter/objects.rs#L1849) |
| `?mp` | ?mp | Handled | — | [objects.rs:1868](../../crates/crysirror-qlie/src/adapter/objects.rs#L1868) |
| `?mw` | ?mw | Handled | — | [objects.rs:1821](../../crates/crysirror-qlie/src/adapter/objects.rs#L1821) |
| `?sc` | ?sc | Handled | — | [objects.rs:1967](../../crates/crysirror-qlie/src/adapter/objects.rs#L1967) |
| `?swp` | ?swp,masked -> (shared pointer_swipe tracker) | Handled | — | [objects.rs:1835](../../crates/crysirror-qlie/src/adapter/objects.rs#L1835) |
| `gbc` | gbc | Handled | — | [objects.rs:1872](../../crates/crysirror-qlie/src/adapter/objects.rs#L1872) |
| `gsc` | gsc | Handled | — | [objects.rs:1872](../../crates/crysirror-qlie/src/adapter/objects.rs#L1872) |
| `k2` | k2,code...: every active code (at most 16) in argument order; kl2,first,last scans a code range, where codes the host c… | Handled | — | [objects.rs:1996](../../crates/crysirror-qlie/src/adapter/objects.rs#L1996) |
| `kl2` | k2,code...: every active code (at most 16) in argument order; kl2,first,last scans a code range, where codes the host c… | Handled | — | [objects.rs:1996](../../crates/crysirror-qlie/src/adapter/objects.rs#L1996) |
| `mge` | mge,v only stores +424 for ?mge; scripts poll ?mgp | Handled | — | [objects.rs:1959](../../crates/crysirror-qlie/src/adapter/objects.rs#L1959) |
| `sc` | sc | Handled | — | [objects.rs:1967](../../crates/crysirror-qlie/src/adapter/objects.rs#L1967) |
| `swe` | swe,mask[,inertia]: stores +428/+432 (inertia defaults to 1) | Handled | — | [objects.rs:1950](../../crates/crysirror-qlie/src/adapter/objects.rs#L1950) |
| `swp` | swp,1 -> clears the swipe latch and velocity | Handled | — | [objects.rs:1828](../../crates/crysirror-qlie/src/adapter/objects.rs#L1828) |

## Object and system methods / adapter/objects / object_method / receiver=kind == "rep"

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `a` | a | Partial | unsupported QLIE replacement mode {mode} | [objects.rs:1456](../../crates/crysirror-qlie/src/adapter/objects.rs#L1456) |
| `d` | d | Handled | — | [objects.rs:1490](../../crates/crysirror-qlie/src/adapter/objects.rs#L1490) |
| `?c` | ?c | Handled | — | [objects.rs:1507](../../crates/crysirror-qlie/src/adapter/objects.rs#L1507) |
| `?g` | ?g | Handled | — | [objects.rs:1510](../../crates/crysirror-qlie/src/adapter/objects.rs#L1510) |
| `?lk` | Native exposes the enabled byte here, despite the name ?lk | Handled | — | [objects.rs:1509](../../crates/crysirror-qlie/src/adapter/objects.rs#L1509) |
| `et` | et | Handled | — | [objects.rs:1504](../../crates/crysirror-qlie/src/adapter/objects.rs#L1504) |
| `i` | i | Handled | — | [objects.rs:1497](../../crates/crysirror-qlie/src/adapter/objects.rs#L1497) |
| `lk` | lk | Handled | — | [objects.rs:1455](../../crates/crysirror-qlie/src/adapter/objects.rs#L1455) |
| `nt` | nt | Handled | — | [objects.rs:1504](../../crates/crysirror-qlie/src/adapter/objects.rs#L1504) |

## Object and system methods / adapter/objects / object_method / receiver=kind == "skey"

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `a` | a | Handled | — | [objects.rs:1572](../../crates/crysirror-qlie/src/adapter/objects.rs#L1572) |
| `d` | d,code () removes one binding | Handled | — | [objects.rs:1585](../../crates/crysirror-qlie/src/adapter/objects.rs#L1585) |
| `dc` | dc,n (+30): the sound channel of the key-click effect, clamped to the manager's channel range (64 slots here) | Handled | — | [objects.rs:1592](../../crates/crysirror-qlie/src/adapter/objects.rs#L1592) |
| `?lk` | ?lk | Handled | — | [objects.rs:1571](../../crates/crysirror-qlie/src/adapter/objects.rs#L1571) |
| `i` | i | Handled | — | [objects.rs:1566](../../crates/crysirror-qlie/src/adapter/objects.rs#L1566) |
| `lk` | lk | Handled | — | [objects.rs:1570](../../crates/crysirror-qlie/src/adapter/objects.rs#L1570) |

## Object and system methods / adapter/objects / object_method / receiver=kind == "sys"

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `?ap` | ?ap | Handled | — | [objects.rs:1663](../../crates/crysirror-qlie/src/adapter/objects.rs#L1663) |
| `?fcs` | ?fcs | Handled | — | [objects.rs:1728](../../crates/crysirror-qlie/src/adapter/objects.rs#L1728) |
| `?gs` | ?gs returns the configured game screen width/height | Handled | — | [objects.rs:1776](../../crates/crysirror-qlie/src/adapter/objects.rs#L1776) |
| `?in` | ?in | Handled | — | [objects.rs:1761](../../crates/crysirror-qlie/src/adapter/objects.rs#L1761) |
| `?lk` | ?lk | Handled | — | [objects.rs:1780](../../crates/crysirror-qlie/src/adapter/objects.rs#L1780) |
| `?lkc` | ?lkc | Handled | — | [objects.rs:1780](../../crates/crysirror-qlie/src/adapter/objects.rs#L1780) |
| `?mrd` | ?mrd | Handled | — | [objects.rs:1627](../../crates/crysirror-qlie/src/adapter/objects.rs#L1627) |
| `?mt` | ?mt | Handled | — | [objects.rs:1660](../../crates/crysirror-qlie/src/adapter/objects.rs#L1660) |
| `?n` | ?n | Handled | — | [objects.rs:1708](../../crates/crysirror-qlie/src/adapter/objects.rs#L1708) |
| `?p` | ?p | Handled | — | [objects.rs:1780](../../crates/crysirror-qlie/src/adapter/objects.rs#L1780) |
| `?skp` | skp and ?skp both answer the skip flag ( LABEL_48) | Handled | — | [objects.rs:1610](../../crates/crysirror-qlie/src/adapter/objects.rs#L1610) |
| `?sr` | ?sr | Handled | — | [objects.rs:1728](../../crates/crysirror-qlie/src/adapter/objects.rs#L1728) |
| `?tef` | ?tef | Handled | — | [objects.rs:1749](../../crates/crysirror-qlie/src/adapter/objects.rs#L1749) |
| `?ver` | ?ver returns the build string literal and 1 () | Handled | — | [objects.rs:1771](../../crates/crysirror-qlie/src/adapter/objects.rs#L1771) |
| `ap` | ap | Handled | — | [objects.rs:1657](../../crates/crysirror-qlie/src/adapter/objects.rs#L1657) |
| `dskp` | dskp | Handled | — | [objects.rs:1606](../../crates/crysirror-qlie/src/adapter/objects.rs#L1606) |
| `eve` | eve | Partial | unsupported QLIE system event {} | [objects.rs:1645](../../crates/crysirror-qlie/src/adapter/objects.rs#L1645) |
| `fcs` | sr stores a raw @@label () at system+28; fcs stores the form-close save filename at system+32 | Handled | — | [objects.rs:1724](../../crates/crysirror-qlie/src/adapter/objects.rs#L1724) |
| `in` | in/?in: the global input mask at system+364. The constructor default is ; an omitted argument | Handled | — | [objects.rs:1757](../../crates/crysirror-qlie/src/adapter/objects.rs#L1757) |
| `lk` | lk | Handled | — | [objects.rs:1719](../../crates/crysirror-qlie/src/adapter/objects.rs#L1719) |
| `lkc` | lkc | Handled | — | [objects.rs:1719](../../crates/crysirror-qlie/src/adapter/objects.rs#L1719) |
| `n` | n stores seven save-metadata strings with [...] tags removed (); n2,i,s stores one raw (i <= 6) | Handled | — | [objects.rs:1675](../../crates/crysirror-qlie/src/adapter/objects.rs#L1675) |
| `n2` | n2 | Handled | — | [objects.rs:1699](../../crates/crysirror-qlie/src/adapter/objects.rs#L1699) |
| `p` | p | Handled | — | [objects.rs:1719](../../crates/crysirror-qlie/src/adapter/objects.rs#L1719) |
| `skp` | skp and ?skp both answer the skip flag ( LABEL_48) | Handled | — | [objects.rs:1610](../../crates/crysirror-qlie/src/adapter/objects.rs#L1610) |
| `sr` | sr stores a raw @@label () at system+28; fcs stores the form-close save filename at system+32 | Handled | — | [objects.rs:1724](../../crates/crysirror-qlie/src/adapter/objects.rs#L1724) |
| `tef` | tef: thumbnail effect mode/x/y at system+412..420, applied by to the save thumbnail | Handled | — | [objects.rs:1739](../../crates/crysirror-qlie/src/adapter/objects.rs#L1739) |
| `tz` | tz | Handled | — | [objects.rs:1719](../../crates/crysirror-qlie/src/adapter/objects.rs#L1719) |

## Object and system methods / adapter/objects / object_method / receiver=kind == "timer"

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `d` | d | Handled | — | [objects.rs:1362](../../crates/crysirror-qlie/src/adapter/objects.rs#L1362) |
| `e` | e | Handled | — | [objects.rs:1359](../../crates/crysirror-qlie/src/adapter/objects.rs#L1359) |
| `?e` | ?e | Handled | — | [objects.rs:1366](../../crates/crysirror-qlie/src/adapter/objects.rs#L1366) |
| `?in` | ?in | Handled | — | [objects.rs:1379](../../crates/crysirror-qlie/src/adapter/objects.rs#L1379) |
| `?l` | ?l | Handled | — | [objects.rs:1342](../../crates/crysirror-qlie/src/adapter/objects.rs#L1342) |
| `?out` | ?out | Handled | — | [objects.rs:1385](../../crates/crysirror-qlie/src/adapter/objects.rs#L1385) |
| `?t` | ?t | Handled | — | [objects.rs:1358](../../crates/crysirror-qlie/src/adapter/objects.rs#L1358) |
| `?tt` | ?tt | Handled | — | [objects.rs:1374](../../crates/crysirror-qlie/src/adapter/objects.rs#L1374) |
| `i` | i | Handled | — | [objects.rs:1332](../../crates/crysirror-qlie/src/adapter/objects.rs#L1332) |
| `in` | in | Handled | — | [objects.rs:1375](../../crates/crysirror-qlie/src/adapter/objects.rs#L1375) |
| `l` | l | Handled | — | [objects.rs:1335](../../crates/crysirror-qlie/src/adapter/objects.rs#L1335) |
| `out` | +29 output group mask ( default 0xFF) | Handled | — | [objects.rs:1381](../../crates/crysirror-qlie/src/adapter/objects.rs#L1381) |
| `t` | t | Handled | — | [objects.rs:1353](../../crates/crysirror-qlie/src/adapter/objects.rs#L1353) |
| `tt` | tt | Handled | — | [objects.rs:1370](../../crates/crysirror-qlie/src/adapter/objects.rs#L1370) |

## Object and system methods / adapter/objects / object_method / receiver=matches!(kind.as_str(), "log" \| "llog" \| "flog")

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `a` | a | Handled | — | [objects.rs:1132](../../crates/crysirror-qlie/src/adapter/objects.rs#L1132) |
| `d` | flog d,name () deletes one named entry | Handled | Guard: kind == "flog" | [objects.rs:1223](../../crates/crysirror-qlie/src/adapter/objects.rs#L1223) |
| `cc` | cc | Handled | — | [objects.rs:1127](../../crates/crysirror-qlie/src/adapter/objects.rs#L1127) |
| `?c` | ?c | Handled | — | [objects.rs:1181](../../crates/crysirror-qlie/src/adapter/objects.rs#L1181) |
| `?g` | ?g | Handled | Guard: kind == "log" | [objects.rs:1186](../../crates/crysirror-qlie/src/adapter/objects.rs#L1186) |
| `?k` | ?k answers the raw read state, k its ==1 test | Handled | Guard: kind == "llog" | [objects.rs:1227](../../crates/crysirror-qlie/src/adapter/objects.rs#L1227) |
| `?k2` | ?k answers the raw read state, k its ==1 test | Handled | Guard: kind == "llog" | [objects.rs:1227](../../crates/crysirror-qlie/src/adapter/objects.rs#L1227) |
| `?lk` | ?lk | Handled | — | [objects.rs:1126](../../crates/crysirror-qlie/src/adapter/objects.rs#L1126) |
| `ct` | ct | Handled | Guard: kind == "log" | [objects.rs:1182](../../crates/crysirror-qlie/src/adapter/objects.rs#L1182) |
| `g` | g | Handled | Guard: kind != "log" | [objects.rs:1210](../../crates/crysirror-qlie/src/adapter/objects.rs#L1210) |
| `i` | i | Handled | — | [objects.rs:1127](../../crates/crysirror-qlie/src/adapter/objects.rs#L1127) |
| `k` | ?k answers the raw read state, k its ==1 test | Handled | Guard: kind == "llog" | [objects.rs:1227](../../crates/crysirror-qlie/src/adapter/objects.rs#L1227) |
| `k2` | ?k answers the raw read state, k its ==1 test | Handled | Guard: kind == "llog" | [objects.rs:1227](../../crates/crysirror-qlie/src/adapter/objects.rs#L1227) |
| `lk` | lk | Handled | — | [objects.rs:1125](../../crates/crysirror-qlie/src/adapter/objects.rs#L1125) |
| `s` | s | Handled | Guard: kind == "log" | [objects.rs:1160](../../crates/crysirror-qlie/src/adapter/objects.rs#L1160) |

## Object and system methods / adapter/objects / object_method / receiver=matches!(kind.as_str(), "table" \| "list")

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `a` | a | Handled | — | [objects.rs:542](../../crates/crysirror-qlie/src/adapter/objects.rs#L542) |
| `d` | d | Handled | — | [objects.rs:536](../../crates/crysirror-qlie/src/adapter/objects.rs#L536) |
| `ad` | ad | Handled | — | [objects.rs:511](../../crates/crysirror-qlie/src/adapter/objects.rs#L511) |
| `?c` | ?c | Handled | — | [objects.rs:450](../../crates/crysirror-qlie/src/adapter/objects.rs#L450) |
| `?i` | ?i | Handled | — | [objects.rs:525](../../crates/crysirror-qlie/src/adapter/objects.rs#L525) |
| `?n` | ?n | Handled | — | [objects.rs:468](../../crates/crysirror-qlie/src/adapter/objects.rs#L468) |
| `?s` | ?s | Handled | — | [objects.rs:451](../../crates/crysirror-qlie/src/adapter/objects.rs#L451) |
| `?txt` | ?txt | Handled | — | [objects.rs:498](../../crates/crysirror-qlie/src/adapter/objects.rs#L498) |
| `?v` | ?v | Handled | — | [objects.rs:417](../../crates/crysirror-qlie/src/adapter/objects.rs#L417) |
| `asn` | asn | Handled | — | [objects.rs:543](../../crates/crysirror-qlie/src/adapter/objects.rs#L543) |
| `bpos` | bpos | Handled | — | [objects.rs:558](../../crates/crysirror-qlie/src/adapter/objects.rs#L558) |
| `brep` | brep | Handled | — | [objects.rs:579](../../crates/crysirror-qlie/src/adapter/objects.rs#L579) |
| `comb` | comb | Handled | — | [objects.rs:610](../../crates/crysirror-qlie/src/adapter/objects.rs#L610) |
| `i` | i | Handled | — | [objects.rs:403](../../crates/crysirror-qlie/src/adapter/objects.rs#L403) |
| `idx` | idx | Handled | — | [objects.rs:591](../../crates/crysirror-qlie/src/adapter/objects.rs#L591) |
| `idxn` | idxn | Handled | — | [objects.rs:591](../../crates/crysirror-qlie/src/adapter/objects.rs#L591) |
| `l` | l | Handled | — | [objects.rs:407](../../crates/crysirror-qlie/src/adapter/objects.rs#L407) |
| `s` | s | Handled | — | [objects.rs:473](../../crates/crysirror-qlie/src/adapter/objects.rs#L473) |
| `sort` | sort | Handled | — | [objects.rs:554](../../crates/crysirror-qlie/src/adapter/objects.rs#L554) |
| `sw` | sw,i,j: TStrings.Exchange; an index outside the list raises the native list-index error | Handled | — | [objects.rs:438](../../crates/crysirror-qlie/src/adapter/objects.rs#L438) |
| `txt` | txt | Handled | — | [objects.rs:488](../../crates/crysirror-qlie/src/adapter/objects.rs#L488) |
| `v` | v | Handled | — | [objects.rs:433](../../crates/crysirror-qlie/src/adapter/objects.rs#L433) |

## Object and system methods / adapter/objects / object_method / receiver=matches!(kind.as_str(), "tdat" \| "mdat")

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `c` | c | Handled | — | [objects.rs:1293](../../crates/crysirror-qlie/src/adapter/objects.rs#L1293) |
| `d` | d | Handled | — | [objects.rs:1263](../../crates/crysirror-qlie/src/adapter/objects.rs#L1263) |
| `?g` | ?g | Handled | — | [objects.rs:1256](../../crates/crysirror-qlie/src/adapter/objects.rs#L1256) |
| `?gd` | ?gd | Handled | — | [objects.rs:1312](../../crates/crysirror-qlie/src/adapter/objects.rs#L1312) |
| `?lim` | ?lim | Handled | — | [objects.rs:1249](../../crates/crysirror-qlie/src/adapter/objects.rs#L1249) |
| `cp` | Copy a value | Handled | — | [objects.rs:1299](../../crates/crysirror-qlie/src/adapter/objects.rs#L1299) |
| `g` | g | Handled | — | [objects.rs:1256](../../crates/crysirror-qlie/src/adapter/objects.rs#L1256) |
| `gd` | gd | Handled | — | [objects.rs:1312](../../crates/crysirror-qlie/src/adapter/objects.rs#L1312) |
| `i` | i | Handled | — | [objects.rs:1253](../../crates/crysirror-qlie/src/adapter/objects.rs#L1253) |
| `l` | l | Handled | — | [objects.rs:1285](../../crates/crysirror-qlie/src/adapter/objects.rs#L1285) |
| `lim` | lim | Handled | — | [objects.rs:1240](../../crates/crysirror-qlie/src/adapter/objects.rs#L1240) |
| `s` | s | Handled | — | [objects.rs:1273](../../crates/crysirror-qlie/src/adapter/objects.rs#L1273) |

## Object and system methods / adapter/objects/bar / bar_method

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `a` | a | Handled | — | [bar.rs:233](../../crates/crysirror-qlie/src/adapter/objects/bar.rs#L233) |
| `d` | d | Handled | — | [bar.rs:151](../../crates/crysirror-qlie/src/adapter/objects/bar.rs#L151) |
| `e` | e | Handled | — | [bar.rs:151](../../crates/crysirror-qlie/src/adapter/objects/bar.rs#L151) |
| `dc` | dc | Handled | — | [bar.rs:344](../../crates/crysirror-qlie/src/adapter/objects/bar.rs#L344) |
| `?a` | ?a | Handled | — | [bar.rs:252](../../crates/crysirror-qlie/src/adapter/objects/bar.rs#L252) |
| `?anc` | ?anc | Handled | — | [bar.rs:319](../../crates/crysirror-qlie/src/adapter/objects/bar.rs#L319) |
| `?dc` | ?dc | Handled | — | [bar.rs:283](../../crates/crysirror-qlie/src/adapter/objects/bar.rs#L283) |
| `?e` | ?e | Handled | — | [bar.rs:156](../../crates/crysirror-qlie/src/adapter/objects/bar.rs#L156) |
| `?hit` | ?dc/?sc answer the stored sound file names, ?mm the mode | Handled | — | [bar.rs:282](../../crates/crysirror-qlie/src/adapter/objects/bar.rs#L282) |
| `?in` | ?in | Handled | — | [bar.rs:374](../../crates/crysirror-qlie/src/adapter/objects/bar.rs#L374) |
| `?l` | ?l/?l2 answer the load arguments' image names | Handled | — | [bar.rs:302](../../crates/crysirror-qlie/src/adapter/objects/bar.rs#L302) |
| `?l2` | ?l/?l2 answer the load arguments' image names | Handled | — | [bar.rs:302](../../crates/crysirror-qlie/src/adapter/objects/bar.rs#L302) |
| `?lk` | ?lk | Handled | — | [bar.rs:258](../../crates/crysirror-qlie/src/adapter/objects/bar.rs#L258) |
| `?mi` | ?mi | Handled | — | [bar.rs:374](../../crates/crysirror-qlie/src/adapter/objects/bar.rs#L374) |
| `?mm` | ?mm | Handled | — | [bar.rs:292](../../crates/crysirror-qlie/src/adapter/objects/bar.rs#L292) |
| `?mv` | ?mv | Handled | — | [bar.rs:205](../../crates/crysirror-qlie/src/adapter/objects/bar.rs#L205) |
| `?out` | ?out | Handled | — | [bar.rs:374](../../crates/crysirror-qlie/src/adapter/objects/bar.rs#L374) |
| `?p` | ?p | Handled | — | [bar.rs:228](../../crates/crysirror-qlie/src/adapter/objects/bar.rs#L228) |
| `?sc` | ?sc | Handled | — | [bar.rs:283](../../crates/crysirror-qlie/src/adapter/objects/bar.rs#L283) |
| `?ui` | ?ui | Handled | — | [bar.rs:267](../../crates/crysirror-qlie/src/adapter/objects/bar.rs#L267) |
| `?us` | ?us | Handled | — | [bar.rs:267](../../crates/crysirror-qlie/src/adapter/objects/bar.rs#L267) |
| `?v` | ?v | Handled | — | [bar.rs:201](../../crates/crysirror-qlie/src/adapter/objects/bar.rs#L201) |
| `?vi` | ?vi | Handled | — | [bar.rs:207](../../crates/crysirror-qlie/src/adapter/objects/bar.rs#L207) |
| `?vt` | ?vt | Handled | — | [bar.rs:206](../../crates/crysirror-qlie/src/adapter/objects/bar.rs#L206) |
| `?vtn` | ?vtn | Handled | — | [bar.rs:374](../../crates/crysirror-qlie/src/adapter/objects/bar.rs#L374) |
| `anc` | anc,name (+150/+154/+158) sets the anchor of the bar's images; ?anc answers it | Handled | — | [bar.rs:314](../../crates/crysirror-qlie/src/adapter/objects/bar.rs#L314) |
| `i` | i | Handled | — | [bar.rs:29](../../crates/crysirror-qlie/src/adapter/objects/bar.rs#L29) |
| `in` | in | Handled | — | [bar.rs:344](../../crates/crysirror-qlie/src/adapter/objects/bar.rs#L344) |
| `l` | l | Handled | — | [bar.rs:45](../../crates/crysirror-qlie/src/adapter/objects/bar.rs#L45) |
| `l2` | l2 | Handled | — | [bar.rs:45](../../crates/crysirror-qlie/src/adapter/objects/bar.rs#L45) |
| `lk` | lk | Handled | — | [bar.rs:253](../../crates/crysirror-qlie/src/adapter/objects/bar.rs#L253) |
| `mi` | mi | Handled | — | [bar.rs:344](../../crates/crysirror-qlie/src/adapter/objects/bar.rs#L344) |
| `mm` | mm | Handled | — | [bar.rs:344](../../crates/crysirror-qlie/src/adapter/objects/bar.rs#L344) |
| `mv` | mv | Handled | — | [bar.rs:157](../../crates/crysirror-qlie/src/adapter/objects/bar.rs#L157) |
| `out` | out | Handled | — | [bar.rs:344](../../crates/crysirror-qlie/src/adapter/objects/bar.rs#L344) |
| `p` | p | Handled | — | [bar.rs:220](../../crates/crysirror-qlie/src/adapter/objects/bar.rs#L220) |
| `sc` | sc | Handled | — | [bar.rs:344](../../crates/crysirror-qlie/src/adapter/objects/bar.rs#L344) |
| `skp` | skp,ms ( -> ) advances the bar's value transition by ms; this host completes it | Handled | — | [bar.rs:325](../../crates/crysirror-qlie/src/adapter/objects/bar.rs#L325) |
| `ui` | ui | Handled | — | [bar.rs:259](../../crates/crysirror-qlie/src/adapter/objects/bar.rs#L259) |
| `us` | us | Handled | — | [bar.rs:259](../../crates/crysirror-qlie/src/adapter/objects/bar.rs#L259) |
| `v` | v | Handled | — | [bar.rs:168](../../crates/crysirror-qlie/src/adapter/objects/bar.rs#L168) |
| `vi` | vi | Handled | — | [bar.rs:168](../../crates/crysirror-qlie/src/adapter/objects/bar.rs#L168) |
| `vt` | vt | Handled | — | [bar.rs:212](../../crates/crysirror-qlie/src/adapter/objects/bar.rs#L212) |
| `vtn` | vtn | Handled | — | [bar.rs:344](../../crates/crysirror-qlie/src/adapter/objects/bar.rs#L344) |

## Object and system methods / adapter/reg / reg_method

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `?k` | ?k | Handled | — | [reg.rs:81](../../crates/crysirror-qlie/src/adapter/reg.rs#L81) |
| `?kl` | ?kl | Handled | — | [reg.rs:119](../../crates/crysirror-qlie/src/adapter/reg.rs#L119) |
| `?v` | ?v | Handled | — | [reg.rs:108](../../crates/crysirror-qlie/src/adapter/reg.rs#L108) |
| `?vl` | ?vl | Handled | — | [reg.rs:119](../../crates/crysirror-qlie/src/adapter/reg.rs#L119) |
| `dk` | dk | Handled | — | [reg.rs:82](../../crates/crysirror-qlie/src/adapter/reg.rs#L82) |
| `dv` | dv | Handled | — | [reg.rs:102](../../crates/crysirror-qlie/src/adapter/reg.rs#L102) |
| `k` | k | Handled | — | [reg.rs:74](../../crates/crysirror-qlie/src/adapter/reg.rs#L74) |
| `vi` | vi | Handled | — | [reg.rs:89](../../crates/crysirror-qlie/src/adapter/reg.rs#L89) |
| `vs` | vs | Handled | — | [reg.rs:89](../../crates/crysirror-qlie/src/adapter/reg.rs#L89) |

## Object and system methods / adapter/sdat / sdat_method

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `d` | d | Handled | — | [sdat.rs:64](../../crates/crysirror-qlie/src/adapter/sdat.rs#L64) |
| `l` | l | Handled | — | [sdat.rs:57](../../crates/crysirror-qlie/src/adapter/sdat.rs#L57) |
| `s` | s | Handled | — | [sdat.rs:49](../../crates/crysirror-qlie/src/adapter/sdat.rs#L49) |

## Object and system methods / adapter/snd_native / snd_native_method

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `?fo` | ?fo answers the stored filter chain text | Handled | — | [snd_native.rs:179](../../crates/crysirror-qlie/src/adapter/snd_native.rs#L179) |
| `?l` | ?l () and dsnd \?l (): the loaded file name | Handled | — | [snd_native.rs:136](../../crates/crysirror-qlie/src/adapter/snd_native.rs#L136) |
| `?mnv` | ?mnv,type (): the volume-type level x256 (cfg vtv) | Handled | — | [snd_native.rs:161](../../crates/crysirror-qlie/src/adapter/snd_native.rs#L161) |
| `?nv` | ?nv (): the voice's current amplitude x256 | Handled | — | [snd_native.rs:153](../../crates/crysirror-qlie/src/adapter/snd_native.rs#L153) |
| `?pan` | ?pan (): the voice pan, -256..256 (last pan request) | Handled | — | [snd_native.rs:141](../../crates/crysirror-qlie/src/adapter/snd_native.rs#L141) |
| `?t` | ?t | Handled | — | [snd_native.rs:112](../../crates/crysirror-qlie/src/adapter/snd_native.rs#L112) |
| `?vtn` | ?vtn | Handled | — | [snd_native.rs:112](../../crates/crysirror-qlie/src/adapter/snd_native.rs#L112) |
| `\?l` | ?l () and dsnd \?l (): the loaded file name | Handled | — | [snd_native.rs:136](../../crates/crysirror-qlie/src/adapter/snd_native.rs#L136) |
| `\?p` | dsnd \?p ( -> ): DirectSound buffer status, 0 without a playing buffer, 1 playing, 2 playing and looping | Handled | — | [snd_native.rs:123](../../crates/crysirror-qlie/src/adapter/snd_native.rs#L123) |
| `pm` | pm | Handled | — | [snd_native.rs:173](../../crates/crysirror-qlie/src/adapter/snd_native.rs#L173) |
| `sv` | sv | Handled | — | [snd_native.rs:38](../../crates/crysirror-qlie/src/adapter/snd_native.rs#L38) |
| `t` | t/vtn,type (/) select the volume track; ?t/?vtn read it back | Handled | — | [snd_native.rs:107](../../crates/crysirror-qlie/src/adapter/snd_native.rs#L107) |
| `tt` | tt answers like ?tt plus an empty ResultStr (LABEL_59) | Handled | — | [snd_native.rs:184](../../crates/crysirror-qlie/src/adapter/snd_native.rs#L184) |

## Object and system methods / adapter/sys_native / sys_native_method

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `c` | c | Handled | — | [sys_native.rs:59](../../crates/crysirror-qlie/src/adapter/sys_native.rs#L59) |
| `e` | e | Handled | — | [sys_native.rs:93](../../crates/crysirror-qlie/src/adapter/sys_native.rs#L93) |
| `ee` | ee | Handled | — | [sys_native.rs:67](../../crates/crysirror-qlie/src/adapter/sys_native.rs#L67) |
| `fc` | fc stores a process-wide string (+ global) | Handled | — | [sys_native.rs:163](../../crates/crysirror-qlie/src/adapter/sys_native.rs#L163) |
| `?blt` | Presenter blit mode 0..4 (cfg BitBltMode in this host) | Handled | — | [sys_native.rs:122](../../crates/crysirror-qlie/src/adapter/sys_native.rs#L122) |
| `?ckc` | ?ckc | Handled | — | [sys_native.rs:109](../../crates/crysirror-qlie/src/adapter/sys_native.rs#L109) |
| `?eve` | ?eve | Handled | — | [sys_native.rs:193](../../crates/crysirror-qlie/src/adapter/sys_native.rs#L193) |
| `?fc` | ?fc | Handled | — | [sys_native.rs:166](../../crates/crysirror-qlie/src/adapter/sys_native.rs#L166) |
| `?hdr` | ?hdr | Handled | — | [sys_native.rs:239](../../crates/crysirror-qlie/src/adapter/sys_native.rs#L239) |
| `?mod` | ?mod | Handled | — | [sys_native.rs:110](../../crates/crysirror-qlie/src/adapter/sys_native.rs#L110) |
| `?mpd` | ?mpd (): reader position as file, 1-based line and label | Handled | — | [sys_native.rs:247](../../crates/crysirror-qlie/src/adapter/sys_native.rs#L247) |
| `?msd` | r,label,file: request 8 with "label,file" (); handled by reset_engine after the frame's input pass | Handled | — | [sys_native.rs:83](../../crates/crysirror-qlie/src/adapter/sys_native.rs#L83) |
| `?ods` | /: presentation rectangle size of the screen | Handled | — | [sys_native.rs:115](../../crates/crysirror-qlie/src/adapter/sys_native.rs#L115) |
| `?out` | ?out | Handled | — | [sys_native.rs:147](../../crates/crysirror-qlie/src/adapter/sys_native.rs#L147) |
| `?rev` | StrToIntDef("564", 0): the engine revision | Handled | — | [sys_native.rs:120](../../crates/crysirror-qlie/src/adapter/sys_native.rs#L120) |
| `?run` | ?run | Handled | — | [sys_native.rs:142](../../crates/crysirror-qlie/src/adapter/sys_native.rs#L142) |
| `?ss` | ?ss | Handled | — | [sys_native.rs:155](../../crates/crysirror-qlie/src/adapter/sys_native.rs#L155) |
| `?tc` | ?tc | Handled | — | [sys_native.rs:175](../../crates/crysirror-qlie/src/adapter/sys_native.rs#L175) |
| `?tz` | ?tz | Handled | — | [sys_native.rs:148](../../crates/crysirror-qlie/src/adapter/sys_native.rs#L148) |
| `?v` | ?v | Handled | — | [sys_native.rs:161](../../crates/crysirror-qlie/src/adapter/sys_native.rs#L161) |
| `ckc` | ckc | Handled | — | [sys_native.rs:108](../../crates/crysirror-qlie/src/adapter/sys_native.rs#L108) |
| `cr` | empties the two resource caches ( sets their capacity to 0 and back); engine-owned caches have no VM state | Empty handler | Original no-op status not established | [sys_native.rs:104](../../crates/crysirror-qlie/src/adapter/sys_native.rs#L104) |
| `eve` | eve | Handled | — | [sys_native.rs:184](../../crates/crysirror-qlie/src/adapter/sys_native.rs#L184) |
| `fm` | fm | Handled | — | [sys_native.rs:99](../../crates/crysirror-qlie/src/adapter/sys_native.rs#L99) |
| `h` | h | Handled | — | [sys_native.rs:60](../../crates/crysirror-qlie/src/adapter/sys_native.rs#L60) |
| `hdr` | hdr | Handled | — | [sys_native.rs:218](../../crates/crysirror-qlie/src/adapter/sys_native.rs#L218) |
| `l` | l | Handled | — | [sys_native.rs:61](../../crates/crysirror-qlie/src/adapter/sys_native.rs#L61) |
| `m` | m | Handled | — | [sys_native.rs:62](../../crates/crysirror-qlie/src/adapter/sys_native.rs#L62) |
| `msd` | r,label,file: request 8 with "label,file" (); handled by reset_engine after the frame's input pass | Handled | — | [sys_native.rs:83](../../crates/crysirror-qlie/src/adapter/sys_native.rs#L83) |
| `out` | out | Handled | — | [sys_native.rs:143](../../crates/crysirror-qlie/src/adapter/sys_native.rs#L143) |
| `ql` | ql | Handled | — | [sys_native.rs:65](../../crates/crysirror-qlie/src/adapter/sys_native.rs#L65) |
| `qs` | qs | Handled | — | [sys_native.rs:64](../../crates/crysirror-qlie/src/adapter/sys_native.rs#L64) |
| `r` | r | Handled | — | [sys_native.rs:84](../../crates/crysirror-qlie/src/adapter/sys_native.rs#L84) |
| `ref` | ref ( + ) forces a screen redraw; the engine presents every frame | Empty handler | Original no-op status not established | [sys_native.rs:107](../../crates/crysirror-qlie/src/adapter/sys_native.rs#L107) |
| `rr` | rr | Handled | — | [sys_native.rs:66](../../crates/crysirror-qlie/src/adapter/sys_native.rs#L66) |
| `run` | run | Handled | — | [sys_native.rs:137](../../crates/crysirror-qlie/src/adapter/sys_native.rs#L137) |
| `s` | s | Handled | — | [sys_native.rs:63](../../crates/crysirror-qlie/src/adapter/sys_native.rs#L63) |
| `srr` | srr executes "\ret,<sr label>" () on this thread | Handled | — | [sys_native.rs:207](../../crates/crysirror-qlie/src/adapter/sys_native.rs#L207) |
| `ss` | ss | Handled | — | [sys_native.rs:154](../../crates/crysirror-qlie/src/adapter/sys_native.rs#L154) |
| `tc` | tc/?tc: the main form caption (/) | Handled | — | [sys_native.rs:170](../../crates/crysirror-qlie/src/adapter/sys_native.rs#L170) |
| `v` | v | Handled | — | [sys_native.rs:160](../../crates/crysirror-qlie/src/adapter/sys_native.rs#L160) |

## Object and system methods / adapter/text_statements / text_statement

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `bspl` | bspl | Handled | — | [text_statements.rs:27](../../crates/crysirror-qlie/src/adapter/text_statements.rs#L27) |
| `btra` | btra | Handled | — | [text_statements.rs:36](../../crates/crysirror-qlie/src/adapter/text_statements.rs#L36) |
| `dbrk` | dbrk | Empty handler | Original no-op status not established | [text_statements.rs:51](../../crates/crysirror-qlie/src/adapter/text_statements.rs#L51) |
| `dmon` | dmon | Empty handler | Original no-op status not established | [text_statements.rs:51](../../crates/crysirror-qlie/src/adapter/text_statements.rs#L51) |
| `dout` | dout | Empty handler | Original no-op status not established | [text_statements.rs:51](../../crates/crysirror-qlie/src/adapter/text_statements.rs#L51) |
| `drun` | drun | Handled | — | [text_statements.rs:52](../../crates/crysirror-qlie/src/adapter/text_statements.rs#L52) |
| `fobj` | fobj | Handled | — | [text_statements.rs:62](../../crates/crysirror-qlie/src/adapter/text_statements.rs#L62) |
| `hanc` | hanc | Handled | — | [text_statements.rs:43](../../crates/crysirror-qlie/src/adapter/text_statements.rs#L43) |
| `hlen` | hlen | Handled | — | [text_statements.rs:47](../../crates/crysirror-qlie/src/adapter/text_statements.rs#L47) |

## Object and system methods / adapter/theora_native / theora_native_method

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `?bm` | ?bm | Handled | — | [theora_native.rs:45](../../crates/crysirror-qlie/src/adapter/theora_native.rs#L45) |
| `?in` | ?in | Handled | — | [theora_native.rs:46](../../crates/crysirror-qlie/src/adapter/theora_native.rs#L46) |
| `?l` | ?l | Handled | — | [theora_native.rs:53](../../crates/crysirror-qlie/src/adapter/theora_native.rs#L53) |
| `?ost` | ?ost | Handled | — | [theora_native.rs:38](../../crates/crysirror-qlie/src/adapter/theora_native.rs#L38) |
| `?out` | ?out | Handled | — | [theora_native.rs:46](../../crates/crysirror-qlie/src/adapter/theora_native.rs#L46) |
| `?pp` | ?pp | Handled | — | [theora_native.rs:38](../../crates/crysirror-qlie/src/adapter/theora_native.rs#L38) |
| `?sk` | ?sk | Handled | — | [theora_native.rs:57](../../crates/crysirror-qlie/src/adapter/theora_native.rs#L57) |
| `?ss` | ?ss | Handled | — | [theora_native.rs:38](../../crates/crysirror-qlie/src/adapter/theora_native.rs#L38) |
| `?tf` | ?tf | Handled | — | [theora_native.rs:45](../../crates/crysirror-qlie/src/adapter/theora_native.rs#L45) |
| `?v` | ?v | Handled | — | [theora_native.rs:49](../../crates/crysirror-qlie/src/adapter/theora_native.rs#L49) |
| `?vtn` | ?vtn | Handled | — | [theora_native.rs:45](../../crates/crysirror-qlie/src/adapter/theora_native.rs#L45) |
| `ost` | ost | Handled | — | [theora_native.rs:29](../../crates/crysirror-qlie/src/adapter/theora_native.rs#L29) |
| `pp` | pp | Handled | — | [theora_native.rs:29](../../crates/crysirror-qlie/src/adapter/theora_native.rs#L29) |
| `s` | s | Handled | — | [theora_native.rs:71](../../crates/crysirror-qlie/src/adapter/theora_native.rs#L71) |
| `sk` | sk | Handled | — | [theora_native.rs:64](../../crates/crysirror-qlie/src/adapter/theora_native.rs#L64) |
| `ss` | ss | Handled | — | [theora_native.rs:30](../../crates/crysirror-qlie/src/adapter/theora_native.rs#L30) |

## Object and system methods / adapter/visual / effect_current

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `c` | c | Handled | — | [visual.rs:824](../../crates/crysirror-qlie/src/adapter/visual.rs#L824) |
| `dc` | mb ( +4C) makes request a full presenter redraw each frame (); vtn (+71) selects the effect sounds' volume track. Both … | Handled | — | [visual.rs:786](../../crates/crysirror-qlie/src/adapter/visual.rs#L786) |
| `?bg` | Queries of the stored effect state (): ?v +6D, ?dc +60, ?sc, ?vtn +71, ?mb (1 when set, else 0), ?fdm mode/strength, ?b… | Handled | — | [visual.rs:798](../../crates/crysirror-qlie/src/adapter/visual.rs#L798) |
| `?c` | ?c | Handled | — | [visual.rs:757](../../crates/crysirror-qlie/src/adapter/visual.rs#L757) |
| `?dc` | Queries of the stored effect state (): ?v +6D, ?dc +60, ?sc, ?vtn +71, ?mb (1 when set, else 0), ?fdm mode/strength, ?b… | Handled | — | [visual.rs:798](../../crates/crysirror-qlie/src/adapter/visual.rs#L798) |
| `?fdm` | ?fdm | Handled | — | [visual.rs:812](../../crates/crysirror-qlie/src/adapter/visual.rs#L812) |
| `?mb` | ?mb | Handled | — | [visual.rs:807](../../crates/crysirror-qlie/src/adapter/visual.rs#L807) |
| `?sc` | Queries of the stored effect state (): ?v +6D, ?dc +60, ?sc, ?vtn +71, ?mb (1 when set, else 0), ?fdm mode/strength, ?b… | Handled | — | [visual.rs:798](../../crates/crysirror-qlie/src/adapter/visual.rs#L798) |
| `?v` | Queries of the stored effect state (): ?v +6D, ?dc +60, ?sc, ?vtn +71, ?mb (1 when set, else 0), ?fdm mode/strength, ?b… | Handled | — | [visual.rs:798](../../crates/crysirror-qlie/src/adapter/visual.rs#L798) |
| `?vtn` | Queries of the stored effect state (): ?v +6D, ?dc +60, ?sc, ?vtn +71, ?mb (1 when set, else 0), ?fdm mode/strength, ?b… | Handled | — | [visual.rs:798](../../crates/crysirror-qlie/src/adapter/visual.rs#L798) |
| `bg` | bg | Handled | — | [visual.rs:817](../../crates/crysirror-qlie/src/adapter/visual.rs#L817) |
| `fdi` | fdi | Handled | Guard: a.first().map(Value::integer).transpose()? == Some(2) | [visual.rs:855](../../crates/crysirror-qlie/src/adapter/visual.rs#L855) |
| `fdm` | fdm | Handled | — | [visual.rs:765](../../crates/crysirror-qlie/src/adapter/visual.rs#L765) |
| `fdo` | fdo | Handled | Guard: a.first().map(Value::integer).transpose()? == Some(2) | [visual.rs:855](../../crates/crysirror-qlie/src/adapter/visual.rs#L855) |
| `fdp` | mb ( +4C) makes request a full presenter redraw each frame (); vtn (+71) selects the effect sounds' volume track. Both … | Handled | — | [visual.rs:786](../../crates/crysirror-qlie/src/adapter/visual.rs#L786) |
| `i` | i | Handled | — | [visual.rs:992](../../crates/crysirror-qlie/src/adapter/visual.rs#L992) |
| `mb` | mb ( +4C) makes request a full presenter redraw each frame (); vtn (+71) selects the effect sounds' volume track. Both … | Handled | — | [visual.rs:786](../../crates/crysirror-qlie/src/adapter/visual.rs#L786) |
| `pri` | mb ( +4C) makes request a full presenter redraw each frame (); vtn (+71) selects the effect sounds' volume track. Both … | Handled | — | [visual.rs:786](../../crates/crysirror-qlie/src/adapter/visual.rs#L786) |
| `sc` | mb ( +4C) makes request a full presenter redraw each frame (); vtn (+71) selects the effect sounds' volume track. Both … | Handled | — | [visual.rs:786](../../crates/crysirror-qlie/src/adapter/visual.rs#L786) |
| `v` | mb ( +4C) makes request a full presenter redraw each frame (); vtn (+71) selects the effect sounds' volume track. Both … | Handled | — | [visual.rs:786](../../crates/crysirror-qlie/src/adapter/visual.rs#L786) |
| `vtn` | mb ( +4C) makes request a full presenter redraw each frame (); vtn (+71) selects the effect sounds' volume track. Both … | Handled | — | [visual.rs:786](../../crates/crysirror-qlie/src/adapter/visual.rs#L786) |

## Script commands

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `cfe` | cfe | Handled | — | [adapter.rs:1089](../../crates/crysirror-qlie/src/adapter.rs#L1089) |
| `blank` | blank | Handled | — | [adapter.rs:1381](../../crates/crysirror-qlie/src/adapter.rs#L1381) |
| `bmat` | bmat | Handled | — | [adapter.rs:1336](../../crates/crysirror-qlie/src/adapter.rs#L1336) |
| `bsub` | bsub | Handled | — | [adapter.rs:1106](../../crates/crysirror-qlie/src/adapter.rs#L1106) |
| `char` | char | Handled | — | [adapter.rs:1209](../../crates/crysirror-qlie/src/adapter.rs#L1209) |
| `clk` | clk | Handled | — | [adapter.rs:996](../../crates/crysirror-qlie/src/adapter.rs#L996) |
| `copy` | copy | Handled | — | [adapter.rs:1067](../../crates/crysirror-qlie/src/adapter.rs#L1067) |
| `cpy` | cpy | Handled | — | [adapter.rs:1067](../../crates/crysirror-qlie/src/adapter.rs#L1067) |
| `ddlg` | dthm and ddlg are explicit release-build no-ops | Empty handler | Original no-op status not established | [adapter.rs:1331](../../crates/crysirror-qlie/src/adapter.rs#L1331) |
| `del` | del | Handled | — | [adapter.rs:1036](../../crates/crysirror-qlie/src/adapter.rs#L1036) |
| `derr` | derr | Unsupported | QLIE script error: {} | [adapter.rs:1028](../../crates/crysirror-qlie/src/adapter.rs#L1028) |
| `div` | div | Handled | — | [adapter.rs:1280](../../crates/crysirror-qlie/src/adapter.rs#L1280) |
| `draw` | draw | Handled | — | [adapter.rs:1004](../../crates/crysirror-qlie/src/adapter.rs#L1004) |
| `dthm` | dthm and ddlg are explicit release-build no-ops | Empty handler | Original no-op status not established | [adapter.rs:1331](../../crates/crysirror-qlie/src/adapter.rs#L1331) |
| `ecpy` | ecpy | Handled | — | [adapter.rs:1192](../../crates/crysirror-qlie/src/adapter.rs#L1192) |
| `efn` | efn | Handled | — | [adapter.rs:1089](../../crates/crysirror-qlie/src/adapter.rs#L1089) |
| `elen` | elen | Handled | — | [adapter.rs:1172](../../crates/crysirror-qlie/src/adapter.rs#L1172) |
| `epos` | epos | Handled | — | [adapter.rs:1179](../../crates/crysirror-qlie/src/adapter.rs#L1179) |
| `exop` | exop | Handled | — | [adapter.rs:1377](../../crates/crysirror-qlie/src/adapter.rs#L1377) |
| `fexs` | fexs | Handled | — | [adapter.rs:1037](../../crates/crysirror-qlie/src/adapter.rs#L1037) |
| `fmt` | fmt | Handled | — | [adapter.rs:1227](../../crates/crysirror-qlie/src/adapter.rs#L1227) |
| `fsha` | fsha | Handled | — | [adapter.rs:940](../../crates/crysirror-qlie/src/adapter.rs#L940) |
| `len` | len | Handled | — | [adapter.rs:1067](../../crates/crysirror-qlie/src/adapter.rs#L1067) |
| `lexs` | lexs | Handled | — | [adapter.rs:1204](../../crates/crysirror-qlie/src/adapter.rs#L1204) |
| `lowc` | lowc | Handled | — | [adapter.rs:1160](../../crates/crysirror-qlie/src/adapter.rs#L1160) |
| `max` | max | Handled | — | [adapter.rs:1134](../../crates/crysirror-qlie/src/adapter.rs#L1134) |
| `min` | min | Handled | — | [adapter.rs:1134](../../crates/crysirror-qlie/src/adapter.rs#L1134) |
| `play` | play | Handled | — | [adapter.rs:1022](../../crates/crysirror-qlie/src/adapter.rs#L1022) |
| `pos` | pos | Handled | — | [adapter.rs:1147](../../crates/crysirror-qlie/src/adapter.rs#L1147) |
| `rep` | rep | Handled | — | [adapter.rs:1280](../../crates/crysirror-qlie/src/adapter.rs#L1280) |
| `scp` | scp | Handled | — | [adapter.rs:950](../../crates/crysirror-qlie/src/adapter.rs#L950) |
| `skip` | skip | Handled | — | [adapter.rs:974](../../crates/crysirror-qlie/src/adapter.rs#L974) |
| `srct` | srct | Handled | — | [adapter.rs:939](../../crates/crysirror-qlie/src/adapter.rs#L939) |
| `stid` | stid | Handled | — | [adapter.rs:1365](../../crates/crysirror-qlie/src/adapter.rs#L1365) |
| `stop` | stop | Handled | — | [adapter.rs:1008](../../crates/crysirror-qlie/src/adapter.rs#L1008) |
| `swap` | swap | Handled | — | [adapter.rs:1220](../../crates/crysirror-qlie/src/adapter.rs#L1220) |
| `text` | text | Handled | — | [adapter.rs:1381](../../crates/crysirror-qlie/src/adapter.rs#L1381) |
| `thrd` | thrd | Handled | — | [adapter.rs:939](../../crates/crysirror-qlie/src/adapter.rs#L939) |
| `trim` | trim | Handled | — | [adapter.rs:1216](../../crates/crysirror-qlie/src/adapter.rs#L1216) |
| `tsub` | tsub | Handled | — | [adapter.rs:939](../../crates/crysirror-qlie/src/adapter.rs#L939) |
| `upc` | upc | Handled | — | [adapter.rs:1160](../../crates/crysirror-qlie/src/adapter.rs#L1160) |
| `vget` | vget | Handled | — | [adapter.rs:941](../../crates/crysirror-qlie/src/adapter.rs#L941) |
| `vset` | vset | Handled | — | [adapter.rs:941](../../crates/crysirror-qlie/src/adapter.rs#L941) |
| `wait` | wait | Handled | — | [adapter.rs:967](../../crates/crysirror-qlie/src/adapter.rs#L967) |
| `zenc` | zenc | Handled | — | [adapter.rs:1085](../../crates/crysirror-qlie/src/adapter.rs#L1085) |
| `zero` | zero | Handled | — | [adapter.rs:1004](../../crates/crysirror-qlie/src/adapter.rs#L1004) |

## Subselectors and conditional dispatch

Patterns retain their source namespace and guards. `_` is the fallback of that table. Boolean/optional results follow the function's return contract. Call-site names identify the implementation to inspect, including indirect service calls.

### adapter.rs / QlieVm::due_timer

```rust
fn due_timer(&mut self, m: &Machine) -> Option<thread::Callback>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `o.kind != "timer" \|\| o.thread != thread` | Local state/value operations | Handled | [731](../../crates/crysirror-qlie/src/adapter.rs#L731) |

### adapter.rs / QlieVm::key_callback

```rust
fn key_callback(&self, m: &Machine) -> Result<Option<thread::Callback>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `code & 0xffff == 0x120` | Local state/value operations | Handled | [826](../../crates/crysirror-qlie/src/adapter.rs#L826) |

### adapter.rs / QlieVm::native

```rust
fn native(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        ins: &Instruction,
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under ins.op.as_str()="vget" \| "vset"` | `ins.op == "vget"` | Local state/value operations | Handled | [943](../../crates/crysirror-qlie/src/adapter.rs#L943) |
| `condition; under ins.op.as_str()="copy" \| "cpy" \| "len"` | `ins.op == "len"` | Value::Int, i32::try_from | Handled | [1070](../../crates/crysirror-qlie/src/adapter.rs#L1070) |
| `condition; under ins.op.as_str()="efn" \| "cfe"` | `ins.op == "efn"` | Local state/value operations | Handled | [1091](../../crates/crysirror-qlie/src/adapter.rs#L1091) |
| `condition; under ins.op.as_str()="min" \| "max"` | `ins.op == "min"` | Local state/value operations | Handled | [1139](../../crates/crysirror-qlie/src/adapter.rs#L1139) |
| `condition; under ins.op.as_str()="lowc" \| "upc"` | `ins.op == "lowc"` | Local state/value operations | Handled | [1165](../../crates/crysirror-qlie/src/adapter.rs#L1165) |
| `condition; under ins.op.as_str()="rep" \| "div"` | `ins.op == "rep"` | Local state/value operations | Handled | [1284](../../crates/crysirror-qlie/src/adapter.rs#L1284) |
| `ins.op.as_str()` | `op if op.starts_with('^')` | self.expand_line | Handled | [1382](../../crates/crysirror-qlie/src/adapter.rs#L1382) |
| `ins.op.as_str()` | `_` | self.selection.len, self.object_method | Handled | [1385](../../crates/crysirror-qlie/src/adapter.rs#L1385) |

### adapter/abmp_native.rs / QlieVm::abmp_native_method

```rust
fn abmp_native_method(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        id: &str,
        op: &str,
        a: &[Value],
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under op="lsk" \| "lst"` | `op == "lsk"` | Local state/value operations | Handled | [52](../../crates/crysirror-qlie/src/adapter/abmp_native.rs#L52) |
| `(slot, self.abmp_clock(id)?); under op="?lsk" \| "?lst"` | `(Some(slot), Some(clock)) if op == "?lsk"` | Local state/value operations | Handled | [63](../../crates/crysirror-qlie/src/adapter/abmp_native.rs#L63) |
| `(slot, self.abmp_clock(id)?); under op="?lsk" \| "?lst"` | `(Some(slot), Some(clock))` | Local state/value operations | Handled | [64](../../crates/crysirror-qlie/src/adapter/abmp_native.rs#L64) |
| `(slot, self.abmp_clock(id)?); under op="?lsk" \| "?lst"` | `_` | Local state/value operations | Handled | [65](../../crates/crysirror-qlie/src/adapter/abmp_native.rs#L65) |
| `condition; under op="gfs" \| "gfk"` | `op == "gfs"` | Local state/value operations | Handled | [134](../../crates/crysirror-qlie/src/adapter/abmp_native.rs#L134) |
| `condition; under op="gfs" \| "gfk"` | `op == "gfs"` | Local state/value operations | Handled | [140](../../crates/crysirror-qlie/src/adapter/abmp_native.rs#L140) |
| `condition; under op="?gfs" \| "?gfk"` | `op == "?gfs"` | i32::from | Handled | [153](../../crates/crysirror-qlie/src/adapter/abmp_native.rs#L153) |
| `condition; under op="?tmp"` | `self.objects[id].properties.contains_key("tmp")` | Value::Int | Handled | [187](../../crates/crysirror-qlie/src/adapter/abmp_native.rs#L187) |
| `op` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [261](../../crates/crysirror-qlie/src/adapter/abmp_native.rs#L261) |

### adapter/animation.rs / easing_with_strength

```rust
fn easing_with_strength(
    mode: i32,
    strength: i32,
) -> Result<crysirror_scene::layer::Easing>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under mode=15..=20` | `matches!(mode, 17 \| 18)` | Local state/value operations | Handled | [297](../../crates/crysirror-qlie/src/adapter/animation.rs#L297) |
| `mode` | `0` | Local state/value operations | Handled | [264](../../crates/crysirror-qlie/src/adapter/animation.rs#L264) |
| `mode` | `1` | Local state/value operations | Handled | [265](../../crates/crysirror-qlie/src/adapter/animation.rs#L265) |
| `mode` | `2` | Local state/value operations | Handled | [266](../../crates/crysirror-qlie/src/adapter/animation.rs#L266) |
| `mode` | `6` | Local state/value operations | Unsupported: QLIE noisy interpolation requires a retained procedural motion producer | [267](../../crates/crysirror-qlie/src/adapter/animation.rs#L267) |
| `mode` | `3` | Local state/value operations | Handled | [268](../../crates/crysirror-qlie/src/adapter/animation.rs#L268) |
| `mode` | `4` | Local state/value operations | Handled | [272](../../crates/crysirror-qlie/src/adapter/animation.rs#L272) |
| `mode` | `7` | Local state/value operations | Handled | [276](../../crates/crysirror-qlie/src/adapter/animation.rs#L276) |
| `mode` | `8` | Local state/value operations | Handled | [280](../../crates/crysirror-qlie/src/adapter/animation.rs#L280) |
| `mode` | `12` | Local state/value operations | Handled | [284](../../crates/crysirror-qlie/src/adapter/animation.rs#L284) |
| `mode` | `13` | Local state/value operations | Handled | [289](../../crates/crysirror-qlie/src/adapter/animation.rs#L289) |
| `mode` | `14` | Local state/value operations | Handled | [292](../../crates/crysirror-qlie/src/adapter/animation.rs#L292) |
| `mode` | `15..=20` | Local state/value operations | Handled | [295](../../crates/crysirror-qlie/src/adapter/animation.rs#L295) |
| `mode` | `21` | Local state/value operations | Handled | [301](../../crates/crysirror-qlie/src/adapter/animation.rs#L301) |
| `mode` | `_` | Local state/value operations | Handled | [310](../../crates/crysirror-qlie/src/adapter/animation.rs#L310) |

### adapter/animation.rs / sample

```rust
fn sample(
    track: &Track,
    time: u64,
    values: &Values,
    random: &mut animation_clock::RandomTable,
) -> Result<Option<Sample>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `mode == 0 \|\| mode == 11 \|\| !(0..=21).contains(&mode) \|\| a[0] == b[0] \|\| points.windows(2).all(\|pair\| pair[0] == pair[1])` | Local state/value operations | Handled | [427](../../crates/crysirror-qlie/src/adapter/animation.rs#L427) |
| `condition` | `let Some(curve) = match mode { 2 => Some(F::BSpline), 5 => Some(F::Hermite), 9 => Some(F::TangentBezier), 10 => Some(F::AlternatingBezier { odd: (index - 1) & 1 != 0, }), _ => None, }` | Local state/value operations | Handled | [439](../../crates/crysirror-qlie/src/adapter/animation.rs#L439) |
| `mode` | `2` | Local state/value operations | Handled | [440](../../crates/crysirror-qlie/src/adapter/animation.rs#L440) |
| `mode` | `5` | Local state/value operations | Handled | [441](../../crates/crysirror-qlie/src/adapter/animation.rs#L441) |
| `mode` | `9` | Local state/value operations | Handled | [442](../../crates/crysirror-qlie/src/adapter/animation.rs#L442) |
| `mode` | `10` | Local state/value operations | Handled | [443](../../crates/crysirror-qlie/src/adapter/animation.rs#L443) |
| `mode` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [446](../../crates/crysirror-qlie/src/adapter/animation.rs#L446) |
| `condition` | `mode == 6` | f64::from | Handled | [450](../../crates/crysirror-qlie/src/adapter/animation.rs#L450) |
| `condition` | `a[45] != 0 && a[0] != b[0]` | Local state/value operations | Handled | [496](../../crates/crysirror-qlie/src/adapter/animation.rs#L496) |
| `condition` | `b[0]>a[0]` | Local state/value operations | Handled | [539](../../crates/crysirror-qlie/src/adapter/animation.rs#L539) |

### adapter/animation/legacy.rs / finish

```rust
fn finish(tag: &str, channel: u8, frames: &mut [[i32; 115]]) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `tag == "ablayer15" && channel == 1 && frames.iter().all(\|key\| key[36] == 0 && key[37] == 0)` | Local state/value operations | Handled | [91](../../crates/crysirror-qlie/src/adapter/animation/legacy.rs#L91) |
| `condition` | `matches!(kind, 20 \| 21) \|\| generator` | Local state/value operations | Handled | [115](../../crates/crysirror-qlie/src/adapter/animation/legacy.rs#L115) |
| `condition` | `matches!(kind, 20 \| 21)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [115](../../crates/crysirror-qlie/src/adapter/animation/legacy.rs#L115) |
| `condition` | `key[94] & 4 != 0` | Local state/value operations | Handled | [127](../../crates/crysirror-qlie/src/adapter/animation/legacy.rs#L127) |
| `condition` | `key[94] & 8 != 0` | Local state/value operations | Handled | [130](../../crates/crysirror-qlie/src/adapter/animation/legacy.rs#L130) |
| `condition` | `matches!(kind, 20 \| 21)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [135](../../crates/crysirror-qlie/src/adapter/animation/legacy.rs#L135) |

### adapter/animation/legacy.rs / upgrade

```rust
fn upgrade(tag: &str, raw: &[u8]) -> Result<[i32; 115]>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(key[1] as u8, 3 \| 4 \| 7 \| 13 \| 18 \| 19)` | Local state/value operations | Handled | [77](../../crates/crysirror-qlie/src/adapter/animation/legacy.rs#L77) |

### adapter/animation_clock.rs / Clock::advance

```rust
fn advance(
        &mut self,
        bundle: &resources::Bundle,
        requested_motion: usize,
        input: u64,
        values: &animation::Values,
        flags: &[bool; 32],
        targets: &BTreeMap<u8, u8>,
        random: &mut RandomTable,
        audio_level: i32,
    ) -> Result<Frame>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under key[52]=1` | `(1..32).contains(&selector)` | i64::from | Handled | [247](../../crates/crysirror-qlie/src/adapter/animation_clock.rs#L247) |
| `key[52]; under key[52]=3 \| 9 \| 14 if entered` | `3` | Local state/value operations | Handled | [268](../../crates/crysirror-qlie/src/adapter/animation_clock.rs#L268) |
| `key[52]; under key[52]=3 \| 9 \| 14 if entered` | `9` | Local state/value operations | Handled | [269](../../crates/crysirror-qlie/src/adapter/animation_clock.rs#L269) |
| `key[52]; under key[52]=3 \| 9 \| 14 if entered` | `_` | Local state/value operations | Handled | [270](../../crates/crysirror-qlie/src/adapter/animation_clock.rs#L270) |
| `key[52]; under key[52]=6 \| 10 \| 15 if entered` | `6` | Local state/value operations | Handled | [301](../../crates/crysirror-qlie/src/adapter/animation_clock.rs#L301) |
| `key[52]; under key[52]=6 \| 10 \| 15 if entered` | `10` | Local state/value operations | Handled | [302](../../crates/crysirror-qlie/src/adapter/animation_clock.rs#L302) |
| `key[52]; under key[52]=6 \| 10 \| 15 if entered` | `_` | Local state/value operations | Handled | [303](../../crates/crysirror-qlie/src/adapter/animation_clock.rs#L303) |
| `condition; under key[52]=12 \| 13 if entered` | `key[52] == 12` | Local state/value operations | Handled | [313](../../crates/crysirror-qlie/src/adapter/animation_clock.rs#L313) |
| `key[52]` | `0 \| 2 \| 8 \| 11` | Local state/value operations | Empty handler: Original no-op status not established | [239](../../crates/crysirror-qlie/src/adapter/animation_clock.rs#L239) |
| `key[52]` | `1` | i64::from, usize::try_from | Handled | [240](../../crates/crysirror-qlie/src/adapter/animation_clock.rs#L240) |
| `key[52]` | `3 \| 9 \| 14 if entered` | Local state/value operations | Handled | [266](../../crates/crysirror-qlie/src/adapter/animation_clock.rs#L266) |
| `key[52]` | `4` | i64::from, usize::try_from | Handled | [274](../../crates/crysirror-qlie/src/adapter/animation_clock.rs#L274) |
| `key[52]` | `5 if entered` | usize::try_from, self.reset, self.frame | Handled | [288](../../crates/crysirror-qlie/src/adapter/animation_clock.rs#L288) |
| `key[52]` | `6 \| 10 \| 15 if entered` | Local state/value operations | Handled | [299](../../crates/crysirror-qlie/src/adapter/animation_clock.rs#L299) |
| `key[52]` | `7` | self.tracks.iter_mut, i64::from | Handled | [307](../../crates/crysirror-qlie/src/adapter/animation_clock.rs#L307) |
| `key[52]` | `12 \| 13 if entered` | Local state/value operations | Handled | [312](../../crates/crysirror-qlie/src/adapter/animation_clock.rs#L312) |
| `key[52]` | `3 \| 5 \| 6 \| 9 \| 10 \| 12 \| 13 \| 14 \| 15` | Local state/value operations | Empty handler: Original no-op status not established | [322](../../crates/crysirror-qlie/src/adapter/animation_clock.rs#L322) |
| `key[52]` | `event` | Local state/value operations | Unsupported: unsupported QLIE animation event {event} | [323](../../crates/crysirror-qlie/src/adapter/animation_clock.rs#L323) |

### adapter/camera.rs / QlieVm::camera_method

```rust
fn camera_method(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        id: &str,
        op: &str,
        a: &[Value],
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under op="i"` | `self.objects[id].properties.contains_key("col")` | self.submit | Handled | [47](../../crates/crysirror-qlie/src/adapter/camera.rs#L47) |
| `condition; under op="col"` | `self.objects[id].properties.contains_key("col")` | serde_json::from_slice, h.query_engine, EngineQuery::Layer | Handled | [138](../../crates/crysirror-qlie/src/adapter/camera.rs#L138) |
| `op; under op="p" \| "p2" \| "p3" if self.objects[id].kind == "cam"` | `_` | Local state/value operations | Handled | [222](../../crates/crysirror-qlie/src/adapter/camera.rs#L222) |
| `condition; under op="?mm"` | `self.objects[id].kind == "cam"` | Value::Int | Handled | [258](../../crates/crysirror-qlie/src/adapter/camera.rs#L258) |
| `op` | `"p" if self.objects[id].kind != "cam"` | animation::repeat, self.property_easing, Vec::new, self.submit | Handled | [74](../../crates/crysirror-qlie/src/adapter/camera.rs#L74) |
| `op` | `"p" \| "p2" \| "p3" if self.objects[id].kind == "cam"` | self.property_easing, self.submit, self.objects.get_mut | Handled | [221](../../crates/crysirror-qlie/src/adapter/camera.rs#L221) |
| `op` | `_` | Local state/value operations | Unsupported: unsupported QLIE camera method \\{op} | [277](../../crates/crysirror-qlie/src/adapter/camera.rs#L277) |
| `condition` | `matches!(op, "pri" \| "i")` | self.objects.values, self.submit | Handled | [279](../../crates/crysirror-qlie/src/adapter/camera.rs#L279) |
| `condition` | `matches!(op, "pri" \| "i")` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [279](../../crates/crysirror-qlie/src/adapter/camera.rs#L279) |

### adapter/capture.rs / scale_paint_alpha

```rust
fn scale_paint_alpha(mut paint: serde_json::Value, alpha: u64) -> Result<serde_json::Value>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `paint["kind"].as_str().unwrap_or("")` | `"group"` | Local state/value operations | Handled | [199](../../crates/crysirror-qlie/src/adapter/capture.rs#L199) |
| `paint["kind"].as_str().unwrap_or("")` | `"solid" \| "text" \| "glyph_run"` | Local state/value operations | Handled | [200](../../crates/crysirror-qlie/src/adapter/capture.rs#L200) |
| `paint["kind"].as_str().unwrap_or("")` | `"image" \| "projected_mesh"` | Local state/value operations | Handled | [201](../../crates/crysirror-qlie/src/adapter/capture.rs#L201) |
| `paint["kind"].as_str().unwrap_or("")` | `kind` | Local state/value operations | Unsupported: QLIE capture alpha for paint kind {kind} | [202](../../crates/crysirror-qlie/src/adapter/capture.rs#L202) |

### adapter/cfg_settings.rs / QlieVm::cfg_setting_method

```rust
fn cfg_setting_method(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        id: &str,
        op: &str,
        a: &[Value],
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `op` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [187](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L187) |
| `op` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [326](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L326) |

### adapter/cfg_settings.rs / QlieVm::game_bool

```rust
fn game_bool(&self, key: &str, default: bool) -> bool
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `self.game_settings() .into_iter() .flat_map(\|s\| s.lines()) .filter_map(\|line\| line.split_once('=')) .find(\|(k, _)\| k.trim().eq_ignore_ascii_case(key)) .map(\|(_, v)\| v.trim().to_ascii_lowercase()) .as_deref()` | `Some("true")` | Local state/value operations | Handled | [104](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L104) |
| `self.game_settings() .into_iter() .flat_map(\|s\| s.lines()) .filter_map(\|line\| line.split_once('=')) .find(\|(k, _)\| k.trim().eq_ignore_ascii_case(key)) .map(\|(_, v)\| v.trim().to_ascii_lowercase()) .as_deref()` | `Some("false")` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [105](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L105) |
| `self.game_settings() .into_iter() .flat_map(\|s\| s.lines()) .filter_map(\|line\| line.split_once('=')) .find(\|(k, _)\| k.trim().eq_ignore_ascii_case(key)) .map(\|(_, v)\| v.trim().to_ascii_lowercase()) .as_deref()` | `_` | Local state/value operations | Handled | [106](../../crates/crysirror-qlie/src/adapter/cfg_settings.rs#L106) |

### adapter/clip.rs / QlieVm::clip_method

```rust
fn clip_method(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        op: &str,
        a: &[Value],
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `op` | `_` | Local state/value operations | Unsupported: unsupported QLIE clip method \\{op} | [29](../../crates/crysirror-qlie/src/adapter/clip.rs#L29) |
| `op` | `_` | Local state/value operations | Handled | [47](../../crates/crysirror-qlie/src/adapter/clip.rs#L47) |
| `condition; under op="pl"` | `let Some(o) = self.objects.get_mut(&s(0)).filter(\|o\| o.kind == "list")` | super::objects::strings_install | Handled | [66](../../crates/crysirror-qlie/src/adapter/clip.rs#L66) |
| `op` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [70](../../crates/crysirror-qlie/src/adapter/clip.rs#L70) |

### adapter/dialogs.rs / QlieVm::dialog_method

```rust
fn dialog_method(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        op: &str,
        a: &[Value],
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `op == "sl"` | self.list_dialog | Handled | [28](../../crates/crysirror-qlie/src/adapter/dialogs.rs#L28) |
| `condition` | `op == "f"` | self.font_dialog | Handled | [31](../../crates/crysirror-qlie/src/adapter/dialogs.rs#L31) |
| `condition` | `op == "c"` | self.color_dialog_method | Handled | [34](../../crates/crysirror-qlie/src/adapter/dialogs.rs#L34) |
| `condition` | `matches!(op, "o" \| "s")` | self.file_dialog_method | Handled | [37](../../crates/crysirror-qlie/src/adapter/dialogs.rs#L37) |
| `condition` | `matches!(op, "o" \| "s")` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [37](../../crates/crysirror-qlie/src/adapter/dialogs.rs#L37) |
| `condition` | `matches!(op, "d1" \| "d2" \| "d3" \| "sd1" \| "sd2" \| "sd3")` | self.choice_dialog_method | Handled | [40](../../crates/crysirror-qlie/src/adapter/dialogs.rs#L40) |
| `condition` | `matches!(op, "d1" \| "d2" \| "d3" \| "sd1" \| "sd2" \| "sd3")` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [40](../../crates/crysirror-qlie/src/adapter/dialogs.rs#L40) |
| `condition` | `(widget == Some(d.field as u64) && event["kind"] == "submit") \|\| (widget == Some(d.button as u64) && event["kind"] == "activate")` | Local state/value operations | Handled | [79](../../crates/crysirror-qlie/src/adapter/dialogs.rs#L79) |
| `condition` | `widget == Some(d.field as u64) && event["kind"] == "cancel"` | Local state/value operations | Handled | [85](../../crates/crysirror-qlie/src/adapter/dialogs.rs#L85) |

### adapter/dialogs/choice.rs / QlieVm::choice_dialog_method

```rust
fn choice_dialog_method(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        op: &str,
        a: &[Value],
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `op` | `_` | usize::from | Handled | [68](../../crates/crysirror-qlie/src/adapter/dialogs/choice.rs#L68) |
| `condition` | `event["kind"] == "activate"` | Local state/value operations | Handled | [114](../../crates/crysirror-qlie/src/adapter/dialogs/choice.rs#L114) |

### adapter/dialogs/color.rs / QlieVm::color_dialog_method

```rust
fn color_dialog_method(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        a: &[Value],
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `event["kind"] == "activate"` | u64::from | Handled | [47](../../crates/crysirror-qlie/src/adapter/dialogs/color.rs#L47) |

### adapter/dialogs/file.rs / QlieVm::file_dialog_method

```rust
fn file_dialog_method(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        op: &str,
        a: &[Value],
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `op == "o"` | Local state/value operations | Handled | [53](../../crates/crysirror-qlie/src/adapter/dialogs/file.rs#L53) |

### adapter/dialogs/font.rs / QlieVm::font_dialog

```rust
fn font_dialog(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        a: &[Value],
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `a[1].text().is_empty()` | Local state/value operations | Handled | [83](../../crates/crysirror-qlie/src/adapter/dialogs/font.rs#L83) |
| `condition` | `id == dialog.root + 5 && kind == "activate" \|\| id == dialog.field && kind == "cancel"` | Local state/value operations | Handled | [146](../../crates/crysirror-qlie/src/adapter/dialogs/font.rs#L146) |
| `condition` | `id == dialog.button && kind == "activate" \|\| id == dialog.field && kind == "submit"` | Local state/value operations | Handled | [151](../../crates/crysirror-qlie/src/adapter/dialogs/font.rs#L151) |
| `condition` | `kind == "change" && (dialog.root + 10..=dialog.root + 13).contains(&id)` | Local state/value operations | Handled | [155](../../crates/crysirror-qlie/src/adapter/dialogs/font.rs#L155) |
| `condition` | `kind == "activate" && id == dialog.root + 14` | self.submit | Handled | [167](../../crates/crysirror-qlie/src/adapter/dialogs/font.rs#L167) |
| `condition` | `kind == "change" && (dialog.root + 20..=dialog.root + 22).contains(&id)` | serde_json::from_slice, h.query_engine, EngineQuery::UiWidget, self.submit | Handled | [171](../../crates/crysirror-qlie/src/adapter/dialogs/font.rs#L171) |
| `condition` | `kind == "activate" && matches!(id,value if value==dialog.root+6\|\|value==dialog.root+7)` | self.submit | Handled | [192](../../crates/crysirror-qlie/src/adapter/dialogs/font.rs#L192) |
| `condition; under receiver=kind == "activate" && matches!(id,value if value==dialog.root+6\|\|value==dialog.root+7)` | `id == dialog.root + 6` | Local state/value operations | Handled | [195](../../crates/crysirror-qlie/src/adapter/dialogs/font.rs#L195) |

### adapter/dialogs/list.rs / QlieVm::list_dialog

```rust
fn list_dialog(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        a: &[Value],
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `id == dialog.button && kind == "activate" \|\| kind == "cancel"` | Local state/value operations | Handled | [143](../../crates/crysirror-qlie/src/adapter/dialogs/list.rs#L143) |
| `condition` | `kind != "activate"` | Local state/value operations | Handled | [147](../../crates/crysirror-qlie/src/adapter/dialogs/list.rs#L147) |
| `condition` | `id == dialog.root + 6 \|\| id == dialog.root + 7` | self.list_dialog_read_fields, self.list_dialog_rows | Handled | [150](../../crates/crysirror-qlie/src/adapter/dialogs/list.rs#L150) |
| `condition` | `id == dialog.root + 6` | Local state/value operations | Handled | [152](../../crates/crysirror-qlie/src/adapter/dialogs/list.rs#L152) |
| `condition` | `id == dialog.root + 5 && picker.clipboard.is_none()` | self.list_dialog_read_fields, h.begin_desktop_request, self.submit | Handled | [161](../../crates/crysirror-qlie/src/adapter/dialogs/list.rs#L161) |

### adapter/files.rs / decode_list

```rust
fn decode_list(bytes: Vec<u8>, mode: i32, key: i32) -> Option<Vec<u8>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `mode == 1` | crysirror_resource::lane_feedback::decrypt | Handled | [285](../../crates/crysirror-qlie/src/adapter/files.rs#L285) |

### adapter/files.rs / encode_list

```rust
fn encode_list(text: &[u8], mode: i32, key: i32) -> Option<Vec<u8>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `mode` | `0` | Local state/value operations | Handled | [298](../../crates/crysirror-qlie/src/adapter/files.rs#L298) |
| `mode` | `1` | crysirror_resource::lane_feedback::encrypt | Handled | [299](../../crates/crysirror-qlie/src/adapter/files.rs#L299) |
| `mode` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [305](../../crates/crysirror-qlie/src/adapter/files.rs#L305) |

### adapter/files.rs / QlieVm::file_method

```rust
fn file_method(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        id: &str,
        kind: &str,
        op: &str,
        a: &[Value],
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `kind == "path"` | crate::program::canonical, Value::Int, i32::from | Partial: unsupported QLIE path method {op} | [105](../../crates/crysirror-qlie/src/adapter/files.rs#L105) |
| `op; under receiver=kind == "path"` | `_` | Local state/value operations | Unsupported: unsupported QLIE path method {op} | [142](../../crates/crysirror-qlie/src/adapter/files.rs#L142) |
| `condition; under op="g" \| "gl"` | `op == "g"` | Local state/value operations | Handled | [241](../../crates/crysirror-qlie/src/adapter/files.rs#L241) |
| `op` | `_` | Local state/value operations | Unsupported: unsupported QLIE file method {op} | [260](../../crates/crysirror-qlie/src/adapter/files.rs#L260) |

### adapter/files.rs / QlieVm::list_file_method

```rust
fn list_file_method(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        id: &str,
        op: &str,
        a: &[Value],
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `op == "lff"` | self.pool.spawn, VfsPath::new, crysirror_resource::read_all, crysirror_resource::text_stream::decode_bom_or_shift_jis | Handled | [329](../../crates/crysirror-qlie/src/adapter/files.rs#L329) |
| `condition` | `op == "lff"` | Value::Int, self.objects.get_mut, super::objects::strings_install | Handled | [368](../../crates/crysirror-qlie/src/adapter/files.rs#L368) |

### adapter/filters.rs / constant_mode

```rust
fn constant_mode(value: &str) -> Result<crysirror_image::rgb_lut::ConstantByteBlend>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `(id, value.to_ascii_lowercase().as_str())` | `(0 \| 1, _) \| (_, "" \| "pass" \| "normal")` | Local state/value operations | Handled | [15](../../crates/crysirror-qlie/src/adapter/filters.rs#L15) |
| `(id, value.to_ascii_lowercase().as_str())` | `(2, _) \| (_, "screen")` | Local state/value operations | Handled | [16](../../crates/crysirror-qlie/src/adapter/filters.rs#L16) |
| `(id, value.to_ascii_lowercase().as_str())` | `(3, _) \| (_, "multiple" \| "multiply" \| "modulate")` | Local state/value operations | Handled | [17](../../crates/crysirror-qlie/src/adapter/filters.rs#L17) |
| `(id, value.to_ascii_lowercase().as_str())` | `(4, _) \| (_, "difference")` | Local state/value operations | Handled | [18](../../crates/crysirror-qlie/src/adapter/filters.rs#L18) |
| `(id, value.to_ascii_lowercase().as_str())` | `(5, _) \| (_, "lighten")` | Local state/value operations | Handled | [19](../../crates/crysirror-qlie/src/adapter/filters.rs#L19) |
| `(id, value.to_ascii_lowercase().as_str())` | `(6, _) \| (_, "darken")` | Local state/value operations | Handled | [20](../../crates/crysirror-qlie/src/adapter/filters.rs#L20) |
| `(id, value.to_ascii_lowercase().as_str())` | `(7, _) \| (_, "add" \| "lineardodge")` | Local state/value operations | Handled | [21](../../crates/crysirror-qlie/src/adapter/filters.rs#L21) |
| `(id, value.to_ascii_lowercase().as_str())` | `(8, _) \| (_, "sub")` | Local state/value operations | Handled | [22](../../crates/crysirror-qlie/src/adapter/filters.rs#L22) |
| `(id, value.to_ascii_lowercase().as_str())` | `(9, _) \| (_, "subnot" \| "linearburn")` | Local state/value operations | Handled | [23](../../crates/crysirror-qlie/src/adapter/filters.rs#L23) |
| `(id, value.to_ascii_lowercase().as_str())` | `(10, _) \| (_, "exclusion")` | Local state/value operations | Handled | [24](../../crates/crysirror-qlie/src/adapter/filters.rs#L24) |
| `(id, value.to_ascii_lowercase().as_str())` | `(11, _) \| (_, "overlay")` | Local state/value operations | Handled | [25](../../crates/crysirror-qlie/src/adapter/filters.rs#L25) |
| `(id, value.to_ascii_lowercase().as_str())` | `(12, _) \| (_, "hardlight")` | Local state/value operations | Handled | [26](../../crates/crysirror-qlie/src/adapter/filters.rs#L26) |
| `(id, value.to_ascii_lowercase().as_str())` | `(13, _) \| (_, "linearlight")` | Local state/value operations | Handled | [27](../../crates/crysirror-qlie/src/adapter/filters.rs#L27) |
| `(id, value.to_ascii_lowercase().as_str())` | `(14, _) \| (_, "pinlight")` | Local state/value operations | Handled | [28](../../crates/crysirror-qlie/src/adapter/filters.rs#L28) |
| `(id, value.to_ascii_lowercase().as_str())` | `_` | Local state/value operations | Unsupported: unsupported QLIE constant blend {value} | [29](../../crates/crysirror-qlie/src/adapter/filters.rs#L29) |

### adapter/filters.rs / luminance_map

```rust
fn luminance_map(a: &[String]) -> Result<serde_json::Value>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `a[0].as_str()` | `"binalize"` | f64::from | Handled | [49](../../crates/crysirror-qlie/src/adapter/filters.rs#L49) |
| `a[0].as_str()` | `"gmap2"` | Local state/value operations | Handled | [58](../../crates/crysirror-qlie/src/adapter/filters.rs#L58) |
| `a[0].as_str()` | `"gmap3"` | Local state/value operations | Handled | [67](../../crates/crysirror-qlie/src/adapter/filters.rs#L67) |
| `a[0].as_str()` | `_` | Local state/value operations | Unsupported: QLIE unknown colour-map method | [84](../../crates/crysirror-qlie/src/adapter/filters.rs#L84) |

### adapter/filters.rs / QlieVm::filter_steps

```rust
fn filter_steps(&self, source: &str) -> Result<Vec<Step>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under a[0].as_str()="zoom"\|"zoom2"` | `a[0]=="zoom"` | Local state/value operations | Handled | [265](../../crates/crysirror-qlie/src/adapter/filters.rs#L265) |
| `condition; under a[0].as_str()="range" \| "toneline"` | `a[0]=="range"` | Local state/value operations | Handled | [286](../../crates/crysirror-qlie/src/adapter/filters.rs#L286) |
| `a[0].as_str()` | `"blend"` | Step::Effects, serde_json::to_vec, std::mem::take, crate::program::canonical, self.config.assets.get | Handled | [179](../../crates/crysirror-qlie/src/adapter/filters.rs#L179) |
| `a[0].as_str()` | `"goverlay"` | Local state/value operations | Handled | [208](../../crates/crysirror-qlie/src/adapter/filters.rs#L208) |
| `a[0].as_str()` | `"blurmask"` | Local state/value operations | Handled | [214](../../crates/crysirror-qlie/src/adapter/filters.rs#L214) |
| `a[0].as_str()` | `"distwave"` | f64::from, self.random.as_ref, Vec::new | Partial: unsupported QLIE distwave waveform {value} / unsupported QLIE distwave edge {value} | [227](../../crates/crysirror-qlie/src/adapter/filters.rs#L227) |
| `a[0].as_str()` | `"resize"` | crate::expression::parse_integer | Handled | [244](../../crates/crysirror-qlie/src/adapter/filters.rs#L244) |
| `a[0].as_str()` | `"clipmask"` | Local state/value operations | Handled | [255](../../crates/crysirror-qlie/src/adapter/filters.rs#L255) |
| `a[0].as_str()` | `"zoom"\|"zoom2"` | f64::from | Handled | [262](../../crates/crysirror-qlie/src/adapter/filters.rs#L262) |
| `a[0].as_str()` | `"stretch"` | Local state/value operations | Handled | [269](../../crates/crysirror-qlie/src/adapter/filters.rs#L269) |
| `a[0].as_str()` | `"sphere"` | Local state/value operations | Handled | [275](../../crates/crysirror-qlie/src/adapter/filters.rs#L275) |
| `a[0].as_str()` | `"binalize"\|"gmap2"\|"gmap3"` | Local state/value operations | Handled | [276](../../crates/crysirror-qlie/src/adapter/filters.rs#L276) |
| `a[0].as_str()` | `"gamma"` | crysirror_image::rgb_lut::RgbLookup::gamma_byte_ramp, f64::from | Handled | [277](../../crates/crysirror-qlie/src/adapter/filters.rs#L277) |
| `a[0].as_str()` | `"bright"` | crysirror_image::rgb_lut::RgbLookup::brightness_contrast_byte250, f64::from | Handled | [281](../../crates/crysirror-qlie/src/adapter/filters.rs#L281) |
| `a[0].as_str()` | `"range" \| "toneline"` | crysirror_image::rgb_lut::RgbLookup::linear_byte_ramp | Handled | [285](../../crates/crysirror-qlie/src/adapter/filters.rs#L285) |
| `a[0].as_str()` | `"color"` | crysirror_image::rgb_lut::RgbLookup::constant_byte256 | Handled | [298](../../crates/crysirror-qlie/src/adapter/filters.rs#L298) |
| `a[0].as_str()` | `"chroma"` | f64::from | Handled | [313](../../crates/crysirror-qlie/src/adapter/filters.rs#L313) |
| `a[0].as_str()` | `"huechroma"` | f64::from | Handled | [321](../../crates/crysirror-qlie/src/adapter/filters.rs#L321) |
| `a[0].as_str()` | `"blur"` | Local state/value operations | Handled | [327](../../crates/crysirror-qlie/src/adapter/filters.rs#L327) |
| `a[0].as_str()` | `"colorfill"` | crysirror_image::rgb_lut::RgbLookup::constant_byte256 | Handled | [338](../../crates/crysirror-qlie/src/adapter/filters.rs#L338) |
| `a[0].as_str()` | `"flip"` | Local state/value operations | Handled | [356](../../crates/crysirror-qlie/src/adapter/filters.rs#L356) |
| `a[0].as_str()` | `"mirror"` | Local state/value operations | Handled | [358](../../crates/crysirror-qlie/src/adapter/filters.rs#L358) |
| `a[0].as_str()` | `"mosaic" \| "mosic"` | Local state/value operations | Handled | [360](../../crates/crysirror-qlie/src/adapter/filters.rs#L360) |
| `a[0].as_str()` | `"canvas"` | Local state/value operations | Handled | [367](../../crates/crysirror-qlie/src/adapter/filters.rs#L367) |
| `a[0].as_str()` | `_` | Local state/value operations | Unsupported: unsupported QLIE resource filter {command} | [368](../../crates/crysirror-qlie/src/adapter/filters.rs#L368) |

### adapter/generator.rs / QlieVm::generator_method

```rust
fn generator_method(
        &mut self,
        h: &mut dyn HostServices,
        id: &str,
        op: &str,
        a: &[Value],
    ) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `op == "g"` | h.now_ms | Handled | [126](../../crates/crysirror-qlie/src/adapter/generator.rs#L126) |
| `condition` | `!reset && (source.is_empty() \|\| self.objects[id].generator.is_some()) && self.objects[id] .properties .get("generator_source") .and_then(\|v\| v.first()) .map(Value::text) .as_deref() == Some(source.as_str())` | Local state/value operations | Handled | [162](../../crates/crysirror-qlie/src/adapter/generator.rs#L162) |

### adapter/generator.rs / QlieVm::update_generators

```rust
fn update_generators(&mut self, h: &mut dyn HostServices) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(o.kind.as_str(), "cam" \| "camex")` | Local state/value operations | Handled | [346](../../crates/crysirror-qlie/src/adapter/generator.rs#L346) |

### adapter/generator/string.rs / parse

```rust
fn parse(source: &str, seed: u32, now: u64) -> Result<BTreeMap<u8, Generator>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `name.as_str()` | `"none"` | Local state/value operations | Handled | [33](../../crates/crysirror-qlie/src/adapter/generator/string.rs#L33) |
| `name.as_str()` | `"random"` | Local state/value operations | Handled | [34](../../crates/crysirror-qlie/src/adapter/generator/string.rs#L34) |
| `name.as_str()` | `"absrandom"` | Local state/value operations | Handled | [35](../../crates/crysirror-qlie/src/adapter/generator/string.rs#L35) |
| `name.as_str()` | `"square"` | Local state/value operations | Handled | [36](../../crates/crysirror-qlie/src/adapter/generator/string.rs#L36) |
| `name.as_str()` | `"abssquare"` | Local state/value operations | Handled | [37](../../crates/crysirror-qlie/src/adapter/generator/string.rs#L37) |
| `name.as_str()` | `"triangle"` | Local state/value operations | Handled | [38](../../crates/crysirror-qlie/src/adapter/generator/string.rs#L38) |
| `name.as_str()` | `"abstriangle"` | Local state/value operations | Handled | [39](../../crates/crysirror-qlie/src/adapter/generator/string.rs#L39) |
| `name.as_str()` | `"sin"` | Local state/value operations | Handled | [40](../../crates/crysirror-qlie/src/adapter/generator/string.rs#L40) |
| `name.as_str()` | `"abssin"` | Local state/value operations | Handled | [41](../../crates/crysirror-qlie/src/adapter/generator/string.rs#L41) |
| `name.as_str()` | `"randomlinear"` | Local state/value operations | Handled | [42](../../crates/crysirror-qlie/src/adapter/generator/string.rs#L42) |
| `name.as_str()` | `"absrandomlinear"` | Local state/value operations | Handled | [43](../../crates/crysirror-qlie/src/adapter/generator/string.rs#L43) |
| `name.as_str()` | `"randombezier"` | Local state/value operations | Handled | [44](../../crates/crysirror-qlie/src/adapter/generator/string.rs#L44) |
| `name.as_str()` | `"absrandombezier"` | Local state/value operations | Handled | [45](../../crates/crysirror-qlie/src/adapter/generator/string.rs#L45) |
| `name.as_str()` | `_` | Local state/value operations | Unsupported: unsupported QLIE generator wave {name} | [46](../../crates/crysirror-qlie/src/adapter/generator/string.rs#L46) |

### adapter/gesture_guide.rs / QlieVm::gesture_guide_method

```rust
fn gesture_guide_method(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        op: &str,
        a: &[Value],
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `op` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [96](../../crates/crysirror-qlie/src/adapter/gesture_guide.rs#L96) |

### adapter/glyph_pictures.rs / QlieVm::glyph_picture_method

```rust
fn glyph_picture_method(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        id: &str,
        op: &str,
        a: &[Value],
    ) -> Result<Option<bool>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!matches!(op, "glpi" \| "glpo" \| "glps" \| "glpf" \| "glpa" \| "gt")` | Local state/value operations | Handled | [235](../../crates/crysirror-qlie/src/adapter/glyph_pictures.rs#L235) |
| `condition` | `matches!(op, "glpi" \| "glpo" \| "glps" \| "glpf" \| "glpa" \| "gt")` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [235](../../crates/crysirror-qlie/src/adapter/glyph_pictures.rs#L235) |
| `op` | `_` | Local state/value operations | Handled | [347](../../crates/crysirror-qlie/src/adapter/glyph_pictures.rs#L347) |

### adapter/image_native.rs / QlieVm::image_native_method

```rust
fn image_native_method(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        id: &str,
        op: &str,
        a: &[Value],
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under op="p2" \| "p3"` | `op == "p2"` | Local state/value operations | Handled | [22](../../crates/crysirror-qlie/src/adapter/image_native.rs#L22) |
| `condition; under op="p2" \| "p3"` | `op == "p3"` | Local state/value operations | Handled | [25](../../crates/crysirror-qlie/src/adapter/image_native.rs#L25) |
| `condition; under op="?p2" \| "?p3"` | `op == "?p3"` | Local state/value operations | Handled | [62](../../crates/crysirror-qlie/src/adapter/image_native.rs#L62) |
| `op; under op="?in" \| "?out" \| "?ps" \| "?sp" \| "?tf" \| "?tab" \| "?vtn" \| "?alr"` | `_` | Local state/value operations | Handled | [131](../../crates/crysirror-qlie/src/adapter/image_native.rs#L131) |
| `condition; under op="bc"` | `o.kind == "sbtn"` | self.button_callbacks.push_back | Handled | [164](../../crates/crysirror-qlie/src/adapter/image_native.rs#L164) |
| `op` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [169](../../crates/crysirror-qlie/src/adapter/image_native.rs#L169) |

### adapter/input_device.rs / QlieVm::input_device_method

```rust
fn input_device_method(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        id: &str,
        op: &str,
        a: &[Value],
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under op="?bsv" \| "?swe"` | `op == "?swe"` | Local state/value operations | Handled | [139](../../crates/crysirror-qlie/src/adapter/input_device.rs#L139) |
| `op` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [156](../../crates/crysirror-qlie/src/adapter/input_device.rs#L156) |

### adapter/input_device.rs / QlieVm::synthetic_key

```rust
fn synthetic_key(&mut self, code: i32) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `code & bit != 0` | self.synthetic_key | Handled | [169](../../crates/crysirror-qlie/src/adapter/input_device.rs#L169) |
| `condition` | `matches!(key, 0xF0 \| 0xF1)` | Local state/value operations | Handled | [176](../../crates/crysirror-qlie/src/adapter/input_device.rs#L176) |
| `code & 0xF0000` | `0x00000` | Local state/value operations | Handled | [181](../../crates/crysirror-qlie/src/adapter/input_device.rs#L181) |
| `code & 0xF0000` | `0x10000` | Local state/value operations | Handled | [185](../../crates/crysirror-qlie/src/adapter/input_device.rs#L185) |
| `code & 0xF0000` | `0x20000 \| 0x30000 \| 0x40000` | Local state/value operations | Handled | [186](../../crates/crysirror-qlie/src/adapter/input_device.rs#L186) |
| `code & 0xF0000` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [190](../../crates/crysirror-qlie/src/adapter/input_device.rs#L190) |

### adapter/keys.rs / gesture_bit

```rust
fn gesture_bit(key: i32) -> Option<u64>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `key` | `0x100..=0x117` | Local state/value operations | Handled | [38](../../crates/crysirror-qlie/src/adapter/keys.rs#L38) |
| `key` | `0x120` | Local state/value operations | Handled | [39](../../crates/crysirror-qlie/src/adapter/keys.rs#L39) |
| `key` | `0x121` | Local state/value operations | Handled | [40](../../crates/crysirror-qlie/src/adapter/keys.rs#L40) |
| `key` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [41](../../crates/crysirror-qlie/src/adapter/keys.rs#L41) |

### adapter/keys.rs / QlieVm::key_without_drag

```rust
fn key_without_drag(&self, code: i32, edge: Edge) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `self.keys.gesture_mask & bit != 0 && range.contains(&key)` | self.key_edge, i32::from | Handled | [131](../../crates/crysirror-qlie/src/adapter/keys.rs#L131) |
| `condition` | `matches!(key, 1 \| 2 \| 4)` | self.key_edge, self.swipe.dragging, self.swipe.latched | Handled | [135](../../crates/crysirror-qlie/src/adapter/keys.rs#L135) |

### adapter/keys.rs / QlieVm::native_key_active

```rust
fn native_key_active(&self, code: i32) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `code & 0xF0000` | `0x00000 \| 0x80000` | self.key_edge | Handled | [146](../../crates/crysirror-qlie/src/adapter/keys.rs#L146) |
| `code & 0xF0000` | `0x10000 \| 0x90000` | self.key_without_drag | Handled | [147](../../crates/crysirror-qlie/src/adapter/keys.rs#L147) |
| `code & 0xF0000` | `0x20000 \| 0xA0000` | self.key_edge | Handled | [148](../../crates/crysirror-qlie/src/adapter/keys.rs#L148) |
| `code & 0xF0000` | `0x30000 \| 0xB0000` | self.key_edge | Handled | [151](../../crates/crysirror-qlie/src/adapter/keys.rs#L151) |
| `code & 0xF0000` | `0x40000` | self.key_edge | Handled | [152](../../crates/crysirror-qlie/src/adapter/keys.rs#L152) |
| `code & 0xF0000` | `0x50000` | self.key_edge | Handled | [153](../../crates/crysirror-qlie/src/adapter/keys.rs#L153) |
| `code & 0xF0000` | `0xC0000` | self.key_without_drag | Handled | [154](../../crates/crysirror-qlie/src/adapter/keys.rs#L154) |
| `code & 0xF0000` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [155](../../crates/crysirror-qlie/src/adapter/keys.rs#L155) |

### adapter/math.rs / QlieVm::math_method

```rust
fn math_method(
        &mut self,
        m: &mut Machine,
        op: &str,
        args: &[Value],
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under op="min" \| "max"` | `op == "min"` | Local state/value operations | Handled | [94](../../crates/crysirror-qlie/src/adapter/math.rs#L94) |
| `op; under op="sqrt" \| "sin" \| "cos" \| "tan"` | `_` | Local state/value operations | Handled | [117](../../crates/crysirror-qlie/src/adapter/math.rs#L117) |
| `condition; under op="pow" \| "atan"` | `op == "pow"` | f64::from | Handled | [134](../../crates/crysirror-qlie/src/adapter/math.rs#L134) |
| `condition; under op="abs" \| "sqr"` | `op == "abs"` | Local state/value operations | Handled | [195](../../crates/crysirror-qlie/src/adapter/math.rs#L195) |
| `op` | `_` | Local state/value operations | Unsupported: unsupported QLIE math method \\{op} | [203](../../crates/crysirror-qlie/src/adapter/math.rs#L203) |

### adapter/math.rs / QlieVm::misc_method

```rust
fn misc_method(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        op: &str,
        args: &[Value],
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under op="gsf"` | `matches!(selector, 0x1000 \| 0x1001)` | Local state/value operations | Handled | [292](../../crates/crysirror-qlie/src/adapter/math.rs#L292) |
| `condition; under op="gsf"` | `matches!(selector, 0x1000 \| 0x1001)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [292](../../crates/crysirror-qlie/src/adapter/math.rs#L292) |
| `op` | `_` | Local state/value operations | Unsupported: unsupported QLIE miscellaneous method \\{op} | [303](../../crates/crysirror-qlie/src/adapter/math.rs#L303) |

### adapter/message.rs / QlieVm::message_method

```rust
fn message_method(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        id: &str,
        ins: &Instruction,
        a: &[Value],
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under op="o" \| "t" \| "n"` | `op == "n"` | Local state/value operations | Handled | [472](../../crates/crysirror-qlie/src/adapter/message.rs#L472) |
| `condition; under op="o" \| "t" \| "n"` | `op == "t"` | Local state/value operations | Handled | [474](../../crates/crysirror-qlie/src/adapter/message.rs#L474) |
| `condition; under op="kwb" \| "kwe"` | `op == "kwb"` | Local state/value operations | Handled | [683](../../crates/crysirror-qlie/src/adapter/message.rs#L683) |
| `op; under op="?kwb" \| "?kwe" \| "?nsw"` | `_` | Local state/value operations | Handled | [736](../../crates/crysirror-qlie/src/adapter/message.rs#L736) |
| `condition; under op="kwd" \| "tlt"` | `op == "kwd"` | i32::from | Handled | [750](../../crates/crysirror-qlie/src/adapter/message.rs#L750) |
| `op` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [799](../../crates/crysirror-qlie/src/adapter/message.rs#L799) |

### adapter/minor_kinds.rs / QlieVm::minor_kind_method

```rust
fn minor_kind_method(
        &mut self,
        h: &mut dyn HostServices,
        id: &str,
        kind: &str,
        m: &mut Machine,
        op: &str,
        a: &[Value],
    ) -> Result<Option<bool>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under (kind, op)=("ppngpool", "d")` | `name.to_ascii_lowercase().ends_with(".png")` | self.resource_candidates, self.config.assets.contains_key, self.png_pool.insert | Handled | [38](../../crates/crysirror-qlie/src/adapter/minor_kinds.rs#L38) |
| `condition; under (kind, op)=("twitter", "s" \| "url")` | `op == "url"` | Local state/value operations | Handled | [58](../../crates/crysirror-qlie/src/adapter/minor_kinds.rs#L58) |
| `(kind, op)` | `("ext", "s")` | self.global_strings.insert | Handled | [27](../../crates/crysirror-qlie/src/adapter/minor_kinds.rs#L27) |
| `(kind, op)` | `("ext", "i")` | Local state/value operations | Empty handler: Original no-op status not established | [30](../../crates/crysirror-qlie/src/adapter/minor_kinds.rs#L30) |
| `(kind, op)` | `("ppngpool", "d")` | self.resource_candidates, self.config.assets.contains_key, self.png_pool.insert | Handled | [35](../../crates/crysirror-qlie/src/adapter/minor_kinds.rs#L35) |
| `(kind, op)` | `("ppngpool", "a")` | self.png_pool.remove | Handled | [47](../../crates/crysirror-qlie/src/adapter/minor_kinds.rs#L47) |
| `(kind, op)` | `("ppngpool", "cc")` | self.png_pool.clear | Handled | [50](../../crates/crysirror-qlie/src/adapter/minor_kinds.rs#L50) |
| `(kind, op)` | `("twitter", "s" \| "url")` | self.objects.get_mut | Handled | [55](../../crates/crysirror-qlie/src/adapter/minor_kinds.rs#L55) |
| `(kind, op)` | `("twitter", "?g")` | Value::Int | Handled | [63](../../crates/crysirror-qlie/src/adapter/minor_kinds.rs#L63) |
| `(kind, op)` | `("twitter", "i")` | self.objects.get_mut | Handled | [75](../../crates/crysirror-qlie/src/adapter/minor_kinds.rs#L75) |
| `(kind, op)` | `("twitter", "auth" \| "post")` | h.append_log | Handled | [80](../../crates/crysirror-qlie/src/adapter/minor_kinds.rs#L80) |
| `(kind, op)` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [85](../../crates/crysirror-qlie/src/adapter/minor_kinds.rs#L85) |

### adapter/misc_native.rs / QlieVm::file_query

```rust
fn file_query(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        op: &str,
        a: &[Value],
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `op == "bsno" \|\| (op == "fsha" && name.eq_ignore_ascii_case("self"))` | VfsPath::new, crate::program::canonical, h.storage, self.pool.spawn, crysirror_resource::checksum::sha256 | Handled | [337](../../crates/crysirror-qlie/src/adapter/misc_native.rs#L337) |

### adapter/misc_native.rs / QlieVm::misc_native_method

```rust
fn misc_native_method(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        op: &str,
        a: &[Value],
    ) -> Result<Option<bool>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `a[2].integer()?; under op="code"` | `0` | Local state/value operations | Handled | [270](../../crates/crysirror-qlie/src/adapter/misc_native.rs#L270) |
| `a[2].integer()?; under op="code"` | `v` | Local state/value operations | Handled | [271](../../crates/crysirror-qlie/src/adapter/misc_native.rs#L271) |
| `condition; under op="code"` | `mode == 0` | Local state/value operations | Handled | [274](../../crates/crysirror-qlie/src/adapter/misc_native.rs#L274) |
| `op` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [284](../../crates/crysirror-qlie/src/adapter/misc_native.rs#L284) |

### adapter/native_wait.rs / QlieVm::frame_wait_method

```rust
fn frame_wait_method(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        op: &str,
        a: &[Value],
    ) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `op == "?msd"` | Value::Int, i32::from | Handled | [308](../../crates/crysirror-qlie/src/adapter/native_wait.rs#L308) |

### adapter/native_wait.rs / QlieVm::wait_of

```rust
fn wait_of(&self, record: &Record, now: u64, m: &mut Machine) -> Result<Option<Wait>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `record.mode` | `0` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [246](../../crates/crysirror-qlie/src/adapter/native_wait.rs#L246) |
| `record.mode` | `1` | Value::Int | Handled | [247](../../crates/crysirror-qlie/src/adapter/native_wait.rs#L247) |
| `record.mode` | `2` | Wait::Stop | Handled | [251](../../crates/crysirror-qlie/src/adapter/native_wait.rs#L251) |
| `record.mode` | `3` | Wait::FixedClock | Handled | [258](../../crates/crysirror-qlie/src/adapter/native_wait.rs#L258) |
| `record.mode` | `4` | Wait::Clock | Handled | [259](../../crates/crysirror-qlie/src/adapter/native_wait.rs#L259) |
| `record.mode` | `5` | self.clock_slot | Handled | [260](../../crates/crysirror-qlie/src/adapter/native_wait.rs#L260) |
| `record.mode` | `6` | Local state/value operations | Handled | [264](../../crates/crysirror-qlie/src/adapter/native_wait.rs#L264) |
| `record.mode` | `7 \| 8` | Local state/value operations | Handled | [265](../../crates/crysirror-qlie/src/adapter/native_wait.rs#L265) |
| `record.mode` | `9` | Local state/value operations | Handled | [266](../../crates/crysirror-qlie/src/adapter/native_wait.rs#L266) |
| `record.mode` | `10` | Local state/value operations | Handled | [267](../../crates/crysirror-qlie/src/adapter/native_wait.rs#L267) |
| `record.mode` | `mode` | Local state/value operations | Unsupported: QLIE msd wait mode {mode} is not a native mode | [268](../../crates/crysirror-qlie/src/adapter/native_wait.rs#L268) |

### adapter/native_wait.rs / QlieVm::wait_statement

```rust
fn wait_statement(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        op: &str,
        a: &[String],
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `op` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [107](../../crates/crysirror-qlie/src/adapter/native_wait.rs#L107) |

### adapter/objects.rs / log_plain_text

```rust
fn log_plain_text(source: &str) -> String
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `(name, parts.len())` | `("lim", 3)` | crate::expression::parse_integer | Handled | [35](../../crates/crysirror-qlie/src/adapter/objects.rs#L35) |
| `(name, parts.len())` | `("pc" \| "pr", 2) \| ("rb", 3)` | Local state/value operations | Handled | [39](../../crates/crysirror-qlie/src/adapter/objects.rs#L39) |
| `(name, parts.len())` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [40](../../crates/crysirror-qlie/src/adapter/objects.rs#L40) |

### adapter/objects.rs / native_key

```rust
fn native_key(code: i32) -> Result<u64>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `code & 0xffff` | `0xf0` | Local state/value operations | Handled | [82](../../crates/crysirror-qlie/src/adapter/objects.rs#L82) |
| `code & 0xffff` | `0xf1` | Local state/value operations | Handled | [83](../../crates/crysirror-qlie/src/adapter/objects.rs#L83) |
| `code & 0xffff` | `value` | B::windows_virtual_key | Partial: unsupported QLIE input key {code:#x} | [84](../../crates/crysirror-qlie/src/adapter/objects.rs#L84) |

### adapter/objects.rs / set_name_value

```rust
fn set_name_value(o: &mut Object, name: &str, value: &str) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Some(line) = o.lines.iter_mut().find(\|l\| { l.split_once('=') .is_some_and(\|p\| p.0.eq_ignore_ascii_case(name)) })` | Local state/value operations | Handled | [69](../../crates/crysirror-qlie/src/adapter/objects.rs#L69) |

### adapter/objects.rs / QlieVm::free_object

```rust
fn free_object(&mut self, h: &mut dyn HostServices, o: Object) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `o.kind == "bar"` | self.objects.remove, self.free_object | Handled | [252](../../crates/crysirror-qlie/src/adapter/objects.rs#L252) |
| `condition` | `matches!(o.kind.as_str(), "cam" \| "camex") && o.properties.contains_key("col")` | self.submit | Handled | [266](../../crates/crysirror-qlie/src/adapter/objects.rs#L266) |
| `condition` | `o.kind == "snd" && o.channel != 0` | h.submit_audio | Handled | [278](../../crates/crysirror-qlie/src/adapter/objects.rs#L278) |

### adapter/objects.rs / QlieVm::object

```rust
fn object(&mut self, m: &Machine) -> Result<String>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(kind.as_str(), "rep" \| "path")` | Self::builtin | Handled | [338](../../crates/crysirror-qlie/src/adapter/objects.rs#L338) |
| `condition` | `id.starts_with("system/")` | Local state/value operations | Handled | [350](../../crates/crysirror-qlie/src/adapter/objects.rs#L350) |
| `condition` | `id.starts_with("object/")` | Local state/value operations | Handled | [352](../../crates/crysirror-qlie/src/adapter/objects.rs#L352) |

### adapter/objects.rs / QlieVm::object_method

```rust
fn object_method(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        ins: &Instruction,
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `self.objects[&id].kind.as_str()` | `"dsnd"` | Local state/value operations | Handled | [376](../../crates/crysirror-qlie/src/adapter/objects.rs#L376) |
| `self.objects[&id].kind.as_str()` | `"csr"` | Local state/value operations | Handled | [377](../../crates/crysirror-qlie/src/adapter/objects.rs#L377) |
| `self.objects[&id].kind.as_str()` | `"scsr"` | Local state/value operations | Handled | [378](../../crates/crysirror-qlie/src/adapter/objects.rs#L378) |
| `self.objects[&id].kind.as_str()` | `other` | Local state/value operations | Handled | [379](../../crates/crysirror-qlie/src/adapter/objects.rs#L379) |
| `condition` | `matches!(kind.as_str(), "file" \| "path")` | self.file_method | Handled | [394](../../crates/crysirror-qlie/src/adapter/objects.rs#L394) |
| `condition` | `matches!(kind.as_str(), "table" \| "list") && matches!(op, "lff" \| "stf")` | self.list_file_method | Handled | [397](../../crates/crysirror-qlie/src/adapter/objects.rs#L397) |
| `condition` | `matches!(kind.as_str(), "table" \| "list")` | self.objects.get_mut, self.resolve, crate::program::canonical, strings::install_text, Value::Int | Partial: unsupported QLIE {kind} method \\{op} | [400](../../crates/crysirror-qlie/src/adapter/objects.rs#L400) |
| `condition; under receiver=matches!(kind.as_str(), "table" \| "list") / op="?s"` | `kind == "table"` | Local state/value operations | Handled | [455](../../crates/crysirror-qlie/src/adapter/objects.rs#L455) |
| `condition; under receiver=matches!(kind.as_str(), "table" \| "list") / op="s"` | `kind == "table"` | Local state/value operations | Handled | [477](../../crates/crysirror-qlie/src/adapter/objects.rs#L477) |
| `condition; under receiver=matches!(kind.as_str(), "table" \| "list") / op="idx" \| "idxn"` | `op == "idx"` | Local state/value operations | Handled | [598](../../crates/crysirror-qlie/src/adapter/objects.rs#L598) |
| `op; under receiver=matches!(kind.as_str(), "table" \| "list")` | `_` | Local state/value operations | Unsupported: unsupported QLIE {kind} method \\{op} | [622](../../crates/crysirror-qlie/src/adapter/objects.rs#L622) |
| `condition` | `matches!(kind.as_str(), "ext" \| "ppngpool" \| "twitter")` | self.minor_kind_method | Unsupported: unsupported QLIE {kind} method \\{op} | [626](../../crates/crysirror-qlie/src/adapter/objects.rs#L626) |
| `condition` | `kind == "reg"` | self.reg_method | Handled | [632](../../crates/crysirror-qlie/src/adapter/objects.rs#L632) |
| `condition` | `kind == "clip"` | self.clip_method | Handled | [635](../../crates/crysirror-qlie/src/adapter/objects.rs#L635) |
| `condition` | `kind == "sdat"` | self.sdat_method | Handled | [638](../../crates/crysirror-qlie/src/adapter/objects.rs#L638) |
| `condition` | `kind == "dlgs"` | self.dialog_method | Handled | [641](../../crates/crysirror-qlie/src/adapter/objects.rs#L641) |
| `condition` | `kind == "cfg"` | self.config_settings_map, self.objects.get_mut, crate::expression::parse_integer, super::cfg_settings::load, self.cfg_setting_method | Partial: unsupported QLIE cfg method \\{op} | [644](../../crates/crysirror-qlie/src/adapter/objects.rs#L644) |
| `condition; under receiver=kind == "cfg"` | `!self.objects[&id] .properties .contains_key("native/config_loaded")` | self.config_settings_map, self.objects.get_mut, crate::expression::parse_integer, super::cfg_settings::load | Handled | [645](../../crates/crysirror-qlie/src/adapter/objects.rs#L645) |
| `condition; under receiver=kind == "cfg"` | `let Some(name) = settings.get(&format!("font[{index}].name"))` | Local state/value operations | Handled | [701](../../crates/crysirror-qlie/src/adapter/objects.rs#L701) |
| `condition; under receiver=kind == "cfg" / op="fs" \| "smr"` | `op == "fs"` | Local state/value operations | Handled | [726](../../crates/crysirror-qlie/src/adapter/objects.rs#L726) |
| `condition; under receiver=kind == "cfg" / op="ma" \| "ma2"` | `op == "ma2"` | Local state/value operations | Handled | [788](../../crates/crysirror-qlie/src/adapter/objects.rs#L788) |
| `condition; under receiver=kind == "cfg" / op="?ma" \| "?ma2"` | `op == "?ma2"` | Value::Int | Handled | [807](../../crates/crysirror-qlie/src/adapter/objects.rs#L807) |
| `condition; under receiver=kind == "cfg" / op="?vtv" \| "?vtm"` | `op == "?vtm"` | i32::from | Handled | [840](../../crates/crysirror-qlie/src/adapter/objects.rs#L840) |
| `condition; under receiver=kind == "cfg" / op="?cv" \| "?cvv" \| "?cvm" \| "?cvi" \| "cvv" \| "cvm" \| "cvi"` | `let Some(index) = key .trim() .to_ascii_lowercase() .strip_prefix("act[") .and_then(\|s\| s.strip_suffix(']')) .and_then(\|s\| s.parse::<usize>().ok()) .filter(\|i\| *i < 64)` | Local state/value operations | Handled | [912](../../crates/crysirror-qlie/src/adapter/objects.rs#L912) |
| `condition; under receiver=kind == "cfg" / op="?cv" \| "?cvv" \| "?cvm" \| "?cvi" \| "cvv" \| "cvm" \| "cvi"` | `op == "?cv"` | Value::Int | Handled | [936](../../crates/crysirror-qlie/src/adapter/objects.rs#L936) |
| `condition; under receiver=kind == "cfg" / op="?cv" \| "?cvv" \| "?cvm" \| "?cvi" \| "cvv" \| "cvm" \| "cvi"` | `op == "?cvv"` | Local state/value operations | Handled | [949](../../crates/crysirror-qlie/src/adapter/objects.rs#L949) |
| `key; under receiver=kind == "cfg" / op="?ms" \| "?mtw" \| "?mcw"` | `"ms"` | Local state/value operations | Handled | [981](../../crates/crysirror-qlie/src/adapter/objects.rs#L981) |
| `key; under receiver=kind == "cfg" / op="?ms" \| "?mtw" \| "?mcw"` | `"mtw"` | Local state/value operations | Handled | [982](../../crates/crysirror-qlie/src/adapter/objects.rs#L982) |
| `key; under receiver=kind == "cfg" / op="?ms" \| "?mtw" \| "?mcw"` | `_` | Local state/value operations | Handled | [983](../../crates/crysirror-qlie/src/adapter/objects.rs#L983) |
| `condition; under receiver=kind == "cfg" / op="?fn" \| "?faa" \| "?fst" \| "?fsc"` | `op == "?fn"` | Local state/value operations | Handled | [1041](../../crates/crysirror-qlie/src/adapter/objects.rs#L1041) |
| `op; under receiver=kind == "cfg" / op="?fn" \| "?faa" \| "?fst" \| "?fsc"` | `_` | Local state/value operations | Handled | [1051](../../crates/crysirror-qlie/src/adapter/objects.rs#L1051) |
| `condition; under receiver=kind == "cfg" / op="fnl"` | `matches!( name.to_ascii_lowercase().as_str(), "system" \| "system_fix" \| "system_pro" \| "system_ext" )` | Local state/value operations | Handled | [1092](../../crates/crysirror-qlie/src/adapter/objects.rs#L1092) |
| `op; under receiver=kind == "cfg"` | `_` | Local state/value operations | Unsupported: unsupported QLIE cfg method \\{op} | [1115](../../crates/crysirror-qlie/src/adapter/objects.rs#L1115) |
| `condition` | `matches!(kind.as_str(), "log" \| "llog" \| "flog")` | self.objects.get_mut, native_log::method, Value::Int, i32::from, Vec::with_capacity | Partial: unsupported QLIE native {kind} method \\{op} | [1119](../../crates/crysirror-qlie/src/adapter/objects.rs#L1119) |
| `condition; under receiver=matches!(kind.as_str(), "log" \| "llog" \| "flog")` | `kind == "log" && native_log::method(o, m, op, &a)?` | Local state/value operations | Handled | [1121](../../crates/crysirror-qlie/src/adapter/objects.rs#L1121) |
| `condition; under receiver=matches!(kind.as_str(), "log" \| "llog" \| "flog") / op="a"` | `kind == "log"` | Vec::with_capacity, Value::Text, Value::Int, serde_json::to_string | Handled | [1134](../../crates/crysirror-qlie/src/adapter/objects.rs#L1134) |
| `op; under receiver=matches!(kind.as_str(), "log" \| "llog" \| "flog")` | `"s" if kind == "log"` | usize::try_from, Vec::with_capacity, Value::Text, Value::Int, serde_json::to_string | Handled | [1160](../../crates/crysirror-qlie/src/adapter/objects.rs#L1160) |
| `op; under receiver=matches!(kind.as_str(), "log" \| "llog" \| "flog")` | `"ct" if kind == "log"` | Local state/value operations | Handled | [1182](../../crates/crysirror-qlie/src/adapter/objects.rs#L1182) |
| `op; under receiver=matches!(kind.as_str(), "log" \| "llog" \| "flog")` | `"?g" if kind == "log"` | usize::try_from, Value::Int | Handled | [1186](../../crates/crysirror-qlie/src/adapter/objects.rs#L1186) |
| `op; under receiver=matches!(kind.as_str(), "log" \| "llog" \| "flog")` | `"?g" \| "g" if kind != "log"` | Value::Int, crate::expression::parse_integer | Handled | [1210](../../crates/crysirror-qlie/src/adapter/objects.rs#L1210) |
| `op; under receiver=matches!(kind.as_str(), "log" \| "llog" \| "flog")` | `"s" if kind != "log"` | Local state/value operations | Handled | [1219](../../crates/crysirror-qlie/src/adapter/objects.rs#L1219) |
| `op; under receiver=matches!(kind.as_str(), "log" \| "llog" \| "flog")` | `"d" if kind == "flog"` | Local state/value operations | Handled | [1223](../../crates/crysirror-qlie/src/adapter/objects.rs#L1223) |
| `op; under receiver=matches!(kind.as_str(), "log" \| "llog" \| "flog")` | `"?k" \| "?k2" \| "k2" \| "k" if kind == "llog"` | Value::Int, i32::from | Handled | [1227](../../crates/crysirror-qlie/src/adapter/objects.rs#L1227) |
| `op; under receiver=matches!(kind.as_str(), "log" \| "llog" \| "flog")` | `_` | Local state/value operations | Unsupported: unsupported QLIE native {kind} method \\{op} | [1230](../../crates/crysirror-qlie/src/adapter/objects.rs#L1230) |
| `condition` | `matches!(kind.as_str(), "tdat" \| "mdat")` | crate::program::canonical, self.stacks.entry, self.stacks.get, Value::Int, self.save_reader | Partial: unsupported QLIE native checkpoint method \\{op} | [1234](../../crates/crysirror-qlie/src/adapter/objects.rs#L1234) |
| `op; under receiver=matches!(kind.as_str(), "tdat" \| "mdat")` | `_` | Local state/value operations | Unsupported: unsupported QLIE native checkpoint method \\{op} | [1317](../../crates/crysirror-qlie/src/adapter/objects.rs#L1317) |
| `condition` | `kind == "timer"` | self.objects.get_mut, Value::Int, i32::from | Partial: unsupported QLIE timer method \\{op} | [1321](../../crates/crysirror-qlie/src/adapter/objects.rs#L1321) |
| `op; under receiver=kind == "timer"` | `_` | Local state/value operations | Unsupported: unsupported QLIE timer method \\{op} | [1386](../../crates/crysirror-qlie/src/adapter/objects.rs#L1386) |
| `condition` | `kind == "dat"` | crate::program::canonical, self.save_reader, save_metadata::SaveTarget::File, self.load_reader, self.mutate_save | Partial: unsupported QLIE original-save method \\{op} | [1390](../../crates/crysirror-qlie/src/adapter/objects.rs#L1390) |
| `condition; under receiver=kind == "dat" / op="g" \| "?g" \| "gd" \| "?gd"` | `matches!(op, "gd" \| "?gd")` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [1442](../../crates/crysirror-qlie/src/adapter/objects.rs#L1442) |
| `op; under receiver=kind == "dat"` | `_` | Local state/value operations | Unsupported: unsupported QLIE original-save method \\{op} | [1444](../../crates/crysirror-qlie/src/adapter/objects.rs#L1444) |
| `condition` | `kind == "rep"` | self.rewrite_line, self.objects.get_mut, regex::escape, regex::RegexBuilder::new, Value::Int | Partial: unsupported QLIE replacement mode {mode} / unsupported QLIE replacement method \\{op} | [1447](../../crates/crysirror-qlie/src/adapter/objects.rs#L1447) |
| `condition; under receiver=kind == "rep"` | `op == "?r"` | self.rewrite_line | Handled | [1448](../../crates/crysirror-qlie/src/adapter/objects.rs#L1448) |
| `condition; under receiver=kind == "rep" / op="a"` | `mode == 0` | regex::escape | Handled | [1467](../../crates/crysirror-qlie/src/adapter/objects.rs#L1467) |
| `op; under receiver=kind == "rep"` | `_` | Local state/value operations | Unsupported: unsupported QLIE replacement method \\{op} | [1532](../../crates/crysirror-qlie/src/adapter/objects.rs#L1532) |
| `condition` | `kind == "math"` | self.math_method | Handled | [1536](../../crates/crysirror-qlie/src/adapter/objects.rs#L1536) |
| `condition` | `kind == "misc"` | self.misc_native_method | Handled | [1539](../../crates/crysirror-qlie/src/adapter/objects.rs#L1539) |
| `condition` | `kind == "misc" && op != "prm"` | self.misc_method | Handled | [1544](../../crates/crysirror-qlie/src/adapter/objects.rs#L1544) |
| `condition` | `kind == "misc" && op == "prm"` | Value::Int, self.config.parameters.len, self.config.parameters.get | Handled | [1547](../../crates/crysirror-qlie/src/adapter/objects.rs#L1547) |
| `condition` | `kind == "skey"` | self.gesture_guide_method, self.objects.get_mut, Value::Int, i32::from | Partial: unsupported QLIE system key method \\{op} | [1560](../../crates/crysirror-qlie/src/adapter/objects.rs#L1560) |
| `op; under receiver=kind == "skey"` | `_` | Local state/value operations | Unsupported: unsupported QLIE system key method \\{op} | [1596](../../crates/crysirror-qlie/src/adapter/objects.rs#L1596) |
| `condition` | `kind == "sys"` | self.sys_native_method, self.objects.get_mut, Value::Int, i32::from, h.now_ms | Partial: unsupported QLIE system event {} / unsupported QLIE system method \\{op} | [1600](../../crates/crysirror-qlie/src/adapter/objects.rs#L1600) |
| `op; under receiver=kind == "sys"` | `_` | Local state/value operations | Unsupported: unsupported QLIE system method \\{op} | [1790](../../crates/crysirror-qlie/src/adapter/objects.rs#L1790) |
| `condition` | `matches!(kind.as_str(), "cam" \| "camex")` | self.camera_method | Handled | [1794](../../crates/crysirror-qlie/src/adapter/objects.rs#L1794) |
| `condition` | `matches!(kind.as_str(), "msg" \| "btn" \| "sbtn")` | self.glyph_picture_method | Handled | [1797](../../crates/crysirror-qlie/src/adapter/objects.rs#L1797) |
| `condition` | `matches!(kind.as_str(), "msg" \| "btn" \| "sbtn") && self.message_method(h, m, &id, ins, &a)?` | Local state/value operations | Handled | [1802](../../crates/crysirror-qlie/src/adapter/objects.rs#L1802) |
| `condition` | `kind == "bar"` | self.bar_method | Handled | [1807](../../crates/crysirror-qlie/src/adapter/objects.rs#L1807) |
| `condition` | `kind == "eff"` | self.effect | Handled | [1810](../../crates/crysirror-qlie/src/adapter/objects.rs#L1810) |
| `condition` | `kind == "inp"` | self.input_device_method, i32::from, Value::Int, self.swipe.reset_latch, self.swipe.report | Partial: unsupported QLIE input method \\{op} | [1813](../../crates/crysirror-qlie/src/adapter/objects.rs#L1813) |
| `condition; under receiver=kind == "inp" / op="gbc" \| "gsc"` | `op == "gbc"` | h.query_engine, EngineQuery::UiSubtreeEvents, serde_json::from_slice | Handled | [1876](../../crates/crysirror-qlie/src/adapter/objects.rs#L1876) |
| `condition; under receiver=kind == "inp" / op="gbc" \| "gsc"` | `events.iter().any(\|e\| e["kind"] == "activate")` | Local state/value operations | Handled | [1893](../../crates/crysirror-qlie/src/adapter/objects.rs#L1893) |
| `condition; under receiver=kind == "inp" / op="gbc" \| "gsc"` | `op == "gbc"` | Local state/value operations | Handled | [1921](../../crates/crysirror-qlie/src/adapter/objects.rs#L1921) |
| `condition; under receiver=kind == "inp" / op="sc" \| "?sc"` | `let Some(keys) = self .objects .get(&Self::builtin("skey", self.objects[&id].thread)) .filter(\|o\| !o.locked)` | self.native_key_active | Handled | [1971](../../crates/crysirror-qlie/src/adapter/objects.rs#L1971) |
| `condition; under receiver=kind == "inp" / op="k2" \| "kl2"` | `op == "k2"` | Local state/value operations | Handled | [1997](../../crates/crysirror-qlie/src/adapter/objects.rs#L1997) |
| `condition; under receiver=kind == "inp" / op="k2" \| "kl2"` | `op == "kl2"` | self.native_key_active | Handled | [2005](../../crates/crysirror-qlie/src/adapter/objects.rs#L2005) |
| `op; under receiver=kind == "inp"` | `_` | Local state/value operations | Unsupported: unsupported QLIE input method \\{op} | [2021](../../crates/crysirror-qlie/src/adapter/objects.rs#L2021) |
| `condition` | `matches!(kind.as_str(), "img" \| "btn" \| "msg" \| "simg" \| "sbtn") && (self.abmp_native_method(h, m, &id, op, &a)? \|\| self.image_native_method(h, m, &id, op, &a)?)` | Local state/value operations | Handled | [2025](../../crates/crysirror-qlie/src/adapter/objects.rs#L2025) |
| `condition` | `kind == "theora" && self.theora_native_method(h, m, &id, op, &a)?` | Local state/value operations | Handled | [2031](../../crates/crysirror-qlie/src/adapter/objects.rs#L2031) |
| `condition` | `kind == "snd" && self.snd_native_method(h, m, &id, op, &a)?` | Local state/value operations | Handled | [2034](../../crates/crysirror-qlie/src/adapter/objects.rs#L2034) |
| `condition` | `matches!( kind.as_str(), "img" \| "btn" \| "msg" \| "simg" \| "sbtn" \| "theora" \| "snd" )` | self.objects.get_mut, h.now_ms, Value::Int, i32::from, u32::try_from | Partial: unsupported QLIE {kind} method \\{op} | [2037](../../crates/crysirror-qlie/src/adapter/objects.rs#L2037) |
| `condition; under op="l"` | `kind == "snd"` | h.submit_audio | Handled | [2147](../../crates/crysirror-qlie/src/adapter/objects.rs#L2147) |
| `condition; under op="l"` | `kind == "theora"` | self.resolve_movie | Handled | [2183](../../crates/crysirror-qlie/src/adapter/objects.rs#L2183) |
| `condition; under op="l"` | `kind == "theora" \|\| kind == "snd"` | self.objects.get_mut, h.submit_audio | Handled | [2188](../../crates/crysirror-qlie/src/adapter/objects.rs#L2188) |
| `condition; under op="l" / receiver=kind == "theora" \|\| kind == "snd"` | `kind == "snd" && o.asset.as_deref() != Some(&path)` | h.submit_audio | Handled | [2190](../../crates/crysirror-qlie/src/adapter/objects.rs#L2190) |
| `condition; under op="p" \| "p2" if kind == "snd"` | `op == "p" && n(0)? != 0` | Local state/value operations | Handled | [2299](../../crates/crysirror-qlie/src/adapter/objects.rs#L2299) |
| `condition; under op="p" \| "p2" if kind == "snd"` | `loops == -2 && self.objects[&id].channel != 0` | h.submit_audio | Handled | [2300](../../crates/crysirror-qlie/src/adapter/objects.rs#L2300) |
| `condition; under op="?nt" \| "?tt" if kind == "snd"` | `op == "?nt"` | u64::from | Handled | [2393](../../crates/crysirror-qlie/src/adapter/objects.rs#L2393) |
| `condition; under op="p4"` | `mode == 6` | Local state/value operations | Handled | [2460](../../crates/crysirror-qlie/src/adapter/objects.rs#L2460) |
| `condition; under op="alg" \| "alg2"` | `op == "alg2"` | Local state/value operations | Handled | [2498](../../crates/crysirror-qlie/src/adapter/objects.rs#L2498) |
| `condition; under op="?us" \| "?ui"` | `op == "?us"` | Local state/value operations | Handled | [2659](../../crates/crysirror-qlie/src/adapter/objects.rs#L2659) |
| `condition; under op="exv" \| "?exv"` | `op == "exv"` | Local state/value operations | Handled | [2838](../../crates/crysirror-qlie/src/adapter/objects.rs#L2838) |
| `condition; under op="ds" \| "hs"` | `op == "ds"` | self.put_parent, self.submit | Handled | [2936](../../crates/crysirror-qlie/src/adapter/objects.rs#L2936) |
| `condition; under op="ui" \| "us"` | `op == "us"` | Local state/value operations | Handled | [2948](../../crates/crysirror-qlie/src/adapter/objects.rs#L2948) |
| `op` | `"ck" if matches!(kind.as_str(), "btn" \| "sbtn")` | self.objects.get_mut, h.now_ms | Handled | [2042](../../crates/crysirror-qlie/src/adapter/objects.rs#L2042) |
| `op` | `"?ck" if matches!(kind.as_str(), "btn" \| "sbtn")` | Value::Int | Handled | [2070](../../crates/crysirror-qlie/src/adapter/objects.rs#L2070) |
| `op` | `"sc" if kind != "snd"` | self.objects.get_mut, u32::try_from | Handled | [2086](../../crates/crysirror-qlie/src/adapter/objects.rs#L2086) |
| `op` | `"?sc" if kind != "snd"` | Value::Int | Handled | [2096](../../crates/crysirror-qlie/src/adapter/objects.rs#L2096) |
| `op` | `"wim" if kind != "snd"` | self.objects.get_mut, f64::from | Handled | [2108](../../crates/crysirror-qlie/src/adapter/objects.rs#L2108) |
| `op` | `"?wim" if kind != "snd"` | Value::Int | Handled | [2115](../../crates/crysirror-qlie/src/adapter/objects.rs#L2115) |
| `op` | `"s" if kind == "snd"` | h.submit_audio | Handled | [2277](../../crates/crysirror-qlie/src/adapter/objects.rs#L2277) |
| `op` | `"p" \| "p2" if kind == "snd"` | h.submit_audio, self.play_sound | Handled | [2295](../../crates/crysirror-qlie/src/adapter/objects.rs#L2295) |
| `op` | `"fo" if kind == "snd"` | super::sound_filters::parse, self.objects.get_mut | Handled | [2315](../../crates/crysirror-qlie/src/adapter/objects.rs#L2315) |
| `op` | `"sk" if kind == "snd"` | h.audio_seek_preparation, h.release_audio_seek, h.audio_info, u64::from, h.begin_audio_seek | Handled | [2325](../../crates/crysirror-qlie/src/adapter/objects.rs#L2325) |
| `op` | `"pan" if kind == "snd"` | self.objects.get_mut, h.submit_audio | Handled | [2363](../../crates/crysirror-qlie/src/adapter/objects.rs#L2363) |
| `op` | `"?sk" if kind == "snd"` | f64::from, Value::Int, i32::try_from | Handled | [2379](../../crates/crysirror-qlie/src/adapter/objects.rs#L2379) |
| `op` | `"?nt" \| "?tt" if kind == "snd"` | h.audio_info, u64::from, Value::Int, i32::try_from | Handled | [2389](../../crates/crysirror-qlie/src/adapter/objects.rs#L2389) |
| `op` | `"p" if kind == "theora"` | self.media_ids.contains, self.media_ids.insert, self.objects.get_mut, h.media_request, self.put_parent | Handled | [2404](../../crates/crysirror-qlie/src/adapter/objects.rs#L2404) |
| `op` | `"?tt" \| "tt" if kind == "theora"` | h.media_info, Value::Int, i32::try_from | Handled | [2431](../../crates/crysirror-qlie/src/adapter/objects.rs#L2431) |
| `op` | `"ost" if kind != "snd"` | self.objects.get_mut | Handled | [2512](../../crates/crysirror-qlie/src/adapter/objects.rs#L2512) |
| `op` | `"pri" if kind != "snd"` | Local state/value operations | Handled | [2518](../../crates/crysirror-qlie/src/adapter/objects.rs#L2518) |
| `op` | `"?pri" if kind != "snd"` | Value::Int | Handled | [2526](../../crates/crysirror-qlie/src/adapter/objects.rs#L2526) |
| `op` | `"?wh" if kind != "snd"` | super::render::image_extent, Value::Int | Handled | [2563](../../crates/crysirror-qlie/src/adapter/objects.rs#L2563) |
| `op` | `"?p4" if kind != "snd"` | serde_json::from_slice, h.query_engine, EngineQuery::Layer, Value::Int | Handled | [2570](../../crates/crysirror-qlie/src/adapter/objects.rs#L2570) |
| `op` | `"?p" if kind == "snd"` | Value::Int, i32::from | Handled | [2715](../../crates/crysirror-qlie/src/adapter/objects.rs#L2715) |
| `op` | `"v" if kind == "snd"` | self.objects.get_mut, h.submit_audio | Handled | [2799](../../crates/crysirror-qlie/src/adapter/objects.rs#L2799) |
| `op` | `"?v" if kind == "snd"` | h.audio_info, Value::Int | Handled | [2814](../../crates/crysirror-qlie/src/adapter/objects.rs#L2814) |
| `op` | `_` | Local state/value operations | Unsupported: unsupported QLIE {kind} method \\{op} | [2975](../../crates/crysirror-qlie/src/adapter/objects.rs#L2975) |

### adapter/objects/bar.rs / QlieVm::bar_method

```rust
fn bar_method(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        id: &str,
        op: &str,
        a: &[Value],
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under op="l" \| "l2"` | `!self.objects[id] .properties .get("bar/loading") .and_then(\|v\| v.first()) .is_some_and(\|v\| v.text() == signature)` | self.objects.remove, self.free_object, Default::default, String::new, self.resolve | Handled | [52](../../crates/crysirror-qlie/src/adapter/objects/bar.rs#L52) |
| `condition; under op="l" \| "l2"` | `index == 2 && op == "l"` | String::new | Handled | [80](../../crates/crysirror-qlie/src/adapter/objects/bar.rs#L80) |
| `condition; under op="v" \| "vi"` | `op == "vi"` | Local state/value operations | Handled | [171](../../crates/crysirror-qlie/src/adapter/objects/bar.rs#L171) |
| `op; under op="?v" \| "?vi" \| "?mv" \| "?vt"` | `_` | Local state/value operations | Handled | [208](../../crates/crysirror-qlie/src/adapter/objects/bar.rs#L208) |
| `condition; under op="?us" \| "?ui"` | `op == "?us"` | Local state/value operations | Handled | [274](../../crates/crysirror-qlie/src/adapter/objects/bar.rs#L274) |
| `condition; under op="vtn" \| "dc" \| "sc" \| "in" \| "out" \| "mi" \| "mm"` | `let Some(child) = self.objects[id] .properties .get(&format!("bar/{part}_id")) .and_then(\|v\| v.first()) .map(Value::text)` | Local state/value operations | Handled | [360](../../crates/crysirror-qlie/src/adapter/objects/bar.rs#L360) |
| `condition; under op="?in" \| "?out" \| "?mi" \| "?vtn"` | `matches!(op, "?in" \| "?out")` | Local state/value operations | Handled | [382](../../crates/crysirror-qlie/src/adapter/objects/bar.rs#L382) |
| `condition; under op="?in" \| "?out" \| "?mi" \| "?vtn"` | `matches!(op, "?in" \| "?out")` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [382](../../crates/crysirror-qlie/src/adapter/objects/bar.rs#L382) |
| `op` | `_` | Local state/value operations | Unsupported: unsupported QLIE scrollbar method \\{op} | [389](../../crates/crysirror-qlie/src/adapter/objects/bar.rs#L389) |

### adapter/objects/native_log.rs / method

```rust
fn method(
    object: &mut Object,
    machine: &mut Machine,
    operation: &str,
    arguments: &[Value],
) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `operation` | `"?o"` | usize::try_from, Value::Int | Handled | [45](../../crates/crysirror-qlie/src/adapter/objects/native_log.rs#L45) |
| `operation` | `"?c" if object.properties.contains_key(LEGACY)` | Value::Int | Handled | [63](../../crates/crysirror-qlie/src/adapter/objects/native_log.rs#L63) |
| `operation` | `"o"` | serde_json::to_string | Handled | [67](../../crates/crysirror-qlie/src/adapter/objects/native_log.rs#L67) |
| `operation` | `"s" if object.properties.contains_key(LEGACY)` | usize::try_from, serde_json::to_string | Handled | [77](../../crates/crysirror-qlie/src/adapter/objects/native_log.rs#L77) |
| `operation` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [85](../../crates/crysirror-qlie/src/adapter/objects/native_log.rs#L85) |

### adapter/objects/strings.rs / install_text

```rust
fn install_text(object: &mut Object, source: &str) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `object.kind == "table"` | Local state/value operations | Handled | [15](../../crates/crysirror-qlie/src/adapter/objects/strings.rs#L15) |
| `condition; under receiver=object.kind == "table"` | `!name.is_empty() && !name.contains("//") && !value.contains("//")` | Local state/value operations | Handled | [20](../../crates/crysirror-qlie/src/adapter/objects/strings.rs#L20) |

### adapter/pointer_pick.rs / QlieVm::collision_hit

```rust
fn collision_hit(&self, h: &mut dyn HostServices, id: &str, x: i32, y: i32) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `o.kind == "bar"` | Local state/value operations | Handled | [65](../../crates/crysirror-qlie/src/adapter/pointer_pick.rs#L65) |

### adapter/pointer_pick.rs / QlieVm::select_points

```rust
fn select_points(&self, h: &mut dyn HostServices) -> Result<Vec<([i32; 2], i32)>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `sp == [0, 0] \|\| !self.group_admitted(&id)` | Local state/value operations | Handled | [160](../../crates/crysirror-qlie/src/adapter/pointer_pick.rs#L160) |

### adapter/reg.rs / QlieVm::reg_method

```rust
fn reg_method(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        op: &str,
        a: &[Value],
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(op, "?kl" \| "?vl")` | Local state/value operations | Handled | [67](../../crates/crysirror-qlie/src/adapter/reg.rs#L67) |
| `condition` | `matches!(op, "?kl" \| "?vl")` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [67](../../crates/crysirror-qlie/src/adapter/reg.rs#L67) |
| `condition; under op="vs" \| "vi"` | `op == "vs"` | RegValue::Text | Handled | [90](../../crates/crysirror-qlie/src/adapter/reg.rs#L90) |
| `condition; under op="?kl" \| "?vl"` | `op == "?vl"` | Local state/value operations | Handled | [121](../../crates/crysirror-qlie/src/adapter/reg.rs#L121) |
| `condition; under op="?kl" \| "?vl" / names=Some(names)` | `let Some(o) = self.objects.get_mut(&target).filter(\|o\| o.kind == "list")` | super::objects::strings_install | Handled | [134](../../crates/crysirror-qlie/src/adapter/reg.rs#L134) |
| `op` | `_` | Local state/value operations | Unsupported: unsupported QLIE reg method \\{op} | [142](../../crates/crysirror-qlie/src/adapter/reg.rs#L142) |

### adapter/render.rs / update

```rust
fn update(vm: &mut QlieVm, h: &mut dyn HostServices) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(o.kind.as_str(), "btn" \| "sbtn")` | serde_json::from_slice, h.query_engine, EngineQuery::UiInteraction, Vec::new | Handled | [1648](../../crates/crysirror-qlie/src/adapter/render.rs#L1648) |
| `condition` | `o.kind == "sbtn" && activated` | Local state/value operations | Handled | [1670](../../crates/crysirror-qlie/src/adapter/render.rs#L1670) |
| `condition` | `o.kind != "theora"` | h.media_request | Handled | [1756](../../crates/crysirror-qlie/src/adapter/render.rs#L1756) |
| `condition` | `matches!(o.kind.as_str(), "btn" \| "sbtn")` | serde_json::from_slice, h.query_engine, EngineQuery::Layer, usize::from | Handled | [1939](../../crates/crysirror-qlie/src/adapter/render.rs#L1939) |

### adapter/render.rs / update_bars

```rust
fn update_bars(
    vm: &mut QlieVm,
    h: &mut dyn HostServices,
    commands: &mut Vec<serde_json::Value>,
    ui: &mut Vec<serde_json::Value>,
) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Some(event) = events.into_iter().rev().find(\|e\| e.kind == "change")` | Local state/value operations | Handled | [1414](../../crates/crysirror-qlie/src/adapter/render.rs#L1414) |

### adapter/render.rs / Traversal<'_>::bundle

```rust
fn bundle(
        &mut self,
        bundle: &resources::Bundle,
        time: u64,
        parent: u32,
        motion_index: usize,
        depth: usize,
    ) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `sample.kind == 12` | self.layer | Handled | [736](../../crates/crysirror-qlie/src/adapter/render.rs#L736) |
| `condition` | `sample.kind == 0` | Local state/value operations | Handled | [742](../../crates/crysirror-qlie/src/adapter/render.rs#L742) |
| `condition` | `sample.kind == 5` | Local state/value operations | Handled | [745](../../crates/crysirror-qlie/src/adapter/render.rs#L745) |
| `condition` | `sample.kind == 6` | self.collision_quad, Arc::from, Vec::new, self.collision_regions.push, self.collision_keys.push | Handled | [751](../../crates/crysirror-qlie/src/adapter/render.rs#L751) |
| `condition` | `sample.kind == 8` | usize::try_from, self.object.images.get, u64::from, self.collision_quad, serde_json::to_string | Handled | [768](../../crates/crysirror-qlie/src/adapter/render.rs#L768) |
| `condition` | `matches!(sample.kind, 15 \| 16 \| 17)` | self.blend_mode, animation::blend, self.sampling, self.layer | Handled | [836](../../crates/crysirror-qlie/src/adapter/render.rs#L836) |
| `condition; under sample.kind=_` | `mode&0x7f==0` | Local state/value operations | Handled | [847](../../crates/crysirror-qlie/src/adapter/render.rs#L847) |
| `sample.kind` | `15` | Local state/value operations | Handled | [840](../../crates/crysirror-qlie/src/adapter/render.rs#L840) |
| `sample.kind` | `17` | Local state/value operations | Handled | [840](../../crates/crysirror-qlie/src/adapter/render.rs#L840) |
| `sample.kind` | `_` | animation::blend, self.sampling | Handled | [841](../../crates/crysirror-qlie/src/adapter/render.rs#L841) |
| `condition` | `matches!(sample.kind, 20 \| 21)` | self.particles | Handled | [857](../../crates/crysirror-qlie/src/adapter/render.rs#L857) |
| `condition` | `matches!(sample.kind,18\|19)` | usize::try_from, f64::from, std::ptr::eq, self.blend_mode, animation::blend | Handled | [870](../../crates/crysirror-qlie/src/adapter/render.rs#L870) |
| `condition` | `sample.kind==19&&selected!=255&&f64::from(sample.model_morph_weight)>0.99999` | std::ptr::eq | Handled | [885](../../crates/crysirror-qlie/src/adapter/render.rs#L885) |
| `condition` | `sample.kind==19` | Local state/value operations | Handled | [897](../../crates/crysirror-qlie/src/adapter/render.rs#L897) |
| `condition` | `matches!(sample.kind, 1 \| 14)` | Local state/value operations | Handled | [931](../../crates/crysirror-qlie/src/adapter/render.rs#L931) |
| `condition` | `member.nested.is_some()&&!matches!(sample.kind,3\|4\|7\|13)` | Local state/value operations | Handled | [945](../../crates/crysirror-qlie/src/adapter/render.rs#L945) |
| `condition` | `sample.kind == 7 && member.nested.is_none()` | Local state/value operations | Handled | [946](../../crates/crysirror-qlie/src/adapter/render.rs#L946) |
| `condition` | `sample.kind==9&&amount==255` | usize::try_from, self.object.images.get, std::array::from_fn, u64::from | Handled | [1032](../../crates/crysirror-qlie/src/adapter/render.rs#L1032) |
| `condition` | `matches!(sample.kind, 9 \| 11 \| 24)` | self.dual_image | Handled | [1142](../../crates/crysirror-qlie/src/adapter/render.rs#L1142) |
| `condition` | `!matches!(sample.kind, 1 \| 14) && nested.is_none()` | self.layer, serde_json::from_value | Handled | [1161](../../crates/crysirror-qlie/src/adapter/render.rs#L1161) |
| `condition` | `self.coverage_write\|\|sample.kind==13` | self.layer | Handled | [1164](../../crates/crysirror-qlie/src/adapter/render.rs#L1164) |
| `condition` | `matches!(sample.kind,1\|14)` | Local state/value operations | Handled | [1202](../../crates/crysirror-qlie/src/adapter/render.rs#L1202) |
| `condition` | `sample.kind==2` | Local state/value operations | Handled | [1203](../../crates/crysirror-qlie/src/adapter/render.rs#L1203) |
| `condition` | `sample.kind == 22` | Local state/value operations | Handled | [1216](../../crates/crysirror-qlie/src/adapter/render.rs#L1216) |
| `condition` | `sample.kind == 10` | usize::try_from | Handled | [1228](../../crates/crysirror-qlie/src/adapter/render.rs#L1228) |

### adapter/render.rs / Traversal<'_>::collision_frame

```rust
fn collision_frame(&mut self, parent:u32) -> Result<crysirror_scene::projection::HierarchyFrame>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Some(node)=self.object.published_children.iter() .find(\|node\|node["id"].as_u64()==Some(u64::from(id)))` | Local state/value operations | Handled | [62](../../crates/crysirror-qlie/src/adapter/render.rs#L62) |

### adapter/render.rs / Traversal<'_>::dual_image

```rust
fn dual_image(
        &mut self,
        bundle: &resources::Bundle,
        motion: usize,
        track: usize,
        sample: &animation::Sample,
        primary: &UploadedImage,
    ) -> Result<Option<UploadedImage>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `sample.kind` | `9 if amount == 255` | Local state/value operations | Handled | [251](../../crates/crysirror-qlie/src/adapter/render.rs#L251) |
| `sample.kind` | `9` | Local state/value operations | Handled | [252](../../crates/crysirror-qlie/src/adapter/render.rs#L252) |
| `sample.kind` | `11` | Local state/value operations | Handled | [253](../../crates/crysirror-qlie/src/adapter/render.rs#L253) |
| `sample.kind` | `24` | Local state/value operations | Handled | [254](../../crates/crysirror-qlie/src/adapter/render.rs#L254) |
| `sample.kind` | `_` | Local state/value operations | Handled | [255](../../crates/crysirror-qlie/src/adapter/render.rs#L255) |

### adapter/render.rs / Traversal<'_>::layer

```rust
fn layer(&mut self, mut spec: serde_json::Value) -> Result<u32>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `spec["paint"]["kind"] == "image"` | u32::from | Handled | [105](../../crates/crysirror-qlie/src/adapter/render.rs#L105) |
| `condition` | `self.coverage_write&&!matches!(kind,"group"\|"ordered_state"\|"coverage_reset"\|"coverage_write")` | Local state/value operations | Handled | [142](../../crates/crysirror-qlie/src/adapter/render.rs#L142) |
| `condition` | `matches!(kind,"group"\|"ordered_state"\|"coverage_reset"\|"coverage_write")` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [142](../../crates/crysirror-qlie/src/adapter/render.rs#L142) |
| `condition` | `!matches!(spec["paint"]["kind"].as_str(),Some("image"\|"legacy_mesh"\|"projected_mesh"\|"coverage_write"))` | Local state/value operations | Handled | [145](../../crates/crysirror-qlie/src/adapter/render.rs#L145) |

### adapter/render.rs / Traversal<'_>::particles

```rust
fn particles(
        &mut self,
        bundle: &resources::Bundle,
        sample: &animation::Sample,
        parent: u32,
        depth: usize,
    ) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `sample.kind == 21` | Local state/value operations | Handled | [361](../../crates/crysirror-qlie/src/adapter/render.rs#L361) |

### adapter/render.rs / Traversal<'_>::sampling

```rust
fn sampling(&self,sample:&animation::Sample) -> u8
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `sample.kind==22` | Local state/value operations | Handled | [93](../../crates/crysirror-qlie/src/adapter/render.rs#L93) |
| `condition` | `mode==0` | Local state/value operations | Handled | [95](../../crates/crysirror-qlie/src/adapter/render.rs#L95) |

### adapter/replacement.rs / QlieVm::expand_line

```rust
fn expand_line(&mut self, m: &mut Machine, ins: &Instruction) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `ins.op == "blank"` | String::new | Handled | [121](../../crates/crysirror-qlie/src/adapter/replacement.rs#L121) |
| `condition` | `ins.op == "text"` | Local state/value operations | Handled | [123](../../crates/crysirror-qlie/src/adapter/replacement.rs#L123) |

### adapter/replacement.rs / QlieVm::rewrite_line

```rust
fn rewrite_line(&self, line: &str, manager: &str) -> Result<String>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `mode == 0` | String::from_utf8, crysirror_text::byte_replace::replace_spans_bytes | Handled | [27](../../crates/crysirror-qlie/src/adapter/replacement.rs#L27) |
| `mode` | `1` | Local state/value operations | Handled | [53](../../crates/crysirror-qlie/src/adapter/replacement.rs#L53) |
| `mode` | `2` | Local state/value operations | Handled | [60](../../crates/crysirror-qlie/src/adapter/replacement.rs#L60) |
| `mode` | `3` | String::new | Handled | [61](../../crates/crysirror-qlie/src/adapter/replacement.rs#L61) |
| `mode` | `_` | Local state/value operations | Unsupported: unsupported QLIE replacement mode {mode} | [78](../../crates/crysirror-qlie/src/adapter/replacement.rs#L78) |

### adapter/resources.rs / bundle

```rust
fn bundle(
    input: Arc<[u8]>,
    client: ClientId,
    cancel: &crysirror_tasks::Cancellation,
    depth: usize,
    slots: &mut usize,
    id: usize,
) -> Result<Bundle>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `e.metadata["kind"] == 6 && !bytes.is_empty()` | Arc::from, Box::new | Handled | [191](../../crates/crysirror-qlie/src/adapter/resources.rs#L191) |
| `condition` | `e.metadata["kind"] == 8 && !bytes.is_empty()` | crysirror_asset::tqmo::Model::parse, Arc::from, Arc::new | Handled | [198](../../crates/crysirror-qlie/src/adapter/resources.rs#L198) |

### adapter/save_metadata.rs / Envelope::set_body

```rust
fn set_body(&mut self, bytes: &[u8], key: u32, mode: u8) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `mode` | `0` | Local state/value operations | Handled | [145](../../crates/crysirror-qlie/src/adapter/save_metadata.rs#L145) |
| `mode` | `1` | crysirror_resource::byte_pair::encode_literals | Handled | [146](../../crates/crysirror-qlie/src/adapter/save_metadata.rs#L146) |
| `mode` | `2` | crysirror_resource::word_run_length::encode_literals | Handled | [147](../../crates/crysirror-qlie/src/adapter/save_metadata.rs#L147) |
| `mode` | `_` | Local state/value operations | Unsupported: unsupported QLIE save encoding mode {mode} | [148](../../crates/crysirror-qlie/src/adapter/save_metadata.rs#L148) |

### adapter/save_metadata/native.rs / NativeObject::read

```rust
fn read(kind: &Text, r: &mut Reader<'_>) -> Result<Self>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `kind.as_str()` | `"img" \| "simg"` | ObjectPayload::Image, Image::read | Handled | [1352](../../crates/crysirror-qlie/src/adapter/save_metadata/native.rs#L1352) |
| `kind.as_str()` | `"msg"` | ObjectPayload::Message, MessageObject::read | Handled | [1353](../../crates/crysirror-qlie/src/adapter/save_metadata/native.rs#L1353) |
| `kind.as_str()` | `"btn" \| "sbtn"` | MessageObject::read | Handled | [1354](../../crates/crysirror-qlie/src/adapter/save_metadata/native.rs#L1354) |
| `kind.as_str()` | `"bar"` | Image::read | Handled | [1363](../../crates/crysirror-qlie/src/adapter/save_metadata/native.rs#L1363) |
| `kind.as_str()` | `"file"` | ObjectPayload::File | Handled | [1385](../../crates/crysirror-qlie/src/adapter/save_metadata/native.rs#L1385) |
| `kind.as_str()` | `"list" \| "table" \| "ppngpool"` | ObjectPayload::Strings | Handled | [1386](../../crates/crysirror-qlie/src/adapter/save_metadata/native.rs#L1386) |
| `kind.as_str()` | `"theora"` | Local state/value operations | Handled | [1390](../../crates/crysirror-qlie/src/adapter/save_metadata/native.rs#L1390) |
| `kind.as_str()` | `"timer"` | Local state/value operations | Handled | [1401](../../crates/crysirror-qlie/src/adapter/save_metadata/native.rs#L1401) |
| `kind.as_str()` | `"sys" \| "dat" \| "sdat" \| "snd" \| "dsnd" \| "eff" \| "scsr" \| "csr" \| "log" \| "rep" \| "path" \| "slog" \| "flog" \| "llog" \| "math" \| "cfg" \| "misc" \| "inp" \| "skey" \| "clip" \| "dlgs" \| "reg" \| "mdat" \| "tdat" \| "cam" \| "camex" \| "twitter" \| "ext"` | Local state/value operations | Handled | [1414](../../crates/crysirror-qlie/src/adapter/save_metadata/native.rs#L1414) |
| `kind.as_str()` | `_` | Local state/value operations | Unsupported: unsupported QLIE native object payload class {kind} | [1419](../../crates/crysirror-qlie/src/adapter/save_metadata/native.rs#L1419) |

### adapter/save_metadata/native.rs / ValueRecord::read

```rust
fn read(r: &mut Reader<'_>, objects: &mut dyn ObjectCodec) -> Result<Self>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `kind == 3` | Local state/value operations | Handled | [858](../../crates/crysirror-qlie/src/adapter/save_metadata/native.rs#L858) |
| `kind; under receiver=else: kind == 3` | `0` | Local state/value operations | Handled | [868](../../crates/crysirror-qlie/src/adapter/save_metadata/native.rs#L868) |
| `kind; under receiver=else: kind == 3` | `1` | Local state/value operations | Handled | [872](../../crates/crysirror-qlie/src/adapter/save_metadata/native.rs#L872) |
| `kind; under receiver=else: kind == 3` | `2` | Vec::with_capacity, Vec::new | Handled | [876](../../crates/crysirror-qlie/src/adapter/save_metadata/native.rs#L876) |
| `kind; under receiver=else: kind == 3` | `_` | Local state/value operations | Handled | [892](../../crates/crysirror-qlie/src/adapter/save_metadata/native.rs#L892) |

### adapter/save_state.rs / QlieVm::begin_restore

```rust
fn begin_restore(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        body: &[u8],
    ) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `id.starts_with("object/")` | Local state/value operations | Handled | [428](../../crates/crysirror-qlie/src/adapter/save_state.rs#L428) |
| `condition` | `id.starts_with("object/") && !names.contains_key(id)` | Local state/value operations | Handled | [434](../../crates/crysirror-qlie/src/adapter/save_state.rs#L434) |
| `condition` | `kind == "snd" && loops.is_some()` | Local state/value operations | Handled | [598](../../crates/crysirror-qlie/src/adapter/save_state.rs#L598) |

### adapter/save_state.rs / QlieVm::capture_state

```rust
fn capture_state(
        &self,
        h: &mut dyn HostServices,
        m: &Machine,
        flags: i32,
    ) -> Result<Vec<u8>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `o.kind == "snd" && o.channel != 0` | h.audio_info, f64::from | Handled | [313](../../crates/crysirror-qlie/src/adapter/save_state.rs#L313) |
| `condition` | `matches!(o.kind.as_str(), "cam" \| "camex") && o.properties.contains_key("col")` | Local state/value operations | Handled | [385](../../crates/crysirror-qlie/src/adapter/save_state.rs#L385) |

### adapter/sdat.rs / QlieVm::sdat_method

```rust
fn sdat_method(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        op: &str,
        a: &[Value],
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `op == "i"` | Local state/value operations | Handled | [28](../../crates/crysirror-qlie/src/adapter/sdat.rs#L28) |
| `condition` | `op == "d"` | Local state/value operations | Handled | [39](../../crates/crysirror-qlie/src/adapter/sdat.rs#L39) |
| `op` | `_` | self.pool.spawn, SdatIo::Exists | Handled | [72](../../crates/crysirror-qlie/src/adapter/sdat.rs#L72) |

### adapter/snd_native.rs / QlieVm::snd_native_method

```rust
fn snd_native_method(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        id: &str,
        op: &str,
        a: &[Value],
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `vm.objects[id].channel == 0` | Local state/value operations | Handled | [26](../../crates/crysirror-qlie/src/adapter/snd_native.rs#L26) |
| `op` | `_` | Local state/value operations | Handled | [191](../../crates/crysirror-qlie/src/adapter/snd_native.rs#L191) |

### adapter/sound_filters.rs / parse

```rust
fn parse(source: &str) -> Result<Chain>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `name.as_str()` | `"echo"` | Local state/value operations | Handled | [54](../../crates/crysirror-qlie/src/adapter/sound_filters.rs#L54) |
| `name.as_str()` | `"reverb"` | Local state/value operations | Handled | [58](../../crates/crysirror-qlie/src/adapter/sound_filters.rs#L58) |
| `name.as_str()` | `"chorus"` | Local state/value operations | Handled | [63](../../crates/crysirror-qlie/src/adapter/sound_filters.rs#L63) |
| `name.as_str()` | `"volume"` | Local state/value operations | Handled | [75](../../crates/crysirror-qlie/src/adapter/sound_filters.rs#L75) |
| `name.as_str()` | `"distortion"` | Local state/value operations | Handled | [80](../../crates/crysirror-qlie/src/adapter/sound_filters.rs#L80) |
| `name.as_str()` | `"noise"` | Local state/value operations | Handled | [85](../../crates/crysirror-qlie/src/adapter/sound_filters.rs#L85) |
| `name.as_str()` | `"3bandeq"` | Local state/value operations | Handled | [95](../../crates/crysirror-qlie/src/adapter/sound_filters.rs#L95) |
| `name.as_str()` | `"mono"` | Local state/value operations | Handled | [102](../../crates/crysirror-qlie/src/adapter/sound_filters.rs#L102) |
| `name.as_str()` | `"pitchshift"` | Local state/value operations | Handled | [103](../../crates/crysirror-qlie/src/adapter/sound_filters.rs#L103) |
| `name.as_str()` | `"timescale"` | Local state/value operations | Handled | [109](../../crates/crysirror-qlie/src/adapter/sound_filters.rs#L109) |
| `name.as_str()` | `"agc"` | Local state/value operations | Handled | [114](../../crates/crysirror-qlie/src/adapter/sound_filters.rs#L114) |
| `name.as_str()` | `other` | Local state/value operations | Unsupported: unsupported QLIE sound filter {other} | [120](../../crates/crysirror-qlie/src/adapter/sound_filters.rs#L120) |

### adapter/sys_native.rs / QlieVm::sys_native_method

```rust
fn sys_native_method(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        id: &str,
        op: &str,
        a: &[Value],
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `op` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [68](../../crates/crysirror-qlie/src/adapter/sys_native.rs#L68) |
| `condition; under op="?ss"` | `self.sys_int(id, "ss", 0) != 0` | Value::Int | Handled | [156](../../crates/crysirror-qlie/src/adapter/sys_native.rs#L156) |
| `op` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [262](../../crates/crysirror-qlie/src/adapter/sys_native.rs#L262) |

### adapter/text_statements.rs / QlieVm::text_statement

```rust
fn text_statement(&mut self, m: &mut Machine, op: &str, a: &[String]) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under op="drun"` | `command == "clear"` | self.global_strings.remove | Handled | [54](../../crates/crysirror-qlie/src/adapter/text_statements.rs#L54) |
| `condition; under op="drun"` | `command == "file" \|\| command.is_empty()` | self.global_strings.get | Handled | [57](../../crates/crysirror-qlie/src/adapter/text_statements.rs#L57) |
| `op` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [70](../../crates/crysirror-qlie/src/adapter/text_statements.rs#L70) |

### adapter/theora_native.rs / QlieVm::theora_native_method

```rust
fn theora_native_method(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        id: &str,
        op: &str,
        a: &[Value],
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `op; under op="ss" \| "pp" \| "ost"` | `_` | Local state/value operations | Handled | [30](../../crates/crysirror-qlie/src/adapter/theora_native.rs#L30) |
| `condition; under op="ss" \| "pp" \| "ost"` | `op == "ost"` | Value::Int, i32::from | Handled | [33](../../crates/crysirror-qlie/src/adapter/theora_native.rs#L33) |
| `condition; under op="?ss" \| "?pp" \| "?ost"` | `op == "?ss"` | Local state/value operations | Handled | [40](../../crates/crysirror-qlie/src/adapter/theora_native.rs#L40) |
| `op; under op="?bm" \| "?in" \| "?out" \| "?tf" \| "?vtn"` | `_` | Local state/value operations | Handled | [46](../../crates/crysirror-qlie/src/adapter/theora_native.rs#L46) |
| `op` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [76](../../crates/crysirror-qlie/src/adapter/theora_native.rs#L76) |

### adapter/thread.rs / QlieVm::thread_instruction

```rust
fn thread_instruction(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        ins: &Instruction,
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `selector; under ins.op.as_str()="thrd"` | `1` | self.start_system_reader | Handled | [517](../../crates/crysirror-qlie/src/adapter/thread.rs#L517) |
| `selector; under ins.op.as_str()="thrd"` | `0` | self.discard_system_objects, self.suspended_host.take, self.restore_host_context, h.now_ms, self.submit | Handled | [526](../../crates/crysirror-qlie/src/adapter/thread.rs#L526) |
| `selector; under ins.op.as_str()="thrd"` | `_` | Local state/value operations | Unsupported: unsupported QLIE thread selector {selector} | [542](../../crates/crysirror-qlie/src/adapter/thread.rs#L542) |
| `ins.op.as_str()` | `"srct"` | Machine::context_builtin, self.selection.iter, self.selection.clone | Handled | [497](../../crates/crysirror-qlie/src/adapter/thread.rs#L497) |
| `ins.op.as_str()` | `"thrd"` | self.start_system_reader, self.discard_system_objects, self.suspended_host.take, self.restore_host_context, h.now_ms | Partial: unsupported QLIE thread selector {selector} | [513](../../crates/crysirror-qlie/src/adapter/thread.rs#L513) |
| `ins.op.as_str()` | `"tsub"` | self.take_host_context, h.now_ms | Handled | [545](../../crates/crysirror-qlie/src/adapter/thread.rs#L545) |
| `ins.op.as_str()` | `_` | Local state/value operations | Handled | [585](../../crates/crysirror-qlie/src/adapter/thread.rs#L585) |

### adapter/visual.rs / QlieVm::effect_current

```rust
fn effect_current(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        op: &str,
        a: &[Value],
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under op="fdm"` | `!matches!(mode,1..=4\|7..=8\|12..=20)` | Local state/value operations | Handled | [768](../../crates/crysirror-qlie/src/adapter/visual.rs#L768) |
| `op; under op="?v" \| "?dc" \| "?sc" \| "?vtn" \| "?mb" \| "?bg"` | `_` | Value::Int | Handled | [808](../../crates/crysirror-qlie/src/adapter/visual.rs#L808) |
| `condition; under op="fdi" \| "fdo" if a.first().map(Value::integer).transpose()? == Some(2)` | `op == "fdo"` | Local state/value operations | Handled | [880](../../crates/crysirror-qlie/src/adapter/visual.rs#L880) |
| `condition; under op="fdi" \| "fdo"` | `op == "fdo"` | Local state/value operations | Handled | [923](../../crates/crysirror-qlie/src/adapter/visual.rs#L923) |
| `op` | `"fdi" \| "fdo" if a.first().map(Value::integer).transpose()? == Some(2)` | self.property_easing, super::animation::blend, Vec::new, std::mem::replace, self.submit | Handled | [855](../../crates/crysirror-qlie/src/adapter/visual.rs#L855) |
| `op` | `"fdi" if a.first().map(Value::integer).transpose()? == Some(4)` | self.property_easing, self.cancel_animated_transition, self.submit | Handled | [885](../../crates/crysirror-qlie/src/adapter/visual.rs#L885) |
| `op` | `"fdo" if !matches!(a.first().map(Value::integer).transpose()?, Some(0 \| 1 \| 3)) \|\| a.get(2).map(Value::text).unwrap_or_default().to_ascii_lowercase().ends_with(".b")` | Local state/value operations | Empty handler: Original no-op status not established | [915](../../crates/crysirror-qlie/src/adapter/visual.rs#L915) |
| `op` | `_` | Local state/value operations | Unsupported: unsupported QLIE effect method \\{op} | [993](../../crates/crysirror-qlie/src/adapter/visual.rs#L993) |

### adapter/visual.rs / QlieVm::position_repeated

```rust
fn position_repeated(
        &mut self,
        h: &mut dyn HostServices,
        id: &str,
        duration: u64,
        repeat: i32,
    ) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `mode == 6` | self.begin_jitter_position | Handled | [689](../../crates/crysirror-qlie/src/adapter/visual.rs#L689) |

### adapter/visual.rs / QlieVm::property_easing

```rust
fn property_easing(
        &self,
        id: &str,
        key: &str,
    ) -> Result<crysirror_scene::layer::Easing>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(self.objects[id].kind.as_str(), "cam" \| "camex")` | Local state/value operations | Handled | [18](../../crates/crysirror-qlie/src/adapter/visual.rs#L18) |
| `condition` | `key == "fdm"` | i32::from | Handled | [30](../../crates/crysirror-qlie/src/adapter/visual.rs#L30) |
| `condition` | `!matches!(mode,1..=4\|7..=8\|12..=20)` | Local state/value operations | Handled | [34](../../crates/crysirror-qlie/src/adapter/visual.rs#L34) |
| `condition` | `matches!(self.objects[id].kind.as_str(), "cam" \| "camex")` | Local state/value operations | Handled | [37](../../crates/crysirror-qlie/src/adapter/visual.rs#L37) |
| `condition` | `!matches!(mode, 0 \| 1 \| 3 \| 4 \| 7 \| 8 \| 12..=20)` | Local state/value operations | Handled | [41](../../crates/crysirror-qlie/src/adapter/visual.rs#L41) |

### adapter/visual.rs / QlieVm::put_parent

```rust
fn put_parent(&mut self, h: &mut dyn HostServices, id: &str) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `o.kind == "eff"` | self.root_layer_for | Handled | [542](../../crates/crysirror-qlie/src/adapter/visual.rs#L542) |

### expression.rs / Parser<'_, F>::expression

```rust
fn expression(&mut self, min: u8, evaluate: bool) -> Result<Value>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under token=Token::Name(mut name)` | `self.op("[")` | Vec::new, self.op, Value::Int, self.expression | Handled | [159](../../crates/crysirror-qlie/src/expression.rs#L159) |
| `condition; under token=Token::Name(mut name)` | `self.op(",") \|\| self.op("]")` | Value::Int | Handled | [164](../../crates/crysirror-qlie/src/expression.rs#L164) |
| `condition; under token=Token::Name(mut name)` | `!self.op(",")` | Local state/value operations | Handled | [170](../../crates/crysirror-qlie/src/expression.rs#L170) |
| `condition; under token=Token::Name(mut name)` | `name.starts_with("@@")` | Value::Text | Handled | [181](../../crates/crysirror-qlie/src/expression.rs#L181) |
| `op.as_str(); under token=Token::Op(op) if matches!(op.as_str(), "+" \| "-" \| "!" \| "~")` | `"!"` | Value::Int, i32::from | Handled | [199](../../crates/crysirror-qlie/src/expression.rs#L199) |
| `op.as_str(); under token=Token::Op(op) if matches!(op.as_str(), "+" \| "-" \| "!" \| "~")` | `"~"` | Value::Int | Handled | [200](../../crates/crysirror-qlie/src/expression.rs#L200) |
| `op.as_str(); under token=Token::Op(op) if matches!(op.as_str(), "+" \| "-" \| "!" \| "~")` | `"-"` | Value::Int | Handled | [201](../../crates/crysirror-qlie/src/expression.rs#L201) |
| `op.as_str(); under token=Token::Op(op) if matches!(op.as_str(), "+" \| "-" \| "!" \| "~")` | `_` | Value::Int | Handled | [202](../../crates/crysirror-qlie/src/expression.rs#L202) |
| `op.as_str()` | `"\|\|"` | Local state/value operations | Handled | [214](../../crates/crysirror-qlie/src/expression.rs#L214) |
| `op.as_str()` | `"&&"` | Local state/value operations | Handled | [215](../../crates/crysirror-qlie/src/expression.rs#L215) |
| `op.as_str()` | `"==" \| "!="` | Local state/value operations | Handled | [218](../../crates/crysirror-qlie/src/expression.rs#L218) |
| `op.as_str()` | `"<" \| ">" \| "<=" \| ">="` | Local state/value operations | Handled | [219](../../crates/crysirror-qlie/src/expression.rs#L219) |
| `op.as_str()` | `"\|"` | Local state/value operations | Handled | [220](../../crates/crysirror-qlie/src/expression.rs#L220) |
| `op.as_str()` | `"^"` | Local state/value operations | Handled | [221](../../crates/crysirror-qlie/src/expression.rs#L221) |
| `op.as_str()` | `"&"` | Local state/value operations | Handled | [222](../../crates/crysirror-qlie/src/expression.rs#L222) |
| `op.as_str()` | `"<<" \| ">>"` | Local state/value operations | Handled | [223](../../crates/crysirror-qlie/src/expression.rs#L223) |
| `op.as_str()` | `"+" \| "-"` | Local state/value operations | Handled | [224](../../crates/crysirror-qlie/src/expression.rs#L224) |
| `op.as_str()` | `"*" \| "/" \| "%"` | Local state/value operations | Handled | [225](../../crates/crysirror-qlie/src/expression.rs#L225) |
| `op.as_str()` | `_` | Local state/value operations | Handled | [226](../../crates/crysirror-qlie/src/expression.rs#L226) |
| `op.as_str(); under op.as_str()=_` | `"+"` | Local state/value operations | Handled | [260](../../crates/crysirror-qlie/src/expression.rs#L260) |
| `op.as_str(); under op.as_str()=_` | `"-"` | Local state/value operations | Handled | [261](../../crates/crysirror-qlie/src/expression.rs#L261) |
| `op.as_str(); under op.as_str()=_` | `"*"` | Local state/value operations | Handled | [262](../../crates/crysirror-qlie/src/expression.rs#L262) |
| `op.as_str(); under op.as_str()=_` | `"/"` | Local state/value operations | Handled | [263](../../crates/crysirror-qlie/src/expression.rs#L263) |
| `op.as_str(); under op.as_str()=_` | `"%"` | Local state/value operations | Handled | [267](../../crates/crysirror-qlie/src/expression.rs#L267) |
| `op.as_str(); under op.as_str()=_` | `"&"` | Local state/value operations | Handled | [271](../../crates/crysirror-qlie/src/expression.rs#L271) |
| `op.as_str(); under op.as_str()=_` | `"\|"` | Local state/value operations | Handled | [272](../../crates/crysirror-qlie/src/expression.rs#L272) |
| `op.as_str(); under op.as_str()=_` | `"^"` | Local state/value operations | Handled | [273](../../crates/crysirror-qlie/src/expression.rs#L273) |
| `op.as_str(); under op.as_str()=_` | `"<<"` | Local state/value operations | Handled | [274](../../crates/crysirror-qlie/src/expression.rs#L274) |
| `op.as_str(); under op.as_str()=_` | `">>"` | Local state/value operations | Handled | [275](../../crates/crysirror-qlie/src/expression.rs#L275) |
| `op.as_str(); under op.as_str()=_` | `"<"` | i32::from | Handled | [276](../../crates/crysirror-qlie/src/expression.rs#L276) |
| `op.as_str(); under op.as_str()=_` | `">"` | i32::from | Handled | [277](../../crates/crysirror-qlie/src/expression.rs#L277) |
| `op.as_str(); under op.as_str()=_` | `"<="` | i32::from | Handled | [278](../../crates/crysirror-qlie/src/expression.rs#L278) |
| `op.as_str(); under op.as_str()=_` | `">="` | i32::from | Handled | [279](../../crates/crysirror-qlie/src/expression.rs#L279) |
| `op.as_str(); under op.as_str()=_` | `_` | Local state/value operations | Handled | [280](../../crates/crysirror-qlie/src/expression.rs#L280) |
| `op.as_str()` | `"&&"` | Value::Int, i32::from | Handled | [239](../../crates/crysirror-qlie/src/expression.rs#L239) |
| `op.as_str()` | `"\|\|"` | Value::Int, i32::from | Handled | [240](../../crates/crysirror-qlie/src/expression.rs#L240) |
| `op.as_str()` | `"+" \| "&" if matches!(left, Value::Text(_))` | Value::Text | Handled | [243](../../crates/crysirror-qlie/src/expression.rs#L243) |
| `op.as_str()` | `"==" \| "!="` | Value::Int, i32::from | Handled | [249](../../crates/crysirror-qlie/src/expression.rs#L249) |
| `op.as_str()` | `_` | Value::Int, i32::from | Handled | [256](../../crates/crysirror-qlie/src/expression.rs#L256) |

### machine.rs / Machine::assign

```rust
fn assign(&mut self, name: &str, value: Value) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!self.frame.locals.contains_key(&key) && (self.frame.arrays.get(&key).is_some_and(\|count\| *count > 0) \|\| (!self.global_contains(&key) && self .global_array_extent(&key) .is_some_and(\|count\| count > 0)))` | Local state/value operations | Handled | [407](../../crates/crysirror-qlie/src/machine.rs#L407) |
| `kind.map(String::as_str)` | `Some("int")` | Value::Int | Handled | [429](../../crates/crysirror-qlie/src/machine.rs#L429) |
| `kind.map(String::as_str)` | `Some("str")` | Value::Text | Handled | [430](../../crates/crysirror-qlie/src/machine.rs#L430) |
| `kind.map(String::as_str)` | `_` | Local state/value operations | Handled | [431](../../crates/crysirror-qlie/src/machine.rs#L431) |

### machine.rs / Machine::cal

```rust
fn cal(&mut self, source: &str) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Some(name) = assignment.strip_suffix("++")` | self.eval, self.assign, Value::Int | Handled | [458](../../crates/crysirror-qlie/src/machine.rs#L458) |
| `condition` | `let Some(name) = assignment.strip_suffix("--")` | self.eval, self.assign, Value::Int | Handled | [461](../../crates/crysirror-qlie/src/machine.rs#L461) |

### machine.rs / Machine::control

```rust
fn control(&mut self, ins: &Instruction) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under ins.op.as_str()="vget" \| "vset"` | `ins.op == "vget"` | self.value, self.key, self.game_value, self.game_key | Handled | [1030](../../crates/crysirror-qlie/src/machine.rs#L1030) |
| `condition; under ins.op.as_str()="vget" \| "vset"` | `ins.op == "vget"` | self.assign | Handled | [1048](../../crates/crysirror-qlie/src/machine.rs#L1048) |
| `condition; under ins.op.as_str()="for"` | `a.get(1).is_some_and(\|condition\| !condition.is_empty()) && !self.eval(&a[1])?.truth()` | self.frame.loops.remove | Handled | [1092](../../crates/crysirror-qlie/src/machine.rs#L1092) |
| `condition; under ins.op.as_str()="end"` | `begin.op == "for"` | self.cal | Handled | [1103](../../crates/crysirror-qlie/src/machine.rs#L1103) |
| `condition; under ins.op.as_str()="end"` | `begin.op == "case"` | self.frame.cases.pop | Handled | [1109](../../crates/crysirror-qlie/src/machine.rs#L1109) |
| `condition; under ins.op.as_str()="brk" \| "cnt"` | `ins.op == "brk"` | self.frame.loops.remove | Handled | [1124](../../crates/crysirror-qlie/src/machine.rs#L1124) |
| `condition; under ins.op.as_str()="of" \| "ans"` | `matches!(op, "if" \| "for" \| "case")` | Local state/value operations | Handled | [1172](../../crates/crysirror-qlie/src/machine.rs#L1172) |
| `condition; under ins.op.as_str()="of" \| "ans"` | `matches!(op, "if" \| "for" \| "case")` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [1172](../../crates/crysirror-qlie/src/machine.rs#L1172) |
| `condition; under ins.op.as_str()="of" \| "ans"` | `op == "end"` | Local state/value operations | Handled | [1174](../../crates/crysirror-qlie/src/machine.rs#L1174) |
| `condition; under ins.op.as_str()="of" \| "ans"` | `depth == 0 && matches!(op, "of" \| "ans" \| "else")` | Local state/value operations | Handled | [1176](../../crates/crysirror-qlie/src/machine.rs#L1176) |
| `condition; under ins.op.as_str()="of" \| "ans"` | `matches!(op, "of" \| "ans" \| "else")` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [1176](../../crates/crysirror-qlie/src/machine.rs#L1176) |
| `condition; under ins.op.as_str()="sub" \| "subp" \| "subr" \| "jmp" \| "go"` | `ins.op == "subp"` | Local state/value operations | Handled | [1193](../../crates/crysirror-qlie/src/machine.rs#L1193) |
| `condition; under ins.op.as_str()="sub" \| "subp" \| "subr" \| "jmp" \| "go"` | `matches!(ins.op.as_str(), "sub" \| "subr")` | self.call_parameter | Handled | [1201](../../crates/crysirror-qlie/src/machine.rs#L1201) |
| `kind.as_str(); under ins.op.as_str()="sarr"` | `"int"` | Value::Int | Handled | [1283](../../crates/crysirror-qlie/src/machine.rs#L1283) |
| `kind.as_str(); under ins.op.as_str()="sarr"` | `"str" \| "obj"` | Value::Text, String::new | Handled | [1284](../../crates/crysirror-qlie/src/machine.rs#L1284) |
| `kind.as_str(); under ins.op.as_str()="sarr"` | `_` | self.object_types.insert, Value::Text | Handled | [1285](../../crates/crysirror-qlie/src/machine.rs#L1285) |
| `ins.op.as_str()` | `"var" \| "gvar" \| "svar"` | self.declare | Handled | [994](../../crates/crysirror-qlie/src/machine.rs#L994) |
| `ins.op.as_str()` | `"init"` | self.key, self.global_arrays, self.shared_bank.arrays.get, self.value, self.assign | Handled | [995](../../crates/crysirror-qlie/src/machine.rs#L995) |
| `ins.op.as_str()` | `"cal"` | self.cal | Handled | [1027](../../crates/crysirror-qlie/src/machine.rs#L1027) |
| `ins.op.as_str()` | `"vget" \| "vset"` | self.value, self.key, self.game_value, self.game_key, self.eval | Handled | [1028](../../crates/crysirror-qlie/src/machine.rs#L1028) |
| `ins.op.as_str()` | `"include"` | self.resolve_file, self.push_inline | Handled | [1054](../../crates/crysirror-qlie/src/machine.rs#L1054) |
| `ins.op.as_str()` | `"if"` | self.eval | Handled | [1060](../../crates/crysirror-qlie/src/machine.rs#L1060) |
| `ins.op.as_str()` | `"else"` | self.frame.cases.last, self.frame.cases.pop, self.frame.cases.last_mut | Handled | [1066](../../crates/crysirror-qlie/src/machine.rs#L1066) |
| `ins.op.as_str()` | `"for"` | self.frame.loops.insert, self.cal, self.eval, self.frame.loops.remove | Handled | [1085](../../crates/crysirror-qlie/src/machine.rs#L1085) |
| `ins.op.as_str()` | `"end"` | self.cal, self.frame.cases.pop | Handled | [1100](../../crates/crysirror-qlie/src/machine.rs#L1100) |
| `ins.op.as_str()` | `"brk" \| "cnt"` | self.frame.loops.remove, self.frame.cases.retain | Handled | [1113](../../crates/crysirror-qlie/src/machine.rs#L1113) |
| `ins.op.as_str()` | `"case"` | self.eval | Handled | [1141](../../crates/crysirror-qlie/src/machine.rs#L1141) |
| `ins.op.as_str()` | `"of" \| "ans"` | self.frame.cases.pop, self.eval, self.frame.cases.last_mut | Handled | [1147](../../crates/crysirror-qlie/src/machine.rs#L1147) |
| `ins.op.as_str()` | `"sub" \| "subp" \| "subr" \| "jmp" \| "go"` | self.eval, self.call_parameter, self.capture_call_results, self.jump | Handled | [1185](../../crates/crysirror-qlie/src/machine.rs#L1185) |
| `ins.op.as_str()` | `"ret"` | self.call_parameter | Handled | [1219](../../crates/crysirror-qlie/src/machine.rs#L1219) |
| `ins.op.as_str()` | `"vexs"` | self.key, self.result, Value::Int, i32::from, self.frame.locals.contains_key | Handled | [1231](../../crates/crysirror-qlie/src/machine.rs#L1231) |
| `ins.op.as_str()` | `"sarr"` | self.eval_array_shape, Self::array_element_count, self.frame.arrays.contains_key, self.frame.arrays.get, self.global_arrays | Handled | [1243](../../crates/crysirror-qlie/src/machine.rs#L1243) |
| `ins.op.as_str()` | `"garr"` | self.result, Value::Int | Handled | [1321](../../crates/crysirror-qlie/src/machine.rs#L1321) |
| `ins.op.as_str()` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [1341](../../crates/crysirror-qlie/src/machine.rs#L1341) |

### machine.rs / Machine::declare

```rust
fn declare(&mut self, op: &str, args: &[String]) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `op == "var"` | self.frame.array_shapes.insert | Handled | [501](../../crates/crysirror-qlie/src/machine.rs#L501) |
| `condition` | `op == "svar"` | self.shared_bank.array_shapes.insert | Handled | [503](../../crates/crysirror-qlie/src/machine.rs#L503) |
| `condition` | `op == "svar"` | Local state/value operations | Handled | [518](../../crates/crysirror-qlie/src/machine.rs#L518) |
| `condition` | `op == "svar"` | self.shared_bank.arrays.insert | Handled | [531](../../crates/crysirror-qlie/src/machine.rs#L531) |
| `condition` | `(if global { if op == "svar" { &self.shared_bank.values } else { self.thread_globals() } } else { &self.frame.locals }) .contains_key(&key)` | Local state/value operations | Handled | [556](../../crates/crysirror-qlie/src/machine.rs#L556) |
| `condition` | `op == "svar"` | Local state/value operations | Handled | [557](../../crates/crysirror-qlie/src/machine.rs#L557) |
| `kind.as_str()` | `"int"` | Value::Int | Handled | [578](../../crates/crysirror-qlie/src/machine.rs#L578) |
| `kind.as_str()` | `"str" \| "obj"` | Value::Text, String::new | Handled | [579](../../crates/crysirror-qlie/src/machine.rs#L579) |
| `kind.as_str()` | `_` | self.object_types.insert, Value::Text | Handled | [580](../../crates/crysirror-qlie/src/machine.rs#L580) |
| `condition` | `op == "svar"` | self.shared_bank.values.insert | Handled | [591](../../crates/crysirror-qlie/src/machine.rs#L591) |

### machine.rs / Machine::inline_label_chain

```rust
fn inline_label_chain(
        &self,
        path: &str,
        label: &str,
        depth: usize,
    ) -> Result<Option<Vec<(String, usize)>>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `ins.op != "include"` | Local state/value operations | Handled | [842](../../crates/crysirror-qlie/src/machine.rs#L842) |

### machine.rs / Machine::value

```rust
fn value(&self, name: &str) -> Result<Value>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `self.frame.arrays.get(&name).is_some_and(\|count\| *count > 0)` | self.frame.locals.get | Handled | [226](../../crates/crysirror-qlie/src/machine.rs#L226) |
| `condition` | `let Some(value) = self.frame.locals.get(&format!("{name}[0]"))` | Local state/value operations | Handled | [227](../../crates/crysirror-qlie/src/machine.rs#L227) |
| `condition` | `self .global_array_extent(&name) .is_some_and(\|count\| count > 0)` | self.global_value | Handled | [237](../../crates/crysirror-qlie/src/machine.rs#L237) |
| `condition` | `let Some(value) = self.global_value(&format!("{name}[0]"))` | Local state/value operations | Handled | [241](../../crates/crysirror-qlie/src/machine.rs#L241) |
| `condition` | `name.starts_with("paramstr[")` | Value::Text, String::new | Handled | [245](../../crates/crysirror-qlie/src/machine.rs#L245) |
| `condition` | `name.starts_with("paramint[")` | Value::Int | Handled | [248](../../crates/crysirror-qlie/src/machine.rs#L248) |
| `condition; under name.as_str()="selffilename" \| "selffilepath"` | `name == "selffilepath"` | Self::frame_reader_path | Handled | [269](../../crates/crysirror-qlie/src/machine.rs#L269) |
| `name.as_str()` | `"true"` | Value::Int | Handled | [252](../../crates/crysirror-qlie/src/machine.rs#L252) |
| `name.as_str()` | `"#quot"` | Value::Text | Handled | [254](../../crates/crysirror-qlie/src/machine.rs#L254) |
| `name.as_str()` | `"false" \| "null" \| "nil"` | Value::Int | Handled | [255](../../crates/crysirror-qlie/src/machine.rs#L255) |
| `name.as_str()` | `"self"` | Value::Text, self.reader_path | Handled | [256](../../crates/crysirror-qlie/src/machine.rs#L256) |
| `name.as_str()` | `"selffilename" \| "selffilepath"` | Value::Text, Self::frame_reader_path | Handled | [257](../../crates/crysirror-qlie/src/machine.rs#L257) |
| `name.as_str()` | `"selfthreadname"` | Value::Text, self.thread_kind | Handled | [278](../../crates/crysirror-qlie/src/machine.rs#L278) |
| `name.as_str()` | `_` | Local state/value operations | Unsupported: undefined QLIE variable {name} | [279](../../crates/crysirror-qlie/src/machine.rs#L279) |

### machine/context.rs / Machine::assign_game

```rust
fn assign_game(&mut self, name: &str, value: Value) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!game.frame.locals.contains_key(&key) && (game.frame.arrays.get(&key).is_some_and(\|count\| *count > 0) \|\| (!self.globals.contains_key(&key) && !self.shared_bank.values.contains_key(&key) && self .arrays .get(&key) .or_else(\|\| self.shared_bank.arrays.get(&key)) .is_some_and(\|count\| *count > 0)))` | Local state/value operations | Handled | [101](../../crates/crysirror-qlie/src/machine/context.rs#L101) |

### machine/context.rs / Machine::game_value

```rust
fn game_value(&self, name: &str) -> Result<Value>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `game.frame.arrays.get(&name).is_some_and(\|count\| *count > 0)` | Local state/value operations | Handled | [41](../../crates/crysirror-qlie/src/machine/context.rs#L41) |
| `condition` | `let Some(value) = game.frame.locals.get(&format!("{name}[0]"))` | Local state/value operations | Handled | [42](../../crates/crysirror-qlie/src/machine/context.rs#L42) |
| `condition` | `self .arrays .get(&name) .or_else(\|\| self.shared_bank.arrays.get(&name)) .is_some_and(\|count\| *count > 0)` | self.global_value_for | Handled | [52](../../crates/crysirror-qlie/src/machine/context.rs#L52) |
| `condition` | `let Some(value) = self.global_value_for(ThreadKind::Game, &format!("{name}[0]"))` | Local state/value operations | Handled | [58](../../crates/crysirror-qlie/src/machine/context.rs#L58) |

### machine/context.rs / Machine::set_source_thread

```rust
fn set_source_thread(&mut self, selector: i32) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `selector` | `0` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [140](../../crates/crysirror-qlie/src/machine/context.rs#L140) |
| `selector` | `1` | Local state/value operations | Handled | [141](../../crates/crysirror-qlie/src/machine/context.rs#L141) |
| `selector` | `_` | Local state/value operations | Unsupported: unsupported QLIE source-thread selector {selector} | [142](../../crates/crysirror-qlie/src/machine/context.rs#L142) |

### program.rs / Program::parse_source

```rust
fn parse_source(path: &str, text: &str, file_admission: bool) -> Result<Self>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `op == "then"` | Local state/value operations | Handled | [245](../../crates/crysirror-qlie/src/program.rs#L245) |
| `condition` | `matches!(op.as_str(), "if" \| "for" \| "case")` | Local state/value operations | Handled | [256](../../crates/crysirror-qlie/src/program.rs#L256) |
| `condition` | `op == "else"` | Local state/value operations | Handled | [259](../../crates/crysirror-qlie/src/program.rs#L259) |
| `condition` | `op == "end"` | Local state/value operations | Handled | [273](../../crates/crysirror-qlie/src/program.rs#L273) |
