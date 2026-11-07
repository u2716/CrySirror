use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
mod line;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Instruction {
    pub op: String,
    pub args: Vec<String>,
    pub line: usize,
    pub end: Option<usize>,
    pub alternate: Option<usize>,
    pub preprocess: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Program {
    pub path: String,
    pub code: Vec<Instruction>,
    pub labels: BTreeMap<String, usize>,
    pub includes: Vec<String>,
    pub inline_includes: bool,
    ///Native label origin in its physical source file (zero based). Native
    ///saved positions are relative line plus a UTF16 command column.
    #[serde(default)]
    pub native_labels: BTreeMap<String, usize>,
    #[serde(default)]
    pub native_columns: BTreeMap<usize, [usize; 2]>,
}
pub fn decode_text(bytes: &[u8]) -> Result<String> {
    ensure!(bytes.len() <= 16 * 1024 * 1024, "QLIE text size limit");
    if bytes.starts_with(&[0xff, 0xfe]) || bytes.iter().take(512).any(|b| *b == 0) {
        let bytes = bytes.strip_prefix(&[0xff, 0xfe]).unwrap_or(bytes);
        ensure!(bytes.len() % 2 == 0, "QLIE odd UTF16 source");
        String::from_utf16(
            &bytes
                .chunks_exact(2)
                .map(|p| u16::from_le_bytes([p[0], p[1]]))
                .collect::<Vec<_>>(),
        )
        .context("QLIE UTF16 source")
    } else {
        encoding_rs::SHIFT_JIS
            .decode_without_bom_handling_and_without_replacement(bytes)
            .map(|s| s.into_owned())
            .context("QLIE CP932 source")
    }
}
/// Commas inside quoted strings, expressions or array subscripts are data.
pub fn arguments(source: &str) -> Result<Vec<String>> {
    let mut quote = false;
    let mut depth = 0i32;
    let mut start = 0;
    let mut out = Vec::new();
    for (i, c) in source.char_indices() {
        match c {
            '"' => quote = !quote,
            '(' | '[' if !quote => depth += 1,
            ')' | ']' if !quote => {
                depth -= 1;
                ensure!(depth >= 0, "QLIE expression delimiter");
            }
            ',' if !quote && depth == 0 => {
                out.push(source[start..i].trim().into());
                start = i + 1;
            }
            _ => {}
        }
    }
    ensure!(!quote && depth == 0, "QLIE unterminated argument");
    out.push(source[start..].trim().into());
    Ok(out)
}
/// Native data splitter (4E65B4), distinct from expression argument parsing.
/// Delimiters inside quotes or balanced {}, () and [] remain part of a field;
/// quote characters are retained for the caller's subsequent format pass.
pub fn split_native(source: &str, delimiter: &str, strip_space: bool, raw: bool) -> Vec<String> {
    let separator = delimiter.chars().next();
    let mut result = Vec::new();
    let mut value = String::new();
    let mut quoted = false;
    let mut depth = 0i32;
    let mut chars = source.chars().peekable();
    let mut last = None;
    while let Some(c) = chars.next() {
        last = Some(c);
        if c == '\r' && chars.peek() == Some(&'\n') {
            chars.next();
            last = Some('\n');
            result.push(std::mem::take(&mut value));
            continue;
        }
        if strip_space && matches!(c, ' ' | '\t' | '\u{a0}') {
            continue;
        }
        if !raw {
            if c == '"' {
                quoted = !quoted;
            }
            if matches!(c, '{' | '(' | '[') {
                depth += 1;
            }
            if matches!(c, '}' | ')' | ']') {
                depth -= 1;
            }
        }
        if !quoted && depth == 0 && Some(c) == separator {
            result.push(std::mem::take(&mut value));
        } else {
            value.push(c);
        }
    }
    if !value.is_empty() {
        result.push(value);
    }
    if last.is_some() && last == separator {
        result.push(String::new());
    }
    result
}
pub fn canonical(path: &str) -> Result<String> {
    let mut components = Vec::new();
    let path = path.replace('\\', "/");
    ensure!(
        !path.starts_with('/') && !path.contains(':') && !path.contains('\0'),
        "QLIE relative resource path"
    );
    for c in path.split('/') {
        match c {
            "" | "." => {}
            ".." => {
                ensure!(components.pop().is_some(), "QLIE path escapes game root");
            }
            _ => components.push(c),
        }
    }
    Ok(components.join("/").to_ascii_lowercase())
}
impl Program {
    pub fn parse(path: &str, bytes: &[u8]) -> Result<Self> {
        Self::parse_text(path, &decode_text(bytes)?)
    }
    pub fn parse_text(path: &str, text: &str) -> Result<Self> {
        Self::parse_source(path, text, true)
    }
    pub(crate) fn parse_inline(path: &str, text: &str) -> Result<Self> {
        Self::parse_source(path, text, false)
    }
    fn parse_source(path: &str, text: &str, file_admission: bool) -> Result<Self> {
        ensure!(
            text.len() <= line::MAX_TEXT_BYTES,
            "QLIE parsed text size limit"
        );
        let mut p = Self {
            path: canonical(path)?,
            code: Vec::new(),
            labels: BTreeMap::new(),
            includes: Vec::new(),
            inline_includes: false,
            native_labels: BTreeMap::new(),
            native_columns: BTreeMap::new(),
        };
        let mut blocks: Vec<(usize, Option<usize>)> = Vec::new();
        let mut preprocess = true;
        let lines = if file_admission {
            line::admit(text)?
        } else {
            line::split(text)?
        };
        for (line, admitted) in lines.iter().enumerate() {
            let source = admitted.trim_matches(|c: char| c <= '\u{20}');
            let leading =
                admitted.len() - admitted.trim_start_matches(|c: char| c <= '\u{20}').len();
            let source_column = admitted[..leading].encode_utf16().count();
            if source.is_empty() {
                if preprocess {
                    p.code.push(Instruction {
                        op: "blank".into(),
                        args: vec![],
                        line: line + 1,
                        end: None,
                        alternate: None,
                        preprocess: true,
                    });
                }
                continue;
            }
            if let Some(include) = source.strip_prefix("@@@") {
                p.includes
                    .push(include.trim_start_matches('!').trim().to_owned());
                if !include.starts_with('!') {
                    p.inline_includes = true;
                    p.code.push(Instruction {
                        op: "include".into(),
                        args: vec![include.trim().to_owned()],
                        line: line + 1,
                        end: None,
                        alternate: None,
                        preprocess: false,
                    });
                }
                continue;
            }
            if source.starts_with("@@") {
                preprocess = !source.starts_with("@@!");
                p.labels.insert(source.to_ascii_lowercase(), p.code.len());
                p.native_labels.insert(source.to_ascii_lowercase(), line);
                continue;
            }
            // Editor profile marker at the beginning of a script, not dialogue.
            if p.labels.is_empty() && source.parse::<u8>().is_ok() {
                continue;
            }
            if source.starts_with('\\') {
                let mut starts = Vec::new();
                let mut quote = false;
                for (i, c) in source.char_indices() {
                    if c == '"' {
                        quote = !quote;
                    } else if c == '\\' && !quote {
                        starts.push(i);
                    }
                }
                starts.push(source.len());
                for span in starts.windows(2) {
                    let raw = source[span[0] + 1..span[1]].trim();
                    let mut a = arguments(raw)?;
                    // A final comma terminates a native command, including
                    // `\scp,sl,\?v,...`. Interior placeholders and quoted
                    // empty strings remain actual argument fields.
                    while a.last().is_some_and(|value| value.is_empty()) {
                        a.pop();
                    }
                    if a.is_empty() {
                        continue;
                    }
                    let op = a.remove(0).to_ascii_lowercase();
                    if op == "then" {
                        continue;
                    }
                    let pc = p.code.len();
                    p.native_columns.insert(
                        pc,
                        [
                            source_column + source[..span[0]].encode_utf16().count(),
                            source_column + source[..span[1]].encode_utf16().count(),
                        ],
                    );
                    if matches!(op.as_str(), "if" | "for" | "case") {
                        blocks.push((pc, None));
                    }
                    if op == "else" {
                        let block = blocks.last_mut().context("QLIE else without block")?;
                        ensure!(block.1.is_none(), "QLIE duplicate else");
                        p.code[block.0].alternate = Some(pc + 1);
                        block.1 = Some(pc);
                    }
                    let mut instruction = Instruction {
                        op: op.clone(),
                        args: a,
                        line: line + 1,
                        end: None,
                        alternate: None,
                        preprocess,
                    };
                    if op == "end" {
                        let (start, other) = blocks.pop().context("QLIE end without block")?;
                        p.code[start].end = Some(pc + 1);
                        if let Some(other) = other {
                            p.code[other].end = Some(pc + 1);
                        }
                        instruction.end = Some(start);
                    }
                    p.code.push(instruction);
                }
            } else {
                let (op, args) = if let Some(s) = source.strip_prefix('^') {
                    let mut a = arguments(s)?;
                    (format!("^{}", a.remove(0)), a)
                } else {
                    ("text".into(), vec![source.into()])
                };
                p.code.push(Instruction {
                    op,
                    args,
                    line: line + 1,
                    end: None,
                    alternate: None,
                    preprocess,
                });
            }
            ensure!(p.code.len() <= 1_000_000, "QLIE instruction count limit");
        }
        for (pc, instruction) in p.code.iter().enumerate() {
            p.native_columns.entry(pc).or_insert_with(|| {
                [
                    0,
                    lines
                        .get(instruction.line.saturating_sub(1))
                        .map_or(0, |line| line.encode_utf16().count()),
                ]
            });
        }
        ensure!(blocks.is_empty(), "QLIE unclosed block in {path}");
        Ok(p)
    }
}
