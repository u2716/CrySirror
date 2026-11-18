// SPDX-License-Identifier: MPL-2.0
//! Bounded NPSF resource packets. Caller-owned substitution tables are data;
//! native filename keys and save payload semantics stay with their adapter.
use anyhow::{Context, Result, ensure};
use std::io::Read;

fn validate(table: &[u8; 256]) -> Result<()> {
    let mut seen = [false; 256];
    for &v in table {
        ensure!(!seen[v as usize], "NPSF substitution is not a permutation");
        seen[v as usize] = true;
    }
    Ok(())
}
/// Recover D from a caller-owned effective table E(x)=255-D(D(D(x))).
/// Return only the mathematically unique root: cycles divisible by three, or
/// three equal-length cycles which could be joined, are ambiguous and fail.
/// No candidate search, heuristic key selection or guessed material occurs.
pub fn unique_complement_cube_root(effective: &[u8; 256]) -> Result<[u8; 256]> {
    validate(effective)?;
    let cube = effective.map(|v| 255 - v);
    let mut seen = [false; 256];
    let mut cycles = Vec::new();
    let mut counts = [0u16; 257];
    for start in 0..256 {
        if seen[start] {
            continue;
        }
        let mut cycle = Vec::new();
        let mut at = start;
        while !seen[at] {
            seen[at] = true;
            cycle.push(at);
            at = cube[at] as usize;
        }
        ensure!(at == start, "NPSF permutation cycle");
        counts[cycle.len()] += 1;
        cycles.push(cycle);
    }
    ensure!(
        counts
            .iter()
            .enumerate()
            .all(|(n, c)| *c == 0 || (n % 3 != 0 && *c < 3)),
        "NPSF inverse table is ambiguous; supply its own caller-owned permutation"
    );
    let mut root = [0u8; 256];
    for cycle in cycles {
        let n = cycle.len();
        let power = if n == 1 {
            0
        } else {
            (1..n)
                .find(|v| v * 3 % n == 1)
                .context("NPSF permutation cube inverse")?
        };
        for (i, &at) in cycle.iter().enumerate() {
            root[at] = cycle[(i + power) % n] as u8;
        }
    }
    ensure!(
        (0..256).all(|i| 255 - root[root[root[i] as usize] as usize] == effective[i]),
        "NPSF inverse table verification"
    );
    Ok(root)
}
/// Decode a complete packet. A mask count of one means the first4096 bytes;
/// other nonzero counts are explicit. A zero mask needs no permutation.
pub fn decode(
    bytes: &[u8],
    shift: u8,
    inverse: Option<&[u8; 256]>,
    limit: usize,
    check: &dyn Fn() -> Result<()>,
) -> Result<Vec<u8>> {
    ensure!(
        bytes.len() >= 16 && bytes.len() - 16 <= limit && &bytes[..4] == b"NPSF",
        "NPSF packet extent/signature"
    );
    let plain = u32::from_le_bytes(bytes[4..8].try_into()?) as usize;
    let mask = u32::from_le_bytes(bytes[8..12].try_into()?) as usize;
    let compressed = u32::from_le_bytes(bytes[12..16].try_into()?);
    ensure!(
        plain <= limit && compressed <= 1,
        "NPSF output/compression profile"
    );
    let count = if mask == 1 {
        (bytes.len() - 16).min(4096)
    } else {
        mask
    };
    ensure!(count <= bytes.len() - 16, "NPSF masked extent");
    check()?;
    let mut input = bytes[16..].to_vec();
    if count != 0 {
        let table = inverse.context("NPSF needs caller-owned inverse permutation")?;
        validate(table)?;
        for chunk in input[..count].chunks_mut(4096) {
            check()?;
            for byte in chunk {
                *byte = table[*byte as usize].wrapping_sub(shift);
            }
        }
    }
    if compressed == 0 {
        ensure!(input.len() == plain, "NPSF plain extent");
        check()?;
        return Ok(input);
    }
    let mut decoder = flate2::read::ZlibDecoder::new(input.as_slice());
    let mut output = Vec::new();
    output
        .try_reserve_exact(plain)
        .context("NPSF output allocation")?;
    let mut buffer = [0u8; 8192];
    loop {
        check()?;
        let count = decoder.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        ensure!(
            count <= plain.saturating_sub(output.len()),
            "NPSF expanded extent"
        );
        output.extend_from_slice(&buffer[..count]);
    }
    ensure!(output.len() == plain, "NPSF truncated expansion");
    check()?;
    Ok(output)
}
/// Original uncompressed writer profile. Invert the admitted decode table;
/// no game-specific table is embedded or distributed in this implementation.
pub fn encode(
    bytes: &[u8],
    shift: u8,
    inverse: Option<&[u8; 256]>,
    limit: usize,
    check: &dyn Fn() -> Result<()>,
) -> Result<Vec<u8>> {
    ensure!(
        !bytes.is_empty() && bytes.len() <= limit && bytes.len() <= u32::MAX as usize,
        "NPSF input extent"
    );
    let mut forward = [0u8; 256];
    if let Some(table) = inverse {
        validate(table)?;
        for (i, &v) in table.iter().enumerate() {
            forward[v as usize] = i as u8;
        }
    }
    let size = bytes
        .len()
        .checked_add(16)
        .context("NPSF packet size overflow")?;
    let mut output = Vec::new();
    output
        .try_reserve_exact(size)
        .context("NPSF packet allocation")?;
    output.extend_from_slice(b"NPSF");
    output.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    output.extend_from_slice(
        &(if inverse.is_some() {
            bytes.len() as u32
        } else {
            0
        })
        .to_le_bytes(),
    );
    output.extend_from_slice(&0u32.to_le_bytes());
    for chunk in bytes.chunks(4096) {
        check()?;
        if inverse.is_some() {
            output.extend(
                chunk
                    .iter()
                    .map(|v| forward[v.wrapping_add(shift) as usize]),
            );
        } else {
            output.extend_from_slice(chunk);
        }
    }
    check()?;
    Ok(output)
}
