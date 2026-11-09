# RFVP integrated runtime

Mode: `integrated`. The desktop registers this crate as `rfvp`.
`crysirror-rfvp-native` maintains the complete native runtime source; the
preserved `crysirror-rfvp` / `crysirror-rfvp-vm` extraction path remains inactive.

`Config` selects the client-relative resource `root`, required `default_font`,
`nls` (`sjis`, `gbk`, `utf8`) and native `screen`. HCB, BIN packs and loose
resources use scoped read-at handles; saves keep the existing `rfvp/` namespace
on the local save mount. Resources are read-only. Original game slots are
native; framework snapshots explicitly fail.

One worker owns native state, software composition and native audio mixing.
CrySirror owns the window, original-canvas input, cursor resources, movie decode through native pack/loose-file readers,
GPU presentation and audio device. Frames are full, straight RGBA8 with native
size and tightly packed rows. A single tick may be in flight; bounded mailboxes
retain the last complete picture during loading. The native canvas is limited
to 8,388,608 pixels; resizing the host window does not resize it.

Native mixed stereo PCM is published at 48 kHz in 10 ms blocks, at most 100 ms
per admitted tick. Shared codecs decode seekable WAV/Vorbis/MP3/FLAC resources
on the worker. Source completion returns to the native audio state before its
next tick. The native software mixer admits 24 simultaneous voices (2 BGM,
16 SE per-kind limits); loaded PCM/encoded bytes are bounded to 256 MiB total. Focus loss pauses native ticks and audio mixing; shutdown flushes global saves
on the owner without waiting on the window thread. Disconnected outputs retire
the session and its resources on that worker. No second audio device is opened.

`nfs` and `smb` are opt-in features, both off by default. The desktop forwards
these features to all four integrated backends. Network games retain the same
CLI, e.g. `crysirror play nfs://host/export/game`; network credentials stay in the
host provider. Native VM/config/save data contains no URL credentials. No game
resource tree is copied locally. The standalone RFVP `host_api::network` provides
an optional read-only directory/filesystem with separately injected save storage.

| Platform | Current boundary |
| --- | --- |
| macOS | Release compiled; scoped offscreen evidence is recorded separately |
| Windows / Linux | Portable worker/CPU/storage source connected; not compiled in this session |
| iOS / Android / browser | Native packaging, worker lifecycle and host output not connected by this change |
| Vita / Switch | Deferred |

Hardware presentation, audible playback, real-server latency and whole-game
compatibility require their own execution evidence. Software composition is an
explicit CPU path; this change makes no GPU rendering or fixed-FPS guarantee.
