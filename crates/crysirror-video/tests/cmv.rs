// SPDX-License-Identifier: MPL-2.0
use crysirror_video::{
    cmv::{Index, Movie},
    VideoStream,
};
use std::io::{Cursor, Read, Seek, SeekFrom};

#[test]
fn cmv_index_rejects_unbounded_frame_table() {
    let mut head = vec![0; 44];
    head[..4].copy_from_slice(b"CMV6");
    head[4..8].copy_from_slice(&44u32.to_le_bytes());
    head[8..12].copy_from_slice(&44u32.to_le_bytes());
    head[16..20].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(Index::read(&mut Cursor::new(head)).is_err());
}

#[test]
#[ignore = "requires an original CMV6 with indexed Vorbis audio"]
fn original_cmv_stream_renders_and_rewinds() -> anyhow::Result<()> {
    use anyhow::Context;
    let asset = std::env::var_os("CRYSIRROR_CMV_FILE").context("CMV fixture path")?;
    let mut reader = std::fs::File::open(asset)?;
    let index = Index::read(&mut reader)?;
    let size = index.metadata.size;
    let (offset, _) = index.audio.context("CMV Ogg track missing")?;
    reader.seek(SeekFrom::Start(offset))?;
    let mut header = [0; 4];
    reader.read_exact(&mut header)?;
    assert_eq!(&header, b"OggS");
    let mut movie = Movie::new(reader)?;
    let first = movie.next_frame()?.context("first picture missing")?;
    let second = movie.next_frame()?.context("second picture missing")?;
    assert_eq!([first.width, first.height], size);
    assert_eq!(first.pts_ms, 0);
    assert!(second.pts_ms > 0);
    assert!(second.rgba.chunks_exact(4).all(|pixel| pixel[3] == 255));
    movie.rewind()?;
    assert_eq!(movie.next_frame()?, Some(first));
    Ok(())
}
