//! Bounded Ogg decoding for memory-limited hosts. Seeking recreates the codec
//! and prerolls from zero, preserving the static decoder's sample timeline.
use std::{io::Cursor, sync::Arc};
use kira::{Frame, sound::{FromFileError, streaming::{Decoder, StreamingSoundData}}};
use symphonia::core::{
    audio::SampleBuffer, codecs::Decoder as CodecDecoder, formats::FormatReader,
    io::MediaSourceStream,
};
use super::{PcmSourceFormat, decode, source_format};

#[derive(Clone)]
pub struct StreamAsset {
    encoded: Arc<[u8]>,
    pub sample_rate: u32,
    pub frame_count: usize,
    pub source_format: PcmSourceFormat,
}

impl StreamAsset {
    pub fn new(bytes: &[u8]) -> anyhow::Result<Self> {
        anyhow::ensure!(bytes.starts_with(b"OggS"), "streaming PAL asset is not Ogg");
        let encoded: Arc<[u8]> = Arc::from(bytes);
        let mut frame_count = 0usize;
        let sample_rate = decode::visit_frames(encoded.clone(), |frames| {
            frame_count = frame_count.checked_add(frames.len())
                .ok_or_else(|| anyhow::anyhow!("streaming audio length overflow"))?;
            Ok(())
        })?;
        anyhow::ensure!(frame_count != 0, "streaming audio contains no frames");
        let source_format = source_format::describe_decoded_source(bytes, sample_rate, frame_count)?;
        Ok(Self { encoded, sample_rate, frame_count, source_format })
    }

    pub fn decoder(&self) -> Result<StreamDecoder, FromFileError> {
        StreamDecoder::new(self.clone())
    }

    pub fn sound(&self) -> Result<StreamingSoundData<FromFileError>, FromFileError> {
        Ok(StreamingSoundData::from_decoder(self.decoder()?))
    }
}

pub struct StreamDecoder {
    asset: StreamAsset,
    format: Box<dyn FormatReader>,
    codec: Box<dyn CodecDecoder>,
    track_id: u32,
    position: usize,
}

impl StreamDecoder {
    fn new(asset: StreamAsset) -> Result<Self, FromFileError> {
        let stream = MediaSourceStream::new(Box::new(Cursor::new(asset.encoded.clone())), Default::default());
        let format = symphonia::default::get_probe().format(
            &Default::default(), stream, &Default::default(), &Default::default())?.format;
        let track = format.default_track().ok_or(FromFileError::NoDefaultTrack)?;
        let codec = symphonia::default::get_codecs().make(&track.codec_params, &Default::default())?;
        let track_id = track.id;
        Ok(Self { asset, format, codec, track_id, position: 0 })
    }
}

impl Decoder for StreamDecoder {
    type Error = FromFileError;
    fn sample_rate(&self) -> u32 { self.asset.sample_rate }
    fn num_frames(&self) -> usize { self.asset.frame_count }
    fn decode(&mut self) -> Result<Vec<Frame>, Self::Error> {
        if self.position >= self.asset.frame_count { return Ok(Vec::new()); }
        let packet = loop {
            let packet = self.format.next_packet()?;
            if packet.track_id() == self.track_id { break packet; }
        };
        let decoded = self.codec.decode(&packet)?;
        let channels = decoded.spec().channels.count();
        if !matches!(channels, 1 | 2) {
            return Err(FromFileError::UnsupportedChannelConfiguration);
        }
        let mut samples = SampleBuffer::<f32>::new(decoded.capacity() as u64, *decoded.spec());
        samples.copy_interleaved_ref(decoded);
        let frames: Vec<_> = samples.samples().chunks_exact(channels)
            .map(|sample| Frame::new(sample[0], if channels == 1 { sample[0] } else { sample[1] }))
            .collect();
        self.position += frames.len();
        Ok(frames)
    }
    fn seek(&mut self, _index: usize) -> Result<usize, Self::Error> {
        // Container timestamps can omit Vorbis preroll. The scheduler discards
        // decoded frames before the requested index after this exact reset.
        *self = Self::new(self.asset.clone())?;
        Ok(0)
    }
}
