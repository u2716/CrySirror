// Copyright (c) 2026 CrySirror contributors. MIT.
//! Original CTRL417E00 stack formatting. This preserves encoded native control
//! payloads for the host adapter; no control is acknowledged as a drawn glyph.
use super::*;
impl Machine {
    pub(super) fn text_control(&mut self, bytes: &[u8], next: u32) -> Result<Event> {
        let bytes = &bytes[..bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len())];
        let tag = bytes.first().copied().unwrap_or(0);
        let (event, consumed) = match tag {
            b'd' => {
                let value = &self.tail(1)?[0];
                let bytes = match value {
                    Value::Integer(value) => value.to_string().into_bytes(),
                    Value::String(value) => {
                        value[..value.iter().position(|b| *b == 0).unwrap_or(value.len())].to_vec()
                    }
                    //The old native CRT's %f rounding/nonfinite profiles are
                    //not interchangeable with Rust Display or Unicode text.
                    Value::Float(_) => {
                        bail!("Majiro CTRL d float needs original CRT fixed-decimal profile")
                    }
                };
                ensure!(
                    bytes.len() <= 1023,
                    "Majiro dynamic text exceeds native message buffer"
                );
                (Event::Text { pc: self.pc, bytes }, 1)
            }
            b'c' | b'l' | b'o' => {
                let values = self.tail(2)?;
                let bytes = format!(
                    "{}0x{:08x}0x{:08x}",
                    char::from(tag),
                    values[0].integer()? as u32,
                    values[1].integer()? as u32
                )
                .into_bytes();
                (Event::Control { pc: self.pc, bytes }, 2)
            }
            b's' | b't' => {
                let value = self.tail(1)?[0].integer()?;
                (
                    Event::Control {
                        pc: self.pc,
                        bytes: format!("{}0x{:08x}", char::from(tag), value as u32).into_bytes(),
                    },
                    1,
                )
            }
            b'g' => {
                let values = self.tail(6)?;
                let mut output = String::from("g");
                for value in values {
                    output.push_str(&format!("0x{:08x}", value.integer()? as u32));
                }
                (
                    Event::Control {
                        pc: self.pc,
                        bytes: output.into_bytes(),
                    },
                    6,
                )
            }
            b'f' => {
                let values = self.tail(5)?;
                let font = values[3].bytes()?;
                let font = &font[..font.iter().position(|b| *b == 0).unwrap_or(font.len())];
                let mut output = format!(
                    "f0x{:08x}0x{:08x}0x{:08x}0x{:08x}",
                    values[0].integer()? as u32,
                    values[1].integer()? as u32,
                    values[2].integer()? as u32,
                    values[4].integer()? as u32
                )
                .into_bytes();
                ensure!(
                    output.len() + font.len() <= 127,
                    "Majiro CTRL f native buffer extent"
                );
                output.extend_from_slice(font);
                (
                    Event::Control {
                        pc: self.pc,
                        bytes: output,
                    },
                    5,
                )
            }
            b'x' => {
                let value = self.tail(1)?[0].bytes()?;
                let value = &value[..value.iter().position(|b| *b == 0).unwrap_or(value.len())];
                ensure!(value.len() < 127, "Majiro CTRL x native buffer extent");
                let mut output = Vec::with_capacity(1 + value.len());
                output.push(b'x');
                output.extend_from_slice(value);
                (
                    Event::Control {
                        pc: self.pc,
                        bytes: output,
                    },
                    1,
                )
            }
            _ => {
                ensure!(bytes.len() <= 127, "Majiro CTRL native buffer extent");
                (
                    Event::Control {
                        pc: self.pc,
                        bytes: bytes.to_vec(),
                    },
                    0,
                )
            }
        };
        //Arguments stay live until the actual text/control service succeeds.
        //A retry reuses this immutable event rather than reformatting/popping.
        Ok(self.queue(event, next, consumed))
    }
}
