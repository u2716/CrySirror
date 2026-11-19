# Shared CPU video

`open` dispatches OMV/Theora, MPEG, WMV and MP4 from bytes. `isobmff::Mp4Stream`
also accepts an owned, seekable reader so the runtime can read through an asset
capability without copying an entire movie. It decodes AVC/H.264, HEVC/H.265 and
AV1 into engine-owned RGBA8 frames with presentation timestamps. It opens no
window, GPU, audio device, filesystem path or decoder process.

The Media service owns workers, bounded four-frame queues, cancellation, error
publication and playback clocks. `Mp4Stream::seek` resets codec reference state,
seeks to a sync sample, and discards pre-roll before publishing the target frame.
The decoder itself runs serial within its already-budgeted engine worker.

Sources and exact versions/commits are in `SOURCE.json`. OxideAV is an MIT Rust
library dependency; it is not a legacy game engine. The three codec Git commits
are pinned because their same-numbered published releases lacked decoded output
shape/pixel-format reporting. Rust 1.95 or newer is required. The existing AC-3
implementation stays at oxideav-ac3 0.0.10, sharing core 0.1.42.

Limits are explicit: 1 GiB encoded input, 64 MiB container metadata, 64 MiB
compressed packets and RGBA frames. Sequence headers are checked before decode
because these upstream codec revisions do not enforce their `DecoderLimits`
parameter. Coded dimensions are bounded to 8192 per axis and 8,388,608 pixels.
These are per-stream limits, not a bound on the entire decoder's reference-picture
memory. Layered codecs, encrypted tracks, and sample-description switching fail
explicitly. HDR tone mapping and hardware decode are not implemented. The RGBA8
conversion uses nearest chroma expansion; 10-bit input is quantized to 8-bit.

MP4 audio is not yet implemented. With Media audio enabled, a movie containing an
audio track reports an explicit error. The caller may deliberately set
`audio: false` to play its video. Audio-free MP4 works with default Media options.

The default release tests use self-generated 64×64, five-frame clips: H.264
Baseline/Main with B frames, HEVC Main/Main10, and AV1 Main 8/10-bit. They assert
frame count, dimensions, PTS, independent FFmpeg reference RGB samples,
seek/rewind identity, cancellation, EOF and malformed input. These samples do not
establish exhaustive codec/profile conformance or realtime 4K performance.
`tests/fixtures/generate_mp4.py` reproduces test data and reference samples;
FFmpeg is only the offline fixture encoder/reference decoder. Tests themselves
need no FFmpeg, external assets or device.

## CMV5, CMV6, CMV7, CMV8 and CMV9

The shared decoder reads an indexed CMV stream through a seekable reader
and decodes JBPD or JBPE into top-down RGBA. CMV9 retains an immutable
static base while dynamic blocks are decoded per picture. The runtime
passes the indexed Ogg subrange to its existing Vorbis decoder.

## WebM / VP8

`webm` demuxes WebM (Matroska `DocType` `webm`/`matroska`) over a seekable
reader: EBML header, `Info` (timecode scale, duration), the first video
`TrackEntry`, and `SimpleBlock`/`BlockGroup` packets in sized or unknown-size
`Cluster`s. Laced video blocks, non-VP8 video and WebM audio decoding fail
explicitly; alpha `BlockAdditions` are not decoded (pictures are opaque).
Element headers are read through a 256 KiB buffer with a locally tracked
position, so archive-backed readers are not hit per byte. Header elements are
bounded to 1 MiB and blocks to 32 MiB.

VP8 decoding uses the crates.io `vp8-decoder` =0.1.2 (RFC 6386 reference port,
MIT OR Apache-2.0 with the RFC's BSD-3-Clause tables; notice in NOTICE.md).
Output I420 is converted to RGBA8 with BT.601 studio-range integer arithmetic.

Validation (2026-11-18, Apple M5, release): `tests/webm.rs` decodes a
self-generated 64x48 libvpx clip (10 frames, small clusters) against an FFmpeg
RGBA oracle (max channel difference 1), checks rewind identity and truncated
input. Locally, three shipped Silverio Ragnarok movies (1920x1080 382 frames,
960x540 540 frames, 960x544 170 frames) matched FFmpeg's frame counts and its
libvpx frame 30/60 pixels within 1 level; 1080p decoding measured about
4.8 ms/frame from a file. Game movies are not part of the repository.
