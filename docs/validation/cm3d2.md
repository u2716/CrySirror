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

## 2026-11-23 — desktop Escape and the CM3D2 1.00 title

The installed 64-bit game supplied the Unity scenes, button colliders, managed
scripts and KISS archives. TitleCtrl registers the title click callbacks,
TitleMouseOver moves the selection cursor and UICamera.ProcessTouch checks
the pressed collider and drag distance before sending OnClick. In mouse mode
the original UICamera clears its current selection on Escape. ConfigMgr closes
the settings panel through OK, and SaveAndLoadMgr closes the load panel through
Cancel.

- The release owner-corpus test runs from a fresh temporary writable directory.
  It boots through both startup clicks, hovers Config, holds without opening,
  drags out and back to cancel, then releases a fresh press to open settings.
  Clicking where Start lies beneath the settings background leaves the title
  intact. Escape leaves settings open. OK closes it. Continue opens the load
  panel, whose Cancel restores the title.
- Reproduce the installed-data test with
  `CM3D2_ROOT="$HOME/Downloads/game-cm3d2" cargo test --locked --release -p crysirror-cm3d2 --features runtime --test owner_corpus title_settings_and_load_cancel -- --ignored`.
- The desktop input regression exercises physical Escape press, repeat, release
  and a second press, for ordinary and Siglus shortcut policies. Other desktop
  tests cover focused text accepting Japanese commits while filtering control
  text, and the host close policy. Live IME candidate windows and physical
  keyboard/window interaction were not observed in the locked session.
- The Mac QA player rendered the full ordinary title/settings/load/cancel route
  headlessly. For AVG32 Kanon, three 640 × 480 offscreen captures at frame 1536
  used identical input except for zero, one or two Escape presses. One press
  changed 107,344 pixels by opening the context menu. Two restored the reference
  image exactly.
- The QA executable in `target/release/crysirror-cm3d2-qa` uses the current
  desktop source and a local validation launcher with Minori omitted. The default
  all-engine build stopped in Minori on missing shared image and audio APIs.
  The normal player binary was not replaced. Local tracks, screenshots, build
  and test logs are under `target/cm3d2-ui/`.

## 2026-11-23 — title input and the day-2 schedule editor

The standard all-engine Mac release linked from this checkout, including Minori.
For this run Cargo used a fresh build-cache directory while publishing to
`target/release/crysirror`. The installed CM3D2 1.00 x64 data was read in place.
Replays used offscreen Metal at 1920 × 1080, muted, with separate writable state.

- Both title UICamera components in the installed `level8` enable mouse,
  keyboard and controller. TitleMgr creates UpArrow, DownArrow and Return
  commands; its methods wrap through Start, Continue, Config and End. No
  invocation of TitleCommand.BaseExecute was recovered beyond its definition.
  CrySirror binds these actions on a new press. The installed-data test checks
  wrap-around, all four positions, held keys, safe confirmation through
  Continue, hidden/modal controls and a moving mouse taking back selection.
- In the standard player, hovering each of the four title buttons changed the
  rendered pixels inside its collider. Keyboard confirmation opened Settings.
  The five previously blank label layers contain `1920x1080`, `X2`, `Medium`,
  `High` and `X1` for this sample without a root config.xml. The shortcut On/Off
  selection changes both buttons' resting tint. Graphics dropdown interaction,
  importing custom graphics settings and applying them to the renderer remain
  open. Physical keyboard, controller, IME and live window tests were unavailable
  while the Mac session was locked.
- The private seed is a slot previously produced through the day-2 player UI.
  Boot through Title → Continue → slot 1 → OK. The daily Schedule button opens
  the roster. The sole leader resists removal; an empty row exposes no task.
  Assign the spare character to slot 2, select a different daytime work, open
  the removal prompt, click the underlying panel OK, then Cancel the prompt.
  Schedule OK returns to the daily menu. Save over the private slot copy and
  reload it through a fresh title session.
- The installed MaidPlate.SetRightPlateVisible(false) uses a 419 × 111 collider
  centered at local x = −140 and drives the LeftPlate Animation on hover.
  Its default clip has five root-scale keys. The replay tests actual pixels on
  pointer entry and selects the character near the left edge of the card.
- The replay uses normal Next and declines optional daytime communication,
  then advances work summaries to the daytime result panel. Assignment and
  the selected work's completion count are checked independently. The native
  per-character result rows still need a presentation implementation.

To reproduce, use Python with Pillow and supply a day-2 daytime slot with the
leader scheduled and one spare character. The output directory must be new.
Every capture has its own input track, state report and PNG.

```sh
cargo build --offline --locked --release -p crysirror-desktop --bin crysirror \
  --config 'build.build-dir="target/cm3d2-ui/day/full"'
python3 crates/crysirror-cm3d2/tests/replay_schedule.py target/release/crysirror \
  "$HOME/Downloads/game-cm3d2" /path/to/day2/SaveData000.save /path/to/new-output
```

On the final release run, character 13 in slot 2 changed from daytime work 3
to work 1 (sewing). Reload restored work 1 with zero prior completions. At
ResultWork the completion count was one. At identical frames, mouse hover
changed 11,176 pixels within the daily Schedule button and 5,984 pixels within
the roster card. The native title-input corpus test, 25 CM3D2 library tests and
8 desktop input/lifecycle tests passed. Local outputs are in
`target/cm3d2-ui/day/accepted/` (roster, edited, reloaded and results) and
`target/cm3d2-ui/day/state-config-labels/`.
