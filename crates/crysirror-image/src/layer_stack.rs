// SPDX-License-Identifier: MPL-2.0
//! A sixteen-layer CPU picture stack baked into one image: layer 0 is the opaque base,
//! layers 1..=15 are placed over it, copied or alpha-blended. Each layer may carry a
//! filtered copy (invert, mosaic, blurs, radial and spin blurs, tinted grey) that
//! replaces its pixels when baking. Tint and multiply change the layer pixels in place.
//! Integer arithmetic follows the CMVS CTexture compositor (0x14003cbf0..0x14003f270).

use crate::ImageSurface;
use std::sync::Arc;

pub const LAYERS: usize = 16;

#[derive(Clone, Debug)]
struct Layer {
    pixels: ImageSurface,
    filtered: Option<ImageSurface>,
    /// Alpha-blended when baked; otherwise only colour is copied.
    alpha: bool,
    position: [i32; 2],
}

#[derive(Clone, Debug, Default)]
pub struct LayerStack {
    layers: [Option<Arc<Layer>>; LAYERS],
}

/// Channel of a `0x00RRGGBB` colour for RGBA channel `channel`.
fn component(color: u32, channel: usize) -> u32 {
    (color >> (16 - 8 * channel as u32)) & 0xff
}

/// Distance and angle in whole degrees, as 0x14001fa30 measures them.
fn polar(centre: [i32; 2], point: [i32; 2]) -> (i32, i32) {
    let dx = point[0] - centre[0];
    let dy = point[1] - centre[1];
    let theta = f64::from(dy).atan2(f64::from(dx));
    let mut angle = (theta / std::f64::consts::PI * -180.0) as i32;
    if angle < 0 {
        angle += 360;
    }
    angle = 360 - angle;
    if angle == 360 {
        angle = 0;
    }
    let distance = if dx != 0 { (f64::from(dx) / theta.cos()) as i32 } else { dy.abs() };
    (angle, distance)
}

impl LayerStack {
    pub fn is_empty(&self) -> bool {
        self.layers.iter().all(Option::is_none)
    }

    /// Places `pixels` as layer `index`; layer 0 is the base the others are placed on.
    pub fn place(&mut self, index: usize, pixels: ImageSurface, alpha: bool, position: [i32; 2]) {
        if let Some(slot) = self.layers.get_mut(index) {
            *slot = Some(Arc::new(Layer { pixels, filtered: None, alpha, position }));
        }
    }

    /// Source pixels after the most recent layer filter, before baking.
    pub fn surface(&self, index: usize) -> Option<&ImageSurface> {
        let layer = self.layers.get(index)?.as_ref()?;
        Some(layer.filtered.as_ref().unwrap_or(&layer.pixels))
    }

    pub fn has_overlays(&self) -> bool {
        self.layers[1..].iter().any(Option::is_some)
    }

    pub fn reposition(&mut self, index: usize, position: [i32; 2]) {
        if let Some(Some(layer)) = self.layers.get_mut(index) {
            Arc::make_mut(layer).position = position;
        }
    }

    /// Copies both original and filtered pixels and installs the new offset.
    pub fn copy_layer(&mut self, index: usize, source: &Self, from: usize, position: [i32; 2]) {
        if let Some(slot) = self.layers.get_mut(index) {
            *slot = source.layers.get(from).cloned().flatten();
            self.reposition(index, position);
        }
    }

    /// Fill the underlying layer, preserving its independent filtered copy.
    pub fn fill_rgba(&mut self, index: usize, color: [u8; 4]) {
        if let Some(Some(layer)) = self.layers.get_mut(index) {
            for pixel in Arc::make_mut(layer).pixels.rgba.chunks_exact_mut(4) {
                pixel.copy_from_slice(&color);
            }
        }
    }

    /// An opaque black `width`×`height` layer (0x14003e940).
    pub fn solid(&mut self, index: usize, [width, height]: [u32; 2], position: [i32; 2]) {
        let rgba = [0, 0, 0, 255].repeat((width * height) as usize);
        self.place(index, ImageSurface { width, height, rgba }, true, position);
    }

    pub fn release(&mut self, index: usize) {
        if let Some(slot) = self.layers.get_mut(index) {
            *slot = None;
        }
    }

    /// Layers `index`, or every layer for 16 and above, until the first empty one.
    fn targets(&mut self, index: usize) -> impl Iterator<Item = Option<&mut Layer>> {
        let range = if index >= LAYERS { 0..LAYERS } else { index..index + 1 };
        self.layers[range].iter_mut().map(|slot| slot.as_mut().map(Arc::make_mut))
    }

    /// Grey by the channel mean, then scaled by `color` (0x14003f0e0). False when it
    /// stops at an empty layer.
    pub fn tint(&mut self, index: usize, color: u32) -> bool {
        for layer in self.targets(index) {
            let Some(layer) = layer else { return false };
            for pixel in layer.pixels.rgba.chunks_exact_mut(4) {
                let grey = (u32::from(pixel[0]) + u32::from(pixel[1]) + u32::from(pixel[2])) / 3;
                for (channel, value) in pixel[..3].iter_mut().enumerate() {
                    *value = (grey * component(color, channel) / 255) as u8;
                }
            }
        }
        true
    }

    /// Channels scaled by `color / 128`, saturating (0x14003f270).
    pub fn multiply(&mut self, index: usize, color: u32) -> bool {
        for layer in self.targets(index) {
            let Some(layer) = layer else { return false };
            for pixel in layer.pixels.rgba.chunks_exact_mut(4) {
                for (channel, value) in pixel[..3].iter_mut().enumerate() {
                    *value = ((u32::from(*value) * component(color, channel)) >> 7).min(255) as u8;
                }
            }
        }
        true
    }

    /// Filter `mode` with `parameter` into the layer's filtered copy (0x14003e320).
    /// Mode 0 drops the copy; unknown modes leave the layer as it is. False for an empty layer.
    pub fn filter(&mut self, index: usize, mode: u32, parameter: u32) -> bool {
        let Some(layer) = self.layers.get_mut(index).and_then(Option::as_mut).map(Arc::make_mut) else {
            return false;
        };
        let source = &layer.pixels;
        let filtered = match mode {
            0 => None,
            1 => Some(invert(source)),
            3 if parameter > 0 => Some(mosaic(source, parameter)),
            4 => Some(neighbours(source, parameter)),
            9 => Some(grey_tint(source, parameter)),
            10 => Some(box_blur(source, parameter & !1)),
            11 | 12 => Some(radial(source, parameter & !1, mode == 12, false)),
            13 | 14 => Some(radial(source, parameter & !1, mode == 14, true)),
            15 | 16 => Some(spin(source, parameter & !1, mode == 16)),
            _ => return true,
        };
        if mode == 0 || filtered.is_some() {
            layer.filtered = filtered;
        }
        true
    }

    /// 0x14003ec30: the base layer made opaque with every other layer placed on it.
    /// None without a base. A base alone is returned as it is.
    pub fn bake(&self) -> Option<ImageSurface> {
        let base = self.layers[0].as_ref()?;
        let shown = |layer: &Layer| layer.filtered.as_ref().unwrap_or(&layer.pixels).clone();
        if self.layers[1..].iter().all(Option::is_none) {
            return Some(shown(base));
        }
        let mut out = shown(base);
        for pixel in out.rgba.chunks_exact_mut(4) {
            pixel[3] = 255;
        }
        let (width, height) = (out.width as i32, out.height as i32);
        for layer in self.layers[1..].iter().flatten() {
            let source = layer.filtered.as_ref().unwrap_or(&layer.pixels);
            let [x, y] = layer.position;
            let (w, h) = (source.width as i32, source.height as i32);
            if x.saturating_add(w) <= 0 || x >= width || y.saturating_add(h) <= 0 || y >= height {
                continue;
            }
            let (skip_x, skip_y) = ((-x).max(0), (-y).max(0));
            let columns = (w - skip_x).min(width - x.max(0));
            let rows = (h - skip_y).min(height - y.max(0));
            for row in 0..rows {
                for column in 0..columns {
                    let from = (((skip_y + row) * w + skip_x + column) * 4) as usize;
                    let to = (((y.max(0) + row) * width + x.max(0) + column) * 4) as usize;
                    let texel = &source.rgba[from..from + 4];
                    let target = &mut out.rgba[to..to + 4];
                    if !layer.alpha {
                        target[..3].copy_from_slice(&texel[..3]);
                        continue;
                    }
                    match texel[3] {
                        0 => {}
                        255 => target.copy_from_slice(texel),
                        alpha => {
                            let alpha = u32::from(alpha);
                            for channel in 0..3 {
                                let mixed = (256 - alpha) * u32::from(target[channel]) + alpha * u32::from(texel[channel]);
                                target[channel] = ((mixed & 0xffff) >> 8) as u8;
                            }
                        }
                    }
                }
            }
        }
        Some(out)
    }
}

fn blank(source: &ImageSurface) -> ImageSurface {
    ImageSurface { width: source.width, height: source.height, rgba: vec![0; source.rgba.len()] }
}

fn at(surface: &ImageSurface, x: i32, y: i32) -> &[u8] {
    let index = ((y * surface.width as i32 + x) * 4) as usize;
    &surface.rgba[index..index + 4]
}

/// Mode 1 (0x14003e870).
fn invert(source: &ImageSurface) -> ImageSurface {
    let mut out = source.clone();
    for pixel in out.rgba.chunks_exact_mut(4) {
        for value in &mut pixel[..3] {
            *value = !*value;
        }
    }
    out
}

/// Mode 3 (0x14003e670): each block takes its lower right pixel.
fn mosaic(source: &ImageSurface, block: u32) -> ImageSurface {
    let mut out = blank(source);
    let (width, height) = (source.width, source.height);
    for top in (0..height).step_by(block as usize) {
        for left in (0..width).step_by(block as usize) {
            let w = block.min(width - left);
            let h = block.min(height - top);
            let sample: [u8; 4] = at(source, (left + w - 1) as i32, (top + h - 1) as i32).try_into().expect("texel");
            for y in top..top + h {
                for x in left..left + w {
                    let index = ((y * width + x) * 4) as usize;
                    out.rgba[index..index + 4].copy_from_slice(&sample);
                }
            }
        }
    }
    out
}

/// Mode 4 (0x14003e0c0): the mean of the pixels `offset` either side, mixed half and
/// half with the pixel itself; alpha too.
fn neighbours(source: &ImageSurface, offset: u32) -> ImageSurface {
    let mut out = blank(source);
    let (width, offset) = (source.width as i32, offset.min(source.width) as i32);
    for y in 0..source.height as i32 {
        for x in 0..width {
            let mut sum = [0u32; 4];
            let mut count = 0;
            for (neighbour, present) in [(x - offset, x - offset >= 0), (x + offset, x + offset < width)] {
                if present {
                    for (total, &value) in sum.iter_mut().zip(at(source, neighbour, y)) {
                        *total += u32::from(value);
                    }
                    count += 1;
                }
            }
            if count >= 2 {
                sum.iter_mut().for_each(|total| *total >>= 1);
            }
            let index = ((y * width + x) * 4) as usize;
            for channel in 0..4 {
                out.rgba[index + channel] = ((sum[channel] + u32::from(source.rgba[index + channel])) >> 1) as u8;
            }
        }
    }
    out
}

/// Mode 9 (0x14003e500): grey tinted by the colour, alpha kept.
fn grey_tint(source: &ImageSurface, color: u32) -> ImageSurface {
    let mut out = source.clone();
    for pixel in out.rgba.chunks_exact_mut(4) {
        let grey = (u32::from(pixel[0]) + u32::from(pixel[1]) + u32::from(pixel[2])) / 3;
        for (channel, value) in pixel[..3].iter_mut().enumerate() {
            *value = (grey * component(color, channel) / 255) as u8;
        }
    }
    out
}

/// Mode 10 (0x14003d9e0): the mean of every second pixel within `reach` along the row.
fn box_blur(source: &ImageSurface, reach: u32) -> ImageSurface {
    let mut out = blank(source);
    let (width, reach) = (source.width as i32, reach.min(source.width.next_multiple_of(2)) as i32);
    for y in 0..source.height as i32 {
        for x in 0..width {
            let mut sum = [0u32; 3];
            let mut count = 0;
            for offset in (-reach..=reach).step_by(2) {
                if (0..width).contains(&(x + offset)) {
                    for (total, &value) in sum.iter_mut().zip(at(source, x + offset, y)) {
                        *total += u32::from(value);
                    }
                    count += 1;
                }
            }
            let index = ((y * width + x) * 4) as usize;
            for channel in 0..3 {
                out.rgba[index + channel] = (sum[channel] / count.max(1)) as u8;
            }
            out.rgba[index + 3] = 255;
        }
    }
    out
}

/// Modes 11..=14 (0x14003ce80, 0x14003d430): samples along the line through the centre,
/// away from it (`inward` false) or towards it, in 10-bit fixed point. `scaled` grows the
/// sample count with the distance.
fn radial(source: &ImageSurface, length: u32, scaled: bool, inward: bool) -> ImageSurface {
    let mut out = blank(source);
    let (width, height) = (source.width as i32, source.height as i32);
    let centre = [width >> 1, height >> 1];
    let length = i64::from(length);
    for y in 0..height {
        for x in 0..width {
            let index = ((y * width + x) * 4) as usize;
            let (_, distance) = polar(centre, [x, y]);
            if distance <= 0 {
                out.rgba[index..index + 3].copy_from_slice(&source.rgba[index..index + 3]);
                out.rgba[index + 3] = 255;
                continue;
            }
            let samples = if scaled { length * i64::from(distance) / i64::from(centre[0].max(1)) + 2 } else { length + 1 };
            let step = [((centre[0] - x) << 10) / distance, ((centre[1] - y) << 10) / distance];
            let mut point = [x << 10, y << 10];
            let mut sum = [0u32; 3];
            let mut count = 0u32;
            for _ in 0..samples.max(0) {
                let [px, py] = [point[0] >> 10, point[1] >> 10];
                // Both axes advance monotonically. Once outside, no later tap contributes.
                if !(0..width).contains(&px) || !(0..height).contains(&py) { break; }
                for (total, &value) in sum.iter_mut().zip(at(source, px, py)) {
                    *total += u32::from(value);
                }
                count += 1;
                if step == [0, 0] { break; }
                for axis in 0..2 {
                    point[axis] = if inward { point[axis] + step[axis] } else { point[axis] - step[axis] };
                }
            }
            for channel in 0..3 {
                out.rgba[index + channel] = (sum[channel] / count.max(1)) as u8;
            }
            out.rgba[index + 3] = 255;
        }
    }
    out
}

/// Modes 15 and 16 (0x14003dbc0): one-degree samples on the circle through the pixel.
/// The original places the circle about (320, 240) whatever the layer size.
fn spin(source: &ImageSurface, length: u32, scaled: bool) -> ImageSurface {
    let mut out = blank(source);
    let (width, height) = (source.width as i32, source.height as i32);
    let centre = [width >> 1, height >> 1];
    let length = i64::from(length);
    for y in 0..height {
        for x in 0..width {
            let index = ((y * width + x) * 4) as usize;
            let (angle, distance) = polar(centre, [x, y]);
            if distance <= 0 {
                out.rgba[index..index + 3].copy_from_slice(&source.rgba[index..index + 3]);
                out.rgba[index + 3] = 255;
                continue;
            }
            let samples = if scaled { length * i64::from(distance) / i64::from(centre[0].max(1)) + 1 } else { length };
            let first = 180 - samples / 2 + i64::from(angle);
            let mut sum = [0u64; 3];
            let mut count = 0u64;
            // Identical angles repeat after a revolution. Weight each distinct sample.
            for step in 0..=samples.min(359) {
                let repeats = ((samples - step) / 360 + 1) as u64;
                let degrees = ((first + step) % 360) as i32;
                let radians = if degrees == 0 { 0.0 } else { f64::from(degrees) / 180.0 * std::f64::consts::PI };
                let px = 320 - (radians.cos() * f64::from(distance)) as i32;
                let py = 240 - (radians.sin() * f64::from(distance)) as i32;
                if (0..width).contains(&px) && (0..height).contains(&py) {
                    for (total, &value) in sum.iter_mut().zip(at(source, px, py)) {
                        *total += u64::from(value) * repeats;
                    }
                    count += repeats;
                }
            }
            if count == 0 {
                out.rgba[index..index + 4].copy_from_slice(&[0, 0, 0, 255]);
            } else {
                for channel in 0..3 {
                    out.rgba[index + channel] = (sum[channel] / count) as u8;
                }
                out.rgba[index + 3] = 255;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat(width: u32, height: u32, pixel: [u8; 4]) -> ImageSurface {
        ImageSurface { width, height, rgba: pixel.repeat((width * height) as usize) }
    }

    #[test]
    fn layers_blend_over_an_opaque_base() {
        let mut stack = LayerStack::default();
        stack.place(0, flat(4, 4, [0, 0, 0, 10]), false, [0, 0]);
        stack.place(1, flat(2, 2, [200, 100, 0, 128]), true, [3, -1]);
        let out = stack.bake().unwrap();
        assert_eq!(at(&out, 0, 0), [0, 0, 0, 255]);
        assert_eq!(at(&out, 3, 0), [100, 50, 0, 255]);
        assert_eq!(at(&out, 3, 1), [0, 0, 0, 255]);
    }

    #[test]
    fn tint_multiply_and_filters_follow_the_integer_kernels() {
        let mut stack = LayerStack::default();
        stack.place(0, flat(4, 2, [90, 30, 60, 255]), false, [0, 0]);
        assert!(stack.tint(0, 0x00ff_8000));
        assert_eq!(&stack.layers[0].as_ref().unwrap().pixels.rgba[..4], [60, 30, 0, 255]);
        // Every layer until the first empty one, which reports false.
        assert!(!stack.multiply(16, 0x0080_0100));
        assert_eq!(&stack.layers[0].as_ref().unwrap().pixels.rgba[..4], [60, 0, 0, 255]);
        assert!(stack.filter(0, 1, 0));
        assert_eq!(&stack.bake().unwrap().rgba[..4], [195, 255, 255, 255]);
        assert!(!stack.filter(3, 1, 0));
        assert!(stack.filter(0, 10, 3));
        assert_eq!(at(&stack.bake().unwrap(), 1, 0), [60, 0, 0, 255]);
    }
}
