# SoftPAL upstream integration

Mode: `integrated`.

| Item | Recorded value |
| --- | --- |
| Repository | [sena-rs](https://github.com/xmoezzz/sena-rs) |
| Complete-runtime base | Not recorded; do not infer it from a component import. |
| Selected Vita driver import | `7f633dbb89408297827d6c1857eb8e97dc363dff` |
| Component source record | [Vita driver provenance](native/vita/README.md) |

## Changes retained in CrySirror

- Maintain the complete asset/script/VM implementation as internal workspace modules.
- Use a CrySirror-owned session worker, input and managed frame/PCM output.
- Inject resource, writable save and font roots; retain native NLS/aspect/save policy.
- Preserve the native Vita scene-driver boundary as a separate platform component.

Before comparing complete upstream updates, establish the missing complete-runtime
source revision. A component import does not supply that identity. Rendering,
local changes and platform boundaries are listed in
[the integration reference](../../docs/INTEGRATED_ENGINES.md); compilation and
observed behavior remain separate validation records.
