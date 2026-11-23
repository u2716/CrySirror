// SPDX-License-Identifier: MPL-2.0
//! HScene surface-effect emitters: `h/list/siru/obi` (`SiruObiInfoTable`
//! per posture, `{file}_{id:02}`) and `h/list/siru/emitter`
//! (`EmitterTargetInfoTable` `Emitter_def`).
//!
//! `ObiCtrl.Setup` builds one fluid per actor with the table entry
//! `2·i + sex` (males 0, 2, …; females 1, 3, …); each entry's key indexes
//! the sex's emitter targets (`EmitterTargetInfo.Parent`, the bone the
//! emitter follows). `ObiCtrl.Proc` (`<Proc>g__IsAnimation`) plays an
//! entry when the actor's animator state hash equals a `ShootInfo`'s
//! `StateHash` and its normalized time crosses `NormalizeTime`; an
//! `Inside` shot needs the scene's inside flag.
use crate::unity::Unity;
use anyhow::{Context, Result};
use crysirror_unity::memorypack::Reader;

pub const OBI_LIST: &str = "lib/h/list/siru/obi/000_01.unity3d";
pub const EMITTER_LIST: &str = "lib/h/list/siru/emitter/000_01.unity3d";

/// `ShootInfo` (`Inside`, `StateHash`, `NormalizeTime`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shoot {
    pub inside: bool,
    pub state_hash: u32,
    pub time: f32,
}

/// One `SiruObiInfo`: the table entry (`2·i + sex`), the emitter target
/// index, `Info.OffsetRot` (degrees, the `Value` vector; the Lerp range
/// applies under a lerp parameter) and the shots.
#[derive(Clone, Debug, PartialEq)]
pub struct Emitter {
    pub actor: i32,
    pub target: i32,
    pub offset_rot: [f32; 3],
    pub offset_rot_max: [f32; 3],
    pub shoots: Vec<Shoot>,
}

fn vec3(b: &[u8]) -> [f32; 3] {
    let f = |i: usize| b.get(i * 4..i * 4 + 4).map_or(0., |s| f32::from_le_bytes([s[0], s[1], s[2], s[3]]));
    [f(0), f(1), f(2)]
}

/// `Information`: header, the version-tolerant `SiruObiParamInfo`, then
/// `OffsetPos` and `OffsetRot` (`UseLerpValue`: `Value` and the lerp's
/// min/mid/max vectors).
fn offset_rot(info: &[u8]) -> Result<([f32; 3], [f32; 3])> {
    let mut r = Reader::new(info);
    r.object_header()?.context("null SiruObiInfo.Info")?;
    r.tolerant()?;
    r.take(48)?;
    let rot = r.take(48)?;
    Ok((vec3(&rot[..12]), vec3(&rot[36..48])))
}

/// A `SiruObiInfoTable` text asset.
pub fn parse_obi(b: &[u8]) -> Result<Vec<Emitter>> {
    let mut r = Reader::new(b);
    r.object_header()?.context("null SiruObiInfoTable")?;
    let n = r.i32()?;
    let mut out = Vec::new();
    for _ in 0..n.max(0) {
        let actor = r.i32()?;
        let m = r.i32()?;
        for _ in 0..m.max(0) {
            let target = r.i32()?;
            let Some(f) = r.tolerant()? else { continue };
            let field = |i: usize| f.get(i).copied().unwrap_or(&[]);
            let (offset_rot, offset_rot_max) = offset_rot(field(0))?;
            let mut shoots = Vec::new();
            if let Some(a) = Reader::new(field(4)).array(16)? {
                for e in a.chunks_exact(16) {
                    shoots.push(Shoot { inside: e[4] != 0, state_hash: u32::from_le_bytes([e[8], e[9], e[10], e[11]]), time: f32::from_le_bytes([e[12], e[13], e[14], e[15]]) });
                }
            }
            out.push(Emitter { actor, target, offset_rot, offset_rot_max, shoots });
        }
    }
    Ok(out)
}

/// `EmitterTargetInfoTable`: sex → the targets' `Parent` bones in order.
pub fn parse_targets(b: &[u8]) -> Result<Vec<(i32, Vec<String>)>> {
    let mut r = Reader::new(b);
    let top = r.tolerant()?.context("null EmitterTargetInfoTable")?;
    let mut r = Reader::new(top.first().copied().context("EmitterTargetInfoTable.Table")?);
    let n = r.i32()?;
    let mut out = Vec::new();
    for _ in 0..n.max(0) {
        let sex = r.i32()?;
        let m = r.i32()?;
        let mut parents = Vec::new();
        for _ in 0..m.max(0) {
            let f = r.tolerant()?.unwrap_or_default();
            parents.push(f.get(2).and_then(|b| Reader::new(b).string().ok().flatten()).unwrap_or_default());
        }
        out.push((sex, parents));
    }
    Ok(out)
}

fn text(unity: &mut Unity, bundle: &str, name: &str) -> Result<Option<Vec<u8>>> {
    for key in unity.bundle_files().into_iter().filter(|k| k.starts_with(&format!("bundle:{bundle}#")) && !k.ends_with(".resS")) {
        let ids: Vec<i64> = unity.file(&key)?.objects.iter().filter(|o| o.class_id == 49).map(|o| o.path_id).collect();
        for id in ids {
            if unity.decode(&key, id)?.string("m_Name") == Some(name) {
                return unity.text_asset(&key, id).map(Some);
            }
        }
    }
    Ok(None)
}

/// Mount the hit-object prefabs for this actor before localizing emitter targets.
/// HitObjectCtrl.Load (0x180d92690) reads the sex-specific base table.
/// Its Add callback resets the instance root's position and rotation while
/// preserving its scale and all child transforms.
pub fn attach_hit_objects(unity: &mut Unity, human: &mut crate::chara::Human) -> Result<()> {
    let bundle = "lib/h/list/hit/object/base/000_01.unity3d";
    let bytes = text(unity, bundle, "base")?.context("HitObjectBaseData table")?;
    let mut r = Reader::new(&bytes);
    anyhow::ensure!(r.object_header()? == Some(1), "hit-object table layout");
    let groups = r.i32()?;
    anyhow::ensure!(groups >= 0, "negative hit-object group count");
    for _ in 0..groups {
        let sex = r.i32()?;
        let count = r.i32()?;
        anyhow::ensure!(count >= 0, "negative hit-object count");
        for _ in 0..count {
            anyhow::ensure!(r.object_header()? == Some(3), "hit-object record layout");
            let parent = r.string()?.context("hit-object parent")?;
            let bundle = r.string()?.context("hit-object bundle")?;
            let asset = r.string()?.context("hit-object asset")?;
            if sex != i32::from(human.female) {
                continue;
            }
            let Some(bone) = human.scene.nodes.iter().position(|n| n.name == parent) else {
                human.log.push(format!("missing hit-object parent {parent} for {asset}"));
                continue;
            };
            let prefab = crate::level::Level::prefab(unity, &format!("lib/{bundle}"), &asset)?;
            let root = human.scene.graft(&prefab.scene, &prefab.key, bone);
            let transform = &mut human.scene.nodes[root].xform;
            transform.position = [0.; 3];
            transform.rotation = [0., 0., 0., 1.];
        }
    }
    anyhow::ensure!(r.remaining() == 0, "trailing hit-object table data");
    Ok(())
}

/// A posture's emitters with their bones (`(sex, emitter, bone)`); empty
/// when the posture has no table.
pub fn load(unity: &mut Unity, posture: &str) -> Result<Vec<(i32, Emitter, String)>> {
    let Some(obi) = text(unity, OBI_LIST, posture)? else { return Ok(Vec::new()) };
    let targets = parse_targets(&text(unity, EMITTER_LIST, "Emitter_def")?.context("Emitter_def")?)?;
    let mut out = Vec::new();
    for e in parse_obi(&obi)? {
        let sex = e.actor & 1;
        let bone = targets.iter().find(|t| t.0 == sex).and_then(|t| usize::try_from(e.target).ok().and_then(|i| t.1.get(i))).cloned().with_context(|| format!("emitter target {sex}/{}", e.target))?;
        out.push((sex, e, bone));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crysirror_unity::memorypack::{member, Writer};

    #[test]
    fn obi_table_entries_and_shoots() {
        // Info: header 3, an empty tolerant param, OffsetPos, OffsetRot.
        let mut info = Writer::default();
        info.u8(3);
        info.u8(0);
        for _ in 0..12 {
            info.f32(0.);
        }
        for v in [180., 0., 0., 0., 0., 0., 0., 0., 0., 90., 0., 0.] {
            info.f32(v);
        }
        let mut shoots = Writer::default();
        shoots.i32(1);
        shoots.f32(0.);
        shoots.u8(1);
        shoots.u8(0);
        shoots.u8(0);
        shoots.u8(0);
        shoots.i32(crysirror_unity::animator::crc32("Drop") as i32);
        shoots.f32(0.25);
        let mut obj = Writer::default();
        obj.tolerant(&[info.out, member(|w| w.string(Some(""))), member(|w| w.i32(-1)), member(|w| w.i32(-1)), shoots.out]).unwrap();
        let mut table = Writer::default();
        table.u8(1);
        table.i32(1);
        table.i32(1);
        table.i32(1);
        table.i32(0);
        let mut b = table.out;
        b.extend(obj.out);
        let e = parse_obi(&b).unwrap();
        assert_eq!(e, vec![Emitter { actor: 1, target: 0, offset_rot: [180., 0., 0.], offset_rot_max: [90., 0., 0.], shoots: vec![Shoot { inside: true, state_hash: crysirror_unity::animator::crc32("Drop"), time: 0.25 }] }]);
    }
}
