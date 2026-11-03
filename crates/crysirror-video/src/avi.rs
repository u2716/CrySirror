// SPDX-License-Identifier: MPL-2.0
//! Video-for-Windows RIFF/AVI with Cinepak video and optional PCM audio.
//!
// Adopted from siglus_rs 256c398 crates/avg32/src/movie.rs (MPL-2.0).
//! The demuxer walks one linear play-through of a `movi` list (no seeking,
//! no OpenDML/index chunks; `rec ` lists are flattened) because the titles
//! that ship these files always play start to finish. Frames are decoded
//! with the pure-Rust `oxideav-cinepak` decoder into RGBA; audio is returned
//! as interleaved little-endian PCM for the engine mixer. Nothing here owns
//! a clock, a VM buffer or a device.
use anyhow::{Context, Result, bail, ensure};
use oxideav_cinepak::{CinepakDecoder, CinepakFrame, CinepakPixelFormat};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StreamKind {
    Video,
    Audio,
    Other,
}

/// `(sample_rate, channels, bits_per_sample)` for a recognised PCM `auds` stream.
type AudioFormat = Option<(u32, u16, u16)>;
/// One `strl`: kind, PCM audio format, and the video compression FourCC.
type StreamInfo = (StreamKind, AudioFormat, Option<[u8; 4]>);

/// A demuxed AVI: raw per-frame video chunks plus, if present, the PCM audio
/// track (WAVE_FORMAT_PCM only; compressed audio stays silent rather than
/// being misdecoded).
#[derive(Debug, Clone, Default)]
pub struct AviMovie {
    /// Frame duration in microseconds (`avih.dwMicroSecPerFrame`).
    pub micro_sec_per_frame: u32,
    pub video_frames: Vec<Vec<u8>>,
    pub audio: Option<AviAudio>,
    /// BITMAPINFOHEADER compression of the video stream, when present.
    pub video_codec: Option<[u8; 4]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AviAudio {
    pub sample_rate: u32,
    pub channels: u16,
    pub bits_per_sample: u16,
    pub pcm: Vec<u8>,
}

/// True for a RIFF/AVI file.
pub fn is_avi(bytes: &[u8]) -> bool {
    bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"AVI "
}

impl AviMovie {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        if !is_avi(bytes) {
            bail!("not a RIFF/AVI file");
        }
        let mut micro_sec_per_frame = 66_667u32; // ~15fps fallback.
        let mut streams: Vec<StreamInfo> = Vec::new();
        let mut video_frames = Vec::new();
        let mut audio_pcm = Vec::new();

        let mut pos = 12usize;
        while let Some((id, body, end)) = next_chunk(bytes, pos) {
            if id == b"LIST" && bytes.get(body..body + 4) == Some(b"hdrl") {
                let (rate, found) = parse_hdrl(&bytes[body + 4..end])?;
                micro_sec_per_frame = rate;
                streams = found;
            } else if id == b"LIST" && bytes.get(body..body + 4) == Some(b"movi") {
                parse_movi(
                    &bytes[body + 4..end],
                    &streams,
                    &mut video_frames,
                    &mut audio_pcm,
                )?;
            }
            pos = end + (end - body) % 2;
        }

        let audio = streams
            .iter()
            .find_map(|(kind, format, _)| {
                matches!(kind, StreamKind::Audio)
                    .then_some(*format)
                    .flatten()
            })
            .filter(|_| !audio_pcm.is_empty())
            .map(|(sample_rate, channels, bits_per_sample)| AviAudio {
                sample_rate,
                channels,
                bits_per_sample,
                pcm: audio_pcm,
            });
        let video_codec = streams
            .iter()
            .find(|(kind, _, _)| *kind == StreamKind::Video)
            .and_then(|(_, _, codec)| *codec);

        Ok(Self {
            micro_sec_per_frame: micro_sec_per_frame.max(1),
            video_frames,
            audio,
            video_codec,
        })
    }

    /// Frame duration in milliseconds.
    pub fn frame_ms(&self) -> f64 {
        f64::from(self.micro_sec_per_frame.max(1)) / 1000.
    }

    /// Fails for a video stream whose declared compression is not Cinepak.
    /// An undeclared codec is decoded as Cinepak, as the original players did.
    pub fn require_cinepak(&self) -> Result<()> {
        if let Some(codec) = self.video_codec {
            ensure!(
                codec.eq_ignore_ascii_case(b"cvid"),
                "AVI video codec {:?} is not Cinepak",
                String::from_utf8_lossy(&codec)
            );
        }
        Ok(())
    }
}

/// Reads one RIFF chunk header at `pos` and returns `(id, body_start, body_end)`.
fn next_chunk(bytes: &[u8], pos: usize) -> Option<(&[u8], usize, usize)> {
    if pos.checked_add(8)? > bytes.len() {
        return None;
    }
    let id = &bytes[pos..pos + 4];
    let size = u32::from_le_bytes(bytes[pos + 4..pos + 8].try_into().expect("four bytes")) as usize;
    let body = pos + 8;
    let end = body.saturating_add(size).min(bytes.len());
    Some((id, body, end))
}

fn parse_hdrl(data: &[u8]) -> Result<(u32, Vec<StreamInfo>)> {
    let mut micro_sec_per_frame = 66_667u32;
    let mut streams = Vec::new();
    let mut pos = 0usize;
    while let Some((id, body, end)) = next_chunk(data, pos) {
        if id == b"avih" && end - body >= 4 {
            micro_sec_per_frame =
                u32::from_le_bytes(data[body..body + 4].try_into().expect("four bytes"));
        } else if id == b"LIST" && data.get(body..body + 4) == Some(b"strl") {
            streams.push(parse_strl(&data[body + 4..end])?);
        }
        pos = end + (end - body) % 2;
    }
    Ok((micro_sec_per_frame, streams))
}

fn parse_strl(data: &[u8]) -> Result<StreamInfo> {
    let mut kind = StreamKind::Other;
    let mut audio_format = None;
    let mut codec = None;
    let mut pos = 0usize;
    while let Some((id, body, end)) = next_chunk(data, pos) {
        if id == b"strh" && end - body >= 4 {
            kind = match &data[body..body + 4] {
                b"vids" => StreamKind::Video,
                b"auds" => StreamKind::Audio,
                _ => StreamKind::Other,
            };
        } else if id == b"strf" && kind == StreamKind::Audio && end - body >= 16 {
            let format_tag =
                u16::from_le_bytes(data[body..body + 2].try_into().expect("two bytes"));
            let channels =
                u16::from_le_bytes(data[body + 2..body + 4].try_into().expect("two bytes"));
            let sample_rate =
                u32::from_le_bytes(data[body + 4..body + 8].try_into().expect("four bytes"));
            let bits_per_sample =
                u16::from_le_bytes(data[body + 14..body + 16].try_into().expect("two bytes"));
            if format_tag == 1 {
                audio_format = Some((sample_rate, channels, bits_per_sample));
            }
        } else if id == b"strf" && kind == StreamKind::Video && end - body >= 20 {
            let fourcc: [u8; 4] = data[body + 16..body + 20].try_into().expect("four bytes");
            codec = (fourcc != [0; 4]).then_some(fourcc);
        }
        pos = end + (end - body) % 2;
    }
    Ok((kind, audio_format, codec))
}

fn parse_movi(
    data: &[u8],
    streams: &[StreamInfo],
    video_frames: &mut Vec<Vec<u8>>,
    audio_pcm: &mut Vec<u8>,
) -> Result<()> {
    let mut pos = 0usize;
    while let Some((id, body, end)) = next_chunk(data, pos) {
        if id == b"LIST" && data.get(body..body + 4) == Some(b"rec ") {
            parse_movi(&data[body + 4..end], streams, video_frames, audio_pcm)?;
        } else if let Some(index) = stream_index(id) {
            match &id[2..4] {
                b"dc" | b"db" if matches!(streams.get(index), Some((StreamKind::Video, _, _))) => {
                    video_frames.push(data[body..end].to_vec());
                }
                b"wb" if matches!(streams.get(index), Some((StreamKind::Audio, _, _))) => {
                    audio_pcm.extend_from_slice(&data[body..end]);
                }
                _ => {}
            }
        }
        pos = end + (end - body) % 2;
    }
    Ok(())
}

/// AVI stream-data chunk IDs are `NNxx`: `NN` ASCII decimal stream index,
/// `xx` the payload kind (`dc`/`db` video, `wb` audio).
fn stream_index(id: &[u8]) -> Option<usize> {
    let tens = id[0].checked_sub(b'0').filter(|d| *d <= 9)?;
    let ones = id[1].checked_sub(b'0').filter(|d| *d <= 9)?;
    Some(usize::from(tens) * 10 + usize::from(ones))
}

/// Stateful Cinepak frame decoder producing straight opaque RGBA.
#[derive(Default)]
pub struct CinepakRgba {
    decoder: CinepakDecoder,
}

impl CinepakRgba {
    pub fn new() -> Self {
        Self::default()
    }

    /// Forget inter-frame state (codebooks and the previous picture).
    pub fn reset(&mut self) {
        self.decoder.reset();
    }

    /// Decode one chunk into `(width, height, rgba)`.
    pub fn decode(&mut self, chunk: &[u8]) -> Result<(u32, u32, Vec<u8>)> {
        let frame = self
            .decoder
            .decode_frame(chunk, None)
            .map_err(|err| anyhow::anyhow!("Cinepak frame: {err}"))?;
        crysirror_image::rgba_len(frame.width, frame.height)?;
        Ok((frame.width, frame.height, expand_to_rgba(&frame)?))
    }
}

fn expand_to_rgba(frame: &CinepakFrame) -> Result<Vec<u8>> {
    let pixels = frame.pixels();
    let stride = frame.stride();
    let (width, height) = (frame.width as usize, frame.height as usize);
    let bytes_per_pixel = match frame.pixel_format {
        CinepakPixelFormat::Rgb24 => 3,
        CinepakPixelFormat::Gray8 => 1,
    };
    ensure!(
        stride >= width * bytes_per_pixel
            && pixels.len() >= stride * height.saturating_sub(1) + width * bytes_per_pixel,
        "Cinepak plane size"
    );
    let mut rgba = Vec::with_capacity(width * height * 4);
    for y in 0..height {
        let row = &pixels[y * stride..y * stride + width * bytes_per_pixel];
        match frame.pixel_format {
            CinepakPixelFormat::Rgb24 => {
                for pixel in row.chunks_exact(3) {
                    rgba.extend_from_slice(&[pixel[0], pixel[1], pixel[2], 255]);
                }
            }
            CinepakPixelFormat::Gray8 => {
                for &luminance in row {
                    rgba.extend_from_slice(&[luminance, luminance, luminance, 255]);
                }
            }
        }
    }
    Ok(rgba)
}

/// [`crate::VideoStream`] over an AVI/Cinepak file.
pub(crate) struct AviStream {
    movie: AviMovie,
    decoder: CinepakRgba,
    info: crate::VideoInfo,
    next: usize,
    first: Option<crate::Frame>,
}

impl AviStream {
    pub(crate) fn new(bytes: Vec<u8>) -> Result<Self> {
        let movie = AviMovie::parse(&bytes)?;
        movie.require_cinepak()?;
        ensure!(!movie.video_frames.is_empty(), "AVI has no video frames");
        let mut decoder = CinepakRgba::new();
        let (width, height, rgba) = decoder
            .decode(&movie.video_frames[0])
            .context("AVI frame 0")?;
        let fps = 1000. / movie.frame_ms();
        Ok(Self {
            info: crate::VideoInfo {
                container: crate::Container::Avi,
                width,
                height,
                fps: Some(fps),
            },
            movie,
            decoder,
            next: 1,
            first: Some(crate::Frame {
                pts_ms: 0,
                width,
                height,
                rgba,
            }),
        })
    }
}

impl crate::VideoStream for AviStream {
    fn info(&self) -> &crate::VideoInfo {
        &self.info
    }
    fn next_frame(&mut self) -> Result<Option<crate::Frame>> {
        if let Some(first) = self.first.take() {
            return Ok(Some(first));
        }
        let Some(chunk) = self.movie.video_frames.get(self.next) else {
            return Ok(None);
        };
        let (width, height, rgba) = self
            .decoder
            .decode(chunk)
            .with_context(|| format!("AVI frame {}", self.next))?;
        let pts_ms = (self.next as f64 * self.movie.frame_ms()).round() as u64;
        self.next += 1;
        Ok(Some(crate::Frame {
            pts_ms,
            width,
            height,
            rgba,
        }))
    }
    fn rewind(&mut self) -> Result<()> {
        self.decoder.reset();
        self.next = 0;
        self.first = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(id: &[u8; 4], body: Vec<u8>) -> Vec<u8> {
        let mut out = Vec::with_capacity(8 + body.len() + 1);
        out.extend_from_slice(id);
        out.extend_from_slice(&(body.len() as u32).to_le_bytes());
        out.extend_from_slice(&body);
        if body.len() % 2 == 1 {
            out.push(0);
        }
        out
    }

    fn list(list_type: &[u8; 4], body: Vec<u8>) -> Vec<u8> {
        let mut payload = list_type.to_vec();
        payload.extend_from_slice(&body);
        chunk(b"LIST", payload)
    }

    fn strl(fcc_type: &[u8; 4], strf: Vec<u8>) -> Vec<u8> {
        list(
            b"strl",
            [chunk(b"strh", fcc_type.to_vec()), chunk(b"strf", strf)].concat(),
        )
    }

    fn synthetic_avi(video_strf: Vec<u8>) -> Vec<u8> {
        let audio_strf = {
            let mut bytes = vec![0u8; 16];
            bytes[0..2].copy_from_slice(&1u16.to_le_bytes()); // WAVE_FORMAT_PCM
            bytes[2..4].copy_from_slice(&2u16.to_le_bytes()); // stereo
            bytes[4..8].copy_from_slice(&22_050u32.to_le_bytes());
            bytes[14..16].copy_from_slice(&16u16.to_le_bytes());
            bytes
        };
        let hdrl = list(
            b"hdrl",
            [
                chunk(b"avih", 66_667u32.to_le_bytes().to_vec()),
                strl(b"vids", video_strf),
                strl(b"auds", audio_strf),
            ]
            .concat(),
        );
        let movi = list(
            b"movi",
            [
                chunk(b"00dc", vec![1, 2, 3, 4]),
                list(b"rec ", chunk(b"01wb", vec![5, 6, 7, 8])),
            ]
            .concat(),
        );
        let mut body = b"AVI ".to_vec();
        body.extend_from_slice(&hdrl);
        body.extend_from_slice(&movi);
        chunk(b"RIFF", body)
    }

    #[test]
    fn demuxes_synthetic_container() {
        let movie = AviMovie::parse(&synthetic_avi(vec![0; 4])).unwrap();
        assert_eq!(movie.micro_sec_per_frame, 66_667);
        assert_eq!(movie.video_frames, vec![vec![1, 2, 3, 4]]);
        assert_eq!(movie.video_codec, None);
        let audio = movie.audio.expect("synthetic container carries PCM audio");
        assert_eq!(audio.sample_rate, 22_050);
        assert_eq!(audio.channels, 2);
        assert_eq!(audio.bits_per_sample, 16);
        assert_eq!(audio.pcm, vec![5, 6, 7, 8]);
    }

    #[test]
    fn rejects_non_riff_and_non_cinepak() {
        assert!(AviMovie::parse(b"not a riff file").is_err());
        let mut strf = vec![0u8; 40];
        strf[16..20].copy_from_slice(b"MJPG");
        let movie = AviMovie::parse(&synthetic_avi(strf.clone())).unwrap();
        assert!(movie.require_cinepak().is_err());
        strf[16..20].copy_from_slice(b"cvid");
        let movie = AviMovie::parse(&synthetic_avi(strf)).unwrap();
        movie.require_cinepak().unwrap();
        assert!(crate::sniff(&synthetic_avi(vec![0; 4])).unwrap() == crate::Container::Avi);
    }

    #[test]
    fn decodes_real_cinepak_frames_through_the_stream() {
        let (w, h) = (16u32, 8u32);
        let rgb: Vec<u8> = (0..w * h).flat_map(|i| if i % w < 8 { [200, 30, 30] } else { [20, 40, 220] }).collect();
        let frame = oxideav_cinepak::encode_rgb24(&rgb, w, h, oxideav_cinepak::EncoderOptions::default()).unwrap();
        let mut strf = vec![0u8; 40];
        strf[16..20].copy_from_slice(b"cvid");
        let mut avi = synthetic_avi(strf);
        // Replace the placeholder video payload with two real frames.
        let movi_at = avi.windows(4).position(|w| w == b"movi").unwrap();
        avi.truncate(movi_at - 8);
        let movi = list(b"movi", [chunk(b"00dc", frame.clone()), chunk(b"00dc", frame)].concat());
        avi.extend_from_slice(&movi);
        let size = (avi.len() - 8) as u32;
        avi[4..8].copy_from_slice(&size.to_le_bytes());
        let mut stream = crate::open(avi).unwrap();
        assert_eq!(stream.info().container, crate::Container::Avi);
        assert_eq!((stream.info().width, stream.info().height), (w, h));
        let first = stream.next_frame().unwrap().unwrap();
        assert_eq!(first.rgba.len(), (w * h * 4) as usize);
        let left = &first.rgba[0..4];
        let right = &first.rgba[(12 * 4) as usize..(12 * 4 + 4) as usize];
        assert!(left[0] > 150 && left[2] < 90, "{left:?}");
        assert!(right[2] > 150 && right[0] < 90, "{right:?}");
        let second = stream.next_frame().unwrap().unwrap();
        assert_eq!(second.pts_ms, 67);
        assert!(stream.next_frame().unwrap().is_none());
        stream.rewind().unwrap();
        assert_eq!(stream.next_frame().unwrap().unwrap().rgba, first.rgba);
    }

    #[test]
    fn malformed_cinepak_chunk_fails_visibly() {
        assert!(CinepakRgba::new().decode(&[1, 2, 3, 4]).is_err());
    }
}
