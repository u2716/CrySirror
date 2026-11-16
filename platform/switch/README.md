# Nintendo Switch host

The current source includes the libnx/deko3d shell, NRO build recipe and shared
console-player session dispatch in `runtime/`. Adopted sessions use the shared
renderer's custom backend; integrated sessions retain their declared native
route. Source presence does not establish a successful build or device result.
See [platform characteristics and memory](PLATFORM.md) for the custom target,
allocation/layout, fences and platform limitations.

## Historical scaffold and portability probe

The notes below describe the earlier scaffold/probe stages, not the current
implementation status. Consult current source and dated validation notes for
later driver and package work.

The platform now has an SDK-free host contract and explicit driver slots.
A playable Switch backend and NRO packaging remain unimplemented.

Native deko3d shader source tooling and a reduced software-BVH compute source
are now present. Current source revisions remain uncompiled, and source
translation failures are still outstanding. See
[native shader sources and validation boundaries](../../docs/CONSOLE_SHADERS.md).
The reduced 3D feature tier for this platform is `crysirror-render-tier`.
VN composition is not reduced.

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

### Historical std Runtime probe for adopted engines (2026-11-13)

Integrated runtimes (Siglus, RFVP, BGI, Softpal) keep their upstream Horizon
hosts and renderers as internal modules. Adopted engines share one console
path: the std-based CrySirror Runtime and adapters, a libnx host behind
`crysirror-console`, and a deko3d executor of the shared render extraction.
No adopted VM gets its own Switch renderer.

Toolchain inputs, following the siglus_rs Switch port (`d34d980`,
`platform/switch/rust/aarch64-switch.json`, copied here unchanged):
`os = "horizon"`, `env = "newlib"`, std rebuilt with
`-Zbuild-std=std,panic_abort -Zjson-target-spec`, devkitA64 GCC for C
dependencies. Local: devkitPro at `/opt/devkitpro` (devkitA64, libnx,
`libdeko3d.a`, `uam`), nightly 2026-09-17 with rust-src.

Check-only probe of `crysirror-runtime` (no link, no NRO, no execution):

```sh
export PATH=/opt/devkitpro/devkitA64/bin:$PATH DEVKITPRO=/opt/devkitpro
export CC_aarch64_switch=aarch64-none-elf-gcc AR_aarch64_switch=aarch64-none-elf-ar
export CFLAGS_aarch64_switch="-march=armv8-a+crc+crypto -mtune=cortex-a57 -mtp=soft -fPIE -ffunction-sections -D__SWITCH__ -I/opt/devkitpro/libnx/include"
CARGO_TARGET_DIR=target/switch RUSTFLAGS='--cfg getrandom_backend="custom"' \
  cargo +nightly check --release -p crysirror-runtime \
  --target platform/switch/rust/aarch64-switch.json \
  -Zbuild-std=std,panic_abort -Zjson-target-spec \
  --config 'patch.crates-io.libc.path="<libc-0.2.191 + patches/libc-0.2.191-horizon-aarch64.patch>"'
```

Blockers found, in order (each was cleared before the next appeared):

1. `libc` 0.2.191 (workspace graph): its Horizon module is the 32-bit 3DS
   newlib, so aarch64 lacks `dev_t`/`ino_t`/`off_t`. Fixed by
   `patches/libc-0.2.191-horizon-aarch64.patch`, the siglus_rs 0.2.178 layout
   patch ported to 0.2.191 (applied to a crates.io copy, not vendored).
2. `getrandom` 0.2 (`rsa` in crysirror-resource): needs its `custom`
   feature on Horizon. Added a Horizon-only dependency line; the host must
   register libnx entropy (siglus_rs uses `randomGet`).
3. `filetime` (via `tar` <- `pure-magic`): no `lutimes`/`UTIME_OMIT`.
   Resolved 2026-11-14: `pure-magic` is not linked on console targets.
4. `memmap2` (via `pure-magic`): resolved with item 3. The next
   blocker is `tokio`/`mio` (`Waker`, `ucred::get_peer_cred`) via
   reqwest/hyper in the network stack.
   Both 3 and 4 come from content sniffing in `crysirror-resource/src/sniff.rs`;
   the console build needs that sniffing without `pure-magic`'s archive/mmap
   dependencies.
5. Not yet reached, known from the manifest: `rustix`/`cap-std` back the
   Runtime's capability VFS (`crysirror-runtime/src/vfs.rs`, `assets.rs`) and
   have no Horizon backend; a console VFS must keep the same capability-root
   rules over std/newlib files under `sdmc:/switch/CrySirror`. `tokio`/`mio`/
   `socket2` arrive through crysirror-network.

Renderer: `crysirror-wgpu` is about 18.9k lines over 46 WGSL files in
`crysirror-shaders`. The deko3d path needs the frame extraction split from its
wgpu executor so that a second executor issues the same passes;
`crysirror-shader-tools` already exports per-entry GLSL 460 for `uam`. The
siglus_rs Horizon renderer (`render/horizon`, `runtime/source/gpu.c`) is the
working precedent for that executor shape. Vita follows the same split with
GXM.

Compilation of the full graph, an NRO, emulator or hardware behaviour have not
been achieved.

No SDK/header/signing files or game assets are included. No emulator or console
execution is claimed.

[Rust Switch target](https://doc.rust-lang.org/rustc/platform-support/aarch64-nintendo-switch-freestanding.html)
provides the target identity and freestanding build boundary.
