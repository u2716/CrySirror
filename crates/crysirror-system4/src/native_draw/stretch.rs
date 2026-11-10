// GPL-2.0-or-later. Native policy: xsystem4 a85467d DrawGraph.c/draw.c,
// Copyright (C)2019 Nunuhara Cabbage. Rust (C)2026 CrySirror contributors.
//! Original stretch wrappers 40C0..4880 and Graph slots30..32/44..48 were
//! read completely. Sampling/reduction/blending use shared immutable jobs.
use super::*;
use crysirror_core::image_blend::{
    RgbaBlendArithmetic, RgbaBlendComponent, RgbaBlendEquation, RgbaBlendFactor, RgbaBlendState,
    RgbaByteRounding,
};
use serde_json::json;

fn process(input: ImageInput, effect: serde_json::Value) -> Result<ImageInput> {
    Ok(ImageInput::Process {
        input: Box::new(input),
        effects: serde_json::to_vec(&vec![effect])?.into(),
    })
}
// The original nearest kernel splits an extended-precision ratio at its
// nearest-even integer, then truncates the fractional 16.16 part separately.
// floor(ratio*65536) is observably different for e.g. 2/3.
fn step(source: i32, destination: i32) -> i32 {
    let value = f64::from(source) / f64::from(destination);
    let whole = value.round_ties_even();
    ((whole as i64) * 65536 + ((value - whole) * 65536.).trunc() as i64) as i32
}
pub(super) fn operation(
    owner: ClientId,
    name: &[u8],
    values: &[i32],
    bridge: &Bridge,
    cg: &NativeCg,
    native_tables: bool,
) -> Result<Option<(i32, ImageInput, bool)>> {
    if !matches!(
        name,
        b"CopyStretch"
            | b"CopyStretchBlend"
            | b"CopyStretchBlendAMap"
            | b"CopyStretchAMap"
            | b"CopyStretchInterp"
            | b"CopyStretchAMapInterp"
            | b"CopyReduce"
            | b"CopyReduceAMap"
    ) {
        return Ok(None);
    }
    let [dst, dx, dy, dw, dh, src, sx, sy, sw, sh, rest @ ..] = values else {
        bail!("native stretch argument ABI")
    };
    ensure!(
        rest.len() == usize::from(name == b"CopyStretchBlend"),
        "native stretch blend arity"
    );
    ensure!(
        [*dw, *dh, *sw, *sh].iter().all(|v| (1..=8192).contains(v)),
        "native stretch extent budget"
    );
    let sprites = bridge
        .sact
        .as_ref()
        .and_then(|s| s.sprites.as_ref())
        .context("native stretch before SACT")?;
    let source = sprites
        .graphics(*src)
        .context("native stretch missing source")?;
    let destination = sprites
        .graphics(*dst)
        .context("native stretch missing destination")?;
    // The original 1380 clip helper adjusts source/destination ratios using
    // sequential integer divisions. A clipped rectangle is not equivalent to
    // clipping a final resized image. Keep that unaudited profile explicit.
    ensure!(
        *sx >= 0
            && *sy >= 0
            && *dx >= 0
            && *dy >= 0
            && i64::from(*sx) + i64::from(*sw) <= i64::from(source.size[0])
            && i64::from(*sy) + i64::from(*sh) <= i64::from(source.size[1])
            && i64::from(*dx) + i64::from(*dw) <= i64::from(destination.size[0])
            && i64::from(*dy) + i64::from(*dh) <= i64::from(destination.size[1]),
        "native stretch ratio-changing clipping profile is unsupported"
    );
    let alpha_only = matches!(
        name,
        b"CopyStretchAMap" | b"CopyStretchAMapInterp" | b"CopyReduceAMap"
    );
    if alpha_only || name == b"CopyStretchBlendAMap" {
        ensure!(
            source.alpha && (!alpha_only || destination.alpha),
            "native stretch alpha plane absent"
        );
    }
    let source_input = cg.sprite_input(owner, *src, bridge)?;
    let size = [*dw as u32, *dh as u32];
    let sampled = if matches!(name, b"CopyStretchInterp" | b"CopyStretchAMapInterp") {
        process(
            source_input,
            json!({"kind":"rect_resample_byte256","spec":{"size":size,"source":[sx,sy,sw,sh]}}),
        )?
    } else if matches!(name, b"CopyReduce" | b"CopyReduceAMap") {
        process(
            source_input,
            json!({"kind":"box_reduce","spec":{"size":size,"source":[sx,sy,sw,sh],"window":[1,1],
            "anchors":{"kind":"area"},"arithmetic":{"kind":"mean"},"alpha":"aggregate"}}),
        )?
    } else {
        process(
            ImageInput::Raster {
                destination: Box::new(ImageInput::Solid {
                    size,
                    color: [0; 4],
                }),
                source: Box::new(source_input),
            },
            json!({"kind":"fixed_affine_nearest","spec":{"rectangle":[0,0,dw,dh],"origin":[0,0],
                "linear":[[step(*sw,*dw),0],[0,step(*sh,*dh)]],"translation":[i64::from(*sx)*65536,i64::from(*sy)*65536],
                "fractional_bits":16,"math":"copy"}}),
        )?
    };
    let math = if alpha_only {
        ImageCompositeMath::CopyAlpha
    } else if matches!(name, b"CopyStretchBlend" | b"CopyStretchBlendAMap") {
        let factor = if name == b"CopyStretchBlend" {
            RgbaBlendFactor::ConstantAlpha
        } else {
            RgbaBlendFactor::SourceAlpha
        };
        let complement = if name == b"CopyStretchBlend" {
            RgbaBlendFactor::OneMinusConstantAlpha
        } else {
            RgbaBlendFactor::OneMinusSourceAlpha
        };
        let mut blend = RgbaBlendState {
            write_mask: [true, true, true, false],
            color: RgbaBlendComponent {
                source: factor,
                destination: complement,
                equation: RgbaBlendEquation::Add,
            },
            ..Default::default()
        };
        if let Some(alpha) = rest.first() {
            blend.constant = [(*alpha).clamp(0, 255) as f32 / 255.; 4];
        }
        if native_tables {
            blend.arithmetic = RgbaBlendArithmetic::Byte255 {
                source: RgbaByteRounding::Floor,
                destination: RgbaByteRounding::Floor,
            };
        }
        ImageCompositeMath::Normalized(blend)
    } else {
        ImageCompositeMath::CopyRgb
    };
    Ok(Some((
        *dst,
        composite(
            cg.sprite_input(owner, *dst, bridge)?,
            sampled,
            [0, 0, *dw, *dh],
            [*dx, *dy],
            math,
        ),
        alpha_only,
    )))
}
