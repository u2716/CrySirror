# Custom Maid 3D 2 validation

## 2026-11-21 — remaining menu scenes, system buttons and UI rendering (local, Apple M4 Max / Metal, offscreen, muted)

- Build/tests: `cargo build --release -p crysirror-desktop --bin crysirror`; `cargo test --release -p crysirror-cm3d2 --lib` (23 passed).
- Runs start from the day-2 save (`c1` track, title Continue) and enter labels with the validation-only `CM3D2_DEBUG_JUMP=<file>:<label>` (`CM3D2_DEBUG_FLAGS`, `CM3D2_DEBUG_EVAL` set flags/TJS first).
- `SceneTrophy`: the daily menu's trophy button opens the 49-trophy list (2/49, 25 points; owned in full colour, others greyed), OK returns to `DaytimeMain.ks *昼メニュー`.
- `SceneStaffRoll` from `asa_main_0001.ks *エンディング開始`: credits scroll with fading pictures; Return skips to the KISS logo; a click returns to `*GAME_END` and the post-ending script runs.
- Title: Config opens the config panel (all controls laid out, slider values from the current settings), OK closes it; End halts the session (`yield: Halted`). Start/Continue as before.
- Message window (`M_OP_0002`): clicks advance; BACKLOG shows the last lines with speaker and voice buttons, right click closes it; SKIP and AUTO advance the script by themselves; CONFIG opens the config panel.
- Rendering fixes checked on captures: NGUI images blend on gamma-encoded bytes (the message band has the original's density), sliced sprites keep atlas padding in pixels (config/backlog frames enclose their content), zero-height nine-patch centres are drawn, `UILabel` `spacingX`/ClampContent (trophy labels on one line), unused scroll bars hidden (`showScrollBars`).
- Not established: `SceneSingleEffect` past `startmvpeffect` (the debug path did not reach the MVP branch), `SceneCompetitiveShow`, `SceneDance_DDFL`, particles, maid thumbnails, comparison with captures of the original game.

## 2026-11-20 — save and load from the daily menu (local, Apple M4 Max / Metal, offscreen, muted)

- Behavior: after the first yotogi night, day 2's daytime menu → SAVE → slot 1 → OK wrote `SaveData/SaveData000.save` (86,822 bytes) under `--data-dir`; the panel then listed it as NEW with date, 2 days, 2 maids. LOAD → slot 1 → OK restored it: the KAG resumed at `DaytimeMain.ks *昼メニュー`, ran `reset_scene` and returned to the day-2 SceneDaily menu with the same status (day 2, two maids, grade 1). The first load attempt stopped on the 256 MiB asset cache (generated models were never released); with `release_model` the same track loads without it.
- Title Continue: a fresh boot (clicks at frames 90/260, Continue 650, slot 1 at 720, OK at 760) reaches the day-2 SceneDaily menu of that save by frame 1700; the title also wrote `SaveData-01.save`.
- Not established: loading saves written by the original game (none available), `SceneToTitle`.

## 2026-11-20 — first yotogi night (local, Apple M4 Max / Metal, offscreen, muted)

- Build: `cargo build --release -p crysirror-desktop --bin crysirror`; `cargo test --release -p crysirror-cm3d2 --lib` (19 passed) and the owner-corpus `game_tables_build_and_a_new_maid_initializes` (13 stages, 92 skills with 664 commands, 3 wait-motion personalities, 12 undressing units).
- Behavior: a headless input track continuing the first-day run selects the Salon stage, picks four skills, plays commands of each and finishes the result: SceneYotogi StageSelect at frame 87380, SkillSelect 88852, Play 89436 (skills 10/20/30/30), Result 102260 (yotogi class level +1, EXP +224, 淫欲 +52, 変態 +21; three skill units), then the ADV script returns to `SceneDaily`. Captures were inspected for the stage list, skill select, play (bars, commands, tower, undressing, translucent man, characters posed by the skill files) and result screens.
- Not established: comparison with the original game's captures, audio (muted), save/load, the other night scenes.

## 2026-11-20 — first in-game day (local, Apple M4 Max / Metal, offscreen, muted)

- Run: the opening track continued by a replay driver (session scratchpad `drive.py`) that presses SceneEdit OK → 名前で呼ぶ → confirmation, the last choice of each menu, `DailyPanel/Next` and result `Ok`, and otherwise advances text; 85,186 frames.
- Observed: second maid created by `CreateNewMaid` and edited, her interview, `SceneStartDaily` (days eyecatch), the daytime menu (`StatusViewer` with salon figures, funds and days), Next, 「コミュニケーションを行わない」, `SceneDaily ResultWorkDaytime` (work results simulated), the night menu, then `YTG_main_0000` → `SceneCall SceneCharacterSelect type=Yotogi`, which is not ported (session error says so).
- Corpus test `game_tables_build_and_a_new_maid_initializes` (ignored, needs `CM3D2_ROOT`): 49 trophies, 129 shop items, 5 salon grades, 92 yotogi skills, 12 noon and 13 night works; a new maid starts with 16 skills and the Novice class.
- Not exercised: schedule editing, communication, yotogi, income result (reached only after the night), saves.

## 2026-11-20 — maid creation through the interview to the first day (local, Apple M4 Max / Metal, offscreen, muted)

- Run: `play ~/Downloads/game-cm3d2 --headless --no-audio --frames 60200 --input-track <opening advances, SceneEdit OK (1818,995), 名前で呼ぶ (1088,626), confirmation OK (1162,640), then clicks alternating (967,555)/(967,610)>`.
- Observed: SceneEdit's OK opens `FFNameDialog`, then 「このメイドにしますか？」; OK ends the scene and `M_OP_0002`, `C_MEN_main_0001` (interview) and `M_OP_0003` run. The new maid carries a name from `MaidRandomName` of the installed Assembly-CSharp; the interview menu shows the cup question from `UpdateBodyParam` (美乳), the experience question and 「質問を終わる」; the run ends at `AllMain.ks *ASA` → `SceneCall SceneStartDaily`, whose controller is not ported (session error says so).
- Diagnosed and fixed in this run: the `choicesshow` macro body from `ADVKagManager.Initialize` kept its leading tabs, so `@_choicesshow`/`@s` were read as text and the script fell through into the first choice; `crysirror-kag` now records host macro bodies like scenario `[macro]` bodies. NGUI labels inside instantiated buttons anchored to the first copy and to the other scene.
- `cargo test --release -p crysirror-cm3d2 --lib` (16 passed), `-p crysirror-kag --lib` (55 passed), `-p crysirror-krkr --lib` (12 passed).

## 2026-11-20 — characters and room in toon shading (local, Apple M4 Max / Metal, offscreen, muted)

- Run: `play ~/Downloads/game-cm3d2 --headless --no-audio --frames 2300 --input-track <title Start, name entry OK, two advances>`.
- Observed at `M_OP_0001:46 *L1` (`place.ks *執務室＿叔父`): the uncle's maid in `kaiwa_tati_taiki_f` with her full body, blue `HAIR` parts colour, face, eyes and clothing in the toon model with thin outlines; the uncle seated in `suwari_fuka_m` with his head attached to `ManBip Head`, as the orange `CM3D2/Man` silhouette; the `Shitsumu` room through the same toon model. No session error.
- Diagnosed and fixed in this run: body triangles weighted to bones hidden by other slots (`node消去`) are dropped instead of scaling bones; head parts ride the head bone; the face's `_ShadowColor` only blends at the light's 0.1 shadow strength.
- Differences remaining: man self-overlap (no GrabPass), and the items in SATD-cm3d2-rendering. Not compared side by side with the original.
- Unrelated failures seen while testing: `crysirror-scene` `input_routes` and canvas/layer integration tests (`canvas_draw_subtree`, `canvas_subtree_snapshot`, `layer_matrix`, `layer_visual_motion`), `crysirror-image` `tests/effects.rs` compile, and other crates' test builds (`crysirror-system-nnn` `native_move`, `crysirror-bgi-integrated`), all in code this work does not touch.

## 2026-11-20 — title Start into the opening scenario (local, offscreen, muted)

- Run: `play ~/Downloads/game-cm3d2 --headless --no-audio --frames 1000` with clicks at frames 90, 260 and on START (340, 643) at 560.
- Observed: title Start fades out 3 s, `SceneMgr.CloseScene` jumps to `*title_end`, `reset_scene.ks`, `CameraControl`, `MessageWindow on`, `M_OP_0001.ks *top`: `fade3d out sync`, `fade in`, `talk`, text, `hitret`. The capture shows the message window with 「主人公の名前を決めて下さい。」 in the game's embedded font and the SKIP/AUTO/VOICE/BACKLOG/CONFIG buttons; diagnostics `M_OP_0001:20 *L0`, no error. BGM001 and system SEs decode through shared audio (muted output).
- Not exercised: the name entry (`SceneUserEdit`), backlog, skip/auto, voice.

## 2026-11-20 — boot to title (local, Apple M4 Max / Metal, offscreen, muted)

- Build: `cargo build --release -p crysirror-desktop --bin crysirror`.
- Run: `crysirror play ~/Downloads/game-cm3d2 --headless --no-audio --frames 600 --input-track <two clicks at frames 90 and 260> --capture title.png`.
- Observed: detection mounts the 14 present archives in `GameUty.Init` order;
  `SceneLogo` (KISS logo), `SceneWarning`, `AllMain.ks` → `reset_scene.ks`
  (`SceneCall SceneADV`, `wait`, `MotionBlendEnabled`, `return`) →
  `SceneCall SceneTitle`. The captured title shows the NGUI logo, the four
  buttons with the START cursor, the system shortcut bar and the
  `Salon_Entrance` 3D background through the shared PBR renderer. Session
  diagnostics: scene `SceneTitle`, KAG `AllMain.ks:20 *title`, no error.
- Not exercised: audio (muted run; BGM is not implemented), title buttons
  other than the hit test, any scene after the title.

## 2026-11-20 — shared format corpus

- `CM3D2_ROOT=... cargo test --release -p crysirror-cm3d2 -- --ignored`:
  427/427 `.model` and 3058/3058 `.anm` convert and load as engine models;
  44/44 `.nei` tables decrypt and parse.
- `UNITY_DATA=.../CM3D2x64_Data UNITY_SCHEMA=.../unity_schema.json cargo test --release -p crysirror-unity -- --ignored`:
  41 serialized files; all GameObject, Transform, MonoScript, Material,
  Camera, Texture2D (decoded), Font (51 embedded TrueType), Mesh (1264),
  MeshRenderer, MeshFilter and Shader objects read; 17461 MonoBehaviours decode
  under the generated schema with every byte consumed (scripts of
  Assembly-UnityScript-firstpass are not in the schema).
- Offscreen render of `body001`/`face003`/`hair_f001` with `maid_stand01.anm`
  retargeted to the body and the parts following its skeleton (project under
  the session scratchpad, not distributed).
