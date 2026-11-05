//! Source PCM metadata retained independently of the mixer's stereo frames.
use super::{StaticSoundData, is_asf_container};
use std::io::Cursor;
use symphonia::core::io::MediaSourceStream;
use wmv_decoder::AsfWmaDecoder;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PcmSourceKind {
    Wave,
    Ogg,
    Wma,
    Other,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PcmSourceFormat {
    pub kind: PcmSourceKind,
    pub format_tag: u16,
    pub sample_rate: u32,
    pub average_bytes_per_second: u32,
    pub channels: u16,
    pub bits_per_sample: u16,
    pub block_align: u16,
    pub extra_size: u16,
    pub decoded_bytes: u32,
    /// Native Ogg seeks divide byte offsets by four, even for a mono file.
    pub seek_bytes_per_frame: u16,
}

impl PcmSourceFormat {
    pub(super) fn frame_at_byte(self, offset: i32) -> anyhow::Result<usize> {
        anyhow::ensure!(offset >= 0, "negative native PCM byte offset");
        anyhow::ensure!(
            self.seek_bytes_per_frame != 0,
            "source PCM seek alignment is unavailable"
        );
        Ok(offset as usize / usize::from(self.seek_bytes_per_frame))
    }

    pub(super) fn native_pal_format(self) -> bool {
        matches!(self.kind, PcmSourceKind::Wave | PcmSourceKind::Ogg)
    }
}

pub(super) fn describe_source(
    bytes: &[u8],
    data: &StaticSoundData,
) -> anyhow::Result<PcmSourceFormat> {
    describe_decoded_source(bytes, data.sample_rate, data.frames.len())
}

pub(super) fn describe_decoded_source(
    bytes: &[u8], sample_rate: u32, frame_count: usize,
) -> anyhow::Result<PcmSourceFormat> {
    if is_asf_container(bytes) {
        let decoder = AsfWmaDecoder::open(Cursor::new(bytes.to_vec()))
            .map_err(|error| anyhow::anyhow!("WMA format metadata: {error}"))?;
        return decoded_format(PcmSourceKind::Wma, sample_rate, frame_count, decoder.channels(), 16);
    }
    if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WAVE") {
        return wave_format(bytes);
    }
    let stream = MediaSourceStream::new(Box::new(Cursor::new(bytes.to_vec())), Default::default());
    let probed = symphonia::default::get_probe().format(
        &Default::default(),
        stream,
        &Default::default(),
        &Default::default(),
    )?;
    let parameters = &probed
        .format
        .default_track()
        .ok_or_else(|| anyhow::anyhow!("audio source has no default track"))?
        .codec_params;
    let channels = parameters
        .channels
        .ok_or_else(|| anyhow::anyhow!("audio channel metadata unavailable"))?
        .count();
    let kind = if bytes.starts_with(b"OggS") {
        PcmSourceKind::Ogg
    } else {
        PcmSourceKind::Other
    };
    let bits = if kind == PcmSourceKind::Ogg {
        16
    } else {
        parameters.bits_per_sample.unwrap_or(16)
    };
    let mut format = decoded_format(kind, sample_rate, frame_count, u16::try_from(channels)?, u16::try_from(bits)?)?;
    if kind == PcmSourceKind::Ogg {
        format.seek_bytes_per_frame = 4;
        if let Some(frames) = parameters.n_frames {
            format.decoded_bytes = u32::try_from(
                frames
                    .checked_mul(u64::from(format.block_align))
                    .ok_or_else(|| anyhow::anyhow!("decoded PCM byte size overflow"))?,
            )?;
        }
    }
    Ok(format)
}

fn decoded_format(
    kind: PcmSourceKind,
    sample_rate: u32,
    frame_count: usize,
    channels: u16,
    bits: u16,
) -> anyhow::Result<PcmSourceFormat> {
    let align = channels
        .checked_mul(bits.div_ceil(8))
        .ok_or_else(|| anyhow::anyhow!("PCM block alignment overflow"))?;
    anyhow::ensure!(align != 0, "zero PCM block alignment");
    let length = frame_count
        .checked_mul(usize::from(align))
        .ok_or_else(|| anyhow::anyhow!("decoded PCM byte size overflow"))?;
    Ok(PcmSourceFormat {
        kind,
        format_tag: 1,
        sample_rate,
        average_bytes_per_second: sample_rate.checked_mul(u32::from(align))
            .ok_or_else(|| anyhow::anyhow!("PCM average byte rate overflow"))?,
        channels,
        bits_per_sample: bits,
        block_align: align,
        extra_size: 0,
        decoded_bytes: u32::try_from(length)?,
        seek_bytes_per_frame: align,
    })
}

fn wave_format(bytes: &[u8]) -> anyhow::Result<PcmSourceFormat> {
    let mut offset = 12usize;
    let mut layout = None;
    let mut data_bytes = None;
    while let Some(header) = bytes.get(offset..offset.saturating_add(8)) {
        let length = u32::from_le_bytes(header[4..8].try_into().unwrap()) as usize;
        let begin = offset
            .checked_add(8)
            .ok_or_else(|| anyhow::anyhow!("WAVE chunk overflow"))?;
        let end = begin
            .checked_add(length)
            .ok_or_else(|| anyhow::anyhow!("WAVE chunk overflow"))?;
        let chunk = bytes
            .get(begin..end)
            .ok_or_else(|| anyhow::anyhow!("truncated WAVE chunk"))?;
        if &header[..4] == b"fmt " && chunk.len() >= 16 {
            let word = |index| u16::from_le_bytes([chunk[index], chunk[index + 1]]);
            let dword = |index| u32::from_le_bytes(chunk[index..index + 4].try_into().unwrap());
            layout = Some((word(0), word(2), dword(4), dword(8), word(12), word(14),
                if chunk.len() >= 18 { word(16) } else { 0 }));
        } else if &header[..4] == b"data" {
            data_bytes = Some(u32::try_from(length)?);
        }
        offset = end
            .checked_add(length & 1)
            .ok_or_else(|| anyhow::anyhow!("WAVE chunk overflow"))?;
    }
    let (tag, channels, sample_rate, average_bytes_per_second, block_align, bits, extra_size) =
        layout.ok_or_else(|| anyhow::anyhow!("WAVE fmt chunk absent"))?;
    anyhow::ensure!(
        channels != 0 && block_align != 0,
        "invalid WAVE channel/block layout"
    );
    // A compressed WAVE block does not correspond to one decoded sample frame.
    let seek_bytes_per_frame = if matches!(tag, 1 | 3) { block_align } else { 0 };
    Ok(PcmSourceFormat {
        kind: PcmSourceKind::Wave,
        format_tag: tag,
        sample_rate,
        average_bytes_per_second,
        channels,
        bits_per_sample: bits,
        block_align,
        extra_size,
        decoded_bytes: data_bytes.ok_or_else(|| anyhow::anyhow!("WAVE data chunk absent"))?,
        seek_bytes_per_frame,
    })
}
