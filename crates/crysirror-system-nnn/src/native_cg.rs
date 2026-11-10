// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! CCGDataControl header grammar and CCommonPrintCG counters. No pixel or
//! interpolation kernel lives here: Layers consume the native integer offsets.
use crate::config::Names;
use anyhow::{Context, Result, ensure};

#[derive(Clone, Debug)]
pub(crate) struct CgChoice {
    pub raw: Vec<u8>,
    pub block: usize,
    pub index: usize,
}
#[derive(Clone, Debug)]
pub(crate) enum CgAudio {
    Sound(i32),
    Voice(Vec<u8>),
    StopScripts,
}
pub(crate) fn cg_voice(table: &Names, group: usize, index: usize) -> Result<Option<Vec<u8>>> {
    ensure!(
        table.tokens.len() % 3 == 0,
        "NNN cgVoice triple record layout"
    );
    let mut start = 0;
    let mut end = 0;
    let mut previous = None;
    for (i, row) in table.tokens.chunks_exact(3).enumerate() {
        let player = crate::config::integer(&row[0])?;
        let cg = crate::config::integer(&row[1])?;
        ensure!(
            player > 0 && player <= 256 && cg > 0,
            "NNN cgVoice one-based indices"
        );
        if player as usize == group + 1 {
            if previous != Some(player) {
                start = i;
            }
            end = i + 1;
        }
        previous = Some(player);
    }
    for row in table.tokens[start * 3..end * 3].chunks_exact(3).rev() {
        if crate::config::integer(&row[1])? as usize <= index + 1 {
            return Ok(Some(row[2].clone()));
        }
    }
    Ok(None)
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct AxisOverride {
    pub reverse: Option<bool>,
    pub tiles: Option<u32>,
    pub frames: Option<u32>,
    pub wait: Option<u32>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CgEntry {
    pub name: Vec<u8>,
    pub animation: Option<(u32, u32)>,
    pub axes: [AxisOverride; 2],
}
fn digits(input: &mut &[u8]) -> Result<u32> {
    let count = input.iter().take_while(|b| b.is_ascii_digit()).count();
    ensure!(count > 0 && count <= 10, "NNN CG decimal field extent");
    let mut value = 0u32;
    for byte in &input[..count] {
        value = value
            .checked_mul(10)
            .and_then(|v| v.checked_add(u32::from(*byte - b'0')))
            .context("NNN CG decimal field overflow")?;
    }
    *input = &input[count..];
    Ok(value)
}
impl CgEntry {
    pub fn parse(raw: &[u8], code_byte: i32) -> Result<Self> {
        ensure!(
            raw.len() <= 1024 && !raw.contains(&0),
            "NNN CG catalogue token bounds"
        );
        let mut name = raw;
        for prefix in [b'*', b'@', b'!'] {
            if name.first() == Some(&prefix) {
                name = &name[1..];
            }
        }
        let mut animation = None;
        let mut had_animation_header = false;
        if name.first() == Some(&b'#') {
            had_animation_header = true;
            let end = name
                .iter()
                .position(|b| *b == b'/')
                .context("NNN CG animation header missing slash")?;
            let mut numbers = &name[1..end];
            let start = digits(&mut numbers)?;
            ensure!(
                !numbers.is_empty() && !numbers[0].is_ascii_digit(),
                "NNN CG animation count separator"
            );
            numbers = &numbers[1..];
            let count = digits(&mut numbers)?;
            ensure!(numbers.is_empty(), "NNN CG animation trailing header data");
            ensure!(
                count <= 256 && start.checked_add(count).is_some(),
                "NNN CG animation frame budget"
            );
            if count != 0 {
                animation = Some((start, count));
            }
            name = &name[end + 1..];
        }
        let mut axes = [AxisOverride::default(); 2];
        if name.first() == Some(&b'[') {
            let end = name
                .iter()
                .position(|b| *b == b']')
                .context("NNN CG scroll header missing bracket")?;
            // GetCGScrollSpecial examines the raw token before GetCGFileName
            // strips headers. A preceding # header therefore disables it.
            if !had_animation_header {
                let mut rest = &name[1..end];
                let mut axis = None;
                let mut sub = 0;
                while !rest.is_empty() {
                    let arrow = if code_byte == 1 {
                        match rest[0] {
                            b'^' => Some((1, false, 1)),
                            b'|' => Some((1, true, 1)),
                            b'<' => Some((0, false, 1)),
                            b'>' => Some((0, true, 1)),
                            _ => None,
                        }
                    } else {
                        match rest.get(..2) {
                            Some([0x81, 0xaa]) => Some((1, false, 2)),
                            Some([0x81, 0xab]) => Some((1, true, 2)),
                            Some([0x81, 0xa9]) => Some((0, false, 2)),
                            Some([0x81, 0xa8]) => Some((0, true, 2)),
                            _ => None,
                        }
                    };
                    if let Some((index, reverse, width)) = arrow {
                        axes[index].reverse = Some(reverse);
                        axis = Some(index);
                        sub = 0;
                        rest = &rest[width..];
                    } else if matches!(rest[0], b':' | b'_') {
                        sub = if rest[0] == b':' { 1 } else { 2 };
                        rest = &rest[1..];
                    } else if rest[0].is_ascii_digit() {
                        let value = digits(&mut rest)?;
                        let index = axis.context("NNN CG scroll number without direction")?;
                        match sub {
                            0 => {
                                ensure!((1..=16).contains(&value), "NNN CG tile axis budget");
                                axes[index].tiles = Some(value);
                            }
                            1 => axes[index].frames = (value != 0).then_some(value),
                            _ => axes[index].wait = Some(value),
                        }
                    } else {
                        // The C++ loop can hang on an unrecognized byte. Reject
                        // malformed headers instead of retaining that defect.
                        anyhow::bail!("NNN CG unknown scroll token byte {:#x}", rest[0]);
                    }
                }
            }
            name = &name[end + 1..];
        }
        ensure!(
            name.len() >= 2 && name.len() <= 128 && !name.contains(&b'/') && !name.contains(&b'\\'),
            "NNN CG basename bounds"
        );
        Ok(Self {
            name: name.to_vec(),
            animation,
            axes,
        })
    }
    pub fn tiles(&self) -> [u32; 2] {
        if self.animation.is_some() {
            [1, 1]
        } else {
            [
                self.axes[0].tiles.unwrap_or(1),
                self.axes[1].tiles.unwrap_or(1),
            ]
        }
    }
}
#[derive(Clone, Debug)]
pub(crate) struct CgProfile {
    pub reverse: [bool; 2],
    pub frames: [u32; 2],
    pub wait: [u32; 2],
    pub anime_hold: Option<u32>,
    pub return_one_set: bool,
    pub background: Option<Vec<u8>>,
    pub color: [u8; 4],
    pub code_byte: i32,
    pub frame_ms: u64,
    pub sounds: [i32; 3],
    pub voice_wait: u32,
}
impl CgProfile {
    pub fn read(table: &Names, setup: &Names, group: usize) -> Result<Self> {
        let n = |key: &[u8], default: i32| -> Result<i32> {
            Ok(table.integer(key)?.unwrap_or(default))
        };
        let mut frames = [0; 2];
        let mut wait = [0; 2];
        for (i, axis) in ["X", "Y"].into_iter().enumerate() {
            let f = n(format!("scrollTime{axis}").as_bytes(), 30)?;
            let w = n(format!("scrollWait{axis}").as_bytes(), 10)?;
            ensure!(
                (1..=864000).contains(&f) && (0..=864000).contains(&w),
                "NNN CG scroll time bounds"
            );
            frames[i] = f as u32;
            wait[i] = w as u32;
        }
        let hold = n(b"animeFrame", 1)?;
        ensure!(hold <= 864000, "NNN CG animation hold budget");
        let frame_ms = setup.integer(b"frameTime")?.unwrap_or(50);
        ensure!(
            (1..=60000).contains(&frame_ms),
            "NNN CG native frame period"
        );
        let bg = n(b"taBGFlag", 0)?;
        let background = if bg == 0 {
            None
        } else {
            let key = if bg == 1 {
                "taBGFileName".into()
            } else {
                format!("taBGCharaFileName{}", group + 1)
            };
            Some(
                table
                    .parameter(key.as_bytes())
                    .unwrap_or(b"bg_cgcharaback")
                    .to_vec(),
            )
        };
        let packed = n(b"taBGColorR", 0)?.wrapping_shl(16)
            | n(b"taBGColorG", 0)?.wrapping_shl(8)
            | n(b"taBGColorB", 0)?;
        Ok(Self {
            reverse: [
                n(b"scrollDirectionX", 0)? != 0,
                n(b"scrollDirectionY", 0)? != 0,
            ],
            frames,
            wait,
            anime_hold: (hold > 0).then_some(hold as u32),
            return_one_set: n(b"returnOneLookCGSet", 0)? != 0,
            background,
            color: [(packed >> 16) as u8, (packed >> 8) as u8, packed as u8, 255],
            code_byte: setup.integer(b"codeByte")?.unwrap_or(2),
            frame_ms: frame_ms as u64,
            sounds: [
                n(b"nextSound", 3)?,
                n(b"prevSound", 3)?,
                n(b"backSound", 2)?,
            ],
            voice_wait: n(b"cgVoiceWaitTime", 10)?.max(0) as u32,
        })
    }
}
#[derive(Clone, Debug)]
pub(crate) struct CgMotion {
    pub size: [u32; 2],
    pub elapsed: u64,
    frames: [u32; 2],
    wait: [u32; 2],
    reverse: [bool; 2],
    pub dragged: Option<[i64; 2]>,
    stopped: bool,
}
impl CgMotion {
    pub fn new(size: [u32; 2], entry: &CgEntry, profile: &CgProfile) -> Result<Self> {
        let mut frames = profile.frames;
        let mut wait = profile.wait;
        let mut reverse = profile.reverse;
        for i in 0..2 {
            frames[i] = entry.axes[i].frames.unwrap_or(frames[i]);
            wait[i] = entry.axes[i].wait.unwrap_or(wait[i]);
            reverse[i] = entry.axes[i].reverse.unwrap_or(reverse[i]);
            ensure!(
                (1..=864000).contains(&frames[i]) && wait[i] <= 864000,
                "NNN CG scroll override budget"
            );
        }
        Ok(Self {
            size,
            elapsed: 0,
            frames,
            wait,
            reverse,
            dragged: None,
            stopped: false,
        })
    }
    pub fn scrolling(&self) -> bool {
        !self.stopped
            && (0..2).any(|i| {
                self.size[i] > [800, 600][i]
                    && self.elapsed < u64::from(self.frames[i] + self.wait[i])
            })
    }
    pub fn stop_scroll(&mut self) {
        self.stopped = true;
    }
    pub fn position(&self) -> [i64; 2] {
        if let Some(position) = self.dragged {
            return position;
        }
        std::array::from_fn(|i| {
            let total = u64::from(self.frames[i] + self.wait[i]);
            let mut count = if self.stopped || self.size[i] <= [800, 600][i] {
                0
            } else {
                total
                    .saturating_sub(self.elapsed)
                    .min(u64::from(self.frames[i]))
            };
            if self.reverse[i] {
                count = u64::from(self.frames[i]) - count;
            }
            (i64::from(self.size[i]) - i64::from([800, 600][i])) * count as i64
                / i64::from(self.frames[i])
        })
    }
    pub fn drag(&mut self, delta: [i64; 2]) {
        let old = self.position();
        // Source clamps to zero, then to (picture-screen), in that order.
        self.dragged = Some(std::array::from_fn(|i| {
            (old[i] - delta[i])
                .max(0)
                .min(i64::from(self.size[i]) - i64::from([800, 600][i]))
        }));
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_cg_headers_preserve_animation_and_sjis_scroll_fields() {
        let entry = CgEntry::parse(b"*@!#17-3/bg_preview", 2).unwrap();
        assert_eq!(entry.animation, Some((17, 3)));
        assert_eq!(entry.name, b"bg_preview");
        let mut raw = b"[".to_vec();
        raw.extend([0x81, 0xa8]);
        raw.extend(b"2:40_7");
        raw.extend([0x81, 0xaa]);
        raw.extend(b"3:60_0]ev_pan");
        let entry = CgEntry::parse(&raw, 2).unwrap();
        assert_eq!(entry.tiles(), [2, 3]);
        assert_eq!(
            entry.axes[0],
            AxisOverride {
                reverse: Some(true),
                tiles: Some(2),
                frames: Some(40),
                wait: Some(7)
            }
        );
        assert_eq!(
            entry.axes[1],
            AxisOverride {
                reverse: Some(false),
                tiles: Some(3),
                frames: Some(60),
                wait: Some(0)
            }
        );
        assert_eq!(CgEntry::parse(b"[>2:40_7^3:60_0]ev_pan", 1).unwrap(), entry);
        let voices = Names::parse(b"1 1 ga001 1 4 ga004 2 1 gb001", false).unwrap();
        assert_eq!(cg_voice(&voices, 0, 2).unwrap(), Some(b"ga001".to_vec()));
        assert_eq!(cg_voice(&voices, 0, 3).unwrap(), Some(b"ga004".to_vec()));
        assert_eq!(cg_voice(&voices, 1, 0).unwrap(), Some(b"gb001".to_vec()));
        assert_eq!(cg_voice(&voices, 2, 0).unwrap(), None);
        // These are malformed original inputs, not unsupported no-op commands.
        for raw in [
            b"[?]ev_x".as_slice(),
            b"[>2ev_x",
            b"#4294967295-2/bg_x",
            b"[>0]ev_x",
            b"#1-257/bg_x",
        ] {
            assert!(CgEntry::parse(raw, 1).is_err());
        }
    }
}
