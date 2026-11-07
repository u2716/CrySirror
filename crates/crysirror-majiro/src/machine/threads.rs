// Copyright (c) 2026 CrySirror contributors. MIT.
//! Original41DED0 thread identities, isolated operand/local state and newest-
//! first cooperative execution. Resource workers never borrow these contexts.
use super::*;
use std::collections::{BTreeSet, VecDeque};
const MAX_THREADS: usize = 32;
pub(super) struct ThreadState {
    pc: u32,
    line: u16,
    stack: Vec<Value>,
    frames: Vec<Frame>,
    local: BTreeMap<u32, Value>,
    pending: Option<(Event, u32, usize)>,
    halted: bool,
    module: usize,
}
pub(super) struct Scheduler {
    pub current: i32,
    next: u32,
    parked: BTreeMap<i32, ThreadState>,
    order: Vec<i32>,
    ready: VecDeque<i32>,
    blocked: BTreeMap<i32, u64>,
    tick: Option<u64>,
}
impl Default for Scheduler {
    fn default() -> Self {
        Self {
            current: 0,
            next: 1,
            parked: BTreeMap::new(),
            order: vec![0],
            ready: VecDeque::new(),
            blocked: BTreeMap::new(),
            tick: None,
        }
    }
}
impl Machine {
    pub(super) fn reserve_argument_copies(&self, args: &[Value], copies: usize) -> Result<()> {
        fn values(values: &[Value]) -> usize {
            values
                .iter()
                .map(|value| {
                    std::mem::size_of::<Value>()
                        + if let Value::String(bytes) = value {
                            bytes.len()
                        } else {
                            0
                        }
                })
                .sum()
        }
        fn frames(frames: &[Frame]) -> usize {
            frames
                .iter()
                .map(|frame| values(&frame.arguments) + values(&frame.locals))
                .sum()
        }
        let mut bytes = values(&self.stack) + frames(&self.frames);
        for bank in &self.banks {
            for value in bank.values() {
                bytes = bytes.saturating_add(values(std::slice::from_ref(value)));
            }
        }
        for thread in self.scheduler.parked.values() {
            bytes = bytes.saturating_add(values(&thread.stack) + frames(&thread.frames));
            for value in thread.local.values() {
                bytes = bytes.saturating_add(values(std::slice::from_ref(value)));
            }
        }
        let extra = values(args)
            .checked_mul(copies)
            .context("Majiro callback argument extent overflow")?;
        ensure!(
            bytes
                .checked_add(extra)
                .is_some_and(|n| n <= 64 * 1024 * 1024),
            "Majiro script-thread/callback value memory budget"
        );
        Ok(())
    }
    pub(super) fn frame_serial(&mut self) -> Result<u64> {
        let serial = self.next_frame;
        self.next_frame = serial
            .checked_add(1)
            .context("Majiro frame identity exhausted")?;
        Ok(serial)
    }
    pub(crate) fn seed_thread_ids(&mut self, seed: u32) {
        self.scheduler.next = seed.max(1);
    }
    pub(crate) fn context_id(&self) -> u64 {
        self.frames.last().map_or(0, |frame| frame.serial)
    }
    pub(crate) fn live_contexts(&self) -> BTreeSet<u64> {
        self.frames
            .iter()
            .chain(
                self.scheduler
                    .parked
                    .values()
                    .flat_map(|thread| thread.frames.iter()),
            )
            .map(|frame| frame.serial)
            .collect()
    }
    pub(crate) fn thread_count(&self) -> usize {
        self.scheduler.order.len()
    }
    pub(crate) fn hook_count(&self) -> usize {
        self.hooks.len()
    }
    pub(super) fn initializing(&self) -> bool {
        self.frames
            .iter()
            .chain(
                self.scheduler
                    .parked
                    .values()
                    .flat_map(|thread| thread.frames.iter()),
            )
            .any(|frame| frame.global_initializer)
    }
    pub(crate) fn thread_finished(&self) -> bool {
        self.halted
    }
    pub(crate) fn begin_tick(&mut self, now: u64) {
        if self.scheduler.tick != Some(now) {
            for id in &self.scheduler.order {
                if !self.scheduler.ready.contains(id) {
                    self.scheduler.ready.push_back(*id);
                }
            }
            self.scheduler.tick = Some(now);
        }
    }
    pub(crate) fn preserve_budget_continuation(&mut self) {
        let id = self.scheduler.current;
        if !self.halted
            && !self.scheduler.blocked.contains_key(&id)
            && !self.scheduler.ready.contains(&id)
        {
            self.scheduler.ready.push_front(id);
        }
    }
    pub(crate) fn select_ready(&mut self, now: u64) -> Result<bool> {
        while let Some(id) = self.scheduler.ready.pop_front() {
            if self.thread_alive(id)
                && !self
                    .scheduler
                    .blocked
                    .get(&id)
                    .is_some_and(|deadline| *deadline > now)
            {
                self.switch_thread(id)?;
                return Ok(true);
            }
        }
        Ok(false)
    }
    pub(crate) fn defer_current(&mut self, until: u64) {
        self.scheduler.blocked.insert(self.scheduler.current, until);
    }
    pub(crate) fn rotate_current(&mut self, now: u64) -> Result<bool> {
        if self.scheduler.ready.is_empty() {
            return Ok(false);
        }
        let current = self.scheduler.current;
        if !self.scheduler.ready.contains(&current) {
            self.scheduler.ready.push_back(current);
        }
        self.select_ready(now)
    }
    pub(super) fn thread_alive(&self, id: i32) -> bool {
        if id == self.scheduler.current {
            !self.halted
        } else {
            self.scheduler
                .parked
                .get(&id)
                .is_some_and(|thread| !thread.halted)
        }
    }
    fn take_thread(&mut self) -> ThreadState {
        ThreadState {
            pc: self.pc,
            line: self.line,
            stack: std::mem::take(&mut self.stack),
            frames: std::mem::take(&mut self.frames),
            local: std::mem::take(&mut self.banks[2]),
            pending: self.pending.take(),
            halted: self.halted,
            module: self.active_module,
        }
    }
    pub(super) fn switch_thread(&mut self, id: i32) -> Result<()> {
        if id == self.scheduler.current {
            return Ok(());
        }
        let thread = self
            .scheduler
            .parked
            .remove(&id)
            .context("Majiro foreign/dead thread")?;
        let old = self.take_thread();
        if !old.halted {
            self.scheduler.parked.insert(self.scheduler.current, old);
        }
        self.scheduler.current = id;
        self.pc = thread.pc;
        self.line = thread.line;
        self.stack = thread.stack;
        self.frames = thread.frames;
        self.banks[2] = thread.local;
        self.pending = thread.pending;
        self.halted = thread.halted;
        self.active_module = thread.module;
        self.program = self.modules[thread.module].program.clone();
        Ok(())
    }
    pub(super) fn wake_thread(&mut self, id: i32) {
        self.scheduler.blocked.remove(&id);
        if id != self.scheduler.current && !self.scheduler.ready.contains(&id) {
            self.scheduler.ready.push_back(id);
        }
    }
    pub(super) fn thread_depth(&self, id: i32) -> Option<usize> {
        if id == self.scheduler.current {
            Some(self.frames.len())
        } else {
            self.scheduler
                .parked
                .get(&id)
                .map(|thread| thread.frames.len())
        }
    }
    pub(super) fn spawn_thread(
        &mut self,
        target: (usize, u32),
        arguments: Vec<Value>,
    ) -> Result<i32> {
        ensure!(
            self.scheduler.order.len() < MAX_THREADS,
            "Majiro native thread budget"
        );
        ensure!(
            arguments.len() <= MAX_LOCALS,
            "Majiro thread argument budget"
        );
        self.modules[target.0].program.instruction(target.1)?;
        let mut id = self.scheduler.next;
        while self.scheduler.order.contains(&(id as i32)) || matches!(id, 0 | 0xffff_fffe) {
            id = if id == 0xffff_fffd {
                1
            } else {
                id.wrapping_add(1)
            };
        }
        let serial = self.frame_serial()?;
        let frame = Frame {
            serial,
            interrupted: None,
            global_initializer: false,
            return_pc: 0,
            return_module: target.0,
            stack_base: 0,
            arguments,
            locals: vec![],
            discard_return: true,
        };
        self.scheduler.parked.insert(
            id as i32,
            ThreadState {
                pc: target.1,
                line: 0,
                stack: vec![],
                frames: vec![frame],
                local: BTreeMap::new(),
                pending: None,
                halted: false,
                module: target.0,
            },
        );
        self.scheduler.order.insert(0, id as i32);
        self.scheduler.next = if id == 0xffff_fffd {
            1
        } else {
            id.wrapping_add(1)
        };
        //41E180 snapshots the thread list. A newly created thread joins the
        //next owner update, rather than stealing its creator's current PC.
        Ok(id as i32)
    }
    pub(crate) fn retire_current(&mut self) -> bool {
        let id = self.scheduler.current;
        self.scheduler.order.retain(|entry| *entry != id);
        self.scheduler.blocked.remove(&id);
        if !self.initializing() {
            self.hooks
                .retain(|hook| hook.owner != id || hook.persistent);
        }
        if id == 0 {
            //40B350 closes remaining tasks when primary thread0 completes.
            //Native finalizer opcodes remain explicitly unsupported, so no
            //accepted task can have an unexecuted native destructor here.
            self.scheduler.parked.clear();
            self.scheduler.order.clear();
            self.scheduler.ready.clear();
            return true;
        }
        self.scheduler.order.is_empty()
    }
    pub(super) fn finish_global_initializer(&mut self) {
        //424CB0 marks startup hooks persistent, then41E580 recreates primary
        //thread0. Persistent hooks keep their resolved module addresses.
        for hook in &mut self.hooks {
            hook.persistent = true;
        }
        self.scheduler.parked.clear();
        self.scheduler.order = vec![0];
        self.scheduler.ready.clear();
        self.scheduler.blocked.clear();
        self.scheduler.current = 0;
        self.banks[2].clear();
    }
}
