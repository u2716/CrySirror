// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Normalized fixed-factor arithmetic on the shared checked bitmap visitor.
use crate::{
    ImageSurface,
    bitmap::{BitmapRect, BlitOptions, PixelMath, visit_blit_rgba8},
};
use anyhow::Result;
use crysirror_core::image_blend::{
    RgbaBlendArithmetic, RgbaBlendComponent, RgbaBlendEquation as E, RgbaBlendFactor as F,
    RgbaBlendState, RgbaByteRounding,
};
fn factor(f: F, c: usize, s: &[f32; 4], d: &[f32; 4], k: &[f32; 4]) -> f32 {
    match f {
        F::Zero => 0.,
        F::One => 1.,
        F::SourceColor => s[c],
        F::OneMinusSourceColor => 1. - s[c],
        F::DestinationColor => d[c],
        F::OneMinusDestinationColor => 1. - d[c],
        F::SourceAlpha => s[3],
        F::OneMinusSourceAlpha => 1. - s[3],
        F::DestinationAlpha => d[3],
        F::OneMinusDestinationAlpha => 1. - d[3],
        F::ConstantColor => k[c],
        F::OneMinusConstantColor => 1. - k[c],
        F::ConstantAlpha => k[3],
        F::OneMinusConstantAlpha => 1. - k[3],
        F::SourceAlphaSaturate => {
            if c == 3 {
                1.
            } else {
                s[3].min(1. - d[3])
            }
        }
    }
}
fn component(mode: RgbaBlendComponent, c: usize, s: &[f32; 4], d: &[f32; 4], k: &[f32; 4]) -> u8 {
    let value = match mode.equation {
        E::Min => s[c].min(d[c]),
        E::Max => s[c].max(d[c]),
        eq => {
            let source = s[c] * factor(mode.source, c, s, d, k);
            let destination = d[c] * factor(mode.destination, c, s, d, k);
            match eq {
                E::Add => source + destination,
                E::Subtract => source - destination,
                E::ReverseSubtract => destination - source,
                _ => unreachable!(),
            }
        }
    };
    (value.clamp(0., 1.) * 255.).round() as u8
}
fn byte_factor(f: F, c: usize, s: &[i32; 4], d: &[i32; 4], k: &[i32; 4]) -> i32 {
    match f {
        F::Zero => 0,
        F::One => 255,
        F::SourceColor => s[c],
        F::OneMinusSourceColor => 255 - s[c],
        F::DestinationColor => d[c],
        F::OneMinusDestinationColor => 255 - d[c],
        F::SourceAlpha => s[3],
        F::OneMinusSourceAlpha => 255 - s[3],
        F::DestinationAlpha => d[3],
        F::OneMinusDestinationAlpha => 255 - d[3],
        F::ConstantColor => k[c],
        F::OneMinusConstantColor => 255 - k[c],
        F::ConstantAlpha => k[3],
        F::OneMinusConstantAlpha => 255 - k[3],
        F::SourceAlphaSaturate => {
            if c == 3 {
                255
            } else {
                s[3].min(255 - d[3])
            }
        }
    }
}
fn byte_product(value: i32, round: RgbaByteRounding) -> i32 {
    match round {
        RgbaByteRounding::Floor => value / 255,
        RgbaByteRounding::Nearest => (value + 127) / 255,
        RgbaByteRounding::Ceil => (value + 254) / 255,
    }
}
struct ByteTransform {
    scale: [i32; 4],
    bias: [i32; 4],
    constant: [i32; 4],
    source: RgbaByteRounding,
    destination: RgbaByteRounding,
}
impl ByteTransform {
    fn prepare(state: &RgbaBlendState) -> Option<Self> {
        let RgbaBlendArithmetic::Byte255 {
            source,
            destination,
        } = state.arithmetic
        else {
            return None;
        };
        Some(Self {
            scale: state.source_scale.map(|v| (v * 255.).round() as i32),
            bias: state.source_bias.map(|v| (v * 255.).round() as i32),
            constant: state.constant.map(|v| (v * 255.).round() as i32),
            source,
            destination,
        })
    }
}
fn combine_bytes(dst: &mut [u8], source: &[u8], state: &RgbaBlendState, transform: &ByteTransform) {
    let s = core::array::from_fn(|c| {
        (i32::from(source[c]) * transform.scale[c])
            .div_euclid(255)
            .saturating_add(transform.bias[c])
            .clamp(0, 255)
    });
    let d = core::array::from_fn(|c| i32::from(dst[c]));
    let k = transform.constant;
    for c in 0..4 {
        if !state.write_mask[c] {
            continue;
        }
        let mode = if c == 3 { state.alpha } else { state.color };
        let value = match mode.equation {
            E::Min => s[c].min(d[c]),
            E::Max => s[c].max(d[c]),
            eq => {
                let source = byte_product(
                    s[c] * byte_factor(mode.source, c, &s, &d, &k),
                    transform.source,
                );
                let destination = byte_product(
                    d[c] * byte_factor(mode.destination, c, &s, &d, &k),
                    transform.destination,
                );
                match eq {
                    E::Add => source + destination,
                    E::Subtract => source - destination,
                    E::ReverseSubtract => destination - source,
                    _ => unreachable!(),
                }
            }
        };
        dst[c] = value.clamp(0, 255) as u8;
    }
}
fn combine(dst: &mut [u8], source: &[u8], state: &RgbaBlendState, bytes: Option<&ByteTransform>) {
    let alpha = f32::from(source[3]) / 255.;
    if state.source_color_key.is_some_and(|key| source[..3] == key)
        || state.source_alpha_key == Some(source[3])
        || alpha < state.source_alpha_range[0]
        || alpha > state.source_alpha_range[1]
    {
        return;
    }
    if let Some(transform) = bytes {
        combine_bytes(dst, source, state, transform);
        return;
    }
    let s = core::array::from_fn(|c| {
        (f32::from(source[c]) / 255. * state.source_scale[c] + state.source_bias[c]).clamp(0., 1.)
    });
    let d = core::array::from_fn(|c| f32::from(dst[c]) / 255.);
    for c in 0..4 {
        if state.write_mask[c] {
            dst[c] = component(
                if c == 3 { state.alpha } else { state.color },
                c,
                &s,
                &d,
                &state.constant,
            );
        }
    }
}
impl ImageSurface {
    /// Blend immutable source pixels with explicit normalized factors/fragment
    /// state. Clipping uses the same checked visitor as ordinary blit. Invalid
    /// descriptors fail before mutation. Cancellation may leave a partial private
    /// destination, which callers must discard rather than publish.
    pub fn blend_fixed_factors(
        &mut self,
        source: &Self,
        rect: BitmapRect,
        destination: [i32; 2],
        state: RgbaBlendState,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<()> {
        state
            .validate()
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        let bytes = ByteTransform::prepare(&state);
        visit_blit_rgba8(
            &mut self.rgba,
            [self.width, self.height],
            &source.rgba,
            [source.width, source.height],
            BlitOptions {
                source_rect: rect,
                destination,
                opacity: 255,
                mode: PixelMath::Copy,
                color_key: None,
            },
            check,
            &mut |d, s| combine(d, s, &state, bytes.as_ref()),
        )
    }
}
