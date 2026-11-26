// SPDX-License-Identifier: MPL-2.0
//! Edit.MaidProfile.Create for new and player-edited stock characters.
use super::*;
use crate::{chara::Maid, params::MaidStatus};

struct Block {
    table: Arc<Table>,
    top: usize,
    bottom: usize,
    right: usize,
}

impl Block {
    fn open(s: &mut State, number: u8) -> Result<Self> {
        let table = s.table(&format!("profile_comment_{number}.nei"))?;
        let mut block = Self {
            table,
            top: 0,
            bottom: 0,
            right: 0,
        };
        block.seek(0)?;
        Ok(block)
    }
    fn seek(&mut self, start: usize) -> Result<()> {
        let t = &self.table;
        if let Some(row) = (start..t.rows).find(|&r| t.exists(0, r)) {
            self.top = row;
            self.right = (0..t.columns)
                .take_while(|&c| t.exists(c, row))
                .count()
                .saturating_sub(1);
        }
        ensure!(
            self.right > 0 && t.rows > self.top,
            "invalid profile comment block"
        );
        self.bottom = (self.top..t.rows)
            .take_while(|&r| (0..=self.right).any(|c| t.exists(c, r)))
            .last()
            .context("empty profile block")?;
        for c in self.right + 2..t.columns {
            if (self.top..=self.bottom).any(|row| t.exists(c, row)) {
                self.right = c - 1;
            }
        }
        Ok(())
    }
    fn next(&mut self) -> Result<()> {
        self.seek(self.bottom + 1)
    }
    fn first(&self, pred: impl Fn(&Table, usize) -> bool) -> Option<usize> {
        (self.top + 1..=self.bottom).find(|&r| pred(&self.table, r))
    }
    fn text(&self, pred: impl Fn(&Table, usize) -> bool) -> String {
        self.first(pred)
            .map(|r| self.table.string(self.right, r))
            .unwrap_or_default()
    }
}
fn range(t: &Table, r: usize, c: usize, n: i32) -> bool {
    (!t.exists(c, r) || t.int(c, r) <= n) && (!t.exists(c + 1, r) || n <= t.int(c + 1, r))
}

pub(super) fn initialize(s: &mut State, m: &mut MaidStatus, model: &Maid) -> Result<()> {
    let mut b = Block::open(s, 1)?;
    let feature_table = s.table("maid_status_feature_list.nei")?;
    let features: BTreeSet<_> = m
        .feature
        .keys()
        .map(|&id| Ok(feature_table.string(1, row_id(&feature_table, id)?)))
        .collect::<Result<_>>()?;
    let part = |t: &Table, r| {
        m.parts_dic
            .get(&t.string(1, r))
            .is_none_or(|value| *value == t.string(2, r))
    };
    let feature = |t: &Table, r| !t.exists(1, r) || features.contains(&t.string(1, r));
    let mut text = b.text(part);
    b.next()?;
    text += &b.text(feature);
    b.next()?;
    text += &b.text(part);
    let mut b = Block::open(s, 2)?;
    for value in [m.body.bust, m.body.waist, m.body.hip] {
        text += &b
            .table
            .string(b.right, b.bottom)
            .replace("[n]", &value.to_string());
        b.next()?;
    }
    text += &b.text(|t, r| {
        t.string(1, r).contains(&m.body.cup)
            && (t.string(2, r) != "○" || model.value("MuneTare") >= 50)
    });
    b.next()?;
    text += &b.text(|t, r| m.body.waist < t.int(1, r) && range(t, r, 2, m.body.hip));
    for _ in 0..2 {
        b.next()?;
        text += &b.text(|t, r| {
            m.personal == t.int(1, r) && (!t.exists(2, r) || features.contains(&t.string(2, r)))
        });
    }
    let mut b = Block::open(s, 3)?;
    text += &b.text(|t, r| {
        range(t, r, 1, m.body.bust) && range(t, r, 3, m.body.waist) && range(t, r, 5, m.body.hip)
    });
    b.next()?;
    text += &b.text(|t, r| m.personal == t.int(1, r));
    b.next()?;
    text += &b.text(|t, r| m.current_maid_class == t.int(1, r));
    let mut b = Block::open(s, 4)?;
    text += &b.text(|t, r| m.personal == t.int(1, r));
    b.next()?;
    text += &b.text(|t, r| m.contract_type == t.int(1, r));
    b.next()?;
    for _ in 0..2 * m.seikeiken {
        b.next()?;
    }
    let value = if m.seikeiken == 0 {
        m.a_inyoku()
    } else {
        m.play_number
    };
    if let Some(r) = b.first(|t, r| !t.exists(1, r) || t.int(1, r) <= value) {
        m.study_rate = b.table.int(2, r).clamp(0, 1000);
        text += &b.table.string(b.right, r);
    }
    let b = Block::open(s, 5)?;
    if !m.propensity.is_empty() {
        let definitions = s.table("maid_status_propensity_list.nei")?;
        let owned: BTreeSet<_> = m
            .propensity
            .keys()
            .map(|&id| Ok(definitions.string(1, row_id(&definitions, id)?)))
            .collect::<Result<_>>()?;
        let stats = [m.a_inyoku(), m.a_m_value(), m.a_hentai(), m.a_housi()];
        let appeal = [m.a_lovely(), m.a_elegance(), m.a_charm()];
        let strongest = (0..3).rev().max_by_key(|&i| appeal[i]).unwrap();
        if let Some(r) = b.first(|t, r| {
            let kind = match t.string(2, r).as_str() {
                "処女" => Some(0),
                "前穴" => Some(1),
                "後穴" => Some(2),
                "両穴" => Some(3),
                _ => None,
            };
            owned.contains(&t.string(1, r))
                && kind.is_none_or(|k| m.seikeiken == k)
                && stats
                    .iter()
                    .enumerate()
                    .all(|(i, &v)| !t.exists(3 + i, r) || t.int(3 + i, r) <= v)
                && (t.int(7, r) <= 0 || t.int(7, r) <= appeal[strongest])
        }) {
            let variant = if b.table.int(7, r) > 0 { strongest } else { 0 };
            text += &b.table.string(
                b.right.checked_sub(2).context("invalid profile columns")? + variant,
                r,
            );
        }
    }
    for (number, count) in [(6, 1), (7, 3)] {
        let mut b = Block::open(s, number)?;
        for _ in 0..count {
            b.next()?;
            text += &b.text(feature);
        }
    }
    m.profile = text;
    Ok(())
}
