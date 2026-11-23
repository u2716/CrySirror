// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! CG difference publication boundary. Shared graph/decoder jobs own all
//! pixels; the native mode retains raw old-picture identity and Print clocks.
use super::*;
use crate::{
    native_cg_composition::CgTile,
    native_cg_difference::{DifferencePrint, DifferenceState},
};
impl PictureViewer {
    pub(super) fn difference_active(&self) -> bool {
        self.difference
            .as_ref()
            .is_some_and(|state| state.enabled && state.profile.selector != 0)
    }
    pub(super) fn poll_difference(
        &mut self,
        host: &mut dyn HostServices,
        mut vm: Option<&mut NativeAdapter>,
    ) -> Result<bool> {
        if let Some(job) = self.difference_base_job.as_mut() {
            match job.poll()? {
                ImageProcessState::Pending => return Ok(false),
                ImageProcessState::Failed(error) => {
                    anyhow::bail!("NNN CG prior engine frame: {error}")
                }
                ImageProcessState::Ready(image) => {
                    ensure!(
                        image.client() == self.client && image.size() == SIZE,
                        "NNN CG prior-frame scope"
                    );
                    self.difference_frame = Some(image);
                    self.difference_base_job = None;
                }
            }
        }
        let Some(work) = self.difference_draw.as_mut() else {
            return Ok(true);
        };
        let Some(ready) = work.poll(host)? else {
            return Ok(false);
        };
        if ready.random.is_some() {
            ensure!(
                vm.is_some(),
                "NNN randomized CG Print lost its native owner"
            );
        }
        let published = host.publish_image_resource(self.client, ROOT, ready.image.clone())?;
        let path = published.path.relative().to_owned();
        let commands = serde_json::json!({"layers":[{"op":"remove","id":ROOT},{"op":"put","layer":{
            "id":ROOT,"rect":[0,0,SIZE[0],SIZE[1]],"clip":[0,0,SIZE[0],SIZE[1]],"order":[2000000001,0],
            "paint":{"kind":"image","asset":path}}}]});
        if let Err(error) = host.submit_engine(self.client, 1, &serde_json::to_vec(&commands)?) {
            let _ = host.release_image_resource(self.client, &path);
            return Err(error.into());
        }
        // The mode cannot run another background VM update while this Print
        // is pending. Publish its exact ready image before accepting RNG and
        // native Print counters; a completion retry cannot rerun the planner.
        if let Some(random) = ready.random {
            vm.as_deref_mut()
                .unwrap()
                .accept_mode_wipe_context(random, ready.tables)?;
        }
        self.difference = Some(ready.difference);
        self.difference_frame = Some(ready.image);
        self.live
            .as_mut()
            .context("NNN completed CG Print lost its picture")?
            .pose = self.difference_pose.take();
        if let Some(old) = self.difference_path.replace(path) {
            host.release_image_resource(self.client, &old)?;
        }
        for path in self.difference_retired_paths.drain(..) {
            host.release_image_resource(self.client, &path)?;
        }
        self.difference_draw = None;
        // A completed worker is the end of an already admitted Print. A new
        // source Calcu/input batch begins on a subsequent owner update.
        Ok(false)
    }
    pub(super) fn draw_difference(
        &mut self,
        host: &mut dyn HostServices,
        vm: Option<&mut NativeAdapter>,
        native_tick: bool,
    ) -> Result<()> {
        if self.difference_draw.is_some() || self.difference_base_job.is_some() {
            return Ok(());
        }
        let state = self
            .difference
            .as_ref()
            .context("NNN CG difference policy absent")?;
        let live = self
            .live
            .as_ref()
            .context("NNN CG difference live picture absent")?;
        let position = live.motion.position();
        let pose = (position, 0);
        let changed = live.pose != Some(pose);
        if !changed && (!native_tick || !state.forces_redraw()) {
            return Ok(());
        }
        let first = state.plan(0);
        let needs_random = matches!(first,DifferencePrint::Wipe{kind,..} if crate::native_simple_wipe::needs_random(kind));
        let (random, tables) = if needs_random {
            let vm = vm.context("NNN randomized CG wipe requires its native VM owner")?;
            let Some((random, tables)) = vm.acquire_mode_wipe_context()? else {
                return Ok(());
            };
            (Some(random), tables)
        } else {
            (None, Default::default())
        };
        let Some(base) = self.difference_frame.as_ref().cloned() else {
            // A one-item catalogue can enable sabunFlag without loading a
            // new CG. There was no publication at which to capture the prior
            // scene, so admit that dependency here before the first Print.
            self.difference_base_job = Some(host.begin_layer_prefix_image_over(
                self.client, SIZE, [i32::MAX, i32::MAX], [0, 0, 0, 255],
            )?);
            return Ok(());
        };
        let count = live.views.len() - usize::from(live.background);
        let masked = live.entry.name[..2].eq_ignore_ascii_case(b"ta");
        let tiles = live.views[..count]
            .iter()
            .map(|view| CgTile {
                image: if masked {
                    view.keyed.clone()
                } else {
                    view.opaque.clone()
                },
                source_clip: view.source_clip,
                masked,
            })
            .collect::<Vec<_>>();
        let mut base = ImageInput::Resource(base);
        if masked {
            base = if live.background {
                let picture = &live.views[count];
                let size = picture.raw.size();
                if let Some((source, destination)) =
                    crysirror_image::coverage_bounds::intersect_source_window(
                        [0, 0, size[0] as i32, size[1] as i32],
                        [0, 0],
                        picture.source_clip,
                    )?
                {
                    ImageInput::Composite {
                        base: Box::new(base),
                        layers: vec![crysirror_core::image_processing::ImageCompositeLayer {
                            input: Box::new(ImageInput::Resource(picture.opaque.clone())),
                            source,
                            destination,
                            clip: None,
                            opacity: 255,
                            math: crysirror_core::image_processing::ImageCompositeMath::CopyRgb,
                        }],
                    }
                } else {
                    base
                }
            } else {
                ImageInput::Solid {
                    size: SIZE,
                    color: live.profile.color,
                }
            };
        }
        let columns = live.entry.tiles()[0];
        self.difference_draw = Some(super::draw_work::CgDrawWork::begin(
            self.client,
            SIZE,
            base,
            tiles,
            columns,
            position,
            state.clone(),
            self.old_difference.clone(),
            random,
            tables,
            host,
        )?);
        self.difference_pose = Some(pose);
        Ok(())
    }
}
