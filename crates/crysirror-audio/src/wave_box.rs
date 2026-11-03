//! Bounded Buriko wave containers over the shared streaming audio decoders.
use crate::decoder::{AudioSource, SourceBufferStatus, SourceRead, StereoFrame};
use anyhow::{Result, ensure};
use crysirror_core::service::AssetReader;
use std::sync::Arc;

#[derive(Clone, Copy, Debug)]
pub struct WaveBoxInfo {
    pub payload_offset: u32,
    pub payload_bytes: u32,
    pub frames: u32,
    pub sample_rate: u32,
    pub channels: u32,
    pub loop_start: u32,
}
impl WaveBoxInfo {
    pub fn parse(header: &[u8]) -> Option<Self> {
        if header.len() < 64 || header.get(4..8) != Some(b"bw  ") {
            return None;
        }
        let word = |at| u32::from_le_bytes(header[at..at + 4].try_into().unwrap());
        Some(Self {
            payload_offset: word(0),
            payload_bytes: word(8),
            frames: word(12),
            sample_rate: word(16),
            channels: word(20),
            loop_start: word(28),
        })
    }
    pub fn validate(self, bytes: u64) -> Result<()> {
        ensure!(
            self.payload_offset >= 64
                && self.payload_bytes > 0
                && u64::from(self.payload_offset) + u64::from(self.payload_bytes) <= bytes,
            "wave box payload range"
        );
        ensure!(
            self.frames > 0 && self.sample_rate > 0 && matches!(self.channels, 1 | 2),
            "wave box audio metadata"
        );
        ensure!(self.loop_start <= self.frames, "wave box loop range");
        Ok(())
    }
}

pub(crate) fn open(
    reader: Arc<dyn AssetReader>,
    info: WaveBoxInfo,
) -> Result<Box<dyn AudioSource>> {
    info.validate(reader.len()?)?;
    let payload = crysirror_resource::subrange(
        reader,
        u64::from(info.payload_offset),
        u64::from(info.payload_bytes),
    )?;
    let inner = crate::decoder::open_reader(
        Box::new(crate::decoder::AssetMedia(crysirror_io::AssetIo::new(
            payload,
        )?)),
        Some("ogg"),
    )?;
    Ok(Box::new(Limited {
        inner,
        limit: u64::from(info.frames),
        position: 0,
    }))
}
struct Limited {
    inner: Box<dyn AudioSource>,
    limit: u64,
    position: u64,
}
impl AudioSource for Limited {
    fn has_replay_gain(&self) -> bool {
        self.inner.has_replay_gain()
    }
    fn channel_count(&self) -> Option<u16> {
        self.inner.channel_count()
    }
    fn sample_rate(&self) -> u32 {
        self.inner.sample_rate()
    }
    fn frame_count(&self) -> Option<u64> {
        Some(
            self.inner
                .frame_count()
                .unwrap_or(self.limit)
                .min(self.limit),
        )
    }
    fn read(&mut self, out: &mut [StereoFrame]) -> Result<usize> {
        let count = out
            .len()
            .min(self.limit.saturating_sub(self.position) as usize);
        if count == 0 {
            return Ok(0);
        }
        let n = self.inner.read(&mut out[..count])?;
        self.position += n as u64;
        Ok(n)
    }
    fn seek(&mut self, frame: u64) -> Result<()> {
        let frame = frame.min(self.limit);
        self.inner.seek(frame)?;
        self.position = frame;
        Ok(())
    }
    fn poll_seek(&mut self, frame: u64) -> Result<bool> {
        let frame = frame.min(self.limit);
        let ready = self.inner.poll_seek(frame)?;
        if ready {
            self.position = frame;
        }
        Ok(ready)
    }
    fn service(&mut self) -> Result<()> {
        self.inner.service()
    }
    fn read_position(&self) -> Option<u64> {
        Some(self.position)
    }
    fn is_nonblocking(&self) -> bool {
        self.inner.is_nonblocking()
    }
    fn poll_read(&mut self, out: &mut [StereoFrame]) -> Result<SourceRead> {
        let count = out
            .len()
            .min(self.limit.saturating_sub(self.position) as usize);
        if count == 0 {
            return Ok(SourceRead::End);
        }
        let result = self.inner.poll_read(&mut out[..count])?;
        if let SourceRead::Frames(n) = result {
            self.position += n as u64;
        }
        Ok(result)
    }
    fn buffer_status(&self) -> Option<SourceBufferStatus> {
        self.inner.buffer_status()
    }
}
