# PS Vita host scaffold

The platform now has an SDK-free host contract and explicit driver slots.
A playable Vita backend and VPK packaging remain unimplemented.

Native GXM Cg VP/FP sources for a reduced software-ray profile and a shared
stackless BVH packer are now present. No OpenGL/vitaGL path is used. Current
sources are uncompiled; the full raster/effect shader port remains incomplete.
See [native shader sources and validation boundaries](../../docs/CONSOLE_SHADERS.md).

```sh
python3 tools/console_host.py psv
```

This performs a Release cross-check of `crysirror-console`, rebuilding
`core,alloc` with the installed nightly/rust-src. It does not execute a game,
link Vita devices, produce a VPK or test Vita3K/hardware.

`host.toml` records the target, package format and proposed capability-root
layout. The common coordinator in `crysirror-console` retains pending simulation
and presentation tickets, refuses another tick while pending, resets elapsed
time on resume, and requires real renderer/audio/storage/clock/worker support
before accepting an engine session. It contains no engine algorithms or SDK ABI.

Remaining concrete driver work:

1. Implement a VitaSDK host entry and lifetime owner (SDK-specific unsafe stays
   in that future host crate). Sample controllers/touch and monotonic time into
   CrySirror input/clock values.
2. Implement the shared engine rendering contract on Vita's available native
   graphics API. The desktop wgpu backend cannot simply be selected here.
   Start with shared retained 2D layers, images and glyphs, then evaluate 3D.
3. Connect scoped file/save/cache access and bounded asynchronous I/O/CPU tasks.
   Audit current std/capability filesystem and native dependency assumptions
   against the Vita target before linking the full Runtime/adapters.
4. Feed shared engine PCM into the native audio device; drain/cancel ownership
   on suspend and exit. Establish memory/texture budgets by measurement.
5. Add ELF-to-VPK packaging with the existing CrySirror icon, then separately
   validate compilation, emulator and hardware behavior.

The local VitaSDK headers/toolchain are reference/build inputs only; they are
not copied into this repository. No game data, fonts or SDK binary is bundled.

[Rust Vita target requirements](https://doc.rust-lang.org/rustc/platform-support/armv7-sony-vita-newlibeabihf.html)
document partial std support and VitaSDK/build-std requirements. The target
contract compiling does not establish that the full engine is portable yet.
