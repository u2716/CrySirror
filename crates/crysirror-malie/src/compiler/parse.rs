// SPDX-License-Identifier: MPL-2.0
//! Malie script parser and code generator (675AE0 and callees).
//!
//! Statements 676BD0/676090, declarations 68A6E0/68A770/68A9A0/68A3C0/
//! 68B240, structs 68ACD0, expressions 689780..689B60/689FA0/68A250,
//! constant folding 678C60, array-size evaluation 677E00, expression code
//! 6792E0/679A30/678B70/67A920, emit helpers 679F40..67A0B0, labels
//! 675540/675D90/675380, messages 675680/6759D0, `&` directives 675C20.
//! Code is generated while parsing, as in the original, including its
//! quirks (expression-relative `?:` targets, global loads for subscripted
//! locals, the no-op jump after an `if` test, `switch` evaluating only its
//! expression). Where the original dereferences null the compile fails.
use super::lex::{self, keyword_class, text, w, Lexer, Token, W};
use crate::program::{Function, Program, Symbol as ProgramSymbol};
use anyhow::{bail, Result};
use std::collections::{BTreeMap, HashMap};

// ---------------------------------------------------------------- types
#[derive(Clone, Debug, Default)]
struct Type {
    kind: i32,
    size: i32,
    sub: Option<Box<Type>>,
    /// Function parameter scope (kind 19).
    scope: Option<usize>,
    /// Struct record (kind 16).
    record: Option<usize>,
    /// Declarator name (kind 21).
    name: W,
}
/// 7B5DC8: basic type sizes.
const BASIC_SIZE: [i32; 16] = [0, 0, 1, 1, 2, 2, 4, 4, 4, 4, 4, 8, 0, 0, 0, 0];
fn basic(kind: i32) -> Type {
    Type { kind, size: BASIC_SIZE[kind as usize], ..Default::default() }
}
fn node(kind: i32, size: i32, sub: Option<Type>) -> Type {
    Type { kind, size, sub: sub.map(Box::new), ..Default::default() }
}
/// 68B2E0.
fn sizeof(t: &Type) -> i32 {
    let mut t = t;
    loop {
        if t.kind == 18 {
            return t.size.wrapping_mul(t.sub.as_deref().map_or(0, sizeof));
        }
        if t.kind == 19 {
            return 0;
        }
        match &t.sub {
            Some(s) if t.kind != 20 => t = s,
            _ => return t.size,
        }
    }
}
/// 68AF70: equal kinds and identical (shared basic) subtypes.
fn same_type(a: &Type, b: &Type) -> bool {
    if a.kind != b.kind {
        return false;
    }
    match (&a.sub, &b.sub) {
        (None, None) => true,
        (Some(x), Some(y)) => x.kind <= 15 && y.kind <= 15 && x.kind == y.kind,
        _ => false,
    }
}

// --------------------------------------------------------------- scopes
#[derive(Clone, Debug)]
struct Sym {
    name: W,
    ty: Type,
    storage: i32,
    qual: i32,
    conv: i32,
    offset: i32,
}
#[derive(Default)]
struct Scope {
    parent: Option<usize>,
    list: Vec<usize>,
    map: HashMap<W, usize>,
    /// Data/local bytes (frame [2]).
    data: i32,
    /// Parameter bytes (frame [3]).
    params: i32,
}
struct Record {
    name: W,
    scope: usize,
}
struct Link {
    name: W,
    kind: u32,
    pc: u32,
}
struct Label {
    name: W,
    addr: i64,
    fixups: Vec<usize>,
}

// ---------------------------------------------------------- expressions
#[derive(Clone, Debug)]
enum Node {
    Ident(W),
    Int(u32),
    Str(W),
    Op(u8, Option<Box<Node>>, Option<Box<Node>>),
}
fn op(k: u8, a: Option<Node>, b: Option<Node>) -> Node {
    Node::Op(k, a.map(Box::new), b.map(Box::new))
}
fn is_int(n: &Option<Box<Node>>) -> Option<u32> {
    match n.as_deref() {
        Some(Node::Int(v)) => Some(*v),
        _ => None,
    }
}

pub struct Compiler<'a> {
    lex: Lexer<'a>,
    cur: Token,
    syms: Vec<Sym>,
    scopes: Vec<Scope>,
    global: usize,
    scope: usize,
    records: Vec<Record>,
    links: Vec<Link>,
    link_map: HashMap<W, usize>,
    labels: Vec<Label>,
    label_map: HashMap<W, usize>,
    current_fn: Option<usize>,
    current_label: Option<usize>,
    in_body: bool,
    label_counter: u32,
    code: Vec<u8>,
    base: usize,
    last_op: i32,
    store_op: u8,
    strings: Vec<u8>,
    string_map: HashMap<W, u32>,
    messages: Vec<u8>,
    ranges: Vec<[u32; 2]>,
    message_open: bool,
    errors: Vec<String>,
}

const CODE_LIMIT: usize = 12 * 1024 * 1024;
const STRING_LIMIT: usize = 0x40_0000;

impl<'a> Compiler<'a> {
    pub fn new(lex: Lexer<'a>) -> Self {
        let mut c = Self {
            lex,
            cur: Token::default(),
            syms: Vec::new(),
            scopes: vec![Scope::default()],
            global: 0,
            scope: 0,
            records: Vec::new(),
            links: Vec::new(),
            link_map: HashMap::new(),
            labels: Vec::new(),
            label_map: HashMap::new(),
            current_fn: None,
            current_label: None,
            in_body: false,
            label_counter: 0,
            code: Vec::new(),
            base: 0,
            last_op: -1,
            store_op: 0,
            strings: Vec::new(),
            string_map: HashMap::new(),
            messages: Vec::new(),
            ranges: Vec::new(),
            message_open: false,
            errors: Vec::new(),
        };
        c.next();
        c
    }
    fn next(&mut self) {
        self.cur = self.lex.next();
    }
    fn kind(&self) -> i32 {
        self.cur.kind
    }
    /// 68A300.
    fn error(&mut self, m: impl Into<String>) {
        let m = m.into();
        self.errors.push(format!("line {}: {m}", self.lex.line()));
    }
    /// 675A00: report, skip through the next `;`.
    fn fail(&mut self, m: impl Into<String>) {
        self.error(m);
        while self.kind() != lex::SEMI && self.kind() != lex::EOF {
            self.next();
        }
        self.next();
    }

    // ------------------------------------------------------------ output
    fn begin(&mut self) {
        self.base = self.code.len();
        self.last_op = -1;
    }
    fn end(&mut self) {
        self.last_op = -1;
        if self.code.len() > CODE_LIMIT {
            self.error("code exceeds 12 MiB");
        }
    }
    /// 679F40: a second consecutive BOOL is dropped.
    fn op(&mut self, b: u8) {
        if self.last_op == 29 && b == 29 {
            return;
        }
        self.last_op = i32::from(b);
        self.code.push(b);
    }
    fn u8(&mut self, v: u8) {
        self.code.push(v);
    }
    fn u32(&mut self, v: u32) -> usize {
        let at = self.code.len();
        self.code.extend_from_slice(&v.to_le_bytes());
        at
    }
    fn patch(&mut self, at: usize, v: u32) {
        self.code[at..at + 4].copy_from_slice(&v.to_le_bytes());
    }
    fn rel(&self) -> u32 {
        (self.code.len() - self.base) as u32
    }
    /// 67A020.
    fn push_int(&mut self, v: u32) {
        if v == 0 {
            self.op(15);
        } else if v.wrapping_sub(1) > 0xFE {
            self.op(13);
            self.u32(v);
        } else {
            self.op(17);
            self.u8(v as u8);
        }
    }
    /// 678A50/679FC0.
    fn push_string(&mut self, s: &[u16]) {
        let s = &s[..lex::text_len(s).min(s.len())];
        let at = match self.string_map.get(s) {
            Some(&v) => v,
            None => {
                let bytes = (s.len() + 1) * 2;
                if self.strings.len() + bytes < STRING_LIMIT {
                    let at = self.strings.len() as u32;
                    for &u in s {
                        self.strings.extend_from_slice(&u.to_le_bytes());
                    }
                    self.strings.extend_from_slice(&[0, 0]);
                    self.string_map.insert(s.to_vec(), at);
                    at
                } else {
                    u32::MAX
                }
            }
        };
        if at >= 0x10000 {
            self.op(12);
            self.u32(at);
        } else if at >= 256 {
            self.op(10);
            self.code.extend_from_slice(&(at as u16).to_le_bytes());
        } else {
            self.op(9);
            self.u8(at as u8);
        }
    }

    // ----------------------------------------------------------- symbols
    fn new_scope(&mut self, parent: Option<usize>) -> usize {
        self.scopes.push(Scope { parent, ..Default::default() });
        self.scopes.len() - 1
    }
    fn local(&self, scope: usize, name: &[u16]) -> Option<usize> {
        self.scopes[scope].map.get(name).copied()
    }
    /// 67AC40.
    fn lookup(&self, name: &[u16]) -> Option<usize> {
        let mut s = Some(self.scope);
        while let Some(i) = s {
            if let Some(v) = self.local(i, name) {
                return Some(v);
            }
            s = self.scopes[i].parent;
        }
        None
    }
    /// 67AAF0.
    fn declare(&mut self, scope: usize, name: W, ty: Type, storage: i32, qual: i32, conv: i32) -> usize {
        let size = sizeof(&ty);
        let sc = &mut self.scopes[scope];
        let offset = if storage == 6 {
            let o = sc.params;
            sc.params = sc.params.wrapping_add(size);
            o
        } else if storage == 1 {
            sc.data = sc.data.wrapping_add(size);
            sc.data.wrapping_neg()
        } else {
            let o = sc.data;
            sc.data = sc.data.wrapping_add(size);
            o
        };
        self.syms.push(Sym { name: name.clone(), ty, storage, qual, conv, offset });
        let id = self.syms.len() - 1;
        let sc = &mut self.scopes[scope];
        sc.list.push(id);
        sc.map.insert(name, id);
        id
    }
    /// 67AFF0.
    fn link_add(&mut self, name: &[u16]) -> i32 {
        let i = self.links.len();
        self.links.push(Link { name: name.to_vec(), kind: 0, pc: 0 });
        self.link_map.insert(name.to_vec(), i);
        i as i32
    }
    /// 67B110.
    fn link_set(&mut self, name: &[u16], kind: u32, pc: u32) {
        match self.link_map.get(name) {
            Some(&i) => {
                self.links[i].kind = kind;
                self.links[i].pc = pc;
            }
            None => log::debug!("Malie compiler LINK ERROR : {}", text(name)),
        }
    }
    /// 675540.
    fn label(&mut self, name: W, addr: i64) -> usize {
        if let Some(&i) = self.label_map.get(&name) {
            if self.labels[i].addr == -1 {
                self.labels[i].addr = addr;
            } else if addr != -1 {
                let n = text(&self.labels[i].name);
                self.fail(format!("label {n} is defined twice"));
            }
            return i;
        }
        self.labels.push(Label { name: name.clone(), addr, fixups: Vec::new() });
        self.label_map.insert(name, self.labels.len() - 1);
        self.labels.len() - 1
    }
    fn internal_label(&mut self, addr: i64) -> usize {
        let name = w(&format!("$L{:08}", self.label_counter));
        self.label_counter += 1;
        self.label(name, addr)
    }
    /// 675450 codes 3/4/5: JUMP/TJUMP/FJUMP to a label.
    fn jump(&mut self, code: u8, label: usize) {
        self.begin();
        self.op(code - 3);
        let at = self.u32(0);
        self.labels[label].fixups.push(at);
        self.end();
    }
    /// 675450 other codes: a raw byte; 12 also ends an open message.
    fn raw(&mut self, code: u8) {
        if code == 12 && self.message_open {
            self.close_message();
        }
        self.code.push(code);
    }
    fn close_message(&mut self) {
        if self.message_open {
            self.message_open = false;
            self.messages.extend_from_slice(&[0, 0]);
        }
    }
    /// 675680.
    fn message(&mut self, s: &[u16]) -> Result<()> {
        if !self.message_open {
            let Some(f) = self.lookup(&w("_ms_message")) else { bail!("_ms_message is not declared") };
            let index = self.syms[f].offset as u32;
            self.begin();
            self.push_int(self.ranges.len() as u32);
            self.op(45);
            self.u32(index);
            self.op(14);
            self.end();
            self.message_open = true;
            self.ranges.push([self.messages.len() as u32, 0]);
        }
        let s = &s[..lex::text_len(s).min(s.len())];
        if self.messages.len() + s.len() * 2 >= 12_582_912 {
            bail!("message text exceeds 12 MiB");
        }
        for &u in s {
            self.messages.extend_from_slice(&u.to_le_bytes());
        }
        let len = self.messages.len() as u32;
        let last = self.ranges.last_mut().unwrap();
        last[1] = len - last[0];
        Ok(())
    }

    // -------------------------------------------------------- statements
    /// 676BD0.
    fn top(&mut self) -> Result<()> {
        match self.kind() {
            lex::STRING | lex::TEXT => self.next(),
            37 => {
                self.next();
                self.directive()?;
            }
            lex::MSTRING => {
                let t = self.cur.text.clone();
                self.message(&t)?;
                self.next();
            }
            lex::MEND => {
                self.close_message();
                self.next();
            }
            _ => self.statement()?,
        }
        Ok(())
    }
    /// 675780.
    fn block(&mut self) -> Result<()> {
        while self.kind() != lex::RBRACE && self.kind() != lex::EOF {
            self.top()?;
        }
        Ok(())
    }
    /// 675C20: `&call label;`, `&clear;` .. `&pause;`.
    fn directive(&mut self) -> Result<()> {
        const NAMES: [&str; 7] = ["call", "clear", "end", "messageout", "ol", "page", "pause"];
        let code = NAMES.iter().position(|n| w(n) == self.cur.text).map(|i| i as u8 + 7);
        match code {
            None => {
                let m = format!("unknown directive &{}", text(&self.cur.text));
                self.fail(m);
            }
            Some(7) => {
                self.next();
                if let Some(l) = self.label_ref() {
                    self.code.push(7);
                    self.code.extend_from_slice(&(l as u32).to_le_bytes());
                    if self.kind() == lex::SEMI {
                        self.next();
                    } else {
                        self.fail("syntax error");
                    }
                }
            }
            Some(c) => {
                self.raw(c);
                self.next();
                if self.kind() == lex::SEMI {
                    self.next();
                } else {
                    self.fail("syntax error");
                }
            }
        }
        Ok(())
    }
    /// 675D90.
    fn label_ref(&mut self) -> Option<usize> {
        let name;
        let mut bad;
        if self.kind() == lex::OTHER && self.cur.first() == 64 {
            self.next();
            bad = self.kind() != lex::IDENT && self.kind() != lex::NUMBER;
            let Some(owner) = self.current_label else {
                self.fail("local label outside a label");
                return None;
            };
            let mut n = self.labels[owner].name.clone();
            n.push(64);
            n.extend_from_slice(&self.cur.text);
            name = n;
            self.next();
        } else {
            bad = self.kind() != lex::IDENT;
            let mut n = self.cur.text.clone();
            self.next();
            if self.kind() == lex::OTHER && self.cur.first() == 64 {
                self.next();
                if self.kind() != lex::IDENT && self.kind() != lex::NUMBER {
                    bad = true;
                }
                n.push(64);
                n.extend_from_slice(&self.cur.text);
                self.next();
            }
            name = n;
        }
        if bad {
            self.fail(format!("invalid label {}", text(&name)));
            return None;
        }
        Some(self.label(name, -1))
    }
    fn expect(&mut self, kind: i32) -> bool {
        if self.kind() != kind {
            self.fail("syntax error");
            return false;
        }
        self.next();
        true
    }
    /// 676090.
    fn statement(&mut self) -> Result<()> {
        match self.kind() {
            lex::IDENT => {
                if self.find_record(&self.cur.text.clone()).is_some() {
                    self.declarations(self.scope);
                    return Ok(());
                }
                let first = self.cur.clone();
                self.next();
                if self.kind() == lex::COLON {
                    let pc = self.code.len() as i64;
                    let l = self.label(first.text, pc);
                    self.current_label = Some(l);
                    self.next();
                    return Ok(());
                }
                let cur = std::mem::take(&mut self.cur);
                self.lex.unget(cur);
                self.lex.unget(first);
                self.next();
                let e = self.assign();
                if self.kind() != lex::SEMI {
                    self.fail("syntax error");
                    return Ok(());
                }
                self.next();
                self.emit_statement(e, true)?;
            }
            lex::OTHER if self.cur.first() == 64 => {
                let Some(owner) = self.current_label else {
                    self.fail("syntax error");
                    return Ok(());
                };
                self.next();
                let local = self.cur.text.clone();
                self.next();
                if self.kind() != lex::COLON {
                    self.fail("syntax error");
                    return Ok(());
                }
                let mut name = self.labels[owner].name.clone();
                name.push(64);
                name.extend_from_slice(&local);
                let pc = self.code.len() as i64;
                self.label(name, pc);
                self.next();
            }
            lex::LBRACE => {
                self.next();
                self.block()?;
                self.expect(lex::RBRACE);
            }
            lex::SEMI => self.next(),
            22 | 23 => {
                let e = self.assign();
                if self.kind() != lex::SEMI {
                    self.fail("syntax error");
                    return Ok(());
                }
                self.next();
                self.emit_statement(e, true)?;
            }
            // do
            63 => {
                self.next();
                let start = self.internal_label(self.code.len() as i64);
                self.top()?;
                if !self.expect(86) || !self.expect(lex::LPAREN) {
                    return Ok(());
                }
                let cond = self.assign();
                if !self.expect(lex::RPAREN) || !self.expect(lex::SEMI) {
                    return Ok(());
                }
                self.emit_statement(cond, false)?;
                self.jump(4, start);
            }
            // extern int long static void: declarations / definitions
            66 | 71 | 72 | 78 | 84 => self.declaration_statement()?,
            // for
            68 => {
                self.next();
                if !self.expect(lex::LPAREN) {
                    return Ok(());
                }
                let init = self.assign();
                if !self.expect(lex::SEMI) {
                    return Ok(());
                }
                let cond = self.assign();
                if !self.expect(lex::SEMI) {
                    return Ok(());
                }
                let step = self.assign();
                if !self.expect(lex::RPAREN) {
                    return Ok(());
                }
                self.emit_statement(init, true)?;
                let head = self.internal_label(self.code.len() as i64);
                let exit = self.internal_label(-1);
                self.emit_statement(cond, false)?;
                self.jump(5, exit);
                self.top()?;
                self.emit_statement(step, true)?;
                self.jump(3, head);
                self.labels[exit].addr = self.code.len() as i64;
            }
            // goto
            69 => {
                self.next();
                let Some(l) = self.label_ref() else { return Ok(()) };
                if self.kind() != lex::SEMI {
                    self.fail("syntax error");
                    return Ok(());
                }
                self.next();
                self.jump(3, l);
            }
            // if
            70 => {
                self.next();
                if !self.expect(lex::LPAREN) {
                    return Ok(());
                }
                let cond = self.assign();
                if !self.expect(lex::RPAREN) {
                    return Ok(());
                }
                self.emit_statement(cond, false)?;
                let otherwise = self.internal_label(-1);
                self.jump(5, otherwise);
                let after = self.internal_label(-1);
                self.jump(3, after);
                self.labels[after].addr = self.code.len() as i64;
                self.top()?;
                if self.kind() == 65 {
                    self.next();
                    let end = self.internal_label(-1);
                    self.jump(3, end);
                    self.labels[otherwise].addr = self.code.len() as i64;
                    self.top()?;
                    self.labels[end].addr = self.code.len() as i64;
                } else {
                    self.labels[otherwise].addr = self.code.len() as i64;
                }
            }
            // return
            74 => {
                self.next();
                if self.kind() == lex::SEMI {
                    self.epilogue(false);
                } else {
                    let e = self.assign();
                    if self.kind() != lex::SEMI {
                        self.fail("syntax error");
                        return Ok(());
                    }
                    self.next();
                    self.emit_statement(e, false)?;
                    self.epilogue(true);
                }
            }
            // struct, union
            79 | 82 => self.record(),
            // switch: only the expression is evaluated.
            80 => {
                self.next();
                if !self.expect(lex::LPAREN) {
                    return Ok(());
                }
                let e = self.assign();
                if !self.expect(lex::RPAREN) {
                    return Ok(());
                }
                self.emit_statement(e, false)?;
            }
            // while
            86 => {
                self.next();
                if !self.expect(lex::LPAREN) {
                    return Ok(());
                }
                let cond = self.assign();
                if !self.expect(lex::RPAREN) {
                    return Ok(());
                }
                let body = self.internal_label(-1);
                let test = self.internal_label(-1);
                self.jump(3, test);
                self.labels[body].addr = self.code.len() as i64;
                self.top()?;
                self.labels[test].addr = self.code.len() as i64;
                self.emit_statement(cond, false)?;
                self.jump(4, body);
            }
            _ => self.fail("syntax error"),
        }
        Ok(())
    }
    fn declaration_statement(&mut self) -> Result<()> {
        if self.declarations(self.scope) {
            let f = self.current_fn.expect("declarations found a function body");
            let name = self.syms[f].name.clone();
            let pc = self.code.len();
            let l = self.label(name.clone(), pc as i64);
            self.current_label = Some(l);
            self.link_set(&name, 3, pc as u32);
            let saved = self.scope;
            self.scope = self.syms[f].ty.scope.unwrap_or(self.scope);
            self.next();
            self.begin();
            self.op(49);
            let frame = self.u32(0);
            self.end();
            self.in_body = true;
            self.block()?;
            self.in_body = false;
            if self.kind() == lex::RBRACE {
                self.next();
                self.epilogue(false);
                let data = self.scopes[self.scope].data as u32;
                self.patch(frame, data);
                self.scope = saved;
            } else {
                self.fail("syntax error");
            }
            self.current_fn = None;
        } else if !self.in_body {
            if let Some(f) = self.current_fn {
                if self.syms[f].storage == 0 {
                    let name = self.syms[f].name.clone();
                    let l = self.label(name.clone(), -1);
                    self.current_label = Some(l);
                    self.link_set(&name, 3, u32::MAX);
                }
            }
        }
        Ok(())
    }
    /// 676000.
    fn epilogue(&mut self, value: bool) {
        let Some(f) = self.current_fn else { return };
        self.begin();
        if !value {
            self.push_int(0);
        }
        match self.syms[f].conv {
            1 => self.op(50),
            2 => {
                self.op(51);
                let n = self.scopes[self.scope].params as u8;
                self.u8(n);
            }
            _ => {}
        }
        self.end();
    }

    // ------------------------------------------------------ declarations
    fn find_record(&self, name: &[u16]) -> Option<usize> {
        self.records.iter().position(|r| r.name == name)
    }
    /// 68A6E0: true when a function body `{` follows.
    fn declarations(&mut self, scope: usize) -> bool {
        loop {
            loop {
                let k = self.kind();
                let class = keyword_class(k);
                if class != 2 && class != 3 && !(k == lex::IDENT && self.find_record(&self.cur.text.clone()).is_some()) {
                    return false;
                }
                let before = (self.lex.line(), self.cur.clone());
                self.declaration(scope);
                // The original loops forever when nothing was consumed
                // (e.g. `struct` inside a declaration list).
                if (self.lex.line(), self.cur.clone()) == before {
                    self.fail("declaration does not advance (the original loops forever)");
                    return false;
                }
                if self.kind() != lex::SEMI {
                    break;
                }
                self.next();
                if self.current_fn.is_some() {
                    return false;
                }
            }
            if self.kind() == lex::LBRACE {
                return true;
            }
        }
    }
    /// 68A9A0: (type, storage, qualifier).
    fn specifiers(&mut self) -> (Option<Type>, i32, i32) {
        let mut t: Option<Type> = None;
        let mut storage = 0;
        let mut qual = 0;
        let finish = |t: Option<Type>| match t {
            Some(t) if t.kind == 14 => Some(basic(6)),
            Some(t) if t.kind == 15 => Some(basic(7)),
            t => t,
        };
        let mut first = true;
        loop {
            let k = self.kind();
            if !(1..=85).contains(&k) {
                return if first { (t, storage, qual) } else { (finish(t), storage, qual) };
            }
            first = false;
            match k {
                lex::IDENT => {
                    if t.is_some() {
                        return (finish(t), storage, qual);
                    }
                    match self.find_record(&self.cur.text.clone()) {
                        Some(r) => {
                            let size = self.scopes[self.records[r].scope].data;
                            t = Some(Type { kind: 16, size, record: Some(r), ..Default::default() });
                        }
                        None => {
                            self.error("type specifier");
                            return (None, storage, qual);
                        }
                    }
                }
                56 => storage = 1,
                59 | 71 | 72 | 75 => match &t {
                    Some(x) if x.kind == 15 => {
                        t = Some(basic(match k {
                            59 => 3,
                            71 => 7,
                            72 => 9,
                            _ => 5,
                        }))
                    }
                    Some(x) if x.kind == 14 => {}
                    Some(_) => {
                        self.error("type specifier");
                        return (None, storage, qual);
                    }
                    None => {
                        t = Some(basic(match k {
                            59 => 2,
                            71 => 6,
                            72 => 8,
                            _ => 4,
                        }))
                    }
                },
                60 => qual = 1,
                64 => t = Some(basic(11)),
                66 => storage = 4,
                67 => t = Some(basic(10)),
                73 => storage = 2,
                76 => t = Some(basic(14)),
                78 => storage = 3,
                81 => storage = 5,
                83 => t = Some(basic(15)),
                84 => t = Some(basic(1)),
                85 => qual = 2,
                _ => return (finish(t), storage, qual),
            }
            self.next();
        }
    }
    /// 68A3C0: a declarator chain (kind 21 leaf carries the name).
    fn declarator(&mut self) -> Option<Type> {
        let mut t = match self.kind() {
            lex::IDENT => {
                let d = Type { kind: 21, name: self.cur.text.clone(), ..Default::default() };
                self.next();
                Some(d)
            }
            lex::LPAREN => {
                self.next();
                let d = self.declarator();
                if self.kind() == lex::RPAREN {
                    self.next();
                }
                d
            }
            24 => {
                self.next();
                let inner = self.declarator();
                Some(node(20, 4, inner))
            }
            54 | 55 => {
                let k = if self.kind() == 54 { 12 } else { 13 };
                self.next();
                let inner = self.declarator();
                Some(node(k, 0, inner))
            }
            _ => None,
        };
        if self.kind() == lex::LPAREN {
            self.next();
            let scope = self.new_scope(Some(self.scope));
            while self.kind() != lex::EOF && self.kind() != lex::RPAREN {
                let (spec, _storage, qual) = self.specifiers();
                let Some(spec) = spec else {
                    self.error("parameter declaration");
                    return t;
                };
                if let Some(d) = self.declarator() {
                    let (ty, name) = combine(d, spec, &mut 0);
                    if self.local(scope, &name).is_some() {
                        self.error(format!("{} is declared twice", text(&name)));
                    } else {
                        self.declare(scope, name, ty, 6, qual, 0);
                    }
                }
                if self.kind() == lex::COMMA {
                    self.next();
                }
            }
            if self.kind() == lex::RPAREN {
                self.next();
            }
            let mut f = node(19, 0, t);
            f.scope = Some(scope);
            t = Some(f);
        }
        while self.kind() == lex::LBRACKET {
            self.next();
            if self.kind() == lex::RBRACKET {
                // The original sizes `[]` with a string address.
                self.error("unsized array");
                return t;
            }
            let n = self.assign();
            if self.kind() == lex::RBRACKET {
                self.next();
            }
            let size = match n.as_ref().map(const_eval) {
                Some(Ok(v)) => v,
                Some(Err(e)) => {
                    self.error(format!("array size: {e}"));
                    0
                }
                None => {
                    self.error("array size");
                    0
                }
            };
            t = Some(node(18, size, t));
        }
        t
    }
    /// 68A770.
    fn declaration(&mut self, scope: usize) {
        let (spec, mut storage, qual) = self.specifiers();
        if storage == 0 {
            storage = i32::from(self.scopes[scope].parent.is_some());
        }
        let Some(spec) = spec else { return };
        // 68A770 initialises the convention once for the whole list.
        let mut conv = 1;
        loop {
            let Some(d) = self.declarator() else { return };
            let (ty, name) = combine(d, spec.clone(), &mut conv);
            if name.is_empty() {
                return;
            }
            match self.local(scope, &name) {
                Some(e) => {
                    let ex = &self.syms[e];
                    if ex.ty.kind != 19 || ex.conv == 0 {
                        self.error(format!("{} is declared twice", text(&name)));
                    } else if !same_type(&ex.ty, &ty) || ex.storage != storage || ex.qual != qual || ex.conv != conv {
                        self.error(format!("{} is declared differently", text(&name)));
                    } else {
                        self.current_fn = Some(e);
                    }
                }
                None => {
                    let function = ty.kind == 19;
                    let id = self.declare(scope, name.clone(), ty, storage, qual, conv);
                    if function {
                        self.syms[id].offset = self.link_add(&name);
                        self.current_fn = Some(id);
                    }
                }
            }
            if self.kind() != lex::COMMA {
                return;
            }
            self.next();
        }
    }
    /// 68ACD0.
    fn record(&mut self) {
        self.next();
        if self.kind() != lex::IDENT {
            return;
        }
        let name = self.cur.text.clone();
        if self.find_record(&name).is_some() {
            self.error(format!("struct {} is defined twice", text(&name)));
            return;
        }
        let scope = self.new_scope(None);
        self.records.push(Record { name, scope });
        self.next();
        if self.kind() == lex::LBRACE {
            self.next();
            self.declarations(scope);
            if self.kind() != lex::RBRACE {
                self.error("struct body");
                return;
            }
            self.next();
        }
        if self.kind() == lex::SEMI {
            self.next();
        } else {
            self.error("';' expected");
        }
    }

    // -------------------------------------------------------- expressions
    /// 689780.
    fn assign(&mut self) -> Option<Node> {
        let lhs = self.conditional();
        let k = match self.kind() {
            42 => 124,
            43 => 104,
            44 => 105,
            45 => 106,
            46 => 107,
            47 => 108,
            48 => 110,
            49 => 109,
            50 => 117,
            51 => 118,
            52 => 119,
            _ => return lhs,
        };
        self.next();
        let mut rhs = self.assign();
        if k != 124 {
            rhs = Some(op(k, lhs.clone(), rhs));
        }
        Some(op(124, lhs, rhs))
    }
    /// 68A250.
    fn conditional(&mut self) -> Option<Node> {
        let c = self.binary(0);
        if self.kind() != 53 {
            return c;
        }
        self.next();
        let t = self.assign();
        if t.is_some() && self.kind() == lex::COLON {
            self.next();
            if let Some(f) = self.conditional() {
                return Some(op(122, c, Some(op(123, t, Some(f)))));
            }
        }
        self.error("conditional expression");
        None
    }
    /// 689850..689890 by precedence level.
    fn binary(&mut self, level: usize) -> Option<Node> {
        const LEVELS: [&[(i32, u8)]; 10] = [
            &[(41, 121)],
            &[(40, 120)],
            &[(39, 119)],
            &[(38, 118)],
            &[(37, 117)],
            &[(35, 115), (36, 116)],
            &[(31, 111), (32, 113), (33, 112), (34, 114)],
            &[(29, 110), (30, 109)],
            &[(27, 107), (28, 108)],
            &[(24, 104), (25, 105), (26, 106)],
        ];
        let sub = |s: &mut Self| if level + 1 < LEVELS.len() { s.binary(level + 1) } else { s.unary() };
        let mut l = sub(self);
        while let Some(&(_, k)) = LEVELS[level].iter().find(|(t, _)| *t == self.kind()) {
            self.next();
            let r = sub(self);
            l = Some(op(k, l, r));
        }
        l
    }
    /// 689FA0.
    fn unary(&mut self) -> Option<Node> {
        let k = match self.kind() {
            20 => 94,
            21 => 99,
            24 => 95,
            27 => 97,
            28 => 98,
            37 => 96,
            22 | 23 => {
                let k = if self.kind() == 22 { 102 } else { 103 };
                self.next();
                let n = self.postfix();
                return Some(op(k, n, None));
            }
            77 => {
                self.next();
                if self.kind() != lex::LPAREN {
                    self.error("sizeof");
                    return None;
                }
                self.next();
                if self.kind() != lex::IDENT {
                    self.error("sizeof");
                    return None;
                }
                let Some(s) = self.lookup(&self.cur.text.clone()) else {
                    self.error("sizeof: unknown identifier");
                    return None;
                };
                self.next();
                if self.kind() != lex::RPAREN {
                    self.error("sizeof");
                    return None;
                }
                self.next();
                return Some(Node::Int(sizeof(&self.syms[s].ty) as u32));
            }
            _ => {
                let n = self.postfix();
                let k = match self.kind() {
                    22 => 100,
                    23 => 101,
                    _ => return n,
                };
                self.next();
                return Some(op(k, n, None));
            }
        };
        self.next();
        let n = self.unary();
        Some(op(k, n, None))
    }
    /// 689B60.
    fn postfix(&mut self) -> Option<Node> {
        let mut ty: Option<Type> = None;
        let mut base = match self.kind() {
            lex::IDENT => {
                let name = self.cur.text.clone();
                match self.lookup(&name) {
                    Some(s) => ty = Some(self.syms[s].ty.clone()),
                    None => {
                        self.error(format!("{} is not declared", text(&name)));
                        let scope = self.scope;
                        self.declare(scope, name.clone(), basic(6), 0, 0, 0);
                    }
                }
                self.next();
                Some(Node::Ident(name))
            }
            lex::NUMBER => {
                let v = wtoi(&self.cur.text);
                self.next();
                Some(Node::Int(v as u32))
            }
            lex::STRING => {
                // 6905A0 copies the measured text.
                let n = lex::text_len(&self.cur.text).min(self.cur.text.len());
                let s = self.cur.text[..n].to_vec();
                self.next();
                Some(Node::Str(s))
            }
            lex::LPAREN => {
                self.next();
                let e = self.assign();
                self.next();
                e
            }
            _ => None,
        };
        let mut offset: Option<Node> = None;
        loop {
            match self.kind() {
                lex::LPAREN => {
                    self.next();
                    let first = self.assign();
                    let mut args = op(92, first, None);
                    while self.kind() == lex::COMMA {
                        self.next();
                        let a = self.assign();
                        args = op(92, a, Some(args));
                    }
                    if self.kind() != lex::RPAREN {
                        self.error("')' expected");
                        return base;
                    }
                    self.next();
                    base = Some(op(91, base, Some(args)));
                }
                lex::LBRACKET => {
                    self.next();
                    let Some(elem) = ty.as_ref().filter(|t| t.kind == 18).and_then(|t| t.sub.as_deref().cloned()) else {
                        self.error("subscript");
                        return base;
                    };
                    let size = Node::Int(sizeof(&elem) as u32);
                    let index = self.assign();
                    let term = op(104, index, Some(size));
                    offset = Some(match offset {
                        Some(o) => op(107, Some(o), Some(term)),
                        None => term,
                    });
                    if self.kind() != lex::RBRACKET {
                        self.error("subscript");
                        return base;
                    }
                    self.next();
                    ty = Some(elem);
                }
                lex::ARROW | lex::DOT => {
                    if self.kind() == lex::ARROW {
                        match ty.as_ref().filter(|t| t.kind == 20).and_then(|t| t.sub.as_deref().cloned()) {
                            Some(p) => ty = Some(p),
                            None => {
                                self.error("member access");
                                return base;
                            }
                        }
                    }
                    self.next();
                    let record = ty.as_ref().filter(|t| t.kind == 16).and_then(|t| t.record);
                    let (Some(r), true) = (record, self.kind() == lex::IDENT) else {
                        self.error("member access");
                        return base;
                    };
                    let Some(m) = self.local(self.records[r].scope, &self.cur.text) else {
                        self.error(format!("no member {}", text(&self.cur.text)));
                        return base;
                    };
                    self.next();
                    let term = Node::Int(self.syms[m].offset as u32);
                    offset = Some(match offset {
                        Some(o) => op(107, Some(o), Some(term)),
                        None => term,
                    });
                    ty = Some(self.syms[m].ty.clone());
                }
                _ => {
                    return match offset {
                        Some(o) => Some(op(93, base, Some(o))),
                        None => base,
                    };
                }
            }
        }
    }

    // -------------------------------------------------------------- codegen
    /// 679BA0 (discard) / 679BD0 (value).
    fn emit_statement(&mut self, e: Option<Node>, discard: bool) -> Result<()> {
        let Some(mut e) = e else { bail!("missing expression (the original dereferences null)") };
        fold(&mut e)?;
        self.begin();
        self.emit(&e)?;
        if discard {
            self.op(14);
        }
        self.end();
        Ok(())
    }
    fn symbol(&self, n: &Node) -> Result<usize> {
        match n {
            Node::Ident(name) => self.lookup(name).ok_or_else(|| anyhow::anyhow!("{} is not declared", text(name))),
            _ => bail!("the original reads a symbol from a non-identifier node"),
        }
    }
    /// 679A30.
    fn address(&mut self, n: &Node) -> Result<()> {
        match n {
            Node::Ident(_) => {
                let s = &self.syms[self.symbol(n)?];
                let (storage, offset) = (s.storage, s.offset);
                match storage {
                    1 => {
                        self.push_int(offset as u32);
                        self.store_op = 47;
                    }
                    6 => {
                        self.push_int(offset.wrapping_add(8) as u32);
                        self.store_op = 47;
                    }
                    _ => {
                        self.push_int(offset as u32);
                        self.store_op = 7;
                    }
                }
            }
            Node::Op(93, a, b) => {
                self.emit_opt(b)?;
                self.address_opt(a)?;
                self.op(20);
            }
            _ => {}
        }
        Ok(())
    }
    fn address_opt(&mut self, n: &Option<Box<Node>>) -> Result<()> {
        match n {
            Some(n) => self.address(n),
            None => bail!("missing operand (the original dereferences null)"),
        }
    }
    fn emit_opt(&mut self, n: &Option<Box<Node>>) -> Result<()> {
        match n {
            Some(n) => self.emit(n),
            None => bail!("missing operand (the original dereferences null)"),
        }
    }
    fn load_op(&self) -> u8 {
        if self.store_op == 7 { 6 } else { 46 }
    }
    /// 6792E0.
    fn emit(&mut self, n: &Node) -> Result<()> {
        match n {
            Node::Ident(_) => {
                let s = &self.syms[self.symbol(n)?];
                let (array, storage, offset) = (s.ty.kind == 18, s.storage, s.offset);
                if array {
                    self.push_int(offset as u32);
                    self.op(43);
                } else if storage == 1 {
                    self.push_int(offset as u32);
                    self.op(46);
                } else if storage == 6 {
                    self.push_int(offset.wrapping_add(8) as u32);
                    self.op(46);
                } else {
                    self.push_int(offset as u32);
                    self.op(6);
                }
            }
            Node::Int(v) => self.push_int(*v),
            Node::Str(s) => {
                let s = s.clone();
                self.push_string(&s);
            }
            Node::Op(k, a, b) => match *k {
                91 => self.call(a, b)?,
                92 => {
                    self.emit_opt(a)?;
                    if let Some(b) = b {
                        self.emit(b)?;
                    }
                }
                93 => {
                    self.emit_opt(b)?;
                    self.address_opt(a)?;
                    self.op(20);
                    self.op(6);
                }
                94 => {
                    self.emit_opt(a)?;
                    self.op(32);
                }
                95 | 97 => self.emit_opt(a)?,
                96 => self.address_opt(a)?,
                98 => {
                    self.emit_opt(a)?;
                    self.op(19);
                }
                99 => {
                    self.emit_opt(a)?;
                    self.op(28);
                }
                100 | 101 => {
                    self.address_opt(a)?;
                    let l = self.load_op();
                    self.op(l);
                    self.op(18);
                    self.op(if *k == 100 { 41 } else { 42 });
                    self.address_opt(a)?;
                    let s = self.store_op;
                    self.op(s);
                    self.op(14);
                }
                102 | 103 => {
                    self.address_opt(a)?;
                    let l = self.load_op();
                    self.op(l);
                    self.op(if *k == 102 { 41 } else { 42 });
                    self.address_opt(a)?;
                    let s = self.store_op;
                    self.op(s);
                }
                104..=121 => {
                    const OPS: [u8; 18] = [22, 23, 24, 20, 21, 40, 39, 33, 35, 34, 36, 37, 38, 25, 27, 26, 30, 31];
                    self.emit_opt(a)?;
                    self.emit_opt(b)?;
                    self.op(OPS[(*k - 104) as usize]);
                }
                122 => {
                    self.emit_opt(a)?;
                    self.op(2);
                    let skip = self.u32(0);
                    let Some(Node::Op(123, t, f)) = b.as_deref() else { bail!("malformed ?: node") };
                    self.emit_opt(t)?;
                    self.op(0);
                    let end = self.u32(0);
                    let here = self.rel();
                    self.patch(skip, here);
                    self.emit_opt(f)?;
                    let here = self.rel();
                    self.patch(end, here);
                }
                124 => {
                    self.emit_opt(b)?;
                    self.address_opt(a)?;
                    let s = self.store_op;
                    self.op(s);
                }
                _ => {}
            },
        }
        Ok(())
    }
    /// 678B70.
    fn call(&mut self, callee: &Option<Box<Node>>, args: &Option<Box<Node>>) -> Result<()> {
        let mut count = 0u32;
        let mut a = args.as_deref();
        while let Some(Node::Op(92, Some(_), rest)) = a {
            count += 1;
            a = rest.as_deref();
        }
        let Some(callee) = callee else { bail!("call without a callee (the original dereferences null)") };
        let f = self.symbol(callee)?;
        let (storage, index, conv) = (self.syms[f].storage, self.syms[f].offset as u32, self.syms[f].conv);
        if count > 0 {
            self.emit_opt(args)?;
        }
        if storage == 4 {
            if index >= 256 {
                self.op(3);
                self.u32(index);
            } else {
                self.op(4);
                self.u8(index as u8);
            }
            self.u8(count as u8);
        } else {
            self.op(45);
            self.u32(index);
            match conv {
                1 => self.u8((4 * count) as u8),
                2 => {
                    let params = self.syms[f].ty.scope.map_or(0, |s| self.scopes[s].params);
                    if (4 * count) as i32 != params {
                        self.error("__stdcall 引数サイズエラー");
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    // ------------------------------------------------------------- driver
    pub fn compile(mut self) -> Result<Program> {
        while self.kind() != lex::EOF {
            self.top()?;
        }
        // 675AE0: STOP, fixups, unresolved labels.
        self.begin();
        self.op(5);
        self.end();
        for l in &self.labels {
            for &at in &l.fixups {
                self.code[at..at + 4].copy_from_slice(&(l.addr as u32).to_le_bytes());
            }
        }
        let unresolved: Vec<String> = self.labels.iter().filter(|l| l.addr == -1).map(|l| text(&l.name)).collect();
        for n in unresolved {
            self.error(format!("label {n} is not defined"));
        }
        let mut errors: Vec<String> = self.lex.errors.iter().map(|d| format!("{}({}): {}", d.file, d.line, d.message)).collect();
        errors.extend(self.errors);
        if !errors.is_empty() {
            bail!("Malie script compile errors:\n{}", errors.join("\n"));
        }
        let symbols = self.scopes[self.global]
            .list
            .iter()
            .map(|&i| {
                let s = &self.syms[i];
                ProgramSymbol { name: text(&s.name), kind: s.storage as u32, size: s.qual as u32, offset: s.offset as u32, convention: s.conv as u32 }
            })
            .collect();
        let functions = self.links.iter().enumerate().map(|(i, l)| Function { name: text(&l.name), index: i as u32, kind: l.kind, pc: l.pc }).collect();
        let labels: BTreeMap<String, u32> = self.labels.iter().map(|l| (text(&l.name), l.addr as u32)).collect();
        Ok(Program {
            symbols,
            functions,
            labels,
            global_size: self.scopes[self.global].data as u32,
            strings: self.strings,
            code: self.code,
            message_ranges: self.ranges,
            messages: self.messages,
        })
    }
}

/// 68B240: reverse a declarator around the specifier type.
fn combine(d: Type, spec: Type, conv: &mut i32) -> (Type, W) {
    let mut out = spec;
    let mut cur = Some(d);
    while let Some(mut n) = cur {
        match n.kind {
            21 => return (out, n.name),
            12 | 13 => {
                *conv = if n.kind == 12 { 1 } else { 2 };
                cur = n.sub.take().map(|b| *b);
            }
            _ => {
                let next = n.sub.take().map(|b| *b);
                n.sub = Some(Box::new(out));
                out = n;
                cur = next;
            }
        }
    }
    (out, W::new())
}

/// _wtoi.
fn wtoi(t: &[u16]) -> i32 {
    let s = text(t);
    let s = s.trim_start();
    let (neg, digits) = match s.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, s.strip_prefix('+').unwrap_or(s)),
    };
    let mut v: i64 = 0;
    for c in digits.chars() {
        let Some(d) = c.to_digit(10) else { break };
        v = (v * 10 + i64::from(d)).min(i64::from(i32::MAX) + 1);
    }
    let v = if neg { -v } else { v };
    v.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

/// 677E00 without a symbol context (array dimensions).
fn const_eval(n: &Node) -> Result<i32> {
    let r = |n: &Option<Box<Node>>| -> Result<i32> {
        match n {
            Some(n) => const_eval(n),
            None => bail!("missing operand"),
        }
    };
    Ok(match n {
        Node::Int(v) => *v as i32,
        Node::Ident(_) | Node::Str(_) => bail!("not a constant"),
        Node::Op(k, a, b) => match *k {
            92 => 0,
            94 => i32::from(r(a)? == 0),
            97 => r(a)?,
            98 => r(a)?.wrapping_neg(),
            99 => !r(a)?,
            104 => r(b)?.wrapping_mul(r(a)?),
            105 | 106 => {
                let (x, y) = (r(a)? as u32, r(b)? as u32);
                if y == 0 {
                    bail!("division by zero");
                }
                (if *k == 105 { x / y } else { x % y }) as i32
            }
            107 => r(b)?.wrapping_add(r(a)?),
            108 => r(a)?.wrapping_sub(r(b)?),
            109 => r(a)?.wrapping_shr(r(b)? as u32),
            110 => r(a)?.wrapping_shl(r(b)? as u32),
            111 => i32::from((r(a)? as u32) < r(b)? as u32),
            112 => i32::from((r(a)? as u32) > r(b)? as u32),
            113 => i32::from((r(a)? as u32) <= r(b)? as u32),
            114 => i32::from((r(a)? as u32) >= r(b)? as u32),
            115 => i32::from(r(a)? == r(b)?),
            116 => i32::from(r(a)? != r(b)?),
            117 => r(b)? & r(a)?,
            118 => r(b)? ^ r(a)?,
            119 => r(b)? | r(a)?,
            120 => i32::from(r(a)? != 0 && r(b)? != 0),
            121 => i32::from(r(a)? != 0 || r(b)? != 0),
            122 | 123 => 0,
            _ => bail!("not a constant"),
        },
    })
}

/// 678C60: in-place folding; true when this node became a constant.
fn fold(n: &mut Node) -> Result<bool> {
    loop {
        let Node::Op(k, a, b) = n else { return Ok(false) };
        let k = *k;
        let (x, y) = (is_int(a), is_int(b));
        let value: Option<u32> = match k {
            93 => {
                if !fold_opt(b)? {
                    return Ok(false);
                }
                continue;
            }
            94 => x.map(|v| u32::from(v == 0)),
            95 => {
                fold_opt(a)?;
                return Ok(false);
            }
            97 => x,
            98 => x.map(u32::wrapping_neg),
            99 => x.map(|v| !v),
            104..=121 => match (x, y) {
                (Some(x), Some(y)) => Some(match k {
                    104 => y.wrapping_mul(x),
                    105 | 106 => {
                        if y == 0 {
                            bail!("constant division by zero");
                        }
                        if k == 105 { x / y } else { x % y }
                    }
                    107 => y.wrapping_add(x),
                    108 => x.wrapping_sub(y),
                    109 => (x as i32).wrapping_shr(y) as u32,
                    110 => x.wrapping_shl(y),
                    111 => u32::from(x < y),
                    112 => u32::from(x > y),
                    113 => u32::from(x <= y),
                    114 => u32::from(x >= y),
                    115 => u32::from(x == y),
                    116 => u32::from(x != y),
                    117 => y & x,
                    118 => y ^ x,
                    119 => y | x,
                    120 => u32::from(x != 0 && y != 0),
                    _ => u32::from(x != 0 || y != 0),
                }),
                _ => {
                    if !fold_opt(a)? && !fold_opt(b)? {
                        return Ok(false);
                    }
                    continue;
                }
            },
            124 => {
                // 6792C0 folds the target's subscript chain and reports 0.
                if let Some(t) = a.as_deref_mut() {
                    while let Node::Op(93, _, tb) = t {
                        if !fold_opt(tb)? {
                            break;
                        }
                    }
                }
                if !fold_opt(b)? {
                    return Ok(false);
                }
                continue;
            }
            _ => return Ok(false),
        };
        match value {
            Some(v) => {
                *n = Node::Int(v);
                return Ok(true);
            }
            None => {
                if !fold_opt(a)? {
                    return Ok(false);
                }
            }
        }
    }
}
fn fold_opt(n: &mut Option<Box<Node>>) -> Result<bool> {
    match n {
        Some(n) => fold(n),
        None => bail!("missing operand (the original dereferences null)"),
    }
}
