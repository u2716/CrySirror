# ADR 0008: Native engine platform-seam hosting (superseded)

Status: superseded by [ADR 0047](0047-engine-api-capability-extraction.md).
Original decision: 2026-10-24. Hosting removed: 2026-10-26.

## Historical decision

The original RFVP path kept an entire legacy engine, including its software
renderer, behind platform file/time/input/presentation seams. It uploaded the
composed screen through present_surface. That boundary was rejected because it
did not extract reusable image/text/UI/effect/audio capabilities into CrySirror.
The former vendoring, build-feature and embedding instructions are retired.

## Replacement

Native interpreters are first-class workspace crates. Adapters retain original
instruction/value/wait/resource/save policy and call shared engine resources and
operations. Reusable mechanisms belong in shared engine crates with independent
APIs. A matching screen is not proof of this ownership boundary.

See [ADR 0047](0047-engine-api-capability-extraction.md),
[native adapter requirements](../NATIVE_ADAPTERS.md) and the current
[RFVP adapter source](../../crates/crysirror-rfvp/src/lib.rs).

## Historical evidence scope

The old painter fixture and `docs/rfvp-differential.json` recorded hosted-path
behavior; they do not validate the replacement integration. The old source path
was `crates/rfvp`, and the old adapter used a process-global interpreter with
silent/movie/snapshot limitations. Those are historical observations, not current
configuration instructions. Dated results remain in [STATUS.md](../../STATUS.md).
