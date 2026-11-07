// SPDX-License-Identifier: MPL-2.0
//!UD31 ->435AB0 native29/412BF0. Native selects block8..40 and switches
//!from old to new at phase256. BoxReduce owns exact means, FixedAffine owns
//!cell replication, and Composite owns RGB placement; no private pixel path.
use super::*;
use crysirror_core::image_processing::{ImageCompositeLayer, ImageCompositeMath, ImageResource};
use crysirror_image::{
    box_reduce::{BoxAlpha, BoxAnchors, BoxArithmetic, BoxReduce},
    fixed_affine::{FixedAffineMath, FixedAffineNearest},
};
pub(super) fn input(
    phase: u32,
    old: Arc<dyn ImageResource>,
    target: Arc<dyn ImageResource>,
) -> Result<ImageInput> {
    ensure!(phase < 512, "SugarPot mosaic phase");
    let amount = if phase < 256 { phase } else { 512 - phase };
    let block = 8 + amount / 8;
    let source = if phase < 256 { old } else { target };
    let widths = [1024 / block * block, 1024 % block];
    let heights = [600 / block * block, 600 % block];
    let mut layers = vec![];
    for y in 0..2 {
        for x in 0..2 {
            let width = widths[x];
            let height = heights[y];
            if width == 0 || height == 0 {
                continue;
            }
            let origin = [
                if x == 0 { 0 } else { widths[0] },
                if y == 0 { 0 } else { heights[0] },
            ];
            let window = [
                if x == 0 { block } else { width },
                if y == 0 { block } else { height },
            ];
            let coarse = ImageInput::Process {
                input: Box::new(ImageInput::Resource(source.clone())),
                effects: serde_json::to_vec(&vec![
                    crysirror_image::effects::ImageEffect::BoxReduce {
                        spec: BoxReduce {
                            size: [width / window[0], height / window[1]],
                            source: Some([origin[0], origin[1], width, height]),
                            window: window.map(|n| n as u16),
                            anchors: BoxAnchors::Grid { stride: window },
                            arithmetic: BoxArithmetic::Mean,
                            alpha: BoxAlpha::Opaque,
                        },
                    },
                ])?
                .into(),
            };
            //For 0<=coordinate<1024 and 8<=block<=40, ceil(2^24/block)
            //has less than1/40 total quantization error: its fixed-point floor
            //is exactly coordinate/block, including block boundaries. Edge
            //strips contain one coarse sample on their short axis.
            let step = ((1u32 << 24) + block - 1) / block;
            let rectangle = [
                origin[0] as i32,
                origin[1] as i32,
                width as i32,
                height as i32,
            ];
            let expanded = ImageInput::Process {
                input: Box::new(ImageInput::Raster {
                    destination: Box::new(ImageInput::Solid {
                        size: [1024, 600],
                        color: [0, 0, 0, 255],
                    }),
                    source: Box::new(coarse),
                }),
                effects: serde_json::to_vec(&vec![
                    crysirror_image::effects::ImageEffect::FixedAffineNearest {
                        spec: FixedAffineNearest {
                            rectangle,
                            origin: [rectangle[0], rectangle[1]],
                            linear: [
                                [if x == 0 { step as i32 } else { 0 }, 0],
                                [0, if y == 0 { step as i32 } else { 0 }],
                            ],
                            translation: [0, 0],
                            fractional_bits: 24,
                            math: FixedAffineMath::CopyRgb,
                        },
                    },
                ])?
                .into(),
            };
            layers.push(ImageCompositeLayer {
                input: Box::new(expanded),
                source: rectangle,
                destination: [rectangle[0], rectangle[1]],
                clip: None,
                opacity: 255,
                math: ImageCompositeMath::CopyRgb,
            });
        }
    }
    Ok(ImageInput::Composite {
        base: Box::new(ImageInput::Solid {
            size: [1024, 600],
            color: [0, 0, 0, 255],
        }),
        layers,
    })
}
