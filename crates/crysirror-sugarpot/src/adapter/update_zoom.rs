// SPDX-License-Identifier: MPL-2.0
//!449870 UD20/22/24 ->435AB0 native18/20/22 ->40FAE0/410830.
//!Register-aware analysis is necessary: EDI selects arg8 (new target), not
//!the old arg4 used by the neighbouring17/19 branches of the merged helper.
//!UD19/21/23 ->435AB0 native17/19/21 apply the same helpers to the old
//!scene over the new one: UD19 zooms in by 255/(256-p), UD21 shrinks by
//!(255-p)/255 (408DB0 restores the target under the previous rectangle),
//!UD23 rotates by p*1.41176 degrees while zooming in by 255/(256-p).
use super::*;
use crysirror_core::image_processing::{ImageCompositeLayer, ImageCompositeMath, ImageResource};
use crysirror_image::rect_map::{RectMap, RectMapRegion, RectMapSample};
pub(super) fn input(
    kind: i32,
    phase: u32,
    old: Arc<dyn ImageResource>,
    target: Arc<dyn ImageResource>,
) -> Result<ImageInput> {
    ensure!(
        matches!(kind, 19..=24),
        "SugarPot zoom selector"
    );
    let phase = phase.clamp(1, 255);
    // Old-scene variants draw the old stage over the new one.
    let (old, target) = if matches!(kind, 19 | 21 | 23) { (target, old) } else { (old, target) };
    let target = if matches!(kind, 23 | 24) {
        use crysirror_image::fixed_affine::{FixedAffineMath, FixedAffineNearest};
        let angle = (f64::from(phase) * 1.411764740943909) as f32;
        let radians = f64::from(angle) * 6.283185307 / 360.;
        let cosine = (radians.cos() * 128.) as i32;
        let sine = (radians.sin() * 128.) as i32;
        //410920 rotates the already-rounded40FA70 integer table. Factoring
        //the pure integer transform before the nearest zoom preserves that
        //order exactly; a single scaled float matrix would round differently.
        ImageInput::Process {
            input: Box::new(ImageInput::Raster {
                destination: Box::new(ImageInput::Solid {
                    size: [1024, 600],
                    color: [0, 0, 0, 255],
                }),
                source: Box::new(ImageInput::Resource(target)),
            }),
            effects: serde_json::to_vec(&vec![
                crysirror_image::effects::ImageEffect::FixedAffineNearest {
                    spec: FixedAffineNearest {
                        rectangle: [0, 0, 1024, 600],
                        origin: [512, 300],
                        linear: [[cosine, sine], [-sine, cosine]],
                        translation: [512 * 128, 300 * 128],
                        fractional_bits: 7,
                        math: FixedAffineMath::CopyRgb,
                    },
                },
            ])?
            .into(),
        }
    } else {
        ImageInput::Resource(target)
    };
    let scale = match kind {
        19 | 23 => (255. / f64::from(256 - phase)) as f32,
        21 => (f64::from(255 - phase) / 255.) as f32,
        22 => (f64::from(phase) / 255.) as f32,
        _ => (255. / f64::from(phase)) as f32,
    };
    //40FAE0 stores the ratio in float, then truncates x87 size expressions.
    //All normal admitted samples keep both extents positive. The terminal
    //256 branch copies the untransformed target instead of evaluating scale.
    let (source, destination) = if scale > 1. {
        let width = (1024. / f64::from(scale)) as i32;
        let height = (600. / f64::from(scale)) as i32;
        (
            [512 - (width >> 1), 300 - (height >> 1), width, height],
            [0, 0, 1024, 600],
        )
    } else {
        let width = (1024. * f64::from(scale)) as i32;
        let height = (600. * f64::from(scale)) as i32;
        (
            [0, 0, 1024, 600],
            [512 - (width >> 1), 300 - (height >> 1), width, height],
        )
    };
    let spec = RectMap {
        output_size: None,
        regions: vec![RectMapRegion {
            source,
            destination,
        }],
        clear: [0, 0, 0, 255],
        sample: RectMapSample::NearestRoundLinear,
        copy_alpha: false,
    };
    //For UD22 the growing rectangle contains the previous one, so pixels
    //outside it retain the old stage. Do not fill those pixels with black.
    Ok(ImageInput::Composite {
        base: Box::new(ImageInput::Resource(old)),
        layers: vec![ImageCompositeLayer {
            input: Box::new(ImageInput::Process {
                input: Box::new(target),
                effects: serde_json::to_vec(&vec![
                    crysirror_image::effects::ImageEffect::RectMap { spec },
                ])?
                .into(),
            }),
            source: destination,
            destination: [destination[0], destination[1]],
            clip: None,
            opacity: 255,
            math: ImageCompositeMath::CopyRgb,
        }],
    })
}
