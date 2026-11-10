//! Worker-only source reads; owner-side shared image completion precedes the
//! native optional-button class being connected to the VM.
use super::*;
use crate::{
    native_film_button::{Piece, Policy, Presenter},
    native_title::NativeButtonSpec,
};
struct Input {
    source: Option<ImageInput>,
    effects: Arc<[u8]>,
    job: Option<ImageJob>,
    piece: Option<Piece>,
    rect: [i32; 4],
}
pub(super) struct FilmButton {
    spec: NativeButtonSpec,
    inputs: Vec<Input>,
    client: ClientId,
}
impl FilmButton {
    pub fn prepare(
        config: &crate::native_bootstrap::NativeConfiguration,
        storage: &dyn ClientStorage,
        resources: &NativeResources,
        client: ClientId,
        cancel: &Cancellation,
    ) -> Result<Option<Self>> {
        if config
            .setup
            .integer(b"skipFilmButtonCreateFlag")?
            .unwrap_or(0)
            == 0
        {
            return Ok(None);
        }
        let table = native_load::table(config, storage, "skipFilmButton", cancel)?;
        let spec = NativeButtonSpec::from_named_setup(&table, "skipFilmButton", config)?;
        let n = |key: &str, default: i32| -> Result<i32> {
            Ok(table.integer(key.as_bytes())?.unwrap_or(default))
        };
        let count = n("addPartsNumber", 0)?;
        ensure!(
            (0..=64).contains(&count),
            "NNN film-button decoration budget"
        );
        let levels = u32::try_from(config.setup.integer(b"dwqPackLevel")?.unwrap_or(0))?;
        let mut sources = vec![(
            spec.source.clone(),
            [spec.origin[0], spec.origin[1], spec.size[0], spec.size[1]],
        )];
        for i in 1..=count {
            let name = table
                .parameter(format!("kazari{i}FileName").as_bytes())
                .unwrap_or(b"ta_scene_bt1");
            let size = [
                n(&format!("kazari{i}SizeX"), 16)?,
                n(&format!("kazari{i}SizeY"), 16)?,
            ];
            ensure!(
                size.iter().all(|n| (1..=8192).contains(n)),
                "NNN film-button decoration bounds"
            );
            let point = [
                spec.origin[0]
                    .checked_add(n(&format!("kazari{i}PrintX"), 0)?)
                    .context("NNN film-button decoration x overflow")?,
                spec.origin[1]
                    .checked_add(n(&format!("kazari{i}PrintY"), 0)?)
                    .context("NNN film-button decoration y overflow")?,
            ];
            sources.push((
                config
                    .resources
                    .picture(&[b"sys/".as_slice(), name].concat(), levels)?,
                [point[0], point[1], size[0], size[1]],
            ));
        }
        let mut inputs = Vec::new();
        for (lookup, rect) in sources {
            cancel.check()?;
            let image = resources.image_source(&lookup, cancel)?;
            let effects = if image.has_mask {
                vec![]
            } else {
                vec![crysirror_image::effects::ImageEffect::SpriteAlpha {
                    spec: crysirror_image::sprite_alpha::SpriteAlpha::CornerKey { right: false },
                }]
            };
            inputs.push(Input {
                source: Some(image.input),
                effects: serde_json::to_vec(&effects)?.into(),
                job: None,
                piece: None,
                rect,
            });
        }
        Ok(Some(Self {
            spec,
            inputs,
            client,
        }))
    }
    pub fn poll(
        &mut self,
        host: &mut dyn HostServices,
        published: &mut Vec<String>,
    ) -> Result<bool> {
        for (index, input) in self.inputs.iter_mut().enumerate() {
            if input.piece.is_some() {
                continue;
            }
            if let Some(source) = input.source.take() {
                input.job = Some(
                    host.image_metadata(self.client)?
                        .begin_process(source, input.effects.clone())?,
                );
            }
            let image = match input
                .job
                .as_mut()
                .context("NNN film-button image job absent")?
                .poll()?
            {
                ImageProcessState::Pending => return Ok(false),
                ImageProcessState::Failed(error) => anyhow::bail!("NNN film-button image: {error}"),
                ImageProcessState::Ready(image) => image,
            };
            let size = image.size();
            ensure!(
                size[0] >= input.rect[2] as u32 && size[1] >= input.rect[3] as u32,
                "NNN film-button sprite bounds"
            );
            let path = host
                .publish_image_resource(self.client, 0x66060100 + index as u32, image.clone())?
                .path
                .relative()
                .to_owned();
            published.push(path.clone());
            input.piece = Some(Piece {
                path,
                size,
                image,
                rect: input.rect,
            });
            input.job = None;
        }
        Ok(true)
    }
    pub fn finish(self) -> Result<Presenter> {
        let pieces = self
            .inputs
            .into_iter()
            .map(|i| i.piece.context("NNN film-button source not prepared"))
            .collect::<Result<Vec<_>>>()?;
        Presenter::new(Arc::new(Policy {
            client: self.client,
            root: 1,
            canvas: SIZE,
            spec: self.spec,
            pieces,
        }))
    }
}
