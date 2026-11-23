// SPDX-License-Identifier: MPL-2.0
//! `AL.ActionMapUI`, the map HUD (`ActionMapUI.Create` from the `am/pre/u`
//! bundle): map camera previous/next/reset (`_rpMapCameraID` over the
//! move-point cameras), `_btn2DMap` (close and open `MapSelectUI`),
//! `_btnNext` (the `_nextTime` subject: `EventManager.TimeZone`), config
//! and help; `_txtMapName` and the time-zone `ImageSprite`.
use crate::{level::Level, unity::Unity};
use anyhow::{Context, Result};
use crysirror_unity::managed::Value;

/// A HUD button's meaning.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HudAction {
    CameraPrev,
    CameraNext,
    CameraReset,
    /// `_btn2DMap`: `CloseExeAsync` then `MapSelectUI.Create/OpenExe`.
    Map2D,
    /// `_btnNext`: advance the time zone.
    NextTime,
    Config,
    Help,
}

pub struct Hud {
    buttons: Vec<(usize, HudAction)>,
    map_name: Option<usize>,
}

impl Hud {
    /// Instantiate `ActionMapUI` and read its component's references.
    pub fn open(unity: &mut Unity) -> Result<(Level, Self)> {
        let level = Level::prefab(unity, crate::actionmap::UI_BUNDLE, "ActionMapUI")?;
        let c = level.scene.nodes.iter().flat_map(|n| &n.components).find(|c| c.script == "AL.ActionMapUI").context("ActionMapUI component")?.fields.clone();
        let node = |field: &str| c.pptr(field).and_then(|p| level.owner(p.path_id));
        let mut buttons = Vec::new();
        for (field, action) in [
            ("_btnMapCameraPrev", HudAction::CameraPrev),
            ("_btnMapCameraNext", HudAction::CameraNext),
            ("_btnMapCameraReset", HudAction::CameraReset),
            ("_btn2DMap", HudAction::Map2D),
            ("_btnNext", HudAction::NextTime),
            ("_btnConfig", HudAction::Config),
            ("_btnHelp", HudAction::Help),
        ] {
            if let Some(n) = node(field) {
                buttons.push((n, action));
            }
        }
        let map_name = node("_txtMapName");
        Ok((level, Self { buttons, map_name }))
    }

    pub fn action(&self, node: usize) -> Option<HudAction> {
        self.buttons.iter().find(|b| b.0 == node).map(|b| b.1)
    }

    /// `SetMapName`.
    pub fn set_map_name(&self, level: &mut Level, name: &str) {
        if let Some(n) = self.map_name {
            level.states.entry(n).or_default().text = Some(name.to_owned());
        }
    }
}

/// `AL.ImageSprite.Set(index)` on every `ImageSprite` of a UI prefab
/// (`MapSelectUI.ChangeTimeZoneInternal`, `ActionMapUI.ChangeTimeZoneInternal`):
/// the component's `_image` shows `_sprites[index]`.
pub fn set_image_sprites(level: &mut Level, index: usize) -> usize {
    let mut changes = Vec::new();
    for n in &level.scene.nodes {
        for c in n.components.iter().filter(|c| c.script == "AL.ImageSprite") {
            let Some(image) = c.fields.pptr("_image").and_then(|p| level.owner(p.path_id)) else { continue };
            let Some(Value::PPtr(sprite)) = c.fields.array("_sprites").and_then(|s| s.get(index)) else { continue };
            changes.push((image, *sprite));
        }
    }
    let count = changes.len();
    for (image, sprite) in changes {
        level.states.entry(image).or_default().sprite = Some(sprite);
    }
    count
}
