// Copyright (c)2026 CrySirror contributors. MPL-2.0.
//! JSON boundary only; normalized arithmetic belongs to Image/Runtime.
use anyhow::{bail, Result};
use crysirror_core::{image_blend::*, image_processing::*};
use serde::Deserialize;
#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Component {
    source: String,
    destination: String,
    equation: String,
}
impl Default for Component {
    fn default() -> Self {
        Self {
            source: "one".into(),
            destination: "zero".into(),
            equation: "add".into(),
        }
    }
}
#[derive(Default, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Arithmetic {
    #[default]
    FloatNearest,
    Byte255 {
        source: String,
        destination: String,
    },
}
#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
struct State {
    arithmetic: Arithmetic,
    color: Component,
    alpha: Component,
    constant: [f32; 4],
    write_mask: [bool; 4],
    source_scale: [f32; 4],
    source_bias: [f32; 4],
    source_alpha_range: [f32; 2],
    source_color_key: Option<[u8; 3]>,
    source_alpha_key: Option<u8>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            arithmetic: Default::default(),
            color: Default::default(),
            alpha: Default::default(),
            constant: [0.; 4],
            write_mask: [true; 4],
            source_scale: [1.; 4],
            source_bias: [0.; 4],
            source_alpha_range: [0., 1.],
            source_color_key: None,
            source_alpha_key: None,
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Options {
    source: [i32; 4],
    #[serde(default)]
    destination: [i32; 2],
    #[serde(default)]
    clip: Option<[i32; 4]>,
    #[serde(default)]
    state: State,
}
fn factor(s: &str) -> Result<RgbaBlendFactor> {
    use RgbaBlendFactor::*;
    Ok(match s {
        "zero" => Zero,
        "one" => One,
        "source_color" => SourceColor,
        "one_minus_source_color" => OneMinusSourceColor,
        "destination_color" => DestinationColor,
        "one_minus_destination_color" => OneMinusDestinationColor,
        "source_alpha" => SourceAlpha,
        "one_minus_source_alpha" => OneMinusSourceAlpha,
        "destination_alpha" => DestinationAlpha,
        "one_minus_destination_alpha" => OneMinusDestinationAlpha,
        "constant_color" => ConstantColor,
        "one_minus_constant_color" => OneMinusConstantColor,
        "constant_alpha" => ConstantAlpha,
        "one_minus_constant_alpha" => OneMinusConstantAlpha,
        "source_alpha_saturate" => SourceAlphaSaturate,
        _ => bail!("unknown RGBA blend factor {s}"),
    })
}
fn component(c: Component) -> Result<RgbaBlendComponent> {
    Ok(RgbaBlendComponent {
        source: factor(&c.source)?,
        destination: factor(&c.destination)?,
        equation: match c.equation.as_str() {
            "add" => RgbaBlendEquation::Add,
            "subtract" => RgbaBlendEquation::Subtract,
            "reverse_subtract" => RgbaBlendEquation::ReverseSubtract,
            "min" => RgbaBlendEquation::Min,
            "max" => RgbaBlendEquation::Max,
            _ => bail!("unknown RGBA blend equation"),
        },
    })
}
fn rounding(s: &str) -> Result<RgbaByteRounding> {
    Ok(match s {
        "floor" => RgbaByteRounding::Floor,
        "nearest" => RgbaByteRounding::Nearest,
        "ceil" => RgbaByteRounding::Ceil,
        _ => bail!("unknown RGBA byte rounding"),
    })
}
pub(crate) fn input(
    base: ImageInput,
    source: ImageInput,
    value: serde_json::Value,
) -> Result<ImageInput> {
    let options: Options = serde_json::from_value(value)?;
    let s = options.state;
    let state = RgbaBlendState {
        arithmetic: match s.arithmetic {
            Arithmetic::FloatNearest => RgbaBlendArithmetic::FloatNearest,
            Arithmetic::Byte255 {
                source,
                destination,
            } => RgbaBlendArithmetic::Byte255 {
                source: rounding(&source)?,
                destination: rounding(&destination)?,
            },
        },
        color: component(s.color)?,
        alpha: component(s.alpha)?,
        constant: s.constant,
        write_mask: s.write_mask,
        source_scale: s.source_scale,
        source_bias: s.source_bias,
        source_alpha_range: s.source_alpha_range,
        source_color_key: s.source_color_key,
        source_alpha_key: s.source_alpha_key,
    };
    state
        .validate()
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    Ok(ImageInput::Composite {
        base: Box::new(base),
        layers: vec![ImageCompositeLayer {
            input: Box::new(source),
            source: options.source,
            destination: options.destination,
            clip: options.clip,
            opacity: 255,
            math: ImageCompositeMath::Normalized(state),
        }],
    })
}
