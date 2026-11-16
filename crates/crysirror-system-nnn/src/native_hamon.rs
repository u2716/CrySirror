// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! HAMON69 parameters, class-wide vector-cache identity and Print phase.
//! Complete839-line source/header and software MMX read. Per-pixel vector
//! construction, table application and two-image sampling belong to shared
//! workers, including the source's four-lane direction-vector quirk.
use anyhow::{Result, ensure};
use crysirror_image::temporal_pattern::PatternRng;
pub(crate) fn parameters(values: &[i32], canvas: [u32; 2]) -> [i32; 32] {
    let defaults = [
        20,
        0,
        canvas[0] as i32 / 2,
        canvas[1] as i32 / 2,
        20,
        30,
        10,
        8,
        100,
        100,
        0,
        0,
        100,
        0,
        0,
        0,
        0,
    ];
    // Source copies32 words but initializes only the first17. Define omitted
    // trailing undefined words0; raw restore supplies and retains all32.
    let mut p = std::array::from_fn(|i| {
        values
            .get(i)
            .copied()
            .unwrap_or_else(|| defaults.get(i).copied().unwrap_or(0))
    });
    p[5] = p[5].max(1);
    p[6] = p[6].max(1);
    p[8] = p[8].clamp(0, 100);
    p[9] = p[9].max(1);
    p[10] = p[10].clamp(0, canvas[1] as i32 - 1);
    p
}
pub(crate) fn validate(p: &[i32; 32]) -> Result<()> {
    ensure!(
        p[5] > 0 && p[6] > 0 && p[9] > 0,
        "NNN ripple divisor domain"
    );
    crate::native_curve::validate(p[0], p[14], p[13])
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct VectorKey {
    pub canvas: [u32; 2],
    pub center: [i32; 2],
    pub start_y: i32,
}
#[derive(Clone, Default)]
pub(crate) struct HamonClass {
    pub vector: Option<VectorKey>,
}
impl HamonClass {
    pub(crate) fn set(&mut self, p: &[i32; 32], canvas: [u32; 2]) -> Result<bool> {
        validate(p)?;
        ensure!(
            canvas.iter().all(|n| (1..=16384).contains(n)),
            "NNN ripple canvas"
        );
        let key = VectorKey {
            canvas,
            center: [p[2], p[3]],
            start_y: p[10],
        };
        let changed = self.vector != Some(key);
        self.vector = Some(key);
        Ok(changed)
    }
}
#[derive(Clone, Copy)]
pub(crate) struct Phase {
    pub count: i32,
    pub wave_length: i32,
    pub speed: i32,
    pub angular_speed: i32,
    pub hold: i32,
    pub power: f64,
    pub weights: [u16; 2],
}
pub(crate) fn phase(
    p: &[i32; 32],
    count: i32,
    frames: i32,
    canvas: [u32; 2],
    rng: &mut PatternRng,
) -> Result<Option<Phase>> {
    validate(p)?;
    let divisor = frames.max(1);
    let count = crate::native_curve::value_with_random(
        count.min(divisor),
        p[0],
        0,
        p[0],
        p[14],
        p[13],
        rng,
    )?;
    // The source consumes its curve before this screen/32 subdivision guard,
    // and before either source picture/capture/null-buffer selection.
    if canvas[0] / 32 < 1 || canvas[1] / 32 < 1 {
        return Ok(None);
    }
    let power = (f64::from(p[4]) - f64::from(count) * (f64::from(p[8]) * 0.01)).max(0.0);
    let mut weights = [256, 0];
    if count >= p[11] {
        let dv = divisor.wrapping_sub(p[11]).max(1);
        let c = count.wrapping_sub(p[11]);
        weights = [
            ((((dv.wrapping_sub(c)).wrapping_mul(100)) / dv).wrapping_mul(256) / 100) as u16,
            (((c.wrapping_mul(100)) / dv).wrapping_mul(256) / 100) as u16,
        ];
    }
    Ok(Some(Phase {
        count,
        wave_length: p[5],
        speed: p[6],
        angular_speed: p[7],
        hold: p[12],
        power,
        weights,
    }))
}
#[derive(Clone, Copy)]
pub(crate) struct SourceSelection {
    pub first_capture: bool,
    pub second_capture: bool,
    pub same_first: bool,
    pub second_slot: i32,
}
pub(crate) fn sources(p: &[i32; 32], bank_limit: i32) -> Result<SourceSelection> {
    ensure!(
        matches!(bank_limit, 16 | 32),
        "NNN ripple picture bank profile"
    );
    Ok(SourceSelection {
        first_capture: p[15] != 0,
        second_capture: p[16] != 0 && p[1] != -1,
        same_first: p[1] == -1,
        second_slot: p[1].clamp(0, bank_limit - 1),
    })
}

pub(crate) fn image_input(
    frame: &crate::native_picture_effect::PictureEffectFrame,
    p: &[i32; 32],
    field: &crate::native_hamon_vectors::Field,
    capture: Option<&std::sync::Arc<dyn crysirror_core::image_processing::ImageResource>>,
    rng: &mut PatternRng,
) -> Result<Option<crysirror_core::image_processing::ImageInput>> {
    use crysirror_core::image_processing::ImageInput;
    use crysirror_image::{effects::ImageEffect, radial_wave_pair::RadialWavePair};
    ensure!(
        field.key.canvas == frame.canvas,
        "NNN HAMON vector canvas identity"
    );
    let Some(phase) = phase(p, frame.state.count, frame.state.frames, frame.canvas, rng)? else {
        return Ok(None);
    };
    let select = sources(p, frame.bank_limit)?;
    if !select.first_capture && !frame.geometry_assigned {
        return Ok(None);
    }
    let first = if select.first_capture {
        capture.cloned()
    } else {
        frame
            .bank
            .picture(frame.slot)
            .map(|p| p.picture.raw.clone())
    };
    let Some(first) = first else { return Ok(None) };
    let second = if select.same_first {
        Some(first.clone())
    } else if select.second_capture {
        capture.cloned()
    } else {
        frame
            .bank
            .picture(select.second_slot)
            .map(|p| p.picture.raw.clone())
    };
    let Some(second) = second else {
        return Ok(None);
    };
    let fallback = crate::native_geometry::NativeRects::natural(frame.canvas);
    // nextPic==-1 reuses the first IMAGE, not its destination. The original
    // subsequent GetDstPoint(-1) was outside EFFECT storage; clamp that invalid
    // index just like other reviewed bank getters instead of reading memory.
    let second_rect = frame
        .bank
        .rectangles(select.second_slot)
        .unwrap_or(&fallback);
    let spec = RadialWavePair {
        canvas: frame.canvas,
        count: phase.count,
        wave_length: phase.wave_length,
        speed: phase.speed,
        angular_speed: phase.angular_speed,
        hold: phase.hold,
        power: phase.power,
        weights: phase.weights,
        source_origins: [
            [
                frame.rectangles.destination[0].wrapping_neg(),
                frame.rectangles.destination[1].wrapping_neg(),
            ],
            [
                second_rect.destination[0].wrapping_neg(),
                second_rect.destination[1].wrapping_neg(),
            ],
        ],
    };
    spec.validate()?;
    Ok(Some(ImageInput::Process {
        input: Box::new(ImageInput::Ternary {
            destination: Box::new(ImageInput::Resource(field.image.clone())),
            first: Box::new(ImageInput::Resource(first)),
            second: Box::new(ImageInput::Resource(second)),
        }),
        effects: serde_json::to_vec(&[ImageEffect::RadialWavePair { spec }])?.into(),
    }))
}
// Native source policy for the shared worker implementation:
// - Vector capacity is align4(width)*height. For each4-lane group, radius
//   uses x+lane, but directionX for lanes2/3 ALSO uses x+1, not x+2/x+3.
// - rr<1 becomes1; unit components use(double*256/r+.5) truncation into
//   signed16 words. Rounded radius is clamped1..floor(diagonal), then the
//   BYTE OFFSET(radius*4) is packed into16 bits (large distances can wrap).
// - Each radius gets a signed16 displacement: ramp power over distance10,
//   then linear falloff across hold, signed%wave_length phase, native3.14159
//   cosine +.5/trunc BEFORE multiplying256 and retaining low16.
// - Signed pmulhw(vector,displacement) produces offsets. Both sources use the
//   same field and native-negated respective destination origins. Out-of-range
//   source pixels are black; masks, CharaClip and current destination RGB are
//   ignored. Products use low16, summed once then>>8, not independent floors.
// - Cached vectors are one CLASS resource: the last setter's center/startY
//   governs EVERY HAMON slot until another setter, not each slot's own center.
// - para9/elasticity is computed and never used; printStartY only invalidates
//   the vector cache; rendering still starts at0,0 over the entire canvas.
// - nextPic==-1 overrides capture2 selection, reusing source1, but its second
//   GetDstPoint(-1) still applies the native invalid-index bounded replacement.
// Fresh/raw InitParameter replay all32 words and reset counters/cache identity.
// CountIsMaxFALSE, Calcu/BeginPrint/Init empty, NeedPicPrintFALSE.
