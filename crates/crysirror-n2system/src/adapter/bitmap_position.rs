// SPDX-License-Identifier: MPL-2.0
//! Native bitmap constructor45DF00/4179E0. Unlike animation scalar channels,
//! constructor positions are integer strings, with integer half extents and
//! canvas-relative percentage/pixel suffixes. No fractional layout is inferred.
use super::*;

fn number(text: &str) -> Result<i32> {
    let sign = usize::from(text.starts_with('+') || text.starts_with('-'));
    let end = sign + text[sign..].bytes().take_while(u8::is_ascii_digit).count();
    if end == sign {
        return Ok(0);
    }
    text[..end]
        .parse()
        .context("N2System bitmap coordinate integer range")
}
fn integer_text(text: &str) -> bool {
    !text.is_empty()
        && text
            .bytes()
            .all(|b| b.is_ascii_digit() || b == b'+' || b == b'-')
}
impl N2SystemVm {
    pub(super) fn bitmap_position(
        &self,
        host: &mut dyn HostServices,
        parent: u32,
        value: &Value,
        size: u32,
        axis: usize,
    ) -> Result<i32> {
        ensure!(
            axis < 2 && size > 0 && size <= 16384,
            "N2System bitmap position extent/axis"
        );
        let canvas = i64::from(self.config.canvas[axis]);
        let mut origin = 0i64;
        let mut bounds = canvas;
        let mut inherited = 0i64;
        let mut ancestor = Some(parent);
        for depth in 0..128 {
            let Some(id) = ancestor else {
                break;
            };
            let entity = &self.entities.nodes[&id];
            if matches!(entity.kind, Kind::Visual | Kind::Image { .. }) {
                let layer = self.layer(host, id)?;
                let position = layer["rect"][axis]
                    .as_f64()
                    .context("N2System bitmap parent position")?;
                ensure!(
                    position.is_finite() && position.abs() <= 1_000_000.,
                    "N2System bitmap parent range"
                );
                origin = position.trunc() as i64;
                bounds = entity.size[axis].trunc() as i64;
                if entity.container != Some(false) {
                    inherited = origin;
                }
                break;
            }
            ancestor = entity.parent;
            ensure!(
                depth < 127 || ancestor.is_none(),
                "N2System bitmap parent depth"
            );
        }
        let text = value.string()?.trim().to_ascii_uppercase();
        let half = i64::from(size >> 1);
        let offset = |s: &str| -> Result<i64> {
            if let Some(n) = s.strip_suffix('%') {
                ensure!(integer_text(n), "N2System bitmap percent coordinate");
                return Ok((f64::from(number(n)?) / 100. * canvas as f64).trunc() as i64);
            }
            if let Some(n) = s.strip_suffix("PX").or_else(|| s.strip_suffix('P')) {
                ensure!(integer_text(n), "N2System bitmap pixel coordinate");
                return Ok(i64::from(number(n)?));
            }
            bail!("N2System bitmap coordinate suffix")
        };
        let position = if integer_text(&text) {
            inherited + i64::from(number(&text)?)
        } else if let Some(s) = text.strip_prefix('@') {
            if integer_text(s) {
                origin + i64::from(number(s)?)
            } else {
                inherited + offset(s)?
            }
        } else if let Some(s) = text.strip_prefix('#') {
            canvas / 2 - half + offset(s)?
        } else {
            match text.as_str() {
                "CENTER" if axis == 0 => canvas / 2 - half,
                "MIDDLE" if axis == 1 => canvas / 2 - half,
                "INHERIT" => origin + bounds / 2 - half,
                "INLEFT" if axis == 0 => origin,
                "INTOP" if axis == 1 => origin,
                "INRIGHT" if axis == 0 => origin + bounds - i64::from(size),
                "INBOTTOM" if axis == 1 => origin + bounds - i64::from(size),
                "ONLEFT" if axis == 0 => origin - half,
                "ONTOP" if axis == 1 => origin - half,
                "ONRIGHT" if axis == 0 => origin + bounds - half,
                "ONBOTTOM" if axis == 1 => origin + bounds - half,
                "OUTLEFT" if axis == 0 => origin - i64::from(size),
                "OUTTOP" if axis == 1 => origin - i64::from(size),
                "OUTRIGHT" if axis == 0 => origin + bounds,
                "OUTBOTTOM" if axis == 1 => origin + bounds,
                _ => bail!("N2System invalid original bitmap coordinate {text:?}"),
            }
        };
        ensure!(
            position.unsigned_abs() <= 1_000_000,
            "N2System bitmap coordinate bounds"
        );
        Ok(i32::try_from(position)?)
    }
}
