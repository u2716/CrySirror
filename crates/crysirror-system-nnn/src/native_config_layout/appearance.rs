// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! CCommonButton::AppearPrint/Put/StretchPut/SpecialPrint geometry and clock
//! policy. Shared image services execute copying, scaling and blending.
use crate::native_title::{NativeButtonSpec, NativeButtonState};
use anyhow::{Result, ensure};
#[derive(Clone, Copy, Debug)]
pub(crate) enum Paint {
    Normal,
    Copy {
        source: [i32; 4],
        destination: [i32; 2],
        percent: i32,
    },
    Stretch {
        source: [i32; 4],
        destination: [i32; 4],
        percent: i32,
    },
}
pub(crate) fn plan(
    spec: &NativeButtonSpec,
    state: &mut NativeButtonState,
    picture: [u32; 2],
    count: i32,
    maximum: i32,
    kind: i32,
    delta: [i32; 2],
    number: i32,
    total: i32,
) -> Result<Option<Paint>> {
    if count >= maximum {
        return Ok(Some(Paint::Normal));
    }
    if !spec.exists {
        return Ok(None);
    }
    ensure!(
        total > 0 && number >= 0,
        "NNN button appearance group bounds"
    );
    let divisor = maximum.max(1);
    let percent = if kind & 1 != 0 {
        (count.wrapping_mul(100) / divisor).clamp(0, 100)
    } else {
        100
    };
    let style = kind / 2;
    if !(0..=11).contains(&style) {
        return Ok(None);
    } // Original switch has no fallback painter.
    let mut offset = delta.map(|d| d.wrapping_mul(divisor.wrapping_sub(count)) / divisor);
    if matches!(style, 1 | 2) {
        let rotate = if style == 2 {
            f64::from(count) / f64::from(divisor) * (3.14159 * 2.)
        } else {
            0.
        };
        let angle = f64::from(number) / f64::from(total) * (3.14159 * 2.) + rotate;
        let [x, y] = offset.map(f64::from);
        offset = [
            (x * angle.cos() - y * angle.sin() + 0.5) as i32,
            (x * angle.sin() + y * angle.cos() + 0.5) as i32,
        ];
    }
    if style <= 5 && percent <= 0 {
        return Ok(None);
    }
    let mut reset = state.clone();
    reset.animation[reset.mode] = 0;
    let (cells, _) = reset.frame(spec);
    let mut source = spec.source_rect(state.mode, cells[0], picture)?;
    let mut at = std::array::from_fn(|i| spec.origin[i].wrapping_add(offset[i]));
    let paint = match style {
        0..=2 => Paint::Copy {
            source,
            destination: at,
            percent,
        },
        3..=5 => {
            let multi = (count.wrapping_mul(100) / divisor).min(100);
            if multi <= 0 {
                return Ok(None);
            }
            let scale = [
                if style == 4 { 100 } else { multi },
                if style == 5 { 100 } else { multi },
            ];
            let half = std::array::from_fn::<_, 2, _>(|axis| {
                (spec.size[axis] / 2).wrapping_mul(scale[axis]) / 100
            });
            let center = std::array::from_fn::<_, 2, _>(|axis| {
                spec.origin[axis].wrapping_add(spec.size[axis] / 2)
            });
            // StretchPut explicitly resets delta to0 in this branch.
            Paint::Stretch {
                source,
                destination: [
                    center[0] - half[0],
                    center[1] - half[1],
                    half[0] * 2,
                    half[1] * 2,
                ],
                percent,
            }
        }
        _ => {
            let phase = (count.wrapping_mul(10000) / divisor).clamp(0, 10000);
            let delay = 10000 / (total * 2 + 1);
            let phase = phase - number * delay;
            if phase < 0 {
                return Ok(None);
            }
            let percent = (phase * 100 / (10000 - total * delay).max(1)).min(100);
            let direction = style - 6;
            let axis = if matches!(direction, 2 | 3 | 5) { 0 } else { 1 };
            let centered = direction >= 4;
            // PutInFrom* resets its animation before testing the cropped
            // size; a zero-percent invocation still resets that clock.
            state.animation[state.mode] = 0;
            let skip =
                spec.size[axis].wrapping_mul(100 - percent) / if centered { 200 } else { 100 };
            source[axis + 2] -= if centered { skip * 2 } else { skip };
            if source[axis + 2] <= 0 {
                return Ok(None);
            }
            if centered || matches!(direction, 0 | 2) {
                source[axis] += skip;
            }
            // PutInFrom* use the original point, not the earlier delta.
            at = spec.origin;
            if centered || matches!(direction, 1 | 3) {
                at[axis] += skip;
            }
            Paint::Copy {
                source,
                destination: at,
                percent: 100,
            }
        }
    };
    // Every actually invoked Put/StretchPut/PutInFrom resets the CURRENT
    // animation cell to0, but never the click/hover timer or other modes.
    state.animation[state.mode] = 0;
    Ok(Some(paint))
}
#[derive(Clone, Debug, Default)]
pub(crate) struct EntryClock {
    pub count: i32,
}
impl EntryClock {
    pub(crate) fn sample(&self, wait: i32, frames: i32) -> (bool, i32, i32) {
        if self.count < wait {
            (true, 0, 1)
        } else if self.count < wait.wrapping_add(frames) {
            (true, self.count.wrapping_sub(wait), frames)
        } else {
            (false, 1, 1)
        }
    }
    /// True consumes the whole native Calcu, including the count-to-end tick.
    /// Config's ordinary controls resume only on the following admitted update.
    pub(crate) fn advance(&mut self, wait: i32, frames: i32, skip: bool) -> bool {
        if !self.sample(wait, frames).0 {
            return false;
        }
        self.count = self.count.wrapping_add(1);
        if self.count >= wait.wrapping_add(frames) || skip {
            self.count = wait.wrapping_add(frames);
        }
        true
    }
}
