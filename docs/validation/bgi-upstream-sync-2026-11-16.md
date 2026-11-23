# BGI upstream synchronization, 2026-11-16

## Source

The integrated runtime advances from
`2f8a9272a8f7ca075d82ac88fb802fa7247b3a99` to
[`0bc6d1c0d5b662acc9006be94896324600ad05f1`](https://github.com/xmoezzz/ethornell-rs/commit/0bc6d1c0d5b662acc9006be94896324600ad05f1),
including 15 upstream commits. The original upstream checkout was not modified.

| Area | Imported behavior |
| --- | --- |
| VM/profile | Integer formatting, native strings, native version/compatibility checks, compact/fused instructions and dialog/sprite contracts. |
| Text/images | Multilingual encoding/font handling, bitmap loading, NVL paragraph/page retention and text-window presentation. |
| Backgrounds | Transition state and camera pan origins for BackF and selected BackML layers. |
| Audio | Mixed-rate intro/loop resampling, channel replacement coalescing and propagation of playback/control errors. |
| Movies | MP4/H.264, VP9 and AV1 with AAC-LC audio, bounded frame queues and retained source ownership. |
| Optional mobile output | Retained upstream mixer-preserving route recovery, selected only outside the default CrySirror host backend. |

The [source record](../../crates/crysirror-bgi-integrated/SOURCE.json) records
167 upstream files and their current local hashes. Thirty-eight mapped files
changed or were added. Eight mapped files retain local differences after
normalization. All file hashes were checked after reconciliation. MP4 demuxer
dependencies remain at upstream's locked `rff-*` version 0.4.7.

## Host boundary

Mode remains `integrated`. CrySirror keeps its session worker, frame/PCM queues,
input/dialog bridge, custom bootstrap, installed-resource view, scoped storage
and network overlays. Existing native console modules are preserved. This
update does not change the desktop composition/transfer path.

The mobile device-output module is inactive in `crysirror-host` sessions; the
host continues to own devices and lifecycle. Its readiness handling must not
call the optional mobile device backend on a session-owned PCM backend. The
standalone renderer's missing-device pause condition likewise must not pause
a valid CrySirror CPU-frame session.

The frame/PCM adapter is now gated by its `crysirror-host` feature. Without that
feature it called methods which only exist with session output enabled. Direct
native-renderer consumers retain `HostRuntime`; default player registration and
configuration remain unchanged. No reflective engine API signature changed.

## Compilation and behavior

| Command | Result |
| --- | --- |
| `cargo check --release --offline -p crysirror-bgi-integrated --features nfs,smb` | Passed with the final MP4 dependency pins. |
| `cargo check --release --offline --locked -p crysirror-bgi-integrated --no-default-features --features desktop-window,nfs,smb` | Passed after gating the session-only adapter. |

Checks used the existing macOS Release target tree. No test suite, regression
run, game, original executable, emulator or screen capture was run. These
results do not establish movie pixels/audio, full-game compatibility or
Windows/Linux/mobile/browser/console target behavior. The desktop executable
was not rebuilt by these library checks.
