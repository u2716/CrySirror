// SPDX-License-Identifier: MPL-2.0
//! Top-level records emitted by 0x4258e0 and consumed by 0x426310.
//! Short and long payload lengths coexist. Scene resources have one or two
//! 16-bit indices before their long length. There is no alignment padding.
use anyhow::{Context, Result, bail, ensure};

fn take<'a>(input: &mut &'a [u8], size: usize) -> Result<&'a [u8]> {
    let (field, tail) = input
        .split_at_checked(size)
        .context("truncated CSP1 checkpoint record")?;
    *input = tail;
    Ok(field)
}

fn word(input: &mut &[u8]) -> Result<u16> {
    Ok(u16::from_le_bytes(take(input, 2)?.try_into()?))
}

fn index(input: &mut &[u8], limit: u16) -> Result<()> {
    ensure!(
        word(input)? < limit,
        "CSP1 resource index outside its native table"
    );
    Ok(())
}

fn c_string(input: &mut &[u8]) -> Result<()> {
    let bytes = input
        .iter()
        .position(|&b| b == 0)
        .context("unterminated CSP1 resource name")?;
    take(input, bytes + 1)?;
    Ok(())
}

// The original child readers copy fixed fields and then follow their local
// counts. Check them within the enclosing record, before a resource is opened.
fn scene_record(tag: u16, mut data: &[u8]) -> Result<()> {
    match tag {
        0x480 | 0x481 => {
            let components = word(&mut data)?;
            ensure!(components <= 8, "CSP1 bitmap component count");
            for _ in 0..components {
                index(&mut data, 8)?;
                take(&mut data, 100)?;
            }
        }
        0x4c0 | 0x4c1 => {
            take(&mut data, 1336)?;
            if word(&mut data)? != 0 {
                take(&mut data, 40)?;
            }
        }
        0x500 => {
            take(&mut data, 4)?;
            c_string(&mut data)?;
            take(&mut data, 160)?;
            let glyphs = usize::from(word(&mut data)?);
            take(&mut data, glyphs * 124)?;
        }
        0x540 => {
            take(&mut data, 4)?;
            let entries = u32::from_le_bytes(take(&mut data, 4)?.try_into()?);
            for _ in 0..entries {
                for _ in 0..3 {
                    c_string(&mut data)?;
                }
                take(&mut data, 8)?;
            }
        }
        0x620 => {
            take(&mut data, 16)?;
            c_string(&mut data)?;
            take(&mut data, 1056)?;
        }
        0x700 => {
            c_string(&mut data)?;
            take(&mut data, 12)?;
        }
        _ => return Ok(()),
    }
    ensure!(
        data.is_empty(),
        "invalid CSP1 resource record length for {tag:#06x}"
    );
    Ok(())
}

// The grammar is shared by the detached reader and the native writer.
fn layout(tag: u16) -> Result<(bool, &'static [u16])> {
    Ok(match tag {
        0xff..=0x10b | 0x200..=0x204 | 0x280..=0x281 | 0x288 | 0x300 => (false, &[]),
        0x282..=0x283 | 0x2a0..=0x2a2 | 0x301 | 0x540 | 0x600 | 0x610 | 0x620 | 0x900 | 0x911 => {
            (true, &[])
        }
        0x400 => (true, &[2]),
        0x480 => (true, &[128]),
        0x481 => (true, &[2, 5]),
        0x4c0 => (true, &[1024]),
        0x4c1 => (true, &[2, 20]),
        0x500 => (true, &[8]),
        0x700 => (true, &[6]),
        _ => bail!("unsupported CSP1 checkpoint record {tag:#06x}"),
    })
}

fn payload(tag: u16, data: &[u8]) -> Result<()> {
    let size = data.len();
    let expected = match tag {
        0x103..=0x106 | 0x108 | 0x10a | 0x200..=0x204 | 0x280..=0x281 => Some(4),
        0x107 => Some(64),
        0x109 => Some(40),
        0x10b => Some(1796),
        0x282 => Some(256),
        0x283 => Some(65536),
        0x288 => Some(28),
        0x301 => Some(4368),
        0x400 => Some(1582),
        0x600 => Some(56),
        0x610 => Some(28),
        _ => None,
    };
    ensure!(
        expected.is_none_or(|n| n == size),
        "invalid CSP1 record size for {tag:#06x}"
    );
    match tag {
        0xff..=0x102 | 0x300 => ensure!(data.last() == Some(&0), "unterminated CSP1 string"),
        0x2a0 => ensure!(size % 4 == 0, "misaligned CSP1 integer bank"),
        0x2a2 if !data.is_empty() => {
            let cells = data
                .strip_suffix(&[0])
                .context("unterminated CSP1 string bank")?;
            ensure!(
                cells.split(|&b| b == 0).all(|s| s.len() < 1024),
                "CSP1 string exceeds native cell"
            );
        }
        _ => scene_record(tag, data)?,
    }
    Ok(())
}

pub(crate) fn visit<'a>(
    mut input: &'a [u8],
    mut record: impl FnMut(u16, &[u16], &'a [u8]) -> Result<()>,
) -> Result<()> {
    while !input.is_empty() {
        let tag = word(&mut input)?;
        let (wide, limits) = layout(tag)?;
        let mut indices = [0; 2];
        for (i, limit) in indices.iter_mut().zip(limits) {
            *i = word(&mut input)?;
            ensure!(*i < *limit, "CSP1 resource index outside its native table");
        }
        let size = if wide {
            u32::from_le_bytes(take(&mut input, 4)?.try_into()?) as usize
        } else {
            usize::from(word(&mut input)?)
        };
        let data = take(&mut input, size)?;
        payload(tag, data)?;
        record(tag, &indices[..limits.len()], data)?;
    }
    Ok(())
}

pub(crate) fn append(out: &mut Vec<u8>, tag: u16, indices: &[u16], data: &[u8]) -> Result<()> {
    let (wide, limits) = layout(tag)?;
    ensure!(
        indices.len() == limits.len() && indices.iter().zip(limits).all(|(i, n)| i < n),
        "CSP1 resource index outside its native table"
    );
    payload(tag, data)?;
    let length = if wide { 4 } else { 2 };
    let header = 2 + 2 * indices.len() + length;
    ensure!(
        out.len().saturating_add(header).saturating_add(data.len()) <= 0x300000
            && (wide || data.len() <= u16::MAX as usize),
        "CSP1 record exceeds native checkpoint allocation"
    );
    out.try_reserve(header + data.len())?;
    out.extend_from_slice(&tag.to_le_bytes());
    for i in indices {
        out.extend_from_slice(&i.to_le_bytes());
    }
    out.extend_from_slice(&(data.len() as u32).to_le_bytes()[..length]);
    out.extend_from_slice(data);
    Ok(())
}

pub(super) fn validate(input: &[u8]) -> Result<()> {
    visit(input, |_, _, _| Ok(()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mixed_length_and_index_fields_end_at_the_exact_record_boundary() {
        let mut bytes = vec![0x00, 0x01, 3, 0, b'a', b'b', 0];
        bytes.extend([0x81, 0x04, 1, 0, 4, 0, 2, 0, 0, 0, 0, 0]);
        bytes.extend([0x11, 0x09, 0, 0, 0, 0]);
        assert!(validate(&bytes).is_ok());
        for end in [1, 5, 8, 10, 13, 18, 20, 23] {
            assert!(validate(&bytes[..end]).is_err(), "end {end}");
        }
        bytes[11] = 5;
        assert!(validate(&bytes).is_err());
    }
}

#[cfg(feature = "runtime")]
pub(crate) fn required(tag: u16) -> bool {
    matches!(tag,0xff..=0x10b | 0x200..=0x204 | 0x280..=0x283 | 0x288 | 0x2a0..=0x2a2 | 0x900 | 0x301 | 0x300)
}

/// sub_426310 reconstructs message windows before their ADV owners and
/// bitmaps before sprites. Put the component owners' records in native order.
#[cfg(feature = "runtime")]
pub(crate) fn ordered(records: &[u8]) -> Result<Vec<u8>> {
    const ORDER: &[u16] = &[
        0x100, 0xff, 0x101, 0x102, 0x103, 0x104, 0x105, 0x106, 0x107, 0x108, 0x109, 0x10a, 0x10b,
        0x200, 0x201, 0x202, 0x203, 0x204, 0x280, 0x281, 0x282, 0x283, 0x288, 0x2a0, 0x2a1, 0x2a2,
        0x500, 0x400, 0x480, 0x481, 0x4c0, 0x4c1, 0x540, 0x600, 0x610, 0x620, 0x900, 0x301, 0x300,
        0x700,
    ];
    let mut fields = std::collections::BTreeMap::new();
    visit(records, |tag, idx, data| {
        let order = ORDER
            .iter()
            .position(|&t| t == tag)
            .context("unexpected record in CVNS checkpoint")?;
        ensure!(
            fields.insert((order, idx.to_vec()), data).is_none(),
            "duplicate CVNS checkpoint record {tag:#06x}"
        );
        Ok(())
    })?;
    let mut out = Vec::with_capacity(records.len());
    for &tag in ORDER {
        if required(tag) {
            let pos = ORDER.iter().position(|&t| t == tag).unwrap();
            ensure!(
                fields.contains_key(&(pos, Vec::new())),
                "missing CVNS checkpoint record {tag:#06x}"
            );
        }
    }
    for ((order, idx), data) in fields {
        append(&mut out, ORDER[order], &idx, data)?;
    }
    Ok(out)
}
