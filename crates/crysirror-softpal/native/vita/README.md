# SoftPAL Vita output

Retained from `xmoezzz/sena-rs` revision
`7f633dbb89408297827d6c1857eb8e97dc363dff`,
`platform/vita/source/{scene_gpu,cpu_memory}.{c,h}` and
`platform/common/{console,sena}.h`, under MPL-2.0.

The original GXM path reuses vita2d's linked `texture_v` and `texture_tint_f`
GXP programs. There are no missing private Cg/GXP files to generate for this
path. Its eleven blend states, texture cache, memory budgets and deferred
retirement remain in the native driver. The original large-image diagnostic
was moved from the standalone launcher's main file into this driver.

The optional `vita-gxm` feature enables the C build and the retained borrowed
scene C ABI in `native_vm::ffi`; both are off in the default desktop build.
The host selects scene mode before stepping, uploads borrowed texture/draw
views before the next step, and calls the native scene renderer on the same
owner thread with an initialized vita2d context. Free the scene cache before
destroying that context. The allocator source is retained for its original
memory accounting and optional host use; no global allocator is installed.
`src/vita.rs` exposes the output entry points and cursor ABI.

This imports the existing output implementation, not a standalone app, window,
input loop or audio device. No compilation or device execution was performed
for this import. The default CrySirror worker/output remains unchanged; a full
console session still needs its host/output lifecycle connection.
