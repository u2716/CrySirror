// SPDX-License-Identifier: MPL-2.0
//! `AL.Serialization.TableData`: MemoryPack table TextAssets (`sd/lis`
//! `bgm[…]`, `sse[…]` and the other lists) — column keys and rows of string
//! values — and the `AL.ContentProvider` dictionaries built from them.
use anyhow::{ensure, Context, Result};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Table {
    /// `TableData.Key` of each column.
    pub keys: Vec<i32>,
    pub rows: Vec<Vec<String>>,
}

struct Pack<'a> {
    b: &'a [u8],
    at: usize,
}

impl Pack<'_> {
    fn take(&mut self, n: usize) -> Result<&[u8]> {
        let s = self.b.get(self.at..self.at + n).context("TableData truncated")?;
        self.at += n;
        Ok(s)
    }
    fn i32(&mut self) -> Result<i32> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into()?))
    }
    fn string(&mut self) -> Result<String> {
        let h = self.i32()?;
        if h == -1 {
            return Ok(String::new());
        }
        if h >= 0 {
            let u: Vec<u16> = self.take(h as usize * 2)?.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
            return Ok(String::from_utf16_lossy(&u));
        }
        let n = !h as usize;
        let _utf16 = self.i32()?;
        Ok(String::from_utf8(self.take(n)?.to_vec())?)
    }
}

impl Table {
    /// Parse the TextAsset bytes (`m_Script`).
    pub fn parse(b: &[u8]) -> Result<Self> {
        let mut p = Pack { b, at: 0 };
        let tag = p.take(1)?[0];
        let list = match tag {
            // Union-style header: member count, the type name, reserved words.
            6 => {
                let ty = p.string()?;
                ensure!(ty.starts_with("AL.Serialization.TableData"), "TableData type {ty}");
                p.take(12)?;
                false
            }
            // `chara/list` tables: category number and a reserved word.
            4 => {
                let _category = p.i32()?;
                p.take(4)?;
                true
            }
            other => anyhow::bail!("TableData header {other}"),
        };
        let nkeys = p.i32()?;
        ensure!((0..=256).contains(&nkeys), "TableData key count");
        let keys = (0..nkeys).map(|_| p.i32()).collect::<Result<Vec<_>>>()?;
        let nrows = p.i32()?;
        ensure!((0..=1_000_000).contains(&nrows), "TableData row count");
        let mut rows = Vec::with_capacity(nrows as usize);
        for _ in 0..nrows {
            let _index = p.i32()?;
            if !list {
                // Info object header and its reserved fields.
                p.take(9)?;
            }
            let n = p.i32()?;
            ensure!((0..=4096).contains(&n), "TableData column count");
            rows.push((0..n).map(|_| p.string()).collect::<Result<Vec<_>>>()?);
        }
        ensure!(p.at == b.len(), "TableData: {} trailing bytes", b.len() - p.at);
        Ok(Self { keys, rows })
    }

    /// Rows as (id, bundle, asset): columns 0, 2 and 3 of the sound lists.
    pub fn assets(&self) -> Vec<(i32, String, String)> {
        self.rows
            .iter()
            .filter_map(|r| Some((r.first()?.parse().ok()?, r.get(2)?.clone(), r.get(3)?.clone())))
            .collect()
    }
}
