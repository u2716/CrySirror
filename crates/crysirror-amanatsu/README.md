# crysirror-amanatsu

Onboarding mode: **adopt** (ADR 0102). 甘夏ろけーしょん (ILLGAMES
`AmanatsuLocation`, Unity 6000.0.65f1, IL2CPP). Game logic is ported from the
original program (method bodies from `GameAssembly.dll` under IDA, type and
method names from `global-metadata.dat` through Cpp2IL); all assets are read
from the user's installation through `crysirror-unity` (UnityFS bundles,
serialized format 22, type trees). No game data, metadata dumps or code are
distributed.

## Implemented

- `adv.rs`: ADV scenarios — `ILLGAMES.ADV.ScenarioData` MemoryPack bytes of
  the scenario MonoBehaviours in `lib/adv/scene` (`ScenarioCommand`: hash,
  version, multi, `Command`, string arguments; the `Command` names).

- `unity.rs`: the installation through client storage — bundle/player file
  keys, the type-tree registry (bundles plus generated MonoBehaviour trees),
  sprites, TMP font fallbacks, build scene names.
- `level.rs`: a build scene — hierarchy, script-driven uGUI state, Animator
  playback (shared `crysirror_unity::animator`), `Button.m_OnClick`
  persistent calls (built-in targets run here, `TitleScene` methods go to the
  scene controller), `Selectable.interactable`, and the
  `Manager.Scene.SceneFadeCanvas` fade (`_time` 1 s from the Addressables
  manager prefab).
- `scenes.rs`: `InitScene` (fade colour, `_loadLevel`), `LogoScene` (fade
  out, `_waitTime`, click skip unless `_clickWait`, `SceneSettings.Load`),
  `TitleScene` (+ `SelectMenuAnimation`, `PressAnyAnimation`): press-any →
  menu, `OnGamePlay`/`OnCustom`/`OnNetWork` sub menus, `OnBack` (Back button
  calls, right click), `OnStart`/`OnCustomMale|Female`/`OnMemory` scene
  loads, `OnQuit`/Escape quit, F1 config. `SceneSettings_00_Master` ids map to
  build scene names.
- `tables.rs`: `AL.Serialization.TableData` (MemoryPack TextAssets: column
  keys, rows of strings), e.g. the `sd/lis` `bgm[…]`/`sse[…]` lists.
- `sound.rs`: `Manager.Sound`/`AL.Utils.Sound` — `DicBGM`/`DicSystemSE`,
  `Loader` (bundle, asset) → AudioClip → FSB5 → shared decoder stream on the
  host's channels (BGM looping); Vorbis setup headers from the installed
  `UnityPlayer.dll`. Title BGM, menu SE (`PropertyData._sseID`) and the logo's
  random `sd/dt/bc` brand call play.
- `adapter.rs`: the session — boot, scene loads after the fade-in, frame
  layout and layer submission, Button click dispatch (press and release on
  the same interactable handler).

## Validation

`AMANATSU_ROOT=<game> cargo test --release -p crysirror-amanatsu --test owner_corpus -- --ignored --nocapture`:
2,678 scenarios (35,224 commands, 45 command kinds in use) decode; textures
and meshes of `lib` decode through the shared readers (see
`crysirror-unity` README). Scope still to port: [SATD.md](SATD.md).
