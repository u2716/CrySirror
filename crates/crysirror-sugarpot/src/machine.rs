// SPDX-License-Identifier: MPL-2.0
//! Native control/value state only. 44C880 owns cross-file gsb/go, section/local
//! labels, conditional skips and non-fallthrough switches. The adapter admits
//! each presentation/resource command before committing its next native line.
use crate::{
    expression::{self, Value},
    program::{self, Instruction, Program},
};
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Arc};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Frame {
    pub script: String,
    pub native_script: String,
    pub context: String,
    pub section: String,
    pub pc: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Loop {
    pub script: String,
    pub section: String,
    pub pc: usize,
    pub call_depth: usize,
    pub macro_depth: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct State {
    pub frame: Frame,
    pub stack: Vec<Frame>,
    pub loops: Vec<Loop>,
    pub macro_stack: Vec<Frame>,
    pub variables: BTreeMap<String, Value>,
    pub definitions: BTreeMap<String, Value>,
    pub variable_order: Vec<String>,
    pub definition_order: Vec<String>,
    pub list_names: BTreeMap<String, Vec<String>>,
    pub definition_list_names: BTreeMap<String, Vec<String>>,
    pub executed: u64,
    pub halted: bool,
}
#[derive(Clone)]
pub struct Machine {
    pub programs: BTreeMap<String, Arc<Program>>,
    pub state: State,
    macros: BTreeMap<String, String>,
}
#[derive(Clone, Debug)]
pub enum Event {
    Blank,
    Command {
        name: String,
        arguments: Vec<String>,
    },
    Text(String),
    Selection(Vec<String>),
    Window(i32),
    Halted,
}
impl Machine {
    pub fn new(programs: BTreeMap<String, Arc<Program>>, script: &str) -> Result<Self> {
        let mut variables = BTreeMap::new();
        variables.insert("reg".into(), Value::String(String::new()));
        variables.insert("dbg".into(), Value::Integer(0));
        variables.insert("bgm".into(), Value::String(String::new()));
        let mut definitions = BTreeMap::new();
        let mut definition_order = Vec::new();
        //458870 exact native DEF bank; these names retain original case.
        for (name, n) in [
            ("ALL", -1),
            ("NUM", 0),
            ("SIZE", 1),
            ("POS", 2),
            ("TRANS", 3),
            ("MEMBER", 3),
            ("SYS", 4),
            ("TXT", 4),
            ("MAX", 4),
            ("ADD", 5),
            ("CEL", 5),
            ("DIR", 5),
            ("PNT", 6),
            ("FRM", 6),
            ("COLOR", 6),
            ("FP", 7),
            ("VALUE", 7),
            ("PP", 8),
            ("PER", 8),
            ("BTN", 9),
            ("MOD", 10),
            ("BL_BCK", 0),
            ("BL_FWD", 1),
            ("BL_TOP", 2),
            ("BL_BTM", 3),
            ("BL_EXT", 4),
            ("OL_CUTIN", 0),
            ("OL_RGB", 1),
            ("OL_MASK", 2),
            ("OL_BL", 10),
            ("OL_BR", 11),
            ("OL_BT", 12),
            ("OL_BB", 13),
            ("OL_WL", 15),
            ("OL_WR", 16),
            ("OL_WT", 17),
            ("OL_WB", 18),
            ("EF_L2R", 1),
            ("EF_R2L", 2),
            ("EF_T2B", 3),
            ("EF_B2T", 4),
            ("EF_SLR", 11),
            ("EF_STB", 12),
            ("EF_RLR", 16),
            ("EF_RRL", 17),
            ("LEFT", 65520),
            ("CENTER", 65521),
            ("RIGHT", 65522),
            ("NON", 65523),
        ] {
            definitions.insert(name.into(), Value::Integer(n));
            definition_order.push(name.into());
        }
        let frame = Frame {
            script: String::new(),
            native_script: String::new(),
            context: String::new(),
            section: String::new(),
            pc: 0,
        };
        let mut m = Self {
            programs,
            state: State {
                frame,
                stack: vec![],
                loops: vec![],
                macro_stack: vec![],
                variables,
                definitions,
                variable_order: vec!["reg".into(), "dbg".into(), "bgm".into()],
                definition_order,
                list_names: BTreeMap::new(),
                definition_list_names: BTreeMap::new(),
                executed: 0,
                halted: false,
            },
            macros: BTreeMap::new(),
        };
        m.enter(script, "START")?;
        if script.eq_ignore_ascii_case("start.scr") {
            m.state.frame.native_script = "START".into();
        }
        Ok(m)
    }
    pub fn current(&self) -> Result<&Instruction> {
        self.programs
            .get(&self.state.frame.script)
            .context("SugarPot current script unavailable")?
            .instructions
            .get(self.state.frame.pc)
            .context("SugarPot PC outside script")
    }
    pub fn commit(&mut self) {
        self.state.frame.pc += 1;
        self.state.executed += 1;
    }
    pub fn value(&self, s: &str) -> Result<Value> {
        expression::value_banks(s, &self.state.variables, &self.state.definitions)
    }
    pub fn number(&self, s: &str) -> Result<i32> {
        expression::integer_banks(s, &self.state.variables, &self.state.definitions)
    }
    pub fn set_reg(&mut self, n: i32) {
        self.state.variables.insert("reg".into(), Value::Integer(n));
    }
    fn enter(&mut self, script: &str, section: &str) -> Result<()> {
        let native_script = script.trim_matches('"').trim_end_matches(".scr").to_owned();
        let script = script
            .trim_matches('"')
            .trim_end_matches(".scr")
            .to_ascii_lowercase()
            + ".scr";
        let section = section.to_owned();
        let p = self
            .programs
            .get(&script)
            .with_context(|| format!("SugarPot missing script {script}"))?;
        let pc = *p
            .sections
            .get(&section)
            .with_context(|| format!("SugarPot missing section {script}.{section}"))?;
        let definitions = p.macros.clone();
        for (name, body) in definitions {
            let key = format!("#macro:{name}");
            self.macros.insert(name.clone(), key.clone());
            self.programs.insert(
                key.clone(),
                Arc::new(Program {
                    name: key,
                    instructions: body,
                    sections: BTreeMap::new(),
                    labels: BTreeMap::new(),
                    macros: BTreeMap::new(),
                }),
            );
        }
        self.state.frame = Frame {
            script,
            native_script,
            context: String::new(),
            section,
            pc,
        };
        Ok(())
    }
    fn transfer(&mut self, target: &str, local: bool, push: bool) -> Result<()> {
        let target = if target.starts_with('$') {
            self.value(target)?.string()?.to_owned()
        } else {
            target.trim_matches('"').to_owned()
        };
        ensure!(self.state.stack.len() < 256, "SugarPot native call depth");
        if push {
            let mut caller = self.state.frame.clone();
            caller.pc += 1;
            self.state.stack.push(caller);
        }
        if local {
            let label = target.trim_start_matches('.').to_owned();
            let frame = &mut self.state.frame;
            let p = &self.programs[&frame.script];
            frame.context.clear();
            frame.pc = *p
                .labels
                .get(&(frame.section.clone(), label.clone()))
                .with_context(|| format!("SugarPot missing local label {label}"))?;
        } else {
            let (script, section) = target.split_once('.').unwrap_or((&target, "START"));
            let script = if script.is_empty() {
                self.state.frame.script.clone()
            } else {
                script.into()
            };
            self.enter(&script, section)?;
        }
        Ok(())
    }
    fn scan(&self, start: usize, opening: &str, closing: &str, branches: &[&str]) -> Result<usize> {
        let p = &self.programs[&self.state.frame.script];
        let mut depth = 0usize;
        for (pc, line) in p.instructions.iter().enumerate().skip(start) {
            if line.source.starts_with("::") {
                bail!(
                    "SugarPot control crosses section at {}:{}",
                    p.name,
                    line.line
                );
            }
            let Some(flow) = line.source.strip_prefix('^') else {
                continue;
            };
            let (name, _) = program::call(flow)?;
            if name == opening {
                depth += 1;
            } else if name == closing {
                if depth == 0 {
                    return Ok(pc);
                }
                depth -= 1;
            } else if depth == 0 && branches.contains(&name) {
                return Ok(pc);
            }
        }
        bail!("SugarPot missing ^{closing}")
    }
    fn assignment(&mut self, line: &str) -> Result<()> {
        let end = line[1..]
            .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
            .context("SugarPot assignment operator missing")?
            + 1;
        let key = line[1..end].to_owned();
        ensure!(
            self.state.variables.contains_key(&key) || self.state.definitions.contains_key(&key),
            "SugarPot assignment to undeclared variable {key}"
        );
        let tail = line[end..].trim();
        let op = ["+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=", "="]
            .into_iter()
            .find(|op| tail.starts_with(op))
            .context("unsupported SugarPot assignment operator")?;
        let value = self.value(tail[op.len()..].trim())?;
        let value = if line.starts_with('%') {
            Value::Integer(value.integer()?)
        } else {
            Value::String(value.string()?.to_owned())
        };
        //44C880 assignment first looks in VAR, then DEF; argument parsing
        //still uses the explicit prefix/bare bank selection in451920.
        let in_var = self.state.variables.contains_key(&key);
        let old = if in_var {
            &self.state.variables[&key]
        } else {
            &self.state.definitions[&key]
        }
        .clone();
        let value = if op == "=" {
            value
        } else if op == "+=" && line.starts_with('$') {
            Value::String(
                if let Value::String(old) = old {
                    old
                } else {
                    String::new()
                } + value.string()?,
            )
        } else {
            let a = if let Value::Integer(old) = old {
                old
            } else {
                0
            };
            let b = value.integer()?;
            Value::Integer(match op {
                "+=" => a.wrapping_add(b),
                "-=" => a.wrapping_sub(b),
                "*=" => a.wrapping_mul(b),
                "/=" => {
                    ensure!(b != 0, "SugarPot zero divisor");
                    a.wrapping_div(b)
                }
                "%=" => {
                    ensure!(b != 0, "SugarPot zero divisor");
                    a.wrapping_rem(b)
                }
                "&=" => a & b,
                "|=" => a | b,
                "^=" => a ^ b,
                _ => bail!("unsupported SugarPot assignment"),
            })
        };
        if in_var {
            self.state.variables.insert(key, value);
        } else {
            self.state.definitions.insert(key, value);
        }
        Ok(())
    }
    /// Execute one native control instruction or return a service event at the
    /// unchanged PC. A retry never repeats a completed branch/assignment.
    pub fn event(&mut self) -> Result<Option<Event>> {
        if self.state.halted {
            return Ok(Some(Event::Halted));
        }
        if self.state.frame.script.starts_with("#macro:")
            && self.state.frame.pc >= self.programs[&self.state.frame.script].instructions.len()
        {
            self.state.frame = self
                .state
                .macro_stack
                .pop()
                .context("SugarPot macro return ownership")?;
            self.state
                .loops
                .retain(|l| l.macro_depth <= self.state.macro_stack.len());
            return Ok(None);
        }
        let line = self.current()?.source.clone();
        if line.is_empty() {
            return Ok(Some(Event::Blank));
        }
        if line.starts_with(':') {
            self.commit();
            return Ok(None);
        }
        if line.starts_with('!') {
            self.commit();
            return Ok(None);
        } //44C880 case0x21 is an explicit native no-op.
        if line.starts_with(['%', '$']) {
            self.assignment(&line)?;
            self.commit();
            return Ok(None);
        }
        if let Some(flow) = line.strip_prefix('^') {
            let (name, args) = program::call(flow)?;
            match name {
                "go" | "gsb" | "jp" | "jsb" => {
                    ensure!(args.len() == 1, "SugarPot transfer signature");
                    self.transfer(
                        &args[0],
                        matches!(name, "jp" | "jsb"),
                        matches!(name, "gsb" | "jsb"),
                    )?;
                    self.state.executed += 1;
                }
                "ret" => {
                    let mut caller = self
                        .state
                        .stack
                        .pop()
                        .context("SugarPot ^ret without native caller")?;
                    let source = &self.programs[&caller.script].instructions[caller.pc - 1].source;
                    if source.starts_with("^jsb") {
                        caller.context.clear();
                    }
                    self.state.frame = caller;
                    self.state
                        .loops
                        .retain(|l| l.call_depth <= self.state.stack.len());
                    self.state.executed += 1;
                }
                "if" => {
                    ensure!(args.len() == 1, "SugarPot if signature");
                    if expression::condition_banks(
                        &args[0],
                        &self.state.variables,
                        &self.state.definitions,
                    )? {
                        self.state.frame.context = format!("^if{}", self.state.frame.pc + 1);
                        self.commit();
                    } else {
                        let mut pc = self.state.frame.pc + 1;
                        loop {
                            pc = self.scan(pc, "if", "endi", &["elif", "else"])?;
                            let (name, args) = program::call(
                                &self.programs[&self.state.frame.script].instructions[pc].source
                                    [1..],
                            )?;
                            if name != "elif"
                                || expression::condition_banks(
                                    args.first().context("SugarPot elif condition")?,
                                    &self.state.variables,
                                    &self.state.definitions,
                                )?
                            {
                                self.state.frame.pc = pc + 1;
                                self.state.frame.context = format!("^if{}", pc + 1);
                                break;
                            }
                            pc += 1;
                        }
                        self.state.executed += 1;
                    }
                }
                "elif" | "else" => {
                    self.state.frame.pc =
                        self.scan(self.state.frame.pc + 1, "if", "endi", &[])? + 1;
                    self.state.frame.context = format!("^if{}", self.state.frame.pc);
                    self.state.executed += 1;
                }
                "endi" | "ends" => {
                    self.state.frame.context = format!(
                        "^{}{}",
                        if name == "endi" { "if" } else { "sw" },
                        self.state.frame.pc + 1
                    );
                    self.commit();
                }
                "sw" => {
                    ensure!(args.len() == 1, "SugarPot switch signature");
                    let value = self.value(&args[0])?;
                    let mut pc = self.state.frame.pc + 1;
                    loop {
                        pc = self.scan(pc, "sw", "ends", &["case", "df"])?;
                        let (name, args) = program::call(
                            &self.programs[&self.state.frame.script].instructions[pc].source[1..],
                        )?;
                        if name != "case"
                            || self.value(args.first().context("SugarPot case value")?)? == value
                        {
                            self.state.frame.pc = pc + 1;
                            self.state.frame.context = format!("^sw{}", pc + 1);
                            break;
                        }
                        pc += 1;
                    }
                    self.state.executed += 1;
                }
                "case" | "df" => {
                    self.state.frame.pc =
                        self.scan(self.state.frame.pc + 1, "sw", "ends", &[])? + 1;
                    self.state.frame.context = format!("^sw{}", self.state.frame.pc);
                    self.state.executed += 1;
                }
                "wh" => {
                    ensure!(args.len() == 1, "SugarPot while signature");
                    if expression::condition_banks(
                        &args[0],
                        &self.state.variables,
                        &self.state.definitions,
                    )? {
                        ensure!(self.state.loops.len() < 256, "SugarPot loop stack budget");
                        let f = &self.state.frame;
                        self.state.loops.push(Loop {
                            script: f.script.clone(),
                            section: f.section.clone(),
                            pc: f.pc,
                            call_depth: self.state.stack.len(),
                            macro_depth: self.state.macro_stack.len(),
                        });
                        self.commit();
                    } else {
                        self.state.frame.pc =
                            self.scan(self.state.frame.pc + 1, "wh", "endw", &[])? + 1;
                        self.state.executed += 1;
                    }
                }
                "endw" | "cont" | "brk" => {
                    let l = self
                        .state
                        .loops
                        .last()
                        .context("SugarPot loop control without while")?
                        .clone();
                    ensure!(
                        l.script == self.state.frame.script
                            && l.section == self.state.frame.section
                            && l.call_depth == self.state.stack.len()
                            && l.macro_depth == self.state.macro_stack.len(),
                        "SugarPot loop continuation scope"
                    );
                    let (_, condition) =
                        program::call(&self.programs[&l.script].instructions[l.pc].source[1..])?;
                    let continuing = name != "brk"
                        && expression::condition_banks(
                            condition
                                .first()
                                .context("SugarPot retained while condition")?,
                            &self.state.variables,
                            &self.state.definitions,
                        )?;
                    if continuing {
                        self.state.frame.pc = l.pc + 1;
                    } else {
                        // Portable repair of native cont(false): its popped
                        // while node can leave endw dereferencing absent state.
                        // Exit the matching bounded loop consistently.
                        self.state.frame.pc = self.scan(l.pc + 1, "wh", "endw", &[])? + 1;
                        self.state.loops.pop();
                    }
                    self.state.executed += 1;
                }
                _ => bail!("unsupported SugarPot native control ^{name}"),
            }
            return Ok(None);
        }
        let first = line.chars().next().unwrap();
        if first.is_ascii_alphabetic() || first == '_' {
            let (name, args) = program::call(&line)?;
            if self.macros.contains_key(name) {
                ensure!(
                    args.is_empty(),
                    "SugarPot macro arguments are not native parameters"
                );
                self.invoke_macro(name, true)?;
                return Ok(None);
            }
            return Ok(Some(Event::Command {
                name: name.into(),
                arguments: args,
            }));
        }
        if first == '/' {
            let body = line
                .strip_prefix('/')
                .and_then(|s| s.split_once("//").map(|(body, _)| body))
                .context("SugarPot selection terminator")?;
            let options = body.split('/').map(str::to_owned).collect::<Vec<_>>();
            ensure!(
                !options.is_empty() && options.len() <= 15 && options.iter().all(|s| !s.is_empty()),
                "SugarPot native fifteen-choice budget"
            );
            return Ok(Some(Event::Selection(options)));
        }
        if first == '>' {
            return Ok(Some(Event::Window(self.number(line[1..].trim())?)));
        }
        if matches!(first, '[' | '\\' | '@') || !first.is_ascii() {
            return Ok(Some(Event::Text(line)));
        }
        bail!("unsupported SugarPot native line {line:?}")
    }
    pub fn invoke_macro(&mut self, name: &str, advance: bool) -> Result<()> {
        let script = self
            .macros
            .get(name)
            .cloned()
            .with_context(|| format!("SugarPot unknown macro {name}"))?;
        ensure!(
            self.state.macro_stack.len() < 64,
            "SugarPot macro recursion budget"
        );
        let mut frame = self.state.frame.clone();
        if advance {
            frame.pc += 1;
        }
        self.state.macro_stack.push(frame.clone());
        self.state.frame = Frame {
            script,
            native_script: frame.native_script,
            context: frame.context,
            section: frame.section,
            pc: 0,
        };
        Ok(())
    }
    pub fn return_title(&mut self) -> Result<()> {
        self.state.stack.clear();
        self.state.loops.clear();
        self.state.macro_stack.clear();
        self.enter("TITLE", "START")
    }
    pub fn declare(&mut self, args: &[String]) -> Result<()> {
        for arg in args {
            let first = arg.chars().next().context("SugarPot empty variable")?;
            ensure!(
                matches!(first, '%' | '$' | '!'),
                "SugarPot variable type {first} unsupported"
            );
            let key = arg[1..].to_owned();
            ensure!(
                !self.state.variables.contains_key(&key),
                "duplicate SugarPot variable {key}"
            );
            self.state.variable_order.push(key.clone());
            self.state.variables.insert(
                key,
                if first == '$' {
                    Value::String(String::new())
                } else if first == '!' {
                    Value::Float(0.)
                } else {
                    Value::Integer(0)
                },
            );
        }
        Ok(())
    }
}

impl Machine {
    ///4561A0 reconstructs source nodes and linked native calls independently of
    ///the displayed page. Saved PCs are original one-based node numbers; the
    ///next interpreter fetch increments6000 before dispatching its next node.
    pub(crate) fn from_original_slot(
        template: &Self,
        slot: &crate::native_storage::slot::Slot,
    ) -> Result<Self> {
        let programs = template.programs.clone();
        use crate::native_storage::{
            NativeValue,
            slot::{Call, StoredValue},
        };
        fn key(name: &str) -> Result<String> {
            ensure!(
                !name.is_empty() && !name.contains(['/', '\\', ':', '\0']),
                "SugarPot saved script basename"
            );
            let name = name.to_ascii_lowercase();
            Ok(if name.ends_with(".scr") {
                name
            } else {
                name + ".scr"
            })
        }
        fn frame(
            programs: &BTreeMap<String, Arc<Program>>,
            script: &str,
            section: &str,
            pc: i32,
            context: &str,
        ) -> Result<Frame> {
            let key = key(script)?;
            let program = programs
                .get(&key)
                .with_context(|| format!("SugarPot saved script {script} unavailable"))?;
            ensure!(
                program.sections.contains_key(section),
                "SugarPot saved section unavailable"
            );
            let pc = usize::try_from(pc).context("SugarPot saved negative PC")?;
            ensure!(
                pc <= program.instructions.len(),
                "SugarPot saved PC outside source nodes"
            );
            Ok(Frame {
                script: key,
                native_script: script.into(),
                section: section.into(),
                pc,
                context: context.into(),
            })
        }
        fn value(v: &NativeValue) -> Value {
            match v {
                NativeValue::Integer(n) => Value::Integer(*n),
                NativeValue::FloatBits(n) => Value::Float(f32::from_bits(*n)),
                NativeValue::Text(s) => Value::String(s.clone()),
            }
        }
        let current = frame(
            &programs,
            &slot.script,
            &slot.section,
            slot.line,
            &slot.display_caption,
        )?;
        let mut machine = Self::new(programs, &current.script)?;
        //Native restore retains already registered macro names and then
        //registers the loaded file's definitions. This cannot be reconstructed
        //from the display string or only the final file's macro declarations.
        let current_macros = machine.macros.clone();
        machine.macros = template.macros.clone();
        machine.macros.extend(current_macros);
        machine.state.frame = current;
        machine.state.variables.clear();
        machine.state.definitions.clear();
        machine.state.list_names.clear();
        machine.state.definition_list_names.clear();
        machine.state.variable_order.clear();
        machine.state.definition_order.clear();
        for (definitions, records) in [(false, &slot.variables), (true, &slot.definitions)] {
            for record in records {
                ensure!(
                    !record.name.is_empty() && record.name.len() <= 255,
                    "SugarPot saved variable name"
                );
                let bank = if definitions {
                    &mut machine.state.definitions
                } else {
                    &mut machine.state.variables
                };
                ensure!(
                    !bank.contains_key(&record.name),
                    "SugarPot duplicate name within original native bank"
                );
                let v = match &record.value {
                    StoredValue::Scalar(v) => value(v),
                    StoredValue::List {
                        element_name,
                        elements,
                    } => {
                        let names = if definitions {
                            &mut machine.state.definition_list_names
                        } else {
                            &mut machine.state.list_names
                        };
                        names.insert(
                            record.name.clone(),
                            vec![element_name.clone(); elements.len()],
                        );
                        Value::List(elements.iter().map(value).collect())
                    }
                };
                bank.insert(record.name.clone(), v);
                if definitions {
                    machine.state.definition_order.push(record.name.clone());
                } else {
                    machine.state.variable_order.push(record.name.clone());
                }
            }
        }
        ensure!(
            slot.variables.first().is_some_and(|v| v.name == "reg")
                && machine.state.variables.contains_key("dbg")
                && machine.state.variables.contains_key("bgm"),
            "SugarPot saved reserved VAR prefix"
        );
        ensure!(
            matches!(slot.calls.first(), Some(Call::Top))
                && slot.calls.iter().skip(1).all(|c| !matches!(c, Call::Top)),
            "SugarPot saved native top call order"
        );
        let mut loops = vec![];
        for (index, call) in slot.calls.iter().enumerate().skip(1) {
            match call {
                Call::Top => unreachable!(),
                Call::Gsb {
                    script,
                    section,
                    pc,
                    continuation,
                } => {
                    let f = frame(&machine.programs, script, section, *pc, continuation)?;
                    let ins = machine.programs[&f.script]
                        .instructions
                        .get(f.pc.checked_sub(1).context("SugarPot saved gsb PC0")?)
                        .context("SugarPot saved gsb node")?;
                    ensure!(
                        ins.source.starts_with("^gsb"),
                        "SugarPot saved gsb source mismatch"
                    );
                    machine.state.stack.push(f);
                }
                Call::Jsb { section, pc } => {
                    //jsb has no file field. Its file is the next gsb caller's
                    //file, or the final saved file when no cross-file call
                    //lies inside it. Do not alias every jsb to the root file.
                    let script = slot.calls[index + 1..]
                        .iter()
                        .find_map(|c| {
                            if let Call::Gsb { script, .. } = c {
                                Some(script.as_str())
                            } else {
                                None
                            }
                        })
                        .unwrap_or(&slot.script);
                    let f = frame(&machine.programs, script, section, *pc, "")?;
                    let ins = machine.programs[&f.script]
                        .instructions
                        .get(f.pc.checked_sub(1).context("SugarPot saved jsb PC0")?)
                        .context("SugarPot saved jsb node")?;
                    ensure!(
                        ins.source.starts_with("^jsb"),
                        "SugarPot saved jsb source mismatch"
                    );
                    machine.state.stack.push(f);
                }
                Call::While(pc) => loops.push((machine.state.stack.len(), *pc)),
            }
        }
        for (depth, pc) in loops {
            let owner = machine
                .state
                .stack
                .get(depth)
                .unwrap_or(&machine.state.frame);
            let pc = usize::try_from(pc)?
                .checked_sub(1)
                .context("SugarPot saved while PC0")?;
            let program = &machine.programs[&owner.script];
            ensure!(
                program
                    .instructions
                    .get(pc)
                    .is_some_and(|i| i.source.starts_with("^wh")),
                "SugarPot saved while source mismatch"
            );
            let section = program
                .sections
                .iter()
                .filter(|(_, at)| **at <= pc + 1)
                .max_by_key(|(_, at)| **at)
                .map(|(s, _)| s.clone())
                .context("SugarPot saved while section")?;
            machine.state.loops.push(Loop {
                script: owner.script.clone(),
                section,
                pc,
                call_depth: depth,
                macro_depth: 0,
            });
        }
        Ok(machine)
    }
}
