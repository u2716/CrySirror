//! Cutin window sources are read on the bootstrap worker, decoded by shared
//! image workers and published once. The native printer retains only scoped
//! paths/sizes; it has no pixel buffers, renderer or synchronous disk access.
use super::*;
use crate::native_text::CutinPolicy;
struct Window {
    number: i32,
    input: Option<ImageInput>,
    effects: Arc<[u8]>,
    job: Option<ImageJob>,
}
pub(super) struct Cutin {
    policy: CutinPolicy,
    windows: Vec<Window>,
}
impl Cutin {
    pub fn prepare(
        config: &crate::native_bootstrap::NativeConfiguration,
        services: &PreparedNativeServices,
        storage: &dyn ClientStorage,
        resources: &NativeResources,
        cancel: &Cancellation,
    ) -> Result<Option<Self>> {
        if config.setup.integer(b"useCutinFlag")?.unwrap_or(0) == 0 {
            return Ok(None);
        }
        let bytes = read(
            storage,
            &config.resources.path("spt/cutinChara.xtx")?,
            1 << 20,
            cancel,
        )?
        .unwrap_or_default();
        let mut policy = CutinPolicy::parse(
            &crate::config::Names::parse(&bytes, false)?,
            services
                .print_setup
                .integer(b"cutinNamePrintFlag")?
                .unwrap_or(0)
                != 0,
        )?;
        let numbers = policy.window_numbers()?;
        ensure!(
            numbers.len() <= 64,
            "NNN Cutin window resource count budget"
        );
        ensure!(
            (1..=64).contains(
                &services
                    .print_setup
                    .integer(b"cutinWindowBufferNumber")?
                    .unwrap_or(4)
            ),
            "NNN Cutin window buffer bounds"
        );
        let prefix = services
            .print_setup
            .parameter(b"filenameCutin")
            .unwrap_or(b"ta_cutinwindow");
        let levels = u32::try_from(config.setup.integer(b"dwqPackLevel")?.unwrap_or(0))?;
        let mut windows = Vec::new();
        for number in numbers {
            cancel.check()?;
            let path = [b"sys/".as_slice(), prefix, number.to_string().as_bytes()].concat();
            let lookup = config.resources.picture(&path, levels)?;
            let Some(decoded) = resources.optional_image_source(&lookup, cancel)? else {
                policy.bind_window(number, None);
                continue;
            };
            let effects = if decoded.has_mask {
                vec![]
            } else {
                vec![crysirror_image::effects::ImageEffect::SpriteAlpha {
                    spec: crysirror_image::sprite_alpha::SpriteAlpha::CornerKey { right: false },
                }]
            };
            windows.push(Window {
                number,
                input: Some(decoded.input),
                effects: serde_json::to_vec(&effects)?.into(),
                job: None,
            });
        }
        Ok(Some(Self { policy, windows }))
    }
    pub fn font_sizes(&self, base: i32) -> Result<std::collections::BTreeSet<u16>> {
        self.policy.font_sizes(base)
    }
    pub fn poll(
        &mut self,
        client: ClientId,
        host: &mut dyn HostServices,
        published: &mut Vec<String>,
    ) -> Result<bool> {
        for (index, window) in self.windows.iter_mut().enumerate() {
            if let Some(input) = window.input.take() {
                window.job = Some(
                    host.image_metadata(client)?
                        .begin_process(input, window.effects.clone())?,
                );
            }
            let Some(job) = window.job.as_mut() else {
                continue;
            };
            let image = match job.poll()? {
                ImageProcessState::Pending => return Ok(false),
                ImageProcessState::Failed(error) => {
                    anyhow::bail!("NNN Cutin window decode: {error}")
                }
                ImageProcessState::Ready(image) => image,
            };
            let size = image.size();
            let path = host
                .publish_image_resource(client, 0x66050000 + index as u32, image)?
                .path
                .relative()
                .to_owned();
            published.push(path.clone());
            self.policy.bind_window(window.number, Some((path, size)));
            window.job = None;
        }
        Ok(true)
    }
    pub fn finish(self) -> Result<Arc<CutinPolicy>> {
        ensure!(
            self.windows
                .iter()
                .all(|w| w.input.is_none() && w.job.is_none()),
            "NNN unprepared Cutin window resources"
        );
        Ok(Arc::new(self.policy))
    }
}
