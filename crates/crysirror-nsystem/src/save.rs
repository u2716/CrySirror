// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! Original N-System slot files `SAVE\GAMEnn.SAV` (HANANYU.EXE
//! `CSaveData::SaveGame` 0x418A30 / `LoadGame` 0x419070).
//!
//! - 32 bytes: lower-case hex MD5 of `<window title>SaveData` (CP932).
//! - Plain block, u32 length (of the block, counting this field) first:
//!   subtitle (64), thumbnail (u32 bytes, u32 w, u32 h, top-down BGRA),
//!   variables 0..999 (i32 ×1000), 256 bytes (+106484, preserved), string
//!   variables (256 × 64), image slots (256 × 64 names), BGMLIST names
//!   (256 × 64), BGM (name 64, playing u8, volume u8), SE slots (256 × name
//!   32, playing u8, volume u8), 0x4000 bytes (+42924, preserved).
//! - Screen blob: u32 length, then bytes XORed in 32-byte blocks with the hex
//!   MD5 of `<title><n>` (the MSD key). It is the snapshot the last `LPT`
//!   took (`CDIBSnrScreen::Save` 0x410DA0): `CDIBSnrVVram::Save` 0x43B050
//!   (2 × i32, 256 actions, 2 × 16 layers, 2 cameras), `CSnrBuffer::Save`
//!   0x4426C0 (script name 64, PC, call stack), the load point, 8 bytes of
//!   screen state and the message buffer fields.
//!
//! Variables 1000.. are global (CONFIG.SAV) and are not part of a slot.
use anyhow::{Context, Result, bail, ensure};

pub const SLOT_LIMIT: u64 = 4 * 1024 * 1024;
const NAME: usize = 64;

fn md5_hex(data: &[u8]) -> [u8; 32] {
    use md5::Digest;
    let digest = md5::Md5::digest(data);
    let mut out = [0u8; 32];
    for (i, b) in digest.iter().enumerate() {
        out[2 * i..2 * i + 2].copy_from_slice(format!("{b:02x}").as_bytes());
    }
    out
}
/// The `%s%d` block XOR shared with MSD scripts (self-inverse).
pub fn xor_blocks(bytes: &mut [u8], title_cp932: &[u8]) {
    for (n, chunk) in bytes.chunks_mut(32).enumerate() {
        let mut key = title_cp932.to_vec();
        key.extend(n.to_string().as_bytes());
        for (b, k) in chunk.iter_mut().zip(md5_hex(&key)) {
            *b ^= k;
        }
    }
}
fn cstr(b: &[u8]) -> Vec<u8> {
    b.iter().take_while(|c| **c != 0).copied().collect()
}
fn fixed(name: &[u8], n: usize) -> Result<Vec<u8>> {
    ensure!(name.len() < n && !name.contains(&0), "N-System save name exceeds {n} bytes");
    let mut out = name.to_vec();
    out.resize(n, 0);
    Ok(out)
}

struct Reader<'a> {
    b: &'a [u8],
    at: usize,
}
impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self.at.checked_add(n).filter(|e| *e <= self.b.len()).context("N-System save truncated")?;
        let s = &self.b[self.at..end];
        self.at = end;
        Ok(s)
    }
    fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
    fn i32(&mut self) -> Result<i32> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into()?))
    }
    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into()?))
    }
    fn f32(&mut self) -> Result<f32> {
        Ok(f32::from_le_bytes(self.take(4)?.try_into()?))
    }
    fn name(&mut self, n: usize) -> Result<Vec<u8>> {
        Ok(cstr(self.take(n)?))
    }
}
#[derive(Default)]
struct Writer(Vec<u8>);
impl Writer {
    fn bytes(&mut self, b: &[u8]) {
        self.0.extend_from_slice(b);
    }
    fn u8(&mut self, v: u8) {
        self.0.push(v);
    }
    fn i32(&mut self, v: i32) {
        self.bytes(&v.to_le_bytes());
    }
    fn u32(&mut self, v: u32) {
        self.bytes(&v.to_le_bytes());
    }
    fn f32(&mut self, v: f32) {
        self.bytes(&v.to_le_bytes());
    }
    fn name(&mut self, name: &[u8], n: usize) -> Result<()> {
        self.bytes(&fixed(name, n)?);
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Sound {
    pub name: Vec<u8>,
    pub playing: bool,
    pub volume: u8,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Thumbnail {
    pub width: u32,
    pub height: u32,
    /// Top-down BGRA.
    pub bgra: Vec<u8>,
}
/// `CDIBSnrLayer::Save` 0x43A600 (defaults: .data 0x5C88E8).
#[derive(Clone, Debug, PartialEq)]
pub struct LayerRecord {
    pub dirty: u8,
    pub image: Vec<u8>,
    /// Movie sprites only (`CDIBMovie` class): frame position, flag.
    pub movie: Option<(u32, u8)>,
    pub pos: [i32; 3],
    pub size: [i32; 2],
    /// ARGB vertex colour (0xFFFFFFFF plain).
    pub color: u32,
    pub anti: u8,
    pub blend: u8,
    pub action: i32,
    pub b48: [u8; 2],
    pub scale: [f32; 2],
}
impl Default for LayerRecord {
    fn default() -> Self {
        Self { dirty: 0, image: Vec::new(), movie: None, pos: [0; 3], size: [0; 2], color: u32::MAX, anti: 1, blend: 0, action: 0, b48: [1, 0], scale: [1.0; 2] }
    }
}
/// `CDIBSnrAction::Save` 0x430440: header bytes, the 208-byte ACTMAKE
/// parameter block and the family state (Scroll: elapsed ms).
#[derive(Clone, Debug, PartialEq)]
pub struct ActionRecord {
    /// +4: moving this frame (`Play` sets it; a stopped action clears it on its next move).
    pub active: u8,
    /// +5: `Play` 0x4306E0 / `Stop` 0x430700.
    pub playing: u8,
    pub kind: i32,
    pub flags: [u8; 3],
    pub params: Vec<u8>,
    pub elapsed: Option<u32>,
}
impl Default for ActionRecord {
    fn default() -> Self {
        Self { active: 0, playing: 0, kind: 0, flags: [0; 3], params: vec![0; 208], elapsed: None }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct Screen {
    pub vvram: [i32; 2],
    pub actions: Vec<ActionRecord>,
    /// Two pages of 16.
    pub layers: Vec<LayerRecord>,
    /// `CDIBCamera::Save` 0x41F7E0 records, preserved.
    pub cameras: [Vec<u8>; 2],
    pub script: Vec<u8>,
    pub pc: u32,
    pub stack: Vec<u32>,
    pub load_point: i32,
    pub screen_state: [u8; 8],
    /// Message buffer fields (`CDIBSnrScreen::Save` order).
    pub message: [u8; 44],
}
const CAMERA: usize = 168;
/// `CDIBCamera::Reset` 0x41F760 as `CDIBCamera::Save` 0x41F7E0 writes it.
pub fn default_camera() -> Vec<u8> {
    let mut c = vec![0u8; CAMERA];
    c[0] = 1;
    // Save order: +4, +5, then i32/f32 fields from +8; +8/+12 = 1.0,
    // +24 = -1000.0 (−flt_5C4598), +52 = 1.0.
    let field = |off: usize| 2 + (off - 8);
    for (off, v) in [(8, 1.0f32), (12, 1.0), (24, -1000.0), (52, 1.0)] {
        c[field(off)..field(off) + 4].copy_from_slice(&v.to_le_bytes());
    }
    c
}
#[derive(Clone, Debug, PartialEq)]
pub struct Slot {
    pub subtitle: Vec<u8>,
    pub thumbnail: Option<Thumbnail>,
    pub vars: Vec<i32>,
    pub flags: Vec<u8>,
    pub strings: Vec<Vec<u8>>,
    pub images: Vec<Vec<u8>>,
    pub bgm_list: Vec<Vec<u8>>,
    pub bgm: Sound,
    pub se: Vec<Sound>,
    pub reserved: Vec<u8>,
    pub screen: Screen,
}

pub fn header(title_cp932: &[u8]) -> [u8; 32] {
    let mut key = title_cp932.to_vec();
    key.extend(b"SaveData");
    md5_hex(&key)
}

fn read_screen(b: &[u8]) -> Result<Screen> {
    let mut r = Reader { b, at: 0 };
    let vvram = [r.i32()?, r.i32()?];
    let mut actions = Vec::with_capacity(256);
    for _ in 0..256 {
        let active = r.u8()?;
        let playing = r.u8()?;
        let kind = r.i32()?;
        let flags = [r.u8()?, r.u8()?, r.u8()?];
        let params = r.take(208)?.to_vec();
        let elapsed = match kind {
            0 => None,
            2 => Some(r.u32()?),
            other => bail!("N-System save action family {other} has no reviewed layout"),
        };
        actions.push(ActionRecord { active, playing, kind, flags, params, elapsed });
    }
    let mut layers = Vec::with_capacity(32);
    for _ in 0..32 {
        let dirty = r.u8()?;
        let image = r.name(NAME)?;
        // CSpriteCache::Read 0x443840 makes a CDIBMovie for ".mpg" names.
        let movie = if image.to_ascii_lowercase().ends_with(b".mpg") { Some((r.u32()?, r.u8()?)) } else { None };
        let pos = [r.i32()?, r.i32()?, r.i32()?];
        let size = [r.i32()?, r.i32()?];
        let color = r.u32()?;
        let anti = r.u8()?;
        let blend = r.u8()?;
        let action = r.i32()?;
        let b48 = [r.u8()?, r.u8()?];
        let scale = [r.f32()?, r.f32()?];
        layers.push(LayerRecord { dirty, image, movie, pos, size, color, anti, blend, action, b48, scale });
    }
    let cameras = [r.take(CAMERA)?.to_vec(), r.take(CAMERA)?.to_vec()];
    let script = r.name(NAME)?;
    let pc = r.u32()?;
    let depth = r.u32()?;
    ensure!(depth <= 256, "N-System save call stack depth");
    let mut stack = Vec::new();
    for _ in 0..depth {
        stack.push(r.u32()?);
    }
    let load_point = r.i32()?;
    let screen_state = r.take(8)?.try_into()?;
    let message = r.take(44)?.try_into()?;
    ensure!(r.at == b.len(), "N-System save screen blob has {} trailing bytes", b.len() - r.at);
    Ok(Screen { vvram, actions, layers, cameras, script, pc, stack, load_point, screen_state, message })
}
fn write_screen(s: &Screen) -> Result<Vec<u8>> {
    ensure!(s.actions.len() == 256 && s.layers.len() == 32, "N-System save screen shape");
    let mut w = Writer::default();
    w.i32(s.vvram[0]);
    w.i32(s.vvram[1]);
    for a in &s.actions {
        w.u8(a.active);
        w.u8(a.playing);
        w.i32(a.kind);
        w.bytes(&a.flags);
        ensure!(a.params.len() == 208, "N-System action parameter block");
        w.bytes(&a.params);
        match (a.kind, a.elapsed) {
            (0, None) => {}
            (2, Some(e)) => w.u32(e),
            (k, _) => bail!("N-System save action family {k} has no reviewed layout"),
        }
    }
    for l in &s.layers {
        w.u8(l.dirty);
        w.name(&l.image, NAME)?;
        ensure!(l.movie.is_some() == l.image.to_ascii_lowercase().ends_with(b".mpg"), "N-System movie layer record");
        if let Some((frame, flag)) = l.movie {
            w.u32(frame);
            w.u8(flag);
        }
        for v in l.pos.iter().chain(&l.size) {
            w.i32(*v);
        }
        w.u32(l.color);
        w.u8(l.anti);
        w.u8(l.blend);
        w.i32(l.action);
        w.bytes(&l.b48);
        w.f32(l.scale[0]);
        w.f32(l.scale[1]);
    }
    for c in &s.cameras {
        ensure!(c.len() == CAMERA, "N-System camera record");
        w.bytes(c);
    }
    w.name(&s.script, NAME)?;
    w.u32(s.pc);
    w.u32(s.stack.len() as u32);
    for v in &s.stack {
        w.u32(*v);
    }
    w.i32(s.load_point);
    w.bytes(&s.screen_state);
    w.bytes(&s.message);
    Ok(w.0)
}

impl Slot {
    pub fn decode(bytes: &[u8], title_cp932: &[u8]) -> Result<Self> {
        ensure!(bytes.len() as u64 <= SLOT_LIMIT, "N-System save size budget");
        ensure!(bytes.get(..32) == Some(&header(title_cp932)[..]), "N-System save header is not this build's (title MD5)");
        let mut r = Reader { b: bytes, at: 32 };
        let length = r.u32()? as usize;
        let block_end = 32usize.checked_add(length).filter(|e| *e <= bytes.len()).context("N-System save block length")?;
        let subtitle = r.name(NAME)?;
        let size = r.u32()? as usize;
        let (width, height) = (r.u32()?, r.u32()?);
        let thumbnail = if size == 0 {
            None
        } else {
            ensure!(width <= 1024 && height <= 1024 && size as u64 == u64::from(width) * u64::from(height) * 4, "N-System save thumbnail shape");
            Some(Thumbnail { width, height, bgra: r.take(size)?.to_vec() })
        };
        let mut vars = Vec::with_capacity(1000);
        for _ in 0..1000 {
            vars.push(r.i32()?);
        }
        let flags = r.take(256)?.to_vec();
        let names = |r: &mut Reader, n: usize| -> Result<Vec<Vec<u8>>> { (0..256).map(|_| r.name(n)).collect() };
        let strings = names(&mut r, NAME)?;
        let images = names(&mut r, NAME)?;
        let bgm_list = names(&mut r, NAME)?;
        let bgm = Sound { name: r.name(NAME)?, playing: r.u8()? != 0, volume: r.u8()? };
        let mut se = Vec::with_capacity(256);
        for _ in 0..256 {
            se.push(Sound { name: r.name(32)?, playing: r.u8()? != 0, volume: r.u8()? });
        }
        let reserved = r.take(0x4000)?.to_vec();
        ensure!(r.at == block_end, "N-System save block length mismatch");
        let blob_len = r.u32()? as usize;
        let mut blob = r.take(blob_len)?.to_vec();
        ensure!(r.at == bytes.len(), "N-System save has trailing bytes");
        xor_blocks(&mut blob, title_cp932);
        let screen = read_screen(&blob)?;
        Ok(Self { subtitle, thumbnail, vars, flags, strings, images, bgm_list, bgm, se, reserved, screen })
    }
    pub fn encode(&self, title_cp932: &[u8]) -> Result<Vec<u8>> {
        ensure!(self.vars.len() == 1000 && self.flags.len() == 256 && self.reserved.len() == 0x4000, "N-System save block shape");
        ensure!(self.strings.len() == 256 && self.images.len() == 256 && self.bgm_list.len() == 256 && self.se.len() == 256, "N-System save table shape");
        let mut w = Writer::default();
        w.bytes(&header(title_cp932));
        w.u32(0);
        w.name(&self.subtitle, NAME)?;
        match &self.thumbnail {
            Some(t) => {
                ensure!(t.bgra.len() as u64 == u64::from(t.width) * u64::from(t.height) * 4, "N-System thumbnail shape");
                w.u32(t.bgra.len() as u32);
                w.u32(t.width);
                w.u32(t.height);
                w.bytes(&t.bgra);
            }
            None => {
                w.u32(0);
                w.u32(0);
                w.u32(0);
            }
        }
        for v in &self.vars {
            w.i32(*v);
        }
        w.bytes(&self.flags);
        for table in [&self.strings, &self.images, &self.bgm_list] {
            for n in table.iter() {
                w.name(n, NAME)?;
            }
        }
        w.name(&self.bgm.name, NAME)?;
        w.u8(u8::from(self.bgm.playing));
        w.u8(self.bgm.volume);
        for s in &self.se {
            w.name(&s.name, 32)?;
            w.u8(u8::from(s.playing));
            w.u8(s.volume);
        }
        w.bytes(&self.reserved);
        let length = (w.0.len() - 32) as u32;
        w.0[32..36].copy_from_slice(&length.to_le_bytes());
        let mut blob = write_screen(&self.screen)?;
        xor_blocks(&mut blob, title_cp932);
        w.u32(blob.len() as u32);
        w.bytes(&blob);
        Ok(w.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample() -> Slot {
        let mut layers = vec![LayerRecord::default(); 32];
        layers[0].image = b"bg21a".to_vec();
        layers[10] = LayerRecord { image: b"DAMMY".to_vec(), pos: [-20, 0, 0], action: 4, ..Default::default() };
        let mut actions = vec![ActionRecord::default(); 256];
        actions[2] = ActionRecord { kind: 2, playing: 1, elapsed: Some(120), ..Default::default() };
        Slot {
            subtitle: "見出し".as_bytes().to_vec(),
            thumbnail: Some(Thumbnail { width: 2, height: 1, bgra: vec![1, 2, 3, 255, 4, 5, 6, 255] }),
            vars: (0..1000).collect(),
            flags: vec![0; 256],
            strings: vec![Vec::new(); 256],
            images: vec![Vec::new(); 256],
            bgm_list: vec![Vec::new(); 256],
            bgm: Sound { name: b"M06".to_vec(), playing: true, volume: 1 },
            se: vec![Sound { name: Vec::new(), playing: false, volume: 0 }; 256],
            reserved: vec![0; 0x4000],
            screen: Screen {
                vvram: [0, 0],
                actions,
                layers,
                cameras: [vec![0; CAMERA], vec![0; CAMERA]],
                script: b"S001".to_vec(),
                pc: 16832,
                stack: Vec::new(),
                load_point: 61,
                screen_state: [0; 8],
                message: [0; 44],
            },
        }
    }
    #[test]
    fn slot_round_trips_and_checks_the_build_header() {
        let title = b"title";
        let slot = sample();
        let bytes = slot.encode(title).unwrap();
        assert_eq!(&bytes[..32], &header(title));
        assert_eq!(Slot::decode(&bytes, title).unwrap(), slot);
        assert!(Slot::decode(&bytes, b"other").is_err());
        let mut truncated = bytes.clone();
        truncated.pop();
        assert!(Slot::decode(&truncated, title).is_err());
    }
}
