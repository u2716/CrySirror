# ADR 0104: N2System slot checkpoints use a CrySirror save version

Status: accepted 2026-11-17; stage 1 (capture/write) and stage 2 (restore) implemented for path-addressable scenes.

## Context

Original N2System slots are five NPF files per slot (`script`, `frames`, `bklg`,
`val`, plus the shared `common/val.npf`) and a thumbnail. Totono trial 1.0.0.0
`sub_4780B0` writes them from the live native engine: `sub_47ABD0` and
`sub_44DC20` copy process-memory structures verbatim, including host pointer
values and per-object heap layouts. Those bytes cannot be reproduced or
validated by this adopted VM, and the adopter must not serialize process memory
or pointers (ADR 0099, native-save skill).

`Save(0)` (common variables, NPSF encoding and read history) remains native-
compatible and unchanged. Only slots 1..9999 are affected.

## Decision

Non-zero slots use an explicit CrySirror checkpoint, version `n2system-slot` 1,
stored at `n2system-v1/slots/NNNN.json` in the client's scoped save overlay.
It is never written under an original NPF name and is never presented as an
original save. Original slot import/export is `Unsupported`.

Stage 1 captures, at the point the `Save` call completes (the stored script
position is the instruction after `Save`, as in `sub_415060`): VM variables,
flags and arrays; every script thread with frames, stack, pending call, wait and
enabled state; the entity tree and aliases; retained layer descriptions; dialogue
pages, backlog history, read marks and request flags. The file is validated and
written atomically off the VM thread.

Entities that display runtime-generated bitmaps (screen captures, surfaces,
effect output, movies) have no path-addressable source. Stage 1 refuses to
write a checkpoint containing them and names the entity; it never writes a
checkpoint that silently loses them.

## Restore (stage 2)

`Load(n)` reads and validates the checkpoint and prepares every module it names
that the live VM lacks on a shared worker, with no VM mutation. The owner then
re-validates (entry/canvas, routines, layers, wait properties) and commits once
at the frame boundary: it tears down the live world, restores variables, flags,
arrays, the entity tree and retained layers (parents first), threads, text
pages, backlog, choices, sound parameters and the text style. Clock-relative
waits are rebased to the restore instant. Images and decoders are acquired
lazily from their catalog paths; sounds that were playing restart at the saved
position. Choice widgets are registered again by the normal refresh. Any
preflight failure leaves the live game untouched. The calling thread never
resumes: its script continues from the checkpoint's `Save` successor.

Validated 2026-11-17 against Kimi to Kanojo (headless): a quick-save written
mid-dialogue was applied in a fresh process at the title stage (4 modules
loaded by the worker) and reproduced the saved scene.

Not restored: in-flight reveal/fade progress (pages show fully), motion tracks
mid-flight, the Mt19937 state, thumbnails, and scenes with runtime-generated
bitmaps (still refused at save).

## Consequences
- The CrySirror schema is versioned independently of the VM ABI and of the
  original NPF versions; migrations are explicit.
- Framework `snapshot`/`restore` stays unsupported for this adapter.
