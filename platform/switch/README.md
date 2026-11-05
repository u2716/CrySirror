# Nintendo Switch host scaffold

The platform now has an SDK-free host contract and explicit driver slots.
A playable Switch backend and NRO packaging remain unimplemented.

Native deko3d shader source tooling and a reduced software-BVH compute source
are now present. Current source revisions remain uncompiled, and source
translation failures are still outstanding. See
[native shader sources and validation boundaries](../../docs/CONSOLE_SHADERS.md).

```sh
python3 tools/console_host.py switch
```

This performs a Release cross-check of `crysirror-console` for
`aarch64-nintendo-switch-freestanding` with nightly/rust-src. It checks the
`no_std` contract only: this target does not provide the std-based Runtime's
filesystem/threading environment automatically. It does not produce or launch
an NRO. No local Switch SDK installation has been assumed.

`host.toml` records target/package identity and proposed game/save/cache roots.
`crysirror-console` shares the same pending-frame/lifecycle/ownership protocol
as the Vita scaffold; it owns no VM, image/text implementation or renderer.
Missing rendering, audio, scoped storage, clock or worker capabilities block
boot explicitly.

Remaining concrete driver work:

1. Select and pin a supported runtime/toolchain, then implement its main loop,
   input/monotonic clock and suspend/resume ownership in a dedicated host crate.
   A homebrew libnx driver and an owner-supplied official SDK driver would be
   distinct build inputs; neither is silently substituted for the other.
2. Connect the chosen native graphics backend to shared engine render state.
   Do not route this through desktop winit/wgpu or upload a legacy engine's
   composed screen as a general integration.
3. Supply bounded asynchronous I/O/CPU scheduling and capability-scoped storage.
   Port/audit std-dependent Runtime and adapter dependencies before claiming
   that a freestanding contract check builds the full engine.
4. Connect engine PCM/audio lifecycle and define device resource budgets from
   actual target measurements. Preserve per-adapter NLS/save semantics.
5. Implement an NRO package recipe with CrySirror's own icon; keep compilation,
   package validation and hardware behavior as separate results.

No SDK/header/signing files or game assets are included. No emulator or console
execution is claimed.

[Rust Switch target](https://doc.rust-lang.org/rustc/platform-support/aarch64-nintendo-switch-freestanding.html)
provides the target identity and freestanding build boundary.
