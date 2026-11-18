// SPDX-License-Identifier: MPL-2.0
// NSS lowering mirrors NitroSharp Compiler/Emitter.cs at e3869842 (MIT).
// This IR is private to CrySirror; it is not a native N2System bytecode ABI.
use crate::{
    catalog::Builtin,
    source::*,
    value::{Binary, Value},
};
use anyhow::{Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Instruction {
    Push(Value, bool),
    Load(Name, bool),
    Store(Name),
    Index,
    StoreIndex(Option<Binary>),
    BindArgument(String, usize),
    Unary(char),
    Binary(Binary),
    Pop,
    Jump(usize),
    Unless(usize),
    Return,
    Argument(String, usize, usize),
    Call(String, usize),
    Chapter,
    Scene,
    SelectBegin,
    IsPressed,
    SelectYield,
    SelectEnd,
    Curve(Vec<bool>),
    Activate(String),
    Unsupported(u32, String),
    ClearText,
    AppendText(String),
    LineEnd,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Block {
    pub window: String,
    pub parts: Vec<DialoguePart>,
    pub entry: String,
}
/// Original contexts carry an end-PC even though CrySirror lowers their
/// control flow to jumps. Escape counts these lexical contexts before callers.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Scope {
    pub begin: usize,
    pub end: usize,
    pub selection: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Code {
    pub display_name: String,
    pub kind: RoutineKind,
    pub parameters: Vec<String>,
    pub line: u32,
    pub instructions: Vec<Instruction>,
    /// Required private IR metadata. Modules are rebuilt from NSS at boot;
    /// older scope-less IR must be recompiled instead of misexecuting Escape.
    pub scopes: Vec<Scope>,
    pub blocks: BTreeMap<String, Block>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Module {
    pub entry: Option<String>,
    pub imports: Vec<String>,
    pub routines: BTreeMap<String, Code>,
}

struct Compiler {
    code: Vec<Instruction>,
    breaks: Vec<Vec<usize>>,
    blocks: BTreeMap<String, Block>,
    scopes: Vec<Scope>,
}
impl Module {
    pub fn compile(source: Source) -> Result<Self> {
        let mut routines = BTreeMap::new();
        let entry = source
            .routines
            .iter()
            .find(|r| r.kind == RoutineKind::Chapter)
            .map(|r| r.name.clone());
        for routine in source.routines {
            let mut compiler = Compiler {
                code: Vec::new(),
                breaks: Vec::new(),
                blocks: BTreeMap::new(),
                scopes: Vec::new(),
            };
            compiler.body(&routine.body)?;
            compiler.emit(Instruction::Return)?;
            for (name, block) in &mut compiler.blocks {
                let mut dialogue = Compiler {
                    code: Vec::new(),
                    breaks: Vec::new(),
                    blocks: BTreeMap::new(),
                    scopes: Vec::new(),
                };
                let mut clear = true;
                for part in &block.parts {
                    match part {
                        DialoguePart::Text(text) => {
                            if clear {
                                dialogue.emit(Instruction::ClearText)?;
                                clear = false;
                            }
                            dialogue.emit(Instruction::AppendText(text.clone()))?;
                        }
                        DialoguePart::Blank => {
                            dialogue.emit(Instruction::LineEnd)?;
                            clear = true;
                        }
                        DialoguePart::Code(body) => dialogue.body(body)?,
                    }
                }
                dialogue.emit(Instruction::Return)?;
                block.parts.clear();
                ensure!(
                    dialogue.blocks.is_empty(),
                    "N2System nested executable dialogue declaration"
                );
                block.entry = format!("\u{1f}{}::{name}", routine.name);
                ensure!(
                    routines
                        .insert(
                            block.entry.clone(),
                            Code {
                                display_name: routine.name.clone(),
                                kind: RoutineKind::Function,
                                parameters: Vec::new(),
                                line: routine.line,
                                instructions: dialogue.code,
                                scopes: dialogue.scopes,
                                blocks: BTreeMap::new()
                            }
                        )
                        .is_none(),
                    "N2System dialogue identity collision"
                );
            }
            routines.insert(
                routine.name.clone(),
                Code {
                    display_name: routine.name,
                    kind: routine.kind,
                    parameters: routine.parameters,
                    line: routine.line,
                    instructions: compiler.code,
                    scopes: compiler.scopes,
                    blocks: compiler.blocks,
                },
            );
        }
        Ok(Self {
            entry,
            imports: source.includes,
            routines,
        })
    }
}
impl Compiler {
    fn emit(&mut self, instruction: Instruction) -> Result<usize> {
        ensure!(
            self.code.len() < 1_000_000,
            "N2System lowered instruction limit"
        );
        let index = self.code.len();
        self.code.push(instruction);
        Ok(index)
    }
    fn patch(&mut self, index: usize, target: usize) -> Result<()> {
        match self.code.get_mut(index) {
            Some(Instruction::Jump(dst) | Instruction::Unless(dst)) => *dst = target,
            _ => bail!("N2System invalid compiler jump"),
        }
        Ok(())
    }
    fn body(&mut self, body: &[Statement]) -> Result<()> {
        for statement in body {
            self.statement(statement)?;
        }
        Ok(())
    }
    fn finish_breaks(&mut self) -> Result<()> {
        for index in self
            .breaks
            .pop()
            .ok_or_else(|| anyhow::anyhow!("N2System break scope"))?
        {
            self.patch(index, self.code.len())?;
        }
        Ok(())
    }
    fn statement(&mut self, s: &Statement) -> Result<()> {
        match s {
            Statement::Block(body) => {
                let begin = self.code.len();
                self.body(body)?;
                self.scopes.push(Scope {
                    begin,
                    end: self.code.len(),
                    selection: false,
                });
            }
            Statement::Unsupported { line, syntax } => {
                self.emit(Instruction::Unsupported(*line, syntax.clone()))?;
            }
            Statement::Eval(e) => {
                self.expr(e, true)?;
                self.emit(Instruction::Pop)?;
            }
            Statement::Return => {
                self.emit(Instruction::Return)?;
            }
            Statement::Break => {
                ensure!(!self.breaks.is_empty(), "N2System break outside loop");
                let i = self.emit(Instruction::Jump(usize::MAX))?;
                self.breaks.last_mut().unwrap().push(i);
            }
            Statement::If(condition, yes, no) => {
                self.expr(condition, true)?;
                let alternate = self.emit(Instruction::Unless(usize::MAX))?;
                self.statement(yes)?;
                if let Some(no) = no {
                    let exit = self.emit(Instruction::Jump(usize::MAX))?;
                    self.patch(alternate, self.code.len())?;
                    self.statement(no)?;
                    self.patch(exit, self.code.len())?;
                } else {
                    self.patch(alternate, self.code.len())?;
                }
            }
            Statement::While(condition, body) => {
                let begin = self.code.len();
                self.expr(condition, true)?;
                let exit = self.emit(Instruction::Unless(usize::MAX))?;
                self.breaks.push(Vec::new());
                // One native while context owns its body braces. Escaping it
                // must skip the back edge as well as the remaining body.
                if let Statement::Block(body) = body.as_ref() {
                    self.body(body)?;
                } else {
                    self.statement(body)?;
                }
                self.emit(Instruction::Jump(begin))?;
                self.patch(exit, self.code.len())?;
                self.finish_breaks()?;
                self.scopes.push(Scope {
                    begin,
                    end: self.code.len(),
                    selection: false,
                });
            }
            Statement::Chapter(target) => {
                self.expr(target, false)?;
                self.emit(Instruction::Chapter)?;
            }
            Statement::Scene(target) => {
                self.expr(target, false)?;
                self.emit(Instruction::Scene)?;
            }
            Statement::Case(label, body) => {
                self.expr(label, false)?;
                self.emit(Instruction::IsPressed)?;
                let next = self.emit(Instruction::Unless(usize::MAX))?;
                let begin = self.code.len();
                self.body(body)?;
                self.scopes.push(Scope {
                    begin,
                    end: self.code.len(),
                    selection: false,
                });
                self.patch(next, self.code.len())?;
            }
            Statement::Select(cases) => {
                let start = self.code.len();
                self.emit(Instruction::SelectBegin)?;
                self.breaks.push(Vec::new());
                for (label, body) in cases {
                    if let Some(label) = label {
                        self.expr(label, false)?;
                        self.emit(Instruction::IsPressed)?;
                        let next = self.emit(Instruction::Unless(usize::MAX))?;
                        let begin = self.code.len();
                        self.body(body)?;
                        self.scopes.push(Scope {
                            begin,
                            end: self.code.len(),
                            selection: false,
                        });
                        self.patch(next, self.code.len())?;
                    } else {
                        let begin = self.code.len();
                        self.body(body)?;
                        self.scopes.push(Scope {
                            begin,
                            end: self.code.len(),
                            selection: false,
                        });
                    }
                }
                self.emit(Instruction::SelectYield)?;
                self.emit(Instruction::Unless(start))?;
                self.finish_breaks()?;
                self.emit(Instruction::SelectEnd)?;
                self.scopes.push(Scope {
                    begin: start,
                    end: self.code.len(),
                    selection: true,
                });
            }
            Statement::Dialogue {
                name,
                window,
                parts,
            } => {
                ensure!(
                    self.blocks
                        .insert(
                            name.clone(),
                            Block {
                                window: window.clone(),
                                parts: parts.clone(),
                                entry: String::new(),
                            }
                        )
                        .is_none(),
                    "N2System duplicate dialogue block {name}"
                );
                self.emit(Instruction::Activate(name.clone()))?;
            }
        }
        Ok(())
    }
    fn expr(&mut self, e: &Expr, constants: bool) -> Result<()> {
        match e {
            Expr::Index(target, key) => {
                self.expr(target, false)?;
                self.expr(key, false)?;
                self.emit(Instruction::Index)?;
            }
            Expr::IndexAssign(target, op, value) => {
                self.expr(value, constants)?;
                self.expr(target, false)?;
                self.emit(Instruction::StoreIndex(*op))?;
                self.emit(Instruction::Push(Value::Null, false))?;
            }
            Expr::Stringify(value) => {
                self.expr(value, false)?;
                self.emit(Instruction::Unary('S'))?;
            }
            Expr::Literal(v) => {
                self.emit(Instruction::Push(v.clone(), constants))?;
            }
            Expr::Name(name) => {
                self.emit(Instruction::Load(name.clone(), constants))?;
            }
            Expr::Unary(op, e) => {
                self.expr(e, constants)?;
                self.emit(Instruction::Unary(*op))?;
            }
            // The reference evaluates the right operand before the left.
            Expr::Binary(op, left, right) => {
                self.expr(right, constants)?;
                self.expr(left, constants)?;
                self.emit(Instruction::Binary(*op))?;
            }
            Expr::Assign(name, op, value) => {
                self.expr(value, constants)?;
                if let Some(op) = op {
                    self.emit(Instruction::Load(name.clone(), false))?;
                    self.emit(Instruction::Binary(*op))?;
                }
                self.emit(Instruction::Store(name.clone()))?;
                self.emit(Instruction::Push(Value::Null, false))?;
            }
            Expr::Update(name, increment) => {
                self.emit(Instruction::Load(name.clone(), false))?;
                self.emit(Instruction::Unary(if *increment { 'I' } else { 'D' }))?;
                self.emit(Instruction::Store(name.clone()))?;
                self.emit(Instruction::Push(Value::Null, false))?;
            }
            Expr::Call(name, args) => {
                let builtin = Builtin::parse(name).is_some();
                for (i, arg) in args.iter().enumerate() {
                    let guard = if builtin {
                        None
                    } else {
                        Some(self.emit(Instruction::Argument(name.clone(), i, usize::MAX))?)
                    };
                    self.expr(
                        arg,
                        builtin
                            && i > 0
                            && !matches!(
                                Builtin::parse(name),
                                Some(Builtin::Array | Builtin::AssocArray)
                            ),
                    )?;
                    if let Some(guard) = guard {
                        self.emit(Instruction::BindArgument(name.clone(), i))?;
                        self.code[guard] = Instruction::Argument(name.clone(), i, self.code.len());
                    }
                }
                self.emit(Instruction::Call(
                    name.clone(),
                    if builtin { args.len() } else { 0 },
                ))?;
            }
            Expr::Curve(points) => {
                for (x, y, _) in points {
                    self.expr(x, constants)?;
                    self.expr(y, constants)?;
                }
                self.emit(Instruction::Curve(points.iter().map(|p| p.2).collect()))?;
            }
        }
        Ok(())
    }
}
