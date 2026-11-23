# System4 shared native completion, 2026-11-09–10

Scope: authorized Rance7 static module/AIN analysis and targeted release CPU
validation on macOS. The adapter remains `adopt`; source contracts, provenance,
coverage definitions and remaining parameter/codec/platform limits are in
[the native completion note](../native-api/system4.md).
Original EXE/DLL files were not executed. No new game, GPU, window or screen
capture was used for this batch.

## Implementation

The 15 AIN libraries declare351 exports. All351 names now have semantic routes
for explicit supported domains, including the previously missing38 names.
The batch adds RGB-only screen/multiply, original Math modes/Bezier/array-table
policy, shared stretch/area reduction, line/wrap/overlay, four rotation/zoom and
twelve axis-projection operations, polygon texture/colour rasterization, sprite
copy/enumeration/hits, native civil time, PCM analysis/FFT/Hann, scoped original
PCM sample queries, disabled-pad queries and shared log/modal/menu actions.

Reusable algorithms and owned worker results belong to shared Image/Audio;
System4 retains native values/resources/waits. Original PCM queries preserve
source layout for RIFF PCM8/16 mono/stereo; compressed codecs are explicitly
outside that domain. New in-place/clipping domains without recovered compatible
behavior fail visibly. Portable floating arithmetic does not establish original
x87 or hardware-specific SIMD bit identity.

Read-only CODE inspection counted1,062,488 instruction records,126 distinct
opcode names and668 CALLHLL sites reaching237 distinct static targets. None of
these figures measures execution coverage. The layout catalogue has273 entries;
29 still lack an executor route. Of those, this CODE includes ENDFUNC3671 and
EOF372 compiler/file markers; their runtime execution remains unsupported.
Complete original saves and complete-game compatibility are not established.

## CPU behavior validation

- Shared Image filtered unit tests passed8/8: exhaustive byte-pair screen and
  multiply tables with hidden RGB, area reduction/cropping, Bernstein progress,
  clipped inclusive lines, wrap phase/threshold masks, rotation and projection.
- Shared triangle tests passed3/3: inclusive boundaries, hidden RGB/alpha direct
  copies, homogeneous interpolation, byte conversion, winding and same-surface
  feedback. These are CPU resource operations, not rendered GPU screenshots.
- Shared Audio filtered unit tests passed4/4: PCM8/16 channel order and word
  boundaries, positive-sign RDFT/Hann behavior, original source layout, foreign
  client rejection, cloned-result budget lifetime and stable failure polling.
- System4 `--features runtime --lib --test sprites --test input` passed66 library
  tests,3 sprite tests and3 input tests, with3 existing owner/font tests ignored.
  New tests execute native Math, typed geometry including all16 rotation/axis
  names and both polygon exports, source sample waits, FFT/PCM arrays and error
  policy. Date/time checks preserve outputs until one real receipt; input latch
  clearing preserves physical held state and later edges.
- The old library baseline had16 failures caused by missing retained scene-root
  fixtures. Tests now install that required real root; pixel, pending-PC and
  client ownership assertions were retained. Native constructor/map assumptions
  were corrected using original SactEngine evidence, not relaxed to bypass
  unsupported behavior. The two initial failures in the new PCM tests were
  authored fixture errors (hard-coded heap slot1 despite a live local page);
  the fixtures now use the allocator's actual returned handles. An additional
  GetFormat assertion was corrected from44.1 kHz to the WAV fixture's actual
  48 kHz; the adapter returned the correct source rate and the focused format
  test passed after correction.
- A focused review corrected stored-float projection intermediates and signed
  wrapping16.16 row accumulators. The3 affected shared image tests passed again.
- Final getter/setter-chain review found and corrected a reversed blend-rate /
  brightness projection. Original F080/F130 and6FE0/7000/7F20/7FA0 establish the
  offsets, ranges and lazy declaration;5870 proves Screen uses blend rate and
  Multiply requires full blend rate. The shared byte kernels were unchanged.
  A separate final sprite suite passed4/4, checking these distinctions and
  invalid-value rejection without state/publication changes. The last native
  post-review tests passed FFT/PCM and geometry/polygon (2), then source sample
  and format/reference wait behavior (1).
- wgpu `shader_migration::destination_reading_rgb_table_shader_validates_without_a_gpu`
  passed1/1 using pure Naga parsing/validation. No GPU/device was created.

All Cargo commands used `--release -j4`, selected packages and bounded unit or
integration targets. No full-workspace suite was run. The3 ignored tests are
not counted as passes. These CPU checks do not establish native font/GDI pixel
identity, audio-device behavior, GPU output or new Rance7 gameplay progress.

## Compilation

Targeted release compilation covered System4 with its runtime feature, shared
source-layout PCM services and the wgpu byte-blend shader path. Separate focused
post-review verification includes native sample format/reference writes and the
geometry/polygon dispatcher. API reference regeneration is performed with the
coordinated shared-API batch; no separate fingerprint claim is made here.

## Previous gameplay evidence

The prior dated offscreen run in
[the combined bring-up note](system-nnn-alice-2026-11-09.md) completed the opening
movie and stopped at SACT2.SP_SetDrawMethod(1029,1). That call now has a source
implementation, CPU byte-table tests and compiled shader support. This batch did
not rerun that game path, so it does not claim the next scenario/title/menu was
observed or that the engine is completely playable.
