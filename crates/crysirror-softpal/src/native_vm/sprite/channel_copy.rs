//! PalChannelCopyEx's clipped, byte-channel operation. Native channel words
//! refer to BGRA memory; portable image storage is RGBA.

use super::{PalRect, SpriteHandle, SpriteSystem};

impl SpriteSystem {
    /// Arena adapter for the reviewed eight-word native contract. Aliased
    /// surfaces are traversed in place, including overlap feedback.
    #[allow(clippy::too_many_arguments)]
    pub fn channel_copy_ex(
        &mut self,
        destination: SpriteHandle,
        destination_rect: &PalRect,
        destination_channel: u32,
        source: SpriteHandle,
        source_x: i32,
        source_y: i32,
        source_channel: u32,
        operation: u32,
    ) -> bool {
        if destination_channel > 3 || source_channel > 3 || operation > 2 {
            return false;
        }
        let Some(destination_id) = self.get(destination).map(|sprite| sprite.surface) else {
            return false;
        };
        let Some(source_id) = self.get(source).map(|sprite| sprite.surface) else {
            return false;
        };
        let Some(dst) = self.surface(destination_id) else {
            return false;
        };
        let Some(src) = self.surface(source_id) else {
            return false;
        };
        if [dst.width, dst.height, src.width, src.height]
            .iter()
            .any(|&dimension| dimension > i32::MAX as u32)
        {
            return false;
        }
        let mut dx = i64::from(destination_rect.left.max(0));
        let mut dy = i64::from(destination_rect.top.max(0));
        let mut width = i64::from(destination_rect.right.min(dst.width as i32)) - dx;
        let mut height = i64::from(destination_rect.bottom.min(dst.height as i32)) - dy;
        if width <= 0 || height <= 0 {
            return false;
        }
        let mut sx = i64::from(source_x) + dx - i64::from(destination_rect.left);
        let mut sy = i64::from(source_y) + dy - i64::from(destination_rect.top);
        // Native arithmetic wraps at 32 bits. Reject an overflowing rectangle
        // instead of turning it into an unrelated or invalid pointer access.
        if i32::try_from(sx).is_err() || i32::try_from(sy).is_err() {
            return false;
        }
        if sx < 0 {
            dx -= sx;
            width += sx;
            sx = 0;
        }
        if sy < 0 {
            dy -= sy;
            height += sy;
            sy = 0;
        }
        width = width.min(i64::from(src.width) - sx);
        height = height.min(i64::from(src.height) - sy);
        if width <= 0 || height <= 0 {
            return false;
        }
        let dst_width = dst.width as usize;
        let src_width = src.width as usize;
        let source_pixels = (source_id != destination_id).then(|| src.pixels.clone());
        let dst_channel = [2, 1, 0, 3][destination_channel as usize];
        let src_channel = [2, 1, 0, 3][source_channel as usize];
        let pixels = self.surface_mut(destination_id).unwrap().pixels_mut();
        for y in 0..height as usize {
            let dst_row = ((dy as usize + y) * dst_width + dx as usize) * 4;
            let src_row = ((sy as usize + y) * src_width + sx as usize) * 4;
            for x in 0..width as usize {
                let dst_index = dst_row + x * 4 + dst_channel;
                let src_index = src_row + x * 4 + src_channel;
                let source = source_pixels
                    .as_ref()
                    .map_or_else(|| pixels[src_index], |source| source[src_index]);
                pixels[dst_index] = match operation {
                    0 => source,
                    1 => pixels[dst_index].saturating_add(source),
                    2 => {
                        ((257 * (u32::from(pixels[dst_index]) * u32::from(source) + 128)) >> 16)
                            as u8
                    }
                    _ => unreachable!(),
                };
            }
        }
        true
    }
}
