// SPDX-License-Identifier: MPL-2.0
//! Apply a bottom-aligned colour strip in stored byte space. Only the first
//! texel of each row supplies colour. Destination coverage stays intact.
use crate::ImageSurface;
use anyhow::Result;

#[derive(Clone, Copy)]
pub enum StripBlend {
    Overlay,
    Screen,
}

pub fn apply(image: &mut ImageSurface, strip: &ImageSurface, blend: StripBlend) -> Result<()> {
    image.validate()?;
    strip.validate()?;
    for (y, row) in image
        .rgba
        .chunks_exact_mut(image.width as usize * 4)
        .enumerate()
    {
        let index = (y as i64 + i64::from(strip.height) - i64::from(image.height))
            .clamp(0, i64::from(strip.height) - 1) as usize
            * strip.width as usize
            * 4;
        let color = &strip.rgba[index..index + 3];
        for p in row.chunks_exact_mut(4).filter(|p| p[3] != 0) {
            for (d, s) in p[..3].iter_mut().zip(color) {
                let a = u32::from(*d);
                let b = u32::from(*s);
                *d = match blend {
                    StripBlend::Screen => 254 - ((255 - a) * (255 - b) >> 8),
                    StripBlend::Overlay if a < 128 => (2 * ((a * b + 255) >> 8)).min(255),
                    StripBlend::Overlay => {
                        255 - (2 * (((255 - a) * (255 - b) + 255) >> 8)).min(255)
                    }
                } as u8;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn extends_first_row_and_preserves_destination_alpha() {
        let mut image = ImageSurface {
            width: 1,
            height: 3,
            rgba: vec![0, 0, 0, 1, 3, 4, 5, 0, 255, 255, 255, 200],
        };
        let strip = ImageSurface {
            width: 1,
            height: 2,
            rgba: vec![255, 0, 0, 0, 0, 0, 0, 0],
        };
        apply(&mut image, &strip, StripBlend::Screen).unwrap();
        assert_eq!(image.rgba, [254, 0, 0, 1, 3, 4, 5, 0, 254, 254, 254, 200]);
    }
}
