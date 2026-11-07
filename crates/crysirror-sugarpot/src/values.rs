// SPDX-License-Identifier: MPL-2.0
//! Original value/container commands, not shared image/text/audio mechanisms.
//! 4438C0/443B80/443E40 have identical list construction; 444100/444620
//! append, 444400 removes the head, 444920 removes the tail. Native reg changes
//! its tag on conversion/removal. Named nullsub_1 entries are explicit below.
use crate::{expression::Value, machine::Machine};
use anyhow::{Context, Result, bail, ensure};
impl Machine {
    pub fn value_command(&mut self, name: &str, args: &[String]) -> Result<bool> {
        if ["nop", "pick", "insp", "delp", "swap", "dummy"].contains(&name) {
            return Ok(true);
        }
        match name {
            "def" => {
                let declaration = args.join(",");
                let (key, literal) = declaration
                    .split_once(|c: char| c.is_ascii_whitespace())
                    .context("SugarPot def name/value")?;
                let key = key.trim();
                ensure!(
                    !key.is_empty() && !self.state.variables.contains_key(key),
                    "SugarPot duplicate def"
                );
                let value = self.value(literal.trim())?;
                ensure!(!matches!(value, Value::List(_)), "SugarPot def container");
                self.state.variable_order.push(key.into());
                self.state.variables.insert(key.into(), value);
            }
            "array" => {
                for arg in args {
                    self.value(arg)?;
                }
            } //4438A0 only parses arguments; no array is allocated.
            "que" | "stack" | "pool" => {
                ensure!(args.len() == 2, "SugarPot list declaration signature");
                let key = self.value(&args[0])?.string()?.to_owned();
                ensure!(
                    !self.state.variables.contains_key(&key),
                    "SugarPot duplicate list"
                );
                let value = self.value(&args[1])?;
                ensure!(
                    !matches!(value, Value::List(_)),
                    "SugarPot nested list initializer"
                );
                self.state.list_names.insert(key.clone(), vec![name.into()]);
                self.state.variable_order.push(key.clone());
                self.state.variables.insert(key, Value::List(vec![value]));
            }
            "enque" | "push" => {
                ensure!(args.len() >= 2, "SugarPot list append signature");
                let key = self.value(&args[0])?.string()?.to_owned();
                let values = args[1..]
                    .iter()
                    .map(|a| self.value(a))
                    .collect::<Result<Vec<_>>>()?;
                ensure!(
                    values.iter().all(|v| !matches!(v, Value::List(_))),
                    "SugarPot nested list append"
                );
                let Value::List(list) = self
                    .state
                    .variables
                    .get_mut(&key)
                    .context("SugarPot unknown list")?
                else {
                    bail!("SugarPot list required")
                };
                ensure!(list.len() + values.len() <= 65535, "SugarPot list budget");
                let names = self
                    .state
                    .list_names
                    .get_mut(&key)
                    .context("SugarPot native list child names")?;
                names.extend(std::iter::repeat_n(
                    if name == "push" {
                        "stack".into()
                    } else {
                        "que".into()
                    },
                    values.len(),
                ));
                list.extend(values);
            }
            "deque" | "pop" => {
                ensure!(args.len() == 1, "SugarPot list removal signature");
                let key = self.value(&args[0])?.string()?.to_owned();
                let Value::List(list) = self
                    .state
                    .variables
                    .get_mut(&key)
                    .context("SugarPot unknown list")?
                else {
                    bail!("SugarPot list required")
                };
                ensure!(!list.is_empty(), "SugarPot native empty-list dereference");
                let value = if name == "pop" {
                    list.pop().unwrap()
                } else {
                    list.remove(0)
                };
                let names = self
                    .state
                    .list_names
                    .get_mut(&key)
                    .context("SugarPot native list child names")?;
                if name == "pop" {
                    names.pop();
                } else {
                    names.remove(0);
                }
                self.state.variables.insert("reg".into(), value);
            }
            "max" | "min" => {
                // Portable repair of 445A10/445BC0: native tests the BEST
                // type slot, permitting a scan beyond initialized stack arguments.
                // Bound the scan to parsed arguments, preserving raw DWORD signed
                // comparisons, first-index tie behavior, and the zero-argument result.
                let mut selected = 0;
                let mut extreme = None;
                for (index, arg) in args.iter().enumerate() {
                    let word = match self.value(arg)? {
                        Value::Integer(n) => n,
                        Value::Float(n) => n.to_bits() as i32,
                        Value::String(s) => {
                            let (bytes, _, errors) = encoding_rs::SHIFT_JIS.encode(&s);
                            ensure!(!errors, "SugarPot max/min string encoding");
                            let mut word = [0; 4];
                            let n = bytes.len().min(4);
                            word[..n].copy_from_slice(&bytes[..n]);
                            i32::from_le_bytes(word)
                        }
                        _ => bail!("SugarPot max/min container"),
                    };
                    if extreme.is_none_or(|old| {
                        if name == "max" {
                            word > old
                        } else {
                            word < old
                        }
                    }) {
                        selected = index;
                        extreme = Some(word);
                    }
                }
                self.set_reg(selected as i32);
            }
            "num" => {
                let value = args.first().map(|a| self.value(a)).transpose()?;
                let s = match value {
                    Some(Value::String(s)) => s,
                    _ => String::new(),
                };
                let text = s
                    .chars()
                    .map(|c| {
                        if ('\u{ff01}'..='\u{ff5e}').contains(&c) {
                            char::from_u32(c as u32 - 0xfee0).unwrap()
                        } else {
                            c
                        }
                    })
                    .collect::<String>();
                let text = text.trim_start();
                let (sign, text) = if let Some(s) = text.strip_prefix('-') {
                    (-1, s)
                } else if let Some(s) = text.strip_prefix('~') {
                    (0, s)
                } else {
                    (1, text)
                };
                let (base, text) =
                    if let Some(s) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
                        (16, s)
                    } else {
                        (10, text)
                    };
                let digits = text
                    .chars()
                    .take_while(|c| c.is_digit(base))
                    .collect::<String>();
                let n = crate::expression::crt_positive(&digits, base);
                self.set_reg(if sign < 0 {
                    n.wrapping_neg()
                } else if sign == 0 {
                    !n
                } else {
                    n
                });
            }
            "str" => {
                let value = args.first().map(|a| self.value(a)).transpose()?;
                let (n, fractional) = match value {
                    Some(Value::Integer(n)) => (n, false),
                    Some(Value::Float(n)) => {
                        // 445CE0 FILDs the IEEE bits as signed integer, converts
                        // that to f32, then 45BC90 truncates before multiplying
                        // the fraction: the fractional result is always zero.
                        let converted = Value::Float(n.to_bits() as i32 as f32).integer()?;
                        (converted, true)
                    }
                    _ => (0, false),
                };
                let mut s: String = n
                    .to_string()
                    .chars()
                    .map(|c| char::from_u32(c as u32 + 0xfee0).unwrap())
                    .collect();
                if fractional {
                    // Portable repair: native 45BEB0 widens each byte of the
                    // already-CP932 fullwidth dot separately into an undersized
                    // destination. Preserve its determined numeric formatting
                    // with one correctly encoded fullwidth dot and zero.
                    s.push_str("．０");
                }
                self.state.variables.insert("reg".into(), Value::String(s));
            }
            "nop" | "pick" | "insp" | "delp" | "swap" | "menu" | "snap" | "set" | "layer"
            | "msg" | "call" | "ret" | "save" | "load" | "anm" | "ss" | "bgm" | "env" | "se"
            | "print" | "slct" | "ofs" | "scroll" | "move" | "udwait" | "ssset" | "ssreset"
            | "sson" | "ssoff" | "anmon" | "anmoff" | "anmenable" | "anmdisable" => {
                for arg in args {
                    self.value(arg)?;
                }
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
}
