# Siglus

Mode: `integrated`. The default player registers this crate as `siglus`.
The maintained upstream VM, native resources, text, composition, effects,
audio, proc flow and original saves live in its internal modules. Source
revision/digests are recorded in [SOURCE.json](SOURCE.json).

## Host boundary

`adapter.rs` creates and drives the complete runtime on one session worker.
Only one native frame is outstanding. Input is queued while a frame is pending;
the host receives complete immutable frames and stereo 48 kHz float PCM.
The native Kira mixer has a session-owned pull backend and opens no audio device.
The original renderer runs offscreen; ordinary composition and wipe/effect paths
retain their native shaders. CPU readback transfers the opaque native backbuffer
to CrySirror's presenter without sharing device pointers between wgpu versions.

CrySirror owns the window/event loop, presentation resizing, input coordinates,
audio output and modal message boxes. The native canvas retains Gameexe dimensions.
Native state/device objects are created and destroyed on the worker; shutdown
joins it after admitted work completes. Framework snapshots are rejected;
native game saves use the host-selected per-client writable directory.

`resource_root`, `user_root` and optional `font_file`/`font_face_index` are injected
by the host. Gameexe/Scene resources use upstream decoding and automatic material
recovery; key.toml is an optional override. Recovered material stays private.
No game data or font is included. The host's supplied font is a fallback when a
configured game font is unavailable.

## Platforms and remaining host work

| Target | Boundary |
| --- | --- |
| Desktop | Registered integrated factory, offscreen native renderer, native PCM and host input/message-box bridge; validation is recorded separately. |
| iOS / Android | Reuse the player registration and retained host sessions; target compilation, lifecycle and device behavior require separate validation. |
| Browser | No connected integrated Siglus host; no fallback to the old extracted VM. |
| Vita / Switch | Original platform source is retained; CrySirror console device backends remain deferred. |

The initial host profile requires a local or OS-mounted game directory. Remote
storage providers, native desktop configuration/tool windows, full raw gamepad/
IME forwarding and optimized GPU interop need their own host work and evidence.
The retained legacy platform-shell modules are not the CrySirror app entry point.
Compilation/launch is not a claim of exhaustive game or hardware compatibility.
See [the dated validation](../../docs/validation/siglus-integrated.md) for observed
release/offscreen results.

The previous `crysirror-siglus-vm` extraction implementation remains available
only through an explicit reference registry, not the default player factory.
