// SPDX-License-Identifier: MPL-2.0
//! Ring-dictionary LZSS used by many legacy archive formats. Engine-owned so
//! any VM or archive plugin can request it through `TransformSpec`.
use crate::{ByteTransform, DecodeContext, TransformSpec, read_all};
use anyhow::{Context, Result, ensure};
use crysirror_core::service::AssetReader;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// `bytewise`: 8-flag control byte (LSB first, 1 = literal) and two-byte
/// references. The 12/4 layout zero-fills short output for legacy padded archives.
/// The 11/5 and 10/6 layouts require exact input and output lengths.
/// `bitwise`: MSB-first bitstream with configurable position and length widths.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LzssMode {
    Bytewise,
    Bitwise,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LzssParams {
    /// Byte-control or MSB-first bitstream layout.
    pub mode: LzssMode,
    /// Initial ring write position, wrapped to dictionary size.
    pub initial: usize,
    /// Dictionary position width. Bytewise supports 12, 11 and 10.
    pub position_bits: u32,
    /// Match length width. Bytewise uses the remaining 4, 5 or 6 bits.
    pub size_bits: u32,
    /// Minimum match length. Bytewise uses 3 for 12/4 and 2 for 11/5 or 10/6.
    pub min_match: usize,
    /// Bytewise 12/4 only: the 4-bit length field is stored inverted (GLib).
    pub invert_length: bool,
}
impl Default for LzssParams {
    fn default() -> Self {
        Self {
            mode: LzssMode::Bytewise,
            initial: 0xFEE,
            position_bits: 12,
            size_bits: 4,
            min_match: 3,
            invert_length: false,
        }
    }
}
pub fn lzss_decompress(input: &[u8], size: usize, params: &LzssParams) -> Result<Vec<u8>> {
    match params.mode {
        LzssMode::Bytewise => bytewise(input, size, params, bytewise_strict(params)?),
        LzssMode::Bitwise => bitwise(input, size, params),
    }
}

fn bytewise_strict(params: &LzssParams) -> Result<bool> {
    match (params.position_bits, params.size_bits, params.min_match) {
        (12, 4, 3) => Ok(false),
        (11, 5, 2) | (10, 6, 2) if !params.invert_length => Ok(true),
        _ => anyhow::bail!(
            "unsupported bytewise LZSS parameters (expected 12/4/3, 11/5/2 or 10/6/2)"
        ),
    }
}

/// Append a bytewise LZSS stream to an output buffer. Matches can overlap
/// and the initial dictionary is zero-filled. Output grows by at most
/// `input.len() + input.len().div_ceil(8)`, with bounded match-search scratch.
/// Failure or cancellation preserves the caller's existing output bytes.
pub fn lzss_compress_bytewise(
    input: &[u8],
    params: &LzssParams,
    output: &mut Vec<u8>,
    check_cancel: impl Fn() -> Result<()>,
) -> Result<()> {
    ensure!(params.mode == LzssMode::Bytewise, "expected bytewise LZSS");
    bytewise_strict(params)?;
    check_cancel()?;
    if input.is_empty() {
        return Ok(());
    }
    let maximum = input
        .len()
        .checked_add(input.len().div_ceil(8))
        .context("bytewise LZSS packed size overflow")?;
    output.try_reserve_exact(maximum)?;
    let start = output.len();
    if let Err(error) = encode_bytewise(input, params, output, &check_cancel) {
        output.truncate(start);
        return Err(error);
    }
    Ok(())
}

const MATCH_BUCKETS: usize = 4096;
const NO_MATCH: usize = usize::MAX;

struct MatchWindow<'a> {
    input: &'a [u8],
    ring: usize,
    newest: [usize; MATCH_BUCKETS],
    older: [usize; 4096],
}

impl MatchWindow<'_> {
    fn byte(&self, position: usize) -> u8 {
        position
            .checked_sub(self.ring)
            .and_then(|index| self.input.get(index))
            .copied()
            .unwrap_or_default()
    }

    fn bucket(&self, position: usize) -> usize {
        let left = usize::from(self.byte(position));
        let right = usize::from(self.byte(position + 1));
        (left * 257 ^ right) & (MATCH_BUCKETS - 1)
    }

    fn insert(&mut self, position: usize) {
        let bucket = self.bucket(position);
        self.older[position & (self.ring - 1)] = self.newest[bucket];
        self.newest[bucket] = position;
    }

    fn longest(&self, position: usize, target: &[u8], minimum: usize) -> (usize, usize) {
        let mut candidate = self.newest[self.bucket(position)];
        let oldest = position - self.ring;
        let (mut source, mut length) = (0, minimum - 1);
        // The match window can hold thousands of colliding pairs. Capping
        // their search keeps work bounded while preserving stream validity.
        for _ in 0..64 {
            if candidate == NO_MATCH || candidate < oldest {
                break;
            }
            let matched = target
                .iter()
                .enumerate()
                .take_while(|&(offset, &byte)| self.byte(candidate + offset) == byte)
                .count();
            if matched > length {
                source = candidate;
                length = matched;
                if length == target.len() {
                    break;
                }
            }
            candidate = self.older[candidate & (self.ring - 1)];
        }
        (source, length)
    }
}

fn encode_bytewise(
    input: &[u8],
    params: &LzssParams,
    output: &mut Vec<u8>,
    check_cancel: &dyn Fn() -> Result<()>,
) -> Result<()> {
    let ring = 1 << params.position_bits;
    let mask = ring - 1;
    let length_mask = (1 << params.size_bits) - 1;
    let maximum_match = params.min_match + length_mask;
    let initial = params.initial & mask;
    let mut dictionary = MatchWindow {
        input,
        ring,
        newest: [NO_MATCH; MATCH_BUCKETS],
        older: [NO_MATCH; 4096],
    };
    for position in 0..ring {
        dictionary.insert(position);
    }
    let mut cursor = 0;
    let mut next_check = 0;
    while cursor < input.len() {
        if cursor >= next_check {
            check_cancel()?;
            next_check = cursor.saturating_add(4096);
        }
        let flags = output.len();
        output.push(0);
        for bit in 0..8 {
            let remaining = &input[cursor..];
            if remaining.is_empty() {
                break;
            }
            let position = ring + cursor;
            let target = &remaining[..remaining.len().min(maximum_match)];
            let (source, length) = dictionary.longest(position, target, params.min_match);
            let advance = if length >= params.min_match {
                let source = (initial + (source & mask)) & mask;
                let mut encoded_length = length - params.min_match;
                if params.invert_length {
                    encoded_length ^= length_mask;
                }
                output.push(source as u8);
                output.push(((source >> 8) << params.size_bits | encoded_length) as u8);
                length
            } else {
                output[flags] |= 1 << bit;
                output.push(remaining[0]);
                1
            };
            for index in position..position + advance {
                dictionary.insert(index);
            }
            cursor += advance;
        }
    }
    check_cancel()
}
fn bytewise(input: &[u8], size: usize, p: &LzssParams, strict: bool) -> Result<Vec<u8>> {
    let mut dict = [0u8; 0x1000];
    let ring_mask = (1 << p.position_bits) - 1;
    let length_mask = (1 << p.size_bits) - 1;
    let mut pos = p.initial & ring_mask;
    let mut out = Vec::new();
    out.try_reserve_exact(size)?;
    out.resize(size, 0);
    let (mut at, mut ptr) = (0usize, 0usize);
    let mut control = 0u16;
    while at < size {
        control >>= 1;
        if control & 0x100 == 0 {
            let Some(&b) = input.get(ptr) else { break };
            ptr += 1;
            control = u16::from(b) | 0xFF00;
        }
        if control & 1 != 0 {
            let Some(&b) = input.get(ptr) else { break };
            ptr += 1;
            out[at] = b;
            at += 1;
            dict[pos] = b;
            pos = (pos + 1) & ring_mask;
        } else {
            if input.len() < ptr + 2 {
                break;
            }
            let (lo, hi) = (usize::from(input[ptr]), usize::from(input[ptr + 1]));
            ptr += 2;
            let mut src = lo | ((hi >> p.size_bits) << 8);
            let length_bits = if p.invert_length { !hi } else { hi };
            let count = (length_bits & length_mask) + p.min_match;
            ensure!(
                !strict || count <= size - at,
                "LZSS match exceeds output size"
            );
            for _ in 0..count.min(size - at) {
                let b = dict[src];
                src = (src + 1) & ring_mask;
                out[at] = b;
                at += 1;
                dict[pos] = b;
                pos = (pos + 1) & ring_mask;
            }
        }
    }
    ensure!(!strict || at == size, "truncated LZSS stream");
    ensure!(
        !strict || ptr == input.len(),
        "trailing data in LZSS stream"
    );
    Ok(out)
}
fn bitwise(input: &[u8], size: usize, p: &LzssParams) -> Result<Vec<u8>> {
    ensure!(
        (1..=16).contains(&p.position_bits) && (1..=16).contains(&p.size_bits),
        "LZSS bit widths must be 1..=16"
    );
    ensure!(
        !p.invert_length,
        "invert_length applies only to bytewise LZSS"
    );
    ensure!(
        p.min_match <= usize::MAX - (1usize << p.size_bits),
        "LZSS match length overflow"
    );
    let mask = (1usize << p.position_bits) - 1;
    let mut dict = vec![0u8; mask + 1];
    let mut pos = p.initial & mask;
    let mut out = vec![0u8; size];
    let mut at = 0usize;
    let mut bit = 0usize;
    let total = input.len().saturating_mul(8);
    let mut get = |n: u32| -> Option<usize> {
        if bit + n as usize > total {
            return None;
        }
        let mut v = 0usize;
        for _ in 0..n {
            v = (v << 1) | usize::from((input[bit / 8] >> (7 - bit % 8)) & 1);
            bit += 1;
        }
        Some(v)
    };
    while at < size {
        let Some(flag) = get(1) else { break };
        if flag != 0 {
            let Some(b) = get(8) else { break };
            out[at] = b as u8;
            at += 1;
            dict[pos] = b as u8;
            pos = (pos + 1) & mask;
        } else {
            let (Some(mut src), Some(n)) = (get(p.position_bits), get(p.size_bits)) else {
                break;
            };
            for _ in 0..(n + p.min_match).min(size - at) {
                let b = dict[src];
                src = (src + 1) & mask;
                out[at] = b;
                at += 1;
                dict[pos] = b;
                pos = (pos + 1) & mask;
            }
        }
    }
    Ok(out)
}
pub(crate) struct Lzss;
impl ByteTransform for Lzss {
    fn apply(
        &self,
        spec: &TransformSpec,
        input: Arc<dyn AssetReader>,
        _: &serde_json::Value,
        context: &DecodeContext,
    ) -> Result<Arc<dyn AssetReader>> {
        ensure!(spec.key_id.is_none(), "lzss takes no key");
        let params: LzssParams = if spec.parameters.is_null() {
            LzssParams::default()
        } else {
            serde_json::from_value(spec.parameters.clone()).context("lzss parameters")?
        };
        let size = spec.expected_size.context("lzss requires expected_size")?;
        let data = read_all(input.as_ref(), context.limits.max_resident_bytes)?;
        context.decode_with(size, || lzss_decompress(&data, size as usize, &params))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytewise_encoder_handles_initial_history_overlap_wrap_and_inverted_lengths() {
        let mut raw = vec![0; 85];
        raw.extend((0..10_000u32).map(|index| (index.wrapping_mul(0x0101_916d) >> 13) as u8));
        raw.extend(b"ABCBBC".repeat(1400));
        raw.extend_from_slice(b"Z");
        for (position_bits, size_bits, min_match, invert_length) in
            [(12, 4, 3, true), (11, 5, 2, false), (10, 6, 2, false)]
        {
            let params = LzssParams {
                mode: LzssMode::Bytewise,
                initial: (1 << position_bits) - 3,
                position_bits,
                size_bits,
                min_match,
                invert_length,
            };
            let mut packed = vec![0x55];
            lzss_compress_bytewise(b"abcdefghi", &params, &mut packed, || Ok(())).unwrap();
            assert_eq!(packed, b"\x55\xffabcdefgh\x01i");
            let start = packed.len();
            lzss_compress_bytewise(&raw, &params, &mut packed, || Ok(())).unwrap();
            let encoded = &packed[start..];
            assert_eq!(encoded[0] & 1, 0);
            assert!(encoded.len() < raw.len() / 2);
            assert_eq!(lzss_decompress(encoded, raw.len(), &params).unwrap(), raw);
        }
    }

    #[test]
    fn bytewise_encoder_rolls_back_appended_bytes_on_cancellation() {
        let mut output = vec![0xaa];
        let checks = std::cell::Cell::new(0);
        let result =
            lzss_compress_bytewise(&[3; 9000], &LzssParams::default(), &mut output, || {
                checks.set(checks.get() + 1);
                ensure!(checks.get() < 3, "cancelled");
                Ok(())
            });
        assert!(result.is_err());
        assert_eq!(output, [0xaa]);
        lzss_compress_bytewise(&[], &LzssParams::default(), &mut output, || Ok(())).unwrap();
        assert_eq!(output, [0xaa]);
    }

    #[test]
    fn bytewise_11_5_requires_a_complete_stream_and_allows_overlapping_matches() {
        let params = LzssParams {
            initial: 0x7df,
            position_bits: 11,
            size_bits: 5,
            min_match: 2,
            ..LzssParams::default()
        };
        let packed = [1, b'A', 0xdf, 0xe1];
        assert_eq!(lzss_decompress(&packed, 4, &params).unwrap(), b"AAAA");
        assert!(lzss_decompress(&packed[..3], 4, &params).is_err());
        assert!(lzss_decompress(&packed, 3, &params).is_err());
        assert!(lzss_decompress(&packed, 5, &params).is_err());
        assert!(lzss_decompress(&[1, b'A', 0xdf, 0xe1, 0], 4, &params).is_err());
    }

    #[test]
    fn overlapping_matches_obey_both_native_layouts_and_reject_inapplicable_options() {
        let bytewise = LzssParams::default();
        assert_eq!(
            lzss_decompress(&[1, b'A', 0xee, 0xf0], 4, &bytewise).unwrap(),
            b"AAAA"
        );
        let bits = "1010000010111011110001";
        let mut packed = vec![0; bits.len().div_ceil(8)];
        for (i, b) in bits.bytes().enumerate() {
            packed[i / 8] |= (b - b'0') << (7 - i % 8);
        }
        let bitwise = LzssParams {
            mode: LzssMode::Bitwise,
            position_bits: 8,
            size_bits: 4,
            min_match: 2,
            initial: 239,
            ..bytewise
        };
        assert_eq!(lzss_decompress(&packed, 4, &bitwise).unwrap(), b"AAAA");
        assert!(
            lzss_decompress(
                &[],
                1,
                &LzssParams {
                    min_match: 2,
                    ..bytewise
                }
            )
            .is_err()
        );
        assert!(
            lzss_decompress(
                &[],
                1,
                &LzssParams {
                    invert_length: true,
                    ..bitwise
                }
            )
            .is_err()
        );
        assert!(
            lzss_decompress(
                &[],
                1,
                &LzssParams {
                    min_match: usize::MAX,
                    ..bitwise
                }
            )
            .is_err()
        );
    }
}
