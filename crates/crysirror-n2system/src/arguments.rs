// SPDX-License-Identifier: MPL-2.0
// Native argument conversions: NitroSharp BuiltInFunctionDispatcher.cs,
// e3869842 (MIT). Values retain references for output arguments.
use crate::{
    catalog::CONSTANTS,
    source::Name,
    value::Value,
    vm::{Argument, Call},
};
use anyhow::{bail, ensure, Context, Result};

pub struct Arguments<'a> {
    args: &'a [Argument],
}
impl<'a> Arguments<'a> {
    pub fn new(call: &'a Call) -> Self {
        Self {
            args: &call.arguments,
        }
    }
    pub fn len(&self) -> usize {
        self.args.len()
    }
    pub fn value(&self, at: usize) -> &Value {
        self.args
            .get(at)
            .map(|a| a.value.scalar())
            .unwrap_or(&Value::Null)
    }
    pub fn string(&self, at: usize) -> Result<String> {
        if at >= self.len() {
            Ok(String::new())
        } else {
            self.value(at).string()
        }
    }
    pub fn integer(&self, at: usize) -> Result<i32> {
        Ok(match self.value(at) {
            Value::Number(n) => {
                ensure!(
                    n.is_finite() && *n >= i32::MIN as f32 && f64::from(*n) <= i32::MAX as f64,
                    "N2System integer range"
                );
                n.trunc() as i32
            }
            Value::String(s) => s.parse().context("N2System integer string")?,
            Value::Null => 0,
            _ => bail!("N2System integer argument {}", at + 1),
        })
    }
    pub fn priority(&self, at: usize) -> Result<i32> {
        match self.value(at) {
            Value::Number(n) => {
                ensure!(n.is_finite(), "N2System nonfinite priority");
                // Shipped Totono NSS uses 3000000000 for frontmost controls.
                // Keep this saturation local to draw ordering; counts and
                // resource indices must still reject overflowing integers.
                Ok(n.trunc() as i32)
            }
            Value::String(s) => {
                // The native CreateColor priority conversion accepts an atol
                // prefix (including the shipped `20000i` token).
                let s = s.trim_start();
                let sign = usize::from(s.starts_with('+') || s.starts_with('-'));
                let end = sign + s[sign..].bytes().take_while(u8::is_ascii_digit).count();
                ensure!(end > sign, "N2System priority requires digits");
                let number: f64 = s[..end].parse()?;
                Ok(number as i32)
            }
            _ => self.integer(at),
        }
    }
    pub fn time(&self, at: usize) -> Result<u64> {
        let n = match self.value(at) {
            Value::Null => 0.,
            Value::Number(n) => f64::from(*n),
            Value::String(s) => s.parse::<i32>().context("N2System time string")? as f64,
            _ => bail!("N2System time argument {}", at + 1),
        };
        ensure!(
            n.is_finite() && (0. ..=86_400_000.).contains(&n),
            "N2System wait duration range"
        );
        Ok(n.round() as u64)
    }
    pub fn truth(&self, at: usize) -> Result<bool> {
        if at >= self.len() {
            Ok(false)
        } else {
            self.value(at).truth()
        }
    }
    pub fn constant(&self, at: usize) -> Result<&'static str> {
        match self.value(at) {
            Value::Constant(id) => CONSTANTS
                .get(*id as usize)
                .copied()
                .context("N2System constant range"),
            _ => bail!("N2System constant argument {}", at + 1),
        }
    }
    pub fn reference(&self, at: usize) -> Result<&Name> {
        self.args
            .get(at)
            .and_then(|a| a.reference.as_ref())
            .context("N2System output argument must be a variable reference")
    }
    pub fn color(&self, at: usize) -> Result<[u8; 4]> {
        let rgb = match self.value(at) {
            Value::Number(_) => self.integer(at)? as u32,
            Value::String(s) => {
                if let Ok(rgb) = u32::from_str_radix(s.trim_start_matches('#'), 16) {
                    rgb
                } else {
                    return color_named(s);
                }
            }
            Value::Constant(_) => return color_named(self.constant(at)?),
            _ => bail!("N2System color argument {}", at + 1),
        };
        Ok([(rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8, 255])
    }
    pub fn anim_wait(&self, at: usize, duration: u64) -> Result<u64> {
        if at >= self.len() {
            return Ok(0);
        }
        match self.value(at) {
            Value::Boolean(b) => Ok(if *b { duration } else { 0 }),
            Value::Number(n) if *n == 1. => Ok(duration),
            _ => self.time(at),
        }
    }
}
fn color_named(name: &str) -> Result<[u8; 4]> {
    Ok(match name.to_ascii_lowercase().as_str() {
        "black" => [0, 0, 0, 255],
        "white" | "whited" => [255; 4],
        "gray" => [128, 128, 128, 255],
        "red" => [255, 0, 0, 255],
        "green" => [0, 128, 0, 255],
        "blue" => [0, 0, 255, 255],
        "yellow" => [255, 255, 0, 255],
        _ => bail!("N2System unknown color {name}"),
    })
}

pub fn coordinate(
    value: &Value,
    current: f32,
    size: f32,
    parent_size: f32,
    axis: usize,
) -> Result<f32> {
    let n = match value {
        Value::Number(n) => *n,
        Value::Delta(n) => current + n,
        Value::Constant(id) => match CONSTANTS
            .get(*id as usize)
            .copied()
            .context("N2System coordinate constant")?
        {
            "Inherit" => 0.,
            "Center" | "Middle" => (parent_size - size) * 0.5,
            "InLeft" if axis == 0 => 0.,
            "InTop" if axis == 1 => 0.,
            "OnLeft" if axis == 0 => -size * 0.5,
            "OnTop" if axis == 1 => -size * 0.5,
            "OutLeft" | "Left" if axis == 0 => -size,
            "OutTop" | "Top" if axis == 1 => -size,
            "InRight" if axis == 0 => parent_size - size,
            "InBottom" if axis == 1 => parent_size - size,
            "OnRight" if axis == 0 => parent_size - size * 0.5,
            "OnBottom" if axis == 1 => parent_size - size * 0.5,
            "OutRight" | "Right" if axis == 0 => parent_size,
            "OutBottom" | "Bottom" if axis == 1 => parent_size,
            name => bail!("N2System coordinate {name} on axis {axis}"),
        },
        _ => bail!("N2System numeric/alignment coordinate required"),
    };
    ensure!(n.is_finite(), "N2System nonfinite coordinate");
    Ok(n)
}
