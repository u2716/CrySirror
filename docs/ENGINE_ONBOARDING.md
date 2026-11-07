# Engine onboarding

[ADR 0102](adr/0102-engine-adoption-and-runtime-integration.md) defines two
accepted modes. This guide describes the required implementation and evidence;
it does not assert that existing adapters already meet every item.

## Choose and record the boundary

| Question | `adopt` | `integrated` |
| --- | --- | --- |
| Starting point | New implementation from reviewed native evidence, or maintainable original VM/parser source | Existing runtime maintained within workspace crates |
| VM policy | Original opcodes, values, syscalls, waits and saves | Original runtime semantics and session policy |
| Rendering/text/UI/effects | Shared CrySirror capabilities with thin native mappings | Native composition may remain internal; declared frame output goes to the host |
| Audio | Shared engine decoding/mixing/channel capabilities | Native audio may remain internal; declared PCM output goes to managed engine audio |
| Window/event loop/devices | CrySirror host | CrySirror host |
| Completion claim | Which shared capabilities and native semantics are implemented | Which runtime/host/output/platform contracts are connected |

Put the mode in the crate README, with entry points, native profile, mounts,
writable namespace, encoding policy and remaining limitations. Source
availability alone does not choose a mode. Continue an existing engine's mode
unless the task explicitly changes it; default new unclassified work to `adopt`.
Do not turn a difficult syscall into an undeclared runtime fallback.

## Adopt and reverse implementation

Inspect complete relevant dispatchers and helper/lifecycle bodies at the
recorded source revision or binary digest. Reuse existing evidence when its
identity and scope still match. Follow state writes, error branches and
subselectors before implementing hooks; fixing the first failing call does not
establish surrounding semantics.

Keep original opcode/value/resource/wait/save behavior in the VM. Extract
generic image, glyph/layout, composition, UI, effect and audio mechanisms into
shared crates. Original slot bytes and CrySirror snapshots require separate
restoration policy; native memory layouts are not portable framework saves.

Maintain an inventory near the implementation: opcode IDs, syscall IDs,
subselectors, original no-ops, implemented/unsupported paths and unknown
denominators. Count an operation complete only after its reviewed branches and
dependencies are implemented. Report counts with their denominator and native
profile; opcode presence is not syscall/subselector coverage. Keep exercised
game paths separate from source coverage.

Authorized reverse engineering uses the
[onboarding skill](../.agents/skills/crysirror-engine-onboarding/SKILL.md).
Static inspection is the default. Preserve user originals and keep derived
outputs private and ignored. Analysis permissions do not authorize execution of
an original EXE/DLL, emulator, debugger target or shader compiler. Existing
explicit authorization for a specific action remains valid.

## Integrated runtime contracts

Maintain native runtime modules within workspace crates, with a documented host
seam. Remove standalone launcher/device ownership from the default integration
path. Define these contracts at that seam:

| Area | Required contract |
| --- | --- |
| Session/lifecycle | One mutable owner; startup failure, readiness, cancellation, pause/resume, restart and shutdown. Workers retire before resources disappear; no unbounded process-global game state. |
| Graphics | Native canvas, format, dimensions, stride, alpha/color space, dirty/full-frame policy, ownership and memory/queue bounds. Retain frames through presentation completion. Resize presentation independently of native game resolution. |
| GPU path | Supported backend/device identity, synchronization and loss/recreation behavior. Native renderer code stays outside core/compat. Readback or device interop is an explicit path with measured costs. |
| Audio | PCM format/rate/channels, timestamps and clock owner; bounded queue/backpressure, underrun/EOF, pause, flush, seek, drain and teardown. Use managed output; do not open a second device or feed audio through two mixers. |
| A/V scheduling | Advance the native clock once per admitted tick; specify audio/presentation deadlines. Loading waits, skipped presentation, mute and suspended views must not advance the VM twice. |
| Input/UI | Original-canvas coordinates, press/release, focus, repeat and modal ownership; explicit touch mapping and host dialog/menu behavior. |
| Resources/NLS | Scoped bounded readers and writable storage, seek/read-at providers, encoding, archive priority, case policy and dependent resources. Do not require every provider to become an OS path. |
| Saves | Original-save and framework-snapshot support separately; reconstruct pending resources, audio, motion and input before publishing the restored session. |

Opaque native frames/audio are permitted for this mode; they do not count as
newly extracted shared APIs. Shared migration needs separate implementation
and evidence. Unsupported output paths fail before boot or at the actual
operation; they cannot silently become ignored commands.

## Platform capability matrix

Maintain one table in the adapter README for targets affected by the task.
Include unimplemented targets as explicit gaps. Distinguish `not connected`,
`source implemented`, `release compiled` and `behavior observed` per cell.
Unavailable SDKs are not evidence that a backend works.

| Target | Review separately |
| --- | --- |
| Windows / Linux / macOS | Build/link dependencies, storage, winit lifecycle, GPU backend, audio output, dialogs, encoding and save locations. One OS does not validate the other two. |
| iOS / Android | Native packaging, view generations, touch, suspend/audio interruption, surface/device replacement, storage permissions and process-death/save policy. Keep desktop dialogs/winit assumptions outside portable runtime code. |
| Browser | wasm dependencies, injected resources, asynchronous reads, workers/atomics, audio activation and GPU/readback limits. Native threads, local paths and dynamic libraries do not automatically translate. |
| Vita / Switch | Deferred native renderer/shader/audio/storage/task backends; SDK and dependency portability remain gaps. No desktop GL/wgpu/window assumption or desktop RT guarantee. |

An initial scoped implementation need not claim every target. Keep platform
code behind host/backend boundaries so missing services can be connected
deliberately. Record real canvas, memory, codec, thread and rendering limits.

## Documentation ownership

| Location | Maintained content |
| --- | --- |
| Adapter `README.md` | Current mode, module entry points, launch/configuration, host contracts, platform matrix and actual gaps. |
| `SOURCE.json`, licence/notice files | Source URL/revision/digest, provenance, adopted parts and local changes; no invented pins or proprietary payloads. |
| Source comments / focused module notes | Native call/field mappings, original no-op evidence, assumptions and unresolved branches beside their implementation. |
| `docs/validation/<topic>.md` | Dated commands, exact binary/profile/environment, implementation versus compilation versus behavior, failures and unexercised boundaries. Private data and large raw traces stay ignored. |
| Shared API reference / ADR / guide | Public contracts, architectural decisions and user-facing operation. Regenerate docs/api and its fingerprint log when the public API changes. |

Do not append implementation specifications to STATUS.md, duplicate them in
giant HANDOFF logs, or save temporary agent/build coordination as project rules.
Update the owner document, remove contradicted current claims and repair links
when retiring a document. Preserve provenance and licence notices.

## Validation and completion

Use local release checks appropriate to the changed boundary and existing
authorization. Do not add superficial tests, run unrelated whole-workspace
regressions or create debug artifacts. For a new
risky semantic/output boundary, choose the smallest check capable of exposing
an incorrect state transition or lifetime; existing fixtures may suffice.

Record implementation, compilation and behavior separately. Exercise real
input/resource/save/output paths when authorized. Offscreen runs can validate
composed frames and native progression, but not physical menu interaction,
audible output, mobile lifecycle or console hardware. A muted run proves
neither audio fidelity nor native audio completion. Repeat checks only for new
changes or unresolved failures. Report remaining paths without upgrading a
title screen or one save round-trip into complete-game support.
