// GPL-2.0-or-later. Complete native11E80/17190/172B0/17480/11E50
//and1F80 blending, radius generation and pixel-conversion asm read statically.
//! Native53's radius-angle curve; shared Image owns geometry and pixel work.
use super::*;
pub(super) fn validate(duration: u32) -> Result<()> {
    ensure!(
        duration > 1,
        "native radial Effect duration1 divides by zero"
    );
    Ok(())
}
pub(super) fn sample(
    time: u32,
    duration: u32,
    old: Arc<dyn ImageResource>,
    new: Arc<dyn ImageResource>,
) -> Result<ImageInput> {
    validate(duration)?;
    let size = old.size();
    ensure!(
        time < duration && size == new.size(),
        "native radial Effect clock/extent"
    );
    let [w, h] = size.map(|n| n as i32);
    let half = duration / 2;
    let amplitude = if time < half {
        f64::from(time) / f64::from(half)
    } else {
        1. - f64::from(time - half) / f64::from(half)
    } as f32;
    let radius = (u64::from(size[0] / 2).pow(2) + u64::from(size[1] / 2).pow(2)).isqrt() + 1;
    ensure!(radius <= 65536, "native radial Effect radius table budget");
    let angles = (0..radius)
        .map(|r| {
            ((360. * (radius - r) as f64 * f64::from(amplitude) / radius as f64).trunc() as u32
                % 360) as u16
        })
        .collect::<Vec<_>>();
    //Native radius lookup uses (x-w/2,y-h/2), but its rotation vector uses
    //(x-w/2,h/2-1-y). Keep that one-row difference; a conventional centred
    //swirl is not equivalent.17480's magic-double conversion rounds nearest
    //ties-even, then clamps each source coordinate to the source edge.
    let alpha = (255u64 * u64::from(time) / u64::from(duration)) as u32;
    let input = blur::mixed(old, new, alpha)?;
    Ok(ImageInput::Process{input:Box::new(input),effects:serde_json::to_vec(&json!([{
        "kind":"radial_angle_lookup","spec":{"field_origin":[w/2,h/2],"vector_origin":[w/2,h/2-1],
        "source_center":[w/2,h/2],"y_up":true,"angles":angles}
    }]))?.into()})
}
