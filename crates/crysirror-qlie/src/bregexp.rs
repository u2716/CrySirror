//! BREGEXP.DLL `BSubst` command strings (`s/pattern/replacement/flags`) as
//! used by QLIE `\bsub`. Perl syntax is translated to the shared `regex`
//! crate; `k` (Shift-JIS awareness) is implicit for Unicode text.
use anyhow::{bail, ensure, Context, Result};

/// Parses `s<d>pattern<d>replacement<d>flags`. Returns the compiled pattern,
/// a `regex`-crate replacement template and whether `g` was given.
pub fn substitution(command: &str) -> Result<(regex::Regex, String, bool)> {
    ensure!(command.len() <= 65536, "QLIE BREGEXP command length");
    let rest = command.strip_prefix('s').context("QLIE BREGEXP substitution")?;
    let delimiter = rest.chars().next().context("QLIE BREGEXP delimiter")?;
    ensure!(
        !delimiter.is_alphanumeric() && !delimiter.is_whitespace() && delimiter != '\\',
        "QLIE BREGEXP delimiter"
    );
    let mut fields = vec![String::new()];
    let mut chars = rest[delimiter.len_utf8()..].chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                // An escaped delimiter is the literal delimiter character.
                Some(n) if n == delimiter => fields.last_mut().unwrap().push(n),
                Some(n) => {
                    let f = fields.last_mut().unwrap();
                    f.push('\\');
                    f.push(n);
                }
                None => bail!("QLIE BREGEXP trailing escape"),
            }
        } else if c == delimiter && fields.len() < 3 {
            fields.push(String::new());
        } else {
            fields.last_mut().unwrap().push(c);
        }
    }
    ensure!(fields.len() == 3, "QLIE BREGEXP substitution fields");
    let flags = &fields[2];
    let mut builder = regex::RegexBuilder::new(&fields[0]);
    builder.size_limit(1024 * 1024);
    let mut global = false;
    for flag in flags.chars() {
        match flag {
            'g' => global = true,
            'i' => {
                builder.case_insensitive(true);
            }
            'm' => {
                builder.multi_line(true);
            }
            's' => {
                builder.dot_matches_new_line(true);
            }
            'x' => {
                builder.ignore_whitespace(true);
            }
            // k: Shift-JIS character mode; o: compile once.
            'k' | 'o' => {}
            _ => bail!("QLIE BREGEXP flag {flag}"),
        }
    }
    Ok((builder.build()?, replacement(&fields[1]), global))
}

/// Perl `$1`, `${1}`, `\1` and `$&` become `regex` group references; every
/// other `$` is literal.
fn replacement(perl: &str) -> String {
    let chars = perl.chars().collect::<Vec<_>>();
    let mut out = String::new();
    let mut at = 0;
    let digits = |from: usize| {
        let mut end = from;
        while end < chars.len() && chars[end].is_ascii_digit() {
            end += 1;
        }
        end
    };
    while at < chars.len() {
        let c = chars[at];
        if c == '\\' && at + 1 < chars.len() {
            let n = chars[at + 1];
            if n.is_ascii_digit() {
                let end = digits(at + 1);
                out.push_str(&format!("${{{}}}", chars[at + 1..end].iter().collect::<String>()));
                at = end;
                continue;
            }
            match n {
                'n' => out.push('\n'),
                't' => out.push('\t'),
                '$' => out.push_str("$$"),
                _ => out.push(n),
            }
            at += 2;
        } else if c == '$' && at + 1 < chars.len() && chars[at + 1].is_ascii_digit() {
            let end = digits(at + 1);
            out.push_str(&format!("${{{}}}", chars[at + 1..end].iter().collect::<String>()));
            at = end;
        } else if c == '$' && chars.get(at + 1) == Some(&'&') {
            out.push_str("${0}");
            at += 2;
        } else if c == '$' && chars.get(at + 1) == Some(&'{') {
            let close = chars[at..].iter().position(|&c| c == '}').map(|p| p + at);
            match close {
                Some(close) if chars[at + 2..close].iter().all(char::is_ascii_digit) && close > at + 2 => {
                    out.extend(&chars[at..=close]);
                    at = close + 1;
                }
                _ => {
                    out.push_str("$$");
                    at += 1;
                }
            }
        } else if c == '$' {
            out.push_str("$$");
            at += 1;
        } else {
            out.push(c);
            at += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    fn apply(command: &str, text: &str) -> String {
        let (regex, rep, global) = substitution(command).unwrap();
        if global {
            regex.replace_all(text, rep.as_str()).into_owned()
        } else {
            regex.replace(text, rep.as_str()).into_owned()
        }
    }
    #[test]
    fn biman_script_forms() {
        assert_eq!(apply(r"s/\[rb,([^,]+),[^,]+\]/$1/gk", "a[rb,漢字,かんじ]b[rb,x,y]"), "a漢字bx");
        assert_eq!(apply(r"s/\[([^\]]*).*?\]//gk", "x[n]y[pc,1]z"), "xyz");
        assert_eq!(apply(r"s/[n]$//k", "ann"), "an");
        assert_eq!(apply(r"s/.*?\[(.*?)\].*/\1/k", "ab[cd]ef"), "cd");
        assert_eq!(apply(r"s/a/$x\//g", "aa"), "$x/$x/");
    }
}
