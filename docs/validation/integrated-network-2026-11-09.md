# Integrated resource transports and RFVP — 2026-11-09

## Implementation

Siglus, BGI and SoftPAL standalone repositories add optional `nfs` / `smb`
features, both default-off. Their commits are `7555aaa`, `fc0efa7`, `5f5b743`;
RFVP adds optional network host storage and thread-local session isolation in
`91ef488`. All four were pushed to their respective main branches. CrySirror
ports these source changes and injects its already mounted, client-scoped
storage rather than reconnecting from an unscoped native path. Resources are
read-only; native save roots remain local. BGI network resources preserve local user-file precedence and use its existing
Unix descriptor / Windows capability save policies; Windows was not compiled here.

The desktop registers `crysirror-rfvp-integrated`. Its complete native runtime,
software raster and native mixer are workspace source; the extraction crates
are preserved. One worker owns mutable native state and reads. Bounded
mailboxes permit one in-flight tick while the host retains the last completed
frame. Frames and mixed PCM use host output services. Shared codecs provide
seekable audio decoding; native Movie declarations use the managed media host.

## Compilation and bounded checks

On this macOS/aarch64 workspace, these release checks passed:

- Standalone Siglus VM library/CLI and asset crate with `nfs,smb`.
- Standalone BGI host-driven app with `nfs,smb`; default host-driven profile.
- Standalone SoftPAL library/CLI with `nfs,smb`; no-default profile.
- Standalone RFVP library with `no_std,host-session,nfs,smb`.
- CrySirror's three existing integrated adapters with `nfs,smb`.
- RFVP integrated adapter and desktop player with `nfs,smb`.

Default dependency trees for standalone Siglus/BGI/SoftPAL contain neither
`game-network` nor NFS/SMB SDKs. Two focused transport tests passed: independent
cloned seek cursors and scoped read-only mounted resources, including handle
lifetime after mount release. No full workspace test suite was run. The release
API example regenerated the reference.

The final host hardening also isolates legacy text/config/UI and exit state plus
fallback RNG per session thread. Hosted unknown syscalls and audio failures are
explicit errors. Its final release check passed. The short behavior checks below
preceded these lifecycle/input/error hardening changes; focus, function keys,
exit and multiple simultaneous sessions were not exercised.

A normal two-job player build was cancelled during lengthy ThinLTO linking. A
one-off top-level `-C lto=off` attempt failed because dependency objects contained
LLVM 22 bitcode and the system linker used LLVM 21. No repository build profile
was changed. The final player build retained Rust ThinLTO and bounded cross-module
function importing for this one build:

```sh
cargo rustc --release -p crysirror-desktop --bin crysirror --features nfs,smb -j 4 -- -C llvm-args=-import-instr-limit=0
```

This final release build passed. The generated `target/release/crysirror`
accepted `play --help`. The flag is a build-time optimization choice, not a
repository profile change or a runtime FPS measurement.

## Observed behavior

A private, device-free release harness ran the standalone repository's authored
`win95_painter_demo/painter.hcb` through the new registered adapter and runtime.
The required font was read into memory from macOS; no font file was added to the
repository or game directory. Native output was 1024×640; four native ticks
published a frame visual. A local run finished in 28 ms; maximum host tick was
0.62 ms.

The same scoped harness then injected a single 120 ms resource-read delay via a
read-only memory provider. Four native ticks and frame publication completed in
186 ms; the maximum host tick/startup poll was 1.46 ms. This exercises the
worker/read/output boundary without a server. It does not establish desktop
frame pacing or whole-game compatibility.

## Unexercised boundaries

No real NFS/SMB server, credentials, reconnect or measured network throughput;
no physical window/GPU presentation, audible audio, original save round trip,
movie or real-game long session. Windows/Linux were not cross-compiled. Mobile,
browser and console host lifecycles remain outside this change. The main release executable was rebuilt; its physical window and real-server
playback were not exercised.
