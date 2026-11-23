# minori native ABI profiles

## Scenario selection

`sc-next` identifies the supported text-scenario contract. NEXT transfers take a script name and an optional label, preserve local variables and replace the current script. Includes push a frame. Input files can interleave Japanese and English instructions. A wait unit in this adapter is 10 ms. The generated project and presenter use the dimensions of the installed full-screen white backdrop. Older projects without a canvas setting retain their 800 × 600 default.

Installed-game detection scans i386 PE initialized data for the NUL-terminated command literals `.if`, `.chain`, `.HSVfilter` and the `NEXT` key. The detector requires references to all four literal addresses in executable sections. A candidate must also open the script bank and parse the startup scenario. Untagged instructions are valid, including in a monolingual installation. Ambiguous executables and missing dialect evidence produce a diagnostic. These are structural recognition criteria for the evidenced subset. The executable name identifies a file for reading and has no role in the VM dispatch decision. An authored project can explicitly request `sc-next`. Previously exported projects using the older label remain readable.

## Retail command registries

The i386 constructors at `0x4765a0`, `0x451fbe` and `0x476e20` register 38, 45 and 52 dot commands respectively in the three reference executables. CrySirror reads the referenced command strings between the opening `.label`, `.select`, `.panel`, `.movie`, `.movie2` registrations and the closing `.chain`, `.end`, `.debug` registrations. Generated projects retain that registry. The loader checks it for each script, including scripts opened by a chain or include. Original program addresses below identify the analysis samples only.

| Registry | Names added to the preceding row |
| --- | --- |
| 38 | `.HSVfilter`, `.chain`, `.debug`, `.deleteVar`, `.effect`, `.end`, `.endScroll`, `.exec`, `.frame`, `.goto`, `.hscroll`, `.if`, `.label`, `.message`, `.movie`, `.movie2`, `.overlay`, `.panel`, `.playBGM`, `.playSE`, `.playSE2`, `.playVoice`, `.pragma`, `.screen`, `.scroll`, `.scrollAcc`, `.scrollFG`, `.select`, `.set`, `.setGlobal`, `.setTimer`, `.shakeScreen`, `.stage`, `.transition`, `.vscroll`, `.wait`, `.waitTimer`, `.wscroll` |
| 45 | `.animation`, `.char`, `.cutin`, `.rotateBG`, `.rotateFG`, `.waitAnimation`, `.waitBGM` |
| 52 | `.effect2`, `.playBGM2`, `.playSE3`, `.playSE4`, `.scrollXF`, `.setFG`, `.sqz` |

The roster describes native registration. The three samples also differ in resource decoding and presentation. Named `@` controls have their own dispatcher.

Four more i386 samples expose 33, 42, 47 and 52 commands. The first command-name references in their constructors are at 0x45f4f9, 0x458860, 0x4632a6 and 0x479729 respectively. The 33-command constructor registers `.transition` directly after `.movie`. Its startup uses `.mis` scenarios, `.inc` includes and a headerless PAZ index. Its movie bank reuses one 256-byte permutation. The 42- and 47-command samples share the compact bank-name table layout but initialize their member parameters differently. The latter also discards a CRC-derived prefix of the RC4 stream. The newer 52-command sample uses a twelve-bank table and stack-initialized member properties.

In the 42-command sample, the parser at 0x477019 and executor at 0x4770ba carry a separate soundtrack for `.movie2`. On natural EOF the scenario resumes. In the newer 52-command sample, New Game opens a six-button route screen. The constructor at 0x48e950 references the six native script commands and rectangles used by the click handler at 0x48f500. Detection reads those arrays from the installed executable. In the 33-command sample, 0x45d870 draws panel mode 10 at (17, 412) using the message-panel alpha and configured ARGB color.

## Scenario timer

`.setTimer [value]` assigns a process-clock counter. `.waitTimer [target]` yields until the unsigned 32-bit counter reaches the target or the scenario skip predicate resumes execution. Both parse the first operand as a signed decimal prefix, defaulting to zero, and store its 32-bit representation. Additional operands are ignored. The clock starts at zero during engine initialization and advances every 10 ms while other instructions, input waits and movies are active. Assigning the counter preserves the phase of the next clock tick. Script transfers retain the counter.

The 33-command constructor binds the write to 0x478750 and the wait to 0x478820. The corresponding pairs are 0x495ba0 / 0x495d90 (38), 0x470930 / 0x470aa0 (42), 0x469419 / 0x4694f3 (45), 0x44d2a0 / 0x44d400 (47), 0x45c430 / 0x45c4f0 and 0x45e520 / 0x45e5e0 (the two 52-command samples). The 52-command timer callback at 0x403160 increments whole ticks and carries the sub-tick remainder. The wait dispatcher at 0x46221a compares unsigned values.

### Dot-command inventory on 2026-11-22

Nine retail installations expose 52 distinct dot-command names. The scenario reader and dispatcher cover all 52, including label binding at parse time. Several distributions include a second executable with the same registry. Counts measure top-level dispatch only.

| Registered names | Names with a dispatch path | Missing names |
| ---: | ---: | ---: |
| 33 | 33 | 0 |
| 38 | 38 | 0 |
| 42 | 42 | 0 |
| 43 | 43 | 0 |
| 45 | 45 | 0 |
| 47 | 47 | 0 |
| 51 | 51 | 0 |
| 52 (three installations) | 52 | 0 |

The two rotation commands first appear in the 42-name registry. The animation pair is present in the 45-name sample, foreground replacement and frame selection in the 47-name sample, and the independent second effect slot in the 52-name samples. Executable references select the registry at launch.

* `.rotateBG` / `.rotateFG` accept `set`, `start`, `end` and `wait`. Each plane maintains its own 10 ms motion clock, source and destination origins, direction and angular endpoints. The late executors at 0x459ee0 and 0x45a540 route to a ten-bit nearest-neighbour inverse transform.
* `.animation` owns a catalogue of MMV scene objects. The late executor at 0x453e70 loads a standing-bank clip by name and supports play, pause, position, parent, loop and clear. The reader interprets the 76-byte and 136-byte track layouts. Image and solid tracks have frame intervals, position keys and draw priority. `.waitAnimation` at 0x45e090 selects one logical ID or the set of playing objects.
* `.setFG` at 0x45c080 replaces the foreground independently of the background and character catalogue. Compound materials select the main image, camera-relative auxiliaries and fixed artwork. The presenter loads v1 standing-image ANI patches. Custom eye sequence selection still needs verification.
* `.sqz frame NAME N` at 0x45ca50 selects a frame on an existing named SQZ bitmap in manual mode. SQZ1 has a 20-byte header and independently compressed BGRA frames. The shared reader validates offsets against the scoped asset length and reads only the selected frame. Positive rates advance with the clock.
* `.effect2` at 0x4564e0 uses the same effect dispatcher at 0x466d90 as `.effect`, with a second retained object, render target and retirement flag. The late catalogue has 18 named effects. See the presentation SATD for the implemented selectors and outstanding work.

The late .wscroll initializer at 0x466860 saves the initial horizontal origin and derives the vertical amplitude from the image and canvas heights. The update at 0x460360 uses a 10 ms tick. The blitter at 0x43b570 wraps across the right edge and clamps vertical sampling. .scrollFG at 0x466630 moves the foreground independently. .scrollAcc at 0x4666b0 selects quadratic ease-in for positive durations and ease-out for negative ones. Cursor coordinates can reach their final pixel before the duration expires.

The late secondary music and fourth SE executors call 0x464fc0 and 0x4660c0. They own separate voices and use 200-tick default fades. The primary music executor resets the BGM clock. A BGM wait resumes on the first tick strictly greater than the target or on permitted skip. The standalone .cutin has a named bitmap catalogue at 0x44dc70–0x44e020, separate from the moving Cutin effect. It permits load, present, release and releaseall. An empty present hides the image while retaining it.

The .exec handler drops the two stage source-image caches, then passes the command name and space-joined tail to the current scene. Stage-preparation workers in the adapter drop source images before the next scenario command. Published planes and the cutin catalogue survive. In the 38-name registry the scene callback at 0x421420 emits a debugger message. In the 42-, 45-, 47- and two 52-name registries the corresponding callbacks return immediately (0x41dfdc, 0x4183ef, 0x439460, 0x47f780 and 0x482560). The .debug executors clear an input latch whose other direct references are writes. They do not change the scenario control/skip flags.

Three further i386 constructors register 43, 51 and 52 names. The archive, title and screen-effect ABIs vary independently. The 43-command sample uses the ten-bank PAZ reader, a six-button 800 × 600 title and an unlockable episode selector. The 51-command sample combines a twelve-bank reader with a 1280 × 720 stacked title. The 52-command sample uses five time-of-day artwork families. Its title switches to D during the first five minutes of the 19th hour and to E for the rest of the night. Bank pointers, command registrations and the mounted UI assets identify these contracts.

In the 43-command reference, the title constructor at 0x40be5e reads FD_01 and FD_10. The selector at 0x40cf0b enables adjacent chapters from the individual clear flags and copies unlocked button regions onto the base artwork. Its click handler at 0x40e09b uses a one-based array of nine script pointers and sets exitFromEvent to 100 before entering the selected script. In the 51-command reference, the title constructor at 0x47f430 distinguishes the disabled-load artwork from the unlocked memories artwork. Its settings hit test at 0x491ed0 and knob placement at 0x491470 use the flat configuration bank. In the five-period reference, 0x481b50 reads local hour and minute, then selects the progress overlay from Groute3, Groute2 and Groute1, in that order.

## Comparison of public command inventories

The maintained AetherMinori registry has **32** named dot commands. ReMinori lists **24** structured dot commands and all 24 occur in the first registry. The eight additional registry names are `.char`, `.frame`, `.HSVfilter`, `.overlay`, `.playSE3`, `.screen`, `.scrollXF` and `.select`. These totals count names once and exclude parser directives and subcommands. The ef retail corpus report includes five of those eight, namely frame, HSVfilter, overlay, screen and select. Neither registry establishes the entire retail native dispatch surface. [AetherMinori registry](https://github.com/AetherKiri/AetherMinori/blob/f99edd7e72240c73ab5fd3cdb6a0f366fbd7c0eb/src/command.rs) · [ReMinori parser](https://github.com/luoyily/ReMinori/blob/79b00990324404bfbd496e7fa76c690cf040f744/crates/formats/src/script.rs) · [ef corpus](https://github.com/AetherKiri/AetherMinori/blob/f99edd7e72240c73ab5fd3cdb6a0f366fbd7c0eb/research/ef-first-tale/FORMAT.md)

| Command | Evidence for the adopted scenario core | Behavior in the eden* reference |
| --- | --- | --- |
| `.if` | A true comparison transfers control to the label. | Same branch direction. Missing-target handling differs between the references. |
| `.chain` | The original NEXT consumer preserves locals and resolves the optional entry label after replacing the script. | The reference clears locals on success and starts at the beginning of the target script. |
| `.end` | The adopted interpreter closes the message and can execute following instructions. EOF ends a frame. | The reference calls the host end callback and stops the VM. |
| `.wait` | Delay is expressed in 10 ms units. | The reference waits for a count of VM frames. Real-time equivalence requires the original clock. |

The comparison distinguishes original-program findings for ef from observed behavior in the two Rust sources. The public material does not establish an independent numbered syscall table for either retail executable. Rust host methods, graphics requests and command handler addresses cannot supply a native syscall count. The address inventory in [command.rs](src/command.rs) contains historical engine-family coordinates. [Original conditional evidence](https://github.com/AetherKiri/AetherMinori/blob/f99edd7e72240c73ab5fd3cdb6a0f366fbd7c0eb/research/ef-first-tale/CONTROL_FLOW.md) · [Original NEXT consumer](https://github.com/AetherKiri/AetherMinori/blob/f99edd7e72240c73ab5fd3cdb6a0f366fbd7c0eb/research/ef-first-tale/CHAIN_NEXT.md) · [ReMinori flow handlers](https://github.com/luoyily/ReMinori/blob/79b00990324404bfbd496e7fa76c690cf040f744/crates/vm/src/exec/handlers/flow.rs) · [ReMinori timing](https://github.com/luoyily/ReMinori/blob/79b00990324404bfbd496e7fa76c690cf040f744/crates/vm/src/exec/handlers/system.rs)

## Host and resource contracts

The 51- and 52-name readers share one parser between `.effect` and `.effect2`
(`450960` and `4503C0`, respectively). It stores two strings (empty when
omitted) and three signed decimal prefixes (default −1). The command spelling
selects slot 2. Executors `450BC0` and `450620` pass all five fields and the
slot to the scene. In the 51-name scene at `460C50`, the live pointers are
at +0x2098 and +0x20a0, each followed by its own end flag. The last-request
tuples begin at +0x240 and +0x284 in the scenario record. A fade calls the
selected object's virtual method at +0x10. SnowH (`42F0C0`) starts the fade
once, zeroes its elapsed counter and preserves its current opacity.
The 52-name executor calls `460D20`. Both late scene handlers record the last
request separately for each slot, including control requests. A repeat of the
same five operands can reuse a live object. Changing even the third integer
recreates SnowH. `screenEffect` gates creation, with the native WScrollST
exception. Snow/SnowH read `effectDrawInterval` in 10 ms units (default 10).

In the three retail executables, the foreground window's Ctrl key-down and
key-up handlers set and clear the same byte that `.debug` clears. The bytes
are `4D45E8`, `559E8D` and `575510` in the 43-, 51- and 52-name registries.
No direct reader of these bytes was found. The scenario's physical-key skip
predicate is separate.

Ordinary camera motion uses 10 ms steps. Frame stills remain in screen coordinates while the background and its stage extras scroll. A message reveals its first character immediately and advances at 100 ms intervals. An advance input during reveal displays the rest of the message. A later input resumes script execution.

A client-scoped worker reads the entry scenario and global variables, then reads further scenarios on demand. The host advances native timer waits, submits choice input, prepares audio and presents media. Global variables use the native Shift-JIS, zlib and Blowfish-LE `system.dat` envelope in the client's save root. Manual slots and framework snapshots are tracked separately.

In the compact 43-command reader, the four encrypted member properties follow the bank-name pointers by 0x30 bytes. In the 51-command reader the index and data tables precede the names by 0x300 and 0x180 bytes respectively. Both readers start member RC4 at byte zero. The two long-table literal/drop readers share the same index-table distance (0x460). Their data tables have different addresses. CrySirror reads the paired table arguments from the installed i386 initializer. In the five-period reference these arguments occur at 0x4895a1 and 0x4895c7; the data table precedes the names by 0x1e0 bytes. The previously supported layout uses 0x200. No external parameter file is required.

The 51-command stage executor at 0x457f20 forwards the background operand independently of the foreground mask. For a wildcard background, 0x435e00 allocates the canvas-sized plane and 0x416900 clears the pixels. The adapter publishes a transparent plane in the original world-layer position so foreground masks, filtered stage images and subsequent camera operations keep their native indices.

The `minori/paz` resource plugin resolves the bank-name pointer table in initialized PE data, verifies the key layout against the encrypted index, and derives the index and payload keys for the bank slot. The table can move with the image base. The 10-slot, two-table reader uses a six-position byte transform with Blowfish-LE. Its movie reader inverts 256 permutations and selects a row by the member-relative 64 KiB block number. The four-table variant decodes four nine-byte member parameters with RC4 during initialization. The 12-slot initializer assigns four string addresses and member payload decoding drops `(crc32(key) >> 12) & 255` bytes of RC4 output. Later movie readers repeat the 64 KiB mask. The resource reader validates member extents against the joined source and enforces the allocation budget when opening buffered entries. Movie data remains seekable.

The registration evidence and archive decode were recovered with static analysis. The normal player results are recorded separately. [Retail validation](../../docs/validation/minori-2026-11-21.md)

## Retail tile transition profiles

| Installation | Draw dispatcher | Mode 2 | Modes 3–13 | Stage policy |
| --- | --- | --- | --- | --- |
| 天使の日曜日 (43 commands) | `4287EC` | `428046` | `428111` | Fixed 20 × 15 cells on an 800 × 600 canvas. Stage state 15 at `42BAA5` advances the normal draw at speeds above 255. |
| 夏空のペルセウス (51) | `43D600` | `43C880` | `43C9C0` | Round both dimensions up to 40-pixel cells. State 14 at `45CB45` forces the final copy when the signed speed exceeds 255. |
| ソレヨリノ前奏詩 (52) | `43DF70` | `43D1D0` | `43D310` | Same canvas grid. State 14 at `45C3AC` passes zero on the normal draw even above 255. |

The detector follows the dispatcher call into the tile worker. For canvas
grids it also checks the two calls from the stage to the same dispatcher and
the argument of the normal draw. It records the resulting policy in the
optional project setting `tile_transition_grid`. A missing or unrecognized
worker leaves the corresponding stage modes unsupported.

The tile counter divides by the number of cells. Each crossed step copies a
square that grows by four pixels from the cell's top-left corner. The previous
signed quotient determines whether a copy occurs on a delayed draw. Modes
3–11 use simultaneous, ordered, diagonal or radial offsets. Mode 12 draws
one CRT random offset per tile, and mode 13 shuffles column starts before
adding the row offsets. The clock samples ten-millisecond ticks, applies the
script speed and gates draws on `transitionDrawInterval` (default 5). A
forced draw copies the whole incoming stage. With `screenEffect` disabled
the three stage constructors bypass the draw worker altogether.

The adapter publishes the tile mask through a shared hard byte-rule transition.
For a full-canvas incoming SQZ background, the sequence worker retains the
latest decoded frame. Each newly admitted rectangular blit copies that entire
square into retained storage, including previously revealed pixels inside the
square. Completed cells retain their last copy when later frames arrive.
Completion restores the current full sequence frame. Other moving or replaced
composite sources still require the native paired-stage capture.

Mode 2 accepts two full-canvas opaque PNG backgrounds at the origin without
additional visible layers. Its first 200 phases sample the outgoing background.
Phases 200–399 sample the incoming background using reversed table indices.
The centered crop uses the native endpoint-aligned integer sampler. Each
admitted draw averages each stored channel with its previous output, rounding
down. Phase 400 or advance publishes the incoming image directly. Composed
stages and animated zoom sources remain unsupported. [Focused checks and remaining coverage](../../docs/validation/minori-commands-2026-11-22.md#effect2-debug-and-stage-transitions)

## Scene clocks and retained layers

The early, middle and late i386 constructors dispatch shake through 0x43a770, 0x42a413 and 0x466bf0. All three select the mode from the first ASCII byte, case-insensitively. The early dispatcher stores a raw unsigned countdown. The later two use the absolute duration and reserve the nonpositive default mode for asynchronous background displacement. The decimal duration is in 10 ms units. The early screen loop reads the default two-tick scroll draw interval. The middle loop reads the three-tick shake interval for H/V/R/Z/O and the two-tick scroll interval for T. The late zoom loop retains the zoomed image on successive redraws. In the middle random-screen branch the two axis helpers read the same snapshot and each writes the entire destination including its black strip. CrySirror applies these screen poses through shared layer roots. Native settings overrides, skip policy and rendering parity still require validation.

In the middle registry 0x42a4f9 selects a system-bank transition rule and 0x41943c complements its bytes. 0x424a12 gates drawing by the transition draw interval and computes integer phase from the script speed. 0x423ebf blends the stored RGB bytes with a weight of `clamp(rule + phase - 256, 0, 255)` and completes at phase 512. 0x4241aa copies the newly crossed thresholds and completes on the update after the previous phase passes 255. The default draw interval is five 10 ms ticks. The presenter uses the shared byte-rule transition for types 1 and 18.

In the early registry, 0x47faa0 and 0x47f670 initialise 50 snow particles. The second constructor consumes the base constructor's RNG draws before replacing them with the horizontal records. 0x47f800 advances the 16.16 positions. 0x47fc50 uses the effect draw clock and 0x4220e0 selects the unsigned maximum packed ARGB texel. CrySirror processes the snow plane on a worker and publishes it in a client-owned dynamic surface. The `end` operand sets the effect retirement flag, which the original stage transition consumes at 0x43bda0. The row filters use separate slots for the fixed foreground, background and standing art. The original applies Overlay, then Screen, in asset-local coordinates. The early 0x450e30, middle 0x4370bb and late 0x41b6c0 SV kernels build signed fixed-point channel tables from saturation and value.

The late reader at 0x479c10 invokes the 0x4751e0 macro expander while reading each source line. Its definition has current and next character IDs, a step of ten, an exclusive upper bound, and independent active/pending bits. `@load` emits a negative-ID `.char loada` followed by a position command. Its fifth operand selects a new ID range. `@stage` and `@update` emit keeps, promote pending IDs and reuse the saved stage text. `@finish` and `@finish_all` erase definitions. 0x475d40 resolves `@@name` to the negative next ID and marks it pending. These expansions precede label binding in the extended reader. In the late character handler 0x4551c0 the movie branch looks up the chosen character and its bitmap dimensions. The presenter binds the decoder surface to that character's layer.

### Stage-bound effects and native title evidence

The 38-command registry dispatches Cutin through 0x43a9d0. Its constructor, fade request and draw tick are 0x480af0, 0x480f50 and 0x480f70. The picture resolves in the background bank. The optional arrival sound has the same stem. WScroll2 takes its two source planes and fixed foreground from the current stage. 0x47db90 checks the pause variable, accumulates signed pixel displacement, carries the far-plane remainder after crossing five pixels and uses the absolute sine of the traveled distance for the foreground offset. 0x47da70 wraps the source rectangle. Types 14 through 17 at 0x432e10 through 0x432f80 resize the incoming raster from the top-left. Type 15 also shrinks the outgoing raster into the lower right and clears the other two rectangles. The 45-command registry uses matching rectangles at 0x4247c1 through 0x4248de, with floor-mapped source pixels at 0x4377ee.

In the 52-command registry, WScrollST at 0x436330 resolves far and near plane assets with fixed-name alternatives, then searches for the screen, overlay, multiply and add suffixes in order. 0x4354b0 positions three copies of each plane with separate signed displacements. 0x435b90 looks up standing layers by prefix and reads the foreground animation for the walking curve. 0x410f20 loads a character's primary image and allows an absent secondary image. The available command sets the live-character flag at 0x4551c0.

The middle Firefly constructor at 0x458bbc selects the three system-bank sizes and allocates the requested population. 0x458ab5 draws seven control points per particle, then draws its size and lifetime. 0x45935a computes the degree-six Bernstein basis. Coverage rises before progress 0.255 and falls after 0.755. 0x419236 compares complete ARGB words after scaling coverage with a divisor of 255.

The original title scene constructors are 0x40ab30, 0x408a8c and 0x4886d0 in registry order. All three retail manuals describe New Game, Load Game, System, Memories and Exit. The later constructor chooses the background family from the local hour and progress flags. The original slot path constructors are 0x415fb0, 0x4103cf and 0x4a3620. Filename evidence alone does not validate slot interchange.

The three additional i386 registrations, their native parser and executor addresses, and the observed nested command selectors are recorded alongside the installed-game corpus results. [Three retail registries](../../docs/validation/minori-commands-2026-11-22.md)

## Sound, ANI and Breath in the 43-, 51- and 52-command registries

The unit of script time is 10 ms. The early BGM parser at `462989` initializes omitted fades to one tick. An explicitly supplied non-numeric fade uses two ticks. The SE parser at `462602` uses two for both. The two later registries use 200-tick defaults. Early `421F83` accepts one update when the interval expires and resets its timestamp. `447D3F` then advances one integer percentage point toward its target. A late update at `4781E0` interpolates between the captured start and target percentages over the specified duration, truncating the signed difference toward zero. BGM can change gain and pan on a playing loop. Replacing a logical slot can leave the previous transport fading concurrently. BGM uses the separate volume operand and SE uses the filename qualifier. Both honor the native master gain and mute properties.

The actor table is an array of short filename prefixes in the executable's initialized data. The voice-setting getter at `488AB0` in the 51-command executable resolves unknown actors to `etc`. The 52-command voice command at `45D6F0` releases the previous slot, checks the actor flag and global mute, and reduces the scaled volume by five percentage points. The actor checkboxes use the same persisted properties. Example redraw functions are `4119EA`, `491C00` and `4A05B0` in registry order. These are distinct from the code that tests clicks. For example, the rectangle tested at `492440` extends across the actor label and begins three pixels below the checkmark. Detection reads the pointer table and the draw coordinates without executing the installed program.

The executable also holds six terminated arrays of (pose, duration) pairs. Native pointer tables are at `4BCD30`, `527220` and `541E84`. Program 5 uses three ticks per pose in the early table and thirty in each later table. The eye object reads `EYE_SEQ_DEFAULT` on creation. A numeric suffix on a recognized eye patch can select another program. Programs 0–4 add 50–549 ticks on entry to the first pose. Program 5 cycles without that jitter. The 51-command reader at `40C950` also looks up an optional map before rejecting an out-of-range selector. References to its map in the examined executable lead to construction, lookup and destruction. No registration of an additional program was identified. The adjacent table at `4F9D60` belongs to the mouth pose selector at `40CE60`.

In the 52-command binary, `44D510` removes a leading speaker control character, converts aligned CP932 ideographic spaces to spaces and removes spaces for actor lookup. The display name retains its formatting. `43B0D0` dispatches an active voice callback when its serial changes. `43AD90` accepts the wildcard speaker, strips the ANI actor prefix and angle-bracket suffix, and matches the part before the speaker alias separator. The DirectSound notification buffer at `4764A0` uses 8,192-byte blocks. `476C10` publishes the mean absolute amplitude of the 4,096 interleaved signed PCM16 samples and increments the serial. `40CF40` applies the 800 threshold and prior-pose hysteresis. The adapter measures complete windows of decoded source PCM before gain or pan, and selects the window using the playing source's frame cursor. A replacement starts a new callback generation.

The 52-command `Breath` constructor at `429D20` loads the alpha mask from the system bank. Its fallback is a solid 256-square bitmap. Only the primary registered effect receives the mouth position in `465D80`. The effect emits at that pose's center on a fresh active speech callback. Its deque at `42A560` retains the newest 32 puffs. Each starts at a random intensity of 40–71 and width of 160–223. The frame-clock worker at `42A020` grows the mask by half the puff's age, subtracts a fifth of its age from its intensity and accumulates white coverage with an alpha ceiling of 127. Slot replacement, retirement and scene teardown release the published plane. The shared image operation uses a clipped destination, nearest source coordinates and integer alpha accumulation. The shared PCM and image helpers are additive Rust functions with no reflective or serialized command changes.

## Stage image qualifiers and mode-one dialogue

In the Genesis executable, `427D90` separates the eye or mouth selector,
optional actor and pose number. The companion ANI reader at `43CC70` uses
separate eye and mouth tables and composites the chosen patch at its signed
offset. The FDB00 background operand combines sepia with a closed mouth
or closed eyes. Its filename is the remainder after all leading qualifiers.
The character image suffix dispatcher at `41CEE0` recognizes screen,
overlay, multiply and add. The opening film-grain character uses multiply.

For mode one, `453E00` in the Genesis image binds 26-pixel text for two
rectangles. Their horizontal extents are 65–221 and 169–1235, with a shared
top at panel bitmap height minus 208. The corresponding すぴぱら method at
`4448F0` uses 120–1160 for both and subtracts 232. Both set the second
text metric to 12. The panel origin is canvas height minus bitmap height
plus 64 in `4547C0` and `445150`, respectively. The detector recognizes
the executable rectangle assignments. Other panel modes retain their
existing presentation.
