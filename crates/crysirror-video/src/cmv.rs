// SPDX-License-Identifier: MPL-2.0
//! Indexed CMV5, CMV6, CMV7, CMV8 and CMV9 movies. Only the index, static base and current
//! frame payload are read into the decoder. Audio is exposed as an Ogg range.
mod jbpd;

use crate::{Container, Frame, VideoInfo, VideoMetadata, VideoStream};
use anyhow::{ensure, Context, Result};
use crysirror_image::ImageSurface;
use std::io::{Read, Seek, SeekFrom};

const HEADER: usize = 44;
const ENTRY: usize = 20;
const MAX_FRAMES: usize = 1 << 20;
const MAX_PACKET: usize = 64 * 1024 * 1024;

#[derive(Debug)]
struct FormatError(&'static str);
impl std::fmt::Display for FormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}
impl std::error::Error for FormatError {}

#[derive(Clone, Copy)]
struct Entry {
    offset: u64,
    size: u32,
}

#[derive(Clone, Copy)]
enum Codec {
    Cmv5,
    Jbpd,
    Cmv7,
    Jbpe,
    Delta { base: Entry },
}

pub struct Index {
    frames: Vec<Entry>,
    pub audio: Option<(u64, u64)>,
    pub metadata: VideoMetadata,
    rate: u32,
    scale: u32,
    codec: Codec,
}

pub fn is_input(reader: &mut (impl Read + Seek)) -> Result<bool> {
    reader.seek(SeekFrom::Start(0))?;
    let mut magic = [0; 4];
    let read = reader.read_exact(&mut magic);
    reader.seek(SeekFrom::Start(0))?;
    match read {
        Ok(()) => Ok(matches!(&magic, b"CMV5" | b"CMV6" | b"CMV7" | b"CMV8" | b"CMV9")),
        Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => Ok(false),
        Err(error) => Err(error.into()),
    }
}

fn word(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(
        bytes[offset..offset + 4]
            .try_into()
            .expect("validated CMV structure"),
    )
}

impl Index {
    pub fn read(reader: &mut (impl Read + Seek)) -> Result<Self> {
        let length = reader.seek(SeekFrom::End(0))?;
        ensure!(
            length <= crate::MAX_VIDEO_BYTES as u64,
            "CMV encoded byte budget"
        );
        reader.seek(SeekFrom::Start(0))?;
        let mut head = [0; HEADER];
        reader.read_exact(&mut head)?;
        let alpha = matches!(&head[..4], b"CMV7" | b"CMV8");
        let delta = &head[..4] == b"CMV9";
        // 0x14004c360 reads CMV5 and CMV7 through the index alone, without the checks below.
        let early = matches!(&head[..4], b"CMV5" | b"CMV7");
        ensure!(
            early || matches!(&head[..4], b"CMV6" | b"CMV8" | b"CMV9"),
            "CMV signature"
        );
        let count = word(&head, 16) as usize;
        ensure!((1..=MAX_FRAMES).contains(&count), "CMV frame count");
        let table_end = HEADER + (count + 1) * ENTRY;
        // A zero data offset means the payload follows the index (0x14004c360).
        let data_at = match word(&head, 4) {
            0 => table_end as u64,
            offset => u64::from(offset),
        };
        ensure!(
            (early || length == u64::from(word(&head, 8))) && data_at <= length,
            "CMV length"
        );
        ensure!(data_at >= table_end as u64, "CMV index range");
        let rate = word(&head, 20);
        let scale = word(&head, 24);
        ensure!(rate != 0 && scale != 0, "CMV frame rate");
        let [w, h] = [word(&head, 28), word(&head, 32)];
        crysirror_image::rgba_len(w, h)?;
        let channels = if alpha { 4 } else { 3 };
        ensure!(early || word(&head, 36) == channels * 8, "CMV bit depth");
        let output_bytes = u64::from(w) * u64::from(h) * u64::from(channels);
        let frame_ms = 1000. * f64::from(scale) / f64::from(rate);
        ensure!(
            frame_ms.is_finite() && (0.1..=10000.).contains(&frame_ms),
            "CMV frame duration"
        );
        let duration_ms = Some((count as u64 * u64::from(scale) * 1000).div_ceil(u64::from(rate)));

        let mut raw = [0; ENTRY];
        let mut frames = Vec::new();
        frames.try_reserve_exact(count)?;
        let mut audio = None;
        for i in 0..=count {
            reader.read_exact(&mut raw)?;
            let size = word(&raw, 4);
            let decoded = u64::from(word(&raw, 8));
            let kind = word(&raw, 12);
            let offset = data_at + u64::from(word(&raw, 16));
            ensure!(offset + u64::from(size) <= length, "CMV packet range");
            if i < count {
                ensure!(
                    early || (kind == 2 && word(&raw, 0) as usize == i),
                    "CMV frame index order"
                );
                // Scene clips include a 54-byte BMP header in decoded metadata.
                ensure!(
                    early || decoded == output_bytes || decoded == output_bytes + 54,
                    "CMV decoded packet size"
                );
                ensure!(
                    size > 0 && size as usize <= MAX_PACKET,
                    "CMV frame packet size"
                );
                frames.push(Entry { offset, size });
            } else if early {
                // Header word 40 announces the trailing audio entry.
                if word(&head, 40) != 0 && size > 0 {
                    audio = Some((offset, u64::from(size)));
                }
            } else if kind == 0 && size > 0 {
                ensure!(!delta, "CMV9 audio is unsupported");
                audio = Some((offset, u64::from(size)));
            } else {
                ensure!(size == 0, "CMV extra index entry");
            }
        }
        let codec = if delta {
            let mut bytes = [0; 4];
            reader.read_exact(&mut bytes)?;
            let size = u32::from_le_bytes(bytes);
            let offset = table_end as u64;
            ensure!(
                size >= 8 && size as usize <= MAX_PACKET && offset + u64::from(size) == data_at,
                "CMV9 static block range"
            );
            Codec::Delta {
                base: Entry { offset, size },
            }
        } else {
            ensure!(
                early || data_at == table_end as u64,
                "CMV resource table spacing"
            );
            match &head[..4] {
                b"CMV5" => Codec::Cmv5,
                b"CMV7" => Codec::Cmv7,
                b"CMV8" => Codec::Jbpe,
                _ => Codec::Jbpd,
            }
        };
        Ok(Self {
            frames,
            audio,
            metadata: VideoMetadata {
                size: [w, h],
                duration_ms,
                frame_ms: Some(frame_ms),
            },
            rate,
            scale,
            codec,
        })
    }

    fn timestamp(&self, frame: usize) -> u64 {
        (frame as u64 * u64::from(self.scale) * 1000) / u64::from(self.rate)
    }
}

pub struct Movie<R> {
    reader: R,
    index: Index,
    info: VideoInfo,
    next: usize,
    packet: Vec<u8>,
    base: Option<(Vec<u8>, ImageSurface)>,
}

impl<R: Read + Seek> Movie<R> {
    pub fn new(mut reader: R) -> Result<Self> {
        let index = Index::read(&mut reader)?;
        let [width, height] = index.metadata.size;
        let info = VideoInfo {
            container: Container::Cmv,
            width,
            height,
            fps: Some(f64::from(index.rate) / f64::from(index.scale)),
        };
        let mut packet = Vec::new();
        let base = if let Codec::Delta { base } = index.codec {
            read_packet(&mut reader, base, &mut packet)?;
            let grid = word(&packet, 4);
            let cols = (grid & 65535) as usize;
            let rows = (grid >> 16) as usize;
            ensure!(
                cols == (width as usize).div_ceil(16) && rows == (height as usize).div_ceil(16),
                "CMV9 static grid"
            );
            let mask_end = 8 + (cols * rows).div_ceil(8);
            let mask = packet
                .get(8..mask_end)
                .context("CMV9 truncated static bitmap")?
                .to_vec();
            let image = jbpd::decode_masked(&packet[mask_end..], &mask, true, None)?;
            ensure!(
                [image.width, image.height] == [width, height],
                "CMV9 static dimensions"
            );
            Some((mask, image))
        } else {
            None
        };
        Ok(Self {
            reader,
            index,
            info,
            next: 0,
            packet,
            base,
        })
    }

    pub fn metadata(&self) -> VideoMetadata {
        self.index.metadata
    }

    pub fn decode_next(&mut self) -> Result<Option<Frame>> {
        let Some(&entry) = self.index.frames.get(self.next) else {
            return Ok(None);
        };
        read_packet(&mut self.reader, entry, &mut self.packet)?;
        let decoded = match &self.base {
            Some((mask, base)) => jbpd::decode_masked(&self.packet, mask, false, Some(base)),
            None => match self.index.codec {
                Codec::Cmv5 => jbpd::decode_cmv5(&self.packet),
                Codec::Cmv7 => jbpd::decode_cmv7(&self.packet),
                Codec::Jbpe => jbpd::decode_alpha(&self.packet),
                _ => jbpd::decode(&self.packet),
            },
        }?;
        ensure!(
            [decoded.width, decoded.height] == self.index.metadata.size,
            "CMV frame dimensions differ from header"
        );
        let frame = Frame {
            pts_ms: self.index.timestamp(self.next),
            width: decoded.width,
            height: decoded.height,
            rgba: decoded.rgba,
        };
        self.next += 1;
        Ok(Some(frame))
    }
}

fn read_packet(reader: &mut (impl Read + Seek), entry: Entry, buffer: &mut Vec<u8>) -> Result<()> {
    reader.seek(SeekFrom::Start(entry.offset))?;
    if buffer.len() < entry.size as usize {
        buffer.try_reserve_exact(entry.size as usize - buffer.len())?;
    }
    buffer.resize(entry.size as usize, 0);
    reader.read_exact(buffer)?;
    Ok(())
}

impl<R: Read + Seek + Send> VideoStream for Movie<R> {
    fn info(&self) -> &VideoInfo {
        &self.info
    }
    fn next_frame(&mut self) -> Result<Option<Frame>> {
        self.decode_next()
    }
    fn rewind(&mut self) -> Result<()> {
        self.next = 0;
        Ok(())
    }
}
