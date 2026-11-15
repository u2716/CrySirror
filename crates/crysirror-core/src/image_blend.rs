// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Explicit normalized RGBA fragment/blend state, independent of GPU and VM.
use crate::{Error, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RgbaBlendFactor {
    Zero,
    One,
    SourceColor,
    OneMinusSourceColor,
    DestinationColor,
    OneMinusDestinationColor,
    SourceAlpha,
    OneMinusSourceAlpha,
    DestinationAlpha,
    OneMinusDestinationAlpha,
    ConstantColor,
    OneMinusConstantColor,
    ConstantAlpha,
    OneMinusConstantAlpha,
    /// RGB min(sourceAlpha,1-destinationAlpha); alpha factor is one.
    SourceAlphaSaturate,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RgbaBlendEquation {
    Add,
    Subtract,
    ReverseSubtract,
    /// Min/Max compare components directly and ignore both factors.
    Min,
    Max,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RgbaBlendComponent {
    pub source: RgbaBlendFactor,
    pub destination: RgbaBlendFactor,
    pub equation: RgbaBlendEquation,
}
impl Default for RgbaBlendComponent {
    fn default() -> Self {
        Self {
            source: RgbaBlendFactor::One,
            destination: RgbaBlendFactor::Zero,
            equation: RgbaBlendEquation::Add,
        }
    }
}
/// Quantization of one nonnegative byte product divided by255.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RgbaByteRounding {
    Floor,
    Nearest,
    Ceil,
}
/// Explicit arithmetic profiles. Byte255 is integer table arithmetic, including
/// transform quantization before either product. Constants/scales/biases become
/// nearest signed byte-unit coefficients; transform is floor(s*scale255/255)+
/// bias255, clamped0..255. Product rounding is independent for the two terms;
/// equation results clamp0..255. Min/Max ignore product factors/rounding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RgbaBlendArithmetic {
    FloatNearest,
    Byte255 {
        source: RgbaByteRounding,
        destination: RgbaByteRounding,
    },
    /// Byte255 transforms and factors, but the signed sum of both unscaled
    /// products is divided by255 once: `round((s*fs +/- d*fd)/255)`. Native
    /// integer alpha blitters (e.g. Circus DC2 40B5F0) divide the whole sum.
    Byte255Sum { rounding: RgbaByteRounding },
    /// Integer power-of-two weights: `(s*fs +/- d*fd) >> shift`, where only
    /// One/Zero/Constant*/OneMinusConstant* factors are admitted and constants
    /// are rounded to multiples of `1/(1<<shift)` (e.g. 16-level native blits).
    ByteShiftSum { shift: u8 },
}
/// CPU encoded-UNORM8 profile. Raw source keys and inclusive alpha range discard
/// a whole fragment before the source transform. The fragment is then
/// clamp(source/255*scale+bias,0,1), without intermediate byte quantization.
/// Factors read that fragment and the original destination/255. Color and alpha
/// have independent equations; final channels clamp0..1 and round to UNORM8.
/// The default arithmetic is binary32. Byte255 is selected explicitly. There is no sRGB conversion, alpha unpremultiply,
/// /256 approximation, or claim of bit-identical rounding on every GPU driver.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RgbaBlendState {
    pub arithmetic: RgbaBlendArithmetic,
    pub color: RgbaBlendComponent,
    pub alpha: RgbaBlendComponent,
    pub constant: [f32; 4],
    pub write_mask: [bool; 4],
    /// Each scale and bias is finite within -64..64.
    pub source_scale: [f32; 4],
    pub source_bias: [f32; 4],
    /// Inclusive raw source alpha range in0..1; default[0,1] discards nothing.
    pub source_alpha_range: [f32; 2],
    /// Exact original encoded RGB/alpha keys, before source transformation.
    pub source_color_key: Option<[u8; 3]>,
    pub source_alpha_key: Option<u8>,
    /// Invert `source_color_key`: only fragments equal to the key are kept
    /// (native "copy key colour only" hole punches).
    pub source_color_key_invert: bool,
}
impl Default for RgbaBlendState {
    fn default() -> Self {
        Self {
            arithmetic: RgbaBlendArithmetic::FloatNearest,
            color: Default::default(),
            alpha: Default::default(),
            constant: [0.; 4],
            write_mask: [true; 4],
            source_scale: [1.; 4],
            source_bias: [0.; 4],
            source_alpha_range: [0., 1.],
            source_color_key: None,
            source_alpha_key: None,
            source_color_key_invert: false,
        }
    }
}
impl RgbaBlendState {
    pub fn validate(&self) -> Result<()> {
        if self
            .constant
            .iter()
            .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
        {
            return Err(Error::Invalid("RGBA blend constant must be finite0..1"));
        }
        if self
            .source_scale
            .iter()
            .chain(&self.source_bias)
            .any(|v| !v.is_finite() || v.abs() > 64.)
        {
            return Err(Error::Invalid(
                "RGBA source transform must be finite bounded64",
            ));
        }
        if self
            .source_alpha_range
            .iter()
            .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
            || self.source_alpha_range[0] > self.source_alpha_range[1]
        {
            return Err(Error::Invalid("RGBA source alpha range"));
        }
        if self.source_color_key_invert && self.source_color_key.is_none() {
            return Err(Error::Invalid("RGBA inverted colour key requires a key"));
        }
        if let RgbaBlendArithmetic::ByteShiftSum { shift } = self.arithmetic {
            use RgbaBlendFactor as F;
            let admitted = |f: RgbaBlendFactor| {
                matches!(
                    f,
                    F::One | F::Zero | F::ConstantColor | F::OneMinusConstantColor
                        | F::ConstantAlpha | F::OneMinusConstantAlpha
                )
            };
            if !(1..=8).contains(&shift)
                || ![self.color, self.alpha].iter().all(|c| {
                    admitted(c.source) && admitted(c.destination)
                        && matches!(c.equation, RgbaBlendEquation::Add | RgbaBlendEquation::Subtract | RgbaBlendEquation::ReverseSubtract)
                })
                || self.source_scale != [1.; 4]
                || self.source_bias != [0.; 4]
            {
                return Err(Error::Invalid("RGBA shift arithmetic admits constant factors only"));
            }
        }
        Ok(())
    }
}
