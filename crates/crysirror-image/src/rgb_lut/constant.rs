//! Stored-byte constant blends. RGB tables leave alpha to their caller;
//! pixel profiles declare their separate coverage/output-alpha policy.
use super::{RgbLookup, RgbLookupProfile};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConstantByteBlend {
    Normal,
    Screen,
    Multiply,
    Difference,
    Lighten,
    Darken,
    Add,
    Subtract,
    LinearBurn,
    Overlay,
    HardLight,
    Exclusion,
    LinearLight,
    PinLight,
    /// Signed delta floor /256. Pixel opacity255 retains source coverage;
    /// smaller opacity uses floor(source_alpha*opacity/256). Output alpha is
    /// that coverage. This scalar profile deliberately differs from Normal's
    /// /128 quantization and never claims an MMX equivalent.
    Scalar256,
    /// Ignore source coverage. Opacity255 copies RGB; smaller values use
    /// signed delta floor /256. Pixel output retains destination alpha.
    OpaqueScalar256,
    /// RGB-only screen: S'=floor(S*opacity/255), then
    /// 255-floor((255-D)*(255-S')/255). Source alpha is ignored;
    /// destination alpha is retained. Unlike Screen this has no /128 weight.
    OpaqueScreen255,
    /// RGB-only product floor(D*S/255), ignoring source alpha and opacity.
    /// Destination alpha is retained. Callers control visibility separately.
    OpaqueMultiply255,
    /// Native straight mask: endpoints0/255 preserve/copy, intermediate
    /// source and destination contributions floor separately at /256.
    /// Destination alpha is retained.
    MaskSeparate256,
    /// Native translucent mask: opaque texels use weights(weight,256-weight),
    /// partial masks use a=floor(mask*weight/256), b=255-a with one final floor.
    /// Full opacity uses MaskSeparate256 instead.
    MaskPiecewise256 { weight:u8 },
}
impl ConstantByteBlend {
    /// One stored-byte blend lane. Coverage is already modulated by the
    /// caller; no alpha-zero elision is implicit in the packed kernel.
    pub fn channel(self, destination: u8, color: u8, alpha: u8) -> u8 {
        let (destination, color, alpha) =
            (i32::from(destination), i32::from(color), i32::from(alpha));
        let full = alpha + (alpha >> 7);
        let half = (alpha >> 1) + (alpha >> 7);
        let positive = (color * full) >> 8;
        let white = 255 + (((color - 255) * half) >> 7);
        let middle = 128 + (((color - 128) * half) >> 7);
        use ConstantByteBlend::*;
        let value = match self {
            Normal => destination + (((color - destination) * ((alpha + 1) >> 1)) >> 7),
            Scalar256 => destination + ((color - destination) * alpha).div_euclid(256),
            OpaqueScalar256 => {
                if alpha == 255 {
                    color
                } else {
                    destination + ((color - destination) * alpha).div_euclid(256)
                }
            }
            OpaqueScreen255 => {
                let source = color * alpha / 255;
                255 - (255 - destination) * (255 - source) / 255
            }
            OpaqueMultiply255 => destination * color / 255,
            MaskSeparate256 => match alpha {
                0=>destination,255=>color,
                _=>((color*alpha)>>8)+((destination*(256-alpha))>>8),
            },
            MaskPiecewise256{weight}=>{
                if alpha==0||weight==0 {destination} else {
                    let (a,b)=if alpha==255 {(i32::from(weight),256-i32::from(weight))}
                        else {let a=(alpha*i32::from(weight))>>8;(a,255-a)};
                    (color*a+destination*b)>>8
                }
            },
            Screen => positive + destination - ((positive * destination) >> 8),
            Multiply => ((white + (white >> 7)) * destination) >> 8,
            Difference => (positive - destination).abs(),
            Lighten => positive.max(destination),
            Darken => white.min(destination),
            Add => positive + destination,
            Subtract => destination - positive,
            LinearBurn => destination - (((255 - color) * full) >> 8),
            Exclusion => positive + destination - 2 * ((positive * destination) >> 8),
            Overlay | HardLight => {
                let high = if self == Overlay {
                    destination > 128
                } else {
                    middle > 128
                };
                if high {
                    (2 * (middle + destination - ((middle * destination) >> 8)) - 255).max(0)
                } else {
                    (2 * middle * destination) >> 8
                }
            }
            LinearLight => 2 * middle + destination - 255,
            PinLight => {
                if middle > 128 {
                    (2 * middle - 255).max(destination)
                } else {
                    (2 * middle).min(destination)
                }
            }
        };
        value.clamp(0, 255) as u8
    }
    /// Straight source coverage modulated with /128 tint arithmetic. Alpha
    /// belongs to the destination, except Scalar256 which stores its effective
    /// source coverage, including zero at opacity0.
    pub fn pixel(self, destination: [u8; 4], source: [u8; 4], opacity: u8) -> [u8; 4] {
        if matches!(
            self,
            Self::OpaqueScalar256 | Self::OpaqueScreen255 | Self::OpaqueMultiply255
        ) {
            return [
                self.channel(destination[0], source[0], opacity),
                self.channel(destination[1], source[1], opacity),
                self.channel(destination[2], source[2], opacity),
                destination[3],
            ];
        }
        if self == Self::Scalar256 {
            let alpha = if opacity == 255 {
                source[3]
            } else {
                ((u32::from(source[3]) * u32::from(opacity)) >> 8) as u8
            };
            return [
                self.channel(destination[0], source[0], alpha),
                self.channel(destination[1], source[1], alpha),
                self.channel(destination[2], source[2], alpha),
                alpha,
            ];
        }
        if matches!(self,Self::MaskSeparate256|Self::MaskPiecewise256{..}) {
            let alpha=if opacity==255 {source[3]} else {((u32::from(source[3])*u32::from(opacity))>>8) as u8};
            return [self.channel(destination[0],source[0],alpha),self.channel(destination[1],source[1],alpha),self.channel(destination[2],source[2],alpha),destination[3]];
        }
        if opacity == 0 {
            return destination;
        }
        let weight = (u32::from(opacity) >> 1) + (u32::from(opacity) >> 7);
        let alpha = ((u32::from(source[3]) * weight) >> 7) as u8;
        [
            self.channel(destination[0], source[0], alpha),
            self.channel(destination[1], source[1], alpha),
            self.channel(destination[2], source[2], alpha),
            destination[3],
        ]
    }
}
impl RgbLookup {
    /// Constant blends with explicit packed-word /256 arithmetic. Full-range
    /// weights use a+(a>>7), centered products (a>>1)+(a>>7), and normal
    /// interpolation (a+1)>>1. Alpha and hidden-RGB policy remain independent.
    /// These lookup tables also work in authored engine image jobs.
    pub fn constant_byte256(color: [u8; 4], mode: ConstantByteBlend) -> Self {
        let table = |channel: u8| {
            (0..=255u8)
                .map(|destination| {
                    if color[3] == 0 && !matches!(mode, ConstantByteBlend::OpaqueMultiply255) {
                        destination
                    } else {
                        mode.channel(destination, channel, color[3])
                    }
                })
                .collect()
        };
        Self {
            red: table(color[0]),
            green: table(color[1]),
            blue: table(color[2]),
            profile: RgbLookupProfile::StraightRgba,
        }
    }
}

#[cfg(test)]
mod opaque_tests {
    use super::ConstantByteBlend as B;
    #[test]
    fn native_mask_blends_match_checked_bitmap_kernels_at_endpoints_and_partial_masks() {
        use crate::bitmap::{BitmapRect,BlitOptions,PixelMath,blit_rgba8};
        for mask in [0,1,64,127,128,254,255] {
            for (mode,pixel) in std::iter::once((B::MaskSeparate256,PixelMath::MaskSeparate256))
                .chain([0,1,64,128,253,255].map(|weight|(B::MaskPiecewise256{weight},PixelMath::MaskPiecewise256{weight:u16::from(weight)}))) {
                let source=[201,127,3,mask];let destination=[97,111,255,73];
                let mut expected=destination;
                blit_rgba8(&mut expected,[1,1],&source,[1,1],BlitOptions{source_rect:BitmapRect::new(0,0,1,1),destination:[0,0],opacity:255,mode:pixel,color_key:None}).unwrap();
                assert_eq!(mode.pixel(destination,source,255),expected,"mask={mask} mode={mode:?}");
            }
        }
    }
    #[test]
    fn opaque_rgb_tables_cover_all_byte_pairs_and_keep_hidden_rgb() {
        for d in 0..=255u32 {
            for s in 0..=255u32 {
                let source = [s as u8; 4];
                let destination = [d as u8, d as u8, d as u8, 73];
                assert_eq!(
                    B::OpaqueScreen255.pixel(destination, source, 255),
                    [
                        (255 - (255 - d) * (255 - s) / 255) as u8,
                        (255 - (255 - d) * (255 - s) / 255) as u8,
                        (255 - (255 - d) * (255 - s) / 255) as u8,
                        73
                    ]
                );
                assert_eq!(
                    B::OpaqueMultiply255.pixel(destination, source, 0),
                    [
                        (d * s / 255) as u8,
                        (d * s / 255) as u8,
                        (d * s / 255) as u8,
                        73
                    ]
                );
            }
        }
        assert_eq!(
            B::OpaqueScreen255.pixel([27, 128, 255, 0], [173, 99, 42, 0], 0),
            [27, 128, 255, 0]
        );
        assert_eq!(
            B::OpaqueScreen255.pixel([1, 1, 1, 37], [254, 254, 254, 0], 128),
            [128, 128, 128, 37]
        );
        assert_eq!(
            B::OpaqueMultiply255.pixel([127, 255, 64, 9], [129, 254, 63, 0], 255),
            [64, 254, 15, 9]
        );
    }
}
