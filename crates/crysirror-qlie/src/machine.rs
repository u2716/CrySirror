use crate::{
    expression::{self, Value},
    program::{self, Instruction, Program},
};
use anyhow::{bail, ensure, Context, Result};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

#[derive(Clone, Debug, Default)]
pub struct Frame {
    pub path: String,
    pub pc: usize,
    pub locals: BTreeMap<String, Value>,
    pub loops: BTreeSet<usize>,
    pub cases: Vec<(Value, bool, usize)>,
    pub inline: bool,
    pub arrays: BTreeMap<String, usize>,
}
pub struct Machine {
    pub programs: BTreeMap<String, Arc<Program>>,
    pub frame: Frame,
    pub stack: Vec<Frame>,
    pub globals: BTreeMap<String, Value>,
    pub object_types: BTreeMap<String, String>,
    pub arrays: BTreeMap<String, usize>,
    pub scope: String,
    pub executed: u64,
    next_object: u32,
    transient: BTreeSet<String>,
    next_transient: u64,
}
impl Machine {
    pub fn new(programs: BTreeMap<String, Arc<Program>>, entry: &str) -> Result<Self> {
        let path = program::canonical(entry)?;
        let p = programs.get(&path).context("QLIE startup.s missing")?;
        let pc = p
            .labels
            .get("@@top")
            .or_else(|| p.labels.get("@@!top"))
            .copied()
            .unwrap_or(0);
        let mut m = Self {
            programs,
            frame: Frame {
                path,
                pc,
                ..Default::default()
            },
            stack: Vec::new(),
            globals: BTreeMap::new(),
            object_types: BTreeMap::new(),
            arrays: BTreeMap::new(),
            scope: String::new(),
            executed: 0,
            next_object: 0,
            transient: BTreeSet::new(),
            next_transient: 0,
        };
        for prefix in [
            "resultint",
            "resultstr",
            "resultobj",
            "resultbtnstr",
            "resultbtnint",
        ] {
            for n in 0..64 {
                m.globals.insert(
                    format!("{prefix}[{n}]"),
                    if prefix.ends_with("str") {
                        Value::Text(String::new())
                    } else {
                        Value::Int(0)
                    },
                );
            }
        }
        for builtin in [
            "sys", "cfg", "skey", "inp", "dat", "mdat", "eff", "rep", "log", "flog", "llog",
            "misc", "cam", "camex", "dlgs", "tdat",
        ] {
            m.globals
                .insert(builtin.into(), Value::Text(builtin.into()));
            m.object_types.insert(builtin.into(), builtin.into());
        }
        for prefix in ["snd", "dsnd", "ssnd"] {
            for n in 0..64 {
                let key = format!("{prefix}[{n}]");
                m.globals.insert(key.clone(), Value::Text(key.clone()));
                m.object_types.insert(key, "snd".into());
            }
        }
        m.globals.insert("resultstopint".into(), Value::Int(0));
        // Native TImoScript.Initialize constant registration (not host keycodes).
        for (name, value) in [
            ("_lkMinimize", 1),
            ("_lkScreen", 2),
            ("_lkSave", 4),
            ("_lkLoad", 8),
            ("_lkConfig", 16),
            ("_lkLog", 32),
            ("_lkView", 64),
            ("_lkAuto", 128),
            ("_lkSkip", 256),
            ("_lkJump", 512),
            ("_lkMenu", 1024),
            ("_lkReset", 2048),
            ("_stSE", 0),
            ("_stBGM", 1),
            ("_stVOICE", 2),
            ("_mmDivide", 0),
            ("_mmStep", 1),
            ("_mmLinear", 3),
            ("_mmOutQuart", 4),
            ("_mmInQuart", 7),
            ("_mmOutInQuart", 8),
            ("_mmInOutQuart", 12),
            ("_mmSquare", 13),
            ("_mmTriangle", 14),
            ("_mmSin", 15),
            ("_mmOutElastic", 16),
            ("_mmInElastic", 17),
            ("_mmOutBack", 18),
            ("_mmInBack", 19),
            ("_mmOutBounce", 20),
            ("_mmInBounce", 21),
        ] {
            m.globals
                .insert(name.to_ascii_lowercase(), Value::Int(value));
        }
        Ok(m)
    }
    pub fn current(&self) -> Result<Option<Instruction>> {
        Ok(self
            .programs
            .get(&self.frame.path)
            .context("QLIE current module")?
            .code
            .get(self.frame.pc)
            .cloned())
    }
    pub fn value(&self, name: &str) -> Result<Value> {
        let name = name.to_ascii_lowercase();
        if let Some(value) = self.frame.locals.get(&name) {
            return Ok(value.clone());
        }
        if self.frame.arrays.get(&name).is_some_and(|count| *count > 0) {
            if let Some(value) = self.frame.locals.get(&format!("{name}[0]")) {
                return Ok(value.clone());
            }
        }
        if let Some(value) = self.globals.get(&name) {
            return Ok(value.clone());
        }
        if self.arrays.get(&name).is_some_and(|count| *count > 0) {
            if let Some(value) = self.globals.get(&format!("{name}[0]")) {
                return Ok(value.clone());
            }
        }
        if name.starts_with("paramstr[") {
            return Ok(Value::Text(String::new()));
        }
        if name.starts_with("paramint[") {
            return Ok(Value::Int(0));
        }
        match name.as_str() {
            "true" => Ok(Value::Int(1)),
            // TImoScript.Initialize registers #quot as a string constant.
            "#quot" => Ok(Value::Text("\"".into())),
            "false" | "null" | "nil" => Ok(Value::Int(0)),
            "self" => Ok(Value::Text(self.frame.path.clone())),
            "selffilename" | "selffilepath" => {
                // Generated ET/NT commands execute in their lexical reader;
                // their temporary module names are not native script files.
                let frame = if self.frame.inline {
                    self.stack
                        .iter()
                        .rev()
                        .find(|f| !f.inline)
                        .unwrap_or(&self.frame)
                } else {
                    &self.frame
                };
                Ok(Value::Text(if name == "selffilepath" {
                    frame
                        .path
                        .rsplit_once('/')
                        .map(|(path, _)| format!("{path}/"))
                        .unwrap_or_default()
                } else {
                    frame.path.clone()
                }))
            }
            "selfthreadname" => Ok(Value::Text("game".into())),
            _ => bail!("undefined QLIE variable {name}"),
        }
    }
    pub fn eval(&self, s: &str) -> Result<Value> {
        if s.trim().is_empty() {
            return Ok(Value::Text(String::new()));
        }
        expression::evaluate(s, |name| self.value(name))
    }
    fn call_parameter(&self, s: &str) -> Result<Value> {
        // Native call parameters enable plain-string mode in Calculation.
        // Defined variables and expressions still evaluate normally.
        match self.eval(s) {
            Ok(value) => Ok(value),
            Err(_) => Ok(Value::Text(s.to_owned())),
        }
    }
    /// Execute generated native commands in the caller's local scope. Only
    /// this temporary command stream is owned here; image/text work remains
    /// with the normal semantic adapter and shared services.
    pub fn inject(&mut self, source: &str, bindings: BTreeMap<String, Value>) -> Result<()> {
        ensure!(
            source.len() <= 1024 * 1024 && self.transient.len() < 256,
            "QLIE dynamic command budget"
        );
        self.next_transient = self
            .next_transient
            .checked_add(1)
            .context("QLIE dynamic command id")?;
        let directory = self
            .frame
            .path
            .rsplit_once('/')
            .map(|(p, _)| p)
            .unwrap_or("");
        let name = if directory.is_empty() {
            format!("__qlie_eval_{}.s", self.next_transient)
        } else {
            format!("{directory}/__qlie_eval_{}.s", self.next_transient)
        };
        let mut program = Program::parse_text(&name, source)?;
        program.labels.insert("@@top".into(), 0);
        program.includes.push(self.frame.path.clone());
        for ins in &mut program.code {
            ins.preprocess = false;
        }
        let arrays = self.frame.arrays.clone();
        let mut locals = self.frame.locals.clone();
        locals.extend(bindings);
        self.programs.insert(name.clone(), Arc::new(program));
        self.transient.insert(name.clone());
        self.jump("@@top", Some(&name), Some(vec![]), false)?;
        self.frame.locals = locals;
        self.frame.arrays = arrays;
        self.frame.inline = true;
        Ok(())
    }
    pub fn key(&self, s: &str) -> Result<String> {
        let s = s.trim();
        if let Some((name, tail)) = s.split_once('[') {
            let index = tail.strip_suffix(']').context("QLIE array name")?;
            let index = self.eval(index)?.integer()?;
            ensure!((0..65536).contains(&index), "QLIE array bounds");
            Ok(format!("{}[{index}]", name.to_ascii_lowercase()))
        } else {
            Ok(s.to_ascii_lowercase())
        }
    }
    pub fn assign(&mut self, name: &str, value: Value) -> Result<()> {
        let mut key = self.key(name)?;
        if !self.frame.locals.contains_key(&key)
            && (self.frame.arrays.get(&key).is_some_and(|count| *count > 0)
                || (!self.globals.contains_key(&key)
                    && self.arrays.get(&key).is_some_and(|count| *count > 0)))
        {
            key = format!("{key}[0]");
        }
        ensure!(
            self.frame.locals.contains_key(&key) || self.globals.contains_key(&key),
            "assignment to undefined QLIE variable {key}"
        );
        if self.frame.locals.contains_key(&key) {
            self.frame.locals.insert(key, value);
        } else {
            self.globals.insert(key, value);
        }
        Ok(())
    }
    pub fn result(&mut self, n: usize, value: Value) {
        let prefix = if matches!(value, Value::Text(_)) {
            "resultstr"
        } else {
            "resultint"
        };
        self.globals.insert(format!("{prefix}[{n}]"), value);
    }
    pub fn result_text(&mut self, n: usize, value: impl Into<String>) {
        self.globals
            .insert(format!("resultstr[{n}]"), Value::Text(value.into()));
    }
    pub fn cal(&mut self, source: &str) -> Result<()> {
        for assignment in program::arguments(source)? {
            if let Some(name) = assignment.strip_suffix("++") {
                let n = self.eval(name)?.integer()?.wrapping_add(1);
                self.assign(name, Value::Int(n))?;
            } else if let Some(name) = assignment.strip_suffix("--") {
                let n = self.eval(name)?.integer()?.wrapping_sub(1);
                self.assign(name, Value::Int(n))?;
            } else {
                let Some((name, expr)) = assignment.split_once('=') else {
                    let value = self.eval(&assignment)?;
                    self.result(0, value);
                    continue;
                };
                let value = self.eval(expr)?;
                self.assign(name, value)?;
            }
        }
        Ok(())
    }
    fn declare(&mut self, op: &str, args: &[String]) -> Result<()> {
        let kind = args
            .first()
            .context("QLIE variable type")?
            .to_ascii_lowercase();
        ensure!(
            [
                "int", "str", "obj", "list", "table", "img", "btn", "theora", "msg", "cam", "snd",
                "null", "bar", "timer", "simg", "sbtn"
            ]
            .contains(&kind.as_str()),
            "unsupported QLIE variable type {kind}"
        );
        let global = op != "var";
        for declaration in &args[1..] {
            let (name, count) = if let Some((base, tail)) = declaration.split_once('[') {
                let count = self
                    .eval(tail.strip_suffix(']').context("QLIE array declaration")?)?
                    .integer()?;
                ensure!((0..=65536).contains(&count), "QLIE array size");
                (base.to_ascii_lowercase(), Some(count as usize))
            } else {
                (declaration.to_ascii_lowercase(), None)
            };
            if let Some(n) = count {
                if global {
                    self.arrays.insert(name.clone(), n);
                } else {
                    self.frame.arrays.insert(name.clone(), n);
                }
            }
            for i in 0..count.unwrap_or(1) {
                let key = if count.is_some() {
                    format!("{name}[{i}]")
                } else {
                    name.clone()
                };
                if (if global {
                    &self.globals
                } else {
                    &self.frame.locals
                })
                .contains_key(&key)
                {
                    continue;
                }
                ensure!(
                    self.globals.len() + self.frame.locals.len() < 131072,
                    "QLIE variable count limit"
                );
                let value = match kind.as_str() {
                    "int" => Value::Int(0),
                    "str" | "obj" => Value::Text(String::new()),
                    _ => {
                        self.next_object = self
                            .next_object
                            .checked_add(1)
                            .context("QLIE object identity exhausted")?;
                        let id = format!("object/{}", self.next_object);
                        self.object_types.insert(id.clone(), kind.clone());
                        Value::Text(id)
                    }
                };
                if global {
                    self.globals.insert(key, value);
                } else {
                    self.frame.locals.insert(key, value);
                }
            }
        }
        Ok(())
    }
    pub fn resolve_file(&self, file: &str) -> Result<String> {
        let direct = program::canonical(file)?;
        if self.programs.contains_key(&direct) {
            return Ok(direct);
        }
        let prefix = self
            .frame
            .path
            .rsplit_once('/')
            .map(|(p, _)| p)
            .unwrap_or("");
        let relative = program::canonical(&format!("{prefix}/{file}"))?;
        ensure!(
            self.programs.contains_key(&relative),
            "QLIE script module absent: {file}"
        );
        Ok(relative)
    }
    fn find_label(
        &self,
        path: &str,
        label: &str,
        seen: &mut BTreeSet<String>,
    ) -> Result<Option<(String, usize)>> {
        if !seen.insert(path.into()) {
            return Ok(None);
        }
        ensure!(seen.len() <= 1024, "QLIE include graph limit");
        let p = self
            .programs
            .get(path)
            .context("QLIE included module absent")?;
        if let Some(pc) = p.labels.get(&label.to_ascii_lowercase()) {
            return Ok(Some((path.into(), *pc)));
        }
        for inc in &p.includes {
            let root = program::canonical(inc)?;
            let relative = program::canonical(&format!(
                "{}/{}",
                path.rsplit_once('/').map(|x| x.0).unwrap_or(""),
                inc
            ))?;
            let resolved = if self.programs.contains_key(&root) {
                root
            } else {
                relative
            };
            if let Some(found) = self.find_label(&resolved, label, seen)? {
                return Ok(Some(found));
            }
        }
        Ok(None)
    }
    pub fn jump(
        &mut self,
        label: &str,
        file: Option<&str>,
        call: Option<Vec<Value>>,
        clear: bool,
    ) -> Result<()> {
        let path = if let Some(file) = file.filter(|s| !s.is_empty() && *s != "self") {
            self.resolve_file(file)?
        } else {
            self.frame.path.clone()
        };
        let label = if label.is_empty() { "@@top" } else { label };
        let mut seen = BTreeSet::new();
        let p = &self.programs[&path];
        let mut found = if p.inline_includes
            && label.eq_ignore_ascii_case("@@top")
            && !p.labels.contains_key("@@top")
        {
            Some((path.clone(), 0))
        } else {
            self.find_label(&path, label, &mut seen)?
        };
        // Library routines imported by one script share that script's include
        // scope. A routine can call a sibling module without importing it again.
        // Explicit file-qualified calls remain confined to that selected file.
        if found.is_none() && file.is_none_or(|s| s.is_empty() || s == "self") {
            for caller in self.stack.iter().rev() {
                found = self.find_label(&caller.path, label, &mut seen)?;
                if found.is_some() {
                    break;
                }
            }
        }
        let (path, pc) = found.with_context(|| format!("QLIE label not found {path}:{label}"))?;
        if let Some(parameters) = call {
            ensure!(self.stack.len() < 256, "QLIE call stack limit");
            let mut frame = Frame {
                path,
                pc,
                ..Default::default()
            };
            frame
                .locals
                .insert("paramcount".into(), Value::Int(parameters.len() as i32));
            for (i, value) in parameters.into_iter().enumerate() {
                frame
                    .locals
                    .insert(format!("paramstr[{i}]"), Value::Text(value.text()));
                frame.locals.insert(
                    format!("paramint[{i}]"),
                    Value::Int(value.integer().unwrap_or(0)),
                );
            }
            self.frame.pc += 1;
            self.stack.push(std::mem::replace(&mut self.frame, frame));
        } else {
            if clear {
                self.stack.clear();
                self.frame.locals.clear();
                self.frame.arrays.clear();
            }
            self.frame.path = path;
            self.frame.pc = pc;
            self.frame.loops.clear();
            self.frame.cases.clear();
        }
        Ok(())
    }
    pub fn return_call(&mut self) -> Result<bool> {
        if let Some(mut frame) = self.stack.pop() {
            if self.frame.inline {
                frame.locals = std::mem::take(&mut self.frame.locals);
                frame.arrays = std::mem::take(&mut self.frame.arrays);
            }
            if self.transient.remove(&self.frame.path) {
                self.programs.remove(&self.frame.path);
            }
            self.frame = frame;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    /// true: native interpreter consumed the instruction. false: pending syscall.
    pub fn control(&mut self, ins: &Instruction) -> Result<bool> {
        let a = &ins.args;
        match ins.op.as_str() {
            "var" | "gvar" | "svar" => self.declare(&ins.op, a)?,
            "init" => {
                for name in a {
                    let key = self.key(name)?;
                    let count = self
                        .frame
                        .arrays
                        .get(&key)
                        .or_else(|| self.arrays.get(&key))
                        .copied();
                    for i in 0..count.unwrap_or(1) {
                        let key = if count.is_some() {
                            format!("{key}[{i}]")
                        } else {
                            key.clone()
                        };
                        let value = self.value(&key)?;
                        ensure!(
                            !matches!(&value,Value::Text(id) if self.object_types.contains_key(id)),
                            "QLIE object init requires its semantic method"
                        );
                        self.assign(
                            &key,
                            if matches!(value, Value::Text(_)) {
                                Value::Text(String::new())
                            } else {
                                Value::Int(0)
                            },
                        )?;
                    }
                }
            }
            "cal" => self.cal(&a.join(","))?,
            "include" => {
                let path = self.resolve_file(a.first().context("QLIE inline module")?)?;
                ensure!(self.stack.len() < 256, "QLIE inline include stack limit");
                let frame = Frame {
                    path,
                    pc: 0,
                    locals: self.frame.locals.clone(),
                    arrays: self.frame.arrays.clone(),
                    inline: true,
                    ..Default::default()
                };
                self.frame.pc += 1;
                self.stack.push(std::mem::replace(&mut self.frame, frame));
                return Ok(true);
            }
            "if" => {
                if !self.eval(a.first().context("QLIE if condition")?)?.truth() {
                    self.frame.pc = ins.alternate.or(ins.end).context("QLIE if target")?;
                    return Ok(true);
                }
            }
            "else" => {
                let end = ins.end.context("QLIE else target")?;
                if self
                    .frame
                    .cases
                    .last()
                    .is_some_and(|(_, _, case_end)| *case_end == end)
                {
                    if self.frame.cases.last().unwrap().1 {
                        self.frame.cases.pop();
                        self.frame.pc = end;
                        return Ok(true);
                    }
                    self.frame.cases.last_mut().unwrap().1 = true;
                } else {
                    self.frame.pc = end;
                    return Ok(true);
                }
            }
            "for" => {
                ensure!(a.len() == 3, "QLIE for ABI");
                if self.frame.loops.insert(self.frame.pc) {
                    self.cal(&a[0])?;
                }
                if !self.eval(&a[1])?.truth() {
                    self.frame.loops.remove(&self.frame.pc);
                    self.frame.pc = ins.end.context("QLIE for end")?;
                    return Ok(true);
                }
            }
            "end" => {
                let start = ins.end.context("QLIE block start")?;
                let begin = &self.programs[&self.frame.path].code[start];
                if begin.op == "for" {
                    let increment = begin.args[2].clone();
                    self.cal(&increment)?;
                    self.frame.pc = start;
                    return Ok(true);
                } else if begin.op == "case" {
                    self.frame.cases.pop();
                }
            }
            "brk" | "cnt" => {
                let start = self
                    .frame
                    .loops
                    .iter()
                    .rev()
                    .find(|&&start| start < self.frame.pc)
                    .copied()
                    .context("QLIE loop control outside for")?;
                let begin = &self.programs[&self.frame.path].code[start];
                let end = begin.end.context("QLIE loop end")?;
                if ins.op == "brk" {
                    self.frame.loops.remove(&start);
                    self.frame.pc = end;
                } else {
                    self.frame.pc = end - 1;
                }
                return Ok(true);
            }
            "case" => {
                let value = self.eval(a.first().context("QLIE case expression")?)?;
                self.frame
                    .cases
                    .push((value, false, ins.end.context("QLIE case end")?));
            }
            "of" | "ans" => {
                let (value, matched, end) = self
                    .frame
                    .cases
                    .last()
                    .cloned()
                    .context("QLIE case branch outside case")?;
                if matched {
                    self.frame.cases.pop();
                    self.frame.pc = end;
                    return Ok(true);
                }
                let matches = a
                    .iter()
                    .map(|arg| self.eval(arg))
                    .collect::<Result<Vec<_>>>()?
                    .contains(&value);
                if matches {
                    self.frame.cases.last_mut().unwrap().1 = true;
                } else {
                    let p = &self.programs[&self.frame.path];
                    let mut depth = 0;
                    let mut target = end - 1;
                    for pc in self.frame.pc + 1..end - 1 {
                        let op = p.code[pc].op.as_str();
                        if matches!(op, "if" | "for" | "case") {
                            depth += 1;
                        } else if op == "end" {
                            depth -= 1;
                        } else if depth == 0 && matches!(op, "of" | "ans" | "else") {
                            target = pc;
                            break;
                        }
                    }
                    self.frame.pc = target;
                    return Ok(true);
                }
            }
            "sub" | "subp" | "jmp" | "go" => {
                ensure!(!a.is_empty(), "QLIE jump arguments");
                let label = self.eval(&a[0])?.text();
                let file = a
                    .get(1)
                    .filter(|s| !s.is_empty())
                    .map(|s| self.eval(s).map(|v| v.text()))
                    .transpose()?;
                let call = if ins.op == "subp" {
                    Some(if a.len() > 2 {
                        vec![self.call_parameter(
                            &a.iter().skip(2).cloned().collect::<Vec<_>>().join(","),
                        )?]
                    } else {
                        vec![]
                    })
                } else if ins.op == "sub" {
                    Some(
                        a.iter()
                            .skip(2)
                            .map(|s| self.call_parameter(s))
                            .collect::<Result<_>>()?,
                    )
                } else {
                    None
                };
                self.jump(&label, file.as_deref(), call, ins.op == "go")?;
                return Ok(true);
            }
            "ret" => {
                ensure!(self.return_call()?, "QLIE ret outside subroutine");
                return Ok(true);
            }
            "vexs" => {
                let key = self.key(a.first().context("QLIE vexs argument")?)?;
                self.result(
                    0,
                    Value::Int(i32::from(
                        self.frame.locals.contains_key(&key)
                            || self.globals.contains_key(&key)
                            || self.arrays.contains_key(&key)
                            || self.frame.arrays.contains_key(&key),
                    )),
                );
            }
            "garr" | "sarr" => {
                let key = a.first().context("QLIE array query")?.to_ascii_lowercase();
                self.result(
                    0,
                    Value::Int(
                        *self
                            .frame
                            .arrays
                            .get(&key)
                            .or_else(|| self.arrays.get(&key))
                            .context("QLIE array not declared")? as i32,
                    ),
                );
            }
            _ => return Ok(false),
        }
        self.frame.pc += 1;
        self.executed += 1;
        Ok(true)
    }
}
