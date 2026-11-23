// SPDX-License-Identifier: MPL-2.0
use super::{Program, Var};
use anyhow::{Context, Result, bail, ensure};

impl Program {
    /// VAR captures its destination and chooses a source before ReplaceVars.
    /// Each leading '*' follows one additional entry in the variable dictionary.
    pub(super) fn assign_variable(&mut self, args: &[String]) -> Result<()> {
        let [ty, name, alternatives @ ..] = args else { bail!("VAR requires a type and destination") };
        let choice = if alternatives.len() > 1 { self.random(alternatives.len()) } else { 0 };
        let raw = alternatives.get(choice).map_or("", String::as_str);
        let key = raw.trim_start_matches('*');
        let mut value = key.to_owned();
        for _ in 0..raw.len() - key.len() {
            value = self.vars.get(&value).with_context(|| format!("unknown VAR reference {value:?}"))?.as_text();
        }
        self.vars.insert(name.clone(), Var::parse(ty, &value));
        Ok(())
    }

    /// Base.Vector.Do (GameAssembly RVA A41C40), over CommandController.V3Dic.
    pub(super) fn vector(&mut self, args: &[String]) -> Result<()> {
        let [name, op, rest @ ..] = args else { bail!("Vector requires a name and operator") };
        let coordinates = || {
            let mut xyz = [0.; 3];
            for (axis, text) in xyz.iter_mut().zip(rest) {
                if let Ok(n) = text.trim().parse::<f32>() {
                    *axis = n;
                }
            }
            xyz
        };
        let value = match op.as_str() {
            "=" => {
                let first = rest.first().context("Vector assignment requires a source")?;
                self.vectors.get(first).copied().unwrap_or_else(coordinates)
            }
            "+=" | "-=" if self.vectors.contains_key(name) => {
                let first = rest.first().context("Vector arithmetic requires a source")?;
                let delta = if first.trim().parse::<f32>().is_ok() {
                    coordinates()
                } else {
                    *self.vectors.get(first).with_context(|| format!("unknown vector {first:?}"))?
                };
                let initial = self.vectors[name];
                if op == "+=" {
                    std::array::from_fn(|i| initial[i] + delta[i])
                } else {
                    std::array::from_fn(|i| initial[i] - delta[i])
                }
            }
            // Native initializes the result to Vector3.zero, then stores it
            // even for an unknown operator or an absent arithmetic destination.
            _ => [0.; 3],
        };
        self.vectors.insert(name.clone(), value);
        Ok(())
    }

    /// Base.Format.ConvertBeforeArgsProc / Do (A2CF40 / A2D200). Keep the
    /// boxed value through formatting; string arguments do not use a numeric
    /// mask. GetArgToSplitLast splits each remaining argument on commas.
    pub(super) fn format(&mut self, args: &[String]) -> Result<()> {
        let [name, template, rest @ ..] = args else { bail!("Format requires a name and template") };
        let parameters: Vec<_> = rest.iter().flat_map(|s| s.split(',')).collect();
        let mut text = String::new();
        let mut chars = template.chars().peekable();
        while let Some(ch) = chars.next() {
            if matches!(ch, '{' | '}') && chars.next_if_eq(&ch).is_some() {
                text.push(ch);
            } else if ch == '{' {
                let mut field = String::new();
                loop {
                    let ch = chars.next().context("unterminated Format field")?;
                    if ch == '}' { break }
                    field.push(ch);
                }
                let (field, spec) = field.split_once(':').unwrap_or((&field, ""));
                let (index, width) = field.split_once(',').unwrap_or((field, "0"));
                let key = *parameters.get(index.trim().parse::<usize>()?).context("Format index out of range")?;
                let width: i32 = width.trim().parse()?;
                ensure!(width.unsigned_abs() < 1_000_000, "Format alignment too large");
                let value = match self.vars.get(key) {
                    None => key.to_owned(),
                    Some(Var::Str(s)) => s.clone(),
                    Some(Var::Bool(b)) => if *b { "True" } else { "False" }.to_owned(),
                    Some(v) if spec.is_empty() => v.as_text(),
                    Some(Var::Int(n)) if spec.bytes().all(|c| c == b'0') => {
                        let digits = format!("{:0width$}", n.unsigned_abs(), width = spec.len());
                        if *n < 0 { format!("-{digits}") } else { digits }
                    }
                    Some(_) => bail!("unsupported numeric Format specifier {spec:?}"),
                };
                let padding = (width.unsigned_abs() as usize).saturating_sub(value.encode_utf16().count());
                if width > 0 { text.extend(std::iter::repeat_n(' ', padding)); }
                text.push_str(&value);
                if width < 0 { text.extend(std::iter::repeat_n(' ', padding)); }
            } else {
                ensure!(ch != '}', "unmatched Format brace");
                text.push(ch);
            }
        }
        self.vars.insert(name.clone(), Var::Str(text));
        Ok(())
    }
}

/// ILLGAMES.StringExtensions.GetColorCheck (331B150). A missing or invalid
/// HTML color means the caller restores the SceneFadeCanvas's initial color.
pub(super) fn fade_color(s: &str) -> Option<[f32; 4]> {
    let mut parts = s.split(',');
    if parts.clone().count() >= 3 {
        let mut color = [0., 0., 0., 1.];
        for component in &mut color {
            if let Some(value) = parts.next().and_then(|s| s.trim().parse::<f32>().ok()) {
                *component = if value > 1. { (value / 255.).min(1.) } else { value };
            }
        }
        return Some(color);
    }
    // UnityPlayer.dll ColorUtility names followed by their RGBA32 table.
    let hex = match s.to_ascii_lowercase().as_str() {
        "red" => "ff0000", "cyan" | "aqua" => "00ffff", "blue" => "0000ff",
        "darkblue" => "00008b", "lightblue" => "add8e6", "purple" => "800080",
        "yellow" => "ffff00", "lime" => "00ff00", "fuchsia" | "magenta" => "ff00ff",
        "white" => "ffffff", "silver" => "c0c0c0", "grey" => "808080", "black" => "000000",
        "orange" => "ffa500", "brown" => "a52a2a", "maroon" => "800000", "green" => "008000",
        "olive" => "808000", "navy" => "000080", "teal" => "008080", "transparent" => "00000000",
        _ => s.strip_prefix('#')?,
    };
    if !hex.is_ascii() { return None }
    let mut rgba = [255u8; 4];
    match hex.len() {
        3 | 4 => {
            for (out, c) in rgba.iter_mut().zip(hex.chars()) {
                *out = c.to_digit(16)? as u8 * 17;
            }
        }
        6 | 8 => {
            for (out, pair) in rgba.iter_mut().zip(hex.as_bytes().chunks_exact(2)) {
                *out = u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok()?;
            }
        }
        _ => return None,
    }
    Some(rgba.map(|n| f32::from(n) / 255.))
}
