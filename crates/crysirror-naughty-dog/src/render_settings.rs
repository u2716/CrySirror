// SPDX-License-Identifier: MPL-2.0
//! Per-area render settings, `dc1.psarc/render-settings.bin`.
//!
//! The module holds 238 `render-settings-2` records named
//! `*AREA-render-settings-NN*` (`*%s-render-settings-%02i*`, `sub_1271E4`).
//! Trigger regions select them with a `render-settings` entry (`StringId`
//! value, `menu-temple-ingame10`'s region names
//! `*menu-temple-render-settings-01*`); cutscenes call
//! `set-render-settings`. Field names and offsets come from the module's
//! `render-settings-menu-entries` (records `{":name", offset, type, step,
//! step, min, max}` per group) and the group bases from the EBOOT's
//! settings writer (`sub_123C1C`): lighting at 0 (`:ambient-scale` +0xcc),
//! post at 0x440 (tint, ambient, light-scale and shadow-colour RGBI), the
//! tonemap at 0x4b0. Other groups (shadow, fog, wind, sky, skydome,
//! particles, rain, environment) are not mapped.
use crate::dc::Dc;
use anyhow::Result;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tonemap {
    pub light_range: [f32; 2],
    pub manual_exposure: f32,
    pub adaption_time: f32,
    pub set_point: f32,
    pub white: f32,
    pub shoulder: f32,
    pub linear: f32,
    pub angle: f32,
    pub toe: f32,
    pub toe_numerator: f32,
    pub toe_denominator: f32,
    pub exposure_bias: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderSettings {
    pub ambient_scale: f32,
    /// RGB × intensity.
    pub tint: [f32; 3],
    pub ambient: [f32; 3],
    pub light_scale: [f32; 3],
    pub shadow_colour: [f32; 3],
    pub tonemap: Tonemap,
}

const LIGHTING_AMBIENT_SCALE: u32 = 0xcc;
const POST: u32 = 0x440;
const TONEMAP: u32 = 0x4b0;

/// The record named `name` (a `StringId`) of the loaded `render-settings`
/// module.
pub fn read(dc: &Dc, name: u32) -> Result<Option<RenderSettings>> {
    let Some(module) = dc.module(crate::dc::sid("render-settings")) else { return Ok(None) };
    let Some(at) = module.entries.iter().find(|(n, _)| *n == name).map(|(_, a)| *a) else { return Ok(None) };
    let m = &dc.mem;
    if m.type_of(at).ok() != Some(crate::dc::sid("render-settings-2")) {
        return Ok(None);
    }
    let f = |o: u32| m.f32(at + o);
    let rgbi = |o: u32| -> Result<[f32; 3]> {
        let i = f(o + 12)?;
        Ok([f(o)? * i, f(o + 4)? * i, f(o + 8)? * i])
    };
    let t = |o: u32| f(TONEMAP + o);
    Ok(Some(RenderSettings {
        ambient_scale: f(LIGHTING_AMBIENT_SCALE)?,
        tint: rgbi(POST)?,
        ambient: rgbi(POST + 0x20)?,
        light_scale: rgbi(POST + 0x30)?,
        shadow_colour: rgbi(POST + 0x40)?,
        tonemap: Tonemap {
            light_range: [t(4)?, t(8)?],
            manual_exposure: t(0xc)?,
            adaption_time: t(0x14)?,
            set_point: t(0x18)?,
            white: t(0x20)?,
            shoulder: t(0x24)?,
            linear: t(0x28)?,
            angle: t(0x2c)?,
            toe: t(0x30)?,
            toe_numerator: t(0x34)?,
            toe_denominator: t(0x38)?,
            exposure_bias: t(0x3c)?,
        },
    }))
}
