// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Worker-read native selection atlases, prepared and retained by shared image
//! services before the UI can publish a special selection generation.
use super::*;
use crate::native_ui::special::{Atlas, Policy};
struct Input {
    source: Option<ImageInput>,
    effects: Arc<[u8]>,
    job: Option<ImageJob>,
    ready: Option<Atlas>,
}
pub(super) struct Special {
    setup: crate::config::Names,
    variables: crate::config::Names,
    inputs: Vec<Input>,
}
impl Special {
    pub fn prepare(
        config: &crate::native_bootstrap::NativeConfiguration,
        services: &PreparedNativeServices,
        resources: &NativeResources,
        cancel: &Cancellation,
    ) -> Result<Option<Self>> {
        let setup = &services.selection_setup;
        let count = setup.integer(b"useSpecialNumber")?.unwrap_or(0);
        ensure!(
            (0..=64).contains(&count),
            "NNN special selection count budget"
        );
        if count == 0 {
            return Ok(None);
        }
        let window = setup
            .parameter(b"filenameSpecialWindow")
            .context("NNN special selection window filename absent")?;
        let cursor = setup
            .parameter(b"filenameSpecialCursor")
            .or_else(|| setup.parameter(b"cursorFileName"))
            .or(Some(b"ta_select01"));
        let levels = u32::try_from(config.setup.integer(b"dwqPackLevel")?.unwrap_or(0))?;
        let mut inputs = Vec::new();
        for name in std::iter::once(window).chain(cursor) {
            cancel.check()?;
            let path = [b"sys/".as_slice(), name].concat();
            let decoded =
                resources.image_source(&config.resources.picture(&path, levels)?, cancel)?;
            let effects = if decoded.has_mask {
                vec![]
            } else {
                vec![crysirror_image::effects::ImageEffect::SpriteAlpha {
                    spec: crysirror_image::sprite_alpha::SpriteAlpha::CornerKey { right: false },
                }]
            };
            inputs.push(Input {
                source: Some(decoded.input),
                effects: serde_json::to_vec(&effects)?.into(),
                job: None,
                ready: None,
            });
        }
        Ok(Some(Self {
            setup: setup.clone(),
            variables: config.variable_names.clone(),
            inputs,
        }))
    }
    pub fn poll(
        &mut self,
        client: ClientId,
        host: &mut dyn HostServices,
        published: &mut Vec<String>,
    ) -> Result<bool> {
        for (i, input) in self.inputs.iter_mut().enumerate() {
            if input.ready.is_some() {
                continue;
            }
            if let Some(source) = input.source.take() {
                input.job = Some(
                    host.image_metadata(client)?
                        .begin_process(source, input.effects.clone())?,
                );
            }
            match input
                .job
                .as_mut()
                .context("NNN special selection atlas job absent")?
                .poll()?
            {
                ImageProcessState::Pending => return Ok(false),
                ImageProcessState::Failed(error) => {
                    anyhow::bail!("NNN special selection atlas: {error}")
                }
                ImageProcessState::Ready(image) => {
                    let size = image.size();
                    let path = host
                        .publish_image_resource(client, 0x66030000 + i as u32, image.clone())?
                        .path
                        .relative()
                        .to_owned();
                    published.push(path.clone());
                    input.ready = Some(Atlas {
                        path,
                        size,
                        image: Some(image),
                    });
                    input.job = None;
                }
            }
        }
        Ok(true)
    }
    pub fn finish(self) -> Result<Arc<Policy>> {
        let mut inputs = self.inputs.into_iter();
        let window = inputs
            .next()
            .and_then(|i| i.ready)
            .context("NNN special selection window unprepared")?;
        let cursor = inputs
            .next()
            .map(|i| i.ready.context("NNN special selection cursor unprepared"))
            .transpose()?;
        Ok(Arc::new(Policy {
            setup: self.setup,
            variables: self.variables,
            window,
            cursor,
        }))
    }
}
