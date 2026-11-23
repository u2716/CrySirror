// SPDX-License-Identifier: MPL-2.0
//!UD34/413340 native float phase policy over shared RectMap rows. The
//!E518B0 phase is PE.data virtual zero-fill, retained across original loads.
use super::*;
use crysirror_core::image_processing::ImageResource;
use crysirror_image::rect_map::{RectMap, RectMapRegion, RectMapSample};
pub(super) fn input(
    phase: u32,
    angle: f32,
    old: Arc<dyn ImageResource>,
    target: Arc<dyn ImageResource>,
) -> Result<(ImageInput, f32)> {
    ensure!(phase < 512 && angle.is_finite(), "SugarPot wave phase");
    let amount = if phase < 256 { phase } else { 512 - phase };
    let source = if phase < 256 { old } else { target };
    if amount == 0 {
        return Ok((ImageInput::Resource(source), angle));
    }
    let amplitude = (f64::from((f64::from(amount) / 256.) as f32) * 1024.) as f32;
    let mut row_angle = angle;
    let mut regions = Vec::with_capacity(600);
    for row in 0..600 {
        let sine = f64::from(row_angle).sin() as f32;
        let offset = (f64::from(sine) * f64::from(amplitude)) as i32;
        row_angle = (f64::from(row_angle) + 0.05000000074505806) as f32;
        if offset.unsigned_abs() >= 1024 {
            continue;
        }
        let (sx, dx) = if offset < 0 {
            (-offset, 0)
        } else {
            (0, offset)
        };
        let width = 1024 - offset.abs();
        regions.push(RectMapRegion {
            source: [sx, row, width, 1],
            destination: [dx, row, width, 1],
        });
    }
    let mut next = (f64::from(angle) + 0.1000000014901161) as f32;
    if next < 0. {
        next = 6.2831855;
    } else if f64::from(next) > 6.283185482025146 {
        next = 0.;
    }
    Ok((
        ImageInput::Process {
            input: Box::new(ImageInput::Resource(source)),
            effects: serde_json::to_vec(&vec![crysirror_image::effects::ImageEffect::RectMap {
                spec: RectMap {
                    regions,
                    output_size: None,
                    clear: [0, 0, 0, 255],
                    sample: RectMapSample::Floor,
                    copy_alpha: false,
                },
            }])?
            .into(),
        },
        next,
    ))
}
