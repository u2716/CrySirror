# System-NNN

[Opcode and syscall implementation list](../../docs/native-api/system-nnn.md).

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
  System SE keeps its configured preload snapshot and two alternating channels;
  button voices have separate channels, duplicate suppression and character
  volume rules. Logo audio follows the original per-page timing and one-shot
  flags; replay voices retain packed-before-loose lookup and character gates.
  Connected primary General actors preserve delayed entry/exit sounds and
  voices across mode changes. PrintMovie, PrintCG and ChartMenu share their
  native frame admission with General; resource/audio completion does not
  consume an extra update. Movie exit waits count native frames.
- Default user commands follow the legacy `<name>IsDefault` registration over
  `spt/command.xtx`. Their receipt stays pending while the mode actor runs and is
  acknowledged at its SetYoyaku point. `printmovie` plays the movielist entry
  through shared Media and `lastWait` native frames. `selectplace` is a modal
  CCommonButtonGroup over its background SPT, with exist/enable words read
  through native GetVarData. It writes `selectedPlaceVarName` = place + 1.
  Unregistered commands go to the game's own override, which is not reviewed
  and fails explicitly.
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
  ANIME (19) selects original bank pictures after native Calcu, preserving the
  selected slot's geometry and its distinct opaque, mask and translucent paths.
  CHANGE (40) composes independently weighted source generations through shared
  packed pair blending; its original endpoint parameter mutations remain native.
  SPECIALMOVE (61) retains independent Print clocks for movement, transparency,
  scale and colour while shared scanline operations draw every branch. TACLIP
  (136) uses shared directed mask clipping, packed fade arithmetic and strokes.
  BOSSDEATH (48) preserves native per-row jitter and its disabled NOP endpoint,
  including original save flags and background-layer transparency reset.
  ZANTETSU (86) splits a masked source in left/right painter order through
  shared fixed scanlines and byte-truncated coverage. CHARAMOZAIC (139) uses a
  shared cancellable block sampler: mask-dependent random search, image and
  resulting RNG state complete together; the VM never scans source pixels.
  GEOCIRCLE (87/115), GEOTORUS (89/116), GEOBOX (114/118) and GEOPOLYGON
  (88/117) retain their ordinary picture pass before shared geometry painting.
  Ellipse spans, cutout policies, stored-byte mixing and colour offsets are
  reusable engine operations. The source's box-add ellipse call, polygon-add
  whole-screen fill and torus row/cutout behavior remain explicit native policy.
  FULLANIME (79) reads numbered frame files through the session's scoped
  file view, then prepares index and palette image jobs asynchronously. Its
  last-print cache belongs to the effect container across slots; missing frames
  retain the existing image and still advance that native cache. Raw restore
  preserves its sixteen parameter words without replaying SetParam.
  RAIN (13) retains one shared particle bank per native effect container:
  ordered setters, Calcu and save initializers preserve the original random
  stream while shared integer lines draw the rain.
  KONOHA (104) emits ordered atlas sprites through bounded shared composition
  batches. WATER (24) retains its cross-slot Print clock and uses shared row
  sampling; its restore hook resets effect counters as in the native setter.
  WATERDROP (95) retains the original full-screen colour-fill behavior.
  RAIN2 (56) repeats natural-size particle planes through shared tile sampling.
  KAGEROU (22), WATERFALL (91), GRASSPLANE (93) and SOYOSOYO (92) use
  shared displaced scanlines with their distinct stored-mask arithmetic.
  FISH (111), KE (27), HAE (108) and blink/lip animation (98) retain native
  per-object timers, atlas coordinates and random order. Lip animation follows
  the measured voice-duration countdown rather than a guessed mixer flag.
  REMEMBER (26) uses reusable quantized row-edge offsets; TAIYO (51) uses
  shared squared-radius circular offsets. PERS (67), TURN (64), TURNNEW (140)
  and MOVIE_WARP (105) use shared wrapping fixed-point, double affine and
  radial samplers. MASKWIPE (60) and MOVEMASKWIPE (101) compose full-canvas
  picture/capture pairs with shared RGB-mask coverage and signed two-axis
  mask displacement, retaining native crop, endpoint and low-word arithmetic.
  CIRCLE (28) retains its integer aperture boundaries and native end-of-count
  transition into Fill or NOP, including the original Fill parameter indexing.
  SLASH (76) uses shared sliding-window sampling with native packed-word
  normalization and delayed line/column phase changes.
  KIRA (42) loads its constructor-owned system bitmap through shared resource
  workers, retaining the shared star bank, per-Print random order and original
  save reinitialization; colour extraction and saturated addition use engine jobs.
  RAIN4 (102) retains ordered depth planes and per-slot motion. Shared repeated
  scaling samples virtual tiles in canvas-sized work, with the native clipped
  starting phase and separately quantized16.16 steps.
  SPEEDLINE (85) keeps its shared line bank, ordered Calcu updates and save
  reinitialization. Shared RGB spans provide the native saturating gradient
  weights, including its distinct lower-row overlap.
  SAN (15) retains seven global light-band counters and ordered Print/RNG
  updates. Shared stored-word RGB spans perform its two-sided additive ramps,
  retaining native signed phase, clipping and low-word saturation.
  KOMOREBI (78) uses shared dual-mask RGB offsets with native independent
  linear strides and full-coverage identity. CHARAMETAL (112) uses shared
  spatial RGB bands with packed colour, native diagonal phase and retained
  mask allocation; original picture assignment and mask-exists flags do not
  replace the helpers' own resource checks.
  Source32 ANIMATION (137) uses one depth-selected RGB/indexed image bank for
  effect Print and CG preload/display. File reads, palette expansion, blank
  planes and paired transitions use retained shared jobs; native filenames,
  loadOk, cached failures, lastAnime and the original17 parameter words remain
  native state. The class is admitted explicitly through
  `Bridge::poll_source32_animation_configuration`; it does not change MUGEN2
  Legacy, whose constructor has no137 class. The live native tag and off flag
  have separate setters. Palette observations from ordinary pictures, STAFF,
  CG and animation commit in native read order.
  HAHEN (81) retains partition files, procedural grid jobs, class-wide caches
  and RNG candidates until the native setter completes. Shared floating-edge
  polygon raster jobs draw its translated fragments; original-save initialization
  rebuilds class data in native slot order.
  BATHWINDOW (97) plans ordered Calcu/RNG updates and completes retained-mask
  brush jobs before the first Print, including the post-text screen layer.
  Filename flushing, mask capacity and native source trim remain independent
  of the displayed image; zero-write updates preserve the original metadata.
  BEACH (83) updates its shared wave table for all active slots before Print.
  HANABIRA (70) maps single-cell, translucent and paired-frame sprites to shared
  image batches; HOTARU (71) uses shared signed radial brushes. HOTARUNEW (138)
  keeps its distinct Source32 constructor/profile and stored-mask sprite math;
  the MUGEN2 profile does not invent this originally absent class or its RNG.
  CLOUD (82) retains its class-wide Print clock and packed repeating-row
  sampling, including signed high-word wrapping and explicit coverage math.
  FLOWER (12), SNOW (14) and SNOW2 (57) use constructor-owned shared images,
  indexed fallback resources and class-wide particle banks. Constructor work
  is asynchronous, and its random-state candidate commits with the prepared
  images; native setters and original-save hooks retain their distinct rules.
  HAMON (69) keeps its class-wide radial field as an engine data-image lease.
  Setters and original-save restoration await cancellable field construction;
  shared two-source sampling preserves packed radius offsets, signed-vector
  arithmetic, capture selection and independently positioned picture origins.
  Native picture mask storage retains its capacity and byte
  history independently of the current file's mask-exists flag, through
  cancellable shared channel-buffer image operations.
  Title and film-button animation select their current/next cells through the
  shared integer sprite sampler; native Print retains clock advancement and
  percentage quantisation.
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
  Effect records retain untouched parameter tails and padding across setters,
  clear operations and raw restore; Fill also retains signed parameter words.
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
Chart grow tabs use the original dual-state skins and masked hit regions;
new-mark animation retains its native Print clock and selected-map acknowledgement.
Replay confirmation and option controls still use shared controls rather than
complete native skins. Source SystemFile profiles retain the14/16-record
layouts, old/expanded system-variable banks and the independent SYSTEMFLAG2
record; VERSION2 button voices read its live catalogue preference. These
explicit source layouts do not identify a new executable ABI: the installed
bootstrap remains the reviewed Mugen2 profile. The map editor's persistence,
Source32 executable bootstrap and its native preference controls,
other saved outer actors beyond PRINT/SELECT/Chart,
Native script callbacks for custom animation tags, character-specific replay-voice controls, unconnected
special modes and debug all-on controls remain unsupported.
Load/Save/Config and catalogue controls use shared modal UI; their full original
skins and unconnected General-mode/secondary-actor entry profiles remain
unavailable. The primary actors' connected General audio policy is separate
from those missing skins and mode profiles.
Explicit Source32 character-volume controls
(`Bridge::configure_source32_character_volume_controls`) use shared sliders and
toggles; old Config setup keys do not activate a class absent from MUGEN2. Their
signed five-bit volume fields and transposed voice switches write the original
SystemFile; released sliders prepare typed system-sound previews through the
existing audio owner. These shared controls use paged layout independently of
the native skin pages. Reviewed MUGEN2 Config entry stops its8 script-sound and4
voice channels while retaining BGM and loop-work. Native Config reset-button wiring, dynamic character
labels/voice substitutions and complete native Config skins remain separate
work.
SelectPlace character overlays, autoPicLoad, hover variables, keyboard
movement and the in-mode option buttons/game keys are not connected; the first
three fail explicitly. Other default user-command modes (selectheroin,
printcalendar, printresult, nameinput, selectplace2, selectzukan) have no actor.
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
