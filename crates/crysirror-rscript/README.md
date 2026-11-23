# codeX RScript native adapter

[Opcode table, ABI profile and blend modes](../../docs/native-api/rscript.md).
[Native plugin decision](docs/adr/0001-native-plugin-dlls.md).

Onboarding mode: **adopt** (ADR 0102). This crate owns the RScript GSC
program reader, the dispatcher's operand layouts, the script thread
(control flow, calls, expressions, nested system calls), the native scene
records (configuration and scene blocks in their original layout) and the
layer, text box, choice, button, wait and audio policies of the engine.
Archives, WCG/LWG decoding, composition, transitions, text rendering, audio
and movies are shared CrySirror services. No original EXE/DLL is executed;
the executable is read as data to select its APINI layout.

## Layout

- `gsc.rs`, `layouts.rs` — program sections and the 1.11 operand table.
- `apini.rs` — APINI block layouts by verified build; FileVersion check.
- `memory.rs` — variables, registers, configuration/scene blocks, defaults.
- `text.rs` — `@`/`$` expansion, message markup, fixed-cell layout with
  hanging/opening kinsoku and ruby, glyph reveal.
- `sprite.rs` — fade and point-motion components with native integer math.
- `scene.rs` — script layers (entrance/exit effects, moves, blend presets,
  buttons, hover overlays), text boxes, companion panel, choice window.
- `screens.rs` — map screen, paged selection, gauges, text-entry dialog.
- `save.rs` — original slot/system file layouts, the save/load screen,
  thumbnails, quick save/load and the system save.
- `states.rs` — game-state leave/enter (load, previous choice, title, quit).
- `extras.rs` — title, legacy top menu and the states reached from it.
- `gallery.rs` — CG view, scene view, music menu and extra menu.
- `config.rs`, `status.rs` — configuration screen, font window, tab pages.
- `panel.rs` — panel commands, backlog browsing, button mode 1, auto hide.
- `anim.rs` — kind 2 animated layers (`.fsc` flows).
- `zoom.rs` — zoom/shrink transitions and the state-switch fade.
- `plugins.rs` — built-in plug-in facades.
- `vm.rs` — dispatcher and handlers; waits and continuations.
- `owner.rs` — fixed-period tick, input, wait resolution.
- `present.rs` — the native priority tree as shared retained layers.

## Shared capabilities added for this engine

- `crysirror-formats-arc` `liar-soft/lwg`: canvas height/width order fixed,
  16-bit entry count, per-layer `x`, `y`, `format` metadata.
- `crysirror-audio`: WAVE files with Vorbis ACM format tags (0x674F..0x6771)
  decode their embedded Ogg stream through the shared Vorbis decoder.
- `crysirror-image` `wcg_encode`: WCG writer and the percentage point
  sampler (save thumbnails); `cel_convert` (`ImageEffect::CelConvert`):
  mirror, rotation/zoom and tone conversions of image loads.

## Platforms

| Target | State |
| --- | --- |
| Desktop (wgpu + winit) | Compiles; linked into the player through `crysirror-launch`. |
| iOS / Android / browser | Same adapter code; no platform-specific work. Not built for this change. |
| PS Vita / Switch | Uses only shared layer/transition/text/audio services; console builds not run. |

## Plug-ins

Game DLLs are never loaded. Opcode 0x19 and the APINI extension DLL go
through `plugins.rs`: a built-in facade per canonical DLL name, the native
fallback when the game does not ship the DLL, a visible failure for a
shipped DLL without a facade. Approximations are listed in the
native-api page.

## Provenance

`SOURCE.json`: the verified build and the functions its behaviour was
recovered from. CrySirror code under MPL-2.0.
