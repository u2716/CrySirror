# Majiro native adapter

Onboarding mode: **adopt** (ADR 0102). This first-class crate owns original
MJO parsing/execution and the native syscall semantics; pixel, glyph, audio,
storage and presentation work goes through shared CrySirror services. No
original EXE/DLL is executed. Native findings cite the examined module's
functions (static IDA analysis; see `SOURCE.json`).

## VM (`machine/`)

- X/V1.000 MJO reader with bounded validation; fixed CRC-table XOR.
- Stack machine with the native integer/float/string semantics, arrays
  (401D60/40CB00/40EE00: 1..3 dimensions, innermost last, positive-modulo
  subscripts, reference sharing), argument stores, switch/case selector.
- Frame click targets `0x83B..0x847` (40D0A0..40D300): left/right/middle
  targets, AIP4 and destructors. A click observed by the 40B350 input pass jumps
  every frame whose target lies ahead and abandons a parked native call.
- Destructors run on return (40D490) and on thread kill (frames without one
  are discarded).
- Cooperative threads, event hooks (`event_hook[_after]`, `do_event`,
  thread-targeted events), module loading (`ExecuteScript`, named calls) and
  `RunScript` (409D90: reset thread 0 and enter a scenario module).
- `wait(-2)` yields one scheduler round inside the same update (49ADE4) instead
  of costing a frame. The adapter runs up to 64 such rounds per update.

The optional executable is read as PE data to identify reviewed early,
classic and extended x86 native layouts. Identification checks the hash/argc
reader,16-byte typed stack records, integer return construction and registered
page/mode/string call targets; later layouts also check their picture-password
contract. Early engines do not require a password native they never registered.
Executable renaming or unrelated file metadata does not select an ABI. Unknown
or ambiguous layouts fail explicitly. Extended CRT string scratch holds2048
bytes; early/classic hold512.

Original ARC V1/V2/V3 archives are mounted through the shared resource reader.
V1 derives entry lengths from the next record, including its terminal offset;
it is not parsed using the later explicit-size record layout.

MJO bytecode uses the fixed1024-byte CRC-table XOR. Encrypted RCT pictures use a
separate password set by the game's `SetEncryptionKey` script call, from which
the shared decoder derives the picture XOR table. These are distinct resources.

## Graphics (`adapter/gfx.rs`)

Majiro is a software-composited engine: sprites draw *destructively* into
their destination pictures during the native dirty-tile redraw (43C790,
160x40 tiles), and scripts read pictures back (front-buffer captures for
transitions, `grp_point_get`). Pages are therefore retained as VM-visible
native pictures (8-bit indexed with palette, or 24-bit BGR, with an optional
linked transparency page). All pixel kernels are the shared
`crysirror_image::transparency_planes` capability (copy, transparency blend,
masked blend with destination-mask update, additive, weighted fills, stretch,
swap, duotone, mosaic). RCT/RC8 planes are decoded by the shared
`crysirror_formats_arc::images_c::majiro` readers. The primary picture is
published as an engine image resource on layer 1 when its content changes;
`SetViewportOffset` wraps it like the native display offset.

Sprites follow 43DBE0/43CDC0: move interpolation, keyframe tracks for cells,
transparency and zoom, priority lists with groups, push/pop hiding, render
lock, paste, and rotate/zoom into a temporary picture (nearest sampling; the
native 448750 filter is not reproduced).

## Text (`adapter/text.rs`)

Glyph coverage comes from engine glyph-image jobs (cached per face, size and
character). `fontout` (413360/4132C0/4131E0/413560) and PROC (417070) compose
it with the native colour and shadow modes into pages. PROC lays out one line
per execution with gyoutou kinsoku, and fires `CONSOLE_ON`, `NAME_DISP`,
`CONSOLE_WROTE` and `CONSOLE_CLS` with the native PC rewind, so the original
script hooks own text animation and click waits. Advances assume the
monospace GDI cell (full width = 2 x half width).

## Input, regions and sound (`adapter/syscalls.rs`)

The shared logical buttons map through the native key table (word_481F04):
Enter/Space act as left click, Escape/Insert as right click, Shift/Ctrl as the
middle (skip) code, arrows as navigation. Hit regions (42F090/42F1B0) provide
hover/leave/click callbacks, keyboard focus movement and background handlers.
Music, four sound slots, voice and twenty voice channels use shared audio
channels with the portable volume settings.
Native channel-state queries use the shared mixer source clock: zero when
stopped, one before the first endpoint, and minus one while still active after
the first pass. A loop is not considered completed merely because looping was
enabled. Stopped native handles remain absent while a retired mixer voice fades.


## Settings, saves and files

- Portable settings live in the client Save namespace. Setters commit at
  once (the native 4430E0 writes synchronously); persistence is a coalesced
  background atomic write. Strict original registry import is unsupported.
- Saves (`adapter/saves.rs`) capture what 41F7A0 captures: thread 0 frames,
  stack and thread locals plus the save-scope globals, label and BGM. Slots use
  the original names (`majiro_%04d`, `quick%d`, `last`) as **portable
  CrySirror JSON records, not interchangeable with MajiroSavV1.103**. Load
  follows 424CB0: transient sprites/pages are released, init-time ones are
  kept (43C220/4403F0), thread 0 is restored and `HOT_RESET(1,"",0)` runs.
  Persistent globals and the `SetFlag` table are kept in a separate system
  record.
- Input files share one buffered line per handle exactly like
  4026F0/402420/4029E0.

## Message history (`adapter/history.rs`)

PROC records each message once in the native 40000-byte record ring beside
the console controls (4156E0). `6621F84D`/`2CB1BC41` walk page starts like
416B50/4169F0, and `307F28BC` replays a page like 416280: controls run
through the console with recording off but hooks on (`CONSOLE_CLS`, `CRLF`),
text runs PROC in replay mode (`CONSOLE_WROTE` receives replay=1), with the
native PC rewind between records. Voice-history records are not replayed.

## Not implemented

Save-slot thumbnails, voice replay from the backlog, movie playback (`movie_play` is skipped with a visible warning),
the exact 448750 rotation filter, auto/skip modes, and original save-file
interchange. Physical optical-drive emulation is not provided. Unknown syscalls
fail visibly at their original PC.

Native joystick modes use source-separated host controller samples and retain
their original two-controller key mappings. `media_policy: "installed_resources"`
checks this client's original executable metadata, ENV and entry MJO through
shared storage; missing/read-failed resources retain the native call while the
shared UI offers Retry or Cancel. Automatic directory detection selects that
portable installed-data policy; manually authored configs default to the explicit
native-optical requirement.

The desktop always links Majiro and detects original archive/ENV layouts
without extraction; no per-engine feature flag is required. Original game
data, fonts and modules are never distributed.
