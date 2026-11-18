// SPDX-License-Identifier: MPL-2.0
//! Ordered pose instances of one retained bitmap. Geometry history is separate
//! from image ownership: all instances sample the current immutable image or
//! target, while visibility, clipping, filtering and parent cameras stay live.
use super::{LayerSpec, Paint, SpriteTransform};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PoseCopy {
    pub position: [f32; 2],
    pub transform: SpriteTransform,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PoseCopies {
    /// Input poses are painter ordered; the original draws after them when
    /// true and before them when false.
    pub before: bool,
    /// Thousandths of the original layer's own opacity, truncated before
    /// ancestor opacity. All copies share this scale, without age attenuation.
    pub opacity: u16,
    pub poses: Vec<PoseCopy>,
}
/// Independent, replaceable scalar pose channels on the shared engine clock.
/// A replacement seeds only that channel from its current value, so rewinding
/// for an echo cannot resurrect a superseded animation. Completed curves stay
/// available until the caller's last delayed instance reaches the endpoint.
#[derive(Clone, Debug)]
pub struct LaggedPose {
    seed: PoseCopy,
    channels: std::collections::BTreeMap<super::MotionProperty, crate::animation::ScalarTween>,
    path: Option<(
        crate::animation::ScalarTween,
        Vec<crate::animation::CubicBezierPath>,
    )>,
}
impl LaggedPose {
    pub fn new(seed: PoseCopy) -> Result<Self> {
        seed.transform.validate()?;
        ensure!(
            seed.position.iter().all(|v| v.is_finite()),
            "lagged pose seed"
        );
        Ok(Self {
            seed,
            channels: Default::default(),
            path: None,
        })
    }
    pub fn begin(
        &mut self,
        property: super::MotionProperty,
        curve: crate::animation::ScalarTween,
    ) -> Result<()> {
        curve.validate()?;
        let mut seed = self.seed.clone();
        set(
            &mut seed,
            property,
            if curve.duration_ms == 0 {
                curve.end
            } else {
                curve.start
            },
        )?;
        if matches!(
            property,
            super::MotionProperty::X | super::MotionProperty::Y
        ) {
            self.path = None;
        }
        self.seed = seed;
        if curve.duration_ms == 0 {
            self.channels.remove(&property);
        } else {
            self.channels.insert(property, curve);
        }
        Ok(())
    }
    pub fn sample(&self, now: u64) -> Result<PoseCopy> {
        let mut value = self.seed.clone();
        for (&property, curve) in &self.channels {
            set(&mut value, property, curve.sample(now))?;
        }
        if let Some((clock, segments)) = &self.path {
            let p = clock.sample(now).clamp(0., 1.) * segments.len() as f32;
            let segment = (p as usize).min(segments.len() - 1);
            let point = segments[segment].sample(p - segment as f32)?;
            value.position = [point[0], point[1]];
        }
        Ok(value)
    }
    pub fn begin_path(
        &mut self,
        clock: crate::animation::ScalarTween,
        segments: Vec<crate::animation::CubicBezierPath>,
    ) -> Result<()> {
        clock.validate()?;
        ensure!(
            !segments.is_empty() && segments.len() <= 4096,
            "lagged pose path budget"
        );
        for segment in &segments {
            segment.validate()?;
        }
        self.channels.remove(&super::MotionProperty::X);
        self.channels.remove(&super::MotionProperty::Y);
        self.path = Some((clock, segments));
        Ok(())
    }
    pub fn moving(&self, now: u64) -> bool {
        self.channels.values().any(|curve| !curve.complete(now))
            || self.path.as_ref().is_some_and(|(c, _)| !c.complete(now))
    }
}
fn set(pose: &mut PoseCopy, property: super::MotionProperty, value: f32) -> Result<()> {
    use super::MotionProperty as P;
    match property {
        P::X => pose.position[0] = value,
        P::Y => pose.position[1] = value,
        P::Z => pose.transform.z = value,
        P::ScaleX => pose.transform.scale_x = value,
        P::ScaleY => pose.transform.scale_y = value,
        P::ScaleZ => pose.transform.scale_z = value,
        P::RotationX => pose.transform.rotate_x = value,
        P::RotationY => pose.transform.rotate_y = value,
        P::RotationZ => pose.transform.rotate = value,
        P::PivotX => pose.transform.pivot_x = value,
        P::PivotY => pose.transform.pivot_y = value,
        P::PivotZ => pose.transform.pivot_z = value,
        _ => anyhow::bail!("lagged pose requires a geometry channel"),
    }
    Ok(())
}
impl PoseCopies {
    pub fn validate(&self, paint: &Paint) -> Result<()> {
        ensure!(
            self.poses.len() <= 128 && self.opacity <= 1000,
            "pose copy capacity/opacity"
        );
        ensure!(
            matches!(paint, Paint::Image { .. } | Paint::Solid { .. }),
            "pose copies require a bitmap quad"
        );
        for pose in &self.poses {
            ensure!(
                pose.position
                    .iter()
                    .all(|n| n.is_finite() && n.abs() <= 1_000_000.),
                "pose copy position bounds"
            );
            pose.transform.validate()?;
        }
        Ok(())
    }
}
pub(super) struct Variants<'a> {
    spec: &'a LayerSpec,
    next: usize,
}
pub(super) fn variants(spec: &LayerSpec) -> Variants<'_> {
    Variants { spec, next: 0 }
}
impl<'a> Iterator for Variants<'a> {
    type Item = (u8, Cow<'a, LayerSpec>);
    fn next(&mut self) -> Option<Self::Item> {
        let count = self
            .spec
            .pose_copies
            .as_ref()
            .map_or(0, |copies| copies.poses.len());
        if self.next > count {
            return None;
        }
        let at = self.next;
        self.next += 1;
        let Some(copies) = self.spec.pose_copies.as_ref().filter(|_| count > 0) else {
            return Some((0, Cow::Borrowed(self.spec)));
        };
        let original = if copies.before { count } else { 0 };
        if at == original {
            return Some((at as u8, Cow::Borrowed(self.spec)));
        }
        let pose = &copies.poses[if copies.before { at } else { at - 1 }];
        let mut spec = self.spec.clone();
        spec.rect[0] = pose.position[0];
        spec.rect[1] = pose.position[1];
        spec.transform = pose.transform.clone();
        spec.opacity = (u32::from(spec.opacity) * u32::from(copies.opacity) / 1000) as u8;
        spec.pose_copies = None;
        Some((at as u8, Cow::Owned(spec)))
    }
}
