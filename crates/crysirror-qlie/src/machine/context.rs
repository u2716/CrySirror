//! Original cooperative GameThread/SystemThread readers. Their call stacks,
//! local declarations, gvar banks and selection scopes are separate. Only
//! svar follows 6E633C's calculator-bank inheritance; see banks.rs.
use super::*;
pub(super) const CONTEXT_BUILTINS: &[&str] =
    &["sys", "eff", "inp", "skey", "cam", "camex", "rep", "path"];
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThreadKind {
    #[default]
    Game,
    System,
}
impl ThreadKind {
    pub fn native_name(self) -> &'static str {
        match self {
            //6E4810..6E4865 map internal reader names to script values.
            Self::Game => "game",
            Self::System => "system",
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionContext {
    pub frame: Frame,
    pub stack: Vec<Frame>,
    pub scope: String,
    pub(super) transient: BTreeSet<String>,
}
impl Machine {
    pub(crate) fn game_value(&self, name: &str) -> Result<Value> {
        if self.thread_kind() == ThreadKind::Game {
            return self.value(name);
        }
        let Some(game) = self.suspended_game.as_ref() else {
            return self.value(name);
        };
        let name = self.indexed_key(&game.frame, &name.to_ascii_lowercase())?;
        if let Some(value) = game.frame.locals.get(&name) {
            return Ok(value.clone());
        }
        if game.frame.arrays.get(&name).is_some_and(|count| *count > 0) {
            if let Some(value) = game.frame.locals.get(&format!("{name}[0]")) {
                return Ok(value.clone());
            }
        }
        if let Some(id) = Self::context_builtin(&name, ThreadKind::Game) {
            return Ok(Value::Text(id));
        }
        if let Some(value) = self.global_value_for(ThreadKind::Game, &name) {
            return Ok(value.clone());
        }
        if self
            .arrays
            .get(&name)
            .or_else(|| self.shared_bank.arrays.get(&name))
            .is_some_and(|count| *count > 0)
        {
            if let Some(value) = self.global_value_for(ThreadKind::Game, &format!("{name}[0]")) {
                return Ok(value.clone());
            }
        }
        bail!("undefined QLIE game variable {name}")
    }
    pub(crate) fn game_key(&self, name: &str) -> Result<String> {
        let name = name.trim();
        if let Some((base, tail)) = name.split_once('[') {
            let indices =
                program::arguments(tail.strip_suffix(']').context("QLIE game array name")?)?
                    .iter()
                    .map(|part| {
                        if part.is_empty() {
                            return Ok("0".to_owned());
                        }
                        expression::evaluate(part, |name| self.game_value(name))?
                            .integer()
                            .map(|v| v.to_string())
                    })
                    .collect::<Result<Vec<_>>>()?;
            let name = format!("{}[{}]", base.to_ascii_lowercase(), indices.join(","));
            let frame = if self.thread_kind() == ThreadKind::System {
                self.suspended_game
                    .as_ref()
                    .map(|v| &v.frame)
                    .unwrap_or(&self.frame)
            } else {
                &self.frame
            };
            self.indexed_key(frame, &name)
        } else {
            Ok(name.to_ascii_lowercase())
        }
    }
    pub(crate) fn assign_game(&mut self, name: &str, value: Value) -> Result<()> {
        if self.thread_kind() == ThreadKind::Game {
            return self.assign(name, value);
        }
        let mut key = self.game_key(name)?;
        let Some(game) = self.suspended_game.as_mut() else {
            return self.assign(name, value);
        };
        if !game.frame.locals.contains_key(&key)
            && (game.frame.arrays.get(&key).is_some_and(|count| *count > 0)
                || (!self.globals.contains_key(&key)
                    && !self.shared_bank.values.contains_key(&key)
                    && self
                        .arrays
                        .get(&key)
                        .or_else(|| self.shared_bank.arrays.get(&key))
                        .is_some_and(|count| *count > 0)))
        {
            key = format!("{key}[0]");
        }
        if game.frame.locals.contains_key(&key) {
            game.frame.locals.insert(key, value);
        } else {
            ensure!(
                self.globals.contains_key(&key) || self.shared_bank.values.contains_key(&key),
                "assignment to undefined QLIE game variable {key}"
            );
            if self.globals.contains_key(&key) {
                self.globals.insert(key, value);
            } else {
                self.shared_bank.values.insert(key, value);
            }
        }
        Ok(())
    }
    pub fn thread_kind(&self) -> ThreadKind {
        self.frame.thread
    }
    pub fn source_thread_kind(&self) -> ThreadKind {
        self.frame
            .source_thread
            .unwrap_or_else(|| self.thread_kind())
    }
    /// srct selects which original thread owns subsequent builtin operations;
    /// the executing reader, parameters and local variables remain unchanged.
    pub fn set_source_thread(&mut self, selector: i32) -> Result<()> {
        self.frame.source_thread = match selector {
            0 => None,
            1 => Some(ThreadKind::Game),
            _ => bail!("unsupported QLIE source-thread selector {selector}"),
        };
        Ok(())
    }
    pub fn context_builtin(name: &str, thread: ThreadKind) -> Option<String> {
        CONTEXT_BUILTINS.contains(&name).then(|| match thread {
            ThreadKind::Game => name.into(),
            ThreadKind::System => format!("system/{name}"),
        })
    }
    pub fn suspended_game(&self) -> Option<&ExecutionContext> {
        self.suspended_game.as_ref()
    }
    /// Every suspended local value remains a GC root. System UI setup must
    /// not collect a game object that only the paused reader can still reach.
    pub fn execution_frames(&self) -> impl Iterator<Item = &Frame> {
        std::iter::once(&self.frame).chain(self.stack.iter()).chain(
            self.suspended_game
                .iter()
                .flat_map(|context| std::iter::once(&context.frame).chain(context.stack.iter())),
        )
    }
    ///70A1D8 resets the system reader and jumps from sysstartup.s. The game
    ///reader resumes after its invoking thrd instruction, without an extra
    ///subroutine frame. True means the host must also suspend its game waits.
    pub fn enter_system(&mut self, label: &str) -> Result<bool> {
        self.enter_system_at(label, true)
    }
    /// Native input actions interrupt the reader between instructions. Unlike
    /// an executed thrd, their saved PC already names the next instruction.
    pub fn enter_system_event(&mut self, label: &str) -> Result<bool> {
        self.enter_system_at(label, false)
    }
    fn enter_system_at(&mut self, label: &str, advance: bool) -> Result<bool> {
        let reader_path = program::canonical("sysstartup.s")?;
        let path = reader_path.clone();
        let label = if label.is_empty() { "@@!Top" } else { label };
        let (path, pc) = self
            .find_label(&path, label, &mut BTreeSet::new())?
            .with_context(|| format!("QLIE system entry not found: {label}"))?;
        let suspend = self.suspended_game.is_none();
        if suspend {
            ensure!(
                self.thread_kind() == ThreadKind::Game,
                "QLIE system entry has no game reader"
            );
            if advance {
                self.frame.pc = self
                    .frame
                    .pc
                    .checked_add(1)
                    .context("QLIE thread resume PC overflow")?;
            }
            let context = ExecutionContext {
                frame: std::mem::take(&mut self.frame),
                stack: std::mem::take(&mut self.stack),
                scope: std::mem::take(&mut self.scope),
                transient: std::mem::take(&mut self.transient),
            };
            self.suspended_game = Some(context);
        } else {
            ensure!(
                self.thread_kind() == ThreadKind::System,
                "QLIE cannot reset system reader from an unfinished cross-thread call"
            );
            self.discard_active_context();
        }
        self.frame = Frame {
            reader_path,
            call_label: label.to_ascii_lowercase(),
            path,
            pc,
            thread: ThreadKind::System,
            ..Default::default()
        };
        Ok(suspend)
    }
    ///70A488 clears the system reader and restores the paused game exactly.
    pub fn resume_game(&mut self) -> Result<()> {
        ensure!(
            self.thread_kind() == ThreadKind::System,
            "QLIE thrd,0 outside SystemThread"
        );
        let context = self
            .suspended_game
            .take()
            .context("QLIE suspended game reader absent")?;
        self.discard_active_context();
        self.frame = context.frame;
        self.stack = context.stack;
        self.scope = context.scope;
        self.transient = context.transient;
        Ok(())
    }
    fn discard_active_context(&mut self) {
        for path in std::mem::take(&mut self.transient) {
            self.programs.remove(&path);
        }
        self.stack.clear();
        self.scope.clear();
    }
    /// A tsub executes a routine in the paused game reader's library scope,
    ///then returns to the invoking SystemThread instruction. It does not
    ///replace either original reader or turn a native thread into a data VM.
    pub fn call_game(
        &mut self,
        label: &str,
        file: Option<&str>,
        parameters: Vec<Value>,
    ) -> Result<()> {
        if self.thread_kind() == ThreadKind::Game {
            return self.jump(label, file, Some(parameters), false);
        }
        let game = self
            .suspended_game
            .as_ref()
            .context("QLIE tsub has no suspended game reader")?;
        let mut seen = BTreeSet::new();
        let explicit = file.filter(|value| !value.is_empty() && *value != "self");
        let selected = if let Some(file) = explicit {
            let direct = program::canonical(file)?;
            let relative = program::canonical(&format!(
                "{}/{}",
                Self::frame_reader_path(&game.frame)
                    .rsplit_once('/')
                    .map(|v| v.0)
                    .unwrap_or(""),
                file
            ))?;
            let path = if self.programs.contains_key(&direct) {
                direct
            } else {
                relative
            };
            self.find_label(&path, label, &mut seen)?
        } else {
            let mut found = None;
            for frame in std::iter::once(&game.frame).chain(game.stack.iter().rev()) {
                found = self.find_label(Self::frame_reader_path(frame), label, &mut seen)?;
                if found.is_some() {
                    break;
                }
            }
            found
        };
        let (path, _) =
            selected.with_context(|| format!("QLIE game routine not found: {label}"))?;
        let reader_path = if let Some(file) = explicit {
            let direct = program::canonical(file)?;
            if self.programs.contains_key(&direct) {
                direct
            } else {
                program::canonical(&format!(
                    "{}/{}",
                    Self::frame_reader_path(&game.frame)
                        .rsplit_once('/')
                        .map(|v| v.0)
                        .unwrap_or(""),
                    file
                ))?
            }
        } else {
            Self::frame_reader_path(&game.frame).to_owned()
        };
        let return_label = game.frame.call_label.clone();
        let game_scope = game.scope.clone();
        let caller_scope = self.scope.clone();
        self.jump(label, Some(&path), Some(parameters), false)?;
        self.frame.thread = ThreadKind::Game;
        self.frame.reader_path = reader_path;
        self.frame.return_label = return_label;
        self.frame.source_thread = None;
        self.frame.return_scope = Some(caller_scope);
        self.scope = game_scope;
        Ok(())
    }
}
