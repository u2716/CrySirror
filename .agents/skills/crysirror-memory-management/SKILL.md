---
name: crysirror-memory-management
description: Maintain CrySirror CPU and GPU memory budgets, resource ownership, asynchronous lifetimes and allocation-failure behavior. Use for caches, loading, rendering or OOM work, including the distinct PS Vita GXM and Switch deko3d memory and synchronization constraints.
---

# CPU and GPU memory maintenance

Read `AGENTS.md`, the affected subsystem's source and
[memory contracts](../../../docs/MEMORY_MANAGEMENT.md). For console work, read
only the affected platform's [PS Vita](../../../platform/psv/PLATFORM.md) or
[Switch](../../../platform/switch/PLATFORM.md) notes. Preserve native semantics
and the selected `adopt`/`integrated` boundary.

## Establish ownership and the real budget

Trace allocation, sharing, publication, last use and release before changing
cache sizes or heap settings. Identify physical backing separately from logical
handles/views. Include fixed host allocations, VM state, task scratch, CPU/GPU
copies, upload/readback staging, history/render targets, queued frames and
retired resources. An encoded file's size is not its decoded/resident cost.

Record requested, aligned/committed, live, reserved/in-flight and retired bytes
where the backend can expose them, plus object/memblock/descriptor counts.
Distinguish estimated resource accounting, allocator statistics and OS/driver
measurements. Shared physical memory must not be counted as independent CPU
and GPU pools, while genuine mirror/staging allocations must not disappear.

Check the actual process/launch-mode limits. A desktop budget, emulator result,
configured heap or advertised physical capacity is not a portable application
budget. Increasing the PSV newlib heap can remove space needed by GPU-mapped
user allocations. Switch GPU allocations also consume the process's available
backing memory.

## Maintain the allocation lifecycle

For NFS/SMB caches and pending reads, use
[network storage](../crysirror-network-storage/SKILL.md). Include per-connection
blocks, directory metadata, active replies and decoder overlap in the budget.

- Reserve output, scratch and staging before admitting bounded worker jobs;
  include overlap with the old live resource during replacement/restore.
- Retain immutable outputs through their final CPU consumer and GPU completion.
  CPU task completion, dropping one handle and ending a render pass are not
  universal GPU release fences.
- Reuse existing shared caches, arenas and retirement queues. Evict only entries
  whose ownership/completion permits it; never free a VM-live resource just
  because it is not visible this frame.
- Return reservations on failure/cancellation and retire late results without
  publishing stale client/session generations. Inspect initialization rollback,
  failed submission, suspend/resume, game switching and shutdown.
- Check sizes, strides, format widths, mips, layers, samples, alignment and
  integer narrowing before allocation/copy. Respect the actual helper/library
  format implementation rather than inferring byte size from a format name.
- Use bounded recovery with a defined completion condition. Do not hide OOM by
  dropping native operations, looping retries, increasing every budget or
  assuming process exit will retire a live-session leak.

## Apply platform-specific constraints

Vita uses native GXM/GXP, separate user/CDRAM allocation choices and constrained
memblock counts; it has no general compute stage. Switch uses the native
deko3d/DKSH path with explicit memory/cache/queue ownership. Respect actual
supported formats/passes and the shared console session seam. No OpenGL/vitaGL
fallback or hardware-RT assumption is introduced by memory work.

Keep visual-novel canvas, text and original operation semantics intact. Apply
only declared 3D tier substitutions, with their reported limits. Review
[API compatibility](../../../docs/api/README.md#compatibility-assessment) before
changing allocation/service contracts, defaults or completion behavior.

## Verify and document the changed boundary

Use existing diagnostics and the smallest authorized release workload. Compare
the same native profile/build/environment at named lifecycle points, including
replacement or repeated entry/exit when relevant. Explain peaks, retained caches
and retirement completion; RSS alone does not prove a leak or its absence.
Keep emulator and hardware evidence separate. Do not invent unified counters,
free-memory queries or a measured platform budget that the source does not have.

Update resource ownership/budget notes beside the subsystem and the relevant
platform document when its contract changes. Put measured commands and results
in focused validation notes, with omissions explicit. Keep raw traces bounded
and private. A skill/documentation change requires no new build tree, game run
or stress test.
