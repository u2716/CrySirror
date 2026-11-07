// Copyright (C)2026 CrySirror contributors. GPL-2.0-or-later.
//! Native AliceLogo2 four-season timing/resource policy, read statically from
//! original10001490/10001FB0/100028A0/100036B0. This builds shared immutable
//! image operations; it contains no surface, rasterizer, decoder or pixel loop.
use super::*;
use crysirror_core::image_blend::{
    RgbaBlendArithmetic as A, RgbaBlendComponent as C, RgbaBlendEquation as E,
    RgbaBlendFactor as F, RgbaBlendState as S, RgbaByteRounding as Q,
};
use crysirror_vfx::falling_atlas::SequenceRng;
#[derive(Clone, Copy)]
pub(super) struct Particle {
    start: [i32; 2],
    end: [i32; 2],
    duration: u32,
    texture: i32,
}
pub(super) fn particles(season: i32) -> Result<Arc<[Particle]>> {
    let mut rng = SequenceRng::msvc(1);
    let count = match season {
        1 => 192,
        3 => 16,
        4 => 512,
        _ => 0,
    };
    let mut output = Vec::with_capacity(count);
    for _ in 0..count {
        let mut next = |n: u32| -> Result<i32> { Ok((rng.next_u32()? % n) as i32) };
        let (start, end, duration, texture) = match season {
            1 => {
                let x = next(1600)? - 800;
                let y = -next(600)?;
                (
                    [x, y],
                    [x + 800 + next(800)?, 600 + next(600)?],
                    next(1500)? as u32 + 3000,
                    0,
                )
            }
            3 => {
                let x = next(832)? - 16;
                let y = -next(600)?;
                ([x, y], [x, 600 + next(600)?], next(3000)? as u32 + 8000, 0)
            }
            4 => {
                let x = next(864)? - 32;
                let y = -next(600)?;
                (
                    [x, y],
                    [x + next(32)? - 16, 600 + next(600)?],
                    next(1500)? as u32 + 3000,
                    next(8)?,
                )
            }
            _ => unreachable!(),
        };
        output.push(Particle {
            start,
            end,
            duration,
            texture,
        });
    }
    Ok(output.into())
}
fn blend(alpha: Option<u8>, source_alpha: bool) -> ImageCompositeMath {
    let mut state = S {
        write_mask: [true, true, true, false],
        arithmetic: A::Byte255 {
            source: Q::Floor,
            destination: Q::Floor,
        },
        ..Default::default()
    };
    state.color = C {
        source: if source_alpha {
            F::SourceAlpha
        } else {
            F::ConstantAlpha
        },
        destination: if source_alpha {
            F::OneMinusSourceAlpha
        } else {
            F::OneMinusConstantAlpha
        },
        equation: E::Add,
    };
    if source_alpha {
        state.source_scale[3] = f32::from(alpha.unwrap_or(255)) / 255.;
    } else {
        state.constant[3] = f32::from(alpha.unwrap_or(255)) / 255.;
    }
    ImageCompositeMath::Normalized(state)
}
fn dimensions(input: &ImageInput) -> Result<[u32; 2]> {
    match input {
        ImageInput::Resource(resource) => Ok(resource.size()),
        ImageInput::Solid { size, .. } => Ok(*size),
        _ => anyhow::bail!("native Logo unknown unprepared input extent"),
    }
}
fn put(
    out: &mut Vec<ImageCompositeLayer>,
    input: ImageInput,
    size: [u32; 2],
    position: [i32; 2],
    math: ImageCompositeMath,
) {
    // The original clip helper10009720 clips source and destination together,
    // before invoking Graph. Rejected off-screen unsigned-wrap coordinates do
    // not consume a graph node or become an out-of-budget shared coordinate.
    let x = position[0].max(0);
    let y = position[1].max(0);
    let right = (i64::from(position[0]) + i64::from(size[0])).min(800);
    let bottom = (i64::from(position[1]) + i64::from(size[1])).min(600);
    if right <= i64::from(x) || bottom <= i64::from(y) {
        return;
    }
    out.push(ImageCompositeLayer {
        input: Box::new(input),
        source: [
            x - position[0],
            y - position[1],
            (right - i64::from(x)) as i32,
            (bottom - i64::from(y)) as i32,
        ],
        destination: [x, y],
        clip: None,
        opacity: 255,
        math,
    });
}
fn image(scene: &Scene, id: i32) -> Result<ImageInput> {
    Ok(ImageInput::Resource(
        scene
            .images
            .get(&id)
            .with_context(|| format!("native Logo image {id} absent"))?
            .clone(),
    ))
}
fn draw(
    out: &mut Vec<ImageCompositeLayer>,
    scene: &Scene,
    id: i32,
    pos: [i32; 2],
    math: ImageCompositeMath,
) -> Result<()> {
    let input = image(scene, id)?;
    let size = dimensions(&input)?;
    put(out, input, size, pos, math);
    Ok(())
}
fn center(scene: &Scene, id: i32) -> Result<[i32; 2]> {
    let size = dimensions(&image(scene, id)?)?;
    Ok([(800 - size[0] as i32) / 2, (600 - size[1] as i32) / 2])
}
fn point(p: &Particle, phase: u32) -> [i32; 2] {
    // Original IMUL followed by unsigned DIV is intentional, including winter
    // negative horizontal deltas. Signed interpolation changes native motion.
    std::array::from_fn(|axis| {
        p.start[axis].wrapping_add(
            phase
                .wrapping_mul(p.end[axis].wrapping_sub(p.start[axis]) as u32)
                .wrapping_div(p.duration) as i32,
        )
    })
}
pub(super) fn sample(
    scene: &Scene,
    time: u32,
    check: &dyn Fn() -> Result<()>,
) -> Result<VecDeque<ImageCompositeLayer>> {
    ensure!(time < 11000, "native Logo timeline outside active cycle");
    check()?;
    let mut out = Vec::with_capacity(524);
    match scene.season {
        1 => {
            draw(&mut out, scene, 17, [0, 0], ImageCompositeMath::CopyRgb)?;
            let blink = (time / 400) & 1;
            let id = if time < 5000 || (7600..8400).contains(&time) {
                if blink == 0 { 3 } else { 0 }
            } else {
                2 - blink as i32
            };
            draw(&mut out, scene, id, [0, 0], blend(None, true))?;
            for p in scene.particles.iter() {
                check()?;
                let phase = (time + 8000) % p.duration;
                draw(
                    &mut out,
                    scene,
                    8 + (8 * phase / p.duration) as i32,
                    point(p, phase),
                    blend(None, true),
                )?;
            }
            if time >= 7000 {
                draw(&mut out, scene, 45, center(scene, 45)?, blend(None, true))?;
            } else if time >= 6000 {
                let delta = (time - 6000) as i32;
                let shift = 32 - 32 * delta / 1000;
                let mut pos = center(scene, 45)?;
                pos[0] -= shift;
                draw(
                    &mut out,
                    scene,
                    45,
                    pos,
                    blend(Some((255 * delta / 1000) as u8), true),
                )?;
                pos[0] += shift * 2;
                draw(
                    &mut out,
                    scene,
                    45,
                    pos,
                    blend(Some((255 * delta / 1000) as u8), true),
                )?;
            }
        }
        2 => {
            draw(&mut out, scene, 30, [0, 0], ImageCompositeMath::CopyRgb)?;
            // Raw100023E1..10002467 proves the hidden EDX alpha omitted by
            // Hex-Rays: the same shimmer image fades up/down each2-second cycle.
            let phase = time % 2000;
            let alpha = if phase < 1000 {
                255 * phase / 1000
            } else {
                255 - 255 * (phase - 1000) / 1000
            };
            draw(&mut out, scene, 31, [0, 0], blend(Some(alpha as u8), false))?;
            let mut rng = SequenceRng::msvc(time / 300);
            let noise = dimensions(&image(scene, 32)?)?;
            for n in 0..32 {
                check()?;
                let x = (rng.next_u32()? % (800 + noise[0])) as i32 - noise[0] as i32;
                let y = (rng.next_u32()? % (600 + noise[1])) as i32 - noise[1] as i32;
                draw(
                    &mut out,
                    scene,
                    32,
                    [x, y],
                    blend(Some(((time + n * 8) & 127) as u8), true),
                )?;
            }
            let index = if time / 200 == 24 {
                0
            } else {
                [4, 1, 4, 3][((time / 200) & 3) as usize]
            };
            let id = if index == 4 { 44 } else { 4 + index };
            let x = 800 - (time as i32 * (800 / 4 + 2 * 800)) / 10000;
            draw(&mut out, scene, id, [x, 0], blend(None, true))?;
            let mut pos = center(scene, 45)?;
            pos[0] = pos[0].max(x + 800 - 260);
            draw(&mut out, scene, 45, pos, blend(None, true))?;
        }
        3 => {
            let source_width = (800. - 400. * f64::from(time) * 0.000099999997) as f32;
            let source_height = (600. - 300. * f64::from(time) * 0.000099999997) as f32;
            let x = ((800. - f64::from(source_width)) * 0.5) as f32;
            let y = ((600. - f64::from(source_height)) * 0.5) as f32;
            let input=ImageInput::Process {input:Box::new(image(scene,22)?),effects:serde_json::to_vec(&json!([
                {"kind":"rect_resample_byte256","spec":{"size":[800,600],"source":[x,y,source_width,source_height]}}
            ]))?.into()};
            put(
                &mut out,
                input,
                [800, 600],
                [0, 0],
                ImageCompositeMath::CopyRgb,
            );
            let blink = ((time / 350) & 1) as i32;
            let id = 18
                + if time >= 6650 {
                    if blink == 0 { 0 } else { 3 }
                } else {
                    blink + 1
                };
            draw(&mut out, scene, id, [0, 0], blend(None, true))?;
            for (n, p) in scene.particles.iter().enumerate() {
                check()?;
                let phase = (time + 15000) % p.duration;
                let mut pos = point(p, phase);
                let angle = (1440 * time / 10000 + n as u32 * 30) % 360;
                let radians = f64::from(angle) * 0.0174532925199433;
                pos[0] = pos[0].wrapping_sub((radians.cos() * -96.).trunc() as i32);
                let sine = if angle >= 180 {
                    f64::from(angle - 180) * 0.0174532925199433
                } else {
                    radians
                };
                pos[1] = pos[1].wrapping_add((sine.sin() * 16.).trunc() as i32);
                let id = 38
                    + if angle >= 180 {
                        3 - i32::from(phase % 1000 < 500)
                    } else {
                        i32::from(phase % 1000 >= 500)
                    };
                draw(&mut out, scene, id, pos, blend(None, true))?;
            }
            if time >= 8000 {
                draw(&mut out, scene, 45, center(scene, 45)?, blend(None, true))?;
            } else if time >= 7000 {
                let delta = time - 7000;
                let size = dimensions(&image(scene, 45)?)?;
                ensure!(
                    size[0] <= 800 && size[1] <= 600,
                    "native Logo centered image extent"
                );
                let target = [800 - delta * (800 - size[0]) / 1000, size[1] * delta / 1000];
                if target[0] > 0 && target[1] > 0 {
                    // Graph128 samples nearest with rounded16.16 increments,
                    // independently of the bilinear background/snow profile.
                    let step = std::array::from_fn::<_, 2, _>(|axis| {
                        let value = f64::from(size[axis]) / f64::from(target[axis]);
                        let whole = value.round_ties_even();
                        whole as i64 * 65536 + ((value - whole) * 65536.).trunc() as i64
                    });
                    let input=ImageInput::Process {input:Box::new(image(scene,45)?),effects:serde_json::to_vec(&json!([
                        {"kind":"canvas_margins","margins":[0,0,800-size[0],600-size[1]],"fill":[0,0,0,0]},
                        {"kind":"piecewise_resample","spec":{"regions":[{"destination_origin":[0,0],"destination_step":[1,1],"size":target,"source_origin_q16":[0,0],"source_step_q16":step}],
                        "source_weight256":256,"destination_weight256":0,"alpha":"weighted"}}
                    ]))?.into()};
                    put(
                        &mut out,
                        input,
                        target,
                        [(800 - target[0] as i32) / 2, (600 - target[1] as i32) / 2],
                        blend(None, true),
                    );
                }
            }
        }
        4 => {
            draw(&mut out, scene, 28, [0, 0], ImageCompositeMath::CopyRgb)?;
            let fast = (3600..4800).contains(&time);
            let id = 24 + 3 - (((if fast { 2 * time } else { time }) / 300) & 3) as i32;
            let angle = f64::from(360 * time / 1200) * 0.0174532925199433;
            let pos = if fast {
                [0, 0]
            } else {
                [
                    (angle.cos() * 32.).trunc() as i32,
                    (angle.sin() * 8.).trunc() as i32,
                ]
            };
            draw(&mut out, scene, id, pos, blend(None, true))?;
            for p in scene.particles.iter() {
                check()?;
                let phase = (time + 8000) % p.duration;
                draw(
                    &mut out,
                    scene,
                    100 + p.texture,
                    point(p, phase),
                    blend(None, true),
                )?;
            }
            if time >= 8000 {
                draw(&mut out, scene, 45, center(scene, 45)?, blend(None, true))?;
            } else if time >= 7000 {
                draw(
                    &mut out,
                    scene,
                    45,
                    center(scene, 45)?,
                    blend(Some((255 * (time - 7000) / 1000) as u8), true),
                )?;
            }
        }
        _ => anyhow::bail!("native Logo unknown season"),
    }
    let alpha = if time < 1500 {
        255 - 255 * time / 1500
    } else if time >= 8500 {
        255 * (time - 8500) / 2500
    } else {
        0
    };
    if alpha != 0 {
        put(
            &mut out,
            ImageInput::Solid {
                size: [800, 600],
                color: [255; 4],
            },
            [800, 600],
            [0, 0],
            blend(Some(alpha as u8), false),
        );
    }
    check()?;
    Ok(out.into())
}
