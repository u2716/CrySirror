// SPDX-License-Identifier: MPL-2.0
//! `.preset` files (`CharacterMgr.PresetLoad`, `Maid.DeserializePropPre`,
//! `MaidProp.Deserialize`) and `CharacterMgr.PresetSet`.
use crate::chara::{Maid, MPN};
use anyhow::{ensure, Result};
use crysirror_asset::interchange::cm3d2::Reader;

#[derive(Clone, Debug, PartialEq)]
pub struct Prop {
    pub idx: i32,
    pub name: String,
    pub kind: i32,
    pub value: i32,
    pub file: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Preset {
    /// `PresetType`: 0 Wear, 1 Body, 2 All.
    pub kind: i32,
    pub props: Vec<Prop>,
    /// `MaidParts` colours (preset version ≥ 2).
    pub parts_color: Option<[crate::parts_color::PartsColor; 7]>,
}

/// `CM3.dicDelItem`: the menu that empties a prop without a file.
pub fn del_item(mpn: &str) -> Option<&'static str> {
    Some(match mpn {
        "hairt" => "_I_hairt_del.menu", "hairs" => "_I_hairs_del.menu", "acctatoo" => "_I_acctatoo_del.menu",
        "underhair" => "_I_underhair_del.menu", "hokuro" => "_I_hokuro_del.menu", "lip" => "_I_lip_del.menu",
        "wear" => "_I_wear_del.menu", "skirt" => "_I_skirt_del.menu", "mizugi" => "_I_mizugi_del.menu",
        "bra" => "_I_bra_del.menu", "panz" => "_I_panz_del.menu", "stkg" => "_I_stkg_del.menu",
        "shoes" => "_I_shoes_del.menu", "headset" => "_I_headset_del.menu", "glove" => "_I_glove_del.menu",
        "acchead" => "_I_acchead_del.menu", "hairaho" => "_I_hairaho_del.menu", "accha" => "_I_accha_del.menu",
        "acchana" => "_I_acchana_del.menu", "acckamisub" => "_I_acckamisub_del.menu", "acckami" => "_I_acckami_del.menu",
        "accmimi" => "_I_accmimi_del.menu", "accnip" => "_I_accnip_del.menu", "acckubi" => "_I_acckubi_del.menu",
        "acckubiwa" => "_I_acckubiwa_del.menu", "accheso" => "_I_accheso_del.menu", "accude" => "_I_accude_del.menu",
        "accashi" => "_I_accashi_del.menu", "accsenaka" => "_I_accsenaka_del.menu", "accshippo" => "_I_accshippo_del.menu",
        "accanl" => "_I_accanl_del.menu", "accvag" => "_I_accvag_del.menu", "megane" => "_I_megane_del.menu",
        "accxxx" => "_I_accxxx_del.menu", "handitem" => "_I_handitem_del.menu", "acchat" => "_I_acchat_del.menu",
        "onepiece" => "_I_onepiece_del.menu", "folder_underhair" => "_I_underhair_folder_del.menu",
        "kousoku_upper" => "_I_KousokuU_del.menu", "kousoku_lower" => "_I_KousokuL_del.menu",
        _ => return None,
    })
}

impl Preset {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let mut r = Reader::new(bytes);
        let version = r.header("CM3D2_PRESET")?;
        let kind = r.i32()?;
        let thumb = r.count(1)?;
        r.take(thumb)?;
        let list_version = r.header("CM3D2_MPROP_LIST")?;
        let n = r.count(8)?;
        let mut props = Vec::with_capacity(n);
        for _ in 0..n {
            if list_version >= 4 {
                r.string()?;
            }
            r.header("CM3D2_MPROP")?;
            let idx = r.i32()?;
            let name = r.string()?;
            let kind = r.i32()?;
            let _default = r.i32()?;
            let value = r.i32()?;
            let _link_max = r.i32()?;
            let file = r.string()?;
            let _rid = r.i32()?;
            let _dut = r.bool()?;
            let _max = r.i32()?;
            let _min = r.i32()?;
            props.push(Prop { idx, name, kind, value, file });
        }
        ensure!(!props.is_empty(), "preset without props");
        let parts_color = if version >= 2 { Some(crate::parts_color::deserialize(&mut r)?) } else { None };
        Ok(Self { kind, props, parts_color })
    }

    /// `CharacterMgr.PresetSet`: the prop ranges of the preset type.
    pub fn apply(&self, maid: &mut Maid) {
        for p in &self.props {
            let selected = match self.kind {
                1 => (1..=38).contains(&p.idx) || (73..=76).contains(&p.idx),
                0 => (39..=69).contains(&p.idx),
                _ => (1..=69).contains(&p.idx) || (73..=76).contains(&p.idx),
            };
            if !selected || p.idx == 21 {
                continue;
            }
            let Some(mpn) = MPN.get(p.idx as usize) else { continue };
            // SetProp(MPN, value) then SetProp(MPN, file, 0).
            if let Some(prop) = maid.props.get_mut(p.idx as usize) {
                prop.value = p.value.clamp(prop.min, prop.max);
                prop.dut = true;
            }
            let file = if p.file.is_empty() { del_item(&mpn.to_ascii_lowercase()).unwrap_or("").to_owned() } else { p.file.clone() };
            maid.set_prop(mpn, &file);
        }
        if self.kind != 0 {
            for (i, c) in self.parts_color.iter().flatten().enumerate() {
                maid.set_parts_color(i, *c);
            }
        }
    }
}
