// SPDX-License-Identifier: MPL-2.0
//! Flat CPZ3 reader. The original reader checks ciphertext before transforming it.
use super::*;

fn checksum(data: &[u8]) -> u32 {
    let (words, tail) = data.as_chunks::<4>();
    let sum = words.iter().fold(0x7834_1256u32, |sum, word| {
        sum.wrapping_add(u32::from_le_bytes(*word))
    });
    tail.iter()
        .fold(sum, |sum, &byte| sum.wrapping_add(u32::from(byte)))
}

fn transform(data: &mut [u8], base: &[u32; 16], key: u32) {
    let mut fold = key ^ (key >> 16);
    fold ^= fold >> 8;
    fold ^= fold >> 4;
    let shift = ((fold ^ 0xd) & 15) + 8;
    let table = base.map(|word| word.wrapping_add(key));
    let (words, tail) = data.as_chunks_mut::<4>();
    let mut slot = 3;
    for word in words {
        let value = u32::from_le_bytes(*word) ^ table[slot];
        *word = value
            .wrapping_add(0x6e58_a5c2)
            .rotate_left(shift)
            .to_le_bytes();
        slot = (slot + 1) % 16;
    }
    let count = tail.len();
    for (i, byte) in tail.iter_mut().enumerate() {
        *byte = (*byte ^ (table[slot] >> (4 * (count - i))) as u8).wrapping_add(0x52);
        slot = (slot + 1) % 16;
    }
}

pub(in super::super) fn open(source: &dyn AssetReader, cx: &DecodeContext) -> Result<Vec<Row>> {
    let header = bytes(source, 0, 20, 20)?;
    ensure!(&header[..4] == b"CPZ3", "expected CPZ3 signature");
    let count = (le32(&header, 4)? ^ 0x5e9c_4f37) as usize;
    limits(cx, count)?;
    let size = u64::from(le32(&header, 8)? ^ 0xf32a_ed17);
    ensure!(count as u64 * 25 <= size, "CPZ3 record count exceeds index");
    let mut records = index(source, cx, 20, size)?;
    ensure!(
        checksum(&records) == le32(&header, 12)?,
        "CPZ3 index checksum mismatch"
    );
    let base = key_bytes(cx, "purple-software/cpz3")?.with_bytes(|b| -> Result<[u32; 16]> {
        ensure!(b.len() == 64, "CPZ3 needs a 64-byte archive table");
        Ok(std::array::from_fn(|i| {
            u32::from_le_bytes(b.as_chunks::<4>().0[i])
        }))
    })?;
    transform(
        &mut records,
        &base,
        le32(&header, 16)? ^ 0xdddd_dddd ^ 0x7bf4_a539,
    );
    let mut cursor = Cursor::new(&records);
    let mut rows = Vec::with_capacity(count);
    let file_len = source.len()?;
    for _ in 0..count {
        cx.check_cancelled()?;
        let record_len = cursor.le32()? as usize;
        ensure!(record_len >= 25, "CPZ3 short record");
        let mut row = Cursor::new(cursor.take(record_len - 4)?);
        let len = u64::from(row.le32()?);
        let at = 20 + size + u64::from(row.le32()?);
        ensure!(at + len <= file_len, "CPZ3 member outside archive");
        row.take(4)?;
        let sum = row.le32()?;
        let key = row.le32()? ^ 0xc7f5_da63;
        let name = sjis(row.zero()?)?;
        rows.push(Row::packed(
            name,
            at,
            len,
            len,
            decode(move |src, _| {
                ensure!(checksum(src) == sum, "CPZ3 member checksum mismatch");
                let mut out = src.to_vec();
                transform(&mut out, &base, key);
                Ok(out)
            }),
        ));
    }
    Ok(rows)
}
