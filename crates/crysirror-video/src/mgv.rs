// SPDX-License-Identifier: MPL-2.0
//! CMVS MGV1 movies: a frame table, an optional Ogg Vorbis track and an Ogg Theora track.
//! Layout from `MGV_InputFile` in mog3x64.dll of the 2021-10-07 CMVS build.
use anyhow::{ensure, Context, Result};
use std::io::{Read, Seek, SeekFrom};

const HEADER_BYTES: u64 = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Layout {
    pub frames: u32,
    pub size: [u32; 2],
    /// Frame rate as numerator over denominator.
    pub rate: [u32; 2],
    /// Byte offset and length of the Ogg Vorbis track.
    pub audio: Option<(u64, u64)>,
    /// Byte offset and length of the Ogg Theora track.
    pub video: (u64, u64),
}

impl Layout {
    pub fn frame_ms(&self) -> f64 {
        f64::from(self.rate[1]) * 1000. / f64::from(self.rate[0])
    }

    pub fn duration_ms(&self) -> u64 {
        (f64::from(self.frames) * self.frame_ms()).round() as u64
    }

    pub fn read(reader: &mut (impl Read + Seek)) -> Result<Self> {
        let start = reader.stream_position()?;
        reader.seek(SeekFrom::Start(0))?;
        let mut head = [0; HEADER_BYTES as usize];
        reader.read_exact(&mut head).context("MGV1 header")?;
        let end = reader.seek(SeekFrom::End(0))?;
        reader.seek(SeekFrom::Start(start))?;
        let word = |at: usize| u32::from_le_bytes([head[at], head[at + 1], head[at + 2], head[at + 3]]);
        ensure!(&head[..4] == b"MGV1" && u64::from(word(4)) == HEADER_BYTES, "MGV1 header");
        let (video, audio, frames) = (u64::from(word(8)), u64::from(word(12)), word(16));
        let size = [word(20) & 0xffff, word(20) >> 16];
        let rate = [word(28), word(24)];
        ensure!(frames > 0 && size[0] > 0 && size[1] > 0, "MGV1 picture dimensions");
        ensure!(rate[0] > 0 && rate[1] > 0, "MGV1 frame rate");
        let audio_at = HEADER_BYTES + u64::from(frames) * 4;
        let video_at = audio_at.checked_add(audio).context("MGV1 audio extent")?;
        ensure!(
            video > 0 && video_at.checked_add(video) == Some(end),
            "MGV1 track extents disagree with the file"
        );
        Ok(Self {
            frames,
            size,
            rate,
            audio: (audio > 0).then_some((audio_at, audio)),
            video: (video_at, video),
        })
    }
}

pub fn is_input(reader: &mut (impl Read + Seek)) -> Result<bool> {
    let start = reader.stream_position()?;
    reader.seek(SeekFrom::Start(0))?;
    let mut magic = [0; 4];
    let found = reader.read_exact(&mut magic).is_ok() && &magic == b"MGV1";
    reader.seek(SeekFrom::Start(start))?;
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn file(audio: usize, video: usize) -> Vec<u8> {
        let mut out = b"MGV1".to_vec();
        for word in [32, video as u32, audio as u32, 2, 720 << 16 | 1280, 1000, 24000] {
            out.extend(u32::to_le_bytes(word));
        }
        out.resize(32 + 8 + audio + video, 0);
        out
    }

    #[test]
    fn tracks_follow_the_frame_table() {
        let layout = Layout::read(&mut Cursor::new(file(5, 9))).unwrap();
        assert_eq!(layout.size, [1280, 720]);
        assert_eq!(layout.audio, Some((40, 5)));
        assert_eq!(layout.video, (45, 9));
        assert_eq!(layout.frame_ms(), 1000. / 24.);
    }

    #[test]
    fn inconsistent_extents_are_rejected() {
        let mut bytes = file(5, 9);
        bytes.pop();
        assert!(Layout::read(&mut Cursor::new(bytes)).is_err());
        assert!(!is_input(&mut Cursor::new(b"CMV6".to_vec())).unwrap());
    }
}
