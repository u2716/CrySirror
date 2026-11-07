//! 6D648C statements without a wait: regular-expression split/replace,
//! half-width conversion and width, debug channel and object lookup.
//!
//! - `bspl,text,pattern` (68B25C): split; ResultInt0 the part count,
//!   ResultStr0..15 the parts.
//! - `btra,pattern,replacement,text` (68B34C, as `bsub` with its arguments
//!   in this order): replace every match.
//! - `hanc,text` (4E7678): half-width conversion; `hlen,text` (4E6C2C):
//!   column count.
//! - `dout`, `dmon`, `dbrk` do nothing in the release player; `drun,clear`
//!   empties the reader's debug text and `drun[,file]` answers it in
//!   ResultStr0 (always empty, since `dout` writes nothing).
//! - `fobj,name` (6D8766): when variable `name` holds an object, ResultObj0
//!   is that object.
use super::*;

fn regex(pattern: &str) -> Result<regex::Regex> {
    Ok(regex::RegexBuilder::new(pattern).size_limit(1024 * 1024).build()?)
}

impl QlieVm {
    pub(super) fn text_statement(&mut self, m: &mut Machine, op: &str, a: &[String]) -> Result<bool> {
        let text = |m: &mut Machine, i: usize| -> Result<String> {
            a.get(i).map(|v| m.eval(v).map(|v| v.text())).transpose().map(|v| v.unwrap_or_default())
        };
        match op {
            "bspl" => {
                ensure!(a.len() == 2, "QLIE regex split ABI");
                let (input, pattern) = (text(m, 0)?, text(m, 1)?);
                let parts: Vec<String> = regex(&pattern)?.split(&input).map(str::to_owned).collect();
                m.result(0, Value::Int(i32::try_from(parts.len())?));
                for (i, part) in parts.into_iter().take(16).enumerate() {
                    m.result_text(i, part);
                }
            }
            "btra" => {
                ensure!(a.len() == 3, "QLIE regex translation ABI");
                let (pattern, replacement, input) = (text(m, 0)?, text(m, 1)?, text(m, 2)?);
                let result = regex(&pattern)?.replace_all(&input, replacement.as_str()).into_owned();
                ensure!(result.len() <= 1024 * 1024, "QLIE regex output limit");
                m.result_text(0, result);
            }
            "hanc" => {
                let input = text(m, 0)?;
                m.result_text(0, crysirror_text::legacy_width::to_halfwidth(&input));
            }
            "hlen" => {
                let input = text(m, 0)?;
                m.result(0, Value::Int(i32::try_from(crysirror_text::legacy_width::columns(&input))?));
            }
            "dout" | "dmon" | "dbrk" => {}
            "drun" => {
                let command = text(m, 0)?.to_lowercase();
                if command == "clear" {
                    self.global_strings.remove("debug");
                }
                if command == "file" || command.is_empty() {
                    let debug = self.global_strings.get("debug").cloned().unwrap_or_default();
                    m.result_text(0, debug);
                }
            }
            "fobj" => {
                ensure!(a.len() == 1, "QLIE object lookup ABI");
                let name = text(m, 0)?;
                let value = m.key(&name).ok().and_then(|key| m.value(&key).ok());
                if let Some(value) = value.filter(|v| m.object_types.contains_key(&v.text())) {
                    m.thread_globals_mut().insert("resultobj[0]".into(), value);
                }
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
}
