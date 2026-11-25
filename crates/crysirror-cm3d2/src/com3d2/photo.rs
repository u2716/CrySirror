// SPDX-License-Identifier: MPL-2.0
//! SceneEdit.CoWaitPutCloth and CoroutineKasizukiThumShot.
use super::*;
use crysirror_core::{
    image_processing::{ImageJob, ImageProcessState},
    path::{Mount, VfsPath},
};

#[derive(Clone, Copy)]
pub(super) enum Kind {
    Icon,
    Card,
    Guest,
    Facility,
}
impl Kind {
    pub fn size(self) -> [u32; 2] {
        match self {
            Self::Icon => [90, 90],
            _ => [630, 890],
        }
    }
    pub fn head_offset(self) -> Option<[f32; 3]> {
        match self {
            Self::Icon => Some([0.38, 1.07, 0.]),
            Self::Card | Self::Facility => Some([0.64, 2.25, 0.]),
            Self::Guest => None,
        }
    }
    fn path(self, guid: &str) -> Result<VfsPath> {
        let prefix = match self {
            Self::Icon => "icon_thum_",
            Self::Card => "_tmp_thum_",
            Self::Guest => "_tmp_kasizuki_",
            Self::Facility => "f_cos_tmp_",
        };
        Ok(VfsPath::new(
            Mount::Save,
            &format!("Thumb/{prefix}{guid}.png"),
        )?)
    }
}

enum Stage {
    Clothes(u32),
    First(Vec<(Kind, ImageJob)>),
    Fade(u64),
    Pose(u32),
    Look(u32, crysirror_core::graphics::WorldPhoto),
    Last(Vec<(Kind, ImageJob)>),
}
pub(super) struct Session {
    stage: Stage,
    file: String,
    label: String,
    guid: String,
    facility: Option<usize>,
}
impl Session {
    pub fn start(script: &AdvKag) -> Result<Self> {
        let tjs = script.tjs.lock().map_err(|_| anyhow::anyhow!("TJS lock"))?;
        let maid = tjs
            .runtime
            .host()
            .state
            .maid(0)
            .context("missing SceneEdit subject")?;
        let retail = &tjs.runtime.host().state.retail;
        let facility = retail.edit_facility;
        let guid = if let Some(slot) = facility {
            crate::tjs::retail::wardrobe::settings(retail, slot)?
                .guid
                .clone()
        } else {
            maid.guid_id.clone()
        };
        Ok(Self {
            stage: Stage::Clothes(90),
            file: script
                .current_storage()
                .context("missing SceneEdit caller")?
                .into(),
            label: script.tag_backup.get("label").cloned().unwrap_or_default(),
            guid,
            facility,
        })
    }
    fn publish(
        guid: &str,
        jobs: &mut Vec<(Kind, ImageJob)>,
        op: &Opener,
        script: &AdvKag,
    ) -> Result<bool> {
        while let Some((kind, job)) = jobs.last_mut() {
            match job.poll()? {
                ImageProcessState::Pending => return Ok(false),
                ImageProcessState::Failed(reason) => bail!("COM3D2 portrait: {reason}"),
                ImageProcessState::Ready(image) => {
                    ensure!(image.size() == kind.size(), "invalid COM3D2 portrait size");
                    let [width, height] = image.size();
                    let png = crysirror_image::encode_png(&crysirror_image::ImageSurface {
                        width,
                        height,
                        rgba: image.rgba().to_vec(),
                    })?;
                    op.storage.write_atomic(&kind.path(guid)?, &png)?;
                    if matches!(kind, Kind::Icon) {
                        let mut tjs = script.tjs.lock().map_err(|_| anyhow::anyhow!("TJS lock"))?;
                        tjs.runtime
                            .host_mut()
                            .state
                            .retail
                            .set_stock_thumbnail(guid, image)?;
                    }
                }
            }
            jobs.pop();
        }
        Ok(true)
    }
    pub fn update(
        &mut self,
        g: &mut Game,
        host: &mut dyn HostServices,
        client: ClientId,
        op: &mut Opener,
        now: u64,
    ) -> Result<bool> {
        match &mut self.stage {
            Stage::Clothes(n) if *n > 0 => *n -= 1,
            Stage::Clothes(_) => {
                g.sound.play(
                    host,
                    client,
                    &g.files,
                    KindOfSound::Se,
                    "SE022.ogg",
                    0.,
                    false,
                )?;
                let mut shots = Vec::new();
                let kinds: &[Kind] = if self.facility.is_some() {
                    &[Kind::Facility]
                } else {
                    &[Kind::Icon, Kind::Card]
                };
                for &kind in kinds {
                    let view = g.actors.photo_view(kind, g.adv_camera, &g.files, op, now)?;
                    shots.push((kind, host.begin_world_photo(client, view)?));
                }
                g.actors.photo_gaze(None)?;
                g.actors.photo_blink(true)?;
                self.stage = Stage::First(shots);
            }
            Stage::First(jobs) => {
                if Self::publish(&self.guid, jobs, op, &g.script)? {
                    g.leaving = Some(now);
                    self.stage = Stage::Fade(now);
                }
            }
            Stage::Fade(t) if now.saturating_sub(*t) >= 1000 => {
                if self.facility.is_some() {
                    g.script
                        .load(&g.files, op.storage.as_ref(), &self.file, &self.label)?;
                    g.script.exec();
                    g.enter(Scene::Script, op, host, client, now)?;
                    return Ok(true);
                }
                g.script
                    .load(&g.files, op.storage.as_ref(), "KasizukiThumShot.ks", "")?;
                g.script.exec();
                g.script
                    .update(now, false, false, &g.files, op.storage.as_ref())?;
                for effect in std::mem::take(&mut g.script.ui) {
                    match effect {
                        crate::kag::AdvUi::RetailMotionScript {
                            slot,
                            file,
                            label,
                            maid,
                            man,
                            positions,
                        } => {
                            let effects = g.actors.start_motion(
                                &g.script, slot, &file, &label, maid, man, positions, &g.files, op,
                                now,
                            )?;
                            g.edit_motion_effects(effects, op, now)?;
                        }
                        crate::kag::AdvUi::Camera {
                            target,
                            distance,
                            around,
                        } => g.adv_camera = (target, distance, around),
                        other => g.edit_motion_effects(vec![other], op, now)?,
                    }
                }
                self.stage = Stage::Pose(30);
            }
            Stage::Fade(_) => {}
            Stage::Pose(n) if *n > 1 => *n -= 1,
            Stage::Pose(_) => {
                let view = g
                    .actors
                    .photo_view(Kind::Guest, g.adv_camera, &g.files, op, now)?;
                g.actors.photo_gaze(Some(&view))?;
                self.stage = Stage::Look(30, view);
            }
            Stage::Look(n, _) if *n > 1 => *n -= 1,
            Stage::Look(_, view) => {
                self.stage = Stage::Last(vec![(
                    Kind::Guest,
                    host.begin_world_photo(client, view.clone())?,
                )])
            }
            Stage::Last(jobs) => {
                if Self::publish(&self.guid, jobs, op, &g.script)? {
                    g.actors.photo_gaze(None)?;
                    g.leaving = None;
                    if !self.label.is_empty() {
                        g.script
                            .load(&g.files, op.storage.as_ref(), &self.file, &self.label)?;
                        g.script.exec();
                        g.enter(Scene::Script, op, host, client, now)?;
                    }
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
}
use crate::sound::Kind as KindOfSound;
