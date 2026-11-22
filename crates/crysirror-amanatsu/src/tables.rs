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

/// `ContentProvider.ShortMovieInfoContents` (`am/lis` `al_short_movie`):
/// short movie id → `Map` (`ALShortMovieContent.MapID`).
pub fn short_movie_maps(unity: &mut crate::unity::Unity) -> Result<std::collections::HashMap<i32, i32>> {
    let (key, id) = unity.bundle_asset("lib/am/lis/000_00.unity3d", "al_short_movie[000_00]", 49)?.context("al_short_movie")?;
    let t = Table::parse(&unity.text_asset(&key, id)?)?;
    let col = |k: i32| t.keys.iter().position(|&c| c == k);
    // `TableData.Key`: ID 5, Map 103.
    let (Some(i), Some(m)) = (col(5), col(103)) else { anyhow::bail!("al_short_movie columns {:?}", t.keys) };
    Ok(t.rows.iter().filter_map(|r| Some((r.get(i)?.parse().ok()?, r.get(m)?.parse().ok()?))).collect())
}

/// `ADV.Commands.Chara.Motion`: a motion id of the `ani/lis/m` lists
/// (`anim_f_{personality:03}` for heroines, `anim_m` for the player;
/// every bundle's `{list}[…]` table) → the state name of the character's
/// motion controller (`TableData.Key` 5 ID, 68 state).
pub fn motion_state(unity: &mut crate::unity::Unity, list: &str, id: i32) -> Result<Option<String>> {
    let keys: Vec<String> = unity.bundle_files().into_iter().filter(|k| k.starts_with("bundle:lib/ani/lis/m/") && !k.ends_with(".resS")).collect();
    for key in keys {
        let file = unity.file(&key)?;
        let texts: Vec<i64> = file.objects.iter().filter(|o| o.class_id == crysirror_unity::class_id::TEXT_ASSET).map(|o| o.path_id).collect();
        for tid in texts {
            let name = unity.decode(&key, tid)?.string("m_Name").unwrap_or("").to_owned();
            if !name.strip_prefix(list).is_some_and(|r| r.starts_with('[')) {
                continue;
            }
            let t = Table::parse(&unity.text_asset(&key, tid)?).with_context(|| format!("motion list {name}"))?;
            let col = |k: i32| t.keys.iter().position(|&c| c == k);
            let (Some(i), Some(s)) = (col(5), col(68)) else { continue };
            if let Some(r) = t.rows.iter().find(|r| r.get(i).and_then(|v| v.parse::<i32>().ok()) == Some(id)) {
                return Ok(r.get(s).filter(|v| !v.is_empty()).cloned());
            }
        }
    }
    Ok(None)
}
