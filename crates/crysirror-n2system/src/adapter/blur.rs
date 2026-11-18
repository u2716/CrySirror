// SPDX-License-Identifier: MPL-2.0
//! SetBlur413600/44B830/457840/465B80 configures pose echoes, not a bitmap
//! blur. 45B850 draws oldest first at a constant thousandths opacity; the
//! final flag chooses before/after the live sprite. 465E80 independently
//! rewinds movement/rotation/scale channels; replacements reset that channel.
use super::*;
use crysirror_scene::{
    animation::ScalarTween,
    layer::{
        LayerSpec, MotionProperty,
        pose_copies::{LaggedPose, PoseCopies, PoseCopy},
    },
};

pub(super) struct Blur {
    poses: LaggedPose,
    count: usize,
    interval: u64,
    opacity: u16,
    before: bool,
    published: bool,
}
impl Blur {
    pub(super) fn active(&self, now: u64) -> Result<bool> {
        let current = self.poses.sample(now)?;
        for age in 1..=self.count {
            if self
                .poses
                .sample(now.saturating_sub(age as u64 * self.interval))?
                != current
            {
                return Ok(true);
            }
        }
        Ok(false)
    }
}
impl N2SystemVm {
    pub(super) fn native_blur(
        &mut self,
        host: &mut dyn HostServices,
        current: u32,
        args: &Arguments<'_>,
    ) -> Result<()> {
        ensure!(args.len() == 6, "N2System SetBlur signature");
        let enabled = args.string(1)?.eq_ignore_ascii_case("true");
        let count = args.integer(2)?;
        let opacity = args.integer(3)?;
        let interval = args.integer(4)?;
        // 465B80 rejects these before changing the previous allocation.
        if enabled && (count > 128 || !(1..=1000).contains(&opacity)) {
            return Ok(());
        }
        ensure!(
            !enabled || interval >= 0,
            "N2System negative echo interval has an unaudited native clock conversion"
        );
        let before = args.string(5)?.eq_ignore_ascii_case("true");
        let mut targets = BTreeSet::new();
        for root in self.query(current, &args.string(0)?)? {
            targets.extend(self.entities.bitmap_targets(root));
        }
        let mut updates = Vec::new();
        for id in targets {
            if !matches!(self.entities.nodes[&id].kind, Kind::Visual) {
                continue;
            }
            let layer: LayerSpec = serde_json::from_value(self.layer(host, id)?)?;
            if !matches!(
                layer.paint,
                crysirror_scene::layer::Paint::Image { .. }
                    | crysirror_scene::layer::Paint::Solid { .. }
            ) {
                continue;
            }
            if enabled {
                ensure!(
                    !self.movies.contains_key(&id) && !self.frame_animations.contains_key(&id),
                    "N2System pose echo over mutable playback requires native reset hooks"
                );
            }
            let blur = if enabled && count > 0 {
                Some(Blur {
                    poses: LaggedPose::new(PoseCopy {
                        position: [layer.rect[0], layer.rect[1]],
                        transform: layer.transform,
                    })?,
                    count: count as usize,
                    interval: interval as u64,
                    opacity: opacity as u16,
                    before,
                    published: false,
                })
            } else {
                None
            };
            updates.push((id, blur));
        }
        ensure!(
            self.blurs
                .iter()
                .filter(|(id, _)| !updates.iter().any(|(n, _)| n == *id))
                .map(|(_, b)| b.count)
                .sum::<usize>()
                + updates
                    .iter()
                    .filter_map(|(_, b)| b.as_ref())
                    .map(|b| b.count)
                    .sum::<usize>()
                <= 4096,
            "N2System pose echo budget"
        );
        self.submit(
            host,
            updates
                .iter()
                .map(|(id, _)| json!({"op":"set_pose_copies","id":id,"copies":null}))
                .collect(),
            vec![],
        )?;
        for (id, blur) in updates {
            if let Some(blur) = blur {
                self.blurs.insert(id, blur);
            } else {
                self.blurs.remove(&id);
            }
        }
        Ok(())
    }
    pub(super) fn record_blur_motion(
        &mut self,
        host: &mut dyn HostServices,
        id: u32,
        property: &str,
        to: f32,
        duration: u64,
        easing: Json,
    ) -> Result<()> {
        if !self.blurs.contains_key(&id) {
            return Ok(());
        }
        let property: MotionProperty = serde_json::from_value(json!(property))?;
        if property == MotionProperty::Opacity {
            return Ok(());
        }
        let layer: LayerSpec = serde_json::from_value(self.layer(host, id)?)?;
        let from = match property {
            MotionProperty::X => layer.rect[0],
            MotionProperty::Y => layer.rect[1],
            MotionProperty::Z => layer.transform.z,
            MotionProperty::ScaleX => layer.transform.scale_x,
            MotionProperty::ScaleY => layer.transform.scale_y,
            MotionProperty::ScaleZ => layer.transform.scale_z,
            MotionProperty::RotationX => layer.transform.rotate_x,
            MotionProperty::RotationY => layer.transform.rotate_y,
            MotionProperty::RotationZ => layer.transform.rotate,
            _ => bail!("N2System unsupported pose echo channel"),
        };
        let curve = ScalarTween::new(
            from,
            to,
            host.now_ms(),
            duration,
            serde_json::from_value(easing)?,
        )?;
        self.blurs
            .get_mut(&id)
            .unwrap()
            .poses
            .begin(property, curve)
    }
    pub(super) fn record_blur_path(&mut self, id: u32, path: &graphics::PathTrack) -> Result<()> {
        if let Some(blur) = self.blurs.get_mut(&id) {
            blur.poses
                .begin_path(path.progress, path.segments.clone())?;
        }
        Ok(())
    }
    pub(super) fn sample_blurs(&mut self, host: &mut dyn HostServices) -> Result<()> {
        let now = host.now_ms();
        let mut commands = Vec::new();
        let mut published = Vec::new();
        for (&id, blur) in &self.blurs {
            let current = blur.poses.sample(now)?;
            let poses = (1..=blur.count)
                .rev()
                .map(|age| {
                    blur.poses
                        .sample(now.saturating_sub(age as u64 * blur.interval))
                })
                .collect::<Result<Vec<_>>>()?;
            let visible = poses.iter().any(|pose| pose != &current);
            if visible || blur.published {
                let copies = visible.then_some(PoseCopies {
                    before: blur.before,
                    opacity: blur.opacity,
                    poses,
                });
                commands.push(json!({"op":"set_pose_copies","id":id,"copies":copies}));
                published.push((id, visible));
            }
        }
        self.submit(host, commands, vec![])?;
        for (id, visible) in published {
            self.blurs.get_mut(&id).unwrap().published = visible;
        }
        Ok(())
    }
    pub(super) fn reset_blur_bitmap(&mut self, host: &mut dyn HostServices, id: u32) -> Result<()> {
        if !self.blurs.contains_key(&id) {
            return Ok(());
        }
        let layer: LayerSpec = serde_json::from_value(self.layer(host, id)?)?;
        let poses = LaggedPose::new(PoseCopy {
            position: [layer.rect[0], layer.rect[1]],
            transform: layer.transform,
        })?;
        self.submit(
            host,
            vec![json!({"op":"set_pose_copies","id":id,"copies":null})],
            vec![],
        )?;
        let blur = self.blurs.get_mut(&id).unwrap();
        blur.poses = poses;
        blur.published = false;
        Ok(())
    }
}
