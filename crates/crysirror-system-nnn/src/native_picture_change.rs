// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! CHANGE40 software32, effectchange.cpp/Picture.cpp. Curves and native
//! count/parameter mutations are kept here; shared image jobs own all pixels.
use crate::native_picture_clip::CharaClip;
use anyhow::{Result, ensure};
use crysirror_image::{packed_pair_blend::PackedPairBlend, temporal_pattern::PatternRng};

#[derive(Clone, Copy, Debug)]
pub(crate) struct ChangePicture {
    pub size: [u32; 2],
    pub clip: CharaClip,
    pub assigned: bool,
    pub has_mask: bool,
}
pub(crate) fn parameters(values: &[i32]) -> Option<[i32; 8]> {
    // SetParam rejects fewer than two parameters without changing the effect.
    if values.len() < 2 {
        return None;
    }
    let mut p = [0; 8];
    for i in 0..8 {
        // Original off-by-one default tests leave omitted para6/7 as
        // uninitialised stack data. Define only those missing words as zero.
        p[i] = values.get(i).copied().unwrap_or(match i {
            0 => 20,
            1 => 13,
            2 => 100,
            3 => p[2],
            _ => 0,
        });
    }
    Some(p)
}
pub(crate) fn validate(p: &[i32; 8], frames: i32) -> Result<()> {
    crate::native_curve::validate(frames, p[7], p[6])?;
    crate::native_curve::validate(frames, p[5], p[4])
}
pub(crate) fn percentages(
    p: &[i32; 8],
    count: i32,
    frames: i32,
    random: &mut PatternRng,
) -> Result<[i32; 2]> {
    let trans =
        crate::native_curve::value_with_random(count, frames, p[3], p[2], p[7], p[6], random)?;
    let change = crate::native_curve::value_with_random(count, frames, 100, 0, p[5], p[4], random)?;
    Ok([
        (change.wrapping_mul(trans) / 100).clamp(0, 100),
        (100i32.wrapping_sub(change).wrapping_mul(trans) / 100).clamp(0, 100),
    ])
}
pub(crate) fn count_is_max(
    p: &mut [i32; 8],
    count: &mut i32,
    frames: i32,
    random: &mut PatternRng,
) -> Result<bool> {
    *count = frames;
    let [a, b] = percentages(p, *count, frames, random)?;
    // allEffect.cpp ChangeEffectCommandOnly490 has its command assignment
    // commented out: the function is explicitly just return TRUE. Calls here
    // must not invent BUFFER/NOP/TRANS changes, exchanges or sibling writes.
    if a != 0 && b != 0 {
        return Ok(false);
    }
    if a == 0 {
        if b != 0 && b != 100 {
            p[1] = b;
        }
    } else if a != 100 {
        p[1] = a;
    }
    Ok(true)
}

/// CPicture::ClipEtc keeps the original destination unchanged when fixing a
/// negative source origin. Source offsets for the other picture follow only
/// actual destination movement, not this private source correction.
fn clip_etc(
    destination: &mut [i64; 2],
    source: &mut [i64; 2],
    size: &mut [i64; 2],
    picture: [u32; 2],
    screen: [i64; 4],
) -> bool {
    let ends = [destination[0] + size[0], destination[1] + size[1]];
    if ends[0] <= screen[0]
        || ends[1] <= screen[1]
        || destination[0] > screen[2]
        || destination[1] > screen[3]
    {
        return false;
    }
    for axis in 0..2 {
        if destination[axis] < screen[axis] {
            let delta = screen[axis] - destination[axis];
            destination[axis] += delta;
            source[axis] += delta;
            size[axis] -= delta;
        }
        if source[axis] < 0 {
            size[axis] += source[axis];
            source[axis] = 0;
        }
        if ends[axis] > screen[axis + 2] {
            size[axis] -= ends[axis] - screen[axis + 2];
        }
        if source[axis] + size[axis] > i64::from(picture[axis]) {
            size[axis] = i64::from(picture[axis]) - source[axis];
        }
    }
    size.iter().all(|n| *n > 0)
        && source
            .iter()
            .zip(picture)
            .all(|(n, max)| *n < i64::from(max))
}

pub(crate) fn draw_plan(
    p: &[i32; 8],
    count: i32,
    frames: i32,
    source: [i32; 4],
    destination: [i32; 2],
    pictures: [Option<ChangePicture>; 2],
    screen_clip: [i32; 4],
    random: &mut PatternRng,
) -> Result<Option<PackedPairBlend>> {
    // Print consumes both curve draws before inspecting picture IDs/resources.
    let percentages = percentages(p, count, frames, random)?;
    let [Some(first), Some(second)] = pictures else {
        return Ok(None);
    };
    if !first.assigned
        || !second.assigned
        || p[1] == -1
        || !second.has_mask
        || first.size != second.size
    {
        return Ok(None);
    }
    ensure!(
        first.size.iter().all(|n| (1..=16384).contains(n)),
        "NNN CHANGE picture dimensions"
    );
    ensure!(
        source
            .iter()
            .chain(&destination)
            .chain(&screen_clip)
            .all(|n| n.unsigned_abs() <= 1_000_000),
        "NNN CHANGE coordinates"
    );
    if source[2] <= 0
        || source[3] <= 0
        || screen_clip[0] >= screen_clip[2]
        || screen_clip[1] >= screen_clip[3]
    {
        return Ok(None);
    }
    let Some((rect_a, at_a)) = first.clip.source_clip(source, destination)? else {
        return Ok(None);
    };
    let Some((rect_b, at_b)) = second.clip.source_clip(source, destination)? else {
        return Ok(None);
    };
    // Both independent CharaClip calls must succeed. Their destination union
    // is then clipped to screen and both complete source images in order.
    let left = at_a[0].min(at_b[0]);
    let top = at_a[1].min(at_b[1]);
    let right = (at_a[0] + rect_a[2]).max(at_b[0] + rect_b[2]);
    let bottom = (at_a[1] + rect_a[3]).max(at_b[1] + rect_b[3]);
    let mut target = [i64::from(left), i64::from(top)];
    let mut size = [i64::from(right - left), i64::from(bottom - top)];
    let origin = [
        i64::from(source[0]) + target[0] - i64::from(destination[0]),
        i64::from(source[1]) + target[1] - i64::from(destination[1]),
    ];
    let mut a = origin;
    let mut b = origin;
    let before = target;
    if !clip_etc(
        &mut target,
        &mut a,
        &mut size,
        first.size,
        screen_clip.map(i64::from),
    ) {
        return Ok(None);
    }
    for axis in 0..2 {
        b[axis] += target[axis] - before[axis];
    }
    let before = target;
    if !clip_etc(
        &mut target,
        &mut b,
        &mut size,
        second.size,
        screen_clip.map(i64::from),
    ) {
        return Ok(None);
    }
    for axis in 0..2 {
        a[axis] += target[axis] - before[axis];
    }
    let spec = PackedPairBlend {
        sources: [
            [a[0] as i32, a[1] as i32, size[0] as i32, size[1] as i32],
            [b[0] as i32, b[1] as i32, size[0] as i32, size[1] as i32],
        ],
        destination: target.map(|n| n as i32),
        weights256: percentages.map(|n| ((n * 256 + 50) / 100) as u16),
        use_masks: first.has_mask,
    };
    spec.validate()?;
    Ok(Some(spec))
}

pub(crate) fn image_input(
    frame: &crate::native_picture_effect::PictureEffectFrame,
    p: &[i32; 8],
    base: crysirror_core::image_processing::ImageInput,
    random: &mut PatternRng,
) -> Result<Option<crysirror_core::image_processing::ImageInput>> {
    use crysirror_core::image_processing::ImageInput;
    use crysirror_image::effects::ImageEffect;
    let first = frame.bank.picture(frame.slot);
    let second = if p[1] == -1 {
        None
    } else {
        frame.bank.picture(p[1].clamp(0, frame.bank_limit - 1))
    };
    let metadata = |p: Option<&crate::native_picture_bank::BankPicture>| {
        p.map(|p| ChangePicture {
            size: p.picture.raw.size(),
            clip: CharaClip::from_bounds(p.picture.source_clip),
            assigned: p.assigned,
            has_mask: p.picture.has_mask,
        })
    };
    let source = [
        frame.rectangles.source[0],
        frame.rectangles.source[1],
        frame.rectangles.destination[2],
        frame.rectangles.destination[3],
    ];
    let destination = [
        frame.rectangles.destination[0],
        frame.rectangles.destination[1],
    ];
    let Some(spec) = draw_plan(
        p,
        frame.state.count,
        frame.state.frames,
        source,
        destination,
        [metadata(first), metadata(second)],
        [0, 0, frame.canvas[0] as i32, frame.canvas[1] as i32],
        random,
    )?
    else {
        return Ok(None);
    };
    Ok(Some(ImageInput::Process {
        input: Box::new(ImageInput::Ternary {
            destination: Box::new(base),
            first: Box::new(ImageInput::Resource(first.unwrap().picture.raw.clone())),
            second: Box::new(ImageInput::Resource(second.unwrap().picture.raw.clone())),
        }),
        effects: serde_json::to_vec(&[ImageEffect::PackedPairBlend { spec }])?.into(),
    }))
}
