//! Streaming XPCM modes1/3, adapted from GARbro AudioPCM.cs (morkt, MIT;
//! full notice in circus.LICENSE). Fixed-point FFT rounding and32-sample
//! overlap stay format-exact. This codec owns no file path, mixer or device.
use super::View;
use crate::decoder::{AudioSource, MAX_PACKET_FRAMES, StereoFrame};
use anyhow::{Context, Result, ensure};
use flate2::read::ZlibDecoder;
use std::io::{BufReader, Read, Seek, SeekFrom};
#[path = "circus_spectral_tables.rs"]
mod tables;

const BLOCK_SAMPLES: usize = 4064;
const FFT_SAMPLES: usize = 4096;
const BLOCK_BYTES: usize = 8192;
const SCALE: [[i32; 8]; 4] = [
    [2048, 2048, 4096, 4096, 8192, 8192, 16384, 16384],
    [2048, 2048, 2048, 4096, 4096, 4096, 8192, 16384],
    [2048, 2048, 2048, 2048, 2048, 4096, 4096, 8192],
    [1024, 1024, 1024, 2048, 2048, 2048, 2048, 2048],
];

///The custom LZSS stream references at most65535 previously decoded bytes.
///No allocation depends on the declared PCM/music duration.
struct Lzss {
    input: BufReader<View>,
    remaining: u64,
    flags: u16,
    history: Vec<u8>,
    written: u64,
    copy: Option<(usize, usize)>,
}
impl Lzss {
    fn new(input: View) -> Self {
        let remaining = input.body_len;
        Self {
            input: BufReader::with_capacity(64 * 1024, input),
            remaining,
            flags: 0,
            history: vec![0; 65536],
            written: 0,
            copy: None,
        }
    }
    fn byte(&mut self) -> std::io::Result<u8> {
        let mut b = [0];
        self.input.read_exact(&mut b)?;
        self.remaining = self.remaining.checked_sub(1).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "XPCM LZSS compressed boundary",
            )
        })?;
        Ok(b[0])
    }
    fn word(&mut self) -> std::io::Result<usize> {
        let lo = self.byte()?;
        let hi = self.byte()?;
        Ok(usize::from(u16::from_le_bytes([lo, hi])))
    }
    fn next(&mut self) -> std::io::Result<Option<u8>> {
        let value = if let Some((distance, remaining)) = self.copy {
            let at = (self.written - distance as u64) as usize & 65535;
            self.copy = (remaining > 1).then_some((distance, remaining - 1));
            self.history[at]
        } else {
            if self.remaining == 0 {
                return Ok(None);
            }
            self.flags >>= 1;
            if self.flags & 0x100 == 0 {
                self.flags = u16::from(self.byte()?) | 0xff00;
            }
            if self.flags & 1 != 0 {
                self.byte()?
            } else {
                let ctl = self.byte()?;
                let (distance, count) = if ctl >= 0xc0 {
                    (
                        usize::from(self.byte()?) | usize::from(ctl & 3) << 8,
                        4 + usize::from(ctl >> 2 & 15),
                    )
                } else if ctl & 0x80 != 0 {
                    let distance = usize::from(ctl & 31);
                    (
                        if distance == 0 {
                            usize::from(self.byte()?)
                        } else {
                            distance
                        },
                        2 + usize::from(ctl >> 5 & 3),
                    )
                } else if ctl == 0x7f {
                    let count = 2 + self.word()?;
                    (self.word()?, count)
                } else {
                    let distance = self.word()?;
                    (distance, usize::from(ctl) + 4)
                };
                if distance == 0 || distance as u64 > self.written {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "XPCM LZSS backreference outside decoded history",
                    ));
                }
                self.copy = Some((distance, count));
                return self.next();
            }
        };
        self.history[self.written as usize & 65535] = value;
        self.written += 1;
        Ok(Some(value))
    }
}
impl Read for Lzss {
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        let mut done = 0;
        for byte in out {
            let Some(value) = self.next()? else { break };
            *byte = value;
            done += 1;
        }
        Ok(done)
    }
}
enum Encoded {
    Zlib(ZlibDecoder<View>),
    Lzss(Lzss),
}
impl Encoded {
    fn read_block(&mut self, out: &mut [u8; BLOCK_BYTES]) -> Result<()> {
        match self {
            Self::Zlib(reader) => reader.read_exact(out)?,
            Self::Lzss(reader) => reader.read_exact(out)?,
        };
        Ok(())
    }
    fn reset(self) -> Result<Self> {
        let (mut input, zlib) = match self {
            Self::Zlib(reader) => (reader.into_inner(), true),
            Self::Lzss(reader) => (reader.input.into_inner(), false),
        };
        input.seek(SeekFrom::Start(0))?;
        Ok(if zlib {
            Self::Zlib(ZlibDecoder::new(input))
        } else {
            Self::Lzss(Lzss::new(input))
        })
    }
    fn finish(&mut self) -> Result<()> {
        //GARbro's source buffer allows up to16386 unused trailing spectral
        //bytes. Bound that padding and consume the zlib trailer/CRC as well.
        let mut padding = 0usize;
        let mut scratch = [0; 4096];
        loop {
            let count = match self {
                Self::Zlib(reader) => reader.read(&mut scratch)?,
                Self::Lzss(reader) => reader.read(&mut scratch)?,
            };
            if count == 0 {
                break;
            }
            padding += count;
            ensure!(
                padding <= 16386,
                "XPCM spectral trailing coefficient budget"
            );
        }
        Ok(())
    }
}

pub(super) struct Spectral {
    encoded: Option<Encoded>,
    rate: u32,
    channels: u16,
    sample_count: u64,
    position: u64,
    decoded: u64,
    quantization: usize,
    block: Vec<i16>,
    block_at: usize,
    previous_tail: [i16; 32],
    real: Vec<i32>,
    imag: Vec<i32>,
}
impl Spectral {
    pub(super) fn new(
        input: View,
        decoded_bytes: u64,
        rate: u32,
        channels: u16,
        quantization: u8,
        zlib: bool,
    ) -> Result<Self> {
        ensure!(quantization < 4, "XPCM spectral quantization mode");
        ensure!(
            decoded_bytes % u64::from(channels * 2) == 0,
            "XPCM spectral PCM alignment"
        );
        Ok(Self {
            encoded: Some(if zlib {
                Encoded::Zlib(ZlibDecoder::new(input))
            } else {
                Encoded::Lzss(Lzss::new(input))
            }),
            rate,
            channels,
            sample_count: decoded_bytes / 2,
            position: 0,
            decoded: 0,
            quantization: usize::from(quantization),
            block: Vec::new(),
            block_at: 0,
            previous_tail: [0; 32],
            real: vec![0; FFT_SAMPLES],
            imag: vec![0; FFT_SAMPLES],
        })
    }
    fn decode_block(&mut self) -> Result<bool> {
        if self.decoded >= self.sample_count {
            return Ok(false);
        }
        let mut encoded = [0u8; BLOCK_BYTES];
        self.encoded
            .as_mut()
            .context("XPCM spectral stream owner")?
            .read_block(&mut encoded)?;
        self.real.fill(0);
        self.imag.fill(0);
        for index in 0..2048 {
            let high0 = encoded[2 * index];
            let high1 = encoded[2 * index + 1];
            let a = encoded[4096 + index];
            let b = encoded[6144 + index];
            let lo0 = (b >> 4) | (a & 0xf0);
            let lo1 = (a << 4) | (b & 15);
            let coefficient = |lo, hi| {
                let index = u16::from_le_bytes([lo, hi]);
                if index & 1 == 0 {
                    -i32::from(index / 2)
                } else {
                    i32::from((index / 2 + 1) as i16)
                }
            };
            let scale = SCALE[self.quantization][index / 256];
            self.real[index] = scale.wrapping_mul(coefficient(lo0, high0));
            self.imag[index] = scale.wrapping_mul(coefficient(lo1, high1));
        }
        fft(&mut self.real, &mut self.imag);
        let mut pcm = [0i16; FFT_SAMPLES];
        for (index, (out, &value)) in pcm.iter_mut().zip(&self.real).enumerate() {
            let value = if index < 32 && self.decoded != 0 {
                //The original overlap deliberately casts the FFT value to
                //uint before the64-bit sum; retain its carry correction.
                let mixed = index as i64 * i64::from(value as u32)
                    + (32 - index) as i64 * i64::from(self.previous_tail[index]);
                ((mixed as i32).wrapping_add(((mixed >> 32) & 31) as i32)) >> 5
            } else {
                value
            };
            *out = value.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
        }
        self.previous_tail.copy_from_slice(&pcm[BLOCK_SAMPLES..]);
        let count = BLOCK_SAMPLES.min((self.sample_count - self.decoded) as usize);
        self.block.clear();
        self.block.extend_from_slice(&pcm[..count]);
        self.block_at = 0;
        self.decoded += count as u64;
        if self.decoded == self.sample_count {
            self.encoded.as_mut().unwrap().finish()?;
        }
        Ok(true)
    }
    fn reset(&mut self) -> Result<()> {
        let owner = self.encoded.take().context("XPCM spectral stream owner")?;
        self.encoded = Some(owner.reset()?);
        self.position = 0;
        self.decoded = 0;
        self.block.clear();
        self.block_at = 0;
        self.previous_tail.fill(0);
        Ok(())
    }
}
impl AudioSource for Spectral {
    fn sample_rate(&self) -> u32 {
        self.rate
    }
    fn channel_count(&self) -> Option<u16> {
        Some(self.channels)
    }
    fn frame_count(&self) -> Option<u64> {
        Some(self.sample_count / u64::from(self.channels))
    }
    fn read_position(&self) -> Option<u64> {
        Some(self.position)
    }
    fn read(&mut self, out: &mut [StereoFrame]) -> Result<usize> {
        ensure!(out.len() <= MAX_PACKET_FRAMES, "XPCM packet frame budget");
        let mut count = 0;
        for frame in out {
            if self.block_at == self.block.len() && !self.decode_block()? {
                break;
            }
            let left = f32::from(self.block[self.block_at]) / 32768.;
            self.block_at += 1;
            let right = if self.channels == 2 {
                let sample = f32::from(self.block[self.block_at]) / 32768.;
                self.block_at += 1;
                sample
            } else {
                left
            };
            *frame = [left, right];
            self.position += 1;
            count += 1;
        }
        Ok(count)
    }
    fn seek(&mut self, frame: u64) -> Result<()> {
        ensure!(
            frame <= self.frame_count().unwrap(),
            "XPCM seek outside decoded frames"
        );
        if frame == self.position {
            return Ok(());
        }
        self.reset()?;
        let mut remaining = frame * u64::from(self.channels);
        while remaining > 0 {
            ensure!(self.decode_block()?, "XPCM seek truncated spectral stream");
            let consume = remaining.min(self.block.len() as u64) as usize;
            self.block_at = consume;
            remaining -= consume as u64;
        }
        self.position = frame;
        Ok(())
    }
}

///Radix2 DIF form ofGARbro's fixed-point sub_411AB0. The two quadrants
///preserve their distinct negative rounding; this is not a floating FFT.
fn fft(real: &mut [i32], imag: &mut [i32]) {
    let mut span = FFT_SAMPLES;
    while span > 2 {
        let half = span / 2;
        let quarter = span / 4;
        let stride = FFT_SAMPLES / span;
        for base in (0..FFT_SAMPLES).step_by(span) {
            for j in 0..half {
                let left = base + j;
                let right = left + half;
                let r = real[left].wrapping_sub(real[right]);
                let i = imag[left].wrapping_sub(imag[right]);
                real[left] = real[left].wrapping_add(real[right]);
                imag[left] = imag[left].wrapping_add(imag[right]);
                let (r, i) = if j == 0 {
                    (r, i)
                } else if j == quarter {
                    (i, r.wrapping_neg())
                } else {
                    let second = j > quarter;
                    let angle = if second { j - quarter } else { j } * stride;
                    let c = tables::TWIDDLE[2 * angle];
                    let s = tables::TWIDDLE[2 * angle + 1];
                    let mul = |x: i32, y: i32| ((i64::from(x) * i64::from(y)) >> 12) as i32;
                    if second {
                        (
                            mul(i, c).wrapping_sub(mul(r, s)),
                            mul(r, c).wrapping_add(mul(i, s)).wrapping_neg(),
                        )
                    } else {
                        (
                            mul(r, c).wrapping_add(mul(i, s)),
                            mul(i, c).wrapping_sub(mul(r, s)),
                        )
                    }
                };
                real[right] = r;
                imag[right] = i;
            }
        }
        span /= 2;
    }
    for left in (0..FFT_SAMPLES).step_by(2) {
        let right = left + 1;
        let r = real[left].wrapping_sub(real[right]);
        let i = imag[left].wrapping_sub(imag[right]);
        real[left] = real[left].wrapping_add(real[right]);
        imag[left] = imag[left].wrapping_add(imag[right]);
        real[right] = r;
        imag[right] = i;
    }
    for index in 1..FFT_SAMPLES - 1 {
        let reversed = index.reverse_bits() >> (usize::BITS - 12);
        if index < reversed {
            real.swap(index, reversed);
            imag.swap(index, reversed);
        }
    }
    for value in real {
        let shifted = value.wrapping_shl(4);
        *value = shifted.wrapping_add((shifted >> 31) & 0x3fff) >> 14;
    }
}
