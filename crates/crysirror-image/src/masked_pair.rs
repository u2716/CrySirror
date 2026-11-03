// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Three immutable inputs with independently quantized mask contributions.
use crate::{effects::MAX_EFFECT_PIXELS, ImageSurface};
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
/// Two equally sized source rectangles over an immutable destination. Sources
/// are read from their admission-time images, even when caller leases alias.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaskedPairBlend {
    pub sources: [[i32; 4]; 2],
    pub destination: [i32; 2],
    /// Independent weights with sum <=256. Partial masks quantize to14bits;
    /// a full255 mask uses exactly weight<<6. Each RGB product floors separately.
    pub weights256: [u16; 2],
    #[serde(default)]
    pub clip: Option<[i32; 4]>,
}
impl MaskedPairBlend {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.sources.iter().all(|r| r[2] >= 0 && r[3] >= 0),
            "masked pair source extent"
        );
        ensure!(
            self.sources[0][2..] == self.sources[1][2..],
            "masked pair source sizes differ"
        );
        ensure!(
            u32::from(self.weights256[0]) + u32::from(self.weights256[1]) <= 256,
            "masked pair weights"
        );
        ensure!(
            self.clip.is_none_or(|r| r[2] >= 0 && r[3] >= 0),
            "masked pair clip extent"
        );
        Ok(())
    }
    pub fn apply(
        &self,
        destination: &ImageSurface,
        first: &ImageSurface,
        second: &ImageSurface,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<ImageSurface> {
        self.validate()?;
        destination.validate()?;
        first.validate()?;
        second.validate()?;
        check()?;
        ensure!(
            u64::from(destination.width) * u64::from(destination.height) <= MAX_EFFECT_PIXELS,
            "masked pair pixel budget"
        );
        let [dx, dy] = self.destination.map(i64::from);
        let [ax, ay, w, h] = self.sources[0].map(i64::from);
        let [bx, by, _, _] = self.sources[1].map(i64::from);
        let [cx, cy, cw, ch] = self.clip.map(|r| r.map(i64::from)).unwrap_or([
            0,
            0,
            i64::from(destination.width),
            i64::from(destination.height),
        ]);
        let left = 0.max(-ax).max(-bx).max(-dx).max(cx - dx);
        let top = 0.max(-ay).max(-by).max(-dy).max(cy - dy);
        let right = w
            .min(i64::from(first.width) - ax)
            .min(i64::from(second.width) - bx)
            .min(i64::from(destination.width) - dx)
            .min(cx + cw - dx);
        let bottom = h
            .min(i64::from(first.height) - ay)
            .min(i64::from(second.height) - by)
            .min(i64::from(destination.height) - dy)
            .min(cy + ch - dy);
        let mut out = destination.clone();
        for y in top..bottom {
            check()?;
            for x in left..right {
                if x % 4096 == 0 {
                    check()?;
                }
                let ai = (((ay + y) * i64::from(first.width) + ax + x) * 4) as usize;
                let bi = (((by + y) * i64::from(second.width) + bx + x) * 4) as usize;
                let di = (((dy + y) * i64::from(destination.width) + dx + x) * 4) as usize;
                let a = coverage(first.rgba[ai + 3], self.weights256[0]);
                let b = coverage(second.rgba[bi + 3], self.weights256[1]);
                for c in 0..3 {
                    out.rgba[di + c] = ((u32::from(first.rgba[ai + c]) * a >> 14)
                        + (u32::from(second.rgba[bi + c]) * b >> 14)
                        + (u32::from(destination.rgba[di + c]) * (16384 - a - b) >> 14))
                        as u8;
                }
            }
        }
        check()?;
        Ok(out)
    }
}
fn coverage(mask: u8, weight: u16) -> u32 {
    if mask == 255 {
        u32::from(weight) << 6
    } else {
        u32::from(weight) * u32::from(mask) >> 2
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn spec() -> MaskedPairBlend {
        MaskedPairBlend {
            sources: [[0, 0, 1, 1]; 2],
            destination: [0, 0],
            weights256: [64, 192],
            clip: None,
        }
    }
    #[test]
    fn independent_masks_and_separate_floors_keep_exact_endpoints_and_alpha() {
        let d = ImageSurface::filled(1, 1, [100, 100, 100, 91]).unwrap();
        for (a, b, want) in [
            (255, 255, 65),
            (128, 255, 52),
            (0, 128, 69),
            (255, 0, 125),
            (0, 0, 100),
        ] {
            let a = ImageSurface::filled(1, 1, [200, 200, 200, a]).unwrap();
            let b = ImageSurface::filled(1, 1, [20, 20, 20, b]).unwrap();
            assert_eq!(
                spec().apply(&d, &a, &b, &|| Ok(())).unwrap().rgba,
                [want, want, want, 91]
            );
        }
        let d = ImageSurface::filled(1, 1, [255, 255, 255, 77]).unwrap();
        let s = ImageSurface::filled(1, 1, [255, 255, 255, 128]).unwrap();
        let mut p = spec();
        p.weights256 = [128, 128];
        assert_eq!(
            p.apply(&d, &s, &s, &|| Ok(())).unwrap().rgba,
            [253, 253, 253, 77]
        );
        let encoded = serde_json::to_vec(&p).unwrap();
        assert_eq!(
            serde_json::from_slice::<MaskedPairBlend>(&encoded)
                .unwrap()
                .weights256,
            [128, 128]
        );
    }
    #[test]
    fn clipping_preserves_both_source_phases_and_aliases_read_the_immutable_input() {
        let d = ImageSurface {
            width: 4,
            height: 1,
            rgba: [10, 20, 30, 40]
                .into_iter()
                .flat_map(|r| [r, r, r, 255])
                .collect(),
        };
        let mut p = spec();
        p.sources = [[0, 0, 3, 1]; 2];
        p.destination = [1, 0];
        p.weights256 = [256, 0];
        p.clip = Some([2, 0, 2, 1]);
        assert_eq!(
            p.apply(&d, &d, &d, &|| Ok(()))
                .unwrap()
                .rgba
                .chunks_exact(4)
                .map(|p| p[0])
                .collect::<Vec<_>>(),
            [10, 20, 20, 30]
        );
        p.sources = [[i32::MIN, i32::MAX, i32::MAX, i32::MAX]; 2];
        p.destination = [i32::MAX, i32::MIN];
        p.clip = None;
        assert_eq!(p.apply(&d, &d, &d, &|| Ok(())).unwrap().rgba, d.rgba);
    }
    #[test]
    fn invalid_admission_and_cancellation_do_not_mutate_inputs() {
        let d = ImageSurface::filled(2, 2, [9, 8, 7, 6]).unwrap();
        let original = d.clone();
        let mut p = spec();
        p.weights256 = [256, 1];
        assert!(p.apply(&d, &d, &d, &|| Ok(())).is_err());
        p = spec();
        p.sources[1][2] = 2;
        assert!(p.validate().is_err());
        p = spec();
        p.clip = Some([0, 0, -1, 1]);
        assert!(p.validate().is_err());
        p = spec();
        let calls = std::cell::Cell::new(0);
        assert!(p
            .apply(&d, &d, &d, &|| {
                calls.set(calls.get() + 1);
                anyhow::ensure!(calls.get() < 2, "cancel after admission");
                Ok(())
            })
            .is_err());
        assert_eq!(d, original);
    }
}
