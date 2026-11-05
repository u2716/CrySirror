//! Ordered coverage-only atlas stamping over immutable stored RGB.
use crate::ImageSurface;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlphaStamp {
    pub source: [i32; 4],
    pub position: [i32; 2],
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlphaStamps {
    pub stamps: Vec<AlphaStamp>,
    #[serde(default)]
    pub clear: Option<u8>,
    #[serde(default)]
    pub preserve_opaque: bool,
}
impl AlphaStamps {
    pub fn validate(&self) -> Result<()> {
        ensure!(self.stamps.len() <= 4096, "alpha stamp count");
        let mut work = 0u64;
        for stamp in &self.stamps {
            ensure!(
                stamp
                    .source
                    .iter()
                    .chain(&stamp.position)
                    .all(|v| v.unsigned_abs() <= 1_000_000)
                    && stamp.source[2] > 0
                    && stamp.source[3] > 0,
                "alpha stamp rectangle"
            );
            work = work
                .checked_add(stamp.source[2] as u64 * stamp.source[3] as u64)
                .ok_or_else(|| anyhow::anyhow!("alpha stamp work overflow"))?;
        }
        ensure!(work <= 64 * 1024 * 1024, "alpha stamp pixel budget");
        Ok(())
    }
    pub fn apply(
        &self,
        destination: &ImageSurface,
        source: &ImageSurface,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<ImageSurface> {
        self.validate()?;
        destination.validate()?;
        source.validate()?;
        ensure!(
            u64::from(destination.width) * u64::from(destination.height)
                <= crate::effects::MAX_EFFECT_PIXELS,
            "alpha stamp output budget"
        );
        let mut out = destination.clone();
        if let Some(alpha) = self.clear {
            for (index, pixel) in out.rgba.chunks_exact_mut(4).enumerate() {
                if index % 16384 == 0 {
                    check()?;
                }
                pixel[3] = alpha;
            }
        }
        for stamp in &self.stamps {
            check()?;
            let [sx, sy, w, h] = stamp.source;
            let [dx, dy] = stamp.position;
            let left = 0.max(-sx).max(-dx);
            let top = 0.max(-sy).max(-dy);
            let right = w.min(source.width as i32 - sx).min(out.width as i32 - dx);
            let bottom = h.min(source.height as i32 - sy).min(out.height as i32 - dy);
            for row in top..bottom {
                check()?;
                for col in left..right {
                    let from =
                        (((sy + row) as usize * source.width as usize + (sx + col) as usize) * 4)
                            + 3;
                    let to =
                        (((dy + row) as usize * out.width as usize + (dx + col) as usize) * 4) + 3;
                    if !self.preserve_opaque || out.rgba[to] != 255 {
                        out.rgba[to] = source.rgba[from];
                    }
                }
            }
        }
        Ok(out)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn coverage_overwrites_partial_but_preserves_opaque_without_changing_rgb() {
        let dst = ImageSurface {
            width: 2,
            height: 1,
            rgba: vec![1, 2, 3, 9, 4, 5, 6, 9],
        };
        let src = ImageSurface {
            width: 3,
            height: 1,
            rgba: vec![0, 0, 0, 255, 0, 0, 0, 80, 0, 0, 0, 40],
        };
        let op = AlphaStamps {
            clear: Some(0),
            preserve_opaque: true,
            stamps: vec![
                AlphaStamp {
                    source: [0, 0, 2, 1],
                    position: [0, 0],
                },
                AlphaStamp {
                    source: [2, 0, 1, 1],
                    position: [0, 0],
                },
                AlphaStamp {
                    source: [2, 0, 1, 1],
                    position: [1, 0],
                },
            ],
        };
        assert_eq!(
            op.apply(&dst, &src, &|| Ok(())).unwrap().rgba,
            vec![1, 2, 3, 255, 4, 5, 6, 40]
        );
    }
    #[test]
    fn negative_positions_clip_both_resources_and_preserve_hidden_rgb() {
        let dst = ImageSurface {
            width: 1,
            height: 1,
            rgba: vec![7, 8, 9, 0],
        };
        let src = ImageSurface {
            width: 2,
            height: 1,
            rgba: vec![1, 1, 1, 10, 2, 2, 2, 30],
        };
        let op = AlphaStamps {
            clear: None,
            preserve_opaque: false,
            stamps: vec![AlphaStamp {
                source: [0, 0, 2, 2],
                position: [-1, 0],
            }],
        };
        assert_eq!(
            op.apply(&dst, &src, &|| Ok(())).unwrap().rgba,
            vec![7, 8, 9, 30]
        );
    }
}
