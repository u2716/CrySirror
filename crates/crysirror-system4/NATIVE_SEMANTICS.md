# Native execution and ownership audit

Reference VM: xsystem4 `a85467d78eb5c7fb6e581d7fe5a6fdaec42b9ef7`.
Format/type definitions: libsys4 `b14839f23af0963abbdb17f7cac57a36fd8e9bd2`.
The source-file hashes and licence notices are retained in SOURCE.json.

`vm.c`'s `_function_call`/`function_call` move arguments in original order into
local pages, while callee return values stay on the operand stack. References
retain their original page/index pair and ownership. AIN v5 methods do not add
the later v6.1 `this` reference. Native frame cleanup calls destructors before
removing the frame, then releases page members in reverse order. The Rust
continuation stack retains this order even if a destructor reaches a pending
HLL call. It does not execute a nested C interpreter or callback pointer.

`vm_execute_ain` allocates every global structure before ordinary initializers,
resolves reference initializers afterwards, runs function `0` if declared, then
runs recursive global constructors and main. `page.c` allocates struct members
recursively and invokes child constructors before the parent. Arrays are trees
of pages with native dimension order, not a flattened generic collection.
Heap slots use original LIFO reuse and independent allocation sequences. Null
page and null reference remain distinct. Intrinsic VM scalar defaults are
implemented only for reviewed types; no unknown operation gets a zero result.

The adapter's bootstrap and execution batches own the full mutable Machine on a
shared TaskPool worker. Admission is reserved before ownership moves. The owner
observes a completion receipt before reading/mutating that state or calling a
host service. A pending HLL or Sleep never advances its original PC. Limits:
64 MiB AIN, 1,000,000 heap slots, 8,000,000 accounted heap words, 65,536 operand
words, 4,096 call frames, 64 nested structs/copies and 32 array dimensions.
Errors remain native locations; public serialized snapshots/original saves are
unsupported. This is a bounded VM subset, not a full System4 implementation.

Static original OutputLog evidence takes priority over compatibility stubs:
Create RVA0x1000 obtains string bytes, checks a cached connection and calls
GetModuleHandleA("DPLogViewer.ade"), GetProcAddress("GetInterface") and a GUID
interface request. It returns -1 if any lookup fails. Only a real interface can
return a log handle. Disconnected Output/Clear return without dispatch; Save,
EnableAutoSave and DisableAutoSave return false. SHA validation restricts those
branches to the reviewed DLL. No Windows DLL is loaded or executed. The pinned
xsystem4 code creates its own incrementing handle table instead; that mechanism
and its unimplemented persistence macros were deliberately not adopted.

CALLSYS6 now appends its original CP932 bytes through the existing bounded
client log operation. It leaves the string for the original following S_POP.
The owner only completes the native call after the shared append commits.

Structure assignment now calls the destination destructor, releases members in
reverse order, then deep-copies the source into the same destination heap slot.
The copy occurs after destructors, which may modify source state. Reference
fields keep aliases; owned strings/structures/arrays get original ordered fresh
slots. Self-assignment retains native identity. Original ASSERTfalse is fatal.

The release-verified source adds allocation replacement, rank-zero free,
outer-only realloc and list mutations. A_REALLOC shrink releases removed
elements in ascending order; deleting a complete page releases them in reverse.
Nested existing arrays retain their dimensions when the outer array grows.
Native constructors/destructors remain resumable at external-call boundaries.

Original System40.ini syntax is bounded native configuration policy. Numeric
resource IDs remain original one-based AIN IDs; ALD link/physical parsing is the
shared formats-arc Archive API. Sprite geometry, visibility, tint and opacity
map to shared Layer state, with no private pixels/compositor. Native screen and
multiply need exact RGB-only destination-alpha-preserving shared profiles;
current general alpha blends are not substituted. Input uses shared owner
observations and clearable VM flags; no fabricated unknown/focus values.

The new string family uses the separately licensed shared text-legacy kernel.
S_GETPART computes bytes before releasing the consumed source, then allocates
the result heap slot afterwards, preserving native LIFO slot reuse/sequence.
S_LENGTH2/S_FIND/STOI consume owned temporaries; C_REF/C_ASSIGN and list edits
borrow the native slot. Character reads return signed native single bytes and
little-endian packed pairs. NUL-prefix traversal and stored length are distinct.
The reviewed numeric narrowing deliberately retains its original uncompacted
tail; undefined C overflow/overrun fails visibly before an opcode commits.
The worker passes actual cancellation into each bounded scan. Public step()
uses the same path without retaining a callback in cloned VM state.


Native Math uses one shared Resource MT19937-1998 state per Machine. SetSeed
forces an odd seed and discards the first word; table shuffle preserves its
original biased float*i selection. The Machine retains table IDs/cursor, while
all random-word generation is the common kernel. Civil-time seeding owns a
scoped asynchronous host query outside cloned logical state; a virtual clock
cannot substitute for Unix seconds.

Original Sys42VM static evidence corrects two public-reference approximations.
Peek-lock calls return actual nested depth, not fixed1. OJMP<0 leaves the string
on the operand stack; OJMP>=0 invokes the real AIN resolver, and SJUMP consumes
its scenario name and unwinds frames before entering a native label function.
The original reference explicitly marks its preallocated-page variant as a
guess; that profile is not silently used. SLBL presence, even empty, is retained
by the AIN reader and selects its explicit legacy-label behavior.

The MSG entry validates native id/count/string parameters and calls original
AIN code. Strings retain encoded byte extent, including embedded NUL, and are
released by ordinary frame cleanup. It creates no glyph/layout implementation.
A_FIND and SH_A_FIND_SREF retain native owned/borrowed key distinctions, original
NULL-page key lifetime, and scalar or NUL-prefix comparisons. Callback searches
are logical continuations which run original AIN functions and await their real
HLL completions. Array slot reuse/layout invalidation is an explicit error,
including a callback that returns a match after invalidating the page.

Engine InputBuffer stores scoped event receipts during native locks; explicit
Peek and Update drain them through clearable native flags. A missing backend
InputObservation before first publication does not erase an actually supplied
VmAdapter InputFrame, nor invent a pointer or focus value. Delivery at async
owner boundaries is not exact original32-op callback timing. Shared mutable
presentation/final-frame fencing remains required before visual acceptance.


DrawPluginManager stores exact C-prefix native names and requires an actual
client-scoped engine image interface before Load success. Same-name repeat
returns0; IsLoad reflects that table. Six DrawGraph RGB/alpha/RGBA copy/fill
exports await shared ImageProcess completion, publish immutable engine resources
and update retained Layers before acknowledging their original void call.
CopyAlpha is a reusable Core/Image/Runtime operation; no private pixel loop lives
in this adapter. Unknown exports/plugins and unsupported alpha-less targets keep
the native call pending and fail. This is partial DrawGraph support.

Native file-location policy maps System10/12/14/16 to scoped storage metadata,
logical SaveFolder alias and original GameName. CP932 decode and case-aware
capability lookup are shared engine operations; actual filesystem observations
run on the bounded worker. The native owned filename is released only after
receipt completion. A client's Save mount takes precedence over readonly
installed SaveData; traversal/drives/foreign clients fail before native ACK.

Reviewed2004 MsgSkip.dll differs from the public reference's warning stubs.
The exact save is little-endian message-count DWORD and ceil(count/8) MSB-first
bytes. GetNumofFlag includes padding bits. UseFlag defaults true; disabling it
makes valid GetFlag return1 and suppresses save while SetFlag still records bits.
Repeat Init flushes an active prior file, then loads a fresh exact-count image;
nonzero malformed engine-save files are deleted/reset. Original readonly data
is never overwritten or removed. Enable/state normalize to booleans. Shared
Core integer_fields owns generic bit addressing and shared storage/task services
own bounded IO and atomic writes; only native record/order policy stays here.

Adapter close/halt/error waits for read-history writes. Closing during Sleep
retains its PC/deadline without advancing virtual time or acknowledging it.
An already admitted Init may complete, while subsequent script instructions are
not executed. Fatal unsupported operations retain their error/native pending call
until read-history write completion; unfinished read/image/time queries are
cancelled. A failed atomic write remains visible, never a reported successful
save. These flags do not substitute for original global/group/resume saves or
serialized VM/resource reconstruction. All10 MsgSkip exports are independently
tested; actual original-game reachability and native undocumented controller
conditions remain separate evidence.


Native scene.c background setters have separate engine retained resources:
clear-colour backdrop and optional wallpaper. SetWP0 removes the latter only;
SetWP_Color leaves an existing wallpaper intact. Numbered CG waits for actual
shared header/pixel receipts and a matching source fingerprint. WP_SetSP copies
stored RGB through common CopyRgb into an opaque immutable resource, independent
of source position/show/opacity and later source deletion. Image decode/copy and
composition are never implemented here. Manual frame-publication timing remains
an explicit gap.

A_SORT/A_SORT_MEM preserve original key types and custom/member stable index
fallback. Member nonstrings compare raw signed words even when their declared
kind is float. String callbacks receive newly owned copies; struct callbacks use
native heap references. The bounded rank1 continuation carries a working sort
and publishes native words only after actual AIN comparator returns. External
waits retain exact calls; reallocated/reused original pages fail on shape/sequence
validation. Cancellation is checked between each native step. The deterministic
merge schedule is not a platform qsort call-order emulation, nor a claim about
unstable equal-key permutations or side effects relying on them. Unsupported
multi-rank/delegate-value profiles remain errors. Original VM saves are still
unsupported and these continuations do not make Machine.clone an original save.

SystemService's complete eight-method mixer family stores original config count,
names and raw default volumes as native metadata. Current volume/mute/hierarchy
are shared engine AudioMixGroup state. Renaming a native group never reparents
it. An invalid group index returns0 without changing its referenced output;
foreign clients, wrong AIN argument kinds and missing native config fail without
acknowledging the original call. BGI/WAI metadata uses the shared format parsers
on the startup worker. This mixer family is routed independently of the pending
SACT stream actor: Sound/Music playback is not registered merely because its
resource/PCM fixtures pass. Decoder reuse through stop/replay, callback/fade/EOF
and terminal cursor policy still require completion.

Shared native/media channel seek is a retained scoped operation, preserving the
same source, gain ramp, loop schedule and pause state. Ready acknowledges actual
cursor publication, survives later EOF and cannot be inferred from voice
absence. Worker results remain pending until frame publication; failed/cancelled
frames explicitly reject unpublished success. Cancelling an older sequence
cannot cancel a newer one, and cancelling a pending seek realigns the previous
logical cursor. Native source wrappers consume that shared operation; they do
not implement another seek worker or reopen a decoder to fake completion.

The four stored-pixel HLL queries use shared resident ImageInput/ImageResource
access. They read stored channel bytes before display tint/opacity. Native
try-queries leave absent sprites undeclared; DrawGraph.GetAlphaColor lazily
creates the original empty logical sprite, returning0 without writing the output
for an outside/empty coordinate. SP_GetAMapValue returns-1 for an absent,
outside or RGB-only surface. DrawGraph's in-bounds alpha query succeeds with an
output of-1 on RGB-only storage. All referenced RGB outputs are checked before
any write. Foreign scopes and native undefined out-of-bounds RGB GL reads fail
with their original call retained. Shared lookup never implicitly decodes or
materializes a full image from a custom provider; unsupported resident access is
an explicit error. This does not complete DrawGraph text/transforms/effects.

## Reviewed original optional music interface

The exact Rance7 SACT2/MusicSystem module pair is checked through scoped startup-worker reads and SHA256 before admitting Music_Pause, Music_Restart and Music_IsPause. Raw original QIF at10004530 proves that the Pause GUID reaches the unimplemented-interface log and returns null; the SACT facade returns0 when that interface is absent. This policy is not inferred from the public reference warning stubs. Unknown or changed modules receive no profile. Original initialization, client ownership and scalar ABI checks remain mandatory. Ordinary release tests passed6 with1ignored; parent explicitly ran the readonly original profile test1/1, without executing the game, DLL or original engine. Full Sound/Music playback dispatch remains unregistered.

## Shared glyph image preparation prerequisite

`ImageMetadata.begin_glyph_image` now freezes an authorized font and returns an ordinary immutable engine ImageResource through a bounded, cancellable worker. Its regular white-RGB/coverage-alpha kernel is the same TextSystem primitive used by normal engine text, including precomputed coverage faces. It is usable without System4, a window or GPU. Source layout, font weight, CP932/block metrics and native raster math remain semantic policy. In particular, original DrawGraph PMap preserves destination alpha with per-glyph coverage/color; AMap overwrites each glyph rectangle including zero coverage. A union mask for a whole run is not a replacement for that overwrite operation. Native text routing remains unimplemented at this point.

## Delegate source family

All16 DG_* instruction bodies now retain native weak object/function/heap-sequence triples, original collection order/dedup/garbage collection and string-to-method signature matching. Object references are not strengthened by registration or method dispatch. Aliased DG_MINUSA deliberately walks its shrinking live source, including the original skip-after-erase behavior. A shared VM call-frame allocation primitive preserves local-before-owned-argument allocation order. Each original callback receives typed copies, retains caller arguments, preserves reference aliases, resumes ordinary native external waits and returns through original byte PCs; only the last return survives. Final typed argument cleanup runs before returning to the original completion address. No C callback/host engine is invoked.

Release validation passed80 with2ignored across the runtime library, existing VM and7 new authored bytecode tests. Those cover static/bound methods, generation reuse, collections/copies, original self-subtraction order, owned argument isolation/finalization, wait/clone continuation and explicit invalid-call boundaries. This remains an AINv5 checked-signature profile, not whole native HLL/original-save completion. Ordered collection comparison work is bounded to64M.
