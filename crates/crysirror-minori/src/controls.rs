// SPDX-License-Identifier: MPL-2.0
//! The extended scenario reader expands character macros before label binding.
use crate::{
    command::CommandKind,
    script::{Program, Statement},
};
use anyhow::{Result, ensure};
use std::collections::BTreeMap;

struct Binding {
    base: i32,
    current: i32,
    next: i32,
    limit: i32,
    active: bool,
    pending: bool,
    position: [i32; 2],
}
impl Binding {
    fn signed_current(&self) -> i32 {
        if self.active {
            self.current
        } else {
            -self.current
        }
    }
}
fn command(kind: CommandKind, arguments: Vec<String>) -> Statement {
    Statement::Command { kind, arguments }
}
fn character(args: &[&str]) -> Statement {
    command(
        CommandKind::Char,
        args.iter().map(|s| s.to_string()).collect(),
    )
}
fn number(text: &str, default: i32) -> i32 {
    text.parse().unwrap_or(default)
}

pub fn expand(program: Program) -> Result<Program> {
    let mut bindings = BTreeMap::<String, Binding>::new();
    let mut last_stage = None;
    let mut serial = 0;
    let mut result = Program::default();
    for statement in program.statements {
        let output = &mut result.statements;
        match statement {
            Statement::Control { name, arguments: a } => match name.as_str() {
                "define" if a.len() >= 4 => {
                    let base = number(&a[1], 100);
                    ensure!(
                        base > 0 && base <= i32::MAX - 100,
                        "minori character definition range"
                    );
                    bindings.entry(a[0].clone()).or_insert(Binding {
                        base,
                        current: base,
                        next: base + 10,
                        limit: base + 100,
                        active: false,
                        pending: false,
                        position: [number(&a[2], 0), number(&a[3], 0)],
                    });
                }
                "load" if a.len() >= 2 => {
                    if let Some(b) = bindings.get_mut(&a[0]) {
                        for (i, v) in a.iter().skip(2).take(2).enumerate() {
                            if v != "*" {
                                b.position[i] = number(v, 0);
                            }
                        }
                        if let Some(base) = a.get(4) {
                            serial = (serial + 1) % 10;
                            b.next = number(base, 0)
                                .checked_add(serial)
                                .ok_or_else(|| anyhow::anyhow!("minori character load range"))?;
                            ensure!(
                                (0..=i32::MAX - 100).contains(&b.next),
                                "minori character load range"
                            );
                            b.limit = b.next + 100;
                        }
                        b.pending = true;
                        let id = (-b.next).to_string();
                        output.push(character(&["loada", &id, &a[1], &a[0]]));
                        output.push(character(&[
                            "pos",
                            &id,
                            &b.position[0].to_string(),
                            &b.position[1].to_string(),
                            &b.signed_current().to_string(),
                        ]));
                    }
                }
                "pos" if a.len() >= 3 => {
                    if let Some(b) = bindings.get_mut(&a[0]) {
                        b.position = [number(&a[1], 0), number(&a[2], 0)];
                        output.push(character(&[
                            "pos",
                            &b.signed_current().to_string(),
                            &a[1],
                            &a[2],
                        ]));
                    }
                }
                "move" | "moveofs" if a.len() >= 3 => {
                    if let Some(b) = bindings.get(&a[0]) {
                        let mut args = vec![
                            name,
                            b.signed_current().to_string(),
                            a[1].clone(),
                            a[2].clone(),
                        ];
                        for (i, default) in ["*", "*", "100", "off", "-1"].iter().enumerate() {
                            args.push(a.get(i + 3).map_or(*default, String::as_str).to_owned());
                        }
                        output.push(command(CommandKind::Char, args));
                    }
                }
                "stage" | "update" => {
                    for b in bindings.values_mut() {
                        if !b.pending && b.active {
                            output.push(character(&["keep", &b.current.to_string()]));
                        }
                        if b.pending {
                            b.current = b.next;
                            b.next = b
                                .next
                                .checked_add(10)
                                .filter(|n| *n < b.limit)
                                .unwrap_or(b.base);
                            b.active = true;
                            b.pending = false;
                        }
                    }
                    if name == "stage" {
                        last_stage = Some(command(CommandKind::Stage, a));
                    }
                    if let Some(stage) = &last_stage {
                        output.push(stage.clone());
                    }
                }
                "finish" => {
                    if let Some(key) = a.first() {
                        bindings.remove(key);
                    }
                }
                "finish_all" => bindings.clear(),
                "define" | "load" | "pos" | "move" | "moveofs" => {}
                _ => anyhow::bail!("unknown minori control @{name}"),
            },
            Statement::Command {
                kind: CommandKind::Char,
                mut arguments,
            } => {
                if let Some(arg) = arguments.get_mut(1) {
                    if let Some(name) = arg.strip_prefix("@@") {
                        if let Some(b) = bindings.get_mut(name) {
                            b.pending = true;
                            *arg = (-b.next).to_string();
                        }
                    } else if let Some(name) = arg.strip_prefix('@') {
                        if let Some(b) = bindings.get(name) {
                            *arg = b.signed_current().to_string();
                        }
                    }
                }
                output.push(command(CommandKind::Char, arguments));
            }
            Statement::Label(ref label) => {
                result.labels.insert(label.clone(), output.len());
                output.push(statement);
            }
            other => output.push(other),
        }
        ensure!(
            output.len() <= 100_000,
            "expanded minori scenario exceeds statement budget"
        );
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn expansion_precedes_label_binding_and_rotates_ids() {
        let src = "@define hero 600 10 20\n@load hero first.png\n.char movie @@hero clip.avi\n@stage * bg.png 0 0\n.label start\n@update\n@load hero second.png\n@finish_all";
        let p = expand(Program::parse(src).unwrap()).unwrap();
        assert_eq!(
            p.statements[0],
            character(&["loada", "-610", "first.png", "hero"])
        );
        assert_eq!(p.statements[2], character(&["movie", "-610", "clip.avi"]));
        assert_eq!(p.labels["start"], 4);
        assert_eq!(p.statements[5], character(&["keep", "610"]));
        assert_eq!(
            p.statements[7],
            character(&["loada", "-620", "second.png", "hero"])
        );
    }
}
