// SPDX-License-Identifier: MPL-2.0
//! Skin an art group with a clip's first frame and list the triangles
//! whose edges stretch far beyond their bind-pose length, with the joints
//! their vertices use. `skin_stretch MODEL_PAK SKELETON_PAK CLIP_PAK CLIP`
use anyhow::{Context, Result};
use crysirror_core::service::MemoryAssetReader;
use crysirror_naughty_dog::{anim, mesh::Loaded, package::Directory, scene, skeleton};
use glam::{Mat4, Vec3};
use std::collections::BTreeMap;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let bytes = std::fs::read(args.first().context("MODEL_PAK")?)?;
    let reader = MemoryAssetReader(bytes.into());
    let model = Loaded::read(&reader, 1 << 30)?;
    let meshes = scene::meshes_of(&model, &Directory::read(&reader)?)?;
    let skel = skeleton::of_package(&Loaded::read(&MemoryAssetReader(std::fs::read(args.get(1).context("SKELETON_PAK")?)?.into()), 1 << 30)?)?.context("no skeleton")?;
    let clips = Loaded::read(&MemoryAssetReader(std::fs::read(args.get(2).context("CLIP_PAK")?)?.into()), 1 << 30)?;
    let name = args.get(3).context("CLIP")?;
    let (_, payload) = anim::items(&clips).find(|(n, _)| n == name).context("clip not found")?;
    let clip = anim::decode(&clips, payload, name, skel.animated)?;
    let mut world = vec![Mat4::IDENTITY; skel.names.len()];
    for j in skel.order() {
        let (mut t, mut r, mut s) = skel.rest[j];
        if let Some(track) = clip.joints.get(j) {
            t = track.translation.as_ref().map_or(t, |v| v[0]);
            r = track.rotation.as_ref().map_or(r, |v| v[0]);
            s = track.scale.as_ref().map_or(s, |v| v[0]);
        }
        let local = Mat4::from_scale_rotation_translation(s, r, t);
        world[j] = skel.parents[j].map_or(local, |p| world[p] * local);
    }
    let palette: Vec<Mat4> = world.iter().zip(&skel.inverse_bind).map(|(w, ib)| *w * Mat4::from_cols_array(ib)).collect();
    let mut joints: BTreeMap<String, usize> = BTreeMap::new();
    let (mut total, mut bad) = (0, 0);
    for m in &meshes {
        for f in &m.fragments {
            if f.skin.is_empty() {
                continue;
            }
            let posed: Vec<Vec3> = f
                .positions
                .iter()
                .zip(&f.skin)
                .map(|(p, inf)| {
                    let p = Vec3::from_array(*p);
                    let sum: f32 = inf.iter().map(|x| x.1).sum();
                    inf.iter().filter(|(j, _)| usize::from(*j) < palette.len()).map(|(j, w)| palette[usize::from(*j)].transform_point3(p) * (*w / sum.max(1e-6))).sum()
                })
                .collect();
            for t in &f.triangles {
                total += 1;
                let edge = |a: usize, b: usize, v: &dyn Fn(usize) -> Vec3| (v(a) - v(b)).length();
                let bind = |i: usize| Vec3::from_array(f.positions[i]);
                let pose = |i: usize| posed[i];
                let (a, b, c) = (usize::from(t[0]), usize::from(t[1]), usize::from(t[2]));
                let worst = [(a, b), (b, c), (c, a)].iter().map(|&(x, y)| edge(x, y, &pose) / edge(x, y, &bind).max(1e-3)).fold(0f32, f32::max);
                if worst > 4. {
                    bad += 1;
                    for v in [a, b, c] {
                        for (j, w) in &f.skin[v] {
                            if *w > 0. {
                                *joints.entry(skel.names.get(usize::from(*j)).cloned().unwrap_or(format!("#{j}"))).or_default() += 1;
                            }
                        }
                    }
                }
            }
        }
    }
    if std::env::var_os("PARENTS").is_some() {
        for (j, n) in skel.names.iter().enumerate().filter(|(_, n)| n.contains("helper") || n.contains("twist")) {
            let p = skel.parents[j].map_or("-".to_string(), |p| skel.names[p].clone());
            let animated = clip.joints.get(j).is_some_and(|t| t.rotation.is_some() || t.translation.is_some());
            println!("  joint {j:3} {n} parent {p} animated {animated} (animated joints < {})", skel.animated);
        }
    }
    println!("{bad} of {total} triangles stretched more than 4×");
    let mut list: Vec<_> = joints.into_iter().collect();
    list.sort_by(|a, b| b.1.cmp(&a.1));
    for (n, c) in list.iter().take(20) {
        println!("  {c:5} {n}");
    }
    Ok(())
}
