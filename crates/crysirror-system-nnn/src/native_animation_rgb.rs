// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Retained depth32 ANIMATION137 cache/read/draw transaction. Original global
//! lastAnime and each CPicture's cache/mask/load facts remain adapter policy;
//! file IO, decoding, zero planes and composition are engine-owned jobs.
use crate::{
    image_bridge::{NativePictureViews, PicturePreparation},
    native_animation::{AnimationConfig, FrameSelection},
    native_geometry::NativeRects,
    native_picture_cache::PictureCache,
    native_picture_reload::{ContextData, Reload},
};
use anyhow::{Context, Result, ensure};
use crysirror_core::{
    image_processing::{ImageInput, ImageJob, ImageProcessState, ImageResource},
    service::HostServices,
    state::ClientId,
};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone)]
pub(crate) struct RgbBank {
    pub config: AnimationConfig,
    pub last_anime: i32,
    pub palette: crate::native_png_palette::PngPalette,
    client: ClientId,
    slots: BTreeMap<i32, PictureCache<NativePictureViews>>,
    old: PictureCache<NativePictureViews>,
}
impl RgbBank {
    /// CommonPrintCG and effect137 address the very same native CPicture
    /// bank. Its public native getter clamps instead of allocating CG copies.
    pub(crate) fn buffer(&self, number: i32) -> Result<Option<&PictureCache<NativePictureViews>>> {
        let Some(index) = self.config.buffer_index(number)? else {
            return Ok(None);
        };
        Ok(Some(
            self.slots
                .get(&index)
                .context("NNN ANIMATION constructed buffer absent")?,
        ))
    }
}

#[derive(Clone)]
pub(crate) struct LoadedBuffer {
    pub bank: RgbBank,
    pub returned: bool,
}
/// Shared native cache mutation used by CG's ordered preload loop. Loading a
/// buffer this way does not change lastAnimeNumber (only effect137 Print does).
pub(crate) struct BufferLoad {
    candidate: RgbBank,
    index: i32,
    load: Reload,
    ready: Option<LoadedBuffer>,
    pub api_calls: u64,
}
impl BufferLoad {
    pub(crate) fn begin(
        bank: &RgbBank,
        context: &ContextData,
        number: i32,
        owner: u64,
        filename: &[u8],
    ) -> Result<Self> {
        let index = bank
            .config
            .buffer_index(number)?
            .context("NNN ANIMATION buffer disabled")?;
        let current = bank
            .buffer(index)?
            .context("NNN ANIMATION buffer disabled")?;
        let load = Reload::begin(context, current, owner, filename)?;
        Ok(Self {
            candidate: bank.clone(),
            index,
            load,
            ready: None,
            api_calls: 0,
        })
    }
    pub(crate) fn poll(&mut self, host: &mut dyn HostServices) -> Result<Option<LoadedBuffer>> {
        if let Some(ready) = &self.ready {
            return Ok(Some(ready.clone()));
        }
        let before = self.load.api_calls;
        let result = self.load.poll(host);
        self.api_calls += self.load.api_calls - before;
        let Some(loaded) = result? else {
            return Ok(None);
        };
        if let Some(header) = loaded.png_header.as_ref() {
            self.candidate.palette.observe_header(header)?;
        }
        self.candidate.slots.insert(self.index, loaded.cache);
        let ready = LoadedBuffer {
            bank: self.candidate.clone(),
            returned: loaded.returned,
        };
        self.ready = Some(ready.clone());
        Ok(Some(ready))
    }
}
pub(crate) struct BlankPreparation {
    client: ClientId,
    config: AnimationConfig,
    picture: PicturePreparation,
    ready: Option<RgbBank>,
    pub api_calls: u64,
}
impl BlankPreparation {
    pub(crate) fn begin(
        client: ClientId,
        context: &ContextData,
        config: AnimationConfig,
        size: [u32; 2],
        host: &mut dyn HostServices,
    ) -> Result<Self> {
        config.validate()?;
        ensure!(
            client == context.resources.client() && config.enabled && !config.indexed(),
            "NNN ANIMATION RGB constructor profile"
        );
        crysirror_image::rgba_len(size[0], size[1])?;
        let source = crate::resource_bridge::NativeImageSource {
            input: ImageInput::Solid {
                size,
                color: [0; 4],
            },
            has_mask: true,
            size,
            encoded_len: 0,
            clip_insets: Some([0; 4]),
            png_header: None,
        };
        // CPicture(size,TRUE) had allocated but uninitialized RGB/mask bytes.
        // Explicit shared zero planes define that old undefined read. Its
        // constructor CharaClip is FULL, not mask-derived empty coverage.
        // Immutable initial leases may alias; reload replaces only one cache.
        let picture = PicturePreparation::detached(client, source, &context.pool, host)?;
        let api_calls = picture.api_calls;
        Ok(Self {
            client,
            config,
            picture,
            ready: None,
            api_calls,
        })
    }
    pub(crate) fn poll(&mut self, host: &mut dyn HostServices) -> Result<Option<RgbBank>> {
        if let Some(ready) = &self.ready {
            return Ok(Some(ready.clone()));
        }
        let before = self.picture.api_calls;
        let result = self.picture.poll_detached(host);
        self.api_calls += self.picture.api_calls - before;
        let Some(picture) = result? else {
            return Ok(None);
        };
        let cache = PictureCache::sized(self.client, picture, false, true)?;
        let bank = RgbBank {
            config: self.config.clone(),
            last_anime: 0,
            palette: Default::default(),
            client: self.client,
            slots: (0..self.config.buffer_max)
                .map(|i| (i, cache.clone()))
                .collect(),
            old: cache,
        };
        self.ready = Some(bank.clone());
        Ok(Some(bank))
    }
}

#[derive(Clone)]
pub(crate) struct Painted {
    pub bank: RgbBank,
    /// None means native no-print/caching, while candidate resource metadata
    /// still commits. It is never a fabricated successfully drawn image.
    pub image: Option<Arc<dyn ImageResource>>,
}
enum Work {
    Primary(Reload),
    Old(Reload),
    Paint(ImageJob),
}
pub(crate) struct FrameWork {
    candidate: RgbBank,
    context: ContextData,
    selection: FrameSelection,
    owner: u64,
    count: i32,
    frames: i32,
    rectangles: NativeRects,
    canvas: [u32; 2],
    background: bool,
    base: ImageInput,
    at: Option<[i32; 2]>,
    work: Option<Work>,
    ready: Option<Painted>,
    failed: Option<String>,
    pub api_calls: u64,
}
impl FrameWork {
    pub(crate) fn begin(
        bank: &RgbBank,
        context: ContextData,
        selection: FrameSelection,
        owner: u64,
        count: i32,
        frames: i32,
        rectangles: NativeRects,
        canvas: [u32; 2],
        background: bool,
        base: ImageInput,
    ) -> Result<Self> {
        ensure!(
            bank.client == context.resources.client()
                && bank.config.enabled
                && !bank.config.indexed(),
            "NNN ANIMATION RGB frame owner/profile"
        );
        let cache = bank
            .slots
            .get(&selection.slot)
            .context("NNN ANIMATION RGB cache identity absent")?;
        let load = Reload::begin(&context, cache, owner, &selection.filename)?;
        Ok(Self {
            candidate: bank.clone(),
            context,
            selection,
            owner,
            count,
            frames,
            rectangles,
            canvas,
            background,
            base,
            at: None,
            work: Some(Work::Primary(load)),
            ready: None,
            failed: None,
            api_calls: 0,
        })
    }
    pub(crate) fn poll(&mut self, host: &mut dyn HostServices) -> Result<Option<Painted>> {
        if let Some(error) = &self.failed {
            anyhow::bail!("NNN ANIMATION RGB frame: {error}")
        }
        if let Some(ready) = &self.ready {
            return Ok(Some(ready.clone()));
        }
        let result = self.advance(host);
        if let Err(error) = &result {
            self.failed = Some(format!("{error:#}"));
        }
        result
    }
    fn finish(&mut self, image: Option<Arc<dyn ImageResource>>) -> Option<Painted> {
        self.work = None;
        let ready = Painted {
            bank: self.candidate.clone(),
            image,
        };
        self.ready = Some(ready.clone());
        Some(ready)
    }
    fn prepare_draw(
        &mut self,
        host: &mut dyn HostServices,
        old_returned: bool,
    ) -> Result<Option<Painted>> {
        let current = self
            .candidate
            .slots
            .get(&self.selection.slot)
            .context("NNN ANIMATION current cache absent")?;
        let mut picture = current
            .storage
            .as_ref()
            .context("NNN ANIMATION allocated RGB buffer absent")?
            .clone();
        picture.has_mask = current.mask_exists;
        let old = if old_returned {
            let mut view = self
                .candidate
                .old
                .storage
                .as_ref()
                .context("NNN ANIMATION allocated old buffer absent")?
                .clone();
            view.has_mask = self.candidate.old.mask_exists;
            Some((
                view,
                self.selection
                    .transition
                    .context("NNN ANIMATION old blend weights absent")?,
            ))
        } else {
            None
        };
        let input = crate::native_animation_draw::image_input(
            self.base.clone(),
            &picture,
            current.load_ok,
            old.as_ref().map(|(view, weights)| (view, *weights)),
            false,
            self.at.context("NNN ANIMATION draw position absent")?,
            self.canvas,
            self.background,
        )?;
        let Some(input) = input else {
            return Ok(self.finish(None));
        };
        self.work = Some(Work::Paint(
            host.image_metadata(self.candidate.client)?
                .begin_process(input, b"[]".as_slice().into())?,
        ));
        self.api_calls += 1;
        Ok(None)
    }
    fn advance(&mut self, host: &mut dyn HostServices) -> Result<Option<Painted>> {
        loop {
            let work = self
                .work
                .take()
                .context("NNN ANIMATION RGB transaction absent")?;
            match work {
                Work::Primary(mut load) => {
                    let before = load.api_calls;
                    let result = load.poll(host);
                    self.api_calls += load.api_calls - before;
                    let Some(loaded) = result? else {
                        self.work = Some(Work::Primary(load));
                        return Ok(None);
                    };
                    if let Some(header) = loaded.png_header.as_ref() {
                        self.candidate.palette.observe_header(header)?;
                    }
                    self.candidate
                        .slots
                        .insert(self.selection.slot, loaded.cache);
                    if !loaded.returned || self.selection.caching {
                        return Ok(self.finish(None));
                    }
                    // These native writes/evaluations precede the OLD file
                    // read and are part of this retained frame candidate.
                    self.candidate.last_anime = self.selection.anime;
                    self.at = Some(crate::native_animation_draw::destination(
                        self.count,
                        self.frames,
                        &self.rectangles,
                    )?);
                    if let Some(name) = &self.selection.old_filename {
                        self.work = Some(Work::Old(Reload::begin(
                            &self.context,
                            &self.candidate.old,
                            self.owner,
                            name,
                        )?));
                    } else {
                        return self.prepare_draw(host, false);
                    }
                }
                Work::Old(mut load) => {
                    let before = load.api_calls;
                    let result = load.poll(host);
                    self.api_calls += load.api_calls - before;
                    let Some(loaded) = result? else {
                        self.work = Some(Work::Old(load));
                        return Ok(None);
                    };
                    if let Some(header) = loaded.png_header.as_ref() {
                        self.candidate.palette.observe_header(header)?;
                    }
                    self.candidate.old = loaded.cache;
                    return self.prepare_draw(host, loaded.returned);
                }
                Work::Paint(mut job) => match job.poll()? {
                    ImageProcessState::Pending => {
                        self.work = Some(Work::Paint(job));
                        return Ok(None);
                    }
                    ImageProcessState::Failed(error) => {
                        anyhow::bail!("NNN ANIMATION RGB image worker: {error}")
                    }
                    ImageProcessState::Ready(image) => {
                        ensure!(
                            image.client() == self.candidate.client && image.size() == self.canvas,
                            "NNN ANIMATION RGB image result scope"
                        );
                        return Ok(self.finish(Some(image)));
                    }
                },
            }
        }
    }
}
