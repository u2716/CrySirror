// SPDX-License-Identifier: MPL-2.0
//! Game-bank records in CSP1. The suffixes live independently in CSS0.
use super::{evaluate::State, save_slot::csp1};
use anyhow::{Context, Result, ensure};

pub(super) fn capture(
    state: &State,
    [bits, integers, strings]: [u32; 3],
    out: &mut Vec<u8>,
) -> Result<()> {
    ensure!(
        bits <= 262144 && integers <= 4096 && strings <= 256,
        "invalid CVNS game-bank boundary"
    );
    let mut values = Vec::with_capacity(integers as usize * 4);
    for index in 0..integers {
        values.extend_from_slice(
            &state
                .integer(index)
                .context("CVNS integer outside bank")?
                .to_le_bytes(),
        );
    }
    csp1::append(out, 0x2a0, &[], &values)?;
    let mut flags = state.packed_bit_suffix(0);
    flags.truncate((bits / 8) as usize);
    if let Some(first) = flags.first_mut() {
        *first |= 1;
    } // 0x4258e0 marks bit zero in the saved prefix
    csp1::append(out, 0x2a1, &[], &flags)?;
    let mut text = Vec::new();
    for index in 0..strings {
        text.extend_from_slice(state.string(index).context("CVNS string outside bank")?);
        text.push(0);
    }
    csp1::append(out, 0x2a2, &[], &text)
}

pub(super) fn restore(state: &mut State, strings: u32, tag: u16, data: &[u8]) -> Result<()> {
    match tag {
        0x2a0 => {
            ensure!(
                data.len() <= 4096 * 4 && data.len() % 4 == 0,
                "invalid CSP1 integer bank"
            );
            for (index, bytes) in data.as_chunks::<4>().0.iter().enumerate() {
                state
                    .fill_integers(index as u32, 1, i32::from_le_bytes(*bytes))
                    .map_err(anyhow::Error::msg)?;
            }
        }
        0x2a1 => {
            ensure!(data.len() <= 32768, "invalid CSP1 bit bank");
            state.restore_bits(0, data).map_err(anyhow::Error::msg)?;
        }
        0x2a2 => {
            ensure!(strings <= 256, "invalid CSP1 string count");
            let mut src = data;
            for index in 0..strings {
                let len = src
                    .iter()
                    .position(|&b| b == 0)
                    .context("unterminated CSP1 string")?;
                ensure!(len < 1024, "oversized CSP1 string");
                state
                    .fill_strings(index, 1, &src[..len])
                    .map_err(anyhow::Error::msg)?;
                // 0x426704 increments EDI by lstrlenA without skipping the NUL.
                src = &src[len..];
            }
        }
        _ => anyhow::bail!("unexpected CSP1 game-bank tag {tag:#x}"),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_prefix_load_keeps_css0_and_observes_the_x86_string_cursor() -> Result<()> {
        let mut original = State::new().with_string_cells(256);
        original.fill_strings(0, 1, b"one").unwrap();
        original.fill_strings(1, 1, b"two").unwrap();
        original.fill_integers(1, 1, -19).unwrap();
        let mut out = Vec::new();
        capture(&original, [8, 2, 2], &mut out)?;
        let mut candidate = original.clone();
        candidate.fill_strings(2, 1, b"system").unwrap();
        candidate.fill_integers(2, 1, 43).unwrap();
        csp1::visit(&out, |tag, _, data| restore(&mut candidate, 2, tag, data))?;
        assert_eq!(candidate.bit(0), Some(true));
        assert_eq!(
            (candidate.integer(1), candidate.integer(2)),
            (Some(-19), Some(43))
        );
        assert_eq!(candidate.string(0), Some(b"one".as_slice()));
        assert_eq!(candidate.string(1), Some(b"".as_slice()));
        assert_eq!(candidate.string(2), Some(b"system".as_slice()));
        Ok(())
    }
}
