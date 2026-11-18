# Squirrel host interpreter

Reusable source VM for native script adapters. Rendering, storage and audio
remain outside this crate. No file, system, dynamic compiler or thread libraries
are exported to scripts. Rust callbacks exchange copied JSON values and bounded
closure references. The VM stays on its creating thread.

| Limit | Boundary |
| --- | --- |
| Source | 256 KiB, compiler recursion 128 |
| Native heap | 32 MiB, checked vector extents |
| Execution | Shared instruction allowance per invocation, including callbacks |
| Host values | Depth 16, 1 MiB per call, 4,096 table entries |
| Closures | 4,096 retained identities per session |

Vendored official Squirrel source at
`f9267f2f2a9afa1face89d8ea078c686331e30d1`. See `vendor/ORIGIN` and
`vendor/COPYRIGHT`. Changes add budgets and the isolated host bridge. A failed
native allocation poisons the session; its arena is reclaimed without resuming
partially unwound VM state. No game code or assets are included.

Desktop compilation is verified separately from mobile, browser and console
portability. This C++ boundary has not yet been built for those targets.
