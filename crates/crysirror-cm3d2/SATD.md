# CM3D2 self-admitted technical debt

## SATD-cm3d2-game-logic

**Scope and current location.** The port of `Assembly-CSharp` game logic in
[`src/scenes.rs`](src/scenes.rs), [`src/kag.rs`](src/kag.rs) and
[`src/tjs.rs`](src/tjs.rs). Ported: scene start/update of `SceneFirst`,
`SceneLogo`, `SceneWarning`, `SceneADV`, `TitleMgr` (Start button);
ADV tags `scenecall`, `wait`, `motionblendenabled`, `eval`, `s`; TJS
functions `SetManFlag`, `GetManFlag`, `SetTmpFlag`, `GetTmpFlag`,
`SetNewTrophy`.

**Remaining work.** Every other scene controller (`SceneUserEdit`, `SceneEdit`,
`SceneCharacterSelect`, `SceneDaily`, `SceneStartDaily`, `SceneYotogi`,
`SceneMaidManagement`, `SceneShop`, `SceneTrophy`, `SceneTryInfo`,
`SceneToTitle`, `SceneSingleEffect`, `SceneStaffRoll`, `SceneDance_DDFL`,
`SceneCompetitiveShow`), title Continue/Config/End and keyboard commands,
the remaining ~90 KAG tags of `BaseKagManager`/`ADVKagManager`/
`YotogiKagManager`/`MotionKagManager`, the 45 remaining TJS functions of
`ScriptManager.Initialize`, `MessageWindowMgr` (text, names, backlog,
skip/auto, voice), `SystemShortcut`, `ConfigMgr`, `SaveAndLoadMgr`,
`CharacterMgr`/`Maid`/`TBody` (`.menu` assembly, parts, morphs, bone morphs,
physics hair, IK, eye/head look), `MaidParam`/`PlayerParam`/`Schedule`/
`Yotogi` state from the `.nei` tables, `SoundMgr` (BGM/SE/voice from
`sound.arc`/`voice*.arc`), and original `SaveData/*.save` compatibility.

**Closure evidence.** Installed-game headless runs through user edit, maid
creation, schedule days, yotogi, saves/loads and an ending, recorded in
[docs/validation/cm3d2.md](../../docs/validation/cm3d2.md).

| Date | Change | Remaining scope |
| --- | --- | --- |
| 2026-11-20 | First controllers, ADV KAG loop and TJS context; boot reaches the title with its 3D background. | Everything listed above. |
| 2026-11-20 | Title Start path, message window open/close/text, `talk`/`hitret`/`fade`/`fade3d`/`cameracontrol`/BGM/SE tags, `SoundMgr` volumes. The opening scenario reaches its first `hitret`. | Name entry (`SceneUserEdit`), typewriter/skip/auto/backlog/voice of `MessageWindowMgr`, delayed `playse`, and the rest listed above. |
| 2026-11-20 | Every registered tag of `BaseKagManager`, `ADVKagManager` and `MotionKagManager` (wait events, motion queue/layers/crossfade, items and temp props, masks/mask modes, face/blend/mouth/lip sync/kuchipaku, eye targets, IK targets, physics switches, prefabs on BG/characters, texture layers incl. `res:` textures, choices with `SelectButton` prefabs/`UIGrid`/`TweenAlpha`, voices, days), native `R_call`/`R_return`, `ReplacePersonal`; `MaidProp` (temp/normal files, `AllProcProp` re-dirty rules), `TBody.FixMaskFlag`, the `Maid.Update` face pipeline. The opening runs to `SceneCall SceneEdit`. | Applying head/eye look, IK, auto twist, hair/bust physics and character prefabs to the pose (needs the CPU skeleton); voice lip sync from audio level; `ScheduleAPI.DayStartManage`, `Maid.UpdateMaidRanking` and the full `MaidParam`/`PlayerParam`; Yotogi tags (`talkrepeat`, `voicewait`, `playbgmroom`); `SceneEdit` and later scenes; backlog/skip/auto/typewriter; saves. |
| 2026-11-20 | Typed `param.Status`/`param_player.Status` with their original `BinaryWriter` layouts ([`src/params.rs`](src/params.rs), [`src/binio.rs`](src/binio.rs)); `MaidParam`/`PlayerParam` operations ([`src/maid_param.rs`](src/maid_param.rs): `Initialize`, clamped setters with the original quirks, `feature_learn`/`propensity_learn`, class acquisition/bonus, seikeiken, skills, salon grade, items, trophies); every `.nei` decrypted at boot into [`src/gamedata.rs`](src/gamedata.rs) (class EXP, trophies, shop, salon grades, scenario items, yotogi skills, `MaidRandomName` from the installed Assembly-CSharp) and [`src/schedule_data.rs`](src/schedule_data.rs) (`ScheduleCSVData`); all 55 `ScriptManager` TJS functions with character-manager requests; `ProfileCtrl.UpdateProfileData` on the edit OK; SceneEdit OK → name call → confirmation. The opening, interview questions and first edit run to `SceneCall SceneStartDaily`. | `SceneStartDaily`, `SceneDaily` and the later scenes; `ScheduleAPI` (day start, work simulation, results); `UpdateProfileComment` text; whether `DLL_KAG_ClearCallStack` also clears the `R_call` stack (only the parser stack is cleared); `TrophyAchieveEffect`; Yotogi tags (`talkrepeat`, `voicewait`, `playbgmroom`); backlog/skip/auto/typewriter; saves (the parameter sections serialize; the save file frame and slots do not exist yet). |
| 2026-11-20 | `ScheduleAPI` ([`src/schedule.rs`](src/schedule.rs): slots, work availability and error checks, `DayStartManage`, random work, `DecideSuccess`, noon/night result simulation, `Correction`, `BackupParamAccessor`, `WorkResultScene` noon/night, result trophies); `SceneStartDaily` and `SceneDaily` (Daytime/Night menu with Next and the label buttons, `ResultWorkDaytime`/`ResultWorkNight` running the work results, `ResultIncome` with loan repayment and salon grade-up dialog) in [`src/daily.rs`](src/daily.rs); `StatusViewer`; TJS eval accepts a trailing `;`. A day runs from the date eyecatch through the noon result to the night menu. | `SceneCharacterSelect` (Yotogi/Vip/Communication/Dance) and the `WfScreen` framework, `SceneYotogi`, `SceneMaidManagement`, `SceneShop`, `SceneTrophy`; the schedule, save and load panels of the menu; the per-maid rows of the work results; maid thumbnails (`ThumShot`); the native KAG state blob of saves (`DLL_KAG_CreateSerializeData`); `MainCamera.Blur`/`Vignett`. |
| 2026-11-20 | `SceneYotogi` ([`src/yotogi.rs`](src/yotogi.rs)) over a generic `WfScreenManager`/`WfScreenChildren` ([`src/wf.rs`](src/wf.rs)): stage select (playable stages by best salon grade, thumbnails, BG/camera/BGM per stage), skill select (new-skill acquisition, category tabs, stage/condition/seikeiken/drunk filters, seven slots with HP cost and `only_one`, wait motion per personality/condition, costume edit label, swoon confirmation), play (`NextSkill`, valid commands per normal/insert state and skill level, `EvalExpression`, `ApplyExecCommandStatus` with corrections and study rate, propensity/erogenous-zone awakening, every normal/drunk/proclivity label rule, RC/RRC/RR classification, `yotogi_skill_call_main`/`command_call_main` dispatch, command factory groups, skill tower, parameter bars and scroll, repeat voice, control mask, undressing units, man transparency, swoon), result (skill EXP animation with level-up, parameter changes, acquired skills/propensities, `GIG` voices, yotogi trophies), `ApplyStatus` and `ResetWorld`; static tables in [`src/yotogi_data.rs`](src/yotogi_data.rs); `yotogi_kag` (`talk`/`hitret`/`talkrepeat`/`voicewait`/`playbgmroom`/`controlmask`, unregistered tags ignored as `KagScript.CallTag` does), `yotogicall`, `charaactivateleader`; `TagSceneCall` closes the message window; the file index descends dotted folders. A yotogi night runs from stage select through four skills and the result back to the daily scene. | `MaidManagement`, `Shop`, `Trophy` scenes; the schedule, save and load panels of the menu; the per-maid rows of the work results; maid thumbnails (`ThumShot`); saves (`GameMain.Serialize`: the `KagScript` blob is a read-history block, `KAGParser.store()` as a TJS struct string, a flag byte and the `KagStackCall` list — the KAG snapshot needs krkr line/position mapping); in yotogi, the head-bone camera target of `InitMaidGesture` (CPU skeleton pose), skill icon drag swapping and hover names, `UIWFTabButton` selected states, `HideScroll` window folding, `AttributeViewer` hover, `MulTexRemove` of only layers 10–20, `TrophyAchieveEffect`; `MainCamera.Blur`/`Vignett`. |
| 2026-11-21 | `SceneTrophy` (trophy list, totals), `SceneStaffRoll` (credits scroll and pictures of `Resources.LoadAll(SceneStaffRoll/Texture)`, Return skip, logo, label return), `SceneSingleEffect` with `startmvpeffect`; title Config/End; `MessageWindowMgr` skip/auto/voice/backlog/config buttons and the skip/auto advance; `ConfigMgr` panel (message, man transparency and volume sliders). | `SceneCompetitiveShow` (`CompetitiveShowMgr`/`MaidExaminationMgr`), `SceneDance_DDFL` (`DanceMain`, its Unity animation/audio timeline), `TrophyAchieveEffect`, `TutorialPanel`; config screen settings without a host counterpart (resolution, full screen, anti-aliasing, shadow/texture quality, bloom, capture size, 3D sound, sequential voice: recorded only), popup lists; skip ignores the read-history condition (`MsgAlreadySkip`) and the text speed has no typewriter to drive; maid thumbnails (`ThumShot`, needs an offscreen render of a second camera). |
| 2026-11-20 | Original saves ([`src/save.rs`](src/save.rs)): `GameMain.Serialize`/`Deserialize` with header, `CharacterMgr` (player, stock men, stock maids), `MaidProp` lists in `AddProp` order (`nFileNameRID` as Mono `String.GetHashCode`), `MULTI_COL`, `MaidParam`, `MAID_MISC`, and the `ScriptManager` KAG (`KagScript.Serialize` blob through the shared `krkr_store` CM3D2 profile and the plug-in's `Dictionary.toStructString`, `KagStackCall`); slot files `SaveData/SaveDataNNN.save`; `SaveAndLoadMgr`/`SaveAndLoadCtrl` panels from the daily menu and the title's Continue ([`src/saveload.rs`](src/saveload.rs)): pages, units, dialogs, save, load, cancel; `TitleMgr` writes slot −1 (`タイトルへ戻る`). Loading resumes at the saved label with the call stack and restored characters. | No sample save of the original game was available: interoperability with saves written by the game is unverified (field layouts follow the decompiled writers). Not written or kept: the read-history block (empty), maid thumbnails, comments edited in the panel (`OnChangeComment`), save inside `if` blocks or with active macro arguments (refused). `SceneToTitle` (loading slot −1), title Config/End, and the rest listed above. |

## SATD-cm3d2-ngui

**Scope and current location.** [`src/ngui.rs`](src/ngui.rs): layout of
`UIRoot`/`UIPanel`/`UIWidget` and drawing of `UISprite` (simple, sliced),
`UI2DSprite`, `UITexture` and `UILabel` (alignment, pivot placement,
wrapping, `ShrinkContent`, line spacing).

**Remaining work.** `UILabel` effects (shadow/outline), justified text,
gradient, colour markup and bitmap `UIFont` labels, tiled/filled/advanced sprites, flips, panel clipping and soft
clip, anchors to targets declared after the widget and to cameras,
transform rotation, `UIGrid`/`UITable`/`UIScrollView` layout, tweens other
than the title's, `UIButton` colour/sprite states and the `UICamera` event
model (hover, press, drag, keyboard navigation).

**Closure evidence.** Captures of every scene's UI against the original
layout, and input tracks exercising its controls.

| Date | Change | Remaining scope |
| --- | --- | --- |
| 2026-11-20 | Widget layout, anchors to earlier widgets/screen, panel depth and alpha, sprites and textures as image layers. | As listed. |
| 2026-11-20 | Anchors resolve within their scene and within their `Instantiate` copy (`Scene::find_ref`); `UILabel` layout through the shared text measurement. | As listed, plus `TrophyAchieveEffect`. |
| 2026-11-21 | Stored-byte (gamma) blending of image layers, sliced-sprite padding in pixels, zero-size nine-patch centres, `UILabel` `spacingX` (glyph runs) and ClampContent line limits, `showScrollBars` fading, run-time `UITexture` Unity textures. | Label effects, popup lists, scroll dragging, the rest listed. |
| 2026-11-20 | Run-time `Transform.localScale`, `SetDimensions`, `UIButton.isEnabled` (collider off, `disabledColor`), `UI2DSprite.sprite2D` images; `UIPanel` clipping as layer viewport clips and clipped hit tests; `UIScrollView.ResetPosition`; `UITable` cells sized by descendant widgets; right clicks reach `onClick`; slider `onChange` delegates added at run time. | Soft clip edges (`mClipSoftness` alpha), scroll dragging and scroll bars, and the rest listed. |

## SATD-cm3d2-rendering

**Scope and current location.** Materials of [`src/bg.rs`](src/bg.rs) and of
`crysirror-asset/src/interchange/cm3d2.rs`; parts colours in
[`src/parts_color.rs`](src/parts_color.rs); body visibility and part
attachment in [`src/chara.rs`](src/chara.rs).

**Implemented.** `CM3D2/Toony_Lighted*` and `CM3D2/Lighted*` forward passes
as the engine toon model (ramp, shadow texture/rate, rim, hair highlight,
specular, `_ShadowColor` under cast shadows at the light's 0.1 strength),
the `CM3D2/Outline` hull, `_MultiColTex` tables of `MaidParts` (native
`WriteColorTableToTexture`/`…ShadowTable…`), `テクスチャ合成` layers
(`2dAlpha`/`2dMultiply`), `TBody.FixVisibleFlag` triangle removal, part
roots parented to the slot bone, the `LightMain` light and level
`RenderSettings` ambient, no tonemapping (Unity gamma space).

**Remaining work.** `CM3D2/Man` reads a GrabPass (`Color·ALPHAMAN +
grab·(1−ALPHAMAN)`, alpha `ALPHAMAN_CUT`), so its own overlaps do not stack;
the engine alpha blend lets them stack. `ボーンにアタッチ` hand items keep the
rest offset instead of the identity local transform, and
`アタッチ <slot> <point>` attach points are recorded but not applied. Unity
lightmaps (none in the 1.00 levels), `Bloom`/`Blur`/`Vignetting`/
`ScreenOverlay` camera effects (Assembly-UnityScript, not in the managed
schema), Unity `ParticleSystem` (`Prefab/Particle/*`: yotogi fluids,
`PKamihubuki` confetti; no reader for the native class yet) and `Animator`
clips of UI objects (the MVP sprite), the mosaic and `Lighted_EzRiflect`/`Mirror` shaders, Unity
`Particles/*` and `Toon/Lighted_CM3D*` materials (PBR approximation), and
point/spot lights of the forward-add pass.

**Closure evidence.** Side-by-side captures of maid, room and title scenes.

| Date | Change | Remaining scope |
| --- | --- | --- |
| 2026-11-20 | PBR approximation of game and Unity materials, base texture, colour and alpha mode. | Toon shading and everything below. |
| 2026-11-20 | Engine toon model and outlines; CM3D2 and Unity scene materials mapped to it; parts colour tables; texture layers; visibility by triangle removal; bone-attached parts; Unity lighting/ambient. | As listed. |
