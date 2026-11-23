// SPDX-License-Identifier: MPL-2.0
use super::{Game, HostServices, Ui, game_random, sound};
use crate::watergun::{Click, Jet, drop_mask};
use anyhow::{Context, Result};
use serde_json::json;

const JET_ID: u32 = 0x4f00;
impl Game {
    pub(super) fn open_water_gun(&mut self, host: &mut dyn HostServices, ci: usize) -> Result<()> {
        let (level, mut jet) = Jet::load(&mut self.unity, &self.water_params)?;
        let view = self.view.clone().context("water-gun command camera")?;
        let (spot, _) = self.command.context("water-gun command target")?;
        let (card, _, personal, _) = self.spots.get(spot).cloned().context("water-gun target")?;
        let previous = self.extra(&card).action_motion_id;
        let (id, state) = crate::watergun::entry_motion(&mut self.unity, personal, previous)?;
        let restore = crate::tables::motion_state(
            &mut self.unity,
            &format!("anim_f_{personal:03}"),
            previous,
        )?
        .context("water-gun return pose")?;
        let tex = host.upload_rgba(self.client_id, 0x5900_0000, 32, 32, &drop_mask())?;
        jet.set_texture(tex.relative());
        let chunks = jet.model(&level, &mut self.unity)?;
        let (scale, rot, pos) = jet.model_pose(&view).to_scale_rotation_translation();
        let (rx, ry, rz) = rot.to_euler(glam::EulerRot::XYZ);
        // Admit every chunk before publishing the gun or hiding the command menu.
        for glb in chunks {
            self.next_model += 1;
            match host.upload_model(self.client_id, 0x6000_0000 + self.next_model, &glb) {
                Ok(path) => jet.model_entities.push((0x100 + self.next_model, path)),
                Err(err) => {
                    for (_, path) in jet.model_entities.drain(..) {
                        host.release_model(self.client_id, &path)?;
                    }
                    return Err(err.into());
                }
            }
        }
        if let Err(e) = self.map_change_motion(host, self.client_id, &card, &state) {
            for (_, path) in jet.model_entities.drain(..) {
                host.release_model(self.client_id, &path)?;
            }
            return Err(e);
        }
        jet.previous_motion = Some((card, restore));
        self.log
            .push(format!("WaterGun target pose {previous} → {id} {state}"));
        for (id, path) in &jet.model_entities {
            self.world.push(json!({"op":"spawn","entity":{"id":id,"name":"WaterGun model","geometry":{"kind":"scene","asset":path.relative()},
                "transform":{"translation":pos.to_array(),"rotation_degrees":[rx.to_degrees(),ry.to_degrees(),rz.to_degrees()],"scale":scale.to_array()}}}));
        }
        let (p, q) = jet.pose(&view);
        let (x, y, z) = q.to_euler(glam::EulerRot::XYZ);
        self.world.push(json!({"op":"spawn","entity":{"id":JET_ID,"name":"WaterGun stream","transform":{
            "translation":p.to_array(),"rotation_degrees":[x.to_degrees(),y.to_degrees(),z.to_degrees()],"scale":[1.,1.,1.]},"vfx":jet.system}}));
        if let Some((level, _)) = self.overlays.get_mut(ci) {
            if let Some(panel) = level.find("Command/Canvas/Panel") {
                level.set_active(panel, false);
            }
        }
        self.overlays.push((level, Ui::new(12)));
        self.water = Some(jet);
        self.log.push("WaterGun opened: laser".into());
        Ok(())
    }

    pub(super) fn water_input(&mut self, buttons: u64, now: u64) {
        let Some(jet) = self.water.as_mut() else {
            return;
        };
        let was_held = jet.held;
        if jet.update(buttons, now as f64 / 1000., || {
            game_random(&mut self.rng, 1 << 24) as f32 / (1 << 24) as f32
        }) {
            if let Some(view) = self.view.as_ref() {
                let (p, q) = jet.pose(view);
                let (x, y, z) = q.to_euler(glam::EulerRot::XYZ);
                self.world.push(json!({"op":"set_transform","id":JET_ID,"transform":{"translation":p.to_array(),
                    "rotation_degrees":[x.to_degrees(),y.to_degrees(),z.to_degrees()],"scale":[1.,1.,1.]}}));
            }
            self.world
                .push(json!({"op":"set_vfx","id":JET_ID,"vfx":jet.system}));
        }
        if jet.held != was_held {
            self.log
                .push(format!("WaterGun emitter speed {}", jet.speed));
        }
    }

    pub(super) fn water_click(&mut self, host: &mut dyn HostServices, node: usize) -> Result<()> {
        let Some((level, _)) = self.overlays.last_mut() else {
            return Ok(());
        };
        let Some(jet) = self.water.as_mut() else {
            return Ok(());
        };
        match jet.controls.click(level, node) {
            Some(Click::Toggle) => {
                self.level
                    .sounds
                    .push(sound::Call::SystemSe(self.sound_ids.ok_s));
                self.log.push(format!(
                    "WaterGun mode {}",
                    if jet.controls.laser { "laser" } else { "burst" }
                ));
            }
            Some(Click::End) => {
                for (id, path) in jet.model_entities.drain(..) {
                    self.world.push(json!({"op":"despawn","id":id}));
                    host.release_model(self.client_id, &path)?;
                }
                let restore = jet.previous_motion.take();
                self.water = None;
                if let Some((card, state)) = restore {
                    self.map_change_motion(host, self.client_id, &card, &state)?;
                    self.log.push(format!("WaterGun restored {state}"));
                }
                self.world.push(json!({"op":"despawn","id":JET_ID}));
                let (_, mut ui) = self.overlays.pop().expect("water-gun overlay");
                self.removed.extend(ui.clear());
                if let Some((level, _)) = self.command.and_then(|(_, ci)| self.overlays.get_mut(ci))
                {
                    if let Some(panel) = level.find("Command/Canvas/Panel") {
                        crate::talk::show(level, panel);
                    }
                }
                self.level
                    .sounds
                    .push(sound::Call::SystemSe(self.sound_ids.cancel));
                self.pressed = None;
                self.log.push("WaterGun closed".into());
            }
            None => {}
        }
        Ok(())
    }
}
