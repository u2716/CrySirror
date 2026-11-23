// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Message-window rebinding while retained GlyphRuns keep their reveal clocks.
//! The original three-plane window order is policy; every pixel is produced by
//! a shared ImageInput graph. A background frame and all window crops commit in
//! one owner engine batch, with no second message or synthetic VM receipt.
use super::*;
use crysirror_core::image_processing::{
    ImageCompositeLayer, ImageCompositeMath, ImageInput, ImageJob, ImageProcessState, ImageResource,
};

pub(super) fn window_layers(windows: &[MessageWindow]) -> Result<Vec<ImageCompositeLayer>> {
    let mut layers = Vec::new();
    for window in windows {
        let resource = window
            .resource
            .clone()
            .context("NNN unresolved native window")?;
        let mut append = |source_y: i32, math: ImageCompositeMath| {
            layers.push(ImageCompositeLayer {
                input: Box::new(ImageInput::Resource(resource.clone())),
                source: [0, source_y, window.rect[2], window.rect[3]],
                destination: [window.rect[0], window.rect[1]],
                clip: None,
                opacity: 255,
                math,
            });
        };
        if window.stacked_planes & 2 != 0 {
            let rows = if window.stacked_planes & 1 != 0 { 2 } else { 1 };
            append(
                window.rect[3]
                    .checked_mul(rows)
                    .context("NNN window plane offset")?,
                ImageCompositeMath::MaskSeparate256,
            );
        }
        if window.percent >= 100 {
            append(0, ImageCompositeMath::MaskSeparate256);
        } else if window.percent > 0 {
            append(
                0,
                ImageCompositeMath::MaskPiecewise256 {
                    weight: window.percent * 256 / 100,
                },
            );
        }
        if window.stacked_planes & 1 != 0 {
            append(window.rect[3], ImageCompositeMath::MaskSeparate256);
        }
    }
    Ok(layers)
}
pub(super) fn composite_input(
    background: Arc<dyn ImageResource>,
    windows: &[MessageWindow],
) -> Result<ImageInput> {
    Ok(ImageInput::Composite {
        base: Box::new(ImageInput::Resource(background)),
        layers: window_layers(windows)?,
    })
}

pub(super) struct Refresh {
    background: Arc<dyn ImageResource>,
    nodes: Vec<Value>,
    windows: Vec<MessageWindow>,
    job: Option<ImageJob>,
    path: Option<String>,
    ready: bool,
}
impl NativeTextBridge {
    pub(crate) fn published_message(&self) -> bool {
        self.published
    }

    pub(crate) fn prepare_background_refresh(
        &mut self,
        host: &mut dyn HostServices,
        background: Arc<dyn ImageResource>,
    ) -> Result<bool> {
        ensure!(
            self.published && self.ready.is_none() && self.worker.is_none(),
            "NNN frame/window refresh before message publication"
        );
        ensure!(
            background.client() == self.profile.client
                && background.size() == self.profile.canvas
                && background.opaque(),
            "NNN native frame/window refresh scope"
        );
        if self.background_refresh.is_none() {
            let mut windows = Vec::new();
            let mut nodes = Vec::new();
            for window in self
                .profile
                .window
                .iter()
                .chain(self.profile.name_window.iter())
            {
                let bytes =
                    host.query_engine(self.profile.client, EngineQuery::Layer(window.id))?;
                self.api_calls += 1;
                let layer: LayerSpec =
                    serde_json::from_slice(&bytes).context("NNN live message window absent")?;
                if layer.visible && (window.percent > 0 || window.stacked_planes != 0) {
                    ensure!(
                        window.resource.is_some(),
                        "NNN live unresolved window source"
                    );
                    nodes.push(json!({"op":"put", "layer":serde_json::to_value(layer)?}));
                    windows.push(window.clone());
                }
            }
            let job = if windows.is_empty() {
                None
            } else {
                self.api_calls += 1;
                Some(host.image_metadata(self.profile.client)?.begin_process(
                    composite_input(background.clone(), &windows)?,
                    b"[]".as_slice().into(),
                )?)
            };
            self.background_refresh = Some(Refresh {
                background: background.clone(),
                nodes,
                windows,
                ready: job.is_none(),
                job,
                path: None,
            });
        }
        let refresh = self.background_refresh.as_mut().unwrap();
        ensure!(
            Arc::ptr_eq(&refresh.background, &background),
            "NNN pending window refresh target changed"
        );
        if refresh.ready {
            return Ok(true);
        }
        match refresh.job.as_mut().unwrap().poll()? {
            ImageProcessState::Pending => Ok(false),
            ImageProcessState::Failed(error) => {
                anyhow::bail!("NNN live window composition: {error}")
            }
            ImageProcessState::Ready(image) => {
                ensure!(
                    image.client() == self.profile.client && image.size() == self.profile.canvas,
                    "NNN live window composition scope"
                );
                let published =
                    host.publish_image_resource(self.profile.client, refresh.windows[0].id, image)?;
                self.api_calls += 1;
                let path = published.path.relative().to_owned();
                for (node, window) in refresh.nodes.iter_mut().zip(&refresh.windows) {
                    let [x, y, w, h] = window.rect;
                    let [cw, ch] = self.profile.canvas;
                    node["layer"]["paint"] = json!({"kind":"image", "asset":path});
                    node["layer"]["uv"] = json!([
                        x as f32 / cw as f32,
                        y as f32 / ch as f32,
                        (x + w) as f32 / cw as f32,
                        (y + h) as f32 / ch as f32
                    ]);
                }
                refresh.path = Some(path);
                refresh.job = None;
                refresh.ready = true;
                Ok(true)
            }
        }
    }
    pub(crate) fn background_refresh_operations(&self) -> Result<Vec<Value>> {
        let refresh = self
            .background_refresh
            .as_ref()
            .context("NNN frame/window refresh absent")?;
        ensure!(refresh.ready, "NNN frame/window refresh incomplete");
        Ok(refresh.nodes.clone())
    }
    pub(crate) fn accept_background_refresh(&mut self, host: &mut dyn HostServices) -> Result<()> {
        let refresh = self
            .background_refresh
            .as_ref()
            .context("NNN frame/window refresh absent")?;
        ensure!(refresh.ready, "NNN frame/window refresh incomplete");
        let refresh = self.background_refresh.take().unwrap();
        self.background = Some(refresh.background);
        if let Some(path) = refresh.path {
            if let Some(previous) = self.window_path.replace(path) {
                host.release_image_resource(self.profile.client, &previous)?;
                self.api_calls += 1;
            }
        }
        Ok(())
    }
}
