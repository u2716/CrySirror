//! Native cursor mattes use independent masked contributions over the retained
//! scene prefix. Shared capture and MaskedPairBlend own all pixel arithmetic;
//! two ordinary source-over Layers would produce a different destination term.
use super::*;
use crysirror_core::image_processing::{ImageInput, ImageJob, ImageProcessState};
use crysirror_image::{effects::ImageEffect, masked_pair::MaskedPairBlend};
pub(super) struct Plan {
    pub atlas: Atlas,
    pub rect: [i32; 4],
    pub sources: [[i32; 4]; 2],
    pub weights: [u16; 2],
}
struct Work {
    plan: Plan,
    capture: Option<ImageJob>,
    blend: Option<ImageJob>,
}
#[derive(Default)]
pub(crate) struct Presenter {
    work: Option<Work>,
    at: Option<u64>,
    path: Option<String>,
}
impl Presenter {
    pub fn pending(&self) -> bool {
        self.work.is_some()
    }
    pub fn release(
        &mut self,
        profile: &SelectionProfile,
        host: &mut dyn HostServices,
    ) -> Result<()> {
        self.work = None;
        self.at = None;
        if let Some(path) = self.path.take() {
            host.release_image_resource(profile.client, &path)?;
        }
        Ok(())
    }
    pub fn path(&self) -> Option<&str> {
        self.path.as_deref()
    }
    pub fn poll(
        &mut self,
        layout: &mut Layout,
        profile: &SelectionProfile,
        host: &mut dyn HostServices,
    ) -> Result<bool> {
        if self.work.is_none() {
            if self.at == Some(host.now_ms()) {
                return Ok(true);
            }
            self.at = Some(host.now_ms());
            let Some(plan) = layout.cursor_plan(profile, host)? else {
                if self.path.is_some() {
                    host.submit_engine(
                        profile.client,
                        1,
                        &serde_json::to_vec(
                            &json!({"layers":[{"op":"remove","id":profile.layer_base+384}]}),
                        )?,
                    )?;
                    self.release(profile, host)?;
                    self.at = Some(host.now_ms());
                }
                return Ok(true);
            };
            let capture = host.begin_layer_prefix_image_over(
                profile.client,
                profile.canvas,
                [profile.order - 1, 300],
                [0, 0, 0, 255],
            )?;
            self.work = Some(Work {
                plan,
                capture: Some(capture),
                blend: None,
            });
        }
        let work = self.work.as_mut().unwrap();
        if let Some(capture) = work.capture.as_mut() {
            let image = match capture.poll()? {
                ImageProcessState::Pending => return Ok(false),
                ImageProcessState::Failed(e) => anyhow::bail!("NNN cursor scene prefix: {e}"),
                ImageProcessState::Ready(image) => image,
            };
            let source = work
                .plan
                .atlas
                .image
                .as_ref()
                .context("NNN special cursor immutable image absent")?
                .clone();
            let input = || ImageInput::Resource(source.clone());
            let effect = ImageEffect::MaskedPairBlend {
                spec: MaskedPairBlend {
                    sources: work.plan.sources,
                    destination: [work.plan.rect[0], work.plan.rect[1]],
                    weights256: work.plan.weights,
                    clip: None,
                },
            };
            work.blend = Some(host.image_metadata(profile.client)?.begin_process(
                ImageInput::Ternary {
                    destination: Box::new(ImageInput::Resource(image)),
                    first: Box::new(input()),
                    second: Box::new(input()),
                },
                serde_json::to_vec(&vec![effect])?.into(),
            )?);
            work.capture = None;
        }
        let image = match work.blend.as_mut().unwrap().poll()? {
            ImageProcessState::Pending => return Ok(false),
            ImageProcessState::Failed(e) => anyhow::bail!("NNN shared cursor matte: {e}"),
            ImageProcessState::Ready(image) => image,
        };
        ensure!(
            image.client() == profile.client && image.size() == profile.canvas,
            "NNN cursor result scope/layout"
        );
        let path = host
            .publish_image_resource(profile.client, profile.layer_base + 384, image)?
            .path
            .relative()
            .to_owned();
        let batch = json!({"layers":[{"op":"put","layer":{"id":profile.layer_base+384,"parent":profile.root,"rect":[0,0,profile.canvas[0],profile.canvas[1]],"order":[profile.order-1,300],"paint":{"kind":"image","asset":path}}}]});
        if let Err(error) = host.submit_engine(profile.client, 1, &serde_json::to_vec(&batch)?) {
            let _ = host.release_image_resource(profile.client, &path);
            return Err(error.into());
        }
        if let Some(old) = self.path.replace(path) {
            host.release_image_resource(profile.client, &old)?;
        }
        self.work = None;
        self.at = Some(host.now_ms());
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crysirror_core::service::MemoryAssetReader;
    use std::time::{Duration, Instant};
    #[test]
    fn native_cursor_freezes_prefix_and_uses_independent_mask_weights_without_consuming_ui() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("game")).unwrap();
        let project=serde_json::from_value(json!({"schema":2,"name":"cursor matte","content_revision":"1","assets":"game","canvas":[1,1],"clients":[{"name":"native","adapter":"crysirror-script-v1","game":"authored"}]})).unwrap();
        let mut rt = crysirror_runtime::Runtime::with_asset_root(
            project,
            root.path(),
            &root.path().join("state"),
            Default::default(),
        )
        .unwrap();
        rt.set_canvas_image_presenter(true);
        let client = ClientId(1);
        rt.host.submit_engine(client,1,&serde_json::to_vec(&json!({"layers":[{"op":"put","layer":{"id":1,"rect":[0,0,1,1],"paint":{"kind":"solid","color":[100,100,100,255]}}}],"ui":[{"op":"put","widget":{"id":300,"kind":"button","rect":[0,0,1,1],"interaction_only":true,"keyboard_focusable":true}}]})).unwrap()).unwrap();
        rt.host
            .engine
            .ui
            .tick(crysirror_core::service::InputFrame::default(), [1, 1])
            .unwrap();
        let bytes = crysirror_image::encode_png(&crysirror_image::ImageSurface {
            width: 1,
            height: 2,
            rgba: vec![200, 200, 200, 128, 20, 20, 20, 255],
        })
        .unwrap();
        let mut job = rt
            .host
            .image_metadata(client)
            .unwrap()
            .begin_process(
                ImageInput::Encoded {
                    name: "cursor.png".into(),
                    bytes: bytes.into(),
                    size: [1, 2],
                },
                b"[]".as_slice().into(),
            )
            .unwrap();
        let until = Instant::now() + Duration::from_secs(5);
        let image = loop {
            match job.poll().unwrap() {
                ImageProcessState::Ready(image) => break image,
                ImageProcessState::Failed(e) => panic!("{e}"),
                ImageProcessState::Pending => {
                    assert!(Instant::now() < until);
                    std::thread::yield_now();
                }
            }
        };
        let atlas = Atlas {
            path: "authored-unused-path".into(),
            size: [1, 2],
            image: Some(image),
        };
        let policy=Arc::new(Policy{setup:Names::parse(b"useSpecialNumber 1 specialWindowSizeX 1 specialWindowSizeY 1 specialFontSize 1 specialCursorSizeX 1 specialCursorSizeY 1 specialCursorDeltaX 0 specialCursorDeltaY 0 specialAnimePatternNumber 2 specialAnimeFrame 4 animePrintType 1",false).unwrap(),variables:Names::parse(b"",false).unwrap(),window:atlas.clone(),cursor:Some(atlas)});
        let mut profile = SelectionProfile {
            client,
            root: 1,
            widget_base: 300,
            layer_base: 400,
            order: 30,
            canvas: [1, 1],
            face: TextFace {
                font: "unused".into(),
                size: 1,
                spacing: 0,
                color: [255; 4],
            },
            rows: vec![],
            message_origins: vec![],
            single_byte_map: None,
        };
        let mut layout = policy.layout(&mut profile, 1).unwrap();
        // At 3/4 of this native cell, weights are separately floored to64/192.
        layout.cursor_count = 3;
        layout.cursor_at = Some(rt.host.now_ms());
        let before: Value = serde_json::from_slice(
            &rt.host
                .query_engine(client, EngineQuery::UiInteraction(300))
                .unwrap(),
        )
        .unwrap();
        assert_eq!(before["hovered"], true);
        let mut presenter = Presenter::default();
        assert!(!presenter.poll(&mut layout, &profile, &mut rt.host).unwrap());
        let mut captured = false;
        while !presenter.poll(&mut layout, &profile, &mut rt.host).unwrap() {
            rt.prepare_canvas_images(|request, _| {
                assert_eq!(request.size, [1, 1]);
                assert!(request.layers.iter().all(|l| l.id != 784));
                captured = true;
                Ok(Some(Arc::new(MemoryAssetReader(Arc::from([
                    100, 100, 100, 255,
                ])))))
            })
            .unwrap();
            assert!(Instant::now() < until);
            std::thread::yield_now();
        }
        assert!(captured);
        let result = rt
            .assets
            .surface(client, presenter.path().unwrap())
            .unwrap();
        assert_eq!(&*result.rgba, &[52, 52, 52, 255]);
        let after: Value = serde_json::from_slice(
            &rt.host
                .query_engine(client, EngineQuery::UiInteraction(300))
                .unwrap(),
        )
        .unwrap();
        assert_eq!(after, before);
        rt.host
            .submit_engine(
                client,
                1,
                &serde_json::to_vec(&json!({"layers":[{"op":"remove","id":784}]})).unwrap(),
            )
            .unwrap();
        presenter.release(&profile, &mut rt.host).unwrap();
        assert!(presenter.path().is_none());
    }
}
