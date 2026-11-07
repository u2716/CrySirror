# ADR 0102: Engine adoption and runtime integration

Date: 2026-11-06
Status: accepted policy; no new implementation or platform is certified by this record.

## Decision

CrySirror supports two explicit engine onboarding modes. Record the selected
mode and its concrete boundary in the adapter README before implementation.
Use the owner's selected mode; otherwise continue the existing engine's mode.
A new integration without a selection uses `adopt`. Changing an existing
engine's mode requires a deliberate migration, not relabelling its package.

| Mode | Implementation boundary | Acceptance |
| --- | --- | --- |
| `adopt` | First-class VM and native semantic/resource/wait/save policy over shared CrySirror capabilities. Implement from scratch through authorized source/binary analysis, or adopt maintainable interpreter source with provenance. | Native semantics plus independently usable engine capabilities; a factory or matching screenshot alone is insufficient. |
| `integrated` | An existing runtime, including its native composition where needed, maintained inside workspace crates and connected to CrySirror's host services. | Managed lifecycle, bounded frame/PCM output, input/storage integration, documented platform capabilities and actual scoped behavior evidence. This does not establish capability extraction. |

Siglus, BGI and SoftPAL use `integrated`. Other engines retain
their current `adopt` boundary unless explicitly migrated. Both modes preserve
source provenance, original instruction/value/wait/save behavior and visible
failure for unknown operations or layouts. Original no-op operations stay
identified as such; unfinished functionality cannot be counted as a no-op.

## Common requirements

- Maintain the implementation as first-class workspace crates. A renamed
  upstream package, external engine Git dependency or separately launched
  player is not the maintained integration boundary. Preserve notices and
  identify the original source revision and local modifications.
- The host owns windows/views, event loops and output devices. Core/compat stay
  safe and free of window/GPU/audio dependencies. Logical VM/save state never
  stores live device pointers, executable code or usable host handles.
- Engine-wide task ownership, bounded queues, cancellation and completion
  remain required under ADR 0050. Each session owns its mutable runtime state;
  outputs have explicit lifetimes through presentation or consumption.
- Assets use scoped CrySirror storage capabilities and separate writable save
  roots. Archive reads work with injected seek/read-at providers rather than
  assuming a local OS path. Encodings remain engine-specific policies.
- Current boundaries belong in the crate README, provenance in SOURCE.json and
  licence notices, operation findings beside corresponding code, and dated
  behavior evidence separately. STATUS.md and HANDOFF logs are not the
  implementation specification. Do not persist temporary agent ownership,
  build freezes or queue instructions as project rules.
- Declare desktop, iOS, Android, browser, Vita and Switch capabilities
  separately. Unimplemented targets/backends fail visibly. Native shaders,
  CPU threads, browser workers and hardware RT are target capabilities, not
  consequences of either mode. Console device backends remain deferred.

## Integration-specific requirements

The integrated runtime cannot create its own window, event loop, audio device
or unmanaged process. It may keep native graphics/audio algorithms internally,
but exchanges owned, bounded outputs with CrySirror. A CPU frame needs a declared
pixel/alpha/color-space/stride contract and native canvas dimensions. Any GPU
interoperability needs an explicit compatible backend and completion/ownership
contract; portable GPU pointer sharing is not assumed.

Audio publishes timestamped PCM with declared format, rate and channels to
CrySirror's managed audio path. Define queue limits, backpressure, EOF, pause,
seek, drain and device-loss behavior; prevent double mixing, double clock
advancement and duplicate output. Muting output cannot acknowledge a native
decoder/wait that has not completed. Output timing and native voice/save policy
remain distinct.

Input maps from host presentation to the original game canvas, with consistent
keyboard/pointer/touch, focus and modal ownership. Resize, suspend, view
replacement, restart and shutdown release or retain resources in a defined
order. Acceptance covers these boundaries and actual native game paths;
desktop startup alone is not mobile/browser support.

## Relationship to existing decisions

This ADR extends ADR 0047: shared-capability extraction remains binding for
`adopt`; `integrated` is a separate admitted mode under the requirements above.
It generalizes the original BGI/SoftPAL exception. Siglus was explicitly selected
for integrated migration on 2026-11-06; other adapters do not change mode automatically.
ADR 0050 (async execution), ADR 0099 (staged original restore), ADR 0100 (browser)
and ADR 0101 (native packages/console seams) retain their scope.

Operational requirements and documentation ownership are maintained in
[engine onboarding](../ENGINE_ONBOARDING.md). Authorized source/binary analysis
is guided by the repository's
[onboarding skill](../../.agents/skills/crysirror-engine-onboarding/SKILL.md).
