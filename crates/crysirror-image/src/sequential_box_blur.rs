//! Box filtering with explicit sequential feedback into the destination.
//! Unlike an immutable convolution, overlapping source/destination samples
//! observe earlier writes in the specified scan order. Publication stays atomic.
use crate::{ImageSurface, effects::MAX_EFFECT_PIXELS};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SequentialBoxBlur {
    pub source: [u32; 4],
    pub destination: [u32; 2],
    pub vertical: bool,
    pub radius: u16,
    pub write_mask: [bool; 4],
}
impl SequentialBoxBlur {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.source[2..].iter().all(|v| *v > 0),
            "sequential blur empty source"
        );
        ensure!(
            self.source
                .iter()
                .chain(&self.destination)
                .all(|v| *v <= crate::MAX_IMAGE_DIMENSION),
            "sequential blur coordinate budget"
        );
        let axis = usize::from(self.vertical);
        ensure!(
            u32::from(self.radius) * 2 + 1 <= self.source[axis + 2],
            "sequential blur kernel exceeds source axis"
        );
        Ok(())
    }
    pub fn apply(
        &self,
        input: &ImageSurface,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<ImageSurface> {
        self.validate()?;
        input.validate()?;
        ensure!(
            u64::from(input.width) * u64::from(input.height) <= MAX_EFFECT_PIXELS,
            "sequential blur pixel budget"
        );
        let size = [input.width, input.height];
        for axis in 0..2 {
            ensure!(
                u64::from(self.source[axis]) + u64::from(self.source[axis + 2])
                    <= u64::from(size[axis])
                    && u64::from(self.destination[axis]) + u64::from(self.source[axis + 2])
                        <= u64::from(size[axis]),
                "sequential blur surface bounds"
            );
        }
        check()?;
        let mut output = input.clone();
        let axis = usize::from(self.vertical);
        let length = self.source[axis + 2];
        let lines = self.source[3 - axis];
        let radius = u32::from(self.radius);
        let divisor = radius * 2 + 1;
        let index = |origin: [u32; 2], line: u32, position: u32, channel: usize| -> usize {
            let mut point = origin;
            point[axis] += position;
            point[1 - axis] += line;
            ((point[1] * input.width + point[0]) * 4) as usize + channel
        };
        let source = [self.source[0], self.source[1]];
        for line in 0..lines {
            check()?;
            let mut sums = [0u32; 4];
            for channel in 0..4 {
                if !self.write_mask[channel] {
                    continue;
                }
                sums[channel] = radius * u32::from(output.rgba[index(source, line, 0, channel)]);
                for position in 0..=radius {
                    sums[channel] += u32::from(output.rgba[index(source, line, position, channel)]);
                }
            }
            for position in 0..length {
                if position != 0 {
                    let old = if position <= radius {
                        0
                    } else {
                        position - radius - 1
                    };
                    let new = (position + radius).min(length - 1);
                    for channel in 0..4 {
                        if !self.write_mask[channel] {
                            continue;
                        }
                        // Read both operands before the new destination write.
                        let add = u32::from(output.rgba[index(source, line, new, channel)]);
                        let subtract = u32::from(output.rgba[index(source, line, old, channel)]);
                        sums[channel] = sums[channel]
                            .checked_add(add)
                            .and_then(|v| v.checked_sub(subtract))
                            .ok_or_else(|| {
                                anyhow::anyhow!("sequential blur feedback sum outside safe range")
                            })?;
                    }
                }
                for channel in 0..4 {
                    if !self.write_mask[channel] {
                        continue;
                    }
                    ensure!(
                        sums[channel] <= 255 * divisor,
                        "sequential blur feedback lookup outside byte range"
                    );
                    let value = sums[channel] / divisor;
                    output.rgba[index(self.destination, line, position, channel)] = value as u8;
                }
            }
        }
        check()?;
        Ok(output)
    }
}
