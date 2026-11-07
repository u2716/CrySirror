// Copyright (c) 2026 CrySirror contributors. MIT.
//! Native array element opcodes (40CB00 ldelem, 40EE00.. stelem families,
//! 40CD30 subscripts) and the frame click/AIP target opcodes 0x843..0x847
//! (40D1C0/40D2C0/40D300 plus the 40B350 input pass).
use super::*;

/// Mouse/key latch bits mirrored from thread offsets 19512/19516/19520.
pub const CLICK_LEFT: u8 = 1;
pub const CLICK_RIGHT: u8 = 2;
pub const CLICK_MIDDLE: u8 = 4;

impl Machine {
    /// Global arrays are created on first access (41D980/41D6F0); locals and
    /// arguments must already hold an array of the requested element type.
    fn array_variable(&mut self, flags: u16, hash: u32, offset: i16) -> Result<ArrayRef> {
        let element = ((flags >> 8) & 7) as u8;
        ensure!(element <= 2, "Majiro array element type");
        let scope = (flags >> 5) & 7;
        if scope < 3 && !self.banks[scope as usize].contains_key(&hash) {
            ensure!(
                self.banks[scope as usize].len() < 65536,
                "Majiro variable budget"
            );
            self.banks[scope as usize]
                .insert(hash, Value::Array(ArrayRef::new(element, [1, 1, 1])?));
        }
        match self.variable_typed(flags & !0x1800, hash, offset, element + 3)? {
            Value::Array(array) => Ok(array),
            _ => bail!("Majiro array variable required"),
        }
    }
    /// 40CD30 pops `count` subscripts (innermost on top) and wraps each with a
    /// positive modulo of its dimension.
    fn element_index(&mut self, array: &ArrayRef, count: usize) -> Result<usize> {
        ensure!((1..=3).contains(&count), "Majiro array subscript count");
        let subscripts = self
            .tail(count)?
            .iter()
            .map(Value::integer)
            .collect::<Result<Vec<_>>>()?;
        self.stack.truncate(self.stack.len() - count);
        let dims = array.lock().dims;
        let wrap = |value: i32, dim: i32| value.rem_euclid(dim) as usize;
        // subscripts are in push order; dims are [d8, d12, d16].
        let index = match count {
            1 => wrap(subscripts[0], dims[2]),
            2 => wrap(subscripts[1], dims[2]) + wrap(subscripts[0], dims[1]) * dims[2] as usize,
            _ => {
                wrap(subscripts[2], dims[2])
                    + dims[2] as usize
                        * (wrap(subscripts[1], dims[1])
                            + wrap(subscripts[0], dims[0]) * dims[1] as usize)
            }
        };
        ensure!(
            index < array.lock().elements.len(),
            "Majiro array subscript outside allocation"
        );
        Ok(index)
    }
    pub(super) fn load_element(&mut self, flags: u16, hash: u32, offset: i16) -> Result<()> {
        let array = self.array_variable(flags, hash, offset)?;
        let index = self.element_index(&array, usize::from(flags >> 11))?;
        let mut data = array.lock();
        let mut value = data.elements[index].clone();
        let modify = flags & 7;
        if modify != 0 {
            let updated = match &value {
                Value::Integer(n) => Value::Integer(match modify {
                    1 | 3 => n.wrapping_add(1),
                    2 | 4 => n.wrapping_sub(1),
                    _ => bail!("Majiro element modifier"),
                }),
                Value::Float(n) => Value::Float(match modify {
                    1 | 3 => n + 1.0,
                    2 | 4 => n - 1.0,
                    _ => bail!("Majiro element modifier"),
                }),
                _ => bail!("Majiro string element modifier"),
            };
            if modify <= 2 {
                value = updated.clone();
            }
            data.elements[index] = updated;
        }
        drop(data);
        value = match (flags >> 3) & 3 {
            0 => value,
            invert => match value {
                Value::Integer(n) => Value::Integer(match invert {
                    1 => n.wrapping_neg(),
                    2 => i32::from(n == 0),
                    _ => !n,
                }),
                Value::Float(n) if invert == 1 => Value::Float(-n),
                _ => bail!("Majiro element invert type"),
            },
        };
        self.push(value)
    }
    pub(super) fn store_element(&mut self, op: u16, flags: u16, hash: u32, offset: i16) -> Result<()> {
        let base = if op >= 0x2d0 { 0x2d0 } else { 0x270 };
        let group = (op - base) / 8;
        let ty = ((op - base) % 8) as u8;
        let array = self.array_variable(flags, hash, offset)?;
        let index = self.element_index(&array, usize::from(flags >> 11))?;
        let rhs = self.tail(1)?[0].clone();
        let element = array.lock().element;
        let value = if group == 0 {
            match (element, rhs) {
                (1, Value::Integer(n)) => Value::Float(n as f32),
                (0, Value::Float(n)) => Value::Integer(n as i32),
                (_, rhs) => rhs,
            }
        } else {
            let old = array.lock().elements[index].clone();
            let operator = match group {
                1 => 0x100,
                2 => 0x108,
                3 => 0x110,
                4 => 0x118,
                5 => 0x120,
                6 => 0x130,
                7 => 0x128,
                8 => 0x180,
                9 => 0x168,
                10 => 0x188,
                _ => bail!("Majiro element assignment group"),
            } | u16::from(ty);
            scalar::binary(operator, &old, &rhs)?
        };
        ensure!(value.kind() == element, "Majiro element store type");
        array.lock().elements[index] = value.clone();
        self.stack.pop();
        if base == 0x270 {
            self.push(value)?;
        }
        Ok(())
    }
    /// 0x843 sets all three click targets; a latched click jumps immediately.
    pub(super) fn bsel_set_click(&mut self, target: u32) -> Option<u32> {
        let frame = self.frames.last_mut().unwrap();
        frame.bsel.click = [target; 3];
        let latch = self.clicks;
        for (bit, slot) in [(CLICK_LEFT, 0), (CLICK_MIDDLE, 2), (CLICK_RIGHT, 1)] {
            if latch & bit != 0 {
                let frame = self.frames.last_mut().unwrap();
                let jump = frame.bsel.click[slot];
                frame.bsel.click[slot] = 0;
                self.clicks &= !bit;
                return Some(jump);
            }
        }
        None
    }
    /// 0x83B/0x83D/0x83C set one click target (left/right/middle).
    pub(super) fn bsel_set_one(&mut self, slot: usize, bit: u8, target: u32) -> Option<u32> {
        self.frames.last_mut().unwrap().bsel.click[slot] = target;
        if self.clicks & bit != 0 {
            self.frames.last_mut().unwrap().bsel.click[slot] = 0;
            self.clicks &= !bit;
            return Some(target);
        }
        None
    }
    /// 0x846 resumes at the nearest frame whose AIP4 lies ahead of its PC.
    pub(super) fn bsel_aip4(&mut self) -> Option<u32> {
        let top = self.frames.len() - 1;
        for i in (0..=top).rev() {
            let pc = if i == top { self.pc } else { self.frames[i + 1].return_pc };
            let target = self.frames[i].bsel.aip4;
            if target > pc {
                self.frames[i].bsel.aip4 = 0;
                if i == top {
                    return Some(target);
                }
                self.frames[i + 1].return_pc = target;
                return None;
            }
        }
        None
    }
    /// 40B350 input pass for the current thread: an observed click jumps every
    /// frame whose matching target lies ahead. A parked native call in the top
    /// frame is abandoned (its wait deadline is cleared natively).
    fn click_current(&mut self, latch: u8) -> bool {
        self.clicks |= latch;
        let mut jumped = false;
        let top = self.frames.len() - 1;
        for i in 0..=top {
            for (bit, slot) in [(CLICK_LEFT, 0), (CLICK_RIGHT, 1), (CLICK_MIDDLE, 2)] {
                if latch & bit == 0 {
                    continue;
                }
                let pc = if i == top { self.pc } else { self.frames[i + 1].return_pc };
                let target = self.frames[i].bsel.click[slot];
                if target > pc {
                    self.frames[i].bsel.click = [0; 3];
                    if i == top {
                        if let Some((_, _, consumed)) = self.pending.take() {
                            let keep = self.stack.len().saturating_sub(consumed);
                            self.stack.truncate(keep);
                        }
                        self.pc = target;
                        self.clicks = 0;
                        jumped = true;
                    } else {
                        self.frames[i + 1].return_pc = target;
                    }
                }
            }
        }
        jumped
    }
    /// Deliver one owner input observation to every live script thread.
    /// Returns the ids of threads whose current frame jumped.
    pub fn deliver_clicks(&mut self, latch: u8) -> Result<Vec<i32>> {
        if latch == 0 {
            return Ok(vec![]);
        }
        let caller = self.scheduler.current;
        let mut jumped = vec![];
        for id in self.scheduler.order.clone() {
            if !self.thread_alive(id) {
                continue;
            }
            self.switch_thread(id)?;
            if self.click_current(latch) {
                jumped.push(id);
                self.wake_thread(id);
            }
        }
        self.switch_thread(caller)?;
        for id in &jumped {
            self.scheduler.blocked.remove(id);
        }
        Ok(jumped)
    }
    /// mk_read consumes the current thread's latched clicks.
    pub fn take_clicks(&mut self) -> u8 {
        std::mem::take(&mut self.clicks)
    }
}
