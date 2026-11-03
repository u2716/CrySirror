# AliceSoft System 4 native work

`crysirror-system4` reads original AIN v5 and executes a bounded native VM
subset. It is a first-class Rust workspace crate under GPL-2.0-or-later. No
original C engine, renderer, SDL loop, font renderer, mixer or composed screen
is embedded. See SOURCE.json and LICENSE for exact pinned provenance.

The VM retains original byte PCs and i32/f32 bits, heap slot identity, sequence
and reference counts, typed global/local/struct/array pages, two-word variable
references and original function argument/return convention. All global storage
exists before initial values, reference initializers, the original function
named `0`, child-before-parent global constructors and main. Constructor and
destructor calls are resumable continuations. Multidimensional arrays remain
page trees. Unknown types, instructions and native calls fail visibly.

HLL calls retain their PC, stack and heap until typed completion. Capture by
cloning preserves native state, including pending calls and destructor work;
this is not a serialized framework snapshot or original save implementation.
The worker adapter explicitly rejects both serialized snapshot methods.

`worker_adapter::NativeAdapter` implements the real core VM port. The new
optional `runtime` feature provides `factory::register`; the desktop `system4`
feature now registers this factory and detects original System40.ini roots.
The linked immutable player33 was built by the release owner; complete native
play-through acceptance remains pending. Shared TaskPool workers own capability reads, envelope
and program parsing, layout validation, initial heap allocation and native
instruction batches. Only the owner completes shared host calls. The thin
bridge currently maps GetTime/Sleep to the shared engine clock, preserving the
Sleep operand and PC until its actual deadline. Config explicitly carries the
native logical canvas and Gothic/Mincho font aliases. SACT2.Init first verifies
published shared font faces, measurement and device-free audio availability,
then atomically creates the opaque-black hidden main Canvas and engine Layers.
Missing canvas/fonts fail without resource commit; same-client repeat Init is
idempotent and cross-client reuse is rejected. Backend readiness is not a claim
that native sprite/image/text/audio HLL functions are complete.

The reviewed 2004 OutputLog DLL obtains an optional *loaded Windows debug
interface*. With no such interface it returns `Create = -1`; that explicit
branch is protected by the original DLL SHA256. This is not a successful log
resource and does not copy xsystem4's private log system. A connected Windows debug interface,
unknown library revisions and unknown exports fail. The next actual Rance7
wall has moved beyond the original fallback `system.Output` (`CALLSYS 6`).
The bridge decodes its CP932 string and appends to the existing client-scoped
engine log, then acknowledges the exact pending call. It leaves the original
string on the native stack; subsequent native instructions release it. The
Engine script log and native Output share one canonical bounded ring.

Actual frozen Rance7 CPU owner validation executes435,187 original instructions
and enters main. The real DrawGraph binding and MsgSkip92120-message history
initialize successfully. A later unsupported SACT2.SetWP at PC0x15bde in
WP_SET_COLOR is retained; its fatal path waits for actual read-history atomic
save before reporting the error. Artifact SHA256:
e0abb3dad2f1c62bdb75dd1fdfdb32e4ee8956f8f01accfcbb88b1961a94c883.
The explicit owner test passes1/1 in0.53s. This proves no title/render/gameplay
acceptance and predates the next scene-background source batch. Public
fixed-success plugin/MsgSkip stubs are not adopted.

Shared ALD verification read6606 native CG entries across GA/GB6506+100.
A separate explicit owner test decoded all37 nonempty AliceLogo2 ALK QNTs,
allversion2, totaling11,220,069 pixels. That proves shared resource decoding,
not the real seasonal logo animation or its rendering.

S_MOD maps native types/refcounts to crysirror_text::format::format_one_bytes.
That reusable device-free kernel replaces one compatible `%d`/`%s` field,
preserves encoded bytes, supports bounded width/zero padding and keeps the
original integer/string precision semantics. `%s` copies its declared byte
length including NUL values; templates containing NUL are explicitly outside
this profile. Unsupported conversions fail before native stack/heap/PC changes.
Native string arguments are released only when a matching `%s` consumed them,
while the original format slot is always released. No private printf kernel
or font/layout implementation is kept in the VM.

Actual release validation and owner bootstrap evidence are recorded separately
in STATUS.md and ignored `target/verification/system4-*` logs. Owner game files,
DLLs and fonts are never included. The native executor remains partial: later
opcodes, complete HLL lifecycle/settings, rendering, UI, text, audio, saves still need their original semantics and shared API mappings.

Original CP932 INI/bootstrap, ALD native class/one-based-number policy, array
reallocation/list/fill/copy operations and42 shorthand operations are present.
Sprite properties use engine Layers; clearable keys use shared InputObservation.
Unavailable keys/pointer/focus and native RGB-only screen/multiply profiles fail
explicitly. No native pixel loop or composed framebuffer is hosted.

The optional `native-text` feature (enabled by default and by `runtime`) maps
eleven string/character/numeric opcodes to `crysirror-text-legacy`, a separate
first-class GPL shared crate. The VM only supplies original stack, borrowing,
slot-reuse/refcount and return policy. The shared operations preserve broad
lead-byte masks, signed chars, stored extent versus NUL prefix and the original
numeric compaction tail. Worker cancellation is passed to bounded scans without
retaining a callback inside logical state. Native undefined overrun/atoi overflow
is an explicit error at the original PC.

Shared encoded-image metadata uses Core ImageEncodingInfo queries, the same
Image decoder selection/header parser as pixels, and bounded cancellable
Runtime workers. Plane declarations remain distinct from decoded opacity;
metadata does not validate compressed payload checksums. Native CG uses these
receipts and source fingerprints before shared ImageProcess/ImageResource/Layer
publication. Missing files follow original return0; corrupt/unsupported data is
an unacknowledged error. There is no private QNT parser or pixel compositor.

Native Math scalar/RNG/table work stays on its shared worker. Random words come
from Resource Mt19937_1998; time(NULL) seeding awaits actual scoped CivilTime,
separate from virtual game time. Nested LockPeek/UnlockPeek preserve real depth,
and input uses engine InputBuffer receipts with explicit Peek/Update delivery.
Unavailable backend observations use the actual VmAdapter input frame;
pointer/focus queries still require observed values. The exact32-op native
callback cadence and manual per-client presentation fence remain incomplete.

MSG calls the original AIN message function with id/count/owned bytes; it does
not privately render text. FT_ASSIGNS preserves its returned owned string.
Native OJMP metadata drives resolver/no-resolver behavior and SJUMP releases
old frames top-down with resumable destructors. Explicit empty SLBL remains
distinct from no SLBL. The public reference's guessed no-metadata preallocated
scenario-page profile is not substituted for the reviewed native ABI.
A_FIND/SH_A_FIND_SREF preserve raw-word/NUL-prefix comparisons and owned versus
borrowed keys, and call real AIN comparators across service waits. Invalidated
callback array identity/layout fails instead of using stale original pointers.

Release evidence is separated by source point: a full11-target batch passed77
ordinary tests/5ignored. The final startup/input and callback-pointer guards
then passed43/3 across find7,VM35/1,sact1/2. Logs are
system4-functions-find-final.log and system4-final-scoped-guards.log.
Shared byte-text6/0,Image135/2,QNT5/0 and final Runtime worker1 plus integration2
passed independently; ignored cases are never counted as passed. Python tooling
201/201 and final QNT-ledger16/16 passed. The first new owner attempt exposed
an optional-input startup regression; that failure and its frozen binary remain
recorded, alongside the corrected passing435040-instruction owner run.

Immutable desktop player33 predates these additions. No new full desktop player
was built by this adapter task. Full DrawGraph/plugin capability extraction,
SACT text/effects/audio/UI, native delegates, AliceLogo2 animation,
original saves and resource reconstruction remain incomplete.

Generic CopyAlpha in Core/Image/Runtime copies or opacity-interpolates only
stored alpha while retaining destination RGB. Standalone Image1 and Runtime2
new+3 prior release tests pass with no ignored cases, including clipping/source
phase, client isolation and dropped-query cleanup. API docs measured42a8c3604407b796
with independent no-bless8+coverage1passed; later private adapters add no public
shape. Native DrawGraph now maps six bounded RGB/alpha/RGBA copy/fill exports to
those common Image jobs, immutable resources and retained Layers. Native name
registration occurs only after a real scoped image interface binds; exact repeat
Load returns0. Remaining exports/unknown plugins/alpha-less destinations fail.
Its independent library17/17, sact1/2ignored and sprites2/0 passed separately.

The next private module batch maps System10/12/14/16 file queries, save-folder
alias and game name onto scoped engine storage with real case-aware worker
receipts. SaveFolder is a native logical alias for that client's Save capability,
never an ambient OS directory. Original installed SaveData is readonly fallback.
Native MsgSkip maps all10 reviewed2004DLL exports: exact DWORD count plus
MSB-first bits, padding-bit count, real UseFlag policy, repeat Init and corrupt
save reset. It uses shared Core integer_fields, capability IO workers and atomic
storage writes. Normal halt, requested close during native Sleep, and fatal
unsupported HLL wait for the actual flag write before exposing halt/error; the
original unsupported PC/wait is retained. These records do not implement original
VM/global/group/resume saves. New release library26/26 passed, including six
MsgSkip tests; the original owner result above is separate. No Desktop rebuild.


The complete scene.c background-setter family now maps clear colour, numbered
CG wallpaper and sprite RGB snapshot to common Layer/Image capabilities. New
wallpaper snapshots are immutable and opaque where the native RGB-copy policy
requires it; deleting/repainting the source sprite cannot change them. A missing
CG leaves the old background intact, while corrupt image data retains its call.
Background layers remain beneath even minimum-z sprites. Four independent Runtime
cases passed inside library30/30; original game rendering is not yet verified.

A_SORT/A_SORT_MEM now run bounded rank1 scalar/string/member/custom families as
VM continuations. String comparator arguments are fresh owned slots; struct
arguments borrow original references. Real AIN callbacks may wait at HLLs;
final sorted words publish only after actual returns and heap identity/layout
validation. Custom/member equal keys preserve original index. A deterministic
merge schedule does not reproduce platform qsort callback call order or unstable
equal-key permutations; such side-effect-sensitive native behavior is not
claimed. Multi-rank/unreviewed value profiles fail explicitly. No generic
renderer/resource algorithm is stored in this native heap operation.

Combined release82passed3ignored (library30, find7, sort9, sact1/2,VM35/1) passed
in system4-native-scene-sort-final.log. Final comparator guards and member-string,
cancellation/heap-reuse regressions then passed11/11 in
system4-native-array-sort-guards.log. Bundled Python ran201tests OK(1skip);
the default XcodePython3.9 attempt failed6 imports and is retained separately.
This source postdates frozen e0abb3... owner and immutable desktop player33;
no newer original-game/desktop/render/title result is implied.
