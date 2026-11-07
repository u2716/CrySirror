//! Bounded byte-pair dictionary streams. Each block supplies a 256-symbol
//! replacement table, followed by tokens expanded using a bounded stack.
//! The 0xff435031 header selects flags and a declared decoded length; older
//! streams have only the decoded length. Both share the same block grammar.
use anyhow::{ensure, Context, Result};

const TAG: u32 = 0xff435031;
fn take<'a>(source: &mut &'a [u8], len: usize) -> Result<&'a [u8]> {
    ensure!(source.len() >= len, "truncated byte-pair stream");
    let (head, tail) = source.split_at(len);
    *source = tail;
    Ok(head)
}
fn byte(source: &mut &[u8]) -> Result<u8> {
    Ok(take(source, 1)?[0])
}
fn word(source: &mut &[u8]) -> Result<u32> {
    Ok(u32::from_le_bytes(take(source, 4)?.try_into()?))
}

/// Expand a byte-pair stream, checking both its declaration and every emitted
/// byte against the caller's budget. Cyclic dictionaries fail visibly.
pub fn decode(mut source: &[u8], limit: usize) -> Result<Vec<u8>> {
    let first = word(&mut source)?;
    let (flags, size) = if first == TAG {
        (word(&mut source)?, word(&mut source)? as usize)
    } else {
        (0, first as usize)
    };
    ensure!(flags & !1 == 0, "unsupported byte-pair stream flags");
    ensure!(size <= limit, "byte-pair decoded byte budget");
    let mut out = Vec::with_capacity(size);
    while !source.is_empty() {
        let mut left = std::array::from_fn::<_, 256, _>(|i| i as u8);
        let mut right = [0u8; 256];
        let mut index = 0usize;
        loop {
            let mut count = byte(&mut source)? as usize;
            if count > 127 {
                index = index
                    .checked_add(count - 127)
                    .context("byte-pair dictionary cursor")?;
                count = 0;
            }
            if index >= 256 {
                ensure!(index == 256, "byte-pair dictionary cursor");
                break;
            }
            for _ in 0..=count {
                ensure!(index < 256, "byte-pair dictionary overrun");
                left[index] = byte(&mut source)?;
                if left[index] != index as u8 {
                    right[index] = byte(&mut source)?;
                }
                index += 1;
            }
            if index == 256 {
                break;
            }
        }
        let count = if flags & 1 != 0 {
            u16::from_le_bytes(take(&mut source, 2)?.try_into()?) as usize
        } else {
            word(&mut source)? as usize
        };
        let tokens = take(&mut source, count)?;
        for &token in tokens {
            let mut stack = vec![token];
            let mut steps = 0usize;
            while let Some(symbol) = stack.pop() {
                steps += 1;
                ensure!(
                    steps <= size.saturating_mul(2).saturating_add(512),
                    "cyclic byte-pair dictionary"
                );
                if left[symbol as usize] == symbol {
                    ensure!(out.len() < size, "byte-pair output exceeds declaration");
                    out.push(symbol);
                } else {
                    ensure!(stack.len() < 512, "byte-pair dictionary nesting");
                    stack.push(right[symbol as usize]);
                    stack.push(left[symbol as usize]);
                }
            }
        }
    }
    ensure!(out.len() == size, "byte-pair decoded length mismatch");
    Ok(out)
}

/// Produce a valid dictionary stream using literal blocks. This lossless
/// profile has bounded linear encoding cost; compression ratio is optional.
pub fn encode_literals(source: &[u8], limit: usize) -> Result<Vec<u8>> {
    ensure!(source.len() <= limit, "byte-pair input byte budget");
    let size = u32::try_from(source.len()).context("byte-pair input length")?;
    let mut out = Vec::new();
    out.extend_from_slice(&TAG.to_le_bytes());
    out.extend_from_slice(&1u32.to_le_bytes());
    out.extend_from_slice(&size.to_le_bytes());
    for block in source.chunks(32768) {
        // Skip identity symbols0..127, define128 as itself, then skip129..255.
        out.extend_from_slice(&[255, 128, 254]);
        out.extend_from_slice(&(block.len() as u16).to_le_bytes());
        out.extend_from_slice(block);
    }
    Ok(out)
}
