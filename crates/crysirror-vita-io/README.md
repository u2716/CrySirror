# crysirror-vita-io

64-bit read-only file access on PS Vita. This is a platform helper, not an
engine integration, so it has no onboarding mode.

On `armv7-sony-vita-newlibeabihf`, Rust std's file backend uses newlib's
`off_t`, which the libc crate defines as a 32-bit `c_long` for this target.
`File::seek` and `Metadata::len` then fail or wrap above 2 GiB. Game archives
are often larger: SugarPot `grp.aos` is 2.8 GB, and its AOS2 probe failed on
Vita until reads went through SceIo. SceIo itself is 64-bit (`SceOff`).

| API | Vita | Other targets |
| --- | --- | --- |
| `File::open` | `sceIoOpen` read-only, size from `sceIoGetstatByFd` | `std::fs::File` |
| `File::read_at` | `sceIoPread` at a 64-bit offset (no shared cursor) | seek + read under a lock |
| `Read`/`Seek` for `File` | cursor over `read_at` | same |
| `stat(path)` | `sceIoGetstat`: (is directory, 64-bit size) | `std::fs::metadata` |

Users:
- `crysirror-capfs` console backend: read-only opens and file metadata sizes.
- `crysirror-launch` local source files: engine detection and archive mounts.

Writes still use std; save files are far below 2 GiB.
