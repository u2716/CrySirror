# ADR 0009: Siglus native renderer hosting (superseded)

Status: superseded by [ADR 0047](0047-engine-api-capability-extraction.md).
Original proposal: 2026-10-24. This is a retired design, not paused work.

## Historical proposal

The proposal embedded the complete `siglus_scene_vm` and ran its original
FramePlanner/WGSL renderer into an offscreen texture, then composed that texture
as a CrySirror layer. It proposed switching factories after a native fixture and
pixel comparison. That renderer boundary conflicts with the later engine-owned
capability requirement. Its vendoring, native-render plugin and factory-switch
recipe has been removed from the active instructions.

## Source and evidence

The investigation referenced siglus_rs
`5f358ef1360b0439e92aed2b749e8f25d8721954` and the former
`crates/siglus-upstream` package. The original record reported headless and
native-render compilation; it did not complete the proposed CrySirror host,
fixture or differential acceptance. Those observations are not acceptance of
current native integration and do not establish current renderer dependencies.

## Replacement

`crysirror-siglus-vm` is maintained as a first-class native VM and adapter.
Rendering, glyphs, layers, effects, audio and codecs use shared CrySirror services.
Original instruction/value/resource/wait/save semantics stay native. Missing
mechanisms must be added to shared APIs before their native mapping is accepted.

See [ADR 0047](0047-engine-api-capability-extraction.md),
[ADR 0061](0061-siglus-plain-message-engine-apis.md) and
[native adapter documentation](../NATIVE_ADAPTERS.md). Per-family source and
behavior records in [STATUS.md](../../STATUS.md) define actual coverage; no whole
Siglus compatibility follows from registration or a title capture.
