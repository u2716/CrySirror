// SPDX-License-Identifier: MPL-2.0
//! Ordinary water-gun command eligibility and the shipped mode controls.
mod jet;
pub use jet::{Jet, drop_mask};

use crate::{level::Level, tables::Table, unity::Unity};
use anyhow::{Context, Result};
use crysirror_unity::{PPtr, managed::Fields};
use std::collections::HashMap;

pub struct Standby(HashMap<i32, bool>);
impl Standby {
    pub fn load(unity: &mut Unity) -> Result<Self> {
        let (key, id) = unity
            .bundle_asset(
                "lib/am/lis/000_00.unity3d",
                "al_command_standby[000_00]",
                49,
            )?
            .context("al_command_standby")?;
        let table = Table::parse(&unity.text_asset(&key, id)?)?;
        let col = |k| {
            table
                .keys
                .iter()
                .position(|v| *v == k)
                .with_context(|| format!("standby column {k}"))
        };
        let (id, action) = (col(5)?, col(70)?);
        let mut entries = HashMap::new();
        for row in table.rows {
            let id = row.get(id).context("standby ID")?.parse()?;
            let value = row.get(action).context("standby Action")?;
            entries.insert(id, value.eq_ignore_ascii_case("true"));
        }
        Ok(Self(entries))
    }

    /// Missing table rows leave the prefab's interactability unchanged.
    pub fn enabled(&self, pose: i32, area: i32, tutorial: bool) -> Option<bool> {
        self.0
            .get(&pose)
            .map(|allow| *allow && area == 0 && !tutorial)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Click {
    Toggle,
    End,
}

pub struct Controls {
    end: usize,
    mode: usize,
    sprites: [PPtr; 2],
    pub laser: bool,
    pub laser_speed: f32,
    pub burst_speed: f32,
    pub on_seconds: [f32; 2],
    pub off_seconds: [f32; 2],
}

impl Controls {
    pub fn load(unity: &mut Unity, params: &Fields) -> Result<(Level, Self)> {
        let mut level = Level::prefab(unity, "lib/am/pre/s/000_00.unity3d", "WaterGun")?;
        let component = |script: &str| {
            level
                .scene
                .nodes
                .iter()
                .flat_map(|n| &n.components)
                .find(|c| c.script == script)
                .map(|c| &c.fields)
                .with_context(|| format!("WaterGun {script}"))
        };
        let ui = component("AL.WaterGunUI")?;
        let owner = |f: &str| {
            ui.pptr(f)
                .and_then(|p| level.owner(p.path_id))
                .with_context(|| format!("WaterGun {f}"))
        };
        let (end, mode) = (owner("_btnEnd")?, owner("_btnWaterGun")?);
        let sprite =
            ui.pptr("_selWaterGun")
                .and_then(|p| {
                    level.scene.nodes[mode].components.iter().find(|c| {
                        c.path_id == p.path_id && c.script == "SelectableSpriteChangeCtrl"
                    })
                })
                .context("water-gun button sprite selector")?;
        let main = |k: &str| {
            sprite
                .fields
                .fields(k)
                .and_then(|s| s.pptr("Main"))
                .context("mode sprite")
        };
        let emitter = component("WaterGunObiFFEmitKeyPress")?;
        let span = |key: &str| -> Result<[f32; 2]> {
            let v = emitter.fields(key).context("emitter interval")?;
            let (a, b) = (
                v.float("x").context("interval x")? as f32,
                v.float("y").context("interval y")? as f32,
            );
            Ok([a.min(b), a.max(b)])
        };
        let controls = Self {
            end,
            mode,
            sprites: [main("Off")?, main("On")?],
            laser: true,
            laser_speed: params.float("EmitLaserSpeed").context("EmitLaserSpeed")? as f32,
            burst_speed: params
                .float("EmitMachinegunSpeed")
                .context("EmitMachinegunSpeed")? as f32,
            on_seconds: span("_onDuration")?,
            off_seconds: span("_offDuration")?,
        };
        controls.apply(&mut level);
        Ok((level, controls))
    }
    fn apply(&self, level: &mut Level) {
        level.states.entry(self.mode).or_default().sprite =
            Some(self.sprites[usize::from(self.laser)]);
    }
    pub fn click(&mut self, level: &mut Level, button: usize) -> Option<Click> {
        if button == self.end {
            return Some(Click::End);
        }
        if button != self.mode {
            return None;
        }
        self.laser = !self.laser;
        self.apply(level);
        Some(Click::Toggle)
    }
}

/// Native SetBaseID chooses base 0 for an unlisted map pose. The entry calls
/// AnimPlay with WaterGunAnimManager's kind 1 from the base's Data_00 list.
pub fn entry_motion(unity: &mut Unity, personal: i64, previous: i32) -> Result<(i32, String)> {
    let mut table = |name: &str| -> Result<Table> {
        let (key, id) = unity
            .bundle_asset("lib/wg/lis/000_00.unity3d", name, 49)?
            .context("water-gun animation table")?;
        Table::parse(&unity.text_asset(&key, id)?)
    };
    let find = |t: &Table, id: i32, index: usize| -> Result<Option<i32>> {
        let key = t
            .keys
            .iter()
            .position(|&k| k == 5)
            .context("water-gun row ID")?;
        let data = t
            .keys
            .iter()
            .enumerate()
            .filter(|(_, k)| **k == 18)
            .nth(index)
            .map(|(i, _)| i)
            .context("water-gun pose column")?;
        t.rows
            .iter()
            .find(|r| r.get(key).and_then(|v| v.parse::<i32>().ok()) == Some(id))
            .map(|r| Ok(r.get(data).context("water-gun animation ID")?.parse()?))
            .transpose()
    };
    let base = find(&table("al_wg_baseID[000_00]")?, previous, 0)?.unwrap_or(0);
    let id = find(&table("al_wg_animID[000_00]")?, base, 1)?.unwrap_or(0);
    let state = crate::tables::motion_state(unity, &format!("anim_f_water_{personal:03}"), id)?
        .context("water-gun animation state")?;
    Ok((id, state))
}
