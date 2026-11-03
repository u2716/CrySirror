# CatSystem2 native groundwork

Current source checkpoint (2026-11-01): release runtime lib155passed/1ignored.
Original KCS aliases execute into actual engine Image/Canvas/Layer poses and root
completion; SOUND prepares shared audio resources and uses real playback/PCM,
counted loops, status and original-owner-clock fades. Native stop executes actual
KCS SKIP/output with bounded continuation. The latest pColor SOUND blend profile
is compiled, but has not yet had a new original-media acceptance. Paired audio
promotion and other native/CST/input/save/class families remain incomplete.
The last original-media receipt remains KCS20410/FES59, IMAGE+CIMAGE2 shared
leases and all6 real Plane views, paused at baseimg then. Source-only progress
since that receipt is not title, GPU/rendering or gameplay acceptance. See the
latest HANDOFF/STATUS entries for precise compilation and behavioral evidence.

This first-class workspace crate reads original KCS v2 bytecode. It is not a
host for CatSystem2 or another engine. The VM owns only bytecode, numeric
values/addresses, call frames, native pending calls and native configuration.
Archive decryption is the shared `crysirror-formats-arc` `cat-system/int`
plugin. Image decoding uses that shared plugin's HG3 support. The opt-in
`runtime` feature also selects `crysirror-formats-arc`, which remains GPL-3.0-or-later;
the original engine core and renderer are never dependencies.

The engine adapter has a bounded, cancellable source job through
`crysirror-tasks`. Its immutable capability-scoped result is published by the
owner before execution. Factories remain dormant and do no I/O or create
workers. Mapped gcall 393 changes the original production menu item through shared UI
state and acknowledges its void return only after successful submission. The
debug-state query requires a reviewed module ABI and a confirmed absent debug
key. The original task ID and virtual-canvas queries preserve native configuration;
byte-string copy preflights every destination and result slot before mutation.
XML resource loading reserves a shared worker before scoped I/O and CPU parsing.
It publishes immutable `crysirror-resource::xml::XmlDocument` resources before
returning logical native handles. XML integer/text queries retain the original
`document` wrapper and default-value policy. Handles are bounded, client-owned
integers, never native pointers. Unsupported parser encodings/entities and
internal faults remain visible; only an actual `NotFound` returns native zero.
Other native calls fail with their original PC and pending arguments.

Config is `{script, font, heap_bytes, executable?, canvas, random_seeds?}`.
Optional `random_seeds: [float_seed, integer_seed]` carries two exact u32 words
for reproducible native streams. Omitted/null uses one owned shared OS seed job;
the adapter never derives seed words from a clock. The desktop detector may omit
this optional field; both the strict Rust Config and Python desktop-project
validator accept that default. `schemas/catsystem2-config.schema.json` records
the same names and bounds; canonical paths/UTF-8 byte limits remain enforced by
the runtime/tool validators. `parse_startup` preserves the original
`SCRIPT/start`, virtual screen size and configured KCS heap. Installed resource
roots and mounts are selected by the host. A `.kcs` suffix alone is insufficient:
the worker validates the 52-byte native header, original zlib extent, static
string offsets and owned-memory bounds before admitting the program.

The implemented VM subset covers typed byte/word/dword operands, signed/float
arithmetic and comparisons, narrow assignments, prefix/postfix increment, compound
assignment, stack operations, jumps, original call/return frames, address/copy
operations, conversion and pending gcalls. Unmapped debug instructions fail with
their native PC. CatScene line decoding is present as a separate bounded
format parser. Bounded FES integer-expression execution is independently tested;
ordinary and forced FES task execution share the original60Hz clock/wait policy.
Nested FES/KCS tasks, typed object publication and CST command execution remain incomplete.

Reference pins, source hashes, original-module static-analysis hash and exact
IDA functions are in [SOURCE.json](SOURCE.json). No original executable, DLL,
font, script or game content is part of this crate. FelineSystem2 was inspected
as a reference only; its unlicensed code and renderer are not adopted.

Local validation:

```
cargo check --release -p crysirror-catsystem2 --features runtime
cargo test --release -p crysirror-catsystem2
CRYSIRROR_CATSYSTEM2_GAME=/path/to/authorized/data cargo test --release -p crysirror-catsystem2 --features runtime --test owner_game -- --ignored --nocapture
```

The opt-in owner test opens original archives, parses original KCS/CST records
and reports the original configured main script's first pending native call.
It does not establish a rendered title, gameplay, original-save compatibility
or a completed CatSystem2 integration. Local evidence belongs under
`target/verification/catsystem2/`; ignored tests are not counted as passes.


Continuation2026-10-31: native400 imports original LoopPoint metadata through
shared client-scoped AudioLoopCatalog. Its native parser/name policy runs on a
bounded worker; owner publication completes before the original void call is
acknowledged. Source coordinates explicitly retain44.1k reference frames and
open end(-1). The common catalog/checked conversion API is reusable without
CatSystem2 and never changes PlayAsset defaults. Native sound playback remains
unmapped; importing metadata is not audible game support.

Native359 now projects exact5628/72/32/12-byte configuration records from the
reviewed native constructor and startup overlays, then the original gamedef and
client Save/setup.xml sequence. Native resource mode3 prefers configured loose
files before archives. Native789 preserves GAME/flag/max. Shared XML/Storage
own parsing and I/O; only original record layout remains in the adapter.
Diagnostics explicitly state that Windows settings were not imported. Unknown
or malformed XML fails before native acknowledgement; the original permissive
parse-failure fallback and complete settings mutation/persistence are unfinished.
The original interpreter now has executable bounded integer expressions, live
and backup flag banks, parser/command-slot policy and source dirty state.
Native string comparisons, NUL-buffer ABI, memory fill, debug log, numeric
string variables and CSS save-container loading retain original byte/value
policy. Generic byte delimiter splitting lives in `crysirror-text::byte_split`;
CSS compression uses shared Resource transforms and scoped Save I/O.

Reviewed type2 planes own shared transparent Layer groups and Solid payloads.
Native global flat Z is mapped to shared Scene stable floating depth state;
task parents do not become render parents. Original47/48 visibility events
update shared resources with their imperative source semantics. Type6 click
controllers begin genuinely unbound, with no original input registration or
geometry; their later hit registration must use shared UI services.
Sound config938 retains an actual shared XML AST and found predicate; this
representation is not audio playback. Native104/105/848/849 commit real receiver
state into shared InputRouting, which owns priority, coverage, capture, hover,
journal and leased delivery. Native71 preserves the original event-wait yield
and next-owner-tick false result; original keyslot/message callback/result mapping
remains unsupported. Shared explicit Drag is tested, but automatic hardware drag
admission is unfinished.

Type15 image lists begin with the original empty bitmap wrapper and no manager.
Native256 reserves shared workers, parses actual startup page dimensions and
uses shared ImageJobs for the transparent atlas and hidden base-plane resources.
Native260 retains upload XY and262 retains source publication flags; neither
performs rendering. No original compositor or private bitmap storage is hosted.
Historical native100 phase (superseded by the receipt below) executes20,292 original KCS
instructions, then preserves native448 atPC0x1943 with arguments[8].
Native100 overrides/restores the original uniform solid vertexARGB through
shared Layers.setSolidColor, preserving unrelated motion and layer state.
Native447 loads the actual FES source/local bank, and a subsequent71 constructs
its real lowbutton through shared InputRouting. Native117 clears292 allocated
texture pages using sharedImage paged planning and existing engineImageJobs;
all real published pages are checked for zero pixels and client isolation.
The3072x6656 virtual cache is distinct from imageList6's3072x4096 work image.
FES remainsnotrunning at this KCS frontier. A separate actual original FES VM
test executesfour expressions then retains its first platform query; this is
not integrated game execution. Input callbacks/hover, platform status, later
FES objects/waits and original saves remain explicit gaps. No title/gameplay,
GPU pixel parity or complete integration is claimed.

Latest local evidence: native117-lib-tests2.log29passed/1ignored,
native117-canonical-owner.log1/1passed,shared-paged-image-tests.log3/3passed.
The public PagedImage APIs own geometry/clear planning/immutable generations;
Cat retains only original allocation/handle/wait policy. SingleImage64MiB
limits remain unchanged. Full source receipts and next frontier are inHANDOFF.md.

Native100phase final release evidence:lib30passed/1ignored,canonicalowner1/1,
sharedScene solid-color2/2,public Layers.setSolidColor1/1 andPython3.11 tooling
201/201. FinalAPI docs8/8+coverage1/1passed with fingerprintc8c180ba4ff65706.
Native448/FES input prelude remains the next explicit original-PC boundary.


Current continuation (2026-11-01): the latest exact original-owner test passes
1/1 with KCS20,360 instructions and FES53 operations. Original452 is executed;
ordinary FES reaches START_LOGO row4 and retains `execfes brandlogo` at byte
0x973. Its KCS event71 and FES owner continuation remain intact. The test checks
real shared BASE Layer0x4a000008 and MotionStatus/client isolation, actual
bgvoice gain0.8 and settings/input receipts. Frozen batch4 identity and logs
are in HANDOFF.md; this is CPU validation, not a rendered title.

The original MACRO, task/result, section/named value and shared civil-clock
families have release coverage. All39 DEFINE kinds/66 OBJECT slots/repeated
headers/missing trailing cells/KEYBLOCK are parsed. The reviewed flat runtime class
profile now constructs native metadata and publishes real shared resources after
the original script frame. Unsupported profiles still fail before native completion.
Shared objects must be
published. The implicit BASE alone is already an actual engine Layer group;
no-DEFINE all-object waits query its genuine shared motion completion, retaining
the native extra completion yield.

A native motion-program role reuses this same KCS Machine with original
1192-byte external data,256 raw parameters, per-frame entry reset, and separate
frame/active/skip fields. Current release lib78passed/1ignored includes20
scalar/end slots and22 finite matrix/vector ABI mappings using the workspace's
glam math primitives. Numeric alias/singular/nonfinite limits fail visibly.
The remaining3 slots use shared explicit-seed RNG and actual engine logs.
Set-once custom data, raw pose/parameter copies and terminal evaluation retain
original VM semantics. Current FES alias binding executes those Controllers and
publishes actual class poses/root completion; unsupported class/resource profiles
still fail visibly.
Original pBase/pScale/pRot/pDepth/motFade/motFade2/mcFade resources were replayed in
Rust CPU; original motFade skip runs its own completion branch, rather than
receiving a hardcoded alpha endpoint. No private renderer or mixer is embedded.

Shared Scene motion supports position/scale/rotation/pivot XYZ and byte-opacity
for any client, independently of Cat. Three new and five prior release behavior
tests pass. Shared external producer status and affine local matrices now have
independent release tests (Scene75/0, actual public App/Runtime API5/0); they do
not require this VM. Shared API docs8+coverage1pass at actual42a8c3604407b796 fingerprint.
Nested FES/KCS lifetime handling, typed object/input publication, complete CST/
dialogue/native audio/original save support and game title/start remain open.

Current source continuation2026-11-01: bounded nested FES exec now reserves
shared I/O, retains original arguments and immutable input observation, loads
an actual child with the same Main string/interpreter root and its own512-word
bank, publishes shared Layer/InputRouting, and forces its entire subtree before
publishing parent106. Descendants run once across dependency/quota boundaries.
Deferred destruction removes actual engine resources and native banks only
with the atomic owner receipt; protected jobs and unported object owners fail.
Unloaded, paused and deleted tasks do not invent pointer observations.

Actual row/string/macro/word order, strvar literal/format/copy/clear, independent
player string tables, normalized byte strcmp and raw-tail CP932 space handling
are native policy. Shared Text owns cancellable bounded byte-span assembly and
safe explicit C32 printf. Pointer/floating/I64/locale printf profiles and dynamic
constructor/structural-token profiles remain explicit gaps. System4's separate
first-placeholder formatter is unchanged.

Latest release source validation: Cat95passed/1ignored; sharedText42passed/
3ignored, including an independent authored C snprintf oracle. See lifecycle
source audit for precise logs. These source tests do not supersede frozen
owner batch4 KCS20360/FES53, establish rendered title/gameplay, or complete
CatSystem2. Typed39 resources/aliases/motion binding, execkcs/auxiliary
lifetimes, real CST/text/UI/audio playback and original live saves remain.

Additional source and engine capability batch, 2026-11-01: Cat release105passed/
1ignored (`native-tuple-value-family-tests.log`) now includes original sound
attenuation/group/channel selection and lazy tuple-component policies. Sound
channel ranges remain native resource-ID policy; no SOUND playback is fabricated.
Shared Audio owns finite dB/silence-sentinel conversion and public setVoice
controls (Audio36passed/2ignored; actual Engine PCM API3passed). Shared Canvas
owns live isolated composition before outer opacity/transform, independently of
this VM (final Scene93passed/0ignored across10targets; public Engine API1passed).
The same existing Canvas resources refresh immutable snapshots and preserve old
captures; old Present semantics remain intact. These are CPU/resource/API tests,
not rendered game or GPU visual acceptance. Unified public docs are measured
7139e6a8e9650eac with actual no-bless8+coverage1passed. Typed OBJECT publication,
class-specific motion/Layer/Audio binding and actual title/start remain incomplete;
the frozen batch4 game receipt above has not changed.

The complete OBJECT66-field value family, named-array evaluation and KEYBLOCK
metadata now have release111passed/1ignored with runtime feature. This retains
original masks/defaults/partial writes/lazy expression order without executable
pointers. Native empty-bracket uninitialised behavior is explicitly unsupported.
These source policies still require real class resource publication; they do not
remove the typed39 resource gate. See the object-values source audit for exact
validation and failed authored compile attempts.


Current actual class-frame receipt: `cat-owner-live-class-frame` SHA256
`4e9e2b083710ffa6adea60b6ab3dd900596a5fed096b5b4b094408ed42d09ae5`
passes the exact original owner test1/1 in1.11s. The original IMAGE+CIMAGE
obtain2 shared image leases/Canvas draws and all6 PLANE Canvas/Layer views.
The parent execfes completes; flow reaches59operations/KCS20,410. This is
CPU resource acceptance, not title or GPU acceptance. Next original baseimg
at brandlogo START physical79/byte0xad8 remains explicit, preserving owner
KCS PC0x92c and its FES owner continuation. Native aliases/motion/SOUND,
remaining classes/input/CST/save continue as incomplete source families.
