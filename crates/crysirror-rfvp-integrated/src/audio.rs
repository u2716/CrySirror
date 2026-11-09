//! Native RFVP mixing over CrySirror's streaming codec API, without devices.
use anyhow::{Result, ensure};
use crysirror_audio::decoder::{self, AudioSource};
use crysirror_rfvp_native as rfvp;
use rfvp::host_api::*;
use std::{collections::BTreeMap, sync::Arc};
struct Source {
    stream: AudioByteStream,
    len: u64,
}
impl crysirror_core::service::AssetReader for Source {
    fn len(&self) -> crysirror_core::Result<u64> {
        Ok(self.len)
    }
    fn read_at(&self, offset: u64, b: &mut [u8]) -> crysirror_core::Result<usize> {
        self.stream
            .read_at(offset, b)
            .map_err(|e| crysirror_core::Error::Io(format!("RFVP audio read: {e:?}")))
    }
}
struct Codecs;
struct Decoder {
    source: Box<dyn AudioSource>,
    scratch: Vec<[f32; 2]>,
}
impl Codecs {
    fn open_reader(source: Box<dyn AudioSource>) -> RfvpResult<(Decoder, AudioStreamDesc)> {
        let rate = source.sample_rate();
        if rate == 0 {
            return Err(RfvpError::InvalidData);
        }
        Ok((
            Decoder {
                source,
                scratch: Vec::new(),
            },
            AudioStreamDesc {
                sample_rate: rate,
                channels: 2,
                sample_format: AudioSampleFormat::I16,
            },
        ))
    }
}
// SoftAudioVorbis is the original mixer's pluggable encoded-stream interface.
// CrySirror supplies all its codecs here, including WAV, MP3 and FLAC.
impl SoftAudioVorbis for Codecs {
    type Decoder = Decoder;
    fn open(&mut self, bytes: &[u8]) -> RfvpResult<(Decoder, AudioStreamDesc)> {
        Self::open_reader(
            decoder::open_memory(Arc::from(bytes), None).map_err(|_| RfvpError::InvalidData)?,
        )
    }
    fn open_stream(&mut self, stream: AudioByteStream) -> RfvpResult<(Decoder, AudioStreamDesc)> {
        let len = stream.len()?;
        Self::open_reader(
            decoder::open_asset(Arc::new(Source { stream, len }), None)
                .map_err(|_| RfvpError::InvalidData)?,
        )
    }
    fn decode_interleaved_i16(&mut self, d: &mut Decoder, out: &mut [i16]) -> RfvpResult<usize> {
        d.scratch.resize(out.len() / 2, [0.; 2]);
        let n = d.source.read(&mut d.scratch).map_err(|_| RfvpError::Io)?;
        for (dst, frame) in out.chunks_exact_mut(2).zip(&d.scratch[..n]) {
            dst[0] = (frame[0].clamp(-1., 1.) * 32767.).round() as i16;
            dst[1] = (frame[1].clamp(-1., 1.) * 32767.).round() as i16;
        }
        Ok(n * 2)
    }
    fn seek_start(&mut self, d: &mut Decoder) -> RfvpResult<()> {
        d.source.seek(0).map_err(|_| RfvpError::Io)
    }
    fn close(&mut self, _: Decoder) {}
}
pub(super) struct Audio {
    mixer: SoftAudioMixer<Codecs>,
    loaded: BTreeMap<u32, usize>,
    carry: u64,
}
impl Audio {
    pub fn new() -> Self {
        Self {
            mixer: SoftAudioMixer::new(
                Codecs,
                SoftAudioConfig {
                    mix_frames: 480,
                    ..Default::default()
                },
            ),
            loaded: BTreeMap::new(),
            carry: 0,
        }
    }
    pub fn observations(&self) -> Vec<(AudioStreamId, bool)> {
        self.loaded
            .keys()
            .map(|&id| (AudioStreamId(id), self.mixer.is_playing(AudioStreamId(id))))
            .collect()
    }
    pub fn render(&mut self, ms: u64) -> Result<Vec<[f32; 2]>> {
        ensure!(ms <= 100, "RFVP audio time budget");
        self.carry += ms * 48;
        let mut out = Vec::with_capacity(self.carry as usize);
        let mut block = [0i16; 960];
        while self.carry >= 480 {
            self.mixer
                .mix_next(&mut block)
                .map_err(|e| anyhow::anyhow!("RFVP mix: {e:?}"))?;
            out.extend(
                block
                    .chunks_exact(2)
                    .map(|s| [s[0] as f32 / 32768., s[1] as f32 / 32768.]),
            );
            self.carry -= 480;
        }
        Ok(out)
    }
    fn reserve(&mut self, id: AudioStreamId, n: usize) -> RfvpResult<()> {
        let total = self
            .loaded
            .values()
            .copied()
            .sum::<usize>()
            .saturating_sub(self.loaded.get(&id.0).copied().unwrap_or(0))
            .saturating_add(n);
        if n > 64 * 1024 * 1024
            || total > 256 * 1024 * 1024
            || (!self.loaded.contains_key(&id.0) && self.loaded.len() >= 512)
        {
            return Err(RfvpError::CapacityExceeded);
        }
        self.loaded.insert(id.0, n);
        Ok(())
    }
}
impl RfvpAudio for Audio {
    fn load_encoded(
        &mut self,
        id: AudioStreamId,
        kind: EncodedAudioKind,
        bytes: &[u8],
    ) -> RfvpResult<()> {
        self.reserve(id, bytes.len())?;
        self.mixer.load_stream(
            id,
            kind,
            AudioByteStream::new(Box::new(rfvp::io::Cursor::new(bytes.to_vec()))),
        )
    }
    fn load_stream(
        &mut self,
        id: AudioStreamId,
        kind: EncodedAudioKind,
        stream: AudioByteStream,
    ) -> RfvpResult<()> {
        self.reserve(id, 0)?;
        self.mixer.load_stream(id, kind, stream)
    }
    fn create_stream(&mut self, id: AudioStreamId, desc: AudioStreamDesc) -> RfvpResult<()> {
        self.reserve(id, 0)?;
        self.mixer.create_stream(id, desc)
    }
    fn submit_i16(&mut self, id: AudioStreamId, s: &[i16]) -> RfvpResult<()> {
        let n = self
            .loaded
            .get(&id.0)
            .copied()
            .ok_or(RfvpError::NotFound)?
            .checked_add(s.len().checked_mul(2).ok_or(RfvpError::CapacityExceeded)?)
            .ok_or(RfvpError::CapacityExceeded)?;
        self.reserve(id, n)?;
        self.mixer.submit_i16(id, s)
    }
    fn submit_f32(&mut self, id: AudioStreamId, s: &[f32]) -> RfvpResult<()> {
        let n = self
            .loaded
            .get(&id.0)
            .copied()
            .ok_or(RfvpError::NotFound)?
            .checked_add(s.len().checked_mul(2).ok_or(RfvpError::CapacityExceeded)?)
            .ok_or(RfvpError::CapacityExceeded)?;
        self.reserve(id, n)?;
        self.mixer.submit_f32(id, s)
    }
    fn play(&mut self, id: AudioStreamId, p: AudioParams, fade: u32) -> RfvpResult<()> {
        self.mixer.play(id, p, fade)
    }
    fn stop(&mut self, id: AudioStreamId, fade: u32) -> RfvpResult<()> {
        self.mixer.stop(id, fade)
    }
    fn pause(&mut self, id: AudioStreamId) -> RfvpResult<()> {
        self.mixer.pause(id)
    }
    fn resume(&mut self, id: AudioStreamId) -> RfvpResult<()> {
        self.mixer.resume(id)
    }
    fn set_params(&mut self, id: AudioStreamId, p: AudioParams) -> RfvpResult<()> {
        self.mixer.set_params(id, p)
    }
    fn set_master_volume(&mut self, v: f32) -> RfvpResult<()> {
        self.mixer.set_master_volume(v)
    }
    fn destroy_stream(&mut self, id: AudioStreamId) {
        self.mixer.destroy_stream(id);
        self.loaded.remove(&id.0);
    }
    fn tick(&mut self, _: u64) -> RfvpResult<()> {
        Ok(())
    }
}

/// The same seekable native source can be retained by the managed movie worker.
pub(super) fn media_reader(
    stream: AudioByteStream,
) -> RfvpResult<Arc<dyn crysirror_core::service::AssetReader>> {
    let len = stream.len()?;
    Ok(Arc::new(Source { stream, len }))
}
