# AVG32 engine integration

The AVG32 interpreter is xmoezzz's implementation, maintained in CrySirror as a
first-class workspace VM. Native scenario, resource, wait and save policy stays
in the adapter; reusable image, text, video and audio mechanisms use shared
CrySirror services. License: MPL-2.0.

## Where each part runs

| Part | Implementation |
|---|---|
| Scenario bytecode, flags, Gameexe.ini, menus, save layout, meswin layout | `src/vm` (original logic) |
| PDT buffers, copies, masks, fades, stretches, all transitions | `crysirror_image::avg32` (shared raster device, moved out of the VM) |
| PDT10/11 decoding | `crysirror_image::pdt` |
| Cinepak AVI movies (RIFF demux, CVID decode) | `crysirror_video::avi` (engine video service); playback policy in `src/vm/movie.rs` on the engine clock |
| `0x66` text into PDT work buffers | engine glyph-image jobs (`ImageMetadata::begin_glyph_image`), coverage blended with `PdtManager::draw_coverage` before the scenario continues |
| Presented screen | one engine dynamic surface + Layer |
| Message, choice, menu, backlog and name-entry text | engine GlyphRun layers (`vm/text.rs`, `menu::Overlay`) |
| Menu/backlog panels | engine solid layers |
| BGM (CD tracks, DirectSound WAV), WAV, SE, KOE | engine audio channels (`sound::AudioCommand`) |
| Game files and `SAVE.INI` | client capability storage (`vm/fs.rs`), saves on the save mount |

## Port changes

File access, clock, audio device, font rasterisation, frame composition and movie
decoding are replaced by engine services or visible failures; interpreter
semantics are unchanged.

## Known gaps (fail visibly or are recorded as warnings)

- Work-buffer text uses the project font through engine coverage; GDI
  hinting/metrics of the original system font are not reproduced.
- AVI movies with a non-Cinepak video codec or compressed audio: video fails
  visibly; compressed audio stays silent.
- `NVL_SYSTEM` `FN.DAT` and `＊００` external glyphs still blit game-supplied
  bitmap glyphs into buffer 0 (pixels, not engine glyphs).
- Glyph visibility follows buffer-0 write damage, not the native present
  timing; buffer-0 copies taken while text is shown do not carry the text.
- Framework snapshots are unsupported; the original `SAVE.INI` slots work.
