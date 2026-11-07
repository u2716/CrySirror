//! Native TImoReader replacement/ET/NT policy. Labels prefixed @@! bypass
//! reader preprocessing. The generated operations run through the ordinary
//! VM, including original ImoScripter functions, rather than a translated VN.
use super::*;
impl QlieVm {
    pub(super) fn rewrite_line(&self, line: &str, manager: &str) -> Result<String> {
        let Some(rep) = self.objects.get(manager) else {
            return Ok(line.to_owned());
        };
        let mut result = line.to_owned();
        if !rep.locked {
            for from in rep.lines.iter().rev() {
                let to = rep
                    .table
                    .get(from)
                    .context("QLIE replacement entry missing")?;
                let mode = rep
                    .properties
                    .get(&format!("rule:{from}"))
                    .and_then(|v| v.first())
                    .map(Value::integer)
                    .transpose()?
                    .unwrap_or(0);
                if from.is_empty() {
                    continue;
                }
                if mode == 0 {
                    let regex = rep
                        .regex_rules
                        .get(from)
                        .context("QLIE compiled literal rule missing")?;
                    //4E4434 selects Delphi IgnoreCase and suppresses ReplaceAll
                    //when the replacement itself contains the pattern. Output
                    //is never recursively searched by the shared span copier.
                    let one = regex.is_match(to);
                    let matches = regex
                        .find_iter(&result)
                        .take(if one { 1 } else { usize::MAX })
                        .map(|matched| (matched.range(), to.as_bytes()))
                        .collect::<Vec<_>>();
                    result = String::from_utf8(crysirror_text::byte_replace::replace_spans_bytes(
                        result.as_bytes(),
                        &matches,
                        1024 * 1024,
                        &|| Ok(()),
                    )?)?;
                } else {
                    let regex = rep
                        .regex_rules
                        .get(from)
                        .context("QLIE compiled replacement rule missing")?;
                    result = match mode {
                        1 => {
                            if regex.is_match(&result) {
                                to.clone() + &result
                            } else {
                                result
                            }
                        }
                        2 => regex.replace_all(&result, to.as_str()).into_owned(),
                        3 => {
                            let mut matches = regex.find_iter(&result).peekable();
                            if matches.peek().is_none() {
                                result
                            } else {
                                let mut output = String::new();
                                for matched in matches {
                                    output += to;
                                    output += matched.as_str();
                                    ensure!(
                                        output.len() <= 1024 * 1024,
                                        "QLIE capture output budget"
                                    );
                                }
                                output
                            }
                        }
                        _ => bail!("unsupported QLIE replacement mode {mode}"),
                    };
                }
                ensure!(
                    result.len() <= 1024 * 1024,
                    "QLIE replacement output budget"
                );
            }
        }
        let prefix = |key: &str| {
            rep.properties
                .get(key)
                .and_then(|v| v.first())
                .map(Value::text)
                .unwrap_or_default()
        };
        if result.is_empty() {
            result = prefix("nt");
        }
        if result
            .chars()
            .next()
            .is_some_and(|c| !matches!(c, '\\' | ' ' | '\t' | '/' | '@'))
        {
            result = prefix("et") + &result;
        }
        Ok(result)
    }
    pub(super) fn expand_line(&mut self, m: &mut Machine, ins: &Instruction) -> Result<bool> {
        if !ins.preprocess {
            return Ok(true);
        }
        if m.frame.path.starts_with("scenario/") {
            let key = (m.frame.path.clone(), ins.line);
            self.last_read = self.read_lines.contains(&key);
            if !self.objects.get("llog").is_some_and(|o| o.locked) {
                ensure!(
                    self.read_lines.len() < 1_000_000,
                    "QLIE read line history budget"
                );
                self.read_lines.insert(key);
            }
        }
        let line = if ins.op == "blank" {
            String::new()
        } else if ins.op == "text" {
            ins.args[0].clone()
        } else if ins.args.is_empty() {
            ins.op.clone()
        } else {
            format!("{},{}", ins.op, ins.args.join(","))
        };
        let source = self.rewrite_line(&line, &Self::builtin("rep", m.source_thread_kind()))?;
        if source.is_empty() {
            return Ok(true);
        }
        ensure!(
            source.starts_with('\\'),
            "QLIE reader produced unhandled text: {source}"
        );
        m.inject(&source, BTreeMap::new())?;
        Ok(true)
    }
}
