# 甘夏ろけーしょん self-admitted technical debt

## SATD-amanatsu-port

**Scope and current location.** The whole adopt-mode port: asset readers in
`crysirror-unity` (shared) and game semantics in this crate.

**Remaining work.** `ActionMap` beyond the new-game map select,
`ChangeMap`, heroine entry/standing placement, the `ActionMapUI` HUD
(cameras, 2D map, `Skip`) and time-zone advance (tutorial
`ConfirmDialog`, HUD config/help and button fade animations, night events
(`GetNightEventHeroine`/`NightEvent`), the auto save after a zone change,
link-partner placement (`SetCharaForMap` `isLink`), the `MapSelectUI`
hover faces (`_charaInfos`) and player icon (`_pIcon`),
`MapSelectUI` next-zone reloading a map behind it, the light settings'
baked lightmaps (`_colorLightmaps`/`_lightmapIndices`: the renderer has no
second UV set) and reflection cubemaps, Azure's scattering integral,
clouds, moon and stars (the sky is a gradient fit from the Azure hour,
latitude and Rayleigh/Mie gradients), `ActionPoint` choice and `_animIDList` motions, NavMesh
movement, `CharaSelectUI` sources/sorting/pairs/release, scroll views and
`RectMask2D` clipping, `CommandScene` (scenario choice by parameters, command
costs (`Heroine.ConsumptionCost`/`_tglCosts`), the favourability level-up
short movies (`IsNextShortMovie` gates are ported, the `with` movies and
LV increase are not), parameter gauge/max animations, `IsForgetPoint`; talks now play `e1-approach` and one
`e1-N`), ADV expressions/motions/event CGs/cloth states and the
auto/skip/log/config buttons and text reveal,
the map-select title/option/save buttons, map open flags from save progress);
map rendering parity (lightmaps of `ma/lit`, reflection probes, the Azure sky
time of day, Beautify sharpen/depth of field/film grain/vignette and URP
ShadowsMidtonesHighlights (tonemap, exposure, saturation, contrast, colour
temperature and bloom are applied), the PS Vita lite tonemap without colour
grading, `AL/MapColorMask`, water
and particle shaders, `CameraLight` prefabs); the synchronous
`ChangeMap` import (≈1–2 s inside one VM step behind the fade) moving to a
worker under ADR 0050; characters beyond the card-driven assembly in
`CustomScene` (the scene's preset instead of `AL_F_00`, accessories,
`customshape` face/body shape values, makeup/paint/eyelid/mouth layers and
the eye UV layout of the `LIF/*` texture creation, the original toon
ramps (`mt_ramp`), gloss and rim terms, the skin shader graph (the card
`skinShadowColor` → `_Shadow_color` tint is read as colour × alpha ×
`_Shadow_power`; `_Highlight_color`, `_SkinIntensity` detail rate, the
`mt_body_detail` normal map/line mask, `mt_body_shine` and the sunburn,
nipple and underhair layers are not drawn: their textures use separate UV
layouts/positioned decals not in the body's UV0), clothing body cut-outs, blend
shapes/expressions, MagicaCloth depth curves, inertia and MeshCloth
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
and import of saves in the installation's `UserData/save`, `ConfigWindow`, `ExitDialog`
confirmation (quit is immediate), the TOS check/viewer, the language dialog,
Selectable hover/press transitions (`ColorTint`, Animator triggers), the
ILLGAMES `PlayAnimationUI` tweens (LitMotion easing) and the uploader/
downloader; Animator parameter conditions; sound: BGM cross-fades, config
volumes, the title call voice, ENV/3D SE and voices outside the title, the
`UnityPlayer.dll` setup-header scan on a worker; the ADV commands still reported as unported (character, event CG
and H commands), characters (body/face/hair/clothes, colour masks, blend
shapes, bones, physics), maps and H scenes,
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
H scenes and save/load, recorded in a validation note.

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
| 2026-11-21 | `CommandUI` menu on later clicks: greeting, talk/listen/H-topic/with/massage scenarios by family, 戻る; panel hidden during events. Scenario choice by parameters, invite/massage/H scenes and point changes remain. | Listed above. |
| 2026-11-21 | Physics and lighting: shared spring bones (`crysirror_asset::spring`, GLB `crysirror_springs`, renderer pose pass) fed by MagicaCloth BoneCloth; shared `Environment.grading`; map global Volume (Beautify tonemap/exposure/saturation/contrast/temperature, URP Bloom). | Listed above. |
| 2026-11-22 | Body physics: the bone prefab `p_cf_al_body_bone_00` grafted onto the skeleton; its 14 MagicaCloth BoneSprings (bust, hip, upper arm, thigh, belly, waist) tuned from `ShakeCorrectData` by the card softness values and bust weight (`HumanBody.SetSpringSetting`/`ChangeBustGravity`); shared capsule colliders and distance limits in `crysirror_asset::spring`; skin shadow colour from the card. | Listed above. |
| 2026-11-22 | Clothes dressed state (`Table.sKeyNameTable` DEF/NUGE objects, `CmpClothes.objOpt01/02` by `hideOpt`); eyes: card pupil (`mt_eyepipil`) and highlights (`mt_eye_hi_up`) per eye with `CmpEyeLookMaterial` ranges. | Listed above. |
| 2026-11-22 | `ActionMapUI` HUD (map name, move-point cameras prev/next/reset, 2D map back to `MapSelectUI`, `Skip` → `EventManager.TimeZone`: `Cycle.IncrementTimeZone` mod 4 and the map reloaded in the zone's light setting/Volume); `MapSelectUI._btnNextTimeZone`; `ImageSprite.Set` for the zone icons and map illustrations. | Listed above. |
| 2026-11-22 | Zone lighting: the active light groups' point/spot lights as engine lights (map layers 0/4/7/8, URP intensity/range); the Azure hour/latitude/gradients as a shared gradient sky (`crysirror_asset::sky`) uploaded through the new `upload_hdr` host service. | Listed above. |
| 2026-11-22 | Heroine placement per zone (`GetForPlacementTable`/`SetCharaForMap`: shuffled heroines to shuffled maps under `_mapLimitFull`, esthetics/H maps excluded; only the current map's heroines stand there); `PlayBGMBYActionMap` (`_bgmID` per zone) and the `TimeZoneChange` SE. | Listed above. |
| 2026-11-22 | Heroine parameters (`params.rs`: `GameParameter`/`ParameterInfo` level-ups, favourability gauge and `LatePoint`, `PropertyData._param` command points with `IndividualityRate`, `IsNextShortMovie` gates; talk/listen/H-topic points with the chosen choice, command-menu stars/gauges); native saves (shared `crysirror_unity::memorypack`; `save.rs` `SaveData`/`PlayerData`/`NPCData` `.sav` layout), the auto save after a zone change into the engine's save mount and つづきから loading it. | Listed above. |
