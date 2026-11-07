# System-NNN implementation notes

Current implementation entry points and supported profiles are indexed in
[README.md](README.md). Native operation mappings and remaining gaps belong
beside their source; source provenance is recorded in SOURCE.json.
The dated notes below retain historical implementation and validation evidence.

## Production boundaries

- `adapter.rs`: real Core VmAdapter, move-only ScriptSession batches on TaskPool;
  native calls commit on owner and retain PC until exact receipt succeeds.
- `native_bootstrap.rs`: original MSI/config/SPT loader, reviewed module SHA
  `e85e92ec538d50eae08d27286177149972d443a96d1415277c1ae1f9ee108bdb`,
  controller/new-game policy and actual seed admission.
- `installed.rs`: NNN-owned dormant factory, resource/clock preparation,
  original windows and services, shared ImageJobs and resulting adapter.
- `resource_bridge.rs`, `image_bridge.rs`: original names/pack precedence and
  assignment policy, shared GPK/DWQ/VAW registry and Encoded ImageInput. Raw image
  domain preserves original alpha; background display normalizes opaque XRGB.
  No-mask foreground uses shared corner key; mask presence is source metadata,
  never inferred from whether its pixels happen to be opaque.
- `native_presentation.rs`: shared Canvas capture → persistent Pattern jobs /
  ImageJobs. `native_overlap.rs`: type0 immutable old/target Q8 crossfade;
  SUNA prints once then freezes throughout overlap. `native_draw.rs`: DRAW0
  prints once without CountUp; positive frame count prints/counts/waits.
  Base capture is reused for ensuing frames. CTRANS picture slots are excluded
  from base capture and composed once through public ImageInput graphs. Native full frame-debt behavior is
  not yet reproduced; only linear fade/easing profile supported.
- `native_skip.rs`: five explicit original skip flags; table-driven stopf5
  reset is configured from original function names, never hard-coded index2.
  Unknown functions retain name/index/PC. Original module0x5355c0 fully reviewed.
- `bridge.rs`: legacy16slots; Fill54 max9999, NOP0 max99999; natural load geometry;
  explicit active-picture preload fence rather than stale-byte substitution.
  Fill stays active when picture loads; replacing Fill restores assigned image.
- `native_text.rs`: original single-byte table read from reviewed module,
  Shift_JIS and shared glyph/reveal; first click completes reveal, next edge
  acknowledges. Native window percent50 uses shared MaskPiecewise256 and
  cropped immutable image nodes; text remains separate GlyphRuns. Message wait
  with ongoing screen effects explicitly unsupported until continuous frame
  recomposition policy is connected.
- `native_ui.rs`: original CASE selection over shared UI retained events;
  candidate variables validate before atomic owner mutation.
- `audio_bridge.rs`/`native_audio.rs`: scoped worker source resolution, shared
  audio decoder/mixer, actual dB unit conversion; stepped fades, finite repeats
  and spatial music profiles unsupported.
- `native_save.rs`: validated original binary envelope preserving padding and
  opaque fields. Complete VM/resource restoration still fails explicitly.

Shared additions live in image temporal_pattern/rgb_lut/fixed_mix/bitmap,
Core PatternResource/ImageInput contracts, Runtime image/pattern/canvas resource
workers and Scene GlyphReveal rates. Parent regenerates API docs. SOURCE.json
records fixed-source/static-module facts; public source has no license grant,
so its implementation is neither copied nor linked.

## Actual validation, separate from implementation

- `native-draw-effect-policy-check.log`: release check passed after DRAW/FILL/NOP
  and picture assignment policy changes.
- `native-image-prefix-overlap-tests.log`: 7 authored bridge tests passed before
  final DRAW extension; real engine Pattern/Image/Canvas/audio resources, known
  fixture capture provider. Not original GPU game output.
- `native-window50-services-tests.log`: 4 authored native service tests passed.
  `native-window50-font-actual.log`: explicit system-font window50 test passed,
  mask0/128/255 exact RGB255/190/128 plus independent GlyphRuns/click/reveal.
- `owner-bootstrap-core-actual.log`: previous true Core-port original-data test
  passed 2/2; sys → preprologue, 65 original instructions, 8 engine calls and
  1 frozen Canvas, next boundary PC0x1bc at the then-unimplemented OVERRAP.
  The current installed startup has not yet reached that point because of the
  mount issue above.
- `owner-blackcyc-final.log`: prior standalone actual loose-pack decode passed
  1625 DWQ and 156 audio wrappers / 2,555,904 PCM frames. The newly extracted
  shared GpkIndex parser was then revalidated against the same full actual packs
  in `owner-blackcyc-index-regression.log`:1/1 passed, identical counts.
- `native-draw-tests.log`: new DRAW0/2-frame authored behavior test passed1/1;
  exact shared Pattern steps/global RNG consumption, post-Print effect counters,
  immutable display Layer and unknown next-PC retention were asserted.

- `native-stopf5-check.log` and `native-stopf5-tests.log`: release check and
  actual native function receipt test passed1/1, with relocated table index0
  and unknown name/PC retention assertions.

Next: use a unique parent-built player containing CTRANS, poll actual admitted
dependencies through the separate pump instead of wasting simulated frames, and
fix each next original boundary while retaining engine ownership.
Original title, Start/long story, ongoing input and original saves remain required.

## Resumed implementation / validation

- `native_shell.rs`: reviewed Nothing/Logo/Opening mode/config/skip/timer policy;
  shared MediaProbe proves only actual video decoder/header capability, without
  inventing the original DirectShow numeric3/4 or full audio decoding.
- `NativeAdapter::poll_dependency` and `InstalledAdapter::poll_dependency` drain
  admitted work only; no fresh VM batch, input sample or clock advancement.
  Core generic maintenance integration remains parent-owned. Diagnostics cache
  original script/depth even while a worker owns the mutable session.
- `native_geometry.rs`: six original rect fields and plain Layer crop/UV mapping.
- `native_chara_trans.rs`: original fourteen native parameters and persistent
  endpoint. Resource bridge separately retains raw/opaque/display engine leases;
  mask presence is header metadata. Masked/unkeyed raster work remains shared.
- `native_effects.rs` LayerGrey129 maps exact original grouped luma to public
  `crysirror-image/fixed_luma.rs` / `ImageEffect::FixedLuma`. No VM pixel loop.
- `native-rectangle-check.log`, `native-ctrans-check.log`,
  `installed-dependency-check.log`: release checks passed.
- `native-ctrans-tests.log`: lib5/5 + bridge12/12 passed. Independent resources
  assert exact native mask0/128/255/unkeyed RGB, immutable input/base, unchanged
  global RNG and existing Draw/Overlap/resource behavior. No actual GPU claim.
- First `native-shell-completion-tests.log`: installed2/2 passed; shell3/4,
  new pending-click test found removal before first layer publication. Fixed by
  removing only a published layer; retry in `native-shell-completion-tests2.log`
  passed installed2/2 and shell4/4. The pending-click fixture retains one real
  edge through source/image jobs without creating/removing an absent Layer.

## Next Opening effects implemented after player22

- NativeMAE1 was then fully traced through actual wrapper0x4c6660 ->
  SetEffectScript0x4c18e0. It returns before SetParam without changing command,
  counters or resources; explicit keep-current policy is implemented. Release
  native-mae-test.log passed1/1, asserting previous Fill state/count and identical
  engine state/API call count. This is not a generic unknown acknowledgment.

## 2026-11-01 actual player24 and ordered painter follow-up

Immutable `target/verification/krkr-ctrl-exits/player24`, SHA256
`f6cc7fc86de4ed35650c510ecdda07cc2007900f0876500727295da611ef8d98`,
ordinary Controller boot20k frames reached141 original instructions/PC0x6c2.
The common published-resource capture fix therefore passed the prior25/0x397
frontier. Next exact error: interleaved image/effect bands. No title or final
capture was produced. Log: player24-controller.log. GPU released to Cat/KRKR.

## Further source-only startup work before player26

Full original background SPT static inspection recorded1224records under ignored
backscript-all-records.log. This is format inspection, not execution coverage.
Opening's next missing native effect is FadeOut11 at0x86d; other branches also
contain Move52, Buffer66 and Hotaru71, which are still unsupported.

FadeOut11 now reuses the existing public weighted-colour lookup, with native
progress direction and CountIsMax changing the effective native command to Fill54
without resetting original count/max. Terminal Fill is exact byte colour rather
than a Q15 endpoint254; it disables picture-underlay drawing and persists for
DRAW0. Fade Print treats g/b=-1 as inheritance; terminal Fill masks each original
parameter separately, so [10,-1,-1] changes from weighted grey10 to [10,255,255].
This difference is explicitly tested. No new shared enum/API or pixel loop.

FadeOut dedicated releasecheck then passed (`native-fadeout-final-check.log`,
9.79s). The next player must include both ordered painter and FadeOut.

## Actual immutable player26 — 2026-11-01

Artifact `target/verification/krkr-ctrl-exits/player26`, SHA256
`c67df6c247e15f6b4ec72a222d3c4d9c87b33828bfbbca2f7d6ad8d531166897`.
Ordinary Controller20k run naturally failed at211original instructions/PC0x8a7,
code0x3333001a/parameter1. It passed the old141/0x6c2 interleaving boundary and
actual FadeOut11 before this point. Log player26-controller.log; no final capture
or Title/Start/full integration claim. No owner process was terminated; GPU was
released to Cat, then KRKR.

This callback is SETFILM. Complete pinned SystemFunctionSetFilm and original
0x5324b0 both suppress the read-history write when back-script mode is active
and backScriptSetCGOkFlag is0; otherwise they write script-index/film-minus1
into original SystemFile. Next work must implement that explicit known native
policy, not acknowledge all SetFilm or invent a shared read-history write.
Normal/front-script original persistent marks are still unimplemented.

## After27 source: exact background mark policy and frame returns

ScriptSession background profile now preserves ENDFILM/ENDKOMA/ENDIF receipts;
normal Yoyaku still resumes ENDIF/ENDKOMA immediately. NativeDraw accepts the
original Control receipt and performs one shared frame, CountUp and native wait
before ACK. This follows complete CBackExec ExecScript/Calcu/Print and actual
CalcuBackScript/LoadBackScript source; it is not a timer-only ACK or a fabricated
DRAW opcode. No public Core/API variant, private renderer or pixel loop added.

## Stable source for unique28: shared masked pair + final merged docs

New public immutable three-input image operation is complete and independently
verified: image/masked_pair.rs MaskedPairBlend, Core ImageInput::Ternary,
Runtime existing worker, Bitmap.beginMaskedPair. Source facts audit:
docs/nnn-masked-pair-source-audit.json. Sources/clip retain phase; independent
14bit coverage/product flooring preserves destination alpha, weights sum<=256;
immutable aliases, owner/epoch/revision/cancel and existing budgets remain.
Release kernel3, Runtime6, public bitmap effects5 all passed. Native Title has
NOT yet adopted this operation. No actual Title success claim.

Final single API regen included Root CgCatalog/Entry and Cat PagedImage's4types,
2constants plus already-tested byte_groups. Fingerprint1171d6b86d446b00 (previous
c4f0841c769d7a18), CHANGES/README actual semantics. No-bless release api_docs8/8
and api_coverage1/1 passed in masked-pair-cgm-paged-api-docs-verify.log. Root
notified source stable for unique28; production FREEZE until its SHA fixed.
Next actual28 should carry after27 background mark/control returns and reveal
whether original Opening now completes. Current explicit completion guard is
still installed.rs: printmovie/Title continuation missing. Do not bypass it.
GPU all peers released at this point; Root may use28 first for Siglus/Circus.

## Actual28 and stable source for29: native Movie transport

Immutable28 SHA b6d6b7479ebad8d8d599b1a0a47c8251eb59daa204168864ad230979713dee8f
ordinary20k Controller naturally exited1 at original Opening-completed / missing
Movie-and-Title guard. Log player28-controller.log has no final instr count;
no invented count, final capture or Title success. GPU released back toRoot.
Thus real SETFILM suppression/control returns reached original NEXT.

After28 native_movie.rs + installed routing now map original Movie selector,
mode27, optional movielist pair fallback, shared Media Play/imageLayer/Remove,
pointer/noSkip and post-stop wait. Actor starts only after actual Opening NEXT.
NativeOuterState owner-holds machine/bridge/configuration/scoped loader. No
NewGame or VarInit. NativeStartupShell diagnostics enter Movie too. Actual old
Movie constructor/Init/Calcu decomp were reviewed (4fe2a0/4fe5b0/4fe010); don't
copy later source FillScreen calls. FillExitScreen is saved-buffer metadata,
not live screen clear. Title continuation still explicitly errors.

Release finalcheck passed (native-movie-final-check.log), finaltests12/12
(installed2/movie5/shell5, native-movie-final-tests.log), including calculated
selector with stored0 vs mode27, named lookup fallback, native volume endpoint,
transport phase/frame publication, skip/noSkip and wait. Test host models Media
receipts; do not claim real decoder success from them. Sourceaudit is
docs/nnn-native-movie-source-audit.json and SOURCE reference. Music default fade
computes original100ms stepped decibel policy but existing control_request
explicitly fails when that nonzero fade is needed while music is playing;
shared Host only has a linear fade. Do not approximate this in native code.

## Stable30: actual29 Movie continuation and real shared mouse bits

Actual29 SHA24218d166db484803755533a3518625cc7f6e50747007c692753b1d72a0b4850,
ordinary20k MUGEN2 exits1 at original Movie completed toTitle / missing Title
UI+background guard. player29-controller.log does not expose selector/decoder
branch; don't call actual OP playback proven. GPU released.

After29 fixed own adapter trig/click, Logo and Movie pointer maps from incorrect
low1/2(bitsEnter/Space) to MOUSE_LEFT/RIGHT16/17. Trig3/click3 remain unbound and
explicitly fail when read (Core currently has no middle-button logical input);
Left arrow no longer becomes middle mouse. Installed text advance is MouseLeft
orEnter, notSpace. Original CheckClickKey maps Space toWINDOWOFF and Enter to
MESSAGEOKURI. Release check6.28s, Movie5+Shell5 passed in
native-input-step-tests.log (Core worker tests actual input mapping). freeze30.

Preparatory native_title.rs parses original menu button copy/default/picture
animation layout and state counters, but is NOT yet wired to resources/UI.
Current frame getters return engine-owned immutable Arc handles, including
last actually published Draw/Overlap frames; new from_outer_background carries
one original context/loader/RNG and instruction base, unused until presenter.
Known before wiring: reject or correctly handle reverse-animation pattern1
(degenerate native signed negative index; do not allow u32 underflow); validate
source/header/geometry bounds on restore. NativeTitle actors still to implement.
Next Root30 will contain generic failure JSON; can read true Movie diagnostics.

Next Title design: source tasks NativeResources→ImageJob; no-mask uses shared
CornerKey, mask-exists retained (never infer from opaque()). Shared
ImageResource.extract_alpha_rect + WidgetHitMask threshold1 matches original
CheckOnPic(mask!=0). Existing UiInteraction/activate_on_press retained events
supply input; never private pixel hit loops. Normal native button type5 uses
shared MaskedPair, intro fade uses MaskPiecewise256 and frozen frame0. Compose
bounded button crops in parallel then one shared full-frame Composite, publish
immutable complete frame; don't let new background under old button matte tear.
Input/native frame counters/clickTime/autoRestart and optional controls stay
native. L/C accelerators currently missing in shared logical key API; parent
notified, shared shape pause remains. Native SystemSound uses nya/systemse.xtx
and sys wave lookup + two alternating channels, NOT script spt/selist/se.
Only native Original VM data/layout/wait/save policy belongs here.

## Frozen31: installed actual Title resource/VM/widget/image pipeline

Actual30 failure JSON now proves exact native state: frame5862/time93792ms,
instructions231/pc464(0x1d0), native_movie assetnull/skipped_by_original_variable
true/finishedtrue/apiCalls0; Media empty and channel_audio active0. Thus Movie
skip is legitimate original selector0, not decoder completion. No OP claim.

New installed Title pipeline is wired, not only helpers: original sprite/source
workers→public DWQ/ImageJobs→declared-mask/CornerKey preparation→common retained
widgets+alpha_region→cropped MaskedPair/Composite workers→immutable whole-frame
resource/public Image Layer. Original mode4 SPT restarts via worker loader with
same bank/RNG/bridge and no VarInit. Scope/clock/resources validated. New outer
music driver shares AudioBridge policy and actual engine audio receipts; it
never fabricates a NativeCall. Underlying engine pixel operations are usable
without NNN (already independently verified); no native pixel renderer exists.

native-title-installed-check.log passed11.29s; authored native-title-tests.log
2/2 passed real common image/widget operations, exact two-source pixels, older
immutable resources, alpha-hit pass-through and clickTime1 after final frame.
It is CPU-only and not original-game behavior. Source audit
nnn-native-title-source-audit.json. Root builds unique31 now; production FREEZE
until SHA. Do not claim actual Title until that artifact runs original data.

Remaining explicit: system sounds (nya/systemse.xtx/sys waves/two alternating
channels, not script selist); L/C hardware key capability; original action/Start
exit and quiesce; autoRestart exit; some background native effects; live message
recomposition and complete native saves. Title first-click sound currently
errors rather than pretending success. NewGame remains unforced.

Potential next: original Title Init ClearWork/LayerOff semantics need exact old
module mapping; current known fresh Opening path has original mode state, but
no modern-source-only variable-clearing loops were imported. For Start, queue
mode change until admitted background work reaches safe frame boundary. Do not
ack/drop an unknown native instruction; an ongoing known background DRAW may
legitimately be discarded only by explicit original outer-mode cancellation,
transferring RNG from the last actually published frame. Actual clicked Title
frame must become the new original exit frame through common ImageResource,
not private capture or stale background-only resource. Native helpers expose
completed immutable Draw/Overlap handles; no middle of an image job is a frame.

Root Core integer_fields docs final regen8/no-bless8+coverage1 passed; actual
fingerprint stays c8c180ba4ff65706 (no new DTO/class). Source registry/README and
CHANGES note this. No outstanding API docs queue from this agent.

## Actual31 / Move52 / latest frozen combined-validation source

Actual31 SHAeb4bb341fde3b473690ed09aac5480cbd553d52bc2cc313bda8becfc95648f31:
plain original20k Controller failedframe8220 at261instructions/pc0xe39,
callback33330013 operands[3,0,52,300]. Titlemode4, spritespreparedtrue,
sharedmusicreadytrue, widgetscreated, Titlecounter70/api216; full menuWait200
not reached, no finalcapture. player31-title.log contains Root generic JSON.
GPU released. This is genuine Title pipeline progress, not full Title/Start.

Move52 now implemented solely as native rectangle/count policy plus existing
shared Composite. native_move.rs uses signed nearest-away/matching MSVC wrap,
XY interpolation only until CountIsMax; hidden Calcu still runs when a picture
exists; Print happens before CountUp; end copies current/From rects toTo and
command0. Zero/negative countMax does not fake CountIsMax. NativePresentation
MoveFrame uses common image graph, no native pixel loop; Draw returns completed
move state/rect updates. Bridge maps original52, excludes duplicate underlays,
retains terminal picture and restores engine plain layer before replacing it.
Release native-move-check passed; native-move-input-final-check passed4m40
(includes locks). native-move-tests.log lib11+bridge18+CompositeMove1=30passed.
A later hiddenMove test was added but awaits combined run; don't count it yet.

Cat shared Core now suppliesKEY_C49/KEY_L50/MOUSE_MIDDLE51/KEY_Z52 and partial
windows_virtual_key(u32). NNN binds actual middle mouse now (never Left arrow),
Title held L/C accelerators use shared mapper, mapped-status diagnostics replace
old unsupported note. NativeInputTitle final7/8-style regression task50503 FAILED
because it read an editing intermediate signature (Installed2args vs Title1);
that transient is now fixed to2args. Do NOT call that log pass. Rootcombined
will verify the final source. No native OS polling was introduced.

Next whole native system-sound leaf is SOURCE-LANDED but NOT YET COMPILED/TESTED:
SourceKind::SystemSound maps vaw/sys, optional startup nya/systemse.xtx,
noSystemSeFlag/systemSoundMulti, same AudioBridge with2 alternating channels64/65,
native sound enabled/gain, shared begin_audio decoder receipts. It never uses
script selist/se or overwrites music policy. TitleFrame sound queue gates final
frame publication and click action until actual shared audio Ready. New actual
Core WAV/alternating-channel lib test has been written, unrun. Delayed sounds
still explicit unsupported; Start/exit still explicit. Dependency pump now
handles admitted native VM work plus outer music/system sound/frame pipeline.

## Unified release result after source-only freeze

New audit docs/nnn-native-newgame-source-audit.json records actual Title vtable
0x566b9c, Calcu4dff40/FinalExit4df680 and callback InitNewGame53a710/SetYoyaku4010e0.
IDA was static only. Real old clear effect defaultsFALSE retaining picturecache
and previousEffect metadata; Init resets vars/controllerPC/stack then sets pending
exec, never mode0. Final clicked fullTitleframe is the native exit resource.
Start/cancel behavior is not implemented yet. Read these facts before the next
atomic Title-to-Controller work; don't clear unrelated original caches or assume
later source-only fields exist. Shared Image/Layer/UI services own all pixels.

## API catalogue final verified / native Start source pending verification

Root-owned api-docs-bless.log observed actualbef9ef5b2845175f; CHANGES now records
it plus the intermediate112d value and previousverifiedc8. Root no-bless
api-docs-verify.log passed api_docs8/8+api_coverage1/1. Includes shared Input/
Platform/C-L-Z-middle, ReadingWait and ALD registrations, no fake SDK/hardware
claims. No pending docs queue from this agent.

Whole legacy SystemFile static audit now docs/nnn-native-system-file-source-audit.json.
Read complete commonSystemFile.{h,cpp}1577lines; actual RTTI0x553184 ctor458b70,
Load458c80/Save459640/CreateInit4590d0/limits459c90/SetFilm459b70/Check459c00.
Actual module writes10records; modern14/16 block layouts differ. Existing
NativePreferences.original preserves raw bytes for next native module batch.
Reuse Core.integer_fields for generic bit arithmetic, scoped Storage worker
for native-save I/O. Foreground SETFILM still Unsupported; no fakeACK. Complete
maincontrol.cpp is not yet read (only startup count block read), so finish that
review before hooking count initialization. No new SystemFile Rust code yet.

Root next-native-tests3.log targeted50/50passed after the import fix: lib15,
Bridge18, installed2,Move2,Movie5,Shell5,Title3. This is a targeted subset of the
prior complete67/8 suite, not full70 rerun. Actual game remains31.

Root then authorized one source-complete small follow-up, now source FROZEN:
- Actor first clicked button locks other inputs, including simultaneous C/L;
  exactly one last group event/sound is returned. Verified original461d80,
  not the differing commonButtonGroup priority implementation. Native
  menuAnimeAdjustFlag resets all group animation counters (default0).
- NativeTextBridge tracks window layers apart from glyphs; NewGame hide removes
  both, preserves rows/bodycells for later APPEND. A prior glyph-only CLR still
  permits removing its retained window. NewGame no longer calls accept_clear.
- title_newgame_requested participates in dependency_pending (owner completion
  can drain without clock/input/newVM batch).
- actualMugen530350 always stops previous UI sound. Legacy startup multi=false
  because the later systemSoundMulti key does not exist in this PE; retain
  generic AudioBridge multi policy only for future separately-reviewed profiles.
New tests authored: native hide/engine layers/rows/clock (lib15→16) and actual
shared widgets/images dual-held accelerator lock (Title3→4); Shell existing
startup fixture now proves later multi key does not change legacy policy.
Root to schedule short lib+Title+Shell+installed+Bridge run; none added here.

MainControl.cpp2084lines now completely inspected (including native Game and
MultiThreadGame/End save lifecycles; do not copy its Win32 host). SystemFile audit
updated from partial startup review to complete file. Corrected audit wording:
legacy SYSTEMVAR128 differs from later1024 and extra record families; no claim
that the CG payload is enlarged (its320words match old1312-byte record).
Also exact original Menu CalcuSuper461d80 and UI sound530350 statically inspected,
ignored native-ui-group-*-static.log/C*.c. Source NativeRNG seed namespace needs
future mainControl vs game table separation; actual defaults-1 unaffected.

Final native small-batch verification: Root native-batch3-lib-fixes.log lib16/16
passed after author fixture supplied all16px layout parameters (full_rows3,fonts4)
and required name-color table. Exact old-binary backtrace is retained in
systemnnn/native-hide-exact-backtrace.log. Native production validation never
relaxed. Root frozen nnn-batch3 binaries ranBridge18/Title4/Shell5/installed2=29,
allpassed/noignored. Current selectedtotal45passed, not complete70 rerun.

Root now builds unique32 includingSystem4. Native source MUST STAY FROZEN until
SHA. No original32 behavior yet. Actual31 remainslatestMUGEN2; allother claims
separate. ForegroundSETFILM/SystemFile/readmarks, saves, someeffects/menus/fades
still explicit. Next SystemFile/nativebootstrap batch must use shared integer
fields and Storage workers, not private generic algorithms or unknownACKs.

## Actual immutable32 single original-game validation

ArtifactSHA783940ef4a493cdebe59c04172191887952257a5072b19200d97f897c0abcd0c.
Command ordinaryoriginalController/headless/noaudio/20k/capture; session59725
finished exit1, GPUreleased. 348instructions/pc0x1d0, observed original Title
backscript halted naturally (NEXT), no worker/resourcepending. Title2568,
sharedwidgets5enabled, masks2700..3300nonzeropixels each /8820, center255;
BGM1active, rendered12,807,168audioframes/0underrun. Source Movie video header
verifiedMPEG, movieaudio notverified; noactualOPplaybackclaim (originalselector0).
Atframe17150/time274400ms explicitoldguard autoRestart→nativeOpening unsupported;
queuedframe2569butlastpublishedcounter2568. No finalPNG, noTitlevisualcheck/Start
claim. Do not patch this frontier or continuouslyrerun; fulloutermode/source
work scheduled after completeSystemFile/Boot/originalsave batches. Parentknows.
Useplayer32-title-summary.json rather than dumpingfullruntimeUI maskJSON.

## Current source after immutable33: SystemFile/Boot/close module batch

Installed samples CivilTime first, prepares resources/SystemFile on its worker,
validates authenticated PE font-name literal+zero-fill defaults, derives native
project/cglist limits, overlays saved/template bytes and imports200..299 before
original execution. CGame empty-name defaults/clear flags preserve padding.
Bridge SETFILM now stores actual project-index/film-1 with original background/
skip gates. Other SystemFile flags have complete native record methods but their
full native consumer families/calculated getters are still not all wired.

Host request_close now optsin only with original VM/outer state. New VM batches
stop; existing Draw/Overlap drains one shared frame/RNG boundary. A separate
shutdown admission cancels known Message/Selection waits without advancingPC or
producing a dialog return. Unknown callbacks still fail exactPC. Scoped audio
preparations release and owned channels stop; other client's samechannel lives.
Close waits separately for real CivilTime + atomic SystemFile write twice,
matching actualGame52d6b0 and End52c6f0; saveCount increments twice, halt afterReady.
Native_dataN original-slot import/export remains unsupported and must NOT use
this SystemFile/global state format as a replacement.

Final narrow release57passed/1ignored (lib28,bridge18,installed2,shell5,title4),
log native-system-file-close-final2-tests.log. Initial54beforeclose +56before
strongerforegroundtest are historical only; final57 is the currentselectedset.
First sourceclose test extension exposed real Bridgebackgroundguard; explicit
shutdown path fixed. No test/production weakening. Constructor ignored test
explicitlyran1passed via CRYSIRROR_NNN_TEST_MODULE=.../mugen2-static.exe; only data
read, no original process or font file. Coreweekday3passed/Python201passed.

Shared API docsactual1cae8f9410b1b5b9 (priorcbe5), no-bless8+coverage1passed;
registered ImageEncoding Rust DTOs, optionalGPL text-legacy6types, ResourceMT,
Motion13 and weekday contract. Peers own their actual new module code/tests.
Docs/api source and README/CHANGES stable. No desktopbuild/GPUgame validation of
this newerNNNsource; immutable33 untouched. Sourceaudit now includes fullnative
hostlifecycle and exactvalidation facts. Next complete source families: remaining
SystemFile progress/getters + commonDataFile/GameStatus/four native contexts and
allEffect/message/audio reconstruction, then remaining fullouter modes and one
unified original-game acceptance. Do not return to one-error/gamefrontierloops.

### Subsequent private native dataN context batch

native_save.rs now has worker-only prepare_script_contexts/prepare_stored_variables
(private), native four active contexts/original saved fields472..475/488+256*n,
249-call limits, native tables20(sub)/22(command)/23(script). Native adjust1
parent PCs ignore obsolete rawPC and use stack253; active non4/9/10 uses254.
Raw adjust0 recovers CALL IDs in one cancellable record scan. All fixed/name/ABI
checks are staged before loader calls. SetVarByLoad intentionally keeps live
200..299/globalRNG; don't overwrite them from a slot. Machine retains last typed
script/command IDs on exact ACK only, with private constructors/getters; no
framework snapshot schema changed because this VM still rejects snapshots.

Actual final narrow release45passed/5ignored (lib32,machine3,native_control6,
native_services4), native-game-save-context-final-tests.log exit0. Four contexts
are genuinely rebuilt and the active original instruction is polled in fixtures;
no live adapter/resource swap is admitted. General-mode command adjustment,
corrupt/depth/table/cancel rejection and unchanged source bank/savebytes tested.
Native resource/effect/text/audio/original-slot UI is the NEXT dependency; do
not turn original_save_import_is_transactional on or register a fake importer.
Source audit docs/nnn-native-game-save-source-audit.json has exact static/read
coverage and ABI offsets. No shared API change or Desktop/GPU since1cae docs.

### Subsequent saved EFFECT/image family

Read complete effectstruct.h and4 full CGame effect/name save/load routines,
staticlegacy534390/5342f0/534640/541220 confirms16x1024raw+16x256names.
SavedEffect retains256words/name; rects3/7/11/15/19/23,counts27/28,
params29..252,notTrans253,previous254,pad255. Unknowncmd is just native data,
not a successful effect. New private native_save_images::SavedPictures validates
allnames/scope then serially prepares enabled+named slots, leaves warm unnamed/
inactive ones alone. Normal ImageBridge now wraps the very same private
PicturePreparation workflow; no fake NativeCall/duplicate image algorithm.
Raw/opaque/display all shared ImageResources; resources/client must match.

Final release52passed/1ignored (lib34+Bridge18), native-game-save-image-tests.log;
actual sharedDWQ/PNG/ImageJobs/alpha/scoping/errorretention/unchangedScene+clock
verified, including ordinary LOADDWQ regression. No Source/game/Desktop/GPU
acceptance and no importer enabled. Root was notified of the common boundary:
Core VmAdapter.import_original_save has no HostServices; Runtime transactional
branch returns completed immediately. A real async prepare/status/commit receipt
needs resourceReady and scope/epoch/cancel before VM+Scene replacement, with
no playback/scene/clock mutation during prepare. Do not work around it by
stashing bytes and declaring transactional import successful. Meanwhile next
native families are semantic EFFECT reconstruction, text/selection/audio/slotUI.

### Native builtin persistence/visibility and foreground control family

Complete ReceiveUserFunction0/SaveSystemFile and YoyakuExecRoutine reviewed;
actual5355c0 dispatch and5330f0 save callback, hooks1028/1032 both403030 empty.
Foreground ENDFILM/ENDKOMA/ENDIF immediately resume; skip film-end resets its
four flags, preserves F4/F5. Background still waits for a real shared frame.
Scene replay ENDFILM is not ACKed. Selected release67passed/1ignored in
native-foreground-control-family-tests.log.

savesystemfile now resolves original function table and waits for real scoped
CivilTime + atomic storage. Native candidate arguments commit only when Ready;
stale receipt can't write again. resultFlag rejects before any mutation until
ResultData exists. Resource-write-bound argument candidates reject before any
I/O (an atomic native owner plan is still required). layer(slot,onoff=1) calls
actual Layer visibility, maintains native mask, and excludes hidden slots from
next engine Canvas capture. Only defined0..31 shift domain accepted.

Final selected lib39passed/1ignored, native-builtins-system-save-layer-final-lib.log.
The first run36/2failed/1ignored exposed authored fixture assumptions (same-game
save roots and Snapshot CanvasPixel needing a raster backend), corrected to
distinct game identity and actual retained Snapshot-node assertions. No GPU or
private screenshot renderer was added. This is not full System-NNN completion;
remaining whole-source families include original semantic effect/text/audio/slot
restore, ResultData/notice/general native functions, all native outer transitions
and actual complete play acceptance. Do not enable the original-save importer.

### Native current-script date / week / holiday family

Private native_calendar.rs retains only original date/year IDs and immutable
native holiday tuples. Variables reads week/holiday from CURRENT stored bank on
each expression read, not an owner-batch cached value; clone retains tuple data.
Gregorian weekday math is shared Core civil_weekday. Boot worker reads original
nya/saijitsu.xtx only if original key useSaijistu is enabled (spelling verified
in0x542180). Missing NameList input means zero tuples; parse/other I/O fails.
Original count/3 ignores trailing one/two tokens. Legacy year is always bound;
expanded notUseYearVar uses fallback2000. Invalid Gregorian inputs explicitly
fail at checked profile instead of reproducing invalid C arithmetic.

Complete GetCalcuVarData/GetWeek/CheckSaijitsu/SetSystemVarNumber and saijitsuList
files reviewed, authenticated static5335e0/534210/5335a0/452730/4528f0. Actual
legacy5355c0 DOES NOT have later adjustdate builtin, so don't add ACK just from
newer public source. Calendar final release lib42passed/1ignored, log
native-calendar-family-tests.log; Python tooling201passed. No Desktop/GPU/game
acceptance; remaining systemclock fields need exact per-read async continuation,
not prebound batch constants. Other groups (wheel/progress/config getters) stay
explicit. Root original-save async prepare/commit boundary remains unimplemented.

### Native calculated preference groups, source-complete legacy binding

Actual GetSystemParam vtable232->534b30 full mapping confirmed, SYSTEM DATA
expMode at1792/expCheck at1856 (object+68 record origin). Both arrays store16
entries, but actual542180 binds ONLY5 expmode names and5 expcheck names. Do not
turn all16 names into calculated variables from the array size. Bridge freezes
these owner-held values for the current NativeAdapter VM admission; other names
retain ordinary words. No UI configuration can mutate this file concurrently
inside that single native worker batch.

Final release lib43passed/1ignored (native-calculated-preference-family-tests2.log).
New test executes real worker postfix assignments for all10 getter bindings from
actual encoded native save bytes and checks sixth-name fallback, unchanged
source storage words/clock/Scene/API count. First test authored fixture compiled
against immutable Arc bytes and needed to_vec for its own mutation; no production
relaxation. No new player/Desktop/GPU run, no full calculated-service completion.

### Shared native restore boundary available; native consumer remains incomplete

Core native_restore.rs and Runtime native_restore.rs now provide scoped async
begin/poll/status/cancel/release/commit, target-only VM holds, restricted readonly
resource preparation, invisible quota-reserved immutable images, worker scene
validation and atomic owner installation. ADR0099 and docs/ASYNC_SERVICES.md
contain the bounded contract. Independent Runtime6/6release tests and Python201
passed; API docs8+coverage1 passed actual feb2ec9be29523ba.

This supersedes the older statement that the generic boundary is unimplemented.
NNN has NOT yet opted into it or enabled transactional-original-save flags:
script contexts/effect/text/window/audio need a complete detached native seed
and accurate original restore order. Current shared coordinator explicitly
rejects audio/World replacement and remaining restored-client external producers.
Full dataN therefore still requires a real prepared per-client Audio replacement
ownership/install service; sequential requests or a cloned live mixer are not
acceptable. Continue full General/Opening/Title/Mode-return source families and
native save policy; no new Desktop/GPU/original-game acceptance since prior32.

### Native outer returns / actual Title.Init source batch

Title autoRestart/script-END returns are held until the exact shared composed
frame is ready; button selection precedes both. Nothing/Logo/Opening reentry
retains original variables/RNG and SystemFile RAM, never VarInit. Opening uses
left/right click or real END, directly returns configured next, and drains
admitted work without forged ACK/NEXT. Do not interpret Opening's ignored fade
config tokens as ReturnFadeOut. Mouse edge that finishes Logo is not reused in
Opening.Init. Shared immutable cover and exact terminal Logo fill prevent an
old background from showing through while the next async frame is pending.

Actual static evidence corrected older assumptions:534b00 posts WM_CLOSE;
52f5f0 increments launch only once per process and clamps0x7ffffffe;52f4c0 stops
8sound/4voice;53fa30 clears work-row enable words, not all parameters/variables.
StopVoice+116 is empty in this PE. Source audit records these facts. Do not copy
later Title.Init variable/name/voice loops blindly. General zero-fade ordinary/
special selection is validated, including conditional special capture flags.
Nonzero General effects/delayed voice/Load/Config/Omake/other return consumers
and complete backdrop/backlog/cursor/window policy remain gaps.

Actual native-title-init-return-final-tests.log67passed1ignored. Root later
flow-bounds-native-batch-tests.log revalidated lib49/1ignored +native_shell5.
An authored4pixel sheet for4x2pixel states was corrected to8, never by relaxing
native bounds. No new player/Desktop/GPU/title capture/Start/story acceptance.

Shared audio original-restore foundation now supports pure prepared replacements
through Core AudioReplacementJob/opaque one-shot resource and the Runtime's
infallible final channel install. Core/Runtime9/9passed (native-restore-audio-tests2).
Sequential audio and target Media/World/classic scene audio remain unsupported;
peer producers are preserved. New static-font dependency/input-epoch guards are
in verification. NNN begin_original_restore/dataN itself is still not enabled:
it needs the full detached native seed/state/message/selection/effect/audio
consumer, not a helper-only or factory success flag.

### Original audio saved-work/consumer batch (2026-11-01)

`native_audio::NativeAudioPolicy` sound_work/voice_work are now canonical
[[i32;16];8], not callback12 arrays. Ordinary callbacks explicitly populate
native loop/number/delta/startXYZ/time/endXYZ/doppler indices0..10; native11..15
remain untouched. STOP clearsword0. This makes the existing Title ClearWork
correct instead of zeroing a callback number. Native fields/settings/decoder
profiles remain distinct; no shader/PCM engine is private to NNN.

New private native_save_audio::SavedAudioWork reads/replaces full native status
records (music36/37, sound52, voice184, names312), preserving unrelated bytes.
SavedAudioPreparation::begin(client,file,profile,current,music,sound,resolver,pool)
validates entire NativeSaveFile before admitting sources; native worker returns
pure AudioRequests into Host.begin_audio_replacement. poll retains terminal
errors; finish returns opaque replacement + detached policy + original raw work.
No live play/Scene/clock mutation and no NativeCall synthesis. Native dataN
begin_original_restore still unsupported until all other state families compose.

Actual static original536740 proven from531dc0 xrefs:8sound/4voice, work0 loop
checks. Actual531dc0 and531370 clamp directly and have no later public source
-101 override. Kept exact legacy behavior and added regression. Voice normal
callback531820 bounds names62+twoNUL, while replay531370 uses saved64-byte name
directly; restored63-byte Cstring now survives. Music0/unbounded1/onceN/finite
uses existing engine counted loops for reviewed single source; MML still gap.

Final native-audio-save-work-final-tests.log release57passed1ignored (8newcases)
includes actual PCM finite completion/unbounded playback and shared WAV worker
replacement preparation, old/peer isolation/zero clock, whole-profile rejection
beforeIO, late-corrupt-source retained failure, raw/status/name preservation.
No new Desktop/Game/GPU/title/Start/complete original-save result. IDA closed;
ignored saved-audio-{addresses}.c/function receipts and source audit record facts.

Shared static-font/input restore guards are ready:14passed1ignored plus explicit
systemfont1passed. Unified docs a7e4bc50985744b6 no-bless8+coverage1passed,
Python201passed. Next Alice GlyphImageSpec is a later publicbatch; do not mix
its future fingerprint with this verified historical receipt.

Audio restore component next point: AudioBridge.restore_seed now retains Arc
music/sound/system tables + fixed-size native policy + resolver/TaskPool handles.
seed.prepare(file,profile) feeds the actual shared replacement pipeline;
seed.finish(ready) constructs a detached AudioBridge plus opaque replacement,
not a liveDSP/mixer clone. Bridge exposes audio_restore_seed/original_audio_work
for full original-restore assembler. Export retains raw voice-name tails while
merging normal current fields; all16 native work words preserved.
Native i32 volume arithmetic wraps before clamp. Direct saved63byte CP932 names
must bypass callback receipt truncation before decoding; last pair regression
now covers both ASCII and finalSJIS pair. Last exact release58/1ignored logs
native-audio-restore-seed-tests and native-audio-restore-name-boundary-tests.
Still no begin_original_restore override/complete dataN VM+Bridge consumer.

Shared GlyphImage + prepared ServiceBatch/applyBatch final docs are now actual
1bea1d048c6dd276 no-bless8+coverage1passed. Intermediate081f failed stale before
Cat's public batch froze, never a passing receipt; historicala7e4 kept truthful.

## Original message/selection waiting actors (2026-11-01)

Actual narrow release lib tests4 passed65/1ignored. New native_save_wait reads
original mode/row/control fields, and detached text/selection preparation uses
shared Text workers, ImageJobs and UI. A saved SELECT resumes native CASE from
its saved nextPC without manufacturing a callback/ACK; actual Core VmAdapter
wait dispatch is hooked and tested (only its chosen END executes). Text blocks
the load click, then uses shared reveal completion and a later advance click.
Original word33 is total INCLUDING supplemental word34. Word180 selects a
system-window variant and is NOT visibility. Nonzero variants/timed/special
selection remain explicit native-consumer gaps. Complete dataN entry is not
claimed; full Bridge/effect/frame assembly is the next step.

Subsequent source after that test point adds saved-effect hydration, raw/opaque/
keyed engine resource views and class-global SUNA warm cache. These are not
covered by tests4 until the next combined check/test; no new Desktop artifact.

## Native original dataN transaction — 2026-11-01 verified atomic point

Actual native-original-entry/check5 releasecheck succeeded. Frozen binary `native-original-entry/lib-89191493c4f8c016`, SHA89191493c4f8c0165b27249dde2b521370940516c898f3021af531073a279b1a: ordinarylib70passed/2ignored, explicit licensed runtime-font test1passed covering both NativeAdapter/InstalledAdapter. Logs library-actual5.log/runtime-actual5.log. No game/desktop completion claim. The earlier test timeout was an authored Halt expectation: original END restarts sys under a loader and correctly ran millions of instructions. Preserve this production policy; do not patch END to satisfy a framework test.

New `native_game_restore.rs` actually stages full original records/context, current persistent200..299/globalRNG, detached image/audio/text/selection resources, serializes a Scene plan, and returns a ready native VM to generic Runtime commit. Original window display is status44, messageSerial476, window variant180. Old module SUNA InitParameter0x4a68d0 regenerates global pattern/count0 from originalpara0; default Init for other currently supported classes is no-op. Static complete routine/vtable evidence is in native-load-family.json/native-load-init-leaves-static3.log/native-saved-effect-init.json and public source-audit JSON.

Do next: complete ordinary/restored PRINT+SELECT native actor-frame/window refresh and General family before lifting dynamic-effect/last-screen-layer gates. The source CShakin scheduler differs from floor elapsed/period: full bounded reusable AdaptiveStepClock source is now in Scene animation/adaptive_steps.rs with3 tests queued, not yet counted as passing. It must be truly consumed by native actors, not left as an isolated helper. Keep原保存UI/export and other remaining gates explicit; NativeAdapter original snapshots are still not a completed framework save path.
