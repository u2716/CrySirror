// SPDX-License-Identifier: MPL-2.0
use super::{ScreenAbi, Sprite, sprite};
use crate::parameters::{SoundSettings, Speeds};
use anyhow::Result;
use crysirror_core::{
    service::{HostServices, InputFrame, buttons},
    state::ClientId,
};
use serde_json::Value;

pub(super) const KNOBS: std::ops::Range<u32> = 8002..8005;

struct Sliders {
    left: i32,
    right: i32,
    hit_y: [[i32; 2]; 3],
    draw_x: i32,
    draw_y: [i32; 3],
    reversed: bool,
}

impl Sliders {
    fn for_abi(abi: ScreenAbi) -> Self {
        match abi {
            ScreenAbi::Early => Self {
                left: 28,
                right: 252,
                hit_y: [[84, 120], [160, 196], [232, 276]],
                draw_x: 33,
                draw_y: [96, 171, 246],
                reversed: false,
            },
            ScreenAbi::SignedDuration => Self {
                left: 133,
                right: 357,
                hit_y: [[100, 136], [176, 212], [248, 292]],
                draw_x: 138,
                draw_y: [112, 187, 262],
                reversed: false,
            },
            ScreenAbi::PersistentZoom => Self {
                left: 253,
                right: 478,
                hit_y: [[111, 135], [149, 173], [186, 210]],
                draw_x: 253,
                draw_y: [111, 149, 186],
                reversed: true,
            },
        }
    }

    fn hit(&self, x: i32, y: i32) -> Option<usize> {
        if !(self.left..self.right).contains(&x) {
            return None;
        }
        self.hit_y
            .iter()
            .position(|&[top, bottom]| (top..bottom).contains(&y))
    }

    fn value(&self, x: i32) -> u8 {
        let value = (x.saturating_sub(self.left + 11).clamp(0, 200) / 2) as u8;
        if self.reversed { 100 - value } else { value }
    }

    fn placed(&self, knob: &Sprite, index: usize, speed: u8) -> Value {
        let mut knob = knob.clone();
        let displayed = if self.reversed { 100 - speed } else { speed };
        knob.rect[0] = self.draw_x + 2 * i32::from(displayed);
        knob.rect[1] = self.draw_y[index];
        knob.layer(KNOBS.start + index as u32, 255)
    }
}

pub(super) struct Settings {
    sliders: Sliders,
    knob: Sprite,
    drag: Option<usize>,
    actors: Vec<(String, Sprite, [i32; 4])>,
    exit_buttons: Option<[[i32; 4]; 2]>,
    initial_speeds: Speeds,
    initial_sound: SoundSettings,
}

impl Settings {
    pub fn open(
        h: &mut dyn HostServices,
        c: ClientId,
        abi: ScreenAbi,
        speeds: Speeds,
        actor_keys: &[String],
        points: &[[i32; 2]],
        sound: &SoundSettings,
        ops: &mut Vec<Value>,
    ) -> Result<Self> {
        let storage = h.storage(c)?;
        let grouped = storage.exists(&super::asset("sys", "config/configbase.png")?);
        let dir = if grouped { "config/" } else { "" };
        let sliders = if !grouped && storage.exists(&super::asset("sys", "topmenu1noload.png")?) {
            Sliders {
                left: 40,
                right: 260,
                hit_y: [[152, 192], [228, 268], [304, 340]],
                draw_x: 43,
                draw_y: [163, 238, 313],
                reversed: false,
            }
        } else {
            Sliders::for_abi(abi)
        };
        let base = sprite(h, c, &["sys"], &format!("{dir}configbase.png"))?;
        ops.push(base.layer(super::title::BACK, 255));
        let knob = sprite(h, c, &["sys"], &format!("{dir}knob.png"))?;
        for (index, speed) in speeds.0.into_iter().enumerate() {
            ops.push(sliders.placed(&knob, index, speed));
        }
        // Coordinates are half-open mouse rectangles, independent of the
        // bitmap extents. Select the executable's voice-panel layout.
        let first = points.first().copied();
        let exit_buttons = match first {
            Some([568, 358]) => Some([[578, 552, 654, 588], [666, 552, 754, 588]]),
            Some([578, 430]) => Some([[592, 600, 648, 640], [701, 600, 775, 640]]),
            Some([926, 273]) => Some([[556, 352, 715, 478], [556, 496, 715, 552]]),
            _ => None,
        };
        let mut actors = Vec::new();
        if !points.is_empty() {
            let mark = sprite(
                h,
                c,
                &["sys"],
                if grouped {
                    "config/button.png"
                } else {
                    "checkmark.png"
                },
            )?;
            for (index, (key, point)) in actor_keys.iter().zip(points).enumerate() {
                let mut art = mark.clone();
                art.rect[..2].copy_from_slice(point);
                if sound.get(key, 1) != 0 {
                    ops.push(art.layer(8010 + index as u32, 255));
                }
                let [x, y] = *point;
                let hit = match (first, x) {
                    (Some([568, 358]), 568) => [558, y - 2, 678, y + 22],
                    (Some([568, 358]), 690) => [678, y - 2, 796, y + 22],
                    (Some([578, 430]), 578) => [578, y + 3, 688, y + 35],
                    (Some([578, 430]), 699) => [699, y + 3, 776, y + 35],
                    (Some([926, 273]), 926) => [924, y, 1064, y + 24],
                    (Some([926, 273]), 1084) => [1080, y, 1220, y + 24],
                    _ => [
                        x,
                        y,
                        x.saturating_add(art.rect[2]),
                        y.saturating_add(art.rect[3]),
                    ],
                };
                actors.push((key.clone(), art, hit));
            }
        }
        Ok(Self {
            actors,
            exit_buttons,
            initial_sound: sound.clone(),
            initial_speeds: speeds,
            sliders,
            knob,
            drag: None,
        })
    }

    pub fn remove(&self, ops: &mut Vec<Value>) {
        ops.extend(KNOBS.map(super::remove));
        ops.extend((0..self.actors.len()).map(|i| super::remove(8010 + i as u32)));
    }

    pub fn input(
        &mut self,
        input: InputFrame,
        speeds: &mut Speeds,
        sound: &mut SoundSettings,
        ops: &mut Vec<Value>,
    ) -> bool {
        if input.buttons_pressed & buttons::MOUSE_LEFT != 0 {
            let hit = |[l, t, r, b]: [i32; 4]| {
                (l..r).contains(&input.pointer_x) && (t..b).contains(&input.pointer_y)
            };
            if let Some([ok, cancel]) = self.exit_buttons {
                if hit(cancel) {
                    *speeds = self.initial_speeds;
                    *sound = self.initial_sound.clone();
                    return true;
                }
                if hit(ok) {
                    return true;
                }
            }
            for (index, (key, art, bounds)) in self.actors.iter().enumerate() {
                if hit(*bounds) {
                    sound.toggle_actor(key);
                    let id = 8010 + index as u32;
                    ops.push(if sound.get(key, 1) != 0 {
                        art.layer(id, 255)
                    } else {
                        super::remove(id)
                    });
                    break;
                }
            }
            self.drag = self.sliders.hit(input.pointer_x, input.pointer_y);
        } else if input.buttons_down & buttons::MOUSE_LEFT == 0 {
            self.drag = None;
        }
        if let Some(index) = self.drag {
            let value = self.sliders.value(input.pointer_x);
            if value != speeds.0[index] {
                speeds.0[index] = value;
                ops.push(self.sliders.placed(&self.knob, index, value));
            }
        }
        false
    }
}
