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

The current source pin is `3ee43dab43854e2f9c614dd1d26213e17911d877`.
The synchronization includes source-curve property `0x8E`, sprite primary-bitmap getter
property `0x10` handling, profile-selected 64/128 SE channels and main-thread memory
regions, resource stream read/write/seek completion, composition backing-image
selection, text styling, path/encoding and empty-record fixes. Native semantics
remain upstream's; source presence does not establish whole-game coverage.

The previous import had no pinned source record. Its normalized sources matched
`0cfff1d274ecc4554d0af8af1c7b1c7908595e96` except the retained host changes, so that
revision was used for a three-way reconciliation. It is not claimed as the
original import revision. The host-specific frame/PCM, bootstrap, file policy
and installed-view changes were preserved.

## Platform evidence

| Target | Current boundary / evidence |
| --- | --- |
| macOS desktop | Default integrated player; this update's release compilation is recorded separately. |
| Windows / Linux | Shared desktop source retained; this update did not compile or run these targets. |
| iOS / Android | Retained native session/platform paths; this update did not validate target compilation or device lifecycle. |
| Browser | Upstream hosted-file/output modules retained; CrySirror integrated browser support is not established by this update. |
| Vita / Switch | Deferred CrySirror backends; original native GPU modules are source only here. |

The current default host profile uses local or OS-mounted directories. Remote
provider integration and other target/device behavior require their own work
and evidence. See [the update validation](../../docs/validation/bgi-upstream-sync.md)
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
