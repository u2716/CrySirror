// SPDX-License-Identifier: MPL-2.0
//! Print a camera clip's joint and `apReference` samples.
//! `camera_clip CLIP_PAK CLIP [ANIMATED_JOINTS] [STEP]`
use anyhow::{Context, Result};
use crysirror_core::service::MemoryAssetReader;
use crysirror_naughty_dog::{anim, mesh::Loaded};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let clips = Loaded::read(&MemoryAssetReader(std::fs::read(args.first().context("CLIP_PAK")?)?.into()), 1 << 30)?;
    let name = args.get(1).context("CLIP")?;
    let animated = args.get(2).map_or(Ok(2), |s| s.parse())?;
    let step = args.get(3).map_or(Ok(10), |s| s.parse())?;
    let (_, payload) = anim::items(&clips).find(|(n, _)| n == name).context("clip not found")?;
    println!("hierarchy {:08x}", anim::hierarchy(&clips, payload)?);
    for (name, stride, components, frames) in anim::raw_channels(&clips, payload) {
        println!("raw channel {name:08x} stride {stride} components {components} frames {frames}");
        for (f, v) in anim::raw_samples(&clips, payload, name).iter().enumerate().step_by(step.max(1)) {
            println!("    {f}: {v:?}");
        }
    }
    let clip = anim::decode(&clips, payload, name, animated)?;
    println!("{} frames at {} fps, {} joints, unsupported {:?}", clip.frames, clip.fps, clip.joints.len(), clip.unsupported);
    for (j, t) in clip.joints.iter().enumerate() {
        for f in (0..clip.frames).step_by(step.max(1)) {
            let tr = t.translation.as_ref().and_then(|v| v.get(f));
            let r = t.rotation.as_ref().and_then(|v| v.get(f));
            println!("  joint {j} frame {f}: t {tr:?} r {r:?}");
        }
    }
    for (f, (p, q)) in clip.align.iter().enumerate().step_by(step.max(1)) {
        println!("  align {f}: {p:?} {q:?}");
    }
    for (f, (p, q)) in clip.ap_reference.iter().enumerate().step_by(step.max(1)) {
        println!("  apReference {f}: {p:?} {q:?}");
    }
    Ok(())
}
