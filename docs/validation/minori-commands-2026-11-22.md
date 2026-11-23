# Three retail minori scenario interpreters

The i386 command constructors register 43 names in 天使の日曜日, 51 in 夏空のペルセウス and 52 in ソレヨリノ前奏詩. The executable registry selects the ABI. Each command is a native object with a parser and an executor. The scenario analysis found named commands, macros and nested handler dispatch. It did not establish a separate numbered syscall table.

## Registration and corpus coverage

Each cell gives the native parser VA, executor VA and the number of command sites in the Japanese program after parsing and control expansion. Addresses are hexadecimal at the original executable image base. IDA’s `nullsub` symbols identify empty parser functions. An empty cell means the name was absent from that registry. Zero sites means the original registers the command but none of the decoded scripts emitted it. Label counts are source definitions bound during parsing. A site can be unreachable in an actual route.

The census used the command list from normal installed-game detection and the mounted script bank of each installation. All 12, 89 and 102 scenario files respectively decoded and parsed. Both later registries also ran the production control expander before counting. For example, its generated operations raise the last game's character count from 1,261 source statements to 8,152. Counting labels, 26, 29 and 29 distinct registered names respectively occur in these programs, with 33 distinct names across all three. The final column identifies specific evidence from this run. A dash leaves execution of that row unassessed here.

| Command | 天使の日曜日 | 夏空のペルセウス | ソレヨリノ前奏詩 | Behavior checked |
| --- | --- | --- | --- | --- |
| `.animation` |  | 44E320 / 44E6C0 · 0 | 44DD10 / 44E0B0 · 0 | — |
| `.chain` | 4632F4 / 461994 · 11 | 44ECA0 / 44ECE0 · 55 | 4519D0 / 44E710 · 77 | Opening script handoffs |
| `.char` |  | 44F100 / 44F980 · 290 | 44EB40 / 44F360 · 8152 | Delayed replacement, layer identity, coordinates and opacity |
| `.cutin` | 4641E0 / 46436D · 0 | 450160 / 450380 · 0 | 44FBB0 / 44FDD0 · 0 | — |
| `.debug` | 4632F4 / 461752 · 0 | 44ECA0 / 4505B0 · 0 | 4519D0 / 450000 · 0 | Clears the foreground Ctrl latch. All identified direct references write it. |
| `.deleteVar` | 4632F4 / 463348 · 0 | 44ECA0 / 4507A0 · 0 | 4519D0 / 4501F0 · 0 | — |
| `.effect` | 465679 / 46579D · 49 | 450960 / 450BC0 · 47 | 4503C0 / 450620 · 134 | Wrapped positions, crossfade output and removal |
| `.effect2` |  | 450960 / 450BC0 · 4 | 4503C0 / 450620 · 0 | Native parameter tuple, independent slot, SnowH fade and primary Firefly lifetime |
| `.end` | nullsub_4 / 461846 · 16 | nullsub_2 / 450D50 · 85 | nullsub_2 / 450760 · 87 | — |
| `.endScroll` | 463B47 / 463B73 · 24 | 450DF0 / 450E50 · 14 | 450800 / 450860 · 22 | — |
| `.exec` | 461B95 / 461C1B · 0 | 451030 / 4510D0 · 0 | 450A30 / 450AD0 · 0 | — |
| `.frame` | 4632F4 / 46456A · 0 | 44ECA0 / 451280 · 0 | 4519D0 / 450C80 · 0 | — |
| `.goto` | 4632F4 / 46318B · 4 | 44ECA0 / 451400 · 10 | 4519D0 / 450E10 · 35 | — |
| `.hscroll` | 463D12 / 463D41 · 2 | 455360 / 455420 · 1 | 454CF0 / 454DB0 · 0 | — |
| `.HSVfilter` | 4651FF / 465281 · 0 | 451520 / 451740 · 0 | 450F20 / 451140 · 0 | — |
| `.if` | 462D98 / 462E63 · 44 | 4519A0 / 451A30 · 20 | 4513C0 / 451450 · 8 | — |
| `.label` | 4632F4 / 4012CF · 46 | 44ECA0 / 4514E0 · 34 | 4519D0 / 44E3F0 · 78 | Native executor returns true immediately in all three |
| `.message` | 465432 / 4654AA · 12905 | 452190 / 4522C0 · 18319 | 451BF0 / 451D20 · 28358 | Displayed dialogue, voice pan and gain, delayed load |
| `.movie` | 46681E / 4668D5 · 5 | 4524B0 / 4526F0 · 15 | 451F50 / 452190 · 6 | — |
| `.movie2` | 466571 / 466652 · 0 | 452940 / 452BF0 · 0 | 452410 / 4526C0 · 0 | — |
| `.overlay` | 465072 / 4650C4 · 52 | 452E70 / 452EF0 · 0 | 454B10 / 452950 · 0 | — |
| `.panel` | 466A80 / 466AFA · 399 | 453070 / 4531A0 · 502 | 452AD0 / 452C00 · 820 | Visible in opening story replays |
| `.playBGM` | 462989 / 462A47 · 320 | 453380 / 453550 · 350 | 452F40 / 452DC0 · 804 | — |
| `.playBGM2` |  |  | 452F40 / 453110 · 6 | — |
| `.playSE` | 462602 / 46282D · 154 | 453700 / 453890 · 287 | 4532B0 / 453450 · 840 | — |
| `.playSE2` | 462602 / 4626C3 · 27 | 453700 / 453A30 · 110 | 4532B0 / 4535D0 · 237 | — |
| `.playSE3` |  | 453700 / 453BC0 · 54 | 4532B0 / 453750 · 18 | — |
| `.playSE4` |  | 453700 / 453D70 · 0 | 4532B0 / 453900 · 2 | — |
| `.playVoice` | 462BC0 / 462BE1 · 13 | 453EF0 / 453F30 · 10 | 453A90 / 453AD0 · 1 | Qualified voice playback and replacement |
| `.pragma` | 461D91 / 461DCA · 61 | 454100 / 454150 · 85 | 453CA0 / 453CF0 · 135 | — |
| `.rotateBG` | 4646DE / 46485E · 0 | 454A50 / 454780 · 0 | 454330 / 454070 · 0 | — |
| `.rotateFG` | 464A6B / 464BEB · 0 | 454A50 / 454E40 · 0 | 454330 / 454720 · 0 | — |
| `.screen` | 465072 / 464EC0 · 66 | 452E70 / 455230 · 0 | 454B10 / 454B90 · 0 | — |
| `.scroll` | 463F85 / 463DEB · 8 | 455850 / 4554A0 · 1 | 454E30 / 454FA0 · 4 | — |
| `.scrollAcc` | 463F85 / 464072 · 0 | 455850 / 4559C0 · 0 | 454E30 / 455350 · 0 | — |
| `.scrollFG` | 463F85 / 463FC5 · 0 | 4556C0 / 4557D0 · 0 | 4551C0 / 4552D0 · 0 | — |
| `.scrollXF` |  | 455A80 / 455E50 · 5 | 455410 / 4557E0 · 10 | — |
| `.select` | 466D18 / 466E0D · 0 | 456120 / 456240 · 2 | 455AB0 / 455BD0 · 1 | — |
| `.set` | 46360C / 4638A5 · 3 | 456540 / 4567F0 · 62 | 455ED0 / 456180 · 22 | — |
| `.setFG` |  | 456940 / 456980 · 0 | 4562D0 / 456310 · 0 | — |
| `.setGlobal` | 46360C / 463453 · 19 | 456540 / 456B90 · 64 | 455ED0 / 456570 · 106 | — |
| `.setTimer` | 4622B7 / 4622EA · 0 | 456D60 / 456C30 · 0 | 4566D0 / 456610 · 0 | — |
| `.shakeScreen` | 4639F9 / 463A4C · 23 | 456F60 / 457050 · 17 | 4568D0 / 4569C0 · 28 | — |
| `.sqz` |  | 4571E0 / 457350 · 0 | 456B50 / 456CC0 · 0 | — |
| `.stage` | 465C67 / 465F1C · 5113 | 457B50 / 457F20 · 6538 | 4574D0 / 4578A0 · 9798 | Visible stage layers, standing eye patch and replacement image |
| `.transition` | 466374 / 4663CF · 5035 | 4584D0 / 4585D0 · 6556 | 457EA0 / 457FA0 · 9520 | Source operands 0 and 1. Separate authored static-stage checks cover modes 3–13. |
| `.vscroll` | 463D12 / 463C68 · 14 | 455360 / 4552E0 · 7 | 454CF0 / 454C70 · 8 | — |
| `.wait` | 462485 / 4624B1 · 130 | 4586F0 / 4587D0 · 151 | 458320 / 4580B0 · 181 | — |
| `.waitAnimation` | 462169 / 4621B1 · 0 | 4588E0 / 4589B0 · 0 | 458170 / 458240 · 0 | — |
| `.waitBGM` | 462485 / 462049 · 0 | 458AA0 / 458B20 · 0 | 458320 / 4583A0 · 0 | — |
| `.waitTimer` | 4622B7 / 462391 · 0 | 456D60 / 456DF0 · 0 | 4566D0 / 456760 · 0 | — |
| `.wscroll` | 463E98 / 463EDB · 0 | 455550 / 455640 · 0 | 455050 / 455140 · 0 | — |

## Operands seen in source

The selectors below are literal operands in the original sources. The character macro expander can additionally synthesize operations. Panel and transition entries in this table give only the first operand. The later fields include resource names, positions and transition parameters.

| Family | 天使の日曜日 | 夏空のペルセウス | ソレヨリノ前奏詩 |
| --- | --- | --- | --- |
| `.effect` | `*`, `CrossFade2`, `WScroll2` | `*`, `CrossFade2`, `Firefly`, `WScroll2`, `end`, `fadeout` | `CrossFade`, `WScroll2`, `end` |
| `.effect2` | — | `SnowH`, `fadeout` | — |
| `.char` | — | `keep`, `load`, `pos` | `available`, `blendrate`, `keep`, `load`, `moveofs`, `movie`, `order2`, `pos`, `visible` |
| `.shakeScreen` | `2`, `H`, `R` | `R`, `V` | `R`, `V`, `Z` |
| `.pragma` | `disable_control`, `enable_control` | `disable_control`, `enable_control` | `disable_control`, `enable_control` |
| `.panel` | `0`, `1`, `3` | `0`, `1` | `0`, `1`, `2` |
| `.transition` | `0`, `1` | `0`, `1` | `0`, `1` |

## Native behavior and verification

- **Reader and control flow.** The late reader discards unregistered dotted source lines before constructing a command. Two real lines in `B02.sc` and `B03.sc` follow that branch. The registered `.label` executors all return success immediately. Their parser and the label index still affect control flow.
- **Walking.** `WScroll2` takes its planes from the current stage. It carries signed far-plane displacement, wraps both planes, samples the foreground SQZ frame for an optional sync curve and responds to the pause and ratio variables. `WScroll3` has two foreground periods and takes signed speed from the third numeric operand. It has an authored test and no occurrences in these three corpora.
- **Fades and color.** The early `CrossFade2` draws two inputs and copies its final source at the endpoint. Known tone prefixes resolve through the same bank as the base image. The original lookup uses the integer luminance `(5R + 2G + B) >> 3` and three separate sepia multipliers. A regression checks exact RGBA values, including RGB under zero alpha. The final corpus has 91 sepia and eight mono selectors.
- **Characters.** The inline command dispatcher handles load, position, visibility and blend. Subcommands are enqueued as their escape is reached during text reveal. Their deadlines are relative to that read. The 26 load requests in the 51-command game include 25 with an ANI companion. Replacement has an independent layer, reuses the previous position and transfers ordering when the blend completes. ANI v1 signed offsets, BGRA patches, default eye sequence and voice-linked mouth poses use shared image and audio services. The authored test inspects the actual eye bitmap and screen position.
- **Audio and dialogue.** Qualified voice operands carry a resource, gain and pan through both message and direct voice requests. The audible-channel regression measures mixed stereo PCM and checks replacement by a missing voice. Background music retains its transport on same-name parameter changes. The original scenarios contain 34 voice waits paired with auto-advance in the 43-command corpus, one of each in the 51-command corpus and one auto-advance in the 52-command corpus. The late text reader at `4208E0`, `420911` and `420942` sets an auto-advance flag, voice wait with resume state 4, or timed wait with a ten-tick counter, respectively. Its scene loop consumes these at `45B4CF`, `45B5F1` and `45B620`. The voice wait field is populated by the voice-channel start at `45D891`. Regressions hold the voice gate while audio is active, check the ten-tick delay and verify automatic continuation by observing the next stage without another input.

## Release tests

The preceding isolated adapter build passed 63 unit tests and all ten scenario integration tests. An additional local probe ran the three retail effect sequences described below. The integration tests use generated assets and assert on shared mixer samples, scene-layer geometry, opacity, bitmap pixels and input-driven VM state. The private retail bank contents are outside the repository.

## Retail regression

All player runs used the release desktop binary with `play <installation> --headless --no-audio --data-dir <isolated> --input-track <track>`. Each of the three title tracks clicked System, pressed Escape and clicked New Game. At frame 4,600 all three runs had reached a dialogue input wait. The native script was `FD_01.sc` in 天使の日曜日 and `K01.sc` in each of the other two games. All reports had `window_created=false` and `audio_emitted=false`. Seven older installations also reached the title at frame 450. Additional New Game tracks reached `000_01mugai.sc` in ef - the first tale, `A001.sc` in eden* and `K00.sc` in 罪ノ光ランデヴー. The other four title checks covered ef - the latter tale, Wind -re:gratitude-, すぴぱら story #01 and Trinoline: Genesis.

The desktop matrix ran with the dialogue wait and auto-advance fixes. The subsequent change to start inline action deadlines at the text cursor passed the final adapter tests and three retail probes. That earlier desktop relink stopped at its compilation time limit. The follow-up matrix below identifies the subsequently linked player.

Separate short scenarios assembled from retail commands and original assets exercised walking and fading in all three profiles. They include material from `FD_02.sc`, `A02.sc` and `B15.sc`. Each probe observed both effect layers and returned to the title. The 51-command probe also observed an inline character replacement. These runs reuse the original command operands and resources, with a shortened scenario and scripted waits. The probe used a 1280 × 720 test canvas. The normal player exercised the 43-command game at its detected 800 × 600 canvas. Original-window pixel equivalence and the routes leading to the sampled scenes remain unverified.

## Follow-up on five presentation behaviors

| Behavior | Original evidence | Regression observation |
| --- | --- | --- |
| BGM and SE fades | `421F83` / `447D3F` step the early gain; `4781E0` interpolates late gain. | `bgm_and_se_follow_the_registry_fade_clock_and_release_the_tail` reads both physical mixer channels at the same elapsed time on the two ABIs, then waits for both to stop. A second test reads simultaneous old and new music transports during replacement and checks that changing the incoming loop’s gain preserves its playback position. |
| Per-character voice | `4119EA`, `491C00` and `4A05B0` draw the registered actors. The PE-derived tables contain 16, 5 and 7 keys in registry order, including the native fallback. | `installed_title_keeps_settings_across_two_trips_through_the_story` inspects the checkmark, clicks the actor label, cancels once, applies the second edit and observes the voice channel during dialogue. |
| Selected blink sequence | All six pose programs were read from each original executable. The same ID 5 advances after 3 ticks in the 43-command profile and after 30 in both later profiles. | `ani_pose_selector_overrides_global_blink_at_original_deadlines` and `detected_eye_program_uses_the_early_three_tick_period` assert four successive rendered patch colors. |
| Mouth and audio callback | `476C10` publishes a completed buffer's mean absolute level; `43B0D0` checks the serial; `40CF40` selects a visible pose. | `mouth_uses_completed_pcm_windows_and_updates_once_per_callback` compares five published mouth bitmaps with the mixer source position. A single loud sample in a quiet block keeps the mouth closed. Two rising blocks open it in stages. |
| Registered Breath effect | `429D20`, `42A020`, `42A560` and `465D80` construct, draw, enqueue and position the puffs. None of the three script banks requests Breath. | `breath_accumulates_white_mask_at_speaking_mouth_and_clears` samples white alpha at the mouth, checks that a distant pixel stays transparent, then tests layer removal. |

The follow-up release tests passed all 63 adapter unit tests, 19 scenario tests and two installation-dependent probes. Additional assertions cover stereo sample counting across short decoder reads, an incomplete final PCM block, master mute, fallback actor selection, a recognized actor's attenuated gain and the clipped alpha ceiling. The two probes are local because they require the installed assets.

A targeted probe issued Breath during a shortened scene using the original ソレヨリノ前奏詩 `myr-B15-0003` voice, 真響 ANI, standing art and 480 × 480 system mask. It used the original space-padded speaker operand. With voice channel 1 active, the frame contained the mouth patch centered at (847, 426) and nonzero Breath alpha in its neighborhood. The other probe exercised the previously documented retail walking, crossfade and replacement sequences across all three profiles.

### Reproducing the checks

Run `cargo test --release --locked -p crysirror-minori --lib --test scenario` for the redistributable fixtures. The normal player is built with `cargo build --release --locked -p crysirror-desktop --bin crysirror`. The runtime matrix uses this executable with an isolated writable data directory per installation, headless rendering, disabled audio-device output and frame-indexed pointer input. The API reference generator retained fingerprint `e849c4b9bdfabb09`. All three generated artifacts matched their workspace contents byte for byte. The installation-derived fields in the minori project config are documented in the adapter ABI notes.

The three System tracks select settings at frame 140, toggle an actor by clicking its label at frame 195, confirm with OK at frame 230 and select New Game at frame 340. Their native save properties provide an observable checkpoint independent of title rendering.

### Final linked player

The isolated macOS release player has SHA-256 `fb596413073c8c3d840773f9fa3f3986b925d6b5baf892a57905268d5d18408d`. All runs in this matrix used the same executable. The signature identifies this validation artifact only.

| Installation | Property after clicking the actor and OK | Original script at frame 4,600 |
| --- | --- | --- |
| 天使の日曜日 | `miy = 0` | `FD_01.sc`, dialogue input |
| 夏空のペルセウス | `ren = 0` | `K01.sc`, dialogue input |
| ソレヨリノ前奏詩 | `tow = 0` | `K01.sc`, dialogue input |

The three executable-derived eye program tables and checkmark positions were also inspected through `play --detect-only`. Each game supplied six complete eye programs. All three normal replays exited at the frame cap, reported no window or audio device output and left the expected tab-separated setting in their isolated native save directory.

The same player reached the title in seven older installations at frame 450 and entered each one's original story. The 6,200-frame checkpoints were as follows. All seven processes exited successfully at the frame cap with window and audio-device output disabled.

| Older installation | Original script | State |
| --- | --- | --- |
| ef - the first tale | `000_01mugai.sc` | Timer wait at 6,200. A separate 14,000-frame replay reached dialogue input in the same script. |
| ef - the latter tale | `pre001.sc` | Dialogue input |
| eden* | `A001.sc` | Dialogue input |
| すぴぱら STORY #01 | `A00_01.sc` | Dialogue input |
| Trinoline: Genesis | `FDA00.sc` | Dialogue input after route selection |
| 罪ノ光ランデヴー | `K00.sc` | Dialogue input |
| Wind -re:gratitude- | `A001.mis` | Dialogue input |

## Effect2, debug and stage transitions

The four literal `.effect2` calls in 夏空のペルセウス are `SnowH` at
`K01.sc:54` and `C02.sc:474`, and `fadeout` at `K01.sc:70` and
`C02.sc:500`. The first pair straddles a primary-slot clear at line 56 and
`Firefly Firefly_c 90 6000` at line 67. Each sequence changes stages using
mode 0 while the snow is active. ソレヨリノ前奏詩 registers the same command
class and emits no calls. 天使の日曜日 has no second effect command.

| Focused check | Observation |
| --- | --- |
| Shared effect parser and slots | Signed decimal prefixes, omitted operands, and extra arguments were tested on both command spellings. Repeating a live SnowH tuple kept the visible alpha. Changing only its last integer restarted it. The second slot published intermediate fading pixels before disappearing. The first slot's Firefly survived. Disabled `screenEffect` suppressed weather and completed stage changes directly. |
| Ctrl latch | The three retail executors write zero to the byte updated by foreground Ctrl key-down and key-up. The fixture pressed Ctrl, executed `.debug` with the key held, then released and pressed it again. The observed latch was true, false and true. None of the three original scenario banks emits this command. |
| Tile pixels and stage state | An 83 × 51 canvas exercised all eleven patterns, the clipped right/bottom cells, and a delayed draw crossing a signed counter boundary. The stage fixtures read the published mask, checked its release and the incoming layer, exercised both kinds of canvas-grid speed policy and the fixed grid, and verified the skip key at speed zero. An unsupported moving effect, an unknown grid, and a mode-2 draw with undersized backgrounds each raised an error before replacing the old stage. |

The isolated release build passed three adapter unit tests, eight scenario
tests and one detector unit test.
One scenario set both draw intervals to 300 ticks in the native save
properties. Snow alpha and a tile pixel stayed at their initial values at
2.9 seconds and changed after crossing three seconds. The shared API
reference was regenerated with signature fingerprint `e849c4b9bdfabb09`.

The release detector also ran directly on 15 PE files across ten installations.
It recognized seven engine images in five installations. The other eight
returned no tile profile. One of these is the outer 夏空のペルセウス launcher.
Its embedded executable at file offset `0x63000` exactly matches the
standalone engine image. The detector reads executable sections of the
selected image. The focused fixture can be reproduced with `MINORI_TILE_PE`
and `MINORI_TILE_GRID` and the ignored test
`retail_tile_grid_from_installed_program`.

The three original script banks contain only transition modes 0 and 1.
The mode 3–13 checks used authored bitmaps and read back the managed mask
and stage state. There was no game replay or native framebuffer comparison
in this follow-up. The 51-command mode-2 worker at `43C880` zooms the
outgoing image below phase 200, zooms the incoming image below phase 400,
and uses `41A930` to average packed bytes against prior output. The
stage rebuild at `4656E0` can refresh either source during a transition.
The following checks extend coverage to supported single-background inputs. Paired composite capture remains open.
The SnowH draw also checks the native layer-manager threshold at `41D360`.
Matching that threshold under load, the graphics-mode override of the draw
intervals, and matching random-generator positions in a live process remain
unverified. [Per-version addresses and contracts](../../crates/crysirror-minori/NATIVE_ABI.md#retail-tile-transition-profiles)

### Retained transition pixels

The mode-2 helpers in the three original images reference identical 200-double
tables. For every table entry and every canvas dimension from 1 to 8192, the
integer crop extent equaled the extent derived from `1 + 99*i*i/(199*199)`.
The 51-command sampler at `41A520` includes both crop endpoints. The packed
average at `41A930` truncates each byte independently. A release scenario
uses two gradients. At the screen edges it checks successive red-channel
pairs (20,120), (50,100), (127,152), and (165,178), followed by the incoming
asset at phase 400. This checks feedback across draws and both scene sources.

A separate release scenario uses a three-frame, 16 fps SQZ background. It
checks an unchanged red tile between draws even after a new frame decoded,
a previously exposed area recopied in blue on a later draw, and a completed
blue tile alongside a still-growing green tile. The fixture runs through the
normal VM, sequence decoder and client surface publication. Desktop GPU
framebuffer comparison and transitions involving multiple moving layers
remain open. The isolated release run passed all three focused adapter unit
tests and all ten focused scenario tests. [Transition profiles](../../crates/crysirror-minori/NATIVE_ABI.md#retail-tile-transition-profiles)

## Open verification

Full routes, every choice, original save interchange and original-window pixel comparisons remain open. The tests use the mixer's decoded-source clock. Physical DirectSound notification and display latency have no side-by-side measurement. No additional registered eye program beyond the executable tables was found in these three installations. Older-title route checks only establish the checkpoints reported here. The current checkbox locator also recognizes すぴぱら, Trinoline: Genesis and 罪ノ光ランデヴー. It does not recover draw positions in the two ef titles or eden*, and Wind yielded no actor table. [Adapter debt](../../crates/crysirror-minori/SATD.md)

## November 23 desktop playback

On Apple M4 Max the previous release reached the すぴぱら title for 720 frames.
Starting a new game and rendering every third frame reproduced a 256 MiB
prepared-asset failure on `petal2_fg_0035.png`. The last successful capture
was frame 603. With the same input and capture interval, the fixed player
reached frame 1080 with both foreground and background image 0126 ready.
The cache reported 265,420,800 resident bytes against 268,435,456 allowed.
A separate run reached the cherry-tree narration in `A00_01.sc`.

In Trinoline: Genesis, selecting the leftmost portrait opens `FDB00.sc`.
Advancing past the first line used to fail in the next stage command at
replay frame 1000. The stage operand combines sepia and a fixed mouth pose.
Replaying through the chapter also exercises sepia with a fixed eye pose.

The two installed help files show the exact opening lines used for the text
comparison. Genesis puts the speaker and body on the same row. In a
1280×720 capture the previous speaker ink started at y=558. The corrected
name and body start near y=590 and y=589 respectively. The source is a
scaled help screenshot with window decorations. It supports placement
comparison rather than pixel identity. The すぴぱら narration uses its own
120-pixel left inset. Runs used separate save directories, offscreen Metal
and muted audio.

The final desktop release rendered the Genesis opening beach, both figures,
film grain, speaker and dialogue at frame 1900. A second route replay passed
the affected stage changes and reached dialogue in `FDB01.sc` at frame 8200.
The すぴぱら opening was checked at frame 1080 with a capture every three frames
and at frame 6200 for the first narration. Release tests passed for all nine
async asset tests, 66 Minori unit tests and 31 Minori scenario tests. New
scenarios assert the two text origins and the actual recolored ANI patch
pixels for both fixed-mouth and fixed-eye operands.
