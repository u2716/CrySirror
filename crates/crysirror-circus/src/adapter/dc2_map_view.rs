//! DC2 process28 map screen view (40FD90 frame loop). Native tables and
//! policy: 40F970 hover rectangles, 40FB10 confirm/right-click rules,
//! 40EFC0 chip animation, 40F930's draw order and clip/blit pairs. Images,
//! composition, audio and input are shared engine services.
use super::*;
use crate::dc2::map::MapWait;
use crate::dc2_bitmap::{Draw, Request as BitmapRequest};
use crysirror_core::service::buttons;
use std::collections::VecDeque;

///Host image slots: background, chip00, chip01, movestop, chips 0..9.
const FIRST_SLOT: u8 = 52;
const SLOTS: u8 = 14;
const CHIPS: [&str; 10] = [
    "MAP\\chip_otome",
    "MAP\\chip_yume",
    "MAP\\chip_anzu",
    "MAP\\chip_nanaka",
    "MAP\\chip_koko",
    "MAP\\chip_minatu",
    "MAP\\chip_mayuki",
    "MAP\\chip_akane",
    "MAP\\chip_maya",
    "MAP\\chip_erika",
];
///501030 frame counts and 5010A0..501338 per-frame durations (ms).
const FRAMES: [&[u32]; 10] = [
    &[500, 500, 500, 500, 500, 500, 500, 500, 500, 200, 200, 200, 500, 400, 500, 500, 500, 500],
    &[500, 500, 500, 500, 500, 500, 500, 500, 500, 500, 500, 500, 100, 200, 200, 400, 700, 200, 200],
    &[500, 500, 500, 500, 500, 500, 500, 500, 200, 1000, 200, 500, 200, 300, 1000, 200, 200, 200],
    &[500, 500, 500, 500, 500, 500, 500, 500, 600, 100, 200, 300, 600, 200, 500, 500, 500, 500],
    &[500, 500, 500, 500, 500, 500, 500, 500, 500, 500, 500, 500, 400, 100, 100, 100, 400, 700, 100, 100],
    &[500, 500, 500, 500, 500, 500, 500, 500, 200, 500, 200, 500, 200, 500, 200, 500, 100, 300, 100, 100, 100, 500],
    &[500, 500, 500, 500, 500, 500, 500, 500, 500, 500, 500, 500, 200, 200, 700, 400, 700],
    &[500, 500, 500, 500, 500, 500, 500, 500, 500, 500, 500, 500, 300, 200, 200, 200, 600, 300],
    &[500, 500, 500, 500, 500, 500, 500, 500, 500, 500, 500, 500, 300, 200, 200, 800],
    &[500, 500, 500, 500, 500, 500, 500, 500, 500, 500, 500, 500, 500, 500, 500, 500, 600, 200, 400, 300, 100, 300, 700, 600],
];
///501478 (school) / 5014E8 (island): place hit rectangles [x1,y1,x2,y2).
const HIT: [[[i32; 4]; 7]; 2] = [
    [[71, 241, 95, 265], [203, 451, 227, 475], [288, 309, 312, 333], [409, 91, 433, 115], [0; 4], [543, 295, 567, 319], [618, 438, 642, 462]],
    [[494, 398, 518, 422], [325, 380, 349, 404], [426, 331, 450, 355], [214, 331, 238, 355], [629, 192, 653, 216], [124, 239, 148, 263], [0; 4]],
];
///501558: confirm (yes, no) rectangles.
const CONFIRM: [[i32; 4]; 2] = [[576, 518, 672, 590], [692, 518, 788, 590]];
///501398 / 501408: chip anchor rectangles per place.
const ANCHOR: [[[i32; 4]; 7]; 2] = [
    [[55, 49, 223, 201], [137, 369, 305, 521], [222, 227, 390, 379], [343, 9, 511, 161], [0; 4], [477, 213, 645, 365], [552, 356, 720, 508]],
    [[466, 322, 634, 474], [297, 304, 465, 456], [398, 255, 566, 407], [186, 255, 354, 407], [601, 116, 769, 268], [96, 163, 264, 315], [0; 4]],
];
///40F100: hovered-place clip [x1,y1,x2,y2) and 501060 blit origin.
const HIGHLIGHT: [&[([i32; 4], [i32; 2])]; 2] = [
    &[
        ([0, 136, 248, 280], [0, 24]),
        ([0, 224, 264, 488], [0, -768]),
        ([144, 216, 352, 368], [144, -200]),
        ([320, 0, 528, 112], [320, 0]),
        ([351, 216, 503, 400], [351, -352]),
        ([504, 208, 704, 368], [504, -48]),
        ([576, 248, 800, 488], [576, -504]),
    ],
    &[
        ([456, 368, 552, 456], [456, 288]),
        ([280, 352, 408, 432], [280, 96]),
        ([384, 304, 488, 392], [384, 136]),
        ([128, 288, 320, 416], [128, -48]),
        ([592, 168, 696, 248], [592, 168]),
        ([80, 216, 184, 288], [80, -248]),
    ],
];
///40F3E0: 24x24 stop markers from movestop x=816.
const STOPS: [[[i32; 2]; 6]; 2] = [
    [[71, 231], [203, 451], [288, 309], [409, 91], [543, 295], [618, 438]],
    [[124, 239], [214, 331], [325, 380], [426, 331], [494, 398], [629, 192]],
];

#[derive(Default)]
pub(super) struct View {
    owner: Option<crate::dc2_bitmap::Owner>,
    loaded: u8,
    ///A273B0 (start) and the next 40 ms tick.
    started: u64,
    next: u64,
    ///A274D1: one 40FD90 frame has run; input is accepted.
    ready: bool,
    ///A274D2: finished; one more frame is drawn before process28 ends.
    done: bool,
    result: Option<i32>,
    ///A274D8 confirm, A274DC menu request, 50109C chosen place.
    confirm: bool,
    menu: bool,
    chosen: i32,
    hover_place: i32,
    hover_button: i32,
    ///A273A8/A273AC: last hovered (SFX edge).
    last_place: i32,
    last_button: i32,
    frames: [(usize, u64); 10],
    drawn: Option<Vec<Draw>>,
    pending: Option<Vec<Draw>>,
    down: u64,
    ///Release edges seen while a frame or sound was still in flight.
    released: u64,
    sounds: VecDeque<u16>,
    sound: Option<crate::dc2_audio::Request>,
    last_sfx: u32,
}
pub(super) enum Frame {
    Busy,
    Waiting(u64),
    Done(i32),
}
fn inside(rect: [i32; 4], x: i32, y: i32) -> bool {
    rect[0] <= x && x < rect[2] && rect[1] <= y && y < rect[3]
}
impl View {
    pub(super) fn close(&mut self, bridge: &mut Bridge<'_>) -> Result<bool> {
        //40EC90 releases every map surface.
        for slot in FIRST_SLOT..FIRST_SLOT + SLOTS {
            if bridge.services.images.contains_key(&slot) {
                match NativeHost::resource(bridge, &crate::machine::resource_ops::ResourceRequest::Unload { slot })? {
                    None => return Ok(false),
                    Some(_) => {}
                }
            }
        }
        *self = Self::default();
        Ok(true)
    }
    pub(super) fn tick(
        &mut self,
        bridge: &mut Bridge<'_>,
        map: &MapWait,
        settings: &crate::dc2_settings::SystemData,
    ) -> Result<Frame> {
        let now = bridge.host.now_ms();
        if self.owner.as_ref() != Some(&map.owner) {
            ensure!(self.owner.is_none(), "DC2 process28 view owner");
            *self = Self { owner: Some(map.owner.clone()), chosen: -1, hover_place: -1, hover_button: -1, last_place: -1, last_button: -1, last_sfx: 2, ..Default::default() };
        }
        //40ECC0: surfaces, then A273B0 and the 40 ms timer.
        let school = usize::from(!map.school);
        while self.loaded < SLOTS {
            let index = self.loaded;
            let name = match index {
                0 => if map.school { "MAP\\moveschool_BG00" } else { "MAP\\moveisland_BG00" },
                1 => if map.school { "MAP\\moveschool_chip00" } else { "MAP\\moveland_chip00" },
                2 => if map.school { "MAP\\moveschool_chip01" } else { "MAP\\moveisland_chip01" },
                3 => "MAP\\movestop",
                chip => {
                    if !map.chips[usize::from(chip - 4)].present {
                        self.loaded += 1;
                        continue;
                    }
                    CHIPS[usize::from(chip - 4)]
                }
            };
            if !NativeHost::emit(bridge, &crate::machine::Event::LoadImage { slot: FIRST_SLOT + index, name: name.into() })? {
                return Ok(Frame::Busy);
            }
            self.loaded += 1;
            if self.loaded == SLOTS {
                self.started = now;
                self.next = now + 40;
                self.frames = [(0, now); 10];
            }
        }
        let input = bridge.input;
        self.released |= self.down & !input.buttons_down;
        self.down = input.buttons_down;
        //Pending SFX first (427070 is synchronous natively).
        loop {
            if let Some(request) = &self.sound {
                if !bridge.dc2_direct_audio(request)? {
                    return Ok(Frame::Busy);
                }
                self.sound = None;
                self.sounds.pop_front();
            }
            let Some(id) = self.sounds.front().copied() else { break };
            if i32::from_le_bytes(settings.bytes()[48..52].try_into()?) == 0 {
                self.sounds.pop_front();
                continue;
            }
            match super::menu::sfx_request(bridge, settings, id, &mut self.last_sfx)? {
                None => return Ok(Frame::Busy),
                Some(None) => {
                    self.sounds.pop_front();
                }
                Some(Some(request)) => self.sound = Some(request),
            }
        }
        if let Some(draw) = self.pending.clone() {
            if !bridge.dc2_bitmap(&BitmapRequest::Present { owner: map.owner.clone(), slot: 1, draw: draw.clone() })? {
                return Ok(Frame::Busy);
            }
            self.drawn = Some(draw);
            self.pending = None;
            if self.done {
                let result = self.result.context("DC2 process28 result")?;
                if !self.close(bridge)? {
                    return Ok(Frame::Busy);
                }
                return Ok(Frame::Done(result));
            }
        }
        let released = std::mem::take(&mut self.released);
        let (x, y) = (input.pointer_x, input.pointer_y);
        if self.ready && !self.done {
            //40FAE0 -> 40F970 while A274DC is clear.
            if !self.menu {
                self.hover(map, school, x, y);
            }
            //40FB10: code 1 on left release, code 2 on right release.
            if released & buttons::MOUSE_LEFT != 0 && !self.menu {
                if !self.confirm {
                    if self.hover_place != -1 {
                        self.confirm = true;
                        self.chosen = self.hover_place;
                        self.sounds.push_back(223);
                    }
                } else {
                    for (index, rect) in CONFIRM.iter().enumerate() {
                        if inside(*rect, x, y) {
                            self.sounds.push_back(223);
                            if index == 0 {
                                self.done = true;
                                self.result = Some(self.hover_place);
                            } else {
                                self.confirm = false;
                                self.chosen = -1;
                                self.hover_place = -1;
                            }
                        }
                    }
                }
            } else if released & buttons::MOUSE_RIGHT != 0 {
                if self.confirm {
                    self.confirm = false;
                    self.chosen = -1;
                    self.hover_place = -1;
                } else if now.saturating_sub(self.started) >= 500 {
                    if self.menu {
                        self.menu = false;
                    } else {
                        //var(5,40)=99: the script opens its right-click menu.
                        self.menu = true;
                        self.done = true;
                        self.result = Some(99);
                    }
                }
            }
        }
        if now < self.next {
            return Ok(if self.sounds.is_empty() { Frame::Waiting(self.next) } else { Frame::Busy });
        }
        //40FD90: 40EFC0 animation, 40F930 draw.
        for (index, (frame, last)) in self.frames.iter_mut().enumerate() {
            let table = FRAMES[index];
            if now.saturating_sub(*last) > u64::from(table[*frame]) {
                *frame = (*frame + 1) % table.len();
                *last = now;
            }
        }
        self.next = now + 40;
        self.ready = true;
        let draw = self.draw(bridge, map, school)?;
        if self.drawn.as_ref() != Some(&draw) || self.done {
            self.pending = Some(draw);
        }
        Ok(Frame::Busy)
    }
    ///40F970 against the current pointer.
    fn hover(&mut self, map: &MapWait, school: usize, x: i32, y: i32) {
        if !self.confirm {
            self.hover_place = -1;
            for (index, rect) in HIT[school].iter().enumerate() {
                if inside(*rect, x, y) {
                    self.hover_place = index as i32;
                    if index as i32 != self.last_place {
                        self.sounds.push_back(224);
                        self.last_place = index as i32;
                    }
                }
            }
            if self.hover_place == -1 {
                self.last_place = -1;
            }
        }
        if self.chosen != -1 {
            self.hover_place = self.chosen;
            self.last_place = self.chosen;
        }
        self.hover_button = -1;
        for (index, rect) in CONFIRM.iter().enumerate() {
            if inside(*rect, x, y) {
                self.hover_button = index as i32;
                if index as i32 != self.last_button && self.confirm {
                    self.sounds.push_back(224);
                    self.last_button = index as i32;
                }
            }
        }
        if self.hover_button == -1 {
            self.last_button = -1;
        }
        let _ = map;
    }
    ///501094: the last character standing on the hovered place.
    fn hover_chip(&self, map: &MapWait) -> i32 {
        let mut chip = -1;
        for (index, entry) in map.chips.iter().enumerate() {
            if entry.place == self.hover_place {
                chip = index as i32;
            }
        }
        chip
    }
    fn draw(&self, bridge: &Bridge<'_>, map: &MapWait, school: usize) -> Result<Vec<Draw>> {
        let extent = |slot: u8| -> Result<[i32; 2]> {
            let size = bridge.services.image_extents.get(&slot).copied().context("DC2 map surface extent")?;
            Ok([i32::try_from(size[0])?, i32::try_from(size[1])?])
        };
        let mut draw = Vec::new();
        //40B060 clip then 40B840/40B1E0 at `origin`.
        let mut blit = |slot: u8, clip: [i32; 4], origin: [i32; 2]| -> Result<()> {
            let [w, h] = extent(slot)?;
            let x1 = clip[0].max(origin[0]).max(0);
            let y1 = clip[1].max(origin[1]).max(0);
            let x2 = clip[2].min(origin[0] + w).min(800);
            let y2 = clip[3].min(origin[1] + h).min(600);
            if x2 > x1 && y2 > y1 {
                draw.push(Draw::Image {
                    slot,
                    source: [x1 - origin[0], y1 - origin[1], x2 - x1, y2 - y1],
                    destination: [x1, y1],
                });
            }
            Ok(())
        };
        let (background, chip00, chip01, stop) = (FIRST_SLOT, FIRST_SLOT + 1, FIRST_SLOT + 2, FIRST_SLOT + 3);
        blit(background, [0, 0, 800, 600], [0, 0])?;
        //40F100
        if let Some((clip, origin)) = usize::try_from(self.hover_place).ok().and_then(|p| HIGHLIGHT[school].get(p)) {
            blit(chip01, *clip, *origin)?;
        }
        //40F890
        if self.confirm {
            for (index, rect) in CONFIRM.iter().enumerate() {
                let lift = if self.hover_button == index as i32 { 464 } else { 393 };
                blit(chip00, *rect, [rect[0] - 96 * index as i32, rect[1] - lift])?;
            }
        }
        //40F830
        if self.hover_place != -1 {
            blit(chip00, [136, 524, 560, 580], [136, 524 - 56 * self.hover_place])?;
        }
        //40F3E0
        for [sx, sy] in STOPS[school] {
            blit(stop, [sx, sy, sx + 24, sy + 24], [sx - 816, sy])?;
        }
        //40F620
        let hovered = self.hover_chip(map);
        for (index, chip) in map.chips.iter().enumerate() {
            if !chip.present {
                continue;
            }
            let (frame, _) = self.frames[index];
            let atlas = [168 * (frame as i32 % 5), 152 * (frame as i32 / 5)];
            let lift = if hovered == index as i32 { 0x2f8 } else { 0 };
            let (mut base, mut top) = (map.shift, 0);
            if map.school && chip.place == 0 {
                top = 100;
                base -= 50;
            }
            let anchor = ANCHOR[school][chip.place as usize];
            let left = base + chip.offset + anchor[0];
            blit(
                FIRST_SLOT + 4 + index as u8,
                [left, anchor[1] + top, base + chip.offset + anchor[2], top + anchor[3]],
                [left - atlas[0], top + anchor[1] - atlas[1] - lift],
            )?;
        }
        //40F780
        if hovered != -1 && map.chips[hovered as usize].present {
            let row = if hovered / 6 > 0 { 480 } else { 160 } - if map.school { 160 } else { 0 };
            blit(stop, [0, 440, 136, 600], [-(136 * (hovered % 6)), 440 - row])?;
        }
        Ok(draw)
    }
}
