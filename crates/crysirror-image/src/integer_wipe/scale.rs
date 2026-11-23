// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Fixed16.16 sampling for centre zooms and expanding image panels.
use crate::ImageSurface;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ScaleWipe {
    CentreZoom { level: u8 },
    ExpandingPanel { reverse: bool, blend: bool },
}
impl ScaleWipe {
    pub fn validate(self) -> Result<()> {
        if let Self::CentreZoom { level } = self {
            ensure!(level < 4, "centre zoom level");
        }
        Ok(())
    }
    pub(super) fn apply(
        self,
        old: &ImageSurface,
        new: &ImageSurface,
        count: i64,
        duration: i64,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<ImageSurface> {
        self.validate()?;
        let w = i64::from(old.width);
        let h = i64::from(old.height);
        match self {
            Self::CentreZoom { level } => {
                let limit = (duration / 2).max(1);
                let (source, c) = if count >= limit {
                    (new, (duration - count).min(limit))
                } else {
                    (old, count)
                };
                let scale = 2i64 << (level + 1);
                let aw = w / scale;
                let ah = h / scale;
                let x1 = (w / 2 - aw / 2) * c / limit;
                let y1 = (h / 2 - ah / 2) * c / limit;
                let x2 = w + (w / 2 + aw / 2 - w) * c / limit;
                let y2 = h + (h / 2 + ah / 2 - h) * c / limit;
                let mut output = old.clone();
                if x2 <= x1 || y2 <= y1 {
                    return Ok(output);
                }
                let dx = (x2 - x1) * 65536 / w;
                let dy = (y2 - y1) * 65536 / h;
                for y in 0..h {
                    check()?;
                    for x in 0..w {
                        let sx = x1 + ((x * dx) >> 16);
                        let sy = y1 + ((y * dy) >> 16);
                        let at = ((y * w + x) * 4) as usize;
                        let from = ((sy * w + sx) * 4) as usize;
                        output.rgba[at..at + 4].copy_from_slice(&source.rgba[from..from + 4]);
                    }
                }
                Ok(output)
            }
            Self::ExpandingPanel { reverse, blend } => {
                let (old, new, c) = if reverse {
                    (new, old, duration - count)
                } else {
                    (old, new, count)
                };
                if c <= 0 {
                    return Ok(old.clone());
                }
                if c >= duration {
                    return Ok(new.clone());
                }
                let aw = w * c / duration;
                let ah = h * c / duration;
                let x1 = (w / 2 - aw / 2).max(0);
                let y1 = (h / 2 - ah / 2).max(0);
                let x2 = (w / 2 + aw / 2).min(w);
                let y2 = (h / 2 + ah / 2).min(h);
                let dw = x2 - x1;
                let dh = y2 - y1;
                let mut output = old.clone();
                if dw <= 0 || dh <= 0 {
                    return Ok(output);
                }
                let quantize = |s: i64, d: i64| {
                    let step = s as f64 / d as f64;
                    let whole = step as i64;
                    whole * 65536 + ((step - whole as f64) * 65536.) as i64
                };
                let dx = quantize(w, dw);
                let dy = quantize(h, dh);
                let weight = c * 100 / duration * 256 / 100;
                for y in y1..y2 {
                    check()?;
                    for x in x1..x2 {
                        let sx = ((x - x1) * dx) >> 16;
                        let sy = ((y - y1) * dy) >> 16;
                        let at = ((y * w + x) * 4) as usize;
                        let from = ((sy * w + sx) * 4) as usize;
                        if !blend {
                            output.rgba[at..at + 4].copy_from_slice(&new.rgba[from..from + 4]);
                        } else if new.rgba[from + 3] > 0 {
                            let a = i64::from(new.rgba[from + 3]) * weight / 256;
                            for channel in 0..3 {
                                output.rgba[at + channel] =
                                    ((i64::from(new.rgba[from + channel]) * a >> 8)
                                        + (i64::from(old.rgba[at + channel]) * (255 - a) >> 8))
                                        as u8;
                            }
                            output.rgba[at + 3] = 255;
                        }
                    }
                }
                Ok(output)
            }
        }
    }
}
