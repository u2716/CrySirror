// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Original MOVE52 rectangle/count policy. Shared Composite performs pixels.
use crate::native_geometry::NativeRects;
use anyhow::{ensure, Result};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeMoveState {
    pub count: i32,
    pub frames: i32,
    pub ended: bool,
    pub rectangles: NativeRects,
}
impl NativeMoveState {
    pub fn parse(parameters: &[i32], rectangles: NativeRects) -> Result<Self> {
        let frames = parameters.first().copied().unwrap_or(20);
        ensure!(
            (-864000..=864000).contains(&frames),
            "NNN Move native frame bounds"
        );
        Ok(Self {
            count: 0,
            frames,
            ended: false,
            rectangles,
        })
    }
    pub fn command(&self) -> i32 {
        if self.ended {
            0
        } else {
            52
        }
    }
    /// CAllEffect::CalcuOnly runs even when Eye is off, but only when a picture
    /// is assigned. It interpolates XY; width/height stay From until the end.
    pub fn calcu(&mut self, has_picture: bool) {
        if !has_picture {
            return;
        }
        if self.count >= self.frames || self.frames == 0 {
            self.rectangles.destination = self.rectangles.destination_to;
            return;
        }
        let mut destination = self.rectangles.destination_from;
        for axis in 0..2 {
            let product = self.rectangles.destination_to[axis]
                .wrapping_sub(destination[axis])
                .wrapping_mul(self.count);
            let half = self.frames >> 1;
            let offset = if product >= 0 {
                product.wrapping_add(half) / self.frames
            } else {
                product
                    .wrapping_neg()
                    .wrapping_add(half)
                    .wrapping_div(self.frames)
                    .wrapping_neg()
            };
            destination[axis] = destination[axis].wrapping_add(offset);
        }
        self.rectangles.destination = destination;
    }
    /// Native CountUp is after Print; reaching max copies both current rects
    /// and From endpoints to To, then changes command52 to ordinary NOP0.
    pub fn count_up(&mut self) {
        if self.count < self.frames {
            self.count = self.count.saturating_add(1).min(self.frames);
            if self.count == self.frames {
                self.rectangles.destination = self.rectangles.destination_to;
                self.rectangles.source = self.rectangles.source_to;
                self.rectangles.destination_from = self.rectangles.destination_to;
                self.rectangles.source_from = self.rectangles.source_to;
                self.ended = true;
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn signed_rounding_xy_only_and_post_print_endpoint_are_native() {
        let mut r = NativeRects::natural([8, 4]);
        r.set_destination_pair([3, -3, 8, 4], [0, 0, 6, 2]);
        r.source_to = [1, 1, 6, 2];
        let mut state = NativeMoveState::parse(&[2, 999], r).unwrap();
        state.calcu(true);
        assert_eq!(state.rectangles.destination, [3, -3, 8, 4]);
        state.count_up();
        state.calcu(true);
        assert_eq!(state.rectangles.destination, [1, -1, 8, 4]);
        assert_eq!(state.command(), 52);
        state.count_up();
        assert_eq!(state.rectangles.destination, [0, 0, 6, 2]);
        assert_eq!(state.rectangles.source, [1, 1, 6, 2]);
        assert_eq!(state.command(), 0);
    }
    #[test]
    fn zero_negative_and_picture_absent_do_not_fake_countmax() {
        for frames in [0, -1] {
            let mut r = NativeRects::natural([2, 2]);
            r.set_destination_pair([0, 0, 2, 2], [7, 9, 1, 1]);
            let mut state = NativeMoveState::parse(&[frames], r.clone()).unwrap();
            state.calcu(false);
            assert_eq!(state.rectangles, r);
            state.calcu(true);
            assert_eq!(state.rectangles.destination, [7, 9, 1, 1]);
            state.count_up();
            assert_eq!(state.count, 0);
            assert_eq!(state.command(), 52);
        }
        assert_eq!(
            NativeMoveState::parse(&[], NativeRects::natural([1, 1]))
                .unwrap()
                .frames,
            20
        );
    }
}
