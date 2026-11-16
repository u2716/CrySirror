//! XPCM mode 2: first-generation Circus adaptive 8-bit delta (D.C. DC.EXE
//! 42CD60/42CB10). Each signed byte adds `byte << shift` to its channel's
//! running value, which is stored as a 16-bit sample. A zero byte lowers the
//! shift by one (not below 0); +127 or -128 raises it by one (not above 8).
//! Stereo channels interleave per byte. The payload follows the 28-byte
//! header and holds one byte per decoded 16-bit sample.
use super::View;
use crate::decoder::{AudioSource, StereoFrame};
use anyhow::{Result, ensure};
use std::io::{Read, Seek, SeekFrom};

#[derive(Clone, Copy, Default)]
struct Channel {
    value: i32,
    shift: u32,
}
impl Channel {
    fn step(&mut self, byte: i8) -> i16 {
        self.value = self.value.wrapping_add(i32::from(byte).wrapping_shl(self.shift));
        if byte == 0 {
            self.shift = self.shift.saturating_sub(1);
        } else if (byte == 127 || byte == -128) && self.shift != 8 {
            self.shift += 1;
        }
        //42CB10 stores the low word of the running dword.
        self.value as u16 as i16
    }
}

pub(super) struct Delta {
    view: View,
    rate: u32,
    channels: u16,
    frames: u64,
    at: u64,
    state: [Channel; 2],
    buffer: Vec<u8>,
}
impl Delta {
    pub(super) fn new(view: View, decoded: u64, rate: u32, channels: u16) -> Result<Self> {
        ensure!(matches!(channels, 1 | 2), "XPCM delta channel count");
        let align = 2 * u64::from(channels);
        ensure!(decoded % align == 0, "XPCM delta length/alignment");
        Ok(Self {
            view,
            rate,
            channels,
            frames: decoded / align,
            at: 0,
            state: [Channel::default(); 2],
            buffer: Vec::new(),
        })
    }
}
impl AudioSource for Delta {
    fn channel_count(&self) -> Option<u16> {
        Some(self.channels)
    }
    fn sample_rate(&self) -> u32 {
        self.rate
    }
    fn frame_count(&self) -> Option<u64> {
        Some(self.frames)
    }
    fn read(&mut self, out: &mut [StereoFrame]) -> Result<usize> {
        let count = (out.len() as u64).min(self.frames - self.at) as usize;
        let width = usize::from(self.channels);
        self.buffer.resize(count * width, 0);
        self.view.read_exact(&mut self.buffer)?;
        for (frame, bytes) in out.iter_mut().zip(self.buffer.chunks_exact(width)) {
            let left = self.state[0].step(bytes[0] as i8);
            let right = if width == 2 {
                self.state[1].step(bytes[1] as i8)
            } else {
                left
            };
            *frame = [f32::from(left) / 32768., f32::from(right) / 32768.];
        }
        self.at += count as u64;
        Ok(count)
    }
    /// The running state depends on every earlier byte: decode the prefix.
    fn seek(&mut self, frame: u64) -> Result<()> {
        ensure!(frame <= self.frames, "XPCM delta seek outside stream");
        if frame < self.at {
            self.view.seek(SeekFrom::Start(0))?;
            self.at = 0;
            self.state = [Channel::default(); 2];
        }
        let mut scratch = vec![[0.; 2]; 4096];
        while self.at < frame {
            let want = ((frame - self.at) as usize).min(scratch.len());
            let read = self.read(&mut scratch[..want])?;
            ensure!(read > 0, "truncated XPCM delta payload");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn adaptive_shift_follows_native_byte_rules() {
        let mut channel = Channel::default();
        assert_eq!(channel.step(127), 127);
        assert_eq!(channel.shift, 1);
        assert_eq!(channel.step(1), 129);
        assert_eq!(channel.step(0), 129);
        assert_eq!(channel.shift, 0);
        assert_eq!(channel.step(0), 129);
        assert_eq!(channel.shift, 0);
        for _ in 0..12 {
            channel.step(-128);
        }
        assert_eq!(channel.shift, 8);
    }
}
