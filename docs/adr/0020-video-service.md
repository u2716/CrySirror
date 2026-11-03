# 0020: Engine video service over the Siglus decoders

Date: 2026-10-25. Status: implemented; MPEG-2 and WMV2 behavior-validated,
OMV/Theora compile-only.

## Context

Video codecs are shared engine capabilities. The original Siglus codec sources
were `siglus_omv_decoder`/`theora-rs` (OMV and Ogg/Theora), `na_mpeg2_decoder`
(MPEG-1/2 PS/ES with MPA/AC-3) and `na_wmv_player` (ASF with WMV2/WMV3/VC-1/WMA).
The adopted first-class codec crates retain provenance and applicable notices in
their SOURCE.json records; the former siglus-upstream package is retired.

## Decision

`crysirror-video` (safe, device-free) sniffs the container (`OggS`, the ASF
GUID, MPEG pack/sequence start codes, or an OMV header whose size points
at an Ogg page) and returns RGBA8 frames with presentation times through one
`VideoStream` trait. Unknown or undecodable data fails.

- OMV/Theora uses the VM's own header parser and RGB/RGBA/YUV conversion via
  the new `siglus_scene_vm::movie::omv_public` wrappers (UPSTREAM change 5), so
  alpha rows and display sizes match Siglus exactly.
- MPEG mirrors `siglus_scene_vm`: the whole stream is pushed, then flushed,
  and frames are timed by index at the sequence header's frame rate.
- ASF uses `AsfWmv2Decoder` timestamps and `yuv420p_to_rgba_scaled`.

`Player` presents the frame due at a time, loops by rewinding and reports the
end. Playback state is engine data: each client `ClientWorld` holds named
`VideoSpec {asset, looping, playing, time_ms, slot}` changed by
`WorldCommand::SetVideo`, and `time_ms` advances with the 16 ms world step while
playing and the client is not paused. Saves therefore restore the same
picture. The runtime keeps a decoder per (client, name), rebuilds it when the
asset/loop changes or time goes backwards, and publishes new frames to the
client's dynamic surface slot (`__surface/1000+`, at most 8 videos per
client). Any layer image or texture path naming that surface shows the video.
Scripts: `World.playVideo(name, asset, {loop?, paused?}) → surface path`,
`pauseVideo`, `resumeVideo`, `seekVideo`, `stopVideo`, `video(name)`. The Python
schema accepts `set_video`.

## Validation (2026-10-25)

- `crysirror-video/tests/decode.rs` with synthetic ffmpeg fixtures (solid
  colors, no game data): MPEG-2 PS 25 fps and WMV2/ASF 10 fps. Exact frame
  counts, timestamps, colors (≥250 / ≤5), rewind, timed `Player` presentation,
  looping, NaN step rejection, and visible failure of unknown/truncated input.
- `crysirror-engine/tests/video.rs`: script → world → runtime → surface; red
  then blue by world time, pause holds time, backward seek rebuilds, end of
  stream, stop, path escape and unknown-option rejection.

**Upstream finding:** `na_mpeg2_decoder` emits the final MPEG-2 picture only
partly decoded at flush (mean blue 170 versus ffmpeg's 255). The same pipeline
backs `siglus_scene_vm`; the test isolates and documents that frame instead
of hiding it.

**Not validated:** OMV/Ogg-Theora decoding has no fixture (the local ffmpeg
lacks a Theora encoder and no Theora/OMV file is available). Only container
sniffing and the failure path are tested; this is compile-only.

## Follow-up implementation and gaps

The original full-memory MPEG/no-audio boundary was extended by shared incremental
Media decoding and worker-prepared PCM routing (ADRs 0048/0072); final DSP/output
workers followed in 0079/0080. Codec-specific buffering and seek limitations must
still be reviewed separately. PBR video-material binding and hardware decoding
are not established by this ADR. The Theora fixture limitation above belongs to
its original validation checkpoint, not a claim that all video paths are untested.
