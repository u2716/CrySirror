// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Legacy FLASH17, VT56791C: Set4ABCB0, Count4ABC50, Print4ABDC0.
//! Native cadence/parameter policy lowers to a shared solid image, not pixels.
use crysirror_core::image_processing::ImageInput;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FlashState {
    pub frames: i32,
    pub on: i32,
    pub off: i32,
    pub color: [i32; 3],
    pub count: i32,
    pub ended: bool,
}
impl FlashState {
    pub fn parse(values: &[i32]) -> Self {
        let on = values.get(1).copied().unwrap_or(1);
        let red = values.get(3).copied().unwrap_or(255);
        let green = values.get(4).copied().unwrap_or(red);
        Self {
            frames: values.first().copied().unwrap_or(20),
            on,
            off: values.get(2).copied().unwrap_or(on),
            color: [red, green, values.get(5).copied().unwrap_or(green)],
            count: 0,
            ended: false,
        }
    }
    pub fn parameters(&self) -> [i32; 6] {
        [
            self.frames,
            self.on,
            self.off,
            self.color[0],
            self.color[1],
            self.color[2],
        ]
    }
    pub fn count_up(&mut self, steps: u32) {
        if self.ended || self.count >= self.frames {
            return;
        }
        self.count = self.count.wrapping_add(steps as i32).min(self.frames);
        if self.count == self.frames {
            self.ended = true;
        }
    }
    pub fn terminal_color(&self) -> [u8; 3] {
        self.color.map(|value| value as u8)
    }
    pub fn image_input(&self, base: ImageInput, canvas: [u32; 2]) -> ImageInput {
        if !self.ended && self.count % self.on.wrapping_add(self.off).max(1) >= self.on {
            return base;
        }
        let color = if self.ended {
            //CountIsMax changes command to54; Fill4ABB10 truncates each channel
            //before calling the same FillScreen helper. FLASH itself passes raw ints.
            self.terminal_color()
        } else {
            let [red, mut green, mut blue] = self.color;
            if green == -1 {
                green = red;
            }
            if blue == -1 {
                blue = green;
            }
            //CFillScreen performs signed x86 shifts/OR, not per-channel saturation.
            let rgb =
                (red as u32).wrapping_shl(16) | (green as u32).wrapping_shl(8) | (blue as u32);
            [(rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8]
        };
        ImageInput::Solid {
            size: canvas,
            color: [color[0], color[1], color[2], 255],
        }
    }
}
#[derive(Clone)]
pub struct FlashFrame {
    pub state: FlashState,
    pub canvas: [u32; 2],
}
