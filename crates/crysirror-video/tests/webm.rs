// SPDX-License-Identifier: MPL-2.0
//! WebM/VP8 demux and decode. `fixtures/testsrc_vp8.webm` is ffmpeg's
//! synthetic `testsrc` (64x48, 10 fps, 1 s, libvpx, 2 KB clusters) and the
//! oracle is libvpx's RGBA output for frame 5:
//!   ffmpeg -f lavfi -i testsrc=size=64x48:rate=10:duration=1 -c:v libvpx \
//!     -b:v 200k -auto-alt-ref 0 -cluster_size_limit 2000 testsrc_vp8.webm
//!   ffmpeg -i testsrc_vp8.webm -vf "select=eq(n\,5)" -vframes 1 \
//!     -f rawvideo -pix_fmt rgba testsrc_vp8.oracle.rgba
use std::io::Cursor;

const WEBM: &[u8] = include_bytes!("fixtures/testsrc_vp8.webm");
const ORACLE: &[u8] = include_bytes!("fixtures/testsrc_vp8.oracle.rgba");

#[test]
fn streaming_decode_matches_libvpx_oracle() {
    assert!(crysirror_video::webm::is_input(&mut Cursor::new(WEBM)).unwrap());
    let mut meta = None;
    let mut frames = Vec::new();
    crysirror_video::webm::decode_video_with_metadata(
        Cursor::new(WEBM),
        || false,
        |m| {
            meta = Some(m);
            Ok(())
        },
        |f| {
            frames.push(f);
            Ok(true)
        },
    )
    .unwrap();
    assert_eq!(meta.unwrap().size, [64, 48]);
    assert_eq!(frames.len(), 10);
    assert_eq!(frames[5].pts_ms, 500);
    let diff = frames[5].rgba.iter().zip(ORACLE).map(|(a, b)| a.abs_diff(*b)).max().unwrap();
    assert!(diff <= 1, "max channel difference {diff}");
}
#[test]
fn whole_file_stream_rewinds() {
    let mut s = crysirror_video::open(WEBM.to_vec()).unwrap();
    assert_eq!(s.info().container, crysirror_video::Container::WebM);
    let first = s.next_frame().unwrap().unwrap();
    while s.next_frame().unwrap().is_some() {}
    s.rewind().unwrap();
    assert_eq!(s.next_frame().unwrap().unwrap(), first);
}
#[test]
fn truncated_input_fails() {
    assert!(crysirror_video::webm::decode_video_with_metadata(Cursor::new(&WEBM[..200]), || false, |_| Ok(()), |_| Ok(true)).is_err());
}
/// `testsrc_vp8_alpha.webm`: the same pattern as yuva420p with alpha = 4*x,
/// encoded by libvpx (AlphaMode=1, BlockAdditional id 1); the oracle is
/// FFmpeg's libvpx decode of frame 5 as RGBA:
///   ffmpeg -f lavfi -i "testsrc=size=64x48:rate=10:duration=1,format=yuva420p,
///     geq=lum='lum(X,Y)':cb='cb(X,Y)':cr='cr(X,Y)':a='X*4'" -c:v libvpx \
///     -b:v 200k -auto-alt-ref 0 testsrc_vp8_alpha.webm
#[test]
fn alpha_stream_matches_libvpx_oracle() {
    const ALPHA: &[u8] = include_bytes!("fixtures/testsrc_vp8_alpha.webm");
    const ORACLE: &[u8] = include_bytes!("fixtures/testsrc_vp8_alpha.oracle.rgba");
    assert!(crysirror_video::webm::info(Cursor::new(ALPHA)).unwrap().alpha);
    let mut s = crysirror_video::open(ALPHA.to_vec()).unwrap();
    let frame = (0..6).map(|_| s.next_frame().unwrap().unwrap()).last().unwrap();
    let alpha_diff = frame.rgba.chunks(4).zip(ORACLE.chunks(4)).map(|(a, b)| a[3].abs_diff(b[3])).max().unwrap();
    assert!(alpha_diff <= 1, "max alpha difference {alpha_diff}");
}
