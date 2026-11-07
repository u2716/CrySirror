use crate::{
    expression::{self, Value},
    program::{self, Instruction, Program},
};
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
mod arrays;
mod banks;
mod checkpoint;
mod context;
pub use context::{ExecutionContext, ThreadKind};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Frame {
    pub path: String,
    pub pc: usize,
    ///Native reader file remains stable while imported library code executes.
    #[serde(default)]
    pub reader_path: String,
    #[serde(default)]
    pub call_label: String,
    ///6E98CC records the caller's active label in the return block; named
    ///ret searches this destination, not the called routine's entry label.
    #[serde(default)]
    pub return_label: String,
    ///Original subr captures only the native result banks, not all globals.
    #[serde(default)]
    pub call_results: Option<BTreeMap<String, Value>>,
    ///Native routine+12 owns its16 selected objects independently of calls.
    #[serde(default)]
    pub selection: Vec<String>,
    pub locals: BTreeMap<String, Value>,
    pub loops: BTreeSet<usize>,
    pub cases: Vec<(Value, bool, usize)>,
    pub inline: bool,
    pub arrays: BTreeMap<String, usize>,
    pub array_types: BTreeMap<String, String>,
    #[serde(default)]
    pub array_shapes: BTreeMap<String, Vec<usize>>,
    pub thread: ThreadKind,
    pub source_thread: Option<ThreadKind>,
    /// Cross-thread calls restore the invoking thread's selected object.
    pub return_scope: Option<String>,
}
pub struct Machine {
    pub programs: BTreeMap<String, Arc<Program>>,
    pub frame: Frame,
    pub stack: Vec<Frame>,
    pub globals: BTreeMap<String, Value>,
    pub object_types: BTreeMap<String, String>,
    pub arrays: BTreeMap<String, usize>,
    pub array_types: BTreeMap<String, String>,
    pub array_shapes: BTreeMap<String, Vec<usize>>,
    system_bank: banks::ValueBank,
    shared_bank: banks::ValueBank,
    initial_bank: banks::ValueBank,
    pub scope: String,
    pub executed: u64,
    next_object: u32,
    transient: BTreeSet<String>,
    next_transient: u64,
    suspended_game: Option<ExecutionContext>,
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
                reader_path: path.clone(),
                call_label: "@@top".into(),
                path,
                pc,
                ..Default::default()
            },
            stack: Vec::new(),
            globals: BTreeMap::new(),
            object_types: BTreeMap::new(),
            arrays: BTreeMap::new(),
            array_types: BTreeMap::new(),
            array_shapes: BTreeMap::new(),
            system_bank: Default::default(),
            shared_bank: Default::default(),
            initial_bank: Default::default(),
            scope: String::new(),
            executed: 0,
            next_object: 0,
            transient: BTreeSet::new(),
            next_transient: 0,
            suspended_game: None,
        };
        //6E460F..6E4738: general banks have16 slots, callback/subroutine
        //banks4. These extents are observable through native garr/sarr.
        for (prefix, count) in [
            ("resultint", 16),
            ("resultstr", 16),
            ("resultobj", 16),
            ("resultbtnstr", 4),
            ("resultbtnint", 4),
            ("resultsubstr", 4),
            ("resultsubint", 4),
        ] {
            m.arrays.insert(prefix.into(), count);
            m.array_types.insert(
                prefix.into(),
                if prefix.ends_with("str") {
                    "str"
                } else if prefix == "resultobj" {
                    "obj"
                } else {
                    "int"
                }
                .into(),
            );
            for n in 0..count {
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
            "misc", "math", "cam", "camex", "dlgs", "tdat", "path",
        ] {
            m.globals
                .insert(builtin.into(), Value::Text(builtin.into()));
            m.object_types.insert(builtin.into(), builtin.into());
        }
        for prefix in ["snd", "dsnd", "ssnd"] {
            m.arrays.insert(prefix.into(), 64);
            m.array_types.insert(prefix.into(), "snd".into());
            for n in 0..64 {
                let key = format!("{prefix}[{n}]");
                m.globals.insert(key.clone(), Value::Text(key.clone()));
                m.object_types.insert(key, "snd".into());
            }
        }
        for name in context::CONTEXT_BUILTINS {
            m.object_types
                .insert(format!("system/{name}"), (*name).into());
        }
        m.globals.insert("resultbtnobj".into(), Value::Int(0));
        m.globals.insert("resultstopint".into(), Value::Int(0));
        m.globals.insert("resultclkint".into(), Value::Int(0));
        m.globals.insert("resultbtn".into(), Value::Int(0));
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
            //6E4E0A..6E4F05 initialize the public names, independently of
            //the numeric animation-key dispatch at5CFB7C.
            ("_mmStep", 0),
            ("_mmLinear", 1),
            ("_mmOutQuart", 3),
            ("_mmInQuart", 4),
            ("_mmOutInQuart", 7),
            ("_mmInOutQuart", 8),
            ("_mmSquare", 12),
            ("_mmTriangle", 13),
            ("_mmSin", 14),
            ("_mmOutElastic", 15),
            ("_mmInElastic", 16),
            ("_mmOutBack", 17),
            ("_mmInBack", 18),
            ("_mmOutBounce", 19),
            ("_mmInBounce", 20),
            ("_mmDivide", 21),
        ] {
            m.globals
                .insert(name.to_ascii_lowercase(), Value::Int(value));
        }
        // Each native calculator registers its own builtins before System's
        // shared (svar) bank is linked to Game's; gvar remains per-thread.
        m.system_bank = banks::ValueBank {
            values: m.globals.clone(),
            arrays: m.arrays.clone(),
            array_types: m.array_types.clone(),
            array_shapes: m.array_shapes.clone(),
        };
        m.initial_bank = m.system_bank.clone();
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
        let name = self.indexed_key(&self.frame, &name.to_ascii_lowercase())?;
        if let Some(value) = self.frame.locals.get(&name) {
            return Ok(value.clone());
        }
        if self.frame.arrays.get(&name).is_some_and(|count| *count > 0) {
            if let Some(value) = self.frame.locals.get(&format!("{name}[0]")) {
                return Ok(value.clone());
            }
        }
        if let Some(id) = Self::context_builtin(&name, self.source_thread_kind()) {
            return Ok(Value::Text(id));
        }
        if let Some(value) = self.global_value(&name) {
            return Ok(value.clone());
        }
        if self
            .global_array_extent(&name)
            .is_some_and(|count| count > 0)
        {
            if let Some(value) = self.global_value(&format!("{name}[0]")) {
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
            "self" => Ok(Value::Text(self.reader_path().to_owned())),
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
                    Self::frame_reader_path(frame)
                        .rsplit_once('/')
                        .map(|(path, _)| format!("{path}/"))
                        .unwrap_or_default()
                } else {
                    Self::frame_reader_path(frame).to_owned()
                }))
            }
            "selfthreadname" => Ok(Value::Text(self.thread_kind().native_name().into())),
            _ => bail!("undefined QLIE variable {name}"),
        }
    }
    pub fn eval(&self, s: &str) -> Result<Value> {
        if s.trim().is_empty() {
            return Ok(Value::Text(String::new()));
        }
        expression::evaluate(s, |name| self.value(name))
    }
    pub(crate) fn call_parameter(&self, s: &str) -> Result<Value> {
        // Native call parameters enable plain-string mode in Calculation.
        // Defined variables and expressions still evaluate normally. A bare
        // literal stays a raw token (6DE63C type 4): ParamStr keeps its text
        // ("00"), ParamInt its StrToIntDef value.
        let token = s.trim();
        if !token.is_empty()
            && (token.bytes().all(|b| b.is_ascii_digit())
                || token.strip_prefix('$').is_some_and(|h| !h.is_empty() && h.bytes().all(|b| b.is_ascii_hexdigit())))
        {
            return Ok(Value::Text(token.to_owned()));
        }
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
        let mut program = Program::parse_inline(&name, source)?;
        program.labels.insert("@@top".into(), 0);
        program.includes.push(self.frame.path.clone());
        for ins in &mut program.code {
            ins.preprocess = false;
        }
        let reader_path = self.reader_path().to_owned();
        let selection = self.frame.selection.clone();
        let call_label = self.frame.call_label.clone();
        let arrays = self.frame.arrays.clone();
        let array_types = self.frame.array_types.clone();
        let array_shapes = self.frame.array_shapes.clone();
        let mut locals = self.frame.locals.clone();
        locals.extend(bindings);
        self.programs.insert(name.clone(), Arc::new(program));
        self.transient.insert(name.clone());
        self.jump("@@top", Some(&name), Some(vec![]), false)?;
        self.frame.locals = locals;
        self.frame.reader_path = reader_path;
        self.frame.selection = selection;
        self.frame.call_label = call_label;
        self.scope = self.frame.selection.first().cloned().unwrap_or_default();
        self.frame.arrays = arrays;
        self.frame.array_types = array_types;
        self.frame.array_shapes = array_shapes;
        self.frame.inline = true;
        Ok(())
    }
    pub fn key(&self, s: &str) -> Result<String> {
        let s = s.trim();
        if let Some((name, tail)) = s.split_once('[') {
            let index = tail.strip_suffix(']').context("QLIE array name")?;
            let indices = program::arguments(index)?
                .iter()
                .map(|part| {
                    if part.is_empty() {
                        Ok("0".into())
                    } else {
                        self.eval(part)?.integer().map(|v| v.to_string())
                    }
                })
                .collect::<Result<Vec<_>>>()?;
            self.indexed_key(
                &self.frame,
                &format!("{}[{}]", name.to_ascii_lowercase(), indices.join(",")),
            )
        } else {
            Ok(s.to_ascii_lowercase())
        }
    }
    /// Native calculator declarations have a first element even when their
    /// source omitted array syntax. `scalar[0]` addresses that same cell; it
    /// must not create a new array or bypass an actual local array binding.
    fn canonical_binding_key(&self, frame: &Frame, name: &str) -> String {
        let Some(base) = name.strip_suffix("[0]") else {
            return name.into();
        };
        if frame.locals.contains_key(name) {
            return name.into();
        }
        if frame.locals.contains_key(base) {
            return base.into();
        }
        if frame.arrays.contains_key(base) {
            return name.into();
        }
        if self.global_value_for(frame.thread, name).is_some() {
            return name.into();
        }
        if self.global_value_for(frame.thread, base).is_some() {
            return base.into();
        }
        name.into()
    }
    pub fn assign(&mut self, name: &str, value: Value) -> Result<()> {
        let mut key = self.key(name)?;
        if !self.frame.locals.contains_key(&key)
            && (self.frame.arrays.get(&key).is_some_and(|count| *count > 0)
                || (!self.global_contains(&key)
                    && self
                        .global_array_extent(&key)
                        .is_some_and(|count| count > 0)))
        {
            key = format!("{key}[0]");
        }
        ensure!(
            self.frame.locals.contains_key(&key) || self.global_contains(&key),
            "assignment to undefined QLIE variable {key}"
        );
        if self.frame.locals.contains_key(&key) {
            self.frame.locals.insert(key, value);
        } else {
            self.set_global_value(key, value);
        }
        Ok(())
    }
    pub fn result(&mut self, n: usize, value: Value) {
        let prefix = if matches!(value, Value::Text(_)) {
            "resultstr"
        } else {
            "resultint"
        };
        self.thread_globals_mut()
            .insert(format!("{prefix}[{n}]"), value);
    }
    pub fn result_text(&mut self, n: usize, value: impl Into<String>) {
        self.thread_globals_mut()
            .insert(format!("resultstr[{n}]"), Value::Text(value.into()));
    }
    pub fn cal(&mut self, source: &str) -> Result<()> {
        for assignment in program::arguments(source)? {
            if assignment.trim().is_empty() {
                continue;
            }
            if let Some(name) = assignment.strip_suffix("++") {
                let n = self.eval(name)?.integer()?.wrapping_add(1);
                self.assign(name, Value::Int(n))?;
            } else if let Some(name) = assignment.strip_suffix("--") {
                let n = self.eval(name)?.integer()?.wrapping_sub(1);
                self.assign(name, Value::Int(n))?;
            } else {
                let Some((name, expr)) = assignment_parts(&assignment) else {
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
                //6E6AD0's original built-in factory is also available to
                //declarations, not just startup's pre-created managers.
                "int", "str", "obj", "null", "img", "msg", "btn", "bar", "simg", "sbtn", "file",
                "list", "table", "theora", "timer", "sys", "dat", "sdat", "snd", "dsnd", "eff",
                "scsr", "csr", "log", "cam", "camex", "rep", "path", "mdat", "tdat", "slog",
                "flog", "math", "cfg", "misc", "inp", "skey", "clip", "dlgs", "reg", "twitter",
                "ppngpool", "ext"
            ]
            .contains(&kind.as_str()),
            "unsupported QLIE variable type {kind}"
        );
        let global = op != "var";
        for declaration in &args[1..] {
            let (name, count) = if let Some((base, tail)) = declaration.split_once('[') {
                let shape = self
                    .eval_array_shape(tail.strip_suffix(']').context("QLIE array declaration")?)?;
                let count = Self::array_element_count(&shape)?;
                let name = base.to_ascii_lowercase();
                if op == "var" {
                    self.frame.array_shapes.insert(name.clone(), shape);
                } else if op == "svar" {
                    self.shared_bank.array_shapes.insert(name.clone(), shape);
                } else if self.thread_kind() == ThreadKind::System {
                    self.system_bank.array_shapes.insert(name.clone(), shape);
                } else {
                    self.array_shapes.insert(name.clone(), shape);
                }
                (name, Some(count))
            } else {
                (declaration.to_ascii_lowercase(), None)
            };
            if let Some(n) = count {
                if global {
                    if op == "svar" {
                        self.shared_bank.arrays.insert(name.clone(), n);
                        self.shared_bank
                            .array_types
                            .insert(name.clone(), kind.clone());
                    } else if self.thread_kind() == ThreadKind::System {
                        self.system_bank.arrays.insert(name.clone(), n);
                        self.system_bank
                            .array_types
                            .insert(name.clone(), kind.clone());
                    } else {
                        self.arrays.insert(name.clone(), n);
                        self.array_types.insert(name.clone(), kind.clone());
                    }
                } else {
                    self.frame.arrays.insert(name.clone(), n);
                    self.frame.array_types.insert(name.clone(), kind.clone());
                }
            }
            for i in 0..count.unwrap_or(1) {
                let key = if count.is_some() {
                    format!("{name}[{i}]")
                } else {
                    name.clone()
                };
                if (if global {
                    if op == "svar" {
                        &self.shared_bank.values
                    } else {
                        self.thread_globals()
                    }
                } else {
                    &self.frame.locals
                })
                .contains_key(&key)
                {
                    continue;
                }
                ensure!(
                    self.globals.len()
                        + self.system_bank.values.len()
                        + self.shared_bank.values.len()
                        + self.frame.locals.len()
                        < 131072,
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
                    if op == "svar" {
                        self.shared_bank.values.insert(key, value);
                    } else {
                        self.thread_globals_mut().insert(key, value);
                    }
                } else {
                    self.frame.locals.insert(key, value);
                }
            }
        }
        Ok(())
    }
    pub fn frame_reader_path(frame: &Frame) -> &str {
        if frame.reader_path.is_empty() {
            &frame.path
        } else {
            &frame.reader_path
        }
    }
    pub fn reader_path(&self) -> &str {
        Self::frame_reader_path(&self.frame)
    }
    /// 703B9C TImoPath self-path callback: ExtractFilePath of the reader's
    /// current file (6BC87C). Jumping into an included library label seeks
    /// the reader to that file (6BC830), so this is the executing code's file
    /// rather than the file that first opened the reader.
    pub fn source_code_path(&self) -> &str {
        if self.thread_kind() == ThreadKind::System && self.source_thread_kind() == ThreadKind::Game
        {
            if let Some(game) = &self.suspended_game {
                return &game.frame.path;
            }
        }
        &self.frame.path
    }
    pub fn resolve_file(&self, file: &str) -> Result<String> {
        let direct = program::canonical(file)?;
        if self.programs.contains_key(&direct) {
            return Ok(direct);
        }
        let prefix = self
            .reader_path()
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
    ///6D648C lexs queries the source reader's label/include tree without
    ///advancing the executing reader or creating a call frame.
    pub fn source_label_exists(&self, label: &str) -> Result<bool> {
        let (frame, stack) = if self.source_thread_kind() == ThreadKind::Game
            && self.thread_kind() == ThreadKind::System
        {
            let game = self
                .suspended_game
                .as_ref()
                .context("QLIE source Game reader absent")?;
            (&game.frame, &game.stack)
        } else {
            (&self.frame, &self.stack)
        };
        let mut seen = BTreeSet::new();
        if self
            .find_label(Self::frame_reader_path(frame), label, &mut seen)?
            .is_some()
        {
            return Ok(true);
        }
        for caller in stack.iter().rev() {
            if self
                .find_label(Self::frame_reader_path(caller), label, &mut seen)?
                .is_some()
            {
                return Ok(true);
            }
        }
        Ok(false)
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
        } else if self.frame.inline && self.transient.contains(&self.frame.path) {
            self.frame.path.clone()
        } else {
            self.reader_path().to_owned()
        };
        let reader_path = if file.is_some_and(|file| !file.is_empty() && file != "self") {
            path.clone()
        } else {
            self.reader_path().to_owned()
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
                found = self.find_label(Self::frame_reader_path(caller), label, &mut seen)?;
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
                reader_path,
                call_label: label.to_ascii_lowercase(),
                return_label: self.frame.call_label.clone(),
                thread: self.thread_kind(),
                source_thread: self.frame.source_thread,
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
            self.scope.clear();
        } else {
            if clear {
                self.stack.clear();
                self.frame.locals.clear();
                self.frame.arrays.clear();
                self.frame.array_types.clear();
                self.frame.array_shapes.clear();
                self.frame.loops.clear();
                self.frame.cases.clear();
                self.frame.inline = false;
                self.frame.return_scope = None;
                self.frame.selection.clear();
                self.scope.clear();
            } else if self.frame.path != path {
                // Reader-local block addresses cannot survive a non-call
                // transfer to another script's instruction array.
                self.frame.loops.clear();
                self.frame.cases.clear();
            }
            self.frame.path = path;
            self.frame.reader_path = reader_path;
            self.frame.call_label = label.to_ascii_lowercase();
            if clear {
                self.frame.call_results = None;
                self.frame.return_label.clear();
            }
            self.frame.pc = pc;
            self.frame.loops.clear();
            self.frame.cases.clear();
        }
        Ok(())
    }
    pub(crate) fn capture_call_results(&self) -> BTreeMap<String, Value> {
        let mut result = BTreeMap::new();
        for (prefix, count) in [
            ("resultint", 16),
            ("resultstr", 16),
            ("resultobj", 16),
            ("resultbtnint", 4),
            ("resultbtnstr", 4),
            ("resultsubint", 4),
            ("resultsubstr", 4),
        ] {
            for index in 0..count {
                let key = format!("{prefix}[{index}]");
                if let Some(value) = self.global_value_for(self.thread_kind(), &key) {
                    result.insert(key, value.clone());
                }
            }
        }
        for name in ["resultbtnobj", "resultclkint", "resultstopint"] {
            if let Some(value) = self.global_value_for(self.thread_kind(), name) {
                result.insert(name.into(), value.clone());
            }
        }
        result
    }
    pub fn return_named(&mut self, label: Option<&str>) -> Result<bool> {
        let Some(label) = label.filter(|label| !label.is_empty()) else {
            //Native inline command strings/includes do not create routines.
            //An explicit ret exits their containing native call as a whole.
            while self.frame.inline && !self.stack.is_empty() {
                self.return_frame(false)?;
            }
            return self.return_call();
        };
        let current = self.thread_kind();
        let target = self.stack.iter().enumerate().rev();
        let target = std::iter::once((self.stack.len(), &self.frame))
            .chain(target)
            .take_while(|(_, frame)| frame.thread == current)
            .find(|(_, frame)| !frame.inline && frame.return_label.eq_ignore_ascii_case(label))
            .map(|(index, _)| index)
            .context("QLIE named return routine absent")?;
        ensure!(target > 0, "QLIE named return outside subroutine");
        while self.stack.len() > target {
            self.return_frame(false)?;
        }
        self.return_frame(true)
    }
    pub fn return_call(&mut self) -> Result<bool> {
        self.return_frame(true)
    }
    fn return_frame(&mut self, restore_results: bool) -> Result<bool> {
        if let Some(mut frame) = self.stack.pop() {
            if restore_results {
                if let Some(results) = self.frame.call_results.take() {
                    self.thread_globals_mut().extend(results);
                }
            }
            let return_scope = self.frame.return_scope.take();
            if self.frame.inline {
                frame.locals = std::mem::take(&mut self.frame.locals);
                frame.arrays = std::mem::take(&mut self.frame.arrays);
                frame.array_types = std::mem::take(&mut self.frame.array_types);
                frame.array_shapes = std::mem::take(&mut self.frame.array_shapes);
                frame.selection = std::mem::take(&mut self.frame.selection);
            }
            self.scope = return_scope
                .unwrap_or_else(|| frame.selection.first().cloned().unwrap_or_default());
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
        // A jmp can leave a lexical block without visiting its end opcode.
        // Discard its runtime control record before another for/brk/case
        // observes it, while retaining records at their own loop header.
        let pc = self.frame.pc;
        let program = self
            .programs
            .get(&self.frame.path)
            .context("QLIE control program")?;
        self.frame.loops.retain(|start| {
            *start <= pc
                && program
                    .code
                    .get(*start)
                    .and_then(|ins| ins.end)
                    .is_some_and(|end| pc < end)
        });
        self.frame.cases.retain(|(_, _, end)| {
            program
                .code
                .get(end.saturating_sub(1))
                .and_then(|ins| ins.end)
                .is_some_and(|start| start <= pc && pc < *end)
        });
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
                        .or_else(|| self.global_arrays().get(&key))
                        .or_else(|| self.shared_bank.arrays.get(&key))
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
            "vget" | "vset" => {
                ensure!(a.len() == 2, "QLIE cross-thread value ABI");
                let (destination, source) = if ins.op == "vget" {
                    (
                        self.value(&self.key(&a[0])?)?,
                        self.game_value(&self.game_key(&a[1])?)?,
                    )
                } else {
                    (self.game_value(&self.game_key(&a[0])?)?, self.eval(&a[1])?)
                };
                if [&destination, &source]
                    .iter()
                    .any(|v| matches!(v,Value::Text(id) if self.object_types.contains_key(id)))
                {
                    return Ok(false);
                }
                ensure!(
                    std::mem::discriminant(&destination) == std::mem::discriminant(&source),
                    "QLIE cross-thread value type mismatch"
                );
                if ins.op == "vget" {
                    self.assign(&a[0], source)?;
                } else {
                    self.assign_game(&a[0], source)?;
                }
            }
            "include" => {
                let path = self.resolve_file(a.first().context("QLIE inline module")?)?;
                ensure!(self.stack.len() < 256, "QLIE inline include stack limit");
                let frame = Frame {
                    reader_path: self.reader_path().to_owned(),
                    path,
                    pc: 0,
                    locals: self.frame.locals.clone(),
                    arrays: self.frame.arrays.clone(),
                    array_types: self.frame.array_types.clone(),
                    array_shapes: self.frame.array_shapes.clone(),
                    inline: true,
                    thread: self.thread_kind(),
                    source_thread: self.frame.source_thread,
                    selection: self.frame.selection.clone(),
                    call_label: self.frame.call_label.clone(),
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
                ensure!(a.len() <= 3, "QLIE for ABI");
                if self.frame.loops.insert(self.frame.pc) {
                    if let Some(initial) = a.first() {
                        self.cal(initial)?;
                    }
                }
                if a.get(1).is_some_and(|condition| !condition.is_empty())
                    && !self.eval(&a[1])?.truth()
                {
                    self.frame.loops.remove(&self.frame.pc);
                    self.frame.pc = ins.end.context("QLIE for end")?;
                    return Ok(true);
                }
            }
            "end" => {
                let start = ins.end.context("QLIE block start")?;
                let begin = &self.programs[&self.frame.path].code[start];
                if begin.op == "for" {
                    if let Some(increment) = begin.args.get(2).cloned() {
                        self.cal(&increment)?;
                    }
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
                // cnt/brk can cross nested case blocks without executing
                //their end instruction. Retire only those lexical cases.
                let target = self.frame.pc;
                let code = &self.programs[&self.frame.path].code;
                self.frame.cases.retain(|(_, _, end)| {
                    code.get(end.saturating_sub(1))
                        .and_then(|ins| ins.end)
                        .is_some_and(|start| start <= target && target < *end)
                });
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
            "sub" | "subp" | "subr" | "jmp" | "go" => {
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
                } else if matches!(ins.op.as_str(), "sub" | "subr") {
                    Some(
                        a.iter()
                            .skip(2)
                            .map(|s| self.call_parameter(s))
                            .collect::<Result<_>>()?,
                    )
                } else {
                    None
                };
                let results = (ins.op == "subr").then(|| self.capture_call_results());
                self.jump(&label, file.as_deref(), call, ins.op == "go")?;
                if let Some(results) = results {
                    self.frame.call_results = Some(results);
                }

                return Ok(true);
            }
            "ret" => {
                let label = a
                    .first()
                    .filter(|label| !label.is_empty())
                    .map(|label| self.call_parameter(label).map(|value| value.text()))
                    .transpose()?;
                ensure!(
                    self.return_named(label.as_deref())?,
                    "QLIE ret outside subroutine"
                );
                return Ok(true);
            }
            "vexs" => {
                let key = self.key(a.first().context("QLIE vexs argument")?)?;
                self.result(
                    0,
                    Value::Int(i32::from(
                        self.frame.locals.contains_key(&key)
                            || self.global_contains(&key)
                            || self.global_array_extent(&key).is_some()
                            || self.frame.arrays.contains_key(&key),
                    )),
                );
            }
            "sarr" => {
                for declaration in a {
                    let (base, tail) = declaration
                        .split_once('[')
                        .context("QLIE resize array name")?;
                    let base = base.trim().to_ascii_lowercase();
                    let shape = self.eval_array_shape(
                        tail.strip_suffix(']')
                            .context("QLIE resize array dimension")?,
                    )?;
                    let count = Self::array_element_count(&shape)?;
                    let local = self.frame.arrays.contains_key(&base);
                    let old = *(if local {
                        self.frame.arrays.get(&base)
                    } else {
                        self.global_arrays()
                            .get(&base)
                            .or_else(|| self.shared_bank.arrays.get(&base))
                    })
                    .context("QLIE resize undeclared array")?;
                    let kind = (if local {
                        self.frame.array_types.get(&base)
                    } else {
                        self.global_array_kind(&base)
                    })
                    .context("QLIE array element type absent")?
                    .clone();
                    if count > old {
                        ensure!(
                            self.globals.len()
                                + self.system_bank.values.len()
                                + self.shared_bank.values.len()
                                + self.frame.locals.len()
                                + count
                                - old
                                <= 131072,
                            "QLIE variable count limit"
                        );
                        for index in old..count {
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
                            let key = format!("{base}[{index}]");
                            if local {
                                self.frame.locals.insert(key, value);
                            } else {
                                self.set_global_array_value(&base, index, value);
                            }
                        }
                    } else {
                        for index in count..old {
                            let key = format!("{base}[{index}]");
                            if local {
                                self.frame.locals.remove(&key);
                            } else {
                                self.remove_global_value(&key);
                            }
                        }
                    }
                    if local {
                        self.frame.arrays.insert(base.clone(), count);
                        self.frame.array_shapes.insert(base, shape);
                    } else {
                        self.set_global_array_shape(&base, shape);
                        self.set_global_array_extent(base, count);
                    }
                }
            }
            "garr" => {
                ensure!(a.len() == 1, "QLIE array extent query ABI");
                let key = a[0].split('[').next().unwrap().trim().to_ascii_lowercase();
                ensure!(
                    self.frame.locals.contains_key(&key)
                        || self.frame.arrays.contains_key(&key)
                        || self.global_contains(&key)
                        || self.global_array_extent(&key).is_some(),
                    "QLIE array not declared"
                );
                let shape = self
                    .frame_array_shape(&self.frame, &key)
                    .unwrap_or_else(|| vec![1]);
                for index in 0..4 {
                    self.result(
                        index,
                        Value::Int(shape.iter().rev().nth(index).copied().unwrap_or(1) as i32),
                    );
                }
            }
            _ => return Ok(false),
        }
        self.frame.pc += 1;
        self.executed += 1;
        Ok(true)
    }
}

/// Assignment is a single top-level '='. Native cal also accepts ordinary
/// comparisons (the shipped auto-selection script contains ResultInt[0]==0),
/// and '=' inside a quoted string or array expression is never a store.
fn assignment_parts(source: &str) -> Option<(&str, &str)> {
    let mut quoted = false;
    let mut depth = 0usize;
    let bytes = source.as_bytes();
    for (at, c) in source.char_indices() {
        match c {
            '"' => quoted = !quoted,
            '(' | '[' if !quoted => depth += 1,
            ')' | ']' if !quoted => depth = depth.saturating_sub(1),
            '=' if !quoted && depth == 0 => {
                let previous = at.checked_sub(1).and_then(|i| bytes.get(i)).copied();
                if bytes.get(at + 1) == Some(&b'=')
                    || previous.is_some_and(|c| matches!(c, b'=' | b'!' | b'<' | b'>'))
                {
                    continue;
                }
                return Some((&source[..at], &source[at + 1..]));
            }
            _ => {}
        }
    }
    None
}
