// SPDX-License-Identifier: MPL-2.0
//! Static policy recovered from the installed Tsukumo-2012 executable:
//! 450150 loads SCR and indexes :: sections/: local labels; 44FAF0 strips
//! ASCII whitespace/comments outside quoted strings; 450670 registers macros.
//! AOS2/SCR compression belongs to the existing shared format registry.
use anyhow::{Context, Result, ensure};
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct Instruction {
    pub line: u32,
    pub byte_offset: u32,
    pub source: String,
}
#[derive(Clone, Debug)]
pub struct Program {
    pub name: String,
    pub instructions: Vec<Instruction>,
    pub sections: BTreeMap<String, usize>,
    pub labels: BTreeMap<(String, String), usize>,
    pub macros: BTreeMap<String, Vec<Instruction>>,
}
/// Native command spelling is lower case and case-sensitive. Resource names
/// use Windows filename comparison; variables and labels keep native case.
pub fn call(source: &str) -> Result<(&str, Vec<String>)> {
    let end = source
        .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .unwrap_or(source.len());
    ensure!(end > 0, "missing SugarPot command");
    let name = &source[..end];
    let tail = source[end..].trim();
    if tail.is_empty() {
        return Ok((name, vec![]));
    }
    if !tail.starts_with('(') {
        return Ok((name, split_arguments(tail)?));
    }
    ensure!(
        tail.ends_with(')'),
        "unterminated SugarPot command arguments"
    );
    Ok((name, split_arguments(&tail[1..tail.len() - 1])?))
}
pub fn split_arguments(source: &str) -> Result<Vec<String>> {
    if source.trim().is_empty() {
        return Ok(vec![]);
    }
    let mut out: Vec<String> = vec![];
    let mut quote = false;
    let mut depth = 0usize;
    let mut start = 0;
    for (at, ch) in source.char_indices() {
        match ch {
            '"' => quote = !quote,
            '(' if !quote => depth += 1,
            ')' if !quote => {
                ensure!(depth > 0, "unbalanced SugarPot argument");
                depth -= 1;
            }
            ',' if !quote && depth == 0 => {
                out.push(source[start..at].trim().into());
                start = at + 1;
            }
            _ => {}
        }
    }
    ensure!(!quote && depth == 0, "unterminated SugarPot argument");
    out.push(source[start..].trim().into());
    ensure!(
        out.len() <= 10 && out.iter().all(|s| s.len() <= 8192),
        "SugarPot argument budget"
    );
    Ok(out)
}
fn normalized_line(line: &str) -> Result<String> {
    let mut quote = false;
    let mut end = line.len();
    for (at, c) in line.char_indices() {
        if c == '"' {
            quote = !quote;
        }
        if c == '#' && !quote {
            end = at;
            break;
        }
    }
    ensure!(!quote, "unterminated SugarPot literal");
    Ok(line[..end].trim_matches([' ', '\t', '\r']).to_owned())
}
impl Program {
    pub fn parse(name: &str, bytes: &[u8]) -> Result<Self> {
        ensure!(
            bytes.len() <= 4 * 1024 * 1024 && !bytes.contains(&0),
            "SugarPot source size/NUL"
        );
        let (text, errors) = encoding_rs::SHIFT_JIS.decode_without_bom_handling(bytes);
        ensure!(!errors, "invalid SugarPot CP932 source");
        let mut p = Self {
            name: name.into(),
            instructions: vec![],
            sections: BTreeMap::new(),
            labels: BTreeMap::new(),
            macros: BTreeMap::new(),
        };
        let mut section = String::new();
        let mut offset = 0usize;
        let mut defining: Option<(String, Vec<Instruction>)> = None;
        let mut text_continues = false;
        let mut selecting: Option<Instruction> = None;
        for (i, (raw, encoded)) in text
            .split_inclusive('\n')
            .zip(bytes.split_inclusive(|b| *b == b'\n'))
            .enumerate()
        {
            let mut source = normalized_line(raw.trim_end_matches('\n'))
                .with_context(|| format!("{name}:{}", i + 1))?;
            let mut ins = Instruction {
                line: (i + 1) as u32,
                byte_offset: offset as u32,
                source: source.clone(),
            };
            // Preserve encoded offsets, rather than confusing UTF-8 offsets
            // with the original CP932 resource positions.
            offset += encoded.len();
            ensure!(
                i < 65535 && source.len() <= 65535,
                "SugarPot source line budget"
            );
            //44FAF0 recursively skips comment-only physical lines. They do
            //not occupy a native instruction slot. Consecutive Japanese/@
            //text lines are joined with the native explicit line break;
            //a blank line still terminates its text/page instruction.
            if raw.trim_start_matches([' ', '\t']).starts_with('#') {
                text_continues = false;
                continue;
            }
            if let Some(mut pending) = selecting.take() {
                pending.source.push_str(&source);
                ensure!(
                    pending.source.len() <= 65535,
                    "SugarPot selection source budget"
                );
                ins = pending;
                source = ins.source.clone();
            }
            if source.starts_with('/') && !source.ends_with("//") {
                selecting = Some(ins);
                text_continues = false;
                continue;
            }
            let text_start = source
                .chars()
                .next()
                .is_some_and(|c| !c.is_ascii() || matches!(c, '[' | '\\' | '@'));
            let can_join = source
                .chars()
                .next()
                .is_some_and(|c| !c.is_ascii() || c == '@');
            if defining.is_none() && text_continues && can_join {
                let previous = p
                    .instructions
                    .last_mut()
                    .context("SugarPot text continuation")?;
                previous.source.push('\n');
                previous.source.push_str(&source);
                text_continues = !raw.contains('#');
                continue;
            }
            text_continues = text_start && !raw.contains('#');

            if source.starts_with("^macro") {
                ensure!(defining.is_none(), "nested SugarPot macro");
                let (_, args) = call(&source[1..])?;
                ensure!(args.len() == 1, "SugarPot macro signature");
                let key = args[0].trim_matches('"').to_owned();
                defining = Some((key, vec![]));
                continue;
            }
            if source == "^endm" {
                let (key, body) = defining.take().context("SugarPot endm without macro")?;
                ensure!(
                    p.macros.insert(key, body).is_none(),
                    "duplicate SugarPot macro"
                );
                continue;
            }
            if let Some((_, body)) = &mut defining {
                body.push(ins);
                continue;
            }
            let pc = p.instructions.len();
            if let Some(key) = source.strip_prefix("::") {
                section = key.to_owned();
                ensure!(
                    p.sections.insert(section.clone(), pc + 1).is_none(),
                    "duplicate SugarPot section"
                );
            } else if let Some(key) = source.strip_prefix(':') {
                ensure!(
                    p.labels
                        .insert((section.clone(), key.to_owned()), pc + 1)
                        .is_none(),
                    "duplicate SugarPot local label"
                );
            }
            p.instructions.push(ins);
        }
        ensure!(
            defining.is_none()
                && selecting.is_none()
                && p.sections.contains_key("START")
                && p.sections.contains_key("END"),
            "SugarPot missing START/END or macro end"
        );
        Ok(p)
    }
}
