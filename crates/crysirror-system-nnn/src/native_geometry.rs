// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Original six EFFECT rectangles and API coordinate translation. These are
//! native parameters/save fields, not a pixel or composition implementation.
use anyhow::{ensure, Result};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeRects {
    pub source: [i32; 4],
    pub destination: [i32; 4],
    pub source_from: [i32; 4],
    pub source_to: [i32; 4],
    pub destination_from: [i32; 4],
    pub destination_to: [i32; 4],
}
impl NativeRects {
    pub fn natural(size: [u32; 2]) -> Self {
        let r = [0, 0, size[0] as i32, size[1] as i32];
        Self {
            source: r,
            destination: r,
            source_from: r,
            source_to: r,
            destination_from: r,
            destination_to: r,
        }
    }
    /// Legacy SetEffectRect0x530cd0 updates indices2/3/5 only. The fields called
    /// right/bottom hold widths/heights in the original non-stretch Blt path.
    pub fn set_destination_pair(&mut self, from: [i32; 4], to: [i32; 4]) {
        self.destination_from = from;
        self.destination_to = to;
        self.destination = from;
    }
    /// Original unscaled Blt clips its source plane and keeps destination
    /// phase. The shared integer region mapper owns clipping arithmetic; this
    /// adapter only translates original six-rectangle semantics to Layer DTOs.
    pub(crate) fn clipped_layer_with_source_bounds(&self,size:[u32;2],bounds:[i32;4])->Result<Option<([f32;4],[f32;4])>> {
        let [x,y,w,h]=self.destination;
        let [sx,sy,_,_]=self.source;
        let Some((src,at))=crysirror_image::coverage_bounds::intersect_source_window([sx,sy,w,h],[x,y],bounds)? else{return Ok(None);};
        let mut clipped=self.clone();clipped.source=src;clipped.destination=[at[0],at[1],src[2],src[3]];
        clipped.clipped_layer(size)
    }
    pub(crate) fn clipped_layer(&self, size: [u32; 2]) -> Result<Option<([f32; 4], [f32; 4])>> {
        let [sx, sy, _, _] = self.source;
        let [x, y, w, h] = self.destination;
        ensure!(
            x.unsigned_abs() <= 1_000_000 && y.unsigned_abs() <= 1_000_000,
            "NNN picture destination coordinate profile"
        );
        if w <= 0 || h <= 0 {
            return Ok(None);
        }
        let Some(([sx, sy, cw, ch], [dx, dy])) = crysirror_core::region_map::clip_region_copy(
            Some(size),
            [u32::try_from(w)?, u32::try_from(h)?],
            [i64::from(sx), i64::from(sy), i64::from(w), i64::from(h)],
            [0, 0],
            None,
        )?
        else {
            return Ok(None);
        };
        Ok(Some((
            [
                (i64::from(x) + dx) as f32,
                (i64::from(y) + dy) as f32,
                cw as f32,
                ch as f32,
            ],
            [
                sx as f32 / size[0] as f32,
                sy as f32 / size[1] as f32,
                (sx + cw) as f32 / size[0] as f32,
                (sy + ch) as f32 / size[1] as f32,
            ],
        )))
    }
    /// A plain native picture maps to an ordinary engine rectangle and UVs.
    /// Other source clipping/scaling profiles remain explicit rather than
    /// replacing a native blit with a stretched/clamped texture.
    pub fn plain_layer(&self, size: [u32; 2]) -> Result<([f32; 4], [f32; 4])> {
        let [sx, sy, _, _] = self.source;
        let [x, y, w, h] = self.destination;
        ensure!(
            sx >= 0
                && sy >= 0
                && w > 0
                && h > 0
                && i64::from(sx) + i64::from(w) <= i64::from(size[0])
                && i64::from(sy) + i64::from(h) <= i64::from(size[1]),
            "NNN plain picture source clipping requires shared unscaled-rectangle mapping"
        );
        Ok((
            [x as f32, y as f32, w as f32, h as f32],
            [
                sx as f32 / size[0] as f32,
                sy as f32 / size[1] as f32,
                (i64::from(sx) + i64::from(w)) as f32 / size[0] as f32,
                (i64::from(sy) + i64::from(h)) as f32 / size[1] as f32,
            ],
        ))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_destination_rectangles_do_not_reset_source_or_stretch_uvs() {
        let mut r = NativeRects::natural([800, 1200]);
        let before = r.source;
        r.set_destination_pair([0, -200, 800, 1200], [0, 0, 800, 1200]);
        assert_eq!(r.source, before);
        assert_eq!(r.source_to, before);
        assert_eq!(r.destination, [0, -200, 800, 1200]);
        assert_eq!(r.destination_to, [0, 0, 800, 1200]);
        assert_eq!(
            r.plain_layer([800, 1200]).unwrap(),
            ([0., -200., 800., 1200.], [0., 0., 1., 1.])
        );
        r.set_destination_pair([204, 118, 394, 367], [204, 118, 394, 367]);
        assert_eq!(
            r.plain_layer([800, 1200]).unwrap().1,
            [0., 0., 394. / 800., 367. / 1200.]
        );
        assert!(r.plain_layer([2, 2]).is_err());
    }
}

#[cfg(test)]
mod clipped_tests {
    use super::*;
    #[test]
    fn native_unscaled_copy_uses_shared_source_clip_without_stretch_or_phase_loss() {
        let mut r = NativeRects::natural([8, 6]);
        r.source = [-2, 1, 12345, 23456]; // original src right/bottom are not Blt extents
        r.destination = [-10, 20, 7, 8];
        assert_eq!(
            r.clipped_layer([8, 6]).unwrap(),
            Some(([-8., 20., 5., 5.], [0., 1. / 6., 5. / 8., 1.]))
        );
        r.source[0] = 9;
        assert_eq!(r.clipped_layer([8, 6]).unwrap(), None);
        r.source[0] = 0;
        r.destination[2] = 0;
        assert_eq!(r.clipped_layer([8, 6]).unwrap(), None);
        r.destination[2] = 8193;
        assert!(r.clipped_layer([8, 6]).is_err());
    }
}
