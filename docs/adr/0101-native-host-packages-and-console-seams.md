# ADR 0101: Native application packages and console host seams

Status: accepted for host packaging/scaffolding; console device backends deferred.

Native app hosts reuse CrySirror's existing Runtime, adapters, resource workers,
shared renderer and audio. The desktop package exposes `crysirror_player`; the
`crysirror-mobile` crate is the C/JNI surface boundary. Apple/Android UI owns
native views and source selection; a Rust session owner holds engine/device
state. Native NLS selection is applied before VM creation through each adapter's
actual configuration field, without global byte recoding.

macOS/iOS/Android use `crysirror-wgpu`. PS Vita and Switch require native device
backends and a portability audit of the std-dependent runtime and adapters.
They do not inherit desktop winit, native dialogs or a fictional wgpu surface.

The console milestone consists of `platform/psv`, `platform/switch`, the
`crysirror-console` SDK-free no_std contract and a Release cross-check helper.
The common coordinator represents explicit simulation/presentation completion,
resource ownership, suspend/resume and shutdown. A platform output must transfer
an immutable engine frame to an owned presentation ticket, retaining it until
completion. Backend capability declarations must reflect real implementations;
unconnected renderer/audio/storage/clock/worker services fail before boot.
No SDK device driver or playable console engine is implied by this scaffold.

The freestanding Switch target checks core/alloc code; it does not make the
full std-based engine linkable. Vita's partial std support likewise requires
filesystem/threading and dependency validation. Future driver work stays in
host/backend crates, preserving safe window/device-free core/compat and native
VM/NLS/save semantics. Console hardware, packaging and performance claims
require separate actual results.

See [native packaging](../NATIVE_PACKAGING.md),
[Vita scaffold](../../platform/psv/README.md),
[Switch scaffold](../../platform/switch/README.md).


## Native view replacement

Session identity outlives its native view. Each attachment has a monotonically
increasing generation and a factory-held native view lease. Detach waits for
retirement of that generation only; old callbacks cannot detach its successor.
The owner drops a Surface before releasing its native lease. No VM snapshot or
save conversion is required to replace a view. Android retains the session ID
across Activity recreation within the same process; Apple view dismantling retires the
view rather than shutting down the Player. OS process death still requires
normal native saves and does not imply automatic live-VM recovery.
