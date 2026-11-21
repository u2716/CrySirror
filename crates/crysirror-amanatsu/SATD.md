# 甘夏ろけーしょん self-admitted technical debt

## SATD-amanatsu-port

**Scope and current location.** The whole adopt-mode port: asset readers in
`crysirror-unity` (shared) and game semantics in this crate.

**Remaining work.** `ActionMap` beyond the new-game map select and
`ChangeMap` (tutorial `ConfirmDialog`, `ActionMapUI` HUD and its camera
position switching, time zones and `Skip`, heroine placement, `CommandScene`,
the map-select title/option/save buttons, map open flags from save progress);
map rendering parity (lightmaps of `ma/lit`, reflection probes, the Azure sky
time of day, URP global volumes / colour grading, `AL/MapColorMask`, water
and particle shaders, `CameraLight` prefabs); the synchronous
`ChangeMap` import (≈1–2 s inside one VM step behind the fade) moving to a
worker under ADR 0050; characters beyond the card-driven assembly in
`CustomScene` (the scene's preset instead of `AL_F_00`, accessories,
`customshape` face/body shape values, makeup/paint/eyelid/mouth layers and
the eye UV layout of the `LIF/*` texture creation, the original toon
ramps (`mt_ramp`), gloss and rim terms, clothing body cut-outs, blend
shapes/expressions, hair/cloth physics, height blending between the
`_S/_M/_L` clips, controller transitions, characters on maps); the
`CustomScene` character maker UI (instantiated from `lib/custom` prefabs at
runtime; only the background is drawn) and `Memory` scene controller; on the title: save data (the
Memories entry, `SaveLoadUI` for つづきから), `ConfigWindow`, `ExitDialog`
confirmation (quit is immediate), the TOS check/viewer, the language dialog,
Selectable hover/press transitions (`ColorTint`, Animator triggers), the
ILLGAMES `PlayAnimationUI` tweens (LitMotion easing) and the uploader/
downloader; Animator parameter conditions; sound: BGM cross-fades, config
volumes, the title call voice, ENV/3D SE and voices outside the title, the
`UnityPlayer.dll` setup-header scan on a worker; the ADV command interpreter (45 command
kinds in use), characters (body/face/hair/clothes, colour masks, blend
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
