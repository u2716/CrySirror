# CatSystem2 implementation notes

Current implementation entry points and supported profiles are indexed in
[README.md](README.md). Native operation mappings and remaining gaps belong
beside their source; source provenance is recorded in SOURCE.json.
The dated notes below retain historical implementation and validation evidence.

## Implemented semantic boundaries

- Original KCS v2 first-class bounded interpreter, typed stack/value/address,
  call/return/native pending frames; unknown opcodes remain at sourcePC.
-393 retained engine MENU104 item119 disable;465 reviewed absent debug flag;
  native taskID68/virtualcanvas8; byte string operations13-22,36,41-42; shared
  XML590-593 handles, original transparent wrapper/default/close semantics.
-400 real17-entry BGM LoopPoint import, shared client AudioLoopCatalog; original
 44100reference frame and duplicate-start-only update policy. Playback unmapped.
-359/789 exact original constructor/startup/gamedef/clientSave setup overlays.
  Mode3 loose-before-archive matters. Real flag count2048. No Windows settings
  import; malformed XML fails rather than claiming permissive source fallback.
-209 live/backup flag banks/parser/256 command slots and original integer
  expression behavior210-213. Backup remains uninitialized until source copy.
  Unsupported tokens/types fail; command execution beyond reviewed subset partial.
- Shared Text byte_split preserves native SJIS lead/trail boundaries, delimiter
  collapse, NUL output/capacity/cancel; VM41 only maps owned buffer ABI.
- Type2 Plane, type6 click controller, type15 image-list constructors only.
  Plane79 real Solid payload;88 native global flat floatZ maps shared Scene
  depth state on payload only,0<=Z<1e6; capture upperbound65536 unsupported.
 92 imperative47/48 child messages, not permanent inherited visibility.
-938 genuine shared soundconf XML AST/found(5groups/6parts), not an audio mixer.
-247 scoped Save CSS header/tag/records + sharedzlib; isolated owner save absent
  genuinely returns0. Present-save apply/write/version behavior unimplemented.
-276/277 original numeric raw-byte string variables/dirty;278$str unmapped.
-104/105/848/849 actual shared InputRouting bind/enable/hover/drag flags.
  Full RECT8CDAD0=[-100000,-100000,100000,100000], not empty/global sentinel;
  event_mode5 is distinct from fixed input source bits7. Bind targetmain3 is
  distinct from INPUT service1. No fake focused keyboard Button.
-71/65A180→877390 event wait yields after validating16byte caller slot; actual
  Main tick5E84E0→877450→8762A0 next owner step pushes0, clears original flag,
  leaves slot untouched.5E8270 is destructor. Atomic Machine.resume_native_event
  supports tested message copy+true result, but adapter callbacks/keyslot/result
  mapping remains unsupported; do not wire synthetic key bits as native slots.
-256 actual first image-list initialization uses shared ImageJobs to allocate
  zero32bpp atlas/base, then owner publishes actual engine images/layers before
  native ACK. `68E6F0` is empty bitmap wrapper, not a directory scanner;
  `686E50` is two RECTs(page,dirty), not packing. `742660` tile dimensions derive
  startup TEXTURE size via offsets with1048byte native manager header. Source
  requires page definitions; first init only, reinit fails. Scratch remains empty.
-260/743E10 stores original uploadXY(task112/116,manager4/5), no rendering.
  Uninitialized coords are None until setter.262/msg286 writes124/120=value,
  nonzero sets140=1, zero leaves140 unchanged. Neither composes/shows the image.
  Native image entries/composition/dirty uploads/capture/resource replacement,
  auxiliary image/timer/sound tasks remain unsupported. Never copy743600 raster
  compositor into Cat; use reusable shared Image/Canvas/Layer operations.

## Shared engine work and ownership

Shared stable float depth: Scene layer/depth_order.rs canonical side-map,
LayerCommand::SetDepth{id,depth:Option<f32>}, ordinary before depth within major,
pure frame calculation plus accepted owner commit_frame_order; snapshots freeze
effective order. Runtime hook and public Layers.setDepth were added by root.
No automatic task parentZ inheritance for Cat flat native draw order.

Other shared extraction: Text byte_split and Resource zlib per-chunk/prepublish
cancellation. Do not move these algorithms back into this VM.

## Reviewed class34/FES and current native448

Class34 complete constructor/attach/firsttick vtable8D2A90 and handler62BED0 are
in ignored static evidence. Constructor125 before attach cannot reach INPUT:
parent0/emptychildren,self125unhandled; savedcursor remainsUnknown.449 binds
true imageList bitmap dimensions/uploadXY. Native447331/332 creates real local
bank and loads FES. It does not execute the player. Subsequent71 legitimately
constructs FES lowbutton9 via shared InputRouting even while playerloaded;
629850's!running branch does only758640(debugFPSdisabled0).

Standalone FES original VM is independently validated:4flag assignments then
steam_isenable1002 atSTARTrow5/physical33/byte0x790. This is not integrated KCS
FES execution. Platform manager status is not implemented; do not forcefalse.
No OBJECT/MACRO/KEYBLOCK/DEFINE/plaintext preprocess/string substitutions are
admitted. SUBCOMMAND AST is retained, not executable alias effects. Unknown
builtins retain source location; nonbuiltins use original integer evaluator.
Blank waits/section restart/budget are distinct, never converted to caller ACK.

Completed100 source chain:640D30→Plane59→676BB0→67D7C0→6840C0; triangle
constructor682370 defaults overrideenable0/colorFFFFFFFF.6824D0 output uses
stored original vertexARGB when enable0 and fields348/352's exact override when
nonzero. Source enabled/color are raw native fields. ActualPlane4 is79's uniform
solid two triangles. It uses shared SetSolidColor to retain all motion/spec
without replacement. BaseARGB is retained so disabling restores it;79 geometry
replacement resets original override fields. The public Layers.setSolidColor
facade/COMMAND_API entry are connected. NonSolid/foreign/missing targets fail;
shared batch rollback preserves previous color and all motion. Current100 is
mapped; actual native448 remains unacknowledged and its input prelude is the
next required engine capability/semantic boundary. Do not expand this scope
before Root29 is fixed.

Source receipts: docs/catsystem2-fes-source-audit.json and
catsystem2-texture-cache-source-audit.json. Raw owner source/IDA and game bytes
remain only in ignored verification paths.

## Validation ledger

- Cat lib release23/23: native66-fes-tests.log.
- Actual original adapter release1/1: native66-fes-owner.log;earlierbootstrap-owner.log1/1.
- Actual canonical Runtime release1/1: native449-canonical-runtime-owner.log. All31INTs
  mountedinoneRuntime,sourcepublication0instructions/frame/time,then20,227PC1926.
  PublicAssets.surface resolvesactualatlasandforeignclientfails;canonicalLayer
  basehidden/1280x720/Z1000;Runtime.tick fourmouse/Ctrledgesretaincallbackerror,
  pending/instructionsandsameleasedreceipt. NohelperRuntimeusedinthisnewtest.
- Unified24actualheadless exits1atthesameclass34PC18eb beforefinalcapture;
  player24-original.log. NoPNGorGPU-pixel/titleclaim.
- KCS VM release12/12: native71-vm-tests.log (new atomic delayed event resume).
- Cat runtime-feature releasecheck: native256-check.log.
- Shared router release7/7: shared-input-routing-drag-tests.log.
- Engine lifecycle isolation release3/3: shared-input-routing-lifecycle-tests.log.
- Shared inflate cancellation release2/2: shared-inflate-cancel-tests.log.
- Shared depth4/4 and byte split3/3 previously passed; see STATUS.
- Python3.11 tooling201/201: python-tooling-311.log. Default system Python
  lackedtomllib; initial6importfailures retainedseparately,notcountedaspass.
- Root/NNN generated API docs8/8, fingerprint bd71afbaa37a19da; independently
  recorded. No new engine facade is required for256/260/262 native state.

31 original INT mounted namespaces were previously validated, including26
scene.int duplicate CSTL rows: scoped INT aliases are accepted only when entire
stored payloads agree with known same archive key. Preserve actual ArchiveMount
coverage, not only raw index parsing. No user game bytes were added to the repo.

## Validation — 2026-11-01

- native117-production-check.log: release check passed1m31s.
- native117-lib-tests2.log:29passed/1owner ignored. The initial authored storage
  fixture lacked remove/list; native117-lib-tests.log records that compile failure,
  corrected before this final pass.
- native117-canonical-owner.log: actual Runtime1/1passed0.42s,20,285/native100,
  all292shared pixels and foreign scope verified, four input admissions above.
- shared-paged-image-tests.log:3/3passed, including original page budget, holes/
  overlap/clipping/cancel/undefined partial pages, foreign/size/duplicate/missing/
  stale completions, frozen clones/dirty generations. PerImage64MiB unchanged.
- native447-result-lib-tests2.log:28passed/1ignored, worker result/failure caching
  and player admission. native447-canonical-owner5.log1/1passed at117/0xb4.
- fes-flow-source-owner.log: standalone actual4expressions→Steam1/1, not integrated.
- Shared Text grouped splitter3/3passed. Final merged API docs by NNN8/8+coverage1/1,
  fingerprint1171d6b86d446b00, includesText+Paged Rust-only APIs. No fake facade.
- Python3.11 tooling201/201earlier; no Python tooling changed in this increment.

## Native100/SetSolidColor latest validation

- native100-production-check.log passed5.43s before switching the final call from
  replacement to the newly authorized dedicated public command.
- native100-lib-tests.log30passed/1ignored; same final command path compiled.
- shared-solid-color-tests2.log2/2passed: preserved spec/motion/depth and rejected
  foreign/missing/nonSolid with batch rollback. Initial authored rotation spelling
  error is recorded in shared-solid-color-tests.log, not counted as a pass.
- native100-canonical-owner.log1/1passed0.40s after6m19queue/link, actual20,292/448,
  true shared black solid and all292page pixel/scope checks, four retainedinput
  admissions. FES0expressions/runningfalse remain explicit.
- Reflected API test fixed only authored mutable-App helper callsites, pending
  final shared-solid-color-api-tests3.log passed1/1. Production API is frozen.
  NNN solid-color-api-docs-verify.log passed8/8+coverage1/1, fingerprint
  c8c180ba4ff65706. README/CHANGES/JSON/TypeScript are updated.

## FES input/platform/config continuation — 2026-11-01

Latest **validated behavior** is canonical Runtime original KCS20292/native448
PC0x1943 retained, original flow.fes47 instructions, INIT_GLOBAL row37 physical
267 byte0x131a pending `setconf 0 stillread`. Actual mounted scene.int/H01.cst
metadata set flag1003=1. Steam manager remains absent (native null-manager
query0), not forced global platform success. New shared input observation is
available in the explicit owner test and retains the real virtual [0,0] pointer.
Root exact owner test passed1/1 (`cat-owner-exact.log`); earlier short-name
`--exact` selected0 tests and is not a pass. Old frozen phase1 binary exited101
only because its stale assertion required0 FES instructions; its47-instruction
receipt is behavior evidence, not a passing test.

Root combined release650 passed/43 ignored included Cat lib37 passed/1 ignored,
Runtime input1/1, Runtime platform1/1, Engine reflected input/platform2/2. Shared
InputObservation/Platform APIs and desktop C/L/Z/middle mapping remain frozen;
NNN owns unified docs generation. HostServices defaults fail Unsupported. Unknown
pointer/focus metadata is not an invented coordinate or key state.

Static source inventory `target/verification/catsystem2/fes-complete-static-inventory.json`
records **131/131 FES native functions extracted**, not131 implemented commands.
Actual getsndconfex address is783090;783490 is inside its body. Original FES
endfes77A7C0/endkcs77AAA0 each return1 even with no child, therefore yield a
real one-frame boundary; do not silently continue the same tick. Execfes77A630
queues event2/16 arguments and returns-3;629850 synchronously forces new child
333 before retaining it. Original task children tick before parent script.

### CatSystem2 native FES clock/task phase — validated source batch (2026-11-01)

Implementation: forced native448 and normal owner FES tasks now use one shared
service dispatcher. Original60Hz simulation clock, numeric/frame/blank/section
waits, keyskip/reset_frame and no-child endfes/endkcs one-frame boundaries are
preserved. Resource continuations retain their source input revision and clock;
ordinary tasks run once per admitted revision, child-before-parent, ahead of
Main continuation. Empty-object fallback hover follows the actual native clear-
selection/pointer observation branch and returns unhandled. This does not claim
physical/object callback, nested child creation/destruction or typed resource
execution; those still fail visibly. Huge diagnostics JSON was split into two
identical-schema maps after an actual macro recursion compilation failure.

Behavior: updated real-game owner test (native stillread0, shared bgvoice
gain0.8 and foreign-client rejection) awaits explicit execution by Root. The
previous47-FES/setconf frontier remains the latest verified real-game result
until that run; no newer title/gameplay/complete-integration result is inferred
from unit tests.

Next source is DEFINE/OBJECT/MACRO and nested task/resource publication.
Original typed declaration tableB071A8 contains39 classes; OBJECT header table
B07748 contains35 named groups covering66 numeric fields; motion/alias field
tableB07A18 is separate. These are VM policy metadata, not a new renderer.

### CatSystem2 actual shared sound and FES task wait acceptance (2026-11-01)

Actual authorized game-data behavior: Root froze
`target/verification/native-source-batch/cat-owner-batch3`
SHA25636b1f9e80dd759e4a7da7e2c338f3ddf8d84087cffc11ab9b37f6d0e5572bed3,
then ran the exact owner test:1 passed/0 ignored in1.01s. Original KCS executes
20320 instructions and retains native452 atPC0xa71,args[8],next0xa7c. Original
flow.fes executes52 operations and remains running in START_LOGO,row3,
clock414/frame3 with actual Until415 wait. Original448 completed at its genuine
FES wait boundary; normal child phase continued across the subsequent frame.

The actual test observes native stillread0 and dirty preferences; real engine
audio group catsystem2/sound/part/4 has authored gain0.8 under shared master;
foreign client999 cannot read it. No audio device or original engine/EXE was
used. The1.01s duration is CPU verification time, not a rendered game benchmark.
No title, logo playback, interactive gameplay or full compatibility claim.

Official exit reference confirms exit only publishes the exit code; child keeps
running until parent endfes. Do not halt the VM or close the application on it:
https://cs2.suki.jp/manual/fes/reference/cn/exit.html

## Continuation: result, macro/declaration and civil/section families

Implementation: native452/453 now use the actual base-task20 result slot.
FES exit writes task20 and continues the same frame; result writes player519.
Native450/451 preserve original335/336 section/named-value messages, local
bank ownership, existing frame base/wait and pending operation ownership.
FES MACRO now executes the original one-pass case-sensitive token policy;
raw numbers/literals/SJIS pairs and original PC are retained. DEFINE39 types,
OBJECT66 fields/repeated headers/missing tails and KEYBLOCK AST are production
linked. A nonempty declaration plan still fails before bank allocation: no
shared resource/object implementation or successful game load is fabricated.

The full gettime/getweek pair now delegates to shared CivilTimeQuery and
Gregorian weekday. GetLocalTime samples once before evaluating destination
flags; retries retain exact owner/nativePC/operation and Ready/Failed receipt.
Simulation clock zero never becomes local civil time. Foreign/invalid receipts
fail persistently. NNN owns the shared calendar helper; no private calendar
algorithm was added. Static original6A7450 matched360 legal date samples.

Compilation/independent behavior: release lib49passed/1ignored for result
family;56/1 for macros/declarations; latest59/1 for civil/section family.
Logs: native-task-result-lib-tests.log, native-declaration-macro-tests.log,
native-civil-section-tests.log. No new GPU run or original-game run was made.
Latest actual owner receipt remains frozen cat-owner-batch3 at KCS20320/FES52
/native452PC0xa71; those tests and immutable player33 predate newer source.

Static object projection helpers767A80..768F00 and original base/image/plane/
button/sound constructors were additionally exported to ignored target; they
are source evidence for completing shared object publication, not handlers.
Next: finish real typed resource publication, child execfes/execkcs lifetimes,
object/input/motion/audio bindings and CST/save execution as whole families.
Never turn declarations or a native frontier into a completed integration.

## Current batch4 actual receipt and original motion role

Compilation: runtime lib60passed/1ignored for shared object wait; latest
motion role64passed/1ignored (native-motion-role-lib-tests.log). Shared
Scene motion3new+5prior tests pass,0ignored. NNN unified API no-bless8/8
+coverage1/1 passed; actual fingerprint1cae8f9410b1b5b9.

Actual original game: frozen `cat-owner-batch4`, SHA256
c5f41f86d362361c30b6f4a209b283c5d80d19a74ac1efca3f59a0612f41f5f1,
exact owner test1/1passed in1.05s. KCS20360/FES53, ordinary START_LOGO row4,
physical66 byte0x973 pending `execfes brandlogo`. KCS71 event@0x921 and
ordinary owner phase origin0x92c/revision439 are retained. This is not a title.
Logs/receipt/summary: native-owner-batch4*. The test checks real shared BASE
Layer0x4a000008, MotionStatus/client isolation, bgvoice gain0.8 and settings/
input observations. It contains the60-test source point, not newer Controller.

The implicit754310 BASE _TOPOBJ_ now owns a genuine transparent engine Layer
group. In the admitted no-DEFINE profile, all-object wait queries this shared
LayerMotion and only clears its native-1 wait after actual completion, still
yielding that call. Native skip is NOT EndMotion:748610->789870->78A5C0 sets
a program flag; until an original motion Controller is actually bound to the
object, busy source skip explicitly fails and leaves engine tracks active.
Original animation updates occur at the end of the FES phase; future native
motion bindings must drive external-time engine tracks in that order.

New public native `motion::Controller` reuses the same KCS Machine. Motion
programs have a1192-byte external block; frame144,active148,skip152,param156,
256 raw args.78A4E0/877270 reset PC/stack/frame per evaluation, preserving
external/parameter bytes.20 scalar/end native slots(1..17/38/41/42) are handled;
matrices/RNG/debug retain exact original native PC/pending state. Setting skip
only writes original152; actual resource bytecode decides endpoint/active.
This native data VM is not yet wired to FES object aliases or scene poses.

Static whole families:122 object-command entries/C headers and45 motion
entries were exported;72 motion KCS resources were inspected read-only in
ignored private/motion. Pure Rust current KCS+Controller replay succeeded
for original pBase/pScale/pDepth/motFade/motFade2/mcFade sampled frames; pRot
retains native26PC0x12. Original motFade skip executes fromARGB00ffffff to
ffffffff and active1->0 by its own code. No original EXE/DLL is executed.
The direct replay compiler originally selected mixed build-dependency/LTO
artifacts; failed attempts were not behavior passes. Final compile uses actual
owner Cargo artifact profile opt3 dependencies and thinLTO, then exits0.
Source replay log motion-vm-static-replay.log; no rendered-motion claim.

Continue complete nested FES/KCS loading/lifetimes + typed39-object shared
publication, remaining motion math/source policy, input callbacks, CST/text/
audio and original saves. Do not relabel current execfes boundary as done.

## Current motion45 source family (after batch4)

Implementation: motion::Controller now maps the reviewed45 native slots.
The22 geometric slots preserve row-vector/native matrix order, opposite vector
XYZ rotation signs, vectorW, exact aliases and actual scalar-pointer arguments
through sharedglam0.27. Unsupported partial overlaps/singular/nonfinite profiles
remain explicit before writes/ACK. Native24 is translation and25 is scale,
verified against PE scalar-dispatch tableB32F30 rather than inferred names.

Native39/40 borrow client-owned explicit-seed independent float/integer streams
using sharedResource::legacy_stream::Mt19937Split. Native float division is
f64 by4294967295 thenf32, including endpoints. Result-stack preflight precedes
RNG draws. No game seed/default entropy has been invented. Native0 uses actual
engine committed LogReceipt and shared explicitSJIS codec; foreign/failed log
receipts remain failed without duplicate publication on polls. Controller owner
cannot be rebound. These mappings do not bind a procedural motion to a Layer.

Reusable kernels: Resource Split initialization shares twist/temper with the
existing1998 generator. formats-arc e.rs uses fast stateless first_u32 rather
than a duplicated MT implementation; no full624-word allocation per record.
Shared explicitSJIS avoids the dual-valid UTF8 guessing ambiguity. Existing
1998 vectors and archive formats pass their regression tests unchanged.

Compilation and independent behavior: combined release command exit0:
Cat72passed/1ignored; formats-arc44/1; Resource33/3. Log:
native-motion-services-lib-tests2.log. First attempt failed on a test accessing
private NativeCall state; replaced with authored16-byte VM stack exhaustion,
then all tests passed. Actual Runtime LogBook/client receipt tests pass without
a GPU. Split vectors use independent pinned libsys4 C twist/temper oracle at
seeds0/1/4357/deadbeef and positions0/1/2/623/624/1247/1248, plus cursorClone.
No ignored owner case is counted. Public Resource helper docs regeneration is
with NNN; last verified1cae8f9410b1b5b9 must not be claimed as a new fingerprint.

Pure Rust production VM+math replay now succeeds on seven original resources,
including pRot (prior scalar-only native26 frontier superseded). Current log
motion-matrix-vm-static-replay.log. Actual owner game remains frozen batch4:
KCS20360/FES53 pending execfes brandlogo, not title/render/gameplay. No repeat
actual game run was used to claim this motion family complete.

Next whole-source batch: nestedFES/KCS lifetime. Read77A630,629850 event2/3/4/5/6,
62BED0/331..340,627E50,7559B0 and728870. Original exit updates task20 without
destroying the child. Forced child firsttick precedes publishing parent106;
child-before-parent ordering and descendant lowbutton priorities must survive
async quotas. String bank177/273 is not interpreter176/109; verify constructor
attach/firsttick timing before assuming either is inherited at331. Typed39
objects and native CST/audio/save remain incomplete; do not remove their gates.

## Current shared procedural/matrix and motion lifecycle checkpoint

Implementation: native custom-data set/read now uses separately owned positive
KCS allocations after fixed globals/stack; crossing allocations fails safely,
Clone copies actual data, and an allocation never expands the result stack.
Original pose/parameter/delay methods and78A540 terminal evaluation are ported:
frameFFFFFFFF/skip1 execute real bytecode, then restore only frame/active/skip.
Output pose/params/custom data persist. Terminal and regular budget ownership
are distinct; original inactive state is not fabricated as running.

Actual reusable Scene capability: Layer begin/commit/request_skip/remove_external_
motion and set_rect, plus ExternalMotionPose/Stamp/Status. Generation/revision
receipts use exactu64 strings; stale/foreign samples atomically roll back pose
and status. LayerMotion.complete combines real external producers and scalar
tracks. A skip request is not a fabricated endpoint. No executable code/VM is
stored. Retire producer capabilities before scene-state snapshots; snapshot/
restore of an external continuation explicitly fails. Put/Remove retire stale
producers, while normal transform/rect/opacity updates retain them.

Shared SpriteTransform.local_matrix is finite nonsingular affine4x4 in column
major order, after local scale/YPR before anchor. Shear/planar2D parent chains
and3D leaf sprite/model projection use the same engine path; projective/singular
matrices, matrixbillboards and3D Layer ancestry fail visibly (World hierarchy
is required). Canvas bounds/pixel-query/frozen2D paths respect the new field.
Public Layers five new methods and existing setTransform/motionStatus expose
this without Cat; the API test uses actual App/Runtime and projected geometry.

Validation: latest Cat78passed/1ignored after nested-root policy. Earlier
motion terminal76/1 and KCSintegration12+preparation1passed. SharedScene75/0
(lib50,snapshot9,external5,matrix3,visual3,UI5); public EngineAPI external1,
solid-color1 plus Engine lib3 pass in shared-procedural-public-api-tests.log.
Resource33/3+formats-arc44/1 remain passed. Python201 OK(1skip), bundledPython,
native-source-python-tests3.log. Earlier incompatible Python3.9/tomllib and
isolatedCatvenv/numpy attempts were failures, not full-suite passes. Scope diff
check passes. NNN final no-blessAPI8+coverage1passed; fingerprint is measured
42a8c3604407b796, which also includes sibling Alice CopyAlpha changes.

Latest private source:62DAA0 at FES vtable+4 resolves parent177 during ATTACH,
BEFORE331. So nested raw-string owner remains Main3, not parentFES local bank.
Nested constructor/load, independent512 locals, raw340arguments and ancestry
priority2/3/postorder are covered by78/1. Source5DC510 marks3 then destructs at
next owner child phase;5DC790 cascades child3/dtors.6280C0 sets parent image-list
mode286=1, marks lowbutton3, detaches Interpreter through7576F0/728870 before
7559B0, cleans objects and delegates base destructor.77A7C0/77AAA0 themselves
DO reset parent_EXITCODE_=-1; do not infer otherwise from629850's event3 body.
Remaining: complete execfes/execkcs retained worker+forced-firsttick+deferred
retirement, typed39 shared object/alias binding, real CST/audio/save. No new
actual game run: batch4 remains KCS20360/FES53 pending execfes brandlogo.

### Whole nested FES and string source batch2026-11-01

Latest actual release lib95passed/1ignored in
`target/verification/catsystem2/native-string-table-family-tests2.log`.
Earlier table-family test compile failed on an authored raw-field insertion,
corrected before that successful run; it is not a pass. SharedText42passed/
3ignored in `shared-byte-text-kernels-tests.log`; new byte_replace/printf32
mechanisms are reusable without Cat and leave format_one_bytes unchanged.
C32 authored C snprintf oracle is `shared-printf32-c-oracle.log`.

Private new modules fes_children and task_policy/retirement now cover bounded
plain-FES child loading, argument-once admission, immutable input receipts,
actual forced subtree phase/cursor, parent106/result timing, deferred destroy
and atomic real Layer/InputRouting removal. Native raw strings share Main3;
each player keeps its own512-word bank and table518. Original73FF40 expands
whole row, macros then per-word. Pending raw row is retained for785670's
second-ASCII-space remainder. Original77C6D0's unused first expression and
second-argument lookup+destination are confirmed in x86; do not normalize it
into a nicer guessed API. Empty copy/clear and zero-vararg literal strings
retain original C-string semantics. Unsafe vararg profiles fail pending.

### Native multi-Controller manager source, same checkpoint

Private `motion_set.rs` now follows788AF0..789B40's whole bounded orchestration
family over the same Controller/KCS: base/output copies, output initially
unknown, ordered multi-controller pose seeding, start0 sentinel, delayed and
rounded scaled clock, group/id mutation, actual activity versus retained-track
status, skip flags, terminal evaluation and explicit output-to-base commit.
Quota continuation keeps the admitted owner/clock/mode/index; it cannot replay
prior controllers or reseed a partial original entry. Used resources remain
immutable KCS programs; actual logs/RNG use scoped engine services.

Release100passed/1ignored in `native-motion-set-owner-tests.log`; prior99/1 in
`native-motion-set-family-tests.log`. The fifth authored test uses actual
Runtime log receipts, deliberately pauses after the first append, rejects a
foreign owner, then confirms original ordered once-only resumption and terminal
logging. This does not connect motion to live FES objects or render a pose.
No snapshot/native motion-save restore/cache is fabricated. Resource-name
comparison is explicitly boundedASCII, scaled-clock conversion finiteDWORD.

PublicText docs finished with measured `ceb843236c48cb69` (previous42a8 kept),
NNN actual no-blessAPI8+coverage1passed in
`target/verification/systemnnn/api-byte-text-kernels-verify.log`. Private manager
adds no public API shape. Full Python201OK was independently run for strings.

Static next: original class pose source does NOT permit one guessed generic
mapping.749330/749420 are CIMAGE seed/apply;74AC30 is CPLANE seed,74AD40 is the
separate output function (single-CC entry, old padding heuristic missed it).
CPLANE seed takes native priority184 into poseword5, but apply writes that word
to position.z72 without changing priority184. CIMAGE applies only the high
ARGB byte as alpha, while Sound74C590 uses that byte as volume and word6 as its
other native scalar. Inspect all class-owned fields before shared Layer/Audio
publication; do not route every pose as ordinary opacity/translation. Source
exports are ignored local `.c` files, never adopted native renderers. All IDA
processes closed. No latest game/GPU/title claim; batch4 remains unchanged.

### Shared audio/composition and native policy continuation, 2026-11-01

Actual release Cat105passed/1ignored in native-tuple-value-family-tests.log.
Prior sound policy suites101/102/103 each passed with1ignored; later105 includes
them. Original5D2200 is nonlinear wrapping percent-to-device attenuation, e.g.
50 gives-1000dB100 (~0.31622776 amplitude), not linear0.5. C oracle is authored
sound-attenuation-oracle.c, never original native code linked/executed. SOUNDGROUP
selects actual channel ranges, not bus ancestry; pure SoundSlotPlan prefers
unloaded, releasing, auto-release then minimum. Resource retirement/load and
class dirty-state publication are still required before any native completion.

Audio::levels independently36passed/2ignored, public Engine Audio native3passed
with actual PCM and pre-mutation validation. Ordinary finite-100dB is distinct
from explicit legacy sentinel silence. Voice fades remain linear amplitude.
Canvas PresentIsolated is engine-owned live snapshot composition, preserving
outer scope/generation/motion/COW/clip and old Present. Final Scene10targets
93passed/0ignored (shared-canvas-isolation-final-tests.log), actual public Canvas
API1passed and Audio3passed (shared-canvas-audio-final-api-tests.log). No GPU
visual claim. NNN unified docs actual7139e6a8e9650eac, independent no-bless8+1pass.

Original759680 tuples use literal comma substring search, not CSV/nesting;
skipped components never evaluate. Native prefix parsing shared within adapter.
Current whole759E50 OBJECT-value projection is being separately verified, not
admitted as engine resources. The real game checkpoint is still batch4, not a
new title/scene run. Do not remove typed39 gate until class resources and real
native lifetimes can be published/retired through shared engine APIs.

### Whole OBJECT value/reference/KEYBLOCK source family, 2026-11-01

Actual release --lib107passed/0ignored, then --features runtime --lib111passed/
1ignored (`native-object-field-family-tests.log`,
`native-object-reference-keyblock-tests2.log`). The earlier111-family compile
attempt failed on an authored Rust cast and nonexistent test-set helper; fixed
before passing, never counted. Six new tests cover the whole66-code field ABI,
expression order/rollback, reference cases and native KEYBLOCK table behavior.

New fes_object_values retains original759E50's896-byte zeroed row,three mask
words,BLEND255,individual POS flags,partial tuples and index-1 PRI addition flag.
Parent slot60 is never an original pointer: a separate optional logical identity
is retained. Indexed fields still evaluate their index even where native copies
only the name. Fields zero-keys still execute reference-index expression first.

fes_references implements758E00/758D40 raw name/array syntax and actual Main
string substitution before real count lookup. Original754E80/754F90/729B90 uses
case-sensitive bytes; do not make it case-insensitive. Index-1 becomes0 only
when actual count<=1. Empty brackets leave original scratch uninitialised and
are an explicit unsupported profile. New static export log records two attempted
entry mismatches, not valid functions; all IDA DBs closed without saving.

fes_key_blocks follows7597B0 sixDWORD defaults0,0,-1,-1,-1,-1. Partial duplicate
rows preserve untouched values; missing second tuple component prevents reading
third token; out-of-range index never evaluates value tails. This native data
is not an InputRouting registration. Typed39 resource publication remains
explicitly gated and these helpers do not create a running game object. Detailed
contract/limits: docs/catsystem2-object-values-source-audit.json.

### Class pose and native2Dpan/shared stereo-gain batch, 2026-11-01

Production source is stable. Latest release runtime lib117passed/1ignored in
native-pan-stereo-host-tests3.log. Prior finalclass115/1 independently covers
signed-zero behavior; object/reference/keyblock111/1 is preserved history.
The new native HostPCM test first failed to compile from a missing test borrow,
then ran116pass/1fail/1ignored from an authored projectassets dot/empty path.
Both mistakes fixed before actual117/1; no failed result counted as pass.

native_object_pose.rs implements all reviewed CIMAGE/CPLANE/SOUND seed/apply
callbacks as original class state only. CIMAGE6/7 is position,3/4pivot, NOT UV;
its matrix/RGB startszero and onlyalpha is seeded/applied. Storedpriority is
biased+20, but original compare uses unbiased output before deciding to write;
nonzero-to-nonzero visibility still increments native count. CPLANE constructor
vectorW is1 (real8D54A0/8D5690); callback setsW0 and preserves native float equal
signedzero cases. Seed5 ispriority, apply5 ispositionZ; priority remains unchanged.
Raw fullmatrix is copied, Euler onlyrecorded. SOUND6 isintegerpan, highbyte is
lowvolume byte; x86CVTTSS2SI indefinite DWORD retained. No shared publish invented.

Read-only original5D1F60+5D23B0 prove2Dpan attenuates onlyonechannel via its native
percent/db curve. 3Dbranch issourceposition; do not emulate it with ordinarypan.
New shared Core AudioRequest::SetStereoGain and Mixer set_stereo_gain/stereo_gain
provide independent output amplitudes, preserving existing equal-powerpan. Gain
runs afterspatial/pan,beforeMaster/reverb sends. Same rendered voice-frame ramps,
paused/dependencyheld values and exclusiveworker observations/publication apply.
Public Audio.setVoice exposes stereo_gain[2]0..4, validates before any mutations.
Actual sharedAudio8targets78passed/2ignored (shared-stereo-gain-tests.log), actual
EngineAudio4/0 (shared-stereo-gain-public-api-tests.log). Cat immediate2Dpolicy
calls public Audio levels and returns that typed request; RuntimeHost PCM test
proves it reaches commonmixer. No native191owner-tickfade approximation was added.

Unified docs measured5ba5be1c88e5f542, independent no-bless8+coverage1passed
(systemnnn/api-stereo-gain-verify.log); prior7139 preserved. AudioRequest is a
real Rust-only capability enum, not a fabricated JSON operation/reflectionclass.
See docs/catsystem2-native-pose-audio-source-audit.json for contracts and limits.

NEXT whole source stage: actual755D10 constructor order ->7597B0 KEYBLOCK ->
7589E0 DEFINE ->758F70 OBJECT ->75AAC0 aliases, then6287B0 native object cache
and62D4A0 class publication dispatch. Real publication functions now exported:
65C820 CIMAGE,65DC00 IMAGE,65CCB0 CPLANE,661430 PLANE,662640 SOUND. Whole original
CIMAGE class export inventory and native visual publication sources are ignored
local files, not adopted renderers. Read all65DC00 (20KB) before mapping its
resource/profile branches. SoundPaths in loop_policy.rs already resolves native
startup/SOUNDCONFIG/int/prefix names; reuse it with actual shared worker resources,
not hardcoded archive names. Old6285C0 is genuinelysoundpoll;62C820 is not a
function start, so earlier failed decompile is not missing class implementation.
Typed39 gate stays until resources/waits/lifetimes have actual engine receipts.
No newgame/Desktop/GPU/title receipt. All IDA databases closed withoutsaving.

### Object-dispatch boundary final source receipt

Release runtime lib118passed/1ignored (native-object-dispatch-boundary-tests.log).
Original7690E0 first resolves object names before7692C0's general native or
expression route. Existing implicit BASE and future declared-object calls now
retain PendingKind::Object with original row/args, before argument expression
side effects; they cannot become defaultzero expressions or receive a general
native ACK. Runtime unsupported path retains the exact original operation.
Authored test repeats polls, checks no increment/tail instruction execution and
rejects wrong completion. This is a boundary correction, not a new object handler
or permission to remove resource gate. All prior117/1 actual HostPCM coverage
remains in this full suite. Final shared API remains5ba5be1c88e5f542, no shape edit.

New source/standalone engine paths were validated; actual game checkpoint stays
batch4. Future full player/real game validation remains Root's builder ownership.
No actual game/source scan count is substituted for native object publication.

Final bookkeeping receipt: HomebrewPython3.11 full tooling201/201 OK,0skip,
2.440s (native-object-dispatch-python-final.log); git diff --check passed. No own
Cargo/IDA/GPU/game process or queued validation remains. All native EXE reads
were static; ignored local exports/compiled authored oracles are not distributed.
Renderer source inspection confirms existing shared Paint::Snapshot takes an
ordinary offscreen texture path; renderer was neither changed nor GPU-tested by
this agent. Canvas93CPU/public1 remains exactly the recorded validation scope.

## 2026-11-01 resumed HG3 resource preparation family

Shared `formats-arc` HG3 now lives in `legacy/e/images/hg3.rs`: real relative
tag links, original frame IDs, condensed/canvas geometry, signed origin/base,
raw flags/mode/cptype/ATS metadata. 24/32-bit padded controls and shared JPEG plus
alpha work; alpha-only/WebP/segmented/mixed profiles remain explicit partial.
Format option `expand_canvas:false` preserves condensed out-of-canvas pixels.
The pinned arc source and complete pinned MIT layout references were read;
provenance retained, no original engine/bitmap writer adopted. Ledger still31partial.

`native_resource_paths.rs` follows complete5D4B50/6A73A0/5D59E0 native INT/LST
policy: configured extension, explicit path, list declaration order and each
row loose directory before archive; shared case lookup supplies authority.
`native_image_resource.rs` retains worker source/frame selection, separate
shared ImageJob, then actual owner publication/rollback. Missing file and
missing native frame are distinct; no wrong-frame fallback, default zero
geometry or ACK. `runtime` opts into the existing GPL format plugin; base
VM still has no upstream engine dependency.

Actual release evidence: `shared-hg3-family-final-tests.log` archives_e20pass/1ignored
and hg3_frames5pass. Inventory Python16pass. `native-image-resource-tests.log`
new real Runtime tests4pass; frozen same lib `cat-hg3-worker-lib` SHA256
`d4f9214c40773b705c227bd1ab8a4ec79f80e471cd4468c2c9a97df6f2cec23b`
then ran124pass/1ignored in `native-image-resource-all-lib-tests.log`. No owner
media/GPU run, typed gate unchanged. Full Python initially200pass/1failure due
to concurrent Siglus SOURCE hash registration; Root reports fixing it, final
rerun still required. Current shared API hash belongs to NNN's unified docs.

## Typed bank + native index AST follow-up

Production source now includes native_class_state.rs, fes_object_bank.rs and
native_image_publication.rs, but Machine::new still requires empty declarations.
Seven declared types/six distinct constructor variants only: BASE/VIEW, CIMAGE,
IMAGE, CPLANE, PLANE, SOUND. Full class fields, native index map and all-array
values-once semantics now have release tests. Typed bank constructor creates
logical IDs and original named values, never native/GPU/process pointers.

Bank tests caught real old missing named-array AST: original728FA0 attaches
index right child, while ours only removed brackets. Fix distinguishes true
value[name] indexing from prefix flag#[expr] grouping, nested reads/side effects
and selected owner banks. Exact final release131passed/1ignored in
native-object-bank-family-tests4.log; frozen cat-object-bank-lib SHA
561445d890edb3aa786a1df6dc524213cc4735e3b532e920cac368c005170569.
Earlier tests1/1failed, then full130pass1failed1ignored, are not passes.

Actual ImageList publisher independent Runtime126/1 source point tests true
shared image→Canvas atlas→Plane crop→PresentIsolated plus motion/status/opacity
and rollback; distinct class source point128/1. Still no native script binding,
owner-game advance or GPU claim. Full Python finally201OK in
native-hg3-worker-python-final.log. Current measured common API docs fingerprint
feb2ec9be29523ba, no-bless8+coverage1 by NNN; no fake private VM DTO registration.

NEXT ignored draft target/verification/catsystem2/fes_object_resources.next.rs
contains pending whole-bank CPU metadata→shared solid/HG3 ImageJobs→owner Layer/
Canvas publication, NOT compiled. Needs review/fixes before production: root Put
ordering/idempotent retained receipt; native name/index map traversal order;
Draw.pivot must use image.pose.pivot, not invented zero (746C40 actual copy
coordinates subtract it after748EF0 world position); explicit flat/white-RGB
texturedPlane profile until shared post-composition tint and full World matrix
bindings. Native masks/path modes/multi-image/dynamic metadata/KEYBLOCK remain
visible gates in draft. Resource readiness is not original native ACK.

## 2026-11-01 pending native HG3 profile + whole typed resource batch

Current production source (not yet release-compiled/validated): shared HG3
explicit `pixel_profile: native` requires complete40-byte stdinfo and supports
packed24 rows, numbered control segments, overwrite order/gap rejection, native
JPEG defaultmode1/depth-specific alpha and WebP embedded alpha/mode. Archive
profile remains the default. Ten authored shared tests exist; earlier5-test
receipt does not cover this new source. Cat native_image_resource explicitly
selects native profile; authored native segment row count is1, not reserved0.

### Final receipt for preceding pending batch

`native-hg3-typed-resource-batch-tests2.log` actually exited0: Cat135passed/1ignored,
formats lib44/1, archives_e20/1, HG3frames10/0. First run132passed/3failed/1ignored
caught adapter presentation IDs in Canvas's reserved range; presentation now uses
independent0x10000000+logical object ID, while class roots/Canvases keep their own
namespaces. Shared guard remains. Four new whole-bank Runtime tests pass including
raw-alpha/opaque-flag resources, real Canvas/Layer, idempotence/foreign rejection,
rollback and expression-once semantics. Frozen `cat-typed-resources-lib` SHA256
922e9bf77e9b43e544f3fb73c611458f7f843f4feea887cc2ed0eb22e9a125ac.
Python201/201 passed3.976s. Original Machine gate and old owner game receipt remain
unchanged. No active Cat Cargo/IDA/GPU process.

Next whole family: native motion alias values/parameter reference IR. Full7693A0,
75AAC0,76A470/570/070/130/190/230/769FC0/76A3E0 reviewed; original assembly corrects
HexRays spills, CVTTSS2SI defaults and exact MULSS/DIVSS order. Table has41 actual
parameter token names (p,BX/bx throughCX/cx), not45 implemented argument types.
Argument references select declaration kind/default but consume the NEXT actual
user word; constant colour/literal entries consume none. CX/cx take the original
zeroed-output/default switch branch without evaluating the expression. Raw `cxa`
and `CA` argument constants both select colour alpha. Next source draft lives at
ignored `target/verification/catsystem2/fes_alias_values.next.rs`; not compiled or
linked. Needs actual Interpreter temporary-default method, native class value
projection and original motion resource/producer/lifetime bridge before admission.

### Later source correction before next Cat release batch

Complete755D80 now exported: it consumes the DEFINE/index FIFO pushed by75C4D0,
and6287B0 builds actual resource entries in that order. The earlier publisher's
name-sort assumption was wrong: signed6A6CF0 applies to lookup-tree comparisons,
not initialization. Production `Bank::ordered_ids` now returns implicit root then
DEFINE/index FIFO. A new Runtime same-draw-key regression checks later DEFINE
wins even when names sort oppositely. New source has not been release-verified;
135/1 is the frozen historical result, not validation of this correction.
Alias-value draft has also become production private source plus4 new tests,
with a temporary-default evaluator wrapper; it remains uncompiled/unbound.
Latest next-Cat-lib expected test additions total5, not a new completed engine.

### Full native IMAGE/CIMAGE source correction + bounded HG3 work (next batch)

Complete65DC00/65C820 show different mode0 placement: IMAGE adds its pivot back
before the draw context subtracts it; CIMAGE sends748EF0 directly. PATHMODE1
retains native source origin and sets offset0/pivot=base, unlike path0's cleared
surface origin and offset=base/pivot=base-origin. The publisher now applies these
native policies separately over the same shared Canvas API. New real Runtime
fixture uses nonzero origin/base and checks PATHMODE0/1 positions. Literal
multi-image filenames remain rejected in both path modes until the full source
list policy is admitted. This new correction is not validated by135/1.

### Latest actual release141/1; pending notes above are superseded only for validation

`native-alias-fifo-geometry-budget-tests.log` exited0: Cat141passed1ignored,
formats lib45passed1ignored, HG3frames11passed0ignored. This actually covers the
alias4/FIFO1/IMAGE placement1 new tests, shared active-context1 and decode-work1.
New frozen `cat-alias-fifo-geometry-lib` SHA256
d9c6a383cc4ec6e265a2e801349069d183fe746d6a1d699ac50a5ab22eaf20d3.
No Source-only/expected count is substituted for this result. No new original
game/GPU/title receipt. Public docs actuala7e4bc50985744b6 was verified8+coverage1
by the unified agent; these private helpers add no new public shape.

Next complete phase: preserve629850 script execution before6287B0 FIFO resource
initialization,62D4A0 per-class resource updates and6285C0 sound observation.
Full wrappers newly exported/read:62DB00(87lines),65C820 CIMAGE(178),65DC00 IMAGE(764),
65CCB0 CPLANE(212),661430 PLANE(278),662640 SOUND(434). The gate is intentionally
not replaced with metadata-only success. Original BASE748580 calls child748690
before own MotionSet evaluation, commits final output when all tracks end;
Base seed748570 and callback748640 are actual one-byte RETs confirmed in raw
assembly. Two guessed addresses7485D0/7483A0 were function interiors and must
never be adopted as empty handlers. FES resource FIFO and motion-tree traversal
are distinct orders. Continue sealed per-frame resource/motion/input/lifetime
admission; no repeating original-game frontier loop needed before whole batches.

### Live class frame consumer142/1 (current source; owner verification pending)

Current production includes fes_live_frames: original331/447 typed metadata is
constructed only for reviewed flat classes, and629850 executes script before
real shared resource/Canvas/Layer publication. Only that receipt lets a forced
child tick complete and its parent execfes ACK. Subsequent pose-only frames
reuse image Arc/generation. Native endfes/task disposal removes real class
Canvas views/sender records/input/root in one shared batch, then asks shared
Runtime to retire leases while preserving older snapshot readers. Failed image
release remains queued. Generic native ACK never accepts Object; direct class
disp/size/pri completes via its own path. No unknown class/function is ACKed.

Plane mapping corrected from raw74A4E0/540/600/620 and full74AC30/74AD40:
physical POS0x30 is pose6/7/8; BASE0x40 is3/4, priority5. Prior helper tests had
swapped Rust names, now corrected. Other32 types, full alias/motion, SOUND,
baseimg/rand/CST, nonzero/unknown atlas base, affine/nonwhite Plane, dynamic
metadata/KEYBLOCK remain incomplete. VIEW really uses BASE constructor7544F0
case5 (same2c allocation/7480F0), not a guessed camera implementation.

Actual live-class-frame-tests2.log142passed0failed1ignored. Frozen lib
cat-live-class-frame-lib SHA256c23d01f06874329edd51582a8956a298c2c23768bd68a58fea3223ebbf4fff6b.
Python201/201 passed. Current owner_game build13666 is the one authorized actual
media run for this complete batch; its strengthened test now requires IMAGE +
CIMAGE shared leases/draws and all6 PLANE views, rather than accepting metadata
or the older FES53 gate. No new actual game/title result until that runs.

Next alias loader draft is only ignored
`target/verification/catsystem2/next-fes_motion_loading.rs`, not in mod graph.
Before landing, provide Bank.active_subtree/State.motion_seed, actual postorder
motion update at7563A0 LABEL115 (after script yield, before6287B0), real shared
external producer stamps and completion/skip, resource cache bounds, and
original native builtin precedence over same-named aliases. Do not merely insert
controllers without executing/publishing output. No ambient clock seeds: Root
confirmed no shared session entropy capability; use explicit owner seeds and
coordinate a general public contract.77BBB0 rand uses raw integer stream modulo
(max-min+1) wrapping native DWORD;77BC80 randtbl consumes ignored fourth argument,
initializes destination range and performs1001 pairs of swaps, not a generic
Fisher-Yates shuffle. Full source for both is already exported/read.

### Actual original typed frame accepted (supersedes pending13666 above)

Build13666 passed; frozen cat-owner-live-class-frame SHA256
4e9e2b083710ffa6adea60b6ab3dd900596a5fed096b5b4b094408ed42d09ae5.
Exact original-owner test1/1passed0ignored in1.11s, log
live-class-owner-actual.log. Original IMAGE+CIMAGE2 real shared image leases,
2 Canvas draws and all6 PLANE Canvas/Layer views asserted. Parent execfes ACKed;
original flow59ops/KCS20410. New strict test requires those engine resources,
not merely a cached declaration bank. No GPU/title claim. Nativebaseimg at
brandlogo START physical79 byte0xad8/ownerPC0x92c retains FES owner continuation.
Continue complete baseimg/alias/motion/SOUND source families; no need to rerun
the same first-frame acceptance per helper. Owner fixture already mounts ALL
actual INT namespaces (the older MenuHost helper is the limited mount fixture).

### Native baseimg family143/1; baseimg_pri source-only next point

Full77A860/event0/1+ImageList296 now production in fes_base_image.rs, backed by
shared Canvas-source bitmap0, native ordering and a low live view under existing
high-ID native base payload. No-arg removes only draw0/hides, retaining old copied
bitmap; bitmap args copy raw resident bitmap, Plane args flush/copy currentVRAM
with source upper bounds. All copies/snapshots live in the engine. Raw779D70 /
757BC0 /757BD0 confirm same-clock event continuation, not artificial wait.

Actual native-baseimg-family-tests2.log143passed0failed1ignored; immutable
cat-native-baseimg-lib SHA2564d4ae635ba599c99318a120a8271051e641848b6f4c68a120a6d0dc479141fa5.
Test uses real Runtime Assets.surface old2x2 RGBA (encoded ImageMetadata is the
wrong service for __image_resource paths), real old-reader retention/Plane copy/
hide and unchanged payload transform/external driver stamp. First142/1fail/1
was only that wrong fixture query. No new game run after owner4e9e2b.

Next Source-only change already in production: baseimg_pri780710/event28 /
ImageList297 uses signed expression/default1000, base position(0,0,z), existing
Layer SetRect/SetTransform/SetDepth/SetVisible and preserves producer. New FES
regression uncompiled; next expected library144, never claim143 covers it.

Root approved common bounded async session seed material (no clock fallback,
not crypto/credentials API). Complete Core/Runtime drafts only under ignored
next-core-session_seed.rs and next-runtime-session_seed.rs; existing getrandom
0.3.4 is installed/source read, will need exact Runtime dep. Coordinate NNN public
window after viewport+retained audio before mod/trait/source graph admission;
Alice confirmed service.rs audio stable, only seed mod/reexport/default method
is ours. Planned Source Os{words:u8}/Fixed{words:Vec<u32>}, bound1..16; typed
Material{client,words}, State Pending/Ready/Failed/Cancelled, Query client/poll/
cancel. Runtime reserve-before-OS, deterministic fixture, no fallback, ownership/
cooperative cancellation. No shared source was changed and no seed test yet.

### 2026-11-01 — CatSystem2 live alias/motion/root and owned random streams

Implementation: actual native object aliases resolve original mot KCS through
bounded resource workers, execute the first-class KCS Controller in child-first
native tree order, and publish class poses plus the root external-motion stamp
in one engine transaction. Native wait observes real shared completion. Both
native skip and public producer skip execute the original KCS skip flag, retaining
the original completion-yield. Native rand/randtbl consume the same owned integer
stream (randtbl uses1001 pairs of swaps); no ambient time seeds. baseimg_pri
preserves existing native payload/producer and applies signed/default depth.

Shared capability: Core/Runtime now expose bounded asynchronous session seed
material,1..16 words, explicit Fixed or OS source, client-owned completion/cancel,
reserve-before-entropy and visible failure; the default host is Unsupported.
Core remains no_std and dependency-free. Runtime getrandom0.3.4 samples on its
worker; this is game-stream seed material, not a credential/security API.
Cat's optional random_seeds is an exact [u32;2], mirrored by strict Python
validation and its native config schema. Tooling validation4/4 actually passed.

Compilation and behavior: shared-session-seed-tests2.log release8/8 passed
(Core2, Runtime-private4, real Host integration2). The initial compile rejected
accidental Core serde derives; those were removed, with no Core dependency added.
The integration includes actual OS16-word sampling without any strength or
uniqueness claim, fixed reproducibility, queue/cancel/drop and client isolation.
live-motion-root-random-tests2.log release149passed0failed1ignored. Immutable
cat-live-motion-root-random-lib SHA256
cf1e5e1b1686f04db546e3d46854d981c7dbe33270fabaee1f48fd8705154a22.
The previous test build had only a fixture access to nonexistent LayerSpec.depth;
it now queries real LayerSystem.depth. Fixtures use authored KCS, real shared
Canvas/Layer resources, real native/public skip and actual root revisions. The
strict Rust config contract passed. Public docs regeneration is coordinated and
pending; prior531a48a7ce30ee00 is not a seed/config receipt.

Original media: no new game/GPU/title run in this batch. The last real original
receipt remains IMAGE+CIMAGE2 leases/all6 planes at59FES/KCS20410. SOUND playback,
other native classes, CST/input and original saves remain incomplete; complete
SOUND source/resource/playback consumers are next before another owner run.

### 2026-11-01 — CatSystem2 original SOUND basic resource/transport consumer

Implemented:76BAF0/76C000/770200/7702F0/7792A0/779A80/779AC0 argument and flag
policy plus662640/6285C0 resource/transport stages, retained between real owner
frame phases. Native prefix/LST/INT lookup runs on bounded workers. Load publishes
an actual shared immutable AudioResource; it does not create/play a voice. Play
waits for real PlayResource admission. Shared Audio owns decoding, mixing, pause,
looping, source status and PCM. Native preference integer products/dB and exact
channel-range policy stay in the adapter. Grouped active release/pause/stop and
paired crossfade are explicitly gated until their original fader lifecycle is
implemented; no linear-amplitude approximation is substituted. Remaining native
blend/stop require their real original KCS controller policy.

Compilation/behavior: first releasecheck rejected two overlooked oldFrames.reap
call signatures; all callers corrected, no host contract relaxed. The following
live-sound-family-tests.log release actually passed151/0failed/1ignored. Frozen
cat-live-sound-basic-lib SHA256de67223b464b06e2e00f29b19ea91f699cb33911fa5a6abf7d01178afa54fe6f.
The new authored FES/WAV test exercises real Runtime resource preparation,
load-without-playback, actual48-frame stereo PCM0.5, paused cursor preservation,
looping past EOF, dynamic loop-off, actual EOF/native playing reset and empty-load
release. A second pure test covers consumed/ignored arguments, defaults and flags.
This is independent engine-service behavior, not a new original game/title/GPU
receipt; the complete SOUND family and other typed classes remain ongoing.

Static read-only review also found5EA3F0's real fader recurrence: subtract elapsed
native ticks from binary32 remaining, then current+=(target-current)/remaining
once per update. No elapsed multiplier; original action completes on the update
following target arrival. This is not ordinary linear tweening. A shared Scene
scalar step kernel is proposed to Root; no such new public API is claimed yet.

### 2026-11-01 — Cat SOUND count ABI corrected and real native fade updates

Source correction: the151/153 authored loop fixtures exercised real shared PCM,
but their original0/1 boolean-loop assumption was wrong. Read-only original
5D1955/5D1A9C bytes compare native word0x124 to1(end), preserve0(unlimited), and
decrement other DWORDs at each pass. Native77 is total pass count:1 once,0
unbounded,N finite. The adapter now uses actual shared loop_passes; count1
ignores LoopPoint, while other counts use the explicitly published native range.
Original5D17A0 halves44100 coordinates only for22050; other rates retain original
coordinates. Native218 changes count/range while keeping the current cursor.
Historical test passes are not evidence that the older boolean mapping was right.

Shared extraction: Scene animation::step_remaining_time_scalar performs the
original binary32 remaining-time per-update recurrence. Its release3/3 passed
(shared-remaining-step-tests.log), including irregular/zero elapsed, invalid and
overflow inputs, and target-arrival versus next-update action distinction. Native
Cat fader keeps only original clock/action policy, using this shared kernel and
shared Audio levels/SetGain/transport. Faders deduplicate input revision, retain
partial progress/completed actions across failures and advance independently of
the FES script. Grouped releases remain retained until the real terminal action.
Paired crossfade and play on another release-owned slot still fail explicitly.

Compilation/independent behavior: live-sound-counted-fade-tests.log release
154passed0failed1ignored. Frozen cat-sound-counted-fade-lib SHA25622bbca50ca6416b595dfe08d3859f58fce66cb392d552d95e0070ae109a89891.
The authored512-frame WAV produces exactly1024 audible PCM frames for native
count2 then actual EOF. First fade update reaches original88percent/dB, not an
invented amplitude0.9; target arrival remains unpaused until the next owner update.
The shared step3-test receipt is separate. Prior153/1 remains historical before
the loop correction. No new original-game/title/GPU receipt. The new pColor blend
and bounded stop continuation source is a subsequent currently unverified batch.

### 2026-11-01 — Cat native stop and reviewed pColor controller source

Implementation: original76B7B0 stop retains a bounded child-first target
continuation, asks original KCS Controllers to skip, executes them at original
clock0 and commits their actual output before setting SOUND playword0. No
controller is deleted or acknowledged complete on behalf of code ignoring SKIP.
Native blend now loads/executes the real pColor resource and publishes a genuine
root producer. The SOUND-only unobservable source-pointer RGB normalization is
restricted to the reviewed767-byte pColor layout (CRC32975a7795, code SHA256
6bdd6c4a8a5a6c2f69af35366a14e5a03453f5fd86073d64a9d5a670f012d893), globals0,
stack1024 and no static strings; arbitrary replacements fail visibly. This CRC
is a layout guard, not a security checksum. Original bytes were only read from
ignored local media; none were added to source.

Compilation/behavior: first live-sound-builtin-motion-tests.log was153passed/
1failed/1ignored. The new real stop test exposed a genuine old MotionSet clock
boundary: a wrapped DWORD delta can round to2^32 in f32. Static78945A calls
79FE00, which truncates to signed64 and returns EAX low DWORD, so2^32 yields0.
The bounded VM lowering now follows that actual ABI, with independent boundary
checks; it was not bypassed by terminal evaluation. Final
live-sound-builtin-motion-tests2.log release155passed0failed1ignored. Immutable
cat-sound-builtin-motion-lib SHA256c4c8360398a9912247dc8a4bdd3abe5495e0714734c3c500c2daeab606966830. Existing genuine alias/engine-skip/
native-skip tests also run an actual object-stop path to the shared Canvas
endpoint and verify ignored extra arguments. Positive original pColor/game
acceptance is not covered by those authored tests and remains pending.

The subsequent grouped FES fade/release fixture and richer actual audio-resource
diagnostics/ReplayGain metadata guard are a new currently unverified source point.
Logical channel exchange for original paired promotion is being added as a
separate shared engine capability; its package releasecheck passed, independent
behavior/API verification remains pending. No title/GPU/new original-game claim.

## 2026-11-02 gameplay frontier

Nekopara vol3 reaches `kcs.int/sscript.kcs` and stops at native 342, PC 0xe6585. Tools: `owner_kcs_native_census` counts natives per script and, with `CRYSIRROR_CATSYSTEM2_DUMP_DIR`, dumps KCS code. The dispatch table is at A2DD38 (dword per id); FES natives are at 8F4A58 (36-byte entries, 32-byte name then handler); `noact` names the arrow-key default focus button, it never disarms a press. See STATUS.md for placeholders that still need real implementations.

## 2026-11-02 first gameplay screen

Nekopara vol3 now plays from the title into `sscript.kcs` and renders the first scene: background, message window, name plate, tool buttons and the first line of text. Added: shared KCS heap across tasks, several KCS script tasks, FES section calls (630) and arguments (645-647), STRING8 text (`fes_text.rs`), KCS Plane natives (`native_plane.rs`), CatScene reader and stacks (`native_scene.rs`), byte strings (`native_strings.rs`), KCS image draws on Plane views (257, 258, 710, 732), FES `send` and button routing to the owning KCS task.

Dialogue now advances (payloads are KEYCUSTOMIZE slots) and class 54 draws E-mote bodies via eluna (`native_m2.rs`). Frontier: native 261 (ImageList message 285) in a 22000-frame click run. `native_inert.rs` lists natives acknowledged with no effect; `CRYSIRROR_CATSYSTEM2_SCOUT=<id-to-return-slot json>` is a local-only probe in `adapter.rs`, to remove before this is called done. `crates/crysirror-puppet-eluna/examples/psb_probe.rs` lists a model's parts.

## 2026-11-03 completion push (in progress, compile-checked only)

Owner order: implement every remaining native; no full/workspace tests until done (only
`cargo check --release -p crysirror-catsystem2 --features runtime`). No subagents.

Seam: `src/native_classes/` families (`IDS`/`State`/`handle`→`Ok(false)` yields and re-offers
the call/`tick`/`retire`); its claims arm is the FIRST arm of the adapter native match.
Ctx carries machine, host, tasks, settings, handles, interpreter, saves, pool, input, m2,
movies, kcs_sound. `create_opaque` now keeps class/parent/name.

Corrections found: native_pending/native_inert listed 37 ids already implemented by
native_store/native_misc (dead entries, removed). `native_messages::FORWARDING` treated the
first argument of message-34/35 natives as the target task; those address the always-present
root (5DE390 answers INPUT/SOUND ids), so sound natives 156-163 etc. were silently skipped when
their channel number was not a task id. All 34/35 entries removed.

Implemented (compile-checked, not run): Plane 95 (own click area from geometry), 346/654/655
(original no-ops: 5E01F0 71/85/86 → LABEL_104), 651/694 (24/29 fall through 5DE8E0 default),
653 (Plane render type this[170]; 78/79 now drop the payload for type≠1). INPUT family
(`input_task.rs`): 106 (INPUT 132 hover marker), 109 (INPUT 136 last source), 149 (125
pointer), 150 (137/138 autocur glide, 5E66C0 per-tick (target-x)/remaining with
remaining+=start-now), 286 (145 clear markers + re-evaluate), 394 (146 alpha hit mask via
WidgetHitMask), 401 (149 software cursor → ui set_cursor_visible), 404 (151 key-slot poll table
→ Bindings.polled, filtered in slots_for/held_sources), 937 (157 → 710240 wheel accumulator →
input_routes discard_queued). Containers 240/250 (FES player bank range ↔ CSS record).
Shared additions: InputRouteCommand SetHoverMarker/ClearHoverMarkers{reevaluate}/DiscardQueued.

Static facts for the remaining work:
- Class 8 (ctor 5E38C0, handler 5E4540, 0x770 bytes) is the KCS text window: msg 103 append
  text (124, 75 script uses), 104 clear (125).
- Class 11 is the INPUT task (5E6F50); class 13 the SOUND task (5EAEA0).
- 168/347/348 → SOUND 193/195/196 → DirectSound3D SetPosition/SetVelocity/(+0x14 vtbl 0x30)
  on channel+0x10/+0x14; 3D interfaces exist only for mono buffers created with DSBCAPS_CTRL3D
  (5D0A49). Position also requires B91BFC+0xF14C. Shared Mixer has Spatial but AudioRequest
  lacks a SetSpatial request; creation-flag source still to trace.
- Class 35 (ctor 5F6A10, handler 5F6DE0, callback 5F6ED0) is a full-screen render-target
  post-effect: 378 = 88-byte matrix/offset block, 379 = enable. Classes 36-39/50-52/57/58 share
  messages 377-395 (same family). Needs a shared whole-frame post-process capability.
- Class 48 (ctor 634480, handler 634770) is a text plane over a 696880 layout object with a
  232-byte config (449-468).
- Hex-Rays/idalib crashes under current memory pressure; `target/verification/catsystem2/tools/
  cs2dis.py ADDR [n] [nostop]` and `cs2scan.py SUBSTR` disassemble the static EXE with capstone
  (homebrew python3.11, run with -I).
Remaining ids: `native_pending.rs` + `native_inert.rs` (≈176).

### E-mote class 54 natives (static findings, not yet implemented)

- Native → message: 907→526 (57E420→5744D0, a1), 908→527 (57E500→57C1E0 reset every
  player's queue this[96]), 909→528 (57BFD0 any player busy, value), 924→540 lip sync
  (a2 = SOUND channel, a1 = on/off; SOUND 222/200/220/221/211), 929→543 (574D70 name into
  player[63] timeline controller, flags [9]=1 [10]=1 [11]=[12]=0), 930→544 (574DC0 clear),
  931→545 (5743F0 player[40]=a1, [39]=1), 965→547 (player[64] 598760 load named set,
  +32 enable flag), 1025→548/549 (575650 pause bool, then 57E180), 1027→551 (591620 queue
  by name in this[96]; native always answers 0), 1028→553 (this[96]+8 != 0), 1029/1030→554
  (587A70 player[25] map lookup), 1047→552 (591820 advance queue), 1048→550 (575860 clear
  queue). 965/1025/1027-1030/1047/1048 do nothing unless the M2 manager singleton
  B9BD94 exists (created at 57CDB3).
- player[63] is NOT an E-mote PSB timeline: its tick 585BD0 samples CatSystem2-side keyframe
  tracks (5A28C0 track lookup by name and time, 5944D0 variable lookup, 5835B0 writes
  value × weight). Mapping 929/930 to ElunaPuppet::play would be wrong. Needed first: decode
  the M2 track data the model loader attaches (find the loader that fills player[63]/[96]/[64])
  and add a generic keyframe-track player to a shared crate, then drive puppet variables.

### Batch 2 (compile-checked only)

- os_window: 9 (64F910; a1 == 0 → 0x433 → hide cursor + teardown → adapter halts;
  a1 != 0 → WM_CLOSE MessageBox confirmation, fails visibly: no host confirmation-dialog
  service yet), 441/565 (676A80/6761E0 frame dump: mode/folder/counter; tick captures with
  begin_frame_capture and writes `folder/folder_%010d.bmp` (Bgr24) to the Save mount —
  encode currently runs on the owner thread).
- image_ops (ImageList class 15, 5F3AE0): 342 (292: this[35]=1, manager → this[31]=0 and
  recompose), 464 (300 → 744540 restart draw motion + dirty rects → republish), 534 (304:
  this[35]=a1). Non-ImageList targets are original no-ops.
- containers: 609/610 (728D60/728D80): 6A6D50 copies `count` BYTES between flags and the
  backup allocation (only a quarter of the flags); restore before any backup fails visibly.
- Static notes for next batch: 241/243/253/385/386/576 serialize subsystems into CSS
  containers (690210 static, VM+180 object 73FE70/740070/740290 shared with natives
  276-278/336/991/992/994, 72D120/72D320 on handle+4, msg 431). 315/316/317 snapshot an
  object via 7898E0 into a {obj,buf,size} handle. 365/366 load a refcounted raw "image"
  folder blob; 368 → 68F480 36-byte image info; 369 → 68BEC0.

### Class 8 text window (5E38C0 / 5E4540) static facts

- Nekopara vol3 creates class 8 tasks "info", "cursor", "bank btn" (sscript). Natives used:
  418 (5 uses) configure, 124 (75) append, 125 (12) clear, 557 (5) msg 121 → this[1880].
- 418 (652150) / 123 (6522E0) send message 101 with a 1128-byte block copied to this+708:
  first 96 bytes from the VM struct (a2), then rect, count+WORD[256] char set from a3
  (418 also a second count+WORD[256] set from a4; DBCS bytes become big-endian WORDs).
  Fields: +708 x, +712 y, +716 width, +720 height (plane rect via 5E01F0 41/51), +728
  draw-on-config flag (677090), +732 default colour (→ 1884..1900), +768 glyph capacity
  (40-byte glyph records at this+684 = 0x2AC), +772 → this[184], +776 mode, +792 type
  (1 → 1864=1; 2/3 → 1868=1). Builds a 64×64×4 glyph cache (691690) using the setconf
  fonts at B91BFC+63480 / +63608 (weight 400, or 700 when +63744).
- 103 (5E5AE0): no-op until 101 allocated 0x2AC. Parses SJIS: '[' (0x5B) ruby open
  ('/' separates base and ruby, ']' closes → 5E5660), '\\' escapes: '\\c<n>;' colour
  (this+0xB8), '\\f…' size: s/S → +0x75C, m? → +0x760, l/L → +0x76C, … (+0x764/0x768),
  '+'/'-' relative; each char → 5E4F60; then 5E3B30(this+0x310) lays out lines.
- 104: clears glyph records, resets cursor (1836/1840 = 752/756), this[184] = this[772].
- Not yet implemented: needs 5E4F60/5E5660/5E3B30/5E3F90/5E4220 and the typing tick
  read from disassembly (Hex-Rays still crashes under memory pressure).

### Batch 3: class 8 text window (compile-checked only)

- `native_classes/text_window.rs`: 418/123 (config, msg 101), 124 (append, msg 103),
  125 (clear, msg 104), 557 (edge colour, msg 121). Native 66 now builds class 8 as a Plane
  (5E38C0 on 5DF070) and registers window state; non-window receivers are no-ops.
- Ported original layout: 5E5AE0 parser (DBCS WORDs; '\\c<n>;' colour, '\\f' ss/s/n/l/ll,
  ±n, n sizes, '\\n', '\\\\'; other escapes skipped; 6A49D0 number parse incl. 0x hex),
  5E4F60 placement (full-width advance = size, half-width = (size-3)/2+4, letter/line
  spacing, wrap at width, stop at height or capacity, 「 hanging indent when +0x754),
  5E3B30 vertical alignment modes 1/2, and 5E3F90's draw-once positions (+0x24).
- Glyphs publish as two GlyphRun layers under the Plane payload (ids 0x14000000+id*2):
  type 2/3 edge = new shared `GlyphCoverageKernel::BoxSum` radius 1 (6A3EF0 separable 3-tap
  unnormalised sum, crysirror-text `glyph_coverage::box_sum`, wired in crysirror-wgpu) in
  the edge colour, face over it at glyph size h-2 on the font ascent (GGO cell placement).
- Fail visibly (not used by Nekopara vol3 configs): ruby `[base/ruby]` (5E5660), kinsoku
  (+0x31C), timed reveal (+0x308 == 1), shadow type 1 (6A40D0). Bold weight 700
  (B91BFC+63744) is not applied yet.
- docs/api must be regenerated after the final test run: new InputRouteCommand ops
  (set_hover_marker, clear_hover_markers, discard_queued) and GlyphCoverageKernel::BoxSum.

### Batch 4 (compile-checked only)

- 320 (659350): in-place cubic spline over VM records {x,y,b,c,d} (FMM end conditions with
  the original's x[3]-x[1] first-end denominator and unscaled last c), `subsystems.rs`.
- 336 (642460): "$str<n>" string variables attached to scene name entry a1 under (a2, n)
  (72D7C0 → 734AD0, 62-byte cut, same key replaces); `NativeScene.attributes`, cleared by
  330/331. Not yet serialized by 385/386 (72D120/72D320 still pending).
- 625 (644060 → 295): draw motion time/play; draws from 257/732 never carry a motion
  (message 280 word 14 zero), so no visible change — implemented as that no-op.
- Deferred: 683/684/685 = `config.int/3DTable.txt` table object (787F50 parser ~900 asm
  lines, 788330 cell query by row/column) used only by the 3D View (class 43, kx2).

### Effect tasks (next): static facts so far

- Classes Nekopara creates: 18 raster, 26 wipe, 27 draw, 29 wipe2, 30 wipe3, 31, 35 blur,
  36 mosaic, 38 rdraw, 39 rwipe, 50 rwmos, 51 rfmos, 52 wbreak (handlers in
  class-handlers.json). Natives: 196 (641050) asks msg 13 for the class, then sends its
  parameter message (17→265, 18→263, 19→266, 20→267, 26/29→274, 27→275, 28→276,
  30→277, 31→278, pointer to a VM block); 197 → 257 start, 200 → 259 running flag (value),
  201 → 264 (class 18) / 273 (class 25) when a1 == 0, 203 → 261 stop.
- Class 26 (5EFD50, vtable slots 5EF890/5F1850/5EFA10): 274 → 5EFB30 copies a 56-byte block
  to this+0x6C (word 0 duration ≥ 1, +0x70 image A, +0x7C image B, +0x88 rule image, +0x9C/
  +0xA0 offsets), output extent = min of the three image sizes, allocates a 0x5C output
  image; 257: start = 5DCCE0 time, running = 1, speed this[46] = 1.0/1.5/2/3/4 by setconf
  B91BFC+77780; 259 answers running; 260 sets this[28]/this[31]; 261/258 finish (5EFEE0).
  Per-frame image generation is slot 5EFA10 (not yet read).
- Class 26 per frame (slot 5EFA10): frame = ftol((now - start) * speed); frame ≥ duration →
  677090 draws image B's region (+0x80,+0x84,+0x94,+0x98) at (+0x8C,+0x90) and stops; else
  5EFF10(frame) composes into the 0x5C output image and 677090 draws it at (+0x8C,+0x90).
  5EFF10 kernel (exact): phase = clamp(min(frame,dur)*512/dur, 0, 512); per pixel w =
  clamp(phase - 256 + rule.B, 0, 256) (rule byte 0 of a 32bpp image at offsets +0x9C/+0xA0);
  each of the 4 channels out = (B*w + A*(256-w)) >> 8; A at (+0x74,+0x78), B at (+0x80,+0x84)
  plus the same offsets; extent (+0xC0,+0xC4).
- 677090 targets the render manager's screen surface: 70DAE0(rm[2]) = rm[2]+8, a tiled
  texture grid uploaded by 6802A0. Its place in the frame composition (above/below planes) is
  NOT yet established; the adapter currently treats image draws to destination 0 as no-ops.
  This must be settled before any transition class can be shown. Scripts around 0991EB call
  261 (ImageList work bitmap handle), 112/116 (images), 340, 862, 111 before creating "wipe".

### Batch 5 (compile-checked only)

- 111 (64BB80): space-separated "name[:frame]" images (6A5AB0/6A53B0); first loaded is the
  base, later ones AlphaOnAlphaScalar256 over it at their canvas origins (bitmap +0x24/+0x28,
  68B620 clipping) — same composite math the adapter already uses for 115. Blend mode
  bitmap +0x54 is not consulted (as for 115).
- 365/366/364/368 (64B010/64B3A0/64C190/64B9C0): refcounted "image" file blocks; 364 decodes
  frame a2 from the block (new `Pending::begin_bytes`, `Request::Bytes`); 368 writes the
  6A1E70 36-byte stdinfo {origin x/y, size, canvas, bpp/8, base x/y}. Only HG3 is decoded
  (68F480 also tries other formats).
- 369 (64B410 → 68BEC0): 32-bit images only, every stored byte * ARGB factor / 255 (floor);
  new shared `ImageEffect::ChannelScale { factors }` in crysirror-image.
- Not yet: 315/316/317 serialize an object through 7898E0 (0x454-byte records with names at
  +0x14 and 78A3F0/78A630/78A680 sub-objects); object kind still unidentified.

### Batch 6 (compile-checked only)

- Plane blend 402 (6718F0 factor pairs) via new shared `Blend::{SourceOver, Modulate,
  InverseDestination, ModulateAdd}`. Movie 267 stop, 998 position. Container/string bank
  243/253. E-mote 909 (busy).
- Scene table: `NativeScene` now has both 72CCB0 lists: `names` (72CCF0/72D050/72D8F0) and
  `history` (72CDB0/72D080/72CF40/72D7C0). This fixes the old 329/330/331/333/982 arms that used
  one list for both. 385/386 (72D120/72D320 format incl. legacy records), 978/979/981/993,
  608, 980, 1033 (no .cstl, as 1038). Not done: 994 (5B12A0), 991/992 (5B0B10 family).
- 1036 language prefixes (`language_prefix_of`).
- Particles, class 16 (`native_classes/particles.rs`): 271/272/273/274/275/426/427/693. ptcl
  KCS script restarted per particle per frame (877270), native table A40778 ported (0..50,
  48 fails visibly). Quads published as one PuppetFrame part per task; 67EC90 translation by
  vertex w, 678170 perspective 1000/(1000 - z) in x/y only. rhw is dropped, so rotated quads are
  affine, not perspective-correct. Layer blend carries 6718F0's mode; overlap within the task's
  isolated target is Normal except mode 1 (Add). The 67C1C0 camera (renderer +0x60/+0xE4) is
  not applied. `ptclpi` = Windows DLL: 693 logs and takes the load-failure path (no particles).
- Screen surface notes for transitions: 67FB80 draws pass callbacks 1, then [2, 67B810, 3] while
  pass 2 answers true, then 4 and 5 (callback +0xC0 registered by classes 35..). 67B810 uses
  renderer grid tiles through 680990 as render targets. Whether 677090 output sits above or
  below Planes is still open.
- HG-2 (241, format 4, encoder 69FF50) has no shared codec. 649/576/688/689 not read further.
- 168/347/348: `<SOUND><use3d>` (+0xF144) is 0 in Nekopara vol3, so no 3D interface exists and the
  original changes nothing; use3d != 0 fails visibly (no spatial routing yet). 279..283, 287, 288,
  570: Win32 developer dialogs (classes 32/33/47), noted once in the log and not presented.
- 903 stays pending: SOUND 214 reads a per-channel name→value map at slot +44 whose writer is
  not identified, so answering 0 is not established as faithful.
- Classes 44/45/46 (600/601, 690..692): "export" worker threads (633640, 633EC0, 634250), not read.

### Batch 7: transitions (compile-checked only)

- Correction: 677090 does not draw to a "screen". rm[2]+8 is the VRAM tile atlas (6802A0 uploads, 680990
  selects a tile); Planes sample it. In the adapter VRAM is the main ImageList Canvas.
- Classes 26/27/28/29/30/31 in `effect_tasks.rs` (natives 196, 197, 200, 203). Kernels read exactly:
  Soft T=clamp(f*512/d-256+r,0,256); Fade f*256/d; Hard r>=256-f*256/d; Ramp start[r]=d-r*d/255 then
  (f-start)*256/v; Mix (B*w+A*(256-w))>>8 on 4 channels, A.a==0 takes B.rgb (5EF930). Alpha classes 28/31:
  a=(A.a*T)>>8 or 255 minus it. Shared `ImageTransition.rule_table` (RuleTable/RuleOperation).
- Scripts pass the list work bitmap (LIST_BITMAP) as second image, so the frame is laid over the live
  list Canvas with inverted weights rather than reading it back: result agrees within rounding, and
  the alpha result assumes an opaque Canvas. Draw records (key 0x7F00xxxx) use source-over,
  while 677090 replaces pixels; this matters only for non-opaque output (classes 28/31).
- Classes 18 raster, 19, 20 etc.: 196 logs once and drops parameters; 201 is still pending.

### Batch 8 (compile-checked only, nothing run)

- Task factory 5DD580: classes 3,4,5,7,9,10,55,56 have no constructor (null task). Scripts
  create 2,6,8,12,15,16,18,26-36,38,39,43-48,50-52,54,57,58,62.
- Class 48 (Select.kcs): natives 611-621 need config word 232; the script passes 196, so
  they are no-ops. Name tables 633-640 read `config.int/nametable.csv`; 567 and 904 done.
- Platform tasks 44 ChangeWP, 45 Export, 46 ChangeSnd, 62 Tweet: start logs and does not touch
  the OS or network, finished queries answer 1, 1022 answers 0. **Deliberate gap.**
- Database 869-873/878 (`qdb.int`, absent in Nekopara vol3): empty-map model; a present file
  fails visibly because the 82ABD0 parser is not ported.
- `tables_3d.rs`: 683-685 over `config.int/3DTable.txt`. `host_misc.rs`: 76 (parent id),
  315-317 (scripts only keep and free the handle), 514, 577, 598, 628, 741 (windowed 0),
  846 (no host minimize request, logged once), 1009 (bmp/png only, type 5 fails visibly),
  1049 (`text_window::plain_text`).
- `scene_records.rs`: 991/992/994 per-entry chain of zlib records; tracked flags are
  GAME/flag local0..7, tracked strings are `localstr` in config/localstr.xml. 790 now reads
  GAME/flag local/global ranges from `NativeSettings::flag_ranges`.
- `render_settings.rs`: 541/542 (tile release), 568/859 (viewport depth range), 681 (cache
  drop), 858 (shader query, result dropped) change no frame; 879/881 only keep link state
  because draws carry no frame motion. 879 also skips the original's sound message 211.
- 576/688/689 send messages 431/439/440/441 to a task id read from the script struct at
  0x2c630. No class handler with those cases was found by constant search, so the receiver is
  still unidentified. 649 targets class 34 (message 343, handler 62BED0).
- Render manager holds one frame hook (67FE70, this[48..49]), so one shader class at a time.
  Hook phases: 0 region, 2 per-pass target and z band (a4+20..a4+24), 3 composite, 5 end of
  frame. Class/native map: 35 521/522, 36 523/524, 38 527/528, 39 530/531 (905 same message
  with word 4 = 1), 50 853/854/855, 51 856/857, 52 860/861, 57 967/968, 58 970/971/972; 520
  reads the enable state (377). Each needs z-band capture plus its D3D9 pixel shader decoded;
  the renderer has `Paint::Snapshot`, `Distortion` and `CompositeFrame`, but no z-band backdrop.
- Nekopara vol3 ships no kx2.int or veff archive. Natives 81/988 should be no-ops when the file
  is absent and fail visibly when present. Message 46 (433) builds a child "ptn plane" grid;
  messages 79/80 (398/399) keep a per-plane trail history (5E31D0).
- E-mote lip sync (924): 637F30 message 540 reads sound messages 222/200 and drives players
  through 574CE0/575600; it needs a sound level source the host does not expose yet.
- 538 → 68D840 pads an image's canvas and shifts its offsets (+0x30/+0x34).

### Batch 9 (compile-checked; unit tests of this crate only, nothing run against a game)

- `native_pending.rs` and `native_inert.rs` are gone: no KCS native is acknowledged silently
  any more. An id outside every family fails with "unsupported native gcall".
- `shader_tasks.rs`: rdraw 38 (527/528), rwipe 39 (530/531/905), rwmos 50 (853/854/855), rfmos
  51 (856/857) are shared screen transitions (`layers` op `transition`), started when the
  script hands over parameters, durations in 60 Hz frames. Scenes use `rdraw` 1165 times,
  `rwipe` 170, `rwmos` 8. **Deviations:** the originals fade only under their z band (the text
  window stays out of it); if a bitmap publishes in an earlier frame than the parameters, the
  fade has nothing old to show and the change is a cut. Classes 35, 36, 52, 57, 58 fail
  visibly when enabled: no Nekopara scene uses them.
- `plane_effects.rs`: 344 world matrix → `local_matrix` (a perspective row fails visibly), 398/399
  history trail (5DFE50/5E29C0, mode 0 only; modes 1..5 fail visibly), 433 ptn child grid, 701/988
  video object through `Movies` with 702/989 answering its id, 81 kx2 (missing file no-op;
  present file fails visibly: Grisaia's kx2.int has raster_f/x/y.kx2).
- `scene_log.rs`: 699/1034 rebuild the log from the visit history on a worker. The `novel` page
  flag (72E8E0 a7) is not modelled because `native_log::Row` has no pages.
- `emote_messages.rs` + `native_m2.rs`: `#attribute` (910), `#touch` tables (1029/1030), `action.def`
  macros (1027/1028/1047/1048/908, with `restore`; `easing` is linear), lip sync 924 by `#lip_flow`
  and `#lip_table`/`#lip_variable`, ended at a marked step once `audio_playing` is false. Voices with a
  `.lv` table (see batch 10) drive the variable from it instead. 929/930 play a same-named PSB
  motion. 907, 931, 965, 1025 are no-ops: the shared player has no inertia solver, and
  `#pattern_shift` is commented out in this title.
- `cg_register.rs`: 419..425, 458, 459 and the FES `cgreg*`, `loadlist2` natives; the register's
  save record is this adapter's JSON, so it does not read an original game's savegen.dat record.
  cglist layout: {next, list no, count} then records {size, name[64], n, strings[n * 64 or 256]}.
- `fes_save_panel.rs`: SAVEPANEL (class 12) is now an IMAGE state; setsave, getsavestr,
  getsavedate, ptype SIMAGE (thumbnail placed by the panel bitmap's frame of that part id), dispid.
  241 writes PNG into the record, not HG-2.
- `fes_image_methods.rs`: settbl, pl, getbase, getoffset, setanm/anm (`.anm`: 32-byte header, count at
  +8, 68-byte rows with frame word 2, hold word 4). `arc` → motArc with 6 words. Still logged as
  no capability: attach. Inert: attr/refex (no `ats` tag in the images checked), setnum, mode.
- `particle_plugins.rs`: em_note, em_double, em_question, em_question_a, em_sweat, em_angry restated
  from their DLL code (vtable init 2070-style, update, Hermite path). `particles.rs` gained
  pivot/rotation/scale per particle (5F4DD0). Scenes use em_* 325 times. rain and dash are in
  `particle_rain.rs` and `particle_dash.rs`; blur, steam, filmnoise (`particle_effects.rs`) and cmnptcl (`particle_cmn.rs`) follow; see batch 12.
- `host_misc.rs`: 903 reads `pcm_tag.int/<sound>.tag` ({name[20], value} rows). 649/650: child FES state
  is not saved (logged once). 576/688/689 belong to class 43 (3D view), created only with
  `config.int/3DTable.txt`, which this title lacks; a real class 43 fails visibly.
- Grisaia: see batch 11 for `scene.int.FWP`.

## Batch 10 (2026-11-04): implemented after the first offscreen pass

- Plane trail modes 1..5 (`plane_effects.rs`): 5E2C70 turns and stretches only unkeyed copies, 5E1BB0
  halves the chord, 5E1F30 does not, and both stop before the last record. 5E22A0 (modes 4, 5) hangs
  copies off the path at -90 degrees, turns the Plane itself, and 5DFA20 sets the slice thickness. Width
  stands in for +0x160, which the original only has for rectangle Plane kinds. Not run in a game: no
  script reached here uses a non-zero mode.
- `native_classes/database.rs` replaces the empty 869..873/878 model. The table format is `KCDB` (magic,
  packed size, plain size, zlib of columns and fixed rows), not XML. Only INDEX and the flags are kept,
  since no native reads the titles. Neither test title ships a qdb, so only the unit tests exercise it.
- Tone table 594..597 (`tables_3d.rs`), parsed from 78D750. Unit tested; no data file in the test titles.
- `particle_rain.rs`: the `rain` plugin. Unit-compiled only; no scene use found.
- Voice levels: 5EAC60 loads `<voice>.lv`, 60 Hz floats, and 5A1F30 feeds them straight to the lip
  variable. Nekopara's voice archives carry `.lv` beside each `.ogg`, so this needs no PCM from the host.
  **Inference:** the table index is an argument not traced to its source; playback position is used
  because the tables hold exactly 60 entries per second of voice (301 entries, 5.01 s in XA_13_001).
  `.lpb` (message 220, `LPD` format) appears in neither title and is not implemented; such a file is
  ignored, not reported.
- Offscreen check (Nekopara vol3, `--headless --no-audio`, 16000 frames): exits 0 with no error, and
  story frames render with Backdrop in the build. A temporary log showed `face_talk` receiving table
  values (1.48, 4.39). Under `--no-audio` voices end within a few frames, so only a few lip ticks run
  per line and mouth timing against heard voice is **not** verified. Transition mid-frames and particle
  visuals have not been inspected. Native 521/523 (blur, mosaic) did not fire in those frames, so they
  and raster are compile-checked only.

Still open: inertia solver (`#inertia_*`, `#material`, 907/931), 965, 1025; plugins blur, dash, steam,
filmnoise, cmnptcl; `.lpb`; kx2 models (81); 3D view class 43 save state; child FES state save (649/650);
FES attach, attr, refex, setnum, mode; true mesh motion for 57/58/wbreak (crossfade timing now); `novel`
page flag; 3D sound paths 168/347/348.

## Batch 11 (2026-11-04): inertia, E-mote options, dash, Grisaia start-up

- `crysirror-puppet-eluna/src/inertia.rs`: reusable spring solver (5930C0, 594530, 594760, 5948F0).
  `native_m2.rs` feeds it the Plane base (native 84 → message 51 → 574410) and `#material*`, `#inertia_*`
  tables. It stays idle until the base first moves (574410 sets +0x160). Offsets add onto the variable.
- FES `m2_getconf`/`m2_setconf` (786360, 786840) over `M2Config` in `NativeSettings`. `iner_lv`,
  `iner_enable`, `eff_iner`, `eff_blink`, `eff_lip` take effect. `motion`, `eff_dif`, `fade`, `interleave` are
  stored only. Saved as `m2config.json` on the Save mount, this adapter's own form, not the original's.
- 907 is real. 931 (5743F0 → the motion setup's a5[11]), 965 (`#pattern_shift`) and 1025 stay no-ops.
- `particle_dash.rs`. Its active counter is per instance; the original's is a static shared by instances.
- FES natives: `fw_*` are empty on the owner's instruction; the scripts set `$700=2` themselves. `config` reads
  volumes (`vol_pcm` is mapped to `vol_voice` by inference). `opensite`, `opensiteenc`, `clipboard` log and do
  nothing. `lockactive` does nothing. `delete` hides and disables the member rather than destroying it.
- Bugs found by running Grisaia, both of which affect any title:
  - A repeated `#DEFINE` with a larger count (`PLANE pl[4]`, then `pl[6]`) now adds the missing members;
    the whole-array count stays the first one (7544F0, 755400). The old "first declaration stands" was wrong.
  - Button hits no longer require display: title scripts click invisible map buttons.
- `detect.rs` mounts `<name>.int.FWP` as `<name>.int`.
- **FWP resource decoding is now implemented** in `crysirror-formats-arc/src/legacy/e/int_fwp.rs`,
  called automatically by the existing `cat-system/int` resource loader for `.int.FWP` sources.
  Installer `0x482DE0` XORs the inclusive interval `[file_size/5, file_size-file_size/5]`,
  indexed by absolute offset modulo 40. The plugin recovers all 40 bytes from consistent,
  independent encrypted `CatScene` headers, without network access or embedded game passwords.
  Shared `crysirror_resource::io::xor_region` keeps reads random-access and precedes normal INT
  Blowfish and alias handling. The existing detection mounts need no configuration changes.
  Release compilation and reading/decompressing all 281 original Grisaia scenes passed;
  this is resource validation, not post-fix gameplay validation. No desktop binary rebuild here:
  the next ordinary player build includes the change. No new test files are retained.

## Batch 12 (2026-11-04): compile-checked only, nothing run, on the owner's instruction

- All `ptclpi` plugins now have an implementation: `particle_effects.rs` (filmnoise, blur, steam) and
  `particle_cmn.rs` (cmnptcl, reading `ptcl<a><bbb>.plist` from `ptcl.int`; no plist means no particles, as in the
  original). `Record` gained depths, colour, per-vertex alpha and visibility. No test title has a scene using
  them or ships a plist, so none has been seen on screen.
- cmnptcl deviations: the key-to-field mapping relies on the keys sorting alphabetically, which fits every field
  checked. Tangential force uses the current radial direction; the original's branch on the sign of the
  tangential value (+0x120) was read only in outline. Blend function keys are ignored. The plist location
  (`ptcl.int/`) is inferred; the original asks the host through vtable +128.
- FES natives added: bitcount; isdebug, inputdevice, check_install, vram_get (0); psversion (0x300, a chosen value);
  fps_get (the configured rate, not a measurement); setexvar/getexvar (session only); cursormove, touch_kill,
  touch_enable, enablebgbutton, dbgtime, exit_boot, m2_resetview, m2_waiteff, checktitle,
  steam_achievement_reset (log and do nothing).

- 649/650 (7582E0, 758280): a child FES's 512 local words go into the save record; loading restores them and enters
  `_LOAD_` when the script has it. The record is the same 2048 bytes as the original's.
- `m2eff`, `m2att`, `m2fspeed`, `m2prmscale`, `m2iner`, `m2enable_blink`, `m2enable_lip`, `m2splay`, `m2draw`: 778050..778230
  only count the array and return in this build, so they are accepted silently.

- `.lpb` voices (`lpb_levels`, 5AB280, 5A1B90): tried before `.lv`, as 6383xx asks message 220 first. No test title has one.
- `dic_*` over a fuller database model (titles, IND1..24, affixes) now kept in `NativeSettings`; no test title has a qdb.
- `sortsave`, `sortsave2`: newest first by the Save mount's modified time.
- Approximations: `fontcom` answers an empty font list (the host does not enumerate fonts); `cgreg_getlist` always empties
  its string instead of walking the CG list.

- 3D sound 168/347/348 under `<use3d>`: position and minimum distance become pan and inverse-distance gain for a listener at
  the origin. Velocity (Doppler) has no host control. An approximation of DirectSound3D, unheard so far.
- Screen meshes in `shader_tasks.rs`: wbreak (class 52, 5FDA10, 5FE650), page curl (57, 5FFA60) and cell flip (58, 601030) replace
  the crossfade. They draw through `PuppetFrame` textured by `backdrop:frame`, a new case in `crysirror-wgpu`, with an
  invisible Backdrop layer to capture. Known differences: the mesh samples the previous frame's capture; wbreak draws from a
  local generator, not the session random streams; 58's back face is left to what lies beneath; the curl's 16-pixel cell is
  chosen, since 5FEE20's cell size was not read.
- kx2 (81, 67CE50): meshes parse and replace the Plane's quad with UVs over the file's bounds. 681FE0's UV use was read only in
  outline. The file is read on the VM thread, against ADR 0050's rule for I/O. Grisaia's raster_f/x/y.kx2 have this layout.

The remaining-item list from this batch is superseded by Batch 13 below. FWP automatic decoding is implemented in the shared INT reader.

## Batch 13 — FES operations and shared image/text resources (2026-11-04)

Implementation:
- FES maketask/deletetask deliver original KCS messages 103/104 and string slots
  700/701; they do not call the native task factory directly.
- Resource redirects use client-scoped shared Storage state. Reads, metadata and
  native-file fast paths apply one bounded rewrite; write authority is unchanged.
  Empty native search strings fail instead of entering the original infinite loop.
- HG3 ats metadata feeds FES attr and natives 536/724. Image clip/tone/filter,
  ref/refex/copy, coordinate/key anchors, button attach/drag, sound time/tag waits,
  ismot and built-in arc/pend variants use actual resource and input state.
- Shared ImageEffect operations add fixed-point triangular blur and saturating
  coverage neighborhoods. Image jobs own all pixels; FES retains native policy.
  The 24-bit horizontal scratch bug is intentionally repaired: both axes write
  every RGB channel through the initialized shared fixed16 convolution. Native
  weights, truncation and padding remain; undefined native pixels are not reproduced.
- PANEL/SAVEPANEL use separate configured/live parts, original source and digit
  selection, shown-part order, setnum, scoped frame resources, glyph batches and
  immutable image composition. PNG save thumbnails return through shared decode.
- FILELIST make/get and BGTABLE loading/exist use scoped storage workers. Save
  panel requests retain original evaluated parameters across dependency polls.
- mag controls reveal timing, strid reads inline IDs, NVSTRING insert retains
  independently styled paragraphs. Novel page boundaries survive log rebuilding.
- E-mote 931 restores a shared playback checkpoint. Pattern shifts have independent
  curves/durations. Native 965 loads waiting ranges, maintains correlated groups,
  and consumes session MT streams on the owner after asynchronous model loading.
- The reviewed Grisaia/Nekopara factories return null for class 14/43. No fake
  3D task is allocated. Save 576/688/689 preserve their distinct missing-receiver
  results; unknown factories with a real 3D task remain unsupported.

Compilation: release checks with the runtime feature are recorded in STATUS.md.
Behavior: no tests, game runs or pixel comparison were performed in this batch.
No new test code or test files were added. docs/api was not regenerated: its
current generator is test-driven and tests were not run.

Remaining profiles, explicitly rejected rather than acknowledged:
- Foreign HG-2 save thumbnails have no shared decoder; CrySirror-written PNG is supported.
- PANEL zero-size font/GDI default-font behavior is not reproduced.
- FILELIST ambient Windows directory substitutions other than the scoped save
  root and BGTABLE2's non-CSV trailing-token extension are not implemented.
- Other native binaries with real class43 or additional bitmap setters need
  their own semantic profiles. These results do not establish complete games.

## Batch 14 — actual offscreen checks and live log/menu fixes (2026-11-04)

Native698 now shares977 insertion; its absent eighth argument is history index-1.
KCS format strings are expanded before log filtering. The plain history mode
removes native controls without treating CP932 trail bytes as backslashes; the
preserve-layout option is retained but its original global configuration source
is not mapped. Formatted scene/name/body remain bounded.

Menu40019 and40041 open shared modal information dialogs. Shared menus/dialogs
consume input before native journals without deleting receivers or invalidating
receipts; UI-held keys stay masked until release. Unknown commands such as40037
still fail explicitly. No external system utility is launched.

Final release compilation passed; real Metal offscreen runs exited0 for Grisaia
6000frames (698 count7/history inserts7), Nekopara15000frames (977 count14/inserts14),
and40019 open/Enter-close. Shared blur GPU readback matched RGB references exactly.
No physical OS-menu clicks/audio output/full playthrough/unit suite were tested.
Evidence is in target/verification/catsystem2/offscreen-current/summary.json.

The lower-left color block observed in this batch is fixed by Batch15 below.
Unresolved: final info-dialog line clipping and Ctrl stress performance dominated
by repeated E-mote scene rebuilds. The Ctrl run was interrupted and must not be reported as a pass.

## Batch 15 — MASK4 destination-alpha composition (2026-11-04)

The lower-left Grisaia block was sys_mwnd_fmask.hg3, a231x251 pink mask whose
FES object declares MASK4. Original687370's32bpp channel-mask kernel688EF0
multiplies destination alpha by source alpha, keeping straight destination RGB;
it ignores the ordinary blend weight, after743600's zero-alpha rejection.
FES now maps that policy to shared Layer/Canvas DestinationIn. The engine owns
the premultiplied GPU operation and snapshot support. Masks1..3 remain explicit
unsupported channel-selection profiles. This GPU mapping is not a claim of
bit-exact native integer-rounding parity for all alpha values.

Release compilation passed. Grisaia6000-frame offscreen run exits0 with seven
698/history inserts and zero former pink pixels; screenshot inspected. Shared
Canvas alpha0/128/255 probes on black/white match normal-alpha references within
one channel byte. No unit tests, audio or complete-playthrough claims.
Evidence: target/verification/catsystem2/offscreen-current/mask-result.json.
