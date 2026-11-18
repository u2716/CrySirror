// SPDX-License-Identifier: MPL-2.0
// NSS grammar follows NitroSharp Syntax/{Lexer,Parser,SyntaxFacts}.cs at
// e3869842bb5bdd5ed95344027297620cd54454a6 (MIT, Committee of Zero).
use crate::value::{Binary, Value};
use anyhow::{bail, ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

const TOKEN_LIMIT: usize = 1_000_000;
const DEPTH_LIMIT: usize = 128;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Name {
    pub text: String,
    pub flag: bool,
    pub explicit: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Expr {
    Literal(Value),
    Index(Box<Expr>, Box<Expr>),
    Stringify(Box<Expr>),
    IndexAssign(Box<Expr>, Option<Binary>, Box<Expr>),
    Name(Name),
    Call(String, Vec<Expr>),
    Unary(char, Box<Expr>),
    Binary(Binary, Box<Expr>, Box<Expr>),
    Assign(Name, Option<Binary>, Box<Expr>),
    Update(Name, bool),
    Curve(Vec<(Expr, Expr, bool)>),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Statement {
    Block(Vec<Statement>),
    Unsupported {
        line: u32,
        syntax: String,
    },
    Eval(Expr),
    If(Expr, Box<Self>, Option<Box<Self>>),
    While(Expr, Box<Self>),
    Break,
    Return,
    Chapter(Expr),
    Scene(Expr),
    Select(Vec<(Option<Expr>, Vec<Self>)>),
    Case(Expr, Vec<Self>),
    Dialogue {
        name: String,
        window: String,
        parts: Vec<DialoguePart>,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DialoguePart {
    Text(String),
    Blank,
    Code(Vec<Statement>),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoutineKind {
    Chapter,
    Scene,
    Function,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Routine {
    pub name: String,
    pub kind: RoutineKind,
    pub parameters: Vec<String>,
    pub body: Vec<Statement>,
    pub line: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Source {
    pub includes: Vec<String>,
    pub routines: Vec<Routine>,
}

#[derive(Clone, Debug, PartialEq)]
enum Kind {
    Word(String, Option<char>, bool),
    Number(f32),
    Symbol(String),
    Include,
    Pre(String),
    EndPre,
    BlockName(String),
    Markup(String),
    Blank,
    Eof,
}
#[derive(Clone, Debug)]
struct Token {
    kind: Kind,
    start: usize,
    end: usize,
    line: u32,
}

struct Scanner<'a> {
    text: &'a str,
    at: usize,
    line: u32,
    modes: Vec<bool>,
}
impl<'a> Scanner<'a> {
    fn rest(&self) -> &'a str {
        &self.text[self.at..]
    }
    fn chr(&self) -> Option<char> {
        self.rest().chars().next()
    }
    fn bump(&mut self) -> Option<char> {
        let c = self.chr()?;
        self.at += c.len_utf8();
        if c == '\n' {
            self.line += 1;
        }
        Some(c)
    }
    fn starts(&self, s: &str) -> bool {
        self.rest()
            .get(..s.len())
            .is_some_and(|x| x.eq_ignore_ascii_case(s))
    }
    fn trivia(&mut self) -> Result<()> {
        loop {
            while self
                .chr()
                .is_some_and(|c| c.is_whitespace() || c == '\u{feff}')
            {
                self.bump();
            }
            if self.rest().starts_with("//")
                || self.rest().starts_with(".//")
                || self.rest().starts_with(">//")
                || self.rest().starts_with("..")
            {
                while self.chr().is_some_and(|c| c != '\n') {
                    self.bump();
                }
                continue;
            }
            if self.rest().starts_with("/*") {
                self.at += 2;
                let mut quoted = false;
                let mut closed = false;
                while let Some(c) = self.chr() {
                    if !quoted && self.rest().starts_with("*/") {
                        self.at += 2;
                        closed = true;
                        break;
                    }
                    if c == '\n' {
                        quoted = false;
                    } else if c == '"' {
                        quoted = !quoted;
                    }
                    self.bump();
                }
                ensure!(
                    closed,
                    "N2System unterminated comment at line {}",
                    self.line
                );
                continue;
            }
            return Ok(());
        }
    }
    fn quoted(&mut self) -> Result<(String, Option<char>)> {
        self.bump();
        let sigil = match self.chr() {
            Some('$' | '#') => self.bump(),
            _ => None,
        };
        let begin = self.at;
        while self
            .chr()
            .is_some_and(|c| c != '"' && c != '\r' && c != '\n')
        {
            self.bump();
        }
        let value = self.text[begin..self.at].to_owned();
        ensure!(
            self.bump() == Some('"'),
            "N2System unterminated string at line {}",
            self.line
        );
        Ok((value, sigil))
    }
    fn next(&mut self) -> Result<Token> {
        let dialogue = self.modes.last().copied().unwrap_or(false);
        if !dialogue {
            self.trivia()?;
        }
        let start = self.at;
        let line = self.line;
        let kind = if self.at == self.text.len() {
            Kind::Eof
        } else if dialogue && self.starts("</pre>") {
            self.at += 6;
            self.modes.pop();
            Kind::EndPre
        } else if dialogue && self.chr() == Some('[') {
            self.bump();
            let begin = self.at;
            while self
                .chr()
                .is_some_and(|c| c != ']' && c != '\n' && c != '\r')
            {
                self.bump();
            }
            let name = self.text[begin..self.at].to_owned();
            if self.chr() == Some(']') {
                self.bump();
            }
            ensure!(
                !name.is_empty(),
                "N2System empty dialogue block identity at line {line}"
            );
            while self.chr().is_some_and(|c| matches!(c, ' ' | '\t' | '\r' | '\n')) {
                self.bump();
            }
            Kind::BlockName(name)
        } else if dialogue && self.chr() != Some('{') {
            let begin = self.at;
            let mut nested = 0usize;
            let mut newlines = 0usize;
            let mut blank_start = None;
            while self.at < self.text.len() {
                if nested == 0 && (self.chr() == Some('{') || self.starts("</pre>")) {
                    break;
                }
                if self.starts("<pre>") {
                    nested += 1;
                    self.at += 5;
                    continue;
                }
                if nested > 0 && self.starts("</pre>") {
                    nested -= 1;
                    self.at += 6;
                    continue;
                }
                if nested == 0 && self.rest().starts_with("//") {
                    while self.chr().is_some_and(|c| c != '\n') {
                        self.bump();
                    }
                }
                if self.chr() == Some('\n') {
                    if newlines == 0 { blank_start = Some((self.at, self.line)); }
                    newlines += 1;
                } else if self.chr().is_some_and(|c| !c.is_whitespace()) {
                    newlines = 0;
                    blank_start = None;
                }
                if newlines >= 2 {
                    let (at, line) = blank_start.unwrap();
                    if self.text[begin..at].trim().is_empty() {
                        self.bump();
                    } else {
                        // Leave the blank-line token for the next scan. It is
                        // a native dialogue input boundary, not text whitespace.
                        self.at = at;
                        self.line = line;
                    }
                    break;
                }
                self.bump();
            }
            let text = self.text[begin..self.at].to_owned();
            if text.trim().is_empty() {
                Kind::Blank
            } else {
                Kind::Markup(text)
            }
        } else if self.starts("<pre ") {
            self.at += 5;
            let begin = self.at;
            while self
                .chr()
                .is_some_and(|c| c != '>' && c != '\r' && c != '\n')
            {
                self.bump();
            }
            let window = self.text[begin..self.at].trim().to_owned();
            ensure!(
                !window.is_empty() && self.bump() == Some('>'),
                "N2System invalid PRE at line {line}"
            );
            self.modes.push(true);
            self.trivia()?;
            Kind::Pre(window)
        } else if self.chr().is_some_and(|c| c == '#' || c == '$')
            && self.rest().chars().nth(1).is_none_or(stop)
        {
            Kind::Word(self.bump().unwrap().to_string(), None, true)
        } else if self.rest().starts_with("#include") {
            self.at += "#include".len();
            Kind::Include
        } else if self.chr() == Some('"') {
            let (s, sigil) = self.quoted()?;
            Kind::Word(s, sigil, true)
        } else if self.chr() == Some('#')
            && self
                .rest()
                .get(1..7)
                .is_some_and(|s| s.bytes().all(|c| c.is_ascii_hexdigit()))
            && self
                .rest()
                .get(7..)
                .and_then(|s| s.chars().next())
                .is_none_or(stop)
        {
            let n = u32::from_str_radix(&self.rest()[1..7], 16)?;
            self.at += 7;
            Kind::Number(n as f32)
        } else {
            let mut punctuation = None;
            for p in [
                "@->", "->", "==", "!=", "<=", ">=", "++", "--", "+=", "-=", "*=", "/=", "&&", "||",
            ] {
                if self.rest().starts_with(p) {
                    self.at += p.len();
                    punctuation = Some(p.to_owned());
                    break;
                }
            }
            if let Some(p) = punctuation {
                Kind::Symbol(p)
            } else if self.chr().is_some_and(|c| {
                "{}[](),:;=+-*/%<>!&@.".contains(c)
                    && !(c == '.'
                        && self
                            .rest()
                            .chars()
                            .nth(1)
                            .is_some_and(|c| c.is_ascii_digit()))
            }) {
                let c = self.bump().unwrap();
                if c == '{' {
                    self.modes.push(false);
                } else if c == '}' {
                    self.modes.pop();
                }
                Kind::Symbol(c.to_string())
            } else {
                let sigil = match self.chr() {
                    Some('$' | '#') => self.bump(),
                    _ => None,
                };
                let begin = self.at;
                while self
                    .chr()
                    .is_some_and(|c| !stop(c) || c == '@' && self.at > begin)
                {
                    self.bump();
                }
                ensure!(self.at > begin, "N2System invalid token at line {line}");
                let value = &self.text[begin..self.at];
                if sigil.is_none() {
                    if let Ok(n) = value.parse::<f32>() {
                        ensure!(n.is_finite(), "N2System numeric literal overflow");
                        Kind::Number(n)
                    } else {
                        Kind::Word(value.into(), None, false)
                    }
                } else {
                    Kind::Word(value.into(), sigil, false)
                }
            }
        };
        ensure!(
            self.modes.len() <= DEPTH_LIMIT,
            "N2System lexical nesting limit"
        );
        Ok(Token {
            kind,
            start,
            end: self.at,
            line,
        })
    }
}
fn stop(c: char) -> bool {
    c.is_whitespace() || "\",:;{}[]()=+-*/<>%!|&$#@".contains(c)
}

struct Parser<'a> {
    text: &'a str,
    tokens: Vec<Token>,
    at: usize,
    depth: usize,
    parameters: BTreeSet<String>,
}
impl Source {
    pub fn parse(text: &str) -> Result<Self> {
        ensure!(text.len() <= 8 * 1024 * 1024, "N2System source byte limit");
        let mut scanner = Scanner {
            text,
            at: 0,
            line: 1,
            modes: vec![false],
        };
        let mut tokens = Vec::new();
        loop {
            let t = scanner.next()?;
            let eof = t.kind == Kind::Eof;
            tokens.push(t);
            ensure!(tokens.len() <= TOKEN_LIMIT, "N2System source token limit");
            if eof {
                break;
            }
        }
        let mut p = Parser {
            text,
            tokens,
            at: 0,
            depth: 0,
            parameters: BTreeSet::new(),
        };
        let mut source = Self {
            includes: Vec::new(),
            routines: Vec::new(),
        };
        while p.kind() != &Kind::Eof {
            if p.kind() == &Kind::Include {
                p.bump();
                let (name, _, _) = p.word()?;
                source.includes.push(name);
                p.terminators();
                continue;
            }
            if !matches!(p.kind(),Kind::Word(s,None,false) if ["chapter","scene","function"].contains(&s.as_str()))
            {
                let line = p.current().line;
                while p.kind() != &Kind::Eof && p.current().line == line {
                    p.bump();
                }
                continue;
            }
            let line = p.current().line;
            let (kind, _, _) = p.word()?;
            let kind = match kind.as_str() {
                "chapter" => RoutineKind::Chapter,
                "scene" => RoutineKind::Scene,
                "function" => RoutineKind::Function,
                _ => bail!("N2System expected declaration at line {line}, got {kind}"),
            };
            let start = p.current().start;
            let (mut name, _, _) = p.word()?;
            while !p.symbol("(")
                && !p.symbol("{")
                && p.kind() != &Kind::Eof
                && p.current().start == p.tokens[p.at - 1].end
            {
                let end = p.bump().end;
                name = p.text[start..end].to_owned();
            }
            let mut parameters = Vec::new();
            p.parameters.clear();
            if kind == RoutineKind::Function && p.eat("(") {
                while !p.eat(")") {
                    let (name, _, _) = p.word()?;
                    ensure!(
                        p.parameters.insert(name.clone()),
                        "N2System duplicate parameter {name}"
                    );
                    parameters.push(name);
                    if !p.eat(",") {
                        p.expect(")")?;
                        break;
                    }
                }
            }
            let body = p.block()?;
            let routine = Routine {
                name: name.clone(),
                kind,
                parameters,
                body,
                line,
            };
            if let Some(index) = source.routines.iter().position(|r| r.name == name) {
                ensure!(
                    kind == RoutineKind::Function
                        && source.routines[index].kind == RoutineKind::Function,
                    "N2System duplicate chapter/scene {name}"
                );
                source.routines[index] = routine;
            } else {
                source.routines.push(routine);
            }
        }

        Ok(source)
    }
}
impl Parser<'_> {
    fn current(&self) -> &Token {
        &self.tokens[self.at]
    }
    fn kind(&self) -> &Kind {
        &self.current().kind
    }
    fn bump(&mut self) -> Token {
        let t = self.current().clone();
        if t.kind != Kind::Eof {
            self.at += 1;
        }
        t
    }
    fn symbol(&self, s: &str) -> bool {
        matches!(self.kind(),Kind::Symbol(t) if t==s)
    }
    fn eat(&mut self, s: &str) -> bool {
        if self.symbol(s) {
            self.bump();
            true
        } else {
            false
        }
    }
    fn expect(&mut self, s: &str) -> Result<()> {
        ensure!(
            self.eat(s),
            "N2System expected {s} at line {}, got {:?}",
            self.current().line,
            self.kind()
        );
        Ok(())
    }
    fn word(&mut self) -> Result<(String, Option<char>, bool)> {
        let t = self.bump();
        if let Kind::Word(s, sigil, quoted) = t.kind {
            Ok((s, sigil, quoted))
        } else {
            bail!(
                "N2System expected name at line {}, got {:?}",
                t.line,
                t.kind
            )
        }
    }
    fn terminators(&mut self) -> bool {
        let mut n = false;
        while self.eat(";") || self.eat(":") {
            n = true;
        }
        n
    }
    fn terminate(&mut self) -> Result<()> {
        let ended = self.terminators()
            || self.symbol("}")
            || self.at > 0 && self.current().line > self.tokens[self.at - 1].line;
        ensure!(
            ended,
            "N2System statement terminator at line {}",
            self.current().line
        );
        Ok(())
    }
    fn block(&mut self) -> Result<Vec<Statement>> {
        self.expect("{")?;
        self.depth += 1;
        ensure!(
            self.depth <= DEPTH_LIMIT,
            "N2System statement nesting limit"
        );
        let mut v = Vec::new();
        while !self.eat("}") {
            ensure!(self.kind() != &Kind::Eof, "N2System unterminated block");
            v.push(self.statement()?);
        }
        self.depth -= 1;
        Ok(v)
    }
    fn statement(&mut self) -> Result<Statement> {
        if self.symbol("<")
            && matches!(self.tokens.get(self.at+1).map(|t|&t.kind),Some(Kind::Word(name,None,false)) if name=="voice")
        {
            let line = self.current().line;
            self.raw_until(&[">"])?;
            self.expect(">")?;
            return Ok(Statement::Unsupported {
                line,
                syntax: "voice markup outside PRE".into(),
            });
        }

        if self.eat("/") {
            return self.statement();
        }
        if self.eat(";") || self.eat(")") || self.eat(":") {
            return Ok(Statement::Block(Vec::new()));
        }
        if self.symbol("{") {
            return Ok(Statement::Block(self.block()?));
        }
        if let Kind::Pre(window) = self.kind().clone() {
            self.bump();
            let Kind::BlockName(name) = self.bump().kind else {
                bail!("N2System missing PRE block name")
            };
            let mut parts = Vec::new();
            while self.kind() != &Kind::EndPre {
                match self.kind().clone() {
                    Kind::Markup(s) => {
                        self.bump();
                        parts.push(DialoguePart::Text(s));
                    }
                    Kind::Blank => {
                        self.bump();
                        parts.push(DialoguePart::Blank);
                    }
                    Kind::Symbol(s) if s == "{" => parts.push(DialoguePart::Code(self.block()?)),
                    _ => bail!(
                        "N2System invalid PRE part at line {}: {:?}",
                        self.current().line,
                        self.kind()
                    ),
                }
            }
            self.bump();
            return Ok(Statement::Dialogue {
                name,
                window,
                parts,
            });
        }
        if let Kind::Word(keyword, None, false) = self.kind().clone() {
            match keyword.as_str() {
                "case" => {
                    self.bump();
                    let label = self.expr(0)?;
                    self.eat(":");
                    return Ok(Statement::Case(label, self.block()?));
                }
                "if" => {
                    self.bump();
                    self.expect("(")?;
                    let e = self.expr(0)?;
                    self.expect(")")?;
                    let a = Box::new(self.statement()?);
                    let b = if matches!(self.kind(),Kind::Word(s,None,false) if s=="else") {
                        self.bump();
                        if self.eat("(") {
                            if self.eat(")") {
                                Some(Box::new(self.statement()?))
                            } else {
                                let condition = self.expr(0)?;
                                self.expect(")")?;
                                Some(Box::new(Statement::If(
                                    condition,
                                    Box::new(self.statement()?),
                                    None,
                                )))
                            }
                        } else {
                            Some(Box::new(self.statement()?))
                        }
                    } else {
                        None
                    };
                    return Ok(Statement::If(e, a, b));
                }
                "while" => {
                    self.bump();
                    self.expect("(")?;
                    let e = self.expr(0)?;
                    self.expect(")")?;
                    return Ok(Statement::While(e, Box::new(self.statement()?)));
                }
                "return" => {
                    self.bump();
                    self.terminate()?;
                    return Ok(Statement::Return);
                }
                "break" => {
                    self.bump();
                    self.terminate()?;
                    return Ok(Statement::Break);
                }
                "call_chapter" => {
                    self.bump();
                    let target = if matches!(self.kind(), Kind::Word(_, Some(_), _)) {
                        self.expr(0)?
                    } else {
                        Expr::Literal(Value::String(self.raw_until(&[";", ":"])?))
                    };
                    self.terminate()?;
                    return Ok(Statement::Chapter(target));
                }
                "call_scene" => {
                    self.bump();
                    let target = if matches!(self.kind(), Kind::Word(_, Some(_), _)) {
                        self.expr(0)?
                    } else {
                        Expr::Literal(Value::String(self.raw_until(&[";", ":"])?))
                    };
                    self.terminate()?;
                    return Ok(Statement::Scene(target));
                }
                "select" => {
                    self.bump();
                    self.expect("{")?;
                    let mut cases = Vec::new();
                    while !self.eat("}") {
                        if matches!(self.kind(),Kind::Word(s,None,false) if s=="case") {
                            self.bump();
                            let label = self.expr(0)?;
                            self.eat(":");
                            cases.push((Some(label), self.block()?));
                        } else {
                            cases.push((None, vec![self.statement()?]));
                        }
                    }
                    return Ok(Statement::Select(cases));
                }
                _ => {}
            }
        }
        // Native labels in this grammar decorate the following statement.
        if matches!(self.kind(), Kind::Word(_, _, _))
            && matches!(self.tokens.get(self.at+1).map(|t|&t.kind),Some(Kind::Symbol(s)) if s==":")
        {
            self.bump();
            self.bump();
            return self.statement();
        }
        let line = self.current().line;
        let e = if matches!(self.kind(), Kind::Word(_, None, _))
            && !matches!(self.tokens.get(self.at+1).map(|t|&t.kind),Some(Kind::Symbol(s)) if ["=","+=","-=","*=","/=","++","--"].contains(&s.as_str()))
        {
            let (name, _, _) = self.word()?;
            let paren = self.eat("(");
            let args = self.arguments(paren)?;
            Expr::Call(name, args)
        } else {
            self.expr(0)?
        };
        let _ = line;
        self.terminate()?;
        Ok(Statement::Eval(e))
    }
    fn raw_until(&mut self, ends: &[&str]) -> Result<String> {
        let start = self.current().start;
        let mut end = start;
        while self.kind() != &Kind::Eof && !ends.iter().any(|s| self.symbol(s)) {
            end = self.bump().end;
        }
        ensure!(end > start, "N2System empty path or selection label");
        Ok(self.text[start..end].trim().trim_matches('"').to_owned())
    }
    fn arguments(&mut self, paren: bool) -> Result<Vec<Expr>> {
        let mut args = Vec::new();
        loop {
            if paren && self.eat(")") {
                break;
            }
            if !paren && (self.symbol(";") || self.symbol(":")) {
                break;
            }
            ensure!(
                self.kind() != &Kind::Eof,
                "N2System unterminated argument list"
            );
            if self.eat(",") || self.eat(".") || self.eat("&") || paren && self.eat(";") {
                continue;
            }
            args.push(self.expr(0)?);
            ensure!(args.len() <= 16384, "N2System argument limit");
        }
        Ok(args)
    }
    fn expr(&mut self, min: u8) -> Result<Expr> {
        self.depth += 1;
        ensure!(
            self.depth <= DEPTH_LIMIT,
            "N2System expression nesting limit"
        );
        let mut left = if let Kind::Symbol(s) = self.kind().clone() {
            if ["!", "+", "-", "@"].contains(&s.as_str()) {
                self.bump();
                let operand = if s == "@" && (self.symbol(",") || self.symbol(")")) {
                    Expr::Literal(Value::Number(0.))
                } else {
                    self.expr(8)?
                };
                Expr::Unary(s.chars().next().unwrap(), Box::new(operand))
            } else if self.eat("[") {
                let value = self.expr(0)?;
                self.expect("]")?;
                Expr::Stringify(Box::new(value))
            } else if self.eat("(") {
                let x = self.expr(0)?;
                if self.eat(",") {
                    let y = self.expr(0)?;
                    self.expect(")")?;
                    let mut points = vec![(x, y, true)];
                    while self.symbol("(") || self.symbol("{") {
                        let starting = self.eat("(");
                        if !starting {
                            self.expect("{")?;
                        }
                        let x = self.expr(0)?;
                        self.expect(",")?;
                        let y = self.expr(0)?;
                        self.expect(if starting { ")" } else { "}" })?;
                        points.push((x, y, starting));
                    }
                    Expr::Curve(points)
                } else {
                    self.eat(";");
                    self.expect(")")?;
                    x
                }
            } else {
                bail!(
                    "N2System expected expression at line {}",
                    self.current().line
                )
            }
        } else {
            let token = self.bump();
            match token.kind {
                Kind::Number(n) => Expr::Literal(Value::Number(n)),
                Kind::Word(s, sigil, quoted) => {
                    if sigil.is_none() && self.eat("(") {
                        Expr::Call(s, self.arguments(true)?)
                    } else if sigil.is_some() || self.parameters.contains(&s) {
                        Expr::Name(Name {
                            text: s,
                            flag: sigil == Some('#'),
                            explicit: true,
                        })
                    } else if s.eq_ignore_ascii_case("null") {
                        Expr::Literal(Value::Null)
                    } else if s.eq_ignore_ascii_case("true") {
                        Expr::Literal(Value::Boolean(true))
                    } else if s.eq_ignore_ascii_case("false") {
                        Expr::Literal(Value::Boolean(false))
                    } else if quoted {
                        Expr::Literal(Value::String(s))
                    } else {
                        Expr::Name(Name {
                            text: s,
                            flag: false,
                            explicit: false,
                        })
                    }
                }
                _ => bail!(
                    "N2System expression token at line {}: {:?}",
                    token.line,
                    token.kind
                ),
            }
        };
        while self.eat("[") {
            let key = self.expr(0)?;
            self.expect("]")?;
            left = Expr::Index(Box::new(left), Box::new(key));
        }
        loop {
            let Kind::Symbol(symbol) = self.kind().clone() else {
                break;
            };
            let (priority, op) = match symbol.as_str() {
                "=" | "+=" | "-=" | "*=" | "/=" | "++" | "--" => (1, None),
                "||" => (2, Some(Binary::Or)),
                "&&" => (2, Some(Binary::And)),
                "==" => (3, Some(Binary::Equal)),
                "!=" => (3, Some(Binary::NotEqual)),
                "<" => (4, Some(Binary::Less)),
                "<=" => (4, Some(Binary::LessEqual)),
                ">" => (4, Some(Binary::Greater)),
                ">=" => (4, Some(Binary::GreaterEqual)),
                "+" => (5, Some(Binary::Add)),
                "-" => (5, Some(Binary::Subtract)),
                "*" => (6, Some(Binary::Multiply)),
                "/" => (6, Some(Binary::Divide)),
                "%" => (6, Some(Binary::Remainder)),
                _ => break,
            };
            if priority < min {
                break;
            }
            self.bump();
            if priority == 1 {
                if matches!(left, Expr::Index(..)) {
                    let op = match symbol.as_str() {
                        "+=" | "++" => Some(Binary::Add),
                        "-=" | "--" => Some(Binary::Subtract),
                        "*=" => Some(Binary::Multiply),
                        "/=" => Some(Binary::Divide),
                        _ => None,
                    };
                    let value = if symbol == "++" || symbol == "--" {
                        Expr::Literal(Value::Number(1.))
                    } else {
                        self.expr(priority)?
                    };
                    left = Expr::IndexAssign(Box::new(left), op, Box::new(value));
                    continue;
                }
                let Expr::Name(name) = left else {
                    bail!("N2System assignment requires a variable")
                };
                if symbol == "++" || symbol == "--" {
                    left = Expr::Update(name, symbol == "++");
                    continue;
                }
                let assign = match symbol.as_str() {
                    "+=" => Some(Binary::Add),
                    "-=" => Some(Binary::Subtract),
                    "*=" => Some(Binary::Multiply),
                    "/=" => Some(Binary::Divide),
                    _ => None,
                };
                left = Expr::Assign(name, assign, Box::new(self.expr(priority)?));
            } else {
                left = Expr::Binary(
                    op.context("N2System binary operator")?,
                    Box::new(left),
                    Box::new(self.expr(priority)?),
                );
            }
        }
        self.depth -= 1;
        Ok(left)
    }
}
