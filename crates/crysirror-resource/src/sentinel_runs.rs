// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Bounded little-endian sentinel runs. Only the selected marker is packed;
//! other words are literals. A marker is followed by a positive same-width
//! count. These kernels own byte expansion independently of a file/VM format.
use anyhow::{Result, ensure};
fn word(bytes: &[u8]) -> u32 {
    if bytes.len() == 1 {
        u32::from(bytes[0])
    } else {
        u32::from_le_bytes(bytes.try_into().unwrap())
    }
}
fn put(out: &mut Vec<u8>, value: u32, width: usize) {
    out.extend_from_slice(&value.to_le_bytes()[..width]);
}
fn encode(
    input: &[u8],
    width: usize,
    marker: u32,
    check: &dyn Fn() -> Result<()>,
) -> Result<Vec<u8>> {
    ensure!(
        input.len() % width == 0 && input.len() <= 64 << 20,
        "sentinel run input extent"
    );
    let mut out = Vec::new();
    let mut offset = 0;
    let max = if width == 1 {
        255usize
    } else {
        u32::MAX as usize
    };
    while offset < input.len() {
        check()?;
        let value = word(&input[offset..offset + width]);
        offset += width;
        if value != marker {
            put(&mut out, value, width);
            continue;
        }
        let mut count = 1usize;
        while count < max && offset < input.len() && word(&input[offset..offset + width]) == marker
        {
            offset += width;
            count += 1;
            if count % 4096 == 0 {
                check()?;
            }
        }
        put(&mut out, marker, width);
        put(&mut out, count as u32, width);
    }
    Ok(out)
}
fn decode(
    input: &[u8],
    width: usize,
    marker: u32,
    elements: usize,
    limit: usize,
    check: &dyn Fn() -> Result<()>,
) -> Result<Vec<u8>> {
    let bytes = elements
        .checked_mul(width)
        .ok_or_else(|| anyhow::anyhow!("sentinel run output overflow"))?;
    ensure!(
        bytes <= limit && bytes <= 64 << 20 && input.len() % width == 0,
        "sentinel run output/input budget"
    );
    let mut out = Vec::with_capacity(bytes);
    let mut offset = 0;
    while offset < input.len() {
        check()?;
        let value = word(&input[offset..offset + width]);
        offset += width;
        let count = if value == marker {
            ensure!(offset + width <= input.len(), "sentinel run missing count");
            let count = word(&input[offset..offset + width]) as usize;
            offset += width;
            ensure!(count > 0, "sentinel run zero count");
            count
        } else {
            1
        };
        let count_bytes = count
            .checked_mul(width)
            .ok_or_else(|| anyhow::anyhow!("sentinel run count overflow"))?;
        ensure!(
            count_bytes <= bytes.saturating_sub(out.len()),
            "sentinel run exceeds declared output"
        );
        for n in 0..count {
            if n % 4096 == 0 {
                check()?;
            }
            put(&mut out, value, width);
        }
    }
    ensure!(out.len() == bytes, "sentinel run incomplete output");
    Ok(out)
}
pub fn encode_bytes(input: &[u8], marker: u8, check: &dyn Fn() -> Result<()>) -> Result<Vec<u8>> {
    encode(input, 1, u32::from(marker), check)
}
pub fn decode_bytes(
    input: &[u8],
    marker: u8,
    elements: usize,
    limit: usize,
    check: &dyn Fn() -> Result<()>,
) -> Result<Vec<u8>> {
    decode(input, 1, u32::from(marker), elements, limit, check)
}
pub fn encode_words(input: &[u8], marker: u32, check: &dyn Fn() -> Result<()>) -> Result<Vec<u8>> {
    encode(input, 4, marker, check)
}
pub fn decode_words(
    input: &[u8],
    marker: u32,
    elements: usize,
    limit: usize,
    check: &dyn Fn() -> Result<()>,
) -> Result<Vec<u8>> {
    decode(input, 4, marker, elements, limit, check)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sentinel_runs_split_byte_counts_and_check_corrupt_word_extents() {
        let mut raw = vec![0; 256];
        raw.extend([3, 0, 4]);
        let packed = encode_bytes(&raw, 0, &|| Ok(())).unwrap();
        assert_eq!(packed, [0, 255, 0, 1, 3, 0, 1, 4]);
        assert_eq!(decode_bytes(&packed, 0, 259, 259, &|| Ok(())).unwrap(), raw);
        let raw: Vec<_> = [u32::MAX, u32::MAX, 0, 7, u32::MAX]
            .into_iter()
            .flat_map(u32::to_le_bytes)
            .collect();
        let packed = encode_words(&raw, u32::MAX, &|| Ok(())).unwrap();
        assert_eq!(
            decode_words(&packed, u32::MAX, 5, 20, &|| Ok(())).unwrap(),
            raw
        );
        for invalid in [vec![0, 0], vec![0], vec![0, 3], vec![1, 2, 3]] {
            assert!(decode_bytes(&invalid, 0, 2, 2, &|| Ok(())).is_err());
        }
        assert!(decode_words(&[255; 8], u32::MAX, 5, 20, &|| Ok(())).is_err());
    }
}
