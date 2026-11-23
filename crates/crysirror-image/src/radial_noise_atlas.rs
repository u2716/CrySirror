// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Worker-owned radial byte-brush atlas. Draw order is explicit and the random
//! stream is returned transactionally; owners never inspect the image pixels.
use crate::{effects::MAX_EFFECT_PIXELS, rgba_len, temporal_pattern::PatternRng};
use anyhow::{Result, ensure};
use crysirror_core::image_processing::{PackedImageSpec, PackedPixelLayout};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum RadialNoiseMath {
    /// max(minimum, (radius_bias-floor(radius))/divisor + base + random%range).
    /// Signed division truncates toward zero, including negative radial terms.
    Add {
        radius_bias: i32,
        divisor: u32,
        base: i32,
        range: u32,
        minimum: u8,
    },
    /// (base+random%range)*max(radius_bias-floor(radius),0)/divisor.
    Multiply {
        radius_bias: i32,
        divisor: u32,
        base: u32,
        range: u32,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RadialNoiseTile {
    pub rectangle: [u32; 4],
    /// Local coordinates of the centre; integer radius is floor(sqrt(x²+y²)).
    pub center: [i32; 2],
    pub math: RadialNoiseMath,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RadialNoiseAtlas {
    pub size: [u32; 2],
    /// Tiles execute in vector order, then row-major inside each rectangle.
    /// Even a clamped zero sample consumes one draw.
    pub tiles: Vec<RadialNoiseTile>,
}
impl RadialNoiseAtlas {
    pub fn validate(&self) -> Result<()> {
        rgba_len(self.size[0], self.size[1])?;
        ensure!(self.tiles.len() <= 1024, "radial noise atlas tile count");
        let mut work = 0u64;
        for tile in &self.tiles {
            let [x, y, w, h] = tile.rectangle;
            ensure!(
                w > 0
                    && h > 0
                    && u64::from(x) + u64::from(w) <= u64::from(self.size[0])
                    && u64::from(y) + u64::from(h) <= u64::from(self.size[1])
                    && tile.center.iter().all(|n| n.unsigned_abs() <= 1_000_000),
                "radial noise atlas tile extent"
            );
            let (bias, divisor, base, range) = match tile.math {
                RadialNoiseMath::Add {
                    radius_bias,
                    divisor,
                    base,
                    range,
                    ..
                } => (radius_bias, divisor, i64::from(base), range),
                RadialNoiseMath::Multiply {
                    radius_bias,
                    divisor,
                    base,
                    range,
                } => (radius_bias, divisor, i64::from(base), range),
            };
            ensure!(
                bias.unsigned_abs() <= 1_000_000
                    && divisor > 0
                    && divisor <= 1_000_000
                    && base.unsigned_abs() <= 1_000_000
                    && (1..=32768).contains(&range),
                "radial noise atlas arithmetic domain"
            );
            work += u64::from(w) * u64::from(h);
        }
        ensure!(
            work <= MAX_EFFECT_PIXELS,
            "radial noise atlas sample budget"
        );
        Ok(())
    }
    pub fn prepare(
        &self,
        random: &mut PatternRng,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<PackedImageSpec> {
        self.validate()?;
        random.validate()?;
        check()?;
        let mut candidate = random.clone();
        let mut bytes = vec![0; rgba_len(self.size[0], self.size[1])?];
        for tile in &self.tiles {
            let [x, y, w, h] = tile.rectangle;
            for row in 0..h {
                check()?;
                for col in 0..w {
                    if col % 4096 == 0 {
                        check()?;
                    }
                    let dx = i64::from(col) - i64::from(tile.center[0]);
                    let dy = i64::from(row) - i64::from(tile.center[1]);
                    let radius = ((dx * dx + dy * dy) as f64).sqrt() as i64;
                    let draw = candidate.next_draw()?;
                    let value = match tile.math {
                        RadialNoiseMath::Add {
                            radius_bias,
                            divisor,
                            base,
                            range,
                            minimum,
                        } => ((i64::from(radius_bias) - radius) / i64::from(divisor)
                            + i64::from(base)
                            + i64::from(draw % range))
                        .max(i64::from(minimum)),
                        RadialNoiseMath::Multiply {
                            radius_bias,
                            divisor,
                            base,
                            range,
                        } => {
                            i64::from(base + draw % range)
                                * (i64::from(radius_bias) - radius).max(0)
                                / i64::from(divisor)
                        }
                    };
                    // Native brush bytes are packed, not a floating-point
                    // exposure. Explicitly preserve integer-byte truncation.
                    let at =
                        (((y + row) as usize * self.size[0] as usize) + (x + col) as usize) * 4;
                    bytes[at + 3] = value as u8;
                }
            }
        }
        check()?;
        let result = PackedImageSpec {
            size: self.size,
            layout: PackedPixelLayout::Bgra8,
            bytes: bytes.into(),
        };
        *random = candidate;
        Ok(result)
    }
}
