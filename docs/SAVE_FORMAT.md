# Save envelope v1

A new wrapper format, **not** the RFVP or Siglus original save format. Native
save import/export belongs to the corresponding compatibility adapter.

All integer fields are unsigned little-endian. No native pointer or usize is
serialized.

| Order | Field |
|---|---|
| 1 | Eight bytes `CRYSAVE\0` |
| 2 | u32 envelope version, currently 1 |
| 3 | u32 VM payload schema |
| 4 | u32 services payload schema |
| 5 | u32 length + UTF-8 engine ID |
| 6 | u32 length + UTF-8 game ID |
| 7 | u32 length + UTF-8 content revision |
| 8 | u32 length + opaque services payload |
| 9 | u32 length + opaque VM payload |
| 10 | u32 CRC32 over every preceding byte |

Identity strings are nonempty, at most 256 bytes, and cannot contain control
characters. Decode checks total/section bounds, checksum, format version,
expected identity and absence of trailing data before allocating payload copies.
CRC32 detects accidental damage and is not an authentication mechanism.

Both payload schemas must match the consumer or pass an explicit migration before
restore. A different engine/game/content revision is not silently accepted.
Runtime implements payload serialization and staged framework restore: candidate
VM/service state is decoded and validated before publication. An adapter may
reject a snapshot state it cannot reconstruct; this is not a guarantee of every
subsystem or native-save profile. Platform durability and recovery semantics
remain the responsibility of the storage backend implementing scoped atomic
writes; the desktop DiskVfs supplies that backend. See
[architecture](ARCHITECTURE.md) for ownership and
[ADR 0099](adr/0099-staged-original-save-restore.md) for the separate bounded
resource-dependent original-save restore coordinator.


## Historical Siglus audio schema checkpoints

The schema numbers below identify the ADR 0076/0083 checkpoints, not the latest
Siglus VM snapshot schema. Current VM schemas and reconstruction limits are
defined by each adapter; do not infer migration support from these older notes.

Siglus runtime payload schema 2 at ADR 0076 included logical BGM state.
Schema-1 payloads require migration; the envelope rejects a mismatch. Pending
resource checkpoints retain the original uncommitted instruction. Active audio
restores declarations at authored range start after candidate commit; worker
receipts, decoder read-ahead, device cursors and DSP history are never serialized.
Original Siglus file codecs/header layouts stay separate and unchanged. BGM loop/
pause/delay/volume persistence uses those existing fields and shared preparation;
this bounded profile does not establish every original save variant.


The ADR 0083 Siglus runtime audio checkpoint used schema **5**, following BGM
schema 2, PCM/SE schema 3 and KOE schema 4. No automatic migration of older
payloads is supplied. PCMCH declarations reconstruct running/READY/paused cues
at source start; remaining integer-ms resume delay is scheduled again. A pending
stop reconstructs stopped and a pause fade reconstructs fully paused. These are
logical checkpoints, not exact decoder/DSP/device-history snapshots. Original
PCMCH file records preserve native volume and loop/READY/delay predicates/order;
load treats delay only as a restart predicate and does not schedule it again.
