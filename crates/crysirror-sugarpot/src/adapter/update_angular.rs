// SPDX-License-Identifier: MPL-2.0
//!UD27/28 native411440/411B80. The four original half-row boundary equations
//!produce an immutable shared RectMap mask. No VM coverage/pixel loop exists.
use super::*;
use crysirror_core::image_processing::{ImageCompositeLayer, ImageCompositeMath, ImageResource};
use crysirror_image::rect_map::{RectMap, RectMapRegion, RectMapSample};
const SLOPE: f64 = 6.666666507720947;
const RECIPROCAL: f64 = 11.37777805328369;
fn growing(row: i32, angle: f64) -> i32 {
    if angle <= 0. {
        return 0;
    }
    if angle >= 90. {
        return 512;
    }
    let width = if angle <= 45. {
        f64::from(row) * (angle * SLOPE) / 300.
    } else {
        f64::from(row) * 512. / ((90. - angle) * RECIPROCAL)
    };
    (width as i32).clamp(0, 512)
}
fn shrinking(row: i32, angle: f64) -> i32 {
    if angle <= 0. {
        return 512;
    }
    if angle >= 90. {
        return 0;
    }
    let width = if angle < 45. {
        f64::from(row) * 512. / (angle * RECIPROCAL)
    } else {
        f64::from(row) * ((90. - angle) * SLOPE) / 300.
    };
    (width as i32).clamp(0, 512)
}
pub(super) fn input(
    kind: i32,
    phase: u32,
    old: Arc<dyn ImageResource>,
    target: Arc<dyn ImageResource>,
) -> Result<ImageInput> {
    ensure!(matches!(kind, 27 | 28), "SugarPot angular update selector");
    let phase = phase.clamp(1, 255);
    let sequential = kind == 28;
    let ratio = (f64::from(phase) / if sequential { 255. } else { 256. }) as f32;
    let theta = (f64::from(ratio) * if sequential { 360. } else { 90. }) as f32;
    let angles: [f64; 4] = std::array::from_fn(|quarter| {
        f64::from(if sequential {
            (f64::from(theta) - quarter as f64 * 90.) as f32
        } else {
            theta
        })
    });
    let mut regions = Vec::with_capacity(1200);
    let mut span = |x: i32, y: i32, width: i32| {
        if width > 0 {
            regions.push(RectMapRegion {
                source: [0, 0, 1, 1],
                destination: [x, y, width, 1],
            });
        }
    };
    for row in 0..300 {
        //Native visits both halves from their lower row upward. The left
        //and right blade formulas use j versus300-j, including that asymmetry.
        span(512, 299 - row, growing(row, angles[0]));
        let width = 512 - shrinking(300 - row, angles[1]);
        span(1024 - width, 599 - row, width);
        let width = growing(300 - row, angles[2]);
        span(512 - width, 599 - row, width);
        span(0, 299 - row, 512 - shrinking(row, angles[3]));
    }
    let mask = ImageInput::Process {
        input: Box::new(ImageInput::Solid {
            size: [1, 1],
            color: [255; 4],
        }),
        effects: serde_json::to_vec(&vec![crysirror_image::effects::ImageEffect::RectMap {
            spec: RectMap {
                regions,
                output_size: Some([1024, 600]),
                clear: [0; 4],
                sample: RectMapSample::Floor,
                copy_alpha: true,
            },
        }])?
        .into(),
    };
    let selected = ImageInput::Process {
        input: Box::new(ImageInput::Ternary {
            destination: Box::new(ImageInput::Resource(target.clone())),
            first: Box::new(ImageInput::Resource(target)),
            second: Box::new(mask),
        }),
        effects: serde_json::to_vec(&vec![
            crysirror_image::effects::ImageEffect::AlphaMaskPlane {
                spec: crysirror_image::alpha_mask::AlphaMaskPlane {
                    channel: 0,
                    offset: 0.,
                    restore: false,
                },
            },
        ])?
        .into(),
    };
    Ok(ImageInput::Composite {
        base: Box::new(ImageInput::Resource(old)),
        layers: vec![ImageCompositeLayer {
            input: Box::new(selected),
            source: [0, 0, 1024, 600],
            destination: [0, 0],
            clip: None,
            opacity: 255,
            //The mask is strictly0/255: shared separate-floor mode
            //has exact copy/skip endpoints and retains stage alpha255.
            math: ImageCompositeMath::MaskSeparate256,
        }],
    })
}
