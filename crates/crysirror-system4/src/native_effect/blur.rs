// GPL-2.0-or-later. Full native GLEffectCopy02560/1F80/2110/1900,
// timelines5940/6CC0/7220/74D0/7740/E1D0/E490 and116C0/11A90 read.
//! Native timing/coefficient policy; shared workers own averaging/sampling.
use super::*;
pub(super) fn recognizes(kind: i32) -> bool {
    matches!(kind, 6 | 11 | 19 | 40 | 41 | 50 | 51)
}
pub(super) fn immediate(kind: i32, duration: u32) -> bool {
    matches!(kind, 11 | 19 | 40 | 41) && duration <= 1
}
pub(super) fn validate(kind: i32, size: [u32; 2], duration: u32) -> Result<()> {
    ensure!(
        recognizes(kind) && duration > 0,
        "native blur effect profile"
    );
    if kind == 6 {
        ensure!(
            duration > 1,
            "native mosaic duration1 divides by zero in original profile"
        );
    }
    if matches!(kind, 11 | 19) && duration > 1 {
        let axis = size[usize::from(kind == 19)];
        let log = axis.next_power_of_two().trailing_zeros();
        let half = duration / 2;
        let exponent = u64::from(log) * u64::from(duration - half) / u64::from(half) + 1;
        ensure!(
            exponent <= 15,
            "native blur even-window width exceeds shared bounded profile"
        );
    }
    if matches!(kind, 50 | 51) || kind == 40 && duration > 1 {
        ensure!(
            size[0] >= 4,
            "native horizontal blur tiny axis has undefined source traversal"
        );
    }
    if matches!(kind, 50 | 51) || kind == 41 && duration > 1 {
        ensure!(
            size[1] >= 4,
            "native vertical blur tiny axis has undefined source traversal"
        );
    }
    Ok(())
}
fn process(input: ImageInput, effects: serde_json::Value) -> Result<ImageInput> {
    Ok(ImageInput::Process {
        input: Box::new(input),
        effects: serde_json::to_vec(&effects)?.into(),
    })
}
fn nearest(base: ImageInput, new: Arc<dyn ImageResource>, alpha: u32) -> ImageInput {
    let size = new.size();
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
    layer(
        base,
        ImageInput::Resource(new),
        [0, 0, size[0] as i32, size[1] as i32],
        [0, 0],
        ImageCompositeMath::Normalized(state),
    )
}
pub(super) fn mixed(
    old: Arc<dyn ImageResource>,
    new: Arc<dyn ImageResource>,
    alpha: u32,
) -> Result<ImageInput> {
    let size = old.size();
    if size[0] < 4 {
        return Ok(nearest(ImageInput::Resource(old), new, alpha));
    }
    //1F80's original default MMX2110 handles aligned four-pixel groups with
    //independent /256 floors and complementary weights summing255. Its
    //scalar tail instead uses two nearest /255 table lookups. Native full
    //RGB24 frame rows are four-byte aligned; caller coordinates are zero.
    let input = process(
        ImageInput::Transition {
            from: old.clone(),
            to: new.clone(),
            mask: None,
        },
        json!([{"kind":"transition","spec":{
        "progress":0.0,"softness":0.0,"blur":0.0,"fixed_weighted":{"left_weight":255-alpha,"right_weight":alpha,
        "fractional_bits":8,"rounding":"separate_terms","alpha":"opaque"}}}]),
    )?;
    let bulk = size[0] / 4 * 4;
    if bulk == size[0] {
        return Ok(input);
    }
    let scalar = nearest(ImageInput::Resource(old), new, alpha);
    Ok(layer(
        input,
        scalar,
        [bulk as i32, 0, (size[0] - bulk) as i32, size[1] as i32],
        [bulk as i32, 0],
        ImageCompositeMath::CopyRgb,
    ))
}
fn radius(wanted: u32, length: u32) -> Result<u32> {
    ensure!(length >= 4, "native blur undefined tiny source axis");
    Ok(if wanted < 1 {
        1
    } else if wanted >= length / 2 {
        length / 2 - 1
    } else {
        wanted
    })
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
        new.size() == size && time < duration,
        "native blur frame/clock"
    );
    validate(kind, size, duration)?;
    let alpha = (255u64 * u64::from(time) / u64::from(duration)) as u32;
    if kind == 6 {
        let half = duration / 2;
        let phase = if time >= half { duration - time } else { time };
        let cell = (((96u64 * u64::from(phase) / u64::from(half)) as u32) & !1) + 1;
        ensure!(cell <= u16::MAX as u32, "native mosaic cell budget");
        return process(
            nearest(ImageInput::Resource(old), new, alpha),
            json!([{"kind":"nearest_cell_sample","spec":{
            "cell":[cell,cell],"origin":[(size[0]/2)%cell,(size[1]/2)%cell]}}]),
        );
    }
    if matches!(kind, 11 | 19) {
        if duration <= 1 {
            return Ok(ImageInput::Resource(new));
        }
        let vertical = kind == 19;
        let axis = size[usize::from(vertical)];
        let half = duration / 2;
        let phase = if time >= half { duration - time } else { time };
        let log = axis.next_power_of_two().trailing_zeros();
        let exponent = u64::from(log) * u64::from(phase) / u64::from(half) + 1;
        ensure!(exponent <= 15, "native even blur window budget");
        let side = 1u32 << (exponent as u32 - 1);
        return process(
            mixed(old, new, alpha)?,
            json!([{"kind":"box_blur_span","spec":{"vertical":vertical,"before":side,"after":side}}]),
        );
    }
    if matches!(kind, 40 | 41) {
        if duration <= 1 {
            return Ok(ImageInput::Resource(new));
        }
        let vertical = kind == 41;
        let length = size[usize::from(vertical)];
        let half = duration / 2;
        //Native HD return branch subtracts two separately truncated terms;
        //combining them as a single triangular fraction changes odd durations.
        let wanted = if time >= half {
            length / 2 - (u64::from(length / 2) * u64::from(time - half) / u64::from(half)) as u32
        } else {
            (u64::from(time) * u64::from(length / 2) / u64::from(half)) as u32
        };
        let radius = radius(wanted, length)?;
        let radii = if vertical { [0, radius] } else { [radius, 0] };
        return process(
            mixed(old, new, alpha)?,
            json!([{"kind":"box_blur_clamped","spec":{"radii":radii,"passes":1}}]),
        );
    }
    let wanted = 48u64 * u64::from(time) / u64::from(duration);
    let wanted = if kind == 51 && wanted > 24 {
        48 - wanted
    } else {
        wanted
    };
    let wanted = if kind == 50 { wanted + 1 } else { wanted } as u32;
    let rx = radius(wanted, size[0])?;
    let ry = radius(wanted, size[1])?;
    let effects = json!([
        {"kind":"box_blur_clamped","spec":{"radii":[rx,0],"passes":1}},
        {"kind":"box_blur_clamped","spec":{"radii":[0,ry],"passes":1}}
    ]);
    if kind == 50 {
        //E1D0 blurs old first, then in-place table-blends the new frame by
        //floor(alpha²/255). It is not a fade-to-black alias despite its name.
        Ok(nearest(
            process(ImageInput::Resource(old), effects)?,
            new,
            alpha * alpha / 255,
        ))
    } else {
        ensure!(kind == 51, "native blur sampler family");
        //E490 first performs the nearest /255 table blend, then horizontal
        //and vertical integer averages with a truncation between the passes.
        process(nearest(ImageInput::Resource(old), new, alpha), effects)
    }
}
