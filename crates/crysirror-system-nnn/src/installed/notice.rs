// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Native CNotice queue, sheet selection and three-stage animation policy.
//! Pixels, resources and presentation remain ordinary shared Image/Layer calls.
use super::*;
use crysirror_core::image_processing::ImageResource;
use std::collections::VecDeque;
// Notice is drawn after the native scene, text and last-screen effect.
// Keep it outside actor_frame's exclusive [MAX-1, 0] capture prefix.
const LAYER: u32 = 0x66010000;
#[derive(Clone)]
struct Entry {
    point: [[i32; 2]; 3],
    time: [u32; 3],
    percent: [i32; 3],
    sound: [i32; 3],
}
struct Sheet {
    effects: Arc<[u8]>,
    input: Option<ImageInput>,
    job: Option<ImageJob>,
    ready: Option<Arc<dyn ImageResource>>,
}
pub(super) struct Notice {
    entries: Vec<Entry>,
    sheets: Vec<Sheet>,
    size: [u32; 2],
    blocks: [u32; 2],
    capacity: usize,
    queue: VecDeque<usize>,
    phase: usize,
    count: u32,
    frame_ms: u64,
    at: Option<u64>,
    root: u32,
    client: ClientId,
    path: Option<String>,
    present: bool,
    sheet_index: Option<usize>,
    sound: Option<i32>,
}
impl Notice {
    pub fn prepare(
        config: &crate::native_bootstrap::NativeConfiguration,
        storage: &dyn ClientStorage,
        resources: &NativeResources,
        client: ClientId,
        cancel: &Cancellation,
    ) -> Result<Option<Self>> {
        if config.setup.integer(b"noticeFlag")?.unwrap_or(0) == 0 {
            return Ok(None);
        }
        let setup = native_load::table(config, storage, "notice", cancel)?;
        let n = |key: &str, default| -> Result<i32> {
            Ok(setup.integer(key.as_bytes())?.unwrap_or(default))
        };
        let count = usize::try_from(n("noticeNumber", 1)?)?;
        ensure!((1..=256).contains(&count), "NNN notice entry budget");
        let size = [
            u32::try_from(n("sizeX", 160)?)?,
            u32::try_from(n("sizeY", 80)?)?,
        ];
        let blocks = [
            u32::try_from(n("blockX", 1)?)?,
            u32::try_from(n("blockY", 1)?)?,
        ];
        ensure!(
            size.iter().all(|v| (1..=8192).contains(v))
                && blocks.iter().all(|v| (1..=256).contains(v)),
            "NNN notice sheet geometry"
        );
        let mut entries: Vec<Entry> = Vec::new();
        for i in 0..count {
            let prior = entries.last().cloned().unwrap_or(Entry {
                point: [[0, 0]; 3],
                time: [30; 3],
                percent: [100; 3],
                sound: [0; 3],
            });
            let mut entry = prior;
            for (stage, tag) in ["start", "wait", "end"].iter().enumerate() {
                let inherited = if i == 0 && stage > 0 {
                    entry.point[stage - 1]
                } else {
                    entry.point[stage]
                };
                entry.point[stage] = [
                    n(&format!("{tag}X{}", i + 1), inherited[0])?,
                    n(&format!("{tag}Y{}", i + 1), inherited[1])?,
                ];
                let t = if i == 0 && stage > 0 {
                    entry.time[stage - 1]
                } else {
                    entry.time[stage]
                };
                // Original Print divides by zero for nonpositive stage times.
                // Define a one-frame stage instead of depending on native UB.
                entry.time[stage] = n(&format!("{tag}Time{}", i + 1), t as i32)?
                    .max(1)
                    .try_into()?;
                let p = if i == 0 && stage > 0 {
                    entry.percent[stage - 1]
                } else {
                    entry.percent[stage]
                };
                entry.percent[stage] = n(&format!("{tag}Percent{}", i + 1), p)?;
                entry.sound[stage] = n(&format!("{tag}Sound{}", i + 1), entry.sound[stage])?;
            }
            entries.push(entry);
        }
        let tag = setup
            .parameter(b"filenameNoticeTag")
            .unwrap_or(b"ta_notice");
        let per_sheet = blocks[0] as usize * blocks[1] as usize;
        let levels = u32::try_from(config.setup.integer(b"dwqPackLevel")?.unwrap_or(0))?;
        let mut sheets = Vec::new();
        for i in 0..count.div_ceil(per_sheet) {
            let mut name = b"sys/".to_vec();
            name.extend(tag);
            if i != 0 {
                name.extend((i + 1).to_string().bytes());
            }
            let decoded =
                resources.image_source(&config.resources.picture(&name, levels)?, cancel)?;
            let effects = if decoded.has_mask {
                vec![]
            } else {
                vec![crysirror_image::effects::ImageEffect::SpriteAlpha {
                    spec: crysirror_image::sprite_alpha::SpriteAlpha::CornerKey { right: false },
                }]
            };
            sheets.push(Sheet {
                effects: serde_json::to_vec(&effects)?.into(),
                input: Some(decoded.input),
                job: None,
                ready: None,
            });
        }
        Ok(Some(Self {
            entries,
            sheets,
            size,
            blocks,
            capacity: usize::try_from(n("bufferMax", 1)?.clamp(1, 256))?,
            queue: VecDeque::new(),
            phase: 0,
            count: 0,
            frame_ms: u64::try_from(config.setup.integer(b"frameTime")?.unwrap_or(50).max(1))?,
            at: None,
            root: 1,
            client,
            path: None,
            present: false,
            sheet_index: None,
            sound: None,
        }))
    }
    pub fn copy_ready(&self) -> Result<Self> {
        ensure!(
            self.sheets.iter().all(|s| s.ready.is_some()),
            "NNN notice restore before image preparation"
        );
        Ok(Self {
            entries: self.entries.clone(),
            sheets: self
                .sheets
                .iter()
                .map(|s| Sheet {
                    effects: s.effects.clone(),
                    input: None,
                    job: None,
                    ready: s.ready.clone(),
                })
                .collect(),
            size: self.size,
            blocks: self.blocks,
            capacity: self.capacity,
            queue: self.queue.clone(),
            phase: self.phase,
            count: self.count,
            frame_ms: self.frame_ms,
            at: self.at,
            root: self.root,
            client: self.client,
            path: self.path.clone(),
            present: self.present,
            sheet_index: self.sheet_index,
            sound: self.sound,
        })
    }
    pub fn prepare_images(&mut self, host: &mut dyn HostServices) -> Result<bool> {
        for sheet in &mut self.sheets {
            if sheet.ready.is_some() {
                continue;
            }
            if let Some(input) = sheet.input.take() {
                sheet.job = Some(
                    host.image_metadata(self.client)?
                        .begin_process(input, sheet.effects.clone())?,
                );
            }
            match sheet
                .job
                .as_mut()
                .context("NNN notice image job absent")?
                .poll()?
            {
                // Admit one sheet job at a time; a large native notice table
                // must not exhaust the shared asynchronous image-job budget.
                ImageProcessState::Pending => return Ok(false),
                ImageProcessState::Failed(e) => anyhow::bail!("NNN notice image: {e}"),
                ImageProcessState::Ready(image) => {
                    ensure!(
                        image.size()[0] >= self.size[0] * self.blocks[0]
                            && image.size()[1] >= self.size[1] * self.blocks[1],
                        "NNN notice sheet too small"
                    );
                    sheet.ready = Some(image);
                    sheet.job = None;
                }
            }
        }
        Ok(true)
    }
    pub fn start(&mut self, number: i32) {
        let Ok(number) = usize::try_from(number) else {
            return;
        };
        // Legacy CNotice::Start (MUGEN2 0x44e140) enqueues duplicates.
        // sameDontSetFlag belongs to a later source profile, not this runtime.
        if number >= self.entries.len() {
            return;
        }
        let reset = self.queue.is_empty() || self.queue.len() >= self.capacity;
        if self.queue.len() >= self.capacity {
            self.queue.pop_front();
        }
        self.queue.push_back(number);
        if reset {
            self.phase = 0;
            self.count = 0;
            self.at = None;
        }
    }
    pub fn poll(&mut self, host: &mut dyn HostServices) -> Result<bool> {
        if !self.prepare_images(host)? {
            return Ok(false);
        }
        let Some(&number) = self.queue.front() else {
            return Ok(true);
        };
        let now = host.now_ms();
        if self
            .at
            .is_some_and(|at| now.saturating_sub(at) < self.frame_ms)
        {
            return Ok(true);
        }
        self.at = Some(now);
        self.count = self.count.saturating_add(1);
        let e = &self.entries[number];
        if self.count == 1 && self.phase == 0 {
            // The native main scene is repainted before Print's first-frame
            // early return. Retained Layers must explicitly retire the previous
            // notice when a full queue replaces its active head.
            if self.present {
                host.submit_engine(
                    self.client,
                    1,
                    &serde_json::to_vec(
                        &serde_json::json!({"layers":[{"op":"remove","id":LAYER}]}),
                    )?,
                )?;
                self.present = false;
            }
            self.sound = Some(e.sound[0]);
            return Ok(true);
        }
        let from = if self.phase == 0 { 0 } else { 1 };
        let to = if self.phase == 2 { 2 } else { 1 };
        let duration = e.time[self.phase];
        let count = self.count.min(duration);
        let lerp = |a: i32, b: i32| -> Result<i32> {
            Ok(i32::try_from(
                i64::from(a)
                    + (i64::from(b) - i64::from(a)) * i64::from(count) / i64::from(duration),
            )?)
        };
        let point = [
            lerp(e.point[from][0], e.point[to][0])?,
            lerp(e.point[from][1], e.point[to][1])?,
        ];
        let opacity = lerp(e.percent[from], e.percent[to])?.clamp(0, 100) as u32 * 255 / 100;
        if self.count >= duration {
            self.count = 0;
            self.phase += 1;
            if self.phase > 2 {
                self.queue.pop_front();
                self.phase = 0;
                if self.present {
                    host.submit_engine(
                        self.client,
                        1,
                        &serde_json::to_vec(
                            &serde_json::json!({"layers":[{"op":"remove","id":LAYER}]}),
                        )?,
                    )?;
                    self.present = false;
                }
                if let Some(path) = self.path.take() {
                    host.release_image_resource(self.client, &path)?;
                }
                self.sheet_index = None;
                return Ok(true);
            }
            self.sound = Some(e.sound[self.phase]);
        }
        let per = self.blocks[0] as usize * self.blocks[1] as usize;
        let image = self.sheets[number / per].ready.as_ref().unwrap().clone();
        if self.sheet_index != Some(number / per) {
            let path = host
                .publish_image_resource(self.client, LAYER, image.clone())?
                .path
                .relative()
                .to_owned();
            if let Some(old) = self.path.replace(path) {
                host.release_image_resource(self.client, &old)?;
            }
            self.sheet_index = Some(number / per);
        }
        let path = self.path.clone().context("NNN notice publication absent")?;
        let x = (number as u32 % self.blocks[0]) * self.size[0];
        let y = (number as u32 / self.blocks[0] % self.blocks[1]) * self.size[1];
        host.submit_engine(self.client,1,&serde_json::to_vec(&serde_json::json!({"layers":[{"op":"put","layer":{
            "id":LAYER,"parent":self.root,"rect":[point[0],point[1],self.size[0],self.size[1]],"order":[2147483647,0],"opacity":opacity,
            "uv":[x as f32/image.size()[0] as f32,y as f32/image.size()[1] as f32,(x+self.size[0]) as f32/image.size()[0] as f32,(y+self.size[1]) as f32/image.size()[1] as f32],
            "paint":{"kind":"image","asset":path}}}]}))?)?;
        self.path = Some(path);
        self.present = true;
        Ok(true)
    }
    pub fn pending_sound(&self) -> Option<i32> {
        self.sound.filter(|n| *n > 0).map(|n| n - 1)
    }
    pub fn sound_done(&mut self) {
        self.sound = None;
    }
}
