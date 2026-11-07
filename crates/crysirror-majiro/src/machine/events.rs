// Copyright (c) 2026 CrySirror contributors. MIT.
//! Original4098C0 appends and409950 prepends hooks. 4094E0 traverses head to
//! tail and stacks callbacks (last pushed runs first);409690 traverses back
//! to front while targeting an explicit thread. Hooks store resolved logical
//! module/PC pairs, never executable host pointers.
use super::*;
#[derive(Clone)]
pub(super) struct Hook {
    pub owner: i32,
    pub persistent: bool,
    event: u32,
    target: (usize, u32),
}
fn event_hash(value: &Value) -> Result<u32> {
    let bytes = value.bytes()?;
    let bytes = &bytes[..bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len())];
    ensure!(bytes.len() <= 4096, "Majiro event name byte budget");
    Ok(crate::program::name_hash(bytes))
}
impl Machine {
    /// Native services dispatch through the same hook traversal as do_event.
    /// Validate the whole callback plan before consuming the retained call;
    /// callbacks are stacked only after its procedure/function result exists.
    pub(crate) fn complete_with_callback(
        &mut self,
        output: Vec<Value>,
        event: Option<(u32, Vec<Value>)>,
    ) -> Result<()> {
        if let Some((event, arguments)) = event {
            let plan = self.event_plan(event, None, &arguments)?;
            self.reserve_argument_copies(&arguments, plan.len() + 1)?;
            self.acknowledge(output)?;
            self.deliver_plan(plan, arguments.into_iter().rev().collect())
        } else {
            self.acknowledge(output)
        }
    }

    /// Native semantic calls completed without host access. False leaves the
    /// original event and every operand untouched for the shared-service seam.
    pub(crate) fn native_thread_control(&mut self, event: &Event) -> Result<bool> {
        let Event::Native {
            hash,
            arguments: a,
            returns,
            ..
        } = event
        else {
            return Ok(false);
        };
        let output = |n| {
            if *returns {
                vec![Value::Integer(n)]
            } else {
                vec![]
            }
        };
        match *hash {
            0x078a756e | 0x1295bbda => {
                ensure!(a.len() == 2, "Majiro event_hook ABI");
                ensure!(self.hooks.len() < 4096, "Majiro event hook budget");
                let hook = Hook {
                    owner: self.scheduler.current,
                    persistent: false,
                    event: event_hash(&a[0])?,
                    target: self.resolve_function(a[1].integer()? as u32)?,
                };
                self.acknowledge(output(0))?;
                if *hash == 0x078a756e {
                    self.hooks.push(hook);
                } else {
                    self.hooks.insert(0, hook);
                }
            }
            0xa93c9856 => {
                ensure!(a.len() == 2, "Majiro event hook removal ABI");
                let event = event_hash(&a[0])?;
                let target = self.resolve_function(a[1].integer()? as u32)?;
                self.acknowledge(output(0))?;
                //4099E0 cannot remove persistent hooks or startup hooks while
                //the initialization stage is protected by409370(1).
                let initializing = self.initializing();
                if !initializing {
                    self.hooks.retain(|hook| {
                        hook.owner != self.scheduler.current
                            || hook.persistent
                            || hook.event != event
                            || hook.target != target
                    });
                }
            }
            0x76ee6c90 | 0x29f1ac40 => {
                let targeted = *hash == 0x29f1ac40;
                ensure!(
                    a.len() >= if targeted { 2 } else { 1 },
                    "Majiro event dispatch ABI"
                );
                let (event, thread, skip) = if targeted {
                    let id = a[0].integer()?;
                    (
                        event_hash(&a[1])?,
                        Some(if id == -1 { self.scheduler.current } else { id }),
                        2,
                    )
                } else {
                    (event_hash(&a[0])?, None, 1)
                };
                let plan = self.event_plan(event, thread, &a[skip..])?;
                self.reserve_argument_copies(&a[skip..], plan.len() + 1)?;
                let arguments = a[skip..].iter().rev().cloned().collect::<Vec<_>>();
                //Native zero result is placed below the interrupt frames.
                self.acknowledge(output(0))?;
                self.deliver_plan(plan, arguments)?;
            }
            0xf2f9daa8 | 0xae497f1a => {
                ensure!(!a.is_empty(), "Majiro thread_begin ABI");
                let target = self.resolve_function(a[0].integer()? as u32)?;
                self.reserve_argument_copies(&a[1..], 1)?;
                let arguments = a[1..].iter().rev().cloned().collect();
                let id = self.spawn_thread(target, arguments)?;
                self.acknowledge(output(id))?;
            }
            0x9fb53fbe => {
                ensure!(a.len() == 1, "Majiro thread existence ABI");
                self.acknowledge(output(i32::from(self.thread_alive(a[0].integer()?))))?;
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
    pub(super) fn event_plan(
        &self,
        event: u32,
        thread: Option<i32>,
        arguments: &[Value],
    ) -> Result<Vec<(i32, (usize, u32))>> {
        ensure!(
            arguments.len() <= MAX_LOCALS,
            "Majiro callback argument budget"
        );
        let mut plan = vec![];
        let mut depths: BTreeMap<i32, usize> = BTreeMap::new();
        let hooks: Box<dyn Iterator<Item = &Hook> + '_> = if thread.is_some() {
            Box::new(self.hooks.iter().rev())
        } else {
            Box::new(self.hooks.iter())
        };
        for hook in hooks.filter(|hook| hook.event == event) {
            let id = thread.unwrap_or(hook.owner);
            if !self.thread_alive(id) {
                continue;
            }
            let count = depths.entry(id).or_insert(self.thread_depth(id).unwrap());
            *count += 1;
            ensure!(*count <= MAX_FRAMES, "Majiro event callback depth budget");
            self.modules[hook.target.0]
                .program
                .instruction(hook.target.1)?;
            plan.push((id, hook.target));
        }
        ensure!(
            self.next_frame.checked_add(plan.len() as u64).is_some(),
            "Majiro callback identity budget"
        );
        Ok(plan)
    }
    pub(super) fn deliver_plan(
        &mut self,
        plan: Vec<(i32, (usize, u32))>,
        arguments: Vec<Value>,
    ) -> Result<()> {
        let caller = self.scheduler.current;
        for (id, target) in plan {
            self.switch_thread(id)?;
            let serial = self.frame_serial()?;
            let frame = Frame {
                serial,
                interrupted: self.pending.take(),
                global_initializer: false,
                return_pc: self.pc,
                return_module: self.active_module,
                stack_base: self.stack.len(),
                arguments: arguments.clone(),
                locals: vec![],
                discard_return: true,
                bsel: Bsel::default(),
            };
            self.frames.push(frame);
            self.active_module = target.0;
            self.program = self.modules[target.0].program.clone();
            self.pc = target.1;
            self.wake_thread(id);
        }
        self.switch_thread(caller)?;
        //Dead nonpersistent bindings are pruned on dispatch in4094E0, even
        //when their callbacks never ran. Persistent startup bindings survive.
        let dead = self
            .hooks
            .iter()
            .filter(|hook| !hook.persistent && !self.thread_alive(hook.owner))
            .map(|hook| hook.owner)
            .collect::<std::collections::BTreeSet<_>>();
        if !self.initializing() {
            self.hooks
                .retain(|hook| hook.persistent || !dead.contains(&hook.owner));
        }
        Ok(())
    }
}
impl Machine {
    /// Complete the pending native call, then call a script function in the
    /// current thread (40AE30). `native_args` are in parameter order.
    pub(crate) fn complete_and_call(
        &mut self,
        output: Vec<Value>,
        function: u32,
        native_args: Vec<Value>,
    ) -> Result<()> {
        ensure!(self.frames.len() < MAX_FRAMES, "Majiro callback depth budget");
        let target = self.resolve_function(function)?;
        self.acknowledge(output)?;
        let serial = self.frame_serial()?;
        self.frames.push(Frame {
            serial,
            interrupted: None,
            global_initializer: false,
            return_pc: self.pc,
            return_module: self.active_module,
            stack_base: self.stack.len(),
            arguments: native_args.into_iter().rev().collect(),
            locals: vec![],
            discard_return: true,
            bsel: Bsel::default(),
        });
        self.active_module = target.0;
        self.program = self.modules[target.0].program.clone();
        self.pc = target.1;
        Ok(())
    }
    /// Dispatch an event while keeping the pending native call parked; it is
    /// re-delivered after the hooks return (native PC rewind). Returns false
    /// when no hook is registered for the event.
    pub(crate) fn interrupt_with_event(&mut self, event: u32, native_args: Vec<Value>) -> Result<bool> {
        let plan = self.event_plan(event, None, &native_args)?;
        if plan.is_empty() {
            return Ok(false);
        }
        self.reserve_argument_copies(&native_args, plan.len() + 1)?;
        self.deliver_plan(plan, native_args.into_iter().rev().collect())?;
        Ok(true)
    }
    /// 409D90/41DD40: reset the current thread and enter a module's entry.
    pub(crate) fn run_script(&mut self, name: String, program: Arc<Program>, native_args: Vec<Value>) -> Result<()> {
        program.instruction(program.entry)?;
        let index = match self.modules.iter().position(|m| m.name == name) {
            Some(index) => index,
            None => {
                ensure!(self.modules.len() < modules::MAX_MODULES, "Majiro linked module count budget");
                self.modules.push(modules::LinkedModule { name, program: program.clone() });
                self.modules.len() - 1
            }
        };
        let id = self.scheduler.current;
        if !self.initializing() {
            self.hooks.retain(|hook| hook.owner != id || hook.persistent);
        }
        self.pending = None;
        self.stack.clear();
        let serial = self.frame_serial()?;
        self.frames = vec![Frame {
            serial,
            interrupted: None,
            global_initializer: false,
            return_pc: 0,
            return_module: index,
            stack_base: 0,
            arguments: native_args.into_iter().rev().collect(),
            locals: vec![],
            discard_return: true,
            bsel: Bsel::default(),
        }];
        self.active_module = index;
        self.program = self.modules[index].program.clone();
        self.pc = self.program.entry;
        self.executed += 1;
        Ok(())
    }
    pub fn current_thread(&self) -> i32 {
        self.scheduler.current
    }
    pub fn depth(&self) -> usize {
        self.frames.len()
    }
    /// Request termination of a thread (4EF903F3). 40B350 then resumes every
    /// frame that registered a destructor (0x847) at that target; frames
    /// without one are discarded. A thread without destructors ends at once.
    pub(crate) fn kill_thread(&mut self, id: i32) -> Result<()> {
        if !self.thread_alive(id) {
            return Ok(());
        }
        let caller = self.scheduler.current;
        self.switch_thread(id)?;
        let mut kept: Vec<Frame> = vec![];
        let frames = std::mem::take(&mut self.frames);
        let bodies: Vec<usize> = (0..frames.len())
            .map(|i| if i + 1 == frames.len() { self.active_module } else { frames[i + 1].return_module })
            .collect();
        let mut resume: Vec<(u32, usize)> = vec![];
        for (i, frame) in frames.into_iter().enumerate() {
            if frame.bsel.destructor != 0 {
                resume.push((frame.bsel.destructor, bodies[i]));
                let mut frame = frame;
                frame.bsel = Bsel::default();
                frame.interrupted = None;
                kept.push(frame);
            }
        }
        self.pending = None;
        if kept.is_empty() {
            self.halted = true;
            self.frames = vec![];
        } else {
            // Re-link: each kept frame returns into the next lower kept frame
            // at that frame's destructor; the top frame starts at its own.
            for i in 1..kept.len() {
                kept[i].return_pc = resume[i - 1].0;
                kept[i].return_module = resume[i - 1].1;
            }
            let top = kept.len() - 1;
            let base = kept[top].stack_base.min(self.stack.len());
            self.stack.truncate(base);
            self.pc = resume[top].0;
            self.active_module = resume[top].1;
            self.program = self.modules[self.active_module].program.clone();
            self.frames = kept;
        }
        if id != caller && self.thread_alive(caller) {
            self.switch_thread(caller)?;
        } else if id != caller {
            self.switch_thread(caller).ok();
        }
        if !self.thread_alive(id) {
            self.scheduler.order.retain(|x| *x != id);
            self.scheduler.blocked.remove(&id);
            self.scheduler.parked.remove(&id);
            self.hooks.retain(|hook| hook.owner != id || hook.persistent);
        } else {
            self.wake_thread(id);
        }
        Ok(())
    }
    pub(crate) fn thread_ids(&self) -> Vec<i32> {
        self.scheduler.order.clone()
    }
    pub(crate) fn global(&self, hash: u32) -> Option<Value> {
        self.banks[0].get(&hash).cloned()
    }
    pub(crate) fn set_global(&mut self, hash: u32, value: Value) {
        self.banks[0].insert(hash, value);
    }
}
impl Machine {
    pub(crate) fn hook_exists(&self, event: u32) -> bool {
        self.hooks.iter().any(|hook| hook.event == event && self.thread_alive(hook.owner))
    }
}
