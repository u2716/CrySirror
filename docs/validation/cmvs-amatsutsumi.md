# CMVS Amatsutsumi sample

2026-11-18. The supplied Amatsutsumi installation is the first CMVS target.
The [native ABI](../../crates/crysirror-cmvs/NATIVE_ABI.md) records the exact
Windows x64 executable and startup-script hashes and the pinned analysis source.

| Area | Evidence |
| --- | --- |
| Sample inspection | Read-only SHA-256 of the executable and script matched the pinned Amatsutsumi record. Static PS2A decoding found 5,141 expanded bytes, 4,726 code bytes and entry PC `0`. The `ps.cpz`, `script.cpz` and `bg.cpz` headers read `CPZ6`. |
| Startup boundary | Static decoding puts the BGM source registration (`0x2013`) at PC `0x00de`, the script archive registration (`0x2010`) at `0x0114` and the first save-menu scan (`0x22b3`) at `0x025a`. The CMVS VM records the resource bindings and issues an owner request for the save menu. |
| Archive profile | The shared CPZ6 mode 1 reader mounted the original `script.cpz` with 87 entries. It decoded `main.ps3` with entry PC `0x38d1e` and 235,208 code bytes. The 96-byte table was read through the selected client's file-backed `.rdata` range. |
| Compilation | The macOS arm64 Release player built with `cargo +stable build --release --locked -p crysirror-desktop --bin crysirror`. The local run used `CARGO_PROFILE_RELEASE_LTO=false`, `CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16` and `CARGO_PROFILE_RELEASE_DEBUG=0`. |
| Headless capture | `play --headless --frames 220 --no-audio` exited 0 and wrote a 1280×720 nonblank brand-logo frame. The runtime reported frame 220, `logo.ps3+0x36566`, `window_created=false` and `audio_emitted=false`. |
| Title regression | The earlier build failed at `menu.ps3+0x36704` (`0x2210`). A subsequent `--frames 1150` headless run exited 0 with the native title loop active. Its 1280×720 capture contained the five script-drawn button layers. |
| Native OP | An idle-title run at `--frames 3300` exited 0 in `op.ps3`. CMV6 playback reported 225 published frames and a 121,125 ms duration. Shared audio prepared the indexed Ogg track, and the offscreen capture had no audio output device. |
| Headless input replay | Enter at frames 230–232 shortened the logo sequence on the earlier build. After the title fix, a mouse replay at canvas position (641, 281) activated New Game, initialized the toolbar and message windows, and opened `cmovie.ps3`. |
| First chapter | The same replay at `--frames 7600 --no-audio` exited 0 with `window_created=false`. It finished the first story movie, switched to `sn1001.ps3`, captured a scene checkpoint at resume PC `0xe39e` and rendered the first five glyphs on the 1280×720 canvas while waiting for the next click. |
| Dialogue and exit | A 6,400-frame headless replay with eight additional clicks passed without a VM error and left a later visible glyph run. A separate 1,500-frame title replay chose Exit and confirmed the dialog. The CMVS VM reached `Halted` and the process returned 0 with `window_created=false`. |
| Bytecode corpus | The installed audit decoded 2,957,772 instructions across all 87 archived scripts and the loose startup script. The [dispatch inventory](../native-api/cmvs.md) records the 14 distinct non-system instructions and 431 syscall selectors found in the corpus. |
| Tests | Thirteen CMVS tests, including the opt-in installed archive and bytecode audits, and twelve Video tests, including CMV6 decode/rewind on the opt-in original OP, passed in Release. The Python upstream-provenance check passed. |

The player runs used the owner's original installation as read-only input and
an isolated temporary save root. The original Windows executable was read as
data. Its hash and script hashes are validation evidence. The launcher
identifies the reviewed ABI by the native layout and the PE build stamp.

Later on 2026-11-18, static inspection of the installed `cmvs64.exe` matched
all 976 case RVAs in the `Pc20160201X64` dispatch inventory. Synthetic Release
checks exercised the newly connected arithmetic and byte-string operations,
including opcodes absent from this installation's scripts. The runtime build
passed 13 CMVS tests and the standalone VM build passed 12 script tests.
The later pass used synthetic scripts only. Full game acceptance is pending
until the remaining ABI operations are implemented.

A subsequent synthetic Release pass covered CP932 format fields, fixed-width
reads across NUL bytes, writes to loaded script text, three aliased integer
results, Windows-style local clock registers, and native 3D axis order and
quantization. With `runtime`, all 16 CMVS library/script tests passed. The
standalone VM passed 14 script tests. Neither pass used an installed game.

The text settings pass used synthetic bytecode to check mode, clamping,
result registers and stack reclamation. It also checked font-name copying
into loaded script text and preservation of an existing glyph's weight when
the setting changes. The Release runtime passed 18 library/script tests,
and the standalone VM passed 14 script tests. The window-focus policy and
complete settings-screen behavior still need installed-game acceptance.

The audio settings pass compared the x64 volume wrappers and the two effect
gain paths in `0x140017750`. Release checks passed 19 CMVS library/script
tests, seven shared gain tests and two runtime media tests. The tests checked
both master and category gain, client isolation, retained media settings,
rollback on invalid targets or gains, and unchanged transport fade deadlines.
The Release check of `crysirror-launch` passed. The tests used synthetic
inputs and did not run the game or an audio device.

The audio switch pass checked BGM intro/loop retention, a zero-fade restart,
and restoration of paused versus already-started looping effects. Eleven
shared audio replacement tests passed in Release, including proof that
invalid channel scope leaves live playback unchanged and that other client
channels retain their exact transport state. The CMVS runtime passed 21
library/script tests. These were synthetic checks.

The independent audio channel pass inspected the original x64 channel 15
wrappers, the ASSET source registration, the DirectSound elapsed clock and
the per-voice amplitude arithmetic. The Release CMVS runtime passed 22
library/script tests, including fixed-point gain rounding and overflow for
the independent channel. This pass did not exercise installed-game playback
or an audio device. Matching the DirectSound position clock and reproducing
gain arithmetic during a native fade remain open.

The stereo meter pass connected `20ae` and `2243` using the x64 startup
buffer table and three-window PCM history. Two shared Release tests checked
signed PCM16 conversion, fractional window boundaries, pre-gain levels,
client isolation, pause/resume, replacement and mixer-worker publication.
The Release CMVS runtime passed 23 library/script tests, including all four
native buffer modes at both quality settings. These synthetic checks did
not exercise installed-game playback. The host-rate resampler and original
DirectSound buffer-fill timing still need parity evidence.

The fixed-point gain pass connected `20ac`, `20ad`, `20b8` and `20b9`.
An SE reopen now publishes the retained multiplier and stereo gains
alongside the replacement source. Channel 15 uses the same fader for
`2244`. The shared Release audio library passed 84 tests, with two
ignored, and CMVS passed 23 library/script tests. The new tests exercised
wrapping products, signed PCM16 shifts, the first frame after replacement,
live update without seeking, and the reopen rule for a saved negative or
out-of-range multiplier. These were synthetic tests. BGM ducking, native
fade timing, and game acceptance remain open.

The device-gain pass connected `20a5` to the mapped software channels
and `20a7` to prepared voice playback. Release passed 85 shared audio
tests, 23 CMVS tests and 20 focused runtime media tests. The device test
checked gain set before source publication, the pre-pan shift order,
client isolation and unchanged playback state on gain updates. The media
test checked the PCM16 fader on initial and resumed reader audio. Three
tests in those scopes were ignored. The full Runtime library run also
reported eight Siglus failures, seven from a missing registered adapter
and one from an empty initial integer stack. This pass did not run the
installed game. BGM ducking and native fade timing remain open.

The resident sample pass connected `2248`, `2249`, `224a`, `224b` and
`224f`. The shared Release audio library passed 86 tests, with two
ignored. The CMVS Release library and script suite passed 24 tests. The
new tests checked decoded PCM sharing, continued budget ownership through
the final mixer voice, primary-only stop, independent one-shot duplication,
device gain at play time, and release of every instance in a slot. No
installed game or audio device was used. Exact DirectSound capacity, the
duplicate-volume behavior and decoder parity remain open.

### Software effect and voice transport

2026-11-19. IDA 9.0 with Hex-Rays recovered the effect pause and rewind
wrappers (`20b2`, `20b4`), voice retirement and wait (`20bc`, `20bd`),
and allocated-voice activation (`20a7`). The original mix thread and
frame error check established the retirement and invalid-slot boundaries.

The scoped Release run passed 89 shared audio and 24 CMVS library/script
tests, with two shared audio tests ignored. Synthetic cases covered a
retained pause and cursor-preserving resume, EOF activation without
phantom PCM, decoder rewind without reopening, fade ownership through an
asynchronous rewind, and retirement at mixer publication. The installed
game and its audio device were outside this pass. Open native buffer
accounting is tracked in
[SATD-cmvs-software-playback-clock](../../crates/crysirror-cmvs/SATD.md#satd-cmvs-software-playback-clock).

### Pointer gestures and newest save-slot query

2026-11-19. IDA resolved unrestricted pointer drag mode 2 (`2364`) and the
result and threshold commands (`236a`, `236b`). The focused Release tests
checked the activation threshold, live deltas, release frame, click edge
consumption, zero speed for an axis-aligned drag, and native result slots.

IDA resolved `22b7`, reported at `intproc.ps3+0x6801c`, to the original
newest-slot header scan. The CMVS Release library and script suites passed
29 tests. Synthetic save headers covered the raw operand range, missing
folders and slots, accepted signatures, case-insensitive lookup, literal
filename prefixes, ties and the original short-read scratch behavior. The
tests used a read-only save capability. This run did not replay the
installed title.

The Release desktop player built and ran a synthetic `start.ps3` through
`play --headless --no-audio` with an isolated save directory. The script
asserted slot 7 over the timestamp tie with slot 8, asserted −1 for an
invalid range and reached `Halted`. The process exited 0 and reported
`window_created=false` and `audio_emitted=false`. It used the installed
executable only as read-only CMVS profile data.
The same script and slot assertions passed again on a rebuilt Release player
with a fresh synthetic save directory.

### Existing-slot metadata

2026-11-19. IDA traced `22bb`, `22bc` and `22be` to the native header
signature test and the two in-place edits of the 600-byte prefix. The
CMVS Release library and script suites passed 31 tests. The added cases
covered independent short-read scratch, bounds, missing and unrecognized
slots, cancelled or rejected writes, and preservation of the payload.

The Release desktop player ran a synthetic `start.ps3` with
`play --headless --no-audio` and isolated save data. It halted after
ten script assertions covering the existence probe, comment and word
edits, rejected edits, and the existing newest-slot query. The edited
file matched the expected bytes across its entire length. The short
slot kept its bytes, the missing slot stayed absent, and the run
reported `window_created=false` and `audio_emitted=false`.

### Object image release and save-history limit

2026-11-19. The CMVS Release library and script suites passed 32 tests.
The added history case checked the default of 32, the zero and full
limits, newest-first checkpoint capture and live history length.

The Release desktop player used a synthetic script and a one-pixel BMP
with `play --headless --no-audio`. The script loaded, released, reloaded
and released the same image again using the original scene object. It
verified preserved result registers for `2031` and `22b1`, zero live
image resources after release and normal termination. A second script
verified that `2031` reports frame error bit `0x10` for a missing
object. Both fixtures used isolated local data directories.

### Shared preview image codecs

2026-11-19. IDA traced the original MSK0 payload offset, the PB2A header
transform and the memory entry point's frame and name arguments. The
registry's Release image suite passed 18 tests. Four new fixtures covered
MSK0 row order and non-default payload offset, PB2A edge tiles, literal and
matched LZSS bytes, JBP1 padding crop and alpha runs, the first type 5 frame, and the
missing type 6 name. They also covered malformed offsets, dimensions and
payloads. The two existing external-corpus tests were ignored. This
exercised the registered decoders independently of the `22bd` syscall.

The layout review compared the pinned GARbro MSK0 and PB2A readers to the
IDA results. The original MSK0 reader uses the variable payload offset, and
the original PB2A dispatcher has additional paths beyond GARbro's decoder.
[GARbro MSK0](https://github.com/morkt/GARbro/blob/b09ee4570ccb1daf6ac56710ee8934dc0b8baeb0/ArcFormats/Cmvs/ImageMSK.cs)
[GARbro PB2A](https://github.com/morkt/GARbro/blob/b09ee4570ccb1daf6ac56710ee8934dc0b8baeb0/ArcFormats/Cmvs/ImagePB2.cs)

### Detached preview preparation and PB3 variants

2026-11-19. The CMVS Release library and script suites passed 36 tests.
The new tests check CSV2/CSV9 metadata, signed slot guards and wrapping
filenames, bounded LZSS expansion, cancellation, independent child images,
checkpoint ownership, release, move and rejected layer publication. The
slot tests use a read-only save capability. The scene tests use a recorded
host and exercise replacement and retirement without an output device.

The Release image suite passed 20 tests with two external-corpus tests
ignored. Synthetic PB3 files exercised variable block sizes, interleaved
BGRA with and without native alpha, a selected edge tile over a named base,
and required-base failure through the scoped decoder. `22bd` remains open
in the native inventory. These tests exercised the decoder and scene
prerequisites independently of syscall dispatch.

### Save metadata and preview syscall

2026-11-19. IDA traced the unnamed memory loader, its BMP offset and row
layout, the conditional PNG palette transform, PB3 depth-driven alpha and
four-plane differential decode, and the MSK0 scene stride mismatch.

The Release desktop player built successfully. The CMVS library and script
suites passed 37 tests, the image registry suite passed 22 and the PNG row
test passed. The VM transaction test rejected one scene commit and verified
that the original strings, result registers, stack and PC remained intact.
Two external image-corpus tests were ignored.

Six Release `play --headless --no-audio` runs used synthetic CSV slots and
scripts. All exited zero and reported `Halted`, `window_created=false` and
`audio_emitted=false`. The 86 script assertions covered metadata and byte
stack results, slot wrapping and guards, black fallback, absent and
out-of-range child targets, root and child replacement, and accepted
metadata after failed image loads. Twelve capture checks covered BMP, PNG,
PB2A, PB3B through scoped base lookup and empty image replacements. The
failed-image runs covered unknown magic, the original MSK0 upload stride
mismatch, missing PB2A and PB3B variant names, and a missing PB3B base.
This closes `22bd`'s preview record. Original whole-slot save and restore and
installed-title playthrough acceptance remain open.

### Native thumbnail capture

2026-11-19. IDA traced `22b8` through both D3D9 readback branches, the integer
box sampler and the native BMP writer. It also confirmed that `2003` releases
the retained bitmap. The CMVS Release library and script suites passed 38
tests. The three focused BMP encoder tests passed. The adapter regression
compared the retained bytes directly and checked that synchronous and deferred
capture failures preserved the previous bitmap and VM results. All five
shared Canvas image tests passed, including the new case verifying that
removing the presenter terminates a pending frame capture with a failure
response.

The Release desktop player built and ran three synthetic scripts with
`play --headless --no-audio` in isolated save directories. They all exited
zero, reached their expected halt locations, created no window and emitted no
audio. The input was a 640×360 gradient. Eight pixel checks confirmed the
rendered input, and separate reference code built the expected BMPs to compare
byte lengths and fingerprints. A 3×2 capture retained 78 bytes with fingerprint
`d9b9b900274b475c` after a rejected zero-dimension request. The 641×2 capture
matched the original zero-width sample rule and retained a black 3,902-byte
BMP with fingerprint `3231617d0b66cbb0`. The reset case reached `main.ps3`
with no retained bitmap. These runs validate in-memory thumbnail capture.
Native whole-slot serialization and installed-title playthrough acceptance
remain open.

### Isolated CSV2 slot codec

2026-11-19. IDA identified the original header, optional checksum trailer,
three packed ranges and expanded PS2A image. The original slot writer at
`0x140080b20` was emulated locally with synthetic input and in-memory host
stubs. It emitted a 677-byte CSV2 fixture. The Rust codec recovered its
checkpoint, thumbnail bytes and expanded script, including the original
script header. In a separate 7,301-byte test, the Rust decoder expanded a
stream from the original compressor, and the original decoder expanded the
Rust stream. Both expanded outputs matched the input. Both encoders emitted
752 bytes with different match references.

The Release CMVS library and script suites passed 42 tests. Two installed
corpus tests remained ignored. The focused shared LZSS run passed eight
tests. Tests covered a trailer-free checkpoint, disabled thumbnails, invalid
range lengths and checksum, invalid script labels and cancellation. The
codec is isolated under `SATD-cmvs-x64-dispatch`. Native checkpoint state
mapping and normal `22b5`/`22b6` behavior remain open.

### KUNADO 2021 profile boundary

2026-11-19. IDA static analysis of the KUNADO `cmvs64.exe` (PE stamp
`0x615e1fde`) recovered the CPZ7 V7B reader and keyed archive registration.
A local check opened all 11 archives with the words from `start.ps3`. It read
every entry of `script.cpz`, `video.cpz` and `voice2.cpz` and a sample of the
others without a checksum failure. All 59 scripts decrypted to PS2A and decoded
without an instruction error.

The Release player later ran KUNADO through `play --headless --no-audio` with
isolated data. A 2,500-frame run drew the title. Input replays opened the load
screen and, over 6,000 frames, started a new game and drew the story opening
with its toolbar and a selection bar. Both exited 0 without a window. Route
coverage, settings, saves, audio and the OP movie were not exercised.

### Placeholder families, both profiles

2026-11-19. After the remaining placeholder families were implemented (see the
2026-11-19 rows of SATD-cmvs-20211007-profile), the Release player replayed both
samples with `play --headless --no-audio --frames 30000` and their recorded input
tracks. KUNADO (2021 profile) exited 0 with its main checkpoint in `sn1010.ps3`;
AMATSUTSUMI (2016 profile) exited 0 in `sn1001.ps3`, drawing story text in its
message window. `cargo test --release` passed for the CMVS library (38 tests with
`runtime`), `crysirror-image` (235) and the `crysirror-resource` profile reader.
Most new families (effects 1/3/4/7/9, rasters, picture stacks, image font, text
debug, slot file commands) have no call site in either sample, so only unit tests
exercise them.

### Device setting and glyph alpha revision

2026-11-19. IDA matched the 2016 `0x22ff` device word to the startup reader
for `WINDOW_RESOLVING/OTHERS_DISPLAY_USE`. The 2021 outline rasterizer
exposed both 17-entry alpha tables selected by `0x22f9`. The shared outline
rasterizer uses those tables with its own 4×4 coverage samples.

The Release player ran three isolated scripts through `play --headless
--no-audio`. The two 2016 scripts branched on integer result 0 with the
other-display setting off and on. The 2021 script queried both values of
the alpha setting, persisted the changed value and retained the configured
font face. The write preserved the preceding query result. The captured
32 px glyphs had the same bounds and differed on 95 edge pixels. The local-font
test also verified that a supplied alpha table preserves glyph cell geometry.
The desktop build, CMVS and text Release tests and the API reference tests passed.

An additional 2016 script requested a target texture through `0x202b`.
The player reported an unsupported syscall at `start.ps3+0x18`, confirming
that the previous success without a texture change has been removed.

With the same release build and isolated data, the existing 30,000-frame
input tracks reached `sn1001.ps3` in AMATSUTSUMI and `sn1010.ps3` in KUNADO
and exited 0. The CVNS sample `MJGAMEDISC` exited 1 at
`start.pss+0x1e` on the unresolved `0x210e` bank audit call.

### Ground and walking scenes

2026-11-19. The Release player ran isolated PS2A scripts on the 2016 and 2021
profiles through `play --headless --no-audio`. Both rendered the ground and
translucent water in `0x2290` as projected meshes. The scripts created
`0x2291`, checked its idle and landing results through `0x22ae`, and used
`0x22af` to trigger the landing. A prepared long cue remained active until
`0x22a0` released the scene. After release, the player reported zero owned
textures and no active cue. The walking-state unit test separately checks
both footstep thresholds, the queued transition and the final camera pose.
No original Direct3D frame was compared pixel for pixel.

The desktop Release build and 41 CMVS library tests passed. KUNADO and
AMATSUTSUMI replayed their input tracks for 30,000 frames with muted headless
output and checkpoints in `sn1010.ps3` and `sn1001.ps3`.

### Staggered and rotating quad scenes

2026-11-19. Isolated PS2A scripts ran `0x2292` and `0x2296` through the
Release player with `--headless --no-audio` and both x64 ABI profiles. The
`0x2292` scripts checked idle, pending and active status for both phases.
Frame captures and the layer report showed the staggered alpha values and
the rising foreground quad. The muted mixer showed one active cue after
the first trigger and two after the queued trigger. The release took the
owned texture and active cue counts to zero.

The `0x2296` scripts checked both argument words, ignored the invalid slot,
observed the projected animated quads and tested `0x22a0` retirement.
Both profiles held two texture leases while drawing and released both on
retirement. The clock tests cover the per-quad phase offsets, delta cap,
alpha period and phase cue thresholds. Images and cues in these isolated
tests were synthetic. They exercise the player and resource lifecycle
without providing pixel-for-pixel comparison with Direct3D.

### Orbiting atlas scene

2026-11-19. Both x64 ABI profiles ran an isolated `0x2297` script through
the muted headless Release player. The script exercised slot bounds, result
0 through `0x22ae`, ignored trigger 9 and triggered image replacement
twice with key 10. The layer capture showed the additional overlay and
four planes. The scene held nine published images before and after both
triggers and released them all through `0x22a0`. Unit coverage includes
the 192 native atlas selections, descending bands and alternating clocks.

### Rule-image panels and rising billboards

2026-11-19. Muted headless scripts covered `0x2293` on both x64 profiles.
The left source used its 512-pixel variant. The right source fell back to
the alternate image and its rule. The early capture showed the left panel
becoming visible before the right, as expected from their green rule values.
The scripts waited through the reveal, opened and faded the panels, then
showed, hid and released them. The launch held three textures, rose to
five during the reveal and returned to zero on release.

The `0x2294` scripts in both profiles confirmed hold and growth states,
the resident cue, billboard texture replacement, particle-only fade and
the separate plane fade. The particle-only capture retained the five
planes. Retirement left zero active cues and textures. The scene timings
and phase values also passed the CMVS Release tests (46 library tests).
These captures used the desktop continuous-mask path. No original Direct3D
frame was compared pixel for pixel.

### Projected atlas actors

2026-11-19. Both x64 profiles ran `0x2295` with all of its script trigger
keys through the muted headless Release player. The state checks
covered actor entrances, paired jumps, the queued departure, and
polling while animations remained active. The captures showed the
three projected atlas sprites, including an explicit strip change, and
the mixer reported overlapping resident cues. Both runs ended after
1,400 frames with zero live textures and resident voices. The CMVS
Release library suite passed 48 tests.

After the final scene change, the two real-game input tracks completed
30,000 Release frames each. AMATSUTSUMI reached `sn1001.ps3` and KUNADO
reached `sn1010.ps3`. Both reports recorded no window or audio output.
The isolated scripts exercise the fixed 3D selectors independently of
the routes reached by these game replays.

### CG viewer second-texture mode

2026-11-19. The muted headless Release player opened KUNADO's gallery
from the title. A temporary opcode trace confirmed execution of the
previous failure at `omake.ps3+0x50074`. Separate 2,500-frame runs
also opened the scene and music tabs and returned to CG. Each run
finished with `window_created=false` and `audio_emitted=false`.

Synthetic PS2A scripts for both x64 profiles loaded a red sprite and a
rule with green 255 on the left and 0 on the right. Mode 4 drew the
sprite on the left and masked out the right. Mode 3 hid the sprite.
The CMVS Release library suite passed 49 tests, including child matte
inheritance.

### Console rule-image shaders

2026-11-19. Switch built and packaged the Release NRO using its deko3d
backend. UAM cooked 78 RomFS entries, including all seven entries keyed
to the renderer's exact quad v2 source.

The PS Vita compiler generated all ten new GXM mask fragments. The
compiled catalogue and cooked index passed their Release library tests.
The owner paused the Vita player build before linking and playback.
Console pixel behavior remains unverified on hardware.
