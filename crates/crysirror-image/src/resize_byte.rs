//! Axis-aligned stored-byte resizing. The scan converter rounds horizontal
//! increments to16.16, while vertical edges accumulate single-precision UVs.
//! Linear filtering subtracts half a texel and truncates each7-bit mixture.
use crate::{ImageSurface, effects::MAX_EFFECT_PIXELS};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ByteResizeSampling {
    Point,
    Linear,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResizeByte128 {
    /// An absent dimension retains the current input dimension. Targets below
    /// two retain the entire input, before allocating a replacement surface.
    pub size: [Option<u32>; 2],
    pub sampling: ByteResizeSampling,
}
impl ResizeByte128 {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.size
                .iter()
                .flatten()
                .all(|v| *v <= crate::MAX_IMAGE_DIMENSION),
            "byte resize dimension limit"
        );
        Ok(())
    }
    pub fn output_size(&self, input: [u32; 2]) -> Result<[u32; 2]> {
        self.validate()?;
        let target = std::array::from_fn(|i| self.size[i].unwrap_or(input[i]));
        let size = if target.iter().any(|v| *v < 2) {
            input
        } else {
            target
        };
        crate::rgba_len(size[0], size[1])?;
        ensure!(
            u64::from(size[0]) * u64::from(size[1]) <= MAX_EFFECT_PIXELS,
            "byte resize pixel budget"
        );
        Ok(size)
    }
    pub fn apply(
        &self,
        input: &ImageSurface,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<ImageSurface> {
        input.validate()?;
        ensure!(
            u64::from(input.width) * u64::from(input.height) <= MAX_EFFECT_PIXELS,
            "byte resize input budget"
        );
        let size = self.output_size([input.width, input.height])?;
        check()?;
        if size == [input.width, input.height] {
            return Ok(input.clone());
        }
        let ratio = [
            input.width as f32 / size[0] as f32,
            input.height as f32 / size[1] as f32,
        ];
        let step = (f64::from(ratio[0]) * 65536. + 0.5).floor() as i64;
        let tap = |x: i64, y: i64| -> [u8; 4] {
            let x = x.clamp(0, i64::from(input.width) - 1);
            let y = y.clamp(0, i64::from(input.height) - 1);
            let at = ((y * i64::from(input.width) + x) * 4) as usize;
            input.rgba[at..at + 4].try_into().unwrap()
        };
        let mix = |a: [u8; 4], b: [u8; 4], w: i32| {
            std::array::from_fn(|c| {
                (i32::from(a[c]) + (((i32::from(b[c]) - i32::from(a[c])) * w) >> 7)) as u8
            })
        };
        let mut output = ImageSurface {
            width: size[0],
            height: size[1],
            rgba: vec![0; crate::rgba_len(size[0], size[1])?],
        };
        let mut v = 0f32;
        for y in 0..size[1] {
            check()?;
            let qy = (f64::from(v) * 65536. + 0.5).floor() as i64;
            for x in 0..size[0] {
                let qx = i64::from(x) * step;
                let pixel = match self.sampling {
                    ByteResizeSampling::Point => tap(qx >> 16, qy >> 16),
                    ByteResizeSampling::Linear => {
                        let (u, v) = (qx - 32768, qy - 32768);
                        let (ix, iy) = (u >> 16, v >> 16);
                        let wx = (((u & 65535) + 1) >> 9) as i32;
                        let wy = (((v & 65535) + 1) >> 9) as i32;
                        mix(
                            mix(tap(ix, iy), tap(ix + 1, iy), wx),
                            mix(tap(ix, iy + 1), tap(ix + 1, iy + 1), wx),
                            wy,
                        )
                    }
                };
                let at = ((y * size[0] + x) * 4) as usize;
                output.rgba[at..at + 4].copy_from_slice(&pixel);
            }
            v = (f64::from(v) + f64::from(ratio[1])) as f32;
        }
        check()?;
        Ok(output)
    }
}
