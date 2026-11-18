// SPDX-License-Identifier: MPL-2.0
//! Malie script lexer and preprocessor (Ragnarok `malie.exe`).
//!
//! Character stream 685D60/687820 (2048-entry push-back stack, `\r`
//! skipped, line counting), raw scanner 685DC0, operator/keyword stage
//! 686AE0 + 687AC0, and the preprocessor/lexer 6886E0 with `#include`
//! (687E30), `#define` (687C60/6893B0), macro expansion (6880E0/689550),
//! `#undef`, `#ifdef`/`#ifndef`/`#else`/`#endif` (687DE0/687DA0/687DD0/
//! 6891B0), `#line` (689630), string concatenation and escapes (688E90)
//! and text runs (688D80). Token text is UTF-16 like the original.
use super::source::Source;
use std::collections::HashMap;

pub type W = Vec<u16>;
pub fn w(s: &str) -> W {
    s.encode_utf16().collect()
}
pub fn text(v: &[u16]) -> String {
    String::from_utf16_lossy(v)
}

// Token kinds (686AE0, 6886E0).
pub const EOF: i32 = 0;
pub const IDENT: i32 = 1;
pub const NUMBER: i32 = 2;
pub const STRING: i32 = 3;
pub const OTHER: i32 = 4;
pub const TEXT: i32 = 5;
pub const HASH: i32 = 6;
pub const LPAREN: i32 = 9;
pub const RPAREN: i32 = 10;
pub const LBRACKET: i32 = 11;
pub const RBRACKET: i32 = 12;
pub const LBRACE: i32 = 13;
pub const RBRACE: i32 = 14;
pub const COMMA: i32 = 15;
pub const SEMI: i32 = 16;
pub const COLON: i32 = 17;
pub const ARROW: i32 = 18;
pub const DOT: i32 = 19;
pub const MSTRING: i32 = 126;
pub const MEND: i32 = 127;

/// 7B5CC0: sorted keywords; the token is 54 + index.
pub const KEYWORDS: [&str; 33] = [
    "__cdecl", "__stdcall", "auto", "break", "case", "char", "const", "continue", "default", "do", "double", "else", "extern", "float", "for", "goto", "if", "int", "long", "register", "return", "short", "signed", "sizeof", "static", "struct", "switch", "typedef", "union", "unsigned", "void", "volatile", "while",
];
/// Keyword classes (7B5CC0 second column): 1 statement, 2 storage, 3 type,
/// 4 qualifier, 5 calling convention. 687AA0 only answers for 56..86.
pub const KEYWORD_CLASS: [u8; 33] = [5, 5, 2, 1, 1, 3, 4, 1, 1, 1, 3, 1, 2, 3, 1, 1, 1, 3, 3, 2, 1, 3, 3, 1, 2, 3, 1, 2, 3, 3, 3, 4, 1];
pub fn keyword_class(kind: i32) -> u8 {
    if (56..=86).contains(&kind) { KEYWORD_CLASS[(kind - 54) as usize] } else { 0 }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Token {
    pub kind: i32,
    pub text: W,
}
impl Token {
    pub fn first(&self) -> u16 {
        self.text.first().copied().unwrap_or(0)
    }
}

/// One open source file: decoded characters and the current character.
pub struct Stream {
    chars: Vec<i64>,
    at: usize,
    pushback: Vec<u16>,
    pub c: i64,
    pub line: i64,
}
const PUSHBACK: usize = 2048;
impl Stream {
    pub fn new(chars: Vec<i64>) -> Self {
        let mut s = Self { chars, at: 0, pushback: Vec::new(), c: 0, line: 0 };
        // 685CA0: the first character is fetched without \r or line logic.
        s.c = s.source();
        s
    }
    fn source(&mut self) -> i64 {
        match self.chars.get(self.at) {
            Some(&c) => {
                self.at += 1;
                c
            }
            None => -1,
        }
    }
    /// 685D60.
    pub fn next(&mut self) {
        loop {
            self.c = match self.pushback.pop() {
                Some(c) => i64::from(c),
                None => self.source(),
            };
            if self.c == 10 {
                self.line += 1;
            }
            if self.c != 13 {
                break;
            }
        }
    }
    /// 687820. A full stack drops the character (the original only logs).
    pub fn unget(&mut self, ch: u16) {
        if self.pushback.len() >= PUSHBACK {
            log::warn!("Malie compiler push-back overflow");
            return;
        }
        self.pushback.push(self.c as u16);
        if self.c == 10 {
            self.line -= 1;
        }
        self.c = i64::from(ch);
    }
    /// 687860: push a string so that it is read back in order.
    pub fn unget_str(&mut self, s: &[u16]) {
        for &ch in s.iter().rev() {
            self.unget(ch);
        }
    }
    /// 6878B0: strings return with their quotes.
    pub fn unget_token(&mut self, t: &Token) {
        if t.kind == STRING {
            self.unget(34);
            self.unget_str(&t.text);
            self.unget(34);
        } else {
            self.unget_str(&t.text);
        }
    }
    /// 687800.
    fn at_end(&self) -> bool {
        self.pushback.is_empty() && self.c == -1
    }
    /// 687A00.
    fn skip_blanks(&mut self) {
        while self.c == 32 || self.c == 9 {
            self.next();
        }
    }
    /// 6878F0: drop the rest of the line including the newline.
    fn skip_line(&mut self) {
        while !self.at_end() && self.c != 10 {
            self.next();
        }
        self.next();
    }
}

fn is_alpha(c: i64) -> bool {
    (0..0x80).contains(&c) && (c as u8).is_ascii_alphabetic()
}
fn is_alnum(c: i64) -> bool {
    if c < 0 {
        return false;
    }
    if c < 0x80 {
        return (c as u8).is_ascii_alphanumeric();
    }
    char::from_u32(c as u32).is_some_and(char::is_alphanumeric)
}
fn is_digit(c: i64) -> bool {
    if c < 0 {
        return false;
    }
    if c < 0x80 {
        return (c as u8).is_ascii_digit();
    }
    char::from_u32(c as u32).is_some_and(char::is_numeric)
}
fn is_xdigit(c: i64) -> bool {
    (0..0x80).contains(&c) && (c as u8).is_ascii_hexdigit()
}

const LIMIT: usize = 2047;
struct Buf {
    text: W,
    big: bool,
}
impl Buf {
    fn push(&mut self, c: i64) {
        if self.text.len() >= LIMIT {
            self.big = true;
        } else {
            self.text.push(c as u16);
        }
    }
}

/// 685DC0: one raw token. Whitespace and newlines are kind 4.
pub fn scan(s: &mut Stream) -> Token {
    let line = s.line;
    if s.c == -1 {
        return Token { kind: EOF, text: W::new() };
    }
    let mut b = Buf { text: W::new(), big: false };
    let done = |b: Buf, kind: i32, s: &mut Stream| -> Token {
        if b.big {
            s.line = line;
            return Token { kind: -1, text: b.text };
        }
        Token { kind, text: b.text }
    };
    loop {
        let c = s.c;
        if c >= 0x80 {
            s.next();
            return Token { kind: TEXT, text: vec![c as u16] };
        }
        if c < 0 {
            // A -1 here can only follow a skipped line comment.
            return Token { kind: EOF, text: W::new() };
        }
        if is_alpha(c) || c == 95 {
            b.push(c);
            s.next();
            while is_alnum(s.c) || s.c == 95 {
                b.push(s.c);
                s.next();
            }
            return done(b, IDENT, s);
        }
        if is_digit(c) {
            b.text.push(c as u16);
            let mut count = 1usize;
            if c == 48 {
                s.next();
                if s.c == 120 || s.c == 88 {
                    b.text.push(s.c as u16);
                    s.next();
                    let mut digits = W::new();
                    while is_xdigit(s.c) {
                        if b.text.len() + digits.len() >= LIMIT {
                            b.big = true;
                        } else {
                            digits.push(s.c as u16);
                        }
                        s.next();
                    }
                    // swscanf("%x") leaves the reused counter (1) when no
                    // digit converts; the result is printed with "%d".
                    let value = if digits.is_empty() { 1 } else { u32::from_str_radix(&text(&digits), 16).map_or(1, |v| v as i32) };
                    b.text = w(&value.to_string());
                    return done(b, NUMBER, s);
                }
            } else {
                s.next();
            }
            let mut dot = false;
            loop {
                if s.c == 46 {
                    if dot {
                        break;
                    }
                    dot = true;
                } else if !is_digit(s.c) {
                    break;
                }
                if count >= LIMIT {
                    b.big = true;
                } else {
                    count += 1;
                    b.text.push(s.c as u16);
                }
                s.next();
            }
            return done(b, NUMBER, s);
        }
        if c == 34 {
            return string(s, b, STRING, line);
        }
        if c == 36 {
            s.next();
            if s.c == 34 {
                return string(s, b, MSTRING, line);
            }
            if s.c == 36 {
                s.next();
                return Token { kind: MEND, text: w("$$") };
            }
            return Token { kind: OTHER, text: w("$") };
        }
        if c == 47 {
            s.next();
            if s.c == 42 {
                // Block comment; a `*` followed by another `*` swallows it.
                loop {
                    s.next();
                    while s.c != -1 && s.c != 42 {
                        b.push(s.c);
                        s.next();
                    }
                    if s.c == -1 {
                        // The original spins forever here.
                        return Token { kind: -1, text: W::new() };
                    }
                    s.next();
                    if s.c == 47 {
                        break;
                    }
                    b.push(s.c);
                }
                s.next();
                if b.big {
                    s.line = line;
                    return Token { kind: -1, text: W::new() };
                }
                return Token { kind: OTHER, text: w(" ") };
            }
            if s.c != 47 {
                return Token { kind: OTHER, text: w("/") };
            }
            s.next();
            while s.c != -1 && s.c != 10 {
                s.next();
            }
        } else {
            s.next();
            if c == 0 {
                return Token { kind: EOF, text: W::new() };
            }
            if c != 92 || s.c != 10 {
                return Token { kind: OTHER, text: vec![c as u16] };
            }
            s.next();
        }
        b.text.clear();
        if s.c == -1 {
            return Token { kind: EOF, text: W::new() };
        }
    }
}
fn string(s: &mut Stream, mut b: Buf, kind: i32, line: i64) -> Token {
    s.next();
    if s.c == 34 {
        s.next();
        return Token { kind, text: W::new() };
    }
    let mut n = 0usize;
    while n < 0x1000 && s.c != 0 && !b.big && s.c != 10 {
        if s.c == 92 {
            b.push(s.c);
            n += 1;
            s.next();
        }
        b.push(s.c);
        n += 1;
        s.next();
        if s.c == 34 {
            s.next();
            return Token { kind, text: b.text };
        }
    }
    s.line = line;
    Token { kind: -1, text: W::new() }
}

/// 686AE0: operators, keywords and line continuations over `scan`.
pub fn token(s: &mut Stream) -> Token {
    loop {
        let mut t = scan(s);
        if t.kind == IDENT {
            t.kind = KEYWORDS.iter().position(|k| w(k) == t.text).map_or(IDENT, |i| 54 + i as i32);
            return t;
        }
        if t.kind != OTHER {
            return t;
        }
        let two = |s: &mut Stream, t: &mut Token, kind: i32| {
            t.text.push(s.c as u16);
            s.next();
            t.kind = kind;
        };
        match t.first() {
            9 | 32 => continue,
            0x21 => if s.c == 61 { two(s, &mut t, 36) } else { t.kind = 20 },
            0x23 => t.kind = 6,
            0x24 => t.kind = 8,
            0x25 => if s.c == 61 { two(s, &mut t, 45) } else { t.kind = 26 },
            0x26 => match s.c {
                38 => two(s, &mut t, 40),
                61 => two(s, &mut t, 50),
                _ => t.kind = 37,
            },
            0x28 => t.kind = LPAREN,
            0x29 => t.kind = RPAREN,
            0x2A => if s.c == 61 { two(s, &mut t, 43) } else { t.kind = 24 },
            0x2B => match s.c {
                43 => two(s, &mut t, 22),
                61 => two(s, &mut t, 46),
                _ => t.kind = 27,
            },
            0x2C => t.kind = COMMA,
            0x2D => match s.c {
                45 => two(s, &mut t, 23),
                61 => two(s, &mut t, 47),
                62 => two(s, &mut t, ARROW),
                _ => t.kind = 28,
            },
            0x2E => t.kind = DOT,
            0x2F => if s.c == 61 { two(s, &mut t, 44) } else { t.kind = 25 },
            0x3A => t.kind = COLON,
            0x3B => t.kind = SEMI,
            0x3C => match s.c {
                60 => {
                    two(s, &mut t, 29);
                    if s.c == 61 {
                        two(s, &mut t, 48);
                    }
                }
                61 => two(s, &mut t, 32),
                _ => t.kind = 31,
            },
            0x3D => if s.c == 61 { two(s, &mut t, 35) } else { t.kind = 42 },
            0x3E => match s.c {
                61 => two(s, &mut t, 34),
                62 => {
                    two(s, &mut t, 30);
                    if s.c == 61 {
                        two(s, &mut t, 49);
                    }
                }
                _ => t.kind = 33,
            },
            0x3F => t.kind = 53,
            0x5B => t.kind = LBRACKET,
            0x5C => {
                let n = scan(s);
                if n.kind == OTHER && n.first() == 10 {
                    continue;
                }
                s.unget_token(&n);
                t.kind = 7;
            }
            0x5D => t.kind = RBRACKET,
            0x7B => t.kind = LBRACE,
            0x7C => match s.c {
                61 => two(s, &mut t, 52),
                124 => two(s, &mut t, 41),
                _ => t.kind = 39,
            },
            0x7D => t.kind = RBRACE,
            0x7E => t.kind = 21,
            _ => {}
        }
        return t;
    }
}

/// Body element of a stored macro (6893B0): literal text, an argument, or
/// a stringised argument.
#[derive(Clone, Debug)]
enum Piece {
    Text(W),
    Arg(usize),
    Quoted(usize),
}
#[derive(Clone, Debug)]
struct Macro {
    params: Option<usize>,
    body: Vec<Piece>,
}

pub struct Diagnostic {
    pub file: String,
    pub line: i64,
    pub message: String,
}

struct File {
    name: W,
    stream: Stream,
}

/// The preprocessor/lexer context (689060's 0x4102C-byte record).
pub struct Lexer<'a> {
    source: &'a mut dyn Source,
    file: File,
    includes: Vec<File>,
    pub look: Token,
    macros: HashMap<W, Macro>,
    conditions: Vec<bool>,
    skipping: bool,
    concatenating: bool,
    pub errors: Vec<Diagnostic>,
}
impl<'a> Lexer<'a> {
    pub fn new(source: &'a mut dyn Source, name: &[u16], chars: Vec<i64>) -> Self {
        let mut l = Self {
            source,
            file: File { name: name.to_vec(), stream: Stream::new(chars) },
            includes: Vec::new(),
            look: Token::default(),
            macros: HashMap::new(),
            conditions: Vec::new(),
            skipping: false,
            concatenating: false,
            errors: Vec::new(),
        };
        // 686AE0 call in 689060.
        l.look = token(&mut l.file.stream);
        l
    }
    fn read(&mut self) {
        self.look = token(&mut self.file.stream);
    }
    pub fn error(&mut self, message: impl Into<String>) {
        self.errors.push(Diagnostic { file: text(&self.file.name), line: self.file.stream.line, message: message.into() });
    }
    pub fn line(&self) -> i64 {
        self.file.stream.line
    }
    /// 689180: return `t` to the front of the input.
    pub fn unget(&mut self, t: Token) {
        let look = std::mem::replace(&mut self.look, t);
        self.file.stream.unget_token(&look);
    }
    /// 6891B0.
    fn recompute(&mut self) {
        self.skipping = self.conditions.iter().any(|c| !c);
    }
    /// 6886E0: the next parser token.
    pub fn next(&mut self) -> Token {
        loop {
            loop {
                if self.look.kind == EOF {
                    let Some(parent) = self.includes.pop() else {
                        return Token { kind: EOF, text: W::new() };
                    };
                    self.file = parent;
                    self.read();
                }
                if !self.skipping || self.look.kind == HASH {
                    break;
                }
                self.read();
            }
            match self.look.kind {
                IDENT => {
                    if let Some(m) = self.macros.get(&self.look.text).cloned() {
                        self.expand(&m);
                        continue;
                    }
                }
                STRING | MSTRING => {
                    let kind = self.look.kind;
                    let mut out = std::mem::take(&mut self.look);
                    self.read();
                    if self.concatenating {
                        return out;
                    }
                    self.concatenating = true;
                    let mut t = self.next();
                    while t.kind == kind {
                        out.text.extend_from_slice(&t.text);
                        t = self.next();
                    }
                    self.unget(t);
                    self.unget(out.clone());
                    self.concatenating = false;
                    out.text = escapes(&self.look.text);
                    out.kind = kind;
                    self.read();
                    return out;
                }
                TEXT => return self.text_run(),
                HASH => {
                    self.read();
                    let name = text(&self.look.text);
                    match name.as_str() {
                        "include" if !self.skipping => {
                            self.include();
                            continue;
                        }
                        "define" if !self.skipping => {
                            self.read();
                            self.define();
                            self.read();
                            continue;
                        }
                        "undef" if !self.skipping => {
                            self.read();
                            if self.look.kind == IDENT {
                                self.macros.remove(&self.look.text);
                            }
                            self.read();
                            continue;
                        }
                        "ifdef" | "ifndef" => {
                            self.read();
                            let defined = self.macros.contains_key(&self.look.text);
                            self.conditions.push(if name == "ifndef" { !defined } else { defined });
                            self.recompute();
                            self.read();
                            continue;
                        }
                        "else" => {
                            self.read();
                            if let Some(top) = self.conditions.last_mut() {
                                *top = !*top;
                            }
                            self.recompute();
                            self.read();
                            continue;
                        }
                        "endif" => {
                            self.read();
                            self.conditions.pop();
                            self.recompute();
                            self.read();
                            continue;
                        }
                        "line" => {
                            self.line_directive();
                            continue;
                        }
                        _ if self.skipping => {
                            self.file.stream.skip_line();
                            self.read();
                            continue;
                        }
                        _ => return Token { kind: HASH, text: w("#") },
                    }
                }
                OTHER if self.look.first() == 10 => {
                    self.read();
                    continue;
                }
                _ => {}
            }
            let out = std::mem::take(&mut self.look);
            self.read();
            return out;
        }
    }
    /// 688D80: consecutive non-ASCII characters with single newlines.
    fn text_run(&mut self) -> Token {
        let mut out = std::mem::take(&mut self.look);
        let s = &mut self.file.stream;
        let mut cur = scan(s);
        loop {
            while cur.kind == TEXT {
                out.text.extend_from_slice(&cur.text);
                cur = scan(s);
            }
            if cur.kind != OTHER || cur.first() != 10 {
                break;
            }
            cur = scan(s);
            if cur.kind == OTHER && cur.first() == 10 {
                out.text.extend_from_slice(&[10, 10]);
                cur = scan(s);
                break;
            }
            out.text.push(10);
        }
        s.unget_token(&cur);
        self.read();
        out
    }
    /// 687E30.
    fn include(&mut self) {
        let s = &mut self.file.stream;
        s.skip_blanks();
        if s.c != 34 {
            return;
        }
        let mut path: W = match self.file.name.iter().rposition(|&c| c == 124) {
            Some(i) => self.file.name[..=i].to_vec(),
            None => self.file.name[..file_start(&self.file.name)].to_vec(),
        };
        let s = &mut self.file.stream;
        s.next();
        while s.c != 34 {
            if s.c == 10 || path.len() > 260 {
                self.read();
                return;
            }
            path.push(s.c as u16);
            s.next();
        }
        s.next();
        match super::source::open(self.source, &path) {
            Ok(Some(chars)) => {
                let parent = std::mem::replace(&mut self.file, File { name: path, stream: Stream::new(chars) });
                self.includes.push(parent);
            }
            Ok(None) => self.error(format!("{} が開けません", text(&path))),
            Err(e) => self.error(format!("{}: {e:#}", text(&path))),
        }
        self.read();
    }
    /// 687C60/6893B0.
    fn define(&mut self) {
        if self.look.kind != IDENT {
            return;
        }
        let name = self.look.text.clone();
        let mut params: Option<Vec<W>> = None;
        if self.file.stream.c == 40 {
            self.file.stream.next();
            let mut list = Vec::new();
            self.read();
            while self.look.kind == IDENT {
                list.push(self.look.text.clone());
                self.read();
                if self.look.kind != COMMA {
                    break;
                }
                self.read();
            }
            params = Some(list);
        }
        self.macros.remove(&name);
        let index = |p: &Option<Vec<W>>, t: &W| p.as_ref().and_then(|l| l.iter().position(|x| x == t));
        let s = &mut self.file.stream;
        let mut body = Vec::new();
        let mut cur = scan(s);
        while cur.first() != 10 && cur.kind != EOF {
            if cur.kind == OTHER && cur.first() == 35 {
                cur = scan(s);
                if cur.kind == IDENT {
                    match index(&params, &cur.text) {
                        Some(i) => body.push(Piece::Quoted(i)),
                        None => body.push(Piece::Text(cur.text.clone())),
                    }
                }
            } else if cur.kind == IDENT {
                match index(&params, &cur.text) {
                    Some(i) => body.push(Piece::Arg(i)),
                    None => body.push(Piece::Text(cur.text.clone())),
                }
            } else if cur.kind == STRING {
                let mut t = vec![34];
                t.extend_from_slice(&cur.text);
                t.push(34);
                body.push(Piece::Text(t));
            } else {
                body.push(Piece::Text(cur.text.clone()));
            }
            cur = scan(s);
        }
        self.look = cur;
        self.macros.insert(name, Macro { params: params.map(|p| p.len()), body });
    }
    /// 6880E0/689550.
    fn expand(&mut self, m: &Macro) {
        let mut args: Vec<W> = Vec::new();
        let mut ok = true;
        if let Some(argc) = m.params {
            self.read();
            if self.look.kind != LPAREN {
                let t = std::mem::take(&mut self.look);
                self.file.stream.unget_token(&t);
            } else {
                let mut depth = 0i32;
                let mut cur = W::new();
                loop {
                    let t = scan(&mut self.file.stream);
                    if t.kind == -1 || t.kind == EOF {
                        self.error("macro arguments: unterminated or invalid token");
                        ok = false;
                        break;
                    }
                    if cur.len() + t.text.len() + 2 >= 0x400 {
                        self.error("macro arguments exceed 0x400 characters");
                        ok = false;
                        break;
                    }
                    if t.kind == STRING {
                        cur.push(34);
                        cur.extend_from_slice(&t.text);
                        cur.push(34);
                    } else if t.kind != OTHER {
                        cur.extend_from_slice(&t.text);
                    } else {
                        match t.first() {
                            40 => {
                                depth += 1;
                                cur.push(40);
                            }
                            41 => {
                                depth -= 1;
                                if depth < 0 {
                                    if !cur.is_empty() {
                                        args.push(std::mem::take(&mut cur));
                                    }
                                    break;
                                }
                                cur.push(41);
                            }
                            // A nested comma is dropped (6880E0 only splits at depth 0).
                            44 => {
                                if depth == 0 {
                                    args.push(std::mem::take(&mut cur));
                                }
                            }
                            _ => cur.extend_from_slice(&t.text),
                        }
                    }
                }
            }
            if ok && args.len() != argc {
                self.error("macro argument count mismatch");
                ok = false;
            }
        }
        if ok {
            let mut out = W::new();
            for p in &m.body {
                match p {
                    Piece::Text(t) => out.extend_from_slice(t),
                    Piece::Arg(i) => out.extend_from_slice(args.get(*i).map_or(&[][..], |a| a)),
                    Piece::Quoted(i) => {
                        out.push(34);
                        out.extend_from_slice(args.get(*i).map_or(&[][..], |a| a));
                        out.push(34);
                    }
                }
            }
            self.file.stream.unget_str(&out);
        }
        self.read();
    }
    /// 689630.
    fn line_directive(&mut self) {
        self.read();
        if self.look.kind != NUMBER {
            self.error("#line");
            return;
        }
        let n: i64 = text(&self.look.text).parse().unwrap_or(0);
        if n < 1 {
            self.error("#line");
            return;
        }
        self.read();
        let mut name = None;
        if self.look.kind == STRING {
            name = Some(self.look.text.clone());
            self.read();
        }
        if self.look.first() == 10 {
            self.file.stream.line = n;
            if let Some(name) = name {
                self.file.name = name;
            }
        } else {
            self.error("#line");
        }
    }
}

/// 6900E0: start of the file name component.
pub fn file_start(path: &[u16]) -> usize {
    let mut at = 0;
    let mut start = 0;
    while at < path.len() {
        let rest = &path[at..];
        let sep = rest.iter().position(|&c| c == 92).or_else(|| rest.iter().position(|&c| c == 47)).or_else(|| rest.iter().position(|&c| c == 124));
        let Some(p) = sep else { break };
        start = at + p + 1;
        at = start;
        if path.get(at).is_none() {
            break;
        }
    }
    start
}

/// 688E90. Embedded NULs are kept; consumers measure with `text_len`.
pub fn escapes(src: &[u16]) -> W {
    let mut out = W::new();
    let mut i = 0;
    let at = |i: usize| src.get(i).copied().unwrap_or(0);
    while at(i) != 0 {
        let c = at(i);
        if c != 92 {
            out.push(c);
            i += 1;
            continue;
        }
        i += 1;
        let e = at(i);
        match e {
            34 | 39 | 92 => {
                out.push(e);
                i += 1;
            }
            97 => {
                out.push(7);
                i += 1;
            }
            110 => {
                out.push(10);
                i += 1;
            }
            114 => {
                out.push(13);
                i += 1;
            }
            116 => {
                out.push(9);
                i += 1;
            }
            120 => {
                i += 1;
                let mut v = 0u16;
                for _ in 0..2 {
                    let d = at(i);
                    let Some(n) = char::from_u32(u32::from(d)).and_then(|c| c.to_digit(16)) else { break };
                    v = v.wrapping_mul(16).wrapping_add(n as u16);
                    i += 1;
                }
                out.push(v);
            }
            _ => {
                let mut v = 0u16;
                for _ in 0..3 {
                    let d = at(i);
                    if !(48..=55).contains(&d) {
                        break;
                    }
                    v = v.wrapping_mul(8).wrapping_add(d - 48);
                    i += 1;
                }
                out.push(v);
            }
        }
    }
    out
}

/// 690940/690650: the length of message-aware text, in UTF-16 units up
/// to the NUL that ends it. Control sequences carry their own NUL-ended
/// parts (7,1 ruby, 7,7, 7,8 voice, 7,12/7,60 names) which do not end the
/// text. Reads past the buffer see 0.
pub fn text_len(b: &[u16]) -> usize {
    struct Walk<'a> {
        b: &'a [u16],
        pos: usize,
        cur: u16,
    }
    impl Walk<'_> {
        fn read(&mut self) {
            self.cur = self.b.get(self.pos).copied().unwrap_or(0);
            self.pos += 1;
        }
        fn skip(&mut self, n: usize) {
            for _ in 0..n {
                self.read();
            }
        }
        /// 6904B0.
        fn skip_string(&mut self) {
            if self.cur != 0 {
                loop {
                    self.read();
                    if self.cur == 0 {
                        break;
                    }
                }
            }
            self.read();
        }
        fn walk(&mut self) {
            self.read();
            loop {
                match self.cur {
                    0 => break,
                    1 => self.skip(4),
                    2 | 4 => self.skip(1),
                    3 | 6 => self.skip(2),
                    7 => {
                        self.read();
                        match self.cur {
                            1 => {
                                self.walk();
                                self.read();
                            }
                            2 | 13 => self.skip(2),
                            7 => {
                                self.walk();
                                self.read();
                                self.skip_string();
                            }
                            8 => {
                                self.read();
                                self.skip_string();
                            }
                            12 | 60 => self.skip_string(),
                            6 | 9 | 14 | 15 | 16 => self.read(),
                            // Any other code is dispatched as a character.
                            _ => {}
                        }
                    }
                    _ => self.read(),
                }
            }
        }
    }
    let mut w = Walk { b, pos: 0, cur: 0 };
    w.walk();
    w.pos - 1
}
