# 甘夏ろけーしょん self-admitted technical debt

## SATD-amanatsu-port

**Scope and current location.** The whole adopt-mode port: asset readers in
`crysirror-unity` (shared) and game semantics in this crate.

**Remaining work.** `ActionMap` beyond the new-game map select,
`ChangeMap`, heroine entry/standing placement, the `ActionMapUI` HUD
(cameras, 2D map, `Skip`) and time-zone advance (tutorial
`ConfirmDialog`, HUD config/help and button fade animations, HScene
coordinate card loading, the flag turning morning
events 1/2 into 3, the item events' CGs and heroine placement and the item
HScene settings flag, `AutoSaveMode`,
link-partner placement (`SetCharaForMap` `isLink`), the `MapSelectUI`
hover faces (`_charaInfos`) and player icon (`_pIcon`),
`MapSelectUI` next-zone reloading a map behind it, the light settings'
baked lightmaps (`_colorLightmaps`/`_lightmapIndices`: only map 00's `Lake
Polygon` is lightmapped in the shipped settings; the renderer has no second
UV set) and reflection cubemaps, Azure's scattering integral,
clouds, moon and stars (the sky is a gradient fit from the Azure hour,
latitude and Rayleigh/Mie gradients), `ActionPoint` choice and `_animIDList` motions, NavMesh
movement, `CharaSelectUI` sources/sorting/pairs/release, scroll views and
`RectMask2D` clipping, `CommandScene` (prelude choice, the heroine-motion/water-gun/
`*_Fin` assets of `Get*Asset`, the HScene-after/move-map ends, the exact
`GetGeneralCommandAgreeAsset` branches beyond favourability > 3, cost
for massage/with/invite, the favourability level-up
short movies (`IsNextShortMovie` gates, the movies and the LV increase are
ported; `sm.rs` lists the timeline tracks that are not played), parameter gauge/max animations, `IsForgetPoint`; talks now play `e1-approach` and one
`e1-N`), ADV expressions/motions/event CGs/cloth states and the
auto/skip/log/config buttons and text reveal,
the map-select title/option buttons, the save list's comment entry, local-time stamps (UTC is used) and animations, map open flags from save progress);
map rendering parity (the lake lightmap of `ma/lit`, reflection probes, the Azure sky
time of day, Beautify sharpen/depth of field/film grain/vignette and URP
ShadowsMidtonesHighlights (tonemap, exposure, saturation, contrast, colour
temperature and bloom are applied), the PS Vita lite tonemap without colour
grading, `AL/MapColorMask`, water
and particle shaders, camera-light fades and `CmpCorrectLightAngle`); the synchronous
`ChangeMap` import (≈1–2 s inside one VM step behind the fade) moving to a
worker under ADR 0050; characters beyond the card-driven assembly in
`CustomScene` (the scene's preset instead of `AL_F_00`, accessory swing
and hide states, `customshape` face/body shape values, eye shadow,
paint/eyelid layers, the AL forward program terms listed in the
2026-11-23 rows (skin base pre-grade, SSS, shine, rim, specular, the md
lamp shift, card `LineWidth` on the outline; `_SkinIntensity` detail rate, the
`mt_body_detail` normal map/line mask, `mt_body_shine` and the sunburn
and body detail overlay layers are not drawn: their textures use separate UV
layouts/positioned decals not in the body's UV0), clothing body cut-outs,
the eye bones, MagicaCloth depth curves, inertia and MeshCloth
(BoneCloth hair and the body BoneSprings are simulated as spring bones;
hair `colliderList`s hold only null slots in the shipped prefabs and no
runtime assignment was found in `GameAssembly.dll`, so hair does not collide), height blending between the
`_S/_M/_L` clips, controller transitions, characters on maps); the
`CustomScene` character maker UI (instantiated from `lib/custom` prefabs at
runtime; only the background is drawn) and `Memory` scene controller; on the title: the Memories entry; saves:
the `SaveLoadUI` list (つづきから loads the newest save; MapSelectUI
Save/Load), manual saves, the original's auto-save rotation limit,
`SaveTimeText` in local time (UTC is written), `PlayerData` from the
player card editor (the default male card is saved), save versions (0.0.0)
and import of saves in the installation's `UserData/save`, the `ConfigWindow` parts listed in `config.rs`, the `ExitDialog` animations
confirmation (quit is immediate), the TOS check/viewer, the language dialog,
Selectable hover/press transitions (`ColorTint`, Animator triggers), the
ILLGAMES `PlayAnimationUI` tweens (LitMotion easing) and the uploader/
downloader; Animator parameter conditions; sound: BGM cross-fades, config
volumes, the title call voice, ENV/3D SE and voices outside the title, the
`UnityPlayer.dll` setup-header scan on a worker; the ADV commands still reported as unported (character, event CG
and HScene commands), characters (body/face/hair/clothes, colour masks, blend
shapes, bones, physics), maps and HScene mode,
saves (`UserData/save`), character cards; TextMeshPro Alpha8 SDF atlases
beyond the RGBA decode limit; meshes with `m_MeshCompression`.

**Next work list (survey 2026-11-21, `material_census`, ActionMapScene
decompile).** New game: `ActionMapScene.Initialize` creates `SaveData`, loads
the player from the default card (`PropertyData._defaultMaleCardFileName`
`AL_M_00`, `HumanData.LoadCharaFile`/`LoadFromPreset`), instantiates the
map prefab (`ActiomMapPrefabScenePath`) and `AngleData`, opens
`ActionMapUI`/`MapSelectUI`/`PlayerSelectUI`/`CharaSelectUI` prefabs and
the tutorial `ConfirmDialog`. Map rendering is in place (see history);
characters still need the toon path. Shader census — maps: `Universal Render
Pipeline/Lit` (104), `Nature/SpeedTree8_PBRLit` (459), `AL/Map` (282),
`AL/MapColorMask`, NatureManufacture water, Azure sky, Lux/VFX particles;
characters: ILLGAMES toon shaders `AL/skin_body`, `AL/skin_head`, `AL/hair`,
`AL/hair_outline`, `AL/cloth`, `AL/cloth_alpha`, `AL/acs*`, `AL/eye*`,
`AL/nail` (150–490 properties each: colour masks, paint/cheek layers,
outlines); character cards (PNG + ILLGAMES data), body/face assembly,
Cinemachine cameras, NavMesh wandering, Mecanim humanoid animation.

**Closure evidence.** Installed-game runs through the title, ADV, map and
HScene mode and save/load, recorded in a validation note.

| Date | Change | Remaining scope |
| --- | --- | --- |
| 2026-11-21 | Shared Unity 6 readers (bundles, type trees, generated MonoBehaviour trees, textures incl. BC4/BC6H/BC7, meshes with skinning and blend shapes); ADV scenario decoding. | Everything listed above. |
| 2026-11-21 | Boot flow Init → Logo → Title with the scene fade; shared Mecanim playback (`crysirror_unity::animator`), UnityEvent persistent calls and uGUI raycast/interactable inheritance; title menus, Back/right click, scene loads and quit ported from `TitleScene`. | Listed above. |
| 2026-11-21 | Sound: shared FSB5 (Vorbis/PCM16/IMA ADPCM) conversion in `crysirror-audio`; `TableData` lists; title BGM, menu SE and the logo brand call. | Listed above. |
| 2026-11-21 | 3D maps: shared `crysirror_unity::world6` (Unity 6 materials → engine PBR, GLB chunks, LOD 0, static batches, texture budget), `Scene6::subtree`, root-Canvas `CanvasGroup` fix; `MapSelectUI` + `MapConfirmDialog` + `ChangeMap` (map scene, time-zone light setting, key light, ambient, fog, Cinemachine camera) for all seven maps. | Listed above. |
| 2026-11-21 | Characters: shared skinned GLB export, `Scene6::graft`/`alias_bones`, material hook, MessagePack; `HumanData` cards and list tables; the default heroine assembled (body, head, hair, clothes, skin/clothes/eye colours) in `CustomScene`. | Listed above. |
| 2026-11-21 | Characters posed (`Pose_D_00_Loop` as glTF animation), cel shaded with card outline/hair colours, CustomScene background image plane. | Listed above. |
| 2026-11-21 | `CharaSelectUI` (preset cards grid, thumbnails, entry slots), shared `GridLayoutGroup`, runtime images, `clone_subtree`; entered heroines placed at the map's standing action points with the key-light toon scale. | Listed above. |
| 2026-11-21 | ADV program and the `ADVCore` window: picking a heroine on the map starts a conversation (text, voices, choices with `ReplaceLanguage`, variables, branches) that returns to the map. | Listed above. |
| 2026-11-21 | `CommandUI` menu on later clicks: greeting, talk/listen/topic/with/massage scenarios by family, 戻る; panel hidden during events. Scenario choice by parameters, invite/massage/HScene and point changes remain. | Listed above. |
| 2026-11-21 | Physics and lighting: shared spring bones (`crysirror_asset::spring`, GLB `crysirror_springs`, renderer pose pass) fed by MagicaCloth BoneCloth; shared `Environment.grading`; map global Volume (Beautify tonemap/exposure/saturation/contrast/temperature, URP Bloom). | Listed above. |
| 2026-11-22 | Body physics: the bone prefab `p_cf_al_body_bone_00` grafted onto the skeleton; its 14 MagicaCloth BoneSprings (chest, hip, upper arm, thigh, belly, waist) tuned from `ShakeCorrectData` by the card softness values and chest weight (`HumanBody.SetSpringSetting`/`ChangeBustGravity`); shared capsule colliders and distance limits in `crysirror_asset::spring`; skin shadow colour from the card. | Listed above. |
| 2026-11-22 | Clothes dressed state (`Table.sKeyNameTable` DEF/NUGE objects, `CmpClothes.objOpt01/02` by `hideOpt`); eyes: card pupil (`mt_eyepipil`) and highlights (`mt_eye_hi_up`) per eye with `CmpEyeLookMaterial` ranges. | Listed above. |
| 2026-11-22 | `ActionMapUI` HUD (map name, move-point cameras prev/next/reset, 2D map back to `MapSelectUI`, `Skip` → `EventManager.TimeZone`: `Cycle.IncrementTimeZone` mod 4 and the map reloaded in the zone's light setting/Volume); `MapSelectUI._btnNextTimeZone`; `ImageSprite.Set` for the zone icons and map illustrations. | Listed above. |
| 2026-11-22 | Zone lighting: the active light groups' point/spot lights as engine lights (map layers 0/4/7/8, URP intensity/range); the Azure hour/latitude/gradients as a shared gradient sky (`crysirror_asset::sky`) uploaded through the new `upload_hdr` host service. | Listed above. |
| 2026-11-22 | Heroine placement per zone (`GetForPlacementTable`/`SetCharaForMap`: shuffled heroines to shuffled maps under `_mapLimitFull`, esthetics/HScene maps excluded; only the current map's heroines stand there); `PlayBGMBYActionMap` (`_bgmID` per zone) and the `TimeZoneChange` SE. | Listed above. |
| 2026-11-22 | Heroine parameters (`params.rs`: `GameParameter`/`ParameterInfo` level-ups, favourability gauge and `LatePoint`, `PropertyData._param` command points with `IndividualityRate`, `IsNextShortMovie` gates; talk/listen/topic points with the chosen choice, command-menu stars/gauges); native saves (shared `crysirror_unity::memorypack`; `save.rs` `SaveData`/`PlayerData`/`NPCData` `.sav` layout), the auto save after a zone change into the engine's save mount and つづきから loading it. | Listed above. |
| 2026-11-22 | Command scenario choice (`commands.rs`: `PropertyData._adv` talk/listen/topic/agree/end assets by favourability, map and zone, lowest priority then random) and command cost (`CostProperty.GetMax`, `ConsumptionCost`, `RecoveryCost` per zone, hourglasses; exhaustion lines without cost). | Listed above. |
| 2026-11-22 | Massage (`EventManager.Massage` + `AL.Oil`): agree, esthetics map with the heroine on `Est00` in the first oil pose, pose camera data, `OilMassageScene` UI, CPU-posed hit-object raycast feeding the `SatisfactionData` gauge (front/back groups), Exit → evaluation stage scenario, Massage points, `Count.Massage`, cost, return; shared uGUI anchor overrides and `Slider` value. Remaining: excitement/HScene flag, oil gauge, pose changes/reactions, IK, surface effects, clothes/light UI, camera orbit. | Listed above. |
| 2026-11-22 | 一緒に遊ぶ (`EventManager.With`): shown with a full favourability gauge and the `IsNextShortMovie` gate, agree + `_shortMoiveAfterAssets[LV]`, favourability LV +1 with `LatePoint` as the new gauge. Remaining: the ShortMovie timeline itself. | Listed above. |
| 2026-11-22 | HScene (`EventManager.H` + `AL.H.HScene`): H button by cost and favourability LV > 0 (`SetCost`), agree + opener (`Map_No_Here` without an H place), the hotel's `H00` with the heroine and the default male in the first posture (`aib`) from the posture lists (`h.rs`), posture camera, `HScene` UI, click-stepped Idle/weak/strong loops filling the female gauge, Finish Female, Exit → evaluation, H points, `Count.H`, cost; map changes release the old map a step before loading; generated model releases also drop prepared assets (runtime). Remaining: posture selection and categories, wheel speed, male gauge/finishes, voices/SE, IK/neck, items, surface effects, clothes states, invite and the HScene map choice, evaluation from the HScene result, 3P/rez. | Listed above. |
| 2026-11-22 | Night events (`EventManager.TimeZone` at night → `NightEvent`): `GetNightEventHeroine` (favourability LV ≥ 4, `IsH`, not `IsNightH`), `IsNightEvent` (`_nightEventRate`), the heroine at `NightEvent00`, `Before_Night_(NO_)Cottage` with decline (choice 1) → `Night`, the shower ADV, the night HScene at the shower movie map's `NightEventH00` (`al_short_movie` `Map`), `H_NightShowerAfter` points, `AddHCount(force)`, `Night_H_After`, `Night` into the morning, `NightEventCount` → `morning` with `Judge` or the greeting ADV, morning HScene from `IsHScene` (`H_NightMorningAfter`, `_h.Event_After`), `SceneEnd`. `NPCData.Count` members beyond the gates are kept and saved (`IsH`/`IsTimeZoneH`/`IsNightH` from `HDetail`, per-zone resets); ADV `Open` returns to its caller; shared `g` scenarios; the auto save is `a_000.sav` (`EnsureFileNameIfEmpty`); TMP fonts register once per session and release their load jobs. | Listed above. |
| 2026-11-22 | `SaveLoadUI` (`saveload.rs`): title (`IsTitle`) and map-select (`_btnSaveLoad`) entry, 10 tabs × 10 slots cloned from the templates and the auto slot, `SaveDataComponent.Refresh` states, selection, `StartBubble` kinds, Load/New/Override/Delete dialogs (`SaveLoadDialog`, zone sprites), slot files (`SaveInfo.GetFileName`), the newest-slot mark; the title no longer loads the newest save directly. Shared uGUI: uncontrolled layout-group children take their flexible cell and sit inside it by alignment. | Listed above. |
| 2026-11-22 | HScene postures: `AnimationListInfo` members (`Mode` category, `Positions`, `Category` flags, `ReleasePhase`, `PostureState`, `WeaknessID`), map `HPoint`s (`h/list/hpoint/object` → `hpoint_NN`: `_placeType`, `_isFirst`), `HPointCtrl.FindFirst`, `FindPosture.Check` (`CheckReleasePhase`, weakness variants, place, first not standing) and the random first posture, `PostureSelect` (`hui.rs`: category buttons by `_viewDatas`, the category's posture list placed as `LoopListView2` does, fold/unfold) and `ChangePosture` respawning the pair. Remaining: moving between H points (`Canvas Point`), `HPointLimit`, exhaustion (`isWeakness`), Les/M1F2 actors, the list scroll and animations. | Listed above. |
| 2026-11-22 | HScene point mode (`Option/RightBottomArea/Point` → `Canvas Point`): markers cloned from the place-type filter buttons at the HPoints' screen positions (edge-pinned outside the view), Back, `ChangePoint` with a posture usable at the new place and the posture lists rebuilt; the male gauge and the category finishes (female, male inside/outside, joint) by the states the posture's animator has. Remaining: HPoint particle markers and colliders (no particle systems in the shared importer), the point mode's free camera and filters, the finish sub-menus, surface effects, male finish counts. | Listed above. |
| 2026-11-22 | Short movies: shared `crysirror-unity` `timeline.rs`; `sm.rs` (`stm_NN` Motion/Camera animator tracks, scene fades, `stm_cam_NN` root/near-far-FOV curves at the heroine's height, `al_sm_personal_NN`); the adapter moves to the movie map (`al_short_movie`), places the heroine at `ShortNN` (`_shortMoiveID`), plays states/camera (rig camera facing −Z)/fades on the timeline clock, then the after ADV; 一緒に遊ぶ (agree → movie → `_shortMoiveAfterAssets[LV]` → LV up → back to the map), the night shower and morning greeting movies. Remaining tracks: Face, ClothState, BasePos, two surface-effect tracks, TimeScale, Shake, Activation, Control, Alembic, items; skip. | Listed above. |
| 2026-11-22 | Item events (`EventManager.Item`): `AL.ItemOutlinable` rest items picked on screen, `GetHeroine` (LV ≥ 2), `GetEvent` (`_itemEventRate`, the HScene variant by LV 4/`IsH`/zone H/`IsItemEvent`/`_itemEventHRate`), `rest` with `Judge`/`G_MapID`, 休まない, the item HScene at `ItemEventH00` (no cost), `QuestionsFromNPC` points by `ItemSelectNum`, `IsItemEvent` saved; `SelectNum` now comes from the scenario variable. | Listed above. |
| 2026-11-22 | HScene breath voices (`hvoice.rs`: `breath[personality]` `BreathData`, wait/loop/finish groups re-triggered on a 3–5 s timer). The group choice is an approximation of `HVoiceCtrl` (see the module); short breaths, speech voices, face parameters and SE lists remain. | Listed above. |
| 2026-11-22 | Short movie `SETrack` clips play their `SEClip` sounds at the clip starts (2D; `Is3D`/`SettingNo` not applied). | Listed above. |
| 2026-11-22 | Faces and the command camera: shared morph-target export; `face.rs` (`FaceBlendShape` eyebrow/eyes/mouth patterns → morph weights, `com/lis/exp` expression presets, `CinemachineCameraAngleData` command camera); map heroines show the default expression, conversations apply `Expression`/`CharaExpression`, the command menu frames the heroine with camera 0 (`Stand`) and restores the map view; map heroines spawn one per step after the map (a step's generated uploads are bounded). Remaining: eye look, cheeks, tears, highlights, lip sync, blend transitions, `isFront`/height-specific camera IDs for other poses, faces of HScene/movie characters. | Listed above. |
| 2026-11-22 | 誘う (`EventManager.Invite`): an HScene map other than the current one (`_mapID.HMaps`) with its `_invite` scenario (`Normal_Hotel`/`Normal_Rock`), declining (`SelectNum` 1) or the HScene there on the map's first H point. Remaining: the water-gun variants and the invite point camera. | Listed above. |
| 2026-11-22 | Character lighting: characters take the map's `Directional Light Sun Chara` as the toon key light (`Environment.toon_light`), the materials' `_Lamp` ramps and `_AmbientIntensity`, ramp-relative cast shadows and surface-shaded outlines; `AL/skin_head` cheek/nose/lip makeup baked into the face texture through the face mesh's UV sets. Remaining: the skin shader graph (SSS, skin colour layers, highlights, rim), the weaker `Directional Light C`, the makeup size/position ranges, eye shadow, face detail/paint, moles and the expression blush map. | Listed above. |
| 2026-11-22 | Accessories: card `Accessory` slots as `ao_*` list prefabs under their `AccessoryParentKey` nodes with the `N_move`/`N_move2` corrections and card colours through the material colour mask. Remaining: `DynamicBone` swing, hide categories/visible timings, patterns, gloss/metallic, hair-coloured accessories. | Listed above. |
| 2026-11-22 | HScene faces: HScene characters register their `FaceBlendShape`, motions keep the face morphs, the breath's first `FaceParam` sets patterns/openness/blinking and the mouth moves between its min/max while the breath plays. Remaining: voice-level lip sync, tears/cheek/highlight, neck/eye look behaviour, the other `FaceParam` entries, short-movie faces. | Listed above. |
| 2026-11-22 | Short movie faces: `FaceTrack` clips by personality (expression preset or patterns/openness) at their starts; movie motions keep the face morphs. Remaining: blend times, look targets, tears, cheek, blink and lip-sync flags. | Listed above. |
| 2026-11-22 | Talk lip sync: voice lines move the talking heroine's mouth for the clip length (`AudioClip.m_Length`), shared with the HScene breath mouth. Remaining: voice-level (`LipSync`) amplitude. | Listed above. |
| 2026-11-22 | `ConfigWindow`: the `Config` prefab from the HUD gear, title option/F1 and HScene; toggles, sliders, volume switches, scrolling under `RectMask2D` clipping (shared `ugui` clip rects, `Mask` graphic, dropdown templates, grid preferred sizes, slider handles), `config.xml` in the save mount, volumes as mix groups. Remaining per `config.rs`: dropdowns, colour pickers, voice rows, shortcuts, reset, fade, and applying the non-volume values (text speed/alpha, camera, HScene toggles, auto-save mode). | Listed above. |
| 2026-11-22 | Config values applied: `AutoSaveMode` (0 never, 1 every zone, 2 mornings), `FontSpeed` message reveal (click completes it), `WindowAlpha`, `HScene/Visible`; faces whose entities were despawned are dropped (an HScene start after a talk hit a missing entity). Remaining per `config.rs`. | Listed above. |
| 2026-11-22 | `ExitDialog`: quitting (title Escape/ゲーム終了) asks first; はい quits, いいえ/× close. | Listed above. |
| 2026-11-22 | HScene motion: the posture animators' `AL.H.Motion.State` states (W/S loops 1/2, SP loops, finishes → `_ST` → `_A` idles with `S_` strong variants, two service finishes), the three `Solo` finish options, wheel strength and one-shots advancing at their clip end (`IsNext`). Remaining: the exact transition lambdas, `YuragiCtrl` blending, the four transition states, consecutive finishes, surface effects/particles and pour voices. | Listed above. |
| 2026-11-22 | HScene entry transitions: from the outer idles (Idle, `_OUT_A`) a click plays the entry transition, then its idle until the next click starts the loop. Remaining: the other three transitions (no trigger found yet). | Listed above. |
| 2026-11-22 | HScene clothes: characters spawned for HScene export every clothing variant (`n_*_a/b/c`, unique per part) and the shared `AnimationState.hidden` hides the variants/parts of each part's state; `ClothesSettingMenu` opens from `Clothes`/`Acs`, the part buttons and `ALL` cycle states (`SetClothesStateNext`), the accessory `ALL` toggle hides accessories, `Toggle Female/Male` selects the character. Remaining: coordinate cards, per-slot accessories, the menu animation and the native per-part state rules (each variant in name order, then off). | Listed above. |
| 2026-11-22 | Short movie `ClothStateTrack` (0 dressed, 1 half, 2 off per part) through the HScene clothing states. Not yet observed in a capture (`stm_0/2/3/20` only). | Listed above. |
| 2026-11-22 | Survey: the ADV `morning` scenario alone uses `EventCGSetting`/`EventCGRelease` (`adv/evcg` `ILLGAMES.ADV.EventCG.Data` root, `chaPos`, `camPos` `CameraData`), `CharaMotion` (IDs from `ani/lis/m` `anim_f_NNN`/`anim_m`, e.g. 2004 → `al_f_Hevent_adv_01`), `CharaClothState` and `CharaVisible`; map 00 has 29 particle systems (waterfall mists with `Legacy Shaders/Particles/Additive`, Vefects fire, Azure rain) for which the engine emitter has no texture input yet. Both remain open. | Listed above. |
| 2026-11-22 | Map particles: the shared Unity ParticleSystem importer (`crysirror_unity::particles`) and textured engine VFX entities for the active nodes (pool waterfall mists and foam, morning/evening/night groups). Remaining: Vefects fire/heat-haze shaders (`SH_Vefects_*`, custom erosion/flipbook graphs), Azure weather effects (rain, storm, ripples under the weather profile), soft-additive blending (drawn additive), velocity over lifetime, mesh render mode, per-particle start-size/colour randomness under over-lifetime curves, the HPoint particle markers. | Listed above. |
| 2026-11-22 | ADV event CG: `EventCGSetting`/`Release` (camera at `camPos`, characters at `chaPos`), `CharaStandFind`, `CharaMotion` through the `anim_f_*`/`anim_m` lists, `CharaVisible` `All`/`Head`, `CharaDitherIn/Out` as show/hide, `CharaClothState` for both actors. Remaining: `CharaMotionIKSetPartner`, the two `LookAt…Add/Remove` gaze commands, `CharaChange`, `CharaCoordinate` (Swimsuit), the remaining `CharaVisible` part, dither fades, `CameraData` height correction/animator and `Data.Scenes`, the censor overlay. | Listed above. |
| 2026-11-22 | Eye look: the heroines' irises follow the camera through `EyeLookCalc` pattern 1 and `CmpEyeLookMaterial` ranges as per-node UV offsets (shared `AnimationState.uv_offsets`). Remaining: the animated eye bone pose (rest pose under the root stands in), `LeapSpeed` easing, micro saccades, `ChangeLookEyesPtn`/ADV look commands, the highlight staying fixed while the iris moves (baked together), eyes in HScene/events without a recorded root. | Listed above. |
| 2026-11-22 | `Memory` scene: thumbnails/titles/unlock from `Memory_Thumbnail`, `sm_timeline` and `GlobalData.Flags` (`global.dat` read/written in the save mount, `AddShortMovie` on every movie), time zone toggles, preset card list, play and return, Back; the title's シーン回想 entry. Remaining: grid scrolling (first four rows), the card list's 自作/他作 sources and sorting, hover frames, `ShortCut` keys, `HelpStates`/`TutorialSeen` use. | Listed above. |
| 2026-11-22 | HScene exit transitions: in `Idle_IN_A`/`Idle_OF_A` the wheel forward resumes the loop and back plays the exit transition (then the second exit state after the male finish, then `Idle_OUT_A`). Remaining: `GetNextHash`'s exact order, pour setups (`PourSetup`, `onPourRestore`), consecutive finishes (`SetupConsecutive`). | Listed above. |
| 2026-11-23 | Character shading from the decoded AL forward programs: HSV shade tone and `_Shadow_power` floor under a key light clamped to 1 (shared `ToonShading.light_limit`/`shade_from_base`), the body md layers (`_Detail_mask_01`, shared `Texture2DArray` decode) as a shadow layer, the eye's sclera/iris/pupil/highlight composition, the cheek/highlight/nose/lip placements in linear space, material texture references resolved against the material's file. Remaining: the face/hair/cloth md layers and the `_Lamp` coordinate shift (the layer applies a fixed strength), the grazing `_Shadow_color` tint, SSS, shine, rim and specular terms, eye gradation/rotation/parallax/unfollow and the eye-look offset moving the baked sclera, the nose gloss/line, the expression blush map, hair gradients and gloss. | Listed above. |
| 2026-11-23 | Hair colours from `_Hair_color_mask` in uv0 (card base/start/end), sRGB masks linearised, hair/clothes shadow tone from the ambient. Remaining: the ambient's hue (scalar here), the gloss band's view-dependent shift, fresnel weight and card `glossSize` (the band is baked), `_Add_skin_color`, the `Ex` colours, hair md layers and the rim `_Shadow_color`. | Listed above. |
| 2026-11-23 | Face md layers as the head's shadow layer. Remaining: the face's `_spmap` nose/lip maxima and the `_Lamp` shift (fixed strength), hair and clothes md layers. | Listed above. |
| 2026-11-23 | Grazing `_Shadow_color` tint of the skin shadow tone (shared `ToonShading.shadow_rim`). Remaining: SSS, shine, rim and specular terms. | Listed above. |
| 2026-11-23 | Clothes and hair md layers; override images follow the importer's texture cap. Remaining: the md layers' `_Lamp` coordinate shift (fixed-strength layers), cloth patterns/matcap (`_TexTAMatcap`, `_Matcap_mask`), skin shine and SSS. | Listed above. |
| 2026-11-23 | Coordinate types (`Swimsuit`/`AfterBath`): night shower movie, greeting movie, `SceneEnd`, ADV `CharaCoordinate`. Remaining: the HScene `Coodenate` menu and `HActor.Dispose` restore. | Listed above. |
| 2026-11-23 | HScene `Coodenate` opens the coordinate card panel (no `SelectCoordinate` in the prefab): empty note, disabled `Deside` through the shared Selectable disabled transition, `Reset` reloads the entry type. Remaining: card listing/preview/sort and `LoadFileLimited` with the part flags (no corpus cards), `HActor.Dispose`'s restore after a type change inside an HScene. | Listed above. |
| 2026-11-23 | Skin light model from the DXBC program (lamp blue channel and coordinate, gate, ambient-mixed tone with the light boost, bluish unlit-side light, no ambient term) and the zone camera lights as the character key light. Remaining: skin base pre-grade, SSS, shine, rim, the md lamp shift, the cloth/hair programs under the camera light, `CmpCorrectLightAngle`, camera-light fades. | Listed above. |
| 2026-11-23 | Card `Graphic` ramp and shadow depth on every AL material; cloth (blue-leaning base shade) and hair (`_Shadow_color`) on the shared light model. Remaining: `_Line_width` outline width, the cloth mask tone, saturated-colour shadow lift, accessory programs, `_UseShadowColor` tone replacement. | Listed above. |
| 2026-11-23 | Expression flush (`_expressionmap`/`_HohoakaIntensity`) through the shared toon blend texture and per-node `base_blend`, from expressions and H breath faces. Remaining: tears, the expression highlight switch, rate transitions. | Listed above. |
| 2026-11-23 | HScene accessory slots one by one (`SelectAccessoryState` rows, `showAccessory`). Remaining: empty-row disabled sprites, `ALL` with per-slot states, list scrolling. | Listed above. |
| 2026-11-23 | ActionPoint activities at placement (`_animIDList` weighted by `al_next_action` row −1, the id's `anim_f` state). Remaining: time-zone variants, follow-up actions, point choice, NavMesh. | Listed above. |
