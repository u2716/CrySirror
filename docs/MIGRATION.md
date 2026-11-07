# Native adapter migration requirements

The desktop baseline is wgpu + winit. Native VMs are first-class workspace
crates; all desktop adapter factories are linked by the standard player build.
The previous “fetch sources, connect RFVP, then Siglus, add a static 3D renderer”
sequence has been retired. See [native adapters](NATIVE_ADAPTERS.md) for current
profiles and [the roadmap](adr/0041-engine-capability-roadmap.md) for remaining
engine work.

## Ownership

Record `adopt` or `integrated` according to
[ADR 0102](adr/0102-engine-adoption-and-runtime-integration.md) and
[the onboarding contracts](ENGINE_ONBOARDING.md). Existing engines keep their
mode unless explicitly migrated. Siglus, BGI and SoftPAL use `integrated`;
their preserved extraction paths are inactive in the default player.

For `adopt`, keep original opcode/value/control-flow, resource selection, waits
and save policy in the adapter. Reusable image, text, composition, UI, effects, codecs and
audio mechanisms belong in shared engine crates with independent APIs.
An original engine core, private renderer/mixer or composed-screen upload is not
an accepted `adopt` boundary.

For `integrated`, maintain complete runtime modules inside workspace crates
behind a bounded frame/PCM and host-service seam. CrySirror owns windows, event
loops and devices. Lifecycle, storage/NLS, input, saves and per-platform gaps
must be documented and connected explicitly. Both modes preserve provenance
and license notices; neither a wrapper nor a renamed package is completion.

## Resource and task boundaries

Use client-scoped handles and capability roots. Workers prepare bounded immutable
inputs/results; owners publish completed state. Preserve a native call's PC,
arguments and waits until the required service actually completes. Unknown
operations/layouts fail visibly. Engine-wide async ownership remains mandatory
under [ADR 0050](adr/0050-engine-wide-async-execution.md); existing worker paths
do not establish that every subsystem is fully migrated.

## Saves and validation

Original saves and framework snapshots are distinct formats and promises.
Resource-dependent original restore can use the staged coordinator described in
[ADR 0099](adr/0099-staged-original-save-restore.md). Each adapter still needs its
own source-correct import/export and completion policy; a coordinator does not
supply those automatically.

Verify the shared capability without the VM, then verify real native calls,
input edges, error retention and restore boundaries. Record implementation,
release compilation and behavior separately in the adapter's documentation;
keep native field/operation evidence beside the corresponding source.
A factory, parsed archive or screenshot is not complete integration. Run local
release checks appropriate to the task. Platform support requires actual
platform evidence; Switch/PS Vita remain deferred.
