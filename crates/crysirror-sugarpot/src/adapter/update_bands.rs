// SPDX-License-Identifier: MPL-2.0
//!UD29/30 ->435AB0 native27/28 ->412490/412590. Opposing edge bands
//!use an exact half-weight scalar; the complete old/new base changes at the
//!midpoint. Shared Composite owns clipping, pixel traversal and allocation.
use super::*;
use crysirror_core::image_processing::{ImageCompositeLayer, ImageCompositeMath, ImageResource};
pub(super) fn input(
    kind: i32,
    phase: u32,
    old: Arc<dyn ImageResource>,
    target: Arc<dyn ImageResource>,
) -> Result<ImageInput> {
    ensure!(matches!(kind, 29 | 30), "SugarPot edge band selector");
    let phase = phase.clamp(1, 255);
    let horizontal = kind == 30;
    let size = if horizontal { 1024 } else { 600 };
    let ratio = (f64::from(phase) / 255.) as f32;
    let mut extent = (f64::from(size) * f64::from(ratio)) as i32;
    if horizontal {
        extent += extent % 2;
    }
    let (base, source) = if extent <= size / 2 {
        (old, target)
    } else {
        extent = 2 * (size / 2) - extent;
        (target, old)
    };
    let mut layers = Vec::with_capacity(2);
    if extent > 0 {
        for side in 0..2 {
            let start = if side == 0 { 0 } else { size - extent };
            let rectangle = if horizontal {
                [start, 0, extent, 600]
            } else {
                [0, start, 1024, extent]
            };
            layers.push(ImageCompositeLayer {
                input: Box::new(ImageInput::Resource(source.clone())),
                source: rectangle,
                destination: [rectangle[0], rectangle[1]],
                clip: None,
                opacity: 128,
                math: ImageCompositeMath::OpaqueScalar256,
            });
        }
    }
    Ok(ImageInput::Composite {
        base: Box::new(ImageInput::Resource(base)),
        layers,
    })
}
