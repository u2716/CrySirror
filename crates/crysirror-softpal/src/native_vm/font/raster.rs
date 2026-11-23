use super::PalFontProfile;

const CROSS: &[(i32, i32)] = &[(0, -1), (-1, 0), (1, 0), (0, 1)];
const DIAMOND: &[(i32, i32)] = &[
    (0, -2),
    (-1, -1),
    (0, -1),
    (1, -1),
    (-2, 0),
    (-1, 0),
    (1, 0),
    (2, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
    (0, 2),
];

#[derive(Clone, Copy)]
enum Stamp {
    Overwrite,
    Or,
    Blend,
}

/// Recovered six-bit PAL glyph kernels, with checked destination clipping.
/// Native kernel loops can write outside a caller surface for negative/empty
/// rectangles; those memory faults are not reproduced by the portable adapter.
#[allow(clippy::too_many_arguments)]
pub(super) fn draw_glyph(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    coverage: &[u8],
    coverage_stride: u32,
    coverage_height: u32,
    x: i32,
    y: i32,
    color: u32,
    effect_color: u32,
    effect: u16,
    profile: PalFontProfile,
) {
    if effect > 3 || (effect == 3 && profile == PalFontProfile::Koikake) {
        return;
    }
    let foreground_offset = if effect == 3 { 2 } else { 1 };
    let mut stamp = |offset_x: i32, offset_y: i32, color: u32, mode: Stamp| {
        for source_y in 0..coverage_height {
            for source_x in 0..coverage_stride {
                let Some(&value) = coverage.get((source_y * coverage_stride + source_x) as usize)
                else {
                    return;
                };
                if value == 0 {
                    continue;
                }
                let px = i64::from(x) + i64::from(source_x) + i64::from(offset_x);
                let py = i64::from(y) + i64::from(source_y) + i64::from(offset_y);
                if px < 0 || py < 0 || px >= i64::from(width) || py >= i64::from(height) {
                    continue;
                }
                let index = (py as usize * width as usize + px as usize) * 4;
                let Some(destination) = pixels.get_mut(index..index + 4) else {
                    continue;
                };
                let alpha = value.wrapping_mul(4).wrapping_sub(1);
                let rgb = super::argb_to_rgba(color);
                let source = [rgb[0], rgb[1], rgb[2], rgb[3] | alpha];
                match mode {
                    Stamp::Overwrite => destination.copy_from_slice(&source),
                    Stamp::Or => {
                        for (dst, src) in destination.iter_mut().zip(source) {
                            *dst |= src;
                        }
                    }
                    Stamp::Blend if alpha == 255 || destination[3] == 0 => {
                        destination.copy_from_slice(&source)
                    }
                    Stamp::Blend => {
                        for channel in 0..4 {
                            let value = if channel == 3 {
                                255
                            } else {
                                u32::from(rgb[channel])
                            };
                            destination[channel] = ((value * u32::from(alpha)
                                + u32::from(destination[channel]) * (255 - u32::from(alpha)))
                                >> 8) as u8;
                        }
                    }
                }
            }
        }
    };
    match effect {
        1 => {
            for &(dx, dy) in CROSS {
                stamp(1 + dx, 1 + dy, effect_color, Stamp::Or);
            }
        }
        2 => stamp(2, 2, effect_color, Stamp::Overwrite),
        3 => {
            for &(dx, dy) in DIAMOND {
                stamp(2 + dx, 2 + dy, effect_color, Stamp::Or);
            }
        }
        _ => {}
    }
    stamp(foreground_offset, foreground_offset, color, Stamp::Blend);
}
