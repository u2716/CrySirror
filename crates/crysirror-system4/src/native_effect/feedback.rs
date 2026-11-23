// GPL-2.0-or-later. Complete native GLEffectCopy62B0/6500,2860/2E70,
//19F0/1B80 and constructor01000 LUT initialization read statically.
//! Native feedback timing. Sampling, tables and blending remain shared jobs.
use super::*;
pub(super) fn recognizes(kind: i32) -> bool {
    matches!(kind, 10 | 30 | 52)
}
pub(super) fn sample(
    kind: i32,
    time: u32,
    duration: u32,
    new: Arc<dyn ImageResource>,
    previous: Arc<dyn ImageResource>,
) -> Result<ImageInput> {
    ensure!(
        matches!(kind, 10 | 30) && duration > 0 && time < duration && new.size() == previous.size(),
        "native feedback effect profile/clock/extent"
    );
    let size = new.size();
    let first = duration * 35 / 100;
    let second = duration * 70 / 100;
    if time >= second {
        //19F0 uses the actual current destination as its left source. The
        //default MMX body is signed-deltaQ7; its scalar remainder isQ8.
        //The existing native crossfade plan implements those same products.
        let alpha = 96 * (time - second) / (duration - second);
        return super::sample(1, alpha, 255, previous, new, true);
    }
    let (phase, span, second_phase) = if time < first {
        (time, first, false)
    } else {
        (second - time, second - first, true)
    };
    ensure!(span > 0, "native feedback empty phase");
    //Both native kernels store the curve result as binary32. Keep the
    //original binary32 .2 constant promoted to the FPU computation.
    let ratio = f64::from(phase * 5) / f64::from(span) * f64::from(0.2f32);
    let scale = (ratio * ratio * 5. + 1.) as f32;
    let (rows, column_step, bits) = if kind == 10 {
        //2860 has separately truncated Q8 origins and step, then advances
        //each row by that quantized step. Its scale is always at least1;
        //the native upper-edge clamp cannot change these admitted samples.
        let inv = 1. / f64::from(scale);
        let origin =
            size.map(|n| ((f64::from(n / 2) - f64::from(n / 2) * inv) * 256.).trunc() as i64);
        let step = (inv * 256.).trunc() as i32;
        let rows = (0..size[1])
            .map(|y| [origin[0], origin[1] + i64::from(y) * i64::from(step)])
            .collect();
        (rows, [step, 0], 8)
    } else {
        let angle = if second_phase {
            (f64::from(time - first) * 180. / f64::from(second - first) + 180.) as f32
        } else {
            (f64::from(time * 180) / f64::from(first)) as f32
        };
        let (rows, step) = rotation::coefficients(size, angle, scale)?;
        (rows, step, 16)
    };
    composite_sample(
        ImageInput::Resource(previous),
        new,
        rows,
        column_step,
        bits,
        true,
    )
}

///52 keeps a separate scratch surface. It is identical to the display during
///the first80%; the caller freezes that lease before the final crossfade.
///Complete67F0/30F0 raw arguments and initialization copy were inspected.
pub(super) fn sample_double(
    time: u32,
    duration: u32,
    new: Arc<dyn ImageResource>,
    previous: Arc<dyn ImageResource>,
    anchor: Option<Arc<dyn ImageResource>>,
) -> Result<ImageInput> {
    ensure!(
        duration > 0 && time < duration && new.size() == previous.size(),
        "native double rotate frame/clock"
    );
    let first = duration * 40 / 100;
    let second = duration * 80 / 100;
    if time >= second {
        let anchor = anchor.context("native double rotate frozen scratch absent")?;
        let alpha = 255 * (time - second) / (duration - second);
        return super::sample(1, alpha, 255, anchor, new, true);
    }
    let (scale, angle) = if time < first {
        let ratio = f64::from(time * 4) / f64::from(first) * 0.25;
        (
            (ratio * ratio * 4.) as f32,
            (f64::from(time * 180) / f64::from(first)) as f32,
        )
    } else {
        let span = second - first;
        ensure!(span > 0, "native double rotate empty phase");
        let ratio = f64::from((second - time) * 4) / f64::from(span) * 0.25;
        (
            (ratio * ratio * 4. + 1.) as f32,
            ((f64::from(time - first) / f64::from(span) + 1.) * 180.) as f32,
        )
    };
    if scale < 0.01 {
        return Ok(ImageInput::Resource(previous));
    }
    let size = new.size();
    let mut base = ImageInput::Resource(previous);
    //30F0 applies these two angles sequentially to the same staged scratch,
    //sampling the immutable new image both times. Its second angle is stored
    //as binary32 after multiplying the first binary32 angle by-2.
    for angle in [angle, angle * -2.] {
        let (rows, column_step) = rotation::coefficients(size, angle, scale)?;
        base = composite_sample(base, new.clone(), rows, column_step, 16, false)?;
    }
    Ok(base)
}

fn composite_sample(
    base: ImageInput,
    new: Arc<dyn ImageResource>,
    rows: Vec<[i64; 2]>,
    column_step: [i32; 2],
    bits: u8,
    shift4: bool,
) -> Result<ImageInput> {
    let size = new.size();
    let table = (0u16..=255).map(|n| (n >> 4) as u8).collect::<Vec<_>>();
    let effects = if shift4 {
        json!([
            {"kind":"rgb_lookup","spec":{"red":table,"green":table,"blue":table,"profile":"straight_rgba"}},
            {"kind":"override_alpha","alpha":255}
        ])
    } else {
        json!([{"kind":"override_alpha","alpha":255}])
    };
    let source = ImageInput::Process {
        input: Box::new(ImageInput::Resource(new)),
        effects: serde_json::to_vec(&effects)?.into(),
    };
    let sampled = ImageInput::Process {
        input: Box::new(ImageInput::Raster {
            destination: Box::new(ImageInput::Solid {
                size,
                color: [0; 4],
            }),
            source: Box::new(source),
        }),
        effects: serde_json::to_vec(&json!([{"kind":"scanline_affine_nearest","spec":{
            "rectangle":[0,0,size[0],size[1]],"row_origins":rows,"column_step":column_step,
            "fractional_bits":bits,"coordinate_wrap":"i32","math":"copy"
        }}]))?
        .into(),
    };
    let state = S {
        arithmetic: A::Byte255 {
            source: Q::Nearest,
            destination: Q::Nearest,
        },
        constant: [240. / 255.; 4],
        source_alpha_key: Some(0),
        write_mask: [true, true, true, false],
        color: C {
            source: if shift4 {
                F::One
            } else {
                F::OneMinusConstantAlpha
            },
            destination: F::ConstantAlpha,
            equation: E::Add,
        },
        ..Default::default()
    };
    //Constructor01000 LUT240 is nearest(destination*240/255);2860/2E70
    //add source>>4.30F0 instead adds nearest(source*15/255). In both
    //cases the maximum is240+15, with no saturation/wrapping ambiguity.
    //A rotated sample outside source keeps ALL previous destination bytes:
    //alpha0 discards that fragment before the destination fade contribution.
    Ok(layer(
        base,
        sampled,
        [0, 0, size[0] as i32, size[1] as i32],
        [0, 0],
        ImageCompositeMath::Normalized(state),
    ))
}
