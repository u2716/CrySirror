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
            0xf2f9daa8 => {
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
    fn event_plan(
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
    fn deliver_plan(
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
