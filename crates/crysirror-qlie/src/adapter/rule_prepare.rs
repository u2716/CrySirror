//! Transition rule preparation (6BA388 -> 55E06C -> 53C288).
//!
//! Every rule-image transition redraws the decoded rule into the screen-sized
//! rule buffer as 5x5 stretched tiles under the eff\fdp pose (offset x/y,
//! scale x/y percent, rotation degrees about the screen centre), smoothed,
//! then stretches the rule level range onto 0..=255. The level is the grey
//! value the fade samples (the red channel of the decoded asset here; native
//! keeps it in its ARGB8888 coverage plane). Pixels outside the tile span
//! keep the buffer's previous content natively; this host starts from 0.
use super::*;
use crysirror_core::image_processing::{ImageInput, ImageJob, ImageProcessState};
use crysirror_image::effects::ImageEffect;
use crysirror_image::tiled_transform::{ChannelRangeStretch, TiledTransform};

#[derive(Default)]
pub(super) struct RulePreparation {
    job: Option<(String, ImageJob)>,
    ready: Option<(String, UploadedImage)>,
}
impl QlieVm {
    fn rule_pose(&self, eff: &str) -> Result<[i32; 5]> {
        let mut pose = [0, 0, 100, 100, 0];
        if let Some(values) = self.objects.get(eff).and_then(|o| o.properties.get("fdp")) {
            ensure!(values.len() <= 5, "QLIE fade transform ABI");
            for (slot, v) in pose.iter_mut().zip(values) {
                *slot = v.integer()?;
            }
        }
        // ImoGraphic SetFadeParam: scales below 50% fall back to 100%.
        for scale in &mut pose[2..4] {
            if *scale < 50 {
                *scale = 100;
            }
        }
        Ok(pose)
    }
    /// Returns the published prepared rule's asset path once ready.
    pub(super) fn prepare_rule(
        &mut self,
        h: &mut dyn HostServices,
        eff: &str,
        path: &str,
    ) -> Result<Option<String>> {
        let pose = self.rule_pose(eff)?;
        let key = format!("{path}|{pose:?}");
        if let Some((k, image)) = &self.rule_preparation.ready {
            if *k == key {
                return Ok(Some(image.path.relative().to_owned()));
            }
        }
        if self.rule_preparation.job.as_ref().is_some_and(|(k, _)| *k != key) {
            self.rule_preparation.job = None;
        }
        if self.rule_preparation.job.is_none() {
            let source = self.objects[eff]
                .images
                .first()
                .and_then(|i| i.as_ref())
                .context("QLIE transition rule image")?;
            let resource = h
                .published_image_resource(self.client, source.path.relative())?
                .context("QLIE immutable rule lease")?;
            let [ax, ay, sx, sy, rotation] = pose;
            let effects = serde_json::to_vec(&[
                ImageEffect::TiledTransform {
                    spec: TiledTransform {
                        size: self.config.canvas,
                        radius: 2,
                        offset: [ax as f32, ay as f32],
                        scale: [sx as f32 / 100., sy as f32 / 100.],
                        rotation_degrees: rotation as f32,
                        smooth: true,
                        background: [0, 0, 0, 0],
                    },
                },
                ImageEffect::ChannelRangeStretch {
                    spec: ChannelRangeStretch { channel: 0 },
                },
            ])?;
            let job = h
                .image_metadata(self.client)?
                .begin_process(ImageInput::Resource(resource), effects.into())?;
            self.rule_preparation.job = Some((key.clone(), job));
        }
        let (_, job) = self.rule_preparation.job.as_mut().unwrap();
        let image = match job.poll()? {
            ImageProcessState::Pending => return Ok(None),
            ImageProcessState::Failed(error) => bail!("QLIE rule preparation: {error}"),
            ImageProcessState::Ready(image) => image,
        };
        self.rule_preparation.job = None;
        self.next_image = self.next_image.checked_add(1).context("QLIE rule resource id")?;
        let published = h.publish_image_resource(self.client, self.next_image, image)?;
        let asset = published.path.relative().to_owned();
        if let Some((_, old)) = self.rule_preparation.ready.replace((key, published)) {
            h.release_image_resource(self.client, old.path.relative())?;
        }
        Ok(Some(asset))
    }
}
