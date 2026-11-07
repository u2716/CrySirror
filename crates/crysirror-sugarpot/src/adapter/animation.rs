// SPDX-License-Identifier: MPL-2.0
//! Native animation selection over shared frame archives, image jobs and clocks.
use super::*;
#[derive(Clone)]
pub(super) struct AnimationAsset {
    pub frames: Vec<UploadedImage>,
    pub programs: Vec<Vec<u16>>,
}
#[derive(Clone)]
pub(super) struct AnimationState {
    pub asset: Arc<AnimationAsset>,
    pub selected: usize,
    pub clock: crysirror_scene::sprite::SheetSequence,
}
pub(super) struct AnimationLoad {
    path: String,
    worker: Option<Task<crate::resources::PreparedAnimation>>,
    prepared: Option<crate::resources::PreparedAnimation>,
    frames: Vec<UploadedImage>,
    job: Option<ImageJob>,
}
impl SugarPotVm {
    pub(super) fn begin_animation(&mut self, path: String, bytes: Arc<[u8]>) -> Result<()> {
        let client = self.client;
        let worker = self
            .pool
            .as_ref()
            .unwrap()
            .spawn(move |cancel| crate::resources::animation(bytes, client, &cancel))?;
        self.animation_load = Some(AnimationLoad {
            path,
            worker: Some(worker),
            prepared: None,
            frames: vec![],
            job: None,
        });
        Ok(())
    }
    pub(super) fn load_animation(
        &mut self,
        h: &mut dyn HostServices,
        path: &str,
    ) -> Result<Option<UploadedImage>> {
        let mut load = self
            .animation_load
            .take()
            .context("SugarPot animation load state")?;
        ensure!(
            load.path == path,
            "SugarPot animation continuation mismatch"
        );
        if let Some(worker) = &mut load.worker {
            let Some(prepared) = worker.try_take()? else {
                self.animation_load = Some(load);
                return Ok(None);
            };
            load.prepared = Some(prepared);
            load.worker = None;
        }
        let prepared = load.prepared.as_ref().unwrap();
        if let Some(job) = &mut load.job {
            match job.poll()? {
                ImageProcessState::Pending => {
                    self.animation_load = Some(load);
                    return Ok(None);
                }
                ImageProcessState::Failed(e) => bail!("SugarPot shared animation frame: {e}"),
                ImageProcessState::Ready(resource) => {
                    let image = h.publish_image_resource(self.client, self.next_image, resource)?;
                    self.next_image = self
                        .next_image
                        .checked_add(1)
                        .context("SugarPot image ID exhausted")?;
                    self.depths.insert(image.path.relative().into(), 32);
                    load.frames.push(image);
                    load.job = None;
                }
            }
        }
        if load.frames.len() == prepared.frames.len() {
            let first = load
                .frames
                .first()
                .context("SugarPot empty animation")?
                .clone();
            self.images.insert(path.into(), first.clone());
            self.animations.insert(
                path.into(),
                Arc::new(AnimationAsset {
                    frames: load.frames,
                    programs: prepared.programs.clone(),
                }),
            );
            return Ok(Some(first));
        }
        let bytes = prepared.frames[load.frames.len()].clone();
        load.job = Some(
            h.image_metadata(self.client)?
                .begin_encoded("abm-frame.png", bytes)?,
        );
        self.animation_load = Some(load);
        Ok(None)
    }
    pub(super) fn animation_state(&self, name: &str, now: u64) -> Result<Option<AnimationState>> {
        let path = self.resolve("grp", name, &["abm", "bmp", "dat"])?;
        let Some(asset) = self.animations.get(&path) else {
            return Ok(None);
        };
        Ok(Some(AnimationState {
            asset: asset.clone(),
            selected: 0,
            clock: crysirror_scene::sprite::SheetSequence {
                cells: u16::try_from(asset.programs[0].len())?,
                frame_ms: 33,
                mode: crysirror_scene::sprite::SheetLoop::Repeat,
                started_ms: now,
            },
        }))
    }
    pub(super) fn animate(&mut self, h: &mut dyn HostServices) -> Result<()> {
        let now = h.now_ms();
        let mut layers = vec![];
        for (id, sprite) in self.sprites.iter_mut().enumerate() {
            if !sprite.visible || !self.published_layers.contains(&(id as u32 + 1)) {
                continue;
            }
            let Some(animation) = &mut sprite.animation else {
                continue;
            };
            let cursor = animation.clock.frame(now)? as usize;
            let frame = animation.asset.programs[animation.selected][cursor] as usize;
            let image = animation
                .asset
                .frames
                .get(frame)
                .context("SugarPot native animation frame")?;
            if sprite
                .image
                .as_ref()
                .is_none_or(|old| old.path != image.path)
            {
                sprite.image = Some(image.clone());
                //The native geometric effect reads this current logical
                //bitmap, but owns its replacement presentation until finish.
                if !self.geometry.as_ref().is_some_and(|g| g.owns_sprite(id))
                    && !self.rotation.as_ref().is_some_and(|r| r.owns_sprite(id))
                {
                    layers.push(json!({"op":"set_paint","id":id as u32+1,"paint":{"kind":"image","asset":image.path.relative()}}));
                }
            }
        }
        if !layers.is_empty() {
            self.submit(h, layers, vec![])?;
        }
        Ok(())
    }
    pub(super) fn select_animation(&mut self, id: usize, target: usize, now: u64) -> Result<()> {
        let animation = self
            .sprites
            .get_mut(id)
            .context("SugarPot anmnum sprite")?
            .animation
            .as_mut()
            .context("SugarPot anmnum requires animated image")?;
        animation.selected = target;
        animation.clock.cells = u16::try_from(animation.asset.programs[target].len())?;
        animation.clock.started_ms = now;
        animation.clock.frame_ms = 33;
        Ok(())
    }
}

pub(super) struct AnimationTone {
    key: String,
    state: AnimationState,
    frames: Vec<UploadedImage>,
    next: usize,
}
impl SugarPotVm {
    pub(super) fn tone_animation(
        &mut self,
        h: &mut dyn HostServices,
        key: String,
        state: AnimationState,
    ) -> Result<Option<AnimationState>> {
        let mut tone = self.animation_tone.take().unwrap_or_else(|| AnimationTone {
            key: key.clone(),
            frames: state.asset.frames.clone(),
            state,
            next: 0,
        });
        ensure!(tone.key == key, "SugarPot animated mono continuation");
        while tone.next < tone.frames.len() {
            let Some(image) = self.tone_image(h, tone.frames[tone.next].clone())? else {
                self.animation_tone = Some(tone);
                return Ok(None);
            };
            tone.frames[tone.next] = image;
            tone.next += 1;
        }
        tone.state.asset = Arc::new(AnimationAsset {
            frames: tone.frames,
            programs: tone.state.asset.programs.clone(),
        });
        Ok(Some(tone.state))
    }
    pub(super) fn animation_running(&self) -> bool {
        //The native presenter continues 66ms prompt cells and the auto timer
        //while waiting for input. They must not be parked behind an input-only
        //yield simply because no character sprite has an animation.
        if matches!(self.wait, Some(Wait::Text))
            && (self.native_text.waiting == 128
                || self.settings.values[5] == 1
                || self.window_ui.skip
                || self.windows.get(self.current_window).is_some_and(|w| {
                    w.visibility == 1 && (w.native.foreground.is_some() || w.native.page.is_some())
                }))
        {
            return true;
        }
        self.sprites.iter().enumerate().any(|(id, s)| {
            s.visible && s.animation.is_some() && self.published_layers.contains(&(id as u32 + 1))
        })
    }
}
