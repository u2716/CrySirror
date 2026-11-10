// Copyright (c) 2026 CrySirror contributors. MIT.
//! Original41DED0 thread identities, isolated operand/local state and newest-
//! first cooperative execution. Resource workers never borrow these contexts.
use super::*;
use std::collections::{BTreeSet, VecDeque};
// Native41DED0/426F90 uses a dynamic linked list, not32 slots. This is an
// engine resource bound; per-call values/frames retain the64MiB budget below.
const MAX_THREADS: usize = 1024;
pub(super) struct ThreadState {
    pub(super) pc: u32,
    pub(super) line: u16,
    pub(super) stack: Vec<Value>,
    pub(super) frames: Vec<Frame>,
    pub(super) local: BTreeMap<u32, Value>,
    pub(super) pending: Option<(Event, u32, usize)>,
    pub(super) halted: bool,
    pub(super) module: usize,
    pub(super) clicks: u8,
}
pub(super) struct Scheduler {
    pub current: i32,
    next: u32,
    pub(super) parked: BTreeMap<i32, ThreadState>,
    pub(super) order: Vec<i32>,
    pub(super) ready: VecDeque<i32>,
    pub(super) blocked: BTreeMap<i32, u64>,
    tick: Option<u64>,
    next_round: Vec<i32>,
    pub(super) terminating: BTreeSet<i32>,
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
            next_round: Vec::new(),
            terminating: BTreeSet::new(),
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
        let mut bytes = self
            .read_marks_bytes
            .saturating_add(values(&self.stack) + frames(&self.frames));
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
        allocate_frame_serial(&mut self.next_frame)
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
    /// wait(-2): park until the next scheduler round of this same update.
    pub(crate) fn defer_round(&mut self) {
        let id = self.scheduler.current;
        if !self.scheduler.next_round.contains(&id) {
            self.scheduler.next_round.push(id);
        }
        self.scheduler.blocked.insert(id, u64::MAX);
    }
    /// 49ADE4: start another thread round without rendering.
    pub(crate) fn start_round(&mut self) -> bool {
        if self.scheduler.next_round.is_empty() {
            return false;
        }
        for id in std::mem::take(&mut self.scheduler.next_round) {
            self.scheduler.blocked.remove(&id);
            if !self.scheduler.ready.contains(&id) {
                self.scheduler.ready.push_back(id);
            }
        }
        true
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
            clicks: std::mem::take(&mut self.clicks),
        }
    }
    /// Owner input/event plans edit a live thread's data without selecting it
    /// as the running VM.YUKIIRO411630 (classic40B350) writes its
    ///427240/41E180 input snapshot directly;
    /// the previously executed current thread may already have been retired.
    /// Always put the context back, including on a failed receipt/branch check.
    pub(super) fn edit_thread_state<R>(
        &mut self,
        id: i32,
        phase: &'static str,
        edit: impl FnOnce(&mut ThreadState, &mut u64) -> Result<R>,
    ) -> Result<R> {
        let current = self.scheduler.current;
        ensure!(
            self.thread_alive(id),
            "Majiro {phase}: foreign/dead thread target={id} current={current}"
        );
        if id == current {
            let mut state = self.take_thread();
            let result = edit(&mut state, &mut self.next_frame);
            self.restore_thread_state(state);
            result.with_context(|| format!("Majiro {phase}: target={id} current={current}"))
        } else {
            let mut state = self.scheduler.parked.remove(&id).with_context(|| {
                format!("Majiro {phase}: foreign/dead thread target={id} current={current}")
            })?;
            let result = edit(&mut state, &mut self.next_frame);
            self.scheduler.parked.insert(id, state);
            result.with_context(|| format!("Majiro {phase}: target={id} current={current}"))
        }
    }
    fn restore_thread_state(&mut self, thread: ThreadState) {
        self.pc = thread.pc;
        self.line = thread.line;
        self.stack = thread.stack;
        self.frames = thread.frames;
        self.banks[2] = thread.local;
        self.pending = thread.pending;
        self.halted = thread.halted;
        self.active_module = thread.module;
        self.clicks = thread.clicks;
        self.program = self.modules[thread.module].program.clone();
    }
    pub(super) fn switch_thread(&mut self, id: i32) -> Result<()> {
        ensure!(
            self.thread_alive(id),
            "Majiro scheduler selection: foreign/dead thread target={id} current={}",
            self.scheduler.current
        );
        if id == self.scheduler.current {
            return Ok(());
        }
        let thread = self.scheduler.parked.remove(&id).with_context(|| {
            format!(
                "Majiro scheduler selection: foreign/dead thread target={id} current={}",
                self.scheduler.current
            )
        })?;
        let old = self.take_thread();
        if !old.halted {
            self.scheduler.parked.insert(self.scheduler.current, old);
        }
        self.scheduler.current = id;
        self.restore_thread_state(thread);
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
        while self.scheduler.order.contains(&(id as i32))
            || matches!(id, 0 | 0xffff_fffe | 0xffff_ffff)
        {
            id = if id >= 0xffff_fffd {
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
            bsel: Bsel::default(),
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
                clicks: 0,
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
    ///41DD40 resets frame wait words and termination state for RunScript;
    /// thread-global variables live until41DE20 actually destroys the thread.
    pub(super) fn clear_thread_waits(&mut self) {
        self.scheduler.blocked.clear();
        self.scheduler.next_round.clear();
        self.scheduler.terminating.clear();
    }
    pub(super) fn reset_thread_waits(&mut self, id: i32) {
        self.scheduler.blocked.remove(&id);
        self.scheduler.next_round.retain(|entry| *entry != id);
        self.scheduler.terminating.remove(&id);
    }
    pub(crate) fn retire_current(&mut self) -> bool {
        let id = self.scheduler.current;
        self.scheduler.ready.retain(|entry| *entry != id);
        self.reset_thread_waits(id);
        self.scheduler.order.retain(|entry| *entry != id);
        self.scheduler.blocked.remove(&id);
        if !self.initializing() {
            self.hooks
                .retain(|hook| hook.owner != id || hook.persistent);
        }
        //41DE20 destroys its remaining operand/thread-global values now;
        // retain only a terminal cursor until the owner selects another task.
        self.stack.clear();
        self.frames.clear();
        self.banks[2].clear();
        self.pending = None;
        self.clicks = 0;
        if id == 0 {
            //40B350 closes remaining tasks when primary thread0 completes.
            //The primary thread has finished its own registered destructors.
            self.scheduler.parked.clear();
            self.scheduler.order.clear();
            self.scheduler.ready.clear();
            self.scheduler.blocked.clear();
            self.scheduler.next_round.clear();
            self.scheduler.terminating.clear();
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
        self.scheduler.next_round.clear();
        self.scheduler.terminating.clear();
        self.scheduler.current = 0;
        self.banks[2].clear();
        self.initializer_finished = true;
    }
}

/// Global frame epochs are allocated even when an owner plan is mutating a
/// parked thread. They never derive from the selected VM's operand state.
pub(super) fn allocate_frame_serial(next: &mut u64) -> Result<u64> {
    let serial = *next;
    *next = serial
        .checked_add(1)
        .context("Majiro frame identity exhausted")?;
    Ok(serial)
}
