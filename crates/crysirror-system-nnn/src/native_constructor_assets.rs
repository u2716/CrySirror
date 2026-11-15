// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Constructor CPicture resources in their original admission order. The
//! global PNG palette is native session state, not a worker-completion race.
//! Shared codecs and image services own every byte decode/key/colour expansion.
use crate::{
    image_bridge::{NativePictureViews, PicturePreparation},
    native_effect_constructors::ConstructorAssets,
    native_indexed_picture::{IndexedViews, Preparation as IndexedPreparation},
    native_indexed_source::{self, Decoded},
    native_png_palette::PngPalette,
    native_resources::ResourceNames,
    resource_bridge::{NativeImageSource, NativeResources},
};
use anyhow::{Context, Result, ensure};
use crysirror_core::{service::HostServices, state::ClientId};
use crysirror_image::{linear_byte_storage::LinearByteStorage, png_header::PngHeader};
use crysirror_tasks::{Cancellation, Task, TaskPool};

// CAllEffect's allocation order is not its final numeric Init loop. In both
// the complete Source32 constructor and MUGEN2 0x4c3520, these four CPicture
// probes occur FLOWER -> KIRA -> SNOW -> SNOW2. SNOW and SNOW2 really load two
// separate CPictures even though their names match; do not merge their probes.
const PROBES: [(&[u8], Option<&[u8]>); 4] = [
    (b"sys/ta_eff_saku", Some(b"sys/eff_saku")),
    (b"sys/kirakira", None),
    (b"sys/ta_eff_snow", Some(b"sys/eff_snow")),
    (b"sys/ta_eff_snow", Some(b"sys/eff_snow")),
];
enum Plan {
    Missing,
    Rgb(NativeImageSource),
    Indexed(Decoded),
}
struct ReadResult {
    plans: Vec<Plan>,
    palette: PngPalette,
}
fn read(
    resources: &NativeResources,
    names: &ResourceNames,
    levels: u32,
    mut palette: PngPalette,
    cancel: &Cancellation,
) -> Result<ReadResult> {
    let mut plans = Vec::with_capacity(PROBES.len());
    for (primary, fallback) in PROBES {
        cancel.check()?;
        let lookup = names.picture(primary, levels)?;
        if let Some(source) = resources.optional_image_source(&lookup, cancel)? {
            // LoadDWQ's ordinary PNG path mutates static CPngLoader state too.
            // Observe PLTE before the next constructor; do not DEFLATE the RGB
            // bitmap twice just to obtain this scalar native loader history.
            let (_, bytes) = native_indexed_source::encoded(source.input.clone())?;
            let info = crysirror_formats_blackcyc::Dwq::metadata(&bytes)?;
            if info.pack == 8 {
                let header = PngHeader::read(
                    bytes.get(64..).context("NNN constructor PNG payload")?,
                    &|| cancel.check(),
                )?;
                ensure!(header.size == info.size, "NNN constructor PNG size");
                palette.observe_header(&header)?;
            }
            plans.push(Plan::Rgb(source));
            continue;
        }
        let Some(fallback) = fallback else {
            plans.push(Plan::Missing);
            continue;
        };
        let lookup = names.picture(fallback, levels)?;
        let Some(source) = resources.optional_image_source(&lookup, cancel)? else {
            plans.push(Plan::Missing);
            continue;
        };
        let decoded = native_indexed_source::decode_source(
            resources,
            source,
            palette.clone(),
            &LinearByteStorage::default(),
            None,
            cancel,
        )?;
        palette = decoded.palette.clone();
        plans.push(Plan::Indexed(decoded));
    }
    cancel.check()?;
    Ok(ReadResult { plans, palette })
}
#[derive(Clone)]
pub(crate) struct Ready {
    pub assets: ConstructorAssets,
    pub kira: Option<NativePictureViews>,
    /// Commit with the whole constructor admission, never at task completion.
    pub palette: PngPalette,
    /// Retain real index storage/palette for later native reload/save policy;
    /// expanded RGBA is only the shared output view, not a replacement buffer.
    pub indexed: [Option<Decoded>; 4],
}
enum Decode {
    Rgb(PicturePreparation),
    Indexed(IndexedPreparation),
}
pub(crate) struct Preparation {
    client: ClientId,
    pool: TaskPool,
    task: Option<Task<ReadResult>>,
    plans: Option<std::collections::VecDeque<Plan>>,
    palette: Option<PngPalette>,
    decode: Option<Decode>,
    pending_indexed: Option<Decoded>,
    views: Vec<Option<NativePictureViews>>,
    indexed: Vec<Option<Decoded>>,
    ready: Option<Ready>,
    failure: Option<String>,
}
impl Preparation {
    pub(crate) fn begin(
        client: ClientId,
        pool: &TaskPool,
        resources: NativeResources,
        names: ResourceNames,
        levels: u32,
        palette: PngPalette,
    ) -> Result<Self> {
        ensure!(
            client.0 != 0 && resources.client() == client,
            "NNN constructor asset client"
        );
        ensure!(levels <= 1024, "NNN constructor DWQ level budget");
        let task = pool.spawn(move |cancel| read(&resources, &names, levels, palette, &cancel))?;
        Ok(Self {
            client,
            pool: pool.clone(),
            task: Some(task),
            plans: None,
            palette: None,
            decode: None,
            pending_indexed: None,
            views: Vec::with_capacity(4),
            indexed: Vec::with_capacity(4),
            ready: None,
            failure: None,
        })
    }
    pub(crate) fn poll(&mut self, host: &mut dyn HostServices) -> Result<Option<Ready>> {
        if let Some(error) = &self.failure {
            anyhow::bail!("NNN constructor assets: {error}")
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
    fn advance(&mut self, host: &mut dyn HostServices) -> Result<Option<Ready>> {
        if self.plans.is_none() {
            let Some(read) = self
                .task
                .as_mut()
                .context("NNN constructor read task")?
                .try_take()?
            else {
                return Ok(None);
            };
            self.task = None;
            self.palette = Some(read.palette);
            self.plans = Some(read.plans.into());
        }
        loop {
            if let Some(decode) = &mut self.decode {
                let views = match decode {
                    Decode::Rgb(decode) => decode.poll_detached(host)?,
                    Decode::Indexed(decode) => {
                        decode.poll(host)?.map(|v: IndexedViews| v.unmasked_views())
                    }
                };
                let Some(mut views) = views else {
                    return Ok(None);
                };
                if let Some(decoded) = &self.pending_indexed {
                    views.source_clip = decoded.source_clip;
                }
                ensure!(
                    views.raw.client() == self.client,
                    "NNN constructor view client"
                );
                self.views.push(Some(views));
                self.indexed.push(self.pending_indexed.take());
                self.decode = None;
            }
            let Some(plan) = self.plans.as_mut().unwrap().pop_front() else {
                ensure!(
                    self.views.len() == 4 && self.indexed.len() == 4,
                    "NNN constructor asset result count"
                );
                let ready = Ready {
                    assets: ConstructorAssets {
                        flower: self.views[0].clone(),
                        snow: self.views[2].clone(),
                        snow2: self.views[3].clone(),
                    },
                    kira: self.views[1].clone(),
                    palette: self
                        .palette
                        .take()
                        .context("NNN constructor palette candidate")?,
                    indexed: std::mem::take(&mut self.indexed)
                        .try_into()
                        .map_err(|_| anyhow::anyhow!("NNN constructor indexed count"))?,
                };
                self.ready = Some(ready.clone());
                return Ok(Some(ready));
            };
            match plan {
                Plan::Missing => {
                    self.views.push(None);
                    self.indexed.push(None);
                }
                Plan::Rgb(source) => {
                    self.decode = Some(Decode::Rgb(PicturePreparation::detached(
                        self.client,
                        source,
                        &self.pool,
                        host,
                    )?));
                }
                Plan::Indexed(decoded) => {
                    self.decode = Some(Decode::Indexed(IndexedPreparation::begin(
                        self.client,
                        decoded.plane.clone(),
                        host,
                    )?));
                    self.pending_indexed = Some(decoded);
                }
            }
        }
    }
}
