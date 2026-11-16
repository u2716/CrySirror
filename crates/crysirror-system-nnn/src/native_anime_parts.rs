// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! CCommonAnimeParts tagged configuration, native clocks and drawing choices.
//! Shared integer sprite sampling selects cells; engine image operations own
//! atlas clipping, masks, pair composition and all pixel work.
use crate::config::Names;
use anyhow::{Result, ensure};
use crysirror_scene::sprite_samples::SpriteSamples;

#[derive(Clone, Debug)]
pub(crate) struct AnimePartSpec {
    pub filename: Vec<u8>,
    pub cell: [i32; 2],
    pub origin: [i32; 2],
    kind: i32,
    sequence: SpriteSamples,
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct PartAppearance {
    pub count: i32,
    pub maximum: i32,
    pub kind: i32,
    pub point_is_delta: bool,
}
#[derive(Clone, Copy, Debug)]
pub(crate) enum PartPaint {
    /// Ordinary CPicture::Blt(TRUE), including RGB-key fallback.
    Masked { cell: i32 },
    /// AppearPrint's single-source branch always calls TransLucentBlt3,
    /// including appearance type0/100 percent; a real mask is required.
    Translucent { cell: i32, percent: i32 },
    /// ChangeTranslateBlt uses masks when available, otherwise its Beta
    /// profile omits destination RGB, even when both percentages are zero.
    Pair { cells: [i32; 2], percent: [i32; 2] },
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct PartPrint {
    pub destination: [i32; 2],
    pub paint: PartPaint,
}
impl AnimePartSpec {
    pub(crate) fn read(table: &Names, tag: &[u8]) -> Result<Self> {
        ensure!(
            !tag.is_empty() && tag.len() <= 64 && !tag.contains(&0),
            "NNN animation-part tag bounds"
        );
        let key = |suffix: &[u8]| [tag, suffix].concat();
        let get = |suffix: &[u8], default: i32| -> Result<i32> {
            Ok(table.integer(&key(suffix))?.unwrap_or(default))
        };
        let filename = table
            .parameter(&[b"filename".as_slice(), tag].concat())
            .unwrap_or(b"ta_anime")
            .to_vec();
        ensure!(
            !filename.is_empty() && filename.len() <= 250 && !filename.contains(&0),
            "NNN animation-part filename"
        );
        let kind = get(b"Type", 1)?;
        let pattern = get(b"Pattern", 1)?;
        let speed = get(b"Speed", 1)?;
        let cell = [get(b"SizeX", 16)?, get(b"SizeY", 16)?];
        // Native nonpositive pattern/speed could divide or take a remainder
        // by zero; zero cell width could also divide by zero in SetPicture.
        // Define those invalid constructor cases as a one-tick, one-pixel
        // minimum. Supplied valid source values retain their original meaning.
        let sequence = SpriteSamples {
            cells: pattern.max(1) as u32,
            ticks_per_cell: speed.max(1) as u32,
            ping_pong: matches!(kind, 3 | 4 | 7 | 8),
            hold: matches!(kind, 2 | 4 | 6 | 8),
        };
        sequence.validate()?;
        let cell = cell.map(|n| n.max(1));
        ensure!(
            cell.iter().all(|n| *n <= 16384),
            "NNN animation-part cell extent"
        );
        Ok(Self {
            filename,
            cell,
            origin: [get(b"PrintX", 0)?, get(b"PrintY", 0)?],
            kind,
            sequence,
        })
    }
    pub(crate) fn cycle_ticks(&self) -> Result<u64> {
        self.sequence.cycle_ticks()
    }
    pub(crate) fn source(&self, cell: i32, picture_width: u32) -> [i32; 4] {
        let columns = (picture_width as i32 / self.cell[0]).max(1);
        [
            (cell % columns).wrapping_mul(self.cell[0]),
            (cell / columns).wrapping_mul(self.cell[1]),
            self.cell[0],
            self.cell[1],
        ]
    }
    pub(crate) fn print(
        &self,
        count: u64,
        point: Option<[i32; 2]>,
        delta: bool,
        appearance: Option<PartAppearance>,
    ) -> Result<PartPrint> {
        let sample = self.sequence.sample(count)?;
        let mut cells = [sample.current as i32, sample.next as i32];
        // The one-cell reverse/hold endpoint really yields pic1=-1 in the
        // source, not0: preserve its signed source-rectangle clipping policy.
        if self.sequence.cells == 1
            && self.sequence.ping_pong
            && self.sequence.hold
            && count >= self.sequence.cycle_ticks()?
        {
            cells[0] = -1;
        }
        let percent = if self.kind >= 5 {
            ((u64::from(sample.ticks_per_cell - sample.fraction_ticks) * 100)
                / u64::from(sample.ticks_per_cell)) as i32
        } else {
            100
        };
        let mut destination = point.unwrap_or(self.origin);
        if point.is_some() && appearance.map_or(delta, |a| a.point_is_delta) {
            for axis in 0..2 {
                destination[axis] = destination[axis].wrapping_add(self.origin[axis]);
            }
        }
        let alpha = appearance.map(|a| {
            if a.kind & 1 != 0 {
                (a.count.wrapping_mul(100) / a.maximum.max(1)).clamp(0, 100)
            } else {
                100
            }
        });
        let paint = if percent == 100 || percent == 0 {
            let cell = cells[usize::from(percent == 0)];
            if let Some(percent) = alpha {
                PartPaint::Translucent { cell, percent }
            } else {
                PartPaint::Masked { cell }
            }
        } else {
            let mut weights = [percent, 100 - percent];
            if let Some(alpha) = alpha {
                weights = weights.map(|n| n * alpha / 100);
            }
            PartPaint::Pair {
                cells,
                percent: weights,
            }
        };
        Ok(PartPrint { destination, paint })
    }
}
#[derive(Clone, Debug)]
pub(crate) struct AnimePartClock {
    pub spec: AnimePartSpec,
    pub count: u64,
}
impl AnimePartClock {
    pub(crate) fn new(spec: AnimePartSpec) -> Self {
        Self { spec, count: 0 }
    }
    pub(crate) fn init(&mut self) {
        self.count = 0;
    }
    pub(crate) fn calcu(&mut self, count: u32) -> Result<()> {
        let maximum = self.spec.cycle_ticks()?;
        // Source signed overflow in an extremely long-running Calcu is
        // undefined. Reduce in a wide bounded domain before retaining state.
        let next = self.count.saturating_add(u64::from(count));
        self.count = if self.spec.sequence.hold {
            next.min(maximum)
        } else {
            next % maximum
        };
        Ok(())
    }
}
