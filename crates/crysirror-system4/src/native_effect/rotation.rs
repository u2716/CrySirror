// GPL-2.0-or-later. Complete native GLEffectCopy A420/A5F0/A7B0/A980
//and3730/39E0 C + raw assembly read. No raster loop lives in the adapter.
//! Native clockwise/anticlockwise rotation timing and scanline coefficients.
use super::*;
pub(super) fn recognizes(kind: i32) -> bool {
    (20..=23).contains(&kind)
}
pub(super) fn validate(kind: i32, size: [u32; 2], duration: u32) -> Result<()> {
    ensure!(recognizes(kind) && duration > 0, "native rotate profile");
    ensure!(
        size.iter().all(|n| (1..=8192).contains(n))
            && u64::from(size[0]) * u64::from(size[1]) <= 4_194_304,
        "native rotate canvas budget"
    );
    Ok(())
}
fn signed_word(value: f64) -> Result<i64> {
    ensure!(
        value.is_finite() && value.abs() < i64::MAX as f64,
        "native rotate fixed coefficient"
    );
    //Native helper casts through signed64 then stores the low32 bits. This
    //is defined coordinate arithmetic, never an unchecked host address.
    Ok(i64::from(value.trunc() as i64 as i32))
}
pub(super) fn sample(
    kind: i32,
    time: u32,
    duration: u32,
    old: Arc<dyn ImageResource>,
    new: Arc<dyn ImageResource>,
    previous: Arc<dyn ImageResource>,
) -> Result<ImageInput> {
    let size = old.size();
    validate(kind, size, duration)?;
    ensure!(
        new.size() == size && previous.size() == size && time < duration,
        "native rotate frame/clock"
    );
    let incoming = kind == 21 || kind == 23;
    //The original performs integer360*time, unsigned-to-FPU conversion,
    //then stores each angle/scale argument as binary32 before trig.
    let degrees = f64::from(time.wrapping_mul(360)) / f64::from(duration);
    let angle = if kind >= 22 { 360. - degrees } else { degrees } as f32;
    let fraction = f64::from(time) / f64::from(duration);
    let scale = if incoming { fraction } else { 1. - fraction } as f32;
    //The helper's early return preserves the previously presented native
    //destination. It does not clear, resample or switch to the new scene.
    if scale < 0.01 {
        return Ok(ImageInput::Resource(previous));
    }
    let (rows, column_step) = coefficients(size, angle, scale)?;
    let [w, h] = size;
    let (base, source) = if incoming { (old, new) } else { (new, old) };
    Ok(ImageInput::Process{input:Box::new(ImageInput::Raster{destination:Box::new(ImageInput::Resource(base)),source:Box::new(ImageInput::Resource(source))}),effects:serde_json::to_vec(&json!([{"kind":"scanline_affine_nearest","spec":{
        "rectangle":[0,0,w,h],"row_origins":rows,"column_step":column_step,"fractional_bits":16,"coordinate_wrap":"i32","math":"copy_rgb"
    }}]))?.into()})
}

///Caller supplies native binary32 angle/scale; only O(height) scalar policy
///is retained here. The shared image worker performs all sampling.
pub(super) fn coefficients(
    size: [u32; 2],
    angle: f32,
    scale: f32,
) -> Result<(Vec<[i64; 2]>, [i32; 2])> {
    ensure!(
        scale >= 0.01 && scale.is_finite() && angle.is_finite(),
        "native rotated sample scale/angle"
    );
    let theta = f64::from(angle) * 0.017453292;
    //Portable binary64 trig is explicit; x87 transcendental bit identity is
    //not asserted. Per-row truncation and i32 wrapping remain exact policy.
    let c = theta.cos();
    let s = theta.sin();
    let inv = 1. / f64::from(scale);
    let column_step = [
        signed_word(c * inv * 65536.)? as i32,
        signed_word(s * inv * 65536.)? as i32,
    ];
    let [w, h] = size.map(|v| v as i32);
    let mut rows = Vec::with_capacity(h as usize);
    for row in 0..h {
        let y = f64::from(h / -2 + row);
        rows.push([
            signed_word(
                ((f64::from(w / -2) * c - s * y) / f64::from(scale) + f64::from(w / 2)) * 65536.,
            )?,
            signed_word(
                ((c * y - s * f64::from(w / 2)) / f64::from(scale) + f64::from(h / 2)) * 65536.,
            )?,
        ]);
    }
    Ok((rows, column_step))
}
