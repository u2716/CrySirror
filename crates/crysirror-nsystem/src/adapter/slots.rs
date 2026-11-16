// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! Original slots: the `LPT` snapshot (`CDIBSnrScreen::Save` 0x410DA0 via
//! `MoveCmdLPT` 0x411480), the slot ↔ VM mapping, and slot I/O on the save
//! capability (`GAMEnn.SAV`, quick save 999).
//!
//! Load resumes at the snapshot's load point (`CDIBSnrScreen::Load`
//! 0x410F40 → `JumpToLoadPoint(0, n)`) with the slot's variables, image
//! slots, BGM/SE and both layer pages. A slot is decoded and every resource
//! it names is checked before the live game is replaced.
use super::*;
use crate::save::{self, ActionRecord, LayerRecord, Slot, Sound, Thumbnail};
use crysirror_core::image_processing::{ImageInput, ImageJob, ImageProcessState, PackedImageSpec, PackedPixelLayout};

pub(super) const QUICK_SLOT: u32 = 999;
/// `CreateSaveSum` 0x418530 thumbnail size (the slot sample).
const THUMB: [u32; 2] = [160, 120];

pub(super) fn slot_path(n: u32) -> Result<VfsPath> {
    Ok(VfsPath::new(Mount::Save, &format!("GAME{n:02}.SAV"))?)
}
/// One slot file as the save/load list shows it.
#[derive(Clone)]
pub(super) struct SlotFile {
    pub slot: Option<Arc<Slot>>,
    pub modified_ms: Option<i64>,
}
pub(super) fn read_slot(storage: &Arc<dyn ClientStorage>, n: u32, title: &[u8]) -> Result<SlotFile> {
    let path = slot_path(n)?;
    let Some(meta) = storage.metadata(Mount::Save, path.relative())? else {
        return Ok(SlotFile { slot: None, modified_ms: None });
    };
    ensure!(!meta.is_directory && meta.size.is_none_or(|s| s <= save::SLOT_LIMIT), "N-System slot {n} size/type");
    let reader = storage.open_read(&path)?;
    let bytes = crysirror_resource::read_all(reader.as_ref(), save::SLOT_LIMIT)?;
    let slot = Slot::decode(&bytes, title).with_context(|| format!("N-System slot GAME{n:02}.SAV"))?;
    Ok(SlotFile { slot: Some(Arc::new(slot)), modified_ms: meta.modified_ms })
}

/// Native state outside the VM's own model, kept from the last loaded slot
/// so that saving does not invent it (camera records, reviewed-but-unused
/// fields).
#[derive(Clone, Debug)]
pub(super) struct NativeExtra {
    pub vvram1: i32,
    pub cameras: [Vec<u8>; 2],
    pub screen_state: [u8; 8],
    pub message_1072: [u8; 4],
    pub message_1088: u8,
    pub flags: Vec<u8>,
    pub bgm_list: Vec<Vec<u8>>,
    pub reserved: Vec<u8>,
}
impl Default for NativeExtra {
    fn default() -> Self {
        Self {
            vvram1: 0,
            cameras: [save::default_camera(), save::default_camera()],
            screen_state: [0; 8],
            message_1072: [0; 4],
            message_1088: 0,
            flags: vec![0; 256],
            bgm_list: vec![Vec::new(); 256],
            reserved: vec![0; 0x4000],
        }
    }
}
/// Saving in progress: thumbnail capture/resize, then the atomic write.
pub(super) enum SaveJob {
    Capture { slot: u32, job: ImageJob },
    Resize { slot: u32, job: ImageJob },
    Write(Task<()>),
}

fn argb([r, g, b, a]: [u8; 4]) -> u32 {
    u32::from(a) << 24 | u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b)
}
fn rgba(v: u32) -> [u8; 4] {
    [(v >> 16) as u8, (v >> 8) as u8, v as u8, (v >> 24) as u8]
}

impl NSystemVm {
    fn layer_record(&self, l: &Layer) -> Result<LayerRecord> {
        Ok(LayerRecord {
            image: l.image.as_deref().map(cp932).unwrap_or_default(),
            pos: l.pos,
            color: l.color.map_or(u32::MAX, argb),
            blend: u8::from(l.blend),
            anti: u8::from(l.anti),
            action: l.action,
            ..Default::default()
        })
    }
    fn layer_from(&self, r: &LayerRecord) -> Result<Layer> {
        ensure!(r.movie.is_none(), "N-System slot movie layer (CDIBMovie) is not mapped");
        ensure!(r.size == [0, 0] && r.scale == [1.0, 1.0] && r.b48 == [1, 0], "N-System slot layer size/camera state is not mapped");
        let image = (!r.image.is_empty()).then(|| utf8(&r.image));
        if let Some(name) = &image {
            self.asset("mgd", name, "mgd")?;
        }
        Ok(Layer {
            image,
            pos: r.pos,
            blend: r.blend != 0,
            anti: r.anti != 0,
            action: r.action,
            color: (r.color != u32::MAX).then(|| rgba(r.color)),
        })
    }
    /// `MoveCmdLPT` 0x411480: the screen as a slot will restore it.
    pub(super) fn snapshot(&self, load_point: i32, pc: u32) -> Result<save::Screen> {
        let mut actions = vec![ActionRecord::default(); 256];
        for (i, a) in &self.actions {
            let i = usize::try_from(*i).ok().filter(|i| *i < 256).context("N-System action index")?;
            ensure!(a.points.len() <= 16, "N-System scroll points");
            let mut p = vec![0u8; 208];
            p[0..4].copy_from_slice(&2i32.to_le_bytes());
            p[4..8].copy_from_slice(&(a.duration as i32).to_le_bytes());
            p[8] = u8::from(a.looping);
            p[12..16].copy_from_slice(&(a.points.len() as i32).to_le_bytes());
            for (k, pt) in a.points.iter().enumerate() {
                for (axis, base) in [16usize, 80, 144].iter().enumerate() {
                    p[base + 4 * k..base + 4 * k + 4].copy_from_slice(&pt[axis].to_le_bytes());
                }
            }
            actions[i] = ActionRecord { active: u8::from(a.playing), playing: u8::from(a.playing), kind: 2, flags: [0; 3], params: p, elapsed: Some(a.elapsed as u32) };
        }
        let mut layers = Vec::with_capacity(32);
        for l in self.shown.iter().chain(&self.staged) {
            layers.push(self.layer_record(l)?);
        }
        let script = self.script.as_ref().context("N-System LPT without a script")?;
        let area = self.window.area.context("N-System message area")?;
        let mut m = Vec::with_capacity(44);
        m.push(u8::from(area.kinsoku));
        m.push(u8::from(self.window.down));
        for v in [area.left, area.top, area.right, area.bottom, area.line_pitch, area.pitch] {
            m.extend(v.to_le_bytes());
        }
        m.extend(self.extra.message_1072);
        m.extend(area.size.to_le_bytes());
        m.extend(argb(self.window.colors[0]).to_le_bytes());
        m.extend(argb(self.window.colors[1]).to_le_bytes());
        m.push(self.extra.message_1088);
        m.push(u8::from(self.window.shadow));
        Ok(save::Screen {
            vvram: [0, self.extra.vvram1],
            actions,
            layers,
            cameras: self.extra.cameras.clone(),
            script: cp932(&self.native_script_name(&script.name)),
            pc,
            stack: Vec::new(),
            load_point,
            screen_state: self.extra.screen_state,
            message: m.try_into().map_err(|_| anyhow::anyhow!("N-System message record"))?,
        })
    }
    /// The MSD file stem as packed (`S001`), which native stores.
    pub(super) fn native_script_name(&self, key: &str) -> String {
        self.config
            .assets
            .get(&format!("msd/{key}.msd"))
            .and_then(|p| p.rsplit('/').next())
            .map(|f| f.trim_end_matches(".MSD").trim_end_matches(".msd").to_owned())
            .unwrap_or_else(|| key.to_ascii_uppercase())
    }
    /// `SaveGame` 0x418A30: the live system block plus the LPT snapshot.
    pub(super) fn build_slot(&self, thumbnail: Option<Thumbnail>) -> Result<Slot> {
        let screen = self.snapshot_at.clone().context("N-System save before the first LPT save point")?;
        let names = |m: &BTreeMap<i32, String>| -> Result<Vec<Vec<u8>>> {
            let mut out = vec![Vec::new(); 256];
            for (i, n) in m {
                *out.get_mut(usize::try_from(*i).ok().context("N-System slot index")?).context("N-System slot index range")? = cp932(n);
            }
            Ok(out)
        };
        ensure!(self.strings[256..].iter().all(Vec::is_empty), "N-System string variables beyond 255 have no slot field");
        let se = names(&self.se)?
            .into_iter()
            .enumerate()
            .map(|(i, name)| Sound { name, playing: self.se_loops.contains(&(i as i32)), volume: u8::from(self.se_loops.contains(&(i as i32))) })
            .collect();
        Ok(Slot {
            subtitle: self.subtitle.clone(),
            thumbnail,
            vars: self.vars[..1000].to_vec(),
            flags: self.extra.flags.clone(),
            strings: self.strings[..256].to_vec(),
            images: names(&self.slots)?,
            bgm_list: self.extra.bgm_list.clone(),
            bgm: match &self.bgm {
                Some((name, flag)) => Sound { name: cp932(name), playing: true, volume: *flag },
                None => Sound { name: Vec::new(), playing: false, volume: 0 },
            },
            se,
            reserved: self.extra.reserved.clone(),
            screen,
        })
    }
    /// `LoadGame` 0x419070 + `CDIBSnrScreen::Load` 0x410F40.
    pub(super) fn apply_slot(&mut self, h: &mut dyn HostServices, slot: &Slot) -> Result<()> {
        let s = &slot.screen;
        // Validate everything before touching the live game.
        let key = utf8(&s.script).to_ascii_lowercase();
        let script = self.scripts.get(&key).cloned().with_context(|| format!("N-System slot script {key}"))?;
        ensure!(s.stack.is_empty(), "N-System slot call stack (depth {}) is not mapped", s.stack.len());
        let lp = usize::try_from(s.load_point).ok().context("N-System slot load point")?;
        let offset = *script.load_points.first().and_then(|t| t.get(lp)).context("N-System slot load point outside the script")?;
        let ip = script.at(offset)?;
        ensure!(s.vvram[0] == 0 || s.vvram[0] == 1, "N-System slot front page");
        let mut pages = Vec::with_capacity(32);
        for r in &s.layers {
            pages.push(self.layer_from(r)?);
        }
        let (front, back) = if s.vvram[0] == 0 { (pages[..16].to_vec(), pages[16..].to_vec()) } else { (pages[16..].to_vec(), pages[..16].to_vec()) };
        let mut actions = BTreeMap::new();
        for (i, a) in s.actions.iter().enumerate() {
            match a.kind {
                0 => {}
                2 => {
                    let p = &a.params;
                    let i32_at = |o: usize| i32::from_le_bytes(p[o..o + 4].try_into().unwrap());
                    ensure!(p[9] == 0, "N-System slot spline scroll is not mapped");
                    let count = usize::try_from(i32_at(12)).ok().filter(|c| *c <= 16).context("N-System slot scroll points")?;
                    let points = (0..count).map(|k| [i32_at(16 + 4 * k), i32_at(80 + 4 * k), i32_at(144 + 4 * k)]).collect();
                    actions.insert(i as i32, Action { duration: i32_at(4).max(1) as u64, looping: p[8] != 0, points, elapsed: u64::from(a.elapsed.unwrap_or(0)), playing: a.playing != 0 });
                }
                k => bail!("N-System slot action family {k} is not mapped"),
            }
        }
        let mut images = BTreeMap::new();
        for (i, n) in slot.images.iter().enumerate().filter(|(_, n)| !n.is_empty()) {
            let name = utf8(n);
            self.asset("mgd", &name, "mgd")?;
            images.insert(i as i32, name);
        }
        let mut se = BTreeMap::new();
        for (i, s) in slot.se.iter().enumerate().filter(|(_, s)| !s.name.is_empty()) {
            let name = utf8(&s.name);
            self.asset("se", &name, "ogg")?;
            se.insert(i as i32, (name, s.playing));
        }
        let bgm = (!slot.bgm.name.is_empty() && slot.bgm.playing).then(|| (utf8(&slot.bgm.name), slot.bgm.volume));
        if let Some((name, _)) = &bgm {
            self.asset("bgm", name, "ogg")?;
        }
        let m = &s.message;
        let i32_at = |o: usize| i32::from_le_bytes(m[o..o + 4].try_into().unwrap());
        self.window_layout()?;
        let mut area = self.window.area.context("N-System message area")?;
        area.kinsoku = m[0] != 0;
        [area.left, area.top, area.right, area.bottom, area.line_pitch, area.pitch] = [i32_at(2), i32_at(6), i32_at(10), i32_at(14), i32_at(18), i32_at(22)];
        area.size = i32_at(30);
        ensure!(area.size > 0 && area.right > area.left && area.bottom > area.top, "N-System slot message area");

        // Commit.
        for channel in [BGM_CHANNEL, VOICE_CHANNEL].into_iter().chain((0..64).map(|s| SE_CHANNEL + s)) {
            h.submit_audio(self.client, AudioRequest::Stop { channel, fade_ms: 0.0 })?;
        }
        self.clear_scene(h)?;
        self.vars[..1000].copy_from_slice(&slot.vars);
        for (dst, src) in self.strings.iter_mut().zip(&slot.strings) {
            dst.clone_from(src);
        }
        self.slots = images;
        self.se = se.iter().map(|(i, (n, _))| (*i, n.clone())).collect();
        self.se_loops = se.iter().filter(|(_, (_, p))| *p).map(|(i, _)| *i).collect();
        for i in self.se_loops.clone() {
            let path = self.asset("se", &self.se[&i], "ogg")?;
            self.play(h, SE_CHANNEL + i as u32, path, true)?;
        }
        self.bgm = bgm;
        if let Some((name, _)) = self.bgm.clone() {
            let path = self.asset("bgm", &name, "ogg")?;
            self.play(h, BGM_CHANNEL, path, true)?;
        }
        self.shown = front;
        self.staged = back;
        self.actions = actions;
        self.window.area = Some(area);
        self.window.down = m[1] != 0;
        self.window.colors = [rgba(u32::from_le_bytes(m[34..38].try_into()?)), rgba(u32::from_le_bytes(m[38..42].try_into()?))];
        self.window.shadow = m[43] != 0;
        self.window.name = -1;
        self.subtitle.clone_from(&slot.subtitle);
        self.extra = NativeExtra {
            vvram1: s.vvram[1],
            cameras: s.cameras.clone(),
            screen_state: s.screen_state,
            message_1072: m[26..30].try_into()?,
            message_1088: m[42],
            flags: slot.flags.clone(),
            bgm_list: slot.bgm_list.clone(),
            reserved: slot.reserved.clone(),
        };
        self.snapshot_at = Some(s.clone());
        self.script = Some(script);
        self.ip = ip;
        self.phase = 0;
        self.wait = Wait::None;
        self.timer = None;
        self.clicked = false;
        let mut cmds = Vec::new();
        for i in 0..LAYERS {
            cmds.push(self.layer_put(h, i, &self.shown[i].clone())?);
        }
        self.submit(h, cmds, vec![])?;
        Ok(())
    }
    /// Window Save/QSave: thumbnail of the scene below the window
    /// (`CreateSaveSum` mode 0), then the slot write.
    pub(super) fn begin_save(&mut self, h: &mut dyn HostServices, slot: u32) -> Result<()> {
        ensure!(self.save_job.is_none(), "N-System save already in progress");
        ensure!(self.snapshot_at.is_some(), "N-System save before the first LPT save point");
        let job = h.begin_layer_prefix_image_over(self.client, self.config.canvas, [5000, 0], [0, 0, 0, 255])?;
        self.save_job = Some(SaveJob::Capture { slot, job });
        Ok(())
    }
    /// Poll the save pipeline; true when idle.
    pub(super) fn poll_save(&mut self, h: &mut dyn HostServices) -> Result<bool> {
        let Some(job) = self.save_job.take() else { return Ok(true) };
        self.save_job = Some(match job {
            SaveJob::Capture { slot, mut job } => match job.poll()? {
                ImageProcessState::Pending => SaveJob::Capture { slot, job },
                ImageProcessState::Failed(e) => bail!("N-System save thumbnail capture: {e}"),
                ImageProcessState::Ready(image) => {
                    let effects = serde_json::to_vec(&vec![crysirror_image::effects::ImageEffect::Resize { width: THUMB[0], height: THUMB[1] }])?;
                    SaveJob::Resize { slot, job: h.image_metadata(self.client)?.begin_process(ImageInput::Resource(image), Arc::from(effects))? }
                }
            },
            SaveJob::Resize { slot, mut job } => match job.poll()? {
                ImageProcessState::Pending => SaveJob::Resize { slot, job },
                ImageProcessState::Failed(e) => bail!("N-System save thumbnail resize: {e}"),
                ImageProcessState::Ready(image) => {
                    ensure!(image.size() == THUMB, "N-System thumbnail size");
                    let rgba = image.rgba();
                    let mut bgra = Vec::with_capacity(rgba.len());
                    for p in rgba.chunks_exact(4) {
                        bgra.extend([p[2], p[1], p[0], 255]);
                    }
                    let bytes = self.build_slot(Some(Thumbnail { width: THUMB[0], height: THUMB[1], bgra }))?.encode(&self.title_cp932)?;
                    let storage = h.storage(self.client)?;
                    let path = slot_path(slot)?;
                    let pool = self.pool.as_ref().context("N-System task pool")?;
                    SaveJob::Write(pool.spawn(move |cancel| {
                        cancel.check()?;
                        storage.write_atomic(&path, &bytes)?;
                        Ok(())
                    })?)
                }
            },
            SaveJob::Write(mut task) => match task.try_take()? {
                None => SaveJob::Write(task),
                Some(()) => return Ok(true),
            },
        });
        Ok(false)
    }
    /// Publish a slot thumbnail for the list.
    pub(super) fn begin_thumbnail(&mut self, h: &mut dyn HostServices, t: &Thumbnail) -> Result<ImageJob> {
        Ok(h.image_metadata(self.client)?.begin_packed_pixels(PackedImageSpec {
            size: [t.width, t.height],
            layout: PackedPixelLayout::Bgra8,
            bytes: t.bgra.clone().into(),
        })?)
    }
}
