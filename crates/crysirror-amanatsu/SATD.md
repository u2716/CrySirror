# 甘夏ろけーしょん self-admitted technical debt

## SATD-amanatsu-port

**Scope and current location.** The whole adopt-mode port: asset readers in
`crysirror-unity` (shared) and game semantics in this crate.

**Remaining work.** Scene controllers for `ActionMap`, `CustomScene` and
`Memory` (shown laid out, without logic); on the title: save data (the
Memories entry, `SaveLoadUI` for つづきから), `ConfigWindow`, `ExitDialog`
confirmation (quit is immediate), the TOS check/viewer, the language dialog,
Selectable hover/press transitions (`ColorTint`, Animator triggers), the
ILLGAMES `PlayAnimationUI` tweens (LitMotion easing) and the uploader/
downloader; Animator parameter conditions; sound (BGM, SE, brand/title call
voices: FSB5 `AudioClip` data); the ADV command interpreter (45 command
kinds in use), characters (body/face/hair/clothes, colour masks, blend
shapes, bones, physics), maps and H scenes,
saves (`UserData/save`), character cards; TextMeshPro Alpha8 SDF atlases
beyond the RGBA decode limit; meshes with `m_MeshCompression`.

**Closure evidence.** Installed-game runs through the title, ADV, map and
H scenes and save/load, recorded in a validation note.

| Date | Change | Remaining scope |
| --- | --- | --- |
| 2026-11-21 | Shared Unity 6 readers (bundles, type trees, generated MonoBehaviour trees, textures incl. BC4/BC6H/BC7, meshes with skinning and blend shapes); ADV scenario decoding. | Everything listed above. |
| 2026-11-21 | Boot flow Init → Logo → Title with the scene fade; shared Mecanim playback (`crysirror_unity::animator`), UnityEvent persistent calls and uGUI raycast/interactable inheritance; title menus, Back/right click, scene loads and quit ported from `TitleScene`. | Listed above. |
