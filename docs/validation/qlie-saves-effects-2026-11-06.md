# QLIE saves, input gestures and screen effects (2026-11-06)

Owner-authorized local installation `~/Downloads/game-QLIE-5`; no game data
is stored in the repository. Runs used the headless desktop player with
recorded input tracks kept outside the repository.

## Implementation

- Native save envelope (IMOSAVE110) with CrySirror VM-state body; restore
  of objects, layers, cameras, image replacements and looping sounds.
- `SystemData.dat` and user `Config.txt` in the save mount.
- `snd\fo` PCM filter chains (shared `pcm_filters`), swipe and gesture
  queries (shared `pointer_swipe`), list `lff`/`stf` streams (shared
  `lane_feedback`), log `ct`.
- `eff\fdi` modes 2 (colour blend) and 4 (mosaic); rule-image transitions
  prepared by the shared `tiled_transform` and `channel_range_stretch`
  effects, including the `eff\fdp` pose.

- cfg engine-setting record (6EA71C over the 74C754 record): every
  setting/query pair the dispatcher defines except `fnl`, with 505FE8
  defaults and clamps, Config.txt persistence (50786C, including the
  ILRate -> IL migration) and per-voice `cvi`.
- Native key-code modes (5C8074) with the 5C7798 modifier match (Alt added
  to the shared button set), KeyCallbackTime holds through the shared
  `key_hold` clock, gesture keys from `pointer_swipe`, and inp `k2` (all
  active codes), `kl2`, `i`, `sk`, `ms` (with MouseHideTime), `bsv`,
  `?bsv`, `?swe`, `mp`/`smp`; SwipeEnable/PermitFrame/Intense and
  MouseGestureEnable now come from the config record (70A3CC).
- Method inventory: packed method codes of every native dispatcher
  enumerated from the executable's 6D1B24 type table and compared with the
  adapter; at least 162 remain unhandled (listed in the crate README).

## Compilation

- `cargo check --release -p crysirror-qlie --features runtime`: passes.
- `cargo build --release -p crysirror-desktop`: passes.
- `cargo test --release -p crysirror-image --lib tiled_transform`: 3 passed.
- `cargo test --release -p crysirror-core --lib pointer_swipe`: 3 passed.
- Second pass: `cargo check --release -p crysirror-qlie --features
  runtime` and `cargo build --release -p crysirror-desktop` pass;
  `cargo test --release -p crysirror-core --lib key_hold`: 1 passed. The
  full core lib run has one failure,
  `stepped_pointer::signed_fractional_steps_truncate_coordinates_and_zero_axes_finish`,
  in a file this work did not modify. The API reference was not
  regenerated: the `api_reference` example does not build because of
  `crysirror-system4/src/native_effect/feedback.rs:114` (unrelated
  in-progress code).
- `cargo test --release -p crysirror-engine --test api_docs`: the
  fingerprint/changelog test passes; two failures concern
  `CivilTimeZoneQuery`/`HttpRequestJob` source declarations and the canvas
  `draw_subtree` command map, outside these changes.

## Behavior

- Title, name entry, dialogue, save (thumbnail, date, text), back, load and
  continued play across scenes; backlog open/scroll and log jump; second
  session reload of SystemData and Config.
- 150000-frame scripted run (including an `ogv` image replacement and
  rule-image fades through the new preparation path): no VM error.
- 6000-frame run of the save/load track after the gesture change: no VM
  error. Correction: the track's save/load clicks start at frame 9100, so
  that run did not reach them.
- Not exercised by this game's scripts: `fdi` modes 2 and 4, non-identity
  `fdp`, `lff`/`stf` (debug builds only), `log\ct`, `inp\?mgp`. These are
  implemented from the native code paths cited in the source and are not
  behavior-validated.
- Second pass (after the key-mode and cfg changes): the save/load track to
  16000 frames wrote save slot 0 with no VM error; the 150000-frame
  Ctrl-hold track (`$30011` SkipClick) ran with no VM error and reached
  late scenario scenes. Synthetic keys, input blocking, cursor auto-hide,
  gesture keys and the new cfg methods are not exercised by these tracks.
- Not verified: pixel parity of transitions and filters against the
  original executable.

## Third pass (same day)

Implementation: sys system requests (`c h l m s qs ql rr ee` through
708048 label routing without input locks, `e`, `fm`), events (`eve`/`?eve`
for all five names; only OnFrame fires), `run`, `out`, `?tz`, `ss`, `v`,
`fc`, `tc`/`?tc` (window title), `ckc`, `?mod`, `?ods`, `?rev`, `?blt`,
`hdr`/`?hdr` (values kept, mix not limited), `?mpd`, `srr`, `skp`, `ref`,
`cr`; misc `efn exp cfe crc tsha id lng ?lng now ?now gfe fid fsha exec
code ptof fkey` (shared `sha256` and `crc16_ccitt_serial` in
crysirror-resource; `locale` added to `HostInformation`); cfg setters
`blt gim fdm fps sb cs sfs`; inp `mp`/`smp` through the UI `warp_pointer`
command.

Compilation: `cargo check --release -p crysirror-qlie --features runtime`
passes; `cargo test --release -p crysirror-qlie --features runtime --lib`
1 passed (code cipher round trip and path helpers);
`cargo test --release -p crysirror-resource --lib serial_crc` 1 passed.
`cargo build --release -p crysirror-desktop` fails in
`crysirror-system4/src/native_effect/feedback.rs:114` (unrelated
in-progress code), so this pass has no behavior run.

Inventory: per-kind comparison against the native dispatchers leaves about
590 kind/method pairs unhandled (crate README).

## Fourth pass (same day)

Implementation: kinds `sdat`, `clip` (host clipboard; `DesktopRequest::
ClipboardRead` added to the desktop provider), `reg`, `ext`, `ppngpool`,
`twitter` settings, `dsnd` and `csr`/`scsr` routing; dlgs `d1 d2 d3 sd1
sd2 sd3`; skey `d dc mgi mgl mgp` (gesture guide overlay); timer
`out ?out`; flog `d`; llog `?k`; cam `p p2 p3 ?p2 ?p3 ?mm`, camex `?bm
?col ?mm`; eff `fdo` (fade-out direction), `mb vtn ?v ?dc ?sc ?vtn ?mb
?bg ?fdm`; snd `t ?t ?vtn ?l ?pan ?nv ?mnv pm tt`; theora `ss pp ost sk s
tt` and their queries; image `p2 p3 ?p2 ?p3 a ?a alr ?alr ?alg ?anc ?col
?bm ?mm ?in ?out ?ps ?sp ?tf ?tab ?vtn ?ost ?v ?l ?fo`. The
`crysirror-system4` compile error at `native_effect/feedback.rs:114` was
fixed (the missing `size` binding of the new image).

Compilation: `cargo build --release -p crysirror-desktop` passes;
`cargo test --release -p crysirror-engine --test api_docs` 6 passed, 2
failed (`draw_subtree` command map and missing source declarations, both
from parallel work); API fingerprint `488cc60b60c63756` logged.

Behavior: save/load track (clean save directory) to 16000 frames wrote
save slot 0 with no VM error; 150000-frame Ctrl-hold track: no VM error.
The methods added in this pass are not exercised by these tracks.

## Fifth pass (same day)

Implementation: message queries `?as ?ap ?kwb ?kwe ?kwl ?kwd ?nsw ?ns ?tlt
?ts` and settings `kwd tlt`; button `bc ?bc`; bar `anc ?anc ?dc ?sc ?mm ?l
?l2 skp`; snd `?fo`; dlgs `c` (colour dialog on engine UI, TColor results),
`o`/`s` (desktop provider file chooser, `./`-rooted capability paths).

Compilation: `cargo build --release -p crysirror-desktop` passes;
`cargo test --release -p crysirror-qlie --features runtime --lib` 1 passed.

Behavior: save/load track to 16000 frames (clean save directory) wrote save
slot 0, no VM error; 150000-frame Ctrl-hold track, no VM error. The methods
added in this pass are not exercised by these tracks.

## Sixth pass (2026-11-07)

Implementation: image TABMP slot methods `lsk lst lv lidx ?lsk ?lst ?lv
?tt gfs ?gfs ?gfk tmp ?tmp ss mem flt ?flt ?li ?ls` (per-track time,
delay and enable slots added to the shared-clock state; the misnamed `gsf`
handler became native `gfs`; `gfk` now stores >= 256 as 255 and ignores
slots outside 1..31; `?fo` reads the indexed slot); message picture glyphs
`glpi glpo glps glpf glpa gt` on msg/btn/sbtn (new
`PositionedGlyph.image` drawn by the wgpu glyph expansion and
`CellLayoutOptions.cell_widths` in the shared cell layout); image/bar
`?hit`, inp `?hit` and `bs` (6B0BA4/6B1A0C pointer navigation over `sp`
points in `tab` order); snd `sv` (`crysirror_audio::onset::first_audible`);
dsnd `\?l \?p`; cfg `fnl` (system/font/fontdata.txt). `rep\?r` was already
handled (inventory artifact).

Compilation: `cargo build --release` passes. Tests:
`crysirror-qlie --features runtime --lib` 4 passed; `crysirror-text --lib`
97 passed; `crysirror-wgpu --lib` 32 passed; `crysirror-audio --lib` 61
passed; `crysirror-scene --lib` 96 passed, 1 failed
(`input_routes::menu_collision_leaves_ui_journal_and_clock_uncommitted`,
a menu input-route test in a file this pass did not change);
`crysirror-engine --test api_docs` 6 passed, 2 failed (`draw_subtree`
command map; `CivilTimeZoneQuery` and `HttpRequestJob` without source
declarations, both from other in-tree work). API fingerprint
`11159237bd0ab027` logged.

Behavior: save/load track to 16000 frames (clean save directory) wrote save
slot 0 with no VM error; 150000-frame Ctrl-hold track: no VM error. The
methods added in this pass are not exercised by these tracks; picture
glyph drawing, `bs` navigation and `sv` have no behaviour evidence.

## Seventh pass (2026-11-07)

Implementation: twitter `auth`/`post` log and do nothing; misc `bsno`
(keys located in the game executable next to its "564" literal, exe name
and SHA-256; `fsha,self` uses the same executable, now passed by detection
as `executable`); sys `r` (engine reset) and `msd`/`?msd`; statements
`time sndt wav msg btn` (native frame wait modes 5/6/10/7/9 with their
click rules), `bspl btra hanc hlen dmon dout drun fobj` (`dbrk` is a no-op
as natively); `crysirror_text::legacy_width` for `hanc`/`hlen`.

Compilation: `cargo build --release` passes. Tests: `crysirror-qlie
--features runtime --lib` 5 passed (includes the `bsno` literal search on a
synthetic image); `crysirror-text --lib` 99 passed; `crysirror-desktop
--lib detect` 11 passed. The literal search was also checked against the
authorized test executable with an equivalent script: one matching run,
literal lengths 65/96/88/127 as in the disassembly. API fingerprint
unchanged (`11159237bd0ab027`).

Behavior: save/load track to 16000 frames (clean save directory) wrote save
slot 0 with no VM error; 150000-frame Ctrl-hold track: no VM error. The
statements, waits, `msd`, `r` and `bsno` added here are not exercised by
these tracks.

## Eighth pass (2026-11-07)

Implementation: `inp\?hit` stops below a hit object whose `mi` byte is set
(vmt+40 of TImoObjectDataImage/Message/Button reads +BE4); `bs` points and
non-world collision rectangles go through the object's layer affine chain
(`crysirror_scene::layer::local_to_parent`); picture-glyph sheets keep
their resolved path and are republished during the restore rebuild, with
the stashed glyph images re-pointed; saves record each sounding voice's
position, remaining loops and pause state and the rebuild restarts it there
(one-shot voices included); a message saved mid-reveal resumes from the
glyphs shown at the save. The README entry for dual-capture transitions was
stale (they and the mosaic transition are implemented) and was removed.

Compilation: `cargo build --release` passes; `crysirror-qlie --features
runtime --lib` 5 passed. API fingerprint unchanged.

Behavior: save/load track to 16000 frames (clean save directory) wrote and
loaded save slot 0 with no VM error; 150000-frame Ctrl-hold track: no VM
error. Whether the save/load track saved during a sound or a reveal was not
checked.
