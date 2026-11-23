# minori implementation debt

## SATD-minori-ef-presentation

**Location** [VM](src/vm.rs), [host adapter](src/lib.rs) and [presentation](src/presentation.rs).

The scenario core parses native control flow, selects language-tagged lines and emits typed requests. The host presents static stages, canvas-sized frame stills, dialogue, choices, panels, audio and movies. Character motion, compound stage filters, effects, masked/tiled transition families, text subcommands and the retail title/system UI require additional shared-capability bindings. Unmapped event types terminate with the native request and script name. Establish the retail ABI and exercise each command variant before marking it supported.

**2026-11-21** Added the ef AA parser, native control flow and first host mappings. Added staged crossfades, axis/diagonal/timed camera motion and a 100 ms message reveal clock with separate reveal and advance inputs. Added an authored normal-entry regression for include, persistent global state, chain-to-label and input. It awaits execution. Root workspace dependency resolution succeeded. The standalone Vita manifest could not resolve its pre-existing clipboard dependency from the offline registry. Its lock file includes the new workspace dependency. Host builds and hardware behavior are unverified. Closure requires original-program comparison, title and settings input, a muted headless route with choices, waits, media, return to title and a reproducible run record.

## SATD-minori-native-saves

**Location** [globals](src/globals.rs) and [adapter](src/lib.rs).

**2026-11-21** Added scoped ef system-variable loading and atomic publication on chain, exit and normal completion. Verify bidirectional global-file interchange, recover and implement original slot layout and transactional scene restoration, then verify the first restored frame, input, music and next script branch.

## SATD-minori-eden-runtime

**Location** [profiles](../crysirror-formats-arc/src/minori/profiles.rs), [factory](src/lib.rs).

**2026-11-21** Added the eden* retail PAZ profile from format evidence. Its title flow, 1024 × 640 scenario ABI, system data and slots need native verification before choosing shared handlers or introducing a semantic variant. Closure requires the retail title, settings and a muted headless route through save and restore.

## SATD-minori-paz-verification

**Location** [shared PAZ reader](../crysirror-formats-arc/src/minori.rs) and [detection](../crysirror-launch/src/detect/minori.rs).

**2026-11-21** Added index parsing, executable-derived parameters, data transforms, split readers and seekable movie access. Verify against both installed profiles across script, image, audio and movie banks, including a split boundary and independent known plaintext. An authored regression covers movie reads across both a split volume and the mask wrap. It awaits execution. Record build, host and real-game results separately.


**2026-11-21 · Three installed games.** CrySirror's desktop player stopped before script dispatch in all three. Two script banks had no recognized index parameters. In the third, both Blowfish parameters matched and an independent reader verified all 25 entry extents, but the member password fingerprint was absent. The current crate passed 43 unit tests and its authored include/chain/input regression. Resolve and verify archive parameter discovery before using these samples to draw conclusions about VM compatibility. [Playback evidence](../../docs/validation/minori-2026-11-21.md)

**2026-11-21 · SATD-minori-ef-presentation / SATD-minori-eden-runtime / SATD-minori-paz-verification.** Replaced title-based scenario selection with PE command and NEXT references plus a decoded startup scenario. PAZ banks now select their own key catalogs independently. Documented the 24 common registry names, eight additional names and divergent control-flow semantics. Recover the remaining native registration and handler contracts in both executables before treating new scenario profiles as compatible; retail launch and archive verification remain open.

**2026-11-21 · Retail archive and VM progress.** The normal player now mounts all three installations and executes their original entry chains. The early movie implementation selects inverse permutation rows by the member-relative offset. The resource profiles resolve native bank pointers and member parameters. The constructor inventory contains 38, 45 and 52 registered dot commands. Messages now preserve empty tab-separated operands and skip semicolon comment lines. The presentation adapter implements paired upper/lower panel strips and the single upper strip. Validation reached a timed snow effect in the early script, the character loader in the late script and the opening movie in the middle registry. Outstanding work includes effect/character composition, additional original registered commands, native UI and slot persistence. Route and archive acceptance remain open. [Playback evidence](../../docs/validation/minori-2026-11-21.md)

**2026-11-21 · SATD-minori-eden-runtime, movie playback.** The original script movie number now stays separate from the scoped host media player ID. In the longer replay the stream reached the media worker. Independent probing identified the opening AVI as WMV3 (768 × 478, 24 fps) with stereo 44.1 kHz PCM. Added bounded RIFF packet reads over the existing source capability and dispatch to CrySirror's WMV3 decoder and PCM mixer in the shared video and Media layers. Verify decoded frames, timing, audio and scene continuation in the same replay before closing this part. [AVI packet reader](../crysirror-video/src/avi/stream.rs)

**2026-11-21 · SATD-minori-eden-runtime.** A normal-player replay now verifies movie EOF, resource teardown and continuation into `B004.sc`. The paced run reaches `B005.sc` and exposes the unbound native shake request. The 2006 snow dispatcher selects a 50-record fixed-point field, three sprite sizes and a packed ARGB maximum operation; the early Snow and SnowH constructors differ. Character layer 600 in the 2016 prologue is a separate open path. Title UI and whole-slot restoration still require execution evidence. Source save research must distinguish the original edition signature from port-private slot formats.

**2026-11-21 · SATD-minori-ef-presentation / SATD-minori-eden-runtime.** Added native screen/background shake clocks, a worker-owned Snow/SnowH plane, per-asset Overlay/Screen and SV preparation, byte-rule stage transitions, retained character layers, and late compile-time character control expansion. Native coordinates are in NATIVE_ABI.md. Actual input-driven runs reached `100_01.sc`, `C007.sc` and `K02.sc`. Preserve the debt for native configuration and skip timing, character ANI and voice-linked overlays, exact movie/character and freeze lifetimes, remaining WScrollST and other effect/transition families, title/system UI and validated ending routes. Original whole-slot restoration remains under SATD-minori-native-saves. [Playback evidence](../../docs/validation/minori-2026-11-21.md)

**2026-11-21 · SATD-minori-ef-presentation and SATD-minori-eden-runtime.** Cutin now loads the background-bank art and ticks its entrance, hold, exit and sound. WScroll2 uses the prepared stage rasters. WScrollST resolves separate scrolling planes, foreground selectors and walking samples. The late character loader now accepts a missing auxiliary image and supports promotion through available. The middle Firefly field shares the packed-stamp worker with snow and uses per-particle coverage. Early and middle stage stretch mappings use the shared integer-clock renderer. Preserve the records pending original-frame comparisons, all command variants, title/settings behavior, movie and route completion and full native slot restore. The original constructors and observed route boundaries are documented separately.

**2026-11-22 · SATD-minori-ef-presentation and SATD-minori-eden-runtime.** Added the retail title lifecycle, hit rectangles, progress-dependent artwork, late local-hour lookup and System/back/New Game input. Nine original-data checkpoints cover these interactions. Native settings controls, save/load and memories actions, title modal policy, stage cleanup on return and end-to-end retail routes still need implementation or validation. Added the standing image's per-plane HSV and overlay override after the early 107_02.sc replay. [Retail title checkpoints](../../docs/validation/minori-2026-11-21.md)

**2026-11-22 · Startup across four additional installations.** Extended executable-derived bank discovery, the retail title layouts and scenario entry to the 33-, 42-, 47- and second 52-command samples. Three reached a dialogue wait, including natural movie EOF and one Genesis route selection. Wind reached the first mode-10 panel in `A001.mis`. Its corrected mask placement still needs a normal-player replay. Remaining startup-adjacent checks include the other five Genesis route buttons, cleared-state artwork and non-start title actions. Full routes and native slots remain open. [Startup checkpoints](../../docs/validation/minori-2026-11-21.md)

**2026-11-22 · SATD-minori-ef-presentation and SATD-minori-eden-runtime, scenario clock.** The retail registry inventory now covers all seven executables. Added the common setTimer and waitTimer dispatch paths with a 32-bit, 10 ms clock shared across story and title transitions. The authored clock and scenario regressions await execution. Keyboard skip uses the scenario flags and the cls property on control-lock profiles. The native latched menu skip, joystick override and debug fast-forward paths still require the broader input and settings work. Sixteen top-level names remain unbound. Effect selectors and other secondary dispatchers require a separate inventory. [Command counts and native timer evidence](NATIVE_ABI.md#scenario-timer)

**2026-11-22 · SATD-minori-ef-presentation and SATD-minori-eden-runtime.** Added nine top-level dispatch paths for scene exec and debug, standalone cutins, waveform, foreground and accelerated camera motion, secondary music, fourth sound effects and BGM-clock waits. Static comparison against seven native registries gives 45 of 52 union names. Rotation, animation/wait, foreground replacement, animated SQZ images and the second effect slot remain unbound. The 33- and 38-name registries now have a path for every top-level name. Headless playback, frame and audio comparisons, and the new regressions still await execution. [Per-registry counts and native handlers](NATIVE_ABI.md#dot-command-inventory-on-2026-11-22)

**2026-11-22 · SATD-minori-paz-verification, SATD-minori-ef-presentation and SATD-minori-eden-runtime.** Recovered three additional installed PAZ reader layouts and title paths. The episode menu reads its nine scripts from the native click handler. The 51-command stage now accepts a wildcard background beneath a foreground mask. New Game, the settings round trip and opening dialogue have retail playback evidence for all three. The wider records remain open for whole routes, settings, additional native UI and bank coverage. [Playback evidence](../../docs/validation/minori-2026-11-21.md)

**2026-11-22 · SATD-minori-ef-presentation.** Recovered parser and executor addresses for the 43-, 51- and 52-command installations. The three original scenario banks now parse and expand. Bound late stage walking, early crossfade, bitmap tone selectors, timed inline character changes, v1 ANI eye and mouth patches, qualified voice operands, and dialogue voice waits, timed waits and automatic continuation. Inline deadlines now start when the text cursor reaches the escape. The retail debug handler clears a program byte whose consumer remains unidentified. Added output regressions for displaced tiles, effect teardown, layer opacity, ANI bitmap coordinates, tone-table bytes and mixed PCM. Preserve the record pending native pixel and clock comparisons, speaker overrides and audio fades, custom eye sequences, remaining registered effects including Breath and end-to-end routes. [Registration, corpus and behavior evidence](../../docs/validation/minori-commands-2026-11-22.md)

**2026-11-22 · SATD-minori-ef-presentation and SATD-minori-eden-runtime, audio and ANI.** Added separate native percentage-step and duration fades for BGM and SE, independent outgoing transports, actor-specific voice gating and persistent native checkboxes. The detector reads the actor keys, checkbox draw positions and six eye programs from the PE. ANI layers now select poses from the script default or the resource override. Mouth poses consume completed 4,096-sample source windows with the native threshold and prior-pose rule. Breath receives the primary slot's speaking mouth callback and composites the native alpha mask on its own surface. The three installations contain no scripted Breath invocation. Original display/device latency, effect combinations, full routes and slot interchange remain open under the existing records. [Native contracts](NATIVE_ABI.md#sound-ani-and-breath-in-the-43--51--and-52-command-registries) · [Regression evidence](../../docs/validation/minori-commands-2026-11-22.md#follow-up-on-five-presentation-behaviors)

**Validation of the five presentation changes.** The final isolated release passed 63 unit tests, 19 scenario tests and two retail probes. One probe combined the original 真響 voice, mouth patches and Breath mask. The linked desktop player persisted the selected voice checkbox and reached dialogue in each requested installation. Seven older games reached dialogue input in their original story scripts, including a longer 14,000-frame ef first tale replay. The frame caps and individual states are in the [final linked-player matrix](../../docs/validation/minori-commands-2026-11-22.md#final-linked-player). The broader presentation, route and save records remain open.

**2026-11-22 · SATD-minori-ef-presentation, effect slots and transitions.**
Compared the four original secondary-slot calls in 夏空のペルセウス and the
two late executable handlers. The adapter now retains each slot's last
command tuple and observes the native decimal parsing and screen-effect
setting. The three debug executors clear the foreground Ctrl latch.
Static tile transitions 3–13 have version-qualified workers, per-tile clock
and masks. The ABI notes record the distinct high-speed policy in the two
canvas-grid stage loops. Mode 2 still needs the native integer zoom and
previous-output averaging. Animated and replaced source surfaces require
per-draw capture. No original-window pixel comparison, device-dependent
cadence measurement or full-route validation was added in this change.
Retain the record for those paths and the previous open presentation scope.
[Source and behavior checks](../../docs/validation/minori-commands-2026-11-22.md#effect2-debug-and-stage-transitions)

**2026-11-22 · SATD-minori-ef-presentation, retained transition pixels.**
Mode 2 implements the repeating zoom and byte average for two full-size opaque
PNG backgrounds at the origin. Modes 3–13 retain per-draw source pixels for
a full-size incoming SQZ background. Multiple moving layers, retiring effects,
composited inputs and original-window comparisons remain open. [Behavior tests](../../docs/validation/minori-commands-2026-11-22.md#retained-transition-pixels)

**2026-11-23 · SATD-minori-ef-presentation.** The main stage image now
separates stacked fixed eye/mouth qualifiers from the filename and applies
the selected ANI patches. The two verified mode-one layouts use the panel
anchor and text columns from their native draw methods. Other panel modes,
other executable layouts and original-font pixel comparisons remain open.
The fixed-pose comparison covers the stage background. Qualified standing
character paths and combinations with moving layers need separate evidence.
[Native layout and ANI references](NATIVE_ABI.md#stage-image-qualifiers-and-mode-one-dialogue)
