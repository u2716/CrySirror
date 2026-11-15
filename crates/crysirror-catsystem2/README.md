# CatSystem2

Mode: `adopt`.

[Developer API reference, opcodes, gcalls, class methods and selectors](../../docs/native-api/catsystem2.md).
[Source provenance and native module identities](SOURCE.json).

| Module | Responsibility |
| --- | --- |
| `src/kcs.rs` | Bounded KCS v2 stack/memory ABI and pending native calls. |
| `src/native_interpreter.rs` | Native expression/flag/string semantics. |
| `src/adapter.rs` | Registered `catsystem2` factory and shared-service completion. |
| `src/native_classes/` | Native receiver/class and gcall policy. |
| `src/fes_runtime.rs`, `src/fes_*` | FES commands, tasks, motions and continuations. |
| Native resource, sound and image modules | Scoped workers over shared resource/Image/Audio services. |

KCS, FES and class-call IDs have separate namespaces. Image/page creation,
input registration and audible playback are separate completion contracts.
Native addresses are checked guest-memory offsets. Framework snapshots fail
explicitly; original save compatibility and individual argument profiles are
specified in the reference and source handlers.

Desktop, mobile, browser and console execution results require their own
validation. Source availability does not establish a working backend. The host
owns windows, events and output devices; native rendering uses shared engine
capabilities. No original EXE/DLL executes and no original compositor is hosted.
