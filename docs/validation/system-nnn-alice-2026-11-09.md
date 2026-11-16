# System-NNN MUGEN2 and System4 Rance7 bring-up, 2026-11-09

Targets: owner-authorized `game-MUGEN2` (MSI media, System-NNN) and
`game-rance7` (System 4.0 media). Apple M5, release builds, offscreen
`crysirror play --headless --no-audio`. Original files were read in place.

## System-NNN implementation

- Installed asset reads use Win32 name semantics: exact name, then the
  unique ASCII case-insensitive name in the same capability
  (`native_resources::open_installed_asset`). MUGEN2 ships
  `init2/printmessage.fxf`; it was previously treated as absent, so message
  text used constructor defaults (printX/Y 100) instead of 79/418.
- HAIKEIYURE100 (MUGEN2 VT 0x567334: Set 0x4B7D00, Calcu 0x4B7C00, Print
  0x4B80E0, MoveScreen 0x4B7E90) is a SHAKE-family state; the whole-screen
  profile (para5 1, para11 0) is one shared Composite copy. The continuation
  below also connects picture-layer and native speed-curve profiles.
- LAYERSEPIA128 (Set 0x4A7500, Print 0x4A7630, MMX kernel 0x4D7EF0) uses the
  LAYERGREY curve and the shared `FixedLumaSpec` with a per-channel tint.
- Ruby `#［N…］` (pinned SystemNNN mymessage.cpp/myfont.cpp; MUGEN2 game
  setup rubiFlag1, percent43, deltaY-3, sukima0) is parsed from message rows
  and placed by the MakePic centre/clamp/leftLimit rule, in the shared body
  glyph run with per-glyph size/colour. Each ruby glyph is revealed on the
  tick of the base cell under its x (native reveals by pixel column).
- Diagnostics expose the pending native call and the dependency owners.
- The continuation below connects declared text shade colours and the native
  inline command table; the initial game run did not exercise those additions.

## System4 implementation

- AliceLogo2.Init accepts Rance7's declared IMainSystem(26) argument; the DLL
  export 0x10007310 is cdecl without parameters.
- Confirm2 (DLL SHA256 111b56f4…98ed) is implemented from the DLL: key file
  of magic 0x11223344, 0, volume serial 0, key^0x52 through the Save
  capability; CheckProtectFile reads one byte of the named protect file at a
  generator-chosen offset. The generator matched 307/307 sampled bytes of the
  owner's `戦国ランスDM.DAT` (key 1215). No drive scan or key search exists.
  Owner policy (2026-11-09): when the protect file is unreachable from the
  client capability (the media root is outside `GAMEDATA`), the check passes;
  a reachable file still gets the original byte comparison.
- SACT2 Joypad_ClearKeyDownFlag/IsKeyDown/GetNumof/JoypadQuake_Set implement
  the `UseJoypad = false` profile (no pad device); an enabled pad fails.
- Examples: `crysirror-system4` `ain_libraries` (HLL declarations) and
  `ain_function` (function disassembly); `crysirror-resource`
  `installer_list` (MSI logical files).

## Behavior evidence

- MUGEN2: title at about 15000 frames; NEW GAME into `preprologue`/`prologue`.
  Before: text at the window top, then failures at frame 31816 (SETEFFECT
  100), 46901 (SETEFFECT 128), 48537 (ruby markup). After: a 250000-frame
  click-through ended without error in `prologue` at 13579 instructions with
  correct window text, name plate and ruby rows.
- Rance7: previously stopped at frame 970 (AliceLogo2.Init ABI), then 2280
  (Confirm2.Init), then 3369 (Joypad). Now the logo, disc/key check and the
  opening movie run (the opening is unskippable: tagMovie@run only tests
  keys when its skip argument is set). The run then stops at frame 42829:
  SACT2.SP_SetDrawMethod(1029, 1) SCREEN, which still requires the original
  SACT2 blend arithmetic.

## Tests

- Initial `crysirror-system-nnn --features runtime --lib`: 74 passed, 7 failed; the
  same 7 fail with these changes reverted (71 passed).
- `crysirror-system4 --lib native_confirm`: 2 passed (disc-byte test with
  `CRYSIRROR_RANCE7_PROTECT_FILE`). `--features runtime --lib`: 36 passed,
  16 failed; the same 16 fail with these changes reverted (34 passed).
- `crysirror-image --lib fixed_luma`: 3 passed.

## Not validated

Windowed play, audio output, MUGEN2 beyond `prologue`, selections, saves,
Rance7 past the opening movie.

## System-NNN continuation

Implementation: MML segment/repeat policy now uses a shared worker-owned audio
playlist. HAIKEIYURE100 picture/whole-screen branches and native curves use shared
image composition. Native inline colours, name substitutions, ruby, built-in
gaiji and shadows use shared text/image glyphs. Quick slots retain comment
fields, use the native Notice queue and optional pre-load capture. Timed choices
retain resolved controls when saved again. Idle-title restoration and saved
last-screen effects use a detached shared Layer candidate before atomic commit.
Neither preparation nor resource polling advances original effect counters.

Compilation: release System-NNN runtime check passed. The final release player
build passed with `cargo rustc --release -p crysirror-desktop --bin crysirror -j4
-- -C llvm-args=-import-instr-limit=0`; its `--help` invocation exited successfully.
No original EXE/DLL was executed.

Scoped offline behavior: `cargo test --release -p crysirror-system-nnn --features
runtime --lib native_ -j4` passed 71, failed 0, ignored 2, filtered out 14. The
post-text restore check uses an explicit fake presenter to verify dependency
ordering, retained counters and unchanged live state; it does not establish GPU
or font rasterization parity. The shared playlist checks passed 2/2, including
PCM segment boundaries and large seeks through infinite repetition. The exact
nearest resize check passed 1/1. The shared detached-Layer check passed 1/1:
candidate coordinates and ownership are retained, readback runs on the worker,
and the live scene remains unchanged. It also checks nearest resize through the
shared worker, including the resized resource's output dimensions. Ignored
font/media checks were not run.

No additional MUGEN2 game session, physical audio, real-GPU capture or workspace
regression was run in this continuation. Configured special choice layouts,
film-skip UI reconstruction, Omake/Cutin/extension save actors and outer save
modes beyond message/selection remain explicit limits. Normal title Load/Config/
Omake menus and dynamic user-gaiji font switching are not completed by this batch.

## System-NNN Cutin continuation — 2026-11-10

Implementation: character-table coordinates, font deltas, speaker suppression,
signed Cutin mode and live save-row records now use shared text/Layers. Row views
share the source GlyphRun's reveal clock. Solid windows and asynchronously
prepared image windows retain their native row ordering and integer mask rules.
The source's out-of-array writes after suppressing a speaker are normalized to
retained rows, with the difference marked in code. Groups depending on hidden
rows were not yet supported in this initial Cutin batch; the continuation below
retains that native RAM separately from visible rows.

Compilation: release System-NNN runtime and wgpu checks passed. The desktop
player was not rebuilt in this continuation.

Scoped offline behavior: three Cutin policy/layer/save-seed checks, one existing
optional-callback receipt check, two shared glyph-range checks and one shared
mask-versus-bitmap check passed. Naga parsed and validated the updated quad
shader on the CPU. No game, GPU, audio device or workspace test suite ran.

Remaining work: native menu background scripts and extended actors, Omake
galleries, configured film-skip buttons, saved outer modes beyond PRINT/SELECT
and dynamic user gaiji switching. These results do not establish complete
System-NNN or MUGEN2 gameplay compatibility.

## System-NNN retained rows and menu readers — 2026-11-10

Implementation: the printer now retains 64 message RAM rows separately from its
visible rows. Cutin group widths and LPRINT scrolling use that full RAM; CLEAR
retains it, while original saves keep the source writer's visible-row-only
MESSAGE payload. Configured film-skip buttons use scoped shared UI, hit masks,
prefix capture and masked image jobs. Selection preserves native click delays,
sound queues and F5/ENFILM state, and cancels a DRAW countdown only after its
already-admitted frame finishes.

Modal Load/Save/Config background SPT readers now share native variables, RNG
and the single audio owner. Reader switches suspend native effect/resource
banks and logical Layers/UI, retaining the foreground PRINT/SELECT receipt.
Title menu return reloads its configured background script rather than resuming
a cancelled countdown. Non-looping background effects keep painting/counting
after NEXT without creating an instruction or acknowledging a fake callback.
The detached restore boundary also accepts an idle calculated Title mode.

Compilation: the release System-NNN runtime library check passed. The desktop
player was not rebuilt in this continuation.

Scoped offline behavior: the release runtime test binary passed five Cutin
checks, three film-button checks, two modal-reader checks (each covering native
modes 5/6/12) and one retained-background check. The latter checks unchanged
guest PC/instruction counts and verifies that dependency pumps do not advance
effect counters. Modal checks cover effect-bank restoration, original receipt
retention, shared RNG/globals and Title-reader reload. Shared capture uses an
explicit fake presenter, so these results do not establish GPU output parity.
No game session, audio device, GPU or workspace suite ran.

Omake galleries, automatic mode-owned extension actors, saved outer actors
beyond PRINT/SELECT and dynamic user-gaiji switching remain incomplete. Generic
modal controls do not reproduce native menu skins or every General-mode hook;
background scripts requesting a foreground Message/Selection transfer still fail
explicitly. Complete System-NNN compatibility is not claimed.

## System-NNN Omake and native extension continuation — 2026-11-10

Implementation: Omake now routes CG/scene catalogues, music and movie selection
through the same scoped VM/resource/audio owner. Its root uses original button
atlases and shared modal widgets. Catalogue controls use shared UI. Original
CG unlock bits, difference groups and signed `scene.dat` script/film values are
retained; scene films execute in the native foreground reader and ENDFILM moves
to the next real label-table target before returning to the suspended Title.
Native button variants copy the preceding row's same column without positional
deltas or accelerator inheritance. Variant selection reads stored variables,
without drawing RNG values. Extended buttons route connected modes or write
their signed scenario return value after native NewGame initialization.

Static native review found empty automatic EXT callbacks for the admitted
standard mode classes; those now retain their zero-sized blocks ahead of manual
EXT blocks. ChartMenu has actual road/selection buffers and remains rejected.
The envelope permits the original combined automatic/manual block counts.

ListenBGM consumes the original eight-token catalogue by filename, even when a
track has no script BGM-table entry. The shared audio owner handles native loop,
volume and fade parameters. Direct playback clears original script music IDs
instead of synthesizing one. Initial stop, selection/end/exit state and the
`listenbgmmode`/`listenbgmnumber` variables belong to the native modal reader;
updating them does not advance its suspended instruction.

`NativeAdapter::change_user_gaiji` implements the two-byte native extension
interface as scoped resource work plus shared key/lookup/resize/reduction jobs.
Font generations are retained for existing GlyphRuns/snapshots, with admission
bounds, cancellation and failure cleanup. The source's base SPT dispatcher has
no corresponding opcode; none was invented.

Compilation: release runtime and no-default-feature System-NNN library checks
passed. The release desktop player was rebuilt successfully; its command-line
help also starts successfully. These are compilation/startup results, not game
compatibility evidence.

Scoped offline behavior: seven selected release library checks passed: menu
variants/extended action selection, two modal-reader checks, actual scene film
execution/return, signed scene catalogue parsing, user-sheet cell/blank-image
preparation, and native extension layout. The existing shared-frame/widget
integration check was exercised beneath an engine modal parent and passed.
Three selected audio-owner checks also passed, including direct catalogue
playback with an empty script BGM table, single-pass versus looping transport,
original stop envelopes, scoped system sounds and client-isolated teardown.
The two modal-reader checks additionally verify listen-state variable writes
survive reader swaps without consuming the suspended native receipt or RNG.
No whole-workspace suite, game session, GPU or audio-device test ran.

Remaining boundaries: ChartMenu's dynamic extension actor, original saved outer
actors beyond PRINT/SELECT, CG animation/scroll/tile and voice profiles, full
catalogue/settings/slot skins and remaining General-mode fade/voice hooks. These changes do not complete
the entire System-NNN engine or certify the owner's complete game.

## System-NNN CG view and ChartMenu EXT — 2026-11-10

Implementation: the CG viewer now resolves the original animation/scroll
headers, unlocked catalogue wraparound and single-set return policy. Immutable
frame/tile resources are read on the session pool and decoded by shared Image
jobs. Shared Layers retain native origins, viewport clipping, tile positions,
scroll waits/durations, Ctrl-dragging, stored-byte masks and animated visibility.
Resource preparation and modal audio completion do not consume native frames.
Uncovered pixels use a deterministic configured background instead of depending
on a previous uninitialized screen allocation. Unsupported difference wipes
and General entry profiles still fail explicitly.

CG voice thresholds, delayed triggers, stop markers and navigation sounds route
to the existing shared audio owner. Replay replaces voice channel zero without
rewriting the scenario's saved voice-name/loop-work records. Global voice-off
returns before replacing the current voice; character-specific replay controls
remain outside this profile.

ChartMenu's automatic EXT layout is prepared from its own decoded setup. Static
review of the legacy constructor and save/load callbacks agrees with the public
source's aligned selection-word/road-byte buffers. Fresh selections retain
all-one padding; roads start at zero. Repeated automatic blocks load in native
file order, and manual fallback blocks still clear independently. Original-slot
restore retains the configured actor layout; NewGame clears roads without
resetting selection permissions. The configured `sloton` function runs through
an actual VM receipt and rejects invalid slots before advancing its PC. The
full ChartMenu map/editor and other ChartMenu functions are not implemented.

Scoped offline behavior: three CG checks, two ChartMenu checks and three shared
audio-owner checks passed in release mode. They cover real shared image jobs
and layer state, animation hold/pause timing, scroll/drag geometry, delayed
voice events, native EXT bytes/padding/load order, real VM acknowledgement and
error retention, audio decoding, loop policy and client-scoped cleanup. No
game, window/GPU, audio-device or whole-workspace test ran.

Compilation: release runtime/no-default-feature library checks and the named
native_title/native_shell/native_movie integration targets compile. The release
desktop player was rebuilt successfully and its command-line help starts.
Checking
all System-NNN test targets remains blocked by pre-existing MoveFrame literals
in tests/native_move.rs missing native_command; that test was not modified or
executed by this work.

## System-NNN Chart actor and shared wipes — 2026-11-10

Implementation: Chart maps/functions, progress files and original compressed
checkpoints now use scoped storage/tasks, shared image jobs, Layers and UI.
An adjacent choice resumes its retained native receipt; checkpoint replay
reconstructs original reader contexts and retains current global variables.
Native status preflight precedes BGM replacement. Layer gates, window variants
and raw audio records are restored without replaying unrelated voice channels.
SystemMenu connects native button images, intro counters/RNG, extended mode
gates, save availability, quick slots and Title return. Menu resource completion
cannot run another Chart/background batch or advance native animation clocks.

Shared integer image wipes connect native selectors 0–23, 40 and 69–77 to
OVERLAP and the connected General exit player. Character sepia 127 reuses shared
fixed-point luminance/tint and mask composition, including original save words.
SUNA omitted parameters use initialized defaults, with the native uninitialized
read explicitly identified in code. Result unlocks enqueue the original Notice
index instead of rejecting enabled notices. System UI sound numbers remain
one-based through the adapter/audio seam.

Compilation: release System-NNN library checks passed with and without the
runtime feature. Public API artifacts were regenerated successfully.

Scoped offline behavior: six Chart/checkpoint checks, the SystemMenu intro
check, shared sentinel-run and integer-line checks, the shared integer-wipe
check, three General return checks, button delay/NULL-input checks, SUNA default
checks and the result-notice check passed. The result-notice check used the
already compiled release unit binary. No game, window/GPU, audio-device or
whole-workspace suite was run in this continuation.

These results do not establish a complete System-NNN port. Remaining boundaries
include other native modes/effect families, unconnected wipe selectors, original
Chart tab/dialog skins and editor persistence, complete catalogue/settings/slot
skins and General entry/exit voice and retained-enter-screen profiles. No new
desktop executable or full-game behavior is claimed by this continuation.


## System-NNN complete wipe table and buffered effects — 2026-11-10

Implementation: native wipe selectors0–95 now map to shared integer,
scanline/shape/scale, fragment and explicit-random-sample operations. Native
SIMPLEWIPE59 uses the same table as OVERLAP and connected General exits.
GET73/PUT2 retain immutable engine captures; BUFFER66 retains its image without
changing pixels; ONCE75 carries its actual first-print flag. STRETCH36 uses
packed reduction and independently quantized axis sampling. LIGHT20,
GRADATION50, COLORBALANCE120 and ADDSUB131 use shared Image colour operations.
Their original parameter arrays and count/countMax records restore separately.
Common XY calculation includes hidden picture effects and precedes native
picture-index lookup. General random wipes commit each sampled RNG candidate
once, allowing background readers to continue from the latest owner state.
Undefined source stack words, initial GET storage and out-of-range shape writes
are explicitly initialized/clipped in code rather than emulated as heap data.

Compilation: the release System-NNN library check with `runtime` passed after
these hooks were connected. Public API generation completed successfully.

Offline behavior: all96 selectors were exercised over tiny immutable image
pairs, including shared random-table lifetime. Four focused native checks passed:
GET/PUT/ONCE and buffered-wipe painter order with original counters; hidden
coordinate/target ordering; LIGHT/GRADATION word and row bounds; and the stored
COLORBALANCE tonal table discontinuity. These checks use no game assets, font
files, original executable, window/GPU or audio device. They do not establish
complete System-NNN coverage; other effect families and outer actors remain.
The focused shared-sampling check also passed, covering weighted2x2 reduction,
independent fixed-point axis phases and partial-mask saturated add/subtract.

## System-NNN default user commands — 2026-11-12

Implementation: MUGEN2 `printmovie` and `selectplace` are original default user
commands (535980 ReceiveUserCommand0; registration in 542180 reads
`<name>IsDefault` over `spt/command.xtx`). The retained command receipt is
acknowledged only when the mode returns. PrintMovie (4fdfb0/4fe010) plays the
movielist entry through shared Media and waits `lastWait` native frames.
SelectPlace (509020/508980/507d00/507530) is a modal CCommonButtonGroup over
its background SPT. Exist/enable words use native GetVarData (-1 reads 0x7fff),
and the selection writes `selectedPlaceVarName` = place + 1. Shared Media ids
for the three PrintMovie owners are now below the runtime's 1024 limit.
Unregistered commands (MUGEN2 `printdate`, game override) fail explicitly.

Compilation: release `crysirror` player build passed.

Offline behavior: with `RUST_MIN_STACK=8388608`, release
`crysirror-system-nnn --features runtime --lib` passed 125, failed 1, ignored 2.
The failure is `saved_screen_layer_cannot_be_silently_moved_under_the_restored_text`
(`validate_saved_wait_effects`), which this change does not touch. With the
default 2 MiB test stack, `complete_legacy_preference_variable_groups_*` and
`newgame_resets_original_variable_ranges_*` overflow. Removing the new
`try_commit` hook did not change that.

Game behavior (offscreen, `--headless --no-audio`, click every 60 or 120
frames): earlier stops at frame ~380k (`printmovie 1`, then
`selectplace`) are passed. A 600000-frame run showed the SelectPlace screen with
the background script and four place buttons. With the pointer on place 1, the
script continued into `sister1`. A 1000000-frame run ended without error in
`epilogue`. Two runs ended at different epilogue PCs with an in-flight sound
load. Movie frames, audio output, other places/routes, saves during these modes
and windowed play were not checked.
