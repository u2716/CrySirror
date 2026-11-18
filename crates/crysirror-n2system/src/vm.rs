// SPDX-License-Identifier: MPL-2.0
// NsScript execution follows NitroSharp VM/NsScriptVM.cs at e3869842 (MIT).
// Native services leave this interpreter as owned requests. They must complete
// once before the requesting thread can execute another instruction.
use crate::{
    catalog::Builtin,
    program::{Instruction, Module},
    source::{Name, RoutineKind},
    value::Value,
};
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Argument {
    pub value: Value,
    pub reference: Option<Name>,
}
impl From<Value> for Argument {
    fn from(value: Value) -> Self {
        Self {
            value,
            reference: None,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Call {
    pub function: Builtin,
    pub arguments: Vec<Argument>,
    pub module: String,
    pub routine: String,
    pub instruction: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Frame {
    pub module: String,
    pub routine: String,
    pub pc: usize,
    pub base: usize,
    pub result: bool,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Thread {
    pub name: String,
    /// Lexical callers inherited by separately scheduled scene/process threads.
    #[serde(default)]
    pub scopes: Vec<String>,
    pub frames: Vec<Frame>,
    pub stack: Vec<Argument>,
    pub pending: Option<Call>,
    pub selected: bool,
    pub halted: bool,
}
impl Thread {
    pub fn inherited_scopes(&self) -> Result<Vec<String>> {
        let mut seen = std::collections::BTreeSet::new();
        let result = self
            .frames
            .iter()
            .rev()
            .map(|f| &f.module)
            .chain(&self.scopes)
            .filter(|name| seen.insert((*name).clone()))
            .cloned()
            .collect::<Vec<_>>();
        ensure!(
            result.len() <= 256,
            "N2System inherited module scope capacity"
        );
        Ok(result)
    }
}
#[derive(Clone, Debug)]
pub enum Step {
    Budget,
    Halted,
    Call(Call),
    Load(String),
    Choice(String),
    SelectEnd,
    Frame,
    Scene(String, String),
    ClearText,
    AppendText(String),
    LineEnd,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Machine {
    pub modules: BTreeMap<String, Module>,
    pub variables: BTreeMap<String, Value>,
    pub flags: BTreeMap<String, Value>,
    pub arrays: crate::array::Arrays,
    /// Reference compiler diagnostics retained separately from native calls.
    #[serde(default)]
    pub unresolved_functions: std::collections::BTreeSet<(String, String, usize, String)>,
}

pub fn module_name(name: &str) -> Result<String> {
    let name = name.replace('\\', "/");
    let name = name.trim_matches('"').trim_start_matches("./");
    let name = name.strip_prefix("nss/").unwrap_or(name);
    let name = if name.to_ascii_lowercase().ends_with(".nss") {
        &name[..name.len() - 4]
    } else {
        name
    };
    ensure!(
        !name.is_empty()
            && name.len() <= 1024
            && !name.starts_with('/')
            && !name.contains(':')
            && name
                .split('/')
                .all(|p| !p.is_empty() && p != "." && p != ".."),
        "N2System unsafe module path"
    );
    Ok(name.to_owned())
}
impl Machine {
    pub fn insert(&mut self, name: &str, module: Module) -> Result<()> {
        let name = module_name(name)?;
        ensure!(
            self.modules.len() < 4096 || self.modules.contains_key(&name),
            "N2System module limit"
        );
        for routine in module.routines.values() {
            for parameter in &routine.parameters {
                self.variables
                    .entry(parameter.clone())
                    .or_insert(Value::Number(0.));
            }
            for op in &routine.instructions {
                match op {
                    Instruction::Load(n, _) if n.explicit => {
                        self.table_mut(n)
                            .entry(n.text.clone())
                            .or_insert(Value::Number(0.));
                    }
                    Instruction::Store(n) => {
                        self.table_mut(n)
                            .entry(n.text.clone())
                            .or_insert(Value::Number(0.));
                    }
                    _ => {}
                }
            }
        }
        ensure!(
            self.variables.len() <= 32766 && self.flags.len() <= 32766,
            "N2System variable/flag capacity"
        );
        self.modules.insert(name, module);
        Ok(())
    }
    fn table_mut(&mut self, name: &Name) -> &mut BTreeMap<String, Value> {
        if name.flag {
            &mut self.flags
        } else {
            &mut self.variables
        }
    }
    pub fn assign(&mut self, name: &Name, value: Value) -> Result<()> {
        let limit = 32766;
        let table = self.table_mut(name);
        ensure!(
            table.len() < limit || table.contains_key(&name.text),
            "N2System global capacity"
        );
        table.insert(name.text.clone(), value);
        Ok(())
    }
    pub fn read(&self, name: &Name) -> Value {
        let table = if name.flag {
            &self.flags
        } else {
            &self.variables
        };
        table.get(&name.text).cloned().unwrap_or(Value::Number(0.))
    }
    pub fn start(&self, module: &str, routine: Option<&str>, name: String) -> Result<Thread> {
        let mut module = module_name(module)?;
        let source = self
            .modules
            .get(&module)
            .context("N2System entry module not prepared")?;
        let routine = if let Some(r) = routine {
            r.to_owned()
        } else {
            source
                .entry
                .clone()
                .context("N2System entry chapter missing")?
        };
        module = self
            .resolve(&module, &routine)?
            .with_context(|| format!("N2System entry routine missing: {routine}"))?;
        Ok(Thread {
            name,
            frames: vec![Frame {
                module,
                routine,
                pc: 0,
                base: 0,
                result: false,
            }],
            ..Default::default()
        })
    }
    pub fn complete(&mut self, thread: &mut Thread, result: Value) -> Result<()> {
        ensure!(thread.pending.is_some(), "N2System no pending native call");
        thread.push(result.into())?;
        thread.pending = None;
        Ok(())
    }
    /// Original415B60 clears context control fields and advances each selected
    /// context to its captured end-PC;47E250/47D3E0 establish those bounds.
    /// This returns whether a selection was unwound and needs host UI cleanup.
    pub fn escape(&mut self, thread: &mut Thread, count: usize) -> Result<bool> {
        ensure!(count > 0, "N2System empty escape");
        let call = thread
            .pending
            .as_ref()
            .context("N2System Escape without pending native call")?
            .clone();
        self.complete(thread, Value::Null)?;
        let mut left = count;
        let mut first = true;
        let mut selection = false;
        loop {
            let Some(frame) = thread.frames.last().cloned() else {
                break;
            };
            let routine = self
                .modules
                .get(&frame.module)
                .and_then(|m| m.routines.get(&frame.routine))
                .context("N2System Escape routine missing")?;
            let pc = if first {
                call.instruction
            } else {
                frame.pc.saturating_sub(1)
            };
            first = false;
            let mut scopes = routine
                .scopes
                .iter()
                .filter(|s| s.begin <= pc && pc < s.end)
                .collect::<Vec<_>>();
            scopes.sort_by(|a, b| b.begin.cmp(&a.begin).then(a.end.cmp(&b.end)));
            for scope in scopes {
                selection |= scope.selection;
                if left == 1 {
                    thread.stack.truncate(frame.base);
                    ensure!(
                        scope.end < routine.instructions.len(),
                        "N2System Escape end-PC outside routine"
                    );
                    thread.frames.last_mut().unwrap().pc = scope.end;
                    if selection {
                        thread.selected = false;
                    }
                    return Ok(selection);
                }
                left -= 1;
            }
            // The routine body is the next native context after its nested
            // blocks. Return to the caller using the ordinary stack contract.
            thread.frames.pop();
            thread.stack.truncate(frame.base);
            if frame.result {
                thread.push(Value::Null.into())?;
            }
            left -= 1;
            if left == 0 {
                break;
            }
        }
        if selection {
            thread.selected = false;
        }
        Ok(selection)
    }
    fn resolve(&self, module: &str, name: &str) -> Result<Option<String>> {
        fn search(
            machine: &Machine,
            module: &str,
            name: &str,
            visited: &mut std::collections::BTreeSet<String>,
        ) -> Result<Option<String>> {
            if !visited.insert(module.into()) {
                return Ok(None);
            }
            let source = machine
                .modules
                .get(module)
                .context("N2System current module missing")?;
            if source.routines.contains_key(name) {
                return Ok(Some(module.into()));
            }
            for import in &source.imports {
                let import = module_name(import)?;
                if machine.modules.contains_key(&import) {
                    if let Some(found) = search(machine, &import, name, visited)? {
                        return Ok(Some(found));
                    }
                }
            }
            Ok(None)
        }
        search(self, module, name, &mut Default::default())
    }
    fn resolve_in_thread(
        &self,
        thread: &Thread,
        module: &str,
        name: &str,
    ) -> Result<Option<String>> {
        for scope in std::iter::once(module)
            .chain(thread.frames.iter().rev().map(|f| f.module.as_str()))
            .chain(thread.scopes.iter().map(String::as_str))
        {
            if let Some(owner) = self.resolve(scope, name)? {
                return Ok(Some(owner));
            }
        }
        Ok(None)
    }
    fn missing_import(&self, module: &str) -> Result<Option<String>> {
        for import in &self.modules[module].imports {
            let name = module_name(import)?;
            if !self.modules.contains_key(&name) {
                return Ok(Some(name));
            }
        }
        Ok(None)
    }
    pub fn step(&mut self, thread: &mut Thread, budget: u32) -> Result<Step> {
        if thread.halted || thread.frames.is_empty() {
            thread.halted = true;
            return Ok(Step::Halted);
        }
        if let Some(call) = &thread.pending {
            return Ok(Step::Call(call.clone()));
        }
        for _ in 0..budget {
            if thread.frames.is_empty() {
                thread.halted = true;
                return Ok(Step::Halted);
            }
            let frame = thread.frames.last().unwrap().clone();
            let routine = self
                .modules
                .get(&frame.module)
                .and_then(|m| m.routines.get(&frame.routine))
                .context("N2System missing active routine")?;
            let op = routine
                .instructions
                .get(frame.pc)
                .cloned()
                .with_context(|| {
                    format!(
                        "N2System instruction outside {}::{}",
                        frame.module, frame.routine
                    )
                })?;
            thread.frames.last_mut().unwrap().pc += 1;
            match op {
                Instruction::Push(mut value, constants) => {
                    if constants {
                        if let Value::String(s) = &value {
                            if let Some(c) = Value::constant(s) {
                                value = c;
                            }
                        }
                    }
                    thread.push(value.into())?;
                }
                Instruction::Load(name, _) => {
                    let known = if name.flag {
                        self.flags.contains_key(&name.text)
                    } else {
                        self.variables.contains_key(&name.text)
                    };
                    if known || name.explicit {
                        let value = if name.text == "SYSTEM_present_process" {
                            Value::String(routine.display_name.clone())
                        } else {
                            self.read(&name)
                        };
                        thread.push(Argument {
                            value,
                            reference: Some(name),
                        })?;
                    } else {
                        thread.push(
                            Value::constant(&name.text)
                                .unwrap_or(Value::String(name.text))
                                .into(),
                        )?;
                    }
                }
                Instruction::Index => {
                    let key = thread.pop()?.value;
                    let value = thread.pop()?;
                    let id = self.ensure_array(&value)?;
                    let child = self.arrays.child(id, &key)?;
                    thread.push(self.arrays.value(child)?.into())?;
                }
                Instruction::StoreIndex(op) => {
                    let target = thread.pop()?.value;
                    let value = thread.pop()?.value;
                    let Value::Array(id, _) = target else {
                        bail!("N2System indexed assignment target missing")
                    };
                    let node = self
                        .arrays
                        .nodes
                        .get_mut(&id)
                        .context("N2System indexed assignment target retired")?;
                    node.scalar = if let Some(op) = op {
                        node.scalar.binary(op, &value)?
                    } else {
                        value.scalar().clone()
                    };
                }
                Instruction::BindArgument(name, index) => {
                    let module = self
                        .resolve_in_thread(thread, &frame.module, &name)?
                        .context("N2System parameter owner missing")?;
                    let parameter = self.modules[&module].routines[&name]
                        .parameters
                        .get(index)
                        .context("N2System parameter index")?
                        .clone();
                    let value = thread.pop()?.value;
                    self.assign(
                        &Name {
                            text: parameter,
                            flag: false,
                            explicit: true,
                        },
                        value,
                    )?;
                }
                Instruction::Store(name) => {
                    let value = thread.pop()?.value;
                    self.assign(&name, value)?;
                }
                Instruction::Pop => {
                    thread.pop()?;
                }
                Instruction::Unary(op) => {
                    let arg = thread.pop()?.value;
                    let a = arg.scalar().clone();
                    let value = match op {
                        '+' => a,
                        'S' => Value::String(a.string()?),
                        '-' => {
                            let Value::Number(n) = a else {
                                bail!("N2System negate type")
                            };
                            Value::Number(-n)
                        }
                        '!' => Value::Boolean(!a.truth()?),
                        '@' => {
                            if let Ok(n) = a.number() {
                                Value::Delta(n)
                            } else {
                                Value::String(format!("@{}", a.text()?))
                            }
                        }
                        'I' | 'D' => {
                            let Value::Number(n) = a else {
                                bail!("N2System increment type")
                            };
                            Value::Number(n + if op == 'I' { 1. } else { -1. })
                        }
                        _ => bail!("N2System unary operation {op}"),
                    };
                    thread.push(value.into())?;
                }
                Instruction::Binary(op) => {
                    let left = thread.pop()?.value;
                    let right = thread.pop()?.value;
                    thread.push(left.binary(op, &right)?.into())?;
                }
                Instruction::Jump(target) => thread.frames.last_mut().unwrap().pc = target,
                Instruction::Unless(target) => {
                    if !thread.pop()?.value.truth()? {
                        thread.frames.last_mut().unwrap().pc = target;
                    }
                }
                Instruction::Return => {
                    thread.frames.pop();
                    thread.stack.truncate(frame.base);
                    if frame.result {
                        thread.push(Value::Null.into())?;
                    }
                }
                Instruction::Argument(name, index, target) => {
                    let Some(module) = self.resolve_in_thread(thread, &frame.module, &name)? else {
                        if let Some(module) = self.missing_import(&frame.module)? {
                            thread.frames.last_mut().unwrap().pc = frame.pc;
                            return Ok(Step::Load(module));
                        }
                        // Emitter.EmitFunctionCall (77a93db4) does not evaluate
                        // arguments of an unresolved user helper. Its diagnostic
                        // is recorded at Call below and its result is null.
                        thread.frames.last_mut().unwrap().pc = target;
                        continue;
                    };
                    if index >= self.modules[&module].routines[&name].parameters.len() {
                        thread.frames.last_mut().unwrap().pc = target;
                    }
                }
                Instruction::Call(name, count) => {
                    ensure!(
                        thread.stack.len() >= count,
                        "N2System native argument stack"
                    );
                    if let Some(function) = Builtin::parse(&name) {
                        let arguments = thread.stack.split_off(thread.stack.len() - count);
                        let call = Call {
                            function,
                            arguments,
                            module: frame.module,
                            routine: frame.routine,
                            instruction: frame.pc,
                        };
                        if let Some(value) = self.indexed_builtin(&call)? {
                            thread.push(value.into())?;
                        } else {
                            thread.pending = Some(call.clone());
                            return Ok(Step::Call(call));
                        }
                        continue;
                    }
                    let Some(module) = self.resolve_in_thread(thread, &frame.module, &name)? else {
                        let missing = self.modules[&frame.module]
                            .imports
                            .iter()
                            .map(|s| module_name(s))
                            .collect::<Result<Vec<_>>>()?
                            .into_iter()
                            .find(|s| !self.modules.contains_key(s));
                        if let Some(module) = missing {
                            thread.frames.last_mut().unwrap().pc = frame.pc;
                            return Ok(Step::Load(module));
                        }
                        ensure!(
                            self.unresolved_functions.len() < 256,
                            "N2System unresolved function diagnostic capacity"
                        );
                        self.unresolved_functions.insert((
                            frame.module,
                            frame.routine,
                            frame.pc,
                            name,
                        ));
                        thread.push(Value::Null.into())?;
                        continue;
                    };
                    let parameters = self.modules[&module].routines[&name].parameters.clone();
                    let count = count.min(parameters.len());
                    let args = thread.stack.split_off(thread.stack.len() - count);
                    for (parameter, arg) in parameters.iter().zip(args) {
                        self.assign(
                            &Name {
                                text: parameter.clone(),
                                flag: false,
                                explicit: true,
                            },
                            arg.value,
                        )?;
                    }
                    thread.call(module, name, true)?;
                }
                Instruction::Chapter => {
                    let value = thread.pop()?.value;
                    let module = module_name(&value.string()?)?;
                    let Some(source) = self.modules.get(&module) else {
                        thread.push(value.into())?;
                        thread.frames.last_mut().unwrap().pc = frame.pc;
                        return Ok(Step::Load(module));
                    };
                    let routine = source
                        .entry
                        .clone()
                        .context("N2System called chapter missing")?;
                    thread.call(module, routine, false)?;
                }
                Instruction::Scene => {
                    let value = thread.pop()?.value;
                    let target = value.string()?;
                    let (module, name) = if let Some((module, name)) = target.split_once("->") {
                        (
                            if module == "@" || module.is_empty() {
                                frame.module.clone()
                            } else {
                                module_name(module)?
                            },
                            name.to_owned(),
                        )
                    } else {
                        (frame.module.clone(), target)
                    };
                    if !self.modules.contains_key(&module) {
                        thread.push(value.into())?;
                        thread.frames.last_mut().unwrap().pc = frame.pc;
                        return Ok(Step::Load(module));
                    }
                    let owner = self
                        .resolve_in_thread(thread, &module, &name)?
                        .context("N2System called scene missing")?;
                    ensure!(
                        self.modules[&owner].routines[&name].kind == RoutineKind::Scene,
                        "N2System scene target has another routine kind"
                    );
                    return Ok(Step::Scene(owner, name));
                }
                Instruction::SelectBegin => thread.selected = false,
                Instruction::IsPressed => {
                    let query = thread.pop()?.value.string()?;
                    return Ok(Step::Choice(query));
                }
                Instruction::SelectYield => {
                    thread.push(Value::Boolean(thread.selected).into())?;
                    return Ok(Step::Frame);
                }
                Instruction::SelectEnd => return Ok(Step::SelectEnd),
                Instruction::Unsupported(line, syntax) => bail!(
                    "N2System {} line {line}: {syntax} is not covered by the recovered source grammar",
                    frame.module
                ),
                Instruction::ClearText => return Ok(Step::ClearText),
                Instruction::AppendText(text) => return Ok(Step::AppendText(text)),
                Instruction::LineEnd => return Ok(Step::LineEnd),
                Instruction::Activate(name) => {
                    self.variables.insert(
                        "SYSTEM_present_preprocess".into(),
                        Value::String(routine.blocks[&name].window.clone()),
                    );
                    self.variables.insert(
                        "SYSTEM_present_text".into(),
                        Value::String(format!("@{name}")),
                    );
                }
                Instruction::Curve(starting) => {
                    ensure!(
                        thread.stack.len() >= starting.len() * 2,
                        "N2System Bezier operands"
                    );
                    let args = thread
                        .stack
                        .split_off(thread.stack.len() - starting.len() * 2);
                    let points = args
                        .chunks_exact(2)
                        .map(|a| (Box::new(a[0].value.clone()), Box::new(a[1].value.clone())))
                        .collect::<Vec<_>>();
                    let mut segments = Vec::new();
                    let mut at = 0;
                    while at < points.len() {
                        ensure!(starting[at], "N2System Bezier segment start");
                        let last = if at + 3 < points.len()
                            && !starting[at + 1]
                            && !starting[at + 2]
                            && starting[at + 3]
                        {
                            at + 3
                        } else {
                            bail!("N2System Bezier needs four control points")
                        };
                        segments.push([
                            points[at].clone(),
                            points[at + 1].clone(),
                            points[at + 2].clone(),
                            points[last].clone(),
                        ]);
                        if last + 1 == points.len() {
                            break;
                        }
                        at = last;
                    }
                    thread.push(Value::Curve(segments).into())?;
                }
            }
        }
        Ok(Step::Budget)
    }
}
impl Thread {
    fn push(&mut self, arg: Argument) -> Result<()> {
        ensure!(self.stack.len() < 65536, "N2System evaluation stack limit");
        self.stack.push(arg);
        Ok(())
    }
    fn pop(&mut self) -> Result<Argument> {
        self.stack
            .pop()
            .context("N2System evaluation stack underflow")
    }
    fn call(&mut self, module: String, routine: String, result: bool) -> Result<()> {
        ensure!(self.frames.len() < 256, "N2System call stack limit");
        self.frames.push(Frame {
            module,
            routine,
            pc: 0,
            base: self.stack.len(),
            result,
        });
        Ok(())
    }
    pub fn choice_result(&mut self, pressed: bool) -> Result<()> {
        let frame = self
            .frames
            .last_mut()
            .context("N2System selection without call frame")?;
        let _ = frame;
        self.selected |= pressed;
        self.push(Value::Boolean(pressed).into())
    }
}
