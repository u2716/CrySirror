//! Seekable IMOAVI100 intra-frame movies over a caller-owned reader.
//! The native MotionMovie loader keeps sparse FRMIDX offsets, signed frame
//! redirects and content identities. Image decoding and presentation remain
//! independent of the originating VM, windows and devices.
use crate::mpeg::{DecodeStats, VideoFrame};
use anyhow::{Context, Result, bail, ensure};
use crysirror_image::ImageSurface;
use std::{
    collections::BTreeSet,
    io::{Read, Seek, SeekFrom},
};

const MAX_FRAMES: u32 = 1_000_000;
const MAX_PAYLOAD: u32 = 64 * 1024 * 1024;
const FRAME_HEADER: u64 = 28;
const MAX_REDIRECTS: usize = 4096;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Info {
    pub width: u32,
    pub height: u32,
    pub frames: u32,
    pub duration_ms: u64,
    pub frame_ms: f64,
    pub has_audio: bool,
}
impl Info {
    /// MotionMovie4DD350 selects floor(clamp(time)*count/duration), then
    ///4DD284 clamps to the last picture. The source holds, rather than loops.
    pub fn frame_at_ms(&self, time_ms: u64) -> Result<u32> {
        ensure!(
            self.frames > 0 && self.duration_ms > 0,
            "IMOAVI invalid time metadata"
        );
        Ok(
            ((u128::from(time_ms.min(self.duration_ms)) * u128::from(self.frames)
                / u128::from(self.duration_ms))
            .min(u128::from(self.frames - 1))) as u32,
        )
    }
    /// Ceil is necessary: publishing frame1 at41ms for a24fps source would
    /// precede the original floor-based selection boundary at42ms.
    pub fn pts_ms(&self, index: u32) -> Result<u64> {
        ensure!(
            self.frames > 0 && self.duration_ms > 0 && index < self.frames,
            "IMOAVI frame timestamp range"
        );
        Ok(
            (u128::from(index) * u128::from(self.duration_ms)).div_ceil(u128::from(self.frames))
                as u64,
        )
    }
}
#[derive(Clone, Copy)]
struct Header {
    info: Info,
    base: u64,
    region: u64,
    file_end: u64,
}
#[derive(Clone, Copy)]
struct Entry {
    next: u32,
    content: u32,
    redirect: i32,
    size: u32,
    codec: u32,
}

fn word(reader: &mut impl Read) -> Result<u32> {
    let mut bytes = [0; 4];
    reader
        .read_exact(&mut bytes)
        .context("IMOAVI truncated word")?;
    Ok(u32::from_le_bytes(bytes))
}
fn tag(reader: &mut impl Read, expected: &[u8; 8]) -> Result<()> {
    let mut bytes = [0; 8];
    reader
        .read_exact(&mut bytes)
        .context("IMOAVI truncated tag")?;
    ensure!(
        &bytes == expected,
        "IMOAVI missing {} header",
        String::from_utf8_lossy(expected)
    );
    ensure!(word(reader)? == 100, "unsupported IMOAVI section version");
    Ok(())
}
pub fn is_input(reader: &mut (impl Read + Seek)) -> Result<bool> {
    let position = reader.stream_position()?;
    let mut bytes = [0; 8];
    let result = reader.read_exact(&mut bytes);
    reader.seek(SeekFrom::Start(position))?;
    match result {
        Ok(()) => Ok(&bytes == b"IMOAVI\0\0"),
        Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => Ok(false),
        Err(error) => Err(error.into()),
    }
}
fn header(reader: &mut (impl Read + Seek)) -> Result<Header> {
    let start = reader.stream_position()?;
    let file_end = reader.seek(SeekFrom::End(0))?;
    ensure!(
        file_end.saturating_sub(start) <= crate::MAX_VIDEO_BYTES as u64,
        "IMOAVI encoded byte budget"
    );
    reader.seek(SeekFrom::Start(start))?;
    tag(reader, b"IMOAVI\0\0")?;
    tag(reader, b"SOUND\0\0\0")?;
    let audio_size = word(reader)?;
    let _native_sound_word = word(reader)?;
    let after_audio = reader
        .stream_position()?
        .checked_add(u64::from(audio_size))
        .context("IMOAVI sound extent overflow")?;
    ensure!(after_audio <= file_end, "IMOAVI sound outside container");
    reader.seek(SeekFrom::Start(after_audio))?;
    tag(reader, b"MOVIE\0\0\0")?;
    let duration_ms = u64::from(word(reader)?);
    let frames = word(reader)?;
    let width = word(reader)?;
    let height = word(reader)?;
    let region = u64::from(word(reader)?);
    let base = reader.stream_position()?;
    ensure!(
        duration_ms > 0 && (1..=MAX_FRAMES).contains(&frames),
        "IMOAVI frame/time budget"
    );
    crysirror_image::rgba_len(width, height)?;
    let frame_ms = duration_ms as f64 / f64::from(frames);
    ensure!(frame_ms > 0. && frame_ms <= 10000., "IMOAVI frame duration");
    let end = base
        .checked_add(region)
        .context("IMOAVI frame region overflow")?;
    ensure!(
        region >= FRAME_HEADER && end.checked_add(16).is_some_and(|v| v <= file_end),
        "IMOAVI frame region outside container"
    );
    Ok(Header {
        info: Info {
            width,
            height,
            frames,
            duration_ms,
            frame_ms,
            has_audio: audio_size != 0,
        },
        base,
        region,
        file_end,
    })
}
/// Header facts only; no packets or JPEGs are decoded. Restore the cursor.
pub fn info(reader: &mut (impl Read + Seek)) -> Result<Info> {
    let position = reader.stream_position()?;
    let result = header(reader).map(|header| header.info);
    reader.seek(SeekFrom::Start(position))?;
    result
}

pub struct Movie<R> {
    reader: R,
    header: Header,
    offsets: Vec<(u32, u64)>,
    cursor: Option<(u32, u64)>,
    previous: Option<(u32, ImageSurface)>,
    encoded_bytes: u64,
    video_info: crate::VideoInfo,
    next: u32,
}
impl<R: Read + Seek> Movie<R> {
    pub fn new(reader: R) -> Result<Self> {
        Self::open(reader, || false)
    }
    pub fn open(mut reader: R, mut cancelled: impl FnMut() -> bool) -> Result<Self> {
        let header = header(&mut reader)?;
        reader.seek(SeekFrom::Start(header.base + header.region))?;
        tag(&mut reader, b"FRMIDX\0\0")?;
        let count = word(&mut reader)?;
        ensure!(
            count > 0 && count <= header.info.frames && count <= MAX_FRAMES,
            "IMOAVI sparse index count"
        );
        ensure!(
            reader
                .stream_position()?
                .checked_add(u64::from(count) * 8)
                .is_some_and(|end| end <= header.file_end),
            "IMOAVI sparse index outside container"
        );
        let mut offsets = Vec::new();
        offsets.try_reserve_exact(count as usize)?;
        let mut last = None;
        for i in 0..count {
            if i % 256 == 0 {
                ensure!(!cancelled(), "IMOAVI open cancelled");
            }
            let index = word(&mut reader)?;
            let offset = u64::from(word(&mut reader)?);
            ensure!(
                index < header.info.frames
                    && last.is_none_or(|v| index > v)
                    && offset
                        .checked_add(FRAME_HEADER)
                        .is_some_and(|end| end <= header.region),
                "IMOAVI sparse frame bounds/order"
            );
            if i == 0 {
                ensure!(index == 0 && offset == 0, "IMOAVI first sparse frame");
            }
            offsets.push((index, offset));
            last = Some(index);
        }
        Ok(Self {
            reader,
            header,
            offsets,
            cursor: None,
            previous: None,
            encoded_bytes: 0,
            video_info: crate::VideoInfo { container: crate::Container::ImoAvi,
                width: header.info.width, height: header.info.height,
                fps: Some(1000. / header.info.frame_ms) },
            next: 0,
        })
    }
    pub fn info(&self) -> Info {
        self.header.info
    }
    fn entry(&mut self, offset: u64) -> Result<Entry> {
        ensure!(
            offset
                .checked_add(FRAME_HEADER)
                .is_some_and(|end| end <= self.header.region),
            "IMOAVI frame header outside region"
        );
        self.reader
            .seek(SeekFrom::Start(self.header.base + offset))?;
        let next = word(&mut self.reader)?;
        let _signed_previous = word(&mut self.reader)? as i32;
        let content = word(&mut self.reader)?;
        let _native_unused = word(&mut self.reader)?;
        let redirect = word(&mut self.reader)? as i32;
        let size = word(&mut self.reader)?;
        let codec = word(&mut self.reader)?;
        ensure!(
            size <= MAX_PAYLOAD && offset + FRAME_HEADER + u64::from(size) <= self.header.region,
            "IMOAVI packet extent/budget"
        );
        if next != 0 {
            ensure!(
                u64::from(next) >= FRAME_HEADER + u64::from(size)
                    && offset
                        .checked_add(u64::from(next))
                        .is_some_and(|end| end <= self.header.region),
                "IMOAVI next-frame stride"
            );
        }
        Ok(Entry {
            next,
            content,
            redirect,
            size,
            codec,
        })
    }
    /// Decode a frame using its native sparse index. Redirects can point in
    /// either direction; cycles and out-of-region seeks fail before reading.
    /// None means end/cancellation, never successful decoding of unknown data.
    pub fn decode_frame(
        &mut self,
        index: u32,
        mut cancelled: impl FnMut() -> bool,
    ) -> Result<Option<VideoFrame>> {
        if index >= self.header.info.frames || cancelled() {
            return Ok(None);
        }
        let at = self.offsets.partition_point(|&(frame, _)| frame <= index);
        let (mut current, mut offset) = *self
            .offsets
            .get(at.checked_sub(1).context("IMOAVI sparse predecessor")?)
            .context("IMOAVI sparse predecessor")?;
        // Retain only one sequential cursor. Never grow a second resident
        // index merely because a long movie was played once.
        if let Some((frame, position)) = self
            .cursor
            .filter(|&(frame, _)| frame <= index && frame >= current)
        {
            current = frame;
            offset = position;
        }
        while current < index {
            if cancelled() {
                return Ok(None);
            }
            let entry = self.entry(offset)?;
            ensure!(
                entry.next > 0,
                "IMOAVI chain ends before declared frame count"
            );
            offset = offset
                .checked_add(u64::from(entry.next))
                .context("IMOAVI frame stride overflow")?;
            current += 1;
        }
        self.cursor = Some((index, offset));
        let mut visited = BTreeSet::new();
        let entry = loop {
            if cancelled() {
                return Ok(None);
            }
            ensure!(
                visited.len() < MAX_REDIRECTS && visited.insert(offset),
                "IMOAVI redirect cycle/budget"
            );
            let entry = self.entry(offset)?;
            if entry.redirect == 0 {
                break entry;
            }
            offset = offset
                .checked_add_signed(i64::from(entry.redirect))
                .context("IMOAVI redirect outside region")?;
        };
        let image = if let Some((content, image)) = &self.previous {
            if *content == entry.content {
                Some(image.clone())
            } else {
                None
            }
        } else {
            None
        };
        let image = if let Some(image) = image {
            image
        } else {
            ensure!(entry.size > 0, "IMOAVI empty image payload");
            self.reader
                .seek(SeekFrom::Start(self.header.base + offset + FRAME_HEADER))?;
            let mut bytes = vec![0; entry.size as usize];
            self.reader
                .read_exact(&mut bytes)
                .context("IMOAVI truncated image payload")?;
            self.encoded_bytes = self
                .encoded_bytes
                .checked_add(u64::from(entry.size))
                .context("IMOAVI byte counter")?;
            if cancelled() {
                return Ok(None);
            }
            let Some(image) =
                decode_picture(entry.codec, &bytes, self.header.info, &mut cancelled)?
            else {
                return Ok(None);
            };
            image.validate()?;
            self.previous = Some((entry.content, image.clone()));
            image
        };
        Ok(Some(VideoFrame {
            pts_ms: self.header.info.pts_ms(index)? as i64,
            width: image.width,
            height: image.height,
            rgba: image.rgba,
        }))
    }
}

impl<R: Read + Seek + Send> crate::VideoStream for Movie<R> {
    fn info(&self) -> &crate::VideoInfo { &self.video_info }
    fn next_frame(&mut self) -> Result<Option<crate::Frame>> {
        let Some(frame) = self.decode_frame(self.next, ||false)? else {return Ok(None)};
        self.next += 1;
        Ok(Some(crate::Frame { pts_ms:frame.pts_ms as u64, width:frame.width,
            height:frame.height, rgba:frame.rgba }))
    }
    fn rewind(&mut self) -> Result<()> {
        self.next = 0; self.cursor = None; self.previous = None;
        Ok(())
    }
}

fn jpeg(bytes: &[u8], info: Info) -> Result<ImageSurface> {
    ensure!(
        bytes.starts_with(&[0xff, 0xd8]),
        "IMOAVI packet is not JPEG"
    );
    let image = crysirror_image::decode_standard(bytes)?;
    ensure!(
        [image.width, image.height] == [info.width, info.height],
        "IMOAVI JPEG dimensions differ from movie"
    );
    Ok(image)
}
fn decode_picture(
    codec: u32,
    bytes: &[u8],
    info: Info,
    cancelled: &mut impl FnMut() -> bool,
) -> Result<Option<ImageSurface>> {
    match codec {
        0 | 1 => {
            let image = jpeg(bytes, info)?;
            if codec == 1 {
                ensure!(
                    image
                        .rgba
                        .chunks_exact(4)
                        .all(|p| p[0] == p[1] && p[1] == p[2]),
                    "IMOAVI gray JPEG profile"
                );
            }
            if cancelled() {
                return Ok(None);
            }
            Ok(Some(image))
        }
        2 => {
            let mut cursor = std::io::Cursor::new(bytes);
            let rgb_len = word(&mut cursor)? as usize;
            let end = 4usize
                .checked_add(rgb_len)
                .context("IMOAVI RGB extent overflow")?;
            let rgb = bytes.get(4..end).context("IMOAVI truncated RGB JPEG")?;
            cursor.set_position(end as u64);
            let alpha_len = word(&mut cursor)? as usize;
            let alpha_start = end.checked_add(4).context("IMOAVI alpha extent overflow")?;
            let alpha_end = alpha_start
                .checked_add(alpha_len)
                .context("IMOAVI alpha extent overflow")?;
            let alpha = bytes
                .get(alpha_start..alpha_end)
                .context("IMOAVI truncated alpha JPEG")?;
            let mut image = jpeg(rgb, info)?;
            if cancelled() {
                return Ok(None);
            }
            let alpha = jpeg(alpha, info)?;
            if cancelled() {
                return Ok(None);
            }
            ensure!(
                alpha
                    .rgba
                    .chunks_exact(4)
                    .all(|p| p[0] == p[1] && p[1] == p[2]),
                "IMOAVI alpha JPEG must be grayscale"
            );
            for (pixel, alpha) in image
                .rgba
                .chunks_exact_mut(4)
                .zip(alpha.rgba.chunks_exact(4))
            {
                pixel[3] = alpha[0];
            }
            Ok(Some(image))
        }
        0x10..=0x13 => {
            let mut cursor = std::io::Cursor::new(bytes);
            ensure!(
                [word(&mut cursor)?, word(&mut cursor)?] == [info.width, info.height],
                "IMOAVI raw dimensions differ from movie"
            );
            let stride = match codec {
                0x10 => 3,
                0x11 => 4,
                _ => 2,
            };
            let len = crysirror_image::rgba_len(info.width, info.height)?;
            let pixels = bytes.get(8..).context("IMOAVI raw header")?;
            ensure!(pixels.len() == len / 4 * stride, "IMOAVI raw byte count");
            let mut rgba = Vec::new();
            rgba.try_reserve_exact(len)?;
            // Original MMX pair loops leave odd tails undefined. Decode all
            // bounded pixels deterministically, including an odd row width.
            for (index, pixel) in pixels.chunks_exact(stride).enumerate() {
                if index % 16384 == 0 && cancelled() {
                    return Ok(None);
                }
                let color = match codec {
                    0x10 => [pixel[2], pixel[1], pixel[0], 255],
                    0x11 => [pixel[2], pixel[1], pixel[0], pixel[3]],
                    0x12 => {
                        let p = u32::from(u16::from_le_bytes([pixel[0], pixel[1]]));
                        //4DD610 deliberately expands R/B to251, unlike the
                        //usual full five-bit replication to255.
                        let packed =
                            ((p & 31) << 3) | ((p & 0x7e0) << 5) | ((p & 0xf800) << 8) | 0xff000000;
                        let packed = packed | ((packed >> 6) & 0x03030303);
                        [(packed >> 16) as u8, (packed >> 8) as u8, packed as u8, 255]
                    }
                    _ => {
                        let p = u16::from_le_bytes([pixel[0], pixel[1]]);
                        [
                            ((p >> 8) & 15) as u8 * 17,
                            ((p >> 4) & 15) as u8 * 17,
                            (p & 15) as u8 * 17,
                            ((p >> 12) & 15) as u8 * 17,
                        ]
                    }
                };
                rgba.extend_from_slice(&color);
            }
            Ok(Some(ImageSurface {
                width: info.width,
                height: info.height,
                rgba,
            }))
        }
        _ => bail!("unsupported IMOAVI picture codec {codec:#x}"),
    }
}

pub fn decode_video_with_metadata<R: Read + Seek>(
    reader: R,
    mut cancelled: impl FnMut() -> bool,
    mut metadata: impl FnMut(crate::VideoMetadata) -> Result<()>,
    mut publish: impl FnMut(VideoFrame) -> Result<bool>,
) -> Result<DecodeStats> {
    let mut movie = Movie::open(reader, &mut cancelled)?;
    let info = movie.info();
    metadata(crate::VideoMetadata {
        size: [info.width, info.height],
        duration_ms: Some(info.duration_ms),
        frame_ms: Some(info.frame_ms),
    })?;
    let mut stats = DecodeStats::default();
    for index in 0..info.frames {
        if cancelled() {
            stats.cancelled = true;
            break;
        }
        let Some(frame) = movie.decode_frame(index, &mut cancelled)? else {
            stats.cancelled = true;
            break;
        };
        stats.encoded_bytes = movie.encoded_bytes;
        stats.outputs += 1;
        if !publish(frame)? {
            break;
        }
    }
    Ok(stats)
}
