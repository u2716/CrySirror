// SPDX-License-Identifier: MPL-2.0
//! TQMO 1.0 upgrade (5770D8 ->577510). Model material names carry scene
//! annotations after backslashes; 575214 splits outside quotes/parentheses.
use super::*;
impl Model {
    /// Upgrade the native v1.0 model exactly once. Original source version
    /// remains available independently of the upgraded geometry/state.
    pub fn upgrade_legacy(&mut self) -> Result<()> {
        if self.version != 10 || self.legacy_upgraded {
            return Ok(());
        }
        for object in &mut self.objects {
            object.scale_basis([1., -1., 1.])?;
        }
        for material in &mut self.materials {
            let sections = split(&material.name, '\\');
            for section in sections.iter().skip(1) {
                let mut args = split(section, ',');
                args.resize(args.len().max(4), String::new());
                match args[0].to_ascii_lowercase().as_str() {
                    "sa" => self.scene.ambient_argb = argb(&args[1]),
                    "ss" => self.scene.base_argb = argb(&args[1]),
                    "dl" => {
                        let v = [
                            float(&args[1], 0.),
                            -float(&args[2], 1.),
                            float(&args[3], 1.),
                        ];
                        self.scene.light_direction =
                            glam::Vec3::from_array(v).normalize_or_zero().to_array();
                    }
                    "bm" => material.native_render_flags[0] = integer(&args[1], 0) as u32,
                    "tf" => material.native_render_flags[1] = integer(&args[1], 0) as u32,
                    "fog" => {
                        self.scene.fog_near = float(&args[1], 100000.);
                        self.scene.fog_far = float(&args[2], 100000.);
                        self.scene.fog_mode = float(&args[3], 1.);
                    }
                    "fogo" => {
                        self.scene.extra_vectors[0] = [
                            float(&args[1], 0.),
                            -float(&args[2], 0.),
                            float(&args[3], 0.),
                        ]
                    }
                    "fogm" => {
                        self.scene.extra_vectors[1] = [
                            float(&args[1], 0.),
                            float(&args[2], 0.),
                            float(&args[3], 1.),
                        ]
                    }
                    "fogc" => {
                        if !args[1].is_empty() {
                            let rgb = argb(&args[1]);
                            self.scene.fog_argb[1..].copy_from_slice(&rgb[1..]);
                        }
                        if !args[2].is_empty() {
                            self.scene.fog_argb[0] =
                                f32::from(u32::from_str_radix(&args[2], 16).unwrap_or(255) as u8)
                                    / 255.;
                        }
                    }
                    // Native only interprets the above annotation keys.
                    _ => {}
                }
            }
        }
        // 5770D8 applies this final direction correction after 577510.
        self.scene.light_direction[2] = -self.scene.light_direction[2];
        self.legacy_upgraded = true;
        Ok(())
    }
}
fn split(text: &str, delimiter: char) -> Vec<String> {
    let mut out = Vec::new();
    let mut token = String::new();
    let mut quoted = false;
    let mut parentheses = 0i32;
    for ch in text.chars() {
        if ch == '\t' {
            continue;
        }
        if ch == '"' {
            quoted = !quoted;
            continue;
        }
        if !quoted {
            if ch == '(' {
                parentheses += 1;
            }
            if ch == ')' {
                parentheses -= 1;
            }
        }
        if !quoted && parentheses == 0 && ch == delimiter {
            out.push(std::mem::take(&mut token));
        } else {
            token.push(ch);
        }
    }
    if !token.is_empty() {
        out.push(token);
    }
    out
}
fn float(text: &str, fallback: f32) -> f32 {
    text.trim()
        .parse::<f32>()
        .ok()
        .filter(|v| v.is_finite())
        .unwrap_or(fallback)
}
fn integer(text: &str, fallback: i32) -> i32 {
    let text = text.trim();
    let hex = text
        .strip_prefix('$')
        .or_else(|| text.strip_prefix("0x"))
        .or_else(|| text.strip_prefix("0X"));
    if let Some(hex) = hex {
        u32::from_str_radix(hex, 16)
            .map(|v| v as i32)
            .unwrap_or(fallback)
    } else {
        text.parse().unwrap_or(fallback)
    }
}
fn argb(text: &str) -> [f32; 4] {
    let value = u32::from_str_radix(text.trim(), 16).unwrap_or(0xffffff);
    [
        1.,
        ((value >> 16) & 255) as f32 / 255.,
        ((value >> 8) & 255) as f32 / 255.,
        (value & 255) as f32 / 255.,
    ]
}
