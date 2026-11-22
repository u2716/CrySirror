// SPDX-License-Identifier: MPL-2.0
//! `AL.H` data: the posture lists (`h/list/animationinfo/info`, MemoryPack
//! `List<AnimationListInfo>` per category file) with each character's
//! animator (`CharaInfo.Info`: bundle + controller of the posture).
use crate::unity::Unity;
use anyhow::{Context, Result};
use crysirror_unity::memorypack::Reader;

pub const LIST_INFO: &str = "lib/h/list/animationinfo/info/000_01.unity3d";
pub const SCENE_BUNDLE: &str = "lib/h/scene/hscene.unity3d";
pub const SCENE_PREFAB: &str = "HScene";

/// `AssetBundleInfo` (bundle, asset, manifest).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AssetInfo {
    pub bundle: String,
    pub asset: String,
}

/// `CharaInfo`: one character's animators in a posture.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CharaInfo {
    pub base: AssetInfo,
    pub info: AssetInfo,
    pub neck: String,
}

impl CharaInfo {
    pub fn is_male(&self) -> bool {
        self.info.bundle.contains("/male/")
    }
}

/// `AnimationListInfo` (the posture).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Posture {
    /// Category file (`aib`, `hou`, `sof`, `som`, `rez`, `3p`).
    pub file: String,
    pub id: i32,
    pub name: String,
    pub charas: Vec<CharaInfo>,
    pub camera: String,
    /// `Mode`, `Kind`.
    pub mode: i32,
    pub kind: i32,
    /// `Positions`: the H place kinds the posture can use.
    pub positions: Vec<i32>,
    /// `Category` (`PostureCategory`; none: -1).
    pub category: i32,
    /// `ReleasePhase`, `PostureState`, `WeaknessID`.
    pub release_phase: i32,
    pub state: i32,
    pub weakness: i32,
}

fn asset_info(r: &mut Reader) -> Result<AssetInfo> {
    let Some(n) = r.object_header()? else { return Ok(AssetInfo::default()) };
    let bundle = r.string()?.unwrap_or_default();
    let asset = r.string()?.unwrap_or_default();
    for _ in 2..n {
        r.string()?;
    }
    Ok(AssetInfo { bundle, asset })
}

fn chara_info(b: &[u8]) -> Result<CharaInfo> {
    let m = Reader::new(b).tolerant()?.context("null CharaInfo")?;
    let field = |i: usize| m.get(i).copied().unwrap_or(&[]);
    let base = if field(0).is_empty() { AssetInfo::default() } else { asset_info(&mut Reader::new(field(0)))? };
    let info = if field(1).is_empty() { AssetInfo::default() } else { asset_info(&mut Reader::new(field(1)))? };
    let neck = if field(3).is_empty() { String::new() } else { Reader::new(field(3)).string()?.unwrap_or_default() };
    Ok(CharaInfo { base, info, neck })
}

/// One category file: an object holding an `IntKeyDictionary` of
/// `AnimationListInfo` (version-tolerant
/// members: ID, name, chara infos, mode, kind, positions, file offsets,
/// flags, camera file, SE file, …).
pub fn parse_postures(file: &str, b: &[u8]) -> Result<Vec<Posture>> {
    let mut r = Reader::new(b);
    r.object_header()?.context("null posture table")?;
    let n = r.i32()?;
    let mut out = Vec::new();
    for _ in 0..n.max(0) {
        let _key = r.i32()?;
        let Some(m) = r.tolerant()? else { continue };
        let field = |i: usize| m.get(i).copied().unwrap_or(&[]);
        let id = if field(0).len() >= 4 { Reader::new(field(0)).i32()? } else { 0 };
        let name = Reader::new(field(1)).string().ok().flatten().unwrap_or_default();
        let mut charas = Vec::new();
        if !field(2).is_empty() {
            let mut cr = Reader::new(field(2));
            let count = cr.i32()?;
            for _ in 0..count.max(0) {
                // Each element is a version-tolerant object.
                let start = cr.at;
                let Some(members) = cr.tolerant()? else { continue };
                let _ = members;
                charas.push(chara_info(&field(2)[start..cr.at])?);
            }
        }
        let camera = Reader::new(field(9)).string().ok().flatten().unwrap_or_default();
        let int = |i: usize| if field(i).len() >= 4 { Reader::new(field(i)).i32().unwrap_or(0) } else { 0 };
        let positions = {
            let mut r = Reader::new(field(5));
            match r.i32() {
                Ok(n) if n > 0 => (0..n).filter_map(|_| r.i32().ok()).collect(),
                _ => Vec::new(),
            }
        };
        // `Nullable<int>` (unmanaged layout): has-value, padding, value.
        let category = match field(14) {
            [1, _, _, _, rest @ ..] if rest.len() >= 4 => Reader::new(rest).i32().unwrap_or(-1),
            _ => -1,
        };
        out.push(Posture { file: file.to_owned(), id, name, charas, camera, mode: int(3), kind: int(4), positions, category, release_phase: int(15), state: int(18), weakness: int(12) });
    }
    Ok(out)
}

/// Every posture of the info lists.
pub fn load_postures(unity: &mut Unity) -> Result<Vec<Posture>> {
    let key = unity.bundle_files().into_iter().find(|k| k.starts_with(&format!("bundle:{LIST_INFO}#"))).context("H animation info bundle")?;
    let ids: Vec<(i64, String)> = {
        let f = unity.file(&key)?;
        f.objects.iter().filter(|o| o.class_id == 49).map(|o| (o.path_id, String::new())).collect()
    };
    let mut out = Vec::new();
    for (id, _) in ids {
        let name = unity.decode(&key, id)?.string("m_Name").unwrap_or("").to_owned();
        let file = name.split('_').next().unwrap_or("").to_owned();
        out.extend(parse_postures(&file, &unity.text_asset(&key, id)?)?);
    }
    Ok(out)
}

/// `h/list/hpoint/object`: map id → the `HPoint` prefab of the map.
pub const HPOINT_LIST: &str = "lib/h/list/hpoint/object/000_01.unity3d";

/// `AL.H.HPoint`: a place of the map where H can happen.
#[derive(Clone, Debug, PartialEq)]
pub struct HPoint {
    /// `_placeType` (0 floor, 1 chair, 2 table, 3 beach chair, 4 poolside,
    /// 5 pool, …): matched against `AnimationListInfo.Positions`.
    pub place: i32,
    pub id: i32,
    /// `_isFirst`: where an H starts (`HPointCtrl.FindFirst`).
    pub first: bool,
    pub pos: [f32; 3],
    pub rot: [f32; 4],
}

/// The `HPoint`s of a map (`HPointCtrl`: the list's prefab for the map).
pub fn load_hpoints(unity: &mut Unity, map: i32) -> Result<Vec<HPoint>> {
    let key = unity.bundle_files().into_iter().find(|k| k.starts_with(&format!("bundle:{HPOINT_LIST}#"))).context("H point list bundle")?;
    let id = unity.file(&key)?.objects.iter().find(|o| o.class_id == 49).map(|o| o.path_id).context("H point list")?;
    let b = unity.text_asset(&key, id)?;
    let mut r = Reader::new(&b);
    r.object_header()?.context("null H point list")?;
    let n = r.i32()?;
    let mut found = None;
    for _ in 0..n.max(0) {
        let k = r.i32()?;
        let info = asset_info(&mut r)?;
        if k == map {
            found = Some(info);
        }
    }
    let Some(info) = found else { return Ok(Vec::new()) };
    let level = crate::level::Level::prefab(unity, &format!("lib/{}", info.bundle), &info.asset)?;
    let mut out = Vec::new();
    for (i, n) in level.scene.nodes.iter().enumerate() {
        let Some(c) = n.components.iter().find(|c| c.script == "AL.H.HPoint") else { continue };
        let (_, rot, pos) = crysirror_unity::world6::world_matrix(&level.scene, i).to_scale_rotation_translation();
        out.push(HPoint { place: c.fields.int("_placeType").unwrap_or(0) as i32, id: c.fields.int("_id").unwrap_or(0) as i32, first: c.fields.int("_isFirst").unwrap_or(0) != 0, pos: pos.to_array(), rot: rot.to_array() });
    }
    Ok(out)
}

/// `AnimationListInfo.CheckReleasePhase` over the favourability,
/// inclusiveness, proactivity and curiosity levels.
pub fn release_phase_ok(phase: i32, lv: [i32; 4]) -> bool {
    match phase {
        1 => lv[0] >= 4,
        2 => lv[0] == 6,
        3 => lv[1] >= 2,
        4 => lv[1] == 4,
        5 => lv[2] >= 2,
        6 => lv[2] == 4,
        7 => lv[3] >= 2,
        8 => lv[3] == 4,
        _ => true,
    }
}

impl Posture {
    /// `HScene.Resource.FindPosture.Check`: unlocked, the exhaustion
    /// variant only while weak (service has none), usable at the place,
    /// and an H does not start standing.
    /// SATD-amanatsu-port: `HPointLimit` exclusions are not read.
    pub fn usable(&self, weak: bool, place: i32, lv: [i32; 4], first: bool) -> bool {
        if !release_phase_ok(self.release_phase, lv) {
            return false;
        }
        if weak && self.weakness != -1 {
            return false;
        }
        if !weak && self.mode != 1 && self.weakness == -1 {
            return false;
        }
        if !self.positions.contains(&place) {
            return false;
        }
        !first || place != 0 || self.state != 0
    }
}

/// `PostureMainSelecter.CheckPosture` for a heroine with the male player:
/// caress, service and the insert/special categories (no Les or M1F2).
pub fn category_allowed(category: i32) -> bool {
    (0..=4).contains(&category)
}

/// `FindPosture(isFirst)`: a random usable posture of the allowed
/// categories.
pub fn first_posture<'a>(postures: &'a [Posture], place: i32, lv: [i32; 4], random: &mut dyn FnMut(usize) -> usize) -> Option<&'a Posture> {
    let list: Vec<&Posture> = postures.iter().filter(|p| category_allowed(p.mode) && p.usable(false, place, lv, true)).collect();
    (!list.is_empty()).then(|| list[random(list.len())])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn posture(mode: i32, positions: Vec<i32>, release: i32, state: i32, weakness: i32) -> Posture {
        Posture { mode, positions, release_phase: release, state, weakness, ..Default::default() }
    }

    #[test]
    fn posture_checks() {
        let lv = [1, 0, 0, 0];
        // Standing floor postures are not first; a chair one is.
        assert!(!posture(0, vec![0], 0, 0, 24).usable(false, 0, lv, true));
        assert!(posture(0, vec![0], 0, 1, 17).usable(false, 0, lv, true));
        assert!(posture(0, vec![1], 0, 0, 27).usable(false, 1, lv, true));
        // Place, release phase and exhaustion variants.
        assert!(!posture(0, vec![1], 0, 1, 27).usable(false, 0, lv, false));
        assert!(!posture(2, vec![0], 1, 1, 18).usable(false, 0, lv, false));
        assert!(posture(2, vec![0], 1, 1, 18).usable(false, 0, [4, 0, 0, 0], false));
        assert!(!posture(2, vec![0], 0, 1, -1).usable(false, 0, lv, false));
        assert!(posture(1, vec![0], 0, 1, -1).usable(false, 0, lv, false));
        assert!(posture(2, vec![0], 0, 1, -1).usable(true, 0, lv, false));
    }
}
