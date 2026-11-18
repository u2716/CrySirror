# Malie: Silverio Ragnarok (2026-11-18)

Owner-authorized local install; no game data is in the repository.
Host: Apple M5, macOS, Metal, release build, `crysirror play <dir>
--headless --no-audio` with scripted input tracks.

Implementation, compilation and behaviour are separate:

- Compilation: `cargo check/test --release -p crysirror-malie -p
  crysirror-video` pass (message tokenizer, animated picture matrices,
  WebM/VP8 fixture tests).
- Title: logo, title tweens and the menu render after ~1800 frames;
  Start (881,333) enters `main@adv`.
- Story: with a click every 12 frames, 40,000 frames reached label `r_1a_a2`
  (247 cleared pages) without an adapter error, through the first
  three-way choice (115-117), novel and normal message frames, character and
  cut-in planes, exported eye-open animations, BGM/SE/voice requests,
  `imgblk` transitions and `layerwait type=MOVIE`.
- Movies: in-scene WebM/VP8 backgrounds and chapter titles present through
  the shared Media service. In accelerated headless runs decoding lags the
  simulated clock; waits complete by the media clock. Real-time window
  playback was not observed.
- Recorded gaps reported by diagnostics on this route: `<ef>` effects 1/3,
  split-screen camera 1.
- Not exercised: audio output (runs use `--no-audio`), window/input on a real
  display, save/load and settings dialogs (not implemented).

## 2026-11-18 (later): full native surface, dialogs and saves

- Compilation/tests: `cargo test --release -p crysirror-malie -p
  crysirror-video` pass (including a libvpx alpha WebM oracle);
  `cargo test --release -p crysirror-engine --test api_docs` passes.
- Story route: 40,000 frames (click every 12) reach `r_1a_a2`, 244 pages,
  no adapter error and no recorded capability gap.
- Split camera: `layercamera id='1'` scene at engine time ~459 s shows both
  cameras divided by the −20° splitter.
- Dialogs: right-click opens the config shell (game page); text, sound
  (with the preview movie), voice, load, save and backlog pages render with
  native option states, sliders and listboxes. Save to slot 001 writes a
  record with date and thumbnail; a second save shows the native overwrite
  prompt; load from slot 001 closes the dialog and resumes the page.
- Not exercised: extra/award screens (title Extra stays disabled until
  `g_ex` is set by play), scene replay, quick load prompt, auto/skip timing,
  real-time window playback and audio output.

## 2026-11-18: native screen/frame failure paths

- Screen ids, frame item 201, MalieSystem 120, ADV messages, `layercmd` and
  dialog names now follow 52D470/4FA860/4FAA30/6124D0/50C000/613800/52D7F0/
  613260 (see the crate README); `cargo test --release -p crysirror-malie`
  passes.
- Story route (40,000 frames) unchanged: `r_1a_a2`, 244 pages, no gap, no
  error. Backlog track still opens the backlog dialog.
- Not exercised: the newly connected ADV messages (no script path sends
  them in this route), `screen src='map'` (LUN_MAP not reached).

## 2026-11-18: remaining native paths and the script compiler

- Compiler: `cargo test --release -p crysirror-malie --test compiler` passes
  (self-authored sources run on the VM: control flow, calls of both
  conventions, macros, includes, messages, Shift-JIS `.txt` and zlib `.mls`).
- Comparison against the shipped exec.dat (local only, not committed):
  declarations generated from its symbol table plus reconstructed bodies
  compile to identical instructions for CLEAR_BATTLE, Malie_GetConfig,
  _ms_overlap, MALIE_CJUMP_CMP, MALIE_NAME, MALIE_NAME_NUM, MALIE_CHAR,
  calender, _change_screen, _obj_ALL_CLEAR, run, another_mode and all
  496,600 instructions of maliescenario; the message section (2,622,786
  bytes, 13,878 ranges), symbols, global size and link table are identical.
- Story route (40,000 frames, three runs): `r_1a_a2`, 243–245 pages
  (timing variance), no gap, no error. Backlog track unchanged.
- Not exercised: ADV messages 1001–1102 and screen 1000/12346 switches (no
  script in this profile sends them), module switching at run time (no
  source file ships), the empty modal frames.
