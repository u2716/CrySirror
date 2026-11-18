// SPDX-License-Identifier: MPL-2.0
//! WebM (Matroska subset) demuxing over a caller-owned seekable reader, and
//! VP8 decoding through the pinned pure-Rust `vp8-decoder` (RFC 6386,
//! bit-exact against the WebM test vectors). One video track is read;
//! Segment/Cluster elements may have unknown sizes. Pictures are converted
//! from I420 (BT.601 studio range, as VP8 specifies) to RGBA8. Alpha
//! BlockAdditions are not decoded: pictures are opaque.
//! No VM state, filesystem path, clock or device is held here.
use crate::mpeg::{DecodeStats, VideoFrame};
use anyhow::{bail, ensure, Context, Result};
use std::io::{Read, Seek, SeekFrom};

const EBML: u32 = 0x1A45_DFA3;
const DOC_TYPE: u32 = 0x4282;
const SEGMENT: u32 = 0x1853_8067;
const INFO: u32 = 0x1549_A966;
const TIMECODE_SCALE: u32 = 0x2A_D7B1;
const DURATION: u32 = 0x4489;
const TRACKS: u32 = 0x1654_AE6B;
const TRACK_ENTRY: u32 = 0xAE;
const TRACK_NUMBER: u32 = 0xD7;
const TRACK_TYPE: u32 = 0x83;
const CODEC_ID: u32 = 0x86;
const DEFAULT_DURATION: u32 = 0x23_E383;
const VIDEO: u32 = 0xE0;
const PIXEL_WIDTH: u32 = 0xB0;
const PIXEL_HEIGHT: u32 = 0xBA;
const CLUSTER: u32 = 0x1F43_B675;
const CLUSTER_TIMECODE: u32 = 0xE7;
const SIMPLE_BLOCK: u32 = 0xA3;
const BLOCK_GROUP: u32 = 0xA0;
const BLOCK: u32 = 0xA1;
const BLOCK_ADDITIONS: u32 = 0x75A1;
const BLOCK_MORE: u32 = 0xA6;
const BLOCK_ADD_ID: u32 = 0xEE;
const BLOCK_ADDITIONAL: u32 = 0xA5;
const ALPHA_MODE: u32 = 0x53C0;
/// Level-1 elements that terminate an unknown-size cluster.
const LEVEL1: [u32; 8] = [CLUSTER, 0x1C53_BB6B, 0x1254_C367, INFO, TRACKS, 0x114D_9B74, 0x1941_A469, 0x1043_A770];
const MAX_BLOCK: u64 = 32 * 1024 * 1024;
const MAX_HEADER_ELEMENT: u64 = 1024 * 1024;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Info {
    pub width: u32,
    pub height: u32,
    pub codec: String,
    pub track: u64,
    pub timecode_scale_ns: u64,
    pub duration_ms: Option<u64>,
    pub frame_ms: Option<f64>,
    pub has_audio: bool,
    /// `AlphaMode=1`: BlockAdditional id 1 carries a VP8 alpha stream.
    pub alpha: bool,
}
fn read_id(r: &mut impl Read) -> Result<Option<u32>> {
    let mut b = [0u8; 1];
    if r.read(&mut b)? == 0 {
        return Ok(None);
    }
    let len = b[0].leading_zeros() as usize + 1;
    ensure!(len <= 4, "WebM element id width");
    let mut id = u32::from(b[0]);
    for _ in 1..len {
        r.read_exact(&mut b)?;
        id = id << 8 | u32::from(b[0]);
    }
    Ok(Some(id))
}
/// Element data size; `None` is the reserved unknown size.
fn read_size(r: &mut impl Read) -> Result<Option<u64>> {
    let mut b = [0u8; 1];
    r.read_exact(&mut b)?;
    let len = b[0].leading_zeros() as usize + 1;
    ensure!(len <= 8, "WebM element size width");
    let mut v = u64::from(b[0]) & (0xff >> len);
    let mut all = v == 0xff >> len;
    for _ in 1..len {
        r.read_exact(&mut b)?;
        all &= b[0] == 0xff;
        v = v << 8 | u64::from(b[0]);
    }
    Ok(if all { None } else { Some(v) })
}
fn read_bytes(r: &mut impl Read, size: u64, max: u64) -> Result<Vec<u8>> {
    ensure!(size <= max, "WebM element exceeds its byte budget");
    let mut v = vec![0; size as usize];
    r.read_exact(&mut v)?;
    Ok(v)
}
fn uint(bytes: &[u8]) -> Result<u64> {
    ensure!(bytes.len() <= 8, "WebM unsigned integer width");
    Ok(bytes.iter().fold(0, |v, b| v << 8 | u64::from(*b)))
}
fn float(bytes: &[u8]) -> Result<f64> {
    Ok(match bytes.len() {
        0 => 0.,
        4 => f64::from(f32::from_be_bytes(bytes.try_into()?)),
        8 => f64::from_be_bytes(bytes.try_into()?),
        _ => bail!("WebM float width"),
    })
}
/// Iterate the children of a sized element held in memory.
fn children(data: &[u8], mut f: impl FnMut(u32, &[u8]) -> Result<()>) -> Result<()> {
    let mut c = std::io::Cursor::new(data);
    while (c.position() as usize) < data.len() {
        let id = read_id(&mut c)?.context("WebM truncated child")?;
        let size = read_size(&mut c)?.context("WebM unknown-size header child")?;
        let start = c.position() as usize;
        let end = start.checked_add(usize::try_from(size)?).filter(|e| *e <= data.len()).context("WebM child extent")?;
        f(id, &data[start..end])?;
        c.set_position(end as u64);
    }
    Ok(())
}
pub fn is_input(reader: &mut (impl Read + Seek)) -> Result<bool> {
    let start = reader.stream_position()?;
    let mut head = Vec::new();
    let result = reader.by_ref().take(64).read_to_end(&mut head);
    reader.seek(SeekFrom::Start(start))?;
    result?;
    Ok(head.starts_with(&EBML.to_be_bytes()) && head.windows(4).any(|w| w == b"webm"))
}
/// Buffered reader with a locally tracked position: element headers are
/// read a byte at a time, which must not reach a decrypting asset reader.
struct Input<R> {
    inner: std::io::BufReader<R>,
    pos: u64,
}
impl<R: Read + Seek> Input<R> {
    fn stream_position(&mut self) -> std::io::Result<u64> {
        Ok(self.pos)
    }
    fn seek(&mut self, to: SeekFrom) -> std::io::Result<u64> {
        let target = match to {
            SeekFrom::Start(v) => v,
            SeekFrom::Current(d) => self.pos.checked_add_signed(d).ok_or_else(|| std::io::Error::other("WebM seek"))?,
            SeekFrom::End(_) => return Err(std::io::Error::other("WebM end-relative seek")),
        };
        self.inner.seek_relative(target as i64 - self.pos as i64)?;
        self.pos = target;
        Ok(target)
    }
}
impl<R: Read> Read for Input<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.inner.read(buf)?;
        self.pos += n as u64;
        Ok(n)
    }
}
/// Demuxer positioned at the first cluster after reading the header.
pub struct Demuxer<R> {
    reader: Input<R>,
    pub info: Info,
    segment_end: u64,
    cluster_end: Option<u64>,
    cluster_unknown: bool,
    cluster_time: i64,
}
impl<R: Read + Seek> Demuxer<R> {
    pub fn new(mut reader: R) -> Result<Self> {
        let len = reader.seek(SeekFrom::End(0))?;
        reader.seek(SeekFrom::Start(0))?;
        let mut reader = Input { inner: std::io::BufReader::with_capacity(256 * 1024, reader), pos: 0 };
        ensure!(read_id(&mut reader)? == Some(EBML), "WebM EBML header");
        let size = read_size(&mut reader)?.context("WebM EBML header size")?;
        let header = read_bytes(&mut reader, size, 4096)?;
        let mut doc = String::new();
        children(&header, |id, v| {
            if id == DOC_TYPE {
                doc = String::from_utf8_lossy(v).trim_end_matches('\0').to_owned();
            }
            Ok(())
        })?;
        ensure!(doc == "webm" || doc == "matroska", "WebM document type {doc:?}");
        ensure!(read_id(&mut reader)? == Some(SEGMENT), "WebM segment");
        let segment_end = match read_size(&mut reader)? {
            Some(s) => reader.stream_position()?.checked_add(s).context("WebM segment extent")?.min(len),
            None => len,
        };
        let mut info = Info { timecode_scale_ns: 1_000_000, ..Default::default() };
        let mut duration = None;
        let mut video = None;
        loop {
            let at = reader.stream_position()?;
            ensure!(at < segment_end, "WebM has no cluster");
            let id = read_id(&mut reader)?.context("WebM truncated segment")?;
            let size = read_size(&mut reader)?;
            match id {
                CLUSTER => {
                    reader.seek(SeekFrom::Start(at))?;
                    break;
                }
                INFO => {
                    let data = read_bytes(&mut reader, size.context("WebM info size")?, MAX_HEADER_ELEMENT)?;
                    children(&data, |id, v| {
                        match id {
                            TIMECODE_SCALE => info.timecode_scale_ns = uint(v)?,
                            DURATION => duration = Some(float(v)?),
                            _ => {}
                        }
                        Ok(())
                    })?;
                }
                TRACKS => {
                    let data = read_bytes(&mut reader, size.context("WebM tracks size")?, MAX_HEADER_ELEMENT)?;
                    children(&data, |id, entry| {
                        if id != TRACK_ENTRY {
                            return Ok(());
                        }
                        let (mut number, mut kind, mut codec, mut dur, mut w, mut h, mut alpha) = (0, 0, String::new(), None, 0, 0, false);
                        children(entry, |id, v| {
                            match id {
                                TRACK_NUMBER => number = uint(v)?,
                                TRACK_TYPE => kind = uint(v)?,
                                CODEC_ID => codec = String::from_utf8_lossy(v).trim_end_matches('\0').to_owned(),
                                DEFAULT_DURATION => dur = Some(uint(v)?),
                                VIDEO => children(v, |id, v| {
                                    match id {
                                        PIXEL_WIDTH => w = uint(v)?,
                                        PIXEL_HEIGHT => h = uint(v)?,
                                        ALPHA_MODE => alpha = uint(v)? == 1,
                                        _ => {}
                                    }
                                    Ok(())
                                })?,
                                _ => {}
                            }
                            Ok(())
                        })?;
                        match kind {
                            1 if video.is_none() => video = Some((number, codec, dur, w, h, alpha)),
                            2 => info.has_audio = true,
                            _ => {}
                        }
                        Ok(())
                    })?;
                }
                _ => match size {
                    Some(s) => {
                        reader.seek(SeekFrom::Current(i64::try_from(s)?))?;
                    }
                    None => bail!("WebM unknown-size element {id:#x} before clusters"),
                },
            }
        }
        let (track, codec, dur, w, h, alpha) = video.context("WebM has no video track")?;
        info.alpha = alpha;
        ensure!(codec == "V_VP8", "WebM video codec {codec:?} is not supported");
        ensure!((1..=8192).contains(&w) && (1..=8192).contains(&h), "WebM video dimensions");
        ensure!((1..=1_000_000_000).contains(&info.timecode_scale_ns), "WebM timecode scale");
        info.width = w as u32;
        info.height = h as u32;
        info.codec = codec;
        info.track = track;
        info.frame_ms = dur.map(|ns| ns as f64 / 1e6).filter(|ms| *ms > 0. && *ms <= 10_000.);
        info.duration_ms = duration
            .map(|d| d * info.timecode_scale_ns as f64 / 1e6)
            .filter(|d| d.is_finite() && *d >= 0. && *d < 1e12)
            .map(|d| d.round() as u64);
        Ok(Self { reader, info, segment_end, cluster_end: None, cluster_unknown: false, cluster_time: 0 })
    }
    /// Next video packet: (presentation ms, VP8 frame bytes).
    pub fn next_packet(&mut self) -> Result<Option<(i64, Vec<u8>)>> {
        Ok(self.next_packet_alpha()?.map(|(t, d, _)| (t, d)))
    }
    /// Next packet with its BlockAdditional alpha frame, when present.
    pub fn next_packet_alpha(&mut self) -> Result<Option<(i64, Vec<u8>, Option<Vec<u8>>)>> {
        loop {
            let at = self.reader.stream_position()?;
            if at >= self.segment_end {
                return Ok(None);
            }
            if let Some(end) = self.cluster_end {
                if at >= end {
                    self.cluster_end = None;
                }
            }
            let Some(id) = read_id(&mut self.reader)? else { return Ok(None) };
            let size = read_size(&mut self.reader)?;
            if self.cluster_end.is_none() || (self.cluster_unknown && LEVEL1.contains(&id)) {
                // Segment level.
                if id == CLUSTER {
                    let start = self.reader.stream_position()?;
                    self.cluster_unknown = size.is_none();
                    self.cluster_end = Some(size.map_or(self.segment_end, |s| (start + s).min(self.segment_end)));
                    self.cluster_time = 0;
                    continue;
                }
                self.cluster_end = None;
                match size {
                    Some(s) => {
                        self.reader.seek(SeekFrom::Current(i64::try_from(s)?))?;
                        continue;
                    }
                    None => bail!("WebM unknown-size level-1 element {id:#x}"),
                }
            }
            let size = size.context("WebM unknown-size cluster child")?;
            match id {
                CLUSTER_TIMECODE => {
                    self.cluster_time = i64::try_from(uint(&read_bytes(&mut self.reader, size, 8)?)?)?;
                }
                SIMPLE_BLOCK => {
                    let data = read_bytes(&mut self.reader, size, MAX_BLOCK)?;
                    if let Some((t, d)) = self.block(&data)? {
                        return Ok(Some((t, d, None)));
                    }
                }
                BLOCK_GROUP => {
                    let data = read_bytes(&mut self.reader, size, MAX_BLOCK)?;
                    let mut found = None;
                    let mut alpha = None;
                    children(&data, |id, v| {
                        if id == BLOCK && found.is_none() {
                            found = Some(v.to_vec());
                        }
                        if id == BLOCK_ADDITIONS {
                            children(v, |id, more| {
                                if id == BLOCK_MORE {
                                    let (mut add_id, mut payload) = (1, None);
                                    children(more, |id, v| {
                                        match id {
                                            BLOCK_ADD_ID => add_id = uint(v)?,
                                            BLOCK_ADDITIONAL => payload = Some(v.to_vec()),
                                            _ => {}
                                        }
                                        Ok(())
                                    })?;
                                    if add_id == 1 {
                                        alpha = payload;
                                    }
                                }
                                Ok(())
                            })?;
                        }
                        Ok(())
                    })?;
                    if let Some(block) = found {
                        if let Some((t, d)) = self.block(&block)? {
                            return Ok(Some((t, d, alpha)));
                        }
                    }
                }
                _ => {
                    self.reader.seek(SeekFrom::Current(i64::try_from(size)?))?;
                }
            }
        }
    }
    fn block(&self, data: &[u8]) -> Result<Option<(i64, Vec<u8>)>> {
        let mut c = std::io::Cursor::new(data);
        let track = read_size(&mut c)?.context("WebM block track")?;
        let at = c.position() as usize;
        ensure!(data.len() >= at + 3, "WebM truncated block");
        if track != self.info.track {
            return Ok(None);
        }
        let relative = i16::from_be_bytes([data[at], data[at + 1]]);
        let flags = data[at + 2];
        ensure!(flags & 0x06 == 0, "WebM laced video blocks are not supported");
        let tc = self.cluster_time.checked_add(i64::from(relative)).context("WebM timecode")?;
        let pts = (i128::from(tc) * i128::from(self.info.timecode_scale_ns) / 1_000_000) as i64;
        Ok(Some((pts, data[at + 3..].to_vec())))
    }
}
/// BT.601 studio-range I420 to RGBA8 (fixed point, 8-bit fraction).
pub fn i420_to_rgba(p: &vp8_decoder::Picture<'_>) -> Result<Vec<u8>> {
    let (w, h) = (p.width as usize, p.height as usize);
    let mut out = vec![0u8; crysirror_image::rgba_len(p.width, p.height)?];
    for y in 0..h {
        let yr = &p.y[y * p.y_stride..];
        let ur = &p.u[(y / 2) * p.uv_stride..];
        let vr = &p.v[(y / 2) * p.uv_stride..];
        let row = &mut out[y * w * 4..(y + 1) * w * 4];
        for x in 0..w {
            let c = (i32::from(yr[x]) - 16) * 298;
            let d = i32::from(ur[x / 2]) - 128;
            let e = i32::from(vr[x / 2]) - 128;
            let px = &mut row[x * 4..x * 4 + 4];
            px[0] = ((c + 409 * e + 128) >> 8).clamp(0, 255) as u8;
            px[1] = ((c - 100 * d - 208 * e + 128) >> 8).clamp(0, 255) as u8;
            px[2] = ((c + 516 * d + 128) >> 8).clamp(0, 255) as u8;
            px[3] = 255;
        }
    }
    Ok(out)
}
/// Decode the alpha VP8 frame and copy its luma into the alpha channel.
/// A block without an alpha frame keeps the previous alpha picture.
fn apply_alpha(decoder: &mut vp8_decoder::Decoder, data: Option<&[u8]>, rgba: &mut [u8], w: u32, h: u32) -> Result<()> {
    let Some(data) = data else { return Ok(()) };
    let picture = match decoder.decode(data) {
        Ok(Some(p)) => p,
        Ok(None) | Err(vp8_decoder::DecodeError::NeedKeyframe) => return Ok(()),
        Err(e) => bail!("VP8 alpha decode: {e:?}"),
    };
    ensure!(picture.width == w && picture.height == h, "WebM alpha plane size");
    for y in 0..h as usize {
        for x in 0..w as usize {
            rgba[(y * w as usize + x) * 4 + 3] = picture.y[y * picture.y_stride + x];
        }
    }
    Ok(())
}
pub fn info(reader: impl Read + Seek) -> Result<Info> {
    Ok(Demuxer::new(reader)?.info)
}
pub fn decode_video_with_metadata<R: Read + Seek>(
    reader: R,
    mut cancelled: impl FnMut() -> bool,
    mut metadata: impl FnMut(crate::VideoMetadata) -> Result<()>,
    mut publish: impl FnMut(VideoFrame) -> Result<bool>,
) -> Result<DecodeStats> {
    let mut stats = DecodeStats::default();
    if cancelled() {
        stats.cancelled = true;
        return Ok(stats);
    }
    let mut demux = Demuxer::new(reader)?;
    let info = demux.info.clone();
    metadata(crate::VideoMetadata { size: [info.width, info.height], duration_ms: info.duration_ms, frame_ms: info.frame_ms })?;
    let mut decoder = vp8_decoder::Decoder::new();
    let mut alpha_decoder = vp8_decoder::Decoder::new();
    while let Some((pts_ms, data, alpha)) = demux.next_packet_alpha()? {
        if cancelled() {
            stats.cancelled = true;
            break;
        }
        stats.encoded_bytes += data.len() as u64;
        let picture = match decoder.decode(&data) {
            Ok(Some(p)) => p,
            Ok(None) | Err(vp8_decoder::DecodeError::NeedKeyframe) => continue,
            Err(e) => bail!("VP8 decode at {pts_ms} ms: {e:?}"),
        };
        ensure!(picture.width == info.width && picture.height == info.height, "VP8 picture size changed within the track");
        let mut rgba = i420_to_rgba(&picture)?;
        if info.alpha {
            apply_alpha(&mut alpha_decoder, alpha.as_deref(), &mut rgba, info.width, info.height)?;
        }
        let frame = VideoFrame { pts_ms, width: info.width, height: info.height, rgba };
        stats.outputs += 1;
        if !publish(frame)? {
            stats.cancelled = true;
            break;
        }
    }
    Ok(stats)
}
/// Whole-file stream for [`crate::open`]: packets are demuxed and decoded
/// on demand from the in-memory bytes.
pub struct WebmStream {
    bytes: std::sync::Arc<[u8]>,
    demux: Demuxer<std::io::Cursor<std::sync::Arc<[u8]>>>,
    decoder: vp8_decoder::Decoder,
    alpha: vp8_decoder::Decoder,
    info: crate::VideoInfo,
}
impl WebmStream {
    pub fn new(bytes: Vec<u8>) -> Result<Self> {
        let bytes: std::sync::Arc<[u8]> = bytes.into();
        let demux = Demuxer::new(std::io::Cursor::new(bytes.clone()))?;
        let info = crate::VideoInfo {
            container: crate::Container::WebM,
            width: demux.info.width,
            height: demux.info.height,
            fps: demux.info.frame_ms.map(|ms| 1000. / ms),
        };
        Ok(Self { bytes, demux, decoder: vp8_decoder::Decoder::new(), alpha: vp8_decoder::Decoder::new(), info })
    }
}
impl crate::VideoStream for WebmStream {
    fn info(&self) -> &crate::VideoInfo {
        &self.info
    }
    fn next_frame(&mut self) -> Result<Option<crate::Frame>> {
        while let Some((pts, data, alpha)) = self.demux.next_packet_alpha()? {
            match self.decoder.decode(&data) {
                Ok(Some(p)) => {
                    let (w, h) = (p.width, p.height);
                    let mut rgba = i420_to_rgba(&p)?;
                    if self.demux.info.alpha {
                        apply_alpha(&mut self.alpha, alpha.as_deref(), &mut rgba, w, h)?;
                    }
                    return Ok(Some(crate::Frame { pts_ms: pts.max(0) as u64, width: w, height: h, rgba }));
                }
                Ok(None) | Err(vp8_decoder::DecodeError::NeedKeyframe) => {}
                Err(e) => bail!("VP8 decode at {pts} ms: {e:?}"),
            }
        }
        Ok(None)
    }
    fn rewind(&mut self) -> Result<()> {
        self.demux = Demuxer::new(std::io::Cursor::new(self.bytes.clone()))?;
        self.decoder.reset();
        self.alpha.reset();
        Ok(())
    }
}
