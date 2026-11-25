// SPDX-License-Identifier: MPL-2.0
//! Trigger regions of a package and, per spawner whose name contains a
//! filter word, the regions holding it (parented regions in the parent
//! spawner's frame): `trigger_regions PAK [WORD…]`.
use anyhow::Result;
use crysirror_core::service::MemoryAssetReader;
use crysirror_naughty_dog::{dc::sid, mesh::Loaded, scene, trigger};
use glam::{Quat, Vec3};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let p = Loaded::read(&MemoryAssetReader(std::fs::read(&args[0])?.into()), 1 << 30)?;
    let mut spawners = Vec::new();
    for item in p.directory.items.iter().filter(|i| i.kind == "SPAWNER_GROUP") {
        spawners.extend(scene::spawner_records(&p, item.payload_offset as usize)?);
    }
    let frame = |name: u32| spawners.iter().find(|s| sid(&s.name) == name).map(|s| (Vec3::from_array(s.position), Quat::from_array(s.rotation).normalize()));
    let mut regions = Vec::new();
    for item in p.directory.items.iter().filter(|i| i.kind == "REGION_TREE") {
        regions.extend(trigger::regions(&p, item.payload_offset as usize)?);
    }
    let kinds = |r: &trigger::Region| r.shapes.iter().map(|s| match s { trigger::Shape::Planes(f) => format!("planes{}", f.len()), trigger::Shape::Sphere(..) => "sphere".into(), trigger::Shape::Cylinder { .. } => "cylinder".into(), trigger::Shape::Quads => "quads".into() }).collect::<Vec<_>>().join(",");
    println!("{} regions", regions.len());
    if args.len() == 1 {
        for r in &regions {
            println!("  {} parent {:?} categories {:08x?} [{}] enter {:?} exit {:?}", r.name, r.parent.map(|p| format!("{p:08x}")), r.categories, kinds(r), r.enter.map(|x| format!("{x:#x}")), r.exit.map(|x| format!("{x:#x}")));
        }
    }
    for s in spawners.iter().filter(|s| args[1..].iter().any(|w| s.name.contains(w.as_str()))) {
        let at = Vec3::from_array(s.position);
        let inside: Vec<&str> = regions
            .iter()
            .filter(|r| {
                let local = match r.parent {
                    Some(parent) => match frame(parent) {
                        Some((t, q)) => q.inverse() * (at - t),
                        None => return false,
                    },
                    None => at,
                };
                r.contains_local(local)
            })
            .map(|r| r.name.as_str())
            .collect();
        println!("{}: {inside:?}", s.name);
    }
    Ok(())
}
