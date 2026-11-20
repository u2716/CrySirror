// SPDX-License-Identifier: MPL-2.0
//! CVNS input block `I` at engine +1604 (`this[401]`), refreshed by `0x412460`.
//!
//! `0x436a70` counts the frames each key has been held; 1 marks a new press and
//! consumers write 2. Window messages latch the mouse button presses that
//! `0x412460` exposes for one frame.

use crysirror_core::service::{buttons as key, InputFrame};

/// `0x436a70` key indices and their virtual keys.
const KEYS: [(usize, u64); 24] = [
    (29, key::ENTER),
    (33, key::CTRL),
    (34, key::SHIFT),
    (45, key::SPACE),
    (46, key::UP),
    (47, key::DOWN),
    (48, key::LEFT),
    (49, key::RIGHT),
    (38, key::ESCAPE),
    (50, key::PAGE_UP),
    (51, key::PAGE_DOWN),
    (39, key::TAB),
    (99, key::F1),
    (100, key::F2),
    (101, key::F3),
    (102, key::F4),
    (103, key::F5),
    (104, key::F6),
    (105, key::F7),
    (106, key::F8),
    (107, key::F9),
    (108, key::F10),
    (109, key::F11),
    (110, key::F12),
];

#[derive(Default)]
pub(super) struct Input {
    /// Words of the native block indexed like `I[k]`.
    pub words: Vec<u32>,
    host: InputFrame,
    pressed: u64,
}

impl Input {
    pub fn new() -> Self {
        Self {
            words: vec![0; 111],
            ..Self::default()
        }
    }

    /// Accumulates host input between native frames.
    pub fn admit(&mut self, frame: InputFrame) {
        self.pressed |= frame.buttons_pressed;
        self.host = frame;
    }

    pub fn consume_movie_skip(&mut self) -> bool {
        let buttons = key::MOUSE_LEFT | key::ENTER;
        let skip = self.pressed & buttons != 0;
        self.pressed &= !buttons;
        if skip && self.host.buttons_down & key::ENTER != 0 {
            self.words[29] = 2;
        }
        skip
    }

    /// `0x412460` at the start of a native frame.
    pub fn begin_frame(&mut self) {
        let down = self.host.buttons_down;
        for (index, button) in KEYS {
            self.words[index] = if down & button != 0 { self.words[index].saturating_add(1) } else { 0 };
        }
        self.words[16] = self.host.pointer_x as u32;
        self.words[17] = self.host.pointer_y as u32;
        self.words[18] = u32::from(self.pressed & key::MOUSE_LEFT != 0);
        self.words[19] = u32::from(self.pressed & key::MOUSE_RIGHT != 0);
        self.pressed = 0;
    }

    pub fn pointer(&self) -> [i32; 2] {
        [self.words[16] as i32, self.words[17] as i32]
    }

    /// `0x436d90`: the input mask; `consume` marks reported presses as used.
    pub fn mask(&mut self, consume: bool) -> u32 {
        self.edges(u32::MAX, consume)
    }

    /// `0x437140`: whether any selected input is pending; `consume` uses only those.
    pub fn take(&mut self, selection: u32, consume: bool) -> bool {
        self.edges(selection, consume) != 0
    }

    fn edges(&mut self, selection: u32, consume: bool) -> u32 {
        // Mouse presses and wheel steps are flags; keys report only their first frame.
        const FLAGS: [(usize, u32); 4] = [(18, 1), (19, 2), (22, 0x100_0000), (23, 0x200_0000)];
        const KEY_EDGES: [(usize, u32); 28] = [
            (29, 4),
            (45, 8),
            (39, 0x10),
            (38, 0x20),
            (50, 0x40),
            (51, 0x80),
            (46, 0x100),
            (47, 0x200),
            (48, 0x400),
            (49, 0x800),
            (99, 0x1000),
            (100, 0x2000),
            (101, 0x4000),
            (102, 0x8000),
            (103, 0x1_0000),
            (104, 0x2_0000),
            (105, 0x4_0000),
            (106, 0x8_0000),
            (107, 0x10_0000),
            (108, 0x20_0000),
            (109, 0x40_0000),
            (110, 0x80_0000),
            (8, 0x400_0000),
            (10, 0x800_0000),
            (0, 0x1000_0000),
            (2, 0x2000_0000),
            (4, 0x4000_0000),
            (6, 0x8000_0000),
        ];
        let mut mask = 0;
        for (index, bit) in FLAGS {
            if selection & bit != 0 && self.words[index] != 0 {
                mask |= bit;
                if consume {
                    self.words[index] = 0;
                }
            }
        }
        for (index, bit) in KEY_EDGES {
            if selection & bit != 0 && self.words[index] == 1 {
                mask |= bit;
                if consume {
                    self.words[index] = 2;
                }
            }
        }
        mask
    }
}
