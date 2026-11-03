# Native adapter migration requirements

The desktop baseline is wgpu + winit. Native VMs are first-class workspace
crates; all desktop adapter factories are linked by the standard player build.
The previous “fetch sources, connect RFVP, then Siglus, add a static 3D renderer”
sequence has been retired. See [native adapters](NATIVE_ADAPTERS.md) for current
profiles and [the roadmap](adr/0041-engine-capability-roadmap.md) for remaining
engine work.

## Ownership

Keep original opcode/value/control-flow, resource selection, waits and save
policy in the adapter. Reusable image, text, composition, UI, effects, codecs and
audio mechanisms belong in shared engine crates with independent APIs.
An original engine core, private renderer/mixer or composed-screen upload is not
an accepted integration boundary. Preserve source provenance and license notices.

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
release compilation and behavior separately in [STATUS.md](../STATUS.md).
A factory, parsed archive or screenshot is not complete integration. Run local
release checks/tests; do not enable remote CI. Platform support requires actual
platform evidence; Switch/PS Vita remain deferred.
