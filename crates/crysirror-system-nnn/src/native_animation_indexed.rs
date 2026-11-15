// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Indexed ANIMATION137 buffer bank. Original CPicture index/allocation,
//! same-name/loadOk and global palette-history facts survive shared RGB output.
use crate::{
    native_animation::{AnimationConfig, FrameSelection},
    native_geometry::NativeRects,
    native_indexed_reload::{IndexedPicture, Reload},
    native_picture_cache::PictureCache,
    native_picture_reload::ContextData,
    native_png_palette::PngPalette,
};
use anyhow::{Context, Result, ensure};
use crysirror_core::{
    image_processing::{ImageInput, ImageJob, ImageProcessState, ImageResource},
    service::HostServices,
    state::ClientId,
};
use std::{collections::BTreeMap, sync::Arc};
#[derive(Clone)]
pub(crate) struct IndexedBank {
    pub config: AnimationConfig,
    pub last_anime: i32,
    pub palette: PngPalette,
    client: ClientId,
    slots: BTreeMap<i32, PictureCache<IndexedPicture>>,
    old: PictureCache<IndexedPicture>,
}
impl IndexedBank {
    pub(crate) fn buffer(&self, number: i32) -> Result<Option<&PictureCache<IndexedPicture>>> {
        let Some(index) = self.config.buffer_index(number)? else {
            return Ok(None);
        };
        Ok(Some(
            self.slots
                .get(&index)
                .context("NNN indexed ANIMATION allocated buffer")?,
        ))
    }
}
pub(crate) struct BlankPreparation {
    config: AnimationConfig,
    client: ClientId,
    picture: crate::native_indexed_blank::Preparation,
    ready: Option<IndexedBank>,
}
impl BlankPreparation {
    pub(crate) fn begin(
        client: ClientId,
        context: &ContextData,
        config: AnimationConfig,
        size: [u32; 2],
        palette: PngPalette,
    ) -> Result<Self> {
        config.validate()?;
        ensure!(
            client == context.resources.client() && config.enabled && config.indexed(),
            "NNN indexed ANIMATION constructor profile"
        );
        let picture =
            crate::native_indexed_blank::Preparation::begin(client, &context.pool, size, palette)?;
        Ok(Self {
            config,
            client,
            picture,
            ready: None,
        })
    }
    pub(crate) fn poll(&mut self, host: &mut dyn HostServices) -> Result<Option<IndexedBank>> {
        if let Some(ready) = &self.ready {
            return Ok(Some(ready.clone()));
        }
        let Some(cache) = self.picture.poll(host)? else {
            return Ok(None);
        };
        let palette = cache
            .storage
            .as_ref()
            .context("NNN indexed ANIMATION blank palette")?
            .decoded
            .palette
            .clone();
        let bank = IndexedBank {
            config: self.config.clone(),
            client: self.client,
            last_anime: 0,
            palette,
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
pub(crate) struct LoadedBuffer {
    pub bank: IndexedBank,
    pub returned: bool,
}
pub(crate) struct BufferLoad {
    candidate: IndexedBank,
    index: i32,
    load: Reload,
    ready: Option<LoadedBuffer>,
}
impl BufferLoad {
    pub(crate) fn begin(
        bank: &IndexedBank,
        context: &ContextData,
        number: i32,
        owner: u64,
        name: &[u8],
    ) -> Result<Self> {
        let index = bank
            .config
            .buffer_index(number)?
            .context("NNN indexed ANIMATION disabled")?;
        let current = bank
            .buffer(index)?
            .context("NNN indexed ANIMATION disabled")?;
        let load = Reload::begin(context, current, owner, name, bank.palette.clone())?;
        Ok(Self {
            candidate: bank.clone(),
            index,
            load,
            ready: None,
        })
    }
    pub(crate) fn poll(&mut self, host: &mut dyn HostServices) -> Result<Option<LoadedBuffer>> {
        if let Some(ready) = &self.ready {
            return Ok(Some(ready.clone()));
        }
        let Some(loaded) = self.load.poll(host)? else {
            return Ok(None);
        };
        self.candidate.palette = loaded.palette;
        self.candidate.slots.insert(self.index, loaded.cache);
        let ready = LoadedBuffer {
            bank: self.candidate.clone(),
            returned: loaded.returned,
        };
        self.ready = Some(ready.clone());
        Ok(Some(ready))
    }
}
#[derive(Clone)]
pub(crate) struct Painted {
    pub bank: IndexedBank,
    pub image: Option<Arc<dyn ImageResource>>,
}
enum Work {
    Primary(Reload),
    Old(Reload),
    Paint(ImageJob),
}
pub(crate) struct FrameWork {
    candidate: IndexedBank,
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
    failure: Option<String>,
    pub api_calls: u64,
}
impl FrameWork {
    pub(crate) fn begin(
        bank: &IndexedBank,
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
                && bank.config.indexed(),
            "NNN indexed ANIMATION frame profile"
        );
        let current = bank
            .slots
            .get(&selection.slot)
            .context("NNN indexed ANIMATION cache identity")?;
        let load = Reload::begin(
            &context,
            current,
            owner,
            &selection.filename,
            bank.palette.clone(),
        )?;
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
            failure: None,
            api_calls: 0,
        })
    }
    pub(crate) fn poll(&mut self, host: &mut dyn HostServices) -> Result<Option<Painted>> {
        if let Some(error) = &self.failure {
            anyhow::bail!("NNN indexed ANIMATION: {error}")
        }
        if let Some(ready) = &self.ready {
            return Ok(Some(ready.clone()));
        }
        let result = self.advance(host);
        if let Err(error) = &result {
            self.failure = Some(format!("{error:#}"));
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
    fn draw(&mut self, host: &mut dyn HostServices, old_returned: bool) -> Result<Option<Painted>> {
        let cache = self
            .candidate
            .slots
            .get(&self.selection.slot)
            .context("NNN indexed ANIMATION current cache")?;
        let current = cache
            .storage
            .as_ref()
            .context("NNN indexed ANIMATION current allocation")?
            .native_views();
        let old = if old_returned {
            Some((
                self.candidate
                    .old
                    .storage
                    .as_ref()
                    .context("NNN indexed ANIMATION old allocation")?
                    .native_views(),
                self.selection
                    .transition
                    .context("NNN indexed ANIMATION transition weights")?,
            ))
        } else {
            None
        };
        let input = crate::native_animation_draw::image_input(
            self.base.clone(),
            &current,
            cache.load_ok,
            old.as_ref().map(|(view, weights)| (view, *weights)),
            true,
            self.at.context("NNN indexed ANIMATION destination")?,
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
                .context("NNN indexed ANIMATION receipt absent")?;
            match work {
                Work::Primary(mut load) => {
                    let before = load.api_calls;
                    let result = load.poll(host);
                    self.api_calls += load.api_calls - before;
                    let Some(loaded) = result? else {
                        self.work = Some(Work::Primary(load));
                        return Ok(None);
                    };
                    self.candidate.palette = loaded.palette;
                    self.candidate
                        .slots
                        .insert(self.selection.slot, loaded.cache);
                    if !loaded.returned || self.selection.caching {
                        return Ok(self.finish(None));
                    }
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
                            self.candidate.palette.clone(),
                        )?));
                    } else {
                        return self.draw(host, false);
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
                    self.candidate.palette = loaded.palette;
                    self.candidate.old = loaded.cache;
                    return self.draw(host, loaded.returned);
                }
                Work::Paint(mut job) => match job.poll()? {
                    ImageProcessState::Pending => {
                        self.work = Some(Work::Paint(job));
                        return Ok(None);
                    }
                    ImageProcessState::Failed(error) => {
                        anyhow::bail!("NNN indexed ANIMATION image worker: {error}")
                    }
                    ImageProcessState::Ready(image) => {
                        ensure!(
                            image.client() == self.candidate.client && image.size() == self.canvas,
                            "NNN indexed ANIMATION result scope"
                        );
                        return Ok(self.finish(Some(image)));
                    }
                },
            }
        }
    }
}
