//! 41ADE0's reviewed RGB24 display records become engine immutable image
//! layers. Clipping, colour-key comparison and pixel copies are shared Image
//! operations; this adapter contains no raster loop or software framebuffer.
use super::*;
use crate::dc2_bitmap::Draw;
use crysirror_core::image_processing::{ImageCompositeLayer, ImageCompositeMath, ImageInput};

pub(super) fn layers(
    bridge: &mut Bridge<'_>,
    records: &[Draw],
    extent: [u32; 2],
    //The caller owns the DC2 bitmap state for the duration of the tick
    //(Bridge::dc2_bitmap takes it out of services), so its display-format
    //results are passed explicitly instead of read through the bridge.
    validated: &BTreeMap<u8, (String, bool)>,
) -> Result<Vec<ImageCompositeLayer>> {
    let clip = Some([0, 0, i32::try_from(extent[0])?, i32::try_from(extent[1])?]);
    let mut layers = Vec::with_capacity(records.len());
    for record in records {
        //Weighted variants reuse the plain source mapping with /16 arithmetic.
        let (record, weight) = match record {
            Draw::Blend { slot, source, destination, weight } => (
                Draw::Image { slot: *slot, source: *source, destination: *destination },
                Some(*weight),
            ),
            Draw::SolidBlend { rect, color, weight } => (
                Draw::Solid { rect: *rect, color: [color[0], color[1], color[2], 255] },
                Some(*weight),
            ),
            other => (other.clone(), None),
        };
        let record = &record;
        let shift = |k: u8| {
            use crysirror_core::image_blend::{RgbaBlendArithmetic as A, RgbaBlendComponent, RgbaBlendFactor as F, RgbaBlendState};
            let k = f32::from(k) / 16.;
            RgbaBlendState {
                arithmetic: A::ByteShiftSum { shift: 4 },
                color: RgbaBlendComponent { source: F::ConstantAlpha, destination: F::OneMinusConstantAlpha, ..Default::default() },
                alpha: RgbaBlendComponent { source: F::Zero, destination: F::One, ..Default::default() },
                constant: [k; 4],
                write_mask: [true, true, true, false],
                ..Default::default()
            }
        };
        match record {
            Draw::Image {
                slot,
                source,
                destination,
            } => {
                let Some(uploaded) = bridge.services.images.get(slot) else {
                    continue; //40B840's explicit absent/native-1 source guard.
                };
                let logical = bridge
                    .services
                    .image_extents
                    .get(slot)
                    .copied()
                    .context("DC2 native display source extent")?;
                let image = bridge
                    .host
                    .published_image_resource(bridge.client, uploaded.path.relative())?
                    .context("DC2 native display immutable image lease missing")?;
                use crysirror_core::image_blend::{
                    RgbaBlendArithmetic as Arithmetic, RgbaBlendComponent, RgbaBlendFactor as F,
                    RgbaBlendState, RgbaByteRounding as Rounding,
                };
                let argb32 = validated
                    .get(slot)
                    .is_some_and(|(key, alpha)| *alpha && key == uploaded.path.relative());
                let math = if let Some(weight) = weight {
                    if argb32 {
                        //40C0E0: alpha'*w/16 over the destination, one /0xFF0 sum.
                        let mut state = RgbaBlendState {
                            arithmetic: Arithmetic::Byte255Sum { rounding: Rounding::Floor },
                            color: RgbaBlendComponent {
                                source: F::SourceAlpha,
                                destination: F::OneMinusSourceAlpha,
                                ..Default::default()
                            },
                            write_mask: [true, true, true, false],
                            source_alpha_key: Some(0),
                            ..Default::default()
                        };
                        state.source_scale[3] = f32::from(weight) / 16.;
                        state
                    } else {
                        //40C360: keyed (src*w + dst*(16-w)) >> 4.
                        RgbaBlendState { source_color_key: Some([255, 0, 255]), ..shift(weight) }
                    }
                } else if argb32 {
                    //40B5F0 (selected by the image's ARGB32 flag): first byte is
                    //inverted alpha; 0xFF skips, otherwise each RGB byte is
                    //(src*(255-a')+dst*a')/255 truncated as one sum. No colour key.
                    RgbaBlendState {
                        arithmetic: Arithmetic::Byte255Sum { rounding: Rounding::Floor },
                        color: RgbaBlendComponent {
                            source: F::SourceAlpha,
                            destination: F::OneMinusSourceAlpha,
                            ..Default::default()
                        },
                        write_mask: [true, true, true, false],
                        source_alpha_key: Some(0),
                        ..Default::default()
                    }
                } else {
                    //40B840 copies RGB24 except the exact encoded FF00FF key,
                    //leaving the opaque destination alpha untouched. Byte255
                    //with source=1,destination=0 is an exact copy, not source-over.
                    RgbaBlendState {
                        arithmetic: Arithmetic::Byte255 {
                            source: Rounding::Floor,
                            destination: Rounding::Floor,
                        },
                        write_mask: [true, true, true, false],
                        source_color_key: Some([255, 0, 255]),
                        ..Default::default()
                    }
                };
                layers.push(ImageCompositeLayer {
                    //The shared upload has POT padding, whereas40B840 clips to
                    //the native logical bitmap. A shared copy into a logical
                    //surface preserves those clipping bounds for the outer blit.
                    input: Box::new(ImageInput::Composite {
                        base: Box::new(ImageInput::Solid {
                            size: logical,
                            color: [0; 4],
                        }),
                        layers: vec![ImageCompositeLayer {
                            input: Box::new(ImageInput::Resource(image)),
                            source: [0, 0, i32::try_from(logical[0])?, i32::try_from(logical[1])?],
                            destination: [0, 0],
                            clip: None,
                            opacity: 255,
                            math: ImageCompositeMath::Copy,
                        }],
                    }),
                    source: *source,
                    destination: *destination,
                    clip,
                    opacity: 255,
                    math: ImageCompositeMath::Normalized(math),
                });
            }
            Draw::Solid { rect, color } => layers.push(ImageCompositeLayer {
                input: Box::new(ImageInput::Solid {
                    size: [u32::try_from(rect[2])?, u32::try_from(rect[3])?],
                    color: *color,
                }),
                source: [0, 0, rect[2], rect[3]],
                destination: [rect[0], rect[1]],
                clip,
                opacity: 255,
                math: match weight {
                    Some(weight) => ImageCompositeMath::Normalized(shift(weight)),
                    None => ImageCompositeMath::CopyRgb,
                },
            }),
            Draw::Blend { .. } | Draw::SolidBlend { .. } => unreachable!(),
        }
    }
    Ok(layers)
}
