# System-NNN

Mode: `adopt`. The word-addressed SPT VM and native game policies are maintained
in this workspace crate. Graphics, text, UI, resources, audio and storage use
CrySirror services. There is no hosted original executable or private renderer.

The `runtime` feature provides the installed-game factory. It opens original
loose resources, archives and supported MSI installations through client-scoped
storage. The current native profile is MUGEN2 / Legacy1000 with 16 effect slots.

## Capabilities

- Native expressions, variables, control flow, script transfers and suspended
  callbacks run on cancellable workers. Completion commits on the session owner.
- Black Cyc archives, DWQ images and VAW audio use shared format decoders.
- Pictures, transitions, background shake and image effects use immutable shared
  image jobs and retained Layers. Native counters and global RNG remain native.
- Message windows, names, inline colours, name substitutions, ruby and gaiji use
  shared layout, positioned glyphs and image glyphs. Gaiji come from the game's
  `sys/spfont24` sheet; source data is not bundled.
- Cutin dialogue uses native character-table coordinates, per-row font sizes,
  speaker suppression, signed modes and original saved row records. Solid and
  image windows interleave with row views of one shared GlyphRun/reveal clock.
  Window masks and translucency use shared stored-byte blend profiles; image
  loading and gaiji preparation run on scoped workers. The printer retains all
  64 native RAM rows, including hidden rows used to size Cutin groups; original
  saves retain the source writer's visible-row-only message section.
- Music, sound and voice use the shared decoder/mixer, native volume/fade policy
  and scoped channels. MML supports segment sequences and nested finite/infinite
  repeats without expanding the program into PCM. Playlist clips must share a
  sample rate and provide measured durations.
- Configured quick-save/load uses original `dataN.sav` slots, native thumbnails,
  retained comment fields, notice queues and optional pre-load screen capture.
  Notice animation uses shared image resources and Layers.
- Load/Save use a paged native-slot catalogue and shared modal controls; Config
  edits native system-file settings and applies audio, text and window changes.
  Menus pause the retained gameplay clocks and resume the original wait on return.
  Configured menu background SPT readers share the live variables, RNG and audio
  owner while effect/resource banks are suspended independently. Title return
  reloads its background reader; foreground return retains its PRINT/SELECT
  receipt. A non-looping background's retained effects continue after NEXT.
- Omake opens CG and scene catalogues, original unlock flags, CG difference
  pictures, frame animations, scrolling/tiled CGs, film-sequence scene replay,
  music and movies. CG viewports retain native origins, scroll direction/wait/
  duration, Ctrl-dragging, wheel navigation and unlocked-catalogue wraparound.
  CG replay voices use the original threshold catalogue and native delayed
  trigger, through the shared voice channel and global voice preferences.
  Same-block CG difference navigation uses the original first-tile or last-animation
  source, native per-Print counters and shared wipe graphs. Repeated first-column
  wipes retain their tile-row order and commit random state after image completion.
  The Omake root uses the game's button atlases through shared image jobs and
  scoped modal widgets.
  Menu variants inherit native button columns; extended buttons can return a
  signed scenario value or enter a connected native mode. The return path is
  retained when navigating between modes. Music playback uses original catalogue
  filenames directly, with the configured loop, volume and fades; it does not
  require a script BGM-table entry. Native listen-mode/selection variables are
  updated without advancing the suspended VM receipt.
- Chart maps use original node/route permissions, read flags, percentages,
  scrolling, minimaps and checkpoint replay. The six configured Chart functions
  use native variables and original `chartN/slotN.dat` records. Chart waits also
  round-trip through ordinary saves; replay retains current global variables.
- SystemMenu uses native button images, entry counters and extended mode gates.
  Title, Omake and SystemMenu also retain their General entry capture and
  transition countdown, with background scripts suspended during the entry
  update. Shared image jobs draw the transition; native counters, skip policy
  and the global overlap buffers stay with their mode owner.
  Connected actions include Save, Load, Config, quick slots, message-window
  hide, skip, automatic advance and return to Title. Foreground actions retain
  the suspended PRINT/SELECT receipt. Window hiding preserves glyphs, reveal
  clocks and original per-layer visibility; a restore click does not advance
  dialogue. The live WindowOn flag remains distinct from saved STATUS44.
  Title return retains its original exit capture and configured wipe countdown,
  including the retained-enter overlap selector and native no-print negative
  selectors. Positive exit targets remain available as the global enter buffer.
- All 96 native simple-wipe selectors (0–95) use shared Image operations,
  including shaped reveals, fragments, scanline warps, zooms, panels and random
  wipes. OVERLAP, SIMPLEWIPE (59) and the connected General exit player use
  these operations with native clocks and caller-owned RNG state.
- GET (73), PUT (2) and BUFFER (66) retain engine image leases in painter order.
  ONCE (75) retains its first-print flag and native count window. STRETCH (36)
  uses native packed reductions and fixed-point sampling. LIGHT (20),
  GRADATION (50), COLORBALANCE (120), ADDSUB (131) and character sepia (127)
  use shared colour operations, including native parameter/counter save words.
  Common XY interpolation also runs for hidden picture effects; wipe picture
  indices resolve after those coordinates update.
- BGZOOMIN (37), CHARABURE (132), BOKASHI (77), BEZIERMOVE (62),
  BSPLINE2 (119), RASTER (23), POLESCROLL (94), CURTAIN (68) and MELTDOWN (32)
  use shared
  rectangle sampling, box reduction, Bézier paths, scanline displacement and
  composition. LUPIN (113) uses a shared full-canvas colour input; SAIKE (130)
  uses shared low-word RGB lookups and incremented-mask composition. Native curves,
  RNG ordering, jitter/scroll work words and saved counters remain in this adapter. BSPLINE2 follows the original quadratic
  Bézier formula. Unsafe original uninitialised defaults and out-of-buffer
  writes have defined, bounded replacements marked beside their implementation.
- CHARARASTER (107) uses shared scanline copies with the original 32-bit
  coverage arithmetic and clipped phase. TARGET1 (103) uses shared rectangle
  copies for opaque, translucent and saturated-colour targeting lines. Native
  parameter words, curve/RNG order, line order and original saves stay native.
  NOISE (106) retains sixteen native moving bands, their timers and the global
  RNG sequence while shared scanline jobs draw the displaced masked picture.
  KOMAANIME (63) selects atlas cells with the original count-loop and position
  interpolation, including low-word translucent-mask arithmetic. CHARAGHOST
  (110) applies native per-row coverage through shared scanline image jobs;
  its source32 clipping phase and separately rounded RGB products are retained.
  DUALMASK (121) and CHARAmaskAppear (123) use independent alpha/RGB-mask
  resources through shared three-input image operations. STRETCHBLT (65),
  CLIPZOOMIN (122) and SENTENCE (133) use shared fixed scanline sampling,
  directional clips, stored-colour adjustment and optional floating strokes;
  native atlas selection, curves, RNG order and original save words stay here.
  RELEAF (33) uses shared neighbour differences and row-end repair; OLDFILM
  (55) retains its line timers and Print-time random sequence over shared fill
  operations. CHARABOKASHI (125) and BGBOKASHI (126) use clipped shared box
  sums with the original distinct mask/RGB normalisation profiles.
  MEISAI (72) displaces the existing backdrop through a silhouette mask.
  RAINBOW (96) keeps one palette per effect container: ascending-slot Calcu
  updates precede every Print, and native restore preserves its random draws
  relative to other initialized effects. Shared image operations own row tint
  blending and masked backdrop sampling.
- DWQ picture metadata retains native source trim independently of mutable
  effect rectangles. Mask bounds are queried on a cancellable image worker;
  stored already-cut margins and mask-palette red coverage retain native
  semantics. Indexed/RGB PNG transparency follows the native mask policy
  through a shared alpha operation without changing generic PNG decoding.
- Native extensions can use `NativeAdapter::change_user_gaiji` to prepare and
  switch two-byte user glyph sheets asynchronously. Existing displayed glyphs
  retain their image generations. The original base SPT dispatcher defines no
  user-gaiji switching opcode; this API does not invent one.
- Configured film-skip buttons use shared hit masks, UI events and image
  composition, retaining native click delays, decorations and sound queues.
  Selection finishes an admitted frame before cancelling the DRAW countdown;
  it does not invent effect steps or acknowledge an unfinished image job.
- Special choices use count-indexed native window positions, variable-selected
  atlas cells, centering, separate supplemental text and native cursor counters.
  Cursor mattes use shared prefix capture and exact masked-pair image jobs.
- Original saves restore message and ordinary/timed selection waits, images,
  effects, native script contexts and audio in a single staged transaction.
  An idle title can be replaced without first changing the visible title or UI.
  Last-screen effects retain their native post-text ordering through detached
  shared Layer composition; preparation does not advance native effect counters.

## Limits

Omake's 100×32 table and Cutin records round-trip through original
save sections. The reviewed manual EXT callbacks clear their payloads on save
and ignore them on load. Reviewed automatic mode callbacks that are genuinely
empty retain their original zero-sized blocks before manual EXT blocks.
ChartMenu's selection words and road bytes retain their original EXT layout,
alignment and initialization; NewGame clears roads while retaining permissions.
Chart map tabs/replay confirmation currently use shared controls rather than the
complete native skins. The map editor's persistence, Chart new-mark animation,
other saved outer actors beyond PRINT/SELECT/Chart,
Custom animation tags, character-specific replay-voice controls, unconnected
special modes and debug all-on controls remain unsupported.
Load/Save/Config and catalogue controls use shared modal UI; their full original
skins, other General-mode entry profiles and General-mode entry/exit voices
are not reproduced.
Menu background scripts that request a foreground Message/Selection mode transfer
remain unsupported. Film-skip button profiles outside the reviewed common-button
parameters fail explicitly.
Resource jobs and in-flight native callbacks must reach a safe boundary before loading.
Unknown instructions, callbacks and layouts retain their original PC and fail.
These limits apply separately from resource-format recognition.

## Platforms

| Host | Boundary |
| --- | --- |
| macOS | Release library compilation and scoped offline checks; no new windowed game run is claimed here. |
| Windows / Linux | Portable Rust services; this change was not built or exercised on those hosts. |
| iOS / Android / browser | Installed-game factory is not connected to these hosts. |
| Switch / PS Vita | Deferred. |

Native findings are kept beside their implementations; source identities and
notices are recorded in `SOURCE.json` and licence files. Execution evidence is
separate from these current contracts.
