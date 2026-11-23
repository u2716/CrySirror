# Siglus upstream integration

Mode: `integrated`.

| Item | Recorded value |
| --- | --- |
| Repository | [https://github.com/xmoezzz/siglus_rs](https://github.com/xmoezzz/siglus_rs) |
| Complete-runtime base | `bef39ea90c745a4888a92a09d18be1eecfdd10ca` |
| Source record | [SOURCE.json](SOURCE.json) |

## Synchronized source

The complete maintained runtime source is synchronized through `bef39ea`,
including Unicode text layout/font fallback, native save/menu handling, window
state updates and the corrected mask-number inheritance from PR #106.
Standalone application binaries and upstream CI/release workflows are outside
the integrated-runtime boundary. CrySirror retains its own host and packaging.

The internal source mapping and original-file hashes are recorded in
`SOURCE.json`. Selected network code is now included in the runtime baseline.

## Changes retained in CrySirror

- Maintain complete VM/assets/archive source as internal workspace modules; rewrite crate paths without an engine Git dependency.
- Create and own each complete runtime on a bounded session worker; register the integrated factory in the default native player.
- Add offscreen native backbuffer production and CPU readback for CrySirror-owned presentation; retain ordinary, wipe and effect shader paths.
- Use a session-owned Kira pull backend producing stereo float PCM at 48000 Hz; CrySirror owns the output device.
- Inject host resource, writable-save and font roots; use upstream resource-pair material recovery without writing caller game data.
- Forward host keyboard, pointer, focus and modal message-box results; preserve native key policy and global/native save handling.
- Use the project icon and caller font fallback, including injected byte fonts and face index; no upstream font or proprietary game resources are copied.
- Preserve the current Vita GXM shader paths and Switch host-owned PCM backend.
- Correct Vita display captures to libvita2d's 960-pixel pitch. The CrySirror PSV player constructs the complete session on one explicitly sized 2 MiB owner thread; retain this startup stack requirement when changing console hosts.

When updating, compare the new upstream revision with the source record and
retain the host changes above. Update this record and SOURCE.json together.
Later selected imports, rendering/output paths and platform boundaries are in
[the integration reference](../../docs/INTEGRATED_ENGINES.md). Compilation and
observed behavior belong in the linked validation notes, separately from the
imported source revision.

Latest synchronization: [2026-11-14 validation](../../docs/validation/siglus-upstream-sync-2026-11-14.md).
