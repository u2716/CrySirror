//! Stored-byte constant blends. RGB tables preserve destination alpha and
//! encode packed-word truncation before the shared image worker applies them.
use super::{RgbLookup, RgbLookupProfile};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
}
impl RgbLookup {
    /// Constant blends with explicit packed-word /256 arithmetic. Full-range
    /// weights use a+(a>>7), centered products (a>>1)+(a>>7), and normal
    /// interpolation (a+1)>>1. Alpha and hidden-RGB policy remain independent.
    /// These lookup tables also work in authored engine image jobs.
    pub fn constant_byte256(color: [u8; 4], mode: ConstantByteBlend) -> Self {
        let alpha = i32::from(color[3]);
        let full = alpha + (alpha >> 7);
        let half = (alpha >> 1) + (alpha >> 7);
        let table = |channel: u8| {
            let color = i32::from(channel);
            let positive = (color * full) >> 8;
            let white = 255 + (((color - 255) * half) >> 7);
            let middle = 128 + (((color - 128) * half) >> 7);
            (0..=255i32)
                .map(|destination| {
                    if alpha == 0 {
                        return destination as u8;
                    }
                    use ConstantByteBlend::*;
                    let value = match mode {
                        Normal => destination + (((color - destination) * ((alpha + 1) >> 1)) >> 7),
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
                            let high = if mode == Overlay {
                                destination > 128
                            } else {
                                middle > 128
                            };
                            if high {
                                (2 * (middle + destination - ((middle * destination) >> 8)) - 255)
                                    .max(0)
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
