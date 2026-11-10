// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! CCommonPrintCG lifecycle over shared image jobs and retained Layers.
//! Native frame/scroll/drag policy stays here; no bitmap renderer is embedded.
use super::*;
use crate::config::Names;
use crate::native_cg::{CgAudio, CgChoice, CgEntry, CgMotion, CgProfile};
use crysirror_core::{image_processing::ImageResource, service::buttons};
use crysirror_image::{effects::ImageEffect, sprite_alpha::SpriteAlpha};
use std::{collections::VecDeque, sync::Arc};
mod draw_work;
mod difference;
use crate::native_cg_difference::{DifferenceProfile,DifferenceState,OldDifferenceImage};
use crate::image_bridge::{NativePictureViews,PicturePreparation};
const ROOT: u32 = 0x66047fff;
const LAYER: u32 = ROOT + 1;
struct Prepared {
    profile: CgProfile,
    entry: CgEntry,
    inputs: VecDeque<(crate::resource_bridge::NativeImageSource, SpriteAlpha)>,
    difference:DifferenceProfile,
    background: bool,
    voice: Option<Vec<u8>>,
}
struct Loading {
    prepared: Prepared,
    images: Vec<(NativePictureViews,SpriteAlpha)>,
    job: Option<PicturePreparation>,
    alpha:Option<SpriteAlpha>,
}
struct Live {
    profile: CgProfile,
    entry: CgEntry,
    paths: Vec<String>,
    views:Vec<NativePictureViews>,
    sizes: Vec<[u32; 2]>,
    background: bool,
    motion: CgMotion,
    ticks: u64,
    remainder: u64,
    last_ms: u64,
    drag: Option<[i32; 2]>,
    pose: Option<([i64; 2], usize)>,
}
pub(super) struct PictureViewer {
    difference:Option<DifferenceState>,
    old_difference:Option<Arc<dyn ImageResource>>,
    difference_base_job:Option<ImageJob>,
    difference_frame:Option<Arc<dyn ImageResource>>,
    difference_draw:Option<draw_work::CgDrawWork>,
    difference_pose:Option<([i64;2],usize)>,
    difference_path:Option<String>,
    difference_retired_paths:Vec<String>,
    pictures: Arc<[CgChoice]>,
    index: usize,
    group: usize,
    task: Option<Task<Prepared>>,
    loading: Option<Loading>,
    live: Option<Live>,
    config: Arc<crate::native_bootstrap::NativeConfiguration>,
    resources: NativeResources,
    storage: Arc<dyn ClientStorage>,
    pool: TaskPool,
    client: ClientId,
    audio: VecDeque<CgAudio>,
    closing: bool,
    last_voice: Option<Vec<u8>>,
    voice_remaining: Option<u64>,
}
impl PictureViewer {
    pub fn new(
        pictures: Arc<[CgChoice]>,
        index: usize,
        group: usize,
        config: Arc<crate::native_bootstrap::NativeConfiguration>,
        resources: NativeResources,
        storage: Arc<dyn ClientStorage>,
        pool: TaskPool,
        client: ClientId,
    ) -> Result<Self> {
        ensure!(
            !pictures.is_empty() && pictures.len() <= 65536 && index < pictures.len(),
            "NNN CG unlocked catalogue extent"
        );
        let mut this = Self {
            difference:None,old_difference:None,difference_base_job:None,difference_frame:None,
            difference_draw:None,difference_pose:None,difference_path:None,difference_retired_paths:Vec::new(),
            pictures,
            index,
            group,
            task: None,
            loading: None,
            live: None,
            config,
            resources,
            storage,
            pool,
            client,
            audio: VecDeque::new(),
            closing: false,
            last_voice: None,
            voice_remaining: None,
        };
        this.load()?;
        Ok(this)
    }
    fn load(&mut self) -> Result<()> {
        let name = self.pictures[self.index].raw.clone();
        let cg_index = self.pictures[self.index].index;
        let config = self.config.clone();
        let resources = self.resources.clone();
        let group = self.group;
        let storage = self.storage.clone();
        self.task = Some(self.pool.spawn(move |cancel| {
            let table = super::native_load::table(&config, storage.as_ref(), "printCG", &cancel)?;
            crate::native_general::ModeReturn::validate_entry(&table)?;
            let difference=DifferenceProfile::read(&table)?;
            let profile = CgProfile::read(&table, &config.setup, group)?;
            let entry = CgEntry::parse(&name, profile.code_byte)?;
            let levels = u32::try_from(config.setup.integer(b"dwqPackLevel")?.unwrap_or(0))?;
            let mut inputs = VecDeque::new();
            let mut budget = 0u64;
            let mut push = |source: crate::resource_bridge::NativeImageSource, alpha: SpriteAlpha| -> Result<()> {
                // Native policy may wrap an encoded image in a shared alpha
                // operation; budget the checked encoded source independently.
                let size = source.size;
                budget = budget
                    .checked_add(u64::from(size[0]) * u64::from(size[1]) * 12 + source.encoded_len as u64)
                    .context("NNN CG memory extent overflow")?;
                ensure!(budget <= 256 << 20, "NNN CG resident frame memory budget");
                inputs.push_back((source, alpha));
                Ok(())
            };
            if let Some((start, count)) = entry.animation {
                for frame in start..start + count {
                    cancel.check()?;
                    // CEffectAnimation's original default tag. The base VM
                    // does not admit an unimplemented custom-tag callback.
                    let path = format!("bg/bganime{frame}");
                    push(
                        resources
                            .image_source(&config.resources.picture(path.as_bytes(), levels)?, &cancel)?,
                        SpriteAlpha::Opaque,
                    )?;
                }
            } else {
                let [x, y] = entry.tiles();
                for tile in 0..x * y {
                    cancel.check()?;
                    let mut name = entry.name.clone();
                    if tile != 0 {
                        name.extend(format!("_{}", tile + 1).as_bytes());
                    }
                    let source = resources
                        .image_source(&config.resources.script_image(&name, levels)?, &cancel)?;
                    let alpha = if !name[..2].eq_ignore_ascii_case(b"ta") {
                        SpriteAlpha::Opaque
                    } else if source.has_mask {
                        SpriteAlpha::Preserve
                    } else {
                        SpriteAlpha::CornerKey { right: false }
                    };
                    push(source, alpha)?;
                }
            }
            let needs_background =
                entry.animation.is_none() && entry.name[..2].eq_ignore_ascii_case(b"ta");
            let background = needs_background && profile.background.is_some();
            if background {
                let path = [b"bg/".as_slice(), profile.background.as_ref().unwrap()].concat();
                push(
                    resources.image_source(&config.resources.picture(&path, levels)?, &cancel)?,
                    SpriteAlpha::Opaque,
                )?;
            }
            let voice = read(
                storage.as_ref(),
                &config.resources.path("spt/cgVoice.xtx")?,
                1 << 20,
                &cancel,
            )?
            .map(|bytes| -> Result<_> {
                crate::native_cg::cg_voice(&Names::parse(&bytes, false)?, group, cg_index)
            })
            .transpose()?
            .flatten();
            Ok(Prepared {
                difference,
                profile,
                entry,
                inputs,
                background,
                voice,
            })
        })?);
        Ok(())
    }
    pub fn pending(&self) -> bool {
        self.task.is_some() || self.loading.is_some() || self.difference_base_job.is_some() || self.difference_draw.is_some()
    }
    pub fn paths(&self) -> Vec<String> {
        self.live
            .as_ref()
            .map(|live| live.paths.clone())
            .unwrap_or_default().into_iter().chain(self.difference_path.iter().cloned()).chain(self.difference_retired_paths.iter().cloned()).collect()
    }
    pub fn take_audio(&mut self) -> Option<CgAudio> {
        self.audio.pop_front()
    }
    pub fn resume_clock(&mut self, now: u64) {
        if let Some(live) = self.live.as_mut() {
            live.last_ms = now;
        }
    }
    pub fn poll(&mut self,host:&mut dyn HostServices,input:InputFrame,admit:bool)->Result<bool> {
        self.poll_with_owner(host,input,admit,None)
    }
    pub(super) fn poll_native(&mut self,host:&mut dyn HostServices,input:InputFrame,admit:bool,vm:&mut NativeAdapter)->Result<bool> {
        self.poll_with_owner(host,input,admit,Some(vm))
    }
    fn poll_with_owner(&mut self,host:&mut dyn HostServices,input:InputFrame,admit:bool,mut vm:Option<&mut NativeAdapter>)->Result<bool> {
        if self.closing {
            return Ok(true);
        }
        if !self.poll_difference(host,vm.as_deref_mut())? {return Ok(false);}
        if self.old_difference.is_none() {
            if let Some(owner)=vm.as_deref_mut(){self.old_difference=owner.retained_cg_difference_picture();}
        }
        if let Some(task) = self.task.as_mut() {
            let Some(prepared) = task.try_take()? else {
                return Ok(false);
            };
            self.task = None;
            self.loading = Some(Loading {
                prepared,
                images: vec![],
                job: None,
                alpha:None,
            });
        }
        if let Some(loading) = self.loading.as_mut() {
            if let Some(job)=loading.job.as_mut() {
                let Some(image)=job.poll_detached(host)? else{return Ok(false);};
                loading.images.push((image,loading.alpha.take().context("NNN CG picture alpha policy absent")?));
                loading.job=None;
            }
            if let Some((source,alpha))=loading.prepared.inputs.pop_front() {
                loading.job=Some(PicturePreparation::detached(self.client,source,&self.pool,host)?);
                loading.alpha=Some(alpha);return Ok(false);
            }
            let loading = self.loading.take().unwrap();
            self.publish(host, loading)?;
        }
        let live = self.live.as_mut().context("NNN CG ready frame absent")?;
        let now = host.now_ms();
        let dt = now.saturating_sub(live.last_ms);
        live.last_ms = now;
        if !admit {
            return Ok(false);
        }
        live.remainder = live.remainder.saturating_add(dt);
        let ticks = live.remainder / live.profile.frame_ms;
        live.remainder %= live.profile.frame_ms;
        live.ticks = live.ticks.saturating_add(ticks);
        live.motion.elapsed = live.motion.elapsed.saturating_add(ticks);
        if input.buttons_pressed & (buttons::ESCAPE | buttons::MOUSE_RIGHT) != 0 {
            self.closing = true;
            self.audio.push_back(CgAudio::Sound(live.profile.sounds[2]));
            return Ok(true);
        }
        if let Some(remaining) = &mut self.voice_remaining {
            *remaining = remaining.saturating_sub(ticks);
            if *remaining == 0 {
                let name = self.last_voice.as_ref().unwrap().clone();
                self.audio.push_back(if name.first() == Some(&b'-') {
                    CgAudio::StopScripts
                } else {
                    CgAudio::Voice(name)
                });
                self.voice_remaining = None;
            }
        }
        let ctrl = input.buttons_down & buttons::CTRL != 0;
        let point = [input.pointer_x, input.pointer_y];
        let mut direction = if input.buttons_pressed & (buttons::WHEEL_UP | buttons::LEFT) != 0 {
            -1
        } else if input.buttons_pressed
            & (buttons::WHEEL_DOWN | buttons::SPACE | buttons::RIGHT | buttons::ENTER)
            != 0
        {
            1
        } else {
            0
        };
        if input.buttons_pressed & buttons::MOUSE_LEFT != 0 {
            if ctrl {
                live.motion.stop_scroll();
                live.drag = Some(point);
            } else {
                direction = 1;
            }
        } else if input.buttons_down & buttons::MOUSE_LEFT != 0 {
            if let Some(old) = live.drag {
                live.motion.drag([
                    i64::from(point[0]) - i64::from(old[0]),
                    i64::from(point[1]) - i64::from(old[1]),
                ]);
                live.drag = Some(point);
            }
        } else {
            live.drag = None;
        }
        if ticks != 0 && input.buttons_down & buttons::MOUSE_MIDDLE != 0 {
            direction = 1;
        }
        if direction != 0 {
            let next = if direction > 0 {
                (self.index + 1) % self.pictures.len()
            } else {
                (self.index + self.pictures.len() - 1) % self.pictures.len()
            };
            if live.profile.return_one_set
                && self.pictures[next].block != self.pictures[self.index].block
            {
                self.closing = true;
                self.audio.push_back(CgAudio::Sound(live.profile.sounds[2]));
                return Ok(true);
            }
            let next_entry=CgEntry::parse(&self.pictures[next].raw,live.profile.code_byte)?;
            if let Some(state)=self.difference.as_mut() {
                let old_kind=state.navigate(self.pictures[next].block==self.pictures[self.index].block,
                    next_entry.animation.is_some(),next!=self.index,live.entry.animation.is_some());
                if let Some(kind)=old_kind {
                    let index=match kind {OldDifferenceImage::FirstTile=>0,OldDifferenceImage::LastAnimationFrame=>live.views.len()-usize::from(live.background)-1};
                    let image=live.views[index].raw.clone();
                    if let Some(owner)=vm.as_deref_mut(){owner.retain_cg_difference_picture(image.clone())?;}
                    self.old_difference=Some(image);
                }
            }
            if next != self.index {
                self.audio.push_back(CgAudio::Sound(
                    live.profile.sounds[usize::from(direction < 0)],
                ));
                self.index = next;
                self.load()?;
                return Ok(false);
            }
        }
        if self.difference_active(){self.draw_difference(host,vm.as_deref_mut(),ticks!=0)?;}
        else{self.draw_pose(host)?;}
        Ok(false)
    }
    fn publish(&mut self, host: &mut dyn HostServices, loading: Loading) -> Result<()> {
        let Prepared {
            profile,
            entry,
            background,
            voice,
            difference,
            ..
        } = loading.prepared;
        if self.difference.is_none(){self.difference=Some(DifferenceState::new(difference));}
        let voice_wait = profile.voice_wait;
        let sizes = loading
            .images
            .iter()
            .map(|(image,_)| image.raw.size())
            .collect::<Vec<_>>();
        ensure!(!sizes.is_empty(), "NNN CG ready image list empty");
        let [nx, ny] = entry.tiles();
        let size = if entry.animation.is_some() {
            sizes[0]
        } else {
            [
                (0..nx as usize).map(|i| sizes[i][0]).sum(),
                (0..ny as usize).map(|j| sizes[j * nx as usize][1]).sum(),
            ]
        };
        ensure!(
            size[0] <= 262144 && size[1] <= 262144,
            "NNN CG tiled logical extent"
        );
        let motion = CgMotion::new(size, &entry, &profile)?;
        let mut paths: Vec<String> = Vec::new();
        let mut views=Vec::new();
        for (i,(view,alpha)) in loading.images.into_iter().enumerate() {
            let image=if matches!(alpha,SpriteAlpha::Opaque){view.opaque.clone()}else{view.keyed.clone()};
            if image.client() != self.client {
                for path in &paths {
                    let _ = host.release_image_resource(self.client, path);
                }
                anyhow::bail!("NNN CG foreign image owner");
            }
            match host.publish_image_resource(self.client, LAYER + i as u32, image) {
                Ok(publication) => {paths.push(publication.path.relative().to_owned());views.push(view);},
                Err(e) => {
                    for path in &paths {
                        let _ = host.release_image_resource(self.client, path);
                    }
                    return Err(e.into());
                }
            }
        }
        let old = self.live.take();
        self.live = Some(Live {
            profile,
            entry,
            paths,
            views,
            sizes,
            background,
            motion,
            ticks: 0,
            remainder: 0,
            last_ms: host.now_ms(),
            drag: None,
            pose: None,
        });
        let difference_active=self.difference_active();
        if difference_active {
            // Keep the old scene visible while source views/wipe jobs finish.
            // Publishing the new tiles first would flash the final CG before
            // a count0 difference Print. The capture is an engine-owned prior
            // screen for uncovered ordinary pixels, not the old wipe source.
            if self.difference_frame.is_none(){
                match host.begin_layer_prefix_image_over(self.client,SIZE,[i32::MAX,i32::MAX],[0,0,0,255]) {
                    Ok(job)=>self.difference_base_job=Some(job),
                    Err(error)=>{
                        for path in &self.live.take().unwrap().paths {let _=host.release_image_resource(self.client,path);}
                        self.live=old;return Err(error.into());
                    }
                }
            }
            if let Some(old)=old {self.difference_retired_paths.extend(old.paths);}
        }else {
            let result=self.put_layers(host,old.is_some());
            if let Err(error)=result {
                for path in &self.live.take().unwrap().paths {let _=host.release_image_resource(self.client,path);}
                self.live=old;return Err(error);
            }
            if let Some(old)=old{for path in old.paths{host.release_image_resource(self.client,&path)?;}}
            if let Some(old)=self.difference_path.take(){host.release_image_resource(self.client,&old)?;}
            for path in self.difference_retired_paths.drain(..){host.release_image_resource(self.client,&path)?;}
            self.difference_base_job=None;self.difference_frame=None;
        }
        if let Some(voice) = voice {
            if self.last_voice.as_ref() != Some(&voice) {
                self.last_voice = Some(voice);
                self.voice_remaining = (voice_wait > 0).then_some(u64::from(voice_wait));
            }
        }
        Ok(())
    }
    fn put_layers(&mut self, host: &mut dyn HostServices, replace: bool) -> Result<()> {
        let live = self.live.as_ref().unwrap();
        let mut commands = Vec::new();
        if replace {
            commands.push(serde_json::json!({"op":"remove","id":ROOT}));
        }
        // Uncovered pixels have a deterministic native background rather than
        // depending on an uninitialized prior screen allocation.
        commands.push(serde_json::json!({"op":"put","layer":{"id":ROOT,"rect":[0,0,800,600],"clip":[0,0,800,600],"order":[2000000001,0],"paint":{"kind":"solid","color":live.profile.color}}}));
        let count = live.paths.len() - usize::from(live.background);
        if live.background {
            let i = count;
            let [w, h] = live.sizes[i];
            commands.push(serde_json::json!({"op":"put","layer":{"id":LAYER+i as u32,"parent":ROOT,"rect":[0,0,w,h],"order":[2000000001,1],"paint":{"kind":"image","asset":live.paths[i]}}}));
        }
        let position = if live.entry.animation.is_some() {
            [0, 0]
        } else {
            live.motion.position()
        };
        let [nx, _] = live.entry.tiles();
        for i in 0..count {
            let [w, h] = live.sizes[i];
            let animated = live.entry.animation.is_some();
            let crop = if !animated {
                Some([0, 0, w.min(live.sizes[0][0]), h.min(live.sizes[0][1])])
            } else {
                None
            };
            let rect = if animated {
                [0, 0, i64::from(w), i64::from(h)]
            } else {
                [
                    (i as i64 % nx as i64) * i64::from(live.sizes[0][0]) - position[0],
                    (i as i64 / nx as i64) * i64::from(live.sizes[0][1]) - position[1],
                    i64::from(w),
                    i64::from(h),
                ]
            };
            let masked = !animated && live.entry.name[..2].eq_ignore_ascii_case(b"ta");
            commands.push(serde_json::json!({"op":"put","layer":{"id":LAYER+i as u32,"parent":ROOT,"rect":rect,"source_crop":crop,"visible":!animated||i==0,"order":[2000000001,2+i],
                "blend":{"stored_byte":{"mode":if masked {"mask_separate256"} else {"opaque_scalar256"},"force_output_opaque":true}},
                "paint":{"kind":"image","asset":live.paths[i]}}}));
        }
        host.submit_engine(
            self.client,
            1,
            &serde_json::to_vec(&serde_json::json!({"layers":commands}))?,
        )?;
        self.live.as_mut().unwrap().pose = Some((position, 0));
        Ok(())
    }
    fn draw_pose(&mut self, host: &mut dyn HostServices) -> Result<()> {
        let live = self.live.as_mut().unwrap();
        let count = live.paths.len() - usize::from(live.background);
        let frame = if live.entry.animation.is_some() {
            live.profile
                .anime_hold
                .map(|hold| {
                    (live.ticks.saturating_sub(1) / u64::from(hold) % count as u64) as usize
                })
                .unwrap_or(0)
        } else {
            0
        };
        let position = if live.entry.animation.is_some() {
            [0, 0]
        } else {
            live.motion.position()
        };
        if live.pose == Some((position, frame)) {
            return Ok(());
        }
        let mut commands = Vec::new();
        if live.entry.animation.is_some() {
            for i in 0..count {
                commands.push(
                    serde_json::json!({"op":"set_visible","id":LAYER+i as u32,"visible":i==frame}),
                );
            }
        } else {
            let [nx, _] = live.entry.tiles();
            let [w, h] = live.sizes[0];
            for i in 0..count {
                let size = live.sizes[i];
                commands.push(
                    serde_json::json!({"op":"set_visible","id":LAYER+i as u32,"visible":true}),
                );
                commands.push(serde_json::json!({"op":"set_rect","id":LAYER+i as u32,"rect":[(i as i64%nx as i64)*i64::from(w)-position[0],(i as i64/nx as i64)*i64::from(h)-position[1],size[0],size[1]]}));
            }
        }
        host.submit_engine(
            self.client,
            1,
            &serde_json::to_vec(&serde_json::json!({"layers":commands}))?,
        )?;
        live.pose = Some((position, frame));
        Ok(())
    }
    pub fn cleanup(&self) -> Vec<serde_json::Value> {
        if self.live.is_some() {
            vec![serde_json::json!({"op":"remove","id":ROOT})]
        } else {
            vec![]
        }
    }
    pub fn remove(&mut self, host: &mut dyn HostServices) -> Result<()> {
        self.task = None;
        self.loading = None;
        self.difference_draw=None;self.difference_base_job=None;self.difference_frame=None;
        host.submit_engine(
            self.client,
            1,
            &serde_json::to_vec(&serde_json::json!({"layers":self.cleanup()}))?,
        )?;
        if let Some(live) = self.live.take() {
            for path in live.paths {
                host.release_image_resource(self.client, &path)?;
            }
        }
        if let Some(path)=self.difference_path.take(){host.release_image_resource(self.client,&path)?;}
        for path in self.difference_retired_paths.drain(..){host.release_image_resource(self.client,&path)?;}
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        XorProfile, config::Names, expression::VariableProfile,
        native_bootstrap::NativeConfiguration, native_resources::ResourceNames,
    };
    use crysirror_core::service::EngineQuery;
    use serde_json::{Value, json};
    use std::time::{Duration, Instant};
    fn runtime() -> (tempfile::TempDir, crysirror_runtime::Runtime) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("assets")).unwrap();
        let project = serde_json::from_value(json!({"schema":2,"name":"CG actor","content_revision":"1","assets":"assets","canvas":[800,600],"clients":[{"name":"native","adapter":"crysirror-script-v1","game":"authored"}]})).unwrap();
        let mut rt = crysirror_runtime::Runtime::with_asset_root(
            project,
            dir.path(),
            &dir.path().join("state"),
            Default::default(),
        )
        .unwrap();
        rt.tick(0, Default::default()).unwrap();
        (dir, rt)
    }
    fn viewer(
        rt: &mut crysirror_runtime::Runtime,
        raw: &[u8],
        inputs: Vec<ImageInput>,
    ) -> PictureViewer {
        let names = Names { tokens: vec![] };
        let config = Arc::new(NativeConfiguration {
            resources: ResourceNames::new("").unwrap(),
            setup: names.clone(),
            main_control: None,
            chart_setup: None,
            variable_names: names.clone(),
            project: names.clone(),
            commands: names.clone(),
            functions: names.clone(),
            initial: None,
            module_sha256: "authored".into(),
            profile: VariableProfile::Legacy1000,
            scripts: XorProfile::default(),
        });
        let profile = CgProfile::read(
            &Names::parse(b"animeFrame 2 codeByte 1", false).unwrap(),
            &names,
            0,
        )
        .unwrap();
        let entry = CgEntry::parse(raw, 1).unwrap();
        PictureViewer {
            difference:None,old_difference:None,difference_base_job:None,difference_frame:None,
            difference_draw:None,difference_pose:None,difference_path:None,difference_retired_paths:Vec::new(),
            pictures: vec![CgChoice {
                raw: raw.to_vec(),
                block: 0,
                index: 0,
            }]
            .into(),
            index: 0,
            group: 0,
            task: None,
            loading: Some(Loading {
                prepared: Prepared {
                    profile,
                    entry,
                    inputs: inputs
                        .into_iter()
                        .map(|input| {let size=match &input{ImageInput::Solid{size,..}=>*size,_=>panic!("authored CG fixture extent")};
                            (crate::resource_bridge::NativeImageSource{input,has_mask:false,size,encoded_len:0,clip_insets:None},SpriteAlpha::Opaque)})
                        .collect(),
                    background: false,
                    voice: None,
                    difference:DifferenceProfile{selector:0,frames:1},
                },
                images: vec![],
                job: None,
                alpha:None,
            }),
            live: None,
            config,
            resources: NativeResources::blackcyc(ClientId(1), Arc::new(|_, _| Ok(None))).unwrap(),
            storage: rt.host.storage(ClientId(1)).unwrap(),
            pool: TaskPool::new("nnn-cg-test", 1, 1).unwrap(),
            client: ClientId(1),
            audio: VecDeque::new(),
            closing: false,
            last_voice: None,
            voice_remaining: None,
        }
    }
    fn ready(viewer: &mut PictureViewer, rt: &mut crysirror_runtime::Runtime) {
        let end = Instant::now() + Duration::from_secs(5);
        while viewer.pending() {
            viewer
                .poll(&mut rt.host, InputFrame::default(), false)
                .unwrap();
            rt.tick(0, Default::default()).unwrap();
            assert!(Instant::now() < end);
            std::thread::yield_now();
        }
    }
    fn layer(rt: &mut crysirror_runtime::Runtime, id: u32) -> Value {
        serde_json::from_slice(
            &rt.host
                .query_engine(ClientId(1), EngineQuery::Layer(id))
                .unwrap(),
        )
        .unwrap()
    }
    #[test]
    fn native_cg_tiles_scroll_and_drag_use_shared_layers_and_retire_resources() {
        let (_dir, mut rt) = runtime();
        let mut cg = viewer(
            &mut rt,
            b"[<2:30_10]ev_pan",
            vec![
                ImageInput::Solid {
                    size: [600, 600],
                    color: [255, 0, 0, 255],
                },
                ImageInput::Solid {
                    size: [600, 600],
                    color: [0, 255, 0, 255],
                },
            ],
        );
        ready(&mut cg, &mut rt);
        assert_eq!(
            layer(&mut rt, LAYER)["rect"],
            json!([-400.0, 0.0, 600.0, 600.0])
        );
        assert_eq!(
            layer(&mut rt, LAYER + 1)["rect"],
            json!([200.0, 0.0, 600.0, 600.0])
        );
        assert_eq!(
            layer(&mut rt, ROOT)["clip"],
            json!([0.0, 0.0, 800.0, 600.0])
        );
        rt.tick(1250, Default::default()).unwrap();
        cg.poll(&mut rt.host, Default::default(), true).unwrap();
        assert_eq!(
            layer(&mut rt, LAYER)["rect"],
            json!([-200.0, 0.0, 600.0, 600.0])
        );
        cg.poll(
            &mut rt.host,
            InputFrame {
                buttons_down: buttons::CTRL | buttons::MOUSE_LEFT,
                buttons_pressed: buttons::MOUSE_LEFT,
                pointer_x: 100,
                pointer_y: 100,
            },
            true,
        )
        .unwrap();
        cg.poll(
            &mut rt.host,
            InputFrame {
                buttons_down: buttons::CTRL | buttons::MOUSE_LEFT,
                pointer_x: 20,
                pointer_y: 100,
                ..Default::default()
            },
            true,
        )
        .unwrap();
        assert_eq!(
            layer(&mut rt, LAYER)["rect"],
            json!([-80.0, 0.0, 600.0, 600.0])
        );
        cg.remove(&mut rt.host).unwrap();
        assert!(layer(&mut rt, LAYER).is_null());
        assert!(cg.paths().is_empty());
    }
    #[test]
    fn native_cg_animation_preserves_hold_counts_and_suspended_clock() {
        let (_dir, mut rt) = runtime();
        let mut cg = viewer(
            &mut rt,
            b"#3-2/bg_preview",
            vec![
                ImageInput::Solid {
                    size: [8, 8],
                    color: [255, 0, 0, 255],
                },
                ImageInput::Solid {
                    size: [8, 8],
                    color: [0, 255, 0, 255],
                },
            ],
        );
        cg.loading.as_mut().unwrap().prepared.voice = Some(b"ga001".to_vec());
        ready(&mut cg, &mut rt);
        assert_eq!(layer(&mut rt, LAYER)["visible"], true);
        for visible in [0, 0, 1, 1, 0] {
            rt.tick(50, Default::default()).unwrap();
            cg.poll(&mut rt.host, Default::default(), true).unwrap();
            assert_eq!(layer(&mut rt, LAYER)["visible"], visible == 0);
            assert_eq!(layer(&mut rt, LAYER + 1)["visible"], visible == 1);
        }
        let old = cg.live.as_ref().unwrap().ticks;
        rt.tick(1000, Default::default()).unwrap();
        cg.poll(&mut rt.host, Default::default(), false).unwrap();
        rt.tick(50, Default::default()).unwrap();
        cg.poll(&mut rt.host, Default::default(), true).unwrap();
        assert_eq!(cg.live.as_ref().unwrap().ticks, old + 1);
        rt.tick(200, Default::default()).unwrap();
        cg.poll(&mut rt.host, Default::default(), true).unwrap();
        assert!(matches!(cg.take_audio(),Some(CgAudio::Voice(name)) if name == b"ga001"));
        assert!(cg.take_audio().is_none());
        assert!(
            cg.poll(
                &mut rt.host,
                InputFrame {
                    buttons_pressed: buttons::MOUSE_RIGHT,
                    ..Default::default()
                },
                true
            )
            .unwrap()
        );
        assert!(matches!(cg.take_audio(), Some(CgAudio::Sound(2))));
        cg.remove(&mut rt.host).unwrap();
    }
}
