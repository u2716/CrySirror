// SPDX-License-Identifier: MPL-2.0
//! Native key.cfg bindings and per-frame action edges.

use super::{evaluate::error, machine::Machine, DecodeError};
use anyhow::{ensure, Result};
use crysirror_core::service::{buttons as key, InputFrame};

const QUERIES: &[(u16, usize)] = &[
    (0x219c, 15),
    (0x219e, 16),
    (0x21a0, 0),
    (0x21a2, 1),
    (0x21a4, 7),
    (0x21a6, 8),
    (0x21a8, 6),
    (0x21aa, 9),
    (0x21ac, 10),
    (0x21ae, 11),
    (0x21b0, 12),
    (0x21b2, 13),
    (0x21b4, 14),
    (0x21b6, 2),
    (0x21b8, 3),
    (0x21ba, 4),
    (0x21bc, 5),
    (0x2344, 22),
    (0x2346, 23),
    (0x234a, 20),
    (0x22f0, 17),
    (0x22f2, 18),
    (0x22f4, 19),
    (0x2348, 21),
    (0x21de, END_ACTION),
];

/// Input object state at +1520 (`0x21de`/`0x21df`): native key code 29 (End), outside
/// the 24 key.cfg functions (0x14006a820).
const END_ACTION: usize = 24;

#[derive(Default)]
pub(super) struct Actions {
    bindings: [u64; 24],
    down: u64,
    host: InputFrame,
    pending_presses: u64,
    pressed: [bool; 25],
    released: [bool; 25],
    held: [bool; 25],
    mouse: [[bool; 3]; 4],
    pointer: Option<[i32; 2]>,
    physical_pointer: Option<[i32; 2]>,
    moved: bool,
    function_keys_enabled: bool,
    function_key: u32,
}

impl Actions {
    pub fn configure(&mut self, bytes: &[u8]) -> Result<()> {
        let (text, errors) = encoding_rs::SHIFT_JIS.decode_without_bom_handling(bytes);
        ensure!(!errors, "invalid CMVS key configuration");
        let mut action = None;
        let mut codes = [[0; 5]; 24];
        for line in text.lines().map(str::trim) {
            if let Some(heading) = line
                .strip_prefix('[')
                .and_then(|line| line.strip_suffix(']'))
            {
                action = heading
                    .strip_prefix("KEY_FUNCTION_")
                    .and_then(parse_index::<24>);
                continue;
            }
            let Some(action) = action else {
                continue;
            };
            let Some((name, value)) = line.split_once('=') else {
                continue;
            };
            if let Some(index) = name
                .trim()
                .strip_prefix("KEY_DATA_")
                .and_then(parse_index::<5>)
            {
                codes[action][index] = host_key(value.trim().parse()?);
            }
        }
        self.bindings = codes.map(|action| action.into_iter().fold(0, |mask, key| mask | key));
        Ok(())
    }

    pub fn admit(&mut self, frame: InputFrame) {
        self.host = frame;
        self.pending_presses |= frame.buttons_pressed;
    }

    #[cfg(feature = "runtime")]
    pub fn suspend(&mut self) {
        self.pending_presses = 0;
        self.down = 0;
        self.pressed.fill(false);
        self.released.fill(false);
        self.held.fill(false);
        self.mouse.fill([false; 3]);
        self.moved = false;
    }

    pub fn begin_frame(&mut self) {
        let down = self.host.buttons_down | std::mem::take(&mut self.pending_presses);
        let pressed = down & !self.down;
        let released = self.down & !down;
        self.down = down;
        let fixed = [host_key(29)];
        for (index, &binding) in self.bindings.iter().chain(&fixed).enumerate() {
            self.pressed[index] = binding & pressed != 0;
            self.released[index] = binding & released != 0;
            self.held[index] = binding & down != 0;
        }
        for (index, button) in [
            key::MOUSE_LEFT,
            key::MOUSE_RIGHT,
            key::WHEEL_UP,
            key::WHEEL_DOWN,
        ]
        .into_iter()
        .enumerate()
        {
            let release = if index < 2 { released } else { pressed };
            self.mouse[index] = [
                release & button != 0,
                down & button != 0,
                pressed & button != 0,
            ];
        }
        let pointer = [self.host.pointer_x, self.host.pointer_y];
        self.moved = self.physical_pointer.is_some_and(|last| last != pointer);
        if self.physical_pointer != Some(pointer) {
            self.pointer = Some(pointer);
            self.physical_pointer = Some(pointer);
        }
        if self.function_keys_enabled && pressed & ((key::F12 << 1) - key::F1) != 0 {
            let function_keys = pressed & ((key::F12 << 1) - key::F1);
            self.function_key = 1 + function_keys.trailing_zeros() - key::F1.trailing_zeros();
        }
    }

    pub fn held_skip(&self) -> bool {
        self.held[7] || self.held[8]
    }

    pub fn confirm_released(&mut self) -> bool {
        if !self.released[0] {
            return false;
        }
        self.consume(0);
        true
    }

    pub(super) fn consume(&mut self, action: usize) {
        self.pressed[action] = false;
        self.released[action] = false;
    }

    #[cfg(feature = "runtime")]
    pub(super) fn state(&self, action: usize) -> [bool; 3] {
        [
            self.released[action],
            self.held[action],
            self.pressed[action],
        ]
    }

    pub(super) fn pointer(&self) -> [i32; 2] {
        self.pointer.unwrap_or([-1, -1])
    }

    pub(super) fn mouse(&self, index: usize) -> [bool; 3] {
        self.mouse[index]
    }

    pub(super) fn clear_edges(&mut self) {
        self.pressed.fill(false);
        self.released.fill(false);
        for flags in &mut self.mouse {
            flags[2] = false;
        }
    }

    pub(super) fn clear_all_edges(&mut self) {
        self.clear_edges();
        for flags in &mut self.mouse {
            flags[0] = false;
        }
    }

    #[cfg(feature = "runtime")]
    pub(super) fn move_pointer(&mut self, point: [i32; 2]) {
        self.pointer = Some(point);
    }

    pub fn command(
        &mut self,
        machine: &mut Machine,
        opcode: u16,
        script_skip: bool,
    ) -> Result<bool, DecodeError> {
        let pc = machine.pc();
        let put = |machine: &mut Machine, index, value| {
            machine
                .state_mut()
                .set_system_integer(index, value)
                .map_err(|why| error(pc, opcode, why))
        };
        match opcode {
            0x20c8 => {
                let [x, y] = self.pointer.unwrap_or([-1, -1]);
                put(machine, 1, x as u32)?;
                put(machine, 2, y as u32)?;
            }
            0x20c9 => put(machine, 0, u32::from(self.moved))?,
            0x219a => {
                self.clear_all_edges();
            }
            0x22ef => put(machine, 0, u32::from(script_skip || self.held_skip()))?,
            0x234d => self.function_keys_enabled = true,
            0x234e => put(machine, 0, self.function_key)?,
            0x234f => self.function_key = 0,
            0x235c | 0x235d => put(
                machine,
                0,
                u32::from(self.pressed[(opcode - 0x235c) as usize]),
            )?,
            _ => {
                let query = opcode & !1;
                let Some(&(_, action)) = QUERIES.iter().find(|&&(op, _)| op == query) else {
                    return Ok(false);
                };
                if opcode == query {
                    put(machine, 0, u32::from(self.released[action]))?;
                    put(machine, 4, u32::from(self.held[action]))?;
                } else if action != 7 && action != 8 {
                    self.consume(action);
                } else {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }
}

fn parse_index<const MAX: usize>(value: &str) -> Option<usize> {
    let index: usize = value.parse().ok()?;
    (1..=MAX).contains(&index).then(|| index - 1)
}

fn host_key(code: u16) -> u64 {
    match code {
        1 => key::MOUSE_LEFT,
        2 => key::MOUSE_MIDDLE,
        3 => key::MOUSE_RIGHT,
        4 => key::WHEEL_UP,
        5 => key::WHEEL_DOWN,
        20 => key::BACKSPACE,
        21 => key::TAB,
        22 => key::ENTER,
        23 => key::SHIFT,
        24 => key::CTRL,
        25 => key::ESCAPE,
        26 => key::SPACE,
        27 => key::PAGE_UP,
        28 => key::PAGE_DOWN,
        29 => key::END,
        30 => key::HOME,
        31 => key::INSERT,
        32 => key::DELETE,
        33 => key::UP,
        34 => key::DOWN,
        35 => key::LEFT,
        36 => key::RIGHT,
        52..=63 => key::F1 << (code - 52),
        76..=85 => key::KEY_0 << (code - 76),
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn releasing_one_native_binding_does_not_clear_another_held_binding() {
        let mut actions = Actions::default();
        actions
            .configure(b"[KEY_FUNCTION_01]\nKEY_DATA_01=1\nKEY_DATA_02=22\n")
            .unwrap();
        actions.admit(InputFrame {
            buttons_down: key::ENTER | key::MOUSE_LEFT,
            ..InputFrame::default()
        });
        actions.begin_frame();
        assert!(actions.held[0] && actions.pressed[0]);

        actions.admit(InputFrame {
            buttons_down: key::ENTER,
            ..InputFrame::default()
        });
        actions.begin_frame();
        assert!(actions.released[0] && actions.held[0]);
        assert!(actions.confirm_released());
        assert!(!actions.released[0] && actions.held[0]);
    }
}
