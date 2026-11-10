//! Stored-byte ramps with explicit single-precision intermediates and
//! half-away-from-zero byte rounding. Alpha stays owned by the source image.
use super::{RgbLookup, RgbLookupProfile};
use anyhow::{Result, ensure};
impl RgbLookup {
    pub fn identity() -> Self {
        Self::uniform((0..=255u8).collect())
    }
    fn uniform(table: Vec<u8>) -> Self {
        Self {
            red: table.clone(),
            green: table.clone(),
            blue: table,
            profile: RgbLookupProfile::StraightRgba,
        }
    }
    /// Map two input/output anchors. Equal input anchors produce a step at
    /// that byte; equal output anchors produce a constant table.
    pub fn linear_byte_ramp(input: [i32; 2], output: [i32; 2]) -> Result<Self> {
        ensure!(
            input
                .iter()
                .chain(&output)
                .all(|n| (-65535..=65535).contains(n)),
            "RGB ramp anchor bounds"
        );
        if input[0] == input[1] {
            return Ok(Self::uniform(
                (0..=255)
                    .map(|v| output[usize::from(v >= input[0])].clamp(0, 255) as u8)
                    .collect(),
            ));
        }
        let slope = (f64::from(output[1] - output[0]) / f64::from(input[1] - input[0])) as f32;
        let intercept = (f64::from(output[0]) - f64::from(input[0]) * f64::from(slope)) as f32;
        Ok(Self::uniform(
            (0..=255)
                .map(|v| {
                    let value = (f64::from(v) * f64::from(slope) + f64::from(intercept)) as f32;
                    value.round().clamp(0., 255.) as u8
                })
                .collect(),
        ))
    }
    /// Power curve with rounded encoded-byte output; hidden RGB participates.
    pub fn gamma_byte_ramp(gamma: f32) -> Result<Self> {
        ensure!(
            gamma.is_finite() && gamma > 0.,
            "RGB gamma must be positive and finite"
        );
        Ok(Self::uniform(
            (0..=255)
                .map(|v| {
                    let value = ((f64::from(v) / 255.).powf(1. / f64::from(gamma)) * 255.) as f32;
                    value.round().clamp(0., 255.) as u8
                })
                .collect(),
        ))
    }
    /// Centred contrast with a125-byte half-window and250-byte brightness
    /// travel. Inputs clamp to -1..1; anchor rounding precedes ramp sampling.
    pub fn brightness_contrast_byte250(brightness: f32, contrast: f32) -> Result<Self> {
        ensure!(
            brightness.is_finite() && contrast.is_finite(),
            "RGB brightness/contrast must be finite"
        );
        let brightness = brightness.clamp(-1., 1.);
        let contrast = contrast.clamp(-1., 1.);
        let shift = (f64::from(brightness) * 250.) as f32;
        let edge = (f64::from(contrast.abs()) * 125.) as f32;
        let high = (255. - f64::from(contrast.abs()) * 125.) as f32;
        let (shift, edge, high) = (
            shift.round() as i32,
            edge.round() as i32,
            high.round() as i32,
        );
        if contrast >= 0. {
            Self::linear_byte_ramp([edge - shift, high - shift], [0, 255])
        } else {
            Self::linear_byte_ramp([0, 255], [edge + shift, high + shift])
        }
    }
}
