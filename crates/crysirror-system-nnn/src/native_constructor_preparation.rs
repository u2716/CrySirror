// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! One retained admission for constructor file reads, shared image preparation,
//! CPU particle state, and brush publication. Nothing mutates the live random
//! stream until the owner commits Ready after every dependency has completed.
use crate::{
    native_constructor_assets::{Preparation as AssetsPreparation, Ready as AssetReady},
    native_effect_constructors::{Constructed, Construction, ConstructorProfile},
    native_png_palette::PngPalette,
    native_resources::ResourceNames,
    resource_bridge::NativeResources,
};
use anyhow::{Context, Result, ensure};
use crysirror_core::{
    image_processing::{ImageJob, ImageProcessState, ImageResource},
    service::HostServices,
    state::ClientId,
};
use crysirror_image::temporal_pattern::PatternRng;
use crysirror_tasks::TaskPool;
use std::sync::Arc;
#[derive(Clone)]
pub(crate) struct Ready {
    pub classes: Constructed,
    pub assets: AssetReady,
    pub brush_atlas: Arc<dyn ImageResource>,
}
pub(crate) struct Preparation {
    client: ClientId,
    profile: ConstructorProfile,
    pool: TaskPool,
    canvas: [u32; 2],
    random: PatternRng,
    assets: AssetsPreparation,
    prepared_assets: Option<AssetReady>,
    construction: Option<Construction>,
    classes: Option<Constructed>,
    brush_job: Option<ImageJob>,
    ready: Option<Ready>,
    failure: Option<String>,
}
impl Preparation {
    pub(crate) fn begin(
        profile: ConstructorProfile,
        pool: &TaskPool,
        canvas: [u32; 2],
        resources: NativeResources,
        names: ResourceNames,
        levels: u32,
        random: PatternRng,
        palette: PngPalette,
    ) -> Result<Self> {
        random.validate()?;
        ensure!(
            canvas.iter().all(|n| (1..=16384).contains(n)),
            "NNN constructor preparation canvas"
        );
        let client = resources.client();
        let assets = AssetsPreparation::begin(client, pool, resources, names, levels, palette)?;
        Ok(Self {
            client,
            profile,
            pool: pool.clone(),
            canvas,
            random,
            assets,
            prepared_assets: None,
            construction: None,
            classes: None,
            brush_job: None,
            ready: None,
            failure: None,
        })
    }
    pub(crate) fn poll(&mut self, host: &mut dyn HostServices) -> Result<Option<Ready>> {
        if let Some(error) = &self.failure {
            anyhow::bail!("NNN effect constructors: {error}")
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
        if self.prepared_assets.is_none() {
            let Some(assets) = self.assets.poll(host)? else {
                return Ok(None);
            };
            self.construction = Some(match self.profile {
                ConstructorProfile::Mugen2Legacy => Construction::mugen2_legacy(
                    &self.pool,
                    self.canvas,
                    assets.assets.clone(),
                    self.random.clone(),
                )?,
                ConstructorProfile::Source32 => Construction::source32_software(
                    &self.pool,
                    self.canvas,
                    assets.assets.clone(),
                    self.random.clone(),
                )?,
            });
            self.prepared_assets = Some(assets);
        }
        if self.classes.is_none() {
            let Some(classes) = self
                .construction
                .as_mut()
                .context("NNN constructor state task absent")?
                .poll()?
            else {
                return Ok(None);
            };
            self.brush_job = Some(
                host.image_metadata(self.client)?
                    .begin_packed_pixels(classes.bath_template.clone())?,
            );
            self.classes = Some(classes);
            self.construction = None;
        }
        match self
            .brush_job
            .as_mut()
            .context("NNN constructor brush dependency absent")?
            .poll()?
        {
            ImageProcessState::Pending => Ok(None),
            ImageProcessState::Failed(error) => {
                anyhow::bail!("NNN constructor brush image: {error}")
            }
            ImageProcessState::Ready(brush_atlas) => {
                ensure!(
                    brush_atlas.client() == self.client && brush_atlas.size() == [128, 384],
                    "NNN constructor brush scope/size"
                );
                let ready = Ready {
                    classes: self
                        .classes
                        .take()
                        .context("NNN constructed classes absent")?,
                    assets: self
                        .prepared_assets
                        .take()
                        .context("NNN constructed assets absent")?,
                    brush_atlas,
                };
                self.brush_job = None;
                self.ready = Some(ready.clone());
                Ok(Some(ready))
            }
        }
    }
}
