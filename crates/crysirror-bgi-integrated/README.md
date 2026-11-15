# BGI integrated runtime

Mode: `integrated`. The default desktop registers this crate as `bgi`.
The complete native core, archive/image readers, BP interpreter, graphics,
text/effects, audio and original saves are maintained as internal modules.
The upstream source revision and per-file digests are in [SOURCE.json](SOURCE.json).

## Host boundary

`adapter.rs` creates and drives the runtime on a session worker. CrySirror owns
the window/event loop, presentation and audio device. The default
`crysirror-host` path returns immutable completed CPU-composed RGBA frames and
session-owned Kira stereo float PCM at 48 kHz. It does not enable the original
standalone desktop event loop or CPAL device. PCM is drained between bounded
clock catch-up slices; the native buffer rejects more than two seconds of
undrained audio. Input, focus, close requests and native dialogs cross the
existing host bridge. Native canvas dimensions remain separate from presentation.

The host injects canonical resource/save roots, bootstrap name and optional font.
Original archives stay in place, including the installed `Install/` view.
Native writes to the logical primary root use a private writable overlay;
subsequent native reads see that overlay before read-only installed resources.
This includes `BGI.gdb` and keeps the existing broader configuration/save policy.
Upstream's narrower writable-name whitelist is not substituted for this policy.
Native saves remain runtime-owned; framework snapshots are unsupported.
NLS follows the native profile, optional host `gbk` policy and native UTF-8 mode
selectors. No game data, game-specific key or font is bundled.

## Upstream synchronization

The current source pin is `2f8a9272a8f7ca075d82ac88fb802fa7247b3a99`.
It includes newer BP instruction profiles and native buffer/operation handling,
linked sprite masks, bitmap animation lifetime, raw image and knob-input fixes,
localized archive views, game-directory fonts and substitute-font metrics.
Scene and global movie samples retain independent presentation schedules.

CrySirror preserves its existing frame/PCM, custom bootstrap, private storage,
network capability, input/dialog and console output seams. Bootstrap detection
uses the retained runtime's declared image bounds (including trailing bytes)
and language tags. Explicit loose/custom BP startup remains available.
Previous complete-runtime pin: `3ee43dab43854e2f9c614dd1d26213e17911d877`.

## Platform evidence

| Target | Current boundary / evidence |
| --- | --- |
| macOS desktop | Default integrated player; this update's release compilation is recorded separately. |
| Windows / Linux | Shared desktop source retained; this update did not compile or run these targets. |
| iOS / Android | Retained native session/platform paths; this update did not validate target compilation or device lifecycle. |
| Browser | Upstream hosted-file/output modules retained; CrySirror integrated browser support is not established by this update. |
| Vita | Original GXM driver, two Cg/GXP programs and callback bridge retained behind optional `vita-gxm`; full console host/output connection remains incomplete. |
| Switch | Original native GPU modules retained; CrySirror host/output connection remains incomplete. |

The [Vita output module](native/vita/README.md) reuses the original shaders and
native draw implementation. It is not enabled by the default desktop player.

The current default host profile uses local or OS-mounted directories. Remote
provider integration and other target/device behavior require their own work
and evidence. See [the update validation](../../docs/validation/bgi-upstream-sync-2026-11-15.md)
and [the engine guide](../../docs/BGI.md).

## Optional network resources

`nfs` and `smb` are off by default. The desktop forwards its matching features.
The session worker receives the existing client-scoped storage capability and
performs resource reads there; the host event loop never reconnects or reads
native resources during a VM tick. Native file cursors share bounded block
caches without staging game data. Save data remains in the local user root.
Network gameplay and latency were not exercised against a real server in this
change.
The network resource binding uses the existing Unix descriptor or Windows capability save policy. Only the macOS branch was compiled in this session.
