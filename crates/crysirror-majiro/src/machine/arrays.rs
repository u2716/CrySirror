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
    pub(super) fn store_element(
        &mut self,
        op: u16,
        flags: u16,
        hash: u32,
        offset: i16,
    ) -> Result<()> {
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
    /// 40B350 / 40D2C0 clear frame wait words68/72 on a forward branch.
    /// Retire the old service identity, including a parent call parked beneath
    /// a callback. No abandoned native event may resume at the branch target.
    fn branch_frame(&mut self, index: usize, target: u32) -> Result<()> {
        self.edit_thread_state(
            self.scheduler.current,
            "frame branch",
            |state, next_frame| state.branch_frame(index, target, next_frame),
        )
    }
    /// 0x843 sets all three click targets; a latched click jumps immediately.
    pub(super) fn bsel_set_click(&mut self, target: u32) -> Result<Option<u32>> {
        let frame = self.frames.last_mut().unwrap();
        frame.bsel.click = [target; 3];
        let latch = self.clicks;
        for (bit, slot) in [(CLICK_LEFT, 0), (CLICK_MIDDLE, 2), (CLICK_RIGHT, 1)] {
            if latch & bit != 0 {
                let frame = self.frames.last_mut().unwrap();
                let jump = frame.bsel.click[slot];
                frame.bsel.click[slot] = 0;
                self.clicks &= !bit;
                self.branch_frame(self.frames.len() - 1, jump)?;
                return Ok(Some(jump));
            }
        }
        Ok(None)
    }
    /// 0x83B/0x83D/0x83C set one click target (left/right/middle).
    pub(super) fn bsel_set_one(
        &mut self,
        slot: usize,
        bit: u8,
        target: u32,
    ) -> Result<Option<u32>> {
        self.frames.last_mut().unwrap().bsel.click[slot] = target;
        if self.clicks & bit != 0 {
            self.frames.last_mut().unwrap().bsel.click[slot] = 0;
            self.clicks &= !bit;
            self.branch_frame(self.frames.len() - 1, target)?;
            return Ok(Some(target));
        }
        Ok(None)
    }
    /// 0x846 resumes at the nearest frame whose AIP4 lies ahead of its PC.
    pub(super) fn bsel_aip4(&mut self) -> Result<Option<u32>> {
        let top = self.frames.len() - 1;
        for i in (0..=top).rev() {
            let pc = if i == top {
                self.pc
            } else {
                self.frames[i + 1].return_pc
            };
            let target = self.frames[i].bsel.aip4;
            if target > pc {
                self.frames[i].bsel.aip4 = 0;
                self.branch_frame(i, target)?;
                return Ok((i == top).then_some(target));
            }
        }
        Ok(None)
    }
    /// YUKIIRO411630/classic40B350 directly applies its427240/41E180 live
    /// thread snapshot without selecting a running context. A click jumps each
    /// frame whose target lies ahead and abandons its corresponding wait.
    /// Deliver one owner input observation to every live script thread.
    /// Returns the ids of threads whose current frame jumped.
    pub fn deliver_clicks(&mut self, latch: u8) -> Result<Vec<i32>> {
        self.deliver_input(latch, false)
    }
    /// Native fast-forward is a left-target condition, not a synthetic mouse
    /// click: mk_read/WaitRelease must not observe a newly latched button.
    pub fn deliver_fast_skip(&mut self) -> Result<Vec<i32>> {
        self.deliver_input(0, true)
    }
    pub fn deliver_input(&mut self, latch: u8, fast: bool) -> Result<Vec<i32>> {
        if latch == 0 && !fast {
            return Ok(vec![]);
        }
        let mut jumped = vec![];
        for id in self.scheduler.order.clone() {
            if !self.thread_alive(id) {
                continue;
            }
            //40B350 applies its live-thread snapshot without41E040. A
            // retired current cursor is not an event caller to restore.
            if self.edit_thread_state(id, "input delivery", |state, next_frame| {
                state.click_input(latch, fast, next_frame)
            })? {
                jumped.push(id);
                self.wake_thread(id);
            }
        }
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

impl super::threads::ThreadState {
    fn branch_frame(&mut self, index: usize, target: u32, next_frame: &mut u64) -> Result<()> {
        let top = self.frames.len() - 1;
        let consumed = if index == top {
            self.pending.as_ref().map(|pending| pending.2)
        } else {
            self.frames[index + 1]
                .interrupted
                .as_ref()
                .map(|pending| pending.2)
        };
        let end = if index == top {
            self.stack.len()
        } else {
            self.frames[index + 1].stack_base
        };
        let start = end
            .checked_sub(consumed.unwrap_or(0))
            .context("Majiro abandoned native stack receipt")?;
        ensure!(
            start >= self.frames[index].stack_base,
            "Majiro abandoned call crosses frame stack base"
        );
        ensure!(
            self.frames
                .iter()
                .skip(index + 1)
                .all(|frame| frame.stack_base >= end),
            "Majiro callback stack base precedes abandoned call"
        );
        let serial = super::threads::allocate_frame_serial(next_frame)?;
        if index == top {
            self.pending = None;
            self.pc = target;
        } else {
            self.frames[index + 1].interrupted = None;
            self.frames[index + 1].return_pc = target;
        }
        if let Some(consumed) = consumed {
            self.stack.drain(start..end);
            for frame in self.frames.iter_mut().skip(index + 1) {
                frame.stack_base -= consumed;
            }
        }
        self.frames[index].serial = serial;
        Ok(())
    }
    fn click_input(&mut self, latch: u8, fast: bool, next_frame: &mut u64) -> Result<bool> {
        ensure!(
            !self.frames.is_empty(),
            "Majiro live input target has no frame"
        );
        self.clicks |= latch;
        let mut jumped = false;
        let top = self.frames.len() - 1;
        for i in 0..=top {
            for (bit, slot) in [(CLICK_LEFT, 0), (CLICK_RIGHT, 1), (CLICK_MIDDLE, 2)] {
                if latch & bit == 0 && !(fast && slot == 0) {
                    continue;
                }
                let pc = if i == top {
                    self.pc
                } else {
                    self.frames[i + 1].return_pc
                };
                let target = self.frames[i].bsel.click[slot];
                if target > pc {
                    self.frames[i].bsel.click = [0; 3];
                    self.branch_frame(i, target, next_frame)?;
                    if i == top {
                        self.clicks = 0;
                        jumped = true;
                    }
                }
            }
        }
        Ok(jumped)
    }
}
