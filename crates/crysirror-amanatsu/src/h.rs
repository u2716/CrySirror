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
        out.push(Posture { file: file.to_owned(), id, name, charas, camera });
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
