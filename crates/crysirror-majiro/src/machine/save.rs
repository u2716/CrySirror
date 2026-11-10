// Copyright (c) 2026 CrySirror contributors. MIT.
//! Save-point capture/restore (41F7A0 / 41F260 / 41E580). The original keeps
//! thread 0's frames, operand stack and thread-local bank plus the save-scope
//! global bank; persistent globals and startup hooks survive a load. This is a
//! portable CrySirror record, not the original MajiroSavV1.103 binary.
use super::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SavedValue {
    I(i32),
    F(f32),
    S(Vec<u8>),
    A { element: u8, dims: [i32; 3], elements: Vec<SavedValue> },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SavedFrame {
    module: String,
    return_pc: u32,
    return_module: String,
    stack_base: usize,
    arguments: Vec<SavedValue>,
    locals: Vec<SavedValue>,
    discard_return: bool,
    click: [u32; 3],
    aip4: u32,
    destructor: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SaveState {
    module: String,
    pc: u32,
    line: u16,
    stack: Vec<SavedValue>,
    frames: Vec<SavedFrame>,
    thread_locals: Vec<(u32, SavedValue)>,
    globals: Vec<(u32, SavedValue)>,
}
impl SaveState {
    /// Module names referenced by the record (for program preparation).
    pub fn modules(&self) -> Vec<String> {
        let mut out = vec![self.module.clone()];
        for f in &self.frames {
            for m in [&f.module, &f.return_module] {
                if !out.contains(m) {
                    out.push(m.clone());
                }
            }
        }
        out
    }
}
fn save_value(v: &Value) -> SavedValue {
    match v {
        Value::Integer(n) => SavedValue::I(*n),
        Value::Float(f) => SavedValue::F(*f),
        Value::String(b) => SavedValue::S(b.clone()),
        Value::Array(a) => {
            let data = a.lock();
            SavedValue::A {
                element: data.element,
                dims: data.dims,
                elements: data.elements.iter().map(save_value).collect(),
            }
        }
    }
}
fn load_value(v: &SavedValue) -> Result<Value> {
    Ok(match v {
        SavedValue::I(n) => Value::Integer(*n),
        SavedValue::F(f) => Value::Float(*f),
        SavedValue::S(b) => Value::String(b.clone()),
        SavedValue::A { element, dims, elements } => {
            let array = ArrayRef::new(*element, *dims)?;
            {
                let mut data = array.lock();
                ensure!(data.elements.len() == elements.len(), "Majiro saved array extent");
                for (slot, value) in data.elements.iter_mut().zip(elements) {
                    *slot = load_value(value)?;
                }
            }
            Value::Array(array)
        }
    })
}
impl Machine {
    /// Capture the current thread (thread 0 at a save point) and the
    /// save-scope bank. Call after the save_point call is acknowledged.
    pub fn capture_save(&self) -> Result<SaveState> {
        ensure!(self.scheduler.current == 0, "Majiro save point outside the main thread");
        let name = |i: usize| self.modules[i].name.clone();
        let bodies: Vec<usize> = (0..self.frames.len())
            .map(|i| if i + 1 == self.frames.len() { self.active_module } else { self.frames[i + 1].return_module })
            .collect();
        Ok(SaveState {
            module: name(self.active_module),
            pc: self.pc,
            line: self.line,
            stack: self.stack.iter().map(save_value).collect(),
            frames: self
                .frames
                .iter()
                .enumerate()
                .map(|(i, f)| SavedFrame {
                    module: name(bodies[i]),
                    return_pc: f.return_pc,
                    return_module: name(f.return_module),
                    stack_base: f.stack_base,
                    arguments: f.arguments.iter().map(save_value).collect(),
                    locals: f.locals.iter().map(save_value).collect(),
                    discard_return: f.discard_return,
                    click: f.bsel.click,
                    aip4: f.bsel.aip4,
                    destructor: f.bsel.destructor,
                })
                .collect(),
            thread_locals: self.banks[2].iter().map(|(k, v)| (*k, save_value(v))).collect(),
            globals: self.banks[1].iter().map(|(k, v)| (*k, save_value(v))).collect(),
        })
    }
    fn module_index(&mut self, name: &str, programs: &BTreeMap<String, Arc<Program>>) -> Result<usize> {
        if let Some(i) = self.modules.iter().position(|m| m.name == name) {
            return Ok(i);
        }
        let program = programs.get(name).with_context(|| format!("Majiro saved module {name} is unavailable"))?;
        ensure!(self.modules.len() < modules::MAX_MODULES, "Majiro linked module count budget");
        self.modules.push(modules::LinkedModule { name: name.to_owned(), program: program.clone() });
        Ok(self.modules.len() - 1)
    }
    /// 41E580 + 41F260: drop every thread and non-persistent hook, then
    /// rebuild thread 0 from the record. Persistent globals are untouched.
    pub fn restore_save(&mut self, state: &SaveState, programs: &BTreeMap<String, Arc<Program>>) -> Result<()> {
        let mut frames = vec![];
        let mut bodies = vec![];
        for f in &state.frames {
            let serial = self.frame_serial()?;
            let return_module = self.module_index(&f.return_module, programs)?;
            bodies.push(self.module_index(&f.module, programs)?);
            frames.push(Frame {
                serial,
                interrupted: None,
                global_initializer: false,
                return_pc: f.return_pc,
                return_module,
                stack_base: f.stack_base,
                arguments: f.arguments.iter().map(load_value).collect::<Result<_>>()?,
                locals: f.locals.iter().map(load_value).collect::<Result<_>>()?,
                discard_return: f.discard_return,
                bsel: Bsel { click: f.click, aip4: f.aip4, destructor: f.destructor },
            });
        }
        ensure!(!frames.is_empty(), "Majiro save without frames");
        let active = self.module_index(&state.module, programs)?;
        self.modules[active].program.instruction(state.pc)?;
        let stack = state.stack.iter().map(load_value).collect::<Result<Vec<_>>>()?;
        let locals = state
            .thread_locals
            .iter()
            .map(|(k, v)| Ok((*k, load_value(v)?)))
            .collect::<Result<BTreeMap<_, _>>>()?;
        let globals = state
            .globals
            .iter()
            .map(|(k, v)| Ok((*k, load_value(v)?)))
            .collect::<Result<BTreeMap<_, _>>>()?;
        // Thread teardown: no destructors run on a load reset.
        self.clear_thread_waits();
        self.scheduler.parked.clear();
        self.scheduler.order = vec![0];
        self.scheduler.ready.clear();
        self.scheduler.blocked.clear();
        self.scheduler.current = 0;
        self.hooks.retain(|hook| hook.persistent);
        self.pending = None;
        self.halted = false;
        self.clicks = 0;
        self.frames = frames;
        self.stack = stack;
        self.banks[2] = locals;
        self.banks[1] = globals;
        self.active_module = active;
        self.program = self.modules[active].program.clone();
        self.pc = state.pc;
        self.line = state.line;
        let _ = bodies;
        Ok(())
    }
    /// Dispatch an event to its hooks before the current thread continues.
    pub fn dispatch_event_now(&mut self, event: u32, native_args: Vec<Value>) -> Result<()> {
        let plan = self.event_plan(event, None, &native_args)?;
        if plan.is_empty() {
            return Ok(());
        }
        self.reserve_argument_copies(&native_args, plan.len() + 1)?;
        self.deliver_plan(plan, native_args.into_iter().rev().collect())
    }
    /// True once, right after `$init@GLOBAL` completed (424CB0 HOT_RESET(0)).
    pub fn take_initializer_finished(&mut self) -> bool {
        std::mem::take(&mut self.initializer_finished)
    }
}
impl Machine {
    /// Persistent-scope bank (system data, 41E580 MajiroSysV1.100).
    pub fn export_persistent(&self) -> Vec<(u32, SavedValue)> {
        self.banks[0].iter().map(|(k, v)| (*k, save_value(v))).collect()
    }
    pub fn import_persistent(&mut self, values: &[(u32, SavedValue)]) -> Result<()> {
        for (k, v) in values {
            self.banks[0].insert(*k, load_value(v)?);
        }
        Ok(())
    }
}
