// GPL-2.0-or-later. Native A050/A230/BF80/C190/E800 and full3CF0
// Q8 point-sampling kernel read statically, including destination-register
// assembly omitted by the decompiler. No raster loop is kept in this VM.
use super::*;
pub(super) fn recognizes(kind: i32) -> bool {
    matches!(kind, 32 | 33 | 43 | 44 | 48)
}
pub(super) fn validate(kind: i32, size: [u32; 2], duration: u32) -> Result<()> {
    ensure!(recognizes(kind) && duration > 0, "native geometry profile");
    if matches!(kind, 32 | 33) {
        ensure!(
            duration > 1,
            "native TV transition duration1 divides by zero"
        );
    }
    if kind != 48 {
        for n in size {
            ensure!(
                u64::from(n) * u64::from(duration) <= u32::MAX as u64,
                "native geometric timeline multiply exceeds defined monotonic profile"
            );
        }
    }
    Ok(())
}
///3CF0 samples integer texels using a separately truncated Q8 origin/step.
///Q8 coordinates are represented exactly by the engine's existing Q16 map.
fn stretch(
    base: ImageInput,
    source: Arc<dyn ImageResource>,
    destination: [i32; 4],
    window: [f32; 4],
) -> Result<ImageInput> {
    let [dx, dy, w, h] = destination;
    if w <= 0 || h <= 0 {
        return Ok(base);
    }
    let size = source.size();
    ensure!(
        window.iter().all(|v| v.is_finite() && *v >= 0.) && window[2] > 0. && window[3] > 0.,
        "native point-stretch source window"
    );
    let origins = [window[0], window[1]];
    let extents = [window[2], window[3]];
    let output = [w, h];
    let origin = std::array::from_fn::<_, 2, _>(|axis| {
        (f64::from(origins[axis]) * 256.).trunc() as i64 * 256
    });
    let step = std::array::from_fn::<_, 2, _>(|axis| {
        (f64::from(extents[axis]) / f64::from(output[axis]) * 256.).trunc() as i64 * 256
    });
    for axis in 0..2 {
        ensure!(
            output[axis] as u32 <= size[axis]
                && origin[axis] >= 0
                && origin[axis] + i64::from(output[axis] - 1) * step[axis]
                    < i64::from(size[axis]) * 65536,
            "native Q8 sample exceeds initialized source profile"
        );
    }
    let input=ImageInput::Process{input:Box::new(ImageInput::Resource(source)),effects:serde_json::to_vec(&json!([{"kind":"piecewise_resample","spec":{
        "regions":[{"destination_origin":[0,0],"destination_step":[1,1],"size":[w,h],"source_origin_q16":origin,"source_step_q16":step}],
        "source_weight256":256,"destination_weight256":0,"alpha":"opaque"}}]))?.into()};
    Ok(layer(
        base,
        input,
        [0, 0, w, h],
        [dx, dy],
        ImageCompositeMath::CopyRgb,
    ))
}
pub(super) fn sample(
    kind: i32,
    time: u32,
    duration: u32,
    old: Arc<dyn ImageResource>,
    new: Arc<dyn ImageResource>,
) -> Result<ImageInput> {
    let size = old.size();
    ensure!(
        size == new.size() && time < duration,
        "native geometry frame/clock"
    );
    validate(kind, size, duration)?;
    let [w, h] = size;
    let full = [0., 0., w as f32, h as f32];
    let black = ImageInput::Solid {
        size,
        color: [0, 0, 0, 255],
    };
    if matches!(kind, 43 | 44) {
        let cut = (u64::from(w) * u64::from(time) / u64::from(duration)) as i32;
        let rest = w as i32 - cut;
        return if kind == 43 {
            stretch(
                stretch(black, new, [0, 0, cut, h as i32], full)?,
                old,
                [cut, 0, rest, h as i32],
                full,
            )
        } else {
            stretch(
                stretch(black, old, [0, 0, rest, h as i32], full)?,
                new,
                [rest, 0, cut, h as i32],
                full,
            )
        };
    }
    if matches!(kind, 32 | 33) {
        let half = duration / 2;
        let extent = if kind == 32 {
            if time < half {
                [
                    w,
                    h - (u64::from(h - 1) * u64::from(time) / u64::from(half)) as u32,
                ]
            } else {
                [
                    w - (u64::from(w - 1) * u64::from(time - half) / u64::from(half)) as u32,
                    1,
                ]
            }
        } else if time < half {
            [(u64::from(w) * u64::from(time) / u64::from(half)) as u32, 1]
        } else {
            [
                w,
                (u64::from(h) * u64::from(time - half) / u64::from(half)) as u32,
            ]
        };
        let destination = [
            ((w - extent[0]) / 2) as i32,
            ((h - extent[1]) / 2) as i32,
            extent[0] as i32,
            extent[1] as i32,
        ];
        return stretch(black, if kind == 32 { old } else { new }, destination, full);
    }
    ensure!(kind == 48, "native geometry sampler family");
    let t = f64::from(time) / f64::from(duration);
    let t = t * t;
    //E800 stores each shrunken source extent and centred origin as binary32
    //before3CF0 converts toQ8. This is source-window zoom, not resized output
    //or the different fourth-power public GL zoom implementation.
    let extent = [
        ((f64::from(w) - f64::from(w) * t) as f32).max(1.),
        ((f64::from(h) - f64::from(h) * t) as f32).max(1.),
    ];
    let window = [
        ((f64::from(w as f32) - f64::from(extent[0])) * 0.5) as f32,
        ((f64::from(h as f32) - f64::from(extent[1])) * 0.5) as f32,
        extent[0],
        extent[1],
    ];
    let base = stretch(black, old, [0, 0, w as i32, h as i32], window)?;
    let alpha = (255u64 * u64::from(time) / u64::from(duration)) as u32;
    let state = S {
        arithmetic: A::Byte255 {
            source: Q::Nearest,
            destination: Q::Nearest,
        },
        constant: [alpha as f32 / 255.; 4],
        write_mask: [true, true, true, false],
        color: C {
            source: F::ConstantAlpha,
            destination: F::OneMinusConstantAlpha,
            equation: E::Add,
        },
        ..Default::default()
    };
    Ok(layer(
        base,
        ImageInput::Resource(new),
        [0, 0, w as i32, h as i32],
        [0, 0],
        ImageCompositeMath::Normalized(state),
    ))
}
