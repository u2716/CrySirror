// SPDX-License-Identifier: MPL-2.0
//! FMOD sound banks (`FSB5`, as Unity stores `AudioClip` resources) to
//! streams the shared decoders read: Vorbis samples become Ogg Vorbis
//! (identification and comment headers rebuilt, the setup header supplied by
//! the caller and checked against the bank's CRC-32, granule positions from
//! the packets' block sizes), PCM16 and IMA ADPCM samples become WAVE.
//!
//! FSB5 Vorbis banks omit the setup header; FMOD keeps the known headers in
//! its runtime. [`find_vorbis_setup`] locates one by CRC-32 in a binary the
//! user installed (the player that embeds FMOD); nothing is bundled.
use anyhow::{bail, ensure, Context, Result};

pub const MODE_PCM16: u32 = 2;
pub const MODE_IMA_ADPCM: u32 = 7;
pub const MODE_VORBIS: u32 = 15;

const FREQUENCIES: [u32; 11] = [0, 8000, 11000, 11025, 16000, 22050, 24000, 32000, 44100, 48000, 96000];

#[derive(Clone, Debug, PartialEq)]
pub struct Sample {
    pub channels: u8,
    pub rate: u32,
    /// Frames per channel.
    pub frames: u64,
    pub loop_points: Option<(u32, u32)>,
    pub vorbis_crc: Option<u32>,
    /// Byte range of the sample's data in the bank.
    pub data: std::ops::Range<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Bank {
    pub mode: u32,
    pub samples: Vec<Sample>,
}

fn u32le(b: &[u8], o: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(b.get(o..o + 4).context("FSB5 truncated")?.try_into()?))
}

impl Bank {
    pub fn parse(b: &[u8]) -> Result<Self> {
        ensure!(b.get(..4) == Some(b"FSB5".as_slice()), "not an FSB5 bank");
        let version = u32le(b, 4)?;
        let count = u32le(b, 8)? as usize;
        let headers = u32le(b, 12)? as usize;
        let names = u32le(b, 16)? as usize;
        let data_size = u32le(b, 20)? as usize;
        let mode = u32le(b, 24)?;
        let base = match version {
            0 => 0x40,
            1 => 0x3C,
            v => bail!("FSB5 version {v}"),
        };
        let data_start = base + headers + names;
        let data_end = (data_start + data_size).min(b.len());
        ensure!(count <= 4096, "FSB5 sample count");
        let mut at = base;
        let mut samples = Vec::with_capacity(count);
        let mut offsets = Vec::with_capacity(count);
        for _ in 0..count {
            let bits = u64::from_le_bytes(b.get(at..at + 8).context("FSB5 sample header")?.try_into()?);
            at += 8;
            let mut s = Sample {
                channels: ((bits >> 5) & 1) as u8 + 1,
                rate: FREQUENCIES.get(((bits >> 1) & 0xF) as usize).copied().unwrap_or(44100),
                frames: (bits >> 34) & 0x3FFF_FFFF,
                loop_points: None,
                vorbis_crc: None,
                data: 0..0,
            };
            offsets.push(((bits >> 6) & 0x0FFF_FFFF) as usize * 16);
            let mut more = bits & 1 != 0;
            while more {
                let c = u32le(b, at)?;
                more = c & 1 != 0;
                let size = ((c >> 1) & 0xFF_FFFF) as usize;
                let body = at + 4;
                ensure!(body + size <= b.len(), "FSB5 chunk past end");
                match c >> 25 {
                    1 => s.channels = b[body],
                    2 => s.rate = u32le(b, body)?,
                    3 => s.loop_points = Some((u32le(b, body)?, u32le(b, body + 4)?)),
                    11 => s.vorbis_crc = Some(u32le(b, body)?),
                    _ => {}
                }
                at = body + size;
            }
            samples.push(s);
        }
        for i in 0..samples.len() {
            let start = data_start + offsets[i];
            let end = offsets.get(i + 1).map_or(data_end, |o| data_start + o);
            ensure!(start <= end && end <= b.len(), "FSB5 sample data range");
            samples[i].data = start..end;
        }
        Ok(Self { mode, samples })
    }
}

/// A Vorbis setup packet of CRC-32 `crc` in `binary`: FMOD stores each
/// known packet whole, its size in the 32 bits before the CRC in its table.
pub fn find_vorbis_setup(binary: &[u8], crc: u32) -> Option<Vec<u8>> {
    let mut sizes = Vec::new();
    let needle = crc.to_le_bytes();
    let mut i = 0;
    while let Some(p) = find(&binary[i..], &needle) {
        let at = i + p;
        if at >= 4 {
            let size = u32::from_le_bytes(binary[at - 4..at].try_into().ok()?) as usize;
            if (32..=0x10000).contains(&size) && !sizes.contains(&size) {
                sizes.push(size);
            }
        }
        i = at + 1;
    }
    let mut i = 0;
    while let Some(p) = find(&binary[i..], b"\x05vorbis") {
        let at = i + p;
        for &size in &sizes {
            if let Some(packet) = binary.get(at..at + size) {
                if crc32(packet) == crc {
                    return Some(packet.to_vec());
                }
            }
        }
        i = at + 1;
    }
    None
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

/// CRC-32 (IEEE, reflected) as FMOD identifies setup headers.
pub fn crc32(data: &[u8]) -> u32 {
    let mut c = !0u32;
    for &b in data {
        c ^= u32::from(b);
        for _ in 0..8 {
            c = if c & 1 != 0 { (c >> 1) ^ 0xEDB8_8320 } else { c >> 1 };
        }
    }
    !c
}

/// Ogg page CRC (polynomial 0x04C11DB7, unreflected, zero initial value).
fn ogg_crc(data: &[u8]) -> u32 {
    let mut c = 0u32;
    for &b in data {
        c ^= u32::from(b) << 24;
        for _ in 0..8 {
            c = if c & 0x8000_0000 != 0 { (c << 1) ^ 0x04C1_1DB7 } else { c << 1 };
        }
    }
    c
}

struct OggWriter {
    out: Vec<u8>,
    serial: u32,
    sequence: u32,
}

impl OggWriter {
    /// One packet per page (packets are below the 255×255 lacing limit).
    fn page(&mut self, packet: &[u8], granule: u64, first: bool, last: bool) -> Result<()> {
        ensure!(packet.len() < 255 * 255, "Vorbis packet too large for one page");
        let mut lacing = vec![255u8; packet.len() / 255];
        lacing.push((packet.len() % 255) as u8);
        let start = self.out.len();
        self.out.extend_from_slice(b"OggS\0");
        self.out.push(u8::from(first) * 2 | u8::from(last) * 4);
        self.out.extend_from_slice(&granule.to_le_bytes());
        self.out.extend_from_slice(&self.serial.to_le_bytes());
        self.out.extend_from_slice(&self.sequence.to_le_bytes());
        self.out.extend_from_slice(&[0; 4]);
        self.out.push(lacing.len() as u8);
        self.out.extend_from_slice(&lacing);
        self.out.extend_from_slice(packet);
        let crc = ogg_crc(&self.out[start..]);
        self.out[start + 22..start + 26].copy_from_slice(&crc.to_le_bytes());
        self.sequence += 1;
        Ok(())
    }
}

/// FMOD's Vorbis block sizes (exponents): 256 and 2048 samples.
const BLOCKSIZES: (u8, u8) = (8, 11);

fn ident_header(channels: u8, rate: u32) -> Vec<u8> {
    let mut h = b"\x01vorbis".to_vec();
    h.extend_from_slice(&0u32.to_le_bytes());
    h.push(channels);
    h.extend_from_slice(&rate.to_le_bytes());
    h.extend_from_slice(&[0; 12]);
    h.push(BLOCKSIZES.0 | (BLOCKSIZES.1 << 4));
    h.push(1);
    h
}

fn comment_header() -> Vec<u8> {
    let vendor = b"CrySirror FSB5";
    let mut h = b"\x03vorbis".to_vec();
    h.extend_from_slice(&(vendor.len() as u32).to_le_bytes());
    h.extend_from_slice(vendor);
    h.extend_from_slice(&0u32.to_le_bytes());
    h.push(1);
    h
}

/// Ogg Vorbis stream of a Vorbis sample; `setup` must match the sample's
/// CRC-32.
pub fn vorbis_to_ogg(bank: &[u8], sample: &Sample, setup: &[u8]) -> Result<Vec<u8>> {
    let crc = sample.vorbis_crc.context("FSB5 sample has no Vorbis chunk")?;
    ensure!(crc32(setup) == crc, "Vorbis setup header CRC {:08x} != {crc:08x}", crc32(setup));
    let ident_bytes = ident_header(sample.channels, sample.rate);
    let ident = lewton::header::read_header_ident(&ident_bytes).map_err(|e| anyhow::anyhow!("Vorbis ident: {e:?}"))?;
    let setup_header = lewton::header::read_header_setup(setup, sample.channels, BLOCKSIZES).map_err(|e| anyhow::anyhow!("Vorbis setup: {e:?}"))?;
    let mut w = OggWriter { out: Vec::with_capacity(sample.data.len() + 8192), serial: crc, sequence: 0 };
    w.page(&ident_bytes, 0, true, false)?;
    w.page(&comment_header(), 0, false, false)?;
    w.page(setup, 0, false, false)?;
    let data = &bank[sample.data.clone()];
    let mut packets = Vec::new();
    let mut at = 0;
    while at + 2 <= data.len() {
        let size = u16::from_le_bytes([data[at], data[at + 1]]) as usize;
        if size == 0 || at + 2 + size > data.len() {
            break;
        }
        packets.push(&data[at + 2..at + 2 + size]);
        at += 2 + size;
    }
    let mut granule = 0u64;
    let mut first = true;
    for (i, p) in packets.iter().enumerate() {
        let n = lewton::audio::get_decoded_sample_count(&ident, &setup_header, p).map_err(|e| anyhow::anyhow!("Vorbis packet {i}: {e:?}"))? as u64;
        if !first {
            granule += n;
        }
        first = false;
        let last = i + 1 == packets.len();
        // The last page's granule trims the final block to the sample length.
        let g = if last && sample.frames > 0 { sample.frames } else { granule };
        w.page(p, g, false, last)?;
    }
    Ok(w.out)
}

fn wave(format: u16, channels: u16, rate: u32, block_align: u16, bits: u16, extra: &[u8], data: &[u8], byte_rate: u32) -> Vec<u8> {
    let fmt_len = 16 + if extra.is_empty() { 0 } else { 2 + extra.len() };
    let mut w = Vec::with_capacity(data.len() + 64);
    w.extend_from_slice(b"RIFF");
    w.extend_from_slice(&((4 + 8 + fmt_len + 8 + data.len()) as u32).to_le_bytes());
    w.extend_from_slice(b"WAVEfmt ");
    w.extend_from_slice(&(fmt_len as u32).to_le_bytes());
    w.extend_from_slice(&format.to_le_bytes());
    w.extend_from_slice(&channels.to_le_bytes());
    w.extend_from_slice(&rate.to_le_bytes());
    w.extend_from_slice(&byte_rate.to_le_bytes());
    w.extend_from_slice(&block_align.to_le_bytes());
    w.extend_from_slice(&bits.to_le_bytes());
    if !extra.is_empty() {
        w.extend_from_slice(&(extra.len() as u16).to_le_bytes());
        w.extend_from_slice(extra);
    }
    w.extend_from_slice(b"data");
    w.extend_from_slice(&(data.len() as u32).to_le_bytes());
    w.extend_from_slice(data);
    w
}

/// WAVE stream of a PCM16 or IMA ADPCM sample (FMOD's IMA blocks are 36
/// bytes per channel, 64 frames: the WAVE IMA ADPCM layout).
pub fn pcm_to_wave(bank: &[u8], mode: u32, sample: &Sample) -> Result<Vec<u8>> {
    let data = &bank[sample.data.clone()];
    let ch = u16::from(sample.channels);
    match mode {
        MODE_PCM16 => {
            let bytes = (sample.frames as usize * ch as usize * 2).min(data.len());
            Ok(wave(1, ch, sample.rate, ch * 2, 16, &[], &data[..bytes], sample.rate * u32::from(ch) * 2))
        }
        MODE_IMA_ADPCM => {
            let align = 36 * ch;
            let per_block = 64u32;
            let byte_rate = sample.rate * u32::from(align) / per_block;
            Ok(wave(0x11, ch, sample.rate, align, 4, &(per_block as u16).to_le_bytes(), data, byte_rate))
        }
        m => bail!("FSB5 mode {m} is not PCM16/IMA ADPCM"),
    }
}

/// The bank's first sample as a stream the shared decoders read.
pub fn to_stream(bank: &[u8], setup: &dyn Fn(u32) -> Option<Vec<u8>>) -> Result<Vec<u8>> {
    let b = Bank::parse(bank)?;
    let s = b.samples.first().context("FSB5 bank without samples")?;
    match b.mode {
        MODE_VORBIS => {
            let crc = s.vorbis_crc.context("FSB5 Vorbis sample without CRC")?;
            let packet = setup(crc).with_context(|| format!("Vorbis setup header {crc:08x} not found"))?;
            vorbis_to_ogg(bank, s, &packet)
        }
        m => pcm_to_wave(bank, m, s),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crcs() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(ogg_crc(b"123456789"), 0x89A1_897F);
    }

    #[test]
    fn pcm16_bank() {
        // Version 1 header, one mono 44.1 kHz sample of 2 frames, PCM16.
        let mut b = b"FSB5".to_vec();
        for v in [1u32, 1, 8, 0, 4, MODE_PCM16] {
            b.extend_from_slice(&v.to_le_bytes());
        }
        b.resize(0x3C, 0);
        let bits: u64 = (8 << 1) | (2u64 << 34);
        b.extend_from_slice(&bits.to_le_bytes());
        b.extend_from_slice(&[1, 0, 2, 0]);
        let bank = Bank::parse(&b).unwrap();
        assert_eq!(bank.samples[0].rate, 44100);
        assert_eq!(bank.samples[0].frames, 2);
        let w = pcm_to_wave(&b, bank.mode, &bank.samples[0]).unwrap();
        assert_eq!(&w[..4], b"RIFF");
        assert_eq!(&w[w.len() - 4..], &[1, 0, 2, 0]);
    }
}
