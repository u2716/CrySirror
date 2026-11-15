# RFVP Vita output

Retained from `xmoezzz/rfvp` revision
`fa53713017da3efb936404c82bc673367b9b608b`,
`platform/vita/host/c/rfvp_psv_gpu.c` and
`platform/vita/host/src/{render,texture_policy,thumbnail}.rs`, under MPL-2.0.

The original driver reuses vita2d's linked color and textured GXP programs,
including RFVP's SRC_ALPHA/ONE additive blend patch. There is no separate
private Cg/GXP set to copy. Texture scaling/filtering, clipping, tint, delayed
frees and save-preview sampling are retained. The Rust bridge lives under
`src/host_api/vita`; only crate/module paths were adjusted. Existing source
tests were not imported into this backend.

The optional `vita-gxm` feature builds the driver only on the Vita target and
exports `host_api::vita::PsvRenderer`. The host must initialize the driver,
own the renderer and native session on one thread, then drop every renderer
resource before calling shutdown. It owns input, timing, audio and lifecycle;
the original standalone app is not imported.

The integrated crate forwards the feature without changing its default CPU
session renderer. Native GXM session selection and connection to CrySirror's
console output remain separate work. This import was not compiled or executed.
