// SPDX-License-Identifier: MPL-2.0
//! Bounded RIFF movie reads for the shared Media workers.
use super::{CinepakRgba, is_avi};
use crate::{VideoMetadata, mpeg::VideoFrame};
use anyhow::{Context, Result, ensure};
use std::io::{Read, Seek, SeekFrom};

pub fn is_input(input: &mut (impl Read + Seek)) -> Result<bool> {
    let start = input.stream_position()?;
    let mut head = [0; 12];
    let result = input.read_exact(&mut head);
    input.seek(SeekFrom::Start(start))?;
    match result {
        Ok(()) => Ok(is_avi(&head)),
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => Ok(false),
        Err(e) => Err(e.into()),
    }
}

fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
}

struct Chunk {
    kind: [u8; 4],
    start: u64,
    end: u64,
    next: u64,
}

impl Chunk {
    fn read(r: &mut (impl Read + Seek), pos: u64, limit: u64) -> Result<Self> {
        ensure!(
            pos <= limit && limit - pos >= 8,
            "truncated AVI chunk header"
        );
        r.seek(SeekFrom::Start(pos))?;
        let mut h = [0; 8];
        r.read_exact(&mut h)?;
        let size = u64::from(u32_at(&h, 4));
        let start = pos + 8;
        let next = start
            .checked_add(size + size % 2)
            .context("AVI chunk overflow")?;
        ensure!(next <= limit, "AVI chunk extends past list");
        Ok(Self {
            kind: h[..4].try_into()?,
            start,
            end: start + size,
            next,
        })
    }
    fn data(&self, r: &mut (impl Read + Seek), max: usize) -> Result<Vec<u8>> {
        let size = usize::try_from(self.end - self.start)?;
        ensure!(size <= max, "AVI chunk budget");
        r.seek(SeekFrom::Start(self.start))?;
        let mut data = vec![0; size];
        r.read_exact(&mut data)?;
        Ok(data)
    }
    fn list_type(&self, r: &mut (impl Read + Seek)) -> Result<[u8; 4]> {
        ensure!(self.end - self.start >= 4, "truncated AVI list type");
        r.seek(SeekFrom::Start(self.start))?;
        let mut value = [0; 4];
        r.read_exact(&mut value)?;
        Ok(value)
    }
}

struct Track {
    kind: [u8; 4],
    scale: u32,
    rate: u32,
    start: u32,
    length: u32,
    format: Vec<u8>,
}

pub struct Movie<R> {
    reader: R,
    tracks: Vec<Track>,
    video: usize,
    audio: Option<usize>,
    decoder: Option<Pictures>,
    limits: Vec<(u64, u64)>,
    pos: u64,
    packets: u64,
    samples: u64,
}

/// Open the same WMV3 packet decoder for in-memory VideoStream callers.
pub(crate) struct MemoryMovie {
    movie: Movie<std::io::Cursor<Vec<u8>>>,
    info: crate::VideoInfo,
    list: (u64, u64),
    first: u64,
}

impl MemoryMovie {
    pub(crate) fn new(bytes: Vec<u8>) -> Result<Self> {
        let movie = Movie::open(std::io::Cursor::new(bytes))?;
        let t = &movie.tracks[movie.video];
        let info = crate::VideoInfo {
            container: crate::Container::Avi,
            width: u32_at(&t.format, 4),
            height: u32_at(&t.format, 8),
            fps: Some(f64::from(t.rate) / f64::from(t.scale)),
        };
        let first = movie.pos;
        let list = movie.limits[0];
        Ok(Self {
            movie,
            info,
            list,
            first,
        })
    }
}

impl crate::VideoStream for MemoryMovie {
    fn info(&self) -> &crate::VideoInfo {
        &self.info
    }
    fn next_frame(&mut self) -> Result<Option<crate::Frame>> {
        self.movie
            .next_picture(|| false)?
            .map(|f| {
                Ok(crate::Frame {
                    pts_ms: u64::try_from(f.pts_ms)?,
                    width: f.width,
                    height: f.height,
                    rgba: f.rgba,
                })
            })
            .transpose()
    }
    fn rewind(&mut self) -> Result<()> {
        self.movie.pos = self.first;
        self.movie.limits.clear();
        self.movie.limits.push(self.list);
        self.movie.packets = 0;
        self.movie.decoder = None;
        Ok(())
    }
}

enum Pictures {
    Wmv3(Box<wmv_decoder::Wmv3Decoder>),
    Cinepak(CinepakRgba),
}

impl<R: Read + Seek> Movie<R> {
    pub fn open(mut reader: R) -> Result<Self> {
        let len = reader.seek(SeekFrom::End(0))?;
        let root = Chunk::read(&mut reader, 0, len)?;
        ensure!(
            root.kind == *b"RIFF" && root.list_type(&mut reader)? == *b"AVI ",
            "AVI signature"
        );
        ensure!(root.next == len, "unsupported multi-segment AVI");
        let mut pos = 12;
        let mut tracks = Vec::new();
        while pos < root.end {
            let c = Chunk::read(&mut reader, pos, root.end)?;
            pos = c.next;
            if c.kind != *b"LIST" {
                continue;
            }
            match &c.list_type(&mut reader)? {
                b"hdrl" => {
                    ensure!(
                        tracks.is_empty() && c.end - c.start <= 4 << 20,
                        "AVI header list budget"
                    );
                    let mut i = c.start + 4;
                    while i < c.end {
                        let st = Chunk::read(&mut reader, i, c.end)?;
                        i = st.next;
                        if st.kind == *b"LIST" && st.list_type(&mut reader)? == *b"strl" {
                            ensure!(tracks.len() < 100, "AVI stream count");
                            tracks.push(Self::track(&mut reader, &st)?);
                        }
                    }
                }
                b"movi" => {
                    let video = tracks
                        .iter()
                        .position(|t| t.kind == *b"vids")
                        .context("AVI video stream")?;
                    let audio = tracks.iter().position(|t| t.kind == *b"auds");
                    let t = &tracks[video];
                    ensure!(
                        t.format.len() >= 40 && t.rate != 0 && t.scale != 0,
                        "AVI video format"
                    );
                    let [w, h] = [u32_at(&t.format, 4), u32_at(&t.format, 8)];
                    crysirror_image::rgba_len(w, h)?;
                    return Ok(Self {
                        reader,
                        tracks,
                        video,
                        audio,
                        decoder: None,
                        limits: vec![(c.end, c.next)],
                        pos: c.start + 4,
                        packets: 0,
                        samples: 0,
                    });
                }
                _ => {}
            }
        }
        anyhow::bail!("AVI movie list missing")
    }

    fn track(r: &mut R, list: &Chunk) -> Result<Track> {
        let mut pos = list.start + 4;
        let mut head = Vec::new();
        let mut format = Vec::new();
        while pos < list.end {
            let c = Chunk::read(r, pos, list.end)?;
            pos = c.next;
            match &c.kind {
                b"strh" => head = c.data(r, 4096)?,
                b"strf" => format = c.data(r, 65536)?,
                _ => {}
            }
        }
        ensure!(head.len() >= 36, "AVI stream header");
        Ok(Track {
            kind: head[..4].try_into()?,
            scale: u32_at(&head, 20),
            rate: u32_at(&head, 24),
            start: u32_at(&head, 28),
            length: u32_at(&head, 32),
            format,
        })
    }
    pub fn metadata(&self) -> VideoMetadata {
        let t = &self.tracks[self.video];
        let frame_ms = 1000. * f64::from(t.scale) / f64::from(t.rate);
        VideoMetadata {
            size: [u32_at(&t.format, 4), u32_at(&t.format, 8)],
            duration_ms: Some((frame_ms * f64::from(t.length)).round() as u64),
            frame_ms: Some(frame_ms),
        }
    }
    pub fn has_audio(&self) -> bool {
        self.audio.is_some()
    }
    pub fn codec(&self) -> Result<&'static str> {
        let fourcc = &self.tracks[self.video].format[16..20];
        if fourcc.eq_ignore_ascii_case(b"WMV3") {
            Ok("wmv3")
        } else if fourcc == [0; 4] || fourcc.eq_ignore_ascii_case(b"cvid") {
            Ok("cinepak")
        } else {
            anyhow::bail!(
                "unsupported AVI video codec {:?}",
                String::from_utf8_lossy(fourcc)
            )
        }
    }
    fn packet(
        &mut self,
        stream: usize,
        suffixes: &[[u8; 2]],
        cancelled: &mut impl FnMut() -> bool,
    ) -> Result<Option<Vec<u8>>> {
        while let Some(&(end, next)) = self.limits.last() {
            if cancelled() {
                return Ok(None);
            }
            if self.pos == end {
                self.limits.pop();
                self.pos = next;
                continue;
            }
            let c = Chunk::read(&mut self.reader, self.pos, end)?;
            self.pos = c.next;
            if c.kind == *b"LIST" && c.list_type(&mut self.reader)? == *b"rec " {
                ensure!(self.limits.len() < 8, "AVI record nesting");
                self.limits.push((c.end, c.next));
                self.pos = c.start + 4;
                continue;
            }
            if c.kind[..2] == [(stream / 10) as u8 + b'0', (stream % 10) as u8 + b'0']
                && suffixes.iter().any(|s| c.kind[2..] == *s)
            {
                return Ok(Some(c.data(&mut self.reader, 32 << 20)?));
            }
        }
        Ok(None)
    }

    pub fn next_picture(
        &mut self,
        mut cancelled: impl FnMut() -> bool,
    ) -> Result<Option<VideoFrame>> {
        if self.decoder.is_none() {
            let f = &self.tracks[self.video].format;
            self.decoder = Some(match self.codec()? {
                "wmv3" => Pictures::Wmv3(Box::new(wmv_decoder::Wmv3Decoder::new(
                    u32_at(f, 4),
                    u32_at(f, 8),
                    &f[40..],
                )?)),
                _ => Pictures::Cinepak(CinepakRgba::new()),
            });
        }
        while let Some(bytes) = self.packet(self.video, &[*b"dc", *b"db"], &mut cancelled)? {
            let t = &self.tracks[self.video];
            let pts_ms = i64::try_from(
                (u128::from(self.packets) + u128::from(t.start)) * u128::from(t.scale) * 1000
                    / u128::from(t.rate),
            )?;
            self.packets += 1;
            if bytes.is_empty() {
                continue;
            }
            let (width, height, rgba) = match self.decoder.as_mut().unwrap() {
                Pictures::Wmv3(dec) => {
                    let Some(frame) =
                        dec.decode_frame_owned(&bytes, false, u32::try_from(pts_ms)?)?
                    else {
                        continue;
                    };
                    (
                        frame.width,
                        frame.height,
                        wmv_decoder::yuv420p_to_rgba_scaled(&frame, frame.width, frame.height),
                    )
                }
                Pictures::Cinepak(dec) => dec.decode(&bytes)?,
            };
            return Ok(Some(VideoFrame {
                pts_ms,
                width,
                height,
                rgba,
            }));
        }
        Ok(None)
    }

    pub fn pcm(
        &mut self,
        mut cancelled: impl FnMut() -> bool,
        mut publish: impl FnMut(i64, u32, u16, Vec<i16>) -> Result<bool>,
    ) -> Result<()> {
        let Some(index) = self.audio else {
            return Ok(());
        };
        let t = &self.tracks[index];
        let f = &t.format;
        ensure!(f.len() >= 16 && f[..2] == [1, 0], "AVI requires PCM audio");
        let channels = u16::from_le_bytes(f[2..4].try_into()?);
        let rate = u32_at(f, 4);
        let bits = u16::from_le_bytes(f[14..16].try_into()?);
        ensure!(
            (1..=8).contains(&channels)
                && (8000..=192000).contains(&rate)
                && matches!(bits, 8 | 16),
            "AVI PCM parameters"
        );
        let start = if t.rate == 0 {
            0
        } else {
            u64::try_from(u128::from(t.start) * u128::from(t.scale) * 1000 / u128::from(t.rate))?
        };
        while let Some(bytes) = self.packet(index, &[*b"wb"], &mut cancelled)? {
            ensure!(
                bytes.len() % (usize::from(channels) * usize::from(bits / 8)) == 0,
                "misaligned AVI PCM packet"
            );
            let pcm: Vec<_> = if bits == 16 {
                bytes
                    .chunks_exact(2)
                    .map(|s| i16::from_le_bytes([s[0], s[1]]))
                    .collect()
            } else {
                bytes
                    .into_iter()
                    .map(|v| (i16::from(v) - 128) << 8)
                    .collect()
            };
            let pts = i64::try_from(start + self.samples * 1000 / u64::from(rate))?;
            self.samples += u64::try_from(pcm.len() / usize::from(channels))?;
            if !pcm.is_empty() && !publish(pts, rate, channels, pcm)? {
                break;
            }
        }
        Ok(())
    }
}
