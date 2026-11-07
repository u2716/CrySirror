# ADR 0095: Installed AVG32 games over a shared raster device

Status: implemented installed-game runtime profile.

## Context

The retired portable adapter linked the `codex/avg32-portable-vm` build of siglus_rs,
which compiles out every graphics, transition, menu and save path; it could
show only opaque full-screen pictures and single-line text from explicit
TPC32 files. An installed AVG32 game (Kanon: PDT routing, `#WAKUPDT` window,
multi-buffer copies, dither fades, `#CDTRACK` music, `SAVE.INI`) needs all of
them.

## Decision

- The AVG32 interpreter is xmoezzz's implementation, maintained directly in
  `crysirror-avg32/src/vm`. Native scenario, value, wait, menu and save policy
  remains in that VM; shared mechanisms use CrySirror services.
- The reusable pixel model moves out of the VM: `crysirror_image::avg32` owns
  the 32-buffer RGB+mask bank, its copy/mask/colour-key/swap/fill/fade/
  mono/invert/stretch/get-put primitives and every native transition.
- The presented screen is one engine dynamic surface Layer. Characters the
  original would rasterise onto buffer 0 are retained glyphs in an engine
  GlyphRun Layer; a glyph disappears when buffer 0 is overwritten under it
  (bank write damage), which is when the native frame lost it. Menus,
  backlog and name entry become engine solid + GlyphRun layers.
- Audio is device-free: the original sound policy emits commands on engine
  channels; "playing" is the engine's channel state.
- Files and the original `SAVE.INI` go through client capability storage;
  only the save mount is writable.
- Config is `{root, font, encoding}`; everything else comes from the game's
  own `Gameexe.ini`.

## Consequences

- No private font rasteriser, mixer, window or file access remains in the
  VM. Glyph timing follows buffer-0 writes rather than native present
  timing, and text copied between PDT buffers is not carried (reported as a
  warning when drawn outside buffer 0).
- Cinepak movies, framework snapshots, `FN.DAT` novel glyphs and `＊００`
  external glyphs as engine glyphs remain open.
- The AVG32 raster primitives are shared Rust code but not yet part of the
  script `Bitmap` API.
