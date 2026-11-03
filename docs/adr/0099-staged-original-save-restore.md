# ADR 0099: Staged original-save restoration

Status: implemented bounded coordinator; native engine consumers remain in progress.

## Context

A native save is not a CrySirror framework snapshot. Rebuilding original script
contexts may also require scoped archive reads, images and native presentation
state. The old synchronous `VmAdapter::import_original_save` has no resource
host, and reporting success before those dependencies finish is incorrect.
Sequentially changing the live scene and then loading resources also permits a
failed restore to leave half a replacement running.

## Decision

An adapter can opt into `VmAdapter::begin_original_restore`, returning an
owner-held `OriginalRestorePreparation`. It must start from a native safepoint
and retain detached state; it does not move the current VM or execute another
script batch. Existing adapters keep their old synchronous methods unchanged.

The Runtime supplies instance/client/epoch/serial-scoped tickets and explicit
begin, poll, status, cancel, release and commit operations. Only the target VM
is held. Other clients can continue; polling never ticks a VM, clock or input.
A framework restore, successful native restore/import or close obsoletes pending
preparations. Terminal receipts remain observable until released, within bounds.

The preparation host grants read-only client storage, immutable measurement,
image/pattern workers and decoder-header probes. Scene/input/storage mutation,
playback and font registration are rejected. Image publication allocates a
unique path and reserves the existing shared capacity, but remains absent from
the live resource catalog. Cancellation and failure release that reservation.

After native resources are ready, a bounded CPU worker decodes versioned shared
scene batches. The replacement VM stays on the owner; no device pointer or
executable continuation is moved to the generic worker. Ticket input and plan
bytes share bounded admission. Commit validates against the latest engine state,
checks ready immutable image references and performs the final catalog install,
old caller-lease retirement and VM/scene swap without a later fallible action.
Old scene/capture references keep the actual immutable image generation alive.

Audio uses pure play recipes prepared on shared workers without registering a
live voice. Its ready resource is Runtime-instance/client scoped and consumed
once. The owner obtains an exclusive ChannelReplacementCommit after all voice,
slot, generation, group and memory checks; image publication is followed by its
infallible install and VM/scene swap. Existing mixer/decoder/DSP state is never
cloned. An empty replacement explicitly clears this client's channels/tails and
pending play/seek/sequence work. Other clients' voices and DSP remain intact.

Sequential audio requests, client reverb-history restoration, World simulation,
Media producer ownership, classic scene audio and remaining external motion
producers in the restored client stay unsupported. These require their own
prepared ownership/install contracts. Native consumers must not advertise
complete save compatibility merely because the shared coordinator is available.

## Validation and consequences

`tests/original_restore.rs` uses synthetic VMs and real engine ImageJobs; it does
not use a legacy engine, game data, device or private renderer. Checks cover
pending/ready publication, unchanged state on error, read-only preparation,
client/runtime isolation, cancellation, epoch invalidation, old-image lifetime
and unrelated producers. Exact completed release results are in STATUS.md.

This API is a shared Rust capability boundary, not a serialized project command
or a new script class. Native format/layout/wait policy remains in its adapter.
