# Malie

Mode: **adopt**. Native EXEC values, calls and screen/message policy belong
here. CrySirror owns archive reads, images, text, retained UI, audio, video
and output. Profile: Silverio Ragnarok 1.0 (trial-2 content), original
`malie.exe` SHA-256
`f688839990c4b838c0805578d52c67213a13afa94f68491cfe03ee3db8a25892`.
Implementation and exercised behavior are tracked separately; registration
does not establish a running game.

## Coverage against the EXE

| Surface | Native evidence | Status |
| --- | --- | --- |
| EXEC opcodes | interpreter 67A0F0 | 52/52. 44 dumps state to the debug log and continues (679C00 + `fgetc(stdin)`), 11/16 are native no-ops |
| Registered natives | registry 62AAA0 (31 entries) | 31/31 in `adapter/natives.rs`; script imports absent from the registry are `LINK ERROR`s natively and return 0 (67AEA0) |
| MalieSystem frame | callback 62CF80 (`System_GetScreen(1000)`) | every selector in the switch; unknown numbers return 0 as natively |
| Tags | dispatcher 62DA70 (57 names) | all 57 in `adapter/tags.rs`; other names are ignored as natively |
| Squirrel DOM API | binding names in the EXE | `getElementById/createElement/setCurrentTime/play/pause`, element `get/setAttribute/addEventListener/insertBefore/focus`, `Tweener.addTween/removeTween/getTime`, `MalieSystem.getFlag/setFlag/get/setConfigValue/tag/cmd/playLabel/gotoLabel/openDialog/endDialog/endModal/playSystemSe/isExistFile/isSkip/isDebug` |
| Dialogs | 613260 names | config shell (save, load, game, text, sound, voice, key, shortcut, backlog, skill/wiki pages), extra shell (cg, scene, scenerestore, music, movie, status), award, as/menu/sub/WIKIDIALOG modals, end/title/qload/qsave/continue message boxes |
| Script compiler | 6758A0/675AE0 | full port in `compiler/` (MalieSystem 120) |

Screen and frame semantics (52D470, 4FA860, 4FAA30, 6124D0, 613260):

- Root frames are 1000, 12346, 300, 451, 5, 302, 310 and 452;
  `System_GetScreen` returns null (0) for any other id and FrameLayer calls on
  null return 0.
- `System_ChangeScreen` switches only the registered screens: 300 shows the
  ADV screen, 1000 hides it, 12346 (the TakeScreen capture) holds the ADV
  picture. Other ids (442, 445 map, 450 …) miss the registry and do nothing.
  `<screen src='map'>`/unknown sources then suspend the script (6137D0)
  until a 6138F0 resume (`Wait::Halted`).
- `MalieSystem_OpenDialog(441)` opens the title; for another root frame it is
  a ChangeScreen inside a transition (612580). For an id with no root frame
  612580 dereferences null in the original; that remains a visible error.
- MalieSystem 154/158–162/401 message ADV item 201 (CUTIN2), whose class has
  no message callback: they return 0 (154/158 register the character first).
- ADV screen messages follow 52D7F0 in full: dialogs, skip/auto, window
  hide/show (incl. PSP_HIDE 1004), click release (1001/1009), quick
  save/load, voice repeat, the end prompt, Layer_LoadBG, suspend/resume,
  wheel, the menu and WIKIDIALOG modals, and the choice history (1100 back
  to the previous choice from a ten-entry snapshot stack, 1101 jump to the
  next choice, 1102 jump to the next `chapter`). Other numbers return 0.
- `layercmd` reaches the layer's content object; none of the content classes
  handles messages, so it has no effect.
- Dialog names follow 613260; names outside it return -1. `skill`/`wiki` are
  config pages 10/9 without includes (the shell opens with no page);
  `scenerestore` is the scene page; `as`, `menu`, `sub` and WIKIDIALOG open
  modal frames whose layouts (as/main.svg, menu/menu.svg, wiki/wiki.svg) this
  profile does not ship, so they are empty and close on cancel.

## Script compiler (MalieSystem 120)

`compiler/` ports the EXE's compiler (6758A0 → 675AE0) used when a script
switches modules (613800): source lookup `.ms` (UTF-8), `.mls`
("MalieScenario" + zlib, CP932), `.txt` and the plain name (CP932); the
preprocessor (`#include`, `#define` with arguments and `#`, `#undef`,
`#ifdef`/`#ifndef`/`#else`/`#endif`, `#line`); the C-subset parser with
types, structs, arrays, pointers, `sizeof`, scopes and calling conventions;
`$"…"` messages, `$$`, `&` directives and labels; constant folding and code
generation. Original quirks are kept (no folding inside call arguments,
expression-relative `?:` targets, the no-op jump after an `if` test,
`switch` evaluating only its expression, definitions using the prototype's
parameter scope). Where the original loops forever or dereferences null the
compile fails instead. The new module replaces the running one and its
`main` starts; saves record the module and recompile it on load.

Validated by compiling sources reconstructed from Ragnarok's exec.dat (its
symbol table, hand-written bodies for 12 functions, and a pattern
decompilation of `maliescenario`): code, symbols, link table, global size,
the 2.6 MB message section and all 13,878 message ranges are identical.

## Presentation contracts

- `back`/`chara` roots are camera-relative planes (default camera z=-300,
  90° vertical FOV); other roots use canvas pixels. Timed writes start at the
  previous target (63F4C0) and ease with the `accel` curve.
- Camera 1 (`layercamera id='1'`) renders the perspective world a second
  time; each camera is clipped to its side of the rotated splitter line.
- `char`, `charclear`, `face`, `cg`, `cg_h`, `cutin`, `movie` and `clear`
  are expanded into further tags exactly as their native handlers format
  them; characters are `chara`-root layers anchored at their bottom centre,
  with the native registry of face/dress/position/place slot (7B5410).
- Layer filters `col|sil <color> <percent>` and the global CG/character
  filters map onto shared group tint/add terms.
- `ef effect1..5` (EFFECT_ef) are Tweener sequences on a screen backdrop;
  their alpha/scale/x/y/blend constants come from 533CC0–536580, the step
  order is an approximation of those sequences.
- Movies (`m:movie`, inline `<param src-type="movie">`) use the shared Media
  service with WebM/VP8 (including alpha) in `crysirror-video`.
- The original blends gamma-encoded bytes; whole-layer opacity is converted
  with sRGB-to-linear (exact over black, approximate elsewhere).
- Picture paths are truncated per component to the 20-byte LIBP name field.

## Saves and settings

Slots follow malie.ini (`SaveFileMax=240`, 20 per page, quick from 200, auto
from 220). A record holds the interpreter state, ADV presentation, tag state,
BGM/environment audio, backlog and a thumbnail captured on demand. `g_*`
EXEC symbols are system-wide (persist across loads and sessions), as are
seen CGs/movies/music, recorded scenes and read labels. Settings follow the
config page ids (`*_opt`, `*_chk`, `*_vol`) with malie.ini defaults. Records
use CrySirror's own versioned JSON in the save mount; the original
`savedata` format has not been recovered, so original files are neither
read nor written.

## Platforms

Desktop (macOS/Metal) offscreen behaviour is exercised. The adapter is
device-free; mobile, browser and console hosts are not validated, and the
`crysirror-squirrel` C++ boundary has only been built for desktop.

Validation notes: [Silverio Ragnarok](../../docs/validation/malie-ragnarok.md).
