# BGI adapter status and ownership

The current first-class `crysirror-bgi-vm` and `crysirror-bgi` crates provide a
original native BP interpreter and a CrySirror semantic adapter. Images, canvases, layers,
glyphs and audio use CrySirror services. No original BGI application, renderer
or mixer is hosted.

The earlier hosted implementation was removed on 2026-10-26 under
[ADR 0047](adr/0047-engine-api-capability-extraction.md). That historical removal
does not describe the later BP adapter. ADRs 0037/0039 record the superseded work
and are not current integration instructions or acceptance evidence for this path.

See the [current adapter profile](../crates/crysirror-bgi/README.md) for its
configuration and unsupported operations, and [native adapter requirements](NATIVE_ADAPTERS.md)
for shared capability ownership. Migrating the original VM source does not establish complete native host ABI
+coverage, whole-adapter saves or whole-game compatibility. Dated
validation remains in [STATUS.md](../STATUS.md).
