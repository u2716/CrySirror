// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! CPicture::CutDataByMask/ClipChara metadata. Source bounds are independent
//! of effect rectangles and alpha bytes. Shared coverage queries scan pixels
//! on a worker; this module only applies native padding and header policy.
use anyhow::{Result, ensure};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CharaClip {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}
impl CharaClip {
    pub(crate) fn from_bounds([left,top,right,bottom]:[i32;4])->Self { Self{left,top,right,bottom} }

    pub(crate) fn bounds(self) -> [i32; 4] {
        [self.left, self.top, self.right, self.bottom]
    }
    pub(crate) fn full(size: [u32; 2]) -> Self {
        Self {
            left: 0,
            top: 0,
            right: size[0] as i32,
            bottom: size[1] as i32,
        }
    }
    pub(crate) fn header(size: [u32; 2], insets: [i32; 4]) -> Self {
        // DWQ stores top,left,right,bottom in that order at16/20/24/28.
        Self {
            left: insets[1],
            top: insets[0],
            right: (size[0] as i32).wrapping_sub(insets[2]),
            bottom: (size[1] as i32).wrapping_sub(insets[3]),
        }
    }
    pub(crate) fn from_coverage(size: [u32; 2], bounds: Option<[u32; 4]>) -> Result<Self> {
        ensure!(
            size.iter().all(|v| (1..=16384).contains(v)),
            "NNN CharaClip dimensions"
        );
        let (left, top, mut right) = if let Some([x, y, w, h]) = bounds {
            ensure!(
                w > 0
                    && h > 0
                    && x.checked_add(w).is_some_and(|n| n <= size[0])
                    && y.checked_add(h).is_some_and(|n| n <= size[1]),
                "NNN CharaClip coverage bounds"
            );
            let end = x + w;
            (
                x.saturating_sub(1),
                y.saturating_sub(1),
                if end == size[0] { end } else { end + 1 },
            )
        } else {
            (size[0] - 1, size[1] - 1, 1)
        };
        // The original right-to-left loop keeps one blank column before this
        // extra increment and omits it at width-1. Bottom is never trimmed.
        // These asymmetric margins also retain the native empty-mask result.
        if right < size[0] - 1 {
            right += 1;
        }
        Ok(Self {
            left: left as i32,
            top: top as i32,
            right: right as i32,
            bottom: size[1] as i32,
        })
    }
    pub(crate) fn source_clip(
        self,
        source: [i32; 4],
        destination: [i32; 2],
    ) -> Result<Option<([i32; 4], [i32; 2])>> {
        crysirror_image::coverage_bounds::intersect_source_window(
            source,
            destination,
            [self.left, self.top, self.right, self.bottom],
        )
    }
}
