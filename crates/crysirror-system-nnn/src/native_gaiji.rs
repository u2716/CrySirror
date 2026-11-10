// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Original spfont24 cells: 50x48 at twice the logical size. Generic shared
//! colour-key, RGB lookup, nearest resize and box reduction prepare glyph images.
use crate::{
    native_services_bootstrap::PreparedNativeServices,
    resource_bridge::NativeResources,
};
use anyhow::{ensure, Context, Result};
use crysirror_core::image_processing::{ImageCompositeLayer, ImageCompositeMath, ImageResource};
use crysirror_core::{
    image_processing::{ImageInput, ImageJob, ImageProcessState},
    service::HostServices,
    state::ClientId,
};
use crysirror_image::{
    box_reduce::{BoxAlpha, BoxAnchors, BoxArithmetic, BoxReduce},
    effects::ImageEffect,
};
use crysirror_tasks::Cancellation;
use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;
pub(crate) struct Gaiji {
    custom: bool,
    input: Option<ImageInput>,
    font_job: Option<ImageJob>,
    font: Option<Arc<dyn ImageResource>>,
    pending: VecDeque<(u8, u16)>,
    job: Option<(u8, u16, ImageJob)>,
    pub images: BTreeMap<(u8, u16), String>,
}
impl Gaiji {
    pub(crate) fn custom(input: ImageInput, pending: VecDeque<(u8, u16)>) -> Result<Self> {
        ensure!(
            pending.len() <= 2048
                && pending
                    .iter()
                    .all(|(number, size)| (21..=54).contains(number) && (1..=512).contains(size)),
            "NNN user gaiji cell/size budget"
        );
        Ok(Self {
            custom: true,
            input: Some(input),
            font_job: None,
            font: None,
            pending,
            job: None,
            images: BTreeMap::new(),
        })
    }

    pub(crate) fn include_sizes(
        &mut self,
        sizes: impl IntoIterator<Item = u16>,
        services: &PreparedNativeServices,
    ) -> Result<()> {
        let ruby = crate::native_text::RubyProfile::from_game_setup(&services.game_setup)?;
        let mut all = std::collections::BTreeSet::new();
        for size in sizes {
            all.insert(size);
            if let Some(ruby) = &ruby {
                all.insert(u16::try_from(i32::from(size) * ruby.percent / 100)?);
            }
        }
        for size in all {
            ensure!((1..=512).contains(&size), "NNN Cutin gaiji size budget");
            for number in services.inline.gaiji.values().copied() {
                if !self.pending.contains(&(number, size)) {
                    self.pending.push_back((number, size));
                }
            }
        }
        ensure!(self.pending.len() <= 2048, "NNN gaiji image count budget");
        Ok(())
    }
    pub fn prepare(
        config: &crate::native_bootstrap::NativeConfiguration,
        services: &PreparedNativeServices,
        resources: &NativeResources,
        cancel: &Cancellation,
    ) -> Result<Option<Self>> {
        let levels = u32::try_from(config.setup.integer(b"dwqPackLevel")?.unwrap_or(0))?;
        let lookup = config.resources.picture(b"sys/spfont24", levels)?;
        let Some(font) = resources.optional_image_source(&lookup, cancel)? else {
            return Ok(None);
        };
        let body = services.print_setup.integer(b"fontSize")?.unwrap_or(24);
        let name = services.print_setup.integer(b"nameFontSize")?.unwrap_or(20);
        let mut sizes = vec![body, name];
        if let Some(ruby) = crate::native_text::RubyProfile::from_game_setup(&services.game_setup)?
        {
            sizes.push(body * ruby.percent / 100);
        }
        sizes.sort_unstable();
        sizes.dedup();
        let mut pending = VecDeque::new();
        let numbers: std::collections::BTreeSet<_> =
            services.inline.gaiji.values().copied().collect();
        for number in numbers {
            for size in &sizes {
                ensure!((1..=512).contains(size), "NNN gaiji font size budget");
                pending.push_back((number, *size as u16));
            }
        }
        Ok(Some(Self {
            custom: false,
            input: Some(font.input),
            font_job: None,
            font: None,
            pending,
            job: None,
            images: BTreeMap::new(),
        }))
    }
    pub fn poll(
        &mut self,
        client: ClientId,
        host: &mut dyn HostServices,
        published: &mut Vec<String>,
    ) -> Result<bool> {
        if let Some(input) = self.input.take() {
            let effects = vec![
                ImageEffect::SpriteAlpha {
                    spec: crysirror_image::sprite_alpha::SpriteAlpha::ColorKey { color: [0; 3] },
                },
                ImageEffect::RgbLookup {
                    spec: crysirror_image::rgb_lut::RgbLookup::weighted_color_fixed(
                        [255; 3], 0, 1, 0,
                    )?,
                },
            ];
            self.font_job = Some(
                host.image_metadata(client)?
                    .begin_process(input, serde_json::to_vec(&effects)?.into())?,
            );
        }
        if let Some(job) = self.font_job.as_mut() {
            match job.poll()? {
                ImageProcessState::Pending => return Ok(false),
                ImageProcessState::Failed(e) => anyhow::bail!("NNN gaiji sheet: {e}"),
                ImageProcessState::Ready(font) => {
                    self.font = Some(font);
                    self.font_job = None;
                }
            }
        }
        if let Some((number, size, job)) = self.job.as_mut() {
            match job.poll()? {
                ImageProcessState::Pending => return Ok(false),
                ImageProcessState::Failed(e) => anyhow::bail!("NNN gaiji cell: {e}"),
                ImageProcessState::Ready(image) => {
                    let path = host
                        .publish_image_resource(
                            client,
                            0x66020000 + self.images.len() as u32,
                            image,
                        )?
                        .path
                        .relative()
                        .to_owned();
                    self.images.insert((*number, *size), path.clone());
                    published.push(path);
                    self.job = None;
                }
            }
        }
        let Some((number, size)) = self.pending.pop_front() else {
            return Ok(true);
        };
        let font = self.font.as_ref().context("NNN gaiji sheet absent")?;
        ensure!(
            number > 0 && number <= 54 && font.size()[1] >= 48,
            "NNN gaiji cell outside the original sheet"
        );
        let cell = if self.custom {
            number.checked_sub(20)
        } else {
            Some(number - 1)
        };
        let present =
            number <= 50 && cell.is_some_and(|cell| (u32::from(cell) + 1) * 50 <= font.size()[0]);
        if !present {
            // Original Gaiji returns without painting when its horizontal cell
            // is absent. Keep its logical advance and reveal step as blank.
            self.job = Some((
                number,
                size,
                host.image_metadata(client)?.begin_process(
                    ImageInput::Solid {
                        size: [1, 1],
                        color: [0; 4],
                    },
                    b"[]".as_slice().into(),
                )?,
            ));
            return Ok(false);
        }
        let input = ImageInput::Composite {
            base: Box::new(ImageInput::Solid {
                size: [50, 48],
                color: [0; 4],
            }),
            layers: vec![ImageCompositeLayer {
                input: Box::new(ImageInput::Resource(font.clone())),
                source: [i32::from(cell.unwrap()) * 50, 0, 50, 48],
                destination: [0, 0],
                clip: None,
                opacity: 255,
                math: ImageCompositeMath::Copy,
            }],
        };
        let effects = vec![
            ImageEffect::ResizeNearest {
                width: u32::from(size) * 2 + 2,
                height: u32::from(size) * 2,
            },
            ImageEffect::BoxReduce {
                spec: BoxReduce {
                    size: [u32::from(size) + 1, u32::from(size)],
                    source: None,
                    window: [2, 2],
                    anchors: BoxAnchors::Grid { stride: [2, 2] },
                    arithmetic: BoxArithmetic::Mean,
                    alpha: BoxAlpha::Aggregate,
                },
            },
        ];
        self.job = Some((
            number,
            size,
            host.image_metadata(client)?
                .begin_process(input, serde_json::to_vec(&effects)?.into())?,
        ));
        Ok(false)
    }
}
#[cfg(all(test, feature = "runtime"))]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};
    #[test]
    fn native_user_gaiji_uses_custom_sheet_offset_and_retains_blank_unsupported_cells() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("game")).unwrap();
        let project=serde_json::from_value(serde_json::json!({"schema":2,"name":"glyph sheet","content_revision":"1","assets":"game","canvas":[1,1],"clients":[{"name":"native","adapter":"crysirror-script-v1","game":"authored"}]})).unwrap();
        let mut rt = crysirror_runtime::Runtime::with_asset_root(
            project,
            dir.path(),
            &dir.path().join("state"),
            Default::default(),
        )
        .unwrap();
        let input = ImageInput::Composite {
            base: Box::new(ImageInput::Solid {
                size: [350, 48],
                color: [0, 0, 0, 255],
            }),
            layers: vec![ImageCompositeLayer {
                input: Box::new(ImageInput::Solid {
                    size: [50, 48],
                    color: [255; 4],
                }),
                source: [0, 0, 50, 48],
                destination: [250, 0],
                clip: None,
                opacity: 255,
                math: ImageCompositeMath::Copy,
            }],
        };
        let mut gaiji =
            Gaiji::custom(input, [(25, 24), (26, 24), (51, 24)].into_iter().collect()).unwrap();
        let mut paths = Vec::new();
        let until = Instant::now() + Duration::from_secs(5);
        while !gaiji.poll(ClientId(1), &mut rt.host, &mut paths).unwrap() {
            assert!(Instant::now() < until);
            std::thread::yield_now();
        }
        for (number, alpha) in [(25, 255), (26, 0), (51, 0)] {
            let image = rt
                .host
                .published_image_resource(ClientId(1), &gaiji.images[&(number, 24)])
                .unwrap()
                .unwrap();
            assert_eq!(image.rgba()[3], alpha);
            assert_eq!(image.size(), if number == 51 { [1, 1] } else { [25, 24] });
        }
        for path in paths {
            rt.host.release_image_resource(ClientId(1), &path).unwrap();
        }
    }
}
