// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! WCG ("WG") 32-bit image encoding and the percentage point-sampling
//! shrink codeX RScript uses for save thumbnails (recovered from the
//! Shapeshifter build: writer 0x471F20, plane split 0x471ED0, plane coder
//! 0x471DA0, shrink 0x462870).
//!
//! A WCG file is a 16-byte header (`WG`, version 0x271, depth 32, an unused
//! word, width, height) and two planes of 16-bit pixel words: first the high
//! words (red and inverted alpha), then the low words (blue and green). Each
//! plane is `u32 decoded bytes, u32 coded bytes, u16 table count, u16 0`, the
//! little-endian word table and an MSB-first bit stream of table indices. An
//! index is coded as a width field (3 bits, 4 when the table has 4096 or more
//! words): 1 means one more bit selects index 0 or 1; `w + 1` means the index
//! lies in `2^w .. 2^(w+1)` and its low `w` bits follow, with the width
//! extended by a unary run once `w` reaches the escape width (6 or 14). A
//! zero width field introduces a repetition: 4 bits give the count minus two,
//! then the index follows. Any table order decodes; this encoder sorts words
//! by frequency so frequent words take short codes.
use anyhow::{Result, bail, ensure};

/// Header version written by the native encoder.
pub const WCG_VERSION: u16 = 0x0271;

struct BitWriter {
    bytes: Vec<u8>,
    bits: usize,
}
impl BitWriter {
    fn put(&mut self, value: u32, n: usize) {
        for i in (0..n).rev() {
            if self.bits % 8 == 0 {
                self.bytes.push(0);
            }
            if (value >> i) & 1 != 0 {
                *self.bytes.last_mut().unwrap() |= 0x80 >> (self.bits % 8);
            }
            self.bits += 1;
        }
    }
}

fn plane(words: &[u16], out: &mut Vec<u8>) -> Result<()> {
    let mut freq = vec![0u32; 1 << 16];
    for &w in words {
        freq[usize::from(w)] += 1;
    }
    let mut table: Vec<u16> = (0..=u16::MAX).filter(|&w| freq[usize::from(w)] != 0).collect();
    ensure!(table.len() <= usize::from(u16::MAX), "WCG plane uses all 65536 words; the table count does not fit");
    table.sort_by(|a, b| freq[usize::from(*b)].cmp(&freq[usize::from(*a)]).then(a.cmp(b)));
    let mut index = vec![0u32; 1 << 16];
    for (i, &w) in table.iter().enumerate() {
        index[usize::from(w)] = i as u32;
    }
    let (prefix, escape) = if table.len() < 4096 { (3, 6) } else { (4, 14) };
    let mut bits = BitWriter { bytes: Vec::new(), bits: 0 };
    let code = |bits: &mut BitWriter, i: u32| {
        if i < 2 {
            bits.put(1, prefix);
            bits.put(i, 1);
            return;
        }
        let w = 31 - i.leading_zeros() as usize;
        if w < escape {
            bits.put(w as u32 + 1, prefix);
        } else {
            bits.put(escape as u32 + 1, prefix);
            for _ in escape..w {
                bits.put(1, 1);
            }
            bits.put(0, 1);
        }
        bits.put(i & ((1 << w) - 1), w);
    };
    let mut at = 0;
    while at < words.len() {
        let w = words[at];
        let run = words[at..].iter().take(17).take_while(|&&x| x == w).count();
        if run >= 2 {
            bits.put(0, prefix);
            bits.put(run as u32 - 2, 4);
        }
        code(&mut bits, index[usize::from(w)]);
        at += run;
    }
    out.extend(((words.len() * 2) as u32).to_le_bytes());
    out.extend((bits.bytes.len() as u32).to_le_bytes());
    out.extend((table.len() as u16).to_le_bytes());
    out.extend([0, 0]);
    for w in &table {
        out.extend(w.to_le_bytes());
    }
    out.extend(bits.bytes);
    Ok(())
}

/// Encodes straight RGBA rows as a WCG file.
pub fn encode_wcg(width: u32, height: u32, rgba: &[u8]) -> Result<Vec<u8>> {
    let n = (width as usize).checked_mul(height as usize).filter(|&n| n > 0);
    let Some(n) = n else { bail!("WCG image {width}x{height} is empty or too large") };
    ensure!(rgba.len() == n * 4, "WCG source has {} bytes, expected {}", rgba.len(), n * 4);
    let mut out = Vec::with_capacity(16 + n * 2);
    out.extend(b"WG");
    out.extend(WCG_VERSION.to_le_bytes());
    out.extend(32u16.to_le_bytes());
    out.extend(0u16.to_le_bytes());
    out.extend(width.to_le_bytes());
    out.extend(height.to_le_bytes());
    let high: Vec<u16> = rgba.chunks_exact(4).map(|p| u16::from(p[0]) | u16::from(p[3] ^ 0xFF) << 8).collect();
    let low: Vec<u16> = rgba.chunks_exact(4).map(|p| u16::from(p[2]) | u16::from(p[1]) << 8).collect();
    plane(&high, &mut out)?;
    plane(&low, &mut out)?;
    Ok(out)
}

/// Point-sampled shrink to `percent_x`/`percent_y` of the source size
/// (0 counts as 1, 100/100 copies). Source columns and rows advance in
/// 16.16 steps of `0x640000 / percent`, and the integer part is taken by
/// dividing by 0xFFFF as the native routine does.
pub fn percent_shrink(width: u32, height: u32, rgba: &[u8], percent_x: u16, percent_y: u16) -> Result<(u32, u32, Vec<u8>)> {
    ensure!(rgba.len() == width as usize * height as usize * 4, "shrink source size does not match {width}x{height}");
    if percent_x == 100 && percent_y == 100 {
        return Ok((width, height, rgba.to_vec()));
    }
    let (px, py) = (u32::from(percent_x.max(1)), u32::from(percent_y.max(1)));
    let (w, h) = (px * width / 100, py * height / 100);
    let (sx, sy) = (0x64_0000 / px, 0x64_0000 / py);
    let mut out = Vec::with_capacity(w as usize * h as usize * 4);
    let mut ay = 0u32;
    for _ in 0..h {
        let row = (ay / 0xFFFF).min(height.saturating_sub(1));
        let mut ax = 0u32;
        for _ in 0..w {
            let col = (ax / 0xFFFF).min(width.saturating_sub(1));
            let at = (row * width + col) as usize * 4;
            out.extend_from_slice(&rgba[at..at + 4]);
            ax = ax.wrapping_add(sx);
        }
        ay = ay.wrapping_add(sy);
    }
    Ok((w, h, out))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode_plane(b: &[u8], at: &mut usize, pixels: usize) -> Vec<u16> {
        let rd32 = |o: usize| u32::from_le_bytes(b[o..o + 4].try_into().unwrap()) as usize;
        let (size, comp, count) = (rd32(*at), rd32(*at + 4), usize::from(u16::from_le_bytes([b[*at + 8], b[*at + 9]])));
        assert_eq!(size, pixels * 2);
        *at += 12;
        let table: Vec<u16> = (0..count).map(|i| u16::from_le_bytes([b[*at + 2 * i], b[*at + 2 * i + 1]])).collect();
        *at += count * 2;
        let data = &b[*at..*at + comp];
        *at += comp;
        let mut pos = 0;
        let mut get = |n: usize| {
            let mut v = 0usize;
            for _ in 0..n {
                v = v << 1 | usize::from((data[pos / 8] >> (7 - pos % 8)) & 1);
                pos += 1;
            }
            v
        };
        let (escape, prefix) = if count < 4096 { (6, 3) } else { (14, 4) };
        let mut out = Vec::new();
        while out.len() < pixels {
            let mut seq = 1;
            let mut width = get(prefix);
            if width == 0 {
                seq = get(4) + 2;
                width = get(prefix);
            }
            let index = if width == 1 {
                get(1)
            } else {
                width -= 1;
                if width >= escape {
                    while get(1) != 0 {
                        width += 1;
                    }
                }
                (1 << width) | get(width)
            };
            for _ in 0..seq {
                out.push(table[index]);
            }
        }
        out
    }

    #[test]
    fn encoded_planes_decode_to_the_source() {
        let (w, h) = (37u32, 23u32);
        let mut rgba = Vec::new();
        for i in 0..w * h {
            // Runs, rare words and enough distinct words for long codes.
            let v = if i % 5 < 3 { 7 } else { (i * 2654435761u32 >> 7) as u8 };
            rgba.extend([v, v.wrapping_mul(3), (i % 251) as u8, if i % 9 == 0 { 0 } else { 255 }]);
        }
        let b = encode_wcg(w, h, &rgba).unwrap();
        assert_eq!(&b[..2], b"WG");
        assert_eq!(u16::from_le_bytes([b[2], b[3]]), WCG_VERSION);
        let mut at = 16;
        let n = (w * h) as usize;
        let high = decode_plane(&b, &mut at, n);
        let low = decode_plane(&b, &mut at, n);
        assert_eq!(at, b.len());
        for (i, p) in rgba.chunks_exact(4).enumerate() {
            assert_eq!(high[i], u16::from(p[0]) | u16::from(p[3] ^ 0xFF) << 8);
            assert_eq!(low[i], u16::from(p[2]) | u16::from(p[1]) << 8);
        }
    }

    #[test]
    fn shrink_follows_the_native_steps() {
        let (w, h) = (200u32, 100u32);
        let rgba: Vec<u8> = (0..w * h).flat_map(|i| [(i % w) as u8, (i / w) as u8, 0, 255]).collect();
        let (sw, sh, out) = percent_shrink(w, h, &rgba, 12, 25).unwrap();
        assert_eq!((sw, sh), (24, 25));
        // Column 1 starts at 0x640000 / 12 / 0xFFFF = 8.
        assert_eq!(out[4], 8);
        assert_eq!(out[(sw as usize) * 4 + 1], 4);
    }
}
