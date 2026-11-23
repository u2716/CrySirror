// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Chart grow-radio/new-mark retained preparation and ordered shared painting.
//! The owner supplies an immutable lower painter prefix and commits candidate
//! clocks only after this job's image completes. No adapter reads pixels.
use crate::{
    image_bridge::{NativePictureViews, PicturePreparation},
    native_chart::ChartNewMark,
    native_radio_group::RadioGroup,
    resource_bridge::{NativeImageSource, NativeResources},
};
use anyhow::{Result, ensure};
use crysirror_core::{
    image_processing::{ImageInput, ImageJob, ImageProcessState, ImageResource},
    service::HostServices,
    state::ClientId,
};
use crysirror_tasks::{Task, TaskPool};
use std::{collections::VecDeque, sync::Arc};

#[derive(Clone, Copy)]
enum Identity {
    Grow(usize),
    Mark,
}
struct Source {
    identity: Identity,
    source: Option<NativeImageSource>,
}
/// One CPicture per grow group is reloaded for its current selected state,
/// rather than retaining two independent per-state image generations.
#[derive(Clone)]
pub(crate) struct Pictures {
    pub grow: Vec<Option<NativePictureViews>>,
    pub mark: Option<NativePictureViews>,
}
pub(crate) struct Preparation {
    client: ClientId,
    pool: TaskPool,
    read: Option<Task<Vec<Source>>>,
    queued: VecDeque<Source>,
    decode: Option<(Identity, PicturePreparation)>,
    candidate: Pictures,
    pub api_calls: u64,
}
impl Preparation {
    pub(crate) fn begin(
        client: ClientId,
        resources: NativeResources,
        pool: TaskPool,
        radio: &RadioGroup,
        mark: Option<&ChartNewMark>,
        previous: Option<&Pictures>,
        mark_lookup: Option<crate::native_resources::ImageLookup>,
    ) -> Result<Self> {
        ensure!(
            client.0 != 0 && resources.client() == client,
            "NNN chart skin file-view scope"
        );
        let count = radio.spec.buttons.len();
        let mut requests = Vec::with_capacity(count + usize::from(mark.is_some()));
        for i in 0..count {
            let (spec, _) = radio.active(i)?;
            requests.push((Identity::Grow(i), spec.button.source.clone().ok_or_else(||anyhow::anyhow!("NNN chart grow button without filename requires its original picture-slot owner"))?));
        }
        if mark.is_some() {
            let lookup =
                mark_lookup.ok_or_else(|| anyhow::anyhow!("NNN chart new-mark lookup absent"))?;
            requests.push((Identity::Mark, lookup));
        }
        let read = pool.spawn(move |cancel| {
            let mut out = Vec::with_capacity(requests.len());
            let mut bytes = 0u64;
            for (identity, lookup) in requests {
                cancel.check()?;
                let source = resources.optional_image_source(&lookup, &cancel)?;
                if let Some(image) = &source {
                    bytes = bytes
                        .checked_add(
                            u64::from(image.size[0]) * u64::from(image.size[1]) * 12
                                + image.encoded_len as u64,
                        )
                        .ok_or_else(|| anyhow::anyhow!("NNN chart skin memory overflow"))?;
                    ensure!(bytes <= 256 << 20, "NNN chart skin resident memory budget");
                }
                out.push(Source { identity, source });
            }
            Ok(out)
        })?;
        let candidate = previous
            .filter(|p| p.grow.len() == count)
            .cloned()
            .unwrap_or(Pictures {
                grow: vec![None; count],
                mark: None,
            });
        Ok(Self {
            client,
            pool,
            read: Some(read),
            queued: VecDeque::new(),
            decode: None,
            candidate,
            api_calls: 0,
        })
    }
    pub(crate) fn poll(&mut self, host: &mut dyn HostServices) -> Result<Option<Pictures>> {
        if let Some(read) = self.read.as_mut() {
            let Some(sources) = read.try_take()? else {
                return Ok(None);
            };
            self.read = None;
            self.queued = sources.into();
        }
        // Resource latency is not a Calcu/Print step. Retain both the decoder
        // and the old successful CPicture when an optional reload is missing.
        if let Some((identity, decode)) = self.decode.as_mut() {
            let before = decode.api_calls;
            let ready = decode.poll_detached(host)?;
            self.api_calls += decode.api_calls - before;
            let Some(picture) = ready else {
                return Ok(None);
            };
            match identity {
                Identity::Grow(i) => self.candidate.grow[*i] = Some(picture),
                Identity::Mark => self.candidate.mark = Some(picture),
            }
            self.decode = None;
        }
        while let Some(next) = self.queued.pop_front() {
            if let Some(source) = next.source {
                let decode = PicturePreparation::detached(self.client, source, &self.pool, host)?;
                self.api_calls += decode.api_calls;
                self.decode = Some((next.identity, decode));
                return Ok(None);
            }
        }
        Ok(Some(self.candidate.clone()))
    }
}
#[derive(Clone)]
pub(crate) struct Frame {
    pub image: Arc<dyn ImageResource>,
    pub radio: RadioGroup,
    pub mark: Option<ChartNewMark>,
}
enum Draw {
    Button(
        crate::native_title::NativeButtonSpec,
        crate::native_title::NativeButtonState,
        NativePictureViews,
    ),
    Mark(
        crate::native_anime_parts::AnimePartSpec,
        crate::native_anime_parts::PartPrint,
        NativePictureViews,
    ),
}
pub(crate) struct FrameWork {
    client: ClientId,
    size: [u32; 2],
    job: Option<ImageJob>,
    current: Arc<dyn ImageResource>,
    draws: VecDeque<Draw>,
    radio: RadioGroup,
    mark: Option<ChartNewMark>,
    ready: Option<Frame>,
}
impl FrameWork {
    pub(crate) fn begin(
        client: ClientId,
        prefix: Arc<dyn ImageResource>,
        mut radio: RadioGroup,
        mut mark: Option<ChartNewMark>,
        pictures: &Pictures,
        host: &mut dyn HostServices,
    ) -> Result<Self> {
        let size = prefix.size();
        ensure!(
            prefix.client() == client && pictures.grow.len() == radio.spec.buttons.len(),
            "NNN chart decoration frame scope"
        );
        let mut draws = VecDeque::new();
        for (i, picture) in pictures.grow.iter().enumerate() {
            if !radio.exists(i)? {
                continue;
            }
            if let Some(picture) = picture {
                let (spec, state) = radio.active(i)?;
                draws.push_back(Draw::Button(
                    spec.button.clone(),
                    state.clone(),
                    picture.clone(),
                ));
            }
            // The CPicture object exists even when its initial LoadDWQ fails;
            // native CommonButton::Print still reaches Anime after no-op Blt.
            radio.printed(i, true)?;
        }
        if let Some(mark) = mark.as_mut() {
            if let Some(picture) = &pictures.mark {
                for i in 0..pictures.grow.len() {
                    let (spec, _) = radio.active(i)?;
                    if let Some(print) = mark.print(i, spec.button.origin)? {
                        draws.push_back(Draw::Mark(
                            mark.clock.spec.clone(),
                            print,
                            picture.clone(),
                        ));
                    }
                }
            }
            mark.printed()?;
        }
        let mut work = Self {
            client,
            size,
            job: None,
            current: prefix,
            draws,
            radio,
            mark,
            ready: None,
        };
        work.admit(host)?;
        Ok(work)
    }
    fn admit(&mut self, host: &mut dyn HostServices) -> Result<()> {
        // A bounded micro-batch stays below the engine's graph-depth/node
        // budget even for the original maximum radio count. Each completed
        // batch becomes one immutable resource, not a nested unbounded graph.
        let mut input = ImageInput::Resource(self.current.clone());
        for _ in 0..6 {
            let Some(draw) = self.draws.pop_front() else {
                break;
            };
            let next = match draw {
                Draw::Button(spec, state, picture) => crate::native_button_image::image_input(
                    &spec,
                    &state,
                    &picture,
                    self.size,
                    input.clone(),
                )?,
                Draw::Mark(spec, print, picture) => crate::native_anime_parts_image::image_input(
                    &spec,
                    print,
                    &picture,
                    self.size,
                    input.clone(),
                )?,
            };
            if let Some(next) = next {
                input = next;
            }
        }
        self.job = Some(
            host.image_metadata(self.client)?
                .begin_process(input, b"[]".as_slice().into())?,
        );
        Ok(())
    }
    pub(crate) fn poll(&mut self, host: &mut dyn HostServices) -> Result<Option<Frame>> {
        if let Some(ready) = &self.ready {
            return Ok(Some(ready.clone()));
        }
        match self
            .job
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("NNN chart skin image job absent"))?
            .poll()?
        {
            ImageProcessState::Pending => Ok(None),
            ImageProcessState::Failed(error) => anyhow::bail!("NNN chart skin painting: {error}"),
            ImageProcessState::Ready(image) => {
                ensure!(
                    image.client() == self.client && image.size() == self.size,
                    "NNN chart skin result scope"
                );
                self.current = image;
                self.job = None;
                if !self.draws.is_empty() {
                    self.admit(host)?;
                    return Ok(None);
                }
                let frame = Frame {
                    image: self.current.clone(),
                    radio: self.radio.clone(),
                    mark: self.mark.clone(),
                };
                self.ready = Some(frame.clone());
                Ok(Some(frame))
            }
        }
    }
}
