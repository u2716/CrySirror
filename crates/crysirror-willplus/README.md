# WillPlus / AdvHD

[Opcode and syscall implementation list](../../docs/native-api/willplus.md).

Mode: `adopt`. Native scripts and resource lookup policy belong here; image,
text, UI, audio and storage will use CrySirror services.

Initialization provides cancellable, client-scoped ARC catalogues and bounded
WS2 instruction decoding for three explicit layouts. Existing shared Will ARC
and Pulltop ARC readers are reused, with no unpacking or local-path requirement.
The caller supplies archive order, entry resource and decoded WS2 profile;
initialization does not guess a game's startup scene or encryption variant.

## Runtime

`adapter` registers `willplus`; the desktop detector (`detect/willplus.rs`)
recognises `AdvHD.exe` beside Pulltop-layout ARCs and mounts every ARC as
`__willplus/<stem>` (canvas 1280x720, start scene `START`). Debug only:
`WILLPLUS_START=<SCENE>` redirects the first scene and
`WILLPLUS_HALT=<SCENE>:<pc hex>` stops before one instruction (captures).

- `vm`: WS2 executor (ror-2 obfuscated scripts, layout V3). Opcode meanings
  are inferred from operand evidence because AdvHD.exe is packed; the
  module doc lists every connected opcode.
- `system`: the AdvHD side of the Lua boundary: `cfunc` table, `cclass`
  objects, `os`/`io` (`save\` paths on the client Save mount, game
  directory read-only) and `LUA_*` constants. Constant values are
  host-assigned except `DATA_MODE_NORMAL/QUICK/AUTO` = 0/1/2, which the
  scripts also pass as literals.
- `layers`: `cclass.lua_Layers` sets (keyed layers, PNA record sublayers,
  relative effects, frame-flip animations, set transitions, hit tests,
  text/thumbnail/preview layers). Layer `order` is one flat space in the
  shared Layer service, so the host flattens its tree depth-first.
- `message`: WS2 text drawn into the scripts' own window
  (`getMessageWindowKey` -> `system/Txt01`, name `system/Name01`) in the
  `CLegacyFontInfo` style; `getMessageSpeed` ms per character,
  `getAutoWaitTime` ms per character in auto mode, Ctrl or skip mode;
  click-wait animation, window buttons (hit-tested `system` sublayers),
  right click (hide), wheel up (log). Choices (0x0F) use `initSelect`,
  `getSelectTextKey`, `setSelectTextColor`, `disableSelectItem` and
  `LegacyGame__lua_SelectItem`. Button availability comes from the
  `LUA_STATE_ENABLE*` states the host sets, followed by `setButtonState`.
- `save`: normal (1-based), quick and auto (rotating 1..5), select-point
  and continue data for `CSaveDataManager__lua_*` and the save/load
  `NeedProcess` requests. A save holds the state at the start of the
  current line or choice; loading re-runs it. Thumbnails compose the saved
  scene (no screen readback). Read lines, picked choices, flags/variables
  >= 1000 (CG/scene unlocks) and registry settings persist in
  `willplus_system.json`. The host must call `checkInitVersion` before
  `initLegacyGame`, otherwise the scripts load no settings.
- `log`: the backlog's log layer (scroll, entry hit tests, voice replay,
  scene jump-back via `LUA_PROCESS_BACKGAME` and per-entry previews).
- Audio receipts from `begin_audio` are kept until preparation is Ready;
  releasing them earlier cancels playback.

Behaviour evidence (headless, `--no-audio`, owner copy of the game,
2026-11-14): title -> START -> CO_01 with the Lua window, names, faces and
the window button bar; CO_11's heroine choice (one item locked by its flag)
-> CHAM_1_01; CONTINUE resumes the last line; config SYSTEM page from the
title and from the window; SAVE/LOAD pages with thumbnails, dates and
text; quick save; backlog with JUMP confirm and jump-back; EXTRA gallery;
0x40 masks (CHAM_2_01 cut-ins) and 0x56 streaks (CHAM_5_06).

Inferred, not verified against the executable: 0x56's effect shape,
0x1B page numbers, the log's line layout, the absence of a visible effect
for 0x42 attributes and 0x46 groups 10/11. WSC execution is not connected.

WS2 operand layouts follow [VNTranslationTools](https://github.com/arcusmaximus/VNTranslationTools/tree/main/VNTextPatch.Shared/Scripts/AdvHd).
Its MIT notice is retained in LICENSE. These are decoding layouts, not evidence
of implemented opcode behavior.
