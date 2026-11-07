# QLIE

The native `.s` interpreter calls CrySirror's resource, Layer, Canvas, UI,
image-processing, Media and Audio services. The desktop host owns the window,
input and output devices. No original engine executable or DLL is loaded.

```sh
./target/release/crysirror play ~/Downloads/game-QLIE-5
```

Detection reads the original `GameData/*.pack` files, mounts them in data/patch
order and reads the canvas size from `System/Etc/Game.txt`. Installation sources
do not need to be unpacked or converted into a project file first.

Resource support includes FilePack 3.1 names, embedded key files and data ciphers;
ABMP nested resources; DPNG tiled images; old compact and current animation keys;
native external value banks, image replacement;
and archive-backed audio/video. Image decoding and media preparation run on the
shared engine workers. Native collision tracks become UI hit rectangles, and
named button motions respond to engine-owned interaction state.

The interpreter retains native variables, arrays, expressions, include scope,
branches, loops, calls and waits. Reader replacement and ET/NT commands execute
inside the same VM. Formatted messages use shared cell layout, font resources,
positioned glyphs and engine reveal clocks. Native `*MesSurface` bindings place
those glyphs within the original ABMP animation hierarchy. Ruby, width conversion,
line-breaking rules and formatted numbers use the shared text crate.

`Layers.setParent` preserves an owned node's paint and clocks while changing its
parent. `Layers.advanceTime` advances one client's automatic layer motion, text
reveal and transitions, independently of other clients and external producers.
Both APIs also work in authored CrySirror scenes without this adapter.
Camera oscillation uses shared `ScalarOscillator` signals and versioned Layer
external-motion producers. Constant-colour screen filtering uses the shared
`RgbLookup::screen_constant_byte256` arithmetic and image workers. Quote-aware
native data splitting preserves nested filter and generator parameters.

Onboarding mode: `adopt` (ADR 0102). The crate keeps native VM, resource,
wait and save policy; reusable algorithms live in shared crates.

Saves (`dat`/`mdat`/`tdat` `s`/`l`) write the native IMOSAVE110 envelope
(signature, encrypted key tag, version, TDateTime, metadata strings, JPEG
thumbnail at `ThumWidth`x`ThumHeight` q94, checksum) around a tagged
CrySirror VM-state body. Restores rebuild objects, layers, cameras, image
replacements and looping sounds through engine jobs. Original-engine save
bodies and IMOSYSDATA12 SystemData files are reported and left untouched;
they are not imported. `SystemData.dat` (shared calculator bank, read logs,
read lines) and `GameData/System/Config.txt` live in the save mount.

Shared mechanisms used by this adapter: `pointer_swipe` (drag, fling and
gesture direction), `pcm_filters` (echo, reverb, chorus, volume,
distortion, noise, 3-band EQ, mono, pitch shift, time scale, AGC),
`box_downscale` (thumbnails), `tiled_transform` (transition-rule pose and
range stretch) and `lane_feedback` (save, pack and list-file streams).

Input follows the native device: skey bindings, `inp\k2`/`kl2` and click
checks evaluate key codes in their 5C8074 mode (pressed, click release,
repeat and hold after `KeyCallbackTime`, down, released, down without drag)
with the exact Ctrl/Alt/Shift set; gesture keys 0x100..0x121 come from the
shared swipe tracker under `MouseGestureEnable`. `inp\i` blocks input,
`sk` posts synthetic keys, `ms` and `MouseHideTime` drive cursor
visibility. The cfg engine-setting record (505FE8 defaults, 50786C
persistence) is complete apart from `fnl`.

Scene transitions cover alpha, rule-image (with the `fdp` pose), animated
ABMP, colour-blend and mosaic modes. ABMP dual-image rule drawing uses the
shared affine mask filter; pixel parity has not been verified.

Remaining gaps (2026-11-07). The packed method codes each native
dispatcher accepts and the 61 statements of 6D648C were enumerated from the
executable and compared with this adapter's handlers; every enumerated
method and statement has a handler. Single-character methods dispatched by
`switch` tables are not all enumerated. Behaviour notes:
- twitter `auth`/`post` are logged and ignored (the Twitter v1.1 posting
  API is retired); its settings methods work.
- Native waits keep absolute host deadlines instead of per-frame
  countdowns; `msd`/`?msd` convert between the two. Stored records for
  non-running frames apply when that frame runs again.
- `bsno` reads its four key literals, name and SHA-256 from the game's own
  executable (the literals are not copied into this source).
- `sys\r` resets objects, effects, readers and banks (svar, registry,
  read flags and process strings survive) and jumps to `label` in `file`.
- Platform differences: `fnl` lists the host's font aliases where native
  enumerates Windows families for `system*` entries; `sv` sums the 128
  decoded samples of each probe (the native sum also covers 128 stale
  buffer samples whose values are not reproducible).
- Host-specific values without a CrySirror counterpart are kept and
  reported but have no effect: cfg TaskTray/Multicore/CRC/DirectSound/
  BGImage/close-dialog settings and TouchDeviceMode, sys `hdr` (output
  limiter), snd `pm`, eff `mb` (full-redraw request), skey `dc`, image `ss`
  (5D0498 stores four values nothing reads).
- `reg` keeps a registry of the game's own keys in the save capability
  (`Registry.json`); keys of other software are not visible.
- Saves keep each sounding voice's position, remaining loops and pause
  state (TssSaveData) and a message's reveal position (TMLSaveData);
  picture-glyph sheets are reloaded on restore.
- Not implemented: importing original-engine save bodies (TImoSaveData).
  The authorized test installation contains no original save to work
  from.

Format and syntax references:
[QLIE analysis](https://habr.com/en/articles/426431/),
[QlieScript.cs](https://github.com/arcusmaximus/VNTranslationTools/blob/main/VNTextPatch.Shared/Scripts/QlieScript.cs),
[qlie-unpack](https://github.com/rafi441/qlie-unpack),
[QlieKit](https://github.com/Inori/FuckGalEngine/tree/master/QLIE).
