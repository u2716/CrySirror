# BGI Vita output

Retained from `xmoezzz/ethornell-rs` revision
`fc0efa7586f130aa5dd29d4846aac5dc66124dad`,
`platform/vita/player/native/gpu.c`, `platform/vita/player/shaders` and
`platform/console/include/ethornell_console.h`, under MPL-2.0.

`vita-gxm` enables the native driver build on the Vita target. `build.rs`
embeds the two existing GXP programs as aligned C arrays; it does not compile
Cg. The matching vertex/fragment sources are retained alongside the binaries.
The driver keeps the original texture uploads, XRGB alpha handling, five blend
states, clipping, vertex rings and delayed texture retirement.

`src/vita.rs` exposes the original initialization/shutdown functions and the
callback table consumed by `native_render::Renderer::new_native`. A single
CrySirror console output owner initializes GXM, owns all renderers on that
thread, drops their resources, then shuts GXM down. The standalone player,
event loop and audio device are not imported.

Source/resource import and feature wiring are implemented. This import has
not been compiled or run in CrySirror; the default integrated desktop session
still uses its existing output. Full console session/output wiring is separate.
