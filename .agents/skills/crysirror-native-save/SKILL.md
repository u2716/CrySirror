---
name: crysirror-native-save
description: Maintain original game-save formats and restore behavior in CrySirror, including native version compatibility, scoped writes, detached resource reconstruction, transactional commit and failure rollback. Use for native slots, system/read-history data or save import/export rather than framework snapshots alone.
---

# Original saves and complete restoration

Read `AGENTS.md`, the engine's native reference and save implementation, the
[framework envelope distinction](../../../docs/SAVE_FORMAT.md) and
[original-restore contract](../../../docs/adr/0099-staged-original-save-restore.md).
Use [engine semantics](../crysirror-engine-semantics/SKILL.md) before changing
native save opcodes, syscall completion or ABI selection. Without matching
official material, original-program reverse evidence governs; an unofficial
reader or writer is not a parity guarantee.

## Identify the actual save contract

Distinguish original slots, quick/auto saves, system settings, global/read-history
records, thumbnails and CrySirror framework snapshots. They may have separate
versions and commit rules. Do not wrap a framework snapshot in a native filename
and call it an original save.

Record the game/patch/edition, native ABI, file/profile identity and source/binary
evidence. Inspect headers, version flags, lengths, checksums/transforms and the
complete load/write callers. Different original versions may need explicit
layout branches; a parser failure is not proof that every save is incompatible.
Compare structure and semantics before sharing a parser or applying migration.
Keep native save versions, VM ABI and CrySirror payload schemas distinct.

| Recover | Include |
| --- | --- |
| Binary layout | Field offsets/widths, endianness, native encoding, counts, packed/compressed regions and integrity checks. |
| Logical state | Frames/PCs, values/references, variables, object/resource identities, timers, waits and native restart policy. |
| Persistent families | Slot versus system/global/read-history data, merge/overwrite order and optional companion files. |
| Resource state | Image/text/UI, motion, audio/movie state and dependencies required before the restored VM may run. |
| Error/commit policy | Compatibility rejection, partial-file handling, safe points, write atomicity and successful native return timing. |

Validate counts/ranges before allocation, narrowing or decompression. Preserve
unknown/reserved bytes only where the reviewed format requires it; do not invent
defaults or serialize process memory/pointers. Unknown layouts, unresolved keys
or missing reconstruction semantics fail visibly without resetting user data.

## Protect user data and storage authority

Keep originals read-only during investigation. Use a task-owned local save/user
overlay for write verification and preserve the selected original slot's identity.
Do not overwrite/delete a user's slot, system data or read history as a test.
If a requested migration changes existing files, prepare recoverable copies and
an explicit write plan within the authorized scope, with failure recovery.
Copy only needed save inputs, not the full game installation.

Use scoped storage and existing atomic-write facilities. Keep save paths and
resource references inside the client's authority; reject traversal/foreign
handles in untrusted save data. Network game assets remain read-only and native
writes go to the local overlay. Distinguish atomic replacement from proven
crash/power-loss durability on the actual filesystem. Do not expose save contents,
keys or private game data in public fixtures or logs.

## Choose the existing restoration boundary

For `adopt`, inspect [VmAdapter](../../../crates/crysirror-core/src/vm.rs),
[preparation contracts](../../../crates/crysirror-core/src/native_restore.rs)
and the [Runtime coordinator](../../../crates/crysirror-runtime/src/native_restore.rs).
Use `begin_original_restore` where detached resource-dependent reconstruction is
needed. Do not label a synchronous import transactional unless all fallible work
is staged before live mutation. An unsupported optional API remains unsupported.

For `integrated`, preserve the complete runtime's native slot/restore policy and
document its host/storage/completion seam. Do not replace its saves with the
framework format or force a new VM implementation merely to use the coordinator.
The host still owns session lifetime, resources/output devices and scoped writes.

## Prepare, validate and commit

1. Start at the reviewed native safepoint. Decode into detached candidate state
   and preserve the live VM/scene/audio until the replacement is ready.
2. Prepare actual dependencies through the restricted resource host. Reserve
   image/audio/output memory, including old/new overlap. Preparation must not
   tick the VM, advance clocks, consume input, play audio or write user data.
3. Reconstruct the native state required by this profile. Resource handles,
   decoder histories and device pointers are not portable save values; follow
   the original restart/seek policy instead of assuming exact device replay.
4. Validate the complete plan and ticket's runtime/client/epoch identity.
   Prepare scene/resource and audio replacements before publication. The
   shared coordinator's unsupported producers remain explicit gaps.
5. Commit through the owner once, after preflight leaves no later fallible
   resource-loading action. Retain old resources until their last consumer/GPU
   completion and preserve unrelated clients. Native load success follows the
   completed replacement, not parsing or a ready helper alone.
6. On failure/cancel, retire unpublished results/reservations and leave the
   live game unchanged. On commit/close/restart, invalidate obsolete tickets and
   transient input/receipts according to native policy; late results cannot
   replace a successor session.

Do not satisfy a missing audio/movie/motion restoration contract by silently
clearing it or acknowledging an incomplete load. Use
[memory management](../crysirror-memory-management/SKILL.md) and
[network storage](../crysirror-network-storage/SKILL.md) for their lifetimes.

## Verify the claimed direction and version

Keep import, export, version migration and framework snapshot support separate.
Use the smallest authorized release check and existing representative saves.
Where relevant verify original-to-CrySirror load, CrySirror write/reload, pending
resources, missing/corrupt input, cancellation and unchanged live state on error.
Track the tested versions and restoration of scene/text/input/audio separately.
A self-round-trip alone cannot prove the original game accepts exported files.
Running the original program follows the session's separate execution scope.

Maintain a readable profile table: recognized version, readable/writable fields,
reconstructed dependencies, import/export direction, migration and explicit
gaps. Document native contracts beside code and dated behavior separately.
Use [API compatibility](../../../docs/api/README.md#compatibility-assessment) for
changed defaults/versions/completion and explain consumer migration. A skill-only
change needs no save access, game run, build or regression suite.
