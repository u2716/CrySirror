# Documentation index

Use the guides below for current interfaces. Dated verification entries describe
only the source and environment tested in that entry. Implementation, successful
compilation and observed behavior are separate claims.

## Start here

| Task | Guide |
| --- | --- |
| Build, play an installed game, render a project or package data | [Desktop guide](DESKTOP_API.md) |
| Understand native VM ownership and supported profiles | [Native adapters](NATIVE_ADAPTERS.md) |
| Adopt/reverse-implement or integrate a native engine | [Engine onboarding](ENGINE_ONBOARDING.md) and [safe onboarding skill](../.agents/skills/crysirror-engine-onboarding/SKILL.md) |
| Find classes, commands and JS/TS types | [Generated API reference](api/README.md) |
| Connect local MCP project tools | [MCP](MCP.md) |
| Generate project JSON from an offline recipe | [TS/JS authoring](AUTHORING.md) |

## Engine systems

- [Architecture and ownership](ARCHITECTURE.md)
- [World, Layer, UI, Text and Audio](ENGINE_SYSTEMS.md)
- [Async services and completion boundaries](ASYNC_SERVICES.md)
- [Frame pipeline, streaming and profiling](FRAME_PIPELINE.md)
- [Resource formats and archive services](RESOURCES.md)
- [Bitmap operations](BITMAPS.md) and [physics](PHYSICS.md)
- [Framework saves and native-save boundaries](SAVE_FORMAT.md)
- [Adapter migration requirements](MIGRATION.md)
- [RealLive](REALLIVE.md), [BGI](BGI.md),
  [SoftPAL](NATIVE_ADAPTERS.md#bgi-and-softpal) and
  [NScripter](../crates/crysirror-nscripter/README.md)
- [Local game imports](LOCAL_GAME_IMPORT.md)
- [Application icons](../icon/README.md)

## Decisions and evidence

[ADRs](adr/README.md) record design decisions; a superseded decision is historical,
not a current implementation instruction. The [capability roadmap](adr/0041-engine-capability-roadmap.md)
is a maintained checklist, not a fresh whole-workspace test report.
[STATUS.md](../STATUS.md) preserves dated implementation, compilation and behavior
results, including failures and later corrections. Individual game milestones
must not be generalized into full engine or whole-game compatibility.

The following records are historical research snapshots. Their old “current” or
“next” statements apply to the recorded investigation, not today's code:

- [Initial source acquisition audit](SOURCE_AUDIT.md)
- [2026-10-26 priorities review](engine-next-priorities-20261026.md)
- [Siglus transform audit](native-audit-siglus-transform.md)
- [KiriKiri bootstrap research](krkr-bootstrap-gap.md)
- [KiriKiri shipped-plugin inventory](krkr-plugin-audit.md)

API signatures come from code generation. User-facing CLI options live in the
desktop guide. Adapter-specific semantic limits belong in the adapter docs;
historical source hashes and test receipts remain in their original records.

- [Native application packages](NATIVE_PACKAGING.md): macOS, iOS, Android and engine-specific NLS launch settings.
