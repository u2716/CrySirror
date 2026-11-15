# Shared CPU source migration

License: MPL-2.0; full text is the monorepo root LICENSE. This crate forbids
unsafe code and has no window, GPU or audio SDK dependency.

The shared input, event, projection and UI mechanisms were developed in
xmoezzz's Siglus/RFVP implementations and are maintained here as engine services.
Native VM policies remain separate from the shared retained state and rendering
operations.

Changes to directly migrated code:

- input.rs exposes vmkey_to_vk_code publicly; algorithm and original tests stay.
- animation.rs adds serde snapshot derives with strict fields. The original
  distinction between stored value and interpolated value is preserved.
- projection.rs binds the original math to the engine's SpriteTransform type,
  including original radian rotation, object anchors, billboard/camera, culling
  and homogeneous W. Formatting changed; original tests remain.
- RFVP coordinate mapping exposes methods and content_rect for engine callers.
  It keeps half-open hit regions and aspect-fit letterbox behavior.

World/typed entity lifecycle, fixed-step scheduling, hierarchy, render extraction,
layer clips/mutations, widget tree/layout/focus/modal routing and persistence are
new engine integration. World tweens execute the migrated IntEvent. UI executes
the migrated InputState. Both raster and compute paths consume layer primitives.
No original VM, opcode/value decoder, native save codec or game UI is registered
by this migration. Those policies remain separate compatibility work.

Reproduce the actual original-vs-migrated algorithm comparison:

```sh
python3 tools/compare_scene_upstream.py --source-root /absolute/external/references
```

The historical differential driver compiles the reference input,
IntEvent, projection and RFVP helper code in a temporary driver, then compares
175,700 observations. No game data, SDK, credentials or fonts are imported.

`animation/adaptive_steps.rs` is independently expressed shared scheduling data
and arithmetic, using the complete SystemNNN CShakin routines as behavioral
evidence at78a05053c78b3b80065a9a209c6685a7c60a2e83. No original C++ code or
licence grant is imported. The authenticated old-module zero-lookahead policy
and later public-source prediction policy are explicit constructor parameters;
see docs/nnn-native-adaptive-clock-source-audit.json for complete static evidence
and independent test receipts. Native triggering/preferences remain adapter policy.
