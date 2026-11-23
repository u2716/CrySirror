//!40CCD0's literal floor(sourceByte*weight/16) over a frozen original
//! capture. The shared Image worker owns traversal, clipping and byte math.
use super::*;
use crysirror_core::image_processing::{
    ImageCompositeLayer, ImageCompositeMath, ImageInput, ImageResource,
};
pub(super) fn input(
    client: ClientId,
    original: Arc<dyn ImageResource>,
    weight: u8,
) -> Result<ImageInput> {
    ensure!(
        weight <= 16 && original.client() == client && original.size() == crate::dc2::CANVAS,
        "DC2 captured dimmer source ownership/extent/weight"
    );
    Ok(ImageInput::Composite {
        base: Box::new(ImageInput::Solid {
            size: crate::dc2::CANVAS,
            color: [0, 0, 0, 255],
        }),
        layers: vec![ImageCompositeLayer {
            input: Box::new(ImageInput::Resource(original)),
            source: [0, 0, 800, 600],
            destination: [0, 0],
            clip: None,
            opacity: 255,
            math: ImageCompositeMath::IntegerStored {
                destination_format: 1,
                source_format: 1,
                mode: 0xf0,
                parameter: i32::from(16 - weight) * 16,
            },
        }],
    })
}
