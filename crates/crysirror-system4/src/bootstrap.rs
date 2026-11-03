// Copyright (C) 2019 Nunuhara Cabbage; Rust (C) 2026 CrySirror contributors.
// GPL-2.0-or-later. libsys4 b14839f src/ini.c, ini_lexer.l, ini_parser.y;
// xsystem4 a85467d src/system4.c. Native configuration policy only.
//! Bounded original System40.ini syntax over caller-owned bytes. Backslashes
//! in strings are literal (as in the native lexer), not JSON escapes.
use anyhow::{bail, ensure, Context, Result};
use crysirror_core::path::{Mount, VfsPath};
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Integer(i32),
    Float(f32),
    Boolean(bool),
    String(String),
    List(Vec<Value>),
    Formation(Vec<Entry>),
    /// Native Formation entries retain list-assignment wrappers: libsys4
    /// normalizes sparse assignments only in the root document.
    ListAssignment {
        index: usize,
        value: Box<Value>,
    },
}
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub name: String,
    pub value: Value,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Document {
    pub entries: Vec<Entry>,
    pub warnings: Vec<String>,
}
#[derive(Clone, Debug, PartialEq)]
enum Token {
    Word(String),
    Value(Value),
    Symbol(u8),
}
struct Parser {
    tokens: Vec<Token>,
    at: usize,
    nodes: usize,
    warnings: Vec<String>,
}
impl Parser {
    fn symbol(&mut self, value: u8) -> bool {
        if self.tokens.get(self.at) == Some(&Token::Symbol(value)) {
            self.at += 1;
            true
        } else {
            false
        }
    }
    fn need(&mut self, value: u8) -> Result<()> {
        ensure!(
            self.symbol(value),
            "System4 INI expected `{}`",
            char::from(value)
        );
        Ok(())
    }
    fn value(&mut self, depth: usize) -> Result<Value> {
        ensure!(
            depth < 32 && self.nodes < 100_000,
            "System4 INI value/depth budget"
        );
        self.nodes += 1;
        if self.symbol(b'{') {
            if self.symbol(b'}') {
                return Ok(Value::List(Vec::new()));
            }
            let values = self.list(depth + 1, true)?;
            self.need(b'}')?;
            return Ok(Value::List(values));
        }
        let Some(Token::Value(value)) = self.tokens.get(self.at) else {
            bail!("System4 INI expected typed value");
        };
        let value = value.clone();
        self.at += 1;
        Ok(value)
    }
    fn list(&mut self, depth: usize, trailing: bool) -> Result<Vec<Value>> {
        let mut values = vec![self.value(depth)?];
        while self.symbol(b',') {
            if trailing && self.tokens.get(self.at) == Some(&Token::Symbol(b'}')) {
                break;
            }
            values.push(self.value(depth)?);
        }
        Ok(values)
    }
    fn entries(&mut self, depth: usize) -> Result<Vec<Entry>> {
        ensure!(depth < 32, "System4 INI formation depth budget");
        let mut output: Vec<Entry> = Vec::new();
        while self.at < self.tokens.len() && self.tokens.get(self.at) != Some(&Token::Symbol(b'}'))
        {
            let Some(Token::Word(name)) = self.tokens.get(self.at) else {
                bail!("System4 INI expected identifier");
            };
            let name = name.clone();
            self.at += 1;
            if name == "Formation" {
                let Value::String(name) = self.value(depth)? else {
                    bail!("System4 INI Formation name must be string");
                };
                self.need(b'{')?;
                let children = self.entries(depth + 1)?;
                self.need(b'}')?;
                ensure!(!children.is_empty(), "System4 INI empty Formation");
                output.push(Entry {
                    name,
                    value: Value::Formation(children),
                });
                continue;
            }
            let index = if self.symbol(b'[') {
                let Value::Integer(index) = self.value(depth)? else {
                    bail!("System4 INI array index must be integer");
                };
                ensure!(
                    (0..1024).contains(&index),
                    "System4 INI sparse array budget"
                );
                self.need(b']')?;
                Some(index as usize)
            } else {
                None
            };
            self.need(b'=')?;
            let mut values = self.list(depth, false)?;
            let value = if values.len() == 1 {
                values.pop().unwrap()
            } else {
                Value::List(values)
            };
            if let Some(index) = index {
                if depth > 0 {
                    output.push(Entry {
                        name,
                        value: Value::ListAssignment {
                            index,
                            value: Box::new(value),
                        },
                    });
                    ensure!(output.len() <= 4096, "System4 INI entry budget");
                    continue;
                }
                let position =
                    if let Some(position) = output.iter().position(|entry| entry.name == name) {
                        position
                    } else {
                        output.push(Entry {
                            name: name.clone(),
                            value: Value::List(Vec::new()),
                        });
                        output.len() - 1
                    };
                if let Value::List(values) = &mut output[position].value {
                    values.resize(values.len().max(index + 1), Value::Null);
                    values[index] = value;
                } else {
                    self.warnings.push(format!(
                        "ignored native list assignment to non-list {name}[{index}]"
                    ));
                }
            } else {
                output.push(Entry { name, value });
            }
            ensure!(output.len() <= 4096, "System4 INI entry budget");
        }
        Ok(output)
    }
}
fn tokens(text: &str) -> Result<Vec<Token>> {
    let bytes = text.as_bytes();
    let mut at = 0;
    let mut output = Vec::new();
    while at < bytes.len() {
        ensure!(output.len() < 100_000, "System4 INI token budget");
        let c = bytes[at];
        if c.is_ascii_whitespace() {
            at += 1;
            continue;
        }
        if c == b';' || bytes.get(at..at + 2) == Some(b"//") {
            at += bytes[at..]
                .iter()
                .position(|c| *c == b'\n')
                .unwrap_or(bytes.len() - at);
            continue;
        }
        if b"{}[]=,".contains(&c) {
            output.push(Token::Symbol(c));
            at += 1;
            continue;
        }
        if c == b'"' {
            at += 1;
            let start = at;
            while bytes.get(at).is_some_and(|c| *c != b'"' && *c != b'\n') {
                at += 1;
            }
            ensure!(
                bytes.get(at) == Some(&b'"') && at - start < 65536,
                "System4 INI unterminated/oversized string"
            );
            output.push(Token::Value(Value::String(text[start..at].into())));
            at += 1;
            continue;
        }
        if c.is_ascii_digit() {
            let start = at;
            while bytes.get(at).is_some_and(u8::is_ascii_digit) {
                at += 1;
            }
            if bytes.get(at) == Some(&b'.') && bytes.get(at + 1).is_some_and(u8::is_ascii_digit) {
                at += 1;
                while bytes.get(at).is_some_and(u8::is_ascii_digit) {
                    at += 1;
                }
                let value = text[start..at].parse::<f32>()?;
                ensure!(value.is_finite(), "System4 INI float range");
                output.push(Token::Value(Value::Float(value)));
            } else {
                output.push(Token::Value(Value::Integer(text[start..at].parse()?)));
            }
            continue;
        }
        if c.is_ascii_alphabetic() {
            let start = at;
            at += 1;
            while bytes
                .get(at)
                .is_some_and(|c| c.is_ascii_alphanumeric() || b"_-".contains(c))
            {
                at += 1;
            }
            let name = &text[start..at];
            output.push(match name {
                "true" => Token::Value(Value::Boolean(true)),
                "false" => Token::Value(Value::Boolean(false)),
                _ => Token::Word(name.into()),
            });
            continue;
        }
        bail!("unsupported System4 INI token at byte{at}");
    }
    Ok(output)
}
impl Document {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        ensure!(
            !bytes.is_empty() && bytes.len() <= 1024 * 1024 && !bytes.contains(&0),
            "System4 INI byte budget/NUL"
        );
        let text = encoding_rs::SHIFT_JIS
            .decode_without_bom_handling_and_without_replacement(bytes)
            .context("System4 INI encoding")?;
        let mut parser = Parser {
            tokens: tokens(&text)?,
            at: 0,
            nodes: 0,
            warnings: Vec::new(),
        };
        let entries = parser.entries(0)?;
        ensure!(
            !entries.is_empty() && parser.at == parser.tokens.len(),
            "System4 INI trailing/empty document"
        );
        Ok(Self {
            entries,
            warnings: parser.warnings,
        })
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mixer {
    pub name: String,
    pub volume: i32,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bootstrap {
    pub program: String,
    pub canvas: [u32; 2],
    pub game_name: String,
    pub save_folder: String,
    pub native_vm: Option<String>,
    pub joypad: bool,
    pub mixers: Vec<Mixer>,
}
impl Bootstrap {
    pub fn from_document(doc: &Document) -> Result<Self> {
        let mut program = None;
        let mut canvas = [800, 600];
        let mut game_name = None;
        let mut save_folder = "SaveData".to_string();
        let mut native_vm = None;
        let mut joypad = false;
        let mut default_volume = 100;
        let mut mixers = Vec::new();
        for entry in &doc.entries {
            let string = || -> Result<String> {
                if let Value::String(value) = &entry.value {
                    Ok(value.clone())
                } else {
                    bail!("System4 INI {} must be string", entry.name)
                }
            };
            let integer = || -> Result<i32> {
                if let Value::Integer(value) = entry.value {
                    Ok(value)
                } else {
                    bail!("System4 INI {} must be integer", entry.name)
                }
            };
            match entry.name.as_str() {
                "CodeName" => program = Some(string()?),
                "GameName" => game_name = Some(string()?),
                "SaveFolder" => save_folder = string()?,
                "MainVM" => native_vm = Some(string()?),
                "ViewWidth" => canvas[0] = u32::try_from(integer()?)?,
                "ViewHeight" => canvas[1] = u32::try_from(integer()?)?,
                "UseJoypad" => {
                    let Value::Boolean(value) = entry.value else {
                        bail!("System4 INI UseJoypad must be boolean");
                    };
                    joypad = value;
                }
                "DefaultVolumeRate" => default_volume = integer()?,
                "VolumeValancer" => {
                    let Value::List(values) = &entry.value else {
                        bail!("System4 native mixer configuration must be list");
                    };
                    ensure!(values.len() <= 128, "System4 mixer count budget");
                    mixers.clear();
                    for value in values {
                        let mixer = match value {
                            Value::Null => Mixer {
                                name: "Unnamed".into(),
                                volume: 0,
                            },
                            Value::String(name) => Mixer {
                                name: name.clone(),
                                volume: default_volume,
                            },
                            Value::List(pair) if pair.len() == 2 => {
                                let (Value::String(name), Value::Integer(volume)) =
                                    (&pair[0], &pair[1])
                                else {
                                    bail!("System4 native mixer name/volume ABI");
                                };
                                Mixer {
                                    name: name.clone(),
                                    volume: *volume,
                                }
                            }
                            _ => bail!("System4 native mixer configuration ABI"),
                        };
                        mixers.push(mixer);
                    }
                }
                _ => {} // Retained in Document; not interpreted as a host feature.
            }
        }
        ensure!(
            canvas.iter().all(|v| (1..=8192).contains(v)),
            "System4 native view budget"
        );
        let program = program.context("System4 INI missing CodeName")?;
        ensure!(
            program.to_ascii_lowercase().ends_with(".ain"),
            "System4 INI CodeName is not AIN"
        );
        ensure!(
            VfsPath::new(Mount::Asset, &program)?.relative() == program,
            "System4 INI program capability path"
        );
        ensure!(
            VfsPath::new(Mount::Save, &save_folder)?.relative() == save_folder,
            "System4 INI save capability path"
        );
        Ok(Self {
            game_name: game_name.unwrap_or_else(|| program.clone()),
            program,
            canvas,
            save_folder,
            native_vm,
            joypad,
            mixers,
        })
    }
}
