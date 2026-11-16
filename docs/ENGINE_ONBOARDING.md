# Engine onboarding

[ADR 0102](adr/0102-engine-adoption-and-runtime-integration.md) defines two
accepted modes. This guide describes the required implementation and evidence;
it does not assert that existing adapters already meet every item.

## Starting with an unfamiliar engine

| Need | Skill |
| --- | --- |
| Find existing online engine/format material and assess its evidence | [Engine research](../.agents/skills/crysirror-engine-research/SKILL.md) |
| Install local tools and begin original-program analysis | [Reverse engineering](../.agents/skills/crysirror-reverse-engineering/SKILL.md) |
| Maintain ABI profiles, opcode/syscall contracts and API compatibility | [Engine semantics](../.agents/skills/crysirror-engine-semantics/SKILL.md) |
| Maintain CPU/GPU budgets, lifetimes and target allocation behavior | [Memory management](../.agents/skills/crysirror-memory-management/SKILL.md) |
| Connect NFS/SMB paths, scoped readers, caches and local saves | [Network storage](../.agents/skills/crysirror-network-storage/SKILL.md) |
| Maintain original save versions and transactional reconstruction | [Native saves](../.agents/skills/crysirror-native-save/SKILL.md) |
| Maintain platform lifecycle, packaging, signing and build-only CI | [Platform hosts](../.agents/skills/crysirror-platform-host/SKILL.md) |
| Add a shared archive parser, transforms and VFS mounts | [Archive onboarding](../.agents/skills/crysirror-archive-onboarding/SKILL.md) |
| Connect a VM or complete runtime to the unified player/host | [Engine onboarding](../.agents/skills/crysirror-engine-onboarding/SKILL.md) |

Use matching official material where available. Otherwise, original-program
reverse evidence is the basis for native behavior. Unofficial implementations
and articles, including our own, are useful references but not guaranteed parity.
Record disagreements and intentional compatibility extensions explicitly.

If you have a game directory but do not know the engine or reverse-engineering
tools, start with the
[reverse-engineering skill](../.agents/skills/crysirror-reverse-engineering/SKILL.md).
It guides the agent through identifying files, reusing or installing a local
IDA/Ghidra setup, tracing an initial native operation and producing an API map.
You do not need to identify functions or understand assembly before starting.

For example: "Use $crysirror-reverse-engineering to investigate the engine in
this game directory and help add CrySirror support. Start with local static
analysis." Include the directory, desired feature and any known upstream
repository. The agent should inspect available evidence and tools before asking
for more information. Details are in the
[tool setup](../.agents/skills/crysirror-reverse-engineering/references/tool-setup.md)
and [first analysis](../.agents/skills/crysirror-reverse-engineering/references/first-analysis.md)
references. Original-program execution and validation follow the session's
actual permission scope. Continue with the mode-specific contracts below when
implementing support.

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

Native engine work follows this order for each reviewed slice: establish
ABI/profile identity and differences; maintain opcode/syscall/subselector
semantics and developer tables; then map the contract to existing capabilities.
Assess API compatibility before extending shared services. Reuse proven
compatible profiles, separate genuinely different encodings/semantics and keep
unknown profiles explicit. The
[semantics skill](../.agents/skills/crysirror-engine-semantics/SKILL.md)
provides the workflow. Integrated engines retain their complete dispatch
catalogues and host seam while following the same contract-first analysis.

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

Present the inventory as a readable API table: native API/opcode, purpose,
profile, corresponding CrySirror API, connected behavior and remaining gap.
Keep implementation history out of that table.
Use the [opcode and syscall lists](native-api/README.md) as the entry point.

Each developer reference must include:

| Section | Required information |
| --- | --- |
| ABI | Native profile/version, complete command identity, operand/value forms and PC units. |
| Execution | Argument/result ownership, return/branch behavior, pending work, waits and error/default paths. |
| Shared services | Native operation family, corresponding CrySirror service and completion boundary. |
| Implementation tables | Opcode/syscall/export IDs or names, purpose, source implementation, argument guards and explicit gaps. |
| Dynamic dispatch | Receiver/class/module scope, overloads, conditional ranges, aliases, registration loops and caller-defined namespaces. |
| Evidence | Source identities separately from compilation and exercised behavior. Unknown native totals remain explicit. |

Maintain the ABI/service contract above each reference's generated marker.
Regenerate source tables after dispatch changes. Preserve full guards and
fallbacks; an allowlist is not an implementation and a registered callback that
returns an error is unsupported. Keep proven native no-ops distinct from empty
handlers. Replace obsolete summaries when consolidating documentation, repair
their incoming links and retain source provenance, licence notices and native
findings beside the code. Current reference material must not depend on a
handoff log.

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

The default integrated player must use GPU composition. CPU asset decoding and
glyph rasterization are separate from full-screen composition. Uploading a
CPU-composed screen is not GPU composition. Record existing CPU composition or
per-frame GPU readback as integration work in the version/change table.

## Platform capability matrix

Use the [memory contract](MEMORY_MANAGEMENT.md) and source-specific
[PS Vita](../platform/psv/PLATFORM.md) / [Switch](../platform/switch/PLATFORM.md)
notes for memory domains, shader/backend limitations and completion ownership.
Configured heaps, theoretical capacity and emulator observations are distinct
from a measured application budget or real-device acceptance.

Maintain one table in the adapter README for targets affected by the task.
Include unimplemented targets as explicit gaps. Distinguish `not connected`,
`source implemented`, `release compiled` and `behavior observed` per cell.
Unavailable SDKs are not evidence that a backend works.

| Target | Review separately |
| --- | --- |
| Windows / Linux / macOS | Build/link dependencies, storage, winit lifecycle, GPU backend, audio output, dialogs, encoding and save locations. One OS does not validate the other two. |
| iOS / Android | Native packaging, view generations, touch, suspend/audio interruption, surface/device replacement, storage permissions and process-death/save policy. Keep desktop dialogs/winit assumptions outside portable runtime code. |
| Browser | wasm dependencies, injected resources, asynchronous reads, workers/atomics, audio activation and GPU/readback limits. Native threads, local paths and dynamic libraries do not automatically translate. |
| Vita / Switch | Native GXM/deko3d rendering, audio, storage and task backends. Track implementation, compilation and device behavior separately. No desktop GL/wgpu/window assumption or desktop RT guarantee. |

An initial scoped implementation need not claim every target. Keep platform
code behind host/backend boundaries so missing services can be connected
deliberately. Record real canvas, memory, codec, thread and rendering limits.

## Documentation ownership

`UPSTREAM.md` is reserved for the four engines maintained in separate upstream
repositories: Siglus, BGI, SoftPAL and RFVP. Each integrated crate records its
upstream revision and retained host changes there. Adopted VMs and shared
components use source comments, `SOURCE.json` or focused `SOURCE_NOTES.md` for
native findings and attribution; they do not maintain an `UPSTREAM.md`.

For integrated engines, maintain a concise upstream-version and local-change
table in [the integration reference](INTEGRATED_ENGINES.md). Record the complete
base commit, later selected imports and files changed for the host. State the
actual default rendering path: GPU composition, CPU composition or GPU readback.
Keep update instructions separate from dated execution logs. Do not describe
uploading a CPU-composed screen as GPU composition.

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
