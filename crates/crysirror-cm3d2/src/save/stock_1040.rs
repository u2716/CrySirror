// SPDX-License-Identifier: MPL-2.0
//! Standalone CharacterMgr.Deserialize, up to the first employee status.
//! All ranges borrow an immutable slot and end before its first bookmark.
use super::player_1040::{self, Player};
use crate::{parts_color::PartsColor, tjs::retail};
use anyhow::{Context, Result, ensure};
use crysirror_asset::interchange::cm3d2::Reader;

type Properties<'a> = Vec<(String, &'a [u8])>;

pub(crate) struct Man<'a> {
    pub properties: Properties<'a>,
    pub active_slot: i32,
    pub thumbnail: &'a [u8],
    pub thumbnail_time: String,
    pub color: [f32; 4],
    pub voice_pitch: i32,
}

pub(crate) struct Appearance<'a> {
    pub properties: Properties<'a>,
    pub colors: [PartsColor; 9],
}

pub(crate) struct StockPrefix<'a> {
    pub player: Player,
    pub men: Vec<Man<'a>>,
    pub employees: usize,
    pub first: Option<Appearance<'a>>,
    pub next: usize,
}

pub(crate) fn read_slot(bytes: &[u8]) -> Result<StockPrefix<'_>> {
    let (player, body) = player_1040::read_slot_management(bytes)?;
    let source = &bytes[body.clone()];
    let mut r = Reader::new(source);
    let count = r.count(1)?;
    let mut men = Vec::new();
    for index in 0..count {
        let properties =
            retail::stock_props(&mut r, source).with_context(|| format!("stock man {index}"))?;
        ensure!(
            r.header("CM3D2_MAID_MISC")? == 1040,
            "unsupported standalone character metadata"
        );
        let active_slot = r.i32()?;
        let size = r.count(1)?;
        let thumbnail = r.take(size)?;
        let thumbnail_time = r.string()?;
        let color = [r.f32()?, r.f32()?, r.f32()?, r.f32()?];
        let voice_pitch = r.i32()?;
        men.push(Man {
            properties,
            active_slot,
            thumbnail,
            thumbnail_time,
            color,
            voice_pitch,
        });
    }
    let employees = r.count(1)?;
    let first = if employees > 0 {
        let properties =
            retail::stock_props(&mut r, source).context("first employee properties")?;
        let colors = retail::read_retail_colors(&mut r)?;
        Some(Appearance { properties, colors })
    } else {
        None
    };
    let next = body.start + r.position();
    // The loader must be at an actual section boundary even for an empty roster.
    let tag = if employees > 0 {
        "CM3D2_MAID_STATUS"
    } else {
        "CM3D2_SCRIPT"
    };
    ensure!(
        r.header(tag)? == 1040,
        "unsupported standalone {tag} version"
    );
    Ok(StockPrefix {
        player,
        men,
        employees,
        first,
        next,
    })
}
