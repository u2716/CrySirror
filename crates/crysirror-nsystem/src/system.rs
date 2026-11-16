// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! `SAVE\CONFIG.SAV` (HANANYU.EXE `CSaveData::SaveConfig` 0x4196B0 /
//! `LoadConfig` 0x419950), written at every `LPT`:
//!
//! - global variables 1000..1999 (i32 × 1000);
//! - u32 count, then per script a read-history record: 256 name bytes (the
//!   64-byte name plus whatever follows it in the native 72-byte table entry;
//!   kept verbatim when loaded, zero-padded when new), u32 size, bitset
//!   (`SetKidokuFlag` 0x417790: bit n = MESRUN read id n; `SetupKidoku`
//!   0x417630 grows it in 1 KiB steps);
//! - CG flags: 20 pages × 100 entries × 20 variants (`SetCGFlag` 0x4177F0,
//!   names from CGLIST.MSD screen-100 records);
//! - scene flags: 20 lists × 20 (`SetSceneFlag` 0x417930, SCENELIST.MSD);
//! - 32 bytes of settings (defaults `.data` 0x5C2F30).
use anyhow::{Context, Result, ensure};

pub const LIMIT: u64 = 16 * 1024 * 1024;
pub const CG_BYTES: usize = 40000;
pub const SCENE_BYTES: usize = 400;

#[derive(Clone, Debug, PartialEq)]
pub struct ReadHistory {
    pub name_raw: Vec<u8>,
    pub bits: Vec<u8>,
}
impl ReadHistory {
    pub fn new(name: &[u8]) -> Self {
        let mut name_raw = name.to_vec();
        name_raw.resize(256, 0);
        Self { name_raw, bits: vec![0; 1024] }
    }
    pub fn name(&self) -> &[u8] {
        let n = self.name_raw.iter().take(64).position(|c| *c == 0).unwrap_or(64);
        &self.name_raw[..n]
    }
    pub fn get(&self, id: u32) -> bool {
        self.bits.get(id as usize / 8).is_some_and(|b| b & (1 << (id % 8)) != 0)
    }
    /// `SetupKidoku` growth, then the bit.
    pub fn set(&mut self, id: u32) -> Result<()> {
        ensure!(id < 8 * 1024 * 1024, "N-System read id range");
        let need = id as usize / 8 + 1;
        if self.bits.len() < need {
            let size = ((need / 1024 + 1) << 10) + if need % 1024 != 0 { 1024 } else { 0 };
            self.bits.resize(size, 0);
        }
        self.bits[id as usize / 8] |= 1 << (id % 8);
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct System {
    pub globals: Vec<i32>,
    pub history: Vec<ReadHistory>,
    pub cg: Vec<u8>,
    pub scenes: Vec<u8>,
    pub settings: [u8; 32],
}
impl System {
    pub fn new(settings: [u8; 32]) -> Self {
        Self { globals: vec![0; 1000], history: Vec::new(), cg: vec![0; CG_BYTES], scenes: vec![0; SCENE_BYTES], settings }
    }
    pub fn decode(b: &[u8]) -> Result<Self> {
        ensure!(b.len() as u64 <= LIMIT && b.len() >= 4004, "N-System CONFIG.SAV size");
        let i32_at = |o: usize| -> Result<i32> { Ok(i32::from_le_bytes(b.get(o..o + 4).context("CONFIG.SAV truncated")?.try_into()?)) };
        let globals = (0..1000).map(|i| i32_at(4 * i)).collect::<Result<_>>()?;
        let count = i32_at(4000)? as u32;
        ensure!(count <= 1000, "CONFIG.SAV read-history count");
        let mut at = 4004;
        let mut history = Vec::new();
        for _ in 0..count {
            let name_raw = b.get(at..at + 256).context("CONFIG.SAV truncated")?.to_vec();
            let size = i32_at(at + 256)? as u32 as usize;
            ensure!(size <= 1 << 20, "CONFIG.SAV read-history size");
            let bits = b.get(at + 260..at + 260 + size).context("CONFIG.SAV truncated")?.to_vec();
            history.push(ReadHistory { name_raw, bits });
            at += 260 + size;
        }
        let cg = b.get(at..at + CG_BYTES).context("CONFIG.SAV truncated")?.to_vec();
        at += CG_BYTES;
        let scenes = b.get(at..at + SCENE_BYTES).context("CONFIG.SAV truncated")?.to_vec();
        at += SCENE_BYTES;
        let settings = b.get(at..at + 32).context("CONFIG.SAV truncated")?.try_into()?;
        ensure!(at + 32 == b.len(), "CONFIG.SAV trailing bytes");
        Ok(Self { globals, history, cg, scenes, settings })
    }
    pub fn encode(&self) -> Result<Vec<u8>> {
        ensure!(self.globals.len() == 1000 && self.cg.len() == CG_BYTES && self.scenes.len() == SCENE_BYTES && self.history.len() <= 1000, "CONFIG.SAV shape");
        let mut out = Vec::new();
        for v in &self.globals {
            out.extend(v.to_le_bytes());
        }
        out.extend((self.history.len() as u32).to_le_bytes());
        for h in &self.history {
            ensure!(h.name_raw.len() == 256, "CONFIG.SAV read-history name");
            out.extend(&h.name_raw);
            out.extend((h.bits.len() as u32).to_le_bytes());
            out.extend(&h.bits);
        }
        out.extend(&self.cg);
        out.extend(&self.scenes);
        out.extend(self.settings);
        Ok(out)
    }
    pub fn history_mut(&mut self, script: &[u8]) -> &mut ReadHistory {
        let i = match self.history.iter().position(|h| h.name().eq_ignore_ascii_case(script)) {
            Some(i) => i,
            None => {
                self.history.push(ReadHistory::new(script));
                self.history.len() - 1
            }
        };
        &mut self.history[i]
    }
    pub fn read(&self, script: &[u8], id: u32) -> bool {
        self.history.iter().any(|h| h.name().eq_ignore_ascii_case(script) && h.get(id))
    }
}

/// CGLIST.MSD / SCENELIST.MSD name tables (`ReadCGList` 0x417B60,
/// `ReadSceneList` 0x417D90) for flag lookup by image or script name.
#[derive(Clone, Debug, Default)]
pub struct Lists {
    /// (page, entry within page, variant names).
    pub cg: Vec<(usize, usize, Vec<String>)>,
    /// (list, entry, script name).
    pub scenes: Vec<(usize, usize, String)>,
}
impl Lists {
    /// Byte offset in the CG flag table for an image name (extension cut).
    pub fn cg_index(&self, name: &str) -> Option<usize> {
        let stem = name.split('.').next().unwrap_or(name);
        self.cg.iter().find_map(|(page, entry, names)| {
            names.iter().position(|n| n.eq_ignore_ascii_case(stem)).map(|v| 2000 * page + 20 * entry + v)
        })
    }
    pub fn scene_index(&self, name: &str) -> Option<usize> {
        let stem = name.split('.').next().unwrap_or(name);
        self.scenes.iter().find(|(_, _, n)| n.eq_ignore_ascii_case(stem)).map(|(list, entry, _)| 20 * list + entry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn config_round_trips_and_grows_history_like_native() {
        let mut s = System::new([7; 32]);
        s.globals[1] = 5;
        let h = s.history_mut(b"S001");
        h.set(3).unwrap();
        h.set(8191).unwrap();
        assert_eq!(h.bits.len(), 1024);
        h.set(8192).unwrap();
        // need = 1025 → ((1025/1024+1)<<10) + 1024 = 3072 (SetupKidoku).
        assert_eq!(h.bits.len(), 3072);
        let bytes = s.encode().unwrap();
        let back = System::decode(&bytes).unwrap();
        assert_eq!(back, s);
        assert!(back.read(b"s001", 3) && !back.read(b"S001", 4));
    }
}

/// `Novel.ini` [ConfigEx] / [charNNN] (`CSaveData` +219144, 0x510 bytes;
/// reader 0x417F70, writer 0x4181A0). Per character: voice on, ARGB text
/// colour (SETCHARID 0x4122A0 applies it; MESVPLAY 0x411F00 mutes a voice
/// whose character is off).
#[derive(Clone, Debug, PartialEq)]
pub struct ConfigEx {
    pub none_effect: bool,
    pub frame_speed: i32,
    pub alt_and_enter: bool,
    pub system_se: bool,
    pub none_pause: bool,
    pub bgm_small_volume: bool,
    pub chars: Vec<(bool, [u8; 4])>,
}
impl Default for ConfigEx {
    fn default() -> Self {
        Self { none_effect: false, frame_speed: 1, alt_and_enter: false, system_se: true, none_pause: false, bgm_small_volume: false, chars: vec![(true, [255; 4]); 256] }
    }
}
impl ConfigEx {
    /// Missing keys take the reader's defaults; FrameSpeed is raised to 1.
    pub fn parse_ini(text: &str) -> Self {
        let mut out = Self::default();
        let mut section = String::new();
        for line in text.lines() {
            let line = line.trim();
            if let Some(s) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
                section = s.to_ascii_lowercase();
                continue;
            }
            let Some((k, v)) = line.split_once('=') else { continue };
            let (k, v) = (k.trim().to_ascii_lowercase(), v.trim());
            let int = |v: &str| v.split(|c: char| !c.is_ascii_digit() && c != '-').next().and_then(|n| n.parse::<i32>().ok()).unwrap_or(0);
            if section == "configex" {
                match k.as_str() {
                    "noneeffect" => out.none_effect = int(v) != 0,
                    "framespeed" => out.frame_speed = int(v).max(1),
                    "altandenter" => out.alt_and_enter = int(v) != 0,
                    "systemse" => out.system_se = int(v) != 0,
                    "nonepause" => out.none_pause = int(v) != 0,
                    "bgmsmallvolume" => out.bgm_small_volume = int(v) != 0,
                    _ => {}
                }
            } else if let Some(n) = section.strip_prefix("char").and_then(|n| n.parse::<usize>().ok()).filter(|n| *n < 256) {
                match k.as_str() {
                    "voice" => out.chars[n].0 = int(v) != 0,
                    "color" => {
                        let p: Vec<u8> = v.split(',').map(|x| x.trim().parse::<i32>().unwrap_or(255).clamp(0, 255) as u8).collect();
                        if p.len() == 4 {
                            out.chars[n].1 = [p[0], p[1], p[2], p[3]];
                        }
                    }
                    _ => {}
                }
            }
        }
        out
    }
    /// The writer's keys for the first `count` characters.
    pub fn to_ini(&self, count: usize) -> String {
        let b = |v: bool| u8::from(v);
        let mut s = format!(
            "[ConfigEx]\r\nNoneEffect={}\r\nFrameSpeed={}\r\nAltAndEnter={}\r\nSystemSE={}\r\nNonePause={}\r\nBGMSmallVolume={}\r\n",
            b(self.none_effect), self.frame_speed, b(self.alt_and_enter), b(self.system_se), b(self.none_pause), b(self.bgm_small_volume)
        );
        for (i, (voice, c)) in self.chars.iter().take(count).enumerate() {
            s += &format!("[char{i:03}]\r\nvoice={}\r\ncolor={},{},{},{}\r\n", b(*voice), c[0], c[1], c[2], c[3]);
        }
        s
    }
}
