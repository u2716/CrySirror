# RFVP upstream integration

Mode: `integrated`.

| Item | Recorded value |
| --- | --- |
| Repository | [https://github.com/xmoezzz/rfvp](https://github.com/xmoezzz/rfvp) |
| Complete-runtime base | `9f07adef2eb5d559777f9bf59a67b756875cb24f` |
| Source record | [SOURCE.json](../crysirror-rfvp-native/SOURCE.json) |

## Changes retained in CrySirror

- Thread-local globals and clock for one fixed owner thread per session.
- Host movie, cursor and source-completion seams retained from the extraction implementation.
- Original native composition and mixing; shared streaming audio codec provider and managed frame/PCM presentation.
- Optional nfs/smb storage capability; native shutdown flushes global saves without executing another VM instruction.
- Exit/legacy UI/text/config state and fallback RNG are session-local; unknown syscalls and audio output errors fail visibly in the host-session profile.
- Movie sources retain the native BIN/loose-file VFS reader and are decoded through managed host media workers.

When updating, compare the new upstream revision with the source record and
retain the host changes above. Update this record and SOURCE.json together.
Later selected imports, rendering/output paths and platform boundaries are in
[the integration reference](../../docs/INTEGRATED_ENGINES.md). Compilation and
observed behavior belong in the linked validation notes, separately from the
imported source revision.
