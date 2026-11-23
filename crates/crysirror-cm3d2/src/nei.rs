// SPDX-License-Identifier: MPL-2.0
//! `.nei` tables (`csv.arc`), read through `DLL_CSV_*` in `cm3d2_x64.dll`.
//!
//! `DLL_CSV_Open` (0x180006fd0) requires `size - 5` to be a multiple of 16 and
//! calls `sub_180094270`: AES-128-CBC (CryptoPP `Rijndael::Dec`) over the
//! first `size - 5` bytes with the native table key. The IV is four
//! xorshift words derived from the last u32; `byte[-4] ^ byte[-5]` (< 16) is
//! the count of trailing padding bytes. `sub_1800934D0` then parses the
//! plaintext: i32 magic −9014409, i32 columns, i32 rows, `columns × rows`
//! row-major cells `{u32 offset, u32 size}` and the cell bytes. A cell's size
//! includes its NUL; strings are Shift-JIS (`NUty.ReadAsShiftJis`), integers
//! and reals use C `atoi`/`atof`.
use anyhow::{ensure, Context, Result};

const MAGIC: i32 = -9014409;

pub fn decrypt(bytes: &[u8], key: &[u8; 16]) -> Result<Vec<u8>> {
    ensure!(bytes.len() >= 5 && (bytes.len() - 5) % 16 == 0, "nei size is not 16n+5");
    let n = bytes.len();
    let pad = usize::from(bytes[n - 4] ^ bytes[n - 5]);
    ensure!(pad < 16, "nei padding count");
    let seed = u32::from_le_bytes(bytes[n - 4..].try_into()?) ^ 0xBFBF_BFBF;
    let a = seed ^ (seed >> 19) ^ 0xD9EA_5670;
    let b = a ^ (a >> 19) ^ 0xC7F2_4898;
    let c = b ^ (b >> 19) ^ 0x8E41_5C26;
    let s = seed ^ (seed << 11);
    let d = s ^ c ^ ((s ^ (c >> 11)) >> 8);
    let mut iv = [0u8; 16];
    for (i, word) in [a, b, c, d].iter().enumerate() {
        iv[i * 4..i * 4 + 4].copy_from_slice(&word.to_le_bytes());
    }
    let mut plain = crysirror_resource::aes_cbc_decrypt(&bytes[..n - 5], &iv, key, false)?;
    ensure!(plain.len() >= pad, "nei payload shorter than its padding");
    plain.truncate(plain.len() - pad);
    Ok(plain)
}

/// A parsed table; cells are addressed `(x = column, y = row)`.
#[derive(Clone, Debug)]
pub struct Table {
    pub columns: usize,
    pub rows: usize,
    cells: Vec<(u32, u32)>,
    data: Vec<u8>,
}

impl Table {
    pub fn parse(plain: Vec<u8>) -> Result<Self> {
        let word = |at: usize| -> Result<i32> {
            Ok(i32::from_le_bytes(plain.get(at..at + 4).context("truncated nei header")?.try_into()?))
        };
        ensure!(word(0)? == MAGIC, "nei magic");
        let (columns, rows) = (word(4)?, word(8)?);
        ensure!(columns >= 0 && rows >= 0, "nei dimensions");
        let count = (columns as usize).checked_mul(rows as usize).context("nei dimensions")?;
        let data_at = count.checked_mul(8).and_then(|n| n.checked_add(12)).context("nei dimensions")?;
        ensure!(data_at <= plain.len(), "nei cell index outside the table");
        let mut cells = Vec::with_capacity(count);
        for i in 0..count {
            let at = 12 + 8 * i;
            let offset = u32::from_le_bytes(plain[at..at + 4].try_into()?);
            let size = u32::from_le_bytes(plain[at + 4..at + 8].try_into()?);
            ensure!(
                (data_at as u64 + u64::from(offset) + u64::from(size)) <= plain.len() as u64,
                "nei cell outside the table"
            );
            cells.push((offset, size));
        }
        let data = plain[data_at..].to_vec();
        Ok(Self { columns: columns as usize, rows: rows as usize, cells, data })
    }
    pub fn open(bytes: &[u8], key: &[u8; 16]) -> Result<Self> {
        Self::parse(decrypt(bytes, key)?)
    }
    /// Cell bytes without the terminating NUL; empty outside the table.
    pub fn bytes(&self, x: usize, y: usize) -> &[u8] {
        if x >= self.columns || y >= self.rows {
            return &[];
        }
        let (offset, size) = self.cells[y * self.columns + x];
        let start = offset as usize;
        let raw = &self.data[start..start + size as usize];
        let end = raw.iter().position(|b| *b == 0).unwrap_or(raw.len());
        &raw[..end]
    }
    pub fn exists(&self, x: usize, y: usize) -> bool {
        !self.bytes(x, y).is_empty()
    }
    pub fn string(&self, x: usize, y: usize) -> String {
        encoding_rs::SHIFT_JIS.decode(self.bytes(x, y)).0.into_owned()
    }
    /// C `atoi`: optional sign and leading digits after whitespace.
    pub fn int(&self, x: usize, y: usize) -> i32 {
        crate::text::atoi(self.bytes(x, y))
    }
    pub fn real(&self, x: usize, y: usize) -> f32 {
        crate::text::atof(self.bytes(x, y))
    }
    /// `DLL_CSV_GetCellBool`: a non-empty cell other than `0`/`false`.
    pub fn bool(&self, x: usize, y: usize) -> bool {
        let s = self.string(x, y);
        let s = s.trim();
        !s.is_empty() && s != "0" && !s.eq_ignore_ascii_case("false")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_cells_are_row_major_with_nul() {
        let mut t = Vec::new();
        t.extend(MAGIC.to_le_bytes());
        t.extend(2i32.to_le_bytes());
        t.extend(1i32.to_le_bytes());
        t.extend([0u32, 3, 3, 1].iter().flat_map(|v| v.to_le_bytes()));
        t.extend(b"12\0\0");
        let table = Table::parse(t).unwrap();
        assert_eq!(table.int(0, 0), 12);
        assert!(!table.exists(1, 0));
        assert_eq!(table.string(5, 5), "");
    }
}
