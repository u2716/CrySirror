// SPDX-License-Identifier: MPL-2.0
//! Bounded original FES orchestration VM. Raster/text/UI/audio operations are
//! native pending operations; no renderer, image algorithm or mixer lives here.
use crate::native_interpreter::Interpreter;
use anyhow::{ensure, Context, Result};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone, Debug)]
pub(crate) struct Alias {
    pub mode: Vec<u8>,
    pub name: Vec<u8>,
    pub resource: Vec<u8>,
    pub defaults: Vec<(Vec<u8>, Vec<u8>)>,
    pub arguments: Vec<Vec<u8>>,
}
#[derive(Clone, Debug)]
pub(crate) struct Line {
    pub offset: usize,
    pub bytes: Vec<u8>,
}
#[derive(Clone, Debug)]
pub(crate) struct Program {
    pub lines: Vec<Line>,
    pub sections: BTreeMap<Vec<u8>, Vec<usize>>,
    pub aliases: BTreeMap<Vec<u8>, Alias>,
    pub macros: crate::fes_macros::Macros,
    pub declarations: crate::fes_declarations::Declarations,
    flow: BTreeMap<Vec<u8>, Vec<crate::fes_flow::Row>>,
}
pub(crate) fn words(bytes: &[u8], check: &dyn Fn() -> Result<()>) -> Result<Vec<Vec<u8>>> {
    let out =
        crysirror_text::byte_groups::split_grouped_bytes(bytes, b" ", Default::default(), check)?;
    Ok(out
        .spans
        .into_iter()
        .map(|r| out.written_bytes[r].to_vec())
        .collect())
}

impl Program {
    pub fn parse(bytes: &[u8], check: &dyn Fn() -> Result<()>) -> Result<Self> {
        check()?;
        ensure!(
            bytes.len() <= 16 * 1024 * 1024 && !bytes.contains(&0),
            "CatSystem2 FES text extent/NUL"
        );
        let mut lines = Vec::new();
        let mut sections: BTreeMap<Vec<u8>, Vec<usize>> = BTreeMap::new();
        let mut active: Option<Vec<u8>> = None;
        let mut at = 0;
        while at < bytes.len() {
            check()?;
            ensure!(lines.len() < 65536, "CatSystem2 FES line budget");
            let start = at;
            while at < bytes.len() && !matches!(bytes[at], b'\r' | b'\n') {
                at += 1;
            }
            let line = &bytes[start..at];
            ensure!(
                line.len() < 1024,
                "CatSystem2 FES native line buffer extent"
            );
            if at < bytes.len() {
                let cr = bytes[at] == b'\r';
                at += 1;
                if cr && bytes.get(at) == Some(&b'\n') {
                    at += 1;
                }
            }
            let w = words(line, check)?;
            ensure!(
                !line.windows(2).any(|w| matches!(w, b"//" | b"/*" | b"*/")),
                "CatSystem2 unsupported non-preprocessed FES comments"
            );
            let index = lines.len();
            if line.first() == Some(&b'#') {
                ensure!(
                    !w.is_empty() && w[0].len() > 1 && w[0].len() < 32,
                    "CatSystem2 unsupported FES section header"
                );
                let key = w[0][1..].to_vec();
                ensure!(
                    w.len() == 1 || matches!(key.as_slice(), b"KEYBLOCK" | b"OBJECT"),
                    "CatSystem2 unsupported FES section arguments {}",
                    String::from_utf8_lossy(&key)
                );
                active = Some(key);
            }
            if let Some(key) = &active {
                sections.entry(key.clone()).or_default().push(index);
            } else {
                ensure!(line.is_empty(), "CatSystem2 FES text before first section");
            }
            lines.push(Line {
                offset: start,
                bytes: line.to_vec(),
            });
        }
        ensure!(
            sections.contains_key(b"START".as_slice()),
            "CatSystem2 FES START section missing"
        );
        let mut macros = crate::fes_macros::Macros::default();
        if let Some(rows) = sections.get(b"MACRO".as_slice()) {
            for &row in rows {
                check()?;
                let line = &lines[row].bytes;
                if line.is_empty() || line.first() == Some(&b'#') { continue; }
                let tokens = words(line, check)?;
                ensure!(tokens.len() == 2, "CatSystem2 FES MACRO row layout");
                macros.insert(tokens[0].clone(), tokens[1].clone())?;
            }
        }
        let declarations = crate::fes_declarations::Declarations::parse(&lines, &sections, check)?;
        let mut aliases = BTreeMap::new();
        if let Some(rows) = sections.get(b"SUBCOMMAND".as_slice()) {
            for &i in rows {
                let b = &lines[i].bytes;
                if b.first() == Some(&b'#') || b.is_empty() {
                    continue;
                }
                let w = words(b, check)?;
                let colon = w
                    .iter()
                    .position(|w| w == b":")
                    .context("CatSystem2 FES alias separator")?;
                ensure!(
                    colon >= 3
                        && colon <= 19
                        && matches!(w[0].as_slice(), b"oneshot" | b"continue")
                        && w[1].len() < 32,
                    "CatSystem2 unsupported FES alias layout"
                );
                let mut defaults = Vec::new();
                for p in &w[3..colon] {
                    let split = p
                        .iter()
                        .position(|b| *b == b'[')
                        .context("CatSystem2 FES alias parameter")?;
                    ensure!(
                        split > 0
                            && p.last() == Some(&b']')
                            && !p[split + 1..p.len() - 1].contains(&b'['),
                        "CatSystem2 FES alias parameter layout"
                    );
                    defaults.push((p[..split].to_vec(), p[split + 1..p.len() - 1].to_vec()));
                }
                ensure!(
                    w.len() - colon - 1 <= 16,
                    "CatSystem2 FES alias argument budget"
                );
                let alias = Alias {
                    mode: w[0].clone(),
                    name: w[1].clone(),
                    resource: w[2].clone(),
                    defaults,
                    arguments: w[colon + 1..].to_vec(),
                };
                aliases.insert(alias.name.clone(), alias);
            }
        }
        let mut flow = BTreeMap::new();
        for (section, rows) in &sections {
            let mut tokens = Vec::with_capacity(rows.len());
            for &row in rows {
                check()?;
                tokens.push(
                    words(&lines[row].bytes, check)?
                        .into_iter()
                        .next()
                        .unwrap_or_default(),
                );
            }
            flow.insert(
                section.clone(),
                crate::fes_flow::build(&tokens).map_err(anyhow::Error::msg)?,
            );
        }
        check()?;
        Ok(Self {
            lines,
            sections,
            aliases,
            macros,
            declarations,
            flow,
        })
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum PendingKind {
    Native,
    Object,
    BlankWait,
    SectionRestart,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PendingOperation {
    ///Immutable expanded row before per-word expansion; native raw-tail commands use this.
    pub raw:Vec<u8>,
    pub kind: PendingKind,
    pub section: Vec<u8>,
    pub ordinal: u32,
    pub line: usize,
    pub byte_offset: usize,
    pub name: Vec<u8>,
    pub arguments: Vec<Vec<u8>>,
}
#[derive(Clone, Debug)]
pub(crate) struct Machine {
    pub program: Arc<Program>,
    pub owner: u32,
    pub string_table:crate::native_values::StringVariables,
    pub section: Vec<u8>,
    pub ordinal: u32,
    pub executed: u64,
    pub pending: Option<PendingOperation>,
    pub click_down: bool,
    pub force_skip: bool,
    /// Clock at which a running rdraw/rwipe object (780290/780490) completes.
    pub transition_until: Option<u32>,
    /// The objects of `wait <object>`.
    pub object_wait: Vec<u32>,
    pub skip_enabled: bool,
    pub skip_ctrl: bool,
    pub skip_continue: bool,
    pub skip_forced: bool,
    pub timing: crate::fes_timing::State,
    ///Native player519 is separate from task20 and parent's108 value.
    pub result_code: u32,
    call_stack: Vec<(Vec<u8>, u32)>,
    /// 756B10: `@` and the implicit event object while a button section runs.
    event_target: Option<(Vec<u8>, u32)>,
    /// 757850's synchronous section call: the stack depth it returns to and the
    /// wait it interrupted, restored when the section ends without waiting.
    external: Option<(usize, Option<crate::fes_timing::Wait>)>,
}
///Only the native owner-local expression/name scope, without claiming typed
///resource admission or constructing a runnable player.
pub(crate) fn initialize_player_bank(owner:u32, interpreter:&mut Interpreter)->Result<()> {
        ensure!(
            interpreter.create_player_bank(owner)?,
            "CatSystem2 unsupported FES player reinitialization"
        );
        for name in [
            b"_EXITCODE_".as_slice(),
            b"_CLICK_L_",
            b"_CLICK_R_",
            b"_SCRL_U_",
            b"_SCRL_D_",
            b"_PUSH_UP_",
            b"_PUSH_DOWN_",
            b"_PUSH_RIGHT_",
            b"_PUSH_LEFT_",
        ] {
            interpreter.set_named_value(
                name,
                0,
                if name == b"_EXITCODE_" { u32::MAX } else { 0 },
            )?;
        }
    Ok(())
}
impl Machine {
    ///757800 changes the section and next row only. It does not reset the
    ///native frame base, wait, call stack, object state or result value.
    pub(crate) fn set_section(&mut self, section: &[u8]) -> Result<bool> {
        ensure!(!section.contains(&0) && section.len() < 32,
            "CatSystem2 FES section-name extent");
        if !self.program.sections.contains_key(section) { return Ok(false); }
        ensure!(self.pending.is_none(),
            "CatSystem2 external section change while original operation owns a dependency");
        self.section = section.to_vec(); self.ordinal = 1;
        Ok(true)
    }
    pub fn new(program: Arc<Program>, owner: u32, interpreter: &mut Interpreter) -> Result<Self> {
        //Parsing declarations never establishes engine resource ownership.
        //Until a scoped publication batch is provided, fail before allocating
        //a local bank or acknowledging the original FES load native.
        ensure!(program.declarations.is_empty(),
            "CatSystem2 FES declarations require shared object/input resource publication ({} DEFINE, {} OBJECT, {} KEYBLOCK)",
            program.declarations.definitions.len(), program.declarations.assignments.len(), program.declarations.key_blocks.len());
        initialize_player_bank(owner, interpreter)?;
        Ok(Self::from_initialized(program, owner))
    }
    ///Only the task owner can attach a reviewed metadata bank. This does not
    ///complete a frame: the owner must retain its native continuation until
    ///the class resources have a real shared-engine publication receipt.
    pub(crate) fn from_initialized(program: Arc<Program>, owner: u32) -> Self {
        Self {
            program,
            owner,
            string_table:Default::default(),
            section: b"START".to_vec(),
            ordinal: 1,
            executed: 0,
            pending: None,
            click_down: false,
            force_skip: false,
            transition_until: None,
            object_wait: Vec::new(),
            skip_enabled: true,
            skip_ctrl: false,
            skip_continue: false,
            skip_forced: false,
            timing: Default::default(),
            result_code: 0,
            call_stack: Vec::new(),
            event_target: None,
            external: None,
        }
    }
    ///757850: run `section` now, whatever wait the player is parked on.
    pub(crate) fn begin_external(&mut self,section:&[u8],name:&[u8],index:u32)->Result<bool> {
        let depth=self.call_stack.len();
        if !self.begin_event(section,name,index)? {return Ok(false);}
        self.external=Some((depth,self.timing.wait.take()));
        Ok(true)
    }
    /// The section reached a wait of its own, which the player keeps.
    pub(crate) fn end_external(&mut self){self.external=None;}
    ///756B10 runs an event section inline. A waiting player resumes its row after
    ///the call returns; a `next` inside the section abandons that row, as native.
    pub(crate) fn begin_event(&mut self, section: &[u8], name: &[u8], index: u32) -> Result<bool> {
        if !self.program.sections.contains_key(section) {
            return Ok(false);
        }
        ensure!(self.call_stack.len() < 64, "CatSystem2 FES call depth budget");
        self.pending = None;
        self.call_stack.push((self.section.clone(), self.ordinal));
        self.section = section.to_vec();
        self.ordinal = 1;
        self.event_target = Some((name.to_vec(), index));
        Ok(true)
    }
    ///Runs original integer-expression commands until a native operation. The
    ///operation is retained at its original section/row/byte offset on retries.
    pub fn step(
        &mut self,
        interpreter: &mut Interpreter,
        budget: u32,
    ) -> Result<Option<PendingOperation>> {
        self.step_at_clock(interpreter, budget, self.timing.now)
    }
    pub fn step_at_clock(
        &mut self,
        interpreter: &mut Interpreter,
        budget: u32,
        clock: u32,
    ) -> Result<Option<PendingOperation>> {
        self.step_at_clock_with_strings(interpreter,budget,clock,None)
    }
    pub(crate) fn step_at_clock_with_strings(&mut self,interpreter:&mut Interpreter,budget:u32,clock:u32,
        strings:Option<&crate::native_values::StringVariables>)->Result<Option<PendingOperation>> {
        ensure!(
            interpreter.select_player_bank(self.owner),
            "CatSystem2 FES local bank missing"
        );
        if let Some(p) = &self.pending {
            return Ok(Some(p.clone()));
        }
        let runnable = self
            .timing
            .observe(clock, self.skip_enabled, self.click_down);
        interpreter.set_named_value(b"_FRAME_", 0, self.timing.frame())?;
        if !runnable && self.external.is_none() {
            return Ok(None);
        }
        for _ in 0..budget {
            let rows = self
                .program
                .sections
                .get(&self.section)
                .context("CatSystem2 FES section missing")?;
            let Some(&i) = rows.get(self.ordinal as usize) else {
                if let Some((section, ordinal)) = self.call_stack.pop() {
                    self.section = section;
                    self.ordinal = ordinal;
                    if self.external.as_ref().is_some_and(|(depth,_)|*depth==self.call_stack.len()) {
                        self.timing.wait=self.external.take().unwrap().1;
                        return Ok(None);
                    }
                    continue;
                }
                //7563A0 restarts row1 with native wait1. This is a real timing
                //boundary, not a successful native caller ACK or a halt.
                let last = *rows.last().context("CatSystem2 empty FES section")?;
                let line = &self.program.lines[last];
                let p = PendingOperation {
                    raw:Vec::new(),
                    kind: PendingKind::SectionRestart,
                    section: self.section.clone(),
                    ordinal: self.ordinal,
                    line: last + 1,
                    byte_offset: line.offset + line.bytes.len(),
                    name: Vec::new(),
                    arguments: Vec::new(),
                };
                self.pending = Some(p.clone());
                return Ok(Some(p));
            };
            let line = &self.program.lines[i];
            //769F30:765D20 expands the complete row, then766720 applies
            //macros.765F40 expands each parsed command word a second time.
            let expand=|source:&[u8]|->Result<Vec<u8>> {
                if let Some(bank)=strings {bank.expand(source,2559)}else{
                    ensure!(!source.windows(4).any(|s|s==b"$str"),"CatSystem2 FES string-bank root unavailable");
                    Ok(source.to_vec())
                }
            };
            let expanded = self.program.macros.expand(&expand(&line.bytes)?, &|| Ok(()))?;
            let mut w = words(&expanded, &|| Ok(()))?.iter().map(|s|expand(s)).collect::<Result<Vec<_>>>()?;
            if w.first().is_some_and(|word|word==b"@") {
                let (name,index)=self.event_target.as_ref().context("CatSystem2 FES @ outside an object event")?;
                w[0]=format!("{}[{index}]",String::from_utf8_lossy(name)).into_bytes();
            }
            //Row branch links are compiled from original tokens. Dynamic
            //replacement of structural control tokens needs a new graph, not
            //a guessed branch destination.
            let raw=words(&line.bytes,&||Ok(()))?;
            for control in [b"if".as_slice(),b"else",b"endif"] {
                ensure!(raw.first().is_some_and(|s|s.eq_ignore_ascii_case(control))==w.first().is_some_and(|s|s.eq_ignore_ascii_case(control)),
                    "CatSystem2 dynamically substituted FES structural token");
            }
            if w.is_empty() {
                let p = PendingOperation {
                    raw:Vec::new(),
                    kind: PendingKind::BlankWait,
                    section: self.section.clone(),
                    ordinal: self.ordinal,
                    line: i,
                    byte_offset: line.offset,
                    name: Vec::new(),
                    arguments: Vec::new(),
                };
                self.pending = Some(p.clone());
                return Ok(Some(p));
            }
            if w[0] == b"if" {
                ensure!(w.len() == 2, "CatSystem2 FES if argument count");
                let condition = interpreter.evaluate(&w[1])? != 0;
                let links = &self.program.flow[&self.section];
                let row = links
                    .get(self.ordinal as usize)
                    .context("CatSystem2 FES if row missing")?;
                self.ordinal = if condition {
                    row.when_true
                } else {
                    row.when_false
                }
                .unwrap_or(links.len()) as u32;
                self.executed += 1;
                continue;
            }
            if w[0] == b"endif" {
                ensure!(w.len() == 1, "CatSystem2 FES endif argument count");
                self.advance()?;
                self.executed += 1;
                continue;
            }
            if w[0] == b"reset_frame" {
                ensure!(w.len() == 1, "CatSystem2 FES reset_frame argument count");
                self.timing.reset_frame();
                interpreter.set_named_value(b"_FRAME_", 0, 0)?;
                self.advance()?;
                self.executed += 1;
                continue;
            }
            if w[0] == b"keyskip" {
                let mut enabled = true;
                let mut ctrl = false;
                let mut continuous = false;
                let mut i = 1;
                while i < w.len() {
                    let word = &w[i];
                    if word.eq_ignore_ascii_case(b"on") {
                        enabled = true;
                    } else if word.eq_ignore_ascii_case(b"off") {
                        enabled = false;
                    } else if word.eq_ignore_ascii_case(b"ctrl") {
                        ctrl = true;
                    } else if word.eq_ignore_ascii_case(b"cont") {
                        continuous = true;
                    } else if word.eq_ignore_ascii_case(b"force") {
                        i += 1;
                        self.skip_forced = if let Some(word) = w.get(i) {
                            interpreter.evaluate(word)? != 0
                        } else {
                            false
                        };
                        if self.skip_forced {
                            self.click_down = true;
                            self.force_skip = true;
                            interpreter.set_named_value(b"_CLICK_L_", 0, u32::MAX)?;
                        }
                    } else {
                        enabled = interpreter.evaluate(word)? != 0;
                    }
                    i += 1;
                }
                self.skip_enabled = enabled;
                self.skip_ctrl = ctrl;
                self.skip_continue = continuous;
                self.advance()?;
                self.executed += 1;
                continue;
            }
            if matches!(w[0].as_slice(), b"call" | b"next") {
                // 76A680 finds no such section: the row does nothing.
                if !w.get(1).is_some_and(|target|self.program.sections.contains_key(target)) {
                    self.advance()?;
                    self.executed += 1;
                    continue;
                }
                if w[0] == b"call" {
                    ensure!(
                        self.call_stack.len() < 64,
                        "CatSystem2 FES call depth budget"
                    );
                    let links = &self.program.flow[&self.section];
                    let ordinal = crate::fes_flow::successor(links, self.ordinal as usize)
                        .map_err(anyhow::Error::msg)?
                        .unwrap_or(links.len()) as u32;
                    self.call_stack.push((self.section.clone(), ordinal));
                } else {
                    self.call_stack.clear();
                }
                self.section = w[1].clone();
                self.ordinal = 1;
                self.executed += 1;
                continue;
            }
            //7690E0 resolves the first token against the actual object table
            //before considering general natives/expressions. Never evaluate an
            //unsupported object command as a default-zero flag expression.
            let object_name=w[0].split(|byte|matches!(byte,b'['|b' ')).next().unwrap_or_default();
            if object_name==b"_TOPOBJ_" || self.program.declarations.definitions.iter()
                .any(|(_,definition)|definition.reference.name==object_name) {
                let pending=PendingOperation{raw:expanded.clone(),kind:PendingKind::Object,
                    section:self.section.clone(),ordinal:self.ordinal,line:i,byte_offset:line.offset,
                    name:w[0].clone(),arguments:w[1..].to_vec()};
                self.pending=Some(pending.clone());return Ok(Some(pending));
            }
            if !crate::fes_natives::is_builtin(&w[0]) {
                interpreter.set_default(self.event_target.as_ref().map_or(0,|target|target.1));
                interpreter.evaluate(&expanded)?;
                self.executed += 1;
                self.advance()?;
                continue;
            }
            let p = PendingOperation {
                raw:expanded.clone(),
                kind: PendingKind::Native,
                section: self.section.clone(),
                ordinal: self.ordinal,
                line: i,
                byte_offset: line.offset,
                name: w[0].clone(),
                arguments: w[1..].to_vec(),
            };
            self.pending = Some(p.clone());
            return Ok(Some(p));
        }
        Ok(None)
    }
    pub fn diagnostics(&self) -> serde_json::Value {
        let aliases=self.program.aliases.values().map(|a|serde_json::json!({"mode":String::from_utf8_lossy(&a.mode),"name":String::from_utf8_lossy(&a.name),"resource":String::from_utf8_lossy(&a.resource),"parameters":a.defaults.len(),"arguments":a.arguments.len()})).collect::<Vec<_>>();
        serde_json::json!({"owner":self.owner,"section":String::from_utf8_lossy(&self.section),"ordinal":self.ordinal,"executed":self.executed,"call_depth":self.call_stack.len(),"click_down":self.click_down,"force_skip":self.force_skip,"result_code":self.result_code,"skip_enabled":self.skip_enabled,"skip_ctrl":self.skip_ctrl,"skip_continue":self.skip_continue,"clock":self.timing.now,"frame_base":self.timing.base,"frame":self.timing.frame(),"wait":self.timing.wait.map(|w|format!("{w:?}")),"lines":self.program.lines.len(),"sections":self.program.sections.len(),"macro_count":self.program.macros.len(),"aliases":aliases,"pending":self.pending.as_ref().map(|p|serde_json::json!({"kind":format!("{:?}",p.kind),"section":String::from_utf8_lossy(&p.section),"ordinal":p.ordinal,"line":p.line,"byte_offset":p.byte_offset,"name":String::from_utf8_lossy(&p.name),"arguments":p.arguments.iter().map(|b|String::from_utf8_lossy(b)).collect::<Vec<_>>()}))})
    }
    pub(crate) fn acknowledge_native(&mut self, operation: &PendingOperation) -> Result<()> {
        ensure!(
            self.pending.as_ref() == Some(operation) && operation.kind==PendingKind::Native,
            "CatSystem2 stale FES native completion"
        );
        self.complete_operation()
    }
    pub(crate) fn acknowledge_object(&mut self,operation:&PendingOperation)->Result<()> {
        ensure!(self.pending.as_ref()==Some(operation)&&operation.kind==PendingKind::Object,
            "CatSystem2 stale FES object completion");
        self.complete_operation()
    }
    /// 77BA10 `break`: the rest of this section is skipped, back to whoever called it.
    pub(crate) fn break_section(&mut self, operation: &PendingOperation) -> Result<()> {
        ensure!(self.pending.as_ref() == Some(operation), "CatSystem2 stale FES break");
        self.ordinal = self.program.sections.get(&self.section).context("CatSystem2 FES section missing")?.len() as u32;
        self.executed = self.executed.checked_add(1).context("CatSystem2 FES instruction overflow")?;
        self.pending = None;
        Ok(())
    }
    fn complete_operation(&mut self)->Result<()> {
        self.advance()?;
        self.executed = self
            .executed
            .checked_add(1)
            .context("CatSystem2 FES instruction overflow")?;
        self.pending = None;
        Ok(())
    }
    pub(crate) fn begin_object_wait(&mut self, operation: &PendingOperation) -> Result<()> {
        ensure!(operation.kind==PendingKind::Native && operation.name==b"wait" && operation.arguments.is_empty(),
            "CatSystem2 all-object wait operation mismatch");
        self.acknowledge_native(operation)?;
        self.timing.wait=Some(crate::fes_timing::Wait::Objects);
        Ok(())
    }
    pub(crate) fn begin_wait(
        &mut self,
        operation: &PendingOperation,
        relative: Option<u32>,
        duration: u32,
    ) -> Result<()> {
        ensure!(
            self.pending.as_ref() == Some(operation),
            "CatSystem2 stale FES wait admission"
        );
        if operation.kind == PendingKind::SectionRestart {
            self.ordinal = 1;
        } else {
            self.advance()?;
        }
        if let Some(target) = relative {
            self.timing.frame_wait(target);
        } else {
            self.timing.numeric_wait(duration);
        }
        self.executed += 1;
        self.pending = None;
        Ok(())
    }
    fn advance(&mut self) -> Result<()> {
        let links = &self.program.flow[&self.section];
        self.ordinal = crate::fes_flow::successor(links, self.ordinal as usize)
            .map_err(anyhow::Error::msg)?
            .unwrap_or(links.len()) as u32;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_keyskip_defaults_and_force_off_do_not_erase_injected_click() {
        let source=Arc::new(Program::parse(b"#START\nkeyskip off ctrl cont force 1\nkeyskip off ctrl cont force 0\nreset_frame\n$0=_FRAME_\nsteam_isenable 1",&||Ok(())).unwrap());
        let mut interpreter = Interpreter::new(8).unwrap();
        let mut vm = Machine::new(source, 9, &mut interpreter).unwrap();
        assert!(vm.skip_enabled);
        let pending = vm
            .step_at_clock(&mut interpreter, 100, 50)
            .unwrap()
            .unwrap();
        assert_eq!(pending.name, b"steam_isenable");
        assert!(!vm.skip_enabled && vm.skip_ctrl && vm.skip_continue && !vm.skip_forced);
        assert!(vm.click_down && vm.force_skip);
        assert_eq!(interpreter.evaluate(b"_CLICK_L_").unwrap(), u32::MAX);
        assert_eq!(interpreter.flag(0), 0);
        assert_eq!(interpreter.evaluate(b"_FRAME_").unwrap(), 0);
        vm.acknowledge_native(&pending).unwrap();
        vm.step_at_clock(&mut interpreter, 100, 51).unwrap();
        assert_eq!(interpreter.evaluate(b"_FRAME_").unwrap(), 1);
    }
    #[test]
    fn original_branch_links_nested_else_call_return_and_next_preserve_global_flags() {
        let source=b"#START\n$0=0\nif (1)\nif (0)\n$0=100\nelse\n$0=7\nendif\nelse\n$0=999\nendif\ncall SUB\nnext TAIL\n$0=777\n#SUB\n$0+=3\n#TAIL\nsteam_isenable 4\n";
        let program = Arc::new(Program::parse(source, &|| Ok(())).unwrap());
        let mut interpreter = Interpreter::new(32).unwrap();
        let mut machine = Machine::new(program, 8, &mut interpreter).unwrap();
        let pending = machine.step(&mut interpreter, 64).unwrap().unwrap();
        assert_eq!(interpreter.flag(0), 10);
        assert_eq!(pending.name, b"steam_isenable");
        assert_eq!(pending.section, b"TAIL");
        assert!(machine.call_stack.is_empty());
        assert_eq!(
            machine.step(&mut interpreter, 64).unwrap(),
            Some(pending.clone())
        );
        assert_eq!(interpreter.flag(0), 10);
        let mut stale = pending.clone();
        stale.ordinal += 1;
        assert!(machine.acknowledge_native(&stale).is_err());
        machine.acknowledge_native(&pending).unwrap();
        assert_eq!(
            machine.step(&mut interpreter, 64).unwrap().unwrap().kind,
            PendingKind::SectionRestart
        );
    }
    #[test]
    fn real_local_bank_and_expression_execution_stop_at_original_pending_operation() {
        let p=Arc::new(Program::parse(b"#SUBCOMMAND\r\noneshot move motMove px[0] : [0]\r\n#START\r\n$3 = 9\r\n\\2 = 17\r\nsteam_isenable 4\r\n$3 = 10\r\n",&||Ok(())).unwrap());
        let mut interpreter = Interpreter::new(32).unwrap();
        let mut vm = Machine::new(p, 8, &mut interpreter).unwrap();
        assert_eq!(interpreter.evaluate(b"_EXITCODE_").unwrap(), u32::MAX);
        let pending = vm.step(&mut interpreter, 64).unwrap().unwrap();
        assert_eq!(interpreter.flag(3), 9);
        assert_eq!(interpreter.evaluate(b"\\2").unwrap(), 17);
        assert_eq!(pending.name, b"steam_isenable");
        assert_eq!(pending.ordinal, 3);
        assert_eq!(vm.executed, 2);
        assert!(interpreter.create_player_bank(9).unwrap());
        assert_eq!(interpreter.evaluate(b"\\2").unwrap(), 0);
        assert!(!interpreter.create_player_bank(8).unwrap());
        assert_eq!(interpreter.evaluate(b"\\2").unwrap(), 0);
        assert_eq!(vm.step(&mut interpreter, 64).unwrap(), Some(pending));
        assert_eq!(interpreter.flag(3), 9);
        assert!(!interpreter.select_player_bank(999));
        assert_eq!(interpreter.evaluate(b"\\2").unwrap(), 17);
    }
    #[test]
    fn budget_blank_wait_and_section_restart_are_distinct_pending_boundaries() {
        let mut interpreter = Interpreter::new(8).unwrap();
        let p = Arc::new(Program::parse(b"#START\n$0=4\n$1=5\n", &|| Ok(())).unwrap());
        let mut vm = Machine::new(p, 1, &mut interpreter).unwrap();
        assert_eq!(vm.step(&mut interpreter, 1).unwrap(), None);
        assert_eq!((interpreter.flag(0), interpreter.flag(1)), (4, 0));
        let end = vm.step(&mut interpreter, 8).unwrap().unwrap();
        assert_eq!(end.kind, PendingKind::SectionRestart);
        assert_eq!(interpreter.flag(1), 5);
        let p = Arc::new(Program::parse(b"#START\n\n$2=7", &|| Ok(())).unwrap());
        let mut vm = Machine::new(p, 2, &mut interpreter).unwrap();
        assert_eq!(
            vm.step(&mut interpreter, 8).unwrap().unwrap().kind,
            PendingKind::BlankWait
        );
        assert_eq!(interpreter.flag(2), 0);
    }
    #[test]
    fn string_row_macro_word_order_and_pending_snapshot_keep_original_pc() {
        let source=b"#MACRO\nVALUE 8\n#START\n$0=$str1\nwait $str2";
        let program=Arc::new(Program::parse(source,&||Ok(())).unwrap());
        let mut interpreter=Interpreter::new(8).unwrap();
        let mut vm=Machine::new(program,12,&mut interpreter).unwrap();
        let mut bank=crate::native_values::StringVariables::default();
        bank.assign(1,b"VALUE".to_vec()).unwrap();bank.assign(2,b"$str3".to_vec()).unwrap();bank.assign(3,b"2".to_vec()).unwrap();
        let op=vm.step_at_clock_with_strings(&mut interpreter,100,3,Some(&bank)).unwrap().unwrap();
        assert_eq!(interpreter.flag(0),8);assert_eq!(op.name,b"wait");assert_eq!(op.arguments,[b"2".to_vec()]);
        assert_eq!(op.byte_offset,source.windows(4).position(|s|s==b"wait").unwrap());
        bank.assign(3,b"99".to_vec()).unwrap();
        assert_eq!(vm.step_at_clock_with_strings(&mut interpreter,100,3,Some(&bank)).unwrap(),Some(op));
        let program=Arc::new(Program::parse(b"#START\n$str1",&||Ok(())).unwrap());
        let mut vm=Machine::new(program,13,&mut interpreter).unwrap();
        assert!(vm.step(&mut interpreter,100).unwrap_err().to_string().contains("string-bank root unavailable"));
        assert_eq!(vm.ordinal,1);assert_eq!(vm.executed,0);
    }
    #[test]
    fn dynamic_structural_markers_fail_before_expression_mutation() {
        let program=Arc::new(Program::parse(b"#START\n$str1 1",&||Ok(())).unwrap());
        let mut interpreter=Interpreter::new(8).unwrap();let mut vm=Machine::new(program,21,&mut interpreter).unwrap();
        let mut bank=crate::native_values::StringVariables::default();bank.assign(1,b"if".to_vec()).unwrap();
        assert!(vm.step_at_clock_with_strings(&mut interpreter,100,1,Some(&bank)).unwrap_err().to_string().contains("structural token"));
        assert_eq!(vm.ordinal,1);assert_eq!(vm.executed,0);
    }
    #[test]
    fn unsupported_declarations_templates_and_cancellation_never_make_a_fake_player() {
        for b in [
            b"#START\n#OBJECT\nimage foo".as_slice(),
            b"#START\n//not-preprocessed",
        ] {
            assert!(Program::parse(b, &|| Ok(())).is_err());
        }
        assert!(Program::parse(b"#START\n$0=1", &|| anyhow::bail!("cancelled")).is_err());
    }
    #[test]
    fn original_macros_expand_expression_and_pending_arguments_without_changing_source_pc() {
        let source = b"#MACRO\nVALUE 2\nVALUE 7\nNEXT other\n#START\n$0=VALUE\nif $0==VALUE\nwait VALUE\nendif\nnext NEXT\n#other\n$1=4";
        let program = Arc::new(Program::parse(source, &|| Ok(())).unwrap());
        assert_eq!(program.macros.len(), 2);
        let mut interpreter = Interpreter::new(8).unwrap();
        let mut vm = Machine::new(program, 3, &mut interpreter).unwrap();
        let pending = vm.step(&mut interpreter, 32).unwrap().unwrap();
        assert_eq!(interpreter.flag(0), 7);
        assert_eq!(pending.name, b"wait");
        assert_eq!(pending.arguments, [b"7".to_vec()]);
        assert_eq!(&source[pending.byte_offset..pending.byte_offset + 10], b"wait VALUE");
        assert_eq!(vm.step(&mut interpreter, 32).unwrap(), Some(pending.clone()));
        vm.begin_wait(&pending, None, 7).unwrap();
        assert!(vm.step_at_clock(&mut interpreter, 32, 8).unwrap().is_some());
        assert_eq!(interpreter.flag(1), 4);
    }
    #[test]
    fn complete_declaration_inventory_is_retained_without_fake_resource_admission() {
        let source = b"#DEFINE\nPLANE pl[6]\nIMAGE logo\n#OBJECT BASE POS SIZE\npl 0,0 0,0 1280,720\n#OBJECT PRI VRAM DISP\npl[0] 5000 0,0 0\npl[1] 6000 1536,0\n#KEYBLOCK 4\n0 1,2 3,4,5,6\n#START\n$0=9";
        let program = Arc::new(Program::parse(source, &|| Ok(())).unwrap());
        let declarations = &program.declarations;
        assert_eq!(declarations.definitions.len(), 2);
        assert_eq!(declarations.assignments.len(), 3);
        assert_eq!(declarations.assignments[2].1.fields, [(41, b"6000".to_vec()), (40, b"1536,0".to_vec())]);
        assert_eq!(declarations.key_blocks[0].1.count, 4);
        assert_eq!(declarations.key_blocks[0].1.rows.len(), 1);
        let mut interpreter = Interpreter::new(8).unwrap();
        assert!(Machine::new(program, 3, &mut interpreter).unwrap_err().to_string().contains("shared object/input resource publication"));
        assert!(!interpreter.select_player_bank(3));
        assert_eq!(interpreter.flag(0), 0);
    }
    #[test]
    fn original_implicit_base_commands_retain_source_without_running_argument_expressions() {
        let source=b"#START\n$0=7\n_TOPOBJ_[0] fade (++$0)\n$0=99\n";
        let program=Arc::new(Program::parse(source,&||Ok(())).unwrap());
        let mut interpreter=Interpreter::new(8).unwrap();
        let mut machine=Machine::new(program,8,&mut interpreter).unwrap();
        let pending=machine.step(&mut interpreter,100).unwrap().unwrap();
        assert_eq!(pending.kind,PendingKind::Object);assert_eq!(pending.name,b"_TOPOBJ_[0]");
        assert_eq!(pending.arguments,[b"fade".to_vec(),b"(++$0)".to_vec()]);
        assert_eq!(interpreter.flag(0),7);
        assert_eq!(&source[pending.byte_offset..pending.byte_offset+len_native_root_prefix()],b"_TOPOBJ_[0]");
        for _ in 0..3 {assert_eq!(machine.step(&mut interpreter,100).unwrap(),Some(pending.clone()));}
        assert!(machine.acknowledge_native(&pending).is_err());
        assert_eq!(machine.pending,Some(pending));assert_eq!(interpreter.flag(0),7);
        fn len_native_root_prefix()->usize {b"_TOPOBJ_[0]".len()}
    }

}
