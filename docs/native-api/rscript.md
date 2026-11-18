# codeX RScript native API

Adapter: [`crysirror-rscript`](../../crates/crysirror-rscript/README.md) (adopt).

## ABI profile

| Item | Value |
| --- | --- |
| Revision | FileVersion 1.11.0.3 |
| Build profile | `rscript-1.11.0.3-wide` (APINI 0x240 bytes; screen width +428). Other APINI layouts are rejected until verified against their executable |
| Verified build | Shapeshifter trial executable, SHA-256 `f9e3299467ff1bae3942e9ddf7c8092b70ec2f758581bfe4471deceeaaff191d`: dispatcher 0x4358A0 (253-case switch), WinMain/APINI reader 0x433730, blend dispatcher 0x4464E0 |
| Program | `.gsc`: nine-dword header, code, string index/data, word arrays, label names/offsets/data |
| Memory | 10 000 word variables (0..6999 per play-through), 102 word registers, configuration block 0x872 bytes, scene block 0x996C bytes (script stack +0x1E, return stack +0x34, depth +0x5C) |
| Calls | Depth 0..9; calls pass ten values in variables 10..19; return value in variable 0 |
| Expressions | Opcodes with a non-zero high nibble: destination register, one (0xF000 load) or two signed word operands; operand modes immediate, register, variable with indirection depth |
| Value operands | Low signed word, high word = indirection count through variables |
| Text | `@` variable references (indirection by `@` count) then `$0`..`$9` string registers; markup `^B ^I ^C ^D ^F ^S ^L ^N ^V ^A ^G`, `|base[ruby]` |

The operand layout of every opcode below was compared against the decompiled
dispatcher of the verified build; all 138 layouts match `layouts.rs`.

## Opcodes

Status: **M** mapped to a CrySirror handler, **N** native no-op (evidence
given), **R** refused: execution stops visibly with the opcode, PC and
operands. An opcode outside the table also stops execution.

| Opcode | Operands | Native behaviour | Status |
| --- | --- | --- | --- |
| 03/04/05 | target | jump if register 0 is zero / non-zero / always | M |
| 08 | – | end the thread (nested call returns; the main thread restarts the opening) | M |
| 09 | u16 | register ← MSVC `rand()` | M |
| 0A / 0B | – | click wait / click wait, then clear box 0 | M |
| 0C / 0F / 10 | script, label (, 10 values) / value | goto / call with parameters / return | M |
| 0D | tenths | interruptible sleep | M |
| 0E | header, question, 5 targets, 5 answers, 3 values | choice window, jump to the chosen target | M |
| 11 | – | save screen in save mode (sub_43E860 → sub_430FB0(1, 1)); the thread waits until it closes | M |
| 12 | variable, array | copy a word array into variables | M |
| 13 | script | system call: nested thread, scene restored afterwards | M |
| 14 / 15 / E6 | image, flags / white / image | background layer 0 (event image, fill, kind-2 image) | M |
| 16 / 21 / 22 | layer, x, y, kind, speed | move layer 0 / absolute / relative (14 motion kinds) | M |
| 17 / 18 | count, kind | screen shake / flash | M |
| 19 | DLL name, value | plug-in scene by DLL name: variable 0 = value, then the plug-in state (sub_43D500, sub_42D680) | M (built-in facade; an unshipped DLL continues the game, a shipped one without a facade fails) |
| 20 | layer, string, x, y, … | text layer (source 2) from a string (sub_43CC70) | M |
| 23 | layer, pattern | animation pattern of layers holding an image or source (sub_43CAC0 → sub_414C60) | M |
| 1A / 1B / 1C / 1D | – / – / kind, steps, ms / zoom, anchors | batch begin / flush / transition / zoom-shrink transition | M (1D drawn as a crossfade) |
| 1E | layer, image, x, y, effect, flags | load a layer image with one of 23 entrance effects | M |
| 24 / 25 / 26 / 27 / 28 / 29 / 31 / 79 | layer targets | hide with exit effect / visibility / anchors / blend preset / order / group / kind / image directory | M |
| 2A / 53 | box, visible / box | show/hide a text box / clear it | M |
| 2B / 2C / 2D / 2E / 2F | | effect screen / its order / tone overlay / its order / coordinate scale | M |
| 30 | image, effect | message face image, record +468 (sub_43B990 → sub_429000) | M |
| 3A | – | end the thread while the top-menu start flag (word_4ED28C) is set | M |
| 3B | layer, image | image at the layer's current origin (sub_43CB80 → sub_4109A0) | M |
| 32 / 33 / 37 / 35 / 34 | | snapshots, return to the previous choice, auto rebuild, end skip | M |
| 38 / 39 | 5 flags / – | panel, menu, tab, backlog and snapshot switches / clear backlog | M |
| 3C / 3D | track, fade, steps / fade, steps | BGM `Track%02d.wav` (Vorbis ACM WAVE) | M |
| 3E / 3F / 40 / 44 | channel… | sound-effect channels 0..2: load / play (999 loops) / stop / wait | M |
| 41 | movie | play `mov\%04d.mpg` | M |
| 42 / 43 / 45 | voice (raw dword)… | voice (bank = voice / 10000) / stop / wait | M |
| 46 / 47 / 4D / 48 / 49 / 4A / 4B | | layer buttons (modes 0/1/2), stop, button wait, reset flag, hover overlays | M (mode-1 native screen buttons report a diagnostic) |
| 50 | box | clear a box's source and text (sub_43F560 → sub_428B50) | M |
| 54 | box, string | set a box's text at once, or clear its source (sub_43F6C0) | M |
| 51 / 52 | box, voice, repeat, pan, name, text, wait / box, …, text, wait | message (new page / append) with reveal, click wait and backlog | M |
| 5A..6C | box selectors | text box records: position, text area, frame, panel, colour, size, face, align, indent, wait icon, glow, face order, name area, spacing, ruby | M |
| 6E | layer, x var, y var | record position divided by the coordinate scale (sub_43DA80; a zero scale stops like the native division fault) | M |
| 6F / 70 / 71 | a, b, c / a / a, n | var 0 ← b·a/c (c = 0 stops) / ⌊√a·1000⌋ / aⁿ in 16 bits | M |
| 72 | image, set | seen-image flags: set, or query into var 0 | M |
| 73 / 74 / 75 | index, value / index, value / index, var | scene words +0x9956 / +0x9942 set, +0x9942 read | M |
| 78 | layer targets, size | text-layer font size, record +92 (sub_43CDB0 → sub_414C10) | M |
| 82 / 83 / 86 / 87 / 88 | gauge, … | gauge records (scene +0x5030, 52 bytes): Num/Bar/Fix images / min, max, words / position / digit and bar offsets / value clamped to [min, max] (sub_4652D0), bar animated one unit per tick | M |
| 84 | gauge, visible | gauge visibility flag | M |
| 8C / 8D / 8E | index, value / index, value / index, var | configuration words +0x50 / +0x3C set, +0x3C read | M |
| 96 / 97 | register, string | set / append a string register | M |
| 98 | register, var | a variable as full-width Shift-JIS digits (sub_44E0A0) | M |
| 99 | var, register | atoi of a string register (spaces/tabs, sign; sub_44E540) | M |
| 9A / 9B | register, register | copy / append between string registers | M |
| 9C | dialog | text entry `texedt%02d.lwg` (CEditMenu, window message 0x432); `text0`..`text9` fields bound to string registers, `ok`/`cansel`; the thread waits | M |
| 9D | register, limit | the register's flag byte (byte limit of its edit field; sub_43F1A0) | M |
| 9E / 9F | register, index | string register → configuration string +0x64 / back | M |
| C8 | target, 10 values | local call | M |
| C9 | layer targets, 4 words | record flip/scale/mask words +26..+32 used by kind-1 layers (sub_43C630 → sub_413DF0) | M |
| D2 / D3 / D4 / D5 | – / script, target, value, condition / script, string / header, …, arrangement | paged selection (CDynSelect): begin / append an entry (bit 15 of the condition variable inverts it) / the "next page" label / open: up to five entries, else four per page and "next" (sub_409750), result in var 0 | M |
| DC / DD / DE / DF | timer var, hide, cancel / map, sub / – / button, enable | map screen `map%04d.lwg` with buttons `01`..`20` (CMap): wait / open (sub map at priority 0x90) / close / hide or show a button | M |
| E1 | x var, y var, timer var, hide, cancel | wait for a click anywhere, then store the pointer (sub_43E0C0) | M |
| E7 | layer | wait while the layer sprite animates (sub_43CEB0 → sub_415110) | M |
| CA | from, to, value | fill variables | M |
| FF | 5 values | read and ignored (`nullsub_1` in the verified dispatcher) | N |

Coverage: 137 of 138 opcodes mapped; FF is a verified no-op.

## Save screen (0x11)

`CSave` (0x41D230) in save mode: `savescrn.lwg` `bg_save` (`bg_load` in load
mode), ten slots placed at the `0`..`9` layers, `exit`, `next`/`prev`
(next shown while page ≤ pages − 2 and stopping at page 9, prev while page
≠ 0), optional page buttons `01`..`10` (the current one pressed and inert)
and the `nonbl.lwg` page digit at `number`. Each slot (CFileBtn,
sub_40C040) is an invisible hit area the size of `saveconf.lwg`, inverted
while hovered (blend 0x67/0x17), with `dt1_NNNN`/`dt2_NNNN` icons from
header variables 1/2 (the second at `icon`), the date at `date` in APINI
palette colour [variable 3] and the date size, the thumbnail at `thmb`, the
page text at `text` (APINI colour +504, size +514) and `dat_new` at `new`
on the last saved slot (sub_40CB60).

Saving a slot (sub_4304C0) asks `確認` / `セーブデータを上書きします。…` with
OK and Cancel (MessageBoxA type 0x21) when the slot exists, stores the slot
number in configuration +0x3A, then writes `<save dir>\<prefix>NN.dat`
(NN = page·10 + slot + 1; sub_433240) stamped with the local time. The
page is configuration +0x38. The header keeps variables 1..3 of the
message snapshot, the snapshot's backdrop image number and the last
message, choice or selection text (byte_50A76C). Right click, Escape and
`exit` close the screen and the script continues. The message window is
hidden and the scene darkened by 80% unless the APINI keeps the backdrop.

Before the screen is drawn the presented frame is captured and shrunk to
whole percentages of the screen given by the `thmb` layer size
(sub_462870: 16.16 steps of `0x640000 / percent`, integer part by
division by 0xFFFF); a save from the screen writes it as
`<prefix>NN.wcg` (sub_471F20: `WG`, version 0x271, depth 32, high words
red/inverted alpha then low words blue/green, each a dictionary-coded
plane) unless the slot thumbnail comes from `thmb\thmbNNNN`.

## Loading (load screen, quick load, previous choice)

In load mode a slot loads only when it exists, after `確認` /
`セーブデータをロードします。…` (sub_42FBF0): the slot file replaces the
live scene and variables 0..6999 and the previous-choice snapshot
(sub_432FB0), the decision sound plays and the game state is entered again
(message 0x410). Entering a state stops sound and voice, fades the music
out over 10 steps (sub_4527E0) and fades the screen in 16 steps of 20 ms
(sub_44FCF0: the last frame darkens in 17 steps of 20 ms, then the new
state shows); the game state's start (0x40E) gives every box the
configured message speed (sub_42A230), reloads the script on top of the
scene's script stack, resumes at its return address and rebuilds layers,
boxes, effect screen, overlay, gauges and the map (sub_43CF20,
sub_43CFE0). Leaving the game state writes the system save (sub_4306B0).

Quick save (`qsave`, sub_42EE10) asks `クイックセーブしますか？` and writes
slot 0 without a thumbnail and without changing the last slot; quick load
(`qload`, sub_42EDB0) asks `クイックロードしますか？` when slot 0 exists. The
previous-choice button (`rev`, sub_42EE50) copies the previous-choice
snapshot into the live scene (sub_4330C0) and enters the game state again.

The system save `<prefix>.dat` (sub_433180, read at start-up by
sub_432D60) holds the configuration block, variables 7000..9999 and two
bit-set maps (u32 count, then u16 key, u32 length, bytes; bit n is
`1 << (n & 7)` of byte `n >> 3`): read text (key = script, bit = string
index) and seen images (key 0). An absent file keeps the defaults.

The map screen's number, sub-map and hidden flags live in the scene at
+0x9928 (CMap record), so they are saved and rebuilt with the scene. The
message source record of each box is script, name, text (sub_42A4D0).

## Companion panel, keys and backlog

Panel buttons (CTexBoxEx 0x427400) and their callbacks: skip (+228; needs
the tab switch), next (+232; skip to the next choice), rev (+236), bak and
fow (backlog), hide (+240), voc (+256; the shown page's voice), auto
(+260), menu (+244; configuration, needs the menu switch), save (+248) and
load (+252; need the panel save/load switch), extd (+268; a plug-in DLL,
answered by a plug-in facade, see below), qsave (+272), qload (+276). The `slide_lev`
track with the `slide` knob (vertical when taller than wide) sets every
box's back transparency and configuration word 0x1A (sub_4289D0,
sub_42EBE0).

Keys (sub_42FCA0): Tab skip, Shift hide (or show), Ctrl held skip,
Escape/right click the menu (a hidden window comes back first), cursor
up/wheel up backlog back, cursor down/wheel down backlog forward (wheel
down clicks when not browsing). With the auto-hide switch on, 100 ticks
without input during a button wait hide the cursor and the message window
until the next input (sub_4308C0, sub_42E8D0).

Backlog browsing shows a page start of the 100-entry ring in box 0 with
that entry's box state, name and voice, in the APINI backlog colour when
set (+518/+520): back goes to the page start before the shown one
(sub_42A010, starting from the newest, sub_429F80); forward goes to the
next one, and reaching the newest page closes the backlog and restores
the live page (sub_429590, sub_429D80). A click closes it.

Button mode 1 (sub_431440) opens native screens by id: 0 quit, 1 title, 2
load (title state), 3 configuration (title state), 4 configuration, 5 load,
6 save, 7 music menu, 8 CG view, 9 scene view, 10 extra menu.

Timed button, map and pointer waits with a shown timer drive gauge 0:
minimum 0, maximum the timer, words 4 and 0, the remaining time every
10 ms (sub_43E0C0, sub_41CDE0, sub_41CE60).

## Configuration screen

`CConfig` (0x417D30, opened by sub_41B170) on `confscrn.lwg`: buttons
`save`, `load` (shown while the panel save/load switch is on), `exit`
(SC_CLOSE), `title` (`タイトル画面に戻ります。…`, then the title state) and
`close`; radio groups whose buttons exist (a group takes the index of the
existing buttons): `scm_wnd/scm_ful` screen mode (fullscreen request),
`msp_now/msp_fst/msp_nom/msp_slw` message speed (written to every box,
+70), and off/on pairs `sef` sound (loops restart or stop), `bgm` music,
`gef` effects, `msk` skip unread, `voc` voice, `cmp` companion panel
(shows or hides it when switched), `usr` variable 7000, `vocst` voice
continues, `awh` auto hide, `bgr` audio while inactive; `exNN_k` scene word
NN and `gxNN_k` configuration word 0x3C + 2NN, each shown while the word
ten places later is set. Sliders (`X_lev` track, `X_vol` knob) set music,
voice and sound volume (0..255, applied at once) and the auto wait
(0..5000). `pvNN_k` voice-bank switches hide the button of the current
value and play `9999.wav`/`9998.wav` of the bank; `bgm_test`, `sef_test`,
`voc_test`, `pvNN_test` play the APINI test track (+528), sound (+530),
voice (+532) and bank voices (+536). `font` opens `fontwnd.lwg` (ten rows
22 pixels apart, prev/next pages, exit): the chosen face becomes the
configuration font (+0x83E) and the message face. `tab_0..2` switch to
the `status` and `desig` pages, whose text formats and names are read
from the executable (.rdata of the verified build).

## Title and states

After movies 2 and 1 (played once per run) the APINI start script runs
when the top menu is skipped; otherwise a black 640×480 screen shows
`.\grps\top_me01..07` (two frames stacked vertically) at (54, 223 + 23i):
new game with script 0, 1, load, configuration, extra menu, new game with
script 7000 (shown while variable 7000 is set), quit. New game copies the
scene into both snapshots (sub_432BC0). Load from the title shows
`.\grps\bg01` under the load screen; configuration from the title shows
`confback` and its title button writes the system save and returns.

## Extras

The extra menu (`extrmenu.lwg`: `cg`, `sc`, `ms`, `exit`) leads to the CG
view, the scene view and the music menu; button mode 1 reaches them too.
The CG view (`cgview.lwg`) pages twelve thumbnails over the executable's
table of six images per entry (0x4CB9F8); a thumbnail shows once variable
9000 + its first image is set, opens the entry's `.\grpe\NNNN` images one
click at a time with a 16-step dither, and closes after the last or on a
right click. The scene view (`scview.lwg`, table 0x4EB298: flag variable,
thumbnail, start value, 52-byte title) shows the hovered title at `text`
and replays an entry through script 9999 with variable 0 set; the replay's
end returns to the view, and leaving the view restores the scene kept when
it opened. The music menu (`muscmenu.lwg`) plays `Track(N)` for buttons
`1..20` (`N_p` while playing) with play, stop, pause, rev, fow and a
`level`/`bar` volume slider.

## Animated layers and load conversions

Kind 2 layers (CMainCelAnim, sub_402280) take the layers of `NNNN.lwg` as
frames and step through `NNNN.fsc` once per tick (sub_402510), waiting or
not: per line, `:name` label, `.` wait a tick, `-` stop, `>name` jump,
`?V N name` jump when variable V is N, `?rnd N name` jump when
`rand() % N` is 0, otherwise show that frame. Without a flow the frames
loop.

Kind 0 image loads (CMainCelLim sub_415810) apply, on the shared image
worker before the picture shows: the mirror bits (+26), rotation (+30
degrees, 256-entry tables) with zoom (+28 percent), then the load flags'
tone: 1 black, 2 negative, 3 average grey, 4..6 tints (0x8B4513,
0x483D8B, 0xFF8000), 7/11 night, 8..10/12/13 per-channel tints, 15 the
hue lift around blue. Record +32 names a picture drawn over the image
(sub_416460). 0xC9 sets +26..+32.

## Zoom and shrink transitions

Kinds 1xx (sub_453A60) and 2xx (sub_4537E0) move between numpad anchors
(tens digit, then units digit; 7 8 9 top, 4 6 middle, 1 2 3 bottom,
others the centre). Zoom shows 100/(i+100) of the old frame around the
first anchor stretched to the screen for i = 0..600 by 20, then the new
frame from 600 to 0 around the second; shrink draws the old frame at
100/(i+100) of the screen around the first anchor on black for i =
0..2000 by 50, then the new frame back up. Each step is 5 ms. The old
frame is the presented frame; the new one is captured as the layer prefix
below the transition's own layers.

## Plug-in DLLs

Game DLLs are never loaded. The DLLs the verified build calls — the
opcode 0x19 plug-in scene (its `GetInfo` export receives a callback
table) and the APINI extension DLL at +339 (`Init`/`Term`, the panel's
`extd` button) — are answered by built-in facades registered under the
canonical file name (`crysirror_rscript::plugins::FACADES`), like the
KiriKiri plug-in facades. When the game does not ship the DLL the native
fallback applies: the 0x19 scene returns to the game and `extd` does
nothing. A shipped DLL without a facade stops with its name. The
Shapeshifter trial names `FlowDll.dll` but ships no DLL, so no facade is
registered yet.

## Framework snapshot

A framework snapshot is the native safepoint: the message and
previous-choice snapshots in the slot layout (zero date) plus the system
data; restore and original-save import re-enter the game state like a
slot load.

## Native blend modes (dispatcher 0x4464E0)

The verified build has cases 0, 2..8, 0x0B..0x0F, 0x12..0x16 and
0x67..0x6D; any other mode draws nothing. Button hover style 3 selects mode
0x17, so a hovered style-3 button face is not drawn and only its hover
overlays show. Mapping to shared blends: 0 normal; 2 level fade (opacity);
3 darken / 4 brighten by level; 8 half blend; 0x0D invert; 0x0E additive;
0x67 inverse destination; 0x68 hard light (the emboss rows); 0x6B additive
glow in the sprite colour; 0x6C/0x6D darken/brighten the destination.
Dissolve (0x12) and blinds (0x14/0x15) patterns are drawn as level fades.

## Inferred or approximated

- Text cells are placed by the native fixed-cell layout; glyph baselines use
  an ascent of 0.86 of the cell height (GDI tmAscent is not available).
- Transition kind 2 (4×4 dither dissolve) uses the shared ordered dither;
  `.msk` wipes invert the mask once (native: dark pixels change first) and
  use a 31-level soft edge.
- The WCG thumbnail coder writes any valid table order (words sorted by
  frequency), not the native compressor's byte stream; the unused header
  word is written as 0.
- The backlog shows the entry's face and voice availability; the face
  image itself is not swapped.
- The font window lists the faces the host authorizes for the client,
  each row with the name and a sample in that face.
- Background audio off mutes the channels while the window is inactive.
- The zoom's half-level accumulation (blend kind 8) keeps the last nine
  steps; older steps weigh below 1/256.
- The picture over a kind 0 image (+32) is drawn as its own sprite with
  the layer's blend instead of being merged into the image first.
- Mode-2 level fades use opacity `255 − level`; the native `max(T, level)`
  differs on partially transparent pixels.
