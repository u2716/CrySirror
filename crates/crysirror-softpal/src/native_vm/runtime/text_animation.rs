//! New-build text threshold kernels (0x409450/0x4096A0/0x409A50).
//! Glyph rectangles come from the portable font layout. Native font metrics,
//! ruby geometry and append-time reuse require their own layout validation.
use super::AdvTextLineLayout;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct TextAnimationMode {
    pub kind: i32,
    pub fade: bool,
    pub space_skip: bool,
}

#[derive(Clone, Debug)]
pub(super) struct TextThresholdMap {
    thresholds: Vec<u16>,
    pub extent: u32,
    fade: bool,
}

impl TextThresholdMap {
    pub fn build(
        mode: TextAnimationMode,
        width: u32,
        height: u32,
        lines: &[AdvTextLineLayout],
        effect: u16,
    ) -> Self {
        let mut thresholds = vec![0; width as usize * height as usize];
        let continuous = mode.kind == 0 && mode.fade && !mode.space_skip;
        let mut extent = 0u32;
        if continuous {
            // The continuous kernel advances by the complete surface width
            // per line block, including horizontal blank space.
            for (index, line) in lines.iter().enumerate() {
                let base = (index as u32).wrapping_mul(width);
                for y in line.y..line.y.saturating_add(line.height).min(height) {
                    for x in 0..width {
                        thresholds[(y * width + x) as usize] = base.wrapping_add(x) as u16;
                    }
                }
                if let Some(&end) = line.char_x.last() {
                    extent = base.wrapping_add(end);
                }
            }
        } else {
            let (left_margin, bottom_margin, right_margin) = match effect {
                0 => (1, 1, 4),
                1 | 2 => (2, 2, 4),
                3 => (3, 4, 4),
                _ => (0, 0, 0),
            };
            let mut threshold = 64u32;
            for line in lines {
                for index in 0..line.char_count {
                    let x0 = line.char_x[index];
                    let x1 = line.char_x[index + 1];
                    let left = if index == 0 {
                        x0.saturating_sub(left_margin)
                    } else {
                        x0
                    };
                    let right = if index + 1 == line.char_count {
                        x1.saturating_add(right_margin)
                    } else {
                        x1
                    };
                    for y in line.y
                        ..line
                            .y
                            .saturating_add(line.height)
                            .saturating_add(bottom_margin)
                            .min(height)
                    {
                        for x in left..right.min(width) {
                            let offset = if mode.kind == 0 { x - left } else { 0 };
                            thresholds[(y * width + x) as usize] =
                                threshold.wrapping_add(offset) as u16;
                        }
                    }
                    threshold = threshold.wrapping_add(x1.wrapping_sub(x0));
                }
            }
            extent = threshold.wrapping_sub(64);
        }
        Self {
            thresholds,
            extent: extent
                .saturating_add(64)
                .saturating_add(if mode.space_skip { 64 } else { 0 }),
            fade: mode.fade,
        }
    }

    pub fn alpha(&self, pixel: usize, original: u8, progress: u32) -> u8 {
        let Some(&threshold) = self.thresholds.get(pixel) else {
            return 0;
        };
        threshold_alpha(original, threshold, progress, self.fade)
    }
}

fn threshold_alpha(original: u8, threshold: u16, progress: u32, fade: bool) -> u8 {
    let threshold = u32::from(threshold);
    if progress <= threshold {
        return 0;
    }
    let ramp = progress.saturating_sub(64);
    if !fade || ramp > threshold {
        return original;
    }
    ((u32::from(original) * (ramp + 64 - threshold)) >> 6) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_alpha_ramp_keeps_its_strict_threshold_and_six_bit_shift() {
        assert_eq!(threshold_alpha(255, 64, 64, true), 0);
        assert_eq!(threshold_alpha(255, 64, 65, true), 3);
        assert_eq!(threshold_alpha(255, 64, 96, true), 127);
        assert_eq!(threshold_alpha(255, 64, 128, true), 255);
        assert_eq!(threshold_alpha(255, 64, 65, false), 255);
        assert_eq!(threshold_alpha(123, 0, 1, true), 123);
    }

    #[test]
    fn modes_distinguish_whole_glyph_wipe_and_surface_space() {
        let lines = [
            AdvTextLineLayout {
                y: 0,
                height: 1,
                char_start: 0,
                char_count: 2,
                char_x: vec![0, 4, 6],
            },
            AdvTextLineLayout {
                y: 1,
                height: 1,
                char_start: 2,
                char_count: 1,
                char_x: vec![0, 2],
            },
        ];
        let glyph = TextThresholdMap::build(
            TextAnimationMode {
                kind: 1,
                fade: false,
                space_skip: true,
            },
            8,
            2,
            &lines,
            99,
        );
        assert_eq!(glyph.alpha(0, 255, 65), 255);
        assert_eq!(glyph.alpha(3, 255, 65), 255);
        assert_eq!(glyph.alpha(4, 255, 65), 0);
        let wipe = TextThresholdMap::build(
            TextAnimationMode {
                kind: 0,
                fade: false,
                space_skip: true,
            },
            8,
            2,
            &lines,
            99,
        );
        assert_eq!(wipe.alpha(0, 255, 65), 255);
        assert_eq!(wipe.alpha(3, 255, 65), 0);
        let surface = TextThresholdMap::build(
            TextAnimationMode {
                kind: 0,
                fade: true,
                space_skip: false,
            },
            8,
            2,
            &lines,
            99,
        );
        assert_eq!(surface.thresholds[8], 8);
        assert_eq!(wipe.thresholds[8], 70);
        assert_eq!(surface.extent, 74);
        assert_eq!(wipe.extent, 136);
    }
}
