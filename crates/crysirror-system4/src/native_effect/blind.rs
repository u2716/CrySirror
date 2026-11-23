// GPL-2.0-or-later. Complete original GLEffectCopy5AF0/5D30/5F70 and
// RGB-copy35B0 read statically. Only native integer strip/reveal policy.
use super::*;
pub(super) fn validate(kind: i32, size: [u32; 2], duration: u32) -> Result<()> {
    ensure!(
        matches!(kind, 7 | 8 | 9) && duration > 0,
        "native blind transition profile"
    );
    for (axis, length) in size.into_iter().enumerate() {
        if kind == 9 || axis == usize::from(kind == 7) {
            ensure!(
                (u64::from(length) + 224) * u64::from(duration) <= u32::MAX as u64,
                "native blind timeline multiply exceeds monotonic defined profile"
            );
        }
    }
    Ok(())
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
        "native blind frame/clock"
    );
    validate(kind, size, duration)?;
    let mut output = ImageInput::Resource(old);
    //Type9 is the union of15 vertical AND15 horizontal strips, not225
    //intersections. At most32 immutable native-copy declarations are needed.
    for vertical in [false, true] {
        if kind != 9 && vertical != (kind == 7) {
            continue;
        }
        let length = size[usize::from(vertical)] as i32;
        let origin = ((((length as u64 + 224) * u64::from(time) / u64::from(duration) + 15) & !15)
            as i32)
            - 224;
        let rect = |start: i32, count: i32| {
            if vertical {
                [0, start, size[0] as i32, count]
            } else {
                [start, 0, count, size[1] as i32]
            }
        };
        let prefix = origin.max(0).min(length);
        if prefix > 0 {
            let r = rect(0, prefix);
            output = layer(
                output,
                ImageInput::Resource(new.clone()),
                r,
                [r[0], r[1]],
                ImageCompositeMath::CopyRgb,
            );
        }
        for band in 0..15 {
            let at = origin + band * 16;
            if at < 0 || at >= length {
                continue;
            }
            let count = (15 - band).min(length - at);
            let r = rect(at, count);
            output = layer(
                output,
                ImageInput::Resource(new.clone()),
                r,
                [r[0], r[1]],
                ImageCompositeMath::CopyRgb,
            );
        }
    }
    Ok(output)
}
