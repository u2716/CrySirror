// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Retained optional32-bit CPicture loads for effect-owned picture caches.
//! Original FALSE, same-name TRUE and loadOk are distinct; shared workers own
//! reads, decode, RGB/keyed views and retained raw-mask channel preparation.
use crate::{
    image_bridge::{NativePictureViews, PicturePreparation},
    native_picture_cache::{BeginLoad, PictureCache},
    native_resources::ResourceNames,
    resource_bridge::{NativeImageSource, NativeResources},
};
use anyhow::{Context, Result, ensure};
use crysirror_core::{service::HostServices, state::ClientId};
use crysirror_tasks::{Task, TaskPool};

#[derive(Clone)]
pub(crate) struct ContextData {
    pub resources: NativeResources,
    pub names: ResourceNames,
    pub levels: u32,
    pub pool: TaskPool,
}
#[derive(Clone)]
pub(crate) struct Reloaded {
    pub cache: PictureCache<NativePictureViews>,
    pub returned: bool,
    pub png_header:Option<crysirror_image::png_header::PngHeader>,
}
pub(crate) struct Reload {
    png_header:Option<crysirror_image::png_header::PngHeader>,
    client: ClientId,
    pool: TaskPool,
    candidate: PictureCache<NativePictureViews>,
    token: Option<u64>,
    read: Option<Task<Option<NativeImageSource>>>,
    decode: Option<PicturePreparation>,
    ready: Option<Reloaded>,
    failure: Option<String>,
    pub api_calls: u64,
}
impl Reload {
    pub(crate) fn begin(
        context: &ContextData,
        current: &PictureCache<NativePictureViews>,
        owner: u64,
        name: &[u8],
    ) -> Result<Self> {
        let client = context.resources.client();
        ensure!(
            client == current.client() && !current.indexed,
            "NNN RGB cache owner/depth profile"
        );
        let mut candidate = current.clone();
        let disposition = candidate.begin(owner, name, false)?;
        let mut work = Self {
            png_header:None,
            client,
            pool: context.pool.clone(),
            candidate,
            token: None,
            read: None,
            decode: None,
            ready: None,
            failure: None,
            api_calls: 0,
        };
        match disposition {
            BeginLoad::CachedTrue => {
                work.ready = Some(Reloaded {
                    cache: work.candidate.clone(),
                    returned: true,
                    png_header:None,
                })
            }
            BeginLoad::Pending { .. } => {
                anyhow::bail!("NNN pending cached read requires its retained Reload owner")
            }
            BeginLoad::Read { token } => {
                let lookup = context.names.picture(name, context.levels)?;
                let resources = context.resources.clone();
                work.read = Some(context.pool.spawn(move |cancel| {
                    cancel.check()?;
                    let source = resources.optional_image_source(&lookup, &cancel)?;
                    cancel.check()?;
                    Ok(source)
                })?);
                work.token = Some(token);
            }
        }
        Ok(work)
    }
    pub(crate) fn poll(&mut self, host: &mut dyn HostServices) -> Result<Option<Reloaded>> {
        if let Some(error) = &self.failure {
            anyhow::bail!("NNN cached picture load: {error}");
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
    fn advance(&mut self, host: &mut dyn HostServices) -> Result<Option<Reloaded>> {
        if let Some(read) = self.read.as_mut() {
            let Some(source) = read.try_take()? else {
                return Ok(None);
            };
            self.read = None;
            let token = self.token.context("NNN cache read token absent")?;
            let Some(source) = source else {
                self.candidate.missing(token)?;
                let ready = Reloaded {
                    cache: self.candidate.clone(),
                    returned: false,
                    png_header:None,
                };
                self.ready = Some(ready.clone());
                return Ok(Some(ready));
            };
            self.png_header=source.png_header.clone();
            let history = self
                .candidate
                .storage
                .as_ref()
                .and_then(|v| v.mask_storage.clone());
            let decode = PicturePreparation::detached(self.client, source, &self.pool, host)?
                .with_mask_history(history);
            self.api_calls += decode.api_calls;
            self.decode = Some(decode);
        }
        let decode = self
            .decode
            .as_mut()
            .context("NNN cache decode dependency absent")?;
        let before = decode.api_calls;
        let result = decode.poll_detached(host);
        self.api_calls += decode.api_calls - before;
        let Some(picture) = result? else {
            return Ok(None);
        };
        let mask = picture.has_mask;
        self.candidate.loaded(
            self.token.context("NNN cache completion token absent")?,
            picture,
            mask,
        )?;
        self.decode = None;
        let ready = Reloaded {
            cache: self.candidate.clone(),
            returned: true,
            png_header:self.png_header.clone(),
        };
        self.ready = Some(ready.clone());
        Ok(Some(ready))
    }
}
