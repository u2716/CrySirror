// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! A single depth-selected ANIMATION bank shared by effect137 and the CG menu.
//! Native filenames/lastAnime/cache metadata stay here; both backends retain
//! CrySirror image leases and cancellable read/decode/draw transactions.
use crate::{
    image_bridge::NativePictureViews, native_animation::AnimationConfig,
    native_animation_indexed as indexed, native_animation_rgb as rgb, native_geometry::NativeRects,
    native_picture_reload::ContextData, native_png_palette::PngPalette,
};
use anyhow::Result;
use crysirror_core::{
    image_processing::{ImageInput, ImageResource},
    service::HostServices,
    state::ClientId,
};
use std::sync::Arc;
pub(crate) fn configuration(setup: &crate::config::Names) -> Result<AnimationConfig> {
    let config = AnimationConfig {
        enabled: setup.integer(b"animeEffectEnable")?.unwrap_or(0) != 0,
        buffer_max: setup.integer(b"animeBufferMax")?.unwrap_or(60),
        depth: setup.integer(b"animeBufferDepth")?.unwrap_or(32),
        // GameCallBack only sets enable/max/depth before CEffect creation.
        // No invented setup key replaces the native SetAnimeTageName call.
        tag: b"bg\\bganime".to_vec(),
    };
    config.validate()?;
    Ok(config)
}
#[derive(Clone)]
pub(crate) enum Bank {
    Disabled(AnimationConfig),
    Rgb(rgb::RgbBank),
    Indexed(indexed::IndexedBank),
}
impl Bank {
    pub(crate) fn palette(&self) -> Option<&PngPalette> {
        match self {
            Self::Rgb(b) => Some(&b.palette),
            Self::Indexed(b) => Some(&b.palette),
            Self::Disabled(_) => None,
        }
    }
    pub(crate) fn set_palette(&mut self, palette: PngPalette) {
        match self {
            Self::Rgb(b) => b.palette = palette,
            Self::Indexed(b) => b.palette = palette,
            Self::Disabled(_) => {}
        }
    }

    pub(crate) fn config(&self) -> &AnimationConfig {
        match self {
            Self::Disabled(c) => c,
            Self::Rgb(b) => &b.config,
            Self::Indexed(b) => &b.config,
        }
    }
    pub(crate) fn last_anime(&self) -> i32 {
        match self {
            Self::Disabled(_) => 0,
            Self::Rgb(b) => b.last_anime,
            Self::Indexed(b) => b.last_anime,
        }
    }
    pub(crate) fn set_tag(&mut self, tag: &[u8]) {
        match self {
            Self::Disabled(c) => c.set_tag(tag),
            Self::Rgb(b) => b.config.set_tag(tag),
            Self::Indexed(b) => b.config.set_tag(tag),
        }
    }
    pub(crate) fn buffer(&self, number: i32) -> Result<Option<(NativePictureViews, bool)>> {
        Ok(match self {
            Self::Disabled(_) => None,
            Self::Rgb(bank) => bank.buffer(number)?.and_then(|c| {
                c.storage.as_ref().map(|s| {
                    let mut view = s.clone();
                    view.has_mask = c.mask_exists;
                    (view, c.load_ok)
                })
            }),
            Self::Indexed(bank) => bank
                .buffer(number)?
                .and_then(|c| c.storage.as_ref().map(|s| (s.native_views(), c.load_ok))),
        })
    }
}
enum CreateWork {
    Disabled(AnimationConfig),
    Rgb(rgb::BlankPreparation),
    Indexed(indexed::BlankPreparation),
}
pub(crate) struct Creation {
    config: AnimationConfig,
    palette: PngPalette,
    work: CreateWork,
    ready: Option<Bank>,
}
impl Creation {
    pub(crate) fn begin(
        client: ClientId,
        context: &ContextData,
        config: AnimationConfig,
        size: [u32; 2],
        palette: PngPalette,
        host: &mut dyn HostServices,
    ) -> Result<Self> {
        config.validate()?;
        let initial = config.clone();
        let initial_palette = palette.clone();
        let work = if !config.enabled {
            CreateWork::Disabled(config)
        } else if config.indexed() {
            CreateWork::Indexed(indexed::BlankPreparation::begin(
                client, context, config, size, palette,
            )?)
        } else {
            CreateWork::Rgb(rgb::BlankPreparation::begin(
                client, context, config, size, host,
            )?)
        };
        Ok(Self {
            config: initial,
            palette: initial_palette,
            work,
            ready: None,
        })
    }
    pub(crate) fn config(&self) -> &AnimationConfig {
        &self.config
    }
    pub(crate) fn poll(&mut self, host: &mut dyn HostServices) -> Result<Option<Bank>> {
        if let Some(ready) = &self.ready {
            return Ok(Some(ready.clone()));
        }
        let mut ready = match &mut self.work {
            CreateWork::Disabled(c) => Some(Bank::Disabled(c.clone())),
            CreateWork::Rgb(work) => work.poll(host)?.map(Bank::Rgb),
            CreateWork::Indexed(work) => work.poll(host)?.map(Bank::Indexed),
        };
        if let Some(bank) = ready.as_mut() {
            bank.set_palette(self.palette.clone());
        }
        if let Some(ready) = &ready {
            self.ready = Some(ready.clone());
        }
        Ok(ready)
    }
}
#[derive(Clone)]
pub(crate) struct Painted {
    pub bank: Bank,
    pub image: Option<Arc<dyn ImageResource>>,
}
enum FrameBackend {
    Rgb(rgb::FrameWork),
    Indexed(indexed::FrameWork),
}
pub(crate) struct FrameWork {
    work: FrameBackend,
    ready: Option<Painted>,
    pub parameters: [i32; 17],
}
impl FrameWork {
    pub(crate) fn begin(
        bank: &Bank,
        context: ContextData,
        mut parameters: [i32; 17],
        owner: u64,
        count: i32,
        frames: i32,
        rectangles: NativeRects,
        canvas: [u32; 2],
        background: bool,
        anime_off: i32,
        base: ImageInput,
    ) -> Result<Option<Self>> {
        let selector = crate::native_animation::AnimationClass {
            config: bank.config().clone(),
            last_anime: bank.last_anime(),
            ..Default::default()
        };
        let Some(selection) = selector.select(&mut parameters, count, anime_off)? else {
            return Ok(None);
        };
        let work = match bank {
            Bank::Disabled(_) => return Ok(None),
            Bank::Rgb(bank) => FrameBackend::Rgb(rgb::FrameWork::begin(
                bank, context, selection, owner, count, frames, rectangles, canvas, background,
                base,
            )?),
            Bank::Indexed(bank) => FrameBackend::Indexed(indexed::FrameWork::begin(
                bank, context, selection, owner, count, frames, rectangles, canvas, background,
                base,
            )?),
        };
        Ok(Some(Self {
            work,
            ready: None,
            parameters,
        }))
    }
    pub(crate) fn poll(&mut self, host: &mut dyn HostServices) -> Result<Option<Painted>> {
        if let Some(ready) = &self.ready {
            return Ok(Some(ready.clone()));
        }
        let ready = match &mut self.work {
            FrameBackend::Rgb(work) => work.poll(host)?.map(|r| Painted {
                bank: Bank::Rgb(r.bank),
                image: r.image,
            }),
            FrameBackend::Indexed(work) => work.poll(host)?.map(|r| Painted {
                bank: Bank::Indexed(r.bank),
                image: r.image,
            }),
        };
        if let Some(ready) = &ready {
            self.ready = Some(ready.clone());
        }
        Ok(ready)
    }
}
#[derive(Clone)]
pub(crate) struct LoadedBuffer {
    pub bank: Bank,
    pub returned: bool,
}
enum LoadBackend {
    Rgb(rgb::BufferLoad),
    Indexed(indexed::BufferLoad),
}
pub(crate) struct BufferLoad {
    work: LoadBackend,
    ready: Option<LoadedBuffer>,
}
impl BufferLoad {
    pub(crate) fn begin(
        bank: &Bank,
        context: &ContextData,
        number: i32,
        owner: u64,
        name: &[u8],
    ) -> Result<Option<Self>> {
        let work = match bank {
            Bank::Disabled(_) => return Ok(None),
            Bank::Rgb(bank) => {
                LoadBackend::Rgb(rgb::BufferLoad::begin(bank, context, number, owner, name)?)
            }
            Bank::Indexed(bank) => LoadBackend::Indexed(indexed::BufferLoad::begin(
                bank, context, number, owner, name,
            )?),
        };
        Ok(Some(Self { work, ready: None }))
    }
    pub(crate) fn poll(&mut self, host: &mut dyn HostServices) -> Result<Option<LoadedBuffer>> {
        if let Some(ready) = &self.ready {
            return Ok(Some(ready.clone()));
        }
        let ready = match &mut self.work {
            LoadBackend::Rgb(work) => work.poll(host)?.map(|r| LoadedBuffer {
                bank: Bank::Rgb(r.bank),
                returned: r.returned,
            }),
            LoadBackend::Indexed(work) => work.poll(host)?.map(|r| LoadedBuffer {
                bank: Bank::Indexed(r.bank),
                returned: r.returned,
            }),
        };
        if let Some(ready) = &ready {
            self.ready = Some(ready.clone());
        }
        Ok(ready)
    }
}
