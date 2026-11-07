// SPDX-License-Identifier: MPL-2.0
//!44BBB0 is a separate, restricted compiler, not live44C880 with DEF disabled.
//!4561A0 then overwrites its initialized prefix and reapplies the saved pointer.
use super::*;
pub(super) struct Work {
    pub memory: Vec<u8>,
    pub caption: Option<String>,
}
fn lead(character: char) -> bool {
    let mut text = [0; 4];
    let (bytes, _, _) = encoding_rs::SHIFT_JIS.encode(character.encode_utf8(&mut text));
    bytes
        .first()
        .is_some_and(|b| (0x81..=0x9f).contains(b) || (0xe0..=0xef).contains(b))
}
pub(super) fn rebuild(
    slot: &crate::native_storage::slot::Slot,
    source: &str,
    machine: &Machine,
    check: &dyn Fn() -> Result<()>,
) -> Result<Work> {
    check()?;
    let mut compiled = String::new();
    let mut caption = None;
    let mut chars = source.chars().peekable();
    if chars
        .peek()
        .is_some_and(|c| matches!(c, '\\' | '@' | '[') || lead(*c))
    {
        while let Some(character) = chars.next() {
            check()?;
            if character == '\0' {
                break;
            }
            ensure!(compiled.len() < 0x30000, "SugarPot reload compiler budget");
            if character == '[' {
                let mut header = String::new();
                loop {
                    check()?;
                    match chars
                        .next()
                        .context("SugarPot reload unterminated header")?
                    {
                        ']' => break,
                        '\\' => {
                            chars
                                .next()
                                .context("SugarPot reload header substitution code")?;
                            let key = name(&mut chars);
                            header.push_str(substitution(machine, &key, false)?);
                        }
                        value => header.push(value),
                    }
                    ensure!(
                        encoding_rs::SHIFT_JIS.encode(&header).0.len() < 64,
                        "SugarPot reload native header budget"
                    );
                }
                //No brackets or label bytes are copied to419C here.
                caption = Some(header);
            } else if matches!(character, '\\' | '@') {
                let code = chars.next().context("SugarPot reload escape code")?;
                match code {
                    'c' | 'C' => {
                        compiled.push(character);
                        compiled.push(code);
                        while let Some(&next) = chars.peek() {
                            check()?;
                            if next == '\0' || matches!(next, '[' | '\\' | '@') || lead(next) {
                                break;
                            }
                            compiled.push(chars.next().unwrap());
                        }
                    }
                    'f' | 'F' | 'i' | 'I' => {
                        compiled.push(character);
                        compiled.push(code);
                        break;
                    }
                    'n' | 'N' | '0'..='9' => {
                        compiled.push(character);
                        compiled.push(code);
                    }
                    's' | 'S' => {
                        let key = name(&mut chars);
                        compiled.push_str(substitution(machine, &key, false)?);
                    }
                    //p/P and every other code take native LABEL_62. No
                    //marker/code is copied; no unknown operation is ACKed.
                    _ => break,
                }
            } else {
                //44BBB0's ASCII default fails to advance v1. Portable repair:
                //consume the initialized scalar, retaining ordinary CP932
                //bytes instead of entering that original infinite loop.
                compiled.push(character);
            }
        }
    }
    let (bytes, _, invalid) = encoding_rs::SHIFT_JIS.encode(&compiled);
    let (redraw, _, invalid_redraw) = encoding_rs::SHIFT_JIS.encode(&slot.current_text);
    ensure!(
        !invalid && !invalid_redraw && bytes.len() < 0x10000 && redraw.len() < 0x10000,
        "SugarPot reload CP932/work budget"
    );
    //The original419C block is64KiB. Its retained bytes after an inserted NUL
    //matter when saved lpString is beyond that NUL. Seed unsaved prior-VM
    //bytes with zero as a portable repair, then retain every initialized byte
    //written by44BBB0 before4561A0's conditional prefix overwrite.
    let mut memory = vec![0; 0x10000];
    check()?;
    memory[..bytes.len()].copy_from_slice(&bytes);
    if !bytes.is_empty() && !bytes.eq_ignore_ascii_case(&redraw) {
        memory[..redraw.len()].copy_from_slice(&redraw);
        memory[redraw.len()] = 0;
    }
    Ok(Work { memory, caption })
}
