// SPDX-License-Identifier: MPL-2.0
//!UD35/36 native413560/413690: source-axis centre crop and perpendicular
//!destination-axis squeeze, with initialized black borders. Pixel sampling
//!and output ownership use the shared RectMap worker.
use super::*;
use crysirror_core::image_processing::ImageResource;
use crysirror_image::rect_map::{RectMap, RectMapRegion, RectMapSample};
pub(super) fn input(
    kind: i32,
    phase: u32,
    old: Arc<dyn ImageResource>,
    target: Arc<dyn ImageResource>,
) -> Result<ImageInput> {
    ensure!(
        matches!(kind, 35 | 36) && phase < 512,
        "SugarPot squeeze update selector/phase"
    );
    let source = if phase < 256 { old } else { target };
    let amount = if phase < 256 { phase } else { 512 - phase };
    let ratio = f64::from((f64::from(amount) / 256.) as f32);
    let x = (1024. * ratio * 0.5) as i32;
    let y = (600. * ratio * 0.5) as i32;
    let regions = if amount == 256 {
        vec![]
    } else if kind == 35 {
        vec![RectMapRegion {
            source: [x, 0, 1024 - 2 * x, 600],
            destination: [0, y, 1024, 600 - 2 * y],
        }]
    } else {
        vec![RectMapRegion {
            source: [0, y, 1024, 600 - 2 * y],
            destination: [x, 0, 1024 - 2 * x, 600],
        }]
    };
    Ok(ImageInput::Process {
        input: Box::new(ImageInput::Resource(source)),
        effects: serde_json::to_vec(&vec![crysirror_image::effects::ImageEffect::RectMap {
            spec: RectMap {
                regions,
                output_size: None,
                clear: [0, 0, 0, 255],
                sample: RectMapSample::NearestRoundLinear,
                copy_alpha: false,
            },
        }])?
        .into(),
    })
}
