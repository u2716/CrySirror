// SPDX-License-Identifier: MPL-2.0
//! Type 0x53's two projected panels and rule-image transition.

use super::{hidden_layer, hundredths, sine, view_projection, wall};
use anyhow::Result;
use crysirror_core::service::UploadedImage;
use crysirror_scene::{layer::Blend, projected_mesh::MeshFrame};
use glam::Vec3;
use serde_json::{json, Value};

pub(in crate::adapter) const PANELS: [[&str; 2]; 2] = [
    ["try007c3.pb3", "try007c1.pb3"],
    ["try007c4.pb3", "try007c2.pb3"],
];
pub(in crate::adapter) const OVERLAY: &str = "__try007a.pb3";
const RULES: [[&str; 2]; 2] = [
    ["try007d3.pb3", "try007d1.pb3"],
    ["try007d4.pb3", "try007d2.pb3"],
];
const REVEAL_MS: u32 = 6000;

enum RuleState {
    Requested,
    Loading,
    Running(u32),
}

pub(super) struct Door {
    fallback: [bool; 2],
    queued: Option<u32>,
    phase: u32,
    phase_ms: u32,
    pan_phase: u32,
    pan_ms: u32,
    angle: [f32; 2],
    opened: f32,
    opacity: u8,
    shown: bool,
    rule: Option<RuleState>,
}

impl Door {
    pub fn new(fallback: [bool; 2]) -> Self {
        Self {
            fallback,
            queued: None,
            phase: 0,
            phase_ms: 0,
            pan_phase: 0,
            pan_ms: 0,
            angle: [0.; 2],
            opened: 0.,
            opacity: 255,
            shown: false,
            rule: None,
        }
    }

    pub fn trigger(&mut self, key: u32) {
        if (1..=5).contains(&key) {
            self.queued = Some(key);
        }
    }

    pub fn status(&self) -> u32 {
        if self.queued.is_some() {
            u32::MAX
        } else {
            self.phase
        }
    }

    pub fn rule_request(&mut self) -> Option<[&'static str; 2]> {
        if !matches!(self.rule, Some(RuleState::Requested)) {
            return None;
        }
        self.rule = Some(RuleState::Loading);
        Some(std::array::from_fn(|i| {
            RULES[i][usize::from(self.fallback[i])]
        }))
    }

    pub fn rules_loaded(&mut self) {
        self.rule = Some(RuleState::Running(0));
        self.shown = true;
    }

    /// The original advances the rule transition before submitting its panels.
    pub fn sample_rule(&mut self, delta: u32) -> bool {
        let Some(RuleState::Running(elapsed)) = &mut self.rule else {
            return false;
        };
        *elapsed = elapsed.saturating_add(delta);
        if *elapsed >= REVEAL_MS {
            self.rule = None;
            return true;
        }
        false
    }

    pub fn advance(&mut self, delta: u32) {
        if self.phase == 0 {
            self.phase = self.queued.take().unwrap_or(0);
        }
        match self.phase {
            1 => {
                self.phase = 10;
                self.rule = Some(RuleState::Requested);
            }
            2 | 3 => {
                self.shown = self.phase == 3;
                self.phase = 0;
            }
            4 | 5 => {
                self.phase += 7;
                self.phase_ms = 0;
            }
            10 if self.rule.is_none() => self.phase = 0,
            11 => {
                self.phase_ms = self.phase_ms.saturating_add(delta);
                if self.phase_ms >= 800 {
                    self.opened = 5.;
                    self.phase = 0;
                } else {
                    self.opened = hundredths(sine(self.phase_ms as f32 * 90. / 800.) * 5.);
                }
            }
            12 => {
                self.phase_ms = self.phase_ms.saturating_add(delta);
                if self.phase_ms >= 500 {
                    self.shown = false;
                    self.phase = 0;
                } else {
                    self.opacity = (256 - (self.phase_ms << 8) / 500).min(255) as u8;
                }
            }
            _ => {}
        }
        self.pan_ms = self.pan_ms.wrapping_add(delta);
        let swing = match self.pan_phase {
            0 | 2 => {
                let t = self.pan_ms.min(2000) as f32 / 2000.;
                let swing = if self.pan_phase == 0 { t } else { 1. - t };
                if self.pan_ms >= 2000 {
                    self.pan_phase += 1;
                    self.pan_ms = 0;
                }
                swing
            }
            _ => {
                let swing = if self.pan_phase == 1 { 1. } else { 0. };
                if self.pan_ms > 200 {
                    self.pan_phase = (self.pan_phase + 1) % 4;
                    self.pan_ms = 0;
                }
                swing
            }
        };
        self.angle = [self.opened + swing, self.opened - swing];
    }

    pub fn layers(
        &self,
        ids: [u32; 3],
        index: usize,
        textures: &[Option<UploadedImage>],
        size: [f32; 2],
        camera: [f32; 7],
    ) -> Result<Vec<Value>> {
        let projection = view_projection(camera, size)?;
        let mut layers = Vec::with_capacity(3);
        let progress = match self.rule {
            Some(RuleState::Running(elapsed)) => Some(elapsed as f32 / REVEAL_MS as f32),
            _ => None,
        };
        for (panel, (width, x)) in [([-20., 0.], -0.3), ([0., 20.], 0.1)]
            .into_iter()
            .enumerate()
        {
            let id = ids[panel];
            if let Some(image) = textures[panel].as_ref().filter(|_| self.shown) {
                let quad = wall(
                    image,
                    width,
                    [-7.5, 12.5],
                    Vec3::new(x, 0., 28.),
                    self.angle[panel],
                    Blend::Normal,
                );
                let frame = MeshFrame {
                    size,
                    parts: vec![quad],
                }
                .project_homogeneous(projection, false)?;
                let mask = progress
                    .zip(textures[3 + panel].as_ref())
                    .map(|(progress, mask)| {
                        json!({
                            "asset": mask.path.relative(),
                            "channel": "green",
                            "coverage": {"progress": progress, "feather": 0.5}
                        })
                    });
                layers.push(json!({"op":"put","layer":{
                    "id": id, "rect":[0.,0.,size[0],size[1]], "order":[-1,index],
                    "opacity": self.opacity, "filter":{"encoded_sampling":true,"mask":mask},
                    "paint":{"kind":"projected_mesh","frame":frame}
                }}));
            } else {
                layers.push(hidden_layer(id, index));
            }
        }
        let id = ids[2];
        if let Some(image) = &textures[2] {
            let size = image.size;
            let width = size[0].min(1024) as f32;
            let height = size[1].min(614) as f32;
            layers.push(json!({"op":"put","layer":{
                "id":id,"rect":[0.,0.,width,height],"order":[-1,index],
                "uv":[0.,0.,width / size[0] as f32,height / size[1] as f32],
                "paint":{"kind":"image","asset":image.path.relative()}
            }}));
        } else {
            layers.push(hidden_layer(id, index));
        }
        Ok(layers)
    }
}

#[cfg(test)]
mod tests {
    use super::Door;

    #[test]
    fn rule_choice_and_queued_phases_follow_native_visibility() {
        let mut door = Door::new([false, true]);
        assert!(!door.shown);
        door.trigger(1);
        assert_eq!(door.status(), u32::MAX);
        door.advance(0);
        assert_eq!(door.status(), 10);
        assert_eq!(door.rule_request(), Some(["try007d3.pb3", "try007d2.pb3"]));
        assert_eq!(door.rule_request(), None);

        door.rules_loaded();
        assert!(door.shown);
        door.trigger(4);
        assert!(!door.sample_rule(5999));
        assert!(door.sample_rule(1));
        door.advance(1);
        assert_eq!(door.status(), u32::MAX);
        door.advance(0);
        assert_eq!(door.status(), 11);
        door.advance(400);
        assert_eq!(door.opened, 3.53);
        door.advance(400);
        assert_eq!(door.opened, 5.);
        assert_eq!(door.status(), 0);

        door.trigger(5);
        door.advance(0);
        door.advance(250);
        assert_eq!(door.opacity, 128);
        door.advance(250);
        assert!(!door.shown);
        door.trigger(3);
        door.advance(0);
        assert!(door.shown);
        assert_eq!(door.opacity, 128);
        door.trigger(2);
        door.advance(0);
        assert!(!door.shown);
    }
}
