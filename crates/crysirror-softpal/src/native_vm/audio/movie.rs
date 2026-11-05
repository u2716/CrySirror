//! WMV/WMA and MPEG program-stream soundtracks on the movie presentation clock.
//! Container demuxing comes from the siglus_rs imports; MPEG audio decoding uses
//! the same Symphonia registry already used by Kira. No host decoder is invoked.

use super::{is_asf_container, Frame, StaticSoundData, StaticSoundSettings};
use anyhow::{bail, ensure, Context};
use na_mpeg2_decoder::{Demuxer, StreamType};
use std::{io::Cursor, sync::Arc};
use symphonia::core::{
    audio::SampleBuffer,
    codecs::{CodecParameters, CodecType, CODEC_TYPE_MP1, CODEC_TYPE_MP2, CODEC_TYPE_MP3},
    formats::Packet,
};
use wmv_decoder::{asf::AsfFile, AsfWmaDecoder};

const MAX_PCM_BYTES: usize = 256 * 1024 * 1024;
const MAX_PCM_FRAMES: usize = MAX_PCM_BYTES / std::mem::size_of::<Frame>();

/// Decode the soundtrack aligned to movie time zero. `None` means a supported
/// container has no audio stream; unsupported codecs and corrupt streams fail.
/// Leading gaps become silence, and samples preceding video time zero are cut.
pub fn decode_movie_audio(bytes: &[u8]) -> anyhow::Result<Option<StaticSoundData>> {
    if is_asf_container(bytes) {
        decode_asf(bytes)
    } else if bytes.starts_with(&[0, 0, 1, 0xba]) {
        decode_mpeg(bytes)
    } else if bytes.starts_with(&[0, 0, 1, 0xb3]) {
        // A raw MPEG video elementary stream cannot carry an audio track.
        Ok(None)
    } else {
        bail!("unsupported movie audio container")
    }
}

/// Match the video duration so whole-movie audio looping does not restart early
/// when a soundtrack ends before the last picture. This also removes codec tails.
pub fn resize_movie_audio(data: &mut StaticSoundData, duration_ms: u64) -> anyhow::Result<()> {
    let length = samples_at_ms(data.sample_rate, duration_ms)?;
    ensure!(
        length <= MAX_PCM_FRAMES,
        "movie audio duration exceeds PCM limit"
    );
    let mut frames = data.frames.to_vec();
    frames.try_reserve(length.saturating_sub(frames.len()))?;
    frames.resize(length, Frame::from_mono(0.0));
    data.frames = frames.into();
    Ok(())
}

/// PAL movie loop points use milliseconds; sound-group loop points use samples.
/// An end of zero retains the existing end-of-audio convention.
pub fn movie_loop_samples(
    sample_rate: u32,
    start_ms: u64,
    end_ms: u64,
) -> anyhow::Result<(i64, i64)> {
    let start = i64::try_from(samples_at_ms(sample_rate, start_ms)?)?;
    let end = i64::try_from(samples_at_ms(sample_rate, end_ms)?)?;
    ensure!(end == 0 || end > start, "invalid movie audio loop range");
    Ok((start, end))
}

/// Build a movie loop while retaining a nonzero start when end means EOF.
pub fn movie_loop_region(
    sample_rate: u32,
    start_ms: u64,
    end_ms: u64,
) -> anyhow::Result<kira::sound::Region> {
    use kira::sound::{EndPosition, PlaybackPosition, Region};
    let (start, end) = movie_loop_samples(sample_rate, start_ms, end_ms)?;
    Ok(Region {
        start: PlaybackPosition::Samples(usize::try_from(start)?),
        end: if end == 0 {
            EndPosition::EndOfAudio
        } else {
            EndPosition::Custom(PlaybackPosition::Samples(usize::try_from(end)?))
        },
    })
}

fn samples_at_ms(rate: u32, ms: u64) -> anyhow::Result<usize> {
    ensure!(rate > 0, "movie audio sample rate is zero");
    Ok(usize::try_from(u128::from(rate) * u128::from(ms) / 1000)?)
}

struct Timeline {
    rate: u32,
    frames: Vec<Frame>,
    last_timestamp: Option<i64>,
}

impl Timeline {
    fn new(rate: u32) -> anyhow::Result<Self> {
        ensure!(rate > 0, "movie audio sample rate is zero");
        Ok(Self {
            rate,
            frames: Vec::new(),
            last_timestamp: None,
        })
    }

    fn append(
        &mut self,
        position: i64,
        samples: &[f32],
        channels: usize,
        jitter: i64,
        repeated_timestamp_is_tail: bool,
    ) -> anyhow::Result<()> {
        ensure!(
            matches!(channels, 1 | 2),
            "unsupported movie audio channel count {channels}"
        );
        ensure!(
            samples.len() % channels == 0,
            "partial movie audio PCM frame"
        );
        ensure!(
            samples.iter().all(|sample| sample.is_finite()),
            "non-finite movie audio sample"
        );
        let mut position = if repeated_timestamp_is_tail && self.last_timestamp == Some(position) {
            self.frames.len() as i64
        } else {
            position
        };
        self.last_timestamp = Some(position);
        // ASF stores only integer milliseconds: preserve continuous PCM across
        // rounding differences instead of dropping/inserting samples every frame.
        if (position - self.frames.len() as i64).abs() <= jitter {
            position = self.frames.len() as i64;
        }
        let skip = usize::try_from(position.saturating_neg().max(0))?.min(samples.len() / channels);
        let start = usize::try_from(position.max(0))?;
        let available = samples.len() / channels - skip;
        let end = start
            .checked_add(available)
            .context("movie PCM range overflow")?;
        ensure!(end <= MAX_PCM_FRAMES, "movie audio exceeds PCM limit");
        if end > self.frames.len() {
            self.frames.try_reserve(end - self.frames.len())?;
            self.frames.resize(end, Frame::from_mono(0.0));
        }
        for (output, input) in self.frames[start..end]
            .iter_mut()
            .zip(samples[skip * channels..].chunks_exact(channels))
        {
            *output = Frame::new(input[0], *input.get(1).unwrap_or(&input[0]));
        }
        Ok(())
    }

    fn finish(self) -> anyhow::Result<StaticSoundData> {
        ensure!(
            !self.frames.is_empty(),
            "movie audio stream produced no samples"
        );
        Ok(StaticSoundData {
            sample_rate: self.rate,
            frames: Arc::from(self.frames),
            settings: StaticSoundSettings::new(),
            slice: None,
        })
    }
}

fn decode_asf(bytes: &[u8]) -> anyhow::Result<Option<StaticSoundData>> {
    let mut object_offset = 0usize;
    while object_offset < bytes.len() {
        let object =
            wmv_decoder::asf::ObjectHeader::read(&mut Cursor::new(&bytes[object_offset..]))
                .context("movie ASF object header")?;
        object_offset = object_offset
            .checked_add(usize::try_from(object.size)?)
            .context("ASF object range overflow")?;
        ensure!(object_offset <= bytes.len(), "truncated movie ASF object");
    }
    let asf = AsfFile::open(&mut Cursor::new(bytes)).context("movie ASF header")?;
    // The shared ASF reader treats a short final packet as EOF. Validate the
    // declared packet range here so truncation cannot become successful silence.
    let data_end = if asf.packet_count > 0 {
        asf.data_offset
            .checked_add(
                asf.packet_count
                    .checked_mul(u64::from(asf.packet_size))
                    .context("ASF packet count overflow")?,
            )
            .context("ASF data range overflow")?
    } else {
        bytes.len() as u64
    };
    ensure!(
        data_end <= bytes.len() as u64,
        "truncated movie ASF packets"
    );
    if asf.audio_streams.is_empty() {
        return Ok(None);
    }
    ensure!(
        asf.audio_streams
            .iter()
            .any(|stream| matches!(stream.format_tag, 0x160..=0x162)),
        "unsupported movie ASF audio codec"
    );
    let mut decoder = AsfWmaDecoder::open(Cursor::new(&bytes[..data_end as usize]))
        .context("movie WMA decoder")?;
    let mut timeline = Timeline::new(decoder.sample_rate())?;
    while let Some(decoded) = decoder.next_frame().context("movie WMA frame")? {
        ensure!(
            decoded.frame.sample_rate == timeline.rate,
            "movie WMA sample rate changed"
        );
        let position = i64::try_from(samples_at_ms(timeline.rate, u64::from(decoded.pts_ms))?)?;
        timeline.append(
            position,
            &decoded.frame.samples,
            usize::from(decoded.frame.channels),
            i64::from(timeline.rate / 1000 + 1),
            true,
        )?;
    }
    Ok(Some(timeline.finish()?))
}

fn decode_mpeg(bytes: &[u8]) -> anyhow::Result<Option<StaticSoundData>> {
    validate_program_stream(bytes)?;
    let mut demux = Demuxer::new_auto();
    let mut packets = Vec::new();
    let mut audio = Vec::new();
    let mut anchors = Vec::new();
    let mut video_origin = None;
    for chunk in bytes.chunks(16 * 1024) {
        packets.clear();
        demux.push_into(chunk, None, &mut packets);
        for packet in packets.drain(..) {
            match packet.stream_type {
                StreamType::MpegVideo => {
                    video_origin = video_origin.or(packet.pts_90k);
                }
                StreamType::MpegAudio => {
                    if let Some(pts) = packet.pts_90k {
                        anchors.push((audio.len(), pts));
                    }
                    audio.extend_from_slice(&packet.data);
                }
                StreamType::DvdLpcmAudio => bail!("unsupported MPEG private-stream audio codec"),
                StreamType::Unknown => {}
            }
        }
    }
    if audio.is_empty() {
        return Ok(None);
    }
    let origin = video_origin
        .or_else(|| anchors.first().map(|anchor| anchor.1))
        .unwrap_or(0);
    let first = MpegAudioHeader::parse(&audio)?;
    let mut parameters = CodecParameters::new();
    parameters
        .for_codec(first.codec)
        .with_sample_rate(first.rate);
    let mut decoder = symphonia::default::get_codecs()
        .make(&parameters, &Default::default())
        .context("MPEG movie audio decoder")?;
    let mut timeline = Timeline::new(first.rate)?;
    let (mut offset, mut anchor_index, mut timestamp, mut decoded_samples) = (0, 0, 0i64, 0u64);
    while offset < audio.len() {
        let header = MpegAudioHeader::parse(&audio[offset..])?;
        ensure!(
            header.rate == first.rate && header.codec == first.codec,
            "MPEG movie audio format changed"
        );
        let end = offset
            .checked_add(header.length)
            .context("MPEG audio frame overflow")?;
        ensure!(end <= audio.len(), "truncated MPEG movie audio frame");
        while anchor_index < anchors.len() && anchors[anchor_index].0 <= offset {
            let ticks = signed_pts_delta(anchors[anchor_index].1, origin);
            timestamp = (i128::from(ticks) * i128::from(first.rate) / 90_000) as i64;
            anchor_index += 1;
        }
        let packet = Packet::new_from_slice(
            0,
            decoded_samples,
            u64::from(header.samples),
            &audio[offset..end],
        );
        let buffer = decoder
            .decode(&packet)
            .context("MPEG movie audio frame decode")?;
        ensure!(
            buffer.spec().rate == timeline.rate,
            "MPEG movie PCM sample rate changed"
        );
        let channels = buffer.spec().channels.count();
        let mut pcm = SampleBuffer::<f32>::new(buffer.capacity() as u64, *buffer.spec());
        pcm.copy_interleaved_ref(buffer);
        timeline.append(timestamp, pcm.samples(), channels, 1, false)?;
        let frames = pcm.samples().len() / channels;
        timestamp += frames as i64;
        decoded_samples += frames as u64;
        offset = end;
    }
    Ok(Some(timeline.finish()?))
}

fn signed_pts_delta(value: i64, origin: i64) -> i64 {
    const WRAP: i64 = 1 << 33;
    (value - origin + WRAP / 2).rem_euclid(WRAP) - WRAP / 2
}

/// Validate PS packet lengths before using the imported incremental demuxer,
/// which otherwise retains incomplete PES data without reporting an error.
fn validate_program_stream(bytes: &[u8]) -> anyhow::Result<()> {
    let mut offset = 0usize;
    let mut audio_id = None;
    while offset < bytes.len() {
        ensure!(
            bytes.get(offset..offset + 3) == Some(&[0, 0, 1]),
            "invalid MPEG program stream boundary"
        );
        let id = *bytes.get(offset + 3).context("truncated MPEG start code")?;
        if (0xc0..=0xdf).contains(&id) {
            ensure!(
                audio_id.is_none_or(|previous| previous == id),
                "multiple MPEG movie audio streams are unsupported"
            );
            audio_id = Some(id);
        }
        let length = if id == 0xba {
            let marker = *bytes.get(offset + 4).context("truncated MPEG pack")?;
            if marker & 0xc0 == 0x40 {
                14 + usize::from(*bytes.get(offset + 13).context("truncated MPEG-2 pack")? & 7)
            } else {
                12
            }
        } else if id == 0xb9 {
            4
        } else {
            let length = bytes
                .get(offset + 4..offset + 6)
                .context("truncated MPEG packet length")?;
            let length = usize::from(u16::from_be_bytes([length[0], length[1]]));
            if length == 0 {
                ensure!(
                    (0xe0..=0xef).contains(&id),
                    "unbounded non-video MPEG packet"
                );
                bytes[offset + 6..]
                    .windows(4)
                    .position(|window| window[..3] == [0, 0, 1] && window[3] >= 0xb9)
                    .map_or(bytes.len() - offset, |next| next + 6)
            } else {
                6 + length
            }
        };
        offset = offset
            .checked_add(length)
            .context("MPEG packet length overflow")?;
        ensure!(
            offset <= bytes.len(),
            "truncated MPEG program-stream packet"
        );
    }
    Ok(())
}

struct MpegAudioHeader {
    codec: CodecType,
    rate: u32,
    length: usize,
    samples: u32,
}
impl MpegAudioHeader {
    fn parse(bytes: &[u8]) -> anyhow::Result<Self> {
        let header = u32::from_be_bytes(
            bytes
                .get(..4)
                .context("truncated MPEG audio header")?
                .try_into()?,
        );
        ensure!(header >> 21 == 0x7ff, "invalid MPEG audio sync");
        let version = (header >> 19) & 3;
        let layer = (header >> 17) & 3;
        let bitrate = ((header >> 12) & 15) as usize;
        let frequency = ((header >> 10) & 3) as usize;
        ensure!(
            version != 1 && layer != 0 && frequency < 3 && (1..15).contains(&bitrate),
            "unsupported MPEG audio header"
        );
        let rates = match (version == 3, layer) {
            (true, 3) => [
                0, 32, 64, 96, 128, 160, 192, 224, 256, 288, 320, 352, 384, 416, 448,
            ],
            (true, 2) => [
                0, 32, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 384,
            ],
            (true, _) => [
                0, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320,
            ],
            (false, 3) => [
                0, 32, 48, 56, 64, 80, 96, 112, 128, 144, 160, 176, 192, 224, 256,
            ],
            (false, _) => [0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160],
        };
        let rate = [44100, 48000, 32000][frequency]
            / match version {
                3 => 1,
                2 => 2,
                _ => 4,
            };
        let (codec, samples, factor, slot) = match layer {
            3 => (CODEC_TYPE_MP1, 384, 12, 4),
            2 => (CODEC_TYPE_MP2, 1152, 144, 1),
            _ if version == 3 => (CODEC_TYPE_MP3, 1152, 144, 1),
            _ => (CODEC_TYPE_MP3, 576, 72, 1),
        };
        let length =
            ((factor * rates[bitrate] * 1000 / rate + ((header >> 9) & 1)) * slot) as usize;
        Ok(Self {
            codec,
            rate,
            length,
            samples,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const MPEG: &[u8] = include_bytes!("../../tests/fixtures/movie_audio_delayed.mpg");
    const WMV: &[u8] = include_bytes!("../../tests/fixtures/movie_audio_delayed.wmv");

    fn assert_delayed_tone(bytes: &[u8]) {
        let audio = decode_movie_audio(bytes)
            .unwrap()
            .expect("movie soundtrack");
        assert_eq!(audio.sample_rate, 44100);
        let first = audio
            .frames
            .iter()
            .position(|frame| frame.left.abs() > 0.005)
            .unwrap();
        assert!(
            (2205..11025).contains(&first),
            "tone begins at sample {first}"
        );
        assert!(audio.frames[..2000]
            .iter()
            .all(|frame| frame.left == 0.0 && frame.right == 0.0));
        assert!(audio.frames.iter().any(|frame| frame.left.abs() > 0.05));
        assert!(audio.frames.len() > 18000 && audio.frames.len() < 44100);
        assert!(audio.frames.iter().all(|frame| frame.left == frame.right));
    }

    #[test]
    fn mpeg_mp2_soundtrack_preserves_delay_relative_to_video_pts() {
        assert_delayed_tone(MPEG);
    }

    #[test]
    fn wmv_wma_soundtrack_preserves_asf_presentation_delay() {
        assert_delayed_tone(WMV);
    }

    #[test]
    fn no_soundtrack_is_distinct_from_truncated_or_unsupported_input() {
        for bytes in [
            include_bytes!("../../tests/fixtures/timing_25fps.mpg").as_slice(),
            include_bytes!("../../tests/fixtures/timing_25fps.wmv").as_slice(),
        ] {
            assert!(decode_movie_audio(bytes).unwrap().is_none());
        }
        assert!(decode_movie_audio(&MPEG[..MPEG.len() - 1]).is_err());
        assert!(decode_movie_audio(&WMV[..WMV.len() - 1]).is_err());
        assert!(decode_movie_audio(b"not a movie").is_err());
        let mut corrupt = MPEG.to_vec();
        let frame = corrupt
            .windows(2)
            .position(|bytes| bytes == [0xff, 0xfd])
            .unwrap();
        corrupt[frame] = 0;
        assert!(decode_movie_audio(&corrupt).is_err());
    }

    #[test]
    fn timeline_trims_preroll_and_preserves_gaps_without_millisecond_drift() {
        let mut timeline = Timeline::new(44100).unwrap();
        timeline
            .append(-2, &[1.0, 2.0, 3.0, 4.0], 1, 0, false)
            .unwrap();
        timeline.append(4, &[5.0], 1, 0, false).unwrap();
        assert_eq!(
            timeline
                .frames
                .iter()
                .map(|frame| frame.left)
                .collect::<Vec<_>>(),
            [3.0, 4.0, 0.0, 0.0, 5.0]
        );
        let mut timeline = Timeline::new(44100).unwrap();
        let pcm = [0.25; 2048];
        for index in 0..100 {
            let milliseconds = index * 2048 * 1000 / 44100;
            timeline
                .append((milliseconds * 44100 / 1000) as i64, &pcm, 1, 45, false)
                .unwrap();
        }
        assert_eq!(timeline.frames.len(), 100 * 2048);
        assert_eq!(signed_pts_delta(9, (1 << 33) - 9), 18);
        assert_eq!(signed_pts_delta((1 << 33) - 9, 9), -18);
    }

    #[test]
    fn movie_loop_points_convert_milliseconds_and_video_duration_pads_silence() {
        let mut audio = decode_movie_audio(MPEG).unwrap().unwrap();
        let length = audio.frames.len();
        resize_movie_audio(&mut audio, 1000).unwrap();
        assert_eq!(audio.frames.len(), 44100);
        assert!(audio.frames[length..].iter().all(|frame| frame.left == 0.0));
        assert_eq!(movie_loop_samples(44100, 125, 375).unwrap(), (5512, 16537));
        assert_eq!(movie_loop_samples(44100, 0, 0).unwrap(), (0, 0));
        let region = movie_loop_region(44100, 125, 0).unwrap();
        assert_eq!(region.start, kira::sound::PlaybackPosition::Samples(5512));
        assert_eq!(region.end, kira::sound::EndPosition::EndOfAudio);
        assert!(movie_loop_samples(44100, 200, 100).is_err());
        assert!(movie_loop_samples(0, 0, 0).is_err());
        resize_movie_audio(&mut audio, 100).unwrap();
        assert_eq!(audio.frames.len(), 4410);
        assert!(resize_movie_audio(&mut audio, u64::MAX).is_err());
    }

    #[cfg(feature = "host-audio")]
    #[test]
    fn movie_handle_plays_loops_seeks_and_stops_independently_of_pal_groups() {
        use super::super::{apply_loop_region, AudioConfig, AudioSystem, PalSoundGroup, PalVolume};
        use kira::{sound::PlaybackState, Tween};
        let mut audio = AudioSystem::new(AudioConfig { enabled: true }).unwrap();
        audio.set_primary_volume(PalVolume::MIN).unwrap();
        for group in PalSoundGroup::ALL {
            audio.set_group_volume(group, PalVolume::MIN).unwrap();
        }
        let mut soundtrack = decode_movie_audio(MPEG).unwrap().unwrap();
        resize_movie_audio(&mut soundtrack, 600).unwrap();
        let (start, end) = movie_loop_samples(44100, 200, 400).unwrap();
        let mut handle = audio
            .play_movie_sound(apply_loop_region(soundtrack, true, start, end))
            .unwrap();
        let sink = audio.host_sink();
        let mut pcm = vec![0; 48000 * 2];
        sink.render_i16(&mut pcm);
        assert!(pcm[..2000].iter().all(|sample| *sample == 0));
        assert!(pcm.iter().any(|sample| sample.abs() > 1000));
        assert_eq!(handle.state(), PlaybackState::Playing);
        // Kira publishes position at the next processing callback boundary.
        sink.render_i16(&mut []);
        assert!((0.19..=0.41).contains(&handle.position()));
        handle.seek_to(0.25);
        sink.render_i16(&mut pcm[..2048]);
        sink.render_i16(&mut []);
        assert!((0.25..0.29).contains(&handle.position()));
        handle.stop(Tween::default());
        sink.render_i16(&mut pcm[..4096]);
        sink.render_i16(&mut pcm[..4096]);
        assert!(pcm[..4096].iter().all(|sample| *sample == 0));
        assert_eq!(handle.state(), PlaybackState::Stopped);
    }
}
