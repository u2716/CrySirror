# Native Circus MES adapter

[Opcode and syscall implementation list](../../docs/native-api/circus.md).

Mode: `adopt`.

First-class bounded parsers identify the offset1 D.C.II P.C. ABI (0x5769),
offset2 D.C.III With You (0xa09f), DC4 (0xaab6) and DC4 Plus Harmony (0xabb6).
Operand ranges, version-prefix lengths and continuation IDs are profile-specific;
parsing an older profile does not select the DC4PH execution dispatcher.
Original operand widths, signed words, shifted
Shift-JIS strings, label metadata and 0x77 continuations remain native data.
Invalid headers, truncated operands and non-boundary PCs fail explicitly.
The original D.C. `#ADV_TXT` profile has its own bounded record, read-history
and label parser in `advtxt`; its LF/CR records are not later MES opcodes.

D.C. (DC.EXE) scenario contract (`dc1_story`, projected by `adapter/dc1_story`):
- 404600 dispatcher runs records until a presentation opcode returns control;
  408F70 then owns modes 0/1/4-11/14-18/21-25 (reveal, key waits, timed
  waits, choices, scrolls, grade wipe, shake). Native tables (palette 43C040,
  Messwin3/KEY_WAIT/TimeChip/map_name atlases, place names 43CE10/44097C,
  day rows 44DC70, bsteff 442670, eyecatches 441C68, map 468EE8/44DC28)
  are read from the installed executable.
- 4013F0 with the default BA9D88 path: 30 px cells, 28 px advance, 32 px
  lines (+8 after `@R`), shadow palette 6 at +2/+2; ruby `｛｝` uses the
  12 px GDI font at x - 6*k + 6, y - 10; `≪≫` dots at x - 1, y - 18;
  `@K`, `@W`, `@TWn`, `@N`, `@B` follow 4013F0/402030.
- Sprites are 800x600 images with an `m` mask: alpha = 255 - mask red
  (427F10). `@`/`*` suffixes are 42D750 palette tints; every DC1 `bst`,
  `bsteff` and most `back` CRX are 24-bit, so they only strip the suffix.
- Background changes use 406DA0 types 0/1 with BA9D4C: a grade rule wipe,
  weight clamp(p + blue - 255), step 32 per timer-10 tick (opcode 97 sets
  that interval); 426260 map wipes step 64. The shared crossfade receives
  the inverted grade with vague 255.
- Audio: BGM 200*param - 900 (cB), change = 42A400 fade then start; SE -1000;
  voice -300 from voice.dat (CircusVoice52), then the loose VOICE .pcm;
  when both are missing 429B60 plays nothing (some scripts name voices of
  another scene directory). XPCM mode 2 is decoded by crysirror-audio.
- 414760 day flow: day rows, 42B370/42B3C0 alarm map and the 44DC28 scene
  are implemented.
- Input follows 418F70: left/Enter = BA9F58, Space = BA9F48 (hide window
  in the scenario), right/Esc/Backspace = BA9F70 (system menu), wheel or
  PageUp/PageDown = BA9F94/98.
- Title 414070/413CB0 (hover, Extra/Config submenus), option screen
  407CC0/4083E0 (`dc1_config`, settings in 418160 units) from the title and
  from the 40B7E0 system menu, Messwin3 window buttons (close, log, next,
  auto-skip), 40BC70/40BCD0 backlog and the 414500 return-to-title dialog.
  Layer order is global, so each pass uses its own order band.
- Unported, failing visibly: 42C090/42C0D0 and 42B9E0/42BA30 day events,
  final-day routing, 4237C0 save/load, 420C00 CG mode, 41FE00 music mode,
  41F720 extra mode, 41E180 name entry, window-size change, the
  BA9D4C-off transition and opcodes 15/16/19-21/55/56/62/68/77/98/99/112/
  119-121. Settings are not yet written back by 422C60.

The 0xabb6 interpreter executes original native instructions through a thin
client-scoped HostServices adapter. `runtime` enables the optional factory
registration. The other profiles are parsed but explicitly rejected by execution.
VM state includes native byte banks, system selectors, label/read/savepoint
metadata, string jump labels, branch scans, pending resource/UI declarations
and audio queues. No GPU/device pointer is part of that state.

Reusable CRXG and cursor decoding live in crysirror-image, PCK/DAT archive
indexing and seekable ranges in crysirror-resource, and XPCM PCM/Ogg decoding,
preparation and mixing in crysirror-audio. Retained layers and widgets belong
to CrySirror. Resource workers return immutable results; the VM owner alone
publishes resources/engine commands and advances completion-dependent PCs.

Ordinary native falling effects declare five ordered emitters in one shared
VFX system. The engine owns the existing particle pools, falling/atlas node,
screen mesh publication and worker completion receipts. The adapter retains
only native density/depth/resource declarations and explicit frame policy.
Its process-wide integer random stream is seeded once from virtual engine
milliseconds. Native random instructions borrow that same stream through an
acknowledged exchange before changing VM state. Storm birth/reset differences,
other effect families and concurrent/replaced effect pools still fail visibly.

Complete-game compatibility remains unverified. Unknown native profiles and
unaudited save projections fail at their original operation. Parsing installed
scripts or registering a factory does not establish title/story completion.

`native_save` preserves the complete original 356324-byte slot prefix, opaque
bytes and native read-run metadata. Its bounded patches implement the audited
PC, byte-bank, image-name, civil timestamp and checkpoint sprite fixups. It
deliberately cannot construct an empty slot body from logical VM JSON.
`native_live` maintains the source-specific body and dialogue checkpoint;
`save_resources` and `restore_ops` coordinate resource-dependent restoration.
Original slots and their selected 128x72 PNG thumbnails use these native paths.

`NativeLiveState` additionally tracks the audited original initialization and
live mutations: nonzero native sprite constructor fields, both byte-bank reset
arrays, original read-history metadata/counters, random history, texture
extents, sprite transforms, placement rows, voice timing and music fields.
This VM metadata is distinct from shared engine rendering resources. Successful
instructions synchronize it; unaudited instructions leave explicit admission
gaps visible in runtime inspection. Original thumbnail drawing selects native
layers and UI explicitly.

Release/offscreen validation on 2026-11-05 exercised DC4PHDL for 12,200 host
frames through scene D and wrote its original 356324-byte automatic slot.
A separate 9,800-frame run wrote a quick slot, advanced dialogue, then restored
the saved first sentence without replaying the opening. These runs created no
window or audio device. They validate those paths, not every opcode selector,
save profile or the complete game. Public API changes are documented in docs/api;
native field mappings and remaining semantic gaps belong beside their source.

DC3WY (0xa09f) release/headless validation on 2026-11-13 (`--no-audio`,
Ctrl-skip input track, debug-only `CIRCUS_DC3_START=<scene>` entry that
redirects the first transfer to MAIN): 60 start points covering every `wy_*`
story script ran without an adapter error; 18 of them reached the ending
credits (`MED`) and returned to the title. This validates those script paths
under skipping, not unskipped presentation of every effect.

D.C.III (DC3.exe, 0x8a71) runs on the same interpreter through
`dc3_original`: an opcode table aligned on the 23 system scripts both
installations ship, plus an image-slot remap. Validation on 2026-11-13: logo,
window-size menu, title menu with its button images and title weather.

D.C.III 0x0E selectors that enter process state 14 (42B300: 0 crossfade,
1/2 black fades with the scene reset, 3 flash, 4/6 shake, 16 tile break, 17
lookup wipe, 21 rule wipe, and the immediate default) run in
`dc3_original_effects`: selector rewrites, step tables, C1CD3C progress,
4073F0's xorshift128 setup, 408C40 tables and 407F20 thresholds stay in the
VM. `adapter/dc3_original_screen` freezes the presented screen as a Canvas
copy at the start of the frame whose script changes it, then composes
frames with Layer overlays or shared image jobs (`linear_alpha_lookup`,
rule-table transitions, `tile_scatter` plus stored-byte composites). Known
differences: 411FB0/413850/407F20 blends are reproduced through engine
opacity to within one level, not bit-exactly; the live scene stands in for
the frozen next-screen snapshot; the final 34 ms hold after synchronous
loops is not inserted; 413850 reads past its 2688-byte table at the start of
the diagonal wipe, which is treated as zero; 42EFF0 maps to forced stops of
the voice and sound-effect groups. Selector 20 and 17/0 fail visibly
(unused by the shipped scripts). Interpreter-level selectors (5, 9, 13, 14,
31, 32, 51, 52) keep the DC3WY handlers and are not yet audited against
424FE0. 

D.C.III 0x27 (40AB30) is the older DC2-generation weather, not DC3WY's
particle pools; `dc3_original_weather` keeps its policy over the shared
keyed-atlas Layer presentation (`adapter/dc2_weather`): petals 1/3/4/9/11
(40A0C0/40A530/407300, original slot 17, xorshift128 407190, cosine 481EFC,
layer 1 below / layer 7 above the portraits), rain 2 (4084B0/408600/4087A0,
slot 18), fog 7 (4091F0/423290: EFF\FOGX* in slot 16 now translates to
slot 21 and mode 3 runs the shared two-band scroll on objects 52/53;
SYSTEM\FOG* stays slot 20/object 54) and the 6 background pan (0x69 mode 2
stores the target; 40A7A0 runs over 10*p calls). The 40AD20 timer is
stepped every 30 ms of engine time beneath script waits. Known
differences: native layer 2 (back fog band) is below the portraits, while
object 52 is above them; periods are converted from the 30 ms timer to
60 Hz sequence frames; 0x27 255 and kinds 5/8/10 fail visibly.

Validation 2026-11-13 (release, `--headless --no-audio`, click track,
`CIRCUS_DC3_START`): dc3_1_0223_f1_pro draws keyed SACRA petals over the
scene; dc3_1_1211_b1_cmn shows the EFF\FOGX scroll. A sweep of 81 story
starts (every sixth scene, 16000 frames) ran 79 without an adapter error;
the other two stop at original 0x6D (DC3_1_0223_C1_PRO:0x186, 424FE0 case
109) and at a 0x2C into original slot 7 (portrait overlay, dc3_1_1217_b1_hmn).
The shared framebuffer clear released every slot outside DC4PH's 37..44,
dropping DC3's system atlases 28..34 (MESWIN, ADVMENU chips) at the first
0x0E reset, so the message window vanished; it now keeps 28..34 for the
DC3 profile, as the movie path does. 42A160 case 6's dialogue controls
(templates 3..6 hit-tested by 4130B0, actions through 421AD0) are not
connected in either DC3 version, so the quick-menu buttons are not drawn.

Update 2026-11-13: both DC3 versions now draw CRM parts (BST portraits,
FACE) at their CRXG canvas origin (D.C.III 401AA0 adds the image record's
origin to the script position; `images::load_profile_image_origin`). Before,
every portrait was drawn at the left edge of its 1024-wide canvas. The DC3WY
`BST\MRK` width shift is not applied to CRM parts (their native canvas is
1024 wide). Process case 6 dialogue controls are connected (`dc3_controls`):
template 9 (collapsed tab) or 10 (expanded bar) is published as an inline
control menu while a rendered line waits; a release outside every control
advances the line. Actions: `n` (DC3WY value 99/227, D.C.III template
swap), `c` (hide window until a click), `r` skip toggle, `a` auto toggle
(41E740 timer 5*(23-99/29)*bytes/2 ms, voice wait with 99/28), D.C.III
`s`/`l`/`h`/`o`/`b`/`t` -> SAVE/LOAD/ADVMENU/OPTION/LOG/manual (NAMES.DAT
bootstrap table), DC3WY `o`/`ds`/`dl` -> config/save/load. Quick save/load,
DC3WY native backlog and voice replay, and D.C.III's auto-speed bar fail
visibly. Validation (release, headless, `--no-audio`, D.C.III
dc3_1_1214_e2_cmn): four portraits spread across the screen, the tab and
the expanded bar with hover tooltip draw, auto advanced 0x43c3 -> 0x50d0
without clicks, `clear` hid the window; DC3WY wy_1_0708_b1_rcc ran 400000
frames into WY_1_0720_E1_RCC without an adapter error. `cargo test --release
-p crysirror-circus`: 102 passed, 23 failed (DC4 machine fixtures and a
texel-quad extent test); not established whether these predate this change
in the current working tree.

Validation 2026-11-13 (release, `--headless --no-audio`, window-size click
track, `CIRCUS_DC3_START`): dc3_1_1216_d1_cmn ran its 0x0E 2/1/0/32 sequence
into DC3_1_1216_F1_FRE without an adapter error (trace: state-1 23 timer
calls, crossfades about 20); a capture during selector 1 shows the partly
faded classroom. Selector 17/1 (dc3_1_0223_f1_pro) and 21/3, 21/5, 21/4
(dc3_1_1211_a1_pro, dc3_1_0222_e2_pro, dc3_1_0226_b1_pro) completed and
stopped at the following 0x27. Selector 4/0 (dc3_1212_free_entrance, into
DC3_1_1213_A1_CMN) and 6/0 (dc3_2_1312_f1_hmn, to pc 0x464a) ran without an
error; their frames were not inspected. Selectors 3 and 16 were not reached.
Unported interpreter selectors and opcodes still fail visibly
(`dc3_original::UNTRANSLATED`).

Run the actual engine without a window or audio device:

```sh
cargo run --release -p crysirror-circus --features runtime --example headless_boot -- GAME_ROOT PRIVATE_WRITABLE_ROOT
```

`native_trace` is a semantic diagnostic only; it does not render or replace
the engine host. Original game files and fonts are never embedded.

Pinned format reference and licence provenance are in SOURCE.json and LICENSE.
No original engine renderer, mixer, executable or game data is included.
