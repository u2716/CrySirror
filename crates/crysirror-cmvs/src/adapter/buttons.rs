// SPDX-License-Identifier: MPL-2.0

//! Native title-button records and transitions over CrySirror layers and input.

use super::visual::{ChildImage, Objects};
use crate::vm::CmvsVm;
use anyhow::{ensure, Context, Result};
use crysirror_core::{region_map::contains_rect, service::HostServices, state::ClientId};

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct Button {
    id: u32,
    enabled: bool,
    rect: [i32; 4],
    links: [i32; 2],
    images: [ChildImage; 4],
    /// Second rectangle from `0x2219`; the hit test in 0x14001b630 reads only `rect`.
    #[serde(default)]
    alternate: [i32; 4],
}

impl Button {
    fn hit(&self, point: [i32; 2]) -> bool {
        self.enabled && contains_rect(self.rect.map(i64::from), point.map(i64::from))
    }

    fn centre(&self) -> [i32; 2] {
        let [x, y, w, h] = self.rect;
        [x.saturating_add(w / 2), y.saturating_add(h / 2)]
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct Group {
    object: usize,
    pointer_size: [u32; 2],
    buttons: Vec<Button>,
    hover: Option<usize>,
    pressed: Option<usize>,
    pressed_visual: bool,
    /// List objects (`0x2370`, class 0x14001b5c0) only.
    #[serde(default)]
    list: Option<List>,
}

/// List record +72..+100.
#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
struct List {
    /// +72 (`0x237c`): a decided or cancelled update leaves the input edge for the script.
    keep_input: bool,
    /// +76..+88 (`0x237d`): pointer hits count only inside this rectangle.
    clip: [i32; 4],
    /// +92 (`0x237e`): no keyboard movement between items.
    no_keys: bool,
    /// +96/+100 (`0x237f`): the pointer relative to the item it last hit, else -1.
    relative: [i32; 2],
}

#[derive(Default, Clone, serde::Serialize, serde::Deserialize)]
pub(super) struct Groups {
    /// Slots 0..=5 are button groups; 6..=13 are the `0x2370` list objects.
    groups: [Option<Group>; 14],
    merged: bool,
}

impl Groups {
    pub fn move_object(&mut self, from: usize, to: usize) {
        for group in &mut self.groups {
            match group.as_mut() {
                Some(record) if record.object == from => record.object = to,
                Some(record) if record.object == to => *group = None,
                _ => {}
            }
        }
    }

    pub fn apply(
        &mut self,
        opcode: u16,
        vm: &mut CmvsVm,
        objects: &mut Objects,
        host: &mut dyn HostServices,
        client: ClientId,
        canvas: [u32; 2],
    ) -> Result<()> {
        if opcode == 0x2087 {
            let [flag] = vm.owner_arguments(opcode)?;
            vm.complete_owner_call(1, None)?;
            self.merged = flag != 0;
            return Ok(());
        }
        let [slot] = vm.owner_arguments(opcode)?;
        let group = self
            .groups
            .get_mut(slot as usize)
            .context("CMVS button group outside 0..5")?;
        let (words, value) = match opcode {
            0x2210 | 0x221b => {
                let (object, pointer_size, words) = if opcode == 0x221b {
                    let [_, object, width, height] = vm.owner_arguments(opcode)?;
                    ensure!(width > 0 && height > 0, "CMVS button pointer scale");
                    (object, [width, height], 4)
                } else {
                    let [_, object] = vm.owner_arguments(opcode)?;
                    (object, canvas, 2)
                };
                ensure!(
                    object < 256 && objects.contains(object as usize),
                    "CMVS button group needs a scene object"
                );
                // 0x14001b5c0 clips list hits to the screen until 0x237d sets a rectangle.
                let list = (slot >= 6).then_some(List {
                    keep_input: false,
                    clip: [0, 0, canvas[0] as i32, canvas[1] as i32],
                    no_keys: false,
                    relative: [-1; 2],
                });
                *group = Some(Group {
                    object: object as usize,
                    pointer_size,
                    buttons: Vec::new(),
                    hover: None,
                    pressed: None,
                    pressed_visual: false,
                    list,
                });
                (words, None)
            }
            0x2211 => {
                *group = None;
                (1, None)
            }
            opcode => {
                let group = group.as_mut().context("CMVS button group missing")?;
                match opcode {
                    0x2212 => {
                        let [_, id] = vm.owner_arguments(opcode)?;
                        ensure!(
                            group.buttons.len() < 1024,
                            "CMVS button count exceeds limit"
                        );
                        let prev = group.buttons.last_mut().map_or(-1, |tail| {
                            tail.links[1] = id as i32;
                            tail.id as i32
                        });
                        group.buttons.push(Button {
                            id,
                            enabled: true,
                            rect: [0; 4],
                            links: [prev, -1],
                            images: [ChildImage::default(); 4],
                            alternate: [0; 4],
                        });
                        (2, Some(1))
                    }
                    0x2213 => {
                        let [_, id, x, y, w, h] = vm.owner_arguments(opcode)?;
                        if let Some(button) = group.buttons.iter_mut().find(|b| b.id == id) {
                            button.rect = [x, y, w, h].map(|v| v as i32);
                        }
                        (6, None)
                    }
                    0x2214 => {
                        let [_, id, state, child, x, y, w, h, ox, oy] =
                            vm.owner_arguments(opcode)?;
                        if let Some(button) = group.buttons.iter_mut().find(|b| b.id == id) {
                            let image = ChildImage {
                                child,
                                crop: [x, y, w, h].map(|v| v as i16),
                                origin: [ox, oy].map(|v| v as i16),
                            };
                            let targets: &[usize] = match (self.merged, state) {
                                (_, 0) => &[0],
                                (false, 1) => &[1],
                                (false, 2) => &[2],
                                (false, 3) | (true, 2) => &[3],
                                (true, 1) => &[1, 2],
                                _ => &[],
                            };
                            for &index in targets {
                                button.images[index] = image;
                            }
                        }
                        (10, None)
                    }
                    0x2215 => {
                        let images: Vec<_> = group
                            .buttons
                            .iter()
                            .map(|b| b.images[if b.enabled { 0 } else { 3 }])
                            .collect();
                        objects.button_images(host, client, group.object, &images, true)?;
                        group.hover = None;
                        (1, None)
                    }
                    0x2216 => (
                        1,
                        Some(group.hover.map_or(u32::MAX, |i| group.buttons[i].id)),
                    ),
                    0x2218 => {
                        group.pressed = None;
                        group.pressed_visual = false;
                        (1, None)
                    }
                    0x2219 => {
                        let [_, id, x, y, width, height] = vm.owner_arguments(opcode)?;
                        if let Some(button) = group.buttons.iter_mut().find(|button| button.id == id)
                        {
                            button.alternate = [x, y, width, height].map(|value| value as i32);
                        }
                        (6, None)
                    }
                    0x2217 => (1, Some(group.update(vm, objects, host, client, canvas)?)),
                    0x221a => {
                        let [_, id, warp] = vm.owner_arguments(opcode)?;
                        let target = group
                            .buttons
                            .iter()
                            .position(|b| b.id == id)
                            .filter(|&i| group.buttons[i].enabled);
                        if let Some(i) = target {
                            let images = group.hover_images(target);
                            objects.button_images(host, client, group.object, &images, false)?;
                            let button = &group.buttons[i];
                            if warp != 0
                                && !button.hit(scale(vm.pointer(), canvas, group.pointer_size))
                            {
                                move_pointer(
                                    vm,
                                    host,
                                    client,
                                    scale(button.centre(), group.pointer_size, canvas),
                                )?;
                            }
                            group.hover = target;
                        }
                        (3, Some(u32::from(target.is_some())))
                    }
                    _ => unreachable!("recognized button opcode"),
                }
            }
        };
        vm.complete_owner_call(words, value)?;
        Ok(())
    }
}

impl Groups {
    /// List commands `0x237c..=0x237f`. False for a missing list (fault `0x100000`).
    pub fn list_command(&mut self, opcode: u16, vm: &mut CmvsVm) -> Result<bool> {
        let words = match opcode {
            0x237d => 5,
            0x237f => 1,
            _ => 2,
        };
        let [index] = vm.owner_arguments(opcode)?;
        let list = (index < 8)
            .then(|| self.groups[6 + index as usize].as_mut())
            .flatten()
            .and_then(|group| group.list.as_mut());
        let Some(list) = list else {
            vm.complete_owner_call(words, None)?;
            return Ok(false);
        };
        match opcode {
            0x237c => {
                let [_, flag] = vm.owner_arguments(opcode)?;
                list.keep_input = flag != 0;
            }
            0x237d => {
                let [_, x, y, width, height] = vm.owner_arguments(opcode)?;
                list.clip = [x, y, width, height].map(|value| value as i32);
            }
            0x237e => {
                let [_, flag] = vm.owner_arguments(opcode)?;
                list.no_keys = flag != 0;
            }
            _ => {
                let [x, y] = list.relative;
                vm.complete_owner_results(words, &[(1, x as u32), (2, y as u32)])?;
                return Ok(true);
            }
        }
        vm.complete_owner_call(words, None)?;
        Ok(true)
    }
}

impl Group {
    fn hover_images(&self, next: Option<usize>) -> Vec<ChildImage> {
        if next == self.hover {
            return Vec::new();
        }
        self.hover
            .map(|i| self.buttons[i].images[0])
            .into_iter()
            .chain(next.map(|i| self.buttons[i].images[1]))
            .collect()
    }

    fn update(
        &mut self,
        vm: &mut CmvsVm,
        objects: &mut Objects,
        host: &mut dyn HostServices,
        client: ClientId,
        canvas: [u32; 2],
    ) -> Result<u32> {
        let keys = self.list.is_none_or(|list| !list.no_keys);
        let previous = keys && vm.action(2)[0];
        let next = keys && vm.action(3)[0];
        let mut hover = None;
        if previous || next {
            let link = self
                .hover
                .map_or(-1, |i| self.buttons[i].links[usize::from(next)]);
            hover = if link >= 0 {
                self.buttons.iter().position(|b| b.id == link as u32)
            } else if next {
                (!self.buttons.is_empty()).then_some(0)
            } else {
                self.buttons.len().checked_sub(1)
            };
        }
        let anchor = if self.list.is_some() { objects.anchor(self.object) } else { [0; 2] };
        let centre = hover.map(|i| {
            let [x, y] = self.buttons[i].centre();
            [x + anchor[0], y + anchor[1]]
        });
        let point = centre.unwrap_or_else(|| scale(vm.pointer(), canvas, self.pointer_size));
        let local = [point[0] - anchor[0], point[1] - anchor[1]];
        match &mut self.list {
            // 0x14001bb00: hits only inside the clip rectangle, measured from the object.
            Some(list) => {
                list.relative = [-1; 2];
                if contains_rect(list.clip.map(i64::from), point.map(i64::from)) {
                    if let Some(hit) = self.buttons.iter().rposition(|button| button.hit(local)) {
                        let [x, y, ..] = self.buttons[hit].rect;
                        list.relative = [local[0] - x, local[1] - y];
                        hover = Some(hit);
                    }
                }
            }
            None => {
                if let Some(hit) = self.buttons.iter().rposition(|button| button.hit(point)) {
                    hover = Some(hit);
                }
            }
        }
        let mut images = self.hover_images(hover);
        let mut pressed_visual = hover == self.hover && self.pressed_visual;
        let [released, held, pressed] = vm.action(0);
        let cancel = !released && vm.action(1)[0];
        let mut press_target = if pressed { hover } else { self.pressed };
        let result = if released {
            hover
                .filter(|&i| press_target == Some(i))
                .map_or(-10i32 as u32, |i| self.buttons[i].id)
        } else {
            if !held {
                press_target = None;
            }
            if held != pressed_visual {
                if let Some(i) = hover {
                    images.push(self.buttons[i].images[if held { 2 } else { 1 }]);
                    pressed_visual = held;
                }
            }
            if cancel {
                -2i32 as u32
            } else {
                u32::MAX
            }
        };
        objects.button_images(host, client, self.object, &images, false)?;
        if let Some(centre) = centre {
            move_pointer(vm, host, client, scale(centre, self.pointer_size, canvas))?;
        }
        let keep = self.list.is_some_and(|list| list.keep_input);
        for (index, consume) in [released && !keep, cancel && !keep, previous, next].into_iter().enumerate() {
            if consume {
                vm.consume_action(index);
            }
        }
        self.hover = hover;
        self.pressed = press_target;
        self.pressed_visual = pressed_visual;
        Ok(result)
    }
}

fn scale(point: [i32; 2], from: [u32; 2], to: [u32; 2]) -> [i32; 2] {
    std::array::from_fn(|i| {
        let value = i64::from(point[i]) * i64::from(to[i]) / i64::from(from[i]);
        value.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
    })
}

fn move_pointer(
    vm: &mut CmvsVm,
    host: &mut dyn HostServices,
    client: ClientId,
    point: [i32; 2],
) -> Result<()> {
    host.warp_pointer(client, point[0], point[1])?;
    vm.move_pointer(point);
    Ok(())
}
