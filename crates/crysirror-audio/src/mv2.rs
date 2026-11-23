// SPDX-License-Identifier: MPL-2.0
//! MV2X subband audio. The CVNS decoder reads low bits first but assembles
//! each field most significant bit first (0x43c290). Each channel has its
//! own 1,024-sample synthesis history (0x43c400).
use crate::decoder::{AudioSource, MediaSource, StereoFrame};
use anyhow::{Result, ensure};
use std::io::{BufReader, Read, Seek, SeekFrom};

const BLOCK: usize = 320;
// Q10 cosine, one quadrant of the 128-step synthesis phase.
const COSINE: [i32; 33] = [
    1024, 1022, 1019, 1012, 1004, 993, 979, 964, 946, 925, 903, 878, 851, 822, 791, 758, 724, 687,
    649, 609, 568, 526, 482, 437, 391, 344, 297, 248, 199, 150, 100, 50, 0,
];
// Half of the symmetric 512-tap Q10 synthesis window.
const WINDOW: [i32; 257] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -2, -2, -2, -2, -2, -2,
    -2, -2, -2, -3, -3, -3, -3, -3, -3, -3, -3, -3, -3, -3, -3, -3, -3, -3, -3, -2, -2, -2, -2, -1,
    -1, -1, 0, 0, 0, 0, 1, 1, 2, 3, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 20,
    21, 22, 23, 24, 25, 26, 27, 28, 29, 29, 30, 31, 31, 32, 32, 32, 32, 32, 32, 31, 31, 30, 29, 28,
    27, 25, 23, 22, 20, 17, 15, 12, 9, 6, 2, 0, -4, -8, -12, -17, -21, -26, -31, -36, -41, -46,
    -52, -57, -63, -69, -74, -80, -86, -91, -97, -102, -108, -113, -118, -123, -128, -132, -136,
    -140, -144, -147, -149, -151, -153, -154, -155, -155, -155, -154, -152, -149, -146, -142, -138,
    -132, -126, -119, -111, -102, -93, -82, -71, -58, -45, -31, -16, -1, 15, 33, 51, 70, 90, 111,
    133, 155, 178, 202, 227, 252, 278, 304, 331, 358, 385, 413, 442, 470, 499, 527, 556, 585, 614,
    643, 671, 700, 728, 756, 783, 810, 836, 862, 887, 911, 934, 957, 979, 1000, 1020, 1038, 1056,
    1073, 1088, 1102, 1115, 1127, 1138, 1147, 1154, 1161, 1166, 1169, 1171, 1172,
];
// The lower 136 scale indices round to zero.
const SCALE: [i32; 120] = [
    1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 4, 4, 5, 6, 6, 7, 8, 10, 11, 12, 14, 16, 18, 20, 23, 26,
    29, 33, 38, 42, 48, 54, 61, 69, 78, 88, 100, 113, 128, 144, 163, 184, 207, 234, 265, 299, 337,
    381, 430, 486, 548, 619, 699, 789, 891, 1006, 1136, 1282, 1448, 1634, 1845, 2083, 2352, 2655,
    2998, 3385, 3821, 4314, 4870, 5499, 6208, 7009, 7912, 8933, 10085, 11386, 12854, 14512, 16384,
    18496, 20882, 23575, 26615, 30048, 33923, 38298, 43237, 48813, 55108, 62216, 70239, 79298,
    89524, 101070, 114104, 128820, 145433, 164189, 185363, 209269, 236257, 266726, 301124, 339958,
    383801, 433297, 489178, 552264, 623487, 703894, 794672, 897156, 1012857, 1143480, 1290948,
    1457434, 1645392, 1857589,
];

fn coefficient(row: usize, band: usize) -> i32 {
    let phase = ((16 + row) * (2 * band + 1)) & 127;
    let phase = phase.min(128 - phase);
    if phase <= 32 {
        COSINE[phase]
    } else {
        -COSINE[64 - phase]
    }
}

fn scale(index: usize) -> i32 {
    match index {
        0..136 => 0,
        136..256 => SCALE[index - 136],
        // 0x43c330 indexes across the following packed Q10 cosine words.
        _ => {
            let pair = (index - 256) * 2;
            let low = coefficient(pair / 32, pair % 32) as u16;
            let high = coefficient(pair / 32, pair % 32 + 1);
            high << 16 | i32::from(low)
        }
    }
}

struct Bits {
    input: BufReader<Box<dyn MediaSource>>,
    byte: u8,
    left: u8,
}

impl Bits {
    fn field(&mut self, width: u32) -> Result<u32> {
        let mut value = 0;
        for _ in 0..width {
            if self.left == 0 {
                let mut byte = [0];
                self.input.read_exact(&mut byte)?;
                self.byte = byte[0];
                self.left = 8;
            }
            value = (value << 1) | u32::from(self.byte & 1);
            self.byte >>= 1;
            self.left -= 1;
        }
        Ok(value)
    }

    fn bands(&mut self) -> Result<[i32; BLOCK]> {
        let count = self.field(10)? as usize;
        ensure!(count <= BLOCK, "MV2 spectral band count");
        let scale = scale(self.field(9)? as usize);
        let mut bands = [0; BLOCK];
        let mut at = 0;
        while at < count {
            let mut width = 0;
            while self.field(1)? != 0 {
                width += 1;
                ensure!(width < 32, "MV2 coefficient width");
            }
            if width == 0 {
                at += 1 + self.field(3)? as usize;
            } else {
                let raw = self.field(width)?;
                let signed = if raw < 1 << (width - 1) {
                    (raw as i32)
                        .wrapping_sub(1u32.wrapping_shl(width) as i32)
                        .wrapping_add(1)
                } else {
                    raw as i32
                };
                bands[at] = scale.wrapping_mul(signed);
                at += 1;
            }
        }
        Ok(bands)
    }
}

#[derive(Clone)]
struct Synthesis {
    history: [i32; 1024],
    cursor: usize,
}

impl Synthesis {
    fn new() -> Self {
        Self {
            history: [0; 1024],
            cursor: 0,
        }
    }

    fn group(&mut self, bands: &[i32], shift: u32, out: &mut [StereoFrame], channel: usize) {
        for (row, value) in self.history[self.cursor..self.cursor + 64]
            .iter_mut()
            .enumerate()
        {
            *value = bands.iter().enumerate().fold(0i32, |sum, (band, sample)| {
                sum.wrapping_add(sample.wrapping_mul(coefficient(row, band)) >> 10)
            });
        }
        for (i, frame) in out.iter_mut().enumerate() {
            let mut sum = 0i32;
            for tap in 0..16 {
                let position = self.cursor + i + (tap / 2) * 128 + (tap % 2) * 96;
                let window = i + tap * 32;
                let weight = WINDOW[window.min(512 - window)];
                let term = self.history[position & 1023].wrapping_mul(weight) >> 10;
                sum = if tap & 2 == 0 {
                    sum.wrapping_add(term)
                } else {
                    sum.wrapping_sub(term)
                };
            }
            frame[channel] = (sum >> shift).clamp(-32767, 32767) as f32 / 32768.;
        }
        self.cursor = self.cursor.wrapping_sub(64) & 1023;
    }
}

struct Stream {
    bits: Bits,
    states: Vec<Synthesis>,
    data_start: u64,
    rate: u32,
    frames: u64,
    shift: u32,
    position: u64,
    pcm: [StereoFrame; BLOCK],
}

pub(crate) fn open(mut reader: Box<dyn MediaSource>) -> Result<Box<dyn AudioSource>> {
    let mut header = [0; 18];
    reader.read_exact(&mut header)?;
    let data_start = reader.stream_position()?;
    let frames = u32::from_le_bytes(header[4..8].try_into()?);
    let rate = u16::from_le_bytes(header[10..12].try_into()?);
    let channels = header[12];
    let blocks = u32::from_le_bytes(header[14..18].try_into()?);
    ensure!(
        &header[..4] == b"MV2X" && (1..=2).contains(&channels) && rate > 0,
        "invalid MV2 format"
    );
    ensure!(
        frames > 0 && frames.div_ceil(BLOCK as u32) == blocks,
        "MV2 frame/block count"
    );
    Ok(Box::new(Stream {
        bits: Bits {
            input: BufReader::new(reader),
            byte: 0,
            left: 0,
        },
        states: vec![Synthesis::new(); channels as usize],
        data_start,
        rate: u32::from(rate),
        frames: u64::from(frames),
        shift: 6u32.wrapping_sub(u32::from(header[13])) & 31,
        position: 0,
        pcm: [[0.; 2]; BLOCK],
    }))
}

impl Stream {
    fn decode_block(&mut self) -> Result<()> {
        for (channel, state) in self.states.iter_mut().enumerate() {
            let bands = self.bits.bands()?;
            for (group, pcm) in bands.chunks_exact(32).zip(self.pcm.chunks_exact_mut(32)) {
                state.group(group, self.shift, pcm, channel);
            }
        }
        if self.states.len() == 1 {
            for frame in &mut self.pcm {
                frame[1] = frame[0];
            }
        }
        Ok(())
    }
}

impl AudioSource for Stream {
    fn channel_count(&self) -> Option<u16> {
        Some(self.states.len() as u16)
    }
    fn sample_rate(&self) -> u32 {
        self.rate
    }
    fn frame_count(&self) -> Option<u64> {
        Some(self.frames)
    }

    fn read(&mut self, mut out: &mut [StereoFrame]) -> Result<usize> {
        let mut total = 0;
        while !out.is_empty() && self.position < self.frames {
            let offset = (self.position % BLOCK as u64) as usize;
            if offset == 0 {
                self.decode_block()?;
            }
            let count = out
                .len()
                .min(BLOCK - offset)
                .min((self.frames - self.position) as usize);
            out[..count].copy_from_slice(&self.pcm[offset..offset + count]);
            out = &mut out[count..];
            self.position += count as u64;
            total += count;
        }
        Ok(total)
    }

    fn seek(&mut self, frame: u64) -> Result<()> {
        ensure!(frame <= self.frames, "MV2 seek beyond end");
        if frame < self.position {
            self.bits.input.seek(SeekFrom::Start(self.data_start))?;
            self.states.fill(Synthesis::new());
            self.position = 0;
            self.bits.left = 0;
        }
        while self.position < frame {
            let offset = (self.position % BLOCK as u64) as usize;
            if offset == 0 {
                self.decode_block()?;
            }
            self.position += (frame - self.position).min((BLOCK - offset) as u64);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    // One impulse in the first subband, followed by silence. The second
    // channel starts with empty history. Only one frame of block two is used.
    fn stereo_impulse() -> Vec<u8> {
        let mut bytes = b"MV2X".to_vec();
        bytes.extend_from_slice(&321u32.to_le_bytes());
        bytes.extend_from_slice(&128u16.to_le_bytes());
        bytes.extend_from_slice(&22050u16.to_le_bytes());
        bytes.extend_from_slice(&[2, 4]);
        bytes.extend_from_slice(&2u32.to_le_bytes());
        let mut cursor = bytes.len() * 8;
        for (width, value) in [
            (10, 1u32),
            (9, 216),
            (3, 5),
            (10, 0),
            (9, 0),
            (10, 0),
            (9, 0),
            (10, 0),
            (9, 0),
        ] {
            for bit in (0..width).rev() {
                if cursor % 8 == 0 {
                    bytes.push(0);
                }
                bytes[cursor / 8] |= (((value >> bit) & 1) as u8) << (cursor % 8);
                cursor += 1;
            }
        }
        bytes
    }

    #[test]
    fn synthesis_keeps_channel_history_and_seek_reconstructs_it() -> Result<()> {
        let bytes: Arc<[u8]> = stereo_impulse().into();
        let mut source = crate::decoder::open_memory(bytes.clone(), None)?;
        let mut frames = [[0.; 2]; 322];
        assert_eq!(source.channel_count(), Some(2));
        assert_eq!(source.sample_rate(), 22050);
        assert_eq!(source.read(&mut frames[..45])?, 45);
        assert_eq!(source.read(&mut frames[45..])?, 276);
        for (frame, value) in [
            (45, 3),
            (97, 23),
            (193, 249),
            (245, 4220),
            (300, -989),
            (320, 288),
        ] {
            assert_eq!(frames[frame], [value as f32 / 32768., 0.]);
        }
        source.seek(315)?;
        assert_eq!(source.read(&mut frames[..10])?, 6);
        assert_eq!(frames[..6], frames[315..321]);
        assert_eq!(source.read(&mut frames)?, 0);
        let mut cut = crate::decoder::open_reader(
            Box::new(std::io::Cursor::new(bytes[..bytes.len() - 1].to_vec())),
            None,
        )?;
        assert!(cut.read(&mut frames).is_err());
        Ok(())
    }
}
